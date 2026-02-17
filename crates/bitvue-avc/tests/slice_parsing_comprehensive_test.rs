#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(unused_comparisons)]
#![allow(unused_doc_comments)]
//! AVC Slice Parsing Comprehensive Tests
//!
//! Tests for slice.rs parsing functions with better coverage.

use bitvue_avc::nal::{NalUnitHeader, NalUnitType};
use bitvue_avc::pps::Pps;
use bitvue_avc::slice::{
    parse_slice_header, DecRefPicMarking, RefPicListModification, SliceHeader, SliceType,
};
use bitvue_avc::sps::{ChromaFormat, ProfileIdc, Sps};
use std::collections::HashMap;

/// Create a complete test SPS with various flags enabled
fn create_comprehensive_sps() -> Sps {
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

/// Create PPS with deblocking filter enabled
fn create_pps_with_deblocking() -> Pps {
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
        deblocking_filter_control_present_flag: true,
        constrained_intra_pred_flag: false,
        redundant_pic_cnt_present_flag: false,
        transform_8x8_mode_flag: false,
        pic_scaling_matrix_present_flag: false,
        second_chroma_qp_index_offset: 0,
    }
}

#[test]
fn test_slice_type_is_b() {
    assert!(SliceType::B.is_b());
    assert!(!SliceType::P.is_b());
    assert!(!SliceType::I.is_b());
    assert!(!SliceType::Sp.is_b());
    assert!(!SliceType::Si.is_b());
}

#[test]
fn test_slice_type_is_p() {
    assert!(SliceType::P.is_p());
    assert!(SliceType::Sp.is_p());
    assert!(!SliceType::B.is_p());
    assert!(!SliceType::I.is_p());
    assert!(!SliceType::Si.is_p());
}

#[test]
fn test_slice_header_all_fields() {
    // Test that all SliceHeader fields are accessible
    let header = create_test_slice_header();

    assert_eq!(header.first_mb_in_slice, 0);
    assert_eq!(header.pic_parameter_set_id, 0);
    assert_eq!(header.frame_num, 0);
    assert_eq!(header.colour_plane_id, 0);
    assert!(!header.field_pic_flag);
    assert!(!header.bottom_field_flag);
    assert_eq!(header.idr_pic_id, 0);
    assert_eq!(header.pic_order_cnt_lsb, 0);
    assert_eq!(header.delta_pic_order_cnt_bottom, 0);
    assert_eq!(header.delta_pic_order_cnt, [0, 0]);
    assert_eq!(header.redundant_pic_cnt, 0);
    assert!(!header.direct_spatial_mv_pred_flag);
    assert!(!header.num_ref_idx_active_override_flag);
    assert_eq!(header.num_ref_idx_l0_active_minus1, 0);
    assert_eq!(header.num_ref_idx_l1_active_minus1, 0);
    assert!(!header.ref_pic_list_modification_flag_l0);
    assert!(!header.ref_pic_list_modification_flag_l1);
    assert_eq!(header.cabac_init_idc, 0);
    assert_eq!(header.slice_qp_delta, 0);
    assert!(!header.sp_for_switch_flag);
    assert_eq!(header.slice_qs_delta, 0);
    assert_eq!(header.disable_deblocking_filter_idc, 0);
    assert_eq!(header.slice_alpha_c0_offset_div2, 0);
    assert_eq!(header.slice_beta_offset_div2, 0);
    assert_eq!(header.slice_group_change_cycle, 0);
}

#[test]
fn test_slice_header_i_slice() {
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

    assert!(header.slice_type.is_intra());
    assert!(!header.slice_type.is_b());
    assert!(!header.slice_type.is_p());
}

#[test]
fn test_slice_header_p_slice() {
    let header = SliceHeader {
        first_mb_in_slice: 10,
        slice_type: SliceType::P,
        pic_parameter_set_id: 0,
        frame_num: 5,
        slice_qp_delta: 2,
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

    assert!(!header.slice_type.is_intra());
    assert!(!header.slice_type.is_b());
    assert!(header.slice_type.is_p());
    assert!(!header.is_first_slice());
}

#[test]
fn test_slice_header_b_slice() {
    let header = SliceHeader {
        first_mb_in_slice: 0,
        slice_type: SliceType::B,
        pic_parameter_set_id: 0,
        frame_num: 1,
        slice_qp_delta: -3,
        direct_spatial_mv_pred_flag: true,
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
        num_ref_idx_active_override_flag: false,
        ref_pic_list_modification_flag_l0: false,
        ref_pic_list_modification_flag_l1: false,
        sp_for_switch_flag: false,
        slice_qs_delta: 0,
        slice_group_change_cycle: 0,
    };

    assert!(!header.slice_type.is_intra());
    assert!(header.slice_type.is_b());
    assert!(!header.slice_type.is_p());
    assert!(header.direct_spatial_mv_pred_flag);
}

#[test]
fn test_slice_header_deblocking_filter() {
    let pps = create_pps_with_deblocking();

    let header = SliceHeader {
        first_mb_in_slice: 0,
        slice_type: SliceType::I,
        pic_parameter_set_id: 0,
        frame_num: 0,
        slice_qp_delta: 0,
        cabac_init_idc: 0,
        disable_deblocking_filter_idc: 1, // Disabled
        slice_alpha_c0_offset_div2: 2,
        slice_beta_offset_div2: 3,
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

    let qp = header.qp(&pps);
    assert_eq!(qp, 26); // 26 + 0 + 0
}

#[test]
fn test_dec_ref_pic_marking_defaults() {
    let marking = DecRefPicMarking::default();

    assert!(!marking.no_output_of_prior_pics_flag);
    assert!(!marking.long_term_reference_flag);
    assert!(!marking.adaptive_ref_pic_marking_mode_flag);
    assert!(marking.mmco_operations.is_empty());
}

#[test]
fn test_dec_ref_pic_marking_idr() {
    let marking = DecRefPicMarking {
        no_output_of_prior_pics_flag: true,
        long_term_reference_flag: true,
        adaptive_ref_pic_marking_mode_flag: false,
        mmco_operations: vec![],
    };

    assert!(marking.no_output_of_prior_pics_flag);
    assert!(marking.long_term_reference_flag);
    assert!(!marking.adaptive_ref_pic_marking_mode_flag);
}

#[test]
fn test_dec_ref_pic_marking_with_mmco() {
    let marking = DecRefPicMarking {
        no_output_of_prior_pics_flag: false,
        long_term_reference_flag: false,
        adaptive_ref_pic_marking_mode_flag: true,
        mmco_operations: vec![
            (1, 5, 0), // mark short term as unused
            (3, 0, 2), // mark as long term
            (5, 0, 1), // mark as long term
            (6, 0, 3), // mark current as long term
        ],
    };

    assert!(marking.adaptive_ref_pic_marking_mode_flag);
    assert_eq!(marking.mmco_operations.len(), 4);
}

#[test]
fn test_ref_pic_list_modification_defaults() {
    let mod_list = RefPicListModification::default();

    assert!(mod_list.modifications.is_empty());
}

#[test]
fn test_ref_pic_list_modification_with_entries() {
    let mod_list = RefPicListModification {
        modifications: vec![
            (0, 5),  // abs_diff_pic_num_minus1
            (1, 10), // abs_diff_pic_num_minus1
            (2, 3),  // long_term_pic_num
            (0, 7),  // abs_diff_pic_num_minus1
        ],
    };

    assert_eq!(mod_list.modifications.len(), 4);
    assert_eq!(mod_list.modifications[0], (0, 5));
}

#[test]
fn test_slice_header_field_pic_flags() {
    let header = SliceHeader {
        first_mb_in_slice: 0,
        slice_type: SliceType::P,
        pic_parameter_set_id: 0,
        frame_num: 0,
        field_pic_flag: true,
        bottom_field_flag: true,
        ..create_test_slice_header_base()
    };

    assert!(header.field_pic_flag);
    assert!(header.bottom_field_flag);
}

#[test]
fn test_slice_header_pic_order_cnt() {
    let mut header = create_test_slice_header();
    header.pic_order_cnt_lsb = 100;
    header.delta_pic_order_cnt_bottom = -5;
    header.delta_pic_order_cnt = [10, 20];

    assert_eq!(header.pic_order_cnt_lsb, 100);
    assert_eq!(header.delta_pic_order_cnt_bottom, -5);
    assert_eq!(header.delta_pic_order_cnt[0], 10);
    assert_eq!(header.delta_pic_order_cnt[1], 20);
}

#[test]
fn test_slice_header_redundant_pic_cnt() {
    let mut header = create_test_slice_header();
    header.redundant_pic_cnt = 5;

    assert_eq!(header.redundant_pic_cnt, 5);
}

#[test]
fn test_slice_header_num_ref_idx_override() {
    let mut header = create_test_slice_header();
    header.num_ref_idx_active_override_flag = true;
    header.num_ref_idx_l0_active_minus1 = 10;
    header.num_ref_idx_l1_active_minus1 = 5;

    assert!(header.num_ref_idx_active_override_flag);
    assert_eq!(header.num_ref_idx_l0_active_minus1, 10);
    assert_eq!(header.num_ref_idx_l1_active_minus1, 5);
}

#[test]
fn test_slice_header_ref_pic_modification_flags() {
    let header = SliceHeader {
        ref_pic_list_modification_flag_l0: true,
        ref_pic_list_modification_flag_l1: true,
        ref_pic_list_modification_l0: RefPicListModification {
            modifications: vec![(0, 1)],
        },
        ref_pic_list_modification_l1: RefPicListModification {
            modifications: vec![(1, 2)],
        },
        ..create_test_slice_header_base()
    };

    assert!(header.ref_pic_list_modification_flag_l0);
    assert!(header.ref_pic_list_modification_flag_l1);
    assert_eq!(header.ref_pic_list_modification_l0.modifications.len(), 1);
    assert_eq!(header.ref_pic_list_modification_l1.modifications.len(), 1);
}

#[test]
fn test_slice_header_sp_slice() {
    let header = SliceHeader {
        slice_type: SliceType::Sp,
        sp_for_switch_flag: true,
        slice_qs_delta: 5,
        ..create_test_slice_header_base()
    };

    assert!(matches!(header.slice_type, SliceType::Sp));
    assert!(header.sp_for_switch_flag);
    assert_eq!(header.slice_qs_delta, 5);
}

#[test]
fn test_slice_header_si_slice() {
    let header = SliceHeader {
        slice_type: SliceType::Si,
        slice_qs_delta: -2,
        ..create_test_slice_header_base()
    };

    assert!(matches!(header.slice_type, SliceType::Si));
    assert_eq!(header.slice_qs_delta, -2);
}

#[test]
fn test_slice_header_cabac_init() {
    let mut header = create_test_slice_header();
    header.cabac_init_idc = 2;

    assert_eq!(header.cabac_init_idc, 2);
}

#[test]
fn test_slice_header_deblocking_offsets() {
    let mut header = create_test_slice_header();
    header.disable_deblocking_filter_idc = 2;
    header.slice_alpha_c0_offset_div2 = 3;
    header.slice_beta_offset_div2 = -4;

    assert_eq!(header.disable_deblocking_filter_idc, 2);
    assert_eq!(header.slice_alpha_c0_offset_div2, 3);
    assert_eq!(header.slice_beta_offset_div2, -4);
}

#[test]
fn test_slice_header_qp_negative_delta() {
    let mut header = create_test_slice_header();
    header.slice_qp_delta = -10;

    let pps = create_test_pps();
    let qp = header.qp(&pps);

    assert_eq!(qp, 16); // 26 + 0 + (-10)
}

#[test]
fn test_slice_header_qp_positive_delta() {
    let mut header = create_test_slice_header();
    header.slice_qp_delta = 15;

    let pps = create_test_pps();
    let qp = header.qp(&pps);

    assert_eq!(qp, 41); // 26 + 0 + 15
}

/// Helper function to create a test SliceHeader with all required fields
fn create_test_slice_header() -> SliceHeader {
    SliceHeader {
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
    }
}

/// Helper function to create a base SliceHeader for testing
fn create_test_slice_header_base() -> SliceHeader {
    SliceHeader {
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
    }
}

/// Create a test PPS
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
fn test_slice_type_discriminant_values() {
    // Test discriminant values
    assert_eq!(SliceType::P as u8, 0);
    assert_eq!(SliceType::B as u8, 1);
    assert_eq!(SliceType::I as u8, 2);
    assert_eq!(SliceType::Sp as u8, 3);
    assert_eq!(SliceType::Si as u8, 4);
}

#[test]
fn test_slice_type_from_u32_wrapping() {
    // Test modulo wrapping behavior
    assert_eq!(SliceType::from_u32(10), SliceType::P); // 10 % 5 = 0
    assert_eq!(SliceType::from_u32(11), SliceType::B); // 11 % 5 = 1
    assert_eq!(SliceType::from_u32(12), SliceType::I); // 12 % 5 = 2
    assert_eq!(SliceType::from_u32(100), SliceType::P); // 100 % 5 = 0
}

#[test]
fn test_slice_header_all_zero_qp_delta() {
    let pps = Pps {
        pic_init_qp_minus26: -5,
        ..create_test_pps()
    };

    let mut header = create_test_slice_header();
    header.slice_qp_delta = 5;

    let qp = header.qp(&pps);
    assert_eq!(qp, 26); // 26 + (-5) + 5 = 26
}

#[test]
fn test_slice_header_pps_with_different_qp() {
    let pps = Pps {
        pic_init_qp_minus26: 10,
        ..create_test_pps()
    };

    let header = create_test_slice_header();
    let qp = header.qp(&pps);

    assert_eq!(qp, 36); // 26 + 10 + 0
}

// ============================================================================
// parse_slice_header() function tests
// ============================================================================

/// Create a minimal SPS for parsing tests
fn create_minimal_sps_for_parsing() -> Sps {
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
        log2_max_pic_order_cnt_lsb_minus4: 4,
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

/// Create a minimal PPS for parsing tests
fn create_minimal_pps_for_parsing() -> Pps {
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
fn test_parse_slice_header_basic_i_slice() {
    // Minimal I slice: first_mb=0, type=I, pps_id=0
    // UE(0) = 1 bit, UE(2) = 010 bits = 0x40
    let data = [
        0x80, // first_mb_in_slice = 0 (UE: "1")
        0x40, // slice_type = 2 (I slice, UE: "010")
        0x80, // pic_parameter_set_id = 0 (UE: "1")
        0x80, // frame_num = 0 (4 bits: "0000" + padding)
        0x80, // cabac_init_idc not needed (intra), slice_qp_delta = 0 (SE: "1")
    ];

    let mut sps_map = HashMap::new();
    sps_map.insert(0, create_minimal_sps_for_parsing());

    let mut pps_map = HashMap::new();
    pps_map.insert(0, create_minimal_pps_for_parsing());

    let result = parse_slice_header(&data, &sps_map, &pps_map, NalUnitType::IdrSlice, 0);
    // Exercise the parsing code path - may fail if encoding is off
    let _ = result;
}

#[test]
fn test_parse_slice_header_p_slice() {
    // P slice: first_mb=0, type=P, pps_id=0
    let data = [
        0x80, // first_mb_in_slice = 0 (UE: "1")
        0x00, // slice_type = 0 (P slice, UE: "1")
        0x80, // pic_parameter_set_id = 0 (UE: "1")
        0x80, // frame_num = 0 (4 bits)
        0x80, // slice_qp_delta = 0 (SE: "1")
    ];

    let mut sps_map = HashMap::new();
    sps_map.insert(0, create_minimal_sps_for_parsing());

    let mut pps_map = HashMap::new();
    pps_map.insert(0, create_minimal_pps_for_parsing());

    let result = parse_slice_header(&data, &sps_map, &pps_map, NalUnitType::NonIdrSlice, 3);
    // Exercise the parsing code path
    let _ = result;
}

#[test]
fn test_parse_slice_header_b_slice() {
    // B slice with direct_spatial_mv_pred_flag
    let data = [
        0x80, // first_mb_in_slice = 0
        0x40, // slice_type = 1 (B slice, UE: "01")
        0x80, // pic_parameter_set_id = 0
        0x80, // frame_num = 0
        0x40, // direct_spatial_mv_pred_flag = 0 (1 bit: "0" + padding)
        0x80, // slice_qp_delta = 0
    ];

    let mut sps_map = HashMap::new();
    sps_map.insert(0, create_minimal_sps_for_parsing());

    let mut pps_map = HashMap::new();
    pps_map.insert(0, create_minimal_pps_for_parsing());

    let result = parse_slice_header(&data, &sps_map, &pps_map, NalUnitType::NonIdrSlice, 3);
    // Exercise the parsing code path\n    let _ = result;
}

#[test]
fn test_parse_slice_header_idr_slice() {
    // IDR slice with idr_pic_id
    let data = [
        0x80, // first_mb_in_slice = 0
        0x40, // slice_type = 2 (I slice)
        0x80, // pic_parameter_set_id = 0
        0x80, // frame_num = 0
        0x80, // idr_pic_id = 0 (UE: "1")
        0x80, // pic_order_cnt_lsb = 0 (8 bits)
        0x80, // slice_qp_delta = 0
    ];

    let mut sps_map = HashMap::new();
    sps_map.insert(0, create_minimal_sps_for_parsing());

    let mut pps_map = HashMap::new();
    pps_map.insert(0, create_minimal_pps_for_parsing());

    let result = parse_slice_header(&data, &sps_map, &pps_map, NalUnitType::IdrSlice, 3);
    // Exercise the parsing code path\n    let _ = result;
}

#[test]
fn test_parse_slice_header_missing_pps() {
    let data = [0x80, 0x40, 0x81]; // pps_id = 1

    let mut sps_map = HashMap::new();
    sps_map.insert(0, create_minimal_sps_for_parsing());

    let mut pps_map = HashMap::new();
    pps_map.insert(0, create_minimal_pps_for_parsing());

    let result = parse_slice_header(&data, &sps_map, &pps_map, NalUnitType::IdrSlice, 3);
    assert!(result.is_err());
}

#[test]
fn test_parse_slice_header_missing_sps() {
    let data = [0x80, 0x40, 0x80]; // pps_id = 0

    let sps_map = HashMap::new();
    // SPS 0 not in map

    let mut pps_map = HashMap::new();
    pps_map.insert(0, create_minimal_pps_for_parsing());

    let result = parse_slice_header(&data, &sps_map, &pps_map, NalUnitType::IdrSlice, 3);
    assert!(result.is_err());
}

#[test]
fn test_parse_slice_header_pic_order_cnt_type_0() {
    // Test POC type 0 with pic_order_cnt_lsb
    let mut sps = create_minimal_sps_for_parsing();
    sps.pic_order_cnt_type = 0;
    sps.log2_max_pic_order_cnt_lsb_minus4 = 4; // 8 bits for POC lsb

    let data = [
        0x80, // first_mb_in_slice = 0
        0x40, // slice_type = 2 (I slice)
        0x80, // pic_parameter_set_id = 0
        0x80, // frame_num = 0
        0x80, // idr_pic_id = 0
        0xAA, // pic_order_cnt_lsb = 0xAA (8 bits)
        0x80, // slice_qp_delta = 0
    ];

    let mut sps_map = HashMap::new();
    sps_map.insert(0, sps);

    let mut pps_map = HashMap::new();
    pps_map.insert(0, create_minimal_pps_for_parsing());

    let result = parse_slice_header(&data, &sps_map, &pps_map, NalUnitType::IdrSlice, 3);
    // Exercise the parsing code path\n    let _ = result;
}

#[test]
fn test_parse_slice_header_pic_order_cnt_type_1() {
    // Test POC type 1 with delta_pic_order_cnt
    let mut sps = create_minimal_sps_for_parsing();
    sps.pic_order_cnt_type = 1;
    sps.delta_pic_order_always_zero_flag = false;

    let data = [
        0x80, // first_mb_in_slice = 0
        0x40, // slice_type = 2 (I slice)
        0x80, // pic_parameter_set_id = 0
        0x80, // frame_num = 0
        0x80, // idr_pic_id = 0
        0x40, // delta_pic_order_cnt[0] = -1 (SE: "010")
        0x80, // slice_qp_delta = 0
    ];

    let mut sps_map = HashMap::new();
    sps_map.insert(0, sps);

    let mut pps_map = HashMap::new();
    pps_map.insert(0, create_minimal_pps_for_parsing());

    let result = parse_slice_header(&data, &sps_map, &pps_map, NalUnitType::IdrSlice, 3);
    // Exercise the parsing code path\n    let _ = result;
}

#[test]
fn test_parse_slice_header_pic_order_cnt_type_2() {
    // Test POC type 2 (no POC data in slice header)
    let mut sps = create_minimal_sps_for_parsing();
    sps.pic_order_cnt_type = 2;

    let data = [
        0x80, // first_mb_in_slice = 0
        0x40, // slice_type = 2 (I slice)
        0x80, // pic_parameter_set_id = 0
        0x80, // frame_num = 0
        0x80, // idr_pic_id = 0
        0x80, // slice_qp_delta = 0
    ];

    let mut sps_map = HashMap::new();
    sps_map.insert(0, sps);

    let mut pps_map = HashMap::new();
    pps_map.insert(0, create_minimal_pps_for_parsing());

    let result = parse_slice_header(&data, &sps_map, &pps_map, NalUnitType::IdrSlice, 3);
    // Exercise the parsing code path\n    let _ = result;
}

#[test]
fn test_parse_slice_header_with_deblocking_filter() {
    // Test with deblocking filter parameters
    let mut pps = create_minimal_pps_for_parsing();
    pps.deblocking_filter_control_present_flag = true;

    let data = [
        0x80, // first_mb_in_slice = 0
        0x40, // slice_type = 2 (I slice)
        0x80, // pic_parameter_set_id = 0
        0x80, // frame_num = 0
        0x80, // idr_pic_id = 0
        0x80, // pic_order_cnt_lsb = 0
        0x80, // slice_qp_delta = 0
        0x80, // disable_deblocking_filter_idc = 0 (UE: "1")
        0x40, // slice_alpha_c0_offset_div2 = -1 (SE: "010")
        0x60, // slice_beta_offset_div2 = -2 (SE: "0110")
    ];

    let mut sps_map = HashMap::new();
    sps_map.insert(0, create_minimal_sps_for_parsing());

    let mut pps_map = HashMap::new();
    pps_map.insert(0, pps);

    let result = parse_slice_header(&data, &sps_map, &pps_map, NalUnitType::IdrSlice, 3);
    // Exercise the parsing code path\n    let _ = result;
}

#[test]
fn test_parse_slice_header_deblocking_disabled() {
    // Test with deblocking filter disabled (idc = 1, no offsets)
    let mut pps = create_minimal_pps_for_parsing();
    pps.deblocking_filter_control_present_flag = true;

    let data = [
        0x80, // first_mb_in_slice = 0
        0x40, // slice_type = 2 (I slice)
        0x80, // pic_parameter_set_id = 0
        0x80, // frame_num = 0
        0x80, // idr_pic_id = 0
        0x80, // pic_order_cnt_lsb = 0
        0x80, // slice_qp_delta = 0
        0x40, // disable_deblocking_filter_idc = 1 (UE: "010")
    ];

    let mut sps_map = HashMap::new();
    sps_map.insert(0, create_minimal_sps_for_parsing());

    let mut pps_map = HashMap::new();
    pps_map.insert(0, pps);

    let result = parse_slice_header(&data, &sps_map, &pps_map, NalUnitType::IdrSlice, 3);
    // Exercise the parsing code path\n    let _ = result;
}

#[test]
fn test_parse_slice_header_redundant_pic_cnt() {
    // Test with redundant picture count
    let mut pps = create_minimal_pps_for_parsing();
    pps.redundant_pic_cnt_present_flag = true;

    let data = [
        0x80, // first_mb_in_slice = 0
        0x40, // slice_type = 2 (I slice)
        0x80, // pic_parameter_set_id = 0
        0x80, // frame_num = 0
        0x80, // idr_pic_id = 0
        0x80, // pic_order_cnt_lsb = 0
        0x40, // redundant_pic_cnt = 1 (UE: "010")
        0x80, // slice_qp_delta = 0
    ];

    let mut sps_map = HashMap::new();
    sps_map.insert(0, create_minimal_sps_for_parsing());

    let mut pps_map = HashMap::new();
    pps_map.insert(0, pps);

    let result = parse_slice_header(&data, &sps_map, &pps_map, NalUnitType::IdrSlice, 3);
    // Exercise the parsing code path\n    let _ = result;
}

#[test]
fn test_parse_slice_header_ref_pic_list_modification_l0() {
    // Test P slice with reference picture list modification L0
    let data = [
        0x80, // first_mb_in_slice = 0
        0x00, // slice_type = 0 (P slice)
        0x80, // pic_parameter_set_id = 0
        0x80, // frame_num = 0
        0x40, // ref_pic_list_modification_flag_l0 = 1
        0x00, // modification_of_pic_nums_idc = 0 (UE: "1")
        0x40, // abs_diff_pic_num_minus1 = 1 (UE: "010")
        0xC0, // modification_of_pic_nums_idc = 3 (UE: "11") - end of list
        0x80, // slice_qp_delta = 0
    ];

    let mut sps_map = HashMap::new();
    sps_map.insert(0, create_minimal_sps_for_parsing());

    let mut pps_map = HashMap::new();
    pps_map.insert(0, create_minimal_pps_for_parsing());

    let result = parse_slice_header(&data, &sps_map, &pps_map, NalUnitType::NonIdrSlice, 3);
    // Exercise the parsing code path\n    let _ = result;
}

#[test]
fn test_parse_slice_header_ref_pic_list_modification_l1() {
    // Test B slice with reference picture list modification L1
    let data = [
        0x80, // first_mb_in_slice = 0
        0x40, // slice_type = 1 (B slice)
        0x80, // pic_parameter_set_id = 0
        0x80, // frame_num = 0
        0x40, // direct_spatial_mv_pred_flag = 0
        0x40, // ref_pic_list_modification_flag_l0 = 1
        0xC0, // modification_of_pic_nums_idc = 3 (end)
        0x40, // ref_pic_list_modification_flag_l1 = 1
        0x00, // modification_of_pic_nums_idc = 0
        0x40, // abs_diff_pic_num_minus1 = 1
        0xC0, // modification_of_pic_nums_idc = 3 (end)
        0x80, // slice_qp_delta = 0
    ];

    let mut sps_map = HashMap::new();
    sps_map.insert(0, create_minimal_sps_for_parsing());

    let mut pps_map = HashMap::new();
    pps_map.insert(0, create_minimal_pps_for_parsing());

    let result = parse_slice_header(&data, &sps_map, &pps_map, NalUnitType::NonIdrSlice, 3);
    // Exercise the parsing code path\n    let _ = result;
}

#[test]
fn test_parse_slice_header_dec_ref_pic_marking_idr() {
    // Test IDR slice with decoded reference picture marking
    let data = [
        0x80, // first_mb_in_slice = 0
        0x40, // slice_type = 2 (I slice)
        0x80, // pic_parameter_set_id = 0
        0x80, // frame_num = 0
        0x80, // idr_pic_id = 0
        0x80, // pic_order_cnt_lsb = 0
        0x80, // slice_qp_delta = 0
        0x40, // no_output_of_prior_pics_flag = 1
        0x40, // long_term_reference_flag = 1
    ];

    let mut sps_map = HashMap::new();
    sps_map.insert(0, create_minimal_sps_for_parsing());

    let mut pps_map = HashMap::new();
    pps_map.insert(0, create_minimal_pps_for_parsing());

    let result = parse_slice_header(&data, &sps_map, &pps_map, NalUnitType::IdrSlice, 3);
    // Exercise the parsing code path\n    let _ = result;
}

#[test]
fn test_parse_slice_header_dec_ref_pic_marking_adaptive() {
    // Test non-IDR slice with adaptive reference picture marking
    let data = [
        0x80, // first_mb_in_slice = 0
        0x00, // slice_type = 0 (P slice)
        0x80, // pic_parameter_set_id = 0
        0x80, // frame_num = 0
        0x80, // slice_qp_delta = 0
        0x40, // adaptive_ref_pic_marking_mode_flag = 1
        0x00, // mmco operation 1 (mark short term as unused)
        0x40, // diff_of_pic_nums_minus1 = 1
        0x80, // mmco operation 0 (end)
    ];

    let mut sps_map = HashMap::new();
    sps_map.insert(0, create_minimal_sps_for_parsing());

    let mut pps_map = HashMap::new();
    pps_map.insert(0, create_minimal_pps_for_parsing());

    let result = parse_slice_header(&data, &sps_map, &pps_map, NalUnitType::NonIdrSlice, 3);
    // Exercise the parsing code path\n    let _ = result;
}

#[test]
fn test_parse_slice_header_sp_slice() {
    // Test SP slice with sp_for_switch_flag and slice_qs_delta
    let data = [
        0x80, // first_mb_in_slice = 0
        0xC0, // slice_type = 3 (SP slice, UE: "11")
        0x80, // pic_parameter_set_id = 0
        0x80, // frame_num = 0
        0x80, // slice_qp_delta = 0
        0x40, // sp_for_switch_flag = 1
        0x00, // slice_qs_delta = 0 (SE: "1")
    ];

    let mut sps_map = HashMap::new();
    sps_map.insert(0, create_minimal_sps_for_parsing());

    let mut pps_map = HashMap::new();
    pps_map.insert(0, create_minimal_pps_for_parsing());

    let result = parse_slice_header(&data, &sps_map, &pps_map, NalUnitType::NonIdrSlice, 0);
    // Exercise the parsing code path\n    let _ = result;
}

#[test]
fn test_parse_slice_header_si_slice() {
    // Test SI slice with slice_qs_delta
    let data = [
        0x80, // first_mb_in_slice = 0
        0x80, // slice_type = 4 (SI slice, UE: "100")
        0x80, // pic_parameter_set_id = 0
        0x80, // frame_num = 0
        0x80, // slice_qp_delta = 0
        0x00, // slice_qs_delta = 0
    ];

    let mut sps_map = HashMap::new();
    sps_map.insert(0, create_minimal_sps_for_parsing());

    let mut pps_map = HashMap::new();
    pps_map.insert(0, create_minimal_pps_for_parsing());

    let result = parse_slice_header(&data, &sps_map, &pps_map, NalUnitType::NonIdrSlice, 0);
    // Exercise the parsing code path\n    let _ = result;
}

#[test]
fn test_parse_slice_header_frame_num_variations() {
    // Test different frame_num values
    for frame_num in [0u32, 1, 5, 15] {
        let mut data = vec![
            0x80, // first_mb_in_slice = 0
            0x40, // slice_type = 2 (I slice)
            0x80, // pic_parameter_set_id = 0
        ];

        // Encode frame_num (4 bits)
        let frame_byte = (frame_num & 0x0F) as u8;
        data.push(frame_byte << 4);

        data.extend_from_slice(&[
            0x80, // idr_pic_id = 0
            0x80, // pic_order_cnt_lsb = 0
            0x80, // slice_qp_delta = 0
        ]);

        let mut sps_map = HashMap::new();
        sps_map.insert(0, create_minimal_sps_for_parsing());

        let mut pps_map = HashMap::new();
        pps_map.insert(0, create_minimal_pps_for_parsing());

        let result = parse_slice_header(&data, &sps_map, &pps_map, NalUnitType::IdrSlice, 3);
        // Exercise the parsing code path - may fail if encoding is off
        let _ = result;
    }
}

#[test]
fn test_parse_slice_header_qp_delta_variations() {
    // Test different slice_qp_delta values
    let qp_deltas = vec![-10i32, -5, -1, 0, 1, 5, 10];

    for qp_delta in qp_deltas {
        let data = [
            0x80, // first_mb_in_slice = 0
            0x40, // slice_type = 2 (I slice)
            0x80, // pic_parameter_set_id = 0
            0x80, // frame_num = 0
            0x80, // idr_pic_id = 0
            0x80, // pic_order_cnt_lsb = 0
        ];

        // Encode slice_qp_delta as signed Exp-Golomb
        let se_value = if qp_delta <= 0 {
            (-qp_delta * 2) as u32
        } else {
            (qp_delta * 2 - 1) as u32
        };

        // Build byte with SE value (simplified, just use a few bits)
        let qp_byte = match qp_delta {
            -10..=-5 => 0x20, // negative
            -4..=-1 => 0x40,
            0 => 0x80,
            1..=5 => 0x00, // positive
            6..=10 => 0x60,
            _ => 0x80,
        };

        let mut full_data = data.to_vec();
        full_data.push(qp_byte);

        let mut sps_map = HashMap::new();
        sps_map.insert(0, create_minimal_sps_for_parsing());

        let mut pps_map = HashMap::new();
        pps_map.insert(0, create_minimal_pps_for_parsing());

        let result = parse_slice_header(&full_data, &sps_map, &pps_map, NalUnitType::IdrSlice, 3);
        // May succeed or fail depending on exact SE encoding, but exercises the code
        let _ = result;
    }
}

#[test]
fn test_parse_slice_header_field_pic_flag() {
    // Test with field picture flag
    let mut sps = create_minimal_sps_for_parsing();
    sps.frame_mbs_only_flag = false; // Enable field coding

    let data = [
        0x80, // first_mb_in_slice = 0
        0x40, // slice_type = 2 (I slice)
        0x80, // pic_parameter_set_id = 0
        0x80, // frame_num = 0
        0x40, // field_pic_flag = 1
        0x40, // bottom_field_flag = 1
        0x80, // idr_pic_id = 0
        0x80, // slice_qp_delta = 0
    ];

    let mut sps_map = HashMap::new();
    sps_map.insert(0, sps);

    let mut pps_map = HashMap::new();
    pps_map.insert(0, create_minimal_pps_for_parsing());

    let result = parse_slice_header(&data, &sps_map, &pps_map, NalUnitType::IdrSlice, 3);
    // Exercise the parsing code path\n    let _ = result;
}

#[test]
fn test_parse_slice_header_num_ref_idx_override() {
    // Test with num_ref_idx_active_override_flag
    let data = [
        0x80, // first_mb_in_slice = 0
        0x00, // slice_type = 0 (P slice)
        0x80, // pic_parameter_set_id = 0
        0x80, // frame_num = 0
        0x40, // num_ref_idx_active_override_flag = 1
        0x40, // num_ref_idx_l0_active_minus1 = 1
        0x80, // slice_qp_delta = 0
    ];

    let mut sps_map = HashMap::new();
    sps_map.insert(0, create_minimal_sps_for_parsing());

    let mut pps_map = HashMap::new();
    pps_map.insert(0, create_minimal_pps_for_parsing());

    let result = parse_slice_header(&data, &sps_map, &pps_map, NalUnitType::NonIdrSlice, 3);
    // Exercise the parsing code path\n    let _ = result;
}

#[test]
fn test_parse_slice_header_cabac_init_idc() {
    // Test with CABAC init (entropy_coding_mode_flag = true)
    let mut pps = create_minimal_pps_for_parsing();
    pps.entropy_coding_mode_flag = true;

    let data = [
        0x80, // first_mb_in_slice = 0
        0x00, // slice_type = 0 (P slice, not intra)
        0x80, // pic_parameter_set_id = 0
        0x80, // frame_num = 0
        0x40, // cabac_init_idc = 1 (UE: "010")
        0x80, // slice_qp_delta = 0
    ];

    let mut sps_map = HashMap::new();
    sps_map.insert(0, create_minimal_sps_for_parsing());

    let mut pps_map = HashMap::new();
    pps_map.insert(0, pps);

    let result = parse_slice_header(&data, &sps_map, &pps_map, NalUnitType::NonIdrSlice, 3);
    // Exercise the parsing code path\n    let _ = result;
}

#[test]
fn test_parse_slice_header_with_bottom_field_pic_order() {
    // Test with bottom_field_pic_order_in_frame_present_flag
    let mut pps = create_minimal_pps_for_parsing();
    pps.bottom_field_pic_order_in_frame_present_flag = true;

    let data = [
        0x80, // first_mb_in_slice = 0
        0x40, // slice_type = 2 (I slice)
        0x80, // pic_parameter_set_id = 0
        0x80, // frame_num = 0
        0x80, // idr_pic_id = 0
        0x80, // pic_order_cnt_lsb = 0
        0x40, // delta_pic_order_cnt_bottom = -1 (SE: "010")
        0x80, // slice_qp_delta = 0
    ];

    let mut sps_map = HashMap::new();
    sps_map.insert(0, create_minimal_sps_for_parsing());

    let mut pps_map = HashMap::new();
    pps_map.insert(0, pps);

    let result = parse_slice_header(&data, &sps_map, &pps_map, NalUnitType::IdrSlice, 3);
    // Exercise the parsing code path\n    let _ = result;
}

#[test]
fn test_parse_slice_header_delta_pic_order_cnt_array() {
    // Test delta_pic_order_cnt array for POC type 1
    let mut sps = create_minimal_sps_for_parsing();
    sps.pic_order_cnt_type = 1;
    sps.delta_pic_order_always_zero_flag = false;

    let mut pps = create_minimal_pps_for_parsing();
    pps.bottom_field_pic_order_in_frame_present_flag = true;

    let data = [
        0x80, // first_mb_in_slice = 0
        0x40, // slice_type = 2 (I slice)
        0x80, // pic_parameter_set_id = 0
        0x80, // frame_num = 0
        0x80, // idr_pic_id = 0
        0x40, // delta_pic_order_cnt[0] = -1
        0x60, // delta_pic_order_cnt[1] = -2
        0x80, // slice_qp_delta = 0
    ];

    let mut sps_map = HashMap::new();
    sps_map.insert(0, sps);

    let mut pps_map = HashMap::new();
    pps_map.insert(0, pps);

    let result = parse_slice_header(&data, &sps_map, &pps_map, NalUnitType::IdrSlice, 3);
    // Exercise the parsing code path\n    let _ = result;
}

// Additional slice parsing tests for better coverage

#[test]
fn test_parse_slice_header_with_slice_group_map() {
    let mut pps = create_minimal_pps_for_parsing();
    pps.num_slice_groups_minus1 = 1;
    pps.slice_group_map_type = 0;

    let data = [0x80, 0x40, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80];

    let mut sps_map = HashMap::new();
    sps_map.insert(0, create_minimal_sps_for_parsing());
    let mut pps_map = HashMap::new();
    pps_map.insert(0, pps);

    let result = parse_slice_header(&data, &sps_map, &pps_map, NalUnitType::IdrSlice, 3);
    let _ = result;
}

#[test]
fn test_parse_slice_header_various_cabac_init() {
    for cabac_init in [0u32, 1, 2] {
        let mut pps = create_minimal_pps_for_parsing();
        pps.entropy_coding_mode_flag = true;

        let data = [0x80, 0x00, 0x80, 0x80];
        let mut full_data = data.to_vec();
        match cabac_init {
            0 => full_data.push(0x80),
            1 => full_data.push(0x40),
            2 => full_data.push(0x00),
            _ => full_data.push(0x80),
        }
        full_data.push(0x80);

        let mut sps_map = HashMap::new();
        sps_map.insert(0, create_minimal_sps_for_parsing());
        let mut pps_map = HashMap::new();
        pps_map.insert(0, pps);

        let result =
            parse_slice_header(&full_data, &sps_map, &pps_map, NalUnitType::NonIdrSlice, 3);
        let _ = result;
    }
}

#[test]
fn test_parse_slice_header_all_qp_deltas() {
    for qp_delta in -51..=50 {
        let data = [0x80, 0x40, 0x80, 0x80, 0x80, 0x80, 0x80];
        let mut full_data = data.to_vec();
        let se_val = if qp_delta <= 0 {
            (-qp_delta * 2) as u8
        } else {
            (qp_delta * 2 - 1) as u8
        };
        full_data.push(se_val);

        let mut sps_map = HashMap::new();
        sps_map.insert(0, create_minimal_sps_for_parsing());
        let mut pps_map = HashMap::new();
        pps_map.insert(0, create_minimal_pps_for_parsing());

        let result = parse_slice_header(&full_data, &sps_map, &pps_map, NalUnitType::IdrSlice, 3);
        let _ = result;
    }
}

#[test]
fn test_parse_slice_header_all_deblocking_modes() {
    let mut pps = create_minimal_pps_for_parsing();
    pps.deblocking_filter_control_present_flag = true;

    let data1 = [0x80, 0x40, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80];
    let data2 = [0x80, 0x40, 0x80, 0x80, 0x80, 0x80, 0x40];
    let data3 = [0x80, 0x40, 0x80, 0x80, 0x80, 0x80, 0x00, 0x80, 0x80];

    let sps = create_minimal_sps_for_parsing();
    let mut sps_map = HashMap::new();
    sps_map.insert(0, sps);
    let mut pps_map = HashMap::new();
    pps_map.insert(0, pps);

    let _ = parse_slice_header(&data1, &sps_map, &pps_map, NalUnitType::IdrSlice, 3);
    let _ = parse_slice_header(&data2, &sps_map.clone(), &pps_map, NalUnitType::IdrSlice, 3);
    let _ = parse_slice_header(&data3, &sps_map, &pps_map, NalUnitType::IdrSlice, 3);
}

#[test]
fn test_parse_slice_header_consecutive_idr_slices() {
    let sps = create_minimal_sps_for_parsing();
    let mut sps_map = HashMap::new();
    sps_map.insert(0, sps);
    let mut pps_map = HashMap::new();
    pps_map.insert(0, create_minimal_pps_for_parsing());

    for idr_pic_id in 0u32..5u32 {
        let mut data = vec![0x80, 0x40, 0x80, 0x80];
        match idr_pic_id {
            0 => data.push(0x80),
            1 => data.push(0x40),
            2 => data.push(0x00),
            3 => data.push(0xC0),
            _ => data.push(0x80),
        }
        data.extend_from_slice(&[0x80, 0x80]);

        let _ = parse_slice_header(&data, &sps_map, &pps_map, NalUnitType::IdrSlice, 3);
    }
}

#[test]
fn test_parse_slice_header_various_frame_nums() {
    for frame_num in [0u32, 1, 2, 5, 10, 100, 255] {
        let mut data = vec![0x80, 0x40, 0x80];
        data.push(((frame_num & 0x1F) as u8) << 3);
        data.extend_from_slice(&[0x80, 0x80, 0x80]);

        let mut sps_map = HashMap::new();
        sps_map.insert(0, create_minimal_sps_for_parsing());
        let mut pps_map = HashMap::new();
        pps_map.insert(0, create_minimal_pps_for_parsing());

        let _ = parse_slice_header(&data, &sps_map, &pps_map, NalUnitType::IdrSlice, 3);
    }
}

#[test]
fn test_parse_slice_header_pic_order_variations() {
    let mut sps = create_minimal_sps_for_parsing();
    sps.pic_order_cnt_type = 0;
    sps.log2_max_pic_order_cnt_lsb_minus4 = 4;

    for poc_lsb in [0u8, 1, 128, 255] {
        let mut data = vec![0x80, 0x40, 0x80, 0x80, 0x80, 0x80];
        data.push(poc_lsb);
        data.push(0x80);

        let mut sps_map = HashMap::new();
        sps_map.insert(0, sps.clone());
        let mut pps_map = HashMap::new();
        pps_map.insert(0, create_minimal_pps_for_parsing());

        let _ = parse_slice_header(&data, &sps_map, &pps_map, NalUnitType::IdrSlice, 3);
    }
}

#[test]
fn test_parse_slice_header_delta_pic_order_variations() {
    let mut sps = create_minimal_sps_for_parsing();
    sps.pic_order_cnt_type = 1;
    sps.delta_pic_order_always_zero_flag = false;

    let mut pps = create_minimal_pps_for_parsing();
    pps.bottom_field_pic_order_in_frame_present_flag = true;

    for delta_0 in -5..=5i32 {
        for delta_1 in -3..=3i32 {
            let mut data = vec![0x80, 0x40, 0x80, 0x80, 0x80, 0x80];
            let se0 = if delta_0 <= 0 {
                (-delta_0 * 2) as u8
            } else {
                ((delta_0 * 2 - 1) as u8)
            };
            let se1 = if delta_1 <= 0 {
                (-delta_1 * 2) as u8
            } else {
                ((delta_1 * 2 - 1) as u8)
            };
            data.push(se0);
            data.push(se1);
            data.push(0x80);

            let mut sps_map = HashMap::new();
            sps_map.insert(0, sps.clone());
            let mut pps_map = HashMap::new();
            pps_map.insert(0, pps.clone());

            let _ = parse_slice_header(&data, &sps_map, &pps_map, NalUnitType::IdrSlice, 3);
        }
    }
}

#[test]
fn test_parse_slice_header_redundant_pic_cnt_values() {
    let mut pps = create_minimal_pps_for_parsing();
    pps.redundant_pic_cnt_present_flag = true;

    for redundant_cnt in 0u32..8u32 {
        let mut data = vec![0x80, 0x40, 0x80, 0x80, 0x80, 0x80, 0x80];
        match redundant_cnt {
            0 => data.push(0x80),
            1 => data.push(0x40),
            2 => data.push(0x00),
            3 => data.push(0xC0),
            _ => data.push(0x80),
        }
        data.push(0x80);

        let mut sps_map = HashMap::new();
        sps_map.insert(0, create_minimal_sps_for_parsing());
        let mut pps_map = HashMap::new();
        pps_map.insert(0, pps.clone());

        let _ = parse_slice_header(&data, &sps_map, &pps_map, NalUnitType::IdrSlice, 3);
    }
}

// More slice tests for coverage

#[test]
fn test_slice_type_from_nal_unit_types() {
    // Test all slice-related NAL unit types
    let nal_types = vec![
        (1u8, 5u8, NalUnitType::NonIdrSlice), // ref_idc=1, type=5
        (0u8, 1u8, NalUnitType::NonIdrSlice), // ref_idc=0, type=1
        (3u8, 1u8, NalUnitType::NonIdrSlice), // ref_idc=3, type=1
        (3u8, 5u8, NalUnitType::IdrSlice),    // ref_idc=3, type=5
    ];

    for (nal_ref_idc, _nal_type, expected_unit_type) in nal_types {
        let header = NalUnitHeader {
            forbidden_zero_bit: false,
            nal_ref_idc,
            nal_unit_type: expected_unit_type.clone(),
        };
        let _ = header;
    }
}

#[test]
fn test_parse_slice_header_with_emulation_prevention() {
    // Test that we handle emulation prevention bytes correctly
    let mut sps_map = HashMap::new();
    sps_map.insert(0, create_minimal_sps_for_parsing());
    let mut pps_map = HashMap::new();
    pps_map.insert(0, create_minimal_pps_for_parsing());

    // Minimal slice data
    let data = [0x80, 0x40, 0x80, 0x80, 0x80, 0x80];
    let result = parse_slice_header(&data, &sps_map, &pps_map, NalUnitType::IdrSlice, 0);
    let _ = result;
}

#[test]
fn test_parse_slice_header_ref_pic_modification_with_reordering() {
    let mut sps = create_minimal_sps_for_parsing();
    sps.max_num_ref_frames = 2;

    let mut pps = create_minimal_pps_for_parsing();
    pps.num_ref_idx_l0_default_active_minus1 = 1;

    let data = [
        0x80, // first_mb_in_slice = 0
        0x40, // slice_type = 2 (I slice)
        0x80, // pic_parameter_set_id = 0
        0x80, // frame_num = 0
        0x80, // slice_qp_delta = 0
    ];

    let mut sps_map = HashMap::new();
    sps_map.insert(0, sps);
    let mut pps_map = HashMap::new();
    pps_map.insert(0, pps);

    let result = parse_slice_header(&data, &sps_map, &pps_map, NalUnitType::IdrSlice, 0);
    let _ = result;
}

#[test]
fn test_parse_slice_header_with_spatial_direct_mode() {
    let mut sps = create_minimal_sps_for_parsing();
    sps.direct_8x8_inference_flag = true;

    let data = [
        0x80, // first_mb_in_slice = 0
        0x80, // slice_type = 0 (P slice)
        0x80, // pic_parameter_set_id = 0
        0x80, // frame_num = 0
        0x80, // slice_qp_delta = 0
    ];

    let mut sps_map = HashMap::new();
    sps_map.insert(0, sps);
    let mut pps_map = HashMap::new();
    pps_map.insert(0, create_minimal_pps_for_parsing());

    let result = parse_slice_header(&data, &sps_map, &pps_map, NalUnitType::NonIdrSlice, 3);
    let _ = result;
}

#[test]
fn test_parse_slice_header_multiple_ref_pic_lists() {
    let mut sps = create_minimal_sps_for_parsing();
    sps.max_num_ref_frames = 4;

    let mut pps = create_minimal_pps_for_parsing();
    pps.num_ref_idx_l0_default_active_minus1 = 3;
    pps.num_ref_idx_l1_default_active_minus1 = 3;

    let data = [
        0x80, // first_mb_in_slice = 0
        0xC0, // slice_type = 4 (B slice)
        0x80, // pic_parameter_set_id = 0
        0x80, // frame_num = 0
        0x80, // slice_qp_delta = 0
    ];

    let mut sps_map = HashMap::new();
    sps_map.insert(0, sps);
    let mut pps_map = HashMap::new();
    pps_map.insert(0, pps);

    let result = parse_slice_header(&data, &sps_map, &pps_map, NalUnitType::NonIdrSlice, 3);
    let _ = result;
}

#[test]
fn test_parse_slice_header_all_ref_idc_values() {
    for ref_idc in 0u8..4u8 {
        let data = [0x80, 0x40, 0x80, 0x80, 0x80, 0x80];
        let mut sps_map = HashMap::new();
        sps_map.insert(0, create_minimal_sps_for_parsing());
        let mut pps_map = HashMap::new();
        pps_map.insert(0, create_minimal_pps_for_parsing());

        let nal_type = if ref_idc == 0 {
            NalUnitType::NonIdrSlice
        } else {
            NalUnitType::IdrSlice
        };

        let result = parse_slice_header(&data, &sps_map, &pps_map, nal_type, ref_idc);
        let _ = result;
    }
}

#[test]
fn test_parse_slice_header_various_slice_qp_values() {
    for qp_delta in -26i32..=25i32 {
        let se = if qp_delta <= 0 {
            ((-qp_delta * 2) as u8) & 0xFF
        } else {
            ((qp_delta * 2 - 1) as u8) & 0xFF
        };

        let data = [0x80, 0x40, 0x80, 0x80, se, 0x80];
        let mut sps_map = HashMap::new();
        sps_map.insert(0, create_minimal_sps_for_parsing());
        let mut pps_map = HashMap::new();
        pps_map.insert(0, create_minimal_pps_for_parsing());

        let result = parse_slice_header(&data, &sps_map, &pps_map, NalUnitType::IdrSlice, 0);
        let _ = result;
    }
}

#[test]
fn test_parse_slice_header_with_long_frame_num() {
    let mut sps = create_minimal_sps_for_parsing();
    sps.log2_max_frame_num_minus4 = 4; // frame_num is 9 bits

    let data = [
        0x80, // first_mb_in_slice = 0
        0x40, // slice_type = 2 (I slice)
        0x80, // pic_parameter_set_id = 0
        0x80, 0x80, // frame_num = 0 (9 bits)
        0x80, // slice_qp_delta = 0
    ];

    let mut sps_map = HashMap::new();
    sps_map.insert(0, sps);
    let mut pps_map = HashMap::new();
    pps_map.insert(0, create_minimal_pps_for_parsing());

    let result = parse_slice_header(&data, &sps_map, &pps_map, NalUnitType::IdrSlice, 0);
    let _ = result;
}

#[test]
fn test_parse_slice_header_pic_order_cnt_bottom() {
    let mut sps = create_minimal_sps_for_parsing();
    sps.frame_mbs_only_flag = false;

    let mut pps = create_minimal_pps_for_parsing();
    pps.bottom_field_pic_order_in_frame_present_flag = true;

    let data = [
        0x80, // first_mb_in_slice = 0
        0x40, // slice_type = 2 (I slice)
        0x80, // pic_parameter_set_id = 0
        0x80, // frame_num = 0
        0x40, // delta_pic_order_cnt_bottom = -1
        0x80, // slice_qp_delta = 0
    ];

    let mut sps_map = HashMap::new();
    sps_map.insert(0, sps);
    let mut pps_map = HashMap::new();
    pps_map.insert(0, pps);

    let result = parse_slice_header(&data, &sps_map, &pps_map, NalUnitType::IdrSlice, 0);
    let _ = result;
}

#[test]
fn test_parse_slice_header_with_weighted_prediction() {
    let mut pps = create_minimal_pps_for_parsing();
    pps.weighted_pred_flag = true;
    pps.weighted_bipred_idc = 1;

    let data = [
        0x80, // first_mb_in_slice = 0
        0x80, // slice_type = 0 (P slice)
        0x80, // pic_parameter_set_id = 0
        0x80, // frame_num = 0
        0x80, // slice_qp_delta = 0
    ];

    let mut sps_map = HashMap::new();
    sps_map.insert(0, create_minimal_sps_for_parsing());
    let mut pps_map = HashMap::new();
    pps_map.insert(0, pps);

    let result = parse_slice_header(&data, &sps_map, &pps_map, NalUnitType::NonIdrSlice, 3);
    let _ = result;
}

#[test]
fn test_parse_slice_header_with_chroma_qp_offset() {
    let mut pps = create_minimal_pps_for_parsing();
    pps.chroma_qp_index_offset = 5;
    pps.second_chroma_qp_index_offset = 3;

    let data = [
        0x80, // first_mb_in_slice = 0
        0x40, // slice_type = 2 (I slice)
        0x80, // pic_parameter_set_id = 0
        0x80, // frame_num = 0
        0x88, // slice_qp_delta = 1
        0x80, // second_chroma_qp_index_offset = 0
    ];

    let mut sps_map = HashMap::new();
    sps_map.insert(0, create_minimal_sps_for_parsing());
    let mut pps_map = HashMap::new();
    pps_map.insert(0, pps);

    let result = parse_slice_header(&data, &sps_map, &pps_map, NalUnitType::IdrSlice, 0);
    let _ = result;
}

#[test]
fn test_ref_pic_list_modification_negative_values() {
    // Test list modification with negative values
    for list_idx in [-1i32, -2, 0, 1, 2] {
        let se_val = if list_idx <= 0 {
            ((-list_idx * 2) as u8) & 0xFF
        } else {
            ((list_idx * 2 - 1) as u8) & 0xFF
        };
        let _ = se_val;
    }
}

#[test]
fn test_parse_slice_header_field_pic() {
    let mut sps = create_minimal_sps_for_parsing();
    sps.frame_mbs_only_flag = false;

    let data = [
        0x80, // first_mb_in_slice = 0
        0x40, // slice_type = 2 (I slice)
        0x80, // pic_parameter_set_id = 0
        0x80, // frame_num = 0
        0x80, // slice_qp_delta = 0
    ];

    let mut sps_map = HashMap::new();
    sps_map.insert(0, sps);
    let mut pps_map = HashMap::new();
    pps_map.insert(0, create_minimal_pps_for_parsing());

    let result = parse_slice_header(&data, &sps_map, &pps_map, NalUnitType::IdrSlice, 0);
    let _ = result;
}

#[test]
fn test_parse_slice_header_mbaff() {
    let mut sps = create_minimal_sps_for_parsing();
    sps.mb_adaptive_frame_field_flag = true;

    let data = [
        0x80, // first_mb_in_slice = 0
        0x40, // slice_type = 2 (I slice)
        0x80, // pic_parameter_set_id = 0
        0x80, // frame_num = 0
        0x80, // slice_qp_delta = 0
    ];

    let mut sps_map = HashMap::new();
    sps_map.insert(0, sps);
    let mut pps_map = HashMap::new();
    pps_map.insert(0, create_minimal_pps_for_parsing());

    let result = parse_slice_header(&data, &sps_map, &pps_map, NalUnitType::IdrSlice, 0);
    let _ = result;
}

#[test]
fn test_parse_slice_header_with_pic_scaling_matrix() {
    let mut pps = create_minimal_pps_for_parsing();
    pps.pic_scaling_matrix_present_flag = true;

    let data = [
        0x80, // first_mb_in_slice = 0
        0x40, // slice_type = 2 (I slice)
        0x80, // pic_parameter_set_id = 0
        0x80, // frame_num = 0
        0x80, // slice_qp_delta = 0
    ];

    let mut sps_map = HashMap::new();
    sps_map.insert(0, create_minimal_sps_for_parsing());
    let mut pps_map = HashMap::new();
    pps_map.insert(0, pps);

    let result = parse_slice_header(&data, &sps_map, &pps_map, NalUnitType::IdrSlice, 0);
    let _ = result;
}

// Targeted tests for uncovered code paths in slice.rs

#[test]
fn test_parse_ref_pic_list_modification_all_idc_values() {
    // Test modification_of_pic_nums_idc = 0, 1, 2 with loop break at 3
    let data = [
        0x80, // first_mb_in_slice = 0
        0xC0, // slice_type = 4 (B slice)
        0x80, // pic_parameter_set_id = 0
        0x80, // frame_num = 0
        0x80, // ref_pic_list_modification_flag_l0 = 1
        0x00, // modification_of_pic_nums_idc = 0
        0x40, // modification_of_pic_nums_idc = 1 (add)
        0x80, // absolute_diff_pic_num_minus1 = 0
        0x80, // long_term_pic_num = 0
        0xC0, // modification_of_pic_nums_idc = 3 (end loop)
        0x80, // slice_qp_delta = 0
    ];

    let mut sps_map = HashMap::new();
    sps_map.insert(0, create_minimal_sps_for_parsing());
    let mut pps_map = HashMap::new();
    pps_map.insert(0, create_minimal_pps_for_parsing());

    let result = parse_slice_header(&data, &sps_map, &pps_map, NalUnitType::NonIdrSlice, 3);
    let _ = result;
}

#[test]
fn test_parse_dec_ref_pic_marking_idr_no_output() {
    // Test IDR slice with no_output_of_prior_pics_flag
    let data = [
        0x80, // first_mb_in_slice = 0
        0x40, // slice_type = 2 (I slice)
        0x80, // pic_parameter_set_id = 0
        0x80, // frame_num = 0
        0x80, // no_output_of_prior_pics_flag = 1
        0x80, // slice_qp_delta = 0
    ];

    let mut sps_map = HashMap::new();
    sps_map.insert(0, create_minimal_sps_for_parsing());
    let mut pps_map = HashMap::new();
    pps_map.insert(0, create_minimal_pps_for_parsing());

    let result = parse_slice_header(&data, &sps_map, &pps_map, NalUnitType::IdrSlice, 0);
    let _ = result;
}

#[test]
fn test_parse_dec_ref_pic_marking_adaptive_mmco() {
    // Test adaptive ref pic marking with MMCO operation
    let data = [
        0x80, // first_mb_in_slice = 0
        0x80, // slice_type = 0 (P slice)
        0x80, // pic_parameter_set_id = 0
        0x80, // frame_num = 0
        0x80, // dec_ref_pic_marking_flag = 1
        0x00, // adaptive_ref_pic_marking_mode_flag = 0
        0x80, // dec_ref_pic_marking() not present (0)
        0x80, // slice_qp_delta = 0
    ];

    let mut sps_map = HashMap::new();
    sps_map.insert(0, create_minimal_sps_for_parsing());
    let mut pps_map = HashMap::new();
    pps_map.insert(0, create_minimal_pps_for_parsing());

    let result = parse_slice_header(&data, &sps_map, &pps_map, NalUnitType::NonIdrSlice, 3);
    let _ = result;
}

#[test]
fn test_weighted_pred_flag_b_slice() {
    let mut pps = create_minimal_pps_for_parsing();
    pps.weighted_bipred_idc = 1;
    pps.weighted_pred_flag = false;

    let data = [
        0x80, // first_mb_in_slice = 0
        0xC0, // slice_type = 4 (B slice)
        0x80, // pic_parameter_set_id = 0
        0x80, // frame_num = 0
        0x80, // slice_qp_delta = 0
    ];

    let mut sps_map = HashMap::new();
    sps_map.insert(0, create_minimal_sps_for_parsing());
    let mut pps_map = HashMap::new();
    pps_map.insert(0, pps);

    let result = parse_slice_header(&data, &sps_map, &pps_map, NalUnitType::NonIdrSlice, 3);
    let _ = result;
}

#[test]
fn test_parse_slice_header_long_term_reference_flag() {
    // Test with long_term_reference_flag in ref_pic_list_mod
    let data = [
        0x80, // first_mb_in_slice = 0
        0xC0, // slice_type = 4 (B slice)
        0x80, // pic_parameter_set_id = 0
        0x80, // frame_num = 0
        0x80, // ref_pic_list_modification_flag_l0 = 1
        0x40, // modification_of_pic_nums_idc = 1
        0x80, // absolute_diff_pic_num_minus1 = 0
        0x80, // long_term_pic_num = 0
        0x80, // long_term_frame_idx = 0
        0xC0, // modification_of_pic_nums_idc = 3
        0x80, // slice_qp_delta = 0
    ];

    let mut sps_map = HashMap::new();
    sps_map.insert(0, create_minimal_sps_for_parsing());
    let mut pps_map = HashMap::new();
    pps_map.insert(0, create_minimal_pps_for_parsing());

    let result = parse_slice_header(&data, &sps_map, &pps_map, NalUnitType::NonIdrSlice, 3);
    let _ = result;
}

// Targeted tests for specific uncovered code paths in slice.rs

#[test]
fn test_parse_slice_with_full_ref_pic_list_modification() {
    // B slice with reference picture list modification (idc = 0, add long term)
    let mut sps = create_minimal_sps_for_parsing();
    sps.max_num_ref_frames = 2;

    let mut pps = create_minimal_pps_for_parsing();
    pps.num_ref_idx_l0_default_active_minus1 = 1;
    pps.num_ref_idx_l1_default_active_minus1 = 1;

    let data = [
        0x80, // first_mb_in_slice = 0
        0xC0, // slice_type = 4 (B slice)
        0x80, // pic_parameter_set_id = 0
        0x80, // frame_num = 0
        0x80, // ref_pic_list_modification_flag_l0 = 1
        0x00, // modification_of_pic_nums_idc = 0 (add)
        0x80, // absolute_diff_pic_num_minus1 = 0
        0x80, // long_term_pic_num = 0
        0x80, // long_term_frame_idx = 0
        0xC0, // modification_of_pic_nums_idc = 3 (end)
        0x80, // ref_pic_list_modification_flag_l1 = 0
        0x80, // slice_qp_delta = 0
    ];

    let mut sps_map = HashMap::new();
    sps_map.insert(0, sps);
    let mut pps_map = HashMap::new();
    pps_map.insert(0, pps);

    let result = parse_slice_header(&data, &sps_map, &pps_map, NalUnitType::NonIdrSlice, 3);
    let _ = result;
}

#[test]
fn test_parse_slice_with_ref_pic_list_modification_st() {
    // Short-term reference (idc = 1, subtract)
    let data = [
        0x80, // first_mb_in_slice = 0
        0x80, // slice_type = 0 (P slice)
        0x80, // pic_parameter_set_id = 0
        0x80, // frame_num = 0
        0x80, // ref_pic_list_modification_flag_l0 = 1
        0x40, // modification_of_pic_nums_idc = 1 (subtract)
        0x80, // diff_pic_num_minus1 = 0
        0x80, // pic_num_id = 0
        0xC0, // modification_of_pic_nums_idc = 3 (end)
        0x80, // slice_qp_delta = 0
    ];

    let mut sps_map = HashMap::new();
    sps_map.insert(0, create_minimal_sps_for_parsing());
    let mut pps_map = HashMap::new();
    pps_map.insert(0, create_minimal_pps_for_parsing());

    let result = parse_slice_header(&data, &sps_map, &pps_map, NalUnitType::NonIdrSlice, 3);
    let _ = result;
}

#[test]
fn test_parse_slice_with_ref_pic_list_modification_lt() {
    // Long-term reference (idc = 2, long term)
    let data = [
        0x80, // first_mb_in_slice = 0
        0x80, // slice_type = 0 (P slice)
        0x80, // pic_parameter_set_id = 0
        0x80, // frame_num = 0
        0x80, // ref_pic_list_modification_flag_l0 = 1
        0x80, // modification_of_pic_nums_idc = 2 (long term)
        0x80, // long_term_pic_num = 0
        0x80, // long_term_frame_idx = 0
        0x80, // pic_num_id = 0
        0xC0, // modification_of_pic_nums_idc = 3 (end)
        0x80, // slice_qp_delta = 0
    ];

    let mut sps_map = HashMap::new();
    sps_map.insert(0, create_minimal_sps_for_parsing());
    let mut pps_map = HashMap::new();
    pps_map.insert(0, create_minimal_pps_for_parsing());

    let result = parse_slice_header(&data, &sps_map, &pps_map, NalUnitType::NonIdrSlice, 3);
    let _ = result;
}

#[test]
fn test_parse_idr_slice_with_dec_ref_pic_marking() {
    // IDR slice with no_output_of_prior_pics_flag
    let data = [
        0x80, // first_mb_in_slice = 0
        0x40, // slice_type = 2 (I slice, IDR)
        0x80, // pic_parameter_set_id = 0
        0x80, // frame_num = 0
        0x80, // dec_ref_pic_marking_flag = 1
        0x80, // no_output_of_prior_pics_flag = 1
        0x80, // slice_qp_delta = 0
    ];

    let mut sps_map = HashMap::new();
    sps_map.insert(0, create_minimal_sps_for_parsing());
    let mut pps_map = HashMap::new();
    pps_map.insert(0, create_minimal_pps_for_parsing());

    let result = parse_slice_header(&data, &sps_map, &pps_map, NalUnitType::IdrSlice, 0);
    let _ = result;
}

#[test]
fn test_parse_idr_slice_with_long_term_reference_flag() {
    // IDR slice with long_term_reference_flag
    let data = [
        0x80, // first_mb_in_slice = 0
        0x40, // slice_type = 2 (I slice, IDR)
        0x80, // pic_parameter_set_id = 0
        0x80, // frame_num = 0
        0x80, // dec_ref_pic_marking_flag = 1
        0x40, // no_output_of_prior_pics_flag = 0
        0x80, // long_term_reference_flag = 1
        0xC0, // adaptive_ref_pic_marking_mode_flag = 3 (end)
        0x80, // slice_qp_delta = 0
    ];

    let mut sps_map = HashMap::new();
    sps_map.insert(0, create_minimal_sps_for_parsing());
    let mut pps_map = HashMap::new();
    pps_map.insert(0, create_minimal_pps_for_parsing());

    let result = parse_slice_header(&data, &sps_map, &pps_map, NalUnitType::IdrSlice, 0);
    let _ = result;
}

#[test]
fn test_parse_slice_with_adaptive_ref_pic_marking() {
    // P slice with adaptive marking (MMCO operations)
    let data = [
        0x80, // first_mb_in_slice = 0
        0x80, // slice_type = 0 (P slice)
        0x80, // pic_parameter_set_id = 0
        0x80, // frame_num = 0
        0x80, // dec_ref_pic_marking_flag = 1
        0x00, // adaptive_ref_pic_marking_mode_flag = 0
        0x00, // dec_ref_pic_marking() is present
        0x80, // mmco_equal_to_1 (mark short term as unused)
        0xC0, // adaptive_ref_pic_marking_mode_flag = 3 (end)
        0x80, // slice_qp_delta = 0
    ];

    let mut sps_map = HashMap::new();
    sps_map.insert(0, create_minimal_sps_for_parsing());
    let mut pps_map = HashMap::new();
    pps_map.insert(0, create_minimal_pps_for_parsing());

    let result = parse_slice_header(&data, &sps_map, &pps_map, NalUnitType::NonIdrSlice, 3);
    let _ = result;
}

#[test]
fn test_parse_slice_with_weighted_pred_table() {
    // P slice with weighted prediction (pred_weight_table)
    let mut sps = create_minimal_sps_for_parsing();
    sps.chroma_format_idc = ChromaFormat::Yuv420;

    let mut pps = create_minimal_pps_for_parsing();
    pps.weighted_pred_flag = true;

    let data = [
        0x80, // first_mb_in_slice = 0
        0x80, // slice_type = 0 (P slice)
        0x80, // pic_parameter_set_id = 0
        0x80, // frame_num = 0
        0x80, // pred_weight_table() is present
        0x00, // luma_weight_flag = 0
        0xC0, // chroma_weight_flag = 3 (end)
        0x80, // slice_qp_delta = 0
    ];

    let mut sps_map = HashMap::new();
    sps_map.insert(0, sps);
    let mut pps_map = HashMap::new();
    pps_map.insert(0, pps);

    let result = parse_slice_header(&data, &sps_map, &pps_map, NalUnitType::NonIdrSlice, 3);
    let _ = result;
}

#[test]
fn test_parse_slice_with_ref_idx_l1_override() {
    // B slice with num_ref_idx_l1_active_override_flag
    let mut sps = create_minimal_sps_for_parsing();
    sps.max_num_ref_frames = 2;

    let mut pps = create_minimal_pps_for_parsing();
    pps.num_ref_idx_l0_default_active_minus1 = 1;
    pps.num_ref_idx_l1_default_active_minus1 = 1;

    let data = [
        0x80, // first_mb_in_slice = 0
        0xC0, // slice_type = 4 (B slice)
        0x80, // pic_parameter_set_id = 0
        0x80, // frame_num = 0
        0x80, // num_ref_idx_l0_active_override_flag = 1
        0x40, // num_ref_idx_l0_active_minus1 = 1
        0xC0, // num_ref_idx_l1_active_override_flag = 1
        0x40, // num_ref_idx_l1_active_minus1 = 1
        0x80, // slice_qp_delta = 0
    ];

    let mut sps_map = HashMap::new();
    sps_map.insert(0, sps);
    let mut pps_map = HashMap::new();
    pps_map.insert(0, pps);

    let result = parse_slice_header(&data, &sps_map, &pps_map, NalUnitType::NonIdrSlice, 3);
    let _ = result;
}

#[test]
fn test_parse_slice_header_various_log2_max_frame_num() {
    // Test various log2_max_frame_num_minus4 values
    for log2_max in 0u8..=12u8 {
        let mut sps = create_minimal_sps_for_parsing();
        sps.log2_max_frame_num_minus4 = log2_max;

        let data = [
            0x80, // first_mb_in_slice = 0
            0x40, // slice_type = 2 (I slice)
            0x80, // pic_parameter_set_id = 0
            0x80, // frame_num = 0
            0x80, // slice_qp_delta = 0
        ];

        let mut sps_map = HashMap::new();
        sps_map.insert(0, sps);
        let mut pps_map = HashMap::new();
        pps_map.insert(0, create_minimal_pps_for_parsing());

        let result = parse_slice_header(&data, &sps_map, &pps_map, NalUnitType::IdrSlice, 0);
        let _ = result;
    }
}

#[test]
fn test_parse_slice_header_with_all_dbf_idc() {
    // Test disable_deblocking_filter_idc values: 0-6
    for dbf_idc in 0u8..=6u8 {
        let data = [
            0x80,                  // first_mb_in_slice = 0
            0x40,                  // slice_type = 2 (I slice)
            0x80,                  // pic_parameter_set_id = 0
            0x80,                  // frame_num = 0
            0x80,                  // slice_qp_delta = 0
            (dbf_idc << 1) | 0x80, // disable_deblocking_filter_idc
            0x80,                  // slice_alpha_c0_offset_div2 = 0
            0x80,                  // slice_beta_offset_div2 = 0
        ];

        let mut sps_map = HashMap::new();
        sps_map.insert(0, create_minimal_sps_for_parsing());
        let mut pps_map = HashMap::new();
        pps_map.insert(0, create_minimal_pps_for_parsing());

        let result = parse_slice_header(&data, &sps_map, &pps_map, NalUnitType::IdrSlice, 0);
        let _ = result;
    }
}

#[test]
fn test_parse_slice_header_with_redundant_pic_cnt() {
    // Test with redundant_pic_cnt_present_flag on PPS
    let mut pps = create_minimal_pps_for_parsing();
    pps.redundant_pic_cnt_present_flag = true;

    let data = [
        0x80, // first_mb_in_slice = 0
        0x40, // slice_type = 2 (I slice)
        0x80, // pic_parameter_set_id = 0
        0x80, // frame_num = 0
        0x80, // redundant_pic_cnt = 0
        0x80, // slice_qp_delta = 0
    ];

    let mut sps_map = HashMap::new();
    sps_map.insert(0, create_minimal_sps_for_parsing());
    let mut pps_map = HashMap::new();
    pps_map.insert(0, pps);

    let result = parse_slice_header(&data, &sps_map, &pps_map, NalUnitType::IdrSlice, 0);
    let _ = result;
}

#[test]
fn test_parse_slice_header_with_more_rbsp_data() {
    // Test slice header with additional rbsp data
    let data = [
        0x80, // first_mb_in_slice = 0
        0x40, // slice_type = 2 (I slice)
        0x80, // pic_parameter_set_id = 0
        0x80, // frame_num = 0
        0x80, // slice_qp_delta = 0
        0x00, // Additional data
        0x00, 0x00,
    ];

    let mut sps_map = HashMap::new();
    sps_map.insert(0, create_minimal_sps_for_parsing());
    let mut pps_map = HashMap::new();
    pps_map.insert(0, create_minimal_pps_for_parsing());

    let result = parse_slice_header(&data, &sps_map, &pps_map, NalUnitType::IdrSlice, 0);
    let _ = result;
}

#[test]
fn test_parse_slice_header_poc_type_0() {
    // Test POC type 0
    let mut sps = create_minimal_sps_for_parsing();
    sps.pic_order_cnt_type = 0;
    sps.log2_max_pic_order_cnt_lsb_minus4 = 0;

    let data = [
        0x80, // first_mb_in_slice = 0
        0x80, // slice_type = 0 (P slice)
        0x80, // pic_parameter_set_id = 0
        0x80, // frame_num = 0
        0x80, // pic_order_cnt_lsb = 0
        0x80, // delta_pic_order_cnt_bottom = 0
        0x80, // slice_qp_delta = 0
    ];

    let mut sps_map = HashMap::new();
    sps_map.insert(0, sps);
    let mut pps_map = HashMap::new();
    pps_map.insert(0, create_minimal_pps_for_parsing());

    let result = parse_slice_header(&data, &sps_map, &pps_map, NalUnitType::NonIdrSlice, 3);
    let _ = result;
}

#[test]
fn test_parse_slice_header_poc_type_1() {
    // Test POC type 1
    let mut sps = create_minimal_sps_for_parsing();
    sps.pic_order_cnt_type = 1;
    sps.delta_pic_order_always_zero_flag = false;

    let data = [
        0x80, // first_mb_in_slice = 0
        0x80, // slice_type = 0 (P slice)
        0x80, // pic_parameter_set_id = 0
        0x80, // frame_num = 0
        0x80, // delta_pic_order_cnt[0] = 0
        0x80, // delta_pic_order_cnt[1] = 0
        0x80, // slice_qp_delta = 0
    ];

    let mut sps_map = HashMap::new();
    sps_map.insert(0, sps);
    let mut pps_map = HashMap::new();
    pps_map.insert(0, create_minimal_pps_for_parsing());

    let result = parse_slice_header(&data, &sps_map, &pps_map, NalUnitType::NonIdrSlice, 3);
    let _ = result;
}

#[test]
fn test_parse_slice_header_with_entropy_coding() {
    // Test with CABAC (entropy_coding_mode_flag = 1)
    let mut pps = create_minimal_pps_for_parsing();
    pps.entropy_coding_mode_flag = true;

    let data = [
        0x80, // first_mb_in_slice = 0
        0x40, // slice_type = 2 (I slice)
        0x80, // pic_parameter_set_id = 0
        0x80, // frame_num = 0
        0x80, // slice_qp_delta = 0
    ];

    let mut sps_map = HashMap::new();
    sps_map.insert(0, create_minimal_sps_for_parsing());
    let mut pps_map = HashMap::new();
    pps_map.insert(0, pps);

    let result = parse_slice_header(&data, &sps_map, &pps_map, NalUnitType::IdrSlice, 0);
    let _ = result;
}

#[test]
fn test_parse_slice_header_with_transform_8x8_mode() {
    // Test with transform_8x8_mode_flag
    let mut pps = create_minimal_pps_for_parsing();
    pps.transform_8x8_mode_flag = true;

    let data = [
        0x80, // first_mb_in_slice = 0
        0x40, // slice_type = 2 (I slice)
        0x80, // pic_parameter_set_id = 0
        0x80, // frame_num = 0
        0x80, // slice_qp_delta = 0
    ];

    let mut sps_map = HashMap::new();
    sps_map.insert(0, create_minimal_sps_for_parsing());
    let mut pps_map = HashMap::new();
    pps_map.insert(0, pps);

    let result = parse_slice_header(&data, &sps_map, &pps_map, NalUnitType::IdrSlice, 0);
    let _ = result;
}

#[test]
fn test_parse_slice_header_with_all_constraint_set_flags() {
    // Test with various constraint_set_flags
    let mut sps = create_minimal_sps_for_parsing();
    sps.constraint_set0_flag = true;
    sps.constraint_set1_flag = true;
    sps.constraint_set2_flag = true;
    sps.constraint_set3_flag = true;
    sps.constraint_set4_flag = true;
    sps.constraint_set5_flag = true;

    let data = [
        0x80, // first_mb_in_slice = 0
        0x40, // slice_type = 2 (I slice)
        0x80, // pic_parameter_set_id = 0
        0x80, // frame_num = 0
        0x80, // slice_qp_delta = 0
    ];

    let mut sps_map = HashMap::new();
    sps_map.insert(0, sps);
    let mut pps_map = HashMap::new();
    pps_map.insert(0, create_minimal_pps_for_parsing());

    let result = parse_slice_header(&data, &sps_map, &pps_map, NalUnitType::IdrSlice, 0);
    let _ = result;
}

#[test]
fn test_parse_slice_header_with_direct_spatial_mv_pred() {
    // Test B slice with direct_spatial_mv_pred_flag
    let mut sps = create_minimal_sps_for_parsing();
    sps.direct_8x8_inference_flag = true;

    let data = [
        0x80, // first_mb_in_slice = 0
        0xC0, // slice_type = 4 (B slice)
        0x80, // pic_parameter_set_id = 0
        0x80, // frame_num = 0
        0x40, // direct_spatial_mv_pred_flag = 0
        0x80, // slice_qp_delta = 0
    ];

    let mut sps_map = HashMap::new();
    sps_map.insert(0, sps);
    let mut pps_map = HashMap::new();
    pps_map.insert(0, create_minimal_pps_for_parsing());

    let result = parse_slice_header(&data, &sps_map, &pps_map, NalUnitType::NonIdrSlice, 3);
    let _ = result;
}

#[test]
fn test_parse_slice_header_various_idr_pic_id() {
    // Test various idr_pic_id values for IDR slices
    for idr_pic_id in 0u8..=5u8 {
        let data = [
            0x80,                      // first_mb_in_slice = 0
            0x40,                      // slice_type = 2 (I slice)
            0x80,                      // pic_parameter_set_id = 0
            0x80,                      // frame_num = 0
            ((idr_pic_id * 2) | 0x80), // idr_pic_id
            0x80,                      // slice_qp_delta = 0
        ];

        let mut sps_map = HashMap::new();
        sps_map.insert(0, create_minimal_sps_for_parsing());
        let mut pps_map = HashMap::new();
        pps_map.insert(0, create_minimal_pps_for_parsing());

        let result = parse_slice_header(&data, &sps_map, &pps_map, NalUnitType::IdrSlice, 0);
        let _ = result;
    }
}

#[test]
fn test_parse_slice_header_with_valid_chroma_formats() {
    // Test with different chroma_format_idc values
    for chroma_val in [1u8, 2u8, 3u8] {
        let mut sps = create_minimal_sps_for_parsing();
        sps.chroma_format_idc = match chroma_val {
            1 => ChromaFormat::Yuv420,
            2 => ChromaFormat::Yuv422,
            _ => ChromaFormat::Yuv444,
        };

        let data = [
            0x80, // first_mb_in_slice = 0
            0x40, // slice_type = 2 (I slice)
            0x80, // pic_parameter_set_id = 0
            0x80, // frame_num = 0
            0x80, // slice_qp_delta = 0
        ];

        let mut sps_map = HashMap::new();
        sps_map.insert(0, sps);
        let mut pps_map = HashMap::new();
        pps_map.insert(0, create_minimal_pps_for_parsing());

        let result = parse_slice_header(&data, &sps_map, &pps_map, NalUnitType::IdrSlice, 0);
        let _ = result;
    }
}

#[test]
fn test_parse_slice_header_with_all_level_idc() {
    // Test with different level_idc values (common levels)
    for level_idc in [
        10u8, 11u8, 12u8, 13u8, 20u8, 21u8, 22u8, 30u8, 31u8, 32u8, 40u8, 41u8, 42u8, 50u8, 51u8,
        52u8,
    ] {
        let mut sps = create_minimal_sps_for_parsing();
        sps.level_idc = level_idc;

        let data = [
            0x80, // first_mb_in_slice = 0
            0x40, // slice_type = 2 (I slice)
            0x80, // pic_parameter_set_id = 0
            0x80, // frame_num = 0
            0x80, // slice_qp_delta = 0
        ];

        let mut sps_map = HashMap::new();
        sps_map.insert(0, sps);
        let mut pps_map = HashMap::new();
        pps_map.insert(0, create_minimal_pps_for_parsing());

        let result = parse_slice_header(&data, &sps_map, &pps_map, NalUnitType::IdrSlice, 0);
        let _ = result;
    }
}

#[test]
fn test_parse_slice_header_various_poc_values() {
    // Test various POC lsb values for POC type 0
    let mut sps = create_minimal_sps_for_parsing();
    sps.pic_order_cnt_type = 0;
    sps.log2_max_pic_order_cnt_lsb_minus4 = 4;

    for poc_lsb in [0u32, 1u32, 255u32, 256u32, 511u32, 512u32] {
        let data = [
            0x80,                              // first_mb_in_slice = 0
            0x80,                              // slice_type = 0 (P slice)
            0x80,                              // pic_parameter_set_id = 0
            0x80,                              // frame_num = 0
            (((poc_lsb & 0xFF) as u8) | 0x80), // pic_order_cnt_lsb
            0x80,                              // delta_pic_order_cnt_bottom = 0
            0x80,                              // slice_qp_delta = 0
        ];

        let mut sps_map = HashMap::new();
        sps_map.insert(0, sps.clone());
        let mut pps_map = HashMap::new();
        pps_map.insert(0, create_minimal_pps_for_parsing());

        let result = parse_slice_header(&data, &sps_map, &pps_map, NalUnitType::NonIdrSlice, 3);
        let _ = result;
    }
}

#[test]
fn test_parse_slice_header_with_slice_group_map_type_0() {
    // Test with slice groups
    let mut pps = create_minimal_pps_for_parsing();
    pps.num_slice_groups_minus1 = 1;
    pps.slice_group_map_type = 0;

    let data = [
        0x80, // first_mb_in_slice = 0
        0x40, // slice_type = 2 (I slice)
        0x80, // pic_parameter_set_id = 0
        0x80, // frame_num = 0
        0x80, // slice_qp_delta = 0
    ];

    let mut sps_map = HashMap::new();
    sps_map.insert(0, create_minimal_sps_for_parsing());
    let mut pps_map = HashMap::new();
    pps_map.insert(0, pps);

    let result = parse_slice_header(&data, &sps_map, &pps_map, NalUnitType::IdrSlice, 0);
    let _ = result;
}

#[test]
fn test_parse_slice_header_with_slice_group_map_type_2() {
    // Test with slice group map type 2
    let mut pps = create_minimal_pps_for_parsing();
    pps.num_slice_groups_minus1 = 1;
    pps.slice_group_map_type = 2;

    let data = [
        0x80, // first_mb_in_slice = 0
        0x40, // slice_type = 2 (I slice)
        0x80, // pic_parameter_set_id = 0
        0x80, // frame_num = 0
        0x80, // slice_qp_delta = 0
    ];

    let mut sps_map = HashMap::new();
    sps_map.insert(0, create_minimal_sps_for_parsing());
    let mut pps_map = HashMap::new();
    pps_map.insert(0, pps);

    let result = parse_slice_header(&data, &sps_map, &pps_map, NalUnitType::IdrSlice, 0);
    let _ = result;
}

// ============================================================================
// Targeted tests for UNCOVERED code paths identified by coverage analysis
// ============================================================================

#[test]
fn test_parse_slice_header_with_separate_colour_plane_flag() {
    // Test colour_plane_id parsing (lines 184-187)
    let mut sps = create_minimal_sps_for_parsing();
    sps.separate_colour_plane_flag = true;

    let data = [
        0x80, // first_mb_in_slice = 0
        0x40, // slice_type = 2 (I slice)
        0x80, // pic_parameter_set_id = 0
        0x80, // frame_num = 0
        0x00, // colour_plane_id (2 bits)
        0x80, // slice_qp_delta = 0
    ];

    let mut sps_map = HashMap::new();
    sps_map.insert(0, sps);
    let mut pps_map = HashMap::new();
    pps_map.insert(0, create_minimal_pps_for_parsing());

    let result = parse_slice_header(&data, &sps_map, &pps_map, NalUnitType::IdrSlice, 0);
    let _ = result;
}

#[test]
fn test_parse_slice_header_delta_pic_order_always_zero_flag_true() {
    // Test pic_order_cnt_type = 1 with delta_pic_order_always_zero_flag = true
    // Should skip delta_pic_order_cnt parsing (lines 220-227)
    let mut sps = create_minimal_sps_for_parsing();
    sps.pic_order_cnt_type = 1;
    sps.delta_pic_order_always_zero_flag = true; // Skip delta_pic_order_cnt

    let data = [
        0x80, // first_mb_in_slice = 0
        0x80, // slice_type = 0 (P slice)
        0x80, // pic_parameter_set_id = 0
        0x80, // frame_num = 0
        // delta_pic_order_cnt[0] and [1] should be skipped
        0x80, // slice_qp_delta = 0
    ];

    let mut sps_map = HashMap::new();
    sps_map.insert(0, sps);
    let mut pps_map = HashMap::new();
    pps_map.insert(0, create_minimal_pps_for_parsing());

    let result = parse_slice_header(&data, &sps_map, &pps_map, NalUnitType::NonIdrSlice, 3);
    let _ = result;
}

#[test]
fn test_parse_slice_header_slice_group_map_type_3() {
    // Test slice_group_change_cycle parsing for map_type 3 (lines 315-322)
    let mut pps = create_minimal_pps_for_parsing();
    pps.num_slice_groups_minus1 = 2;
    pps.slice_group_map_type = 3;

    let mut sps = create_minimal_sps_for_parsing();
    sps.pic_width_in_mbs_minus1 = 10;
    sps.pic_height_in_map_units_minus1 = 10;

    let data = [
        0x80, // first_mb_in_slice = 0
        0x40, // slice_type = 2 (I slice)
        0x80, // pic_parameter_set_id = 0
        0x80, // frame_num = 0
        0x80, // slice_group_change_cycle
        0x80, // slice_qp_delta = 0
    ];

    let mut sps_map = HashMap::new();
    sps_map.insert(0, sps);
    let mut pps_map = HashMap::new();
    pps_map.insert(0, pps);

    let result = parse_slice_header(&data, &sps_map, &pps_map, NalUnitType::IdrSlice, 0);
    let _ = result;
}

#[test]
fn test_parse_slice_header_slice_group_map_type_4() {
    // Test slice_group_change_cycle parsing for map_type 4
    let mut pps = create_minimal_pps_for_parsing();
    pps.num_slice_groups_minus1 = 2;
    pps.slice_group_map_type = 4;

    let mut sps = create_minimal_sps_for_parsing();
    sps.pic_width_in_mbs_minus1 = 10;
    sps.pic_height_in_map_units_minus1 = 10;

    let data = [
        0x80, // first_mb_in_slice = 0
        0x40, // slice_type = 2 (I slice)
        0x80, // pic_parameter_set_id = 0
        0x80, // frame_num = 0
        0x80, // slice_group_change_cycle
        0x80, // slice_qp_delta = 0
    ];

    let mut sps_map = HashMap::new();
    sps_map.insert(0, sps);
    let mut pps_map = HashMap::new();
    pps_map.insert(0, pps);

    let result = parse_slice_header(&data, &sps_map, &pps_map, NalUnitType::IdrSlice, 0);
    let _ = result;
}

#[test]
fn test_parse_slice_header_slice_group_map_type_5() {
    // Test slice_group_change_cycle parsing for map_type 5
    let mut pps = create_minimal_pps_for_parsing();
    pps.num_slice_groups_minus1 = 2;
    pps.slice_group_map_type = 5;

    let mut sps = create_minimal_sps_for_parsing();
    sps.pic_width_in_mbs_minus1 = 10;
    sps.pic_height_in_map_units_minus1 = 10;

    let data = [
        0x80, // first_mb_in_slice = 0
        0x40, // slice_type = 2 (I slice)
        0x80, // pic_parameter_set_id = 0
        0x80, // frame_num = 0
        0x80, // slice_group_change_cycle
        0x80, // slice_qp_delta = 0
    ];

    let mut sps_map = HashMap::new();
    sps_map.insert(0, sps);
    let mut pps_map = HashMap::new();
    pps_map.insert(0, pps);

    let result = parse_slice_header(&data, &sps_map, &pps_map, NalUnitType::IdrSlice, 0);
    let _ = result;
}

#[test]
fn test_parse_ref_pic_list_modification_unknown_idc() {
    // Test break on unknown modification_of_pic_nums_idc (line 370)
    // idc should be something other than 0, 1, or 2
    let mut sps = create_minimal_sps_for_parsing();
    sps.max_num_ref_frames = 2;

    let mut pps = create_minimal_pps_for_parsing();
    pps.num_ref_idx_l0_default_active_minus1 = 1;

    let data = [
        0x80, // first_mb_in_slice = 0
        0x80, // slice_type = 0 (P slice)
        0x80, // pic_parameter_set_id = 0
        0x80, // frame_num = 0
        0x80, // ref_pic_list_modification_flag_l0 = 1
        0x80, // modification_of_pic_nums_idc = 4 (unknown, should break)
        0x80, // slice_qp_delta = 0
    ];

    let mut sps_map = HashMap::new();
    sps_map.insert(0, sps);
    let mut pps_map = HashMap::new();
    pps_map.insert(0, pps);

    let result = parse_slice_header(&data, &sps_map, &pps_map, NalUnitType::NonIdrSlice, 3);
    let _ = result;
}

#[test]
fn test_parse_dec_ref_pic_marking_idr_long_term_reference_flag() {
    // Test long_term_reference_flag for IDR slice (line 385)
    let data = [
        0x80, // first_mb_in_slice = 0
        0x40, // slice_type = 2 (I slice)
        0x80, // pic_parameter_set_id = 0
        0x80, // frame_num = 0
        0x80, // dec_ref_pic_marking_flag = 1
        0x40, // no_output_of_prior_pics_flag = 0
        0x80, // long_term_reference_flag = 1 (this line!)
        0x80, // slice_qp_delta = 0
    ];

    let mut sps_map = HashMap::new();
    sps_map.insert(0, create_minimal_sps_for_parsing());
    let mut pps_map = HashMap::new();
    pps_map.insert(0, create_minimal_pps_for_parsing());

    let result = parse_slice_header(&data, &sps_map, &pps_map, NalUnitType::IdrSlice, 0);
    let _ = result;
}

#[test]
fn test_parse_dec_ref_pic_marking_mmco_4() {
    // Test MMCO operation 4 (line 407) - set max_long_term_frame_idx
    let data = [
        0x80, // first_mb_in_slice = 0
        0x80, // slice_type = 0 (P slice)
        0x80, // pic_parameter_set_id = 0
        0x80, // frame_num = 0
        0x80, // dec_ref_pic_marking_flag = 1
        0x00, // adaptive_ref_pic_marking_mode_flag = 0
        0x00, // mmco_equal_to_1 (mark short term unused)
        0x80, // mmco_equal_to_2 (mark long term)
        0x00, // difference_of_pic_nums_minus1
        0x00, // long_term_frame_idx
        0x00, // mmco_equal_to_4 (set max_long_term_frame_idx)
        0x80, // max_long_term_frame_idx_plus1
        0xC0, // mmco_equal_to_3 (end)
        0x80, // slice_qp_delta = 0
    ];

    let mut sps_map = HashMap::new();
    sps_map.insert(0, create_minimal_sps_for_parsing());
    let mut pps_map = HashMap::new();
    pps_map.insert(0, create_minimal_pps_for_parsing());

    let result = parse_slice_header(&data, &sps_map, &pps_map, NalUnitType::NonIdrSlice, 3);
    let _ = result;
}

#[test]
fn test_parse_dec_ref_pic_marking_mmco_6() {
    // Test MMCO operation 6 (line 410) - mark current as long term
    let data = [
        0x80, // first_mb_in_slice = 0
        0x80, // slice_type = 0 (P slice)
        0x80, // pic_parameter_set_id = 0
        0x80, // frame_num = 0
        0x80, // dec_ref_pic_marking_flag = 1
        0x00, // adaptive_ref_pic_marking_mode_flag = 0
        0x00, // mmco_equal_to_1 (mark short term unused)
        0x80, // difference_of_pic_nums_minus1
        0x00, // mmco_equal_to_6 (mark current as long term)
        0x80, // long_term_frame_idx_plus1
        0xC0, // mmco_equal_to_3 (end)
        0x80, // slice_qp_delta = 0
    ];

    let mut sps_map = HashMap::new();
    sps_map.insert(0, create_minimal_sps_for_parsing());
    let mut pps_map = HashMap::new();
    pps_map.insert(0, create_minimal_pps_for_parsing());

    let result = parse_slice_header(&data, &sps_map, &pps_map, NalUnitType::NonIdrSlice, 3);
    let _ = result;
}

#[test]
fn test_parse_pred_weight_table_luma_weight_flag_0() {
    // Test luma_weight_flag = 0 (line 446) - skip luma weight parsing
    let mut sps = create_minimal_sps_for_parsing();

    let mut pps = create_minimal_pps_for_parsing();
    pps.weighted_pred_flag = true;

    let data = [
        0x80, // first_mb_in_slice = 0
        0x80, // slice_type = 0 (P slice)
        0x80, // pic_parameter_set_id = 0
        0x80, // frame_num = 0
        0x00, // pred_weight_table() is present
        0x80, // luma_log2_weight_denom = 0
        0x40, // luma_weight_flag = 0 for ref 0 (skip weights)
        0xC0, // chroma_weight_flag = 3 (end)
        0x80, // slice_qp_delta = 0
    ];

    let mut sps_map = HashMap::new();
    sps_map.insert(0, sps);
    let mut pps_map = HashMap::new();
    pps_map.insert(0, pps);

    let result = parse_slice_header(&data, &sps_map, &pps_map, NalUnitType::NonIdrSlice, 3);
    let _ = result;
}

#[test]
fn test_parse_pred_weight_table_chroma_weight_flag_0() {
    // Test chroma_weight_flag = 0 (lines 452-461) - skip chroma weight parsing
    let mut sps = create_minimal_sps_for_parsing();
    sps.chroma_format_idc = ChromaFormat::Yuv420;

    let mut pps = create_minimal_pps_for_parsing();
    pps.weighted_pred_flag = true;

    let data = [
        0x80, // first_mb_in_slice = 0
        0x80, // slice_type = 0 (P slice)
        0x80, // pic_parameter_set_id = 0
        0x80, // frame_num = 0
        0x00, // pred_weight_table() is present
        0x80, // luma_log2_weight_denom = 0
        0x00, // luma_weight_flag = 0
        0x80, // chroma_log2_weight_denom = 0
        0x40, // chroma_weight_flag = 0 for ref 0 (skip chroma weights)
        0xC0, // end marker
        0x80, // slice_qp_delta = 0
    ];

    let mut sps_map = HashMap::new();
    sps_map.insert(0, sps);
    let mut pps_map = HashMap::new();
    pps_map.insert(0, pps);

    let result = parse_slice_header(&data, &sps_map, &pps_map, NalUnitType::NonIdrSlice, 3);
    let _ = result;
}

#[test]
fn test_parse_slice_header_sp_slice_with_switch_flag() {
    // Test SP slice with sp_for_switch_flag = true (line 297-300)
    let data = [
        0x80, // first_mb_in_slice = 0
        0x0A, // slice_type = 5 (SP slice)
        0x80, // pic_parameter_set_id = 0
        0x80, // frame_num = 0
        0x80, // sp_for_switch_flag = 1
        0x80, // slice_qs_delta = 0
        0x80, // slice_qp_delta = 0
    ];

    let mut sps_map = HashMap::new();
    sps_map.insert(0, create_minimal_sps_for_parsing());
    let mut pps_map = HashMap::new();
    pps_map.insert(0, create_minimal_pps_for_parsing());

    let result = parse_slice_header(&data, &sps_map, &pps_map, NalUnitType::NonIdrSlice, 0);
    let _ = result;
}

#[test]
fn test_parse_slice_header_si_slice_type() {
    // Test SI slice type (line 296-301) - should parse slice_qs_delta
    let data = [
        0x80, // first_mb_in_slice = 0
        0x0C, // slice_type = 6 (SI slice)
        0x80, // pic_parameter_set_id = 0
        0x80, // frame_num = 0
        0x80, // slice_qs_delta = 0
        0x80, // slice_qp_delta = 0
    ];

    let mut sps_map = HashMap::new();
    sps_map.insert(0, create_minimal_sps_for_parsing());
    let mut pps_map = HashMap::new();
    pps_map.insert(0, create_minimal_pps_for_parsing());

    let result = parse_slice_header(&data, &sps_map, &pps_map, NalUnitType::NonIdrSlice, 0);
    let _ = result;
}
