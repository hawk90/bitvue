//! Partition Tree Parsing
//!
//! Per AV1 Specification Section 5.11.4 (Decode Partition)
//!
//! AV1 uses a recursive partition structure:
//! - Superblock (128x128 or 64x64) is the top level
//! - Each block can be partitioned into smaller blocks
//! - Minimum block size is 4x4
//!
//! ## Partition Types
//!
//! ```text
//! PARTITION_NONE:      PARTITION_HORZ:     PARTITION_VERT:
//! ┌───────────┐        ┌───────────┐        ┌─────┬─────┐
//! │           │        ├───────────┤        │     │     │
//! │           │        │           │        │     │     │
//! └───────────┘        └───────────┘        └─────┴─────┘
//!
//! PARTITION_SPLIT:     PARTITION_HORZ_A:   PARTITION_HORZ_B:
//! ┌─────┬─────┐        ┌───────────┐        ┌─────┬─────┐
//! ├─────┼─────┤        ├─────┬─────┤        ├───────────┤
//! │     │     │        │     │     │        │           │
//! └─────┴─────┘        └─────┴─────┘        └───────────┘
//!
//! PARTITION_VERT_A:    PARTITION_VERT_B:   PARTITION_HORZ_4:
//! ┌─────┬─────┐        ┌───────────┐        ┌───────────┐
//! │     ├─────┤        ├─────┬─────┐        ├───────────┤
//! │     │     │        │     │     │        ├───────────┤
//! └─────┴─────┘        └─────┴─────┘        ├───────────┤
//!                                            └───────────┘
//!
//! PARTITION_VERT_4:
//! ┌──┬──┬──┬──┐
//! │  │  │  │  │
//! │  │  │  │  │
//! └──┴──┴──┴──┘
//! ```

use crate::symbol::SymbolDecoder;
use bitvue_core::{BitvueError, Result};
use serde::{Deserialize, Serialize};

/// Partition type (AV1 Spec Section 5.11.4)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum PartitionType {
    /// No partition (use entire block)
    None = 0,
    /// Horizontal split (2 blocks)
    Horz = 1,
    /// Vertical split (2 blocks)
    Vert = 2,
    /// Split into 4 equal blocks
    Split = 3,
    /// Horizontal split with top half split again
    HorzA = 4,
    /// Horizontal split with bottom half split again
    HorzB = 5,
    /// Vertical split with left half split again
    VertA = 6,
    /// Vertical split with right half split again
    VertB = 7,
    /// 4-way horizontal split
    Horz4 = 8,
    /// 4-way vertical split
    Vert4 = 9,
}

impl PartitionType {
    /// Parse from value (0-9)
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(PartitionType::None),
            1 => Some(PartitionType::Horz),
            2 => Some(PartitionType::Vert),
            3 => Some(PartitionType::Split),
            4 => Some(PartitionType::HorzA),
            5 => Some(PartitionType::HorzB),
            6 => Some(PartitionType::VertA),
            7 => Some(PartitionType::VertB),
            8 => Some(PartitionType::Horz4),
            9 => Some(PartitionType::Vert4),
            _ => None,
        }
    }

    /// Get number of sub-blocks this partition creates
    pub fn sub_block_count(&self) -> usize {
        match self {
            PartitionType::None => 1,
            PartitionType::Horz | PartitionType::Vert => 2,
            PartitionType::Split => 4,
            PartitionType::HorzA | PartitionType::HorzB => 3,
            PartitionType::VertA | PartitionType::VertB => 3,
            PartitionType::Horz4 | PartitionType::Vert4 => 4,
        }
    }

    /// Check if this partition type is allowed for given block size
    pub fn is_allowed(&self, block_size: BlockSize) -> bool {
        match self {
            PartitionType::None => true,
            PartitionType::Horz | PartitionType::Vert => {
                block_size.width() >= 8 && block_size.height() >= 8
            }
            PartitionType::Split => block_size.width() >= 8 && block_size.height() >= 8,
            PartitionType::HorzA | PartitionType::HorzB => {
                block_size.width() >= 16 && block_size.height() >= 16
            }
            PartitionType::VertA | PartitionType::VertB => {
                block_size.width() >= 16 && block_size.height() >= 16
            }
            PartitionType::Horz4 => block_size.width() >= 16 && block_size.height() >= 32,
            PartitionType::Vert4 => block_size.width() >= 32 && block_size.height() >= 16,
        }
    }
}

/// Block size enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlockSize {
    /// 4x4 block
    Block4x4,
    /// 4x8 block
    Block4x8,
    /// 8x4 block
    Block8x4,
    /// 8x8 block
    Block8x8,
    /// 8x16 block
    Block8x16,
    /// 16x8 block
    Block16x8,
    /// 16x16 block
    Block16x16,
    /// 16x32 block
    Block16x32,
    /// 32x16 block
    Block32x16,
    /// 32x32 block
    Block32x32,
    /// 32x64 block
    Block32x64,
    /// 64x32 block
    Block64x32,
    /// 64x64 block
    Block64x64,
    /// 64x128 block
    Block64x128,
    /// 128x64 block
    Block128x64,
    /// 128x128 block
    Block128x128,
}

impl BlockSize {
    /// Get block width in pixels
    pub fn width(&self) -> u32 {
        match self {
            BlockSize::Block4x4 | BlockSize::Block4x8 => 4,
            BlockSize::Block8x4 | BlockSize::Block8x8 | BlockSize::Block8x16 => 8,
            BlockSize::Block16x8 | BlockSize::Block16x16 | BlockSize::Block16x32 => 16,
            BlockSize::Block32x16 | BlockSize::Block32x32 | BlockSize::Block32x64 => 32,
            BlockSize::Block64x32 | BlockSize::Block64x64 | BlockSize::Block64x128 => 64,
            BlockSize::Block128x64 | BlockSize::Block128x128 => 128,
        }
    }

    /// Get block height in pixels
    pub fn height(&self) -> u32 {
        match self {
            BlockSize::Block4x4 | BlockSize::Block8x4 => 4,
            BlockSize::Block4x8 | BlockSize::Block8x8 | BlockSize::Block16x8 => 8,
            BlockSize::Block8x16 | BlockSize::Block16x16 | BlockSize::Block32x16 => 16,
            BlockSize::Block16x32 | BlockSize::Block32x32 | BlockSize::Block64x32 => 32,
            BlockSize::Block32x64 | BlockSize::Block64x64 | BlockSize::Block128x64 => 64,
            BlockSize::Block64x128 | BlockSize::Block128x128 => 128,
        }
    }

    /// Get sub-block size after partition
    pub fn sub_block_size(&self, partition: PartitionType) -> Vec<BlockSize> {
        match partition {
            PartitionType::None => vec![*self],
            PartitionType::Horz => {
                // Split horizontally
                match self {
                    BlockSize::Block8x8 => vec![BlockSize::Block8x4, BlockSize::Block8x4],
                    BlockSize::Block16x16 => vec![BlockSize::Block16x8, BlockSize::Block16x8],
                    BlockSize::Block32x32 => vec![BlockSize::Block32x16, BlockSize::Block32x16],
                    BlockSize::Block64x64 => vec![BlockSize::Block64x32, BlockSize::Block64x32],
                    BlockSize::Block128x128 => vec![BlockSize::Block128x64, BlockSize::Block128x64],
                    _ => vec![*self], // Fallback
                }
            }
            PartitionType::Vert => {
                // Split vertically
                match self {
                    BlockSize::Block8x8 => vec![BlockSize::Block4x8, BlockSize::Block4x8],
                    BlockSize::Block16x16 => vec![BlockSize::Block8x16, BlockSize::Block8x16],
                    BlockSize::Block32x32 => vec![BlockSize::Block16x32, BlockSize::Block16x32],
                    BlockSize::Block64x64 => vec![BlockSize::Block32x64, BlockSize::Block32x64],
                    BlockSize::Block128x128 => vec![BlockSize::Block64x128, BlockSize::Block64x128],
                    _ => vec![*self], // Fallback
                }
            }
            PartitionType::Split => {
                // Split into 4 equal blocks
                match self {
                    BlockSize::Block8x8 => vec![BlockSize::Block4x4; 4],
                    BlockSize::Block16x16 => vec![BlockSize::Block8x8; 4],
                    BlockSize::Block32x32 => vec![BlockSize::Block16x16; 4],
                    BlockSize::Block64x64 => vec![BlockSize::Block32x32; 4],
                    BlockSize::Block128x128 => vec![BlockSize::Block64x64; 4],
                    _ => vec![*self], // Fallback
                }
            }
            _ => {
                // Complex partitions - TODO: implement
                vec![*self]
            }
        }
    }
}

/// Partition tree node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartitionNode {
    /// Block position (top-left corner) in pixels
    pub x: u32,
    pub y: u32,
    /// Block size
    pub size: BlockSize,
    /// Partition type
    pub partition: PartitionType,
    /// Child nodes (if partitioned)
    pub children: Vec<PartitionNode>,
}

impl PartitionNode {
    /// Create a new partition node
    pub fn new(x: u32, y: u32, size: BlockSize, partition: PartitionType) -> Self {
        Self {
            x,
            y,
            size,
            partition,
            children: Vec::new(),
        }
    }

    /// Check if this is a leaf node (no children)
    pub fn is_leaf(&self) -> bool {
        self.children.is_empty()
    }

    /// Get all leaf blocks (flattened)
    pub fn leaf_blocks(&self) -> Vec<(u32, u32, BlockSize)> {
        if self.is_leaf() {
            vec![(self.x, self.y, self.size)]
        } else {
            self.children
                .iter()
                .flat_map(|child| child.leaf_blocks())
                .collect()
        }
    }
}

/// Calculate block size as log2 (for CDF lookup)
///
/// # Safety
///
/// This function is safe for all BlockSize variants because:
/// - All BlockSize variants have positive dimensions (4x4 and up)
/// - width() and height() always return values >= 4
/// - ilog2() will never panic on these values
///
/// # Returns
///
/// Log2 of the larger dimension, clamped to [2, 7]
fn block_size_log2(block_size: BlockSize) -> u8 {
    // For square blocks, use width
    // For non-square, use larger dimension
    let size = block_size.width().max(block_size.height());

    // Safety: All BlockSize variants have dimensions >= 4, so size >= 4
    // ilog2() is safe for values >= 1
    debug_assert!(size >= 4, "BlockSize dimensions must be >= 4");

    (size.ilog2() as u8).clamp(2, 7)
}

/// Calculate child block position for a given partition
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
            // Top and bottom halves
            if child_index == 0 {
                (parent_x, parent_y)
            } else {
                (parent_x, parent_y + h / 2)
            }
        }
        PartitionType::Vert => {
            // Left and right halves
            if child_index == 0 {
                (parent_x, parent_y)
            } else {
                (parent_x + w / 2, parent_y)
            }
        }
        PartitionType::Split => {
            // 4-way split: top-left, top-right, bottom-left, bottom-right
            match child_index {
                0 => (parent_x, parent_y),
                1 => (parent_x + w / 2, parent_y),
                2 => (parent_x, parent_y + h / 2),
                3 => (parent_x + w / 2, parent_y + h / 2),
                _ => (parent_x, parent_y),
            }
        }
        _ => {
            // Complex partitions - for MVP, just return parent position
            (parent_x, parent_y)
        }
    }
}

/// Recursively parse partition tree using symbol decoder
fn parse_partition_recursive(
    decoder: &mut SymbolDecoder,
    x: u32,
    y: u32,
    block_size: BlockSize,
    has_rows: bool,
    has_cols: bool,
) -> Result<PartitionNode> {
    // Get block size log2 for CDF lookup
    let bsize_log2 = block_size_log2(block_size);

    // Read partition symbol from bitstream
    let partition_symbol = decoder.read_partition(bsize_log2, has_rows, has_cols)?;

    // Convert symbol to partition type
    let partition = PartitionType::from_u8(partition_symbol).ok_or_else(|| {
        BitvueError::InvalidData(format!("Invalid partition symbol: {}", partition_symbol))
    })?;

    // Validate partition is allowed for this block size
    if !partition.is_allowed(block_size) {
        return Err(BitvueError::InvalidData(format!(
            "Partition {:?} not allowed for block size {:?}",
            partition, block_size
        )));
    }

    // Create node
    let mut node = PartitionNode::new(x, y, block_size, partition);

    // If not NONE, recursively parse children
    if partition != PartitionType::None {
        let sub_sizes = block_size.sub_block_size(partition);

        for (i, sub_size) in sub_sizes.iter().enumerate() {
            let (child_x, child_y) = child_position(x, y, i, partition, block_size);

            // Check if child blocks are at frame boundaries
            // For MVP, we assume all blocks are within frame
            let child_has_rows = has_rows;
            let child_has_cols = has_cols;

            // Recursively parse child
            let child = parse_partition_recursive(
                decoder,
                child_x,
                child_y,
                *sub_size,
                child_has_rows,
                child_has_cols,
            )?;

            node.children.push(child);
        }
    }

    Ok(node)
}

/// Parse partition tree from tile data
///
/// Uses symbol decoder to read partition symbols from the bitstream
/// and recursively builds the partition tree.
///
/// # Arguments
///
/// * `x` - Block X position in pixels
/// * `y` - Block Y position in pixels
/// * `block_size` - Starting block size (typically superblock size)
/// * `tile_data` - Tile bitstream data
///
/// # Returns
///
/// Root partition node with recursively parsed children.
pub fn parse_partition_tree(
    x: u32,
    y: u32,
    block_size: BlockSize,
    tile_data: &[u8],
) -> Result<PartitionNode> {
    // Create symbol decoder for tile data
    let mut decoder = SymbolDecoder::new(tile_data)?;

    // Recursively parse partition tree
    // For MVP, assume all blocks are within frame boundaries
    parse_partition_recursive(&mut decoder, x, y, block_size, true, true)
}

/// Flatten partition tree to list of leaf blocks
///
/// Converts hierarchical partition tree to a flat list of blocks
/// for visualization in PartitionGrid.
fn flatten_partition_tree(
    node: &PartitionNode,
    depth: u8,
    blocks: &mut Vec<(u32, u32, u32, u32, u8, u8)>,
) {
    if node.is_leaf() {
        // Leaf node: add as block
        blocks.push((
            node.x,
            node.y,
            node.size.width(),
            node.size.height(),
            node.partition as u8,
            depth,
        ));
    } else {
        // Non-leaf: recursively process children
        for child in &node.children {
            flatten_partition_tree(child, depth + 1, blocks);
        }
    }
}

/// Convert partition tree to PartitionGrid
///
/// Extracts all leaf blocks from partition tree and creates a PartitionGrid
/// suitable for visualization.
pub fn partition_tree_to_grid(
    root: &PartitionNode,
    coded_width: u32,
    coded_height: u32,
    sb_size: u32,
) -> bitvue_core::PartitionGrid {
    let mut grid = bitvue_core::PartitionGrid::new(coded_width, coded_height, sb_size);

    // Flatten tree to list of blocks
    let mut blocks = Vec::new();
    flatten_partition_tree(root, 0, &mut blocks);

    // Convert to PartitionGrid blocks
    for (x, y, width, height, partition, depth) in blocks {
        let partition_type = bitvue_core::partition_grid::PartitionType::from(partition);
        let block = bitvue_core::partition_grid::PartitionBlock::new(
            x,
            y,
            width,
            height,
            partition_type,
            depth,
        );
        grid.add_block(block);
    }

    grid
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_partition_type_sub_block_count() {
        assert_eq!(PartitionType::None.sub_block_count(), 1);
        assert_eq!(PartitionType::Horz.sub_block_count(), 2);
        assert_eq!(PartitionType::Vert.sub_block_count(), 2);
        assert_eq!(PartitionType::Split.sub_block_count(), 4);
        assert_eq!(PartitionType::HorzA.sub_block_count(), 3);
    }

    #[test]
    fn test_block_size_dimensions() {
        assert_eq!(BlockSize::Block8x8.width(), 8);
        assert_eq!(BlockSize::Block8x8.height(), 8);
        assert_eq!(BlockSize::Block16x32.width(), 16);
        assert_eq!(BlockSize::Block16x32.height(), 32);
        assert_eq!(BlockSize::Block128x128.width(), 128);
        assert_eq!(BlockSize::Block128x128.height(), 128);
    }

    #[test]
    fn test_block_size_sub_blocks() {
        let subs = BlockSize::Block16x16.sub_block_size(PartitionType::Split);
        assert_eq!(subs.len(), 4);
        assert_eq!(subs[0], BlockSize::Block8x8);

        let subs = BlockSize::Block32x32.sub_block_size(PartitionType::Horz);
        assert_eq!(subs.len(), 2);
        assert_eq!(subs[0], BlockSize::Block32x16);
    }

    #[test]
    fn test_partition_node_leaf_blocks() {
        let node = PartitionNode::new(0, 0, BlockSize::Block16x16, PartitionType::None);
        let leaves = node.leaf_blocks();

        assert_eq!(leaves.len(), 1);
        assert_eq!(leaves[0], (0, 0, BlockSize::Block16x16));
    }

    #[test]
    fn test_partition_node_with_children() {
        let mut parent = PartitionNode::new(0, 0, BlockSize::Block32x32, PartitionType::Split);
        parent.children.push(PartitionNode::new(
            0,
            0,
            BlockSize::Block16x16,
            PartitionType::None,
        ));
        parent.children.push(PartitionNode::new(
            16,
            0,
            BlockSize::Block16x16,
            PartitionType::None,
        ));
        parent.children.push(PartitionNode::new(
            0,
            16,
            BlockSize::Block16x16,
            PartitionType::None,
        ));
        parent.children.push(PartitionNode::new(
            16,
            16,
            BlockSize::Block16x16,
            PartitionType::None,
        ));

        assert!(!parent.is_leaf());
        let leaves = parent.leaf_blocks();
        assert_eq!(leaves.len(), 4);
    }
}
