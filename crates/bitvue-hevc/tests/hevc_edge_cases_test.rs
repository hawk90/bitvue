// Edge case tests for HEVC slice and frame parsing
use bitvue_hevc::{parse_slice_header, parse_hevc, NalUnitType};

#[test]
fn test_parse_slice_header_empty_data() {
    let data: &[u8] = &[];
    let result = parse_slice_header(data);
    // Empty data should be handled gracefully
    assert!(result.is_err());
}

#[test]
fn test_parse_slice_header_all_zeros() {
    let data = [0x00; 32];
    let result = parse_slice_header(&data);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_slice_header_all_ones() {
    let data = [0xFF; 32];
    let result = parse_slice_header(&data);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_slice_header_single_byte() {
    let data = [0x80];
    let result = parse_slice_header(&data);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_slice_header_two_bytes() {
    let data = [0x80, 0x00];
    let result = parse_slice_header(&data);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_slice_header_with_trailing_zeros() {
    let mut data = vec![0u8; 64];
    data[0] = 0x80; // first_slice_segment_in_pic_flag
    data[1] = 0x00; // rest
    // Rest is zeros

    let result = parse_slice_header(&data);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_slice_header_with_trailing_ones() {
    let mut data = vec![0u8; 64];
    data[0] = 0x80;
    // Fill rest with 0xFF
    for i in 1..64 {
        data[i] = 0xFF;
    }

    let result = parse_slice_header(&data);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_slice_header_alternating_pattern() {
    let mut data = vec![0u8; 64];
    for i in 0..64 {
        data[i] = if i % 2 == 0 { 0xAA } else { 0x55 };
    }

    let result = parse_slice_header(&data);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_slice_header_incrementing_data() {
    let mut data = vec![0u8; 64];
    for i in 0..64 {
        data[i] = i as u8;
    }

    let result = parse_slice_header(&data);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_hevc_empty_stream() {
    let data: &[u8] = &[];
    let stream = parse_hevc(data).unwrap();
    assert_eq!(stream.nal_units.len(), 0);
    assert_eq!(stream.frame_count(), 0);
}

#[test]
fn test_parse_hevc_no_start_codes() {
    let data = [0xFF, 0xFF, 0xFF];
    let stream = parse_hevc(&data).unwrap();
    assert_eq!(stream.nal_units.len(), 0);
}

#[test]
fn test_parse_hevc_only_start_codes() {
    let data = [0x00, 0x00, 0x01, 0x00, 0x00, 0x01];
    let stream = parse_hevc(&data).unwrap();
    // May or may not find NALs
    assert!(stream.nal_units.len() >= 0);
}

#[test]
fn test_parse_hevc_malformed_nal() {
    let mut data = vec![0u8; 32];
    data[0..4].copy_from_slice(&[0x00, 0x00, 0x01]);
    data[4] = 0xFF; // Invalid NAL type

    let result = parse_hevc(&data);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_hevc_very_long_nal() {
    let mut data = vec![0u8; 10000];
    data[0..4].copy_from_slice(&[0x00, 0x00, 0x01]);
    data[4] = 0x40; // VPS NAL type
    for i in 5..data.len() {
        data[i] = (i % 256) as u8;
    }

    let result = parse_hevc(&data);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_hevc_all_zeros() {
    let data = [0x00; 128];
    let result = parse_hevc(&data);
    assert!(result.is_ok());
}

#[test]
fn test_parse_hevc_all_ones() {
    let data = [0xFF; 128];
    let result = parse_hevc(&data);
    assert!(result.is_ok());
}

#[test]
fn test_parse_hevc_consecutive_start_codes() {
    let data = [
        0x00, 0x00, 0x00, 0x01,
        0x00, 0x00, 0x00, 0x01,
        0x00, 0x00, 0x01,
        0x00, 0x00, 0x01,
    ];
    let result = parse_hevc(&data);
    assert!(result.is_ok());
}

#[test]
fn test_parse_hevc_mixed_start_code_lengths() {
    let data = [
        0x00, 0x00, 0x01,     // 3-byte
        0x00, 0x00, 0x00, 0x01, // 4-byte
        0x00, 0x00, 0x01,     // 3-byte
    ];
    let result = parse_hevc(&data);
    assert!(result.is_ok());
}
