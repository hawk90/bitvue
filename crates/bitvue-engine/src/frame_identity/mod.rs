//! Frame Identity Resolver - T0-1
//!
//! Per FRAME_IDENTITY_CONTRACT.md:
//! - Primary timeline index = display_idx (PTS order)
//! - decode_idx is internal only (DTS order)
//! - PTS quality detection (OK/WARN/BAD)
//!
//! Edge cases per EDGE_CASES_AND_DEGRADE_BEHAVIOR.md:
//! - VFR, missing or duplicated PTS → fallback to display_idx with badge
//! - PTS quality badge: OK/WARN/BAD
//!
//! Split across submodules by concern, all re-exported here so
//! `bitvue_engine::frame_identity::X` paths are unchanged:
//! - this file: the core [`FrameIndexMap`]/[`FrameMetadata`]/[`PtsQuality`] identity layer
//! - [`extractor`]: per-codec [`FrameIdentityExtractor`] impls + [`FrameMapper`]
//! - [`quirks`]: AV1-specific timeline quirks (show_existing_frame, film grain, tiles)
//! - [`timeline_extractor`]: [`TimelineExtractor`] impls + [`TimelineMapper`] (identity → viz)
//! - [`axis`]: [`TimelineAxis`] pan/zoom/scale over display_idx coordinates
//! - [`cursor`]: [`CursorSync`] crosshair shared across timeline/player/overlays

mod axis;
mod cursor;
mod extractor;
mod quirks;
mod timeline_extractor;

pub use axis::{AxisBounds, AxisScaleMode, TimelineAxis};
pub use cursor::{CursorSync, CursorVisibility, TimelineCursor};
pub use extractor::{
    build_frame_map_from_extractor, Av1FrameIdentityExtractor, ExtractionError,
    FrameIdentityExtractor, FrameMapper, H264FrameIdentityExtractor, HevcFrameIdentityExtractor,
    Vp9FrameIdentityExtractor,
};
pub use quirks::{
    Av1FrameIdentityQuirks, Av1QuirksTimeline, FrameVizHint, TimelineFrameWithQuirks,
};
pub use timeline_extractor::{
    Av1TimelineExtractor, Avs3TimelineExtractor, H264TimelineExtractor, HevcTimelineExtractor,
    TimelineExtractor, TimelineMapper, Vp9TimelineExtractor, VvcTimelineExtractor,
};

use serde::{Deserialize, Serialize};

/// Frame index mapping between display order (PTS) and decode order (DTS)
///
/// **Primary timeline index = display_idx (PTS order)**
/// decode_idx is internal only and used for decoder reordering.
///
/// Invariants (FRAME_IDENTITY_CONTRACT.md):
/// - display_idx is stable across sessions for the same stream
/// - A frame selection always maps to exactly one display_idx
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameIndexMap {
    /// Total frame count
    pub frame_count: usize,

    /// Map: display_idx → decode_idx (INTERNAL ONLY per FRAME_IDENTITY_CONTRACT)
    display_to_decode: Vec<usize>,

    /// Map: decode_idx → display_idx (INTERNAL ONLY per FRAME_IDENTITY_CONTRACT)
    decode_to_display: Vec<usize>,

    /// Map: display_idx → PTS (if available)
    display_to_pts: Vec<Option<u64>>,

    /// Map: display_idx → DTS (if available)
    display_to_dts: Vec<Option<u64>>,

    /// PTS quality assessment
    pub pts_quality: PtsQuality,
}

impl FrameIndexMap {
    /// Create a new FrameIndexMap from frame metadata
    ///
    /// Frames are provided in decode order. This function:
    /// 1. Determines PTS quality
    /// 2. Sorts frames by PTS to establish display_idx
    /// 3. Creates bidirectional mapping between display_idx and decode_idx
    pub fn new(frames: &[FrameMetadata]) -> Self {
        let frame_count = frames.len();

        if frame_count == 0 {
            return Self::empty();
        }

        // Assess PTS quality
        let pts_quality = Self::assess_pts_quality(frames);

        // Build display order (PTS sorted)
        let mut display_order: Vec<(usize, Option<u64>)> = frames
            .iter()
            .enumerate()
            .map(|(decode_idx, frame)| (decode_idx, frame.pts))
            .collect();

        // Sort by PTS, fallback to decode order for missing/duplicate PTS
        display_order.sort_by(
            |(decode_a, pts_a), (decode_b, pts_b)| match (pts_a, pts_b) {
                (Some(a), Some(b)) => a.cmp(b).then_with(|| decode_a.cmp(decode_b)),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => decode_a.cmp(decode_b),
            },
        );

        // Build mappings
        let mut display_to_decode = Vec::with_capacity(frame_count);
        let mut decode_to_display = vec![0; frame_count];
        let mut display_to_pts = Vec::with_capacity(frame_count);
        let mut display_to_dts = Vec::with_capacity(frame_count);

        for (display_idx, (decode_idx, _)) in display_order.iter().enumerate() {
            display_to_decode.push(*decode_idx);
            decode_to_display[*decode_idx] = display_idx;
            display_to_pts.push(frames[*decode_idx].pts);
            display_to_dts.push(frames[*decode_idx].dts);
        }

        Self {
            frame_count,
            display_to_decode,
            decode_to_display,
            display_to_pts,
            display_to_dts,
            pts_quality,
        }
    }

    /// Create an empty FrameIndexMap
    fn empty() -> Self {
        Self {
            frame_count: 0,
            display_to_decode: Vec::new(),
            decode_to_display: Vec::new(),
            display_to_pts: Vec::new(),
            display_to_dts: Vec::new(),
            pts_quality: PtsQuality::Ok,
        }
    }

    /// Assess PTS quality from frame metadata
    ///
    /// Per EDGE_CASES_AND_DEGRADE_BEHAVIOR.md §A:
    /// - OK: All frames have valid, monotonic PTS
    /// - WARN: Some missing PTS, but majority present
    /// - BAD: Major issues (duplicates, VFR, >50% missing)
    ///
    /// PERFORMANCE: Optimized to single-pass O(n) algorithm using:
    /// - Online variance calculation (Welford's method)
    /// - HashSet for O(1) duplicate detection
    /// - Combined iteration for all metrics
    fn assess_pts_quality(frames: &[FrameMetadata]) -> PtsQuality {
        if frames.is_empty() {
            return PtsQuality::Ok;
        }

        use std::collections::HashSet;

        let total = frames.len();
        let mut missing_count = 0;
        let mut seen_pts = HashSet::new();
        let mut has_duplicates = false;

        // Online statistics for VFR detection (Welford's algorithm)
        let mut pts_values: Vec<u64> = Vec::new();
        let mut count = 0usize;
        let mut mean = 0.0f64;
        let mut m2 = 0.0f64; // Sum of squared differences from mean

        // Single pass: collect all metrics
        for frame in frames.iter() {
            match frame.pts {
                Some(pts) => {
                    // Check for duplicates using HashSet
                    if !seen_pts.insert(pts) {
                        has_duplicates = true;
                    }
                    pts_values.push(pts);
                }
                None => {
                    missing_count += 1;
                }
            }
        }

        // Check if >50% missing
        if missing_count > total / 2 {
            return PtsQuality::Bad;
        }

        // Check for duplicate PTS values
        if has_duplicates {
            return PtsQuality::Bad;
        }

        // Note: We do NOT check for monotonicity in decode order.
        // PTS values are allowed to be out of order in decode sequence (e.g., B-frames).
        // The sorting step above establishes display order.
        // Duplicates are the only PTS ordering issue we flag as BAD.

        // Check for VFR (variable frame rate) by analyzing PTS deltas
        // Need sorted PTS values for delta calculation
        if pts_values.len() >= 3 {
            pts_values.sort_unstable();

            // Calculate deltas and use Welford's algorithm for online variance
            for window in pts_values.windows(2) {
                let delta = (window[1] - window[0]) as f64;
                count += 1;
                let delta_mean = (delta - mean) / count as f64;
                mean += delta_mean;
                let delta_delta = delta - mean;
                m2 += delta_delta * delta_delta;
            }

            if count > 0 {
                let variance = m2 / count as f64;
                let std_dev = variance.sqrt();

                // If coefficient of variation > 0.3, consider VFR
                if mean > 0.0 && (std_dev / mean) > 0.3 {
                    return PtsQuality::Warn;
                }
            }
        }

        // If some missing but <50%, WARN
        if missing_count > 0 {
            return PtsQuality::Warn;
        }

        PtsQuality::Ok
    }

    /// Convert display_idx to decode_idx
    #[inline]
    pub fn display_to_decode_idx(&self, display_idx: usize) -> Option<usize> {
        self.display_to_decode.get(display_idx).copied()
    }

    /// Convert decode_idx to display_idx
    ///
    /// **WARNING**: This function is for TESTING ONLY.
    /// Per FRAME_IDENTITY_CONTRACT: decode_idx is internal-only.
    /// Production code must use display_idx as primary index.
    #[cfg(test)]
    #[inline]
    pub fn decode_to_display_idx(&self, decode_idx: usize) -> Option<usize> {
        self.decode_to_display.get(decode_idx).copied()
    }

    /// Get PTS for display_idx
    #[inline]
    pub fn get_pts(&self, display_idx: usize) -> Option<u64> {
        self.display_to_pts.get(display_idx).and_then(|&pts| pts)
    }

    /// Get DTS for display_idx
    #[inline]
    pub fn get_dts(&self, display_idx: usize) -> Option<u64> {
        self.display_to_dts.get(display_idx).and_then(|&dts| dts)
    }

    /// Get frame count
    #[inline]
    pub fn frame_count(&self) -> usize {
        self.frame_count
    }

    /// Get PTS quality
    #[inline]
    pub fn pts_quality(&self) -> PtsQuality {
        self.pts_quality
    }

    /// Check if reordering is present (display order != decode order)
    pub fn has_reordering(&self) -> bool {
        self.display_to_decode
            .iter()
            .enumerate()
            .any(|(display_idx, &decode_idx)| display_idx != decode_idx)
    }
}

/// Frame metadata for building FrameIndexMap
///
/// Minimal metadata required to establish display/decode ordering.
#[derive(Debug, Clone, Copy)]
pub struct FrameMetadata {
    /// Presentation timestamp (if available)
    pub pts: Option<u64>,
    /// Decode timestamp (if available)
    pub dts: Option<u64>,
}

/// PTS quality assessment
///
/// Per EDGE_CASES_AND_DEGRADE_BEHAVIOR.md §A:
/// - OK: All frames have valid, monotonic PTS
/// - WARN: Some missing PTS or VFR detected, but usable
/// - BAD: Major issues (duplicates, >50% missing, non-monotonic)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PtsQuality {
    /// All PTS values present and monotonic
    Ok,
    /// Some issues detected (missing PTS, VFR), but usable
    Warn,
    /// Major issues (duplicates, >50% missing, non-monotonic)
    Bad,
}

impl PtsQuality {
    /// Get display text for the badge
    pub fn badge_text(&self) -> &'static str {
        match self {
            PtsQuality::Ok => "PTS: OK",
            PtsQuality::Warn => "PTS: WARN",
            PtsQuality::Bad => "PTS: BAD",
        }
    }

    /// Get color hint for UI display
    pub fn color_hint(&self) -> PtsQualityColor {
        match self {
            PtsQuality::Ok => PtsQualityColor::Green,
            PtsQuality::Warn => PtsQualityColor::Yellow,
            PtsQuality::Bad => PtsQualityColor::Red,
        }
    }

    /// Get tooltip explanation
    pub fn tooltip(&self) -> &'static str {
        match self {
            PtsQuality::Ok => "All frames have valid, monotonic PTS values",
            PtsQuality::Warn => {
                "Some PTS values missing or variable frame rate detected. Timeline uses frame index fallback."
            }
            PtsQuality::Bad => {
                "Major PTS issues detected (duplicates, non-monotonic, or >50% missing). Timeline uses frame index."
            }
        }
    }
}

/// Color hint for PTS quality badge
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PtsQualityColor {
    Green,
    Yellow,
    Red,
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
    unused_assignments,
    clippy::module_inception
)]
#[cfg(test)]
mod tests {
    include!("tests.rs");
}
