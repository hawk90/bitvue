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

pub use arithmetic::{update_cdf, ArithmeticDecoder};
pub use cdf::{CdfContext, PartitionCdf};

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

    /// Read skip flag, per AV1 spec Section 5.11.11 / Section 9.3's `SkipCdf` context (`ctx`,
    /// 0..=2 -- see `crate::tile::TileContext::skip_context`).
    ///
    /// Unlike every other `read_*` method in this decoder, this one is real: real per-context
    /// default CDFs (`CdfContext`'s `skip_cdf` doc) and real adaptation via `read_symbol_adaptive`
    /// (spec Section 8.3), not the "representative, context-independent, non-adaptive" bar the
    /// rest of this file still uses -- see `docs/DEVELOPMENT_PHASES.md` Phase 4's AV1
    /// entropy-decoding note for which symbols have and haven't been upgraded yet.
    ///
    /// Returns true if block is skipped (uses prediction only, no residual)
    pub fn read_skip(&mut self, ctx: u8) -> Result<bool> {
        let cdf = self.cdf_context.get_skip_cdf_mut(ctx);
        let symbol = self.decoder.read_symbol_adaptive(cdf)?;
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

    /// Read `compound_mode()` per AV1 spec Section 5.11.24, for compound-prediction inter blocks
    /// (`ref_frame[1] != NONE`, i.e. `read_ref_frames` returned two real reference frames).
    ///
    /// Distinct 8-symbol alphabet from the single-ref `read_inter_mode`'s 4-way one -- each
    /// symbol independently selects an MV-selection strategy (`MvKind`) for L0 and L1 (see
    /// `PredictionMode::l0_mv_kind`/`l1_mv_kind`). Symbol ordering matches libaom's
    /// `compound_mode` enum:
    /// - 0: NEAREST_NEARESTMV
    /// - 1: NEAR_NEARMV
    /// - 2: NEAREST_NEWMV
    /// - 3: NEW_NEARESTMV
    /// - 4: NEAR_NEWMV
    /// - 5: NEW_NEARMV
    /// - 6: GLOBAL_GLOBALMV
    /// - 7: NEW_NEWMV
    ///
    /// Uses a representative (non-adaptive) CDF like every other `read_*` method in this
    /// decoder -- see `CdfContext`'s `compound_mode_cdf` field doc.
    pub fn read_compound_mode(&mut self) -> Result<u8> {
        let cdf = self.cdf_context.get_compound_mode_cdf();
        self.decoder.read_symbol(cdf)
    }

    /// Read `ref_frame()` per AV1 spec Section 5.11.25 (`read_ref_frames`), for inter blocks.
    ///
    /// Returns `[ref_frame0, ref_frame1]` -- `ref_frame1` is `RefFrame::Intra` when this is not a
    /// compound-prediction block, matching this crate's existing "Intra used as the None/single-
    /// ref sentinel" convention (see `CodingUnit::new`'s default `[RefFrame::Intra; 2]`).
    ///
    /// `reference_select` is the frame header flag of the same name (whether compound prediction
    /// is enabled for this frame at all -- see `ParsedFrame::reference_select`'s doc for how it's
    /// sourced). `min_block_dim_px` is `min(cu.width, cu.height)`: compound prediction
    /// additionally requires `Min(bw4, bh4) >= 2` per spec, i.e. at least 8px in the smaller
    /// dimension.
    ///
    /// Uses representative (non-adaptive) CDFs like every other `read_*` method in this decoder
    /// -- see `CdfContext`'s ref-frame CDF fields' doc. Real per-block *mode* reading for
    /// compound blocks (`compound_mode`, a different/larger symbol alphabet than this decoder's
    /// 4-way `read_inter_mode`) and compound motion-vector-difference reading are **not**
    /// implemented -- callers still read a single-ref-shaped mode/MV for compound blocks
    /// afterwards, an existing, unchanged approximation this method doesn't attempt to fix. Only
    /// `ref_frame[0]`/`ref_frame[1]`'s categorical values (and the entropy-decoder bit
    /// consumption needed to reach them correctly) are new.
    pub fn read_ref_frames(
        &mut self,
        reference_select: bool,
        min_block_dim_px: u32,
    ) -> Result<[crate::tile::RefFrame; 2]> {
        use crate::tile::RefFrame;

        let is_compound = if reference_select && min_block_dim_px >= 8 {
            let cdf = self.cdf_context.get_comp_mode_cdf();
            self.decoder.read_symbol(cdf)? == 1
        } else {
            false
        };

        if !is_compound {
            let cdf = self.cdf_context.get_single_ref_p1_cdf();
            let backward = self.decoder.read_symbol(cdf)? == 1;

            let ref0 = if backward {
                let cdf = self.cdf_context.get_single_ref_p2_cdf();
                let is_altref = self.decoder.read_symbol(cdf)? == 1;
                if is_altref {
                    RefFrame::AltRef
                } else {
                    let cdf = self.cdf_context.get_single_ref_p6_cdf();
                    let is_altref2 = self.decoder.read_symbol(cdf)? == 1;
                    if is_altref2 {
                        RefFrame::AltRef2
                    } else {
                        RefFrame::BwdRef
                    }
                }
            } else {
                let cdf = self.cdf_context.get_single_ref_p3_cdf();
                let last3_or_golden = self.decoder.read_symbol(cdf)? == 1;
                if last3_or_golden {
                    let cdf = self.cdf_context.get_single_ref_p5_cdf();
                    let is_golden = self.decoder.read_symbol(cdf)? == 1;
                    if is_golden {
                        RefFrame::Golden
                    } else {
                        RefFrame::Last3
                    }
                } else {
                    let cdf = self.cdf_context.get_single_ref_p4_cdf();
                    let is_last2 = self.decoder.read_symbol(cdf)? == 1;
                    if is_last2 {
                        RefFrame::Last2
                    } else {
                        RefFrame::Last
                    }
                }
            };
            return Ok([ref0, RefFrame::Intra]);
        }

        let cdf = self.cdf_context.get_comp_ref_type_cdf();
        let is_bidir = self.decoder.read_symbol(cdf)? == 1;

        if !is_bidir {
            // Unidirectional compound: both references from the same "direction" group.
            let cdf = self.cdf_context.get_uni_comp_ref_cdf();
            let is_bwdref_altref = self.decoder.read_symbol(cdf)? == 1;
            if is_bwdref_altref {
                return Ok([RefFrame::BwdRef, RefFrame::AltRef]);
            }
            let cdf = self.cdf_context.get_uni_comp_ref_p1_cdf();
            let p1 = self.decoder.read_symbol(cdf)? == 1;
            if !p1 {
                return Ok([RefFrame::Last, RefFrame::Last2]);
            }
            let cdf = self.cdf_context.get_uni_comp_ref_p2_cdf();
            let p2 = self.decoder.read_symbol(cdf)? == 1;
            return Ok([
                RefFrame::Last,
                if p2 {
                    RefFrame::Last3
                } else {
                    RefFrame::Golden
                },
            ]);
        }

        // Bidirectional compound: independent forward + backward group choices.
        let cdf = self.cdf_context.get_comp_ref_cdf();
        let fwd_group_b = self.decoder.read_symbol(cdf)? == 1;
        let ref0 = if !fwd_group_b {
            let cdf = self.cdf_context.get_comp_ref_p1_cdf();
            let is_last2 = self.decoder.read_symbol(cdf)? == 1;
            if is_last2 {
                RefFrame::Last2
            } else {
                RefFrame::Last
            }
        } else {
            let cdf = self.cdf_context.get_comp_ref_p2_cdf();
            let is_golden = self.decoder.read_symbol(cdf)? == 1;
            if is_golden {
                RefFrame::Golden
            } else {
                RefFrame::Last3
            }
        };

        let cdf = self.cdf_context.get_comp_bwdref_cdf();
        let bwd_group_b = self.decoder.read_symbol(cdf)? == 1;
        let ref1 = if !bwd_group_b {
            let cdf = self.cdf_context.get_comp_bwdref_p1_cdf();
            let is_altref2 = self.decoder.read_symbol(cdf)? == 1;
            if is_altref2 {
                RefFrame::AltRef2
            } else {
                RefFrame::BwdRef
            }
        } else {
            RefFrame::AltRef
        };

        Ok([ref0, ref1])
    }

    /// Read `use_intrabc` per AV1 spec Section 5.11.6 -- only call when the frame header's
    /// `allow_intrabc` is true and the current block is on an intra frame.
    pub fn read_use_intrabc(&mut self) -> Result<bool> {
        let cdf = self.cdf_context.get_use_intrabc_cdf();
        Ok(self.decoder.read_symbol(cdf)? == 1)
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

    /// Read one transform block's residual coefficients (AV1 spec Section 5.11.39 `coeffs()`),
    /// returning summary statistics rather than a full per-position coefficient array -- this
    /// crate has no dequantization/inverse-transform/pixel-reconstruction stage, so individual
    /// coefficient positions aren't independently useful, only their aggregate magnitude.
    ///
    /// `tx_size_px` is the transform block's size in pixels per side (4/8/16/32/64).
    ///
    /// # Known simplifications (see `symbol/cdf.rs`'s residual-CDF doc for the CDF side)
    ///
    /// - **No neighbor/level context**: real AV1 derives `txb_skip`/`coeff_base`/`coeff_br`
    ///   context from already-decoded neighbor coefficient levels and the above/left transform
    ///   block state. This uses one fixed representative CDF per symbol kind regardless of
    ///   position or neighbors -- consistent with `skip`/`intra_mode`/`inter_mode` already doing
    ///   the same in this codebase (see `CdfContext::new`'s doc).
    /// - **`eob_extra` bits are uniform literal bits**, not the spec's context-coded first bit --
    ///   the resulting `eob` value only needs to land in the right *range*, not match the spec's
    ///   exact encoding, since nothing here reconstructs pixels from it.
    /// - **Single combined level+sign+golomb pass** per position (descending scan order) instead
    ///   of the spec's two separate passes (all levels, then all signs) -- doesn't change which
    ///   information gets read, only its order, which is irrelevant once the CDFs themselves are
    ///   already non-spec-exact.
    /// - **Golomb extension is a bounded, always-terminating read** (capped at 20 length bits) --
    ///   not necessarily bit-exact against the spec's `read_golomb`, but always produces a real,
    ///   finite level value.
    /// - **No real `tx_size()` bitstream reads**: this crate's `CodingUnit.tx_size` is a
    ///   dimension-based heuristic (`TxSize::from_dimensions`), not read from the bitstream (see
    ///   its own doc) -- residual reading here reuses that same heuristic size, inheriting the
    ///   same pre-existing gap rather than introducing a new one.
    ///
    /// None of these change the *shape* of the read sequence (an `all_zero` check, then --  when
    /// not all-zero -- an `eob_pt` symbol, `eob` extra bits, and exactly `eob` per-position level/
    /// sign/golomb reads) -- which is what matters for keeping the shared arithmetic decoder's
    /// position advancing by a plausible amount instead of not reading residual data at all (see
    /// `crate::tile::coding_unit`'s module doc for why that previously caused real desync/crashes
    /// on real streams).
    pub fn read_residual_block(&mut self, tx_size_px: u32) -> Result<ResidualBlockStats> {
        let txb_skip_cdf = self.cdf_context.get_txb_skip_cdf();
        let all_zero = self.decoder.read_symbol(txb_skip_cdf)? == 1;
        if all_zero {
            return Ok(ResidualBlockStats {
                all_zero: true,
                ..Default::default()
            });
        }

        let eob_pt_cdf = self.cdf_context.get_eob_pt_cdf(tx_size_px);
        let eob_pt = self.decoder.read_symbol(eob_pt_cdf)? as u32 + 1;

        let eob: u32 = if eob_pt <= 2 {
            eob_pt
        } else {
            let num_extra_bits = eob_pt - 2;
            let base = 1u32 << (eob_pt - 2);
            let mut extra = 0u32;
            for _ in 0..num_extra_bits {
                let bit = self.decoder.read_bool(16384)? as u32;
                extra = (extra << 1) | bit;
            }
            base + 1 + extra
        };

        let mut stats = ResidualBlockStats {
            all_zero: false,
            ..Default::default()
        };

        for c in (0..eob).rev() {
            let base_level = if c == eob - 1 {
                let cdf = self.cdf_context.get_coeff_base_eob_cdf();
                self.decoder.read_symbol(cdf)? as u32 + 1
            } else {
                let cdf = self.cdf_context.get_coeff_base_cdf();
                self.decoder.read_symbol(cdf)? as u32
            };

            let mut level = base_level;
            if level > 2 {
                let coeff_br_cdf = self.cdf_context.get_coeff_br_cdf();
                for _ in 0..4 {
                    let br = self.decoder.read_symbol(coeff_br_cdf)? as u32;
                    level += br;
                    if br < 3 {
                        break;
                    }
                }
            }

            if level > 0 {
                if c == 0 {
                    let dc_sign_cdf = self.cdf_context.get_dc_sign_cdf();
                    self.decoder.read_symbol(dc_sign_cdf)?;
                } else {
                    self.decoder.read_bool(16384)?;
                }

                if level > 14 {
                    let mut length = 0u32;
                    loop {
                        length += 1;
                        let terminate = self.decoder.read_bool(16384)?;
                        if terminate || length >= 20 {
                            break;
                        }
                    }
                    let mut extra = 1u32;
                    for _ in 0..length.saturating_sub(1) {
                        let bit = self.decoder.read_bool(16384)? as u32;
                        extra = (extra << 1) | bit;
                    }
                    level = extra + 14;
                }

                stats.nonzero_count += 1;
                stats.sum_abs_level += level as u64;
                stats.max_level = stats.max_level.max(level.min(u16::MAX as u32) as u16);
            }
        }

        Ok(stats)
    }
}

/// Summary statistics for one transform block's residual coefficients -- see
/// `SymbolDecoder::read_residual_block`'s doc for what this deliberately does and doesn't capture
/// (aggregate magnitude, not per-position values or real pixel-domain energy).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ResidualBlockStats {
    /// True if `txb_skip` (all_zero) was signaled -- every other field is 0 in that case.
    pub all_zero: bool,
    /// Number of nonzero coefficient levels read.
    pub nonzero_count: u32,
    /// Sum of absolute coefficient levels (a rough energy proxy).
    pub sum_abs_level: u64,
    /// Largest single coefficient level read.
    pub max_level: u16,
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
