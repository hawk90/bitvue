//! Mini Charts - T2-2
//!
//! Per WS_PLAYER_SPATIAL:
//! - Block size distribution chart (current frame)
//! - MV magnitude histogram (current frame)
//! - Charts update with SelectionState.temporal
//!
//! Acceptance Test WS23: Mini charts update on frame change

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Block size in AV1
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BlockSize {
    /// 4x4
    Block4x4,
    /// 8x8
    Block8x8,
    /// 16x16
    Block16x16,
    /// 32x32
    Block32x32,
    /// 64x64
    Block64x64,
    /// 128x128 (superblock)
    Block128x128,
    /// Other rectangular sizes
    Other,
}

impl BlockSize {
    /// Get display name
    pub fn name(&self) -> &'static str {
        match self {
            BlockSize::Block4x4 => "4x4",
            BlockSize::Block8x8 => "8x8",
            BlockSize::Block16x16 => "16x16",
            BlockSize::Block32x32 => "32x32",
            BlockSize::Block64x64 => "64x64",
            BlockSize::Block128x128 => "128x128",
            BlockSize::Other => "Other",
        }
    }

    /// Get area in pixels
    pub fn area(&self) -> u32 {
        match self {
            BlockSize::Block4x4 => 16,
            BlockSize::Block8x8 => 64,
            BlockSize::Block16x16 => 256,
            BlockSize::Block32x32 => 1024,
            BlockSize::Block64x64 => 4096,
            BlockSize::Block128x128 => 16384,
            BlockSize::Other => 0,
        }
    }
}

/// Block size distribution for a frame
///
/// Shows the distribution of block sizes used in encoding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockSizeDistribution {
    /// Frame index
    pub frame_idx: usize,

    /// Count per block size
    pub counts: HashMap<BlockSize, u32>,

    /// Total block count
    pub total_blocks: u32,
}

impl BlockSizeDistribution {
    /// Create a new empty distribution
    pub fn new(frame_idx: usize) -> Self {
        Self {
            frame_idx,
            counts: HashMap::new(),
            total_blocks: 0,
        }
    }

    /// Add a block to the distribution
    pub fn add_block(&mut self, size: BlockSize) {
        *self.counts.entry(size).or_insert(0) += 1;
        self.total_blocks += 1;
    }

    /// Get count for a block size
    pub fn get_count(&self, size: BlockSize) -> u32 {
        self.counts.get(&size).copied().unwrap_or(0)
    }

    /// Get percentage for a block size
    pub fn get_percentage(&self, size: BlockSize) -> f32 {
        if self.total_blocks == 0 {
            0.0
        } else {
            (self.get_count(size) as f32 / self.total_blocks as f32) * 100.0
        }
    }

    /// Get all sizes sorted by count (descending)
    pub fn sizes_by_count(&self) -> Vec<(BlockSize, u32)> {
        let mut sizes: Vec<_> = self.counts.iter().map(|(k, v)| (*k, *v)).collect();
        sizes.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
        sizes
    }

    /// Get chart data (labels and values)
    pub fn chart_data(&self) -> (Vec<&'static str>, Vec<f32>) {
        let sizes = [
            BlockSize::Block4x4,
            BlockSize::Block8x8,
            BlockSize::Block16x16,
            BlockSize::Block32x32,
            BlockSize::Block64x64,
            BlockSize::Block128x128,
        ];

        let labels: Vec<_> = sizes.iter().map(|s| s.name()).collect();
        let values: Vec<_> = sizes.iter().map(|s| self.get_percentage(*s)).collect();

        (labels, values)
    }
}

impl Default for BlockSizeDistribution {
    fn default() -> Self {
        Self::new(0)
    }
}

/// Motion vector magnitude range
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MvMagnitudeRange {
    /// 0 pixels (static)
    Static,
    /// 0-2 pixels
    Range0to2,
    /// 2-4 pixels
    Range2to4,
    /// 4-8 pixels
    Range4to8,
    /// 8-16 pixels
    Range8to16,
    /// 16-32 pixels
    Range16to32,
    /// 32+ pixels
    Range32Plus,
}

impl MvMagnitudeRange {
    /// Get display name
    pub fn name(&self) -> &'static str {
        match self {
            MvMagnitudeRange::Static => "0",
            MvMagnitudeRange::Range0to2 => "0-2",
            MvMagnitudeRange::Range2to4 => "2-4",
            MvMagnitudeRange::Range4to8 => "4-8",
            MvMagnitudeRange::Range8to16 => "8-16",
            MvMagnitudeRange::Range16to32 => "16-32",
            MvMagnitudeRange::Range32Plus => "32+",
        }
    }

    /// Get range from magnitude
    pub fn from_magnitude(magnitude: f32) -> Self {
        if magnitude < 0.01 {
            MvMagnitudeRange::Static
        } else if magnitude < 2.0 {
            MvMagnitudeRange::Range0to2
        } else if magnitude < 4.0 {
            MvMagnitudeRange::Range2to4
        } else if magnitude < 8.0 {
            MvMagnitudeRange::Range4to8
        } else if magnitude < 16.0 {
            MvMagnitudeRange::Range8to16
        } else if magnitude < 32.0 {
            MvMagnitudeRange::Range16to32
        } else {
            MvMagnitudeRange::Range32Plus
        }
    }
}

/// MV magnitude histogram for a frame
///
/// Shows the distribution of motion vector magnitudes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MvMagnitudeHistogram {
    /// Frame index
    pub frame_idx: usize,

    /// Count per magnitude range
    pub counts: HashMap<MvMagnitudeRange, u32>,

    /// Total MV count
    pub total_mvs: u32,

    /// Average magnitude
    pub avg_magnitude: f32,

    /// Max magnitude
    pub max_magnitude: f32,
}

impl MvMagnitudeHistogram {
    /// Create a new empty histogram
    pub fn new(frame_idx: usize) -> Self {
        Self {
            frame_idx,
            counts: HashMap::new(),
            total_mvs: 0,
            avg_magnitude: 0.0,
            max_magnitude: 0.0,
        }
    }

    /// Add a motion vector to the histogram
    pub fn add_mv(&mut self, dx: f32, dy: f32) {
        let magnitude = (dx * dx + dy * dy).sqrt();
        let range = MvMagnitudeRange::from_magnitude(magnitude);

        *self.counts.entry(range).or_insert(0) += 1;
        self.total_mvs += 1;

        // Update statistics
        if magnitude > self.max_magnitude {
            self.max_magnitude = magnitude;
        }

        // Update average (incremental calculation)
        let old_avg = self.avg_magnitude;
        self.avg_magnitude = old_avg + (magnitude - old_avg) / self.total_mvs as f32;
    }

    /// Get count for a magnitude range
    pub fn get_count(&self, range: MvMagnitudeRange) -> u32 {
        self.counts.get(&range).copied().unwrap_or(0)
    }

    /// Get percentage for a magnitude range
    pub fn get_percentage(&self, range: MvMagnitudeRange) -> f32 {
        if self.total_mvs == 0 {
            0.0
        } else {
            (self.get_count(range) as f32 / self.total_mvs as f32) * 100.0
        }
    }

    /// Get chart data (labels and values)
    pub fn chart_data(&self) -> (Vec<&'static str>, Vec<f32>) {
        let ranges = [
            MvMagnitudeRange::Static,
            MvMagnitudeRange::Range0to2,
            MvMagnitudeRange::Range2to4,
            MvMagnitudeRange::Range4to8,
            MvMagnitudeRange::Range8to16,
            MvMagnitudeRange::Range16to32,
            MvMagnitudeRange::Range32Plus,
        ];

        let labels: Vec<_> = ranges.iter().map(|r| r.name()).collect();
        let values: Vec<_> = ranges.iter().map(|r| self.get_percentage(*r)).collect();

        (labels, values)
    }
}

impl Default for MvMagnitudeHistogram {
    fn default() -> Self {
        Self::new(0)
    }
}

/// Mini charts data
///
/// Contains both block size distribution and MV histogram for a frame
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiniChartsData {
    /// Frame index
    pub frame_idx: usize,

    /// Block size distribution
    pub block_distribution: BlockSizeDistribution,

    /// MV magnitude histogram
    pub mv_histogram: MvMagnitudeHistogram,
}

impl MiniChartsData {
    /// Create new mini charts data for a frame
    pub fn new(frame_idx: usize) -> Self {
        Self {
            frame_idx,
            block_distribution: BlockSizeDistribution::new(frame_idx),
            mv_histogram: MvMagnitudeHistogram::new(frame_idx),
        }
    }

    /// Update with new frame index
    pub fn update_frame(&mut self, frame_idx: usize) {
        self.frame_idx = frame_idx;
        self.block_distribution = BlockSizeDistribution::new(frame_idx);
        self.mv_histogram = MvMagnitudeHistogram::new(frame_idx);
    }
}

impl Default for MiniChartsData {
    fn default() -> Self {
        Self::new(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_block_size() {
        assert_eq!(BlockSize::Block4x4.name(), "4x4");
        assert_eq!(BlockSize::Block4x4.area(), 16);

        assert_eq!(BlockSize::Block16x16.name(), "16x16");
        assert_eq!(BlockSize::Block16x16.area(), 256);

        assert_eq!(BlockSize::Block128x128.name(), "128x128");
        assert_eq!(BlockSize::Block128x128.area(), 16384);
    }

    #[test]
    fn test_block_size_distribution() {
        let mut dist = BlockSizeDistribution::new(10);

        dist.add_block(BlockSize::Block8x8);
        dist.add_block(BlockSize::Block8x8);
        dist.add_block(BlockSize::Block16x16);

        assert_eq!(dist.total_blocks, 3);
        assert_eq!(dist.get_count(BlockSize::Block8x8), 2);
        assert_eq!(dist.get_count(BlockSize::Block16x16), 1);
        assert_eq!(dist.get_count(BlockSize::Block32x32), 0);

        // Check percentages
        assert!((dist.get_percentage(BlockSize::Block8x8) - 66.666).abs() < 0.01);
        assert!((dist.get_percentage(BlockSize::Block16x16) - 33.333).abs() < 0.01);
    }

    #[test]
    fn test_block_size_distribution_chart_data() {
        let mut dist = BlockSizeDistribution::new(10);

        dist.add_block(BlockSize::Block8x8);
        dist.add_block(BlockSize::Block8x8);
        dist.add_block(BlockSize::Block16x16);

        let (labels, values) = dist.chart_data();
        assert_eq!(labels.len(), 6);
        assert_eq!(values.len(), 6);

        assert!(labels.contains(&"8x8"));
        assert!(labels.contains(&"16x16"));

        // 8x8 should have ~66.67%
        let idx_8x8 = labels.iter().position(|&l| l == "8x8").unwrap();
        assert!((values[idx_8x8] - 66.666).abs() < 0.01);
    }

    #[test]
    fn test_mv_magnitude_range() {
        assert_eq!(
            MvMagnitudeRange::from_magnitude(0.0),
            MvMagnitudeRange::Static
        );
        assert_eq!(
            MvMagnitudeRange::from_magnitude(1.5),
            MvMagnitudeRange::Range0to2
        );
        assert_eq!(
            MvMagnitudeRange::from_magnitude(3.0),
            MvMagnitudeRange::Range2to4
        );
        assert_eq!(
            MvMagnitudeRange::from_magnitude(6.0),
            MvMagnitudeRange::Range4to8
        );
        assert_eq!(
            MvMagnitudeRange::from_magnitude(12.0),
            MvMagnitudeRange::Range8to16
        );
        assert_eq!(
            MvMagnitudeRange::from_magnitude(24.0),
            MvMagnitudeRange::Range16to32
        );
        assert_eq!(
            MvMagnitudeRange::from_magnitude(50.0),
            MvMagnitudeRange::Range32Plus
        );
    }

    #[test]
    fn test_mv_magnitude_histogram() {
        let mut hist = MvMagnitudeHistogram::new(10);

        hist.add_mv(0.0, 0.0); // Static
        hist.add_mv(1.0, 1.0); // ~1.41 pixels (Range0to2)
        hist.add_mv(3.0, 0.0); // 3.0 pixels (Range2to4)

        assert_eq!(hist.total_mvs, 3);
        assert_eq!(hist.get_count(MvMagnitudeRange::Static), 1);
        assert_eq!(hist.get_count(MvMagnitudeRange::Range0to2), 1);
        assert_eq!(hist.get_count(MvMagnitudeRange::Range2to4), 1);

        // Check statistics
        assert!(hist.avg_magnitude > 0.0);
        assert!(hist.max_magnitude >= 3.0);
    }

    #[test]
    fn test_mv_magnitude_histogram_percentages() {
        let mut hist = MvMagnitudeHistogram::new(10);

        hist.add_mv(0.0, 0.0); // Static
        hist.add_mv(0.0, 0.0); // Static
        hist.add_mv(1.0, 1.0); // Range0to2

        assert_eq!(hist.total_mvs, 3);

        // Static: 2/3 = 66.67%
        assert!((hist.get_percentage(MvMagnitudeRange::Static) - 66.666).abs() < 0.01);

        // Range0to2: 1/3 = 33.33%
        assert!((hist.get_percentage(MvMagnitudeRange::Range0to2) - 33.333).abs() < 0.01);
    }

    #[test]
    fn test_mv_magnitude_histogram_chart_data() {
        let mut hist = MvMagnitudeHistogram::new(10);

        hist.add_mv(0.0, 0.0); // Static
        hist.add_mv(1.0, 1.0); // Range0to2
        hist.add_mv(3.0, 0.0); // Range2to4

        let (labels, values) = hist.chart_data();
        assert_eq!(labels.len(), 7);
        assert_eq!(values.len(), 7);

        assert!(labels.contains(&"0"));
        assert!(labels.contains(&"0-2"));
        assert!(labels.contains(&"2-4"));

        // Each should have ~33.33%
        let idx_static = labels.iter().position(|&l| l == "0").unwrap();
        assert!((values[idx_static] - 33.333).abs() < 0.01);
    }

    #[test]
    fn test_mini_charts_data() {
        let mut charts = MiniChartsData::new(10);

        // Add some block data
        charts.block_distribution.add_block(BlockSize::Block8x8);
        charts.block_distribution.add_block(BlockSize::Block16x16);

        // Add some MV data
        charts.mv_histogram.add_mv(1.0, 1.0);
        charts.mv_histogram.add_mv(5.0, 5.0);

        assert_eq!(charts.frame_idx, 10);
        assert_eq!(charts.block_distribution.total_blocks, 2);
        assert_eq!(charts.mv_histogram.total_mvs, 2);

        // Update to new frame
        charts.update_frame(20);
        assert_eq!(charts.frame_idx, 20);
        assert_eq!(charts.block_distribution.total_blocks, 0); // Reset
        assert_eq!(charts.mv_histogram.total_mvs, 0); // Reset
    }

    #[test]
    fn test_ws23_acceptance_charts_update_on_frame_change() {
        // WS23: Mini charts update on frame change
        let mut charts = MiniChartsData::new(0);

        // Frame 0 data
        charts.block_distribution.add_block(BlockSize::Block8x8);
        charts.mv_histogram.add_mv(1.0, 1.0);

        assert_eq!(charts.frame_idx, 0);
        assert_eq!(charts.block_distribution.total_blocks, 1);
        assert_eq!(charts.mv_histogram.total_mvs, 1);

        // Update to frame 1 - charts should reset
        charts.update_frame(1);

        assert_eq!(charts.frame_idx, 1);
        assert_eq!(
            charts.block_distribution.total_blocks, 0,
            "WS23: Charts must reset on frame change"
        );
        assert_eq!(
            charts.mv_histogram.total_mvs, 0,
            "WS23: Charts must reset on frame change"
        );

        // Add new frame data
        charts.block_distribution.add_block(BlockSize::Block16x16);
        charts.mv_histogram.add_mv(5.0, 5.0);

        assert_eq!(charts.block_distribution.total_blocks, 1);
        assert_eq!(charts.mv_histogram.total_mvs, 1);
    }
}
