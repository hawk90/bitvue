//! Intra prediction mode reading: key-frame and inter-frame luma modes (spec 5.11.10, 5.11.7),
//! chroma mode, angle deltas, CfL alphas (spec 5.11.45) and filter intra.

use super::*;

impl<'a> SymbolDecoder<'a> {
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
}
