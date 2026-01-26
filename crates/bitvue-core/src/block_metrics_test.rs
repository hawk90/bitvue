// BlockMetrics module tests
//
// Per generate-tests skill: Arrange-Act-Assert pattern with fixtures
// and edge case coverage.

use super::*;

// ============================================================================
// Fixtures
// ============================================================================

/// Create a test block metric value
fn create_test_block_metric_value() -> BlockMetricValue {
    BlockMetricValue::new(0, 0, 35.5, BlockMetricType::Psnr)
}

/// Create a test block metrics grid
fn create_test_block_metrics_grid() -> BlockMetricsGrid {
    BlockMetricsGrid::new(0, 1920, 1080, 16, BlockMetricType::Psnr)
}

// ============================================================================
// BlockMetricType Tests
// ============================================================================

#[cfg(test)]
mod block_metric_type_tests {
    use super::*;

    #[test]
    fn test_block_metric_type_psnr() {
        // Arrange & Act
        let metric_type = BlockMetricType::Psnr;

        // Assert
        assert_eq!(metric_type.name(), "PSNR");
        assert_eq!(metric_type.unit(), "dB");
        assert!(metric_type.higher_is_better());
        let (min, max) = metric_type.typical_range();
        assert_eq!(min, 20.0);
        assert_eq!(max, 50.0);
    }

    #[test]
    fn test_block_metric_type_ssim() {
        // Arrange & Act
        let metric_type = BlockMetricType::Ssim;

        // Assert
        assert_eq!(metric_type.name(), "SSIM");
        assert_eq!(metric_type.unit(), "");
        assert!(metric_type.higher_is_better());
        let (min, max) = metric_type.typical_range();
        assert_eq!(min, 0.0);
        assert_eq!(max, 1.0);
    }

    #[test]
    fn test_block_metric_type_mse() {
        // Arrange & Act
        let metric_type = BlockMetricType::Mse;

        // Assert
        assert_eq!(metric_type.name(), "MSE");
        assert_eq!(metric_type.unit(), "");
        assert!(!metric_type.higher_is_better()); // Lower is better
        let (min, max) = metric_type.typical_range();
        assert_eq!(min, 0.0);
        assert_eq!(max, 1000.0);
    }

    #[test]
    fn test_block_metric_type_mad() {
        // Arrange & Act
        let metric_type = BlockMetricType::Mad;

        // Assert
        assert_eq!(metric_type.name(), "MAD");
        assert_eq!(metric_type.unit(), "");
        assert!(!metric_type.higher_is_better()); // Lower is better
        let (min, max) = metric_type.typical_range();
        assert_eq!(min, 0.0);
        assert_eq!(max, 100.0);
    }

    #[test]
    fn test_block_metric_type_default() {
        // Arrange & Act
        let metric_type = BlockMetricType::default();

        // Assert
        assert_eq!(metric_type, BlockMetricType::Psnr);
    }
}

// ============================================================================
// BlockMetricValue Tests
// ============================================================================

#[cfg(test)]
mod block_metric_value_tests {
    use super::*;

    #[test]
    fn test_block_metric_value_new() {
        // Arrange & Act
        let value = BlockMetricValue::new(5, 10, 30.0, BlockMetricType::Psnr);

        // Assert
        assert_eq!(value.block_x, 5);
        assert_eq!(value.block_y, 10);
        assert_eq!(value.value, 30.0);
        assert_eq!(value.metric_type, BlockMetricType::Psnr);
    }

    #[test]
    fn test_block_metric_value_normalized_psnr() {
        // Arrange - PSNR range is 20-50
        let value = BlockMetricValue::new(0, 0, 35.0, BlockMetricType::Psnr);

        // Act
        let normalized = value.normalized();

        // Assert - 35 is midpoint of 20-50 range
        assert_eq!(normalized, 0.5);
    }

    #[test]
    fn test_block_metric_value_normalized_clamped_low() {
        // Arrange
        let value = BlockMetricValue::new(0, 0, 10.0, BlockMetricType::Psnr);

        // Act
        let normalized = value.normalized();

        // Assert - Below range, should clamp to 0
        assert_eq!(normalized, 0.0);
    }

    #[test]
    fn test_block_metric_value_normalized_clamped_high() {
        // Arrange
        let value = BlockMetricValue::new(0, 0, 100.0, BlockMetricType::Psnr);

        // Act
        let normalized = value.normalized();

        // Assert - Above range, should clamp to 1
        assert_eq!(normalized, 1.0);
    }

    #[test]
    fn test_block_metric_value_copy() {
        // Arrange
        let value1 = BlockMetricValue::new(1, 2, 40.0, BlockMetricType::Ssim);

        // Act
        let value2 = value1;

        // Assert - BlockMetricValue is Copy
        assert_eq!(value1.block_x, 1);
        assert_eq!(value2.block_x, 1);
    }
}

// ============================================================================
// BlockMetricsGrid Tests
// ============================================================================

#[cfg(test)]
mod block_metrics_grid_tests {
    use super::*;

    #[test]
    fn test_block_metrics_grid_new() {
        // Arrange & Act
        let grid = BlockMetricsGrid::new(0, 1920, 1080, 16, BlockMetricType::Psnr);

        // Assert
        assert_eq!(grid.display_idx, 0);
        assert_eq!(grid.frame_width, 1920);
        assert_eq!(grid.frame_height, 1080);
        assert_eq!(grid.block_size, 16);
        assert_eq!(grid.width_blocks, 120); // 1920 / 16
        assert_eq!(grid.height_blocks, 68); // 1080 / 16, rounded up
        // All values are initialized to 0.0, not empty
        assert_eq!(grid.values.len(), 120 * 68);
        assert!(grid.values.iter().all(|&v| v == 0.0));
    }

    #[test]
    fn test_block_metrics_grid_get_set() {
        // Arrange
        let mut grid = BlockMetricsGrid::new(0, 640, 480, 32, BlockMetricType::Psnr);

        // Act
        grid.set(0, 0, 35.0);
        let value = grid.get(0, 0);

        // Assert
        assert_eq!(value, Some(35.0));
    }

    #[test]
    fn test_block_metrics_grid_get_out_of_bounds() {
        // Arrange
        let grid = BlockMetricsGrid::new(0, 640, 480, 32, BlockMetricType::Psnr);

        // Act
        let value = grid.get(1000, 1000); // Way out of bounds

        // Assert
        assert!(value.is_none());
    }

    #[test]
    fn test_block_metrics_grid_set_out_of_bounds() {
        // Arrange
        let mut grid = BlockMetricsGrid::new(0, 640, 480, 32, BlockMetricType::Psnr);

        // Act - Should not crash, just do nothing
        grid.set(1000, 1000, 50.0);

        // Assert - Values should remain unchanged
        assert!(grid.values.iter().all(|&v| v == 0.0));
    }

    #[test]
    fn test_block_metrics_grid_get_normalized() {
        // Arrange
        let mut grid = BlockMetricsGrid::new(0, 640, 480, 32, BlockMetricType::Psnr);
        grid.set(5, 3, 35.0); // Midpoint of PSNR range

        // Act
        let normalized = grid.get_normalized(5, 3);

        // Assert
        assert_eq!(normalized, Some(0.5));
    }

    #[test]
    fn test_block_metrics_grid_total_blocks() {
        // Arrange
        let grid = BlockMetricsGrid::new(0, 320, 240, 16, BlockMetricType::Psnr);

        // Act
        let total = grid.total_blocks();

        // Assert - 320/16 = 20, 240/16 = 15, total = 300
        assert_eq!(total, 300);
    }

    #[test]
    fn test_block_metrics_grid_pixel_to_block() {
        // Arrange
        let grid = BlockMetricsGrid::new(0, 640, 480, 32, BlockMetricType::Psnr);

        // Act
        let (block_x, block_y) = grid.pixel_to_block(100, 200);

        // Assert - 100/32 = 3, 200/32 = 6
        assert_eq!(block_x, 3);
        assert_eq!(block_y, 6);
    }

    #[test]
    fn test_block_metrics_grid_block_to_pixel_range() {
        // Arrange
        let grid = BlockMetricsGrid::new(0, 640, 480, 32, BlockMetricType::Psnr);

        // Act
        let (x0, y0, x1, y1) = grid.block_to_pixel_range(5, 3);

        // Assert - Block (5,3) => pixels [160, 96) to [192, 128)
        assert_eq!(x0, 160);
        assert_eq!(y0, 96);
        assert_eq!(x1, 192);
        assert_eq!(y1, 128);
    }

    #[test]
    fn test_block_metrics_grid_large_dimensions() {
        // Arrange & Act
        let grid = BlockMetricsGrid::new(0, 3840, 2160, 32, BlockMetricType::Psnr);

        // Assert - 4K video with 32px blocks
        assert_eq!(grid.width_blocks, 120); // 3840 / 32
        assert_eq!(grid.height_blocks, 68); // 2160 / 32, rounded up
    }

    #[test]
    fn test_block_metrics_grid_ssim_metric() {
        // Arrange & Act
        let mut grid = BlockMetricsGrid::new(0, 640, 480, 32, BlockMetricType::Ssim);
        grid.set(0, 0, 0.5); // Midpoint of SSIM range

        // Act
        let normalized = grid.get_normalized(0, 0);

        // Assert
        assert_eq!(normalized, Some(0.5));
    }
}

// ============================================================================
// Edge Cases
// ============================================================================

#[cfg(test)]
mod edge_case_tests {
    use super::*;

    #[test]
    fn test_normalized_equal_min_max() {
        // Arrange - Create a custom type with equal min/max for edge case
        let value = BlockMetricValue::new(0, 0, 20.0, BlockMetricType::Psnr);

        // Act
        let normalized = value.normalized();

        // Assert - At minimum, should be 0
        assert_eq!(normalized, 0.0);
    }

    #[test]
    fn test_normalized_equal_max() {
        // Arrange
        let value = BlockMetricValue::new(0, 0, 50.0, BlockMetricType::Psnr);

        // Act
        let normalized = value.normalized();

        // Assert - At maximum, should be 1
        assert_eq!(normalized, 1.0);
    }

    #[test]
    fn test_normalized_negative_value() {
        // Arrange
        let value = BlockMetricValue::new(0, 0, -10.0, BlockMetricType::Psnr);

        // Act
        let normalized = value.normalized();

        // Assert - Below range, should clamp to 0
        assert_eq!(normalized, 0.0);
    }

    #[test]
    fn test_block_metric_value_zero_position() {
        // Arrange & Act
        let value = BlockMetricValue::new(0, 0, 30.0, BlockMetricType::Psnr);

        // Assert
        assert_eq!(value.block_x, 0);
        assert_eq!(value.block_y, 0);
    }

    #[test]
    fn test_block_metrics_grid_zero_frame_size() {
        // Arrange - Edge case: 0x0 frame
        let grid = BlockMetricsGrid::new(0, 0, 0, 16, BlockMetricType::Psnr);

        // Assert - Should have 0 blocks
        assert_eq!(grid.total_blocks(), 0);
    }

    #[test]
    fn test_block_metrics_grid_very_small_frame() {
        // Arrange
        let grid = BlockMetricsGrid::new(0, 16, 16, 16, BlockMetricType::Psnr);

        // Assert - Should have exactly 1 block
        assert_eq!(grid.width_blocks, 1);
        assert_eq!(grid.height_blocks, 1);
        assert_eq!(grid.total_blocks(), 1);
    }

    #[test]
    fn test_block_metrics_grid_large_block_size() {
        // Arrange - Block size larger than frame
        let grid = BlockMetricsGrid::new(0, 100, 100, 200, BlockMetricType::Psnr);

        // Assert - Should still have at least 1x1 blocks
        assert_eq!(grid.width_blocks, 1);
        assert_eq!(grid.height_blocks, 1);
    }

    #[test]
    fn test_block_metric_type_all_higher_is_better() {
        // Arrange
        let psnr = BlockMetricType::Psnr;
        let ssim = BlockMetricType::Ssim;
        let mse = BlockMetricType::Mse;
        let mad = BlockMetricType::Mad;

        // Act & Assert
        assert!(psnr.higher_is_better());
        assert!(ssim.higher_is_better());
        assert!(!mse.higher_is_better());
        assert!(!mad.higher_is_better());
    }

    #[test]
    fn test_block_metrics_grid_mse_metric() {
        // Arrange & Act
        let mut grid = BlockMetricsGrid::new(0, 640, 480, 32, BlockMetricType::Mse);
        grid.set(0, 0, 500.0); // Midpoint of MSE range

        // Act
        let normalized = grid.get_normalized(0, 0);

        // Assert - 500 is midpoint of 0-1000
        assert_eq!(normalized, Some(0.5));
    }

    #[test]
    fn test_block_metrics_grid_iter_values() {
        // Arrange
        let mut grid = BlockMetricsGrid::new(0, 320, 240, 16, BlockMetricType::Psnr);
        grid.set(0, 0, 30.0);
        grid.set(1, 0, 35.0);
        grid.set(0, 1, 40.0);

        // Act - Count non-zero values
        let non_zero_count = grid.values.iter().filter(|&&v| v > 0.0).count();

        // Assert
        assert_eq!(non_zero_count, 3);
    }
}
