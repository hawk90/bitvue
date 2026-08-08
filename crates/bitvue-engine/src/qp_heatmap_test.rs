// QP Heatmap overlay module tests
//
// Per generate-tests skill: Arrange-Act-Assert pattern with fixtures
// and edge case coverage.

use super::*;
use crate::StreamId;

/// Create test QP grid with sample data
fn create_test_qp_grid() -> QPGrid {
    let grid_w = 30;
    let grid_h = 17;
    let total = (grid_w * grid_h) as usize;

    let mut qp = Vec::with_capacity(total);
    for row in 0..grid_h {
        for col in 0..grid_w {
            // Create gradient of QP values (0-63)
            let qp_val = ((row * grid_w + col) * 63 / (grid_w * grid_h)) as i16;
            qp.push(qp_val);
        }
    }

    QPGrid::new(grid_w, grid_h, 64, 64, qp, -1)
}

/// Create QP grid with missing values
fn create_test_qp_grid_with_missing() -> QPGrid {
    let total = 510;
    let mut qp = vec![20i16; total];

    // Add some missing values
    qp[50] = -1;
    qp[100] = -1;
    qp[150] = -1;

    QPGrid::new(30, 17, 64, 64, qp, -1)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ============================================================================
    // QPGrid Tests
    // ============================================================================

    #[test]
    fn test_qp_grid_new() {
        // Arrange
        let qp = vec![20i16; 510];

        // Act
        let grid = QPGrid::new(30, 17, 64, 64, qp, -1);

        // Assert
        assert_eq!(grid.grid_w, 30);
        assert_eq!(grid.grid_h, 17);
        assert_eq!(grid.block_w, 64);
        assert_eq!(grid.block_h, 64);
        assert_eq!(grid.qp.len(), 510);
        assert_eq!(grid.qp_min, 20);
        assert_eq!(grid.qp_max, 20);
    }

    #[test]
    fn test_qp_grid_new_calculates_min_max() {
        // Arrange
        let mut qp = vec![20i16; 510];
        qp[0] = 10;  // Min
        qp[509] = 50; // Max

        // Act
        let grid = QPGrid::new(30, 17, 64, 64, qp, -1);

        // Assert
        assert_eq!(grid.qp_min, 10);
        assert_eq!(grid.qp_max, 50);
    }

    #[test]
    fn test_qp_grid_get_valid() {
        // Arrange
        let grid = create_test_qp_grid();

        // Act
        let qp = grid.get(0, 0);

        // Assert
        assert_eq!(qp, Some(0));
    }

    #[test]
    fn test_qp_grid_get_out_of_bounds() {
        // Arrange
        let grid = create_test_qp_grid();

        // Act
        let qp = grid.get(100, 0);

        // Assert
        assert!(qp.is_none());
    }

    #[test]
    fn test_qp_grid_get_missing_value() {
        // Arrange
        let grid = create_test_qp_grid_with_missing();

        // Act
        let qp_0_0 = grid.get(0, 0); // Position 0 - valid
        let qp_20_1 = grid.get(20, 1); // Position 50 (20 + 1*30) - missing
        let qp_10_3 = grid.get(10, 3); // Position 100 (10 + 3*30) - missing

        // Assert
        assert_eq!(qp_0_0, Some(20)); // Valid value
        assert_eq!(qp_20_1, None); // Missing marker at position 50
        assert_eq!(qp_10_3, None); // Missing marker at position 100
    }

    #[test]
    fn test_qp_grid_coded_dimensions() {
        // Arrange
        let grid = create_test_qp_grid();

        // Act
        let (width, height) = grid.coded_dimensions();

        // Assert
        assert_eq!(width, 1920); // 30 * 64
        assert_eq!(height, 1088); // 17 * 64
    }

    #[test]
    fn test_qp_grid_coverage_percent_all_valid() {
        // Arrange
        let grid = create_test_qp_grid();

        // Act
        let coverage = grid.coverage_percent();

        // Assert
        assert_eq!(coverage, 100.0);
    }

    #[test]
    fn test_qp_grid_coverage_percent_with_missing() {
        // Arrange
        let grid = create_test_qp_grid_with_missing();

        // Act
        let coverage = grid.coverage_percent();

        // Assert - 3 missing out of 510
        assert!((coverage - (507.0 / 510.0 * 100.0)).abs() < 0.01);
    }

    #[test]
    fn test_qp_grid_coverage_percent_empty() {
        // Arrange
        let grid = QPGrid::new(0, 0, 64, 64, vec![], -1);

        // Act
        let coverage = grid.coverage_percent();

        // Assert
        assert_eq!(coverage, 0.0);
    }

    // ============================================================================
    // QPScaleMode Tests
    // ============================================================================

    #[test]
    fn test_qp_scale_mode_variants() {
        // Arrange & Act & Assert
        assert_eq!(QPScaleMode::Auto, QPScaleMode::Auto);
        assert_eq!(QPScaleMode::Fixed, QPScaleMode::Fixed);
        assert_ne!(QPScaleMode::Auto, QPScaleMode::Fixed);
    }

    // ============================================================================
    // HeatmapResolution Tests
    // ============================================================================

    #[test]
    fn test_heatmap_resolution_scale() {
        // Arrange & Act & Assert
        assert_eq!(HeatmapResolution::Quarter.scale(), 4);
        assert_eq!(HeatmapResolution::Half.scale(), 2);
        assert_eq!(HeatmapResolution::Full.scale(), 1);
    }

    // ============================================================================
    // Color Tests
    // ============================================================================

    #[test]
    fn test_color_new() {
        // Arrange & Act
        let color = Color::new(100, 150, 200, 255);

        // Assert
        assert_eq!(color.r, 100);
        assert_eq!(color.g, 150);
        assert_eq!(color.b, 200);
        assert_eq!(color.a, 255);
    }

    #[test]
    fn test_color_rgb() {
        // Arrange & Act
        let color = Color::rgb(100, 150, 200);

        // Assert
        assert_eq!(color.r, 100);
        assert_eq!(color.g, 150);
        assert_eq!(color.b, 200);
        assert_eq!(color.a, 255); // Alpha defaults to 255
    }

    #[test]
    fn test_color_lerp_start() {
        // Arrange
        let a = Color::rgb(0, 0, 0);
        let b = Color::rgb(100, 100, 100);

        // Act
        let result = Color::lerp(a, b, 0.0);

        // Assert
        assert_eq!(result.r, 0);
        assert_eq!(result.g, 0);
        assert_eq!(result.b, 0);
    }

    #[test]
    fn test_color_lerp_end() {
        // Arrange
        let a = Color::rgb(0, 0, 0);
        let b = Color::rgb(100, 100, 100);

        // Act
        let result = Color::lerp(a, b, 1.0);

        // Assert
        assert_eq!(result.r, 100);
        assert_eq!(result.g, 100);
        assert_eq!(result.b, 100);
    }

    #[test]
    fn test_color_lerp_middle() {
        // Arrange
        let a = Color::rgb(0, 0, 0);
        let b = Color::rgb(100, 200, 50);

        // Act
        let result = Color::lerp(a, b, 0.5);

        // Assert
        assert_eq!(result.r, 50);
        assert_eq!(result.g, 100);
        assert_eq!(result.b, 25);
    }

    #[test]
    fn test_color_lerp_clamp_low() {
        // Arrange
        let a = Color::rgb(0, 0, 0);
        let b = Color::rgb(100, 100, 100);

        // Act
        let result = Color::lerp(a, b, -0.5);

        // Assert - Should clamp to 0.0
        assert_eq!(result.r, 0);
    }

    #[test]
    fn test_color_lerp_clamp_high() {
        // Arrange
        let a = Color::rgb(0, 0, 0);
        let b = Color::rgb(100, 100, 100);

        // Act
        let result = Color::lerp(a, b, 1.5);

        // Assert - Should clamp to 1.0
        assert_eq!(result.r, 100);
    }

    // ============================================================================
    // QPColorMapper Tests
    // ============================================================================

    #[test]
    fn test_qp_color_mapper_new() {
        // Arrange & Act
        let mapper = QPColorMapper::new(0.7);

        // Assert
        assert_eq!(mapper.base_alpha, 160);
        assert_eq!(mapper.user_opacity, 0.7);
    }

    #[test]
    fn test_qp_color_mapper_new_clamp_opacity() {
        // Arrange & Act
        let mapper_high = QPColorMapper::new(1.5);
        let mapper_low = QPColorMapper::new(-0.5);

        // Assert
        assert_eq!(mapper_high.user_opacity, 1.0);
        assert_eq!(mapper_low.user_opacity, 0.0);
    }

    #[test]
    fn test_qp_color_mapper_map_qp_none() {
        // Arrange
        let mapper = QPColorMapper::new(0.5);

        // Act
        let color = mapper.map_qp(None, 0, 63);

        // Assert - Missing value → transparent
        assert_eq!(color.a, 0);
    }

    #[test]
    fn test_qp_color_mapper_map_qp_min() {
        // Arrange
        let mapper = QPColorMapper::new(1.0);

        // Act
        let color = mapper.map_qp(Some(0), 0, 63);

        // Assert - Should map to first color stop (blue: 0, 70, 255)
        assert_eq!(color.r, 0);
        assert_eq!(color.g, 70);
        assert_eq!(color.b, 255);
        assert_eq!(color.a, 160); // base_alpha * 1.0
    }

    #[test]
    fn test_qp_color_mapper_map_qp_max() {
        // Arrange
        let mapper = QPColorMapper::new(1.0);

        // Act
        let color = mapper.map_qp(Some(63), 0, 63);

        // Assert - Should map to last color stop (red: 255, 40, 40)
        assert_eq!(color.r, 255);
        assert_eq!(color.g, 40);
        assert_eq!(color.b, 40);
        assert_eq!(color.a, 160);
    }

    #[test]
    fn test_qp_color_mapper_map_qp_middle() {
        // Arrange
        let mapper = QPColorMapper::new(1.0);

        // Act
        let color = mapper.map_qp(Some(31), 0, 63); // Middle of range

        // Assert - Should be somewhere in middle (cyan to yellow)
        assert!(color.g > 100); // Should have significant green
        assert!(color.r > 50 && color.r < 255);
    }

    #[test]
    fn test_qp_color_mapper_map_qp_single_value() {
        // Arrange
        let mapper = QPColorMapper::new(1.0);

        // Act
        let color = mapper.map_qp(Some(30), 30, 30); // min == max

        // Assert - Should use 0.5 (middle)
        assert!(color.g > 100); // Should have green component
    }

    #[test]
    fn test_qp_color_mapper_opacity_bucket() {
        // Arrange & Act & Assert
        assert_eq!(QPColorMapper::opacity_bucket(0.0), 0);
        assert_eq!(QPColorMapper::opacity_bucket(0.05), 1); // 0.05 * 20 = 1
        assert_eq!(QPColorMapper::opacity_bucket(0.10), 2); // 0.10 * 20 = 2
        assert_eq!(QPColorMapper::opacity_bucket(0.50), 10);
        assert_eq!(QPColorMapper::opacity_bucket(1.0), 20);
    }

    // ============================================================================
    // HeatmapTexture Tests
    // ============================================================================

    #[test]
    fn test_heatmap_texture_generate() {
        // Arrange
        let grid = create_test_qp_grid();

        // Act
        let texture = HeatmapTexture::generate(
            &grid,
            HeatmapResolution::Half,
            QPScaleMode::Auto,
            0.5,
        );

        // Assert
        assert!(texture.width > 0);
        assert!(texture.height > 0);
        assert_eq!(texture.pixels.len(), (texture.width * texture.height * 4) as usize);
    }

    #[test]
    fn test_heatmap_texture_generate_quarter_resolution() {
        // Arrange
        let grid = create_test_qp_grid();

        // Act
        let texture_half = HeatmapTexture::generate(
            &grid,
            HeatmapResolution::Half,
            QPScaleMode::Auto,
            0.5,
        );
        let texture_quarter = HeatmapTexture::generate(
            &grid,
            HeatmapResolution::Quarter,
            QPScaleMode::Auto,
            0.5,
        );

        // Assert - Quarter should be smaller than half
        assert!(texture_quarter.width <= texture_half.width);
        assert!(texture_quarter.height <= texture_half.height);
    }

    #[test]
    fn test_heatmap_texture_generate_full_resolution() {
        // Arrange
        let grid = create_test_qp_grid();

        // Act
        let texture = HeatmapTexture::generate(
            &grid,
            HeatmapResolution::Full,
            QPScaleMode::Auto,
            0.5,
        );

        // Assert - Full resolution is clamped to 1024 max width per implementation
        assert_eq!(texture.width, 1024); // Clamped to max
        assert!(texture.height > 0);
        assert!(texture.height <= 1088);
    }

    // ============================================================================
    // QPHeatmapCacheKey Tests
    // ============================================================================

    #[test]
    fn test_qp_cache_key_params_new() {
        // Arrange & Act
        let params = QPCacheKeyParams {
            stream: StreamId::A,
            frame_idx: 10,
            resolution: HeatmapResolution::Half,
            scale_mode: QPScaleMode::Auto,
            qp_min: 0,
            qp_max: 63,
            opacity: 0.7,
            codec: "av1",
            file_path: "/path/to/file.mp4",
        };

        // Act
        let key = QPHeatmapCacheKey::new(&params);

        // Assert
        assert_eq!(key.stream, StreamId::A);
        assert_eq!(key.frame_idx, 10);
        assert_eq!(key.resolution, HeatmapResolution::Half);
        assert_eq!(key.scale_mode, QPScaleMode::Auto);
        assert_eq!(key.qp_min, 0);
        assert_eq!(key.qp_max, 63);
        assert_eq!(key.opacity_bucket, 14); // 0.7 * 20 = 14
        assert_eq!(key.codec, "av1");
        assert!(key.file_hash != 0); // Should have hashed the file path
    }

    #[test]
    fn test_qp_cache_key_to_string() {
        // Arrange
        let key = QPHeatmapCacheKey {
            stream: StreamId::A,
            frame_idx: 5,
            resolution: HeatmapResolution::Half,
            scale_mode: QPScaleMode::Auto,
            qp_min: 0,
            qp_max: 63,
            opacity_bucket: 10,
            codec: "av1".to_string(),
            file_hash: 0x123456789ABCDEF0,
        };

        // Act
        let key_str = key.to_string(960, 544);

        // Assert
        assert!(key_str.contains("overlay_qp_heat:A:av1:"));
        assert!(key_str.contains("f5"));
        assert!(key_str.contains("hm960x544"));
        assert!(key_str.contains("scaleauto"));
        assert!(key_str.contains("op10"));
    }

    // ============================================================================
    // QPHeatmapOverlay Tests
    // ============================================================================

    #[test]
    fn test_qp_heatmap_overlay_new() {
        // Arrange
        let grid = create_test_qp_grid();

        // Act
        let overlay = QPHeatmapOverlay::new(grid);

        // Assert
        assert_eq!(overlay.resolution, HeatmapResolution::Half);
        assert_eq!(overlay.scale_mode, QPScaleMode::Auto);
        assert_eq!(overlay.opacity, 0.45);
        assert!(overlay.cached_texture.is_none());
        assert!(overlay.cache_key.is_none());
    }

    #[test]
    fn test_qp_heatmap_overlay_get_texture_generates() {
        // Arrange
        let mut overlay = QPHeatmapOverlay::new(create_test_qp_grid());
        let cache_key = QPHeatmapCacheKey {
            stream: StreamId::A,
            frame_idx: 0,
            resolution: HeatmapResolution::Half,
            scale_mode: QPScaleMode::Auto,
            qp_min: 0,
            qp_max: 63,
            opacity_bucket: 10,
            codec: "av1".to_string(),
            file_hash: 0,
        };

        // Act
        let texture = overlay.get_texture(cache_key);

        // Assert
        assert!(texture.width > 0);
        assert!(texture.height > 0);
        assert!(overlay.cached_texture.is_some());
        assert!(overlay.cache_key.is_some());
    }

    #[test]
    fn test_qp_heatmap_overlay_get_texture_reuses() {
        // Arrange
        let mut overlay = QPHeatmapOverlay::new(create_test_qp_grid());
        let cache_key = QPHeatmapCacheKey {
            stream: StreamId::A,
            frame_idx: 0,
            resolution: HeatmapResolution::Half,
            scale_mode: QPScaleMode::Auto,
            qp_min: 0,
            qp_max: 63,
            opacity_bucket: 10,
            codec: "av1".to_string(),
            file_hash: 0,
        };

        // Act - Get texture twice with same key
        let texture1_addr = overlay.get_texture(cache_key.clone()) as *const HeatmapTexture as usize;
        let texture2_addr = overlay.get_texture(cache_key) as *const HeatmapTexture as usize;

        // Assert - Should return same cached texture
        assert_eq!(texture1_addr, texture2_addr);
    }

    #[test]
    fn test_qp_heatmap_overlay_has_sufficient_coverage() {
        // Arrange
        let overlay = QPHeatmapOverlay::new(create_test_qp_grid());

        // Act & Assert
        assert!(overlay.has_sufficient_coverage());
    }

    #[test]
    fn test_qp_heatmap_overlay_insufficient_coverage() {
        // Arrange - Create grid with mostly missing values
        let mut qp = vec![-1i16; 510];
        qp[0] = 20; // Only 1 valid value (< 20%)
        let grid = QPGrid::new(30, 17, 64, 64, qp, -1);
        let overlay = QPHeatmapOverlay::new(grid);

        // Act & Assert
        assert!(!overlay.has_sufficient_coverage());
    }

    #[test]
    fn test_qp_heatmap_overlay_coverage_percent() {
        // Arrange
        let overlay = QPHeatmapOverlay::new(create_test_qp_grid());

        // Act
        let coverage = overlay.coverage_percent();

        // Assert
        assert_eq!(coverage, 100.0);
    }

    #[test]
    fn test_qp_heatmap_overlay_set_resolution() {
        // Arrange
        let mut overlay = QPHeatmapOverlay::new(create_test_qp_grid());

        // Act
        overlay.set_resolution(HeatmapResolution::Full);

        // Assert
        assert_eq!(overlay.resolution, HeatmapResolution::Full);
        assert!(overlay.cache_key.is_none()); // Cache invalidated
    }

    #[test]
    fn test_qp_heatmap_overlay_set_resolution_same_no_invalidate() {
        // Arrange
        let mut overlay = QPHeatmapOverlay::new(create_test_qp_grid());
        let cache_key = QPHeatmapCacheKey {
            stream: StreamId::A,
            frame_idx: 0,
            resolution: HeatmapResolution::Half,
            scale_mode: QPScaleMode::Auto,
            qp_min: 0,
            qp_max: 63,
            opacity_bucket: 10,
            codec: "av1".to_string(),
            file_hash: 0,
        };
        overlay.cache_key = Some(cache_key);

        // Act - Set to same resolution
        overlay.set_resolution(HeatmapResolution::Half);

        // Assert - Cache should still be valid
        assert!(overlay.cache_key.is_some());
    }

    #[test]
    fn test_qp_heatmap_overlay_set_scale_mode() {
        // Arrange
        let mut overlay = QPHeatmapOverlay::new(create_test_qp_grid());

        // Act
        overlay.set_scale_mode(QPScaleMode::Fixed);

        // Assert
        assert_eq!(overlay.scale_mode, QPScaleMode::Fixed);
        assert!(overlay.cache_key.is_none()); // Cache invalidated
    }

    #[test]
    fn test_qp_heatmap_overlay_set_opacity() {
        // Arrange
        let mut overlay = QPHeatmapOverlay::new(create_test_qp_grid());

        // Act
        overlay.set_opacity(0.8);

        // Assert
        assert_eq!(overlay.opacity, 0.8);
    }

    #[test]
    fn test_qp_heatmap_overlay_set_opacity_clamp() {
        // Arrange
        let mut overlay = QPHeatmapOverlay::new(create_test_qp_grid());

        // Act
        overlay.set_opacity(1.5); // Over 1.0
        assert_eq!(overlay.opacity, 1.0);

        overlay.set_opacity(-0.5); // Under 0.0
        assert_eq!(overlay.opacity, 0.0);
    }

    #[test]
    fn test_qp_heatmap_overlay_set_opacity_same_bucket_no_invalidate() {
        // Arrange
        let mut overlay = QPHeatmapOverlay::new(create_test_qp_grid());
        let cache_key = QPHeatmapCacheKey {
            stream: StreamId::A,
            frame_idx: 0,
            resolution: HeatmapResolution::Half,
            scale_mode: QPScaleMode::Auto,
            qp_min: 0,
            qp_max: 63,
            opacity_bucket: 9, // 0.45 * 20 = 9
            codec: "av1".to_string(),
            file_hash: 0,
        };
        overlay.cache_key = Some(cache_key);

        // Act - Set opacity to same bucket (0.474 rounds to 9.48 -> 9, same as 0.45)
        // 0.474 * 20 = 9.48, rounds to 9
        overlay.set_opacity(0.474);

        // Assert - Cache should still be valid (same bucket)
        assert!(overlay.cache_key.is_some());
    }

    // ============================================================================
    // Edge Cases
    // ============================================================================

    #[test]
    fn test_empty_qp_grid() {
        // Arrange & Act
        let grid = QPGrid::new(0, 0, 64, 64, vec![], -1);

        // Assert
        assert_eq!(grid.grid_w, 0);
        assert_eq!(grid.grid_h, 0);
        assert_eq!(grid.qp.len(), 0);
        assert_eq!(grid.coverage_percent(), 0.0);
    }

    #[test]
    fn test_qp_grid_all_missing() {
        // Arrange
        let qp = vec![-1i16; 100];

        // Act
        let grid = QPGrid::new(10, 10, 64, 64, qp, -1);

        // Assert
        assert_eq!(grid.coverage_percent(), 0.0);
        assert_eq!(grid.qp_min, 0); // Default range
        assert_eq!(grid.qp_max, 63);
    }

    #[test]
    fn test_qp_grid_different_missing_marker() {
        // Arrange
        let qp = vec![20i16, 255, 20, 255, 20]; // 255 as missing marker

        // Act
        let grid = QPGrid::new(5, 1, 64, 64, qp, 255);

        // Assert
        assert_eq!(grid.get(0, 0), Some(20));
        assert_eq!(grid.get(1, 0), None); // 255 is missing
    }

    #[test]
    fn test_color_equality() {
        // Arrange
        let color1 = Color::rgb(100, 150, 200);
        let color2 = Color::rgb(100, 150, 200);
        let color3 = Color::rgb(100, 150, 201);

        // Act & Assert
        assert_eq!(color1, color2);
        assert_ne!(color1, color3);
    }

    #[test]
    fn test_heatmap_texture_small_grid() {
        // Arrange
        let grid = QPGrid::new(2, 2, 64, 64, vec![0, 20, 40, 60], -1);

        // Act
        let texture = HeatmapTexture::generate(
            &grid,
            HeatmapResolution::Full,
            QPScaleMode::Auto,
            1.0,
        );

        // Assert
        assert_eq!(texture.width, 128); // 2 * 64
        assert_eq!(texture.height, 128);
        assert_eq!(texture.pixels.len(), 128 * 128 * 4);
    }
}
