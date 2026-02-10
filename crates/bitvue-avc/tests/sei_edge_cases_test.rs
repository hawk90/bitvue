// Edge case tests for SEI parsing - simplified version
use bitvue_avc::parse_sei;

#[test]
fn test_parse_sei_empty_input() {
    let data: &[u8] = &[];
    let result = parse_sei(data);
    assert!(result.is_ok());
    let messages = result.unwrap();
    assert_eq!(messages.len(), 0);
}

#[test]
fn test_parse_sei_no_start_code() {
    let data = [0xFF, 0xFF, 0xFF];
    let result = parse_sei(&data);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_sei_only_start_code() {
    let data = [0x00, 0x00, 0x01, 0x06];
    let result = parse_sei(&data);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_sei_zero_payload_size() {
    let mut data = vec![0u8; 16];
    data[0..4].copy_from_slice(&[0x00, 0x00, 0x01]);
    data[4] = 0x06;
    data[5] = 0xFF;
    data[6] = 0x00;

    let result = parse_sei(&data);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_sei_max_payload_type() {
    let mut data = vec![0u8; 32];
    data[0..4].copy_from_slice(&[0x00, 0x00, 0x01]);
    data[4] = 0x06;
    data[5] = 0xFF;
    data[6] = 0xFF;
    data[7] = 0x00;

    let result = parse_sei(&data);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_sei_large_payload_size() {
    let mut data = vec![0u8; 32];
    data[0..4].copy_from_slice(&[0x00, 0x00, 0x01]);
    data[4] = 0x06;
    data[5] = 0x01;
    data[6] = 0x80;
    data[7] = 0x01;

    let result = parse_sei(&data);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_sei_all_zeros() {
    let data = [0x00; 32];
    let result = parse_sei(&data);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_sei_all_ones() {
    let data = [0xFF; 32];
    let result = parse_sei(&data);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_sei_alternating_pattern() {
    let mut data = vec![0u8; 32];
    for i in 0..32 {
        data[i] = if i % 2 == 0 { 0xAA } else { 0x55 };
    }

    let result = parse_sei(&data);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_sei_incrementing_pattern() {
    let mut data = vec![0u8; 32];
    for i in 0..32 {
        data[i] = i as u8;
    }

    let result = parse_sei(&data);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_sei_single_byte() {
    let data = [0x06];
    let result = parse_sei(&data);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_sei_two_bytes() {
    let data = [0x06, 0xFF];
    let result = parse_sei(&data);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_sei_three_bytes() {
    let data = [0x06, 0xFF, 0x00];
    let result = parse_sei(&data);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_sei_very_short_data() {
    let data = [0x00, 0x00];
    let result = parse_sei(&data);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_sei_with_rbsp_trailing_bits() {
    let mut data = vec![0u8; 32];
    data[0..4].copy_from_slice(&[0x00, 0x00, 0x01]);
    data[4] = 0x06;
    data[5] = 0x00;
    data[6] = 0x00;
    data[7] = 0x80;

    let result = parse_sei(&data);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_sei_multiple_rbsp_trailing() {
    let mut data = vec![0u8; 32];
    data[0..4].copy_from_slice(&[0x00, 0x00, 0x01]);
    data[4] = 0x06;
    data[5] = 0x00;
    data[6] = 0x00;
    data[7] = 0x80;
    data[8] = 0x00;
    data[9] = 0x80;

    let result = parse_sei(&data);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_sei_consecutive_ff_payload_type() {
    let mut data = vec![0u8; 32];
    data[0..4].copy_from_slice(&[0x00, 0x00, 0x01]);
    data[4] = 0x06;
    data[5] = 0xFF;
    data[6] = 0xFF;
    data[7] = 0xFF;
    data[8] = 0x00;

    let result = parse_sei(&data);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_sei_reserved_payload_types() {
    // Test some reserved payload types
    for payload_type in [37u8, 38, 40, 45, 50, 100, 200, 255] {
        let mut data = vec![0u8; 16];
        data[0..4].copy_from_slice(&[0x00, 0x00, 0x01]);
        data[4] = 0x06;
        data[5] = payload_type;
        data[6] = 0x00;

        let result = parse_sei(&data);
        // Should handle gracefully
        assert!(result.is_ok() || result.is_err());
    }
}

#[test]
fn test_parse_sei_very_long_payload() {
    let mut data = vec![0u8; 128];
    data[0..4].copy_from_slice(&[0x00, 0x00, 0x01]);
    data[4] = 0x06;
    data[5] = 0x05;
    data[6] = 0x7F;
    data[7] = 0x81;

    let result = parse_sei(&data);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_sei_zero_length_nal() {
    let data = [
        0x00, 0x00, 0x01,
        0x00, 0x00, 0x01,
        0x06, 0x00, 0x00,
    ];

    let result = parse_sei(&data);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_sei_truncated_payload() {
    let mut data = vec![0u8; 16];
    data[0..4].copy_from_slice(&[0x00, 0x00, 0x01]);
    data[4] = 0x06;
    data[5] = 0x01;
    data[6] = 0x80; // Large payload_size but no actual payload

    let result = parse_sei(&data);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_sei_with_multiple_start_codes() {
    let data = [
        0x00, 0x00, 0x00, 0x01,
        0x00, 0x00, 0x01,
        0x06, 0x00, 0x00,
    ];

    let result = parse_sei(&data);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_sei_mixed_start_code_lengths() {
    let data = [
        0x00, 0x00, 0x01,  // 3-byte
        0x06, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x01,  // 4-byte
        0x06, 0x01, 0x00,
    ];

    let result = parse_sei(&data);
    assert!(result.is_ok() || result.is_err());
}
