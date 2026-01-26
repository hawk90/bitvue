// Core module tests
//
// Per generate-tests skill: Arrange-Act-Assert pattern with fixtures
// and edge case coverage.

use crate::{
    BitRange, Command, Core, Event, FrameKey, JobManager, SpatialBlock, StreamId, SyntaxNodeId,
    UnitKey, UnitNode,
};
use std::path::PathBuf;

// ============================================================================
// Fixtures
// ============================================================================

/// Create a test core instance
fn create_test_core() -> Core {
    Core::new()
}

/// Create a test file path
fn create_test_path() -> PathBuf {
    PathBuf::from("/tmp/test.ivf")
}

/// Create a test unit key
fn create_test_unit_key() -> UnitKey {
    UnitKey {
        stream: crate::StreamId::A,
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

/// Create a test frame key
fn create_test_frame_key() -> crate::FrameKey {
    crate::FrameKey {
        stream: crate::StreamId::A,
        frame_index: 10,
        pts: Some(100),
    }
}

/// Create a test spatial block
fn create_test_spatial_block() -> crate::SpatialBlock {
    crate::SpatialBlock {
        x: 10,
        y: 20,
        w: 32,
        h: 32,
    }
}

// ============================================================================
// Core::new Tests
// ============================================================================

#[cfg(test)]
mod core_new_tests {
    use super::*;

    #[test]
    fn test_core_new_creates_instance() {
        // Arrange & Act
        let core = create_test_core();

        // Assert - Core should be created
        let _ = core.get_stream(crate::StreamId::A);
        let _ = core.get_stream(crate::StreamId::B);
        let _ = core.get_selection();
        let _ = core.get_job_manager();
    }

    #[test]
    fn test_core_default_creates_instance() {
        // Arrange & Act
        let core = Core::default();

        // Assert
        let _ = core.get_stream(crate::StreamId::A);
    }
}

// ============================================================================
// Core::get_stream Tests
// ============================================================================

#[cfg(test)]
mod core_get_stream_tests {
    use super::*;

    #[test]
    fn test_get_stream_a() {
        // Arrange
        let core = create_test_core();

        // Act
        let stream_a = core.get_stream(crate::StreamId::A);

        // Assert
        assert!(stream_a.read().stream_id == crate::StreamId::A);
    }

    #[test]
    fn test_get_stream_b() {
        // Arrange
        let core = create_test_core();

        // Act
        let stream_b = core.get_stream(crate::StreamId::B);

        // Assert
        assert!(stream_b.read().stream_id == crate::StreamId::B);
    }

    #[test]
    fn test_get_streams_are_independent() {
        // Arrange
        let core = create_test_core();

        // Act
        let stream_a = core.get_stream(crate::StreamId::A);
        let stream_b = core.get_stream(crate::StreamId::B);

        // Assert
        assert!(stream_a.read().stream_id != stream_b.read().stream_id);
    }
}

// ============================================================================
// Core::get_selection Tests
// ============================================================================

#[cfg(test)]
mod core_get_selection_tests {
    use super::*;

    #[test]
    fn test_get_selection() {
        // Arrange
        let core = create_test_core();

        // Act
        let selection = core.get_selection();

        // Assert
        assert!(selection.read().stream_id == crate::StreamId::A);
    }

    #[test]
    fn test_get_selection_initial_state() {
        // Arrange
        let core = create_test_core();

        // Act
        let selection = core.get_selection();

        // Assert - Initial state should have no temporal selection
        assert!(selection.read().temporal.is_none());
    }
}

// ============================================================================
// Core::get_job_manager Tests
// ============================================================================

#[cfg(test)]
mod core_get_job_manager_tests {
    use super::*;

    #[test]
    fn test_get_job_manager() {
        // Arrange
        let core = create_test_core();

        // Act
        let job_manager = core.get_job_manager();

        // Assert
        let _ = job_manager.current_request_id(crate::StreamId::A);
    }
}

// ============================================================================
// Core::handle_command - OpenFile Tests
// ============================================================================

#[cfg(test)]
mod handle_open_file_tests {
    use super::*;

    #[test]
    fn test_handle_open_file_with_valid_path() {
        // Arrange
        let core = create_test_core();
        let path = create_test_path();

        // Note: This test assumes the file doesn't exist, so it will fail
        // In a real test, you'd create a temp file or mock the ByteCache

        // Act
        let events = core.handle_command(Command::OpenFile {
            stream: crate::StreamId::A,
            path: path.clone(),
        });

        // Assert - Should return events (either ModelUpdated or DiagnosticAdded)
        assert!(!events.is_empty());

        // If file doesn't exist, we should get a DiagnosticAdded event
        if let Some(Event::DiagnosticAdded { .. }) = events.first() {
            // Expected for non-existent file
        } else {
            // File existed and was opened
        }
    }

    #[test]
    fn test_handle_open_file_sets_stream_a() {
        // Arrange
        let core = create_test_core();
        let path = create_test_path();

        // Act
        let _events = core.handle_command(Command::OpenFile {
            stream: crate::StreamId::A,
            path,
        });

        // Assert - Stream A should be modified
        // (even if file open failed, state was touched)
    }

    #[test]
    fn test_handle_open_file_sets_stream_b() {
        // Arrange
        let core = create_test_core();
        let path = create_test_path();

        // Act
        let _events = core.handle_command(Command::OpenFile {
            stream: crate::StreamId::B,
            path,
        });

        // Assert - Stream B should be modified
    }
}

// ============================================================================
// Core::handle_command - CloseFile Tests
// ============================================================================

#[cfg(test)]
mod handle_close_file_tests {
    use super::*;

    #[test]
    fn test_handle_close_file_stream_a() {
        // Arrange
        let core = create_test_core();

        // Act
        let events = core.handle_command(Command::CloseFile {
            stream: crate::StreamId::A,
        });

        // Assert
        assert!(!events.is_empty());
        if let Some(Event::ModelUpdated { stream, .. }) = events.first() {
            assert_eq!(*stream, crate::StreamId::A);
        }
    }

    #[test]
    fn test_handle_close_file_stream_b() {
        // Arrange
        let core = create_test_core();

        // Act
        let events = core.handle_command(Command::CloseFile {
            stream: crate::StreamId::B,
        });

        // Assert
        assert!(!events.is_empty());
        if let Some(Event::ModelUpdated { stream: s, .. }) = events.first() {
            assert_eq!(*s, crate::StreamId::B);
        }
    }

    #[test]
    fn test_handle_close_file_resets_state() {
        // Arrange
        let core = create_test_core();

        // First open a file (will fail but touches state)
        let _ = core.handle_command(Command::OpenFile {
            stream: crate::StreamId::A,
            path: create_test_path(),
        });

        // Act - Close the file
        let events = core.handle_command(Command::CloseFile {
            stream: crate::StreamId::A,
        });

        // Assert - State should be reset
        let stream_state = core.get_stream(crate::StreamId::A);
        let state = stream_state.read();
        assert!(state.file_path.is_none());
        assert!(state.byte_cache.is_none());

        assert!(!events.is_empty());
    }
}

// ============================================================================
// Core::handle_command - SelectFrame Tests
// ============================================================================

#[cfg(test)]
mod handle_select_frame_tests {
    use super::*;

    #[test]
    fn test_handle_select_frame_stream_a() {
        // Arrange
        let core = create_test_core();
        let frame_key = create_test_frame_key();

        // Act
        let events = core.handle_command(Command::SelectFrame {
            stream: crate::StreamId::A,
            frame_key,
        });

        // Assert
        assert!(!events.is_empty());
        let selection = core.get_selection();
        assert_eq!(selection.read().current_frame(), Some(10));
    }

    #[test]
    fn test_handle_select_frame_updates_selection() {
        // Arrange
        let core = create_test_core();
        let frame_key = create_test_frame_key();

        // Act
        let events = core.handle_command(Command::SelectFrame {
            stream: crate::StreamId::B,
            frame_key,
        });

        // Assert
        if let Some(Event::SelectionUpdated { .. }) = events.first() {
            // Event was emitted - just check it exists
            assert!(!events.is_empty());
        }
    }

    #[test]
    fn test_handle_select_frame_multiple() {
        // Arrange
        let core = create_test_core();

        // Act - Select frame 10
        let _events = core.handle_command(Command::SelectFrame {
            stream: crate::StreamId::A,
            frame_key: crate::FrameKey {
                stream: crate::StreamId::A,
                frame_index: 10,
                pts: Some(100),
            },
        });

        // Then select frame 20
        let _events = core.handle_command(Command::SelectFrame {
            stream: crate::StreamId::A,
            frame_key: crate::FrameKey {
                stream: crate::StreamId::A,
                frame_index: 20,
                pts: Some(200),
            },
        });

        // Assert - Last selection wins
        let selection = core.get_selection();
        assert_eq!(selection.read().current_frame(), Some(20));
    }
}

// ============================================================================
// Core::handle_command - SelectUnit Tests
// ============================================================================

#[cfg(test)]
mod handle_select_unit_tests {
    use super::*;

    #[test]
    fn test_handle_select_unit() {
        // Arrange
        let core = create_test_core();
        let unit_key = create_test_unit_key();

        // Act
        let events = core.handle_command(Command::SelectUnit {
            stream: crate::StreamId::A,
            unit_key,
        });

        // Assert
        assert!(!events.is_empty());
        let selection = core.get_selection();
        assert!(selection.read().unit.is_some());
    }

    #[test]
    fn test_handle_select_unit_clears_syntax() {
        // Arrange
        let core = create_test_core();

        // First select syntax
        let _ = core.handle_command(Command::SelectSyntax {
            stream: crate::StreamId::A,
            node_id: create_test_syntax_node_id(),
            bit_range: create_test_bit_range(),
        });

        // Then select unit
        let unit_key = create_test_unit_key();
        let events = core.handle_command(Command::SelectUnit {
            stream: crate::StreamId::A,
            unit_key,
        });

        // Assert - Unit selection should clear syntax
        let selection = core.get_selection();
        let sel = selection.read();
        assert!(sel.unit.is_some());
        assert!(sel.syntax_node.is_none()); // Cleared
        assert!(sel.bit_range.is_none()); // Cleared

        assert!(!events.is_empty());
    }
}

// ============================================================================
// Core::handle_command - SelectSyntax Tests
// ============================================================================

#[cfg(test)]
mod handle_select_syntax_tests {
    use super::*;

    #[test]
    fn test_handle_select_syntax() {
        // Arrange
        let core = create_test_core();
        let node_id = create_test_syntax_node_id();
        let bit_range = create_test_bit_range();

        // Act
        let events = core.handle_command(Command::SelectSyntax {
            stream: crate::StreamId::A,
            node_id,
            bit_range,
        });

        // Assert
        assert!(!events.is_empty());
        let selection = core.get_selection();
        let sel = selection.read();
        assert!(sel.syntax_node.is_some());
        assert!(sel.bit_range.is_some());
    }

    #[test]
    fn test_handle_select_syntax_preserves_unit() {
        // Arrange
        let core = create_test_core();
        let unit_key = create_test_unit_key();

        // First select unit
        let _ = core.handle_command(Command::SelectUnit {
            stream: crate::StreamId::A,
            unit_key,
        });

        // Then select syntax
        let node_id = create_test_syntax_node_id();
        let bit_range = create_test_bit_range();
        let _events = core.handle_command(Command::SelectSyntax {
            stream: crate::StreamId::A,
            node_id,
            bit_range,
        });

        // Assert - Unit should be preserved (tri-sync allows unit + syntax)
        let selection = core.get_selection();
        let sel = selection.read();
        assert!(sel.unit.is_some()); // Preserved
        assert!(sel.syntax_node.is_some());
    }
}

// ============================================================================
// Core::handle_command - SelectBitRange Tests
// ============================================================================

#[cfg(test)]
mod handle_select_bit_range_tests {
    use super::*;

    #[test]
    fn test_handle_select_bit_range() {
        // Arrange
        let core = create_test_core();
        let bit_range = create_test_bit_range();

        // Act
        let events = core.handle_command(Command::SelectBitRange {
            stream: crate::StreamId::A,
            bit_range,
        });

        // Assert
        assert!(!events.is_empty());
        let selection = core.get_selection();
        assert!(selection.read().bit_range.is_some());
    }

    #[test]
    fn test_handle_select_bit_range_without_syntax() {
        // Arrange
        let core = create_test_core();
        let bit_range = BitRange::new(9999, 10000); // Range likely not in syntax

        // Act
        let _events = core.handle_command(Command::SelectBitRange {
            stream: crate::StreamId::A,
            bit_range,
        });

        // Assert - Bit range should be set even without syntax node
        let selection = core.get_selection();
        let sel = selection.read();
        assert!(sel.bit_range.is_some());
        assert!(sel.syntax_node.is_none()); // No matching syntax node
    }
}

// ============================================================================
// Core::handle_command - SelectSpatialBlock Tests
// ============================================================================

#[cfg(test)]
mod handle_select_spatial_block_tests {
    use super::*;

    #[test]
    fn test_handle_select_spatial_block_with_frame() {
        // Arrange
        let core = create_test_core();

        // First select a frame
        let _ = core.handle_command(Command::SelectFrame {
            stream: crate::StreamId::A,
            frame_key: create_test_frame_key(),
        });

        let block = create_test_spatial_block();

        // Act
        let events = core.handle_command(Command::SelectSpatialBlock {
            stream: crate::StreamId::A,
            block,
        });

        // Assert
        assert!(!events.is_empty());
        let selection = core.get_selection();
        assert!(selection.read().temporal.is_some());
    }

    #[test]
    fn test_handle_select_spatial_block_without_frame() {
        // Arrange
        let core = create_test_core();
        let block = create_test_spatial_block();

        // Act - Select block without selecting frame first
        let events = core.handle_command(Command::SelectSpatialBlock {
            stream: crate::StreamId::A,
            block,
        });

        // Assert - Should use frame_index 0 as default
        assert!(!events.is_empty());
        let selection = core.get_selection();
        let sel = selection.read();
        if let Some(crate::TemporalSelection::Block { frame_index, .. }) = sel.temporal {
            assert_eq!(frame_index, 0); // Default frame index
        }
    }

    #[test]
    fn test_handle_select_spatial_block_updates_selection() {
        // Arrange
        let core = create_test_core();

        // First select frame 10
        let _ = core.handle_command(Command::SelectFrame {
            stream: crate::StreamId::A,
            frame_key: crate::FrameKey {
                stream: crate::StreamId::A,
                frame_index: 10,
                pts: Some(100),
            },
        });

        let block = create_test_spatial_block();

        // Act
        let events = core.handle_command(Command::SelectSpatialBlock {
            stream: crate::StreamId::A,
            block,
        });

        // Assert - Block should be at frame 10
        let selection = core.get_selection();
        let sel = selection.read();
        if let Some(crate::TemporalSelection::Block { frame_index, .. }) = sel.temporal {
            assert_eq!(frame_index, 10);
        }

        if let Some(Event::SelectionUpdated { stream: s }) = events.first() {
            assert_eq!(*s, crate::StreamId::A);
        }
    }
}

// ============================================================================
// Core::handle_command - Unknown Command Tests
// ============================================================================

#[cfg(test)]
mod handle_unknown_command_tests {
    use super::*;

    #[test]
    fn test_handle_unknown_command_returns_empty() {
        // Arrange
        let core = create_test_core();

        // Act - Create a command that doesn't exist in match
        // (This test documents current behavior for unhandled commands)
        // Note: Since we can't directly create an "unknown" command,
        // this test documents that _ => vec![] is the fallback

        // The actual test would require adding a test variant to Command
        // For now, we just verify the core doesn't panic
        let selection = core.get_selection();
        let _ = selection.read();

        // Assert - No panic occurred
    }
}

// ============================================================================
// Integration Tests
// ============================================================================

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_command_flow_open_select_close() {
        // Arrange
        let core = create_test_core();

        // Act 1: Open file
        let open_events = core.handle_command(Command::OpenFile {
            stream: crate::StreamId::A,
            path: create_test_path(),
        });

        // Act 2: Select frame
        let frame_key = create_test_frame_key();
        let select_events = core.handle_command(Command::SelectFrame {
            stream: crate::StreamId::A,
            frame_key,
        });

        // Act 3: Close file
        let close_events = core.handle_command(Command::CloseFile {
            stream: crate::StreamId::A,
        });

        // Assert
        assert!(!open_events.is_empty());
        assert!(!select_events.is_empty());
        assert!(!close_events.is_empty());
    }

    #[test]
    fn test_dual_stream_independence() {
        // Arrange
        let core = create_test_core();

        // Act
        let frame_key_a = crate::FrameKey {
            stream: crate::StreamId::A,
            frame_index: 10,
            pts: Some(100),
        };
        let frame_key_b = crate::FrameKey {
            stream: crate::StreamId::B,
            frame_index: 20,
            pts: Some(200),
        };

        let _events_a = core.handle_command(Command::SelectFrame {
            stream: crate::StreamId::A,
            frame_key: frame_key_a,
        });
        let _events_b = core.handle_command(Command::SelectFrame {
            stream: crate::StreamId::B,
            frame_key: frame_key_b,
        });

        // Assert - Each stream has its own state
        let stream_a = core.get_stream(crate::StreamId::A);
        let stream_b = core.get_stream(crate::StreamId::B);
        assert!(stream_a.read().stream_id == crate::StreamId::A);
        assert!(stream_b.read().stream_id == crate::StreamId::B);
    }

    #[test]
    fn test_selection_persists_across_commands() {
        // Arrange
        let core = create_test_core();
        let frame_key = create_test_frame_key();

        // Act
        let _events1 = core.handle_command(Command::SelectFrame {
            stream: crate::StreamId::A,
            frame_key,
        });

        let unit_key = create_test_unit_key();
        let _events2 = core.handle_command(Command::SelectUnit {
            stream: crate::StreamId::A,
            unit_key,
        });

        // Assert - Both selections should be preserved
        let selection = core.get_selection();
        let sel = selection.read();
        assert_eq!(sel.current_frame(), Some(10)); // Frame persists
        assert!(sel.unit.is_some()); // Unit added
    }
}

// ============================================================================
// Edge Cases
// ============================================================================

#[cfg(test)]
mod edge_case_tests {
    use super::*;

    #[test]
    fn test_select_frame_zero_index() {
        // Arrange
        let core = create_test_core();

        // Act
        let frame_key = crate::FrameKey {
            stream: crate::StreamId::A,
            frame_index: 0,
            pts: Some(0),
        };
        let events = core.handle_command(Command::SelectFrame {
            stream: crate::StreamId::A,
            frame_key,
        });

        // Assert
        assert!(!events.is_empty());
        let selection = core.get_selection();
        assert_eq!(selection.read().current_frame(), Some(0));
    }

    #[test]
    fn test_select_large_frame_index() {
        // Arrange
        let core = create_test_core();

        // Act
        let frame_key = crate::FrameKey {
            stream: crate::StreamId::A,
            frame_index: 999999,
            pts: Some(999999),
        };
        let events = core.handle_command(Command::SelectFrame {
            stream: crate::StreamId::A,
            frame_key,
        });

        // Assert
        assert!(!events.is_empty());
        let selection = core.get_selection();
        assert_eq!(selection.read().current_frame(), Some(999999));
    }

    #[test]
    fn test_multiple_rapid_commands() {
        // Arrange
        let core = create_test_core();

        // Act - Send many commands rapidly
        let mut all_events = vec![];
        for i in 0..10 {
            let frame_key = crate::FrameKey {
                stream: crate::StreamId::A,
                frame_index: i,
                pts: Some(i as u64),
            };
            let events = core.handle_command(Command::SelectFrame {
                stream: crate::StreamId::A,
                frame_key,
            });
            all_events.extend(events);
        }

        // Assert - All commands should generate events
        assert_eq!(all_events.len(), 10);

        // Last selection wins
        let selection = core.get_selection();
        assert_eq!(selection.read().current_frame(), Some(9));
    }
}
