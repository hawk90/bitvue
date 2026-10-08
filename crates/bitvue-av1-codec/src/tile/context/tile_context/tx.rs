//! `tx_size` and var-tx (`txfm_split`) contexts.

use super::TileContext;
use crate::tile::context::tables::VAR_TX_UNSET;

impl TileContext {
    /// Intra `tx_size()` context (0..=2) for a block at absolute 4x4 position `(x4, y4)` whose
    /// largest transform has width/height classes `max_w_class`/`max_h_class` (0..=4, log2 of
    /// px / 4) -- per dav1d `get_tx_ctx`: `(left_tx_h[y4] >= max_h_class) + (above_tx_w[x4] >=
    /// max_w_class)`. Width is compared against the above neighbour and height against the left
    /// one; they differ for a rectangular block. An unwritten neighbor (`-1`, see the struct field
    /// doc) never satisfies `>=` for any real class, so it correctly contributes `0` without an
    /// explicit `have_top`/`have_left` check (same "default array value" pattern as
    /// `skip_context`).
    pub fn tx_size_context(&self, x4: u32, y4: u32, max_w_class: u8, max_h_class: u8) -> u8 {
        let above = self.above_tx_class.get(x4 as usize).copied().unwrap_or(-1);
        let left = self.left_tx_class.get(y4 as usize).copied().unwrap_or(-1);
        u8::from(left >= max_h_class as i8) + u8::from(above >= max_w_class as i8)
    }

    /// Record a decoded (or table-derived, for non-`Switchable` `TxMode`s) transform's width and
    /// height classes across the block's 4x4-unit footprint, for future `tx_size_context`
    /// lookups: the above array keeps the width class, the left array the height class.
    pub fn set_tx_class(
        &mut self,
        x4: u32,
        y4: u32,
        width_4x4: u32,
        height_4x4: u32,
        w_class: u8,
        h_class: u8,
    ) {
        let x_end = (x4 + width_4x4).min(self.above_tx_class.len() as u32);
        for x in x4..x_end {
            self.above_tx_class[x as usize] = w_class as i8;
        }
        let y_end = (y4 + height_4x4).min(self.left_tx_class.len() as u32);
        for y in y4..y_end {
            self.left_tx_class[y as usize] = h_class as i8;
        }
    }

    /// `read_txfm_split` context `(a, l)` pair (each `0` or `1`) at absolute 4x4 position
    /// `(x4, y4)` for a candidate split of size `candidate_width_class`/`candidate_height_class`
    /// (each 0..=4, `tx_size_class` of the candidate's width/height in pixels) -- per spec/rav1d
    /// `read_tx_tree` (`src/decode.c`): `a = above_var_tx[x4] < candidate_width_class`, `l =
    /// left_var_tx[y4] < candidate_height_class` -- **width for above, height for left**, verified
    /// against the real C directly (`t->a->tx[bx4] < txw` / `t->l.tx[by4] < txh`), not assumed.
    /// For a square candidate these two class values are equal, matching this function's pre-rect
    /// single-`candidate_class` behavior exactly. Caller sums `a + l` for the CDF's `0..=2`
    /// context index (see `above_var_tx`'s doc for why the `0` default is safe).
    pub fn var_tx_context(
        &self,
        x4: u32,
        y4: u32,
        candidate_width_class: u8,
        candidate_height_class: u8,
    ) -> (u8, u8) {
        let above = self
            .above_var_tx
            .get(x4 as usize)
            .copied()
            .unwrap_or(VAR_TX_UNSET);
        let left = self
            .left_var_tx
            .get(y4 as usize)
            .copied()
            .unwrap_or(VAR_TX_UNSET);
        (
            u8::from(above < candidate_width_class as i8),
            u8::from(left < candidate_height_class as i8),
        )
    }

    /// Record a var-tx leaf's size across its own 4x4-unit footprint, for future
    /// `var_tx_context` lookups -- see `above_var_tx`'s doc. `width_class`/`height_class`: real
    /// dav1d stores the leaf's own width-derived class into the above-context array and its own
    /// height-derived class into the left-context array *separately* (not one shared class, see
    /// `var_tx_context`'s doc) -- for a square leaf these are equal, matching pre-rect behavior.
    pub fn set_var_tx_class(
        &mut self,
        x4: u32,
        y4: u32,
        width_4x4: u32,
        height_4x4: u32,
        width_class: u8,
        height_class: u8,
    ) {
        let width_class = width_class as i8;
        let height_class = height_class as i8;
        let x_end = (x4 + width_4x4).min(self.above_var_tx.len() as u32);
        for x in x4..x_end {
            self.above_var_tx[x as usize] = width_class;
        }
        let y_end = (y4 + height_4x4).min(self.left_var_tx.len() as u32);
        for y in y4..y_end {
            self.left_var_tx[y as usize] = height_class;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::symbol::SymbolDecoder;

    // `tx_size()` (intra) tests.

    #[test]
    fn test_tx_size_context_no_neighbors_is_zero() {
        let ctx = TileContext::new(16, 16);
        assert_eq!(ctx.tx_size_context(0, 0, 4, 4), 0);
    }

    #[test]
    fn test_tx_size_context_above_neighbor_at_least_as_large_contributes() {
        let mut ctx = TileContext::new(16, 16);
        ctx.set_tx_class(0, 0, 4, 4, 4, 4); // neighbor's tx class = Tx64x64 (4)
                                            // Query at (0, 5): above-only (x4=0 means have_left irrelevant here since query itself
                                            // reads left_tx_class[y4=5], untouched -> only above contributes).
        assert_eq!(ctx.tx_size_context(0, 5, 3, 3), 1); // 4 >= 3
    }

    /// The above neighbour is compared on width and the left one on height (dav1d `get_tx_ctx`):
    /// for a rectangular block the two thresholds differ.
    #[test]
    fn test_tx_size_context_compares_width_above_and_height_left() {
        let mut ctx = TileContext::new(16, 16);
        // A 16x8 transform (width class 2, height class 1) written at the origin.
        ctx.set_tx_class(0, 0, 4, 2, 2, 1);
        // Block below it: only the above array is relevant. Its width class threshold is 2.
        assert_eq!(ctx.tx_size_context(0, 2, 2, 3), 1); // above 2 >= 2
        assert_eq!(ctx.tx_size_context(0, 2, 3, 3), 0); // above 2 < 3
                                                        // Block to its right: only the left array is relevant. Its height class threshold is 1.
        assert_eq!(ctx.tx_size_context(4, 0, 3, 1), 1); // left 1 >= 1
        assert_eq!(ctx.tx_size_context(4, 0, 3, 2), 0); // left 1 < 2
    }

    #[test]
    fn test_tx_size_context_left_neighbor_smaller_does_not_contribute() {
        let mut ctx = TileContext::new(16, 16);
        ctx.set_tx_class(0, 0, 4, 4, 1, 1); // neighbor's tx class = Tx8x8 (1)
        assert_eq!(ctx.tx_size_context(5, 0, 3, 3), 0); // 1 < 3
    }

    #[test]
    fn test_tx_size_context_sums_both_neighbors() {
        let mut ctx = TileContext::new(16, 16);
        // Isolated placements: above contribution at column x4=5 (query x4), left at row y4=5.
        ctx.set_tx_class(5, 0, 4, 4, 4, 4);
        ctx.set_tx_class(0, 5, 4, 4, 4, 4);
        assert_eq!(ctx.tx_size_context(5, 5, 3, 3), 2);
    }

    #[test]
    fn test_start_superblock_row_resets_left_tx_class_but_not_above() {
        let mut ctx = TileContext::new(16, 16);
        ctx.set_tx_class(0, 0, 4, 4, 4, 4);
        ctx.start_superblock_row();
        assert_eq!(ctx.tx_size_context(5, 0, 3, 3), 0); // left reset to -1
        assert_eq!(ctx.tx_size_context(0, 5, 3, 3), 1); // above persists
    }

    // `read_var_tx_size` (inter/IntraBC var-tx) context tests.

    #[test]
    fn test_var_tx_context_no_neighbors_is_zero_zero() {
        let ctx = TileContext::new(16, 16);
        // Unwritten neighbours count as the largest class (dav1d initialises the arrays to
        // TX_64X64), so a smaller candidate gets no context from them.
        assert_eq!(ctx.var_tx_context(0, 0, 3, 3), (0, 0));
    }

    #[test]
    fn test_var_tx_context_neighbor_at_least_as_large_does_not_contribute() {
        let mut ctx = TileContext::new(16, 16);
        ctx.set_var_tx_class(0, 0, 1, 1, 3, 3); // neighbor leaf class = Tx32x32 (3)
        assert_eq!(ctx.var_tx_context(0, 1, 3, 3), (0, 0)); // above: 3 < 3 false; left unset (64x64)
    }

    #[test]
    fn test_var_tx_context_neighbor_smaller_contributes() {
        let mut ctx = TileContext::new(16, 16);
        ctx.set_var_tx_class(0, 0, 1, 1, 1, 1); // neighbor leaf class = Tx8x8 (1)
        assert_eq!(ctx.var_tx_context(0, 1, 3, 3), (1, 0)); // above: 1 < 3 true; left unset (64x64)
    }

    #[test]
    fn test_var_tx_context_width_and_height_classes_tracked_independently() {
        // A rectangular neighbor (e.g. 32x16, width_class=3, height_class=2): above-context sees
        // the wider dim, left-context sees the shorter one -- distinct from either alone.
        let mut ctx = TileContext::new(16, 16);
        ctx.set_var_tx_class(0, 0, 1, 1, 3, 2);
        // above: candidate width_class=3, neighbor above=3 -> 3<3 false.
        // left: candidate height_class=3, neighbor left=2 -> 2<3 true.
        assert_eq!(ctx.var_tx_context(0, 0, 3, 3), (0, 1));
    }

    #[test]
    fn test_read_tx_size_class_zero_reads_zero_bits() {
        let data = vec![0x80, 0x00, 0xFF, 0xFF, 0xAA, 0xBB];
        let mut decoder = SymbolDecoder::new(&data).unwrap();
        let before = (
            decoder.decoder.range,
            decoder.decoder.value,
            decoder.decoder.cnt,
        );
        let resolved = decoder.read_tx_size(0, 0).unwrap();
        assert_eq!(resolved, 0);
        assert_eq!(
            (
                decoder.decoder.range,
                decoder.decoder.value,
                decoder.decoder.cnt
            ),
            before
        );
    }

    #[test]
    fn test_read_tx_size_class_reads_real_bits_and_never_exceeds_max() {
        let data = vec![0x80, 0x00, 0xFF, 0xFF, 0xAA, 0xBB];
        let mut decoder = SymbolDecoder::new(&data).unwrap();
        let before = (
            decoder.decoder.range,
            decoder.decoder.value,
            decoder.decoder.cnt,
        );
        let resolved = decoder.read_tx_size(4, 1).unwrap();
        assert!(resolved <= 4);
        assert_ne!(
            (
                decoder.decoder.range,
                decoder.decoder.value,
                decoder.decoder.cnt
            ),
            before
        );
    }
}
