#![allow(dead_code)]
//! Comprehensive tests for VVC SPS module
//!
//! Tests Profile, ChromaFormat, ProfileTierLevel, DualTreeConfig,
//! AlfConfig, LmcsConfig, and Sps

use bitvue_vvc::sps::{
    AlfConfig, ChromaFormat, DualTreeConfig, LmcsConfig, Profile, ProfileTierLevel, Sps,
};

// ============================================================================
// Profile Tests
// ============================================================================

#[test]
fn test_profile_main10() {
    let profile = Profile::Main10;
    assert_eq!(profile.idc(), 1);
}

#[test]
fn test_profile_main10_still_picture() {
    let profile = Profile::Main10StillPicture;
    assert_eq!(profile.idc(), 2);
}

#[test]
fn test_profile_main10444() {
    let profile = Profile::Main10444;
    assert_eq!(profile.idc(), 3);
}

#[test]
fn test_profile_main10444_still_picture() {
    let profile = Profile::Main10444StillPicture;
    assert_eq!(profile.idc(), 4);
}

#[test]
fn test_profile_multilayer() {
    let profile = Profile::Multilayer;
    assert_eq!(profile.idc(), 17);
}

#[test]
fn test_profile_multilayer_main10() {
    let profile = Profile::MultilayerMain10;
    assert_eq!(profile.idc(), 65);
}

#[test]
fn test_profile_unknown() {
    let profile = Profile::Unknown(99);
    assert_eq!(profile.idc(), 99);
}

#[test]
fn test_profile_from_u8_valid() {
    assert_eq!(Profile::from(1u8), Profile::Main10);
    assert_eq!(Profile::from(2u8), Profile::Main10StillPicture);
    assert_eq!(Profile::from(3u8), Profile::Main10444);
    assert_eq!(Profile::from(4u8), Profile::Main10444StillPicture);
    assert_eq!(Profile::from(17u8), Profile::Multilayer);
    assert_eq!(Profile::from(65u8), Profile::MultilayerMain10);
}

#[test]
fn test_profile_from_u8_invalid() {
    assert_eq!(Profile::from(0u8), Profile::Unknown(0));
    assert_eq!(Profile::from(10u8), Profile::Unknown(10));
    assert_eq!(Profile::from(100u8), Profile::Unknown(100));
}

#[test]
fn test_profile_copy() {
    let profile = Profile::Main10;
    let copied = profile;
    assert_eq!(copied, profile);
    assert_eq!(copied.idc(), 1);
}

#[test]
fn test_profile_clone() {
    let profile = Profile::Multilayer;
    let cloned = profile.clone();
    assert_eq!(cloned, profile);
}

// ============================================================================
// ChromaFormat Tests
// ============================================================================

#[test]
fn test_chroma_format_monochrome() {
    let format = ChromaFormat::Monochrome;
    assert_eq!(format as u8, 0);
}

#[test]
fn test_chroma_format_420() {
    let format = ChromaFormat::Chroma420;
    assert_eq!(format as u8, 1);
}

#[test]
fn test_chroma_format_422() {
    let format = ChromaFormat::Chroma422;
    assert_eq!(format as u8, 2);
}

#[test]
fn test_chroma_format_444() {
    let format = ChromaFormat::Chroma444;
    assert_eq!(format as u8, 3);
}

#[test]
fn test_chroma_format_from_u8_valid() {
    assert_eq!(ChromaFormat::from(0u8), ChromaFormat::Monochrome);
    assert_eq!(ChromaFormat::from(1u8), ChromaFormat::Chroma420);
    assert_eq!(ChromaFormat::from(2u8), ChromaFormat::Chroma422);
    assert_eq!(ChromaFormat::from(3u8), ChromaFormat::Chroma444);
}

#[test]
fn test_chroma_format_from_u8_invalid() {
    // Invalid values should default to Chroma420
    assert_eq!(ChromaFormat::from(5u8), ChromaFormat::Chroma420);
    assert_eq!(ChromaFormat::from(255u8), ChromaFormat::Chroma420);
}

// ============================================================================
// ProfileTierLevel Tests
// ============================================================================

#[test]
fn test_profile_tier_level_default() {
    let ptl = ProfileTierLevel::default();
    assert_eq!(ptl.general_profile_idc, Profile::Main10);
    assert!(!ptl.general_tier_flag);
    assert_eq!(ptl.general_level_idc, 0);
    assert!(ptl.ptl_frame_only_constraint_flag);
    assert!(!ptl.ptl_multilayer_enabled_flag);
}

#[test]
fn test_profile_tier_level_clone() {
    let mut ptl = ProfileTierLevel::default();
    ptl.general_level_idc = 51;
    ptl.general_tier_flag = true;

    let cloned = ptl.clone();
    assert_eq!(cloned.general_level_idc, 51);
    assert_eq!(cloned.general_tier_flag, true);
}

// ============================================================================
// DualTreeConfig Tests
// ============================================================================

#[test]
fn test_dual_tree_config_default() {
    let config = DualTreeConfig::default();
    assert!(!config.qtbtt_dual_tree_intra_flag);
    assert_eq!(config.max_mtt_hierarchy_depth_intra_slice_luma, 0);
    assert_eq!(config.max_mtt_hierarchy_depth_intra_slice_chroma, 0);
    assert_eq!(config.max_mtt_hierarchy_depth_inter_slice, 0);
}

#[test]
fn test_dual_tree_config_enabled() {
    let config = DualTreeConfig {
        qtbtt_dual_tree_intra_flag: true,
        max_mtt_hierarchy_depth_intra_slice_luma: 3,
        max_mtt_hierarchy_depth_intra_slice_chroma: 2,
        max_mtt_hierarchy_depth_inter_slice: 4,
    };
    assert!(config.qtbtt_dual_tree_intra_flag);
    assert_eq!(config.max_mtt_hierarchy_depth_intra_slice_luma, 3);
}

#[test]
fn test_dual_tree_config_clone() {
    let config = DualTreeConfig {
        qtbtt_dual_tree_intra_flag: true,
        ..Default::default()
    };
    let cloned = config.clone();
    assert_eq!(
        cloned.qtbtt_dual_tree_intra_flag,
        config.qtbtt_dual_tree_intra_flag
    );
}

// ============================================================================
// AlfConfig Tests
// ============================================================================

#[test]
fn test_alf_config_default() {
    let config = AlfConfig::default();
    assert!(!config.alf_enabled_flag);
    assert!(!config.ccalf_enabled_flag);
}

#[test]
fn test_alf_config_enabled() {
    let config = AlfConfig {
        alf_enabled_flag: true,
        ccalf_enabled_flag: true,
    };
    assert!(config.alf_enabled_flag);
    assert!(config.ccalf_enabled_flag);
}

#[test]
fn test_alf_config_clone() {
    let config = AlfConfig {
        alf_enabled_flag: true,
        ..Default::default()
    };
    let cloned = config.clone();
    assert_eq!(cloned.alf_enabled_flag, config.alf_enabled_flag);
}

// ============================================================================
// LmcsConfig Tests
// ============================================================================

#[test]
fn test_lmcs_config_default() {
    let config = LmcsConfig::default();
    assert!(!config.lmcs_enabled_flag);
}

#[test]
fn test_lmcs_config_enabled() {
    let config = LmcsConfig {
        lmcs_enabled_flag: true,
    };
    assert!(config.lmcs_enabled_flag);
}

#[test]
fn test_lmcs_config_clone() {
    let config = LmcsConfig {
        lmcs_enabled_flag: true,
    };
    let cloned = config.clone();
    assert_eq!(cloned.lmcs_enabled_flag, config.lmcs_enabled_flag);
}

// ============================================================================
// Sps Tests
// ============================================================================

#[test]
fn test_sps_default() {
    let sps = Sps::default();
    assert_eq!(sps.sps_seq_parameter_set_id, 0);
    assert_eq!(sps.sps_video_parameter_set_id, 0);
    assert_eq!(sps.sps_max_sublayers_minus1, 0);
    assert_eq!(sps.sps_chroma_format_idc, ChromaFormat::Chroma420);
    assert_eq!(sps.sps_log2_ctu_size_minus5, 2);
    assert!(!sps.sps_subpic_info_present_flag);
    assert_eq!(sps.sps_pic_width_max_in_luma_samples, 0);
    assert_eq!(sps.sps_pic_height_max_in_luma_samples, 0);
}

#[test]
fn test_sps_bit_depth() {
    let mut sps = Sps::default();
    sps.sps_bitdepth_minus8 = 2;
    assert_eq!(sps.bit_depth(), 10); // 8 + 2

    sps.sps_bitdepth_minus8 = 4;
    assert_eq!(sps.bit_depth(), 12); // 8 + 4
}

#[test]
fn test_sps_ctu_size() {
    let mut sps = Sps::default();
    sps.sps_log2_ctu_size_minus5 = 2;
    assert_eq!(sps.ctu_size(), 128); // 2^(2+5) = 128

    sps.sps_log2_ctu_size_minus5 = 3;
    assert_eq!(sps.ctu_size(), 256); // 2^(3+5) = 256
}

#[test]
fn test_sps_min_cb_size() {
    let mut sps = Sps::default();
    sps.sps_log2_min_luma_coding_block_size_minus2 = 0;
    assert_eq!(sps.min_cb_size(), 4); // 2^(0+2) = 4

    sps.sps_log2_min_luma_coding_block_size_minus2 = 1;
    assert_eq!(sps.min_cb_size(), 8); // 2^(1+2) = 8
}

#[test]
fn test_sps_pic_width_in_ctus() {
    let mut sps = Sps::default();
    sps.sps_log2_ctu_size_minus5 = 2; // CTU size = 128
    sps.sps_pic_width_max_in_luma_samples = 1920;

    // (1920 + 128 - 1) / 128 = 2047 / 128 = 15.99... = 15
    assert_eq!(sps.pic_width_in_ctus(), 15);
}

#[test]
fn test_sps_pic_height_in_ctus() {
    let mut sps = Sps::default();
    sps.sps_log2_ctu_size_minus5 = 2; // CTU size = 128
    sps.sps_pic_height_max_in_luma_samples = 1080;

    // (1080 + 128 - 1) / 128 = 1207 / 128 = 9.43... = 9
    assert_eq!(sps.pic_height_in_ctus(), 9);
}

#[test]
fn test_sps_max_poc_lsb() {
    let mut sps = Sps::default();
    sps.sps_log2_max_pic_order_cnt_lsb_minus4 = 4;
    assert_eq!(sps.max_poc_lsb(), 256); // 2^(4+4) = 256
}

#[test]
fn test_sps_has_dual_tree_intra() {
    let mut sps = Sps::default();
    assert!(!sps.has_dual_tree_intra());

    sps.dual_tree.qtbtt_dual_tree_intra_flag = true;
    assert!(sps.has_dual_tree_intra());
}

#[test]
fn test_sps_profile_name_main10() {
    let mut sps = Sps::default();
    sps.profile_tier_level.general_profile_idc = Profile::Main10;
    assert_eq!(sps.profile_name(), "Main 10");
}

#[test]
fn test_sps_profile_name_main10444() {
    let mut sps = Sps::default();
    sps.profile_tier_level.general_profile_idc = Profile::Main10444;
    assert_eq!(sps.profile_name(), "Main 10 4:4:4");
}

#[test]
fn test_sps_profile_name_unknown() {
    let mut sps = Sps::default();
    sps.profile_tier_level.general_profile_idc = Profile::Unknown(99);
    assert_eq!(sps.profile_name(), "Unknown");
}

#[test]
fn test_sps_tier_name_main() {
    let mut sps = Sps::default();
    sps.profile_tier_level.general_tier_flag = false;
    assert_eq!(sps.tier_name(), "Main");
}

#[test]
fn test_sps_tier_name_high() {
    let mut sps = Sps::default();
    sps.profile_tier_level.general_tier_flag = true;
    assert_eq!(sps.tier_name(), "High");
}

#[test]
fn test_sps_level() {
    let mut sps = Sps::default();
    sps.profile_tier_level.general_level_idc = 51; // Level 3.2
    assert_eq!(sps.level(), 51.0 / 16.0); // 3.1875
}

#[test]
fn test_sps_display_width_420() {
    let mut sps = Sps::default();
    sps.sps_pic_width_max_in_luma_samples = 1920;
    sps.sps_chroma_format_idc = ChromaFormat::Chroma420;
    sps.sps_conf_win_left_offset = 0;
    sps.sps_conf_win_right_offset = 0;

    // 1920 - 2 * (0 + 0) = 1920
    assert_eq!(sps.display_width(), 1920);
}

#[test]
fn test_sps_display_height_420() {
    let mut sps = Sps::default();
    sps.sps_pic_height_max_in_luma_samples = 1080;
    sps.sps_chroma_format_idc = ChromaFormat::Chroma420;
    sps.sps_conf_win_top_offset = 0;
    sps.sps_conf_win_bottom_offset = 0;

    // 1080 - 2 * (0 + 0) = 1080
    assert_eq!(sps.display_height(), 1080);
}

#[test]
fn test_sps_display_width_422() {
    let mut sps = Sps::default();
    sps.sps_pic_width_max_in_luma_samples = 1920;
    sps.sps_chroma_format_idc = ChromaFormat::Chroma422;
    sps.sps_conf_win_left_offset = 0;
    sps.sps_conf_win_right_offset = 0;

    // 1920 - 1 * (0 + 0) = 1920
    assert_eq!(sps.display_width(), 1920);
}

#[test]
fn test_sps_display_width_444() {
    let mut sps = Sps::default();
    sps.sps_pic_width_max_in_luma_samples = 1920;
    sps.sps_chroma_format_idc = ChromaFormat::Chroma444;
    sps.sps_conf_win_left_offset = 0;
    sps.sps_conf_win_right_offset = 0;

    // 1920 - 1 * (0 + 0) = 1920
    assert_eq!(sps.display_width(), 1920);
}

#[test]
fn test_sps_clone() {
    let mut sps = Sps::default();
    sps.sps_pic_width_max_in_luma_samples = 1920;
    sps.sps_pic_height_max_in_luma_samples = 1080;

    let cloned = sps.clone();
    assert_eq!(cloned.sps_pic_width_max_in_luma_samples, 1920);
    assert_eq!(cloned.sps_pic_height_max_in_luma_samples, 1080);
}

#[test]
fn test_sps_various_flags() {
    let sps = Sps::default();
    assert!(!sps.sps_subpic_info_present_flag);
    assert!(!sps.sps_conformance_window_flag);
    assert!(!sps.sps_poc_msb_cycle_flag);
    assert!(!sps.sps_gdr_enabled_flag);
    assert!(!sps.sps_ref_pic_resampling_enabled_flag);
    assert!(sps.sps_sao_enabled_flag);
    assert!(sps.sps_temporal_mvp_enabled_flag);
}
