//! `txb_skip` and `dc_sign` contexts (luma and chroma) and the state they read.

use super::TileContext;
use crate::tile::context::tables::DAV1D_SKIP_CTX;

impl TileContext {
    /// `txb_skip` (all_zero) context index for one transform block at absolute 4x4 position
    /// `(x4, y4)`, `tx_w4`/`tx_h4` 4x4 units wide/tall -- per spec/rav1d `get_skip_ctx`'s luma
    /// branch. `is_single_tx_block`: true when this transform block is the coding block's only
    /// one (`tx_cols == 1 && tx_rows == 1`), matching rav1d's `b_dim[2] == t_dim->lw && b_dim[3]
    /// == t_dim->lh` immediate-zero case -- no neighbor lookup needed there, context is always
    /// `0`. Otherwise: OR-reduce `cul_level` across the `tx_w4` above-row / `tx_h4` left-column
    /// positions this transform block's row/column spans (a real bitwise OR of the raw values,
    /// not a boolean "any nonzero" -- see this method's own struct-field doc), then index
    /// `DAV1D_SKIP_CTX[min(la,4)][min(ll,4)]`.
    pub fn txb_skip_context(
        &self,
        x4: u32,
        y4: u32,
        tx_w4: u32,
        tx_h4: u32,
        is_single_tx_block: bool,
    ) -> u8 {
        if is_single_tx_block {
            return 0;
        }
        let la = (0..tx_w4)
            .map(|i| {
                self.above_cul_level
                    .get((x4 + i) as usize)
                    .copied()
                    .unwrap_or(0)
            })
            .fold(0u32, |acc, v| acc | v as u32);
        let ll = (0..tx_h4)
            .map(|i| {
                self.left_cul_level
                    .get((y4 + i) as usize)
                    .copied()
                    .unwrap_or(0)
            })
            .fold(0u32, |acc, v| acc | v as u32);
        DAV1D_SKIP_CTX[la.min(4) as usize][ll.min(4) as usize]
    }

    /// `dc_sign` context index (0..=2) for one transform block at absolute 4x4 position
    /// `(x4, y4)`, `tx_w4`/`tx_h4` 4x4 units wide/tall -- per spec/rav1d `get_dc_sign_ctx`'s luma
    /// branch: sum `(dc_sign_category - 1)` across the above-row and left-column positions this
    /// transform block spans (a real per-position sum, not an OR -- neutral positions contribute
    /// `0`, negative `-1`, positive `+1`), then classify the sign of that sum into `{0,1,2}`.
    pub fn dc_sign_context(&self, x4: u32, y4: u32, tx_w4: u32, tx_h4: u32) -> u8 {
        let above_sum: i32 = (0..tx_w4)
            .map(|i| {
                self.above_dc_sign_category
                    .get((x4 + i) as usize)
                    .copied()
                    .unwrap_or(1) as i32
                    - 1
            })
            .sum();
        let left_sum: i32 = (0..tx_h4)
            .map(|i| {
                self.left_dc_sign_category
                    .get((y4 + i) as usize)
                    .copied()
                    .unwrap_or(1) as i32
                    - 1
            })
            .sum();
        dc_sign_ctx_from_sum(above_sum + left_sum)
    }

    /// Record one decoded transform block's `cul_level`/`dc_sign` state across its 4x4-unit
    /// footprint, for future `txb_skip_context`/`dc_sign_context` lookups. `cul_level`: `min(63,
    /// sum of absolute coefficient levels)` (already clamped by the caller). `dc_sign_symbol`:
    /// the raw `dc_sign` bit read for this block's DC coefficient (`Some(1)`=negative,
    /// `Some(0)`=positive), or `None` when no `dc_sign` bit was read at all -- an all-zero
    /// (`txb_skip`) block, or a block whose DC position happened to decode to a zero level --
    /// both cases mean "neutral" (category `1`), matching rav1d's `all_skip`/`dc_tok==0` paths
    /// (see the struct field's doc).
    pub fn set_residual_ctx(
        &mut self,
        x4: u32,
        y4: u32,
        tx_w4: u32,
        tx_h4: u32,
        cul_level: u8,
        dc_sign_symbol: Option<u8>,
    ) {
        let category = match dc_sign_symbol {
            None => 1,
            Some(1) => 0,
            Some(_) => 2,
        };
        let x_end = (x4 + tx_w4).min(self.above_cul_level.len() as u32);
        for x in x4..x_end {
            self.above_cul_level[x as usize] = cul_level;
            self.above_dc_sign_category[x as usize] = category;
        }
        let y_end = (y4 + tx_h4).min(self.left_cul_level.len() as u32);
        for y in y4..y_end {
            self.left_cul_level[y as usize] = cul_level;
            self.left_dc_sign_category[y as usize] = category;
        }
    }

    /// Chroma `txb_skip` context index -- local remap of spec/rav1d's real chroma branch of
    /// `get_skip_ctx` (`recon_tmpl.c:68-100`, `memorysafety/rav1d`/`videolan/dav1d`,
    /// BSD-2-Clause): real ctx there is `7 + not_one_blk*3 + ca + cl` (13-context table shared
    /// with luma's 7); since this crate's chroma CDF table is a separate `[tx_size_class][0..=5]`
    /// array (no shared luma/chroma axis), the `+7` is dropped and `ca`/`cl` are computed as a
    /// plain boolean OR across the tx block's above/left footprint (`cul_level != 0` = "neighbor
    /// wasn't all-zero") rather than dav1d's bit-packed `0x40`-sentinel trick -- same result,
    /// simpler representation. `not_one_blk`: true when this CU's chroma plane needed more than
    /// one chroma transform block (`chroma_tile_count > 1` at the call site) -- spec: whether the
    /// chroma prediction block exceeds one max-chroma-tx-size tile.
    pub fn txb_skip_context_chroma(
        &self,
        plane: usize,
        cx4: u32,
        cy4: u32,
        tx_w4: u32,
        tx_h4: u32,
        not_one_blk: bool,
    ) -> u8 {
        let above = &self.above_cul_level_chroma[plane.min(1)];
        let left = &self.left_cul_level_chroma[plane.min(1)];
        let ca = (0..tx_w4).any(|i| above.get((cx4 + i) as usize).copied().unwrap_or(0) != 0);
        let cl = (0..tx_h4).any(|i| left.get((cy4 + i) as usize).copied().unwrap_or(0) != 0);
        u8::from(not_one_blk) * 3 + u8::from(ca) + u8::from(cl)
    }

    /// Chroma `dc_sign` context -- identical algorithm to `dc_sign_context` (spec/rav1d's
    /// `get_dc_sign_ctx` doesn't differ between luma/chroma except which above/left array it's
    /// given), applied to `plane`'s own category arrays.
    pub fn dc_sign_context_chroma(
        &self,
        plane: usize,
        cx4: u32,
        cy4: u32,
        tx_w4: u32,
        tx_h4: u32,
    ) -> u8 {
        let above = &self.above_dc_sign_category_chroma[plane.min(1)];
        let left = &self.left_dc_sign_category_chroma[plane.min(1)];
        let above_sum: i32 = (0..tx_w4)
            .map(|i| above.get((cx4 + i) as usize).copied().unwrap_or(1) as i32 - 1)
            .sum();
        let left_sum: i32 = (0..tx_h4)
            .map(|i| left.get((cy4 + i) as usize).copied().unwrap_or(1) as i32 - 1)
            .sum();
        dc_sign_ctx_from_sum(above_sum + left_sum)
    }

    /// Chroma counterpart to `set_residual_ctx`, per plane (`0`=U, `1`=V).
    #[allow(clippy::too_many_arguments)]
    pub fn set_residual_ctx_chroma(
        &mut self,
        plane: usize,
        cx4: u32,
        cy4: u32,
        tx_w4: u32,
        tx_h4: u32,
        cul_level: u8,
        dc_sign_symbol: Option<u8>,
    ) {
        let category = match dc_sign_symbol {
            None => 1,
            Some(1) => 0,
            Some(_) => 2,
        };
        let plane = plane.min(1);
        let above = &mut self.above_cul_level_chroma[plane];
        let above_cat = &mut self.above_dc_sign_category_chroma[plane];
        let x_end = (cx4 + tx_w4).min(above.len() as u32);
        for x in cx4..x_end {
            above[x as usize] = cul_level;
            above_cat[x as usize] = category;
        }
        let left = &mut self.left_cul_level_chroma[plane];
        let left_cat = &mut self.left_dc_sign_category_chroma[plane];
        let y_end = (cy4 + tx_h4).min(left.len() as u32);
        for y in cy4..y_end {
            left[y as usize] = cul_level;
            left_cat[y as usize] = category;
        }
    }
}

/// dav1d `get_dc_sign_ctx`'s final step, `(s != 0) + (s > 0)`: `0` when the neighbours' DC signs
/// balance out (or are all neutral), `1` when negative ones dominate, `2` when positive ones do.
/// This is also the order of `dc_sign`'s default CDFs (`[16000, 13056, 18816]`, neutral first).
fn dc_sign_ctx_from_sum(sum: i32) -> u8 {
    u8::from(sum != 0) + u8::from(sum > 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_txb_skip_context_single_tx_block_is_always_zero() {
        let mut ctx = TileContext::new(16, 16);
        ctx.set_residual_ctx(0, 0, 4, 4, 63, None);
        assert_eq!(ctx.txb_skip_context(0, 0, 4, 4, true), 0);
    }

    #[test]
    fn test_txb_skip_context_no_neighbors_is_table_zero_zero() {
        let ctx = TileContext::new(16, 16);
        assert_eq!(
            ctx.txb_skip_context(0, 0, 2, 2, false),
            DAV1D_SKIP_CTX[0][0]
        );
    }

    #[test]
    fn test_txb_skip_context_above_neighbor_ors_in() {
        let mut ctx = TileContext::new(16, 16);
        ctx.set_residual_ctx(0, 0, 4, 4, 5, None);
        // Query at (0, 5): above-only (left_cul_level[5] untouched by the write above).
        assert_eq!(
            ctx.txb_skip_context(0, 5, 2, 2, false),
            DAV1D_SKIP_CTX[4][0]
        );
    }

    #[test]
    fn test_txb_skip_context_left_neighbor_ors_in() {
        let mut ctx = TileContext::new(16, 16);
        ctx.set_residual_ctx(0, 0, 4, 4, 5, None);
        // Query at (5, 0): left-only (above_cul_level[5] untouched by the write above).
        assert_eq!(
            ctx.txb_skip_context(5, 0, 2, 2, false),
            DAV1D_SKIP_CTX[0][4]
        );
    }

    #[test]
    fn test_txb_skip_context_combines_above_and_left() {
        let mut ctx = TileContext::new(16, 16);
        ctx.set_residual_ctx(0, 0, 2, 2, 5, None);
        assert_eq!(
            ctx.txb_skip_context(0, 0, 2, 2, false),
            DAV1D_SKIP_CTX[4][4]
        );
    }

    #[test]
    fn test_dc_sign_context_no_neighbors_is_neutral() {
        let ctx = TileContext::new(16, 16);
        assert_eq!(ctx.dc_sign_context(0, 0, 2, 2), 0);
    }

    #[test]
    fn test_dc_sign_context_positive_neighbors_is_positive() {
        let mut ctx = TileContext::new(16, 16);
        ctx.set_residual_ctx(0, 0, 2, 2, 10, Some(0)); // dc_sign=0 -> positive category
        assert_eq!(ctx.dc_sign_context(0, 0, 2, 2), 2);
    }

    #[test]
    fn test_dc_sign_context_negative_neighbors_is_negative() {
        let mut ctx = TileContext::new(16, 16);
        ctx.set_residual_ctx(0, 0, 2, 2, 10, Some(1)); // dc_sign=1 -> negative category
        assert_eq!(ctx.dc_sign_context(0, 0, 2, 2), 1);
    }

    #[test]
    fn test_dc_sign_context_sums_both_neighbors_and_can_cancel() {
        let mut ctx = TileContext::new(16, 16);
        ctx.set_residual_ctx(5, 0, 1, 1, 5, Some(0)); // above[5] positive
        ctx.set_residual_ctx(0, 5, 1, 1, 5, Some(1)); // left[5] negative
        assert_eq!(ctx.dc_sign_context(5, 5, 1, 1), 0); // +1 and -1 cancel to neutral
    }

    #[test]
    fn test_set_residual_ctx_all_zero_block_writes_neutral_state() {
        let mut ctx = TileContext::new(16, 16);
        ctx.set_residual_ctx(0, 0, 2, 2, 0, None);
        assert_eq!(
            ctx.txb_skip_context(0, 0, 2, 2, false),
            DAV1D_SKIP_CTX[0][0]
        );
        assert_eq!(ctx.dc_sign_context(0, 0, 2, 2), 0);
    }

    #[test]
    fn test_start_superblock_row_resets_left_residual_ctx_but_not_above() {
        let mut ctx = TileContext::new(16, 16);
        ctx.set_residual_ctx(0, 0, 2, 2, 20, Some(0));
        assert_eq!(
            ctx.txb_skip_context(0, 0, 2, 2, false),
            DAV1D_SKIP_CTX[4][4]
        );
        ctx.start_superblock_row();
        assert_eq!(
            ctx.txb_skip_context(0, 0, 2, 2, false),
            DAV1D_SKIP_CTX[4][0]
        );
    }
}
