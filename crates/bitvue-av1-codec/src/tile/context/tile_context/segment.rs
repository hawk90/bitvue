//! `segment_id` and `seg_id_predicted` contexts.

use super::TileContext;

impl TileContext {
    /// `seg_pred` context index (0..=2) at absolute 4x4 position `(x4, y4)` -- per spec/rav1d:
    /// `above_seg_pred[x4] as u8 + left_seg_pred[y4] as u8`.
    pub fn seg_pred_context(&self, x4: u32, y4: u32) -> u8 {
        let above = self
            .above_seg_pred
            .get(x4 as usize)
            .copied()
            .unwrap_or(false);
        let left = self
            .left_seg_pred
            .get(y4 as usize)
            .copied()
            .unwrap_or(false);
        u8::from(above) + u8::from(left)
    }

    /// Record a CU's `seg_pred` flag across its 4x4-unit footprint, for future `seg_pred_context`
    /// lookups.
    pub fn set_seg_pred(&mut self, x4: u32, y4: u32, width_4x4: u32, height_4x4: u32, val: bool) {
        let x_end = (x4 + width_4x4).min(self.above_seg_pred.len() as u32);
        for x in x4..x_end {
            self.above_seg_pred[x as usize] = val;
        }
        let y_end = (y4 + height_4x4).min(self.left_seg_pred.len() as u32);
        for y in y4..y_end {
            self.left_seg_pred[y as usize] = val;
        }
    }

    /// Read one cell of the `seg_id_grid` (`-1` when out of bounds or not yet decoded).
    fn seg_id_grid_get(&self, x4: u32, y4: u32) -> i16 {
        let idx = y4 as usize * self.seg_id_grid_stride as usize + x4 as usize;
        self.seg_id_grid.get(idx).copied().unwrap_or(-1)
    }

    /// Real `segment_id` context + prediction (spec 5.11.9/5.11.10, real dav1d
    /// `get_cur_frame_segid`, `src/env.h` -- ported index-for-index, not approximated): looks up
    /// the above (`a`), left (`l`), and above-left (`al`) cells directly from `seg_id_grid`
    /// (`AvailU`/`AvailL` simplified to tile-local `y4>0`/`x4>0`, this crate's established
    /// single-tile precedent). Returns `(ctx, pred)`: `ctx` 0..=2 (`2` if all three neighbors
    /// agree, `1` if any two agree, `0` otherwise or if a neighbor is unavailable), `pred` is
    /// `a` when `a == al`, else `l` -- when only one of `AvailU`/`AvailL` holds, `ctx = 0` and
    /// `pred` is whichever single neighbor is available (`0` if neither).
    pub fn segment_id_context(&self, x4: u32, y4: u32) -> (u8, u8) {
        let have_top = y4 > 0;
        let have_left = x4 > 0;
        if have_top && have_left {
            let l = self.seg_id_grid_get(x4 - 1, y4);
            let a = self.seg_id_grid_get(x4, y4 - 1);
            let al = self.seg_id_grid_get(x4 - 1, y4 - 1);
            let ctx = if l == a && al == l {
                2
            } else if l == a || al == l || a == al {
                1
            } else {
                0
            };
            let pred = if a == al { a } else { l };
            (ctx, pred.max(0) as u8)
        } else {
            let pred = if have_left {
                self.seg_id_grid_get(x4 - 1, y4)
            } else if have_top {
                self.seg_id_grid_get(x4, y4 - 1)
            } else {
                0
            };
            (0, pred.max(0) as u8)
        }
    }

    /// Record a CU's real decoded `segment_id` across its 4x4-unit footprint, for future
    /// `segment_id_context` lookups (and any later frame's temporal prediction, when that lands --
    /// see `SegmentationInfo`'s doc for the cross-frame gap this doesn't yet close).
    pub fn set_segment_id(
        &mut self,
        x4: u32,
        y4: u32,
        width_4x4: u32,
        height_4x4: u32,
        seg_id: u8,
    ) {
        let stride = self.seg_id_grid_stride;
        let x_end = (x4 + width_4x4).min(stride);
        let y_end = y4 + height_4x4;
        for y in y4..y_end {
            let row_start = y as usize * stride as usize;
            if row_start >= self.seg_id_grid.len() {
                break;
            }
            for x in x4..x_end {
                if let Some(cell) = self.seg_id_grid.get_mut(row_start + x as usize) {
                    *cell = seg_id as i16;
                }
            }
        }
    }
}
