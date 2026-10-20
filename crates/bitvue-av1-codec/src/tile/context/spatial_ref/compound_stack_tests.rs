//! `SpatialRefContext::compound_mv_stack`, one scan stage at a time (top-right, top-left,
//! edges of the tile), plus seeded sweeps whose combined result is pinned.

use super::*;
use crate::tile::coding_unit::MotionVector;

const PAIR: (i8, i8) = (0, 4);

fn grid() -> SpatialRefContext {
    SpatialRefContext::new(32, 32)
}

fn mv(x: i32, y: i32) -> [MotionVector; 2] {
    [MotionVector::new(x, y), MotionVector::new(-x, y + 1)]
}

fn put(ctx: &mut SpatialRefContext, x4: u32, y4: u32, w4: u32, h4: u32, m: [MotionVector; 2]) {
    ctx.set_block(x4, y4, w4, h4, PAIR.0, PAIR.1, false, m[0], m[1]);
}

fn stack(
    ctx: &SpatialRefContext,
    x4: u32,
    y4: u32,
    bw4: u32,
    bh4: u32,
) -> Vec<([MotionVector; 2], i32)> {
    let (s, cnt) = ctx.compound_mv_stack(x4, y4, bw4, bh4, PAIR.0, PAIR.1, false);
    s[..cnt].iter().map(|e| (e.mv, e.weight)).collect()
}

#[test]
fn a_block_at_the_tile_origin_only_gets_the_padding() {
    let mut ctx = grid();
    put(&mut ctx, 0, 0, 4, 4, mv(8, 8));
    let s = stack(&ctx, 0, 0, 4, 4);
    assert_eq!(s.len(), 2);
    assert!(s
        .iter()
        .all(|(m, w)| *w == 2 && *m == [MotionVector::zero(); 2]));
}

#[test]
fn the_row_above_is_a_nearest_candidate_with_weight_from_its_width() {
    let mut ctx = grid();
    put(&mut ctx, 8, 7, 4, 1, mv(8, 16));
    let s = stack(&ctx, 8, 8, 4, 4);
    assert_eq!(s[0], (mv(8, 16), 640 + 8));
}

#[test]
fn the_column_to_the_left_is_a_nearest_candidate_with_weight_from_its_height() {
    let mut ctx = grid();
    put(&mut ctx, 7, 8, 1, 4, mv(24, 16));
    let s = stack(&ctx, 8, 8, 4, 4);
    assert_eq!(s[0], (mv(24, 16), 640 + 8));
}

#[test]
fn the_top_right_cell_is_a_nearest_candidate_with_weight_4() {
    let mut ctx = grid();
    put(&mut ctx, 12, 7, 1, 1, mv(40, 8));
    let s = stack(&ctx, 8, 8, 4, 4);
    assert_eq!(s[0], (mv(40, 8), 640 + 4));
}

#[test]
fn the_top_right_cell_is_used_up_to_16_units_and_ignored_beyond() {
    let mut ctx = grid();
    put(&mut ctx, 24, 7, 1, 1, mv(40, 8));
    assert_eq!(stack(&ctx, 8, 8, 16, 16)[0].1, 644);
    for (bw4, bh4) in [(16, 17), (17, 16)] {
        assert!(stack(&ctx, 8, 8, bw4, bh4).iter().all(|(_, w)| *w == 2));
    }
}

#[test]
fn the_top_right_cell_is_ignored_at_the_right_edge_of_the_tile() {
    let mut ctx = grid();
    ctx.set_tile_extent(0, 28, 0, 32); // the tile ends where the block does
    put(&mut ctx, 28, 7, 1, 1, mv(40, 8));
    let s = stack(&ctx, 24, 8, 4, 4);
    assert!(s.iter().all(|(_, w)| *w == 2));
}

#[test]
fn the_top_left_cell_is_a_secondary_candidate_with_weight_4() {
    let mut ctx = grid();
    put(&mut ctx, 7, 7, 1, 1, mv(16, 16));
    let s = stack(&ctx, 8, 8, 4, 4);
    assert_eq!(s[0], (mv(16, 16), 4));
}

#[test]
fn the_top_left_cell_needs_both_a_row_and_a_column_inside_the_tile() {
    let mut ctx = grid();
    ctx.set_tile_extent(8, 32, 0, 32); // x4 = 8 is the tile's first column
    put(&mut ctx, 7, 7, 1, 1, mv(16, 16));
    assert!(stack(&ctx, 8, 8, 4, 4).iter().all(|(_, w)| *w == 2));
}

#[test]
fn nearest_candidates_stay_ahead_of_heavier_secondary_ones() {
    let mut ctx = grid();
    put(&mut ctx, 8, 7, 1, 1, mv(8, 8)); // nearest, light
    put(&mut ctx, 9, 6, 4, 1, mv(32, 8)); // secondary row, heavier base
    let s = stack(&ctx, 8, 8, 4, 4);
    assert_eq!(s[0].0, mv(8, 8));
    assert!(s[0].1 >= 640);
}

#[test]
fn candidates_are_clamped_to_the_frame() {
    let mut ctx = SpatialRefContext::new(16, 16);
    ctx.set_frame_extent(36, 36);
    put(
        &mut ctx,
        0,
        0,
        4,
        1,
        [
            MotionVector::new(10_000, -10_000),
            MotionVector::new(-10_000, 10_000),
        ],
    );
    let s = stack(&ctx, 0, 1, 2, 2);
    assert_eq!(s[0].0[0], MotionVector::new(448, -(1 + 2 + 4) * 32));
}

fn next(state: &mut u64) -> u32 {
    *state = state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    (*state >> 33) as u32
}

fn random_mv(state: &mut u64) -> MotionVector {
    MotionVector::new(
        (next(state) % 400) as i32 - 200,
        (next(state) % 400) as i32 - 200,
    )
}

fn sweep(seed: u64, grid_size: u32, pair_bias: u32) -> u64 {
    let mut state = seed;
    let mut hash = 0xcbf29ce484222325u64; // FNV-1a over every stack entry
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
            let (r0, r1) = if next(&mut state) % pair_bias == 0 {
                (PAIR.0, PAIR.1)
            } else {
                (
                    (next(&mut state) % 7) as i8,
                    (next(&mut state) % 7) as i8 - 1,
                )
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
            let (s, cnt) = ctx.compound_mv_stack(x, y, bw, bh, PAIR.0, PAIR.1, false);
            fold(cnt as u64);
            for e in &s[..cnt] {
                for m in e.mv {
                    fold(m.x as u32 as u64);
                    fold(m.y as u32 as u64);
                }
                fold(e.weight as u32 as u64);
            }
        }
    }
    hash
}

#[test]
fn a_seeded_sweep_of_mostly_matching_neighbours_keeps_its_recorded_result() {
    assert_eq!(sweep(0x5eed, 24, 2), 15001382876372821624);
}

#[test]
fn a_seeded_sweep_of_mostly_foreign_neighbours_keeps_its_recorded_result() {
    // Few exact pair matches, so the `cnt < 2` extended-candidate fallback runs often.
    assert_eq!(sweep(0xfeed, 32, 9), 6034536328084987551);
}
