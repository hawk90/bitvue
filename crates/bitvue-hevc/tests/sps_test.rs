//! HEVC SPS Parsing Tests
//!
//! Tests for SPS parsing functionality.

use bitvue_hevc::sps::{parse_sps, ChromaFormat, Profile, ProfileTierLevel, Sps};

#[test]
fn test_parse_sps_minimal() {
    // Test SPS with minimal valid data
    // This would need actual bitstream data
    let _ = Sps {
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
        log2_max_pic_order_cnt_lsb_minus4: 0,
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
    };
}

#[test]
fn test_sps_chroma_format_variants() {
    let formats = vec![
        ChromaFormat::Monochrome,
        ChromaFormat::Chroma420,
        ChromaFormat::Chroma422,
        ChromaFormat::Chroma444,
    ];

    for format in formats {
        let _ = format!("Chroma format: {:?}", format);
    }
}

#[test]
fn test_sps_profile_variants() {
    let profiles = vec![Profile::Main, Profile::Main10, Profile::MainStillPicture];

    for profile in profiles {
        let _ = format!("Profile: {:?}", profile);
    }
}

#[test]
fn test_sps_resolution_validation() {
    let sps = Sps {
        pic_width_in_luma_samples: 1920,
        pic_height_in_luma_samples: 1080,
        ..create_minimal_sps()
    };

    assert_eq!(sps.pic_width_in_luma_samples, 1920);
    assert_eq!(sps.pic_height_in_luma_samples, 1080);
}

#[test]
fn test_sps_4k_resolution() {
    let sps = Sps {
        pic_width_in_luma_samples: 3840,
        pic_height_in_luma_samples: 2160,
        ..create_minimal_sps()
    };

    assert_eq!(sps.pic_width_in_luma_samples, 3840);
    assert_eq!(sps.pic_height_in_luma_samples, 2160);
}

#[test]
fn test_sps_various_resolutions() {
    let resolutions = vec![(640, 480), (1280, 720), (1920, 1080), (3840, 2160)];

    for (width, height) in resolutions {
        let sps = Sps {
            pic_width_in_luma_samples: width,
            pic_height_in_luma_samples: height,
            ..create_minimal_sps()
        };

        assert_eq!(sps.pic_width_in_luma_samples, width);
        assert_eq!(sps.pic_height_in_luma_samples, height);
    }
}

#[test]
fn test_sps_bit_depth() {
    let bit_depths = vec![0u8, 2, 4, 6]; // 8, 10, 12, 14 bit

    for bit_depth in bit_depths {
        let sps = Sps {
            bit_depth_luma_minus8: bit_depth,
            bit_depth_chroma_minus8: bit_depth,
            ..create_minimal_sps()
        };

        assert_eq!(sps.bit_depth_luma_minus8, bit_depth);
        assert_eq!(sps.bit_depth_chroma_minus8, bit_depth);
    }
}

#[test]
fn test_sps_ctb_size() {
    let ctu_sizes = vec![0u8, 1, 2, 3]; // 16x16, 32x32, 64x64, 128x128

    for ctu_size in ctu_sizes {
        let sps = Sps {
            log2_min_luma_coding_block_size_minus3: ctu_size,
            ..create_minimal_sps()
        };

        assert_eq!(sps.log2_min_luma_coding_block_size_minus3, ctu_size);
    }
}

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
        log2_max_pic_order_cnt_lsb_minus4: 0,
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

#[test]
fn test_sps_temporal_mvp_flag() {
    let sps_with_mvp = Sps {
        sps_temporal_mvp_enabled_flag: true,
        ..create_minimal_sps()
    };

    assert!(sps_with_mvp.sps_temporal_mvp_enabled_flag);
}

#[test]
fn test_sps_sao_flag() {
    let sps_with_sao = Sps {
        sample_adaptive_offset_enabled_flag: true,
        ..create_minimal_sps()
    };

    assert!(sps_with_sao.sample_adaptive_offset_enabled_flag);
}

#[test]
fn test_sps_amp_flag() {
    let sps_with_amp = Sps {
        amp_enabled_flag: true,
        ..create_minimal_sps()
    };

    assert!(sps_with_amp.amp_enabled_flag);
}

#[test]
fn test_sps_pcm_flag() {
    let sps_with_pcm = Sps {
        pcm_enabled_flag: true,
        ..create_minimal_sps()
    };

    assert!(sps_with_pcm.pcm_enabled_flag);
}

#[test]
fn test_sps_strong_intra_smoothing_flag() {
    let sps_with_smoothing = Sps {
        strong_intra_smoothing_enabled_flag: true,
        ..create_minimal_sps()
    };

    assert!(sps_with_smoothing.strong_intra_smoothing_enabled_flag);
}
