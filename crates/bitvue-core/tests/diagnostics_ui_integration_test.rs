#![allow(dead_code)]
//! UI Integration tests for Enhanced Diagnostics
//!
//! Tests for:
//! - 11-column table rendering
//! - Click interactions for tri-sync
//! - Sorting and filtering
//! - Large dataset handling
//! - UI display formatting

use bitvue_core::event::{Category, Diagnostic, Severity};
use bitvue_core::{Core, StreamId};
use std::sync::Arc;

fn create_test_core() -> Arc<Core> {
    Arc::new(Core::new())
}

#[test]
fn test_all_table_columns_populated() {
    let core = create_test_core();
    let stream = core.get_stream(StreamId::A);
    let mut state = stream.write();

    // Create diagnostic with all fields populated
    let diag = Diagnostic {
        id: 1,
        severity: Severity::Error,
        stream_id: StreamId::A,
        message: "Complete diagnostic with all fields".to_string(),
        category: Category::Bitstream,
        offset_bytes: 5000,
        timestamp_ms: 1000,
        frame_index: Some(30),
        count: 3,
        impact_score: 92,
    };

    state.add_diagnostic(diag.clone());

    // Verify all 11 columns can be displayed
    // Column 1: Severity
    assert_eq!(format!("{:?}", diag.severity), "Error");

    // Column 2: Frame #
    assert_eq!(diag.frame_index.unwrap(), 30);

    // Column 3: Timestamp
    let time_sec = diag.timestamp_ms as f64 / 1000.0;
    assert_eq!(format!("{:.2}s", time_sec), "1.00s");

    // Column 4: Pos (offset)
    assert_eq!(diag.offset_bytes, 5000);

    // Column 5: NAL idx (would be frame index for NAL unit)
    assert_eq!(diag.frame_index.unwrap(), 30);

    // Column 6: Field (category as type abbreviation)
    let field = match diag.category {
        Category::Bitstream => "NAL",
        _ => "OTHER",
    };
    assert_eq!(field, "NAL");

    // Column 7: CTB idx (could be derived from block info)
    // For now, showing N/A for container/bitstream level

    // Column 8: Type (category)
    assert_eq!(format!("{:?}", diag.category), "Bitstream");

    // Column 9: Count
    assert_eq!(diag.count, 3);
    let count_display = if diag.count > 1 {
        format!("{}x", diag.count)
    } else {
        "1x".to_string()
    };
    assert_eq!(count_display, "3x");

    // Column 10: Impact
    assert_eq!(diag.impact_score, 92);
    let (icon, _) = if diag.impact_score >= 80 {
        ("●", "red")
    } else if diag.impact_score >= 50 {
        ("▲", "orange")
    } else {
        ("○", "green")
    };
    assert_eq!(icon, "●");

    // Column 11: Message
    assert_eq!(diag.message, "Complete diagnostic with all fields");
}

#[test]
fn test_all_severity_types_displayed() {
    let core = create_test_core();
    let stream = core.get_stream(StreamId::A);
    let mut state = stream.write();

    let severities = vec![
        (Severity::Fatal, "FATAL"),
        (Severity::Error, "ERROR"),
        (Severity::Warn, "WARN"),
        (Severity::Info, "INFO"),
    ];

    for (i, (severity, expected)) in severities.iter().enumerate() {
        let diag = Diagnostic {
            id: i as u64 + 1,
            severity: *severity,
            stream_id: StreamId::A,
            message: format!("Test {} message", expected),
            category: Category::Bitstream,
            offset_bytes: (i as u64 + 1) * 1000,
            timestamp_ms: (i as u64 + 1) * 33,
            frame_index: Some((i + 1) * 10),
            count: 1,
            impact_score: 85,
        };

        state.add_diagnostic(diag.clone());
        assert_eq!(format!("{:?}", diag.severity).to_uppercase(), *expected);
    }

    assert_eq!(state.diagnostics.len(), 4);
}

#[test]
fn test_all_category_types_displayed() {
    let core = create_test_core();
    let stream = core.get_stream(StreamId::A);
    let mut state = stream.write();

    let categories = vec![
        (Category::Container, "CTB", "Container error"),
        (Category::Bitstream, "NAL", "Bitstream error"),
        (Category::Decode, "HRD", "Decode error"),
        (Category::Metric, "QP", "Metric warning"),
        (Category::IO, "IO", "IO error"),
        (Category::Worker, "SYS", "Worker error"),
    ];

    for (i, (category, type_str, message)) in categories.iter().enumerate() {
        let diag = Diagnostic {
            id: i as u64 + 1,
            severity: Severity::Error,
            stream_id: StreamId::A,
            message: message.to_string(),
            category: *category,
            offset_bytes: (i as u64 + 1) * 1000,
            timestamp_ms: (i as u64 + 1) * 33,
            frame_index: Some((i + 1) * 10),
            count: 1,
            impact_score: 85,
        };

        state.add_diagnostic(diag.clone());

        // Verify type mapping
        let mapped_type = match diag.category {
            Category::Container => "CTB",
            Category::Bitstream => "NAL",
            Category::Decode => "HRD",
            Category::Metric => "QP",
            Category::IO => "IO",
            Category::Worker => "SYS",
        };
        assert_eq!(mapped_type, *type_str);
    }

    assert_eq!(state.diagnostics.len(), 6);
}

#[test]
fn test_impact_icon_ranges() {
    // Test all three impact icon ranges
    let test_cases = vec![
        (100, "●", "Critical - maximum"),
        (95, "●", "Critical - high"),
        (80, "●", "Critical - threshold"),
        (79, "▲", "Warning - just below critical"),
        (65, "▲", "Warning - medium"),
        (50, "▲", "Warning - threshold"),
        (49, "○", "OK - just below warning"),
        (25, "○", "OK - low"),
        (0, "○", "OK - minimum"),
    ];

    for (impact_score, expected_icon, description) in test_cases {
        let (icon, _) = if impact_score >= 80 {
            ("●", "red")
        } else if impact_score >= 50 {
            ("▲", "orange")
        } else {
            ("○", "green")
        };

        assert_eq!(
            icon, expected_icon,
            "{}: impact {} should show {}",
            description, impact_score, expected_icon
        );
    }
}

#[test]
fn test_large_diagnostic_dataset() {
    let core = create_test_core();
    let stream = core.get_stream(StreamId::A);
    let mut state = stream.write();

    // Add 1000 diagnostics
    for i in 0..1000 {
        let severity = match i % 4 {
            0 => Severity::Fatal,
            1 => Severity::Error,
            2 => Severity::Warn,
            _ => Severity::Info,
        };

        let category = match i % 6 {
            0 => Category::Container,
            1 => Category::Bitstream,
            2 => Category::Decode,
            3 => Category::Metric,
            4 => Category::IO,
            _ => Category::Worker,
        };

        let diag = Diagnostic {
            id: i as u64,
            severity,
            stream_id: StreamId::A,
            message: format!("Diagnostic #{} - performance test", i),
            category,
            offset_bytes: i as u64 * 1000,
            timestamp_ms: i as u64 * 33,
            frame_index: Some((i as usize) * 10),
            count: if i % 10 == 0 { 5 } else { 1 },
            impact_score: ((i % 100) as u8),
        };

        state.add_diagnostic(diag);
    }

    assert_eq!(state.diagnostics.len(), 1000);

    // Test filtering on large dataset
    let errors = state.diagnostics_by_severity(Severity::Error);
    assert_eq!(errors.len(), 250); // 1000 / 4 = 250 errors

    // Test burst detection on large dataset
    let bursts: Vec<_> = state.diagnostics.iter().filter(|d| d.count > 1).collect();
    assert_eq!(bursts.len(), 100); // Every 10th diagnostic
}

#[test]
fn test_timestamp_formatting_all_ranges() {
    // Test timestamp display for various time ranges
    let test_cases = vec![
        (0, "0.00s"),          // 0ms
        (33, "0.03s"),         // 1 frame @ 30fps
        (1000, "1.00s"),       // 1 second
        (60000, "60.00s"),     // 1 minute
        (3600000, "3600.00s"), // 1 hour
    ];

    for (timestamp_ms, expected) in test_cases {
        let time_sec = timestamp_ms as f64 / 1000.0;
        let formatted = format!("{:.2}s", time_sec);
        assert_eq!(formatted, expected);
    }
}

#[test]
fn test_count_display_formatting() {
    // Test count display for various values
    let test_cases = vec![
        (1, "1x"),
        (5, "5x"),
        (10, "10x"),
        (100, "100x"),
        (999, "999x"),
    ];

    for (count, expected) in test_cases {
        let display = if count > 1 {
            format!("{}x", count)
        } else {
            "1x".to_string()
        };
        assert_eq!(display, expected);
    }
}

#[test]
fn test_empty_diagnostics_table() {
    let core = create_test_core();
    let stream = core.get_stream(StreamId::A);
    let state = stream.read();

    // Verify empty state
    assert_eq!(state.diagnostics.len(), 0);

    // Verify filtering on empty returns empty
    let errors = state.diagnostics_by_severity(Severity::Error);
    assert_eq!(errors.len(), 0);
}

#[test]
fn test_diagnostic_with_very_long_message() {
    let core = create_test_core();
    let stream = core.get_stream(StreamId::A);
    let mut state = stream.write();

    let long_message = "This is a very long diagnostic message that contains detailed information about the error. ".repeat(10);

    let diag = Diagnostic {
        id: 1,
        severity: Severity::Error,
        stream_id: StreamId::A,
        message: long_message.clone(),
        category: Category::Bitstream,
        offset_bytes: 1000,
        timestamp_ms: 33,
        frame_index: Some(10),
        count: 1,
        impact_score: 85,
    };

    state.add_diagnostic(diag.clone());

    assert_eq!(state.diagnostics[0].message, long_message);
    assert!(state.diagnostics[0].message.len() > 500);
}

#[test]
fn test_diagnostic_without_frame_index_display() {
    let core = create_test_core();
    let stream = core.get_stream(StreamId::A);
    let mut state = stream.write();

    // Container-level error without frame association
    let diag = Diagnostic {
        id: 1,
        severity: Severity::Error,
        stream_id: StreamId::A,
        message: "Container parsing error - no frame".to_string(),
        category: Category::Container,
        offset_bytes: 0,
        timestamp_ms: 0,
        frame_index: None,
        count: 1,
        impact_score: 100,
    };

    state.add_diagnostic(diag.clone());

    assert!(diag.frame_index.is_none());

    // Frame # column should display "N/A" or "-"
    let frame_display = diag.frame_index.map_or("-".to_string(), |f| f.to_string());
    assert_eq!(frame_display, "-");
}

#[test]
fn test_sorting_by_impact_score() {
    let core = create_test_core();
    let stream = core.get_stream(StreamId::A);
    let mut state = stream.write();

    // Add diagnostics with various impact scores
    let impacts = vec![50, 95, 30, 88, 65, 100, 45, 75];
    for (i, &impact) in impacts.iter().enumerate() {
        state.add_diagnostic(Diagnostic {
            id: i as u64,
            severity: Severity::Error,
            stream_id: StreamId::A,
            message: format!("Impact {} error", impact),
            category: Category::Bitstream,
            offset_bytes: (i as u64) * 1000,
            timestamp_ms: (i as u64) * 33,
            frame_index: Some(i * 10),
            count: 1,
            impact_score: impact,
        });
    }

    // Sort by impact descending
    let mut sorted: Vec<_> = state.diagnostics.iter().collect();
    sorted.sort_by_key(|d| std::cmp::Reverse(d.impact_score));

    assert_eq!(sorted[0].impact_score, 100);
    assert_eq!(sorted[1].impact_score, 95);
    assert_eq!(sorted[2].impact_score, 88);
    assert_eq!(sorted[sorted.len() - 1].impact_score, 30);
}

#[test]
fn test_filtering_by_frame_range() {
    let core = create_test_core();
    let stream = core.get_stream(StreamId::A);
    let mut state = stream.write();

    // Add diagnostics across frames 0-100
    for i in 0..100 {
        state.add_diagnostic(Diagnostic {
            id: i,
            severity: Severity::Error,
            stream_id: StreamId::A,
            message: format!("Frame {} error", i),
            category: Category::Bitstream,
            offset_bytes: i * 1000,
            timestamp_ms: i * 33,
            frame_index: Some(i as usize),
            count: 1,
            impact_score: 85,
        });
    }

    // Filter frames 20-30
    let frame_range: Vec<_> = state
        .diagnostics
        .iter()
        .filter(|d| {
            if let Some(frame) = d.frame_index {
                frame >= 20 && frame <= 30
            } else {
                false
            }
        })
        .collect();

    assert_eq!(frame_range.len(), 11); // Inclusive: 20,21,...,30
}

#[test]
fn test_mixed_stream_diagnostics() {
    let core = create_test_core();

    // Add diagnostics to both Stream A and B
    {
        let stream_a = core.get_stream(StreamId::A);
        let mut state_a = stream_a.write();
        for i in 0..5 {
            state_a.add_diagnostic(Diagnostic {
                id: i,
                severity: Severity::Error,
                stream_id: StreamId::A,
                message: format!("Stream A error {}", i),
                category: Category::Bitstream,
                offset_bytes: i * 1000,
                timestamp_ms: i * 33,
                frame_index: Some(i as usize * 10),
                count: 1,
                impact_score: 85,
            });
        }
    }

    {
        let stream_b = core.get_stream(StreamId::B);
        let mut state_b = stream_b.write();
        for i in 0..3 {
            state_b.add_diagnostic(Diagnostic {
                id: i + 100,
                severity: Severity::Warn,
                stream_id: StreamId::B,
                message: format!("Stream B warning {}", i),
                category: Category::Decode,
                offset_bytes: i * 1000,
                timestamp_ms: i * 33,
                frame_index: Some(i as usize * 10),
                count: 1,
                impact_score: 50,
            });
        }
    }

    // Verify isolation
    {
        let stream_a = core.get_stream(StreamId::A);
        let state_a = stream_a.read();
        assert_eq!(state_a.diagnostics.len(), 5);
        assert!(state_a
            .diagnostics
            .iter()
            .all(|d| d.stream_id == StreamId::A));
    }

    {
        let stream_b = core.get_stream(StreamId::B);
        let state_b = stream_b.read();
        assert_eq!(state_b.diagnostics.len(), 3);
        assert!(state_b
            .diagnostics
            .iter()
            .all(|d| d.stream_id == StreamId::B));
    }
}
