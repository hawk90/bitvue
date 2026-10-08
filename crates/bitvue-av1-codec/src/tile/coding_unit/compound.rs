//! Mode info of a compound-prediction inter block (two references): `compound_mode()`, the joint
//! L0/L1 DRL search, per-direction motion vectors, and `compound_type()` (spec 5.11.24-5.11.28).
//! A `skip_mode` block reads none of it -- the whole tail is derived with zero bits.

use super::contexts::{compound_mode_from_symbol, wedge_ctx};
use super::read_explicit_mv;
use super::types::{CodingUnit, MotionVector, MvKind, PredictionMode};
use crate::symbol::SymbolDecoder;
use crate::tile::{BlockRect, FrameCodingParams, MiRect, MvPredictorContext, TileContext};
use bitvue_engine::Result;

/// Fills `cu.mode` and `cu.mv[0..2]`, reads the compound-type symbols, and records the block in
/// the spatial reference context. `cu.ref_frames` must already hold the compound pair.
pub(super) fn read_compound_mode_info(
    decoder: &mut SymbolDecoder,
    tile_ctx: &mut TileContext,
    mv_ctx: &mut MvPredictorContext,
    rect: BlockRect,
    mi: MiRect,
    frame: &FrameCodingParams,
    cu: &mut CodingUnit,
) -> Result<()> {
    let BlockRect { x, y, .. } = rect;
    let MiRect {
        x4,
        y4,
        width: width_4x4,
        height: height_4x4,
    } = mi;
    let FrameCodingParams {
        use_ref_frame_mvs,
        inter_mode_flags,
        ..
    } = *frame;
    let rav1d_ref0 = cu.ref_frames[0] as i8 - 1;
    let rav1d_ref1 = cu.ref_frames[1] as i8 - 1;

    // skip_mode (spec 5.11.5/5.11.24, dav1d `decode.c:1401-1423`) forces the ENTIRE
    // mode-info tail with zero bits read: `inter_mode = NEARESTMV_NEARESTMV`, `drl_idx =
    // NEAREST` (index 0, no DRL bits), `mv[]` straight from `compound_mv_stack`'s
    // `stack[0]`, `comp_type = AVG`. Real spec never reads `compound_mode()`/DRL/MV-
    // residual/`compound_type()` bits for a skip_mode CU at all -- this crate previously
    // (before this branch existed) fell through to the real-read path below even for
    // skip_mode CUs, a real desync (reading bits a real encoder never wrote).
    let comp_type = if cu.skip_mode {
        cu.mode = PredictionMode::NearestNearestMv;
        let (stack, _n_mvs) = tile_ctx.compound_mv_stack(
            x4,
            y4,
            width_4x4,
            height_4x4,
            rav1d_ref0,
            rav1d_ref1,
            use_ref_frame_mvs,
        );
        cu.mv = stack[0].mv;
        2 // AVG, matches dav1d's `b->comp_type = COMP_INTER_AVG`
    } else {
        // compound_mode() (spec 5.11.24) -- a distinct 8-symbol alphabet from the
        // single-ref 4-way `inter_mode`, see `SymbolDecoder::read_compound_mode`'s doc.
        let ctx =
            tile_ctx.compound_mode_context(x4, y4, width_4x4, height_4x4, rav1d_ref0, rav1d_ref1);
        let mode_symbol = decoder.read_compound_mode(ctx)?;
        cu.mode = compound_mode_from_symbol(mode_symbol)?;

        // Real compound DRL (spec 7.10.2.10, joint L0/L1 candidate stack) -- see
        // `SpatialRefContext::compound_mv_stack`'s doc for the real weighted
        // spatial+temporal search this replaces (previously: independent, zero-fallback
        // per-direction lookups via `MvPredictorContext`, and no DRL bits read at all --
        // a real desync for any compound CU whose candidate list has more than 1 entry,
        // not a rare edge case). Real spec's exact 3-way branch (dav1d
        // `decode.c:1502-1532`): `NewNewMv` reads up to 2 bits from `stack[0]`/`[1]`;
        // else if either direction is `Near`, drl starts at index 1 with up to 1 more
        // bit from `stack[1]`; otherwise (`NearestNearestMv`/`GlobalGlobalMv`/any
        // Nearest+New/Nearest+Global/etc. combination not involving `Near`) index 0, no
        // bits at all.
        let (stack, n_mvs) = tile_ctx.compound_mv_stack(
            x4,
            y4,
            width_4x4,
            height_4x4,
            rav1d_ref0,
            rav1d_ref1,
            use_ref_frame_mvs,
        );
        let l0_kind = cu.mode.l0_mv_kind();
        let l1_kind = cu.mode.l1_mv_kind();
        let mut drl_idx = 0usize;
        if l0_kind == Some(MvKind::New) && l1_kind == Some(MvKind::New) {
            if n_mvs > 1 {
                if decoder
                    .read_drl_bit(crate::tile::context::get_compound_drl_context(&stack, 0))?
                {
                    drl_idx += 1;
                }
                if drl_idx == 1
                    && n_mvs > 2
                    && decoder
                        .read_drl_bit(crate::tile::context::get_compound_drl_context(&stack, 1))?
                {
                    drl_idx += 1;
                }
            }
        } else if l0_kind == Some(MvKind::Near) || l1_kind == Some(MvKind::Near) {
            drl_idx = 1;
            if n_mvs > 2
                && decoder
                    .read_drl_bit(crate::tile::context::get_compound_drl_context(&stack, 1))?
            {
                drl_idx += 1;
            }
        }

        // Per-direction value: `Nearest`/`Near` read straight from the real stack;
        // `New` uses `stack[drl_idx]` as predictor with the explicit residual added on
        // top (unchanged shape from the single-direction version this replaces);
        // `Global` keeps today's `mv_ctx`-sourced zero approximation unchanged (real
        // `gm_params` values still aren't stored, same already-documented gap as
        // single-ref `GlobalMv` below).
        cu.mv[0] = match l0_kind {
            Some(MvKind::New) => {
                let explicit_mv = read_explicit_mv(decoder)?;
                explicit_mv.add(stack[drl_idx].mv[0])
            }
            Some(MvKind::Nearest) | Some(MvKind::Near) => stack[drl_idx].mv[0],
            Some(MvKind::Global) => mv_ctx.get_mv_predictor(cu.mode, x, y, cu.ref_frames[0]),
            None => MotionVector::zero(),
        };
        cu.mv[1] = match l1_kind {
            Some(MvKind::New) => {
                let explicit_mv = read_explicit_mv(decoder)?;
                explicit_mv.add(stack[drl_idx].mv[1])
            }
            Some(MvKind::Nearest) | Some(MvKind::Near) => stack[drl_idx].mv[1],
            Some(MvKind::Global) => mv_ctx.get_mv_predictor_l1(cu.mode, x, y, cu.ref_frames[1]),
            None => MotionVector::zero(),
        };

        // compound_type() (spec 5.11.28: jnt_comp vs. segmentation-mask vs. wedge-mask)
        // -- real per-context CDF + adaptation, see `SymbolDecoder::read_mask_comp`'s
        // doc for the desync this closes (previously never read at all for ANY compound
        // block).
        if inter_mode_flags.enable_masked_compound
            && decoder.read_mask_comp(tile_ctx.mask_comp_context(x4, y4))?
        {
            // seg/wedge branch
            if let Some(wctx) = wedge_ctx(width_4x4, height_4x4) {
                let is_wedge = decoder.read_wedge_comp(wctx)?;
                if is_wedge {
                    decoder.read_wedge_idx(wctx)?;
                }
                decoder.read_bool_equi()?; // mask_sign
                if is_wedge {
                    4
                } else {
                    3
                }
            } else {
                decoder.read_bool_equi()?; // mask_sign
                3 // SEG (no wedge eligible at this size)
            }
        } else if inter_mode_flags.enable_jnt_comp {
            let jctx = tile_ctx.jnt_comp_context(x4, y4);
            1 + u8::from(decoder.read_jnt_comp(jctx)?)
        } else {
            2 // AVG
        }
    };

    tile_ctx.set_spatial_ref_block(
        x4,
        y4,
        width_4x4,
        height_4x4,
        rav1d_ref0,
        rav1d_ref1,
        cu.mode.l0_mv_kind() == Some(MvKind::New) || cu.mode.l1_mv_kind() == Some(MvKind::New),
        cu.mv[0],
        cu.mv[1],
    );
    tile_ctx.set_comp_type(x4, y4, width_4x4, height_4x4, comp_type);

    tracing::debug!(
        "Compound mode {:?} at ({}, {}): mv0={:?}, mv1={:?}",
        cu.mode,
        x,
        y,
        cu.mv[0],
        cu.mv[1]
    );
    Ok(())
}
