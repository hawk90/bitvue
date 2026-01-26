//! Timeline Lane Population
//!
//! Helper utilities for populating timeline lanes from frame data

use crate::DiagnosticsManager;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::timeline::TimelineFrame;
use super::timeline_lane_types::{calculate_bpp, FrameQpStats, FrameSliceStats};

/// Lane type for timeline overlay
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LaneType {
    /// QP average per frame
    QpAvg,
    /// Bits per pixel
    BitsPerPixel,
    /// Slice/tile count
    SliceCount,
    /// Diagnostics density (error/warning count)
    DiagnosticsDensity,
    /// Reorder mismatch band (PTS ≠ DTS)
    ReorderMismatch,
}

impl LaneType {
    /// Get lane display name
    pub fn name(&self) -> &'static str {
        match self {
            LaneType::QpAvg => "QP Average",
            LaneType::BitsPerPixel => "Bits per Pixel",
            LaneType::SliceCount => "Slice Count",
            LaneType::DiagnosticsDensity => "Diagnostics",
            LaneType::ReorderMismatch => "Reorder Mismatch",
        }
    }

    /// Get lane color hint
    pub fn color_hint(&self) -> &'static str {
        match self {
            LaneType::QpAvg => "cyan",
            LaneType::BitsPerPixel => "magenta",
            LaneType::SliceCount => "yellow",
            LaneType::DiagnosticsDensity => "orange",
            LaneType::ReorderMismatch => "red",
        }
    }

    /// Check if this lane uses secondary axis (vs primary)
    pub fn uses_secondary_axis(&self) -> bool {
        matches!(self, LaneType::QpAvg | LaneType::BitsPerPixel)
    }
}

/// Lane data point
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LaneDataPoint {
    /// Frame display_idx
    pub display_idx: usize,
    /// Value (lane-specific units)
    pub value: f32,
}

impl LaneDataPoint {
    pub fn new(display_idx: usize, value: f32) -> Self {
        Self { display_idx, value }
    }
}

/// Lane configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lane {
    /// Lane type
    pub lane_type: LaneType,
    /// Data points (indexed by display_idx)
    pub data: Vec<LaneDataPoint>,
    /// Enabled state
    pub enabled: bool,
    /// Opacity (0.0 to 1.0)
    pub opacity: f32,
}

impl Lane {
    /// Create a new lane
    pub fn new(lane_type: LaneType) -> Self {
        Self {
            lane_type,
            data: Vec::new(),
            enabled: true,
            opacity: 1.0,
        }
    }

    /// Add a data point
    pub fn add_point(&mut self, display_idx: usize, value: f32) {
        self.data.push(LaneDataPoint::new(display_idx, value));
    }

    /// Get value at specific display_idx
    pub fn get_value(&self, display_idx: usize) -> Option<f32> {
        self.data
            .iter()
            .find(|p| p.display_idx == display_idx)
            .map(|p| p.value)
    }

    /// Get value range (min, max)
    pub fn value_range(&self) -> (f32, f32) {
        if self.data.is_empty() {
            return (0.0, 0.0);
        }

        let mut min = f32::MAX;
        let mut max = f32::MIN;

        for point in &self.data {
            if point.value < min {
                min = point.value;
            }
            if point.value > max {
                max = point.value;
            }
        }

        (min, max)
    }
}

/// Statistics types for different lanes
#[derive(Debug, Clone)]
pub enum LaneStatistics {
    Qp(super::timeline_lane_types::QpLaneStatistics),
    Slice(super::timeline_lane_types::SliceLaneStatistics),
}

// ═══════════════════════════════════════════════════════════════════════════
// Lane Population Helpers
// ═══════════════════════════════════════════════════════════════════════════

/// Helper to replace a lane in the lanes vector
pub fn replace_lane(lanes: &mut Vec<Lane>, new_lane: Lane) {
    lanes.retain(|l| l.lane_type != new_lane.lane_type);
    lanes.push(new_lane);
}

/// Populate BPP lane from timeline frames
///
/// BPP = (frame_size_bytes * 8) / (width * height)
/// Per COMPETITOR_PARITY_STATUS: Feature Parity Phase C
pub fn populate_bpp_lane(frames: &[TimelineFrame], frame_width: u32, frame_height: u32) -> Lane {
    if frame_width == 0 || frame_height == 0 {
        return Lane::new(LaneType::BitsPerPixel);
    }

    let mut lane = Lane::new(LaneType::BitsPerPixel);

    for (idx, frame) in frames.iter().enumerate() {
        let bpp = calculate_bpp(frame.size_bytes, frame_width, frame_height);
        lane.add_point(idx, bpp);
    }

    lane
}

/// Populate diagnostics density lane from diagnostics manager
///
/// Per COMPETITOR_PARITY_STATUS: Shows error/warning density per frame
pub fn populate_diagnostics_lane(diagnostics: &DiagnosticsManager) -> Lane {
    let mut lane = Lane::new(LaneType::DiagnosticsDensity);

    // Count diagnostics per frame
    let mut frame_counts: HashMap<usize, usize> = HashMap::new();

    for diag in &diagnostics.diagnostics {
        if diag.severity.is_actionable() {
            if let Some(frame_key) = &diag.frame_key {
                *frame_counts.entry(frame_key.frame_index).or_insert(0) += 1;
            }
        }
    }

    // Add data points for frames with diagnostics
    for (frame_idx, count) in frame_counts {
        lane.add_point(frame_idx, count as f32);
    }

    lane
}

/// Populate reorder mismatch lane from timeline frames
///
/// Per COMPETITOR_PARITY_STATUS: Shows PTS ≠ DTS patterns
pub fn populate_reorder_lane(frames: &[TimelineFrame]) -> Lane {
    let mut lane = Lane::new(LaneType::ReorderMismatch);

    for (idx, frame) in frames.iter().enumerate() {
        // Check if PTS and DTS differ (indicates B-frame reordering)
        let has_mismatch = match (frame.pts, frame.dts) {
            (Some(pts), Some(dts)) => pts != dts,
            _ => false,
        };
        lane.add_point(idx, if has_mismatch { 1.0 } else { 0.0 });
    }

    lane
}

/// Populate QP average lane from frame QP metadata
///
/// Per COMPETITOR_PARITY_STATUS §4.1: QP series plot (avg/min/max)
/// Takes frame QP stats as input (from codec-specific extraction)
pub fn populate_qp_lane(qp_data: &[FrameQpStats]) -> Lane {
    let mut lane = Lane::new(LaneType::QpAvg);

    for stats in qp_data {
        lane.add_point(stats.display_idx, stats.qp_avg);
    }

    lane
}

/// Populate slice/tile count lane from frame metadata
///
/// Per COMPETITOR_PARITY_STATUS §4.1: Slice/Tile count series plot
/// Takes slice counts as input (from codec-specific extraction)
pub fn populate_slice_lane(slice_data: &[FrameSliceStats]) -> Lane {
    let mut lane = Lane::new(LaneType::SliceCount);

    for stats in slice_data {
        lane.add_point(stats.display_idx, stats.slice_count as f32);
    }

    lane
}

/// Calculate QP statistics from a lane
pub fn calculate_qp_statistics(lane: &Lane) -> Option<super::timeline_lane_types::QpLaneStatistics> {
    if lane.data.is_empty() {
        return None;
    }

    let (min, max) = lane.value_range();
    let sum: f32 = lane.data.iter().map(|p| p.value).sum();
    let avg = sum / lane.data.len() as f32;

    // Calculate std dev
    let variance: f32 = lane
        .data
        .iter()
        .map(|p| {
            let d = p.value - avg;
            d * d
        })
        .sum::<f32>()
        / lane.data.len() as f32;

    Some(super::timeline_lane_types::QpLaneStatistics {
        min_qp: min,
        max_qp: max,
        avg_qp: avg,
        std_dev: variance.sqrt(),
        frame_count: lane.data.len(),
    })
}

/// Calculate slice statistics from a lane
pub fn calculate_slice_statistics(
    lane: &Lane,
) -> Option<super::timeline_lane_types::SliceLaneStatistics> {
    if lane.data.is_empty() {
        return None;
    }

    let (min, max) = lane.value_range();
    let sum: f32 = lane.data.iter().map(|p| p.value).sum();
    let avg = sum / lane.data.len() as f32;

    // Count frames with multiple slices/tiles
    let multi_slice_count = lane.data.iter().filter(|p| p.value > 1.0).count();

    Some(super::timeline_lane_types::SliceLaneStatistics {
        min_slices: min as u32,
        max_slices: max as u32,
        avg_slices: avg,
        multi_slice_frame_count: multi_slice_count,
        frame_count: lane.data.len(),
    })
}

#[cfg(test)]
mod tests {
    include!("timeline_lane_population_test.rs");
}
