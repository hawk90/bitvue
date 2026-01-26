//! Edge case tests for Enhanced Diagnostics
//!
//! Tests for:
//! - Boundary conditions
//! - Invalid/unusual data
//! - Concurrent access
//! - Memory limits

use bitvue_core::event::{Category, Diagnostic, Severity};
use bitvue_core::{Core, StreamId};
use std::sync::Arc;

#[test]
fn test_diagnostic_id_boundary_values() {
    let core = Arc::new(Core::new());
    let stream = core.get_stream(StreamId::A);
    let mut state = stream.write();

    // Test minimum ID
    state.add_diagnostic(Diagnostic {
        id: 0,
        severity: Severity::Info,
        stream_id: StreamId::A,
        message: "ID = 0".to_string(),
        category: Category::Bitstream,
        offset_bytes: 0,
        timestamp_ms: 0,
        frame_index: Some(0),
        count: 1,
        impact_score: 50,
    });

    // Test maximum ID
    state.add_diagnostic(Diagnostic {
        id: u64::MAX,
        severity: Severity::Info,
        stream_id: StreamId::A,
        message: "ID = u64::MAX".to_string(),
        category: Category::Bitstream,
        offset_bytes: 0,
        timestamp_ms: 0,
        frame_index: Some(0),
        count: 1,
        impact_score: 50,
    });

    assert_eq!(state.diagnostics.len(), 2);
    assert_eq!(state.diagnostics[0].id, 0);
    assert_eq!(state.diagnostics[1].id, u64::MAX);
}

#[test]
fn test_impact_score_boundary_values() {
    let core = Arc::new(Core::new());
    let stream = core.get_stream(StreamId::A);
    let mut state = stream.write();

    // Test minimum impact (0)
    state.add_diagnostic(Diagnostic {
        id: 1,
        severity: Severity::Info,
        stream_id: StreamId::A,
        message: "Impact = 0".to_string(),
        category: Category::Bitstream,
        offset_bytes: 0,
        timestamp_ms: 0,
        frame_index: Some(0),
        count: 1,
        impact_score: 0,
    });

    // Test maximum impact (100)
    state.add_diagnostic(Diagnostic {
        id: 2,
        severity: Severity::Fatal,
        stream_id: StreamId::A,
        message: "Impact = 100".to_string(),
        category: Category::Bitstream,
        offset_bytes: 0,
        timestamp_ms: 0,
        frame_index: Some(0),
        count: 1,
        impact_score: 100,
    });

    // Test boundary thresholds
    state.add_diagnostic(Diagnostic {
        id: 3,
        severity: Severity::Error,
        stream_id: StreamId::A,
        message: "Impact = 80 (critical threshold)".to_string(),
        category: Category::Bitstream,
        offset_bytes: 0,
        timestamp_ms: 0,
        frame_index: Some(0),
        count: 1,
        impact_score: 80,
    });

    state.add_diagnostic(Diagnostic {
        id: 4,
        severity: Severity::Warn,
        stream_id: StreamId::A,
        message: "Impact = 50 (warning threshold)".to_string(),
        category: Category::Bitstream,
        offset_bytes: 0,
        timestamp_ms: 0,
        frame_index: Some(0),
        count: 1,
        impact_score: 50,
    });

    assert_eq!(state.diagnostics.len(), 4);
}

#[test]
fn test_count_boundary_values() {
    let core = Arc::new(Core::new());
    let stream = core.get_stream(StreamId::A);
    let mut state = stream.write();

    // Test minimum count (1)
    state.add_diagnostic(Diagnostic {
        id: 1,
        severity: Severity::Error,
        stream_id: StreamId::A,
        message: "Count = 1".to_string(),
        category: Category::Bitstream,
        offset_bytes: 0,
        timestamp_ms: 0,
        frame_index: Some(0),
        count: 1,
        impact_score: 85,
    });

    // Test burst (5)
    state.add_diagnostic(Diagnostic {
        id: 2,
        severity: Severity::Error,
        stream_id: StreamId::A,
        message: "Count = 5 (burst)".to_string(),
        category: Category::Bitstream,
        offset_bytes: 0,
        timestamp_ms: 0,
        frame_index: Some(0),
        count: 5,
        impact_score: 95,
    });

    // Test large burst
    state.add_diagnostic(Diagnostic {
        id: 3,
        severity: Severity::Error,
        stream_id: StreamId::A,
        message: "Count = 1000 (large burst)".to_string(),
        category: Category::Bitstream,
        offset_bytes: 0,
        timestamp_ms: 0,
        frame_index: Some(0),
        count: 1000,
        impact_score: 100,
    });

    // Test maximum count
    state.add_diagnostic(Diagnostic {
        id: 4,
        severity: Severity::Fatal,
        stream_id: StreamId::A,
        message: "Count = u32::MAX".to_string(),
        category: Category::Bitstream,
        offset_bytes: 0,
        timestamp_ms: 0,
        frame_index: Some(0),
        count: u32::MAX,
        impact_score: 100,
    });

    assert_eq!(state.diagnostics.len(), 4);
    assert_eq!(state.diagnostics[3].count, u32::MAX);
}

#[test]
fn test_offset_bytes_boundary_values() {
    let core = Arc::new(Core::new());
    let stream = core.get_stream(StreamId::A);
    let mut state = stream.write();

    // Test offset = 0 (start of file)
    state.add_diagnostic(Diagnostic {
        id: 1,
        severity: Severity::Error,
        stream_id: StreamId::A,
        message: "At file start".to_string(),
        category: Category::Container,
        offset_bytes: 0,
        timestamp_ms: 0,
        frame_index: None,
        count: 1,
        impact_score: 100,
    });

    // Test large offset (e.g., 4GB file)
    state.add_diagnostic(Diagnostic {
        id: 2,
        severity: Severity::Error,
        stream_id: StreamId::A,
        message: "At 4GB offset".to_string(),
        category: Category::Bitstream,
        offset_bytes: 4_294_967_296, // 4GB
        timestamp_ms: 0,
        frame_index: Some(10000),
        count: 1,
        impact_score: 85,
    });

    // Test maximum offset
    state.add_diagnostic(Diagnostic {
        id: 3,
        severity: Severity::Error,
        stream_id: StreamId::A,
        message: "At maximum offset".to_string(),
        category: Category::Bitstream,
        offset_bytes: u64::MAX,
        timestamp_ms: 0,
        frame_index: Some(20000),
        count: 1,
        impact_score: 85,
    });

    assert_eq!(state.diagnostics.len(), 3);
    assert_eq!(state.diagnostics[2].offset_bytes, u64::MAX);
}

#[test]
fn test_timestamp_boundary_values() {
    let core = Arc::new(Core::new());
    let stream = core.get_stream(StreamId::A);
    let mut state = stream.write();

    // Test timestamp = 0 (start of video)
    state.add_diagnostic(Diagnostic {
        id: 1,
        severity: Severity::Error,
        stream_id: StreamId::A,
        message: "At time 0".to_string(),
        category: Category::Bitstream,
        offset_bytes: 0,
        timestamp_ms: 0,
        frame_index: Some(0),
        count: 1,
        impact_score: 85,
    });

    // Test long video (10 hours)
    state.add_diagnostic(Diagnostic {
        id: 2,
        severity: Severity::Error,
        stream_id: StreamId::A,
        message: "At 10 hours".to_string(),
        category: Category::Bitstream,
        offset_bytes: 0,
        timestamp_ms: 36_000_000, // 10 hours in ms
        frame_index: Some(1080000), // 10 hours @ 30fps
        count: 1,
        impact_score: 85,
    });

    // Test maximum timestamp
    state.add_diagnostic(Diagnostic {
        id: 3,
        severity: Severity::Error,
        stream_id: StreamId::A,
        message: "At maximum timestamp".to_string(),
        category: Category::Bitstream,
        offset_bytes: 0,
        timestamp_ms: u64::MAX,
        frame_index: Some(usize::MAX),
        count: 1,
        impact_score: 85,
    });

    assert_eq!(state.diagnostics.len(), 3);
}

#[test]
fn test_empty_message() {
    let core = Arc::new(Core::new());
    let stream = core.get_stream(StreamId::A);
    let mut state = stream.write();

    // Test empty message
    state.add_diagnostic(Diagnostic {
        id: 1,
        severity: Severity::Error,
        stream_id: StreamId::A,
        message: "".to_string(),
        category: Category::Bitstream,
        offset_bytes: 0,
        timestamp_ms: 0,
        frame_index: Some(0),
        count: 1,
        impact_score: 85,
    });

    assert_eq!(state.diagnostics.len(), 1);
    assert_eq!(state.diagnostics[0].message, "");
}

#[test]
fn test_special_characters_in_message() {
    let core = Arc::new(Core::new());
    let stream = core.get_stream(StreamId::A);
    let mut state = stream.write();

    // Test Unicode characters
    state.add_diagnostic(Diagnostic {
        id: 1,
        severity: Severity::Error,
        stream_id: StreamId::A,
        message: "Error with Unicode: 한글 日本語 🔥 ●▲○".to_string(),
        category: Category::Bitstream,
        offset_bytes: 0,
        timestamp_ms: 0,
        frame_index: Some(0),
        count: 1,
        impact_score: 85,
    });

    // Test special ASCII characters
    state.add_diagnostic(Diagnostic {
        id: 2,
        severity: Severity::Error,
        stream_id: StreamId::A,
        message: "Error with special chars: \t\n\r\"'\\".to_string(),
        category: Category::Bitstream,
        offset_bytes: 0,
        timestamp_ms: 0,
        frame_index: Some(0),
        count: 1,
        impact_score: 85,
    });

    assert_eq!(state.diagnostics.len(), 2);
}

#[test]
fn test_duplicate_diagnostics() {
    let core = Arc::new(Core::new());
    let stream = core.get_stream(StreamId::A);
    let mut state = stream.write();

    let diag = Diagnostic {
        id: 1,
        severity: Severity::Error,
        stream_id: StreamId::A,
        message: "Duplicate error".to_string(),
        category: Category::Bitstream,
        offset_bytes: 1000,
        timestamp_ms: 33,
        frame_index: Some(10),
        count: 1,
        impact_score: 85,
    };

    // Add same diagnostic 3 times
    state.add_diagnostic(diag.clone());
    state.add_diagnostic(diag.clone());
    state.add_diagnostic(diag.clone());

    // All duplicates should be stored
    assert_eq!(state.diagnostics.len(), 3);
}

#[test]
fn test_clearing_large_diagnostic_set() {
    let core = Arc::new(Core::new());
    let stream = core.get_stream(StreamId::A);
    let mut state = stream.write();

    // Add 10,000 diagnostics
    for i in 0..10_000 {
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

    assert_eq!(state.diagnostics.len(), 10_000);

    // Clear all
    state.clear_diagnostics();

    assert_eq!(state.diagnostics.len(), 0);
}

#[test]
fn test_interleaved_severity_filtering() {
    let core = Arc::new(Core::new());
    let stream = core.get_stream(StreamId::A);
    let mut state = stream.write();

    // Add interleaved severities
    let pattern = vec![
        Severity::Fatal,
        Severity::Error,
        Severity::Warn,
        Severity::Info,
    ];

    for i in 0..100 {
        let severity = pattern[i % 4];
        state.add_diagnostic(Diagnostic {
            id: i as u64,
            severity,
            stream_id: StreamId::A,
            message: format!("{:?} {}", severity, i),
            category: Category::Bitstream,
            offset_bytes: i as u64 * 1000,
            timestamp_ms: i as u64 * 33,
            frame_index: Some(i * 10),
            count: 1,
            impact_score: 85,
        });
    }

    assert_eq!(state.diagnostics_by_severity(Severity::Fatal).len(), 25);
    assert_eq!(state.diagnostics_by_severity(Severity::Error).len(), 25);
    assert_eq!(state.diagnostics_by_severity(Severity::Warn).len(), 25);
    assert_eq!(state.diagnostics_by_severity(Severity::Info).len(), 25);
}

#[test]
fn test_frame_index_gaps() {
    let core = Arc::new(Core::new());
    let stream = core.get_stream(StreamId::A);
    let mut state = stream.write();

    // Add diagnostics with gaps in frame indices
    let frames = vec![0, 5, 10, 50, 51, 100, 200, 500, 1000];

    for (i, &frame) in frames.iter().enumerate() {
        state.add_diagnostic(Diagnostic {
            id: i as u64,
            severity: Severity::Error,
            stream_id: StreamId::A,
            message: format!("Error at frame {}", frame),
            category: Category::Bitstream,
            offset_bytes: (i as u64) * 1000,
            timestamp_ms: (frame as u64) * 33,
            frame_index: Some(frame),
            count: 1,
            impact_score: 85,
        });
    }

    assert_eq!(state.diagnostics.len(), 9);

    // Verify frame indices are preserved
    for (i, &frame) in frames.iter().enumerate() {
        assert_eq!(state.diagnostics[i].frame_index.unwrap(), frame);
    }
}

#[test]
fn test_mixed_frame_index_some_none() {
    let core = Arc::new(Core::new());
    let stream = core.get_stream(StreamId::A);
    let mut state = stream.write();

    // Container error (no frame)
    state.add_diagnostic(Diagnostic {
        id: 1,
        severity: Severity::Error,
        stream_id: StreamId::A,
        message: "Container error".to_string(),
        category: Category::Container,
        offset_bytes: 0,
        timestamp_ms: 0,
        frame_index: None,
        count: 1,
        impact_score: 100,
    });

    // Frame-level error
    state.add_diagnostic(Diagnostic {
        id: 2,
        severity: Severity::Error,
        stream_id: StreamId::A,
        message: "Frame 10 error".to_string(),
        category: Category::Bitstream,
        offset_bytes: 1000,
        timestamp_ms: 330,
        frame_index: Some(10),
        count: 1,
        impact_score: 85,
    });

    // Another container error
    state.add_diagnostic(Diagnostic {
        id: 3,
        severity: Severity::Warn,
        stream_id: StreamId::A,
        message: "HRD warning".to_string(),
        category: Category::Decode,
        offset_bytes: 2000,
        timestamp_ms: 0,
        frame_index: None,
        count: 1,
        impact_score: 50,
    });

    assert_eq!(state.diagnostics.len(), 3);

    // Count diagnostics with and without frames
    let with_frames = state.diagnostics.iter().filter(|d| d.frame_index.is_some()).count();
    let without_frames = state.diagnostics.iter().filter(|d| d.frame_index.is_none()).count();

    assert_eq!(with_frames, 1);
    assert_eq!(without_frames, 2);
}

#[test]
fn test_all_category_impact_combinations() {
    let core = Arc::new(Core::new());
    let stream = core.get_stream(StreamId::A);
    let mut state = stream.write();

    let categories = vec![
        Category::Container,
        Category::Bitstream,
        Category::Decode,
        Category::Metric,
        Category::IO,
        Category::Worker,
    ];

    let impacts = vec![0, 25, 49, 50, 65, 79, 80, 95, 100];

    let mut id = 0;
    for category in &categories {
        for &impact in &impacts {
            state.add_diagnostic(Diagnostic {
                id,
                severity: if impact >= 80 {
                    Severity::Error
                } else if impact >= 50 {
                    Severity::Warn
                } else {
                    Severity::Info
                },
                stream_id: StreamId::A,
                message: format!("{:?} with impact {}", category, impact),
                category: *category,
                offset_bytes: id * 1000,
                timestamp_ms: id * 33,
                frame_index: Some(id as usize),
                count: 1,
                impact_score: impact,
            });
            id += 1;
        }
    }

    assert_eq!(state.diagnostics.len(), categories.len() * impacts.len());
    assert_eq!(state.diagnostics.len(), 54); // 6 categories * 9 impacts
}
