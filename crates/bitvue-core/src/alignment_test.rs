// Alignment engine module tests
//
// Per generate-tests skill: Arrange-Act-Assert pattern with fixtures
// and edge case coverage.

use super::*;
use crate::frame_identity::{FrameIndexMap, FrameMetadata, PtsQuality};

// ============================================================================
// Fixtures
// ============================================================================

/// Create test frames with PTS
fn create_test_frames_with_pts(count: usize, start_pts: u64, pts_delta: u64) -> Vec<FrameMetadata> {
    (0..count)
        .map(|i| FrameMetadata {
            pts: Some(start_pts + i as u64 * pts_delta),
            dts: Some(i as u64 * pts_delta),
        })
        .collect()
}

/// Create test frames without PTS
fn create_test_frames_no_pts(count: usize) -> Vec<FrameMetadata> {
    (0..count)
        .map(|i| FrameMetadata {
            pts: None,
            dts: Some(i as u64 * 33),
        })
        .collect()
}

/// Create test FrameIndexMap with PTS
fn create_test_map_with_pts(count: usize, start_pts: u64, pts_delta: u64) -> FrameIndexMap {
    let frames = create_test_frames_with_pts(count, start_pts, pts_delta);
    FrameIndexMap::new(&frames)
}

/// Create test FrameIndexMap without PTS
fn create_test_map_no_pts(count: usize) -> FrameIndexMap {
    let frames = create_test_frames_no_pts(count);
    FrameIndexMap::new(&frames)
}

// ============================================================================
// AlignmentMethod Tests
// ============================================================================

#[cfg(test)]
mod alignment_method_tests {
    use super::*;

    #[test]
    fn test_alignment_method_pts_exact_display() {
        // Arrange & Act
        let method = AlignmentMethod::PtsExact;

        // Assert
        assert_eq!(method.display_text(), "PTS Exact");
    }

    #[test]
    fn test_alignment_method_pts_nearest_display() {
        // Arrange & Act
        let method = AlignmentMethod::PtsNearest;

        // Assert
        assert_eq!(method.display_text(), "PTS Nearest");
    }

    #[test]
    fn test_alignment_method_display_idx_display() {
        // Arrange & Act
        let method = AlignmentMethod::DisplayIdx;

        // Assert
        assert_eq!(method.display_text(), "Display Index");
    }
}

// ============================================================================
// AlignmentConfidence Tests
// ============================================================================

#[cfg(test)]
mod alignment_confidence_tests {
    use super::*;

    #[test]
    fn test_alignment_confidence_high_display() {
        // Arrange & Act
        let confidence = AlignmentConfidence::High;

        // Assert
        assert_eq!(confidence.display_text(), "High");
        assert!(confidence.tooltip().contains("<5% gaps"));
    }

    #[test]
    fn test_alignment_confidence_medium_display() {
        // Arrange & Act
        let confidence = AlignmentConfidence::Medium;

        // Assert
        assert_eq!(confidence.display_text(), "Medium");
        assert!(confidence.tooltip().contains("5-20%"));
    }

    #[test]
    fn test_alignment_confidence_low_display() {
        // Arrange & Act
        let confidence = AlignmentConfidence::Low;

        // Assert
        assert_eq!(confidence.display_text(), "Low");
        assert!(confidence.tooltip().contains(">20%"));
    }
}

// ============================================================================
// FramePair Tests
// ============================================================================

#[cfg(test)]
mod frame_pair_tests {
    use super::*;

    #[test]
    fn test_frame_pair_complete() {
        // Arrange & Act
        let pair = FramePair {
            stream_a_idx: Some(0),
            stream_b_idx: Some(0),
            pts_delta: Some(0),
            has_gap: false,
        };

        // Assert
        assert!(pair.is_complete());
        assert!(!pair.has_gap);
    }

    #[test]
    fn test_frame_pair_gap_a() {
        // Arrange & Act
        let pair = FramePair {
            stream_a_idx: None,
            stream_b_idx: Some(0),
            pts_delta: None,
            has_gap: true,
        };

        // Assert
        assert!(!pair.is_complete());
        assert!(pair.has_gap);
    }

    #[test]
    fn test_frame_pair_gap_b() {
        // Arrange & Act
        let pair = FramePair {
            stream_a_idx: Some(0),
            stream_b_idx: None,
            pts_delta: None,
            has_gap: true,
        };

        // Assert
        assert!(!pair.is_complete());
        assert!(pair.has_gap);
    }

    #[test]
    fn test_frame_pair_pts_delta_abs() {
        // Arrange
        let pair = FramePair {
            stream_a_idx: Some(0),
            stream_b_idx: Some(0),
            pts_delta: Some(-100),
            has_gap: false,
        };

        // Act
        let delta_abs = pair.pts_delta_abs();

        // Assert
        assert_eq!(delta_abs, Some(100));
    }

    #[test]
    fn test_frame_pair_pts_delta_abs_none() {
        // Arrange
        let pair = FramePair {
            stream_a_idx: None,
            stream_b_idx: Some(0),
            pts_delta: None,
            has_gap: true,
        };

        // Act
        let delta_abs = pair.pts_delta_abs();

        // Assert
        assert!(delta_abs.is_none());
    }
}

// ============================================================================
// AlignmentEngine Construction Tests
// ============================================================================

#[cfg(test)]
mod alignment_engine_construction_tests {
    use super::*;

    #[test]
    fn test_alignment_engine_new_with_pts() {
        // Arrange
        let stream_a = create_test_map_with_pts(10, 1000, 33);
        let stream_b = create_test_map_with_pts(10, 1000, 33);

        // Act
        let engine = AlignmentEngine::new(&stream_a, &stream_b);

        // Assert
        assert_eq!(engine.stream_a_count, 10);
        assert_eq!(engine.stream_b_count, 10);
        assert_eq!(engine.method, AlignmentMethod::PtsExact);
        assert_eq!(engine.confidence, AlignmentConfidence::High);
        assert_eq!(engine.gap_count, 0);
    }

    #[test]
    fn test_alignment_engine_new_without_pts() {
        // Arrange
        let stream_a = create_test_map_no_pts(10);
        let stream_b = create_test_map_no_pts(10);

        // Act
        let engine = AlignmentEngine::new(&stream_a, &stream_b);

        // Assert
        assert_eq!(engine.stream_a_count, 10);
        assert_eq!(engine.stream_b_count, 10);
        assert_eq!(engine.method, AlignmentMethod::DisplayIdx);
        // DisplayIdx with no gaps = Medium confidence
        assert_eq!(engine.confidence, AlignmentConfidence::Medium);
        assert_eq!(engine.gap_count, 0);
    }

    #[test]
    fn test_alignment_engine_new_different_lengths() {
        // Arrange
        let stream_a = create_test_map_with_pts(10, 1000, 33);
        let stream_b = create_test_map_with_pts(5, 1000, 33);

        // Act
        let engine = AlignmentEngine::new(&stream_a, &stream_b);

        // Assert
        assert_eq!(engine.stream_a_count, 10);
        assert_eq!(engine.stream_b_count, 5);
        // Should have gaps for extra frames in A
        assert!(engine.gap_count > 0);
    }
}

// ============================================================================
// PTS Alignment Tests
// ============================================================================

#[cfg(test)]
mod pts_alignment_tests {
    use super::*;

    #[test]
    fn test_align_by_pts_exact_match() {
        // Arrange
        let stream_a = create_test_map_with_pts(10, 1000, 33);
        let stream_b = create_test_map_with_pts(10, 1000, 33);

        // Act
        let engine = AlignmentEngine::new(&stream_a, &stream_b);

        // Assert
        assert_eq!(engine.method, AlignmentMethod::PtsExact);
        assert_eq!(engine.gap_count, 0);
        assert_eq!(engine.frame_pairs.len(), 10);
    }

    #[test]
    fn test_align_by_pts_offset_match() {
        // Arrange
        let stream_a = create_test_map_with_pts(10, 1000, 33);
        let stream_b = create_test_map_with_pts(10, 1033, 33); // 1 frame offset

        // Act
        let engine = AlignmentEngine::new(&stream_a, &stream_b);

        // Assert
        assert_eq!(engine.method, AlignmentMethod::PtsNearest);
        // Some frames should match within threshold
        assert!(!engine.frame_pairs.is_empty());
    }

    #[test]
    fn test_align_by_pts_gaps_in_shorter_stream() {
        // Arrange
        let stream_a = create_test_map_with_pts(10, 1000, 33);
        let stream_b = create_test_map_with_pts(5, 1000, 33);

        // Act
        let engine = AlignmentEngine::new(&stream_a, &stream_b);

        // Assert
        assert!(engine.gap_count > 0);
        // All 5 B frames should be matched
        let b_matched = engine.frame_pairs.iter()
            .filter(|p| p.stream_b_idx.is_some())
            .count();
        assert_eq!(b_matched, 5);
    }

    #[test]
    fn test_align_by_pts_large_offset_no_match() {
        // Arrange
        let stream_a = create_test_map_with_pts(10, 1000, 33);
        let stream_b = create_test_map_with_pts(10, 10000, 33); // Large offset

        // Act
        let engine = AlignmentEngine::new(&stream_a, &stream_b);

        // Assert
        // With large offset, most/all frames should be gaps
        assert!(engine.gap_count >= 10);
    }
}

// ============================================================================
// Display Index Alignment Tests
// ============================================================================

#[cfg(test)]
mod display_idx_alignment_tests {
    use super::*;

    #[test]
    fn test_align_by_display_idx_same_length() {
        // Arrange
        let stream_a = create_test_map_no_pts(10);
        let stream_b = create_test_map_no_pts(10);

        // Act
        let engine = AlignmentEngine::new(&stream_a, &stream_b);

        // Assert
        assert_eq!(engine.method, AlignmentMethod::DisplayIdx);
        assert_eq!(engine.gap_count, 0);
        assert_eq!(engine.frame_pairs.len(), 10);
    }

    #[test]
    fn test_align_by_display_idx_different_lengths() {
        // Arrange
        let stream_a = create_test_map_no_pts(10);
        let stream_b = create_test_map_no_pts(5);

        // Act
        let engine = AlignmentEngine::new(&stream_a, &stream_b);

        // Assert
        assert_eq!(engine.method, AlignmentMethod::DisplayIdx);
        assert_eq!(engine.frame_pairs.len(), 10); // Max length
        assert_eq!(engine.gap_count, 5); // 5 pairs have gaps (indices 5-9)
    }

    #[test]
    fn test_align_by_display_idx_confidence() {
        // Arrange
        let stream_a = create_test_map_no_pts(10);
        let stream_b = create_test_map_no_pts(10);

        // Act
        let engine = AlignmentEngine::new(&stream_a, &stream_b);

        // Assert
        assert_eq!(engine.confidence, AlignmentConfidence::Medium);
    }
}

// ============================================================================
// Confidence Calculation Tests
// ============================================================================

#[cfg(test)]
mod confidence_calculation_tests {
    use super::*;

    #[test]
    fn test_confidence_high_exact_pts() {
        // Arrange
        let stream_a = create_test_map_with_pts(100, 1000, 33);
        let stream_b = create_test_map_with_pts(100, 1000, 33);

        // Act
        let engine = AlignmentEngine::new(&stream_a, &stream_b);

        // Assert
        assert_eq!(engine.confidence, AlignmentConfidence::High);
    }

    #[test]
    fn test_confidence_high_nearest_pts_low_gaps() {
        // Arrange
        let stream_a = create_test_map_with_pts(100, 1000, 33);
        let stream_b = create_test_map_with_pts(98, 1000, 33); // Small gap

        // Act
        let engine = AlignmentEngine::new(&stream_a, &stream_b);

        // Assert
        // <5% gaps with PTS = High confidence
        assert_eq!(engine.confidence, AlignmentConfidence::High);
    }

    #[test]
    fn test_confidence_medium_nearest_pts_moderate_gaps() {
        // Arrange
        let stream_a = create_test_map_with_pts(100, 1000, 33);
        let stream_b = create_test_map_with_pts(85, 1000, 33); // 15% gap (within Medium range)

        // Act
        let engine = AlignmentEngine::new(&stream_a, &stream_b);

        // Assert
        // 5-20% gaps with PTS = Medium confidence
        assert_eq!(engine.confidence, AlignmentConfidence::Medium);
    }

    #[test]
    fn test_confidence_low_nearest_pts_high_gaps() {
        // Arrange
        let stream_a = create_test_map_with_pts(100, 1000, 33);
        let stream_b = create_test_map_with_pts(70, 1000, 33); // 30% gap

        // Act
        let engine = AlignmentEngine::new(&stream_a, &stream_b);

        // Assert
        // >20% gaps with PTS = Low confidence
        assert_eq!(engine.confidence, AlignmentConfidence::Low);
    }

    #[test]
    fn test_confidence_medium_display_idx_low_gaps() {
        // Arrange
        let stream_a = create_test_map_no_pts(100);
        let stream_b = create_test_map_no_pts(98);

        // Act
        let engine = AlignmentEngine::new(&stream_a, &stream_b);

        // Assert
        // DisplayIdx with <5% gaps = Medium
        assert_eq!(engine.confidence, AlignmentConfidence::Medium);
    }
}

// ============================================================================
// Gap Percentage Tests
// ============================================================================

#[cfg(test)]
mod gap_percentage_tests {
    use super::*;

    #[test]
    fn test_gap_percentage_no_gaps() {
        // Arrange
        let stream_a = create_test_map_with_pts(10, 1000, 33);
        let stream_b = create_test_map_with_pts(10, 1000, 33);

        // Act
        let engine = AlignmentEngine::new(&stream_a, &stream_b);

        // Assert
        assert_eq!(engine.gap_percentage(), 0.0);
    }

    #[test]
    fn test_gap_percentage_half_gaps() {
        // Arrange
        let stream_a = create_test_map_with_pts(10, 1000, 33);
        let stream_b = create_test_map_with_pts(5, 1000, 33);

        // Act
        let engine = AlignmentEngine::new(&stream_a, &stream_b);

        // Assert
        // 5 gaps out of 10 = 50%
        assert!(engine.gap_percentage() > 45.0 && engine.gap_percentage() < 55.0);
    }

    #[test]
    fn test_gap_percentage_empty_streams() {
        // Arrange
        let stream_a = create_test_map_with_pts(0, 0, 0);
        let stream_b = create_test_map_with_pts(0, 0, 0);

        // Act
        let engine = AlignmentEngine::new(&stream_a, &stream_b);

        // Assert
        assert_eq!(engine.gap_percentage(), 0.0);
    }
}

// ============================================================================
// Get Pair Tests
// ============================================================================

#[cfg(test)]
mod get_pair_tests {
    use super::*;

    #[test]
    fn test_get_pair_for_a_existing() {
        // Arrange
        let stream_a = create_test_map_with_pts(10, 1000, 33);
        let stream_b = create_test_map_with_pts(10, 1000, 33);
        let engine = AlignmentEngine::new(&stream_a, &stream_b);

        // Act
        let pair = engine.get_pair_for_a(0);

        // Assert
        assert!(pair.is_some());
        let p = pair.unwrap();
        assert_eq!(p.stream_a_idx, Some(0));
        assert_eq!(p.stream_b_idx, Some(0));
    }

    #[test]
    fn test_get_pair_for_a_not_found() {
        // Arrange
        let stream_a = create_test_map_with_pts(10, 1000, 33);
        let stream_b = create_test_map_with_pts(5, 1000, 33);
        let engine = AlignmentEngine::new(&stream_a, &stream_b);

        // Act
        let pair = engine.get_pair_for_a(9);

        // Assert
        assert!(pair.is_some()); // Frame exists but has gap
        let p = pair.unwrap();
        assert_eq!(p.stream_a_idx, Some(9));
        assert_eq!(p.stream_b_idx, None);
    }

    #[test]
    fn test_get_pair_for_b_existing() {
        // Arrange
        let stream_a = create_test_map_with_pts(10, 1000, 33);
        let stream_b = create_test_map_with_pts(10, 1000, 33);
        let engine = AlignmentEngine::new(&stream_a, &stream_b);

        // Act
        let pair = engine.get_pair_for_b(5);

        // Assert
        assert!(pair.is_some());
        let p = pair.unwrap();
        assert_eq!(p.stream_a_idx, Some(5));
        assert_eq!(p.stream_b_idx, Some(5));
    }

    #[test]
    fn test_get_pair_for_b_not_found() {
        // Arrange
        let stream_a = create_test_map_with_pts(5, 1000, 33);
        let stream_b = create_test_map_with_pts(10, 1000, 33);
        let engine = AlignmentEngine::new(&stream_a, &stream_b);

        // Act
        let pair = engine.get_pair_for_b(9);

        // Assert
        assert!(pair.is_some()); // Frame exists but has gap
        let p = pair.unwrap();
        assert_eq!(p.stream_a_idx, None);
        assert_eq!(p.stream_b_idx, Some(9));
    }
}

// ============================================================================
// Edge Cases
// ============================================================================

#[cfg(test)]
mod edge_case_tests {
    use super::*;

    #[test]
    fn test_alignment_empty_streams() {
        // Arrange
        let stream_a = create_test_map_with_pts(0, 0, 0);
        let stream_b = create_test_map_with_pts(0, 0, 0);

        // Act - Should not panic
        let engine = AlignmentEngine::new(&stream_a, &stream_b);

        // Assert
        assert_eq!(engine.stream_a_count, 0);
        assert_eq!(engine.stream_b_count, 0);
        assert_eq!(engine.confidence, AlignmentConfidence::High);
        assert_eq!(engine.gap_percentage(), 0.0);
    }

    #[test]
    fn test_alignment_single_frame_streams() {
        // Arrange
        let stream_a = create_test_map_with_pts(1, 1000, 33);
        let stream_b = create_test_map_with_pts(1, 1000, 33);

        // Act
        let engine = AlignmentEngine::new(&stream_a, &stream_b);

        // Assert
        assert_eq!(engine.stream_a_count, 1);
        assert_eq!(engine.stream_b_count, 1);
        assert_eq!(engine.gap_count, 0);
    }

    #[test]
    fn test_alignment_large_streams() {
        // Arrange
        let stream_a = create_test_map_with_pts(10000, 1000, 33);
        let stream_b = create_test_map_with_pts(10000, 1000, 33);

        // Act - Should not panic
        let engine = AlignmentEngine::new(&stream_a, &stream_b);

        // Assert
        assert_eq!(engine.stream_a_count, 10000);
        assert_eq!(engine.stream_b_count, 10000);
        assert_eq!(engine.confidence, AlignmentConfidence::High);
    }

    #[test]
    fn test_alignment_zero_pts_delta() {
        // Arrange - All frames have same PTS (edge case)
        let stream_a = create_test_map_with_pts(10, 1000, 0); // Delta = 0
        let stream_b = create_test_map_with_pts(10, 1000, 0);

        // Act - Should not panic
        let engine = AlignmentEngine::new(&stream_a, &stream_b);

        // Assert
        // Should still produce alignment
        assert!(!engine.frame_pairs.is_empty());
    }

    #[test]
    fn test_alignment_very_large_pts_delta() {
        // Arrange
        let stream_a = create_test_map_with_pts(10, 0, u64::MAX / 20);
        let stream_b = create_test_map_with_pts(10, 0, u64::MAX / 20);

        // Act - Should not panic
        let engine = AlignmentEngine::new(&stream_a, &stream_b);

        // Assert
        assert!(!engine.frame_pairs.is_empty());
    }

    #[test]
    fn test_alignment_pts_overflow_safe() {
        // Arrange - Frame that could cause overflow in delta calculation
        let mut frames_a = Vec::new();
        let mut frames_b = Vec::new();

        frames_a.push(FrameMetadata {
            pts: Some(u64::MAX),
            dts: Some(0),
        });
        frames_b.push(FrameMetadata {
            pts: Some(0),
            dts: Some(0),
        });

        let stream_a = FrameIndexMap::new(&frames_a);
        let stream_b = FrameIndexMap::new(&frames_b);

        // Act - Should not panic despite large delta
        let engine = AlignmentEngine::new(&stream_a, &stream_b);

        // Assert
        assert!(!engine.frame_pairs.is_empty());
    }
}
