// Index dev HUD module tests
//
// Per generate-tests skill: Arrange-Act-Assert pattern with fixtures
// and edge case coverage.

use super::*;
use crate::{IndexingState, WindowStats};
use std::sync::{Arc, Mutex};

// ============================================================================
// Fixtures
// ============================================================================

/// Create a test HUD
fn create_test_hud() -> IndexDevHUD {
    IndexDevHUD::new("test_session".to_string())
}

/// Create a test window stats
fn create_test_window_stats() -> WindowStats {
    WindowStats {
        window_moves: 5,
        frames_materialized: 100,
        frames_evicted: 20,
        cache_hits: 80,
        cache_misses: 20,
    }
}

// ============================================================================
// IndexDevHUD Construction Tests
// ============================================================================

#[cfg(test)]
mod construction_tests {
    use super::*;

    #[test]
    fn test_new_creates_hud() {
        // Arrange & Act
        let hud = IndexDevHUD::new("test_session".to_string());

        // Assert
        assert_eq!(hud.session_id(), "test_session");
        assert_eq!(hud.update_count(), 0);
    }

    #[test]
    fn test_new_initial_session_snapshot() {
        // Arrange & Act
        let hud = IndexDevHUD::new("test".to_string());

        // Assert
        assert_eq!(hud.session_snapshot().state, IndexingState::Idle);
        assert!(!hud.session_snapshot().quick_complete);
        assert!(!hud.session_snapshot().full_complete);
        assert_eq!(hud.session_snapshot().progress, 0.0);
    }

    #[test]
    fn test_new_initial_window_snapshot_none() {
        // Arrange & Act
        let hud = IndexDevHUD::new("test".to_string());

        // Assert
        assert!(hud.window_snapshot().is_none());
    }

    #[test]
    fn test_new_initial_evidence_snapshot() {
        // Arrange & Act
        let hud = IndexDevHUD::new("test".to_string());

        // Assert
        assert_eq!(hud.evidence_snapshot().frame_evidence_count, 0);
        assert_eq!(hud.evidence_snapshot().session_operation_count, 0);
    }

    #[test]
    fn test_new_initial_performance_metrics() {
        // Arrange & Act
        let hud = IndexDevHUD::new("test".to_string());

        // Assert
        assert!(hud.performance_metrics().quick_index_duration_ms.is_none());
        assert!(hud.performance_metrics().full_index_duration_ms.is_none());
        assert_eq!(hud.performance_metrics().estimated_memory_bytes, 0);
    }
}

// ============================================================================
// Update From Session Tests
// ============================================================================

#[cfg(test)]
mod update_from_session_tests {
    use super::*;
    use crate::IndexSession;

    #[test]
    fn test_update_from_session_increments_count() {
        // Arrange
        let mut hud = create_test_hud();
        let session = IndexSession::new();

        // Act
        hud.update_from_session(&session);

        // Assert
        assert_eq!(hud.update_count(), 1);
    }

    #[test]
    fn test_update_from_session_idle_state() {
        // Arrange
        let mut hud = create_test_hud();
        let session = IndexSession::new();

        // Act
        hud.update_from_session(&session);

        // Assert
        assert_eq!(hud.session_snapshot().state, IndexingState::Idle);
    }

    #[test]
    fn test_update_from_session_progress() {
        // Arrange
        let mut hud = create_test_hud();
        let session = IndexSession::new();

        // Act
        hud.update_from_session(&session);

        // Assert
        assert_eq!(hud.session_snapshot().progress, 0.0);
    }

    #[test]
    fn test_update_from_session_no_estimates() {
        // Arrange
        let mut hud = create_test_hud();
        let session = IndexSession::new();

        // Act
        hud.update_from_session(&session);

        // Assert
        assert!(hud.session_snapshot().total_frames_estimate.is_none());
        assert!(hud.session_snapshot().actual_frames_indexed.is_none());
    }
}

// ============================================================================
// Update From Window Tests
// ============================================================================

#[cfg(test)]
mod update_from_window_tests {
    use super::*;
    use crate::IndexSessionWindow;
    use crate::indexing::SeekPoint;

    fn create_test_window() -> IndexSessionWindow {
        let sparse_keyframes = vec![SeekPoint {
            display_idx: 0,
            byte_offset: 1000,
            is_keyframe: true,
            pts: Some(0u64),
        }];
        IndexSessionWindow::new(
            "test".to_string(),
            10000,
            crate::IndexWindowPolicy::Fixed(1000),
            sparse_keyframes,
        )
    }

    #[test]
    fn test_update_from_window_increments_count() {
        // Arrange
        let mut hud = create_test_hud();
        let window = create_test_window();

        // Act
        hud.update_from_window(&window);

        // Assert
        assert_eq!(hud.update_count(), 1);
    }

    #[test]
    fn test_update_from_window_creates_snapshot() {
        // Arrange
        let mut hud = create_test_hud();
        let window = create_test_window();

        // Act
        hud.update_from_window(&window);

        // Assert
        assert!(hud.window_snapshot().is_some());
    }

    #[test]
    fn test_update_from_window_captures_frame_counts() {
        // Arrange
        let mut hud = create_test_hud();
        let window = create_test_window();

        // Act
        hud.update_from_window(&window);

        // Assert
        let snapshot = hud.window_snapshot().unwrap();
        assert_eq!(snapshot.total_frames, 10000);
        assert_eq!(snapshot.window_start, 0);
        assert_eq!(snapshot.window_size, 1000);
    }

    #[test]
    fn test_update_from_window_hit_rate() {
        // Arrange
        let mut hud = create_test_hud();
        let window = create_test_window();

        // Act
        hud.update_from_window(&window);

        // Assert
        let snapshot = hud.window_snapshot().unwrap();
        assert_eq!(snapshot.hit_rate, 0.0); // No activity yet
    }
}

// ============================================================================
// Update From Evidence Tests
// ============================================================================

#[cfg(test)]
mod update_from_evidence_tests {
    use super::*;
    use crate::{IndexExtractorEvidenceManager, IndexSessionEvidenceManager};

    #[test]
    fn test_update_from_evidence_increments_count() {
        // Arrange
        let mut hud = create_test_hud();
        let frame_evidence = Arc::new(Mutex::new(IndexExtractorEvidenceManager::new_empty()));
        let session_evidence = IndexSessionEvidenceManager::new(
            "test".to_string(),
            frame_evidence.clone(),
        );

        // Act
        hud.update_from_evidence(frame_evidence, &session_evidence);

        // Assert
        assert_eq!(hud.update_count(), 1);
    }

    #[test]
    fn test_update_from_evidence_empty_managers() {
        // Arrange
        let mut hud = create_test_hud();
        let frame_evidence = Arc::new(Mutex::new(IndexExtractorEvidenceManager::new_empty()));
        let session_evidence = IndexSessionEvidenceManager::new(
            "test".to_string(),
            frame_evidence.clone(),
        );

        // Act
        hud.update_from_evidence(frame_evidence, &session_evidence);

        // Assert
        assert_eq!(hud.evidence_snapshot().frame_evidence_count, 0);
        assert_eq!(hud.evidence_snapshot().session_operation_count, 0);
        assert_eq!(hud.evidence_snapshot().bit_offset_evidence_count, 0);
        assert_eq!(hud.evidence_snapshot().syntax_evidence_count, 0);
    }
}

// ============================================================================
// Update Performance Tests
// ============================================================================

#[cfg(test)]
mod update_performance_tests {
    use super::*;

    #[test]
    fn test_update_performance_quick_duration() {
        // Arrange
        let mut hud = create_test_hud();

        // Act
        hud.update_performance(Some(1500), None);

        // Assert
        assert_eq!(hud.performance_metrics().quick_index_duration_ms, Some(1500));
        assert!(hud.performance_metrics().full_index_duration_ms.is_none());
    }

    #[test]
    fn test_update_performance_full_duration() {
        // Arrange
        let mut hud = create_test_hud();

        // Act
        hud.update_performance(None, Some(5000));

        // Assert
        assert!(hud.performance_metrics().quick_index_duration_ms.is_none());
        assert_eq!(hud.performance_metrics().full_index_duration_ms, Some(5000));
    }

    #[test]
    fn test_update_performance_both_durations() {
        // Arrange
        let mut hud = create_test_hud();

        // Act
        hud.update_performance(Some(1000), Some(8000));

        // Assert
        assert_eq!(hud.performance_metrics().quick_index_duration_ms, Some(1000));
        assert_eq!(hud.performance_metrics().full_index_duration_ms, Some(8000));
    }

    #[test]
    fn test_update_performance_estimates_memory() {
        // Arrange
        let mut hud = create_test_hud();

        // First populate evidence snapshot to get non-zero memory estimate
        let frame_evidence = Arc::new(Mutex::new(IndexExtractorEvidenceManager::new_empty()));
        let session_evidence = IndexSessionEvidenceManager::new(
            "test".to_string(),
            frame_evidence.clone(),
        );
        hud.update_from_evidence(frame_evidence, &session_evidence);

        // Act
        hud.update_performance(Some(1000), Some(5000));

        // Assert - Memory estimate still 0 because evidence is empty
        // The estimate only counts actual evidence, which we haven't added
        assert_eq!(hud.performance_metrics().estimated_memory_bytes, 0);
    }

    #[test]
    fn test_update_performance_increments_count() {
        // Arrange
        let mut hud = create_test_hud();

        // Act
        hud.update_performance(None, None);

        // Assert
        assert_eq!(hud.update_count(), 1);
    }
}

// ============================================================================
// Snapshot Accessor Tests
// ============================================================================

#[cfg(test)]
mod snapshot_accessor_tests {
    use super::*;

    #[test]
    fn test_session_snapshot_returns_ref() {
        // Arrange
        let hud = create_test_hud();

        // Act
        let snapshot = hud.session_snapshot();

        // Assert
        assert_eq!(snapshot.state, IndexingState::Idle);
    }

    #[test]
    fn test_window_snapshot_returns_option() {
        // Arrange
        let hud = create_test_hud();

        // Act
        let snapshot = hud.window_snapshot();

        // Assert
        assert!(snapshot.is_none());
    }

    #[test]
    fn test_evidence_snapshot_returns_ref() {
        // Arrange
        let hud = create_test_hud();

        // Act
        let snapshot = hud.evidence_snapshot();

        // Assert
        assert_eq!(snapshot.frame_evidence_count, 0);
    }

    #[test]
    fn test_performance_metrics_returns_ref() {
        // Arrange
        let hud = create_test_hud();

        // Act
        let metrics = hud.performance_metrics();

        // Assert
        assert!(metrics.quick_index_duration_ms.is_none());
    }
}

// ============================================================================
// Session ID Tests
// ============================================================================

#[cfg(test)]
mod session_id_tests {
    use super::*;

    #[test]
    fn test_session_id_returns_value() {
        // Arrange
        let hud = IndexDevHUD::new("my_session".to_string());

        // Act
        let id = hud.session_id();

        // Assert
        assert_eq!(id, "my_session");
    }
}

// ============================================================================
// Format Text Tests
// ============================================================================

#[cfg(test)]
mod format_text_tests {
    use super::*;

    #[test]
    fn test_format_text_includes_session_id() {
        // Arrange
        let hud = IndexDevHUD::new("test_session".to_string());

        // Act
        let text = hud.format_text();

        // Assert
        assert!(text.contains("test_session"));
    }

    #[test]
    fn test_format_text_includes_update_count() {
        // Arrange
        let mut hud = create_test_hud();

        // Act
        let text = hud.format_text();

        // Assert
        assert!(text.contains("Updates: 0"));
    }

    #[test]
    fn test_format_text_includes_session_state() {
        // Arrange
        let hud = create_test_hud();

        // Act
        let text = hud.format_text();

        // Assert
        assert!(text.contains("State:"));
        assert!(text.contains("Idle"));
    }

    #[test]
    fn test_format_text_includes_progress() {
        // Arrange
        let hud = create_test_hud();

        // Act
        let text = hud.format_text();

        // Assert
        assert!(text.contains("Progress:"));
    }
}

// ============================================================================
// SessionSnapshot Tests
// ============================================================================

#[cfg(test)]
mod session_snapshot_tests {
    use super::*;

    #[test]
    fn test_session_snapshot_serializable() {
        // Arrange & Act
        let snapshot = SessionSnapshot {
            state: IndexingState::Idle,
            quick_complete: false,
            full_complete: false,
            total_frames_estimate: Some(1000),
            actual_frames_indexed: Some(500),
            progress: 0.5,
        };

        // Assert - Just verify it can be created
        assert_eq!(snapshot.state, IndexingState::Idle);
        assert_eq!(snapshot.total_frames_estimate, Some(1000));
    }
}

// ============================================================================
// WindowSnapshot Tests
// ============================================================================

#[cfg(test)]
mod window_snapshot_tests {
    use super::*;

    #[test]
    fn test_window_snapshot_complete() {
        // Arrange & Act
        let snapshot = WindowSnapshot {
            total_frames: 10000,
            window_start: 1000,
            window_size: 2000,
            current_position: 1500,
            materialized_count: 500,
            window_revision: 5,
            stats: create_test_window_stats(),
            hit_rate: 0.8,
        };

        // Assert
        assert_eq!(snapshot.total_frames, 10000);
        assert_eq!(snapshot.window_start, 1000);
        assert_eq!(snapshot.window_size, 2000);
        assert_eq!(snapshot.hit_rate, 0.8);
    }
}

// ============================================================================
// EvidenceSnapshot Tests
// ============================================================================

#[cfg(test)]
mod evidence_snapshot_tests {
    use super::*;

    #[test]
    fn test_evidence_snapshot_complete() {
        // Arrange & Act
        let snapshot = EvidenceSnapshot {
            frame_evidence_count: 100,
            session_operation_count: 10,
            bit_offset_evidence_count: 100,
            syntax_evidence_count: 100,
        };

        // Assert
        assert_eq!(snapshot.frame_evidence_count, 100);
        assert_eq!(snapshot.session_operation_count, 10);
    }
}

// ============================================================================
// PerformanceMetrics Tests
// ============================================================================

#[cfg(test)]
mod performance_metrics_tests {
    use super::*;

    #[test]
    fn test_performance_metrics_complete() {
        // Arrange & Act
        let metrics = PerformanceMetrics {
            quick_index_duration_ms: Some(1500),
            full_index_duration_ms: Some(8000),
            indexing_fps: Some(125.0),
            estimated_memory_bytes: 1024000,
        };

        // Assert
        assert_eq!(metrics.quick_index_duration_ms, Some(1500));
        assert_eq!(metrics.indexing_fps, Some(125.0));
        assert_eq!(metrics.estimated_memory_bytes, 1024000);
    }

    #[test]
    fn test_performance_metrics_empty() {
        // Arrange & Act
        let metrics = PerformanceMetrics {
            quick_index_duration_ms: None,
            full_index_duration_ms: None,
            indexing_fps: None,
            estimated_memory_bytes: 0,
        };

        // Assert
        assert!(metrics.quick_index_duration_ms.is_none());
        assert!(metrics.indexing_fps.is_none());
    }
}

// ============================================================================
// Edge Cases
// ============================================================================

#[cfg(test)]
mod edge_case_tests {
    use super::*;

    #[test]
    fn test_multiple_updates_increment_count() {
        // Arrange
        let mut hud = create_test_hud();
        let session = crate::IndexSession::new();

        // Act
        for _ in 0..5 {
            hud.update_from_session(&session);
        }

        // Assert
        assert_eq!(hud.update_count(), 5);
    }

    #[test]
    fn test_update_performance_with_zero_duration() {
        // Arrange
        let mut hud = create_test_hud();

        // Act
        hud.update_performance(Some(0), Some(0));

        // Assert
        assert_eq!(hud.performance_metrics().full_index_duration_ms, Some(0));
        // FPS would be infinity, so should be None
        assert!(hud.performance_metrics().indexing_fps.is_none());
    }

    #[test]
    fn test_format_text_handles_missing_data() {
        // Arrange
        let hud = create_test_hud();

        // Act - Should not panic
        let text = hud.format_text();

        // Assert
        assert!(!text.is_empty());
    }
}
