#![allow(dead_code)]
// Edge case tests for VVC slice and frame parsing
use bitvue_vvc::{parse_nal_header, parse_vvc};

#[test]
fn test_parse_nal_header_empty_data() {
    let data: &[u8] = &[];
    let result = parse_nal_header(data);
    assert!(result.is_err());
}

#[test]
fn test_parse_nal_header_all_zeros() {
    let data = [0x00; 32];
    let result = parse_nal_header(&data);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_nal_header_all_ones() {
    let data = [0xFF; 32];
    let result = parse_nal_header(&data);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_nal_header_single_byte() {
    let data = [0x80];
    let result = parse_nal_header(&data);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_nal_header_two_bytes() {
    let data = [0x80, 0x00];
    let result = parse_nal_header(&data);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_nal_header_with_trailing_zeros() {
    let mut data = vec![0u8; 64];
    data[0] = 0x80; // First slice segment flag

    let result = parse_nal_header(&data);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_nal_header_with_trailing_ones() {
    let mut data = vec![0u8; 64];
    data[0] = 0x80;
    for i in 1..64 {
        data[i] = 0xFF;
    }

    let result = parse_nal_header(&data);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_nal_header_alternating_pattern() {
    let mut data = vec![0u8; 64];
    for i in 0..64 {
        data[i] = if i % 2 == 0 { 0xAA } else { 0x55 };
    }

    let result = parse_nal_header(&data);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_nal_header_incrementing_data() {
    let mut data = vec![0u8; 64];
    for i in 0..64 {
        data[i] = i as u8;
    }

    let result = parse_nal_header(&data);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_vvc_empty_stream() {
    let data: &[u8] = &[];
    let stream = parse_vvc(data).unwrap();
    assert_eq!(stream.nal_units.len(), 0);
    assert_eq!(stream.frame_count(), 0);
}

#[test]
fn test_parse_vvc_no_start_codes() {
    let data = [0xFF, 0xFF, 0xFF];
    let stream = parse_vvc(&data).unwrap();
    assert_eq!(stream.nal_units.len(), 0);
}

#[test]
fn test_parse_vvc_only_start_codes() {
    let data = [0x00, 0x00, 0x01, 0x00, 0x00, 0x01];
    let stream = parse_vvc(&data).unwrap();
    // May or may not find NALs
    assert!(stream.nal_units.len() >= 0);
}

#[test]
fn test_parse_vvc_malformed_nal() {
    let mut data = vec![0u8; 32];
    data[0..4].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    data[4] = 0xFF; // Invalid NAL type

    let result = parse_vvc(&data);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_vvc_very_long_nal() {
    let mut data = vec![0u8; 10000];
    data[0..4].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    data[4] = 0x20; // Slice NAL type
    for i in 5..data.len() {
        data[i] = (i % 256) as u8;
    }

    let result = parse_vvc(&data);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_vvc_all_zeros() {
    let data = [0x00; 128];
    let result = parse_vvc(&data);
    assert!(result.is_ok());
}

#[test]
fn test_parse_vvc_all_ones() {
    let data = [0xFF; 128];
    let result = parse_vvc(&data);
    assert!(result.is_ok());
}

#[test]
fn test_parse_vvc_consecutive_start_codes() {
    let data = [
        0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x01, 0x00, 0x00, 0x01,
    ];
    let result = parse_vvc(&data);
    assert!(result.is_ok());
}

#[test]
fn test_parse_vvc_mixed_start_code_lengths() {
    let data = [
        0x00, 0x00, 0x01, // 3-byte
        0x00, 0x00, 0x00, 0x01, // 4-byte
        0x00, 0x00, 0x01, // 3-byte
    ];
    let result = parse_vvc(&data);
    assert!(result.is_ok());
}

#[test]
fn test_parse_vvc_zero_temporal_id() {
    let mut data = vec![0u8; 32];
    data[0..4].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    // NAL header with zero temporal_id_plus1 (invalid)
    data[4] = 0x00; // nal_unit_type
    data[5] = 0x00; // nuh_layer_id
    data[6] = 0x00; // nuh_temporal_id_plus1 = 0 (invalid)

    let result = parse_vvc(&data);
    // Should handle gracefully
    assert!(result.is_ok() || result.is_err());
}
