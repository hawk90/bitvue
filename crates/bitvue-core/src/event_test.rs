// Event module tests
//
// Per generate-tests skill: Arrange-Act-Assert pattern with fixtures
// and edge case coverage.

use super::*;
use crate::StreamId;
use std::path::PathBuf;

// ============================================================================
// Fixtures
// ============================================================================

/// Create a test path
fn create_test_path() -> PathBuf {
    PathBuf::from("/tmp/test.csv")
}

/// Create a test stream ID A
fn test_stream_a() -> StreamId {
    StreamId::A
}

/// Create a test stream ID B
fn test_stream_b() -> StreamId {
    StreamId::B
}

/// Create a test diagnostic
fn create_test_diagnostic() -> Diagnostic {
    Diagnostic {
        id: 1,
        severity: Severity::Error,
        stream_id: StreamId::A,
        message: "Test error".to_string(),
        category: Category::IO,
        offset_bytes: 1000,
        timestamp_ms: 1234567890,
        frame_index: Some(10),
        count: 1,
        impact_score: 80,
    }
}

// ============================================================================
// Event::ModelUpdated Tests
// ============================================================================

#[cfg(test)]
mod event_model_updated_tests {
    use super::*;

    #[test]
    fn test_event_model_updated_construct() {
        // Arrange
        let kind = ModelKind::Container;
        let stream = test_stream_a();

        // Act
        let event = Event::ModelUpdated { kind, stream };

        // Assert
        assert!(matches!(event, Event::ModelUpdated { .. }));
    }

    #[test]
    fn test_event_model_updated_clone() {
        // Arrange
        let event = Event::ModelUpdated {
            kind: ModelKind::Units,
            stream: test_stream_a(),
        };

        // Act
        let cloned = event.clone();

        // Assert
        assert!(matches!(cloned, Event::ModelUpdated { .. }));
    }

    #[test]
    fn test_event_model_updated_all_kinds() {
        // Arrange & Act - Test all model kinds
        let kinds = [
            ModelKind::Container,
            ModelKind::Units,
            ModelKind::Syntax,
            ModelKind::Timeline,
            ModelKind::Stats,
            ModelKind::Metrics,
        ];

        for kind in kinds {
            let event = Event::ModelUpdated {
                kind,
                stream: test_stream_a(),
            };
            assert!(matches!(event, Event::ModelUpdated { .. }));
        }
    }
}

// ============================================================================
// Event::SelectionUpdated Tests
// ============================================================================

#[cfg(test)]
mod event_selection_updated_tests {
    use super::*;

    #[test]
    fn test_event_selection_updated_construct() {
        // Arrange & Act
        let event = Event::SelectionUpdated {
            stream: test_stream_a(),
        };

        // Assert
        assert!(matches!(event, Event::SelectionUpdated { .. }));
    }

    #[test]
    fn test_event_selection_updated_clone() {
        // Arrange
        let event = Event::SelectionUpdated {
            stream: test_stream_b(),
        };

        // Act
        let cloned = event.clone();

        // Assert
        assert!(matches!(cloned, Event::SelectionUpdated { .. }));
    }
}

// ============================================================================
// Event::FrameDecoded Tests
// ============================================================================

#[cfg(test)]
mod event_frame_decoded_tests {
    use super::*;

    #[test]
    fn test_event_frame_decoded_construct() {
        // Arrange
        let stream = test_stream_a();
        let frame_index = 100usize;

        // Act
        let event = Event::FrameDecoded {
            stream,
            frame_index,
        };

        // Assert
        assert!(matches!(event, Event::FrameDecoded { .. }));
    }

    #[test]
    fn test_event_frame_decoded_extract_fields() {
        // Arrange
        let stream = test_stream_b();
        let frame_index = 50usize;

        // Act
        let event = Event::FrameDecoded {
            stream,
            frame_index,
        };

        // Assert
        match event {
            Event::FrameDecoded {
                stream: s,
                frame_index: fi,
            } => {
                assert_eq!(s, StreamId::B);
                assert_eq!(fi, 50);
            }
            _ => panic!("Expected FrameDecoded event"),
        }
    }
}

// ============================================================================
// Worker Events Tests
// ============================================================================

#[cfg(test)]
mod event_worker_tests {
    use super::*;

    #[test]
    fn test_event_worker_progress() {
        // Arrange
        let job_id = 123u64;
        let progress = 0.5f32;

        // Act
        let event = Event::WorkerProgress { job_id, progress };

        // Assert
        assert!(matches!(event, Event::WorkerProgress { .. }));
        match event {
            Event::WorkerProgress {
                job_id: id,
                progress: p,
            } => {
                assert_eq!(id, 123);
                assert_eq!(p, 0.5);
            }
            _ => panic!("Expected WorkerProgress event"),
        }
    }

    #[test]
    fn test_event_worker_finished() {
        // Arrange
        let job_id = 456u64;

        // Act
        let event = Event::WorkerFinished { job_id };

        // Assert
        assert!(matches!(event, Event::WorkerFinished { .. }));
    }

    #[test]
    fn test_event_worker_error() {
        // Arrange
        let job_id = 789u64;
        let error = "Test error message".to_string();

        // Act
        let event = Event::WorkerError { job_id, error };

        // Assert
        assert!(matches!(event, Event::WorkerError { .. }));
    }

    #[test]
    fn test_event_worker_progress_values() {
        // Arrange & Act - Test different progress values
        let progresses = [0.0f32, 0.25f32, 0.5f32, 0.75f32, 1.0f32];

        for progress in progresses {
            let event = Event::WorkerProgress {
                job_id: 1,
                progress,
            };
            assert!(matches!(event, Event::WorkerProgress { .. }));
        }
    }
}

// ============================================================================
// Diagnostic Events Tests
// ============================================================================

#[cfg(test)]
mod event_diagnostic_tests {
    use super::*;

    #[test]
    fn test_event_diagnostic_added() {
        // Arrange
        let diagnostic = create_test_diagnostic();

        // Act
        let event = Event::DiagnosticAdded { diagnostic };

        // Assert
        assert!(matches!(event, Event::DiagnosticAdded { .. }));
    }

    #[test]
    fn test_event_diagnostics_cleared() {
        // Arrange & Act
        let event = Event::DiagnosticsCleared {
            stream: test_stream_a(),
        };

        // Assert
        assert!(matches!(event, Event::DiagnosticsCleared { .. }));
    }

    #[test]
    fn test_event_diagnostic_extract_fields() {
        // Arrange
        let diagnostic = create_test_diagnostic();

        // Act
        let event = Event::DiagnosticAdded {
            diagnostic: diagnostic.clone(),
        };

        // Assert
        match event {
            Event::DiagnosticAdded { diagnostic: d } => {
                assert_eq!(d.id, 1);
                assert_eq!(d.severity, Severity::Error);
                assert_eq!(d.message, "Test error");
            }
            _ => panic!("Expected DiagnosticAdded event"),
        }
    }
}

// ============================================================================
// Export Events Tests
// ============================================================================

#[cfg(test)]
mod event_export_tests {
    use super::*;

    #[test]
    fn test_event_export_finished() {
        // Arrange
        let path = create_test_path();

        // Act
        let event = Event::ExportFinished { path };

        // Assert
        assert!(matches!(event, Event::ExportFinished { .. }));
    }

    #[test]
    fn test_event_export_failed() {
        // Arrange
        let error = "Export failed".to_string();

        // Act
        let event = Event::ExportFailed { error };

        // Assert
        assert!(matches!(event, Event::ExportFailed { .. }));
    }

    #[test]
    fn test_event_export_finished_extract_path() {
        // Arrange
        let path = PathBuf::from("/tmp/output.csv");

        // Act
        let event = Event::ExportFinished { path };

        // Assert
        match event {
            Event::ExportFinished { path: p } => {
                assert_eq!(p, PathBuf::from("/tmp/output.csv"));
            }
            _ => panic!("Expected ExportFinished event"),
        }
    }
}

// ============================================================================
// ModelKind Tests
// ============================================================================

#[cfg(test)]
mod model_kind_tests {
    use super::*;

    #[test]
    fn test_model_kind_values() {
        // Arrange & Act
        let kinds = [
            ModelKind::Container,
            ModelKind::Units,
            ModelKind::Syntax,
            ModelKind::Timeline,
            ModelKind::Stats,
            ModelKind::Metrics,
        ];

        // Assert - All values exist
        assert_eq!(kinds.len(), 6);
    }

    #[test]
    fn test_model_kind_copy() {
        // Arrange
        let kind = ModelKind::Container;

        // Act
        let copied = kind;

        // Assert - ModelKind is Copy
        assert_eq!(kind, ModelKind::Container);
        assert_eq!(copied, ModelKind::Container);
    }

    #[test]
    fn test_model_kind_equality() {
        // Arrange
        let container1 = ModelKind::Container;
        let container2 = ModelKind::Container;
        let units = ModelKind::Units;

        // Assert
        assert_eq!(container1, container2);
        assert_ne!(container1, units);
    }
}

// ============================================================================
// Diagnostic Struct Tests
// ============================================================================

#[cfg(test)]
mod diagnostic_struct_tests {
    use super::*;

    #[test]
    fn test_diagnostic_complete() {
        // Arrange & Act
        let diagnostic = Diagnostic {
            id: 1,
            severity: Severity::Error,
            stream_id: StreamId::A,
            message: "Test error".to_string(),
            category: Category::IO,
            offset_bytes: 1000,
            timestamp_ms: 1234567890,
            frame_index: Some(10),
            count: 1,
            impact_score: 80,
        };

        // Assert
        assert_eq!(diagnostic.id, 1);
        assert_eq!(diagnostic.severity, Severity::Error);
        assert_eq!(diagnostic.stream_id, StreamId::A);
        assert_eq!(diagnostic.message, "Test error");
        assert_eq!(diagnostic.category, Category::IO);
        assert_eq!(diagnostic.offset_bytes, 1000);
        assert_eq!(diagnostic.timestamp_ms, 1234567890);
        assert_eq!(diagnostic.frame_index, Some(10));
        assert_eq!(diagnostic.count, 1);
        assert_eq!(diagnostic.impact_score, 80);
    }

    #[test]
    fn test_diagnostic_minimal() {
        // Arrange & Act
        let diagnostic = Diagnostic {
            id: 2,
            severity: Severity::Info,
            stream_id: StreamId::B,
            message: "Info message".to_string(),
            category: Category::Worker,
            offset_bytes: 0,
            timestamp_ms: 0,
            frame_index: None,
            count: 0,
            impact_score: 0,
        };

        // Assert
        assert_eq!(diagnostic.id, 2);
        assert_eq!(diagnostic.severity, Severity::Info);
        assert_eq!(diagnostic.frame_index, None);
        assert_eq!(diagnostic.count, 0);
        assert_eq!(diagnostic.impact_score, 0);
    }

    #[test]
    fn test_diagnostic_clone() {
        // Arrange
        let diagnostic = create_test_diagnostic();

        // Act
        let cloned = diagnostic.clone();

        // Assert
        assert_eq!(cloned.id, diagnostic.id);
        assert_eq!(cloned.severity, diagnostic.severity);
        assert_eq!(cloned.message, diagnostic.message);
    }
}

// ============================================================================
// Severity Tests
// ============================================================================

#[cfg(test)]
mod severity_tests {
    use super::*;

    #[test]
    fn test_severity_values() {
        // Arrange & Act
        let severities = [
            Severity::Info,
            Severity::Warn,
            Severity::Error,
            Severity::Fatal,
        ];

        // Assert
        assert_eq!(severities.len(), 4);
    }

    #[test]
    fn test_severity_ordering() {
        // Arrange
        let info = Severity::Info;
        let warn = Severity::Warn;
        let error = Severity::Error;
        let fatal = Severity::Fatal;

        // Assert - Ord is derived, so ordering is by declaration order
        assert!(info < warn);
        assert!(warn < error);
        assert!(error < fatal);
        assert!(info < fatal);
    }

    #[test]
    fn test_severity_partial_ord() {
        // Arrange
        let error1 = Severity::Error;
        let error2 = Severity::Error;

        // Assert - PartialOrd should work
        assert!(error1 <= error2);
        assert!(error1 >= error2);
        assert_eq!(error1.partial_cmp(&error2), Some(std::cmp::Ordering::Equal));
    }
}

// ============================================================================
// Category Tests
// ============================================================================

#[cfg(test)]
mod category_tests {
    use super::*;

    #[test]
    fn test_category_values() {
        // Arrange & Act
        let categories = [
            Category::Container,
            Category::Bitstream,
            Category::Decode,
            Category::Metric,
            Category::IO,
            Category::Worker,
        ];

        // Assert
        assert_eq!(categories.len(), 6);
    }

    #[test]
    fn test_category_equality() {
        // Arrange
        let io1 = Category::IO;
        let io2 = Category::IO;
        let decode = Category::Decode;

        // Assert
        assert_eq!(io1, io2);
        assert_ne!(io1, decode);
    }
}

// ============================================================================
// Integration Tests
// ============================================================================

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_event_clone_all_variants() {
        // Arrange
        let events = vec![
            Event::ModelUpdated {
                kind: ModelKind::Container,
                stream: test_stream_a(),
            },
            Event::SelectionUpdated {
                stream: test_stream_b(),
            },
            Event::FrameDecoded {
                stream: test_stream_a(),
                frame_index: 10,
            },
            Event::WorkerProgress {
                job_id: 1,
                progress: 0.5,
            },
            Event::WorkerFinished { job_id: 2 },
            Event::WorkerError {
                job_id: 3,
                error: "Error".to_string(),
            },
            Event::DiagnosticAdded {
                diagnostic: create_test_diagnostic(),
            },
            Event::DiagnosticsCleared {
                stream: test_stream_a(),
            },
            Event::ExportFinished {
                path: create_test_path(),
            },
            Event::ExportFailed {
                error: "Failed".to_string(),
            },
        ];

        // Act & Assert - All events should be cloneable
        for event in events {
            let _cloned = event.clone();
            // If we got here without panic, clone succeeded
        }
    }

    #[test]
    fn test_diagnostic_with_all_extensions() {
        // Arrange
        let diagnostic = Diagnostic {
            id: 100,
            severity: Severity::Fatal,
            stream_id: StreamId::A,
            message: "Critical failure".to_string(),
            category: Category::Decode,
            offset_bytes: 5000,
            timestamp_ms: 9876543210,
            frame_index: Some(42),
            count: 5,
            impact_score: 100,
        };

        // Assert - All bitvue extensions are set
        assert_eq!(diagnostic.frame_index, Some(42));
        assert_eq!(diagnostic.count, 5);
        assert_eq!(diagnostic.impact_score, 100);
        assert_eq!(diagnostic.severity, Severity::Fatal);
    }
}

// ============================================================================
// Edge Cases
// ============================================================================

#[cfg(test)]
mod edge_case_tests {
    use super::*;

    #[test]
    fn test_diagnostic_zero_impact_score() {
        // Arrange & Act
        let diagnostic = Diagnostic {
            id: 1,
            severity: Severity::Info,
            stream_id: StreamId::A,
            message: "Low priority".to_string(),
            category: Category::Worker,
            offset_bytes: 0,
            timestamp_ms: 0,
            frame_index: None,
            count: 0,
            impact_score: 0,
        };

        // Assert
        assert_eq!(diagnostic.impact_score, 0);
    }

    #[test]
    fn test_diagnostic_max_impact_score() {
        // Arrange & Act
        let diagnostic = Diagnostic {
            id: 1,
            severity: Severity::Fatal,
            stream_id: StreamId::A,
            message: "Critical".to_string(),
            category: Category::IO,
            offset_bytes: 0,
            timestamp_ms: 0,
            frame_index: None,
            count: 1,
            impact_score: 100,
        };

        // Assert
        assert_eq!(diagnostic.impact_score, 100);
    }

    #[test]
    fn test_diagnostic_high_count() {
        // Arrange & Act
        let diagnostic = Diagnostic {
            id: 1,
            severity: Severity::Warn,
            stream_id: StreamId::A,
            message: "Repeated error".to_string(),
            category: Category::Bitstream,
            offset_bytes: 0,
            timestamp_ms: 0,
            frame_index: None,
            count: 1000,
            impact_score: 50,
        };

        // Assert
        assert_eq!(diagnostic.count, 1000);
    }

    #[test]
    fn test_frame_decoded_zero_index() {
        // Arrange & Act
        let event = Event::FrameDecoded {
            stream: test_stream_a(),
            frame_index: 0,
        };

        // Assert
        assert!(matches!(event, Event::FrameDecoded { .. }));
    }

    #[test]
    fn test_worker_progress_full() {
        // Arrange & Act
        let event = Event::WorkerProgress {
            job_id: 1,
            progress: 1.0,
        };

        // Assert
        assert!(matches!(event, Event::WorkerProgress { .. }));
    }
}
