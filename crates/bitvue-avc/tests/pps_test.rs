//! H.264/AVC PPS (Picture Parameter Set) Tests
//!
//! Tests for PPS parsing to improve coverage.

use bitvue_avc::pps;

#[test]
fn test_pps_initial_qp_calculation() {
    let pps = pps::Pps {
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
fn test_pps_initial_qp_with_offset() {
    let pps = pps::Pps {
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

    assert_eq!(pps.initial_qp(), 31); // 26 + 5
}

#[test]
fn test_pps_initial_qp_with_negative_offset() {
    let pps = pps::Pps {
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
        pic_init_qp_minus26: -5,
        pic_init_qs_minus26: 0,
        chroma_qp_index_offset: 0,
        deblocking_filter_control_present_flag: false,
        constrained_intra_pred_flag: false,
        redundant_pic_cnt_present_flag: false,
        transform_8x8_mode_flag: false,
        pic_scaling_matrix_present_flag: false,
        second_chroma_qp_index_offset: 0,
    };

    assert_eq!(pps.initial_qp(), 21); // 26 + (-5)
}

#[test]
fn test_pps_is_cabac() {
    let mut pps = default_pps();
    pps.entropy_coding_mode_flag = true;
    assert!(pps.is_cabac());

    let mut pps = default_pps();
    pps.entropy_coding_mode_flag = false;
    assert!(!pps.is_cabac());
}

fn default_pps() -> pps::Pps {
    pps::Pps {
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
fn test_pps_fields_coverage() {
    // Test that all PPS fields can be set and retrieved
    let pps = pps::Pps {
        pic_parameter_set_id: 5,
        seq_parameter_set_id: 2,
        entropy_coding_mode_flag: true,
        bottom_field_pic_order_in_frame_present_flag: true,
        num_slice_groups_minus1: 1,
        slice_group_map_type: 2,
        num_ref_idx_l0_default_active_minus1: 3,
        num_ref_idx_l1_default_active_minus1: 2,
        weighted_pred_flag: true,
        weighted_bipred_idc: 2,
        pic_init_qp_minus26: -3,
        pic_init_qs_minus26: 2,
        chroma_qp_index_offset: 4,
        deblocking_filter_control_present_flag: true,
        constrained_intra_pred_flag: true,
        redundant_pic_cnt_present_flag: true,
        transform_8x8_mode_flag: true,
        pic_scaling_matrix_present_flag: true,
        second_chroma_qp_index_offset: 6,
    };

    assert_eq!(pps.pic_parameter_set_id, 5);
    assert_eq!(pps.seq_parameter_set_id, 2);
    assert!(pps.entropy_coding_mode_flag);
    assert!(pps.bottom_field_pic_order_in_frame_present_flag);
    assert_eq!(pps.num_slice_groups_minus1, 1);
    assert_eq!(pps.slice_group_map_type, 2);
    assert_eq!(pps.num_ref_idx_l0_default_active_minus1, 3);
    assert_eq!(pps.num_ref_idx_l1_default_active_minus1, 2);
    assert!(pps.weighted_pred_flag);
    assert_eq!(pps.weighted_bipred_idc, 2);
    assert_eq!(pps.pic_init_qp_minus26, -3);
    assert_eq!(pps.pic_init_qs_minus26, 2);
    assert_eq!(pps.chroma_qp_index_offset, 4);
    assert!(pps.deblocking_filter_control_present_flag);
    assert!(pps.constrained_intra_pred_flag);
    assert!(pps.redundant_pic_cnt_present_flag);
    assert!(pps.transform_8x8_mode_flag);
    assert!(pps.pic_scaling_matrix_present_flag);
    assert_eq!(pps.second_chroma_qp_index_offset, 6);
}

#[test]
fn test_pps_different_chroma_qp_offsets() {
    // Test different chroma QP offsets
    let pps1 = pps::Pps {
        chroma_qp_index_offset: -5,
        second_chroma_qp_index_offset: -3,
        ..default_pps()
    };

    assert_eq!(pps1.chroma_qp_index_offset, -5);
    assert_eq!(pps1.second_chroma_qp_index_offset, -3);

    let pps2 = pps::Pps {
        chroma_qp_index_offset: 10,
        second_chroma_qp_index_offset: 12,
        ..default_pps()
    };

    assert_eq!(pps2.chroma_qp_index_offset, 10);
    assert_eq!(pps2.second_chroma_qp_index_offset, 12);
}

#[test]
fn test_pps_weighted_bipred_idc_values() {
    // Test all valid weighted_bipred_idc values (0-2)
    for idc in 0..=2u8 {
        let pps = pps::Pps {
            weighted_bipred_idc: idc,
            ..default_pps()
        };
        assert_eq!(pps.weighted_bipred_idc, idc);
    }
}

#[test]
fn test_pps_slice_group_configurations() {
    // Test various slice group configurations
    let pps1 = pps::Pps {
        num_slice_groups_minus1: 0, // No slice groups
        slice_group_map_type: 0,
        ..default_pps()
    };
    assert_eq!(pps1.num_slice_groups_minus1, 0);

    let pps2 = pps::Pps {
        num_slice_groups_minus1: 7, // Maximum (8 groups)
        slice_group_map_type: 6,
        ..default_pps()
    };
    assert_eq!(pps2.num_slice_groups_minus1, 7);
    assert_eq!(pps2.slice_group_map_type, 6);
}

#[test]
fn test_pps_ref_idx_configurations() {
    // Test various reference index configurations
    let pps = pps::Pps {
        num_ref_idx_l0_default_active_minus1: 15, // Max ref frames L0
        num_ref_idx_l1_default_active_minus1: 15, // Max ref frames L1
        ..default_pps()
    };

    assert_eq!(pps.num_ref_idx_l0_default_active_minus1, 15);
    assert_eq!(pps.num_ref_idx_l1_default_active_minus1, 15);
}

#[test]
fn test_pps_id_ranges() {
    // Test various PPS and SPS ID ranges
    for pps_id in 0..=255u8 {
        let pps = pps::Pps {
            pic_parameter_set_id: pps_id,
            ..default_pps()
        };
        assert_eq!(pps.pic_parameter_set_id, pps_id);
    }

    for sps_id in 0..=31u8 {
        let pps = pps::Pps {
            seq_parameter_set_id: sps_id,
            ..default_pps()
        };
        assert_eq!(pps.seq_parameter_set_id, sps_id);
    }
}
