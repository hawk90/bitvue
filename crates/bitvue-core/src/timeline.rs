//! Timeline Base Track - T4-1
//!
//! Per FRAME_IDENTITY_CONTRACT and TRI_SYNC_CONTRACT:
//! - display_idx is the primary timeline index (PTS order)
//! - Scrub/Jump integrated with selection tri-sync
//! - Timeline marker follows SelectionState.temporal

use serde::{Deserialize, Serialize};

/// Frame marker type for timeline visualization
///
/// Per DATA_MODEL §2.5: markers (error/bookmark/key)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FrameMarker {
    /// No marker
    None,
    /// Keyframe marker
    Key,
    /// Error marker (decode/syntax error)
    Error,
    /// User bookmark
    Bookmark,
    /// Scene change marker
    SceneChange,
}

impl FrameMarker {
    /// Check if this is a critical marker (affects rendering priority)
    pub fn is_critical(&self) -> bool {
        matches!(self, FrameMarker::Error | FrameMarker::Key)
    }

    /// Get marker color hint for UI rendering
    pub fn color_hint(&self) -> &'static str {
        match self {
            FrameMarker::None => "transparent",
            FrameMarker::Key => "blue",
            FrameMarker::Error => "red",
            FrameMarker::Bookmark => "yellow",
            FrameMarker::SceneChange => "green",
        }
    }
}

/// Frame metadata for timeline track
///
/// Per DATA_MODEL §2.5: TimelineIndex per-frame data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineFrame {
    /// Display index (PTS order, per FRAME_IDENTITY_CONTRACT)
    pub display_idx: usize,

    /// Frame size in bytes
    pub size_bytes: u64,

    /// Frame type (I, P, B, etc.)
    pub frame_type: String,

    /// Frame marker
    pub marker: FrameMarker,

    /// Presentation timestamp (if available)
    pub pts: Option<u64>,

    /// Decode timestamp (if available)
    pub dts: Option<u64>,

    /// Is this frame selected?
    pub is_selected: bool,
}

impl TimelineFrame {
    /// Create a new timeline frame
    pub fn new(display_idx: usize, size_bytes: u64, frame_type: String) -> Self {
        Self {
            display_idx,
            size_bytes,
            frame_type,
            marker: FrameMarker::None,
            pts: None,
            dts: None,
            is_selected: false,
        }
    }

    /// Set marker
    pub fn with_marker(mut self, marker: FrameMarker) -> Self {
        self.marker = marker;
        self
    }

    /// Set PTS
    pub fn with_pts(mut self, pts: u64) -> Self {
        self.pts = Some(pts);
        self
    }

    /// Set DTS
    pub fn with_dts(mut self, dts: u64) -> Self {
        self.dts = Some(dts);
        self
    }

    /// Check if frame has reorder (PTS ≠ DTS)
    ///
    /// Per WS_TIMELINE_TEMPORAL: ReorderMismatch band shows when pts!=dts
    pub fn has_reorder(&self) -> bool {
        match (self.pts, self.dts) {
            (Some(p), Some(d)) => p != d,
            _ => false,
        }
    }
}

/// Timeline scrub mode
///
/// Per PERFORMANCE_DEGRADATION_RULES: Quality path disabled during scrub
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScrubMode {
    /// Not scrubbing (normal playback/idle)
    Idle,
    /// Actively scrubbing (mouse drag, arrow keys)
    Active,
}

/// Jump direction for timeline navigation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JumpDirection {
    /// Next frame
    Next,
    /// Previous frame
    Prev,
    /// Next keyframe
    NextKey,
    /// Previous keyframe
    PrevKey,
    /// Next marker (error/bookmark)
    NextMarker,
    /// Previous marker
    PrevMarker,
    /// First frame
    First,
    /// Last frame
    Last,
}

/// Jump control result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JumpResult {
    /// Target display_idx
    pub target_idx: usize,
    /// Whether jump succeeded
    pub success: bool,
}

impl JumpResult {
    pub fn success(target_idx: usize) -> Self {
        Self {
            target_idx,
            success: true,
        }
    }

    pub fn failed(current_idx: usize) -> Self {
        Self {
            target_idx: current_idx,
            success: false,
        }
    }
}

/// Timeline Base Track
///
/// Per T4-1 deliverable: TimelineBase with scrub-to-selection sync and jump controls
///
/// Per FRAME_IDENTITY_CONTRACT:
/// "The app timeline uses display_idx as the canonical horizontal axis."
///
/// Per TRI_SYNC_CONTRACT §2.1:
/// "On SelectFrame(stream, frameKey): Update SelectionState.temporal, Timeline marker moves"
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineBase {
    /// Stream identifier
    pub stream_id: String,

    /// All frames in display order (indexed by display_idx)
    pub frames: Vec<TimelineFrame>,

    /// Current frame (display_idx)
    pub current_frame: Option<usize>,

    /// Scrub mode (affects performance degradation)
    pub scrub_mode: ScrubMode,

    /// Horizontal viewport range (first visible display_idx, count)
    pub viewport: (usize, usize),

    /// Vertical viewport range (first visible lane, lane count)
    pub vertical_viewport: (usize, usize),
}

impl TimelineBase {
    /// Create a new timeline base
    pub fn new(stream_id: String) -> Self {
        Self {
            stream_id,
            frames: Vec::new(),
            current_frame: None,
            scrub_mode: ScrubMode::Idle,
            viewport: (0, 0),
            vertical_viewport: (0, 0),
        }
    }

    /// Add a frame to the timeline
    pub fn add_frame(&mut self, frame: TimelineFrame) {
        self.frames.push(frame);
    }

    /// Get total frame count
    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }

    /// Get frame by display_idx
    pub fn get_frame(&self, display_idx: usize) -> Option<&TimelineFrame> {
        self.frames.get(display_idx)
    }

    /// Get mutable frame by display_idx
    pub fn get_frame_mut(&mut self, display_idx: usize) -> Option<&mut TimelineFrame> {
        self.frames.get_mut(display_idx)
    }

    /// Select a frame (tri-sync integration)
    ///
    /// Per TRI_SYNC_CONTRACT §2.1:
    /// "On SelectFrame(stream, frameKey): Update SelectionState.temporal"
    pub fn select_frame(&mut self, display_idx: usize) -> bool {
        if display_idx >= self.frames.len() {
            return false;
        }

        // Clear previous selection
        if let Some(current) = self.current_frame {
            if let Some(frame) = self.frames.get_mut(current) {
                frame.is_selected = false;
            }
        }

        // Set new selection
        self.current_frame = Some(display_idx);
        if let Some(frame) = self.frames.get_mut(display_idx) {
            frame.is_selected = true;
        }

        true
    }

    /// Clear frame selection
    pub fn clear_selection(&mut self) {
        if let Some(current) = self.current_frame {
            if let Some(frame) = self.frames.get_mut(current) {
                frame.is_selected = false;
            }
        }
        self.current_frame = None;
    }

    /// Get current frame index
    pub fn current_frame_idx(&self) -> Option<usize> {
        self.current_frame
    }

    /// Set scrub mode
    ///
    /// Per PERFORMANCE_DEGRADATION_RULES:
    /// "During scrub: Quality path disabled"
    pub fn set_scrub_mode(&mut self, mode: ScrubMode) {
        self.scrub_mode = mode;
    }

    /// Check if currently scrubbing
    pub fn is_scrubbing(&self) -> bool {
        self.scrub_mode == ScrubMode::Active
    }

    /// Jump to frame (with direction-based navigation)
    ///
    /// Per T4-1 deliverable: Jump controls
    pub fn jump(&mut self, direction: JumpDirection) -> JumpResult {
        let current = self.current_frame.unwrap_or(0);

        let target = match direction {
            JumpDirection::Next => {
                if current + 1 < self.frames.len() {
                    Some(current + 1)
                } else {
                    None
                }
            }
            JumpDirection::Prev => {
                if current > 0 {
                    Some(current - 1)
                } else {
                    None
                }
            }
            JumpDirection::First => Some(0),
            JumpDirection::Last => {
                if !self.frames.is_empty() {
                    Some(self.frames.len() - 1)
                } else {
                    None
                }
            }
            JumpDirection::NextKey => self.find_next_marker(current, FrameMarker::Key),
            JumpDirection::PrevKey => self.find_prev_marker(current, FrameMarker::Key),
            JumpDirection::NextMarker => self.find_next_any_marker(current),
            JumpDirection::PrevMarker => self.find_prev_any_marker(current),
        };

        if let Some(target_idx) = target {
            self.select_frame(target_idx);
            JumpResult::success(target_idx)
        } else {
            JumpResult::failed(current)
        }
    }

    /// Find next frame with specific marker
    fn find_next_marker(&self, from_idx: usize, marker: FrameMarker) -> Option<usize> {
        self.frames
            .iter()
            .enumerate()
            .skip(from_idx + 1)
            .find(|(_, f)| f.marker == marker)
            .map(|(idx, _)| idx)
    }

    /// Find previous frame with specific marker
    fn find_prev_marker(&self, from_idx: usize, marker: FrameMarker) -> Option<usize> {
        (0..from_idx)
            .rev()
            .find(|&idx| self.frames[idx].marker == marker)
    }

    /// Find next frame with any marker (excluding None)
    fn find_next_any_marker(&self, from_idx: usize) -> Option<usize> {
        self.frames
            .iter()
            .enumerate()
            .skip(from_idx + 1)
            .find(|(_, f)| f.marker != FrameMarker::None)
            .map(|(idx, _)| idx)
    }

    /// Find previous frame with any marker (excluding None)
    fn find_prev_any_marker(&self, from_idx: usize) -> Option<usize> {
        (0..from_idx)
            .rev()
            .find(|&idx| self.frames[idx].marker != FrameMarker::None)
    }

    /// Set viewport range (for virtualization)
    ///
    /// Per TRI_SYNC_CONTRACT §4: "Never clear selection due to scrolling"
    pub fn set_viewport(&mut self, first_visible: usize, count: usize) {
        self.viewport = (first_visible, count);
    }

    /// Get visible frame range
    pub fn visible_range(&self) -> (usize, usize) {
        self.viewport
    }

    /// Pan along x-axis (adjust viewport start position)
    ///
    /// Per TRI_SYNC_CONTRACT §4: "Never clear selection due to scrolling"
    ///
    /// # Arguments
    /// * `delta` - Number of frames to pan (positive = right, negative = left)
    pub fn pan_x(&mut self, delta: i32) {
        let (first, count) = self.viewport;
        let new_first = if delta < 0 {
            first.saturating_sub(delta.unsigned_abs() as usize)
        } else {
            first.saturating_add(delta as usize)
        };
        self.viewport = (new_first, count);
    }

    /// Zoom along x-axis (adjust viewport width)
    ///
    /// Per TRI_SYNC_CONTRACT §4: "Never clear selection due to scrolling"
    ///
    /// # Arguments
    /// * `factor` - Zoom factor (> 1.0 = zoom out/more frames, < 1.0 = zoom in/fewer frames)
    /// * `focal_point` - Optional frame index to zoom towards (defaults to viewport center)
    pub fn zoom(&mut self, factor: f32, focal_point: Option<usize>) {
        let (first, count) = self.viewport;

        // Calculate new count based on zoom factor
        let new_count = ((count as f32) * factor).max(1.0) as usize;

        // Adjust first_visible to keep focal point in roughly the same position
        let focal = focal_point.unwrap_or(first + count / 2);
        let relative_pos = if count > 0 {
            (focal.saturating_sub(first) as f32) / (count as f32)
        } else {
            0.5
        };

        let new_first = focal.saturating_sub((new_count as f32 * relative_pos) as usize);

        self.viewport = (new_first, new_count);
    }

    /// Pan along y-axis (adjust vertical viewport for lane scrolling)
    ///
    /// Per TRI_SYNC_CONTRACT §4: "Never clear selection due to scrolling"
    ///
    /// # Arguments
    /// * `delta` - Number of lanes to pan (positive = down, negative = up)
    pub fn pan_y(&mut self, delta: i32) {
        let (first, count) = self.vertical_viewport;
        let new_first = if delta < 0 {
            first.saturating_sub(delta.unsigned_abs() as usize)
        } else {
            first.saturating_add(delta as usize)
        };
        self.vertical_viewport = (new_first, count);
    }

    /// Set vertical viewport range (for lane virtualization)
    pub fn set_vertical_viewport(&mut self, first_visible_lane: usize, lane_count: usize) {
        self.vertical_viewport = (first_visible_lane, lane_count);
    }

    /// Get visible lane range
    pub fn visible_lane_range(&self) -> (usize, usize) {
        self.vertical_viewport
    }

    /// Zoom along y-axis (adjust vertical viewport height for lane zoom)
    ///
    /// Per TRI_SYNC_CONTRACT §4: "Never clear selection due to scrolling"
    ///
    /// # Arguments
    /// * `factor` - Zoom factor (> 1.0 = zoom out/more lanes, < 1.0 = zoom in/fewer lanes)
    /// * `focal_lane` - Optional lane index to zoom towards (defaults to viewport center)
    pub fn zoom_y(&mut self, factor: f32, focal_lane: Option<usize>) {
        let (first, count) = self.vertical_viewport;

        // Calculate new count based on zoom factor
        let new_count = ((count as f32) * factor).max(1.0) as usize;

        // Adjust first_visible to keep focal lane in roughly the same position
        let focal = focal_lane.unwrap_or(first + count / 2);
        let relative_pos = if count > 0 {
            (focal.saturating_sub(first) as f32) / (count as f32)
        } else {
            0.5
        };

        let new_first = focal.saturating_sub((new_count as f32 * relative_pos) as usize);

        self.vertical_viewport = (new_first, new_count);
    }

    /// Check if a frame is in viewport
    pub fn is_frame_visible(&self, display_idx: usize) -> bool {
        let (first, count) = self.viewport;
        display_idx >= first && display_idx < first + count
    }

    /// Get all keyframe indices
    pub fn keyframe_indices(&self) -> Vec<usize> {
        self.frames
            .iter()
            .enumerate()
            .filter_map(|(idx, f)| {
                if f.marker == FrameMarker::Key {
                    Some(idx)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Get all error frame indices
    pub fn error_indices(&self) -> Vec<usize> {
        self.frames
            .iter()
            .enumerate()
            .filter_map(|(idx, f)| {
                if f.marker == FrameMarker::Error {
                    Some(idx)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Get all bookmark indices
    pub fn bookmark_indices(&self) -> Vec<usize> {
        self.frames
            .iter()
            .enumerate()
            .filter_map(|(idx, f)| {
                if f.marker == FrameMarker::Bookmark {
                    Some(idx)
                } else {
                    None
                }
            })
            .collect()
    }
}

/// Per generate-tests skill: Comprehensive test suite with Arrange-Act-Assert pattern
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
    include!("timeline_test.rs");
}
