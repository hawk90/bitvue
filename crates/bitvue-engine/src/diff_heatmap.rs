//! Diff Heatmap Overlay - T3-4
//!
//! Per DIFF_HEATMAP_IMPLEMENTATION_SPEC.md:
//! - A/B diff heatmap with alignment policy
//! - Modes: abs, signed, metric
//! - Half-res default (same as QP heatmap)
//! - 4-stop ramp, 0 is transparent
//! - Alpha increases with diff, max alpha 180 * user_opacity
//! - Cache key: `overlay_diff:<codec>:<fileA>:<fileB>:f<frame>|hm<w>x<h>|mode<abs|signed|metric>|op<bucket>`
//! - Hover shows diff at pixel and block
//!
//! Per COMPARE_ALIGNMENT_POLICY.md:
//! - Disable with explanation on resolution mismatch
//!
//! Per EDGE_CASES_AND_DEGRADE_BEHAVIOR.md:
//! - Clear messaging when disabled

use crate::{CoordinateTransformer, ScreenRect, WorldBounds, ZoomBounds};
use serde::{Deserialize, Serialize};

/// Diff heatmap mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum DiffMode {
    /// Absolute difference: abs(lumaA - lumaB)
    #[default]
    Abs,
    /// Signed difference: lumaA - lumaB
    Signed,
    /// Per-block metric delta (if available)
    Metric,
}

impl DiffMode {
    /// Get display name
    pub fn display_name(&self) -> &'static str {
        match self {
            DiffMode::Abs => "Absolute",
            DiffMode::Signed => "Signed",
            DiffMode::Metric => "Metric",
        }
    }

    /// Get mode key for cache
    pub fn cache_key(&self) -> &'static str {
        match self {
            DiffMode::Abs => "abs",
            DiffMode::Signed => "signed",
            DiffMode::Metric => "metric",
        }
    }
}

/// Parameters for generating diff cache key
#[derive(Debug, Clone)]
pub struct DiffCacheKeyParams<'a> {
    pub codec: &'a str,
    pub file_hash_a: &'a str,
    pub file_hash_b: &'a str,
    pub frame_idx: usize,
    pub heatmap_width: u32,
    pub heatmap_height: u32,
    pub mode: DiffMode,
    pub opacity_bucket: u32,
}

/// Diff heatmap data
///
/// Per DIFF_HEATMAP_IMPLEMENTATION_SPEC.md:
/// - Half-res by default
/// - Values represent luma difference or metric delta
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffHeatmapData {
    /// Original frame width
    pub frame_width: u32,

    /// Original frame height
    pub frame_height: u32,

    /// Heatmap width (half-res)
    pub heatmap_width: u32,

    /// Heatmap height (half-res)
    pub heatmap_height: u32,

    /// Diff values (row-major)
    pub values: Vec<f32>,

    /// Mode used
    pub mode: DiffMode,

    /// Min diff value (for normalization)
    pub min_value: f32,

    /// Max diff value (for normalization)
    pub max_value: f32,
}

impl DiffHeatmapData {
    /// Create diff heatmap from luma planes
    ///
    /// Per spec: abs(lumaA - lumaB) or signed mode
    pub fn from_luma_planes(
        luma_a: &[u8],
        luma_b: &[u8],
        width: u32,
        height: u32,
        mode: DiffMode,
    ) -> Self {
        assert_eq!(luma_a.len(), (width * height) as usize);
        assert_eq!(luma_b.len(), (width * height) as usize);

        // Half-res heatmap
        let hm_width = width.div_ceil(2);
        let hm_height = height.div_ceil(2);
        let mut values = Vec::with_capacity((hm_width * hm_height) as usize);

        let mut min_value = f32::MAX;
        let mut max_value = f32::MIN;

        // Downsample by 2x2 averaging
        for hm_y in 0..hm_height {
            for hm_x in 0..hm_width {
                let src_x = hm_x * 2;
                let src_y = hm_y * 2;

                let mut diff_sum = 0.0;
                let mut count = 0;

                // Average 2x2 block
                for dy in 0..2 {
                    for dx in 0..2 {
                        let x = src_x + dx;
                        let y = src_y + dy;

                        if x < width && y < height {
                            let idx = (y * width + x) as usize;
                            let a = luma_a[idx] as f32;
                            let b = luma_b[idx] as f32;

                            let diff = match mode {
                                DiffMode::Abs => (a - b).abs(),
                                DiffMode::Signed => a - b,
                                DiffMode::Metric => a - b, // Fallback to signed if no metric
                            };

                            diff_sum += diff;
                            count += 1;
                        }
                    }
                }

                let avg_diff = if count > 0 {
                    diff_sum / count as f32
                } else {
                    0.0
                };
                values.push(avg_diff);

                min_value = min_value.min(avg_diff);
                max_value = max_value.max(avg_diff);
            }
        }

        Self {
            frame_width: width,
            frame_height: height,
            heatmap_width: hm_width,
            heatmap_height: hm_height,
            values,
            mode,
            min_value,
            max_value,
        }
    }

    /// Get diff value at heatmap coordinates
    pub fn get_value(&self, hm_x: u32, hm_y: u32) -> Option<f32> {
        if hm_x >= self.heatmap_width || hm_y >= self.heatmap_height {
            return None;
        }

        let idx = (hm_y * self.heatmap_width + hm_x) as usize;
        self.values.get(idx).copied()
    }

    /// Get normalized value (0.0 to 1.0)
    pub fn get_normalized(&self, hm_x: u32, hm_y: u32) -> Option<f32> {
        let value = self.get_value(hm_x, hm_y)?;

        let range = self.max_value - self.min_value;
        if range <= 0.001 {
            // All values are the same
            // If value is non-zero, treat as max (1.0)
            // If value is zero, treat as min (0.0)
            return Some(if value.abs() > 0.001 { 1.0 } else { 0.0 });
        }

        let normalized = (value - self.min_value) / range;
        Some(normalized.clamp(0.0, 1.0))
    }

    /// Get alpha value for rendering
    ///
    /// Per spec: alpha increases with diff, max alpha 180 * user_opacity
    /// 0 is transparent
    pub fn get_alpha(&self, hm_x: u32, hm_y: u32, user_opacity: f32) -> u8 {
        let normalized = match self.get_normalized(hm_x, hm_y) {
            Some(n) => n,
            None => return 0,
        };

        // 0 diff is transparent
        if normalized < 0.001 {
            return 0;
        }

        // 4-stop ramp: 0.0->0, 0.25->45, 0.5->90, 0.75->135, 1.0->180
        let base_alpha = (normalized * 180.0).min(180.0);
        let final_alpha = base_alpha * user_opacity.clamp(0.0, 1.0);

        final_alpha as u8
    }

    /// Generate cache key
    ///
    /// Format: `overlay_diff:<codec>:<fileA>:<fileB>:f<frame>|hm<w>x<h>|mode<abs|signed|metric>|op<bucket>`
    pub fn cache_key(params: &DiffCacheKeyParams<'_>) -> String {
        format!(
            "overlay_diff:{}:{}:{}:f{}|hm{}x{}|mode{}|op{}",
            params.codec,
            params.file_hash_a,
            params.file_hash_b,
            params.frame_idx,
            params.heatmap_width,
            params.heatmap_height,
            params.mode.cache_key(),
            params.opacity_bucket
        )
    }
}

/// Diff heatmap overlay
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffHeatmapOverlay {
    /// Current diff mode
    pub mode: DiffMode,

    /// User opacity (0.0 to 1.0)
    pub user_opacity: f32,

    /// Enable/disable overlay
    pub enabled: bool,

    /// Disable reason (if disabled)
    pub disable_reason: Option<String>,

    /// Coordinate transform
    pub transform: CoordinateTransformer,

    /// World bounds
    pub world_bounds: WorldBounds,

    /// Zoom bounds
    pub zoom_bounds: ZoomBounds,

    /// Experimental resample toggle
    pub experimental_resample: bool,

    /// Hover info (pixel position and diff value)
    pub hover_info: Option<DiffHoverInfo>,

    /// Frozen tooltip (click to freeze)
    pub frozen_tooltip: Option<DiffHoverInfo>,
}

impl DiffHeatmapOverlay {
    /// Create new diff heatmap overlay
    pub fn new(frame_width: u32, frame_height: u32) -> Self {
        let transform = CoordinateTransformer::new_fit(
            frame_width,
            frame_height,
            ScreenRect::new(0.0, 0.0, frame_width as f32, frame_height as f32),
        );

        Self {
            mode: DiffMode::default(),
            user_opacity: 0.7,
            enabled: true,
            disable_reason: None,
            transform,
            world_bounds: WorldBounds::new(0.0, 0.0, frame_width as f32, frame_height as f32),
            zoom_bounds: ZoomBounds::default(),
            experimental_resample: false,
            hover_info: None,
            frozen_tooltip: None,
        }
    }

    /// Set diff mode
    pub fn set_mode(&mut self, mode: DiffMode) {
        self.mode = mode;
    }

    /// Set user opacity
    pub fn set_opacity(&mut self, opacity: f32) {
        self.user_opacity = opacity.clamp(0.0, 1.0);
    }

    /// Enable overlay
    pub fn enable(&mut self) {
        self.enabled = true;
        self.disable_reason = None;
    }

    /// Disable overlay with reason
    pub fn disable(&mut self, reason: String) {
        self.enabled = false;
        self.disable_reason = Some(reason);
    }

    /// Check if overlay is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Get disable reason
    pub fn disable_reason(&self) -> Option<&str> {
        self.disable_reason.as_deref()
    }

    /// Set hover info
    pub fn set_hover(&mut self, pixel_x: u32, pixel_y: u32, diff_value: f32) {
        self.hover_info = Some(DiffHoverInfo {
            pixel_x,
            pixel_y,
            diff_value,
        });
    }

    /// Clear hover info
    pub fn clear_hover(&mut self) {
        self.hover_info = None;
    }

    /// Freeze tooltip at current hover position
    pub fn freeze_tooltip(&mut self) {
        self.frozen_tooltip = self.hover_info.clone();
    }

    /// Unfreeze tooltip
    pub fn unfreeze_tooltip(&mut self) {
        self.frozen_tooltip = None;
    }

    /// Toggle experimental resample
    pub fn toggle_resample(&mut self) {
        self.experimental_resample = !self.experimental_resample;
    }

    /// Get opacity bucket for cache key (quantize to 10 levels)
    pub fn opacity_bucket(&self) -> u32 {
        (self.user_opacity * 10.0) as u32
    }
}

/// Resolution check result for A/B diff
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolutionCheckResult {
    /// Width A
    pub width_a: u32,
    /// Height A
    pub height_a: u32,
    /// Width B
    pub width_b: u32,
    /// Height B
    pub height_b: u32,
    /// Is compatible (resolution mismatch <= 5%)
    pub is_compatible: bool,
    /// Width mismatch percentage
    pub width_mismatch_pct: f32,
    /// Height mismatch percentage
    pub height_mismatch_pct: f32,
    /// Disable reason if not compatible
    pub disable_reason: Option<String>,
}

impl ResolutionCheckResult {
    /// Check resolution compatibility for diff overlay
    ///
    /// Per COMPARE_ALIGNMENT_POLICY.md:
    /// Resolution mismatch > 5% → disable diff overlays
    pub fn check(width_a: u32, height_a: u32, width_b: u32, height_b: u32) -> Self {
        let width_mismatch_pct = if width_a == 0 || width_b == 0 {
            100.0
        } else {
            ((width_a as f32 - width_b as f32).abs() / width_a.max(width_b) as f32) * 100.0
        };

        let height_mismatch_pct = if height_a == 0 || height_b == 0 {
            100.0
        } else {
            ((height_a as f32 - height_b as f32).abs() / height_a.max(height_b) as f32) * 100.0
        };

        let max_mismatch = width_mismatch_pct.max(height_mismatch_pct);
        let is_compatible = max_mismatch <= 5.0;

        let disable_reason = if !is_compatible {
            Some(format!(
                "Resolution mismatch: {}x{} vs {}x{} ({:.1}% difference)",
                width_a, height_a, width_b, height_b, max_mismatch
            ))
        } else {
            None
        };

        Self {
            width_a,
            height_a,
            width_b,
            height_b,
            is_compatible,
            width_mismatch_pct,
            height_mismatch_pct,
            disable_reason,
        }
    }

    /// Get formatted summary
    pub fn summary(&self) -> String {
        if self.is_compatible {
            format!(
                "Resolution compatible: {}x{} ↔ {}x{}",
                self.width_a, self.height_a, self.width_b, self.height_b
            )
        } else {
            self.disable_reason
                .clone()
                .unwrap_or_else(|| "Resolution mismatch".to_string())
        }
    }
}

/// Diff comparison context for A/B analysis
///
/// Per COMPETITOR_PARITY_STATUS.md §4.2:
/// Diff heatmap (A/B compare) - needs alignment integration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffCompareContext {
    /// Resolution check result
    pub resolution_check: ResolutionCheckResult,

    /// Current aligned frame index in A
    pub frame_a_idx: Option<usize>,

    /// Current aligned frame index in B
    pub frame_b_idx: Option<usize>,

    /// Alignment confidence (if available)
    pub alignment_confidence: Option<String>,

    /// Gap indicator (frame has no match)
    pub has_gap: bool,

    /// PTS delta between aligned frames
    pub pts_delta: Option<i64>,

    /// Diff statistics for current frame
    pub diff_stats: Option<DiffStatistics>,
}

impl DiffCompareContext {
    /// Create new context with resolution check
    pub fn new(width_a: u32, height_a: u32, width_b: u32, height_b: u32) -> Self {
        Self {
            resolution_check: ResolutionCheckResult::check(width_a, height_a, width_b, height_b),
            frame_a_idx: None,
            frame_b_idx: None,
            alignment_confidence: None,
            has_gap: false,
            pts_delta: None,
            diff_stats: None,
        }
    }

    /// Set aligned frame pair
    pub fn set_frame_pair(
        &mut self,
        a_idx: Option<usize>,
        b_idx: Option<usize>,
        pts_delta: Option<i64>,
        has_gap: bool,
    ) {
        self.frame_a_idx = a_idx;
        self.frame_b_idx = b_idx;
        self.pts_delta = pts_delta;
        self.has_gap = has_gap;
    }

    /// Set alignment confidence
    pub fn set_alignment_confidence(&mut self, confidence: &str) {
        self.alignment_confidence = Some(confidence.to_string());
    }

    /// Set diff statistics
    pub fn set_diff_stats(&mut self, stats: DiffStatistics) {
        self.diff_stats = Some(stats);
    }

    /// Check if diff is available
    pub fn is_diff_available(&self) -> bool {
        self.resolution_check.is_compatible
            && self.frame_a_idx.is_some()
            && self.frame_b_idx.is_some()
            && !self.has_gap
    }

    /// Get status text for UI
    pub fn status_text(&self) -> String {
        if !self.resolution_check.is_compatible {
            return self
                .resolution_check
                .disable_reason
                .clone()
                .unwrap_or_else(|| "Resolution mismatch".to_string());
        }

        if self.has_gap {
            return "Gap: No matching frame in other stream".to_string();
        }

        match (self.frame_a_idx, self.frame_b_idx) {
            (Some(a), Some(b)) => {
                let confidence = self.alignment_confidence.as_deref().unwrap_or("Unknown");
                let delta_str = self
                    .pts_delta
                    .map(|d| format!(", PTS Δ: {}", d))
                    .unwrap_or_default();
                format!("Frame A:{} ↔ B:{} ({}{})", a, b, confidence, delta_str)
            }
            (Some(a), None) => format!("Frame A:{} (no match in B)", a),
            (None, Some(b)) => format!("Frame B:{} (no match in A)", b),
            (None, None) => "No frames selected".to_string(),
        }
    }
}

/// Diff statistics for a single frame comparison
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DiffStatistics {
    /// Mean absolute difference
    pub mean_diff: f32,
    /// Max absolute difference
    pub max_diff: f32,
    /// Min absolute difference
    pub min_diff: f32,
    /// Standard deviation
    pub std_dev: f32,
    /// Percentage of pixels with diff > 10
    pub diff_area_pct: f32,
    /// Peak signal to noise ratio (if calculable)
    pub psnr: Option<f32>,
}

impl DiffStatistics {
    /// Calculate from diff heatmap data
    pub fn from_heatmap(heatmap: &DiffHeatmapData) -> Self {
        if heatmap.values.is_empty() {
            return Self::default();
        }

        let n = heatmap.values.len() as f32;

        // Calculate mean
        let sum: f32 = heatmap.values.iter().map(|v| v.abs()).sum();
        let mean_diff = sum / n;

        // Calculate min/max
        let min_diff = heatmap
            .values
            .iter()
            .map(|v| v.abs())
            .fold(f32::MAX, f32::min);
        let max_diff = heatmap
            .values
            .iter()
            .map(|v| v.abs())
            .fold(f32::MIN, f32::max);

        // Calculate std dev
        let variance: f32 = heatmap
            .values
            .iter()
            .map(|v| {
                let d = v.abs() - mean_diff;
                d * d
            })
            .sum::<f32>()
            / n;
        let std_dev = variance.sqrt();

        // Calculate diff area (pixels with diff > 10)
        let diff_count = heatmap.values.iter().filter(|v| v.abs() > 10.0).count();
        let diff_area_pct = (diff_count as f32 / n) * 100.0;

        // Calculate PSNR (if max_diff > 0)
        let psnr = if max_diff > 0.0 {
            let mse = heatmap.values.iter().map(|v| v * v).sum::<f32>() / n;
            if mse > 0.0 {
                Some(10.0 * (255.0_f32.powi(2) / mse).log10())
            } else {
                Some(f32::INFINITY) // Perfect match
            }
        } else {
            Some(f32::INFINITY)
        };

        Self {
            mean_diff,
            max_diff,
            min_diff,
            std_dev,
            diff_area_pct,
            psnr,
        }
    }

    /// Format as summary text
    pub fn summary_text(&self) -> String {
        let psnr_str = self
            .psnr
            .map(|p| {
                if p.is_infinite() {
                    "∞".to_string()
                } else {
                    format!("{:.2}", p)
                }
            })
            .unwrap_or_else(|| "N/A".to_string());

        format!(
            "Mean: {:.2} | Max: {:.2} | Diff Area: {:.1}% | PSNR: {} dB",
            self.mean_diff, self.max_diff, self.diff_area_pct, psnr_str
        )
    }
}

/// Diff hover info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffHoverInfo {
    /// Pixel X coordinate
    pub pixel_x: u32,

    /// Pixel Y coordinate
    pub pixel_y: u32,

    /// Diff value at this pixel
    pub diff_value: f32,
}

impl DiffHoverInfo {
    /// Format as tooltip text
    pub fn format_tooltip(&self, mode: DiffMode) -> String {
        format!(
            "Pixel: ({}, {})\nDiff ({}): {:.1}",
            self.pixel_x,
            self.pixel_y,
            mode.display_name(),
            self.diff_value
        )
    }
}

// ============================================================================
// DiffMode Tests
// ============================================================================

#[cfg(test)]
mod diff_mode_tests {
    use super::*;

    #[test]
    fn test_diff_mode_default() {
        // Arrange & Act
        let mode = DiffMode::default();

        // Assert
        assert_eq!(mode, DiffMode::Abs);
    }

    #[test]
    fn test_diff_mode_display_name() {
        // Arrange & Act
        assert_eq!(DiffMode::Abs.display_name(), "Absolute");
        assert_eq!(DiffMode::Signed.display_name(), "Signed");
        assert_eq!(DiffMode::Metric.display_name(), "Metric");
    }

    #[test]
    fn test_diff_mode_cache_key() {
        // Arrange & Act
        assert_eq!(DiffMode::Abs.cache_key(), "abs");
        assert_eq!(DiffMode::Signed.cache_key(), "signed");
        assert_eq!(DiffMode::Metric.cache_key(), "metric");
    }
}

// ============================================================================
// ResolutionCheckResult Tests
// ============================================================================

#[cfg(test)]
mod resolution_check_tests {
    use super::*;

    #[test]
    fn test_check_compatible_same_resolution() {
        // Arrange & Act
        let result = ResolutionCheckResult::check(1920, 1080, 1920, 1080);

        // Assert
        assert!(result.is_compatible);
        assert!(result.disable_reason.is_none());
    }

    #[test]
    fn test_check_compatible_small_difference() {
        // Arrange & Act
        let result = ResolutionCheckResult::check(1920, 1080, 1900, 1080);

        // Assert
        assert!(result.is_compatible);
        assert!(result.disable_reason.is_none());
    }

    #[test]
    fn test_check_incompatible_large_difference() {
        // Arrange & Act
        let result = ResolutionCheckResult::check(1920, 1080, 1280, 720);

        // Assert
        assert!(!result.is_compatible);
        assert!(result.disable_reason.is_some());
    }

    #[test]
    fn test_check_summary_compatible() {
        // Arrange
        let result = ResolutionCheckResult::check(1280, 720, 1280, 720);

        // Act
        let summary = result.summary();

        // Assert
        assert!(summary.contains("1280x720"));
        assert!(!summary.contains("mismatch"));
    }

    #[test]
    fn test_check_summary_incompatible() {
        // Arrange
        let result = ResolutionCheckResult::check(1920, 1080, 1280, 720);

        // Act
        let summary = result.summary();

        // Assert
        assert!(summary.contains("mismatch"));
    }
}

// ============================================================================
// DiffHeatmapData Tests
// ============================================================================

#[cfg(test)]
mod diff_heatmap_data_tests {
    use super::*;

    #[test]
    fn test_diff_heatmap_data_from_luma_planes() {
        // Arrange
        let luma_a = vec![128u8; 640 * 480];
        let luma_b = vec![130u8; 640 * 480]; // Small diff

        // Act
        let data = DiffHeatmapData::from_luma_planes(&luma_a, &luma_b, 640, 480, DiffMode::Abs);

        // Assert
        assert_eq!(data.frame_width, 640);
        assert_eq!(data.frame_height, 480);
        assert_eq!(data.heatmap_width, 320); // half-res
        assert_eq!(data.heatmap_height, 240);
        assert!(!data.values.is_empty());
    }

    #[test]
    fn test_diff_heatmap_data_get_value() {
        // Arrange
        let luma_a = vec![100u8; 64];
        let luma_b = vec![110u8; 64]; // Diff of 10
        let data = DiffHeatmapData::from_luma_planes(&luma_a, &luma_b, 8, 8, DiffMode::Abs);

        // Act
        let value = data.get_value(0, 0);

        // Assert
        assert_eq!(value, Some(10.0));
    }

    #[test]
    fn test_diff_heatmap_data_get_value_out_of_bounds() {
        // Arrange
        let luma_a = vec![128u8; 64];
        let luma_b = vec![128u8; 64];
        let data = DiffHeatmapData::from_luma_planes(&luma_a, &luma_b, 8, 8, DiffMode::Abs);

        // Act
        let value = data.get_value(999, 999);

        // Assert
        assert!(value.is_none());
    }
}

// ============================================================================
// DiffHeatmapOverlay Tests
// ============================================================================

#[cfg(test)]
mod diff_heatmap_overlay_tests {
    use super::*;

    #[test]
    fn test_diff_heatmap_overlay_new() {
        // Arrange & Act
        let overlay = DiffHeatmapOverlay::new(1920, 1080);

        // Assert
        assert!(overlay.is_enabled());
        assert_eq!(overlay.mode, DiffMode::Abs);
    }

    #[test]
    fn test_diff_heatmap_overlay_set_mode() {
        // Arrange
        let mut overlay = DiffHeatmapOverlay::new(1920, 1080);

        // Act
        overlay.set_mode(DiffMode::Signed);

        // Assert
        assert_eq!(overlay.mode, DiffMode::Signed);
    }

    #[test]
    fn test_diff_heatmap_overlay_enable_disable() {
        // Arrange
        let mut overlay = DiffHeatmapOverlay::new(1920, 1080);

        // Act
        overlay.enable();

        // Assert
        assert!(overlay.is_enabled());
        assert!(overlay.disable_reason.is_none());

        // Act - Disable with reason
        overlay.disable("Test disable".to_string());

        // Assert
        assert!(!overlay.is_enabled());
        assert_eq!(overlay.disable_reason, Some("Test disable".to_string()));
    }

    #[test]
    fn test_diff_heatmap_overlay_hover() {
        // Arrange
        let mut overlay = DiffHeatmapOverlay::new(1920, 1080);

        // Act
        overlay.set_hover(100, 200, 15.5);

        // Assert
        assert!(overlay.hover_info.is_some());
        let info = overlay.hover_info.as_ref().unwrap();
        assert_eq!(info.pixel_x, 100);
        assert_eq!(info.pixel_y, 200);
        assert_eq!(info.diff_value, 15.5);
    }

    #[test]
    fn test_diff_heatmap_overlay_clear_hover() {
        // Arrange
        let mut overlay = DiffHeatmapOverlay::new(1920, 1080);
        overlay.set_hover(100, 200, 15.5);

        // Act
        overlay.clear_hover();

        // Assert
        assert!(overlay.hover_info.is_none());
    }
}

// ============================================================================
// DiffCompareContext Tests
// ============================================================================

#[cfg(test)]
mod diff_compare_context_tests {
    use super::*;

    #[test]
    fn test_diff_compare_context_new() {
        // Arrange & Act
        let ctx = DiffCompareContext::new(1920, 1080, 1920, 1080);

        // Assert
        assert!(ctx.resolution_check.is_compatible);
        assert!(ctx.frame_a_idx.is_none());
        assert!(ctx.frame_b_idx.is_none());
        assert!(!ctx.has_gap);
        assert!(!ctx.is_diff_available()); // No frames set
    }

    #[test]
    fn test_diff_compare_context_set_frame_pair() {
        // Arrange
        let mut ctx = DiffCompareContext::new(1920, 1080, 1920, 1080);

        // Act
        ctx.set_frame_pair(Some(10), Some(10), Some(0), false);

        // Assert
        assert_eq!(ctx.frame_a_idx, Some(10));
        assert_eq!(ctx.frame_b_idx, Some(10));
        assert!(ctx.is_diff_available());
    }

    #[test]
    fn test_diff_compare_context_with_gap() {
        // Arrange
        let mut ctx = DiffCompareContext::new(1920, 1080, 1920, 1080);

        // Act
        ctx.set_frame_pair(Some(10), None, None, true); // has_gap = true

        // Assert
        assert!(ctx.has_gap);
        assert!(!ctx.is_diff_available());
    }

    #[test]
    fn test_diff_compare_context_incompatible_resolution() {
        // Arrange
        let ctx = DiffCompareContext::new(1920, 1080, 1280, 720);

        // Act
        let status = ctx.status_text();

        // Assert
        assert!(status.contains("mismatch"));
        assert!(!ctx.is_diff_available());
    }
}

// ============================================================================
// DiffStatistics Tests
// ============================================================================

#[cfg(test)]
mod diff_statistics_tests {
    use super::*;

    #[test]
    fn test_diff_statistics_default() {
        // Arrange & Act
        let stats = DiffStatistics::default();

        // Assert
        assert_eq!(stats.mean_diff, 0.0);
        assert_eq!(stats.max_diff, 0.0);
        assert_eq!(stats.min_diff, 0.0);
    }

    #[test]
    fn test_diff_statistics_from_heatmap() {
        // Arrange
        let luma_a = vec![100u8; 16];
        let luma_b = vec![105u8; 16]; // Diff of 5
        let data = DiffHeatmapData::from_luma_planes(&luma_a, &luma_b, 4, 4, DiffMode::Abs);

        // Act
        let stats = DiffStatistics::from_heatmap(&data);

        // Assert
        assert_eq!(stats.mean_diff, 5.0);
        assert_eq!(stats.min_diff, 5.0);
        assert_eq!(stats.max_diff, 5.0);
    }
}

// ============================================================================
// Edge Cases
// ============================================================================

#[cfg(test)]
mod edge_case_tests {
    use super::*;

    #[test]
    fn test_diff_heatmap_zero_resolution() {
        // Arrange & Act
        let luma_a = vec![];
        let luma_b = vec![];
        let data = DiffHeatmapData::from_luma_planes(&luma_a, &luma_b, 0, 0, DiffMode::Abs);

        // Assert - Should handle gracefully
        assert_eq!(data.frame_width, 0);
        assert_eq!(data.frame_height, 0);
    }

    #[test]
    fn test_opacity_bucket() {
        // Arrange
        let overlay = DiffHeatmapOverlay::new(1920, 1080);

        // Act
        let bucket = overlay.opacity_bucket();

        // Assert - Should be a valid u32
        assert!(bucket < 10); // Buckets are 0-9
    }

    #[test]
    fn test_diff_hover_info_format() {
        // Arrange
        let info = DiffHoverInfo {
            pixel_x: 100,
            pixel_y: 200,
            diff_value: 50.0,
        };

        // Act
        let tooltip = info.format_tooltip(DiffMode::Abs);

        // Assert
        assert!(tooltip.contains("(100, 200)"));
        assert!(tooltip.contains("50.0"));
    }
}

// ============================================================================
// Module Tests
// ============================================================================

#[cfg(test)]
mod tests {

    #[test]
    fn test_diff_heatmap_module_compiles() {
        // Module-level test to ensure the file compiles
        assert!(true);
    }
}
