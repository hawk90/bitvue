//! Index Session Out-of-Core Windowing - T1-1 Session Out-of-Core
//!
//! Deliverable: out_of_core_01:Indexing:Session:AV1:out_of_core
//!
//! Enables handling large indices (100k-1M frames) by only materializing the needed window.
//! Prevents memory exhaustion when working with very long video files.
//!
//! Per OUT_OF_CORE_SPEC:
//! - Only materialize frames in current viewing window
//! - Sparse index for keyframes enables fast seeking
//! - LRU cache eviction of old frames

use crate::indexing::{FrameMetadata, SeekPoint};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};

/// Window size policy for frame metadata materialization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IndexWindowPolicy {
    /// Fixed window size (e.g., 5000 frames)
    Fixed(usize),
    /// Adaptive based on playback position (more frames around current position)
    Adaptive { min: usize, max: usize },
    /// Full materialization (for small indices)
    Full,
}

impl IndexWindowPolicy {
    /// Calculate window size for given context
    ///
    /// For indexing, window size is typically larger than timeline
    /// since we don't need visual rendering overhead.
    pub fn calculate_window_size(&self, total_frames: usize) -> usize {
        match self {
            IndexWindowPolicy::Fixed(size) => (*size).min(total_frames),
            IndexWindowPolicy::Adaptive { min: _, max } => {
                // Use max for now, could be adaptive based on memory pressure
                (*max).min(total_frames)
            }
            IndexWindowPolicy::Full => total_frames,
        }
    }

    /// Check if index should use out-of-core (based on size threshold)
    pub fn should_use_out_of_core(total_frames: usize) -> bool {
        // Use out-of-core for indices > 10k frames
        total_frames > 10_000
    }
}

impl Default for IndexWindowPolicy {
    fn default() -> Self {
        // Default: adaptive window 1k-10k frames
        IndexWindowPolicy::Adaptive {
            min: 1_000,
            max: 10_000,
        }
    }
}

/// Index session window state
///
/// Manages the visible window of frame metadata for out-of-core operation.
#[derive(Debug, Clone)]
pub struct IndexSessionWindow {
    /// Session identifier
    pub session_id: String,

    /// Total frame count in index
    pub total_frames: usize,

    /// Window policy
    pub policy: IndexWindowPolicy,

    /// Current playback/viewing position
    pub current_position: usize,

    /// Visible window start (display_idx)
    pub window_start: usize,

    /// Visible window size (frame count)
    pub window_size: usize,

    /// Materialized frame metadata in current window
    /// Key: display_idx, Value: FrameMetadata
    pub materialized: HashMap<usize, FrameMetadata>,

    /// Sparse index of keyframes for fast seeking
    /// Subset of all keyframes, evenly distributed
    pub sparse_keyframes: Vec<SeekPoint>,

    /// LRU cache for recently accessed frames
    /// Tracks access order for eviction
    lru_queue: VecDeque<usize>,

    /// Window revision counter (increments on window move)
    window_revision: u64,

    /// Statistics
    stats: WindowStats,
}

/// Window statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WindowStats {
    /// Total window moves
    pub window_moves: usize,
    /// Total frames materialized
    pub frames_materialized: usize,
    /// Total frames evicted
    pub frames_evicted: usize,
    /// Cache hits (frame already materialized)
    pub cache_hits: usize,
    /// Cache misses (frame needed materialization)
    pub cache_misses: usize,
}

impl IndexSessionWindow {
    /// Create a new index session window
    pub fn new(
        session_id: String,
        total_frames: usize,
        policy: IndexWindowPolicy,
        sparse_keyframes: Vec<SeekPoint>,
    ) -> Self {
        let window_size = policy.calculate_window_size(total_frames);

        Self {
            session_id,
            total_frames,
            policy,
            current_position: 0,
            window_start: 0,
            window_size,
            materialized: HashMap::new(),
            sparse_keyframes,
            lru_queue: VecDeque::new(),
            window_revision: 0,
            stats: WindowStats::default(),
        }
    }

    /// Set current playback/viewing position
    ///
    /// Automatically adjusts window if position moves outside current window.
    pub fn set_position(&mut self, display_idx: usize) {
        if display_idx >= self.total_frames {
            return;
        }

        self.current_position = display_idx;

        // Check if we need to move the window
        let window_end = self.window_start + self.window_size;

        if display_idx < self.window_start || display_idx >= window_end {
            // Move window to center on new position
            self.move_window_to(display_idx);
        }
    }

    /// Move window to center on given position
    fn move_window_to(&mut self, center_idx: usize) {
        // Calculate new window start (center position in middle of window)
        let half_window = self.window_size / 2;
        let new_start = center_idx.saturating_sub(half_window);

        // Clamp to valid range
        let max_start = self.total_frames.saturating_sub(self.window_size);
        let new_start = new_start.min(max_start);

        if new_start != self.window_start {
            self.window_start = new_start;
            self.window_revision += 1;
            self.stats.window_moves += 1;

            // Evict frames outside new window
            self.evict_outside_window();
        }
    }

    /// Evict frames outside current window
    fn evict_outside_window(&mut self) {
        let window_end = self.window_start + self.window_size;

        let to_evict: Vec<usize> = self
            .materialized
            .keys()
            .filter(|&&idx| idx < self.window_start || idx >= window_end)
            .copied()
            .collect();

        for idx in to_evict {
            self.materialized.remove(&idx);
            self.lru_queue.retain(|&x| x != idx);
            self.stats.frames_evicted += 1;
        }
    }

    /// Materialize a frame (add to window)
    ///
    /// Returns true if frame was newly materialized, false if already present.
    pub fn materialize_frame(&mut self, frame: FrameMetadata) -> bool {
        let display_idx = frame.display_idx;

        use std::collections::hash_map::Entry;
        match self.materialized.entry(display_idx) {
            Entry::Occupied(_) => {
                // Update LRU (move to back)
                self.lru_queue.retain(|&x| x != display_idx);
                self.lru_queue.push_back(display_idx);
                self.stats.cache_hits += 1;
                false
            }
            Entry::Vacant(entry) => {
                // Add new frame
                entry.insert(frame);
                self.lru_queue.push_back(display_idx);
                self.stats.frames_materialized += 1;
                self.stats.cache_misses += 1;

                // Evict LRU if over capacity
                if self.materialized.len() > self.window_size {
                    self.evict_lru();
                }

                true
            }
        }
    }

    /// Evict least recently used frame
    fn evict_lru(&mut self) {
        if let Some(lru_idx) = self.lru_queue.pop_front() {
            self.materialized.remove(&lru_idx);
            self.stats.frames_evicted += 1;
        }
    }

    /// Get frame metadata if materialized
    pub fn get_frame(&mut self, display_idx: usize) -> Option<&FrameMetadata> {
        if self.materialized.contains_key(&display_idx) {
            // Update LRU
            self.lru_queue.retain(|&x| x != display_idx);
            self.lru_queue.push_back(display_idx);
            self.stats.cache_hits += 1;
            self.materialized.get(&display_idx)
        } else {
            self.stats.cache_misses += 1;
            None
        }
    }

    /// Get mutable frame metadata if materialized
    pub fn get_frame_mut(&mut self, display_idx: usize) -> Option<&mut FrameMetadata> {
        if self.materialized.contains_key(&display_idx) {
            // Update LRU
            self.lru_queue.retain(|&x| x != display_idx);
            self.lru_queue.push_back(display_idx);
            self.stats.cache_hits += 1;
            self.materialized.get_mut(&display_idx)
        } else {
            self.stats.cache_misses += 1;
            None
        }
    }

    /// Check if frame is materialized
    pub fn is_materialized(&self, display_idx: usize) -> bool {
        self.materialized.contains_key(&display_idx)
    }

    /// Get nearest keyframe to given position (from sparse index)
    pub fn find_nearest_keyframe(&self, display_idx: usize) -> Option<&SeekPoint> {
        self.sparse_keyframes
            .iter()
            .rev()
            .find(|sp| sp.display_idx <= display_idx)
    }

    /// Get current window range
    pub fn window_range(&self) -> (usize, usize) {
        (self.window_start, self.window_start + self.window_size)
    }

    /// Get window revision counter
    pub fn window_revision(&self) -> u64 {
        self.window_revision
    }

    /// Get window statistics
    pub fn stats(&self) -> &WindowStats {
        &self.stats
    }

    /// Get cache hit rate
    pub fn hit_rate(&self) -> f64 {
        let total = self.stats.cache_hits + self.stats.cache_misses;
        if total > 0 {
            self.stats.cache_hits as f64 / total as f64
        } else {
            0.0
        }
    }

    /// Clear all materialized frames
    pub fn clear(&mut self) {
        let count = self.materialized.len();
        self.materialized.clear();
        self.lru_queue.clear();
        self.stats.frames_evicted += count;
    }

    /// Get materialized frame count
    pub fn materialized_count(&self) -> usize {
        self.materialized.len()
    }

    /// Check if window should be moved for given position
    pub fn should_move_window(&self, display_idx: usize) -> bool {
        let window_end = self.window_start + self.window_size;
        display_idx < self.window_start || display_idx >= window_end
    }
}

#[cfg(test)]
mod tests {
    include!("index_session_window_test.rs");
}
