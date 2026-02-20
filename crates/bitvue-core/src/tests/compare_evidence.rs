#![allow(hidden_glob_reexports)]
#![allow(unreachable_code)]
#![allow(non_camel_case_types)]
#![allow(unused_assignments)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(unused_comparisons)]
#![allow(unused_doc_comments)]
//! Tests for compare_evidence module

use crate::alignment::FramePair;
use crate::compare_evidence::CompareEvidenceManager;
use crate::evidence::{DecodeArtifactType, SyntaxNodeType, VizElementType};
use crate::types::BitRange;

fn create_test_pair(frame_a: usize, frame_b: usize) -> FramePair {
    FramePair {
        stream_a_idx: Some(frame_a),
        stream_b_idx: Some(frame_b),
        pts_delta: Some(0),
        has_gap: false,
    }
}

#[test]
fn test_create_pair_evidence() {
    let mut manager = CompareEvidenceManager::default();

    let pair = create_test_pair(0, 0);
    let bit_range_a = BitRange::new(0, 1000);
    let bit_range_b = BitRange::new(0, 1200);

    let evidence = manager.create_pair_evidence(&pair, bit_range_a, bit_range_b, 1024, 1152);
    assert!(evidence.is_some());

    let evidence = evidence.unwrap();
    assert_eq!(evidence.frame_a_idx, 0);
    assert_eq!(evidence.frame_b_idx, 0);
    assert!(!evidence.stream_a.bit_offset_id.is_empty());
    assert!(!evidence.stream_b.bit_offset_id.is_empty());
    assert!(!evidence.diff_viz_id.is_empty());
}

#[test]
fn test_get_pair_evidence() {
    let mut manager = CompareEvidenceManager::default();

    let pair = create_test_pair(5, 7);
    let bit_range_a = BitRange::new(5000, 6000);
    let bit_range_b = BitRange::new(7000, 8000);

    manager.create_pair_evidence(&pair, bit_range_a, bit_range_b, 1024, 1024);

    let retrieved = manager.get_pair_evidence(5, 7);
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().frame_a_idx, 5);
    assert_eq!(retrieved.unwrap().frame_b_idx, 7);

    let missing = manager.get_pair_evidence(10, 15);
    assert!(missing.is_none());
}

#[test]
fn test_find_pairs_with_frame_a() {
    let mut manager = CompareEvidenceManager::default();

    // Create multiple pairs with same frame A
    for b in 0..3 {
        let pair = create_test_pair(0, b);
        let bit_range_a = BitRange::new(0, 1000);
        let bit_range_b = BitRange::new(b as u64 * 1000, (b as u64 + 1) * 1000);
        manager.create_pair_evidence(&pair, bit_range_a, bit_range_b, 1024, 1024);
    }

    let pairs = manager.find_pairs_with_frame_a(0);
    assert_eq!(pairs.len(), 3);

    let empty = manager.find_pairs_with_frame_a(5);
    assert_eq!(empty.len(), 0);
}

#[test]
fn test_find_pairs_with_frame_b() {
    let mut manager = CompareEvidenceManager::default();

    // Create multiple pairs with same frame B
    for a in 0..3 {
        let pair = create_test_pair(a, 0);
        let bit_range_a = BitRange::new(a as u64 * 1000, (a as u64 + 1) * 1000);
        let bit_range_b = BitRange::new(0, 1000);
        manager.create_pair_evidence(&pair, bit_range_a, bit_range_b, 1024, 1024);
    }

    let pairs = manager.find_pairs_with_frame_b(0);
    assert_eq!(pairs.len(), 3);

    let empty = manager.find_pairs_with_frame_b(5);
    assert_eq!(empty.len(), 0);
}

#[test]
fn test_evidence_chain_traversal_stream_a() {
    let mut manager = CompareEvidenceManager::default();

    let pair = create_test_pair(0, 0);
    let bit_range_a = BitRange::new(0, 1000);
    let bit_range_b = BitRange::new(0, 1000);

    let evidence = manager
        .create_pair_evidence(&pair, bit_range_a, bit_range_b, 1024, 1024)
        .unwrap();

    // Forward traversal for stream A: bit_offset → syntax
    let syntax_ev = manager
        .evidence_chain_a()
        .syntax_index
        .find_by_id(&evidence.stream_a.syntax_id);
    assert!(syntax_ev.is_some());
    assert_eq!(
        syntax_ev.unwrap().bit_offset_link,
        evidence.stream_a.bit_offset_id
    );

    // syntax → decode
    let decode_ev = manager
        .evidence_chain_a()
        .decode_index
        .find_by_id(&evidence.stream_a.decode_id);
    assert!(decode_ev.is_some());
    assert_eq!(decode_ev.unwrap().syntax_link, evidence.stream_a.syntax_id);
}

#[test]
fn test_evidence_chain_traversal_stream_b() {
    let mut manager = CompareEvidenceManager::default();

    let pair = create_test_pair(0, 0);
    let bit_range_a = BitRange::new(0, 1000);
    let bit_range_b = BitRange::new(0, 1000);

    let evidence = manager
        .create_pair_evidence(&pair, bit_range_a, bit_range_b, 1024, 1024)
        .unwrap();

    // Forward traversal for stream B: bit_offset → syntax
    let syntax_ev = manager
        .evidence_chain_b()
        .syntax_index
        .find_by_id(&evidence.stream_b.syntax_id);
    assert!(syntax_ev.is_some());
    assert_eq!(
        syntax_ev.unwrap().bit_offset_link,
        evidence.stream_b.bit_offset_id
    );

    // syntax → decode
    let decode_ev = manager
        .evidence_chain_b()
        .decode_index
        .find_by_id(&evidence.stream_b.decode_id);
    assert!(decode_ev.is_some());
    assert_eq!(decode_ev.unwrap().syntax_link, evidence.stream_b.syntax_id);
}

#[test]
fn test_diff_viz_evidence() {
    let mut manager = CompareEvidenceManager::default();

    let pair = create_test_pair(5, 7);
    let bit_range_a = BitRange::new(5000, 6000);
    let bit_range_b = BitRange::new(7000, 8000);

    let evidence = manager
        .create_pair_evidence(&pair, bit_range_a, bit_range_b, 1024, 1152)
        .unwrap();

    // Check diff heatmap viz evidence
    let diff_viz = manager
        .evidence_chain_a()
        .viz_index
        .find_by_id(&evidence.diff_viz_id);
    assert!(diff_viz.is_some());

    let viz = diff_viz.unwrap();
    assert_eq!(viz.element_type, VizElementType::DiffHeatmap);
    assert_eq!(
        viz.visual_properties.get("frame_a_idx"),
        Some(&"5".to_string())
    );
    assert_eq!(
        viz.visual_properties.get("frame_b_idx"),
        Some(&"7".to_string())
    );

    // Check metadata links to both streams
    assert_eq!(
        viz.metadata.get("stream_a_decode"),
        Some(&evidence.stream_a.decode_id)
    );
    assert_eq!(
        viz.metadata.get("stream_b_decode"),
        Some(&evidence.stream_b.decode_id)
    );
}

#[test]
fn test_clear() {
    let mut manager = CompareEvidenceManager::default();

    // Add pairs
    for i in 0..5 {
        let pair = create_test_pair(i, i);
        let bit_range = BitRange::new(i as u64 * 1000, (i as u64 + 1) * 1000);
        manager.create_pair_evidence(&pair, bit_range.clone(), bit_range, 1024, 1024);
    }

    assert_eq!(manager.pair_count(), 5);

    manager.clear();

    assert_eq!(manager.pair_count(), 0);
    assert!(manager.get_pair_evidence(0, 0).is_none());
}

// UX Compare evidence chain test - Task 11 (S.T4-1.ALL.UX.Compare.impl.evidence_chain.001)

#[test]
fn test_ux_compare_diff_heatmap_pixel_traces_to_both_bitstreams() {
    // UX Compare: User clicks on diff heatmap pixel to trace to both source bitstreams
    let mut manager = CompareEvidenceManager::default();

    // UX Compare: Setup compared frames from two streams
    let pair_0 = create_test_pair(0, 0);
    let pair_1 = create_test_pair(1, 1);
    let pair_2 = create_test_pair(2, 3); // Frame 2 in A aligns to frame 3 in B
    let pair_3 = create_test_pair(3, 4); // Frame 3 in A aligns to frame 4 in B

    let bit_range_a0 = BitRange::new(0, 5000);
    let bit_range_b0 = BitRange::new(0, 5200);
    let bit_range_a1 = BitRange::new(5000, 10000);
    let bit_range_b1 = BitRange::new(5200, 10500);
    let bit_range_a2 = BitRange::new(10000, 15000);
    let bit_range_b3 = BitRange::new(15600, 20800);
    let bit_range_a3 = BitRange::new(15000, 20000);
    let bit_range_b4 = BitRange::new(20800, 26000);

    let ev_0 = manager
        .create_pair_evidence(
            &pair_0,
            bit_range_a0.clone(),
            bit_range_b0.clone(),
            1024,
            1100,
        )
        .unwrap();
    let ev_1 = manager.create_pair_evidence(
        &pair_1,
        bit_range_a1.clone(),
        bit_range_b1.clone(),
        1024,
        1150,
    );
    let ev_2 = manager
        .create_pair_evidence(
            &pair_2,
            bit_range_a2.clone(),
            bit_range_b3.clone(),
            1024,
            1200,
        )
        .unwrap();
    let ev_3 = manager.create_pair_evidence(
        &pair_3,
        bit_range_a3.clone(),
        bit_range_b4.clone(),
        1024,
        1250,
    );

    // UX Compare: User hovers over diff heatmap pixel at position (320, 240)
    // This pixel shows difference between frame 2 in A and frame 3 in B
    let pair_evidence = manager.get_pair_evidence(2, 3).unwrap();

    // UX Compare: Trace from diff heatmap viz → stream A decode → syntax → bit_offset
    let diff_viz = manager
        .evidence_chain_a()
        .viz_index
        .find_by_id(&pair_evidence.diff_viz_id)
        .unwrap();
    assert_eq!(diff_viz.element_type, VizElementType::DiffHeatmap);
    assert_eq!(
        diff_viz.visual_properties.get("frame_a_idx"),
        Some(&"2".to_string())
    );
    assert_eq!(
        diff_viz.visual_properties.get("frame_b_idx"),
        Some(&"3".to_string())
    );

    // UX Compare: Trace stream A: decode → syntax → bit_offset
    let decode_a = manager
        .evidence_chain_a()
        .decode_index
        .find_by_id(&pair_evidence.stream_a.decode_id)
        .unwrap();
    assert_eq!(decode_a.artifact_type, DecodeArtifactType::YuvFrame);
    assert_eq!(decode_a.display_idx, Some(2));
    assert_eq!(decode_a.metadata.get("stream"), Some(&"A".to_string()));

    let syntax_a = manager
        .evidence_chain_a()
        .syntax_index
        .find_by_id(&pair_evidence.stream_a.syntax_id)
        .unwrap();
    assert_eq!(syntax_a.node_type, SyntaxNodeType::FrameHeader);
    assert_eq!(syntax_a.metadata.get("stream"), Some(&"A".to_string()));
    assert_eq!(
        syntax_a.metadata.get("frame_size"),
        Some(&"1024".to_string())
    );

    let bit_offset_a = manager
        .evidence_chain_a()
        .bit_offset_index
        .find_by_id(&pair_evidence.stream_a.bit_offset_id)
        .unwrap();
    assert_eq!(bit_offset_a.bit_range, bit_range_a2);

    // UX Compare: Trace stream B: decode → syntax → bit_offset
    let decode_b = manager
        .evidence_chain_b()
        .decode_index
        .find_by_id(&pair_evidence.stream_b.decode_id)
        .unwrap();
    assert_eq!(decode_b.artifact_type, DecodeArtifactType::YuvFrame);
    assert_eq!(decode_b.display_idx, Some(3));
    assert_eq!(decode_b.metadata.get("stream"), Some(&"B".to_string()));

    let syntax_b = manager
        .evidence_chain_b()
        .syntax_index
        .find_by_id(&pair_evidence.stream_b.syntax_id)
        .unwrap();
    assert_eq!(syntax_b.node_type, SyntaxNodeType::FrameHeader);
    assert_eq!(syntax_b.metadata.get("stream"), Some(&"B".to_string()));
    assert_eq!(
        syntax_b.metadata.get("frame_size"),
        Some(&"1200".to_string())
    );

    let bit_offset_b = manager
        .evidence_chain_b()
        .bit_offset_index
        .find_by_id(&pair_evidence.stream_b.bit_offset_id)
        .unwrap();
    assert_eq!(bit_offset_b.bit_range, bit_range_b3);

    // UX Compare: Verify evidence chain linkage for stream A
    assert_eq!(
        syntax_a.bit_offset_link,
        pair_evidence.stream_a.bit_offset_id
    );
    assert_eq!(decode_a.syntax_link, pair_evidence.stream_a.syntax_id);
    assert_eq!(diff_viz.decode_link, pair_evidence.stream_a.decode_id);

    // UX Compare: Verify evidence chain linkage for stream B
    assert_eq!(
        syntax_b.bit_offset_link,
        pair_evidence.stream_b.bit_offset_id
    );
    assert_eq!(decode_b.syntax_link, pair_evidence.stream_b.syntax_id);

    // UX Compare: Verify diff viz metadata links to both streams
    assert_eq!(
        diff_viz.metadata.get("stream_a_decode"),
        Some(&pair_evidence.stream_a.decode_id)
    );
    assert_eq!(
        diff_viz.metadata.get("stream_b_decode"),
        Some(&pair_evidence.stream_b.decode_id)
    );

    // UX Compare: User can jump to hex view for either stream
    // Stream A byte offset: bit_range_a2 starts at bit 10000 = byte 1250
    let byte_offset_a = bit_range_a2.byte_offset();
    assert_eq!(byte_offset_a, 1250);

    // Stream B byte offset: bit_range_b3 starts at bit 15600 = byte 1950
    let byte_offset_b = bit_range_b3.byte_offset();
    assert_eq!(byte_offset_b, 1950);

    // UX Compare: User can also query all pairs for a specific frame in stream A
    let pairs_with_frame_a2 = manager.find_pairs_with_frame_a(2);
    assert_eq!(pairs_with_frame_a2.len(), 1);
    assert_eq!(pairs_with_frame_a2[0].frame_a_idx, 2);
    assert_eq!(pairs_with_frame_a2[0].frame_b_idx, 3);

    // UX Compare: User can query all pairs for a specific frame in stream B
    let pairs_with_frame_b3 = manager.find_pairs_with_frame_b(3);
    assert_eq!(pairs_with_frame_b3.len(), 1);
    assert_eq!(pairs_with_frame_b3[0].frame_a_idx, 2);
    assert_eq!(pairs_with_frame_b3[0].frame_b_idx, 3);
}
