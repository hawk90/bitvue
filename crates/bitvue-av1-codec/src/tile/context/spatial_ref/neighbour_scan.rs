//! The neighbour scans of dav1d `refmvs_find`, shared by the mode contexts
//! ([`SpatialRefContext::scan`]) and both candidate stacks. The scans only decide *which cells are
//! visited and with what weight*; what a visit does is up to the caller's closure.

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

/// How far the neighbour scans reach for one query.
pub(super) struct ScanExtent {
    pub(super) w4: u32,
    pub(super) h4: u32,
    pub(super) max_rows: i32,
    pub(super) max_cols: i32,
}

/// Which neighbour a visited cell is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Neighbour {
    /// The row above the block.
    Above,
    /// The column to the left of the block.
    Left,
    AboveRight,
    AboveLeft,
    /// A row 3 or 5 units above (the "secondary" scans).
    SecondaryAbove,
    /// A column 3 or 5 units to the left.
    SecondaryLeft,
}

/// Called for every visited cell with the weight of the visit and which neighbour it is.
pub(super) type Visit<'a> = dyn FnMut(&SpatialRefCell, i32, Neighbour) + 'a;

impl SpatialRefContext {
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

    /// The nearest group: the row above, the column to the left and the top-right cell (only a
    /// decoded cell can be there, dav1d's `EDGE_I444_TOP_HAS_RIGHT`, weight 4). Returns
    /// `(n_rows, n_cols)`, each `u32::MAX` while the row/column is outside the tile.
    pub(super) fn scan_nearest(
        &self,
        q: &ScanQuery,
        extent: &ScanExtent,
        visit: &mut Visit,
    ) -> (u32, u32) {
        let mut n_rows = u32::MAX;
        let mut n_cols = u32::MAX;
        if q.y4 > self.row_start {
            n_rows = self.walk_row(
                q.y4 - 1,
                q.x4,
                q.bw4,
                extent.w4,
                extent.max_rows,
                if q.bw4 >= 16 { 4 } else { 1 },
                &mut |cell, weight| visit(cell, weight, Neighbour::Above),
            );
        }
        if q.x4 > self.col_start {
            n_cols = self.walk_col(
                q.x4 - 1,
                q.y4,
                q.bh4,
                extent.h4,
                extent.max_cols,
                if q.bh4 >= 16 { 4 } else { 1 },
                &mut |cell, weight| visit(cell, weight, Neighbour::Left),
            );
        }
        if n_rows != u32::MAX && q.bw4.max(q.bh4) <= 16 && q.x4 + q.bw4 < self.col_end {
            if let Some(cell) = self.cell(q.x4 + q.bw4, q.y4 - 1) {
                visit(cell, 4, Neighbour::AboveRight);
            }
        }
        (n_rows, n_cols)
    }

    /// The top-left cell (weight 4) -- only where both the row above and the column to the left
    /// exist.
    pub(super) fn scan_above_left(
        &self,
        q: &ScanQuery,
        n_rows: u32,
        n_cols: u32,
        visit: &mut Visit,
    ) {
        if n_rows != u32::MAX && n_cols != u32::MAX {
            if let Some(cell) = self.cell(q.x4 - 1, q.y4 - 1) {
                visit(cell, 4, Neighbour::AboveLeft);
            }
        }
    }

    /// The secondary rows/columns 3 and 5 units back, skipped where an earlier scan already
    /// covered them; extends `n_rows`/`n_cols` by what they walked.
    pub(super) fn scan_secondary(
        &self,
        q: &ScanQuery,
        extent: &ScanExtent,
        n_rows: &mut u32,
        n_cols: &mut u32,
        visit: &mut Visit,
    ) {
        for n in 2..=3u32 {
            if n > *n_rows && n as i32 <= extent.max_rows {
                *n_rows += self.walk_row(
                    (q.y4 + 1 - 2 * n) | 1,
                    q.x4 | 1,
                    q.bw4,
                    extent.w4,
                    1 + extent.max_rows - n as i32,
                    if q.bw4 >= 16 { 4 } else { 2 },
                    &mut |cell, weight| visit(cell, weight, Neighbour::SecondaryAbove),
                );
            }
            if n > *n_cols && n as i32 <= extent.max_cols {
                *n_cols += self.walk_col(
                    (q.x4 + 1 - 2 * n) | 1,
                    q.y4 | 1,
                    q.bh4,
                    extent.h4,
                    1 + extent.max_cols - n as i32,
                    if q.bh4 >= 16 { 4 } else { 2 },
                    &mut |cell, weight| visit(cell, weight, Neighbour::SecondaryLeft),
                );
            }
        }
    }

    /// The limits candidate MVs are clamped to for a block (1/8-sample units: 4 samples per 4x4
    /// unit, 8 units per sample): `(left, right, top, bottom)`.
    pub(super) fn mv_clamp_bounds(&self, q: &ScanQuery) -> (i32, i32, i32, i32) {
        (
            -((q.x4 + q.bw4 + 4) as i32) * 32,
            (self.iw4 as i32 - q.x4 as i32 + 4) * 32,
            -((q.y4 + q.bh4 + 4) as i32) * 32,
            (self.ih4 as i32 - q.y4 as i32 + 4) * 32,
        )
    }
}
