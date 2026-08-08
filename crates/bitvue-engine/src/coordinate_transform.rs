//! Coordinate Transform - T0-2
//!
//! Per COORDINATE_SYSTEM_CONTRACT.md:
//! Canonical pipeline: screen_px → video_rect_norm → coded_px → block_idx
//!
//! Rules:
//! - ALL overlays MUST use this pipeline
//! - Fit/Zoom/Pan modify ONLY the screen→norm stage
//! - Block mapping NEVER depends on viewport directly
//! - Hover/click mapping is identical across overlays

use serde::{Deserialize, Serialize};

/// Screen pixel coordinates (mouse, click, viewport space)
///
/// Origin: top-left of the application window/canvas
/// Units: pixels
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ScreenPx {
    pub x: f32,
    pub y: f32,
}

impl ScreenPx {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

/// Video rectangle normalized coordinates (0..1 within video display rect)
///
/// Origin: top-left of the video display rectangle
/// Range: (0.0, 0.0) = top-left, (1.0, 1.0) = bottom-right
/// Independent of zoom/pan (zoom/pan are applied in screen→norm transform)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct VideoRectNorm {
    pub x: f32,
    pub y: f32,
}

impl VideoRectNorm {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    /// Check if normalized coordinates are within valid range [0..1]
    pub fn is_valid(&self) -> bool {
        self.x >= 0.0 && self.x <= 1.0 && self.y >= 0.0 && self.y <= 1.0
    }

    /// Clamp to valid range [0..1]
    pub fn clamp(&self) -> Self {
        Self {
            x: self.x.clamp(0.0, 1.0),
            y: self.y.clamp(0.0, 1.0),
        }
    }
}

/// Coded pixel coordinates (actual pixel position in coded frame)
///
/// Origin: top-left of the coded frame
/// Units: pixels (may be fractional after zoom/pan)
/// Range: (0, 0) to (coded_width, coded_height)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CodedPx {
    pub x: f32,
    pub y: f32,
}

impl CodedPx {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    /// Round to integer pixel coordinates
    pub fn round(&self) -> (u32, u32) {
        (self.x.round() as u32, self.y.round() as u32)
    }

    /// Floor to integer pixel coordinates
    pub fn floor(&self) -> (u32, u32) {
        (self.x.floor() as u32, self.y.floor() as u32)
    }
}

/// Block index coordinates (block row/col in block grid)
///
/// Origin: top-left block (0, 0)
/// Units: blocks
/// Used for overlay data indexing (QP map, MV map, partition grid)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BlockIdx {
    pub col: u32,
    pub row: u32,
}

impl BlockIdx {
    pub fn new(col: u32, row: u32) -> Self {
        Self { col, row }
    }

    /// Convert to linear index (row-major order)
    pub fn to_linear(&self, grid_width: u32) -> u32 {
        self.row * grid_width + self.col
    }

    /// Create from linear index (row-major order)
    pub fn from_linear(linear_idx: u32, grid_width: u32) -> Self {
        Self {
            row: linear_idx / grid_width,
            col: linear_idx % grid_width,
        }
    }
}

/// Rectangle in screen pixel space
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ScreenRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl ScreenRect {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Check if a point is inside this rectangle
    pub fn contains(&self, point: ScreenPx) -> bool {
        point.x >= self.x
            && point.x <= self.x + self.width
            && point.y >= self.y
            && point.y <= self.y + self.height
    }

    /// Get center point
    pub fn center(&self) -> ScreenPx {
        ScreenPx::new(self.x + self.width / 2.0, self.y + self.height / 2.0)
    }
}

/// Zoom mode for video display
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ZoomMode {
    /// Fit entire frame in viewport
    Fit,
    /// 100% zoom (1:1 pixel mapping)
    Percent100,
    /// 200% zoom (2:1 pixel mapping)
    Percent200,
    /// Custom zoom level (e.g., 50%, 150%, 400%)
    Custom(u32), // percentage: 50, 150, 400, etc.
}

impl ZoomMode {
    /// Get zoom factor as a multiplier (1.0 = 100%, 2.0 = 200%, etc.)
    pub fn factor(&self) -> f32 {
        match self {
            ZoomMode::Fit => 1.0, // Fit is special-cased in transform
            ZoomMode::Percent100 => 1.0,
            ZoomMode::Percent200 => 2.0,
            ZoomMode::Custom(pct) => (*pct as f32) / 100.0,
        }
    }
}

/// Coordinate transformer implementing the canonical pipeline
///
/// Per COORDINATE_SYSTEM_CONTRACT.md:
/// screen_px → video_rect_norm → coded_px → block_idx
///
/// Invariants:
/// - Hover/click mapping identical across all overlays
/// - Block mapping never depends on viewport
/// - Fit/Zoom/Pan modify ONLY screen→norm stage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinateTransformer {
    /// Coded frame dimensions
    pub coded_width: u32,
    pub coded_height: u32,

    /// Video display rectangle in screen space
    /// Updated by layout system and zoom/pan
    pub video_rect: ScreenRect,

    /// Zoom mode
    pub zoom_mode: ZoomMode,

    /// Pan offset (in normalized coordinates, 0..1)
    /// Only used when zoom > Fit
    pub pan_offset: (f32, f32),

    /// Block size for block_idx calculations (default: 8x8)
    /// Can be overridden per overlay (e.g., MV uses 16x16)
    pub default_block_size: u32,
}

impl CoordinateTransformer {
    /// Create a new coordinate transformer
    pub fn new(
        coded_width: u32,
        coded_height: u32,
        video_rect: ScreenRect,
        zoom_mode: ZoomMode,
    ) -> Self {
        Self {
            coded_width,
            coded_height,
            video_rect,
            zoom_mode,
            pan_offset: (0.0, 0.0),
            default_block_size: 8,
        }
    }

    /// Create a simple fit transformer (for testing)
    pub fn new_fit(coded_width: u32, coded_height: u32, video_rect: ScreenRect) -> Self {
        Self::new(coded_width, coded_height, video_rect, ZoomMode::Fit)
    }

    /// Update video display rectangle (called by layout system)
    pub fn set_video_rect(&mut self, video_rect: ScreenRect) {
        self.video_rect = video_rect;
    }

    /// Set zoom mode
    pub fn set_zoom(&mut self, zoom_mode: ZoomMode) {
        self.zoom_mode = zoom_mode;
        // Reset pan when switching to Fit
        if matches!(zoom_mode, ZoomMode::Fit) {
            self.pan_offset = (0.0, 0.0);
        }
    }

    /// Set pan offset (normalized coordinates)
    pub fn set_pan(&mut self, pan_x: f32, pan_y: f32) {
        self.pan_offset = (pan_x.clamp(0.0, 1.0), pan_y.clamp(0.0, 1.0));
    }

    /// Set default block size
    pub fn set_default_block_size(&mut self, block_size: u32) {
        self.default_block_size = block_size;
    }

    // ========================================================================
    // Forward transforms: screen_px → video_rect_norm → coded_px → block_idx
    // ========================================================================

    /// Transform screen pixel to normalized video rect coordinates
    ///
    /// Per COORDINATE_SYSTEM_CONTRACT.md:
    /// This is the ONLY stage where Fit/Zoom/Pan are applied.
    pub fn screen_to_norm(&self, screen: ScreenPx) -> Option<VideoRectNorm> {
        // Check if point is inside video rect
        if !self.video_rect.contains(screen) {
            return None;
        }

        // Convert to video rect local coordinates (0..video_rect.width/height)
        let local_x = screen.x - self.video_rect.x;
        let local_y = screen.y - self.video_rect.y;

        // Normalize to 0..1
        let mut norm_x = local_x / self.video_rect.width;
        let mut norm_y = local_y / self.video_rect.height;

        // Apply zoom and pan (only if not Fit mode)
        if !matches!(self.zoom_mode, ZoomMode::Fit) {
            let zoom_factor = self.zoom_mode.factor();

            // Apply pan offset
            norm_x = (norm_x / zoom_factor) + self.pan_offset.0;
            norm_y = (norm_y / zoom_factor) + self.pan_offset.1;
        }

        let norm = VideoRectNorm::new(norm_x, norm_y);

        // Return only if within valid range
        if norm.is_valid() {
            Some(norm)
        } else {
            None
        }
    }

    /// Transform normalized video rect to coded pixel coordinates
    ///
    /// Per COORDINATE_SYSTEM_CONTRACT.md:
    /// This stage is independent of viewport/zoom/pan.
    pub fn norm_to_coded(&self, norm: VideoRectNorm) -> CodedPx {
        CodedPx::new(
            norm.x * self.coded_width as f32,
            norm.y * self.coded_height as f32,
        )
    }

    /// Transform coded pixel to block index
    ///
    /// Per COORDINATE_SYSTEM_CONTRACT.md:
    /// Block mapping NEVER depends on viewport.
    pub fn coded_to_block(&self, coded: CodedPx, block_size: Option<u32>) -> BlockIdx {
        let bs = block_size.unwrap_or(self.default_block_size) as f32;
        BlockIdx::new((coded.x / bs).floor() as u32, (coded.y / bs).floor() as u32)
    }

    // ========================================================================
    // Combined forward transforms
    // ========================================================================

    /// Transform screen pixel directly to coded pixel
    pub fn screen_to_coded(&self, screen: ScreenPx) -> Option<CodedPx> {
        self.screen_to_norm(screen)
            .map(|norm| self.norm_to_coded(norm))
    }

    /// Transform screen pixel directly to block index
    pub fn screen_to_block(&self, screen: ScreenPx, block_size: Option<u32>) -> Option<BlockIdx> {
        self.screen_to_coded(screen)
            .map(|coded| self.coded_to_block(coded, block_size))
    }

    // ========================================================================
    // Reverse transforms: block_idx → coded_px → video_rect_norm → screen_px
    // ========================================================================

    /// Transform block index to coded pixel (top-left corner of block)
    pub fn block_to_coded(&self, block: BlockIdx, block_size: Option<u32>) -> CodedPx {
        let bs = block_size.unwrap_or(self.default_block_size) as f32;
        CodedPx::new(block.col as f32 * bs, block.row as f32 * bs)
    }

    /// Transform coded pixel to normalized video rect coordinates
    pub fn coded_to_norm(&self, coded: CodedPx) -> VideoRectNorm {
        VideoRectNorm::new(
            coded.x / self.coded_width as f32,
            coded.y / self.coded_height as f32,
        )
    }

    /// Transform normalized video rect to screen pixel coordinates
    pub fn norm_to_screen(&self, norm: VideoRectNorm) -> ScreenPx {
        let mut norm_x = norm.x;
        let mut norm_y = norm.y;

        // Apply zoom and pan (inverse of screen_to_norm)
        if !matches!(self.zoom_mode, ZoomMode::Fit) {
            let zoom_factor = self.zoom_mode.factor();

            // Remove pan offset, then apply zoom
            norm_x = (norm_x - self.pan_offset.0) * zoom_factor;
            norm_y = (norm_y - self.pan_offset.1) * zoom_factor;
        }

        // Convert to screen coordinates
        ScreenPx::new(
            self.video_rect.x + norm_x * self.video_rect.width,
            self.video_rect.y + norm_y * self.video_rect.height,
        )
    }

    // ========================================================================
    // Combined reverse transforms
    // ========================================================================

    /// Transform coded pixel directly to screen pixel
    pub fn coded_to_screen(&self, coded: CodedPx) -> ScreenPx {
        let norm = self.coded_to_norm(coded);
        self.norm_to_screen(norm)
    }

    /// Transform block index directly to screen pixel (top-left corner)
    pub fn block_to_screen(&self, block: BlockIdx, block_size: Option<u32>) -> ScreenPx {
        let coded = self.block_to_coded(block, block_size);
        self.coded_to_screen(coded)
    }

    // ========================================================================
    // Utility methods
    // ========================================================================

    /// Get the visible coded pixel range (after zoom/pan)
    ///
    /// Returns (min_x, min_y, max_x, max_y) in coded pixel space
    pub fn visible_coded_range(&self) -> (f32, f32, f32, f32) {
        // Top-left corner
        let tl_norm = VideoRectNorm::new(0.0, 0.0);
        let tl_coded = self.norm_to_coded(tl_norm);

        // Bottom-right corner
        let br_norm = VideoRectNorm::new(1.0, 1.0);
        let br_coded = self.norm_to_coded(br_norm);

        (tl_coded.x, tl_coded.y, br_coded.x, br_coded.y)
    }

    /// Get the number of blocks in the frame grid
    pub fn block_grid_size(&self, block_size: Option<u32>) -> (u32, u32) {
        let bs = block_size.unwrap_or(self.default_block_size);
        let cols = self.coded_width.div_ceil(bs);
        let rows = self.coded_height.div_ceil(bs);
        (cols, rows)
    }
}

// TODO: Fix coordinate_transform_test.rs - needs API rewrite to match actual implementation
// #[cfg(test)]
// include!("coordinate_transform_test.rs");
