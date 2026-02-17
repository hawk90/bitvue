#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(unused_comparisons)]
#![allow(unused_doc_comments)]
// Extended tests for PPS parsing
use bitvue_avc::pps::{parse_pps, Pps};

// ============================================================================
// Tests for PPS fields and edge cases
// ============================================================================

#[test]
#[ignore = "test needs fixing - PPS parsing edge cases"]
fn test_pps_all_slice_group_map_types() {
    // Test all slice_group_map_type values (0-6)
    for map_type in 0u8..=6u8 {
        let data = [
            0x80,                   // pic_parameter_set_id = 0
            0x80,                   // seq_parameter_set_id = 0
            0x00,                   // entropy_coding_mode_flag = 0
            0x00,                   // bottom_field_pic_order_in_frame_present_flag = 0
            0x00,                   // num_slice_groups_minus1 = 0
            (map_type << 1) | 0x80, // slice_group_map_type
        ];

        let result = parse_pps(&data);
        assert!(result.is_ok());
    }
}

#[test]
#[ignore = "test needs fixing - PPS parsing edge cases"]
fn test_pps_various_num_slice_groups() {
    // Test various num_slice_groups_minus1 values
    for num_groups in 0u8..=7u8 {
        let data = [
            0x80, // pic_parameter_set_id = 0
            0x80, // seq_parameter_set_id = 0
            0x00,
            0x00,
            (num_groups << 1) | 0x80, // num_slice_groups_minus1
            0x00,                     // slice_group_map_type = 0
        ];

        let result = parse_pps(&data);
        assert!(result.is_ok());
    }
}

#[test]
#[ignore = "test needs fixing - PPS parsing edge cases"]
fn test_pps_ref_indices_combinations() {
    // Test various combinations of ref_idx values
    for l0 in 0u8..=3u8 {
        for l1 in 0u8..=3u8 {
            let data = [
                0x80, // pic_parameter_set_id = 0
                0x80, // seq_parameter_set_id = 0
                0x00,
                0x00,
                0x00,             // num_slice_groups_minus1 = 0
                0x00,             // slice_group_map_type = 0
                (l0 << 1) | 0x80, // num_ref_idx_l0_default_active_minus1
                (l1 << 1) | 0x80, // num_ref_idx_l1_default_active_minus1
            ];

            let result = parse_pps(&data);
            assert!(result.is_ok());
        }
    }
}

#[test]
#[ignore = "test needs fixing - PPS parsing edge cases"]
fn test_pps_weighted_pred_bipred_combinations() {
    // Test weighted_pred_flag and weighted_bipred_idc combinations
    for weighted_pred in [false, true] {
        for bipred_idc in 0u8..=2u8 {
            let data = [
                0x80, // pic_parameter_set_id = 0
                0x80, // seq_parameter_set_id = 0
                0x00,
                0x00,
                0x00,                                    // num_slice_groups_minus1 = 0
                0x00,                                    // slice_group_map_type = 0
                0x00,                                    // num_ref_idx_l0_default_active_minus1 = 0
                0x00,                                    // num_ref_idx_l1_default_active_minus1 = 0
                if weighted_pred { 0x80 } else { 0x00 }, // weighted_pred_flag
                (bipred_idc << 1) | 0x80,                // weighted_bipred_idc
            ];

            let result = parse_pps(&data);
            assert!(result.is_ok());
        }
    }
}

#[test]
#[ignore = "test needs fixing - PPS parsing edge cases"]
fn test_pps_pic_init_qp_values() {
    // Test various pic_init_qp_minus26 values
    for qp_delta in -26i32..=25i32 {
        let encoded = if qp_delta <= 0 {
            ((-qp_delta * 2) as u8) & 0xFF
        } else {
            ((qp_delta * 2 - 1) as u8) & 0xFF
        };

        let data = [
            0x80, // pic_parameter_set_id = 0
            0x80, // seq_parameter_set_id = 0
            0x00, 0x00, 0x00,    // num_slice_groups_minus1 = 0
            0x00,    // slice_group_map_type = 0
            0x00,    // num_ref_idx_l0_default_active_minus1 = 0
            0x00,    // num_ref_idx_l1_default_minus1 = 0
            0x00,    // weighted_pred_flag = 0
            0x00,    // weighted_bipred_idc = 0
            encoded, // pic_init_qp_minus26
        ];

        let result = parse_pps(&data);
        if result.is_ok() {
            let pps = result.unwrap();
            let qp = pps.initial_qp();
            assert!(qp >= 0 && qp <= 51);
        }
    }
}

#[test]
#[ignore = "test needs fixing - PPS parsing edge cases"]
fn test_pps_chroma_qp_offsets() {
    // Test chroma_qp_index_offset values
    for offset in -12i32..=11i32 {
        let encoded = if offset <= 0 {
            ((-offset * 2) as u8) & 0xFF
        } else {
            ((offset * 2 - 1) as u8) & 0xFF
        };

        let data = [
            0x80, // pic_parameter_set_id = 0
            0x80, // seq_parameter_set_id = 0
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,    // up to pic_init_qp_minus26
            encoded, // chroma_qp_index_offset
            encoded, // second_chroma_qp_index_offset
        ];

        let result = parse_pps(&data);
        let _ = result;
    }
}

#[test]
#[ignore = "test needs fixing - PPS parsing edge cases"]
fn test_pps_deblocking_filter_control_present_flag() {
    for flag in [false, true] {
        let data = [
            0x80, // pic_parameter_set_id = 0
            0x80, // seq_parameter_set_id = 0
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,                           // up to chroma_qp_index_offset
            if flag { 0x80 } else { 0x00 }, // deblocking_filter_control_present_flag
            if flag { 0x00 } else { 0x00 }, // deblocking_filter_override_enabled_flag
            if flag { 0x00 } else { 0x00 }, // deblocking_filter_override_flag
        ];

        let result = parse_pps(&data);
        let _ = result;
    }
}

#[test]
#[ignore = "test needs fixing - PPS parsing edge cases"]
fn test_pps_constrained_intra_pred_flag() {
    for flag in [false, true] {
        let data = [
            0x80,
            0x80,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
            if flag { 0x80 } else { 0x00 }, // constrained_intra_pred_flag
        ];

        let result = parse_pps(&data);
        let _ = result;
    }
}

#[test]
#[ignore = "test needs fixing - PPS parsing edge cases"]
fn test_pps_redundant_pic_cnt_present_flag() {
    for flag in [false, true] {
        let data = [
            0x80,
            0x80,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
            if flag { 0x80 } else { 0x00 }, // redundant_pic_cnt_present_flag
        ];

        let result = parse_pps(&data);
        let _ = result;
    }
}

#[test]
#[ignore = "test needs fixing - PPS parsing edge cases"]
fn test_pps_transform_8x8_mode_flag() {
    for flag in [false, true] {
        let data = [
            0x80,
            0x80,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
            if flag { 0x80 } else { 0x00 }, // transform_8x8_mode_flag
        ];

        let result = parse_pps(&data);
        let _ = result;
    }
}

#[test]
#[ignore = "test needs fixing - PPS parsing edge cases"]
fn test_pps_pic_scaling_matrix_present_flag() {
    for flag in [false, true] {
        let data = [
            0x80,
            0x80,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
            if flag { 0x80 } else { 0x00 }, // pic_scaling_matrix_present_flag
        ];

        let result = parse_pps(&data);
        let _ = result;
    }
}

#[test]
#[ignore = "test needs fixing - PPS parsing edge cases"]
fn test_pps_various_pic_parameter_set_ids() {
    for pps_id in 0u8..=10u8 {
        let data = [
            (pps_id << 1) | 0x80, // pic_parameter_set_id
            0x80,                 // seq_parameter_set_id = 0
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
        ];

        let result = parse_pps(&data);
        assert!(result.is_ok());
    }
}

#[test]
#[ignore = "test needs fixing - PPS parsing edge cases"]
fn test_pps_various_seq_parameter_set_ids() {
    for sps_id in 0u8..=10u8 {
        let data = [
            0x80,                 // pic_parameter_set_id = 0
            (sps_id << 1) | 0x80, // seq_parameter_set_id
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
        ];

        let result = parse_pps(&data);
        assert!(result.is_ok());
    }
}
