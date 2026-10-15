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
        let have_top = self.has_top(y4);
        let have_left = self.has_left(x4);
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

#[cfg(test)]
mod tests {
    use super::*;

    fn grid_of(ctx: &TileContext) -> Vec<i16> {
        let rows = ctx.seg_id_grid.len() / ctx.seg_id_grid_stride as usize;
        (0..rows as u32)
            .flat_map(|y| (0..ctx.seg_id_grid_stride).map(move |x| (x, y)))
            .map(|(x, y)| ctx.seg_id_grid_get(x, y))
            .collect()
    }

    // ---- seg_pred ----

    #[test]
    fn seg_pred_context_counts_the_above_and_left_flags() {
        let mut ctx = TileContext::new(8, 8);
        assert_eq!(ctx.seg_pred_context(2, 3), 0);
        ctx.set_seg_pred(2, 3, 1, 1, true);
        assert_eq!(
            ctx.seg_pred_context(2, 3),
            2,
            "both the column and the row are set"
        );
        assert_eq!(
            ctx.seg_pred_context(2, 0),
            1,
            "only the above flag of column 2"
        );
        assert_eq!(ctx.seg_pred_context(0, 3), 1, "only the left flag of row 3");
        assert_eq!(ctx.seg_pred_context(5, 5), 0);
        ctx.set_seg_pred(2, 3, 1, 1, false);
        assert_eq!(ctx.seg_pred_context(2, 3), 0);
    }

    #[test]
    fn set_seg_pred_covers_the_footprint_on_each_axis_and_clips_at_the_edge() {
        let mut ctx = TileContext::new(8, 6);
        ctx.set_seg_pred(2, 1, 3, 2, true);
        let above: Vec<u8> = (0..8).map(|x| ctx.seg_pred_context(x, 7)).collect();
        // Column flags for x4 = 2, 3, 4 (the row 7 is outside the rows, so only the column counts).
        assert_eq!(above, vec![0, 0, 1, 1, 1, 0, 0, 0]);
        let left: Vec<u8> = (0..6).map(|y| ctx.seg_pred_context(7, y)).collect();
        assert_eq!(left, vec![0, 1, 1, 0, 0, 0]);
        // A footprint hanging over the end is clipped, not wrapped or panicking.
        ctx.set_seg_pred(6, 4, 9, 9, true);
        assert_eq!(ctx.seg_pred_context(7, 5), 2);
        ctx.set_seg_pred(100, 100, 4, 4, true);
    }

    #[test]
    fn seg_pred_context_outside_the_tile_reads_as_unset() {
        let mut ctx = TileContext::new(4, 4);
        ctx.set_seg_pred(0, 0, 4, 4, true);
        assert_eq!(ctx.seg_pred_context(4, 0), 1, "only the row flag");
        assert_eq!(ctx.seg_pred_context(0, 4), 1, "only the column flag");
        assert_eq!(ctx.seg_pred_context(4, 4), 0);
        assert_eq!(ctx.seg_pred_context(u32::MAX, u32::MAX), 0);
    }

    // ---- set_segment_id / segment_id_context ----

    #[test]
    fn a_new_grid_is_undecoded_everywhere() {
        let ctx = TileContext::new(4, 3);
        assert_eq!(grid_of(&ctx), vec![-1; 12]);
        assert_eq!(
            ctx.seg_id_grid_get(4, 0),
            -1,
            "beyond the last row reads as undecoded"
        );
        assert_eq!(ctx.seg_id_grid_get(0, 99), -1);
    }

    #[test]
    fn set_segment_id_writes_the_footprint_and_nothing_else() {
        let mut ctx = TileContext::new(4, 3);
        ctx.set_segment_id(1, 1, 2, 2, 5);
        #[rustfmt::skip]
        assert_eq!(grid_of(&ctx), vec![
            -1, -1, -1, -1,
            -1,  5,  5, -1,
            -1,  5,  5, -1,
        ]);
    }

    #[test]
    fn set_segment_id_clips_to_the_row_and_does_not_wrap_into_the_next_one() {
        let mut ctx = TileContext::new(4, 3);
        ctx.set_segment_id(2, 0, 4, 1, 3);
        #[rustfmt::skip]
        assert_eq!(grid_of(&ctx), vec![
            -1, -1, 3, 3,
            -1, -1, -1, -1,
            -1, -1, -1, -1,
        ]);
        // Completely right of the tile: nothing is written anywhere.
        let mut ctx = TileContext::new(4, 3);
        ctx.set_segment_id(4, 0, 2, 2, 6);
        ctx.set_segment_id(9, 1, 2, 2, 6);
        assert_eq!(grid_of(&ctx), vec![-1; 12]);
    }

    #[test]
    fn set_segment_id_stops_at_the_last_row() {
        let mut ctx = TileContext::new(4, 3);
        ctx.set_segment_id(0, 2, 2, 100, 7);
        #[rustfmt::skip]
        assert_eq!(grid_of(&ctx), vec![
            -1, -1, -1, -1,
            -1, -1, -1, -1,
             7,  7, -1, -1,
        ]);
        ctx.set_segment_id(0, 50, 2, 2, 1);
        ctx.set_segment_id(0, 3, 2, 2, 1);
        assert_eq!(grid_of(&ctx).iter().filter(|&&v| v == 1).count(), 0);
    }

    #[test]
    fn later_writes_replace_earlier_ones() {
        let mut ctx = TileContext::new(4, 4);
        ctx.set_segment_id(0, 0, 4, 4, 2);
        ctx.set_segment_id(1, 1, 2, 2, 0);
        assert_eq!(ctx.seg_id_grid_get(0, 0), 2);
        assert_eq!(ctx.seg_id_grid_get(1, 1), 0);
        assert_eq!(ctx.seg_id_grid_get(2, 2), 0);
        assert_eq!(ctx.seg_id_grid_get(3, 3), 2);
    }

    /// Spec / dav1d `get_cur_frame_segid`, restated: `(ctx, pred)` from the left, above and
    /// above-left ids (negative = not decoded).
    fn reference(have_top: bool, have_left: bool, l: i16, a: i16, al: i16) -> (u8, u8) {
        if have_top && have_left {
            let ctx = if l == a && al == l {
                2
            } else if l == a || al == l || a == al {
                1
            } else {
                0
            };
            let pred = if a == al { a } else { l };
            (ctx, pred.max(0) as u8)
        } else if have_left {
            (0, l.max(0) as u8)
        } else if have_top {
            (0, a.max(0) as u8)
        } else {
            (0, 0)
        }
    }

    #[test]
    fn the_context_matches_the_reference_for_every_neighbour_combination() {
        // Interior position (1,1) of a 4x4 tile: all three neighbours exist.
        let values = [-1i16, 0, 1, 2];
        for &l in &values {
            for &a in &values {
                for &al in &values {
                    let mut ctx = TileContext::new(4, 4);
                    let mut put = |x, y, v: i16| {
                        if v >= 0 {
                            ctx.set_segment_id(x, y, 1, 1, v as u8);
                        }
                    };
                    put(0, 1, l);
                    put(1, 0, a);
                    put(0, 0, al);
                    assert_eq!(
                        ctx.segment_id_context(1, 1),
                        reference(true, true, l, a, al),
                        "l {l} a {a} al {al}"
                    );
                }
            }
        }
    }

    #[test]
    fn at_a_tile_edge_only_the_available_neighbour_counts() {
        let mut ctx = TileContext::new(4, 4);
        ctx.set_segment_id(0, 0, 1, 1, 3);
        ctx.set_segment_id(1, 0, 1, 1, 2);
        ctx.set_segment_id(0, 1, 1, 1, 1);
        // Left edge: above only (0,0) -> 3.
        assert_eq!(ctx.segment_id_context(0, 1), (0, 3));
        // Top edge: left only (0,0) -> 3; and (2,0): left (1,0) -> 2.
        assert_eq!(ctx.segment_id_context(1, 0), (0, 3));
        assert_eq!(ctx.segment_id_context(2, 0), (0, 2));
        // The corner has nobody.
        assert_eq!(ctx.segment_id_context(0, 0), (0, 0));
        // Undecoded neighbours predict 0.
        assert_eq!(ctx.segment_id_context(0, 3), (0, 0));
    }

    #[test]
    fn the_prediction_is_the_above_id_when_it_equals_the_above_left_else_the_left() {
        let mut ctx = TileContext::new(4, 4);
        ctx.set_segment_id(0, 0, 1, 1, 2); // al
        ctx.set_segment_id(1, 0, 1, 1, 2); // a
        ctx.set_segment_id(0, 1, 1, 1, 5); // l
        assert_eq!(ctx.segment_id_context(1, 1), (1, 2), "a == al: predict a");
        ctx.set_segment_id(1, 0, 1, 1, 4);
        assert_eq!(
            ctx.segment_id_context(1, 1),
            (0, 5),
            "all differ: predict l"
        );
    }
}
