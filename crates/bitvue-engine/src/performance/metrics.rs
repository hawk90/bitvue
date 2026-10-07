//! Measurement and reporting: metric kinds, events, timers, cache stats, the `PerfTracker`
//! that aggregates them, and the exportable `PerfReport`.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};

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
    /// Hit testing
    HitTest,
    /// Tooltip building
    TooltipBuild,
    /// Selection propagation
    SelectionPropagation,
    /// Total UI frame time
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

#[allow(
    unused_imports,
    unused_variables,
    unused_mut,
    dead_code,
    unused_comparisons,
    unused_must_use,
    hidden_glob_reexports,
    unreachable_code,
    non_camel_case_types,
    unused_parens,
    unused_assignments
)]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_perf_metrics() {
        assert_eq!(PerfMetric::HitTest.display_name(), "Hit Test");
        assert_eq!(PerfMetric::HitTest.metric_key(), "hit_test_ms");

        assert_eq!(PerfMetric::TooltipBuild.display_name(), "Tooltip Build");
        assert_eq!(PerfMetric::TooltipBuild.metric_key(), "tooltip_build_ms");

        assert_eq!(
            PerfMetric::SelectionPropagation.display_name(),
            "Selection Propagation"
        );
        assert_eq!(
            PerfMetric::SelectionPropagation.metric_key(),
            "selection_propagation_ms"
        );

        assert_eq!(PerfMetric::UiFrame.display_name(), "UI Frame");
        assert_eq!(PerfMetric::UiFrame.metric_key(), "ui_frame_ms");
    }
}
