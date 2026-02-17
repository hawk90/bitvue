#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(unused_comparisons)]
#![allow(unused_doc_comments)]
//! AVC Slice Header Parsing Tests
//!
//! Comprehensive tests for AVC slice header parsing functionality.

use bitvue_avc::slice::{DecRefPicMarking, RefPicListModification, SliceHeader, SliceType};

// SliceType tests

#[test]
fn test_slice_type_from_u32() {
    assert_eq!(SliceType::from_u32(0), SliceType::P);
    assert_eq!(SliceType::from_u32(1), SliceType::B);
    assert_eq!(SliceType::from_u32(2), SliceType::I);
    assert_eq!(SliceType::from_u32(3), SliceType::Sp);
    assert_eq!(SliceType::from_u32(4), SliceType::Si);
    // Test modulo 5 wrapping
    assert_eq!(SliceType::from_u32(5), SliceType::P);
    assert_eq!(SliceType::from_u32(7), SliceType::I); // 7 % 5 = 2
    assert_eq!(SliceType::from_u32(9), SliceType::Si); // 9 % 5 = 4
    assert_eq!(SliceType::from_u32(255), SliceType::P); // 255 % 5 = 0
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
fn test_slice_type_is_b() {
    assert!(SliceType::B.is_b());
    assert!(!SliceType::I.is_b());
    assert!(!SliceType::P.is_b());
    assert!(!SliceType::Sp.is_b());
    assert!(!SliceType::Si.is_b());
}

#[test]
fn test_slice_type_is_p() {
    assert!(SliceType::P.is_p());
    assert!(SliceType::Sp.is_p());
    assert!(!SliceType::I.is_p());
    assert!(!SliceType::B.is_p());
    assert!(!SliceType::Si.is_p());
}

#[test]
fn test_slice_type_name() {
    assert_eq!(SliceType::P.name(), "P");
    assert_eq!(SliceType::B.name(), "B");
    assert_eq!(SliceType::I.name(), "I");
    assert_eq!(SliceType::Sp.name(), "SP");
    assert_eq!(SliceType::Si.name(), "SI");
}

// Helper function to create a default SliceHeader
fn default_slice_header() -> SliceHeader {
    SliceHeader {
        first_mb_in_slice: 0,
        slice_type: SliceType::I,
        pic_parameter_set_id: 0,
        colour_plane_id: 0,
        frame_num: 0,
        field_pic_flag: false,
        bottom_field_flag: false,
        idr_pic_id: 0,
        pic_order_cnt_lsb: 0,
        delta_pic_order_cnt_bottom: 0,
        delta_pic_order_cnt: [0, 0],
        redundant_pic_cnt: 0,
        direct_spatial_mv_pred_flag: false,
        num_ref_idx_active_override_flag: false,
        num_ref_idx_l0_active_minus1: 0,
        num_ref_idx_l1_active_minus1: 0,
        ref_pic_list_modification_flag_l0: false,
        ref_pic_list_modification_flag_l1: false,
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
        cabac_init_idc: 0,
        slice_qp_delta: 0,
        sp_for_switch_flag: false,
        slice_qs_delta: 0,
        disable_deblocking_filter_idc: 0,
        slice_alpha_c0_offset_div2: 0,
        slice_beta_offset_div2: 0,
        slice_group_change_cycle: 0,
    }
}

// SliceHeader tests

#[test]
fn test_slice_header_basic() {
    let header = default_slice_header();

    assert_eq!(header.first_mb_in_slice, 0);
    assert_eq!(header.slice_type, SliceType::I);
}

#[test]
fn test_slice_header_i_slice() {
    let mut header = default_slice_header();
    header.frame_num = 100;

    assert!(header.slice_type.is_intra());
    assert!(!header.slice_type.is_b());
    assert!(!header.slice_type.is_p());
    assert_eq!(header.frame_num, 100);
}

#[test]
fn test_slice_header_p_slice() {
    let mut header = default_slice_header();
    header.slice_type = SliceType::P;
    header.num_ref_idx_l0_active_minus1 = 5;
    header.num_ref_idx_l1_active_minus1 = 0;
    header.ref_pic_list_modification_flag_l0 = true;

    assert!(header.slice_type.is_p());
    assert!(!header.slice_type.is_intra());
    assert_eq!(header.num_ref_idx_l0_active_minus1, 5);
}

#[test]
fn test_slice_header_b_slice() {
    let mut header = default_slice_header();
    header.first_mb_in_slice = 120;
    header.slice_type = SliceType::B;
    header.pic_parameter_set_id = 1;
    header.num_ref_idx_l0_active_minus1 = 3;
    header.num_ref_idx_l1_active_minus1 = 2;
    header.direct_spatial_mv_pred_flag = true;
    header.ref_pic_list_modification_flag_l0 = true;
    header.ref_pic_list_modification_flag_l1 = true;

    assert!(header.slice_type.is_b());
    assert!(header.direct_spatial_mv_pred_flag);
    assert_eq!(header.num_ref_idx_l0_active_minus1, 3);
    assert_eq!(header.num_ref_idx_l1_active_minus1, 2);
}

#[test]
fn test_slice_header_sp_slice() {
    let mut header = default_slice_header();
    header.slice_type = SliceType::Sp;
    header.sp_for_switch_flag = true;
    header.slice_qs_delta = 5;

    assert!(header.slice_type.is_p());
    assert!(header.sp_for_switch_flag);
    assert_eq!(header.slice_qs_delta, 5);
}

#[test]
fn test_slice_header_si_slice() {
    let mut header = default_slice_header();
    header.slice_type = SliceType::Si;
    header.slice_qs_delta = -3;

    assert!(header.slice_type.is_intra());
    assert_eq!(header.slice_qs_delta, -3);
}

#[test]
fn test_slice_header_qp_offsets() {
    let mut header = default_slice_header();
    header.slice_qp_delta = 5;

    assert_eq!(header.slice_qp_delta, 5);
}

#[test]
fn test_slice_header_deblocking() {
    let mut header = default_slice_header();
    header.disable_deblocking_filter_idc = 1;
    header.slice_alpha_c0_offset_div2 = 2;
    header.slice_beta_offset_div2 = 3;

    assert_eq!(header.disable_deblocking_filter_idc, 1);
    assert_eq!(header.slice_alpha_c0_offset_div2, 2);
    assert_eq!(header.slice_beta_offset_div2, 3);
}

#[test]
fn test_slice_header_cabac() {
    let mut header = default_slice_header();
    header.cabac_init_idc = 2;

    assert_eq!(header.cabac_init_idc, 2);
}

#[test]
fn test_slice_header_pic_order_count() {
    let mut header = default_slice_header();
    header.pic_order_cnt_lsb = 12345;
    header.delta_pic_order_cnt_bottom = -10;
    header.delta_pic_order_cnt = [5, -3];

    assert_eq!(header.pic_order_cnt_lsb, 12345);
    assert_eq!(header.delta_pic_order_cnt_bottom, -10);
    assert_eq!(header.delta_pic_order_cnt[0], 5);
    assert_eq!(header.delta_pic_order_cnt[1], -3);
}

#[test]
fn test_slice_header_field_pic() {
    let mut header = default_slice_header();
    header.field_pic_flag = true;
    header.bottom_field_flag = true;

    assert!(header.field_pic_flag);
    assert!(header.bottom_field_flag);
}

#[test]
fn test_slice_header_idr_pic_id() {
    let mut header = default_slice_header();
    header.idr_pic_id = 42;

    assert_eq!(header.idr_pic_id, 42);
}

#[test]
fn test_slice_header_ref_idx_override() {
    let mut header = default_slice_header();
    header.num_ref_idx_active_override_flag = true;
    header.num_ref_idx_l0_active_minus1 = 10;
    header.num_ref_idx_l1_active_minus1 = 5;

    assert!(header.num_ref_idx_active_override_flag);
    assert_eq!(header.num_ref_idx_l0_active_minus1, 10);
    assert_eq!(header.num_ref_idx_l1_active_minus1, 5);
}

#[test]
fn test_slice_header_redundant_pic_cnt() {
    let mut header = default_slice_header();
    header.redundant_pic_cnt = 7;

    assert_eq!(header.redundant_pic_cnt, 7);
}

#[test]
fn test_slice_header_colour_plane_id() {
    let mut header = default_slice_header();
    header.colour_plane_id = 2;

    assert_eq!(header.colour_plane_id, 2);
}

#[test]
fn test_slice_header_frame_num() {
    let mut header = default_slice_header();
    header.frame_num = 12345;

    assert_eq!(header.frame_num, 12345);
}

#[test]
fn test_slice_header_first_mb_in_slice() {
    let mut header = default_slice_header();
    header.first_mb_in_slice = 99;

    assert_eq!(header.first_mb_in_slice, 99);
    assert!(!header.is_first_slice());
}

#[test]
fn test_slice_header_is_first_slice() {
    let header = default_slice_header();

    assert!(header.is_first_slice());
}

#[test]
fn test_slice_header_slice_group_change_cycle() {
    let mut header = default_slice_header();
    header.slice_group_change_cycle = 255;

    assert_eq!(header.slice_group_change_cycle, 255);
}

// RefPicListModification tests

#[test]
fn test_ref_pic_list_modification_default() {
    let ref_mod = RefPicListModification {
        modifications: vec![],
    };

    assert!(ref_mod.modifications.is_empty());
}

#[test]
fn test_ref_pic_list_modification_with_data() {
    let ref_mod = RefPicListModification {
        modifications: vec![(0, 10), (1, 5), (2, 3)],
    };

    assert_eq!(ref_mod.modifications.len(), 3);
    assert_eq!(ref_mod.modifications[0], (0, 10));
    assert_eq!(ref_mod.modifications[1], (1, 5));
}

// DecRefPicMarking tests

#[test]
fn test_dec_ref_pic_marking_default() {
    let marking = DecRefPicMarking {
        no_output_of_prior_pics_flag: false,
        long_term_reference_flag: false,
        adaptive_ref_pic_marking_mode_flag: false,
        mmco_operations: vec![],
    };

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
}

#[test]
fn test_dec_ref_pic_marking_adaptive() {
    let marking = DecRefPicMarking {
        no_output_of_prior_pics_flag: false,
        long_term_reference_flag: false,
        adaptive_ref_pic_marking_mode_flag: true,
        mmco_operations: vec![(1, 10, 0), (3, 5, 2), (0, 0, 0)],
    };

    assert!(marking.adaptive_ref_pic_marking_mode_flag);
    assert_eq!(marking.mmco_operations.len(), 3);
}

#[test]
fn test_slice_header_complete_p_slice() {
    let mut header = default_slice_header();
    header.slice_type = SliceType::P;
    header.frame_num = 100;
    header.num_ref_idx_l0_active_minus1 = 5;
    header.slice_qp_delta = 3;
    header.cabac_init_idc = 0;
    header.disable_deblocking_filter_idc = 0;

    assert!(header.slice_type.is_p());
    assert!(header.is_first_slice());
}

#[test]
fn test_slice_header_complete_b_slice() {
    let mut header = default_slice_header();
    header.slice_type = SliceType::B;
    header.frame_num = 100;
    header.direct_spatial_mv_pred_flag = true;
    header.num_ref_idx_l0_active_minus1 = 3;
    header.num_ref_idx_l1_active_minus1 = 3;
    header.slice_qp_delta = -2;
    header.disable_deblocking_filter_idc = 0;
    header.slice_alpha_c0_offset_div2 = 1;
    header.slice_beta_offset_div2 = 1;

    assert!(header.slice_type.is_b());
    assert!(header.direct_spatial_mv_pred_flag);
}
