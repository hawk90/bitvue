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
//! H.264/AVC Integration Tests
//!
//! Tests for end-to-end H.264 parsing functionality including:
//! - NAL unit parsing from Annex B byte streams
//! - Frame extraction
//! - Basic overlay data extraction

use bitvue_avc::{extract_annex_b_frames, parse_avc, parse_nal_units};

#[test]
fn test_parse_empty_stream() {
    let data: &[u8] = &[];
    let result = parse_avc(data);
    assert!(result.is_ok());

    let stream = result.unwrap();
    assert_eq!(stream.nal_units.len(), 0);
}

#[test]
fn test_extract_frames_empty() {
    let data: &[u8] = &[];
    let frames = extract_annex_b_frames(data);
    assert!(frames.is_ok());
    assert!(frames.unwrap().is_empty());
}

#[test]
fn test_nal_unit_detection() {
    // Test NAL unit start code detection
    let data = [
        0x00, 0x00, 0x00, 0x01, 0x67, // SPS
        0x00, 0x00, 0x00, 0x01, 0x68, // PPS
        0x00, 0x00, 0x00, 0x01, 0x65, // IDR
    ];

    let nal_units = parse_nal_units(&data);
    assert!(nal_units.is_ok());

    let nal_units = nal_units.unwrap();
    assert_eq!(nal_units.len(), 3);

    // Verify NAL unit types
    assert_eq!(
        nal_units[0].header.nal_unit_type,
        bitvue_avc::NalUnitType::Sps
    );
    assert_eq!(
        nal_units[1].header.nal_unit_type,
        bitvue_avc::NalUnitType::Pps
    );
    assert_eq!(
        nal_units[2].header.nal_unit_type,
        bitvue_avc::NalUnitType::IdrSlice
    );
}

#[test]
fn test_three_byte_start_code() {
    // Test 3-byte start code (0x00 0x00 0x01)
    let data = [
        0x00, 0x00, 0x01, 0x67, // SPS with 3-byte start code
        0x42, 0x80,
    ];

    let nal_units = parse_nal_units(&data);
    assert!(nal_units.is_ok());

    let nal_units = nal_units.unwrap();
    assert_eq!(nal_units.len(), 1);
    assert_eq!(
        nal_units[0].header.nal_unit_type,
        bitvue_avc::NalUnitType::Sps
    );
}

#[test]
fn test_nal_ref_idc() {
    // Test NAL reference indication flag
    // nal_ref_idc is in bits 5-6 (value >> 5)
    let tests = vec![
        (0x67, 3), // SPS: 0x67 = 01100111 -> nal_ref_idc = 11 = 3
        (0x68, 3), // PPS: 0x68 = 01101000 -> nal_ref_idc = 11 = 3
        (0x65, 3), // IDR: 0x65 = 01100101 -> nal_ref_idc = 11 = 3
        (0x61, 3), // Non-IDR: 0x61 = 01100001 -> nal_ref_idc = 11 = 3
        (0x21, 1), // Non-ref: 0x21 = 00100001 -> nal_ref_idc = 01 = 1
    ];

    for (nal_header_byte, expected_ref_idc) in tests {
        let header = bitvue_avc::parse_nal_header(nal_header_byte).unwrap();
        assert_eq!(
            header.nal_ref_idc, expected_ref_idc,
            "NAL header 0x{:02x} should have nal_ref_idc={}",
            nal_header_byte, expected_ref_idc
        );
    }
}

#[test]
fn test_overlay_extraction_api() {
    // Test that overlay extraction API is available and doesn't crash
    let data = create_minimal_h264_stream();

    if let Ok(nal_units) = parse_nal_units(&data) {
        // Find SPS
        let sps_option = nal_units
            .iter()
            .find(|nal| nal.header.nal_unit_type == bitvue_avc::NalUnitType::Sps);

        if let Some(nal_with_sps) = sps_option {
            if let Ok(sps) = bitvue_avc::sps::parse_sps(&nal_with_sps.payload) {
                // Test QP grid extraction (should not crash, may return scaffold data)
                let qp_result = bitvue_avc::extract_qp_grid(&nal_units, &sps, 26);
                assert!(qp_result.is_ok(), "QP grid extraction should not crash");

                // Test MV grid extraction
                let mv_result = bitvue_avc::extract_mv_grid(&nal_units, &sps);
                assert!(mv_result.is_ok(), "MV grid extraction should not crash");

                // Test partition grid extraction
                let part_result = bitvue_avc::extract_partition_grid(&nal_units, &sps);
                assert!(
                    part_result.is_ok(),
                    "Partition grid extraction should not crash"
                );
            }
        }
    }
}

#[test]
fn test_v0_4_avc_support_completeness() {
    // Verify v0.4.x H.264/AVC support has all required components

    // 1. NAL parsing
    let data = create_minimal_h264_stream();
    let nal_result = parse_nal_units(&data);
    assert!(nal_result.is_ok(), "Should parse NAL units");
    let nal_units = nal_result.unwrap();
    assert!(!nal_units.is_empty(), "Should have NAL units");

    // 2. SPS type detection
    let has_sps = nal_units
        .iter()
        .any(|nal| nal.header.nal_unit_type == bitvue_avc::NalUnitType::Sps);
    assert!(has_sps, "Should detect SPS NAL unit type");

    // 3. PPS type detection
    let has_pps = nal_units
        .iter()
        .any(|nal| nal.header.nal_unit_type == bitvue_avc::NalUnitType::Pps);
    assert!(has_pps, "Should detect PPS NAL unit type");

    // 4. VCL (Video Coding Layer) detection
    let has_vcl = nal_units
        .iter()
        .any(|nal| nal.header.nal_unit_type.is_vcl());
    assert!(has_vcl, "Should detect VCL NAL unit type");

    // 5. VCL vs non-VCL classification
    for nal in &nal_units {
        let is_vcl = nal.header.nal_unit_type.is_vcl();
        let is_sps_or_pps = nal.header.nal_unit_type == bitvue_avc::NalUnitType::Sps
            || nal.header.nal_unit_type == bitvue_avc::NalUnitType::Pps;

        if is_sps_or_pps {
            assert!(!is_vcl, "SPS/PPS should not be VCL");
        }
    }

    // 6. Overlay extraction functions exist
    let sps = create_test_sps();
    assert!(
        bitvue_avc::extract_qp_grid(&nal_units, &sps, 26).is_ok(),
        "extract_qp_grid should be callable"
    );
    assert!(
        bitvue_avc::extract_mv_grid(&nal_units, &sps).is_ok(),
        "extract_mv_grid should be callable"
    );
    assert!(
        bitvue_avc::extract_partition_grid(&nal_units, &sps).is_ok(),
        "extract_partition_grid should be callable"
    );
}

/// Create a minimal H.264 byte stream for testing
fn create_minimal_h264_stream() -> Vec<u8> {
    let mut data = Vec::new();

    // SPS (Sequence Parameter Set)
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    data.push(0x67); // NAL type: SPS
    data.extend_from_slice(&[0x42, 0x80, 0x1e, 0x90, 0x00]);

    // PPS (Picture Parameter Set)
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    data.push(0x68); // NAL type: PPS
    data.extend_from_slice(&[0xce, 0x06]);

    // IDR frame
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    data.push(0x65); // NAL type: IDR
    data.extend_from_slice(&[0x01, 0x00]);

    data
}

/// Create a test SPS for overlay extraction
fn create_test_sps() -> bitvue_avc::sps::Sps {
    bitvue_avc::sps::Sps {
        // Minimal SPS for 176x144 (QCIF)
        profile_idc: bitvue_avc::sps::ProfileIdc::Baseline,
        constraint_set0_flag: false,
        constraint_set1_flag: false,
        constraint_set2_flag: false,
        constraint_set3_flag: false,
        constraint_set4_flag: false,
        constraint_set5_flag: false,
        level_idc: 10,
        seq_parameter_set_id: 0,
        chroma_format_idc: bitvue_avc::sps::ChromaFormat::Yuv420,
        separate_colour_plane_flag: false,
        bit_depth_luma_minus8: 0,
        bit_depth_chroma_minus8: 0,
        qpprime_y_zero_transform_bypass_flag: false,
        seq_scaling_matrix_present_flag: false,
        log2_max_frame_num_minus4: 0,
        pic_order_cnt_type: 0,
        log2_max_pic_order_cnt_lsb_minus4: 0,
        delta_pic_order_always_zero_flag: false,
        offset_for_non_ref_pic: 0,
        offset_for_top_to_bottom_field: 0,
        num_ref_frames_in_pic_order_cnt_cycle: 0,
        offset_for_ref_frame: Vec::new(),
        max_num_ref_frames: 1,
        gaps_in_frame_num_value_allowed_flag: false,
        pic_width_in_mbs_minus1: 10,
        pic_height_in_map_units_minus1: 8,
        frame_mbs_only_flag: true,
        mb_adaptive_frame_field_flag: false,
        direct_8x8_inference_flag: false,
        frame_cropping_flag: false,
        frame_crop_left_offset: 0,
        frame_crop_right_offset: 0,
        frame_crop_top_offset: 0,
        frame_crop_bottom_offset: 0,
        vui_parameters_present_flag: false,
        vui_parameters: None,
    }
}

#[test]
fn test_frame_extraction_count() {
    let data = create_minimal_h264_stream();
    let frames = extract_annex_b_frames(&data);

    assert!(frames.is_ok());
    let frames = frames.unwrap();

    // Should extract at least one frame
    assert!(!frames.is_empty(), "Should extract at least one frame");

    // All extracted frames should have valid offsets
    for frame in &frames {
        assert!(
            frame.offset as usize <= data.len(),
            "Frame offset should be within data"
        );
        assert!(frame.size > 0, "Frame should have non-zero size");
    }
}

#[test]
fn test_nal_header_parsing() {
    // Test various NAL header values
    // Format: (byte, expected_type, expected_ref_idc)
    let test_cases = vec![
        (0x67, bitvue_avc::NalUnitType::Sps, 3),         // SPS, ref
        (0x68, bitvue_avc::NalUnitType::Pps, 3),         // PPS, ref
        (0x65, bitvue_avc::NalUnitType::IdrSlice, 3),    // IDR, ref
        (0x61, bitvue_avc::NalUnitType::NonIdrSlice, 3), // Non-IDR, ref
        (0x21, bitvue_avc::NalUnitType::NonIdrSlice, 1), // Non-ref slice
        (0x06, bitvue_avc::NalUnitType::Sei, 0),         // SEI, non-ref
        (0x07, bitvue_avc::NalUnitType::Sps, 0),         // SPS (alt, non-ref)
    ];

    for (byte, expected_type, expected_ref_idc) in test_cases {
        let header = bitvue_avc::parse_nal_header(byte).unwrap();
        assert_eq!(
            header.nal_unit_type, expected_type,
            "Byte 0x{:02x} should produce type {:?}",
            byte, expected_type
        );
        assert_eq!(
            header.nal_ref_idc, expected_ref_idc,
            "Byte 0x{:02x} should have ref_idc {}",
            byte, expected_ref_idc
        );
    }
}
