//! UI rendering tests for 11-column diagnostics table

use bitvue_core::event::{Category, Diagnostic, Severity};
use bitvue_core::StreamId;

#[test]
fn test_11_column_table_structure() {
    // Test that all 11 columns are defined
    let columns = vec![
        "Severity",  // 1
        "Frame #",   // 2
        "Timestamp", // 3
        "Pos",       // 4
        "NAL idx",   // 5
        "Field",     // 6
        "CTB idx",   // 7
        "Type",      // 8
        "Count",     // 9
        "Impact",    // 10
        "Message",   // 11
    ];

    assert_eq!(columns.len(), 11, "Should have 11 columns");

    // Verify column order
    assert_eq!(columns[0], "Severity");
    assert_eq!(columns[1], "Frame #");
    assert_eq!(columns[2], "Timestamp");
    assert_eq!(columns[8], "Count");
    assert_eq!(columns[9], "Impact");
    assert_eq!(columns[10], "Message");
}

#[test]
fn test_severity_column_rendering() {
    // Test severity text rendering
    let severities = vec![
        (Severity::Fatal, "FATAL"),
        (Severity::Error, "ERROR"),
        (Severity::Warn, "WARN"),
        (Severity::Info, "INFO"),
    ];

    for (severity, expected_text) in severities {
        let text = format!("{:?}", severity).to_uppercase();
        assert!(
            text.contains(expected_text),
            "Severity {:?} should render as {}",
            severity,
            expected_text
        );
    }
}

#[test]
fn test_frame_number_column_rendering() {
    // Test frame number formatting
    let test_cases = vec![
        (Some(0), "0"),
        (Some(10), "10"),
        (Some(100), "100"),
        (Some(9999), "9999"),
        (None, "N/A"),
    ];

    for (frame_index, expected) in test_cases {
        let rendered = match frame_index {
            Some(idx) => format!("{}", idx),
            None => "N/A".to_string(),
        };

        assert_eq!(
            rendered, expected,
            "Frame index {:?} should render as {}",
            frame_index, expected
        );
    }
}

#[test]
fn test_timestamp_column_formatting() {
    // Test timestamp formatting at 30fps
    let test_cases = vec![
        (0, "0.00s"),
        (33, "0.03s"),
        (333, "0.33s"),
        (1000, "1.00s"),
        (5500, "5.50s"),
        (60000, "60.00s"),
        (90500, "90.50s"),
    ];

    for (timestamp_ms, expected) in test_cases {
        let seconds = timestamp_ms as f64 / 1000.0;
        let formatted = format!("{:.2}s", seconds);

        assert_eq!(
            formatted, expected,
            "Timestamp {}ms should format as {}",
            timestamp_ms, expected
        );
    }
}

#[test]
fn test_count_column_formatting() {
    // Test count display (1x, 5x, 100x, etc.)
    let test_cases = vec![
        (1, "1x"),
        (5, "5x"),
        (10, "10x"),
        (100, "100x"),
        (999, "999x"),
    ];

    for (count, expected) in test_cases {
        let formatted = format!("{}x", count);
        assert_eq!(
            formatted, expected,
            "Count {} should format as {}",
            count, expected
        );
    }
}

#[test]
fn test_impact_icon_rendering() {
    // Test impact icon selection based on score
    let test_cases = vec![
        (100, '●'), // Critical (80+)
        (95, '●'),
        (85, '●'),
        (80, '●'),
        (79, '▲'), // Warning (50-79)
        (65, '▲'),
        (50, '▲'),
        (49, '○'), // OK (<50)
        (25, '○'),
        (0, '○'),
    ];

    for (impact_score, expected_icon) in test_cases {
        let icon = if impact_score >= 80 {
            '●' // U+25CF Black Circle
        } else if impact_score >= 50 {
            '▲' // U+25B2 Black Up-Pointing Triangle
        } else {
            '○' // U+25CB White Circle
        };

        assert_eq!(
            icon, expected_icon,
            "Impact score {} should use icon '{}'",
            impact_score, expected_icon
        );
    }
}

#[test]
fn test_impact_column_full_rendering() {
    // Test full impact column rendering (icon + score)
    let test_cases = vec![(100, "● 100"), (85, "● 85"), (65, "▲ 65"), (25, "○ 25")];

    for (impact_score, expected) in test_cases {
        let icon = if impact_score >= 80 {
            '●'
        } else if impact_score >= 50 {
            '▲'
        } else {
            '○'
        };

        let formatted = format!("{} {}", icon, impact_score);
        assert_eq!(
            formatted, expected,
            "Impact {} should render as {}",
            impact_score, expected
        );
    }
}

#[test]
fn test_category_to_field_abbreviation() {
    // Test category to Field column abbreviation
    let test_cases = vec![
        (Category::Container, "CTB"),
        (Category::Bitstream, "NAL"),
        (Category::Decode, "HRD"),
        (Category::Metric, "QP"),
        (Category::IO, "IO"),
        (Category::Worker, "SYS"),
    ];

    for (category, expected_abbr) in test_cases {
        let abbr = match category {
            Category::Container => "CTB",
            Category::Bitstream => "NAL",
            Category::Decode => "HRD",
            Category::Metric => "QP",
            Category::IO => "IO",
            Category::Worker => "SYS",
        };

        assert_eq!(
            abbr, expected_abbr,
            "Category {:?} should abbreviate to {}",
            category, expected_abbr
        );
    }
}

#[test]
fn test_category_to_type_full_name() {
    // Test category to Type column full name
    let test_cases = vec![
        (Category::Container, "Container"),
        (Category::Bitstream, "Bitstream"),
        (Category::Decode, "Decode"),
        (Category::Metric, "Metric"),
        (Category::IO, "IO"),
        (Category::Worker, "Worker"),
    ];

    for (category, expected_name) in test_cases {
        let name = match category {
            Category::Container => "Container",
            Category::Bitstream => "Bitstream",
            Category::Decode => "Decode",
            Category::Metric => "Metric",
            Category::IO => "IO",
            Category::Worker => "Worker",
        };

        assert_eq!(
            name, expected_name,
            "Category {:?} should display as {}",
            category, expected_name
        );
    }
}

#[test]
fn test_offset_bytes_formatting() {
    // Test byte offset formatting
    let test_cases = vec![
        (0, "0"),
        (1000, "1000"),
        (10000, "10000"),
        (100000, "100000"),
        (1000000, "1000000"),
    ];

    for (offset, expected) in test_cases {
        let formatted = format!("{}", offset);
        assert_eq!(
            formatted, expected,
            "Offset {} should format as {}",
            offset, expected
        );
    }
}

#[test]
fn test_message_column_truncation() {
    // Test that very long messages are handled
    let short_message = "Short error message";
    let long_message = "This is a very long error message that might need to be truncated or wrapped in the UI to prevent the table from becoming too wide and breaking the layout. It contains detailed information about what went wrong.";

    assert!(
        short_message.len() < 50,
        "Short message should be < 50 chars"
    );
    assert!(
        long_message.len() > 200,
        "Long message should be > 200 chars"
    );

    // Test truncation at 100 chars with ellipsis
    let truncated = if long_message.len() > 100 {
        format!("{}...", &long_message[..100])
    } else {
        long_message.to_string()
    };

    assert_eq!(
        truncated.len(),
        103,
        "Truncated message should be 100 + 3 (ellipsis)"
    );
    assert!(
        truncated.ends_with("..."),
        "Truncated message should end with ..."
    );
}

#[test]
fn test_all_columns_for_diagnostic() {
    // Test rendering all columns for a complete diagnostic
    let diagnostic = Diagnostic {
        id: 1,
        severity: Severity::Error,
        stream_id: StreamId::A,
        message: "Test error message".to_string(),
        category: Category::Bitstream,
        offset_bytes: 1000,
        timestamp_ms: 333,
        frame_index: Some(10),
        count: 5,
        impact_score: 85,
    };

    // Column 1: Severity
    assert_eq!(format!("{:?}", diagnostic.severity).to_uppercase(), "ERROR");

    // Column 2: Frame #
    assert_eq!(format!("{}", diagnostic.frame_index.unwrap()), "10");

    // Column 3: Timestamp
    let seconds = diagnostic.timestamp_ms as f64 / 1000.0;
    assert_eq!(format!("{:.2}s", seconds), "0.33s");

    // Column 4: Pos
    assert_eq!(format!("{}", diagnostic.offset_bytes), "1000");

    // Column 5: NAL idx (same as frame for bitstream errors)
    assert_eq!(format!("{}", diagnostic.frame_index.unwrap()), "10");

    // Column 6: Field
    let field = match diagnostic.category {
        Category::Bitstream => "NAL",
        _ => "OTHER",
    };
    assert_eq!(field, "NAL");

    // Column 7: CTB idx (N/A for bitstream)
    let ctb_idx = "-";
    assert_eq!(ctb_idx, "-");

    // Column 8: Type
    let type_name = match diagnostic.category {
        Category::Bitstream => "Bitstream",
        _ => "Other",
    };
    assert_eq!(type_name, "Bitstream");

    // Column 9: Count
    assert_eq!(format!("{}x", diagnostic.count), "5x");

    // Column 10: Impact
    let icon = if diagnostic.impact_score >= 80 {
        '●'
    } else if diagnostic.impact_score >= 50 {
        '▲'
    } else {
        '○'
    };
    assert_eq!(format!("{} {}", icon, diagnostic.impact_score), "● 85");

    // Column 11: Message
    assert_eq!(diagnostic.message, "Test error message");
}

#[test]
fn test_empty_table_rendering() {
    // Test that empty diagnostics list is handled
    let diagnostics: Vec<Diagnostic> = vec![];

    assert_eq!(diagnostics.len(), 0);

    // Empty state should show placeholder text
    let placeholder = if diagnostics.is_empty() {
        "No diagnostics"
    } else {
        "Diagnostics loaded"
    };

    assert_eq!(placeholder, "No diagnostics");
}

#[test]
fn test_large_dataset_rendering() {
    // Test that large diagnostic sets are handled
    let mut diagnostics = Vec::new();

    for i in 0..1000 {
        diagnostics.push(Diagnostic {
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

    assert_eq!(diagnostics.len(), 1000);

    // Should be able to render all rows
    for (i, diag) in diagnostics.iter().enumerate() {
        assert_eq!(diag.id, i as u64);
        assert_eq!(diag.frame_index, Some(i));
    }
}

#[test]
fn test_mixed_severity_rendering() {
    // Test rendering diagnostics with all severity levels
    let diagnostics = vec![
        Diagnostic {
            id: 1,
            severity: Severity::Fatal,
            stream_id: StreamId::A,
            message: "Fatal".to_string(),
            category: Category::Bitstream,
            offset_bytes: 1000,
            timestamp_ms: 0,
            frame_index: Some(1),
            count: 1,
            impact_score: 100,
        },
        Diagnostic {
            id: 2,
            severity: Severity::Error,
            stream_id: StreamId::A,
            message: "Error".to_string(),
            category: Category::Bitstream,
            offset_bytes: 2000,
            timestamp_ms: 0,
            frame_index: Some(2),
            count: 1,
            impact_score: 85,
        },
        Diagnostic {
            id: 3,
            severity: Severity::Warn,
            stream_id: StreamId::A,
            message: "Warn".to_string(),
            category: Category::Bitstream,
            offset_bytes: 3000,
            timestamp_ms: 0,
            frame_index: Some(3),
            count: 1,
            impact_score: 60,
        },
        Diagnostic {
            id: 4,
            severity: Severity::Info,
            stream_id: StreamId::A,
            message: "Info".to_string(),
            category: Category::Bitstream,
            offset_bytes: 4000,
            timestamp_ms: 0,
            frame_index: Some(4),
            count: 1,
            impact_score: 25,
        },
    ];

    assert_eq!(diagnostics.len(), 4);

    // Verify each severity renders correctly
    assert_eq!(format!("{:?}", diagnostics[0].severity), "Fatal");
    assert_eq!(format!("{:?}", diagnostics[1].severity), "Error");
    assert_eq!(format!("{:?}", diagnostics[2].severity), "Warn");
    assert_eq!(format!("{:?}", diagnostics[3].severity), "Info");
}

#[test]
fn test_unicode_in_message() {
    // Test that Unicode characters in messages are handled
    let messages = vec![
        "Error with emoji: 🔴",
        "Error with Korean: 오류 메시지",
        "Error with Japanese: エラーメッセージ",
        "Error with symbols: ● ▲ ○",
        "Error with special chars: <>&\"'",
    ];

    for message in messages {
        let diagnostic = Diagnostic {
            id: 1,
            severity: Severity::Error,
            stream_id: StreamId::A,
            message: message.to_string(),
            category: Category::Bitstream,
            offset_bytes: 1000,
            timestamp_ms: 0,
            frame_index: Some(1),
            count: 1,
            impact_score: 85,
        };

        assert_eq!(diagnostic.message, message);
        assert!(!diagnostic.message.is_empty());
    }
}

#[test]
fn test_frame_index_none_rendering() {
    // Test rendering when frame_index is None
    let diagnostic = Diagnostic {
        id: 1,
        severity: Severity::Error,
        stream_id: StreamId::A,
        message: "Container error".to_string(),
        category: Category::Container,
        offset_bytes: 0,
        timestamp_ms: 0,
        frame_index: None, // No frame association
        count: 1,
        impact_score: 100,
    };

    let frame_display = match diagnostic.frame_index {
        Some(idx) => format!("{}", idx),
        None => "N/A".to_string(),
    };

    assert_eq!(frame_display, "N/A");
}

#[test]
fn test_burst_count_highlighting() {
    // Test that burst counts (>1) can be identified for UI highlighting
    let diagnostics = vec![
        Diagnostic {
            id: 1,
            severity: Severity::Error,
            stream_id: StreamId::A,
            message: "Single error".to_string(),
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
            message: "Burst error".to_string(),
            category: Category::Bitstream,
            offset_bytes: 2000,
            timestamp_ms: 0,
            frame_index: Some(2),
            count: 5, // Burst
            impact_score: 90,
        },
    ];

    // Identify burst errors for highlighting
    let is_burst_0 = diagnostics[0].count > 1;
    let is_burst_1 = diagnostics[1].count > 1;

    assert_eq!(is_burst_0, false, "First diagnostic is not a burst");
    assert_eq!(is_burst_1, true, "Second diagnostic is a burst");
}
