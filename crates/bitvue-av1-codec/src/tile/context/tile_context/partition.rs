//! `partition` context (per-8x8 above/left bitmasks).

use super::TileContext;
use crate::tile::context::tables::PARTITION_CTX_TABLE;

impl TileContext {
    /// `partition` context index (0..=3) for a block at absolute 8x8-unit position `(x8, y8)`,
    /// per spec/rav1d: `has_bl(above_partition[x8]) + 2*has_bl(left_partition[y8])`, where
    /// `has_bl` extracts the bit for the current level `bl` (0=128x128..4=8x8, see
    /// `partition_bl`) from the neighbor's stored bitmask.
    pub fn partition_context(&self, x8: u32, y8: u32, bl: u8) -> u8 {
        let has_bl = |v: u8| (v >> (4 - bl)) & 1;
        let above = self.above_partition.get(x8 as usize).copied().unwrap_or(0);
        let left = self.left_partition.get(y8 as usize).copied().unwrap_or(0);
        has_bl(above) + 2 * has_bl(left)
    }

    /// Record a decoded `partition` symbol across the block's 8x8-unit footprint (`hsz8` units
    /// wide/tall -- `2^(block_size_log2-3)`), for future context lookups. Per spec/rav1d, the
    /// written value isn't the raw symbol but `PARTITION_CTX_TABLE[dir][bl][partition_symbol]` --
    /// see that table's doc for why this is a lookup, not the symbol value itself.
    pub fn set_partition(&mut self, x8: u32, y8: u32, hsz8: u32, bl: u8, partition_symbol: u8) {
        let above_val = PARTITION_CTX_TABLE[0][bl as usize][partition_symbol as usize];
        let left_val = PARTITION_CTX_TABLE[1][bl as usize][partition_symbol as usize];
        let x_end = (x8 + hsz8).min(self.above_partition.len() as u32);
        for x in x8..x_end {
            self.above_partition[x as usize] = above_val;
        }
        let y_end = (y8 + hsz8).min(self.left_partition.len() as u32);
        for y in y8..y_end {
            self.left_partition[y as usize] = left_val;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tile::context::partition_bl;

    #[test]
    fn test_partition_context_starts_at_zero_with_no_neighbors() {
        let ctx = TileContext::new(16, 16);
        assert_eq!(ctx.partition_context(0, 0, 4), 0);
    }

    #[test]
    fn test_partition_context_reflects_above_split_at_8x8() {
        let mut ctx = TileContext::new(16, 16);
        // Split (symbol 3) at 8x8 (bl=4) sets bit 0 of PARTITION_CTX_TABLE's value (0x1f), which
        // `has_bl` for bl=4 reads directly off bit 0.
        ctx.set_partition(0, 0, 1, 4, 3);
        // Query at the same above-column (x8=0), different row -- above contribution only.
        assert_eq!(ctx.partition_context(0, 5, 4), 1);
    }

    #[test]
    fn test_partition_context_reflects_left_split_at_8x8() {
        let mut ctx = TileContext::new(16, 16);
        ctx.set_partition(0, 0, 1, 4, 3);
        assert_eq!(ctx.partition_context(5, 0, 4), 2);
    }

    #[test]
    fn test_partition_context_sums_both_neighbors() {
        let mut ctx = TileContext::new(16, 16);
        ctx.set_partition(0, 0, 1, 4, 3);
        assert_eq!(ctx.partition_context(0, 0, 4), 3);
    }

    #[test]
    fn test_partition_context_none_clears_the_bit() {
        let mut ctx = TileContext::new(16, 16);
        ctx.set_partition(0, 0, 1, 4, 3); // Split first
        ctx.set_partition(0, 0, 1, 4, 0); // None overwrites (0x1e has bit 0 clear)
        assert_eq!(ctx.partition_context(0, 5, 4), 0);
    }

    #[test]
    fn test_start_superblock_row_resets_left_partition_but_not_above() {
        let mut ctx = TileContext::new(16, 16);
        ctx.set_partition(0, 0, 1, 4, 3);
        ctx.start_superblock_row();
        assert_eq!(ctx.partition_context(5, 0, 4), 0); // left reset
        assert_eq!(ctx.partition_context(0, 5, 4), 1); // above persists
    }

    #[test]
    fn test_set_partition_clamps_to_array_bounds() {
        let mut ctx = TileContext::new(4, 4);
        ctx.set_partition(0, 0, 100, 4, 3); // footprint far exceeds the array; must not panic
        assert_eq!(ctx.partition_context(0, 0, 4), 3);
    }

    #[test]
    fn test_partition_ctx_table_matches_rav1d_dav1d_al_part_ctx() {
        // Source: rav1d `DAV1D_AL_PART_CTX` (`src/tables.rs`). Pinning both extremes (128x128's
        // mostly-0xff row, and 8x8's real row) catches an accidental edit.
        assert_eq!(
            PARTITION_CTX_TABLE[0][0],
            [0x00, 0x00, 0x10, 0xff, 0x00, 0x10, 0x10, 0x10, 0xff, 0xff]
        );
        assert_eq!(
            PARTITION_CTX_TABLE[1][4],
            [0x1e, 0x1f, 0x1e, 0x1f, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff]
        );
    }

    #[test]
    fn test_partition_bl_maps_block_size_log2_to_rav1d_block_level() {
        assert_eq!(partition_bl(7), 0); // 128x128 -> rav1d BlockLevel 0
        assert_eq!(partition_bl(3), 4); // 8x8 -> rav1d BlockLevel 4
        assert_eq!(partition_bl(5), 2); // 32x32 -> rav1d BlockLevel 2
    }
}
