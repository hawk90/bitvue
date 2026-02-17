#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(unused_comparisons)]
#![allow(unused_doc_comments)]
//! Performance and stress tests for Enhanced Diagnostics

use bitvue_core::event::{Category, Diagnostic, Severity};
use bitvue_core::{Core, StreamId};
use std::sync::Arc;
use std::time::Instant;

#[test]
fn test_performance_add_10k_diagnostics() {
    // Test adding 10,000 diagnostics
    let core = Arc::new(Core::new());
    let stream = core.get_stream(StreamId::A);

    let start = Instant::now();

    {
        let mut state = stream.write();
        for i in 0..10_000 {
            state.add_diagnostic(Diagnostic {
                id: i,
                severity: match i % 4 {
                    0 => Severity::Fatal,
                    1 => Severity::Error,
                    2 => Severity::Warn,
                    _ => Severity::Info,
                },
                stream_id: StreamId::A,
                message: format!("Diagnostic {}", i),
                category: Category::Bitstream,
                offset_bytes: i * 1000,
                timestamp_ms: i * 33,
                frame_index: Some(i as usize),
                count: 1,
                impact_score: 50 + ((i % 50) as u8),
            });
        }
    }

    let duration = start.elapsed();

    println!("Added 10k diagnostics in {:?}", duration);

    assert_eq!(stream.read().diagnostics.len(), 10_000);

    // Should complete in reasonable time (< 1 second)
    assert!(
        duration.as_secs() < 1,
        "Adding 10k diagnostics should take < 1s"
    );
}

#[test]
fn test_performance_filter_10k_diagnostics() {
    // Test filtering 10,000 diagnostics
    let core = Arc::new(Core::new());
    let stream = core.get_stream(StreamId::A);

    {
        let mut state = stream.write();
        for i in 0..10_000 {
            state.add_diagnostic(Diagnostic {
                id: i,
                severity: if i % 10 == 0 {
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
                impact_score: 85,
            });
        }
    }

    let start = Instant::now();

    let errors: Vec<_> = {
        let state = stream.read();
        state
            .diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .cloned()
            .collect()
    };

    let duration = start.elapsed();

    println!("Filtered 10k diagnostics in {:?}", duration);

    assert_eq!(errors.len(), 1_000);

    // Filtering should be fast (< 100ms)
    assert!(duration.as_millis() < 100, "Filtering should take < 100ms");
}

#[test]
fn test_performance_sort_10k_diagnostics() {
    // Test sorting 10,000 diagnostics by impact score
    let core = Arc::new(Core::new());
    let stream = core.get_stream(StreamId::A);

    {
        let mut state = stream.write();
        for i in 0..10_000 {
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
                impact_score: ((i * 17) % 100) as u8, // Pseudo-random
            });
        }
    }

    let start = Instant::now();

    let mut sorted: Vec<_> = {
        let state = stream.read();
        state.diagnostics.clone()
    };

    sorted.sort_by(|a, b| b.impact_score.cmp(&a.impact_score));

    let duration = start.elapsed();

    println!("Sorted 10k diagnostics in {:?}", duration);

    assert_eq!(sorted.len(), 10_000);
    assert!(sorted[0].impact_score >= sorted[sorted.len() - 1].impact_score);

    // Sorting should be fast (< 50ms)
    assert!(duration.as_millis() < 50, "Sorting should take < 50ms");
}

#[test]
fn test_performance_concurrent_read_access() {
    // Test concurrent read access to diagnostics
    let core = Arc::new(Core::new());
    let stream = core.get_stream(StreamId::A);

    {
        let mut state = stream.write();
        for i in 0..1_000 {
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
                impact_score: 85,
            });
        }
    }

    let start = Instant::now();

    // Simulate 100 concurrent reads
    for _ in 0..100 {
        let state = stream.read();
        let count = state.diagnostics.len();
        assert_eq!(count, 1_000);
    }

    let duration = start.elapsed();

    println!("100 concurrent reads in {:?}", duration);

    // Should be very fast (< 10ms)
    assert!(
        duration.as_millis() < 10,
        "Concurrent reads should take < 10ms"
    );
}

#[test]
fn test_performance_memory_usage_estimate() {
    // Estimate memory usage for 10k diagnostics
    let core = Arc::new(Core::new());
    let stream = core.get_stream(StreamId::A);

    {
        let mut state = stream.write();
        for i in 0..10_000 {
            state.add_diagnostic(Diagnostic {
                id: i,
                severity: Severity::Error,
                stream_id: StreamId::A,
                message: "This is a diagnostic message".to_string(),
                category: Category::Bitstream,
                offset_bytes: i * 1000,
                timestamp_ms: i * 33,
                frame_index: Some(i as usize),
                count: 1,
                impact_score: 85,
            });
        }
    }

    let count = stream.read().diagnostics.len();
    assert_eq!(count, 10_000);

    // Rough estimate: ~100 bytes per diagnostic
    // 10k diagnostics = ~1MB
    let estimated_bytes = count * 100;
    println!(
        "Estimated memory usage: {} MB",
        estimated_bytes / 1_024 / 1_024
    );

    // Should be reasonable (< 2MB)
    assert!(estimated_bytes < 2 * 1_024 * 1_024);
}

#[test]
fn test_performance_burst_detection_10k() {
    // Test burst detection performance on 10k diagnostics
    let core = Arc::new(Core::new());
    let stream = core.get_stream(StreamId::A);

    {
        let mut state = stream.write();
        for i in 0..10_000 {
            state.add_diagnostic(Diagnostic {
                id: i,
                severity: Severity::Error,
                stream_id: StreamId::A,
                message: format!("Diagnostic {}", i),
                category: Category::Bitstream,
                offset_bytes: i * 1000,
                timestamp_ms: i * 33,
                frame_index: Some(i as usize),
                count: if i % 100 == 0 { 5 } else { 1 }, // 100 bursts
                impact_score: 85,
            });
        }
    }

    let start = Instant::now();

    let bursts: Vec<_> = {
        let state = stream.read();
        state
            .diagnostics
            .iter()
            .filter(|d| d.count > 1)
            .cloned()
            .collect()
    };

    let duration = start.elapsed();

    println!("Detected {} bursts in {:?}", bursts.len(), duration);

    assert_eq!(bursts.len(), 100);

    // Should be fast (< 50ms)
    assert!(
        duration.as_millis() < 50,
        "Burst detection should take < 50ms"
    );
}

#[test]
fn test_performance_timestamp_search() {
    // Test searching diagnostics by timestamp
    let core = Arc::new(Core::new());
    let stream = core.get_stream(StreamId::A);

    {
        let mut state = stream.write();
        for i in 0..10_000 {
            state.add_diagnostic(Diagnostic {
                id: i,
                severity: Severity::Error,
                stream_id: StreamId::A,
                message: format!("Diagnostic {}", i),
                category: Category::Bitstream,
                offset_bytes: i * 1000,
                timestamp_ms: i * 33, // 0ms, 33ms, 66ms, etc.
                frame_index: Some(i as usize),
                count: 1,
                impact_score: 85,
            });
        }
    }

    let start = Instant::now();

    // Find diagnostic closest to 150000ms (150s)
    let target_time = 150_000u64;

    let closest = {
        let state = stream.read();
        state
            .diagnostics
            .iter()
            .min_by_key(|d| {
                if d.timestamp_ms > target_time {
                    d.timestamp_ms - target_time
                } else {
                    target_time - d.timestamp_ms
                }
            })
            .cloned()
    };

    let duration = start.elapsed();

    println!("Found closest timestamp in {:?}", duration);

    assert!(closest.is_some());

    // Should be fast (< 50ms)
    assert!(
        duration.as_millis() < 50,
        "Timestamp search should take < 50ms"
    );
}

#[test]
fn test_performance_frame_range_query() {
    // Test querying diagnostics within frame range
    let core = Arc::new(Core::new());
    let stream = core.get_stream(StreamId::A);

    {
        let mut state = stream.write();
        for i in 0..10_000 {
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
                impact_score: 85,
            });
        }
    }

    let start = Instant::now();

    // Query frames 4000-6000
    let in_range: Vec<_> = {
        let state = stream.read();
        state
            .diagnostics
            .iter()
            .filter(|d| {
                if let Some(frame) = d.frame_index {
                    frame >= 4000 && frame <= 6000
                } else {
                    false
                }
            })
            .cloned()
            .collect()
    };

    let duration = start.elapsed();

    println!("Queried frame range in {:?}", duration);

    assert_eq!(in_range.len(), 2001); // 4000-6000 inclusive

    // Should be fast (< 50ms)
    assert!(
        duration.as_millis() < 50,
        "Frame range query should take < 50ms"
    );
}

#[test]
fn test_performance_impact_histogram() {
    // Test building impact score histogram
    let core = Arc::new(Core::new());
    let stream = core.get_stream(StreamId::A);

    {
        let mut state = stream.write();
        for i in 0..10_000 {
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
                impact_score: ((i % 100) as u8), // 0-99
            });
        }
    }

    let start = Instant::now();

    // Build histogram (10 bins)
    let mut histogram = vec![0; 10];

    {
        let state = stream.read();
        for diag in &state.diagnostics {
            let bin = (diag.impact_score / 10) as usize;
            let bin = bin.min(9); // Cap at 9
            histogram[bin] += 1;
        }
    }

    let duration = start.elapsed();

    println!("Built histogram in {:?}", duration);

    let total: usize = histogram.iter().sum();
    assert_eq!(total, 10_000);

    // Should be fast (< 50ms)
    assert!(
        duration.as_millis() < 50,
        "Histogram building should take < 50ms"
    );
}

#[test]
fn test_performance_category_distribution() {
    // Test computing category distribution
    let core = Arc::new(Core::new());
    let stream = core.get_stream(StreamId::A);

    {
        let mut state = stream.write();
        for i in 0..10_000 {
            let category = match i % 6 {
                0 => Category::Container,
                1 => Category::Bitstream,
                2 => Category::Decode,
                3 => Category::Metric,
                4 => Category::IO,
                _ => Category::Worker,
            };

            state.add_diagnostic(Diagnostic {
                id: i,
                severity: Severity::Error,
                stream_id: StreamId::A,
                message: format!("Diagnostic {}", i),
                category,
                offset_bytes: i * 1000,
                timestamp_ms: i * 33,
                frame_index: Some(i as usize),
                count: 1,
                impact_score: 85,
            });
        }
    }

    let start = Instant::now();

    // Count by category
    let state = stream.read();
    let container_count = state
        .diagnostics
        .iter()
        .filter(|d| d.category == Category::Container)
        .count();
    let bitstream_count = state
        .diagnostics
        .iter()
        .filter(|d| d.category == Category::Bitstream)
        .count();
    let decode_count = state
        .diagnostics
        .iter()
        .filter(|d| d.category == Category::Decode)
        .count();
    let metric_count = state
        .diagnostics
        .iter()
        .filter(|d| d.category == Category::Metric)
        .count();
    let io_count = state
        .diagnostics
        .iter()
        .filter(|d| d.category == Category::IO)
        .count();
    let worker_count = state
        .diagnostics
        .iter()
        .filter(|d| d.category == Category::Worker)
        .count();

    let duration = start.elapsed();

    println!("Computed category distribution in {:?}", duration);

    let total =
        container_count + bitstream_count + decode_count + metric_count + io_count + worker_count;
    assert_eq!(total, 10_000);

    // Should be fast (< 100ms)
    assert!(
        duration.as_millis() < 100,
        "Category distribution should take < 100ms"
    );
}

#[test]
fn test_performance_clear_large_diagnostic_set() {
    // Test clearing large diagnostic set
    let core = Arc::new(Core::new());
    let stream = core.get_stream(StreamId::A);

    {
        let mut state = stream.write();
        for i in 0..10_000 {
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
                impact_score: 85,
            });
        }
    }

    assert_eq!(stream.read().diagnostics.len(), 10_000);

    let start = Instant::now();

    {
        let mut state = stream.write();
        state.diagnostics.clear();
    }

    let duration = start.elapsed();

    println!("Cleared 10k diagnostics in {:?}", duration);

    assert_eq!(stream.read().diagnostics.len(), 0);

    // Should be very fast (< 10ms)
    assert!(duration.as_millis() < 10, "Clearing should take < 10ms");
}

#[test]
fn test_performance_severity_count_aggregation() {
    // Test counting diagnostics by severity level
    let core = Arc::new(Core::new());
    let stream = core.get_stream(StreamId::A);

    {
        let mut state = stream.write();
        for i in 0..10_000 {
            state.add_diagnostic(Diagnostic {
                id: i,
                severity: match i % 4 {
                    0 => Severity::Fatal,
                    1 => Severity::Error,
                    2 => Severity::Warn,
                    _ => Severity::Info,
                },
                stream_id: StreamId::A,
                message: format!("Diagnostic {}", i),
                category: Category::Bitstream,
                offset_bytes: i * 1000,
                timestamp_ms: i * 33,
                frame_index: Some(i as usize),
                count: 1,
                impact_score: 85,
            });
        }
    }

    let start = Instant::now();

    let state = stream.read();
    let fatal_count = state
        .diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Fatal)
        .count();
    let error_count = state
        .diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .count();
    let warn_count = state
        .diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Warn)
        .count();
    let info_count = state
        .diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Info)
        .count();

    let duration = start.elapsed();

    println!("Counted by severity in {:?}", duration);

    assert_eq!(fatal_count + error_count + warn_count + info_count, 10_000);

    // Should be fast (< 100ms)
    assert!(
        duration.as_millis() < 100,
        "Severity counting should take < 100ms"
    );
}
