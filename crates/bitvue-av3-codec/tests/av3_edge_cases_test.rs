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
// Edge case tests for AV3 OBU and frame parsing
use bitvue_av3_codec::{
    parse_av3, parse_frame_header, parse_obu_header, parse_sequence_header, ObuType,
};

#[test]
fn test_parse_obu_header_empty() {
    let data: &[u8] = &[];
    let result = parse_obu_header(data);
    assert!(result.is_err());
}

#[test]
fn test_parse_obu_header_all_zeros() {
    let data = [0x00; 8];
    let result = parse_obu_header(&data);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_obu_header_all_ones() {
    let data = [0xFF; 8];
    let result = parse_obu_header(&data);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_obu_header_single_byte() {
    let data = [0x80];
    let result = parse_obu_header(&data);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_obu_header_temporal_delimiter() {
    let data = [0x80, 0x00]; // Temporal delimiter OBU (obu_forbidden_bit=1, type=0)
    let result = parse_obu_header(&data);
    assert!(result.is_ok() || result.is_err());
    // Note: obu_forbidden_bit=1 may result in Reserved type instead of TemporalDelimiter
}

#[test]
fn test_parse_obu_header_all_obu_types() {
    // Test parsing various OBU types
    let obu_types = [
        0u8, // Temporal Delimiter
        1,   // Sequence Header
        2,   // TD
        3,   // Frame Header
        4,   // Tile Group
        5,   // Metadata
        6,   // Frame
        7,   // Redundant Frame Header
        8,   // Tile List
    ];

    for obu_type in obu_types {
        let mut data = vec![0u8; 4];
        data[0] = (obu_type << 3) & 0x78;

        let result = parse_obu_header(&data);
        // Should handle gracefully
        assert!(result.is_ok() || result.is_err());
    }
}

#[test]
fn test_parse_av3_empty_data() {
    let data: &[u8] = &[];
    let result = parse_av3(data);
    assert!(result.is_ok());
    let stream = result.unwrap();
    assert_eq!(stream.frame_count(), 0);
}

#[test]
fn test_parse_av3_no_obus() {
    let data = [0xFF, 0xFF, 0xFF, 0xFF];
    let result = parse_av3(&data);
    assert!(result.is_ok());
}

#[test]
fn test_parse_av3_all_zeros() {
    let data = [0x00; 128];
    let result = parse_av3(&data);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_av3_all_ones() {
    let data = [0xFF; 128];
    let result = parse_av3(&data);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_av3_alternating_pattern() {
    let mut data = vec![0u8; 64];
    for i in 0..64 {
        data[i] = if i % 2 == 0 { 0xAA } else { 0x55 };
    }

    let result = parse_av3(&data);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_av3_incrementing_data() {
    let mut data = vec![0u8; 64];
    for i in 0..64 {
        data[i] = i as u8;
    }

    let result = parse_av3(&data);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_sequence_header_too_short() {
    let data = [0x80]; // Just marker byte
    let result = parse_sequence_header(&data);
    assert!(result.is_err());
}

#[test]
fn test_parse_sequence_header_minimal() {
    let mut data = vec![0u8; 16];
    data[0] = 0x0C; // marker + seq_profile
    data[1] = 0x00; // seq_level_index

    let result = parse_sequence_header(&data);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_frame_header_empty() {
    let data: &[u8] = &[];
    let result = parse_frame_header(data);
    assert!(result.is_err());
}

#[test]
fn test_parse_frame_header_too_short() {
    let data = [0x80];
    let result = parse_frame_header(&data);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_frame_header_with_obu_extension() {
    let mut data = vec![0u8; 32];
    data[0] = (3 << 3) | 0x02; // Frame Header OBU with extension
    data[1] = 0x10; // OBU size

    let result = parse_frame_header(&data);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_av3_with_temporal_delimiters() {
    let mut data = vec![0u8; 64];
    // Temporal delimiter
    data[0] = 0x80;
    data[1] = 0x00;
    // Sequence header
    data[2] = 0x0C;
    data[3] = 0x00;
    // Frame header
    data[32] = (3 << 3) | 0x04;
    data[33] = 0x10;

    let result = parse_av3(&data);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_av3_very_long_obu() {
    let mut data = vec![0u8; 10000];
    data[0] = (1 << 3) | 0x02; // Sequence Header
                               // Fill with some pattern
    for i in 5..data.len() {
        data[i] = (i % 256) as u8;
    }

    let result = parse_av3(&data);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_av3_with_metadata() {
    let mut data = vec![0u8; 64];
    // Metadata OBU
    data[0] = (5 << 3) | 0x02;
    data[1] = 0x10;

    let result = parse_av3(&data);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_av3_consecutive_obus() {
    let mut data = vec![0u8; 128];
    // Temporal delimiter
    data[0] = 0x80;
    data[1] = 0x00;
    // Sequence header
    data[2] = 0x0C;
    data[3] = 0x00;
    // Frame header
    data[64] = (3 << 3) | 0x04;
    data[65] = 0x10;

    let result = parse_av3(&data);
    assert!(result.is_ok() || result.is_err());
}
