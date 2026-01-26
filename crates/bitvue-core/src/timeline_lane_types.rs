//! Timeline Lane Types
//!
//! Data types for timeline lane statistics and frame metadata

use serde::{Deserialize, Serialize};

/// QP statistics for a single frame
///
/// Per COMPETITOR_PARITY_STATUS §4.1: QP series plot needs codec integration
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct FrameQpStats {
    /// Frame display_idx
    pub display_idx: usize,
    /// Average QP for this frame
    pub qp_avg: f32,
    /// Minimum QP for this frame (optional)
    pub qp_min: Option<i8>,
    /// Maximum QP for this frame (optional)
    pub qp_max: Option<i8>,
}

impl FrameQpStats {
    /// Create new frame QP stats
    pub fn new(display_idx: usize, qp_avg: f32) -> Self {
        Self {
            display_idx,
            qp_avg,
            qp_min: None,
            qp_max: None,
        }
    }

    /// Create with min/max values
    pub fn with_range(display_idx: usize, qp_avg: f32, qp_min: i8, qp_max: i8) -> Self {
        Self {
            display_idx,
            qp_avg,
            qp_min: Some(qp_min),
            qp_max: Some(qp_max),
        }
    }

    /// Create from min/max only (estimate avg)
    pub fn from_range(display_idx: usize, qp_min: i8, qp_max: i8) -> Self {
        Self {
            display_idx,
            qp_avg: estimate_qp_avg(qp_min, qp_max),
            qp_min: Some(qp_min),
            qp_max: Some(qp_max),
        }
    }
}

/// Slice/tile count for a single frame
///
/// Per COMPETITOR_PARITY_STATUS §4.1: Slice/Tile count series plot
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct FrameSliceStats {
    /// Frame display_idx
    pub display_idx: usize,
    /// Number of slices (H.264/HEVC) or tiles (AV1/VP9/VVC)
    pub slice_count: u32,
    /// Tile columns (for grid configurations)
    pub tile_cols: Option<u32>,
    /// Tile rows (for grid configurations)
    pub tile_rows: Option<u32>,
}

impl FrameSliceStats {
    /// Create new frame slice stats
    pub fn new(display_idx: usize, slice_count: u32) -> Self {
        Self {
            display_idx,
            slice_count,
            tile_cols: None,
            tile_rows: None,
        }
    }

    /// Create with tile grid info
    pub fn with_tile_grid(
        display_idx: usize,
        slice_count: u32,
        tile_cols: u32,
        tile_rows: u32,
    ) -> Self {
        Self {
            display_idx,
            slice_count,
            tile_cols: Some(tile_cols),
            tile_rows: Some(tile_rows),
        }
    }
}

/// QP lane statistics summary
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct QpLaneStatistics {
    /// Minimum QP across all frames
    pub min_qp: f32,
    /// Maximum QP across all frames
    pub max_qp: f32,
    /// Average QP across all frames
    pub avg_qp: f32,
    /// Standard deviation
    pub std_dev: f32,
    /// Number of frames with QP data
    pub frame_count: usize,
}

impl QpLaneStatistics {
    /// Format as summary text
    pub fn summary_text(&self) -> String {
        format!(
            "QP: min {:.1}, max {:.1}, avg {:.1} (σ {:.2}) [{} frames]",
            self.min_qp, self.max_qp, self.avg_qp, self.std_dev, self.frame_count
        )
    }
}

/// Slice lane statistics summary
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SliceLaneStatistics {
    /// Minimum slice count
    pub min_slices: u32,
    /// Maximum slice count
    pub max_slices: u32,
    /// Average slice count
    pub avg_slices: f32,
    /// Number of frames with multiple slices
    pub multi_slice_frame_count: usize,
    /// Total number of frames
    pub frame_count: usize,
}

impl SliceLaneStatistics {
    /// Format as summary text
    pub fn summary_text(&self) -> String {
        format!(
            "Slices: min {}, max {}, avg {:.1} [{}/{} multi-slice]",
            self.min_slices,
            self.max_slices,
            self.avg_slices,
            self.multi_slice_frame_count,
            self.frame_count
        )
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Helper Functions
// ═══════════════════════════════════════════════════════════════════════════

/// Calculate bits per pixel for a single frame
///
/// BPP = (frame_size_bytes * 8) / (width * height)
pub fn calculate_bpp(frame_size_bytes: u64, width: u32, height: u32) -> f32 {
    let pixel_count = (width as u64) * (height as u64);
    if pixel_count == 0 {
        return 0.0;
    }
    (frame_size_bytes * 8) as f32 / pixel_count as f32
}

/// Calculate average QP from min and max QP values
///
/// Fallback when only min/max are available
pub fn estimate_qp_avg(qp_min: i8, qp_max: i8) -> f32 {
    (qp_min as f32 + qp_max as f32) / 2.0
}

#[cfg(test)]
mod tests {
    include!("timeline_lane_types_test.rs");
}
