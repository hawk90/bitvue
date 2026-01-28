//! Timeline Lane Clustering
//!
//! Marker clustering for LOD (Level of Detail) when zoomed out

use serde::{Deserialize, Serialize};

use super::timeline::FrameMarker;

/// Marker cluster (for LOD)
///
/// Per PERFORMANCE_DEGRADATION_RULES: When zoomed out, cluster nearby markers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarkerCluster {
    /// Cluster center (display_idx)
    pub center_idx: usize,
    /// Start index (inclusive)
    pub start_idx: usize,
    /// End index (inclusive)
    pub end_idx: usize,
    /// Number of markers in cluster
    pub count: usize,
    /// Representative marker type (most critical)
    pub primary_type: FrameMarker,
}

impl MarkerCluster {
    /// Create a single-marker cluster
    pub fn single(display_idx: usize, marker_type: FrameMarker) -> Self {
        Self {
            center_idx: display_idx,
            start_idx: display_idx,
            end_idx: display_idx,
            count: 1,
            primary_type: marker_type,
        }
    }

    /// Check if a marker should be merged into this cluster
    pub fn can_merge(&self, display_idx: usize, cluster_threshold: usize) -> bool {
        let dist_start = display_idx.abs_diff(self.start_idx);
        let dist_end = display_idx.abs_diff(self.end_idx);

        dist_start <= cluster_threshold || dist_end <= cluster_threshold
    }

    /// Merge another marker into this cluster
    pub fn merge(&mut self, display_idx: usize, marker_type: FrameMarker) {
        self.start_idx = self.start_idx.min(display_idx);
        self.end_idx = self.end_idx.max(display_idx);
        self.count += 1;
        self.center_idx = (self.start_idx + self.end_idx) / 2;

        // Update primary type if new marker is more critical
        if marker_type.is_critical() && !self.primary_type.is_critical() {
            self.primary_type = marker_type;
        }
    }
}

/// Marker clustering algorithm
pub struct MarkerClustering;

impl MarkerClustering {
    /// Cluster markers based on zoom level
    ///
    /// Per PERFORMANCE_DEGRADATION_RULES: Cluster threshold grows as zoom-out
    pub fn cluster(
        markers: &[(usize, FrameMarker)],
        cluster_threshold: usize,
    ) -> Vec<MarkerCluster> {
        if markers.is_empty() {
            return Vec::new();
        }

        let mut clusters: Vec<MarkerCluster> = Vec::new();

        for &(display_idx, marker_type) in markers {
            if marker_type == FrameMarker::None {
                continue;
            }

            // Try to merge with existing cluster
            let mut merged = false;
            for cluster in &mut clusters {
                if cluster.can_merge(display_idx, cluster_threshold) {
                    cluster.merge(display_idx, marker_type);
                    merged = true;
                    break;
                }
            }

            // Create new cluster if not merged
            if !merged {
                clusters.push(MarkerCluster::single(display_idx, marker_type));
            }
        }

        clusters
    }

    /// Calculate cluster threshold based on zoom level
    ///
    /// Per EDGE_CASES_AND_DEGRADE_BEHAVIOR: Adaptive clustering
    pub fn calculate_threshold(zoom_level: f32, total_frames: usize) -> usize {
        // At zoom 1.0 (fit all), threshold is ~1% of frames
        // At zoom 10.0 (zoomed in), threshold is smaller
        let base_threshold = (total_frames as f32 * 0.01) / zoom_level;
        base_threshold.clamp(1.0, 100.0) as usize
    }
}

#[cfg(test)]
mod tests {
    include!("timeline_lane_clustering_test.rs");
}
