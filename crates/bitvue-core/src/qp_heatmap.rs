//! QP Heatmap Overlay - T3-1
//!
//! Per QP_HEATMAP_IMPLEMENTATION_SPEC.md:
//! - Texture-based QP heatmap rendering
//! - Coverage handling (missing values → transparent)
//! - Scale handling (auto vs fixed range)
//! - Cache key with invalidation triggers
//!
//! Per CACHE_INVALIDATION_TABLE.md:
//! Invalidate when: frame_idx, hm_res, scale_mode, qp_min/max, opacity

use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// QP Grid - Canonical representation (codec-agnostic)
///
/// Per QP_HEATMAP_IMPLEMENTATION_SPEC.md §1:
/// Dense 2D grid of leaf blocks with QP values.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QPGrid {
    /// Grid width (number of blocks horizontally)
    pub grid_w: u32,

    /// Grid height (number of blocks vertically)
    pub grid_h: u32,

    /// Block width in pixels (e.g., 8 for AV1, 16 for H.264)
    pub block_w: u32,

    /// Block height in pixels
    pub block_h: u32,

    /// QP values (row-major: grid_w * grid_h)
    /// -1 (or missing value) means "no data" → render transparent
    pub qp: Vec<i16>,

    /// Minimum QP value in the data
    pub qp_min: i16,

    /// Maximum QP value in the data
    pub qp_max: i16,

    /// Missing value marker (default: -1)
    pub missing: i16,
}

impl QPGrid {
    /// Create a new QP grid
    pub fn new(
        grid_w: u32,
        grid_h: u32,
        block_w: u32,
        block_h: u32,
        qp: Vec<i16>,
        missing: i16,
    ) -> Self {
        // Calculate qp_min and qp_max in a single pass (excluding missing values)
        let (qp_min, qp_max) = qp
            .iter()
            .filter(|&&v| v != missing)
            .fold(None, |acc, &v| match acc {
                None => Some((v, v)),
                Some((min, max)) => Some((min.min(v), max.max(v))),
            })
            .unwrap_or((0, 63)); // Default range if no valid QPs

        Self {
            grid_w,
            grid_h,
            block_w,
            block_h,
            qp,
            qp_min,
            qp_max,
            missing,
        }
    }

    /// Get QP value at block coordinates
    pub fn get(&self, bx: u32, by: u32) -> Option<i16> {
        if bx >= self.grid_w || by >= self.grid_h {
            return None;
        }

        let idx = (by * self.grid_w + bx) as usize;
        let qp_val = self.qp.get(idx).copied()?;

        if qp_val == self.missing {
            None
        } else {
            Some(qp_val)
        }
    }

    /// Get coded dimensions (grid_w * block_w, grid_h * block_h)
    pub fn coded_dimensions(&self) -> (u32, u32) {
        (self.grid_w * self.block_w, self.grid_h * self.block_h)
    }

    /// Calculate coverage percentage (non-missing blocks / total blocks)
    pub fn coverage_percent(&self) -> f32 {
        let total = self.qp.len();
        if total == 0 {
            return 0.0;
        }

        let valid = self.qp.iter().filter(|&&v| v != self.missing).count();
        (valid as f32 / total as f32) * 100.0
    }
}

/// QP scale mode
///
/// Per QP_HEATMAP_IMPLEMENTATION_SPEC.md §2.3:
/// - Auto: Fit qp_min/qp_max per frame (dynamic range)
/// - Fixed: Use 0..63 range (consistent across frames)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum QPScaleMode {
    Auto,
    Fixed,
}

/// Heatmap resolution mode
///
/// Per QP_HEATMAP_IMPLEMENTATION_SPEC.md §2.2:
/// - Quarter: 1/4 of coded resolution
/// - Half: 1/2 of coded resolution (default)
/// - Full: Same as coded resolution
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HeatmapResolution {
    Quarter,
    Half,
    Full,
}

impl HeatmapResolution {
    /// Get scale factor
    pub fn scale(&self) -> u32 {
        match self {
            HeatmapResolution::Quarter => 4,
            HeatmapResolution::Half => 2,
            HeatmapResolution::Full => 1,
        }
    }
}

/// RGBA8 color
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    /// Linear interpolation between two colors
    pub fn lerp(a: Color, b: Color, t: f32) -> Color {
        let t = t.clamp(0.0, 1.0);
        Color {
            r: (a.r as f32 + (b.r as f32 - a.r as f32) * t) as u8,
            g: (a.g as f32 + (b.g as f32 - a.g as f32) * t) as u8,
            b: (a.b as f32 + (b.b as f32 - a.b as f32) * t) as u8,
            a: (a.a as f32 + (b.a as f32 - a.a as f32) * t) as u8,
        }
    }
}

/// QP color mapper
///
/// Per QP_HEATMAP_IMPLEMENTATION_SPEC.md §2.3:
/// 4-stop color ramp with deterministic colors
pub struct QPColorMapper {
    /// Base alpha for normal blocks (160 ≈ 0.63)
    base_alpha: u8,

    /// User opacity multiplier (0.0..1.0)
    user_opacity: f32,

    /// Color stops (t, color)
    stops: [(f32, Color); 4],
}

impl QPColorMapper {
    /// Create a new color mapper
    ///
    /// Per QP_HEATMAP_IMPLEMENTATION_SPEC.md §2.3:
    /// - 0.00 → (  0,  70, 255) - Blue
    /// - 0.35 → (  0, 200, 180) - Cyan
    /// - 0.65 → (255, 190,   0) - Yellow
    /// - 1.00 → (255,  40,  40) - Red
    pub fn new(user_opacity: f32) -> Self {
        Self {
            base_alpha: 160,
            user_opacity: user_opacity.clamp(0.0, 1.0),
            stops: [
                (0.00, Color::rgb(0, 70, 255)),
                (0.35, Color::rgb(0, 200, 180)),
                (0.65, Color::rgb(255, 190, 0)),
                (1.00, Color::rgb(255, 40, 40)),
            ],
        }
    }

    /// Map QP value to color
    ///
    /// qp: QP value (None for missing)
    /// qp_min, qp_max: Range for normalization
    pub fn map_qp(&self, qp: Option<i16>, qp_min: i16, qp_max: i16) -> Color {
        // Missing value → transparent
        if qp.is_none() {
            return Color::new(0, 0, 0, 0);
        }

        let qp_val = qp.unwrap();

        // Normalize to 0..1
        let t = if qp_max > qp_min {
            ((qp_val - qp_min) as f32 / (qp_max - qp_min) as f32).clamp(0.0, 1.0)
        } else {
            0.5 // Single value case
        };

        // Find color stops to interpolate between
        let mut color = self.stops[0].1;

        for i in 0..self.stops.len() - 1 {
            let (t0, c0) = self.stops[i];
            let (t1, c1) = self.stops[i + 1];

            if t >= t0 && t <= t1 {
                // Interpolate between c0 and c1
                let local_t = (t - t0) / (t1 - t0);
                color = Color::lerp(c0, c1, local_t);
                break;
            }
        }

        // Apply alpha
        let final_alpha = (self.base_alpha as f32 * self.user_opacity) as u8;
        Color::new(color.r, color.g, color.b, final_alpha)
    }

    /// Get opacity bucket for caching
    ///
    /// Per QP_HEATMAP_IMPLEMENTATION_SPEC.md §2.4:
    /// Round to 0.05 steps to increase cache reuse
    pub fn opacity_bucket(opacity: f32) -> u8 {
        (opacity * 20.0).round() as u8
    }
}

/// Heatmap texture data (RGBA8 image buffer)
#[derive(Debug, Clone)]
pub struct HeatmapTexture {
    /// Texture width in pixels
    pub width: u32,

    /// Texture height in pixels
    pub height: u32,

    /// RGBA8 pixel data (width * height * 4 bytes)
    pub pixels: Vec<u8>,
}

impl HeatmapTexture {
    /// Generate heatmap texture from QP grid
    ///
    /// Per QP_HEATMAP_IMPLEMENTATION_SPEC.md §2.2:
    /// - Choose heatmap resolution based on coded dimensions
    /// - Map each heatmap pixel to block index
    /// - Convert QP to color
    pub fn generate(
        grid: &QPGrid,
        resolution: HeatmapResolution,
        scale_mode: QPScaleMode,
        opacity: f32,
    ) -> Self {
        let (coded_width, coded_height) = grid.coded_dimensions();

        // Calculate heatmap dimensions
        let hm_scale = resolution.scale();
        let hm_w = (coded_width / hm_scale).clamp(1, 1024);
        let hm_h = ((hm_w as f32 * coded_height as f32) / coded_width as f32).round() as u32;

        // Scaling factors
        let sx = coded_width as f32 / hm_w as f32;
        let sy = coded_height as f32 / hm_h as f32;

        // Color mapper
        let mapper = QPColorMapper::new(opacity);

        // Determine QP range for normalization
        let (qp_min, qp_max) = match scale_mode {
            QPScaleMode::Auto => (grid.qp_min, grid.qp_max),
            QPScaleMode::Fixed => (0, 63),
        };

        // Generate pixels
        let mut pixels = Vec::with_capacity((hm_w * hm_h * 4) as usize);

        for y in 0..hm_h {
            for x in 0..hm_w {
                // Map heatmap pixel to coded pixel
                let coded_x = (x as f32 * sx) as u32;
                let coded_y = (y as f32 * sy) as u32;

                // Map coded pixel to block index
                let bx = coded_x / grid.block_w;
                let by = coded_y / grid.block_h;

                // Get QP value
                let qp = grid.get(bx, by);

                // Map to color
                let color = mapper.map_qp(qp, qp_min, qp_max);

                // Write RGBA
                pixels.push(color.r);
                pixels.push(color.g);
                pixels.push(color.b);
                pixels.push(color.a);
            }
        }

        Self {
            width: hm_w,
            height: hm_h,
            pixels,
        }
    }
}

/// Parameters for creating a QP heatmap cache key
#[derive(Debug, Clone)]
pub struct QPCacheKeyParams<'a> {
    pub stream: crate::StreamId,
    pub frame_idx: usize,
    pub resolution: HeatmapResolution,
    pub scale_mode: QPScaleMode,
    pub qp_min: i16,
    pub qp_max: i16,
    pub opacity: f32,
    pub codec: &'a str,
    pub file_path: &'a str,
}

/// QP heatmap cache key
///
/// Per QP_HEATMAP_IMPLEMENTATION_SPEC.md §2.4 and CACHE_INVALIDATION_TABLE.md:
/// Invalidate when: frame_idx, hm_res, scale_mode, qp_min/max, opacity
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct QPHeatmapCacheKey {
    /// Stream ID (A or B)
    pub stream: crate::StreamId,

    /// Frame index (display order)
    pub frame_idx: usize,

    /// Heatmap resolution mode
    pub resolution: HeatmapResolution,

    /// Scale mode (auto vs fixed)
    pub scale_mode: QPScaleMode,

    /// QP min (for auto mode)
    pub qp_min: i16,

    /// QP max (for auto mode)
    pub qp_max: i16,

    /// Opacity bucket (rounded to 0.05 steps)
    pub opacity_bucket: u8,

    /// Codec (for cache key uniqueness)
    pub codec: String,

    /// File hash (for cache key uniqueness)
    pub file_hash: u64,
}

impl QPHeatmapCacheKey {
    /// Create a new cache key
    pub fn new(params: &QPCacheKeyParams<'_>) -> Self {
        // Hash file path for uniqueness
        let mut hasher = DefaultHasher::new();
        params.file_path.hash(&mut hasher);
        let file_hash = hasher.finish();

        Self {
            stream: params.stream,
            frame_idx: params.frame_idx,
            resolution: params.resolution,
            scale_mode: params.scale_mode,
            qp_min: params.qp_min,
            qp_max: params.qp_max,
            opacity_bucket: QPColorMapper::opacity_bucket(params.opacity),
            codec: params.codec.to_string(),
            file_hash,
        }
    }

    /// Generate cache key string
    ///
    /// Format per spec:
    /// `overlay_qp_heat:<stream>:<codec>:<filehash>:f<frame_idx>|hm<hm_w>x<hm_h>|scale<auto|fixed>|op<opacity_bucket>`
    pub fn to_string(&self, hm_w: u32, hm_h: u32) -> String {
        let stream_str = match self.stream {
            crate::StreamId::A => "A",
            crate::StreamId::B => "B",
        };

        let scale_str = match self.scale_mode {
            QPScaleMode::Auto => "auto",
            QPScaleMode::Fixed => "fixed",
        };

        format!(
            "overlay_qp_heat:{}:{}:{:016x}:f{}|hm{}x{}|scale{}|op{}",
            stream_str,
            self.codec,
            self.file_hash,
            self.frame_idx,
            hm_w,
            hm_h,
            scale_str,
            self.opacity_bucket
        )
    }
}

/// QPHeatmapOverlay - Main overlay component
///
/// Per T3-1 deliverable: QPHeatmapOverlay
#[derive(Debug, Clone)]
pub struct QPHeatmapOverlay {
    /// QP grid data
    pub grid: QPGrid,

    /// Heatmap resolution mode
    pub resolution: HeatmapResolution,

    /// Scale mode (auto vs fixed)
    pub scale_mode: QPScaleMode,

    /// User opacity (0.0..1.0, default 0.45)
    pub opacity: f32,

    /// Cached texture (if any)
    pub cached_texture: Option<HeatmapTexture>,

    /// Cache key for current texture
    pub cache_key: Option<QPHeatmapCacheKey>,
}

impl QPHeatmapOverlay {
    /// Create a new QP heatmap overlay
    pub fn new(grid: QPGrid) -> Self {
        Self {
            grid,
            resolution: HeatmapResolution::Half,
            scale_mode: QPScaleMode::Auto,
            opacity: 0.45,
            cached_texture: None,
            cache_key: None,
        }
    }

    /// Generate or retrieve cached texture
    pub fn get_texture(&mut self, cache_key: QPHeatmapCacheKey) -> &HeatmapTexture {
        // Check if cache is valid
        let needs_rebuild = self.cache_key.as_ref() != Some(&cache_key);

        if needs_rebuild {
            // Generate new texture
            let texture = HeatmapTexture::generate(
                &self.grid,
                self.resolution,
                self.scale_mode,
                self.opacity,
            );

            self.cached_texture = Some(texture);
            self.cache_key = Some(cache_key);
        }

        self.cached_texture.as_ref().unwrap()
    }

    /// Check if coverage is sufficient
    ///
    /// Per EDGE_CASES_AND_DEGRADE_BEHAVIOR.md §C:
    /// <20% coverage → auto-disable overlay with reason
    pub fn has_sufficient_coverage(&self) -> bool {
        self.grid.coverage_percent() >= 20.0
    }

    /// Get coverage percentage
    pub fn coverage_percent(&self) -> f32 {
        self.grid.coverage_percent()
    }

    /// Set resolution mode
    pub fn set_resolution(&mut self, resolution: HeatmapResolution) {
        if self.resolution != resolution {
            self.resolution = resolution;
            self.cache_key = None; // Invalidate cache
        }
    }

    /// Set scale mode
    pub fn set_scale_mode(&mut self, scale_mode: QPScaleMode) {
        if self.scale_mode != scale_mode {
            self.scale_mode = scale_mode;
            self.cache_key = None; // Invalidate cache
        }
    }

    /// Set opacity
    pub fn set_opacity(&mut self, opacity: f32) {
        let new_opacity = opacity.clamp(0.0, 1.0);
        let old_bucket = QPColorMapper::opacity_bucket(self.opacity);
        let new_bucket = QPColorMapper::opacity_bucket(new_opacity);

        if old_bucket != new_bucket {
            self.opacity = new_opacity;
            self.cache_key = None; // Invalidate cache
        } else {
            // Same bucket, no cache invalidation needed
            self.opacity = new_opacity;
        }
    }
}

/// Per generate-tests skill: Comprehensive test suite with Arrange-Act-Assert pattern
#[cfg(test)]
mod tests {
    include!("qp_heatmap_test.rs");
}
