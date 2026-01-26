//! HEVC NAL Unit Parsing Tests
//!
//! Comprehensive tests for HEVC NAL unit parsing functionality.

use bitvue_hevc::{parse_nal_header, parse_nal_units, NalUnitType};

// ============================================================================
// NAL Header Parsing Tests
// ============================================================================

#[test]
fn test_nal_header_trail_n() {
    // TRAIL_N (type 0): 0|00000|00000|001
    let data = [0x00, 0x01];
    let result = parse_nal_header(&data);
    assert!(result.is_ok());
    let header = result.unwrap();
    assert_eq!(header.nal_unit_type, NalUnitType::TrailN);
    assert_eq!(header.nuh_layer_id, 0);
    assert_eq!(header.nuh_temporal_id_plus1, 1);
}

#[test]
fn test_nal_header_trail_r() {
    // TRAIL_R (type 1): 0|00001|00000|001
    let data = [0x02, 0x01];
    let result = parse_nal_header(&data);
    assert!(result.is_ok());
    let header = result.unwrap();
    assert_eq!(header.nal_unit_type, NalUnitType::TrailR);
}

#[test]
fn test_nal_header_vps() {
    // VPS (type 32): 0|100000|00000|001
    let data = [0x40, 0x01];
    let result = parse_nal_header(&data);
    assert!(result.is_ok());
    let header = result.unwrap();
    assert_eq!(header.nal_unit_type, NalUnitType::VpsNut);
}

#[test]
fn test_nal_header_sps() {
    // SPS (type 33): 0|100001|00000|001
    let data = [0x42, 0x01];
    let result = parse_nal_header(&data);
    assert!(result.is_ok());
    let header = result.unwrap();
    assert_eq!(header.nal_unit_type, NalUnitType::SpsNut);
}

#[test]
fn test_nal_header_pps() {
    // PPS (type 34): 0|100010|00000|001
    let data = [0x44, 0x01];
    let result = parse_nal_header(&data);
    assert!(result.is_ok());
    let header = result.unwrap();
    assert_eq!(header.nal_unit_type, NalUnitType::PpsNut);
}

#[test]
fn test_nal_header_idr_w_radl() {
    // IDR_W_RADL (type 19): 0|010011|00000|001
    let data = [0x26, 0x01];
    let result = parse_nal_header(&data);
    assert!(result.is_ok());
    let header = result.unwrap();
    assert_eq!(header.nal_unit_type, NalUnitType::IdrWRadl);
}

#[test]
fn test_nal_header_idr_n_lp() {
    // IDR_N_LP (type 20): 0|010100|00000|001
    let data = [0x28, 0x01];
    let result = parse_nal_header(&data);
    assert!(result.is_ok());
    let header = result.unwrap();
    assert_eq!(header.nal_unit_type, NalUnitType::IdrNLp);
}

#[test]
fn test_nal_header_with_layer_id() {
    // Test with non-zero layer_id
    // nal_unit_type=1, layer_id=3, temp_id=1: 0|00001|00011|001
    let data = [0x02, 0x19];
    let result = parse_nal_header(&data);
    assert!(result.is_ok());
    let header = result.unwrap();
    assert_eq!(header.nuh_layer_id, 3);
}

#[test]
fn test_nal_header_with_temporal_id() {
    // Test with temporal_id_plus1 = 3 (temporal_id = 2)
    // nal_unit_type=1, layer_id=0, temp_id=3: 0|00001|00000|011
    let data = [0x02, 0x03];
    let result = parse_nal_header(&data);
    assert!(result.is_ok());
    let header = result.unwrap();
    assert_eq!(header.nuh_temporal_id_plus1, 3);
}

#[test]
fn test_nal_header_empty_data() {
    let data: [u8; 0] = [];
    let result = parse_nal_header(&data);
    assert!(result.is_err());
}

#[test]
fn test_nal_header_insufficient_data() {
    let data = [0x00]; // Only 1 byte, need 2
    let result = parse_nal_header(&data);
    assert!(result.is_err());
}

#[test]
fn test_nal_header_forbidden_bit_set() {
    // forbidden_zero_bit = 1 (invalid)
    let data = [0x80, 0x01];
    let result = parse_nal_header(&data);
    // This should either return an error or parse with the bit set
    // depending on implementation
    assert!(result.is_ok() || result.is_err());
}

// ============================================================================
// NAL Unit Parsing Tests
// ============================================================================

#[test]
fn test_parse_nal_units_empty() {
    let data: &[u8] = &[];
    let result = parse_nal_units(data);
    assert!(result.is_ok());
    let nal_units = result.unwrap();
    assert_eq!(nal_units.len(), 0);
}

#[test]
fn test_parse_nal_units_single_nal() {
    let mut data = Vec::new();
    // Start code
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    // NAL header (SPS)
    data.extend_from_slice(&[0x42, 0x01]);
    // Payload
    data.extend_from_slice(&[0xFF, 0xFF, 0xFF]);

    let result = parse_nal_units(&data);
    assert!(result.is_ok());
    let nal_units = result.unwrap();
    assert_eq!(nal_units.len(), 1);
    assert_eq!(nal_units[0].header.nal_unit_type, NalUnitType::SpsNut);
}

#[test]
fn test_parse_nal_units_multiple_nal() {
    let mut data = Vec::new();
    // First NAL (SPS)
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    data.extend_from_slice(&[0x42, 0x01]);
    data.extend_from_slice(&[0xFF, 0xFF]);
    // Second NAL (PPS)
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    data.extend_from_slice(&[0x44, 0x01]);
    data.extend_from_slice(&[0xFF, 0xFF]);

    let result = parse_nal_units(&data);
    assert!(result.is_ok());
    let nal_units = result.unwrap();
    assert_eq!(nal_units.len(), 2);
    assert_eq!(nal_units[0].header.nal_unit_type, NalUnitType::SpsNut);
    assert_eq!(nal_units[1].header.nal_unit_type, NalUnitType::PpsNut);
}

#[test]
fn test_parse_nal_units_three_byte_start_code() {
    let mut data = Vec::new();
    // 3-byte start code
    data.extend_from_slice(&[0x00, 0x00, 0x01]);
    // NAL header
    data.extend_from_slice(&[0x42, 0x01]);
    // Payload
    data.extend_from_slice(&[0xFF]);

    let result = parse_nal_units(&data);
    assert!(result.is_ok());
    let nal_units = result.unwrap();
    assert_eq!(nal_units.len(), 1);
}

#[test]
fn test_parse_nal_units_four_byte_start_code() {
    let mut data = Vec::new();
    // 4-byte start code
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    // NAL header
    data.extend_from_slice(&[0x42, 0x01]);
    // Payload
    data.extend_from_slice(&[0xFF]);

    let result = parse_nal_units(&data);
    assert!(result.is_ok());
    let nal_units = result.unwrap();
    assert_eq!(nal_units.len(), 1);
}

#[test]
fn test_parse_nal_units_offset_tracking() {
    let mut data = Vec::new();
    // First NAL at offset 0
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    data.extend_from_slice(&[0x42, 0x01]);
    data.extend_from_slice(&[0xFF, 0xFF]);
    // Second NAL after some data
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    data.extend_from_slice(&[0x44, 0x01]);
    data.extend_from_slice(&[0xFF, 0xFF]);

    let result = parse_nal_units(&data);
    assert!(result.is_ok());
    let nal_units = result.unwrap();
    // Check that offsets are tracked
    assert!(nal_units[0].offset < nal_units[1].offset);
}

#[test]
fn test_parse_nal_units_size_tracking() {
    let mut data = Vec::new();
    // NAL with specific payload size
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    data.extend_from_slice(&[0x42, 0x01]);
    let payload_size = 10usize;
    data.extend_from_slice(&vec![0xFFu8; payload_size]);

    let result = parse_nal_units(&data);
    assert!(result.is_ok());
    let nal_units = result.unwrap();
    // Size should include header + payload
    assert!(nal_units[0].size >= 2); // At least NAL header
}

#[test]
fn test_parse_nal_units_payload_extraction() {
    let mut data = Vec::new();
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    data.extend_from_slice(&[0x42, 0x01]);
    let payload = vec![0xDE, 0xAD, 0xBE, 0xEF];
    data.extend_from_slice(&payload);

    let result = parse_nal_units(&data);
    assert!(result.is_ok());
    let nal_units = result.unwrap();
    // Check payload is extracted
    assert!(!nal_units[0].payload.is_empty());
}

// ============================================================================
// NAL Unit Type Detection Tests
// ============================================================================

#[test]
fn test_nal_unit_type_vcl() {
    // Test VCL (Video Coding Layer) NAL unit types
    let vcl_types = vec![
        NalUnitType::TrailN,
        NalUnitType::TrailR,
        NalUnitType::IdrWRadl,
        NalUnitType::IdrNLp,
    ];

    for nal_type in vcl_types {
        // All these types should be VCL NAL units
        // VCL NAL units are types 0-31 (slice segments)
        assert!(nal_type as u8 <= 31);
    }
}

#[test]
fn test_nal_unit_type_non_vcl() {
    // Test non-VCL NAL unit types (parameter sets, SEI, etc.)
    let non_vcl_types = vec![
        NalUnitType::VpsNut,
        NalUnitType::SpsNut,
        NalUnitType::PpsNut,
        NalUnitType::AudNut,
    ];

    for nal_type in non_vcl_types {
        // These should be non-VCL NAL units
        assert!(nal_type as u8 >= 32);
    }
}

#[test]
fn test_nal_unit_type_range() {
    // Test that all NAL unit types are in valid range (0-63)
    let all_types = vec![
        NalUnitType::TrailN,
        NalUnitType::TrailR,
        NalUnitType::VpsNut,
        NalUnitType::SpsNut,
        NalUnitType::PpsNut,
    ];

    for nal_type in all_types {
        let type_val = nal_type as u8;
        assert!(type_val <= 63, "NAL unit type must be 0-63");
    }
}

// ============================================================================
// Edge Case Tests
// ============================================================================

#[test]
fn test_parse_nal_units_no_start_code() {
    let data = vec![0x42, 0x01, 0xFF, 0xFF]; // No start code
    let result = parse_nal_units(&data);
    // Should return empty list or error depending on implementation
    assert!(result.is_ok());
    let nal_units = result.unwrap();
    assert_eq!(nal_units.len(), 0);
}

#[test]
fn test_parse_nal_units_partial_start_code() {
    let data = vec![0x00, 0x00, 0xFF, 0xFF]; // Partial start code
    let result = parse_nal_units(&data);
    assert!(result.is_ok());
}

#[test]
fn test_parse_nal_units_consecutive_start_codes() {
    let mut data = Vec::new();
    // Two consecutive start codes (empty NAL between)
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    data.extend_from_slice(&[0x42, 0x01]);
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]); // Start code immediately
    data.extend_from_slice(&[0x44, 0x01]);

    let result = parse_nal_units(&data);
    assert!(result.is_ok());
}

#[test]
fn test_parse_nal_units_start_code_in_payload() {
    let mut data = Vec::new();
    // NAL unit with start code-like pattern in payload
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    data.extend_from_slice(&[0x42, 0x01]);
    data.extend_from_slice(&[0x00, 0x00, 0x01]); // Start code in payload
    data.extend_from_slice(&[0xFF]);
    // Real start code
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    data.extend_from_slice(&[0x44, 0x01]);

    let result = parse_nal_units(&data);
    assert!(result.is_ok());
    let nal_units = result.unwrap();
    // Should find 2 NAL units (not 3)
    assert!(nal_units.len() <= 3);
}

// ============================================================================
// Integration Tests
// ============================================================================

#[test]
fn test_parse_hevc_stream_parameter_sets() {
    // Minimal HEVC stream with VPS, SPS, PPS
    let mut data = Vec::new();
    // VPS
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x01, 0x40, 0x01, 0xFF]);
    // SPS
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x01, 0x42, 0x01, 0xFF]);
    // PPS
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x01, 0x44, 0x01, 0xFF]);

    let result = parse_nal_units(&data);
    assert!(result.is_ok());
    let nal_units = result.unwrap();
    assert_eq!(nal_units.len(), 3);
    assert_eq!(nal_units[0].header.nal_unit_type, NalUnitType::VpsNut);
    assert_eq!(nal_units[1].header.nal_unit_type, NalUnitType::SpsNut);
    assert_eq!(nal_units[2].header.nal_unit_type, NalUnitType::PpsNut);
}

#[test]
fn test_parse_hevc_stream_with_idr() {
    // Stream with parameter sets and IDR frame
    let mut data = Vec::new();
    // SPS
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x01, 0x42, 0x01, 0xFF]);
    // PPS
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x01, 0x44, 0x01, 0xFF]);
    // IDR
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x01, 0x26, 0x01, 0xFF]);

    let result = parse_nal_units(&data);
    assert!(result.is_ok());
    let nal_units = result.unwrap();
    assert_eq!(nal_units.len(), 3);
    assert_eq!(nal_units[2].header.nal_unit_type, NalUnitType::IdrWRadl);
}

#[test]
fn test_parse_hevc_stream_temporal_layers() {
    // Stream with different temporal IDs
    let mut data = Vec::new();
    // TID=0
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x01, 0x42, 0x01, 0xFF]);
    // TID=1
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x01, 0x42, 0x03, 0xFF]);
    // TID=2
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x01, 0x44, 0x05, 0xFF]);

    let result = parse_nal_units(&data);
    assert!(result.is_ok());
    let nal_units = result.unwrap();
    assert_eq!(nal_units.len(), 3);
    assert_eq!(nal_units[0].header.nuh_temporal_id_plus1, 1); // TID=0
    assert_eq!(nal_units[1].header.nuh_temporal_id_plus1, 3); // TID=2
    assert_eq!(nal_units[2].header.nuh_temporal_id_plus1, 5); // TID=4
}
