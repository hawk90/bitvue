// Index extractor evidence module tests
//
// Per generate-tests skill: Arrange-Act-Assert pattern with fixtures
// and edge case coverage.

use super::*;
use crate::indexing::{FrameMetadata, SeekPoint};

// ============================================================================
// Fixtures
// ============================================================================

/// Create a test evidence manager
fn create_test_manager() -> IndexExtractorEvidenceManager {
    IndexExtractorEvidenceManager::new_empty()
}

/// Create a test seek point
fn create_test_seek_point(display_idx: usize, byte_offset: u64) -> SeekPoint {
    SeekPoint {
        display_idx,
        byte_offset,
        is_keyframe: true,
        pts: Some(display_idx as u64 * 100),
    }
}

/// Create a test frame metadata
fn create_test_frame_metadata(display_idx: usize, byte_offset: u64, size: u64) -> FrameMetadata {
    FrameMetadata {
        display_idx,
        decode_idx: display_idx,
        byte_offset,
        size,
        is_keyframe: display_idx.is_multiple_of(3),
        pts: Some(display_idx as u64 * 100),
        dts: Some(display_idx as u64 * 100),
        frame_type: Some(if display_idx.is_multiple_of(3) { "I".to_string() } else { "P".to_string() }),
    }
}

// ============================================================================
// IndexExtractorEvidenceManager Construction Tests
// ============================================================================

#[cfg(test)]
mod construction_tests {
    use super::*;

    #[test]
    fn test_new_empty_creates_manager() {
        // Arrange & Act
        let manager = IndexExtractorEvidenceManager::new_empty();

        // Assert
        assert_eq!(manager.frame_count(), 0);
        assert_eq!(manager.all_frame_evidence().len(), 0);
    }

    #[test]
    fn test_new_with_evidence_chain() {
        // Arrange
        let chain = EvidenceChain::new();

        // Act
        let manager = IndexExtractorEvidenceManager::new(chain);

        // Assert
        assert_eq!(manager.frame_count(), 0);
    }

    #[test]
    fn test_default_creates_manager() {
        // Arrange & Act
        let manager = IndexExtractorEvidenceManager::default();

        // Assert
        assert_eq!(manager.frame_count(), 0);
    }
}

// ============================================================================
// Create Seekpoint Evidence Tests
// ============================================================================

#[cfg(test)]
mod create_seekpoint_evidence_tests {
    use super::*;

    #[test]
    fn test_create_seekpoint_evidence_returns_evidence() {
        // Arrange
        let mut manager = create_test_manager();
        let seek_point = create_test_seek_point(0, 1000);

        // Act
        let evidence = manager.create_seekpoint_evidence(&seek_point);

        // Assert
        assert_eq!(evidence.display_idx, 0);
        assert!(!evidence.bit_offset_id.is_empty());
        assert!(!evidence.syntax_id.is_empty());
    }

    #[test]
    fn test_create_seekpoint_evidence_adds_to_map() {
        // Arrange
        let mut manager = create_test_manager();
        let seek_point = create_test_seek_point(5, 5000);

        // Act
        manager.create_seekpoint_evidence(&seek_point);

        // Assert
        assert_eq!(manager.frame_count(), 1);
        assert!(manager.get_frame_evidence(5).is_some());
    }

    #[test]
    fn test_create_seekpoint_evidence_increments_id() {
        // Arrange
        let mut manager = create_test_manager();
        let seek_point1 = create_test_seek_point(0, 1000);
        let seek_point2 = create_test_seek_point(1, 2000);

        // Act
        let evidence1 = manager.create_seekpoint_evidence(&seek_point1);
        let evidence2 = manager.create_seekpoint_evidence(&seek_point2);

        // Assert - IDs should be different
        assert_ne!(evidence1.bit_offset_id, evidence2.bit_offset_id);
        assert_ne!(evidence1.syntax_id, evidence2.syntax_id);
    }

    #[test]
    fn test_create_seekpoint_evidence_estimates_size() {
        // Arrange
        let mut manager = create_test_manager();
        let seek_point = create_test_seek_point(0, 1000);

        // Act
        let _evidence = manager.create_seekpoint_evidence(&seek_point);
        let bit_range = manager.trace_to_bit_offset(0).unwrap();

        // Assert - Estimated size is 1024 bytes = 8192 bits
        assert_eq!(bit_range.end_bit - bit_range.start_bit, 1024 * 8);
    }
}

// ============================================================================
// Create Frame Metadata Evidence Tests
// ============================================================================

#[cfg(test)]
mod create_frame_metadata_evidence_tests {
    use super::*;

    #[test]
    fn test_create_frame_metadata_evidence_returns_evidence() {
        // Arrange
        let mut manager = create_test_manager();
        let frame = create_test_frame_metadata(0, 1000, 500);

        // Act
        let evidence = manager.create_frame_metadata_evidence(&frame);

        // Assert
        assert_eq!(evidence.display_idx, 0);
        assert!(!evidence.bit_offset_id.is_empty());
        assert!(!evidence.syntax_id.is_empty());
    }

    #[test]
    fn test_create_frame_metadata_evidence_uses_actual_size() {
        // Arrange
        let mut manager = create_test_manager();
        let frame = create_test_frame_metadata(0, 1000, 500);

        // Act
        manager.create_frame_metadata_evidence(&frame);
        let bit_range = manager.trace_to_bit_offset(0).unwrap();

        // Assert - Actual size is 500 bytes = 4000 bits
        assert_eq!(bit_range.end_bit - bit_range.start_bit, 500 * 8);
    }

    #[test]
    fn test_create_frame_metadata_evidence_keyframe() {
        // Arrange
        let mut manager = create_test_manager();
        let frame = create_test_frame_metadata(0, 1000, 500); // display_idx % 3 == 0 is keyframe

        // Act
        manager.create_frame_metadata_evidence(&frame);
        let chain = manager.evidence_chain();

        // Assert
        let evidence = manager.get_frame_evidence(0).unwrap();
        let syntax_ev = chain.syntax_index.find_by_id(&evidence.syntax_id).unwrap();
        assert_eq!(syntax_ev.metadata.get("is_keyframe"), Some(&"true".to_string()));
    }

    #[test]
    fn test_create_frame_metadata_evidence_non_keyframe() {
        // Arrange
        let mut manager = create_test_manager();
        let frame = create_test_frame_metadata(1, 2000, 300); // Not a keyframe

        // Act
        manager.create_frame_metadata_evidence(&frame);
        let chain = manager.evidence_chain();

        // Assert
        let evidence = manager.get_frame_evidence(1).unwrap();
        let syntax_ev = chain.syntax_index.find_by_id(&evidence.syntax_id).unwrap();
        assert_eq!(syntax_ev.metadata.get("is_keyframe"), Some(&"false".to_string()));
    }
}

// ============================================================================
// Update Seekpoint Size Tests
// ============================================================================

#[cfg(test)]
mod update_seekpoint_size_tests {
    use super::*;

    #[test]
    fn test_update_seekpoint_size_creates_new_evidence() {
        // Arrange
        let mut manager = create_test_manager();
        let seek_point = create_test_seek_point(0, 1000);
        manager.create_seekpoint_evidence(&seek_point);
        let old_evidence = manager.get_frame_evidence(0).unwrap().clone();

        // Act
        manager.update_seekpoint_size(0, 500);
        let new_evidence = manager.get_frame_evidence(0).unwrap();

        // Assert
        assert_ne!(old_evidence.bit_offset_id, new_evidence.bit_offset_id);
    }

    #[test]
    fn test_update_seekpoint_size_updates_bit_range() {
        // Arrange
        let mut manager = create_test_manager();
        let seek_point = create_test_seek_point(0, 1000);
        manager.create_seekpoint_evidence(&seek_point);

        // Act
        manager.update_seekpoint_size(0, 500);
        let bit_range = manager.trace_to_bit_offset(0).unwrap();

        // Assert - New size is 500 bytes = 4000 bits
        assert_eq!(bit_range.end_bit - bit_range.start_bit, 500 * 8);
    }

    #[test]
    fn test_update_seekpoint_size_for_nonexistent_frame() {
        // Arrange
        let mut manager = create_test_manager();

        // Act - Should not panic
        manager.update_seekpoint_size(999, 500);

        // Assert - Frame still doesn't exist
        assert!(manager.get_frame_evidence(999).is_none());
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
        let frame = create_test_frame_metadata(5, 5000, 300);
        manager.create_frame_metadata_evidence(&frame);

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
}

// ============================================================================
// Get Evidence By Offset Tests
// ============================================================================

#[cfg(test)]
mod get_evidence_by_offset_tests {
    use super::*;

    #[test]
    fn test_get_evidence_by_offset_exists() {
        // Arrange
        let mut manager = create_test_manager();
        let frame = create_test_frame_metadata(0, 5000, 300);
        manager.create_frame_metadata_evidence(&frame);

        // Act
        let evidence_id = manager.get_evidence_by_offset(5000);

        // Assert
        assert!(evidence_id.is_some());
    }

    #[test]
    fn test_get_evidence_by_offset_not_exists() {
        // Arrange
        let manager = create_test_manager();

        // Act
        let evidence_id = manager.get_evidence_by_offset(9999);

        // Assert
        assert!(evidence_id.is_none());
    }
}

// ============================================================================
// Trace Tests
// ============================================================================

#[cfg(test)]
mod trace_tests {
    use super::*;

    #[test]
    fn test_trace_to_bit_offset_exists() {
        // Arrange
        let mut manager = create_test_manager();
        let frame = create_test_frame_metadata(0, 1000, 500);
        manager.create_frame_metadata_evidence(&frame);

        // Act
        let bit_range = manager.trace_to_bit_offset(0);

        // Assert
        assert!(bit_range.is_some());
        let range = bit_range.unwrap();
        assert_eq!(range.start_bit, 1000 * 8);
        assert_eq!(range.end_bit, (1000 + 500) * 8);
    }

    #[test]
    fn test_trace_to_bit_offset_not_exists() {
        // Arrange
        let manager = create_test_manager();

        // Act
        let bit_range = manager.trace_to_bit_offset(999);

        // Assert
        assert!(bit_range.is_none());
    }

    #[test]
    fn test_trace_to_display_idx_exists() {
        // Arrange
        let mut manager = create_test_manager();
        let frame = create_test_frame_metadata(5, 5000, 300);
        manager.create_frame_metadata_evidence(&frame);

        // Act
        let display_idx = manager.trace_to_display_idx(5000);

        // Assert
        assert_eq!(display_idx, Some(5));
    }

    #[test]
    fn test_trace_to_display_idx_not_exists() {
        // Arrange
        let manager = create_test_manager();

        // Act
        let display_idx = manager.trace_to_display_idx(9999);

        // Assert
        assert!(display_idx.is_none());
    }
}

// ============================================================================
// Evidence Chain Access Tests
// ============================================================================

#[cfg(test)]
mod evidence_chain_access_tests {
    use super::*;

    #[test]
    fn test_evidence_chain_immutable() {
        // Arrange
        let manager = create_test_manager();

        // Act
        let chain = manager.evidence_chain();

        // Assert
        assert_eq!(chain.bit_offset_index.len(), 0);
        assert_eq!(chain.syntax_index.len(), 0);
    }

    #[test]
    fn test_evidence_chain_mutable() {
        // Arrange
        let mut manager = create_test_manager();

        // Act
        let chain = manager.evidence_chain_mut();

        // Assert
        assert_eq!(chain.bit_offset_index.len(), 0);
        assert_eq!(chain.syntax_index.len(), 0);
    }
}

// ============================================================================
// Frame Count Tests
// ============================================================================

#[cfg(test)]
mod frame_count_tests {
    use super::*;

    #[test]
    fn test_frame_count_initially_zero() {
        // Arrange
        let manager = create_test_manager();

        // Act
        let count = manager.frame_count();

        // Assert
        assert_eq!(count, 0);
    }

    #[test]
    fn test_frame_count_increments() {
        // Arrange
        let mut manager = create_test_manager();

        // Act
        for i in 0..5 {
            let frame = create_test_frame_metadata(i, i as u64 * 1000, 100);
            manager.create_frame_metadata_evidence(&frame);
        }

        // Assert
        assert_eq!(manager.frame_count(), 5);
    }
}

// ============================================================================
// All Frame Evidence Tests
// ============================================================================

#[cfg(test)]
mod all_frame_evidence_tests {
    use super::*;

    #[test]
    fn test_all_frame_evidence_empty() {
        // Arrange
        let manager = create_test_manager();

        // Act
        let all = manager.all_frame_evidence();

        // Assert
        assert!(all.is_empty());
    }

    #[test]
    fn test_all_frame_evidence_sorted() {
        // Arrange
        let mut manager = create_test_manager();
        // Add in reverse order
        for i in (0..5).rev() {
            let frame = create_test_frame_metadata(i, i as u64 * 1000, 100);
            manager.create_frame_metadata_evidence(&frame);
        }

        // Act
        let all = manager.all_frame_evidence();

        // Assert - Should be sorted by display_idx
        for (i, evidence) in all.iter().enumerate() {
            assert_eq!(evidence.display_idx, i);
        }
    }

    #[test]
    fn test_all_frame_evidence_returns_all() {
        // Arrange
        let mut manager = create_test_manager();
        for i in 0..3 {
            let frame = create_test_frame_metadata(i, i as u64 * 1000, 100);
            manager.create_frame_metadata_evidence(&frame);
        }

        // Act
        let all = manager.all_frame_evidence();

        // Assert
        assert_eq!(all.len(), 3);
    }
}

// ============================================================================
// Clear Tests
// ============================================================================

#[cfg(test)]
mod clear_tests {
    use super::*;

    #[test]
    fn test_clear_resets_frame_count() {
        // Arrange
        let mut manager = create_test_manager();
        let frame = create_test_frame_metadata(0, 1000, 100);
        manager.create_frame_metadata_evidence(&frame);
        assert_eq!(manager.frame_count(), 1);

        // Act
        manager.clear();

        // Assert
        assert_eq!(manager.frame_count(), 0);
    }

    #[test]
    fn test_clear_resets_evidence_chain() {
        // Arrange
        let mut manager = create_test_manager();
        let frame = create_test_frame_metadata(0, 1000, 100);
        manager.create_frame_metadata_evidence(&frame);

        // Act
        manager.clear();

        // Assert
        let chain = manager.evidence_chain();
        assert_eq!(chain.bit_offset_index.len(), 0);
        assert_eq!(chain.syntax_index.len(), 0);
    }

    #[test]
    fn test_clear_resets_offset_map() {
        // Arrange
        let mut manager = create_test_manager();
        let frame = create_test_frame_metadata(0, 5000, 100);
        manager.create_frame_metadata_evidence(&frame);
        assert!(manager.get_evidence_by_offset(5000).is_some());

        // Act
        manager.clear();

        // Assert
        assert!(manager.get_evidence_by_offset(5000).is_none());
    }
}

// ============================================================================
// Evidence ID Format Tests
// ============================================================================

#[cfg(test)]
mod evidence_id_format_tests {
    use super::*;

    #[test]
    fn test_evidence_id_format_includes_prefix() {
        // Arrange
        let mut manager = create_test_manager();
        let seek_point = create_test_seek_point(0, 1000);

        // Act
        let evidence = manager.create_seekpoint_evidence(&seek_point);

        // Assert - IDs should start with "index_ev_"
        assert!(evidence.bit_offset_id.starts_with("index_ev_"));
        assert!(evidence.syntax_id.starts_with("index_ev_"));
    }

    #[test]
    fn test_evidence_id_increments() {
        // Arrange
        let mut manager = create_test_manager();
        let seek_point1 = create_test_seek_point(0, 1000);
        let seek_point2 = create_test_seek_point(1, 2000);

        // Act
        let evidence1 = manager.create_seekpoint_evidence(&seek_point1);
        let evidence2 = manager.create_seekpoint_evidence(&seek_point2);

        // Assert - Second evidence should have higher ID numbers
        let id1_num = evidence1.bit_offset_id.strip_prefix("index_ev_").unwrap();
        let id2_num = evidence2.bit_offset_id.strip_prefix("index_ev_").unwrap();
        assert!(id2_num > id1_num);
    }
}

// ============================================================================
// Edge Cases
// ============================================================================

#[cfg(test)]
mod edge_case_tests {
    use super::*;

    #[test]
    fn test_multiple_frames_same_offset() {
        // Arrange
        let mut manager = create_test_manager();
        let frame1 = create_test_frame_metadata(0, 5000, 100);
        let frame2 = create_test_frame_metadata(1, 5000, 200);

        // Act
        manager.create_frame_metadata_evidence(&frame1);
        manager.create_frame_metadata_evidence(&frame2);

        // Assert - Last one wins in offset map
        let offset_id = manager.get_evidence_by_offset(5000);
        assert!(offset_id.is_some());
    }

    #[test]
    fn test_update_seekpoint_preserves_metadata() {
        // Arrange
        let mut manager = create_test_manager();
        let seek_point = create_test_seek_point(0, 1000);
        manager.create_seekpoint_evidence(&seek_point);
        let old_evidence = manager.get_frame_evidence(0).unwrap().clone();

        // Act
        manager.update_seekpoint_size(0, 500);
        let new_evidence = manager.get_frame_evidence(0).unwrap();

        // Assert - Metadata should be preserved
        let old_syntax = manager
            .evidence_chain()
            .syntax_index
            .find_by_id(&old_evidence.syntax_id)
            .unwrap();
        let new_syntax = manager
            .evidence_chain()
            .syntax_index
            .find_by_id(&new_evidence.syntax_id)
            .unwrap();

        assert_eq!(
            old_syntax.metadata.get("is_keyframe"),
            new_syntax.metadata.get("is_keyframe")
        );
        assert_eq!(
            old_syntax.metadata.get("display_idx"),
            new_syntax.metadata.get("display_idx")
        );
    }

    #[test]
    fn test_trace_to_bit_offset_returns_accurate_range() {
        // Arrange
        let mut manager = create_test_manager();
        let byte_offset = 12345;
        let size = 678;
        let frame = create_test_frame_metadata(0, byte_offset, size);
        manager.create_frame_metadata_evidence(&frame);

        // Act
        let bit_range = manager.trace_to_bit_offset(0).unwrap();

        // Assert
        assert_eq!(bit_range.start_bit, byte_offset * 8);
        assert_eq!(bit_range.end_bit, (byte_offset + size) * 8);
    }

    #[test]
    fn test_seekpoint_and_frame_metadata_same_index() {
        // Arrange
        let mut manager = create_test_manager();
        let seek_point = create_test_seek_point(0, 1000);
        let frame = create_test_frame_metadata(0, 1000, 500);

        // Act
        manager.create_seekpoint_evidence(&seek_point);
        manager.create_frame_metadata_evidence(&frame);

        // Assert - Frame metadata should override seekpoint evidence
        let _evidence = manager.get_frame_evidence(0).unwrap();
        let bit_range = manager.trace_to_bit_offset(0).unwrap();
        assert_eq!(bit_range.end_bit - bit_range.start_bit, 500 * 8);
    }
}
