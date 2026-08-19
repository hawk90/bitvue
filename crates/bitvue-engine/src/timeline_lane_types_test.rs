// Timeline lane types module tests
//
// Per generate-tests skill: Arrange-Act-Assert pattern with fixtures
// and edge case coverage.

use super::*;

// ============================================================================
// FrameQpStats Tests
// ============================================================================

#[cfg(test)]
mod frame_qp_stats_tests {
    use super::*;

    #[test]
    fn test_frame_qp_stats_new() {
        // Arrange & Act
        let stats = FrameQpStats::new(100, 25.0);

        // Assert
        assert_eq!(stats.display_idx, 100);
        assert_eq!(stats.qp_avg, 25.0);
        assert!(stats.qp_min.is_none());
        assert!(stats.qp_max.is_none());
    }

    #[test]
    fn test_frame_qp_stats_with_range() {
        // Arrange & Act
        let stats = FrameQpStats::with_range(100, 25.0, 20, 30);

        // Assert
        assert_eq!(stats.qp_min, Some(20));
        assert_eq!(stats.qp_max, Some(30));
    }

    #[test]
    fn test_frame_qp_stats_from_range() {
        // Arrange & Act
        let stats = FrameQpStats::from_range(100, 20, 30);

        // Assert
        assert_eq!(stats.qp_avg, 25.0); // (20 + 30) / 2
        assert_eq!(stats.qp_min, Some(20));
        assert_eq!(stats.qp_max, Some(30));
    }

    #[test]
    fn test_frame_qp_stats_equality() {
        // Arrange
        let stats1 = FrameQpStats::new(100, 25.0);
        let stats2 = FrameQpStats::new(100, 25.0);
        let stats3 = FrameQpStats::new(100, 30.0);

        // Assert
        assert_eq!(stats1, stats2);
        assert_ne!(stats1, stats3);
    }
}

// ============================================================================
// FrameSliceStats Tests
// ============================================================================

#[cfg(test)]
mod frame_slice_stats_tests {
    use super::*;

    #[test]
    fn test_frame_slice_stats_new() {
        // Arrange & Act
        let stats = FrameSliceStats::new(100, 5);

        // Assert
        assert_eq!(stats.display_idx, 100);
        assert_eq!(stats.slice_count, 5);
        assert!(stats.tile_cols.is_none());
        assert!(stats.tile_rows.is_none());
    }

    #[test]
    fn test_frame_slice_stats_with_tile_grid() {
        // Arrange & Act
        let stats = FrameSliceStats::with_tile_grid(100, 20, 4, 5);

        // Assert
        assert_eq!(stats.slice_count, 20);
        assert_eq!(stats.tile_cols, Some(4));
        assert_eq!(stats.tile_rows, Some(5));
    }

    #[test]
    fn test_frame_slice_stats_equality() {
        // Arrange
        let stats1 = FrameSliceStats::new(100, 5);
        let stats2 = FrameSliceStats::new(100, 5);
        let stats3 = FrameSliceStats::new(100, 10);

        // Assert
        assert_eq!(stats1, stats2);
        assert_ne!(stats1, stats3);
    }
}

// ============================================================================
// QpLaneStatistics Tests
// ============================================================================

#[cfg(test)]
mod qp_lane_statistics_tests {
    use super::*;

    #[test]
    fn test_qp_lane_statistics_fields() {
        // Arrange & Act
        let stats = QpLaneStatistics {
            min_qp: 10.0,
            max_qp: 40.0,
            avg_qp: 25.0,
            std_dev: 5.0,
            frame_count: 100,
        };

        // Assert
        assert_eq!(stats.min_qp, 10.0);
        assert_eq!(stats.max_qp, 40.0);
        assert_eq!(stats.avg_qp, 25.0);
        assert_eq!(stats.std_dev, 5.0);
        assert_eq!(stats.frame_count, 100);
    }

    #[test]
    fn test_qp_lane_statistics_summary_text() {
        // Arrange
        let stats = QpLaneStatistics {
            min_qp: 10.0,
            max_qp: 40.0,
            avg_qp: 25.0,
            std_dev: 5.0,
            frame_count: 100,
        };

        // Act
        let text = stats.summary_text();

        // Assert
        assert!(text.contains("min 10"));
        assert!(text.contains("max 40"));
        assert!(text.contains("avg 25"));
        assert!(text.contains("σ 5.00"));
        assert!(text.contains("[100 frames]"));
    }
}

// ============================================================================
// SliceLaneStatistics Tests
// ============================================================================

#[cfg(test)]
mod slice_lane_statistics_tests {
    use super::*;

    #[test]
    fn test_slice_lane_statistics_fields() {
        // Arrange & Act
        let stats = SliceLaneStatistics {
            min_slices: 1,
            max_slices: 10,
            avg_slices: 5.5,
            multi_slice_frame_count: 50,
            frame_count: 100,
        };

        // Assert
        assert_eq!(stats.min_slices, 1);
        assert_eq!(stats.max_slices, 10);
        assert_eq!(stats.avg_slices, 5.5);
        assert_eq!(stats.multi_slice_frame_count, 50);
        assert_eq!(stats.frame_count, 100);
    }

    #[test]
    fn test_slice_lane_statistics_summary_text() {
        // Arrange
        let stats = SliceLaneStatistics {
            min_slices: 1,
            max_slices: 10,
            avg_slices: 5.5,
            multi_slice_frame_count: 50,
            frame_count: 100,
        };

        // Act
        let text = stats.summary_text();

        // Assert
        assert!(text.contains("min 1"));
        assert!(text.contains("max 10"));
        assert!(text.contains("avg 5.5"));
        assert!(text.contains("[50/100 multi-slice]"));
    }
}

// ============================================================================
// calculate_bpp Tests
// ============================================================================

#[cfg(test)]
mod calculate_bpp_tests {
    use super::*;

    #[test]
    fn test_calculate_bpp_normal() {
        // Arrange
        let frame_size = 100_000; // 100KB
        let width = 1920;
        let height = 1080;

        // Act
        let bpp = calculate_bpp(frame_size, width, height);

        // Assert
        let expected = (frame_size * 8) as f32 / (width * height) as f32;
        assert!((bpp - expected).abs() < 0.001);
    }

    #[test]
    fn test_calculate_bpp_zero_pixel_count() {
        // Arrange
        let frame_size = 1000;
        let width = 0;
        let height = 100;

        // Act
        let bpp = calculate_bpp(frame_size, width, height);

        // Assert
        assert_eq!(bpp, 0.0);
    }

    #[test]
    fn test_calculate_bpp_zero_frame_size() {
        // Arrange
        let frame_size = 0;
        let width = 1920;
        let height = 1080;

        // Act
        let bpp = calculate_bpp(frame_size, width, height);

        // Assert
        assert_eq!(bpp, 0.0);
    }

    #[test]
    fn test_calculate_bpp_full_hd() {
        // Arrange
        let frame_size = 200_000; // 200KB frame
        let width = 1920;
        let height = 1080;

        // Act
        let bpp = calculate_bpp(frame_size, width, height);

        // Assert - 200KB * 8 / (1920 * 1080) pixels
        let expected = (200_000.0 * 8.0) / (1920.0 * 1080.0);
        assert!((bpp - expected).abs() < 0.01);
    }
}

// ============================================================================
// estimate_qp_avg Tests
// ============================================================================

#[cfg(test)]
mod estimate_qp_avg_tests {
    use super::*;

    #[test]
    fn test_estimate_qp_avg_normal() {
        // Arrange & Act
        let avg = estimate_qp_avg(20, 30);

        // Assert
        assert_eq!(avg, 25.0);
    }

    #[test]
    fn test_estimate_qp_avg_same_values() {
        // Arrange & Act
        let avg = estimate_qp_avg(25, 25);

        // Assert
        assert_eq!(avg, 25.0);
    }

    #[test]
    fn test_estimate_qp_avg_extreme() {
        // Arrange & Act
        let avg = estimate_qp_avg(0, 51);

        // Assert
        assert_eq!(avg, 25.5);
    }

    #[test]
    fn test_estimate_qp_avg_negative() {
        // Arrange & Act
        let avg = estimate_qp_avg(-10, 20);

        // Assert
        assert_eq!(avg, 5.0);
    }
}

// ============================================================================
// Edge Cases
// ============================================================================

#[cfg(test)]
mod edge_case_tests {
    use super::*;

    #[test]
    fn test_frame_qp_stats_negative_qp() {
        // Arrange & Act
        let stats = FrameQpStats::with_range(0, -10.0, -20, -5);

        // Assert - Should work with negative QP
        assert_eq!(stats.qp_avg, -10.0);
        assert_eq!(stats.qp_min, Some(-20));
        assert_eq!(stats.qp_max, Some(-5));
    }

    #[test]
    fn test_frame_slice_stats_zero_slices() {
        // Arrange & Act
        let stats = FrameSliceStats::new(0, 0);

        // Assert
        assert_eq!(stats.slice_count, 0);
    }

    #[test]
    fn test_frame_slice_stats_large_grid() {
        // Arrange & Act
        let stats = FrameSliceStats::with_tile_grid(0, 100, 10, 20);

        // Assert
        assert_eq!(stats.tile_cols, Some(10));
        assert_eq!(stats.tile_rows, Some(20));
    }

    #[test]
    fn test_qp_lane_statistics_zero_std_dev() {
        // Arrange & Act
        let stats = QpLaneStatistics {
            min_qp: 25.0,
            max_qp: 25.0,
            avg_qp: 25.0,
            std_dev: 0.0,
            frame_count: 10,
        };

        // Assert
        assert_eq!(stats.std_dev, 0.0);
    }

    #[test]
    fn test_qp_lane_statistics_single_frame() {
        // Arrange & Act
        let stats = QpLaneStatistics {
            min_qp: 25.0,
            max_qp: 25.0,
            avg_qp: 25.0,
            std_dev: 0.0,
            frame_count: 1,
        };

        // Assert
        assert_eq!(stats.frame_count, 1);
    }

    #[test]
    fn test_slice_lane_statistics_no_multi_slice() {
        // Arrange & Act
        let stats = SliceLaneStatistics {
            min_slices: 5,
            max_slices: 5,
            avg_slices: 5.0,
            multi_slice_frame_count: 0,
            frame_count: 100,
        };

        // Assert
        assert_eq!(stats.multi_slice_frame_count, 0);
    }

    #[test]
    fn test_slice_lane_statistics_all_multi_slice() {
        // Arrange & Act
        let stats = SliceLaneStatistics {
            min_slices: 2,
            max_slices: 5,
            avg_slices: 4.0,
            multi_slice_frame_count: 100,
            frame_count: 100,
        };

        // Assert
        assert_eq!(stats.multi_slice_frame_count, 100);
    }
}
