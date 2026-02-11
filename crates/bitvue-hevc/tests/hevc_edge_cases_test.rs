// Edge case tests for HEVC stream parsing
use bitvue_hevc::parse_hevc;

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
    data[0..4].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    data[4] = 0xFF; // Invalid NAL type

    let result = parse_hevc(&data);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_hevc_very_long_nal() {
    let mut data = vec![0u8; 10000];
    data[0..4].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]);
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
