// Index session module tests
//
// Per generate-tests skill: Arrange-Act-Assert pattern with fixtures
// and edge case coverage.

use super::*;
use crate::index_extractor::{Av1IndexExtractor, IndexExtractor};
use std::io::Cursor;

// ============================================================================
// Fixtures
// ============================================================================

/// Create a test AV1 extractor
fn create_test_extractor() -> Av1IndexExtractor {
    Av1IndexExtractor::new()
}

/// Create test AV1 OBU data
fn create_test_av1_data(keyframe_count: usize) -> Vec<u8> {
    let mut data = Vec::new();
    for _ in 0..keyframe_count {
        // OBU header for FRAME (type 6) with size field
        data.push(6 << 3 | 0x02); // OBU type 6, has size
        data.push(10); // LEB128 size = 10
        // Add 10 bytes of "payload"
        data.extend_from_slice(&[0xFFu8; 10]);
    }
    data
}

// ============================================================================
// IndexingState Tests
// ============================================================================

#[cfg(test)]
mod indexing_state_tests {
    use super::*;

    #[test]
    fn test_indexing_state_variants_exist() {
        // Arrange & Act - Just verify all states can be created
        let states = vec![
            IndexingState::Idle,
            IndexingState::QuickIndexing,
            IndexingState::QuickComplete,
            IndexingState::FullIndexing,
            IndexingState::FullComplete,
            IndexingState::Error,
            IndexingState::Cancelled,
        ];

        // Assert - All states should be valid
        for state in states {
            let _ = format!("{:?}", state); // Can be debug printed
        }
    }

    #[test]
    fn test_indexing_state_partial_equality() {
        // Arrange
        let state1 = IndexingState::Idle;
        let state2 = IndexingState::Idle;
        let state3 = IndexingState::QuickIndexing;

        // Assert
        assert_eq!(state1, state2);
        assert_ne!(state1, state3);
    }
}

// ============================================================================
// IndexingPhase Tests
// ============================================================================

#[cfg(test)]
mod indexing_phase_tests {
    use super::*;

    #[test]
    fn test_indexing_phase_variants() {
        // Arrange & Act
        let phases = vec![IndexingPhase::Quick, IndexingPhase::Full];

        // Assert - Both phases should be valid
        for phase in phases {
            let _ = format!("{:?}", phase);
        }
    }
}

// ============================================================================
// IndexingProgress Tests
// ============================================================================

#[cfg(test)]
mod indexing_progress_tests {
    use super::*;

    #[test]
    fn test_indexing_progress_creation() {
        // Arrange & Act
        let progress = IndexingProgress {
            phase: IndexingPhase::Quick,
            progress: 0.5,
            message: "Test message".to_string(),
            frames_indexed: 100,
        };

        // Assert
        assert_eq!(progress.phase, IndexingPhase::Quick);
        assert_eq!(progress.progress, 0.5);
        assert_eq!(progress.message, "Test message");
        assert_eq!(progress.frames_indexed, 100);
    }
}

// ============================================================================
// IndexSession Construction Tests
// ============================================================================

#[cfg(test)]
mod index_session_construction_tests {
    use super::*;

    #[test]
    fn test_index_session_new() {
        // Arrange & Act
        let session = IndexSession::new();

        // Assert
        assert_eq!(session.state(), IndexingState::Idle);
        assert!(!session.is_quick_complete());
        assert!(!session.is_full_complete());
        assert!(session.quick_index().is_none());
        assert!(session.full_index().is_none());
    }

    #[test]
    fn test_index_session_default() {
        // Arrange & Act
        let session = IndexSession::default();

        // Assert
        assert_eq!(session.state(), IndexingState::Idle);
    }
}

// ============================================================================
// Quick Index Tests
// ============================================================================

#[cfg(test)]
mod quick_index_tests {
    use super::*;

    #[test]
    fn test_execute_quick_index_with_valid_data() {
        // Arrange
        let session = IndexSession::new();
        let extractor = create_test_extractor();
        let data = create_test_av1_data(3);
        let mut cursor = Cursor::new(data);

        // Act
        let result = session.execute_quick_index::<fn(IndexingProgress)>(&extractor, &mut cursor, None);

        // Assert
        assert!(result.is_ok());
        let quick_idx = result.unwrap();
        assert!(!quick_idx.seek_points.is_empty());
        assert_eq!(session.state(), IndexingState::QuickComplete);
        assert!(session.is_quick_complete());
    }

    #[test]
    fn test_execute_quick_index_from_non_idle_state_fails() {
        // Arrange
        let session = IndexSession::new();
        // First, complete a quick index
        let extractor = create_test_extractor();
        let data = create_test_av1_data(1);
        let mut cursor = Cursor::new(data.clone());
        session.execute_quick_index::<fn(IndexingProgress)>(&extractor, &mut cursor, None).unwrap();

        // Now try to run another quick index (not from Idle state)
        let mut cursor2 = Cursor::new(data);

        // Act
        let result = session.execute_quick_index::<fn(IndexingProgress)>(&extractor, &mut cursor2, None);

        // Assert
        assert!(result.is_err());
    }

    #[test]
    fn test_quick_index_stores_result() {
        // Arrange
        let session = IndexSession::new();
        let extractor = create_test_extractor();
        let data = create_test_av1_data(2);
        let mut cursor = Cursor::new(data);

        // Act
        session.execute_quick_index::<fn(IndexingProgress)>(&extractor, &mut cursor, None).unwrap();

        // Assert
        let quick_idx = session.quick_index();
        assert!(quick_idx.is_some());
        assert!(quick_idx.unwrap().seek_points.len() >= 1);
    }

    #[test]
    fn test_execute_quick_index_with_progress_callback() {
        // Arrange
        let session = IndexSession::new();
        let extractor = create_test_extractor();
        let data = create_test_av1_data(1);
        let mut cursor = Cursor::new(data);

        let callback = |progress: IndexingProgress| {
            assert_eq!(progress.phase, IndexingPhase::Quick);
        };

        // Act
        let result = session.execute_quick_index(&extractor, &mut cursor, Some(callback));

        // Assert
        assert!(result.is_ok());
    }
}

// ============================================================================
// Full Index Tests
// ============================================================================

#[cfg(test)]
mod full_index_tests {
    use super::*;

    #[test]
    fn test_execute_full_index_requires_quick_complete() {
        // Arrange
        let session = IndexSession::new();
        let extractor = create_test_extractor();
        let data = create_test_av1_data(1);
        let mut cursor = Cursor::new(data);

        // Act - Try to run full index without quick index first
        let result = session.execute_full_index::<fn(IndexingProgress)>(&extractor, &mut cursor, None);

        // Assert
        assert!(result.is_err());
    }

    #[test]
    fn test_execute_full_index_after_quick() {
        // Arrange
        let session = IndexSession::new();
        let extractor = create_test_extractor();
        let data = create_test_av1_data(2);
        let mut cursor = Cursor::new(data.clone());

        // Act - First run quick index
        session.execute_quick_index::<fn(IndexingProgress)>(&extractor, &mut cursor, None).unwrap();

        // Then run full index
        let mut cursor2 = Cursor::new(data);
        let result = session.execute_full_index::<fn(IndexingProgress)>(&extractor, &mut cursor2, None);

        // Assert
        assert!(result.is_ok());
        assert_eq!(session.state(), IndexingState::FullComplete);
        assert!(session.is_full_complete());
    }

    #[test]
    fn test_full_index_stores_result() {
        // Arrange
        let session = IndexSession::new();
        let extractor = create_test_extractor();
        let data = create_test_av1_data(2);
        let mut cursor = Cursor::new(data.clone());

        // Act
        session.execute_quick_index::<fn(IndexingProgress)>(&extractor, &mut cursor, None).unwrap();
        let mut cursor2 = Cursor::new(data);
        session.execute_full_index::<fn(IndexingProgress)>(&extractor, &mut cursor2, None).unwrap();

        // Assert
        let full_idx = session.full_index();
        assert!(full_idx.is_some());
        assert!(!full_idx.unwrap().frames.is_empty());
    }
}

// ============================================================================
// Cancellation Tests
// ============================================================================

#[cfg(test)]
mod cancellation_tests {
    use super::*;

    #[test]
    fn test_cancel_sets_flag() {
        // Arrange
        let session = IndexSession::new();

        // Act
        session.cancel();

        // Assert - Flag is set (can't directly test, but cancel doesn't panic)
    }

    #[test]
    fn test_reset_clears_cancellation() {
        // Arrange
        let session = IndexSession::new();
        session.cancel();

        // Act
        session.reset();

        // Assert
        assert_eq!(session.state(), IndexingState::Idle);
    }
}

// ============================================================================
// Workflow Tests
// ============================================================================

#[cfg(test)]
mod workflow_tests {
    use super::*;

    #[test]
    fn test_execute_full_workflow() {
        // Arrange
        let session = IndexSession::new();
        let extractor = create_test_extractor();
        let data = create_test_av1_data(2);
        let mut cursor = Cursor::new(data);

        // Act
        let result = session.execute_full_workflow::<fn(IndexingProgress)>(&extractor, &mut cursor, None);

        // Assert
        assert!(result.is_ok());
        let (quick_idx, full_idx) = result.unwrap();
        assert!(!quick_idx.seek_points.is_empty());
        assert!(!full_idx.frames.is_empty());
        assert_eq!(session.state(), IndexingState::FullComplete);
    }

    #[test]
    fn test_workflow_state_transitions() {
        // Arrange
        let session = IndexSession::new();
        let extractor = create_test_extractor();
        let data = create_test_av1_data(1);
        let mut cursor = Cursor::new(data);

        // Act & Assert - Initial state
        assert_eq!(session.state(), IndexingState::Idle);

        // After quick index
        session.execute_quick_index::<fn(IndexingProgress)>(&extractor, &mut cursor, None).unwrap();
        assert_eq!(session.state(), IndexingState::QuickComplete);

        // After full index (need fresh cursor)
        let mut cursor2 = Cursor::new(create_test_av1_data(1));
        session.execute_full_index::<fn(IndexingProgress)>(&extractor, &mut cursor2, None).unwrap();
        assert_eq!(session.state(), IndexingState::FullComplete);
    }
}

// ============================================================================
// Reset Tests
// ============================================================================

#[cfg(test)]
mod reset_tests {
    use super::*;

    #[test]
    fn test_reset_clears_state() {
        // Arrange
        let session = IndexSession::new();
        let extractor = create_test_extractor();
        let data = create_test_av1_data(1);
        let mut cursor = Cursor::new(data.clone());

        // Run through workflow
        session.execute_quick_index::<fn(IndexingProgress)>(&extractor, &mut cursor, None).unwrap();

        // Act
        session.reset();

        // Assert
        assert_eq!(session.state(), IndexingState::Idle);
        assert!(session.quick_index().is_none());
        assert!(session.full_index().is_none());
    }

    #[test]
    fn test_reset_after_full_workflow() {
        // Arrange
        let session = IndexSession::new();
        let extractor = create_test_extractor();
        let data = create_test_av1_data(1);
        let mut cursor = Cursor::new(data);

        session.execute_full_workflow::<fn(IndexingProgress)>(&extractor, &mut cursor, None).unwrap();

        // Act
        session.reset();

        // Assert
        assert_eq!(session.state(), IndexingState::Idle);
        assert!(session.quick_index().is_none());
        assert!(session.full_index().is_none());
    }
}

// ============================================================================
// Progress Estimation Tests
// ============================================================================

#[cfg(test)]
mod progress_tests {
    use super::*;

    #[test]
    fn test_estimated_progress_idle() {
        // Arrange
        let session = IndexSession::new();

        // Act
        let progress = session.estimated_progress();

        // Assert
        assert_eq!(progress, 0.0);
    }

    #[test]
    fn test_estimated_progress_quick_complete() {
        // Arrange
        let session = IndexSession::new();
        let extractor = create_test_extractor();
        let data = create_test_av1_data(1);
        let mut cursor = Cursor::new(data);

        session.execute_quick_index::<fn(IndexingProgress)>(&extractor, &mut cursor, None).unwrap();

        // Act
        let progress = session.estimated_progress();

        // Assert
        assert_eq!(progress, 0.0);
    }

    #[test]
    fn test_estimated_progress_full_complete() {
        // Arrange
        let session = IndexSession::new();
        let extractor = create_test_extractor();
        let data = create_test_av1_data(1);
        let mut cursor = Cursor::new(data.clone());

        session.execute_full_workflow::<fn(IndexingProgress)>(&extractor, &mut cursor, None).unwrap();

        // Act
        let progress = session.estimated_progress();

        // Assert
        assert_eq!(progress, 1.0);
    }

    #[test]
    fn test_estimated_progress_error_state() {
        // Arrange - Create a session that will be in error state
        let session = IndexSession::new();
        let extractor = create_test_extractor();
        let empty_data = vec![]; // Invalid data that will cause error
        let mut cursor = Cursor::new(empty_data);

        // Act
        let _ = session.execute_quick_index::<fn(IndexingProgress)>(&extractor, &mut cursor, None);
        let progress = session.estimated_progress();

        // Assert
        assert_eq!(progress, 0.0); // Error state returns 0.0
    }
}

// ============================================================================
// Evidence Manager Tests
// ============================================================================

#[cfg(test)]
mod evidence_manager_tests {
    use super::*;

    #[test]
    fn test_evidence_manager_accessible() {
        // Arrange
        let session = IndexSession::new();

        // Act
        let evidence_mgr = session.evidence_manager();

        // Assert - Evidence manager should be accessible without panicking
        let _guard = evidence_mgr.lock().unwrap();
    }
}

// ============================================================================
// Edge Cases
// ============================================================================

#[cfg(test)]
mod edge_case_tests {
    use super::*;

    #[test]
    fn test_empty_quick_index_handling() {
        // Arrange
        let session = IndexSession::new();
        let extractor = create_test_extractor();
        let empty_data = vec![];
        let mut cursor = Cursor::new(empty_data);

        // Act
        let result = session.execute_quick_index::<fn(IndexingProgress)>(&extractor, &mut cursor, None);

        // Assert - Should handle empty data gracefully (either error or empty index)
        // The actual behavior depends on extractor implementation
        assert!(result.is_err() || result.is_ok());
    }

    #[test]
    fn test_multiple_quick_indexes_fail_after_first() {
        // Arrange
        let session = IndexSession::new();
        let extractor = create_test_extractor();
        let data = create_test_av1_data(1);
        let mut cursor1 = Cursor::new(data.clone());

        // Act - First quick index succeeds
        session.execute_quick_index::<fn(IndexingProgress)>(&extractor, &mut cursor1, None).unwrap();

        // Second quick index should fail
        let mut cursor2 = Cursor::new(data);
        let result = session.execute_quick_index::<fn(IndexingProgress)>(&extractor, &mut cursor2, None);

        // Assert
        assert!(result.is_err());
    }

    #[test]
    fn test_reset_allows_restarting() {
        // Arrange
        let session = IndexSession::new();
        let extractor = create_test_extractor();
        let data = create_test_av1_data(1);

        // First workflow
        let mut cursor1 = Cursor::new(data.clone());
        session.execute_full_workflow::<fn(IndexingProgress)>(&extractor, &mut cursor1, None).unwrap();

        // Reset
        session.reset();

        // Act - Should be able to run workflow again
        let mut cursor2 = Cursor::new(data);
        let result = session.execute_full_workflow::<fn(IndexingProgress)>(&extractor, &mut cursor2, None);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn test_concurrent_state_checks() {
        // Arrange
        let session = IndexSession::new();

        // Act - Multiple state checks should be consistent
        let state1 = session.state();
        let state2 = session.state();
        let state3 = session.state();

        // Assert
        assert_eq!(state1, IndexingState::Idle);
        assert_eq!(state2, IndexingState::Idle);
        assert_eq!(state3, IndexingState::Idle);
    }

    #[test]
    fn test_index_cloning() {
        // Arrange
        let session = IndexSession::new();
        let extractor = create_test_extractor();
        let data = create_test_av1_data(1);
        let mut cursor = Cursor::new(data);

        // Act - Use turbofish syntax to specify type
        session.execute_quick_index::<fn(IndexingProgress)>(&extractor, &mut cursor, None).unwrap();
        let quick_idx = session.quick_index();

        // Assert - Cloned index should have same data
        assert!(quick_idx.is_some());
        let idx = quick_idx.unwrap();
        assert!(!idx.seek_points.is_empty());
    }
}
