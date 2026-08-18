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
pub mod scan;

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

    /// Read `inter_mode()` per AV1 spec Section 5.11.23, as 3 cascaded real-context-adaptive
    /// booleans (`newmv`/`globalmv`/`refmv`), not a single 4-way symbol -- matches rav1d's actual
    /// decode tree (`decode.rs`, `memorysafety/rav1d`, BSD-2-Clause): `newmv_bit == 0` means
    /// NEWMV; otherwise `globalmv_bit == 0` means GLOBALMV; otherwise `refmv_bit` distinguishes
    /// NEARMV (1) from NEARESTMV (0). `ctx` is `crate::tile::TileContext::inter_mode_context`'s
    /// packed result (`refmv_ctx << 4 | globalmv_ctx << 3 | newmv_ctx`) -- real per-context CDFs
    /// (`CdfContext`'s `newmv_mode_cdf`/`globalmv_mode_cdf`/`refmv_mode_cdf` doc) + real
    /// adaptation via `read_symbol_adaptive`, matching `read_skip`/`read_ref_frames`'s bar.
    ///
    /// Returns INTER mode symbol (0-3):
    /// - 0: NEWMV (read explicit MV)
    /// - 1: NEARESTMV (use nearest neighbor MV)
    /// - 2: NEARMV (use near neighbor MV)
    /// - 3: GLOBALMV (use global motion MV)
    pub fn read_inter_mode(&mut self, ctx: u16) -> Result<u8> {
        let newmv_ctx = (ctx & 7) as u8;
        let cdf = self.cdf_context.get_newmv_mode_cdf_mut(newmv_ctx);
        let newmv_bit = self.decoder.read_symbol_adaptive(cdf)?;
        if newmv_bit == 0 {
            return Ok(0); // NEWMV
        }

        let globalmv_ctx = (ctx >> 3 & 1) as u8;
        let cdf = self.cdf_context.get_globalmv_mode_cdf_mut(globalmv_ctx);
        let globalmv_bit = self.decoder.read_symbol_adaptive(cdf)?;
        if globalmv_bit == 0 {
            return Ok(3); // GLOBALMV
        }

        let refmv_ctx = (ctx >> 4 & 15) as u8;
        let cdf = self.cdf_context.get_refmv_mode_cdf_mut(refmv_ctx);
        let refmv_bit = self.decoder.read_symbol_adaptive(cdf)?;
        Ok(if refmv_bit == 1 { 2 } else { 1 }) // NEARMV : NEARESTMV
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
    /// Real per-context CDF (`CdfContext`'s `compound_mode_cdf` doc) + real adaptation via
    /// `read_symbol_adaptive`, matching `read_inter_mode`'s bar. `ctx` (0..=7) is
    /// `crate::tile::TileContext::compound_mode_context`'s result -- fully real, no temporal
    /// dependency (see that method's doc).
    pub fn read_compound_mode(&mut self, ctx: u8) -> Result<u8> {
        let cdf = self.cdf_context.get_compound_mode_cdf_mut(ctx);
        self.decoder.read_symbol_adaptive(cdf)
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
    /// Real per-context CDFs (`CdfContext`'s ref-frame CDF fields' doc) and real adaptation via
    /// `read_symbol_adaptive`, matching `read_skip`/`read_intra_mode`/`read_partition`'s bar --
    /// context comes from `tile_ctx` at absolute 4x4 position `(x4, y4)` (see
    /// `crate::tile::TileContext`'s `comp_mode_context`/`comp_ref_type_context`/
    /// `single_ref_p*_context`/`uni_comp_ref_p1_context` methods, ported from rav1d's
    /// `get_comp_ctx`/`get_comp_dir_ctx`/`av1_get_*_ctx`/`av1_get_uni_p1_ctx`). The caller is
    /// still responsible for writing the result back via `tile_ctx.set_ref_frames` -- this method
    /// only reads, matching `read_skip`/`read_intra_mode`'s split (context lookup and context
    /// write live in `TileContext`, not here).
    ///
    /// Real per-block *mode* reading for compound blocks (`compound_mode`, a different/larger
    /// symbol alphabet than this decoder's 4-way `read_inter_mode`) and L1 motion-vector reading
    /// are implemented one layer up, in `tile::coding_unit::parse_coding_unit` (real
    /// `compound_mode()` + `MvKind`-based L0/L1 strategy split, `188d3ad`) -- this method only
    /// reads `ref_frame[0]`/`ref_frame[1]`'s categorical values (and the entropy-decoder bit
    /// consumption needed to reach them correctly); mode/MV reading for the resolved ref-frame
    /// pair happens after this method returns, not inside it.
    pub fn read_ref_frames(
        &mut self,
        tile_ctx: &crate::tile::TileContext,
        x4: u32,
        y4: u32,
        reference_select: bool,
        min_block_dim_px: u32,
    ) -> Result<[crate::tile::RefFrame; 2]> {
        use crate::tile::RefFrame;

        let is_compound = if reference_select && min_block_dim_px >= 8 {
            let ctx = tile_ctx.comp_mode_context(x4, y4);
            let cdf = self.cdf_context.get_comp_mode_cdf_mut(ctx);
            self.decoder.read_symbol_adaptive(cdf)? == 1
        } else {
            false
        };

        if !is_compound {
            let ctx = tile_ctx.single_ref_p1_context(x4, y4);
            let cdf = self.cdf_context.get_single_ref_p1_cdf_mut(ctx);
            let backward = self.decoder.read_symbol_adaptive(cdf)? == 1;

            let ref0 = if backward {
                let ctx = tile_ctx.single_ref_p2_context(x4, y4);
                let cdf = self.cdf_context.get_single_ref_p2_cdf_mut(ctx);
                let is_altref = self.decoder.read_symbol_adaptive(cdf)? == 1;
                if is_altref {
                    RefFrame::AltRef
                } else {
                    let ctx = tile_ctx.single_ref_p6_context(x4, y4);
                    let cdf = self.cdf_context.get_single_ref_p6_cdf_mut(ctx);
                    let is_altref2 = self.decoder.read_symbol_adaptive(cdf)? == 1;
                    if is_altref2 {
                        RefFrame::AltRef2
                    } else {
                        RefFrame::BwdRef
                    }
                }
            } else {
                let ctx = tile_ctx.single_ref_p3_context(x4, y4);
                let cdf = self.cdf_context.get_single_ref_p3_cdf_mut(ctx);
                let last3_or_golden = self.decoder.read_symbol_adaptive(cdf)? == 1;
                if last3_or_golden {
                    let ctx = tile_ctx.single_ref_p5_context(x4, y4);
                    let cdf = self.cdf_context.get_single_ref_p5_cdf_mut(ctx);
                    let is_golden = self.decoder.read_symbol_adaptive(cdf)? == 1;
                    if is_golden {
                        RefFrame::Golden
                    } else {
                        RefFrame::Last3
                    }
                } else {
                    let ctx = tile_ctx.single_ref_p4_context(x4, y4);
                    let cdf = self.cdf_context.get_single_ref_p4_cdf_mut(ctx);
                    let is_last2 = self.decoder.read_symbol_adaptive(cdf)? == 1;
                    if is_last2 {
                        RefFrame::Last2
                    } else {
                        RefFrame::Last
                    }
                }
            };
            return Ok([ref0, RefFrame::Intra]);
        }

        let ctx = tile_ctx.comp_ref_type_context(x4, y4);
        let cdf = self.cdf_context.get_comp_ref_type_cdf_mut(ctx);
        let is_bidir = self.decoder.read_symbol_adaptive(cdf)? == 1;

        if !is_bidir {
            // Unidirectional compound: both references from the same "direction" group. Context
            // functions are reused from the single-ref path (rav1d: `av1_get_ref_ctx` for
            // `uni_comp_ref`, `av1_get_uni_p1_ctx` for `uni_comp_ref_p1`, `av1_get_fwd_ref_2_ctx`
            // for `uni_comp_ref_p2`) even though the CDF storage is separate.
            let ctx = tile_ctx.single_ref_p1_context(x4, y4);
            let cdf = self.cdf_context.get_uni_comp_ref_cdf_mut(ctx);
            let is_bwdref_altref = self.decoder.read_symbol_adaptive(cdf)? == 1;
            if is_bwdref_altref {
                return Ok([RefFrame::BwdRef, RefFrame::AltRef]);
            }
            let ctx = tile_ctx.uni_comp_ref_p1_context(x4, y4);
            let cdf = self.cdf_context.get_uni_comp_ref_p1_cdf_mut(ctx);
            let p1 = self.decoder.read_symbol_adaptive(cdf)? == 1;
            if !p1 {
                return Ok([RefFrame::Last, RefFrame::Last2]);
            }
            let ctx = tile_ctx.single_ref_p5_context(x4, y4);
            let cdf = self.cdf_context.get_uni_comp_ref_p2_cdf_mut(ctx);
            let is_golden = self.decoder.read_symbol_adaptive(cdf)? == 1;
            return Ok([
                RefFrame::Last,
                if is_golden {
                    RefFrame::Golden
                } else {
                    RefFrame::Last3
                },
            ]);
        }

        // Bidirectional compound: independent forward + backward group choices. Context functions
        // reused from the single-ref path (rav1d: `av1_get_fwd_ref_ctx`/`_1_ctx`/`_2_ctx` for
        // `comp_ref`/`comp_ref_p1`/`comp_ref_p2`, `av1_get_bwd_ref_ctx`/`_1_ctx` for
        // `comp_bwdref`/`comp_bwdref_p1`).
        let ctx = tile_ctx.single_ref_p3_context(x4, y4);
        let cdf = self.cdf_context.get_comp_ref_cdf_mut(ctx);
        let fwd_group_b = self.decoder.read_symbol_adaptive(cdf)? == 1;
        let ref0 = if !fwd_group_b {
            let ctx = tile_ctx.single_ref_p4_context(x4, y4);
            let cdf = self.cdf_context.get_comp_ref_p1_cdf_mut(ctx);
            let is_last2 = self.decoder.read_symbol_adaptive(cdf)? == 1;
            if is_last2 {
                RefFrame::Last2
            } else {
                RefFrame::Last
            }
        } else {
            let ctx = tile_ctx.single_ref_p5_context(x4, y4);
            let cdf = self.cdf_context.get_comp_ref_p2_cdf_mut(ctx);
            let is_golden = self.decoder.read_symbol_adaptive(cdf)? == 1;
            if is_golden {
                RefFrame::Golden
            } else {
                RefFrame::Last3
            }
        };

        let ctx = tile_ctx.single_ref_p2_context(x4, y4);
        let cdf = self.cdf_context.get_comp_bwdref_cdf_mut(ctx);
        let bwd_group_b = self.decoder.read_symbol_adaptive(cdf)? == 1;
        let ref1 = if !bwd_group_b {
            let ctx = tile_ctx.single_ref_p6_context(x4, y4);
            let cdf = self.cdf_context.get_comp_bwdref_p1_cdf_mut(ctx);
            let is_altref2 = self.decoder.read_symbol_adaptive(cdf)? == 1;
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

    /// Read `mv_joint` (AV1 spec Section 5.11.32 `read_mv()`) -- gates which axis, if any, has a
    /// coded component to read next. Returns one of the `MV_JOINT_*` values (see
    /// `CdfContext::get_mv_joint_cdf_mut`'s doc): 0=both zero, 1=horizontal only, 2=vertical
    /// only, 3=both. Real adaptation via `read_symbol_adaptive` -- MV CDFs have no above/left
    /// neighbor context in the real spec (unlike `skip`/`kfym`), just frame-lifetime adaptation.
    pub fn read_mv_joint(&mut self) -> Result<u8> {
        let cdf = self.cdf_context.get_mv_joint_cdf_mut();
        self.decoder.read_symbol_adaptive(cdf)
    }

    /// Read motion vector component (horizontal or vertical)
    ///
    /// Per AV1 Spec Section 5.11.47 (Motion Vector Component)
    ///
    /// Returns MV component in quarter-pel units (divide by 4 for pixel units)
    pub fn read_mv_component(&mut self) -> Result<i32> {
        // Read MV class (magnitude range)
        let mv_class_cdf = self.cdf_context.get_mv_class_cdf_mut();

        tracing::trace!(
            "  Before read_mv_class: decoder.value={:#06x}, decoder.range={:#06x}",
            self.decoder.value,
            self.decoder.range
        );

        let mv_class = self.decoder.read_symbol_adaptive(mv_class_cdf)?;

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
            let mv_bit_cdf = self.cdf_context.get_mv_bit_cdf_mut();
            for i in 0..num_bits {
                let bit = self.decoder.read_symbol_adaptive(mv_bit_cdf)?;
                mag = (mag << 1) | bit as i32;
                tracing::trace!("      bit[{}] = {} → mag = {}", i, bit, mag);
            }

            tracing::trace!("    Final magnitude = {}", mag);
            mag
        };

        // Read sign (0 = positive, 1 = negative)
        let sign = if magnitude > 0 {
            let mv_sign_cdf = self.cdf_context.get_mv_sign_cdf_mut();
            tracing::trace!(
                "  Before read_sign: decoder.value={:#06x}, decoder.range={:#06x}",
                self.decoder.value,
                self.decoder.range
            );
            let s = self.decoder.read_symbol_adaptive(mv_sign_cdf)?;
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
        let mv_bit_cdf = self.cdf_context.get_mv_bit_cdf_mut();

        tracing::trace!(
            "  Before read_fr: decoder.value={:#06x}, decoder.range={:#06x}",
            self.decoder.value,
            self.decoder.range
        );
        let fr = self.decoder.read_symbol_adaptive(mv_bit_cdf)? as i32;
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
        let hp = self.decoder.read_symbol_adaptive(mv_bit_cdf)? as i32;
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
    /// distinct from `coded_lossless`, which additionally requires zero delta-Q). `tx_size_px`:
    /// transform block size in pixels per side. `y_mode_raw`: the intra prediction mode symbol
    /// (0..=12, only meaningful when `is_intra`) -- spec's `FILTER_PRED` substitution never
    /// applies since this crate's `PredictionMode` has no such variant.
    ///
    /// Five CDF families cover the real decision tree (`CdfContext`'s `txtp_*_cdf` doc); which
    /// tx-size classes reach which family is a direct consequence of the branch conditions below,
    /// not arbitrary -- e.g. `txtp_intra1`/`txtp_inter1` only ever see tx classes 0..=1 (4x4/8x8)
    /// because every larger size is intercepted by an earlier branch first.
    pub fn read_transform_type_is_1d(
        &mut self,
        is_intra: bool,
        coded_lossless: bool,
        qidx_is_zero: bool,
        reduced_tx_set: bool,
        tx_size_px: u32,
        y_mode_raw: u8,
    ) -> Result<TxClass1d> {
        let tx_class = cdf::tx_size_class(tx_size_px);
        // `t_dim->max + intra >= TX_64X64`: intra additionally forces DCT_DCT (2D, no bits) one
        // tx-size class earlier than inter (at 32x32, not just 64x64) -- real spec asymmetry, not
        // a simplification.
        if coded_lossless || qidx_is_zero || tx_class + usize::from(is_intra) >= 4 {
            return Ok(TxClass1d::TwoD);
        }
        if is_intra {
            if reduced_tx_set || tx_class == 2 {
                // Intra2 alphabet (IDTX/DCT_DCT/ADST_ADST/ADST_DCT/DCT_ADST) is entirely
                // TX_CLASS_2D -- no symbol value here can ever produce a 1D class.
                let cdf = self
                    .cdf_context
                    .get_txtp_intra2_cdf_mut(tx_class, y_mode_raw);
                self.decoder.read_symbol_adaptive(cdf)?;
                Ok(TxClass1d::TwoD)
            } else {
                // Intra1 alphabet: IDTX, DCT_DCT, V_DCT, H_DCT, ADST_ADST, ADST_DCT, DCT_ADST.
                let cdf = self
                    .cdf_context
                    .get_txtp_intra1_cdf_mut(tx_class, y_mode_raw);
                let idx = self.decoder.read_symbol_adaptive(cdf)?;
                Ok(match idx {
                    2 => TxClass1d::Vertical,
                    3 => TxClass1d::Horizontal,
                    _ => TxClass1d::TwoD,
                })
            }
        } else if reduced_tx_set || tx_class == 3 {
            // Inter3 alphabet is a single bit choosing between IDTX and DCT_DCT -- both 2D.
            let cdf = self.cdf_context.get_txtp_inter3_cdf_mut(tx_class);
            self.decoder.read_symbol_adaptive(cdf)?;
            Ok(TxClass1d::TwoD)
        } else if tx_class == 2 {
            // Inter2 alphabet: IDTX, V_DCT, H_DCT, DCT_DCT, ADST_DCT, DCT_ADST, FLIPADST_DCT,
            // DCT_FLIPADST, ADST_ADST, FLIPADST_FLIPADST, ADST_FLIPADST, FLIPADST_ADST.
            let cdf = self.cdf_context.get_txtp_inter2_cdf_mut();
            let idx = self.decoder.read_symbol_adaptive(cdf)?;
            Ok(match idx {
                1 => TxClass1d::Vertical,
                2 => TxClass1d::Horizontal,
                _ => TxClass1d::TwoD,
            })
        } else {
            // Inter1 alphabet: IDTX, V_DCT, H_DCT, V_ADST, H_ADST, V_FLIPADST, H_FLIPADST,
            // DCT_DCT, ADST_DCT, DCT_ADST, FLIPADST_DCT, DCT_FLIPADST, ADST_ADST,
            // FLIPADST_FLIPADST, ADST_FLIPADST, FLIPADST_ADST -- V_* at odd idx (1,3,5), H_* at
            // even idx (2,4,6), everything else (0, 7..=15) 2D.
            let cdf = self.cdf_context.get_txtp_inter1_cdf_mut(tx_class);
            let idx = self.decoder.read_symbol_adaptive(cdf)?;
            Ok(match idx {
                1 | 3 | 5 => TxClass1d::Vertical,
                2 | 4 | 6 => TxClass1d::Horizontal,
                _ => TxClass1d::TwoD,
            })
        }
    }

    /// Read one transform block's residual coefficients (AV1 spec Section 5.11.39 `coeffs()`),
    /// returning summary statistics rather than a full per-position coefficient array -- this
    /// crate has no dequantization/inverse-transform/pixel-reconstruction stage, so individual
    /// coefficient positions aren't independently useful, only their aggregate magnitude.
    ///
    /// `width_px`/`height_px`: the transform block's real dimensions in pixels (previously a
    /// single square `tx_size_px` -- generalized for rectangular var-tx, see `scan::coeff_position`
    /// and `TxClass1d`'s docs; square callers just pass equal values). `class` is
    /// `read_transform_type_is_1d`'s result for this same transform block -- callers must read
    /// `read_txb_skip` first (real spec order: `all_zero`, then -- only if not all-zero --
    /// `transform_type()`, see that method's doc), skip this call entirely when it returns `true`,
    /// and otherwise read `transform_type()` next and pass its result here; it feeds both
    /// `eob_bin`'s context and (for `Horizontal`/`Vertical`) real per-class coefficient positions.
    ///
    /// # Known simplifications (see `symbol/cdf.rs`'s residual-CDF doc for the CDF side)
    ///
    /// - **`coeff_base`/`coeff_br` now have real neighbor/level context** (`scan::lo_ctx`, ported
    ///   from rav1d's `get_lo_ctx`, real per-position magnitude-band derivation from
    ///   already-decoded neighbor levels within the same transform block's scan order --
    ///   `6dc76ef`, "real coeff_base/coeff_br neighbor context + scan order") -- this doc
    ///   previously (incorrectly) still described these as context-independent placeholders after
    ///   that landed; corrected. `eob_bin`/`eob_hi_bit`/`coeff_base_eob` also use real context,
    ///   like `skip`/`intra_mode`/`inter_mode`/`ref_frame` (see `CdfContext::new`'s doc). `txb_skip`/
    ///   `dc_sign` **were previously** attempted with real above/left neighbor context and
    ///   reverted after real decode corruption -- root-caused to this crate's `tx_size` being a
    ///   dimension-based *heuristic* (`TxSize::from_dimensions`) rather than a real bitstream
    ///   read, meaning transform-block *boundaries* (and thus neighbor-array indexing) didn't
    ///   reliably match the real encoder's. Since `SymbolDecoder::read_tx_size` now provides a
    ///   real read for key-frame, non-IntraBC coding units, real `txb_skip`/`dc_sign` neighbor
    ///   context (`TileContext::txb_skip_context`/`dc_sign_context`/`set_residual_ctx`, ported
    ///   from rav1d's `get_skip_ctx`/`get_dc_sign_ctx`) is wired back in for exactly that subset
    ///   -- callers must pass `read_txb_skip`'s `txb_skip_ctx`/this method's `dc_sign_ctx` computed
    ///   from real neighbor state only when the transform boundaries are trustworthy, and `0` (the
    ///   old safe fallback) otherwise; see `parse_coding_unit`'s `use_real_residual_ctx` gate.
    /// - **`eob_extra` bits after the first are uniform literal bits**, not spec-exact CDF-coded
    ///   -- the real spec only context-codes the *first* extra bit (`eob_hi_bit`, real here); the
    ///   rest are genuinely literal per spec too, so this isn't a simplification for those.
    /// - **Single combined level+sign+golomb pass** per position (descending scan order) instead
    ///   of the spec's two separate passes (all levels, then all signs) -- doesn't change which
    ///   information gets read, only its order, which is irrelevant once `coeff_base`/`coeff_br`
    ///   themselves are still non-spec-exact.
    /// - **Golomb extension is a bounded, always-terminating read** (capped at 20 length bits) --
    ///   not necessarily bit-exact against the spec's `read_golomb`, but always produces a real,
    ///   finite level value.
    /// - **`tx_size()` is real for inter and IntraBC blocks too**, not just key-frame intra --
    ///   `read_var_tx_size`/`read_txfm_split` (`coding_unit.rs`) recursively read the real
    ///   variable-transform-size syntax for both, including non-square blocks; residual reading
    ///   here just reuses whichever real size the caller resolved.
    /// - **Chroma-plane residual is read by a separate method, `read_chroma_residual_block`, and
    ///   only for a restricted subset of coding blocks** -- callers of *this* method only ever
    ///   handle luma. **Confirmed a real desync bug, not just missing data**: the real fixture is
    ///   4:2:0 (non-monochrome), so any non-skip `HasChroma` coding block's real encoder wrote
    ///   chroma residual bits this crate previously never consumed. A first "shape-only" attempt
    ///   (reusing luma's CDF tables/context with a fixed `ctx=0`) regressed
    ///   `real_fixture_key_frame_intra_modes_are_not_degenerate` and was reverted -- root-caused
    ///   to applying *luma-trained* default probabilities to chroma data (chroma coefficient
    ///   statistics differ enough from luma's that the borrowed CDFs caused real symbol
    ///   misdecodes, which for variable-length constructs like `eob_bin`'s extra bits or the
    ///   golomb extension changes *how many bits get consumed*, not just their interpretation).
    ///   The follow-up fix (`read_chroma_residual_block`) ports **real chroma-specific default
    ///   CDF values** (rav1d's actual `[chroma=1]` axis, not luma's) instead of a from-scratch
    ///   context derivation -- verified via a dedicated research pass that rav1d's real chroma
    ///   transform-size mapping (`dav1d_max_txfm_size_for_bs`) and `HasChroma` condition exactly
    ///   match this crate's scope (square luma coding blocks 8x8 through 128x128, since landed --
    ///   see that method's doc). Still not covered: non-square luma coding blocks -- a real,
    ///   narrower, still-open version of this same gap (luma's own residual reader now handles
    ///   non-square via `read_residual_block`'s width/height split, but chroma's separate reader
    ///   hasn't been extended to match).
    ///
    /// None of these change the *shape* of the read sequence (an `all_zero` check, then -- when
    /// not all-zero -- an `eob_bin` symbol, `eob` extra bits, and exactly `eob + 1` per-position
    /// level/sign/golomb reads -- `eob` here is the raw last-scan-index value, not a count, see
    /// `read_residual_block`'s doc) -- which is what matters for keeping the shared arithmetic
    /// decoder's position advancing by a plausible amount instead of not reading residual data at
    /// all (see `crate::tile::coding_unit`'s module doc for why that previously caused real
    /// desync/crashes on real streams).
    /// Real `txb_skip` (spec 5.11.39's `all_zero`) -- real per-context CDF + adaptation. Callers
    /// MUST read this before `transform_type()` for a LUMA transform block and skip
    /// `transform_type()`/`read_residual_block` entirely when it returns `true` -- ported from
    /// dav1d's `decode_coefs` (`src/recon_tmpl.c`): `all_zero` is read first, unconditionally, and
    /// only when it comes back `false` does the function go on to determine `transform_type`
    /// (chroma: inferred, no bits; luma: real bits) and then coefficients. This crate previously
    /// read `transform_type()` unconditionally for every transform block regardless of
    /// `all_zero`, a real desync bug: every all-zero luma block (common for real content) read a
    /// phantom `transform_type` symbol the real encoder never wrote, permanently shifting the
    /// bitstream position for everything after it -- found while root-causing why this crate's
    /// only committed key-frame fixture was fragile to any bit-position shift at all (see
    /// `docs/DEVELOPMENT_PHASES.md`'s entropy-decoding notes).
    pub fn read_txb_skip(&mut self, tx_size_px_max: u32, txb_skip_ctx: u8) -> Result<bool> {
        let tx_class = cdf::tx_size_class(tx_size_px_max);
        let cdf = self
            .cdf_context
            .get_txb_skip_cdf_mut(tx_class, txb_skip_ctx);
        Ok(self.decoder.read_symbol_adaptive(cdf)? == 1)
    }

    /// Real luma transform-block coefficient read (spec 5.11.39 `coeffs()`), for a block ALREADY
    /// confirmed non-all-zero via `read_txb_skip`. See that method's doc for the real call-site
    /// ordering (`read_txb_skip` -> `transform_type()` -> this) and the desync it fixes -- this
    /// method no longer reads `txb_skip` itself (moved to `read_txb_skip`, called separately
    /// before `transform_type()`).
    pub fn read_residual_block(
        &mut self,
        width_px: u32,
        height_px: u32,
        class: TxClass1d,
        dc_sign_ctx: u8,
    ) -> Result<ResidualBlockStats> {
        // Real spec `txSzCtx` (`Tx_Size_Sqr_Up`, capped): CDF-family selection is by the
        // square-up class of the *larger* dimension, not either axis alone -- for a square
        // transform this is identical to the old single-scalar `tx_class`.
        let tx_class = cdf::tx_size_class(width_px.max(height_px));
        let is_1d = class.is_1d();

        // Real coefficient scan order + per-position neighbor context (spec 8.3.2's
        // `get_coef_base_ctx`/`get_br_ctx`) -- see `symbol::scan`'s module doc. Each axis is
        // independently capped at 32 (real AV1 never scans/contexts past the top-left 32-sample
        // extent on either axis, even for a transform with a 64-sample side -- see
        // `scan::scan_table`'s doc).
        let capped_width = width_px.min(32);
        let capped_height = height_px.min(32);
        let capped_class = tx_class.min(3);

        let eob_bin_cdf = self
            .cdf_context
            .get_eob_bin_cdf_mut(capped_width, capped_height, is_1d);
        let eob_bin = self.decoder.read_symbol_adaptive(eob_bin_cdf)? as u32;

        let eob: u32 = if eob_bin > 1 {
            let eob_hi_bit_cdf = self
                .cdf_context
                .get_eob_hi_bit_cdf_mut(tx_class, eob_bin as u8);
            let eob_hi_bit = self.decoder.read_symbol_adaptive(eob_hi_bit_cdf)? as u32;
            let num_extra_bits = eob_bin - 2;
            let mut extra = 0u32;
            for _ in 0..num_extra_bits {
                let bit = self.decoder.read_bool(16384)? as u32;
                extra = (extra << 1) | bit;
            }
            ((eob_hi_bit | 2) << (eob_bin - 2)) | extra
        } else {
            eob_bin
        };

        let mut stats = ResidualBlockStats {
            all_zero: false,
            ..Default::default()
        };

        let mut levels = scan::LevelBuffer::new(capped_width as usize, capped_height as usize);

        // `eob` (as computed above) is the raw last-scan-index value (rav1d's own local `eob`
        // convention), NOT a coefficient count: real spec has `eob + 1` total positions to read
        // (index `eob` itself down through `0`), with the top position (`c == eob`) using
        // `coeff_base_eob`'s CDF and everything below it using ordinary `coeff_base` -- confirmed
        // against rav1d's `decode_coefs`, whose "dc-only" branch (taken when this raw `eob` is
        // `0`) still reads exactly one `coeff_base_eob`-family symbol for the single coefficient,
        // never zero. Previously this looped `0..eob` (`eob` positions) with `is_eob_pos = c ==
        // eob - 1`, silently dropping the true top coefficient (and for `eob == 0`, reading zero
        // symbols instead of one) -- a real per-block desync, found while tracing this crate's
        // persistent key-frame-intra fragility (see `docs/DEVELOPMENT_PHASES.md`).
        for c in (0..=eob).rev() {
            let (x, y) =
                scan::coeff_position(capped_width, capped_height, is_1d, class.is_vertical(), c);
            let is_eob_pos = c == eob;

            // `br_ctx`: `coeff_br`'s context if this position's token turns out to need
            // extending (`base_level > 2`) -- computed alongside `base_level` since both draw
            // from the same neighbor lookup (`lo_ctx`'s `hi_mag` output, per its doc), matching
            // rav1d's `get_lo_ctx` call site producing both values together.
            let (base_level, br_ctx) = if is_eob_pos {
                // No neighbors decoded yet (this is the first position visited) -- `coeff_br`'s
                // context here is purely positional (spec: no magnitude bucket for the eob
                // position specifically), unlike every other position below.
                let ctx = coeff_base_eob_context(eob, width_px.max(height_px));
                let cdf = self.cdf_context.get_coeff_base_eob_cdf_mut(tx_class, ctx);
                let level = self.decoder.read_symbol_adaptive(cdf)? as u32 + 1;
                let pos_band = if is_1d { y > 0 } else { (x | y) > 1 };
                (level, if pos_band { 14 } else { 7 })
            } else if c == 0 {
                // DC position: 2D's `coeff_base` context is hardcoded to `0` (spec/rav1d), but
                // `coeff_br`'s magnitude still comes from the same 3-neighbor sum `lo_ctx` would
                // produce -- call it regardless of `is_1d` and only override the `coeff_base`
                // context choice, matching dav1d's manual-recompute-for-2D special case.
                let (lo_ctx_val, hi_mag) =
                    scan::lo_ctx(&levels, 0, 0, is_1d, capped_width, capped_height);
                let base_ctx = if is_1d { lo_ctx_val } else { 0 };
                let cdf = self.cdf_context.get_coeff_base_cdf_mut(tx_class, base_ctx);
                let level = self.decoder.read_symbol_adaptive(cdf)? as u32;
                let mag = hi_mag & 63;
                // DC's `coeff_br` context has no position-band offset (spec's lowest band).
                (level, (if mag > 12 { 6 } else { (mag + 1) >> 1 }) as u8)
            } else {
                let (ctx, hi_mag) = scan::lo_ctx(&levels, x, y, is_1d, capped_width, capped_height);
                let cdf = self.cdf_context.get_coeff_base_cdf_mut(tx_class, ctx);
                let level = self.decoder.read_symbol_adaptive(cdf)? as u32;
                let mag = hi_mag & 63;
                let pos_band = if is_1d { y > 0 } else { (x | y) > 1 };
                let band = if pos_band { 14 } else { 7 };
                (
                    level,
                    band + (if mag > 12 { 6 } else { (mag + 1) >> 1 }) as u8,
                )
            };

            let mut level = base_level;
            let extended = level > 2;
            if extended {
                let coeff_br_cdf = self.cdf_context.get_coeff_br_cdf_mut(capped_class, br_ctx);
                for _ in 0..4 {
                    let br = self.decoder.read_symbol_adaptive(coeff_br_cdf)? as u32;
                    level += br;
                    if br < 3 {
                        break;
                    }
                }
            }
            levels.set(x, y, extended, level);

            if level > 0 {
                if c == 0 {
                    let dc_sign_cdf = self.cdf_context.get_dc_sign_cdf_mut(dc_sign_ctx);
                    let sign = self.decoder.read_symbol_adaptive(dc_sign_cdf)?;
                    stats.dc_sign_value = Some(sign);
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

    /// Read one chroma-plane (U or V) transform block's residual coefficients -- the chroma
    /// counterpart to `read_residual_block`, see that method's doc for the full desync-bug
    /// history this closes (part of it) and the "Chroma-plane residual is never read at all"
    /// bullet for why this exists and what it deliberately doesn't cover yet.
    ///
    /// **Scope**: one call reads exactly one chroma transform block of `width_px`x`height_px`
    /// samples (each independently `<= 32`, chroma's real `Max_Tx_Size_Rect` cap regardless of
    /// luma size -- confirmed against rav1d's `DAV1D_MAX_TXFM_SIZE_FOR_BS` table). Callers must
    /// only invoke this for *non-IntraBC* luma coding blocks 8x8 through 128x128 in either
    /// dimension (`width_px`/`height_px` = `min(luma_dim/2, 32)` = 4/8/16/32 per axis, real
    /// rectangular chroma tiles supported since the luma CU itself can be non-square) on *any*
    /// frame type -- see `parse_coding_unit`'s call site for the exact gate and its tiling-loop
    /// shape (a chroma plane bigger than one tile in either axis needs real tiling, potentially
    /// asymmetric per axis).
    ///
    /// **128x128 luma: root-caused and fixed.** Two earlier passes shipped only the `(8..=64)`
    /// range after the tile *count* checked out against the real spec table but decode still
    /// desynced the next superblock. Root cause: `txb_skip`/`dc_sign` previously had **no** `ctx`
    /// parameter at all -- a single fixed CDF slot per `tx_size_class`, unconditionally shared by
    /// every call. A 64x64 luma block's single chroma tile per plane never exposed this (one call
    /// = one adaptation step, indistinguishable from a real single-context CDF). A 128x128 luma
    /// block's 4 tiles per plane, though, adapted that *same* shared global slot 4x more
    /// aggressively than a real encoder's per-position context ever would, drifting it away from
    /// the distribution real future superblocks' encoder-intended symbols assume -- eventual
    /// desync, not from a wrong tile count or wrong call order (both were already ruled out; see
    /// prior revisions of this doc in git history), but from over-adaptation of shared state.
    /// Fixed by adding real per-position `txb_skip`/`dc_sign` chroma context
    /// (`TileContext::txb_skip_context_chroma`/`dc_sign_context_chroma`, `(8..=128)` re-enabled at
    /// `parse_coding_unit`'s call site, real-fixture-verified 128x128 CUs parse cleanly with no
    /// regression to smaller sizes or later superblocks).
    ///
    /// **Non-square luma coding blocks: real support landed.** Previously excluded entirely
    /// (`width == height` gate at the call site) -- meaning every non-square `HasChroma` coding
    /// block's chroma residual bits were never read at all, a real, live desync bug matching this
    /// session's established pattern (silently wrong bits, not a crash) that only became common
    /// once non-square inter var-tx landed (550/1676 real fixture CUs). Fixed by generalizing this
    /// function and its CDF/context plumbing to independent width/height (mirrors
    /// `read_residual_block`'s identical generalization); chroma stays 2D-only (`is_1d` hardcoded
    /// `false`, unaffected by luma's H/V `TxClass1d` distinction since chroma's real
    /// `transform_type()` is never independently read).
    ///
    /// An *earlier* attempt at the 8x8/16x16/32x32-only scope additionally required
    /// `is_key_frame` (misdiagnosing the tx_size_class-3 regression above as frame-type-specific,
    /// since it was only ever tested at `tx_size_class` 3 on an inter frame) -- that restriction
    /// turned out to make the gate *never fire at all* against the real fixture (its key frames
    /// consist entirely of unpartitioned 128x128 blocks; only inter frames have small enough
    /// CUs), so every test passed vacuously until a dedicated test
    /// (`real_fixture_square_chroma_eligible_blocks_exist_and_parse_cleanly`) was added
    /// specifically to catch that. Removing the `is_key_frame` restriction is what's actually
    /// real-fixture-verified now.
    ///
    /// Unlike luma, `is_1d` is always `false` (2D) here -- chroma's real `transform_type()` isn't
    /// independently read at all (derived from luma's), and this crate doesn't attempt to derive
    /// it; `false` is the common case. `txb_skip`/`dc_sign` now use real per-position context
    /// (`txb_skip_ctx`/`dc_sign_ctx` params -- see `TileContext::txb_skip_context_chroma`'s doc
    /// for the desync bug this closes) against a real per-context default CDF
    /// (`CdfContext::txb_skip_cdf_chroma`/`dc_sign_cdf_chroma`'s doc). `coeff_base`/`coeff_br`
    /// reuse `symbol::scan::lo_ctx`'s real neighbor-context formula verbatim (it's plane-agnostic)
    /// against chroma-specific default CDF values, same as before.
    ///
    /// Returns `ResidualBlockStats` (mirrors `read_residual_block`) so the caller can feed
    /// `TileContext::set_residual_ctx_chroma` -- unlike the old no-context version, chroma's
    /// neighbor state must now be kept in sync for later chroma blocks' context lookups.
    pub fn read_chroma_residual_block(
        &mut self,
        width_px: u32,
        height_px: u32,
        txb_skip_ctx: u8,
        dc_sign_ctx: u8,
    ) -> Result<ResidualBlockStats> {
        // Real spec `txSzCtx`: square-up class of the *larger* dimension (see
        // `read_residual_block`'s identical derivation) -- for a square chroma tile these are the
        // same value, matching this function's pre-rect behavior exactly.
        let tx_class = cdf::tx_size_class(width_px.max(height_px)).min(3);

        let txb_skip_cdf = self
            .cdf_context
            .get_txb_skip_cdf_chroma_mut(tx_class, txb_skip_ctx);
        let all_zero = self.decoder.read_symbol_adaptive(txb_skip_cdf)? == 1;
        if all_zero {
            return Ok(ResidualBlockStats {
                all_zero: true,
                ..Default::default()
            });
        }

        let eob_bin_cdf = self
            .cdf_context
            .get_eob_bin_cdf_chroma_mut(width_px, height_px);
        let eob_bin = self.decoder.read_symbol_adaptive(eob_bin_cdf)? as u32;

        let eob: u32 = if eob_bin > 1 {
            let eob_hi_bit_cdf = self
                .cdf_context
                .get_eob_hi_bit_cdf_chroma_mut(tx_class, eob_bin as u8);
            let eob_hi_bit = self.decoder.read_symbol_adaptive(eob_hi_bit_cdf)? as u32;
            let num_extra_bits = eob_bin - 2;
            let mut extra = 0u32;
            for _ in 0..num_extra_bits {
                let bit = self.decoder.read_bool(16384)? as u32;
                extra = (extra << 1) | bit;
            }
            ((eob_hi_bit | 2) << (eob_bin - 2)) | extra
        } else {
            eob_bin
        };

        let mut stats = ResidualBlockStats {
            all_zero: false,
            ..Default::default()
        };
        let mut levels = scan::LevelBuffer::new(width_px as usize, height_px as usize);

        // Same raw-`eob`-is-a-last-index (not a count) fix as `read_residual_block` -- see that
        // method's identical comment for the full derivation against rav1d's `decode_coefs`.
        for c in (0..=eob).rev() {
            let (x, y) = scan::coeff_position(width_px, height_px, false, false, c);
            let is_eob_pos = c == eob;

            let (base_level, br_ctx) = if is_eob_pos {
                let ctx = coeff_base_eob_context(eob, width_px.max(height_px));
                let cdf = self
                    .cdf_context
                    .get_coeff_base_eob_cdf_chroma_mut(tx_class, ctx);
                let level = self.decoder.read_symbol_adaptive(cdf)? as u32 + 1;
                let pos_band = (x | y) > 1;
                (level, if pos_band { 14 } else { 7 })
            } else if c == 0 {
                let cdf = self.cdf_context.get_coeff_base_cdf_chroma_mut(tx_class, 0);
                let level = self.decoder.read_symbol_adaptive(cdf)? as u32;
                let (_, hi_mag) = scan::lo_ctx(&levels, 0, 0, false, width_px, height_px);
                let mag = hi_mag & 63;
                (level, (if mag > 12 { 6 } else { (mag + 1) >> 1 }) as u8)
            } else {
                let (ctx, hi_mag) = scan::lo_ctx(&levels, x, y, false, width_px, height_px);
                let cdf = self
                    .cdf_context
                    .get_coeff_base_cdf_chroma_mut(tx_class, ctx);
                let level = self.decoder.read_symbol_adaptive(cdf)? as u32;
                let mag = hi_mag & 63;
                let pos_band = (x | y) > 1;
                let band = if pos_band { 14 } else { 7 };
                (
                    level,
                    band + (if mag > 12 { 6 } else { (mag + 1) >> 1 }) as u8,
                )
            };

            let mut level = base_level;
            let extended = level > 2;
            if extended {
                let coeff_br_cdf = self
                    .cdf_context
                    .get_coeff_br_cdf_chroma_mut(tx_class, br_ctx);
                for _ in 0..4 {
                    let br = self.decoder.read_symbol_adaptive(coeff_br_cdf)? as u32;
                    level += br;
                    if br < 3 {
                        break;
                    }
                }
            }
            levels.set(x, y, extended, level);

            if level > 0 {
                if c == 0 {
                    let dc_sign_cdf = self.cdf_context.get_dc_sign_cdf_chroma_mut(dc_sign_ctx);
                    let sign = self.decoder.read_symbol_adaptive(dc_sign_cdf)?;
                    stats.dc_sign_value = Some(sign);
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

/// `coeff_base_eob`'s context (0..=3): purely a function of `eob` (the raw last-scan-index value,
/// see `read_residual_block`'s doc for why this is `real coefficient count - 1`, not a count) and
/// tx size, no neighbor/level state needed (unlike `coeff_base`/`coeff_br`, still deferred -- see
/// `read_residual_block`'s doc). Source: rav1d's `decode_coefs` (`memorysafety/rav1d`,
/// BSD-2-Clause, `src/recon_tmpl.c`): the single-coefficient case (`eob == 0`) is a distinct
/// "dc-only" branch that reads `eob_cdf[0]` -- context `0` hardcoded, NOT the generic formula
/// (which would otherwise give `1`, since `0` is never `>` either positive threshold) -- only
/// `eob >= 1` goes through `1 + (eob > 2<<tx2dszctx) + (eob > 4<<tx2dszctx)`, where `tx2dszctx` is
/// `2 * min(tx_size_class, 3)` for a square transform (real AV1 caps the 2D coefficient scan's
/// size class at 32x32).
fn coeff_base_eob_context(eob: u32, tx_size_px: u32) -> u8 {
    if eob == 0 {
        return 0;
    }
    let tx2dszctx = 2 * cdf::tx_size_class(tx_size_px).min(3) as u32;
    1 + u8::from(eob > (2 << tx2dszctx)) + u8::from(eob > (4 << tx2dszctx))
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
    /// The raw `dc_sign` bit read for this block's DC coefficient (`Some(1)`=negative,
    /// `Some(0)`=positive), or `None` if no `dc_sign` bit was read at all (`all_zero`, or the DC
    /// position's level decoded to `0`) -- feeds `TileContext::set_residual_ctx`'s
    /// `dc_sign_symbol` param, see its doc for the "neutral" convention this maps to.
    pub dc_sign_value: Option<u8>,
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
    fn decoder_state(d: &SymbolDecoder) -> (u32, usize, i32) {
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

    #[test]
    fn test_transform_type_coded_lossless_reads_zero_bits() {
        let data = vec![0x80, 0x00, 0xFF, 0xFF, 0xAA, 0xBB];
        let mut decoder = SymbolDecoder::new(&data).unwrap();
        let before = decoder_state(&decoder);
        let is_1d = decoder
            .read_transform_type_is_1d(true, true, false, false, 16, 0)
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
            .read_transform_type_is_1d(false, false, true, false, 16, 0)
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
            .read_transform_type_is_1d(false, false, false, false, 64, 0)
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
            .read_transform_type_is_1d(true, false, false, false, 32, 0)
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
            .read_transform_type_is_1d(false, false, false, false, 32, 0)
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
                .read_transform_type_is_1d(true, false, false, true, 4, 0)
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
            .read_transform_type_is_1d(true, false, false, false, 4, 0)
            .unwrap();
        assert_ne!(decoder_state(&decoder), before);
    }

    #[test]
    fn test_coeff_base_eob_context_increases_with_eob() {
        // tx_size_px=16 -> tx_size_class=2 -> tx2dszctx=4 -> thresholds 2<<4=32, 4<<4=64.
        assert_eq!(coeff_base_eob_context(10, 16), 1); // below both thresholds
        assert_eq!(coeff_base_eob_context(40, 16), 2); // above first only
        assert_eq!(coeff_base_eob_context(100, 16), 3); // above both
    }

    #[test]
    fn test_coeff_base_eob_context_caps_tx2dszctx_at_32x32() {
        // 32x32 and 64x64 share the same tx2dszctx (real AV1 caps the 2D coefficient scan's size
        // class at 32x32) -- same context for the same `eob`.
        assert_eq!(
            coeff_base_eob_context(500, 32),
            coeff_base_eob_context(500, 64)
        );
    }

    /// `eob == 0` (rav1d's "dc-only" branch, a single-coefficient transform block) is context `0`,
    /// NOT the generic formula's result (`1`, since `0` is never `>` either positive threshold) --
    /// see this function's doc for the desync this closes.
    #[test]
    fn test_coeff_base_eob_context_dc_only_is_context_zero() {
        assert_eq!(coeff_base_eob_context(0, 16), 0);
        assert_eq!(coeff_base_eob_context(0, 32), 0);
    }
}
