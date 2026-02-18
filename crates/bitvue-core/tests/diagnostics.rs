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
//! Tests for diagnostics module

use bitvue_core::diagnostics::{
    DegradeMode, Diagnostic, DiagnosticCategory, DiagnosticJumpTarget, DiagnosticSeverity,
    DiagnosticsFilter, DiagnosticsManager, DiagnosticsPanel, DiagnosticsSortColumn,
    DiagnosticsSummary, FeatureDegradeState, SeverityCounts,
};
use bitvue_core::{FrameKey, StreamId};

#[test]
fn test_diagnostic_severity_ordering() {
    assert!(DiagnosticSeverity::Fatal > DiagnosticSeverity::Error);
    assert!(DiagnosticSeverity::Error > DiagnosticSeverity::Warn);
    assert!(DiagnosticSeverity::Warn > DiagnosticSeverity::Info);
}

#[test]
fn test_diagnostic_creation() {
    let diag = Diagnostic::new(
        1,
        DiagnosticSeverity::Error,
        StreamId::A,
        "Parse error".to_string(),
        DiagnosticCategory::Bitstream,
        0x1234,
    );

    assert_eq!(diag.id, 1);
    assert_eq!(diag.severity, DiagnosticSeverity::Error);
    assert_eq!(diag.offset_bytes, 0x1234);
}

#[test]
fn test_diagnostic_with_details() {
    let diag = Diagnostic::new(
        1,
        DiagnosticSeverity::Warn,
        StreamId::A,
        "Warning".to_string(),
        DiagnosticCategory::Decode,
        0,
    )
    .with_bit_range(100, 116)
    .with_codec("AV1".to_string())
    .with_detail("expected".to_string(), "5".to_string());

    assert_eq!(diag.bit_range, Some((100, 116)));
    assert_eq!(diag.codec, Some("AV1".to_string()));
    assert_eq!(diag.details.get("expected"), Some(&"5".to_string()));
}

#[test]
fn test_diagnostics_manager() {
    let mut manager = DiagnosticsManager::new();

    let id1 = manager.add_diagnostic(
        DiagnosticSeverity::Error,
        StreamId::A,
        "Error 1".to_string(),
        DiagnosticCategory::Bitstream,
        100,
    );

    let id2 = manager.add_diagnostic(
        DiagnosticSeverity::Warn,
        StreamId::A,
        "Warning 1".to_string(),
        DiagnosticCategory::Decode,
        200,
    );

    assert_eq!(id1, 1);
    assert_eq!(id2, 2);
    assert_eq!(manager.diagnostics.len(), 2);
}

#[test]
fn test_filter_by_severity() {
    let mut manager = DiagnosticsManager::new();

    manager.add_diagnostic(
        DiagnosticSeverity::Info,
        StreamId::A,
        "Info".to_string(),
        DiagnosticCategory::Bitstream,
        0,
    );

    manager.add_diagnostic(
        DiagnosticSeverity::Error,
        StreamId::A,
        "Error".to_string(),
        DiagnosticCategory::Bitstream,
        0,
    );

    manager.add_diagnostic(
        DiagnosticSeverity::Fatal,
        StreamId::A,
        "Fatal".to_string(),
        DiagnosticCategory::Bitstream,
        0,
    );

    let errors = manager.filter_by_severity(DiagnosticSeverity::Error);
    assert_eq!(errors.len(), 2); // Error + Fatal
}

#[test]
fn test_severity_counts() {
    let mut manager = DiagnosticsManager::new();

    manager.add_diagnostic(
        DiagnosticSeverity::Warn,
        StreamId::A,
        "W1".to_string(),
        DiagnosticCategory::Bitstream,
        0,
    );

    manager.add_diagnostic(
        DiagnosticSeverity::Error,
        StreamId::A,
        "E1".to_string(),
        DiagnosticCategory::Bitstream,
        0,
    );

    manager.add_diagnostic(
        DiagnosticSeverity::Error,
        StreamId::A,
        "E2".to_string(),
        DiagnosticCategory::Bitstream,
        0,
    );

    let counts = manager.count_by_severity();
    assert_eq!(counts.warn, 1);
    assert_eq!(counts.error, 2);
    assert_eq!(counts.total(), 3);
    assert!(counts.has_issues());
}

#[test]
fn test_status_bar_text() {
    let counts = SeverityCounts {
        info: 5,
        warn: 2,
        error: 1,
        fatal: 0,
    };

    assert_eq!(counts.status_bar_text(), "W:2 E:1 F:0");

    let no_issues = SeverityCounts::default();
    assert_eq!(no_issues.status_bar_text(), "No issues");
}

#[test]
fn test_degrade_state() {
    let available = FeatureDegradeState::available("Diff Overlay");
    assert_eq!(available.mode, DegradeMode::Available);

    let degraded = FeatureDegradeState::degraded("Timeline", "PTS quality: WARN")
        .with_action("Use frame index fallback");

    assert_eq!(degraded.mode, DegradeMode::Degraded);
    assert!(degraded.message().contains("Degraded"));
    assert!(degraded.message().contains("PTS quality"));

    let unavailable = FeatureDegradeState::unavailable("Diff Overlay", "Resolution mismatch >5%");

    assert_eq!(unavailable.mode, DegradeMode::Unavailable);
    assert!(unavailable.message().contains("Unavailable"));
}

#[test]
fn test_last_error_summary() {
    let mut manager = DiagnosticsManager::new();

    manager.add_diagnostic(
        DiagnosticSeverity::Info,
        StreamId::A,
        "Info message".to_string(),
        DiagnosticCategory::Bitstream,
        0,
    );

    assert!(manager.last_error_summary().is_none());

    manager.add_diagnostic(
        DiagnosticSeverity::Error,
        StreamId::A,
        "Error message".to_string(),
        DiagnosticCategory::Decode,
        100,
    );

    let summary = manager.last_error_summary();
    assert!(summary.is_some());
    assert!(summary.unwrap().contains("Error message"));
}

// DiagnosticsPanel tests

fn create_test_diagnostics() -> DiagnosticsManager {
    let mut manager = DiagnosticsManager::new();

    // Add various diagnostics
    let id1 = manager.add_diagnostic(
        DiagnosticSeverity::Info,
        StreamId::A,
        "Info: Frame analysis complete".to_string(),
        DiagnosticCategory::Bitstream,
        0,
    );

    // Add frame key to first diagnostic
    if let Some(d) = manager.diagnostics.iter_mut().find(|d| d.id == id1) {
        d.frame_key = Some(FrameKey {
            stream: StreamId::A,
            frame_index: 0,
            pts: None,
        });
    }

    manager.add_diagnostic(
        DiagnosticSeverity::Warn,
        StreamId::A,
        "Warning: Non-standard QP value".to_string(),
        DiagnosticCategory::Decode,
        100,
    );

    let id3 = manager.add_diagnostic(
        DiagnosticSeverity::Error,
        StreamId::A,
        "Error: Invalid syntax element".to_string(),
        DiagnosticCategory::Bitstream,
        200,
    );

    if let Some(d) = manager.diagnostics.iter_mut().find(|d| d.id == id3) {
        d.frame_key = Some(FrameKey {
            stream: StreamId::A,
            frame_index: 5,
            pts: None,
        });
    }

    manager.add_diagnostic(
        DiagnosticSeverity::Error,
        StreamId::B,
        "Error: Decode failure".to_string(),
        DiagnosticCategory::Decode,
        300,
    );

    manager.add_diagnostic(
        DiagnosticSeverity::Fatal,
        StreamId::A,
        "Fatal: Stream corrupted".to_string(),
        DiagnosticCategory::IO,
        400,
    );

    manager
}

#[test]
fn test_diagnostics_panel_creation() {
    let panel = DiagnosticsPanel::new();

    assert_eq!(panel.sort_column, DiagnosticsSortColumn::Id);
    assert!(panel.sort_ascending);
    assert!(panel.selected_id.is_none());
    assert_eq!(panel.page_size, 50);
}

#[test]
fn test_diagnostics_filter_severity() {
    let manager = create_test_diagnostics();
    let mut panel = DiagnosticsPanel::new();

    // No filter - all 5 diagnostics
    assert_eq!(panel.filtered_count(&manager), 5);

    // Filter to WARN and above
    panel.filter.min_severity = Some(DiagnosticSeverity::Warn);
    assert_eq!(panel.filtered_count(&manager), 4); // Warn, Error, Error, Fatal

    // Filter to ERROR and above
    panel.filter.min_severity = Some(DiagnosticSeverity::Error);
    assert_eq!(panel.filtered_count(&manager), 3); // Error, Error, Fatal

    // Filter to FATAL only
    panel.filter.min_severity = Some(DiagnosticSeverity::Fatal);
    assert_eq!(panel.filtered_count(&manager), 1);
}

#[test]
fn test_diagnostics_filter_category() {
    let manager = create_test_diagnostics();
    let mut panel = DiagnosticsPanel::new();

    panel.filter.categories = vec![DiagnosticCategory::Bitstream];
    assert_eq!(panel.filtered_count(&manager), 2); // Info and Error

    panel.filter.categories = vec![DiagnosticCategory::Decode];
    assert_eq!(panel.filtered_count(&manager), 2); // Warn and Error

    panel.filter.categories = vec![DiagnosticCategory::IO];
    assert_eq!(panel.filtered_count(&manager), 1); // Fatal
}

#[test]
fn test_diagnostics_filter_stream() {
    let manager = create_test_diagnostics();
    let mut panel = DiagnosticsPanel::new();

    panel.filter.stream_id = Some(StreamId::A);
    assert_eq!(panel.filtered_count(&manager), 4); // All except Stream B error

    panel.filter.stream_id = Some(StreamId::B);
    assert_eq!(panel.filtered_count(&manager), 1); // Only Stream B error
}

#[test]
fn test_diagnostics_filter_text_search() {
    let manager = create_test_diagnostics();
    let mut panel = DiagnosticsPanel::new();

    panel.filter.text_search = Some("Error".to_string());
    assert_eq!(panel.filtered_count(&manager), 2); // Two error messages

    panel.filter.text_search = Some("QP".to_string());
    assert_eq!(panel.filtered_count(&manager), 1); // Warning about QP

    // Case insensitive
    panel.filter.text_search = Some("fatal".to_string());
    assert_eq!(panel.filtered_count(&manager), 1);
}

#[test]
fn test_diagnostics_panel_sorting() {
    let manager = create_test_diagnostics();
    let mut panel = DiagnosticsPanel::new();

    // Default sort by ID ascending
    let view = panel.get_view(&manager);
    assert_eq!(view[0].id, 1);
    assert_eq!(view[4].id, 5);

    // Sort by severity
    panel.toggle_sort(DiagnosticsSortColumn::Severity);
    let view = panel.get_view(&manager);
    assert_eq!(view[0].severity, DiagnosticSeverity::Info);
    assert_eq!(view[4].severity, DiagnosticSeverity::Fatal);

    // Toggle again for descending
    panel.toggle_sort(DiagnosticsSortColumn::Severity);
    let view = panel.get_view(&manager);
    assert_eq!(view[0].severity, DiagnosticSeverity::Fatal);
}

#[test]
fn test_diagnostics_panel_pagination() {
    let mut manager = DiagnosticsManager::new();

    // Add 75 diagnostics
    for i in 0..75 {
        manager.add_diagnostic(
            DiagnosticSeverity::Info,
            StreamId::A,
            format!("Diagnostic {}", i),
            DiagnosticCategory::Bitstream,
            i as u64,
        );
    }

    let mut panel = DiagnosticsPanel::new();
    panel.page_size = 20;

    assert_eq!(panel.total_pages(&manager), 4); // 75 / 20 = 3.75 -> 4

    // First page
    let page = panel.get_page(&manager);
    assert_eq!(page.len(), 20);
    assert_eq!(page[0].id, 1);

    // Next page
    panel.next_page(&manager);
    let page = panel.get_page(&manager);
    assert_eq!(page[0].id, 21);

    // Last page
    panel.last_page(&manager);
    let page = panel.get_page(&manager);
    assert_eq!(page.len(), 15); // 75 - 60 = 15

    // First page
    panel.first_page();
    assert_eq!(panel.current_page, 0);
}

#[test]
fn test_diagnostics_panel_jump_target() {
    let manager = create_test_diagnostics();
    let mut panel = DiagnosticsPanel::new();

    // Select diagnostic with frame key (id=3, frame 5)
    let frame_key = panel.select_diagnostic(&manager, 3);
    assert!(frame_key.is_some());
    assert_eq!(frame_key.unwrap().frame_index, 5);

    // Get jump target
    let target = panel.get_jump_target(&manager);
    assert!(target.is_some());
    let target = target.unwrap();
    assert!(target.frame_key.is_some());
    assert_eq!(target.offset_bytes, 200);
}

#[test]
fn test_diagnostics_summary() {
    let manager = create_test_diagnostics();
    let panel = DiagnosticsPanel::new();

    let summary = panel.get_summary(&manager);

    assert_eq!(summary.total, 5);
    assert_eq!(summary.filtered, 5);
    assert_eq!(summary.counts.info, 1);
    assert_eq!(summary.counts.warn, 1);
    assert_eq!(summary.counts.error, 2);
    assert_eq!(summary.counts.fatal, 1);

    let header = summary.header_text();
    assert!(header.contains("5 / 5"));
}

#[test]
fn test_diagnostics_filter_combined() {
    let manager = create_test_diagnostics();
    let mut panel = DiagnosticsPanel::new();

    // Combine filters: ERROR+ from Stream A
    panel.filter.min_severity = Some(DiagnosticSeverity::Error);
    panel.filter.stream_id = Some(StreamId::A);

    // Should get: Error (stream A) + Fatal (stream A) = 2
    assert_eq!(panel.filtered_count(&manager), 2);

    let view = panel.get_view(&manager);
    assert!(view.iter().all(|d| d.stream_id == StreamId::A));
    assert!(view.iter().all(|d| d.severity >= DiagnosticSeverity::Error));
}
