//! Comprehensive tests for HEVC SPS module
//!
//! Tests Profile, ChromaFormat, ProfileTierLevel, VuiParameters, Sps

use bitvue_hevc::sps::{
    parse_sps, ChromaFormat, Profile, ProfileTierLevel, Sps, VuiParameters,
};

// ============================================================================
// Profile Tests
// ============================================================================

#[test]
fn test_profile_from_u8_all_known() {
    assert_eq!(Profile::from(1), Profile::Main);
    assert_eq!(Profile::from(2), Profile::Main10);
    assert_eq!(Profile::from(3), Profile::MainStillPicture);
    assert_eq!(Profile::from(4), Profile::RangeExtensions);
    assert_eq!(Profile::from(5), Profile::HighThroughput);
    assert_eq!(Profile::from(6), Profile::Multiview);
    assert_eq!(Profile::from(7), Profile::Scalable);
    assert_eq!(Profile::from(8), Profile::Main3d);
    assert_eq!(Profile::from(9), Profile::ScreenExtended);
    assert_eq!(Profile::from(10), Profile::ScalableRangeExtensions);
    assert_eq!(Profile::from(11), Profile::HighThroughputScreenExtended);
}

#[test]
fn test_profile_from_u8_unknown() {
    assert_eq!(Profile::from(0), Profile::Unknown(0));
    assert_eq!(Profile::from(12), Profile::Unknown(12));
    assert_eq!(Profile::from(255), Profile::Unknown(255));
    assert_eq!(Profile::from(100), Profile::Unknown(100));
}

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
    assert_eq!(Profile::Unknown(42).idc(), 42);
}

#[test]
fn test_profile_copy() {
    let p = Profile::Main10;
    let copied = p;
    assert_eq!(p, Profile::Main10);
    assert_eq!(copied, Profile::Main10);
}

#[test]
fn test_profile_eq() {
    assert_eq!(Profile::Main, Profile::Main);
    assert_eq!(Profile::Main10, Profile::Main10);
    assert_ne!(Profile::Main, Profile::Main10);
    assert_ne!(Profile::Main, Profile::Unknown(1));
}

// ============================================================================
// ChromaFormat Tests
// ============================================================================

#[test]
fn test_chroma_format_from_u8_all_variants() {
    assert_eq!(ChromaFormat::from(0), ChromaFormat::Monochrome);
    assert_eq!(ChromaFormat::from(1), ChromaFormat::Chroma420);
    assert_eq!(ChromaFormat::from(2), ChromaFormat::Chroma422);
    assert_eq!(ChromaFormat::from(3), ChromaFormat::Chroma444);
}

#[test]
fn test_chroma_format_from_u8_unknown() {
    // Unknown values default to Chroma420
    assert_eq!(ChromaFormat::from(4), ChromaFormat::Chroma420);
    assert_eq!(ChromaFormat::from(255), ChromaFormat::Chroma420);
}

#[test]
fn test_chroma_format_copy() {
    let cf = ChromaFormat::Chroma420;
    let copied = cf;
    assert_eq!(cf, ChromaFormat::Chroma420);
    assert_eq!(copied, ChromaFormat::Chroma420);
}

#[test]
fn test_chroma_format_eq() {
    assert_eq!(ChromaFormat::Monochrome, ChromaFormat::Monochrome);
    assert_eq!(ChromaFormat::Chroma420, ChromaFormat::Chroma420);
    assert_ne!(ChromaFormat::Chroma420, ChromaFormat::Chroma422);
}

// ============================================================================
// ProfileTierLevel Tests
// ============================================================================

#[test]
fn test_profile_tier_level_default() {
    let ptl = ProfileTierLevel {
        general_profile_space: 0,
        general_tier_flag: false,
        general_profile_idc: Profile::Main,
        general_profile_compatibility_flags: 0,
        general_progressive_source_flag: false,
        general_interlaced_source_flag: false,
        general_non_packed_constraint_flag: false,
        general_frame_only_constraint_flag: false,
        general_level_idc: 0,
    };

    assert_eq!(ptl.general_profile_space, 0);
    assert!(!ptl.general_tier_flag);
    assert_eq!(ptl.general_profile_idc, Profile::Main);
}

#[test]
fn test_profile_tier_level_with_values() {
    let ptl = ProfileTierLevel {
        general_profile_space: 2,
        general_tier_flag: true,
        general_profile_idc: Profile::Main10,
        general_profile_compatibility_flags: 0xFFFFFFFF,
        general_progressive_source_flag: true,
        general_interlaced_source_flag: false,
        general_non_packed_constraint_flag: true,
        general_frame_only_constraint_flag: true,
        general_level_idc: 51, // Level 5.1
    };

    assert_eq!(ptl.general_profile_space, 2);
    assert!(ptl.general_tier_flag);
    assert_eq!(ptl.general_profile_idc, Profile::Main10);
    assert_eq!(ptl.general_level_idc, 51);
}

#[test]
fn test_profile_tier_level_clone() {
    let ptl = ProfileTierLevel {
        general_profile_space: 1,
        general_tier_flag: true,
        general_profile_idc: Profile::HighThroughput,
        general_profile_compatibility_flags: 12345,
        general_progressive_source_flag: true,
        general_interlaced_source_flag: false,
        general_non_packed_constraint_flag: false,
        general_frame_only_constraint_flag: true,
        general_level_idc: 30,
    };

    let cloned = ptl.clone();
    assert_eq!(cloned.general_profile_space, 1);
    assert_eq!(cloned.general_level_idc, 30);
}

// ============================================================================
// VuiParameters Tests
// ============================================================================

#[test]
fn test_vui_parameters_default() {
    let vui = VuiParameters::default();
    assert!(!vui.aspect_ratio_info_present_flag);
    assert!(!vui.overscan_info_present_flag);
    assert!(!vui.video_signal_type_present_flag);
    assert!(!vui.chroma_loc_info_present_flag);
    assert!(!vui.timing_info_present_flag);
}

#[test]
fn test_vui_parameters_with_aspect_ratio() {
    let vui = VuiParameters {
        aspect_ratio_info_present_flag: true,
        aspect_ratio_idc: Some(255), // Extended SAR
        sar_width: Some(16),
        sar_height: Some(9),
        ..Default::default()
    };

    assert!(vui.aspect_ratio_info_present_flag);
    assert_eq!(vui.aspect_ratio_idc, Some(255));
    assert_eq!(vui.sar_width, Some(16));
    assert_eq!(vui.sar_height, Some(9));
}

#[test]
fn test_vui_parameters_with_timing() {
    let vui = VuiParameters {
        timing_info_present_flag: true,
        num_units_in_tick: Some(1001),
        time_scale: Some(60000),
        ..Default::default()
    };

    assert!(vui.timing_info_present_flag);
    assert_eq!(vui.num_units_in_tick, Some(1001));
    assert_eq!(vui.time_scale, Some(60000));
}

#[test]
fn test_vui_parameters_clone() {
    let vui = VuiParameters {
        aspect_ratio_info_present_flag: true,
        aspect_ratio_idc: Some(1),
        sar_width: Some(4),
        sar_height: Some(3),
        ..Default::default()
    };

    let cloned = vui.clone();
    assert!(cloned.aspect_ratio_info_present_flag);
    assert_eq!(cloned.sar_width, Some(4));
}

// ============================================================================
// Sps Methods Tests
// ============================================================================

#[test]
fn test_sps_bit_depth_luma() {
    let sps = Sps {
        bit_depth_luma_minus8: 0,
        ..default_sps()
    };

    assert_eq!(sps.bit_depth_luma(), 8);

    let sps = Sps {
        bit_depth_luma_minus8: 2,
        ..default_sps()
    };

    assert_eq!(sps.bit_depth_luma(), 10);

    let sps = Sps {
        bit_depth_luma_minus8: 4,
        ..default_sps()
    };

    assert_eq!(sps.bit_depth_luma(), 12);
}

#[test]
fn test_sps_bit_depth_chroma() {
    let sps = Sps {
        bit_depth_chroma_minus8: 0,
        ..default_sps()
    };

    assert_eq!(sps.bit_depth_chroma(), 8);

    let sps = Sps {
        bit_depth_chroma_minus8: 2,
        ..default_sps()
    };

    assert_eq!(sps.bit_depth_chroma(), 10);
}

#[test]
fn test_sps_ctb_size() {
    let sps = Sps {
        log2_min_luma_coding_block_size_minus3: 0,
        log2_diff_max_min_luma_coding_block_size: 0,
        ..default_sps()
    };

    assert_eq!(sps.ctb_size(), 8); // 1 << (0 + 3 + 0) = 8

    let sps = Sps {
        log2_min_luma_coding_block_size_minus3: 0,
        log2_diff_max_min_luma_coding_block_size: 1,
        ..default_sps()
    };

    assert_eq!(sps.ctb_size(), 16); // 1 << (0 + 3 + 1) = 16

    let sps = Sps {
        log2_min_luma_coding_block_size_minus3: 1,
        log2_diff_max_min_luma_coding_block_size: 2,
        ..default_sps()
    };

    assert_eq!(sps.ctb_size(), 64); // 1 << (1 + 3 + 2) = 64
}

#[test]
fn test_sps_min_cb_size() {
    let sps = Sps {
        log2_min_luma_coding_block_size_minus3: 0,
        ..default_sps()
    };

    assert_eq!(sps.min_cb_size(), 8); // 1 << (0 + 3) = 8

    let sps = Sps {
        log2_min_luma_coding_block_size_minus3: 1,
        ..default_sps()
    };

    assert_eq!(sps.min_cb_size(), 16); // 1 << (1 + 3) = 16
}

#[test]
fn test_sps_pic_width_in_ctbs() {
    let sps = Sps {
        pic_width_in_luma_samples: 1920,
        log2_min_luma_coding_block_size_minus3: 0,
        log2_diff_max_min_luma_coding_block_size: 1, // CTB size = 16
        ..default_sps()
    };

    assert_eq!(sps.pic_width_in_ctbs(), 120); // ceil(1920/16) = 120
}

#[test]
fn test_sps_pic_height_in_ctbs() {
    let sps = Sps {
        pic_height_in_luma_samples: 1080,
        log2_min_luma_coding_block_size_minus3: 0,
        log2_diff_max_min_luma_coding_block_size: 1, // CTB size = 16
        ..default_sps()
    };

    assert_eq!(sps.pic_height_in_ctbs(), 68); // ceil(1080/16) = 68
}

#[test]
fn test_sps_display_width_chroma420() {
    let sps = Sps {
        pic_width_in_luma_samples: 1920,
        chroma_format_idc: ChromaFormat::Chroma420,
        conf_win_left_offset: 0,
        conf_win_right_offset: 0,
        ..default_sps()
    };

    assert_eq!(sps.display_width(), 1920);
}

#[test]
fn test_sps_display_width_with_conformance_window() {
    let sps = Sps {
        pic_width_in_luma_samples: 1920,
        chroma_format_idc: ChromaFormat::Chroma420,
        conf_win_left_offset: 10,
        conf_win_right_offset: 10,
        ..default_sps()
    };

    assert_eq!(sps.display_width(), 1880); // 1920 - 2*(10+10) = 1880
}

#[test]
fn test_sps_display_height_chroma420() {
    let sps = Sps {
        pic_height_in_luma_samples: 1080,
        chroma_format_idc: ChromaFormat::Chroma420,
        conf_win_top_offset: 0,
        conf_win_bottom_offset: 0,
        ..default_sps()
    };

    assert_eq!(sps.display_height(), 1080);
}

#[test]
fn test_sps_display_height_with_conformance_window() {
    let sps = Sps {
        pic_height_in_luma_samples: 1080,
        chroma_format_idc: ChromaFormat::Chroma420,
        conf_win_top_offset: 5,
        conf_win_bottom_offset: 5,
        ..default_sps()
    };

    assert_eq!(sps.display_height(), 1060); // 1080 - 2*(5+5) = 1060
}

#[test]
fn test_sps_display_width_chroma444() {
    let sps = Sps {
        pic_width_in_luma_samples: 1920,
        chroma_format_idc: ChromaFormat::Chroma444,
        conf_win_left_offset: 10,
        conf_win_right_offset: 10,
        ..default_sps()
    };

    assert_eq!(sps.display_width(), 1900); // 1920 - 1*(10+10) = 1900
}

#[test]
fn test_sps_max_poc_lsb() {
    let sps = Sps {
        log2_max_pic_order_cnt_lsb_minus4: 0,
        ..default_sps()
    };

    assert_eq!(sps.max_poc_lsb(), 16); // 1 << (0 + 4) = 16

    let sps = Sps {
        log2_max_pic_order_cnt_lsb_minus4: 4,
        ..default_sps()
    };

    assert_eq!(sps.max_poc_lsb(), 256); // 1 << (4 + 4) = 256
}

// ============================================================================
// parse_sps() Tests
// ============================================================================

#[test]
fn test_parse_sps_empty() {
    let data: &[u8] = &[];
    let result = parse_sps(data);
    assert!(result.is_err());
}

#[test]
fn test_parse_sps_insufficient_data() {
    let data = &[0x00]; // Only one byte
    let result = parse_sps(data);
    assert!(result.is_err());
}

#[test]
fn test_parse_sps_minimal() {
    // Minimal SPS data - needs proper bitstream construction
    let data = &[0x00, 0x00, 0x00];
    let result = parse_sps(data);
    // May fail due to incomplete data, but shouldn't panic
}

// ============================================================================
// Clone and Debug Tests
// ============================================================================

#[test]
fn test_sps_clone() {
    let sps = Sps {
        sps_video_parameter_set_id: 5,
        pic_width_in_luma_samples: 1920,
        pic_height_in_luma_samples: 1080,
        ..default_sps()
    };

    let cloned = sps.clone();
    assert_eq!(cloned.sps_video_parameter_set_id, 5);
    assert_eq!(cloned.pic_width_in_luma_samples, 1920);
}

#[test]
fn test_profile_debug() {
    let debug_str = format!("{:?}", Profile::Main);
    assert!(debug_str.contains("Main"));
}

#[test]
fn test_chroma_format_debug() {
    let debug_str = format!("{:?}", ChromaFormat::Chroma420);
    assert!(debug_str.contains("Chroma420") || debug_str.contains("420"));
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_sps_all_false_flags() {
    let sps = Sps {
        sps_temporal_id_nesting_flag: false,
        separate_colour_plane_flag: false,
        conformance_window_flag: false,
        sps_sub_layer_ordering_info_present_flag: false,
        scaling_list_enabled_flag: false,
        amp_enabled_flag: false,
        sample_adaptive_offset_enabled_flag: false,
        pcm_enabled_flag: false,
        long_term_ref_pics_present_flag: false,
        sps_temporal_mvp_enabled_flag: false,
        strong_intra_smoothing_enabled_flag: false,
        vui_parameters_present_flag: false,
        ..default_sps()
    };

    assert!(!sps.sps_temporal_id_nesting_flag);
    assert!(!sps.scaling_list_enabled_flag);
}

#[test]
fn test_sps_all_true_flags() {
    let sps = Sps {
        sps_temporal_id_nesting_flag: true,
        separate_colour_plane_flag: true,
        conformance_window_flag: true,
        sps_sub_layer_ordering_info_present_flag: true,
        scaling_list_enabled_flag: true,
        amp_enabled_flag: true,
        sample_adaptive_offset_enabled_flag: true,
        pcm_enabled_flag: true,
        long_term_ref_pics_present_flag: true,
        sps_temporal_mvp_enabled_flag: true,
        strong_intra_smoothing_enabled_flag: true,
        vui_parameters_present_flag: true,
        ..default_sps()
    };

    assert!(sps.sps_temporal_id_nesting_flag);
    assert!(sps.scaling_list_enabled_flag);
    assert!(sps.vui_parameters_present_flag);
}

#[test]
fn test_sps_various_chroma_formats() {
    let sps_420 = Sps {
        chroma_format_idc: ChromaFormat::Chroma420,
        ..default_sps()
    };
    assert_eq!(sps_420.chroma_format_idc, ChromaFormat::Chroma420);

    let sps_422 = Sps {
        chroma_format_idc: ChromaFormat::Chroma422,
        ..default_sps()
    };
    assert_eq!(sps_422.chroma_format_idc, ChromaFormat::Chroma422);

    let sps_444 = Sps {
        chroma_format_idc: ChromaFormat::Chroma444,
        ..default_sps()
    };
    assert_eq!(sps_444.chroma_format_idc, ChromaFormat::Chroma444);
}

#[test]
fn test_sps_various_bit_depths() {
    for bit_depth_minus8 in 0..=6u8 {
        let sps = Sps {
            bit_depth_luma_minus8: bit_depth_minus8,
            bit_depth_chroma_minus8: bit_depth_minus8,
            ..default_sps()
        };

        assert_eq!(sps.bit_depth_luma(), bit_depth_minus8 + 8);
        assert_eq!(sps.bit_depth_chroma(), bit_depth_minus8 + 8);
    }
}

// ============================================================================
// Helper Function
// ============================================================================

fn default_sps() -> Sps {
    Sps {
        sps_video_parameter_set_id: 0,
        sps_max_sub_layers_minus1: 0,
        sps_temporal_id_nesting_flag: false,
        profile_tier_level: ProfileTierLevel {
            general_profile_space: 0,
            general_tier_flag: false,
            general_profile_idc: Profile::Main,
            general_profile_compatibility_flags: 0,
            general_progressive_source_flag: false,
            general_interlaced_source_flag: false,
            general_non_packed_constraint_flag: false,
            general_frame_only_constraint_flag: false,
            general_level_idc: 0,
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
        log2_max_pic_order_cnt_lsb_minus4: 0,
        sps_sub_layer_ordering_info_present_flag: false,
        sps_max_dec_pic_buffering_minus1: vec![],
        sps_max_num_reorder_pics: vec![],
        sps_max_latency_increase_plus1: vec![],
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
