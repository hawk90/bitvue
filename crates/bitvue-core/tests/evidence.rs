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
//! Tests for evidence module

use bitvue_core::{
    BitOffsetEvidence, BitOffsetIndex, BitRange, DecodeArtifactType, DecodeEvidence, DecodeIndex,
    EvidenceChain, SyntaxEvidence, SyntaxIndex, SyntaxNodeType, VizElementType, VizEvidence,
    VizIndex,
};

#[test]
fn test_bit_offset_evidence() {
    let range = BitRange::new(0, 128);
    let evidence =
        BitOffsetEvidence::new("ev_001".to_string(), range, "OBU_FRAME_HEADER".to_string());

    assert_eq!(evidence.byte_offset, 0);
    assert_eq!(evidence.size_bytes, 16); // 128 bits = 16 bytes
    assert!(evidence.contains_bit(64));
    assert!(!evidence.contains_bit(128));
}

#[test]
fn test_bit_offset_index() {
    let mut index = BitOffsetIndex::new();

    // Add evidence
    let ev1 = BitOffsetEvidence::new(
        "ev_001".to_string(),
        BitRange::new(0, 100),
        "OBU_SEQUENCE_HEADER".to_string(),
    );
    let ev2 = BitOffsetEvidence::new(
        "ev_002".to_string(),
        BitRange::new(100, 300),
        "OBU_FRAME_HEADER".to_string(),
    );

    index.add(ev1);
    index.add(ev2);

    assert_eq!(index.len(), 2);

    // Find by bit offset
    let found = index.find_by_bit_offset(50);
    assert!(found.is_some());
    assert_eq!(found.unwrap().id, "ev_001");

    let found = index.find_by_bit_offset(200);
    assert!(found.is_some());
    assert_eq!(found.unwrap().id, "ev_002");

    // Find by ID
    let found = index.find_by_id("ev_001");
    assert!(found.is_some());
    assert_eq!(found.unwrap().provenance, "OBU_SEQUENCE_HEADER");
}

#[test]
fn test_overlapping_evidence() {
    let mut index = BitOffsetIndex::new();

    index.add(BitOffsetEvidence::new(
        "ev_001".to_string(),
        BitRange::new(0, 100),
        "OBU_A".to_string(),
    ));
    index.add(BitOffsetEvidence::new(
        "ev_002".to_string(),
        BitRange::new(80, 200),
        "OBU_B".to_string(),
    ));

    // Find overlapping with range 90-110
    let range = BitRange::new(90, 110);
    let overlapping = index.find_overlapping(&range);

    assert_eq!(overlapping.len(), 2);
}

// ═══════════════════════════════════════════════════════════════════════
// Stage 02: Syntax Layer Tests
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_syntax_evidence() {
    let mut evidence = SyntaxEvidence::new(
        "syn_001".to_string(),
        SyntaxNodeType::FrameHeader,
        "frame_0_header".to_string(),
        BitRange::new(100, 500),
        "bit_001".to_string(),
    );

    // Test basic properties
    assert_eq!(evidence.node_type, SyntaxNodeType::FrameHeader);
    assert_eq!(evidence.node_label, "frame_0_header");
    assert_eq!(evidence.bit_offset_link, "bit_001");

    // Test metadata
    evidence.add_metadata("frame_type".to_string(), "key_frame".to_string());
    assert_eq!(evidence.metadata.get("frame_type").unwrap(), "key_frame");

    // Test parent/child links
    evidence.set_parent("obu_001".to_string());
    evidence.add_child("tile_001".to_string());
    evidence.add_child("tile_002".to_string());

    assert_eq!(evidence.parent_link, Some("obu_001".to_string()));
    assert_eq!(evidence.child_links.len(), 2);

    // Test decode link
    evidence.link_decode("decode_001".to_string());
    assert_eq!(evidence.decode_link, Some("decode_001".to_string()));

    // Test bit containment
    assert!(evidence.contains_bit(300));
    assert!(!evidence.contains_bit(50));
    assert!(!evidence.contains_bit(500));
}

#[test]
fn test_syntax_index() {
    let mut index = SyntaxIndex::new();

    // Add OBU evidence
    let obu = SyntaxEvidence::new(
        "obu_001".to_string(),
        SyntaxNodeType::Obu,
        "OBU_FRAME".to_string(),
        BitRange::new(0, 1000),
        "bit_obu".to_string(),
    );
    index.add(obu);

    // Add frame header evidence
    let mut frame_header = SyntaxEvidence::new(
        "frame_001".to_string(),
        SyntaxNodeType::FrameHeader,
        "frame_0_header".to_string(),
        BitRange::new(100, 500),
        "bit_frame".to_string(),
    );
    frame_header.set_parent("obu_001".to_string());
    index.add(frame_header);

    // Add tile evidence
    let mut tile = SyntaxEvidence::new(
        "tile_001".to_string(),
        SyntaxNodeType::Tile,
        "tile_0".to_string(),
        BitRange::new(500, 900),
        "bit_tile".to_string(),
    );
    tile.set_parent("obu_001".to_string());
    index.add(tile);

    assert_eq!(index.len(), 3);

    // Test find by ID
    let found = index.find_by_id("frame_001");
    assert!(found.is_some());
    assert_eq!(found.unwrap().node_label, "frame_0_header");

    // Test find by bit offset
    let found = index.find_by_bit_offset(300);
    assert!(found.is_some());
    assert_eq!(found.unwrap().id, "frame_001");

    // Test find by type
    let frame_headers = index.find_by_type(&SyntaxNodeType::FrameHeader);
    assert_eq!(frame_headers.len(), 1);
    assert_eq!(frame_headers[0].id, "frame_001");

    // Test find roots
    let roots = index.find_roots();
    assert_eq!(roots.len(), 1);
    assert_eq!(roots[0].id, "obu_001");

    // Test find children
    let children = index.find_children("obu_001");
    assert_eq!(children.len(), 2);
}

#[test]
fn test_syntax_hierarchy() {
    let mut index = SyntaxIndex::new();

    // Create hierarchy: OBU -> FrameHeader -> Tile -> ModeInfo
    let obu = SyntaxEvidence::new(
        "obu_001".to_string(),
        SyntaxNodeType::Obu,
        "OBU_FRAME".to_string(),
        BitRange::new(0, 10000),
        "bit_obu".to_string(),
    );
    index.add(obu);

    let mut frame_header = SyntaxEvidence::new(
        "frame_001".to_string(),
        SyntaxNodeType::FrameHeader,
        "frame_0_header".to_string(),
        BitRange::new(100, 2000),
        "bit_frame".to_string(),
    );
    frame_header.set_parent("obu_001".to_string());
    index.add(frame_header);

    let mut tile = SyntaxEvidence::new(
        "tile_001".to_string(),
        SyntaxNodeType::Tile,
        "tile_0".to_string(),
        BitRange::new(2000, 8000),
        "bit_tile".to_string(),
    );
    tile.set_parent("obu_001".to_string());
    index.add(tile);

    let mut mode_info = SyntaxEvidence::new(
        "mode_001".to_string(),
        SyntaxNodeType::ModeInfo,
        "mode_0_0".to_string(),
        BitRange::new(2100, 2500),
        "bit_mode".to_string(),
    );
    mode_info.set_parent("tile_001".to_string());
    index.add(mode_info);

    // Verify hierarchy
    assert_eq!(index.len(), 4);

    // Find root
    let roots = index.find_roots();
    assert_eq!(roots.len(), 1);
    assert_eq!(roots[0].node_type, SyntaxNodeType::Obu);

    // Find children of OBU
    let obu_children = index.find_children("obu_001");
    assert_eq!(obu_children.len(), 2);

    // Find children of tile
    let tile_children = index.find_children("tile_001");
    assert_eq!(tile_children.len(), 1);
    assert_eq!(tile_children[0].node_type, SyntaxNodeType::ModeInfo);

    // Find deepest node by bit offset
    let found = index.find_by_bit_offset(2300);
    assert!(found.is_some());
    assert_eq!(found.unwrap().id, "mode_001");
}

// ═══════════════════════════════════════════════════════════════════════
// Stage 03: Decode Layer Tests
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_decode_evidence() {
    let mut evidence = DecodeEvidence::new(
        "decode_001".to_string(),
        DecodeArtifactType::YuvFrame,
        "frame_0_yuv".to_string(),
        "syntax_001".to_string(),
    );

    // Test basic properties
    assert_eq!(evidence.artifact_type, DecodeArtifactType::YuvFrame);
    assert_eq!(evidence.artifact_label, "frame_0_yuv");
    assert_eq!(evidence.syntax_link, "syntax_001");

    // Test frame indices
    evidence.set_frame_indices(10, 15);
    assert_eq!(evidence.frame_idx, Some(10));
    assert_eq!(evidence.display_idx, Some(15));

    // Test data location
    evidence.set_data_location("0x1000".to_string(), 1920 * 1080 * 3 / 2);
    assert_eq!(evidence.data_location, Some("0x1000".to_string()));
    assert_eq!(evidence.data_size, Some(1920 * 1080 * 3 / 2));

    // Test metadata
    evidence.add_metadata("width".to_string(), "1920".to_string());
    evidence.add_metadata("height".to_string(), "1080".to_string());
    assert_eq!(evidence.metadata.get("width").unwrap(), "1920");

    // Test viz link
    evidence.link_viz("viz_001".to_string());
    assert_eq!(evidence.viz_link, Some("viz_001".to_string()));
}

#[test]
fn test_decode_index() {
    let mut index = DecodeIndex::new();

    // Add YUV frame evidence
    let mut yuv_frame = DecodeEvidence::new(
        "yuv_001".to_string(),
        DecodeArtifactType::YuvFrame,
        "frame_0_yuv".to_string(),
        "syntax_frame_001".to_string(),
    );
    yuv_frame.set_frame_indices(0, 0);
    index.add(yuv_frame);

    // Add reference frame evidence
    let mut ref_frame = DecodeEvidence::new(
        "ref_001".to_string(),
        DecodeArtifactType::ReferenceFrame,
        "ref_frame_last".to_string(),
        "syntax_frame_001".to_string(),
    );
    ref_frame.set_frame_indices(0, 0);
    index.add(ref_frame);

    // Add MV field evidence
    let mut mv_field = DecodeEvidence::new(
        "mv_001".to_string(),
        DecodeArtifactType::MotionVectorField,
        "frame_0_mvs".to_string(),
        "syntax_tile_001".to_string(),
    );
    mv_field.set_frame_indices(0, 0);
    index.add(mv_field);

    assert_eq!(index.len(), 3);

    // Test find by ID
    let found = index.find_by_id("yuv_001");
    assert!(found.is_some());
    assert_eq!(found.unwrap().artifact_label, "frame_0_yuv");

    // Test find by artifact type
    let yuv_frames = index.find_by_artifact_type(&DecodeArtifactType::YuvFrame);
    assert_eq!(yuv_frames.len(), 1);
    assert_eq!(yuv_frames[0].id, "yuv_001");

    // Test find by frame index
    let frame_artifacts = index.find_by_frame_idx(0);
    assert_eq!(frame_artifacts.len(), 3);

    // Test find by syntax link
    let frame_linked = index.find_by_syntax_link("syntax_frame_001");
    assert_eq!(frame_linked.len(), 2); // YUV + ref frame
}

#[test]
fn test_decode_evidence_chain() {
    let mut bit_index = BitOffsetIndex::new();
    let mut syntax_index = SyntaxIndex::new();
    let mut decode_index = DecodeIndex::new();

    // Create bit offset evidence
    let bit_ev = BitOffsetEvidence::new(
        "bit_001".to_string(),
        BitRange::new(0, 1000),
        "OBU_FRAME".to_string(),
    );
    bit_index.add(bit_ev);

    // Create syntax evidence linked to bit offset
    let syn_ev = SyntaxEvidence::new(
        "syn_001".to_string(),
        SyntaxNodeType::FrameHeader,
        "frame_0_header".to_string(),
        BitRange::new(0, 500),
        "bit_001".to_string(),
    );
    syntax_index.add(syn_ev);

    // Create decode evidence linked to syntax
    let mut dec_ev = DecodeEvidence::new(
        "dec_001".to_string(),
        DecodeArtifactType::YuvFrame,
        "frame_0_yuv".to_string(),
        "syn_001".to_string(),
    );
    dec_ev.set_frame_indices(0, 0);
    decode_index.add(dec_ev);

    // Verify the chain
    assert_eq!(bit_index.len(), 1);
    assert_eq!(syntax_index.len(), 1);
    assert_eq!(decode_index.len(), 1);

    // Traverse: decode -> syntax -> bit offset
    let decode_ev = decode_index.find_by_id("dec_001").unwrap();
    let syntax_ev = syntax_index.find_by_id(&decode_ev.syntax_link).unwrap();
    let bit_ev = bit_index.find_by_id(&syntax_ev.bit_offset_link).unwrap();

    assert_eq!(bit_ev.provenance, "OBU_FRAME");
    assert_eq!(syntax_ev.node_type, SyntaxNodeType::FrameHeader);
    assert_eq!(decode_ev.artifact_type, DecodeArtifactType::YuvFrame);
}

// ═══════════════════════════════════════════════════════════════════════
// Stage 04: Visualization Layer Tests
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_viz_evidence() {
    let mut evidence = VizEvidence::new(
        "viz_001".to_string(),
        VizElementType::QpHeatmap,
        "qp_heatmap_frame_0".to_string(),
        "decode_001".to_string(),
    );

    // Test basic properties
    assert_eq!(evidence.element_type, VizElementType::QpHeatmap);
    assert_eq!(evidence.element_label, "qp_heatmap_frame_0");
    assert_eq!(evidence.decode_link, "decode_001");

    // Test frame indices
    evidence.set_frame_indices(0, 0);
    assert_eq!(evidence.frame_idx, Some(0));
    assert_eq!(evidence.display_idx, Some(0));

    // Test screen coordinates
    evidence.set_screen_rect(100.0, 50.0, 640.0, 480.0);
    assert_eq!(evidence.screen_rect, Some((100.0, 50.0, 640.0, 480.0)));
    assert!(evidence.contains_screen_point(200.0, 100.0));
    assert!(!evidence.contains_screen_point(50.0, 50.0));

    // Test coded coordinates
    evidence.set_coded_rect(0, 0, 1920, 1080);
    assert_eq!(evidence.coded_rect, Some((0, 0, 1920, 1080)));

    // Test visual properties
    evidence.add_visual_property("color".to_string(), "#FF0000".to_string());
    evidence.add_visual_property("opacity".to_string(), "0.8".to_string());
    assert_eq!(evidence.visual_properties.get("color").unwrap(), "#FF0000");
}

#[test]
fn test_viz_index() {
    let mut index = VizIndex::new();

    // Add QP heatmap viz
    let mut qp_viz = VizEvidence::new(
        "viz_qp_001".to_string(),
        VizElementType::QpHeatmap,
        "qp_heatmap_frame_0".to_string(),
        "decode_yuv_001".to_string(),
    );
    qp_viz.set_frame_indices(0, 0);
    qp_viz.set_screen_rect(0.0, 0.0, 640.0, 480.0);
    index.add(qp_viz);

    // Add MV overlay viz
    let mut mv_viz = VizEvidence::new(
        "viz_mv_001".to_string(),
        VizElementType::MotionVectorOverlay,
        "mv_overlay_frame_0".to_string(),
        "decode_mv_001".to_string(),
    );
    mv_viz.set_frame_indices(0, 0);
    mv_viz.set_screen_rect(0.0, 0.0, 640.0, 480.0);
    index.add(mv_viz);

    // Add timeline lane viz
    let mut timeline_viz = VizEvidence::new(
        "viz_timeline_001".to_string(),
        VizElementType::TimelineLane,
        "timeline_frame_type".to_string(),
        "decode_yuv_001".to_string(),
    );
    timeline_viz.set_frame_indices(0, 0);
    timeline_viz.set_temporal_pos(0.0);
    index.add(timeline_viz);

    assert_eq!(index.len(), 3);

    // Test find by ID
    let found = index.find_by_id("viz_qp_001");
    assert!(found.is_some());
    assert_eq!(found.unwrap().element_label, "qp_heatmap_frame_0");

    // Test find by element type
    let qp_elements = index.find_by_element_type(&VizElementType::QpHeatmap);
    assert_eq!(qp_elements.len(), 1);
    assert_eq!(qp_elements[0].id, "viz_qp_001");

    // Test find by frame index
    let frame_viz = index.find_by_frame_idx(0);
    assert_eq!(frame_viz.len(), 3);

    // Test find by decode link
    let yuv_viz = index.find_by_decode_link("decode_yuv_001");
    assert_eq!(yuv_viz.len(), 2); // QP + timeline

    // Test find at screen point
    let at_point = index.find_at_screen_point(100.0, 100.0);
    assert_eq!(at_point.len(), 2); // QP + MV (timeline has no screen rect)
}

#[test]
fn test_full_evidence_chain() {
    // Create complete evidence chain: bit -> syntax -> decode -> viz
    let mut bit_index = BitOffsetIndex::new();
    let mut syntax_index = SyntaxIndex::new();
    let mut decode_index = DecodeIndex::new();
    let mut viz_index = VizIndex::new();

    // Stage 01: Bit offset
    let bit_ev = BitOffsetEvidence::new(
        "bit_001".to_string(),
        BitRange::new(0, 10000),
        "OBU_FRAME".to_string(),
    );
    bit_index.add(bit_ev);

    // Stage 02: Syntax
    let syn_ev = SyntaxEvidence::new(
        "syn_001".to_string(),
        SyntaxNodeType::FrameHeader,
        "frame_0_header".to_string(),
        BitRange::new(0, 1000),
        "bit_001".to_string(),
    );
    syntax_index.add(syn_ev);

    // Stage 03: Decode
    let mut dec_ev = DecodeEvidence::new(
        "dec_001".to_string(),
        DecodeArtifactType::YuvFrame,
        "frame_0_yuv".to_string(),
        "syn_001".to_string(),
    );
    dec_ev.set_frame_indices(0, 0);
    decode_index.add(dec_ev);

    // Stage 04: Visualization
    let mut viz_ev = VizEvidence::new(
        "viz_001".to_string(),
        VizElementType::QpHeatmap,
        "qp_heatmap_frame_0".to_string(),
        "dec_001".to_string(),
    );
    viz_ev.set_frame_indices(0, 0);
    viz_ev.set_screen_rect(0.0, 0.0, 640.0, 480.0);
    viz_ev.set_coded_rect(0, 0, 1920, 1080);
    viz_index.add(viz_ev);

    // Verify chain lengths
    assert_eq!(bit_index.len(), 1);
    assert_eq!(syntax_index.len(), 1);
    assert_eq!(decode_index.len(), 1);
    assert_eq!(viz_index.len(), 1);

    // Traverse forward: bit -> syntax -> decode -> viz
    let bit_ev = bit_index.find_by_id("bit_001").unwrap();
    let syntax_ev = syntax_index
        .find_by_bit_offset(bit_ev.bit_range.start_bit)
        .unwrap();
    let decode_evidences = decode_index.find_by_syntax_link(&syntax_ev.id);
    assert_eq!(decode_evidences.len(), 1);
    let decode_ev = decode_evidences[0];
    let viz_evidences = viz_index.find_by_decode_link(&decode_ev.id);
    assert_eq!(viz_evidences.len(), 1);
    let viz_ev = viz_evidences[0];

    // Verify chain integrity
    assert_eq!(bit_ev.provenance, "OBU_FRAME");
    assert_eq!(syntax_ev.node_type, SyntaxNodeType::FrameHeader);
    assert_eq!(decode_ev.artifact_type, DecodeArtifactType::YuvFrame);
    assert_eq!(viz_ev.element_type, VizElementType::QpHeatmap);

    // Traverse backward: viz -> decode -> syntax -> bit
    let viz_ev = viz_index.find_by_id("viz_001").unwrap();
    let decode_ev = decode_index.find_by_id(&viz_ev.decode_link).unwrap();
    let syntax_ev = syntax_index.find_by_id(&decode_ev.syntax_link).unwrap();
    let bit_ev = bit_index.find_by_id(&syntax_ev.bit_offset_link).unwrap();

    assert_eq!(bit_ev.id, "bit_001");
    assert_eq!(syntax_ev.id, "syn_001");
    assert_eq!(decode_ev.id, "dec_001");
}

// ═══════════════════════════════════════════════════════════════════════
// Stage 05: Bidirectional Traversal Tests
// ═══════════════════════════════════════════════════════════════════════

fn create_test_evidence_chain() -> EvidenceChain {
    let mut chain = EvidenceChain::new();

    // Create complete evidence chain for frame 0
    let bit_ev = BitOffsetEvidence::new(
        "bit_001".to_string(),
        BitRange::new(0, 10000),
        "OBU_FRAME".to_string(),
    );
    chain.add_bit_offset(bit_ev);

    let syn_ev = SyntaxEvidence::new(
        "syn_001".to_string(),
        SyntaxNodeType::FrameHeader,
        "frame_0_header".to_string(),
        BitRange::new(0, 1000),
        "bit_001".to_string(),
    );
    chain.add_syntax(syn_ev);

    let mut dec_ev = DecodeEvidence::new(
        "dec_001".to_string(),
        DecodeArtifactType::YuvFrame,
        "frame_0_yuv".to_string(),
        "syn_001".to_string(),
    );
    dec_ev.set_frame_indices(0, 0);
    chain.add_decode(dec_ev);

    let mut viz_ev = VizEvidence::new(
        "viz_001".to_string(),
        VizElementType::QpHeatmap,
        "qp_heatmap_frame_0".to_string(),
        "dec_001".to_string(),
    );
    viz_ev.set_frame_indices(0, 0);
    viz_ev.set_screen_rect(0.0, 0.0, 640.0, 480.0);
    chain.add_viz(viz_ev);

    chain
}

#[test]
fn test_evidence_chain_forward_traversal() {
    let chain = create_test_evidence_chain();

    // bit → syntax
    let syntax_ev = chain.bit_to_syntax(500);
    assert!(syntax_ev.is_some());
    assert_eq!(syntax_ev.unwrap().id, "syn_001");

    // syntax → decode
    let decode_evs = chain.syntax_to_decode("syn_001");
    assert_eq!(decode_evs.len(), 1);
    assert_eq!(decode_evs[0].id, "dec_001");

    // decode → viz
    let viz_evs = chain.decode_to_viz("dec_001");
    assert_eq!(viz_evs.len(), 1);
    assert_eq!(viz_evs[0].id, "viz_001");

    // bit → viz (full forward)
    let viz_evs = chain.bit_to_viz(500);
    assert_eq!(viz_evs.len(), 1);
    assert_eq!(viz_evs[0].element_type, VizElementType::QpHeatmap);
}

#[test]
fn test_evidence_chain_backward_traversal() {
    let chain = create_test_evidence_chain();

    // viz → decode
    let decode_ev = chain.viz_to_decode("viz_001");
    assert!(decode_ev.is_some());
    assert_eq!(decode_ev.unwrap().id, "dec_001");

    // decode → syntax
    let syntax_ev = chain.decode_to_syntax("dec_001");
    assert!(syntax_ev.is_some());
    assert_eq!(syntax_ev.unwrap().id, "syn_001");

    // syntax → bit
    let bit_ev = chain.syntax_to_bit("syn_001");
    assert!(bit_ev.is_some());
    assert_eq!(bit_ev.unwrap().id, "bit_001");

    // viz → bit (full backward)
    let bit_ev = chain.viz_to_bit("viz_001");
    assert!(bit_ev.is_some());
    assert_eq!(bit_ev.unwrap().provenance, "OBU_FRAME");
}

#[test]
fn test_evidence_chain_spatial_queries() {
    let chain = create_test_evidence_chain();

    // Find viz at screen point
    let viz_evs = chain.at_screen_point(100.0, 100.0);
    assert_eq!(viz_evs.len(), 1);
    assert_eq!(viz_evs[0].id, "viz_001");

    // Find bit range from screen point
    let bit_range = chain.screen_point_to_bit_range(100.0, 100.0);
    assert!(bit_range.is_some());
    let range = bit_range.unwrap();
    assert_eq!(range.start_bit, 0);
    assert_eq!(range.end_bit, 10000);

    // Point outside viz should return nothing
    let viz_evs = chain.at_screen_point(1000.0, 1000.0);
    assert_eq!(viz_evs.len(), 0);
}

#[test]
fn test_evidence_chain_frame_queries() {
    let chain = create_test_evidence_chain();

    // Find frame evidence
    let frame_ev = chain.find_frame_evidence(0);
    assert_eq!(frame_ev.decode_artifacts.len(), 1);
    assert_eq!(frame_ev.viz_elements.len(), 1);
    assert_eq!(frame_ev.decode_artifacts[0].id, "dec_001");
    assert_eq!(frame_ev.viz_elements[0].id, "viz_001");

    // Find display evidence
    let display_ev = chain.find_display_evidence(0);
    assert_eq!(display_ev.decode_artifacts.len(), 1);
    assert_eq!(display_ev.viz_elements.len(), 1);

    // Non-existent frame
    let frame_ev = chain.find_frame_evidence(999);
    assert_eq!(frame_ev.decode_artifacts.len(), 0);
    assert_eq!(frame_ev.viz_elements.len(), 0);
}

#[test]
fn test_evidence_chain_roundtrip() {
    let chain = create_test_evidence_chain();

    // Forward: bit → viz
    let viz_evs = chain.bit_to_viz(500);
    assert_eq!(viz_evs.len(), 1);
    let viz_id = &viz_evs[0].id;

    // Backward: viz → bit
    let bit_ev = chain.viz_to_bit(viz_id);
    assert!(bit_ev.is_some());
    assert_eq!(bit_ev.unwrap().id, "bit_001");

    // Verify round-trip integrity
    assert!(bit_ev.unwrap().contains_bit(500));
}
