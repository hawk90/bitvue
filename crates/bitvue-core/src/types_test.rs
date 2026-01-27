// Types module tests
//
// Per generate-tests skill: Arrange-Act-Assert pattern with fixtures
// and edge case coverage.

use super::*;

// ============================================================================
// Fixtures
// ============================================================================

/// Create a test bit range
fn create_test_bit_range() -> BitRange {
    BitRange::new(1000, 1100)
}

/// Create a test syntax node ID
fn create_test_node_id() -> SyntaxNodeId {
    "test_node_1".to_string()
}

/// Create a test bitstream info
fn create_test_bitstream_info() -> BitstreamInfo {
    BitstreamInfo {
        codec: "AV1".to_string(),
        source: "test.ivf".to_string(),
        size: 1000000,
        sequence: None,
        frames: vec![],
    }
}

// ============================================================================
// BitRange Tests
// ============================================================================

#[cfg(test)]
mod bit_range_tests {
    use super::*;

    #[test]
    fn test_bit_range_new() {
        // Arrange & Act
        let range = BitRange::new(1000, 1100);

        // Assert
        assert_eq!(range.start_bit, 1000);
        assert_eq!(range.end_bit, 1100);
    }

    #[test]
    fn test_bit_range_contains() {
        // Arrange
        let range = BitRange::new(1000, 1100);

        // Act & Assert
        assert!(range.contains(1000)); // Start is inclusive
        assert!(range.contains(1050));
        assert!(!range.contains(999)); // Before range
        assert!(!range.contains(1100)); // End is exclusive
        assert!(!range.contains(1101)); // After range
    }

    #[test]
    fn test_bit_range_contains_range() {
        // Arrange
        let outer = BitRange::new(1000, 2000);
        let inner = BitRange::new(1200, 1500);
        let overlapping = BitRange::new(1500, 2500);
        let before = BitRange::new(500, 900);
        let after = BitRange::new(2100, 2500);

        // Assert
        assert!(outer.contains_range(&inner));
        assert!(!outer.contains_range(&overlapping));
        assert!(!outer.contains_range(&before));
        assert!(!outer.contains_range(&after));
    }

    #[test]
    fn test_bit_range_size_bits() {
        // Arrange
        let range = BitRange::new(1000, 1100);

        // Act
        let size = range.size_bits();

        // Assert
        assert_eq!(size, 100);
    }

    #[test]
    fn test_bit_range_byte_offset() {
        // Arrange
        let range = BitRange::new(8000, 9000);

        // Act
        let byte_offset = range.byte_offset();

        // Assert
        assert_eq!(byte_offset, 1000); // 8000 / 8
    }

    #[test]
    fn test_bit_range_zero_size() {
        // Arrange & Act
        let range = BitRange::new(1000, 1000);

        // Assert
        assert_eq!(range.size_bits(), 0);
    }

    #[test]
    fn test_bit_range_copy() {
        // Arrange
        let range1 = BitRange::new(1000, 1100);

        // Act
        let range2 = range1;

        // Assert - BitRange is Copy
        assert_eq!(range1.start_bit, 1000);
        assert_eq!(range2.start_bit, 1000);
    }

    #[test]
    fn test_bit_range_equality() {
        // Arrange
        let range1 = BitRange::new(1000, 1100);
        let range2 = BitRange::new(1000, 1100);
        let range3 = BitRange::new(1000, 1200);

        // Assert
        assert_eq!(range1, range2);
        assert_ne!(range1, range3);
    }
}

// ============================================================================
// SequenceInfo Tests
// ============================================================================

#[cfg(test)]
mod sequence_info_tests {
    use super::*;

    #[test]
    fn test_sequence_info_complete() {
        // Arrange & Act
        let info = SequenceInfo {
            profile: 1,
            level: 5,
            width: 1920,
            height: 1080,
            bit_depth: 10,
            chroma_subsampling: "4:2:0".to_string(),
        };

        // Assert
        assert_eq!(info.profile, 1);
        assert_eq!(info.level, 5);
        assert_eq!(info.width, 1920);
        assert_eq!(info.height, 1080);
        assert_eq!(info.bit_depth, 10);
        assert_eq!(info.chroma_subsampling, "4:2:0");
    }

    #[test]
    fn test_sequence_info_clone() {
        // Arrange
        let info = SequenceInfo {
            profile: 0,
            level: 0,
            width: 640,
            height: 480,
            bit_depth: 8,
            chroma_subsampling: "4:2:0".to_string(),
        };

        // Act
        let cloned = info.clone();

        // Assert
        assert_eq!(cloned.width, 640);
        assert_eq!(cloned.height, 480);
    }
}

// ============================================================================
// FrameInfo Tests
// ============================================================================

#[cfg(test)]
mod frame_info_tests {
    use super::*;

    #[test]
    fn test_frame_info_complete() {
        // Arrange & Act
        let info = FrameInfo {
            index: 10,
            frame_type: FrameType::Key,
            offset: 5000,
            size: 1000,
            pts: Some(100),
            dts: Some(90),
            show_frame: true,
            blocks: vec![],
        };

        // Assert
        assert_eq!(info.index, 10);
        assert_eq!(info.frame_type, FrameType::Key);
        assert_eq!(info.offset, 5000);
        assert_eq!(info.size, 1000);
        assert_eq!(info.pts, Some(100));
        assert_eq!(info.dts, Some(90));
        assert!(info.show_frame);
        assert!(info.blocks.is_empty());
    }

    #[test]
    fn test_frame_info_minimal() {
        // Arrange & Act
        let info = FrameInfo {
            index: 0,
            frame_type: FrameType::Inter,
            offset: 0,
            size: 500,
            pts: None,
            dts: None,
            show_frame: false,
            blocks: vec![],
        };

        // Assert
        assert_eq!(info.index, 0);
        assert_eq!(info.frame_type, FrameType::Inter);
        assert!(!info.show_frame);
        assert!(info.pts.is_none());
        assert!(info.dts.is_none());
    }
}

// ============================================================================
// FrameType Tests
// ============================================================================

#[cfg(test)]
mod frame_type_tests {
    use super::*;

    #[test]
    fn test_frame_type_display() {
        // Arrange & Act
        let key_display = format!("{}", FrameType::Key);
        let inter_display = format!("{}", FrameType::Inter);
        let b_frame_display = format!("{}", FrameType::BFrame);
        let intra_only_display = format!("{}", FrameType::IntraOnly);
        let switch_display = format!("{}", FrameType::Switch);

        // Assert
        assert_eq!(key_display, "KEY");
        assert_eq!(inter_display, "INTER");
        assert_eq!(b_frame_display, "B");
        assert_eq!(intra_only_display, "INTRA_ONLY");
        assert_eq!(switch_display, "SWITCH");
    }

    #[test]
    fn test_frame_type_copy() {
        // Arrange
        let frame_type = FrameType::Key;

        // Act
        let copied = frame_type;

        // Assert - FrameType is Copy
        assert_eq!(frame_type, FrameType::Key);
        assert_eq!(copied, FrameType::Key);
    }

    #[test]
    fn test_frame_type_equality() {
        // Arrange
        let key1 = FrameType::Key;
        let key2 = FrameType::Key;
        let inter = FrameType::Inter;

        // Assert
        assert_eq!(key1, key2);
        assert_ne!(key1, inter);
    }
}

// ============================================================================
// BlockInfo Tests
// ============================================================================

#[cfg(test)]
mod block_info_tests {
    use super::*;

    #[test]
    fn test_block_info_complete() {
        // Arrange & Act
        let block = BlockInfo {
            x: 0,
            y: 0,
            width: 32,
            height: 32,
            prediction_mode: PredictionMode::Intra(IntraMode::DC),
            qp: Some(30),
            bits: Some(100),
            motion_vector: None,
        };

        // Assert
        assert_eq!(block.x, 0);
        assert_eq!(block.y, 0);
        assert_eq!(block.width, 32);
        assert_eq!(block.height, 32);
        assert_eq!(block.qp, Some(30));
        assert_eq!(block.bits, Some(100));
        assert!(block.motion_vector.is_none());
    }

    #[test]
    fn test_block_info_with_motion_vector() {
        // Arrange & Act
        let mv = MotionVector {
            x: 10,
            y: -5,
            ref_frame: 0,
        };
        let block = BlockInfo {
            x: 64,
            y: 32,
            width: 16,
            height: 16,
            prediction_mode: PredictionMode::Inter,
            qp: None,
            bits: None,
            motion_vector: Some(mv),
        };

        // Assert
        assert!(block.motion_vector.is_some());
        let mv = block.motion_vector.unwrap();
        assert_eq!(mv.x, 10);
        assert_eq!(mv.y, -5);
        assert_eq!(mv.ref_frame, 0);
    }
}

// ============================================================================
// PredictionMode Tests
// ============================================================================

#[cfg(test)]
mod prediction_mode_tests {
    use super::*;

    #[test]
    fn test_prediction_mode_intra() {
        // Arrange & Act
        let mode = PredictionMode::Intra(IntraMode::DC);

        // Assert
        assert!(matches!(mode, PredictionMode::Intra(_)));
    }

    #[test]
    fn test_prediction_mode_inter() {
        // Arrange & Act
        let mode = PredictionMode::Inter;

        // Assert
        assert_eq!(mode, PredictionMode::Inter);
    }

    #[test]
    fn test_prediction_mode_skip() {
        // Arrange & Act
        let mode = PredictionMode::Skip;

        // Assert
        assert_eq!(mode, PredictionMode::Skip);
    }
}

// ============================================================================
// IntraMode Tests
// ============================================================================

#[cfg(test)]
mod intra_mode_tests {
    use super::*;

    #[test]
    fn test_intra_mode_values() {
        // Arrange & Act
        let modes = [
            IntraMode::DC,
            IntraMode::Vertical,
            IntraMode::Horizontal,
            IntraMode::Diagonal(45),
            IntraMode::Smooth,
            IntraMode::Paeth,
        ];

        // Assert - All modes exist
        assert_eq!(modes.len(), 6);
    }

    #[test]
    fn test_intra_mode_copy() {
        // Arrange
        let mode = IntraMode::DC;

        // Act
        let copied = mode;

        // Assert - IntraMode is Copy
        assert_eq!(mode, IntraMode::DC);
        assert_eq!(copied, IntraMode::DC);
    }
}

// ============================================================================
// MotionVector Tests
// ============================================================================

#[cfg(test)]
mod motion_vector_tests {
    use super::*;

    #[test]
    fn test_motion_vector_construct() {
        // Arrange & Act
        let mv = MotionVector {
            x: 100,
            y: -50,
            ref_frame: 1,
        };

        // Assert
        assert_eq!(mv.x, 100);
        assert_eq!(mv.y, -50);
        assert_eq!(mv.ref_frame, 1);
    }

    #[test]
    fn test_motion_vector_copy() {
        // Arrange
        let mv1 = MotionVector {
            x: 10,
            y: 20,
            ref_frame: 0,
        };

        // Act
        let mv2 = mv1;

        // Assert - MotionVector is Copy
        assert_eq!(mv1.x, 10);
        assert_eq!(mv2.x, 10);
    }
}

// ============================================================================
// OverlayLayer Tests
// ============================================================================

#[cfg(test)]
mod overlay_layer_tests {
    use super::*;

    #[test]
    fn test_overlay_layer_all() {
        // Arrange & Act
        let layers = OverlayLayer::all();

        // Assert
        assert!(!layers.is_empty());
        assert!(layers.len() >= 12);
    }

    #[test]
    fn test_overlay_layer_name() {
        // Arrange
        let layers = OverlayLayer::all();

        // Act & Assert - All layers should have names
        for layer in layers {
            let name = layer.name();
            assert!(!name.is_empty());
        }
    }

    #[test]
    fn test_overlay_layer_specific_names() {
        // Assert specific layer names
        assert_eq!(OverlayLayer::Grid.name(), "Grid");
        assert_eq!(OverlayLayer::BlockPartition.name(), "Block Partition");
        assert_eq!(OverlayLayer::PredictionMode.name(), "Prediction Mode");
        assert_eq!(OverlayLayer::QpMap.name(), "QP Map");
        assert_eq!(OverlayLayer::MotionVectors.name(), "Motion Vectors");
    }
}

// ============================================================================
// SyntaxNode Tests
// ============================================================================

#[cfg(test)]
mod syntax_node_tests {
    use super::*;

    #[test]
    fn test_syntax_node_new() {
        // Arrange
        let node_id = "test_node".to_string();
        let bit_range = BitRange::new(0, 100);
        let field_name = "test_field".to_string();

        // Act
        let node = SyntaxNode::new(
            node_id.clone(),
            bit_range,
            field_name.clone(),
            Some("value".to_string()),
            None,
            0,
        );

        // Assert
        assert_eq!(node.node_id, node_id);
        assert_eq!(node.bit_range.start_bit, 0);
        assert_eq!(node.bit_range.end_bit, 100);
        assert_eq!(node.field_name, field_name);
        assert_eq!(node.value, Some("value".to_string()));
        assert!(node.parent.is_none());
        assert_eq!(node.depth, 0);
        assert!(node.children.is_empty());
    }

    #[test]
    fn test_syntax_node_add_child() {
        // Arrange
        let mut parent = SyntaxNode::new(
            "parent".to_string(),
            BitRange::new(0, 100),
            "parent".to_string(),
            None,
            None,
            0,
        );
        let child_id = "child".to_string();

        // Act
        parent.add_child(child_id.clone());

        // Assert
        assert_eq!(parent.children.len(), 1);
        assert_eq!(parent.children[0], child_id);
    }
}

// ============================================================================
// SyntaxModel Tests
// ============================================================================

#[cfg(test)]
mod syntax_model_tests {
    use super::*;

    #[test]
    fn test_syntax_model_new() {
        // Arrange
        let root_id = "root".to_string();
        let unit_key = "test_unit".to_string();

        // Act
        let model = SyntaxModel::new(root_id.clone(), unit_key);

        // Assert
        assert!(model.nodes.is_empty());
        assert_eq!(model.root_id, root_id);
        assert_eq!(model.unit_key, "test_unit");
    }

    #[test]
    fn test_syntax_model_add_node() {
        // Arrange
        let mut model = SyntaxModel::new("root".to_string(), "unit".to_string());
        let node = SyntaxNode::new(
            "node1".to_string(),
            BitRange::new(0, 100),
            "field".to_string(),
            None,
            None,
            0,
        );

        // Act
        model.add_node(node);

        // Assert
        assert_eq!(model.nodes.len(), 1);
        assert!(model.nodes.contains_key("node1"));
    }

    #[test]
    fn test_syntax_model_get_node() {
        // Arrange
        let mut model = SyntaxModel::new("root".to_string(), "unit".to_string());
        let node = SyntaxNode::new(
            "node1".to_string(),
            BitRange::new(0, 100),
            "field".to_string(),
            None,
            None,
            0,
        );
        model.add_node(node);

        // Act
        let retrieved = model.get_node("node1");

        // Assert
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().node_id, "node1");
    }

    #[test]
    fn test_syntax_model_find_nearest_node() {
        // Arrange
        let mut model = SyntaxModel::new("root".to_string(), "unit".to_string());

        // Add root node
        let root = SyntaxNode::new(
            "root".to_string(),
            BitRange::new(0, 1000),
            "root".to_string(),
            None,
            None,
            0,
        );
        model.add_node(root);

        // Add child node
        let child = SyntaxNode::new(
            "child".to_string(),
            BitRange::new(100, 200),
            "child".to_string(),
            None,
            Some("root".to_string()),
            1,
        );
        model.add_node(child);

        // Act - Find node containing a bit range
        let target = BitRange::new(120, 130);
        let found = model.find_nearest_node(&target);

        // Assert - Should find the child node
        assert!(found.is_some());
        assert_eq!(found.unwrap().node_id, "child");
    }
}

// ============================================================================
// BitstreamInfo Tests
// ============================================================================

#[cfg(test)]
mod bitstream_info_tests {
    use super::*;

    #[test]
    fn test_bitstream_info_complete() {
        // Arrange
        let sequence = SequenceInfo {
            profile: 1,
            level: 5,
            width: 1920,
            height: 1080,
            bit_depth: 10,
            chroma_subsampling: "4:2:0".to_string(),
        };
        let frames = vec![
            FrameInfo {
                index: 0,
                frame_type: FrameType::Key,
                offset: 0,
                size: 1000,
                pts: Some(0),
                dts: Some(0),
                show_frame: true,
                blocks: vec![],
            },
        ];

        // Act
        let info = BitstreamInfo {
            codec: "AV1".to_string(),
            source: "test.ivf".to_string(),
            size: 50000,
            sequence: Some(sequence),
            frames,
        };

        // Assert
        assert_eq!(info.codec, "AV1");
        assert!(info.sequence.is_some());
        assert_eq!(info.frames.len(), 1);
        assert_eq!(info.frames[0].index, 0);
    }
}
