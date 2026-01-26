// Motion Vector overlay module tests
//
// Per generate-tests skill: Arrange-Act-Assert pattern with fixtures
// and edge case coverage.

use super::*;

/// Create test motion vector
fn create_test_mv(dx: i32, dy: i32) -> MotionVector {
    MotionVector::new(dx, dy)
}

/// Create test MV grid with sample data
fn create_test_mv_grid() -> MVGrid {
    let grid_w = 30;
    let grid_h = 17;
    let total = grid_w * grid_h;

    let mut mv_l0 = Vec::with_capacity(total as usize);
    let mut mv_l1 = Vec::with_capacity(total as usize);
    let mut mode = Vec::with_capacity(total as usize);

    for row in 0..grid_h {
        for col in 0..grid_w {
            // Create varying MVs
            let dx = ((col as i32 - 15) * 4) as i32; // -60 to +56 qpel
            let dy = ((row as i32 - 8) * 4) as i32;  // -32 to +28 qpel
            mv_l0.push(create_test_mv(dx, dy));
            mv_l1.push(create_test_mv(-dx, -dy)); // Opposite direction for L1

            // Mix of modes
            mode.push(match (col + row) % 4 {
                0 => BlockMode::Inter,
                1 => BlockMode::Intra,
                2 => BlockMode::Skip,
                _ => BlockMode::None,
            });
        }
    }

    MVGrid::new(1920, 1080, 64, 64, mv_l0, mv_l1, Some(mode))
}

#[cfg(test)]
mod tests {
    use super::*;

    // ============================================================================
    // MotionVector Tests
    // ============================================================================

    #[test]
    fn test_motion_vector_new() {
        // Arrange & Act
        let mv = MotionVector::new(8, 16);

        // Assert
        assert_eq!(mv.dx_qpel, 8);
        assert_eq!(mv.dy_qpel, 16);
    }

    #[test]
    fn test_motion_vector_is_missing() {
        // Arrange
        let mv_normal = MotionVector::new(10, 20);
        let mv_missing_dx = MotionVector::new(MISSING_MV, 10);
        let mv_missing_dy = MotionVector::new(10, MISSING_MV);
        let mv_missing_both = MotionVector::new(MISSING_MV, MISSING_MV);

        // Act & Assert
        assert!(!mv_normal.is_missing());
        assert!(mv_missing_dx.is_missing());
        assert!(mv_missing_dy.is_missing());
        assert!(mv_missing_both.is_missing());
    }

    #[test]
    fn test_motion_vector_to_pixels() {
        // Arrange
        let mv = MotionVector::new(8, 16); // 2 and 4 pixels

        // Act
        let (dx_px, dy_px) = mv.to_pixels();

        // Assert
        assert_eq!(dx_px, 2.0);
        assert_eq!(dy_px, 4.0);
    }

    #[test]
    fn test_motion_vector_magnitude_px() {
        // Arrange
        let mv = MotionVector::new(8, 6); // (2, 1.5) pixels

        // Act
        let mag = mv.magnitude_px();

        // Assert - sqrt(2^2 + 1.5^2) = sqrt(4 + 2.25) = sqrt(6.25) = 2.5
        assert!((mag - 2.5).abs() < 0.001);
    }

    #[test]
    fn test_motion_vector_zero() {
        // Arrange & Act
        let mv = MotionVector::ZERO;

        // Assert
        assert_eq!(mv.dx_qpel, 0);
        assert_eq!(mv.dy_qpel, 0);
    }

    #[test]
    fn test_motion_vector_missing() {
        // Arrange & Act
        let mv = MotionVector::MISSING;

        // Assert
        assert!(mv.is_missing());
        assert_eq!(mv.dx_qpel, MISSING_MV);
        assert_eq!(mv.dy_qpel, MISSING_MV);
    }

    #[test]
    fn test_motion_vector_default() {
        // Arrange & Act
        let mv = MotionVector::default();

        // Assert
        assert_eq!(mv, MotionVector::ZERO);
    }

    // ============================================================================
    // BlockMode Tests
    // ============================================================================

    #[test]
    fn test_block_mode_from_u8() {
        // Arrange & Act & Assert
        assert_eq!(BlockMode::from(0), BlockMode::None);
        assert_eq!(BlockMode::from(1), BlockMode::Inter);
        assert_eq!(BlockMode::from(2), BlockMode::Intra);
        assert_eq!(BlockMode::from(3), BlockMode::Skip);
        assert_eq!(BlockMode::from(255), BlockMode::None); // Invalid defaults to None
    }

    // ============================================================================
    // MVGrid Tests
    // ============================================================================

    #[test]
    fn test_mv_grid_new() {
        // Arrange
        let grid_w = 30;
        let grid_h = 17;
        let total = (grid_w * grid_h) as usize;
        let mv_l0 = vec![MotionVector::ZERO; total];
        let mv_l1 = vec![MotionVector::ZERO; total];

        // Act
        let grid = MVGrid::new(1920, 1080, 64, 64, mv_l0, mv_l1, None);

        // Assert
        assert_eq!(grid.coded_width, 1920);
        assert_eq!(grid.coded_height, 1080);
        assert_eq!(grid.block_w, 64);
        assert_eq!(grid.block_h, 64);
        assert_eq!(grid.grid_w, grid_w);
        assert_eq!(grid.grid_h, grid_h);
        assert_eq!(grid.block_count(), total as usize);
    }

    #[test]
    #[should_panic(expected = "mv_l0 length mismatch")]
    fn test_mv_grid_new_panics_on_wrong_l0_length() {
        // Arrange
        let mv_l0 = vec![MotionVector::ZERO; 100];
        let mv_l1 = vec![MotionVector::ZERO; 510];
        let mode = vec![BlockMode::None; 510];

        // Act & Should panic
        MVGrid::new(1920, 1080, 64, 64, mv_l0, mv_l1, Some(mode));
    }

    #[test]
    fn test_mv_grid_get_l0() {
        // Arrange
        let grid = create_test_mv_grid();

        // Act
        let mv = grid.get_l0(15, 8); // Middle of grid

        // Assert
        assert!(mv.is_some());
        // At col=15, row=8: dx = (15-15)*4 = 0
        assert_eq!(mv.unwrap().dx_qpel, 0);
    }

    #[test]
    fn test_mv_grid_get_l0_out_of_bounds() {
        // Arrange
        let grid = create_test_mv_grid();

        // Act
        let mv = grid.get_l0(100, 0);

        // Assert
        assert!(mv.is_none());
    }

    #[test]
    fn test_mv_grid_get_l1() {
        // Arrange
        let grid = create_test_mv_grid();

        // Act
        let mv = grid.get_l1(15, 8);

        // Assert
        assert!(mv.is_some());
        assert_eq!(mv.unwrap().dx_qpel, 0); // Opposite of L0
    }

    #[test]
    fn test_mv_grid_get_mode() {
        // Arrange
        let grid = create_test_mv_grid();

        // Act
        let mode = grid.get_mode(0, 0); // (0+0) % 4 = 0 = Inter
        let mode_1 = grid.get_mode(1, 0); // (1+0) % 4 = 1 = Intra
        let mode_2 = grid.get_mode(2, 0); // (2+0) % 4 = 2 = Skip

        // Assert
        assert_eq!(mode, Some(BlockMode::Inter));
        assert_eq!(mode_1, Some(BlockMode::Intra));
        assert_eq!(mode_2, Some(BlockMode::Skip));
    }

    #[test]
    fn test_mv_grid_get_mode_none_when_missing() {
        // Arrange
        let total = 510;
        let mv_l0 = vec![MotionVector::ZERO; total];
        let mv_l1 = vec![MotionVector::ZERO; total];

        // Act - No mode vector provided
        let grid = MVGrid::new(1920, 1080, 64, 64, mv_l0, mv_l1, None);
        let mode = grid.get_mode(0, 0);

        // Assert
        assert!(mode.is_none());
    }

    #[test]
    fn test_mv_grid_block_center() {
        // Arrange
        let grid = create_test_mv_grid();

        // Act
        let (cx, cy) = grid.block_center(0, 0);
        let (cx_1, cy_1) = grid.block_center(1, 1);

        // Assert
        assert_eq!(cx, 32.0); // 0*64 + 64/2 = 32
        assert_eq!(cy, 32.0);
        assert_eq!(cx_1, 96.0); // 1*64 + 64/2 = 96
        assert_eq!(cy_1, 96.0);
    }

    #[test]
    fn test_mv_grid_statistics() {
        // Arrange
        let grid = create_test_mv_grid();

        // Act
        let stats = grid.statistics();

        // Assert
        assert_eq!(stats.total_blocks, 510);
        assert!(stats.l0_present > 0);
        assert!(stats.l1_present > 0);
        assert!(stats.inter_count > 0);
        assert!(stats.intra_count > 0);
        assert!(stats.skip_count > 0);
    }

    // ============================================================================
    // MVLayer Tests
    // ============================================================================

    #[test]
    fn test_mv_layer_cache_suffix() {
        // Arrange & Act & Assert
        assert_eq!(MVLayer::L0Only.cache_suffix(), "L0");
        assert_eq!(MVLayer::L1Only.cache_suffix(), "L1");
        assert_eq!(MVLayer::Both.cache_suffix(), "both");
    }

    // ============================================================================
    // Viewport Tests
    // ============================================================================

    #[test]
    fn test_viewport_new() {
        // Arrange & Act
        let vp = Viewport::new(100, 200, 640, 480);

        // Assert
        assert_eq!(vp.x, 100);
        assert_eq!(vp.y, 200);
        assert_eq!(vp.width, 640);
        assert_eq!(vp.height, 480);
    }

    #[test]
    fn test_viewport_contains_block_inside() {
        // Arrange
        let vp = Viewport::new(0, 0, 640, 480);

        // Act & Assert
        assert!(vp.contains_block(100, 100, 64, 64)); // Inside
        assert!(vp.contains_block(0, 0, 64, 64));     // Top-left corner
        assert!(vp.contains_block(576, 416, 64, 64)); // Bottom-right corner
    }

    #[test]
    fn test_viewport_contains_block_overlap() {
        // Arrange
        let vp = Viewport::new(100, 100, 200, 200);

        // Act & Assert - Blocks that overlap but aren't fully inside
        assert!(vp.contains_block(50, 150, 64, 64));  // Overlaps left edge
        assert!(vp.contains_block(250, 150, 64, 64)); // Overlaps right edge
        assert!(vp.contains_block(150, 50, 64, 64));  // Overlaps top edge
        assert!(vp.contains_block(150, 250, 64, 64)); // Overlaps bottom edge
    }

    #[test]
    fn test_viewport_contains_block_outside() {
        // Arrange
        let vp = Viewport::new(100, 100, 200, 200);

        // Act & Assert
        assert!(!vp.contains_block(0, 0, 64, 64));        // Top-left
        assert!(!vp.contains_block(400, 0, 64, 64));      // Top-right
        assert!(!vp.contains_block(0, 400, 64, 64));      // Bottom-left
        assert!(!vp.contains_block(400, 400, 64, 64));    // Bottom-right
        assert!(!vp.contains_block(50, 50, 40, 40));      // Left edge (no overlap)
    }

    // ============================================================================
    // DensityControl Tests
    // ============================================================================

    #[test]
    fn test_density_control_calculate_stride_under_limit() {
        // Arrange
        let visible_blocks = 5000; // Under 8000

        // Act
        let stride = DensityControl::calculate_stride(visible_blocks);

        // Assert
        assert_eq!(stride, 1); // No downsampling needed
    }

    #[test]
    fn test_density_control_calculate_stride_at_limit() {
        // Arrange
        let visible_blocks = 8000; // Exactly at limit

        // Act
        let stride = DensityControl::calculate_stride(visible_blocks);

        // Assert
        assert_eq!(stride, 1);
    }

    #[test]
    fn test_density_control_calculate_stride_over_limit() {
        // Arrange
        let visible_blocks = 32000; // 4x over limit

        // Act
        let stride = DensityControl::calculate_stride(visible_blocks);

        // Assert - sqrt(32000/8000) = sqrt(4) = 2
        assert_eq!(stride, 2);
    }

    #[test]
    fn test_density_control_calculate_stride_large_over_limit() {
        // Arrange
        let visible_blocks = 180000; // 22.5x over limit

        // Act
        let stride = DensityControl::calculate_stride(visible_blocks);

        // Assert - sqrt(180000/8000) = sqrt(22.5) ≈ 4.74, ceil = 5
        assert_eq!(stride, 5);
    }

    #[test]
    fn test_density_control_should_draw() {
        // Arrange & Act & Assert
        assert!(DensityControl::should_draw(0, 0, 1)); // Always draw with stride 1
        assert!(DensityControl::should_draw(2, 2, 2)); // Multiple of stride
        assert!(!DensityControl::should_draw(1, 0, 2)); // Not multiple of stride
        assert!(!DensityControl::should_draw(0, 1, 2)); // Not multiple of stride
    }

    #[test]
    fn test_density_control_estimate_drawn_count() {
        // Arrange
        let visible_blocks = 10000;

        // Act
        let stride_1 = DensityControl::estimate_drawn_count(visible_blocks, 1);
        let stride_2 = DensityControl::estimate_drawn_count(visible_blocks, 2);

        // Assert
        assert_eq!(stride_1, 10000);
        assert_eq!(stride_2, 2500); // 10000 / 4
    }

    // ============================================================================
    // MVScaling Tests
    // ============================================================================

    #[test]
    fn test_mv_scaling_scale_vector() {
        // Arrange
        let mv = MotionVector::new(8, 16); // (2, 4) pixels
        let user_scale = 2.0;
        let zoom_scale = 1.5;

        // Act
        let (dx, dy) = MVScaling::scale_vector(&mv, user_scale, zoom_scale);

        // Assert - 2*2*1.5 = 6, 4*2*1.5 = 12
        assert_eq!(dx, 6.0);
        assert_eq!(dy, 12.0);
    }

    #[test]
    fn test_mv_scaling_clamp_arrow_length_no_clamp() {
        // Arrange
        let (dx, dy) = (10.0, 10.0); // magnitude ≈ 14.14
        let max_length = 48.0;

        // Act
        let (clamped_dx, clamped_dy) = MVScaling::clamp_arrow_length(dx, dy, max_length);

        // Assert - No clamping needed
        assert_eq!(clamped_dx, 10.0);
        assert_eq!(clamped_dy, 10.0);
    }

    #[test]
    fn test_mv_scaling_clamp_arrow_length_clamp() {
        // Arrange
        let (dx, dy) = (40.0, 40.0); // magnitude ≈ 56.57 > 48
        let max_length = 48.0;

        // Act
        let (clamped_dx, clamped_dy) = MVScaling::clamp_arrow_length(dx, dy, max_length);

        // Assert - Should be scaled down
        assert!((clamped_dx - 33.94).abs() < 0.1); // 40 * 48 / 56.57 ≈ 33.94
        assert!((clamped_dy - 33.94).abs() < 0.1);
    }

    #[test]
    fn test_mv_scaling_clamp_arrow_length_zero() {
        // Arrange
        let (dx, dy) = (0.0, 0.0);
        let max_length = 48.0;

        // Act
        let (clamped_dx, clamped_dy) = MVScaling::clamp_arrow_length(dx, dy, max_length);

        // Assert
        assert_eq!(clamped_dx, 0.0);
        assert_eq!(clamped_dy, 0.0);
    }

    // ============================================================================
    // MVCacheKey Tests
    // ============================================================================

    #[test]
    fn test_mv_cache_key_new() {
        // Arrange
        let viewport = Viewport::new(100, 200, 640, 480);

        // Act
        let key = MVCacheKey::new(
            "stream1".to_string(),
            10,
            viewport,
            2,
            MVLayer::Both,
            1.5,
            0.7,
        );

        // Assert
        assert_eq!(key.stream_id, "stream1");
        assert_eq!(key.frame_idx, 10);
        assert_eq!(key.viewport, (100, 200, 640, 480));
        assert_eq!(key.stride, 2);
        assert_eq!(key.layer, "both");
        assert_eq!(key.scale_bucket, 15); // 1.5 * 10 = 15
        assert_eq!(key.opacity_bucket, 14); // 0.7 * 20 = 14
    }

    #[test]
    fn test_mv_cache_key_display_format() {
        // Arrange
        let key = MVCacheKey {
            stream_id: "test".to_string(),
            frame_idx: 5,
            viewport: (10, 20, 640, 480),
            stride: 1,
            layer: "L0".to_string(),
            scale_bucket: 10,
            opacity_bucket: 11,
        };

        // Act
        let display = format!("{}", key);

        // Assert
        assert!(display.contains("overlay_mv:test:f5"));
        assert!(display.contains("vp10,20,640,480"));
        assert!(display.contains("s1"));
        assert!(display.contains("LL0"));
        assert!(display.contains("sc10"));
        assert!(display.contains("op11"));
    }

    // ============================================================================
    // MVOverlay Tests
    // ============================================================================

    #[test]
    fn test_mv_overlay_new() {
        // Arrange
        let grid = create_test_mv_grid();

        // Act
        let overlay = MVOverlay::new(grid);

        // Assert
        assert_eq!(overlay.layer, MVLayer::Both);
        assert_eq!(overlay.user_scale, DEFAULT_USER_SCALE);
        assert_eq!(overlay.opacity, DEFAULT_OPACITY);
        assert!(overlay.cached_visible.is_none());
        assert!(overlay.cache_key.is_none());
    }

    #[test]
    fn test_mv_overlay_set_layer() {
        // Arrange
        let mut overlay = MVOverlay::new(create_test_mv_grid());

        // Act
        overlay.set_layer(MVLayer::L0Only);

        // Assert
        assert_eq!(overlay.layer, MVLayer::L0Only);
        assert!(overlay.cache_key.is_none()); // Cache invalidated
    }

    #[test]
    fn test_mv_overlay_set_layer_same_no_invalidate() {
        // Arrange
        let mut overlay = MVOverlay::new(create_test_mv_grid());
        overlay.update_cache("test".to_string(), 0, Viewport::new(0, 0, 640, 480));
        assert!(overlay.cache_key.is_some()); // Cache exists

        // Act - Set to same value
        overlay.set_layer(MVLayer::Both);

        // Assert - Cache should still be valid
        assert!(overlay.cache_key.is_some());
    }

    #[test]
    fn test_mv_overlay_set_user_scale() {
        // Arrange
        let mut overlay = MVOverlay::new(create_test_mv_grid());

        // Act
        overlay.set_user_scale(2.0);

        // Assert
        assert_eq!(overlay.user_scale, 2.0);
    }

    #[test]
    fn test_mv_overlay_set_user_scale_same_bucket_no_invalidate() {
        // Arrange
        let mut overlay = MVOverlay::new(create_test_mv_grid());
        overlay.update_cache("test".to_string(), 0, Viewport::new(0, 0, 640, 480));

        // Act - Set scale to same bucket (1.04 rounds to 10, same as 1.0)
        overlay.set_user_scale(1.04);

        // Assert - Cache should still be valid (same bucket)
        assert!(overlay.cache_key.is_some());
    }

    #[test]
    fn test_mv_overlay_set_opacity_clamp() {
        // Arrange
        let mut overlay = MVOverlay::new(create_test_mv_grid());

        // Act
        overlay.set_opacity(1.5); // Over 1.0
        assert_eq!(overlay.opacity, 1.0);

        overlay.set_opacity(-0.5); // Under 0.0
        assert_eq!(overlay.opacity, 0.0);
    }

    #[test]
    fn test_mv_overlay_compute_visible_blocks() {
        // Arrange
        let overlay = MVOverlay::new(create_test_mv_grid());
        let viewport = Viewport::new(0, 0, 640, 480);

        // Act
        let (visible, stride) = overlay.compute_visible_blocks(viewport);

        // Assert - Should find blocks in 640x480 area
        assert!(!visible.is_empty());
        assert!(stride >= 1);
    }

    #[test]
    fn test_mv_overlay_update_cache() {
        // Arrange
        let mut overlay = MVOverlay::new(create_test_mv_grid());
        let viewport = Viewport::new(0, 0, 640, 480);

        // Act
        overlay.update_cache("stream1".to_string(), 5, viewport);

        // Assert
        assert!(overlay.cached_visible.is_some());
        assert!(overlay.cache_key.is_some());
        assert_eq!(overlay.cache_key.as_ref().unwrap().frame_idx, 5);
    }

    #[test]
    fn test_mv_overlay_get_visible_blocks_cached() {
        // Arrange
        let mut overlay = MVOverlay::new(create_test_mv_grid());
        let viewport = Viewport::new(0, 0, 640, 480);
        overlay.update_cache("stream1".to_string(), 5, viewport);
        let cached_count = overlay.cached_visible.as_ref().unwrap().len();

        // Act
        let visible = overlay.get_visible_blocks(viewport);

        // Assert - Should return cached copy
        assert_eq!(visible.len(), cached_count);
    }

    #[test]
    fn test_mv_overlay_get_visible_blocks_uncached() {
        // Arrange
        let overlay = MVOverlay::new(create_test_mv_grid());
        let viewport = Viewport::new(0, 0, 640, 480);

        // Act - No cache set
        let visible = overlay.get_visible_blocks(viewport);

        // Assert - Should compute
        assert!(!visible.is_empty());
    }

    #[test]
    fn test_mv_overlay_statistics() {
        // Arrange
        let overlay = MVOverlay::new(create_test_mv_grid());

        // Act
        let stats = overlay.statistics();

        // Assert
        assert_eq!(stats.total_blocks, 510);
    }

    // ============================================================================
    // MVStatistics Tests
    // ============================================================================

    #[test]
    fn test_mv_statistics_default() {
        // Arrange & Act
        let stats = MVStatistics::default();

        // Assert
        assert_eq!(stats.total_blocks, 0);
        assert_eq!(stats.l0_present, 0);
        assert_eq!(stats.l1_present, 0);
        assert_eq!(stats.l0_avg_magnitude, 0.0);
        assert_eq!(stats.l1_avg_magnitude, 0.0);
        assert_eq!(stats.l0_max_magnitude, 0.0);
        assert_eq!(stats.l1_max_magnitude, 0.0);
    }

    #[test]
    fn test_mv_statistics_summary() {
        // Arrange
        let mut stats = MVStatistics::default();
        stats.l0_present = 100;
        stats.l0_avg_magnitude = 5.5;
        stats.l1_present = 50;
        stats.l1_avg_magnitude = 3.2;

        // Act
        let summary = stats.summary();

        // Assert
        assert!(summary.contains("100 vectors"));
        assert!(summary.contains("avg 5.5"));
        assert!(summary.contains("50 vectors"));
        assert!(summary.contains("avg 3.2"));
    }

    #[test]
    fn test_mv_statistics_total_vectors() {
        // Arrange
        let mut stats = MVStatistics::default();
        stats.l0_present = 100;
        stats.l1_present = 50;

        // Act
        let total = stats.total_vectors();

        // Assert
        assert_eq!(total, 150);
    }

    // ============================================================================
    // Edge Cases
    // ============================================================================

    #[test]
    fn test_empty_mv_grid() {
        // Arrange
        let mv_l0 = vec![];
        let mv_l1 = vec![];

        // Act
        let grid = MVGrid::new(0, 0, 64, 64, mv_l0, mv_l1, None);

        // Assert
        assert_eq!(grid.coded_width, 0);
        assert_eq!(grid.coded_height, 0);
        assert_eq!(grid.block_count(), 0);
    }

    #[test]
    fn test_viewport_zero_size() {
        // Arrange
        let vp = Viewport::new(0, 0, 0, 0);

        // Act & Assert - Should not crash
        assert!(!vp.contains_block(0, 0, 64, 64));
    }

    #[test]
    fn test_density_control_zero_blocks() {
        // Arrange
        let visible_blocks = 0;

        // Act
        let stride = DensityControl::calculate_stride(visible_blocks);

        // Assert
        assert_eq!(stride, 1);
    }

    #[test]
    fn test_motion_vector_negative_components() {
        // Arrange & Act
        let mv = MotionVector::new(-8, -16);

        // Assert
        assert_eq!(mv.dx_qpel, -8);
        assert_eq!(mv.dy_qpel, -16);
        assert!(!mv.is_missing());
    }

    #[test]
    fn test_motion_vector_large_values() {
        // Arrange & Act
        let mv = MotionVector::new(1000000, 2000000);

        // Assert
        let (dx_px, dy_px) = mv.to_pixels();
        assert_eq!(dx_px, 250000.0);
        assert_eq!(dy_px, 500000.0);
    }
}
