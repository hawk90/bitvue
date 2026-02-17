#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(unused_comparisons)]
#![allow(unused_doc_comments)]
//! Tests for HEVC frames.rs public API.
//! Targeting 95%+ line coverage for frame extraction methods.

use bitvue_hevc::frames::{
    extract_annex_b_frames, extract_frame_at_index, HevcFrame, HevcFrameType,
};

// ============================================================================
// HevcFrame Struct Tests
// ============================================================================

#[test]
fn test_hevc_frame_default() {
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
        .temporal_id(0)
        .build();

    // Verify all fields match default values
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
    assert!(frame.slice_header.is_none());
}

#[test]
fn test_hevc_frame_clone() {
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
        .temporal_id(0)
        .build();

    let cloned = frame.clone();

    // Verify clone copies all fields
    assert_eq!(cloned.frame_index, 0);
    assert_eq!(cloned.frame_type, HevcFrameType::I);
    assert_eq!(cloned.nal_data, vec![0x00, 0x00, 0x01]);
    assert_eq!(cloned.offset, 0);
    assert_eq!(cloned.size, 3);
    assert_eq!(cloned.poc, 0);
    assert_eq!(cloned.frame_num, 0);
    assert!(cloned.is_idr);
    assert!(cloned.is_irap);
    assert!(cloned.is_ref);
}

#[test]
fn test_hevc_frame_builder_idr_frame() {
    let frame = HevcFrame::builder()
        .frame_index(5)
        .frame_type(HevcFrameType::P)
        .nal_data(vec![0x00, 0x00, 0x01, 0x02, 0x03])
        .offset(100)
        .size(15)
        .poc(10)
        .frame_num(2)
        .is_idr(false)
        .is_irap(true)
        .is_ref(false)
        .temporal_id(5)
        .build();

    // Verify builder sets all fields correctly
    assert_eq!(frame.frame_index, 5);
    assert_eq!(frame.frame_type, HevcFrameType::P);
    assert_eq!(frame.nal_data.len(), 5);
    assert_eq!(frame.offset, 100);
    assert_eq!(frame.size, 15);
    assert_eq!(frame.poc, 10);
    assert_eq!(frame.frame_num, 2);
    assert!(!frame.is_idr);
    assert!(frame.is_irap);
    assert!(!frame.is_ref);
    assert_eq!(frame.temporal_id, Some(5));
}

#[test]
fn test_hevc_frame_type_from_slice() {
    assert_eq!(HevcFrameType::from_slice_type("I"), HevcFrameType::I);
    assert_eq!(HevcFrameType::from_slice_type("P"), HevcFrameType::P);
    assert_eq!(HevcFrameType::from_slice_type("B"), HevcFrameType::B);
    assert_eq!(HevcFrameType::from_slice_type("X"), HevcFrameType::Unknown);
}
