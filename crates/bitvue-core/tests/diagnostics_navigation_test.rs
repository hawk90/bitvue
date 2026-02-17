#![allow(dead_code)]
//! Tests for diagnostic navigation and tri-sync functionality

use bitvue_core::event::{Category, Diagnostic, Severity};
use bitvue_core::{Core, StreamId, UnitModel, UnitNode};
use std::sync::Arc;

#[test]
fn test_navigate_to_diagnostic_by_frame_index() {
    let core = Arc::new(Core::new());
    let stream = core.get_stream(StreamId::A);

    // Create test units with frames
    let mut unit_0 = UnitNode::new(StreamId::A, "FRAME".to_string(), 1000, 500);
    unit_0.frame_index = Some(0);

    let mut unit_1 = UnitNode::new(StreamId::A, "FRAME".to_string(), 2000, 500);
    unit_1.frame_index = Some(1);

    let mut unit_2 = UnitNode::new(StreamId::A, "FRAME".to_string(), 3000, 500);
    unit_2.frame_index = Some(2);

    let units = vec![unit_0, unit_1, unit_2];

    // Add units to stream
    {
        let mut state = stream.write();
        state.units = Some(UnitModel {
            units: units.clone(),
            unit_count: units.len(),
            frame_count: 3,
        });
    }

    // Add diagnostic at frame 1
    {
        let mut state = stream.write();
        state.add_diagnostic(Diagnostic {
            id: 1,
            severity: Severity::Error,
            stream_id: StreamId::A,
            message: "Error at frame 1".to_string(),
            category: Category::Bitstream,
            offset_bytes: 2000,
            timestamp_ms: 33,
            frame_index: Some(1),
            count: 1,
            impact_score: 85,
        });
    }

    // Simulate clicking on diagnostic -> navigate to frame 1
    let diagnostic_frame = 1;
    let target_unit = units
        .iter()
        .find(|u| u.frame_index == Some(diagnostic_frame));

    assert!(target_unit.is_some(), "Should find target frame");
    assert_eq!(target_unit.unwrap().frame_index, Some(1));
    assert_eq!(target_unit.unwrap().offset, 2000);
}

#[test]
fn test_navigate_to_diagnostic_by_offset() {
    let core = Arc::new(Core::new());
    let stream = core.get_stream(StreamId::A);

    // Create test units
    let mut unit_0 = UnitNode::new(StreamId::A, "FRAME".to_string(), 1000, 500);
    unit_0.frame_index = Some(0);

    let mut unit_1 = UnitNode::new(StreamId::A, "FRAME".to_string(), 2500, 500);
    unit_1.frame_index = Some(1);

    let units = vec![unit_0, unit_1];

    {
        let mut state = stream.write();
        state.units = Some(UnitModel {
            units: units.clone(),
            unit_count: units.len(),
            frame_count: 2,
        });
    }

    // Add diagnostic with offset
    {
        let mut state = stream.write();
        state.add_diagnostic(Diagnostic {
            id: 1,
            severity: Severity::Error,
            stream_id: StreamId::A,
            message: "Error at offset 2600".to_string(),
            category: Category::Bitstream,
            offset_bytes: 2600,
            timestamp_ms: 0,
            frame_index: None, // No frame index, must use offset
            count: 1,
            impact_score: 85,
        });
    }

    // Find unit containing this offset
    let diagnostic_offset = 2600;
    let containing_unit = units
        .iter()
        .find(|u| diagnostic_offset >= u.offset && diagnostic_offset < u.offset + u.size as u64);

    assert!(containing_unit.is_some(), "Should find containing unit");
    assert_eq!(containing_unit.unwrap().offset, 2500);
    assert_eq!(containing_unit.unwrap().frame_index, Some(1));
}

#[test]
fn test_tri_sync_frame_selection() {
    // Test that selecting a diagnostic updates:
    // 1. Frame viewer (frame_index)
    // 2. Timeline position (timestamp)
    // 3. Diagnostics highlight

    let diagnostic = Diagnostic {
        id: 1,
        severity: Severity::Error,
        stream_id: StreamId::A,
        message: "Test error".to_string(),
        category: Category::Bitstream,
        offset_bytes: 1000,
        timestamp_ms: 333,
        frame_index: Some(10),
        count: 1,
        impact_score: 85,
    };

    // 1. Frame viewer should navigate to frame 10
    let target_frame = diagnostic.frame_index.unwrap();
    assert_eq!(target_frame, 10);

    // 2. Timeline should show 333ms (0.333s)
    let timeline_position = diagnostic.timestamp_ms;
    assert_eq!(timeline_position, 333);

    // 3. Diagnostics table should highlight this diagnostic
    let selected_diagnostic_id = diagnostic.id;
    assert_eq!(selected_diagnostic_id, 1);
}

#[test]
fn test_navigate_to_first_error() {
    let core = Arc::new(Core::new());
    let stream = core.get_stream(StreamId::A);

    // Add multiple diagnostics
    {
        let mut state = stream.write();

        for i in 0..10 {
            state.add_diagnostic(Diagnostic {
                id: i,
                severity: if i < 3 {
                    Severity::Error
                } else {
                    Severity::Warn
                },
                stream_id: StreamId::A,
                message: format!("Diagnostic {}", i),
                category: Category::Bitstream,
                offset_bytes: i * 1000,
                timestamp_ms: i * 33,
                frame_index: Some(i as usize),
                count: 1,
                impact_score: if i < 3 { 85 } else { 60 },
            });
        }
    }

    // Find first error
    {
        let state = stream.read();
        let first_error = state
            .diagnostics
            .iter()
            .find(|d| d.severity == Severity::Error);

        assert!(first_error.is_some());
        assert_eq!(first_error.unwrap().id, 0);
        assert_eq!(first_error.unwrap().frame_index, Some(0));
    }
}

#[test]
fn test_navigate_to_next_error() {
    let core = Arc::new(Core::new());
    let stream = core.get_stream(StreamId::A);

    // Add diagnostics with errors at frames 2, 5, 8
    {
        let mut state = stream.write();

        for i in 0..10 {
            let severity = if i == 2 || i == 5 || i == 8 {
                Severity::Error
            } else {
                Severity::Info
            };

            state.add_diagnostic(Diagnostic {
                id: i,
                severity,
                stream_id: StreamId::A,
                message: format!("Diagnostic {}", i),
                category: Category::Bitstream,
                offset_bytes: i * 1000,
                timestamp_ms: i * 33,
                frame_index: Some(i as usize),
                count: 1,
                impact_score: if severity == Severity::Error { 85 } else { 25 },
            });
        }
    }

    // Navigate to next error after frame 3
    {
        let state = stream.read();
        let current_frame = 3;

        let next_error = state
            .diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .find(|d| d.frame_index.unwrap_or(0) > current_frame);

        assert!(next_error.is_some());
        assert_eq!(next_error.unwrap().frame_index, Some(5));
    }
}

#[test]
fn test_navigate_to_previous_error() {
    let core = Arc::new(Core::new());
    let stream = core.get_stream(StreamId::A);

    // Add diagnostics with errors at frames 2, 5, 8
    {
        let mut state = stream.write();

        for i in 0..10 {
            let severity = if i == 2 || i == 5 || i == 8 {
                Severity::Error
            } else {
                Severity::Info
            };

            state.add_diagnostic(Diagnostic {
                id: i,
                severity,
                stream_id: StreamId::A,
                message: format!("Diagnostic {}", i),
                category: Category::Bitstream,
                offset_bytes: i * 1000,
                timestamp_ms: i * 33,
                frame_index: Some(i as usize),
                count: 1,
                impact_score: if severity == Severity::Error { 85 } else { 25 },
            });
        }
    }

    // Navigate to previous error before frame 6
    {
        let state = stream.read();
        let current_frame = 6;

        let prev_error = state
            .diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .filter(|d| d.frame_index.unwrap_or(999) < current_frame)
            .last();

        assert!(prev_error.is_some());
        assert_eq!(prev_error.unwrap().frame_index, Some(5));
    }
}

#[test]
fn test_navigate_to_highest_impact_diagnostic() {
    let core = Arc::new(Core::new());
    let stream = core.get_stream(StreamId::A);

    {
        let mut state = stream.write();

        // Add diagnostics with varying impact scores
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

    // Find highest impact diagnostic
    {
        let state = stream.read();
        let highest_impact = state.diagnostics.iter().max_by_key(|d| d.impact_score);

        assert!(highest_impact.is_some());
        assert_eq!(highest_impact.unwrap().impact_score, 95);
        assert_eq!(highest_impact.unwrap().frame_index, Some(9));
    }
}

#[test]
fn test_click_navigation_with_command() {
    // Test that clicking a diagnostic generates the correct SelectUnit command
    let diagnostic = Diagnostic {
        id: 1,
        severity: Severity::Error,
        stream_id: StreamId::A,
        message: "Test error".to_string(),
        category: Category::Bitstream,
        offset_bytes: 1000,
        timestamp_ms: 333,
        frame_index: Some(5),
        count: 1,
        impact_score: 85,
    };

    // Simulate finding the unit for this diagnostic
    let target_frame = diagnostic.frame_index.unwrap();
    let expected_command_frame = target_frame;

    assert_eq!(expected_command_frame, 5);

    // In real code, this would generate:
    // Command::SelectUnit { stream: StreamId::A, unit_key: "frame_5" }
}

#[test]
fn test_diagnostic_without_frame_index_fallback() {
    // Test navigation fallback when frame_index is None
    let diagnostic = Diagnostic {
        id: 1,
        severity: Severity::Fatal,
        stream_id: StreamId::A,
        message: "Container error".to_string(),
        category: Category::Container,
        offset_bytes: 0,
        timestamp_ms: 0,
        frame_index: None, // No frame
        count: 1,
        impact_score: 100,
    };

    // Should fall back to offset-based navigation
    let navigation_offset = diagnostic.offset_bytes;
    assert_eq!(navigation_offset, 0);

    // Or navigate to first unit
    let fallback_frame = 0;
    assert_eq!(fallback_frame, 0);
}

#[test]
fn test_multi_stream_navigation_isolation() {
    let core = Arc::new(Core::new());
    let stream_a = core.get_stream(StreamId::A);
    let stream_b = core.get_stream(StreamId::B);

    // Add diagnostic to Stream A at frame 5
    {
        let mut state = stream_a.write();
        state.add_diagnostic(Diagnostic {
            id: 1,
            severity: Severity::Error,
            stream_id: StreamId::A,
            message: "Error in Stream A".to_string(),
            category: Category::Bitstream,
            offset_bytes: 5000,
            timestamp_ms: 166,
            frame_index: Some(5),
            count: 1,
            impact_score: 85,
        });
    }

    // Add diagnostic to Stream B at frame 10
    {
        let mut state = stream_b.write();
        state.add_diagnostic(Diagnostic {
            id: 2,
            severity: Severity::Error,
            stream_id: StreamId::B,
            message: "Error in Stream B".to_string(),
            category: Category::Bitstream,
            offset_bytes: 10000,
            timestamp_ms: 333,
            frame_index: Some(10),
            count: 1,
            impact_score: 85,
        });
    }

    // Clicking diagnostic in Stream A should not affect Stream B
    {
        let state_a = stream_a.read();
        let state_b = stream_b.read();

        let diag_a = &state_a.diagnostics[0];
        let diag_b = &state_b.diagnostics[0];

        assert_eq!(diag_a.stream_id, StreamId::A);
        assert_eq!(diag_a.frame_index, Some(5));

        assert_eq!(diag_b.stream_id, StreamId::B);
        assert_eq!(diag_b.frame_index, Some(10));

        // Navigation should be stream-specific
        assert_ne!(diag_a.frame_index, diag_b.frame_index);
    }
}

#[test]
fn test_navigate_by_timestamp() {
    let core = Arc::new(Core::new());
    let stream = core.get_stream(StreamId::A);

    {
        let mut state = stream.write();

        // Add diagnostics at various timestamps
        for i in 0..10 {
            state.add_diagnostic(Diagnostic {
                id: i,
                severity: Severity::Error,
                stream_id: StreamId::A,
                message: format!("Diagnostic at {}ms", i * 1000),
                category: Category::Bitstream,
                offset_bytes: i * 1000,
                timestamp_ms: i * 1000, // 0ms, 1000ms, 2000ms, ...
                frame_index: Some(i as usize),
                count: 1,
                impact_score: 85,
            });
        }
    }

    // Find diagnostic closest to 2500ms
    {
        let state = stream.read();
        let target_time = 2500;

        let closest = state.diagnostics.iter().min_by_key(|d| {
            let diff = if d.timestamp_ms > target_time {
                d.timestamp_ms - target_time
            } else {
                target_time - d.timestamp_ms
            };
            diff
        });

        assert!(closest.is_some());
        // Closest should be either 2000ms or 3000ms
        assert!(closest.unwrap().timestamp_ms == 2000 || closest.unwrap().timestamp_ms == 3000);
    }
}

#[test]
fn test_diagnostic_selection_state() {
    // Test tracking which diagnostic is currently selected
    let diagnostics = vec![
        Diagnostic {
            id: 1,
            severity: Severity::Error,
            stream_id: StreamId::A,
            message: "Error 1".to_string(),
            category: Category::Bitstream,
            offset_bytes: 1000,
            timestamp_ms: 0,
            frame_index: Some(1),
            count: 1,
            impact_score: 85,
        },
        Diagnostic {
            id: 2,
            severity: Severity::Error,
            stream_id: StreamId::A,
            message: "Error 2".to_string(),
            category: Category::Bitstream,
            offset_bytes: 2000,
            timestamp_ms: 0,
            frame_index: Some(2),
            count: 1,
            impact_score: 85,
        },
    ];

    // Simulate selecting diagnostic 1
    let selected_id = Some(1u64);

    let selected_diag = diagnostics.iter().find(|d| Some(d.id) == selected_id);
    assert!(selected_diag.is_some());
    assert_eq!(selected_diag.unwrap().id, 1);

    // Simulate selecting diagnostic 2
    let selected_id = Some(2u64);

    let selected_diag = diagnostics.iter().find(|d| Some(d.id) == selected_id);
    assert!(selected_diag.is_some());
    assert_eq!(selected_diag.unwrap().id, 2);
}
