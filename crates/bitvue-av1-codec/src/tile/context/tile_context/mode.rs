//! Key-frame `intra_mode` context.

use super::TileContext;
use crate::tile::context::tables::INTRA_MODE_CONTEXT;

impl TileContext {
    /// Key-frame `intra_mode` context: `(above_mode_class, left_mode_class)`, each 0..=4, for a
    /// block at absolute 4x4 position `(x4, y4)` -- per spec/dav1d:
    /// `INTRA_MODE_CONTEXT[above_mode[x4]]`, `INTRA_MODE_CONTEXT[left_mode[y4]]`.
    pub fn intra_mode_context(&self, x4: u32, y4: u32) -> (u8, u8) {
        let above_raw = self.above_mode.get(x4 as usize).copied().unwrap_or(0);
        let left_raw = self.left_mode.get(y4 as usize).copied().unwrap_or(0);
        (
            INTRA_MODE_CONTEXT[above_raw as usize],
            INTRA_MODE_CONTEXT[left_raw as usize],
        )
    }

    /// Record a decoded intra-mode symbol across the block's 4x4-unit footprint, for future
    /// context lookups.
    pub fn set_mode(&mut self, x4: u32, y4: u32, width_4x4: u32, height_4x4: u32, mode_symbol: u8) {
        let x_end = (x4 + width_4x4).min(self.above_mode.len() as u32);
        for x in x4..x_end {
            self.above_mode[x as usize] = mode_symbol;
        }
        let y_end = (y4 + height_4x4).min(self.left_mode.len() as u32);
        for y in y4..y_end {
            self.left_mode[y as usize] = mode_symbol;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intra_mode_context_defaults_to_dc_pred_class() {
        // No neighbors written yet -- both default to raw mode 0 (DC_PRED), class 0.
        let ctx = TileContext::new(16, 16);
        assert_eq!(ctx.intra_mode_context(0, 0), (0, 0));
    }

    #[test]
    fn test_intra_mode_context_maps_through_intra_mode_context_table() {
        let mut ctx = TileContext::new(16, 16);
        // Raw mode 8 (D67_PRED) -> class 3 per INTRA_MODE_CONTEXT.
        ctx.set_mode(0, 0, 4, 4, 8);
        assert_eq!(ctx.intra_mode_context(0, 5).0, 3); // above contribution only
        assert_eq!(ctx.intra_mode_context(5, 0).1, 3); // left contribution only
    }

    #[test]
    fn test_intra_mode_context_combines_distinct_above_and_left_classes() {
        let mut ctx = TileContext::new(16, 16);
        // A block at (x4=0, y4=0..4) sets above_mode[0..4] AND left_mode[0..4] -- to isolate the
        // two contributions for a query at (x4=0, y4=4), use two non-overlapping blocks: one
        // covering only above_mode[0] (a block above-and-to-the-left, x4=0 y4=0..4), one covering
        // only left_mode[4] (a block directly above the query's row, y4=4 x4=4..8 -- outside the
        // query's own x4=0 column, so it can't also perturb above_mode[0]).
        ctx.set_mode(0, 0, 4, 4, 8); // above_mode[0..4]=8, left_mode[0..4]=8
        ctx.set_mode(4, 4, 4, 4, 4); // above_mode[4..8]=4, left_mode[4..8]=4
                                     // Query (0, 4): above_mode[0]=8 (class 3, untouched by the second call), left_mode[4]=4
                                     // (class 4, set by the second call).
        assert_eq!(ctx.intra_mode_context(0, 4), (3, 4));
    }

    #[test]
    fn test_start_superblock_row_resets_left_mode_but_not_above_mode() {
        let mut ctx = TileContext::new(16, 16);
        ctx.set_mode(0, 0, 4, 4, 8);
        ctx.start_superblock_row();
        assert_eq!(ctx.intra_mode_context(5, 0).1, 0); // left reset to DC_PRED class
        assert_eq!(ctx.intra_mode_context(0, 5).0, 3); // above persists
    }

    #[test]
    fn test_intra_mode_context_table_matches_rav1d_dav1d_intra_mode_context() {
        // Source: rav1d `DAV1D_INTRA_MODE_CONTEXT` (`src/tables.rs`). Pinning the exact table
        // here catches an accidental edit independent of the round-trip tests above.
        assert_eq!(INTRA_MODE_CONTEXT, [0, 1, 2, 3, 4, 4, 4, 4, 3, 0, 1, 2, 0]);
    }
}
