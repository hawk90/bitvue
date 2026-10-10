//! AV1 Symbol Decoder
//!
//! Per AV1 Specification Section 8 (Symbol Decoding)
//!
//! This module implements the AV1 entropy decoder:
//! - Arithmetic decoder (range coder)
//! - CDF (Cumulative Distribution Function) tables
//! - Symbol reading functions
//! - Context management
//!
//! ## References
//!
//! - AV1 Spec Section 8.2.2: Arithmetic Decoder
//! - AV1 Spec Section 8.3: Symbol Decoding Functions
//! - AV1 Spec Section 5.11.44: Partition CDF
//!
//! ## Implementation Status
//!
//! **MVP Phase**:
//! - ✅ Basic arithmetic decoder structure
//! - 🚧 CDF tables (partition only)
//! - 🚧 Symbol reading (read_symbol)
//! - ⏳ Context management (simplified)
//!
//! **Full Implementation (Later)**:
//! - ⏳ All CDF tables
//! - ⏳ CDF update/adaptation
//! - ⏳ All symbol reading functions
//! - ⏳ Full context derivation

pub mod arithmetic;
mod block_flags;
pub mod cdf;
mod inter;
mod intra;
mod residual;
pub mod scan;
mod transform;

pub use arithmetic::{update_cdf, ArithmeticDecoder};
pub use cdf::{CdfContext, PartitionCdf};
pub use residual::ResidualBlockStats;
pub use transform::{LumaTxType, TxClass1d};

use bitvue_engine::Result;

/// Symbol decoder state
///
/// Wraps arithmetic decoder and CDF tables.
/// This is the main interface for reading symbols from bitstream.
pub struct SymbolDecoder<'a> {
    /// Arithmetic decoder
    pub decoder: ArithmeticDecoder<'a>,
    /// CDF tables (probability distributions)
    pub cdf_context: CdfContext,
}

impl<'a> SymbolDecoder<'a> {
    /// Create a new symbol decoder, CDF context defaulted to dav1d's qindex bucket 0
    /// (`qcat = 0`). Thin wrapper kept for every pre-existing caller that doesn't care about
    /// real per-frame qindex-bucket selection -- see `new_with_qcat`'s doc.
    pub fn new(data: &'a [u8]) -> Result<Self> {
        Self::new_with_qcat(data, 0)
    }

    /// Create a new symbol decoder with its residual-coefficient CDF family seeded from dav1d's
    /// real per-frame qindex bucket (`qcat`, real formula
    /// `(base_q_idx>20) + (base_q_idx>60) + (base_q_idx>120)`, see `CdfContext::new_with_qcat`'s
    /// doc). Every other CDF family (partition/skip/intra-mode/ref-frame/MV/delta_q, etc.) is
    /// qcat-independent, unaffected by this choice.
    pub fn new_with_qcat(data: &'a [u8], qcat: u8) -> Result<Self> {
        let decoder = ArithmeticDecoder::new(data)?;
        let cdf_context = CdfContext::new_with_qcat(qcat);

        Ok(Self {
            decoder,
            cdf_context,
        })
    }

    /// See `ArithmeticDecoder::padding_is_conformant`: call after the last symbol of a tile.
    pub fn padding_is_conformant(&self) -> bool {
        self.decoder.padding_is_conformant()
    }

    /// Starts from an existing CDF context instead of the defaults: a frame whose
    /// `primary_ref_frame` names a reference begins with that reference's saved CDFs (spec
    /// `load_cdfs`), not the qindex-bucketed defaults.
    pub fn with_cdf_context(data: &'a [u8], cdf_context: CdfContext) -> Result<Self> {
        Ok(Self {
            decoder: ArithmeticDecoder::new(data)?,
            cdf_context,
        })
    }

    /// Read a `partition` symbol (spec 5.11.4), per its real above/left context (`ctx`, 0..=3,
    /// from `crate::tile::TileContext::partition_context`) -- real per-context default CDFs
    /// (`CdfContext`'s `partition_cdfs` doc) and real adaptation via `read_symbol_adaptive`,
    /// matching `read_skip`/`read_intra_mode`'s bar.
    ///
    /// `has_rows`/`has_cols` (frame-edge legality, from `tile::partition::parse_partition_recursive`'s
    /// real `MiRows`/`MiCols` check) select which of spec 5.11.4's four branches this read takes:
    /// - both true: full alphabet, as above (real context + adaptation).
    /// - `has_cols` only: reduced binary `split_or_horz` (HORZ vs SPLIT), via
    ///   `cdf::split_or_horz_prob`'s aggregated probability -- non-adaptive (see that fn's doc for
    ///   why: rav1d's equivalent never writes back to the real `partition_cdfs` entry).
    /// - `has_rows` only: reduced binary `split_or_vert` (VERT vs SPLIT), symmetric.
    /// - neither: implicit `PARTITION_SPLIT`, **no symbol read at all** -- getting this branch
    ///   wrong (e.g. reading anything here) would desync the shared `SymbolDecoder` for the rest
    ///   of the tile, the same bug shape as this session's `residual()`/`ref_frame()`/`mv_joint`
    ///   fixes.
    ///
    /// Returns partition type (0-9) for current block context.
    pub fn read_partition(
        &mut self,
        block_size_log2: u8,
        ctx: u8,
        has_rows: bool,
        has_cols: bool,
    ) -> Result<u8> {
        if has_rows && has_cols {
            let cdf = self.cdf_context.get_partition_cdf_mut(block_size_log2, ctx);
            return self.decoder.read_symbol_adaptive(cdf);
        }
        // Raw partition symbol values (spec order, matching `tile::PartitionType`'s numeric
        // repr): NONE=0, HORZ=1, VERT=2, SPLIT=3. Not importing `PartitionType` itself here to
        // avoid a `symbol -> tile` dependency alongside `tile`'s existing `-> symbol` one.
        const HORZ: u8 = 1;
        const VERT: u8 = 2;
        const SPLIT: u8 = 3;
        if !has_rows && !has_cols {
            return Ok(SPLIT);
        }
        let cdf = self.cdf_context.get_partition_cdf_mut(block_size_log2, ctx);
        if has_cols {
            let psum = cdf::split_or_horz_prob(cdf);
            let bin_cdf = [psum, 0, 0];
            let is_split = self.decoder.read_symbol(&bin_cdf)? == 1;
            Ok(if is_split { SPLIT } else { HORZ })
        } else {
            let psum = cdf::split_or_vert_prob(cdf);
            let bin_cdf = [psum, 0, 0];
            let is_split = self.decoder.read_symbol(&bin_cdf)? == 1;
            Ok(if is_split { SPLIT } else { VERT })
        }
    }

    /// Real `dav1d_msac_decode_bool_equi` -- a single raw equi-probable (50/50) bit, exposed
    /// publicly for callers outside this module that need it directly (palette cache-reuse flags,
    /// V-plane delta sign) rather than through `read_bools_n`'s loop.
    pub fn read_bool_equi(&mut self) -> Result<bool> {
        self.decoder.read_bool(16384)
    }

    /// Real `L(n)`/`dav1d_msac_decode_bools` -- `n` raw equi-probable (50/50) bits packed
    /// MSB-first, matching the existing golomb/literal-extra-bits pattern used throughout
    /// residual reading (`read_bool(16384)` in a loop), factored out here since palette color
    /// values reuse it directly (ported from dav1d's `dav1d_msac_decode_bools`, `src/msac.h`).
    pub fn read_bools_n(&mut self, n: u32) -> Result<u32> {
        let mut v = 0u32;
        for _ in 0..n {
            let bit = self.read_bool_equi()? as u32;
            v = (v << 1) | bit;
        }
        Ok(v)
    }

    /// Real `NS(n)` (spec 8.2.5) over the arithmetic decoder -- a non-power-of-2 uniform integer
    /// read in `0..n`, used for the first pixel of a palette color-index map
    /// (`SymbolDecoder::read_palette_index_map`'s doc). Ported index-for-index from dav1d's
    /// `dav1d_msac_decode_uniform` (`src/msac.h`), not reconstructed from the spec's `NS(n)`
    /// description alone, since the bit-count/threshold math is easy to get subtly wrong.
    pub fn read_uniform(&mut self, n: u32) -> Result<u32> {
        debug_assert!(n > 0);
        let l = 32 - (n.max(1)).leading_zeros(); // floor(log2(n)) + 1, matches dav1d's ulog2(n)+1
        let m = (1u32 << l) - n;
        let v = self.read_bools_n(l - 1)?;
        if v < m {
            Ok(v)
        } else {
            let extra = self.read_bool_equi()? as u32;
            Ok((v << 1) - m + extra)
        }
    }

    /// Read `has_palette_y` (spec 5.11.46) -- real per-`(bsize_ctx, ctx)` CDF + adaptation.
    /// `bsize_ctx`/`ctx`: `crate::tile::coding_unit::read_palette_mode_info`'s doc.
    pub fn read_has_palette_y(&mut self, bsize_ctx: u8, ctx: u8) -> Result<bool> {
        let cdf = self.cdf_context.get_pal_y_cdf_mut(bsize_ctx, ctx);
        Ok(self.decoder.read_symbol_adaptive(cdf)? == 1)
    }

    /// Read `has_palette_uv` -- real per-`ctx` CDF + adaptation (`ctx`: `PaletteSizeY > 0`).
    pub fn read_has_palette_uv(&mut self, ctx: u8) -> Result<bool> {
        let cdf = self.cdf_context.get_pal_uv_cdf_mut(ctx);
        Ok(self.decoder.read_symbol_adaptive(cdf)? == 1)
    }

    /// Read `palette_size_{y,uv}_minus_2` -- real per-`(plane, bsize_ctx)` CDF + adaptation.
    /// `plane`: 0=y/1=uv. Returns the real palette size (`symbol + 2`, range 2..=8).
    pub fn read_pal_size(&mut self, plane: usize, bsize_ctx: u8) -> Result<u8> {
        let cdf = self.cdf_context.get_pal_sz_cdf_mut(plane, bsize_ctx);
        Ok(self.decoder.read_symbol_adaptive(cdf)? + 2)
    }

    /// Read one `color_map` (palette pixel index) symbol -- real per-`(plane, pal_sz, ctx)` CDF +
    /// adaptation (`SymbolDecoder::read_palette_index_map`'s doc for `ctx`'s real derivation).
    /// `plane`: 0=y/1=uv. Alphabet size `pal_sz - 1` (real spec: `pal_sz` symbols, 0..=`pal_sz-1`).
    pub fn read_color_map_index(&mut self, plane: usize, pal_sz: u8, ctx: u8) -> Result<u8> {
        let cdf = self.cdf_context.get_color_map_cdf_mut(plane, pal_sz, ctx);
        self.decoder.read_symbol_adaptive(cdf)
    }

    /// Read `txfm_split` (spec 5.11.18's `read_var_tx_size`) -- real per-`(cat, ctx)` CDF +
    /// adaptation, matching `read_skip`'s bar. `cat`/`ctx` are `crate::tile::coding_unit::read_var_tx_size`'s
    /// packed category and `TileContext::var_tx_context`'s `a+l` sum, respectively. Returns
    /// `true` if this node splits into 4 smaller transform blocks.
    pub fn read_txfm_split(&mut self, cat: u8, ctx: u8) -> Result<bool> {
        let cdf = self.cdf_context.get_txpart_cdf_mut(cat, ctx);
        let symbol = self.decoder.read_symbol_adaptive(cdf)?;
        Ok(symbol == 1)
    }

    /// Read `restore_switchable` (spec 5.11.58): 0=NONE, 1=WIENER, 2=SGRPROJ.
    pub fn read_restore_switchable(&mut self) -> Result<u8> {
        let cdf = self.cdf_context.get_restore_switchable_cdf_mut();
        self.decoder.read_symbol_adaptive(cdf)
    }

    /// Read `use_wiener` (spec 5.11.58) -- whether a restoration unit of a WIENER frame filters.
    pub fn read_restore_wiener(&mut self) -> Result<bool> {
        let cdf = self.cdf_context.get_restore_wiener_cdf_mut();
        Ok(self.decoder.read_symbol_adaptive(cdf)? == 1)
    }

    /// Read `use_sgrproj` (spec 5.11.58) -- whether a restoration unit of a SGRPROJ frame filters.
    pub fn read_restore_sgrproj(&mut self) -> Result<bool> {
        let cdf = self.cdf_context.get_restore_sgrproj_cdf_mut();
        Ok(self.decoder.read_symbol_adaptive(cdf)? == 1)
    }

    /// Read `use_intrabc` per AV1 spec Section 5.11.6 -- only call when the frame header's
    /// `allow_intrabc` is true and the current block is on an intra frame.
    pub fn read_use_intrabc(&mut self) -> Result<bool> {
        let cdf = self.cdf_context.get_use_intrabc_cdf_mut();
        Ok(self.decoder.read_symbol_adaptive(cdf)? == 1)
    }

    /// Read `mv_joint` (AV1 spec Section 5.11.32 `read_mv()`) -- gates which axis, if any, has a
    /// coded component to read next. Returns one of the `MV_JOINT_*` values (see
    /// `CdfContext::get_mv_joint_cdf_mut`'s doc): 0=both zero, 1=horizontal only, 2=vertical
    /// only, 3=both. Real adaptation via `read_symbol_adaptive` -- MV CDFs have no above/left
    /// neighbor context in the real spec (unlike `skip`/`kfym`), just frame-lifetime adaptation.
    pub fn read_mv_joint(&mut self) -> Result<u8> {
        let cdf = self.cdf_context.get_mv_joint_cdf_mut();
        self.decoder.read_symbol_adaptive(cdf)
    }

    /// Read one MV component's difference (dav1d `read_mv_component_diff`, `src/decode.c`), in
    /// 1/8-sample units, sign applied. `component`: `0` = vertical, `1` = horizontal. `mv_prec`:
    /// `-1` when the frame uses integer MVs (no fractional bits are coded), `0` for quarter-sample
    /// precision (the high-precision bit is not coded and reads as 1), `1` for eighth-sample.
    pub fn read_mv_component_diff(&mut self, component: usize, mv_prec: i8) -> Result<i32> {
        let cdfs = self.cdf_context.get_mv_component_cdfs_mut(component);
        let sign = self.decoder.read_symbol_adaptive(&mut cdfs.sign)? == 1;
        let class = self.decoder.read_symbol_adaptive(&mut cdfs.classes)? as usize;
        let (mut fp, mut hp) = (3i32, 1i32);
        let up: i32;
        if class == 0 {
            up = self.decoder.read_symbol_adaptive(&mut cdfs.class0)? as i32;
            if mv_prec >= 0 {
                fp = self
                    .decoder
                    .read_symbol_adaptive(&mut cdfs.class0_fp[up as usize])?
                    as i32;
                if mv_prec > 0 {
                    hp = self.decoder.read_symbol_adaptive(&mut cdfs.class0_hp)? as i32;
                }
            }
        } else {
            let mut bits = 1i32 << class;
            for n in 0..class {
                bits |= (self.decoder.read_symbol_adaptive(&mut cdfs.class_n[n])? as i32) << n;
            }
            up = bits;
            if mv_prec >= 0 {
                fp = self.decoder.read_symbol_adaptive(&mut cdfs.class_n_fp)? as i32;
                if mv_prec > 0 {
                    hp = self.decoder.read_symbol_adaptive(&mut cdfs.class_n_hp)? as i32;
                }
            }
        }
        let diff = ((up << 3) | (fp << 1) | hp) + 1;
        Ok(if sign { -diff } else { diff })
    }

    /// Read delta Q (quantization parameter delta)
    ///
    /// Per AV1 Spec Section 5.11.38 (Quantization Parameter Delta)
    ///
    /// Returns the delta Q value (can be positive or negative).
    /// Range: -MAX_DELTA_Q to +MAX_DELTA_Q where MAX_DELTA_Q = 63
    ///
    /// # Process
    /// 1. Read delta_q_abs using a variable-length code
    /// 2. If delta_q_abs > 0, read delta_q_sign_bit
    /// 3. Apply sign to get final delta Q value
    ///
    /// # Example
    /// ```ignore
    /// let delta_q = decoder.read_delta_q()?;
    /// // delta_q could be: 0, +1, -1, +2, -2, ..., +63, -63
    /// ```
    /// Real `read_delta_qindex()` (spec 5.11.38), ported exactly from dav1d's `decode_b`
    /// delta-q block (`src/decode.c`), not the spec pseudocode's abstraction alone: a 4-outcome
    /// adaptive symbol (`0..=2` used directly as the magnitude, `3` triggers a golomb-style
    /// variable-length extension -- a real 3-bit `n_bits` selector followed by `n_bits` raw
    /// equi-probable bits, NOT another adaptive symbol read), then -- only if the magnitude is
    /// nonzero -- a single real equi-probable (50/50) sign bit (`dav1d_msac_decode_bool_equi`,
    /// NOT an adaptive CDF, unlike `mv_sign`/`cfl_alpha`'s per-component signs). This crate's
    /// previous implementation substituted a hand-picked single-symbol CDF for the golomb
    /// extension and an adaptive CDF for the sign -- a real desync bug for any block whose delta
    /// magnitude reached the extension (common for real content with meaningful QP variation).
    pub fn read_delta_q(&mut self) -> Result<i16> {
        let cdf = self.cdf_context.get_delta_q_cdf_mut();
        let base = self.decoder.read_symbol_adaptive(cdf)?;
        let abs: i32 = if base < 3 {
            base as i32
        } else {
            let n_bits = 1 + self.read_bools_n(3)?;
            self.read_bools_n(n_bits)? as i32 + 1 + (1i32 << n_bits)
        };
        if abs == 0 {
            return Ok(0);
        }
        let negative = self.read_bool_equi()?;
        let delta_q = if negative { -abs } else { abs } as i16;
        tracing::debug!("Delta Q: {}", delta_q);
        Ok(delta_q)
    }

    /// Real `read_delta_lf()` (spec 5.11.38) -- same real golomb-extension + equi-probable-sign
    /// shape as `read_delta_q`'s doc, over the separate `delta_lf` CDF family. `cdf_index`:
    /// `0` for the single-component (`!delta_lf_multi`) case, `1..=4` for `delta_lf_multi`'s
    /// per-plane components (`crate::tile::coding_unit::parse_coding_unit`'s doc for the real
    /// `n_lfs`/index derivation).
    pub fn read_delta_lf(&mut self, cdf_index: usize) -> Result<i16> {
        let cdf = self.cdf_context.get_delta_lf_cdf_mut(cdf_index);
        let base = self.decoder.read_symbol_adaptive(cdf)?;
        let abs: i32 = if base < 3 {
            base as i32
        } else {
            let n_bits = 1 + self.read_bools_n(3)?;
            self.read_bools_n(n_bits)? as i32 + 1 + (1i32 << n_bits)
        };
        if abs == 0 {
            return Ok(0);
        }
        let negative = self.read_bool_equi()?;
        Ok(if negative { -abs } else { abs } as i16)
    }

    /// Exit the decoder (for testing)
    #[allow(dead_code)]
    pub fn exit(&self) -> bool {
        self.decoder.value == 0
    }
}

/// Spec `txSzCtx` (dav1d `TxfmInfo.ctx`) of a transform: `(Tx_Size_Sqr + Tx_Size_Sqr_Up + 1) >> 1`,
/// i.e. the average of the smaller and the larger side's size class (log2 of px / 4), rounded up.
/// It selects the coefficient CDF family. Equal to the larger side's class for square and 2:1
/// transforms and one lower for 4:1 ones.
pub(crate) fn tx_size_ctx(tx_width_px: u32, tx_height_px: u32) -> usize {
    let a = cdf::tx_size_class(tx_width_px);
    let b = cdf::tx_size_class(tx_height_px);
    (a.min(b) + a.max(b) + 1) >> 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_symbol_decoder_creation() {
        let data = vec![0x80, 0x00, 0x00]; // Initial value 0x8000 (big-endian)
        let result = SymbolDecoder::new(&data);
        assert!(result.is_ok());
    }

    #[test]
    fn test_symbol_decoder_read_partition() {
        // Create decoder with some data
        let data = vec![0x80, 0x00, 0x00, 0x00];
        let mut decoder = SymbolDecoder::new(&data).unwrap();

        // Read partition symbol
        // Note: This will likely fail without real entropy-coded data
        // This is just a structural test
        let _result = decoder.read_partition(6, 0, true, true); // 64x64 block (2^6)
    }

    // `read_transform_type_is_1d` tests. Each early-return branch (`coded_lossless`/
    // `qidx_is_zero`/large-tx) must consume zero bits -- verified by comparing the raw decoder
    // state (`range`/`value`/`cnt`) before and after, since any real symbol read changes it.
    pub(super) fn decoder_state(d: &SymbolDecoder) -> (u32, usize, i32) {
        (d.decoder.range, d.decoder.value, d.decoder.cnt)
    }

    #[test]
    fn test_read_uniform_n_one_reads_zero_bits_and_returns_zero() {
        let data = vec![0x80, 0x00, 0xFF, 0xFF, 0xAA, 0xBB];
        let mut decoder = SymbolDecoder::new(&data).unwrap();
        let before = decoder_state(&decoder);
        let v = decoder.read_uniform(1).unwrap();
        assert_eq!(v, 0);
        assert_eq!(decoder_state(&decoder), before);
    }

    #[test]
    fn test_read_uniform_all_results_in_range() {
        // Property-style check across several synthetic byte streams and `n` values: every
        // decoded value must land in `0..n` (matches this session's established
        // `test_coeff_position_all_positions_unique_and_in_range` discipline).
        for seed in 0u8..8 {
            for n in [2u32, 3, 5, 7, 8, 33] {
                let data = vec![0x40 ^ seed, seed, 0xFF, 0xFF, 0xAA ^ seed, 0xBB, 0x12, 0x34];
                let mut decoder = SymbolDecoder::new(&data).unwrap();
                let v = decoder.read_uniform(n).unwrap();
                assert!(v < n, "read_uniform({n}) returned {v}, out of range");
            }
        }
    }

    #[test]
    fn test_read_bools_n_zero_reads_zero_bits() {
        let data = vec![0x80, 0x00, 0xFF, 0xFF, 0xAA, 0xBB];
        let mut decoder = SymbolDecoder::new(&data).unwrap();
        let before = decoder_state(&decoder);
        let v = decoder.read_bools_n(0).unwrap();
        assert_eq!(v, 0);
        assert_eq!(decoder_state(&decoder), before);
    }

    /// dav1d's default for `use_intrabc` is `CDF1(30531)` (it was a hand-picked 0.97 before), and
    /// the flag adapts like every other symbol -- it used to be read without updating the CDF.
    #[test]
    fn use_intrabc_starts_at_dav1ds_default_and_adapts() {
        let data = [0x55u8; 16];
        let mut dec = SymbolDecoder::new(&data).unwrap();
        assert_eq!(
            dec.cdf_context.get_use_intrabc_cdf_mut(),
            &[32768 - 30531, 0, 0]
        );
        dec.read_use_intrabc().unwrap();
        let cdf = dec.cdf_context.get_use_intrabc_cdf_mut();
        assert_eq!(cdf[2], 1, "adaptation count after one read");
        assert_ne!(cdf[0], 32768 - 30531, "probability moved");
    }
}
