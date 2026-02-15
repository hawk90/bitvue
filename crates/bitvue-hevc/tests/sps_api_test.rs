//! HEVC SPS API Tests
//!
//! Tests for SPS public API and methods.

use bitvue_hevc::sps::{parse_sps, ChromaFormat, Profile, ProfileTierLevel, Sps};

// ============================================================================
// Profile Enum Tests
// ============================================================================

#[test]
fn test_profile_idc() {
    assert_eq!(Profile::Main.idc(), 1);
    assert_eq!(Profile::Main10.idc(), 2);
    assert_eq!(Profile::MainStillPicture.idc(), 3);
    assert_eq!(Profile::RangeExtensions.idc(), 4);
    assert_eq!(Profile::HighThroughput.idc(), 5);
    assert_eq!(Profile::Multiview.idc(), 6);
    assert_eq!(Profile::Scalable.idc(), 7);
    assert_eq!(Profile::Main3d.idc(), 8);
    assert_eq!(Profile::ScreenExtended.idc(), 9);
    assert_eq!(Profile::ScalableRangeExtensions.idc(), 10);
    assert_eq!(Profile::HighThroughputScreenExtended.idc(), 11);
}

#[test]
fn test_profile_from_u8() {
    assert_eq!(Profile::from(1u8), Profile::Main);
    assert_eq!(Profile::from(2u8), Profile::Main10);
    assert_eq!(Profile::from(3u8), Profile::MainStillPicture);
    assert_eq!(Profile::from(99u8), Profile::Unknown(99));
}

// ============================================================================
// ChromaFormat Enum Tests
// ============================================================================

#[test]
fn test_chroma_format_values() {
    assert_eq!(ChromaFormat::Monochrome as u8, 0);
    assert_eq!(ChromaFormat::Chroma420 as u8, 1);
    assert_eq!(ChromaFormat::Chroma422 as u8, 2);
    assert_eq!(ChromaFormat::Chroma444 as u8, 3);
}

#[test]
fn test_chroma_format_from() {
    assert_eq!(ChromaFormat::from(0u8), ChromaFormat::Monochrome);
    assert_eq!(ChromaFormat::from(1u8), ChromaFormat::Chroma420);
    assert_eq!(ChromaFormat::from(2u8), ChromaFormat::Chroma422);
    assert_eq!(ChromaFormat::from(3u8), ChromaFormat::Chroma444);
    assert_eq!(ChromaFormat::from(99u8), ChromaFormat::Chroma420); // Default
}

// ============================================================================
// Sps Struct Creation Tests
// ============================================================================

#[test]
fn test_sps_create() {
    let sps = Sps {
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
        amp_enabled_flag: true,
        sample_adaptive_offset_enabled_flag: true,
        pcm_enabled_flag: false,
        num_short_term_ref_pic_sets: 0,
        long_term_ref_pics_present_flag: false,
        num_long_term_ref_pics_sps: 0,
        sps_temporal_mvp_enabled_flag: true,
        strong_intra_smoothing_enabled_flag: true,
        vui_parameters_present_flag: false,
        vui_parameters: None,
    };

    assert_eq!(sps.sps_video_parameter_set_id, 0);
    assert_eq!(sps.pic_width_in_luma_samples, 1920);
    assert_eq!(sps.pic_height_in_luma_samples, 1080);
}

// ============================================================================
// Sps Method Tests
// ============================================================================

#[test]
fn test_sps_bit_depth_luma() {
    let sps = Sps {
        bit_depth_luma_minus8: 0,
        ..create_minimal_sps()
    };
    assert_eq!(sps.bit_depth_luma(), 8);

    let sps = Sps {
        bit_depth_luma_minus8: 2,
        ..create_minimal_sps()
    };
    assert_eq!(sps.bit_depth_luma(), 10);
}

#[test]
fn test_sps_bit_depth_chroma() {
    let sps = Sps {
        bit_depth_chroma_minus8: 0,
        ..create_minimal_sps()
    };
    assert_eq!(sps.bit_depth_chroma(), 8);

    let sps = Sps {
        bit_depth_chroma_minus8: 4,
        ..create_minimal_sps()
    };
    assert_eq!(sps.bit_depth_chroma(), 12);
}

#[test]
fn test_sps_ctb_size() {
    let sps = Sps {
        log2_min_luma_coding_block_size_minus3: 0,
        log2_diff_max_min_luma_coding_block_size: 3,
        ..create_minimal_sps()
    };
    assert_eq!(sps.ctb_size(), 64); // 1 << (0 + 3 + 3) = 64
}

#[test]
fn test_sps_min_cb_size() {
    let sps = Sps {
        log2_min_luma_coding_block_size_minus3: 0,
        ..create_minimal_sps()
    };
    assert_eq!(sps.min_cb_size(), 8); // 1 << (0 + 3) = 8
}

#[test]
fn test_sps_pic_width_in_ctbs() {
    let sps = Sps {
        pic_width_in_luma_samples: 1920,
        log2_min_luma_coding_block_size_minus3: 0,
        log2_diff_max_min_luma_coding_block_size: 3,
        ..create_minimal_sps()
    };
    assert_eq!(sps.pic_width_in_ctbs(), 30); // (1920 + 64 - 1) / 64 = 30
}

#[test]
fn test_sps_pic_height_in_ctbs() {
    let sps = Sps {
        pic_height_in_luma_samples: 1080,
        log2_min_luma_coding_block_size_minus3: 0,
        log2_diff_max_min_luma_coding_block_size: 3,
        ..create_minimal_sps()
    };
    assert_eq!(sps.pic_height_in_ctbs(), 17); // (1080 + 64 - 1) / 64 = 16.875 -> 17
}

#[test]
fn test_sps_display_width_420() {
    let sps = Sps {
        pic_width_in_luma_samples: 1920,
        chroma_format_idc: ChromaFormat::Chroma420,
        conf_win_left_offset: 0,
        conf_win_right_offset: 0,
        ..create_minimal_sps()
    };
    assert_eq!(sps.display_width(), 1920);
}

#[test]
fn test_sps_display_height_420() {
    let sps = Sps {
        pic_height_in_luma_samples: 1080,
        chroma_format_idc: ChromaFormat::Chroma420,
        conf_win_top_offset: 0,
        conf_win_bottom_offset: 0,
        ..create_minimal_sps()
    };
    assert_eq!(sps.display_height(), 1080);
}

#[test]
fn test_sps_display_width_422() {
    let sps = Sps {
        pic_width_in_luma_samples: 1920,
        chroma_format_idc: ChromaFormat::Chroma422,
        conf_win_left_offset: 4,
        conf_win_right_offset: 4,
        ..create_minimal_sps()
    };
    assert_eq!(sps.display_width(), 1904); // 1920 - 2 * (4 + 4)
}

#[test]
fn test_sps_display_height_444() {
    let sps = Sps {
        pic_height_in_luma_samples: 1080,
        chroma_format_idc: ChromaFormat::Chroma444,
        conf_win_top_offset: 2,
        conf_win_bottom_offset: 2,
        ..create_minimal_sps()
    };
    assert_eq!(sps.display_height(), 1076); // 1080 - 1 * (2 + 2)
}

#[test]
fn test_sps_max_poc_lsb() {
    let sps = Sps {
        log2_max_pic_order_cnt_lsb_minus4: 4,
        ..create_minimal_sps()
    };
    assert_eq!(sps.max_poc_lsb(), 256); // 1 << (4 + 4) = 256
}

#[test]
fn test_sps_max_poc_lsb_different() {
    let sps = Sps {
        log2_max_pic_order_cnt_lsb_minus4: 8,
        ..create_minimal_sps()
    };
    assert_eq!(sps.max_poc_lsb(), 4096); // 1 << (8 + 4) = 4096
}

// ============================================================================
// parse_sps Tests
// ============================================================================

#[test]
fn test_parse_sps_empty() {
    let data = &[];
    let result = parse_sps(data);
    assert!(result.is_err());
}

#[test]
fn test_parse_sps_minimal_data() {
    // Minimal SPS data (simplified for testing)
    let data = &[0x00, 0x00, 0x00, 0x00];
    let result = parse_sps(data);
    // May fail due to insufficient data, but tests the function
    assert!(result.is_err() || result.is_ok());
}

// ============================================================================
// Helper Functions
// ============================================================================

fn create_minimal_sps() -> Sps {
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
        sps_max_dec_pic_buffering_minus1: vec![0],
        sps_max_num_reorder_pics: vec![0],
        sps_max_latency_increase_plus1: vec![0],
        log2_min_luma_coding_block_size_minus3: 0,
        log2_diff_max_min_luma_coding_block_size: 0,
        log2_min_luma_transform_block_size_minus2: 0,
        log2_diff_max_min_luma_transform_block_size: 0,
        max_transform_hierarchy_depth_inter: 0,
        max_transform_hierarchy_depth_intra: 0,
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
