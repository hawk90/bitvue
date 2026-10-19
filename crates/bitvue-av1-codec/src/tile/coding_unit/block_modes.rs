//! The block-level steps of `parse_coding_unit` (spec 5.11.5) that depend on whether the block is
//! intra, IntraBC or inter, and the neighbour-context bookkeeping that follows the mode info.
//! Split out of `parse_coding_unit`; every statement keeps its original order.

use super::types::{CodingUnit, RefFrame};
use super::{compound, contexts, interp_filter, intra, intrabc, ref_frames, single_ref, var_tx};
use crate::symbol::cdf::tx_size_class;
use crate::symbol::SymbolDecoder;
use crate::tile::{
    BlockRect, FrameCodingParams, MiRect, MotionVector, MvPredictorContext, SuperblockCtx,
    TileContext,
};
use bitvue_engine::Result;

/// A block that is not inter: a key-frame or intra-in-inter-frame block, or an IntraBC block.
/// Returns the raw intra `y_mode` symbol (0 for IntraBC), which the transform-type read needs.
pub(super) fn read_intra_block_info(
    decoder: &mut SymbolDecoder,
    tile_ctx: &mut TileContext,
    sb: &SuperblockCtx,
    rect: BlockRect,
    mi: MiRect,
    frame: &FrameCodingParams,
    cu: &mut CodingUnit,
) -> Result<u8> {
    let BlockRect {
        x,
        y,
        width,
        height,
    } = rect;
    let FrameCodingParams {
        is_key_frame,
        allow_intrabc,
        tx_type_flags,
        mi_rows,
        mi_cols,
        ..
    } = *frame;
    let mut y_mode_raw: u8 = 0;
    // Real INTRA CU -- either a key-frame CU (the only case before 2026-08-13) or a genuine
    // intra-coded CU within an inter frame (new). Everything below (`y_mode` through the
    // real per-pixel palette-token read and `tx_size()`) is the SAME unified code path real
    // dav1d uses for both cases -- see `SymbolDecoder::read_intra_mode_inter_frame`'s doc for
    // the one real difference (which CDF/context source `y_mode` draws from).
    cu.ref_frames = [RefFrame::Intra, RefFrame::Intra];
    if !is_key_frame || allow_intrabc {
        tile_ctx.set_spatial_ref_intra_block(mi.x4, mi.y4, mi.width, mi.height);
    }

    // use_intrabc (spec 5.11.6) -- rare (screen-content-coding), only read at all when the
    // frame header allows it. Real spec: `allow_intrabc` is only ever true for an intra-only
    // frame's own header (never for a genuine inter frame), so gating on `is_key_frame` here
    // too is redundant with a well-formed `allow_intrabc` but kept explicit rather than
    // assumed.
    cu.use_intrabc = if is_key_frame && allow_intrabc {
        decoder.read_use_intrabc()?
    } else {
        false
    };

    // tx_size() (spec 5.11.15/16) -- real per-context CDF + adaptation, see
    // `SymbolDecoder::read_tx_size`'s doc. IntraBC is excluded from *this* single-size read:
    // real spec's `read_block_tx_size()` gates the recursive `read_var_tx_size()` tree on
    // `is_inter`, and dav1d's own block-mode dispatch (`b->intra = !intrabc_flag`, verified
    // directly against `src/decode.c`, not assumed) confirms IntraBC blocks are classified
    // `is_inter` for this purpose despite being coded within an intra frame -- real
    // `read_vartx_tree` is called for them identically to real inter blocks (`compute_inter_
    // tx_blocks`, below), not this heuristic-single-size path. The ENTIRE `b->intra` mode-info
    // tail below (`y_mode` through the real per-pixel palette-token read) is likewise excluded
    // for IntraBC: real dav1d dispatches `b->intra = !intrabc_flag`, so a true `use_intrabc`
    // flag makes `b->intra == 0` and skips this whole block -- verified directly against
    // `src/decode.c`'s `if (b->intra) { ... }` wrapper (2026-08-13, found while implementing
    // palette: this crate previously read `y_mode` here UNCONDITIONALLY, a real desync bug on
    // every IntraBC CU that predates this fix).
    if cu.use_intrabc {
        // Displacement vector (before the transform tree, like an inter block's mode info).
        let has_chroma = contexts::has_chroma(mi, &tx_type_flags);
        cu.mv[0] = intrabc::read_displacement_vector(
            decoder,
            tile_ctx,
            mi,
            mi_cols,
            sb.size4 == 32,
            has_chroma,
            (tx_type_flags.subsampling_x, tx_type_flags.subsampling_y),
        )?;
    }
    if !cu.use_intrabc {
        y_mode_raw = intra::read_intra_mode_info(decoder, tile_ctx, rect, mi, frame, cu)?;
    } else {
        // The displacement vector was read above; the rest of an IntraBC block is parsed
        // like an inter block's transform tree (verified symbol-for-symbol against dav1d on
        // locally encoded aomenc and SVT-AV1 screen-content key frames, scratch-only).
        cu.tx_blocks = var_tx::compute_inter_tx_blocks(
            decoder,
            tile_ctx,
            x,
            y,
            width,
            height,
            cu.skip,
            tx_type_flags.coded_lossless,
            tx_type_flags.txfm_mode,
            mi_rows,
            mi_cols,
        )?;
    }
    Ok(y_mode_raw)
}

/// An inter block: reference frames, prediction mode and motion vectors, interpolation filter and
/// the transform tree.
pub(super) fn read_inter_block_info(
    decoder: &mut SymbolDecoder,
    tile_ctx: &mut TileContext,
    mv_ctx: &mut MvPredictorContext,
    rect: BlockRect,
    mi: MiRect,
    frame: &FrameCodingParams,
    cu: &mut CodingUnit,
) -> Result<()> {
    let BlockRect {
        x,
        y,
        width,
        height,
    } = rect;
    let FrameCodingParams {
        tx_type_flags,
        mi_rows,
        mi_cols,
        ..
    } = *frame;
    // ref_frame() (spec 5.11.25) -- see `ref_frames`.
    cu.ref_frames = ref_frames::read_ref_frames(decoder, tile_ctx, rect, mi, frame, cu)?;
    let is_compound = cu.ref_frames[1] != RefFrame::Intra;

    let is_warp = if is_compound {
        compound::read_compound_mode_info(decoder, tile_ctx, mv_ctx, rect, mi, frame, cu)?;
        false
    } else {
        // INTER frame - read prediction mode
        single_ref::read_single_ref_mode_info(decoder, tile_ctx, mv_ctx, rect, mi, frame, cu)?
    };

    // filter (spec 5.11.30) -- see `interp_filter`.
    interp_filter::read_interp_filter(decoder, tile_ctx, mi, frame, cu, is_warp)?;

    // read_block_tx_size() (spec 5.11.16/17/18) for INTER blocks -- real recursive var-tx
    // read, see `read_var_tx_size`'s doc for the exact scope (square CUs only) and why.
    cu.tx_blocks = var_tx::compute_inter_tx_blocks(
        decoder,
        tile_ctx,
        x,
        y,
        width,
        height,
        cu.skip,
        tx_type_flags.coded_lossless,
        tx_type_flags.txfm_mode,
        mi_rows,
        mi_cols,
    )?;
    Ok(())
}

/// Record this block in the above/left neighbour contexts, only after all of its mode info has
/// been parsed (see the comment on the first write below).
pub(super) fn record_neighbour_contexts(
    tile_ctx: &mut TileContext,
    cu: &CodingUnit,
    rect: BlockRect,
    mi: MiRect,
    is_inter: bool,
) {
    let BlockRect { width, height, .. } = rect;
    let MiRect {
        x4,
        y4,
        width: width_4x4,
        height: height_4x4,
    } = mi;
    // Record this block in the above/left neighbour contexts only now that all of its mode info
    // has been parsed. Mode-info syntax of the block itself (`motion_mode`'s overlappable-neighbour
    // and matching-reference checks, the interpolation-filter and compound-type contexts) must see
    // the *neighbours'* values; writing these any earlier makes them read the block itself.
    // dav1d does the same: its `decode_b` updates the context arrays after parsing the block.
    tile_ctx.set_intra_flag(x4, y4, width_4x4, height_4x4, !is_inter);
    if cu.use_intrabc {
        // Neighbouring key-frame intra blocks see an IntraBC block as DC_PRED (dav1d's
        // `edge->mode = DC_PRED`).
        tile_ctx.set_mode(x4, y4, width_4x4, height_4x4, 0);
        // The vector is a candidate for later IntraBC blocks (dav1d `splat_intrabc_mv`: reference
        // 0 = the current frame, no second reference, not NEWMV).
        tile_ctx.set_spatial_ref_block(
            x4,
            y4,
            width_4x4,
            height_4x4,
            -1,
            -1,
            false,
            cu.mv[0],
            MotionVector::zero(),
        );
    }
    if is_inter || cu.use_intrabc {
        // Inter and IntraBC blocks have no palette; record that for their neighbours.
        tile_ctx.set_pal_size(0, x4, y4, width_4x4, height_4x4, 0);
        tile_ctx.set_pal_size(1, x4, y4, width_4x4, height_4x4, 0);
    }
    if is_inter || cu.use_intrabc {
        // Inter and IntraBC blocks record their size as the "intra transform size" their
        // neighbours' `tx_size` context sees (dav1d's inter `set_ctx`: `tx_intra = b_dim`).
        tile_ctx.set_tx_class(
            x4,
            y4,
            width_4x4,
            height_4x4,
            tx_size_class(width) as u8,
            tx_size_class(height) as u8,
        );
    }
    if is_inter {
        let is_compound = cu.ref_frames[1] != RefFrame::Intra;
        if !is_compound {
            // A single-reference block leaves `COMP_INTER_NONE` for its neighbours' `mask_comp` /
            // `jnt_comp` contexts (dav1d: `b->comp_type = COMP_INTER_NONE`); without this a
            // compound block further up or left would stay visible through it.
            tile_ctx.set_comp_type(x4, y4, width_4x4, height_4x4, 0);
        }
        tile_ctx.set_ref_frames(
            x4,
            y4,
            width_4x4,
            height_4x4,
            false,
            is_compound,
            cu.ref_frames[0] as i8 - 1,
            if is_compound {
                cu.ref_frames[1] as i8 - 1
            } else {
                -1
            },
        );
    } else if !cu.use_intrabc {
        // An intra block in an inter frame leaves "no reference, no compound type, no filter" for
        // its neighbours (dav1d's intra `set_ctx`: `ref = -1`, `comp_type = NONE`, `filter =
        // N_SWITCHABLE_FILTERS`), so a neighbour's `interp_filter`/`comp_mode` context does not
        // pick up whatever an earlier block left in those arrays.
        tile_ctx.set_ref_frames(x4, y4, width_4x4, height_4x4, true, false, -1, -1);
        tile_ctx.set_comp_type(x4, y4, width_4x4, height_4x4, 0);
        for dir in 0..2 {
            tile_ctx.set_filter(x4, y4, width_4x4, height_4x4, dir, 3);
        }
    }
}

/// A skipped block has no coefficients: clear its footprint in the residual contexts.
pub(super) fn clear_residual_contexts(
    tile_ctx: &mut TileContext,
    mi: MiRect,
    frame: &FrameCodingParams,
) {
    let MiRect {
        x4,
        y4,
        width: width_4x4,
        height: height_4x4,
    } = mi;
    // A skipped block has no coefficients: its neighbours' `txb_skip`/`dc_sign` contexts must
    // see "none" over its whole footprint (dav1d: `lcoef`/`ccoef` set to 0x40), not whatever
    // an earlier block left there.
    tile_ctx.set_residual_ctx(x4, y4, width_4x4, height_4x4, 0, None);
    if frame.tx_type_flags.subsampling_x
        && frame.tx_type_flags.subsampling_y
        && contexts::has_chroma(mi, &frame.tx_type_flags)
    {
        for plane in 0..2 {
            tile_ctx.set_residual_ctx_chroma(
                plane,
                x4 >> 1,
                y4 >> 1,
                (width_4x4 + 1) >> 1,
                (height_4x4 + 1) >> 1,
                0,
                None,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(sx: bool, sy: bool, mono: bool) -> FrameCodingParams {
        let mut f = FrameCodingParams::for_tests();
        f.tx_type_flags.subsampling_x = sx;
        f.tx_type_flags.subsampling_y = sy;
        f.tx_type_flags.mono_chrome = mono;
        f
    }

    /// A tile whose luma and chroma residual contexts are dirty over the block at `mi`.
    fn dirty(mi: MiRect) -> TileContext {
        let mut ctx = TileContext::new(32, 32);
        ctx.set_residual_ctx(mi.x4, mi.y4, mi.width, mi.height, 9, Some(1));
        for plane in 0..2 {
            ctx.set_residual_ctx_chroma(
                plane,
                mi.x4 >> 1,
                mi.y4 >> 1,
                (mi.width + 1) >> 1,
                (mi.height + 1) >> 1,
                9,
                Some(1),
            );
        }
        ctx
    }

    /// `txb_skip` context of the block against that of an untouched tile (an all-clear neighbourhood
    /// already maps to a non-zero table entry, so compare rather than test for 0).
    fn luma_dirty(ctx: &TileContext, mi: MiRect) -> bool {
        let at = |c: &TileContext| c.txb_skip_context(mi.x4, mi.y4, mi.width, mi.height, false);
        at(ctx) != at(&TileContext::new(32, 32))
    }

    fn chroma_dirty(ctx: &TileContext, mi: MiRect, plane: usize) -> bool {
        ctx.txb_skip_context_chroma(
            plane,
            mi.x4 >> 1,
            mi.y4 >> 1,
            (mi.width + 1) >> 1,
            (mi.height + 1) >> 1,
            false,
        ) != 0
    }

    #[test]
    fn a_skipped_block_clears_luma_always_and_chroma_only_when_it_owns_420_chroma() {
        let mi = MiRect {
            x4: 4,
            y4: 4,
            width: 4,
            height: 4,
        };
        // (subsampling_x, subsampling_y, mono, chroma cleared?)
        for (sx, sy, mono, chroma_cleared) in [
            (true, true, false, true),
            (true, false, false, false), // 4:2:2: this model keeps no chroma contexts for it
            (false, true, false, false),
            (false, false, false, false),
            (true, true, true, false), // monochrome has no chroma
        ] {
            let mut ctx = dirty(mi);
            assert!(luma_dirty(&ctx, mi) && chroma_dirty(&ctx, mi, 0) && chroma_dirty(&ctx, mi, 1));
            clear_residual_contexts(&mut ctx, mi, &frame(sx, sy, mono));
            let label = format!("sx {sx} sy {sy} mono {mono}");
            assert!(!luma_dirty(&ctx, mi), "{label}: luma is always cleared");
            for plane in 0..2 {
                assert_eq!(
                    !chroma_dirty(&ctx, mi, plane),
                    chroma_cleared,
                    "{label} plane {plane}"
                );
            }
        }
    }

    #[test]
    fn a_narrow_block_that_shares_its_chroma_leaves_the_chroma_contexts_alone() {
        // A 4x4 block at an even position does not own chroma (has_chroma is false).
        let mi = MiRect {
            x4: 4,
            y4: 4,
            width: 1,
            height: 1,
        };
        let mut ctx = dirty(mi);
        clear_residual_contexts(&mut ctx, mi, &frame(true, true, false));
        assert!(!luma_dirty(&ctx, mi));
        assert!(chroma_dirty(&ctx, mi, 0) && chroma_dirty(&ctx, mi, 1));

        // At an odd position it does, and covers the rounded-up chroma footprint.
        let mi = MiRect {
            x4: 5,
            y4: 5,
            width: 1,
            height: 1,
        };
        let mut ctx = dirty(mi);
        clear_residual_contexts(&mut ctx, mi, &frame(true, true, false));
        assert!(!chroma_dirty(&ctx, mi, 0) && !chroma_dirty(&ctx, mi, 1));
    }
}
