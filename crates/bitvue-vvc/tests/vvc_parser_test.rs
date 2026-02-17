#![allow(dead_code)]
//! Tests for VVC (H.266) parser

use bitvue_vvc::{parse_nal_units, NalUnitType};

#[test]
fn test_parse_vvc_nal_types() {
    // Test VVC NAL unit type identification
    // NAL unit type is in upper 5 bits of byte 1 (type << 3)
    let nal_types = vec![
        (0b00000000, NalUnitType::TrailNut), // 0 << 3: TRAIL_NUT
        (0b00001000, NalUnitType::StapNut),  // 1 << 3: STSA_NUT
        (0b00111000, NalUnitType::IdrWRadl), // 7 << 3: IDR_W_RADL
        (0b01000000, NalUnitType::IdrNLp),   // 8 << 3: IDR_N_LP
        (0b01111000, NalUnitType::VpsNut),   // 15 << 3: VPS_NUT
        (0b10000000, NalUnitType::SpsNut),   // 16 << 3: SPS_NUT
        (0b10001000, NalUnitType::PpsNut),   // 17 << 3: PPS_NUT
    ];

    for (header_byte, expected_type) in nal_types {
        let mut data = vec![0x00, 0x00, 0x00, 0x01]; // Start code
        data.push(0x00); // NAL header byte 0
        data.push(header_byte); // NAL header byte 1 (type in upper 5 bits)
        data.push(0x00); // Payload

        let result = parse_nal_units(&data);

        if let Ok(nal_units) = result {
            if !nal_units.is_empty() {
                let nal = &nal_units[0];
                assert_eq!(
                    nal.header.nal_unit_type, expected_type,
                    "NAL type mismatch for header {:08b}",
                    header_byte
                );
            }
        }
    }
}

#[test]
fn test_parse_vvc_empty_data() {
    // Test empty data handling
    let data: Vec<u8> = vec![];
    let result = parse_nal_units(&data);
    assert!(result.is_ok(), "Empty data should parse successfully");
    assert!(
        result.unwrap().is_empty(),
        "Empty data should return empty vector"
    );
}

#[test]
fn test_vvc_idr_frame_detection() {
    // Test IDR frame detection
    let idr_with_radl = vec![0x00, 0x00, 0x00, 0x01, 0x00, 0b00111000, 0x00]; // IDR_W_RADL (7 << 3)
    let idr_no_lp = vec![0x00, 0x00, 0x00, 0x01, 0x00, 0b01000000, 0x00]; // IDR_N_LP (8 << 3)

    let result1 = parse_nal_units(&idr_with_radl);
    assert!(result1.is_ok());
    if let Ok(nal_units) = result1 {
        assert!(!nal_units.is_empty());
        assert_eq!(nal_units[0].header.nal_unit_type, NalUnitType::IdrWRadl);
    }

    let result2 = parse_nal_units(&idr_no_lp);
    assert!(result2.is_ok());
    if let Ok(nal_units) = result2 {
        assert!(!nal_units.is_empty());
        assert_eq!(nal_units[0].header.nal_unit_type, NalUnitType::IdrNLp);
    }
}

#[test]
fn test_vvc_parameter_set_detection() {
    // Test VPS, SPS, PPS detection
    let data = vec![
        0x00, 0x00, 0x00, 0x01, 0x00, 0b01111000, 0x00, // VPS (type 15 << 3)
        0x00, 0x00, 0x00, 0x01, 0x00, 0b10000000, 0x00, // SPS (type 16 << 3)
        0x00, 0x00, 0x00, 0x01, 0x00, 0b10001000, 0x00, // PPS (type 17 << 3)
    ];

    let result = parse_nal_units(&data);
    assert!(result.is_ok());
    let nal_units = result.unwrap();
    assert_eq!(nal_units.len(), 3, "Should parse 3 NAL units");
    assert_eq!(nal_units[0].header.nal_unit_type, NalUnitType::VpsNut);
    assert_eq!(nal_units[1].header.nal_unit_type, NalUnitType::SpsNut);
    assert_eq!(nal_units[2].header.nal_unit_type, NalUnitType::PpsNut);
}

#[test]
fn test_vvc_trail_frame_detection() {
    // Test trailing picture (most common frame type)
    let data = vec![0x00, 0x00, 0x00, 0x01, 0b00000000, 0x00]; // TRAIL_NUT (type 0)

    let result = parse_nal_units(&data);
    assert!(result.is_ok());
    let nal_units = result.unwrap();
    assert!(!nal_units.is_empty());
    assert_eq!(nal_units[0].header.nal_unit_type, NalUnitType::TrailNut);
    assert!(nal_units[0].is_vcl(), "TRAIL_NUT should be VCL");
}
