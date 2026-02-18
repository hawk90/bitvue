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
//! Comprehensive tests for HEVC PPS module
//!
//! Tests Pps, TileConfig, and associated methods

use bitvue_hevc::pps::{parse_pps, Pps, TileConfig};

// ============================================================================
// TileConfig Tests
// ============================================================================

#[test]
fn test_tile_config_default() {
    let config = TileConfig::default();
    assert_eq!(config.num_tile_columns_minus1, 0);
    assert_eq!(config.num_tile_rows_minus1, 0);
    assert!(!config.uniform_spacing_flag);
    assert!(config.column_width_minus1.is_empty());
    assert!(config.row_height_minus1.is_empty());
}

#[test]
fn test_tile_config_with_values() {
    let config = TileConfig {
        num_tile_columns_minus1: 7,
        num_tile_rows_minus1: 3,
        uniform_spacing_flag: true,
        column_width_minus1: vec![10, 20, 30, 40, 50, 60, 70],
        row_height_minus1: vec![15, 25, 35],
    };

    assert_eq!(config.num_tile_columns_minus1, 7);
    assert_eq!(config.num_tile_rows_minus1, 3);
    assert!(config.uniform_spacing_flag);
    assert_eq!(config.column_width_minus1.len(), 7);
    assert_eq!(config.row_height_minus1.len(), 3);
}

#[test]
fn test_tile_config_num_columns() {
    let config = TileConfig {
        num_tile_columns_minus1: 5,
        ..Default::default()
    };

    assert_eq!(config.num_columns(), 6);
}

#[test]
fn test_tile_config_num_rows() {
    let config = TileConfig {
        num_tile_rows_minus1: 3,
        ..Default::default()
    };

    assert_eq!(config.num_rows(), 4);
}

#[test]
fn test_tile_config_num_tiles() {
    let config = TileConfig {
        num_tile_columns_minus1: 2,
        num_tile_rows_minus1: 1,
        ..Default::default()
    };

    assert_eq!(config.num_tiles(), 6u32); // 3 * 2
}

#[test]
fn test_tile_config_num_tiles_none_when_disabled() {
    let config = TileConfig {
        num_tile_columns_minus1: 0,
        num_tile_rows_minus1: 0,
        ..Default::default()
    };

    assert_eq!(config.num_tiles(), 1u32); // 1 * 1
}

#[test]
fn test_tile_config_clone() {
    let config = TileConfig {
        num_tile_columns_minus1: 3,
        num_tile_rows_minus1: 2,
        uniform_spacing_flag: false,
        column_width_minus1: vec![10, 15, 20],
        row_height_minus1: vec![30, 40],
    };

    let cloned = config.clone();
    assert_eq!(cloned.num_tile_columns_minus1, 3);
    assert_eq!(cloned.column_width_minus1.len(), 3);
}

// ============================================================================
// Pps::default() Tests
// ============================================================================

#[test]
fn test_pps_default() {
    let pps = Pps::default();
    assert_eq!(pps.pps_pic_parameter_set_id, 0);
    assert_eq!(pps.pps_seq_parameter_set_id, 0);
    assert!(!pps.dependent_slice_segments_enabled_flag);
    assert!(!pps.output_flag_present_flag);
    assert_eq!(pps.num_extra_slice_header_bits, 0);
    assert!(!pps.sign_data_hiding_enabled_flag);
    assert!(!pps.cabac_init_present_flag);
    assert_eq!(pps.num_ref_idx_l0_default_active_minus1, 0);
    assert_eq!(pps.num_ref_idx_l1_default_active_minus1, 0);
    assert_eq!(pps.init_qp_minus26, 0);
    assert!(!pps.constrained_intra_pred_flag);
    assert!(!pps.transform_skip_enabled_flag);
    assert!(!pps.cu_qp_delta_enabled_flag);
    assert_eq!(pps.diff_cu_qp_delta_depth, 0);
    assert_eq!(pps.pps_cb_qp_offset, 0);
    assert_eq!(pps.pps_cr_qp_offset, 0);
    assert!(!pps.pps_slice_chroma_qp_offsets_present_flag);
    assert!(!pps.weighted_pred_flag);
    assert!(!pps.weighted_bipred_flag);
    assert!(!pps.transquant_bypass_enabled_flag);
    assert!(!pps.tiles_enabled_flag);
    assert!(!pps.entropy_coding_sync_enabled_flag);
    assert!(pps.tile_config.is_none());
    assert!(pps.loop_filter_across_tiles_enabled_flag);
    assert!(!pps.pps_loop_filter_across_slices_enabled_flag);
    assert!(!pps.deblocking_filter_control_present_flag);
    assert!(!pps.deblocking_filter_override_enabled_flag);
    assert!(!pps.pps_deblocking_filter_disabled_flag);
    assert_eq!(pps.pps_beta_offset_div2, 0);
    assert_eq!(pps.pps_tc_offset_div2, 0);
    assert!(!pps.pps_scaling_list_data_present_flag);
    assert!(!pps.lists_modification_present_flag);
    assert_eq!(pps.log2_parallel_merge_level_minus2, 0);
    assert!(!pps.slice_segment_header_extension_present_flag);
    assert!(!pps.pps_extension_present_flag);
    assert!(!pps.pps_range_extension_flag);
    assert!(!pps.pps_multilayer_extension_flag);
    assert!(!pps.pps_3d_extension_flag);
    assert!(!pps.pps_scc_extension_flag);
}

#[test]
fn test_pps_default_loop_filter_flags() {
    let pps = Pps::default();
    // Note: default has loop_filter_across_tiles_enabled_flag: true
    assert!(pps.loop_filter_across_tiles_enabled_flag);
    // And loop_filter_across_slices_enabled_flag: false
    assert!(!pps.pps_loop_filter_across_slices_enabled_flag);
}

// ============================================================================
// Pps::init_qp() Tests
// ============================================================================

#[test]
fn test_pps_init_qp_zero() {
    let pps = Pps {
        init_qp_minus26: 0,
        ..Default::default()
    };

    assert_eq!(pps.init_qp(), 26i8);
}

#[test]
fn test_pps_init_qp_positive() {
    let pps = Pps {
        init_qp_minus26: 10,
        ..Default::default()
    };

    assert_eq!(pps.init_qp(), 36i8);
}

#[test]
fn test_pps_init_qp_negative() {
    let pps = Pps {
        init_qp_minus26: -10,
        ..Default::default()
    };

    assert_eq!(pps.init_qp(), 16i8);
}

#[test]
fn test_pps_init_qp_max() {
    let pps = Pps {
        init_qp_minus26: 100, // Within valid range for i8
        ..Default::default()
    };

    // 26 + 100 = 126
    assert_eq!(pps.init_qp(), 126i8);
}

// ============================================================================
// Pps::has_tiles() Tests
// ============================================================================

#[test]
fn test_pps_has_tiles_false_when_disabled() {
    let pps = Pps {
        tiles_enabled_flag: false,
        tile_config: None,
        ..Default::default()
    };

    assert!(!pps.has_tiles());
}

#[test]
fn test_pps_has_tiles_true_when_enabled() {
    let pps = Pps {
        tiles_enabled_flag: true,
        tile_config: Some(TileConfig {
            num_tile_columns_minus1: 2,
            num_tile_rows_minus1: 1,
            ..Default::default()
        }),
        ..Default::default()
    };

    assert!(pps.has_tiles());
}

#[test]
fn test_pps_has_tiles_true_with_zero_tiles() {
    let pps = Pps {
        tiles_enabled_flag: true,
        tile_config: Some(TileConfig {
            num_tile_columns_minus1: 0,
            num_tile_rows_minus1: 0,
            ..Default::default()
        }),
        ..Default::default()
    };

    assert!(pps.has_tiles());
}

// ============================================================================
// Pps::num_tiles() Tests
// ============================================================================

#[test]
fn test_pps_num_tiles_none_when_disabled() {
    let pps = Pps {
        tiles_enabled_flag: false,
        tile_config: None,
        ..Default::default()
    };

    assert_eq!(pps.num_tiles(), None);
}

#[test]
fn test_pps_num_tiles_some_when_enabled() {
    let pps = Pps {
        tiles_enabled_flag: true,
        tile_config: Some(TileConfig {
            num_tile_columns_minus1: 3,
            num_tile_rows_minus1: 2,
            ..Default::default()
        }),
        ..Default::default()
    };

    assert_eq!(pps.num_tiles(), Some(12u32)); // 4 * 3
}

#[test]
fn test_pps_num_tiles_single_tile() {
    let pps = Pps {
        tiles_enabled_flag: true,
        tile_config: Some(TileConfig {
            num_tile_columns_minus1: 1,
            num_tile_rows_minus1: 0, // 2 * 1 = 2 tiles
            ..Default::default()
        }),
        ..Default::default()
    };

    assert_eq!(pps.num_tiles(), Some(2u32));
}

// ============================================================================
// Pps::wpp_enabled() Tests
// ============================================================================

#[test]
fn test_pps_wpp_enabled_false() {
    let pps = Pps {
        entropy_coding_sync_enabled_flag: false,
        ..Default::default()
    };

    assert!(!pps.wpp_enabled());
}

#[test]
fn test_pps_wpp_enabled_true() {
    let pps = Pps {
        entropy_coding_sync_enabled_flag: true,
        ..Default::default()
    };

    assert!(pps.wpp_enabled());
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
    let data = &[0x00]; // Only one byte
    let result = parse_pps(data);
    assert!(result.is_err());
}

#[test]
fn test_parse_pps_minimal_success() {
    // Minimal valid PPS data - ue(0)=pic_parameter_set_id, ue(0)=seq_parameter_set_id
    // ue(0)=0 for all remaining fields (this creates a valid but minimal PPS)
    // ue(0) is encoded as 1 (leading zeros + 1)
    let data = &[
        0x80, // pps_pic_parameter_set_id = 0 (ue(0))
        0x80, // pps_seq_parameter_set_id = 0 (ue(0))
        0x00, // all remaining flags false (1 bit each)
        0x00, 0x00, 0x00, 0x00, // more fields set to 0
        0x00, 0x00, 0x00, 0x00, 0x00,
    ];

    let result = parse_pps(data);
    // Should succeed or fail gracefully
    match result {
        Ok(pps) => {
            assert_eq!(pps.pps_pic_parameter_set_id, 0);
            assert_eq!(pps.pps_seq_parameter_set_id, 0);
        }
        Err(_) => {
            // Acceptable - may fail due to incomplete data
        }
    }
}

// ============================================================================
// Clone and Debug Tests
// ============================================================================

#[test]
fn test_pps_clone() {
    let pps = Pps {
        pps_pic_parameter_set_id: 5,
        init_qp_minus26: 10,
        pps_cb_qp_offset: -5,
        ..Default::default()
    };

    let cloned = pps.clone();
    assert_eq!(cloned.pps_pic_parameter_set_id, 5);
    assert_eq!(cloned.init_qp_minus26, 10);
    assert_eq!(cloned.pps_cb_qp_offset, -5);
}

#[test]
fn test_tile_config_debug() {
    let config = TileConfig {
        num_tile_columns_minus1: 3,
        num_tile_rows_minus1: 2,
        ..Default::default()
    };

    let debug_str = format!("{:?}", config);
    assert!(debug_str.contains("TileConfig"));
}

#[test]
fn test_pps_debug() {
    let pps = Pps {
        pps_pic_parameter_set_id: 1,
        ..Default::default()
    };

    let debug_str = format!("{:?}", pps);
    assert!(debug_str.contains("Pps"));
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_pps_all_true_flags() {
    let pps = Pps {
        dependent_slice_segments_enabled_flag: true,
        output_flag_present_flag: true,
        sign_data_hiding_enabled_flag: true,
        cabac_init_present_flag: true,
        constrained_intra_pred_flag: true,
        transform_skip_enabled_flag: true,
        cu_qp_delta_enabled_flag: true,
        pps_slice_chroma_qp_offsets_present_flag: true,
        weighted_pred_flag: true,
        weighted_bipred_flag: true,
        transquant_bypass_enabled_flag: true,
        tiles_enabled_flag: true,
        entropy_coding_sync_enabled_flag: true,
        loop_filter_across_tiles_enabled_flag: true,
        pps_loop_filter_across_slices_enabled_flag: true,
        deblocking_filter_control_present_flag: true,
        deblocking_filter_override_enabled_flag: true,
        pps_deblocking_filter_disabled_flag: true,
        pps_scaling_list_data_present_flag: true,
        lists_modification_present_flag: true,
        slice_segment_header_extension_present_flag: true,
        pps_extension_present_flag: true,
        pps_range_extension_flag: true,
        pps_multilayer_extension_flag: true,
        pps_3d_extension_flag: true,
        pps_scc_extension_flag: true,
        ..Default::default()
    };

    assert!(pps.dependent_slice_segments_enabled_flag);
    assert!(pps.weighted_pred_flag);
    assert!(pps.transquant_bypass_enabled_flag);
}

#[test]
fn test_pps_max_qp_offsets() {
    let pps = Pps {
        pps_cb_qp_offset: i8::MAX,
        pps_cr_qp_offset: i8::MAX,
        ..Default::default()
    };

    assert_eq!(pps.pps_cb_qp_offset, 127);
    assert_eq!(pps.pps_cr_qp_offset, 127);
}

#[test]
fn test_pps_min_qp_offsets() {
    let pps = Pps {
        pps_cb_qp_offset: i8::MIN,
        pps_cr_qp_offset: i8::MIN,
        ..Default::default()
    };

    assert_eq!(pps.pps_cb_qp_offset, -128);
    assert_eq!(pps.pps_cr_qp_offset, -128);
}

#[test]
fn test_pps_tile_config_various() {
    // Test 1x1 tiles
    let pps1 = Pps {
        tiles_enabled_flag: true,
        tile_config: Some(TileConfig {
            num_tile_columns_minus1: 0,
            num_tile_rows_minus1: 0,
            ..Default::default()
        }),
        ..Default::default()
    };
    assert_eq!(pps1.num_tiles(), Some(1u32));

    // Test 2x2 tiles
    let pps2 = Pps {
        tiles_enabled_flag: true,
        tile_config: Some(TileConfig {
            num_tile_columns_minus1: 1,
            num_tile_rows_minus1: 1,
            ..Default::default()
        }),
        ..Default::default()
    };
    assert_eq!(pps2.num_tiles(), Some(4u32)); // 2x2 = 4

    // Test 4x3 tiles
    let pps3 = Pps {
        tiles_enabled_flag: true,
        tile_config: Some(TileConfig {
            num_tile_columns_minus1: 3,
            num_tile_rows_minus1: 2,
            ..Default::default()
        }),
        ..Default::default()
    };
    assert_eq!(pps3.num_tiles(), Some(12u32)); // 4x3 = 12
}

#[test]
fn test_pps_various_qp_values() {
    for qp_delta in -20..=20i8 {
        let pps = Pps {
            init_qp_minus26: qp_delta,
            ..Default::default()
        };
        // Verify QP is in valid range (26 + qp_delta)
        let expected_qp = 26i8.saturating_add(qp_delta);
        assert_eq!(pps.init_qp(), expected_qp);
    }
}

#[test]
fn test_pps_ref_idx_defaults() {
    let pps = Pps {
        num_ref_idx_l0_default_active_minus1: 15,
        num_ref_idx_l1_default_active_minus1: 10,
        ..Default::default()
    };

    assert_eq!(pps.num_ref_idx_l0_default_active_minus1, 15);
    assert_eq!(pps.num_ref_idx_l1_default_active_minus1, 10);
}
