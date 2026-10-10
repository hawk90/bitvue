//! Inter-block mode and reference frame reading (spec 5.11.23-5.11.25: `inter_mode`,
//! `compound_mode`, `ref_frames`).

use super::*;

impl<'a> SymbolDecoder<'a> {
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
}
