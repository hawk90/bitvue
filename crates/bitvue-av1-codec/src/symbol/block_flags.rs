//! Per-block flag and small-alphabet symbol reading: skip, is_inter, motion mode, inter-intra,
//! wedge and compound masks, interpolation filter, DRL bit and segment-id prediction.

use super::*;

impl<'a> SymbolDecoder<'a> {
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
}
