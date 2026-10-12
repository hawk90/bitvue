//! `PartitionNode` and the block geometry shared by the parser and the grid conversion.

use super::types::{BlockSize, PartitionType};
use serde::{Deserialize, Serialize};

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
pub(super) fn block_size_log2(block_size: BlockSize) -> u8 {
    // For square blocks, use width
    // For non-square, use larger dimension
    let size = block_size.width().max(block_size.height());

    // Safety: All BlockSize variants have dimensions >= 4, so size >= 4
    // ilog2() is safe for values >= 1
    debug_assert!(size >= 4, "BlockSize dimensions must be >= 4");

    (size.ilog2() as u8).clamp(2, 7)
}

/// Calculate child block position for a given partition
pub(super) fn child_position(
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
        PartitionType::HorzB => {
            // sub-blocks: [0]=top-full-width, [1]=bottom-left, [2]=bottom-right
            match child_index {
                0 => (parent_x, parent_y),
                1 => (parent_x, parent_y + h / 2),
                2 => (parent_x + w / 2, parent_y + h / 2),
                _ => (parent_x, parent_y),
            }
        }
        PartitionType::HorzA => {
            // sub-blocks: [0]=top-left, [1]=top-right, [2]=bottom-full-width
            match child_index {
                0 => (parent_x, parent_y),
                1 => (parent_x + w / 2, parent_y),
                2 => (parent_x, parent_y + h / 2),
                _ => (parent_x, parent_y),
            }
        }
        PartitionType::VertB => {
            // sub-blocks: [0]=left-full-height, [1]=top-right, [2]=bottom-right
            match child_index {
                0 => (parent_x, parent_y),
                1 => (parent_x + w / 2, parent_y),
                2 => (parent_x + w / 2, parent_y + h / 2),
                _ => (parent_x, parent_y),
            }
        }
        PartitionType::VertA => {
            // sub-blocks: [0]=top-left, [1]=bottom-left, [2]=right-full-height
            match child_index {
                0 => (parent_x, parent_y),
                1 => (parent_x, parent_y + h / 2),
                2 => (parent_x + w / 2, parent_y),
                _ => (parent_x, parent_y),
            }
        }
        PartitionType::Horz4 => {
            // Four equal horizontal strips at offsets 0, h/4, h/2, 3h/4
            (parent_x, parent_y + (child_index as u32) * (h / 4))
        }
        PartitionType::Vert4 => {
            // Four equal vertical strips at offsets 0, w/4, w/2, 3w/4
            (parent_x + (child_index as u32) * (w / 4), parent_y)
        }
    }
}

/// Maximum partition recursion depth to prevent infinite loops
/// AV1 maximum recursion is: 128x128 → 64x64 → 32x32 → 16x16 → 8x8 → 4x4 (6 levels)
/// Set to 10 to provide safety margin while allowing valid deep recursion
pub(super) const MAX_PARTITION_DEPTH: u8 = 10;

/// Convert a pixel dimension to AV1 "MI" (4x4 mode-info) units, per spec 5.9.5's
/// `MiCols = 2 * ((FrameWidth + 7) >> 3)` / `MiRows` (same formula on height) -- NOT simply
/// `ceil(pixels / 4)`: the extra `>>3`-then-`*2` rounds up to an 8-pixel boundary first, so
/// `MiCols`/`MiRows` are always even. Used by `parse_partition_recursive`'s real `hasRows`/
/// `hasCols` (spec 5.11.4) frame-edge check.
pub(crate) fn mi_units(pixels: u32) -> u32 {
    2 * ((pixels + 7) >> 3)
}
