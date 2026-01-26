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
};
use bitvue_core::Result;
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
///
/// # Returns
///
/// Parsed superblock with partition tree and coding units, plus final QP value
pub fn parse_superblock(
    decoder: &mut SymbolDecoder,
    x: u32,
    y: u32,
    sb_size: u32,
    is_key_frame: bool,
    current_qp: i16,
    delta_q_enabled: bool,
    mv_ctx: &mut crate::tile::MvPredictorContext,
) -> Result<(Superblock, i16)> {
    // Convert superblock size to BlockSize
    let block_size = match sb_size {
        64 => BlockSize::Block64x64,
        128 => BlockSize::Block128x128,
        _ => BlockSize::Block64x64, // Default
    };

    // Parse partition tree
    let partition = parse_partition_recursive(
        decoder, x, y, block_size, true, // has_rows
        true, // has_cols
    )?;

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

/// Recursively parse partition tree
fn parse_partition_recursive(
    decoder: &mut SymbolDecoder,
    x: u32,
    y: u32,
    block_size: BlockSize,
    has_rows: bool,
    has_cols: bool,
) -> Result<PartitionNode> {
    // Get block size log2 for CDF lookup
    let size = block_size.width().max(block_size.height());
    let bsize_log2 = (size.ilog2() as u8).clamp(2, 7);

    // Read partition symbol
    let partition_symbol = decoder.read_partition(bsize_log2, has_rows, has_cols)?;
    let partition = PartitionType::from_u8(partition_symbol).ok_or_else(|| {
        bitvue_core::BitvueError::InvalidData(format!(
            "Invalid partition symbol: {}",
            partition_symbol
        ))
    })?;

    // Create node
    let mut node = PartitionNode::new(x, y, block_size, partition);

    // If not NONE, recursively parse children
    if partition != PartitionType::None {
        let sub_sizes = block_size.sub_block_size(partition);

        for (i, sub_size) in sub_sizes.iter().enumerate() {
            let (child_x, child_y) = child_position(x, y, i, partition, block_size);

            let child = parse_partition_recursive(
                decoder, child_x, child_y, *sub_size, has_rows, has_cols,
            )?;

            node.children.push(child);
        }
    }

    Ok(node)
}

/// Calculate child block position
fn child_position(
    parent_x: u32,
    parent_y: u32,
    child_index: usize,
    partition: PartitionType,
    parent_size: BlockSize,
) -> (u32, u32) {
    let w = parent_size.width();
    let h = parent_size.height();

    match partition {
        PartitionType::None => (parent_x, parent_y),
        PartitionType::Horz => {
            if child_index == 0 {
                (parent_x, parent_y)
            } else {
                (parent_x, parent_y + h / 2)
            }
        }
        PartitionType::Vert => {
            if child_index == 0 {
                (parent_x, parent_y)
            } else {
                (parent_x + w / 2, parent_y)
            }
        }
        PartitionType::Split => match child_index {
            0 => (parent_x, parent_y),
            1 => (parent_x + w / 2, parent_y),
            2 => (parent_x, parent_y + h / 2),
            3 => (parent_x + w / 2, parent_y + h / 2),
            _ => (parent_x, parent_y),
        },
        _ => (parent_x, parent_y),
    }
}

/// Recursively parse coding units for leaf blocks
fn parse_coding_units_recursive(
    decoder: &mut SymbolDecoder,
    partition: &PartitionNode,
    is_key_frame: bool,
    current_qp: i16,
    delta_q_enabled: bool,
    mv_ctx: &mut crate::tile::MvPredictorContext,
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
                coding_units,
            )?;
        }
        Ok(qp)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
