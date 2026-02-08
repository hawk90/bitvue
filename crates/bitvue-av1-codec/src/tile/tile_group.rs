//! Tile Group Parsing
//!
//! Per AV1 Specification Section 5.11.1 (Tile Group OBU Syntax)
//!
//! Tile groups contain one or more tiles. In most cases, a single
//! tile group contains all tiles for a frame.

use crate::bitreader::BitReader;
use crate::leb128::decode_uleb128;
use crate::tile::Tile;
use bitvue_core::{BitvueError, Result};
use serde::{Deserialize, Serialize};

// SECURITY: Maximum tile counts to prevent DoS via excessive tiles
// AV1 spec allows up to 64 tile columns/rows, but we use conservative limits
const MAX_TILE_COLS: u32 = 64;  // Per AV1 spec
const MAX_TILE_ROWS: u32 = 64;  // Per AV1 spec
const MAX_TOTAL_TILES: u32 = 1024; // Conservative limit: 64x64 = 4096 max, we use 1024

/// Tile configuration information
///
/// This comes from the frame header's tile_info() section.
/// For MVP, we'll use simplified defaults.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TileInfo {
    /// Number of tile columns
    pub tile_cols: u32,
    /// Number of tile rows
    pub tile_rows: u32,
    /// Tile column start positions (in superblock units)
    pub tile_col_start_sb: Vec<u32>,
    /// Tile row start positions (in superblock units)
    pub tile_row_start_sb: Vec<u32>,
    /// Use 128x128 superblock
    pub use_128x128_superblock: bool,
}

impl TileInfo {
    /// Create default tile info (single tile)
    pub fn single_tile(frame_width: u32, frame_height: u32, sb_size: u32) -> Self {
        let sb_cols = frame_width.div_ceil(sb_size);
        let sb_rows = frame_height.div_ceil(sb_size);

        Self {
            tile_cols: 1,
            tile_rows: 1,
            tile_col_start_sb: vec![0, sb_cols],
            tile_row_start_sb: vec![0, sb_rows],
            use_128x128_superblock: sb_size == 128,
        }
    }

    /// Create tile info with custom tile counts (with validation)
    pub fn new(tile_cols: u32, tile_rows: u32) -> Result<Self> {
        // SECURITY: Validate tile counts to prevent excessive tiles
        if tile_cols > MAX_TILE_COLS {
            return Err(BitvueError::Decode(format!(
                "Tile columns {} exceeds maximum {}",
                tile_cols, MAX_TILE_COLS
            )));
        }
        if tile_rows > MAX_TILE_ROWS {
            return Err(BitvueError::Decode(format!(
                "Tile rows {} exceeds maximum {}",
                tile_rows, MAX_TILE_ROWS
            )));
        }

        // Check total tile count
        let total_tiles = tile_cols
            .checked_mul(tile_rows)
            .ok_or_else(|| {
                BitvueError::Decode(format!(
                    "Tile columns {} * rows {} would overflow",
                    tile_cols, tile_rows
                ))
            })?;

        if total_tiles > MAX_TOTAL_TILES {
            return Err(BitvueError::Decode(format!(
                "Total tile count {} exceeds maximum {}",
                total_tiles, MAX_TOTAL_TILES
            )));
        }

        Ok(Self {
            tile_cols,
            tile_rows,
            tile_col_start_sb: Vec::new(),
            tile_row_start_sb: Vec::new(),
            use_128x128_superblock: false,
        })
    }

    /// Get total number of tiles
    pub fn tile_count(&self) -> usize {
        // SECURITY: Use checked multiplication to prevent overflow
        self.tile_cols
            .checked_mul(self.tile_rows)
            .unwrap_or(u32::MAX) as usize
    }

    /// Get tile dimensions in superblocks
    pub fn tile_sb_dimensions(&self, tile_col: u32, tile_row: u32) -> Option<(u32, u32)> {
        if tile_col >= self.tile_cols || tile_row >= self.tile_rows {
            return None;
        }

        let col_idx = tile_col as usize;
        let row_idx = tile_row as usize;

        let sb_cols = self.tile_col_start_sb[col_idx + 1] - self.tile_col_start_sb[col_idx];
        let sb_rows = self.tile_row_start_sb[row_idx + 1] - self.tile_row_start_sb[row_idx];

        Some((sb_cols, sb_rows))
    }
}

/// Tile Group (one or more tiles)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TileGroup {
    /// Tile start index
    pub tile_start_idx: u32,
    /// Tile end index
    pub tile_end_idx: u32,
    /// Tiles in this group
    pub tiles: Vec<Tile>,
    /// Tile configuration
    pub tile_info: TileInfo,
}

impl TileGroup {
    /// Get total number of tiles in this group
    pub fn tile_count(&self) -> usize {
        self.tiles.len()
    }

    /// Check if this is a complete frame (all tiles present)
    pub fn is_complete_frame(&self) -> bool {
        self.tile_start_idx == 0 && self.tile_end_idx == (self.tile_info.tile_count() as u32 - 1)
    }
}

/// Parse tile group from OBU payload
///
/// **Implementation Status**: v0.3.x - Multi-tile support added
/// - Single tile per frame (most common case)
/// - Multi-tile frames with tile size delimiters
/// - Handles tile_start_idx and tile_end_idx
///
/// # Arguments
///
/// * `data` - Tile group OBU payload
/// * `tile_info` - Tile configuration from frame header
/// * `frame_header_bytes` - Size of frame header (for positioning)
/// * `tile_start_and_end_present` - Whether tile_start/end indices are present
///
/// # Returns
///
/// Parsed tile group with individual tiles
pub fn parse_tile_group(
    data: &[u8],
    tile_info: TileInfo,
    _frame_header_bytes: usize,
) -> Result<TileGroup> {
    let tile_count = tile_info.tile_count();

    if tile_count == 1 {
        // Single tile - simple case
        // Entire payload is the tile data
        let (sb_cols, sb_rows) = tile_info.tile_sb_dimensions(0, 0).unwrap();

        let tile = Tile::new(0, 0, sb_cols, sb_rows, data.to_vec());

        Ok(TileGroup {
            tile_start_idx: 0,
            tile_end_idx: 0,
            tiles: vec![tile],
            tile_info,
        })
    } else {
        // Multi-tile - use full implementation
        // For now, assume tile_start_and_end_present is false (common case)
        // In full implementation, this would come from frame header
        parse_tile_group_full(data, tile_info, false)
    }
}

/// Parse tile group with size delimiters (full implementation)
///
/// Parses multi-tile frames by reading tile size delimiters.
/// This is used when a frame has multiple tiles.
///
/// # Arguments
///
/// * `data` - Tile group OBU payload
/// * `tile_info` - Tile configuration from frame header
/// * `tile_start_and_end_present` - Whether tile_start/end indices are present
fn parse_tile_group_full(
    data: &[u8],
    tile_info: TileInfo,
    tile_start_and_end_present: bool,
) -> Result<TileGroup> {
    let mut reader = BitReader::new(data);
    let tile_count = tile_info.tile_count();

    // Read tile start/end indices if present
    let (tile_start_idx, tile_end_idx) = if tile_start_and_end_present {
        let tile_bits = (tile_info.tile_cols.max(tile_info.tile_rows).ilog2() as u8).max(1);
        let start = reader.read_bits(tile_bits)?;
        let end = reader.read_bits(tile_bits)?;
        (start, end)
    } else {
        (0, (tile_count - 1) as u32)
    };

    let num_tiles = (tile_end_idx - tile_start_idx + 1) as usize;
    let mut tiles = Vec::with_capacity(num_tiles);

    // Read tile data
    let mut offset = reader.byte_position();

    for tile_idx in tile_start_idx..=tile_end_idx {
        // Calculate tile col/row from linear index
        let tile_col = tile_idx % tile_info.tile_cols;
        let tile_row = tile_idx / tile_info.tile_cols;

        // Get tile dimensions
        let (sb_cols, sb_rows) = tile_info
            .tile_sb_dimensions(tile_col, tile_row)
            .ok_or_else(|| {
                BitvueError::InvalidData(format!(
                    "Invalid tile index: col={}, row={}",
                    tile_col, tile_row
                ))
            })?;

        // Read tile size (except for last tile)
        let tile_size = if tile_idx == tile_end_idx {
            // Last tile - extends to end of data
            data.len() - offset
        } else {
            // Read tile_size_minus_1 as LEB128
            let (size_minus_1, leb_bytes) = decode_uleb128(&data[offset..])?;
            offset += leb_bytes;
            (size_minus_1 + 1) as usize
        };

        // Extract tile data
        if offset + tile_size > data.len() {
            return Err(BitvueError::UnexpectedEof(offset as u64 + tile_size as u64));
        }

        let tile_data = data[offset..offset + tile_size].to_vec();
        offset += tile_size;

        tiles.push(Tile::new(tile_col, tile_row, sb_cols, sb_rows, tile_data));
    }

    Ok(TileGroup {
        tile_start_idx,
        tile_end_idx,
        tiles,
        tile_info,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tile_info_single_tile() {
        let info = TileInfo::single_tile(1920, 1080, 128);

        assert_eq!(info.tile_cols, 1);
        assert_eq!(info.tile_rows, 1);
        assert_eq!(info.tile_count(), 1);

        // 1920 / 128 = 15 superblocks
        // 1080 / 128 = 8.4 -> 9 superblocks
        assert_eq!(info.tile_col_start_sb, vec![0, 15]);
        assert_eq!(info.tile_row_start_sb, vec![0, 9]);
    }

    #[test]
    fn test_tile_info_dimensions() {
        let info = TileInfo::single_tile(1920, 1080, 128);
        let (sb_cols, sb_rows) = info.tile_sb_dimensions(0, 0).unwrap();

        assert_eq!(sb_cols, 15);
        assert_eq!(sb_rows, 9);
    }

    #[test]
    fn test_parse_single_tile_group() {
        let tile_info = TileInfo::single_tile(352, 288, 64);
        let data = vec![0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC];

        let result = parse_tile_group(&data, tile_info.clone(), 0);
        assert!(result.is_ok());

        let tile_group = result.unwrap();
        assert_eq!(tile_group.tile_count(), 1);
        assert_eq!(tile_group.tile_start_idx, 0);
        assert_eq!(tile_group.tile_end_idx, 0);
        assert!(tile_group.is_complete_frame());

        let tile = &tile_group.tiles[0];
        assert_eq!(tile.tile_col, 0);
        assert_eq!(tile.tile_row, 0);
        assert_eq!(tile.data, data);
    }

    #[test]
    fn test_tile_group_complete_frame() {
        let tile_info = TileInfo::single_tile(1920, 1080, 128);
        let mut tile_info_multi = tile_info.clone();
        tile_info_multi.tile_cols = 2;
        tile_info_multi.tile_rows = 2;

        let group_complete = TileGroup {
            tile_start_idx: 0,
            tile_end_idx: 3,
            tiles: vec![],
            tile_info: tile_info_multi.clone(),
        };

        let group_partial = TileGroup {
            tile_start_idx: 0,
            tile_end_idx: 1,
            tiles: vec![],
            tile_info: tile_info_multi,
        };

        assert!(group_complete.is_complete_frame());
        assert!(!group_partial.is_complete_frame());
    }
}
