#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(unused_comparisons)]
#![allow(unused_doc_comments)]
//! Tests for reference graph evidence manager

use bitvue_core::evidence::{DecodeArtifactType, SyntaxNodeType, VizElementType};
use bitvue_core::reference_graph::{ReferenceEdge, ReferenceType};
use bitvue_core::reference_graph_evidence::ReferenceGraphEvidenceManager;
use bitvue_core::BitRange;

#[test]
fn test_create_node_evidence() {
    let mut manager = ReferenceGraphEvidenceManager::default();

    let bit_range = BitRange::new(0, 1000);
    let evidence = manager.create_node_evidence(0, "I".to_string(), bit_range, 100.0, 200.0);

    assert_eq!(evidence.display_idx, 0);
    assert!(!evidence.bit_offset_id.is_empty());
    assert!(!evidence.syntax_id.is_empty());
    assert!(!evidence.decode_id.is_empty());
    assert!(!evidence.viz_id.is_empty());
}

#[test]
fn test_create_edge_evidence() {
    let mut manager = ReferenceGraphEvidenceManager::default();

    // Create nodes first
    manager.create_node_evidence(0, "I".to_string(), BitRange::new(0, 1000), 100.0, 200.0);
    manager.create_node_evidence(1, "P".to_string(), BitRange::new(1000, 2000), 200.0, 200.0);

    // Create edge
    let edge = ReferenceEdge {
        from_idx: 1,
        to_idx: 0,
        ref_type: ReferenceType::L0,
        weight: None,
    };
    let bit_range = BitRange::new(1500, 1600);

    let evidence = manager.create_edge_evidence(&edge, bit_range);

    assert_eq!(evidence.from_idx, 1);
    assert_eq!(evidence.to_idx, 0);
    assert_eq!(evidence.ref_type, ReferenceType::L0);
    assert!(!evidence.syntax_id.is_empty());
    assert!(!evidence.viz_id.is_empty());
}

#[test]
fn test_get_node_evidence() {
    let mut manager = ReferenceGraphEvidenceManager::default();

    let bit_range = BitRange::new(0, 1000);
    manager.create_node_evidence(5, "P".to_string(), bit_range, 300.0, 400.0);

    let retrieved = manager.get_node_evidence(5);
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().display_idx, 5);

    let missing = manager.get_node_evidence(10);
    assert!(missing.is_none());
}

#[test]
fn test_get_edge_evidence() {
    let mut manager = ReferenceGraphEvidenceManager::default();

    manager.create_node_evidence(0, "I".to_string(), BitRange::new(0, 1000), 100.0, 200.0);
    manager.create_node_evidence(1, "P".to_string(), BitRange::new(1000, 2000), 200.0, 200.0);

    let edge = ReferenceEdge {
        from_idx: 1,
        to_idx: 0,
        ref_type: ReferenceType::L0,
        weight: None,
    };
    manager.create_edge_evidence(&edge, BitRange::new(1500, 1600));

    let retrieved = manager.get_edge_evidence(1, 0);
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().ref_type, ReferenceType::L0);

    let missing = manager.get_edge_evidence(0, 1);
    assert!(missing.is_none());
}

#[test]
fn test_evidence_chain_traversal() {
    let mut manager = ReferenceGraphEvidenceManager::default();

    let bit_range = BitRange::new(0, 1000);
    let node_ev = manager.create_node_evidence(0, "I".to_string(), bit_range, 100.0, 200.0);

    // Forward: bit_offset -> syntax
    let syntax_ev = manager
        .evidence_chain()
        .syntax_index
        .find_by_id(&node_ev.syntax_id);
    assert!(syntax_ev.is_some());
    assert_eq!(syntax_ev.unwrap().bit_offset_link, node_ev.bit_offset_id);

    // syntax -> decode
    let decode_ev = manager
        .evidence_chain()
        .decode_index
        .find_by_id(&node_ev.decode_id);
    assert!(decode_ev.is_some());
    assert_eq!(decode_ev.unwrap().syntax_link, node_ev.syntax_id);

    // decode -> viz
    let viz_ev = manager
        .evidence_chain()
        .viz_index
        .find_by_id(&node_ev.viz_id);
    assert!(viz_ev.is_some());
    assert_eq!(viz_ev.unwrap().decode_link, node_ev.decode_id);
}

#[test]
fn test_node_and_edge_counts() {
    let mut manager = ReferenceGraphEvidenceManager::default();

    assert_eq!(manager.node_count(), 0);
    assert_eq!(manager.edge_count(), 0);

    // Add nodes
    for i in 0..5 {
        manager.create_node_evidence(
            i,
            "P".to_string(),
            BitRange::new(i as u64 * 1000, (i as u64 + 1) * 1000),
            i as f32 * 100.0,
            200.0,
        );
    }

    assert_eq!(manager.node_count(), 5);

    // Add edges
    for i in 1..5 {
        let edge = ReferenceEdge {
            from_idx: i,
            to_idx: i - 1,
            ref_type: ReferenceType::L0,
            weight: None,
        };
        manager.create_edge_evidence(
            &edge,
            BitRange::new(i as u64 * 1000 + 500, i as u64 * 1000 + 600),
        );
    }

    assert_eq!(manager.edge_count(), 4);
}

#[test]
fn test_clear() {
    let mut manager = ReferenceGraphEvidenceManager::default();

    manager.create_node_evidence(0, "I".to_string(), BitRange::new(0, 1000), 100.0, 200.0);
    manager.create_node_evidence(1, "P".to_string(), BitRange::new(1000, 2000), 200.0, 200.0);

    assert_eq!(manager.node_count(), 2);

    manager.clear();

    assert_eq!(manager.node_count(), 0);
    assert_eq!(manager.edge_count(), 0);
}

// UX Graph evidence chain test - Task 9 (S.T4-1.ALL.UX.Graph.impl.evidence_chain.001)

#[test]
fn test_ux_graph_node_click_traces_to_bit_offset() {
    // UX Graph: User clicks on a reference graph node to trace it back to bitstream
    let mut manager = ReferenceGraphEvidenceManager::default();

    // UX Graph: Setup reference graph with 3 frames
    let bit_range_0 = BitRange::new(0, 5000);
    let bit_range_1 = BitRange::new(5000, 12000);
    let bit_range_2 = BitRange::new(12000, 18000);

    let node_0 =
        manager.create_node_evidence(0, "I".to_string(), bit_range_0.clone(), 100.0, 200.0);
    let node_1 =
        manager.create_node_evidence(1, "P".to_string(), bit_range_1.clone(), 200.0, 200.0);
    let node_2 =
        manager.create_node_evidence(2, "B".to_string(), bit_range_2.clone(), 300.0, 200.0);

    // UX Graph: Create reference edges: 1->0, 2->1, 2->0
    let edge_1_0 = ReferenceEdge {
        from_idx: 1,
        to_idx: 0,
        ref_type: ReferenceType::L0,
        weight: None,
    };
    let edge_2_1 = ReferenceEdge {
        from_idx: 2,
        to_idx: 1,
        ref_type: ReferenceType::L0,
        weight: None,
    };
    let edge_2_0 = ReferenceEdge {
        from_idx: 2,
        to_idx: 0,
        ref_type: ReferenceType::L1,
        weight: None,
    };

    let edge_ev_1_0 = manager.create_edge_evidence(&edge_1_0, BitRange::new(6000, 6100));
    let edge_ev_2_1 = manager.create_edge_evidence(&edge_2_1, BitRange::new(13000, 13100));
    let edge_ev_2_0 = manager.create_edge_evidence(&edge_2_0, BitRange::new(13100, 13200));

    // UX Graph: User clicks on node 2 (B-frame) to see its bitstream location
    let node_2_evidence = manager.get_node_evidence(2).unwrap();

    // UX Graph: Trace from viz -> decode -> syntax -> bit_offset
    let viz_ev = manager
        .evidence_chain()
        .viz_index
        .find_by_id(&node_2_evidence.viz_id)
        .unwrap();
    assert_eq!(viz_ev.element_type, VizElementType::ReferenceGraphNode);
    assert_eq!(
        viz_ev.visual_properties.get("frame_type"),
        Some(&"B".to_string())
    );

    let decode_ev = manager
        .evidence_chain()
        .decode_index
        .find_by_id(&node_2_evidence.decode_id)
        .unwrap();
    assert_eq!(decode_ev.artifact_type, DecodeArtifactType::YuvFrame);
    assert_eq!(decode_ev.display_idx, Some(2));

    let syntax_ev = manager
        .evidence_chain()
        .syntax_index
        .find_by_id(&node_2_evidence.syntax_id)
        .unwrap();
    assert_eq!(syntax_ev.node_type, SyntaxNodeType::FrameHeader);
    assert_eq!(
        syntax_ev.metadata.get("display_idx"),
        Some(&"2".to_string())
    );

    let bit_offset_ev = manager
        .evidence_chain()
        .bit_offset_index
        .find_by_id(&node_2_evidence.bit_offset_id)
        .unwrap();
    assert_eq!(bit_offset_ev.bit_range, bit_range_2);

    // UX Graph: Verify evidence chain linkage is bidirectional
    assert_eq!(syntax_ev.bit_offset_link, node_2_evidence.bit_offset_id);
    assert_eq!(decode_ev.syntax_link, node_2_evidence.syntax_id);
    assert_eq!(viz_ev.decode_link, node_2_evidence.decode_id);

    // UX Graph: User clicks on edge (2->1) to see reference list bitstream location
    let edge_2_1_evidence = manager.get_edge_evidence(2, 1).unwrap();

    let edge_viz_ev = manager
        .evidence_chain()
        .viz_index
        .find_by_id(&edge_2_1_evidence.viz_id)
        .unwrap();
    assert_eq!(
        edge_viz_ev.visual_properties.get("from_idx"),
        Some(&"2".to_string())
    );
    assert_eq!(
        edge_viz_ev.visual_properties.get("to_idx"),
        Some(&"1".to_string())
    );
    assert_eq!(
        edge_viz_ev.visual_properties.get("ref_type"),
        Some(&"L0".to_string())
    );

    let edge_syntax_ev = manager
        .evidence_chain()
        .syntax_index
        .find_by_id(&edge_2_1_evidence.syntax_id)
        .unwrap();
    assert_eq!(
        edge_syntax_ev.metadata.get("from_idx"),
        Some(&"2".to_string())
    );
    assert_eq!(
        edge_syntax_ev.metadata.get("to_idx"),
        Some(&"1".to_string())
    );

    // UX Graph: Verify edge (2->0) uses L1 reference type
    let edge_2_0_evidence = manager.get_edge_evidence(2, 0).unwrap();
    let edge_2_0_viz = manager
        .evidence_chain()
        .viz_index
        .find_by_id(&edge_2_0_evidence.viz_id)
        .unwrap();
    assert_eq!(
        edge_2_0_viz.visual_properties.get("ref_type"),
        Some(&"L1".to_string())
    );
    assert_eq!(
        edge_2_0_viz.visual_properties.get("color_hint"),
        Some(&"green".to_string())
    );
}
