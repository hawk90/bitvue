//! Block-Level Metrics - Feature Parity: Per-block metric map (PSNR/SSIM)
//!
//! Per COMPETITOR_PARITY_STATUS.md §4.2:
//! - Per-block metric map (PSNR/SSIM) - for spatial overlay visualization
//!
//! Implements:
//! - Block-level PSNR/SSIM calculation
//! - Block metrics grid for heatmap visualization
//! - Integration with compare overlay system

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ═══════════════════════════════════════════════════════════════════════════
// Block Metric Types
// ═══════════════════════════════════════════════════════════════════════════

/// Type of metric to compute
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum BlockMetricType {
    /// Peak Signal-to-Noise Ratio
    #[default]
    Psnr,
    /// Structural Similarity Index
    Ssim,
    /// Mean Squared Error
    Mse,
    /// Mean Absolute Difference
    Mad,
}

impl BlockMetricType {
    pub fn name(&self) -> &'static str {
        match self {
            BlockMetricType::Psnr => "PSNR",
            BlockMetricType::Ssim => "SSIM",
            BlockMetricType::Mse => "MSE",
            BlockMetricType::Mad => "MAD",
        }
    }

    pub fn unit(&self) -> &'static str {
        match self {
            BlockMetricType::Psnr => "dB",
            BlockMetricType::Ssim => "",
            BlockMetricType::Mse => "",
            BlockMetricType::Mad => "",
        }
    }

    /// Value range for color mapping
    pub fn typical_range(&self) -> (f32, f32) {
        match self {
            BlockMetricType::Psnr => (20.0, 50.0), // dB
            BlockMetricType::Ssim => (0.0, 1.0),   // index
            BlockMetricType::Mse => (0.0, 1000.0), // squared diff
            BlockMetricType::Mad => (0.0, 100.0),  // abs diff
        }
    }

    /// Higher is better?
    pub fn higher_is_better(&self) -> bool {
        matches!(self, BlockMetricType::Psnr | BlockMetricType::Ssim)
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Block Metric Value
// ═══════════════════════════════════════════════════════════════════════════

/// Single block metric value
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BlockMetricValue {
    /// X position in blocks
    pub block_x: u32,
    /// Y position in blocks
    pub block_y: u32,
    /// Metric value
    pub value: f32,
    /// Metric type
    pub metric_type: BlockMetricType,
}

impl BlockMetricValue {
    pub fn new(block_x: u32, block_y: u32, value: f32, metric_type: BlockMetricType) -> Self {
        Self {
            block_x,
            block_y,
            value,
            metric_type,
        }
    }

    /// Normalize value to 0.0-1.0 range based on metric type
    pub fn normalized(&self) -> f32 {
        let (min, max) = self.metric_type.typical_range();
        if max > min {
            ((self.value - min) / (max - min)).clamp(0.0, 1.0)
        } else {
            0.5
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Block Metrics Grid
// ═══════════════════════════════════════════════════════════════════════════

/// Grid of block-level metrics for a frame
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockMetricsGrid {
    /// Frame index (display order)
    pub display_idx: usize,
    /// Block size used for calculation
    pub block_size: u32,
    /// Width in blocks
    pub width_blocks: u32,
    /// Height in blocks
    pub height_blocks: u32,
    /// Frame width in pixels
    pub frame_width: u32,
    /// Frame height in pixels
    pub frame_height: u32,
    /// Metric type
    pub metric_type: BlockMetricType,
    /// Block values (row-major order)
    pub values: Vec<f32>,
}

impl BlockMetricsGrid {
    pub fn new(
        display_idx: usize,
        frame_width: u32,
        frame_height: u32,
        block_size: u32,
        metric_type: BlockMetricType,
    ) -> Self {
        let width_blocks = frame_width.div_ceil(block_size);
        let height_blocks = frame_height.div_ceil(block_size);
        let total_blocks = (width_blocks * height_blocks) as usize;

        Self {
            display_idx,
            block_size,
            width_blocks,
            height_blocks,
            frame_width,
            frame_height,
            metric_type,
            values: vec![0.0; total_blocks],
        }
    }

    /// Get value at block position
    pub fn get(&self, block_x: u32, block_y: u32) -> Option<f32> {
        if block_x < self.width_blocks && block_y < self.height_blocks {
            let idx = (block_y * self.width_blocks + block_x) as usize;
            self.values.get(idx).copied()
        } else {
            None
        }
    }

    /// Set value at block position
    pub fn set(&mut self, block_x: u32, block_y: u32, value: f32) {
        if block_x < self.width_blocks && block_y < self.height_blocks {
            let idx = (block_y * self.width_blocks + block_x) as usize;
            if idx < self.values.len() {
                self.values[idx] = value;
            }
        }
    }

    /// Get normalized value (0.0-1.0) at block position
    pub fn get_normalized(&self, block_x: u32, block_y: u32) -> Option<f32> {
        self.get(block_x, block_y).map(|v| {
            let (min, max) = self.metric_type.typical_range();
            if max > min {
                ((v - min) / (max - min)).clamp(0.0, 1.0)
            } else {
                0.5
            }
        })
    }

    /// Total number of blocks
    pub fn total_blocks(&self) -> usize {
        (self.width_blocks * self.height_blocks) as usize
    }

    /// Get block position from pixel coordinates
    pub fn pixel_to_block(&self, x: u32, y: u32) -> (u32, u32) {
        (x / self.block_size, y / self.block_size)
    }

    /// Get pixel range for a block
    pub fn block_to_pixel_range(&self, block_x: u32, block_y: u32) -> (u32, u32, u32, u32) {
        let x0 = block_x * self.block_size;
        let y0 = block_y * self.block_size;
        let x1 = ((block_x + 1) * self.block_size).min(self.frame_width);
        let y1 = ((block_y + 1) * self.block_size).min(self.frame_height);
        (x0, y0, x1, y1)
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Block Metrics Statistics
// ═══════════════════════════════════════════════════════════════════════════

/// Statistics for block metrics grid
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BlockMetricsStatistics {
    /// Metric type
    pub metric_type: BlockMetricType,
    /// Minimum value
    pub min: f32,
    /// Maximum value
    pub max: f32,
    /// Average value
    pub avg: f32,
    /// Standard deviation
    pub std_dev: f32,
    /// Total blocks
    pub total_blocks: usize,
    /// Blocks below threshold (quality issues)
    pub blocks_below_threshold: usize,
    /// Threshold used
    pub threshold: f32,
}

impl BlockMetricsStatistics {
    pub fn from_grid(grid: &BlockMetricsGrid, threshold: f32) -> Self {
        let values = &grid.values;
        if values.is_empty() {
            return Self {
                metric_type: grid.metric_type,
                min: 0.0,
                max: 0.0,
                avg: 0.0,
                std_dev: 0.0,
                total_blocks: 0,
                blocks_below_threshold: 0,
                threshold,
            };
        }

        let min = values.iter().copied().fold(f32::INFINITY, f32::min);
        let max = values.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        let sum: f32 = values.iter().sum();
        let avg = sum / values.len() as f32;

        let variance: f32 =
            values.iter().map(|v| (v - avg).powi(2)).sum::<f32>() / values.len() as f32;
        let std_dev = variance.sqrt();

        let blocks_below = if grid.metric_type.higher_is_better() {
            values.iter().filter(|&&v| v < threshold).count()
        } else {
            values.iter().filter(|&&v| v > threshold).count()
        };

        Self {
            metric_type: grid.metric_type,
            min,
            max,
            avg,
            std_dev,
            total_blocks: values.len(),
            blocks_below_threshold: blocks_below,
            threshold,
        }
    }

    /// Quality percentage (blocks above threshold)
    pub fn quality_percent(&self) -> f32 {
        if self.total_blocks > 0 {
            ((self.total_blocks - self.blocks_below_threshold) as f32 / self.total_blocks as f32)
                * 100.0
        } else {
            0.0
        }
    }

    /// Format as display string
    pub fn format_display(&self) -> String {
        format!(
            "{}: min={:.2}{} max={:.2}{} avg={:.2}{} σ={:.2}",
            self.metric_type.name(),
            self.min,
            self.metric_type.unit(),
            self.max,
            self.metric_type.unit(),
            self.avg,
            self.metric_type.unit(),
            self.std_dev
        )
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Block Metrics Calculator
// ═══════════════════════════════════════════════════════════════════════════

/// Calculator for block-level metrics
#[derive(Debug, Clone)]
pub struct BlockMetricsCalculator {
    /// Block size for calculation
    pub block_size: u32,
    /// Bit depth of video
    pub bit_depth: u8,
    /// Maximum pixel value
    max_value: f32,
}

impl Default for BlockMetricsCalculator {
    fn default() -> Self {
        Self::new(16, 8)
    }
}

impl BlockMetricsCalculator {
    pub fn new(block_size: u32, bit_depth: u8) -> Self {
        let max_value = ((1u32 << bit_depth) - 1) as f32;
        Self {
            block_size,
            bit_depth,
            max_value,
        }
    }

    /// Calculate PSNR for a block (given pixel slices)
    pub fn calculate_psnr_block(&self, src: &[u8], ref_block: &[u8]) -> f32 {
        if src.len() != ref_block.len() || src.is_empty() {
            return 0.0;
        }

        let mse = self.calculate_mse_block(src, ref_block);
        if mse <= 0.0 {
            return 100.0; // Perfect match
        }

        10.0 * (self.max_value * self.max_value / mse).log10()
    }

    /// Calculate MSE for a block
    pub fn calculate_mse_block(&self, src: &[u8], ref_block: &[u8]) -> f32 {
        if src.len() != ref_block.len() || src.is_empty() {
            return 0.0;
        }

        let sum_sq_diff: f64 = src
            .iter()
            .zip(ref_block.iter())
            .map(|(&s, &r)| {
                let diff = s as f64 - r as f64;
                diff * diff
            })
            .sum();

        (sum_sq_diff / src.len() as f64) as f32
    }

    /// Calculate MAD for a block
    pub fn calculate_mad_block(&self, src: &[u8], ref_block: &[u8]) -> f32 {
        if src.len() != ref_block.len() || src.is_empty() {
            return 0.0;
        }

        let sum_abs_diff: f64 = src
            .iter()
            .zip(ref_block.iter())
            .map(|(&s, &r)| (s as f64 - r as f64).abs())
            .sum();

        (sum_abs_diff / src.len() as f64) as f32
    }

    /// Calculate simplified SSIM for a block
    /// Note: This is a simplified implementation; full SSIM requires more complex windowing
    pub fn calculate_ssim_block(&self, src: &[u8], ref_block: &[u8]) -> f32 {
        if src.len() != ref_block.len() || src.is_empty() {
            return 0.0;
        }

        let n = src.len() as f64;

        // Calculate means
        let mean_s: f64 = src.iter().map(|&x| x as f64).sum::<f64>() / n;
        let mean_r: f64 = ref_block.iter().map(|&x| x as f64).sum::<f64>() / n;

        // Calculate variances and covariance
        let mut var_s = 0.0;
        let mut var_r = 0.0;
        let mut covar = 0.0;

        for (&s, &r) in src.iter().zip(ref_block.iter()) {
            let s_diff = s as f64 - mean_s;
            let r_diff = r as f64 - mean_r;
            var_s += s_diff * s_diff;
            var_r += r_diff * r_diff;
            covar += s_diff * r_diff;
        }

        var_s /= n - 1.0;
        var_r /= n - 1.0;
        covar /= n - 1.0;

        // SSIM constants
        let c1 = (0.01 * self.max_value as f64).powi(2);
        let c2 = (0.03 * self.max_value as f64).powi(2);

        let ssim = ((2.0 * mean_s * mean_r + c1) * (2.0 * covar + c2))
            / ((mean_s.powi(2) + mean_r.powi(2) + c1) * (var_s + var_r + c2));

        ssim as f32
    }

    /// Calculate metric for a block
    pub fn calculate_block(
        &self,
        src: &[u8],
        ref_block: &[u8],
        metric_type: BlockMetricType,
    ) -> f32 {
        match metric_type {
            BlockMetricType::Psnr => self.calculate_psnr_block(src, ref_block),
            BlockMetricType::Ssim => self.calculate_ssim_block(src, ref_block),
            BlockMetricType::Mse => self.calculate_mse_block(src, ref_block),
            BlockMetricType::Mad => self.calculate_mad_block(src, ref_block),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Block Metrics Color Mapper
// ═══════════════════════════════════════════════════════════════════════════

/// Color mapper for block metrics heatmap
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockMetricsColorMapper {
    /// Metric type
    pub metric_type: BlockMetricType,
    /// Low value color (RGBA)
    pub low_color: [u8; 4],
    /// High value color (RGBA)
    pub high_color: [u8; 4],
    /// Threshold color (RGBA) for values below threshold
    pub threshold_color: [u8; 4],
    /// Threshold value
    pub threshold: f32,
    /// User opacity (0.0-1.0)
    pub opacity: f32,
}

impl Default for BlockMetricsColorMapper {
    fn default() -> Self {
        Self::for_psnr()
    }
}

impl BlockMetricsColorMapper {
    /// Create mapper for PSNR (red=bad, green=good)
    pub fn for_psnr() -> Self {
        Self {
            metric_type: BlockMetricType::Psnr,
            low_color: [255, 0, 0, 200],         // Red for low PSNR
            high_color: [0, 255, 0, 200],        // Green for high PSNR
            threshold_color: [255, 255, 0, 255], // Yellow for below threshold
            threshold: 30.0,                     // 30 dB threshold
            opacity: 0.5,
        }
    }

    /// Create mapper for SSIM
    pub fn for_ssim() -> Self {
        Self {
            metric_type: BlockMetricType::Ssim,
            low_color: [255, 0, 0, 200],
            high_color: [0, 255, 0, 200],
            threshold_color: [255, 255, 0, 255],
            threshold: 0.9,
            opacity: 0.5,
        }
    }

    /// Create mapper for MSE (reverse - lower is better)
    pub fn for_mse() -> Self {
        Self {
            metric_type: BlockMetricType::Mse,
            low_color: [0, 255, 0, 200],  // Green for low MSE
            high_color: [255, 0, 0, 200], // Red for high MSE
            threshold_color: [255, 255, 0, 255],
            threshold: 100.0,
            opacity: 0.5,
        }
    }

    /// Map value to color
    pub fn map_color(&self, value: f32) -> [u8; 4] {
        let (min, max) = self.metric_type.typical_range();
        let t = if max > min {
            ((value - min) / (max - min)).clamp(0.0, 1.0)
        } else {
            0.5
        };

        // Interpolate between low and high colors
        let mut color = [0u8; 4];
        for (c, (&low, &high)) in color
            .iter_mut()
            .zip(self.low_color.iter().zip(&self.high_color))
        {
            *c = (low as f32 * (1.0 - t) + high as f32 * t) as u8;
        }

        // Apply user opacity
        color[3] = (color[3] as f32 * self.opacity) as u8;

        color
    }

    /// Check if value is below quality threshold
    pub fn is_below_threshold(&self, value: f32) -> bool {
        if self.metric_type.higher_is_better() {
            value < self.threshold
        } else {
            value > self.threshold
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Block Metrics Overlay Data
// ═══════════════════════════════════════════════════════════════════════════

/// Data for rendering block metrics overlay
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockMetricsOverlay {
    /// Metrics grid
    pub grid: BlockMetricsGrid,
    /// Color mapper
    pub color_mapper: BlockMetricsColorMapper,
    /// Statistics
    pub statistics: BlockMetricsStatistics,
    /// Show block boundaries
    pub show_boundaries: bool,
    /// Show value labels
    pub show_labels: bool,
}

impl BlockMetricsOverlay {
    pub fn new(grid: BlockMetricsGrid, color_mapper: BlockMetricsColorMapper) -> Self {
        let statistics = BlockMetricsStatistics::from_grid(&grid, color_mapper.threshold);
        Self {
            grid,
            color_mapper,
            statistics,
            show_boundaries: true,
            show_labels: false,
        }
    }

    /// Get color for block at position
    pub fn get_block_color(&self, block_x: u32, block_y: u32) -> Option<[u8; 4]> {
        self.grid
            .get(block_x, block_y)
            .map(|v| self.color_mapper.map_color(v))
    }

    /// Get RGBA image data for overlay
    pub fn to_rgba(&self) -> Vec<u8> {
        let width = self.grid.frame_width as usize;
        let height = self.grid.frame_height as usize;
        let mut rgba = vec![0u8; width * height * 4];

        for block_y in 0..self.grid.height_blocks {
            for block_x in 0..self.grid.width_blocks {
                if let Some(color) = self.get_block_color(block_x, block_y) {
                    let (x0, y0, x1, y1) = self.grid.block_to_pixel_range(block_x, block_y);

                    for y in y0..y1 {
                        for x in x0..x1 {
                            let idx = ((y as usize) * width + (x as usize)) * 4;
                            if idx + 3 < rgba.len() {
                                rgba[idx] = color[0];
                                rgba[idx + 1] = color[1];
                                rgba[idx + 2] = color[2];
                                rgba[idx + 3] = color[3];
                            }
                        }
                    }
                }
            }
        }

        rgba
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Multi-Frame Block Metrics
// ═══════════════════════════════════════════════════════════════════════════

/// Collection of block metrics across multiple frames
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MultiFrameBlockMetrics {
    /// Metrics by frame index
    pub frames: HashMap<usize, BlockMetricsGrid>,
    /// Metric type
    pub metric_type: BlockMetricType,
    /// Block size
    pub block_size: u32,
}

impl MultiFrameBlockMetrics {
    pub fn new(metric_type: BlockMetricType, block_size: u32) -> Self {
        Self {
            frames: HashMap::new(),
            metric_type,
            block_size,
        }
    }

    /// Add metrics for a frame
    pub fn add_frame(&mut self, grid: BlockMetricsGrid) {
        self.frames.insert(grid.display_idx, grid);
    }

    /// Get metrics for a frame
    pub fn get_frame(&self, display_idx: usize) -> Option<&BlockMetricsGrid> {
        self.frames.get(&display_idx)
    }

    /// Get frame count
    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }

    /// Get average statistics across all frames
    pub fn aggregate_statistics(&self, threshold: f32) -> Option<BlockMetricsStatistics> {
        if self.frames.is_empty() {
            return None;
        }

        let all_values: Vec<f32> = self
            .frames
            .values()
            .flat_map(|g| g.values.iter().copied())
            .collect();

        if all_values.is_empty() {
            return None;
        }

        let min = all_values.iter().copied().fold(f32::INFINITY, f32::min);
        let max = all_values.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        let sum: f32 = all_values.iter().sum();
        let avg = sum / all_values.len() as f32;

        let variance: f32 =
            all_values.iter().map(|v| (v - avg).powi(2)).sum::<f32>() / all_values.len() as f32;
        let std_dev = variance.sqrt();

        let blocks_below = if self.metric_type.higher_is_better() {
            all_values.iter().filter(|&&v| v < threshold).count()
        } else {
            all_values.iter().filter(|&&v| v > threshold).count()
        };

        Some(BlockMetricsStatistics {
            metric_type: self.metric_type,
            min,
            max,
            avg,
            std_dev,
            total_blocks: all_values.len(),
            blocks_below_threshold: blocks_below,
            threshold,
        })
    }
}

/// Comprehensive test suite with Arrange-Act-Assert pattern
#[cfg(test)]
mod tests {
    include!("block_metrics_test.rs");
}
