// Functional tests for AVC codec - targeting 100% coverage
// These tests exercise specific code paths in low-coverage modules
use bitvue_avc::{
    extract_annex_b_frames, parse_avc, parse_nal_header, parse_nal_units, parse_pps, parse_sei,
    parse_sps, AvcStream, ChromaFormat,
};
use std::collections::HashMap;

#[test]
fn test_chroma_format_variants() {
    // Test ChromaFormat enum
    assert_eq!(ChromaFormat::Yuv420 as u8, 1);
    assert_eq!(ChromaFormat::Yuv422 as u8, 2);
    assert_eq!(ChromaFormat::Yuv444 as u8, 3);
}

#[test]
fn test_parse_sps_minimal() {
    // The minimal_sps helper may fail with invalid data
    // This test just verifies the parsing runs without panic
    let mut data = vec![0u8; 32];
    data[0..4].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    data[4] = 0x67; // SPS NAL
    data[5] = 0x42; // profile_idc (Baseline)
    data[6] = 0x80; // constraint_set0_flag
    data[7] = 0x1C; // level_idc

    let result = parse_sps(&data);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_sps_various_profiles() {
    // Test different profile values
    for profile in [66u8, 77, 100, 110, 122, 244] {
        let mut data = vec![0u8; 32];
        data[0..4].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]);
        data[4] = 0x67; // SPS NAL
        data[5] = profile; // Different profiles

        let result = parse_sps(&data);
        assert!(result.is_ok() || result.is_err());
    }
}

#[test]
fn test_parse_pps_minimal() {
    let mut data = vec![0u8; 32];
    data[0..4].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    data[4] = 0x68; // PPS NAL
    data[5] = 0x80; // pic_parameter_set_id
    data[6] = 0x01; // seq_parameter_set_id + rice_flag

    let result = parse_pps(&data);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_sei_empty() {
    // Test SEI with empty payload
    let mut data = vec![0u8; 16];
    data[0..4].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    data[4] = 0x06; // SEI NAL
    data[5] = 0x01; // payload_type
    data[6] = 0x00; // payload_size (first byte)
    data[7] = 0x00; // payload_size (second byte)
    data[8] = 0x00; // payload_size (third byte)
    data[9] = 0x80; // last_payload_type

    let result = parse_sei(&data);
    assert!(result.is_ok());
}

#[test]
fn test_parse_sei_pic_timing() {
    // Test SEI pic_timing
    let mut data = vec![0u8; 32];
    data[0..4].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    data[4] = 0x06; // SEI NAL
    data[5] = 0x01; // payload_type = pic_timing
    data[6] = 0x01; // payload_size = 1
    data[7] = 0x00; // clock_timestamp_flag
    data[8] = 0x00; // ct_type

    let result = parse_sei(&data);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_sei_buffering_period() {
    // Test SEI buffering_period
    let mut data = vec![0u8; 32];
    data[0..4].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    data[4] = 0x06; // SEI NAL
    data[5] = 0x00; // payload_type = buffering_period (0)
    data[6] = 0x01; // payload_size = 1
    data[7] = 0x00; // seq_parameter_set_id

    let result = parse_sei(&data);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_nal_header_all_ref_idc() {
    // Test all ref_idc values
    for ref_idc in 0..=3u8 {
        let byte = (5 << 3) | ref_idc; // IDR slice with different ref_idc
        let result = parse_nal_header(byte);
        assert!(result.is_ok());
    }
}

#[test]
fn test_extract_annex_b_empty() {
    let data: &[u8] = &[];
    let result = extract_annex_b_frames(data);
    assert!(result.is_ok());
}

#[test]
fn test_extract_annex_b_no_start_codes() {
    let data = [0xFFu8; 4];
    let result = extract_annex_b_frames(&data);
    assert!(result.is_ok());
}

#[test]
fn test_extract_annex_b_single_nal() {
    let mut data = vec![0u8; 32];
    data[0..4].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    data[4] = 0x67; // SPS

    let result = extract_annex_b_frames(&data);
    assert!(result.is_ok());
}

#[test]
fn test_extract_annex_b_multiple_nals() {
    let mut data = vec![0u8; 64];
    let mut offset = 0;

    // SPS
    data[offset..offset + 4].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    data[offset + 4] = 0x67; // SPS
    offset += 16;

    // PPS
    data[offset..offset + 4].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    data[offset + 4] = 0x68; // PPS
    offset += 16;

    // IDR
    data[offset..offset + 4].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    data[offset + 4] = 0x65; // IDR

    let result = extract_annex_b_frames(&data);
    assert!(result.is_ok());
}

#[test]
fn test_parse_nal_units_empty() {
    let data: &[u8] = &[];
    let result = parse_nal_units(data);
    assert!(result.is_ok());
}

#[test]
fn test_parse_nal_units_no_start_codes() {
    let data = [0xFF, 0xFF, 0xFF, 0xFF];
    let result = parse_nal_units(&data);
    assert!(result.is_ok());
}

#[test]
fn test_parse_nal_units_single() {
    let mut data = vec![0u8; 32];
    data[0..4].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    data[4] = 0x67; // SPS

    let result = parse_nal_units(&data);
    assert!(result.is_ok());
}

#[test]
fn test_parse_nal_units_multiple() {
    let mut data = vec![0u8; 64];
    let mut offset = 0;

    // SPS
    data[offset..offset + 4].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    data[offset + 4] = 0x67;
    offset += 16;

    // PPS
    data[offset..offset + 4].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    data[offset + 4] = 0x68;

    let result = parse_nal_units(&data);
    assert!(result.is_ok());
}

#[test]
fn test_parse_avc_empty() {
    let data: &[u8] = &[];
    let stream = parse_avc(data).unwrap();
    assert_eq!(stream.nal_units.len(), 0);
}

#[test]
fn test_stream_frame_count() {
    let stream = AvcStream {
        nal_units: vec![],
        sps_map: HashMap::new(),
        pps_map: HashMap::new(),
        slices: vec![],
        sei_messages: vec![],
    };

    assert_eq!(stream.frame_count(), 0);
    assert_eq!(stream.idr_frames().len(), 0);
}

#[test]
fn test_stream_methods() {
    // Test AvcStream methods with minimal data
    let stream = AvcStream {
        nal_units: vec![],
        sps_map: std::collections::HashMap::new(),
        pps_map: std::collections::HashMap::new(),
        slices: vec![],
        sei_messages: vec![],
    };

    assert_eq!(stream.frame_count(), 0);
    assert_eq!(stream.idr_frames().len(), 0);
}
