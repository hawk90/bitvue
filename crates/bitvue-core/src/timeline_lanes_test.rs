// Timeline lanes module tests
//
// Per generate-tests skill: Arrange-Act-Assert pattern with fixtures
// and edge case coverage.

use super::*;
use crate::DiagnosticsManager;
use crate::timeline::TimelineFrame;
use crate::FrameMarker;

// ============================================================================
// Fixtures
// ============================================================================

/// Create test timeline frames
fn create_test_frames(count: usize) -> Vec<TimelineFrame> {
    (0..count)
        .map(|i| TimelineFrame {
            display_idx: i,
            size_bytes: 1000 + i as u64 * 100,
            frame_type: if i % 3 == 0 { "I".to_string() } else { "P".to_string() },
            marker: if i % 3 == 0 {
                FrameMarker::Key
            } else if i % 7 == 0 {
                FrameMarker::Bookmark
            } else {
                FrameMarker::None
            },
            pts: Some(i as u64 * 100),
            dts: Some(i as u64 * 100),
            is_selected: false,
        })
        .collect()
}

/// Create test QP stats
fn create_test_qp_stats() -> Vec<FrameQpStats> {
    vec![
        FrameQpStats::new(0, 20.0),
        FrameQpStats::new(1, 22.0),
        FrameQpStats::new(2, 25.0),
        FrameQpStats::new(3, 18.0),
        FrameQpStats::new(4, 30.0),
    ]
}

/// Create test slice stats
fn create_test_slice_stats() -> Vec<FrameSliceStats> {
    vec![
        FrameSliceStats::new(0, 1),
        FrameSliceStats::new(1, 2),
        FrameSliceStats::new(2, 4),
        FrameSliceStats::new(3, 1),
        FrameSliceStats::new(4, 8),
    ]
}

// ============================================================================
// TimelineLaneSystem Construction Tests
// ============================================================================

#[cfg(test)]
mod construction_tests {
    use super::*;

    #[test]
    fn test_new_creates_system() {
        // Arrange & Act
        let system = TimelineLaneSystem::new(100);

        // Assert
        assert_eq!(system.total_frames, 100);
        assert!(system.lanes.is_empty());
        assert!(system.marker_clusters.is_empty());
        assert_eq!(system.zoom_level, 1.0);
    }

    #[test]
    fn test_new_zero_frames() {
        // Arrange & Act
        let system = TimelineLaneSystem::new(0);

        // Assert
        assert_eq!(system.total_frames, 0);
    }

    #[test]
    fn test_new_large_frame_count() {
        // Arrange & Act
        let system = TimelineLaneSystem::new(1_000_000);

        // Assert
        assert_eq!(system.total_frames, 1_000_000);
    }
}

// ============================================================================
// Lane Management Tests
// ============================================================================

#[cfg(test)]
mod lane_management_tests {
    use super::*;

    #[test]
    fn test_add_lane() {
        // Arrange
        let mut system = TimelineLaneSystem::new(10);
        let lane = Lane::new(LaneType::QpAvg);

        // Act
        system.add_lane(lane);

        // Assert
        assert_eq!(system.lanes.len(), 1);
        assert_eq!(system.lanes[0].lane_type, LaneType::QpAvg);
    }

    #[test]
    fn test_add_multiple_lanes() {
        // Arrange
        let mut system = TimelineLaneSystem::new(10);

        // Act
        system.add_lane(Lane::new(LaneType::QpAvg));
        system.add_lane(Lane::new(LaneType::SliceCount));
        system.add_lane(Lane::new(LaneType::BitsPerPixel));

        // Assert
        assert_eq!(system.lanes.len(), 3);
    }

    #[test]
    fn test_get_lane_exists() {
        // Arrange
        let mut system = TimelineLaneSystem::new(10);
        system.add_lane(Lane::new(LaneType::QpAvg));

        // Act
        let lane = system.get_lane(LaneType::QpAvg);

        // Assert
        assert!(lane.is_some());
        assert_eq!(lane.unwrap().lane_type, LaneType::QpAvg);
    }

    #[test]
    fn test_get_lane_not_exists() {
        // Arrange
        let system = TimelineLaneSystem::new(10);

        // Act
        let lane = system.get_lane(LaneType::QpAvg);

        // Assert
        assert!(lane.is_none());
    }

    #[test]
    fn test_get_lane_mut() {
        // Arrange
        let mut system = TimelineLaneSystem::new(10);
        system.add_lane(Lane::new(LaneType::QpAvg));

        // Act
        if let Some(lane) = system.get_lane_mut(LaneType::QpAvg) {
            lane.add_point(0, 25.0);
        }

        // Assert
        let lane = system.get_lane(LaneType::QpAvg).unwrap();
        assert_eq!(lane.get_value(0), Some(25.0));
    }

    #[test]
    fn test_get_lane_mut_not_exists() {
        // Arrange
        let mut system = TimelineLaneSystem::new(10);

        // Act
        let lane = system.get_lane_mut(LaneType::QpAvg);

        // Assert
        assert!(lane.is_none());
    }

    #[test]
    fn test_enabled_lanes() {
        // Arrange
        let mut system = TimelineLaneSystem::new(10);
        system.add_lane(Lane::new(LaneType::QpAvg));
        system.add_lane(Lane::new(LaneType::SliceCount));
        system.lanes[1].enabled = false;

        // Act
        let enabled = system.enabled_lanes();

        // Assert
        assert_eq!(enabled.len(), 1);
        assert_eq!(enabled[0].lane_type, LaneType::QpAvg);
    }

    #[test]
    fn test_enabled_count() {
        // Arrange
        let mut system = TimelineLaneSystem::new(10);
        system.add_lane(Lane::new(LaneType::QpAvg));
        system.add_lane(Lane::new(LaneType::SliceCount));
        system.add_lane(Lane::new(LaneType::BitsPerPixel));
        system.lanes[1].enabled = false;

        // Act
        let count = system.enabled_count();

        // Assert
        assert_eq!(count, 2);
    }

    #[test]
    fn test_enabled_count_no_lanes() {
        // Arrange
        let system = TimelineLaneSystem::new(10);

        // Act
        let count = system.enabled_count();

        // Assert
        assert_eq!(count, 0);
    }
}

// ============================================================================
// Lane Toggle Tests
// ============================================================================

#[cfg(test)]
mod lane_toggle_tests {
    use super::*;

    #[test]
    fn test_toggle_lane_enables_to_disabled() {
        // Arrange
        let mut system = TimelineLaneSystem::new(10);
        system.add_lane(Lane::new(LaneType::QpAvg));
        assert!(system.get_lane(LaneType::QpAvg).unwrap().enabled);

        // Act
        system.toggle_lane(LaneType::QpAvg);

        // Assert
        assert!(!system.get_lane(LaneType::QpAvg).unwrap().enabled);
    }

    #[test]
    fn test_toggle_lane_disables_to_enabled() {
        // Arrange
        let mut system = TimelineLaneSystem::new(10);
        system.add_lane(Lane::new(LaneType::QpAvg));
        system.lanes[0].enabled = false;

        // Act
        system.toggle_lane(LaneType::QpAvg);

        // Assert
        assert!(system.get_lane(LaneType::QpAvg).unwrap().enabled);
    }

    #[test]
    fn test_toggle_lane_nonexistent() {
        // Arrange
        let mut system = TimelineLaneSystem::new(10);

        // Act - Should not panic
        system.toggle_lane(LaneType::QpAvg);

        // Assert - No change
        assert_eq!(system.lanes.len(), 0);
    }

    #[test]
    fn test_set_lane_enabled() {
        // Arrange
        let mut system = TimelineLaneSystem::new(10);
        system.add_lane(Lane::new(LaneType::QpAvg));
        system.lanes[0].enabled = false;

        // Act
        system.set_lane_enabled(LaneType::QpAvg, true);

        // Assert
        assert!(system.get_lane(LaneType::QpAvg).unwrap().enabled);
    }

    #[test]
    fn test_set_lane_disabled() {
        // Arrange
        let mut system = TimelineLaneSystem::new(10);
        system.add_lane(Lane::new(LaneType::QpAvg));

        // Act
        system.set_lane_enabled(LaneType::QpAvg, false);

        // Assert
        assert!(!system.get_lane(LaneType::QpAvg).unwrap().enabled);
    }

    #[test]
    fn test_set_lane_enabled_nonexistent() {
        // Arrange
        let mut system = TimelineLaneSystem::new(10);

        // Act - Should not panic
        system.set_lane_enabled(LaneType::QpAvg, true);

        // Assert
        assert_eq!(system.lanes.len(), 0);
    }
}

// ============================================================================
// Marker Clustering Tests
// ============================================================================

#[cfg(test)]
mod marker_clustering_tests {
    use super::*;

    fn create_test_markers() -> Vec<(usize, FrameMarker)> {
        vec![
            (0, FrameMarker::Key),
            (10, FrameMarker::Key),
            (20, FrameMarker::Bookmark),
            (30, FrameMarker::Error),
            (40, FrameMarker::Key),
        ]
    }

    #[test]
    fn test_update_marker_clusters() {
        // Arrange
        let mut system = TimelineLaneSystem::new(100);
        let markers = create_test_markers();

        // Act
        system.update_marker_clusters(&markers);

        // Assert
        assert!(!system.marker_clusters.is_empty());
    }

    #[test]
    fn test_set_zoom_level_updates_clusters() {
        // Arrange
        let mut system = TimelineLaneSystem::new(100);
        let markers = create_test_markers();
        system.update_marker_clusters(&markers);
        let _initial_count = system.marker_clusters.len();

        // Act
        system.set_zoom_level(2.0, &markers);

        // Assert - Zoom change should update clusters
        // Clusters may be different count
        assert!(!system.marker_clusters.is_empty());
    }

    #[test]
    fn test_set_zoom_level_same_value() {
        // Arrange
        let mut system = TimelineLaneSystem::new(100);
        let markers = create_test_markers();
        system.set_zoom_level(1.0, &markers);
        let _initial_count = system.marker_clusters.len();

        // Act - Set same value
        system.set_zoom_level(1.0, &markers);

        // Assert - Should still update clusters
        assert_eq!(system.zoom_level, 1.0);
    }

    #[test]
    fn test_get_cluster_at() {
        // Arrange
        let mut system = TimelineLaneSystem::new(100);
        let markers = vec![(10, FrameMarker::Key), (20, FrameMarker::Bookmark)];
        system.update_marker_clusters(&markers);

        // Act
        let cluster = system.get_cluster_at(10);

        // Assert
        assert!(cluster.is_some());
    }

    #[test]
    fn test_get_cluster_at_not_found() {
        // Arrange
        let mut system = TimelineLaneSystem::new(100);
        let markers = vec![(100, FrameMarker::Key)];
        system.update_marker_clusters(&markers);

        // Act
        let cluster = system.get_cluster_at(0);

        // Assert
        assert!(cluster.is_none());
    }

    #[test]
    fn test_get_cluster_at_range_edge() {
        // Arrange
        let mut system = TimelineLaneSystem::new(100);
        let markers = vec![
            (5, FrameMarker::Key),
            (15, FrameMarker::Bookmark),
            (25, FrameMarker::Key),
        ];
        system.update_marker_clusters(&markers);

        // Act - Check at a position that should be within a cluster
        let cluster = system.get_cluster_at(15);

        // Assert
        assert!(cluster.is_some());
    }
}

// ============================================================================
// Population Methods Tests
// ============================================================================

#[cfg(test)]
mod population_methods_tests {
    use super::*;

    #[test]
    fn test_populate_bpp_lane() {
        // Arrange
        let mut system = TimelineLaneSystem::new(5);
        let frames = create_test_frames(5);

        // Act
        system.populate_bpp_lane(&frames, 1920, 1080);

        // Assert
        assert!(system.get_lane(LaneType::BitsPerPixel).is_some());
        let lane = system.get_lane(LaneType::BitsPerPixel).unwrap();
        assert_eq!(lane.data.len(), 5);
    }

    #[test]
    fn test_populate_bpp_lane_replaces_existing() {
        // Arrange
        let mut system = TimelineLaneSystem::new(3);
        let frames1 = create_test_frames(3);
        let frames2 = create_test_frames(3);

        // Act - Populate twice
        system.populate_bpp_lane(&frames1, 1920, 1080);
        let first_len = system.get_lane(LaneType::BitsPerPixel).unwrap().data.len();

        system.populate_bpp_lane(&frames2, 1920, 1080);
        let second_len = system.get_lane(LaneType::BitsPerPixel).unwrap().data.len();

        // Assert - Should replace, not duplicate
        assert_eq!(first_len, 3);
        assert_eq!(second_len, 3);
        assert_eq!(system.lanes.len(), 1); // Only one lane
    }

    #[test]
    fn test_populate_diagnostics_lane() {
        // Arrange
        let mut system = TimelineLaneSystem::new(10);
        let diagnostics = DiagnosticsManager::new();

        // Act
        system.populate_diagnostics_lane(&diagnostics);

        // Assert
        assert!(system.get_lane(LaneType::DiagnosticsDensity).is_some());
    }

    #[test]
    fn test_populate_reorder_lane() {
        // Arrange
        let mut system = TimelineLaneSystem::new(5);
        let frames = create_test_frames(5);

        // Act
        system.populate_reorder_lane(&frames);

        // Assert
        assert!(system.get_lane(LaneType::ReorderMismatch).is_some());
        let lane = system.get_lane(LaneType::ReorderMismatch).unwrap();
        assert_eq!(lane.data.len(), 5);
    }

    #[test]
    fn test_populate_qp_lane() {
        // Arrange
        let mut system = TimelineLaneSystem::new(10);
        let qp_data = create_test_qp_stats();

        // Act
        system.populate_qp_lane(&qp_data);

        // Assert
        assert!(system.get_lane(LaneType::QpAvg).is_some());
        let lane = system.get_lane(LaneType::QpAvg).unwrap();
        assert_eq!(lane.data.len(), 5);
    }

    #[test]
    fn test_populate_slice_lane() {
        // Arrange
        let mut system = TimelineLaneSystem::new(10);
        let slice_data = create_test_slice_stats();

        // Act
        system.populate_slice_lane(&slice_data);

        // Assert
        assert!(system.get_lane(LaneType::SliceCount).is_some());
        let lane = system.get_lane(LaneType::SliceCount).unwrap();
        assert_eq!(lane.data.len(), 5);
    }
}

// ============================================================================
// Statistics Methods Tests
// ============================================================================

#[cfg(test)]
mod statistics_methods_tests {
    use super::*;

    #[test]
    fn test_qp_statistics() {
        // Arrange
        let mut system = TimelineLaneSystem::new(10);
        let qp_data = create_test_qp_stats();
        system.populate_qp_lane(&qp_data);

        // Act
        let stats = system.qp_statistics();

        // Assert
        assert!(stats.is_some());
        let s = stats.unwrap();
        assert_eq!(s.min_qp, 18.0);
        assert_eq!(s.max_qp, 30.0);
        assert_eq!(s.avg_qp, 23.0);
    }

    #[test]
    fn test_qp_statistics_no_lane() {
        // Arrange
        let system = TimelineLaneSystem::new(10);

        // Act
        let stats = system.qp_statistics();

        // Assert
        assert!(stats.is_none());
    }

    #[test]
    fn test_slice_statistics() {
        // Arrange
        let mut system = TimelineLaneSystem::new(10);
        let slice_data = create_test_slice_stats();
        system.populate_slice_lane(&slice_data);

        // Act
        let stats = system.slice_statistics();

        // Assert
        assert!(stats.is_some());
        let s = stats.unwrap();
        assert_eq!(s.min_slices, 1);
        assert_eq!(s.max_slices, 8);
    }

    #[test]
    fn test_slice_statistics_no_lane() {
        // Arrange
        let system = TimelineLaneSystem::new(10);

        // Act
        let stats = system.slice_statistics();

        // Assert
        assert!(stats.is_none());
    }
}

// ============================================================================
// Edge Cases
// ============================================================================

#[cfg(test)]
mod edge_case_tests {
    use super::*;

    #[test]
    fn test_empty_marker_clusters() {
        // Arrange
        let mut system = TimelineLaneSystem::new(100);
        let markers = vec![];

        // Act
        system.update_marker_clusters(&markers);

        // Assert
        assert!(system.marker_clusters.is_empty());
    }

    #[test]
    fn test_none_marker_filtered() {
        // Arrange
        let mut system = TimelineLaneSystem::new(100);
        let markers = vec![
            (0, FrameMarker::Key),
            (10, FrameMarker::None),
            (20, FrameMarker::Key),
        ];

        // Act
        system.update_marker_clusters(&markers);

        // Assert - None markers should be filtered
        // Cluster count should be less than markers length
        assert!(system.marker_clusters.len() <= 2);
    }

    #[test]
    fn test_multiple_lanes_same_type() {
        // Arrange
        let mut system = TimelineLaneSystem::new(10);

        // Act - Add same type multiple times
        system.add_lane(Lane::new(LaneType::QpAvg));
        system.add_lane(Lane::new(LaneType::QpAvg));
        system.add_lane(Lane::new(LaneType::QpAvg));

        // Assert - All are stored (no deduplication in implementation)
        assert_eq!(system.lanes.len(), 3);
    }

    #[test]
    fn test_zoom_level_extreme() {
        // Arrange
        let mut system = TimelineLaneSystem::new(100);
        let markers = vec![(0, FrameMarker::Key), (50, FrameMarker::Key)];

        // Act - Very high zoom
        system.set_zoom_level(100.0, &markers);

        // Assert
        assert_eq!(system.zoom_level, 100.0);
        assert!(!system.marker_clusters.is_empty());
    }

    #[test]
    fn test_zoom_level_zero() {
        // Arrange
        let mut system = TimelineLaneSystem::new(100);
        let markers = vec![(0, FrameMarker::Key)];

        // Act - Zero zoom (edge case)
        system.set_zoom_level(0.0, &markers);

        // Assert
        assert_eq!(system.zoom_level, 0.0);
    }

    #[test]
    fn test_total_frames_zero() {
        // Arrange
        let mut system = TimelineLaneSystem::new(0);
        let markers = vec![];

        // Act
        system.update_marker_clusters(&markers);

        // Assert
        assert!(system.marker_clusters.is_empty());
    }

    #[test]
    fn test_get_cluster_at_empty_clusters() {
        // Arrange
        let system = TimelineLaneSystem::new(100);

        // Act
        let cluster = system.get_cluster_at(50);

        // Assert
        assert!(cluster.is_none());
    }
}
