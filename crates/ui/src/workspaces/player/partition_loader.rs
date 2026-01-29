//! Partition Data Loading for Player Workspace
//!
//! Handles loading partition data from JSON files or generating mock data.

use bitvue_core::{PartitionData, PartitionGrid, PartitionKind, partition_grid::PartitionType};

/// Partition data loader
///
/// Loads or generates partition grid data for visualization.
pub struct PartitionLoader;

impl PartitionLoader {
    /// Load partition data from JSON file or generate mock data
    ///
    /// Tries to load from a mock JSON file, falling back to procedural generation.
    pub fn load_partition_data(width: u32, height: u32) -> PartitionData {
        // Try to load from mock data file
        let mock_path = "docs/bitstream_analyzer_monster_pack_v14/docs/mock_data/partition_map_frame120.json";
        match std::fs::read_to_string(mock_path) {
            Ok(json_str) => Self::try_parse_json(json_str, width, height),
            Err(_) => {
                // Mock file not found, use procedural mock
                tracing::debug!("Mock partition data file not found, using procedural mock");
                Self::create_mock_partition_data(width, height)
            }
        }
    }

    /// Load partition grid (hierarchical blocks)
    ///
    /// Currently generates procedural mock data.
    /// TODO: Load from parser data
    pub fn load_partition_grid(width: u32, height: u32) -> PartitionGrid {
        use bitvue_core::partition_grid::PartitionGrid;

        let sb_size = 64;
        let mut grid = PartitionGrid::new(width, height, sb_size);

        // Generate hierarchical partition tree
        // Superblocks are recursively split into smaller blocks
        for sb_y in (0..height).step_by(sb_size as usize) {
            for sb_x in (0..width).step_by(sb_size as usize) {
                // Each superblock gets recursively partitioned
                Self::recursively_partition(
                    &mut grid,
                    sb_x,
                    sb_y,
                    sb_size.min(width - sb_x),
                    sb_size.min(height - sb_y),
                    0, // depth
                );
            }
        }

        tracing::info!("Created mock partition grid: {}x{}", width, height);
        grid
    }

    /// Try to parse partition JSON data
    fn try_parse_json(json_str: String, frame_width: u32, frame_height: u32) -> PartitionData {
        #[derive(serde::Deserialize)]
        struct PartitionMapJson {
            coded_width: u32,
            coded_height: u32,
            leaf_block_w: u32,
            leaf_block_h: u32,
            #[allow(dead_code)]
            grid_w: u32,
            #[allow(dead_code)]
            grid_h: u32,
            part_kind: Vec<u8>,
        }

        match serde_json::from_str::<PartitionMapJson>(&json_str) {
            Ok(json_data) => {
                // Check if dimensions match
                if json_data.coded_width == frame_width && json_data.coded_height == frame_height {
                    // Convert u8 values to PartitionKind
                    let part_kind: Vec<PartitionKind> = json_data
                        .part_kind
                        .iter()
                        .map(|&val| match val {
                            1 => PartitionKind::Intra,
                            2 => PartitionKind::Inter,
                            3 => PartitionKind::Split,
                            4 => PartitionKind::Skip,
                            _ => PartitionKind::Inter, // Default
                        })
                        .collect();

                    tracing::info!("Loaded partition data from JSON: {}x{}", frame_width, frame_height);
                    PartitionData::new(
                        json_data.coded_width,
                        json_data.coded_height,
                        json_data.leaf_block_w,
                        json_data.leaf_block_h,
                        part_kind,
                    )
                } else {
                    tracing::warn!(
                        "Partition data dimensions mismatch: JSON {}x{}, frame {}x{}",
                        json_data.coded_width,
                        json_data.coded_height,
                        frame_width,
                        frame_height
                    );
                    Self::create_mock_partition_data(frame_width, frame_height)
                }
            }
            Err(e) => {
                tracing::warn!("Failed to parse partition JSON: {}", e);
                Self::create_mock_partition_data(frame_width, frame_height)
            }
        }
    }

    /// Create mock partition data (procedural generation)
    pub fn create_mock_partition_data(width: u32, height: u32) -> PartitionData {
        const BLOCK_SIZE: u32 = 16;
        let grid_w = (width + BLOCK_SIZE - 1) / BLOCK_SIZE;
        let grid_h = (height + BLOCK_SIZE - 1) / BLOCK_SIZE;
        let total_blocks = grid_w * grid_h;

        // Generate alternating pattern of Inter/Intra blocks
        let part_kind: Vec<PartitionKind> = (0..total_blocks)
            .map(|i| {
                let row = i / grid_w;
                let col = i % grid_w;
                // Create a checkerboard-like pattern
                if (row + col) % 2 == 0 {
                    PartitionKind::Inter
                } else {
                    PartitionKind::Intra
                }
            })
            .collect();

        PartitionData::new(width, height, BLOCK_SIZE, BLOCK_SIZE, part_kind)
    }

    /// Recursively partition a region and add blocks to the grid
    fn recursively_partition(
        grid: &mut PartitionGrid,
        x: u32,
        y: u32,
        w: u32,
        h: u32,
        depth: u8,
    ) {
        use bitvue_core::PartitionBlock;

        if depth >= 3 || w <= 16 || h <= 16 {
            // Leaf block - add to grid
            let block = PartitionBlock::new(
                x, y, w, h,
                PartitionType::None,
                depth,
            );
            grid.add_block(block);
        } else {
            // Split into 4 quadrants
            let half_w = w / 2;
            let half_h = h / 2;

            Self::recursively_partition(grid, x, y, half_w, half_h, depth + 1);
            Self::recursively_partition(grid, x + half_w, y, w - half_w, half_h, depth + 1);
            Self::recursively_partition(grid, x, y + half_h, half_w, h - half_h, depth + 1);
            Self::recursively_partition(
                grid,
                x + half_w,
                y + half_h,
                w - half_w,
                h - half_h,
                depth + 1,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_mock_partition_data() {
        let data = PartitionLoader::create_mock_partition_data(1920, 1080);
        assert_eq!(data.width, 1920);
        assert_eq!(data.height, 1080);
        assert!(!data.part_kind.is_empty());
    }

    #[test]
    fn test_create_mock_partition_grid() {
        let grid = PartitionLoader::load_partition_grid(1920, 1080);
        // Grid should have been created with blocks
        assert!(grid.block_count() > 0);
    }

    #[test]
    fn test_recursive_partition_depth() {
        use bitvue_core::partition_grid::PartitionGrid;

        let mut grid = PartitionGrid::new(64, 64, 64);
        PartitionLoader::recursively_partition(&mut grid, 0, 0, 64, 64, 0);

        // At depth 0, should add a single block
        assert_eq!(grid.block_count(), 1);

        // At depth 1, should split into 4 blocks
        let mut grid = PartitionGrid::new(64, 64, 64);
        PartitionLoader::recursively_partition(&mut grid, 0, 0, 64, 64, 1);
        assert_eq!(grid.block_count(), 4);
    }
}
