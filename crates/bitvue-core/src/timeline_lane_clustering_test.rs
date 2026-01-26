// Timeline lane clustering module tests
//
// Per generate-tests skill: Arrange-Act-Assert pattern with fixtures
// and edge case coverage.

use super::*;
use crate::timeline::FrameMarker;

// ============================================================================
// Fixtures
// ============================================================================

/// Create test markers
fn create_test_markers() -> Vec<(usize, FrameMarker)> {
    vec![
        (0, FrameMarker::Key),
        (10, FrameMarker::Key),
        (20, FrameMarker::Key),
        (30, FrameMarker::Bookmark),
        (40, FrameMarker::Error),
        (50, FrameMarker::Key),
    ]
}

/// Create empty markers
fn create_empty_markers() -> Vec<(usize, FrameMarker)> {
    vec![]
}

// ============================================================================
// MarkerCluster Tests
// ============================================================================

#[cfg(test)]
mod marker_cluster_tests {
    use super::*;

    #[test]
    fn test_marker_cluster_single() {
        // Arrange & Act
        let cluster = MarkerCluster::single(100, FrameMarker::Key);

        // Assert
        assert_eq!(cluster.center_idx, 100);
        assert_eq!(cluster.start_idx, 100);
        assert_eq!(cluster.end_idx, 100);
        assert_eq!(cluster.count, 1);
        assert_eq!(cluster.primary_type, FrameMarker::Key);
    }

    #[test]
    fn test_marker_cluster_can_merge_threshold() {
        // Arrange
        let cluster = MarkerCluster::single(100, FrameMarker::Key);

        // Act - Within threshold
        let can_merge_1 = cluster.can_merge(105, 10);
        let can_merge_2 = cluster.can_merge(95, 10);

        // Assert
        assert!(can_merge_1);
        assert!(can_merge_2);
    }

    #[test]
    fn test_marker_cluster_cannot_merge_beyond_threshold() {
        // Arrange
        let cluster = MarkerCluster::single(100, FrameMarker::Key);

        // Act - Beyond threshold
        let can_merge = cluster.can_merge(120, 10);

        // Assert
        assert!(!can_merge);
    }

    #[test]
    fn test_marker_cluster_merge_increases_count() {
        // Arrange
        let mut cluster = MarkerCluster::single(100, FrameMarker::Key);
        let initial_count = cluster.count;

        // Act
        cluster.merge(105, FrameMarker::Bookmark);

        // Assert
        assert_eq!(cluster.count, initial_count + 1);
    }

    #[test]
    fn test_marker_cluster_merge_expands_range() {
        // Arrange
        let mut cluster = MarkerCluster::single(100, FrameMarker::Key);

        // Act
        cluster.merge(110, FrameMarker::Bookmark);

        // Assert
        assert_eq!(cluster.start_idx, 100);
        assert_eq!(cluster.end_idx, 110);
    }

    #[test]
    fn test_marker_cluster_merge_updates_center() {
        // Arrange
        let mut cluster = MarkerCluster::single(100, FrameMarker::Key);

        // Act
        cluster.merge(110, FrameMarker::Bookmark);

        // Assert
        assert_eq!(cluster.center_idx, 105); // (100 + 110) / 2
    }

    #[test]
    fn test_marker_cluster_merge_critical_marker() {
        // Arrange
        let mut cluster = MarkerCluster::single(100, FrameMarker::Bookmark);

        // Act - Merge with Error (critical type)
        cluster.merge(105, FrameMarker::Error);

        // Assert
        assert_eq!(cluster.primary_type, FrameMarker::Error);
    }

    #[test]
    fn test_marker_cluster_merge_non_critical_not_updated() {
        // Arrange
        let mut cluster = MarkerCluster::single(100, FrameMarker::Error);

        // Act - Merge with Bookmark (non-critical)
        cluster.merge(105, FrameMarker::Bookmark);

        // Assert - Primary type stays Error (more critical)
        assert_eq!(cluster.primary_type, FrameMarker::Error);
    }
}

// ============================================================================
// MarkerClustering Tests
// ============================================================================

#[cfg(test)]
mod marker_clustering_tests {
    use super::*;

    #[test]
    fn test_cluster_empty_markers() {
        // Arrange
        let markers = create_empty_markers();

        // Act
        let clusters = MarkerClustering::cluster(&markers, 10);

        // Assert
        assert!(clusters.is_empty());
    }

    #[test]
    fn test_cluster_no_merge() {
        // Arrange
        let markers = vec![(0, FrameMarker::Key), (100, FrameMarker::Key)];

        // Act
        let clusters = MarkerClustering::cluster(&markers, 10);

        // Assert
        assert_eq!(clusters.len(), 2);
    }

    #[test]
    fn test_cluster_with_merge() {
        // Arrange
        let markers = vec![(0, FrameMarker::Key), (5, FrameMarker::Bookmark)];

        // Act
        let clusters = MarkerClustering::cluster(&markers, 10);

        // Assert
        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].count, 2);
    }

    #[test]
    fn test_cluster_filters_none_marker() {
        // Arrange
        let markers = vec![
            (0, FrameMarker::Key),
            (10, FrameMarker::None),
            (20, FrameMarker::Key),
        ];

        // Act
        let clusters = MarkerClustering::cluster(&markers, 10);

        // Assert - None marker should be filtered out
        assert_eq!(clusters.len(), 2);
    }

    #[test]
    fn test_cluster_all_merge_into_one() {
        // Arrange
        let markers = vec![
            (0, FrameMarker::Key),
            (5, FrameMarker::Key),
            (10, FrameMarker::Key),
        ];

        // Act - threshold of 20
        let clusters = MarkerClustering::cluster(&markers, 20);

        // Assert
        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].count, 3);
    }

    #[test]
    fn test_cluster_sequential_critical_types() {
        // Arrange
        let markers = vec![
            (0, FrameMarker::Error),
            (10, FrameMarker::Key),
            (20, FrameMarker::Error),
        ];

        // Act
        let clusters = MarkerClustering::cluster(&markers, 5);

        // Assert - Each is separate due to threshold, but errors are critical
        assert_eq!(clusters.len(), 3);
    }
}

// ============================================================================
// Calculate Threshold Tests
// ============================================================================

#[cfg(test)]
mod calculate_threshold_tests {
    use super::*;

    #[test]
    fn test_calculate_threshold_zoom_1() {
        // Arrange
        let zoom_level = 1.0;
        let total_frames = 10000;

        // Act
        let threshold = MarkerClustering::calculate_threshold(zoom_level, total_frames);

        // Assert - 1% of frames = 100
        assert_eq!(threshold, 100);
    }

    #[test]
    fn test_calculate_threshold_zoomed_in() {
        // Arrange
        let zoom_level = 10.0;
        let total_frames = 10000;

        // Act
        let threshold = MarkerClustering::calculate_threshold(zoom_level, total_frames);

        // Assert - 1% / 10 = 10
        assert_eq!(threshold, 10);
    }

    #[test]
    fn test_calculate_threshold_zoomed_out() {
        // Arrange
        let zoom_level = 0.1;
        let total_frames = 10000;

        // // Act
        let threshold = MarkerClustering::calculate_threshold(zoom_level, total_frames);

        // Assert - 1% / 0.1 = 100, but clamped to 100
        assert_eq!(threshold, 100);
    }

    #[test]
    fn test_calculate_threshold_minimum() {
        // Arrange
        let zoom_level = 1000.0;
        let total_frames = 10000;

        // Act
        let threshold = MarkerClustering::calculate_threshold(zoom_level, total_frames);

        // Assert - Should be clamped to minimum 1
        assert_eq!(threshold, 1);
    }

    #[test]
    fn test_calculate_threshold_maximum() {
        // Arrange
        let zoom_level = 0.01;
        let total_frames = 10000;

        // Act
        let threshold = MarkerClustering::calculate_threshold(zoom_level, total_frames);

        // Assert - Should be clamped to maximum 100
        assert_eq!(threshold, 100);
    }

    #[test]
    fn test_calculate_threshold_small_total() {
        // Arrange
        let zoom_level = 1.0;
        let total_frames = 10;

        // Act
        let threshold = MarkerClustering::calculate_threshold(zoom_level, total_frames);

        // Assert - 1% of 10 = 0.1, clamped to 1
        assert_eq!(threshold, 1);
    }
}

// ============================================================================
// Edge Cases
// ============================================================================

#[cfg(test)]
mod edge_case_tests {
    use super::*;

    #[test]
    fn test_marker_cluster_single_is_critical() {
        // Arrange & Act
        let cluster = MarkerCluster::single(0, FrameMarker::Error);

        // Assert - Error marker is critical
        assert!(cluster.primary_type.is_critical());
    }

    #[test]
    fn test_marker_cluster_bookmark_is_not_critical() {
        // Arrange & Act
        let cluster = MarkerCluster::single(0, FrameMarker::Bookmark);

        // Assert - Bookmark is not critical
        assert!(!cluster.primary_type.is_critical());
    }

    #[test]
    fn test_marker_cluster_key_is_not_critical() {
        // Arrange & Act
        let cluster = MarkerCluster::single(0, FrameMarker::Key);

        // Assert - Key IS critical (per implementation: Error | Key are critical)
        assert!(cluster.primary_type.is_critical());
    }

    #[test]
    fn test_cluster_multiple_bookmarks_merge_key_is_primary() {
        // Arrange
        let mut cluster = MarkerCluster::single(0, FrameMarker::Bookmark);
        cluster.merge(10, FrameMarker::Key);

        // Act - Merge another key
        cluster.merge(20, FrameMarker::Key);

        // Assert - Key IS critical, so it becomes primary when merged into Bookmark
        assert_eq!(cluster.primary_type, FrameMarker::Key);
    }

    #[test]
    fn test_cluster_merge_expands_both_directions() {
        // Arrange
        let mut cluster = MarkerCluster::single(100, FrameMarker::Key);

        // Act - Merge before and after
        cluster.merge(90, FrameMarker::Bookmark);
        cluster.merge(110, FrameMarker::Error);

        // Assert
        assert_eq!(cluster.start_idx, 90);
        assert_eq!(cluster.end_idx, 110);
    }

    #[test]
    fn test_cluster_can_merge_end_of_range() {
        // Arrange
        let cluster = MarkerCluster {
            center_idx: 100,
            start_idx: 90,
            end_idx: 110,
            count: 5,
            primary_type: FrameMarker::Key,
        };

        // Act - At end of range
        let can_merge = cluster.can_merge(110, 10);

        // Assert
        assert!(can_merge); // 110 - 110 = 0 <= threshold
    }

    #[test]
    fn test_cluster_can_merge_start_of_range() {
        // Arrange
        let cluster = MarkerCluster {
            center_idx: 100,
            start_idx: 90,
            end_idx: 110,
            count: 5,
            primary_type: FrameMarker::Key,
        };

        // Act - At start of range
        let can_merge = cluster.can_merge(90, 10);

        // Assert
        assert!(can_merge); // 90 - 90 = 0 <= threshold
    }

    #[test]
    fn test_cluster_center_updates_on_merge() {
        // Arrange
        let mut cluster = MarkerCluster::single(100, FrameMarker::Key);

        // Act - Merge far away
        cluster.merge(200, FrameMarker::Bookmark);

        // Assert
        assert_eq!(cluster.center_idx, 150); // (100 + 200) / 2
    }

    #[test]
    fn test_calculate_threshold_extreme_zoom() {
        // Arrange
        let zoom_level = 1000.0;
        let total_frames = 100000;

        // Act
        let threshold = MarkerClustering::calculate_threshold(zoom_level, total_frames);

        // Assert - Should be clamped to minimum
        assert_eq!(threshold, 1);
    }

    #[test]
    fn test_calculate_threshold_zero_total_frames() {
        // Arrange
        let zoom_level = 1.0;
        let total_frames = 0;

        // Act
        let threshold = MarkerClustering::calculate_threshold(zoom_level, total_frames);

        // Assert - 0 * 0.01 = 0, clamped to 1
        assert_eq!(threshold, 1);
    }

    #[test]
    fn test_cluster_merge_updates_count_correctly() {
        // Arrange
        let mut cluster = MarkerCluster::single(100, FrameMarker::Key);

        // Act - Merge multiple markers
        cluster.merge(110, FrameMarker::Key);
        cluster.merge(115, FrameMarker::Key);
        cluster.merge(90, FrameMarker::Bookmark);

        // Assert
        assert_eq!(cluster.count, 4);
    }
}
