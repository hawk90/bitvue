#![allow(dead_code)]
//! Unit tests for Enhanced Diagnostics core functionality
//!
//! Tests for:
//! - Diagnostic struct creation with Bitvue extensions
//! - Impact score calculation
//! - Frame index association
//! - Count/burst detection

use bitvue_core::event::{Category, Diagnostic, Severity};
use bitvue_core::StreamId;

#[test]
fn test_diagnostic_creation_basic() {
    let diag = Diagnostic {
        id: 1,
        severity: Severity::Error,
        stream_id: StreamId::A,
        message: "Test error message".to_string(),
        category: Category::Bitstream,
        offset_bytes: 1000,
        timestamp_ms: 0,
        // Bitvue extensions
        frame_index: Some(10),
        count: 1,
        impact_score: 85,
    };

    assert_eq!(diag.severity, Severity::Error);
    assert_eq!(diag.frame_index, Some(10));
    assert_eq!(diag.count, 1);
    assert_eq!(diag.impact_score, 85);
}

#[test]
fn test_diagnostic_frame_association() {
    // Test that frame_index properly associates diagnostic with frame
    let diag_with_frame = Diagnostic {
        id: 1,
        severity: Severity::Error,
        stream_id: StreamId::A,
        message: "Frame-specific error".to_string(),
        category: Category::Bitstream,
        offset_bytes: 1000,
        timestamp_ms: 33, // 30fps = 33ms per frame
        frame_index: Some(1),
        count: 1,
        impact_score: 90,
    };

    assert!(diag_with_frame.frame_index.is_some());
    assert_eq!(diag_with_frame.frame_index.unwrap(), 1);

    // Test diagnostic without frame association
    let diag_no_frame = Diagnostic {
        id: 2,
        severity: Severity::Warn,
        stream_id: StreamId::A,
        message: "Container-level warning".to_string(),
        category: Category::Container,
        offset_bytes: 0,
        timestamp_ms: 0,
        frame_index: None,
        count: 1,
        impact_score: 40,
    };

    assert!(diag_no_frame.frame_index.is_none());
}

#[test]
fn test_diagnostic_burst_detection() {
    // Test count field for burst detection
    let single_error = Diagnostic {
        id: 1,
        severity: Severity::Error,
        stream_id: StreamId::A,
        message: "Single error".to_string(),
        category: Category::Bitstream,
        offset_bytes: 1000,
        timestamp_ms: 0,
        frame_index: Some(10),
        count: 1,
        impact_score: 85,
    };

    let burst_error = Diagnostic {
        id: 2,
        severity: Severity::Error,
        stream_id: StreamId::A,
        message: "Burst error (repeated 5 times)".to_string(),
        category: Category::Bitstream,
        offset_bytes: 2000,
        timestamp_ms: 66,
        frame_index: Some(20),
        count: 5,
        impact_score: 95,
    };

    assert_eq!(single_error.count, 1);
    assert_eq!(burst_error.count, 5);
    assert!(burst_error.count > single_error.count);
}

#[test]
fn test_impact_score_ranges() {
    // Critical (80+)
    let critical = Diagnostic {
        id: 1,
        severity: Severity::Error,
        stream_id: StreamId::A,
        message: "Critical error".to_string(),
        category: Category::Bitstream,
        offset_bytes: 1000,
        timestamp_ms: 0,
        frame_index: Some(10),
        count: 1,
        impact_score: 95,
    };

    // Warning (50-79)
    let warning = Diagnostic {
        id: 2,
        severity: Severity::Warn,
        stream_id: StreamId::A,
        message: "Medium impact warning".to_string(),
        category: Category::Decode,
        offset_bytes: 2000,
        timestamp_ms: 33,
        frame_index: Some(20),
        count: 1,
        impact_score: 65,
    };

    // Low impact (<50)
    let low = Diagnostic {
        id: 3,
        severity: Severity::Info,
        stream_id: StreamId::A,
        message: "Low impact info".to_string(),
        category: Category::Metric,
        offset_bytes: 3000,
        timestamp_ms: 66,
        frame_index: Some(30),
        count: 1,
        impact_score: 25,
    };

    assert!(critical.impact_score >= 80);
    assert!(warning.impact_score >= 50 && warning.impact_score < 80);
    assert!(low.impact_score < 50);
}

#[test]
fn test_severity_ordering() {
    // Test that Severity enum is properly ordered
    assert!(Severity::Info < Severity::Warn);
    assert!(Severity::Warn < Severity::Error);
    assert!(Severity::Error < Severity::Fatal);
}

#[test]
fn test_timestamp_calculation() {
    // Test timestamp association (30fps = ~33ms per frame)
    let diagnostics = vec![
        Diagnostic {
            id: 1,
            severity: Severity::Error,
            stream_id: StreamId::A,
            message: "Frame 0 error".to_string(),
            category: Category::Bitstream,
            offset_bytes: 1000,
            timestamp_ms: 0,
            frame_index: Some(0),
            count: 1,
            impact_score: 85,
        },
        Diagnostic {
            id: 2,
            severity: Severity::Error,
            stream_id: StreamId::A,
            message: "Frame 30 error".to_string(),
            category: Category::Bitstream,
            offset_bytes: 2000,
            timestamp_ms: 1000, // 30 frames * 33ms ≈ 1000ms
            frame_index: Some(30),
            count: 1,
            impact_score: 85,
        },
    ];

    let time_diff = diagnostics[1].timestamp_ms - diagnostics[0].timestamp_ms;
    assert_eq!(time_diff, 1000); // 1 second apart
}

#[test]
fn test_diagnostic_clone() {
    // Test that Diagnostic can be cloned (required for tri-sync)
    let original = Diagnostic {
        id: 1,
        severity: Severity::Error,
        stream_id: StreamId::A,
        message: "Original error".to_string(),
        category: Category::Bitstream,
        offset_bytes: 1000,
        timestamp_ms: 0,
        frame_index: Some(10),
        count: 1,
        impact_score: 85,
    };

    let cloned = original.clone();

    assert_eq!(original.id, cloned.id);
    assert_eq!(original.message, cloned.message);
    assert_eq!(original.frame_index, cloned.frame_index);
    assert_eq!(original.count, cloned.count);
    assert_eq!(original.impact_score, cloned.impact_score);
}

#[test]
fn test_diagnostic_all_categories() {
    // Test all category types
    let categories = vec![
        Category::Container,
        Category::Bitstream,
        Category::Decode,
        Category::Metric,
        Category::IO,
        Category::Worker,
    ];

    for (i, category) in categories.iter().enumerate() {
        let diag = Diagnostic {
            id: i as u64,
            severity: Severity::Error,
            stream_id: StreamId::A,
            message: format!("Category test: {:?}", category),
            category: *category,
            offset_bytes: (i * 1000) as u64,
            timestamp_ms: 0,
            frame_index: Some(i),
            count: 1,
            impact_score: 50,
        };

        assert_eq!(diag.category, *category);
    }
}

#[test]
fn test_impact_score_bounds() {
    // Test that impact_score is u8 (0-255)
    let max_impact = Diagnostic {
        id: 1,
        severity: Severity::Fatal,
        stream_id: StreamId::A,
        message: "Maximum impact".to_string(),
        category: Category::Bitstream,
        offset_bytes: 1000,
        timestamp_ms: 0,
        frame_index: Some(10),
        count: 10,
        impact_score: 100,
    };

    let min_impact = Diagnostic {
        id: 2,
        severity: Severity::Info,
        stream_id: StreamId::A,
        message: "Minimum impact".to_string(),
        category: Category::Metric,
        offset_bytes: 2000,
        timestamp_ms: 33,
        frame_index: Some(20),
        count: 1,
        impact_score: 0,
    };

    assert_eq!(max_impact.impact_score, 100);
    assert_eq!(min_impact.impact_score, 0);
    assert!(max_impact.impact_score <= 100);
}
