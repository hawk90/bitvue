#![allow(hidden_glob_reexports)]
#![allow(unreachable_code)]
#![allow(non_camel_case_types)]
#![allow(unused_assignments)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(unused_comparisons)]
#![allow(unused_doc_comments)]
//! Tests for block_metrics module

use crate::{
    BlockMetricType, BlockMetricValue, BlockMetricsCalculator, BlockMetricsColorMapper,
    BlockMetricsGrid, BlockMetricsOverlay, BlockMetricsStatistics, MultiFrameBlockMetrics,
};

#[test]
fn test_block_metric_type() {
    assert_eq!(BlockMetricType::Psnr.name(), "PSNR");
    assert_eq!(BlockMetricType::Psnr.unit(), "dB");
    assert!(BlockMetricType::Psnr.higher_is_better());
    assert!(!BlockMetricType::Mse.higher_is_better());
}

#[test]
fn test_block_metric_value() {
    let val = BlockMetricValue::new(5, 10, 35.0, BlockMetricType::Psnr);
    assert_eq!(val.block_x, 5);
    assert_eq!(val.block_y, 10);
    assert!((val.value - 35.0).abs() < 0.001);

    // 35 dB with range 20-50 should normalize to 0.5
    let norm = val.normalized();
    assert!((norm - 0.5).abs() < 0.001);
}

#[test]
fn test_block_metrics_grid_creation() {
    let grid = BlockMetricsGrid::new(0, 1920, 1080, 16, BlockMetricType::Psnr);
    assert_eq!(grid.display_idx, 0);
    assert_eq!(grid.block_size, 16);
    assert_eq!(grid.width_blocks, 120); // 1920/16
    assert_eq!(grid.height_blocks, 68); // ceil(1080/16)
    assert_eq!(grid.total_blocks(), 120 * 68);
}

#[test]
fn test_block_metrics_grid_get_set() {
    let mut grid = BlockMetricsGrid::new(0, 320, 240, 16, BlockMetricType::Psnr);

    grid.set(5, 10, 42.5);
    assert_eq!(grid.get(5, 10), Some(42.5));
    assert_eq!(grid.get(100, 100), None); // Out of bounds
}

#[test]
fn test_block_metrics_grid_pixel_conversion() {
    let grid = BlockMetricsGrid::new(0, 320, 240, 16, BlockMetricType::Psnr);

    let (bx, by) = grid.pixel_to_block(50, 100);
    assert_eq!(bx, 3); // 50/16 = 3
    assert_eq!(by, 6); // 100/16 = 6

    let (x0, y0, x1, y1) = grid.block_to_pixel_range(3, 6);
    assert_eq!(x0, 48);
    assert_eq!(y0, 96);
    assert_eq!(x1, 64);
    assert_eq!(y1, 112);
}

#[test]
fn test_block_metrics_statistics() {
    let mut grid = BlockMetricsGrid::new(0, 64, 64, 16, BlockMetricType::Psnr);
    // 4x4 blocks
    grid.values = vec![
        30.0, 35.0, 40.0, 45.0, 32.0, 37.0, 42.0, 47.0, 34.0, 39.0, 44.0, 49.0, 36.0, 41.0, 46.0,
        51.0,
    ];

    let stats = BlockMetricsStatistics::from_grid(&grid, 35.0);
    assert!((stats.min - 30.0).abs() < 0.001);
    assert!((stats.max - 51.0).abs() < 0.001);
    assert_eq!(stats.total_blocks, 16);
    assert!(stats.quality_percent() > 0.0);
}

#[test]
fn test_block_metrics_calculator_psnr() {
    let calc = BlockMetricsCalculator::new(16, 8);

    // Identical blocks should have high PSNR
    let block = vec![128u8; 256];
    let psnr = calc.calculate_psnr_block(&block, &block);
    assert!(psnr >= 99.0); // Perfect match

    // Different blocks
    let ref_block = vec![100u8; 256];
    let psnr2 = calc.calculate_psnr_block(&block, &ref_block);
    assert!(psnr2 < 100.0);
    assert!(psnr2 > 0.0);
}

#[test]
fn test_block_metrics_calculator_mse() {
    let calc = BlockMetricsCalculator::new(16, 8);

    // Identical blocks should have 0 MSE
    let block = vec![128u8; 256];
    let mse = calc.calculate_mse_block(&block, &block);
    assert!((mse - 0.0).abs() < 0.001);

    // Known difference
    let ref_block = vec![130u8; 256]; // Diff of 2
    let mse2 = calc.calculate_mse_block(&block, &ref_block);
    assert!((mse2 - 4.0).abs() < 0.001); // 2^2 = 4
}

#[test]
fn test_block_metrics_calculator_ssim() {
    let calc = BlockMetricsCalculator::new(16, 8);

    // Identical blocks should have SSIM = 1
    let block = vec![128u8; 256];
    let ssim = calc.calculate_ssim_block(&block, &block);
    assert!((ssim - 1.0).abs() < 0.01);

    // Different blocks should have lower SSIM
    let ref_block: Vec<u8> = (0..256).map(|i| (i % 256) as u8).collect();
    let ssim2 = calc.calculate_ssim_block(&block, &ref_block);
    assert!(ssim2 < 1.0);
}

#[test]
fn test_block_metrics_color_mapper() {
    let mapper = BlockMetricsColorMapper::for_psnr();

    // Low PSNR should be red-ish
    let low_color = mapper.map_color(20.0);
    assert!(low_color[0] > low_color[1]); // More red than green

    // High PSNR should be green-ish
    let high_color = mapper.map_color(50.0);
    assert!(high_color[1] > high_color[0]); // More green than red
}

#[test]
fn test_block_metrics_overlay() {
    let mut grid = BlockMetricsGrid::new(0, 64, 64, 16, BlockMetricType::Psnr);
    for (i, v) in grid.values.iter_mut().enumerate() {
        *v = 30.0 + (i as f32);
    }

    let mapper = BlockMetricsColorMapper::for_psnr();
    let overlay = BlockMetricsOverlay::new(grid, mapper);

    assert!(overlay.statistics.total_blocks > 0);
    assert!(overlay.get_block_color(0, 0).is_some());
}

#[test]
fn test_block_metrics_overlay_to_rgba() {
    let mut grid = BlockMetricsGrid::new(0, 32, 32, 16, BlockMetricType::Psnr);
    grid.values = vec![35.0, 40.0, 45.0, 50.0]; // 2x2 blocks

    let mapper = BlockMetricsColorMapper::for_psnr();
    let overlay = BlockMetricsOverlay::new(grid, mapper);

    let rgba = overlay.to_rgba();
    assert_eq!(rgba.len(), 32 * 32 * 4);
}

#[test]
fn test_multi_frame_block_metrics() {
    let mut mf = MultiFrameBlockMetrics::new(BlockMetricType::Psnr, 16);

    let grid1 = BlockMetricsGrid::new(0, 64, 64, 16, BlockMetricType::Psnr);
    let grid2 = BlockMetricsGrid::new(1, 64, 64, 16, BlockMetricType::Psnr);

    mf.add_frame(grid1);
    mf.add_frame(grid2);

    assert_eq!(mf.frame_count(), 2);
    assert!(mf.get_frame(0).is_some());
    assert!(mf.get_frame(1).is_some());
    assert!(mf.get_frame(2).is_none());
}

#[test]
fn test_threshold_detection() {
    let mapper = BlockMetricsColorMapper::for_psnr();
    assert!(mapper.is_below_threshold(25.0)); // 25 dB < 30 dB threshold
    assert!(!mapper.is_below_threshold(35.0)); // 35 dB > 30 dB threshold
}
