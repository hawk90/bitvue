//! `skip` and `skip_mode` contexts.

use super::TileContext;

impl TileContext {
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

    /// `skip_mode` context index (0..=2) -- identical shape to `skip_context`, per dav1d
    /// `smctx = t->a->skip_mode[bx4] + t->l.skip_mode[by4]` (no `have_top`/`have_left` branch,
    /// unlike `intra_ctx`).
    pub fn skip_mode_context(&self, x4: u32, y4: u32) -> u8 {
        let above = self
            .above_skip_mode
            .get(x4 as usize)
            .copied()
            .unwrap_or(false);
        let left = self
            .left_skip_mode
            .get(y4 as usize)
            .copied()
            .unwrap_or(false);
        u8::from(above) + u8::from(left)
    }

    /// Record a decoded `skip_mode` flag across the block's 4x4-unit footprint -- mirrors
    /// `set_skip` exactly, separate array (`skip_mode_context`'s doc).
    pub fn set_skip_mode(
        &mut self,
        x4: u32,
        y4: u32,
        width_4x4: u32,
        height_4x4: u32,
        skip_mode: bool,
    ) {
        let x_end = (x4 + width_4x4).min(self.above_skip_mode.len() as u32);
        for x in x4..x_end {
            self.above_skip_mode[x as usize] = skip_mode;
        }
        let y_end = (y4 + height_4x4).min(self.left_skip_mode.len() as u32);
        for y in y4..y_end {
            self.left_skip_mode[y as usize] = skip_mode;
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
