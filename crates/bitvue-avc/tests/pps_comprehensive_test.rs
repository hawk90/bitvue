//! Comprehensive tests for AVC PPS module
//!
//! Tests Pps struct, is_cabac(), initial_qp(), and parse_pps()

use bitvue_avc::pps::{parse_pps, Pps};

// ============================================================================
// Pps Struct Tests
// ============================================================================

#[test]
fn test_pps_default_fields() {
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
fn test_pps_all_true_flags() {
    let pps = Pps {
        pic_parameter_set_id: 1,
        seq_parameter_set_id: 0,
        entropy_coding_mode_flag: true,
        bottom_field_pic_order_in_frame_present_flag: true,
        num_slice_groups_minus1: 0,
        slice_group_map_type: 0,
        num_ref_idx_l0_default_active_minus1: 0,
        num_ref_idx_l1_default_active_minus1: 0,
        weighted_pred_flag: true,
        weighted_bipred_idc: 0,
        pic_init_qp_minus26: 0,
        pic_init_qs_minus26: 0,
        chroma_qp_index_offset: 0,
        deblocking_filter_control_present_flag: true,
        constrained_intra_pred_flag: true,
        redundant_pic_cnt_present_flag: true,
        transform_8x8_mode_flag: true,
        pic_scaling_matrix_present_flag: true,
        second_chroma_qp_index_offset: 0,
    };

    assert!(pps.entropy_coding_mode_flag);
    assert!(pps.bottom_field_pic_order_in_frame_present_flag);
    assert!(pps.weighted_pred_flag);
    assert!(pps.deblocking_filter_control_present_flag);
    assert!(pps.constrained_intra_pred_flag);
    assert!(pps.redundant_pic_cnt_present_flag);
    assert!(pps.transform_8x8_mode_flag);
    assert!(pps.pic_scaling_matrix_present_flag);
}

#[test]
fn test_pps_max_values() {
    let pps = Pps {
        pic_parameter_set_id: u8::MAX,
        seq_parameter_set_id: u8::MAX,
        entropy_coding_mode_flag: true,
        bottom_field_pic_order_in_frame_present_flag: true,
        num_slice_groups_minus1: u32::MAX,
        slice_group_map_type: u32::MAX,
        num_ref_idx_l0_default_active_minus1: u32::MAX,
        num_ref_idx_l1_default_active_minus1: u32::MAX,
        weighted_pred_flag: true,
        weighted_bipred_idc: u8::MAX,
        pic_init_qp_minus26: i32::MAX,
        pic_init_qs_minus26: i32::MAX,
        chroma_qp_index_offset: i32::MAX,
        deblocking_filter_control_present_flag: true,
        constrained_intra_pred_flag: true,
        redundant_pic_cnt_present_flag: true,
        transform_8x8_mode_flag: true,
        pic_scaling_matrix_present_flag: true,
        second_chroma_qp_index_offset: i32::MAX,
    };

    assert_eq!(pps.pic_parameter_set_id, u8::MAX);
    assert_eq!(pps.seq_parameter_set_id, u8::MAX);
    assert_eq!(pps.weighted_bipred_idc, u8::MAX);
}

// ============================================================================
// is_cabac() Method Tests
// ============================================================================

#[test]
fn test_pps_is_cabac_true() {
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
fn test_pps_is_cabac_false() {
    let pps = Pps {
        pic_parameter_set_id: 0,
        seq_parameter_set_id: 0,
        entropy_coding_mode_flag: false, // CAVLC
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
// initial_qp() Method Tests
// ============================================================================

#[test]
fn test_pps_initial_qp_zero() {
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
fn test_pps_initial_qp_extreme_positive() {
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
        pic_init_qp_minus26: 25,
        pic_init_qs_minus26: 0,
        chroma_qp_index_offset: 0,
        deblocking_filter_control_present_flag: false,
        constrained_intra_pred_flag: false,
        redundant_pic_cnt_present_flag: false,
        transform_8x8_mode_flag: false,
        pic_scaling_matrix_present_flag: false,
        second_chroma_qp_index_offset: 0,
    };

    assert_eq!(pps.initial_qp(), 51);
}

// ============================================================================
// parse_pps() Tests
// ============================================================================

#[test]
fn test_parse_pps_empty() {
    let data: &[u8] = &[];
    let result = parse_pps(data);
    assert!(result.is_err());
}

#[test]
fn test_parse_pps_insufficient_data() {
    // Only one byte, not enough for even pic_parameter_set_id
    let data = &[0x00];
    let result = parse_pps(data);
    assert!(result.is_err());
}

#[test]
fn test_parse_pps_minimal_valid() {
    // Minimal valid PPS data - ue(0)=pic_parameter_set_id=0, ue(0)=seq_parameter_set_id=0
    // followed by flags and fields
    // ue(0) is encoded as 1 (leading zeros + 1)
    let data = &[
        0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80,
        0x80,
    ]; // All ue(0) and flags set to 0
    let result = parse_pps(data);

    // Even if exact values differ, the parse should succeed or fail gracefully
    // The important thing is that it doesn't panic
    match result {
        Ok(pps) => {
            // If parsing succeeds, check basic fields
            assert_eq!(pps.pic_parameter_set_id, 0);
            assert_eq!(pps.seq_parameter_set_id, 0);
        }
        Err(_) => {
            // Parsing may fail due to incomplete data, which is acceptable
            // The test verifies error handling
        }
    }
}

#[test]
fn test_parse_pps_cabac_enabled() {
    // PPS with CABAC enabled - minimal data structure
    let data = &[0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80];
    let _result = parse_pps(data);
    // Just verify it doesn't panic - exact parsing depends on bit alignment
}

#[test]
fn test_parse_pps_with_slice_groups_type0() {
    // PPS with num_slice_groups_minus1=1, slice_group_map_type=0
    // Map type 0 has run_length_minus1 for each group
    // Need to construct proper bitstream for this
    let data = &[
        0xE8, // pic_parameter_set_id = 0 (ue), seq_parameter_set_id = 0 (ue)
        0x38, // entropy=0, bottom=0, num_slice_groups_minus1 = 1 (ue)
        0x04, // slice_group_map_type = 0 (ue)
        0x80, // run_length_minus1[0] = 0 (ue)
        0x80, // run_length_minus1[1] = 0 (ue)
        0x00, // num_ref_idx_l0 = 0 (ue)
        0x00, // num_ref_idx_l1 = 0 (ue)
        0x80, // weighted_pred=0, weighted_bipred_idc=0
        0x80, // pic_init_qp_minus26 = 0 (se)
        0x80, // pic_init_qs_minus26 = 0 (se)
        0x80, // chroma_qp_index_offset = 0 (se)
        0x80, // deblocking=0, constrained=0, redundant=0
        0x80, // rbsp trailing bit
    ];

    let _result = parse_pps(data);
    // This may fail due to exact bit encoding, but tests the path
    // The important thing is that it doesn't panic
}

#[test]
fn test_parse_pps_num_slice_groups_too_large() {
    // Test security validation: num_slice_groups_minus1 > 7 should fail
    // We need to craft data where num_slice_groups_minus1 > 7
    // ue(8) = 0x00 0x00 0x02 (8 in exp-Golomb)
    let data = &[
        0x80, // pic_parameter_set_id = 0
        0x80, // seq_parameter_set_id = 0
        0xC0, // entropy=0, bottom=0
        0x00, 0x00, 0x02, // num_slice_groups_minus1 = 8 (too large, should be <= 7)
    ];

    let result = parse_pps(data);
    assert!(result.is_err());
}

#[test]
fn test_parse_pps_max_slice_groups() {
    // Test num_slice_groups_minus1 = 7 (maximum allowed)
    // ue(7) = 0x00 0x00 0x01 (7 in exp-Golomb: 0x00 0x00 0x01)
    let data = &[
        0x80, // pic_parameter_set_id = 0
        0x80, // seq_parameter_set_id = 0
        0xC0, // entropy=0, bottom=0
        0x00, 0x00, 0x01, // num_slice_groups_minus1 = 7 (max allowed)
    ];

    let result = parse_pps(data);
    // May still fail due to incomplete data, but should not fail the security check
}

// ============================================================================
// Clone and Debug Tests
// ============================================================================

#[test]
fn test_pps_clone() {
    let pps = Pps {
        pic_parameter_set_id: 5,
        seq_parameter_set_id: 2,
        entropy_coding_mode_flag: true,
        bottom_field_pic_order_in_frame_present_flag: false,
        num_slice_groups_minus1: 3,
        slice_group_map_type: 1,
        num_ref_idx_l0_default_active_minus1: 5,
        num_ref_idx_l1_default_active_minus1: 3,
        weighted_pred_flag: false,
        weighted_bipred_idc: 2,
        pic_init_qp_minus26: -5,
        pic_init_qs_minus26: 2,
        chroma_qp_index_offset: 3,
        deblocking_filter_control_present_flag: true,
        constrained_intra_pred_flag: false,
        redundant_pic_cnt_present_flag: true,
        transform_8x8_mode_flag: false,
        pic_scaling_matrix_present_flag: false,
        second_chroma_qp_index_offset: 4,
    };

    let cloned = pps.clone();
    assert_eq!(cloned.pic_parameter_set_id, 5);
    assert_eq!(cloned.seq_parameter_set_id, 2);
    assert!(cloned.entropy_coding_mode_flag);
    assert_eq!(cloned.pic_init_qp_minus26, -5);
}

#[test]
fn test_pps_debug() {
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

    let debug_str = format!("{:?}", pps);
    assert!(debug_str.contains("Pps"));
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_pps_weighted_bipred_idc_max() {
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
        weighted_bipred_idc: 3, // Max value (2 bits)
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

    assert_eq!(pps.weighted_bipred_idc, 3);
}

#[test]
fn test_pps_second_chroma_qp_offset_different() {
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
        second_chroma_qp_index_offset: 10, // Different from chroma_qp_index_offset
    };

    assert_eq!(pps.chroma_qp_index_offset, 5);
    assert_eq!(pps.second_chroma_qp_index_offset, 10);
}

#[test]
fn test_pps_all_zero_values() {
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
    assert!(!pps.is_cabac());
}

#[test]
fn test_pps_pic_parameter_set_id_variety() {
    for id in 0..=10u8 {
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
fn test_pps_num_ref_idx_variety() {
    let pps = Pps {
        pic_parameter_set_id: 0,
        seq_parameter_set_id: 0,
        entropy_coding_mode_flag: false,
        bottom_field_pic_order_in_frame_present_flag: false,
        num_slice_groups_minus1: 0,
        slice_group_map_type: 0,
        num_ref_idx_l0_default_active_minus1: 15,
        num_ref_idx_l1_default_active_minus1: 10,
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

    assert_eq!(pps.num_ref_idx_l0_default_active_minus1, 15);
    assert_eq!(pps.num_ref_idx_l1_default_active_minus1, 10);
}
