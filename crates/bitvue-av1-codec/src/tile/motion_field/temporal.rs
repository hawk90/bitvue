//! Temporal candidates a block takes from the projected field.

use super::project::{mv_projection, ProjectedMotionField};
use crate::tile::coding_unit::MotionVector;

/// dav1d `add_temporal_candidate` + its call site's main grid scan (`refmvs.c:193-237`,
/// `416-431`) -- the *second* (and only value-changing) `mv_projection`, using the current
/// block's own ref's `pocdiff` as numerator against the cell's stored `ref2ref` denominator.
/// Returns weight-2 candidates (spec: real dav1d dedups exact-MV matches by bumping weight
/// instead of pushing a duplicate -- callers should do the same, matching this crate's existing
/// `single_ref_mv_stack`/`push_mv_candidate` dedup behavior for spatial candidates).
///
/// `x8_start`/`y8_start`/`w8`/`h8`: the current block's footprint in 8x8 units (`by4>>1`/`bx4>>1`/
/// `imin((w4+1)>>1,8)`/`imin((h4+1)>>1,8)`, `refmvs.c:420-424`). `step_h`/`step_v`: `2` for blocks
/// >=16 4x4-units wide/tall, else `1` (`refmvs.c:423`).
///
/// Block geometry for [`add_temporal_candidates`], in 4x4 units (dav1d `dav1d_refmvs_find`'s
/// `bx4`/`by4`/`bw4`/`bh4`, the tile-clipped `w4`/`h4` and the tile end).
#[derive(Debug, Clone, Copy)]
pub struct TemporalBlock {
    pub bx4: u32,
    pub by4: u32,
    pub bw4: u32,
    pub bh4: u32,
    pub w4: u32,
    pub h4: u32,
    pub col_start: u32,
    pub col_end: u32,
    pub row_end: u32,
}

/// dav1d `fix_mv_precision`: rounds a projected MV to the frame's MV precision.
pub fn fix_mv_precision(
    mv: MotionVector,
    allow_high_precision_mv: bool,
    force_integer_mv: bool,
) -> MotionVector {
    let neg = |v: i32| i32::from(v < 0);
    if force_integer_mv {
        MotionVector {
            x: (mv.x + neg(mv.x) + 3) & !7,
            y: (mv.y + neg(mv.y) + 3) & !7,
        }
    } else if !allow_high_precision_mv {
        MotionVector {
            x: (mv.x + neg(mv.x)) & !1,
            y: (mv.y + neg(mv.y)) & !1,
        }
    } else {
        mv
    }
}

/// Every temporal sample dav1d takes for a block (`refmvs.c:416-452`): the main grid over the
/// block's own footprint, then -- for blocks with `2 <= min(bw4,bh4)` and `max(bw4,bh4) < 16` --
/// up to three more just below and to the right (clamped to the tile and the current 64x64).
/// Returns the projected MVs in sample order, each with whether it is the block's own top-left
/// sample (the one that feeds `globalmv_ctx`).
pub fn add_temporal_candidates(
    projected: &ProjectedMotionField,
    pocdiff_ref0: i32,
    block: TemporalBlock,
) -> Vec<(MotionVector, bool)> {
    temporal_sample_positions(block)
        .into_iter()
        .filter_map(|(x8, y8, first)| {
            projected
                .get(x8, y8)
                .map(|cell| (mv_projection(cell.mv, pocdiff_ref0, cell.ref2ref), first))
        })
        .collect()
}

/// The 8x8-grid cells dav1d samples for a block (`refmvs.c:416-452`), as `(x8, y8, is_the_block's_
/// own_top_left_sample)`: the main grid over the block's footprint, then -- for blocks with
/// `2 <= min(bw4,bh4)` and `max(bw4,bh4) < 16` -- up to three more just below and to the right
/// (clamped to the tile and the current 64x64).
fn temporal_sample_positions(block: TemporalBlock) -> Vec<(u32, u32, bool)> {
    let TemporalBlock {
        bx4,
        by4,
        bw4,
        bh4,
        w4,
        h4,
        col_start,
        col_end,
        row_end,
    } = block;
    let (bx8, by8) = (bx4 >> 1, by4 >> 1);
    let step_h = if bw4 >= 16 { 2 } else { 1 };
    let step_v = if bh4 >= 16 { 2 } else { 1 };
    let w8 = ((w4 + 1) >> 1).min(8);
    let h8 = ((h4 + 1) >> 1).min(8);
    let mut out = Vec::new();
    let mut y = 0;
    while y < h8 {
        let mut x = 0;
        while x < w8 {
            out.push((bx8 + x, by8 + y, x == 0 && y == 0));
            x += step_h;
        }
        y += step_v;
    }
    if bw4.min(bh4) >= 2 && bw4.max(bh4) < 16 {
        let (bh8, bw8) = (bh4 >> 1, bw4 >> 1);
        let has_bottom = by8 + bh8 < (row_end >> 1).min((by8 & !7) + 8);
        if has_bottom && bx8 > (col_start >> 1).max(bx8 & !7) {
            out.push((bx8 - 1, by8 + bh8, false));
        }
        if bx8 + bw8 < (col_end >> 1).min((bx8 & !7) + 8) {
            if has_bottom {
                out.push((bx8 + bw8, by8 + bh8, false));
            }
            if by8 + bh8 - 1 < (row_end >> 1).min((by8 & !7) + 8) {
                out.push((bx8 + bw8, by8 + bh8 - 1, false));
            }
        }
    }
    out
}

/// Compound counterpart of [`add_temporal_candidates`] -- dav1d `add_temporal_candidate`'s
/// `ref.ref[1] != -1` branch: the *same* grid cell is projected against **both** references'
/// `pocdiff` and pushed as one joint pair. Same sample positions as the single-reference case.
pub fn add_temporal_compound_candidates(
    projected: &ProjectedMotionField,
    pocdiff_ref0: i32,
    pocdiff_ref1: i32,
    block: TemporalBlock,
) -> Vec<[MotionVector; 2]> {
    temporal_sample_positions(block)
        .into_iter()
        .filter_map(|(x8, y8, _)| {
            projected.get(x8, y8).map(|cell| {
                [
                    mv_projection(cell.mv, pocdiff_ref0, cell.ref2ref),
                    mv_projection(cell.mv, pocdiff_ref1, cell.ref2ref),
                ]
            })
        })
        .collect()
}
