//! Timeline Diagnostics Bands - T4-3
//!
//! Per WS_TIMELINE_TEMPORAL and EDGE_CASES_AND_DEGRADE_BEHAVIOR:
//! - Scene change detection band
//! - Reorder depth band (PTS ≠ DTS)
//! - Error burst detection and auto-selection

use serde::{Deserialize, Serialize};

/// Diagnostic band type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DiagnosticBandType {
    /// Scene change detection
    SceneChange,
    /// Reorder mismatch (PTS ≠ DTS)
    ReorderMismatch,
    /// Error bursts
    ErrorBurst,
}

impl DiagnosticBandType {
    /// Get band display name
    pub fn name(&self) -> &'static str {
        match self {
            DiagnosticBandType::SceneChange => "Scene Changes",
            DiagnosticBandType::ReorderMismatch => "Reorder Mismatch",
            DiagnosticBandType::ErrorBurst => "Error Bursts",
        }
    }

    /// Get band color hint
    pub fn color_hint(&self) -> &'static str {
        match self {
            DiagnosticBandType::SceneChange => "green",
            DiagnosticBandType::ReorderMismatch => "orange",
            DiagnosticBandType::ErrorBurst => "red",
        }
    }
}

/// Scene change entry
///
/// Per WS_TIMELINE_TEMPORAL: "Scene change markers with confidence tooltip"
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneChange {
    /// Frame display_idx where scene change occurs
    pub display_idx: usize,
    /// Confidence score (0.0 to 1.0)
    pub confidence: f32,
    /// Optional description
    pub description: Option<String>,
}

impl SceneChange {
    pub fn new(display_idx: usize, confidence: f32) -> Self {
        Self {
            display_idx,
            confidence,
            description: None,
        }
    }

    pub fn with_description(mut self, desc: String) -> Self {
        self.description = Some(desc);
        self
    }
}

/// Reorder mismatch entry
///
/// Per WS_TIMELINE_TEMPORAL: "Reorder mismatch band (PTS≠DTS shading)"
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReorderEntry {
    /// Frame display_idx
    pub display_idx: usize,
    /// PTS value
    pub pts: u64,
    /// DTS value
    pub dts: u64,
    /// Reorder depth (|PTS - DTS|)
    pub depth: u64,
}

impl ReorderEntry {
    pub fn new(display_idx: usize, pts: u64, dts: u64) -> Self {
        let depth = pts.abs_diff(dts);
        Self {
            display_idx,
            pts,
            dts,
            depth,
        }
    }

    /// Get depth as frame count (assuming constant frame rate)
    pub fn depth_frames(&self, frame_duration_ms: u64) -> usize {
        if frame_duration_ms == 0 {
            0
        } else {
            (self.depth / frame_duration_ms) as usize
        }
    }
}

/// Error burst entry
///
/// Per EDGE_CASES_AND_DEGRADE_BEHAVIOR: "Error burst auto-select"
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorBurst {
    /// Burst start frame (display_idx)
    pub start_idx: usize,
    /// Burst end frame (display_idx, inclusive)
    pub end_idx: usize,
    /// Number of errors in burst
    pub error_count: usize,
    /// Burst severity (higher = more critical)
    pub severity: f32,
    /// Optional error types
    pub error_types: Vec<String>,
}

impl ErrorBurst {
    pub fn new(start_idx: usize, end_idx: usize, error_count: usize) -> Self {
        // Severity = error_count / burst_length
        let length = (end_idx - start_idx + 1) as f32;
        let severity = error_count as f32 / length;

        Self {
            start_idx,
            end_idx,
            error_count,
            severity,
            error_types: Vec::new(),
        }
    }

    pub fn with_error_types(mut self, types: Vec<String>) -> Self {
        self.error_types = types;
        self
    }

    /// Get burst length in frames
    pub fn length(&self) -> usize {
        self.end_idx - self.start_idx + 1
    }

    /// Get error density (errors per frame)
    pub fn density(&self) -> f32 {
        self.error_count as f32 / self.length() as f32
    }
}

/// Error burst detection algorithm
pub struct ErrorBurstDetection;

impl ErrorBurstDetection {
    /// Detect error bursts from error frame indices
    ///
    /// Per EDGE_CASES_AND_DEGRADE_BEHAVIOR: Group nearby errors into bursts
    pub fn detect_bursts(error_indices: &[usize], burst_gap_threshold: usize) -> Vec<ErrorBurst> {
        if error_indices.is_empty() {
            return Vec::new();
        }

        let mut bursts = Vec::new();
        let mut current_start = error_indices[0];
        let mut current_end = error_indices[0];
        let mut current_count = 1;

        for &idx in &error_indices[1..] {
            if idx - current_end <= burst_gap_threshold {
                // Extend current burst
                current_end = idx;
                current_count += 1;
            } else {
                // Finish current burst, start new one
                bursts.push(ErrorBurst::new(current_start, current_end, current_count));
                current_start = idx;
                current_end = idx;
                current_count = 1;
            }
        }

        // Add final burst
        bursts.push(ErrorBurst::new(current_start, current_end, current_count));

        bursts
    }

    /// Get top N most severe bursts
    ///
    /// Per T4-3 deliverable: "Error burst auto-select" - prioritize worst bursts
    pub fn top_bursts(bursts: &[ErrorBurst], n: usize) -> Vec<usize> {
        let mut indexed: Vec<(usize, f32)> = bursts
            .iter()
            .enumerate()
            .map(|(i, b)| (i, b.severity))
            .collect();

        // Sort by severity descending
        indexed.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        indexed.into_iter().take(n).map(|(i, _)| i).collect()
    }
}

/// Diagnostics Bands system
///
/// Per T4-3 deliverable: DiagnosticsBands with error burst auto-select
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticsBands {
    /// Scene change entries
    pub scene_changes: Vec<SceneChange>,

    /// Reorder mismatch entries
    pub reorder_entries: Vec<ReorderEntry>,

    /// Error bursts
    pub error_bursts: Vec<ErrorBurst>,

    /// Selected burst index (for auto-select)
    pub selected_burst: Option<usize>,

    /// Band visibility toggles
    pub scene_change_visible: bool,
    pub reorder_visible: bool,
    pub error_burst_visible: bool,
}

impl DiagnosticsBands {
    /// Create a new diagnostics bands system
    pub fn new() -> Self {
        Self {
            scene_changes: Vec::new(),
            reorder_entries: Vec::new(),
            error_bursts: Vec::new(),
            selected_burst: None,
            scene_change_visible: true,
            reorder_visible: true,
            error_burst_visible: true,
        }
    }

    /// Add scene change entry
    pub fn add_scene_change(&mut self, scene_change: SceneChange) {
        self.scene_changes.push(scene_change);
    }

    /// Add reorder entry
    pub fn add_reorder_entry(&mut self, entry: ReorderEntry) {
        self.reorder_entries.push(entry);
    }

    /// Detect and set error bursts from error indices
    ///
    /// Per T4-3 deliverable: Error burst detection
    pub fn detect_error_bursts(&mut self, error_indices: &[usize], gap_threshold: usize) {
        self.error_bursts = ErrorBurstDetection::detect_bursts(error_indices, gap_threshold);
    }

    /// Auto-select most severe error burst
    ///
    /// Per T4-3 deliverable: "Error burst auto-select"
    pub fn auto_select_worst_burst(&mut self) {
        if self.error_bursts.is_empty() {
            self.selected_burst = None;
            return;
        }

        let top = ErrorBurstDetection::top_bursts(&self.error_bursts, 1);
        self.selected_burst = top.first().copied();
    }

    /// Get selected burst
    pub fn get_selected_burst(&self) -> Option<&ErrorBurst> {
        self.selected_burst
            .and_then(|idx| self.error_bursts.get(idx))
    }

    /// Select burst by index
    pub fn select_burst(&mut self, idx: usize) {
        if idx < self.error_bursts.len() {
            self.selected_burst = Some(idx);
        }
    }

    /// Clear burst selection
    pub fn clear_burst_selection(&mut self) {
        self.selected_burst = None;
    }

    /// Toggle band visibility
    pub fn toggle_band(&mut self, band_type: DiagnosticBandType) {
        match band_type {
            DiagnosticBandType::SceneChange => {
                self.scene_change_visible = !self.scene_change_visible
            }
            DiagnosticBandType::ReorderMismatch => self.reorder_visible = !self.reorder_visible,
            DiagnosticBandType::ErrorBurst => self.error_burst_visible = !self.error_burst_visible,
        }
    }

    /// Get count of reorder frames
    pub fn reorder_count(&self) -> usize {
        self.reorder_entries.len()
    }

    /// Get max reorder depth
    pub fn max_reorder_depth(&self) -> u64 {
        self.reorder_entries
            .iter()
            .map(|e| e.depth)
            .max()
            .unwrap_or(0)
    }

    /// Get total error count across all bursts
    pub fn total_error_count(&self) -> usize {
        self.error_bursts.iter().map(|b| b.error_count).sum()
    }
}

impl Default for DiagnosticsBands {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
include!("diagnostics_bands_test.rs");
