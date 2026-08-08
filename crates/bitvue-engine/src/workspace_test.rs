// Workspace module tests
//
// Per generate-tests skill: Arrange-Act-Assert pattern with fixtures
// and edge case coverage.

use super::*;
use crate::{SelectionState, SpatialBlock, StreamId, SyncMode, TemporalSelection, UnitKey};

// ============================================================================
// Fixtures
// ============================================================================

/// Create a test workspace state
fn create_test_workspace() -> WorkspaceState {
    WorkspaceState::new()
}

/// Create a test selection state
fn create_test_selection(stream_id: StreamId) -> SelectionState {
    SelectionState::new(stream_id)
}

/// Create a test unit key
fn create_test_unit_key(stream: StreamId) -> UnitKey {
    UnitKey {
        stream,
        unit_type: "OBU_FRAME".to_string(),
        offset: 1000,
        size: 500,
    }
}

/// Create a test spatial block
fn create_test_spatial_block() -> SpatialBlock {
    SpatialBlock {
        x: 10,
        y: 20,
        w: 32,
        h: 32,
    }
}

// ============================================================================
// WorkspaceMode Tests
// ============================================================================

#[cfg(test)]
mod workspace_mode_tests {
    use super::*;

    #[test]
    fn test_workspace_mode_default() {
        // Arrange & Act
        let mode = WorkspaceMode::default();

        // Assert
        assert_eq!(mode, WorkspaceMode::Single);
    }

    #[test]
    fn test_workspace_mode_single_value() {
        // Arrange & Act
        let mode = WorkspaceMode::Single;

        // Assert
        assert!(matches!(mode, WorkspaceMode::Single));
    }

    #[test]
    fn test_workspace_mode_dual_value() {
        // Arrange & Act
        let mode = WorkspaceMode::Dual;

        // Assert
        assert!(matches!(mode, WorkspaceMode::Dual));
    }

    #[test]
    fn test_workspace_mode_equality() {
        // Arrange
        let mode1 = WorkspaceMode::Single;
        let mode2 = WorkspaceMode::Single;
        let mode3 = WorkspaceMode::Dual;

        // Assert
        assert_eq!(mode1, mode2);
        assert_ne!(mode1, mode3);
    }
}

// ============================================================================
// WorkspaceState Tests
// ============================================================================

#[cfg(test)]
mod workspace_state_tests {
    use super::*;

    #[test]
    fn test_workspace_state_new() {
        // Arrange & Act
        let state = WorkspaceState::new();

        // Assert
        assert_eq!(state.mode, WorkspaceMode::Single);
        assert_eq!(state.sync_mode, SyncMode::Off);
        assert_eq!(state.active_stream, StreamId::A);
    }

    #[test]
    fn test_workspace_state_default() {
        // Arrange & Act
        let state = WorkspaceState::default();

        // Assert
        assert_eq!(state.mode, WorkspaceMode::Single);
        assert_eq!(state.sync_mode, SyncMode::Off);
        assert_eq!(state.active_stream, StreamId::A);
    }

    #[test]
    fn test_workspace_state_set_mode_to_single() {
        // Arrange
        let mut state = create_test_workspace();
        state.sync_mode = SyncMode::Full; // Start with sync enabled

        // Act
        state.set_mode(WorkspaceMode::Single);

        // Assert
        assert_eq!(state.mode, WorkspaceMode::Single);
        // Sync mode should be reset when switching to Single
        assert_eq!(state.sync_mode, SyncMode::Off);
    }

    #[test]
    fn test_workspace_state_set_mode_to_dual() {
        // Arrange
        let mut state = create_test_workspace();

        // Act
        state.set_mode(WorkspaceMode::Dual);

        // Assert
        assert_eq!(state.mode, WorkspaceMode::Dual);
        // Sync mode should remain Off when switching to Dual
        assert_eq!(state.sync_mode, SyncMode::Off);
    }

    #[test]
    fn test_workspace_state_set_sync_mode_in_dual() {
        // Arrange
        let mut state = create_test_workspace();
        state.set_mode(WorkspaceMode::Dual);

        // Act
        state.set_sync_mode(SyncMode::Full);

        // Assert
        assert_eq!(state.sync_mode, SyncMode::Full);
    }

    #[test]
    fn test_workspace_state_set_sync_mode_in_single() {
        // Arrange
        let mut state = create_test_workspace();
        // State is in Single mode

        // Act
        state.set_sync_mode(SyncMode::Full);

        // Assert - Sync mode should NOT change in Single mode
        assert_eq!(state.sync_mode, SyncMode::Off);
    }

    #[test]
    fn test_workspace_state_set_active_stream_to_a() {
        // Arrange
        let mut state = create_test_workspace();
        state.active_stream = StreamId::B;

        // Act
        state.set_active_stream(StreamId::A);

        // Assert
        assert_eq!(state.active_stream, StreamId::A);
    }

    #[test]
    fn test_workspace_state_set_active_stream_to_b() {
        // Arrange
        let mut state = create_test_workspace();

        // Act
        state.set_active_stream(StreamId::B);

        // Assert
        assert_eq!(state.active_stream, StreamId::B);
    }

    #[test]
    fn test_workspace_state_is_dual_false_for_single() {
        // Arrange
        let state = WorkspaceState {
            mode: WorkspaceMode::Single,
            sync_mode: SyncMode::Off,
            active_stream: StreamId::A,
        };

        // Act
        let result = state.is_dual();

        // Assert
        assert!(!result);
    }

    #[test]
    fn test_workspace_state_is_dual_true_for_dual() {
        // Arrange
        let state = WorkspaceState {
            mode: WorkspaceMode::Dual,
            sync_mode: SyncMode::Off,
            active_stream: StreamId::A,
        };

        // Act
        let result = state.is_dual();

        // Assert
        assert!(result);
    }

    #[test]
    fn test_workspace_state_is_synced_false_in_single_mode() {
        // Arrange
        let state = WorkspaceState {
            mode: WorkspaceMode::Single,
            sync_mode: SyncMode::Full,
            active_stream: StreamId::A,
        };

        // Act
        let result = state.is_synced();

        // Assert
        assert!(!result); // Not synced because not in Dual mode
    }

    #[test]
    fn test_workspace_state_is_synced_false_in_dual_mode_off() {
        // Arrange
        let state = WorkspaceState {
            mode: WorkspaceMode::Dual,
            sync_mode: SyncMode::Off,
            active_stream: StreamId::A,
        };

        // Act
        let result = state.is_synced();

        // Assert
        assert!(!result);
    }

    #[test]
    fn test_workspace_state_is_synced_true_in_dual_mode_playhead() {
        // Arrange
        let state = WorkspaceState {
            mode: WorkspaceMode::Dual,
            sync_mode: SyncMode::Playhead,
            active_stream: StreamId::A,
        };

        // Act
        let result = state.is_synced();

        // Assert
        assert!(result);
    }

    #[test]
    fn test_workspace_state_is_synced_true_in_dual_mode_full() {
        // Arrange
        let state = WorkspaceState {
            mode: WorkspaceMode::Dual,
            sync_mode: SyncMode::Full,
            active_stream: StreamId::A,
        };

        // Act
        let result = state.is_synced();

        // Assert
        assert!(result);
    }
}

// ============================================================================
// SyncController Tests
// ============================================================================

#[cfg(test)]
mod sync_controller_tests {
    use super::*;

    #[test]
    fn test_sync_selection_off_no_changes() {
        // Arrange
        let mut source = create_test_selection(StreamId::A);
        source.select_point(10);
        let mut target = create_test_selection(StreamId::B);

        // Act
        SyncController::sync_selection(SyncMode::Off, &source, &mut target);

        // Assert - Target should remain unchanged (no selection)
        assert!(target.temporal.is_none());
    }

    #[test]
    fn test_sync_selection_playhead_syncs_point() {
        // Arrange
        let mut source = create_test_selection(StreamId::A);
        source.select_point(10);
        let mut target = create_test_selection(StreamId::B);

        // Act
        SyncController::sync_selection(SyncMode::Playhead, &source, &mut target);

        // Assert
        match target.temporal {
            Some(TemporalSelection::Point { frame_index }) => {
                assert_eq!(frame_index, 10);
            }
            _ => panic!("Expected Point selection"),
        }
        // Unit, syntax, bit_range should NOT be synced in Playhead mode
        assert!(target.unit.is_none());
        assert!(target.syntax_node.is_none());
        assert!(target.bit_range.is_none());
    }

    #[test]
    fn test_sync_selection_playhead_syncs_block() {
        // Arrange
        let mut source = create_test_selection(StreamId::A);
        let block = create_test_spatial_block();
        source.select_block(10, block);
        let mut target = create_test_selection(StreamId::B);

        // Act
        SyncController::sync_selection(SyncMode::Playhead, &source, &mut target);

        // Assert
        match target.temporal {
            Some(TemporalSelection::Block { frame_index, block }) => {
                assert_eq!(frame_index, 10);
                assert_eq!(block.x, 10);
                assert_eq!(block.y, 20);
                assert_eq!(block.w, 32);
                assert_eq!(block.h, 32);
            }
            _ => panic!("Expected Block selection"),
        }
    }

    #[test]
    fn test_sync_selection_playhead_syncs_range() {
        // Arrange
        let mut source = create_test_selection(StreamId::A);
        source.select_range(5, 10);
        let mut target = create_test_selection(StreamId::B);

        // Act
        SyncController::sync_selection(SyncMode::Playhead, &source, &mut target);

        // Assert
        match target.temporal {
            Some(TemporalSelection::Range { start, end }) => {
                assert_eq!(start, 5);
                assert_eq!(end, 10);
            }
            _ => panic!("Expected Range selection"),
        }
    }

    #[test]
    fn test_sync_selection_playhead_syncs_marker() {
        // Arrange
        let mut source = create_test_selection(StreamId::A);
        source.select_point(5); // First create a point
        source.select_marker(10); // Then marker to get Marker selection
        let mut target = create_test_selection(StreamId::B);
        // Pre-create a point in target so select_marker will create a Marker
        target.select_point(0);

        // Act
        SyncController::sync_selection(SyncMode::Playhead, &source, &mut target);

        // Assert - When target has an existing Point, select_marker creates Marker
        match target.temporal {
            Some(TemporalSelection::Marker { frame_index }) => {
                assert_eq!(frame_index, 10);
            }
            _ => panic!("Expected Marker selection after existing Point in target"),
        }
    }

    #[test]
    fn test_sync_selection_full_syncs_temporal() {
        // Arrange
        let mut source = create_test_selection(StreamId::A);
        source.select_point(10);
        let mut target = create_test_selection(StreamId::B);

        // Act
        SyncController::sync_selection(SyncMode::Full, &source, &mut target);

        // Assert - Temporal should be synced
        match target.temporal {
            Some(TemporalSelection::Point { frame_index }) => {
                assert_eq!(frame_index, 10);
            }
            _ => panic!("Expected Point selection"),
        }
    }

    #[test]
    fn test_sync_selection_full_syncs_unit() {
        // Arrange
        let mut source = create_test_selection(StreamId::A);
        let unit_key = create_test_unit_key(StreamId::A);
        source.select_unit(unit_key);
        let mut target = create_test_selection(StreamId::B);

        // Act
        SyncController::sync_selection(SyncMode::Full, &source, &mut target);

        // Assert - Unit should be synced with stream updated to B
        assert!(target.unit.is_some());
        let unit = target.unit.as_ref().unwrap();
        assert_eq!(unit.stream, StreamId::B); // Stream should be updated
        assert_eq!(unit.unit_type, "OBU_FRAME");
        assert_eq!(unit.offset, 1000);
        assert_eq!(unit.size, 500);
    }

    #[test]
    fn test_sync_selection_full_syncs_syntax_node() {
        // Arrange
        let mut source = create_test_selection(StreamId::A);
        source.select_syntax("test_node_1".to_string(), crate::BitRange::new(1000, 1100));
        let mut target = create_test_selection(StreamId::B);

        // Act
        SyncController::sync_selection(SyncMode::Full, &source, &mut target);

        // Assert - Syntax node should be synced (stream-agnostic)
        assert_eq!(target.syntax_node, Some("test_node_1".to_string()));
    }

    #[test]
    fn test_sync_selection_full_syncs_bit_range() {
        // Arrange
        let mut source = create_test_selection(StreamId::A);
        let bit_range = crate::BitRange::new(1000, 1100);
        source.select_bit_range(bit_range);
        let mut target = create_test_selection(StreamId::B);

        // Act
        SyncController::sync_selection(SyncMode::Full, &source, &mut target);

        // Assert - Bit range should be synced (stream-agnostic)
        assert_eq!(target.bit_range, Some(crate::BitRange::new(1000, 1100)));
    }

    #[test]
    fn test_sync_selection_full_syncs_all_selections() {
        // Arrange
        let mut source = create_test_selection(StreamId::A);
        source.select_point(10);
        let unit_key = create_test_unit_key(StreamId::A);
        source.select_unit(unit_key);
        source.select_syntax("test_node_1".to_string(), crate::BitRange::new(1000, 1100));
        let mut target = create_test_selection(StreamId::B);

        // Act
        SyncController::sync_selection(SyncMode::Full, &source, &mut target);

        // Assert - All selections should be synced
        match target.temporal {
            Some(TemporalSelection::Point { frame_index }) => {
                assert_eq!(frame_index, 10);
            }
            _ => panic!("Expected Point selection"),
        }
        assert!(target.unit.is_some());
        assert_eq!(target.unit.as_ref().unwrap().stream, StreamId::B);
        assert_eq!(target.syntax_node, Some("test_node_1".to_string()));
        assert_eq!(target.bit_range, Some(crate::BitRange::new(1000, 1100)));
    }
}

// ============================================================================
// Integration Tests
// ============================================================================

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_workspace_mode_switch_sequence() {
        // Arrange
        let mut state = create_test_workspace();

        // Act 1: Switch to Dual
        state.set_mode(WorkspaceMode::Dual);
        state.set_sync_mode(SyncMode::Full);

        // Assert 1
        assert!(state.is_dual());
        assert!(state.is_synced());

        // Act 2: Switch back to Single
        state.set_mode(WorkspaceMode::Single);

        // Assert 2 - Sync should be reset
        assert!(!state.is_dual());
        assert!(!state.is_synced());
        assert_eq!(state.sync_mode, SyncMode::Off);
    }

    #[test]
    fn test_sync_with_different_active_streams() {
        // Arrange
        let mut source = create_test_selection(StreamId::A);
        source.select_point(10);
        let mut target = create_test_selection(StreamId::B);
        let mut workspace = create_test_workspace();

        // Act - Sync from A to B
        workspace.active_stream = StreamId::A;
        SyncController::sync_selection(SyncMode::Full, &source, &mut target);

        // Assert
        match target.temporal {
            Some(TemporalSelection::Point { frame_index }) => {
                assert_eq!(frame_index, 10);
            }
            _ => panic!("Expected Point selection"),
        }
    }

    #[test]
    fn test_multiple_sync_operations() {
        // Arrange
        let mut source = create_test_selection(StreamId::A);
        let mut target = create_test_selection(StreamId::B);

        // Act 1: First sync
        source.select_point(10);
        SyncController::sync_selection(SyncMode::Full, &source, &mut target);
        let first_frame = match target.temporal {
            Some(TemporalSelection::Point { frame_index }) => frame_index,
            _ => panic!("Expected Point selection"),
        };

        // Act 2: Second sync with different frame
        source.select_point(20);
        SyncController::sync_selection(SyncMode::Full, &source, &mut target);
        let second_frame = match target.temporal {
            Some(TemporalSelection::Point { frame_index }) => frame_index,
            _ => panic!("Expected Point selection"),
        };

        // Assert
        assert_eq!(first_frame, 10);
        assert_eq!(second_frame, 20);
    }
}

// ============================================================================
// Edge Cases
// ============================================================================

#[cfg(test)]
mod edge_case_tests {
    use super::*;

    #[test]
    fn test_sync_with_no_source_selection() {
        // Arrange
        let source = create_test_selection(StreamId::A); // No selection
        let mut target = create_test_selection(StreamId::B);

        // Act
        SyncController::sync_selection(SyncMode::Full, &source, &mut target);

        // Assert - Target should remain unchanged
        assert!(target.temporal.is_none());
    }

    #[test]
    fn test_sync_preserves_target_stream_id() {
        // Arrange
        let mut source = create_test_selection(StreamId::A);
        let unit_key = create_test_unit_key(StreamId::A);
        source.select_unit(unit_key);
        let mut target = create_test_selection(StreamId::B);

        // Act
        SyncController::sync_selection(SyncMode::Full, &source, &mut target);

        // Assert - Target's stream_id should be preserved
        assert_eq!(target.stream_id, StreamId::B);
        // Unit should have updated stream
        assert_eq!(target.unit.as_ref().unwrap().stream, StreamId::B);
    }

    #[test]
    fn test_set_mode_single_resets_sync_regardless_of_current_mode() {
        // Arrange
        let mut state = WorkspaceState {
            mode: WorkspaceMode::Dual,
            sync_mode: SyncMode::Full,
            active_stream: StreamId::A,
        };

        // Act
        state.set_mode(WorkspaceMode::Single);

        // Assert
        assert_eq!(state.mode, WorkspaceMode::Single);
        assert_eq!(state.sync_mode, SyncMode::Off);
    }

    #[test]
    fn test_workspace_state_clone() {
        // Arrange
        let state = WorkspaceState {
            mode: WorkspaceMode::Dual,
            sync_mode: SyncMode::Full,
            active_stream: StreamId::B,
        };

        // Act
        let cloned = state.clone();

        // Assert
        assert_eq!(cloned.mode, WorkspaceMode::Dual);
        assert_eq!(cloned.sync_mode, SyncMode::Full);
        assert_eq!(cloned.active_stream, StreamId::B);
    }
}
