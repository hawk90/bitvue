//! A/B Compare View - T6-2
//!
//! Per COMPARE_ALIGNMENT_POLICY.md and LAYOUT_CONTRACT.md:
//! - Side-by-side player view with Stream A and B
//! - Sync controls (Off/Playhead/Full)
//! - Manual offset UI for alignment adjustment
//! - Resolution mismatch detection
//!
//! Edge cases per EDGE_CASES_AND_DEGRADE_BEHAVIOR.md:
//! - Resolution mismatch → diff overlays disabled with explanation
//! - Alignment failure → banner with reason

use crate::{AlignmentEngine, FrameIndexMap, SyncMode};
use serde::{Deserialize, Serialize};

/// Compare workspace managing A/B streams
///
/// Manages side-by-side comparison with:
/// - Automatic alignment via AlignmentEngine
/// - Manual offset adjustment
/// - Sync controls
/// - Resolution mismatch detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompareWorkspace {
    /// Stream A frame index map
    pub stream_a: FrameIndexMap,

    /// Stream B frame index map
    pub stream_b: FrameIndexMap,

    /// Alignment engine (PTS-based or fallback)
    pub alignment: AlignmentEngine,

    /// Manual offset adjustment (in display_idx units)
    pub manual_offset: i32,

    /// Current sync mode
    pub sync_mode: SyncMode,

    /// Resolution information
    pub resolution_info: ResolutionInfo,

    /// Diff overlays enabled
    pub diff_enabled: bool,

    /// Disable reason (if diff disabled)
    pub disable_reason: Option<String>,
}

impl CompareWorkspace {
    /// Create new compare workspace
    ///
    /// Performs automatic alignment and resolution check.
    pub fn new(
        stream_a: FrameIndexMap,
        stream_b: FrameIndexMap,
        resolution_a: (u32, u32),
        resolution_b: (u32, u32),
    ) -> Self {
        // Create alignment
        let alignment = AlignmentEngine::new(&stream_a, &stream_b);

        // Check resolution compatibility
        let resolution_info = ResolutionInfo::new(resolution_a, resolution_b);

        // Determine if diff overlays can be enabled
        let (diff_enabled, disable_reason) =
            Self::check_diff_eligibility(&resolution_info, &alignment);

        Self {
            stream_a,
            stream_b,
            alignment,
            manual_offset: 0,
            sync_mode: SyncMode::Off,
            resolution_info,
            diff_enabled,
            disable_reason,
        }
    }

    /// Check if diff overlays can be enabled
    ///
    /// Per COMPARE_ALIGNMENT_POLICY.md:
    /// - Resolution mismatch > tolerance → disable
    /// - Alignment failure → disable with reason
    fn check_diff_eligibility(
        resolution_info: &ResolutionInfo,
        alignment: &AlignmentEngine,
    ) -> (bool, Option<String>) {
        // Check resolution mismatch
        if !resolution_info.is_compatible() {
            return (
                false,
                Some(format!(
                    "Resolution mismatch: {}x{} vs {}x{} ({}% difference)",
                    resolution_info.stream_a.0,
                    resolution_info.stream_a.1,
                    resolution_info.stream_b.0,
                    resolution_info.stream_b.1,
                    (resolution_info.mismatch_percentage() * 100.0) as u32
                )),
            );
        }

        // Check alignment confidence
        if alignment.confidence() == crate::AlignmentConfidence::Low
            && alignment.gap_percentage() > 30.0
        {
            return (
                false,
                Some(format!(
                    "Low alignment confidence: {} method, {:.1}% gaps",
                    alignment.method.display_text(),
                    alignment.gap_percentage()
                )),
            );
        }

        (true, None)
    }

    /// Set sync mode
    pub fn set_sync_mode(&mut self, mode: SyncMode) {
        self.sync_mode = mode;
    }

    /// Set manual offset
    ///
    /// Positive offset = B is ahead of A
    /// Negative offset = B is behind A
    pub fn set_manual_offset(&mut self, offset: i32) {
        self.manual_offset = offset;
    }

    /// Adjust manual offset by delta
    pub fn adjust_offset(&mut self, delta: i32) {
        self.manual_offset = self.manual_offset.saturating_add(delta);
    }

    /// Get aligned frame for stream A index
    ///
    /// Takes manual offset into account.
    /// Returns (stream_b_idx, confidence_level)
    pub fn get_aligned_frame(&self, stream_a_idx: usize) -> Option<(usize, AlignmentQuality)> {
        // Apply manual offset
        let adjusted_a_idx = (stream_a_idx as i32 + self.manual_offset).max(0) as usize;

        // Look up alignment
        if let Some(pair) = self.alignment.get_pair_for_a(adjusted_a_idx) {
            if let Some(b_idx) = pair.stream_b_idx {
                let quality = if pair.has_gap {
                    AlignmentQuality::Gap
                } else if pair.pts_delta_abs().unwrap_or(0) == 0 {
                    AlignmentQuality::Exact
                } else {
                    AlignmentQuality::Nearest
                };
                return Some((b_idx, quality));
            }
        }

        None
    }

    /// Check if diff overlays are enabled
    pub fn is_diff_enabled(&self) -> bool {
        self.diff_enabled
    }

    /// Get disable reason
    pub fn disable_reason(&self) -> Option<&str> {
        self.disable_reason.as_deref()
    }

    /// Get sync mode
    pub fn sync_mode(&self) -> SyncMode {
        self.sync_mode
    }

    /// Get manual offset
    pub fn manual_offset(&self) -> i32 {
        self.manual_offset
    }

    /// Reset manual offset to 0
    pub fn reset_offset(&mut self) {
        self.manual_offset = 0;
    }

    /// Get total frame count (max of A and B)
    pub fn total_frames(&self) -> usize {
        self.stream_a.frame_count().max(self.stream_b.frame_count())
    }

    /// Get resolution info
    pub fn resolution_info(&self) -> &ResolutionInfo {
        &self.resolution_info
    }
}

/// Resolution information for both streams
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolutionInfo {
    /// Stream A resolution (width, height)
    pub stream_a: (u32, u32),

    /// Stream B resolution (width, height)
    pub stream_b: (u32, u32),

    /// Compatibility threshold (default 5%)
    pub tolerance: f64,
}

impl ResolutionInfo {
    /// Create resolution info with default 5% tolerance
    pub fn new(stream_a: (u32, u32), stream_b: (u32, u32)) -> Self {
        Self {
            stream_a,
            stream_b,
            tolerance: 0.05, // 5%
        }
    }

    /// Check if resolutions are compatible within tolerance
    pub fn is_compatible(&self) -> bool {
        self.mismatch_percentage() <= self.tolerance
    }

    /// Calculate resolution mismatch percentage
    ///
    /// Returns ratio of pixel difference to average pixel count.
    pub fn mismatch_percentage(&self) -> f64 {
        let a_pixels = (self.stream_a.0 * self.stream_a.1) as f64;
        let b_pixels = (self.stream_b.0 * self.stream_b.1) as f64;

        if a_pixels == 0.0 && b_pixels == 0.0 {
            return 0.0;
        }

        let diff = (a_pixels - b_pixels).abs();
        let avg = (a_pixels + b_pixels) / 2.0;

        diff / avg
    }

    /// Check if resolutions are exactly equal
    pub fn is_exact_match(&self) -> bool {
        self.stream_a == self.stream_b
    }

    /// Get scale indicator for UI
    pub fn scale_indicator(&self) -> String {
        if self.is_exact_match() {
            "1:1".to_string()
        } else {
            format!(
                "{}x{} vs {}x{}",
                self.stream_a.0, self.stream_a.1, self.stream_b.0, self.stream_b.1
            )
        }
    }
}

/// Alignment quality for frame pair
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlignmentQuality {
    /// Exact PTS match (delta = 0)
    Exact,
    /// Nearest neighbor match (small delta)
    Nearest,
    /// Gap in alignment
    Gap,
}

impl AlignmentQuality {
    /// Get display text
    pub fn display_text(&self) -> &'static str {
        match self {
            AlignmentQuality::Exact => "Exact",
            AlignmentQuality::Nearest => "Nearest",
            AlignmentQuality::Gap => "Gap",
        }
    }

    /// Get color hint for UI
    pub fn color_hint(&self) -> AlignmentQualityColor {
        match self {
            AlignmentQuality::Exact => AlignmentQualityColor::Green,
            AlignmentQuality::Nearest => AlignmentQualityColor::Yellow,
            AlignmentQuality::Gap => AlignmentQualityColor::Red,
        }
    }
}

/// Color hint for alignment quality
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlignmentQualityColor {
    Green,
    Yellow,
    Red,
}

/// Sync controls UI state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncControls {
    /// Current sync mode
    pub mode: SyncMode,

    /// Manual offset enabled
    pub manual_offset_enabled: bool,

    /// Manual offset value
    pub manual_offset: i32,

    /// Show alignment info
    pub show_alignment_info: bool,
}

impl SyncControls {
    /// Create new sync controls with default state
    pub fn new() -> Self {
        Self {
            mode: SyncMode::Off,
            manual_offset_enabled: false,
            manual_offset: 0,
            show_alignment_info: false,
        }
    }

    /// Toggle sync mode
    pub fn toggle_sync(&mut self) {
        self.mode = match self.mode {
            SyncMode::Off => SyncMode::Playhead,
            SyncMode::Playhead => SyncMode::Full,
            SyncMode::Full => SyncMode::Off,
        };
    }

    /// Set sync mode
    pub fn set_mode(&mut self, mode: SyncMode) {
        self.mode = mode;
    }

    /// Enable manual offset
    pub fn enable_manual_offset(&mut self) {
        self.manual_offset_enabled = true;
    }

    /// Disable manual offset
    pub fn disable_manual_offset(&mut self) {
        self.manual_offset_enabled = false;
        self.manual_offset = 0;
    }

    /// Set manual offset
    pub fn set_offset(&mut self, offset: i32) {
        self.manual_offset = offset;
    }

    /// Adjust offset by delta
    pub fn adjust_offset(&mut self, delta: i32) {
        self.manual_offset = self.manual_offset.saturating_add(delta);
    }

    /// Reset offset to 0
    pub fn reset_offset(&mut self) {
        self.manual_offset = 0;
    }

    /// Toggle alignment info display
    pub fn toggle_alignment_info(&mut self) {
        self.show_alignment_info = !self.show_alignment_info;
    }
}

impl Default for SyncControls {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    include!("compare_test.rs");
}
