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
mod inter;
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

    /// Read `skip_mode` (spec 5.11.5) -- real per-`ctx` CDF + adaptation, matching `read_skip`'s
    /// bar. Real spec gate (dav1d's `decode_b`): only read at all when the frame's
    /// `skip_mode_present` is true AND `min(bw4, bh4) > 1` (never for 4-wide-or-tall blocks) --
    /// callers must check both before calling this and default to `false` otherwise, matching
    /// spec's own "absent means 0" convention (real spec never signals `skip_mode` for a frame/
    /// block combination that doesn't qualify, not even an implicit always-0 bit). `ctx`:
    /// `TileContext::skip_mode_context`'s doc. Previously never read at all -- see
    /// `crate::tile::coding_unit::parse_coding_unit`'s doc for the desync this closes (this
    /// crate treated every non-key-frame CU as unconditionally inter, matching real spec only
    /// when every CU in every inter frame happens to skip both `skip_mode` and `is_inter`, which
    /// isn't how AV1 works).
    pub fn read_skip_mode(&mut self, ctx: u8) -> Result<bool> {
        let cdf = self.cdf_context.get_skip_mode_cdf_mut(ctx);
        let symbol = self.decoder.read_symbol_adaptive(cdf)?;
        Ok(symbol == 1)
    }

    /// Read `is_inter` (spec 5.11.5's real per-CU intra/inter dispatch bit for non-intra-only
    /// frames) -- real per-`ctx` CDF + adaptation. `ctx`: `TileContext::intra_ctx`'s doc. Returns
    /// `true` for INTER (matches this crate's existing `is_key_frame`-implies-intra convention:
    /// `!read_is_inter(...)` gives the real `b->intra` spec meaning directly). Real spec gate:
    /// only called when NOT `skip_mode` (which forces inter with no bit read) and segmentation
    /// doesn't force a value via a per-segment `SEG_LVL_REF_FRAME`/`SEG_LVL_GLOBALMV` override --
    /// this crate doesn't model per-segment feature data (`SegmentationInfo`'s doc), so callers
    /// gate on `!skip_mode` only; the segmentation-override case is a known, deliberately
    /// undertested gap (this crate's committed fixture never enables segmentation at all --
    /// [[feedback_no_third_party_test_data]]-equivalent caveat, matches `read_segment_id`'s own
    /// existing scope limit).
    pub fn read_is_inter(&mut self, ctx: u8) -> Result<bool> {
        let cdf = self.cdf_context.get_intra_cdf_mut(ctx);
        let symbol = self.decoder.read_symbol_adaptive(cdf)?;
        // dav1d: `b->intra = !decode_bool_adapt(...)` -- the raw decoded bool IS `is_inter`
        // directly (1 => intra=0 => inter; 0 => intra=1), not its complement.
        Ok(symbol == 1)
    }

    /// Read `motion_mode` (spec 5.11.27) when warp is a real candidate at this position (real
    /// spec: 3-symbol alphabet, `0`=SIMPLE/translation, `1`=OBMC, `2`=WARPED) -- real per-exact-
    /// block-size CDF + adaptation. `size_idx`: `crate::tile::coding_unit::motion_mode_size_index`.
    /// Callers must use `read_obmc` instead (a real 2-symbol alphabet) when warp isn't a candidate
    /// (`TileContext::has_matching_single_ref`'s doc) -- reading the wrong arity here is a real
    /// desync, not a value-only difference (see this crate's earlier `txb_skip`/`coeff_base_eob`
    /// wrong-arity fixes this session for the same failure shape).
    pub fn read_motion_mode(&mut self, size_idx: u8) -> Result<u8> {
        let cdf = self.cdf_context.get_motion_mode_cdf_mut(size_idx);
        self.decoder.read_symbol_adaptive(cdf)
    }

    /// Read `use_obmc` (spec 5.11.27's 2-symbol fallback when warp isn't a candidate) -- real
    /// per-exact-block-size CDF + adaptation. Returns `true` for OBMC, `false` for SIMPLE/
    /// translation (matches `motion_mode`'s own `0`/`1` values, so callers can treat both
    /// functions' results uniformly by mapping this to `0`/`1`).
    pub fn read_obmc(&mut self, size_idx: u8) -> Result<bool> {
        let cdf = self.cdf_context.get_obmc_cdf_mut(size_idx);
        let symbol = self.decoder.read_symbol_adaptive(cdf)?;
        Ok(symbol == 1)
    }

    /// Read `interintra` (spec 5.11.29's real eligibility bit -- true means this block actually
    /// uses interintra) -- real per-`ctx` CDF + adaptation. `ctx`:
    /// `crate::tile::coding_unit::y_mode_size_context` (reused verbatim, real dav1d shares the
    /// same block-size-class index between this and non-key-frame `y_mode`).
    pub fn read_interintra(&mut self, ctx: u8) -> Result<bool> {
        let cdf = self.cdf_context.get_interintra_cdf_mut(ctx);
        let symbol = self.decoder.read_symbol_adaptive(cdf)?;
        Ok(symbol == 1)
    }

    /// Read `interintra_mode` (4-symbol) -- real per-`ctx` CDF + adaptation, same `ctx` as
    /// `read_interintra`.
    pub fn read_interintra_mode(&mut self, ctx: u8) -> Result<u8> {
        let cdf = self.cdf_context.get_interintra_mode_cdf_mut(ctx);
        self.decoder.read_symbol_adaptive(cdf)
    }

    /// Read `wedge_interintra` (real per-`wedge_ctx` CDF + adaptation) -- `true` means this
    /// interintra block additionally uses a wedge mask (vs. a plain blend). `wedge_ctx`:
    /// `crate::tile::coding_unit::wedge_ctx`, capped to interintra's real 7-size subset (`<=6`,
    /// `CdfContext::interintra_wedge_cdf`'s doc) -- callers must not call this for the 2 wedge-only
    /// sizes (`wedge_ctx` `7`/`8`, 8x32/32x8) that aren't interintra-eligible at all.
    pub fn read_interintra_wedge(&mut self, wedge_ctx: u8) -> Result<bool> {
        let cdf = self.cdf_context.get_interintra_wedge_cdf_mut(wedge_ctx);
        let symbol = self.decoder.read_symbol_adaptive(cdf)?;
        Ok(symbol == 1)
    }

    /// Read `wedge_index` (spec 5.11.28, 16-symbol) -- real per-`wedge_ctx` CDF + adaptation
    /// (`crate::tile::coding_unit::wedge_ctx`'s doc), shared verbatim between compound wedge and
    /// interintra's own wedge-index read (real spec: the same `wedge_idx()` syntax element).
    pub fn read_wedge_idx(&mut self, wedge_ctx: u8) -> Result<u8> {
        let cdf = self.cdf_context.get_wedge_idx_cdf_mut(wedge_ctx);
        self.decoder.read_symbol_adaptive(cdf)
    }

    /// Read `wedge_compound` (real per-`wedge_ctx` CDF + adaptation) -- within the masked-compound
    /// branch (`read_mask_comp` returned `true`), `true` means wedge, `false` means the (simpler,
    /// no wedge-index bits) segmentation mask variant.
    pub fn read_wedge_comp(&mut self, wedge_ctx: u8) -> Result<bool> {
        let cdf = self.cdf_context.get_wedge_comp_cdf_mut(wedge_ctx);
        let symbol = self.decoder.read_symbol_adaptive(cdf)?;
        // dav1d: `b->comp_type = COMP_INTER_WEDGE - decode_bool_adapt(...)` -- bit=0 => WEDGE,
        // bit=1 => WEDGE-1 (SEG), i.e. the raw bit IS "not wedge" directly.
        Ok(symbol == 0)
    }

    /// Read `mask_comp` (spec 5.11.28's jnt_comp-vs-masked selector) -- real per-`ctx` CDF +
    /// adaptation, `ctx`: `TileContext::mask_comp_context`'s doc. `true` means this compound
    /// block uses a mask (wedge or segmentation, see `read_wedge_comp`); `false` means the
    /// simpler jnt_comp path (`read_jnt_comp`).
    pub fn read_mask_comp(&mut self, ctx: u8) -> Result<bool> {
        let cdf = self.cdf_context.get_mask_comp_cdf_mut(ctx);
        let symbol = self.decoder.read_symbol_adaptive(cdf)?;
        Ok(symbol == 1)
    }

    /// Read `jnt_comp` (spec 5.11.28's weighted-vs-plain-average bit) -- real per-`ctx` CDF +
    /// adaptation, `ctx`: `TileContext::jnt_comp_context`'s doc.
    pub fn read_jnt_comp(&mut self, ctx: u8) -> Result<bool> {
        let cdf = self.cdf_context.get_jnt_comp_cdf_mut(ctx);
        let symbol = self.decoder.read_symbol_adaptive(cdf)?;
        Ok(symbol == 1)
    }

    /// Read `filter` (spec 5.11.30, one subpel-interpolation-filter symbol per direction) --
    /// real per-`(dir, ctx)` CDF + adaptation. `dir`: `0`=horizontal, `1`=vertical. `ctx`:
    /// `TileContext::filter_context`'s doc.
    pub fn read_filter(&mut self, dir: u8, ctx: u8) -> Result<u8> {
        let cdf = self.cdf_context.get_filter_cdf_mut(dir, ctx);
        self.decoder.read_symbol_adaptive(cdf)
    }

    /// Read one `drl_bit` (spec 7.10.2.10's real DRL index-selection bit, single-ref -- see
    /// `crate::tile::context::SpatialRefContext::single_ref_mv_stack`'s doc for the real weighted
    /// candidate list this decides between, and its doc for the temporal/compound/sign_bias-
    /// extension scope this crate omits) -- real per-`ctx` CDF + adaptation. `ctx`:
    /// `crate::tile::context::get_drl_context`'s doc. `true` means "advance to the next DRL
    /// position" (matches real spec's `b->drl_idx += bit` accumulation).
    pub fn read_drl_bit(&mut self, ctx: u8) -> Result<bool> {
        let cdf = self.cdf_context.get_drl_bit_cdf_mut(ctx);
        let symbol = self.decoder.read_symbol_adaptive(cdf)?;
        Ok(symbol == 1)
    }

    /// Read `seg_pred` (spec 5.11.9/5.11.10's temporal segment-id-prediction flag) -- real
    /// per-`ctx` CDF + adaptation, matching `read_skip`'s bar. `ctx`:
    /// `TileContext::seg_pred_context`'s doc.
    pub fn read_seg_pred(&mut self, ctx: u8) -> Result<bool> {
        let cdf = self.cdf_context.get_seg_pred_cdf_mut(ctx);
        let symbol = self.decoder.read_symbol_adaptive(cdf)?;
        Ok(symbol == 1)
    }

    /// Read the raw `segment_id` diff symbol (spec 5.11.9/5.11.10's `S()` read within
    /// `read_segment_id()`) -- real per-`ctx` CDF + adaptation. `ctx`:
    /// `TileContext::segment_id_context`'s doc. Returns the raw 0..=7 symbol -- callers must
    /// still apply `neg_deinterleave` against the predicted segment id to get the real segment id
    /// (see `crate::tile::coding_unit`'s `neg_deinterleave` and its call site's doc).
    pub fn read_segment_id_diff(&mut self, ctx: u8) -> Result<u8> {
        let cdf = self.cdf_context.get_seg_id_cdf_mut(ctx);
        self.decoder.read_symbol_adaptive(cdf)
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

    /// Read INTRA prediction mode
    /// Read key-frame `intra_mode` (spec 5.11.10 `kf_y_mode`), per its `above_mode_class`/
    /// `left_mode_class` context (0..=4 each -- see `crate::tile::TileContext::intra_mode_context`).
    ///
    /// Real per-context default CDFs (`CdfContext`'s `kfym` doc) and real adaptation via
    /// `read_symbol_adaptive`, matching `read_skip`'s bar -- not the "representative,
    /// context-independent, non-adaptive" one the rest of this file still uses (see
    /// `docs/DEVELOPMENT_PHASES.md` Phase 4's AV1 entropy-decoding note).
    ///
    /// Returns INTRA mode symbol (0-12):
    /// - 0: DC_PRED
    /// - 1: V_PRED
    /// - 2: H_PRED
    /// - 3-12: Directional and smooth modes
    pub fn read_intra_mode(&mut self, above_class: u8, left_class: u8) -> Result<u8> {
        let cdf = self.cdf_context.get_kfym_cdf_mut(above_class, left_class);
        self.decoder.read_symbol_adaptive(cdf)
    }

    /// Read `y_mode` for an intra-coded CU within a NON-key frame (spec 5.11.7's
    /// `intra_block_mode_info()`, distinct from `read_intra_mode`'s key-frame-only `kfym`) --
    /// real per-block-size-class CDF + adaptation. `size_ctx`:
    /// `crate::tile::coding_unit::y_mode_size_context`'s doc (NOT an above/left neighbor lookup,
    /// unlike `kfym` -- verified against dav1d's `decode_b`: `IS_INTER_OR_SWITCH(f->frame_hdr) ?
    /// cdf.m.y_mode[...] : cdf.kfym[...][...]`, a real, deliberate CDF-source swap on the SAME
    /// unified intra mode-info code path, not two separate implementations -- everything after
    /// `y_mode` (angle_delta/uv_mode/cfl/palette/filter_intra) is byte-for-byte identical between
    /// key-frame and non-key-frame intra CUs).
    pub fn read_intra_mode_inter_frame(&mut self, size_ctx: u8) -> Result<u8> {
        let cdf = self.cdf_context.get_y_mode_cdf_mut(size_ctx);
        self.decoder.read_symbol_adaptive(cdf)
    }

    /// Read `angle_delta_y`/`angle_delta_uv` (spec `intra_angle_info_y`/`_uv`) -- real per-mode CDF
    /// + adaptation, shared table for Y and UV (`CdfContext::angle_delta_cdf`'s doc).
    ///
    /// `mode_minus_vert`: `mode - V_PRED` (0..=7). Returns the real signed delta, `-3..=3`.
    pub fn read_angle_delta(&mut self, mode_minus_vert: u8) -> Result<i8> {
        let cdf = self.cdf_context.get_angle_delta_cdf_mut(mode_minus_vert);
        let symbol = self.decoder.read_symbol_adaptive(cdf)?;
        Ok(symbol as i8 - 3)
    }

    /// Read `uv_mode` -- real per-`(cfl_allowed, y_mode)` CDF + adaptation
    /// (`CdfContext::uv_mode_cdf`'s doc). Returns the raw mode symbol: `0..=12` are the same 13
    /// intra modes as `y_mode`; `13` (`UV_CFL_PRED`, only reachable when `cfl_allowed`) means
    /// chroma-from-luma, handled by `read_cfl_alphas` at the call site.
    pub fn read_uv_mode(&mut self, cfl_allowed: bool, y_mode: u8) -> Result<u8> {
        let cdf = self.cdf_context.get_uv_mode_cdf_mut(cfl_allowed, y_mode);
        self.decoder.read_symbol_adaptive(cdf)
    }

    /// Read `cfl_alpha_signs` + `cfl_alpha_u`/`cfl_alpha_v` (spec 5.11.45) -- ported exactly from
    /// dav1d's `read_pal_uv`-adjacent CFL block (`src/decode.c`), not the spec pseudocode alone:
    /// the sign symbol (8 outcomes, `+1` giving `1..=8`) packs `(sign_u, sign_v)` as base-3 digits
    /// (`sign_u = sign/3`, `sign_v = sign - sign_u*3`; `0`=zero/absent, `1`=negative, `2`=positive
    /// -- the all-zero combination is unreachable by construction, real spec: CFL would never be
    /// selected if both alphas were zero), each present component's alpha magnitude then read via
    /// its own context (`(sign == 2) as u8 * 3 + other_sign`) and negated when that component's
    /// sign was `1`. Returns `(alpha_u, alpha_v)`, each `-16..=16` with `0` meaning "not present".
    pub fn read_cfl_alphas(&mut self) -> Result<(i8, i8)> {
        let sign_cdf = self.cdf_context.get_cfl_sign_cdf_mut();
        let sign = self.decoder.read_symbol_adaptive(sign_cdf)? + 1;
        let sign_u = sign / 3;
        let sign_v = sign - sign_u * 3;

        let alpha_u = if sign_u != 0 {
            let ctx = if sign_u == 2 { 3 } else { 0 } + sign_v;
            let cdf = self.cdf_context.get_cfl_alpha_cdf_mut(ctx);
            let magnitude = self.decoder.read_symbol_adaptive(cdf)? as i8 + 1;
            if sign_u == 1 {
                -magnitude
            } else {
                magnitude
            }
        } else {
            0
        };
        let alpha_v = if sign_v != 0 {
            let ctx = if sign_v == 2 { 3 } else { 0 } + sign_u;
            let cdf = self.cdf_context.get_cfl_alpha_cdf_mut(ctx);
            let magnitude = self.decoder.read_symbol_adaptive(cdf)? as i8 + 1;
            if sign_v == 1 {
                -magnitude
            } else {
                magnitude
            }
        } else {
            0
        };

        Ok((alpha_u, alpha_v))
    }

    /// Read `use_filter_intra` (spec `filter_intra_mode_info()`) -- real per-`BlockSize` CDF +
    /// adaptation.
    pub fn read_use_filter_intra(&mut self, bs: crate::tile::BlockSize) -> Result<bool> {
        let cdf = self.cdf_context.get_use_filter_intra_cdf_mut(bs);
        Ok(self.decoder.read_symbol_adaptive(cdf)? == 1)
    }

    /// Read `filter_intra_mode` (5-symbol) -- real CDF + adaptation, no context.
    pub fn read_filter_intra_mode(&mut self) -> Result<u8> {
        let cdf = self.cdf_context.get_filter_intra_mode_cdf_mut();
        self.decoder.read_symbol_adaptive(cdf)
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
