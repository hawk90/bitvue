//! Motion Vector Overlay - T3-2
//!
//! Per MV_VECTORS_IMPLEMENTATION_SPEC.md:
//! - Codec-agnostic MV grid (L0/L1 vectors in qpel)
//! - Density control: max 8000 visible vectors with stride sampling
//! - Arrow rendering with scaling and clamping
//! - Cache key generation for visible vector lists

use serde::{Deserialize, Serialize};

/// Sentinel value for missing motion vectors
pub const MISSING_MV: i32 = 2147483647;

/// Maximum visible vectors in viewport (hard cap per spec §2.2)
pub const MAX_VISIBLE_VECTORS: usize = 8000;

/// Maximum arrow length at Fit zoom (pixels, per spec §2.2)
pub const MAX_ARROW_LENGTH_PX: f32 = 48.0;

/// Default user scale for MV display
pub const DEFAULT_USER_SCALE: f32 = 1.0;

/// Default opacity for MV overlay
pub const DEFAULT_OPACITY: f32 = 0.55;

/// Motion vector in quarter-pel units (per spec §1)
///
/// Quarter-pel is the standard subpixel precision used by video codecs.
/// To convert to pixels: px = qpel / 4.0
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MotionVector {
    /// Horizontal displacement in quarter-pel units
    pub dx_qpel: i32,
    /// Vertical displacement in quarter-pel units
    pub dy_qpel: i32,
}

impl MotionVector {
    /// Create a new motion vector
    pub fn new(dx_qpel: i32, dy_qpel: i32) -> Self {
        Self { dx_qpel, dy_qpel }
    }

    /// Check if this vector is missing (uses sentinel value)
    pub fn is_missing(&self) -> bool {
        self.dx_qpel == MISSING_MV || self.dy_qpel == MISSING_MV
    }

    /// Convert quarter-pel to pixels
    pub fn to_pixels(&self) -> (f32, f32) {
        (self.dx_qpel as f32 / 4.0, self.dy_qpel as f32 / 4.0)
    }

    /// Get magnitude in pixels
    pub fn magnitude_px(&self) -> f32 {
        let (dx_px, dy_px) = self.to_pixels();
        (dx_px * dx_px + dy_px * dy_px).sqrt()
    }

    /// Missing vector constant
    pub const MISSING: Self = Self {
        dx_qpel: MISSING_MV,
        dy_qpel: MISSING_MV,
    };

    /// Zero vector constant
    pub const ZERO: Self = Self {
        dx_qpel: 0,
        dy_qpel: 0,
    };
}

impl Default for MotionVector {
    fn default() -> Self {
        Self::ZERO
    }
}

/// Block mode (optional, per spec §1)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum BlockMode {
    None = 0,
    Inter = 1,
    Intra = 2,
    Skip = 3,
}

impl From<u8> for BlockMode {
    fn from(value: u8) -> Self {
        match value {
            1 => BlockMode::Inter,
            2 => BlockMode::Intra,
            3 => BlockMode::Skip,
            _ => BlockMode::None,
        }
    }
}

/// Codec-agnostic motion vector grid (per spec §1)
///
/// Represents MV data for a single frame with L0/L1 reference lists.
/// Grid is organized in row-major order (same as QP grid).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MVGrid {
    /// Coded frame width in pixels
    pub coded_width: u32,
    /// Coded frame height in pixels
    pub coded_height: u32,
    /// Block width in pixels
    pub block_w: u32,
    /// Block height in pixels
    pub block_h: u32,
    /// Grid width in blocks
    pub grid_w: u32,
    /// Grid height in blocks
    pub grid_h: u32,
    /// L0 motion vectors (forward prediction)
    pub mv_l0: Vec<MotionVector>,
    /// L1 motion vectors (backward prediction)
    pub mv_l1: Vec<MotionVector>,
    /// Optional block modes
    pub mode: Option<Vec<BlockMode>>,
}

impl MVGrid {
    /// Create a new MV grid
    ///
    /// # Panics
    /// Panics if grid dimensions don't match vector lengths
    pub fn new(
        coded_width: u32,
        coded_height: u32,
        block_w: u32,
        block_h: u32,
        mv_l0: Vec<MotionVector>,
        mv_l1: Vec<MotionVector>,
        mode: Option<Vec<BlockMode>>,
    ) -> Self {
        let grid_w = coded_width.div_ceil(block_w);
        let grid_h = coded_height.div_ceil(block_h);
        let expected_len = (grid_w * grid_h) as usize;

        assert_eq!(
            mv_l0.len(),
            expected_len,
            "mv_l0 length mismatch: expected {}, got {}",
            expected_len,
            mv_l0.len()
        );
        assert_eq!(
            mv_l1.len(),
            expected_len,
            "mv_l1 length mismatch: expected {}, got {}",
            expected_len,
            mv_l1.len()
        );

        if let Some(ref m) = mode {
            assert_eq!(
                m.len(),
                expected_len,
                "mode length mismatch: expected {}, got {}",
                expected_len,
                m.len()
            );
        }

        Self {
            coded_width,
            coded_height,
            block_w,
            block_h,
            grid_w,
            grid_h,
            mv_l0,
            mv_l1,
            mode,
        }
    }

    /// Get L0 vector at block position
    pub fn get_l0(&self, col: u32, row: u32) -> Option<MotionVector> {
        if col >= self.grid_w || row >= self.grid_h {
            return None;
        }
        let idx = (row * self.grid_w + col) as usize;
        self.mv_l0.get(idx).copied()
    }

    /// Get L1 vector at block position
    pub fn get_l1(&self, col: u32, row: u32) -> Option<MotionVector> {
        if col >= self.grid_w || row >= self.grid_h {
            return None;
        }
        let idx = (row * self.grid_w + col) as usize;
        self.mv_l1.get(idx).copied()
    }

    /// Get block mode at position
    pub fn get_mode(&self, col: u32, row: u32) -> Option<BlockMode> {
        if col >= self.grid_w || row >= self.grid_h {
            return None;
        }
        let idx = (row * self.grid_w + col) as usize;
        self.mode.as_ref().and_then(|m| m.get(idx).copied())
    }

    /// Get block center in coded pixels
    pub fn block_center(&self, col: u32, row: u32) -> (f32, f32) {
        let x = (col * self.block_w) as f32 + (self.block_w as f32 / 2.0);
        let y = (row * self.block_h) as f32 + (self.block_h as f32 / 2.0);
        (x, y)
    }

    /// Total number of blocks
    pub fn block_count(&self) -> usize {
        (self.grid_w * self.grid_h) as usize
    }
}

/// MV layer selection (per spec §2.3)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MVLayer {
    /// Show L0 vectors only
    L0Only,
    /// Show L1 vectors only
    L1Only,
    /// Show both L0 and L1 vectors
    Both,
}

impl MVLayer {
    /// Get cache key suffix
    pub fn cache_suffix(&self) -> &'static str {
        match self {
            MVLayer::L0Only => "L0",
            MVLayer::L1Only => "L1",
            MVLayer::Both => "both",
        }
    }
}

/// Viewport rectangle in coded pixels
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Viewport {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl Viewport {
    pub fn new(x: u32, y: u32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Check if a block is visible in this viewport
    pub fn contains_block(&self, block_x: u32, block_y: u32, block_w: u32, block_h: u32) -> bool {
        let block_right = block_x + block_w;
        let block_bottom = block_y + block_h;
        let vp_right = self.x + self.width;
        let vp_bottom = self.y + self.height;

        // Check for overlap
        !(block_right <= self.x
            || block_x >= vp_right
            || block_bottom <= self.y
            || block_y >= vp_bottom)
    }
}

/// Density control for MV rendering (per spec §2.2)
///
/// Implements stride sampling to cap visible vectors at MAX_VISIBLE_VECTORS
pub struct DensityControl;

impl DensityControl {
    /// Calculate sampling stride for given visible block count
    ///
    /// Per spec §2.2:
    /// - If Nv <= 8000: stride = 1 (draw all)
    /// - Else: stride = ceil(sqrt(Nv/8000))
    pub fn calculate_stride(visible_blocks: usize) -> u32 {
        if visible_blocks <= MAX_VISIBLE_VECTORS {
            1
        } else {
            let ratio = visible_blocks as f32 / MAX_VISIBLE_VECTORS as f32;
            ratio.sqrt().ceil() as u32
        }
    }

    /// Check if a block should be drawn given the stride
    ///
    /// Per spec §2.2: draw blocks where (bx % stride == 0 && by % stride == 0)
    pub fn should_draw(col: u32, row: u32, stride: u32) -> bool {
        col.is_multiple_of(stride) && row.is_multiple_of(stride)
    }

    /// Estimate visible vector count after stride sampling
    pub fn estimate_drawn_count(visible_blocks: usize, stride: u32) -> usize {
        if stride == 1 {
            visible_blocks
        } else {
            // Approximate: visible_blocks / (stride * stride)
            visible_blocks / ((stride * stride) as usize)
        }
    }
}

/// MV scaling and clamping (per spec §2.2)
pub struct MVScaling;

impl MVScaling {
    /// Scale motion vector for display
    ///
    /// Per spec §2.2:
    /// - mv_px = (dx_qpel/4, dy_qpel/4)
    /// - display = mv_px * user_scale * zoom_scale
    pub fn scale_vector(mv: &MotionVector, user_scale: f32, zoom_scale: f32) -> (f32, f32) {
        let (dx_px, dy_px) = mv.to_pixels();
        (
            dx_px * user_scale * zoom_scale,
            dy_px * user_scale * zoom_scale,
        )
    }

    /// Clamp arrow length to max (per spec §2.2: 48px at Fit zoom)
    pub fn clamp_arrow_length(dx: f32, dy: f32, max_length: f32) -> (f32, f32) {
        let magnitude = (dx * dx + dy * dy).sqrt();
        if magnitude > max_length {
            let scale = max_length / magnitude;
            (dx * scale, dy * scale)
        } else {
            (dx, dy)
        }
    }
}

/// Cache key for MV overlay (per spec §4)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MVCacheKey {
    pub stream_id: String,
    pub frame_idx: usize,
    pub viewport: (u32, u32, u32, u32), // (x, y, w, h)
    pub stride: u32,
    pub layer: String,       // "L0", "L1", "both"
    pub scale_bucket: u32,   // user_scale * 10
    pub opacity_bucket: u32, // opacity * 20
}

impl MVCacheKey {
    /// Create cache key from overlay parameters
    pub fn new(
        stream_id: String,
        frame_idx: usize,
        viewport: Viewport,
        stride: u32,
        layer: MVLayer,
        user_scale: f32,
        opacity: f32,
    ) -> Self {
        Self {
            stream_id,
            frame_idx,
            viewport: (viewport.x, viewport.y, viewport.width, viewport.height),
            stride,
            layer: layer.cache_suffix().to_string(),
            scale_bucket: scale_bucket(user_scale),
            opacity_bucket: opacity_bucket(opacity),
        }
    }
}

impl std::fmt::Display for MVCacheKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "overlay_mv:{}:f{}|vp{},{},{},{}|s{}|L{}|sc{}|op{}",
            self.stream_id,
            self.frame_idx,
            self.viewport.0,
            self.viewport.1,
            self.viewport.2,
            self.viewport.3,
            self.stride,
            self.layer,
            self.scale_bucket,
            self.opacity_bucket
        )
    }
}

/// Scale bucketing (per spec §4: round to 0.1 steps)
fn scale_bucket(user_scale: f32) -> u32 {
    (user_scale * 10.0).round() as u32
}

/// Opacity bucketing (per spec §4: round to 0.05 steps)
fn opacity_bucket(opacity: f32) -> u32 {
    (opacity * 20.0).round() as u32
}

/// Motion Vector Overlay (main component)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MVOverlay {
    /// MV grid data
    pub grid: MVGrid,
    /// Layer selection (L0/L1/both)
    pub layer: MVLayer,
    /// User scale factor
    pub user_scale: f32,
    /// Opacity (0.0 to 1.0)
    pub opacity: f32,
    /// Cached visible vectors (optional)
    pub cached_visible: Option<Vec<(u32, u32)>>, // (col, row) pairs
    /// Cache key for current state
    pub cache_key: Option<MVCacheKey>,
}

impl MVOverlay {
    /// Create a new MV overlay
    pub fn new(grid: MVGrid) -> Self {
        Self {
            grid,
            layer: MVLayer::Both,
            user_scale: DEFAULT_USER_SCALE,
            opacity: DEFAULT_OPACITY,
            cached_visible: None,
            cache_key: None,
        }
    }

    /// Set layer selection
    pub fn set_layer(&mut self, layer: MVLayer) {
        if self.layer != layer {
            self.layer = layer;
            self.invalidate_cache();
        }
    }

    /// Set user scale
    pub fn set_user_scale(&mut self, scale: f32) {
        let new_bucket = scale_bucket(scale);
        let old_bucket = scale_bucket(self.user_scale);
        self.user_scale = scale;
        if new_bucket != old_bucket {
            self.invalidate_cache();
        }
    }

    /// Set opacity
    pub fn set_opacity(&mut self, opacity: f32) {
        let new_bucket = opacity_bucket(opacity);
        let old_bucket = opacity_bucket(self.opacity);
        self.opacity = opacity.clamp(0.0, 1.0);
        if new_bucket != old_bucket {
            self.invalidate_cache();
        }
    }

    /// Compute visible blocks for viewport with stride sampling
    ///
    /// Per optimize-code skill: Pre-allocate capacity based on viewport size
    /// to reduce reallocations during iteration.
    pub fn compute_visible_blocks(&self, viewport: Viewport) -> (Vec<(u32, u32)>, u32) {
        // Estimate visible block count for capacity pre-allocation
        // This reduces allocations compared to push-based growth
        let estimated_count = ((viewport.width / self.grid.block_w + 1)
            * (viewport.height / self.grid.block_h + 1)) as usize;

        // Find blocks overlapping viewport
        let mut visible = Vec::with_capacity(estimated_count);
        for row in 0..self.grid.grid_h {
            for col in 0..self.grid.grid_w {
                let block_x = col * self.grid.block_w;
                let block_y = row * self.grid.block_h;
                if viewport.contains_block(block_x, block_y, self.grid.block_w, self.grid.block_h) {
                    visible.push((col, row));
                }
            }
        }

        // Calculate stride
        let stride = DensityControl::calculate_stride(visible.len());

        // Apply stride sampling
        let sampled: Vec<(u32, u32)> = visible
            .into_iter()
            .filter(|(col, row)| DensityControl::should_draw(*col, *row, stride))
            .collect();

        (sampled, stride)
    }

    /// Update cache for new viewport/parameters
    pub fn update_cache(&mut self, stream_id: String, frame_idx: usize, viewport: Viewport) {
        let (visible, stride) = self.compute_visible_blocks(viewport);

        let new_key = MVCacheKey::new(
            stream_id,
            frame_idx,
            viewport,
            stride,
            self.layer,
            self.user_scale,
            self.opacity,
        );

        // Only update if key changed
        if self.cache_key.as_ref() != Some(&new_key) {
            self.cached_visible = Some(visible);
            self.cache_key = Some(new_key);
        }
    }

    /// Invalidate cache
    fn invalidate_cache(&mut self) {
        self.cached_visible = None;
        self.cache_key = None;
    }

    /// Get cached visible blocks (or compute if not cached)
    pub fn get_visible_blocks(&self, viewport: Viewport) -> Vec<(u32, u32)> {
        if let Some(ref cached) = self.cached_visible {
            cached.clone()
        } else {
            self.compute_visible_blocks(viewport).0
        }
    }

    /// Get motion vector statistics
    pub fn statistics(&self) -> MVStatistics {
        self.grid.statistics()
    }
}

/// Motion vector statistics for UI display
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MVStatistics {
    /// Total blocks in grid
    pub total_blocks: usize,
    /// L0 vectors present (non-missing)
    pub l0_present: usize,
    /// L1 vectors present (non-missing)
    pub l1_present: usize,
    /// L0 average magnitude (pixels)
    pub l0_avg_magnitude: f32,
    /// L1 average magnitude (pixels)
    pub l1_avg_magnitude: f32,
    /// L0 max magnitude (pixels)
    pub l0_max_magnitude: f32,
    /// L1 max magnitude (pixels)
    pub l1_max_magnitude: f32,
    /// Blocks with inter mode
    pub inter_count: usize,
    /// Blocks with intra mode
    pub intra_count: usize,
    /// Blocks with skip mode
    pub skip_count: usize,
}

impl MVStatistics {
    /// Format as summary string
    pub fn summary(&self) -> String {
        format!(
            "L0: {} vectors (avg {:.1}px), L1: {} vectors (avg {:.1}px)",
            self.l0_present, self.l0_avg_magnitude, self.l1_present, self.l1_avg_magnitude
        )
    }

    /// Get total vectors present (L0 + L1)
    pub fn total_vectors(&self) -> usize {
        self.l0_present + self.l1_present
    }
}

impl MVGrid {
    /// Calculate motion vector statistics
    pub fn statistics(&self) -> MVStatistics {
        let mut stats = MVStatistics {
            total_blocks: self.block_count(),
            ..Default::default()
        };

        let mut l0_sum = 0.0f32;
        let mut l1_sum = 0.0f32;

        for row in 0..self.grid_h {
            for col in 0..self.grid_w {
                // L0 statistics
                if let Some(mv) = self.get_l0(col, row) {
                    if !mv.is_missing() {
                        stats.l0_present += 1;
                        let mag = mv.magnitude_px();
                        l0_sum += mag;
                        if mag > stats.l0_max_magnitude {
                            stats.l0_max_magnitude = mag;
                        }
                    }
                }

                // L1 statistics
                if let Some(mv) = self.get_l1(col, row) {
                    if !mv.is_missing() {
                        stats.l1_present += 1;
                        let mag = mv.magnitude_px();
                        l1_sum += mag;
                        if mag > stats.l1_max_magnitude {
                            stats.l1_max_magnitude = mag;
                        }
                    }
                }

                // Mode statistics
                if let Some(mode) = self.get_mode(col, row) {
                    match mode {
                        BlockMode::Inter => stats.inter_count += 1,
                        BlockMode::Intra => stats.intra_count += 1,
                        BlockMode::Skip => stats.skip_count += 1,
                        BlockMode::None => {}
                    }
                }
            }
        }

        // Calculate averages
        if stats.l0_present > 0 {
            stats.l0_avg_magnitude = l0_sum / stats.l0_present as f32;
        }
        if stats.l1_present > 0 {
            stats.l1_avg_magnitude = l1_sum / stats.l1_present as f32;
        }

        stats
    }
}

/// Per generate-tests skill: Comprehensive test suite with Arrange-Act-Assert pattern
#[cfg(test)]
mod tests {
    include!("mv_overlay_test.rs");
}
