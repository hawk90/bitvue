#![allow(dead_code)]
//! Comprehensive tests for AVC slice module
//!
//! Tests SliceType, SliceHeader, RefPicListModification, DecRefPicMarking

use bitvue_avc::nal::NalUnitType;
use bitvue_avc::pps::Pps;
use bitvue_avc::slice::{
    parse_slice_header, DecRefPicMarking, RefPicListModification, SliceHeader, SliceType,
};
use bitvue_avc::sps::{ChromaFormat, ProfileIdc, Sps};
use std::collections::HashMap;

// ============================================================================
// SliceType Tests
// ============================================================================

#[test]
fn test_slice_type_from_u32_all_variants() {
    assert_eq!(SliceType::from_u32(0), SliceType::P);
    assert_eq!(SliceType::from_u32(1), SliceType::B);
    assert_eq!(SliceType::from_u32(2), SliceType::I);
    assert_eq!(SliceType::from_u32(3), SliceType::Sp);
    assert_eq!(SliceType::from_u32(4), SliceType::Si);
}

#[test]
fn test_slice_type_from_u32_modulo() {
    // Test that values wrap around with modulo 5
    assert_eq!(SliceType::from_u32(5), SliceType::P);
    assert_eq!(SliceType::from_u32(6), SliceType::B);
    assert_eq!(SliceType::from_u32(7), SliceType::I);
    assert_eq!(SliceType::from_u32(8), SliceType::Sp);
    assert_eq!(SliceType::from_u32(9), SliceType::Si);
    assert_eq!(SliceType::from_u32(10), SliceType::P);
}

#[test]
fn test_slice_type_from_u32_large_values() {
    assert_eq!(SliceType::from_u32(100), SliceType::P); // 100 % 5 = 0
    assert_eq!(SliceType::from_u32(101), SliceType::B); // 101 % 5 = 1
    assert_eq!(SliceType::from_u32(102), SliceType::I); // 102 % 5 = 2
    assert_eq!(SliceType::from_u32(999), SliceType::Si); // 999 % 5 = 4
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
    assert!(!SliceType::P.is_b());
    assert!(!SliceType::I.is_b());
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

#[test]
fn test_slice_type_copy() {
    let p = SliceType::P;
    let copied = p;
    assert_eq!(p, SliceType::P);
    assert_eq!(copied, SliceType::P);
}

#[test]
fn test_slice_type_eq() {
    assert_eq!(SliceType::P, SliceType::P);
    assert_eq!(SliceType::B, SliceType::B);
    assert_ne!(SliceType::P, SliceType::B);
    assert_ne!(SliceType::I, SliceType::Si);
}

// ============================================================================
// RefPicListModification Tests
// ============================================================================

#[test]
fn test_ref_pic_list_modification_default() {
    let ref_pic = RefPicListModification::default();
    assert!(ref_pic.modifications.is_empty());
}

#[test]
fn test_ref_pic_list_modification_new() {
    let ref_pic = RefPicListModification {
        modifications: vec![(0, 100), (1, 200), (2, 300)],
    };

    assert_eq!(ref_pic.modifications.len(), 3);
    assert_eq!(ref_pic.modifications[0], (0, 100));
    assert_eq!(ref_pic.modifications[1], (1, 200));
    assert_eq!(ref_pic.modifications[2], (2, 300));
}

#[test]
fn test_ref_pic_list_modification_clone() {
    let ref_pic = RefPicListModification {
        modifications: vec![(0, 1), (1, 2)],
    };

    let cloned = ref_pic.clone();
    assert_eq!(cloned.modifications.len(), 2);
    assert_eq!(cloned.modifications[0], (0, 1));
}

// ============================================================================
// DecRefPicMarking Tests
// ============================================================================

#[test]
fn test_dec_ref_pic_marking_default() {
    let marking = DecRefPicMarking::default();
    assert!(!marking.no_output_of_prior_pics_flag);
    assert!(!marking.long_term_reference_flag);
    assert!(!marking.adaptive_ref_pic_marking_mode_flag);
    assert!(marking.mmco_operations.is_empty());
}

#[test]
fn test_dec_ref_pic_marking_with_flags() {
    let marking = DecRefPicMarking {
        no_output_of_prior_pics_flag: true,
        long_term_reference_flag: true,
        adaptive_ref_pic_marking_mode_flag: true,
        mmco_operations: vec![(1, 10, 0), (2, 0, 5)],
    };

    assert!(marking.no_output_of_prior_pics_flag);
    assert!(marking.long_term_reference_flag);
    assert!(marking.adaptive_ref_pic_marking_mode_flag);
    assert_eq!(marking.mmco_operations.len(), 2);
}

#[test]
fn test_dec_ref_pic_marking_clone() {
    let marking = DecRefPicMarking {
        no_output_of_prior_pics_flag: true,
        long_term_reference_flag: false,
        adaptive_ref_pic_marking_mode_flag: false,
        mmco_operations: vec![(3, 5, 2)],
    };

    let cloned = marking.clone();
    assert!(cloned.no_output_of_prior_pics_flag);
    assert_eq!(cloned.mmco_operations.len(), 1);
}

// ============================================================================
// SliceHeader Tests
// ============================================================================

#[test]
fn test_slice_header_default_fields() {
    let header = SliceHeader {
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
        ref_pic_list_modification_l0: RefPicListModification::default(),
        ref_pic_list_modification_l1: RefPicListModification::default(),
        dec_ref_pic_marking: DecRefPicMarking::default(),
        cabac_init_idc: 0,
        slice_qp_delta: 0,
        sp_for_switch_flag: false,
        slice_qs_delta: 0,
        disable_deblocking_filter_idc: 0,
        slice_alpha_c0_offset_div2: 0,
        slice_beta_offset_div2: 0,
        slice_group_change_cycle: 0,
    };

    assert_eq!(header.first_mb_in_slice, 0);
    assert_eq!(header.slice_type, SliceType::I);
}

#[test]
fn test_slice_header_with_values() {
    let header = SliceHeader {
        first_mb_in_slice: 10,
        slice_type: SliceType::P,
        pic_parameter_set_id: 5,
        colour_plane_id: 1,
        frame_num: 100,
        field_pic_flag: true,
        bottom_field_flag: true,
        idr_pic_id: 50,
        pic_order_cnt_lsb: 200,
        delta_pic_order_cnt_bottom: -5,
        delta_pic_order_cnt: [1, 2],
        redundant_pic_cnt: 3,
        direct_spatial_mv_pred_flag: true,
        num_ref_idx_active_override_flag: true,
        num_ref_idx_l0_active_minus1: 15,
        num_ref_idx_l1_active_minus1: 10,
        ref_pic_list_modification_flag_l0: true,
        ref_pic_list_modification_flag_l1: true,
        ref_pic_list_modification_l0: RefPicListModification {
            modifications: vec![(0, 100)],
        },
        ref_pic_list_modification_l1: RefPicListModification {
            modifications: vec![(1, 200)],
        },
        dec_ref_pic_marking: DecRefPicMarking {
            no_output_of_prior_pics_flag: true,
            long_term_reference_flag: false,
            adaptive_ref_pic_marking_mode_flag: false,
            mmco_operations: vec![],
        },
        cabac_init_idc: 2,
        slice_qp_delta: 5,
        sp_for_switch_flag: true,
        slice_qs_delta: -3,
        disable_deblocking_filter_idc: 1,
        slice_alpha_c0_offset_div2: 3,
        slice_beta_offset_div2: -2,
        slice_group_change_cycle: 100,
    };

    assert_eq!(header.first_mb_in_slice, 10);
    assert_eq!(header.slice_type, SliceType::P);
    assert_eq!(header.pic_parameter_set_id, 5);
    assert!(header.field_pic_flag);
    assert!(header.bottom_field_flag);
}

// ============================================================================
// SliceHeader::qp() Tests
// ============================================================================

#[test]
fn test_slice_header_qp_zero_delta() {
    let header = SliceHeader {
        slice_qp_delta: 0,
        ..default_slice_header()
    };

    let mut pps = Pps {
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
    };

    assert_eq!(header.qp(&pps), 26);
}

#[test]
fn test_slice_header_qp_positive_delta() {
    let header = SliceHeader {
        slice_qp_delta: 10,
        ..default_slice_header()
    };

    let mut pps = Pps {
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
        pic_init_qp_minus26: 5,
        pic_init_qs_minus26: 0,
        chroma_qp_index_offset: 0,
        deblocking_filter_control_present_flag: false,
        constrained_intra_pred_flag: false,
        redundant_pic_cnt_present_flag: false,
        transform_8x8_mode_flag: false,
        pic_scaling_matrix_present_flag: false,
        second_chroma_qp_index_offset: 0,
    };

    assert_eq!(header.qp(&pps), 26 + 5 + 10); // 41
}

#[test]
fn test_slice_header_qp_negative_delta() {
    let header = SliceHeader {
        slice_qp_delta: -8,
        ..default_slice_header()
    };

    let mut pps = Pps {
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
        pic_init_qp_minus26: -3,
        pic_init_qs_minus26: 0,
        chroma_qp_index_offset: 0,
        deblocking_filter_control_present_flag: false,
        constrained_intra_pred_flag: false,
        redundant_pic_cnt_present_flag: false,
        transform_8x8_mode_flag: false,
        pic_scaling_matrix_present_flag: false,
        second_chroma_qp_index_offset: 0,
    };

    assert_eq!(header.qp(&pps), 26 - 3 - 8); // 15
}

// ============================================================================
// SliceHeader::is_first_slice() Tests
// ============================================================================

#[test]
fn test_slice_header_is_first_slice_zero() {
    let header = SliceHeader {
        first_mb_in_slice: 0,
        ..default_slice_header()
    };

    assert!(header.is_first_slice());
}

#[test]
fn test_slice_header_is_first_slice_non_zero() {
    let header = SliceHeader {
        first_mb_in_slice: 10,
        ..default_slice_header()
    };

    assert!(!header.is_first_slice());
}

// ============================================================================
// parse_slice_header() Tests
// ============================================================================

#[test]
fn test_parse_slice_header_empty() {
    let data: &[u8] = &[];
    let sps_map = HashMap::new();
    let pps_map = HashMap::new();
    let nal_type = NalUnitType::NonIdrSlice;
    let nal_ref_idc = 1;

    let result = parse_slice_header(data, &sps_map, &pps_map, nal_type, nal_ref_idc);
    assert!(result.is_err());
}

#[test]
fn test_parse_slice_header_insufficient_data() {
    let data = &[0x80]; // Only one byte
    let sps_map = HashMap::new();
    let pps_map = HashMap::new();
    let nal_type = NalUnitType::NonIdrSlice;
    let nal_ref_idc = 1;

    let result = parse_slice_header(data, &sps_map, &pps_map, nal_type, nal_ref_idc);
    assert!(result.is_err());
}

#[test]
fn test_parse_slice_header_missing_pps() {
    let data = &[0x80, 0x80]; // first_mb_in_slice=0, slice_type=0, pps_id=0
    let sps_map = HashMap::new();
    let pps_map = HashMap::new();
    let nal_type = NalUnitType::NonIdrSlice;
    let nal_ref_idc = 1;

    let result = parse_slice_header(data, &sps_map, &pps_map, nal_type, nal_ref_idc);
    assert!(result.is_err());
}

#[test]
fn test_parse_slice_header_missing_sps() {
    let data = &[0x80, 0x80]; // first_mb_in_slice=0, slice_type=0, pps_id=0
    let sps_map = HashMap::new();
    let mut pps_map = HashMap::new();
    pps_map.insert(
        0,
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
        },
    );

    let nal_type = NalUnitType::NonIdrSlice;
    let nal_ref_idc = 1;

    let result = parse_slice_header(data, &sps_map, &pps_map, nal_type, nal_ref_idc);
    assert!(result.is_err());
}

// ============================================================================
// Clone and Debug Tests
// ============================================================================

#[test]
fn test_slice_header_clone() {
    let header = SliceHeader {
        first_mb_in_slice: 5,
        slice_type: SliceType::B,
        pic_parameter_set_id: 2,
        ..default_slice_header()
    };

    let cloned = header.clone();
    assert_eq!(cloned.first_mb_in_slice, 5);
    assert_eq!(cloned.slice_type, SliceType::B);
}

#[test]
fn test_slice_header_debug() {
    let header = SliceHeader {
        first_mb_in_slice: 0,
        slice_type: SliceType::I,
        pic_parameter_set_id: 0,
        ..default_slice_header()
    };

    let debug_str = format!("{:?}", header);
    assert!(debug_str.contains("SliceHeader"));
}

#[test]
fn test_slice_type_debug() {
    let debug_str = format!("{:?}", SliceType::P);
    assert!(debug_str.contains("P"));
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_slice_header_all_false_flags() {
    let header = SliceHeader {
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
        ref_pic_list_modification_l0: RefPicListModification::default(),
        ref_pic_list_modification_l1: RefPicListModification::default(),
        dec_ref_pic_marking: DecRefPicMarking::default(),
        cabac_init_idc: 0,
        slice_qp_delta: 0,
        sp_for_switch_flag: false,
        slice_qs_delta: 0,
        disable_deblocking_filter_idc: 0,
        slice_alpha_c0_offset_div2: 0,
        slice_beta_offset_div2: 0,
        slice_group_change_cycle: 0,
    };

    assert!(!header.field_pic_flag);
    assert!(!header.bottom_field_flag);
    assert!(!header.direct_spatial_mv_pred_flag);
}

#[test]
fn test_dec_ref_pic_marking_all_mmco_operations() {
    let marking = DecRefPicMarking {
        no_output_of_prior_pics_flag: false,
        long_term_reference_flag: false,
        adaptive_ref_pic_marking_mode_flag: true,
        mmco_operations: vec![(1, 100, 0), (2, 0, 5), (3, 50, 2), (4, 0, 0), (6, 0, 3)],
    };

    assert_eq!(marking.mmco_operations.len(), 5);
}

#[test]
fn test_ref_pic_list_modification_various_idc() {
    let ref_pic = RefPicListModification {
        modifications: vec![(0, 1), (1, 2), (2, 3), (3, 4)],
    };

    assert_eq!(ref_pic.modifications[0].0, 0);
    assert_eq!(ref_pic.modifications[1].0, 1);
    assert_eq!(ref_pic.modifications[2].0, 2);
    assert_eq!(ref_pic.modifications[3].0, 3);
}

// ============================================================================
// Helper Function
// ============================================================================

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
        ref_pic_list_modification_l0: RefPicListModification::default(),
        ref_pic_list_modification_l1: RefPicListModification::default(),
        dec_ref_pic_marking: DecRefPicMarking::default(),
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
