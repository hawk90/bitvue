//! Coefficient-level scratch buffer and the `coeff_base`/`coeff_br` neighbor-context derivation.

/// `dav1d_lo_ctx_offsets[0]` (the `w == h` square variant). Indexed `[min(y,4)][min(x,4)]`.
/// Source: rav1d `dav1d_lo_ctx_offsets` (`src/tables.c`).
const LO_CTX_OFFSETS_SQUARE: [[u8; 5]; 5] = [
    [0, 1, 6, 6, 21],
    [1, 6, 6, 21, 21],
    [6, 6, 21, 21, 21],
    [6, 21, 21, 21, 21],
    [21, 21, 21, 21, 21],
];
/// `dav1d_lo_ctx_offsets[1]` (`width > height`). Source: rav1d `dav1d_lo_ctx_offsets`
/// (`src/tables.c`) -- real per-aspect-ratio context table for genuinely rectangular transforms
/// (see this module's doc; `nonsquare_tx + (tx & nonsquare_tx)` selects index `1` for every
/// wider-than-tall `RectTxfmSize`, verified against the real enum ordering).
const LO_CTX_OFFSETS_WIDE: [[u8; 5]; 5] = [
    [0, 16, 6, 6, 21],
    [16, 16, 6, 21, 21],
    [16, 16, 21, 21, 21],
    [16, 16, 21, 21, 21],
    [16, 16, 21, 21, 21],
];
/// `dav1d_lo_ctx_offsets[2]` (`width < height`). Source: rav1d `dav1d_lo_ctx_offsets`
/// (`src/tables.c`).
const LO_CTX_OFFSETS_TALL: [[u8; 5]; 5] = [
    [0, 11, 11, 11, 11],
    [11, 11, 11, 11, 11],
    [6, 6, 21, 21, 21],
    [6, 21, 21, 21, 21],
    [21, 21, 21, 21, 21],
];

/// Per-transform-block coefficient-level scratch state (dav1d's `t->scratch.levels`, unpacked --
/// see this module's doc). Cleared per transform block (`LevelBuffer::new`); stores each decoded
/// position's token value scaled `tok * 65` when unextended, `tok + 192` when `coeff_br`-extended
/// -- matches dav1d's `tok * 0x41` / `tok + (3 << 6)` byte encoding exactly, since `lo_ctx`'s
/// magnitude sum treats the raw stored byte as the "how significant is this neighbor" signal, not
/// a literal level -- porting the encoding, not just the final decoded value, is required for the
/// context bucket math below to match real spec. Sized `(width+2)*(height+2)` (generalized from
/// the original square-only `(dim+2)^2` for rectangular transforms -- `x` ranges `0..width`, `y`
/// ranges `0..height`, matching `coeff_position`'s convention) so neighbor reads past the
/// transform block's low-frequency edge (`x+1`/`x+2`/`y+1`/`y+2` beyond bounds) read real zeros
/// without explicit bounds checks, matching dav1d's own padding.
pub struct LevelBuffer {
    data: Vec<u8>,
    stride: usize,
}

impl LevelBuffer {
    /// `width`/`height`: the transform's dimensions. The buffer is square in the larger one: for
    /// the 1D classes `coeff_position` transposes the grid (`TX_CLASS_H` indexes `x` along the
    /// height and `y` along the width), so either axis may be the long one, and an index outside
    /// the allocation would silently read as zero / drop its write (see `get`/`set`).
    pub fn new(width: usize, height: usize) -> Self {
        let stride = width.max(height) + 2;
        Self {
            data: vec![0u8; stride * stride],
            stride,
        }
    }

    fn get(&self, x: u32, y: u32) -> u32 {
        self.data
            .get(x as usize * self.stride + y as usize)
            .copied()
            .unwrap_or(0) as u32
    }

    /// Record one decoded position's token, scaled per this struct's doc. `level`: the raw
    /// pre-golomb token/magnitude (`base_level`, or the post-`coeff_br`-extension value when
    /// `extended`).
    pub fn set(&mut self, x: u32, y: u32, extended: bool, level: u32) {
        let idx = x as usize * self.stride + y as usize;
        debug_assert!(idx < self.data.len(), "level written outside the buffer");
        if let Some(slot) = self.data.get_mut(idx) {
            *slot = if extended {
                (level.min(63) + 192) as u8
            } else {
                (level.min(3) * 65) as u8
            };
        }
    }
}

/// `coeff_base`'s real context (return `.0`, `0..=25` for 2D / `26..=40` for H/V, matching
/// `CdfContext::base_tok`'s 41-context shape) and the 3-neighbor magnitude sum (return `.1`,
/// `coeff_br`'s `hi_mag` -- reused directly by callers computing `coeff_br`'s context, see
/// `SymbolDecoder::read_residual_block`'s doc) for the transform-block-local position `(x, y)`.
/// Per rav1d `get_lo_ctx` (`src/recon_tmpl.c`): sums the already-decoded neighbors at offsets
/// `(0,1)`, `(1,0)`, then (2D) `(1,1)` snapshotting `hi_mag` before adding `(0,2)`/`(2,0)`, or
/// (H/V) `(0,2)` snapshotting `hi_mag` before adding `(0,3)`/`(0,4)` -- these offsets are always
/// *higher*-frequency scan positions (already visited, since decode goes high-to-low frequency),
/// so causality holds without an explicit "already decoded" check.
///
/// `width_dim`/`height_dim` (only used by the 2D branch's offset-table selection) are the
/// transform's real dimensions, NOT the 32-capped ones the scan uses: a 64x32 transform has a
/// 32x32 coefficient area but still selects the "wide" table (dav1d keys it on the `tx` enum).
/// Real dav1d
/// selects a different 5x5 offset table for square vs `width > height` vs `width < height`
/// transforms (`LO_CTX_OFFSETS_SQUARE`/`_WIDE`/`_TALL`'s doc) -- for a square transform these
/// dims are equal and the square table is always selected, matching this function's pre-rect
/// behavior exactly.
pub fn lo_ctx(
    levels: &LevelBuffer,
    x: u32,
    y: u32,
    is_1d: bool,
    width_dim: u32,
    height_dim: u32,
) -> (u8, u32) {
    let mut mag = levels.get(x, y + 1) + levels.get(x + 1, y);
    let hi_mag;
    let offset = if is_1d {
        mag += levels.get(x, y + 2);
        hi_mag = mag;
        mag += levels.get(x, y + 3) + levels.get(x, y + 4);
        26 + if y > 1 { 10 } else { y * 5 }
    } else {
        mag += levels.get(x + 1, y + 1);
        hi_mag = mag;
        mag += levels.get(x, y + 2) + levels.get(x + 2, y);
        let table = match width_dim.cmp(&height_dim) {
            std::cmp::Ordering::Equal => &LO_CTX_OFFSETS_SQUARE,
            std::cmp::Ordering::Greater => &LO_CTX_OFFSETS_WIDE,
            std::cmp::Ordering::Less => &LO_CTX_OFFSETS_TALL,
        };
        table[y.min(4) as usize][x.min(4) as usize] as u32
    };
    let ctx = offset + if mag > 512 { 4 } else { (mag + 64) >> 7 };
    (ctx as u8, hi_mag)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_level_buffer_defaults_to_zero() {
        let buf = LevelBuffer::new(4, 4);
        assert_eq!(buf.get(0, 0), 0);
        assert_eq!(buf.get(3, 3), 0);
    }

    #[test]
    fn test_level_buffer_out_of_range_reads_zero() {
        let buf = LevelBuffer::new(4, 4);
        assert_eq!(buf.get(10, 10), 0);
    }

    #[test]
    fn test_level_buffer_set_unextended_scales_by_65() {
        let mut buf = LevelBuffer::new(4, 4);
        buf.set(1, 1, false, 2);
        assert_eq!(buf.get(1, 1), 130); // 2 * 65
    }

    #[test]
    fn test_level_buffer_set_extended_offsets_by_192() {
        let mut buf = LevelBuffer::new(4, 4);
        buf.set(1, 1, true, 5);
        assert_eq!(buf.get(1, 1), 197); // 5 + 192
    }

    #[test]
    fn test_level_buffer_rectangular_dims_independent_axes() {
        // width=8, height=4: x can reach 7 (padded to 9 slots), y only 3 (padded to 5 slots).
        let mut buf = LevelBuffer::new(8, 4);
        buf.set(7, 3, false, 2);
        assert_eq!(buf.get(7, 3), 130);
        assert_eq!(buf.get(0, 0), 0);
    }

    #[test]
    fn test_lo_ctx_no_neighbors_is_lowest_bucket() {
        let buf = LevelBuffer::new(8, 8);
        let (ctx, hi_mag) = lo_ctx(&buf, 4, 4, false, 8, 8);
        assert_eq!(hi_mag, 0);
        // offset = LO_CTX_OFFSETS_SQUARE[4][4] = 21, mag bucket = (0+64)>>7 = 0
        assert_eq!(ctx, 21);
    }

    #[test]
    fn test_lo_ctx_2d_neighbors_raise_context_and_hi_mag() {
        let mut buf = LevelBuffer::new(8, 8);
        buf.set(0, 1, false, 3); // offset (0,1) from origin
        buf.set(1, 0, false, 3); // offset (1,0) from origin
        buf.set(1, 1, false, 3); // offset (1,1) from origin
        let (ctx, hi_mag) = lo_ctx(&buf, 0, 0, false, 8, 8);
        assert_eq!(hi_mag, 3 * 65 * 3); // 3 neighbors, each scaled by 65
        assert!(ctx > 0);
    }

    #[test]
    fn test_lo_ctx_is_1d_uses_different_offset_formula() {
        let buf = LevelBuffer::new(16, 16);
        let (ctx_y0, _) = lo_ctx(&buf, 0, 0, true, 16, 16);
        let (ctx_y1, _) = lo_ctx(&buf, 0, 1, true, 16, 16);
        let (ctx_y2, _) = lo_ctx(&buf, 0, 2, true, 16, 16);
        assert_eq!(ctx_y0, 26); // offset 26 + 0*5
        assert_eq!(ctx_y1, 31); // offset 26 + 1*5
        assert_eq!(ctx_y2, 36); // offset 26 + 10 (y > 1)
    }

    #[test]
    fn test_lo_ctx_2d_selects_wide_vs_tall_offset_table() {
        // LO_CTX_OFFSETS_SQUARE[0][1]=1, _WIDE[0][1]=16, _TALL[0][1]=11 -- a position where all
        // three tables disagree, so this proves the right one gets selected per aspect ratio.
        let buf_wide = LevelBuffer::new(16, 8);
        let (ctx_wide, _) = lo_ctx(&buf_wide, 1, 0, false, 16, 8);
        assert_eq!(ctx_wide, 16);
        let buf_tall = LevelBuffer::new(8, 16);
        let (ctx_tall, _) = lo_ctx(&buf_tall, 1, 0, false, 8, 16);
        assert_eq!(ctx_tall, 11);
        let buf_square = LevelBuffer::new(8, 8);
        let (ctx_square, _) = lo_ctx(&buf_square, 1, 0, false, 8, 8);
        assert_eq!(ctx_square, 1);
    }
}
