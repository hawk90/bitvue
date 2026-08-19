//! Index Development HUD - Out-of-Core Windowing - T1-1 DevHUD
//!
//! Deliverable: out_of_core_01:Indexing:DevHUD:AV1:out_of_core
//!
//! Timeline windowing diagnostics for DevHUD.
//! Shows materialization patterns, window adjustments, and UI freeze prevention metrics.

use crate::IndexSessionWindow;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// Timeline window visualization for DevHUD
///
/// Provides detailed diagnostics for out-of-core windowing during timeline scrubbing.
#[derive(Debug, Clone)]
pub struct TimelineWindowHUD {
    /// Session ID being monitored
    session_id: String,

    /// Current window visualization
    window_viz: WindowVisualization,

    /// Materialization tracking
    materialization_tracker: MaterializationTracker,

    /// Window performance metrics
    performance: WindowPerformanceMetrics,

    /// HUD update counter
    update_count: u64,
}

/// Window visualization state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowVisualization {
    /// Total frames in index
    pub total_frames: usize,

    /// Window start position
    pub window_start: usize,

    /// Window end position (exclusive)
    pub window_end: usize,

    /// Current playback position
    pub current_position: usize,

    /// Materialized frame indices (within window)
    pub materialized_indices: Vec<usize>,

    /// Sparse keyframe positions (outside window)
    pub sparse_keyframes: Vec<usize>,

    /// Window coverage percentage
    pub coverage_percent: f64,
}

/// Materialization pattern tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterializationTracker {
    /// Total materialization requests
    pub total_requests: u64,

    /// Cache hits
    pub cache_hits: u64,

    /// Cache misses (materialized on-demand)
    pub cache_misses: u64,

    /// Frames evicted from cache
    pub frames_evicted: usize,

    /// Window move count
    pub window_moves: usize,

    /// Recent access pattern (last 100 accesses)
    pub recent_pattern: VecDeque<AccessEvent>,

    /// Materialization density histogram (frames per bucket)
    pub density_histogram: Vec<usize>,
}

/// Frame access event
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AccessEvent {
    /// Cache hit
    Hit,
    /// Cache miss (materialized)
    Miss,
    /// Window moved
    WindowMove,
    /// Frame evicted
    Eviction,
}

/// Window performance metrics for UI freeze prevention
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowPerformanceMetrics {
    /// Average frame access time (microseconds)
    pub avg_access_time_us: f64,

    /// Max frame access time (microseconds) - spike detection
    pub max_access_time_us: u64,

    /// P95 access time (microseconds)
    pub p95_access_time_us: u64,

    /// P99 access time (microseconds)
    pub p99_access_time_us: u64,

    /// Blocking operations detected (> 16ms threshold for 60fps)
    pub blocking_operations: u64,

    /// Total operations measured
    pub total_operations: u64,

    /// Window adjustment latency (microseconds)
    pub window_adjust_latency_us: Vec<u64>,
}

impl TimelineWindowHUD {
    /// Create a new timeline window HUD
    pub fn new(session_id: String) -> Self {
        Self {
            session_id,
            window_viz: WindowVisualization {
                total_frames: 0,
                window_start: 0,
                window_end: 0,
                current_position: 0,
                materialized_indices: Vec::new(),
                sparse_keyframes: Vec::new(),
                coverage_percent: 0.0,
            },
            materialization_tracker: MaterializationTracker {
                total_requests: 0,
                cache_hits: 0,
                cache_misses: 0,
                frames_evicted: 0,
                window_moves: 0,
                recent_pattern: VecDeque::new(),
                density_histogram: Vec::new(),
            },
            performance: WindowPerformanceMetrics {
                avg_access_time_us: 0.0,
                max_access_time_us: 0,
                p95_access_time_us: 0,
                p99_access_time_us: 0,
                blocking_operations: 0,
                total_operations: 0,
                window_adjust_latency_us: Vec::new(),
            },
            update_count: 0,
        }
    }

    /// Update HUD from window state
    pub fn update_from_window(&mut self, window: &IndexSessionWindow) {
        self.update_visualization(window);
        self.update_materialization_tracker(window);
        self.update_count += 1;
    }

    /// Record a frame access event with timing
    pub fn record_access(&mut self, _frame_idx: usize, was_hit: bool, access_time_us: u64) {
        self.materialization_tracker.total_requests += 1;

        if was_hit {
            self.materialization_tracker.cache_hits += 1;
            self.materialization_tracker
                .recent_pattern
                .push_back(AccessEvent::Hit);
        } else {
            self.materialization_tracker.cache_misses += 1;
            self.materialization_tracker
                .recent_pattern
                .push_back(AccessEvent::Miss);
        }

        // Keep recent pattern bounded to 100 events
        if self.materialization_tracker.recent_pattern.len() > 100 {
            self.materialization_tracker.recent_pattern.pop_front();
        }

        // Update performance metrics
        self.update_access_timing(access_time_us);
    }

    /// Record a window move event
    pub fn record_window_move(&mut self, adjust_latency_us: u64) {
        self.materialization_tracker.window_moves += 1;
        self.materialization_tracker
            .recent_pattern
            .push_back(AccessEvent::WindowMove);

        if self.materialization_tracker.recent_pattern.len() > 100 {
            self.materialization_tracker.recent_pattern.pop_front();
        }

        self.performance
            .window_adjust_latency_us
            .push(adjust_latency_us);

        // Keep latency history bounded to 1000 samples
        if self.performance.window_adjust_latency_us.len() > 1000 {
            self.performance.window_adjust_latency_us.remove(0);
        }
    }

    /// Record a frame eviction event
    pub fn record_eviction(&mut self, _frame_idx: usize) {
        // Note: frame_idx is tracked for potential future use
        self.materialization_tracker.frames_evicted += 1;
        self.materialization_tracker
            .recent_pattern
            .push_back(AccessEvent::Eviction);

        if self.materialization_tracker.recent_pattern.len() > 100 {
            self.materialization_tracker.recent_pattern.pop_front();
        }
    }

    /// Update window visualization
    fn update_visualization(&mut self, window: &IndexSessionWindow) {
        self.window_viz.total_frames = window.total_frames;
        self.window_viz.window_start = window.window_start;
        self.window_viz.window_end = window.window_start + window.window_size;
        self.window_viz.current_position = window.current_position;

        // Extract materialized indices from the HashMap
        self.window_viz.materialized_indices = window.materialized.keys().copied().collect();

        // Extract sparse keyframes
        self.window_viz.sparse_keyframes = window
            .sparse_keyframes
            .iter()
            .map(|sp| sp.display_idx)
            .collect();

        // Calculate coverage
        if window.window_size > 0 {
            self.window_viz.coverage_percent =
                self.window_viz.materialized_indices.len() as f64 / window.window_size as f64;
        } else {
            self.window_viz.coverage_percent = 0.0;
        }
    }

    /// Update materialization tracker from window stats
    fn update_materialization_tracker(&mut self, window: &IndexSessionWindow) {
        let stats = window.stats();

        // Update counters from window stats (maintain current values + deltas)
        // Note: We track cumulative counts separately, so just use the latest stats values
        self.materialization_tracker.frames_evicted = stats.frames_evicted;
        self.materialization_tracker.window_moves = stats.window_moves;

        // Build density histogram (10 buckets)
        self.materialization_tracker.density_histogram = self.build_density_histogram();
    }

    /// Build materialization density histogram
    fn build_density_histogram(&self) -> Vec<usize> {
        let bucket_count = 10;
        let mut histogram = vec![0; bucket_count];

        if self.window_viz.total_frames == 0 {
            return histogram;
        }

        let bucket_size = self.window_viz.total_frames.div_ceil(bucket_count);

        for &idx in &self.window_viz.materialized_indices {
            let bucket = (idx / bucket_size).min(bucket_count - 1);
            histogram[bucket] += 1;
        }

        histogram
    }

    /// Update access timing metrics
    fn update_access_timing(&mut self, access_time_us: u64) {
        self.performance.total_operations += 1;

        // Update max
        if access_time_us > self.performance.max_access_time_us {
            self.performance.max_access_time_us = access_time_us;
        }

        // Update average (rolling)
        let n = self.performance.total_operations as f64;
        self.performance.avg_access_time_us =
            (self.performance.avg_access_time_us * (n - 1.0) + access_time_us as f64) / n;

        // Detect blocking operations (> 16ms for 60fps)
        if access_time_us > 16_000 {
            self.performance.blocking_operations += 1;
        }

        // Update percentiles (simplified - would use proper histogram in production)
        // For now, use approximation based on max and avg
        self.performance.p95_access_time_us = (self.performance.avg_access_time_us
            + self.performance.max_access_time_us as f64 * 4.0)
            as u64
            / 5;
        self.performance.p99_access_time_us = (self.performance.avg_access_time_us
            + self.performance.max_access_time_us as f64 * 9.0)
            as u64
            / 10;
    }

    /// Format window visualization as ASCII art
    pub fn format_window_viz(&self, width: usize) -> String {
        if self.window_viz.total_frames == 0 {
            return String::from("[No frames]");
        }

        let mut output = String::new();

        // Timeline bar
        let chars_per_frame = self.window_viz.total_frames as f64 / width as f64;
        let mut bar = vec![' '; width];

        // Mark window region
        let window_start_char = (self.window_viz.window_start as f64 / chars_per_frame) as usize;
        let window_end_char = (self.window_viz.window_end as f64 / chars_per_frame) as usize;
        bar[window_start_char.min(width)..window_end_char.min(width)].fill('░');

        // Mark materialized frames
        for &idx in &self.window_viz.materialized_indices {
            let char_pos = (idx as f64 / chars_per_frame) as usize;
            if char_pos < width {
                bar[char_pos] = '█';
            }
        }

        // Mark current position
        let pos_char = (self.window_viz.current_position as f64 / chars_per_frame) as usize;
        if pos_char < width {
            bar[pos_char] = '▲';
        }

        // Mark sparse keyframes
        for &idx in &self.window_viz.sparse_keyframes {
            let char_pos = (idx as f64 / chars_per_frame) as usize;
            if char_pos < width && bar[char_pos] == ' ' {
                bar[char_pos] = '·';
            }
        }

        output.push('[');
        output.push_str(&bar.iter().collect::<String>());
        output.push(']');
        output.push('\n');

        // Legend
        output.push_str("  ░=window █=materialized ▲=position ·=keyframe\n");

        output
    }

    /// Format recent access pattern
    pub fn format_access_pattern(&self) -> String {
        let mut output = String::new();

        output.push_str("Recent: [");
        for (i, event) in self
            .materialization_tracker
            .recent_pattern
            .iter()
            .enumerate()
        {
            if i > 0 && i % 20 == 0 {
                output.push(' ');
            }
            match event {
                AccessEvent::Hit => output.push('H'),
                AccessEvent::Miss => output.push('M'),
                AccessEvent::WindowMove => output.push('W'),
                AccessEvent::Eviction => output.push('E'),
            }
        }
        output.push_str("]\n");
        output.push_str("  H=hit M=miss W=window-move E=eviction\n");

        output
    }

    /// Format density histogram
    pub fn format_density_histogram(&self) -> String {
        let mut output = String::new();

        output.push_str("Density: ");
        for &count in &self.materialization_tracker.density_histogram {
            let height = if count > 0 {
                ((count as f64).log2().ceil() as usize).min(5)
            } else {
                0
            };
            match height {
                0 => output.push('_'),
                1 => output.push('▁'),
                2 => output.push('▃'),
                3 => output.push('▅'),
                4 => output.push('▇'),
                _ => output.push('█'),
            }
        }
        output.push('\n');

        output
    }

    /// Format complete HUD as human-readable text
    pub fn format_text(&self) -> String {
        let mut output = String::new();

        output.push_str(&format!(
            "=== Timeline Window DevHUD: {} ===\n",
            self.session_id
        ));
        output.push_str(&format!("Updates: {}\n\n", self.update_count));

        // Window state
        output.push_str("--- Window State ---\n");
        output.push_str(&format!("Total Frames: {}\n", self.window_viz.total_frames));
        output.push_str(&format!(
            "Window: {}-{} (size={})\n",
            self.window_viz.window_start,
            self.window_viz.window_end,
            self.window_viz.window_end - self.window_viz.window_start
        ));
        output.push_str(&format!("Position: {}\n", self.window_viz.current_position));
        output.push_str(&format!(
            "Materialized: {} ({:.1}% coverage)\n",
            self.window_viz.materialized_indices.len(),
            self.window_viz.coverage_percent * 100.0
        ));
        output.push_str(&format!(
            "Sparse Keyframes: {}\n",
            self.window_viz.sparse_keyframes.len()
        ));
        output.push('\n');

        // Visualization
        output.push_str("--- Timeline Visualization ---\n");
        output.push_str(&self.format_window_viz(60));
        output.push('\n');

        // Materialization stats
        output.push_str("--- Materialization ---\n");
        output.push_str(&format!(
            "Total Requests: {}\n",
            self.materialization_tracker.total_requests
        ));
        output.push_str(&format!(
            "Cache Hits: {} ({:.1}%)\n",
            self.materialization_tracker.cache_hits,
            if self.materialization_tracker.total_requests > 0 {
                self.materialization_tracker.cache_hits as f64
                    / self.materialization_tracker.total_requests as f64
                    * 100.0
            } else {
                0.0
            }
        ));
        output.push_str(&format!(
            "Cache Misses: {}\n",
            self.materialization_tracker.cache_misses
        ));
        output.push_str(&format!(
            "Evictions: {}\n",
            self.materialization_tracker.frames_evicted
        ));
        output.push_str(&format!(
            "Window Moves: {}\n",
            self.materialization_tracker.window_moves
        ));
        output.push('\n');

        // Access pattern
        if !self.materialization_tracker.recent_pattern.is_empty() {
            output.push_str("--- Access Pattern ---\n");
            output.push_str(&self.format_access_pattern());
            output.push('\n');
        }

        // Density histogram
        if !self.materialization_tracker.density_histogram.is_empty() {
            output.push_str("--- Materialization Density ---\n");
            output.push_str(&self.format_density_histogram());
            output.push('\n');
        }

        // Performance metrics
        output.push_str("--- Performance (UI Freeze Prevention) ---\n");
        output.push_str(&format!(
            "Avg Access Time: {:.1} μs\n",
            self.performance.avg_access_time_us
        ));
        output.push_str(&format!(
            "Max Access Time: {} μs\n",
            self.performance.max_access_time_us
        ));
        output.push_str(&format!(
            "P95 Access Time: {} μs\n",
            self.performance.p95_access_time_us
        ));
        output.push_str(&format!(
            "P99 Access Time: {} μs\n",
            self.performance.p99_access_time_us
        ));
        output.push_str(&format!(
            "Blocking Ops (>16ms): {} / {} ({:.1}%)\n",
            self.performance.blocking_operations,
            self.performance.total_operations,
            if self.performance.total_operations > 0 {
                self.performance.blocking_operations as f64
                    / self.performance.total_operations as f64
                    * 100.0
            } else {
                0.0
            }
        ));

        if !self.performance.window_adjust_latency_us.is_empty() {
            let avg_adjust = self
                .performance
                .window_adjust_latency_us
                .iter()
                .sum::<u64>() as f64
                / self.performance.window_adjust_latency_us.len() as f64;
            output.push_str(&format!("Avg Window Adjust: {:.1} μs\n", avg_adjust));
        }

        output
    }

    /// Get session ID
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Get window visualization
    pub fn window_viz(&self) -> &WindowVisualization {
        &self.window_viz
    }

    /// Get materialization tracker
    pub fn materialization_tracker(&self) -> &MaterializationTracker {
        &self.materialization_tracker
    }

    /// Get performance metrics
    pub fn performance(&self) -> &WindowPerformanceMetrics {
        &self.performance
    }

    /// Get update count
    pub fn update_count(&self) -> u64 {
        self.update_count
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
    include!("index_dev_hud_window_test.rs");
}
