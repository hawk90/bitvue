//! Above/left neighbor-state tracking for entropy-context derivation.
//!
//! Per AV1 spec Section 9.3 (Function `get_ctx`) and rav1d's `BlockContext`
//! (`memorysafety/rav1d`, BSD-2-Clause, `src/env.rs`) -- currently covers the `skip` flag's
//! context (see `SymbolDecoder::read_skip`'s doc) and key-frame `intra_mode`'s context (see
//! `SymbolDecoder::read_intra_mode`'s doc). Real partition-context and inter/compound-mode
//! context derivation both need considerably larger ports deferred to later phases -- partition
//! needs a per-8x8 bitmask tied to dav1d's edge-index tree (`src/decode.rs`'s `decode_sb`);
//! inter/compound mode context needs dav1d's reference-motion-vector-candidate subsystem
//! (`src/refmvs.rs`, `rav1d_refmvs_find`) -- see `docs/DEVELOPMENT_PHASES.md` Phase 4's AV1
//! entropy-decoding note.
//!
//! Units throughout are 4x4 pixels (spec's context-array granularity).

/// Maps a raw intra prediction-mode symbol (0..=12, spec `y_mode`/`uv_mode` values -- matches
/// `bitvue_av1_codec::tile::PredictionMode`'s intra-variant declaration order exactly) to one of
/// 5 mode-context classes used to index the key-frame `kfym` CDF. Source: rav1d
/// `DAV1D_INTRA_MODE_CONTEXT` (`memorysafety/rav1d`, BSD-2-Clause, `src/tables.rs`).
const INTRA_MODE_CONTEXT: [u8; 13] = [0, 1, 2, 3, 4, 4, 4, 4, 3, 0, 1, 2, 0];

/// `DAV1D_AL_PART_CTX[dir][bl][bp]` -- the bitmask written into `above_partition`/`left_partition`
/// after a `partition` symbol `bp` is decided at level `bl`, one row per `bl` (0=128x128..4=8x8,
/// rav1d's `BlockLevel` convention -- opposite of `block_size_log2`, see `partition_bl`), one
/// column per `PartitionType` (0=None..9=Vert4, matches this crate's `PartitionType` enum
/// ordering exactly). `0xff` marks a partition type that's invalid at that level (never actually
/// written in practice, since callers only reach `set_partition` for partition types
/// `PartitionType::is_allowed` at the block's own size already validated). Source: rav1d
/// `DAV1D_AL_PART_CTX` (`memorysafety/rav1d`, BSD-2-Clause, `src/tables.rs`).
const PARTITION_CTX_TABLE: [[[u8; 10]; 5]; 2] = [
    // above
    [
        [0x00, 0x00, 0x10, 0xff, 0x00, 0x10, 0x10, 0x10, 0xff, 0xff],
        [0x10, 0x10, 0x18, 0xff, 0x10, 0x18, 0x18, 0x18, 0x10, 0x1c],
        [0x18, 0x18, 0x1c, 0xff, 0x18, 0x1c, 0x1c, 0x1c, 0x18, 0x1e],
        [0x1c, 0x1c, 0x1e, 0xff, 0x1c, 0x1e, 0x1e, 0x1e, 0x1c, 0x1f],
        [0x1e, 0x1e, 0x1f, 0x1f, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff],
    ],
    // left
    [
        [0x00, 0x10, 0x00, 0xff, 0x10, 0x10, 0x00, 0x10, 0xff, 0xff],
        [0x10, 0x18, 0x10, 0xff, 0x18, 0x18, 0x10, 0x18, 0x1c, 0x10],
        [0x18, 0x1c, 0x18, 0xff, 0x1c, 0x1c, 0x18, 0x1c, 0x1e, 0x18],
        [0x1c, 0x1e, 0x1c, 0xff, 0x1e, 0x1e, 0x1c, 0x1e, 0x1f, 0x1c],
        [0x1e, 0x1f, 0x1e, 0x1f, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff],
    ],
];

/// Maps `block_size_log2` (2..=7, this crate's CDF-lookup convention) to rav1d's `BlockLevel`
/// (0=128x128..4=8x8 -- opposite numeric order). Only valid for `block_size_log2` in 3..=7 (8x8
/// and up); 4x4 blocks (log2=2) never read a `partition` symbol at all, so have no `BlockLevel`.
pub fn partition_bl(block_size_log2: u8) -> u8 {
    7 - block_size_log2.clamp(3, 7)
}

/// Tracks above/left neighbor state for one tile, at 4x4-unit granularity.
///
/// `above_*` arrays span the tile's full width and persist for the whole tile (matches spec: the
/// above-context row is only reset at a new tile, not at every superblock row). `left_*` arrays
/// span the tile's full height and are addressed with the same absolute 4x4 coordinates as the
/// `above_*` ones -- real dav1d instead sizes its left-context column to one superblock and
/// addresses it with row-relative offsets (a memory/cache optimization for a real-time decoder);
/// this crate isn't performance-constrained the same way, so `start_superblock_row` simply clears
/// the whole array at each new superblock row, which is behaviorally equivalent (spec's
/// left-context only ever remembers state from within the current superblock row of a
/// raster-scanned tile) without needing relative-offset bookkeeping at every call site.
pub struct TileContext {
    above_skip: Vec<bool>,
    left_skip: Vec<bool>,
    /// Raw intra-mode symbol (0..=12) of the last block covering this 4x4 position, defaulting
    /// to `0` (`DC_PRED`) -- matches dav1d's `BlockContext::mode` default/edge behavior (an
    /// unwritten position reads as `DC_PRED`'s context class, per spec `INTRA_MODE_CONTEXT[0]`).
    above_mode: Vec<u8>,
    left_mode: Vec<u8>,
    /// `partition` context bitmask, one byte per 8x8 unit (not 4x4, like every other field here --
    /// see `PARTITION_CTX_TABLE`'s doc). Sized to half the tile's 4x4-unit extent (rounded up).
    above_partition: Vec<u8>,
    left_partition: Vec<u8>,
}

impl TileContext {
    /// `tile_width_4x4`/`tile_height_4x4`: tile dimensions in 4x4 units.
    pub fn new(tile_width_4x4: u32, tile_height_4x4: u32) -> Self {
        Self {
            above_skip: vec![false; tile_width_4x4.max(1) as usize],
            left_skip: vec![false; tile_height_4x4.max(1) as usize],
            above_mode: vec![0; tile_width_4x4.max(1) as usize],
            left_mode: vec![0; tile_height_4x4.max(1) as usize],
            above_partition: vec![0; tile_width_4x4.div_ceil(2).max(1) as usize],
            left_partition: vec![0; tile_height_4x4.div_ceil(2).max(1) as usize],
        }
    }

    /// Reset the left-context arrays at the start of each new superblock row.
    pub fn start_superblock_row(&mut self) {
        self.left_skip.iter_mut().for_each(|v| *v = false);
        self.left_mode.iter_mut().for_each(|v| *v = 0);
        self.left_partition.iter_mut().for_each(|v| *v = 0);
    }

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

    /// `skip` context index (0..=2) for a block at absolute 4x4 position `(x4, y4)` -- per
    /// spec/dav1d: `above_skip[x4] as u8 + left_skip[y4] as u8`.
    pub fn skip_context(&self, x4: u32, y4: u32) -> u8 {
        let above = self.above_skip.get(x4 as usize).copied().unwrap_or(false);
        let left = self.left_skip.get(y4 as usize).copied().unwrap_or(false);
        u8::from(above) + u8::from(left)
    }

    /// Record a decoded `skip` flag across the block's 4x4-unit footprint, for future context
    /// lookups -- per spec/dav1d, every 4x4 unit the block covers gets the same value in both the
    /// above-row and left-column arrays (`CaseSet::set_disjoint`/`case.set_disjoint` in rav1d).
    pub fn set_skip(&mut self, x4: u32, y4: u32, width_4x4: u32, height_4x4: u32, skip: bool) {
        let x_end = (x4 + width_4x4).min(self.above_skip.len() as u32);
        for x in x4..x_end {
            self.above_skip[x as usize] = skip;
        }
        let y_end = (y4 + height_4x4).min(self.left_skip.len() as u32);
        for y in y4..y_end {
            self.left_skip[y as usize] = skip;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skip_context_starts_at_zero_with_no_neighbors() {
        let ctx = TileContext::new(16, 16);
        assert_eq!(ctx.skip_context(0, 0), 0);
    }

    #[test]
    fn test_skip_context_reflects_above_neighbor() {
        let mut ctx = TileContext::new(16, 16);
        ctx.set_skip(0, 0, 4, 4, true);
        // A block directly to the right, same 4x4 row -- shares the above-context column but not
        // the left-context row, so only the "above" contribution applies here since we're
        // querying the SAME above-row position that was written.
        assert_eq!(ctx.skip_context(0, 5), 1);
    }

    #[test]
    fn test_skip_context_reflects_left_neighbor() {
        let mut ctx = TileContext::new(16, 16);
        ctx.set_skip(0, 0, 4, 4, true);
        assert_eq!(ctx.skip_context(5, 0), 1);
    }

    #[test]
    fn test_skip_context_sums_both_neighbors() {
        let mut ctx = TileContext::new(16, 16);
        ctx.set_skip(0, 0, 4, 4, true);
        assert_eq!(ctx.skip_context(0, 0), 2);
    }

    #[test]
    fn test_start_superblock_row_resets_left_but_not_above() {
        let mut ctx = TileContext::new(16, 16);
        ctx.set_skip(0, 0, 4, 4, true);
        ctx.start_superblock_row();
        // Left context cleared...
        assert_eq!(ctx.skip_context(5, 0), 0);
        // ...but above context (tile-lifetime) persists.
        assert_eq!(ctx.skip_context(0, 5), 1);
    }

    #[test]
    fn test_set_skip_clamps_to_array_bounds() {
        // A block whose footprint would run past the tile/superblock edge must not panic.
        let mut ctx = TileContext::new(4, 4);
        ctx.set_skip(2, 2, 100, 100, true);
        assert_eq!(ctx.skip_context(3, 3), 2);
    }

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
