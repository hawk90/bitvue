// Performance module tests
#[cfg(test)]
// use super::*; // Not needed - types are already in scope from parent module

// ============================================================================
// Fixtures
// ============================================================================
fn create_test_budget() -> PerfBudget {
    PerfBudget::new()
}

fn create_test_tracker() -> PerfTracker {
    PerfTracker::new()
}

fn create_test_cache_stats(name: &str) -> CacheStats {
    CacheStats::new(name)
}

fn create_test_event() -> PerfEvent {
    PerfEvent::new(PerfMetric::Parse, Duration::from_millis(100))
}

fn create_test_timer(metric: PerfMetric) -> PerfTimer {
    PerfTimer::new(metric)
}

fn create_test_timer_with_tracker(metric: PerfMetric) -> PerfTimer {
    let tracker = std::sync::Arc::new(std::sync::Mutex::new(create_test_tracker()));
    PerfTimer::with_tracker(metric, tracker)
}

// ============================================================================
// DegradeLevel Tests
// ============================================================================
#[cfg(test)]
mod degrade_level_tests {
    use super::*;

    #[test]
    fn test_degrade_level_default() {
        let level = DegradeLevel::default();
        assert_eq!(level, DegradeLevel::Full);
    }

    #[test]
    fn test_degrade_level_ordering() {
        assert!(DegradeLevel::Full < DegradeLevel::Medium);
        assert!(DegradeLevel::Medium < DegradeLevel::Low);
        assert!(DegradeLevel::Low < DegradeLevel::Minimal);
    }

    #[test]
    fn test_degrade_level_name() {
        assert_eq!(DegradeLevel::Full.name(), "Full");
        assert_eq!(DegradeLevel::Medium.name(), "Medium");
        assert_eq!(DegradeLevel::Low.name(), "Low");
        assert_eq!(DegradeLevel::Minimal.name(), "Minimal");
    }

    #[test]
    fn test_degrade_level_degrade() {
        assert_eq!(DegradeLevel::Full.degrade(), DegradeLevel::Medium);
        assert_eq!(DegradeLevel::Medium.degrade(), DegradeLevel::Low);
        assert_eq!(DegradeLevel::Low.degrade(), DegradeLevel::Minimal);
        assert_eq!(DegradeLevel::Minimal.degrade(), DegradeLevel::Minimal);
    }

    #[test]
    fn test_degrade_level_upgrade() {
        assert_eq!(DegradeLevel::Minimal.upgrade(), DegradeLevel::Low);
        assert_eq!(DegradeLevel::Low.upgrade(), DegradeLevel::Medium);
        assert_eq!(DegradeLevel::Medium.upgrade(), DegradeLevel::Full);
        assert_eq!(DegradeLevel::Full.upgrade(), DegradeLevel::Full);
    }
}

// ============================================================================
// PerfMetric Tests
// ============================================================================
#[cfg(test)]
mod perf_metric_tests {
    use super::*;

    #[test]
    fn test_perf_metric_display_name() {
        assert_eq!(PerfMetric::OpenFileTotal.display_name(), "Open File");
        assert_eq!(PerfMetric::IoRead.display_name(), "I/O Read");
        assert_eq!(PerfMetric::Parse.display_name(), "Parse");
        assert_eq!(PerfMetric::Decode.display_name(), "Decode");
        assert_eq!(PerfMetric::Convert.display_name(), "Convert");
        assert_eq!(PerfMetric::OverlayQp.display_name(), "QP Overlay");
        assert_eq!(PerfMetric::OverlayMv.display_name(), "MV Overlay");
        assert_eq!(PerfMetric::Paint.display_name(), "Paint");
        assert_eq!(PerfMetric::HitTest.display_name(), "Hit Test");
        assert_eq!(PerfMetric::TooltipBuild.display_name(), "Tooltip Build");
        assert_eq!(PerfMetric::SelectionPropagation.display_name(), "Selection Propagation");
        assert_eq!(PerfMetric::UiFrame.display_name(), "UI Frame");
    }

    #[test]
    fn test_perf_metric_metric_key() {
        assert_eq!(PerfMetric::OpenFileTotal.metric_key(), "open_file_total_ms");
        assert_eq!(PerfMetric::IoRead.metric_key(), "io_read_ms");
        assert_eq!(PerfMetric::Parse.metric_key(), "parse_ms");
        assert_eq!(PerfMetric::Decode.metric_key(), "decode_ms");
        assert_eq!(PerfMetric::HitTest.metric_key(), "hit_test_ms");
        assert_eq!(PerfMetric::TooltipBuild.metric_key(), "tooltip_build_ms");
        assert_eq!(PerfMetric::SelectionPropagation.metric_key(), "selection_propagation_ms");
        assert_eq!(PerfMetric::UiFrame.metric_key(), "ui_frame_ms");
    }
}

// ============================================================================
// PerfBudget Tests
// ============================================================================
#[cfg(test)]
mod perf_budget_tests {
    use super::*;

    #[test]
    fn test_perf_budget_new() {
        let budget = create_test_budget();
        assert_eq!(budget.degrade_level, DegradeLevel::Full);
        assert!((budget.last_frame_ms - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_perf_budget_record_frame_under_budget() {
        let mut budget = create_test_budget();
        let degraded = budget.record_frame(10.0);
        assert!(!degraded);
        assert_eq!(budget.degrade_level, DegradeLevel::Full);
    }

    #[test]
    fn test_perf_budget_record_frame_over_budget() {
        let mut budget = create_test_budget();
        budget.record_frame(20.0);
        assert_eq!(budget.degrade_level, DegradeLevel::Full);
        budget.record_frame(20.0);
        assert_eq!(budget.degrade_level, DegradeLevel::Full);
        let degraded = budget.record_frame(20.0);
        assert!(degraded);
        assert_eq!(budget.degrade_level, DegradeLevel::Medium);
    }

    #[test]
    fn test_perf_budget_upgrade_after_consecutive_under_budget() {
        let mut budget = create_test_budget();
        budget.degrade_level = DegradeLevel::Low;

        // Need 10 consecutive under-budget frames to upgrade
        for i in 0..9 {
            let upgraded = budget.record_frame(10.0);
            assert!(!upgraded, "Should not upgrade on frame {}", i);
        }

        let upgraded = budget.record_frame(10.0);
        assert!(upgraded);
        assert_eq!(budget.degrade_level, DegradeLevel::Medium);
    }

    #[test]
    fn test_perf_budget_record_hit_test_under() {
        let mut budget = create_test_budget();
        let over = budget.record_hit_test(1.0);
        assert!(!over);
    }

    #[test]
    fn test_perf_budget_record_hit_test_over() {
        let mut budget = create_test_budget();
        let over = budget.record_hit_test(2.0);
        assert!(over);
    }

    #[test]
    fn test_perf_budget_record_overlay_under() {
        let mut budget = create_test_budget();
        let over = budget.record_overlay(5.0);
        assert!(!over);
    }

    #[test]
    fn test_perf_budget_record_overlay_over() {
        let mut budget = create_test_budget();
        let over = budget.record_overlay(7.0);
        assert!(over);
    }

    #[test]
    fn test_perf_budget_record_tooltip_under() {
        let mut budget = create_test_budget();
        let over = budget.record_tooltip(0.5);
        assert!(!over);
    }

    #[test]
    fn test_perf_budget_record_tooltip_over() {
        let mut budget = create_test_budget();
        let over = budget.record_tooltip(1.0);
        assert!(over);
    }

    #[test]
    fn test_perf_budget_record_selection_under() {
        let mut budget = create_test_budget();
        let over = budget.record_selection(1.5);
        assert!(!over);
    }

    #[test]
    fn test_perf_budget_record_selection_over() {
        let mut budget = create_test_budget();
        let over = budget.record_selection(3.0);
        assert!(over);
    }

    #[test]
    fn test_perf_budget_should_simplify_overlays() {
        let mut budget = create_test_budget();

        budget.degrade_level = DegradeLevel::Full;
        assert!(!budget.should_simplify_overlays());

        budget.degrade_level = DegradeLevel::Medium;
        assert!(budget.should_simplify_overlays());

        budget.degrade_level = DegradeLevel::Low;
        assert!(budget.should_simplify_overlays());
    }

    #[test]
    fn test_perf_budget_should_disable_expensive_overlays() {
        let mut budget = create_test_budget();

        budget.degrade_level = DegradeLevel::Full;
        assert!(!budget.should_disable_expensive_overlays());

        budget.degrade_level = DegradeLevel::Medium;
        assert!(!budget.should_disable_expensive_overlays());

        budget.degrade_level = DegradeLevel::Low;
        assert!(budget.should_disable_expensive_overlays());
    }

    #[test]
    fn test_perf_budget_should_defer_tooltips() {
        let mut budget = create_test_budget();

        budget.degrade_level = DegradeLevel::Full;
        assert!(!budget.should_defer_tooltips());

        budget.degrade_level = DegradeLevel::Low;
        assert!(budget.should_defer_tooltips());
    }

    #[test]
    fn test_perf_budget_overlay_lod() {
        let mut budget = create_test_budget();

        budget.degrade_level = DegradeLevel::Full;
        assert!((budget.overlay_lod() - 1.0).abs() < f32::EPSILON);

        budget.degrade_level = DegradeLevel::Medium;
        assert!((budget.overlay_lod() - 0.75).abs() < f32::EPSILON);

        budget.degrade_level = DegradeLevel::Low;
        assert!((budget.overlay_lod() - 0.5).abs() < f32::EPSILON);

        budget.degrade_level = DegradeLevel::Minimal;
        assert!((budget.overlay_lod() - 0.25).abs() < f32::EPSILON);
    }

    #[test]
    fn test_perf_budget_reset() {
        let mut budget = create_test_budget();
        budget.degrade_level = DegradeLevel::Minimal;
        budget.record_frame(20.0);
        budget.record_frame(20.0);

        budget.reset();

        assert_eq!(budget.degrade_level, DegradeLevel::Full);
        assert_eq!(budget.over_budget_count, 0);
        assert_eq!(budget.under_budget_count, 0);
    }
}

// ============================================================================
// PerfEvent Tests
// ============================================================================
#[cfg(test)]
mod perf_event_tests {
    use super::*;

    #[test]
    fn test_perf_event_new() {
        let event = PerfEvent::new(PerfMetric::Parse, Duration::from_millis(100));

        assert_eq!(event.metric_name, "parse_ms");
        assert!((event.value_ms - 100.0).abs() < 0.01);
        assert!(event.stream.is_none());
        assert!(event.frame_idx.is_none());
    }

    #[test]
    fn test_perf_event_with_stream() {
        let event = create_test_event()
            .with_stream("test_stream");

        assert_eq!(event.stream, Some("test_stream".to_string()));
    }

    #[test]
    fn test_perf_event_with_frame() {
        let event = create_test_event()
            .with_frame(42);

        assert_eq!(event.frame_idx, Some(42));
    }

    #[test]
    fn test_perf_event_with_extra() {
        let event = create_test_event()
            .with_extra("custom_field", serde_json::json!(123));

        assert_eq!(event.extra.get("custom_field"), Some(&serde_json::json!(123)));
    }

    #[test]
    fn test_perf_event_to_json_line() {
        let event = create_test_event();
        let json = event.to_json_line();

        assert!(json.contains("\"metric_name\":\"parse_ms\""));
        assert!(json.contains("\"value_ms\":100.0"));
    }

    #[test]
    fn test_perf_event_chaining() {
        let event = create_test_event()
            .with_stream("stream1")
            .with_frame(10)
            .with_extra("key1", serde_json::json!("value1"))
            .with_extra("key2", serde_json::json!(42));

        assert_eq!(event.stream, Some("stream1".to_string()));
        assert_eq!(event.frame_idx, Some(10));
        assert_eq!(event.extra.len(), 2);
    }
}

// ============================================================================
// PerfTimer Tests
// ============================================================================
#[cfg(test)]
mod perf_timer_tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_perf_timer_new() {
        let timer = create_test_timer(PerfMetric::Parse);
        thread::sleep(Duration::from_millis(1));  // Ensure some time passes
        assert!(!timer.elapsed().is_zero());
    }

    #[test]
    fn test_perf_timer_elapsed() {
        let timer = create_test_timer(PerfMetric::Parse);
        thread::sleep(Duration::from_millis(10));
        let elapsed = timer.elapsed();

        assert!(elapsed.as_millis() >= 10);
    }

    #[test]
    fn test_perf_timer_stop() {
        let timer = create_test_timer(PerfMetric::Parse);
        thread::sleep(Duration::from_millis(10));
        let duration = timer.stop();

        assert!(duration.as_millis() >= 10);
    }

    #[test]
    fn test_perf_timer_with_tracker() {
        let tracker = std::sync::Arc::new(std::sync::Mutex::new(create_test_tracker()));
        {
            let _timer = PerfTimer::with_tracker(PerfMetric::Parse, tracker.clone());
            thread::sleep(Duration::from_millis(10));
        } // Timer drops here

        let tracker_locked = tracker.lock().unwrap();
        assert_eq!(tracker_locked.events.len(), 1);
    }
}

// ============================================================================
// CacheStats Tests
// ============================================================================
#[cfg(test)]
mod cache_stats_tests {
    use super::*;

    #[test]
    fn test_cache_stats_new() {
        let stats = create_test_cache_stats("test_cache");
        assert_eq!(stats.name, "test_cache");
        assert_eq!(stats.requests, 0);
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 0);
    }

    #[test]
    fn test_cache_stats_record_hit() {
        let mut stats = create_test_cache_stats("test_cache");
        stats.record_hit();

        assert_eq!(stats.requests, 1);
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 0);
    }

    #[test]
    fn test_cache_stats_record_miss() {
        let mut stats = create_test_cache_stats("test_cache");
        stats.record_miss();

        assert_eq!(stats.requests, 1);
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 1);
    }

    #[test]
    fn test_cache_stats_hit_rate() {
        let mut stats = create_test_cache_stats("test_cache");
        assert_eq!(stats.hit_rate(), 0.0);

        stats.record_hit();
        assert_eq!(stats.hit_rate(), 1.0);

        stats.record_miss();
        assert_eq!(stats.hit_rate(), 0.5);

        stats.record_hit();
        assert!((stats.hit_rate() - 0.6667).abs() < 0.01);
    }

    #[test]
    fn test_cache_stats_hit_rate_empty() {
        let stats = create_test_cache_stats("test_cache");
        assert_eq!(stats.hit_rate(), 0.0);
    }

    #[test]
    fn test_cache_stats_hit_rate_percent() {
        let mut stats = create_test_cache_stats("test_cache");
        stats.record_hit();
        stats.record_miss();

        assert_eq!(stats.hit_rate_percent(), 50.0);
    }

    #[test]
    fn test_cache_stats_reset() {
        let mut stats = create_test_cache_stats("test_cache");
        stats.record_hit();
        stats.record_miss();

        stats.reset();

        assert_eq!(stats.requests, 0);
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 0);
    }
}

// ============================================================================
// PerfTracker Tests
// ============================================================================
#[cfg(test)]
mod perf_tracker_tests {
    use super::*;

    #[test]
    fn test_perf_tracker_new() {
        let tracker = create_test_tracker();
        assert!(tracker.events.is_empty());
        assert!(tracker.enabled);
    }

    #[test]
    fn test_perf_tracker_record() {
        let mut tracker = create_test_tracker();
        tracker.record(PerfMetric::Parse, Duration::from_millis(100));

        assert_eq!(tracker.events.len(), 1);
        assert!(tracker.summaries.contains_key(&PerfMetric::Parse));
    }

    #[test]
    fn test_perf_tracker_record_disabled() {
        let mut tracker = create_test_tracker();
        tracker.enabled = false;
        tracker.record(PerfMetric::Parse, Duration::from_millis(100));

        assert_eq!(tracker.events.len(), 0);
    }

    #[test]
    fn test_perf_tracker_record_event() {
        let mut tracker = create_test_tracker();
        let event = create_test_event();
        tracker.record_event(event.clone());

        assert_eq!(tracker.events.len(), 1);
    }

    #[test]
    fn test_perf_tracker_get_cache_stats() {
        let mut tracker = create_test_tracker();
        let stats = tracker.get_cache_stats("test_cache");

        assert_eq!(stats.name, "test_cache");
    }

    #[test]
    fn test_perf_tracker_cache_stats_reuse() {
        let mut tracker = create_test_tracker();
        let stats1 = tracker.get_cache_stats("test_cache");
        stats1.record_hit();

        let stats2 = tracker.get_cache_stats("test_cache");
        assert_eq!(stats2.hits, 1);
    }

    #[test]
    fn test_perf_tracker_record_cache_hit() {
        let mut tracker = create_test_tracker();
        tracker.record_cache_hit("test_cache");

        let stats = tracker.get_cache_stats("test_cache");
        assert_eq!(stats.hits, 1);
    }

    #[test]
    fn test_perf_tracker_record_cache_miss() {
        let mut tracker = create_test_tracker();
        tracker.record_cache_miss("test_cache");

        let stats = tracker.get_cache_stats("test_cache");
        assert_eq!(stats.misses, 1);
    }

    #[test]
    fn test_perf_tracker_get_summary() {
        let mut tracker = create_test_tracker();
        tracker.record(PerfMetric::Parse, Duration::from_millis(100));

        let summary = tracker.get_summary(PerfMetric::Parse);
        assert!(summary.is_some());
        assert_eq!(summary.unwrap().count, 1);
    }

    #[test]
    fn test_perf_tracker_get_summary_none() {
        let tracker = create_test_tracker();
        let summary = tracker.get_summary(PerfMetric::Parse);
        assert!(summary.is_none());
    }

    #[test]
    fn test_perf_tracker_export_json_lines() {
        let mut tracker = create_test_tracker();
        tracker.record(PerfMetric::Parse, Duration::from_millis(100));

        let lines = tracker.export_json_lines();
        assert_eq!(lines.len(), 1);
        assert!(lines[0].contains("\"metric_name\":\"parse_ms\""));
    }

    #[test]
    fn test_perf_tracker_export_report() {
        let mut tracker = create_test_tracker();
        tracker.record(PerfMetric::Parse, Duration::from_millis(100));
        tracker.record_cache_hit("test_cache");
        tracker.record_cache_miss("test_cache");

        let report = tracker.export_report();
        assert_eq!(report.total_events, 1);
        assert!(report.summaries.contains_key(&PerfMetric::Parse));
        assert!(report.cache_stats.contains_key("test_cache"));
    }

    #[test]
    fn test_perf_tracker_clear() {
        let mut tracker = create_test_tracker();
        tracker.record(PerfMetric::Parse, Duration::from_millis(100));
        tracker.record_cache_hit("test_cache");

        tracker.clear();

        assert!(tracker.events.is_empty());
        assert!(tracker.summaries.is_empty());
        assert!(tracker.cache_stats.is_empty());
    }

    #[test]
    fn test_perf_tracker_set_enabled() {
        let mut tracker = create_test_tracker();
        tracker.set_enabled(false);
        assert!(!tracker.enabled);

        tracker.set_enabled(true);
        assert!(tracker.enabled);
    }
}

// ============================================================================
// MetricSummary Tests
// ============================================================================
#[cfg(test)]
mod metric_summary_tests {
    use super::*;

    #[test]
    fn test_metric_summary_new() {
        let summary = MetricSummary::new();
        assert_eq!(summary.count, 0);
        assert_eq!(summary.total_ms, 0.0);
        assert_eq!(summary.min_ms, f64::MAX);
        assert_eq!(summary.max_ms, 0.0);
    }

    #[test]
    fn test_metric_summary_record() {
        let mut summary = MetricSummary::new();
        summary.record(100.0);
        summary.record(200.0);
        summary.record(150.0);

        assert_eq!(summary.count, 3);
        assert_eq!(summary.total_ms, 450.0);
        assert_eq!(summary.min_ms, 100.0);
        assert_eq!(summary.max_ms, 200.0);
        assert!((summary.avg_ms - 150.0).abs() < 0.01);
    }

    #[test]
    fn test_metric_summary_single_value() {
        let mut summary = MetricSummary::new();
        summary.record(42.0);

        assert_eq!(summary.count, 1);
        assert_eq!(summary.min_ms, 42.0);
        assert_eq!(summary.max_ms, 42.0);
        assert_eq!(summary.avg_ms, 42.0);
    }
}

// ============================================================================
// PerfReport Tests
// ============================================================================
#[cfg(test)]
mod perf_report_tests {
    use super::*;

    #[test]
    fn test_perf_report_format_text() {
        let mut tracker = create_test_tracker();
        tracker.record(PerfMetric::Parse, Duration::from_millis(100));
        tracker.record(PerfMetric::Parse, Duration::from_millis(200));
        tracker.record_cache_hit("test_cache");
        tracker.record_cache_hit("test_cache");
        tracker.record_cache_miss("test_cache");

        let report = tracker.export_report();
        let text = report.format_text();

        assert!(text.contains("Performance Report"));
        assert!(text.contains("Total events: 2"));
        assert!(text.contains("Parse"));
        assert!(text.contains("avg=150.00ms"));
        assert!(text.contains("test_cache"));
        assert!(text.contains("66.7%"));
    }
}
