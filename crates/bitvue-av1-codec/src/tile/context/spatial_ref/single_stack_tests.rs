//! `SpatialRefContext::single_ref_mv_stack`, one scan stage at a time (top row, left column,
//! top-right, top-left, secondary rows, other-reference fallback, frame clamp), plus seeded
//! sweeps whose combined result is pinned.

use super::*;
use crate::tile::coding_unit::MotionVector;

const REF: i8 = 0;

fn grid() -> SpatialRefContext {
    SpatialRefContext::new(32, 32)
}

fn mv(x: i32, y: i32) -> MotionVector {
    MotionVector::new(x, y)
}

fn put(ctx: &mut SpatialRefContext, x4: u32, y4: u32, w4: u32, h4: u32, r: i8, m: MotionVector) {
    ctx.set_block(x4, y4, w4, h4, r, -1, false, m, MotionVector::zero());
}

fn stack(
    ctx: &SpatialRefContext,
    x4: u32,
    y4: u32,
    bw4: u32,
    bh4: u32,
) -> Vec<(MotionVector, i32)> {
    let (s, cnt) = ctx.single_ref_mv_stack(x4, y4, bw4, bh4, REF, false);
    s[..cnt].iter().map(|e| (e.mv, e.weight)).collect()
}

#[test]
fn a_block_at_the_tile_origin_has_no_candidates() {
    let mut ctx = grid();
    put(&mut ctx, 0, 0, 4, 4, REF, mv(8, 8));
    let (s, cnt) = ctx.single_ref_mv_stack(0, 0, 4, 4, REF, false);
    assert_eq!(cnt, 0);
    assert!(s.iter().all(|e| e.mv == MotionVector::zero()));
}

#[test]
fn the_row_above_is_a_nearest_candidate_with_weight_from_its_width() {
    let mut ctx = grid();
    put(&mut ctx, 8, 7, 4, 1, REF, mv(8, 16));
    assert_eq!(stack(&ctx, 8, 8, 4, 4), vec![(mv(8, 16), 640 + 8)]);
}

#[test]
fn the_column_to_the_left_is_a_nearest_candidate_with_weight_from_its_height() {
    let mut ctx = grid();
    put(&mut ctx, 7, 8, 1, 4, REF, mv(24, 16));
    assert_eq!(stack(&ctx, 8, 8, 4, 4), vec![(mv(24, 16), 640 + 8)]);
}

#[test]
fn equal_vectors_from_the_row_and_column_add_their_weights() {
    let mut ctx = grid();
    put(&mut ctx, 8, 7, 4, 1, REF, mv(8, 16));
    put(&mut ctx, 7, 8, 1, 4, REF, mv(8, 16));
    assert_eq!(stack(&ctx, 8, 8, 4, 4), vec![(mv(8, 16), 640 + 16)]);
}

#[test]
fn the_top_right_cell_is_a_nearest_candidate_with_weight_4() {
    let mut ctx = grid();
    put(&mut ctx, 12, 7, 1, 1, REF, mv(40, 8));
    assert_eq!(stack(&ctx, 8, 8, 4, 4), vec![(mv(40, 8), 640 + 4)]);
}

#[test]
fn the_top_right_cell_is_used_up_to_16_units_and_ignored_beyond() {
    let mut ctx = grid();
    put(&mut ctx, 24, 7, 1, 1, REF, mv(40, 8));
    assert_eq!(stack(&ctx, 8, 8, 16, 16)[0].1, 644);
    for (bw4, bh4) in [(16, 17), (17, 16)] {
        assert!(stack(&ctx, 8, 8, bw4, bh4).is_empty());
    }
}

#[test]
fn the_top_right_cell_is_ignored_at_the_right_edge_of_the_tile() {
    let mut ctx = grid();
    ctx.set_tile_extent(0, 28, 0, 32);
    put(&mut ctx, 28, 7, 1, 1, REF, mv(40, 8));
    assert!(stack(&ctx, 24, 8, 4, 4).is_empty());
}

#[test]
fn the_top_left_cell_is_a_secondary_candidate_with_weight_4() {
    let mut ctx = grid();
    put(&mut ctx, 7, 7, 1, 1, REF, mv(16, 16));
    assert_eq!(stack(&ctx, 8, 8, 4, 4), vec![(mv(16, 16), 4)]);
}

#[test]
fn the_top_left_cell_needs_both_a_row_and_a_column_inside_the_tile() {
    let mut ctx = grid();
    ctx.set_tile_extent(8, 32, 0, 32);
    put(&mut ctx, 7, 7, 1, 1, REF, mv(16, 16));
    assert!(stack(&ctx, 8, 8, 4, 4).is_empty());
}

#[test]
fn nearest_candidates_stay_ahead_of_heavier_secondary_ones() {
    let mut ctx = grid();
    put(&mut ctx, 8, 7, 1, 1, REF, mv(8, 8)); // nearest, light
    put(&mut ctx, 9, 6, 4, 1, REF, mv(32, 8)); // secondary row, heavier
    let s = stack(&ctx, 8, 8, 4, 4);
    assert_eq!(s[0].0, mv(8, 8));
    assert!(s[0].1 >= 640);
}

#[test]
fn neighbours_using_another_reference_fill_in_with_weight_2_when_fewer_than_two_were_found() {
    let mut ctx = grid();
    put(&mut ctx, 8, 7, 4, 1, 3, mv(8, 16));
    assert_eq!(stack(&ctx, 8, 8, 4, 4), vec![(mv(8, 16), 2)]);
}

#[test]
fn the_other_reference_vector_is_negated_when_the_sign_bias_differs() {
    let mut ctx = grid();
    let mut bias = [false; 7];
    bias[3] = true;
    ctx.set_sign_bias(bias);
    put(&mut ctx, 8, 7, 4, 1, 3, mv(8, 16));
    assert_eq!(stack(&ctx, 8, 8, 4, 4), vec![(mv(-8, -16), 2)]);
}

#[test]
fn other_reference_neighbours_are_not_used_once_two_candidates_exist() {
    let mut ctx = grid();
    put(&mut ctx, 8, 7, 2, 1, REF, mv(8, 16));
    put(&mut ctx, 10, 7, 2, 1, REF, mv(16, 16));
    put(&mut ctx, 7, 8, 1, 4, 3, mv(99, 99));
    let s = stack(&ctx, 8, 8, 4, 4);
    assert_eq!(s.len(), 2);
    assert!(s.iter().all(|(m, _)| *m != mv(99, 99)));
}

#[test]
fn candidates_are_clamped_to_the_frame() {
    let mut ctx = SpatialRefContext::new(16, 16);
    ctx.set_frame_extent(36, 36);
    put(&mut ctx, 0, 0, 4, 1, REF, mv(10_000, -10_000));
    let s = stack(&ctx, 0, 1, 2, 2);
    assert_eq!(s[0].0, mv(448, -(1 + 2 + 4) * 32));
}

#[test]
fn asking_for_temporal_candidates_without_a_temporal_context_changes_nothing() {
    let mut ctx = grid();
    put(&mut ctx, 8, 7, 4, 1, REF, mv(8, 16));
    let with = ctx.single_ref_mv_stack(8, 8, 4, 4, REF, true);
    let without = ctx.single_ref_mv_stack(8, 8, 4, 4, REF, false);
    assert_eq!(with.1, without.1);
    for i in 0..with.1 {
        assert_eq!(
            (with.0[i].mv, with.0[i].weight),
            (without.0[i].mv, without.0[i].weight)
        );
    }
}

fn next(state: &mut u64) -> u32 {
    *state = state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    (*state >> 33) as u32
}

fn random_mv(state: &mut u64) -> MotionVector {
    mv(
        (next(state) % 400) as i32 - 200,
        (next(state) % 400) as i32 - 200,
    )
}

fn sweep(seed: u64, grid_size: u32, same_ref_bias: u32) -> u64 {
    let mut state = seed;
    let mut hash = 0xcbf29ce484222325u64;
    let mut fold = |v: u64| {
        hash ^= v;
        hash = hash.wrapping_mul(0x100000001b3);
    };
    for _ in 0..300 {
        let mut ctx = SpatialRefContext::new(grid_size, grid_size);
        if next(&mut state).is_multiple_of(3) {
            ctx.set_tile_extent(
                next(&mut state) % 6,
                12 + next(&mut state) % 13,
                next(&mut state) % 6,
                12 + next(&mut state) % 13,
            );
        }
        let mut bias = [false; 7];
        for b in &mut bias {
            *b = next(&mut state).is_multiple_of(2);
        }
        ctx.set_sign_bias(bias);
        for _ in 0..(10 + next(&mut state) % 30) {
            let (w, h) = (1 << (next(&mut state) % 5), 1 << (next(&mut state) % 5));
            let (x, y) = (next(&mut state) % grid_size, next(&mut state) % grid_size);
            if next(&mut state).is_multiple_of(5) {
                ctx.set_intra_block(x, y, w, h);
                continue;
            }
            let r0 = if next(&mut state).is_multiple_of(same_ref_bias) {
                REF
            } else {
                (next(&mut state) % 7) as i8
            };
            let r1 = if next(&mut state).is_multiple_of(4) {
                (next(&mut state) % 7) as i8
            } else {
                -1
            };
            let (m0, m1) = (random_mv(&mut state), random_mv(&mut state));
            ctx.set_block(x, y, w, h, r0, r1, false, m0, m1);
        }
        for _ in 0..30 {
            let (x, y) = (
                next(&mut state) % (grid_size + 2),
                next(&mut state) % (grid_size + 2),
            );
            let (bw, bh) = (1 << (next(&mut state) % 6), 1 << (next(&mut state) % 6));
            let (s, cnt) = ctx.single_ref_mv_stack(x, y, bw, bh, REF, false);
            fold(cnt as u64);
            for e in &s {
                fold(e.mv.x as u32 as u64);
                fold(e.mv.y as u32 as u64);
                fold(e.weight as u32 as u64);
            }
        }
    }
    hash
}

#[test]
fn a_seeded_sweep_of_mostly_matching_neighbours_keeps_its_recorded_result() {
    assert_eq!(sweep(0x5eed, 24, 2), 18119365424757920922);
}

#[test]
fn a_seeded_sweep_of_mostly_foreign_neighbours_keeps_its_recorded_result() {
    // Few same-reference neighbours, so the `cnt < 2` other-reference fallback runs often.
    assert_eq!(sweep(0xfeed, 32, 9), 3620011355296082117);
}
