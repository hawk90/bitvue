//! `SpatialRefContext::scan` (the neighbour scan behind the `inter_mode`/`compound_mode`
//! contexts), one scan stage at a time, plus a seeded sweep whose combined result is pinned.

use super::*;
use crate::tile::coding_unit::MotionVector;

const MV: fn() -> MotionVector = MotionVector::zero;

fn grid() -> SpatialRefContext {
    SpatialRefContext::new(32, 32)
}

fn put(ctx: &mut SpatialRefContext, x4: u32, y4: u32, w4: u32, h4: u32, newmv: bool) {
    ctx.set_block(x4, y4, w4, h4, 0, -1, newmv, MV(), MV());
}

#[test]
fn an_empty_grid_has_no_matches() {
    assert_eq!(grid().scan(8, 8, 4, 4, 0, -1), (0, 0, false));
}

#[test]
fn a_block_at_the_tile_origin_scans_nothing() {
    let mut ctx = grid();
    put(&mut ctx, 0, 0, 4, 4, true);
    assert_eq!(ctx.scan(0, 0, 4, 4, 0, -1), (0, 0, false));
}

#[test]
fn the_row_above_counts_once_and_carries_newmv() {
    let mut ctx = grid();
    put(&mut ctx, 8, 7, 4, 1, true);
    assert_eq!(ctx.scan(8, 8, 4, 4, 0, -1), (1, 1, true));
    let mut ctx = grid();
    put(&mut ctx, 8, 7, 4, 1, false);
    assert_eq!(ctx.scan(8, 8, 4, 4, 0, -1), (1, 1, false));
}

#[test]
fn the_column_to_the_left_counts_once_and_carries_newmv() {
    let mut ctx = grid();
    put(&mut ctx, 7, 8, 1, 4, true);
    assert_eq!(ctx.scan(8, 8, 4, 4, 0, -1), (1, 1, true));
}

#[test]
fn row_and_column_matches_add_up_to_two() {
    let mut ctx = grid();
    put(&mut ctx, 8, 7, 4, 1, false);
    put(&mut ctx, 7, 8, 1, 4, false);
    assert_eq!(ctx.scan(8, 8, 4, 4, 0, -1), (2, 2, false));
}

#[test]
fn newmv_from_either_side_is_enough() {
    let mut ctx = grid();
    put(&mut ctx, 8, 7, 4, 1, false);
    put(&mut ctx, 7, 8, 1, 4, true);
    assert_eq!(ctx.scan(8, 8, 4, 4, 0, -1), (2, 2, true));
}

#[test]
fn the_top_right_cell_counts_as_a_row_match_and_feeds_newmv() {
    let mut ctx = grid();
    put(&mut ctx, 12, 7, 1, 1, true); // just right of the block, one row up
    assert_eq!(ctx.scan(8, 8, 4, 4, 0, -1), (1, 1, true));
}

#[test]
fn the_top_right_cell_is_used_up_to_16_units_and_ignored_beyond() {
    let mut ctx = grid();
    put(&mut ctx, 24, 7, 1, 1, true);
    assert_eq!(ctx.scan(8, 8, 16, 16, 0, -1), (1, 1, true));
    assert_eq!(ctx.scan(8, 8, 16, 17, 0, -1), (0, 0, false));
    assert_eq!(ctx.scan(8, 8, 17, 16, 0, -1), (0, 0, false));
}

#[test]
fn the_top_right_cell_is_ignored_at_the_right_edge_of_the_tile() {
    let mut ctx = grid();
    ctx.set_tile_extent(0, 12, 0, 32);
    put(&mut ctx, 12, 7, 1, 1, true);
    assert_eq!(ctx.scan(8, 8, 4, 4, 0, -1), (0, 0, false));
}

#[test]
fn the_top_left_cell_counts_for_the_total_but_not_for_nearest_or_newmv() {
    let mut ctx = grid();
    put(&mut ctx, 7, 7, 1, 1, true);
    assert_eq!(ctx.scan(8, 8, 4, 4, 0, -1), (0, 1, false));
}

#[test]
fn the_top_left_cell_needs_both_a_row_and_a_column_inside_the_tile() {
    let mut ctx = grid();
    ctx.set_tile_extent(8, 32, 0, 32);
    put(&mut ctx, 7, 7, 1, 1, false);
    assert_eq!(ctx.scan(8, 8, 4, 4, 0, -1), (0, 0, false));
    let mut ctx = grid();
    ctx.set_tile_extent(0, 32, 8, 32);
    put(&mut ctx, 7, 7, 1, 1, false);
    assert_eq!(ctx.scan(8, 8, 4, 4, 0, -1), (0, 0, false));
}

#[test]
fn the_secondary_row_scan_finds_a_match_two_8x8_rows_up_without_newmv() {
    let mut ctx = grid();
    put(&mut ctx, 8, 5, 4, 1, true); // row (8 + 1 - 4) | 1 = 5
    assert_eq!(ctx.scan(8, 8, 4, 4, 0, -1), (0, 1, false));
}

#[test]
fn the_secondary_column_scan_finds_a_match_two_8x8_columns_left_without_newmv() {
    let mut ctx = grid();
    put(&mut ctx, 5, 8, 1, 4, true);
    assert_eq!(ctx.scan(8, 8, 4, 4, 0, -1), (0, 1, false));
}

#[test]
fn the_third_secondary_row_is_reached_from_eight_units_down() {
    let mut ctx = grid();
    put(&mut ctx, 8, 3, 4, 1, false); // n = 3: (8 + 1 - 6) | 1 = 3
    assert_eq!(ctx.scan(8, 8, 4, 4, 0, -1), (0, 1, false));
}

#[test]
fn a_block_two_units_down_has_no_secondary_rows() {
    let mut ctx = grid();
    put(&mut ctx, 8, 0, 4, 1, false);
    assert_eq!(ctx.scan(8, 2, 4, 4, 0, -1), (0, 0, false));
}

#[test]
fn matches_need_the_query_reference_and_compound_needs_the_exact_pair() {
    let mut ctx = grid();
    ctx.set_block(8, 7, 4, 1, 2, 5, false, MV(), MV());
    assert_eq!(ctx.scan(8, 8, 4, 4, 2, -1), (1, 1, false)); // either slot of a compound cell
    assert_eq!(ctx.scan(8, 8, 4, 4, 5, -1), (1, 1, false));
    assert_eq!(ctx.scan(8, 8, 4, 4, 3, -1), (0, 0, false));
    assert_eq!(ctx.scan(8, 8, 4, 4, 2, 5), (1, 1, false));
    assert_eq!(ctx.scan(8, 8, 4, 4, 5, 2), (0, 0, false));
}

#[test]
fn the_block_extent_is_clamped_to_the_frame_edge_and_16_units() {
    let mut ctx = grid();
    put(&mut ctx, 31, 7, 1, 1, false);
    // Starts at the last column: only that column is scanned, whatever the block width.
    assert_eq!(ctx.scan(31, 8, 64, 4, 0, -1), (1, 1, false));
    let mut ctx = grid();
    put(&mut ctx, 20, 7, 1, 1, false);
    // 16 units from x4 = 0 end at 15: a match at 20 is out of reach.
    assert_eq!(ctx.scan(0, 8, 64, 4, 0, -1), (0, 0, false));
}

fn next(state: &mut u64) -> u32 {
    *state = state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    (*state >> 33) as u32
}

#[test]
fn a_seeded_sweep_of_random_grids_and_queries_keeps_its_recorded_result() {
    let mut state = 0x5eed_u64;
    let mut hash = 0xcbf29ce484222325u64; // FNV-1a over every (nearest, count, newmv)
    let mut fold = |v: u64| {
        hash ^= v;
        hash = hash.wrapping_mul(0x100000001b3);
    };
    for _ in 0..300 {
        let mut ctx = SpatialRefContext::new(24, 24);
        if next(&mut state).is_multiple_of(3) {
            let c0 = next(&mut state) % 6;
            let r0 = next(&mut state) % 6;
            ctx.set_tile_extent(
                c0,
                12 + next(&mut state) % 13,
                r0,
                12 + next(&mut state) % 13,
            );
        }
        for _ in 0..(10 + next(&mut state) % 30) {
            let (w, h) = (1 << (next(&mut state) % 5), 1 << (next(&mut state) % 5));
            let (x, y) = (next(&mut state) % 24, next(&mut state) % 24);
            let (r0, r1) = (
                (next(&mut state) % 7) as i8,
                [-1, -1, (next(&mut state) % 7) as i8][(next(&mut state) % 3) as usize],
            );
            if next(&mut state).is_multiple_of(5) {
                ctx.set_intra_block(x, y, w, h);
            } else {
                ctx.set_block(
                    x,
                    y,
                    w,
                    h,
                    r0,
                    r1,
                    next(&mut state).is_multiple_of(2),
                    MV(),
                    MV(),
                );
            }
        }
        for _ in 0..40 {
            let (x, y) = (next(&mut state) % 26, next(&mut state) % 26);
            let (bw, bh) = (1 << (next(&mut state) % 6), 1 << (next(&mut state) % 6));
            let r0 = (next(&mut state) % 7) as i8;
            let r1 = [-1, -1, (next(&mut state) % 7) as i8][(next(&mut state) % 3) as usize];
            let (a, b, c) = ctx.scan(x, y, bw, bh, r0, r1);
            fold(u64::from(a) | u64::from(b) << 8 | u64::from(c) << 16);
        }
    }
    assert_eq!(hash, 3988648800859680722);
}

#[test]
fn a_seeded_sweep_of_tall_and_wide_neighbours_keeps_its_recorded_result() {
    // Neighbours taller/wider than the scan's weight bound, overwritten in part by smaller ones,
    // queried by small blocks: exercises the weight limits that gate the secondary scans.
    let mut state = 0xfeed_u64;
    let mut hash = 0xcbf29ce484222325u64;
    let mut fold = |v: u64| {
        hash ^= v;
        hash = hash.wrapping_mul(0x100000001b3);
    };
    for _ in 0..400 {
        let mut ctx = SpatialRefContext::new(32, 32);
        for _ in 0..(6 + next(&mut state) % 10) {
            let (w, h) = (1 << (next(&mut state) % 5), 1 << (2 + next(&mut state) % 3));
            let (x, y) = (next(&mut state) % 32, next(&mut state) % 32);
            ctx.set_block(
                x,
                y,
                w,
                h,
                (next(&mut state) % 3) as i8,
                -1,
                false,
                MV(),
                MV(),
            );
            ctx.set_block(
                x,
                y,
                w,
                1,
                (next(&mut state) % 3) as i8,
                -1,
                false,
                MV(),
                MV(),
            );
            let (w2, h2) = (1 << (next(&mut state) % 3), 1 << (next(&mut state) % 5));
            ctx.set_block(
                x,
                y,
                w2,
                h2,
                (next(&mut state) % 3) as i8,
                -1,
                false,
                MV(),
                MV(),
            );
        }
        for _ in 0..60 {
            let (x, y) = (8 + next(&mut state) % 20, 8 + next(&mut state) % 20);
            let (bw, bh) = (1 << (next(&mut state) % 4), 1 << (next(&mut state) % 4));
            let (a, b, c) = ctx.scan(x, y, bw, bh, (next(&mut state) % 3) as i8, -1);
            fold(u64::from(a) | u64::from(b) << 8 | u64::from(c) << 16);
        }
    }
    assert_eq!(hash, 14325133273655144885);
}
