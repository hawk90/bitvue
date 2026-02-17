#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(unused_comparisons)]
#![allow(unused_doc_comments)]
//! Tests for coordinate_transform module

use bitvue_core::{
    BlockIdx, CodedPx, CoordinateTransformer, ScreenPx, ScreenRect, VideoRectNorm, ZoomMode,
};

#[test]
fn test_screen_to_norm_fit_mode() {
    // 1920x1080 frame displayed in 960x540 rect
    let transformer =
        CoordinateTransformer::new_fit(1920, 1080, ScreenRect::new(0.0, 0.0, 960.0, 540.0));

    // Top-left corner
    let norm = transformer.screen_to_norm(ScreenPx::new(0.0, 0.0)).unwrap();
    assert_eq!(norm, VideoRectNorm::new(0.0, 0.0));

    // Bottom-right corner
    let norm = transformer
        .screen_to_norm(ScreenPx::new(960.0, 540.0))
        .unwrap();
    assert!((norm.x - 1.0).abs() < 0.001);
    assert!((norm.y - 1.0).abs() < 0.001);

    // Center
    let norm = transformer
        .screen_to_norm(ScreenPx::new(480.0, 270.0))
        .unwrap();
    assert!((norm.x - 0.5).abs() < 0.001);
    assert!((norm.y - 0.5).abs() < 0.001);

    // Outside video rect
    let norm = transformer.screen_to_norm(ScreenPx::new(1000.0, 600.0));
    assert!(norm.is_none());
}

#[test]
fn test_norm_to_coded() {
    let transformer =
        CoordinateTransformer::new_fit(1920, 1080, ScreenRect::new(0.0, 0.0, 960.0, 540.0));

    // Top-left
    let coded = transformer.norm_to_coded(VideoRectNorm::new(0.0, 0.0));
    assert_eq!(coded, CodedPx::new(0.0, 0.0));

    // Bottom-right
    let coded = transformer.norm_to_coded(VideoRectNorm::new(1.0, 1.0));
    assert_eq!(coded, CodedPx::new(1920.0, 1080.0));

    // Center
    let coded = transformer.norm_to_coded(VideoRectNorm::new(0.5, 0.5));
    assert_eq!(coded, CodedPx::new(960.0, 540.0));
}

#[test]
fn test_coded_to_block() {
    let transformer =
        CoordinateTransformer::new_fit(1920, 1080, ScreenRect::new(0.0, 0.0, 960.0, 540.0));

    // Top-left pixel → block (0, 0)
    let block = transformer.coded_to_block(CodedPx::new(0.0, 0.0), None);
    assert_eq!(block, BlockIdx::new(0, 0));

    // Pixel (8, 8) → block (1, 1)
    let block = transformer.coded_to_block(CodedPx::new(8.0, 8.0), None);
    assert_eq!(block, BlockIdx::new(1, 1));

    // Pixel (15, 15) → block (1, 1) (still in same 8x8 block)
    let block = transformer.coded_to_block(CodedPx::new(15.0, 15.0), None);
    assert_eq!(block, BlockIdx::new(1, 1));

    // Test 16x16 blocks
    let block = transformer.coded_to_block(CodedPx::new(32.0, 48.0), Some(16));
    assert_eq!(block, BlockIdx::new(2, 3));
}

#[test]
fn test_screen_to_coded_roundtrip() {
    let transformer =
        CoordinateTransformer::new_fit(1920, 1080, ScreenRect::new(100.0, 50.0, 960.0, 540.0));

    let screen = ScreenPx::new(580.0, 320.0); // Center of video rect
    let coded = transformer.screen_to_coded(screen).unwrap();
    let back_to_screen = transformer.coded_to_screen(coded);

    assert!((screen.x - back_to_screen.x).abs() < 0.1);
    assert!((screen.y - back_to_screen.y).abs() < 0.1);
}

#[test]
fn test_block_to_coded_roundtrip() {
    let transformer =
        CoordinateTransformer::new_fit(1920, 1080, ScreenRect::new(0.0, 0.0, 960.0, 540.0));

    let block = BlockIdx::new(10, 20);
    let coded = transformer.block_to_coded(block, None);
    let back_to_block = transformer.coded_to_block(coded, None);

    assert_eq!(block, back_to_block);
}

#[test]
fn test_zoom_200_percent() {
    let mut transformer =
        CoordinateTransformer::new_fit(1920, 1080, ScreenRect::new(0.0, 0.0, 960.0, 540.0));

    transformer.set_zoom(ZoomMode::Percent200);

    // At 200% zoom with no pan, center of screen should map to (0.25, 0.25) in norm
    let norm = transformer
        .screen_to_norm(ScreenPx::new(480.0, 270.0))
        .unwrap();
    assert!((norm.x - 0.25).abs() < 0.01);
    assert!((norm.y - 0.25).abs() < 0.01);
}

#[test]
fn test_block_grid_size() {
    let transformer =
        CoordinateTransformer::new_fit(1920, 1080, ScreenRect::new(0.0, 0.0, 960.0, 540.0));

    // 8x8 blocks: 1920/8 = 240, 1080/8 = 135
    let (cols, rows) = transformer.block_grid_size(None);
    assert_eq!(cols, 240);
    assert_eq!(rows, 135);

    // 16x16 blocks: 1920/16 = 120, 1080/16 = 67.5 → 68 (ceiling)
    let (cols, rows) = transformer.block_grid_size(Some(16));
    assert_eq!(cols, 120);
    assert_eq!(rows, 68);
}

#[test]
fn test_block_linear_indexing() {
    let block = BlockIdx::new(5, 3);
    let grid_width = 10;

    // Linear index = 3*10 + 5 = 35
    assert_eq!(block.to_linear(grid_width), 35);

    // Reverse
    let recovered = BlockIdx::from_linear(35, grid_width);
    assert_eq!(recovered, block);
}

#[test]
fn test_contract_invariant_viewport_independence() {
    // Per COORDINATE_SYSTEM_CONTRACT.md:
    // Block mapping NEVER depends on viewport

    // Same frame, different video rects
    let transformer1 =
        CoordinateTransformer::new_fit(1920, 1080, ScreenRect::new(0.0, 0.0, 960.0, 540.0));

    let transformer2 =
        CoordinateTransformer::new_fit(1920, 1080, ScreenRect::new(100.0, 100.0, 1200.0, 675.0));

    // Same coded pixel should map to same block in both transformers
    let coded = CodedPx::new(256.0, 128.0);
    let block1 = transformer1.coded_to_block(coded, None);
    let block2 = transformer2.coded_to_block(coded, None);

    assert_eq!(block1, block2);
}
