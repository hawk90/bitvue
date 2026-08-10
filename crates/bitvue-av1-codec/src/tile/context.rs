//! Above/left neighbor-state tracking for entropy-context derivation.
//!
//! Per AV1 spec Section 9.3 (Function `get_ctx`) and rav1d's `BlockContext`
//! (`memorysafety/rav1d`, BSD-2-Clause, `src/env.rs`) -- currently covers only the `skip` flag's
//! context (see `SymbolDecoder::read_skip`'s doc). Real partition-context derivation needs a
//! per-8x8 bitmask tied to dav1d's edge-index tree (`src/decode.rs`'s `decode_sb`), a
//! considerably larger port deferred to a later phase -- see `docs/DEVELOPMENT_PHASES.md` Phase 4's
//! AV1 entropy-decoding note.
//!
//! Units throughout are 4x4 pixels (spec's context-array granularity).

/// Tracks above/left neighbor state for one tile, at 4x4-unit granularity.
///
/// `above_skip` spans the tile's full width and persists for the whole tile (matches spec: the
/// above-context row is only reset at a new tile, not at every superblock row). `left_skip` spans
/// the tile's full height and is addressed with the same absolute 4x4 coordinates as
/// `above_skip` -- real dav1d instead sizes its left-context column to one superblock and
/// addresses it with row-relative offsets (a memory/cache optimization for a real-time decoder);
/// this crate isn't performance-constrained the same way, so `start_superblock_row` simply clears
/// the whole array at each new superblock row, which is behaviorally equivalent (spec's
/// left-context only ever remembers state from within the current superblock row of a
/// raster-scanned tile) without needing relative-offset bookkeeping at every call site.
pub struct TileContext {
    above_skip: Vec<bool>,
    left_skip: Vec<bool>,
}

impl TileContext {
    /// `tile_width_4x4`/`tile_height_4x4`: tile dimensions in 4x4 units.
    pub fn new(tile_width_4x4: u32, tile_height_4x4: u32) -> Self {
        Self {
            above_skip: vec![false; tile_width_4x4.max(1) as usize],
            left_skip: vec![false; tile_height_4x4.max(1) as usize],
        }
    }

    /// Reset the left-context array at the start of each new superblock row.
    pub fn start_superblock_row(&mut self) {
        self.left_skip.iter_mut().for_each(|v| *v = false);
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
}
