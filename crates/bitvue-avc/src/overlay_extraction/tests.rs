//! Unit tests for the overlay extraction modules.

use super::grids::*;
use super::types::*;
use crate::nal::{NalUnit, NalUnitHeader};
use crate::sps::Sps;
use bitvue_engine::{mv_overlay::BlockMode, partition_grid::PartitionType};

fn create_test_sps(width: u32, height: u32) -> Sps {
    Sps {
        profile_idc: crate::sps::ProfileIdc::Baseline,
        constraint_set0_flag: false,
        constraint_set1_flag: false,
        constraint_set2_flag: false,
        constraint_set3_flag: false,
        constraint_set4_flag: false,
        constraint_set5_flag: false,
        level_idc: 40, // Level 4.0
        seq_parameter_set_id: 0,
        chroma_format_idc: crate::sps::ChromaFormat::Yuv420,
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
        pic_width_in_mbs_minus1: (width / 16).saturating_sub(1),
        pic_height_in_map_units_minus1: (height / 16).saturating_sub(1),
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

fn create_test_nal_unit(nal_type: crate::NalUnitType) -> NalUnit {
    NalUnit {
        header: NalUnitHeader {
            forbidden_zero_bit: false,
            nal_ref_idc: 0,
            nal_unit_type: nal_type,
        },
        offset: 0,
        size: 10,
        payload: vec![0; 10],
        raw_payload: vec![0; 10],
    }
}

#[test]
fn test_mb_type_is_intra() {
    assert!(MbType::I4x4.is_intra());
    assert!(MbType::I16x16.is_intra());
    assert!(MbType::IPCM.is_intra());
    assert!(!MbType::PLuma.is_intra());
    assert!(!MbType::BSkip.is_intra());
}

#[test]
fn test_mb_type_is_skip() {
    assert!(MbType::PSkip.is_skip());
    assert!(MbType::BSkip.is_skip());
    assert!(!MbType::I4x4.is_skip());
    assert!(!MbType::PLuma.is_skip());
}

#[test]
fn test_mb_type_to_partition_type() {
    assert_eq!(MbType::I16x16.to_partition_type(), PartitionType::None);
    assert_eq!(MbType::IPCM.to_partition_type(), PartitionType::None);
    assert_eq!(MbType::P8x8.to_partition_type(), PartitionType::Split);
    assert_eq!(MbType::B16x8.to_partition_type(), PartitionType::Horz);
    assert_eq!(MbType::B8x16.to_partition_type(), PartitionType::Vert);
    assert_eq!(MbType::B8x8.to_partition_type(), PartitionType::Split);
    assert_eq!(MbType::BSkip.to_partition_type(), PartitionType::None);
    assert_eq!(MbType::BDirect.to_partition_type(), PartitionType::None);
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
fn test_extract_qp_grid_empty_nal_units() {
    let sps = create_test_sps(640, 480);
    let result = extract_qp_grid(&[], &sps, 26);
    assert!(result.is_ok());
    let qp_grid = result.unwrap();
    // 640/16 * 480/16 = 40 * 30 = 1200 macroblocks
    assert_eq!(qp_grid.grid_w, 40);
    assert_eq!(qp_grid.grid_h, 30);
}

#[test]
fn test_extract_qp_grid_with_idr_slice() {
    let sps = create_test_sps(640, 480);
    let nal = create_test_nal_unit(crate::NalUnitType::IdrSlice);
    let result = extract_qp_grid(&[nal], &sps, 26);
    assert!(result.is_ok());
    let qp_grid = result.unwrap();
    assert_eq!(qp_grid.grid_w, 40);
    assert_eq!(qp_grid.grid_h, 30);
}

#[test]
fn test_extract_qp_grid_non_idr_slice() {
    let sps = create_test_sps(1920, 1080);
    let nal = create_test_nal_unit(crate::NalUnitType::NonIdrSlice);
    let result = extract_qp_grid(&[nal], &sps, 30);
    assert!(result.is_ok());
    let qp_grid = result.unwrap();
    // 1920/16 * 1080/16 = 120 * 67 = 8040 macroblocks
    assert_eq!(qp_grid.grid_w, 120);
    assert_eq!(qp_grid.grid_h, 67);
}

#[test]
fn test_extract_qp_grid_base_qp_variations() {
    let sps = create_test_sps(320, 240);
    let nal = create_test_nal_unit(crate::NalUnitType::IdrSlice);

    for base_qp in [0i16, 10, 26, 40, 51] {
        let result = extract_qp_grid(std::slice::from_ref(&nal), &sps, base_qp);
        assert!(result.is_ok());
    }
}

#[test]
fn test_extract_qp_grid_non_slice_nal() {
    let sps = create_test_sps(640, 480);
    let nal = create_test_nal_unit(crate::NalUnitType::Sps);
    let result = extract_qp_grid(&[nal], &sps, 26);
    assert!(result.is_ok());
    // Should use base_qp for all macroblocks since there's no slice
    let qp_grid = result.unwrap();
    assert_eq!(qp_grid.grid_w, 40);
    assert_eq!(qp_grid.grid_h, 30);
}

#[test]
fn test_extract_mv_grid_empty_nal_units() {
    let sps = create_test_sps(640, 480);
    let result = extract_mv_grid(&[], &sps);
    assert!(result.is_ok());
    let mv_grid = result.unwrap();
    // MV grid uses 16x16 blocks
    assert_eq!(mv_grid.coded_width, 640);
    assert_eq!(mv_grid.coded_height, 480);
}

#[test]
fn test_extract_mv_grid_with_slice() {
    let sps = create_test_sps(640, 480);
    let nal = create_test_nal_unit(crate::NalUnitType::NonIdrSlice);
    let result = extract_mv_grid(&[nal], &sps);
    assert!(result.is_ok());
    let mv_grid = result.unwrap();
    assert_eq!(mv_grid.coded_width, 640);
    assert_eq!(mv_grid.coded_height, 480);
    assert!(mv_grid.mode.is_some());
}

#[test]
fn test_extract_mv_grid_intra_slice() {
    let sps = create_test_sps(320, 240);
    let nal = create_test_nal_unit(crate::NalUnitType::IdrSlice);
    let result = extract_mv_grid(&[nal], &sps);
    assert!(result.is_ok());
    let mv_grid = result.unwrap();
    let modes = mv_grid.mode.as_ref().unwrap();
    // All blocks should be Intra for IDR slice
    assert!(modes.iter().all(|m| *m == BlockMode::Intra));
}

#[test]
fn test_extract_partition_grid_empty_nal_units() {
    let sps = create_test_sps(640, 480);
    let result = extract_partition_grid(&[], &sps);
    assert!(result.is_ok());
    let partition_grid = result.unwrap();
    assert_eq!(partition_grid.coded_width, 640);
    assert_eq!(partition_grid.coded_height, 480);
    // Should have scaffold blocks
    assert!(!partition_grid.blocks.is_empty());
}

#[test]
fn test_extract_partition_grid_with_slice() {
    let sps = create_test_sps(640, 480);
    let nal = create_test_nal_unit(crate::NalUnitType::IdrSlice);
    let result = extract_partition_grid(&[nal], &sps);
    assert!(result.is_ok());
    let partition_grid = result.unwrap();
    assert_eq!(partition_grid.coded_width, 640);
    assert_eq!(partition_grid.coded_height, 480);
}

#[test]
fn test_extract_partition_grid_inter_slice() {
    let sps = create_test_sps(1920, 1080);
    let nal = create_test_nal_unit(crate::NalUnitType::NonIdrSlice);
    let result = extract_partition_grid(&[nal], &sps);
    assert!(result.is_ok());
    let partition_grid = result.unwrap();
    assert_eq!(partition_grid.coded_width, 1920);
    // Height may be aligned to macroblock size (16)
    assert_eq!(partition_grid.coded_height, 1072);
}

#[test]
fn test_macroblock_struct() {
    let mb = Macroblock {
        mb_addr: 0,
        x: 0,
        y: 0,
        mb_type: MbType::I16x16,
        skip: false,
        qp: 26,
        mv_l0: None,
        mv_l1: None,
        ref_idx_l0: None,
        ref_idx_l1: None,
    };
    assert_eq!(mb.mb_addr, 0);
    assert_eq!(mb.qp, 26);
    assert!(!mb.skip);
    assert!(mb.mb_type.is_intra());
}

#[test]
fn test_macroblock_with_motion_vectors() {
    let mb = Macroblock {
        mb_addr: 1,
        x: 16,
        y: 0,
        mb_type: MbType::PLuma,
        skip: false,
        qp: 26,
        mv_l0: Some(MotionVector::new(4, 8)),
        mv_l1: None,
        ref_idx_l0: Some(0),
        ref_idx_l1: None,
    };
    assert_eq!(mb.mv_l0.unwrap().x, 4);
    assert_eq!(mb.mv_l0.unwrap().y, 8);
    assert_eq!(mb.ref_idx_l0.unwrap(), 0);
}

#[test]
fn test_extract_qp_grid_small_resolution() {
    let sps = create_test_sps(160, 120);
    let nal = create_test_nal_unit(crate::NalUnitType::IdrSlice);
    let result = extract_qp_grid(&[nal], &sps, 20);
    assert!(result.is_ok());
    let qp_grid = result.unwrap();
    // 160/16 * 120/16 = 10 * 7 = 70 macroblocks (round up)
    assert_eq!(qp_grid.grid_w, 10);
    assert_eq!(qp_grid.grid_h, 7);
}

#[test]
fn test_extract_mv_grid_high_resolution() {
    let sps = create_test_sps(3840, 2160);
    let nal = create_test_nal_unit(crate::NalUnitType::NonIdrSlice);
    let result = extract_mv_grid(&[nal], &sps);
    assert!(result.is_ok());
    let mv_grid = result.unwrap();
    assert_eq!(mv_grid.coded_width, 3840);
    assert_eq!(mv_grid.coded_height, 2160);
}

#[test]
fn test_extract_partition_grid_various_nal_types() {
    let sps = create_test_sps(640, 480);
    let nals = vec![
        create_test_nal_unit(crate::NalUnitType::Sps),
        create_test_nal_unit(crate::NalUnitType::Pps),
        create_test_nal_unit(crate::NalUnitType::IdrSlice),
    ];
    let result = extract_partition_grid(&nals, &sps);
    assert!(result.is_ok());
}

#[test]
fn test_mb_type_all_variants_is_intra() {
    assert!(MbType::I4x4.is_intra());
    assert!(MbType::I16x16.is_intra());
    assert!(MbType::IPCM.is_intra());
    assert!(!MbType::PLuma.is_intra());
    assert!(!MbType::P8x8.is_intra());
    assert!(!MbType::BDirect.is_intra());
    assert!(!MbType::B16x16.is_intra());
    assert!(!MbType::B16x8.is_intra());
    assert!(!MbType::B8x16.is_intra());
    assert!(!MbType::B8x8.is_intra());
    assert!(!MbType::PSkip.is_intra());
    assert!(!MbType::BSkip.is_intra());
}

#[test]
fn test_mb_type_all_variants_is_skip() {
    assert!(!MbType::I4x4.is_skip());
    assert!(!MbType::I16x16.is_skip());
    assert!(!MbType::IPCM.is_skip());
    assert!(!MbType::PLuma.is_skip());
    assert!(!MbType::P8x8.is_skip());
    assert!(!MbType::BDirect.is_skip());
    assert!(!MbType::B16x16.is_skip());
    assert!(!MbType::B16x8.is_skip());
    assert!(!MbType::B8x16.is_skip());
    assert!(!MbType::B8x8.is_skip());
    assert!(MbType::PSkip.is_skip());
    assert!(MbType::BSkip.is_skip());
}

#[test]
fn test_extract_qp_grid_all_slice_types() {
    let sps = create_test_sps(640, 480);
    let slice_types = [
        crate::NalUnitType::IdrSlice,
        crate::NalUnitType::NonIdrSlice,
        crate::NalUnitType::SliceDataA,
        crate::NalUnitType::SliceDataB,
        crate::NalUnitType::SliceDataC,
    ];
    for nal_type in slice_types {
        let nal = create_test_nal_unit(nal_type);
        let result = extract_qp_grid(&[nal], &sps, 26);
        assert!(result.is_ok(), "Failed for {:?}", nal_type);
    }
}
