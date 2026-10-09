//! Coding Unit Parsing
//!
//! Per AV1 Specification Section 5.11 (Coding Block Syntax)
//!
//! A Coding Unit contains:
//! - Prediction information (INTRA/INTER mode)
//! - Motion vectors (for INTER blocks)
//! - Transform information
//! - Quantization parameters
//! - Residual data
//!
//! ## Implementation Strategy
//!
//! **Phase 1** (Current):
//! - Parse skip flag
//! - Parse prediction mode
//! - Parse reference frames (for INTER)
//!
//! **Phase 2**:
//! - Parse motion vectors
//! - Calculate MV predictors
//! - Reconstruct final MVs
//!
//! **Phase 3**:
//! - Parse transform sizes
//! - Parse quantization info
//! - Parse residuals (optional for visualization)
//!
//! ## Residual reading is not optional -- it was a real desync bug, not a scope choice
//!
//! `parse_coding_unit` used to return immediately after `delta_q`, never reading the AV1 spec's
//! `residual()` syntax element for non-skip coding units. Because a tile's coefficient data is
//! arithmetic-coded with no byte-aligned skip points, this silently desynced the shared
//! `SymbolDecoder`'s position from every subsequent syntax element in the tile as soon as any CU
//! had `skip == false` -- confirmed via a real crash (`get_codec_extended_info` on frame 100 of
//! the real fixture panicked in `ArithmeticDecoder::refill` once the drift exhausted the tile's
//! real bytes). `SymbolDecoder::read_residual_block` (see its own doc for the CDF/context
//! simplifications) now reads a real (if non-spec-exact) residual read sequence for every
//! non-skip CU's transform blocks, closing the gap that caused this.

mod compound;
mod contexts;
mod delta;
mod interp_filter;
mod intra;
mod intrabc;
mod is_inter;
mod palette;
mod ref_frames;
mod residual;
mod segment;
mod single_ref;
mod skip;
mod tx_size;
mod types;
mod var_tx;

pub use palette::PaletteInfo;
pub use types::*;

use crate::tile::{BlockRect, FrameCodingParams, MiRect, SuperblockCtx, TileState};

use crate::symbol::cdf::tx_size_class;
use crate::symbol::SymbolDecoder;
use bitvue_engine::Result;

/// Parse coding unit from symbol decoder
///
/// Reads block-level syntax elements from the bitstream.
///
/// # Arguments
///
/// * `state` - The tile's shared mutable state: symbol decoder, MV predictor context and the
///   above/left entropy-context tracker (currently only `skip` uses it) -- see [`TileState`]
/// * `sb` - The enclosing superblock's origin/size and `cdef_idx()` tracker (see
///   [`SuperblockCtx`]); real spec's `delta_q`/`delta_lf` are read only once per superblock
/// * `rect` - Block position and dimensions in pixels
/// * `frame` - Frame-level, read-only flags (see [`FrameCodingParams`] for what each gates)
/// * `current_qp` - Current quantization parameter value
///
/// # Returns
///
/// Parsed coding unit with prediction info, motion vectors (if INTER), and QP value
pub fn parse_coding_unit(
    state: &mut TileState<'_>,
    sb: &mut SuperblockCtx,
    rect: BlockRect,
    frame: &FrameCodingParams,
    current_qp: i16,
) -> Result<(CodingUnit, i16)> {
    let BlockRect {
        x,
        y,
        width,
        height,
    } = rect;
    let TileState {
        decoder,
        mv_ctx,
        tile_ctx,
    } = state;
    let FrameCodingParams {
        is_key_frame,
        allow_intrabc,
        segmentation,
        tx_type_flags,
        mi_rows,
        mi_cols,
        cdef_bits,
        skip_mode_present,
        ..
    } = *frame;
    let mut cu = CodingUnit::new(x, y, width, height);
    let (x4, y4) = (x / 4, y / 4);
    let (width_4x4, height_4x4) = (width.div_ceil(4).max(1), height.div_ceil(4).max(1));
    let mi = MiRect {
        x4,
        y4,
        width: width_4x4,
        height: height_4x4,
    };

    // segment_id(), pre-skip position (spec 5.11.9/5.11.10) -- see `segment::read_pre_skip`.
    if let Some(id) = segment::read_pre_skip(decoder, tile_ctx, mi, segmentation)? {
        cu.segment_id = id;
    }

    // skip_mode then skip (spec 5.11.5) -- see `skip`.
    cu.skip_mode = skip::read_skip_mode(decoder, tile_ctx, mi, !is_key_frame && skip_mode_present)?;
    cu.skip = skip::read_skip(
        decoder,
        tile_ctx,
        mi,
        cu.skip_mode,
        segmentation,
        cu.segment_id,
    )?;

    // segment_id(), post-skip position -- see `segment::read_post_skip`.
    if let Some(id) = segment::read_post_skip(decoder, tile_ctx, mi, segmentation, cu.skip)? {
        cu.segment_id = id;
    }

    // cdef_idx() then delta_q/delta_lf (spec 5.11.56, 5.11.38) -- see `delta`.
    delta::read_cdef_idx(decoder, sb, mi, cu.skip, cdef_bits)?;
    let new_qp = delta::read_delta_q_lf(decoder, sb, mi, cu.skip, frame, current_qp);
    cu.qp = Some(new_qp);

    // Raw intra mode symbol (0..=12), captured below for `is_intra` CUs -- only meaningful for
    // `SymbolDecoder::read_transform_type_is_1d`'s `y_mode_raw` param.
    let mut y_mode_raw: u8 = 0;

    // is_inter (spec 5.11.5) -- see `is_inter`.
    let is_inter = is_inter::read_is_inter(
        decoder,
        tile_ctx,
        mi,
        is_key_frame,
        cu.skip_mode,
        segmentation,
        cu.segment_id,
    )?;

    // Determine if INTRA or INTER
    if !is_inter {
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
            y_mode_raw = intra::read_intra_mode_info(decoder, tile_ctx, rect, mi, frame, &mut cu)?;
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
    } else {
        // ref_frame() (spec 5.11.25) -- see `ref_frames`.
        cu.ref_frames = ref_frames::read_ref_frames(decoder, tile_ctx, rect, mi, frame, &cu)?;
        let is_compound = cu.ref_frames[1] != RefFrame::Intra;

        let is_warp = if is_compound {
            compound::read_compound_mode_info(decoder, tile_ctx, mv_ctx, rect, mi, frame, &mut cu)?;
            false
        } else {
            // INTER frame - read prediction mode
            single_ref::read_single_ref_mode_info(
                decoder, tile_ctx, mv_ctx, rect, mi, frame, &mut cu,
            )?
        };

        // filter (spec 5.11.30) -- see `interp_filter`.
        interp_filter::read_interp_filter(decoder, tile_ctx, mi, frame, &cu, is_warp)?;

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
    }

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

    // Add this CU to the MV predictor context for future blocks
    // Now uses zero-copy reference instead of cloning the entire CU
    mv_ctx.add_cu(&cu);

    // Read residual() for every transform block tiling this CU -- required for correct bitstream
    // alignment whenever skip == false, not just for producing residual statistics. See this
    // module's doc and `SymbolDecoder::read_residual_block`'s doc.
    if !cu.skip {
        residual::read_residual(decoder, tile_ctx, rect, mi, frame, &mut cu, y_mode_raw)?;
    } else {
        cu.residual = None;
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

    Ok((cu, new_qp))
}

/// Read one explicit MV delta (horizontal + vertical component) from the bitstream, per AV1 spec
/// 5.11.32 `read_mv(ref)`. Used for every `MvKind::New` reference-list slot -- single-ref
/// `NewMv`'s L0, and compound modes' L0 and/or L1 (spec 5.11.26 `assign_mv()`).
///
/// The `mv_joint` symbol gates which axis actually has a coded component -- an axis mv_joint
/// marks "zero" is NOT read from the bitstream at all (it's implicitly 0), it doesn't just
/// happen to decode to a small value. The previous implementation unconditionally read both
/// components for every MV, which desynced the shared `SymbolDecoder` against any real
/// bitstream whenever mv_joint indicated a zero axis -- the same "syntax element not read at
/// all" pattern as this session's earlier residual()/ref_frame() bugs, just not crash-visible
/// here since a `SymbolDecoder` never panics on merely-wrong-but-in-range values.
fn read_explicit_mv(decoder: &mut SymbolDecoder, mv_prec: i8) -> Result<MotionVector> {
    let joint = decoder.read_mv_joint()?;
    // MV_JOINT_HZVNZ(2)/MV_JOINT_HNZVNZ(3): the vertical component is coded, and comes first.
    let mv_y = if matches!(joint, 2 | 3) {
        decoder.read_mv_component_diff(0, mv_prec)?
    } else {
        0
    };
    // MV_JOINT_HNZVZ(1)/MV_JOINT_HNZVNZ(3): the horizontal component is coded.
    let mv_x = if matches!(joint, 1 | 3) {
        decoder.read_mv_component_diff(1, mv_prec)?
    } else {
        0
    };
    Ok(MotionVector::new(mv_x, mv_y))
}
