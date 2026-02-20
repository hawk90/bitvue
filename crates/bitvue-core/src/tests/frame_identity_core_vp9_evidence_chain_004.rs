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
use crate::evidence::*;
use crate::frame_identity::*;

// ========================================================================
// S.T0-1.VP9.FrameIdentity.Core.impl.evidence_chain.004
// Evidence Chain stage 04/05 (viz_primitive) — VP9 Core Frame Identity
// ========================================================================
//
// DELIVERABLE: evidence_chain_04_viz_primitive:FrameIdentity:Core:VP9:evidence_chain
//
// This test suite validates the integration of VizEvidence with
// VP9-specific visualization primitives in the Core data structures:
// - Segmentation map visualization
// - Reference frame usage visualization
// - Loop filter strength visualization
// - Motion vector visualization for VP9
//
// Key VP9 Viz Quirks:
// - Segmentation: Color-coded segments (up to 8 colors)
// - Reference frames: LAST/GOLDEN/ALTREF color coding
// - Loop filter: Heatmap of filter strength
// - Motion vectors: Compound prediction shows 2 MVs per block

/// Helper to create VizEvidence with a custom primitive type
fn make_viz(id: &str, decode_link: &str, primitive_type: &str) -> VizEvidence {
    VizEvidence::new(
        id.to_string(),
        VizElementType::Custom(primitive_type.to_string()),
        primitive_type.to_lowercase(),
        decode_link.to_string(),
    )
}

#[test]
fn test_vp9_evidence_004_segmentation_map_viz() {
    // Test: VP9 segmentation map visualization
    // Scenario: Frame with 4 segments, each with different visual properties
    // Per VP9 spec: Up to 8 segments with independent QP/LF

    let frames = vec![FrameMetadata {
        pts: Some(0),
        dts: Some(0),
    }];
    let map = FrameIndexMap::new(&frames);
    assert_eq!(map.frame_count(), 1);

    let mut viz_index = VizIndex::new();

    // Frame-level viz primitive
    let frame_viz = make_viz("vp9_viz_frame_0", "vp9_decode_frame_0", "Frame");
    viz_index.add(frame_viz);

    // Segmentation overlay viz primitive
    let mut seg_overlay = make_viz("vp9_viz_seg_overlay", "vp9_decode_frame_0", "Overlay");
    seg_overlay.set_screen_rect(0.0, 0.0, 1920.0, 1080.0);
    seg_overlay.add_visual_property("type".to_string(), "segmentation_map".to_string());
    seg_overlay.add_visual_property("num_segments".to_string(), "4".to_string());
    viz_index.add(seg_overlay);

    // Segment 0 viz primitive (background - low QP)
    let mut seg0_viz = make_viz("vp9_viz_seg0", "vp9_decode_seg0", "Rectangle");
    seg0_viz.set_screen_rect(0.0, 0.0, 960.0, 540.0);
    seg0_viz.add_visual_property("segment_id".to_string(), "0".to_string());
    seg0_viz.add_visual_property("color".to_string(), "#00FF00".to_string()); // Green
    seg0_viz.add_visual_property("qp_offset".to_string(), "-10".to_string());
    seg0_viz.add_visual_property("opacity".to_string(), "0.3".to_string());
    viz_index.add(seg0_viz);

    // Segment 1 viz primitive (foreground - high QP)
    let mut seg1_viz = make_viz("vp9_viz_seg1", "vp9_decode_seg1", "Rectangle");
    seg1_viz.set_screen_rect(960.0, 0.0, 960.0, 540.0);
    seg1_viz.add_visual_property("segment_id".to_string(), "1".to_string());
    seg1_viz.add_visual_property("color".to_string(), "#FF0000".to_string()); // Red
    seg1_viz.add_visual_property("qp_offset".to_string(), "+15".to_string());
    seg1_viz.add_visual_property("opacity".to_string(), "0.5".to_string());
    viz_index.add(seg1_viz);

    // Segment 2 viz primitive (skip segment)
    let mut seg2_viz = make_viz("vp9_viz_seg2", "vp9_decode_seg2", "Rectangle");
    seg2_viz.set_screen_rect(0.0, 540.0, 960.0, 540.0);
    seg2_viz.add_visual_property("segment_id".to_string(), "2".to_string());
    seg2_viz.add_visual_property("color".to_string(), "#0000FF".to_string()); // Blue
    seg2_viz.add_visual_property("skip".to_string(), "true".to_string());
    seg2_viz.add_visual_property("opacity".to_string(), "0.2".to_string());
    viz_index.add(seg2_viz);

    // Segment 3 viz primitive (text region)
    let mut seg3_viz = make_viz("vp9_viz_seg3", "vp9_decode_seg3", "Rectangle");
    seg3_viz.set_screen_rect(960.0, 540.0, 960.0, 540.0);
    seg3_viz.add_visual_property("segment_id".to_string(), "3".to_string());
    seg3_viz.add_visual_property("color".to_string(), "#FFFF00".to_string()); // Yellow
    seg3_viz.add_visual_property("qp_offset".to_string(), "-5".to_string());
    seg3_viz.add_visual_property("opacity".to_string(), "0.4".to_string());
    viz_index.add(seg3_viz);

    assert_eq!(viz_index.len(), 6); // 1 frame + 1 overlay + 4 segments

    // Query segmentation overlay
    let found = viz_index.find_by_id("vp9_viz_seg_overlay");
    assert!(found.is_some());
    let overlay = found.unwrap();
    assert_eq!(
        overlay.visual_properties.get("type"),
        Some(&"segmentation_map".to_string())
    );
    assert_eq!(
        overlay.visual_properties.get("num_segments"),
        Some(&"4".to_string())
    );

    // Query segment 0 viz
    let found = viz_index.find_by_id("vp9_viz_seg0");
    assert!(found.is_some());
    let seg0 = found.unwrap();
    assert_eq!(
        seg0.visual_properties.get("color"),
        Some(&"#00FF00".to_string())
    );
    assert_eq!(
        seg0.visual_properties.get("qp_offset"),
        Some(&"-10".to_string())
    );

    // Query segment 2 (skip segment)
    let found = viz_index.find_by_id("vp9_viz_seg2");
    assert!(found.is_some());
    let seg2 = found.unwrap();
    assert_eq!(
        seg2.visual_properties.get("skip"),
        Some(&"true".to_string())
    );
}

#[test]
fn test_vp9_evidence_004_reference_frame_viz() {
    // Test: VP9 reference frame usage visualization
    // Scenario: Blocks using different reference frames (LAST, GOLDEN, ALTREF)
    // Per VP9 spec: Color-code blocks by reference frame

    let frames = vec![FrameMetadata {
        pts: Some(0),
        dts: Some(0),
    }];
    let map = FrameIndexMap::new(&frames);
    assert_eq!(map.frame_count(), 1);

    let mut viz_index = VizIndex::new();

    // Reference frame overlay
    let mut ref_overlay = make_viz("vp9_viz_ref_overlay", "vp9_decode_frame_0", "Overlay");
    ref_overlay.add_visual_property("type".to_string(), "reference_frames".to_string());
    viz_index.add(ref_overlay);

    // Block using LAST reference
    let mut block_last = make_viz("vp9_viz_block_last", "vp9_decode_block_0", "Rectangle");
    block_last.set_screen_rect(0.0, 0.0, 64.0, 64.0);
    block_last.add_visual_property("ref_frame".to_string(), "LAST".to_string());
    block_last.add_visual_property("color".to_string(), "#FF0000".to_string()); // Red for LAST
    block_last.add_visual_property("opacity".to_string(), "0.5".to_string());
    viz_index.add(block_last);

    // Block using GOLDEN reference
    let mut block_golden = make_viz("vp9_viz_block_golden", "vp9_decode_block_1", "Rectangle");
    block_golden.set_screen_rect(64.0, 0.0, 64.0, 64.0);
    block_golden.add_visual_property("ref_frame".to_string(), "GOLDEN".to_string());
    block_golden.add_visual_property("color".to_string(), "#FFD700".to_string()); // Gold for GOLDEN
    block_golden.add_visual_property("opacity".to_string(), "0.5".to_string());
    viz_index.add(block_golden);

    // Block using ALTREF reference
    let mut block_altref = make_viz("vp9_viz_block_altref", "vp9_decode_block_2", "Rectangle");
    block_altref.set_screen_rect(128.0, 0.0, 64.0, 64.0);
    block_altref.add_visual_property("ref_frame".to_string(), "ALTREF".to_string());
    block_altref.add_visual_property("color".to_string(), "#00FFFF".to_string()); // Cyan for ALTREF
    block_altref.add_visual_property("opacity".to_string(), "0.5".to_string());
    viz_index.add(block_altref);

    // Block using compound prediction (LAST + GOLDEN)
    let mut block_compound = make_viz("vp9_viz_block_compound", "vp9_decode_block_3", "Rectangle");
    block_compound.set_screen_rect(192.0, 0.0, 64.0, 64.0);
    block_compound.add_visual_property("ref_frame_0".to_string(), "LAST".to_string());
    block_compound.add_visual_property("ref_frame_1".to_string(), "GOLDEN".to_string());
    block_compound.add_visual_property("compound".to_string(), "true".to_string());
    block_compound.add_visual_property("color".to_string(), "#FF8800".to_string()); // Orange for compound
    block_compound.add_visual_property("opacity".to_string(), "0.6".to_string());
    viz_index.add(block_compound);

    assert_eq!(viz_index.len(), 5);

    // Query LAST block
    let found = viz_index.find_by_id("vp9_viz_block_last");
    assert!(found.is_some());
    let last_block = found.unwrap();
    assert_eq!(
        last_block.visual_properties.get("ref_frame"),
        Some(&"LAST".to_string())
    );
    assert_eq!(
        last_block.visual_properties.get("color"),
        Some(&"#FF0000".to_string())
    );

    // Query compound block
    let found = viz_index.find_by_id("vp9_viz_block_compound");
    assert!(found.is_some());
    let compound_block = found.unwrap();
    assert_eq!(
        compound_block.visual_properties.get("compound"),
        Some(&"true".to_string())
    );
    assert_eq!(
        compound_block.visual_properties.get("ref_frame_0"),
        Some(&"LAST".to_string())
    );
    assert_eq!(
        compound_block.visual_properties.get("ref_frame_1"),
        Some(&"GOLDEN".to_string())
    );
}

#[test]
fn test_vp9_evidence_004_loop_filter_heatmap() {
    // Test: VP9 loop filter strength heatmap visualization
    // Scenario: Different blocks with varying loop filter levels
    // Per VP9 spec: Loop filter level + segment/mode deltas

    let frames = vec![FrameMetadata {
        pts: Some(0),
        dts: Some(0),
    }];
    let map = FrameIndexMap::new(&frames);
    assert_eq!(map.frame_count(), 1);

    let mut viz_index = VizIndex::new();

    // Loop filter overlay
    let mut lf_overlay = make_viz("vp9_viz_lf_overlay", "vp9_decode_frame_0", "Overlay");
    lf_overlay.add_visual_property("type".to_string(), "loop_filter_heatmap".to_string());
    lf_overlay.add_visual_property("base_level".to_string(), "32".to_string());
    viz_index.add(lf_overlay);

    // Block with low loop filter (32 - 10 = 22)
    let mut block_low = make_viz("vp9_viz_lf_block_low", "vp9_decode_block_0", "Rectangle");
    block_low.set_screen_rect(0.0, 0.0, 64.0, 64.0);
    block_low.add_visual_property("lf_level".to_string(), "22".to_string());
    block_low.add_visual_property("color".to_string(), "#00FF00".to_string()); // Green (low)
    block_low.add_visual_property("opacity".to_string(), "0.4".to_string());
    viz_index.add(block_low);

    // Block with medium loop filter (32)
    let mut block_med = make_viz("vp9_viz_lf_block_med", "vp9_decode_block_1", "Rectangle");
    block_med.set_screen_rect(64.0, 0.0, 64.0, 64.0);
    block_med.add_visual_property("lf_level".to_string(), "32".to_string());
    block_med.add_visual_property("color".to_string(), "#FFFF00".to_string()); // Yellow (medium)
    block_med.add_visual_property("opacity".to_string(), "0.5".to_string());
    viz_index.add(block_med);

    // Block with high loop filter (32 + 20 = 52)
    let mut block_high = make_viz("vp9_viz_lf_block_high", "vp9_decode_block_2", "Rectangle");
    block_high.set_screen_rect(128.0, 0.0, 64.0, 64.0);
    block_high.add_visual_property("lf_level".to_string(), "52".to_string());
    block_high.add_visual_property("color".to_string(), "#FF0000".to_string()); // Red (high)
    block_high.add_visual_property("opacity".to_string(), "0.7".to_string());
    viz_index.add(block_high);

    // Block with loop filter disabled (0)
    let mut block_off = make_viz("vp9_viz_lf_block_off", "vp9_decode_block_3", "Rectangle");
    block_off.set_screen_rect(192.0, 0.0, 64.0, 64.0);
    block_off.add_visual_property("lf_level".to_string(), "0".to_string());
    block_off.add_visual_property("color".to_string(), "#000000".to_string()); // Black (off)
    block_off.add_visual_property("opacity".to_string(), "0.2".to_string());
    viz_index.add(block_off);

    assert_eq!(viz_index.len(), 5);

    // Query loop filter overlay
    let found = viz_index.find_by_id("vp9_viz_lf_overlay");
    assert!(found.is_some());
    let overlay = found.unwrap();
    assert_eq!(
        overlay.visual_properties.get("base_level"),
        Some(&"32".to_string())
    );

    // Query low filter block
    let found = viz_index.find_by_id("vp9_viz_lf_block_low");
    assert!(found.is_some());
    let low_block = found.unwrap();
    assert_eq!(
        low_block.visual_properties.get("lf_level"),
        Some(&"22".to_string())
    );
    assert_eq!(
        low_block.visual_properties.get("color"),
        Some(&"#00FF00".to_string())
    );

    // Query high filter block
    let found = viz_index.find_by_id("vp9_viz_lf_block_high");
    assert!(found.is_some());
    let high_block = found.unwrap();
    assert_eq!(
        high_block.visual_properties.get("lf_level"),
        Some(&"52".to_string())
    );
}

#[test]
fn test_vp9_evidence_004_motion_vector_viz() {
    // Test: VP9 motion vector visualization
    // Scenario: Blocks with single and compound MVs
    // Per VP9 spec: Compound prediction uses 2 MVs per block

    let frames = vec![FrameMetadata {
        pts: Some(0),
        dts: Some(0),
    }];
    let map = FrameIndexMap::new(&frames);
    assert_eq!(map.frame_count(), 1);

    let mut viz_index = VizIndex::new();

    // Motion vector overlay
    let mut mv_overlay = make_viz("vp9_viz_mv_overlay", "vp9_decode_frame_0", "Overlay");
    mv_overlay.add_visual_property("type".to_string(), "motion_vectors".to_string());
    viz_index.add(mv_overlay);

    // Block with single MV (LAST reference)
    let mut block_single_mv = make_viz("vp9_viz_mv_single", "vp9_decode_block_0", "Arrow");
    block_single_mv.set_screen_rect(32.0, 32.0, 8.0, 4.0); // Position and size
    block_single_mv.add_visual_property("mv_x".to_string(), "8".to_string()); // 1 pixel = 8/8 pel
    block_single_mv.add_visual_property("mv_y".to_string(), "4".to_string()); // 1/2 pixel
    block_single_mv.add_visual_property("ref_frame".to_string(), "LAST".to_string());
    block_single_mv.add_visual_property("color".to_string(), "#FF0000".to_string());
    block_single_mv.add_visual_property("line_width".to_string(), "2".to_string());
    viz_index.add(block_single_mv);

    // Block with compound MV (LAST + GOLDEN)
    let mut block_compound_mv0 = make_viz("vp9_viz_mv_compound_0", "vp9_decode_block_1", "Arrow");
    block_compound_mv0.set_screen_rect(96.0, 32.0, 12.0, 8.0);
    block_compound_mv0.add_visual_property("mv_x".to_string(), "12".to_string());
    block_compound_mv0.add_visual_property("mv_y".to_string(), "8".to_string());
    block_compound_mv0.add_visual_property("ref_frame".to_string(), "LAST".to_string());
    block_compound_mv0.add_visual_property("compound_idx".to_string(), "0".to_string());
    block_compound_mv0.add_visual_property("color".to_string(), "#FF0000".to_string());
    viz_index.add(block_compound_mv0);

    // Second MV for compound prediction
    let mut block_compound_mv1 = make_viz("vp9_viz_mv_compound_1", "vp9_decode_block_1", "Arrow");
    block_compound_mv1.set_screen_rect(96.0, 32.0, 8.0, 6.0);
    block_compound_mv1.add_visual_property("mv_x".to_string(), "8".to_string());
    block_compound_mv1.add_visual_property("mv_y".to_string(), "6".to_string());
    block_compound_mv1.add_visual_property("ref_frame".to_string(), "GOLDEN".to_string());
    block_compound_mv1.add_visual_property("compound_idx".to_string(), "1".to_string());
    block_compound_mv1.add_visual_property("color".to_string(), "#FFD700".to_string());
    viz_index.add(block_compound_mv1);

    // Block with zero MV (intra or skip)
    let mut block_zero_mv = make_viz("vp9_viz_mv_zero", "vp9_decode_block_2", "Circle");
    block_zero_mv.set_screen_rect(160.0, 32.0, 4.0, 4.0); // Small circle
    block_zero_mv.add_visual_property("mv_x".to_string(), "0".to_string());
    block_zero_mv.add_visual_property("mv_y".to_string(), "0".to_string());
    block_zero_mv.add_visual_property("skip".to_string(), "true".to_string());
    block_zero_mv.add_visual_property("color".to_string(), "#AAAAAA".to_string());
    viz_index.add(block_zero_mv);

    assert_eq!(viz_index.len(), 5); // 1 overlay + 1 single + 2 compound + 1 zero

    // Query single MV
    let found = viz_index.find_by_id("vp9_viz_mv_single");
    assert!(found.is_some());
    let single_mv = found.unwrap();
    assert_eq!(
        single_mv.visual_properties.get("mv_x"),
        Some(&"8".to_string())
    );
    assert_eq!(
        single_mv.visual_properties.get("ref_frame"),
        Some(&"LAST".to_string())
    );

    // Query compound MV (first reference)
    let found = viz_index.find_by_id("vp9_viz_mv_compound_0");
    assert!(found.is_some());
    let compound_mv0 = found.unwrap();
    assert_eq!(
        compound_mv0.visual_properties.get("compound_idx"),
        Some(&"0".to_string())
    );

    // Query compound MV (second reference)
    let found = viz_index.find_by_id("vp9_viz_mv_compound_1");
    assert!(found.is_some());
    let compound_mv1 = found.unwrap();
    assert_eq!(
        compound_mv1.visual_properties.get("compound_idx"),
        Some(&"1".to_string())
    );
    assert_eq!(
        compound_mv1.visual_properties.get("ref_frame"),
        Some(&"GOLDEN".to_string())
    );
}

#[test]
fn test_vp9_evidence_004_altref_frame_viz() {
    // Test: VP9 ALTREF frame visualization marker
    // Scenario: ALTREF frame is invisible but should be marked in viz
    // Per VP9 spec: show_frame=0 for ALTREF

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

    let mut viz_index = VizIndex::new();

    // K0 viz
    let mut k0_viz = make_viz("vp9_viz_k0", "vp9_decode_k0", "Frame");
    k0_viz.add_visual_property("frame_type".to_string(), "KEY_FRAME".to_string());
    k0_viz.add_visual_property("show_frame".to_string(), "1".to_string());
    viz_index.add(k0_viz);

    // ALTREF viz (special marker)
    let mut altref_viz = make_viz("vp9_viz_altref", "vp9_decode_altref", "Marker");
    altref_viz.add_visual_property("frame_type".to_string(), "ALTREF".to_string());
    altref_viz.add_visual_property("show_frame".to_string(), "0".to_string());
    altref_viz.add_visual_property("invisible".to_string(), "true".to_string());
    altref_viz.add_visual_property("marker_color".to_string(), "#00FFFF".to_string());
    altref_viz.add_visual_property("marker_shape".to_string(), "diamond".to_string());
    viz_index.add(altref_viz);

    // P2 viz (references ALTREF)
    let mut p2_viz = make_viz("vp9_viz_p2", "vp9_decode_p2", "Frame");
    p2_viz.add_visual_property("frame_type".to_string(), "INTER_FRAME".to_string());
    p2_viz.add_visual_property("show_frame".to_string(), "1".to_string());
    p2_viz.add_visual_property("refs_altref".to_string(), "true".to_string());
    viz_index.add(p2_viz);

    assert_eq!(viz_index.len(), 3);

    // Query ALTREF viz marker
    let found = viz_index.find_by_id("vp9_viz_altref");
    assert!(found.is_some());
    let altref = found.unwrap();
    assert_eq!(
        altref.visual_properties.get("invisible"),
        Some(&"true".to_string())
    );
    assert_eq!(
        altref.visual_properties.get("marker_shape"),
        Some(&"diamond".to_string())
    );

    // Query P2 that references ALTREF
    let found = viz_index.find_by_id("vp9_viz_p2");
    assert!(found.is_some());
    let p2 = found.unwrap();
    assert_eq!(
        p2.visual_properties.get("refs_altref"),
        Some(&"true".to_string())
    );
}

#[test]
fn test_vp9_evidence_004_show_existing_frame_viz() {
    // Test: VP9 show_existing_frame visualization
    // Scenario: Virtual frame that displays previous decoded frame
    // Per VP9 spec: Minimal frame that references buffer

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

    let mut viz_index = VizIndex::new();

    // P1 viz
    let mut p1_viz = make_viz("vp9_viz_p1", "vp9_decode_p1", "Frame");
    p1_viz.add_visual_property("frame_type".to_string(), "INTER_FRAME".to_string());
    p1_viz.add_visual_property("stored_in_buf".to_string(), "1".to_string());
    viz_index.add(p1_viz);

    // show_existing viz (virtual frame)
    let mut show_viz = make_viz(
        "vp9_viz_show_existing",
        "vp9_decode_show_existing",
        "VirtualFrame",
    );
    show_viz.add_visual_property("show_existing_frame".to_string(), "1".to_string());
    show_viz.add_visual_property("frame_to_show_map_idx".to_string(), "1".to_string());
    show_viz.add_visual_property("virtual".to_string(), "true".to_string());
    show_viz.add_visual_property("border_style".to_string(), "dashed".to_string());
    show_viz.add_visual_property("border_color".to_string(), "#FFFF00".to_string());
    show_viz.add_visual_property("opacity".to_string(), "0.5".to_string());
    viz_index.add(show_viz);

    assert_eq!(viz_index.len(), 2);

    // Query show_existing viz
    let found = viz_index.find_by_id("vp9_viz_show_existing");
    assert!(found.is_some());
    let show = found.unwrap();
    assert_eq!(
        show.visual_properties.get("virtual"),
        Some(&"true".to_string())
    );
    assert_eq!(
        show.visual_properties.get("border_style"),
        Some(&"dashed".to_string())
    );
    assert_eq!(
        show.visual_properties.get("frame_to_show_map_idx"),
        Some(&"1".to_string())
    );
}

#[test]
fn test_vp9_evidence_004_empty_stream() {
    // Test: Empty stream has no viz evidence
    let frames: Vec<FrameMetadata> = vec![];
    let map = FrameIndexMap::new(&frames);
    let viz_index = VizIndex::new();

    assert_eq!(map.frame_count(), 0);
    assert_eq!(viz_index.len(), 0);
    assert!(viz_index.is_empty());
}
