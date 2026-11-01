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

mod block_modes;
mod compound;
mod contexts;
mod delta;
mod interp_filter;
mod intra;
mod intrabc;
mod is_inter;
mod motion_mode;
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
    let TileState { decoder, tile_ctx } = state;
    let FrameCodingParams {
        is_key_frame,
        segmentation,
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

    // Raw intra mode symbol (0..=12), captured for `is_intra` CUs -- only meaningful for
    // `SymbolDecoder::read_transform_type_is_1d`'s `y_mode_raw` param.
    let y_mode_raw = if is_inter {
        block_modes::read_inter_block_info(decoder, tile_ctx, rect, mi, frame, &mut cu)?;
        0
    } else {
        block_modes::read_intra_block_info(decoder, tile_ctx, sb, rect, mi, frame, &mut cu)?
    };

    block_modes::record_neighbour_contexts(tile_ctx, &cu, rect, mi, is_inter);

    // Read residual() for every transform block tiling this CU -- required for correct bitstream
    // alignment whenever skip == false, not just for producing residual statistics. See this
    // module's doc and `SymbolDecoder::read_residual_block`'s doc.
    if !cu.skip {
        residual::read_residual(decoder, tile_ctx, rect, mi, frame, &mut cu, y_mode_raw)?;
    } else {
        cu.residual = None;
        block_modes::clear_residual_contexts(tile_ctx, mi, frame);
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
