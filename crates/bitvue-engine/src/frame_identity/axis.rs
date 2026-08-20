//! Timeline horizontal axis: pan/zoom/scale over `display_idx` coordinates (viz_core.007).

/// Timeline axis scale mode
///
/// Controls how the timeline axis scales to fit viewport.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AxisScaleMode {
    /// Auto-scale to fit all frames in viewport
    Auto,
    /// Fixed scale (pixels per frame)
    Fixed(u32),
    /// Fit exactly N frames in viewport
    FitFrames(usize),
}

/// Timeline axis bounds
///
/// Defines the visible range of the timeline axis.
/// Per FRAME_IDENTITY_CONTRACT: bounds are in display_idx coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AxisBounds {
    /// First visible frame (display_idx)
    pub start: usize,
    /// Last visible frame (display_idx, inclusive)
    pub end: usize,
}

impl AxisBounds {
    /// Create new axis bounds
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    /// Get visible frame count
    pub fn frame_count(&self) -> usize {
        if self.end >= self.start {
            self.end - self.start + 1
        } else {
            0
        }
    }

    /// Check if display_idx is within bounds
    pub fn contains(&self, display_idx: usize) -> bool {
        display_idx >= self.start && display_idx <= self.end
    }

    /// Clamp display_idx to bounds
    pub fn clamp(&self, display_idx: usize) -> usize {
        display_idx.clamp(self.start, self.end)
    }
}

/// Timeline axis
///
/// Deliverable: axis:FrameIdentity:Timeline:AV1:viz_core
///
/// Manages timeline horizontal axis with multi-scale support.
/// Per FRAME_IDENTITY_CONTRACT:
/// - Axis uses display_idx as primary coordinate
/// - All queries/operations use display_idx
#[derive(Debug, Clone)]
pub struct TimelineAxis {
    /// Total frame count (from FrameIndexMap)
    total_frames: usize,
    /// Viewport width in pixels
    viewport_width_px: f32,
    /// Scale mode
    scale_mode: AxisScaleMode,
    /// Current visible bounds (display_idx range)
    bounds: AxisBounds,
    /// Minimum pixels per frame (for readability)
    min_pixels_per_frame: f32,
}

impl TimelineAxis {
    /// Create new timeline axis
    ///
    /// # Arguments
    ///
    /// * `total_frames` - Total frame count from FrameIndexMap
    /// * `viewport_width_px` - Viewport width in pixels
    /// * `scale_mode` - Initial scale mode
    pub fn new(total_frames: usize, viewport_width_px: f32, scale_mode: AxisScaleMode) -> Self {
        let bounds = AxisBounds::new(0, total_frames.saturating_sub(1));

        let mut axis = Self {
            total_frames,
            viewport_width_px,
            scale_mode,
            bounds,
            min_pixels_per_frame: 2.0, // Minimum 2px per frame
        };

        axis.update_bounds_from_scale();
        axis
    }

    /// Get current scale mode
    pub fn scale_mode(&self) -> AxisScaleMode {
        self.scale_mode
    }

    /// Set scale mode
    pub fn set_scale_mode(&mut self, mode: AxisScaleMode) {
        self.scale_mode = mode;
        self.update_bounds_from_scale();
    }

    /// Get current bounds
    pub fn bounds(&self) -> AxisBounds {
        self.bounds
    }

    /// Set bounds (manual pan/zoom)
    ///
    /// Clamps to valid range [0, total_frames - 1]
    pub fn set_bounds(&mut self, start: usize, end: usize) {
        let clamped_start = start.min(self.total_frames.saturating_sub(1));
        let clamped_end = end.min(self.total_frames.saturating_sub(1));

        self.bounds = AxisBounds::new(clamped_start, clamped_end);
        self.scale_mode = AxisScaleMode::Fixed(self.pixels_per_frame() as u32);
    }

    /// Update bounds based on current scale mode
    fn update_bounds_from_scale(&mut self) {
        if self.total_frames == 0 {
            // Empty stream: set end < start to get frame_count() = 0
            self.bounds = AxisBounds::new(0, 0);
            self.bounds.end = 0;
            self.bounds.start = 1; // end < start → frame_count = 0
            return;
        }

        match self.scale_mode {
            AxisScaleMode::Auto => {
                // Show all frames
                self.bounds = AxisBounds::new(0, self.total_frames - 1);
            }
            AxisScaleMode::Fixed(pixels_per_frame) => {
                // Keep current bounds, adjust scale
                let visible_frames =
                    (self.viewport_width_px / pixels_per_frame as f32).max(1.0) as usize;
                let end = (self.bounds.start + visible_frames - 1).min(self.total_frames - 1);
                self.bounds.end = end;
            }
            AxisScaleMode::FitFrames(n) => {
                // Show exactly N frames starting from current position
                let n = n.max(1).min(self.total_frames);
                let end = (self.bounds.start + n - 1).min(self.total_frames - 1);
                self.bounds.end = end;
            }
        }
    }

    /// Get pixels per frame for current scale
    pub fn pixels_per_frame(&self) -> f32 {
        let visible_frames = self.bounds.frame_count();
        if visible_frames == 0 {
            return self.min_pixels_per_frame;
        }

        let ppf = self.viewport_width_px / visible_frames as f32;
        ppf.max(self.min_pixels_per_frame)
    }

    /// Convert display_idx to pixel position
    ///
    /// Per FRAME_IDENTITY_CONTRACT: Uses display_idx as input.
    pub fn display_idx_to_pixel(&self, display_idx: usize) -> f32 {
        if display_idx < self.bounds.start {
            return 0.0;
        }

        let offset = (display_idx - self.bounds.start) as f32;
        offset * self.pixels_per_frame()
    }

    /// Convert pixel position to display_idx
    ///
    /// Per FRAME_IDENTITY_CONTRACT: Returns display_idx.
    pub fn pixel_to_display_idx(&self, pixel_x: f32) -> usize {
        let offset = (pixel_x / self.pixels_per_frame()).floor() as usize;
        let display_idx = self.bounds.start + offset;

        // Clamp to valid range
        display_idx.min(self.total_frames.saturating_sub(1))
    }

    /// Pan axis by frame count (positive = right, negative = left)
    ///
    /// Per FRAME_IDENTITY_CONTRACT: Operates on display_idx.
    pub fn pan(&mut self, delta_frames: isize) {
        if self.total_frames == 0 {
            return;
        }

        let visible_frames = self.bounds.frame_count();
        let max_start = self.total_frames.saturating_sub(visible_frames);

        let current_start = self.bounds.start as isize;
        let new_start = (current_start + delta_frames)
            .max(0)
            .min(max_start as isize) as usize;

        let new_end = (new_start + visible_frames - 1).min(self.total_frames - 1);

        self.bounds = AxisBounds::new(new_start, new_end);
    }

    /// Zoom in (show fewer frames with more detail)
    pub fn zoom_in(&mut self) {
        let current_visible = self.bounds.frame_count();
        let new_visible = (current_visible / 2).max(1);

        self.scale_mode = AxisScaleMode::FitFrames(new_visible);
        self.update_bounds_from_scale();
    }

    /// Zoom out (show more frames with less detail)
    pub fn zoom_out(&mut self) {
        let current_visible = self.bounds.frame_count();
        let new_visible = (current_visible * 2).min(self.total_frames);

        self.scale_mode = AxisScaleMode::FitFrames(new_visible);
        self.update_bounds_from_scale();
    }

    /// Center axis on specific display_idx
    pub fn center_on(&mut self, display_idx: usize) {
        if self.total_frames == 0 {
            return;
        }

        let visible_frames = self.bounds.frame_count();
        let half = visible_frames / 2;

        // Calculate ideal start (centering on display_idx)
        let ideal_start = display_idx.saturating_sub(half);

        // Clamp to ensure we always show visible_frames frames
        let max_start = self.total_frames.saturating_sub(visible_frames);
        let new_start = ideal_start.min(max_start);

        let new_end = (new_start + visible_frames - 1).min(self.total_frames - 1);

        self.bounds = AxisBounds::new(new_start, new_end);
    }

    /// Get viewport width
    pub fn viewport_width(&self) -> f32 {
        self.viewport_width_px
    }

    /// Set viewport width (triggers rescale)
    pub fn set_viewport_width(&mut self, width_px: f32) {
        self.viewport_width_px = width_px;
        self.update_bounds_from_scale();
    }

    /// Get total frame count
    pub fn total_frames(&self) -> usize {
        self.total_frames
    }
}
