// Spatial Hierarchy module tests

// ============================================================================
// Fixtures
// ============================================================================
#[allow(dead_code)]
fn create_test_rect() -> CodedRect {
    CodedRect::new(10, 20, 100, 80)
}

#[allow(dead_code)]
fn create_test_block_evidence(id: &str) -> BlockEvidence {
    BlockEvidence::new(
        id.to_string(),
        0,
        create_test_rect(),
        PredictionMode::IntraDc,
        "ctu_1".to_string(),
        "syntax_1".to_string(),
    )
}

#[allow(dead_code)]
fn create_test_ctu_evidence(id: &str) -> CtuEvidence {
    CtuEvidence::new(
        id.to_string(),
        0,
        create_test_rect(),
        "tile_1".to_string(),
        "syntax_1".to_string(),
    )
}

#[allow(dead_code)]
fn create_test_tile_evidence(id: &str) -> TileEvidence {
    TileEvidence::new(
        id.to_string(),
        0,
        0,
        0,
        create_test_rect(),
        "frame_1".to_string(),
        "syntax_1".to_string(),
    )
}

#[allow(dead_code)]
fn create_test_frame_hierarchy(id: &str, display_idx: u64) -> FrameSpatialHierarchy {
    FrameSpatialHierarchy::new(
        id.to_string(),
        display_idx,
        display_idx,
        1920,
        1080,
        "decode_1".to_string(),
    )
}

#[allow(dead_code)]
fn create_test_index() -> SpatialHierarchyIndex {
    SpatialHierarchyIndex::new()
}

// ============================================================================
// CodedRect Tests
// ============================================================================
#[cfg(test)]
mod coded_rect_tests {
#[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_new_creates_rect() {
        let rect = CodedRect::new(10, 20, 100, 80);
        assert_eq!(rect.x, 10);
        assert_eq!(rect.y, 20);
        assert_eq!(rect.width, 100);
        assert_eq!(rect.height, 80);
    }

    #[test]
    fn test_contains_point_inside() {
        let rect = create_test_rect();
        assert!(rect.contains(15, 25));
    }

    #[test]
    fn test_contains_point_outside() {
        let rect = create_test_rect();
        assert!(!rect.contains(5, 25));
    }

    #[test]
    fn test_contains_point_on_boundary() {
        let rect = CodedRect::new(10, 20, 100, 80);
        assert!(rect.contains(10, 20));
    }

    #[test]
    fn test_area_calculates() {
        let rect = create_test_rect();
        assert_eq!(rect.area(), 8000);
    }

    #[test]
    fn test_center_calculates() {
        let rect = create_test_rect();
        assert_eq!(rect.center(), (60, 60));
    }
}

// ============================================================================
// PredictionMode Tests
// ============================================================================
#[cfg(test)]
mod prediction_mode_tests {
#[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_is_intra_for_intra_modes() {
        assert!(PredictionMode::IntraDc.is_intra());
        assert!(PredictionMode::IntraHorizontal.is_intra());
        assert!(PredictionMode::IntraVertical.is_intra());
    }

    #[test]
    fn test_is_intra_false_for_inter_modes() {
        assert!(!PredictionMode::InterSingle.is_intra());
        assert!(!PredictionMode::InterCompound.is_intra());
    }

    #[test]
    fn test_is_inter_for_inter_modes() {
        assert!(PredictionMode::InterSingle.is_inter());
        assert!(PredictionMode::InterCompound.is_inter());
    }

    #[test]
    fn test_is_inter_false_for_intra_modes() {
        assert!(!PredictionMode::IntraDc.is_inter());
        assert!(!PredictionMode::IntraVertical.is_inter());
    }

    #[test]
    fn test_is_skip_for_skip_modes() {
        assert!(PredictionMode::Skip.is_skip());
        assert!(PredictionMode::Copy.is_skip());
    }

    #[test]
    fn test_display_name() {
        assert_eq!(PredictionMode::IntraDc.display_name(), "INTRA_DC");
        assert_eq!(PredictionMode::InterSingle.display_name(), "INTER");
        assert_eq!(PredictionMode::Skip.display_name(), "SKIP");
    }
}

// ============================================================================
// MotionVector Tests
// ============================================================================
#[cfg(test)]
mod motion_vector_tests {
#[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_new_creates_mv() {
        let mv = MotionVector::new(10, -5, 0);
        assert_eq!(mv.x, 10);
        assert_eq!(mv.y, -5);
        assert_eq!(mv.ref_frame, 0);
    }

    #[test]
    fn test_to_pixels_divides_by_four() {
        let mv = MotionVector::new(8, 4, 0);
        let (px, py) = mv.to_pixels();
        assert!((px - 2.0).abs() < 0.01);
        assert!((py - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_magnitude_calculates() {
        let mv = MotionVector::new(3, 4, 0);
        assert!((mv.magnitude() - 5.0).abs() < 0.01);
    }

    #[test]
    fn test_magnitude_pixels() {
        let mv = MotionVector::new(12, 5, 0);
        let mag = mv.magnitude_pixels();
        assert!((mag - 3.25).abs() < 0.01);
    }

    #[test]
    fn test_angle_calculates() {
        let mv = MotionVector::new(0, 10, 0);
        let angle = mv.angle();
        assert!((angle - std::f32::consts::PI / 2.0).abs() < 0.01);
    }
}

// ============================================================================
// BlockEvidence Tests
// ============================================================================
#[cfg(test)]
mod block_evidence_tests {
#[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_new_creates_block() {
        let block = create_test_block_evidence("block_1");
        assert_eq!(block.id, "block_1");
        assert_eq!(block.block_idx, 0);
        assert_eq!(block.mode, PredictionMode::IntraDc);
    }

    #[test]
    fn test_contains_point() {
        let block = create_test_block_evidence("block_1");
        assert!(block.contains_point(50, 50));
    }

    #[test]
    fn test_contains_point_outside() {
        let block = create_test_block_evidence("block_1");
        assert!(!block.contains_point(5, 5));
    }

    #[test]
    fn test_default_qp_is_zero() {
        let block = create_test_block_evidence("block_1");
        assert_eq!(block.qp, 0);
    }

    #[test]
    fn test_default_mv_is_none() {
        let block = create_test_block_evidence("block_1");
        assert!(block.mv_l0.is_none());
        assert!(block.mv_l1.is_none());
    }
}

// ============================================================================
// CtuEvidence Tests
// ============================================================================
#[cfg(test)]
mod ctu_evidence_tests {
#[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_new_creates_ctu() {
        let ctu = create_test_ctu_evidence("ctu_1");
        assert_eq!(ctu.id, "ctu_1");
        assert_eq!(ctu.ctu_idx, 0);
        assert_eq!(ctu.blocks.len(), 0);
    }

    #[test]
    fn test_add_block_increases_count() {
        let mut ctu = create_test_ctu_evidence("ctu_1");
        ctu.add_block(create_test_block_evidence("block_1"));
        assert_eq!(ctu.blocks.len(), 1);
    }

    #[test]
    fn test_recompute_stats_updates_statistics() {
        let mut ctu = create_test_ctu_evidence("ctu_1");
        let mut block = create_test_block_evidence("block_1");
        block.mode = PredictionMode::IntraDc;
        ctu.add_block(block);
        assert_eq!(ctu.stats.intra_block_count, 1);
    }

    #[test]
    fn test_find_block_at() {
        let mut ctu = create_test_ctu_evidence("ctu_1");
        ctu.add_block(create_test_block_evidence("block_1"));
        let result = ctu.find_block_at(50, 50);
        assert!(result.is_some());
    }

    #[test]
    fn test_find_block_at_not_found() {
        let ctu = create_test_ctu_evidence("ctu_1");
        let result = ctu.find_block_at(5, 5);
        assert!(result.is_none());
    }

    #[test]
    fn test_contains_point() {
        let ctu = create_test_ctu_evidence("ctu_1");
        assert!(ctu.contains_point(50, 50));
    }
}

// ============================================================================
// TileEvidence Tests
// ============================================================================
#[cfg(test)]
mod tile_evidence_tests {
#[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_new_creates_tile() {
        let tile = create_test_tile_evidence("tile_1");
        assert_eq!(tile.id, "tile_1");
        assert_eq!(tile.tile_idx, 0);
        assert_eq!(tile.tile_row, 0);
        assert_eq!(tile.tile_col, 0);
    }

    #[test]
    fn test_add_ctu_increases_count() {
        let mut tile = create_test_tile_evidence("tile_1");
        tile.add_ctu(create_test_ctu_evidence("ctu_1"));
        assert_eq!(tile.ctu_count, 1);
    }

    #[test]
    fn test_recompute_stats_updates_statistics() {
        let mut tile = create_test_tile_evidence("tile_1");
        tile.add_ctu(create_test_ctu_evidence("ctu_1"));
        assert_eq!(tile.stats.block_count, 0); // Empty CTU
    }

    #[test]
    fn test_find_ctu_at() {
        let mut tile = create_test_tile_evidence("tile_1");
        tile.add_ctu(create_test_ctu_evidence("ctu_1"));
        let result = tile.find_ctu_at(50, 50);
        assert!(result.is_some());
    }

    #[test]
    fn test_find_block_at() {
        let mut tile = create_test_tile_evidence("tile_1");
        let mut ctu = create_test_ctu_evidence("ctu_1");
        ctu.add_block(create_test_block_evidence("block_1"));
        tile.add_ctu(ctu);
        let result = tile.find_block_at(50, 50);
        assert!(result.is_some());
    }

    #[test]
    fn test_find_block_at_not_found() {
        let tile = create_test_tile_evidence("tile_1");
        let result = tile.find_block_at(5, 5);
        assert!(result.is_none());
    }
}

// ============================================================================
// FrameSpatialHierarchy Tests
// ============================================================================
#[cfg(test)]
mod frame_hierarchy_tests {
#[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_new_creates_hierarchy() {
        let hierarchy = create_test_frame_hierarchy("frame_1", 100);
        assert_eq!(hierarchy.id, "frame_1");
        assert_eq!(hierarchy.display_idx, 100);
        assert_eq!(hierarchy.width, 1920);
        assert_eq!(hierarchy.height, 1080);
    }

    #[test]
    fn test_add_tile_increases_count() {
        let mut hierarchy = create_test_frame_hierarchy("frame_1", 100);
        hierarchy.add_tile(create_test_tile_evidence("tile_1"));
        assert_eq!(hierarchy.tiles.len(), 1);
    }

    #[test]
    fn test_find_tile_at() {
        let mut hierarchy = create_test_frame_hierarchy("frame_1", 100);
        hierarchy.add_tile(create_test_tile_evidence("tile_1"));
        let result = hierarchy.find_tile_at(50, 50);
        assert!(result.is_some());
    }

    #[test]
    fn test_find_ctu_at() {
        let mut hierarchy = create_test_frame_hierarchy("frame_1", 100);
        let mut tile = create_test_tile_evidence("tile_1");
        tile.add_ctu(create_test_ctu_evidence("ctu_1"));
        hierarchy.add_tile(tile);
        let result = hierarchy.find_ctu_at(50, 50);
        assert!(result.is_some());
    }

    #[test]
    fn test_find_block_at() {
        let mut hierarchy = create_test_frame_hierarchy("frame_1", 100);
        let mut tile = create_test_tile_evidence("tile_1");
        let mut ctu = create_test_ctu_evidence("ctu_1");
        ctu.add_block(create_test_block_evidence("block_1"));
        tile.add_ctu(ctu);
        hierarchy.add_tile(tile);
        let result = hierarchy.find_block_at(50, 50);
        assert!(result.is_some());
    }

    #[test]
    fn test_find_block_at_returns_spatial_hit() {
        let mut hierarchy = create_test_frame_hierarchy("frame_1", 100);
        let mut tile = create_test_tile_evidence("tile_1");
        let mut ctu = create_test_ctu_evidence("ctu_1");
        ctu.add_block(create_test_block_evidence("block_1"));
        tile.add_ctu(ctu);
        hierarchy.add_tile(tile);
        let result = hierarchy.find_block_at(50, 50);
        assert!(result.is_some());
        if let Some(hit) = result {
            assert_eq!(hit.block.id, "block_1");
        }
    }
}

// ============================================================================
// SpatialHierarchyIndex Tests
// ============================================================================
#[cfg(test)]
mod index_tests {
#[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_new_creates_empty_index() {
        let index = create_test_index();
        assert!(index.is_empty());
        assert_eq!(index.len(), 0);
    }

    #[test]
    fn test_add_increments_length() {
        let mut index = create_test_index();
        index.add(create_test_frame_hierarchy("frame_1", 100));
        assert_eq!(index.len(), 1);
    }

    #[test]
    fn test_add_sorts_by_display_idx() {
        let mut index = create_test_index();
        index.add(create_test_frame_hierarchy("frame_3", 300));
        index.add(create_test_frame_hierarchy("frame_1", 100));
        index.add(create_test_frame_hierarchy("frame_2", 200));
        assert_eq!(index.all()[0].display_idx, 100);
    }

    #[test]
    fn test_find_by_id() {
        let mut index = create_test_index();
        index.add(create_test_frame_hierarchy("test_id", 100));
        let result = index.find_by_id("test_id");
        assert!(result.is_some());
    }

    #[test]
    fn test_find_by_display_idx() {
        let mut index = create_test_index();
        index.add(create_test_frame_hierarchy("frame_1", 100));
        let result = index.find_by_display_idx(100);
        assert!(result.is_some());
    }

    #[test]
    fn test_find_by_decode_link() {
        let mut index = create_test_index();
        index.add(create_test_frame_hierarchy("frame_1", 100));
        let result = index.find_by_decode_link("decode_1");
        assert!(result.is_some());
    }

    #[test]
    fn test_all_returns_all() {
        let mut index = create_test_index();
        index.add(create_test_frame_hierarchy("frame_1", 100));
        index.add(create_test_frame_hierarchy("frame_2", 200));
        assert_eq!(index.all().len(), 2);
    }
}
