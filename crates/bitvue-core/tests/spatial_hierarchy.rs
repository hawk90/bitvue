//! Tests for spatial hierarchy

use bitvue_core::spatial_hierarchy::{
    BlockEvidence, CodedRect, CtuEvidence, FrameSpatialHierarchy, MotionVector, PredictionMode,
    TileEvidence,
};

#[test]
fn test_coded_rect() {
    let rect = CodedRect::new(100, 200, 64, 64);
    assert!(rect.contains(100, 200));
    assert!(rect.contains(163, 263));
    assert!(!rect.contains(164, 264));
    assert_eq!(rect.area(), 64 * 64);
    assert_eq!(rect.center(), (132, 232));
}

#[test]
fn test_motion_vector() {
    let mv = MotionVector::new(48, -32, 0);
    let (px, py) = mv.to_pixels();
    assert_eq!(px, 12.0);
    assert_eq!(py, -8.0);
    assert!(mv.magnitude() > 0.0);
}

#[test]
fn test_spatial_hierarchy() {
    let mut frame = FrameSpatialHierarchy::new(
        "frame_001".to_string(),
        42,
        40,
        1920,
        1080,
        "decode_001".to_string(),
    );

    let mut tile = TileEvidence::new(
        "tile_001".to_string(),
        0,
        0,
        0,
        CodedRect::new(0, 0, 960, 540),
        "frame_001".to_string(),
        "syn_tile_001".to_string(),
    );

    let mut ctu = CtuEvidence::new(
        "ctu_001".to_string(),
        0,
        CodedRect::new(0, 0, 64, 64),
        "tile_001".to_string(),
        "syn_ctu_001".to_string(),
    );

    let block = BlockEvidence::new(
        "block_001".to_string(),
        0,
        CodedRect::new(0, 0, 8, 8),
        PredictionMode::IntraDc,
        "ctu_001".to_string(),
        "syn_block_001".to_string(),
    );

    ctu.add_block(block);
    tile.add_ctu(ctu);
    frame.add_tile(tile);

    assert_eq!(frame.stats.tile_count, 1);
    assert_eq!(frame.stats.ctu_count, 1);
    assert_eq!(frame.stats.block_count, 1);

    let hit = frame.find_block_at(4, 4);
    assert!(hit.is_some());
    let hit = hit.unwrap();
    assert_eq!(hit.block.mode, PredictionMode::IntraDc);
}
