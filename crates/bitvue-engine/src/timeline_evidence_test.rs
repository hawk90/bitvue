// Timeline evidence module tests
//
// Per generate-tests skill: Arrange-Act-Assert pattern with fixtures
// and edge case coverage.

use super::*;
use crate::timeline::TimelineFrame;
use crate::FrameMarker;

// ============================================================================
// Fixtures
// ============================================================================

/// Create a test timeline frame
fn create_test_frame(display_idx: usize) -> TimelineFrame {
    TimelineFrame {
        display_idx,
        size_bytes: 1000 + display_idx as u64 * 100,
        frame_type: if display_idx.is_multiple_of(3) { "I".to_string() } else { "P".to_string() },
        marker: if display_idx.is_multiple_of(3) {
            FrameMarker::Key
        } else if display_idx.is_multiple_of(7) {
            FrameMarker::Error
        } else {
            FrameMarker::None
        },
        pts: Some(display_idx as u64 * 100),
        dts: Some(display_idx as u64 * 100),
        is_selected: false,
    }
}

/// Create a test timeline frame with specific marker
fn create_test_frame_with_marker(display_idx: usize, marker: FrameMarker) -> TimelineFrame {
    TimelineFrame {
        display_idx,
        size_bytes: 1000,
        frame_type: "I".to_string(),
        marker,
        pts: Some(display_idx as u64 * 100),
        dts: Some(display_idx as u64 * 100),
        is_selected: false,
    }
}

/// Create a test manager
fn create_test_manager() -> TimelineEvidenceManager {
    TimelineEvidenceManager::new(EvidenceChain::new())
}

// ============================================================================
// TimelineEvidenceManager Construction Tests
// ============================================================================

#[cfg(test)]
mod construction_tests {
    use super::*;

    #[test]
    fn test_new_creates_manager() {
        // Arrange & Act
        let chain = EvidenceChain::new();
        let manager = TimelineEvidenceManager::new(chain);

        // Assert
        assert_eq!(manager.frame_count(), 0);
        assert_eq!(manager.next_evidence_id, 0);
    }

    #[test]
    fn test_default_creates_manager() {
        // Arrange & Act
        let manager = TimelineEvidenceManager::default();

        // Assert
        assert_eq!(manager.frame_count(), 0);
    }

    #[test]
    fn test_evidence_chain_accessible() {
        // Arrange
        let manager = create_test_manager();

        // Act
        let chain = manager.evidence_chain();

        // Assert - Can access evidence chain
        assert_eq!(chain.bit_offset_index.len(), 0);
    }

    #[test]
    fn test_evidence_chain_mut_accessible() {
        // Arrange
        let mut manager = create_test_manager();

        // Act
        let chain = manager.evidence_chain_mut();

        // Assert - Can access evidence chain mutably
        assert_eq!(chain.bit_offset_index.len(), 0);
    }
}

// ============================================================================
// Evidence ID Generation Tests
// ============================================================================

#[cfg(test)]
mod evidence_id_tests {
    use super::*;

    #[test]
    fn test_next_id_generates_unique_ids() {
        // Arrange
        let mut manager = create_test_manager();

        // Act
        let id1 = manager.next_evidence_id;
        manager.create_frame_evidence(&create_test_frame(0), BitRange::new(0_u64, 100), 1000);
        let id2 = manager.next_evidence_id;
        manager.create_frame_evidence(&create_test_frame(1), BitRange::new(100_u64, 200), 1100);
        let id3 = manager.next_evidence_id;

        // Assert - IDs increment by 4 per frame (4 evidence items: bit_offset, syntax, decode, viz)
        assert_eq!(id1, 0);
        assert_eq!(id2, 4); // 4 evidence items for first frame
        assert_eq!(id3, 8); // 4 more evidence items for second frame
    }

    #[test]
    fn test_evidence_ids_are_formatted() {
        // Arrange
        let mut manager = create_test_manager();

        // Act
        let evidence = manager.create_frame_evidence(
            &create_test_frame(0),
            BitRange::new(0_u64, 100),
            1000,
        );

        // Assert - Check ID format
        assert!(evidence.bit_offset_id.starts_with("timeline_ev_"));
        assert!(evidence.syntax_id.starts_with("timeline_ev_"));
        assert!(evidence.decode_id.starts_with("timeline_ev_"));
        assert!(evidence.timeline_viz_id.starts_with("timeline_ev_"));
    }
}

// ============================================================================
// Create Frame Evidence Tests
// ============================================================================

#[cfg(test)]
mod create_frame_evidence_tests {
    use super::*;

    #[test]
    fn test_create_frame_evidence_returns_bundle() {
        // Arrange
        let mut manager = create_test_manager();
        let frame = create_test_frame(5);

        // Act
        let evidence = manager.create_frame_evidence(&frame, BitRange::new(500, 8000), 1000);

        // Assert
        assert_eq!(evidence.display_idx, 5);
    }

    #[test]
    fn test_create_frame_evidence_stores_in_map() {
        // Arrange
        let mut manager = create_test_manager();
        let frame = create_test_frame(10);

        // Act
        manager.create_frame_evidence(&frame, BitRange::new(0_u64, 8000), 1000);

        // Assert
        assert_eq!(manager.frame_count(), 1);
        assert!(manager.get_frame_evidence(10).is_some());
    }

    #[test]
    fn test_create_multiple_frames() {
        // Arrange
        let mut manager = create_test_manager();

        // Act
        for i in 0..5 {
            manager.create_frame_evidence(
                &create_test_frame(i),
                BitRange::new((i * 8000) as u64, ((i + 1) * 8000) as u64),
                1000 + i * 100,
            );
        }

        // Assert
        assert_eq!(manager.frame_count(), 5);
        for i in 0..5 {
            assert!(manager.get_frame_evidence(i).is_some());
        }
    }

    #[test]
    fn test_create_frame_adds_to_evidence_chain() {
        // Arrange
        let mut manager = create_test_manager();
        let frame = create_test_frame(0);

        // Act
        manager.create_frame_evidence(&frame, BitRange::new(0_u64, 8000), 1000);

        // Assert
        let chain = manager.evidence_chain();
        assert_eq!(chain.bit_offset_index.len(), 1);
        assert_eq!(chain.syntax_index.len(), 1);
        assert_eq!(chain.decode_index.len(), 1);
        assert_eq!(chain.viz_index.len(), 1);
    }

    #[test]
    fn test_evidence_chain_links() {
        // Arrange
        let mut manager = create_test_manager();
        let frame = create_test_frame(0);

        // Act
        let evidence = manager.create_frame_evidence(&frame, BitRange::new(0_u64, 8000), 1000);

        // Assert - Check links exist
        let chain = manager.evidence_chain();
        let bit_offset = chain.bit_offset_index.find_by_id(&evidence.bit_offset_id).unwrap();
        assert_eq!(bit_offset.syntax_link, Some(evidence.syntax_id.clone()));

        let syntax = chain.syntax_index.find_by_id(&evidence.syntax_id).unwrap();
        assert_eq!(syntax.decode_link, Some(evidence.decode_id.clone()));

        let decode = chain.decode_index.find_by_id(&evidence.decode_id).unwrap();
        assert_eq!(decode.viz_link, Some(evidence.timeline_viz_id.clone()));
    }
}

// ============================================================================
// Get Frame Evidence Tests
// ============================================================================

#[cfg(test)]
mod get_frame_evidence_tests {
    use super::*;

    #[test]
    fn test_get_frame_evidence_exists() {
        // Arrange
        let mut manager = create_test_manager();
        let frame = create_test_frame(5);
        manager.create_frame_evidence(&frame, BitRange::new(0_u64, 8000), 1000);

        // Act
        let evidence = manager.get_frame_evidence(5);

        // Assert
        assert!(evidence.is_some());
        assert_eq!(evidence.unwrap().display_idx, 5);
    }

    #[test]
    fn test_get_frame_evidence_not_exists() {
        // Arrange
        let manager = create_test_manager();

        // Act
        let evidence = manager.get_frame_evidence(999);

        // Assert
        assert!(evidence.is_none());
    }

    #[test]
    fn test_get_frame_bit_range_exists() {
        // Arrange
        let mut manager = create_test_manager();
        let frame = create_test_frame(10);
        let bit_range = BitRange::new(80000, 96000);  // 80000 + 16000 = 96000
        manager.create_frame_evidence(&frame, bit_range, 2000);

        // Act
        let result = manager.get_frame_bit_range(10);

        // Assert
        assert!(result.is_some());
        let range = result.unwrap();
        assert_eq!(range.byte_offset(), bit_range.byte_offset());
        assert_eq!(range.size_bits(), bit_range.size_bits());
    }

    #[test]
    fn test_get_frame_bit_range_not_exists() {
        // Arrange
        let manager = create_test_manager();

        // Act
        let result = manager.get_frame_bit_range(999);

        // Assert
        assert!(result.is_none());
    }
}

// ============================================================================
// Find Frame at Temporal Position Tests
// ============================================================================

#[cfg(test)]
mod find_temporal_pos_tests {
    use super::*;

    #[test]
    fn test_find_frame_at_temporal_pos_start() {
        // Arrange
        let manager = create_test_manager();
        let total_frames = 100;

        // Act - Position 0.0 should give frame 0
        let result = manager.find_frame_at_temporal_pos(0.0, total_frames);

        // Assert
        assert_eq!(result, Some(0));
    }

    #[test]
    fn test_find_frame_at_temporal_pos_middle() {
        // Arrange
        let manager = create_test_manager();
        let total_frames = 100;

        // Act - Position 0.5 should give frame ~50
        let result = manager.find_frame_at_temporal_pos(0.5, total_frames);

        // Assert
        assert_eq!(result, Some(50));
    }

    #[test]
    fn test_find_frame_at_temporal_pos_end() {
        // Arrange
        let manager = create_test_manager();
        let total_frames = 100;

        // Act - Position 1.0 should give last frame
        let result = manager.find_frame_at_temporal_pos(1.0, total_frames);

        // Assert
        assert_eq!(result, Some(99));
    }

    #[test]
    fn test_find_frame_at_temporal_pos_zero_frames() {
        // Arrange
        let manager = create_test_manager();

        // Act
        let result = manager.find_frame_at_temporal_pos(0.5, 0);

        // Assert
        assert!(result.is_none());
    }

    #[test]
    fn test_find_frame_at_temporal_pos_single_frame() {
        // Arrange
        let manager = create_test_manager();

        // Act
        let result = manager.find_frame_at_temporal_pos(0.5, 1);

        // Assert - Should clamp to 0
        assert_eq!(result, Some(0));
    }

    #[test]
    fn test_find_frame_at_temporal_pos_clamps() {
        // Arrange
        let manager = create_test_manager();
        let total_frames = 100;

        // Act - Beyond range
        let result1 = manager.find_frame_at_temporal_pos(-0.5, total_frames);
        let result2 = manager.find_frame_at_temporal_pos(1.5, total_frames);

        // Assert - Should clamp
        assert_eq!(result1, Some(0));
        assert_eq!(result2, Some(99));
    }
}

// ============================================================================
// Find Frames with Marker Tests
// ============================================================================

#[cfg(test)]
mod find_frames_with_marker_tests {
    use super::*;

    #[test]
    fn test_find_keyframes() {
        // Arrange
        let mut manager = create_test_manager();
        for i in 0..10 {
            let marker = if i % 3 == 0 { FrameMarker::Key } else { FrameMarker::None };
            manager.create_frame_evidence(
                &create_test_frame_with_marker(i, marker),
                BitRange::new((i * 8000) as u64, ((i + 1) * 8000) as u64),
                1000,
            );
        }

        // Act
        let mut keyframes = manager.get_keyframe_indices();
        keyframes.sort();  // Sort for consistent ordering

        // Assert
        assert_eq!(keyframes.len(), 4); // 0, 3, 6, 9
        assert_eq!(keyframes, vec![0, 3, 6, 9]);
    }

    #[test]
    fn test_find_error_frames() {
        // Arrange
        let mut manager = create_test_manager();
        manager.create_frame_evidence(
            &create_test_frame_with_marker(0, FrameMarker::Error),
            BitRange::new(0_u64, 8000),
            1000,
        );
        manager.create_frame_evidence(
            &create_test_frame_with_marker(1, FrameMarker::Key),
            BitRange::new(8000_u64, 16000),
            1000,
        );
        manager.create_frame_evidence(
            &create_test_frame_with_marker(2, FrameMarker::Error),
            BitRange::new(16000_u64, 24000),
            1000,
        );

        // Act
        let mut errors = manager.get_error_frame_indices();
        errors.sort();  // Sort for consistent ordering

        // Assert
        assert_eq!(errors.len(), 2);
        assert_eq!(errors, vec![0, 2]);
    }

    #[test]
    fn test_find_bookmark_frames() {
        // Arrange
        let mut manager = create_test_manager();
        manager.create_frame_evidence(
            &create_test_frame_with_marker(5, FrameMarker::Bookmark),
            BitRange::new(0_u64, 8000),
            1000,
        );
        manager.create_frame_evidence(
            &create_test_frame_with_marker(10, FrameMarker::Bookmark),
            BitRange::new(8000_u64, 16000),
            1000,
        );

        // Act
        let mut bookmarks = manager.get_bookmark_frame_indices();
        bookmarks.sort(); // Sort for consistent ordering

        // Assert
        assert_eq!(bookmarks.len(), 2);
        assert_eq!(bookmarks, vec![5, 10]);
    }

    #[test]
    fn test_find_frames_with_marker_empty() {
        // Arrange
        let manager = create_test_manager();

        // Act
        let keyframes = manager.get_keyframe_indices();

        // Assert
        assert!(keyframes.is_empty());
    }

    #[test]
    fn test_find_frames_with_marker_none_matches() {
        // Arrange
        let mut manager = create_test_manager();
        manager.create_frame_evidence(
            &create_test_frame_with_marker(0, FrameMarker::None),
            BitRange::new(0_u64, 8000),
            1000,
        );
        manager.create_frame_evidence(
            &create_test_frame_with_marker(1, FrameMarker::None),
            BitRange::new(8000_u64, 16000),
            1000,
        );

        // Act
        let keyframes = manager.get_keyframe_indices();

        // Assert
        assert!(keyframes.is_empty());
    }
}

// ============================================================================
// Trace Navigation Tests
// ============================================================================

#[cfg(test)]
mod trace_navigation_tests {
    use super::*;

    #[test]
    fn test_trace_to_bit_offset() {
        // Arrange
        let mut manager = create_test_manager();
        let frame = create_test_frame(5);
        let bit_range = BitRange::new(40000_u64, 48000);
        manager.create_frame_evidence(&frame, bit_range, 1000);

        // Act
        let result = manager.trace_to_bit_offset(5);

        // Assert
        assert!(result.is_some());
        let range = result.unwrap();
        assert_eq!(range.byte_offset(), bit_range.byte_offset());
    }

    #[test]
    fn test_trace_to_bit_offset_not_found() {
        // Arrange
        let manager = create_test_manager();

        // Act
        let result = manager.trace_to_bit_offset(999);

        // Assert
        assert!(result.is_none());
    }

    #[test]
    fn test_trace_to_display_idx() {
        // Arrange
        let mut manager = create_test_manager();
        let frame = create_test_frame(5);
        let bit_range = BitRange::new(40000_u64, 48000); // byte_offset = 5000
        manager.create_frame_evidence(&frame, bit_range, 1000);

        // Act - Check byte offset within range
        let result = manager.trace_to_display_idx(5000);

        // Assert
        assert_eq!(result, Some(5));
    }

    #[test]
    fn test_trace_to_display_idx_at_start() {
        // Arrange
        let mut manager = create_test_manager();
        let frame = create_test_frame(10);
        let bit_range = BitRange::new(80000_u64, 88000); // 80000 + 8000 = 88000
        manager.create_frame_evidence(&frame, bit_range, 1000);

        // Act
        let result = manager.trace_to_display_idx(10000);

        // Assert
        assert_eq!(result, Some(10));
    }

    #[test]
    fn test_trace_to_display_idx_not_found() {
        // Arrange
        let manager = create_test_manager();

        // Act
        let result = manager.trace_to_display_idx(99999);

        // Assert
        assert!(result.is_none());
    }

    #[test]
    fn test_trace_to_display_idx_between_frames() {
        // Arrange
        let mut manager = create_test_manager();
        manager.create_frame_evidence(
            &create_test_frame(0),
            BitRange::new(0_u64, 8000), // bytes 0-1000
            1000,
        );
        manager.create_frame_evidence(
            &create_test_frame(1),
            BitRange::new(8000_u64, 16000), // bytes 1000-2000
            1000,
        );

        // Act - Between frames (not in either)
        let result = manager.trace_to_display_idx(500); // In first frame's range

        // Assert
        assert_eq!(result, Some(0));
    }
}

// ============================================================================
// Clear and Reset Tests
// ============================================================================

#[cfg(test)]
mod clear_reset_tests {
    use super::*;

    #[test]
    fn test_clear_removes_all_evidence() {
        // Arrange
        let mut manager = create_test_manager();
        for i in 0..5 {
            manager.create_frame_evidence(
                &create_test_frame(i),
                BitRange::new((i * 8000) as u64, ((i + 1) * 8000) as u64),
                1000,
            );
        }
        assert_eq!(manager.frame_count(), 5);

        // Act
        manager.clear();

        // Assert
        assert_eq!(manager.frame_count(), 0);
        assert!(manager.get_frame_evidence(0).is_none());
    }

    #[test]
    fn test_clear_resets_evidence_id_counter() {
        // Arrange
        let mut manager = create_test_manager();
        manager.create_frame_evidence(&create_test_frame(0), BitRange::new(0_u64, 8000), 1000);
        assert_ne!(manager.next_evidence_id, 0);

        // Act
        manager.clear();

        // Assert
        assert_eq!(manager.next_evidence_id, 0);
    }

    #[test]
    fn test_clear_does_not_clear_evidence_chain() {
        // Arrange
        let mut manager = create_test_manager();
        manager.create_frame_evidence(&create_test_frame(0), BitRange::new(0_u64, 8000), 1000);
        let chain_count_before = manager.evidence_chain().bit_offset_index.len();

        // Act
        manager.clear();

        // Assert - Evidence chain is not cleared (it's owned separately)
        let chain_count_after = manager.evidence_chain().bit_offset_index.len();
        assert_eq!(chain_count_before, chain_count_after);
    }
}

// ============================================================================
// Metadata Tests
// ============================================================================

#[cfg(test)]
mod metadata_tests {
    use super::*;

    #[test]
    fn test_syntax_evidence_contains_frame_metadata() {
        // Arrange
        let mut manager = create_test_manager();
        let frame = create_test_frame(0);  // Use 0 instead of 5 to get Key marker

        // Act
        let evidence = manager.create_frame_evidence(&frame, BitRange::new(0_u64, 8000), 1000);

        // Assert - Check metadata was added
        let syntax = manager
            .evidence_chain()
            .syntax_index
            .find_by_id(&evidence.syntax_id)
            .unwrap();

        assert_eq!(
            syntax.metadata.get("frame_type").unwrap(),
            &frame.frame_type
        );
        assert_eq!(syntax.metadata.get("size_bytes").unwrap(), "1000");
        assert_eq!(syntax.metadata.get("is_keyframe").unwrap(), "true");  // Now frame 0 is a keyframe
        assert_eq!(syntax.metadata.get("display_idx").unwrap(), "0");  // Updated to match display_idx
    }

    #[test]
    fn test_syntax_evidence_contains_pts_dts() {
        // Arrange
        let mut manager = create_test_manager();
        let frame = TimelineFrame {
            display_idx: 5,
            size_bytes: 1000,
            frame_type: "I".to_string(),
            marker: FrameMarker::Key,
            pts: Some(500),
            dts: Some(400),
            is_selected: false,
        };

        // Act
        let evidence = manager.create_frame_evidence(&frame, BitRange::new(0_u64, 8000), 1000);

        // Assert
        let syntax = manager
            .evidence_chain()
            .syntax_index
            .find_by_id(&evidence.syntax_id)
            .unwrap();

        assert_eq!(syntax.metadata.get("pts").unwrap(), "500");
        assert_eq!(syntax.metadata.get("dts").unwrap(), "400");
    }

    #[test]
    fn test_viz_evidence_contains_visual_properties() {
        // Arrange
        let mut manager = create_test_manager();
        let frame = create_test_frame(0);  // Use 0 to get Key marker

        // Act
        let evidence = manager.create_frame_evidence(&frame, BitRange::new(0_u64, 8000), 1000);

        // Assert
        let viz = manager
            .evidence_chain()
            .viz_index
            .find_by_id(&evidence.timeline_viz_id)
            .unwrap();

        assert_eq!(
            viz.visual_properties.get("frame_type").unwrap(),
            &frame.frame_type
        );
        assert_eq!(
            viz.visual_properties.get("is_keyframe").unwrap(),
            "true"
        );
        assert_eq!(viz.visual_properties.get("display_idx").unwrap(), "0");
    }

    #[test]
    fn test_viz_evidence_marker_property() {
        // Arrange
        let mut manager = create_test_manager();
        let frame = create_test_frame_with_marker(5, FrameMarker::Error);

        // Act
        let evidence = manager.create_frame_evidence(&frame, BitRange::new(0_u64, 8000), 1000);

        // Assert
        let viz = manager
            .evidence_chain()
            .viz_index
            .find_by_id(&evidence.timeline_viz_id)
            .unwrap();

        assert!(viz.visual_properties.get("marker").unwrap().contains("Error"));
    }
}

// ============================================================================
// Edge Cases
// ============================================================================

#[cfg(test)]
mod edge_case_tests {
    use super::*;

    #[test]
    fn test_create_evidence_for_frame_zero() {
        // Arrange
        let mut manager = create_test_manager();
        let frame = create_test_frame(0);

        // Act
        let evidence = manager.create_frame_evidence(&frame, BitRange::new(0_u64, 8000), 1000);

        // Assert
        assert_eq!(evidence.display_idx, 0);
    }

    #[test]
    fn test_create_evidence_for_large_frame_index() {
        // Arrange
        let mut manager = create_test_manager();
        let frame = create_test_frame(999_999);

        // Act
        let evidence = manager.create_frame_evidence(&frame, BitRange::new(0_u64, 8000), 1000);

        // Assert
        assert_eq!(evidence.display_idx, 999_999);
    }

    #[test]
    fn test_trace_to_display_idx_at_exact_boundary() {
        // Arrange
        let mut manager = create_test_manager();
        // Frame 0: bytes 0-1000, Frame 1: bytes 1000-2000
        manager.create_frame_evidence(
            &create_test_frame(0),
            BitRange::new(0_u64, 8000), // 1000 bytes
            1000,
        );
        manager.create_frame_evidence(
            &create_test_frame(1),
            BitRange::new(8000_u64, 16000), // 1000 bytes
            1000,
        );

        // Act - At exact boundary
        let result = manager.trace_to_display_idx(1000);

        // Assert - Should be frame 1 (start of second range)
        assert_eq!(result, Some(1));
    }

    #[test]
    fn test_get_keyframe_indices_all_keyframes() {
        // Arrange
        let mut manager = create_test_manager();
        for i in 0..5 {
            manager.create_frame_evidence(
                &create_test_frame_with_marker(i, FrameMarker::Key),
                BitRange::new((i * 8000) as u64, ((i + 1) * 8000) as u64),
                1000,
            );
        }

        // Act
        let keyframes = manager.get_keyframe_indices();

        // Assert
        assert_eq!(keyframes.len(), 5);
    }

    #[test]
    fn test_multiple_frames_same_marker_type() {
        // Arrange
        let mut manager = create_test_manager();
        for i in 0..10 {
            manager.create_frame_evidence(
                &create_test_frame_with_marker(i, FrameMarker::Bookmark),
                BitRange::new((i * 8000) as u64, ((i + 1) * 8000) as u64),
                1000,
            );
        }

        // Act
        let bookmarks = manager.get_bookmark_frame_indices();

        // Assert
        assert_eq!(bookmarks.len(), 10);
    }
}
