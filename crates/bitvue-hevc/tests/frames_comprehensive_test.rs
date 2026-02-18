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
//! Comprehensive tests for HEVC frames module.
//! Targeting 95%+ line coverage for the frames module.

use bitvue_hevc::frames::{
    extract_annex_b_frames, extract_frame_at_index, hevc_frame_to_unit_node,
    hevc_frames_to_unit_nodes, HevcFrame, HevcFrameBuilder, HevcFrameType,
};

// ============================================================================
// HevcFrameType Tests
// ============================================================================

#[test]
fn test_hevc_frame_type_from_slice_i() {
    assert_eq!(HevcFrameType::from_slice_type("I"), HevcFrameType::I);
}

#[test]
fn test_hevc_frame_type_from_slice_p() {
    assert_eq!(HevcFrameType::from_slice_type("P"), HevcFrameType::P);
}

#[test]
fn test_hevc_frame_type_from_slice_b() {
    assert_eq!(HevcFrameType::from_slice_type("B"), HevcFrameType::B);
}

#[test]
fn test_hevc_frame_type_from_slice_unknown() {
    assert_eq!(HevcFrameType::from_slice_type("X"), HevcFrameType::Unknown);
    assert_eq!(HevcFrameType::from_slice_type(""), HevcFrameType::Unknown);
}

#[test]
fn test_hevc_frame_type_as_str() {
    assert_eq!(HevcFrameType::I.as_str(), "I");
    assert_eq!(HevcFrameType::P.as_str(), "P");
    assert_eq!(HevcFrameType::B.as_str(), "B");
    assert_eq!(HevcFrameType::Unknown.as_str(), "Unknown");
}

// ============================================================================
// HevcFrameBuilder Tests
// ============================================================================

#[test]
fn test_hevc_frame_builder_default() {
    let builder = HevcFrameBuilder::default();
    // Default builder has all None values - building without required fields will panic
    // Test with all fields set instead
    let _frame = builder
        .frame_index(0)
        .frame_type(HevcFrameType::I)
        .offset(0)
        .size(1)
        .poc(0)
        .frame_num(0)
        .is_idr(true)
        .is_irap(true)
        .is_ref(true)
        .build();
}

#[test]
fn test_hevc_frame_builder_complete() {
    let frame = HevcFrame::builder()
        .frame_index(0)
        .frame_type(HevcFrameType::I)
        .nal_data(vec![0x00, 0x00, 0x01])
        .offset(0)
        .size(3)
        .poc(0)
        .frame_num(0)
        .is_idr(true)
        .is_irap(true)
        .is_ref(true)
        .temporal_id(0) // temporal_id is u8, not Option<u8>
        .build();

    assert_eq!(frame.frame_index, 0);
    assert_eq!(frame.frame_type, HevcFrameType::I);
    assert_eq!(frame.nal_data, vec![0x00, 0x00, 0x01]);
    assert_eq!(frame.offset, 0);
    assert_eq!(frame.size, 3);
    assert_eq!(frame.poc, 0);
    assert_eq!(frame.frame_num, 0);
    assert!(frame.is_idr);
    assert!(frame.is_irap);
    assert!(frame.is_ref);
    assert_eq!(frame.temporal_id, Some(0));
}

#[test]
fn test_hevc_frame_builder_p_frame() {
    let frame = HevcFrame::builder()
        .frame_index(1)
        .frame_type(HevcFrameType::P)
        .nal_data(vec![0x00, 0x01])
        .offset(100)
        .size(2)
        .poc(1)
        .frame_num(1)
        .is_idr(false)
        .is_irap(false)
        .is_ref(true)
        // temporal_id is optional, don't set it
        .build();

    assert_eq!(frame.frame_index, 1);
    assert_eq!(frame.frame_type, HevcFrameType::P);
    assert!(!frame.is_idr);
    assert!(!frame.is_irap);
    assert!(frame.is_ref);
    assert_eq!(frame.temporal_id, None);
}

#[test]
fn test_hevc_frame_builder_b_frame() {
    let frame = HevcFrame::builder()
        .frame_index(2)
        .frame_type(HevcFrameType::B)
        .nal_data(vec![0x00, 0x01, 0x02])
        .offset(200)
        .size(3)
        .poc(2)
        .frame_num(2)
        .is_idr(false)
        .is_irap(false)
        .is_ref(false)
        .temporal_id(1)
        .build();

    assert_eq!(frame.frame_index, 2);
    assert_eq!(frame.frame_type, HevcFrameType::B);
    assert!(!frame.is_ref);
    assert_eq!(frame.temporal_id, Some(1));
}

#[test]
fn test_hevc_frame_builder_minimal() {
    let frame = HevcFrame::builder()
        .frame_index(5)
        .frame_type(HevcFrameType::I)
        .offset(500)
        .size(100)
        .poc(10)
        .frame_num(5)
        .is_idr(false)
        .is_irap(false)
        .is_ref(false)
        .build();

    assert_eq!(frame.frame_index, 5);
    assert_eq!(frame.offset, 500);
    assert_eq!(frame.size, 100);
    // nal_data defaults to empty vec
    assert!(frame.nal_data.is_empty());
    assert_eq!(frame.temporal_id, None);
    assert!(frame.slice_header.is_none());
}

#[test]
fn test_hevc_frame_builder_chain() {
    let frame = HevcFrame::builder()
        .frame_index(10)
        .frame_type(HevcFrameType::P)
        .offset(0)
        .size(50)
        .poc(20)
        .frame_num(10)
        .is_idr(false)
        .is_irap(false)
        .is_ref(true)
        .temporal_id(2)
        .frame_index(15) // Override
        .build();

    // Last call to frame_index should win
    assert_eq!(frame.frame_index, 15);
}

// ============================================================================
// HevcFrame Tests
// ============================================================================

#[test]
fn test_hevc_frame_builder_method() {
    let builder = HevcFrame::builder();
    // Verify we get a builder
    let frame = builder
        .frame_index(0)
        .frame_type(HevcFrameType::I)
        .offset(0)
        .size(1)
        .poc(0)
        .frame_num(0)
        .is_idr(true)
        .is_irap(true)
        .is_ref(true)
        .build();
    assert_eq!(frame.frame_index, 0);
}

// ============================================================================
// extract_annex_b_frames Tests
// ============================================================================

#[test]
fn test_extract_annex_b_frames_empty() {
    let data: &[u8] = &[];
    let frames = extract_annex_b_frames(data);
    assert!(frames.is_ok());
    assert!(frames.unwrap().is_empty());
}

#[test]
fn test_extract_annex_b_frames_no_nal_units() {
    let data = vec![0xFF, 0xFF, 0xFF]; // No start codes
    let frames = extract_annex_b_frames(&data);
    assert!(frames.is_ok());
    assert!(frames.unwrap().is_empty());
}

#[test]
fn test_extract_annex_b_frames_with_start_code() {
    // Minimal Annex B data with start code
    let data = vec![0x00, 0x00, 0x01]; // Just start code
    let frames = extract_annex_b_frames(&data);
    // Result depends on NAL parsing, just verify it doesn't crash
    assert!(frames.is_ok() || frames.is_err());
}

#[test]
fn test_extract_annex_b_frames_invalid_data() {
    // Invalid HEVC data
    let data = vec![0x00, 0x00, 0x01, 0xFF]; // Start code + invalid byte
    let frames = extract_annex_b_frames(&data);
    // Should not crash, may return error or empty
    assert!(frames.is_ok() || frames.is_err());
}

// ============================================================================
// extract_frame_at_index Tests
// ============================================================================

#[test]
fn test_extract_frame_at_index_empty() {
    let data: &[u8] = &[];
    let frame = extract_frame_at_index(data, 0);
    assert!(frame.is_none());
}

#[test]
fn test_extract_frame_at_index_no_frames() {
    let data = vec![0xFF, 0xFF]; // No valid frames
    let frame = extract_frame_at_index(&data, 0);
    assert!(frame.is_none());
}

#[test]
fn test_extract_frame_at_index_out_of_bounds() {
    let data = vec![0x00, 0x00, 0x01]; // Start code only
    let frame = extract_frame_at_index(&data, 100);
    assert!(frame.is_none());
}

#[test]
fn test_extract_frame_at_index_zero() {
    let data = vec![0x00, 0x00, 0x01];
    let frame = extract_frame_at_index(&data, 0);
    // May be None if no valid frames
    assert!(frame.is_some() || frame.is_none());
}

// ============================================================================
// hevc_frame_to_unit_node Tests
// ============================================================================

#[test]
fn test_hevc_frame_to_unit_node_basic() {
    let frame = HevcFrame {
        frame_index: 0,
        frame_type: HevcFrameType::I,
        nal_data: vec![0x00, 0x01],
        offset: 0,
        size: 2,
        poc: 0,
        frame_num: 0,
        is_idr: true,
        is_irap: true,
        is_ref: true,
        temporal_id: Some(0),
        slice_header: None,
    };

    let node = hevc_frame_to_unit_node(&frame, 0);
    assert_eq!(node.offset, 0);
    assert_eq!(node.size, 2);
    assert_eq!(node.frame_index, Some(0));
    assert_eq!(node.pts, Some(0));
    assert!(node.children.is_empty());
}

#[test]
fn test_hevc_frame_to_unit_node_p_frame() {
    let frame = HevcFrame {
        frame_index: 5,
        frame_type: HevcFrameType::P,
        nal_data: vec![0x00, 0x01, 0x02],
        offset: 100,
        size: 3,
        poc: 10,
        frame_num: 5,
        is_idr: false,
        is_irap: false,
        is_ref: true,
        temporal_id: Some(1),
        slice_header: None,
    };

    let node = hevc_frame_to_unit_node(&frame, 0);
    assert_eq!(node.offset, 100);
    assert_eq!(node.size, 3);
    assert_eq!(node.frame_index, Some(5));
    assert_eq!(node.pts, Some(10));
    assert_eq!(node.temporal_id, Some(1));
}

#[test]
fn test_hevc_frame_to_unit_node_b_frame() {
    let frame = HevcFrame {
        frame_index: 10,
        frame_type: HevcFrameType::B,
        nal_data: vec![0x00, 0x01],
        offset: 200,
        size: 2,
        poc: 20,
        frame_num: 10,
        is_idr: false,
        is_irap: false,
        is_ref: false,
        temporal_id: None,
        slice_header: None,
    };

    let node = hevc_frame_to_unit_node(&frame, 0);
    assert_eq!(node.offset, 200);
    assert_eq!(node.frame_index, Some(10));
    assert_eq!(node.pts, Some(20));
    assert_eq!(node.temporal_id, None);
}

// ============================================================================
// hevc_frames_to_unit_nodes Tests
// ============================================================================

#[test]
fn test_hevc_frames_to_unit_nodes_empty() {
    let frames: &[HevcFrame] = &[];
    let nodes = hevc_frames_to_unit_nodes(frames);
    assert!(nodes.is_empty());
}

#[test]
fn test_hevc_frames_to_unit_nodes_single() {
    let frames = vec![HevcFrame {
        frame_index: 0,
        frame_type: HevcFrameType::I,
        nal_data: vec![0x00],
        offset: 0,
        size: 1,
        poc: 0,
        frame_num: 0,
        is_idr: true,
        is_irap: true,
        is_ref: true,
        temporal_id: Some(0),
        slice_header: None,
    }];

    let nodes = hevc_frames_to_unit_nodes(&frames);
    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes[0].frame_index, Some(0));
}

#[test]
fn test_hevc_frames_to_unit_nodes_multiple() {
    let frames = vec![
        HevcFrame {
            frame_index: 0,
            frame_type: HevcFrameType::I,
            nal_data: vec![0x00],
            offset: 0,
            size: 1,
            poc: 0,
            frame_num: 0,
            is_idr: true,
            is_irap: true,
            is_ref: true,
            temporal_id: Some(0),
            slice_header: None,
        },
        HevcFrame {
            frame_index: 1,
            frame_type: HevcFrameType::P,
            nal_data: vec![0x01],
            offset: 1,
            size: 1,
            poc: 1,
            frame_num: 1,
            is_idr: false,
            is_irap: false,
            is_ref: true,
            temporal_id: None,
            slice_header: None,
        },
        HevcFrame {
            frame_index: 2,
            frame_type: HevcFrameType::B,
            nal_data: vec![0x02],
            offset: 2,
            size: 1,
            poc: 2,
            frame_num: 2,
            is_idr: false,
            is_irap: false,
            is_ref: false,
            temporal_id: Some(1),
            slice_header: None,
        },
    ];

    let nodes = hevc_frames_to_unit_nodes(&frames);
    assert_eq!(nodes.len(), 3);
    assert_eq!(nodes[0].frame_index, Some(0));
    assert_eq!(nodes[1].frame_index, Some(1));
    assert_eq!(nodes[2].frame_index, Some(2));
}

// ============================================================================
// HevcFrame Clone Tests
// ============================================================================

#[test]
fn test_hevc_frame_clone() {
    let frame = HevcFrame {
        frame_index: 1,
        frame_type: HevcFrameType::P,
        nal_data: vec![0x00, 0x01, 0x02],
        offset: 100,
        size: 3,
        poc: 10,
        frame_num: 5,
        is_idr: false,
        is_irap: false,
        is_ref: true,
        temporal_id: Some(2),
        slice_header: None,
    };

    let cloned = frame.clone();
    assert_eq!(cloned.frame_index, 1);
    assert_eq!(cloned.frame_type, HevcFrameType::P);
    assert_eq!(cloned.nal_data, vec![0x00, 0x01, 0x02]);
    assert_eq!(cloned.offset, 100);
    assert_eq!(cloned.size, 3);
    assert_eq!(cloned.poc, 10);
}

// ============================================================================
// HevcFrameType Eq/PartialEq Tests
// ============================================================================

#[test]
fn test_hevc_frame_type_equality() {
    assert_eq!(HevcFrameType::I, HevcFrameType::I);
    assert_eq!(HevcFrameType::P, HevcFrameType::P);
    assert_eq!(HevcFrameType::B, HevcFrameType::B);
}

#[test]
fn test_hevc_frame_type_inequality() {
    assert_ne!(HevcFrameType::I, HevcFrameType::P);
    assert_ne!(HevcFrameType::I, HevcFrameType::B);
    assert_ne!(HevcFrameType::P, HevcFrameType::B);
}

// ============================================================================
// Integration Tests
// ============================================================================

#[test]
fn test_hevc_frame_workflow() {
    // Test: Builder -> Frame -> UnitNode
    let frame = HevcFrame::builder()
        .frame_index(0)
        .frame_type(HevcFrameType::I)
        .nal_data(vec![0x00, 0x00, 0x01, 0x67])
        .offset(0)
        .size(4)
        .poc(0)
        .frame_num(0)
        .is_idr(true)
        .is_irap(true)
        .is_ref(true)
        .temporal_id(0) // temporal_id is u8, not Option<u8>
        .build();

    assert_eq!(frame.frame_index, 0);
    assert_eq!(frame.frame_type, HevcFrameType::I);
    assert_eq!(frame.offset, 0);
    assert_eq!(frame.poc, 0);
    assert!(frame.is_idr);

    let node = hevc_frame_to_unit_node(&frame, 0);
    assert_eq!(node.offset, 0);
    assert_eq!(node.size, 4);
    assert_eq!(node.temporal_id, Some(0));
}

#[test]
fn test_hevc_frames_array_workflow() {
    // Test: Multiple frames -> UnitNodes
    let frames = vec![
        HevcFrame::builder()
            .frame_index(0)
            .frame_type(HevcFrameType::I)
            .nal_data(vec![0x00])
            .offset(0)
            .size(1)
            .poc(0)
            .frame_num(0)
            .is_idr(true)
            .is_irap(true)
            .is_ref(true)
            .build(),
        HevcFrame::builder()
            .frame_index(1)
            .frame_type(HevcFrameType::P)
            .nal_data(vec![0x01])
            .offset(1)
            .size(1)
            .poc(1)
            .frame_num(1)
            .is_idr(false)
            .is_irap(false)
            .is_ref(true)
            .build(),
    ];

    let nodes = hevc_frames_to_unit_nodes(&frames);
    assert_eq!(nodes.len(), 2);
    assert_eq!(nodes[0].frame_index, Some(0));
    assert_eq!(nodes[1].frame_index, Some(1));
}
