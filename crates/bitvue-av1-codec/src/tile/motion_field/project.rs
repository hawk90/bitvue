//! Projection of the selected sources onto the current frame (spec 7.10).

use super::state::{MfmvSource, MotionFieldState};
use crate::tile::coding_unit::MotionVector;

/// dav1d's `mv_projection` (`refmvs.c:175-191`) -- scales `mv` by the ratio `num/den` of two
/// order-hint deltas, spec 7.9.3's rounding/clip. `den` must be in `1..32`(dav1d asserts this --
/// callers here only ever pass a `pocdiff`/`ref2ref` value already clamped to that range, see
/// `select_motion_field_sources`).
pub(crate) fn mv_projection(mv: MotionVector, num: i32, den: i32) -> MotionVector {
    const DIV_MULT: [i32; 32] = [
        0, 16384, 8192, 5461, 4096, 3276, 2730, 2340, 2048, 1820, 1638, 1489, 1365, 1260, 1170,
        1092, 1024, 963, 910, 862, 819, 780, 744, 712, 682, 655, 630, 606, 585, 564, 546, 528,
    ];
    debug_assert!((1..32).contains(&den));
    debug_assert!((-32..32).contains(&num));
    let frac = num * DIV_MULT[den as usize];
    let y = mv.y * frac;
    let x = mv.x * frac;
    // Real dav1d clips to 0x3fff (1/8-pel units). `(v + 8192 + (v>>31)) >> 14` is dav1d's round-to-nearest-ties-away-from-
    // zero via arithmetic shift; ported verbatim since it's unit-independent (round/shift on the
    // already-scaled product, before the unit-dependent clip).
    const CLIP: i32 = 0x3fff;
    let round = |v: i32| ((v + 8192 + (v >> 31)) >> 14).clamp(-CLIP, CLIP);
    MotionVector::new(round(x), round(y))
}

/// One cell of a [`ProjectedMotionField`] -- **not** a final MV. Real dav1d's `load_tmvs_c`
/// (`refmvs.c:690-761`) uses `mv_projection` only to compute a *position* offset (where to place
/// this entry in the current frame's grid); the value it stores is the saved cell's *original,
/// unrescaled* MV (`rp_proj[...].mv = rb->mv`, `refmvs.c:741`) plus the `ref2ref` order-hint delta
/// that produced that offset (`rp_proj[...].ref = ref2ref`, `refmvs.c:742`) -- repurposed as the
/// denominator for a *second* `mv_projection` at candidate-consumption time
/// (`add_temporal_candidate`, `refmvs.c:201`, numerator = the current block's own ref's
/// `pocdiff`). The actual MV rescale happens exactly once, at consumption, not here.
#[derive(Debug, Clone, Copy)]
pub struct ProjectedMv {
    pub mv: MotionVector,
    pub ref2ref: i32,
}

#[derive(Debug, Clone)]
pub struct ProjectedMotionField {
    cells: Vec<Option<ProjectedMv>>,
    pub cols_8x8: u32,
    pub rows_8x8: u32,
}

impl ProjectedMotionField {
    pub(super) fn empty(cols_8x8: u32, rows_8x8: u32) -> Self {
        Self {
            cells: vec![None; (cols_8x8 as usize) * (rows_8x8 as usize)],
            cols_8x8,
            rows_8x8,
        }
    }

    pub fn get(&self, x8: u32, y8: u32) -> Option<ProjectedMv> {
        if x8 >= self.cols_8x8 || y8 >= self.rows_8x8 {
            return None;
        }
        self.cells[(y8 * self.cols_8x8 + x8) as usize]
    }

    pub(super) fn set(&mut self, x8: u32, y8: u32, value: ProjectedMv) {
        if x8 < self.cols_8x8 && y8 < self.rows_8x8 {
            self.cells[(y8 * self.cols_8x8 + x8) as usize] = Some(value);
        }
    }
}

/// dav1d `load_tmvs_c`'s per-source, per-cell projection (`refmvs.c:690-761`, spec 7.10
/// estimation) -- plain per-cell loop, not the identical-neighbor-run splat dav1d uses for
/// performance (an equivalent, simpler simplification for this crate's non-realtime use).
///
/// Scope note: dav1d also re-derives each cell's `bx8/by8`-block-size run so the *position*
/// offset is applied consistently across a whole run of identical neighboring cells
/// (`refmvs.c:744-757`) -- here every cell is projected independently, which is behaviorally
/// identical (each cell's own saved MV determines its own offset either way) and only differs in
/// CPU cost, not output.
pub fn project_motion_field(
    sources: &[MfmvSource],
    state: &MotionFieldState,
    cols_8x8: u32,
    rows_8x8: u32,
) -> ProjectedMotionField {
    let mut out = ProjectedMotionField::empty(cols_8x8, rows_8x8);
    for src in sources {
        let Some(saved) = &state.grids[src.slot] else {
            continue;
        };
        for y in 0..rows_8x8.min(saved.rows_8x8) {
            for x in 0..cols_8x8.min(saved.cols_8x8) {
                let Some(cell) = saved.get(x, y) else {
                    continue;
                };
                let ref2ref = src.ref2ref[cell.ref_frame as usize - 1];
                if ref2ref == 0 {
                    continue;
                }
                let offset = mv_projection(cell.mv, src.ref2cur, ref2ref);
                // Position offset: eighth-pel-to-8x8-cell shift is `>>6` in dav1d (8 px/cell * 8
                // subpel-units/px = 64).
                // dav1d `apply_sign(abs(offset) >> 6, offset ^ ref_sign)`: forward sources flip.
                let flip = if src.forward { -1 } else { 1 };
                let off_x = (offset.x.unsigned_abs() >> 6) as i32 * offset.x.signum() * flip;
                let off_y = (offset.y.unsigned_abs() >> 6) as i32 * offset.y.signum() * flip;
                let px = x as i32 + off_x;
                let py = y as i32 + off_y;
                // The projection stays inside the superblock row of its source cell (8 cells)
                // and at most one superblock (8 cells) to the left or 16 to the right of it
                // (`load_tmvs_c`: dav1d works one superblock row at a time).
                let (x_sb, y_sb) = ((x & !7) as i32, (y & !7) as i32);
                if py < y_sb
                    || py >= (y_sb + 8).min(rows_8x8 as i32)
                    || px < (x_sb - 8).max(0)
                    || px >= (x_sb + 16).min(cols_8x8 as i32)
                {
                    continue;
                }
                out.set(
                    px as u32,
                    py as u32,
                    ProjectedMv {
                        mv: cell.mv,
                        ref2ref,
                    },
                );
            }
        }
    }
    out
}
