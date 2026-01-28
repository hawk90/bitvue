//! AV1 Tile Parsing
//!
//! Per AV1 Specification Section 5.11 (Tile Syntax)
//! This module parses tile data to extract block-level information:
//! - Partition structure (superblock → block tree)
//! - Prediction modes (Intra/Inter)
//! - Motion vectors (for Inter frames)
//! - Transform information
//!
//! ## Implementation Status
//!
//! **MVP Phase (KEY Frames Only)**:
//! - ✅ Tile group structure
//! - 🚧 Partition tree parser (in progress)
//! - ⏳ Intra mode extraction
//! - ⏳ Block size grid
//!
//! **Full Implementation (Later)**:
//! - ⏳ Symbol decoder (arithmetic coding)
//! - ⏳ INTER frame support
//! - ⏳ Motion vector extraction
//! - ⏳ Transform coefficient parsing

pub mod coding_unit;
pub mod mv_prediction;
pub mod partition;
pub mod superblock;
pub mod tile_group;

pub use coding_unit::{
    parse_coding_unit, CodingUnit, MotionVector, PredictionMode, RefFrame, TxSize,
};
pub use mv_prediction::MvPredictorContext;
pub use partition::{
    parse_partition_tree, partition_tree_to_grid, BlockSize, PartitionNode, PartitionType,
};
pub use superblock::{parse_superblock, Superblock};
pub use tile_group::{parse_tile_group, TileGroup, TileInfo};

use serde::{Deserialize, Serialize};

/// Tile data (single tile within a tile group)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tile {
    /// Tile column index
    pub tile_col: u32,
    /// Tile row index
    pub tile_row: u32,
    /// Tile width in superblocks
    pub sb_cols: u32,
    /// Tile height in superblocks
    pub sb_rows: u32,
    /// Tile data (compressed)
    pub data: Vec<u8>,
    /// Tile data size in bytes
    pub size: usize,
}

impl Tile {
    /// Create a new tile
    pub fn new(tile_col: u32, tile_row: u32, sb_cols: u32, sb_rows: u32, data: Vec<u8>) -> Self {
        let size = data.len();
        Self {
            tile_col,
            tile_row,
            sb_cols,
            sb_rows,
            data,
            size,
        }
    }

    /// Get tile dimensions in pixels (assuming 128x128 superblocks)
    pub fn pixel_dimensions(&self, sb_size: u32) -> (u32, u32) {
        (self.sb_cols * sb_size, self.sb_rows * sb_size)
    }
}

/// Superblock size
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SuperblockSize {
    /// 128x128 superblock
    Sb128x128 = 128,
    /// 64x64 superblock
    Sb64x64 = 64,
}

impl SuperblockSize {
    /// Get size in pixels
    pub fn size(&self) -> u32 {
        match self {
            SuperblockSize::Sb128x128 => 128,
            SuperblockSize::Sb64x64 => 64,
        }
    }

    /// Parse from sequence header flags
    pub fn from_seq_header(use_128x128_superblock: bool) -> Self {
        if use_128x128_superblock {
            SuperblockSize::Sb128x128
        } else {
            SuperblockSize::Sb64x64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tile_creation() {
        let data = vec![0x12, 0x34, 0x56, 0x78];
        let tile = Tile::new(0, 0, 4, 3, data.clone());

        assert_eq!(tile.tile_col, 0);
        assert_eq!(tile.tile_row, 0);
        assert_eq!(tile.sb_cols, 4);
        assert_eq!(tile.sb_rows, 3);
        assert_eq!(tile.size, 4);
        assert_eq!(tile.data, data);
    }

    #[test]
    fn test_tile_pixel_dimensions() {
        let tile = Tile::new(0, 0, 4, 3, vec![]);
        let (width, height) = tile.pixel_dimensions(128);

        assert_eq!(width, 512); // 4 * 128
        assert_eq!(height, 384); // 3 * 128
    }

    #[test]
    fn test_superblock_size() {
        assert_eq!(SuperblockSize::Sb128x128.size(), 128);
        assert_eq!(SuperblockSize::Sb64x64.size(), 64);

        assert_eq!(
            SuperblockSize::from_seq_header(true),
            SuperblockSize::Sb128x128
        );
        assert_eq!(
            SuperblockSize::from_seq_header(false),
            SuperblockSize::Sb64x64
        );
    }
}
