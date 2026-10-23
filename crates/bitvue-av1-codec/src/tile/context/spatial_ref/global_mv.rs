//! Global motion in the candidate stacks (spec 7.10.2.1 `setup_global_mv`, dav1d `get_gmv_2d`):
//! the frame's warp evaluated at a block, and the neighbours whose own vector it replaces.

use super::*;
use crate::frame_header_full::{
    GmParams, GM_IDENTITY_PARAMS, GM_TYPE_IDENTITY, GM_TYPE_ROTZOOM, GM_TYPE_TRANSLATION,
};
use crate::tile::coding_unit::MotionVector;
use crate::tile::motion_field::fix_mv_precision;

/// `WARPEDMODEL_PREC_BITS`.
const WARP_PREC: u32 = 16;

/// The frame's `GmType` / `gm_params` per `RefFrame` (index 0 unused) and the MV precision the
/// global vector is rounded to.
#[derive(Clone, Copy)]
pub(super) struct GlobalMotion {
    gm_type: [u8; 8],
    params: GmParams,
    allow_high_precision_mv: bool,
    force_integer_mv: bool,
}

impl Default for GlobalMotion {
    fn default() -> Self {
        Self {
            gm_type: [GM_TYPE_IDENTITY; 8],
            params: [GM_IDENTITY_PARAMS; 8],
            allow_high_precision_mv: true,
            force_integer_mv: false,
        }
    }
}

/// `Round2Signed(x, n)`.
fn round2_signed(x: i64, n: u32) -> i64 {
    let half = 1i64 << (n - 1);
    if x >= 0 {
        (x + half) >> n
    } else {
        -((-x + half) >> n)
    }
}

impl SpatialRefCell {
    /// Whether a query that has a global MV takes it in place of this neighbour's own vector: the
    /// neighbour is a `GLOBALMV` block of at least 8x8 (spec 7.10.2.9's `large`).
    pub(super) fn substitutes_global_mv(&self) -> bool {
        self.is_global_mode && self.width_4x4.min(self.height_4x4) >= 2
    }
}

impl SpatialRefContext {
    /// Sets the frame's global motion (`GmType`, `gm_params`, MV precision). Call once per frame,
    /// before any block is parsed.
    pub fn set_global_motion(
        &mut self,
        gm_type: [u8; 8],
        params: GmParams,
        allow_high_precision_mv: bool,
        force_integer_mv: bool,
    ) {
        self.global_motion = GlobalMotion {
            gm_type,
            params,
            allow_high_precision_mv,
            force_integer_mv,
        };
    }

    /// Marks the block at this footprint as a `GLOBALMV` / `GLOBAL_GLOBALMV` block (call after
    /// `set_block`).
    pub fn mark_global_mode(&mut self, x4: u32, y4: u32, width_4x4: u32, height_4x4: u32) {
        let x_end = (x4 + width_4x4).min(self.width_4x4);
        let y_end = (y4 + height_4x4).min(self.height_4x4);
        for y in y4..y_end {
            for x in x4..x_end {
                self.cells[(y * self.width_4x4 + x) as usize].is_global_mode = true;
            }
        }
    }

    /// `setup_global_mv` for reference `ref0` (0-based, `-1` for none): the global motion vector
    /// of the `bw4` x `bh4` block at `(x4, y4)`, in 1/8-sample units at the frame's MV precision.
    pub fn global_mv(&self, ref0: i8, x4: u32, y4: u32, bw4: u32, bh4: u32) -> MotionVector {
        let g = &self.global_motion;
        let Some(slot) = usize::try_from(ref0 + 1)
            .ok()
            .filter(|&s| (1..8).contains(&s))
        else {
            return MotionVector::zero();
        };
        let p = g.params[slot].map(i64::from);
        let mv = match g.gm_type[slot] {
            GM_TYPE_IDENTITY => return MotionVector::zero(),
            // Translation-only warps keep the row in parameter 0 and the column in parameter 1.
            GM_TYPE_TRANSLATION => MotionVector {
                y: (p[0] >> (WARP_PREC - 3)) as i32,
                x: (p[1] >> (WARP_PREC - 3)) as i32,
            },
            _ => {
                let x = i64::from(x4) * 4 + i64::from(bw4) * 2 - 1;
                let y = i64::from(y4) * 4 + i64::from(bh4) * 2 - 1;
                let one = 1i64 << WARP_PREC;
                let xc = (p[2] - one) * x + p[3] * y + p[0];
                let yc = p[4] * x + (p[5] - one) * y + p[1];
                if g.allow_high_precision_mv {
                    MotionVector {
                        y: round2_signed(yc, WARP_PREC - 3) as i32,
                        x: round2_signed(xc, WARP_PREC - 3) as i32,
                    }
                } else {
                    MotionVector {
                        y: round2_signed(yc, WARP_PREC - 2) as i32 * 2,
                        x: round2_signed(xc, WARP_PREC - 2) as i32 * 2,
                    }
                }
            }
        };
        fix_mv_precision(mv, g.allow_high_precision_mv, g.force_integer_mv)
    }

    /// The global MV a `GLOBALMV` neighbour's candidate is replaced with: only for references
    /// whose warp is more than a translation (`GmType > TRANSLATION`).
    pub(super) fn global_candidate(
        &self,
        ref0: i8,
        x4: u32,
        y4: u32,
        bw4: u32,
        bh4: u32,
    ) -> Option<MotionVector> {
        let slot = usize::try_from(ref0 + 1)
            .ok()
            .filter(|&s| (1..8).contains(&s))?;
        (self.global_motion.gm_type[slot] >= GM_TYPE_ROTZOOM)
            .then(|| self.global_mv(ref0, x4, y4, bw4, bh4))
    }
}
