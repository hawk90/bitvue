#![allow(dead_code)]
//! Tests for player evidence manager

use bitvue_core::coordinate_transform::{BlockIdx, ScreenPx};
use bitvue_core::player_evidence::PlayerEvidenceManager;
use bitvue_core::{
    BitOffsetIndex, CoordinateTransformer, DecodeIndex, EvidenceChain, ScreenRect, SyntaxIndex,
    VizElementType, VizIndex, ZoomMode,
};

fn create_test_manager() -> PlayerEvidenceManager {
    // Create coordinate transformer (1920x1080 frame in 960x540 viewport)
    let transformer = CoordinateTransformer::new(
        1920,
        1080,
        ScreenRect::new(0.0, 0.0, 960.0, 540.0),
        ZoomMode::Fit,
    );

    // Create empty evidence chain
    let evidence_chain = EvidenceChain {
        bit_offset_index: BitOffsetIndex::new(),
        syntax_index: SyntaxIndex::new(),
        decode_index: DecodeIndex::new(),
        viz_index: VizIndex::new(),
    };

    PlayerEvidenceManager::new(transformer, evidence_chain)
}

#[test]
fn test_create_pixel_evidence() {
    let mut manager = create_test_manager();

    // Create pixel evidence at center of screen
    let screen = ScreenPx::new(480.0, 270.0);
    let viz = manager
        .create_pixel_evidence(screen, 0, "decode_001".to_string())
        .unwrap();

    assert!(matches!(
        viz.element_type,
        VizElementType::Custom(ref s) if s == "pixel_hover"
    ));
    assert_eq!(viz.screen_rect, Some((480.0, 270.0, 1.0, 1.0)));
    assert!(viz.coded_rect.is_some());

    // Center of screen should map to center of coded frame (960, 540)
    let coded = viz.coded_rect.unwrap();
    assert!((coded.0 as i32 - 960).abs() <= 1);
    assert!((coded.1 as i32 - 540).abs() <= 1);
}

#[test]
fn test_create_block_evidence() {
    let mut manager = create_test_manager();

    // Create block evidence at top-left corner (screen 0,0 = coded 0,0 = block 0,0)
    let screen = ScreenPx::new(0.0, 0.0);
    let viz = manager
        .create_block_evidence(screen, 0, 8, "decode_001".to_string())
        .unwrap();

    assert!(matches!(
        viz.element_type,
        VizElementType::Custom(ref s) if s == "block_hover"
    ));
    assert!(viz.coded_rect.is_some());

    // Should be block (0, 0)
    assert_eq!(
        viz.visual_properties.get("block_col"),
        Some(&"0".to_string())
    );
    assert_eq!(
        viz.visual_properties.get("block_row"),
        Some(&"0".to_string())
    );
}

#[test]
fn test_create_qp_heatmap_evidence() {
    let mut manager = create_test_manager();

    // Create QP heatmap evidence for block (10, 20)
    let block = BlockIdx::new(10, 20);
    let viz = manager.create_qp_heatmap_evidence(block, 28, 0, 8, "decode_001".to_string());

    assert_eq!(viz.element_type, VizElementType::QpHeatmap);
    assert_eq!(
        viz.visual_properties.get("qp_value"),
        Some(&"28".to_string())
    );
    assert_eq!(
        viz.visual_properties.get("block_col"),
        Some(&"10".to_string())
    );
    assert_eq!(
        viz.visual_properties.get("block_row"),
        Some(&"20".to_string())
    );
}

#[test]
fn test_create_mv_overlay_evidence() {
    let mut manager = create_test_manager();

    // Create MV overlay evidence for block (5, 5) with MV (3.5, -2.0)
    let block = BlockIdx::new(5, 5);
    let viz = manager.create_mv_overlay_evidence(block, 3.5, -2.0, 0, 16, "decode_001".to_string());

    assert_eq!(viz.element_type, VizElementType::MotionVectorOverlay);
    assert_eq!(viz.visual_properties.get("mv_x"), Some(&"3.50".to_string()));
    assert_eq!(
        viz.visual_properties.get("mv_y"),
        Some(&"-2.00".to_string())
    );

    // Check MV magnitude
    let magnitude = (3.5_f32 * 3.5 + 2.0 * 2.0).sqrt();
    let stored_mag = viz.visual_properties.get("mv_magnitude").unwrap();
    assert!(stored_mag.contains(&format!("{:.2}", magnitude)));
}

#[test]
fn test_create_partition_evidence() {
    let mut manager = create_test_manager();

    // Create partition evidence for block (15, 10)
    let block = BlockIdx::new(15, 10);
    let viz = manager.create_partition_evidence(
        block,
        "SPLIT".to_string(),
        0,
        8,
        "decode_001".to_string(),
    );

    assert_eq!(viz.element_type, VizElementType::PartitionGridOverlay);
    assert_eq!(
        viz.visual_properties.get("partition_type"),
        Some(&"SPLIT".to_string())
    );
}

#[test]
fn test_clear_viz_evidence() {
    let mut manager = create_test_manager();

    // Create some viz evidence
    manager.create_qp_heatmap_evidence(BlockIdx::new(0, 0), 28, 0, 8, "decode_001".to_string());
    manager.create_qp_heatmap_evidence(BlockIdx::new(1, 1), 30, 0, 8, "decode_001".to_string());

    assert_eq!(manager.viz_evidence_count(), 2);

    // Clear
    manager.clear_viz_evidence();

    assert_eq!(manager.viz_evidence_count(), 0);
}

#[test]
fn test_coordinate_transform_contract() {
    // Per COORDINATE_SYSTEM_CONTRACT.md:
    // Hover/click mapping is identical across overlays

    let mut manager = create_test_manager();

    let screen = ScreenPx::new(480.0, 270.0);

    // Create different overlay types at same screen position
    let pixel_viz = manager
        .create_pixel_evidence(screen, 0, "decode_001".to_string())
        .unwrap();
    let block_viz = manager
        .create_block_evidence(screen, 0, 8, "decode_001".to_string())
        .unwrap();

    // Both should have overlapping screen regions
    assert!((pixel_viz.screen_rect.unwrap().0 - 480.0).abs() < 1.0);
    let block_rect = block_viz.screen_rect.unwrap();
    // Pixel should be inside block bounds
    assert!(pixel_viz.screen_rect.unwrap().0 >= block_rect.0);
    assert!(pixel_viz.screen_rect.unwrap().0 <= block_rect.0 + block_rect.2);
}

// UX Player evidence chain integration tests
// Deliverable: evidence_chain_01_bit_offset:UX:Player:ALL:evidence_chain

#[test]
fn test_ux_player_pixel_hover_evidence_trace() {
    let mut manager = create_test_manager();

    // UX Player: User hovers over pixel at screen position (320, 180)
    let screen = ScreenPx::new(320.0, 180.0);
    let viz = manager
        .create_pixel_evidence(screen, 0, "decode_frame_0".to_string())
        .unwrap();

    // UX Player: Verify evidence chain linkage
    assert_eq!(viz.decode_link, "decode_frame_0");
    assert!(viz.screen_rect.is_some());
    assert!(viz.coded_rect.is_some());

    // UX Player: Screen coordinates should be preserved
    let screen_rect = viz.screen_rect.unwrap();
    assert!((screen_rect.0 - 320.0).abs() < 1.0);
    assert!((screen_rect.1 - 180.0).abs() < 1.0);
}

#[test]
fn test_ux_player_block_click_evidence_trace() {
    let mut manager = create_test_manager();

    // UX Player: User clicks on block at screen position (240, 135)
    let screen = ScreenPx::new(240.0, 135.0);
    let viz = manager
        .create_block_evidence(screen, 5, 16, "decode_frame_5".to_string())
        .unwrap();

    // UX Player: Verify block evidence includes coordinate transforms
    assert_eq!(viz.decode_link, "decode_frame_5");
    assert_eq!(viz.frame_idx, Some(5));
    assert_eq!(viz.display_idx, Some(5));

    // UX Player: Verify coded rect is aligned to block size
    let coded_rect = viz.coded_rect.unwrap();
    assert_eq!(coded_rect.2, 16); // Block width
    assert_eq!(coded_rect.3, 16); // Block height
}

#[test]
fn test_ux_player_qp_heatmap_hover_evidence() {
    let mut manager = create_test_manager();

    // UX Player: User hovers over QP heatmap at block (5, 3)
    let block = BlockIdx::new(5, 3);
    let viz = manager.create_qp_heatmap_evidence(block, 42, 10, 8, "decode_frame_10".to_string());

    // UX Player: Verify QP value is captured in visual properties
    assert_eq!(
        viz.visual_properties.get("qp_value"),
        Some(&"42".to_string())
    );
    assert_eq!(viz.element_type, VizElementType::QpHeatmap);

    // UX Player: Verify block index is captured
    assert_eq!(viz.coded_rect.unwrap(), (40, 24, 8, 8)); // 5*8, 3*8, block_size, block_size
}

#[test]
fn test_ux_player_motion_vector_click_evidence() {
    let mut manager = create_test_manager();

    // UX Player: User clicks on motion vector at block (2, 4)
    let block = BlockIdx::new(2, 4);
    let viz = manager.create_mv_overlay_evidence(
        block,
        10.0,
        -5.0,
        20,
        16,
        "decode_frame_20".to_string(),
    );

    // UX Player: Verify motion vector components are captured
    assert_eq!(
        viz.visual_properties.get("mv_x"),
        Some(&"10.00".to_string())
    );
    assert_eq!(
        viz.visual_properties.get("mv_y"),
        Some(&"-5.00".to_string())
    );
    assert_eq!(viz.element_type, VizElementType::MotionVectorOverlay);

    // UX Player: Verify frame linkage
    assert_eq!(viz.frame_idx, Some(20));
    assert_eq!(viz.decode_link, "decode_frame_20");
}

#[test]
fn test_ux_player_multi_overlay_evidence_tracking() {
    let mut manager = create_test_manager();

    // UX Player: User enables multiple overlays (QP + MV + Partition)
    let block = BlockIdx::new(1, 1);

    // Create QP heatmap evidence
    manager.create_qp_heatmap_evidence(block, 28, 0, 8, "decode_frame_0".to_string());

    // Create MV overlay evidence at same block
    manager.create_mv_overlay_evidence(block, 8.0, 4.0, 0, 8, "decode_frame_0".to_string());

    // Create partition grid evidence at same block
    manager.create_partition_evidence(
        block,
        "SPLIT".to_string(),
        0,
        8,
        "decode_frame_0".to_string(),
    );

    // UX Player: Verify all overlays are tracked
    assert_eq!(manager.viz_evidence_count(), 3);

    // UX Player: User disables overlays (clears viz evidence)
    manager.clear_viz_evidence();
    assert_eq!(manager.viz_evidence_count(), 0);
}

#[test]
fn test_ux_player_zoom_coordinate_transform() {
    let transformer = CoordinateTransformer::new(
        1920,
        1080,
        ScreenRect::new(0.0, 0.0, 960.0, 540.0),
        ZoomMode::Fit,
    );
    let evidence_chain = EvidenceChain {
        bit_offset_index: BitOffsetIndex::new(),
        syntax_index: SyntaxIndex::new(),
        decode_index: DecodeIndex::new(),
        viz_index: VizIndex::new(),
    };
    let mut manager = PlayerEvidenceManager::new(transformer, evidence_chain);

    // UX Player: User zooms to 200% (2x)
    let new_transformer = CoordinateTransformer::new(
        1920,
        1080,
        ScreenRect::new(0.0, 0.0, 960.0, 540.0),
        ZoomMode::Custom(200), // 200% = 2x zoom
    );
    manager.update_transformer(new_transformer);

    // UX Player: Create evidence at screen center after zoom
    let screen = ScreenPx::new(480.0, 270.0);
    let viz = manager
        .create_pixel_evidence(screen, 0, "decode_frame_0".to_string())
        .unwrap();

    // UX Player: Verify evidence is created with new transform
    assert!(viz.screen_rect.is_some());
    assert!(viz.coded_rect.is_some());

    // UX Player: Verify screen coordinates are preserved regardless of zoom
    let screen_rect = viz.screen_rect.unwrap();
    assert!((screen_rect.0 - 480.0).abs() < 1.0);
    assert!((screen_rect.1 - 270.0).abs() < 1.0);

    // UX Player: Verify coded rect is within frame bounds (1920x1080)
    let coded_rect = viz.coded_rect.unwrap();
    assert!(coded_rect.0 < 1920);
    assert!(coded_rect.1 < 1080);
}

// AV1 Player evidence chain test - Task 22 (S.T4-2.AV1.Timeline.Player.impl.evidence_chain.001)

#[test]
fn test_av1_player_decode_evidence_chain() {
    // AV1 Player: User clicks on decoded AV1 frame pixel to trace back to OBU
    let transformer = CoordinateTransformer::new(
        640,
        360,
        ScreenRect::new(0.0, 0.0, 640.0, 360.0),
        ZoomMode::Fit,
    );

    let evidence_chain = EvidenceChain {
        bit_offset_index: BitOffsetIndex::new(),
        syntax_index: SyntaxIndex::new(),
        decode_index: DecodeIndex::new(),
        viz_index: VizIndex::new(),
    };

    let mut manager = PlayerEvidenceManager::new(transformer, evidence_chain);

    // AV1 Player: User clicks on pixel at screen (320, 180) - center of 640x360 frame
    let screen_center = ScreenPx::new(320.0, 180.0);

    // AV1 Player: Create evidence linking screen pixel to AV1 KEY_FRAME decode
    let viz = manager
        .create_pixel_evidence(screen_center, 0, "av1_keyframe_0".to_string())
        .unwrap();

    // AV1 Player: Verify evidence contains full chain
    assert_eq!(
        viz.element_type,
        VizElementType::Custom("pixel_hover".to_string())
    );
    assert_eq!(viz.frame_idx, Some(0));

    // AV1 Player: Verify screen coordinates are preserved
    assert!(viz.screen_rect.is_some());
    let (sx, sy, _, _) = viz.screen_rect.unwrap();
    assert!((sx - 320.0).abs() < 1.0);
    assert!((sy - 180.0).abs() < 1.0);

    // AV1 Player: Verify coded coordinates map to AV1 frame (640x360)
    assert!(viz.coded_rect.is_some());
    let (cx, cy, _, _) = viz.coded_rect.unwrap();
    assert!(cx < 640);
    assert!(cy < 360);

    // AV1 Player: Create evidence for pixel in top-left region of AV1 frame
    let top_left = ScreenPx::new(32.0, 32.0);
    let viz_tl = manager
        .create_pixel_evidence(top_left, 0, "av1_keyframe_0".to_string())
        .unwrap();

    // AV1 Player: Verify coded coordinates are in top-left region
    assert!(viz_tl.coded_rect.is_some());
    let (tlx, tly, _, _) = viz_tl.coded_rect.unwrap();
    assert!(tlx < 100); // Top-left region horizontally
    assert!(tly < 100); // Top-left region vertically

    // AV1 Player: Create evidence for multiple decode tiers (fast path vs quality)
    // Fast path: Half resolution (320x180) decode
    let viz_fast = manager
        .create_pixel_evidence(
            ScreenPx::new(160.0, 90.0),
            1,
            "av1_inter_1_half".to_string(),
        )
        .unwrap();
    assert_eq!(viz_fast.frame_idx, Some(1));

    // Quality path: Full resolution (640x360) decode
    let viz_quality = manager
        .create_pixel_evidence(
            ScreenPx::new(320.0, 180.0),
            1,
            "av1_inter_1_full".to_string(),
        )
        .unwrap();
    assert_eq!(viz_quality.frame_idx, Some(1));

    // AV1 Player: Different decode tiers create distinct evidence chains
    assert_ne!(viz_fast.id, viz_quality.id);

    // AV1 Player: Verify evidence IDs are unique and sequential
    let ids: Vec<&str> = vec![&viz.id, &viz_tl.id, &viz_fast.id, &viz_quality.id];
    assert_eq!(ids[0], "player_viz_0");
    assert_eq!(ids[1], "player_viz_1");
    assert_eq!(ids[2], "player_viz_2");
    assert_eq!(ids[3], "player_viz_3");

    // AV1 Player: Get evidence chain for verification
    let chain = manager.evidence_chain();
    assert_eq!(chain.viz_index.all().len(), 4);
}
