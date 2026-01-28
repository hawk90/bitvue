//! User workflow tests for Enhanced Diagnostics end-to-end scenarios

use bitvue_core::event::{Category, Diagnostic, Severity};
use bitvue_core::{Core, StreamId, UnitModel, UnitNode};
use std::sync::Arc;

#[test]
fn test_workflow_open_corrupt_file_and_navigate() {
    // Workflow: Open corrupt file → diagnostics populate → click diagnostic → navigate to frame
    let core = Arc::new(Core::new());
    let stream = core.get_stream(StreamId::A);

    // Step 1: Simulate file open with units
    let mut unit_0 = UnitNode::new(StreamId::A, "SEQUENCE_HEADER".to_string(), 0, 100);
    unit_0.frame_index = None;

    let mut unit_1 = UnitNode::new(StreamId::A, "FRAME".to_string(), 1000, 500);
    unit_1.frame_index = Some(0);

    let mut unit_2 = UnitNode::new(StreamId::A, "FRAME".to_string(), 2000, 500);
    unit_2.frame_index = Some(1);

    let mut unit_3 = UnitNode::new(StreamId::A, "FRAME".to_string(), 3000, 500);
    unit_3.frame_index = Some(2);

    let units = vec![unit_0, unit_1, unit_2, unit_3];

    {
        let mut state = stream.write();
        state.units = Some(UnitModel {
            units: units.clone(),
            unit_count: units.len(),
            frame_count: 3,
        });
    }

    // Step 2: Diagnostics populate during parse
    {
        let mut state = stream.write();

        state.add_diagnostic(Diagnostic {
            id: 0,
            severity: Severity::Error,
            stream_id: StreamId::A,
            message: "Invalid OBU type detected".to_string(),
            category: Category::Bitstream,
            offset_bytes: 2250,
            timestamp_ms: 33,
            frame_index: Some(1),
            count: 1,
            impact_score: 85,
        });

        state.add_diagnostic(Diagnostic {
            id: 1,
            severity: Severity::Fatal,
            stream_id: StreamId::A,
            message: "Unexpected end of file".to_string(),
            category: Category::Bitstream,
            offset_bytes: 3100,
            timestamp_ms: 66,
            frame_index: Some(2),
            count: 1,
            impact_score: 100,
        });
    }

    // Step 3: User sees diagnostics table with 2 errors
    {
        let state = stream.read();
        assert_eq!(state.diagnostics.len(), 2, "Should show 2 diagnostics");
    }

    // Step 4: User clicks first diagnostic (frame 1)
    let clicked_diagnostic = {
        let state = stream.read();
        state.diagnostics[0].clone()
    };

    assert_eq!(clicked_diagnostic.frame_index, Some(1));

    // Step 5: Navigate to frame 1
    let target_frame = clicked_diagnostic.frame_index.unwrap();
    let target_unit = units.iter().find(|u| u.frame_index == Some(target_frame));

    assert!(target_unit.is_some(), "Should find target unit");
    assert_eq!(target_unit.unwrap().offset, 2000);

    // Step 6: Frame viewer, timeline, and diagnostics table are all synced to frame 1
    let selected_frame = target_frame;
    let timeline_position = clicked_diagnostic.timestamp_ms;
    let highlighted_diagnostic = clicked_diagnostic.id;

    assert_eq!(selected_frame, 1);
    assert_eq!(timeline_position, 33);
    assert_eq!(highlighted_diagnostic, 0);
}

#[test]
fn test_workflow_filter_sort_navigate() {
    // Workflow: Filter diagnostics → sort by impact → navigate to highest
    let core = Arc::new(Core::new());
    let stream = core.get_stream(StreamId::A);

    // Add mixed diagnostics
    {
        let mut state = stream.write();

        for i in 0..10 {
            state.add_diagnostic(Diagnostic {
                id: i,
                severity: if i % 3 == 0 {
                    Severity::Error
                } else if i % 3 == 1 {
                    Severity::Warn
                } else {
                    Severity::Info
                },
                stream_id: StreamId::A,
                message: format!("Diagnostic {}", i),
                category: Category::Bitstream,
                offset_bytes: i * 1000,
                timestamp_ms: i * 33,
                frame_index: Some(i as usize),
                count: 1,
                impact_score: 50 + (i as u8 * 5),
            });
        }
    }

    // Step 1: Filter to only errors
    let filtered = {
        let state = stream.read();
        state
            .diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .cloned()
            .collect::<Vec<_>>()
    };

    assert_eq!(filtered.len(), 4, "Should have 4 errors (0, 3, 6, 9)");

    // Step 2: Sort filtered results by impact score (descending)
    let mut sorted = filtered;
    sorted.sort_by(|a, b| b.impact_score.cmp(&a.impact_score));

    assert_eq!(sorted[0].id, 9, "Highest impact error should be #9");
    assert_eq!(sorted[0].impact_score, 95);

    // Step 3: Navigate to highest impact error
    let target_diagnostic = &sorted[0];
    let target_frame = target_diagnostic.frame_index.unwrap();

    assert_eq!(target_frame, 9);
}

#[test]
fn test_workflow_multistream_comparison() {
    // Workflow: Compare diagnostics across two streams
    let core = Arc::new(Core::new());
    let stream_a = core.get_stream(StreamId::A);
    let stream_b = core.get_stream(StreamId::B);

    // Add diagnostics to Stream A
    {
        let mut state = stream_a.write();
        for i in 0..5 {
            state.add_diagnostic(Diagnostic {
                id: i,
                severity: Severity::Error,
                stream_id: StreamId::A,
                message: format!("Stream A error {}", i),
                category: Category::Bitstream,
                offset_bytes: i * 1000,
                timestamp_ms: i * 33,
                frame_index: Some(i as usize),
                count: 1,
                impact_score: 85,
            });
        }
    }

    // Add diagnostics to Stream B
    {
        let mut state = stream_b.write();
        for i in 0..3 {
            state.add_diagnostic(Diagnostic {
                id: i,
                severity: Severity::Warn,
                stream_id: StreamId::B,
                message: format!("Stream B warning {}", i),
                category: Category::Bitstream,
                offset_bytes: i * 1000,
                timestamp_ms: i * 33,
                frame_index: Some(i as usize),
                count: 1,
                impact_score: 60,
            });
        }
    }

    // Compare diagnostic counts
    let count_a = stream_a.read().diagnostics.len();
    let count_b = stream_b.read().diagnostics.len();

    assert_eq!(count_a, 5);
    assert_eq!(count_b, 3);
    assert!(count_a > count_b, "Stream A has more diagnostics");

    // Compare severity distribution
    let errors_a = stream_a
        .read()
        .diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .count();
    let errors_b = stream_b
        .read()
        .diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .count();

    assert_eq!(errors_a, 5);
    assert_eq!(errors_b, 0);
}

#[test]
fn test_workflow_burst_error_detection() {
    // Workflow: Detect burst errors and navigate to them
    let core = Arc::new(Core::new());
    let stream = core.get_stream(StreamId::A);

    {
        let mut state = stream.write();

        // Single errors
        state.add_diagnostic(Diagnostic {
            id: 0,
            severity: Severity::Error,
            stream_id: StreamId::A,
            message: "Single error".to_string(),
            category: Category::Bitstream,
            offset_bytes: 1000,
            timestamp_ms: 0,
            frame_index: Some(0),
            count: 1,
            impact_score: 80,
        });

        // Burst error (count > 1)
        state.add_diagnostic(Diagnostic {
            id: 1,
            severity: Severity::Error,
            stream_id: StreamId::A,
            message: "Burst error".to_string(),
            category: Category::Bitstream,
            offset_bytes: 2000,
            timestamp_ms: 33,
            frame_index: Some(1),
            count: 8,
            impact_score: 95,
        });

        state.add_diagnostic(Diagnostic {
            id: 2,
            severity: Severity::Error,
            stream_id: StreamId::A,
            message: "Another single error".to_string(),
            category: Category::Bitstream,
            offset_bytes: 3000,
            timestamp_ms: 66,
            frame_index: Some(2),
            count: 1,
            impact_score: 80,
        });
    }

    // Find all burst errors (count > 1)
    let bursts = {
        let state = stream.read();
        state
            .diagnostics
            .iter()
            .filter(|d| d.count > 1)
            .cloned()
            .collect::<Vec<_>>()
    };

    assert_eq!(bursts.len(), 1, "Should detect 1 burst");
    assert_eq!(bursts[0].id, 1);
    assert_eq!(bursts[0].count, 8);
    assert_eq!(bursts[0].impact_score, 95, "Burst should have high impact");

    // Navigate to burst
    let burst_frame = bursts[0].frame_index.unwrap();
    assert_eq!(burst_frame, 1);
}

#[test]
fn test_workflow_severity_filter_toggle() {
    // Workflow: Toggle severity filters to show/hide diagnostics
    let core = Arc::new(Core::new());
    let stream = core.get_stream(StreamId::A);

    {
        let mut state = stream.write();

        state.add_diagnostic(Diagnostic {
            id: 0,
            severity: Severity::Fatal,
            stream_id: StreamId::A,
            message: "Fatal".to_string(),
            category: Category::Bitstream,
            offset_bytes: 0,
            timestamp_ms: 0,
            frame_index: Some(0),
            count: 1,
            impact_score: 100,
        });

        state.add_diagnostic(Diagnostic {
            id: 1,
            severity: Severity::Error,
            stream_id: StreamId::A,
            message: "Error".to_string(),
            category: Category::Bitstream,
            offset_bytes: 1000,
            timestamp_ms: 33,
            frame_index: Some(1),
            count: 1,
            impact_score: 85,
        });

        state.add_diagnostic(Diagnostic {
            id: 2,
            severity: Severity::Warn,
            stream_id: StreamId::A,
            message: "Warning".to_string(),
            category: Category::Bitstream,
            offset_bytes: 2000,
            timestamp_ms: 66,
            frame_index: Some(2),
            count: 1,
            impact_score: 60,
        });

        state.add_diagnostic(Diagnostic {
            id: 3,
            severity: Severity::Info,
            stream_id: StreamId::A,
            message: "Info".to_string(),
            category: Category::Bitstream,
            offset_bytes: 3000,
            timestamp_ms: 99,
            frame_index: Some(3),
            count: 1,
            impact_score: 25,
        });
    }

    // Filter: Show only errors and fatal
    let show_fatal = true;
    let show_error = true;
    let show_warn = false;
    let show_info = false;

    let filtered = {
        let state = stream.read();
        state
            .diagnostics
            .iter()
            .filter(|d| {
                (show_fatal && d.severity == Severity::Fatal)
                    || (show_error && d.severity == Severity::Error)
                    || (show_warn && d.severity == Severity::Warn)
                    || (show_info && d.severity == Severity::Info)
            })
            .cloned()
            .collect::<Vec<_>>()
    };

    assert_eq!(
        filtered.len(),
        2,
        "Should show 2 diagnostics (fatal + error)"
    );
    assert_eq!(filtered[0].severity, Severity::Fatal);
    assert_eq!(filtered[1].severity, Severity::Error);
}

#[test]
fn test_workflow_clear_diagnostics_on_reparse() {
    // Workflow: Reparse file → clear old diagnostics → add new ones
    let core = Arc::new(Core::new());
    let stream = core.get_stream(StreamId::A);

    // Initial parse
    {
        let mut state = stream.write();
        for i in 0..5 {
            state.add_diagnostic(Diagnostic {
                id: i,
                severity: Severity::Error,
                stream_id: StreamId::A,
                message: format!("Old diagnostic {}", i),
                category: Category::Bitstream,
                offset_bytes: i * 1000,
                timestamp_ms: i * 33,
                frame_index: Some(i as usize),
                count: 1,
                impact_score: 85,
            });
        }
    }

    assert_eq!(stream.read().diagnostics.len(), 5);

    // Reparse: Clear old diagnostics
    {
        let mut state = stream.write();
        state.diagnostics.clear();
    }

    assert_eq!(stream.read().diagnostics.len(), 0);

    // Add new diagnostics from reparse
    {
        let mut state = stream.write();
        for i in 0..3 {
            state.add_diagnostic(Diagnostic {
                id: i,
                severity: Severity::Warn,
                stream_id: StreamId::A,
                message: format!("New diagnostic {}", i),
                category: Category::Bitstream,
                offset_bytes: i * 1000,
                timestamp_ms: i * 33,
                frame_index: Some(i as usize),
                count: 1,
                impact_score: 60,
            });
        }
    }

    assert_eq!(stream.read().diagnostics.len(), 3);

    let first_new = &stream.read().diagnostics[0];
    assert!(first_new.message.contains("New diagnostic"));
    assert_eq!(first_new.severity, Severity::Warn);
}

#[test]
fn test_workflow_jump_to_first_last_error() {
    // Workflow: Jump to first/last error shortcuts
    let core = Arc::new(Core::new());
    let stream = core.get_stream(StreamId::A);

    {
        let mut state = stream.write();
        for i in 0..10 {
            state.add_diagnostic(Diagnostic {
                id: i,
                severity: if i == 2 || i == 5 || i == 8 {
                    Severity::Error
                } else {
                    Severity::Info
                },
                stream_id: StreamId::A,
                message: format!("Diagnostic {}", i),
                category: Category::Bitstream,
                offset_bytes: i * 1000,
                timestamp_ms: i * 33,
                frame_index: Some(i as usize),
                count: 1,
                impact_score: if i == 2 || i == 5 || i == 8 { 85 } else { 25 },
            });
        }
    }

    // Jump to first error
    let first_error = {
        let state = stream.read();
        state
            .diagnostics
            .iter()
            .find(|d| d.severity == Severity::Error)
            .cloned()
    };

    assert!(first_error.is_some());
    assert_eq!(first_error.unwrap().frame_index, Some(2));

    // Jump to last error
    let last_error = {
        let state = stream.read();
        state
            .diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .last()
            .cloned()
    };

    assert!(last_error.is_some());
    assert_eq!(last_error.unwrap().frame_index, Some(8));
}

#[test]
fn test_workflow_frame_range_filtering() {
    // Workflow: Filter diagnostics by frame range (e.g., frames 10-20)
    let core = Arc::new(Core::new());
    let stream = core.get_stream(StreamId::A);

    {
        let mut state = stream.write();
        for i in 0..30 {
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

    // Filter frames 10-20
    let min_frame = 10;
    let max_frame = 20;

    let filtered = {
        let state = stream.read();
        state
            .diagnostics
            .iter()
            .filter(|d| {
                if let Some(frame) = d.frame_index {
                    frame >= min_frame && frame <= max_frame
                } else {
                    false
                }
            })
            .cloned()
            .collect::<Vec<_>>()
    };

    assert_eq!(
        filtered.len(),
        11,
        "Should have 11 diagnostics (10-20 inclusive)"
    );
    assert_eq!(filtered.first().unwrap().frame_index, Some(10));
    assert_eq!(filtered.last().unwrap().frame_index, Some(20));
}

#[test]
fn test_workflow_impact_threshold_filtering() {
    // Workflow: Filter diagnostics by impact threshold (e.g., >= 80)
    let core = Arc::new(Core::new());
    let stream = core.get_stream(StreamId::A);

    {
        let mut state = stream.write();
        for i in 0..10 {
            state.add_diagnostic(Diagnostic {
                id: i,
                severity: Severity::Error,
                stream_id: StreamId::A,
                message: format!("Diagnostic {}", i),
                category: Category::Bitstream,
                offset_bytes: i * 1000,
                timestamp_ms: i * 33,
                frame_index: Some(i as usize),
                count: 1,
                impact_score: 50 + (i as u8 * 5), // 50, 55, 60, ..., 95
            });
        }
    }

    // Filter impact >= 80
    let min_impact = 80;

    let high_impact = {
        let state = stream.read();
        state
            .diagnostics
            .iter()
            .filter(|d| d.impact_score >= min_impact)
            .cloned()
            .collect::<Vec<_>>()
    };

    assert_eq!(
        high_impact.len(),
        4,
        "Should have 4 high-impact diagnostics (80, 85, 90, 95)"
    );

    for diag in &high_impact {
        assert!(diag.impact_score >= min_impact);
    }
}

#[test]
fn test_workflow_category_filtering() {
    // Workflow: Filter diagnostics by category
    let core = Arc::new(Core::new());
    let stream = core.get_stream(StreamId::A);

    {
        let mut state = stream.write();

        state.add_diagnostic(Diagnostic {
            id: 0,
            severity: Severity::Error,
            stream_id: StreamId::A,
            message: "Container error".to_string(),
            category: Category::Container,
            offset_bytes: 0,
            timestamp_ms: 0,
            frame_index: Some(0),
            count: 1,
            impact_score: 90,
        });

        state.add_diagnostic(Diagnostic {
            id: 1,
            severity: Severity::Error,
            stream_id: StreamId::A,
            message: "Bitstream error".to_string(),
            category: Category::Bitstream,
            offset_bytes: 1000,
            timestamp_ms: 33,
            frame_index: Some(1),
            count: 1,
            impact_score: 85,
        });

        state.add_diagnostic(Diagnostic {
            id: 2,
            severity: Severity::Warn,
            stream_id: StreamId::A,
            message: "Metric warning".to_string(),
            category: Category::Metric,
            offset_bytes: 2000,
            timestamp_ms: 66,
            frame_index: Some(2),
            count: 1,
            impact_score: 60,
        });
    }

    // Filter to only Bitstream category
    let bitstream_only = {
        let state = stream.read();
        state
            .diagnostics
            .iter()
            .filter(|d| d.category == Category::Bitstream)
            .cloned()
            .collect::<Vec<_>>()
    };

    assert_eq!(bitstream_only.len(), 1);
    assert_eq!(bitstream_only[0].message, "Bitstream error");
}

#[test]
fn test_workflow_keyboard_navigation_next_prev() {
    // Workflow: Navigate diagnostics with keyboard (next/previous)
    let core = Arc::new(Core::new());
    let stream = core.get_stream(StreamId::A);

    {
        let mut state = stream.write();
        for i in 0..5 {
            state.add_diagnostic(Diagnostic {
                id: i,
                severity: Severity::Error,
                stream_id: StreamId::A,
                message: format!("Error {}", i),
                category: Category::Bitstream,
                offset_bytes: i * 1000,
                timestamp_ms: i * 33,
                frame_index: Some(i as usize),
                count: 1,
                impact_score: 85,
            });
        }
    }

    // Start at first diagnostic
    let mut current_index = 0usize;

    // Press "Next" (Down arrow or J)
    current_index = (current_index + 1).min(4);
    assert_eq!(current_index, 1);

    // Press "Next" again
    current_index = (current_index + 1).min(4);
    assert_eq!(current_index, 2);

    // Press "Previous" (Up arrow or K)
    current_index = current_index.saturating_sub(1);
    assert_eq!(current_index, 1);

    // Verify selected diagnostic
    let selected = &stream.read().diagnostics[current_index];
    assert_eq!(selected.id, 1);
    assert_eq!(selected.frame_index, Some(1));
}

#[test]
fn test_workflow_search_in_messages() {
    // Workflow: Search for text in diagnostic messages
    let core = Arc::new(Core::new());
    let stream = core.get_stream(StreamId::A);

    {
        let mut state = stream.write();

        state.add_diagnostic(Diagnostic {
            id: 0,
            severity: Severity::Error,
            stream_id: StreamId::A,
            message: "Invalid OBU type detected".to_string(),
            category: Category::Bitstream,
            offset_bytes: 1000,
            timestamp_ms: 0,
            frame_index: Some(0),
            count: 1,
            impact_score: 85,
        });

        state.add_diagnostic(Diagnostic {
            id: 1,
            severity: Severity::Error,
            stream_id: StreamId::A,
            message: "Unexpected end of file".to_string(),
            category: Category::Bitstream,
            offset_bytes: 2000,
            timestamp_ms: 33,
            frame_index: Some(1),
            count: 1,
            impact_score: 100,
        });

        state.add_diagnostic(Diagnostic {
            id: 2,
            severity: Severity::Warn,
            stream_id: StreamId::A,
            message: "Missing reference frame".to_string(),
            category: Category::Decode,
            offset_bytes: 3000,
            timestamp_ms: 66,
            frame_index: Some(2),
            count: 1,
            impact_score: 70,
        });
    }

    // Search for "OBU"
    let search_query = "OBU";
    let search_results = {
        let state = stream.read();
        state
            .diagnostics
            .iter()
            .filter(|d| {
                d.message
                    .to_lowercase()
                    .contains(&search_query.to_lowercase())
            })
            .cloned()
            .collect::<Vec<_>>()
    };

    assert_eq!(search_results.len(), 1);
    assert_eq!(search_results[0].id, 0);

    // Search for "frame"
    let search_query = "frame";
    let search_results = {
        let state = stream.read();
        state
            .diagnostics
            .iter()
            .filter(|d| {
                d.message
                    .to_lowercase()
                    .contains(&search_query.to_lowercase())
            })
            .cloned()
            .collect::<Vec<_>>()
    };

    assert_eq!(search_results.len(), 1);
    assert_eq!(search_results[0].id, 2);
}
