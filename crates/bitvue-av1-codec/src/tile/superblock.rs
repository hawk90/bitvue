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

use crate::tile::{
    parse_coding_unit, BlockSize, CodingUnit, FrameCodingParams, MotionVector, PartitionNode,
    PartitionType, TileState,
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
/// * `state` - The tile's shared mutable state (symbol decoder, MV predictor context, entropy-
///   context tracker -- see [`TileState`]). Callers looping over superblock rows should call
///   `state.tile_ctx.start_superblock_row()` at the start of each row.
/// * `x`, `y` - Superblock position in pixels
/// * `sb_size` - Superblock size (64 or 128)
/// * `frame` - Frame-level, read-only flags (see [`FrameCodingParams`]); `frame.mi_rows`/
///   `mi_cols` must be the real frame extent, not the superblock-rounded one
/// * `current_qp` - Current quantization parameter value
///
/// # Returns
///
/// Parsed superblock with partition tree and coding units, plus final QP value
pub fn parse_superblock(
    state: &mut TileState<'_>,
    x: u32,
    y: u32,
    sb_size: u32,
    frame: &FrameCodingParams,
    current_qp: i16,
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
        &mut state.decoder,
        x,
        y,
        block_size,
        frame.mi_rows,
        frame.mi_cols,
        0, // depth
        &mut state.tile_ctx,
    )?
    .unwrap_or_else(|| PartitionNode::new(x, y, block_size, PartitionType::None));

    // Create superblock
    let mut sb = Superblock::new(x, y, sb_size, partition.clone());

    // Parse coding units for each leaf block
    let final_qp = parse_coding_units_recursive(
        state,
        &partition,
        frame,
        current_qp,
        x / 4,
        y / 4,
        sb_size / 4,
        &mut cdef_idx_state,
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
    state: &mut TileState<'_>,
    partition: &PartitionNode,
    frame: &FrameCodingParams,
    current_qp: i16,
    sb_x4: u32,
    sb_y4: u32,
    sb_size4: u32,
    cdef_idx_state: &mut [i8; 4],
    coding_units: &mut Vec<CodingUnit>,
) -> Result<i16> {
    if partition.is_leaf() {
        // Leaf block - parse coding unit
        let (cu, new_qp) = parse_coding_unit(
            state,
            partition.x,
            partition.y,
            partition.size.width(),
            partition.size.height(),
            frame,
            current_qp,
            sb_x4,
            sb_y4,
            sb_size4,
            cdef_idx_state,
        )?;

        coding_units.push(cu);
        Ok(new_qp)
    } else {
        // Non-leaf - recurse into children
        let mut qp = current_qp;
        for child in &partition.children {
            qp = parse_coding_units_recursive(
                state,
                child,
                frame,
                qp,
                sb_x4,
                sb_y4,
                sb_size4,
                cdef_idx_state,
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
