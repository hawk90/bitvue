#![allow(dead_code)]
//! Comprehensive tests for HEVC NAL unit parsing.
//! Targeting 95%+ line coverage for nal.rs module.

use bitvue_hevc::nal::{find_nal_units, parse_nal_header, parse_nal_units, NalUnitType};

// ============================================================================
// find_nal_units Tests
// ============================================================================

#[test]
fn test_find_nal_units_empty() {
    let data: &[u8] = &[];
    let units = find_nal_units(data);
    assert!(units.is_empty());
}

#[test]
fn test_find_nal_units_no_start_codes() {
    let data: &[u8] = &[0xFF, 0xFF, 0xFF, 0xAA, 0x55];
    let units = find_nal_units(data);
    assert!(units.is_empty());
}

#[test]
fn test_find_nal_units_three_byte_start_code() {
    let data: &[u8] = &[0x00, 0x00, 0x01, 0xFF];
    let units = find_nal_units(data);
    assert_eq!(units.len(), 1);
    assert_eq!(units[0].0, 3);
}

// ============================================================================
// parse_nal_header Tests
// ============================================================================

#[test]
fn test_parse_nal_header_idr_w_radl() {
    // IDR_W_RADL NAL unit type = 19 (from existing test in nal.rs)
    let data: &[u8] = &[0x26, 0x01];
    let result = parse_nal_header(data);
    assert!(result.is_ok());
    let header = result.unwrap();
    assert_eq!(header.nal_unit_type, NalUnitType::IdrWRadl);
    assert_eq!(header.nuh_layer_id, 0);
    assert_eq!(header.nuh_temporal_id_plus1, 1);
}

#[test]
fn test_parse_nal_header_temporal_id() {
    let data: &[u8] = &[0x26, 0x01];
    let result = parse_nal_header(data);
    assert!(result.is_ok());
    let header = result.unwrap();
    assert_eq!(header.nuh_temporal_id_plus1, 1);
    assert_eq!(header.temporal_id(), 0); // temporal_id = plus1 - 1
}

#[test]
fn test_parse_nal_header_insufficient_bytes() {
    // Insufficient bytes for NAL header (needs 2 bytes)
    let data: &[u8] = &[0x26];
    let result = parse_nal_header(data);
    assert!(result.is_err());
}

// ============================================================================
// parse_nal_units Tests
// ============================================================================

#[test]
fn test_parse_nal_units_empty() {
    let data: &[u8] = &[];
    let result = parse_nal_units(data);
    assert!(result.is_ok());
    let units = result.unwrap();
    assert!(units.is_empty());
}

#[test]
fn test_parse_nal_units_no_start_codes_returns_empty() {
    // Data without start codes returns empty Vec (not error)
    let data: &[u8] = &[0x00];
    let result = parse_nal_units(data);
    assert!(result.is_ok());
    let units = result.unwrap();
    assert!(units.is_empty());
}
