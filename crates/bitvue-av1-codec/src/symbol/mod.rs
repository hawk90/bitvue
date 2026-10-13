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
mod delta;
mod inter;
mod intra;
mod mv;
mod palette;
mod partition;
mod residual;
mod restoration;
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

    /// Exit the decoder (for testing)
    #[allow(dead_code)]
    pub fn exit(&self) -> bool {
        self.decoder.value == 0
    }
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
