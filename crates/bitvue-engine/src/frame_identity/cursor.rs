//! Global cursor/crosshair synced across timeline, player, and overlay views (viz_core.009).

use super::{FrameIndexMap, TimelineAxis};

/// Cursor/crosshair visibility state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorVisibility {
    /// Cursor is visible
    Visible,
    /// Cursor is hidden
    Hidden,
}

/// Timeline cursor/crosshair
///
/// Deliverable: cursor:FrameIdentity:Timeline:AV1:viz_core
///
/// Global cursor that syncs across timeline, player, and overlays.
/// Per FRAME_IDENTITY_CONTRACT:
/// - Cursor position is in display_idx coordinates
/// - All operations use display_idx
#[derive(Debug, Clone)]
pub struct TimelineCursor {
    /// Current cursor position (display_idx)
    position: Option<usize>,
    /// Visibility state
    visibility: CursorVisibility,
    /// Total frame count (for bounds checking)
    total_frames: usize,
}

impl TimelineCursor {
    /// Create new timeline cursor
    ///
    /// # Arguments
    ///
    /// * `total_frames` - Total frame count from FrameIndexMap
    pub fn new(total_frames: usize) -> Self {
        Self {
            position: None,
            visibility: CursorVisibility::Hidden,
            total_frames,
        }
    }

    /// Get current cursor position (display_idx)
    ///
    /// Per FRAME_IDENTITY_CONTRACT: Returns display_idx.
    pub fn position(&self) -> Option<usize> {
        self.position
    }

    /// Set cursor position (display_idx)
    ///
    /// Per FRAME_IDENTITY_CONTRACT: Takes display_idx as input.
    /// Automatically clamps to valid range and shows cursor.
    pub fn set_position(&mut self, display_idx: usize) {
        if self.total_frames == 0 {
            self.position = None;
            self.visibility = CursorVisibility::Hidden;
            return;
        }

        // Clamp to valid range
        let clamped = display_idx.min(self.total_frames - 1);
        self.position = Some(clamped);
        self.visibility = CursorVisibility::Visible;
    }

    /// Clear cursor position (hide cursor)
    pub fn clear(&mut self) {
        self.position = None;
        self.visibility = CursorVisibility::Hidden;
    }

    /// Get visibility state
    pub fn visibility(&self) -> CursorVisibility {
        self.visibility
    }

    /// Show cursor at current position
    pub fn show(&mut self) {
        if self.position.is_some() {
            self.visibility = CursorVisibility::Visible;
        }
    }

    /// Hide cursor (keeps position)
    pub fn hide(&mut self) {
        self.visibility = CursorVisibility::Hidden;
    }

    /// Check if cursor is visible
    pub fn is_visible(&self) -> bool {
        self.visibility == CursorVisibility::Visible && self.position.is_some()
    }

    /// Move cursor by delta frames (positive = right, negative = left)
    ///
    /// Per FRAME_IDENTITY_CONTRACT: Operates on display_idx.
    pub fn move_by(&mut self, delta: isize) {
        if let Some(current) = self.position {
            let new_pos = (current as isize + delta).max(0) as usize;
            self.set_position(new_pos);
        }
    }

    /// Move cursor to next frame
    pub fn next_frame(&mut self) {
        self.move_by(1);
    }

    /// Move cursor to previous frame
    pub fn prev_frame(&mut self) {
        self.move_by(-1);
    }

    /// Get total frame count
    pub fn total_frames(&self) -> usize {
        self.total_frames
    }
}

// ============================================================================
// Cursor Sync - Timeline ↔ Player ↔ Overlays
// ============================================================================

/// Cursor sync coordinator
///
/// Synchronizes cursor position across:
/// - Timeline view (horizontal axis)
/// - Player view (current frame display)
/// - Overlay views (QP heatmap, MV, etc.)
///
/// Per FRAME_IDENTITY_CONTRACT:
/// - All sync operations use display_idx
/// - decode_idx is never exposed
#[derive(Debug, Clone)]
pub struct CursorSync {
    /// Global cursor state
    cursor: TimelineCursor,
    /// Frame index map (for PTS queries)
    index_map: FrameIndexMap,
}

impl CursorSync {
    /// Create new cursor sync coordinator
    pub fn new(index_map: FrameIndexMap) -> Self {
        let total_frames = index_map.frame_count();
        Self {
            cursor: TimelineCursor::new(total_frames),
            index_map,
        }
    }

    /// Get cursor
    pub fn cursor(&self) -> &TimelineCursor {
        &self.cursor
    }

    /// Get mutable cursor
    pub fn cursor_mut(&mut self) -> &mut TimelineCursor {
        &mut self.cursor
    }

    /// Sync cursor from timeline click (pixel → display_idx)
    ///
    /// Per FRAME_IDENTITY_CONTRACT: Converts pixel to display_idx.
    pub fn sync_from_timeline_click(&mut self, pixel_x: f32, axis: &TimelineAxis) {
        let display_idx = axis.pixel_to_display_idx(pixel_x);
        self.cursor.set_position(display_idx);
    }

    /// Sync cursor from player seek (display_idx)
    ///
    /// Per FRAME_IDENTITY_CONTRACT: Takes display_idx as input.
    pub fn sync_from_player_seek(&mut self, display_idx: usize) {
        self.cursor.set_position(display_idx);
    }

    /// Sync cursor from PTS (for external control)
    ///
    /// Converts PTS → display_idx and syncs cursor.
    pub fn sync_from_pts(&mut self, pts: u64) -> bool {
        // Find display_idx for this PTS
        for display_idx in 0..self.index_map.frame_count() {
            if self.index_map.get_pts(display_idx) == Some(pts) {
                self.cursor.set_position(display_idx);
                return true;
            }
        }
        false // PTS not found
    }

    /// Get cursor PTS (for external queries)
    ///
    /// Per FRAME_IDENTITY_CONTRACT: Uses display_idx internally.
    pub fn cursor_pts(&self) -> Option<u64> {
        let display_idx = self.cursor.position()?;
        self.index_map.get_pts(display_idx)
    }

    /// Get frame index map
    pub fn index_map(&self) -> &FrameIndexMap {
        &self.index_map
    }
}
