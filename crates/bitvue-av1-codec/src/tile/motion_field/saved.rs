//! The motion vectors a frame saves for later frames (spec 7.9 storage).

use crate::overlay_extraction::cu_spatial_index::CuSpatialIndex;
use crate::tile::coding_unit::{CodingUnit, MotionVector, RefFrame};

/// One saved candidate per 8x8 luma unit -- mirrors dav1d's `refmvs_temporal_block` (`refmvs.h:43-47`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SavedMv {
    pub mv: MotionVector,
    /// Real AV1 ref-frame value, `1..=7` (`RefFrame::Last..=RefFrame::AltRef`) -- never
    /// `RefFrame::Intra`, matching dav1d's `ref: u8` which is only ever written for inter blocks
    /// (`save_tmvs_c`, `refmvs.c:794-798` stores `ref = 0` i.e. "empty" otherwise, modeled here as
    /// `None` in the containing grid instead of a sentinel ref value).
    pub ref_frame: RefFrame,
}

/// 8x8-luma-unit-resolution grid, one [`SavedMv`] (or `None` = empty) per cell -- dav1d's
/// `rf->rp`/`rf->rp_proj` (`refmvs.h:81-84`), sized via `iw8`/`ih8` (`refmvs.c:819-822`).
#[derive(Debug, Clone)]
pub struct MotionFieldGrid {
    cells: Vec<Option<SavedMv>>,
    pub cols_8x8: u32,
    pub rows_8x8: u32,
}

impl MotionFieldGrid {
    pub(super) fn empty(cols_8x8: u32, rows_8x8: u32) -> Self {
        Self {
            cells: vec![None; (cols_8x8 as usize) * (rows_8x8 as usize)],
            cols_8x8,
            rows_8x8,
        }
    }

    pub fn get(&self, x8: u32, y8: u32) -> Option<SavedMv> {
        if x8 >= self.cols_8x8 || y8 >= self.rows_8x8 {
            return None;
        }
        self.cells[(y8 * self.cols_8x8 + x8) as usize]
    }

    pub(super) fn set(&mut self, x8: u32, y8: u32, value: Option<SavedMv>) {
        if x8 < self.cols_8x8 && y8 < self.rows_8x8 {
            self.cells[(y8 * self.cols_8x8 + x8) as usize] = value;
        }
    }
}

/// dav1d `save_tmvs_c` (`refmvs.c:763-802`, spec 7.9 storage). `mfmv_sign[r]`: whether ref index
/// `r` (0..=6, this crate's `ref0` convention -- see `crate::tile::context`'s doc, `RefFrame`
/// discriminant minus 1) is a valid "forward-looking" reference for *this* frame (dav1d's
/// `mfmv_sign[i] = poc_diff < 0`, `refmvs.c:849` -- computed by the caller from `relative_dist`).
pub fn store_motion_field(
    coding_units: &[CodingUnit],
    frame_width: u32,
    frame_height: u32,
    mfmv_sign: &[bool; 7],
) -> MotionFieldGrid {
    let cols_8x8 = frame_width.div_ceil(8).max(1);
    let rows_8x8 = frame_height.div_ceil(8).max(1);
    // 4x4-unit-granularity index (not 8x8) so the bottom-right-of-the-2x2-group sample below can
    // land precisely on that specific 4x4 sub-position even when the 8x8 region straddles more
    // than one CU (real for any block smaller than 8x8, e.g. 4x4/4x8/8x4) -- an 8x8-granularity
    // index's "first CU wins" tiebreak (`CuSpatialIndex::new`'s doc) wouldn't reliably pick that
    // exact sub-position.
    let cols_4x4 = frame_width.div_ceil(4).max(1);
    let rows_4x4 = frame_height.div_ceil(4).max(1);
    let index = CuSpatialIndex::new(coding_units, cols_4x4, rows_4x4, 4, 4);

    let mut grid = MotionFieldGrid::empty(cols_8x8, rows_8x8);
    for y8 in 0..rows_8x8 {
        for x8 in 0..cols_8x8 {
            // Bottom-right 4x4 sub-block of the 2x2 group forming this 8x8 cell
            // (`save_tmvs_c`'s `b[x*2+1]`, `refmvs.c:773`).
            let gx = (2 * x8 + 1).min(cols_4x4 - 1);
            let gy = (2 * y8 + 1).min(rows_4x4 - 1);
            let Some(cu_idx) = index.get_cu_index(gx, gy) else {
                continue;
            };
            grid.set(x8, y8, saved_mv_for_cu(&coding_units[cu_idx], mfmv_sign));
        }
    }
    grid
}

/// Compound-`ref[1]`-preferred-over-`ref[0]` selection + sign/magnitude gating, `save_tmvs_c`'s
/// per-cell body (`refmvs.c:776-798`). Returns `None` for intra blocks and blocks whose eligible
/// ref fails the gate (dav1d's "empty" `ref=0` cell).
fn saved_mv_for_cu(cu: &CodingUnit, mfmv_sign: &[bool; 7]) -> Option<SavedMv> {
    // Magnitude gate is `< 4096` in 1/8-pel units.
    const MAG_LIMIT: i32 = 4096;
    let eligible = |rf: RefFrame, mv: MotionVector| -> Option<SavedMv> {
        if rf == RefFrame::Intra {
            return None;
        }
        let idx0 = rf as usize - 1; // RefFrame::Last(1)..=AltRef(7) -> 0..=6
        if !mfmv_sign[idx0] {
            return None;
        }
        if mv.x.unsigned_abs().max(mv.y.unsigned_abs()) >= MAG_LIMIT as u32 {
            return None;
        }
        Some(SavedMv { mv, ref_frame: rf })
    };
    eligible(cu.ref_frames[1], cu.mv[1]).or_else(|| eligible(cu.ref_frames[0], cu.mv[0]))
}
