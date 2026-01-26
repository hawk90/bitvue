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
pub mod cdf;

pub use arithmetic::ArithmeticDecoder;
pub use cdf::{CdfContext, PartitionCdf};

use bitvue_core::Result;

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
    /// Create a new symbol decoder
    pub fn new(data: &'a [u8]) -> Result<Self> {
        let decoder = ArithmeticDecoder::new(data)?;
        let cdf_context = CdfContext::new();

        Ok(Self {
            decoder,
            cdf_context,
        })
    }

    /// Read a partition symbol
    ///
    /// Returns partition type (0-9) for current block context.
    /// Context depends on block size and neighboring partitions.
    pub fn read_partition(
        &mut self,
        block_size_log2: u8,
        _has_rows: bool,
        _has_cols: bool,
    ) -> Result<u8> {
        // Get CDF for this block size
        let cdf = self.cdf_context.get_partition_cdf(block_size_log2);

        // Read symbol using CDF
        self.decoder.read_symbol(cdf)
    }

    /// Read skip flag
    ///
    /// Returns true if block is skipped (uses prediction only, no residual)
    pub fn read_skip(&mut self) -> Result<bool> {
        let cdf = self.cdf_context.get_skip_cdf();
        let symbol = self.decoder.read_symbol(cdf)?;
        Ok(symbol == 1)
    }

    /// Read INTRA prediction mode
    ///
    /// Returns INTRA mode symbol (0-12):
    /// - 0: DC_PRED
    /// - 1: V_PRED
    /// - 2: H_PRED
    /// - 3-12: Directional and smooth modes
    pub fn read_intra_mode(&mut self) -> Result<u8> {
        let cdf = self.cdf_context.get_intra_mode_cdf();
        self.decoder.read_symbol(cdf)
    }

    /// Read INTER prediction mode
    ///
    /// Returns INTER mode symbol (0-3):
    /// - 0: NEWMV (read explicit MV)
    /// - 1: NEARESTMV (use nearest neighbor MV)
    /// - 2: NEARMV (use near neighbor MV)
    /// - 3: GLOBALMV (use global motion MV)
    pub fn read_inter_mode(&mut self) -> Result<u8> {
        let cdf = self.cdf_context.get_inter_mode_cdf();
        self.decoder.read_symbol(cdf)
    }

    /// Read motion vector component (horizontal or vertical)
    ///
    /// Per AV1 Spec Section 5.11.47 (Motion Vector Component)
    ///
    /// Returns MV component in quarter-pel units (divide by 4 for pixel units)
    pub fn read_mv_component(&mut self) -> Result<i32> {
        // Read MV class (magnitude range)
        let mv_class_cdf = self.cdf_context.get_mv_class_cdf();

        tracing::trace!(
            "  Before read_mv_class: decoder.value={:#06x}, decoder.range={:#06x}",
            self.decoder.value,
            self.decoder.range
        );

        let mv_class = self.decoder.read_symbol(mv_class_cdf)?;

        tracing::trace!(
            "  After read_mv_class: decoder.value={:#06x}, decoder.range={:#06x}",
            self.decoder.value,
            self.decoder.range
        );
        tracing::debug!(
            "  MV class={} (0=zero, 1..11=increasing magnitude ranges)",
            mv_class
        );

        // Calculate magnitude based on class
        let magnitude = if mv_class == 0 {
            // Class 0: magnitude = 0
            tracing::trace!("    Class 0 → magnitude = 0");
            0
        } else {
            // Base magnitude for this class
            let base = match mv_class {
                1 => 1,
                2 => 2,
                3 => 4,
                4 => 8,
                5 => 16,
                6 => 32,
                7 => 64,
                8 => 128,
                9 => 256,
                10 => 512,
                11 => 1024,
                _ => 0,
            };

            // Number of additional bits to read
            let num_bits = if mv_class == 1 { 0 } else { mv_class - 1 };

            tracing::trace!(
                "    Class {} → base={}, reading {} additional bits",
                mv_class,
                base,
                num_bits
            );

            // Read additional bits
            let mut mag = base;
            let mv_bit_cdf = self.cdf_context.get_mv_bit_cdf();
            for i in 0..num_bits {
                let bit = self.decoder.read_symbol(mv_bit_cdf)?;
                mag = (mag << 1) | bit as i32;
                tracing::trace!("      bit[{}] = {} → mag = {}", i, bit, mag);
            }

            tracing::trace!("    Final magnitude = {}", mag);
            mag
        };

        // Read sign (0 = positive, 1 = negative)
        let sign = if magnitude > 0 {
            let mv_sign_cdf = self.cdf_context.get_mv_sign_cdf();
            tracing::trace!(
                "  Before read_sign: decoder.value={:#06x}, decoder.range={:#06x}",
                self.decoder.value,
                self.decoder.range
            );
            let s = self.decoder.read_symbol(mv_sign_cdf)?;
            tracing::trace!(
                "  After read_sign: decoder.value={:#06x}, decoder.range={:#06x}, sign={}",
                self.decoder.value,
                self.decoder.range,
                s
            );
            s
        } else {
            tracing::trace!("  Magnitude is 0, no sign bit");
            0
        };

        // Apply sign
        let signed_mag = if sign == 1 { -magnitude } else { magnitude };

        tracing::debug!(
            "  Signed magnitude = {} (magnitude={}, sign={})",
            signed_mag,
            magnitude,
            sign
        );

        // Read fractional bits (AV1 spec Section 7.9.3)
        // mv_fr: half-pel bit (0 or 2 qpel)
        let mv_bit_cdf = self.cdf_context.get_mv_bit_cdf();

        tracing::trace!(
            "  Before read_fr: decoder.value={:#06x}, decoder.range={:#06x}",
            self.decoder.value,
            self.decoder.range
        );
        let fr = self.decoder.read_symbol(mv_bit_cdf)? as i32;
        tracing::trace!(
            "  After read_fr: decoder.value={:#06x}, decoder.range={:#06x}, fr={}",
            self.decoder.value,
            self.decoder.range,
            fr
        );

        // mv_hp: quarter-pel bit (0 or 1 qpel)
        // For MVP, always read hp bit (assume allow_high_precision_mv = true)
        tracing::trace!(
            "  Before read_hp: decoder.value={:#06x}, decoder.range={:#06x}",
            self.decoder.value,
            self.decoder.range
        );
        let hp = self.decoder.read_symbol(mv_bit_cdf)? as i32;
        tracing::trace!(
            "  After read_hp: decoder.value={:#06x}, decoder.range={:#06x}, hp={}",
            self.decoder.value,
            self.decoder.range,
            hp
        );

        // Combine: MV = (magnitude << 2) | (fr << 1) | hp
        // This gives quarter-pel precision (0, 1, 2, 3 qpel)
        let qpel_offset = (fr << 1) | hp;
        let mv_qpel = (signed_mag * 4)
            + if signed_mag < 0 {
                -qpel_offset
            } else {
                qpel_offset
            };

        tracing::debug!(
            "  MV breakdown: signed_mag={}, fr={}, hp={}, qpel_offset={} → {} qpel",
            signed_mag,
            fr,
            hp,
            qpel_offset,
            mv_qpel
        );

        Ok(mv_qpel)
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
    pub fn read_delta_q(&mut self) -> Result<i16> {
        // First, read delta_q_abs (absolute value of delta Q)
        let abs = self.read_delta_q_abs()?;

        // If abs is 0, delta_q is 0 (no sign bit needed)
        if abs == 0 {
            tracing::trace!("Delta Q: 0");
            return Ok(0);
        }

        // Read sign bit (0 = positive, 1 = negative)
        let sign_cdf = self.cdf_context.get_delta_q_sign_cdf();
        let sign = self.decoder.read_symbol(sign_cdf)?;

        let delta_q = if sign == 1 { -abs } else { abs };

        tracing::debug!("Delta Q: {} (abs={}, sign={})", delta_q, abs, sign);
        Ok(delta_q)
    }

    /// Read delta_q_abs (absolute value of delta Q)
    ///
    /// Per AV1 Spec Section 5.11.38:
    /// - delta_q_abs is encoded using a variable-length code
    /// - Small values (0-3) are encoded directly
    /// - Values >= 4 use a diff-based encoding
    ///
    /// Returns absolute value in range 0..=63
    fn read_delta_q_abs(&mut self) -> Result<i16> {
        let delta_q_cdf = self.cdf_context.get_delta_q_cdf();

        // Read the base value (0-3, or 4+)
        let base = self.decoder.read_symbol(delta_q_cdf)?;

        let abs = if base <= 3 {
            // Small value: use directly
            base as i16
        } else {
            // Large value (4+): use diff-based encoding
            // Read additional diff value
            let diff_cdf = self.cdf_context.get_diff_cdf();
            let diff = self.decoder.read_symbol(diff_cdf)? as i16;

            // Calculate: abs = 4 + diff
            let result = 4 + diff;

            // Clamp to MAX_DELTA_Q (63 per AV1 spec)
            result.min(63)
        };

        tracing::trace!("Delta Q abs: {}", abs);
        Ok(abs)
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
        let _result = decoder.read_partition(6, true, true); // 64x64 block (2^6)
    }
}
