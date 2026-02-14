//! Tests for HEVC lib.rs public API.
//! Targeting 95%+ line coverage for HevcStream methods.

use bitvue_hevc::{HevcStream, parse_hevc, Result};

// ============================================================================
// Helper Functions
// ============================================================================

/// Create minimal valid HEVC bitstream data
fn create_minimal_bitstream() -> Vec<u8> {
    vec![
        0x00, 0x00, 0x01,  // Start code
        0x40, 0x01,         // VPS NAL header (type 32)
        0x42, 0x01,         // SPS NAL header (type 33)
    ]
}

// ============================================================================
// parse_hevc Tests
// ============================================================================

#[test]
fn test_parse_hevc_empty_stream() {
    let data = &[];
    let result = parse_hevc(data);

    assert!(result.is_ok());
    let stream = result.unwrap();

    assert!(stream.nal_units.is_empty());
    assert!(stream.vps_map.is_empty());
    assert!(stream.sps_map.is_empty());
    assert!(stream.pps_map.is_empty());
    assert!(stream.slices.is_empty());
}

#[test]
fn test_parse_hevc_with_vps_sps() {
    let data = create_minimal_bitstream();
    let result = parse_hevc(&data);

    assert!(result.is_ok());
    let stream = result.unwrap();

    // Should have parsed some NAL units (VPS and/or SPS)
    assert!(!stream.nal_units.is_empty());
}

#[test]
fn test_parse_hevc_frame_count() {
    let data = create_minimal_bitstream();
    let result = parse_hevc(&data);

    assert!(result.is_ok());
    let stream = result.unwrap();

    // frame_count should return 0 (no VCL NALs in minimal stream)
    assert_eq!(stream.frame_count(), 0);
}

#[test]
fn test_parse_hevc_idr_frames() {
    let data = create_minimal_bitstream();
    let result = parse_hevc(&data);

    assert!(result.is_ok());
    let stream = result.unwrap();

    // idr_frames should return 0 (no IDR frames in minimal stream)
    assert_eq!(stream.idr_frames().len(), 0);
}

#[test]
fn test_parse_hevc_irap_frames() {
    let data = create_minimal_bitstream();
    let result = parse_hevc(&data);

    assert!(result.is_ok());
    let stream = result.unwrap();

    // irap_frames should return 0 (no IRAP frames in minimal stream)
    assert_eq!(stream.irap_frames().len(), 0);
}

#[test]
fn test_parse_hevc_bit_depth() {
    let data = create_minimal_bitstream();
    let result = parse_hevc(&data);

    assert!(result.is_ok());
    let stream = result.unwrap();

    // Test bit_depth methods
    assert!(stream.bit_depth_luma().is_none()); // No SPS with bit depth info
    assert!(stream.bit_depth_chroma().is_none());
}

#[test]
fn test_parse_hevc_chroma_format() {
    let data = create_minimal_bitstream();
    let result = parse_hevc(&data);

    assert!(result.is_ok());
    let stream = result.unwrap();

    // Test chroma_format method
    assert!(stream.chroma_format().is_none()); // No SPS with chroma format info
}

// ============================================================================
// HevcStream Method Tests
// ============================================================================

#[test]
fn test_hevc_stream_get_vps() {
    let data = create_minimal_bitstream();
    let result = parse_hevc(&data);

    assert!(result.is_ok());
    let stream = result.unwrap();

    let vps_id = 0u8;

    // Test get_vps with existing ID (if VPS was parsed)
    if stream.vps_map.contains_key(&vps_id) {
        assert!(stream.get_vps(vps_id).is_some());
    }

    // Test get_vps with non-existent ID
    assert!(stream.get_vps(99u8).is_none());
}

#[test]
fn test_hevc_stream_get_sps() {
    let data = create_minimal_bitstream();
    let result = parse_hevc(&data);

    assert!(result.is_ok());
    let stream = result.unwrap();

    let sps_id = 0u8;

    // Test get_sps with existing ID (if SPS was parsed)
    if stream.sps_map.contains_key(&sps_id) {
        assert!(stream.get_sps(sps_id).is_some());
    }

    // Test get_sps with non-existent ID
    assert!(stream.get_sps(99u8).is_none());
}

#[test]
fn test_hevc_stream_get_pps() {
    let data = create_minimal_bitstream();
    let result = parse_hevc(&data);

    assert!(result.is_ok());
    let stream = result.unwrap();

    let pps_id = 0u8;

    // Test get_pps with existing ID (if PPS was parsed)
    if stream.pps_map.contains_key(&pps_id) {
        assert!(stream.get_pps(pps_id).is_some());
    }

    // Test get_pps with non-existent ID
    assert!(stream.get_pps(99u8).is_none());
}

#[test]
fn test_hevc_stream_dimensions() {
    let data = create_minimal_bitstream();
    let result = parse_hevc(&data);

    assert!(result.is_ok());
    let stream = result.unwrap();

    // Test dimensions with SPS
    assert!(stream.dimensions().is_none());
}

#[test]
fn test_hevc_stream_frame_rate() {
    let data = create_minimal_bitstream();
    let result = parse_hevc(&data);

    assert!(result.is_ok());
    let stream = result.unwrap();

    // Test frame_rate method (should be None without timing info)
    assert!(stream.frame_rate().is_none());
}

#[test]
fn test_hevc_stream_bit_depth() {
    let data = create_minimal_bitstream();
    let result = parse_hevc(&data);

    assert!(result.is_ok());
    let stream = result.unwrap();

    // Test bit depth methods (should be None without SPS)
    assert!(stream.bit_depth_luma().is_none());
    assert!(stream.bit_depth_chroma().is_none());
}

#[test]
fn test_hevc_stream_chroma_format() {
    let data = create_minimal_bitstream();
    let result = parse_hevc(&data);

    assert!(result.is_ok());
    let stream = result.unwrap();

    // Test chroma_format method (should be None without SPS)
    assert!(stream.chroma_format().is_none());
}

#[test]
fn test_hevc_stream_frame_count() {
    let data = create_minimal_bitstream();
    let result = parse_hevc(&data);

    assert!(result.is_ok());
    let stream = result.unwrap();

    // Test frame_count (should be 0 - no VCL NALs in test data)
    assert_eq!(stream.frame_count(), 0);
}

#[test]
fn test_hevc_stream_idr_frames() {
    let data = create_minimal_bitstream();
    let result = parse_hevc(&data);

    assert!(result.is_ok());
    let stream = result.unwrap();

    // Test idr_frames (should be 0 - no IDR frames in test data)
    assert_eq!(stream.idr_frames().len(), 0);
}

#[test]
fn test_hevc_stream_irap_frames() {
    let data = create_minimal_bitstream();
    let result = parse_hevc(&data);

    assert!(result.is_ok());
    let stream = result.unwrap();

    // Test irap_frames (should be 0 - no IRAP frames in test data)
    assert_eq!(stream.irap_frames().len(), 0);
}

#[test]
fn test_hevc_stream_count_methods() {
    let data = create_minimal_bitstream();
    let result = parse_hevc(&data);

    assert!(result.is_ok());
    let stream = result.unwrap();

    // Test various count methods
    assert_eq!(stream.slices.len(), 0); // No VCL NALs
}
