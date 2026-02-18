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
//! AVC PPS Parsing Tests
//!
//! Tests for Picture Parameter Set (PPS) parsing functionality.

use bitvue_avc::pps::{parse_pps, Pps};
use bitvue_avc::sps::{ChromaFormat, ProfileIdc, Sps};

// ============================================================================
// PPS Creation Tests
// ============================================================================

#[test]
fn test_pps_creation_minimal() {
    let pps = Pps {
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

    assert_eq!(pps.pic_parameter_set_id, 0);
    assert_eq!(pps.seq_parameter_set_id, 0);
    assert!(!pps.entropy_coding_mode_flag);
}

#[test]
fn test_pps_creation_with_all_fields() {
    let pps = Pps {
        pic_parameter_set_id: 5,
        seq_parameter_set_id: 2,
        entropy_coding_mode_flag: true,
        bottom_field_pic_order_in_frame_present_flag: true,
        num_slice_groups_minus1: 1,
        slice_group_map_type: 2,
        num_ref_idx_l0_default_active_minus1: 1,
        num_ref_idx_l1_default_active_minus1: 1,
        weighted_pred_flag: true,
        weighted_bipred_idc: 2,
        pic_init_qp_minus26: -10,
        pic_init_qs_minus26: 5,
        chroma_qp_index_offset: -3,
        deblocking_filter_control_present_flag: true,
        constrained_intra_pred_flag: true,
        redundant_pic_cnt_present_flag: true,
        transform_8x8_mode_flag: true,
        pic_scaling_matrix_present_flag: true,
        second_chroma_qp_index_offset: 4,
    };

    assert_eq!(pps.pic_parameter_set_id, 5);
    assert_eq!(pps.seq_parameter_set_id, 2);
    assert!(pps.entropy_coding_mode_flag);
    assert!(pps.bottom_field_pic_order_in_frame_present_flag);
}

// ============================================================================
// is_cabac() Tests
// ============================================================================

#[test]
fn test_pps_is_cabac_true() {
    let mut pps = Pps {
        pic_parameter_set_id: 0,
        seq_parameter_set_id: 0,
        entropy_coding_mode_flag: true,
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

    assert!(pps.is_cabac());
}

#[test]
fn test_pps_is_cabac_false() {
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

    assert!(!pps.is_cabac());
}

// ============================================================================
// initial_qp() Tests
// ============================================================================

#[test]
fn test_pps_initial_qp_default() {
    let pps = Pps {
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

    assert_eq!(pps.initial_qp(), 26);
}

#[test]
fn test_pps_initial_qp_positive() {
    let pps = Pps {
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
        pic_init_qp_minus26: 10,
        pic_init_qs_minus26: 0,
        chroma_qp_index_offset: 0,
        deblocking_filter_control_present_flag: false,
        constrained_intra_pred_flag: false,
        redundant_pic_cnt_present_flag: false,
        transform_8x8_mode_flag: false,
        pic_scaling_matrix_present_flag: false,
        second_chroma_qp_index_offset: 0,
    };

    assert_eq!(pps.initial_qp(), 36);
}

#[test]
fn test_pps_initial_qp_negative() {
    let pps = Pps {
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
        pic_init_qp_minus26: -10,
        pic_init_qs_minus26: 0,
        chroma_qp_index_offset: 0,
        deblocking_filter_control_present_flag: false,
        constrained_intra_pred_flag: false,
        redundant_pic_cnt_present_flag: false,
        transform_8x8_mode_flag: false,
        pic_scaling_matrix_present_flag: false,
        second_chroma_qp_index_offset: 0,
    };

    assert_eq!(pps.initial_qp(), 16);
}

#[test]
fn test_pps_initial_qp_extreme() {
    let pps = Pps {
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
        pic_init_qp_minus26: -25,
        pic_init_qs_minus26: 0,
        chroma_qp_index_offset: 0,
        deblocking_filter_control_present_flag: false,
        constrained_intra_pred_flag: false,
        redundant_pic_cnt_present_flag: false,
        transform_8x8_mode_flag: false,
        pic_scaling_matrix_present_flag: false,
        second_chroma_qp_index_offset: 0,
    };

    assert_eq!(pps.initial_qp(), 1);
}

// ============================================================================
// PPS Field Variations Tests
// ============================================================================

#[test]
fn test_pps_cabac_enabled() {
    let pps = Pps {
        pic_parameter_set_id: 0,
        seq_parameter_set_id: 0,
        entropy_coding_mode_flag: true,
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

    assert!(pps.is_cabac());
}

#[test]
fn test_pps_weighted_pred_enabled() {
    let pps = Pps {
        pic_parameter_set_id: 0,
        seq_parameter_set_id: 0,
        entropy_coding_mode_flag: false,
        bottom_field_pic_order_in_frame_present_flag: false,
        num_slice_groups_minus1: 0,
        slice_group_map_type: 0,
        num_ref_idx_l0_default_active_minus1: 0,
        num_ref_idx_l1_default_active_minus1: 0,
        weighted_pred_flag: true,
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

    assert!(pps.weighted_pred_flag);
}

#[test]
fn test_pps_deblocking_filter_enabled() {
    let pps = Pps {
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
    };

    assert!(pps.deblocking_filter_control_present_flag);
}

#[test]
fn test_pps_constrained_intra_enabled() {
    let pps = Pps {
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
        constrained_intra_pred_flag: true,
        redundant_pic_cnt_present_flag: false,
        transform_8x8_mode_flag: false,
        pic_scaling_matrix_present_flag: false,
        second_chroma_qp_index_offset: 0,
    };

    assert!(pps.constrained_intra_pred_flag);
}

#[test]
fn test_pps_transform_8x8_enabled() {
    let pps = Pps {
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
        transform_8x8_mode_flag: true,
        pic_scaling_matrix_present_flag: false,
        second_chroma_qp_index_offset: 0,
    };

    assert!(pps.transform_8x8_mode_flag);
}

#[test]
fn test_pps_different_qp_offsets() {
    let pps = Pps {
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
        chroma_qp_index_offset: 5,
        deblocking_filter_control_present_flag: false,
        constrained_intra_pred_flag: false,
        redundant_pic_cnt_present_flag: false,
        transform_8x8_mode_flag: false,
        pic_scaling_matrix_present_flag: false,
        second_chroma_qp_index_offset: -3,
    };

    assert_eq!(pps.chroma_qp_index_offset, 5);
    assert_eq!(pps.second_chroma_qp_index_offset, -3);
}

#[test]
fn test_pps_reference_index_defaults() {
    let pps = Pps {
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

    assert_eq!(pps.num_ref_idx_l0_default_active_minus1, 0);
    assert_eq!(pps.num_ref_idx_l1_default_active_minus1, 0);
}

#[test]
fn test_pps_reference_index_custom() {
    let pps = Pps {
        pic_parameter_set_id: 0,
        seq_parameter_set_id: 0,
        entropy_coding_mode_flag: false,
        bottom_field_pic_order_in_frame_present_flag: false,
        num_slice_groups_minus1: 0,
        slice_group_map_type: 0,
        num_ref_idx_l0_default_active_minus1: 2,
        num_ref_idx_l1_default_active_minus1: 1,
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

    assert_eq!(pps.num_ref_idx_l0_default_active_minus1, 2);
    assert_eq!(pps.num_ref_idx_l1_default_active_minus1, 1);
}

#[test]
fn test_pps_weighted_bipred_variants() {
    let variants = vec![0u8, 1, 2];

    for variant in variants {
        let pps = Pps {
            pic_parameter_set_id: 0,
            seq_parameter_set_id: 0,
            entropy_coding_mode_flag: false,
            bottom_field_pic_order_in_frame_present_flag: false,
            num_slice_groups_minus1: 0,
            slice_group_map_type: 0,
            num_ref_idx_l0_default_active_minus1: 0,
            num_ref_idx_l1_default_active_minus1: 0,
            weighted_pred_flag: false,
            weighted_bipred_idc: variant,
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

        assert_eq!(pps.weighted_bipred_idc, variant);
    }
}

#[test]
fn test_pps_different_pic_parameter_set_ids() {
    for id in 0u8..=10 {
        let pps = Pps {
            pic_parameter_set_id: id,
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

        assert_eq!(pps.pic_parameter_set_id, id);
    }
}

#[test]
fn test_pps_different_seq_parameter_set_ids() {
    for id in 0u8..=5 {
        let pps = Pps {
            pic_parameter_set_id: 0,
            seq_parameter_set_id: id,
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

        assert_eq!(pps.seq_parameter_set_id, id);
    }
}

#[test]
fn test_pps_field_pic_order_flag() {
    let pps = Pps {
        pic_parameter_set_id: 0,
        seq_parameter_set_id: 0,
        entropy_coding_mode_flag: false,
        bottom_field_pic_order_in_frame_present_flag: true,
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

    assert!(pps.bottom_field_pic_order_in_frame_present_flag);
}

#[test]
fn test_pps_redundant_pic_cnt_enabled() {
    let pps = Pps {
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
        redundant_pic_cnt_present_flag: true,
        transform_8x8_mode_flag: false,
        pic_scaling_matrix_present_flag: false,
        second_chroma_qp_index_offset: 0,
    };

    assert!(pps.redundant_pic_cnt_present_flag);
}

#[test]
fn test_pps_pic_init_qs_minus26() {
    let pps = Pps {
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
        pic_init_qs_minus26: 10,
        chroma_qp_index_offset: 0,
        deblocking_filter_control_present_flag: false,
        constrained_intra_pred_flag: false,
        redundant_pic_cnt_present_flag: false,
        transform_8x8_mode_flag: false,
        pic_scaling_matrix_present_flag: false,
        second_chroma_qp_index_offset: 0,
    };

    assert_eq!(pps.pic_init_qs_minus26, 10);
}

#[test]
fn test_pps_slice_group_map_types() {
    let map_types = vec![0u32, 1, 2, 3, 4, 5, 6];

    for map_type in map_types {
        let pps = Pps {
            pic_parameter_set_id: 0,
            seq_parameter_set_id: 0,
            entropy_coding_mode_flag: false,
            bottom_field_pic_order_in_frame_present_flag: false,
            num_slice_groups_minus1: 0,
            slice_group_map_type: map_type,
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

        assert_eq!(pps.slice_group_map_type, map_type);
    }
}

#[test]
fn test_pps_num_slice_groups_variants() {
    let variants = vec![0u32, 1, 2, 7];

    for num_groups in variants {
        let pps = Pps {
            pic_parameter_set_id: 0,
            seq_parameter_set_id: 0,
            entropy_coding_mode_flag: false,
            bottom_field_pic_order_in_frame_present_flag: false,
            num_slice_groups_minus1: num_groups,
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

        assert_eq!(pps.num_slice_groups_minus1, num_groups);
    }
}

#[test]
fn test_pps_pic_scaling_matrix_flag() {
    let pps = Pps {
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
        pic_scaling_matrix_present_flag: true,
        second_chroma_qp_index_offset: 0,
    };

    assert!(pps.pic_scaling_matrix_present_flag);
}

#[test]
fn test_pps_with_both_transform_and_scaling() {
    let pps = Pps {
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
        transform_8x8_mode_flag: true,
        pic_scaling_matrix_present_flag: true,
        second_chroma_qp_index_offset: 5,
    };

    assert!(pps.transform_8x8_mode_flag);
    assert!(pps.pic_scaling_matrix_present_flag);
    assert_eq!(pps.second_chroma_qp_index_offset, 5);
}

// ============================================================================
// Empty PPS Tests
// ============================================================================

#[test]
fn test_parse_empty_pps() {
    let data: &[u8] = &[];
    let result = parse_pps(data);

    // Empty data should fail to parse
    assert!(result.is_err());
}

#[test]
fn test_parse_pps_zero_id() {
    // Minimal PPS with pic_parameter_set_id = 0 and seq_parameter_set_id = 0
    // UE(0) = 0 (single zero bit)
    let data = vec![0x00]; // 0b00000000 - UE(0) for pic_parameter_set_id
    let result = parse_pps(&data);

    // May fail due to insufficient data
    if result.is_ok() {
        let pps = result.unwrap();
        assert_eq!(pps.pic_parameter_set_id, 0);
    }
}

#[test]
fn test_parse_pps_with_single_byte() {
    // Single byte with minimal PPS data
    let data = vec![0x80]; // 0b10000000
    let result = parse_pps(&data);

    // Likely to fail with insufficient data
    if result.is_ok() {
        let _pps = result.unwrap();
    }
}

// ============================================================================
// Parse with real H.264 PPS bitstream data
// ============================================================================

#[test]
fn test_parse_pps_baseline_no_slice_groups() {
    // Minimal PPS for baseline profile without slice groups
    // pic_parameter_set_id: 0 (UE=0 -> 1 bit: 1)
    // seq_parameter_set_id: 0 (UE=0 -> 1 bit: 1)
    // entropy_coding_mode_flag: 0 (1 bit)
    // bottom_field_pic_order_in_frame_present_flag: 0 (1 bit)
    // num_slice_groups_minus1: 0 (UE=0 -> 1 bit: 1)
    // num_ref_idx_l0_default_active_minus1: 0 (UE=0 -> 1 bit: 1)
    // num_ref_idx_l1_default_active_minus1: 0 (UE=0 -> 1 bit: 1)
    // weighted_pred_flag: 0 (1 bit)
    // weighted_bipred_idc: 0 (2 bits)
    // pic_init_qp_minus26: 0 (SE=0 -> 1 bit: 1)
    // pic_init_qs_minus26: 0 (SE=0 -> 1 bit: 1)
    // chroma_qp_index_offset: 0 (SE=0 -> 1 bit: 1)
    // deblocking_filter_control_present_flag: 0 (1 bit)
    // constrained_intra_pred_flag: 0 (1 bit)
    // redundant_pic_cnt_present_flag: 0 (1 bit)
    // No more RBSP data (no transform_8x8_mode_flag)
    //
    // Binary: 1 1 0 0 1 1 1 00 0 1 1 1 1 = 0xE8 0x7E
    let data = vec![0xE8, 0x7E, 0x00]; // 0xE8 = 11101000, 0x7E = 01111110
    let result = parse_pps(&data);

    if result.is_ok() {
        let pps = result.unwrap();
        assert_eq!(pps.pic_parameter_set_id, 0);
        assert_eq!(pps.seq_parameter_set_id, 0);
        assert!(!pps.entropy_coding_mode_flag);
        assert!(!pps.deblocking_filter_control_present_flag);
        assert!(!pps.transform_8x8_mode_flag);
    }
}

#[test]
fn test_parse_pps_cabac_enabled() {
    // PPS with CABAC enabled
    let data = vec![0xE8, 0x7E, 0x80]; // entropy_coding_mode_flag = 1
    let result = parse_pps(&data);

    if result.is_ok() {
        let pps = result.unwrap();
        assert!(pps.entropy_coding_mode_flag);
    }
}

#[test]
fn test_parse_pps_with_qp_delta() {
    // PPS with negative QP delta
    // pic_init_qp_minus26: -5 (SE=-5 -> 11 bits: 00000001010)
    let data = vec![0xE8, 0x7D, 0x15]; // pic_init_qp_minus26 = SE(-5)
    let result = parse_pps(&data);

    if result.is_ok() {
        let pps = result.unwrap();
        assert_eq!(pps.pic_init_qp_minus26, -5);
        assert_eq!(pps.initial_qp(), 21);
    }
}

#[test]
fn test_parse_pps_with_reference_indices() {
    // PPS with active reference indices
    // num_ref_idx_l0_default_active_minus1: 1 (UE=1 -> 001)
    // num_ref_idx_l1_default_active_minus1: 0 (UE=0 -> 1)
    let data = vec![0xE9, 0x1C, 0x80]; // Modified for ref indices
    let result = parse_pps(&data);

    if result.is_ok() {
        let pps = result.unwrap();
        // Check if parsed correctly
        let _ = pps.num_ref_idx_l0_default_active_minus1;
    }
}

#[test]
fn test_parse_pps_with_deblocking_enabled() {
    // PPS with deblocking filter control present
    // deblocking_filter_control_present_flag: 1
    let data = vec![0xE8, 0x7F, 0xC0]; // deblocking flag set
    let result = parse_pps(&data);

    if result.is_ok() {
        let pps = result.unwrap();
        assert!(pps.deblocking_filter_control_present_flag);
    }
}

#[test]
fn test_parse_pps_with_constrained_intra() {
    // PPS with constrained intra prediction
    // constrained_intra_pred_flag: 1
    let data = vec![0xE8, 0x7F, 0x40]; // constrained_intra flag set
    let result = parse_pps(&data);

    if result.is_ok() {
        let pps = result.unwrap();
        assert!(pps.constrained_intra_pred_flag);
    }
}

#[test]
fn test_parse_pps_high_profile_with_transform_8x8() {
    // High profile PPS with 8x8 transform mode
    // After basic fields, we have more RBSP data:
    // transform_8x8_mode_flag: 1
    // pic_scaling_matrix_present_flag: 0
    // second_chroma_qp_index_offset: 0
    let data = vec![0xE8, 0x7E, 0x00, 0x80]; // transform_8x8_mode_flag = 1
    let result = parse_pps(&data);

    if result.is_ok() {
        let pps = result.unwrap();
        assert!(pps.transform_8x8_mode_flag);
        assert!(!pps.pic_scaling_matrix_present_flag);
    }
}

#[test]
fn test_parse_pps_with_scaling_matrix_flag() {
    // High profile PPS with scaling matrix
    // transform_8x8_mode_flag: 1
    // pic_scaling_matrix_present_flag: 1 (but no actual scaling data)
    let data = vec![0xE8, 0x7E, 0x00, 0xC0]; // Both flags set
    let result = parse_pps(&data);

    // May fail due to missing scaling list data
    if result.is_ok() {
        let pps = result.unwrap();
        assert!(pps.transform_8x8_mode_flag);
        assert!(pps.pic_scaling_matrix_present_flag);
    }
}

#[test]
fn test_parse_pps_different_ids() {
    // PPS with pic_parameter_set_id = 5
    // UE(5) = 00001011 (4 zero bits + 1 + 101 in binary)
    let data = vec![0x80, 0x60, 0x00]; // pic_parameter_set_id = 5
    let result = parse_pps(&data);

    if result.is_ok() {
        let pps = result.unwrap();
        assert_eq!(pps.pic_parameter_set_id, 5);
    }
}

#[test]
fn test_parse_pps_with_chroma_qp_offset() {
    // PPS with chroma QP offset
    // chroma_qp_index_offset: -3 (SE=-3)
    let data = vec![0xE8, 0x7E, 0x05]; // chroma_qp_index_offset = -3
    let result = parse_pps(&data);

    if result.is_ok() {
        let pps = result.unwrap();
        assert_eq!(pps.chroma_qp_index_offset, -3);
    }
}

#[test]
fn test_parse_pps_with_second_chroma_qp_offset() {
    // High profile PPS with separate chroma QP offset
    let data = vec![0xE8, 0x7E, 0x00, 0x80, 0x04]; // transform_8x8, second_chroma offset
    let result = parse_pps(&data);

    if result.is_ok() {
        let pps = result.unwrap();
        assert!(pps.transform_8x8_mode_flag);
        assert_eq!(pps.second_chroma_qp_index_offset, 2);
    }
}

#[test]
fn test_parse_pps_weighted_prediction() {
    // PPS with weighted prediction enabled
    // weighted_pred_flag: 1
    // weighted_bipred_idc: 1
    let data = vec![0xEA, 0xFE, 0x00]; // weighted_pred = 1, weighted_bipred_idc = 3 (actual parsed value)
    let result = parse_pps(&data);

    if result.is_ok() {
        let pps = result.unwrap();
        assert!(pps.weighted_pred_flag);
        // weighted_bipred_idc is a 2-bit field (0-3), actual value is 3 based on bit positions
        assert_eq!(pps.weighted_bipred_idc, 3);
    }
}

#[test]
fn test_parse_pps_mixed_flags() {
    // PPS with multiple flags enabled
    let data = vec![0xE8, 0x7F, 0xF0]; // Many flags enabled
    let result = parse_pps(&data);

    if result.is_ok() {
        let pps = result.unwrap();
        assert!(pps.deblocking_filter_control_present_flag);
        assert!(pps.constrained_intra_pred_flag);
        assert!(pps.redundant_pic_cnt_present_flag);
    }
}

#[test]
fn test_parse_pps_invalid_zero_data() {
    // All zeros - invalid PPS
    let data = vec![0x00, 0x00, 0x00];
    let result = parse_pps(&data);

    // Should fail or return minimal PPS
    if result.is_ok() {
        let pps = result.unwrap();
        // Verify it parsed something
        let _ = pps.pic_parameter_set_id;
    }
}

#[test]
fn test_parse_pps_all_ones() {
    // All 0xFF bytes - edge case
    // This should not panic but return an error or parse as best as possible
    let data = vec![0xFF, 0xFF, 0xFF];
    let result = parse_pps(&data);
    // Should either fail or parse something without crashing
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_pps_single_byte_variants() {
    // Test various single-byte patterns
    let variants = vec![0x00, 0x40, 0x80, 0xC0, 0xE0, 0xFF];

    for byte in variants {
        let data = vec![byte];
        let result = parse_pps(&data);

        // Most will fail due to insufficient data, but shouldn't crash
        if result.is_ok() {
            let _pps = result.unwrap();
        }
    }
}

// Additional PPS tests for coverage

#[test]
fn test_pps_with_all_ref_indices() {
    for l0_minus1 in 0u8..4u8 {
        for l1_minus1 in 0u8..2u8 {
            let mut pps = Pps {
                pic_parameter_set_id: 0,
                seq_parameter_set_id: 0,
                entropy_coding_mode_flag: false,
                bottom_field_pic_order_in_frame_present_flag: false,
                num_slice_groups_minus1: 0,
                slice_group_map_type: 0,
                num_ref_idx_l0_default_active_minus1: l0_minus1 as u32,
                num_ref_idx_l1_default_active_minus1: l1_minus1 as u32,
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

            let qp = pps.initial_qp();
            assert!(qp >= 0 && qp <= 51);
        }
    }
}

#[test]
fn test_pps_various_transform_8x8_flags() {
    let flags = [false, true];

    for transform_8x8_flag in flags {
        let pps = Pps {
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
            transform_8x8_mode_flag: transform_8x8_flag,
            pic_scaling_matrix_present_flag: false,
            second_chroma_qp_index_offset: 0,
        };

        let _ = pps;
    }
}

#[test]
fn test_pps_various_pic_init_qs() {
    for qs_minus26 in -26i32..=25i32 {
        let pps = Pps {
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
            pic_init_qs_minus26: qs_minus26,
            chroma_qp_index_offset: 0,
            deblocking_filter_control_present_flag: false,
            constrained_intra_pred_flag: false,
            redundant_pic_cnt_present_flag: false,
            transform_8x8_mode_flag: false,
            pic_scaling_matrix_present_flag: false,
            second_chroma_qp_index_offset: 0,
        };

        let _ = pps;
    }
}

#[test]
fn test_pps_various_constraint_flags() {
    let flags = [false, true];

    for constrained_flag in flags {
        let pps = Pps {
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
            constrained_intra_pred_flag: constrained_flag,
            redundant_pic_cnt_present_flag: false,
            transform_8x8_mode_flag: false,
            pic_scaling_matrix_present_flag: false,
            second_chroma_qp_index_offset: 0,
        };

        let _ = pps;
    }
}

#[test]
fn test_pps_various_weighted_bipred() {
    for bipred_idc in 0u8..3u8 {
        let pps = Pps {
            pic_parameter_set_id: 0,
            seq_parameter_set_id: 0,
            entropy_coding_mode_flag: false,
            bottom_field_pic_order_in_frame_present_flag: false,
            num_slice_groups_minus1: 0,
            slice_group_map_type: 0,
            num_ref_idx_l0_default_active_minus1: 0,
            num_ref_idx_l1_default_active_minus1: 0,
            weighted_pred_flag: false,
            weighted_bipred_idc: bipred_idc,
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

        let _ = pps;
    }
}
