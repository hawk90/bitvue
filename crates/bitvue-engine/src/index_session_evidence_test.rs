// Index session evidence module tests
//
// Per generate-tests skill: Arrange-Act-Assert pattern with fixtures
// and edge case coverage.

use super::*;
use crate::index_session::IndexingState;
use std::sync::{Arc, Mutex};

// ============================================================================
// Fixtures
// ============================================================================

/// Create a test frame evidence manager
fn create_test_frame_evidence_manager() -> Arc<Mutex<IndexExtractorEvidenceManager>> {
    Arc::new(Mutex::new(IndexExtractorEvidenceManager::new_empty()))
}

/// Create a test session evidence manager
fn create_test_session_evidence_manager() -> IndexSessionEvidenceManager {
    let frame_mgr = create_test_frame_evidence_manager();
    IndexSessionEvidenceManager::new("test_session".to_string(), frame_mgr)
}

// ============================================================================
// SessionOperation Tests
// ============================================================================

#[cfg(test)]
mod session_operation_tests {
    use super::*;

    #[test]
    fn test_session_operation_quick_index_start() {
        // Arrange & Act
        let op = SessionOperation::QuickIndexStart;

        // Assert
        assert_eq!(op, SessionOperation::QuickIndexStart);
        let _ = format!("{:?}", op); // Can be debug printed
    }

    #[test]
    fn test_session_operation_quick_index_complete() {
        // Arrange & Act
        let op = SessionOperation::QuickIndexComplete { keyframe_count: 100 };

        // Assert
        match op {
            SessionOperation::QuickIndexComplete { keyframe_count } => {
                assert_eq!(keyframe_count, 100);
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_session_operation_full_index_start() {
        // Arrange & Act
        let op = SessionOperation::FullIndexStart;

        // Assert
        assert_eq!(op, SessionOperation::FullIndexStart);
    }

    #[test]
    fn test_session_operation_full_index_complete() {
        // Arrange & Act
        let op = SessionOperation::FullIndexComplete { total_frames: 1000 };

        // Assert
        match op {
            SessionOperation::FullIndexComplete { total_frames } => {
                assert_eq!(total_frames, 1000);
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_session_operation_session_reset() {
        // Arrange & Act
        let op = SessionOperation::SessionReset;

        // Assert
        assert_eq!(op, SessionOperation::SessionReset);
    }

    #[test]
    fn test_session_operation_cancelled() {
        // Arrange & Act
        let op = SessionOperation::OperationCancelled {
            phase: IndexingPhase::Quick,
        };

        // Assert
        match op {
            SessionOperation::OperationCancelled { phase } => {
                assert_eq!(phase, IndexingPhase::Quick);
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_session_operation_error() {
        // Arrange & Act
        let op = SessionOperation::OperationError {
            phase: IndexingPhase::Full,
            error_message: "Test error".to_string(),
        };

        // Assert
        match op {
            SessionOperation::OperationError {
                phase,
                error_message,
            } => {
                assert_eq!(phase, IndexingPhase::Full);
                assert_eq!(error_message, "Test error");
            }
            _ => panic!("Wrong variant"),
        }
    }
}

// ============================================================================
// IndexSessionEvidenceManager Construction Tests
// ============================================================================

#[cfg(test)]
mod construction_tests {
    use super::*;

    #[test]
    fn test_new_creates_manager() {
        // Arrange
        let frame_mgr = create_test_frame_evidence_manager();

        // Act
        let manager = IndexSessionEvidenceManager::new("test".to_string(), frame_mgr);

        // Assert
        assert_eq!(manager.session_id(), "test");
        assert!(manager.operations().is_empty());
        assert_eq!(manager.next_operation_id, 0);
    }

    #[test]
    fn test_new_with_different_session_id() {
        // Arrange
        let frame_mgr = create_test_frame_evidence_manager();

        // Act
        let manager = IndexSessionEvidenceManager::new("custom_session".to_string(), frame_mgr);

        // Assert
        assert_eq!(manager.session_id(), "custom_session");
    }
}

// ============================================================================
// Record Operation Tests
// ============================================================================

#[cfg(test)]
mod record_operation_tests {
    use super::*;

    #[test]
    fn test_record_operation_returns_id() {
        // Arrange
        let mut manager = create_test_session_evidence_manager();

        // Act
        let id = manager.record_operation(
            SessionOperation::QuickIndexStart,
            IndexingState::Idle,
            IndexingState::QuickIndexing,
            vec![],
        );

        // Assert
        assert_eq!(id, "session_test_session_op_0");
    }

    #[test]
    fn test_record_operation_multiple_increments_id() {
        // Arrange
        let mut manager = create_test_session_evidence_manager();

        // Act
        let id1 = manager.record_operation(
            SessionOperation::QuickIndexStart,
            IndexingState::Idle,
            IndexingState::QuickIndexing,
            vec![],
        );
        let id2 = manager.record_operation(
            SessionOperation::FullIndexStart,
            IndexingState::QuickIndexing,
            IndexingState::FullIndexing,
            vec![],
        );

        // Assert
        assert_eq!(id1, "session_test_session_op_0");
        assert_eq!(id2, "session_test_session_op_1");
    }

    #[test]
    fn test_record_operation_adds_to_operations() {
        // Arrange
        let mut manager = create_test_session_evidence_manager();

        // Act
        manager.record_operation(
            SessionOperation::QuickIndexStart,
            IndexingState::Idle,
            IndexingState::QuickIndexing,
            vec![],
        );

        // Assert
        assert_eq!(manager.operations().len(), 1);
    }
}

// ============================================================================
// Get Operations Tests
// ============================================================================

#[cfg(test)]
mod get_operations_tests {
    use super::*;

    #[test]
    fn test_operations_returns_empty_initially() {
        // Arrange
        let manager = create_test_session_evidence_manager();

        // Act
        let ops = manager.operations();

        // Assert
        assert!(ops.is_empty());
    }

    #[test]
    fn test_get_operation_by_id_exists() {
        // Arrange
        let mut manager = create_test_session_evidence_manager();
        let id = manager.record_operation(
            SessionOperation::QuickIndexStart,
            IndexingState::Idle,
            IndexingState::QuickIndexing,
            vec![],
        );

        // Act
        let result = manager.get_operation(&id);

        // Assert
        assert!(result.is_some());
    }

    #[test]
    fn test_get_operation_by_id_not_exists() {
        // Arrange
        let manager = create_test_session_evidence_manager();

        // Act
        let result = manager.get_operation("invalid_id");

        // Assert
        assert!(result.is_none());
    }

    #[test]
    fn test_get_operations_by_type() {
        // Arrange
        let mut manager = create_test_session_evidence_manager();
        manager.record_operation(
            SessionOperation::QuickIndexStart,
            IndexingState::Idle,
            IndexingState::QuickIndexing,
            vec![],
        );
        manager.record_operation(
            SessionOperation::FullIndexStart,
            IndexingState::QuickIndexing,
            IndexingState::FullIndexing,
            vec![],
        );

        // Act
        let quick_ops = manager.get_operations_by_type(&SessionOperation::QuickIndexStart);

        // Assert
        assert_eq!(quick_ops.len(), 1);
    }

    #[test]
    fn test_last_operation_returns_none_when_empty() {
        // Arrange
        let manager = create_test_session_evidence_manager();

        // Act
        let result = manager.last_operation();

        // Assert
        assert!(result.is_none());
    }

    #[test]
    fn test_last_operation_returns_last() {
        // Arrange
        let mut manager = create_test_session_evidence_manager();
        manager.record_operation(
            SessionOperation::QuickIndexStart,
            IndexingState::Idle,
            IndexingState::QuickIndexing,
            vec![],
        );
        manager.record_operation(
            SessionOperation::FullIndexStart,
            IndexingState::QuickIndexing,
            IndexingState::FullIndexing,
            vec![],
        );

        // Act
        let last = manager.last_operation();

        // Assert
        assert!(last.is_some());
        match last.unwrap().operation {
            SessionOperation::FullIndexStart => {}
            _ => panic!("Expected FullIndexStart"),
        }
    }
}

// ============================================================================
// Trace Tests
// ============================================================================

#[cfg(test)]
mod trace_tests {
    use super::*;

    #[test]
    fn test_trace_operation_to_frames_empty() {
        // Arrange
        let manager = create_test_session_evidence_manager();

        // Act
        let frames = manager.trace_operation_to_frames("invalid_id");

        // Assert
        assert!(frames.is_empty());
    }

    #[test]
    fn test_trace_frame_to_operations_empty() {
        // Arrange
        let manager = create_test_session_evidence_manager();

        // Act
        let ops = manager.trace_frame_to_operations(0);

        // Assert
        assert!(ops.is_empty());
    }
}

// ============================================================================
// Session Stats Tests
// ============================================================================

#[cfg(test)]
mod session_stats_tests {
    use super::*;

    #[test]
    fn test_session_stats_default() {
        // Arrange & Act
        let stats = SessionStats::default();

        // Assert
        assert_eq!(stats.total_operations, 0);
        assert_eq!(stats.quick_index_count, 0);
        assert_eq!(stats.full_index_count, 0);
        assert_eq!(stats.total_keyframes_indexed, 0);
        assert_eq!(stats.total_frames_indexed, 0);
        assert_eq!(stats.cancelled_operations, 0);
        assert_eq!(stats.error_operations, 0);
    }

    #[test]
    fn test_session_stats_with_quick_index() {
        // Arrange
        let mut manager = create_test_session_evidence_manager();
        manager.record_operation(
            SessionOperation::QuickIndexComplete {
                keyframe_count: 10,
            },
            IndexingState::Idle,
            IndexingState::QuickComplete,
            vec![],
        );

        // Act
        let stats = manager.session_stats();

        // Assert
        assert_eq!(stats.total_operations, 1);
        assert_eq!(stats.quick_index_count, 1);
        assert_eq!(stats.total_keyframes_indexed, 10);
    }

    #[test]
    fn test_session_stats_with_full_index() {
        // Arrange
        let mut manager = create_test_session_evidence_manager();
        manager.record_operation(
            SessionOperation::FullIndexComplete {
                total_frames: 1000,
            },
            IndexingState::QuickComplete,
            IndexingState::FullComplete,
            vec![],
        );

        // Act
        let stats = manager.session_stats();

        // Assert
        assert_eq!(stats.total_operations, 1);
        assert_eq!(stats.full_index_count, 1);
        assert_eq!(stats.total_frames_indexed, 1000);
    }

    #[test]
    fn test_session_stats_with_cancelled() {
        // Arrange
        let mut manager = create_test_session_evidence_manager();
        manager.record_operation(
            SessionOperation::OperationCancelled {
                phase: IndexingPhase::Quick,
            },
            IndexingState::QuickIndexing,
            IndexingState::Cancelled,
            vec![],
        );

        // Act
        let stats = manager.session_stats();

        // Assert
        assert_eq!(stats.cancelled_operations, 1);
    }

    #[test]
    fn test_session_stats_with_error() {
        // Arrange
        let mut manager = create_test_session_evidence_manager();
        manager.record_operation(
            SessionOperation::OperationError {
                phase: IndexingPhase::Full,
                error_message: "Test error".to_string(),
            },
            IndexingState::FullIndexing,
            IndexingState::Error,
            vec![],
        );

        // Act
        let stats = manager.session_stats();

        // Assert
        assert_eq!(stats.error_operations, 1);
    }

    #[test]
    fn test_session_stats_accumulates() {
        // Arrange
        let mut manager = create_test_session_evidence_manager();
        manager.record_operation(
            SessionOperation::QuickIndexComplete {
                keyframe_count: 10,
            },
            IndexingState::Idle,
            IndexingState::QuickComplete,
            vec![],
        );
        manager.record_operation(
            SessionOperation::FullIndexComplete {
                total_frames: 100,
            },
            IndexingState::QuickComplete,
            IndexingState::FullComplete,
            vec![],
        );
        manager.record_operation(
            SessionOperation::QuickIndexComplete {
                keyframe_count: 20,
            },
            IndexingState::FullComplete,
            IndexingState::QuickComplete,
            vec![],
        );

        // Act
        let stats = manager.session_stats();

        // Assert
        assert_eq!(stats.total_operations, 3);
        assert_eq!(stats.quick_index_count, 2);
        assert_eq!(stats.full_index_count, 1);
        assert_eq!(stats.total_keyframes_indexed, 30);
        assert_eq!(stats.total_frames_indexed, 100);
    }
}

// ============================================================================
// Evidence Chain Tests
// ============================================================================

#[cfg(test)]
mod evidence_chain_tests {
    use super::*;

    #[test]
    fn test_evidence_chain_accessible() {
        // Arrange
        let manager = create_test_session_evidence_manager();

        // Act
        let chain = manager.evidence_chain();

        // Assert - Evidence chain should be accessible
        let _ = chain.bit_offset_index.all();
    }
}

// ============================================================================
// Clear Tests
// ============================================================================

#[cfg(test)]
mod clear_tests {
    use super::*;

    #[test]
    fn test_clear_resets_operations() {
        // Arrange
        let mut manager = create_test_session_evidence_manager();
        manager.record_operation(
            SessionOperation::QuickIndexStart,
            IndexingState::Idle,
            IndexingState::QuickIndexing,
            vec![],
        );
        assert_eq!(manager.operations().len(), 1);

        // Act
        manager.clear();

        // Assert
        assert!(manager.operations().is_empty());
    }

    #[test]
    fn test_clear_resets_operation_id_counter() {
        // Arrange
        let mut manager = create_test_session_evidence_manager();
        manager.record_operation(
            SessionOperation::QuickIndexStart,
            IndexingState::Idle,
            IndexingState::QuickIndexing,
            vec![],
        );

        // Act
        manager.clear();

        // Assert - Next operation should start from 0 again
        let id = manager.record_operation(
            SessionOperation::QuickIndexStart,
            IndexingState::Idle,
            IndexingState::QuickIndexing,
            vec![],
        );

        assert_eq!(id, "session_test_session_op_0");
    }
}

// ============================================================================
// Edge Cases
// ============================================================================

#[cfg(test)]
mod edge_case_tests {
    use super::*;

    #[test]
    fn test_record_operation_with_empty_frame_list() {
        // Arrange
        let mut manager = create_test_session_evidence_manager();

        // Act - Should not panic with empty frame list
        let id = manager.record_operation(
            SessionOperation::QuickIndexStart,
            IndexingState::Idle,
            IndexingState::QuickIndexing,
            vec![],
        );

        // Assert
        assert!(!id.is_empty());
        assert_eq!(manager.operations().len(), 1);
    }

    #[test]
    fn test_get_operations_by_type_with_nonexistent_type() {
        // Arrange
        let manager = create_test_session_evidence_manager();

        // Act - Search for operation type that doesn't exist
        let ops = manager.get_operations_by_type(&SessionOperation::QuickIndexStart);

        // Assert
        assert!(ops.is_empty());
    }

    #[test]
    fn test_multiple_sessions_have_sequential_ids() {
        // Arrange
        let frame_mgr = create_test_frame_evidence_manager();
        let mut manager1 = IndexSessionEvidenceManager::new("session1".to_string(), frame_mgr.clone());
        let mut manager2 = IndexSessionEvidenceManager::new("session2".to_string(), frame_mgr);

        // Act
        let id1 = manager1.record_operation(
            SessionOperation::QuickIndexStart,
            IndexingState::Idle,
            IndexingState::QuickIndexing,
            vec![],
        );
        let id2 = manager2.record_operation(
            SessionOperation::QuickIndexStart,
            IndexingState::Idle,
            IndexingState::QuickIndexing,
            vec![],
        );

        // Assert - Each manager should have its own ID sequence
        assert_eq!(id1, "session_session1_op_0");
        assert_eq!(id2, "session_session2_op_0");
    }

    #[test]
    fn test_session_stats_with_mixed_operations() {
        // Arrange
        let mut manager = create_test_session_evidence_manager();
        manager.record_operation(
            SessionOperation::QuickIndexComplete {
                keyframe_count: 5,
            },
            IndexingState::Idle,
            IndexingState::QuickComplete,
            vec![],
        );
        manager.record_operation(
            SessionOperation::OperationCancelled {
                phase: IndexingPhase::Quick,
            },
            IndexingState::QuickIndexing,
            IndexingState::Cancelled,
            vec![],
        );
        manager.record_operation(
            SessionOperation::OperationError {
                phase: IndexingPhase::Full,
                error_message: "Parse error".to_string(),
            },
            IndexingState::Cancelled,
            IndexingState::Error,
            vec![],
        );
        manager.record_operation(
            SessionOperation::FullIndexComplete {
                total_frames: 50,
            },
            IndexingState::Error,
            IndexingState::FullComplete,
            vec![],
        );

        // Act
        let stats = manager.session_stats();

        // Assert
        assert_eq!(stats.total_operations, 4);
        assert_eq!(stats.quick_index_count, 1);
        assert_eq!(stats.full_index_count, 1);
        assert_eq!(stats.total_keyframes_indexed, 5);
        assert_eq!(stats.total_frames_indexed, 50);
        assert_eq!(stats.cancelled_operations, 1);
        assert_eq!(stats.error_operations, 1);
    }

    #[test]
    fn test_state_transitions_recorded() {
        // Arrange
        let mut manager = create_test_session_evidence_manager();

        // Act
        manager.record_operation(
            SessionOperation::QuickIndexStart,
            IndexingState::Idle,
            IndexingState::QuickIndexing,
            vec![],
        );

        // Assert
        let op = &manager.operations()[0];
        assert_eq!(op.state_before, IndexingState::Idle);
        assert_eq!(op.state_after, IndexingState::QuickIndexing);
    }

    #[test]
    fn test_timestamp_ms_is_zero_in_tests() {
        // Arrange
        let mut manager = create_test_session_evidence_manager();

        // Act
        manager.record_operation(
            SessionOperation::QuickIndexStart,
            IndexingState::Idle,
            IndexingState::QuickIndexing,
            vec![],
        );

        // Assert
        let op = &manager.operations()[0];
        assert_eq!(op.timestamp_ms, 0); // current_time_ms returns 0 in tests
    }
}
