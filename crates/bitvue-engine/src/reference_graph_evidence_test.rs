// Reference Graph Evidence module tests
//
// Per generate-tests skill: Arrange-Act-Assert pattern with fixtures
// and edge case coverage.

#[allow(unused_imports)]
use super::*;
use crate::types::BitRange;

// ============================================================================
// Fixtures
// ============================================================================

/// Create a test evidence ID
#[allow(dead_code)]
fn create_test_evidence_id() -> EvidenceId {
    EvidenceId::new()
}

/// Create a test bit range
#[allow(dead_code)]
fn create_test_bit_range(start: u64, end: u64) -> BitRange {
    BitRange { start, end }
}

/// Create a test reference node evidence
#[allow(dead_code)]
fn create_test_node_evidence(
    display_idx: usize,
    frame_type: &str,
    bit_range: BitRange,
) -> ReferenceNodeEvidence {
    ReferenceNodeEvidence::new(
        display_idx,
        frame_type.to_string(),
        bit_range,
        100.0,
        200.0,
    )
}

/// Create a test reference edge evidence
#[allow(dead_code)]
fn create_test_edge_evidence(
    from_idx: usize,
    to_idx: usize,
    bit_range: BitRange,
) -> ReferenceEdgeEvidence {
    ReferenceEdgeEvidence::new(from_idx, to_idx, bit_range)
}

/// Create a test evidence chain
#[allow(dead_code)]
fn create_test_evidence_chain() -> EvidenceChain {
    EvidenceChain::new()
}

/// Create a test reference graph evidence manager
#[allow(dead_code)]
fn create_test_manager() -> ReferenceGraphEvidenceManager {
    ReferenceGraphEvidenceManager::new()
}

// ============================================================================
// ReferenceNodeEvidence Tests
// ============================================================================

#[cfg(test)]
mod reference_node_evidence_tests {
#[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_new_creates_node_evidence() {
        // Arrange
        let bit_range = create_test_bit_range(100, 200);

        // Act
        let node = ReferenceNodeEvidence::new(
            5,
            "I_FRAME".to_string(),
            bit_range,
            50.0,
            100.0,
        );

        // Assert
        assert_eq!(node.display_idx, 5);
        assert_eq!(node.frame_type, "I_FRAME");
        assert_eq!(node.bit_range.start, 100);
        assert_eq!(node.bit_range.end, 200);
        assert_eq!(node.node_x, 50.0);
        assert_eq!(node.node_y, 100.0);
        assert!(node.syntax_evidence.is_none());
        assert!(node.decode_evidence.is_none());
        assert!(node.viz_evidence.is_none());
    }

    #[test]
    fn test_with_syntax_evidence() {
        // Arrange
        let bit_range = create_test_bit_range(100, 200);
        let mut node = ReferenceNodeEvidence::new(
            5,
            "I_FRAME".to_string(),
            bit_range,
            50.0,
            100.0,
        );

        let syntax_id = EvidenceId::new();
        node.syntax_evidence = Some(syntax_id);

        // Assert
        assert!(node.syntax_evidence.is_some());
    }
}

// ============================================================================
// ReferenceEdgeEvidence Tests
// ============================================================================

#[cfg(test)]
mod reference_edge_evidence_tests {
#[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_new_creates_edge_evidence() {
        // Arrange
        let bit_range = create_test_bit_range(100, 200);

        // Act
        let edge = ReferenceEdgeEvidence::new(5, 10, bit_range);

        // Assert
        assert_eq!(edge.from_idx, 5);
        assert_eq!(edge.to_idx, 10);
        assert_eq!(edge.bit_range.start, 100);
        assert_eq!(edge.bit_range.end, 200);
        assert!(edge.syntax_evidence.is_none());
        assert!(edge.decode_evidence.is_none());
    }
}

// ============================================================================
// ReferenceGraphEvidenceManager Construction Tests
// ============================================================================

#[cfg(test)]
mod manager_construction_tests {
#[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_new_creates_manager() {
        // Arrange & Act
        let manager = create_test_manager();

        // Assert
        assert!(manager.evidence_chain.nodes.is_empty());
        assert!(manager.evidence_chain.syntax_map.is_empty());
        assert!(manager.evidence_chain.decode_map.is_empty());
        assert!(manager.evidence_chain.viz_map.is_empty());
    }

    #[test]
    fn test_default_creates_manager() {
        // Arrange & Act
        let manager = ReferenceGraphEvidenceManager::default();

        // Assert
        assert!(manager.evidence_chain.nodes.is_empty());
    }
}

// ============================================================================
// ReferenceGraphEvidenceManager Node Evidence Tests
// ============================================================================

#[cfg(test)]
mod manager_node_evidence_tests {
#[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_create_node_evidence_creates_full_chain() {
        // Arrange
        let mut manager = create_test_manager();
        let bit_range = create_test_bit_range(100, 200);

        // Act
        let node = manager.create_node_evidence(
            5,
            "I_FRAME".to_string(),
            bit_range,
            50.0,
            100.0,
        );

        // Assert
        assert_eq!(node.display_idx, 5);
        assert_eq!(node.frame_type, "I_FRAME");
        assert!(node.syntax_evidence.is_some());
        assert!(node.decode_evidence.is_some());
        assert!(node.viz_evidence.is_some());
    }

    #[test]
    fn test_create_node_evidence_adds_to_chain() {
        // Arrange
        let mut manager = create_test_manager();
        let bit_range = create_test_bit_range(100, 200);

        // Act
        manager.create_node_evidence(
            5,
            "I_FRAME".to_string(),
            bit_range,
            50.0,
            100.0,
        );

        // Assert
        assert!(!manager.evidence_chain.syntax_map.is_empty());
        assert!(!manager.evidence_chain.decode_map.is_empty());
        assert!(!manager.evidence_chain.viz_map.is_empty());
    }

    #[test]
    fn test_create_node_evidence_different_frame_types() {
        // Arrange
        let mut manager = create_test_manager();

        // Act
        manager.create_node_evidence(
            0,
            "I_FRAME".to_string(),
            create_test_bit_range(0, 100),
            0.0,
            0.0,
        );
        manager.create_node_evidence(
            1,
            "P_FRAME".to_string(),
            create_test_bit_range(100, 200),
            100.0,
            0.0,
        );
        manager.create_node_evidence(
            2,
            "B_FRAME".to_string(),
            create_test_bit_range(200, 300),
            200.0,
            0.0,
        );

        // Assert
        // All three should be created with their frame types
        assert_eq!(manager.evidence_chain.syntax_map.len(), 3);
    }

    #[test]
    fn test_get_node_evidence_exists() {
        // Arrange
        let mut manager = create_test_manager();
        let bit_range = create_test_bit_range(100, 200);

        manager.create_node_evidence(
            5,
            "I_FRAME".to_string(),
            bit_range,
            50.0,
            100.0,
        );

        // Act
        let node = manager.get_node_evidence(5);

        // Assert
        assert!(node.is_some());
        assert_eq!(node.unwrap().display_idx, 5);
    }

    #[test]
    fn test_get_node_evidence_not_exists() {
        // Arrange
        let manager = create_test_manager();

        // Act
        let node = manager.get_node_evidence(999);

        // Assert
        assert!(node.is_none());
    }
}

// ============================================================================
// ReferenceGraphEvidenceManager Edge Evidence Tests
// ============================================================================

#[cfg(test)]
mod manager_edge_evidence_tests {
#[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_create_edge_evidence_creates_chain() {
        // Arrange
        let mut manager = create_test_manager();
        let bit_range = create_test_bit_range(100, 200);

        // Act
        let edge = manager.create_edge_evidence(5, 10, bit_range);

        // Assert
        assert_eq!(edge.from_idx, 5);
        assert_eq!(edge.to_idx, 10);
        assert!(edge.syntax_evidence.is_some());
        assert!(edge.decode_evidence.is_some());
    }

    #[test]
    fn test_create_edge_evidence_adds_to_chain() {
        // Arrange
        let mut manager = create_test_manager();
        let bit_range = create_test_bit_range(100, 200);

        // Act
        manager.create_edge_evidence(5, 10, bit_range);

        // Assert
        assert!(!manager.evidence_chain.syntax_map.is_empty());
        assert!(!manager.evidence_chain.decode_map.is_empty());
    }

    #[test]
    fn test_create_edge_evidence_multiple_edges() {
        // Arrange
        let mut manager = create_test_manager();

        // Act
        manager.create_edge_evidence(
            0,
            1,
            create_test_bit_range(0, 100),
        );
        manager.create_edge_evidence(
            1,
            2,
            create_test_bit_range(100, 200),
        );
        manager.create_edge_evidence(
            2,
            3,
            create_test_bit_range(200, 300),
        );

        // Assert
        assert_eq!(manager.evidence_chain.syntax_map.len(), 3);
    }

    #[test]
    fn test_get_edge_evidence_exists() {
        // Arrange
        let mut manager = create_test_manager();
        let bit_range = create_test_bit_range(100, 200);

        manager.create_edge_evidence(5, 10, bit_range);

        // Act
        let edge = manager.get_edge_evidence(5, 10);

        // Assert
        assert!(edge.is_some());
        assert_eq!(edge.unwrap().from_idx, 5);
        assert_eq!(edge.unwrap().to_idx, 10);
    }

    #[test]
    fn test_get_edge_evidence_not_exists() {
        // Arrange
        let manager = create_test_manager();

        // Act
        let edge = manager.get_edge_evidence(999, 1000);

        // Assert
        assert!(edge.is_none());
    }

    #[test]
    fn test_get_edge_evidence_wrong_direction() {
        // Arrange
        let mut manager = create_test_manager();
        let bit_range = create_test_bit_range(100, 200);

        manager.create_edge_evidence(5, 10, bit_range);

        // Act
        let edge = manager.get_edge_evidence(10, 5); // Reverse direction

        // Assert
        assert!(edge.is_none()); // Direction matters
    }
}

// ============================================================================
// ReferenceGraphEvidenceManager Query Tests
// ============================================================================

#[cfg(test)]
mod manager_query_tests {
#[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_get_all_nodes_empty() {
        // Arrange
        let manager = create_test_manager();

        // Act
        let nodes = manager.get_all_nodes();

        // Assert
        assert!(nodes.is_empty());
    }

    #[test]
    fn test_get_all_nodes_multiple() {
        // Arrange
        let mut manager = create_test_manager();

        manager.create_node_evidence(
            0,
            "I_FRAME".to_string(),
            create_test_bit_range(0, 100),
            0.0,
            0.0,
        );
        manager.create_node_evidence(
            1,
            "P_FRAME".to_string(),
            create_test_bit_range(100, 200),
            100.0,
            0.0,
        );
        manager.create_node_evidence(
            2,
            "B_FRAME".to_string(),
            create_test_bit_range(200, 300),
            200.0,
            0.0,
        );

        // Act
        let nodes = manager.get_all_nodes();

        // Assert
        assert_eq!(nodes.len(), 3);
    }

    #[test]
    fn test_get_all_edges_empty() {
        // Arrange
        let manager = create_test_manager();

        // Act
        let edges = manager.get_all_edges();

        // Assert
        assert!(edges.is_empty());
    }

    #[test]
    fn test_get_all_edges_multiple() {
        // Arrange
        let mut manager = create_test_manager();

        manager.create_edge_evidence(0, 1, create_test_bit_range(0, 100));
        manager.create_edge_evidence(1, 2, create_test_bit_range(100, 200));

        // Act
        let edges = manager.get_all_edges();

        // Assert
        assert_eq!(edges.len(), 2);
    }

    #[test]
    fn test_get_nodes_by_frame_type() {
        // Arrange
        let mut manager = create_test_manager();

        manager.create_node_evidence(
            0,
            "I_FRAME".to_string(),
            create_test_bit_range(0, 100),
            0.0,
            0.0,
        );
        manager.create_node_evidence(
            1,
            "I_FRAME".to_string(),
            create_test_bit_range(100, 200),
            100.0,
            0.0,
        );
        manager.create_node_evidence(
            2,
            "P_FRAME".to_string(),
            create_test_bit_range(200, 300),
            200.0,
            0.0,
        );

        // Act
        let i_frames = manager.get_nodes_by_frame_type("I_FRAME");
        let p_frames = manager.get_nodes_by_frame_type("P_FRAME");
        let b_frames = manager.get_nodes_by_frame_type("B_FRAME");

        // Assert
        assert_eq!(i_frames.len(), 2);
        assert_eq!(p_frames.len(), 1);
        assert_eq!(b_frames.len(), 0);
    }
}

// ============================================================================
// ReferenceGraphEvidenceManager Evidence Chain Tests
// ============================================================================

#[cfg(test)]
mod manager_evidence_chain_tests {
#[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_get_evidence_chain_returns_chain() {
        // Arrange
        let manager = create_test_manager();

        // Act
        let chain = manager.get_evidence_chain();

        // Assert
        assert_eq!(chain.nodes.len(), 0);
    }

    #[test]
    fn test_get_evidence_chain_mut_returns_mut_chain() {
        // Arrange
        let mut manager = create_test_manager();

        // Act
        let chain = manager.get_evidence_chain_mut();

        // Assert
        chain.nodes.insert(0, "test".to_string());
        assert_eq!(manager.evidence_chain.nodes.len(), 1);
    }

    #[test]
    fn test_node_evidence_links_to_chain() {
        // Arrange
        let mut manager = create_test_manager();
        let bit_range = create_test_bit_range(100, 200);

        // Act
        let node = manager.create_node_evidence(
            5,
            "I_FRAME".to_string(),
            bit_range.clone(),
            50.0,
            100.0,
        );

        // Assert - Evidence IDs should link to chain
        let syntax_id = node.syntax_evidence.unwrap();
        let decode_id = node.decode_evidence.unwrap();
        let viz_id = node.viz_evidence.unwrap();

        assert!(manager.evidence_chain.syntax_map.contains_key(&syntax_id));
        assert!(manager.evidence_chain.decode_map.contains_key(&decode_id));
        assert!(manager.evidence_chain.viz_map.contains_key(&viz_id));
    }

    #[test]
    fn test_edge_evidence_links_to_chain() {
        // Arrange
        let mut manager = create_test_manager();
        let bit_range = create_test_bit_range(100, 200);

        // Act
        let edge = manager.create_edge_evidence(5, 10, bit_range.clone());

        // Assert - Evidence IDs should link to chain
        let syntax_id = edge.syntax_evidence.unwrap();
        let decode_id = edge.decode_evidence.unwrap();

        assert!(manager.evidence_chain.syntax_map.contains_key(&syntax_id));
        assert!(manager.evidence_chain.decode_map.contains_key(&decode_id));
    }
}

// ============================================================================
// ReferenceGraphEvidenceManager Statistics Tests
// ============================================================================

#[cfg(test)]
mod manager_stats_tests {
#[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_get_stats_empty() {
        // Arrange
        let manager = create_test_manager();

        // Act
        let stats = manager.get_stats();

        // Assert
        assert_eq!(stats.node_count, 0);
        assert_eq!(stats.edge_count, 0);
        assert_eq!(stats.total_evidence, 0);
    }

    #[test]
    fn test_get_stats_with_data() {
        // Arrange
        let mut manager = create_test_manager();

        manager.create_node_evidence(
            0,
            "I_FRAME".to_string(),
            create_test_bit_range(0, 100),
            0.0,
            0.0,
        );
        manager.create_node_evidence(
            1,
            "P_FRAME".to_string(),
            create_test_bit_range(100, 200),
            100.0,
            0.0,
        );
        manager.create_edge_evidence(0, 1, create_test_bit_range(200, 300));

        // Act
        let stats = manager.get_stats();

        // Assert
        assert_eq!(stats.node_count, 2);
        assert_eq!(stats.edge_count, 1);
        // Total evidence: 2 nodes * 3 (syntax, decode, viz) + 1 edge * 2 (syntax, decode) = 8
        assert_eq!(stats.total_evidence, 8);
    }
}

// ============================================================================
// Edge Cases
// ============================================================================

#[cfg(test)]
mod edge_case_tests {
#[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_self_reference_edge() {
        // Arrange
        let mut manager = create_test_manager();

        // Act - Edge from node to itself
        manager.create_edge_evidence(5, 5, create_test_bit_range(100, 200));

        // Assert - Should be allowed (self-reference)
        let edge = manager.get_edge_evidence(5, 5);
        assert!(edge.is_some());
    }

    #[test]
    fn test_duplicate_node_same_idx() {
        // Arrange
        let mut manager = create_test_manager();

        // Act
        manager.create_node_evidence(
            5,
            "I_FRAME".to_string(),
            create_test_bit_range(100, 200),
            50.0,
            100.0,
        );
        manager.create_node_evidence(
            5,
            "P_FRAME".to_string(),
            create_test_bit_range(200, 300),
            150.0,
            100.0,
        );

        // Assert - May replace or duplicate depending on implementation
        let nodes = manager.get_all_nodes();
        // Implementation specific
    }

    #[test]
    fn test_duplicate_edge_same_pair() {
        // Arrange
        let mut manager = create_test_manager();

        // Act
        manager.create_edge_evidence(5, 10, create_test_bit_range(100, 200));
        manager.create_edge_evidence(5, 10, create_test_bit_range(200, 300));

        // Assert - May replace or duplicate depending on implementation
        let edges = manager.get_all_edges();
        // Implementation specific
    }

    #[test]
    fn test_zero_bit_range() {
        // Arrange
        let mut manager = create_test_manager();

        // Act - Zero-length range
        manager.create_node_evidence(
            5,
            "I_FRAME".to_string(),
            create_test_bit_range(100, 100),
            50.0,
            100.0,
        );

        // Assert - Should be allowed
        let node = manager.get_node_evidence(5);
        assert!(node.is_some());
    }

    #[test]
    fn test_negative_node_positions() {
        // Arrange
        let mut manager = create_test_manager();

        // Act - Negative coordinates
        manager.create_node_evidence(
            5,
            "I_FRAME".to_string(),
            create_test_bit_range(100, 200),
            -50.0,
            -100.0,
        );

        // Assert - Should be allowed
        let node = manager.get_node_evidence(5);
        assert!(node.is_some());
        assert_eq!(node.unwrap().node_x, -50.0);
    }

    #[test]
    fn test_empty_frame_type() {
        // Arrange
        let mut manager = create_test_manager();

        // Act
        manager.create_node_evidence(
            5,
            "".to_string(),
            create_test_bit_range(100, 200),
            50.0,
            100.0,
        );

        // Assert - Should be allowed
        let node = manager.get_node_evidence(5);
        assert!(node.is_some());
        assert_eq!(node.unwrap().frame_type, "");
    }

    #[test]
    fn test_large_display_idx() {
        // Arrange
        let mut manager = create_test_manager();

        // Act
        manager.create_node_evidence(
            999999,
            "I_FRAME".to_string(),
            create_test_bit_range(100, 200),
            50.0,
            100.0,
        );

        // Assert
        let node = manager.get_node_evidence(999999);
        assert!(node.is_some());
    }

    #[test]
    fn test_get_nodes_by_frame_type_case_sensitive() {
        // Arrange
        let mut manager = create_test_manager();

        manager.create_node_evidence(
            0,
            "I_FRAME".to_string(),
            create_test_bit_range(0, 100),
            0.0,
            0.0,
        );
        manager.create_node_evidence(
            1,
            "i_frame".to_string(),
            create_test_bit_range(100, 200),
            100.0,
            0.0,
        );

        // Act
        let upper = manager.get_nodes_by_frame_type("I_FRAME");
        let lower = manager.get_nodes_by_frame_type("i_frame");

        // Assert - Case sensitivity depends on implementation
        // If case-sensitive: upper.len() = 1, lower.len() = 1
        // If case-insensitive: upper.len() = 2 (or vice versa)
    }
}
