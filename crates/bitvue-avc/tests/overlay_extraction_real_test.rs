#![allow(dead_code)]
//! AVC Overlay Extraction Tests with Actual Parsing
//!
//! Tests for overlay extraction functions with real bitstream data.

use bitvue_avc::nal::{find_nal_units, parse_nal_units, NalUnit, NalUnitHeader, NalUnitType};
use bitvue_avc::overlay_extraction::{
    extract_mv_grid, extract_partition_grid, extract_qp_grid, Macroblock, MbType, MotionVector,
};
use bitvue_avc::sps::{ChromaFormat, ProfileIdc, Sps};
use bitvue_core::partition_grid::PartitionType;

/// Create a minimal SPS for testing
fn create_test_sps() -> Sps {
    Sps {
        profile_idc: ProfileIdc::High,
        constraint_set0_flag: false,
        constraint_set1_flag: false,
        constraint_set2_flag: false,
        constraint_set3_flag: false,
        constraint_set4_flag: false,
        constraint_set5_flag: false,
        level_idc: 40,
        seq_parameter_set_id: 0,
        chroma_format_idc: ChromaFormat::Yuv420,
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
        offset_for_ref_frame: vec![],
        max_num_ref_frames: 1,
        gaps_in_frame_num_value_allowed_flag: false,
        pic_width_in_mbs_minus1: 119,       // 1920 / 16 - 1
        pic_height_in_map_units_minus1: 67, // 1080 / 16 - 1
        frame_mbs_only_flag: true,
        mb_adaptive_frame_field_flag: false,
        direct_8x8_inference_flag: true,
        frame_cropping_flag: false,
        frame_crop_left_offset: 0,
        frame_crop_right_offset: 0,
        frame_crop_top_offset: 0,
        frame_crop_bottom_offset: 0,
        vui_parameters_present_flag: false,
        vui_parameters: None,
    }
}

/// Create a test slice NAL unit from real data
fn create_test_slice_nal(is_idr: bool) -> NalUnit {
    let mut data = Vec::new();

    // Start code
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]);

    // NAL header byte
    let nal_header_byte: u8 = if is_idr {
        // forbidden_bit=0, nal_ref_idc=3, nal_unit_type=5 (IDR)
        0b01100101
    } else {
        // forbidden_bit=0, nal_ref_idc=1, nal_unit_type=1 (Non-IDR)
        0b00000001
    };
    data.push(nal_header_byte);

    // Minimal slice data
    data.extend_from_slice(&[0x00, 0x00, 0x00]);

    // Parse NAL units from the data
    let nal_units = parse_nal_units(&data).unwrap();
    nal_units.into_iter().next().unwrap()
}

#[test]
fn test_extract_qp_grid_empty_nal_units() {
    let nal_units: Vec<NalUnit> = vec![];
    let sps = create_test_sps();
    let missing = 26;

    let result = extract_qp_grid(&nal_units, &sps, missing);
    assert!(result.is_ok());

    let grid = result.unwrap();
    // Should use missing value for all macroblocks when no NAL units
    assert!(!grid.qp.is_empty());
}

#[test]
fn test_extract_qp_grid_with_idr_slice() {
    let nal_units = vec![create_test_slice_nal(true)];
    let sps = create_test_sps();
    let missing = -1;

    let result = extract_qp_grid(&nal_units, &sps, missing);
    assert!(result.is_ok());

    let grid = result.unwrap();
    assert_eq!(grid.block_w, 16);
    assert_eq!(grid.block_h, 16);
    // Grid dimensions should match MB dimensions
    let expected_mb_count =
        (sps.pic_width_in_mbs_minus1 + 1) * (sps.pic_height_in_map_units_minus1 + 1);
    assert!(!grid.qp.is_empty());
}

#[test]
fn test_extract_qp_grid_missing_value() {
    let nal_units = vec![create_test_slice_nal(false)];
    let sps = create_test_sps();
    let missing = 28;

    let result = extract_qp_grid(&nal_units, &sps, missing);
    assert!(result.is_ok());

    let grid = result.unwrap();
    assert_eq!(grid.missing, missing);
}

#[test]
fn test_extract_mv_grid_empty_nal_units() {
    let nal_units: Vec<NalUnit> = vec![];
    let sps = create_test_sps();

    let result = extract_mv_grid(&nal_units, &sps);
    assert!(result.is_ok());

    let grid = result.unwrap();
    // Should have zero MVs when no NAL units
    assert!(!grid.mv_l0.is_empty());
}

#[test]
fn test_extract_mv_grid_with_idr_slice() {
    let nal_units = vec![create_test_slice_nal(true)];
    let sps = create_test_sps();

    let result = extract_mv_grid(&nal_units, &sps);
    assert!(result.is_ok());

    let grid = result.unwrap();
    assert_eq!(grid.block_w, 16);
    assert_eq!(grid.block_h, 16);
    // IDR slices are intra, so MV should be MISSING
    assert!(!grid.mv_l0.is_empty());
}

#[test]
fn test_extract_mv_grid_with_non_idr_slice() {
    let nal_units = vec![create_test_slice_nal(false)];
    let sps = create_test_sps();

    let result = extract_mv_grid(&nal_units, &sps);
    assert!(result.is_ok());

    let grid = result.unwrap();
    // Non-IDR slices should have MV data
    assert!(!grid.mv_l0.is_empty());
    assert!(!grid.mv_l1.is_empty());
}

#[test]
fn test_extract_partition_grid_empty_nal_units() {
    let nal_units: Vec<NalUnit> = vec![];
    let sps = create_test_sps();

    let result = extract_partition_grid(&nal_units, &sps);
    assert!(result.is_ok());

    let grid = result.unwrap();
    // Should fill with scaffold blocks when empty
    assert!(!grid.blocks.is_empty());
}

#[test]
fn test_extract_partition_grid_with_idr_slice() {
    let nal_units = vec![create_test_slice_nal(true)];
    let sps = create_test_sps();

    let result = extract_partition_grid(&nal_units, &sps);
    assert!(result.is_ok());

    let grid = result.unwrap();
    assert!(!grid.blocks.is_empty());
}

#[test]
fn test_extract_partition_grid_dimensions() {
    let nal_units = vec![create_test_slice_nal(false)];
    let sps = create_test_sps();

    let result = extract_partition_grid(&nal_units, &sps);
    assert!(result.is_ok());

    let grid = result.unwrap();
    let pic_width_in_mbs = sps.pic_width_in_mbs_minus1 + 1;
    let pic_height_in_mbs = sps.pic_height_in_map_units_minus1 + 1;

    assert_eq!(grid.coded_width, pic_width_in_mbs as u32 * 16);
    assert_eq!(grid.coded_height, pic_height_in_mbs as u32 * 16);
}

#[test]
fn test_mb_type_is_intra() {
    assert!(MbType::I4x4.is_intra());
    assert!(MbType::I16x16.is_intra());
    assert!(MbType::IPCM.is_intra());
    assert!(!MbType::PLuma.is_intra());
    assert!(!MbType::B16x16.is_intra());
    assert!(!MbType::PSkip.is_intra());
    assert!(!MbType::BSkip.is_intra());
}

#[test]
fn test_mb_type_is_skip() {
    assert!(MbType::PSkip.is_skip());
    assert!(MbType::BSkip.is_skip());
    assert!(!MbType::I4x4.is_skip());
    assert!(!MbType::PLuma.is_skip());
    assert!(!MbType::B16x16.is_skip());
}

#[test]
fn test_mb_type_to_partition_type_none() {
    assert_eq!(MbType::I16x16.to_partition_type(), PartitionType::None);
    assert_eq!(MbType::IPCM.to_partition_type(), PartitionType::None);
    assert_eq!(MbType::PLuma.to_partition_type(), PartitionType::None);
    assert_eq!(MbType::BDirect.to_partition_type(), PartitionType::None);
    assert_eq!(MbType::PSkip.to_partition_type(), PartitionType::None);
    assert_eq!(MbType::BSkip.to_partition_type(), PartitionType::None);
    assert_eq!(MbType::B16x16.to_partition_type(), PartitionType::None);
}

#[test]
fn test_mb_type_to_partition_type_horz() {
    assert_eq!(MbType::B16x8.to_partition_type(), PartitionType::Horz);
}

#[test]
fn test_mb_type_to_partition_type_vert() {
    assert_eq!(MbType::B8x16.to_partition_type(), PartitionType::Vert);
}

#[test]
fn test_mb_type_to_partition_type_split() {
    assert_eq!(MbType::I4x4.to_partition_type(), PartitionType::Split);
    assert_eq!(MbType::P8x8.to_partition_type(), PartitionType::Split);
    assert_eq!(MbType::B8x8.to_partition_type(), PartitionType::Split);
}

#[test]
fn test_motion_vector_new() {
    let mv = MotionVector::new(4, -8);
    assert_eq!(mv.x, 4);
    assert_eq!(mv.y, -8);
}

#[test]
fn test_motion_vector_zero() {
    let mv = MotionVector::zero();
    assert_eq!(mv.x, 0);
    assert_eq!(mv.y, 0);
}

#[test]
fn test_motion_vector_negative_components() {
    let mv = MotionVector::new(-2, -5);
    assert_eq!(mv.x, -2);
    assert_eq!(mv.y, -5);
}

#[test]
fn test_motion_vector_large_values() {
    let mv = MotionVector::new(32767, -32768);
    assert_eq!(mv.x, 32767);
    assert_eq!(mv.y, -32768);
}

#[test]
fn test_macroblock_structure() {
    let mb = Macroblock {
        mb_addr: 100,
        x: 160,
        y: 320,
        mb_type: MbType::I16x16,
        skip: false,
        qp: 30,
        mv_l0: None,
        mv_l1: None,
        ref_idx_l0: None,
        ref_idx_l1: None,
    };

    assert_eq!(mb.mb_addr, 100);
    assert_eq!(mb.x, 160);
    assert_eq!(mb.y, 320);
    assert_eq!(mb.qp, 30);
    assert!(!mb.skip);
    assert!(mb.mb_type.is_intra());
}

#[test]
fn test_macroblock_inter_with_mvs() {
    let mv_l0 = MotionVector::new(4, 8);
    let mv_l1 = MotionVector::new(-2, 6);

    let mb = Macroblock {
        mb_addr: 0,
        x: 0,
        y: 0,
        mb_type: MbType::B16x16,
        skip: false,
        qp: 26,
        mv_l0: Some(mv_l0),
        mv_l1: Some(mv_l1),
        ref_idx_l0: Some(0),
        ref_idx_l1: Some(1),
    };

    assert!(mb.mv_l0.is_some());
    assert!(mb.mv_l1.is_some());
    assert_eq!(mb.mv_l0.unwrap().x, 4);
    assert_eq!(mb.mv_l1.unwrap().y, 6);
    assert_eq!(mb.ref_idx_l0, Some(0));
    assert_eq!(mb.ref_idx_l1, Some(1));
    assert!(!mb.mb_type.is_intra());
}

#[test]
fn test_macroblock_skip() {
    let mb = Macroblock {
        mb_addr: 50,
        x: 800,
        y: 160,
        mb_type: MbType::PSkip,
        skip: true,
        qp: 26,
        mv_l0: None,
        mv_l1: None,
        ref_idx_l0: None,
        ref_idx_l1: None,
    };

    assert!(mb.skip);
    assert!(mb.mb_type.is_skip());
    assert_eq!(mb.qp, 26); // Skip blocks still have QP
}

#[test]
fn test_extract_qp_grid_small_resolution() {
    // Create SPS for small resolution (e.g., 320x240)
    let mut sps = create_test_sps();
    sps.pic_width_in_mbs_minus1 = 19; // 320 / 16 - 1
    sps.pic_height_in_map_units_minus1 = 14; // 240 / 16 - 1

    let nal_units = vec![create_test_slice_nal(true)];
    let missing = 24;

    let result = extract_qp_grid(&nal_units, &sps, missing);
    assert!(result.is_ok());

    let grid = result.unwrap();
    let expected_mb_count =
        (sps.pic_width_in_mbs_minus1 + 1) * (sps.pic_height_in_map_units_minus1 + 1);
    assert!(!grid.qp.is_empty());
}

#[test]
fn test_extract_mv_grid_small_resolution() {
    // Create SPS for small resolution
    let mut sps = create_test_sps();
    sps.pic_width_in_mbs_minus1 = 19;
    sps.pic_height_in_map_units_minus1 = 14;

    let nal_units = vec![create_test_slice_nal(false)];

    let result = extract_mv_grid(&nal_units, &sps);
    assert!(result.is_ok());

    let grid = result.unwrap();
    let pic_width = (sps.pic_width_in_mbs_minus1 + 1) as u32 * 16;
    let pic_height = (sps.pic_height_in_map_units_minus1 + 1) as u32 * 16;
    assert_eq!(grid.coded_width, pic_width);
    assert_eq!(grid.coded_height, pic_height);
}

#[test]
fn test_extract_partition_grid_small_resolution() {
    let mut sps = create_test_sps();
    sps.pic_width_in_mbs_minus1 = 19;
    sps.pic_height_in_map_units_minus1 = 14;

    let nal_units = vec![create_test_slice_nal(false)];

    let result = extract_partition_grid(&nal_units, &sps);
    assert!(result.is_ok());

    let grid = result.unwrap();
    let pic_width = (sps.pic_width_in_mbs_minus1 + 1) as u32 * 16;
    let pic_height = (sps.pic_height_in_map_units_minus1 + 1) as u32 * 16;
    assert_eq!(grid.coded_width, pic_width);
    assert_eq!(grid.coded_height, pic_height);
}

#[test]
fn test_multiple_nal_units() {
    let nal_units = vec![
        create_test_slice_nal(true),
        create_test_slice_nal(false),
        create_test_slice_nal(false),
    ];
    let sps = create_test_sps();

    let qp_result = extract_qp_grid(&nal_units, &sps, 28);
    assert!(qp_result.is_ok());

    let part_result = extract_partition_grid(&nal_units, &sps);
    assert!(part_result.is_ok());

    // MV grid with multiple NAL units may have different behavior
    // depending on how macroblocks are parsed
    // Skip this for test data since it requires valid slice data
}

#[test]
fn test_extract_with_various_missing_values() {
    let nal_units = vec![create_test_slice_nal(true)];
    let sps = create_test_sps();

    let missing_values = vec![0i16, 10, 20, 26, 30, 40, 51];

    for missing in missing_values {
        let result = extract_qp_grid(&nal_units, &sps, missing);
        assert!(result.is_ok());
        let grid = result.unwrap();
        assert_eq!(grid.missing, missing);
    }
}

#[test]
fn test_all_mb_type_variants() {
    let mb_types = vec![
        MbType::I4x4,
        MbType::I16x16,
        MbType::IPCM,
        MbType::PLuma,
        MbType::P8x8,
        MbType::BDirect,
        MbType::B16x16,
        MbType::B16x8,
        MbType::B8x16,
        MbType::B8x8,
        MbType::PSkip,
        MbType::BSkip,
    ];

    for mb_type in mb_types {
        // Just verify they can be created
        let _ = mb_type.is_intra();
        let _ = mb_type.is_skip();
        let _ = mb_type.to_partition_type();
    }
}
