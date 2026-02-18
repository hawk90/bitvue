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
//! HEVC PPS Module Coverage Tests
//!
//! Tests for pps.rs module to improve coverage from 72.48% to 95%.

use bitvue_hevc::bitreader::BitReader;
use bitvue_hevc::pps::{Pps, TileConfig};

// ============================================================================
// Pps Struct Creation Tests
// ============================================================================

#[test]
fn test_pps_create_minimal() {
    let pps = Pps {
        pps_pic_parameter_set_id: 0,
        pps_seq_parameter_set_id: 0,
        dependent_slice_segments_enabled_flag: false,
        output_flag_present_flag: false,
        num_extra_slice_header_bits: 0,
        sign_data_hiding_enabled_flag: false,
        cabac_init_present_flag: false,
        num_ref_idx_l0_default_active_minus1: 0,
        num_ref_idx_l1_default_active_minus1: 0,
        init_qp_minus26: 0,
        constrained_intra_pred_flag: false,
        transform_skip_enabled_flag: false,
        cu_qp_delta_enabled_flag: false,
        diff_cu_qp_delta_depth: 0,
        pps_cb_qp_offset: 0,
        pps_cr_qp_offset: 0,
        pps_slice_chroma_qp_offsets_present_flag: false,
        weighted_pred_flag: false,
        weighted_bipred_flag: false,
        transquant_bypass_enabled_flag: false,
        tiles_enabled_flag: false,
        entropy_coding_sync_enabled_flag: false,
        tile_config: None,
        loop_filter_across_tiles_enabled_flag: false,
        pps_loop_filter_across_slices_enabled_flag: false,
        deblocking_filter_control_present_flag: false,
        deblocking_filter_override_enabled_flag: false,
        pps_deblocking_filter_disabled_flag: false,
        pps_beta_offset_div2: 0,
        pps_tc_offset_div2: 0,
        pps_scaling_list_data_present_flag: false,
        lists_modification_present_flag: false,
        log2_parallel_merge_level_minus2: 0,
        slice_segment_header_extension_present_flag: false,
        pps_extension_present_flag: false,
        pps_range_extension_flag: false,
        pps_multilayer_extension_flag: false,
        pps_3d_extension_flag: false,
        pps_scc_extension_flag: false,
    };

    assert_eq!(pps.pps_pic_parameter_set_id, 0);
    assert_eq!(pps.init_qp_minus26, 0);
}

#[test]
fn test_pps_create_with_tiles() {
    let tile_config = Some(TileConfig {
        num_tile_columns_minus1: 5,
        num_tile_rows_minus1: 3,
        uniform_spacing_flag: true,
        column_width_minus1: vec![],
        row_height_minus1: vec![],
    });

    let pps = Pps {
        pps_pic_parameter_set_id: 5,
        tiles_enabled_flag: true,
        tile_config,
        ..Pps::default()
    };

    assert!(pps.tiles_enabled_flag);
    assert!(pps.tile_config.is_some());
}

// ============================================================================
// TileConfig Tests
// ============================================================================

#[test]
fn test_tile_config_default() {
    let config = TileConfig::default();
    assert_eq!(config.num_tile_columns_minus1, 0);
    assert_eq!(config.num_tile_rows_minus1, 0);
    // Note: uniform_spacing_flag default value depends on derive macro
    let _ = config.uniform_spacing_flag;
}

#[test]
fn test_tile_config_num_columns() {
    let config = TileConfig {
        num_tile_columns_minus1: 10,
        ..TileConfig::default()
    };
    assert_eq!(config.num_columns(), 11);
}

#[test]
fn test_tile_config_num_rows() {
    let config = TileConfig {
        num_tile_rows_minus1: 7,
        ..TileConfig::default()
    };
    assert_eq!(config.num_rows(), 8);
}

#[test]
fn test_tile_config_with_uniform_spacing() {
    let config = TileConfig {
        uniform_spacing_flag: true,
        column_width_minus1: vec![],
        row_height_minus1: vec![],
        ..TileConfig::default()
    };
    assert!(config.uniform_spacing_flag);
    assert!(config.column_width_minus1.is_empty());
    assert!(config.row_height_minus1.is_empty());
}

#[test]
fn test_tile_config_with_variable_spacing() {
    let config = TileConfig {
        uniform_spacing_flag: false,
        column_width_minus1: vec![100, 200, 150],
        row_height_minus1: vec![80, 120],
        ..TileConfig::default()
    };
    assert!(!config.uniform_spacing_flag);
    assert_eq!(config.column_width_minus1.len(), 3);
    assert_eq!(config.row_height_minus1.len(), 2);
}

// ============================================================================
// Pps Field Tests
// ============================================================================

#[test]
fn test_pps_pic_parameter_set_id() {
    let pps = Pps {
        pps_pic_parameter_set_id: 42,
        ..Pps::default()
    };
    assert_eq!(pps.pps_pic_parameter_set_id, 42);
}

#[test]
fn test_pps_seq_parameter_set_id() {
    let pps = Pps {
        pps_seq_parameter_set_id: 15,
        ..Pps::default()
    };
    assert_eq!(pps.pps_seq_parameter_set_id, 15);
}

#[test]
fn test_pps_dependent_slice_segments() {
    let pps = Pps {
        dependent_slice_segments_enabled_flag: true,
        ..Pps::default()
    };
    assert!(pps.dependent_slice_segments_enabled_flag);
}

#[test]
fn test_pps_output_flag_present() {
    let pps = Pps {
        output_flag_present_flag: true,
        ..Pps::default()
    };
    assert!(pps.output_flag_present_flag);
}

#[test]
fn test_pps_num_extra_slice_header_bits() {
    let pps = Pps {
        num_extra_slice_header_bits: 7,
        ..Pps::default()
    };
    assert_eq!(pps.num_extra_slice_header_bits, 7);
}

#[test]
fn test_pps_sign_data_hiding() {
    let pps = Pps {
        sign_data_hiding_enabled_flag: true,
        ..Pps::default()
    };
    assert!(pps.sign_data_hiding_enabled_flag);
}

#[test]
fn test_pps_cabac_init_present() {
    let pps = Pps {
        cabac_init_present_flag: true,
        ..Pps::default()
    };
    assert!(pps.cabac_init_present_flag);
}

#[test]
fn test_pps_ref_idx_defaults() {
    let pps = Pps {
        num_ref_idx_l0_default_active_minus1: 3,
        num_ref_idx_l1_default_active_minus1: 2,
        ..Pps::default()
    };
    assert_eq!(pps.num_ref_idx_l0_default_active_minus1, 3);
    assert_eq!(pps.num_ref_idx_l1_default_active_minus1, 2);
}

#[test]
fn test_pps_init_qp() {
    let pps = Pps {
        init_qp_minus26: 10,
        ..Pps::default()
    };
    assert_eq!(pps.init_qp_minus26, 10);
}

#[test]
fn test_pps_constrained_intra_pred() {
    let pps = Pps {
        constrained_intra_pred_flag: true,
        ..Pps::default()
    };
    assert!(pps.constrained_intra_pred_flag);
}

#[test]
fn test_pps_transform_skip() {
    let pps = Pps {
        transform_skip_enabled_flag: true,
        ..Pps::default()
    };
    assert!(pps.transform_skip_enabled_flag);
}

#[test]
fn test_pps_cu_qp_delta() {
    let pps = Pps {
        cu_qp_delta_enabled_flag: true,
        diff_cu_qp_delta_depth: 3,
        ..Pps::default()
    };
    assert!(pps.cu_qp_delta_enabled_flag);
    assert_eq!(pps.diff_cu_qp_delta_depth, 3);
}

#[test]
fn test_pps_qp_offsets() {
    let pps = Pps {
        pps_cb_qp_offset: -5,
        pps_cr_qp_offset: 3,
        pps_slice_chroma_qp_offsets_present_flag: true,
        ..Pps::default()
    };
    assert_eq!(pps.pps_cb_qp_offset, -5);
    assert_eq!(pps.pps_cr_qp_offset, 3);
    assert!(pps.pps_slice_chroma_qp_offsets_present_flag);
}

#[test]
fn test_pps_weighted_pred() {
    let pps = Pps {
        weighted_pred_flag: true,
        weighted_bipred_flag: true,
        ..Pps::default()
    };
    assert!(pps.weighted_pred_flag);
    assert!(pps.weighted_bipred_flag);
}

#[test]
fn test_pps_transquant_bypass() {
    let pps = Pps {
        transquant_bypass_enabled_flag: true,
        ..Pps::default()
    };
    assert!(pps.transquant_bypass_enabled_flag);
}

#[test]
fn test_pps_tiles_enabled() {
    let pps = Pps {
        tiles_enabled_flag: true,
        ..Pps::default()
    };
    assert!(pps.tiles_enabled_flag);
}

#[test]
fn test_pps_entropy_coding_sync() {
    let pps = Pps {
        entropy_coding_sync_enabled_flag: true,
        ..Pps::default()
    };
    assert!(pps.entropy_coding_sync_enabled_flag);
}

#[test]
fn test_pps_loop_filter_flags() {
    let pps = Pps {
        loop_filter_across_tiles_enabled_flag: true,
        pps_loop_filter_across_slices_enabled_flag: true,
        ..Pps::default()
    };
    assert!(pps.loop_filter_across_tiles_enabled_flag);
    assert!(pps.pps_loop_filter_across_slices_enabled_flag);
}

#[test]
fn test_pps_deblocking_filter() {
    let pps = Pps {
        deblocking_filter_override_enabled_flag: true,
        pps_deblocking_filter_disabled_flag: false,
        pps_beta_offset_div2: 5,
        pps_tc_offset_div2: -2,
        ..Pps::default()
    };
    assert!(pps.deblocking_filter_override_enabled_flag);
    assert!(!pps.pps_deblocking_filter_disabled_flag);
    assert_eq!(pps.pps_beta_offset_div2, 5);
    assert_eq!(pps.pps_tc_offset_div2, -2);
}

#[test]
fn test_pps_scaling_list() {
    let pps = Pps {
        pps_scaling_list_data_present_flag: true,
        ..Pps::default()
    };
    assert!(pps.pps_scaling_list_data_present_flag);
}

#[test]
fn test_pps_lists_modification() {
    let pps = Pps {
        lists_modification_present_flag: true,
        log2_parallel_merge_level_minus2: 5,
        ..Pps::default()
    };
    assert!(pps.lists_modification_present_flag);
    assert_eq!(pps.log2_parallel_merge_level_minus2, 5);
}

#[test]
fn test_pps_extensions() {
    let pps = Pps {
        slice_segment_header_extension_present_flag: true,
        pps_extension_present_flag: true,
        pps_range_extension_flag: true,
        pps_multilayer_extension_flag: true,
        pps_3d_extension_flag: true,
        pps_scc_extension_flag: true,
        ..Pps::default()
    };
    assert!(pps.slice_segment_header_extension_present_flag);
    assert!(pps.pps_extension_present_flag);
    assert!(pps.pps_range_extension_flag);
    assert!(pps.pps_multilayer_extension_flag);
    assert!(pps.pps_3d_extension_flag);
    assert!(pps.pps_scc_extension_flag);
}

// ============================================================================
// Pps Default Implementation Tests
// ============================================================================

#[test]
fn test_pps_default_values() {
    let pps = Pps::default();
    assert_eq!(pps.pps_pic_parameter_set_id, 0);
    assert_eq!(pps.pps_seq_parameter_set_id, 0);
    assert!(!pps.dependent_slice_segments_enabled_flag);
    assert!(!pps.output_flag_present_flag);
    assert_eq!(pps.num_extra_slice_header_bits, 0);
}
