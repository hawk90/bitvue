//! Tests for QP heatmap overlay

use bitvue_core::qp_heatmap::{
    Color, HeatmapResolution, HeatmapTexture, QPCacheKeyParams, QPColorMapper, QPGrid,
    QPHeatmapCacheKey, QPHeatmapOverlay, QPScaleMode,
};
use bitvue_core::StreamId;

#[test]
fn test_qp_grid_creation() {
    let qp = vec![10, 20, 30, -1, 15, 25];
    let grid = QPGrid::new(3, 2, 8, 8, qp, -1);

    assert_eq!(grid.grid_w, 3);
    assert_eq!(grid.grid_h, 2);
    assert_eq!(grid.block_w, 8);
    assert_eq!(grid.block_h, 8);
    assert_eq!(grid.qp_min, 10);
    assert_eq!(grid.qp_max, 30);
    assert_eq!(grid.missing, -1);
}

#[test]
fn test_qp_grid_get() {
    let qp = vec![10, 20, 30, -1, 15, 25];
    let grid = QPGrid::new(3, 2, 8, 8, qp, -1);

    assert_eq!(grid.get(0, 0), Some(10));
    assert_eq!(grid.get(1, 0), Some(20));
    assert_eq!(grid.get(2, 0), Some(30));
    assert_eq!(grid.get(0, 1), None); // Missing value
    assert_eq!(grid.get(1, 1), Some(15));
    assert_eq!(grid.get(2, 1), Some(25));

    // Out of bounds
    assert_eq!(grid.get(3, 0), None);
    assert_eq!(grid.get(0, 2), None);
}

#[test]
fn test_qp_grid_dimensions() {
    let grid = QPGrid::new(120, 68, 16, 16, vec![], -1);
    assert_eq!(grid.coded_dimensions(), (1920, 1088));
}

#[test]
fn test_qp_grid_coverage() {
    // 50% coverage (3 valid out of 6)
    let qp = vec![10, 20, -1, -1, 15, -1];
    let grid = QPGrid::new(3, 2, 8, 8, qp, -1);
    assert!((grid.coverage_percent() - 50.0).abs() < 0.1);

    // 100% coverage
    let qp = vec![10, 20, 30, 40, 15, 25];
    let grid = QPGrid::new(3, 2, 8, 8, qp, -1);
    assert!((grid.coverage_percent() - 100.0).abs() < 0.1);

    // 0% coverage
    let qp = vec![-1, -1, -1, -1];
    let grid = QPGrid::new(2, 2, 8, 8, qp, -1);
    assert!((grid.coverage_percent() - 0.0).abs() < 0.1);
}

#[test]
fn test_color_lerp() {
    let c1 = Color::rgb(0, 0, 0);
    let c2 = Color::rgb(255, 255, 255);

    let mid = Color::lerp(c1, c2, 0.5);
    assert_eq!(mid.r, 127);
    assert_eq!(mid.g, 127);
    assert_eq!(mid.b, 127);
}

#[test]
fn test_qp_color_mapper() {
    let mapper = QPColorMapper::new(1.0);

    // Missing value -> transparent
    let color = mapper.map_qp(None, 0, 63);
    assert_eq!(color.a, 0);

    // Valid QP values
    let color_min = mapper.map_qp(Some(0), 0, 63);
    assert!(color_min.a > 0);

    let color_max = mapper.map_qp(Some(63), 0, 63);
    assert!(color_max.a > 0);
}

#[test]
fn test_opacity_bucket() {
    assert_eq!(QPColorMapper::opacity_bucket(0.0), 0);
    assert_eq!(QPColorMapper::opacity_bucket(0.45), 9);
    assert_eq!(QPColorMapper::opacity_bucket(0.5), 10);
    assert_eq!(QPColorMapper::opacity_bucket(1.0), 20);

    // Test bucketing (0.42 and 0.47 should round to different buckets)
    assert_eq!(QPColorMapper::opacity_bucket(0.42), 8);
    assert_eq!(QPColorMapper::opacity_bucket(0.47), 9);
}

#[test]
fn test_heatmap_resolution_scale() {
    assert_eq!(HeatmapResolution::Quarter.scale(), 4);
    assert_eq!(HeatmapResolution::Half.scale(), 2);
    assert_eq!(HeatmapResolution::Full.scale(), 1);
}

#[test]
fn test_heatmap_texture_generation() {
    // Small 2x2 grid
    let qp = vec![10, 20, 30, 40];
    let grid = QPGrid::new(2, 2, 8, 8, qp, -1);

    let texture = HeatmapTexture::generate(&grid, HeatmapResolution::Full, QPScaleMode::Auto, 0.5);

    assert_eq!(texture.width, 16); // 2 blocks * 8 pixels
    assert_eq!(texture.height, 16);
    assert_eq!(texture.pixels.len(), (16 * 16 * 4) as usize);
}

#[test]
fn test_qp_heatmap_overlay_creation() {
    let qp = vec![10, 20, 30, 40];
    let grid = QPGrid::new(2, 2, 8, 8, qp, -1);
    let overlay = QPHeatmapOverlay::new(grid);

    assert_eq!(overlay.resolution, HeatmapResolution::Half);
    assert_eq!(overlay.scale_mode, QPScaleMode::Auto);
    assert!((overlay.opacity - 0.45).abs() < 0.01);
    assert!(overlay.cached_texture.is_none());
}

#[test]
fn test_qp_heatmap_coverage_check() {
    // Sufficient coverage (50%)
    let qp = vec![10, 20, -1, -1, 15, -1];
    let grid = QPGrid::new(3, 2, 8, 8, qp, -1);
    let overlay = QPHeatmapOverlay::new(grid);
    assert!(overlay.has_sufficient_coverage());

    // Insufficient coverage (16.6%)
    let qp = vec![10, -1, -1, -1, -1, -1];
    let grid = QPGrid::new(3, 2, 8, 8, qp, -1);
    let overlay = QPHeatmapOverlay::new(grid);
    assert!(!overlay.has_sufficient_coverage());
}

#[test]
fn test_cache_key_generation() {
    let params = QPCacheKeyParams {
        stream: StreamId::A,
        frame_idx: 100,
        resolution: HeatmapResolution::Half,
        scale_mode: QPScaleMode::Auto,
        qp_min: 0,
        qp_max: 63,
        opacity: 0.45,
        codec: "AV1",
        file_path: "/path/to/file.ivf",
    };
    let key = QPHeatmapCacheKey::new(&params);

    let key_str = key.to_string(960, 540);
    assert!(key_str.starts_with("overlay_qp_heat:A:AV1:"));
    assert!(key_str.contains(":f100|hm960x540|scaleauto|op9"));
}

#[test]
fn test_cache_invalidation_on_frame_change() {
    let qp = vec![10, 20, 30, 40];
    let grid = QPGrid::new(2, 2, 8, 8, qp, -1);
    let mut overlay = QPHeatmapOverlay::new(grid);

    let params1 = QPCacheKeyParams {
        stream: StreamId::A,
        frame_idx: 0,
        resolution: HeatmapResolution::Half,
        scale_mode: QPScaleMode::Auto,
        qp_min: 10,
        qp_max: 40,
        opacity: 0.45,
        codec: "AV1",
        file_path: "/file.ivf",
    };
    let key1 = QPHeatmapCacheKey::new(&params1);

    // Generate texture
    let _texture1 = overlay.get_texture(key1.clone());
    assert!(overlay.cache_key.is_some());

    // Different frame -> cache should miss
    let params2 = QPCacheKeyParams {
        stream: StreamId::A,
        frame_idx: 1, // Different frame
        resolution: HeatmapResolution::Half,
        scale_mode: QPScaleMode::Auto,
        qp_min: 10,
        qp_max: 40,
        opacity: 0.45,
        codec: "AV1",
        file_path: "/file.ivf",
    };
    let key2 = QPHeatmapCacheKey::new(&params2);

    let _texture2 = overlay.get_texture(key2.clone());
    assert_eq!(overlay.cache_key.as_ref(), Some(&key2));
}

#[test]
fn test_cache_invalidation_on_resolution_change() {
    let qp = vec![10, 20, 30, 40];
    let grid = QPGrid::new(2, 2, 8, 8, qp, -1);
    let mut overlay = QPHeatmapOverlay::new(grid);

    overlay.set_resolution(HeatmapResolution::Half);
    assert!(overlay.cache_key.is_none());

    // Set different resolution -> cache invalidated
    overlay.set_resolution(HeatmapResolution::Full);
    assert!(overlay.cache_key.is_none());

    // Set same resolution -> no invalidation
    let params = QPCacheKeyParams {
        stream: StreamId::A,
        frame_idx: 0,
        resolution: HeatmapResolution::Full,
        scale_mode: QPScaleMode::Auto,
        qp_min: 0,
        qp_max: 63,
        opacity: 0.45,
        codec: "AV1",
        file_path: "/file.ivf",
    };
    overlay.cache_key = Some(QPHeatmapCacheKey::new(&params));
    overlay.set_resolution(HeatmapResolution::Full);
    assert!(overlay.cache_key.is_some());
}

#[test]
fn test_opacity_bucketing_for_cache() {
    let qp = vec![10, 20, 30, 40];
    let grid = QPGrid::new(2, 2, 8, 8, qp, -1);
    let mut overlay = QPHeatmapOverlay::new(grid);

    // Set opacity within same bucket -> no cache invalidation
    overlay.opacity = 0.45;
    let params = QPCacheKeyParams {
        stream: StreamId::A,
        frame_idx: 0,
        resolution: HeatmapResolution::Half,
        scale_mode: QPScaleMode::Auto,
        qp_min: 0,
        qp_max: 63,
        opacity: 0.45,
        codec: "AV1",
        file_path: "/file.ivf",
    };
    overlay.cache_key = Some(QPHeatmapCacheKey::new(&params));

    overlay.set_opacity(0.47); // Same bucket (9)
    assert!(overlay.cache_key.is_some());

    // Set opacity to different bucket -> cache invalidated
    overlay.set_opacity(0.50); // Different bucket (10)
    assert!(overlay.cache_key.is_none());
}
