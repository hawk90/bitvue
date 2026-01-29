//! Partition Data Loading for Player Workspace
//!
//! Handles loading partition data from JSON files or generating mock data.

use bitvue_core::{PartitionData, PartitionGrid, PartitionKind, partition_grid::PartitionType};

/// Partition data loader
///
/// Loads or generates partition grid data for visualization.
pub struct PartitionLoader;

impl PartitionLoader {
    /// Maximum JSON file size to prevent DoS attacks (10 MB)
    ///
    /// Partition JSON files should be relatively small (typically < 1 MB).
    /// This limit prevents attackers from causing memory exhaustion via
    /// huge malicious JSON files.
    const MAX_JSON_SIZE: u64 = 10 * 1024 * 1024;

    /// Load partition data from JSON file or generate mock data
    ///
    /// Tries to load from a mock JSON file, falling back to procedural generation.
    ///
    /// The mock data file path can be configured via:
    /// 1. Environment variable: `BITVUE_MOCK_PARTITION_DATA`
    /// 2. Default relative path from project root
    pub fn load_partition_data(width: u32, height: u32) -> PartitionData {
        // Try environment variable first, then fallback paths
        let mock_paths = [
            // Environment variable override
            std::env::var("BITVUE_MOCK_PARTITION_DATA").ok(),
            // Try project root relative paths
            Self::get_project_root_path("docs/bitstream_analyzer_monster_pack_v14/docs/mock_data/partition_map_frame120.json"),
            Self::get_project_root_path("test_data/mock_data/partition_map_frame120.json"),
            // Try from current directory
            Some("docs/mock_data/partition_map_frame120.json".to_string()),
            Some("test_data/partition_map_frame120.json".to_string()),
        ];

        // Try each path until one works
        for mock_path in mock_paths.into_iter().flatten() {
            // Validate file size before reading to prevent DoS
            match std::fs::metadata(&mock_path) {
                Ok(metadata) => {
                    let file_size = metadata.len();
                    if file_size > Self::MAX_JSON_SIZE {
                        tracing::warn!(
                            "Skipping partition JSON file (too large): {} bytes > {} bytes max",
                            file_size,
                            Self::MAX_JSON_SIZE
                        );
                        continue;
                    }

                    // File size is safe, proceed with reading
                    match std::fs::read_to_string(&mock_path) {
                        Ok(json_str) => {
                            tracing::debug!("Loading partition data from: {} ({} bytes)", mock_path, file_size);
                            return Self::try_parse_json(json_str, width, height);
                        }
                        Err(e) => {
                            tracing::debug!("Failed to read partition JSON: {}", e);
                            continue;
                        }
                    }
                }
                Err(_) => {
                    // File doesn't exist or can't read metadata, try next path
                    continue;
                }
            }
        }

        // All paths failed, use procedural mock
        tracing::debug!("Mock partition data file not found, using procedural mock");
        Self::create_mock_partition_data(width, height)
    }

    /// Get a path relative to the project root directory
    ///
    /// Attempts to find the project root by looking for marker files
    /// (Cargo.toml, .git directory, etc.)
    fn get_project_root_path(relative_path: &str) -> Option<String> {
        // Start from current executable directory and search upwards
        let current_dir = std::env::current_dir().ok()?;

        // Search for project root markers
        for ancestor in current_dir.ancestors() {
            // Check if this looks like the project root
            let has_cargo_toml = ancestor.join("Cargo.toml").exists();
            let has_git = ancestor.join(".git").exists();
            let has_bitvue_dir = ancestor.join("crates").exists();

            if has_cargo_toml || has_git || has_bitvue_dir {
                let full_path = ancestor.join(relative_path);
                if full_path.exists() {
                    return full_path.to_str().map(|s| s.to_string());
                }
            }
        }

        None
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
        assert!(!data.partition_kind.is_empty());
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

        // At depth 0, splits 4x until depth >= 3 or size <= 16
        // Starting with 64x64 at depth 0:
        // - depth 0: splits to 4x 32x32 (depth 1)
        // - each 32x32 splits to 4x 16x16 (depth 2)
        // - each 16x16 stops (size <= 16) at depth 3
        // Total: 4 (depth 1) * 4 (depth 2) = 16 blocks
        assert_eq!(grid.block_count(), 16);

        // At depth 1 with 64x64, should also split to 16 blocks
        let mut grid = PartitionGrid::new(64, 64, 64);
        PartitionLoader::recursively_partition(&mut grid, 0, 0, 64, 64, 1);
        assert_eq!(grid.block_count(), 16);
    }
}
