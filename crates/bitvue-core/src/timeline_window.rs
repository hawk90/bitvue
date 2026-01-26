//! Timeline Out-of-Core Windowing - T0-2 Out-of-Core Implementation
//!
//! Deliverable: out_of_core_01:Foundations:Timeline:AV1:out_of_core
//!
//! Enables handling long streams (100k-1M frames) by only materializing the visible window.
//! Preserves display/decode separation and avoids UI freeze through async loading.
//!
//! Per COORDINATE_SYSTEM_CONTRACT: Timeline uses display_idx as canonical horizontal axis.
//! Per CACHE_INVALIDATION_TABLE: Timeline invalidates on data revision, zoom, filter.
//! Per ASYNC_PIPELINE_BACKPRESSURE: Window loading supports cancellation and backpressure.

use crate::timeline::TimelineFrame; // Use explicit import to avoid ambiguity
use crate::{CacheKey, CacheProvenanceTracker, FrameMarker, InvalidationTrigger};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};

/// Window size policy for materialization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WindowPolicy {
    /// Fixed window size (e.g., 1000 frames)
    Fixed(usize),
    /// Adaptive based on zoom level (more frames when zoomed out)
    Adaptive { min: usize, max: usize },
    /// Full materialization (for small streams)
    Full,
}

impl WindowPolicy {
    /// Calculate window size for given zoom level
    ///
    /// zoom_level: pixels per frame (visual size)
    /// - High zoom (>1 px/frame): frames are larger, fewer fit in viewport
    /// - Low zoom (<1 px/frame): frames are smaller, more fit in viewport
    pub fn calculate_window_size(&self, zoom_level: f32, total_frames: usize) -> usize {
        match self {
            WindowPolicy::Fixed(size) => (*size).min(total_frames),
            WindowPolicy::Adaptive { min, max } => {
                // Inverse relationship: higher zoom = fewer frames visible
                // At zoom 1.0: baseline = max
                // At zoom 10.0: 1/10 of baseline (zoomed in, fewer frames)
                // At zoom 0.1: 10x baseline (zoomed out, more frames)
                let zoom_factor = zoom_level.clamp(0.1, 10.0);
                let baseline = *max as f32;
                let calculated = (baseline / zoom_factor) as usize;
                calculated.clamp(*min, *max).min(total_frames)
            }
            WindowPolicy::Full => total_frames,
        }
    }
}

/// Sparse index entry for fast seeking
///
/// Stores only critical frames (keyframes, markers, scene changes)
/// to enable fast navigation without loading all frames.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SparseIndexEntry {
    /// Display index
    pub display_idx: usize,
    /// Frame type
    pub frame_type: String,
    /// Frame marker
    pub marker: FrameMarker,
    /// Byte offset in file (for seeking)
    pub byte_offset: u64,
    /// Frame size in bytes
    pub size_bytes: u64,
}

/// Timeline window state
///
/// Manages the visible window of frames for out-of-core operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineWindow {
    /// Stream identifier
    pub stream_id: String,

    /// Total frame count in stream
    pub total_frames: usize,

    /// Window policy
    pub policy: WindowPolicy,

    /// Current zoom level (pixels per frame)
    pub zoom_level: f32,

    /// Visible window start (display_idx)
    pub window_start: usize,

    /// Visible window size (frame count)
    pub window_size: usize,

    /// Materialized frames in current window
    /// Key: display_idx, Value: TimelineFrame
    pub materialized: HashMap<usize, TimelineFrame>,

    /// Sparse index of critical frames (keyframes, markers)
    pub sparse_index: Vec<SparseIndexEntry>,

    /// Data revision counter (for cache invalidation)
    pub data_revision: u64,

    /// Filter hash (for cache invalidation)
    pub filter_hash: u64,

    /// Cache provenance tracker
    cache_tracker: CacheProvenanceTracker,

    /// Current cache key
    current_cache_key: Option<CacheKey>,

    /// Pending load requests (for cancellation)
    pending_loads: VecDeque<(usize, usize)>, // (start, count)
}

impl TimelineWindow {
    /// Create a new timeline window
    pub fn new(stream_id: String, total_frames: usize, policy: WindowPolicy) -> Self {
        let default_zoom = 1.0;
        let initial_window_size = policy.calculate_window_size(default_zoom, total_frames);

        Self {
            stream_id,
            total_frames,
            policy,
            zoom_level: default_zoom,
            window_start: 0,
            window_size: initial_window_size,
            materialized: HashMap::new(),
            sparse_index: Vec::new(),
            data_revision: 0,
            filter_hash: 0,
            cache_tracker: CacheProvenanceTracker::new(),
            current_cache_key: None,
            pending_loads: VecDeque::new(),
        }
    }

    /// Add entry to sparse index
    pub fn add_sparse_entry(&mut self, entry: SparseIndexEntry) {
        self.sparse_index.push(entry);
        // Keep sorted by display_idx
        self.sparse_index.sort_by_key(|e| e.display_idx);
    }

    /// Build sparse index from keyframes and markers
    pub fn build_sparse_index_from_frames(&mut self, frames: &[TimelineFrame]) {
        self.sparse_index.clear();
        for (idx, frame) in frames.iter().enumerate() {
            // Include keyframes and frames with markers
            if frame.marker.is_critical() || idx == 0 || idx == frames.len() - 1 {
                self.sparse_index.push(SparseIndexEntry {
                    display_idx: frame.display_idx,
                    frame_type: frame.frame_type.clone(),
                    marker: frame.marker,
                    byte_offset: 0, // Populated by indexer
                    size_bytes: frame.size_bytes,
                });
            }
        }
    }

    /// Set zoom level and recalculate window size
    ///
    /// Per CACHE_INVALIDATION_TABLE: Zoom change invalidates timeline cache
    pub fn set_zoom(&mut self, zoom_level: f32) {
        if (self.zoom_level - zoom_level).abs() > 0.001 {
            self.zoom_level = zoom_level;
            let new_window_size = self
                .policy
                .calculate_window_size(zoom_level, self.total_frames);

            // Always invalidate cache on zoom change, even if window size doesn't change
            self.invalidate_cache(InvalidationTrigger::ZoomChanged);

            if new_window_size != self.window_size {
                self.window_size = new_window_size;
            }
        }
    }

    /// Set filter and invalidate cache
    ///
    /// Per CACHE_INVALIDATION_TABLE: Filter change invalidates timeline cache
    pub fn set_filter(&mut self, filter_hash: u64) {
        if self.filter_hash != filter_hash {
            self.filter_hash = filter_hash;
            self.invalidate_cache(InvalidationTrigger::Manual("filter_changed".to_string()));
        }
    }

    /// Scroll to position and materialize window
    ///
    /// Per SELECTION_PRECEDENCE_RULES: Never clear selection due to scrolling
    pub fn scroll_to(&mut self, display_idx: usize) {
        // Clamp to valid range
        let idx = display_idx.min(self.total_frames.saturating_sub(1));

        // Center window on target frame
        let half_window = self.window_size / 2;
        let new_start = idx.saturating_sub(half_window);

        if new_start != self.window_start {
            self.window_start = new_start;
            self.request_window_load(new_start, self.window_size);
        }
    }

    /// Request window load (async placeholder)
    ///
    /// Per ASYNC_PIPELINE_BACKPRESSURE: Support last-wins cancellation
    pub fn request_window_load(&mut self, start: usize, count: usize) {
        // Cancel any pending loads (last-wins)
        self.pending_loads.clear();

        // Queue new load request
        self.pending_loads.push_back((start, count));

        // In a real implementation, this would:
        // 1. Spawn async task to load frame metadata
        // 2. Support cancellation via token
        // 3. Update materialized map on completion
        // 4. Emit completion event
    }

    /// Materialize a frame into the window
    ///
    /// Used by async loader to populate window
    pub fn materialize_frame(&mut self, frame: TimelineFrame) {
        let idx = frame.display_idx;

        // Only materialize if in current window
        if idx >= self.window_start && idx < self.window_start + self.window_size {
            self.materialized.insert(idx, frame);
        }
    }

    /// Dematerialize frames outside window
    ///
    /// Call after scrolling to free memory
    pub fn dematerialize_outside_window(&mut self) {
        let window_end = self.window_start + self.window_size;
        self.materialized
            .retain(|idx, _| *idx >= self.window_start && *idx < window_end);
    }

    /// Get frame by display_idx (only if materialized)
    pub fn get_frame(&self, display_idx: usize) -> Option<&TimelineFrame> {
        self.materialized.get(&display_idx)
    }

    /// Get mutable frame by display_idx
    pub fn get_frame_mut(&mut self, display_idx: usize) -> Option<&mut TimelineFrame> {
        self.materialized.get_mut(&display_idx)
    }

    /// Check if frame is materialized
    pub fn is_materialized(&self, display_idx: usize) -> bool {
        self.materialized.contains_key(&display_idx)
    }

    /// Get materialized frame count
    pub fn materialized_count(&self) -> usize {
        self.materialized.len()
    }

    /// Get materialized frames in window order
    pub fn materialized_frames(&self) -> Vec<&TimelineFrame> {
        let mut frames: Vec<_> = self.materialized.values().collect();
        frames.sort_by_key(|f| f.display_idx);
        frames
    }

    /// Find nearest keyframe using sparse index
    ///
    /// Returns display_idx of nearest keyframe
    pub fn find_nearest_keyframe(&self, from_idx: usize, forward: bool) -> Option<usize> {
        if forward {
            self.sparse_index
                .iter()
                .filter(|e| e.marker == FrameMarker::Key && e.display_idx > from_idx)
                .map(|e| e.display_idx)
                .next()
        } else {
            self.sparse_index
                .iter()
                .rev()
                .filter(|e| e.marker == FrameMarker::Key && e.display_idx < from_idx)
                .map(|e| e.display_idx)
                .next()
        }
    }

    /// Find nearest marker using sparse index
    pub fn find_nearest_marker(&self, from_idx: usize, forward: bool) -> Option<usize> {
        if forward {
            self.sparse_index
                .iter()
                .filter(|e| e.marker != FrameMarker::None && e.display_idx > from_idx)
                .map(|e| e.display_idx)
                .next()
        } else {
            self.sparse_index
                .iter()
                .rev()
                .filter(|e| e.marker != FrameMarker::None && e.display_idx < from_idx)
                .map(|e| e.display_idx)
                .next()
        }
    }

    /// Get sparse index entries in range
    pub fn sparse_entries_in_range(&self, start: usize, end: usize) -> Vec<&SparseIndexEntry> {
        self.sparse_index
            .iter()
            .filter(|e| e.display_idx >= start && e.display_idx < end)
            .collect()
    }

    /// Increment data revision
    ///
    /// Per CACHE_INVALIDATION_TABLE: Data revision change invalidates timeline cache
    pub fn increment_revision(&mut self) {
        self.data_revision += 1;
        self.invalidate_cache(InvalidationTrigger::DataRevision(self.data_revision));
    }

    /// Invalidate cache
    fn invalidate_cache(&mut self, trigger: InvalidationTrigger) {
        self.cache_tracker.invalidate(trigger);
        self.current_cache_key = None;
    }

    /// Create cache key for current window state
    ///
    /// Per CACHE_INVALIDATION_TABLE: Timeline cache depends on data revision, zoom, filter
    pub fn create_cache_key(&mut self, size_bytes: usize) -> CacheKey {
        let key = CacheKey::Timeline {
            data_revision: self.data_revision,
            zoom_level_x100: (self.zoom_level * 100.0) as u32,
            filter_hash: self.filter_hash,
        };

        self.cache_tracker.add_entry(
            key.clone(),
            size_bytes,
            format!("timeline_window_{}_{}", self.window_start, self.window_size),
        );

        self.current_cache_key = Some(key.clone());
        key
    }

    /// Get cache statistics
    pub fn cache_stats(&self) -> (usize, usize, f64) {
        let stats = self.cache_tracker.stats();
        (stats.total_entries, stats.valid_entries, stats.hit_rate)
    }

    /// Clear all materialized frames and cache
    pub fn clear(&mut self) {
        self.materialized.clear();
        self.pending_loads.clear();
        self.cache_tracker.clear();
        self.current_cache_key = None;
    }

    /// Get visible range (for UI rendering)
    pub fn visible_range(&self) -> (usize, usize) {
        (self.window_start, self.window_size)
    }

    /// Check if frame is in visible window
    pub fn is_in_window(&self, display_idx: usize) -> bool {
        display_idx >= self.window_start && display_idx < self.window_start + self.window_size
    }

    /// Get window coverage ratio (materialized / window_size)
    pub fn coverage_ratio(&self) -> f32 {
        if self.window_size == 0 {
            return 0.0;
        }

        let materialized_in_window = self
            .materialized
            .keys()
            .filter(|idx| self.is_in_window(**idx))
            .count();

        materialized_in_window as f32 / self.window_size as f32
    }

    /// Estimate memory usage (bytes)
    pub fn estimated_memory_usage(&self) -> usize {
        // Rough estimate: 200 bytes per TimelineFrame + sparse index overhead
        self.materialized.len() * 200 + self.sparse_index.len() * 100
    }
}

/// Window load status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WindowLoadStatus {
    /// No load in progress
    Idle,
    /// Loading frames
    Loading { current: usize, total: usize },
    /// Load completed
    Completed,
    /// Load failed
    Failed,
}

/// Progressive window loader
///
/// Handles async loading of frame metadata for visible window.
/// Per ASYNC_PIPELINE_BACKPRESSURE: Supports cancellation and last-wins.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowLoader {
    /// Current load status
    pub status: WindowLoadStatus,

    /// Load progress (0.0 to 1.0)
    pub progress: f32,

    /// Frames loaded in current batch
    pub frames_loaded: usize,

    /// Total frames to load
    pub frames_total: usize,

    /// Load generation (increments on cancel, for stale detection)
    pub generation: u64,
}

impl WindowLoader {
    /// Create a new window loader
    pub fn new() -> Self {
        Self {
            status: WindowLoadStatus::Idle,
            progress: 0.0,
            frames_loaded: 0,
            frames_total: 0,
            generation: 0,
        }
    }

    /// Start loading window
    pub fn start_load(&mut self, frame_count: usize) {
        self.status = WindowLoadStatus::Loading {
            current: 0,
            total: frame_count,
        };
        self.progress = 0.0;
        self.frames_loaded = 0;
        self.frames_total = frame_count;
    }

    /// Update load progress
    pub fn update_progress(&mut self, frames_loaded: usize) {
        self.frames_loaded = frames_loaded;
        self.progress = if self.frames_total > 0 {
            frames_loaded as f32 / self.frames_total as f32
        } else {
            0.0
        };

        self.status = WindowLoadStatus::Loading {
            current: frames_loaded,
            total: self.frames_total,
        };
    }

    /// Complete load
    pub fn complete(&mut self) {
        self.status = WindowLoadStatus::Completed;
        self.progress = 1.0;
    }

    /// Fail load
    pub fn fail(&mut self) {
        self.status = WindowLoadStatus::Failed;
    }

    /// Cancel current load (increments generation for stale detection)
    pub fn cancel(&mut self) {
        self.status = WindowLoadStatus::Idle;
        self.progress = 0.0;
        self.generation += 1;
    }

    /// Check if load is in progress
    pub fn is_loading(&self) -> bool {
        matches!(self.status, WindowLoadStatus::Loading { .. })
    }

    /// Get current generation (for stale detection)
    pub fn current_generation(&self) -> u64 {
        self.generation
    }
}

impl Default for WindowLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    include!("timeline_window_test.rs");
}
