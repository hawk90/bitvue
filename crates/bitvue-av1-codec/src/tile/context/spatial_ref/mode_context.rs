//! `inter_mode` / `compound_mode` contexts from the neighbour scan (dav1d `refmvs_find`'s `refmv_ctx` / `newmv_ctx`).

use super::*;

/// The block whose neighbours are scanned and the reference pair it looks for.
pub(super) struct ScanQuery {
    pub(super) x4: u32,
    pub(super) y4: u32,
    pub(super) bw4: u32,
    pub(super) bh4: u32,
    pub(super) ref0: i8,
    pub(super) ref1: i8,
    /// The block's global MV per reference, where that reference's warp is more than a
    /// translation (see [`SpatialRefContext::global_candidate`]); substitutes `GLOBALMV` neighbours.
    pub(super) gmvs: [Option<crate::tile::coding_unit::MotionVector>; 2],
}

impl ScanQuery {
    fn matches(&self, cell: &SpatialRefCell) -> bool {
        SpatialRefContext::ref_matches(cell, self.ref0, self.ref1)
    }
}

/// How far the neighbour scans reach for one query.
pub(super) struct ScanExtent {
    pub(super) w4: u32,
    pub(super) h4: u32,
    pub(super) max_rows: i32,
    pub(super) max_cols: i32,
}

/// What the scans found so far.
#[derive(Default)]
struct ScanMatches {
    newmv: bool,
    row_mvs: bool,
    col_mvs: bool,
}

impl SpatialRefContext {
    /// Whether a decoded cell's ref matches the query block's `(ref0, ref1)` -- rav1d
    /// `add_spatial_candidate`: single-ref (`ref1 < 0`) matches if the cell's `ref0` *or* `ref1`
    /// equals the query's `ref0`; compound matches only on an exact `(ref0, ref1)` pair match.
    pub(super) fn ref_matches(cell: &SpatialRefCell, ref0: i8, ref1: i8) -> bool {
        if !cell.valid {
            return false;
        }
        if ref1 < 0 {
            cell.ref0 == ref0 || cell.ref1 == ref0
        } else {
            cell.ref0 == ref0 && cell.ref1 == ref1
        }
    }

    /// Spatial neighbor scan shared by `inter_mode_context`/`compound_mode_context`. Returns
    /// `(nearest_match, ref_match_count, have_newmv)`, matching rav1d `rav1d_refmvs_find`'s
    /// same-named locals (`src/refmvs.rs`) computed purely from the "primary" above-row/left-
    /// column/top-right/top-left scans plus the "secondary" (2/3-units-back) row/column scans --
    /// everything except the temporal (`add_temporal_candidate`) contribution, see this struct's
    /// doc. `w4`/`h4` are the query block's own width/height in 4x4 units, capped to 16 (rav1d:
    /// `cmp::min(bw4, 16)`/`cmp::min(bh4, 16)` -- the tile-bound clamp rav1d also applies is
    /// skipped here, matching the rest of this crate's "whole frame as one tile" simplification).
    pub(super) fn scan(
        &self,
        x4: u32,
        y4: u32,
        bw4: u32,
        bh4: u32,
        ref0: i8,
        ref1: i8,
    ) -> (u8, u8, bool) {
        let query = ScanQuery {
            x4,
            y4,
            bw4,
            bh4,
            ref0,
            ref1,
            gmvs: [None; 2],
        };
        let extent = self.scan_extent(&query);
        let mut found = ScanMatches::default();

        let (mut n_rows, mut n_cols) = self.scan_primary(&query, &extent, &mut found);
        let nearest_match = u8::from(found.row_mvs) + u8::from(found.col_mvs);

        self.scan_top_left(&query, n_rows, n_cols, &mut found);
        self.scan_secondary_matches(&query, &extent, &mut n_rows, &mut n_cols, &mut found);

        let ref_match_count = u8::from(found.row_mvs) + u8::from(found.col_mvs);
        (nearest_match, ref_match_count, found.newmv)
    }

    /// The scanned width/height (capped, clamped to the frame edge) and how many 8x8 rows/columns
    /// above/left of the block lie inside the tile.
    pub(super) fn scan_extent(&self, q: &ScanQuery) -> ScanExtent {
        let w4 = q.bw4.min(16).min(self.col_end.saturating_sub(q.x4)).max(1);
        let h4 = q.bh4.min(16).min(self.row_end.saturating_sub(q.y4)).max(1);
        let max_rows = if q.y4 > self.row_start {
            (q.y4 - self.row_start)
                .div_ceil(2)
                .min(2 + u32::from(q.bh4 > 1))
        } else {
            0
        } as i32;
        let max_cols = if q.x4 > self.col_start {
            (q.x4 - self.col_start)
                .div_ceil(2)
                .min(2 + u32::from(q.bw4 > 1))
        } else {
            0
        } as i32;
        ScanExtent {
            w4,
            h4,
            max_rows,
            max_cols,
        }
    }

    /// The "primary" scans: the row above, the column to the left and the top-right cell. These
    /// are the only ones that feed `newmv`. Returns `(n_rows, n_cols)`, each `u32::MAX` while the
    /// row/column is outside the tile.
    fn scan_primary(
        &self,
        q: &ScanQuery,
        extent: &ScanExtent,
        found: &mut ScanMatches,
    ) -> (u32, u32) {
        let (x4, y4, bw4, bh4) = (q.x4, q.y4, q.bw4, q.bh4);
        let mut n_rows = u32::MAX;
        let mut n_cols = u32::MAX;

        if y4 > self.row_start {
            n_rows = self.walk_row(
                y4 - 1,
                x4,
                bw4,
                extent.w4,
                extent.max_rows,
                if bw4 >= 16 { 4 } else { 1 },
                &mut |cell, _| {
                    if q.matches(cell) {
                        found.row_mvs = true;
                        found.newmv |= cell.is_newmv;
                    }
                },
            );
        }
        if x4 > self.col_start {
            n_cols = self.walk_col(
                x4 - 1,
                y4,
                bh4,
                extent.h4,
                extent.max_cols,
                if bh4 >= 16 { 4 } else { 1 },
                &mut |cell, _| {
                    if q.matches(cell) {
                        found.col_mvs = true;
                        found.newmv |= cell.is_newmv;
                    }
                },
            );
        }
        // Top-right.
        if n_rows != u32::MAX && bw4.max(bh4) <= 16 && x4 + bw4 < self.col_end {
            if let Some(cell) = self.cell(x4 + bw4, y4 - 1) {
                if q.matches(cell) {
                    found.row_mvs = true;
                    found.newmv |= cell.is_newmv;
                }
            }
        }
        (n_rows, n_cols)
    }

    /// The top-left cell. Like the secondary scans it never feeds `newmv` (dav1d's
    /// `have_dummy_newmv_match`).
    fn scan_top_left(&self, q: &ScanQuery, n_rows: u32, n_cols: u32, found: &mut ScanMatches) {
        if n_rows != u32::MAX && n_cols != u32::MAX {
            if let Some(cell) = self.cell(q.x4 - 1, q.y4 - 1) {
                if q.matches(cell) {
                    found.row_mvs = true;
                }
            }
        }
    }

    /// The "secondary" scans of the rows/columns 2 and 3 8x8 units back.
    fn scan_secondary_matches(
        &self,
        q: &ScanQuery,
        extent: &ScanExtent,
        n_rows: &mut u32,
        n_cols: &mut u32,
        found: &mut ScanMatches,
    ) {
        let (x4, y4, bw4, bh4) = (q.x4, q.y4, q.bw4, q.bh4);
        for n in 2..=3u32 {
            if n > *n_rows && n as i32 <= extent.max_rows {
                *n_rows += self.walk_row(
                    (y4 + 1 - 2 * n) | 1,
                    x4 | 1,
                    bw4,
                    extent.w4,
                    1 + extent.max_rows - n as i32,
                    if bw4 >= 16 { 4 } else { 2 },
                    &mut |cell, _| found.row_mvs |= q.matches(cell),
                );
            }
            if n > *n_cols && n as i32 <= extent.max_cols {
                *n_cols += self.walk_col(
                    (x4 + 1 - 2 * n) | 1,
                    y4 | 1,
                    bh4,
                    extent.h4,
                    1 + extent.max_cols - n as i32,
                    if bh4 >= 16 { 4 } else { 2 },
                    &mut |cell, _| found.col_mvs |= q.matches(cell),
                );
            }
        }
    }

    /// `(refmv_ctx, newmv_ctx)` per rav1d `rav1d_refmvs_find`'s context build-up (`src/refmvs.rs`).
    pub(super) fn refmv_newmv_ctx(
        &self,
        x4: u32,
        y4: u32,
        bw4: u32,
        bh4: u32,
        ref0: i8,
        ref1: i8,
    ) -> (u8, u8) {
        let (nearest_match, ref_match_count, have_newmv) = self.scan(x4, y4, bw4, bh4, ref0, ref1);
        match nearest_match {
            0 => (ref_match_count.min(2), u8::from(ref_match_count > 0)),
            1 => ((ref_match_count * 3).min(4), 3 - u8::from(have_newmv)),
            _ => (5, 5 - u8::from(have_newmv)), // nearest_match == 2 (max possible)
        }
    }

    /// Packed single-ref `inter_mode` context (spec 5.11.23): `newmv_mode`'s CDF index is
    /// `ctx & 7`, `globalmv_mode`'s is `ctx >> 3 & 1`, `refmv_mode`'s is `ctx >> 4 & 15`. Source:
    /// rav1d `*ctx = refmv_ctx << 4 | globalmv_ctx << 3 | newmv_ctx` (`src/refmvs.rs`).
    /// `globalmv_ctx` is real when a temporal motion-field context was set on this struct (see
    /// this function's body / `set_temporal_context`), falling back to the frame header's
    /// `use_ref_frame_mvs` flag directly otherwise (`rav1d_refmvs_find` initializes its own
    /// `globalmv_ctx` local to exactly this value before ever overriding it with a real temporal
    /// candidate).
    pub fn inter_mode_context(
        &self,
        x4: u32,
        y4: u32,
        bw4: u32,
        bh4: u32,
        ref0: i8,
        use_ref_frame_mvs: bool,
    ) -> u16 {
        let (refmv_ctx, newmv_ctx) = self.refmv_newmv_ctx(x4, y4, bw4, bh4, ref0, -1);
        // Real `globalmv_ctx` (rav1d `add_temporal_candidate`'s `!(x|y)` sample, `refmvs.c:206-
        // 207`): the temporal candidate at this block's own top-left 8x8 cell, compared against
        // the block's global motion vector.
        // Falls back to the pre-existing `use_ref_frame_mvs`-flag approximation when no temporal
        // context was set (this struct's doc) -- unchanged behavior for every caller that doesn't
        // opt in.
        // dav1d `refmvs.c:417-428`: starts as the header's flag; the block's own top-left
        // temporal sample, if it has one, overrides it with "differs from the global MV by at
        // least 2 samples".
        let mut globalmv_ctx = u16::from(use_ref_frame_mvs);
        if let (Some(t), true) = (&self.temporal, use_ref_frame_mvs && ref0 >= 0) {
            let w4 = bw4.min(16).min(self.col_end.saturating_sub(x4)).max(1);
            let h4 = bh4.min(16).min(self.row_end.saturating_sub(y4)).max(1);
            let samples = Self::temporal_samples(
                t,
                x4,
                y4,
                bw4,
                bh4,
                w4,
                h4,
                ref0,
                self.col_start,
                self.col_end,
                self.row_end,
            );
            if let Some((mv, _)) = samples.iter().find(|(_, first)| *first) {
                let global = self.global_mv(ref0, x4, y4, bw4, bh4);
                globalmv_ctx = u16::from(((mv.x - global.x).abs() | (mv.y - global.y).abs()) >= 16);
            }
        }
        (refmv_ctx as u16) << 4 | globalmv_ctx << 3 | newmv_ctx as u16
    }

    /// `compound_mode` context (spec 5.11.24, 0..=7): fully real, no temporal dependency. Source:
    /// rav1d's compound remap of `refmv_ctx`/`newmv_ctx` (`src/refmvs.rs`):
    /// `match refmv_ctx >> 1 { 0 => min(newmv_ctx,1), 1 => 1+min(newmv_ctx,3), _ => clamp(3+newmv_ctx,4,7) }`.
    pub fn compound_mode_context(
        &self,
        x4: u32,
        y4: u32,
        bw4: u32,
        bh4: u32,
        ref0: i8,
        ref1: i8,
    ) -> u8 {
        let (refmv_ctx, newmv_ctx) = self.refmv_newmv_ctx(x4, y4, bw4, bh4, ref0, ref1);
        match refmv_ctx >> 1 {
            0 => newmv_ctx.min(1),
            1 => 1 + newmv_ctx.min(3),
            _ => (3 + newmv_ctx).clamp(4, 7),
        }
    }
}
