//! Two-Phase Index Builder - T1-1
//!
//! Per INDEXING_STRATEGY_SPEC.md:
//! - Phase 1: Quick Index (minimal scan for keyframes/OBU boundaries)
//! - Phase 2: Full Index (background build with progress)
//!
//! Per EDGE_CASES_AND_DEGRADE_BEHAVIOR.md §A:
//! - UI must not wait for full index
//! - Jumps allowed only within indexed range while building
//! - Others show "Index building..." (no queueing)

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

/// Frame seek point (minimal metadata for quick index)
///
/// Per INDEXING_STRATEGY_SPEC.md Phase 1:
/// Quick index stores only essential data for first frame display.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeekPoint {
    /// Frame index in display order
    pub display_idx: usize,

    /// Byte offset in file
    pub byte_offset: u64,

    /// True if this is a keyframe (can be decoded independently)
    pub is_keyframe: bool,

    /// Presentation timestamp (if available)
    pub pts: Option<u64>,
}

/// Quick Index - minimal scan for fast startup
///
/// Per INDEXING_STRATEGY_SPEC.md Phase 1:
/// "Scan minimal headers to locate keyframes/OBU boundaries.
///  Enables first frame display ASAP."
///
/// Invariants:
/// - First seek point is always a keyframe
/// - Seek points are sorted by display_idx
/// - All keyframes are included
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuickIndex {
    /// Seek points (keyframes only)
    pub seek_points: Vec<SeekPoint>,

    /// Total file size in bytes
    pub file_size: u64,

    /// Estimated total frame count (may be inaccurate)
    pub estimated_frame_count: Option<usize>,
}

impl QuickIndex {
    /// Create a new quick index
    pub fn new(seek_points: Vec<SeekPoint>, file_size: u64) -> Self {
        Self {
            seek_points,
            file_size,
            estimated_frame_count: None,
        }
    }

    /// Find the nearest keyframe at or before the given display_idx
    ///
    /// Uses binary search since seek_points are sorted by display_idx.
    pub fn find_nearest_keyframe(&self, display_idx: usize) -> Option<&SeekPoint> {
        if self.seek_points.is_empty() {
            return None;
        }

        // Binary search: find first index where display_idx > target
        let idx = self
            .seek_points
            .partition_point(|sp| sp.display_idx <= display_idx);

        // partition_point returns the index where we'd insert to maintain order,
        // so the keyframe at or before target is at idx - 1 (if idx > 0)
        if idx > 0 {
            let candidate = &self.seek_points[idx - 1];
            if candidate.is_keyframe {
                return Some(candidate);
            }
        }

        // Fallback: shouldn't happen since all seek_points are keyframes,
        // but handle gracefully
        None
    }

    /// Get first keyframe
    pub fn first_keyframe(&self) -> Option<&SeekPoint> {
        self.seek_points.first()
    }

    /// Get last keyframe
    pub fn last_keyframe(&self) -> Option<&SeekPoint> {
        self.seek_points.last()
    }

    /// Check if quick index is empty
    pub fn is_empty(&self) -> bool {
        self.seek_points.is_empty()
    }
}

/// Full frame metadata for complete index
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameMetadata {
    /// Frame index in display order
    pub display_idx: usize,

    /// Frame index in decode order
    pub decode_idx: usize,

    /// Byte offset in file
    pub byte_offset: u64,

    /// Frame size in bytes
    pub size: u64,

    /// True if keyframe
    pub is_keyframe: bool,

    /// Presentation timestamp (if available)
    pub pts: Option<u64>,

    /// Decode timestamp (if available)
    pub dts: Option<u64>,

    /// Frame type (I, P, B, etc.) - codec-specific
    pub frame_type: Option<String>,
}

/// Full Index - complete frame→offset map
///
/// Per INDEXING_STRATEGY_SPEC.md Phase 2:
/// "Background task with progress indicator.
///  Builds full frame → offset map."
///
/// Invariants:
/// - Frames sorted by display_idx
/// - display_idx is contiguous (0, 1, 2, ...)
/// - All frames have valid byte_offset
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FullIndex {
    /// All frame metadata (sorted by display_idx)
    pub frames: Vec<FrameMetadata>,

    /// Total file size in bytes
    pub file_size: u64,

    /// True if index is complete
    pub is_complete: bool,
}

impl FullIndex {
    /// Create a new full index
    pub fn new(frames: Vec<FrameMetadata>, file_size: u64, is_complete: bool) -> Self {
        Self {
            frames,
            file_size,
            is_complete,
        }
    }

    /// Get frame metadata by display_idx
    pub fn get_frame(&self, display_idx: usize) -> Option<&FrameMetadata> {
        self.frames.get(display_idx)
    }

    /// Get total frame count
    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }

    /// Check if display_idx is within indexed range
    pub fn contains(&self, display_idx: usize) -> bool {
        display_idx < self.frames.len()
    }

    /// Get keyframe indices
    pub fn keyframe_indices(&self) -> Vec<usize> {
        self.frames
            .iter()
            .filter(|f| f.is_keyframe)
            .map(|f| f.display_idx)
            .collect()
    }

    /// Convert to QuickIndex (for fallback)
    pub fn to_quick_index(&self) -> QuickIndex {
        let seek_points = self
            .frames
            .iter()
            .filter(|f| f.is_keyframe)
            .map(|f| SeekPoint {
                display_idx: f.display_idx,
                byte_offset: f.byte_offset,
                is_keyframe: f.is_keyframe,
                pts: f.pts,
            })
            .collect();

        QuickIndex::new(seek_points, self.file_size)
    }
}

/// Index build progress tracker
///
/// Per INDEXING_STRATEGY_SPEC.md:
/// "Background task with progress indicator."
///
/// Thread-safe progress tracking for UI updates.
#[derive(Debug, Clone)]
pub struct IndexProgress {
    /// Progress value (0.0 = not started, 1.0 = complete)
    progress: Arc<AtomicU64>, // Stored as f64 bits

    /// True if indexing is complete
    is_complete: Arc<AtomicBool>,

    /// True if indexing is cancelled
    is_cancelled: Arc<AtomicBool>,

    /// Current status message
    status: Arc<Mutex<String>>,
}

impl IndexProgress {
    /// Create a new progress tracker
    pub fn new() -> Self {
        Self {
            progress: Arc::new(AtomicU64::new(0)),
            is_complete: Arc::new(AtomicBool::new(false)),
            is_cancelled: Arc::new(AtomicBool::new(false)),
            status: Arc::new(Mutex::new(String::from("Starting..."))),
        }
    }

    /// Get current progress (0.0 - 1.0)
    pub fn progress(&self) -> f64 {
        f64::from_bits(self.progress.load(Ordering::Relaxed))
    }

    /// Set progress (0.0 - 1.0)
    pub fn set_progress(&self, value: f64) {
        let clamped = value.clamp(0.0, 1.0);
        self.progress.store(clamped.to_bits(), Ordering::Relaxed);
    }

    /// Check if indexing is complete
    pub fn is_complete(&self) -> bool {
        self.is_complete.load(Ordering::Relaxed)
    }

    /// Mark indexing as complete
    pub fn mark_complete(&self) {
        self.set_progress(1.0);
        self.is_complete.store(true, Ordering::Relaxed);
        if let Ok(mut status) = self.status.lock() {
            *status = String::from("Complete");
        }
    }

    /// Check if indexing is cancelled
    pub fn is_cancelled(&self) -> bool {
        self.is_cancelled.load(Ordering::Relaxed)
    }

    /// Cancel indexing
    pub fn cancel(&self) {
        self.is_cancelled.store(true, Ordering::Relaxed);
        if let Ok(mut status) = self.status.lock() {
            *status = String::from("Cancelled");
        }
    }

    /// Get current status message
    pub fn status(&self) -> String {
        self.status
            .lock()
            .ok()
            .map(|s| s.clone())
            .unwrap_or_default()
    }

    /// Set status message
    pub fn set_status(&self, message: impl Into<String>) {
        if let Ok(mut status) = self.status.lock() {
            *status = message.into();
        }
    }
}

impl Default for IndexProgress {
    fn default() -> Self {
        Self::new()
    }
}

/// Index state (quick or full)
///
/// Per INDEXING_STRATEGY_SPEC.md:
/// Two-phase indexing: quick index first, then full index.
#[derive(Debug, Clone)]
pub enum IndexState {
    /// No index yet
    None,

    /// Quick index available
    Quick(QuickIndex),

    /// Full index building (with quick index available)
    Building {
        quick: QuickIndex,
        partial: FullIndex,
        progress: IndexProgress,
    },

    /// Full index complete
    Full(FullIndex),
}

impl IndexState {
    /// Get quick index (if available)
    pub fn quick(&self) -> Option<&QuickIndex> {
        match self {
            IndexState::Quick(q) => Some(q),
            IndexState::Building { quick, .. } => Some(quick),
            IndexState::Full(_f) => {
                // Can derive quick index from full
                // But we don't cache it here to avoid cloning
                None
            }
            IndexState::None => None,
        }
    }

    /// Get full index (if complete)
    pub fn full(&self) -> Option<&FullIndex> {
        match self {
            IndexState::Full(f) => Some(f),
            _ => None,
        }
    }

    /// Check if full index is available
    pub fn has_full_index(&self) -> bool {
        matches!(self, IndexState::Full(_))
    }

    /// Check if indexing is in progress
    pub fn is_building(&self) -> bool {
        matches!(self, IndexState::Building { .. })
    }

    /// Get progress (if building)
    pub fn progress(&self) -> Option<&IndexProgress> {
        match self {
            IndexState::Building { progress, .. } => Some(progress),
            _ => None,
        }
    }

    /// Check if display_idx is accessible (within indexed range)
    ///
    /// Per EDGE_CASES_AND_DEGRADE_BEHAVIOR.md §A:
    /// "Jumps allowed only within indexed range while building"
    pub fn can_access(&self, display_idx: usize) -> bool {
        match self {
            IndexState::None => false,
            IndexState::Quick(q) => {
                // With quick index, can access any keyframe
                q.find_nearest_keyframe(display_idx).is_some()
            }
            IndexState::Building { partial, .. } => {
                // Can access frames within partially built index
                partial.contains(display_idx)
            }
            IndexState::Full(f) => f.contains(display_idx),
        }
    }
}

// ============================================================================
// T1-2: Large File Open Fast-Path
// ============================================================================

/// Open strategy for file loading
///
/// Per T1-2 deliverable: OpenFastPath
///
/// Implements two-phase file open strategy:
/// 1. Fast path: Quick index scan → first frame ASAP
/// 2. Background: Full index build while user interacts
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OpenStrategy {
    /// Fast path: Build quick index only, show first frame ASAP
    /// Target: <= 60ms to first visible frame (per FAST_PATH_QUALITY_PATH_POLICY)
    FastPath,

    /// Full path: Wait for complete index (for small files only)
    FullPath,

    /// Adaptive: Choose based on file size
    /// Small files (<10MB): Full path
    /// Large files (>=10MB): Fast path
    #[default]
    Adaptive,
}

/// File open flow controller
///
/// Per INDEXING_STRATEGY_SPEC.md + FAST_PATH_QUALITY_PATH_POLICY.md:
/// - First frame display does not wait for full index
/// - Quick index enables immediate keyframe access
/// - Full index builds in background with progress
pub struct OpenFastPath {
    /// Open strategy
    strategy: OpenStrategy,

    /// File size threshold for adaptive mode (bytes)
    /// Default: 10MB
    adaptive_threshold: u64,
}

impl OpenFastPath {
    /// Create a new OpenFastPath controller
    pub fn new(strategy: OpenStrategy) -> Self {
        Self {
            strategy,
            adaptive_threshold: 10 * 1024 * 1024, // 10MB
        }
    }

    /// Set adaptive threshold (in bytes)
    pub fn with_adaptive_threshold(mut self, threshold: u64) -> Self {
        self.adaptive_threshold = threshold;
        self
    }

    /// Determine if quick index should be used for given file size
    ///
    /// Per T1-2: "Ensure first visible frame path does not wait for full index"
    pub fn should_use_quick_index(&self, file_size: u64) -> bool {
        match self.strategy {
            OpenStrategy::FastPath => true,
            OpenStrategy::FullPath => false,
            OpenStrategy::Adaptive => file_size >= self.adaptive_threshold,
        }
    }

    /// Check if first frame can be displayed
    ///
    /// Per EDGE_CASES_AND_DEGRADE_BEHAVIOR.md §A:
    /// "Indexing in progress: Allow jumps only within indexed range."
    pub fn can_display_first_frame(&self, index_state: &IndexState) -> bool {
        index_state.can_access(0)
    }

    /// Get status message for UI
    ///
    /// Per EDGE_CASES_AND_DEGRADE_BEHAVIOR.md §A:
    /// "Others show 'Index building...' (no queueing)."
    pub fn status_message(&self, index_state: &IndexState) -> &'static str {
        match index_state {
            IndexState::None => "No index",
            IndexState::Quick(_) => "Quick index ready",
            IndexState::Building { .. } => "Index building...",
            IndexState::Full(_) => "Index complete",
        }
    }
}

impl Default for OpenFastPath {
    fn default() -> Self {
        Self::new(OpenStrategy::Adaptive)
    }
}

/// Index-not-ready gating
///
/// Per T1-2 deliverable: Index-not-ready gating
///
/// Implements constraints when full index is not yet available.
pub struct IndexReadyGate {
    /// Current index state
    index_state: IndexState,
}

impl IndexReadyGate {
    /// Create a new gate with given index state
    pub fn new(index_state: IndexState) -> Self {
        Self { index_state }
    }

    /// Check if frame can be accessed
    ///
    /// Per EDGE_CASES_AND_DEGRADE_BEHAVIOR.md §A:
    /// "Jumps allowed only within indexed range."
    pub fn can_access_frame(&self, display_idx: usize) -> bool {
        self.index_state.can_access(display_idx)
    }

    /// Check if full index operations are available
    ///
    /// Operations requiring full index:
    /// - Frame-by-frame stepping through all frames
    /// - Metrics over entire sequence
    /// - Search operations
    pub fn is_full_index_ready(&self) -> bool {
        self.index_state.has_full_index()
    }

    /// Check if indexing is in progress
    pub fn is_indexing(&self) -> bool {
        self.index_state.is_building()
    }

    /// Get user-facing constraint message
    ///
    /// Per T1-2: Explicit messaging when operations are blocked.
    pub fn constraint_message(&self, display_idx: usize) -> Option<&'static str> {
        if !self.can_access_frame(display_idx) {
            if self.is_indexing() {
                Some("Frame not yet indexed. Index building in progress...")
            } else {
                Some("Frame not accessible with current index")
            }
        } else {
            None
        }
    }

    /// Get available frame range
    ///
    /// Returns (min, max) display_idx that can be accessed.
    pub fn accessible_range(&self) -> Option<(usize, usize)> {
        match &self.index_state {
            IndexState::None => None,
            IndexState::Quick(q) => q.seek_points.first().and_then(|first| {
                q.seek_points
                    .last()
                    .map(|last| (first.display_idx, last.display_idx))
            }),
            IndexState::Building { partial, .. } => {
                if partial.frames.is_empty() {
                    None
                } else {
                    Some((0, partial.frames.len() - 1))
                }
            }
            IndexState::Full(f) => {
                if f.frames.is_empty() {
                    None
                } else {
                    Some((0, f.frames.len() - 1))
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    include!("indexing_test.rs");
}
