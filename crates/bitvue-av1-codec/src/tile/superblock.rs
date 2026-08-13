//! Superblock Parsing
//!
//! Per AV1 Specification Section 5.11.5 (Decode Block)
//!
//! Combines partition tree and coding unit parsing to extract
//! complete block-level information including motion vectors.
//!
//! ## Parsing Flow
//!
//! 1. Parse partition tree (recursive block splitting)
//! 2. For each leaf block, parse coding unit
//! 3. Extract motion vectors from INTER blocks
//! 4. Build MVGrid for visualization

use crate::symbol::SymbolDecoder;
use crate::tile::{
    parse_coding_unit, BlockSize, CodingUnit, MotionVector, PartitionNode, PartitionType,
    TxTypeFrameFlags,
};
use bitvue_engine::Result;
use serde::{Deserialize, Serialize};

/// Superblock data (partition tree + coding units)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Superblock {
    /// Superblock position (top-left corner) in pixels
    pub x: u32,
    pub y: u32,
    /// Superblock size in pixels (64 or 128)
    pub size: u32,

    /// Partition tree
    pub partition: PartitionNode,

    /// Coding units (one per leaf block)
    pub coding_units: Vec<CodingUnit>,
}

impl Superblock {
    /// Create new superblock
    pub fn new(x: u32, y: u32, size: u32, partition: PartitionNode) -> Self {
        Self {
            x,
            y,
            size,
            partition,
            coding_units: Vec::new(),
        }
    }

    /// Get all motion vectors from INTER blocks
    pub fn motion_vectors(&self) -> Vec<(u32, u32, u32, u32, MotionVector)> {
        let mut mvs = Vec::new();

        for cu in &self.coding_units {
            if cu.is_inter() {
                tracing::debug!(
                    "INTER CU at ({}, {}) {}x{}, MV: ({}, {})",
                    cu.x,
                    cu.y,
                    cu.width,
                    cu.height,
                    cu.mv[0].x,
                    cu.mv[0].y
                );
                // Include all MVs, even zero (zero MV is valid - means no motion)
                mvs.push((cu.x, cu.y, cu.width, cu.height, cu.mv[0]));
            }
        }

        mvs
    }
}

/// Parse superblock (partition tree + coding units)
///
/// # Arguments
///
/// * `decoder` - Symbol decoder for reading bitstream
/// * `x`, `y` - Superblock position in pixels
/// * `sb_size` - Superblock size (64 or 128)
/// * `is_key_frame` - True if KEY frame (INTRA only)
/// * `current_qp` - Current quantization parameter value
/// * `delta_q_enabled` - True if delta Q is enabled for this frame
/// * `reference_select` - Frame header's `reference_select` flag (see `parse_coding_unit`'s doc)
/// * `allow_intrabc` - Frame header's `allow_intrabc` flag (see `parse_coding_unit`'s doc)
/// * `allow_screen_content_tools` - Frame header's `allow_screen_content_tools` flag (see
///   `parse_coding_unit`'s doc)
/// * `enable_filter_intra` - Sequence header's `enable_filter_intra` flag (see
///   `parse_coding_unit`'s doc)
/// * `use_ref_frame_mvs` - Frame header's `use_ref_frame_mvs` flag, used as `inter_mode`'s
///   `globalmv_ctx` (see `crate::tile::context::SpatialRefContext::inter_mode_context`'s doc)
/// * `tile_ctx` - Above/left neighbor-state tracker for entropy context, shared across every
///   superblock in the tile (see `crate::tile::TileContext`'s doc). Callers looping over
///   superblock rows should call `tile_ctx.start_superblock_row()` at the start of each row.
/// * `tx_type_flags` - Frame header flags for `transform_type()` (see `parse_coding_unit`'s doc)
/// * `mi_rows`, `mi_cols` - Frame extent in AV1 "MI" (4x4) units (`tile::partition::mi_units` of
///   the real frame pixel width/height) -- drives `parse_partition_recursive`'s real `hasRows`/
///   `hasCols` frame-edge partition legality (spec 5.11.4). Callers with a frame smaller than
///   this superblock loop's `sb_cols * sb_size`/`sb_rows * sb_size` extent (i.e. any frame whose
///   dimensions aren't an exact multiple of `sb_size`) need real values here -- passing the
///   superblock-rounded extent instead would make `has_rows`/`has_cols` always true, silently
///   defeating this parameter.
///
/// # Returns
///
/// Parsed superblock with partition tree and coding units, plus final QP value
#[allow(clippy::too_many_arguments)]
pub fn parse_superblock(
    decoder: &mut SymbolDecoder,
    x: u32,
    y: u32,
    sb_size: u32,
    is_key_frame: bool,
    current_qp: i16,
    delta_q_enabled: bool,
    mv_ctx: &mut crate::tile::MvPredictorContext,
    reference_select: bool,
    allow_intrabc: bool,
    allow_screen_content_tools: bool,
    enable_filter_intra: bool,
    delta_lf_present: bool,
    delta_lf_multi: bool,
    use_ref_frame_mvs: bool,
    segmentation: crate::frame_header_full::SegmentationInfo,
    tile_ctx: &mut crate::tile::TileContext,
    tx_type_flags: TxTypeFrameFlags,
    mi_rows: u32,
    mi_cols: u32,
    cdef_bits: u8,
    skip_mode_present: bool,
) -> Result<(Superblock, i16)> {
    // Convert superblock size to BlockSize
    let block_size = match sb_size {
        64 => BlockSize::Block64x64,
        128 => BlockSize::Block128x128,
        _ => BlockSize::Block64x64, // Default
    };

    // `cdef_idx()`'s per-superblock "already read" tracker (spec 5.11.56, see
    // `crate::tile::coding_unit::parse_coding_unit`'s doc) -- reset fresh for every superblock,
    // mirroring dav1d's `cur_sb_cdef_idx_ptr` (`decode_b`'s caller resets it per-superblock too).
    // `-1` (real dav1d sentinel) = "not yet read"; up to 4 slots for `sb128`'s 2x2 grid of 64x64
    // CDEF units (`sb64` only ever touches slot `0`).
    let mut cdef_idx_state: [i8; 4] = [-1; 4];

    // Parse partition tree -- see `tile::partition::parse_partition_recursive`'s doc for why this
    // module no longer keeps its own copy. `None` (superblock origin fully outside the frame)
    // can't happen for a real caller: superblock loops enumerate `sb_x < sb_cols =
    // frame_width.div_ceil(sb_size)`, which guarantees every superblock's pixel origin is `<
    // frame_width`/`frame_height` -- fall back to an empty superblock defensively rather than
    // panic if that invariant is ever violated.
    let partition = crate::tile::partition::parse_partition_recursive(
        decoder, x, y, block_size, mi_rows, mi_cols, 0, // depth
        tile_ctx,
    )?
    .unwrap_or_else(|| PartitionNode::new(x, y, block_size, PartitionType::None));

    // Create superblock
    let mut sb = Superblock::new(x, y, sb_size, partition.clone());

    // Parse coding units for each leaf block
    let final_qp = parse_coding_units_recursive(
        decoder,
        &partition,
        is_key_frame,
        current_qp,
        delta_q_enabled,
        mv_ctx,
        reference_select,
        allow_intrabc,
        allow_screen_content_tools,
        enable_filter_intra,
        delta_lf_present,
        delta_lf_multi,
        use_ref_frame_mvs,
        segmentation,
        tile_ctx,
        tx_type_flags,
        mi_rows,
        mi_cols,
        x / 4,
        y / 4,
        sb_size / 4,
        cdef_bits,
        &mut cdef_idx_state,
        skip_mode_present,
        &mut sb.coding_units,
    )?;

    tracing::debug!(
        "Parsed superblock with {} coding units (QP: {} -> {})",
        sb.coding_units.len(),
        current_qp,
        final_qp
    );
    let inter_count = sb.coding_units.iter().filter(|cu| cu.is_inter()).count();
    let intra_count = sb.coding_units.iter().filter(|cu| !cu.is_inter()).count();
    tracing::debug!("  INTER: {}, INTRA: {}", inter_count, intra_count);

    Ok((sb, final_qp))
}

/// Recursively parse coding units for leaf blocks
#[allow(clippy::too_many_arguments)]
fn parse_coding_units_recursive(
    decoder: &mut SymbolDecoder,
    partition: &PartitionNode,
    is_key_frame: bool,
    current_qp: i16,
    delta_q_enabled: bool,
    mv_ctx: &mut crate::tile::MvPredictorContext,
    reference_select: bool,
    allow_intrabc: bool,
    allow_screen_content_tools: bool,
    enable_filter_intra: bool,
    delta_lf_present: bool,
    delta_lf_multi: bool,
    use_ref_frame_mvs: bool,
    segmentation: crate::frame_header_full::SegmentationInfo,
    tile_ctx: &mut crate::tile::TileContext,
    tx_type_flags: TxTypeFrameFlags,
    mi_rows: u32,
    mi_cols: u32,
    sb_x4: u32,
    sb_y4: u32,
    sb_size4: u32,
    cdef_bits: u8,
    cdef_idx_state: &mut [i8; 4],
    skip_mode_present: bool,
    coding_units: &mut Vec<CodingUnit>,
) -> Result<i16> {
    if partition.is_leaf() {
        // Leaf block - parse coding unit
        let (cu, new_qp) = parse_coding_unit(
            decoder,
            partition.x,
            partition.y,
            partition.size.width(),
            partition.size.height(),
            is_key_frame,
            current_qp,
            delta_q_enabled,
            mv_ctx,
            reference_select,
            allow_intrabc,
            allow_screen_content_tools,
            enable_filter_intra,
            delta_lf_present,
            delta_lf_multi,
            use_ref_frame_mvs,
            segmentation,
            tile_ctx,
            tx_type_flags,
            mi_rows,
            mi_cols,
            sb_x4,
            sb_y4,
            sb_size4,
            cdef_bits,
            cdef_idx_state,
            skip_mode_present,
        )?;

        coding_units.push(cu);
        Ok(new_qp)
    } else {
        // Non-leaf - recurse into children
        let mut qp = current_qp;
        for child in &partition.children {
            qp = parse_coding_units_recursive(
                decoder,
                child,
                is_key_frame,
                qp,
                delta_q_enabled,
                mv_ctx,
                reference_select,
                allow_intrabc,
                allow_screen_content_tools,
                enable_filter_intra,
                delta_lf_present,
                delta_lf_multi,
                use_ref_frame_mvs,
                segmentation,
                tile_ctx,
                tx_type_flags,
                mi_rows,
                mi_cols,
                sb_x4,
                sb_y4,
                sb_size4,
                cdef_bits,
                cdef_idx_state,
                skip_mode_present,
                coding_units,
            )?;
        }
        Ok(qp)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tile::PartitionType;
    use crate::tile::PredictionMode;

    #[test]
    fn test_superblock_new() {
        let partition = PartitionNode::new(0, 0, BlockSize::Block64x64, PartitionType::None);
        let sb = Superblock::new(0, 0, 64, partition);

        assert_eq!(sb.x, 0);
        assert_eq!(sb.y, 0);
        assert_eq!(sb.size, 64);
        assert_eq!(sb.coding_units.len(), 0);
    }

    #[test]
    fn test_superblock_motion_vectors() {
        let partition = PartitionNode::new(0, 0, BlockSize::Block64x64, PartitionType::None);
        let mut sb = Superblock::new(0, 0, 64, partition);

        // Add INTRA block (no MV)
        let mut cu_intra = CodingUnit::new(0, 0, 64, 64);
        cu_intra.mode = PredictionMode::DcPred;
        sb.coding_units.push(cu_intra);

        // Add INTER block with MV
        let mut cu_inter = CodingUnit::new(64, 0, 64, 64);
        cu_inter.mode = PredictionMode::NewMv;
        cu_inter.ref_frames[0] = crate::tile::RefFrame::Last;
        cu_inter.mv[0] = MotionVector::new(16, -8);
        sb.coding_units.push(cu_inter);

        let mvs = sb.motion_vectors();
        assert_eq!(mvs.len(), 1);
        assert_eq!(mvs[0], (64, 0, 64, 64, MotionVector::new(16, -8)));
    }
}
