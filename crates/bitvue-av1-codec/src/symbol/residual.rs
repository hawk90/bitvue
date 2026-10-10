//! Residual coefficient reading: `all_zero` (`txb_skip`), `eob`, coefficient tokens, signs and
//! Golomb tails for luma and chroma transform blocks (spec 5.11.39 `coeffs()`).

use super::*;

impl<'a> SymbolDecoder<'a> {
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
    pub fn read_txb_skip(
        &mut self,
        tx_width_px: u32,
        tx_height_px: u32,
        txb_skip_ctx: u8,
    ) -> Result<bool> {
        let cdf = self
            .cdf_context
            .get_txb_skip_cdf_mut(tx_size_ctx(tx_width_px, tx_height_px), txb_skip_ctx);
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
        self.read_coefficients(false, width_px, height_px, class, dc_sign_ctx)
    }

    /// dav1d's `decode_coefs` after the `all_zero` flag and the transform type: `eob`, the
    /// coefficient tokens, then signs and Golomb tails. Shared by luma and chroma; they differ
    /// only in which CDF set they use (`chroma`).
    fn read_coefficients(
        &mut self,
        chroma: bool,
        width_px: u32,
        height_px: u32,
        class: TxClass1d,
        dc_sign_ctx: u8,
    ) -> Result<ResidualBlockStats> {
        // Spec `txSzCtx` (dav1d `t_dim->ctx`): the CDF family is chosen by the average of the
        // smaller and larger side's size class, rounded up -- the larger side alone for a square
        // or 2:1 transform, one class lower for a 4:1 one (4x16, 16x64, ...).
        let tx_class = tx_size_ctx(width_px, height_px);
        let is_1d = class.is_1d();

        // Each axis is independently capped at 32 (real AV1 never scans/contexts past the
        // top-left 32-sample extent on either axis, even for a transform with a 64-sample side --
        // see `scan::scan_table`'s doc).
        let capped_width = width_px.min(32);
        let capped_height = height_px.min(32);
        let capped_class = tx_class.min(3);

        let eob_bin_cdf = if chroma {
            self.cdf_context
                .get_eob_bin_cdf_chroma_mut(capped_width, capped_height, is_1d)
        } else {
            self.cdf_context
                .get_eob_bin_cdf_mut(capped_width, capped_height, is_1d)
        };
        let eob_bin = self.decoder.read_symbol_adaptive(eob_bin_cdf)? as u32;

        let eob: u32 = if eob_bin > 1 {
            let eob_hi_bit_cdf = if chroma {
                self.cdf_context
                    .get_eob_hi_bit_cdf_chroma_mut(tx_class, eob_bin as u8)
            } else {
                self.cdf_context
                    .get_eob_hi_bit_cdf_mut(tx_class, eob_bin as u8)
            };
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
        let mut nonzero: Vec<(u32, u32)> = Vec::new();

        // `eob` (as computed above) is the raw last-scan-index value (rav1d's own local `eob`
        // convention), NOT a coefficient count: there are `eob + 1` positions to read (index
        // `eob` itself down through `0`), with the top position (`c == eob`) using
        // `coeff_base_eob`'s CDF and everything below it using ordinary `coeff_base` -- dav1d's
        // "dc-only" branch (raw `eob == 0`) still reads exactly one `coeff_base_eob`-family symbol.
        for c in (0..=eob).rev() {
            let (x, y) =
                scan::coeff_position(capped_width, capped_height, is_1d, class.is_vertical(), c);
            let is_eob_pos = c == eob;

            // `br_ctx`: `coeff_br`'s context if this position's token turns out to need
            // extending (`base_level > 2`) -- computed alongside `base_level` since both draw
            // from the same neighbor lookup (`lo_ctx`'s `hi_mag` output), matching dav1d's
            // `get_lo_ctx` call site producing both values together.
            let (base_level, br_ctx) = if is_eob_pos {
                let ctx = coeff_base_eob_context(eob, width_px, height_px);
                let cdf = if chroma {
                    self.cdf_context
                        .get_coeff_base_eob_cdf_chroma_mut(tx_class, ctx)
                } else {
                    self.cdf_context.get_coeff_base_eob_cdf_mut(tx_class, ctx)
                };
                let level = self.decoder.read_symbol_adaptive(cdf)? as u32 + 1;
                let pos_band = if is_1d { y > 0 } else { (x | y) > 1 };
                // dav1d's dc-only branch (`eob == 0`) decodes its high token with `hi_cdf[0]`.
                let br_ctx = if eob == 0 {
                    0
                } else if pos_band {
                    14
                } else {
                    7
                };
                (level, br_ctx)
            } else if c == 0 {
                // DC position: 2D's `coeff_base` context is hardcoded to `0`, but `coeff_br`'s
                // magnitude still comes from the same 3-neighbor sum `lo_ctx` would produce --
                // call it regardless of `is_1d` and only override the `coeff_base` context choice,
                // matching dav1d's manual-recompute-for-2D special case.
                let (lo_ctx_val, hi_mag) = scan::lo_ctx(&levels, 0, 0, is_1d, width_px, height_px);
                let base_ctx = if is_1d { lo_ctx_val } else { 0 };
                let cdf = if chroma {
                    self.cdf_context
                        .get_coeff_base_cdf_chroma_mut(tx_class, base_ctx)
                } else {
                    self.cdf_context.get_coeff_base_cdf_mut(tx_class, base_ctx)
                };
                let level = self.decoder.read_symbol_adaptive(cdf)? as u32;
                let mag = hi_mag & 63;
                // DC's `coeff_br` context has no position-band offset (the lowest band).
                (level, (if mag > 12 { 6 } else { (mag + 1) >> 1 }) as u8)
            } else {
                let (ctx, hi_mag) = scan::lo_ctx(&levels, x, y, is_1d, width_px, height_px);
                let cdf = if chroma {
                    self.cdf_context
                        .get_coeff_base_cdf_chroma_mut(tx_class, ctx)
                } else {
                    self.cdf_context.get_coeff_base_cdf_mut(tx_class, ctx)
                };
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
                let coeff_br_cdf = if chroma {
                    self.cdf_context
                        .get_coeff_br_cdf_chroma_mut(capped_class, br_ctx)
                } else {
                    self.cdf_context.get_coeff_br_cdf_mut(capped_class, br_ctx)
                };
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
                nonzero.push((c, level));
            }
        }

        // Second pass (dav1d `decode_coefs`, "residual and sign"): after every token has been
        // read, the DC coefficient's sign and Golomb tail, then each non-zero AC coefficient's
        // sign and Golomb tail from the lowest scan position up. Signs and tails never feed back
        // into the token contexts. `nonzero` is in descending scan order, so a DC coefficient
        // (scan position 0) is last.
        if let Some(&(0, level)) = nonzero.last() {
            let dc_sign_cdf = if chroma {
                self.cdf_context.get_dc_sign_cdf_chroma_mut(dc_sign_ctx)
            } else {
                self.cdf_context.get_dc_sign_cdf_mut(dc_sign_ctx)
            };
            stats.dc_sign_value = Some(self.decoder.read_symbol_adaptive(dc_sign_cdf)?);
            let level = self.read_golomb_tail(level)?;
            stats.add_level(level);
        }
        // dav1d walks a linked list built while reading tokens from the end of the block back to
        // the start, so the list runs from the LOWEST scan position up to the end-of-block one.
        for &(c, level) in nonzero.iter().rev() {
            if c == 0 {
                continue;
            }
            self.decoder.read_bool(16384)?;
            let level = self.read_golomb_tail(level)?;
            stats.add_level(level);
        }

        Ok(stats)
    }

    /// A coefficient token of 15 is followed by an Exp-Golomb tail (dav1d `read_golomb`): count
    /// zero bits up to a one (at most 32 of them), then read that many more bits; the value is
    /// `15 + (1 << len | bits) - 1`. Any other token is its own level.
    fn read_golomb_tail(&mut self, level: u32) -> Result<u32> {
        if level < 15 {
            return Ok(level);
        }
        let mut len = 0u32;
        while !self.decoder.read_bool(16384)? && len < 32 {
            len += 1;
        }
        let mut val = 1u32;
        for _ in 0..len {
            val = (val << 1).wrapping_add(self.decoder.read_bool(16384)? as u32);
        }
        Ok(val.wrapping_sub(1).wrapping_add(15))
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
        class: TxClass1d,
    ) -> Result<ResidualBlockStats> {
        // Spec `txSzCtx` (see `tx_size_ctx`): not the larger side's class for a 4:1 tile such as
        // the 4x16 chroma block of an 8x32 luma block.
        let tx_class = tx_size_ctx(width_px, height_px).min(3);

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

        // The chroma transform type is not coded: intra blocks map it from the chroma mode (always
        // 2D), inter blocks inherit the luma type -- `LumaTxType::chroma_class`.
        self.read_coefficients(true, width_px, height_px, class, dc_sign_ctx)
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
fn coeff_base_eob_context(eob: u32, tx_width_px: u32, tx_height_px: u32) -> u8 {
    if eob == 0 {
        return 0;
    }
    // dav1d: `tx2dszctx = min(lw, TX_32X32) + min(lh, TX_32X32)` (log2 of px / 4 per side).
    let tx2dszctx =
        (cdf::tx_size_class(tx_width_px).min(3) + cdf::tx_size_class(tx_height_px).min(3)) as u32;
    1 + u8::from(eob > (2 << tx2dszctx)) + u8::from(eob > (4 << tx2dszctx))
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

impl ResidualBlockStats {
    /// Accounts for one non-zero coefficient of absolute value `level`.
    fn add_level(&mut self, level: u32) {
        self.nonzero_count += 1;
        self.sum_abs_level += u64::from(level);
        self.max_level = self.max_level.max(level.min(u32::from(u16::MAX)) as u16);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coeff_base_eob_context_increases_with_eob() {
        // tx_size_px=16 -> tx_size_class=2 -> tx2dszctx=4 -> thresholds 2<<4=32, 4<<4=64.
        assert_eq!(coeff_base_eob_context(10, 16, 16), 1); // below both thresholds
        assert_eq!(coeff_base_eob_context(40, 16, 16), 2); // above first only
        assert_eq!(coeff_base_eob_context(100, 16, 16), 3); // above both
    }

    /// dav1d: `tx2dszctx = min(lw, 3) + min(lh, 3)` -- the sum of the two sides' size classes, not
    /// twice the larger one. A 4x8 transform has `tx2dszctx = 0 + 1 = 1` (thresholds 4 and 8), where
    /// a square 8x8 has 2.
    #[test]
    fn test_coeff_base_eob_context_uses_the_sum_of_both_sides() {
        assert_eq!(coeff_base_eob_context(5, 4, 8), 2); // 4 < 5 <= 8
        assert_eq!(coeff_base_eob_context(9, 4, 8), 3); // 9 > 8
        assert_eq!(coeff_base_eob_context(5, 8, 8), 1); // 8x8: thresholds 8 and 16
    }

    #[test]
    fn test_coeff_base_eob_context_caps_tx2dszctx_at_32x32() {
        // 32x32 and 64x64 share the same tx2dszctx (real AV1 caps the 2D coefficient scan's size
        // class at 32x32) -- same context for the same `eob`.
        assert_eq!(
            coeff_base_eob_context(500, 32, 32),
            coeff_base_eob_context(500, 64, 64)
        );
    }

    /// `eob == 0` (rav1d's "dc-only" branch, a single-coefficient transform block) is context `0`,
    /// NOT the generic formula's result (`1`, since `0` is never `>` either positive threshold) --
    /// see this function's doc for the desync this closes.
    #[test]
    fn test_coeff_base_eob_context_dc_only_is_context_zero() {
        assert_eq!(coeff_base_eob_context(0, 16, 16), 0);
        assert_eq!(coeff_base_eob_context(0, 32, 32), 0);
    }
}
