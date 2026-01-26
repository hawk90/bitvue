//! Compare Alignment Engine - T6-1
//!
//! Per COMPARE_ALIGNMENT_POLICY.md:
//! 1. PTS-based alignment (primary)
//! 2. display_idx fallback
//! 3. Nearest-neighbor with gap indicator
//! 4. Confidence scoring (High/Med/Low)
//!
//! Edge cases per EDGE_CASES_AND_DEGRADE_BEHAVIOR.md:
//! - Resolution mismatch → disable diff overlays
//! - Alignment failure → banner with reason

use crate::frame_identity::{FrameIndexMap, PtsQuality};
use serde::{Deserialize, Serialize};

/// Alignment engine for comparing two video streams
///
/// Aligns frames from two streams based on PTS with fallback to display_idx.
/// Per COMPARE_ALIGNMENT_POLICY.md:
/// - Primary: PTS-based alignment
/// - Fallback: display_idx if PTS unreliable
/// - Gap detection: nearest-neighbor with gap indicators
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlignmentEngine {
    /// Frame count in stream A
    pub stream_a_count: usize,

    /// Frame count in stream B
    pub stream_b_count: usize,

    /// Alignment method used
    pub method: AlignmentMethod,

    /// Confidence score
    pub confidence: AlignmentConfidence,

    /// Aligned frame pairs
    pub frame_pairs: Vec<FramePair>,

    /// Gap count (frames without match)
    pub gap_count: usize,
}

impl AlignmentEngine {
    /// Create alignment between two streams
    ///
    /// Per COMPARE_ALIGNMENT_POLICY.md:
    /// 1. Try PTS-based alignment first
    /// 2. Fall back to display_idx if PTS unreliable
    /// 3. Use nearest-neighbor for gaps
    pub fn new(stream_a: &FrameIndexMap, stream_b: &FrameIndexMap) -> Self {
        let stream_a_count = stream_a.frame_count();
        let stream_b_count = stream_b.frame_count();

        // Check if PTS is reliable for both streams
        let can_use_pts = Self::can_use_pts_alignment(stream_a, stream_b);

        let (method, frame_pairs) = if can_use_pts {
            Self::align_by_pts(stream_a, stream_b)
        } else {
            Self::align_by_display_idx(stream_a, stream_b)
        };

        // Count gaps
        let gap_count = frame_pairs.iter().filter(|p| p.has_gap).count();

        // Calculate confidence
        let confidence =
            Self::calculate_confidence(&method, gap_count, stream_a_count, stream_b_count);

        Self {
            stream_a_count,
            stream_b_count,
            method,
            confidence,
            frame_pairs,
            gap_count,
        }
    }

    /// Check if PTS-based alignment is reliable
    ///
    /// PTS alignment requires:
    /// - Both streams have OK or WARN PTS quality (not BAD)
    /// - At least some PTS values present
    fn can_use_pts_alignment(stream_a: &FrameIndexMap, stream_b: &FrameIndexMap) -> bool {
        let a_quality = stream_a.pts_quality();
        let b_quality = stream_b.pts_quality();

        // BAD quality → cannot use PTS
        if a_quality == PtsQuality::Bad || b_quality == PtsQuality::Bad {
            return false;
        }

        // Check if at least some PTS values exist
        let a_has_pts = (0..stream_a.frame_count()).any(|i| stream_a.get_pts(i).is_some());
        let b_has_pts = (0..stream_b.frame_count()).any(|i| stream_b.get_pts(i).is_some());

        a_has_pts && b_has_pts
    }

    /// Align frames by PTS (primary method)
    ///
    /// Uses nearest-neighbor matching with gap detection.
    /// Gap threshold: 2x median PTS delta
    fn align_by_pts(
        stream_a: &FrameIndexMap,
        stream_b: &FrameIndexMap,
    ) -> (AlignmentMethod, Vec<FramePair>) {
        let a_count = stream_a.frame_count();
        let b_count = stream_b.frame_count();

        // Collect PTS values with indices
        let a_frames: Vec<(usize, u64)> = (0..a_count)
            .filter_map(|i| stream_a.get_pts(i).map(|pts| (i, pts)))
            .collect();
        let b_frames: Vec<(usize, u64)> = (0..b_count)
            .filter_map(|i| stream_b.get_pts(i).map(|pts| (i, pts)))
            .collect();

        if a_frames.is_empty() || b_frames.is_empty() {
            // Fallback if no PTS values
            return Self::align_by_display_idx(stream_a, stream_b);
        }

        // Calculate gap threshold (2x median delta)
        let gap_threshold = Self::calculate_gap_threshold(&a_frames, &b_frames);

        let mut pairs = Vec::new();
        let mut b_used = vec![false; b_frames.len()];

        // Match each frame in A to nearest in B within threshold
        for &(a_idx, a_pts) in &a_frames {
            let mut best_match: Option<(usize, usize, u64, i64)> = None; // (b_frames_idx, b_idx, b_pts, delta)

            for (b_frames_idx, &(b_idx, b_pts)) in b_frames.iter().enumerate() {
                if b_used[b_frames_idx] {
                    continue;
                }

                let delta_abs = (a_pts as i64 - b_pts as i64).unsigned_abs();

                // Only consider matches within threshold
                if delta_abs <= gap_threshold
                    && (best_match.is_none()
                        || delta_abs < best_match.as_ref().unwrap().3.unsigned_abs())
                {
                    best_match = Some((b_frames_idx, b_idx, b_pts, a_pts as i64 - b_pts as i64));
                }
            }

            if let Some((b_frames_idx, b_idx, _b_pts, pts_delta)) = best_match {
                // Good match within threshold
                b_used[b_frames_idx] = true;
                pairs.push(FramePair {
                    stream_a_idx: Some(a_idx),
                    stream_b_idx: Some(b_idx),
                    pts_delta: Some(pts_delta),
                    has_gap: false,
                });
            } else {
                // No match within threshold - gap in A
                pairs.push(FramePair {
                    stream_a_idx: Some(a_idx),
                    stream_b_idx: None,
                    pts_delta: None,
                    has_gap: true,
                });
            }
        }

        // Add unmatched B frames as gaps
        for (b_frames_idx, &(b_idx, _)) in b_frames.iter().enumerate() {
            if !b_used[b_frames_idx] {
                pairs.push(FramePair {
                    stream_a_idx: None,
                    stream_b_idx: Some(b_idx),
                    pts_delta: None,
                    has_gap: true,
                });
            }
        }

        // Determine if exact or nearest
        let method = if pairs.iter().all(|p| p.pts_delta == Some(0)) {
            AlignmentMethod::PtsExact
        } else {
            AlignmentMethod::PtsNearest
        };

        (method, pairs)
    }

    /// Calculate gap threshold for PTS matching
    ///
    /// Returns 0.5x median PTS delta (half a frame interval), or 500 if not enough data
    fn calculate_gap_threshold(a_frames: &[(usize, u64)], b_frames: &[(usize, u64)]) -> u64 {
        // Calculate median PTS delta in each stream
        let mut deltas = Vec::new();

        for frames in [a_frames, b_frames] {
            if frames.len() >= 2 {
                for window in frames.windows(2) {
                    let delta = window[1].1.saturating_sub(window[0].1);
                    if delta > 0 {
                        deltas.push(delta);
                    }
                }
            }
        }

        if deltas.is_empty() {
            return 500; // Default 0.5ms threshold
        }

        deltas.sort_unstable();
        let median = deltas[deltas.len() / 2];

        // 0.5x median (half frame interval)
        // If frames are 1000 units apart, threshold is 500
        median.saturating_div(2)
    }

    /// Align frames by display_idx (fallback method)
    ///
    /// Simple index-based alignment when PTS is unreliable.
    fn align_by_display_idx(
        stream_a: &FrameIndexMap,
        stream_b: &FrameIndexMap,
    ) -> (AlignmentMethod, Vec<FramePair>) {
        let a_count = stream_a.frame_count();
        let b_count = stream_b.frame_count();
        let max_count = a_count.max(b_count);

        let mut pairs = Vec::with_capacity(max_count);

        for i in 0..max_count {
            let a_idx = if i < a_count { Some(i) } else { None };
            let b_idx = if i < b_count { Some(i) } else { None };
            let has_gap = a_idx.is_none() || b_idx.is_none();

            pairs.push(FramePair {
                stream_a_idx: a_idx,
                stream_b_idx: b_idx,
                pts_delta: None,
                has_gap,
            });
        }

        (AlignmentMethod::DisplayIdx, pairs)
    }

    /// Calculate alignment confidence
    ///
    /// High: PTS exact or near-perfect alignment (<5% gaps)
    /// Medium: PTS nearest with moderate gaps (5-20%)
    /// Low: display_idx fallback or many gaps (>20%)
    fn calculate_confidence(
        method: &AlignmentMethod,
        gap_count: usize,
        stream_a_count: usize,
        stream_b_count: usize,
    ) -> AlignmentConfidence {
        let total_frames = stream_a_count.max(stream_b_count);
        if total_frames == 0 {
            return AlignmentConfidence::High;
        }

        let gap_ratio = gap_count as f64 / total_frames as f64;

        match method {
            AlignmentMethod::PtsExact => AlignmentConfidence::High,
            AlignmentMethod::PtsNearest => {
                if gap_ratio < 0.05 {
                    AlignmentConfidence::High
                } else if gap_ratio < 0.20 {
                    AlignmentConfidence::Medium
                } else {
                    AlignmentConfidence::Low
                }
            }
            AlignmentMethod::DisplayIdx => {
                if gap_ratio < 0.05 {
                    AlignmentConfidence::Medium
                } else {
                    AlignmentConfidence::Low
                }
            }
        }
    }

    /// Get frame pair for stream A index
    pub fn get_pair_for_a(&self, a_idx: usize) -> Option<&FramePair> {
        self.frame_pairs
            .iter()
            .find(|p| p.stream_a_idx == Some(a_idx))
    }

    /// Get frame pair for stream B index
    pub fn get_pair_for_b(&self, b_idx: usize) -> Option<&FramePair> {
        self.frame_pairs
            .iter()
            .find(|p| p.stream_b_idx == Some(b_idx))
    }

    /// Get confidence level
    pub fn confidence(&self) -> AlignmentConfidence {
        self.confidence
    }

    /// Get gap percentage
    pub fn gap_percentage(&self) -> f64 {
        let total = self.stream_a_count.max(self.stream_b_count);
        if total == 0 {
            return 0.0;
        }
        (self.gap_count as f64 / total as f64) * 100.0
    }
}

/// Alignment method used
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlignmentMethod {
    /// PTS values match exactly (delta = 0)
    PtsExact,
    /// PTS-based nearest-neighbor matching
    PtsNearest,
    /// Fallback to display_idx alignment
    DisplayIdx,
}

impl AlignmentMethod {
    /// Get display text
    pub fn display_text(&self) -> &'static str {
        match self {
            AlignmentMethod::PtsExact => "PTS Exact",
            AlignmentMethod::PtsNearest => "PTS Nearest",
            AlignmentMethod::DisplayIdx => "Display Index",
        }
    }
}

/// Alignment confidence level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlignmentConfidence {
    /// High confidence: perfect or near-perfect alignment
    High,
    /// Medium confidence: usable alignment with some gaps
    Medium,
    /// Low confidence: many gaps or fallback method
    Low,
}

impl AlignmentConfidence {
    /// Get display text
    pub fn display_text(&self) -> &'static str {
        match self {
            AlignmentConfidence::High => "High",
            AlignmentConfidence::Medium => "Medium",
            AlignmentConfidence::Low => "Low",
        }
    }

    /// Get tooltip explanation
    pub fn tooltip(&self) -> &'static str {
        match self {
            AlignmentConfidence::High => "Perfect or near-perfect frame alignment (<5% gaps)",
            AlignmentConfidence::Medium => "Usable alignment with some gaps (5-20%)",
            AlignmentConfidence::Low => "Many gaps or fallback alignment method (>20% gaps)",
        }
    }
}

/// Frame pair in alignment
///
/// Represents matched frames from two streams.
/// One or both indices may be None for gaps.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FramePair {
    /// Stream A frame index (None = gap in A)
    pub stream_a_idx: Option<usize>,

    /// Stream B frame index (None = gap in B)
    pub stream_b_idx: Option<usize>,

    /// PTS delta (A - B) in same units as PTS
    pub pts_delta: Option<i64>,

    /// Gap indicator (one stream missing frame)
    pub has_gap: bool,
}

impl FramePair {
    /// Check if this pair has both frames
    pub fn is_complete(&self) -> bool {
        self.stream_a_idx.is_some() && self.stream_b_idx.is_some()
    }

    /// Get PTS delta magnitude
    pub fn pts_delta_abs(&self) -> Option<u64> {
        self.pts_delta.map(|d| d.unsigned_abs())
    }
}

#[cfg(test)]
mod tests {
    include!("alignment_test.rs");
}
