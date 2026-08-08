// Compare evidence module tests
//
// Per generate-tests skill: Arrange-Act-Assert pattern with fixtures
// and edge case coverage.

use super::*;
use crate::{
    BitRange, DecodeArtifactType, EvidenceChain,
    FramePair, SyntaxNodeType, VizElementType,
};

// ============================================================================
// Fixtures
// ============================================================================

/// Create a test compare evidence manager
fn create_test_manager() -> CompareEvidenceManager {
    CompareEvidenceManager::new(EvidenceChain::new(), EvidenceChain::new())
}

/// Create a test frame pair
fn create_test_frame_pair(frame_a: usize, frame_b: usize) -> FramePair {
    FramePair {
        stream_a_idx: Some(frame_a),
        stream_b_idx: Some(frame_b),
        pts_delta: Some(0),
        has_gap: false,
    }
}

/// Create a test bit range
fn create_test_bit_range(start: u64, end: u64) -> BitRange {
    BitRange::new(start, end)
}

// ============================================================================
// CompareEvidenceManager Construction Tests
// ============================================================================

#[cfg(test)]
mod construction_tests {
    use super::*;

    #[test]
    fn test_compare_evidence_manager_new() {
        // Arrange & Act
        let manager = CompareEvidenceManager::new(EvidenceChain::new(), EvidenceChain::new());

        // Assert
        assert_eq!(manager.pair_count(), 0);
        assert_eq!(manager.evidence_chain_a().bit_offset_index.all().len(), 0);
        assert_eq!(manager.evidence_chain_b().bit_offset_index.all().len(), 0);
    }

    #[test]
    fn test_compare_evidence_manager_default() {
        // Arrange & Act
        let manager = CompareEvidenceManager::default();

        // Assert
        assert_eq!(manager.pair_count(), 0);
    }
}

// ============================================================================
// Create Pair Evidence Tests
// ============================================================================

#[cfg(test)]
mod create_pair_evidence_tests {
    use super::*;

    #[test]
    fn test_create_pair_evidence_complete() {
        // Arrange
        let mut manager = create_test_manager();
        let pair = create_test_frame_pair(0, 0);
        let bit_range_a = create_test_bit_range(0, 1000);
        let bit_range_b = create_test_bit_range(0, 1000);

        // Act
        let evidence = manager.create_pair_evidence(&pair, bit_range_a, bit_range_b, 1000, 1000);

        // Assert
        assert!(evidence.is_some());
        let ev = evidence.unwrap();
        assert_eq!(ev.frame_a_idx, 0);
        assert_eq!(ev.frame_b_idx, 0);
        assert_eq!(manager.pair_count(), 1);
    }

    #[test]
    fn test_create_pair_evidence_with_gap_a() {
        // Arrange
        let mut manager = create_test_manager();
        let pair = FramePair {
            stream_a_idx: None,
            stream_b_idx: Some(0),
            pts_delta: None,
            has_gap: true,
        };
        let bit_range_a = create_test_bit_range(0, 1000);
        let bit_range_b = create_test_bit_range(0, 1000);

        // Act
        let evidence = manager.create_pair_evidence(&pair, bit_range_a, bit_range_b, 1000, 1000);

        // Assert - Should return None for pairs with gaps
        assert!(evidence.is_none());
        assert_eq!(manager.pair_count(), 0);
    }

    #[test]
    fn test_create_pair_evidence_with_gap_b() {
        // Arrange
        let mut manager = create_test_manager();
        let pair = FramePair {
            stream_a_idx: Some(0),
            stream_b_idx: None,
            pts_delta: None,
            has_gap: true,
        };
        let bit_range_a = create_test_bit_range(0, 1000);
        let bit_range_b = create_test_bit_range(0, 1000);

        // Act
        let evidence = manager.create_pair_evidence(&pair, bit_range_a, bit_range_b, 1000, 1000);

        // Assert - Should return None for pairs with gaps
        assert!(evidence.is_none());
    }

    #[test]
    fn test_create_pair_evidence_stores_in_map() {
        // Arrange
        let mut manager = create_test_manager();
        let pair = create_test_frame_pair(5, 10);
        let bit_range_a = create_test_bit_range(0, 1000);
        let bit_range_b = create_test_bit_range(0, 1000);

        // Act
        manager.create_pair_evidence(&pair, bit_range_a, bit_range_b, 1000, 1000);

        // Assert - Should be retrievable
        let retrieved = manager.get_pair_evidence(5, 10);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().frame_a_idx, 5);
    }

    #[test]
    fn test_create_pair_evidence_adds_to_chains() {
        // Arrange
        let mut manager = create_test_manager();
        let pair = create_test_frame_pair(0, 0);
        let bit_range_a = create_test_bit_range(0, 1000);
        let bit_range_b = create_test_bit_range(0, 1000);

        // Act
        manager.create_pair_evidence(&pair, bit_range_a, bit_range_b, 1000, 1000);

        // Assert - Evidence should be added to both chains
        assert_eq!(manager.evidence_chain_a().bit_offset_index.all().len(), 1);
        assert_eq!(manager.evidence_chain_b().bit_offset_index.all().len(), 1);
        assert_eq!(manager.evidence_chain_a().syntax_index.all().len(), 1);
        assert_eq!(manager.evidence_chain_b().syntax_index.all().len(), 1);
        assert_eq!(manager.evidence_chain_a().decode_index.all().len(), 1);
        assert_eq!(manager.evidence_chain_b().decode_index.all().len(), 1);
        // Diff viz added to chain A only
        assert_eq!(manager.evidence_chain_a().viz_index.all().len(), 1);
        assert_eq!(manager.evidence_chain_b().viz_index.all().len(), 0);
    }
}

// ============================================================================
// Get Pair Evidence Tests
// ============================================================================

#[cfg(test)]
mod get_pair_evidence_tests {
    use super::*;

    #[test]
    fn test_get_pair_evidence_existing() {
        // Arrange
        let mut manager = create_test_manager();
        let pair = create_test_frame_pair(0, 0);
        let bit_range_a = create_test_bit_range(0, 1000);
        let bit_range_b = create_test_bit_range(0, 1000);
        manager.create_pair_evidence(&pair, bit_range_a, bit_range_b, 1000, 1000);

        // Act
        let evidence = manager.get_pair_evidence(0, 0);

        // Assert
        assert!(evidence.is_some());
        let ev = evidence.unwrap();
        assert_eq!(ev.frame_a_idx, 0);
        assert_eq!(ev.frame_b_idx, 0);
    }

    #[test]
    fn test_get_pair_evidence_not_found() {
        // Arrange
        let manager = create_test_manager();

        // Act
        let evidence = manager.get_pair_evidence(99, 99);

        // Assert
        assert!(evidence.is_none());
    }

    #[test]
    fn test_get_pair_evidence_different_pair() {
        // Arrange
        let mut manager = create_test_manager();
        let pair = create_test_frame_pair(5, 10);
        let bit_range_a = create_test_bit_range(0, 1000);
        let bit_range_b = create_test_bit_range(0, 1000);
        manager.create_pair_evidence(&pair, bit_range_a, bit_range_b, 1000, 1000);

        // Act - Look for different pair
        let evidence = manager.get_pair_evidence(0, 0);

        // Assert
        assert!(evidence.is_none());
    }
}

// ============================================================================
// Find Pairs Tests
// ============================================================================

#[cfg(test)]
mod find_pairs_tests {
    use super::*;

    #[test]
    fn test_find_pairs_with_frame_a() {
        // Arrange
        let mut manager = create_test_manager();
        let pair1 = create_test_frame_pair(5, 0);
        let pair2 = create_test_frame_pair(5, 1);
        let pair3 = create_test_frame_pair(6, 0);

        let bit_range_a = create_test_bit_range(0, 1000);
        let bit_range_b = create_test_bit_range(0, 1000);

        manager.create_pair_evidence(&pair1, bit_range_a, bit_range_b, 1000, 1000);
        manager.create_pair_evidence(&pair2, bit_range_a, bit_range_b, 1000, 1000);
        manager.create_pair_evidence(&pair3, bit_range_a, bit_range_b, 1000, 1000);

        // Act
        let pairs = manager.find_pairs_with_frame_a(5);

        // Assert
        assert_eq!(pairs.len(), 2);
        assert!(pairs.iter().all(|ev| ev.frame_a_idx == 5));
    }

    #[test]
    fn test_find_pairs_with_frame_b() {
        // Arrange
        let mut manager = create_test_manager();
        let pair1 = create_test_frame_pair(0, 5);
        let pair2 = create_test_frame_pair(1, 5);
        let pair3 = create_test_frame_pair(2, 6);

        let bit_range_a = create_test_bit_range(0, 1000);
        let bit_range_b = create_test_bit_range(0, 1000);

        manager.create_pair_evidence(&pair1, bit_range_a, bit_range_b, 1000, 1000);
        manager.create_pair_evidence(&pair2, bit_range_a, bit_range_b, 1000, 1000);
        manager.create_pair_evidence(&pair3, bit_range_a, bit_range_b, 1000, 1000);

        // Act
        let pairs = manager.find_pairs_with_frame_b(5);

        // Assert
        assert_eq!(pairs.len(), 2);
        assert!(pairs.iter().all(|ev| ev.frame_b_idx == 5));
    }

    #[test]
    fn test_find_pairs_with_frame_no_matches() {
        // Arrange
        let mut manager = create_test_manager();
        let pair = create_test_frame_pair(0, 0);
        let bit_range_a = create_test_bit_range(0, 1000);
        let bit_range_b = create_test_bit_range(0, 1000);
        manager.create_pair_evidence(&pair, bit_range_a, bit_range_b, 1000, 1000);

        // Act
        let pairs_a = manager.find_pairs_with_frame_a(99);
        let pairs_b = manager.find_pairs_with_frame_b(99);

        // Assert
        assert_eq!(pairs_a.len(), 0);
        assert_eq!(pairs_b.len(), 0);
    }
}

// ============================================================================
// Evidence Chain Accessor Tests
// ============================================================================

#[cfg(test)]
mod evidence_chain_accessor_tests {
    use super::*;

    #[test]
    fn test_evidence_chain_a() {
        // Arrange
        let manager = create_test_manager();

        // Act
        let chain = manager.evidence_chain_a();

        // Assert - Should return reference
        assert_eq!(chain.bit_offset_index.all().len(), 0);
    }

    #[test]
    fn test_evidence_chain_b() {
        // Arrange
        let manager = create_test_manager();

        // Act
        let chain = manager.evidence_chain_b();

        // Assert - Should return reference
        assert_eq!(chain.bit_offset_index.all().len(), 0);
    }

    #[test]
    fn test_evidence_chain_a_mut() {
        // Arrange
        let mut manager = create_test_manager();

        // Act
        let chain = manager.evidence_chain_a_mut();

        // Assert - Should return mutable reference
        // Can't test mutation directly without internal access
        assert_eq!(chain.bit_offset_index.all().len(), 0);
    }

    #[test]
    fn test_evidence_chain_b_mut() {
        // Arrange
        let mut manager = create_test_manager();

        // Act
        let chain = manager.evidence_chain_b_mut();

        // Assert - Should return mutable reference
        assert_eq!(chain.bit_offset_index.all().len(), 0);
    }
}

// ============================================================================
// Clear and Count Tests
// ============================================================================

#[cfg(test)]
mod clear_and_count_tests {
    use super::*;

    #[test]
    fn test_pair_count() {
        // Arrange
        let mut manager = create_test_manager();
        let pair1 = create_test_frame_pair(0, 0);
        let pair2 = create_test_frame_pair(1, 1);
        let bit_range_a = create_test_bit_range(0, 1000);
        let bit_range_b = create_test_bit_range(0, 1000);

        // Act
        manager.create_pair_evidence(&pair1, bit_range_a, bit_range_b, 1000, 1000);
        manager.create_pair_evidence(&pair2, bit_range_a, bit_range_b, 1000, 1000);

        // Assert
        assert_eq!(manager.pair_count(), 2);
    }

    #[test]
    fn test_pair_count_empty() {
        // Arrange
        let manager = create_test_manager();

        // Act & Assert
        assert_eq!(manager.pair_count(), 0);
    }

    #[test]
    fn test_clear() {
        // Arrange
        let mut manager = create_test_manager();
        let pair = create_test_frame_pair(0, 0);
        let bit_range_a = create_test_bit_range(0, 1000);
        let bit_range_b = create_test_bit_range(0, 1000);
        manager.create_pair_evidence(&pair, bit_range_a, bit_range_b, 1000, 1000);
        assert_eq!(manager.pair_count(), 1);

        // Act
        manager.clear();

        // Assert
        assert_eq!(manager.pair_count(), 0);
        assert!(manager.get_pair_evidence(0, 0).is_none());
    }

    #[test]
    fn test_clear_empty_manager() {
        // Arrange
        let mut manager = create_test_manager();

        // Act - Should not panic
        manager.clear();

        // Assert
        assert_eq!(manager.pair_count(), 0);
    }
}

// ============================================================================
// ComparePairEvidence Tests
// ============================================================================

#[cfg(test)]
mod compare_pair_evidence_tests {
    use super::*;

    #[test]
    fn test_compare_pair_evidence_fields() {
        // Arrange
        let mut manager = create_test_manager();
        let pair = create_test_frame_pair(5, 10);
        let bit_range_a = create_test_bit_range(0, 1000);
        let bit_range_b = create_test_bit_range(0, 1000);

        // Act
        let evidence = manager.create_pair_evidence(&pair, bit_range_a, bit_range_b, 1000, 1000);

        // Assert
        assert!(evidence.is_some());
        let ev = evidence.unwrap();
        assert_eq!(ev.frame_a_idx, 5);
        assert_eq!(ev.frame_b_idx, 10);
        // StreamEvidence should have unique IDs
        assert_ne!(ev.stream_a.bit_offset_id, ev.stream_b.bit_offset_id);
        assert_ne!(ev.stream_a.syntax_id, ev.stream_b.syntax_id);
        assert_ne!(ev.stream_a.decode_id, ev.stream_b.decode_id);
        // Diff viz ID should be unique
        assert!(!ev.diff_viz_id.is_empty());
    }
}

// ============================================================================
// StreamEvidence Tests
// ============================================================================

#[cfg(test)]
mod stream_evidence_tests {
    use super::*;

    #[test]
    fn test_stream_evidence_creation() {
        // Arrange
        let mut manager = create_test_manager();
        let pair = create_test_frame_pair(0, 0);
        let bit_range_a = create_test_bit_range(0, 1000);
        let bit_range_b = create_test_bit_range(0, 1000);

        // Act
        let evidence = manager.create_pair_evidence(&pair, bit_range_a, bit_range_b, 1000, 1000);

        // Assert
        assert!(evidence.is_some());
        let ev = evidence.unwrap();

        // Stream A evidence
        assert!(!ev.stream_a.bit_offset_id.is_empty());
        assert!(!ev.stream_a.syntax_id.is_empty());
        assert!(!ev.stream_a.decode_id.is_empty());

        // Stream B evidence
        assert!(!ev.stream_b.bit_offset_id.is_empty());
        assert!(!ev.stream_b.syntax_id.is_empty());
        assert!(!ev.stream_b.decode_id.is_empty());
    }

    #[test]
    fn test_stream_evidence_unique_ids() {
        // Arrange
        let mut manager = create_test_manager();
        let pair = create_test_frame_pair(0, 0);
        let bit_range_a = create_test_bit_range(0, 1000);
        let bit_range_b = create_test_bit_range(0, 1000);

        // Act
        let evidence = manager.create_pair_evidence(&pair, bit_range_a, bit_range_b, 1000, 1000);

        // Assert
        let ev = evidence.unwrap();

        // All IDs within a stream should be different
        assert_ne!(ev.stream_a.bit_offset_id, ev.stream_a.syntax_id);
        assert_ne!(ev.stream_a.bit_offset_id, ev.stream_a.decode_id);
        assert_ne!(ev.stream_a.syntax_id, ev.stream_a.decode_id);

        assert_ne!(ev.stream_b.bit_offset_id, ev.stream_b.syntax_id);
        assert_ne!(ev.stream_b.bit_offset_id, ev.stream_b.decode_id);
        assert_ne!(ev.stream_b.syntax_id, ev.stream_b.decode_id);

        // IDs should be unique across streams too
        assert_ne!(ev.stream_a.bit_offset_id, ev.stream_b.bit_offset_id);
    }
}

// ============================================================================
// Evidence Chain Integration Tests
// ============================================================================

#[cfg(test)]
mod evidence_chain_integration_tests {
    use super::*;

    #[test]
    fn test_bit_offset_evidence_added() {
        // Arrange
        let mut manager = create_test_manager();
        let pair = create_test_frame_pair(0, 0);
        let bit_range_a = create_test_bit_range(100, 200);
        let bit_range_b = create_test_bit_range(300, 400);

        // Act
        manager.create_pair_evidence(&pair, bit_range_a, bit_range_b, 1000, 1000);

        // Assert
        let chain_a = manager.evidence_chain_a();
        let chain_b = manager.evidence_chain_b();

        assert_eq!(chain_a.bit_offset_index.all().len(), 1);
        assert_eq!(chain_b.bit_offset_index.all().len(), 1);

        // Check bit range stored correctly
        let offset_a = &chain_a.bit_offset_index.all()[0];
        assert_eq!(offset_a.bit_range.start_bit, 100);
        assert_eq!(offset_a.bit_range.end_bit, 200);

        let offset_b = &chain_b.bit_offset_index.all()[0];
        assert_eq!(offset_b.bit_range.start_bit, 300);
        assert_eq!(offset_b.bit_range.end_bit, 400);
    }

    #[test]
    fn test_syntax_evidence_added() {
        // Arrange
        let mut manager = create_test_manager();
        let pair = create_test_frame_pair(5, 10);
        let bit_range_a = create_test_bit_range(0, 1000);
        let bit_range_b = create_test_bit_range(0, 1000);

        // Act
        manager.create_pair_evidence(&pair, bit_range_a, bit_range_b, 1000, 1000);

        // Assert
        let chain_a = manager.evidence_chain_a();
        let chain_b = manager.evidence_chain_b();

        assert_eq!(chain_a.syntax_index.all().len(), 1);
        assert_eq!(chain_b.syntax_index.all().len(), 1);

        // Check syntax evidence has correct type
        let syntax_a = &chain_a.syntax_index.all()[0];
        assert_eq!(syntax_a.node_type, SyntaxNodeType::FrameHeader);

        let syntax_b = &chain_b.syntax_index.all()[0];
        assert_eq!(syntax_b.node_type, SyntaxNodeType::FrameHeader);
    }

    #[test]
    fn test_decode_evidence_added() {
        // Arrange
        let mut manager = create_test_manager();
        let pair = create_test_frame_pair(0, 0);
        let bit_range_a = create_test_bit_range(0, 1000);
        let bit_range_b = create_test_bit_range(0, 1000);

        // Act
        manager.create_pair_evidence(&pair, bit_range_a, bit_range_b, 1000, 1000);

        // Assert
        let chain_a = manager.evidence_chain_a();
        let chain_b = manager.evidence_chain_b();

        assert_eq!(chain_a.decode_index.all().len(), 1);
        assert_eq!(chain_b.decode_index.all().len(), 1);

        // Check decode evidence has correct type
        let decode_a = &chain_a.decode_index.all()[0];
        assert_eq!(decode_a.artifact_type, DecodeArtifactType::YuvFrame);

        let decode_b = &chain_b.decode_index.all()[0];
        assert_eq!(decode_b.artifact_type, DecodeArtifactType::YuvFrame);
    }

    #[test]
    fn test_viz_evidence_added_to_chain_a() {
        // Arrange
        let mut manager = create_test_manager();
        let pair = create_test_frame_pair(0, 0);
        let bit_range_a = create_test_bit_range(0, 1000);
        let bit_range_b = create_test_bit_range(0, 1000);

        // Act
        manager.create_pair_evidence(&pair, bit_range_a, bit_range_b, 1000, 1000);

        // Assert
        let chain_a = manager.evidence_chain_a();
        let chain_b = manager.evidence_chain_b();

        // Viz evidence should be in chain A only
        assert_eq!(chain_a.viz_index.all().len(), 1);
        assert_eq!(chain_b.viz_index.all().len(), 0);

        // Check viz evidence type
        let viz = &chain_a.viz_index.all()[0];
        assert_eq!(viz.element_type, VizElementType::DiffHeatmap);
    }

    #[test]
    fn test_viz_evidence_has_frame_indices() {
        // Arrange
        let mut manager = create_test_manager();
        let pair = create_test_frame_pair(5, 10);
        let bit_range_a = create_test_bit_range(0, 1000);
        let bit_range_b = create_test_bit_range(0, 1000);

        // Act
        manager.create_pair_evidence(&pair, bit_range_a, bit_range_b, 1000, 1000);

        // Assert
        let chain_a = manager.evidence_chain_a();
        let viz = &chain_a.viz_index.all()[0];

        // Check visual properties
        let props = &viz.visual_properties;
        assert_eq!(props.get("frame_a_idx"), Some(&"5".to_string()));
        assert_eq!(props.get("frame_b_idx"), Some(&"10".to_string()));
    }
}

// ============================================================================
// Edge Cases
// ============================================================================

#[cfg(test)]
mod edge_case_tests {
    use super::*;

    #[test]
    fn test_create_pair_evidence_zero_frames() {
        // Arrange
        let mut manager = create_test_manager();
        let pair = create_test_frame_pair(0, 0);
        let bit_range_a = create_test_bit_range(0, 0);
        let bit_range_b = create_test_bit_range(0, 0);

        // Act - Should handle zero-length bit ranges
        let evidence = manager.create_pair_evidence(&pair, bit_range_a, bit_range_b, 0, 0);

        // Assert
        assert!(evidence.is_some());
    }

    #[test]
    fn test_create_pair_evidence_large_frame_indices() {
        // Arrange
        let mut manager = create_test_manager();
        let pair = create_test_frame_pair(usize::MAX, usize::MAX - 1);
        let bit_range_a = create_test_bit_range(0, 1000);
        let bit_range_b = create_test_bit_range(0, 1000);

        // Act
        let evidence = manager.create_pair_evidence(&pair, bit_range_a, bit_range_b, 1000, 1000);

        // Assert
        assert!(evidence.is_some());
        let ev = evidence.unwrap();
        assert_eq!(ev.frame_a_idx, usize::MAX);
        assert_eq!(ev.frame_b_idx, usize::MAX - 1);
    }

    #[test]
    fn test_create_pair_evidence_large_frame_sizes() {
        // Arrange
        let mut manager = create_test_manager();
        let pair = create_test_frame_pair(0, 0);
        let bit_range_a = create_test_bit_range(0, 1000);
        let bit_range_b = create_test_bit_range(0, 1000);

        // Act
        let evidence = manager.create_pair_evidence(&pair, bit_range_a, bit_range_b, usize::MAX, usize::MAX);

        // Assert
        assert!(evidence.is_some());
    }

    #[test]
    fn test_multiple_pairs_same_frame_a() {
        // Arrange
        let mut manager = create_test_manager();
        let pair1 = create_test_frame_pair(0, 0);
        let pair2 = create_test_frame_pair(0, 1);
        let bit_range_a = create_test_bit_range(0, 1000);
        let bit_range_b = create_test_bit_range(0, 1000);

        // Act
        manager.create_pair_evidence(&pair1, bit_range_a, bit_range_b, 1000, 1000);
        manager.create_pair_evidence(&pair2, bit_range_a, bit_range_b, 1000, 1000);

        // Assert
        assert_eq!(manager.pair_count(), 2);
        let pairs = manager.find_pairs_with_frame_a(0);
        assert_eq!(pairs.len(), 2);
    }

    #[test]
    fn test_multiple_pairs_same_frame_b() {
        // Arrange
        let mut manager = create_test_manager();
        let pair1 = create_test_frame_pair(0, 0);
        let pair2 = create_test_frame_pair(1, 0);
        let bit_range_a = create_test_bit_range(0, 1000);
        let bit_range_b = create_test_bit_range(0, 1000);

        // Act
        manager.create_pair_evidence(&pair1, bit_range_a, bit_range_b, 1000, 1000);
        manager.create_pair_evidence(&pair2, bit_range_a, bit_range_b, 1000, 1000);

        // Assert
        assert_eq!(manager.pair_count(), 2);
        let pairs = manager.find_pairs_with_frame_b(0);
        assert_eq!(pairs.len(), 2);
    }
}
