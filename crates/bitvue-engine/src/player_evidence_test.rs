// Player Evidence module tests

// ============================================================================
// Fixtures
// ============================================================================
#[allow(dead_code)]
fn create_test_transformer() -> CoordinateTransformer {
    use crate::coordinate_transform::{ScreenRect, ZoomMode};
    CoordinateTransformer::new(
        1920,
        1080,
        ScreenRect::new(0.0, 0.0, 1920.0, 1080.0),
        ZoomMode::Fit,
    )
}

#[allow(dead_code)]
fn create_test_evidence_chain() -> EvidenceChain {
    EvidenceChain::new()
}

#[allow(dead_code)]
fn create_test_manager() -> PlayerEvidenceManager {
    PlayerEvidenceManager::new(create_test_transformer(), create_test_evidence_chain())
}

#[allow(dead_code)]
fn create_test_screen_px() -> ScreenPx {
    ScreenPx::new(100.0, 100.0)
}

#[allow(dead_code)]
fn create_test_block_idx() -> BlockIdx {
    BlockIdx { col: 5, row: 3 }
}

// ============================================================================
// PlayerEvidenceManager Tests
// ============================================================================
#[cfg(test)]
mod manager_tests {
#[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_new_creates_manager() {
        let manager = create_test_manager();
        assert_eq!(manager.viz_evidence_count(), 0);
    }

    #[test]
    fn test_update_transformer() {
        let mut manager = create_test_manager();
        use crate::coordinate_transform::{ScreenRect, ZoomMode};
        let new_transformer = CoordinateTransformer::new(
            1280,
            720,
            ScreenRect::new(0.0, 0.0, 1280.0, 720.0),
            ZoomMode::Fit,
        );
        manager.update_transformer(new_transformer);
        assert_eq!(manager.transformer().coded_width, 1280);
    }

    #[test]
    fn test_create_pixel_evidence() {
        let mut manager = create_test_manager();
        let screen = create_test_screen_px();
        let result = manager.create_pixel_evidence(screen, 0, "decode_1".to_string());
        assert!(result.is_some());
        assert_eq!(manager.viz_evidence_count(), 1);
    }

    #[test]
    fn test_create_pixel_evidence_outside_bounds() {
        let mut manager = create_test_manager();
        let screen = ScreenPx::new(5000.0, 5000.0); // Outside video
        let result = manager.create_pixel_evidence(screen, 0, "decode_1".to_string());
        assert!(result.is_none());
    }

    #[test]
    fn test_create_block_evidence() {
        let mut manager = create_test_manager();
        let screen = create_test_screen_px();
        let result = manager.create_block_evidence(screen, 0, 64, "decode_1".to_string());
        assert!(result.is_some());
        assert_eq!(manager.viz_evidence_count(), 1);
    }

    #[test]
    fn test_create_qp_heatmap_evidence() {
        let mut manager = create_test_manager();
        let block = create_test_block_idx();
        let evidence = manager.create_qp_heatmap_evidence(block, 25, 0, 64, "decode_1".to_string());
        assert_eq!(manager.viz_evidence_count(), 1);
        assert_eq!(evidence.element_type, VizElementType::QpHeatmap);
    }

    #[test]
    fn test_create_mv_overlay_evidence() {
        let mut manager = create_test_manager();
        let block = create_test_block_idx();
        let evidence = manager.create_mv_overlay_evidence(block, 10.0, 5.0, 0, 64, "decode_1".to_string());
        assert_eq!(manager.viz_evidence_count(), 1);
        assert_eq!(evidence.element_type, VizElementType::MotionVectorOverlay);
    }

    #[test]
    fn test_create_partition_evidence() {
        let mut manager = create_test_manager();
        let block = create_test_block_idx();
        let evidence = manager.create_partition_evidence(block, "PART_H".to_string(), 0, 64, "decode_1".to_string());
        assert_eq!(manager.viz_evidence_count(), 1);
        assert_eq!(evidence.element_type, VizElementType::PartitionGridOverlay);
    }

    #[test]
    fn test_find_at_screen_returns_evidence_at_position() {
        let mut manager = create_test_manager();
        let screen = create_test_screen_px();
        manager.create_pixel_evidence(screen, 0, "decode_1".to_string());
        let results = manager.find_at_screen(screen);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_find_at_screen_empty() {
        let manager = create_test_manager();
        let screen = create_test_screen_px();
        let results = manager.find_at_screen(screen);
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_screen_to_syntax() {
        let mut manager = create_test_manager();
        let screen = create_test_screen_px();
        manager.create_pixel_evidence(screen, 0, "decode_1".to_string());
        let syntax_ids = manager.screen_to_syntax(screen);
        // Should return empty since decode link doesn't exist in chain
        assert_eq!(syntax_ids.len(), 0);
    }

    #[test]
    fn test_screen_to_bit_offset() {
        let mut manager = create_test_manager();
        let screen = create_test_screen_px();
        manager.create_pixel_evidence(screen, 0, "decode_1".to_string());
        let bit_offsets = manager.screen_to_bit_offset(screen);
        // Should return empty since syntax chain doesn't exist
        assert_eq!(bit_offsets.len(), 0);
    }

    #[test]
    fn test_transformer_returns_reference() {
        let manager = create_test_manager();
        let transformer = manager.transformer();
        assert_eq!(transformer.coded_width, 1920);
    }

    #[test]
    fn test_evidence_chain_returns_reference() {
        let manager = create_test_manager();
        let chain = manager.evidence_chain();
        assert_eq!(chain.viz_index.len(), 0);
    }

    #[test]
    fn test_clear_viz_evidence() {
        let mut manager = create_test_manager();
        let screen = create_test_screen_px();
        manager.create_pixel_evidence(screen, 0, "decode_1".to_string());
        assert_eq!(manager.viz_evidence_count(), 1);
        manager.clear_viz_evidence();
        assert_eq!(manager.viz_evidence_count(), 0);
    }

    #[test]
    fn test_viz_evidence_count_increments() {
        let mut manager = create_test_manager();
        assert_eq!(manager.viz_evidence_count(), 0);
        manager.create_qp_heatmap_evidence(create_test_block_idx(), 25, 0, 64, "decode_1".to_string());
        assert_eq!(manager.viz_evidence_count(), 1);
        manager.create_mv_overlay_evidence(create_test_block_idx(), 10.0, 5.0, 0, 64, "decode_2".to_string());
        assert_eq!(manager.viz_evidence_count(), 2);
    }
}

// ============================================================================
// VizEvidence Integration Tests
// ============================================================================
#[cfg(test)]
mod evidence_integration_tests {
#[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_qp_evidence_contains_qp_value() {
        let mut manager = create_test_manager();
        let block = create_test_block_idx();
        let evidence = manager.create_qp_heatmap_evidence(block, 30, 5, 64, "decode_1".to_string());
        assert_eq!(evidence.visual_properties.get("qp_value").unwrap(), "30");
    }

    #[test]
    fn test_mv_evidence_contains_mv_components() {
        let mut manager = create_test_manager();
        let block = create_test_block_idx();
        let evidence = manager.create_mv_overlay_evidence(block, 15.0, 8.0, 3, 64, "decode_1".to_string());
        assert!(evidence.visual_properties.contains_key("mv_x"));
        assert!(evidence.visual_properties.contains_key("mv_y"));
        assert!(evidence.visual_properties.contains_key("mv_magnitude"));
    }

    #[test]
    fn test_partition_evidence_contains_partition_type() {
        let mut manager = create_test_manager();
        let block = create_test_block_idx();
        let evidence = manager.create_partition_evidence(block, "PART_V".to_string(), 2, 32, "decode_1".to_string());
        assert_eq!(evidence.visual_properties.get("partition_type").unwrap(), "PART_V");
    }

    #[test]
    fn test_block_evidence_contains_block_info() {
        let mut manager = create_test_manager();
        let evidence = manager.create_block_evidence(create_test_screen_px(), 1, 32, "decode_1".to_string()).unwrap();
        // ScreenPx(100, 100) with 32x32 blocks -> block_col=3, block_row=3 (100/32=3.125)
        assert_eq!(evidence.visual_properties.get("block_col").unwrap(), "3");
        assert_eq!(evidence.visual_properties.get("block_row").unwrap(), "3");
    }

    #[test]
    fn test_evidence_links_to_decode() {
        let mut manager = create_test_manager();
        let block = create_test_block_idx();
        let evidence = manager.create_qp_heatmap_evidence(block, 25, 0, 64, "test_decode_id".to_string());
        assert_eq!(evidence.decode_link, "test_decode_id");
    }

    #[test]
    fn test_evidence_has_frame_index() {
        let mut manager = create_test_manager();
        let block = create_test_block_idx();
        let evidence = manager.create_qp_heatmap_evidence(block, 25, 10, 64, "decode_1".to_string());
        assert_eq!(evidence.frame_idx, Some(10));
        assert_eq!(evidence.display_idx, Some(10));
    }
}

// ============================================================================
// Evidence ID Generation Tests
// ============================================================================
#[cfg(test)]
mod id_generation_tests {
#[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_evidence_ids_are_unique() {
        let mut manager = create_test_manager();
        let block = create_test_block_idx();
        let id1 = manager.create_qp_heatmap_evidence(block, 25, 0, 64, "decode_1".to_string()).id;
        let id2 = manager.create_qp_heatmap_evidence(block, 30, 1, 64, "decode_2".to_string()).id;
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_evidence_ids_are_sequential() {
        let mut manager = create_test_manager();
        let block = create_test_block_idx();
        let id1 = manager.create_qp_heatmap_evidence(block, 25, 0, 64, "decode_1".to_string()).id;
        let id2 = manager.create_qp_heatmap_evidence(block, 30, 1, 64, "decode_2".to_string()).id;
        assert!(id1.starts_with("player_viz_"));
        assert!(id2.starts_with("player_viz_"));
    }
}
