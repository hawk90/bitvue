//! AV1 Codec Quirks - quirks_AV1.001
//!
//! Handles AV1-specific quirks:
//! - Tile-to-superblock mapping
//! - Film grain flags
//! - show_existing_frame handling
//!
//! FRAME_IDENTITY_CONTRACT:
//! - All frame references use display_idx
//! - decode_idx is internal only

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// AV1 tile configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Av1TileConfig {
    /// Number of tile columns
    pub tile_cols: u32,

    /// Number of tile rows
    pub tile_rows: u32,

    /// Tile column widths in superblocks
    pub tile_col_widths_sb: Vec<u32>,

    /// Tile row heights in superblocks
    pub tile_row_heights_sb: Vec<u32>,

    /// Total number of tiles
    pub tile_count: u32,
}

impl Av1TileConfig {
    /// Create new tile configuration
    pub fn new(tile_cols: u32, tile_rows: u32) -> Self {
        Self {
            tile_cols,
            tile_rows,
            tile_col_widths_sb: Vec::new(),
            tile_row_heights_sb: Vec::new(),
            tile_count: tile_cols * tile_rows,
        }
    }

    /// Get tile index from tile coordinates
    pub fn tile_index(&self, tile_col: u32, tile_row: u32) -> Option<u32> {
        if tile_col >= self.tile_cols || tile_row >= self.tile_rows {
            None
        } else {
            Some(tile_row * self.tile_cols + tile_col)
        }
    }

    /// Get tile coordinates from tile index
    pub fn tile_coords(&self, tile_idx: u32) -> Option<(u32, u32)> {
        if tile_idx >= self.tile_count {
            None
        } else {
            let tile_row = tile_idx / self.tile_cols;
            let tile_col = tile_idx % self.tile_cols;
            Some((tile_col, tile_row))
        }
    }
}

/// Superblock size (64x64 or 128x128)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SuperblockSize {
    /// 64x64 superblock
    Sb64,

    /// 128x128 superblock
    Sb128,
}

impl SuperblockSize {
    /// Get superblock size in pixels
    pub fn size(&self) -> u32 {
        match self {
            SuperblockSize::Sb64 => 64,
            SuperblockSize::Sb128 => 128,
        }
    }

    /// Get number of superblocks for dimension
    pub fn sb_count(&self, pixels: u32) -> u32 {
        pixels.div_ceil(self.size())
    }
}

/// Tile-to-superblock mapping
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TileSbMapping {
    /// Superblock size
    pub sb_size: SuperblockSize,

    /// Tile configuration
    pub tile_config: Av1TileConfig,

    /// Mapping: tile_idx -> (sb_col_start, sb_row_start, sb_cols, sb_rows)
    pub tile_sb_bounds: HashMap<u32, (u32, u32, u32, u32)>,
}

impl TileSbMapping {
    /// Create new tile-to-superblock mapping
    pub fn new(
        width: u32,
        height: u32,
        sb_size: SuperblockSize,
        tile_config: Av1TileConfig,
    ) -> Self {
        let sb_cols = sb_size.sb_count(width);
        let sb_rows = sb_size.sb_count(height);

        let mut tile_sb_bounds = HashMap::new();

        // Calculate superblock bounds for each tile
        let mut sb_col = 0;
        for tile_col in 0..tile_config.tile_cols {
            let mut sb_row = 0;
            for tile_row in 0..tile_config.tile_rows {
                if let Some(tile_idx) = tile_config.tile_index(tile_col, tile_row) {
                    // For simplicity, distribute SBs evenly across tiles
                    // Real implementation would use actual tile sizes from bitstream
                    let tile_sb_cols = sb_cols / tile_config.tile_cols;
                    let tile_sb_rows = sb_rows / tile_config.tile_rows;

                    tile_sb_bounds.insert(tile_idx, (sb_col, sb_row, tile_sb_cols, tile_sb_rows));

                    sb_row += tile_sb_rows;
                }
            }
            sb_col += sb_cols / tile_config.tile_cols;
        }

        Self {
            sb_size,
            tile_config,
            tile_sb_bounds,
        }
    }

    /// Get superblock bounds for a tile
    pub fn get_tile_sb_bounds(&self, tile_idx: u32) -> Option<(u32, u32, u32, u32)> {
        self.tile_sb_bounds.get(&tile_idx).copied()
    }

    /// Check if superblock is in a tile
    pub fn sb_in_tile(&self, sb_col: u32, sb_row: u32, tile_idx: u32) -> bool {
        if let Some((start_col, start_row, cols, rows)) = self.get_tile_sb_bounds(tile_idx) {
            sb_col >= start_col
                && sb_col < start_col + cols
                && sb_row >= start_row
                && sb_row < start_row + rows
        } else {
            false
        }
    }
}

/// Film grain parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Av1FilmGrain {
    /// Apply grain flag
    pub apply_grain: bool,

    /// Grain seed
    pub grain_seed: u16,

    /// Update grain flag
    pub update_grain: bool,

    /// Film grain parameter reference
    pub film_grain_params_ref_idx: Option<u8>,

    /// Chroma scaling from luma
    pub chroma_scaling_from_luma: bool,

    /// Number of y points
    pub num_y_points: u8,

    /// Number of cb points
    pub num_cb_points: u8,

    /// Number of cr points
    pub num_cr_points: u8,
}

impl Av1FilmGrain {
    /// Create new film grain parameters
    pub fn new() -> Self {
        Self {
            apply_grain: false,
            grain_seed: 0,
            update_grain: false,
            film_grain_params_ref_idx: None,
            chroma_scaling_from_luma: false,
            num_y_points: 0,
            num_cb_points: 0,
            num_cr_points: 0,
        }
    }

    /// Check if film grain is enabled
    pub fn is_enabled(&self) -> bool {
        self.apply_grain
    }

    /// Check if film grain parameters should be updated
    pub fn should_update(&self) -> bool {
        self.update_grain
    }
}

impl Default for Av1FilmGrain {
    fn default() -> Self {
        Self::new()
    }
}

/// Show existing frame handling
///
/// Per AV1 spec, show_existing_frame allows displaying a previously
/// decoded frame without decoding a new one.
///
/// FRAME_IDENTITY_CONTRACT: display_idx is PRIMARY
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShowExistingFrame {
    /// Display index of current frame position - PRIMARY identifier
    pub display_idx: usize,

    /// Display index of the existing frame to show
    pub existing_frame_display_idx: usize,

    /// Frame to show index (reference buffer slot)
    pub frame_to_show_map_idx: u8,
}

impl ShowExistingFrame {
    /// Create new show existing frame
    pub fn new(
        display_idx: usize,
        existing_frame_display_idx: usize,
        frame_to_show_map_idx: u8,
    ) -> Self {
        Self {
            display_idx,
            existing_frame_display_idx,
            frame_to_show_map_idx,
        }
    }

    /// Get current display index (PUBLIC API)
    pub fn display_idx(&self) -> usize {
        self.display_idx
    }

    /// Get existing frame display index (PUBLIC API)
    pub fn existing_frame_display_idx(&self) -> usize {
        self.existing_frame_display_idx
    }

    /// Get reference buffer slot
    pub fn ref_slot(&self) -> u8 {
        self.frame_to_show_map_idx
    }
}

/// AV1 quirks handler
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Av1Quirks {
    /// Tile-to-superblock mapping (if tiles are used)
    pub tile_mapping: Option<TileSbMapping>,

    /// Film grain parameters
    pub film_grain: Av1FilmGrain,

    /// Show existing frame entries (indexed by display_idx)
    pub show_existing_frames: HashMap<usize, ShowExistingFrame>,

    /// Frame width
    pub width: u32,

    /// Frame height
    pub height: u32,

    /// Superblock size
    pub sb_size: SuperblockSize,
}

impl Av1Quirks {
    /// Create new AV1 quirks handler
    pub fn new(width: u32, height: u32, sb_size: SuperblockSize) -> Self {
        Self {
            tile_mapping: None,
            film_grain: Av1FilmGrain::new(),
            show_existing_frames: HashMap::new(),
            width,
            height,
            sb_size,
        }
    }

    /// Set tile configuration
    pub fn set_tile_config(&mut self, tile_config: Av1TileConfig) {
        self.tile_mapping = Some(TileSbMapping::new(
            self.width,
            self.height,
            self.sb_size,
            tile_config,
        ));
    }

    /// Get tile mapping
    pub fn tile_mapping(&self) -> Option<&TileSbMapping> {
        self.tile_mapping.as_ref()
    }

    /// Set film grain parameters
    pub fn set_film_grain(&mut self, film_grain: Av1FilmGrain) {
        self.film_grain = film_grain;
    }

    /// Get film grain parameters
    pub fn film_grain(&self) -> &Av1FilmGrain {
        &self.film_grain
    }

    /// Register show existing frame
    ///
    /// IMPORTANT: Uses display_idx as PRIMARY identifier
    pub fn register_show_existing_frame(&mut self, show_existing: ShowExistingFrame) {
        self.show_existing_frames
            .insert(show_existing.display_idx, show_existing);
    }

    /// Get show existing frame by display_idx
    pub fn get_show_existing_frame(&self, display_idx: usize) -> Option<&ShowExistingFrame> {
        self.show_existing_frames.get(&display_idx)
    }

    /// Check if frame is a show existing frame
    pub fn is_show_existing_frame(&self, display_idx: usize) -> bool {
        self.show_existing_frames.contains_key(&display_idx)
    }

    /// Get all show existing frames
    pub fn show_existing_frames(&self) -> Vec<&ShowExistingFrame> {
        self.show_existing_frames.values().collect()
    }

    /// Clear all quirks
    pub fn clear(&mut self) {
        self.tile_mapping = None;
        self.film_grain = Av1FilmGrain::new();
        self.show_existing_frames.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tile_config_creation() {
        let config = Av1TileConfig::new(2, 3);

        assert_eq!(config.tile_cols, 2);
        assert_eq!(config.tile_rows, 3);
        assert_eq!(config.tile_count, 6);
    }

    #[test]
    fn test_tile_index() {
        let config = Av1TileConfig::new(2, 3);

        assert_eq!(config.tile_index(0, 0), Some(0));
        assert_eq!(config.tile_index(1, 0), Some(1));
        assert_eq!(config.tile_index(0, 1), Some(2));
        assert_eq!(config.tile_index(1, 2), Some(5));

        // Out of bounds
        assert_eq!(config.tile_index(2, 0), None);
        assert_eq!(config.tile_index(0, 3), None);
    }

    #[test]
    fn test_tile_coords() {
        let config = Av1TileConfig::new(2, 3);

        assert_eq!(config.tile_coords(0), Some((0, 0)));
        assert_eq!(config.tile_coords(1), Some((1, 0)));
        assert_eq!(config.tile_coords(2), Some((0, 1)));
        assert_eq!(config.tile_coords(5), Some((1, 2)));

        // Out of bounds
        assert_eq!(config.tile_coords(6), None);
    }

    #[test]
    fn test_superblock_size() {
        assert_eq!(SuperblockSize::Sb64.size(), 64);
        assert_eq!(SuperblockSize::Sb128.size(), 128);
    }

    #[test]
    fn test_sb_count() {
        assert_eq!(SuperblockSize::Sb64.sb_count(1920), 30);
        assert_eq!(SuperblockSize::Sb64.sb_count(1080), 17);

        assert_eq!(SuperblockSize::Sb128.sb_count(1920), 15);
        assert_eq!(SuperblockSize::Sb128.sb_count(1080), 9);
    }

    #[test]
    fn test_tile_sb_mapping_creation() {
        let tile_config = Av1TileConfig::new(2, 2);
        let mapping = TileSbMapping::new(1920, 1080, SuperblockSize::Sb64, tile_config);

        assert_eq!(mapping.sb_size, SuperblockSize::Sb64);
        assert_eq!(mapping.tile_config.tile_count, 4);
    }

    #[test]
    fn test_tile_sb_bounds() {
        let tile_config = Av1TileConfig::new(2, 2);
        let mapping = TileSbMapping::new(1920, 1080, SuperblockSize::Sb64, tile_config);

        // Each tile should have superblock bounds
        assert!(mapping.get_tile_sb_bounds(0).is_some());
        assert!(mapping.get_tile_sb_bounds(3).is_some());
        assert!(mapping.get_tile_sb_bounds(4).is_none());
    }

    #[test]
    fn test_film_grain_default() {
        let grain = Av1FilmGrain::new();

        assert!(!grain.is_enabled());
        assert!(!grain.should_update());
    }

    #[test]
    fn test_film_grain_enabled() {
        let mut grain = Av1FilmGrain::new();
        grain.apply_grain = true;

        assert!(grain.is_enabled());
    }

    #[test]
    fn test_show_existing_frame_creation() {
        let show_existing = ShowExistingFrame::new(5, 3, 2);

        assert_eq!(show_existing.display_idx(), 5);
        assert_eq!(show_existing.existing_frame_display_idx(), 3);
        assert_eq!(show_existing.ref_slot(), 2);
    }

    #[test]
    fn test_av1_quirks_creation() {
        let quirks = Av1Quirks::new(1920, 1080, SuperblockSize::Sb64);

        assert_eq!(quirks.width, 1920);
        assert_eq!(quirks.height, 1080);
        assert_eq!(quirks.sb_size, SuperblockSize::Sb64);
        assert!(quirks.tile_mapping.is_none());
    }

    #[test]
    fn test_set_tile_config() {
        let mut quirks = Av1Quirks::new(1920, 1080, SuperblockSize::Sb64);

        let tile_config = Av1TileConfig::new(2, 2);
        quirks.set_tile_config(tile_config);

        assert!(quirks.tile_mapping().is_some());
    }

    #[test]
    fn test_set_film_grain() {
        let mut quirks = Av1Quirks::new(1920, 1080, SuperblockSize::Sb64);

        let mut grain = Av1FilmGrain::new();
        grain.apply_grain = true;
        quirks.set_film_grain(grain);

        assert!(quirks.film_grain().is_enabled());
    }

    #[test]
    fn test_register_show_existing_frame() {
        let mut quirks = Av1Quirks::new(1920, 1080, SuperblockSize::Sb64);

        let show_existing = ShowExistingFrame::new(5, 3, 2);
        quirks.register_show_existing_frame(show_existing);

        assert!(quirks.is_show_existing_frame(5));
        assert!(!quirks.is_show_existing_frame(3));
    }

    #[test]
    fn test_get_show_existing_frame() {
        let mut quirks = Av1Quirks::new(1920, 1080, SuperblockSize::Sb64);

        let show_existing = ShowExistingFrame::new(5, 3, 2);
        quirks.register_show_existing_frame(show_existing);

        let retrieved = quirks.get_show_existing_frame(5).unwrap();
        assert_eq!(retrieved.display_idx(), 5);
        assert_eq!(retrieved.existing_frame_display_idx(), 3);
    }

    #[test]
    fn test_show_existing_frames_list() {
        let mut quirks = Av1Quirks::new(1920, 1080, SuperblockSize::Sb64);

        quirks.register_show_existing_frame(ShowExistingFrame::new(5, 3, 2));
        quirks.register_show_existing_frame(ShowExistingFrame::new(7, 4, 1));

        let list = quirks.show_existing_frames();
        assert_eq!(list.len(), 2);
    }

    #[test]
    fn test_clear_quirks() {
        let mut quirks = Av1Quirks::new(1920, 1080, SuperblockSize::Sb64);

        quirks.set_tile_config(Av1TileConfig::new(2, 2));
        quirks.register_show_existing_frame(ShowExistingFrame::new(5, 3, 2));

        quirks.clear();

        assert!(quirks.tile_mapping.is_none());
        assert!(quirks.show_existing_frames.is_empty());
    }

    #[test]
    fn test_frame_identity_contract_show_existing() {
        // Verify FRAME_IDENTITY_CONTRACT: display_idx is PRIMARY for show_existing_frame

        let mut quirks = Av1Quirks::new(1920, 1080, SuperblockSize::Sb64);

        // Register show existing frame at display_idx=5
        let show_existing = ShowExistingFrame::new(5, 3, 2);
        quirks.register_show_existing_frame(show_existing);

        // Query by display_idx (PUBLIC API)
        let frame = quirks.get_show_existing_frame(5).unwrap();
        assert_eq!(frame.display_idx(), 5);

        // Verify it shows frame at display_idx=3
        assert_eq!(frame.existing_frame_display_idx(), 3);

        // Note: decode_idx is never exposed in this API
    }

    #[test]
    fn test_multiple_show_existing_frames() {
        let mut quirks = Av1Quirks::new(1920, 1080, SuperblockSize::Sb64);

        // Frame 10 shows frame 5
        quirks.register_show_existing_frame(ShowExistingFrame::new(10, 5, 0));

        // Frame 15 shows frame 7
        quirks.register_show_existing_frame(ShowExistingFrame::new(15, 7, 1));

        // Frame 20 shows frame 10 (which itself shows frame 5)
        quirks.register_show_existing_frame(ShowExistingFrame::new(20, 10, 2));

        assert!(quirks.is_show_existing_frame(10));
        assert!(quirks.is_show_existing_frame(15));
        assert!(quirks.is_show_existing_frame(20));

        // Verify all use display_idx
        assert_eq!(
            quirks.get_show_existing_frame(10).unwrap().display_idx(),
            10
        );
        assert_eq!(
            quirks.get_show_existing_frame(15).unwrap().display_idx(),
            15
        );
        assert_eq!(
            quirks.get_show_existing_frame(20).unwrap().display_idx(),
            20
        );
    }

    #[test]
    fn test_tile_roundtrip() {
        let tile_config = Av1TileConfig::new(3, 2);

        // Test index -> coords -> index roundtrip
        for tile_idx in 0..tile_config.tile_count {
            let (col, row) = tile_config.tile_coords(tile_idx).unwrap();
            let recovered = tile_config.tile_index(col, row).unwrap();
            assert_eq!(recovered, tile_idx);
        }
    }
}
