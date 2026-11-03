//! `inter_mode` / `compound_mode` contexts from the neighbour scan (dav1d `refmvs_find`'s `refmv_ctx` / `newmv_ctx`).

use super::neighbour_scan::{Neighbour, ScanQuery};
use super::*;

/// What the scans found so far.
#[derive(Default)]
struct ScanMatches {
    newmv: bool,
    row_mvs: bool,
    col_mvs: bool,
}

impl ScanMatches {
    /// Records a visited neighbour: only the nearest group (above, left, above-right) feeds
    /// `newmv`; the top-left and the secondary scans never do (dav1d's `have_dummy_newmv_match`).
    fn record(&mut self, q: &ScanQuery, cell: &SpatialRefCell, neighbour: Neighbour) {
        if !SpatialRefContext::ref_matches(cell, q.ref0, q.ref1) {
            return;
        }
        match neighbour {
            Neighbour::Above | Neighbour::AboveRight => {
                self.row_mvs = true;
                self.newmv |= cell.is_newmv;
            }
            Neighbour::Left => {
                self.col_mvs = true;
                self.newmv |= cell.is_newmv;
            }
            Neighbour::AboveLeft | Neighbour::SecondaryAbove => self.row_mvs = true,
            Neighbour::SecondaryLeft => self.col_mvs = true,
        }
    }
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

        let (mut n_rows, mut n_cols) = self.scan_nearest(&query, &extent, &mut |cell, _, n| {
            found.record(&query, cell, n)
        });
        let nearest_match = u8::from(found.row_mvs) + u8::from(found.col_mvs);

        self.scan_above_left(&query, n_rows, n_cols, &mut |cell, _, n| {
            found.record(&query, cell, n)
        });
        self.scan_secondary(
            &query,
            &extent,
            &mut n_rows,
            &mut n_cols,
            &mut |cell, _, n| found.record(&query, cell, n),
        );

        let ref_match_count = u8::from(found.row_mvs) + u8::from(found.col_mvs);
        (nearest_match, ref_match_count, found.newmv)
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
