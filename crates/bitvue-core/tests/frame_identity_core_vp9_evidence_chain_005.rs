#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(unused_comparisons)]
#![allow(unused_doc_comments)]
use bitvue_core::evidence::*;
use bitvue_core::frame_identity::*;
use bitvue_core::BitRange;

// ========================================================================
// S.T0-1.VP9.FrameIdentity.Core.impl.evidence_chain.005
// Evidence Chain stage 05/05 (roundtrip_links) — VP9 Core Frame Identity
// ========================================================================
//
// DELIVERABLE: evidence_chain_05_roundtrip_links:FrameIdentity:Core:VP9:evidence_chain
//
// This test suite validates the bidirectional evidence chain traversal
// for VP9-specific features in the Core data structures:
// - bit_offset ↔ syntax ↔ decode ↔ viz roundtrip
// - VP9 ALTREF frame evidence chain
// - VP9 show_existing_frame evidence chain
// - VP9 segmentation evidence chain
//
// Key VP9 Evidence Chain Requirements:
// - All layers must be bidirectionally traversable
// - Evidence links must be deterministic and lossless
// - VP9-specific quirks preserved through all layers

/// Helper to create VizEvidence with a custom primitive type
fn make_viz(id: &str, decode_link: &str, primitive_type: &str) -> VizEvidence {
    VizEvidence::new(
        id.to_string(),
        VizElementType::Custom(primitive_type.to_string()),
        primitive_type.to_lowercase(),
        decode_link.to_string(),
    )
}

/// Helper to create DecodeEvidence with custom artifact type
fn make_decode(id: &str, syntax_link: &str, label: &str) -> DecodeEvidence {
    DecodeEvidence::new(
        id.to_string(),
        DecodeArtifactType::Custom(label.to_string()),
        label.to_string(),
        syntax_link.to_string(),
    )
}

#[test]
fn test_vp9_evidence_005_full_chain_roundtrip() {
    // Test: Complete evidence chain roundtrip for VP9 frame
    // Scenario: bit_offset → syntax → decode → viz → back to bit_offset
    // Per EVIDENCE_CHAIN_CONTRACT: All layers bidirectionally linked

    let frames = vec![FrameMetadata {
        pts: Some(0),
        dts: Some(0),
    }];
    let map = FrameIndexMap::new(&frames);
    assert_eq!(map.frame_count(), 1);

    let mut bit_index = BitOffsetIndex::new();
    let mut syntax_index = SyntaxIndex::new();
    let mut decode_index = DecodeIndex::new();
    let mut viz_index = VizIndex::new();

    // Stage 01: Bit offset evidence
    let mut bit_ev = BitOffsetEvidence::new(
        "vp9_bit_frame_0".to_string(),
        BitRange::new(0, 16000),
        "VP9_FRAME|display_idx_0".to_string(),
    );
    bit_ev.link_syntax("vp9_syn_frame_0".to_string());
    bit_index.add(bit_ev);

    // Stage 02: Syntax evidence
    let mut syn_ev = SyntaxEvidence::new(
        "vp9_syn_frame_0".to_string(),
        SyntaxNodeType::FrameHeader,
        "frame_0".to_string(),
        BitRange::new(0, 16000),
        "vp9_bit_frame_0".to_string(),
    );
    syn_ev.link_decode("vp9_dec_frame_0".to_string());
    syn_ev.add_metadata("frame_type".to_string(), "INTER_FRAME".to_string());
    syntax_index.add(syn_ev);

    // Stage 03: Decode evidence
    let mut dec_ev = make_decode("vp9_dec_frame_0", "vp9_syn_frame_0", "VP9_INTER_DECODE");
    dec_ev.link_viz("vp9_viz_frame_0".to_string());
    dec_ev.add_metadata("ref_frame".to_string(), "LAST".to_string());
    decode_index.add(dec_ev);

    // Stage 04: Viz evidence
    let mut viz_ev = make_viz("vp9_viz_frame_0", "vp9_dec_frame_0", "Frame");
    viz_ev.add_visual_property("color".to_string(), "#FF0000".to_string());
    viz_index.add(viz_ev);

    // Roundtrip: viz → decode → syntax → bit_offset
    let viz_found = viz_index.find_by_id("vp9_viz_frame_0");
    assert!(viz_found.is_some());
    let viz = viz_found.unwrap();
    assert_eq!(viz.decode_link, "vp9_dec_frame_0");

    let dec_found = decode_index.find_by_id(&viz.decode_link);
    assert!(dec_found.is_some());
    let dec = dec_found.unwrap();
    assert_eq!(dec.syntax_link, "vp9_syn_frame_0");

    let syn_found = syntax_index.find_by_id(&dec.syntax_link);
    assert!(syn_found.is_some());
    let syn = syn_found.unwrap();
    assert_eq!(syn.bit_offset_link, "vp9_bit_frame_0");

    let bit_found = bit_index.find_by_id(&syn.bit_offset_link);
    assert!(bit_found.is_some());
    let bit = bit_found.unwrap();
    assert_eq!(bit.id, "vp9_bit_frame_0");

    // Reverse roundtrip: bit_offset → syntax → decode → viz
    assert_eq!(bit.syntax_link, Some("vp9_syn_frame_0".to_string()));
    assert_eq!(syn.decode_link, Some("vp9_dec_frame_0".to_string()));
    assert_eq!(dec.viz_link, Some("vp9_viz_frame_0".to_string()));
}

#[test]
fn test_vp9_evidence_005_altref_chain_roundtrip() {
    // Test: Evidence chain for VP9 ALTREF frame (invisible)
    // Scenario: Full chain for ALTREF with show_frame=0
    // Per VP9 spec: ALTREF decoded but not displayed

    let frames = vec![
        FrameMetadata {
            pts: Some(0),
            dts: Some(0),
        }, // K0
        FrameMetadata {
            pts: Some(2000),
            dts: Some(2000),
        }, // P2 (ALTREF invisible)
    ];
    let map = FrameIndexMap::new(&frames);
    assert_eq!(map.frame_count(), 2);

    let mut bit_index = BitOffsetIndex::new();
    let mut syntax_index = SyntaxIndex::new();
    let mut decode_index = DecodeIndex::new();
    let mut viz_index = VizIndex::new();

    // ALTREF bit offset evidence
    let mut bit_altref = BitOffsetEvidence::new(
        "vp9_bit_altref".to_string(),
        BitRange::new(10000, 18000),
        "VP9_ALTREF|show_frame_0".to_string(),
    );
    bit_altref.link_syntax("vp9_syn_altref".to_string());
    bit_index.add(bit_altref);

    // ALTREF syntax evidence
    let mut syn_altref = SyntaxEvidence::new(
        "vp9_syn_altref".to_string(),
        SyntaxNodeType::FrameHeader,
        "altref_frame".to_string(),
        BitRange::new(10000, 18000),
        "vp9_bit_altref".to_string(),
    );
    syn_altref.link_decode("vp9_dec_altref".to_string());
    syn_altref.add_metadata("frame_type".to_string(), "INTER_FRAME".to_string());
    syn_altref.add_metadata("show_frame".to_string(), "0".to_string());
    syn_altref.add_metadata("is_altref".to_string(), "true".to_string());
    syntax_index.add(syn_altref);

    // ALTREF decode evidence
    let mut dec_altref = make_decode("vp9_dec_altref", "vp9_syn_altref", "VP9_ALTREF_DECODE");
    dec_altref.link_viz("vp9_viz_altref".to_string());
    dec_altref.add_metadata("show_frame".to_string(), "0".to_string());
    dec_altref.add_metadata("refresh_frame_flags".to_string(), "0x04".to_string());
    dec_altref.add_metadata("display_idx".to_string(), "none".to_string());
    decode_index.add(dec_altref);

    // ALTREF viz evidence (marker for invisible frame)
    let mut viz_altref = make_viz("vp9_viz_altref", "vp9_dec_altref", "Marker");
    viz_altref.add_visual_property("invisible".to_string(), "true".to_string());
    viz_altref.add_visual_property("marker_shape".to_string(), "diamond".to_string());
    viz_index.add(viz_altref);

    // Verify complete chain
    let viz = viz_index.find_by_id("vp9_viz_altref").unwrap();
    let dec = decode_index.find_by_id(&viz.decode_link).unwrap();
    let syn = syntax_index.find_by_id(&dec.syntax_link).unwrap();
    let bit = bit_index.find_by_id(&syn.bit_offset_link).unwrap();

    // Verify ALTREF-specific evidence at each layer
    assert!(bit.provenance.contains("ALTREF"));
    assert!(bit.provenance.contains("show_frame_0"));
    assert_eq!(syn.metadata.get("is_altref"), Some(&"true".to_string()));
    assert_eq!(dec.metadata.get("display_idx"), Some(&"none".to_string()));
    assert_eq!(
        viz.visual_properties.get("invisible"),
        Some(&"true".to_string())
    );

    // Reverse traversal: bit → syntax → decode → viz
    assert_eq!(bit.syntax_link, Some("vp9_syn_altref".to_string()));
    assert_eq!(syn.decode_link, Some("vp9_dec_altref".to_string()));
    assert_eq!(dec.viz_link, Some("vp9_viz_altref".to_string()));
}

#[test]
fn test_vp9_evidence_005_show_existing_chain_roundtrip() {
    // Test: Evidence chain for VP9 show_existing_frame
    // Scenario: Virtual frame that references previous decoded frame
    // Per VP9 spec: Minimal frame with frame_to_show_map_idx

    let frames = vec![
        FrameMetadata {
            pts: Some(0),
            dts: Some(0),
        },
        FrameMetadata {
            pts: Some(1000),
            dts: Some(1000),
        },
        FrameMetadata {
            pts: Some(2000),
            dts: Some(2000),
        }, // show_existing
    ];
    let map = FrameIndexMap::new(&frames);
    assert_eq!(map.frame_count(), 3);

    let mut bit_index = BitOffsetIndex::new();
    let mut syntax_index = SyntaxIndex::new();
    let mut decode_index = DecodeIndex::new();
    let mut viz_index = VizIndex::new();

    // show_existing bit offset evidence (minimal size)
    let mut bit_show = BitOffsetEvidence::new(
        "vp9_bit_show".to_string(),
        BitRange::new(20000, 20100),
        "VP9_SHOW_EXISTING|display_idx_2".to_string(),
    );
    bit_show.link_syntax("vp9_syn_show".to_string());
    bit_index.add(bit_show);

    // show_existing syntax evidence
    let mut syn_show = SyntaxEvidence::new(
        "vp9_syn_show".to_string(),
        SyntaxNodeType::FrameHeader,
        "show_existing".to_string(),
        BitRange::new(20000, 20100),
        "vp9_bit_show".to_string(),
    );
    syn_show.link_decode("vp9_dec_show".to_string());
    syn_show.add_metadata("show_existing_frame".to_string(), "1".to_string());
    syn_show.add_metadata("frame_to_show_map_idx".to_string(), "1".to_string());
    syntax_index.add(syn_show);

    // show_existing decode evidence
    let mut dec_show = make_decode("vp9_dec_show", "vp9_syn_show", "VP9_SHOW_EXISTING_DECODE");
    dec_show.link_viz("vp9_viz_show".to_string());
    dec_show.add_metadata("virtual".to_string(), "true".to_string());
    dec_show.add_metadata("references_buf".to_string(), "1".to_string());
    dec_show.add_metadata("no_new_decode".to_string(), "true".to_string());
    decode_index.add(dec_show);

    // show_existing viz evidence
    let mut viz_show = make_viz("vp9_viz_show", "vp9_dec_show", "VirtualFrame");
    viz_show.add_visual_property("virtual".to_string(), "true".to_string());
    viz_show.add_visual_property("border_style".to_string(), "dashed".to_string());
    viz_index.add(viz_show);

    // Verify complete chain
    let viz = viz_index.find_by_id("vp9_viz_show").unwrap();
    let dec = decode_index.find_by_id(&viz.decode_link).unwrap();
    let syn = syntax_index.find_by_id(&dec.syntax_link).unwrap();
    let bit = bit_index.find_by_id(&syn.bit_offset_link).unwrap();

    // Verify show_existing-specific evidence
    assert!(bit.provenance.contains("SHOW_EXISTING"));
    assert_eq!(
        syn.metadata.get("show_existing_frame"),
        Some(&"1".to_string())
    );
    assert_eq!(
        syn.metadata.get("frame_to_show_map_idx"),
        Some(&"1".to_string())
    );
    assert_eq!(dec.metadata.get("virtual"), Some(&"true".to_string()));
    assert_eq!(dec.metadata.get("no_new_decode"), Some(&"true".to_string()));
    assert_eq!(
        viz.visual_properties.get("virtual"),
        Some(&"true".to_string())
    );

    // Verify minimal bit range size
    assert_eq!(bit.bit_range.size_bits(), 100);
    assert_eq!(bit.size_bytes, (100 + 7) / 8);
}

#[test]
fn test_vp9_evidence_005_segmentation_chain_roundtrip() {
    // Test: Evidence chain for VP9 segmentation
    // Scenario: Full chain for segmentation params and segment map
    // Per VP9 spec: Up to 8 segments with independent params

    let frames = vec![FrameMetadata {
        pts: Some(0),
        dts: Some(0),
    }];
    let map = FrameIndexMap::new(&frames);
    assert_eq!(map.frame_count(), 1);

    let mut bit_index = BitOffsetIndex::new();
    let mut syntax_index = SyntaxIndex::new();
    let mut decode_index = DecodeIndex::new();
    let mut viz_index = VizIndex::new();

    // Segmentation params bit offset
    let mut bit_seg = BitOffsetEvidence::new(
        "vp9_bit_seg".to_string(),
        BitRange::new(400, 1200),
        "VP9_SEGMENTATION|num_segments_4".to_string(),
    );
    bit_seg.link_syntax("vp9_syn_seg".to_string());
    bit_index.add(bit_seg);

    // Segmentation params syntax
    let mut syn_seg = SyntaxEvidence::new(
        "vp9_syn_seg".to_string(),
        SyntaxNodeType::SegmentationParams,
        "segmentation_params".to_string(),
        BitRange::new(400, 1200),
        "vp9_bit_seg".to_string(),
    );
    syn_seg.link_decode("vp9_dec_seg".to_string());
    syn_seg.add_metadata("segmentation_enabled".to_string(), "1".to_string());
    syn_seg.add_metadata("num_segments".to_string(), "4".to_string());
    syn_seg.add_metadata("update_map".to_string(), "1".to_string());
    syntax_index.add(syn_seg);

    // Segmentation decode state
    let mut dec_seg = make_decode("vp9_dec_seg", "vp9_syn_seg", "VP9_SEGMENTATION_DECODE");
    dec_seg.link_viz("vp9_viz_seg".to_string());
    dec_seg.add_metadata("seg_map_established".to_string(), "true".to_string());
    dec_seg.add_metadata("num_segments".to_string(), "4".to_string());
    decode_index.add(dec_seg);

    // Segmentation viz overlay
    let mut viz_seg = make_viz("vp9_viz_seg", "vp9_dec_seg", "Overlay");
    viz_seg.add_visual_property("type".to_string(), "segmentation_map".to_string());
    viz_seg.add_visual_property("num_segments".to_string(), "4".to_string());
    viz_index.add(viz_seg);

    // Segment 0 bit offset (nested)
    let mut bit_seg0 = BitOffsetEvidence::new(
        "vp9_bit_seg0".to_string(),
        BitRange::new(500, 700),
        "VP9_SEGMENT_0|qp_offset".to_string(),
    );
    bit_seg0.link_syntax("vp9_syn_seg0".to_string());
    bit_index.add(bit_seg0);

    // Segment 0 syntax (nested)
    let mut syn_seg0 = SyntaxEvidence::new(
        "vp9_syn_seg0".to_string(),
        SyntaxNodeType::Custom("segment_params".to_string()),
        "segment_0".to_string(),
        BitRange::new(500, 700),
        "vp9_bit_seg0".to_string(),
    );
    syn_seg0.set_parent("vp9_syn_seg".to_string());
    syn_seg0.link_decode("vp9_dec_seg0".to_string());
    syn_seg0.add_metadata("segment_id".to_string(), "0".to_string());
    syn_seg0.add_metadata("qp_offset".to_string(), "-10".to_string());
    syntax_index.add(syn_seg0);

    // Segment 0 decode state
    let mut dec_seg0 = make_decode("vp9_dec_seg0", "vp9_syn_seg0", "VP9_SEGMENT_0_DECODE");
    dec_seg0.link_viz("vp9_viz_seg0".to_string());
    dec_seg0.add_metadata("segment_id".to_string(), "0".to_string());
    dec_seg0.add_metadata("qp_offset".to_string(), "-10".to_string());
    decode_index.add(dec_seg0);

    // Segment 0 viz rectangle
    let mut viz_seg0 = make_viz("vp9_viz_seg0", "vp9_dec_seg0", "Rectangle");
    viz_seg0.set_screen_rect(0.0, 0.0, 960.0, 540.0);
    viz_seg0.add_visual_property("segment_id".to_string(), "0".to_string());
    viz_seg0.add_visual_property("color".to_string(), "#00FF00".to_string());
    viz_index.add(viz_seg0);

    // Verify parent segmentation chain
    let viz_parent = viz_index.find_by_id("vp9_viz_seg").unwrap();
    let dec_parent = decode_index.find_by_id(&viz_parent.decode_link).unwrap();
    let syn_parent = syntax_index.find_by_id(&dec_parent.syntax_link).unwrap();
    let bit_parent = bit_index.find_by_id(&syn_parent.bit_offset_link).unwrap();

    assert_eq!(bit_parent.id, "vp9_bit_seg");
    assert_eq!(
        syn_parent.metadata.get("num_segments"),
        Some(&"4".to_string())
    );

    // Verify segment 0 chain
    let viz_seg0 = viz_index.find_by_id("vp9_viz_seg0").unwrap();
    let dec_seg0 = decode_index.find_by_id(&viz_seg0.decode_link).unwrap();
    let syn_seg0 = syntax_index.find_by_id(&dec_seg0.syntax_link).unwrap();
    let bit_seg0 = bit_index.find_by_id(&syn_seg0.bit_offset_link).unwrap();

    // Verify nested structure
    assert_eq!(syn_seg0.parent_link, Some("vp9_syn_seg".to_string()));
    assert!(bit_parent.bit_range.contains_range(&bit_seg0.bit_range));

    // Verify evidence propagation through chain
    assert_eq!(syn_seg0.metadata.get("qp_offset"), Some(&"-10".to_string()));
    assert_eq!(dec_seg0.metadata.get("qp_offset"), Some(&"-10".to_string()));
    assert_eq!(
        viz_seg0.visual_properties.get("segment_id"),
        Some(&"0".to_string())
    );
}

#[test]
fn test_vp9_evidence_005_reference_frame_chain() {
    // Test: Evidence chain for VP9 reference frame usage
    // Scenario: Block references LAST/GOLDEN/ALTREF through full chain
    // Per VP9 spec: 8 reference buffers, 3 named slots

    let frames = vec![FrameMetadata {
        pts: Some(0),
        dts: Some(0),
    }];
    let map = FrameIndexMap::new(&frames);
    assert_eq!(map.frame_count(), 1);

    let mut bit_index = BitOffsetIndex::new();
    let mut syntax_index = SyntaxIndex::new();
    let mut decode_index = DecodeIndex::new();
    let mut viz_index = VizIndex::new();

    // Block bit offset
    let mut bit_block = BitOffsetEvidence::new(
        "vp9_bit_block".to_string(),
        BitRange::new(5000, 5500),
        "VP9_BLOCK|refs_golden".to_string(),
    );
    bit_block.link_syntax("vp9_syn_block".to_string());
    bit_index.add(bit_block);

    // Block syntax
    let mut syn_block = SyntaxEvidence::new(
        "vp9_syn_block".to_string(),
        SyntaxNodeType::ModeInfo,
        "block_mode".to_string(),
        BitRange::new(5000, 5500),
        "vp9_bit_block".to_string(),
    );
    syn_block.link_decode("vp9_dec_block".to_string());
    syn_block.add_metadata("ref_frame".to_string(), "GOLDEN".to_string());
    syn_block.add_metadata("mv_x".to_string(), "12".to_string());
    syn_block.add_metadata("mv_y".to_string(), "8".to_string());
    syntax_index.add(syn_block);

    // Block decode state
    let mut dec_block = make_decode("vp9_dec_block", "vp9_syn_block", "VP9_BLOCK_DECODE");
    dec_block.link_viz("vp9_viz_block".to_string());
    dec_block.add_metadata("ref_frame".to_string(), "GOLDEN".to_string());
    dec_block.add_metadata("ref_buf_1_read".to_string(), "true".to_string());
    decode_index.add(dec_block);

    // Block viz
    let mut viz_block = make_viz("vp9_viz_block", "vp9_dec_block", "Rectangle");
    viz_block.set_screen_rect(64.0, 64.0, 64.0, 64.0);
    viz_block.add_visual_property("ref_frame".to_string(), "GOLDEN".to_string());
    viz_block.add_visual_property("color".to_string(), "#FFD700".to_string());
    viz_index.add(viz_block);

    // Full chain verification
    let viz = viz_index.find_by_id("vp9_viz_block").unwrap();
    let dec = decode_index.find_by_id(&viz.decode_link).unwrap();
    let syn = syntax_index.find_by_id(&dec.syntax_link).unwrap();
    let bit = bit_index.find_by_id(&syn.bit_offset_link).unwrap();

    // Verify reference frame evidence at each layer
    assert!(bit.provenance.contains("refs_golden"));
    assert_eq!(syn.metadata.get("ref_frame"), Some(&"GOLDEN".to_string()));
    assert_eq!(dec.metadata.get("ref_frame"), Some(&"GOLDEN".to_string()));
    assert_eq!(
        viz.visual_properties.get("ref_frame"),
        Some(&"GOLDEN".to_string())
    );

    // Verify reverse links
    assert_eq!(bit.syntax_link, Some("vp9_syn_block".to_string()));
    assert_eq!(syn.decode_link, Some("vp9_dec_block".to_string()));
    assert_eq!(dec.viz_link, Some("vp9_viz_block".to_string()));
}

#[test]
fn test_vp9_evidence_005_compound_prediction_chain() {
    // Test: Evidence chain for VP9 compound prediction
    // Scenario: Block using 2 reference frames (LAST + GOLDEN)
    // Per VP9 spec: Compound prediction blends 2 MVs

    let frames = vec![FrameMetadata {
        pts: Some(0),
        dts: Some(0),
    }];
    let map = FrameIndexMap::new(&frames);
    assert_eq!(map.frame_count(), 1);

    let mut bit_index = BitOffsetIndex::new();
    let mut syntax_index = SyntaxIndex::new();
    let mut decode_index = DecodeIndex::new();
    let mut viz_index = VizIndex::new();

    // Compound block bit offset
    let mut bit_compound = BitOffsetEvidence::new(
        "vp9_bit_compound".to_string(),
        BitRange::new(6000, 6800),
        "VP9_BLOCK|compound|last_golden".to_string(),
    );
    bit_compound.link_syntax("vp9_syn_compound".to_string());
    bit_index.add(bit_compound);

    // Compound block syntax
    let mut syn_compound = SyntaxEvidence::new(
        "vp9_syn_compound".to_string(),
        SyntaxNodeType::ModeInfo,
        "compound_block".to_string(),
        BitRange::new(6000, 6800),
        "vp9_bit_compound".to_string(),
    );
    syn_compound.link_decode("vp9_dec_compound".to_string());
    syn_compound.add_metadata("compound_prediction".to_string(), "true".to_string());
    syn_compound.add_metadata("ref_frame_0".to_string(), "LAST".to_string());
    syn_compound.add_metadata("ref_frame_1".to_string(), "GOLDEN".to_string());
    syntax_index.add(syn_compound);

    // Compound block decode
    let mut dec_compound = make_decode(
        "vp9_dec_compound",
        "vp9_syn_compound",
        "VP9_COMPOUND_DECODE",
    );
    dec_compound.link_viz("vp9_viz_compound_mv0".to_string());
    dec_compound.add_metadata("compound_prediction".to_string(), "true".to_string());
    dec_compound.add_metadata("ref_buf_0_read".to_string(), "true".to_string());
    dec_compound.add_metadata("ref_buf_1_read".to_string(), "true".to_string());
    decode_index.add(dec_compound);

    // Compound viz (MV 0 - LAST)
    let mut viz_mv0 = make_viz("vp9_viz_compound_mv0", "vp9_dec_compound", "Arrow");
    viz_mv0.add_visual_property("compound_idx".to_string(), "0".to_string());
    viz_mv0.add_visual_property("ref_frame".to_string(), "LAST".to_string());
    viz_mv0.add_visual_property("color".to_string(), "#FF0000".to_string());
    viz_index.add(viz_mv0);

    // Compound viz (MV 1 - GOLDEN)
    let mut viz_mv1 = make_viz("vp9_viz_compound_mv1", "vp9_dec_compound", "Arrow");
    viz_mv1.add_visual_property("compound_idx".to_string(), "1".to_string());
    viz_mv1.add_visual_property("ref_frame".to_string(), "GOLDEN".to_string());
    viz_mv1.add_visual_property("color".to_string(), "#FFD700".to_string());
    viz_index.add(viz_mv1);

    // Verify chain for MV0
    let viz0 = viz_index.find_by_id("vp9_viz_compound_mv0").unwrap();
    let dec = decode_index.find_by_id(&viz0.decode_link).unwrap();
    let syn = syntax_index.find_by_id(&dec.syntax_link).unwrap();
    let bit = bit_index.find_by_id(&syn.bit_offset_link).unwrap();

    assert!(bit.provenance.contains("compound"));
    assert_eq!(
        syn.metadata.get("compound_prediction"),
        Some(&"true".to_string())
    );
    assert_eq!(
        dec.metadata.get("compound_prediction"),
        Some(&"true".to_string())
    );

    // Verify both viz elements reference same decode state
    let viz1 = viz_index.find_by_id("vp9_viz_compound_mv1").unwrap();
    assert_eq!(viz0.decode_link, viz1.decode_link);
}

#[test]
fn test_vp9_evidence_005_empty_stream() {
    // Test: Empty stream has no evidence chain
    let frames: Vec<FrameMetadata> = vec![];
    let map = FrameIndexMap::new(&frames);

    let bit_index = BitOffsetIndex::new();
    let syntax_index = SyntaxIndex::new();
    let decode_index = DecodeIndex::new();
    let viz_index = VizIndex::new();

    assert_eq!(map.frame_count(), 0);
    assert!(bit_index.is_empty());
    assert!(syntax_index.is_empty());
    assert!(decode_index.is_empty());
    assert!(viz_index.is_empty());
}
