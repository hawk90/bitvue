//! Multi-Lane Timeline Overlays - T4-2
//!
//! Per WS_TIMELINE_TEMPORAL.md:
//! - Multiple overlaid lanes (QP avg, bpp, slice/tile count, diagnostics density, reorder mismatch)
//! - LOD and marker clustering for performance
//! - Lane overlay toggles

use serde::{Deserialize, Serialize};

// Re-export types from submodules
pub use crate::timeline_lane_clustering::{MarkerCluster, MarkerClustering};
pub use crate::timeline_lane_population::{Lane, LaneDataPoint, LaneType};
pub use crate::timeline_lane_types::{
    calculate_bpp, estimate_qp_avg, FrameQpStats, FrameSliceStats, QpLaneStatistics,
    SliceLaneStatistics,
};

use super::timeline::TimelineFrame;
use crate::DiagnosticsManager;

/// Timeline Lane System
///
/// Per T4-2 deliverable: TimelineLaneSystem with marker clustering and lane toggles
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineLaneSystem {
    /// All available lanes
    pub lanes: Vec<Lane>,

    /// Marker clusters (computed from markers + zoom level)
    pub marker_clusters: Vec<MarkerCluster>,

    /// Current zoom level (1.0 = fit all, higher = zoomed in)
    pub zoom_level: f32,

    /// Total frame count (for clustering threshold calculation)
    pub total_frames: usize,
}

impl TimelineLaneSystem {
    /// Create a new lane system
    pub fn new(total_frames: usize) -> Self {
        Self {
            lanes: Vec::new(),
            marker_clusters: Vec::new(),
            zoom_level: 1.0,
            total_frames,
        }
    }

    /// Add a lane
    pub fn add_lane(&mut self, lane: Lane) {
        self.lanes.push(lane);
    }

    /// Get lane by type
    pub fn get_lane(&self, lane_type: LaneType) -> Option<&Lane> {
        self.lanes.iter().find(|l| l.lane_type == lane_type)
    }

    /// Get mutable lane by type
    pub fn get_lane_mut(&mut self, lane_type: LaneType) -> Option<&mut Lane> {
        self.lanes.iter_mut().find(|l| l.lane_type == lane_type)
    }

    /// Toggle lane enabled state
    ///
    /// Per T4-2 deliverable: Lane overlay toggles
    pub fn toggle_lane(&mut self, lane_type: LaneType) {
        if let Some(lane) = self.get_lane_mut(lane_type) {
            lane.enabled = !lane.enabled;
        }
    }

    /// Set lane enabled state
    pub fn set_lane_enabled(&mut self, lane_type: LaneType, enabled: bool) {
        if let Some(lane) = self.get_lane_mut(lane_type) {
            lane.enabled = enabled;
        }
    }

    /// Get enabled lanes
    pub fn enabled_lanes(&self) -> Vec<&Lane> {
        self.lanes.iter().filter(|l| l.enabled).collect()
    }

    /// Set zoom level and recompute marker clusters
    pub fn set_zoom_level(
        &mut self,
        zoom_level: f32,
        markers: &[(usize, crate::timeline::FrameMarker)],
    ) {
        self.zoom_level = zoom_level;
        self.update_marker_clusters(markers);
    }

    /// Update marker clusters based on current zoom level
    ///
    /// Per T4-2 deliverable: Marker clustering
    pub fn update_marker_clusters(&mut self, markers: &[(usize, crate::timeline::FrameMarker)]) {
        let threshold = MarkerClustering::calculate_threshold(self.zoom_level, self.total_frames);
        self.marker_clusters = MarkerClustering::cluster(markers, threshold);
    }

    /// Get cluster at display_idx
    pub fn get_cluster_at(&self, display_idx: usize) -> Option<&MarkerCluster> {
        self.marker_clusters
            .iter()
            .find(|c| display_idx >= c.start_idx && display_idx <= c.end_idx)
    }

    /// Count enabled lanes
    pub fn enabled_count(&self) -> usize {
        self.lanes.iter().filter(|l| l.enabled).count()
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Lane Population Methods
    // ═══════════════════════════════════════════════════════════════════════════

    /// Populate BPP lane from timeline frames
    pub fn populate_bpp_lane(
        &mut self,
        frames: &[TimelineFrame],
        frame_width: u32,
        frame_height: u32,
    ) {
        let lane =
            crate::timeline_lane_population::populate_bpp_lane(frames, frame_width, frame_height);
        crate::timeline_lane_population::replace_lane(&mut self.lanes, lane);
    }

    /// Populate diagnostics density lane from diagnostics manager
    pub fn populate_diagnostics_lane(&mut self, diagnostics: &DiagnosticsManager) {
        let lane = crate::timeline_lane_population::populate_diagnostics_lane(diagnostics);
        crate::timeline_lane_population::replace_lane(&mut self.lanes, lane);
    }

    /// Populate reorder mismatch lane from timeline frames
    pub fn populate_reorder_lane(&mut self, frames: &[TimelineFrame]) {
        let lane = crate::timeline_lane_population::populate_reorder_lane(frames);
        crate::timeline_lane_population::replace_lane(&mut self.lanes, lane);
    }

    /// Populate QP average lane from frame QP metadata
    pub fn populate_qp_lane(&mut self, qp_data: &[FrameQpStats]) {
        let lane = crate::timeline_lane_population::populate_qp_lane(qp_data);
        crate::timeline_lane_population::replace_lane(&mut self.lanes, lane);
    }

    /// Populate slice/tile count lane from frame metadata
    pub fn populate_slice_lane(&mut self, slice_data: &[FrameSliceStats]) {
        let lane = crate::timeline_lane_population::populate_slice_lane(slice_data);
        crate::timeline_lane_population::replace_lane(&mut self.lanes, lane);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Statistics Methods
    // ═══════════════════════════════════════════════════════════════════════════

    /// Get QP statistics summary across all frames
    pub fn qp_statistics(&self) -> Option<QpLaneStatistics> {
        let lane = self.get_lane(LaneType::QpAvg)?;
        crate::timeline_lane_population::calculate_qp_statistics(lane)
    }

    /// Get slice count statistics summary across all frames
    pub fn slice_statistics(&self) -> Option<SliceLaneStatistics> {
        let lane = self.get_lane(LaneType::SliceCount)?;
        crate::timeline_lane_population::calculate_slice_statistics(lane)
    }
}

#[cfg(test)]
mod tests {
    include!("timeline_lanes_test.rs");
}
