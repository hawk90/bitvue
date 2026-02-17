#![allow(dead_code)]
//! Tests for motion vector overlay

use bitvue_core::mv_overlay::{
    DensityControl, MVCacheKey, MVGrid, MVLayer, MVOverlay, MVScaling, MotionVector, Viewport,
    DEFAULT_OPACITY, DEFAULT_USER_SCALE, MISSING_MV,
};

#[test]
fn test_motion_vector_creation() {
    let mv = MotionVector::new(40, -20);
    assert_eq!(mv.dx_qpel, 40);
    assert_eq!(mv.dy_qpel, -20);
    assert!(!mv.is_missing());
}

#[test]
fn test_motion_vector_missing() {
    let mv = MotionVector::MISSING;
    assert!(mv.is_missing());

    let mv2 = MotionVector::new(MISSING_MV, 0);
    assert!(mv2.is_missing());
}

#[test]
fn test_motion_vector_to_pixels() {
    let mv = MotionVector::new(40, -20);
    let (dx_px, dy_px) = mv.to_pixels();
    assert_eq!(dx_px, 10.0);
    assert_eq!(dy_px, -5.0);
}

#[test]
fn test_motion_vector_magnitude() {
    let mv = MotionVector::new(30, 40); // 3-4-5 triangle in qpel
    let mag = mv.magnitude_px();
    assert!((mag - 12.5).abs() < 0.01); // sqrt(7.5^2 + 10^2) = 12.5
}

#[test]
fn test_mv_grid_creation() {
    let grid_w = 10;
    let grid_h = 10;
    let count = (grid_w * grid_h) as usize;

    let mv_l0 = vec![MotionVector::ZERO; count];
    let mv_l1 = vec![MotionVector::ZERO; count];

    let grid = MVGrid::new(160, 160, 16, 16, mv_l0, mv_l1, None);

    assert_eq!(grid.grid_w, 10);
    assert_eq!(grid.grid_h, 10);
    assert_eq!(grid.block_count(), 100);
}

#[test]
fn test_mv_grid_get() {
    let mv_l0 = vec![MotionVector::new(10, 20); 100];
    let mv_l1 = vec![MotionVector::new(30, 40); 100];

    let grid = MVGrid::new(160, 160, 16, 16, mv_l0, mv_l1, None);

    let l0 = grid.get_l0(5, 5).unwrap();
    assert_eq!(l0.dx_qpel, 10);
    assert_eq!(l0.dy_qpel, 20);

    let l1 = grid.get_l1(5, 5).unwrap();
    assert_eq!(l1.dx_qpel, 30);
    assert_eq!(l1.dy_qpel, 40);

    // Out of bounds
    assert!(grid.get_l0(20, 5).is_none());
}

#[test]
fn test_mv_grid_block_center() {
    let grid = MVGrid::new(
        160,
        160,
        16,
        16,
        vec![MotionVector::ZERO; 100],
        vec![MotionVector::ZERO; 100],
        None,
    );

    let (x, y) = grid.block_center(0, 0);
    assert_eq!(x, 8.0);
    assert_eq!(y, 8.0);

    let (x, y) = grid.block_center(1, 1);
    assert_eq!(x, 24.0);
    assert_eq!(y, 24.0);
}

#[test]
fn test_density_control_stride_calculation() {
    // Below threshold: stride = 1
    assert_eq!(DensityControl::calculate_stride(5000), 1);
    assert_eq!(DensityControl::calculate_stride(8000), 1);

    // Above threshold: stride increases
    assert_eq!(DensityControl::calculate_stride(16000), 2); // sqrt(16000/8000) = sqrt(2) ≈ 1.41 → 2
    assert_eq!(DensityControl::calculate_stride(32000), 2); // sqrt(32000/8000) = 2
    assert_eq!(DensityControl::calculate_stride(72000), 3); // sqrt(72000/8000) = 3
}

#[test]
fn test_density_control_should_draw() {
    // Stride 1: all blocks
    assert!(DensityControl::should_draw(0, 0, 1));
    assert!(DensityControl::should_draw(5, 7, 1));

    // Stride 2: every other block
    assert!(DensityControl::should_draw(0, 0, 2));
    assert!(DensityControl::should_draw(2, 4, 2));
    assert!(!DensityControl::should_draw(1, 2, 2));
    assert!(!DensityControl::should_draw(3, 5, 2));

    // Stride 3
    assert!(DensityControl::should_draw(0, 0, 3));
    assert!(DensityControl::should_draw(3, 6, 3));
    assert!(!DensityControl::should_draw(1, 3, 3));
}

#[test]
fn test_mv_scaling() {
    let mv = MotionVector::new(40, -20); // 10px, -5px

    let (dx, dy) = MVScaling::scale_vector(&mv, 1.0, 1.0);
    assert_eq!(dx, 10.0);
    assert_eq!(dy, -5.0);

    let (dx, dy) = MVScaling::scale_vector(&mv, 2.0, 1.0);
    assert_eq!(dx, 20.0);
    assert_eq!(dy, -10.0);

    let (dx, dy) = MVScaling::scale_vector(&mv, 1.0, 2.0);
    assert_eq!(dx, 20.0);
    assert_eq!(dy, -10.0);
}

#[test]
fn test_mv_scaling_clamp() {
    // Vector longer than max
    let (dx, dy) = MVScaling::clamp_arrow_length(60.0, 80.0, 48.0);
    let magnitude = (dx * dx + dy * dy).sqrt();
    assert!((magnitude - 48.0).abs() < 0.01);

    // Vector shorter than max (unchanged)
    let (dx, dy) = MVScaling::clamp_arrow_length(20.0, 30.0, 48.0);
    assert_eq!(dx, 20.0);
    assert_eq!(dy, 30.0);
}

#[test]
fn test_viewport_contains_block() {
    let vp = Viewport::new(100, 100, 200, 200);

    // Block fully inside
    assert!(vp.contains_block(150, 150, 16, 16));

    // Block partially overlapping
    assert!(vp.contains_block(90, 90, 20, 20));
    assert!(vp.contains_block(290, 290, 20, 20));

    // Block outside
    assert!(!vp.contains_block(0, 0, 16, 16));
    assert!(!vp.contains_block(400, 400, 16, 16));
}

#[test]
fn test_cache_key_generation() {
    let vp = Viewport::new(0, 0, 1920, 1080);
    let key = MVCacheKey::new("A".to_string(), 42, vp, 2, MVLayer::L0Only, 1.5, 0.55);

    assert_eq!(key.frame_idx, 42);
    assert_eq!(key.stride, 2);
    assert_eq!(key.layer, "L0");
    assert_eq!(key.scale_bucket, 15); // 1.5 * 10
    assert_eq!(key.opacity_bucket, 11); // 0.55 * 20

    let key_str = key.to_string();
    assert!(key_str.contains("f42"));
    assert!(key_str.contains("s2"));
    assert!(key_str.contains("LL0"));
}

#[test]
fn test_mv_overlay_creation() {
    let grid = MVGrid::new(
        1920,
        1080,
        16,
        16,
        vec![MotionVector::ZERO; 120 * 68],
        vec![MotionVector::ZERO; 120 * 68],
        None,
    );

    let overlay = MVOverlay::new(grid);
    assert_eq!(overlay.user_scale, DEFAULT_USER_SCALE);
    assert_eq!(overlay.opacity, DEFAULT_OPACITY);
    assert!(overlay.cached_visible.is_none());
}

#[test]
fn test_mv_overlay_layer_change_invalidates_cache() {
    let grid = MVGrid::new(
        160,
        160,
        16,
        16,
        vec![MotionVector::ZERO; 100],
        vec![MotionVector::ZERO; 100],
        None,
    );

    let mut overlay = MVOverlay::new(grid);
    overlay.cached_visible = Some(vec![(0, 0)]);
    overlay.cache_key = Some(MVCacheKey::new(
        "A".to_string(),
        0,
        Viewport::new(0, 0, 160, 160),
        1,
        MVLayer::Both,
        1.0,
        0.55,
    ));

    overlay.set_layer(MVLayer::L0Only);
    assert!(overlay.cached_visible.is_none());
    assert!(overlay.cache_key.is_none());
}
