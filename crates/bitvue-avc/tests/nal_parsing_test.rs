#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(unused_comparisons)]
#![allow(unused_doc_comments)]
//! AVC NAL Unit Parsing Tests
//!
//! Comprehensive tests for AVC (H.264) NAL unit parsing functionality.

use bitvue_avc::nal::{find_nal_units, parse_nal_header, parse_nal_units, NalUnit, NalUnitType};
use bitvue_avc::AvcError;

// ============================================================================
// NAL Header Parsing Tests
// ============================================================================

#[test]
fn test_nal_header_non_idr() {
    // Non-IDR slice (type 1): 0|00001|000
    let data = [0x21]; // nal_ref_idc=1, nal_unit_type=1
    let result = parse_nal_header(data[0]);
    assert!(result.is_ok());
    let header = result.unwrap();
    assert_eq!(header.nal_unit_type, NalUnitType::NonIdrSlice);
    assert_eq!(header.nal_ref_idc, 1);
}

#[test]
fn test_nal_header_idr() {
    // IDR slice (type 5): 0|00101|011
    let data = [0x65]; // nal_ref_idc=3, nal_unit_type=5
    let result = parse_nal_header(data[0]);
    assert!(result.is_ok());
    let header = result.unwrap();
    assert_eq!(header.nal_unit_type, NalUnitType::IdrSlice);
    assert_eq!(header.nal_ref_idc, 3);
}

#[test]
fn test_nal_header_sps() {
    // SPS (type 7): 0|00111|011
    let data = [0x67]; // nal_ref_idc=3, nal_unit_type=7
    let result = parse_nal_header(data[0]);
    assert!(result.is_ok());
    let header = result.unwrap();
    assert_eq!(header.nal_unit_type, NalUnitType::Sps);
    assert_eq!(header.nal_ref_idc, 3);
}

#[test]
fn test_nal_header_pps() {
    // PPS (type 8): 0|01000|011
    let data = [0x68]; // nal_ref_idc=3, nal_unit_type=8
    let result = parse_nal_header(data[0]);
    assert!(result.is_ok());
    let header = result.unwrap();
    assert_eq!(header.nal_unit_type, NalUnitType::Pps);
    assert_eq!(header.nal_ref_idc, 3);
}

#[test]
fn test_nal_header_sei() {
    // SEI (type 6): 0|00110|000
    let data = [0x06]; // nal_ref_idc=0, nal_unit_type=6
    let result = parse_nal_header(data[0]);
    assert!(result.is_ok());
    let header = result.unwrap();
    assert_eq!(header.nal_unit_type, NalUnitType::Sei);
    assert_eq!(header.nal_ref_idc, 0);
}

#[test]
fn test_nal_header_aud() {
    // AUD (type 9): 0|01001|000
    let data = [0x09]; // nal_ref_idc=0, nal_unit_type=9
    let result = parse_nal_header(data[0]);
    assert!(result.is_ok());
    let header = result.unwrap();
    assert_eq!(header.nal_unit_type, NalUnitType::Aud);
    assert_eq!(header.nal_ref_idc, 0);
}

#[test]
fn test_nal_header_end_of_sequence() {
    // End of sequence (type 10): 0|01010|000
    let data = [0x0A]; // nal_ref_idc=0, nal_unit_type=10
    let result = parse_nal_header(data[0]);
    assert!(result.is_ok());
    let header = result.unwrap();
    assert_eq!(header.nal_unit_type, NalUnitType::EndOfSequence);
}

#[test]
fn test_nal_header_end_of_stream() {
    // End of stream (type 11): 0|01011|000
    let data = [0x0B]; // nal_ref_idc=0, nal_unit_type=11
    let result = parse_nal_header(data[0]);
    assert!(result.is_ok());
    let header = result.unwrap();
    assert_eq!(header.nal_unit_type, NalUnitType::EndOfStream);
}

#[test]
fn test_nal_header_filler_data() {
    // Filler data (type 12): 0|01100|000
    let data = [0x0C]; // nal_ref_idc=0, nal_unit_type=12
    let result = parse_nal_header(data[0]);
    assert!(result.is_ok());
    let header = result.unwrap();
    assert_eq!(header.nal_unit_type, NalUnitType::FillerData);
}

#[test]
fn test_nal_header_nal_ref_idc_variants() {
    // Test different nal_ref_idc values with same nal_unit_type
    let test_cases = vec![
        (0x01, 0, NalUnitType::NonIdrSlice), // ref_idc=0
        (0x21, 1, NalUnitType::NonIdrSlice), // ref_idc=1
        (0x41, 2, NalUnitType::NonIdrSlice), // ref_idc=2
        (0x61, 3, NalUnitType::NonIdrSlice), // ref_idc=3
    ];

    for (byte, expected_ref_idc, expected_type) in test_cases {
        let result = parse_nal_header(byte);
        assert!(result.is_ok());
        let header = result.unwrap();
        assert_eq!(header.nal_ref_idc, expected_ref_idc);
        assert_eq!(header.nal_unit_type, expected_type);
    }
}

#[test]
fn test_nal_header_all_slice_types() {
    // Test all slice types (1-5)
    let slice_types = vec![
        (0x21, NalUnitType::NonIdrSlice), // Non-IDR
        (0x22, NalUnitType::SliceDataA),  // Data partition A
        (0x23, NalUnitType::SliceDataB),  // Data partition B
        (0x24, NalUnitType::SliceDataC),  // Data partition C
        (0x65, NalUnitType::IdrSlice),    // IDR
    ];

    for (byte, expected_type) in slice_types {
        let result = parse_nal_header(byte);
        assert!(result.is_ok());
        let header = result.unwrap();
        assert_eq!(header.nal_unit_type, expected_type);
    }
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
    // 3-byte start code
    data.extend_from_slice(&[0x00, 0x00, 0x01]);
    // NAL header (SPS)
    data.push(0x67);
    // Payload
    data.extend_from_slice(&[0xFF, 0xFF, 0xFF]);

    let result = parse_nal_units(&data);
    assert!(result.is_ok());
    let nal_units = result.unwrap();
    assert_eq!(nal_units.len(), 1);
    assert_eq!(nal_units[0].header.nal_unit_type, NalUnitType::Sps);
}

#[test]
fn test_parse_nal_units_multiple_nal() {
    let mut data = Vec::new();
    // First NAL (SPS)
    data.extend_from_slice(&[0x00, 0x00, 0x01]);
    data.push(0x67);
    data.extend_from_slice(&[0xFF, 0xFF]);
    // Second NAL (PPS)
    data.extend_from_slice(&[0x00, 0x00, 0x01]);
    data.push(0x68);
    data.extend_from_slice(&[0xFF, 0xFF]);

    let result = parse_nal_units(&data);
    assert!(result.is_ok());
    let nal_units = result.unwrap();
    assert_eq!(nal_units.len(), 2);
    assert_eq!(nal_units[0].header.nal_unit_type, NalUnitType::Sps);
    assert_eq!(nal_units[1].header.nal_unit_type, NalUnitType::Pps);
}

#[test]
fn test_parse_nal_units_four_byte_start_code() {
    let mut data = Vec::new();
    // 4-byte start code
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    // NAL header
    data.push(0x67);
    // Payload
    data.extend_from_slice(&[0xFF]);

    let result = parse_nal_units(&data);
    assert!(result.is_ok());
    let nal_units = result.unwrap();
    assert_eq!(nal_units.len(), 1);
}

#[test]
fn test_parse_nal_units_mixed_start_codes() {
    let mut data = Vec::new();
    // 3-byte start code
    data.extend_from_slice(&[0x00, 0x00, 0x01]);
    data.push(0x67);
    data.push(0xFF);
    // 4-byte start code
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    data.push(0x68);
    data.push(0xFF);

    let result = parse_nal_units(&data);
    assert!(result.is_ok());
    let nal_units = result.unwrap();
    assert_eq!(nal_units.len(), 2);
}

#[test]
fn test_parse_nal_units_offset_tracking() {
    let mut data = Vec::new();
    // First NAL at offset 0
    data.extend_from_slice(&[0x00, 0x00, 0x01]);
    data.push(0x67);
    data.extend_from_slice(&[0xFF, 0xFF]);
    // Second NAL after some data
    data.extend_from_slice(&[0x00, 0x00, 0x01]);
    data.push(0x68);
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
    data.extend_from_slice(&[0x00, 0x00, 0x01]);
    data.push(0x67);
    let payload_size = 10usize;
    data.extend_from_slice(&vec![0xFFu8; payload_size]);

    let result = parse_nal_units(&data);
    assert!(result.is_ok());
    let nal_units = result.unwrap();
    // Size should include header + payload
    assert!(nal_units[0].size >= 1); // At least NAL header
}

#[test]
fn test_parse_nal_units_payload_extraction() {
    let mut data = Vec::new();
    data.extend_from_slice(&[0x00, 0x00, 0x01]);
    data.push(0x67);
    let payload = vec![0xDE, 0xAD, 0xBE, 0xEF];
    data.extend_from_slice(&payload);

    let result = parse_nal_units(&data);
    assert!(result.is_ok());
    let nal_units = result.unwrap();
    // Check payload is extracted
    assert!(!nal_units[0].payload.is_empty());
}

#[test]
fn test_parse_nal_units_idr_frame() {
    let mut data = Vec::new();
    // SPS
    data.extend_from_slice(&[0x00, 0x00, 0x01]);
    data.push(0x67);
    data.push(0xFF);
    // PPS
    data.extend_from_slice(&[0x00, 0x00, 0x01]);
    data.push(0x68);
    data.push(0xFF);
    // IDR frame
    data.extend_from_slice(&[0x00, 0x00, 0x01]);
    data.push(0x65); // IDR slice
    data.extend_from_slice(&[0xFF, 0xFF]);

    let result = parse_nal_units(&data);
    assert!(result.is_ok());
    let nal_units = result.unwrap();
    assert_eq!(nal_units.len(), 3);
    assert_eq!(nal_units[2].header.nal_unit_type, NalUnitType::IdrSlice);
}

#[test]
fn test_parse_nal_units_with_aud() {
    let mut data = Vec::new();
    // AUD
    data.extend_from_slice(&[0x00, 0x00, 0x01]);
    data.push(0x09);
    data.push(0xFF);
    // SPS
    data.extend_from_slice(&[0x00, 0x00, 0x01]);
    data.push(0x67);
    data.push(0xFF);

    let result = parse_nal_units(&data);
    assert!(result.is_ok());
    let nal_units = result.unwrap();
    assert_eq!(nal_units.len(), 2);
    assert_eq!(nal_units[0].header.nal_unit_type, NalUnitType::Aud);
    assert_eq!(nal_units[1].header.nal_unit_type, NalUnitType::Sps);
}

#[test]
fn test_parse_nal_units_with_sei() {
    let mut data = Vec::new();
    // SPS
    data.extend_from_slice(&[0x00, 0x00, 0x01]);
    data.push(0x67);
    data.push(0xFF);
    // SEI
    data.extend_from_slice(&[0x00, 0x00, 0x01]);
    data.push(0x06);
    data.extend_from_slice(&[0xFF, 0xFF, 0xFF]);

    let result = parse_nal_units(&data);
    assert!(result.is_ok());
    let nal_units = result.unwrap();
    assert_eq!(nal_units.len(), 2);
    assert_eq!(nal_units[1].header.nal_unit_type, NalUnitType::Sei);
}

// ============================================================================
// NAL Unit Type Detection Tests
// ============================================================================

#[test]
fn test_nal_unit_type_vcl() {
    // Test VCL (Video Coding Layer) NAL unit types
    let vcl_types = vec![
        NalUnitType::NonIdrSlice,
        NalUnitType::IdrSlice,
        NalUnitType::SliceDataA,
        NalUnitType::SliceDataB,
        NalUnitType::SliceDataC,
    ];

    for nal_type in vcl_types {
        // All these types should be VCL NAL units (1-5, 19)
        let type_val = nal_type as u8;
        assert!((1..=5).contains(&type_val) || type_val == 19);
    }
}

#[test]
fn test_nal_unit_type_non_vcl() {
    // Test non-VCL NAL unit types
    let non_vcl_types = vec![
        NalUnitType::Sps,
        NalUnitType::Pps,
        NalUnitType::Sei,
        NalUnitType::Aud,
        NalUnitType::EndOfSequence,
        NalUnitType::EndOfStream,
        NalUnitType::FillerData,
    ];

    for nal_type in non_vcl_types {
        // These should be non-VCL NAL units
        let type_val = nal_type as u8;
        assert!(type_val >= 6);
    }
}

#[test]
fn test_nal_unit_type_range() {
    // Test that all NAL unit types are in valid range (0-31)
    let all_types = vec![
        NalUnitType::Unspecified,
        NalUnitType::NonIdrSlice,
        NalUnitType::SliceDataA,
        NalUnitType::SliceDataB,
        NalUnitType::SliceDataC,
        NalUnitType::IdrSlice,
        NalUnitType::Sei,
        NalUnitType::Sps,
        NalUnitType::Pps,
        NalUnitType::Aud,
    ];

    for nal_type in all_types {
        let type_val = nal_type as u8;
        assert!(type_val <= 31, "NAL unit type must be 0-31");
    }
}

#[test]
fn test_nal_ref_idc_meaning() {
    // Test nal_ref_idc values
    // 0 = not used for reference
    // 1, 2, 3 = used for reference (higher = higher priority)
    let data_no_ref = [0x01]; // Non-ref slice
    let data_low_ref = [0x21]; // Low priority ref
    let data_med_ref = [0x41]; // Medium priority ref
    let data_high_ref = [0x61]; // High priority ref

    let header_no_ref = parse_nal_header(data_no_ref[0]).unwrap();
    let header_low_ref = parse_nal_header(data_low_ref[0]).unwrap();
    let header_med_ref = parse_nal_header(data_med_ref[0]).unwrap();
    let header_high_ref = parse_nal_header(data_high_ref[0]).unwrap();

    assert_eq!(header_no_ref.nal_ref_idc, 0);
    assert_eq!(header_low_ref.nal_ref_idc, 1);
    assert_eq!(header_med_ref.nal_ref_idc, 2);
    assert_eq!(header_high_ref.nal_ref_idc, 3);
}

// ============================================================================
// Edge Case Tests
// ============================================================================

#[test]
fn test_parse_nal_units_no_start_code() {
    let data = vec![0x67, 0xFF, 0xFF]; // No start code
    let result = parse_nal_units(&data);
    // Should return empty list
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
    // Two consecutive start codes
    data.extend_from_slice(&[0x00, 0x00, 0x01]);
    data.push(0x67);
    data.extend_from_slice(&[0x00, 0x00, 0x01]); // Start code immediately
    data.push(0x68);

    let result = parse_nal_units(&data);
    assert!(result.is_ok());
}

#[test]
fn test_parse_nal_units_start_code_in_payload() {
    let mut data = Vec::new();
    // NAL unit with start code-like pattern in payload
    data.extend_from_slice(&[0x00, 0x00, 0x01]);
    data.push(0x67);
    data.extend_from_slice(&[0x00, 0x00, 0x01]); // Start code in payload
    data.push(0xFF);
    // Real start code
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    data.push(0x68);

    let result = parse_nal_units(&data);
    // Should either succeed or return error depending on implementation
    // For now, just check that result is not panicking
    let _ = result;
}

#[test]
fn test_parse_nal_units_zero_byte_before_four_byte_start_code() {
    let mut data = Vec::new();
    // Some data
    data.extend_from_slice(&[0xFF, 0xFF]);
    // Zero byte + 3-byte start code (4-byte start code equivalent)
    data.push(0x00);
    data.extend_from_slice(&[0x00, 0x00, 0x01]);
    data.push(0x67);
    data.push(0xFF);

    let result = parse_nal_units(&data);
    assert!(result.is_ok());
    let nal_units = result.unwrap();
    // Should find at least one NAL
    assert!(nal_units.len() >= 1);
}

#[test]
fn test_parse_nal_units_end_of_stream() {
    let mut data = Vec::new();
    // SPS
    data.extend_from_slice(&[0x00, 0x00, 0x01]);
    data.push(0x67);
    data.push(0xFF);
    // PPS
    data.extend_from_slice(&[0x00, 0x00, 0x01]);
    data.push(0x68);
    data.push(0xFF);
    // End of stream
    data.extend_from_slice(&[0x00, 0x00, 0x01]);
    data.push(0x0B); // End of stream NAL

    let result = parse_nal_units(&data);
    assert!(result.is_ok());
    let nal_units = result.unwrap();
    assert_eq!(
        nal_units.last().unwrap().header.nal_unit_type,
        NalUnitType::EndOfStream
    );
}

// ============================================================================
// Integration Tests
// ============================================================================

#[test]
fn test_parse_avc_stream_parameter_sets() {
    // Minimal AVC stream with SPS, PPS
    let mut data = Vec::new();
    // SPS
    data.extend_from_slice(&[0x00, 0x00, 0x01]);
    data.push(0x67);
    data.extend_from_slice(&[0x42, 0x80, 0x1E]); // profile/level
    data.extend_from_slice(&[0xFF, 0xFF]);
    // PPS
    data.extend_from_slice(&[0x00, 0x00, 0x01]);
    data.push(0x68);
    data.push(0xCE); // pps_id=0, sps_id=0
    data.extend_from_slice(&[0xFF, 0xFF]);

    let result = parse_nal_units(&data);
    assert!(result.is_ok());
    let nal_units = result.unwrap();
    assert_eq!(nal_units.len(), 2);
    assert_eq!(nal_units[0].header.nal_unit_type, NalUnitType::Sps);
    assert_eq!(nal_units[1].header.nal_unit_type, NalUnitType::Pps);
}

#[test]
fn test_parse_avc_stream_idr_sequence() {
    // Stream with parameter sets and IDR frame
    let mut data = Vec::new();
    // SPS
    data.extend_from_slice(&[0x00, 0x00, 0x01]);
    data.push(0x67);
    data.push(0xFF);
    // PPS
    data.extend_from_slice(&[0x00, 0x00, 0x01]);
    data.push(0x68);
    data.push(0xFF);
    // IDR frame
    data.extend_from_slice(&[0x00, 0x00, 0x01]);
    data.push(0x65); // IDR slice
    data.extend_from_slice(&[0xFF, 0xFF]);
    // Non-IDR frame
    data.extend_from_slice(&[0x00, 0x00, 0x01]);
    data.push(0x41); // Non-IDR slice
    data.extend_from_slice(&[0xFF, 0xFF]);

    let result = parse_nal_units(&data);
    assert!(result.is_ok());
    let nal_units = result.unwrap();
    assert_eq!(nal_units.len(), 4);
    assert_eq!(nal_units[2].header.nal_unit_type, NalUnitType::IdrSlice);
    assert_eq!(nal_units[3].header.nal_unit_type, NalUnitType::NonIdrSlice);
}

#[test]
fn test_parse_avc_stream_with_aud() {
    // Stream with Access Unit Delimiter
    let mut data = Vec::new();
    // AUD
    data.extend_from_slice(&[0x00, 0x00, 0x01]);
    data.push(0x09);
    data.push(0xF0); // primary_pic_type = 7 (I, P, B frames)
                     // IDR frame
    data.extend_from_slice(&[0x00, 0x00, 0x01]);
    data.push(0x65);
    data.extend_from_slice(&[0xFF, 0xFF]);

    let result = parse_nal_units(&data);
    assert!(result.is_ok());
    let nal_units = result.unwrap();
    assert_eq!(nal_units.len(), 2);
    assert_eq!(nal_units[0].header.nal_unit_type, NalUnitType::Aud);
    assert_eq!(nal_units[1].header.nal_unit_type, NalUnitType::IdrSlice);
}

#[test]
fn test_parse_avc_ref_idc_priority() {
    // Test that frames with different nal_ref_idc are parsed correctly
    let mut data = Vec::new();
    // Non-reference frame (nal_ref_idc=0)
    data.extend_from_slice(&[0x00, 0x00, 0x01]);
    data.push(0x01); // Non-IDR, not for reference
    data.push(0xFF);
    // High priority reference frame (nal_ref_idc=3)
    data.extend_from_slice(&[0x00, 0x00, 0x01]);
    data.push(0x65); // IDR, for reference
    data.push(0xFF);

    let result = parse_nal_units(&data);
    assert!(result.is_ok());
    let nal_units = result.unwrap();
    assert_eq!(nal_units.len(), 2);
    assert_eq!(nal_units[0].header.nal_ref_idc, 0);
    assert_eq!(nal_units[1].header.nal_ref_idc, 3);
}

// ============================================================================
// NalUnitType::from_u8 Tests
// ============================================================================

#[test]
fn test_nal_type_from_u8_all_values() {
    let test_cases = vec![
        (0, NalUnitType::Unspecified),
        (1, NalUnitType::NonIdrSlice),
        (2, NalUnitType::SliceDataA),
        (3, NalUnitType::SliceDataB),
        (4, NalUnitType::SliceDataC),
        (5, NalUnitType::IdrSlice),
        (6, NalUnitType::Sei),
        (7, NalUnitType::Sps),
        (8, NalUnitType::Pps),
        (9, NalUnitType::Aud),
        (10, NalUnitType::EndOfSequence),
        (11, NalUnitType::EndOfStream),
        (12, NalUnitType::FillerData),
        (13, NalUnitType::SpsExtension),
        (14, NalUnitType::PrefixNal),
        (15, NalUnitType::SubsetSps),
        (16, NalUnitType::Dps),
        (17, NalUnitType::Reserved17),
        (18, NalUnitType::Reserved18),
        (19, NalUnitType::AuxSlice),
        (20, NalUnitType::SliceExtension),
        (21, NalUnitType::SliceExtensionDepth),
        (22, NalUnitType::Reserved22),
        (23, NalUnitType::Reserved23),
        (24, NalUnitType::Unspecified24),
        (31, NalUnitType::Unspecified24),
    ];

    for (value, expected) in test_cases {
        assert_eq!(
            NalUnitType::from_u8(value),
            expected,
            "from_u8({}) should match",
            value
        );
    }
}

// ============================================================================
// NalUnitType::is_vcl Tests
// ============================================================================

#[test]
fn test_nal_type_is_vcl_all_vcl_types() {
    let vcl_types = vec![
        NalUnitType::NonIdrSlice,
        NalUnitType::SliceDataA,
        NalUnitType::SliceDataB,
        NalUnitType::SliceDataC,
        NalUnitType::IdrSlice,
        NalUnitType::AuxSlice,
        NalUnitType::SliceExtension,
        NalUnitType::SliceExtensionDepth,
    ];

    for nal_type in vcl_types {
        assert!(nal_type.is_vcl(), "{:?} should be VCL", nal_type);
    }
}

#[test]
fn test_nal_type_is_vcl_non_vcl_types() {
    let non_vcl_types = vec![
        NalUnitType::Unspecified,
        NalUnitType::Sei,
        NalUnitType::Sps,
        NalUnitType::Pps,
        NalUnitType::Aud,
        NalUnitType::EndOfSequence,
        NalUnitType::EndOfStream,
        NalUnitType::FillerData,
        NalUnitType::SpsExtension,
        NalUnitType::PrefixNal,
        NalUnitType::SubsetSps,
        NalUnitType::Dps,
        NalUnitType::Reserved17,
        NalUnitType::Reserved18,
        NalUnitType::Reserved22,
        NalUnitType::Reserved23,
        NalUnitType::Unspecified24,
    ];

    for nal_type in non_vcl_types {
        assert!(!nal_type.is_vcl(), "{:?} should not be VCL", nal_type);
    }
}

// ============================================================================
// NalUnitType::is_parameter_set Tests
// ============================================================================

#[test]
fn test_nal_type_is_parameter_set_true() {
    let ps_types = vec![
        NalUnitType::Sps,
        NalUnitType::Pps,
        NalUnitType::SpsExtension,
        NalUnitType::SubsetSps,
    ];

    for nal_type in ps_types {
        assert!(
            nal_type.is_parameter_set(),
            "{:?} should be parameter set",
            nal_type
        );
    }
}

#[test]
fn test_nal_type_is_parameter_set_false() {
    let non_ps_types = vec![
        NalUnitType::Unspecified,
        NalUnitType::NonIdrSlice,
        NalUnitType::SliceDataA,
        NalUnitType::IdrSlice,
        NalUnitType::Sei,
        NalUnitType::Aud,
        NalUnitType::EndOfSequence,
        NalUnitType::FillerData,
        NalUnitType::PrefixNal,
        NalUnitType::Dps,
        NalUnitType::AuxSlice,
    ];

    for nal_type in non_ps_types {
        assert!(
            !nal_type.is_parameter_set(),
            "{:?} should not be parameter set",
            nal_type
        );
    }
}

// ============================================================================
// NalUnitType::name Tests
// ============================================================================

#[test]
fn test_nal_type_name() {
    let test_cases = vec![
        (NalUnitType::Unspecified, "Unspecified"),
        (NalUnitType::NonIdrSlice, "Non-IDR Slice"),
        (NalUnitType::SliceDataA, "Slice Data A"),
        (NalUnitType::SliceDataB, "Slice Data B"),
        (NalUnitType::SliceDataC, "Slice Data C"),
        (NalUnitType::IdrSlice, "IDR Slice"),
        (NalUnitType::Sei, "SEI"),
        (NalUnitType::Sps, "SPS"),
        (NalUnitType::Pps, "PPS"),
        (NalUnitType::Aud, "AUD"),
        (NalUnitType::EndOfSequence, "End of Sequence"),
        (NalUnitType::EndOfStream, "End of Stream"),
        (NalUnitType::FillerData, "Filler Data"),
        (NalUnitType::SpsExtension, "SPS Extension"),
        (NalUnitType::PrefixNal, "Prefix NAL"),
        (NalUnitType::SubsetSps, "Subset SPS"),
        (NalUnitType::Dps, "DPS"),
        (NalUnitType::Reserved17, "Reserved"),
        (NalUnitType::Reserved18, "Reserved"),
        (NalUnitType::AuxSlice, "Auxiliary Slice"),
        (NalUnitType::SliceExtension, "Slice Extension"),
        (NalUnitType::SliceExtensionDepth, "Slice Extension (Depth)"),
        (NalUnitType::Reserved22, "Reserved"),
        (NalUnitType::Reserved23, "Reserved"),
        (NalUnitType::Unspecified24, "Unspecified"),
    ];

    for (nal_type, expected_name) in test_cases {
        assert_eq!(
            nal_type.name(),
            expected_name,
            "name() for {:?} should match",
            nal_type
        );
    }
}

// ============================================================================
// parse_nal_header Error Tests
// ============================================================================

#[test]
fn test_parse_nal_header_forbidden_bit_set() {
    let byte = 0x80; // forbidden_zero_bit is set (bit 7 = 1)
    let result = parse_nal_header(byte);
    assert!(result.is_err());
    match result {
        Err(AvcError::InvalidNalUnit(msg)) => {
            assert!(msg.contains("forbidden_zero_bit"));
        }
        _ => panic!("Expected InvalidNalUnit error"),
    }
}

#[test]
fn test_parse_nal_header_all_nal_ref_idc_values() {
    for ref_idc in 0..=3 {
        let byte = (ref_idc << 5) | 7; // SPS type with different ref_idc
        let header = parse_nal_header(byte).unwrap();
        assert_eq!(header.nal_ref_idc, ref_idc);
    }
}

// ============================================================================
// find_nal_units Tests
// ============================================================================

#[test]
fn test_find_nal_units_empty() {
    let data: &[u8] = &[];
    let positions = find_nal_units(data);
    assert_eq!(positions.len(), 0);
}

#[test]
fn test_find_nal_units_no_start_codes() {
    let data = [0x00, 0x01, 0x02, 0x03];
    let positions = find_nal_units(&data);
    assert_eq!(positions.len(), 0);
}

#[test]
fn test_find_nal_units_only_3byte_start_codes() {
    let data = [0x00, 0x00, 0x01, 0x67, 0x00, 0x00, 0x01, 0x68];
    let positions = find_nal_units(&data);
    assert_eq!(positions, vec![3, 7]);
}

#[test]
fn test_find_nal_units_only_4byte_start_codes() {
    let data = [0x00, 0x00, 0x00, 0x01, 0x67, 0x00, 0x00, 0x00, 0x01, 0x68];
    let positions = find_nal_units(&data);
    assert_eq!(positions, vec![4, 9]);
}

#[test]
fn test_find_nal_units_mixed_start_codes() {
    let data = [
        0x00, 0x00, 0x01, 0x67, // 3-byte
        0x00, 0x00, 0x00, 0x01, 0x68, // 4-byte
        0x00, 0x00, 0x01, 0x65, // 3-byte
    ];
    let positions = find_nal_units(&data);
    assert_eq!(positions, vec![3, 8, 12]);
}

#[test]
fn test_find_nal_units_partial_start_code() {
    let data = [0x00, 0x00, 0x02, 0x67]; // 0x02 instead of 0x01
    let positions = find_nal_units(&data);
    assert_eq!(positions.len(), 0);
}

#[test]
fn test_find_nal_units_overlapping_patterns() {
    let data = [0x00, 0x00, 0x00, 0x00, 0x01, 0x67];
    let positions = find_nal_units(&data);
    assert_eq!(positions, vec![5]);
}

#[test]
fn test_find_nal_units_at_end_of_data() {
    let data = [0x00, 0x00, 0x00, 0x01];
    let positions = find_nal_units(&data);
    assert_eq!(positions, vec![4]);
}

// ============================================================================
// NalUnit Methods Tests
// ============================================================================

#[test]
fn test_nal_unit_nal_type() {
    let data = [0x00, 0x00, 0x01, 0x67, 0x42];
    let result = parse_nal_units(&data).unwrap();
    assert_eq!(result[0].nal_type(), NalUnitType::Sps);
}

#[test]
fn test_nal_unit_is_reference_true() {
    let data = [0x00, 0x00, 0x01, 0x67, 0x42]; // nal_ref_idc=3
    let result = parse_nal_units(&data).unwrap();
    assert!(result[0].is_reference());
}

#[test]
fn test_nal_unit_is_reference_false() {
    let data = [0x00, 0x00, 0x01, 0x06, 0x42]; // nal_ref_idc=0
    let result = parse_nal_units(&data).unwrap();
    assert!(!result[0].is_reference());
}

#[test]
fn test_nal_unit_is_reference_all_ref_idc() {
    for ref_idc in 1..=3 {
        let byte = (ref_idc << 5) | 7;
        let data = [0x00, 0x00, 0x01, byte, 0x42];
        let result = parse_nal_units(&data).unwrap();
        assert!(
            result[0].is_reference(),
            "nal_ref_idc={} should be reference",
            ref_idc
        );
    }
}

#[test]
fn test_nal_unit_roundtrip() {
    let data = [
        0x00, 0x00, 0x01, 0x67, 0x42, 0x80, 0x0A, 0x00, 0x00, 0x01, 0x68, 0xDE, 0x3C, 0x80,
    ];
    let result = parse_nal_units(&data).unwrap();

    assert_eq!(result.len(), 2);
    assert_eq!(result[0].nal_type(), NalUnitType::Sps);
    assert_eq!(result[1].nal_type(), NalUnitType::Pps);
}

#[test]
fn test_nal_unit_payload_processing() {
    // Test that raw_payload includes emulation prevention bytes
    // Emulation prevention: 0x00 0x00 0x03 -> 0x00 0x00
    let data = [
        0x00, 0x00, 0x01, 0x67, // start code + header
        0x42, 0x00, 0x00, 0x03, 0x00, // payload with emulation prevention (00 00 03)
    ];
    let result = parse_nal_units(&data).unwrap();

    assert_eq!(result[0].raw_payload, vec![0x42, 0x00, 0x00, 0x03, 0x00]);
    assert_eq!(result[0].payload, vec![0x42, 0x00, 0x00, 0x00]);
}
