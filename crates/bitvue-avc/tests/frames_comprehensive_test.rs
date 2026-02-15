//! Comprehensive tests for AVC frames module
//!
//! Tests frame extraction, AvcFrameBuilder, AvcFrameType, and conversion functions

use bitvue_avc::frames::{
    avc_frame_to_unit_node, avc_frames_to_unit_nodes, extract_annex_b_frames,
    extract_frame_at_index, AvcFrame, AvcFrameType,
};
use bitvue_avc::slice::SliceType;

// ============================================================================
// AvcFrameType Tests
// ============================================================================

#[test]
fn test_frame_type_all_variants_as_str() {
    assert_eq!(AvcFrameType::I.as_str(), "I");
    assert_eq!(AvcFrameType::P.as_str(), "P");
    assert_eq!(AvcFrameType::B.as_str(), "B");
    assert_eq!(AvcFrameType::SI.as_str(), "SI");
    assert_eq!(AvcFrameType::SP.as_str(), "SP");
    assert_eq!(AvcFrameType::Unknown.as_str(), "Unknown");
}

#[test]
fn test_frame_type_from_all_slice_types() {
    // Test all SliceType variants
    assert_eq!(AvcFrameType::from_slice_type(SliceType::I), AvcFrameType::I);
    assert_eq!(AvcFrameType::from_slice_type(SliceType::P), AvcFrameType::P);
    assert_eq!(AvcFrameType::from_slice_type(SliceType::B), AvcFrameType::B);
    assert_eq!(
        AvcFrameType::from_slice_type(SliceType::Si),
        AvcFrameType::SI
    );
    assert_eq!(
        AvcFrameType::from_slice_type(SliceType::Sp),
        AvcFrameType::SP
    );
}

#[test]
fn test_frame_type_traits() {
    // Test Clone
    let frame_type = AvcFrameType::I;
    let cloned = frame_type;
    assert_eq!(frame_type, cloned);

    // Test Copy
    let copied = frame_type;
    assert_eq!(frame_type, copied);

    // Test PartialEq
    assert_eq!(AvcFrameType::I, AvcFrameType::I);
    assert_ne!(AvcFrameType::I, AvcFrameType::P);

    // Test Debug
    let debug_str = format!("{:?}", AvcFrameType::I);
    assert!(debug_str.contains("I"));
}

// ============================================================================
// AvcFrameBuilder Tests
// ============================================================================

#[test]
fn test_builder_minimal_valid_frame() {
    let frame = AvcFrame::builder()
        .frame_index(0)
        .frame_type(AvcFrameType::I)
        .offset(0)
        .size(100)
        .poc(0)
        .frame_num(0)
        .is_idr(true)
        .is_ref(true)
        .build()
        .unwrap();

    assert_eq!(frame.frame_index, 0);
    assert_eq!(frame.frame_type, AvcFrameType::I);
    assert_eq!(frame.offset, 0);
    assert_eq!(frame.size, 100);
    assert_eq!(frame.poc, 0);
    assert_eq!(frame.frame_num, 0);
    assert!(frame.is_idr);
    assert!(frame.is_ref);
    assert!(frame.nal_data.is_empty()); // Default value
    assert!(frame.slice_header.is_none()); // Default value
}

#[test]
fn test_builder_complete_frame_with_all_fields() {
    let frame = AvcFrame::builder()
        .frame_index(5)
        .frame_type(AvcFrameType::P)
        .nal_data(vec![0x00, 0x00, 0x00, 0x01, 0x67])
        .offset(1024)
        .size(2048)
        .poc(10)
        .frame_num(3)
        .is_idr(false)
        .is_ref(true)
        .build()
        .unwrap();

    assert_eq!(frame.frame_index, 5);
    assert_eq!(frame.frame_type, AvcFrameType::P);
    assert_eq!(frame.nal_data, vec![0x00, 0x00, 0x00, 0x01, 0x67]);
    assert_eq!(frame.offset, 1024);
    assert_eq!(frame.size, 2048);
    assert_eq!(frame.poc, 10);
    assert_eq!(frame.frame_num, 3);
    assert!(!frame.is_idr);
    assert!(frame.is_ref);
}

#[test]
fn test_builder_chaining_all_setters() {
    // Test that all setters can be chained and return Self
    let result = AvcFrame::builder()
        .frame_index(0)
        .frame_type(AvcFrameType::B)
        .nal_data(vec![])
        .offset(0)
        .size(0)
        .poc(0)
        .frame_num(0)
        .is_idr(false)
        .is_ref(false);

    // Verify the builder is still usable after chaining
    let frame = result.build().unwrap();
    assert_eq!(frame.frame_type, AvcFrameType::B);
}

#[test]
fn test_builder_missing_frame_index() {
    let result = AvcFrame::builder()
        .frame_type(AvcFrameType::I)
        .offset(0)
        .size(100)
        .poc(0)
        .frame_num(0)
        .is_idr(true)
        .is_ref(true)
        .build();

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("frame_index is required"));
}

#[test]
fn test_builder_missing_frame_type() {
    let result = AvcFrame::builder()
        .frame_index(0)
        .offset(0)
        .size(100)
        .poc(0)
        .frame_num(0)
        .is_idr(true)
        .is_ref(true)
        .build();

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("frame_type is required"));
}

#[test]
fn test_builder_missing_offset() {
    let result = AvcFrame::builder()
        .frame_index(0)
        .frame_type(AvcFrameType::I)
        .size(100)
        .poc(0)
        .frame_num(0)
        .is_idr(true)
        .is_ref(true)
        .build();

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("offset is required"));
}

#[test]
fn test_builder_missing_size() {
    let result = AvcFrame::builder()
        .frame_index(0)
        .frame_type(AvcFrameType::I)
        .offset(0)
        .poc(0)
        .frame_num(0)
        .is_idr(true)
        .is_ref(true)
        .build();

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("size is required"));
}

#[test]
fn test_builder_missing_poc() {
    let result = AvcFrame::builder()
        .frame_index(0)
        .frame_type(AvcFrameType::I)
        .offset(0)
        .size(100)
        .frame_num(0)
        .is_idr(true)
        .is_ref(true)
        .build();

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("poc is required"));
}

#[test]
fn test_builder_missing_frame_num() {
    let result = AvcFrame::builder()
        .frame_index(0)
        .frame_type(AvcFrameType::I)
        .offset(0)
        .size(100)
        .poc(0)
        .is_idr(true)
        .is_ref(true)
        .build();

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("frame_num is required"));
}

#[test]
fn test_builder_missing_is_idr() {
    let result = AvcFrame::builder()
        .frame_index(0)
        .frame_type(AvcFrameType::I)
        .offset(0)
        .size(100)
        .poc(0)
        .frame_num(0)
        .is_ref(true)
        .build();

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("is_idr is required"));
}

#[test]
fn test_builder_missing_is_ref() {
    let result = AvcFrame::builder()
        .frame_index(0)
        .frame_type(AvcFrameType::I)
        .offset(0)
        .size(100)
        .poc(0)
        .frame_num(0)
        .is_idr(true)
        .build();

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("is_ref is required"));
}

#[test]
fn test_builder_default_nal_data() {
    let frame = AvcFrame::builder()
        .frame_index(0)
        .frame_type(AvcFrameType::I)
        .offset(0)
        .size(0)
        .poc(0)
        .frame_num(0)
        .is_idr(true)
        .is_ref(true)
        .build()
        .unwrap();

    // nal_data should default to empty vec
    assert!(frame.nal_data.is_empty());
}

#[test]
fn test_builder_negative_poc() {
    let frame = AvcFrame::builder()
        .frame_index(0)
        .frame_type(AvcFrameType::B)
        .offset(0)
        .size(100)
        .poc(-5) // Negative POC is valid for B-frames
        .frame_num(0)
        .is_idr(false)
        .is_ref(false)
        .build()
        .unwrap();

    assert_eq!(frame.poc, -5);
}

// ============================================================================
// AvcFrame Tests
// ============================================================================

#[test]
fn test_frame_builder_method() {
    // Test that AvcFrame::builder() creates a new builder
    let builder = AvcFrame::builder();
    let frame = builder
        .frame_index(0)
        .frame_type(AvcFrameType::I)
        .offset(0)
        .size(100)
        .poc(0)
        .frame_num(0)
        .is_idr(true)
        .is_ref(true)
        .build()
        .unwrap();

    assert_eq!(frame.frame_index, 0);
}

#[test]
fn test_frame_clone() {
    let frame1 = AvcFrame::builder()
        .frame_index(0)
        .frame_type(AvcFrameType::I)
        .nal_data(vec![1, 2, 3])
        .offset(0)
        .size(100)
        .poc(0)
        .frame_num(0)
        .is_idr(true)
        .is_ref(true)
        .build()
        .unwrap();

    let frame2 = frame1.clone();

    assert_eq!(frame1.frame_index, frame2.frame_index);
    assert_eq!(frame1.frame_type, frame2.frame_type);
    assert_eq!(frame1.nal_data, frame2.nal_data);
    assert_eq!(frame1.offset, frame2.offset);
    assert_eq!(frame1.size, frame2.size);
    assert_eq!(frame1.poc, frame2.poc);
}

// ============================================================================
// extract_annex_b_frames Tests
// ============================================================================

#[test]
fn test_extract_empty_data() {
    let data: &[u8] = &[];
    let frames = extract_annex_b_frames(data);
    assert!(frames.is_ok());
    assert!(frames.unwrap().is_empty());
}

#[test]
fn test_extract_no_start_codes() {
    // Data without start codes
    let data = vec![0xFF, 0xFF, 0xFF, 0xFF];
    let frames = extract_annex_b_frames(&data);
    // Should return empty vec (no error), but no frames found
    assert!(frames.is_ok());
    assert!(frames.unwrap().is_empty());
}

#[test]
fn test_extract_single_start_code_no_data() {
    // Just a start code with no following data
    let data = vec![0x00, 0x00, 0x00, 0x01];
    let frames = extract_annex_b_frames(&data);
    assert!(frames.is_ok());
}

#[test]
fn test_extract_three_byte_start_code() {
    // 3-byte start code
    let data = vec![0x00, 0x00, 0x01, 0x67];
    let frames = extract_annex_b_frames(&data);
    assert!(frames.is_ok());
}

#[test]
fn test_extract_four_byte_start_code() {
    // 4-byte start code
    let data = vec![0x00, 0x00, 0x00, 0x01, 0x67];
    let frames = extract_annex_b_frames(&data);
    assert!(frames.is_ok());
}

#[test]
fn test_extract_corrupted_nal_header() {
    // Start code followed by invalid NAL header
    let data = vec![0x00, 0x00, 0x00, 0x01, 0xFF]; // 0xFF is reserved
    let frames = extract_annex_b_frames(&data);
    // Should return an error because parse_avc fails on invalid data
    assert!(frames.is_err());
}

// ============================================================================
// extract_frame_at_index Tests
// ============================================================================

#[test]
fn test_extract_at_index_empty_stream() {
    let data: &[u8] = &[];
    let frame = extract_frame_at_index(data, 0);
    assert!(frame.is_none());
}

#[test]
fn test_extract_at_index_out_of_bounds() {
    // Invalid H.264 data that won't produce frames
    let data = vec![0xFF, 0xFF, 0xFF];
    let frame = extract_frame_at_index(&data, 0);
    assert!(frame.is_none());

    let frame = extract_frame_at_index(&data, 100);
    assert!(frame.is_none());
}

#[test]
fn test_extract_at_index_zero() {
    // Request first frame from empty data
    let data: &[u8] = &[];
    let frame = extract_frame_at_index(data, 0);
    assert!(frame.is_none());
}

#[test]
fn test_extract_at_index_negative_not_possible() {
    // usize cannot be negative, so this test validates type safety
    // The function signature ensures only valid indices can be passed
    let data: &[u8] = &[];
    let _frame = extract_frame_at_index(data, 0); // Compiles and runs
}

// ============================================================================
// avc_frame_to_unit_node Tests
// ============================================================================

#[test]
fn test_frame_to_unit_node_basic_fields() {
    let frame = AvcFrame::builder()
        .frame_index(5)
        .frame_type(AvcFrameType::I)
        .nal_data(vec![0x00, 0x00, 0x01])
        .offset(1000)
        .size(500)
        .poc(42)
        .frame_num(7)
        .is_idr(true)
        .is_ref(true)
        .build()
        .unwrap();

    let unit_node = avc_frame_to_unit_node(&frame, 0);

    assert_eq!(unit_node.offset, 1000);
    assert_eq!(unit_node.size, 500);
    assert_eq!(unit_node.frame_index, Some(5));
    assert_eq!(unit_node.pts, Some(42));
    assert!(unit_node.dts.is_none());
    assert!(unit_node.qp_avg.is_none()); // No slice header
    assert!(unit_node.mv_grid.is_none());
    assert!(unit_node.temporal_id.is_none());
}

#[test]
fn test_frame_to_unit_node_frame_types() {
    let test_cases = vec![
        (AvcFrameType::I, "I"),
        (AvcFrameType::P, "P"),
        (AvcFrameType::B, "B"),
        (AvcFrameType::SI, "SI"),
        (AvcFrameType::SP, "SP"),
        (AvcFrameType::Unknown, "Unknown"),
    ];

    for (frame_type, expected_str) in test_cases {
        let frame = AvcFrame::builder()
            .frame_index(0)
            .frame_type(frame_type)
            .offset(0)
            .size(100)
            .poc(0)
            .frame_num(0)
            .is_idr(false)
            .is_ref(false)
            .build()
            .unwrap();

        let unit_node = avc_frame_to_unit_node(&frame, 0);
        assert_eq!(
            unit_node.frame_type.as_ref().map(|s| s.as_ref()),
            Some(expected_str)
        );
    }
}

#[test]
fn test_frame_to_unit_node_display_name() {
    let frame = AvcFrame::builder()
        .frame_index(10)
        .frame_type(AvcFrameType::P)
        .offset(0)
        .size(100)
        .poc(5)
        .frame_num(2)
        .is_idr(false)
        .is_ref(true)
        .build()
        .unwrap();

    let unit_node = avc_frame_to_unit_node(&frame, 0);
    let display_name = unit_node.display_name.as_ref();

    assert!(display_name.contains("10"));
    assert!(display_name.contains("P"));
    assert!(display_name.contains("Frame"));
}

#[test]
fn test_frame_to_unit_node_key_fields() {
    let frame = AvcFrame::builder()
        .frame_index(0)
        .frame_type(AvcFrameType::I)
        .offset(12345)
        .size(6789)
        .poc(0)
        .frame_num(0)
        .is_idr(true)
        .is_ref(true)
        .build()
        .unwrap();

    let unit_node = avc_frame_to_unit_node(&frame, 0);

    assert_eq!(unit_node.key.offset, 12345);
    assert_eq!(unit_node.key.size, 6789);
    assert_eq!(unit_node.unit_type.as_ref(), "FRAME");
    assert_eq!(unit_node.key.unit_type, "FRAME");
}

#[test]
fn test_frame_to_unit_node_children_empty() {
    let frame = AvcFrame::builder()
        .frame_index(0)
        .frame_type(AvcFrameType::I)
        .offset(0)
        .size(100)
        .poc(0)
        .frame_num(0)
        .is_idr(true)
        .is_ref(true)
        .build()
        .unwrap();

    let unit_node = avc_frame_to_unit_node(&frame, 0);
    assert!(unit_node.children.is_empty());
}

#[test]
fn test_frame_to_unit_node_ref_frames_none() {
    let frame = AvcFrame::builder()
        .frame_index(0)
        .frame_type(AvcFrameType::I)
        .offset(0)
        .size(100)
        .poc(0)
        .frame_num(0)
        .is_idr(true)
        .is_ref(true)
        .build()
        .unwrap();

    let unit_node = avc_frame_to_unit_node(&frame, 0);
    assert!(unit_node.ref_frames.is_none());
    assert!(unit_node.ref_slots.is_none());
}

// ============================================================================
// avc_frames_to_unit_nodes Tests
// ============================================================================

#[test]
fn test_frames_to_unit_nodes_empty() {
    let frames: Vec<AvcFrame> = vec![];
    let unit_nodes = avc_frames_to_unit_nodes(&frames);
    assert!(unit_nodes.is_empty());
}

#[test]
fn test_frames_to_unit_nodes_single() {
    let frame = AvcFrame::builder()
        .frame_index(0)
        .frame_type(AvcFrameType::I)
        .offset(0)
        .size(100)
        .poc(0)
        .frame_num(0)
        .is_idr(true)
        .is_ref(true)
        .build()
        .unwrap();

    let unit_nodes = avc_frames_to_unit_nodes(&[frame.clone()]);
    assert_eq!(unit_nodes.len(), 1);
    assert_eq!(unit_nodes[0].frame_index, Some(0));
}

#[test]
fn test_frames_to_unit_nodes_multiple() {
    let frames = vec![
        AvcFrame::builder()
            .frame_index(0)
            .frame_type(AvcFrameType::I)
            .offset(0)
            .size(100)
            .poc(0)
            .frame_num(0)
            .is_idr(true)
            .is_ref(true)
            .build()
            .unwrap(),
        AvcFrame::builder()
            .frame_index(1)
            .frame_type(AvcFrameType::P)
            .offset(100)
            .size(150)
            .poc(2)
            .frame_num(1)
            .is_idr(false)
            .is_ref(true)
            .build()
            .unwrap(),
        AvcFrame::builder()
            .frame_index(2)
            .frame_type(AvcFrameType::B)
            .offset(250)
            .size(200)
            .poc(1)
            .frame_num(1)
            .is_idr(false)
            .is_ref(false)
            .build()
            .unwrap(),
    ];

    let unit_nodes = avc_frames_to_unit_nodes(&frames);
    assert_eq!(unit_nodes.len(), 3);

    // Verify frame indices are preserved
    assert_eq!(unit_nodes[0].frame_index, Some(0));
    assert_eq!(unit_nodes[1].frame_index, Some(1));
    assert_eq!(unit_nodes[2].frame_index, Some(2));

    // Verify offsets are preserved
    assert_eq!(unit_nodes[0].offset, 0);
    assert_eq!(unit_nodes[1].offset, 100);
    assert_eq!(unit_nodes[2].offset, 250);
}

#[test]
fn test_frames_to_unit_nodes_all_frame_types() {
    let frames = vec![
        AvcFrame::builder()
            .frame_index(0)
            .frame_type(AvcFrameType::I)
            .offset(0)
            .size(100)
            .poc(0)
            .frame_num(0)
            .is_idr(true)
            .is_ref(true)
            .build()
            .unwrap(),
        AvcFrame::builder()
            .frame_index(1)
            .frame_type(AvcFrameType::P)
            .offset(100)
            .size(100)
            .poc(1)
            .frame_num(1)
            .is_idr(false)
            .is_ref(true)
            .build()
            .unwrap(),
        AvcFrame::builder()
            .frame_index(2)
            .frame_type(AvcFrameType::B)
            .offset(200)
            .size(100)
            .poc(0)
            .frame_num(1)
            .is_idr(false)
            .is_ref(false)
            .build()
            .unwrap(),
    ];

    let unit_nodes = avc_frames_to_unit_nodes(&frames);

    assert_eq!(
        unit_nodes[0].frame_type.as_ref().map(|s| s.as_ref()),
        Some("I")
    );
    assert_eq!(
        unit_nodes[1].frame_type.as_ref().map(|s| s.as_ref()),
        Some("P")
    );
    assert_eq!(
        unit_nodes[2].frame_type.as_ref().map(|s| s.as_ref()),
        Some("B")
    );
}

// ============================================================================
// Edge Cases and Error Handling
// ============================================================================

#[test]
fn test_builder_large_values() {
    // Test with large values to ensure no overflow issues
    let frame = AvcFrame::builder()
        .frame_index(usize::MAX)
        .frame_type(AvcFrameType::I)
        .offset(usize::MAX)
        .size(usize::MAX)
        .poc(i32::MAX)
        .frame_num(u32::MAX)
        .is_idr(true)
        .is_ref(true)
        .build()
        .unwrap();

    assert_eq!(frame.frame_index, usize::MAX);
    assert_eq!(frame.offset, usize::MAX);
    assert_eq!(frame.size, usize::MAX);
    assert_eq!(frame.poc, i32::MAX);
    assert_eq!(frame.frame_num, u32::MAX);
}

#[test]
fn test_builder_negative_poc_i32_min() {
    // Test minimum i32 value for POC
    let frame = AvcFrame::builder()
        .frame_index(0)
        .frame_type(AvcFrameType::B)
        .offset(0)
        .size(100)
        .poc(i32::MIN)
        .frame_num(0)
        .is_idr(false)
        .is_ref(false)
        .build()
        .unwrap();

    assert_eq!(frame.poc, i32::MIN);
}

#[test]
fn test_all_frame_types_equal() {
    // Test PartialEq for all frame type pairs
    assert_eq!(AvcFrameType::I, AvcFrameType::I);
    assert_eq!(AvcFrameType::P, AvcFrameType::P);
    assert_eq!(AvcFrameType::B, AvcFrameType::B);
    assert_eq!(AvcFrameType::SI, AvcFrameType::SI);
    assert_eq!(AvcFrameType::SP, AvcFrameType::SP);
    assert_eq!(AvcFrameType::Unknown, AvcFrameType::Unknown);
}

#[test]
fn test_all_frame_types_not_equal() {
    // Test that different types are not equal
    assert_ne!(AvcFrameType::I, AvcFrameType::P);
    assert_ne!(AvcFrameType::P, AvcFrameType::B);
    assert_ne!(AvcFrameType::B, AvcFrameType::SI);
    assert_ne!(AvcFrameType::SI, AvcFrameType::SP);
    assert_ne!(AvcFrameType::SP, AvcFrameType::Unknown);
}

#[test]
fn test_stream_id_parameter_ignored() {
    // The _stream_id parameter is currently ignored but part of the API
    let frame = AvcFrame::builder()
        .frame_index(0)
        .frame_type(AvcFrameType::I)
        .offset(0)
        .size(100)
        .poc(0)
        .frame_num(0)
        .is_idr(true)
        .is_ref(true)
        .build()
        .unwrap();

    // Pass different stream_id values, should produce same result
    let node1 = avc_frame_to_unit_node(&frame, 0);
    let node2 = avc_frame_to_unit_node(&frame, 99);

    // The key.stream should always be A (hardcoded)
    assert_eq!(node1.key.stream, bitvue_core::StreamId::A);
    assert_eq!(node2.key.stream, bitvue_core::StreamId::A);
}
