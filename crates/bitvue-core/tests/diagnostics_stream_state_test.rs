#![allow(dead_code)]
//! Integration tests for Diagnostics in StreamState
//!
//! Tests for:
//! - Adding diagnostics to StreamState
//! - Filtering by severity
//! - Counting errors/warnings
//! - Clearing diagnostics

use bitvue_core::event::{Category, Diagnostic, Severity};
use bitvue_core::{StreamId, StreamState};

fn create_test_diagnostic(
    id: u64,
    severity: Severity,
    frame_idx: Option<usize>,
    impact: u8,
) -> Diagnostic {
    Diagnostic {
        id,
        severity,
        stream_id: StreamId::A,
        message: format!("Test diagnostic #{}", id),
        category: Category::Bitstream,
        offset_bytes: id * 1000,
        timestamp_ms: id * 33,
        frame_index: frame_idx,
        count: 1,
        impact_score: impact,
    }
}

#[test]
fn test_add_diagnostic_to_stream() {
    let mut state = StreamState::new(StreamId::A);

    let diag = create_test_diagnostic(1, Severity::Error, Some(10), 85);
    state.add_diagnostic(diag.clone());

    assert_eq!(state.diagnostics.len(), 1);
    assert_eq!(state.diagnostics[0].id, 1);
}

#[test]
fn test_add_multiple_diagnostics() {
    let mut state = StreamState::new(StreamId::A);

    // Add 3 errors and 2 warnings
    for i in 1..=3 {
        state.add_diagnostic(create_test_diagnostic(
            i,
            Severity::Error,
            Some(i as usize * 10),
            85 + i as u8,
        ));
    }

    for i in 4..=5 {
        state.add_diagnostic(create_test_diagnostic(
            i,
            Severity::Warn,
            Some(i as usize * 10),
            40 + i as u8,
        ));
    }

    assert_eq!(state.diagnostics.len(), 5);
}

#[test]
fn test_filter_diagnostics_by_severity() {
    let mut state = StreamState::new(StreamId::A);

    // Add mixed diagnostics
    state.add_diagnostic(create_test_diagnostic(1, Severity::Error, Some(10), 90));
    state.add_diagnostic(create_test_diagnostic(2, Severity::Warn, Some(20), 50));
    state.add_diagnostic(create_test_diagnostic(3, Severity::Error, Some(30), 85));
    state.add_diagnostic(create_test_diagnostic(4, Severity::Info, Some(40), 25));
    state.add_diagnostic(create_test_diagnostic(5, Severity::Error, Some(50), 88));

    // Filter errors
    let errors = state.diagnostics_by_severity(Severity::Error);
    assert_eq!(errors.len(), 3);

    // Filter warnings
    let warnings = state.diagnostics_by_severity(Severity::Warn);
    assert_eq!(warnings.len(), 1);

    // Filter info
    let infos = state.diagnostics_by_severity(Severity::Info);
    assert_eq!(infos.len(), 1);
}

#[test]
fn test_clear_diagnostics() {
    let mut state = StreamState::new(StreamId::A);

    // Add diagnostics
    for i in 1..=10 {
        state.add_diagnostic(create_test_diagnostic(
            i,
            Severity::Error,
            Some(i as usize),
            80,
        ));
    }

    assert_eq!(state.diagnostics.len(), 10);

    // Clear
    state.clear_diagnostics();

    assert_eq!(state.diagnostics.len(), 0);
}

#[test]
fn test_count_errors_and_warnings() {
    let mut state = StreamState::new(StreamId::A);

    // Add 5 errors
    for i in 1..=5 {
        state.add_diagnostic(create_test_diagnostic(
            i,
            Severity::Error,
            Some(i as usize * 10),
            85,
        ));
    }

    // Add 3 warnings
    for i in 6..=8 {
        state.add_diagnostic(create_test_diagnostic(
            i,
            Severity::Warn,
            Some(i as usize * 10),
            50,
        ));
    }

    // Add 2 info
    for i in 9..=10 {
        state.add_diagnostic(create_test_diagnostic(
            i,
            Severity::Info,
            Some(i as usize * 10),
            20,
        ));
    }

    let error_count = state.diagnostics_by_severity(Severity::Error).len();
    let warn_count = state.diagnostics_by_severity(Severity::Warn).len();
    let info_count = state.diagnostics_by_severity(Severity::Info).len();

    assert_eq!(error_count, 5);
    assert_eq!(warn_count, 3);
    assert_eq!(info_count, 2);
    assert_eq!(state.diagnostics.len(), 10);
}

#[test]
fn test_diagnostic_ordering_preserved() {
    let mut state = StreamState::new(StreamId::A);

    // Add diagnostics in specific order
    for i in 1..=5 {
        state.add_diagnostic(create_test_diagnostic(
            i,
            Severity::Error,
            Some(i as usize),
            80 + i as u8,
        ));
    }

    // Verify order is preserved
    for (i, diag) in state.diagnostics.iter().enumerate() {
        assert_eq!(diag.id, (i + 1) as u64);
    }
}

#[test]
fn test_burst_error_detection() {
    let mut state = StreamState::new(StreamId::A);

    // Add single error
    state.add_diagnostic(create_test_diagnostic(1, Severity::Error, Some(10), 85));

    // Add burst error (count = 5)
    let mut burst = create_test_diagnostic(2, Severity::Error, Some(20), 95);
    burst.count = 5;
    state.add_diagnostic(burst);

    // Add another single error
    state.add_diagnostic(create_test_diagnostic(3, Severity::Error, Some(30), 85));

    assert_eq!(state.diagnostics.len(), 3);
    assert_eq!(state.diagnostics[0].count, 1);
    assert_eq!(state.diagnostics[1].count, 5); // Burst
    assert_eq!(state.diagnostics[2].count, 1);
}

#[test]
fn test_high_impact_errors_filtering() {
    let mut state = StreamState::new(StreamId::A);

    // Add various impact scores
    state.add_diagnostic(create_test_diagnostic(1, Severity::Error, Some(10), 95)); // Critical
    state.add_diagnostic(create_test_diagnostic(2, Severity::Error, Some(20), 65)); // Medium
    state.add_diagnostic(create_test_diagnostic(3, Severity::Error, Some(30), 98)); // Critical
    state.add_diagnostic(create_test_diagnostic(4, Severity::Warn, Some(40), 45)); // Low
    state.add_diagnostic(create_test_diagnostic(5, Severity::Error, Some(50), 88)); // Critical

    // Find critical errors (impact >= 80)
    let critical: Vec<_> = state
        .diagnostics
        .iter()
        .filter(|d| d.impact_score >= 80)
        .collect();

    assert_eq!(critical.len(), 3);
}

#[test]
fn test_frame_specific_errors() {
    let mut state = StreamState::new(StreamId::A);

    // Add errors for specific frames
    state.add_diagnostic(create_test_diagnostic(1, Severity::Error, Some(10), 85));
    state.add_diagnostic(create_test_diagnostic(2, Severity::Error, Some(10), 90)); // Same frame
    state.add_diagnostic(create_test_diagnostic(3, Severity::Error, Some(20), 85));

    // Find errors for frame 10
    let frame_10_errors: Vec<_> = state
        .diagnostics
        .iter()
        .filter(|d| d.frame_index == Some(10))
        .collect();

    assert_eq!(frame_10_errors.len(), 2);
}

#[test]
fn test_diagnostic_without_frame_index() {
    let mut state = StreamState::new(StreamId::A);

    // Container-level error (no frame association)
    let container_error = Diagnostic {
        id: 1,
        severity: Severity::Error,
        stream_id: StreamId::A,
        message: "Container parsing error".to_string(),
        category: Category::Container,
        offset_bytes: 0,
        timestamp_ms: 0,
        frame_index: None, // No frame association
        count: 1,
        impact_score: 100,
    };

    state.add_diagnostic(container_error);

    assert_eq!(state.diagnostics.len(), 1);
    assert!(state.diagnostics[0].frame_index.is_none());
}
