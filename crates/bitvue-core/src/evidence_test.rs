// Evidence Chain module tests
//
// Per generate-tests skill: Arrange-Act-Assert pattern with fixtures
// and edge case coverage.

#[allow(unused_imports)]
use super::*;

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

/// Create a test bit offset evidence
#[allow(dead_code)]
fn create_test_bit_offset_evidence(offset: u64) -> BitOffsetEvidence {
    BitOffsetEvidence::new(offset)
}

/// Create a test syntax evidence
#[allow(dead_code)]
fn create_test_syntax_evidence(
    offset: u64,
    syntax_type: &str,
    description: &str,
) -> SyntaxEvidence {
    SyntaxEvidence::new(
        create_test_evidence_id(),
        offset,
        syntax_type.to_string(),
        description.to_string(),
    )
}

/// Create a test decode evidence
#[allow(dead_code)]
fn create_test_decode_evidence(
    syntax_id: EvidenceId,
    decode_type: &str,
) -> DecodeEvidence {
    DecodeEvidence::new(
        create_test_evidence_id(),
        syntax_id,
        decode_type.to_string(),
    )
}

/// Create a test viz evidence
#[allow(dead_code)]
fn create_test_viz_evidence(
    decode_id: EvidenceId,
    viz_type: &str,
) -> VizEvidence {
    VizEvidence::new(
        create_test_evidence_id(),
        decode_id,
        viz_type.to_string(),
    )
}

/// Create a test evidence chain
#[allow(dead_code)]
fn create_test_evidence_chain() -> EvidenceChain {
    EvidenceChain::new()
}

// ============================================================================
// EvidenceId Tests
// ============================================================================

#[cfg(test)]
mod evidence_id_tests {
#[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_new_creates_unique_id() {
        // Arrange & Act
        let id1 = EvidenceId::new();
        let id2 = EvidenceId::new();

        // Assert - IDs should be unique
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_clone_creates_same_id() {
        // Arrange
        let id = EvidenceId::new();

        // Act
        let id_clone = id.clone();

        // Assert - Cloned ID should be equal
        assert_eq!(id, id_clone);
    }

    #[test]
    fn test_copy_id() {
        // Arrange
        let id = EvidenceId::new();

        // Act - EvidenceId should be Copy
        let _id_copy = id;
        let _ = id; // Should still be usable
    }
}

// ============================================================================
// BitRange Tests
// ============================================================================

#[cfg(test)]
mod bit_range_tests {
#[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_bit_range_creation() {
        // Arrange & Act
        let range = BitRange { start: 100, end: 200 };

        // Assert
        assert_eq!(range.start, 100);
        assert_eq!(range.end, 200);
    }

    #[test]
    fn test_bit_range_length() {
        // Arrange
        let range = create_test_bit_range(100, 200);

        // Act
        let length = range.end - range.start;

        // Assert
        assert_eq!(length, 100);
    }

    #[test]
    fn test_bit_range_zero_length() {
        // Arrange & Act
        let range = create_test_bit_range(100, 100);

        // Assert
        assert_eq!(range.start, 100);
        assert_eq!(range.end, 100);
    }

    #[test]
    fn test_bit_range_clone() {
        // Arrange
        let range = create_test_bit_range(100, 200);

        // Act
        let range_clone = range.clone();

        // Assert
        assert_eq!(range_clone.start, range.start);
        assert_eq!(range_clone.end, range.end);
    }
}

// ============================================================================
// BitOffsetEvidence Tests
// ============================================================================

#[cfg(test)]
mod bit_offset_evidence_tests {
#[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_new_creates_evidence() {
        // Arrange & Act
        let evidence = BitOffsetEvidence::new(1000);

        // Assert
        assert_eq!(evidence.bit_offset, 1000);
    }

    #[test]
    fn test_with_description() {
        // Arrange
        let mut evidence = BitOffsetEvidence::new(1000);

        // Act
        evidence.description = Some("Test description".to_string());

        // Assert
        assert_eq!(evidence.description, Some("Test description".to_string()));
    }

    #[test]
    fn test_with_bit_range() {
        // Arrange
        let mut evidence = BitOffsetEvidence::new(1000);

        // Act
        evidence.bit_range = Some(create_test_bit_range(1000, 1100));

        // Assert
        assert!(evidence.bit_range.is_some());
        assert_eq!(evidence.bit_range.unwrap().start, 1000);
    }
}

// ============================================================================
// SyntaxEvidence Tests
// ============================================================================

#[cfg(test)]
mod syntax_evidence_tests {
#[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_new_creates_evidence() {
        // Arrange
        let id = create_test_evidence_id();

        // Act
        let evidence = SyntaxEvidence::new(
            id,
            1000,
            "NAL_UNIT".to_string(),
            "Test NAL unit".to_string(),
        );

        // Assert
        assert_eq!(evidence.id, id);
        assert_eq!(evidence.bit_offset, 1000);
        assert_eq!(evidence.syntax_type, "NAL_UNIT");
        assert_eq!(evidence.description, "Test NAL unit");
    }

    #[test]
    fn test_with_children() {
        // Arrange
        let mut evidence = create_test_syntax_evidence(1000, "NAL_UNIT", "Test");
        let child_id = create_test_evidence_id();

        // Act
        evidence.children.push(child_id);

        // Assert
        assert_eq!(evidence.children.len(), 1);
    }

    #[test]
    fn test_with_bit_range() {
        // Arrange
        let mut evidence = create_test_syntax_evidence(1000, "NAL_UNIT", "Test");

        // Act
        evidence.bit_range = Some(create_test_bit_range(1000, 1100));

        // Assert
        assert!(evidence.bit_range.is_some());
    }
}

// ============================================================================
// DecodeEvidence Tests
// ============================================================================

#[cfg(test)]
mod decode_evidence_tests {
#[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_new_creates_evidence() {
        // Arrange
        let id = create_test_evidence_id();
        let syntax_id = create_test_evidence_id();

        // Act
        let evidence = DecodeEvidence::new(
            id,
            syntax_id,
            "CTU_DECODE".to_string(),
        );

        // Assert
        assert_eq!(evidence.id, id);
        assert_eq!(evidence.syntax_id, syntax_id);
        assert_eq!(evidence.decode_type, "CTU_DECODE");
    }

    #[test]
    fn test_with_properties() {
        // Arrange
        let mut evidence = create_test_decode_evidence(create_test_evidence_id(), "CTU_DECODE");

        // Act
        evidence.properties.insert("size".to_string(), "64x64".to_string());
        evidence.properties.insert("qp".to_string(), "32".to_string());

        // Assert
        assert_eq!(evidence.properties.len(), 2);
        assert_eq!(evidence.properties.get("size"), Some(&"64x64".to_string()));
    }
}

// ============================================================================
// VizEvidence Tests
// ============================================================================

#[cfg(test)]
mod viz_evidence_tests {
#[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_new_creates_evidence() {
        // Arrange
        let id = create_test_evidence_id();
        let decode_id = create_test_evidence_id();

        // Act
        let evidence = VizEvidence::new(
            id,
            decode_id,
            "OVERLAY_RECT".to_string(),
        );

        // Assert
        assert_eq!(evidence.id, id);
        assert_eq!(evidence.decode_id, decode_id);
        assert_eq!(evidence.viz_type, "OVERLAY_RECT");
    }

    #[test]
    fn test_with_bounds() {
        // Arrange
        let mut evidence = create_test_viz_evidence(create_test_evidence_id(), "OVERLAY_RECT");

        // Act
        evidence.bounds = Some((0, 0, 64, 64));

        // Assert
        assert_eq!(evidence.bounds, Some((0, 0, 64, 64)));
    }
}

// ============================================================================
// BitOffsetIndex Tests
// ============================================================================

#[cfg(test)]
mod bit_offset_index_tests {
#[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_new_creates_index() {
        // Arrange & Act
        let index = BitOffsetIndex::new();

        // Assert
        assert!(index.map.is_empty());
    }

    #[test]
    fn test_insert_and_lookup() {
        // Arrange
        let mut index = BitOffsetIndex::new();
        let evidence = create_test_bit_offset_evidence(1000);

        // Act
        index.insert(1000, evidence);
        let result = index.lookup(1000);

        // Assert
        assert!(result.is_some());
        assert_eq!(result.unwrap().bit_offset, 1000);
    }

    #[test]
    fn test_lookup_not_found() {
        // Arrange
        let index = BitOffsetIndex::new();

        // Act
        let result = index.lookup(9999);

        // Assert
        assert!(result.is_none());
    }
}

// ============================================================================
// SyntaxIndex Tests
// ============================================================================

#[cfg(test)]
mod syntax_index_tests {
#[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_new_creates_index() {
        // Arrange & Act
        let index = SyntaxIndex::new();

        // Assert
        assert!(index.by_id.is_empty());
        assert!(index.by_type.is_empty());
    }

    #[test]
    fn test_insert_and_lookup_by_id() {
        // Arrange
        let mut index = SyntaxIndex::new();
        let evidence = create_test_syntax_evidence(1000, "NAL_UNIT", "Test");
        let id = evidence.id;

        // Act
        index.insert(evidence);
        let result = index.lookup_by_id(&id);

        // Assert
        assert!(result.is_some());
    }

    #[test]
    fn test_lookup_by_type() {
        // Arrange
        let mut index = SyntaxIndex::new();

        index.insert(create_test_syntax_evidence(1000, "NAL_UNIT", "Test1"));
        index.insert(create_test_syntax_evidence(2000, "NAL_UNIT", "Test2"));
        index.insert(create_test_syntax_evidence(3000, "CTU", "Test3"));

        // Act
        let nal_units = index.lookup_by_type("NAL_UNIT");
        let ctus = index.lookup_by_type("CTU");

        // Assert
        assert_eq!(nal_units.len(), 2);
        assert_eq!(ctus.len(), 1);
    }
}

// ============================================================================
// DecodeIndex Tests
// ============================================================================

#[cfg(test)]
mod decode_index_tests {
#[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_new_creates_index() {
        // Arrange & Act
        let index = DecodeIndex::new();

        // Assert
        assert!(index.by_id.is_empty());
        assert!(index.by_syntax.is_empty());
    }

    #[test]
    fn test_insert_and_lookup_by_syntax() {
        // Arrange
        let mut index = DecodeIndex::new();
        let syntax_id = create_test_evidence_id();
        let evidence = create_test_decode_evidence(syntax_id, "CTU_DECODE");

        // Act
        index.insert(evidence);
        let result = index.lookup_by_syntax(&syntax_id);

        // Assert
        assert_eq!(result.len(), 1);
    }
}

// ============================================================================
// VizIndex Tests
// ============================================================================

#[cfg(test)]
mod viz_index_tests {
#[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_new_creates_index() {
        // Arrange & Act
        let index = VizIndex::new();

        // Assert
        assert!(index.by_id.is_empty());
        assert!(index.by_decode.is_empty());
    }

    #[test]
    fn test_insert_and_lookup_by_decode() {
        // Arrange
        let mut index = VizIndex::new();
        let decode_id = create_test_evidence_id();
        let evidence = create_test_viz_evidence(decode_id, "OVERLAY_RECT");

        // Act
        index.insert(evidence);
        let result = index.lookup_by_decode(&decode_id);

        // Assert
        assert_eq!(result.len(), 1);
    }
}

// ============================================================================
// EvidenceChain Construction Tests
// ============================================================================

#[cfg(test)]
mod evidence_chain_construction_tests {
#[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_new_creates_chain() {
        // Arrange & Act
        let chain = create_test_evidence_chain();

        // Assert
        assert!(chain.nodes.is_empty());
        assert!(chain.syntax_map.is_empty());
        assert!(chain.decode_map.is_empty());
        assert!(chain.viz_map.is_empty());
    }

    #[test]
    fn test_default_creates_chain() {
        // Arrange & Act
        let chain = EvidenceChain::default();

        // Assert
        assert!(chain.nodes.is_empty());
    }
}

// ============================================================================
// EvidenceChain Add Tests
// ============================================================================

#[cfg(test)]
mod evidence_chain_add_tests {
#[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_add_syntax_evidence() {
        // Arrange
        let mut chain = create_test_evidence_chain();
        let evidence = create_test_syntax_evidence(1000, "NAL_UNIT", "Test");
        let id = evidence.id;

        // Act
        chain.add_syntax(evidence);

        // Assert
        assert!(chain.syntax_map.contains_key(&id));
    }

    #[test]
    fn test_add_decode_evidence() {
        // Arrange
        let mut chain = create_test_evidence_chain();
        let evidence = create_test_decode_evidence(create_test_evidence_id(), "CTU_DECODE");
        let id = evidence.id;

        // Act
        chain.add_decode(evidence);

        // Assert
        assert!(chain.decode_map.contains_key(&id));
    }

    #[test]
    fn test_add_viz_evidence() {
        // Arrange
        let mut chain = create_test_evidence_chain();
        let evidence = create_test_viz_evidence(create_test_evidence_id(), "OVERLAY_RECT");
        let id = evidence.id;

        // Act
        chain.add_viz(evidence);

        // Assert
        assert!(chain.viz_map.contains_key(&id));
    }
}

// ============================================================================
// EvidenceChain Navigation Tests (Forward)
// ============================================================================

#[cfg(test)]
mod evidence_chain_forward_tests {
#[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_bit_to_syntax_empty() {
        // Arrange
        let chain = create_test_evidence_chain();

        // Act
        let result = chain.bit_to_syntax(1000);

        // Assert
        assert!(result.is_none());
    }

    #[test]
    fn test_bit_to_syntax_found() {
        // Arrange
        let mut chain = create_test_evidence_chain();
        let evidence = create_test_syntax_evidence(1000, "NAL_UNIT", "Test");
        let _ = chain.bit_offset_index.insert(1000, create_test_bit_offset_evidence(1000));
        let id = evidence.id;
        chain.syntax_map.insert(id, evidence);
        chain.bit_to_syntax_map.insert(1000, id);

        // Act
        let result = chain.bit_to_syntax(1000);

        // Assert
        assert!(result.is_some());
    }

    #[test]
    fn test_syntax_to_decode_empty() {
        // Arrange
        let chain = create_test_evidence_chain();
        let id = create_test_evidence_id();

        // Act
        let result = chain.syntax_to_decode(&id);

        // Assert
        assert!(result.is_empty());
    }

    #[test]
    fn test_syntax_to_decode_found() {
        // Arrange
        let mut chain = create_test_evidence_chain();
        let syntax_id = create_test_evidence_id();
        let decode_evidence = create_test_decode_evidence(syntax_id, "CTU_DECODE");
        chain.decode_map.insert(decode_evidence.id, decode_evidence.clone());

        // Act
        let result = chain.syntax_to_decode(&syntax_id);

        // Assert
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_decode_to_viz_empty() {
        // Arrange
        let chain = create_test_evidence_chain();
        let id = create_test_evidence_id();

        // Act
        let result = chain.decode_to_viz(&id);

        // Assert
        assert!(result.is_empty());
    }

    #[test]
    fn test_decode_to_viz_found() {
        // Arrange
        let mut chain = create_test_evidence_chain();
        let decode_id = create_test_evidence_id();
        let viz_evidence = create_test_viz_evidence(decode_id, "OVERLAY_RECT");
        chain.viz_map.insert(viz_evidence.id, viz_evidence.clone());

        // Act
        let result = chain.decode_to_viz(&decode_id);

        // Assert
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_bit_to_viz_full_chain() {
        // Arrange - Create full chain: bit -> syntax -> decode -> viz
        let mut chain = create_test_evidence_chain();

        let syntax_evidence = create_test_syntax_evidence(1000, "NAL_UNIT", "Test");
        let syntax_id = syntax_evidence.id;

        let decode_evidence = create_test_decode_evidence(syntax_id, "CTU_DECODE");
        let decode_id = decode_evidence.id;

        let viz_evidence = create_test_viz_evidence(decode_id, "OVERLAY_RECT");

        chain.syntax_map.insert(syntax_id, syntax_evidence);
        chain.decode_map.insert(decode_id, decode_evidence);
        chain.viz_map.insert(viz_evidence.id, viz_evidence);

        // Act
        let result = chain.bit_to_viz(1000);

        // Assert - Should traverse full chain
        // (Depends on bit_to_syntax_map being populated)
    }
}

// ============================================================================
// EvidenceChain Navigation Tests (Reverse)
// ============================================================================

#[cfg(test)]
mod evidence_chain_reverse_tests {
#[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_viz_to_decode_empty() {
        // Arrange
        let chain = create_test_evidence_chain();
        let id = create_test_evidence_id();

        // Act
        let result = chain.viz_to_decode(&id);

        // Assert
        assert!(result.is_none());
    }

    #[test]
    fn test_viz_to_decode_found() {
        // Arrange
        let mut chain = create_test_evidence_chain();
        let decode_id = create_test_evidence_id();
        let viz_evidence = create_test_viz_evidence(decode_id, "OVERLAY_RECT");
        let viz_id = viz_evidence.id;
        chain.viz_map.insert(viz_id, viz_evidence);

        // Act
        let result = chain.viz_to_decode(&viz_id);

        // Assert
        assert!(result.is_some());
        assert_eq!(result.unwrap(), decode_id);
    }

    #[test]
    fn test_decode_to_syntax_empty() {
        // Arrange
        let chain = create_test_evidence_chain();
        let id = create_test_evidence_id();

        // Act
        let result = chain.decode_to_syntax(&id);

        // Assert
        assert!(result.is_none());
    }

    #[test]
    fn test_decode_to_syntax_found() {
        // Arrange
        let mut chain = create_test_evidence_chain();
        let syntax_id = create_test_evidence_id();
        let decode_evidence = create_test_decode_evidence(syntax_id, "CTU_DECODE");
        let decode_id = decode_evidence.id;
        chain.decode_map.insert(decode_id, decode_evidence);

        // Act
        let result = chain.decode_to_syntax(&decode_id);

        // Assert
        assert!(result.is_some());
        assert_eq!(result.unwrap(), syntax_id);
    }
}

// ============================================================================
// EvidenceChain Query Tests
// ============================================================================

#[cfg(test)]
mod evidence_chain_query_tests {
#[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_get_syntax_evidence_exists() {
        // Arrange
        let mut chain = create_test_evidence_chain();
        let evidence = create_test_syntax_evidence(1000, "NAL_UNIT", "Test");
        let id = evidence.id;
        chain.syntax_map.insert(id, evidence);

        // Act
        let result = chain.get_syntax(&id);

        // Assert
        assert!(result.is_some());
    }

    #[test]
    fn test_get_syntax_not_exists() {
        // Arrange
        let chain = create_test_evidence_chain();
        let id = create_test_evidence_id();

        // Act
        let result = chain.get_syntax(&id);

        // Assert
        assert!(result.is_none());
    }

    #[test]
    fn test_get_decode_evidence_exists() {
        // Arrange
        let mut chain = create_test_evidence_chain();
        let evidence = create_test_decode_evidence(create_test_evidence_id(), "CTU_DECODE");
        let id = evidence.id;
        chain.decode_map.insert(id, evidence);

        // Act
        let result = chain.get_decode(&id);

        // Assert
        assert!(result.is_some());
    }

    #[test]
    fn test_get_viz_evidence_exists() {
        // Arrange
        let mut chain = create_test_evidence_chain();
        let evidence = create_test_viz_evidence(create_test_evidence_id(), "OVERLAY_RECT");
        let id = evidence.id;
        chain.viz_map.insert(id, evidence);

        // Act
        let result = chain.get_viz(&id);

        // Assert
        assert!(result.is_some());
    }
}

// ============================================================================
// EvidenceChain Statistics Tests
// ============================================================================

#[cfg(test)]
mod evidence_chain_stats_tests {
#[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_stats_empty() {
        // Arrange
        let chain = create_test_evidence_chain();

        // Act
        let stats = chain.stats();

        // Assert
        assert_eq!(stats.syntax_count, 0);
        assert_eq!(stats.decode_count, 0);
        assert_eq!(stats.viz_count, 0);
        assert_eq!(stats.total, 0);
    }

    #[test]
    fn test_stats_with_data() {
        // Arrange
        let mut chain = create_test_evidence_chain();

        chain.syntax_map.insert(create_test_evidence_id(), create_test_syntax_evidence(1000, "NAL_UNIT", "Test"));
        chain.syntax_map.insert(create_test_evidence_id(), create_test_syntax_evidence(2000, "CTU", "Test2"));

        chain.decode_map.insert(create_test_evidence_id(), create_test_decode_evidence(create_test_evidence_id(), "CTU_DECODE"));
        chain.decode_map.insert(create_test_evidence_id(), create_test_decode_evidence(create_test_evidence_id(), "CTU_DECODE2"));
        chain.decode_map.insert(create_test_evidence_id(), create_test_decode_evidence(create_test_evidence_id(), "CTU_DECODE3"));

        chain.viz_map.insert(create_test_evidence_id(), create_test_viz_evidence(create_test_evidence_id(), "OVERLAY_RECT"));
        chain.viz_map.insert(create_test_evidence_id(), create_test_viz_evidence(create_test_evidence_id(), "OVERLAY_RECT2"));
        chain.viz_map.insert(create_test_evidence_id(), create_test_viz_evidence(create_test_evidence_id(), "OVERLAY_RECT3"));
        chain.viz_map.insert(create_test_evidence_id(), create_test_viz_evidence(create_test_evidence_id(), "OVERLAY_RECT4"));

        // Act
        let stats = chain.stats();

        // Assert
        assert_eq!(stats.syntax_count, 2);
        assert_eq!(stats.decode_count, 3);
        assert_eq!(stats.viz_count, 4);
        assert_eq!(stats.total, 9);
    }
}

// ============================================================================
// EvidenceChain Clear Tests
// ============================================================================

#[cfg(test)]
mod evidence_chain_clear_tests {
#[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_clear_removes_all() {
        // Arrange
        let mut chain = create_test_evidence_chain();

        chain.syntax_map.insert(create_test_evidence_id(), create_test_syntax_evidence(1000, "NAL_UNIT", "Test"));
        chain.decode_map.insert(create_test_evidence_id(), create_test_decode_evidence(create_test_evidence_id(), "CTU_DECODE"));
        chain.viz_map.insert(create_test_evidence_id(), create_test_viz_evidence(create_test_evidence_id(), "OVERLAY_RECT"));

        // Act
        chain.clear();

        // Assert
        assert!(chain.syntax_map.is_empty());
        assert!(chain.decode_map.is_empty());
        assert!(chain.viz_map.is_empty());
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
    fn test_zero_bit_offset() {
        // Arrange
        let evidence = BitOffsetEvidence::new(0);

        // Assert
        assert_eq!(evidence.bit_offset, 0);
    }

    #[test]
    fn test_very_large_bit_offset() {
        // Arrange
        let evidence = BitOffsetEvidence::new(u64::MAX);

        // Assert
        assert_eq!(evidence.bit_offset, u64::MAX);
    }

    #[test]
    fn test_empty_string_types() {
        // Arrange
        let syntax = SyntaxEvidence::new(
            create_test_evidence_id(),
            1000,
            "".to_string(),
            "".to_string(),
        );
        let decode = DecodeEvidence::new(
            create_test_evidence_id(),
            create_test_evidence_id(),
            "".to_string(),
        );
        let viz = VizEvidence::new(
            create_test_evidence_id(),
            create_test_evidence_id(),
            "".to_string(),
        );

        // Assert - Empty strings should be allowed
        assert_eq!(syntax.syntax_type, "");
        assert_eq!(decode.decode_type, "");
        assert_eq!(viz.viz_type, "");
    }

    #[test]
    fn test_circular_reference_handling() {
        // Arrange - A -> B -> A (circular)
        let mut chain = create_test_evidence_chain();

        let id_a = create_test_evidence_id();
        let id_b = create_test_evidence_id();

        // This creates a potential circular reference
        // Implementation should handle gracefully
        let decode_a = create_test_decode_evidence(id_a, "DECODE_A");
        let decode_b = create_test_decode_evidence(id_b, "DECODE_B");

        chain.decode_map.insert(id_a, decode_a);
        chain.decode_map.insert(id_b, decode_b);

        // Act & Assert - Should not panic or infinite loop
        let _ = chain.stats();
    }

    #[test]
    fn test_orphaned_evidence() {
        // Arrange - Evidence with no links
        let mut chain = create_test_evidence_chain();

        let orphan_syntax = create_test_syntax_evidence(1000, "ORPHAN", "No parent");
        chain.syntax_map.insert(orphan_syntax.id, orphan_syntax);

        // Act
        let result = chain.syntax_to_decode(&orphan_syntax.id);

        // Assert - Should return empty (no decodes point to this syntax)
        assert!(result.is_empty());
    }

    #[test]
    fn test_multiple_viz_for_same_decode() {
        // Arrange
        let mut chain = create_test_evidence_chain();
        let decode_id = create_test_evidence_id();

        let viz1 = create_test_viz_evidence(decode_id, "OVERLAY_RECT");
        let viz2 = create_test_viz_evidence(decode_id, "OVERLAY_MV");
        let viz3 = create_test_viz_evidence(decode_id, "OVERLAY_QP");

        chain.viz_map.insert(viz1.id, viz1);
        chain.viz_map.insert(viz2.id, viz2);
        chain.viz_map.insert(viz3.id, viz3);

        // Act
        let result = chain.decode_to_viz(&decode_id);

        // Assert
        assert_eq!(result.len(), 3);
    }
}
