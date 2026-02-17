#![allow(dead_code)]
//! HEVC Slice Header Tests
//!
//! Tests for slice.rs module functionality.

use bitvue_hevc::nal::NalUnitType;
use bitvue_hevc::pps::Pps;
use bitvue_hevc::slice::{parse_slice_header, SliceHeader, SliceType};
use bitvue_hevc::sps::{ChromaFormat, Profile, ProfileTierLevel, Sps};
use std::collections::HashMap;

// ============================================================================
// Helper Functions
// ============================================================================

fn create_test_sps() -> Sps {
    Sps {
        sps_video_parameter_set_id: 0,
        sps_max_sub_layers_minus1: 0,
        sps_temporal_id_nesting_flag: false,
        profile_tier_level: ProfileTierLevel {
            general_profile_space: 0,
            general_tier_flag: false,
            general_profile_idc: Profile::Main,
            general_profile_compatibility_flags: 0,
            general_progressive_source_flag: true,
            general_interlaced_source_flag: false,
            general_non_packed_constraint_flag: true,
            general_frame_only_constraint_flag: true,
            general_level_idc: 120,
        },
        sps_seq_parameter_set_id: 0,
        chroma_format_idc: ChromaFormat::Chroma420,
        separate_colour_plane_flag: false,
        pic_width_in_luma_samples: 1920,
        pic_height_in_luma_samples: 1080,
        conformance_window_flag: false,
        conf_win_left_offset: 0,
        conf_win_right_offset: 0,
        conf_win_top_offset: 0,
        conf_win_bottom_offset: 0,
        bit_depth_luma_minus8: 0,
        bit_depth_chroma_minus8: 0,
        log2_max_pic_order_cnt_lsb_minus4: 4,
        sps_sub_layer_ordering_info_present_flag: false,
        sps_max_dec_pic_buffering_minus1: vec![4],
        sps_max_num_reorder_pics: vec![2],
        sps_max_latency_increase_plus1: vec![0],
        log2_min_luma_coding_block_size_minus3: 0,
        log2_diff_max_min_luma_coding_block_size: 3,
        log2_min_luma_transform_block_size_minus2: 0,
        log2_diff_max_min_luma_transform_block_size: 3,
        max_transform_hierarchy_depth_inter: 2,
        max_transform_hierarchy_depth_intra: 2,
        scaling_list_enabled_flag: false,
        amp_enabled_flag: false,
        sample_adaptive_offset_enabled_flag: false,
        pcm_enabled_flag: false,
        num_short_term_ref_pic_sets: 0,
        long_term_ref_pics_present_flag: false,
        num_long_term_ref_pics_sps: 0,
        sps_temporal_mvp_enabled_flag: false,
        strong_intra_smoothing_enabled_flag: false,
        vui_parameters_present_flag: false,
        vui_parameters: None,
    }
}

fn create_test_pps() -> Pps {
    Pps {
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
    }
}

// ============================================================================
// SliceType Tests
// ============================================================================

#[test]
fn test_slice_type_b() {
    let slice_type = SliceType::B;
    assert_eq!(slice_type.name(), "B");
    assert!(slice_type.is_inter());
    assert!(!slice_type.is_intra());
}

#[test]
fn test_slice_type_p() {
    let slice_type = SliceType::P;
    assert_eq!(slice_type.name(), "P");
    assert!(slice_type.is_inter());
    assert!(!slice_type.is_intra());
}

#[test]
fn test_slice_type_i() {
    let slice_type = SliceType::I;
    assert_eq!(slice_type.name(), "I");
    assert!(!slice_type.is_inter());
    assert!(slice_type.is_intra());
}

#[test]
fn test_slice_type_from_u32() {
    assert_eq!(SliceType::from_u32(0), Some(SliceType::B));
    assert_eq!(SliceType::from_u32(1), Some(SliceType::P));
    assert_eq!(SliceType::from_u32(2), Some(SliceType::I));
    assert_eq!(SliceType::from_u32(99), None);
}

#[test]
fn test_slice_type_as_str() {
    assert_eq!(SliceType::B.as_str(), "B");
    assert_eq!(SliceType::P.as_str(), "P");
    assert_eq!(SliceType::I.as_str(), "I");
}

// ============================================================================
// SliceHeader Tests
// ============================================================================

#[test]
fn test_slice_header_default() {
    let header = SliceHeader::default();
    assert!(header.first_slice_segment_in_pic_flag);
    assert_eq!(header.slice_type, SliceType::I);
    assert_eq!(header.pic_output_flag, true);
    assert_eq!(header.slice_qp_delta, 0);
    assert_eq!(header.max_num_merge_cand(), 5);
}

#[test]
fn test_slice_header_is_intra() {
    let header = SliceHeader {
        slice_type: SliceType::I,
        ..SliceHeader::default()
    };
    assert!(header.is_intra());
    assert!(!header.is_inter());
}

#[test]
fn test_slice_header_is_inter() {
    let header = SliceHeader {
        slice_type: SliceType::P,
        ..SliceHeader::default()
    };
    assert!(!header.is_intra());
    assert!(header.is_inter());
}

#[test]
fn test_slice_header_num_ref_idx_l0_active() {
    let header = SliceHeader {
        num_ref_idx_l0_active_minus1: 2,
        ..SliceHeader::default()
    };
    assert_eq!(header.num_ref_idx_l0_active(), 3);
}

#[test]
fn test_slice_header_num_ref_idx_l1_active_b_slice() {
    let header = SliceHeader {
        slice_type: SliceType::B,
        num_ref_idx_l1_active_minus1: 1,
        ..SliceHeader::default()
    };
    assert_eq!(header.num_ref_idx_l1_active(), 2);
}

#[test]
fn test_slice_header_num_ref_idx_l1_active_non_b() {
    let header = SliceHeader {
        slice_type: SliceType::P,
        num_ref_idx_l1_active_minus1: 1,
        ..SliceHeader::default()
    };
    assert_eq!(header.num_ref_idx_l1_active(), 0);
}

#[test]
fn test_slice_header_qp() {
    let pps = create_test_pps();
    let header = SliceHeader {
        slice_qp_delta: 5,
        ..SliceHeader::default()
    };
    assert_eq!(header.qp(&pps), 31); // 26 + 0 + 5
}

// ============================================================================
// parse_slice_header Tests
// ============================================================================

#[test]
fn test_parse_slice_header_empty_data() {
    let data = &[];
    let sps_map = HashMap::new();
    let pps_map = HashMap::new();
    let nal_type = NalUnitType::TrailR;

    let result = parse_slice_header(data, &sps_map, &pps_map, nal_type);
    // Empty data should cause an error when reading first bit
    assert!(result.is_err());
}

#[test]
fn test_parse_slice_header_no_pps() {
    let data = &[0x80]; // first_slice_segment_in_pic_flag = 1
    let sps_map = HashMap::new();
    let pps_map = HashMap::new();
    let nal_type = NalUnitType::TrailR;

    let result = parse_slice_header(data, &sps_map, &pps_map, nal_type);
    assert!(result.is_err());
}

#[test]
fn test_parse_slice_header_with_sps_pps() {
    let data = &[0x80, 0x00]; // first_slice=1, pps_id=0
    let sps_map = {
        let mut m = HashMap::new();
        m.insert(0, create_test_sps());
        m
    };
    let pps_map = {
        let mut m = HashMap::new();
        m.insert(0, create_test_pps());
        m
    };

    let nal_type = NalUnitType::TrailR;

    let result = parse_slice_header(data, &sps_map, &pps_map, nal_type);
    // Test data is too short for full parsing, should error
    assert!(result.is_err());
}

#[test]
fn test_parse_slice_header_idr_nal() {
    let data = &[0x80, 0x00]; // first_slice=1, pps_id=0
    let sps_map = {
        let mut m = HashMap::new();
        m.insert(0, create_test_sps());
        m
    };
    let pps_map = {
        let mut m = HashMap::new();
        m.insert(0, create_test_pps());
        m
    };

    let nal_type = NalUnitType::IdrWRadl;

    let result = parse_slice_header(data, &sps_map, &pps_map, nal_type);
    // Test data is too short for full parsing, should error
    assert!(result.is_err());
}
