//! Tests of the reference-motion-vector map, contexts and stacks.

use super::*;
use crate::tile::coding_unit::MotionVector;
use crate::tile::context::get_compound_drl_context;

// `inter_mode`/`compound_mode` (`SpatialRefContext`) tests.

#[test]
fn test_inter_mode_context_no_neighbors_is_zero() {
    let ctx = SpatialRefContext::new(16, 16);
    assert_eq!(ctx.inter_mode_context(0, 0, 4, 4, 0, false), 0);
}

#[test]
fn test_inter_mode_context_use_ref_frame_mvs_sets_globalmv_bit() {
    // No spatial neighbors (refmv_ctx=0, newmv_ctx=0) isolates globalmv_ctx: packed layout is
    // `refmv_ctx << 4 | globalmv_ctx << 3 | newmv_ctx`, so globalmv_ctx alone should produce
    // exactly bit 3 (value 8).
    let ctx = SpatialRefContext::new(16, 16);
    assert_eq!(ctx.inter_mode_context(0, 0, 4, 4, 0, false), 0);
    assert_eq!(ctx.inter_mode_context(0, 0, 4, 4, 0, true), 8);
}

#[test]
fn test_inter_mode_context_top_forward_match_not_newmv() {
    let mut ctx = SpatialRefContext::new(16, 16);
    ctx.set_block(
        0,
        0,
        4,
        4,
        0,
        -1,
        false,
        MotionVector::zero(),
        MotionVector::zero(),
    ); // LAST, covers x4=0..4, y4=0..4
       // Query directly below: top row scan hits row y4=0, columns 0..4 -- a match.
       // nearest_match=1 -> refmv_ctx=min(1*3,4)=3, newmv_ctx=3-0=3 -> packed = 3<<4|3 = 51.
    assert_eq!(ctx.inter_mode_context(0, 1, 4, 4, 0, false), 51);
}

#[test]
fn test_inter_mode_context_top_match_is_newmv_lowers_newmv_ctx() {
    let mut ctx = SpatialRefContext::new(16, 16);
    ctx.set_block(
        0,
        0,
        4,
        4,
        0,
        -1,
        true,
        MotionVector::zero(),
        MotionVector::zero(),
    );
    // Same as above but is_newmv=true -> newmv_ctx = 3-1=2 -> packed = 3<<4|2 = 50.
    assert_eq!(ctx.inter_mode_context(0, 1, 4, 4, 0, false), 50);
}

#[test]
fn test_inter_mode_context_no_ref_match_is_zero() {
    let mut ctx = SpatialRefContext::new(16, 16);
    ctx.set_block(
        0,
        0,
        4,
        4,
        4,
        -1,
        false,
        MotionVector::zero(),
        MotionVector::zero(),
    ); // GOLDEN
       // Query for a different ref (LAST) -- no match at all.
    assert_eq!(ctx.inter_mode_context(0, 1, 4, 4, 0, false), 0);
}

#[test]
fn test_inter_mode_context_single_ref_matches_either_slot_of_compound_neighbor() {
    let mut ctx = SpatialRefContext::new(16, 16);
    ctx.set_block(
        0,
        0,
        4,
        4,
        2,
        5,
        false,
        MotionVector::zero(),
        MotionVector::zero(),
    ); // compound neighbor: LAST3 + ALTREF2
       // Single-ref query for ref0=5 (ALTREF2) matches via the neighbor's ref1 slot.
    assert_eq!(ctx.inter_mode_context(0, 1, 4, 4, 5, false), 51);
}

#[test]
fn test_compound_mode_context_no_neighbors_is_zero() {
    let ctx = SpatialRefContext::new(16, 16);
    assert_eq!(ctx.compound_mode_context(0, 0, 4, 4, 0, 4), 0);
}

#[test]
fn test_compound_mode_context_exact_pair_required() {
    let mut ctx = SpatialRefContext::new(16, 16);
    ctx.set_block(
        0,
        0,
        4,
        4,
        0,
        4,
        false,
        MotionVector::zero(),
        MotionVector::zero(),
    ); // compound (LAST, BWDREF)
       // Swapped pair must NOT match (rav1d: exact `RefMvsRefPair` equality).
    assert_eq!(ctx.compound_mode_context(0, 1, 4, 4, 4, 0), 0);
    // Exact pair matches: nearest_match=1 -> refmv_ctx=3, refmv_ctx>>1=1 -> 1+min(newmv_ctx,3).
    // newmv_ctx=3 (not newmv) -> 1+3=4.
    assert_eq!(ctx.compound_mode_context(0, 1, 4, 4, 0, 4), 4);
}

#[test]
fn test_compound_mode_context_both_sides_match_saturates_to_seven() {
    let mut ctx = SpatialRefContext::new(16, 16);
    // Isolated single-cell placements: row 4 (above the query row) and column 4 (left of the
    // query column), so both the top and left primary scans find exactly one match each.
    ctx.set_block(
        5,
        4,
        4,
        1,
        0,
        4,
        false,
        MotionVector::zero(),
        MotionVector::zero(),
    );
    ctx.set_block(
        4,
        5,
        1,
        4,
        0,
        4,
        false,
        MotionVector::zero(),
        MotionVector::zero(),
    );
    // nearest_match=2 -> refmv_ctx=5, refmv_ctx>>1=2 -> clamp(3+newmv_ctx,4,7).
    // newmv_ctx=5-0=5 -> clamp(8,4,7)=7.
    assert_eq!(ctx.compound_mode_context(5, 5, 4, 4, 0, 4), 7);
}

#[test]
fn test_compound_mv_stack_requires_exact_pair_match() {
    let mut ctx = SpatialRefContext::new(16, 16);
    let mv_a = MotionVector::new(4, 8);
    let mv_b = MotionVector::new(-2, 6);
    // A neighbor with the SWAPPED pair (BWDREF, LAST) at the query's (LAST, BWDREF) position
    // must not contribute to the EXACT-pair main scan -- real spec's exact `RefMvsRefPair`
    // equality, no partial single-slot fallback at that stage (`compound_candidate_mv`'s
    // doc). It's still cnt<2 afterward though, so `fill_compound_extended_candidates`'s
    // per-component fallback picks the swapped neighbor back up: its `ref1`(=LAST) matches
    // our `ref0`, contributing `mv_b` to component 0, and its `ref0`(=BWDREF) matches our
    // `ref1`, contributing `mv_a` to component 1 -- a cross-component reconstruction from a
    // single neighbor whose full pair never matched.
    ctx.set_block(0, 0, 4, 4, 4, 0, false, mv_a, mv_b);
    let (stack, cnt) = ctx.compound_mv_stack(0, 1, 4, 4, 0, 4, false);
    assert_eq!(cnt, 2);
    assert_eq!(stack[0].mv, [mv_b, mv_a]);
    assert_eq!(stack[1].mv, [MotionVector::zero(), MotionVector::zero()]);

    // The exact pair DOES contribute at the main-scan stage, with both MVs preserved as a
    // joint pair (not independently re-derived) -- `stack2[0]` is real. `cnt2` still ends up 2
    // (not 1): the same cnt<2 fallback runs afterward, finds this exact neighbor's
    // per-component matches equal `stack2[0]` exactly, and (per the "avoid duplicating the
    // real entry" rule) falls through to its second extended candidate, which has no further
    // same-ref matches left to draw on and so is the zero-MV global-motion approximation.
    let mut ctx2 = SpatialRefContext::new(16, 16);
    ctx2.set_block(0, 0, 4, 4, 0, 4, false, mv_a, mv_b);
    let (stack2, cnt2) = ctx2.compound_mv_stack(0, 1, 4, 4, 0, 4, false);
    assert_eq!(cnt2, 2);
    assert_eq!(stack2[0].mv, [mv_a, mv_b]);
    assert_eq!(stack2[1].mv, [MotionVector::zero(), MotionVector::zero()]);
}

#[test]
fn test_compound_mv_stack_dedups_identical_pairs_by_weight() {
    let mut ctx = SpatialRefContext::new(16, 16);
    let mv_pair = [MotionVector::new(1, 1), MotionVector::new(2, 2)];
    // Two separate 1x1 neighbors (row above at x4=0 and x4=1) with the IDENTICAL pair --
    // real spec accumulates weight into one entry rather than pushing a duplicate
    // (`push_compound_mv_candidate`'s doc).
    ctx.set_block(0, 0, 1, 1, 0, 4, false, mv_pair[0], mv_pair[1]);
    ctx.set_block(1, 0, 1, 1, 0, 4, false, mv_pair[0], mv_pair[1]);
    let (stack, cnt) = ctx.compound_mv_stack(0, 1, 2, 4, 0, 4, false);
    // The real dedup happens at the main-scan stage: both 1x1 neighbors merge into ONE
    // weighted entry (`stack[0]`, weight = 640 base-bump + 4(top-right corner rescan of the
    // same 1x1 cell) = 644) instead of two separate stack slots -- this is what this test
    // actually exercises. `cnt` itself is 2, not 1, because the same cnt<2 fallback that
    // `test_compound_mv_stack_requires_exact_pair_match` covers runs afterward regardless: it
    // finds the identical pair again via both neighbors' per-component matches and pads a
    // second stack slot with that same value (weight 2, the fallback's fixed weight -- clearly
    // distinguishable from a real match's `>= 640` "nearest" tier).
    assert_eq!(cnt, 2);
    assert_eq!(stack[0].mv, mv_pair);
    assert!(
        stack[0].weight >= 640,
        "real match must be in the nearest tier"
    );
    assert_eq!(stack[1].mv, mv_pair);
    assert_eq!(
        stack[1].weight, 2,
        "fallback-padded slot uses the fixed weight"
    );
}

#[test]
fn test_get_compound_drl_context_matches_weight_thresholds() {
    let mut stack = [CompoundMvStackEntry::default(); 8];
    // Both entries in the "nearest" (>= 640) tier -> ctx 0.
    stack[0].weight = 700;
    stack[1].weight = 650;
    assert_eq!(get_compound_drl_context(&stack, 0), 0);
    // idx in nearest tier, idx+1 in secondary tier (< 640) -> ctx 1.
    stack[1].weight = 100;
    assert_eq!(get_compound_drl_context(&stack, 0), 1);
    // Both in secondary tier -> ctx 2.
    stack[0].weight = 50;
    assert_eq!(get_compound_drl_context(&stack, 0), 2);
}

#[test]
fn test_inter_mode_context_top_left_corner_contributes_but_not_to_newmv() {
    let mut ctx = SpatialRefContext::new(16, 16);
    // Only the top-left corner cell (x4-1, y4-1) matches, and it's a newmv block -- rav1d
    // discards this candidate's newmv contribution (`have_dummy_newmv_match`).
    ctx.set_block(
        4,
        4,
        1,
        1,
        0,
        -1,
        true,
        MotionVector::zero(),
        MotionVector::zero(),
    );
    // nearest_match=0 (top-left isn't scanned until after nearest_match is computed) ->
    // refmv_ctx=min(ref_match_count,2), newmv_ctx=(ref_match_count>0).
    // ref_match_count=1 (top-left counted into have_row_mvs) -> refmv_ctx=1, newmv_ctx=1.
    // packed = 1<<4|1 = 17 -- and critically NOT lowered by the neighbor's newmv flag.
    assert_eq!(ctx.inter_mode_context(5, 5, 4, 4, 0, false), 17);
}

#[test]
fn test_inter_mode_context_secondary_row_scan_finds_distant_match() {
    let mut ctx = SpatialRefContext::new(16, 16);
    // Out of reach of the primary top scan (row y4-1 = 4) but on the row the secondary n=2
    // scan reads: `(y4 + 1 - 2n) | 1` = (5 + 1 - 4) | 1 = 3 (the odd row of each 8x8 pair).
    ctx.set_block(
        0,
        3,
        4,
        1,
        0,
        -1,
        false,
        MotionVector::zero(),
        MotionVector::zero(),
    );
    // Primary top scan (row 4) and top-right/top-left find nothing -> nearest_match=0.
    // Secondary scan finds the match -> ref_match_count=1 -> refmv_ctx=1, newmv_ctx=1.
    assert_eq!(ctx.inter_mode_context(0, 5, 4, 4, 0, false), 17);
}

fn mv_x(x: i32) -> MotionVector {
    MotionVector { x, y: 0 }
}

/// dav1d reads the secondary rows at `((y4 - 2n + 1) | 1)`: the odd row of each 8x8 pair, so
/// for the odd `y4 = 5` the n = 2 row is 3, not 2.
#[test]
fn test_single_ref_stack_secondary_rows_use_the_odd_row_of_each_8x8_pair() {
    let mut ctx = SpatialRefContext::new(16, 16);
    ctx.set_frame_extent(64, 64);
    let (a, b) = (mv_x(16), mv_x(24));
    ctx.set_block(4, 3, 4, 1, 0, -1, false, a, MotionVector::zero());
    ctx.set_block(4, 2, 4, 1, 0, -1, false, b, MotionVector::zero());
    let (stack, cnt) = ctx.single_ref_mv_stack(4, 5, 2, 2, 0, false);
    assert_eq!(cnt, 1);
    assert_eq!(stack[0].mv, a);
    // weight = len (2) * weight (max(2, min(2 * (1 + 3 - 2), 1)) = 2)
    assert_eq!(stack[0].weight, 4);
}

/// A secondary row is only scanned when the rows scanned so far (`n_rows`) do not already
/// reach it: a 4-high neighbour above covers rows 1..=4, so n = 2 is skipped and only n = 3
/// (weight 2 * 2) adds to it, on top of 8 (primary row) + 4 (top-right) + 640.
#[test]
fn test_single_ref_stack_secondary_rows_are_gated_by_rows_already_covered() {
    let mut ctx = SpatialRefContext::new(16, 16);
    ctx.set_frame_extent(64, 64);
    let c = mv_x(16);
    ctx.set_block(4, 1, 4, 4, 0, -1, false, c, MotionVector::zero());
    let (stack, cnt) = ctx.single_ref_mv_stack(4, 5, 2, 2, 0, false);
    assert_eq!(cnt, 1);
    assert_eq!(stack[0].mv, c);
    assert_eq!(stack[0].weight, 8 + 4 + 640 + 4);
}

/// An intra block of an inter frame keeps its size in the reference map: a 4-high intra
/// neighbour makes the row scan cover 2 rows (`weight >> 1` with weight 4), an unrecorded
/// cell would count as 1 unit high.
#[test]
fn test_intra_cells_keep_their_size_for_the_neighbour_scan() {
    let mut ctx = SpatialRefContext::new(16, 16);
    ctx.set_frame_extent(64, 64);
    ctx.set_intra_block(4, 4, 4, 4);
    let mut stack = [MvStackEntry::default(); 8];
    let mut cnt = 0;
    let rows = ctx.scan_row_weighted(&mut stack, &mut cnt, 0, 7, 4, 4, 4, 3, 1);
    assert_eq!(rows, 2);
    assert_eq!(cnt, 0, "an intra block is never a candidate");
}

/// Candidates are clamped to the frame in dav1d's 8x8-aligned units (`iw4 = iw8 << 1`): a
/// 36-sample-wide frame is 10 units wide, so the limit is (10 - 0 + 4) * 32 = 448.
#[test]
fn test_single_ref_stack_clamps_to_the_8x8_aligned_frame() {
    let mut ctx = SpatialRefContext::new(16, 16);
    ctx.set_frame_extent(36, 36);
    ctx.set_block(0, 0, 4, 1, 0, -1, false, mv_x(10_000), MotionVector::zero());
    let (stack, cnt) = ctx.single_ref_mv_stack(0, 1, 2, 2, 0, false);
    assert_eq!(cnt, 1);
    assert_eq!(stack[0].mv.x, 448);
}

#[test]
fn test_set_block_intra_default_never_matches() {
    // A cell that's never written (the "intra block" simplification -- see `set_block`'s doc)
    // stays `valid: false` and never contributes, regardless of what ref0/ref1 it queries for.
    let ctx = SpatialRefContext::new(16, 16);
    assert_eq!(ctx.inter_mode_context(5, 5, 4, 4, 0, false), 0);
    assert_eq!(ctx.compound_mode_context(5, 5, 4, 4, 0, 4), 0);
}
