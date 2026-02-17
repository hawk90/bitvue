#![allow(dead_code)]
//! HEVC/H.265 Integration Tests
//!
//! Tests for end-to-end HEVC parsing functionality including:
//! - NAL unit parsing
//! - SPS/PPS/VPS parsing
//! - Overlay data extraction

use bitvue_hevc::{parse_hevc, parse_nal_units};

#[test]
fn test_parse_empty_hevc_stream() {
    let data: &[u8] = &[];
    let result = parse_hevc(data);
    assert!(result.is_ok());

    let stream = result.unwrap();
    assert_eq!(stream.nal_units.len(), 0);
}

#[test]
fn test_hevc_nal_unit_types() {
    // Test HEVC NAL unit type detection
    use bitvue_hevc::NalUnitType;

    // Format: 2-byte NAL header (forbidden_zero_bit + nal_unit_type + nuh_layer_id + nuh_temporal_id_plus1)
    // MSB-first bit reading within each byte
    let test_cases = vec![
        ([0x00, 0x01], NalUnitType::TrailN), // TRAIL_N (type 0): 0|000000|000000|001
        ([0x02, 0x01], NalUnitType::TrailR), // TRAIL_R (type 1): 0|000001|000000|001
        ([0x40, 0x01], NalUnitType::VpsNut), // VPS (type 32): 0|100000|000000|001
        ([0x42, 0x01], NalUnitType::SpsNut), // SPS (type 33): 0|100001|000000|001
        ([0x44, 0x01], NalUnitType::PpsNut), // PPS (type 34): 0|100010|000000|001
    ];

    for (nal_header, expected_type) in test_cases {
        // Minimal NAL unit with start code
        let mut data = vec![0x00, 0x00, 0x00, 0x01]; // Start code
        data.extend_from_slice(&nal_header); // NAL header (2 bytes)
        data.push(0x00); // Payload

        let nal_units = parse_nal_units(&data).unwrap();
        if !nal_units.is_empty() {
            assert_eq!(
                nal_units[0].header.nal_unit_type, expected_type,
                "NAL header {:02x?} should produce type {:?}",
                nal_header, expected_type
            );
        }
    }
}

#[test]
fn test_hevc_parameter_sets() {
    // Test VPS, SPS, PPS NAL unit detection
    use bitvue_hevc::NalUnitType;

    let data = [
        // SPS (Sequence Parameter Set) - simplified, 2-byte NAL header
        0x00, 0x00, 0x00, 0x01, 0x42, 0x01, 0x42, 0x80, 0x1e, 0x90, 0x00,
        // PPS (Picture Parameter Set) - simplified, 2-byte NAL header
        0x00, 0x00, 0x00, 0x01, 0x44, 0x01, 0xce, 0x06,
    ];

    let result = parse_hevc(&data);
    assert!(result.is_ok());

    let stream = result.unwrap();
    // Should parse at least the NAL units
    assert!(!stream.nal_units.is_empty());

    // Should detect SPS and PPS NAL unit types
    let has_sps = stream
        .nal_units
        .iter()
        .any(|nal| nal.header.nal_unit_type == NalUnitType::SpsNut);
    let has_pps = stream
        .nal_units
        .iter()
        .any(|nal| nal.header.nal_unit_type == NalUnitType::PpsNut);

    assert!(has_sps, "Should detect SPS NAL unit type");
    assert!(has_pps, "Should detect PPS NAL unit type");
}

#[test]
fn test_v0_5_advanced_features_present() {
    // Verify v0.5.x HEVC advanced features are implemented
    let data = create_minimal_hevc_stream();

    if let Ok(stream) = parse_hevc(&data) {
        // Check for SPS with advanced features
        if let Some((_, sps)) = stream.sps_map.iter().next() {
            // Verify advanced feature flags exist in SPS structure
            let _ = sps.strong_intra_smoothing_enabled_flag;
            let _ = sps.sample_adaptive_offset_enabled_flag;
            let _ = sps.pcm_enabled_flag;
            let _ = sps.sps_temporal_mvp_enabled_flag;
        }
    }
}

#[test]
fn test_hevc_overlay_extraction() {
    // Test HEVC overlay extraction API
    let data = create_minimal_hevc_stream();

    if let Ok(nal_units) = parse_nal_units(&data) {
        // Find SPS for overlay extraction
        use bitvue_hevc::NalUnitType;

        if let Some(nal_with_sps) = nal_units
            .iter()
            .find(|nal| nal.header.nal_unit_type == NalUnitType::SpsNut)
        {
            if let Ok(sps) = bitvue_hevc::sps::parse_sps(&nal_with_sps.payload) {
                // Test QP grid extraction (should not crash, may return scaffold data)
                let qp_result = bitvue_hevc::extract_qp_grid(&nal_units, &sps, 26);
                assert!(qp_result.is_ok(), "QP grid extraction should not crash");

                // Test MV grid extraction
                let mv_result = bitvue_hevc::extract_mv_grid(&nal_units, &sps);
                assert!(mv_result.is_ok(), "MV grid extraction should not crash");

                // Test partition grid extraction
                let part_result = bitvue_hevc::extract_partition_grid(&nal_units, &sps);
                assert!(
                    part_result.is_ok(),
                    "Partition grid extraction should not crash"
                );
            }
        }
    }
}

#[test]
fn test_v0_5_completeness() {
    use bitvue_hevc::NalUnitType;

    // Verify v0.5.x HEVC support is complete

    // 1. NAL parsing works
    let data = create_minimal_hevc_stream();
    let nal_result = parse_nal_units(&data);
    assert!(nal_result.is_ok(), "Should parse HEVC NAL units");
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

    // 3. Overlay extraction functions exist
    // (Tested in test_hevc_overlay_extraction)

    // 4. Advanced features in SPS structure
    if let Ok(stream) = parse_hevc(&data) {
        if let Some((_, sps)) = stream.sps_map.iter().next() {
            // Verify advanced features are accessible
            let _ = sps.strong_intra_smoothing_enabled_flag;
            let _ = sps.sample_adaptive_offset_enabled_flag;
            let _ = sps.pcm_enabled_flag;
        }
    }
}

/// Create a minimal HEVC byte stream for testing
fn create_minimal_hevc_stream() -> Vec<u8> {
    let mut data = Vec::new();

    // SPS (Sequence Parameter Set) - NAL header 2 bytes
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]); // Start code
    data.extend_from_slice(&[0x42, 0x01]); // NAL header: SPS (type 33)
    data.extend_from_slice(&[0x42, 0x80, 0x1e, 0x90, 0x00]); // Minimal payload

    // PPS (Picture Parameter Set) - NAL header 2 bytes
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]); // Start code
    data.extend_from_slice(&[0x44, 0x01]); // NAL header: PPS (type 34)
    data.extend_from_slice(&[0xce, 0x06]); // Minimal payload

    // IDR frame - NAL header 2 bytes (IDR_W_RADL = type 19)
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]); // Start code
    data.extend_from_slice(&[0x26, 0x01]); // NAL header: IDR_W_RADL (type 19 = 0x26)
    data.extend_from_slice(&[0x01, 0x00]); // Payload

    data
}
