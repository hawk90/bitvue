#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(unused_comparisons)]
#![allow(unused_doc_comments)]
//! Comprehensive tests for HEVC frames extraction functions.
//! Targeting 95%+ line coverage for extract_annex_b_frames and extract_frame_at_index.

use bitvue_hevc::frames::{
    extract_annex_b_frames, extract_frame_at_index, HevcFrame, HevcFrameType,
};

// ============================================================================
// Helper Functions
// ============================================================================

/// Create minimal valid HEVC bitstream data
fn create_minimal_bitstream() -> Vec<u8> {
    vec![
        0x00, 0x00, 0x01, // Start code (3 bytes)
        0xFF, 0xFF, 0x01, // VPS NAL (2 bytes)
        0x00, 0x01, 0x02, // Extra data
    ]
}

/// Create a simple frame for testing
fn create_test_frame() -> HevcFrame {
    HevcFrame::builder()
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
        .build()
}

#[test]
fn test_extract_annex_b_frames_empty_stream() {
    let data = &[];
    let result = extract_annex_b_frames(data);

    assert!(result.is_ok());
    assert!(result.unwrap().is_empty());
}

#[test]
fn test_extract_annex_b_frames_single_vcl_nal() {
    // Single VCL NAL unit without SPS/PPS data
    // 0x00 0x00 0x01 = start code
    // 0x4C = NAL header (forbidden=0, type=19/IDR_W_RADL, layer=0, temporal=1)
    let data = &[0x00, 0x00, 0x01, 0x4C, 0x01, 0xFF, 0xFF];

    let result = extract_annex_b_frames(data);

    assert!(result.is_ok());
    let frames = result.unwrap();

    // No frames because SPS/PPS are required for slice header parsing
    // Without SPS/PPS, the function returns empty Vec
    assert_eq!(frames.len(), 0);
}

#[test]
fn test_extract_annex_b_frames_two_nals() {
    // Two VCL NAL units without SPS/PPS data
    let data = &[
        0x00, 0x00, 0x01, 0x4C, 0x01, 0xFF, 0xFF, 0x00, 0x00, 0x01, 0x4C, 0x01, 0xAA, 0xBB,
    ];

    let result = extract_annex_b_frames(data);

    assert!(result.is_ok());
    let frames = result.unwrap();

    // No frames because SPS/PPS are required for slice header parsing
    assert_eq!(frames.len(), 0);
}

#[test]
fn test_extract_frame_at_index_empty_stream() {
    let data = &[];
    let result = extract_frame_at_index(data, 0);

    // Empty stream returns Ok with empty Vec, so no frame at index 0
    assert!(result.is_none());
}

#[test]
fn test_extract_frame_at_index_single_frame() {
    // Single VCL NAL frame without SPS/PPS data
    let data = &[
        0x00, 0x00, 0x01, 0x4C, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B,
        0x0C, 0x0D, 0x0E, 0x0F,
    ];

    let result = extract_frame_at_index(data, 0);

    // No frames because SPS/PPS are required
    assert!(result.is_none());
}

#[test]
fn test_extract_frame_at_index_second_frame() {
    // Two VCL NAL frames without SPS/PPS
    let data = &[
        0x00, 0x00, 0x01, 0x4C, 0x01, 0xFF, 0xFF, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09,
        0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F, 0x10, 0x00, 0x00, 0x01, 0x4C, 0x01, 0xAA, 0xBB,
    ];

    // Request second frame (index 1)
    let result = extract_frame_at_index(data, 1);

    // No frames because SPS/PPS are required
    assert!(result.is_none());
}

#[test]
fn test_extract_frame_at_index_out_of_bounds() {
    // Request non-existent frame index
    let data = &[0x00, 0x00, 0x01, 0x4C, 0x01];
    let result = extract_frame_at_index(data, 999);

    // Index out of bounds returns None
    assert!(result.is_none());
}

#[test]
fn test_extract_frame_at_index_large_index() {
    // Very large index should also return None
    let data = &[0x00, 0x00, 0x01];
    let result = extract_frame_at_index(data, usize::MAX);

    assert!(result.is_none());
}

// ============================================================================
// HevcFrameType Tests
// ============================================================================

#[test]
fn test_hevc_frame_type_display() {
    assert_eq!(HevcFrameType::I.as_str(), "I");
    assert_eq!(HevcFrameType::P.as_str(), "P");
    assert_eq!(HevcFrameType::B.as_str(), "B");
    assert_eq!(HevcFrameType::Unknown.as_str(), "Unknown");
}

// ============================================================================
// Error Handling Tests
// ============================================================================

#[test]
fn test_extract_annex_b_frames_error_empty() {
    let data = &[];
    let result = extract_annex_b_frames(data);

    // Empty stream returns empty Vec, not error
    assert!(result.is_ok());
    assert!(result.unwrap().is_empty());
}

#[test]
fn test_extract_annex_b_frames_invalid_nal_start_code() {
    // Use invalid NAL start code (not 0x00 or 0x67)
    let data = &[0x99]; // Invalid NAL
    let result = extract_annex_b_frames(data);

    assert!(result.is_ok());
    assert!(result.unwrap().is_empty());
}

#[test]
fn test_extract_annex_b_frames_missing_vcl_nal() {
    // Non-VCL NAL (VPS = 32/0x20)
    let data = &[0x67, 0x68]; // VPS NAL without start code
    let result = extract_annex_b_frames(data);

    // Non-VCL NAL without start codes returns empty Vec
    assert!(result.is_ok());
    assert!(result.unwrap().is_empty());
}

#[test]
fn test_extract_frame_at_index_empty_stream_no_frame() {
    let data = &[];
    let result = extract_frame_at_index(data, 0);

    // Empty stream returns Ok with empty Vec, so no frame at index 0
    assert!(result.is_none());
}

#[test]
fn test_extract_frame_at_index_no_nal_units() {
    // Stream without NAL units (no start codes)
    let data = &[0xFF, 0xFF, 0xFF, 0xFF];
    let result = extract_frame_at_index(data, 0);

    // No NAL units means no frames
    assert!(result.is_none());
}

#[test]
fn test_extract_frame_at_index_with_valid_nal() {
    // Valid VCL NAL with start code (IDR_W_RADL = 19)
    // 0x00 0x00 0x01 = start code
    // 0x4C 0x01 = NAL header (forbidden=0, type=19/IDR_W_RADL, layer=0, temporal=1)
    let data = &[0x00, 0x00, 0x01, 0x4C, 0x01];
    let result = extract_frame_at_index(data, 0);

    // No frame because SPS/PPS are required for slice header parsing
    assert!(result.is_none());
}
