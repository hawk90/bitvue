#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(unused_comparisons)]
#![allow(unused_doc_comments)]
//! AVC Slice Helper Function Tests
//!
//! Tests for slice.rs internal helper functions.

use bitvue_avc::slice::{DecRefPicMarking, RefPicListModification, SliceHeader, SliceType};
use std::collections::HashMap;

use bitvue_avc::pps::Pps;
use bitvue_avc::sps::{ChromaFormat, ProfileIdc, Sps};

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
        pic_width_in_mbs_minus1: 119,
        pic_height_in_map_units_minus1: 67,
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

/// Create a minimal PPS for testing
fn create_test_pps() -> Pps {
    Pps {
        pic_parameter_set_id: 0,
        seq_parameter_set_id: 0,
        entropy_coding_mode_flag: false,
        bottom_field_pic_order_in_frame_present_flag: false,
        num_slice_groups_minus1: 0,
        slice_group_map_type: 0,
        num_ref_idx_l0_default_active_minus1: 0,
        num_ref_idx_l1_default_active_minus1: 0,
        weighted_pred_flag: false,
        weighted_bipred_idc: 0,
        pic_init_qp_minus26: 0,
        pic_init_qs_minus26: 0,
        chroma_qp_index_offset: 0,
        deblocking_filter_control_present_flag: false,
        constrained_intra_pred_flag: false,
        redundant_pic_cnt_present_flag: false,
        transform_8x8_mode_flag: false,
        pic_scaling_matrix_present_flag: false,
        second_chroma_qp_index_offset: 0,
    }
}

#[test]
fn test_slice_type_from_u32_all_values() {
    // Test values 0-9
    assert_eq!(SliceType::from_u32(0), SliceType::P);
    assert_eq!(SliceType::from_u32(1), SliceType::B);
    assert_eq!(SliceType::from_u32(2), SliceType::I);
    assert_eq!(SliceType::from_u32(3), SliceType::Sp);
    assert_eq!(SliceType::from_u32(4), SliceType::Si);

    // Test values >= 5 (should wrap around)
    assert_eq!(SliceType::from_u32(5), SliceType::P);
    assert_eq!(SliceType::from_u32(6), SliceType::B);
    assert_eq!(SliceType::from_u32(7), SliceType::I);
    assert_eq!(SliceType::from_u32(8), SliceType::Sp);
    assert_eq!(SliceType::from_u32(9), SliceType::Si);
    assert_eq!(SliceType::from_u32(10), SliceType::P);
}

#[test]
fn test_slice_type_is_intra() {
    assert!(SliceType::I.is_intra());
    assert!(SliceType::Si.is_intra());
    assert!(!SliceType::P.is_intra());
    assert!(!SliceType::B.is_intra());
    assert!(!SliceType::Sp.is_intra());
}

#[test]
fn test_slice_type_name() {
    assert_eq!(SliceType::P.name(), "P");
    assert_eq!(SliceType::B.name(), "B");
    assert_eq!(SliceType::I.name(), "I");
    assert_eq!(SliceType::Sp.name(), "SP");
    assert_eq!(SliceType::Si.name(), "SI");
}

#[test]
fn test_slice_type_all_types() {
    // Test all slice types are covered
    assert_eq!(SliceType::P.name(), "P");
    assert_eq!(SliceType::B.name(), "B");
    assert_eq!(SliceType::I.name(), "I");
    assert_eq!(SliceType::Sp.name(), "SP");
    assert_eq!(SliceType::Si.name(), "SI");
}

#[test]
fn test_slice_header_qp_calc() {
    let header = SliceHeader {
        first_mb_in_slice: 0,
        slice_type: SliceType::I,
        pic_parameter_set_id: 0,
        frame_num: 0,
        slice_qp_delta: 0,
        cabac_init_idc: 0,
        disable_deblocking_filter_idc: 0,
        slice_alpha_c0_offset_div2: 0,
        slice_beta_offset_div2: 0,
        num_ref_idx_l0_active_minus1: 0,
        num_ref_idx_l1_active_minus1: 0,
        ref_pic_list_modification_l0: RefPicListModification {
            modifications: vec![],
        },
        ref_pic_list_modification_l1: RefPicListModification {
            modifications: vec![],
        },
        dec_ref_pic_marking: DecRefPicMarking {
            no_output_of_prior_pics_flag: false,
            long_term_reference_flag: false,
            adaptive_ref_pic_marking_mode_flag: false,
            mmco_operations: vec![],
        },
        colour_plane_id: 0,
        field_pic_flag: false,
        bottom_field_flag: false,
        idr_pic_id: 0,
        pic_order_cnt_lsb: 0,
        delta_pic_order_cnt_bottom: 0,
        delta_pic_order_cnt: [0, 0],
        redundant_pic_cnt: 0,
        direct_spatial_mv_pred_flag: false,
        num_ref_idx_active_override_flag: false,
        ref_pic_list_modification_flag_l0: false,
        ref_pic_list_modification_flag_l1: false,
        sp_for_switch_flag: false,
        slice_qs_delta: 0,
        slice_group_change_cycle: 0,
    };

    let pps = create_test_pps();

    // QP = 26 + pic_init_qp_minus26 + slice_qp_delta
    // Default slice_qp_delta is 0
    let qp = header.qp(&pps);
    assert_eq!(qp, 26);
}

#[test]
fn test_slice_header_qp_with_delta() {
    let header = SliceHeader {
        first_mb_in_slice: 0,
        slice_type: SliceType::P,
        pic_parameter_set_id: 0,
        frame_num: 1,
        slice_qp_delta: 5,
        cabac_init_idc: 0,
        disable_deblocking_filter_idc: 0,
        slice_alpha_c0_offset_div2: 0,
        slice_beta_offset_div2: 0,
        num_ref_idx_l0_active_minus1: 0,
        num_ref_idx_l1_active_minus1: 0,
        ref_pic_list_modification_l0: RefPicListModification {
            modifications: vec![],
        },
        ref_pic_list_modification_l1: RefPicListModification {
            modifications: vec![],
        },
        dec_ref_pic_marking: DecRefPicMarking {
            no_output_of_prior_pics_flag: false,
            long_term_reference_flag: false,
            adaptive_ref_pic_marking_mode_flag: false,
            mmco_operations: vec![],
        },
        colour_plane_id: 0,
        field_pic_flag: false,
        bottom_field_flag: false,
        idr_pic_id: 0,
        pic_order_cnt_lsb: 0,
        delta_pic_order_cnt_bottom: 0,
        delta_pic_order_cnt: [0, 0],
        redundant_pic_cnt: 0,
        direct_spatial_mv_pred_flag: false,
        num_ref_idx_active_override_flag: false,
        ref_pic_list_modification_flag_l0: false,
        ref_pic_list_modification_flag_l1: false,
        sp_for_switch_flag: false,
        slice_qs_delta: 0,
        slice_group_change_cycle: 0,
    };

    let pps = create_test_pps();

    let qp = header.qp(&pps);
    assert_eq!(qp, 31); // 26 + 0 + 5
}

#[test]
fn test_slice_header_is_first_slice() {
    let header = SliceHeader {
        first_mb_in_slice: 0,
        slice_type: SliceType::I,
        pic_parameter_set_id: 0,
        frame_num: 0,
        slice_qp_delta: 0,
        cabac_init_idc: 0,
        disable_deblocking_filter_idc: 0,
        slice_alpha_c0_offset_div2: 0,
        slice_beta_offset_div2: 0,
        num_ref_idx_l0_active_minus1: 0,
        num_ref_idx_l1_active_minus1: 0,
        ref_pic_list_modification_l0: RefPicListModification {
            modifications: vec![],
        },
        ref_pic_list_modification_l1: RefPicListModification {
            modifications: vec![],
        },
        dec_ref_pic_marking: DecRefPicMarking {
            no_output_of_prior_pics_flag: false,
            long_term_reference_flag: false,
            adaptive_ref_pic_marking_mode_flag: false,
            mmco_operations: vec![],
        },
        colour_plane_id: 0,
        field_pic_flag: false,
        bottom_field_flag: false,
        idr_pic_id: 0,
        pic_order_cnt_lsb: 0,
        delta_pic_order_cnt_bottom: 0,
        delta_pic_order_cnt: [0, 0],
        redundant_pic_cnt: 0,
        direct_spatial_mv_pred_flag: false,
        num_ref_idx_active_override_flag: false,
        ref_pic_list_modification_flag_l0: false,
        ref_pic_list_modification_flag_l1: false,
        sp_for_switch_flag: false,
        slice_qs_delta: 0,
        slice_group_change_cycle: 0,
    };

    assert!(header.is_first_slice());
}

#[test]
fn test_slice_header_is_not_first_slice() {
    let header = SliceHeader {
        first_mb_in_slice: 10,
        slice_type: SliceType::P,
        pic_parameter_set_id: 0,
        frame_num: 1,
        slice_qp_delta: 0,
        cabac_init_idc: 0,
        disable_deblocking_filter_idc: 0,
        slice_alpha_c0_offset_div2: 0,
        slice_beta_offset_div2: 0,
        num_ref_idx_l0_active_minus1: 0,
        num_ref_idx_l1_active_minus1: 0,
        ref_pic_list_modification_l0: RefPicListModification {
            modifications: vec![],
        },
        ref_pic_list_modification_l1: RefPicListModification {
            modifications: vec![],
        },
        dec_ref_pic_marking: DecRefPicMarking {
            no_output_of_prior_pics_flag: false,
            long_term_reference_flag: false,
            adaptive_ref_pic_marking_mode_flag: false,
            mmco_operations: vec![],
        },
        colour_plane_id: 0,
        field_pic_flag: false,
        bottom_field_flag: false,
        idr_pic_id: 0,
        pic_order_cnt_lsb: 0,
        delta_pic_order_cnt_bottom: 0,
        delta_pic_order_cnt: [0, 0],
        redundant_pic_cnt: 0,
        direct_spatial_mv_pred_flag: false,
        num_ref_idx_active_override_flag: false,
        ref_pic_list_modification_flag_l0: false,
        ref_pic_list_modification_flag_l1: false,
        sp_for_switch_flag: false,
        slice_qs_delta: 0,
        slice_group_change_cycle: 0,
    };

    assert!(!header.is_first_slice());
}

#[test]
fn test_slice_header_first_mb_field() {
    let header = SliceHeader {
        first_mb_in_slice: 0,
        slice_type: SliceType::I,
        pic_parameter_set_id: 0,
        frame_num: 0,
        slice_qp_delta: 0,
        cabac_init_idc: 0,
        disable_deblocking_filter_idc: 0,
        slice_alpha_c0_offset_div2: 0,
        slice_beta_offset_div2: 0,
        num_ref_idx_l0_active_minus1: 0,
        num_ref_idx_l1_active_minus1: 0,
        ref_pic_list_modification_l0: RefPicListModification {
            modifications: vec![],
        },
        ref_pic_list_modification_l1: RefPicListModification {
            modifications: vec![],
        },
        dec_ref_pic_marking: DecRefPicMarking {
            no_output_of_prior_pics_flag: false,
            long_term_reference_flag: false,
            adaptive_ref_pic_marking_mode_flag: false,
            mmco_operations: vec![],
        },
        colour_plane_id: 0,
        field_pic_flag: false,
        bottom_field_flag: false,
        idr_pic_id: 0,
        pic_order_cnt_lsb: 0,
        delta_pic_order_cnt_bottom: 0,
        delta_pic_order_cnt: [0, 0],
        redundant_pic_cnt: 0,
        direct_spatial_mv_pred_flag: false,
        num_ref_idx_active_override_flag: false,
        ref_pic_list_modification_flag_l0: false,
        ref_pic_list_modification_flag_l1: false,
        sp_for_switch_flag: false,
        slice_qs_delta: 0,
        slice_group_change_cycle: 0,
    };

    assert_eq!(header.first_mb_in_slice, 0);
}

#[test]
fn test_dec_ref_pic_marking_structure() {
    let marking = DecRefPicMarking {
        no_output_of_prior_pics_flag: false,
        long_term_reference_flag: false,
        adaptive_ref_pic_marking_mode_flag: false,
        mmco_operations: vec![(1, 2, 0), (3, 0, 1)],
    };

    assert_eq!(marking.mmco_operations.len(), 2);
    assert_eq!(marking.mmco_operations[0], (1, 2, 0));
}

#[test]
fn test_ref_pic_list_modification_structure() {
    let modification = RefPicListModification {
        modifications: vec![(3, 5), (1, 10)],
    };

    assert_eq!(modification.modifications.len(), 2);
    assert_eq!(modification.modifications[0], (3, 5));
}

#[test]
fn test_sps_dimensions() {
    let sps = create_test_sps();

    // pic_width = (pic_width_in_mbs_minus1 + 1) * 16
    assert_eq!(sps.pic_width(), 1920); // (119 + 1) * 16
                                       // pic_height = (pic_height_in_map_units_minus1 + 1) * 16
    assert_eq!(sps.pic_height(), 1088); // (67 + 1) * 16
}

#[test]
fn test_sps_chroma_format() {
    let sps = create_test_sps();
    assert_eq!(sps.chroma_format_idc, ChromaFormat::Yuv420);
}

#[test]
fn test_sps_profile() {
    let sps = create_test_sps();
    assert_eq!(sps.profile_idc, ProfileIdc::High);
}

#[test]
fn test_pps_initial_qp() {
    let pps = create_test_pps();
    // QP = 26 + pic_init_qp_minus26
    assert_eq!(pps.initial_qp(), 26); // 26 + 0
}

#[test]
fn test_pps_is_cabac() {
    let pps = Pps {
        entropy_coding_mode_flag: true,
        ..create_test_pps()
    };

    assert!(pps.is_cabac());
}

#[test]
fn test_pps_is_not_cabac() {
    let pps = create_test_pps();
    assert!(!pps.is_cabac());
}

#[test]
fn test_slice_type_values() {
    // Test all slice types have correct numeric representations
    assert_eq!(SliceType::P as u8, 0);
    assert_eq!(SliceType::B as u8, 1);
    assert_eq!(SliceType::I as u8, 2);
    assert_eq!(SliceType::Sp as u8, 3);
    assert_eq!(SliceType::Si as u8, 4);
}

#[test]
fn test_profile_idc_values() {
    assert_eq!(ProfileIdc::Baseline as u8, 66);
    assert_eq!(ProfileIdc::Main as u8, 77);
    assert_eq!(ProfileIdc::Extended as u8, 88);
    assert_eq!(ProfileIdc::High as u8, 100);
}

#[test]
fn test_chroma_format_values() {
    assert_eq!(ChromaFormat::Monochrome as u8, 0);
    assert_eq!(ChromaFormat::Yuv420 as u8, 1);
    assert_eq!(ChromaFormat::Yuv422 as u8, 2);
    assert_eq!(ChromaFormat::Yuv444 as u8, 3);
}

#[test]
fn test_sps_bit_depth() {
    let sps = create_test_sps();
    assert_eq!(sps.bit_depth_luma(), 8); // 0 + 8
    assert_eq!(sps.bit_depth_chroma(), 8); // 0 + 8
}

#[test]
fn test_sps_with_different_bit_depth() {
    let mut sps = create_test_sps();
    sps.bit_depth_luma_minus8 = 2;
    sps.bit_depth_chroma_minus8 = 2;

    assert_eq!(sps.bit_depth_luma(), 10); // 2 + 8
    assert_eq!(sps.bit_depth_chroma(), 10); // 2 + 8
}

#[test]
fn test_sps_display_dimensions_no_crop() {
    let sps = create_test_sps();
    // With frame_cropping_flag = false, display dimensions = coded dimensions
    assert_eq!(sps.display_width(), 1920);
    assert_eq!(sps.display_height(), 1088);
}

#[test]
fn test_profile_is_high_profile() {
    assert!(ProfileIdc::High.is_high_profile());
    assert!(ProfileIdc::High10.is_high_profile());
    assert!(!ProfileIdc::Baseline.is_high_profile());
    assert!(!ProfileIdc::Main.is_high_profile());
}

#[test]
fn test_sps_max_ref_frames() {
    let sps = create_test_sps();
    assert_eq!(sps.max_num_ref_frames, 1);
}
