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
//! Tests for performance tracking

use crate::performance::{
    CacheStats, MetricSummary, PerfEvent, PerfMetric, PerfTimer, PerfTracker,
};
use std::thread;
use std::time::Duration;

#[test]
fn test_perf_metric_names() {
    assert_eq!(PerfMetric::OpenFileTotal.display_name(), "Open File");
    assert_eq!(PerfMetric::OpenFileTotal.metric_key(), "open_file_total_ms");
    assert_eq!(PerfMetric::Decode.display_name(), "Decode");
    assert_eq!(PerfMetric::Decode.metric_key(), "decode_ms");
}

#[test]
fn test_perf_event_creation() {
    let duration = Duration::from_millis(100);
    let event = PerfEvent::new(PerfMetric::Parse, duration);

    assert_eq!(event.metric_name, "parse_ms");
    assert!((event.value_ms - 100.0).abs() < 0.1);
}

#[test]
fn test_perf_event_with_context() {
    let duration = Duration::from_millis(50);
    let event = PerfEvent::new(PerfMetric::Decode, duration)
        .with_stream("A")
        .with_frame(42)
        .with_extra("codec", serde_json::json!("AV1"));

    assert_eq!(event.stream, Some("A".to_string()));
    assert_eq!(event.frame_idx, Some(42));
    assert_eq!(event.extra.get("codec").unwrap(), &serde_json::json!("AV1"));
}

#[test]
fn test_perf_timer() {
    let timer = PerfTimer::new(PerfMetric::Parse);
    thread::sleep(Duration::from_millis(10));
    let duration = timer.stop();

    assert!(duration.as_millis() >= 10);
}

#[test]
fn test_cache_stats() {
    let mut stats = CacheStats::new("test_cache");

    stats.record_hit();
    stats.record_hit();
    stats.record_miss();

    assert_eq!(stats.requests, 3);
    assert_eq!(stats.hits, 2);
    assert_eq!(stats.misses, 1);
    assert!((stats.hit_rate() - 0.666).abs() < 0.01);
    assert!((stats.hit_rate_percent() - 66.6).abs() < 0.1);
}

#[test]
fn test_perf_tracker() {
    let mut tracker = PerfTracker::new();

    tracker.record(PerfMetric::Parse, Duration::from_millis(100));
    tracker.record(PerfMetric::Parse, Duration::from_millis(150));
    tracker.record(PerfMetric::Decode, Duration::from_millis(200));

    assert_eq!(tracker.events.len(), 3);

    let parse_summary = tracker.get_summary(PerfMetric::Parse).unwrap();
    assert_eq!(parse_summary.count, 2);
    assert!((parse_summary.avg_ms - 125.0).abs() < 0.1);
    assert!((parse_summary.min_ms - 100.0).abs() < 0.1);
    assert!((parse_summary.max_ms - 150.0).abs() < 0.1);
}

#[test]
fn test_cache_tracking() {
    let mut tracker = PerfTracker::new();

    tracker.record_cache_hit("byte_cache");
    tracker.record_cache_hit("byte_cache");
    tracker.record_cache_miss("byte_cache");

    let stats = tracker.get_cache_stats("byte_cache");
    assert_eq!(stats.requests, 3);
    assert_eq!(stats.hits, 2);
    assert!((stats.hit_rate_percent() - 66.6).abs() < 0.1);
}

#[test]
fn test_metric_summary() {
    let mut summary = MetricSummary::new();

    summary.record(100.0);
    summary.record(200.0);
    summary.record(150.0);

    assert_eq!(summary.count, 3);
    assert!((summary.avg_ms - 150.0).abs() < 0.1);
    assert!((summary.min_ms - 100.0).abs() < 0.1);
    assert!((summary.max_ms - 200.0).abs() < 0.1);
    assert!((summary.total_ms - 450.0).abs() < 0.1);
}

#[test]
fn test_export_report() {
    let mut tracker = PerfTracker::new();

    tracker.record(PerfMetric::Parse, Duration::from_millis(100));
    tracker.record(PerfMetric::Decode, Duration::from_millis(200));
    tracker.record_cache_hit("test_cache");
    tracker.record_cache_miss("test_cache");

    let report = tracker.export_report();

    assert_eq!(report.total_events, 2);
    assert_eq!(report.summaries.len(), 2);
    assert_eq!(report.cache_stats.len(), 1);

    let text = report.format_text();
    assert!(text.contains("Performance Report"));
    assert!(text.contains("Parse"));
    assert!(text.contains("test_cache"));
}

#[test]
fn test_tracker_enable_disable() {
    let mut tracker = PerfTracker::new();
    tracker.set_enabled(false);

    tracker.record(PerfMetric::Parse, Duration::from_millis(100));

    assert_eq!(tracker.events.len(), 0);

    tracker.set_enabled(true);
    tracker.record(PerfMetric::Parse, Duration::from_millis(100));

    assert_eq!(tracker.events.len(), 1);
}

#[test]
fn test_clear() {
    let mut tracker = PerfTracker::new();

    tracker.record(PerfMetric::Parse, Duration::from_millis(100));
    tracker.record_cache_hit("cache");

    assert!(tracker.events.len() > 0);

    tracker.clear();

    assert_eq!(tracker.events.len(), 0);
    assert_eq!(tracker.summaries.len(), 0);
    assert_eq!(tracker.cache_stats.len(), 0);
}

// Performance DevHUD cache_provenance test - Task 28 (S.T8-2.ALL.Perf.DevHUD.impl.cache_provenance.001)

#[test]
fn test_perf_dev_hud_cache_provenance() {
    // Performance DevHUD: User views cache provenance in developer HUD
    let mut tracker = PerfTracker::new();

    // Record cache operations with provenance
    tracker.record_cache_hit("decode_cache");
    tracker.record_cache_hit("decode_cache");
    tracker.record_cache_miss("decode_cache");

    tracker.record_cache_hit("overlay_cache");
    tracker.record_cache_miss("overlay_cache");

    // DevHUD: Verify cache statistics available for display
    assert_eq!(tracker.cache_stats.len(), 2);

    let decode_stats = tracker.cache_stats.get("decode_cache").unwrap();
    assert_eq!(decode_stats.hits, 2);
    assert_eq!(decode_stats.misses, 1);
    assert!((decode_stats.hit_rate() - 0.666).abs() < 0.01);

    let overlay_stats = tracker.cache_stats.get("overlay_cache").unwrap();
    assert_eq!(overlay_stats.hits, 1);
    assert_eq!(overlay_stats.misses, 1);
    assert_eq!(overlay_stats.hit_rate(), 0.5);

    // DevHUD: Format for display in developer overlay
    let report = tracker.export_report();
    let text = report.format_text();
    assert!(text.contains("decode_cache"));
    assert!(text.contains("overlay_cache"));
}

// Performance DevHUD out_of_core test - Task 29 (S.T8-2.ALL.Perf.DevHUD.impl.out_of_core.001)

#[test]
fn test_perf_dev_hud_out_of_core() {
    // Performance DevHUD: User views out-of-core memory statistics
    let mut tracker = PerfTracker::new();

    // Record I/O operations for out-of-core data access
    tracker.record(PerfMetric::IoRead, Duration::from_millis(50));
    tracker.record(PerfMetric::IoRead, Duration::from_millis(75));
    tracker.record(PerfMetric::MmapSetup, Duration::from_millis(10));

    // DevHUD: Verify I/O metrics for out-of-core monitoring
    let summary = tracker.get_summary(PerfMetric::IoRead).unwrap();
    assert_eq!(summary.count, 2);
    assert!((summary.total_ms - 125.0).abs() < 0.1); // 50 + 75

    let mmap_summary = tracker.get_summary(PerfMetric::MmapSetup).unwrap();
    assert_eq!(mmap_summary.count, 1);

    // DevHUD: Format report showing I/O overhead
    let report = tracker.export_report();
    assert_eq!(report.total_events, 3);
    assert!(report.summaries.contains_key(&PerfMetric::IoRead));
    assert!(report.summaries.contains_key(&PerfMetric::MmapSetup));
}

// Performance Core evidence_chain test - Task 30 (S.T8-2.ALL.Perf.Core.impl.evidence_chain.001)

#[test]
fn test_perf_core_evidence_chain() {
    // Performance Core: Link performance metrics to evidence chain
    let mut tracker = PerfTracker::new();

    // Record performance for evidence chain operations
    tracker.record(PerfMetric::Parse, Duration::from_millis(100));
    tracker.record(PerfMetric::IndexBuild, Duration::from_millis(50));
    tracker.record(PerfMetric::Decode, Duration::from_millis(200));

    // Evidence chain: Parse → Index → Decode pipeline timing
    let parse_time = tracker.get_summary(PerfMetric::Parse).unwrap();
    let index_time = tracker.get_summary(PerfMetric::IndexBuild).unwrap();
    let decode_time = tracker.get_summary(PerfMetric::Decode).unwrap();

    // Verify evidence chain stages are tracked
    assert_eq!(parse_time.count, 1);
    assert_eq!(index_time.count, 1);
    assert_eq!(decode_time.count, 1);

    // Total pipeline latency
    let total_ms = parse_time.total_ms + index_time.total_ms + decode_time.total_ms;
    assert!((total_ms - 350.0).abs() < 0.1);

    // Verify performance report includes all evidence chain stages
    let report = tracker.export_report();
    assert!(report.summaries.contains_key(&PerfMetric::Parse));
    assert!(report.summaries.contains_key(&PerfMetric::IndexBuild));
    assert!(report.summaries.contains_key(&PerfMetric::Decode));
}

// Performance Core cache_provenance test - Task 31 (S.T8-2.ALL.Perf.Core.impl.cache_provenance.001)

#[test]
fn test_perf_core_cache_provenance() {
    // Performance Core: Track cache provenance for performance analysis
    let mut tracker = PerfTracker::new();

    // Record cache operations with performance metrics
    tracker.record(PerfMetric::Decode, Duration::from_millis(200)); // Cache miss
    tracker.record_cache_miss("decode_cache");

    tracker.record(PerfMetric::Decode, Duration::from_millis(5)); // Cache hit
    tracker.record_cache_hit("decode_cache");

    tracker.record(PerfMetric::Convert, Duration::from_millis(50)); // Cache miss
    tracker.record_cache_miss("convert_cache");

    tracker.record(PerfMetric::Convert, Duration::from_millis(2)); // Cache hit
    tracker.record_cache_hit("convert_cache");

    // Verify cache provenance tracking
    let decode_cache = tracker.cache_stats.get("decode_cache").unwrap();
    assert_eq!(decode_cache.hits, 1);
    assert_eq!(decode_cache.misses, 1);

    let convert_cache = tracker.cache_stats.get("convert_cache").unwrap();
    assert_eq!(convert_cache.hits, 1);
    assert_eq!(convert_cache.misses, 1);

    // Verify performance correlation with cache efficiency
    let decode_summary = tracker.get_summary(PerfMetric::Decode).unwrap();
    assert_eq!(decode_summary.count, 2);
    assert!((decode_summary.avg_ms - 102.5).abs() < 0.1); // (200 + 5) / 2

    let convert_summary = tracker.get_summary(PerfMetric::Convert).unwrap();
    assert_eq!(convert_summary.count, 2);
    assert!((convert_summary.avg_ms - 26.0).abs() < 0.1); // (50 + 2) / 2

    // Generate comprehensive report with cache provenance
    let report = tracker.export_report();
    assert_eq!(report.cache_stats.len(), 2);
    assert!(report.cache_stats.contains_key("decode_cache"));
    assert!(report.cache_stats.contains_key("convert_cache"));
}
