// Selection module tests
//
// Per generate-tests skill: Arrange-Act-Assert pattern with fixtures
// and edge case coverage.

use super::*;
use crate::{BitRange, StreamId, SyntaxNodeId, UnitKey};

// ============================================================================
// Fixtures
// ============================================================================

/// Create a test selection state
fn create_test_selection(stream_id: StreamId) -> SelectionState {
    SelectionState::new(stream_id)
}

/// Create a test spatial block
fn create_test_block() -> SpatialBlock {
    SpatialBlock {
        x: 10,
        y: 20,
        w: 32,
        h: 32,
    }
}

/// Create a test unit key
fn create_test_unit_key() -> UnitKey {
    UnitKey {
        stream: StreamId::A,
        unit_type: "OBU_FRAME".to_string(),
        offset: 1000,
        size: 500,
    }
}

/// Create a test syntax node ID
fn create_test_syntax_node_id() -> SyntaxNodeId {
    "test_node_1".to_string()
}

/// Create a test bit range
fn create_test_bit_range() -> BitRange {
    BitRange::new(1000, 1100)
}

// ============================================================================
// StreamId Tests
// ============================================================================

#[cfg(test)]
mod stream_id_tests {
    use super::*;

    #[test]
    fn test_stream_id_a_exists() {
        // Arrange & Act & Assert
        let _ = StreamId::A;
    }

    #[test]
    fn test_stream_id_b_exists() {
        // Arrange & Act & Assert
        let _ = StreamId::B;
    }
}

// ============================================================================
// FrameKey Tests
// ============================================================================

#[cfg(test)]
mod frame_key_tests {
    use super::*;

    #[test]
    fn test_frame_key_new_with_pts() {
        // Arrange
        let frame_key = FrameKey {
            stream: StreamId::A,
            frame_index: 10,
            pts: Some(100),
        };

        // Assert
        assert_eq!(frame_key.stream, StreamId::A);
        assert_eq!(frame_key.frame_index, 10);
        assert_eq!(frame_key.pts, Some(100));
    }

    #[test]
    fn test_frame_key_new_without_pts() {
        // Arrange
        let frame_key = FrameKey {
            stream: StreamId::B,
            frame_index: 20,
            pts: None,
        };

        // Assert
        assert_eq!(frame_key.stream, StreamId::B);
        assert_eq!(frame_key.frame_index, 20);
        assert_eq!(frame_key.pts, None);
    }
}

// ============================================================================
// UnitKey Tests
// ============================================================================

#[cfg(test)]
mod unit_key_tests {
    use super::*;

    #[test]
    fn test_unit_key_new() {
        // Arrange
        let unit_key = UnitKey {
            stream: StreamId::A,
            unit_type: "OBU_FRAME".to_string(),
            offset: 1000,
            size: 500,
        };

        // Assert
        assert_eq!(unit_key.stream, StreamId::A);
        assert_eq!(unit_key.unit_type, "OBU_FRAME");
        assert_eq!(unit_key.offset, 1000);
        assert_eq!(unit_key.size, 500);
    }
}

// ============================================================================
// SpatialBlock Tests
// ============================================================================

#[cfg(test)]
mod spatial_block_tests {
    use super::*;

    #[test]
    fn test_spatial_block_new() {
        // Arrange
        let block = SpatialBlock {
            x: 10,
            y: 20,
            w: 32,
            h: 32,
        };

        // Assert
        assert_eq!(block.x, 10);
        assert_eq!(block.y, 20);
        assert_eq!(block.w, 32);
        assert_eq!(block.h, 32);
    }

    #[test]
    fn test_spatial_block_area() {
        // Arrange
        let block = SpatialBlock {
            x: 0,
            y: 0,
            w: 16,
            h: 16,
        };

        // Act
        let area = block.w * block.h;

        // Assert
        assert_eq!(area, 256);
    }
}

// ============================================================================
// TemporalSelection Tests
// ============================================================================

#[cfg(test)]
mod temporal_selection_tests {
    use super::*;

    #[test]
    fn test_temporal_selection_block_precedence() {
        // Arrange
        let selection = TemporalSelection::Block {
            frame_index: 10,
            block: create_test_block(),
        };

        // Act
        let precedence = selection.precedence();

        // Assert - Block has highest precedence (4)
        assert_eq!(precedence, 4);
    }

    #[test]
    fn test_temporal_selection_point_precedence() {
        // Arrange
        let selection = TemporalSelection::Point { frame_index: 10 };

        // Act
        let precedence = selection.precedence();

        // Assert - Point has precedence 3
        assert_eq!(precedence, 3);
    }

    #[test]
    fn test_temporal_selection_range_precedence() {
        // Arrange
        let selection = TemporalSelection::Range {
            start: 5,
            end: 10,
        };

        // Act
        let precedence = selection.precedence();

        // Assert - Range has precedence 2
        assert_eq!(precedence, 2);
    }

    #[test]
    fn test_temporal_selection_marker_precedence() {
        // Arrange
        let selection = TemporalSelection::Marker { frame_index: 10 };

        // Act
        let precedence = selection.precedence();

        // Assert - Marker has lowest precedence (1)
        assert_eq!(precedence, 1);
    }

    #[test]
    fn test_temporal_selection_precedence_order() {
        // Arrange
        let block = TemporalSelection::Block {
            frame_index: 0,
            block: SpatialBlock { x: 0, y: 0, w: 1, h: 1 },
        };
        let point = TemporalSelection::Point { frame_index: 0 };
        let range = TemporalSelection::Range { start: 0, end: 1 };
        let marker = TemporalSelection::Marker { frame_index: 0 };

        // Act
        let block_prec = block.precedence();
        let point_prec = point.precedence();
        let range_prec = range.precedence();
        let marker_prec = marker.precedence();

        // Assert - Block > Point > Range > Marker
        assert!(block_prec > point_prec);
        assert!(point_prec > range_prec);
        assert!(range_prec > marker_prec);
    }

    #[test]
    fn test_temporal_selection_frame_index_block() {
        // Arrange
        let selection = TemporalSelection::Block {
            frame_index: 42,
            block: create_test_block(),
        };

        // Act
        let frame_index = selection.frame_index();

        // Assert
        assert_eq!(frame_index, 42);
    }

    #[test]
    fn test_temporal_selection_frame_index_point() {
        // Arrange
        let selection = TemporalSelection::Point { frame_index: 99 };

        // Act
        let frame_index = selection.frame_index();

        // Assert
        assert_eq!(frame_index, 99);
    }

    #[test]
    fn test_temporal_selection_frame_index_range() {
        // Arrange
        let selection = TemporalSelection::Range {
            start: 10,
            end: 20,
        };

        // Act
        let frame_index = selection.frame_index();

        // Assert - Returns start of range
        assert_eq!(frame_index, 10);
    }

    #[test]
    fn test_temporal_selection_spatial_block_block() {
        // Arrange
        let block = SpatialBlock {
            x: 5,
            y: 10,
            w: 16,
            h: 16,
        };
        let selection = TemporalSelection::Block {
            frame_index: 0,
            block,
        };

        // Act
        let spatial = selection.spatial_block();

        // Assert
        assert!(spatial.is_some());
        let sb = spatial.unwrap();
        assert_eq!(sb.x, 5);
        assert_eq!(sb.y, 10);
        assert_eq!(sb.w, 16);
        assert_eq!(sb.h, 16);
    }

    #[test]
    fn test_temporal_selection_spatial_block_point() {
        // Arrange
        let selection = TemporalSelection::Point { frame_index: 10 };

        // Act
        let spatial = selection.spatial_block();

        // Assert
        assert!(spatial.is_none());
    }

    #[test]
    fn test_temporal_selection_is_range() {
        // Arrange
        let selection = TemporalSelection::Range {
            start: 0,
            end: 10,
        };

        // Act
        let is_range = selection.is_range();

        // Assert
        assert!(is_range);
    }

    #[test]
    fn test_temporal_selection_is_range_for_block() {
        // Arrange
        let selection = TemporalSelection::Block {
            frame_index: 0,
            block: create_test_block(),
        };

        // Act
        let is_range = selection.is_range();

        // Assert
        assert!(!is_range);
    }

    #[test]
    fn test_temporal_selection_range_bounds() {
        // Arrange
        let selection = TemporalSelection::Range {
            start: 5,
            end: 15,
        };

        // Act
        let bounds = selection.range_bounds();

        // Assert
        assert_eq!(bounds, Some((5, 15)));
    }

    #[test]
    fn test_temporal_selection_range_bounds_for_non_range() {
        // Arrange
        let selection = TemporalSelection::Point { frame_index: 10 };

        // Act
        let bounds = selection.range_bounds();

        // Assert
        assert!(bounds.is_none());
    }
}

// ============================================================================
// DerivedCursor Tests
// ============================================================================

#[cfg(test)]
mod derived_cursor_tests {
    use super::*;

    #[test]
    fn test_derived_cursor_from_selection_point() {
        // Arrange
        let selection = TemporalSelection::Point { frame_index: 25 };

        // Act
        let cursor = DerivedCursor::from_selection(&selection);

        // Assert
        assert_eq!(cursor.frame_index, 25);
        assert!(cursor.spatial_pos.is_none());
    }

    #[test]
    fn test_derived_cursor_from_selection_block() {
        // Arrange
        let selection = TemporalSelection::Block {
            frame_index: 30,
            block: SpatialBlock {
                x: 10,
                y: 20,
                w: 32,
                h: 32,
            },
        };

        // Act
        let cursor = DerivedCursor::from_selection(&selection);

        // Assert
        assert_eq!(cursor.frame_index, 30);
        assert_eq!(cursor.spatial_pos, Some((10, 20)));
    }
}

// ============================================================================
// SelectionState Tests
// ============================================================================

#[cfg(test)]
mod selection_state_tests {
    use super::*;

    #[test]
    fn test_selection_state_default() {
        // Arrange & Act
        let state = SelectionState::default();

        // Assert
        assert_eq!(state.stream_id, StreamId::A);
        assert!(state.temporal.is_none());
        assert!(state.cursor.is_none());
        assert!(state.unit.is_none());
        assert!(state.syntax_node.is_none());
        assert!(state.bit_range.is_none());
        assert!(state.source_view.is_none());
    }

    #[test]
    fn test_selection_state_new_stream_a() {
        // Arrange & Act
        let state = SelectionState::new(StreamId::A);

        // Assert
        assert_eq!(state.stream_id, StreamId::A);
    }

    #[test]
    fn test_selection_state_new_stream_b() {
        // Arrange & Act
        let state = SelectionState::new(StreamId::B);

        // Assert
        assert_eq!(state.stream_id, StreamId::B);
    }

    #[test]
    fn test_selection_state_select_block() {
        // Arrange
        let mut state = create_test_selection(StreamId::A);
        let block = create_test_block();

        // Act
        state.select_block(10, block);

        // Assert
        assert!(state.temporal.is_some());
        match state.temporal {
            Some(TemporalSelection::Block { frame_index, block: b }) => {
                assert_eq!(frame_index, 10);
                assert_eq!(b.x, 10);
                assert_eq!(b.y, 20);
            }
            _ => panic!("Expected Block selection"),
        }
        assert!(state.cursor.is_some());
        assert_eq!(state.cursor.as_ref().unwrap().frame_index, 10);
        assert_eq!(state.cursor.as_ref().unwrap().spatial_pos, Some((10, 20)));
    }

    #[test]
    fn test_selection_state_select_point() {
        // Arrange
        let mut state = create_test_selection(StreamId::A);

        // Act
        state.select_point(15);

        // Assert
        assert!(state.temporal.is_some());
        match state.temporal {
            Some(TemporalSelection::Point { frame_index }) => {
                assert_eq!(frame_index, 15);
            }
            _ => panic!("Expected Point selection"),
        }
        assert!(state.cursor.is_some());
        assert_eq!(state.cursor.as_ref().unwrap().frame_index, 15);
        assert!(state.cursor.as_ref().unwrap().spatial_pos.is_none());
    }

    #[test]
    fn test_selection_state_select_range() {
        // Arrange
        let mut state = create_test_selection(StreamId::A);

        // Act
        state.select_range(5, 10);

        // Assert
        assert!(state.temporal.is_some());
        match state.temporal {
            Some(TemporalSelection::Range { start, end }) => {
                assert_eq!(start, 5);
                assert_eq!(end, 10);
            }
            _ => panic!("Expected Range selection"),
        }
        assert!(state.cursor.is_some());
        assert_eq!(state.cursor.as_ref().unwrap().frame_index, 5);
    }

    #[test]
    fn test_selection_state_select_marker() {
        // Arrange
        let mut state = create_test_selection(StreamId::A);
        // First create a point selection so select_marker will create a Marker
        state.select_point(10);

        // Act
        state.select_marker(20);

        // Assert - When there's an existing Point, select_marker creates a Marker
        assert!(state.temporal.is_some());
        match state.temporal {
            Some(TemporalSelection::Marker { frame_index }) => {
                assert_eq!(frame_index, 20);
            }
            _ => panic!("Expected Marker selection after existing Point"),
        }
    }

    #[test]
    fn test_selection_state_select_marker_on_fresh_state_creates_point() {
        // Arrange
        let mut state = create_test_selection(StreamId::A);

        // Act - select_marker on fresh state creates Point (not Marker)
        state.select_marker(20);

        // Assert - On fresh state, select_marker creates a Point
        assert!(state.temporal.is_some());
        match state.temporal {
            Some(TemporalSelection::Point { frame_index }) => {
                assert_eq!(frame_index, 20);
            }
            _ => panic!("Expected Point selection on fresh state"),
        }
    }

    #[test]
    fn test_selection_state_select_marker_clears_range() {
        // Arrange
        let mut state = create_test_selection(StreamId::A);
        state.select_range(5, 10);

        // Act
        state.select_marker(20);

        // Assert - Should clear range and create point
        match state.temporal {
            Some(TemporalSelection::Point { frame_index }) => {
                assert_eq!(frame_index, 20);
            }
            _ => panic!("Expected Point selection after marker with existing range"),
        }
    }

    #[test]
    fn test_selection_state_clear_temporal() {
        // Arrange
        let mut state = create_test_selection(StreamId::A);
        state.select_point(10);

        // Act
        state.clear_temporal();

        // Assert
        assert!(state.temporal.is_none());
        assert!(state.cursor.is_none());
    }

    #[test]
    fn test_selection_state_select_unit() {
        // Arrange
        let mut state = create_test_selection(StreamId::A);
        let unit_key = create_test_unit_key();

        // Act
        state.select_unit(unit_key.clone());

        // Assert
        assert_eq!(state.unit, Some(unit_key));
        assert!(state.syntax_node.is_none()); // Should clear syntax_node
        assert!(state.bit_range.is_none()); // Should clear bit_range
    }

    #[test]
    fn test_selection_state_select_syntax() {
        // Arrange
        let mut state = create_test_selection(StreamId::A);
        let node_id = create_test_syntax_node_id();
        let bit_range = create_test_bit_range();

        // Act
        state.select_syntax(node_id.clone(), bit_range);

        // Assert
        assert_eq!(state.syntax_node, Some(node_id));
        assert!(state.bit_range.is_some());
    }

    #[test]
    fn test_selection_state_select_bit_range() {
        // Arrange
        let mut state = create_test_selection(StreamId::A);
        let bit_range = create_test_bit_range();

        // Act
        state.select_bit_range(bit_range);

        // Assert
        assert!(state.bit_range.is_some());
    }

    #[test]
    fn test_selection_state_select_spatial_block() {
        // Arrange
        let mut state = create_test_selection(StreamId::A);
        let block = create_test_block();

        // Act
        state.select_spatial_block(10, block);

        // Assert
        assert!(state.temporal.is_some());
        match state.temporal {
            Some(TemporalSelection::Block { frame_index, .. }) => {
                assert_eq!(frame_index, 10);
            }
            _ => panic!("Expected Block selection"),
        }
    }

    #[test]
    fn test_selection_state_current_frame() {
        // Arrange
        let mut state = create_test_selection(StreamId::A);
        state.select_point(25);

        // Act
        let frame = state.current_frame();

        // Assert
        assert_eq!(frame, Some(25));
    }

    #[test]
    fn test_selection_state_current_frame_none() {
        // Arrange
        let state = create_test_selection(StreamId::A);

        // Act
        let frame = state.current_frame();

        // Assert
        assert!(frame.is_none());
    }

    // ========================================================================
    // Resize Handle Tests
    // ========================================================================

    #[test]
    fn test_selection_state_resize_block_bottom() {
        // Arrange
        let mut state = create_test_selection(StreamId::A);
        let block = SpatialBlock {
            x: 10,
            y: 10,
            w: 32,
            h: 32,
        };
        state.select_block(5, block);

        // Act
        state.resize_block_bottom(64);

        // Assert
        match state.temporal {
            Some(TemporalSelection::Block { block: b, .. }) => {
                assert_eq!(b.h, 64);
                assert_eq!(b.y, 10); // y unchanged
            }
            _ => panic!("Expected Block selection"),
        }
    }

    #[test]
    fn test_selection_state_resize_block_top() {
        // Arrange
        let mut state = create_test_selection(StreamId::A);
        let block = SpatialBlock {
            x: 10,
            y: 10,
            w: 32,
            h: 32,
        };
        state.select_block(5, block);

        // Act
        state.resize_block_top(20, 48); // y=20, h=48 maintains bottom at y=42

        // Assert
        match state.temporal {
            Some(TemporalSelection::Block { block: b, .. }) => {
                assert_eq!(b.y, 20);
                assert_eq!(b.h, 48);
            }
            _ => panic!("Expected Block selection"),
        }
    }

    #[test]
    fn test_selection_state_resize_block_left() {
        // Arrange
        let mut state = create_test_selection(StreamId::A);
        let block = SpatialBlock {
            x: 10,
            y: 10,
            w: 32,
            h: 32,
        };
        state.select_block(5, block);

        // Act
        state.resize_block_left(5, 48); // x=5, w=48 maintains right at x=42

        // Assert
        match state.temporal {
            Some(TemporalSelection::Block { block: b, .. }) => {
                assert_eq!(b.x, 5);
                assert_eq!(b.w, 48);
            }
            _ => panic!("Expected Block selection"),
        }
    }

    #[test]
    fn test_selection_state_resize_block_right() {
        // Arrange
        let mut state = create_test_selection(StreamId::A);
        let block = SpatialBlock {
            x: 10,
            y: 10,
            w: 32,
            h: 32,
        };
        state.select_block(5, block);

        // Act
        state.resize_block_right(64);

        // Assert
        match state.temporal {
            Some(TemporalSelection::Block { block: b, .. }) => {
                assert_eq!(b.w, 64);
                assert_eq!(b.x, 10); // x unchanged
            }
            _ => panic!("Expected Block selection"),
        }
    }

    #[test]
    fn test_selection_state_resize_block_top_left() {
        // Arrange
        let mut state = create_test_selection(StreamId::A);
        let block = SpatialBlock {
            x: 10,
            y: 10,
            w: 32,
            h: 32,
        };
        state.select_block(5, block);

        // Act
        state.resize_block_top_left(5, 5, 48, 48);

        // Assert
        match state.temporal {
            Some(TemporalSelection::Block { block: b, .. }) => {
                assert_eq!(b.x, 5);
                assert_eq!(b.y, 5);
                assert_eq!(b.w, 48);
                assert_eq!(b.h, 48);
            }
            _ => panic!("Expected Block selection"),
        }
    }

    #[test]
    fn test_selection_state_resize_block_top_right() {
        // Arrange
        let mut state = create_test_selection(StreamId::A);
        let block = SpatialBlock {
            x: 10,
            y: 10,
            w: 32,
            h: 32,
        };
        state.select_block(5, block);

        // Act
        state.resize_block_top_right(5, 64, 48);

        // Assert
        match state.temporal {
            Some(TemporalSelection::Block { block: b, .. }) => {
                assert_eq!(b.x, 10); // x unchanged
                assert_eq!(b.y, 5);
                assert_eq!(b.w, 64);
                assert_eq!(b.h, 48);
            }
            _ => panic!("Expected Block selection"),
        }
    }

    #[test]
    fn test_selection_state_resize_block_bottom_left() {
        // Arrange
        let mut state = create_test_selection(StreamId::A);
        let block = SpatialBlock {
            x: 10,
            y: 10,
            w: 32,
            h: 32,
        };
        state.select_block(5, block);

        // Act
        state.resize_block_bottom_left(5, 64, 48);

        // Assert
        match state.temporal {
            Some(TemporalSelection::Block { block: b, .. }) => {
                assert_eq!(b.x, 5);
                assert_eq!(b.y, 10); // y unchanged
                assert_eq!(b.w, 64);
                assert_eq!(b.h, 48);
            }
            _ => panic!("Expected Block selection"),
        }
    }

    #[test]
    fn test_selection_state_resize_block_bottom_right() {
        // Arrange
        let mut state = create_test_selection(StreamId::A);
        let block = SpatialBlock {
            x: 10,
            y: 10,
            w: 32,
            h: 32,
        }
        .clone();
        state.select_block(5, block);

        // Act
        state.resize_block_bottom_right(64, 64);

        // Assert
        match state.temporal {
            Some(TemporalSelection::Block { block: b, .. }) => {
                assert_eq!(b.x, 10); // x unchanged
                assert_eq!(b.y, 10); // y unchanged
                assert_eq!(b.w, 64);
                assert_eq!(b.h, 64);
            }
            _ => panic!("Expected Block selection"),
        }
    }

    #[test]
    fn test_selection_state_resize_without_block() {
        // Arrange
        let mut state = create_test_selection(StreamId::A);
        state.select_point(5); // Not a block selection

        // Act - Should not panic
        state.resize_block_bottom(64);

        // Assert - Point selection unchanged
        match state.temporal {
            Some(TemporalSelection::Point { frame_index }) => {
                assert_eq!(frame_index, 5);
            }
            _ => panic!("Expected Point selection"),
        }
    }
}

// ============================================================================
// SelectionAction Tests
// ============================================================================

#[cfg(test)]
mod selection_action_tests {
    use super::*;

    #[test]
    fn test_selection_action_select_block() {
        // Arrange
        let action = SelectionAction::SelectBlock {
            frame_index: 10,
            block: create_test_block(),
        };

        // Act
        let mut state = create_test_selection(StreamId::A);
        SelectionReducer::reduce_mut(&mut state, action);

        // Assert
        assert!(state.temporal.is_some());
    }

    #[test]
    fn test_selection_action_select_point() {
        // Arrange
        let action = SelectionAction::SelectPoint { frame_index: 15 };

        // Act
        let mut state = create_test_selection(StreamId::A);
        SelectionReducer::reduce_mut(&mut state, action);

        // Assert
        assert!(state.temporal.is_some());
    }

    #[test]
    fn test_selection_action_select_range() {
        // Arrange
        let action = SelectionAction::SelectRange {
            start: 5,
            end: 10,
        };

        // Act
        let mut state = create_test_selection(StreamId::A);
        SelectionReducer::reduce_mut(&mut state, action);

        // Assert
        assert!(state.temporal.is_some());
    }

    #[test]
    fn test_selection_action_clear_temporal() {
        // Arrange
        let action = SelectionAction::ClearTemporal;
        let mut state = create_test_selection(StreamId::A);
        state.select_point(10);

        // Act
        SelectionReducer::reduce_mut(&mut state, action);

        // Assert
        assert!(state.temporal.is_none());
    }

    #[test]
    fn test_selection_action_select_unit() {
        // Arrange
        let unit_key = create_test_unit_key();
        let action = SelectionAction::SelectUnit {
            unit: unit_key.clone(),
        };

        // Act
        let mut state = create_test_selection(StreamId::A);
        SelectionReducer::reduce_mut(&mut state, action);

        // Assert
        assert_eq!(state.unit, Some(unit_key));
    }

    #[test]
    fn test_selection_action_select_syntax() {
        // Arrange
        let node_id = create_test_syntax_node_id();
        let bit_range = create_test_bit_range();
        let action = SelectionAction::SelectSyntax {
            node_id: node_id.clone(),
            bit_range,
        };

        // Act
        let mut state = create_test_selection(StreamId::A);
        SelectionReducer::reduce_mut(&mut state, action);

        // Assert
        assert_eq!(state.syntax_node, Some(node_id));
    }

    #[test]
    fn test_selection_action_select_bit_range() {
        // Arrange
        let bit_range = create_test_bit_range();
        let action = SelectionAction::SelectBitRange { bit_range };

        // Act
        let mut state = create_test_selection(StreamId::A);
        SelectionReducer::reduce_mut(&mut state, action);

        // Assert
        assert!(state.bit_range.is_some());
    }

    #[test]
    fn test_selection_action_clear_all() {
        // Arrange
        let action = SelectionAction::ClearAll;
        let mut state = create_test_selection(StreamId::B);
        state.select_point(10);
        state.select_unit(create_test_unit_key());

        // Act
        SelectionReducer::reduce_mut(&mut state, action);

        // Assert
        assert!(state.temporal.is_none());
        assert!(state.unit.is_none());
        assert_eq!(state.stream_id, StreamId::B); // Stream ID preserved
    }
}

// ============================================================================
// SelectionReducer Tests
// ============================================================================

#[cfg(test)]
mod selection_reducer_tests {
    use super::*;

    #[test]
    fn test_selection_reducer_reduce_returns_new_state() {
        // Arrange
        let state = create_test_selection(StreamId::A);
        let action = SelectionAction::SelectPoint { frame_index: 20 };

        // Act
        let new_state = SelectionReducer::reduce(state, action);

        // Assert
        assert!(new_state.temporal.is_some());
    }

    #[test]
    fn test_selection_reducer_reduce_mut_updates_in_place() {
        // Arrange
        let mut state = create_test_selection(StreamId::A);
        let action = SelectionAction::SelectPoint { frame_index: 30 };

        // Act
        SelectionReducer::reduce_mut(&mut state, action);

        // Assert
        assert!(state.temporal.is_some());
    }

    #[test]
    fn test_selection_reducer_preserves_stream_id() {
        // Arrange
        let state = SelectionState::new(StreamId::B);
        let action = SelectionAction::ClearAll;

        // Act
        let new_state = SelectionReducer::reduce(state, action);

        // Assert
        assert_eq!(new_state.stream_id, StreamId::B);
    }

    #[test]
    fn test_selection_reducer_unit_clears_syntax() {
        // Arrange
        let mut state = create_test_selection(StreamId::A);
        state.select_syntax(create_test_syntax_node_id(), create_test_bit_range());

        // Act - Select unit should clear syntax
        let action = SelectionAction::SelectUnit {
            unit: create_test_unit_key(),
        };
        SelectionReducer::reduce_mut(&mut state, action);

        // Assert
        assert!(state.unit.is_some());
        assert!(state.syntax_node.is_none()); // Cleared
        assert!(state.bit_range.is_none()); // Cleared
    }
}

// ============================================================================
// Edge Cases
// ============================================================================

#[cfg(test)]
mod edge_case_tests {
    use super::*;

    #[test]
    fn test_selection_zero_frame_index() {
        // Arrange
        let mut state = create_test_selection(StreamId::A);

        // Act
        state.select_point(0);

        // Assert
        assert_eq!(state.current_frame(), Some(0));
    }

    #[test]
    fn test_selection_large_frame_index() {
        // Arrange
        let mut state = create_test_selection(StreamId::A);
        let large_index = usize::MAX - 1000;

        // Act
        state.select_point(large_index);

        // Assert
        assert_eq!(state.current_frame(), Some(large_index));
    }

    #[test]
    fn test_range_with_zero_length() {
        // Arrange
        let mut state = create_test_selection(StreamId::A);

        // Act
        state.select_range(10, 10);

        // Assert
        match state.temporal {
            Some(TemporalSelection::Range { start, end }) => {
                assert_eq!(start, 10);
                assert_eq!(end, 10);
            }
            _ => panic!("Expected Range selection"),
        }
    }

    #[test]
    fn test_multiple_selections_last_wins() {
        // Arrange
        let mut state = create_test_selection(StreamId::A);

        // Act
        state.select_point(10);
        state.select_point(20);
        state.select_point(30);

        // Assert
        assert_eq!(state.current_frame(), Some(30));
    }

    #[test]
    fn test_block_then_point_clears_block() {
        // Arrange
        let mut state = create_test_selection(StreamId::A);
        state.select_block(10, create_test_block());

        // Act - Point selection should replace block
        state.select_point(20);

        // Assert
        match state.temporal {
            Some(TemporalSelection::Point { frame_index }) => {
                assert_eq!(frame_index, 20);
            }
            _ => panic!("Expected Point selection"),
        }
    }

    #[test]
    fn test_range_then_marker_clears_range() {
        // Arrange
        let mut state = create_test_selection(StreamId::A);
        state.select_range(5, 10);

        // Act - Marker should clear range and create point
        state.select_marker(15);

        // Assert
        match state.temporal {
            Some(TemporalSelection::Point { frame_index }) => {
                assert_eq!(frame_index, 15);
            }
            _ => panic!("Expected Point selection"),
        }
    }
}
