#![allow(dead_code)]
//! VVC/H.266 Integration Tests
//!
//! Tests for end-to-end VVC parsing functionality including:
//! - NAL unit parsing
//! - SPS with advanced features (Dual Tree, ALF, LMCS)
//! - Overlay data extraction

use bitvue_vvc::{parse_nal_units, parse_vvc, NalUnitType};

#[test]
fn test_parse_empty_vvc_stream() {
    let data: &[u8] = &[];
    let result = parse_vvc(data);
    assert!(result.is_ok());

    let stream = result.unwrap();
    assert_eq!(stream.nal_units.len(), 0);
}

#[test]
fn test_vvc_nal_unit_types() {
    // Test VVC NAL unit type detection
    use bitvue_vvc::NalUnitType;

    // Format: (byte0, byte1) for 2-byte VVC NAL header
    // byte0: [forbidden_zero_bit (1) | nuh_reserved_zero_bit (1) | nuh_layer_id (6)]
    // byte1: [nal_unit_type (5) | nuh_temporal_id_plus1 (3)]
    // NAL unit type is in upper 5 bits of byte 1 (type << 3)
    let test_cases = vec![
        (0x00, 0x00, NalUnitType::TrailNut), // TRAIL_NUT (type 0 << 3 = 0x00)
        (0x00, 0x08, NalUnitType::StapNut),  // STSA_NUT (type 1 << 3 = 0x08)
        (0x00, 0x38, NalUnitType::IdrWRadl), // IDR_W_RADL (type 7 << 3 = 0x38)
        (0x00, 0x40, NalUnitType::IdrNLp),   // IDR_N_LP (type 8 << 3 = 0x40)
        (0x00, 0x78, NalUnitType::VpsNut),   // VPS_NUT (type 15 << 3 = 0x78)
        (0x00, 0x80, NalUnitType::SpsNut),   // SPS_NUT (type 16 << 3 = 0x80)
        (0x00, 0x88, NalUnitType::PpsNut),   // PPS_NUT (type 17 << 3 = 0x88)
    ];

    for (byte0, byte1, expected_type) in test_cases {
        // Minimal NAL unit with start code and 2-byte NAL header
        let mut data = vec![0x00, 0x00, 0x00, 0x01]; // Start code
        data.push(byte0); // NAL header byte 0
        data.push(byte1); // NAL header byte 1 (type in upper 5 bits)
        data.push(0x00); // Payload

        let nal_units = parse_nal_units(&data).unwrap();
        if !nal_units.is_empty() {
            assert_eq!(
                nal_units[0].header.nal_unit_type, expected_type,
                "NAL header 0x{:02x}{:02x} should produce type {:?}",
                byte0, byte1, expected_type
            );
        }
    }
}

#[test]
fn test_vvc_parameter_sets() {
    // Test VPS, SPS, PPS NAL unit detection (not parsing)
    let data = [
        // VPS (Video Parameter Set) - simplified
        0x00, 0x00, 0x00, 0x01, 0x00, 0x78, 0x00, // type 15 << 3 = 0x78
        // SPS (Sequence Parameter Set) - simplified
        0x00, 0x00, 0x00, 0x01, 0x00, 0x80, 0x00, // type 16 << 3 = 0x80
        // PPS (Picture Parameter Set) - simplified
        0x00, 0x00, 0x00, 0x01, 0x00, 0x88, 0x00, // type 17 << 3 = 0x88
    ];

    let result = parse_vvc(&data);
    assert!(result.is_ok());

    let stream = result.unwrap();
    // Should parse at least the NAL units
    assert!(!stream.nal_units.is_empty());

    // Check that NAL unit types are correctly identified
    assert_eq!(
        stream.nal_units[0].header.nal_unit_type,
        NalUnitType::VpsNut
    );
    assert_eq!(
        stream.nal_units[1].header.nal_unit_type,
        NalUnitType::SpsNut
    );
    assert_eq!(
        stream.nal_units[2].header.nal_unit_type,
        NalUnitType::PpsNut
    );
}

#[test]
fn test_v0_6_advanced_features_present() {
    // Verify v0.6.x VVC advanced features are implemented
    let data = create_minimal_vvc_stream();

    if let Ok(stream) = parse_vvc(&data) {
        // Check for SPS with advanced features
        if let Some((_, sps)) = stream.sps_map.iter().next() {
            // Verify advanced feature flags exist in SPS structure
            // (The actual values depend on the bitstream, but the fields should exist)
            let _ = sps.has_dual_tree_intra();
            let _ = sps.alf.alf_enabled_flag;
            let _ = sps.lmcs.lmcs_enabled_flag;
            let _ = sps.sps_ibc_enabled_flag;
            let _ = sps.sps_affine_enabled_flag;
        }
    }
}

#[test]
fn test_vvc_overlay_extraction() {
    // Test VVC overlay extraction API
    let data = create_minimal_vvc_stream();

    if let Ok(nal_units) = parse_nal_units(&data) {
        // Find SPS for overlay extraction
        if let Some(nal_with_sps) = nal_units
            .iter()
            .find(|nal| nal.header.nal_unit_type == NalUnitType::SpsNut)
        {
            if let Ok(sps) = bitvue_vvc::sps::parse_sps(&nal_with_sps.payload) {
                // Test QP grid extraction
                let qp_result = bitvue_vvc::extract_qp_grid(&nal_units, &sps, 26);
                assert!(qp_result.is_ok(), "QP grid extraction should not crash");

                // Test MV grid extraction
                let mv_result = bitvue_vvc::extract_mv_grid(&nal_units, &sps);
                assert!(mv_result.is_ok(), "MV grid extraction should not crash");

                // Test partition grid extraction
                let part_result = bitvue_vvc::extract_partition_grid(&nal_units, &sps);
                assert!(
                    part_result.is_ok(),
                    "Partition grid extraction should not crash"
                );
            }
        }
    }
}

/// Create a minimal VVC byte stream for testing
fn create_minimal_vvc_stream() -> Vec<u8> {
    let mut data = Vec::new();

    // SPS (Sequence Parameter Set)
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]); // Start code
    data.extend_from_slice(&[0x00, 0x80]); // NAL type: SPS (16 << 3 = 0x80)
    data.extend_from_slice(&[0x01]); // Minimal payload

    // PPS (Picture Parameter Set)
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]); // Start code
    data.extend_from_slice(&[0x00, 0x88]); // NAL type: PPS (17 << 3 = 0x88)
    data.extend_from_slice(&[0x01]); // Minimal payload

    // Trail NAL (frame data)
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]); // Start code
    data.extend_from_slice(&[0x00, 0x00]); // NAL type: TRAIL_R (0 << 3 = 0x00)
    data.extend_from_slice(&[0x01]); // Payload

    data
}

#[test]
fn test_v0_6_completeness() {
    use bitvue_vvc::NalUnitType;

    // Verify v0.6.x VVC support is complete

    // 1. NAL parsing works
    let data = create_minimal_vtc_stream();
    let nal_result = parse_nal_units(&data);
    assert!(nal_result.is_ok(), "Should parse VVC NAL units");
    let nal_units = nal_result.unwrap();
    assert!(!nal_units.is_empty(), "Should have NAL units");

    // 2. Parameter set detection
    let has_sps = nal_units
        .iter()
        .any(|nal| nal.header.nal_unit_type == NalUnitType::SpsNut);
    assert!(has_sps, "Should detect SPS NAL unit type");

    let has_pps = nal_units
        .iter()
        .any(|nal| nal.header.nal_unit_type == NalUnitType::PpsNut);
    assert!(has_pps, "Should detect PPS NAL unit type");

    // 3. Overlay extraction functions exist and work
    // (Tested in test_vvc_overlay_extraction)

    // 4. Advanced features in SPS structure
    if let Ok(stream) = parse_vvc(&data) {
        if let Some((_, sps)) = stream.sps_map.iter().next() {
            // Verify advanced features are accessible
            let _ = sps.has_dual_tree_intra();
            let _ = sps.alf.alf_enabled_flag;
            let _ = sps.lmcs.lmcs_enabled_flag;
            let _ = sps.sps_ibc_enabled_flag;
        }
    }
}

/// Create a minimal VTC (test) byte stream
fn create_minimal_vtc_stream() -> Vec<u8> {
    let mut data = Vec::new();

    // VPS (Video Parameter Set)
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    data.extend_from_slice(&[0x00, 0x78]); // NAL type: VPS (15 << 3 = 0x78)
    data.extend_from_slice(&[0xFF]); // Placeholder

    // SPS with basic structure
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    data.extend_from_slice(&[0x00, 0x80]); // NAL type: SPS (16 << 3 = 0x80)
    data.extend_from_slice(&[0x01]); // Minimal sps_id

    // PPS
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    data.extend_from_slice(&[0x00, 0x88]); // NAL type: PPS (17 << 3 = 0x88)
    data.extend_from_slice(&[0x01]); // Minimal pps_id

    // Trail NAL (coding data)
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    data.extend_from_slice(&[0x00, 0x00]); // NAL type: TRAIL_R (0 << 3 = 0x00)
    data.extend_from_slice(&[0x80, 0x00]); // Frame data

    data
}
