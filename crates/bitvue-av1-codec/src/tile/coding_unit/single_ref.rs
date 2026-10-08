//! Mode info of a single-reference inter block: `inter_mode`, DRL + motion vector, `interintra`,
//! and `motion_mode`/OBMC (spec 5.11.23-5.11.29). The compound counterpart lives in `compound`.

use super::contexts::{
    global_motion_forces_simple, has_overlappable_neighbors, inter_mode_from_symbol,
    motion_mode_size_index, wedge_ctx, y_mode_size_context,
};
use super::read_explicit_mv;
use super::types::{CodingUnit, MotionVector, PredictionMode};
use crate::symbol::SymbolDecoder;
use crate::tile::{BlockRect, FrameCodingParams, MiRect, MvPredictorContext, TileContext};
use bitvue_engine::Result;

/// Fills `cu.mode` and `cu.mv`, reads the interintra/motion-mode symbols, and records the block
/// in the spatial reference context. `cu.ref_frames[0]` must already hold the reference.
/// Returns whether the block uses `WARPED_CAUSAL` motion, which skips the interpolation filter.
pub(super) fn read_single_ref_mode_info(
    decoder: &mut SymbolDecoder,
    tile_ctx: &mut TileContext,
    mv_ctx: &mut MvPredictorContext,
    rect: BlockRect,
    mi: MiRect,
    frame: &FrameCodingParams,
    cu: &mut CodingUnit,
) -> Result<bool> {
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
    let rav1d_ref1 = -1i8;

    let ctx =
        tile_ctx.inter_mode_context(x4, y4, width_4x4, height_4x4, rav1d_ref0, use_ref_frame_mvs);
    let mode_symbol = decoder.read_inter_mode(ctx)?;
    cu.mode = inter_mode_from_symbol(mode_symbol)?;

    // DRL (spec 7.10.2.10's real `drl_idx` selection, single-ref only) -- real per-context
    // CDF + adaptation, see `SymbolDecoder::read_drl_bit`'s doc for the desync this closes
    // (this crate previously never read any DRL bits at all, always implicitly using
    // index 0 -- `MvPredictorContext::predict_nearest_mv`'s single-neighbor heuristic).
    // Not read for GLOBALMV (real spec: no DRL for that mode at all).
    if cu.mode == PredictionMode::NewMv {
        let (stack, n_mvs) = tile_ctx.single_ref_mv_stack(
            x4,
            y4,
            width_4x4,
            height_4x4,
            rav1d_ref0,
            use_ref_frame_mvs,
        );
        let mut drl_idx = 0usize;
        if n_mvs > 1 {
            if decoder.read_drl_bit(crate::tile::context::get_drl_context(&stack, 0))? {
                drl_idx += 1;
            }
            if drl_idx == 1
                && n_mvs > 2
                && decoder.read_drl_bit(crate::tile::context::get_drl_context(&stack, 1))?
            {
                drl_idx += 1;
            }
        }
        // The difference is read after the DRL bits (dav1d: `Post-intermode` precedes
        // `Post-residualmv`).
        let explicit_mv = read_explicit_mv(decoder, inter_mode_flags.mv_precision())?;
        let predictor = stack[drl_idx].mv;
        cu.mv[0] = MotionVector::new(explicit_mv.x + predictor.x, explicit_mv.y + predictor.y);
        cu.mv[1] = MotionVector::zero();

        tracing::debug!(
            "NEWMV at ({}, {}): explicit=({:?}), predictor=({:?}), final=({:?})",
            x,
            y,
            explicit_mv,
            predictor,
            cu.mv[0]
        );
    } else if cu.mode == PredictionMode::GlobalMv {
        let predictor = mv_ctx.get_mv_predictor(cu.mode, x, y, cu.ref_frames[0]);
        cu.mv = [predictor, MotionVector::zero()];

        tracing::debug!(
            "Mode {:?} at ({}, {}): using predictor {:?}",
            cu.mode,
            x,
            y,
            cu.mv[0]
        );
    } else {
        // NEARESTMV / NEARMV
        let (stack, n_mvs) = tile_ctx.single_ref_mv_stack(
            x4,
            y4,
            width_4x4,
            height_4x4,
            rav1d_ref0,
            use_ref_frame_mvs,
        );
        let mut drl_idx = if cu.mode == PredictionMode::NearMv {
            1usize
        } else {
            0
        };
        if cu.mode == PredictionMode::NearMv && n_mvs > 2 {
            if decoder.read_drl_bit(crate::tile::context::get_drl_context(&stack, 1))? {
                drl_idx += 1;
            }
            if drl_idx == 2
                && n_mvs > 3
                && decoder.read_drl_bit(crate::tile::context::get_drl_context(&stack, 2))?
            {
                drl_idx += 1;
            }
        }
        cu.mv = [stack[drl_idx].mv, MotionVector::zero()];

        tracing::debug!(
            "Mode {:?} at ({}, {}): drl_idx={} mv={:?}",
            cu.mode,
            x,
            y,
            drl_idx,
            cu.mv[0]
        );
    }

    // interintra (spec 5.11.29) -- real per-context CDF + adaptation, see
    // `SymbolDecoder::read_interintra`'s doc for the desync this closes (previously never
    // read at all for any single-ref inter block).
    let ii_sz_grp = y_mode_size_context(width_4x4, height_4x4);
    let interintra_wedge_ctx = wedge_ctx(width_4x4, height_4x4).filter(|&c| c <= 6);
    let is_interintra = inter_mode_flags.enable_interintra_compound
        && interintra_wedge_ctx.is_some()
        && decoder.read_interintra(ii_sz_grp)?;
    // dav1d stores an inter-intra block's second reference as "intra" (0), which no neighbour
    // scan matches -- not as "none" (-1) -- so it is not a plain single-reference neighbour
    // (`find_matching_ref` requires `ref[1] == -1`). `-2` is that marker here.
    tile_ctx.set_spatial_ref_block(
        x4,
        y4,
        width_4x4,
        height_4x4,
        rav1d_ref0,
        if is_interintra { -2 } else { rav1d_ref1 },
        cu.mode == PredictionMode::NewMv,
        cu.mv[0],
        cu.mv[1],
    );
    if is_interintra {
        decoder.read_interintra_mode(ii_sz_grp)?;
        // `interintra_wedge_ctx` is real here (`is_interintra` only true when `Some`).
        let wctx = interintra_wedge_ctx.unwrap_or(0);
        if decoder.read_interintra_wedge(wctx)? {
            decoder.read_wedge_idx(wctx)?;
        }
    }

    // motion_mode (spec 5.11.27) -- real per-exact-block-size CDF + adaptation, see
    // `SymbolDecoder::read_motion_mode`'s doc for the desync this closes (previously
    // never read at all). Real spec gate: switchable, not interintra, both dims >= 8px,
    // not an excluded warped-global-motion case (real: a `GLOBALMV` block -- the only
    // global-motion mode reachable in this single-ref branch, `GLOBAL_GLOBALMV` being
    // compound-only -- whose `GmType[RefFrame[0]]` is more complex than TRANSLATION reads
    // zero bits here, matching real spec's forced `motion_mode = SIMPLE`; `!force_integer_
    // mv` gates the whole check per spec, same as `read_motion_mode`'s other real gates),
    // and has a real overlappable (non-intra) above/left neighbor.
    let gm_forces_simple = global_motion_forces_simple(
        cu.mode,
        inter_mode_flags.force_integer_mv,
        &inter_mode_flags.gm_type,
        cu.ref_frames[0],
    );
    let mut is_warp = false;
    if inter_mode_flags.switchable_motion_mode
        && !is_interintra
        && !gm_forces_simple
        && width_4x4 >= 2
        && height_4x4 >= 2
        && has_overlappable_neighbors(tile_ctx, x4, y4, width_4x4, height_4x4)
    {
        // `allow_warp`: real spec also requires a real matching-single-reference above/
        // left neighbor (`find_matching_ref`) -- approximated via
        // `has_matching_single_ref` (single-position check, not the full multi-neighbor
        // edge scan real dav1d does -- `TileContext::has_matching_single_ref`'s doc for
        // why a full port is deferred). SVC reference scaling isn't modeled (assumed
        // never scaled, matching this crate's existing no-SVC-support scope).
        let allow_warp = inter_mode_flags.allow_warped_motion
            && tile_ctx.has_matching_edge_ref(
                x4,
                y4,
                width_4x4,
                height_4x4,
                width_4x4.min(frame.mi_cols.saturating_sub(x4)),
                height_4x4.min(frame.mi_rows.saturating_sub(y4)),
                frame.mi_cols,
                rav1d_ref0,
            );
        if allow_warp {
            if let Some(idx) = motion_mode_size_index(width_4x4, height_4x4) {
                is_warp = decoder.read_motion_mode(idx)? == 2;
            }
        } else if let Some(idx) = motion_mode_size_index(width_4x4, height_4x4) {
            decoder.read_obmc(idx)?;
        }
    }
    Ok(is_warp)
}
