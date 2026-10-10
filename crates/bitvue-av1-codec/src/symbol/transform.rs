//! Transform size (spec 5.11.15/16) and transform type (spec 5.11.47) reading, plus the
//! luma/chroma transform-class types the residual reader consumes.

use super::*;

impl<'a> SymbolDecoder<'a> {
    /// Read `tx_size()` (AV1 spec Section 5.11.15/16 `read_tx_size`) for an **intra** coding
    /// block on a `TxfmMode::Switchable` frame -- inter blocks instead use a recursive
    /// `read_var_tx_size()` (spec 5.11.17/18) this crate doesn't implement yet (a coding block
    /// can hold a *mix* of transform sizes there, which `CodingUnit`'s single `tx_size: TxSize`
    /// field can't represent -- see `docs/DEVELOPMENT_PHASES.md`'s AV1 entropy-decoding note).
    ///
    /// `max_tx_class` is the largest transform size that fits the coding block, as a `TxSize`
    /// discriminant (0..=4) -- callers source this from `TxSize::from_dimensions`, which already
    /// matches rav1d's `DAV1D_MAX_TXFM_SIZE_FOR_BS` table for the square coding blocks this crate
    /// models (see this method's caller for the non-square caveat this doesn't need to handle).
    /// Reads nothing and returns `0` (4x4) immediately if `max_tx_class == 0` -- spec: no
    /// `tx_size()` symbol exists once the block is already as small as it can get.
    ///
    /// Real per-context CDF (`CdfContext`'s `txsz_cdf` doc) + real adaptation, matching
    /// `read_skip`/`read_ref_frames`'s bar. `ctx` (0..=2) is
    /// `crate::tile::TileContext::tx_size_context`'s result. Returns the resolved `TxSize`
    /// discriminant (0..=4): the depth symbol (0..=`min(max_tx_class,2)`) is subtracted from
    /// `max_tx_class`, walking the same `Tx64x64→Tx32x32→Tx16x16→Tx8x8→Tx4x4` chain rav1d's
    /// `TxfmInfo.sub` does (this crate's `TxSize` enum happens to already be ordered that way).
    ///
    /// Callers must handle the *other* `TxMode`s themselves before ever reaching this method:
    /// `Only4x4` (unconditionally 4x4) and `Largest` (unconditionally `max_tx_class`, i.e. no
    /// depth reduction) both read zero bits -- this method is only for `Switchable`. Real spec
    /// also forces 4x4 when `CodedLossless`, regardless of `TxMode` -- callers must check that
    /// first too (this method has no way to know it).
    pub fn read_tx_size(&mut self, max_tx_class: u8, ctx: u8) -> Result<u8> {
        if max_tx_class == 0 {
            return Ok(0);
        }
        let cdf = self
            .cdf_context
            .get_txsz_cdf_mut(max_tx_class as usize, ctx);
        let depth = self.decoder.read_symbol_adaptive(cdf)?;
        Ok(max_tx_class.saturating_sub(depth))
    }

    /// Read `transform_type()` (AV1 spec Section 5.11.47), returning only `is_1d` (whether the
    /// resulting `TxClass` is `TX_CLASS_H`/`TX_CLASS_V`, as opposed to `TX_CLASS_2D`) rather than
    /// the full `TxType` -- that's all `eob_bin`'s context axis needs (see
    /// `read_residual_block`'s doc), and this crate has no reconstruction stage that would need
    /// the exact transform kernel. Must still consume the *real* number of bits regardless of
    /// which branch is taken, matching rav1d's `read_coefs` (`memorysafety/rav1d`, BSD-2-Clause,
    /// `src/recon_tmpl.c` -- not yet ported to that project's Rust) exactly, since skipping this
    /// read entirely (as this crate did before) desyncs every later symbol in the tile whenever
    /// the real encoder actually wrote transform-type bits (the common case: `coded_lossless`
    /// false, tx not the largest, `qidx != 0`).
    ///
    /// `is_intra`: whether this is an intra-predicted block (compound/inter blocks are never
    /// "intra" for this purpose). `coded_lossless`/`reduced_tx_set`: frame header flags (see
    /// `ParsedFrame`'s doc). `qidx_is_zero`: the frame's `base_q_idx == 0` (a real spec shortcut
    /// distinct from `coded_lossless`, which additionally requires zero delta-Q).
    /// `tx_width_px`/`tx_height_px`: transform block size in pixels. `y_mode_raw`: the intra prediction mode symbol
    /// (0..=12, only meaningful when `is_intra`) -- spec's `FILTER_PRED` substitution never
    /// applies since this crate's `PredictionMode` has no such variant.
    ///
    /// Five CDF families cover the real decision tree (`CdfContext`'s `txtp_*_cdf` doc); which
    /// tx-size classes reach which family is a direct consequence of the branch conditions below,
    /// not arbitrary -- e.g. `txtp_intra1`/`txtp_inter1` only ever see tx classes 0..=1 (4x4/8x8)
    /// because every larger size is intercepted by an earlier branch first.
    pub fn read_transform_type(
        &mut self,
        is_intra: bool,
        coded_lossless: bool,
        qidx_is_zero: bool,
        reduced_tx_set: bool,
        tx_width_px: u32,
        tx_height_px: u32,
        y_mode_raw: u8,
    ) -> Result<LumaTxType> {
        // dav1d's `t_dim->max` / `t_dim->min`: size classes (log2 of px / 4) of the larger and the
        // smaller transform dimension. Which one applies depends on the decision (spec 5.11.47's
        // `get_tx_set` uses the square-up of the larger side for "too big" and the square of the
        // smaller side for the 16x16 test and for indexing the CDF).
        let max_class = cdf::tx_size_class(tx_width_px.max(tx_height_px));
        let min_class = cdf::tx_size_class(tx_width_px.min(tx_height_px));
        // `t_dim->max + intra >= TX_64X64`: intra additionally forces DCT_DCT (2D, no bits) one
        // tx-size class earlier than inter (at 32x32, not just 64x64) -- real spec asymmetry, not
        // a simplification.
        if coded_lossless || qidx_is_zero || max_class + usize::from(is_intra) >= 4 {
            return Ok(LumaTxType::DCT);
        }
        if is_intra {
            if reduced_tx_set || min_class == 2 {
                // Intra2 alphabet (IDTX/DCT_DCT/ADST_ADST/ADST_DCT/DCT_ADST) is entirely
                // TX_CLASS_2D -- no symbol value here can ever produce a 1D class.
                let cdf = self
                    .cdf_context
                    .get_txtp_intra2_cdf_mut(min_class, y_mode_raw);
                self.decoder.read_symbol_adaptive(cdf)?;
                Ok(LumaTxType::DCT)
            } else {
                // Intra1 alphabet: IDTX, DCT_DCT, V_DCT, H_DCT, ADST_ADST, ADST_DCT, DCT_ADST.
                let cdf = self
                    .cdf_context
                    .get_txtp_intra1_cdf_mut(min_class, y_mode_raw);
                let idx = self.decoder.read_symbol_adaptive(cdf)?;
                Ok(LumaTxType::of_class(match idx {
                    2 => TxClass1d::Vertical,
                    3 => TxClass1d::Horizontal,
                    _ => TxClass1d::TwoD,
                }))
            }
        } else if reduced_tx_set || max_class == 3 {
            // Inter3 alphabet is a single bit choosing between IDTX and DCT_DCT -- both 2D.
            let cdf = self.cdf_context.get_txtp_inter3_cdf_mut(min_class);
            self.decoder.read_symbol_adaptive(cdf)?;
            Ok(LumaTxType::DCT)
        } else if min_class == 2 {
            // Inter2 alphabet: IDTX, V_DCT, H_DCT, DCT_DCT, ADST_DCT, DCT_ADST, FLIPADST_DCT,
            // DCT_FLIPADST, ADST_ADST, FLIPADST_FLIPADST, ADST_FLIPADST, FLIPADST_ADST.
            let cdf = self.cdf_context.get_txtp_inter2_cdf_mut();
            let idx = self.decoder.read_symbol_adaptive(cdf)?;
            Ok(LumaTxType::of_class(match idx {
                1 => TxClass1d::Vertical,
                2 => TxClass1d::Horizontal,
                _ => TxClass1d::TwoD,
            }))
        } else {
            // Inter1 alphabet: IDTX, V_DCT, H_DCT, V_ADST, H_ADST, V_FLIPADST, H_FLIPADST,
            // DCT_DCT, ADST_DCT, DCT_ADST, FLIPADST_DCT, DCT_FLIPADST, ADST_ADST,
            // FLIPADST_FLIPADST, ADST_FLIPADST, FLIPADST_ADST -- V_* at odd idx (1,3,5), H_* at
            // even idx (2,4,6), everything else (0, 7..=15) 2D.
            let cdf = self.cdf_context.get_txtp_inter1_cdf_mut(min_class);
            let idx = self.decoder.read_symbol_adaptive(cdf)?;
            Ok(LumaTxType {
                class: match idx {
                    1 | 3 | 5 => TxClass1d::Vertical,
                    2 | 4 | 6 => TxClass1d::Horizontal,
                    _ => TxClass1d::TwoD,
                },
                // V_ADST/H_ADST/V_FLIPADST/H_FLIPADST (indices 3..=6): the 1D types a 16-wide
                // chroma transform cannot inherit.
                adst_1d: (3..=6).contains(&idx),
            })
        }
    }

    /// [`Self::read_transform_type`], keeping only the transform class.
    #[allow(clippy::too_many_arguments)]
    pub fn read_transform_type_is_1d(
        &mut self,
        is_intra: bool,
        coded_lossless: bool,
        qidx_is_zero: bool,
        reduced_tx_set: bool,
        tx_width_px: u32,
        tx_height_px: u32,
        y_mode_raw: u8,
    ) -> Result<TxClass1d> {
        Ok(self
            .read_transform_type(
                is_intra,
                coded_lossless,
                qidx_is_zero,
                reduced_tx_set,
                tx_width_px,
                tx_height_px,
                y_mode_raw,
            )?
            .class)
    }
}

/// What the chroma transform type derivation needs to know about a luma transform type: its class
/// and whether it is one of the 1D ADST/flip-ADST types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LumaTxType {
    pub class: TxClass1d,
    pub adst_1d: bool,
}

impl LumaTxType {
    /// `DCT_DCT` -- what an all-zero or uncoded luma block counts as.
    pub const DCT: Self = Self {
        class: TxClass1d::TwoD,
        adst_1d: false,
    };

    fn of_class(class: TxClass1d) -> Self {
        Self {
            class,
            adst_1d: false,
        }
    }

    /// The class of the transform an inter block's chroma inherits (dav1d `get_uv_inter_txtp`):
    /// a 32-point chroma transform keeps only IDTX (2D either way); a chroma transform with a
    /// 16-point short side drops the ADST/flip 1D types to `DCT_DCT`.
    pub fn chroma_class(self, uv_width_px: u32, uv_height_px: u32) -> TxClass1d {
        if uv_width_px.max(uv_height_px) >= 32 {
            return TxClass1d::TwoD;
        }
        if uv_width_px.min(uv_height_px) == 16 && self.adst_1d {
            return TxClass1d::TwoD;
        }
        self.class
    }
}

/// Real transform class for one transform block -- `TwoD` (default scan table), or `Horizontal` /
/// `Vertical` (closed-form 1D position, spec `TX_CLASS_H`/`TX_CLASS_V`). Previously collapsed to a
/// single `is_1d: bool` (both `V_DCT`/`H_DCT`-family types treated identically) since this crate's
/// square-transform-only scope made the two indistinguishable in practice (see
/// `scan::coeff_position`'s doc) -- kept distinct now that rectangular var-tx can produce
/// transforms where `H` and `V` derive genuinely different coefficient positions (`H` uses the
/// transform's height, `V` its width; verified against rav1d's `DECODE_COEFS_CLASS` macro,
/// `src/recon_tmpl.c`, not assumed).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TxClass1d {
    TwoD,
    Horizontal,
    Vertical,
}

impl TxClass1d {
    /// `true` for `Horizontal`/`Vertical` -- the CDF-selection axis (`eob_bin`'s `is_1d` param)
    /// doesn't need the H/V distinction, only position derivation does (this type's doc).
    pub fn is_1d(self) -> bool {
        self != TxClass1d::TwoD
    }

    /// `true` for `Vertical` only -- meaningless (never read) when `!self.is_1d()`.
    pub fn is_vertical(self) -> bool {
        self == TxClass1d::Vertical
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::decoder_state;
    use super::*;

    #[test]
    fn test_transform_type_coded_lossless_reads_zero_bits() {
        let data = vec![0x80, 0x00, 0xFF, 0xFF, 0xAA, 0xBB];
        let mut decoder = SymbolDecoder::new(&data).unwrap();
        let before = decoder_state(&decoder);
        let is_1d = decoder
            .read_transform_type_is_1d(true, true, false, false, 16, 16, 0)
            .unwrap();
        assert!(!is_1d.is_1d());
        assert_eq!(decoder_state(&decoder), before);
    }

    #[test]
    fn test_transform_type_qidx_zero_reads_zero_bits() {
        let data = vec![0x80, 0x00, 0xFF, 0xFF, 0xAA, 0xBB];
        let mut decoder = SymbolDecoder::new(&data).unwrap();
        let before = decoder_state(&decoder);
        let is_1d = decoder
            .read_transform_type_is_1d(false, false, true, false, 16, 16, 0)
            .unwrap();
        assert!(!is_1d.is_1d());
        assert_eq!(decoder_state(&decoder), before);
    }

    #[test]
    fn test_transform_type_large_tx_reads_zero_bits() {
        let data = vec![0x80, 0x00, 0xFF, 0xFF, 0xAA, 0xBB];
        let mut decoder = SymbolDecoder::new(&data).unwrap();
        let before = decoder_state(&decoder);
        // 64x64 (tx_size_px=64 -> tx_size_class=4), inter (is_intra=false): 4+0>=4 -> large tx.
        let is_1d = decoder
            .read_transform_type_is_1d(false, false, false, false, 64, 64, 0)
            .unwrap();
        assert!(!is_1d.is_1d());
        assert_eq!(decoder_state(&decoder), before);
    }

    #[test]
    fn test_transform_type_large_tx_threshold_is_one_class_lower_for_intra() {
        let data = vec![0x80, 0x00, 0xFF, 0xFF, 0xAA, 0xBB];
        let mut decoder = SymbolDecoder::new(&data).unwrap();
        let before = decoder_state(&decoder);
        // 32x32 (tx_size_class=3), intra: 3+1>=4 -> large tx, zero bits (unlike inter at the same
        // size -- see `read_transform_type_is_1d`'s doc on this real spec asymmetry).
        let is_1d = decoder
            .read_transform_type_is_1d(true, false, false, false, 32, 32, 0)
            .unwrap();
        assert!(!is_1d.is_1d());
        assert_eq!(decoder_state(&decoder), before);
    }

    #[test]
    fn test_transform_type_32x32_inter_is_not_large_tx_and_reads_bits() {
        let data = vec![0x80, 0x00, 0xFF, 0xFF, 0xAA, 0xBB];
        let mut decoder = SymbolDecoder::new(&data).unwrap();
        let before = decoder_state(&decoder);
        // 32x32, inter: 3+0<4, not large -- falls into the real txtp_inter3 read (reduced/32x32
        // branch), which must consume real bits.
        let _is_1d = decoder
            .read_transform_type_is_1d(false, false, false, false, 32, 32, 0)
            .unwrap();
        assert_ne!(decoder_state(&decoder), before);
    }

    #[test]
    fn test_transform_type_intra2_reduced_branch_never_returns_1d() {
        // Intra2's alphabet (IDTX/DCT_DCT/ADST_ADST/ADST_DCT/DCT_ADST) is entirely TX_CLASS_2D --
        // run it across enough synthetic decoders to exercise multiple symbol values and confirm
        // `is_1d` is always false, matching the alphabet's real composition (no V_DCT/H_DCT).
        for seed in 0u8..8 {
            let data = vec![0x80, seed, 0xFF, 0xFF, 0xAA ^ seed, 0xBB];
            let mut decoder = SymbolDecoder::new(&data).unwrap();
            let is_1d = decoder
                .read_transform_type_is_1d(true, false, false, true, 4, 4, 0)
                .unwrap();
            assert!(!is_1d.is_1d());
        }
    }

    #[test]
    fn test_transform_type_intra1_branch_reads_real_bits_and_returns_valid_bool() {
        let data = vec![0x80, 0x00, 0xFF, 0xFF, 0xAA, 0xBB];
        let mut decoder = SymbolDecoder::new(&data).unwrap();
        let before = decoder_state(&decoder);
        // 4x4, intra, not reduced -- reaches txtp_intra1 (real 7-symbol read).
        let _is_1d = decoder
            .read_transform_type_is_1d(true, false, false, false, 4, 4, 0)
            .unwrap();
        assert_ne!(decoder_state(&decoder), before);
    }
}
