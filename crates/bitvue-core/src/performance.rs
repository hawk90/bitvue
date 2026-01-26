//! Performance Instrumentation - T9-1
//!
//! Per PERF_PROFILING_INSTRUMENTATION.md:
//! - Required timers: open_file, io_read, mmap, parse, index, decode, convert, overlay, upload, paint
//! - Cache hit rate tracking
//! - Developer HUD toggle
//! - Exportable performance reports
//! - JSON logging format per event
//!
//! Per perf_budget_and_instrumentation.json (VQAnalyzer parity):
//! - Performance budgets with automatic degradation
//! - LOD virtualization when over budget

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Performance budget constants (ms) - VQAnalyzer parity
pub mod budget {
    /// Target frame time for 60fps (16.6ms)
    pub const UI_FRAME_TARGET_MS: f64 = 16.6;

    /// Maximum time for hit testing (1.5ms)
    pub const HIT_TEST_MAX_MS: f64 = 1.5;

    /// Maximum time for overlay rendering (6.0ms)
    pub const OVERLAY_RENDER_MAX_MS: f64 = 6.0;

    /// Maximum time for tooltip building (0.8ms)
    pub const TOOLTIP_BUILD_MAX_MS: f64 = 0.8;

    /// Maximum time for selection propagation (2.0ms)
    pub const SELECTION_PROPAGATION_MAX_MS: f64 = 2.0;
}

/// Degradation level for adaptive LOD
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum DegradeLevel {
    /// Full quality - all features enabled
    Full = 0,
    /// Medium quality - reduce overlay detail
    Medium = 1,
    /// Low quality - disable expensive overlays
    Low = 2,
    /// Minimal - only essential rendering
    Minimal = 3,
}

impl Default for DegradeLevel {
    fn default() -> Self {
        Self::Full
    }
}

impl DegradeLevel {
    /// Get human-readable name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Full => "Full",
            Self::Medium => "Medium",
            Self::Low => "Low",
            Self::Minimal => "Minimal",
        }
    }

    /// Degrade one level (if possible)
    pub fn degrade(&self) -> Self {
        match self {
            Self::Full => Self::Medium,
            Self::Medium => Self::Low,
            Self::Low => Self::Minimal,
            Self::Minimal => Self::Minimal,
        }
    }

    /// Upgrade one level (if possible)
    pub fn upgrade(&self) -> Self {
        match self {
            Self::Full => Self::Full,
            Self::Medium => Self::Full,
            Self::Low => Self::Medium,
            Self::Minimal => Self::Low,
        }
    }
}

/// Performance budget checker with adaptive degradation
#[derive(Debug, Clone, Default)]
pub struct PerfBudget {
    /// Current degradation level
    pub degrade_level: DegradeLevel,

    /// Last frame time (ms)
    pub last_frame_ms: f64,

    /// Last hit test time (ms)
    pub last_hit_test_ms: f64,

    /// Last overlay render time (ms)
    pub last_overlay_ms: f64,

    /// Last tooltip build time (ms)
    pub last_tooltip_ms: f64,

    /// Last selection propagation time (ms)
    pub last_selection_ms: f64,

    /// Consecutive frames over budget
    over_budget_count: u32,

    /// Consecutive frames under budget
    under_budget_count: u32,
}

impl PerfBudget {
    /// Create new performance budget tracker
    pub fn new() -> Self {
        Self::default()
    }

    /// Record frame timing and check if degradation needed
    pub fn record_frame(&mut self, frame_ms: f64) -> bool {
        self.last_frame_ms = frame_ms;

        if frame_ms > budget::UI_FRAME_TARGET_MS {
            self.over_budget_count += 1;
            self.under_budget_count = 0;

            // Degrade after 3 consecutive over-budget frames
            if self.over_budget_count >= 3 && self.degrade_level != DegradeLevel::Minimal {
                self.degrade_level = self.degrade_level.degrade();
                self.over_budget_count = 0;
                return true; // Degraded
            }
        } else {
            self.under_budget_count += 1;
            self.over_budget_count = 0;

            // Upgrade after 10 consecutive under-budget frames
            if self.under_budget_count >= 10 && self.degrade_level != DegradeLevel::Full {
                self.degrade_level = self.degrade_level.upgrade();
                self.under_budget_count = 0;
                return true; // Upgraded
            }
        }

        false
    }

    /// Record hit test timing
    pub fn record_hit_test(&mut self, ms: f64) -> bool {
        self.last_hit_test_ms = ms;
        ms > budget::HIT_TEST_MAX_MS
    }

    /// Record overlay render timing
    pub fn record_overlay(&mut self, ms: f64) -> bool {
        self.last_overlay_ms = ms;
        ms > budget::OVERLAY_RENDER_MAX_MS
    }

    /// Record tooltip build timing
    pub fn record_tooltip(&mut self, ms: f64) -> bool {
        self.last_tooltip_ms = ms;
        ms > budget::TOOLTIP_BUILD_MAX_MS
    }

    /// Record selection propagation timing
    pub fn record_selection(&mut self, ms: f64) -> bool {
        self.last_selection_ms = ms;
        ms > budget::SELECTION_PROPAGATION_MAX_MS
    }

    /// Check if overlays should be simplified based on current degradation
    pub fn should_simplify_overlays(&self) -> bool {
        self.degrade_level >= DegradeLevel::Medium
    }

    /// Check if expensive overlays should be disabled
    pub fn should_disable_expensive_overlays(&self) -> bool {
        self.degrade_level >= DegradeLevel::Low
    }

    /// Check if tooltips should be deferred
    pub fn should_defer_tooltips(&self) -> bool {
        self.degrade_level >= DegradeLevel::Low || self.last_tooltip_ms > budget::TOOLTIP_BUILD_MAX_MS
    }

    /// Get recommended overlay LOD (0.0 = minimal, 1.0 = full)
    pub fn overlay_lod(&self) -> f32 {
        match self.degrade_level {
            DegradeLevel::Full => 1.0,
            DegradeLevel::Medium => 0.75,
            DegradeLevel::Low => 0.5,
            DegradeLevel::Minimal => 0.25,
        }
    }

    /// Reset degradation to full quality
    pub fn reset(&mut self) {
        self.degrade_level = DegradeLevel::Full;
        self.over_budget_count = 0;
        self.under_budget_count = 0;
    }
}

/// Performance metric type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PerfMetric {
    /// Total file open time
    OpenFileTotal,
    /// I/O read operations
    IoRead,
    /// Memory mapping setup
    MmapSetup,
    /// Parsing time
    Parse,
    /// Index building
    IndexBuild,
    /// Decoding time
    Decode,
    /// YUV->RGBA conversion
    Convert,
    /// QP overlay building
    OverlayQp,
    /// MV overlay building
    OverlayMv,
    /// Grid overlay building
    OverlayGrid,
    /// Diff overlay building
    OverlayDiff,
    /// Texture upload
    UploadTexture,
    /// Frame painting (egui)
    Paint,
    /// Hit testing (VQAnalyzer parity budget)
    HitTest,
    /// Tooltip building (VQAnalyzer parity budget)
    TooltipBuild,
    /// Selection propagation (VQAnalyzer parity budget)
    SelectionPropagation,
    /// Total UI frame time (VQAnalyzer parity budget)
    UiFrame,
}

impl PerfMetric {
    /// Get display name
    pub fn display_name(&self) -> &'static str {
        match self {
            PerfMetric::OpenFileTotal => "Open File",
            PerfMetric::IoRead => "I/O Read",
            PerfMetric::MmapSetup => "Mmap Setup",
            PerfMetric::Parse => "Parse",
            PerfMetric::IndexBuild => "Index Build",
            PerfMetric::Decode => "Decode",
            PerfMetric::Convert => "Convert",
            PerfMetric::OverlayQp => "QP Overlay",
            PerfMetric::OverlayMv => "MV Overlay",
            PerfMetric::OverlayGrid => "Grid Overlay",
            PerfMetric::OverlayDiff => "Diff Overlay",
            PerfMetric::UploadTexture => "Upload Texture",
            PerfMetric::Paint => "Paint",
            PerfMetric::HitTest => "Hit Test",
            PerfMetric::TooltipBuild => "Tooltip Build",
            PerfMetric::SelectionPropagation => "Selection Propagation",
            PerfMetric::UiFrame => "UI Frame",
        }
    }

    /// Get metric key for JSON export
    pub fn metric_key(&self) -> &'static str {
        match self {
            PerfMetric::OpenFileTotal => "open_file_total_ms",
            PerfMetric::IoRead => "io_read_ms",
            PerfMetric::MmapSetup => "mmap_setup_ms",
            PerfMetric::Parse => "parse_ms",
            PerfMetric::IndexBuild => "index_build_ms",
            PerfMetric::Decode => "decode_ms",
            PerfMetric::Convert => "convert_ms",
            PerfMetric::OverlayQp => "overlay_qp_ms",
            PerfMetric::OverlayMv => "overlay_mv_ms",
            PerfMetric::OverlayGrid => "overlay_grid_ms",
            PerfMetric::OverlayDiff => "overlay_diff_ms",
            PerfMetric::UploadTexture => "upload_texture_ms",
            PerfMetric::Paint => "paint_ms",
            PerfMetric::HitTest => "hit_test_ms",
            PerfMetric::TooltipBuild => "tooltip_build_ms",
            PerfMetric::SelectionPropagation => "selection_propagation_ms",
            PerfMetric::UiFrame => "ui_frame_ms",
        }
    }
}

/// Performance event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerfEvent {
    /// Timestamp (ms since epoch)
    pub timestamp_ms: u64,

    /// Stream ID (if applicable)
    pub stream: Option<String>,

    /// Frame index (if applicable)
    pub frame_idx: Option<usize>,

    /// Metric name
    pub metric_name: String,

    /// Duration (ms)
    pub value_ms: f64,

    /// Extra fields
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

impl PerfEvent {
    /// Create a new performance event
    pub fn new(metric: PerfMetric, duration: Duration) -> Self {
        Self {
            timestamp_ms: 0, // Would be set from system time
            stream: None,
            frame_idx: None,
            metric_name: metric.metric_key().to_string(),
            value_ms: duration.as_secs_f64() * 1000.0,
            extra: HashMap::new(),
        }
    }

    /// Set stream
    pub fn with_stream(mut self, stream: impl Into<String>) -> Self {
        self.stream = Some(stream.into());
        self
    }

    /// Set frame index
    pub fn with_frame(mut self, frame_idx: usize) -> Self {
        self.frame_idx = Some(frame_idx);
        self
    }

    /// Add extra field
    pub fn with_extra(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.extra.insert(key.into(), value);
        self
    }

    /// Format as JSON line
    pub fn to_json_line(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }
}

/// Performance timer
///
/// RAII timer that records elapsed time on drop.
pub struct PerfTimer {
    /// Metric type
    metric: PerfMetric,

    /// Start time
    start: Instant,

    /// Performance tracker reference
    tracker: Option<std::sync::Arc<std::sync::Mutex<PerfTracker>>>,
}

impl PerfTimer {
    /// Create a new timer
    pub fn new(metric: PerfMetric) -> Self {
        Self {
            metric,
            start: Instant::now(),
            tracker: None,
        }
    }

    /// Create a timer with tracker
    pub fn with_tracker(
        metric: PerfMetric,
        tracker: std::sync::Arc<std::sync::Mutex<PerfTracker>>,
    ) -> Self {
        Self {
            metric,
            start: Instant::now(),
            tracker: Some(tracker),
        }
    }

    /// Get elapsed duration
    pub fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }

    /// Stop timer and return duration
    pub fn stop(self) -> Duration {
        self.elapsed()
    }
}

impl Drop for PerfTimer {
    fn drop(&mut self) {
        let duration = self.elapsed();
        if let Some(ref tracker) = self.tracker {
            if let Ok(mut t) = tracker.lock() {
                t.record(self.metric, duration);
            }
        }
    }
}

/// Cache statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CacheStats {
    /// Cache name
    pub name: String,

    /// Total requests
    pub requests: u64,

    /// Cache hits
    pub hits: u64,

    /// Cache misses
    pub misses: u64,
}

impl CacheStats {
    /// Create new cache stats
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            requests: 0,
            hits: 0,
            misses: 0,
        }
    }

    /// Record a hit
    pub fn record_hit(&mut self) {
        self.requests += 1;
        self.hits += 1;
    }

    /// Record a miss
    pub fn record_miss(&mut self) {
        self.requests += 1;
        self.misses += 1;
    }

    /// Get hit rate (0.0 to 1.0)
    pub fn hit_rate(&self) -> f64 {
        if self.requests == 0 {
            0.0
        } else {
            self.hits as f64 / self.requests as f64
        }
    }

    /// Get hit rate percentage
    pub fn hit_rate_percent(&self) -> f64 {
        self.hit_rate() * 100.0
    }

    /// Reset stats
    pub fn reset(&mut self) {
        self.requests = 0;
        self.hits = 0;
        self.misses = 0;
    }
}

/// Performance tracker
///
/// Central collector for performance metrics and cache statistics.
#[derive(Debug, Clone, Default)]
pub struct PerfTracker {
    /// Performance events
    pub events: Vec<PerfEvent>,

    /// Metric summaries
    pub summaries: HashMap<PerfMetric, MetricSummary>,

    /// Cache statistics
    pub cache_stats: HashMap<String, CacheStats>,

    /// Enable tracking
    pub enabled: bool,
}

impl PerfTracker {
    /// Create new performance tracker
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            summaries: HashMap::new(),
            cache_stats: HashMap::new(),
            enabled: true,
        }
    }

    /// Record a performance event
    pub fn record(&mut self, metric: PerfMetric, duration: Duration) {
        if !self.enabled {
            return;
        }

        let event = PerfEvent::new(metric, duration);
        self.events.push(event.clone());

        // Update summary
        let summary = self.summaries.entry(metric).or_default();
        summary.record(duration.as_secs_f64() * 1000.0);
    }

    /// Record a custom event
    pub fn record_event(&mut self, event: PerfEvent) {
        if self.enabled {
            self.events.push(event);
        }
    }

    /// Get or create cache stats
    pub fn get_cache_stats(&mut self, cache_name: &str) -> &mut CacheStats {
        self.cache_stats
            .entry(cache_name.to_string())
            .or_insert_with(|| CacheStats::new(cache_name))
    }

    /// Record cache hit
    pub fn record_cache_hit(&mut self, cache_name: &str) {
        self.get_cache_stats(cache_name).record_hit();
    }

    /// Record cache miss
    pub fn record_cache_miss(&mut self, cache_name: &str) {
        self.get_cache_stats(cache_name).record_miss();
    }

    /// Get metric summary
    pub fn get_summary(&self, metric: PerfMetric) -> Option<&MetricSummary> {
        self.summaries.get(&metric)
    }

    /// Export to JSON lines
    pub fn export_json_lines(&self) -> Vec<String> {
        self.events.iter().map(|e| e.to_json_line()).collect()
    }

    /// Export to performance report
    pub fn export_report(&self) -> PerfReport {
        PerfReport {
            summaries: self.summaries.clone(),
            cache_stats: self.cache_stats.clone(),
            total_events: self.events.len(),
        }
    }

    /// Clear all data
    pub fn clear(&mut self) {
        self.events.clear();
        self.summaries.clear();
        self.cache_stats.clear();
    }

    /// Enable/disable tracking
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
}

/// Metric summary statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricSummary {
    /// Count
    pub count: usize,

    /// Total time (ms)
    pub total_ms: f64,

    /// Min time (ms)
    pub min_ms: f64,

    /// Max time (ms)
    pub max_ms: f64,

    /// Average time (ms)
    pub avg_ms: f64,
}

impl MetricSummary {
    /// Create new summary
    pub fn new() -> Self {
        Self {
            count: 0,
            total_ms: 0.0,
            min_ms: f64::MAX,
            max_ms: 0.0,
            avg_ms: 0.0,
        }
    }

    /// Record a measurement
    pub fn record(&mut self, value_ms: f64) {
        self.count += 1;
        self.total_ms += value_ms;
        self.min_ms = self.min_ms.min(value_ms);
        self.max_ms = self.max_ms.max(value_ms);
        self.avg_ms = self.total_ms / self.count as f64;
    }
}

impl Default for MetricSummary {
    fn default() -> Self {
        Self::new()
    }
}

/// Performance report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerfReport {
    /// Metric summaries
    pub summaries: HashMap<PerfMetric, MetricSummary>,

    /// Cache statistics
    pub cache_stats: HashMap<String, CacheStats>,

    /// Total events recorded
    pub total_events: usize,
}

impl PerfReport {
    /// Format as human-readable text
    pub fn format_text(&self) -> String {
        let mut lines = Vec::new();

        lines.push("=== Performance Report ===".to_string());
        lines.push(format!("Total events: {}", self.total_events));
        lines.push("".to_string());

        // Metric summaries
        lines.push("Metrics:".to_string());
        let mut metrics: Vec<_> = self.summaries.iter().collect();
        metrics.sort_by_key(|(m, _)| m.metric_key());

        for (metric, summary) in metrics {
            lines.push(format!(
                "  {}: count={}, avg={:.2}ms, min={:.2}ms, max={:.2}ms, total={:.2}ms",
                metric.display_name(),
                summary.count,
                summary.avg_ms,
                summary.min_ms,
                summary.max_ms,
                summary.total_ms
            ));
        }

        // Cache stats
        if !self.cache_stats.is_empty() {
            lines.push("".to_string());
            lines.push("Cache Hit Rates:".to_string());

            let mut caches: Vec<_> = self.cache_stats.iter().collect();
            caches.sort_by_key(|(name, _)| *name);

            for (name, stats) in caches {
                lines.push(format!(
                    "  {}: {:.1}% ({}/{} requests)",
                    name,
                    stats.hit_rate_percent(),
                    stats.hits,
                    stats.requests
                ));
            }
        }

        lines.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_budget_constants() {
        // Verify budget constants match perf_budget_and_instrumentation.json
        assert!((budget::UI_FRAME_TARGET_MS - 16.6).abs() < f64::EPSILON);
        assert!((budget::HIT_TEST_MAX_MS - 1.5).abs() < f64::EPSILON);
        assert!((budget::OVERLAY_RENDER_MAX_MS - 6.0).abs() < f64::EPSILON);
        assert!((budget::TOOLTIP_BUILD_MAX_MS - 0.8).abs() < f64::EPSILON);
        assert!((budget::SELECTION_PROPAGATION_MAX_MS - 2.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_degrade_level_ordering() {
        assert!(DegradeLevel::Full < DegradeLevel::Medium);
        assert!(DegradeLevel::Medium < DegradeLevel::Low);
        assert!(DegradeLevel::Low < DegradeLevel::Minimal);
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

    #[test]
    fn test_perf_budget_new() {
        let budget = PerfBudget::new();
        assert_eq!(budget.degrade_level, DegradeLevel::Full);
        assert!((budget.last_frame_ms - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_perf_budget_degrade_after_consecutive_over_budget() {
        let mut budget = PerfBudget::new();

        // First 2 over-budget frames shouldn't degrade
        budget.record_frame(20.0);
        assert_eq!(budget.degrade_level, DegradeLevel::Full);
        budget.record_frame(20.0);
        assert_eq!(budget.degrade_level, DegradeLevel::Full);

        // Third over-budget frame should trigger degradation
        let degraded = budget.record_frame(20.0);
        assert!(degraded);
        assert_eq!(budget.degrade_level, DegradeLevel::Medium);
    }

    #[test]
    fn test_perf_budget_upgrade_after_consecutive_under_budget() {
        let mut budget = PerfBudget::new();
        budget.degrade_level = DegradeLevel::Low;

        // Need 10 consecutive under-budget frames to upgrade
        for i in 0..9 {
            let upgraded = budget.record_frame(10.0);
            assert!(!upgraded, "Should not upgrade on frame {}", i);
        }

        // 10th under-budget frame should upgrade
        let upgraded = budget.record_frame(10.0);
        assert!(upgraded);
        assert_eq!(budget.degrade_level, DegradeLevel::Medium);
    }

    #[test]
    fn test_perf_budget_over_budget_detection() {
        let mut budget = PerfBudget::new();

        assert!(!budget.record_hit_test(1.0)); // Under budget
        assert!(budget.record_hit_test(2.0));  // Over budget

        assert!(!budget.record_overlay(5.0)); // Under budget
        assert!(budget.record_overlay(7.0));  // Over budget

        assert!(!budget.record_tooltip(0.5)); // Under budget
        assert!(budget.record_tooltip(1.0));  // Over budget

        assert!(!budget.record_selection(1.5)); // Under budget
        assert!(budget.record_selection(3.0));  // Over budget
    }

    #[test]
    fn test_perf_budget_lod_values() {
        let mut budget = PerfBudget::new();

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
    fn test_perf_budget_simplify_overlays() {
        let mut budget = PerfBudget::new();

        budget.degrade_level = DegradeLevel::Full;
        assert!(!budget.should_simplify_overlays());

        budget.degrade_level = DegradeLevel::Medium;
        assert!(budget.should_simplify_overlays());

        budget.degrade_level = DegradeLevel::Low;
        assert!(budget.should_simplify_overlays());
    }

    #[test]
    fn test_perf_budget_disable_expensive_overlays() {
        let mut budget = PerfBudget::new();

        budget.degrade_level = DegradeLevel::Full;
        assert!(!budget.should_disable_expensive_overlays());

        budget.degrade_level = DegradeLevel::Medium;
        assert!(!budget.should_disable_expensive_overlays());

        budget.degrade_level = DegradeLevel::Low;
        assert!(budget.should_disable_expensive_overlays());
    }

    #[test]
    fn test_perf_budget_reset() {
        let mut budget = PerfBudget::new();
        budget.degrade_level = DegradeLevel::Minimal;
        budget.record_frame(20.0);
        budget.record_frame(20.0);

        budget.reset();

        assert_eq!(budget.degrade_level, DegradeLevel::Full);
    }

    #[test]
    fn test_new_perf_metrics() {
        assert_eq!(PerfMetric::HitTest.display_name(), "Hit Test");
        assert_eq!(PerfMetric::HitTest.metric_key(), "hit_test_ms");

        assert_eq!(PerfMetric::TooltipBuild.display_name(), "Tooltip Build");
        assert_eq!(PerfMetric::TooltipBuild.metric_key(), "tooltip_build_ms");

        assert_eq!(PerfMetric::SelectionPropagation.display_name(), "Selection Propagation");
        assert_eq!(PerfMetric::SelectionPropagation.metric_key(), "selection_propagation_ms");

        assert_eq!(PerfMetric::UiFrame.display_name(), "UI Frame");
        assert_eq!(PerfMetric::UiFrame.metric_key(), "ui_frame_ms");
    }
}

// Additional comprehensive tests
include!("performance_test.rs");
