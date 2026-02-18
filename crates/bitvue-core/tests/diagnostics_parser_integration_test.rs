#![allow(hidden_glob_reexports)]
#![allow(unreachable_code)]
#![allow(non_camel_case_types)]
#![allow(unused_assignments)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(unused_comparisons)]
#![allow(unused_doc_comments)]
//! End-to-end tests for diagnostics parser integration with StreamState

use bitvue_core::event::{Category, Diagnostic, Severity};
use bitvue_core::{Core, StreamId};
use std::sync::Arc;

#[test]
fn test_add_diagnostics_to_stream_state() {
    let core = Arc::new(Core::new());
    let stream = core.get_stream(StreamId::A);

    // Create test diagnostics
    let diagnostics = vec![
        Diagnostic {
            id: 1,
            severity: Severity::Error,
            stream_id: StreamId::A,
            message: "Test error 1".to_string(),
            category: Category::Bitstream,
            offset_bytes: 1000,
            timestamp_ms: 0,
            frame_index: Some(10),
            count: 1,
            impact_score: 85,
        },
        Diagnostic {
            id: 2,
            severity: Severity::Fatal,
            stream_id: StreamId::A,
            message: "Test error 2".to_string(),
            category: Category::Bitstream,
            offset_bytes: 2000,
            timestamp_ms: 333,
            frame_index: Some(20),
            count: 1,
            impact_score: 100,
        },
    ];

    // Add diagnostics to stream state
    {
        let mut state = stream.write();
        for diagnostic in diagnostics {
            state.add_diagnostic(diagnostic);
        }
    }

    // Verify diagnostics were added
    {
        let state = stream.read();
        assert_eq!(state.diagnostics.len(), 2, "Should have 2 diagnostics");

        // Verify first diagnostic
        assert_eq!(state.diagnostics[0].id, 1);
        assert_eq!(state.diagnostics[0].severity, Severity::Error);
        assert_eq!(state.diagnostics[0].frame_index, Some(10));
        assert_eq!(state.diagnostics[0].impact_score, 85);

        // Verify second diagnostic
        assert_eq!(state.diagnostics[1].id, 2);
        assert_eq!(state.diagnostics[1].severity, Severity::Fatal);
        assert_eq!(state.diagnostics[1].frame_index, Some(20));
        assert_eq!(state.diagnostics[1].impact_score, 100);
    }
}

#[test]
fn test_multiple_streams_diagnostic_isolation() {
    let core = Arc::new(Core::new());
    let stream_a = core.get_stream(StreamId::A);
    let stream_b = core.get_stream(StreamId::B);

    // Add diagnostic to Stream A
    {
        let mut state = stream_a.write();
        state.add_diagnostic(Diagnostic {
            id: 1,
            severity: Severity::Error,
            stream_id: StreamId::A,
            message: "Error in Stream A".to_string(),
            category: Category::Bitstream,
            offset_bytes: 1000,
            timestamp_ms: 0,
            frame_index: Some(5),
            count: 1,
            impact_score: 80,
        });
    }

    // Add diagnostic to Stream B
    {
        let mut state = stream_b.write();
        state.add_diagnostic(Diagnostic {
            id: 2,
            severity: Severity::Warn,
            stream_id: StreamId::B,
            message: "Warning in Stream B".to_string(),
            category: Category::Bitstream,
            offset_bytes: 2000,
            timestamp_ms: 100,
            frame_index: Some(10),
            count: 1,
            impact_score: 60,
        });
    }

    // Verify isolation
    {
        let state_a = stream_a.read();
        let state_b = stream_b.read();

        assert_eq!(
            state_a.diagnostics.len(),
            1,
            "Stream A should have 1 diagnostic"
        );
        assert_eq!(
            state_b.diagnostics.len(),
            1,
            "Stream B should have 1 diagnostic"
        );

        assert_eq!(state_a.diagnostics[0].stream_id, StreamId::A);
        assert_eq!(state_b.diagnostics[0].stream_id, StreamId::B);
    }
}

#[test]
fn test_diagnostic_filtering_by_severity() {
    let core = Arc::new(Core::new());
    let stream = core.get_stream(StreamId::A);

    // Add diagnostics with different severities
    {
        let mut state = stream.write();

        // Add 3 errors
        for i in 0..3 {
            state.add_diagnostic(Diagnostic {
                id: i,
                severity: Severity::Error,
                stream_id: StreamId::A,
                message: format!("Error {}", i),
                category: Category::Bitstream,
                offset_bytes: i * 1000,
                timestamp_ms: 0,
                frame_index: Some(i as usize),
                count: 1,
                impact_score: 85,
            });
        }

        // Add 2 warnings
        for i in 3..5 {
            state.add_diagnostic(Diagnostic {
                id: i,
                severity: Severity::Warn,
                stream_id: StreamId::A,
                message: format!("Warning {}", i),
                category: Category::Bitstream,
                offset_bytes: i * 1000,
                timestamp_ms: 0,
                frame_index: Some(i as usize),
                count: 1,
                impact_score: 60,
            });
        }

        // Add 1 fatal
        state.add_diagnostic(Diagnostic {
            id: 5,
            severity: Severity::Fatal,
            stream_id: StreamId::A,
            message: "Fatal error".to_string(),
            category: Category::Bitstream,
            offset_bytes: 5000,
            timestamp_ms: 0,
            frame_index: Some(5),
            count: 1,
            impact_score: 100,
        });
    }

    // Filter by severity
    {
        let state = stream.read();

        let errors = state.diagnostics_by_severity(Severity::Error);
        let warnings = state.diagnostics_by_severity(Severity::Warn);
        let fatals = state.diagnostics_by_severity(Severity::Fatal);

        assert_eq!(errors.len(), 3, "Should have 3 errors");
        assert_eq!(warnings.len(), 2, "Should have 2 warnings");
        assert_eq!(fatals.len(), 1, "Should have 1 fatal");
    }
}

#[test]
fn test_diagnostic_high_impact_filtering() {
    let core = Arc::new(Core::new());
    let stream = core.get_stream(StreamId::A);

    {
        let mut state = stream.write();

        // Add high impact diagnostics (>= 80)
        for i in 0..3 {
            state.add_diagnostic(Diagnostic {
                id: i,
                severity: Severity::Error,
                stream_id: StreamId::A,
                message: format!("High impact {}", i),
                category: Category::Bitstream,
                offset_bytes: i * 1000,
                timestamp_ms: 0,
                frame_index: Some(i as usize),
                count: 1,
                impact_score: 85 + (i as u8 * 5),
            });
        }

        // Add low impact diagnostics (< 80)
        for i in 3..6 {
            state.add_diagnostic(Diagnostic {
                id: i,
                severity: Severity::Warn,
                stream_id: StreamId::A,
                message: format!("Low impact {}", i),
                category: Category::Bitstream,
                offset_bytes: i * 1000,
                timestamp_ms: 0,
                frame_index: Some(i as usize),
                count: 1,
                impact_score: 50 + ((i - 3) as u8 * 5),
            });
        }
    }

    // Filter by high impact
    {
        let state = stream.read();

        let high_impact: Vec<_> = state
            .diagnostics
            .iter()
            .filter(|d| d.impact_score >= 80)
            .collect();

        assert_eq!(
            high_impact.len(),
            3,
            "Should have 3 high impact diagnostics"
        );

        for diag in high_impact {
            assert!(
                diag.impact_score >= 80,
                "Impact score should be >= 80, got {}",
                diag.impact_score
            );
        }
    }
}

#[test]
fn test_diagnostic_frame_range_filtering() {
    let core = Arc::new(Core::new());
    let stream = core.get_stream(StreamId::A);

    {
        let mut state = stream.write();

        // Add diagnostics across frames 0-99
        for i in 0..100 {
            state.add_diagnostic(Diagnostic {
                id: i,
                severity: Severity::Error,
                stream_id: StreamId::A,
                message: format!("Error at frame {}", i),
                category: Category::Bitstream,
                offset_bytes: i * 1000,
                timestamp_ms: i * 33,
                frame_index: Some(i as usize),
                count: 1,
                impact_score: 85,
            });
        }
    }

    // Filter frames 10-19
    {
        let state = stream.read();

        let range_10_20: Vec<_> = state
            .diagnostics
            .iter()
            .filter(|d| {
                if let Some(frame) = d.frame_index {
                    frame >= 10 && frame < 20
                } else {
                    false
                }
            })
            .collect();

        assert_eq!(
            range_10_20.len(),
            10,
            "Should have 10 diagnostics in frame range 10-19"
        );

        for diag in range_10_20 {
            let frame = diag.frame_index.unwrap();
            assert!(
                frame >= 10 && frame < 20,
                "Frame {} should be in range [10, 20)",
                frame
            );
        }
    }
}

#[test]
fn test_diagnostic_clear() {
    let core = Arc::new(Core::new());
    let stream = core.get_stream(StreamId::A);

    // Add diagnostics
    {
        let mut state = stream.write();
        for i in 0..10 {
            state.add_diagnostic(Diagnostic {
                id: i,
                severity: Severity::Error,
                stream_id: StreamId::A,
                message: format!("Error {}", i),
                category: Category::Bitstream,
                offset_bytes: i * 1000,
                timestamp_ms: 0,
                frame_index: Some(i as usize),
                count: 1,
                impact_score: 85,
            });
        }
    }

    // Verify added
    {
        let state = stream.read();
        assert_eq!(state.diagnostics.len(), 10);
    }

    // Clear diagnostics
    {
        let mut state = stream.write();
        state.clear_diagnostics();
    }

    // Verify cleared
    {
        let state = stream.read();
        assert_eq!(state.diagnostics.len(), 0, "Diagnostics should be cleared");
    }
}

#[test]
fn test_diagnostic_sorting_by_impact() {
    let core = Arc::new(Core::new());
    let stream = core.get_stream(StreamId::A);

    {
        let mut state = stream.write();

        // Add diagnostics with varying impact scores (not in order)
        let impacts = vec![50, 90, 70, 100, 60, 85, 95, 55];
        for (i, impact) in impacts.iter().enumerate() {
            state.add_diagnostic(Diagnostic {
                id: i as u64,
                severity: Severity::Error,
                stream_id: StreamId::A,
                message: format!("Error with impact {}", impact),
                category: Category::Bitstream,
                offset_bytes: i as u64 * 1000,
                timestamp_ms: 0,
                frame_index: Some(i),
                count: 1,
                impact_score: *impact,
            });
        }
    }

    // Sort by impact (descending)
    {
        let state = stream.read();
        let mut sorted: Vec<_> = state.diagnostics.iter().collect();
        sorted.sort_by_key(|d| std::cmp::Reverse(d.impact_score));

        // Verify sorted order
        for i in 1..sorted.len() {
            assert!(
                sorted[i - 1].impact_score >= sorted[i].impact_score,
                "Diagnostics should be sorted by impact score (descending)"
            );
        }

        // First should be highest impact
        assert_eq!(sorted[0].impact_score, 100);
        // Last should be lowest impact
        assert_eq!(sorted[sorted.len() - 1].impact_score, 50);
    }
}

#[test]
fn test_diagnostic_count_errors_and_warnings() {
    let core = Arc::new(Core::new());
    let stream = core.get_stream(StreamId::A);

    {
        let mut state = stream.write();

        // Add 5 errors
        for i in 0..5 {
            state.add_diagnostic(Diagnostic {
                id: i,
                severity: Severity::Error,
                stream_id: StreamId::A,
                message: format!("Error {}", i),
                category: Category::Bitstream,
                offset_bytes: i * 1000,
                timestamp_ms: 0,
                frame_index: Some(i as usize),
                count: 1,
                impact_score: 85,
            });
        }

        // Add 3 warnings
        for i in 5..8 {
            state.add_diagnostic(Diagnostic {
                id: i,
                severity: Severity::Warn,
                stream_id: StreamId::A,
                message: format!("Warning {}", i),
                category: Category::Bitstream,
                offset_bytes: i * 1000,
                timestamp_ms: 0,
                frame_index: Some(i as usize),
                count: 1,
                impact_score: 60,
            });
        }

        // Add 1 fatal
        state.add_diagnostic(Diagnostic {
            id: 8,
            severity: Severity::Fatal,
            stream_id: StreamId::A,
            message: "Fatal".to_string(),
            category: Category::Bitstream,
            offset_bytes: 8000,
            timestamp_ms: 0,
            frame_index: Some(8),
            count: 1,
            impact_score: 100,
        });

        // Add 2 info
        for i in 9..11 {
            state.add_diagnostic(Diagnostic {
                id: i,
                severity: Severity::Info,
                stream_id: StreamId::A,
                message: format!("Info {}", i),
                category: Category::Bitstream,
                offset_bytes: i * 1000,
                timestamp_ms: 0,
                frame_index: Some(i as usize),
                count: 1,
                impact_score: 25,
            });
        }
    }

    // Count by severity
    {
        let state = stream.read();

        let error_count = state
            .diagnostics
            .iter()
            .filter(|d| matches!(d.severity, Severity::Error))
            .count();
        let warning_count = state
            .diagnostics
            .iter()
            .filter(|d| matches!(d.severity, Severity::Warn))
            .count();
        let fatal_count = state
            .diagnostics
            .iter()
            .filter(|d| matches!(d.severity, Severity::Fatal))
            .count();
        let info_count = state
            .diagnostics
            .iter()
            .filter(|d| matches!(d.severity, Severity::Info))
            .count();

        assert_eq!(error_count, 5, "Should have 5 errors");
        assert_eq!(warning_count, 3, "Should have 3 warnings");
        assert_eq!(fatal_count, 1, "Should have 1 fatal");
        assert_eq!(info_count, 2, "Should have 2 info");
    }
}

#[test]
fn test_diagnostic_with_burst_count() {
    let core = Arc::new(Core::new());
    let stream = core.get_stream(StreamId::A);

    {
        let mut state = stream.write();

        // Add diagnostic with burst count
        state.add_diagnostic(Diagnostic {
            id: 1,
            severity: Severity::Error,
            stream_id: StreamId::A,
            message: "Repeated error (burst)".to_string(),
            category: Category::Bitstream,
            offset_bytes: 1000,
            timestamp_ms: 0,
            frame_index: Some(10),
            count: 5, // Burst of 5 errors
            impact_score: 90,
        });

        // Add normal diagnostic
        state.add_diagnostic(Diagnostic {
            id: 2,
            severity: Severity::Error,
            stream_id: StreamId::A,
            message: "Single error".to_string(),
            category: Category::Bitstream,
            offset_bytes: 2000,
            timestamp_ms: 0,
            frame_index: Some(20),
            count: 1,
            impact_score: 85,
        });
    }

    {
        let state = stream.read();

        // Filter burst errors (count > 1)
        let burst_errors: Vec<_> = state.diagnostics.iter().filter(|d| d.count > 1).collect();

        assert_eq!(burst_errors.len(), 1, "Should have 1 burst error");
        assert_eq!(burst_errors[0].count, 5);
        assert_eq!(burst_errors[0].message, "Repeated error (burst)");
    }
}

#[test]
fn test_diagnostic_timestamp_calculation() {
    let core = Arc::new(Core::new());
    let stream = core.get_stream(StreamId::A);

    {
        let mut state = stream.write();

        // Add diagnostics with timestamps at 30fps
        // Frame 0 = 0ms, Frame 30 = 1000ms, Frame 60 = 2000ms
        for frame in vec![0, 30, 60, 90] {
            let timestamp_ms = (frame as u64 * 1000) / 30; // 30fps

            state.add_diagnostic(Diagnostic {
                id: frame as u64,
                severity: Severity::Error,
                stream_id: StreamId::A,
                message: format!("Error at frame {}", frame),
                category: Category::Bitstream,
                offset_bytes: frame as u64 * 1000,
                timestamp_ms,
                frame_index: Some(frame),
                count: 1,
                impact_score: 85,
            });
        }
    }

    {
        let state = stream.read();

        // Verify timestamps
        assert_eq!(state.diagnostics[0].timestamp_ms, 0); // Frame 0
        assert_eq!(state.diagnostics[1].timestamp_ms, 1000); // Frame 30
        assert_eq!(state.diagnostics[2].timestamp_ms, 2000); // Frame 60
        assert_eq!(state.diagnostics[3].timestamp_ms, 3000); // Frame 90
    }
}
