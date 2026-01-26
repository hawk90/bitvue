//! Partition Grid Overlay - T3-3
//!
//! Stores partition tree data for visualization.
//! Represents the hierarchical block structure of a coded frame.

use serde::{Deserialize, Serialize};

/// Grid display mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GridMode {
    /// Scaffold mode: uniform grid at base block size
    Scaffold,
    /// Partition mode: show actual partition boundaries
    Partition,
}

/// Partition kind (block coding type)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PartitionKind {
    /// Intra-coded block
    Intra,
    /// Inter-coded block
    Inter,
    /// Skipped block
    Skip,
    /// Split block
    Split,
}

impl PartitionKind {
    /// Get tint color for visualization (RGB 0-255)
    pub fn tint_color(&self) -> (u8, u8, u8) {
        match self {
            PartitionKind::Intra => (100, 150, 255), // Blue
            PartitionKind::Inter => (255, 150, 100), // Orange
            PartitionKind::Skip => (150, 255, 150),  // Green
            PartitionKind::Split => (255, 255, 100), // Yellow
        }
    }
}

/// Partition data for visualization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartitionData {
    /// Frame width in pixels
    pub width: u32,
    /// Frame height in pixels
    pub height: u32,
    /// Leaf block width
    pub leaf_block_w: u32,
    /// Leaf block height
    pub leaf_block_h: u32,
    /// Partition kind for each leaf block (row-major order)
    pub partition_kind: Vec<PartitionKind>,
}

impl PartitionData {
    /// Create new partition data
    pub fn new(
        width: u32,
        height: u32,
        leaf_block_w: u32,
        leaf_block_h: u32,
        partition_kind: Vec<PartitionKind>,
    ) -> Self {
        Self {
            width,
            height,
            leaf_block_w,
            leaf_block_h,
            partition_kind,
        }
    }

    /// Get grid width (number of leaf blocks horizontally)
    pub fn grid_w(&self) -> u32 {
        self.width.div_ceil(self.leaf_block_w)
    }

    /// Get grid height (number of leaf blocks vertically)
    pub fn grid_h(&self) -> u32 {
        self.height.div_ceil(self.leaf_block_h)
    }

    /// Get partition kind at grid position
    pub fn get(&self, grid_x: u32, grid_y: u32) -> Option<PartitionKind> {
        let grid_w = self.grid_w();
        let idx = (grid_y * grid_w + grid_x) as usize;
        self.partition_kind.get(idx).copied()
    }

    /// Get partition kind at grid position (alias for compatibility)
    pub fn get_kind(&self, grid_x: u32, grid_y: u32) -> Option<PartitionKind> {
        self.get(grid_x, grid_y)
    }

    /// Get cell bounds in pixel coordinates
    pub fn cell_bounds(&self, grid_x: u32, grid_y: u32) -> (u32, u32, u32, u32) {
        let x = grid_x * self.leaf_block_w;
        let y = grid_y * self.leaf_block_h;
        let w = self.leaf_block_w.min(self.width.saturating_sub(x));
        let h = self.leaf_block_h.min(self.height.saturating_sub(y));
        (x, y, w, h)
    }
}

/// Partition type for visualization
///
/// Matches AV1 partition types but codec-agnostic
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum PartitionType {
    /// No partition (leaf block)
    None = 0,
    /// Horizontal split
    Horz = 1,
    /// Vertical split
    Vert = 2,
    /// 4-way split
    Split = 3,
    /// Horizontal A (top split)
    HorzA = 4,
    /// Horizontal B (bottom split)
    HorzB = 5,
    /// Vertical A (left split)
    VertA = 6,
    /// Vertical B (right split)
    VertB = 7,
    /// 4-way horizontal
    Horz4 = 8,
    /// 4-way vertical
    Vert4 = 9,
}

impl From<u8> for PartitionType {
    fn from(value: u8) -> Self {
        match value {
            0 => PartitionType::None,
            1 => PartitionType::Horz,
            2 => PartitionType::Vert,
            3 => PartitionType::Split,
            4 => PartitionType::HorzA,
            5 => PartitionType::HorzB,
            6 => PartitionType::VertA,
            7 => PartitionType::VertB,
            8 => PartitionType::Horz4,
            9 => PartitionType::Vert4,
            _ => PartitionType::None,
        }
    }
}

/// Block in partition grid
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartitionBlock {
    /// Block X position in pixels
    pub x: u32,
    /// Block Y position in pixels
    pub y: u32,
    /// Block width in pixels
    pub width: u32,
    /// Block height in pixels
    pub height: u32,
    /// Partition type of parent (how this block was created)
    pub partition: PartitionType,
    /// Nesting depth (0 = superblock, increases with splits)
    pub depth: u8,
}

impl PartitionBlock {
    /// Create a new partition block
    pub fn new(
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        partition: PartitionType,
        depth: u8,
    ) -> Self {
        Self {
            x,
            y,
            width,
            height,
            partition,
            depth,
        }
    }

    /// Check if point is inside this block
    pub fn contains(&self, px: u32, py: u32) -> bool {
        px >= self.x && px < self.x + self.width && py >= self.y && py < self.y + self.height
    }

    /// Get block area in pixels
    pub fn area(&self) -> u32 {
        self.width * self.height
    }
}

/// Partition grid for a coded frame
///
/// Stores flattened partition tree as a list of leaf blocks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartitionGrid {
    /// Coded frame width in pixels
    pub coded_width: u32,
    /// Coded frame height in pixels
    pub coded_height: u32,
    /// Superblock size (typically 64 or 128)
    pub sb_size: u32,
    /// All leaf blocks (coding units) in the frame
    pub blocks: Vec<PartitionBlock>,
}

impl PartitionGrid {
    /// Create a new partition grid
    pub fn new(coded_width: u32, coded_height: u32, sb_size: u32) -> Self {
        Self {
            coded_width,
            coded_height,
            sb_size,
            blocks: Vec::new(),
        }
    }

    /// Add a block to the grid
    pub fn add_block(&mut self, block: PartitionBlock) {
        self.blocks.push(block);
    }

    /// Get block at pixel position
    pub fn block_at(&self, px: u32, py: u32) -> Option<&PartitionBlock> {
        self.blocks.iter().find(|b| b.contains(px, py))
    }

    /// Get total number of blocks
    pub fn block_count(&self) -> usize {
        self.blocks.len()
    }

    /// Get blocks within viewport
    pub fn blocks_in_viewport(
        &self,
        vp_x: u32,
        vp_y: u32,
        vp_w: u32,
        vp_h: u32,
    ) -> Vec<&PartitionBlock> {
        let vp_right = vp_x.saturating_add(vp_w);
        let vp_bottom = vp_y.saturating_add(vp_h);

        self.blocks
            .iter()
            .filter(|b| {
                let b_right = b.x.saturating_add(b.width);
                let b_bottom = b.y.saturating_add(b.height);
                // Check overlap
                !(b_right <= vp_x || b.x >= vp_right || b_bottom <= vp_y || b.y >= vp_bottom)
            })
            .collect()
    }

    /// Create scaffold grid (uniform blocks, no actual partitioning)
    ///
    /// Useful for displaying grid overlay without real partition data
    pub fn create_scaffold(coded_width: u32, coded_height: u32, block_size: u32) -> Self {
        let mut grid = Self::new(coded_width, coded_height, block_size);

        let cols = coded_width.div_ceil(block_size);
        let rows = coded_height.div_ceil(block_size);

        for row in 0..rows {
            for col in 0..cols {
                let x = col * block_size;
                let y = row * block_size;
                let w = block_size.min(coded_width.saturating_sub(x));
                let h = block_size.min(coded_height.saturating_sub(y));

                grid.add_block(PartitionBlock::new(x, y, w, h, PartitionType::None, 0));
            }
        }

        grid
    }

    /// Get partition statistics
    pub fn statistics(&self) -> PartitionStatistics {
        let mut stats = PartitionStatistics::default();

        for block in &self.blocks {
            stats.total_blocks += 1;
            stats.total_area += block.area() as u64;

            // Count by partition type
            match block.partition {
                PartitionType::None => stats.none_count += 1,
                PartitionType::Horz
                | PartitionType::HorzA
                | PartitionType::HorzB
                | PartitionType::Horz4 => {
                    stats.horz_count += 1;
                }
                PartitionType::Vert
                | PartitionType::VertA
                | PartitionType::VertB
                | PartitionType::Vert4 => {
                    stats.vert_count += 1;
                }
                PartitionType::Split => stats.split_count += 1,
            }

            // Track depth distribution
            if block.depth as usize >= stats.depth_counts.len() {
                stats.depth_counts.resize(block.depth as usize + 1, 0);
            }
            stats.depth_counts[block.depth as usize] += 1;

            // Track min/max block size
            let area = block.area();
            if stats.min_block_area == 0 || area < stats.min_block_area {
                stats.min_block_area = area;
            }
            if area > stats.max_block_area {
                stats.max_block_area = area;
            }
        }

        // Calculate average
        if stats.total_blocks > 0 {
            stats.avg_block_area = stats.total_area as f32 / stats.total_blocks as f32;
        }

        stats
    }

    /// Generate cache key for this partition grid
    pub fn cache_key(&self, stream_id: &str, frame_idx: usize) -> String {
        format!(
            "partition:{}:f{}|{}x{}|sb{}|n{}",
            stream_id,
            frame_idx,
            self.coded_width,
            self.coded_height,
            self.sb_size,
            self.blocks.len()
        )
    }
}

/// Partition statistics for UI display
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PartitionStatistics {
    /// Total number of leaf blocks
    pub total_blocks: usize,
    /// Total area covered (should equal frame area)
    pub total_area: u64,
    /// Average block area in pixels
    pub avg_block_area: f32,
    /// Minimum block area
    pub min_block_area: u32,
    /// Maximum block area
    pub max_block_area: u32,
    /// Blocks created with no partition (superblock-level)
    pub none_count: usize,
    /// Blocks created with horizontal partition
    pub horz_count: usize,
    /// Blocks created with vertical partition
    pub vert_count: usize,
    /// Blocks created with 4-way split
    pub split_count: usize,
    /// Count of blocks at each depth level
    pub depth_counts: Vec<usize>,
}

impl PartitionStatistics {
    /// Get average depth
    pub fn avg_depth(&self) -> f32 {
        if self.total_blocks == 0 {
            return 0.0;
        }

        let mut sum = 0usize;
        for (depth, &count) in self.depth_counts.iter().enumerate() {
            sum += depth * count;
        }
        sum as f32 / self.total_blocks as f32
    }

    /// Get max depth
    pub fn max_depth(&self) -> usize {
        if self.depth_counts.is_empty() {
            0
        } else {
            self.depth_counts.len() - 1
        }
    }

    /// Format as summary string
    pub fn summary(&self) -> String {
        format!(
            "{} blocks, avg {:.0}px², depth {:.1} (max {})",
            self.total_blocks,
            self.avg_block_area,
            self.avg_depth(),
            self.max_depth()
        )
    }
}

/// Per generate-tests skill: Comprehensive test suite with Arrange-Act-Assert pattern
#[cfg(test)]
mod tests {
    include!("partition_grid_test.rs");
}
