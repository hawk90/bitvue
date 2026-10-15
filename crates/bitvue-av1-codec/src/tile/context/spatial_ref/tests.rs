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
    assert_eq!(stack[1].mv, [mv_a, mv_b]);

    // The exact pair DOES contribute at the main-scan stage, with both MVs preserved as a
    // joint pair (not independently re-derived) -- `stack2[0]` is real. `cnt2` still ends up 2
    // (not 1): the same cnt<2 fallback runs afterward, finds this exact neighbor's
    // per-component matches equal `stack2[0]` exactly, and (per the "avoid duplicating the
    // real entry" rule) falls through to its second extended candidate: each component's
    // "diff" candidate, i.e. the other component's vector, `[mv_b, mv_a]`.
    let mut ctx2 = SpatialRefContext::new(16, 16);
    ctx2.set_block(0, 0, 4, 4, 0, 4, false, mv_a, mv_b);
    let (stack2, cnt2) = ctx2.compound_mv_stack(0, 1, 4, 4, 0, 4, false);
    assert_eq!(cnt2, 2);
    assert_eq!(stack2[0].mv, [mv_a, mv_b]);
    assert_eq!(stack2[1].mv, [mv_b, mv_a]);
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

/// How many rows a secondary scan covers is `1 + max_rows - n`, and it bounds the weight a tall
/// neighbour can earn (`min(height, 2 * rows)`): at `y4 = 5` (`max_rows = 3`) the n = 2 scan
/// covers 2 rows, so a 4-high neighbour weighs 4 per unit of its overlap (2) = 8, not 2 * 2 = 4.
#[test]
fn test_single_ref_stack_secondary_row_weight_is_bounded_by_the_rows_it_covers() {
    let mut ctx = SpatialRefContext::new(16, 16);
    ctx.set_frame_extent(64, 64);
    let c = mv_x(16);
    // Rows 0..=3, so row 3 (the n = 2 row) is inside it; the primary row 4 is empty.
    ctx.set_block(4, 0, 4, 4, 0, -1, false, c, MotionVector::zero());
    // A different block takes over row 1, the n = 3 row. The n = 2 scan covers 2 rows, which
    // reaches row 3 - 2 + 1 and so skips the n = 3 scan; a scan that covered only 1 row would
    // leave n = 3 to run, find this block and add a second candidate.
    ctx.set_block(4, 1, 4, 1, 0, -1, false, mv_x(40), MotionVector::zero());
    let (stack, cnt) = ctx.single_ref_mv_stack(4, 5, 2, 2, 0, false);
    assert_eq!(cnt, 1);
    assert_eq!(stack[0].mv, c);
    assert_eq!(stack[0].weight, 8);
}

/// The same for a secondary column (`1 + max_cols - n`).
#[test]
fn test_single_ref_stack_secondary_column_weight_is_bounded_by_the_columns_it_covers() {
    let mut ctx = SpatialRefContext::new(16, 16);
    ctx.set_frame_extent(64, 64);
    let c = mv_x(16);
    // Columns 0..=3 at rows 4..=7, so column 3 (the n = 2 column) is inside it; column 4 is empty.
    ctx.set_block(0, 4, 4, 4, 0, -1, false, c, MotionVector::zero());
    // A different block takes over column 1, the n = 3 column (see the row test above).
    ctx.set_block(1, 4, 1, 4, 0, -1, false, mv_x(40), MotionVector::zero());
    let (stack, cnt) = ctx.single_ref_mv_stack(5, 4, 2, 2, 0, false);
    assert_eq!(cnt, 1);
    assert_eq!(stack[0].mv, c);
    assert_eq!(stack[0].weight, 8);
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

/// The "other reference" fallback walks the neighbours by their recorded size (dav1d skips a
/// whole 4-unit-wide block at once, even where a later smaller block was written over part of
/// it): the block that overwrote cell 9 is never looked at, so only the wide block's vector
/// is a candidate.
#[test]
fn test_single_ref_stack_extended_row_walk_skips_a_neighbour_by_its_recorded_width() {
    let mut ctx = SpatialRefContext::new(32, 32);
    ctx.set_frame_extent(128, 128);
    let (wide, narrow) = (mv_x(16), mv_x(40));
    ctx.set_block(8, 7, 4, 1, 0, -1, false, wide, MotionVector::zero());
    ctx.set_block(9, 7, 1, 1, 0, -1, false, narrow, MotionVector::zero());
    // Reference 3 matches neither block, so the stack comes from the fallback only.
    let (stack, cnt) = ctx.single_ref_mv_stack(8, 8, 4, 4, 3, false);
    assert_eq!(cnt, 1);
    assert_eq!(stack[0].mv, wide);
    assert_eq!(stack[0].weight, 2);
}

/// The same for the left column, walked by the recorded height.
#[test]
fn test_single_ref_stack_extended_column_walk_skips_a_neighbour_by_its_recorded_height() {
    let mut ctx = SpatialRefContext::new(32, 32);
    ctx.set_frame_extent(128, 128);
    let (tall, short) = (mv_x(16), mv_x(40));
    ctx.set_block(7, 8, 1, 4, 0, -1, false, tall, MotionVector::zero());
    ctx.set_block(7, 9, 1, 1, 0, -1, false, short, MotionVector::zero());
    let (stack, cnt) = ctx.single_ref_mv_stack(8, 8, 4, 4, 3, false);
    assert_eq!(cnt, 1);
    assert_eq!(stack[0].mv, tall);
    assert_eq!(stack[0].weight, 2);
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

/// Neighbours that only partly match the references of a compound query still fill the stack
/// (dav1d's `add_compound_extended_candidate`): a vector of a reference on the other side of the
/// current frame is turned around, one on the same side is kept.
#[test]
fn test_compound_extended_candidates_turn_vectors_around_by_sign_bias() {
    let mut ctx = SpatialRefContext::new(16, 16);
    // LAST (0) is a past reference, ALTREF (6) a future one.
    let mut bias = [false; 7];
    bias[6] = true;
    ctx.set_sign_bias(bias);
    // The only neighbour, above the query, uses ALTREF alone with this vector.
    let mv = MotionVector::new(0, 32);
    ctx.set_block(0, 0, 4, 4, 6, -1, false, mv, MotionVector::zero());

    let (stack, cnt) = ctx.compound_mv_stack(0, 4, 4, 4, 0, 6, false);

    assert_eq!(cnt, 2);
    // Component 1 (ALTREF) takes the vector as it is; component 0 (LAST) gets it turned around
    // because LAST and ALTREF have different sign biases.
    assert_eq!(stack[0].mv, [MotionVector::new(0, -32), mv]);
    assert_eq!(stack[1].mv, [MotionVector::zero(), MotionVector::zero()]);
}

/// A compound pair `(ref0, ref1)`: the references the compound stack tests below query for.
const PAIR: (i8, i8) = (0, 4);

fn pair(x0: i32, x1: i32) -> [MotionVector; 2] {
    [mv_x(x0), mv_x(x1)]
}

/// Temporal pairs need both references: a query with `ref1 < 0` (a single-reference query on
/// the compound path) must not read the temporal context at all -- `pocdiff[ref1]` with a
/// negative reference would index out of bounds.
#[test]
fn test_compound_mv_stack_skips_temporal_pairs_when_ref1_is_negative() {
    use crate::tile::motion_field::{project_motion_field, MotionFieldState};
    let mut ctx = SpatialRefContext::new(16, 16);
    ctx.set_frame_extent(64, 64);
    let projected = project_motion_field(&[], &MotionFieldState::new(), 8, 8);
    ctx.set_temporal_context(projected, [1; 7]);
    let with_temporal = ctx.compound_mv_stack(4, 4, 2, 2, PAIR.0, -1, true);
    let without = ctx.compound_mv_stack(4, 4, 2, 2, PAIR.0, -1, false);
    assert_eq!(with_temporal.1, without.1);
    for i in 0..with_temporal.1 {
        assert_eq!(with_temporal.0[i].mv, without.0[i].mv);
        assert_eq!(with_temporal.0[i].weight, without.0[i].weight);
    }
}

/// dav1d reads the secondary rows at `((y4 - 2n + 1) | 1)`, the odd row of each 8x8 pair: at the
/// odd `y4 = 5` the n = 2 row is 3, not 2 (the compound counterpart of the single-reference test).
#[test]
fn test_compound_mv_stack_secondary_rows_use_the_odd_row_of_each_8x8_pair() {
    let mut ctx = SpatialRefContext::new(16, 16);
    ctx.set_frame_extent(64, 64);
    let (a, b) = (pair(16, 17), pair(24, 25));
    ctx.set_block(4, 3, 4, 1, PAIR.0, PAIR.1, false, a[0], a[1]);
    ctx.set_block(4, 2, 4, 1, PAIR.0, PAIR.1, false, b[0], b[1]);
    let (stack, cnt) = ctx.compound_mv_stack(4, 5, 2, 2, PAIR.0, PAIR.1, false);
    // The second entry is the padding `cnt < 2` triggers; only the scanned one is pinned.
    assert_eq!(cnt, 2);
    assert_eq!(stack[0].mv, a);
    // weight = len (2) * weight (max(2, min(2 * (1 + 3 - 2), 1)) = 2)
    assert_eq!(stack[0].weight, 4);
}

/// The same for a secondary column (`(x4 - 2n + 1) | 1`) at the odd `x4 = 5`.
#[test]
fn test_compound_mv_stack_secondary_columns_use_the_odd_column_of_each_8x8_pair() {
    let mut ctx = SpatialRefContext::new(16, 16);
    ctx.set_frame_extent(64, 64);
    let (a, b) = (pair(16, 17), pair(24, 25));
    ctx.set_block(3, 4, 1, 4, PAIR.0, PAIR.1, false, a[0], a[1]);
    ctx.set_block(2, 4, 1, 4, PAIR.0, PAIR.1, false, b[0], b[1]);
    let (stack, cnt) = ctx.compound_mv_stack(5, 4, 2, 2, PAIR.0, PAIR.1, false);
    assert_eq!(cnt, 2);
    assert_eq!(stack[0].mv, a);
    assert_eq!(stack[0].weight, 4);
}

/// How many rows a secondary scan covers is `1 + max_rows - n`, and it bounds the weight a tall
/// neighbour earns (`min(height, 2 * rows)`): at `y4 = 5` (`max_rows = 3`) the n = 2 scan covers
/// 2 rows, so a 4-high neighbour weighs 4 per unit of its overlap (2) = 8, not 2 * 2 = 4. A
/// different block on the n = 3 row would be found by a scan that covered only 1 row.
#[test]
fn test_compound_mv_stack_secondary_row_weight_is_bounded_by_the_rows_it_covers() {
    let mut ctx = SpatialRefContext::new(16, 16);
    ctx.set_frame_extent(64, 64);
    let (a, b) = (pair(16, 17), pair(24, 25));
    ctx.set_block(4, 0, 4, 4, PAIR.0, PAIR.1, false, a[0], a[1]);
    ctx.set_block(4, 1, 4, 1, PAIR.0, PAIR.1, false, b[0], b[1]);
    let (stack, cnt) = ctx.compound_mv_stack(4, 5, 2, 2, PAIR.0, PAIR.1, false);
    assert_eq!(cnt, 2);
    assert_eq!(stack[0].mv, a);
    assert_eq!(stack[0].weight, 8);
    assert_ne!(stack[1].mv, b, "the n = 3 row must not have been scanned");
}

/// The same for a secondary column (`1 + max_cols - n`).
#[test]
fn test_compound_mv_stack_secondary_column_weight_is_bounded_by_the_columns_it_covers() {
    let mut ctx = SpatialRefContext::new(16, 16);
    ctx.set_frame_extent(64, 64);
    let (a, b) = (pair(16, 17), pair(24, 25));
    ctx.set_block(0, 4, 4, 4, PAIR.0, PAIR.1, false, a[0], a[1]);
    ctx.set_block(1, 4, 1, 4, PAIR.0, PAIR.1, false, b[0], b[1]);
    let (stack, cnt) = ctx.compound_mv_stack(5, 4, 2, 2, PAIR.0, PAIR.1, false);
    assert_eq!(cnt, 2);
    assert_eq!(stack[0].mv, a);
    assert_eq!(stack[0].weight, 8);
    assert_ne!(
        stack[1].mv, b,
        "the n = 3 column must not have been scanned"
    );
}

/// A neighbour found by the main scan earns the exact "nearest" bump of 640 on top of its scan
/// weight: a 2-wide block above a 2-wide query weighs 2 * 2 = 4, so 644.
#[test]
fn test_compound_mv_stack_nearest_candidate_weight_is_scan_weight_plus_640() {
    let mut ctx = SpatialRefContext::new(16, 16);
    ctx.set_frame_extent(64, 64);
    let a = pair(16, 17);
    ctx.set_block(4, 3, 2, 1, PAIR.0, PAIR.1, false, a[0], a[1]);
    let (stack, cnt) = ctx.compound_mv_stack(4, 4, 2, 2, PAIR.0, PAIR.1, false);
    assert_eq!(cnt, 2);
    assert_eq!(stack[0].mv, a);
    assert_eq!(stack[0].weight, 640 + 4);
}

// ---- neighbour walks (`walk_row` / `walk_col`) ----

/// Records `(first MV x, weight)` of each visited neighbour.
fn visits_row(
    ctx: &SpatialRefContext,
    row_y4: u32,
    x4: u32,
    bw4: u32,
    w4: u32,
    max_rows: i32,
    step: u32,
) -> (Vec<(i32, i32)>, u32) {
    let mut seen = Vec::new();
    let ret = ctx.walk_row(row_y4, x4, bw4, w4, max_rows, step, &mut |cell, weight| {
        seen.push((cell.mv0.x, weight));
    });
    (seen, ret)
}

fn visits_col(
    ctx: &SpatialRefContext,
    col_x4: u32,
    y4: u32,
    bh4: u32,
    h4: u32,
    max_cols: i32,
    step: u32,
) -> (Vec<(i32, i32)>, u32) {
    let mut seen = Vec::new();
    let ret = ctx.walk_col(col_x4, y4, bh4, h4, max_cols, step, &mut |cell, weight| {
        seen.push((cell.mv0.x, weight));
    });
    (seen, ret)
}

/// A 1x1 block at `(x4, y4)` whose first MV's x names it.
fn put_1x1(ctx: &mut SpatialRefContext, x4: u32, y4: u32, id: i32) {
    ctx.set_block(x4, y4, 1, 1, 0, -1, false, mv_x(id), MotionVector::zero());
}

#[test]
fn walks_starting_outside_the_grid_visit_nothing_and_cover_one_row() {
    let ctx = SpatialRefContext::new(4, 4);
    for (x4, y4) in [(4, 0), (0, 4), (9, 9), (u32::MAX, 0)] {
        assert_eq!(
            visits_row(&ctx, y4, x4, 2, 2, 2, 1),
            (vec![], 1),
            "row ({x4},{y4})"
        );
        assert_eq!(
            visits_col(&ctx, x4, y4, 2, 2, 2, 1),
            (vec![], 1),
            "col ({x4},{y4})"
        );
    }
}

#[test]
fn narrow_neighbours_are_visited_one_by_one_with_twice_their_length() {
    let mut ctx = SpatialRefContext::new(8, 4);
    for x in 0..4 {
        put_1x1(&mut ctx, x, 0, 10 * (x as i32 + 1));
    }
    // Block 4 wide over four 1-wide neighbours: weight 2 each, one row covered.
    assert_eq!(
        visits_row(&ctx, 0, 0, 4, 4, 2, 1),
        (vec![(10, 2), (20, 2), (30, 2), (40, 2)], 1)
    );
    // Step 4 (a block 16+ wide) skips them: one visit of length 4.
    assert_eq!(visits_row(&ctx, 0, 0, 4, 4, 2, 4), (vec![(10, 8)], 1));
    // A shorter walk window stops earlier.
    assert_eq!(
        visits_row(&ctx, 0, 0, 4, 2, 2, 1),
        (vec![(10, 2), (20, 2)], 1)
    );
}

#[test]
fn a_walk_that_runs_off_the_grid_ends_there() {
    let mut ctx = SpatialRefContext::new(4, 4);
    for x in 0..4 {
        put_1x1(&mut ctx, x, 0, x as i32 + 1);
    }
    // The window (8) is wider than the grid (4): the fifth neighbour does not exist.
    assert_eq!(
        visits_row(&ctx, 0, 0, 8, 8, 2, 1),
        (vec![(1, 2), (2, 2), (3, 2), (4, 2)], 1)
    );
    let mut ctx = SpatialRefContext::new(4, 4);
    for y in 0..4 {
        put_1x1(&mut ctx, 0, y, y as i32 + 1);
    }
    assert_eq!(
        visits_col(&ctx, 0, 0, 8, 8, 2, 1),
        (vec![(1, 2), (2, 2), (3, 2), (4, 2)], 1)
    );
}

#[test]
fn a_neighbour_as_wide_as_the_block_is_visited_once_with_a_height_weight() {
    let mut ctx = SpatialRefContext::new(8, 8);
    // 4x4 neighbour at the origin; the block below it is 2 wide.
    ctx.set_block(0, 0, 4, 4, 0, -1, false, mv_x(7), MotionVector::zero());
    // weight = height (4) capped by 2 * max_rows, at least 2; visit length = min(bw4, width) = 2.
    for (max_rows, weight) in [(0, 2), (1, 2), (2, 4), (3, 4), (9, 4)] {
        let (seen, ret) = visits_row(&ctx, 0, 0, 2, 2, max_rows, 1);
        assert_eq!(seen, vec![(7, 2 * weight)], "max_rows {max_rows}");
        assert_eq!(ret, weight as u32 >> 1, "max_rows {max_rows}");
    }
    // A block 1 wide always weighs 2, whatever the neighbour's height.
    assert_eq!(visits_row(&ctx, 0, 0, 1, 1, 9, 1), (vec![(7, 2)], 1));
    // Step 4 raises the length to 4.
    assert_eq!(visits_row(&ctx, 0, 0, 2, 2, 9, 4), (vec![(7, 16)], 2));
    // A neighbour exactly as wide as the block takes this branch too.
    assert_eq!(visits_row(&ctx, 0, 0, 4, 4, 9, 1), (vec![(7, 16)], 2));
    // One column wider than the neighbour does not.
    ctx.set_block(0, 0, 3, 3, 0, -1, false, mv_x(8), MotionVector::zero());
    assert_eq!(visits_row(&ctx, 0, 0, 4, 4, 9, 1).0[0], (8, 6));
}

#[test]
fn the_column_walk_is_the_row_walk_transposed() {
    let mut ctx = SpatialRefContext::new(8, 8);
    ctx.set_block(0, 0, 4, 4, 0, -1, false, mv_x(7), MotionVector::zero());
    for (max_cols, weight) in [(0, 2), (1, 2), (2, 4), (3, 4), (9, 4)] {
        let (seen, ret) = visits_col(&ctx, 0, 0, 2, 2, max_cols, 1);
        assert_eq!(seen, vec![(7, 2 * weight)], "max_cols {max_cols}");
        assert_eq!(ret, weight as u32 >> 1);
    }
    assert_eq!(visits_col(&ctx, 0, 0, 1, 1, 9, 1), (vec![(7, 2)], 1));
    assert_eq!(visits_col(&ctx, 0, 0, 2, 2, 9, 4), (vec![(7, 16)], 2));
    // A neighbour exactly as tall as the block takes the single-visit branch too, and one
    // row shorter does not.
    assert_eq!(visits_col(&ctx, 0, 0, 4, 4, 9, 1), (vec![(7, 16)], 2));
    ctx.set_block(0, 0, 3, 3, 0, -1, false, mv_x(8), MotionVector::zero());
    assert_eq!(visits_col(&ctx, 0, 0, 4, 4, 9, 1).0[0], (8, 6));

    let mut ctx = SpatialRefContext::new(4, 8);
    for y in 0..4 {
        put_1x1(&mut ctx, 0, y, 10 * (y as i32 + 1));
    }
    assert_eq!(
        visits_col(&ctx, 0, 0, 4, 4, 2, 1),
        (vec![(10, 2), (20, 2), (30, 2), (40, 2)], 1)
    );
    assert_eq!(visits_col(&ctx, 0, 0, 4, 4, 2, 4), (vec![(10, 8)], 1));
}

// ---- merge-by-value stack primitives ----

fn entry(x: i32, weight: i32) -> MvStackEntry {
    MvStackEntry {
        mv: mv_x(x),
        weight,
    }
}

#[test]
fn the_single_stack_merges_equal_vectors_and_holds_at_most_eight() {
    let mut stack = [MvStackEntry::default(); 8];
    let mut cnt = 0usize;
    for x in 1..=8 {
        SpatialRefContext::push_mv_candidate(&mut stack, &mut cnt, x * 10, mv_x(x));
    }
    assert_eq!(cnt, 8);
    // A ninth distinct vector does not fit and changes nothing.
    SpatialRefContext::push_mv_candidate(&mut stack, &mut cnt, 999, mv_x(9));
    assert_eq!(cnt, 8);
    assert!(stack.iter().all(|e| e.mv != mv_x(9) && e.weight != 999));
    // An equal vector still merges when the stack is full, anywhere in it.
    SpatialRefContext::push_mv_candidate(&mut stack, &mut cnt, 5, mv_x(8));
    SpatialRefContext::push_mv_candidate(&mut stack, &mut cnt, 7, mv_x(1));
    assert_eq!(cnt, 8);
    assert_eq!((stack[7].mv, stack[7].weight), (mv_x(8), 85));
    assert_eq!((stack[0].mv, stack[0].weight), (mv_x(1), 17));
    // Equality is on the whole vector, not on one component.
    let mut stack = [MvStackEntry::default(); 8];
    let mut cnt = 0;
    SpatialRefContext::push_mv_candidate(&mut stack, &mut cnt, 1, MotionVector::new(1, 2));
    SpatialRefContext::push_mv_candidate(&mut stack, &mut cnt, 1, MotionVector::new(1, 3));
    SpatialRefContext::push_mv_candidate(&mut stack, &mut cnt, 1, MotionVector::new(2, 2));
    assert_eq!(cnt, 3);
    let _ = entry(0, 0);
}

#[test]
fn the_compound_stack_merges_equal_pairs_and_holds_at_most_eight() {
    let mut stack = [CompoundMvStackEntry::default(); 8];
    let mut cnt = 0usize;
    for x in 1..=8 {
        SpatialRefContext::push_compound_mv_candidate(&mut stack, &mut cnt, x * 10, pair(x, -x));
    }
    assert_eq!(cnt, 8);
    SpatialRefContext::push_compound_mv_candidate(&mut stack, &mut cnt, 999, pair(9, -9));
    assert_eq!(cnt, 8);
    assert!(stack.iter().all(|e| e.weight != 999));
    SpatialRefContext::push_compound_mv_candidate(&mut stack, &mut cnt, 5, pair(8, -8));
    assert_eq!((stack[7].mv, stack[7].weight), (pair(8, -8), 85));
    // Both halves of the pair must be equal.
    let mut stack = [CompoundMvStackEntry::default(); 8];
    let mut cnt = 0;
    SpatialRefContext::push_compound_mv_candidate(&mut stack, &mut cnt, 1, pair(1, 2));
    SpatialRefContext::push_compound_mv_candidate(&mut stack, &mut cnt, 1, pair(1, 3));
    SpatialRefContext::push_compound_mv_candidate(&mut stack, &mut cnt, 1, pair(2, 2));
    SpatialRefContext::push_compound_mv_candidate(&mut stack, &mut cnt, 1, pair(2, 1));
    assert_eq!(cnt, 4);
}

// ---- stacks and contexts for blocks the grid does not contain ----

#[test]
fn queries_beyond_the_allocated_grid_find_nothing_and_do_not_panic() {
    let mut ctx = SpatialRefContext::new(4, 4);
    ctx.set_block(0, 0, 4, 4, 0, 4, false, mv_x(3), mv_x(4));
    for (x4, y4) in [(9, 9), (9, 2), (2, 9), (100, 100)] {
        let (_, single) = ctx.single_ref_mv_stack(x4, y4, 2, 2, 0, true);
        assert_eq!(single, 0, "single ({x4},{y4})");
        // The compound stack always pads to two entries.
        let (stack, compound) = ctx.compound_mv_stack(x4, y4, 2, 2, 0, 4, true);
        assert_eq!(compound, 2, "compound ({x4},{y4})");
        // Only padding: no neighbour weight in either entry.
        assert!(stack[..2].iter().all(|e| e.weight == 2));
        assert_eq!(ctx.inter_mode_context(x4, y4, 2, 2, 0, false), 0);
        assert_eq!(ctx.compound_mode_context(x4, y4, 2, 2, 0, 4), 0);
    }
}

#[test]
fn a_block_just_outside_the_grid_corner_still_sees_the_top_left_neighbour() {
    let mut ctx = SpatialRefContext::new(4, 4);
    ctx.set_block(0, 0, 4, 4, 0, -1, false, mv_x(3), MotionVector::zero());
    let (stack, cnt) = ctx.single_ref_mv_stack(4, 4, 2, 2, 0, false);
    assert_eq!(cnt, 1);
    assert_eq!(stack[0].mv, mv_x(3));
}

#[test]
fn a_negative_reference_never_reads_the_temporal_context() {
    use crate::tile::motion_field::{project_motion_field, MotionFieldState};
    let mut ctx = SpatialRefContext::new(16, 16);
    ctx.set_frame_extent(64, 64);
    let projected = project_motion_field(&[], &MotionFieldState::new(), 8, 8);
    ctx.set_temporal_context(projected, [1; 7]);
    // `pocdiff[ref0]` with ref0 = -1 would index out of bounds.
    let with_temporal = ctx.single_ref_mv_stack(4, 4, 2, 2, -1, true);
    let without = ctx.single_ref_mv_stack(4, 4, 2, 2, -1, false);
    assert_eq!(with_temporal.1, without.1);
}

// ---- drl contexts ----

#[test]
fn the_drl_context_is_a_function_of_the_two_weights_relative_to_640() {
    use crate::tile::context::get_drl_context;
    let single = |w0: i32, w1: i32| {
        let mut stack = [MvStackEntry::default(); 8];
        stack[0].weight = w0;
        stack[1].weight = w1;
        get_drl_context(&stack, 0)
    };
    let compound = |w0: i32, w1: i32| {
        let mut stack = [CompoundMvStackEntry::default(); 8];
        stack[0].weight = w0;
        stack[1].weight = w1;
        get_compound_drl_context(&stack, 0)
    };
    for (w0, w1, ctx) in [
        (640, 640, 0),
        (700, 1000, 0),
        (640, 639, 1),
        (1000, 0, 1),
        (639, 640, 0), // second in the nearest tier while the first is not
        (0, 1000, 0),
        (639, 639, 2),
        (0, 0, 2),
        (-5, 100, 2),
    ] {
        assert_eq!(single(w0, w1), ctx, "single {w0} {w1}");
        assert_eq!(compound(w0, w1), ctx, "compound {w0} {w1}");
    }
}

#[test]
fn the_drl_context_clamps_its_index_to_the_stack() {
    use crate::tile::context::get_drl_context;
    let mut stack = [MvStackEntry::default(); 8];
    stack[7].weight = 700;
    stack[6].weight = 10;
    // idx 7 and beyond compare the last entry with itself.
    for idx in [7, 8, 100] {
        assert_eq!(get_drl_context(&stack, idx), 0, "idx {idx}");
    }
    assert_eq!(
        get_drl_context(&stack, 6),
        0,
        "second entry in the nearest tier, first not"
    );
    stack[6].weight = 900;
    assert_eq!(get_drl_context(&stack, 6), 0);
    stack[7].weight = 5;
    assert_eq!(get_drl_context(&stack, 6), 1);
    stack[6].weight = 10;
    assert_eq!(get_drl_context(&stack, 6), 2);
    let mut stack = [CompoundMvStackEntry::default(); 8];
    stack[7].weight = 100;
    for idx in [7, 8, 100] {
        assert_eq!(get_compound_drl_context(&stack, idx), 2, "idx {idx}");
    }
    // The clamp is to the last entry (7), not the one before it.
    let mut stack = [MvStackEntry::default(); 8];
    stack[6].weight = 700;
    stack[7].weight = 10;
    assert_eq!(get_drl_context(&stack, 6), 1);
    for idx in [7, 8, 100] {
        assert_eq!(get_drl_context(&stack, idx), 2, "single idx {idx}");
    }
    let mut stack = [CompoundMvStackEntry::default(); 8];
    stack[6].weight = 700;
    stack[7].weight = 10;
    for idx in [7, 8, 100] {
        assert_eq!(
            get_compound_drl_context(&stack, idx),
            2,
            "compound idx {idx}"
        );
    }
}
