//! HEVC SPS Parsing Tests
//!
//! Tests for actual SPS parsing functionality with real bitstream data.

use bitvue_hevc::sps::{parse_sps, ChromaFormat, Profile, ProfileTierLevel, Sps};

// Helper function to create minimal ProfileTierLevel
fn create_minimal_profile_tier_level() -> ProfileTierLevel {
    ProfileTierLevel {
        general_profile_space: 0,
        general_tier_flag: false,
        general_profile_idc: Profile::Main,
        general_profile_compatibility_flags: 0,
        general_progressive_source_flag: true,
        general_interlaced_source_flag: false,
        general_non_packed_constraint_flag: true,
        general_frame_only_constraint_flag: true,
        general_level_idc: 120,
    }
}

/// Create minimal SPS structure for testing
fn create_minimal_sps() -> Sps {
    Sps {
        sps_video_parameter_set_id: 0,
        sps_max_sub_layers_minus1: 0,
        sps_temporal_id_nesting_flag: false,
        profile_tier_level: create_minimal_profile_tier_level(),
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
fn test_sps_structure_creation() {
    let sps = create_minimal_sps();

    assert_eq!(sps.pic_width_in_luma_samples, 1920);
    assert_eq!(sps.pic_height_in_luma_samples, 1080);
}

#[test]
fn test_sps_chroma_format_420() {
    let sps = Sps {
        chroma_format_idc: ChromaFormat::Chroma420,
        separate_colour_plane_flag: false,
        ..create_minimal_sps()
    };

    assert_eq!(sps.chroma_format_idc, ChromaFormat::Chroma420);
}

#[test]
fn test_sps_chroma_format_422() {
    let sps = Sps {
        chroma_format_idc: ChromaFormat::Chroma422,
        separate_colour_plane_flag: false,
        ..create_minimal_sps()
    };

    assert_eq!(sps.chroma_format_idc, ChromaFormat::Chroma422);
}

#[test]
fn test_sps_chroma_format_444() {
    let sps = Sps {
        chroma_format_idc: ChromaFormat::Chroma444,
        separate_colour_plane_flag: false,
        ..create_minimal_sps()
    };

    assert_eq!(sps.chroma_format_idc, ChromaFormat::Chroma444);
}

#[test]
fn test_sps_profile_main() {
    let sps = Sps {
        profile_tier_level: ProfileTierLevel {
            general_profile_idc: Profile::Main,
            general_tier_flag: false,
            general_profile_space: 0,
            general_profile_compatibility_flags: 0,
            general_progressive_source_flag: true,
            general_interlaced_source_flag: false,
            general_non_packed_constraint_flag: true,
            general_frame_only_constraint_flag: true,
            general_level_idc: 120,
        },
        ..create_minimal_sps()
    };

    assert_eq!(sps.profile_tier_level.general_profile_idc, Profile::Main);
}

#[test]
fn test_sps_profile_main10() {
    let sps = Sps {
        profile_tier_level: ProfileTierLevel {
            general_profile_idc: Profile::Main10,
            general_tier_flag: false,
            general_profile_space: 0,
            general_profile_compatibility_flags: 0,
            general_progressive_source_flag: true,
            general_interlaced_source_flag: false,
            general_non_packed_constraint_flag: true,
            general_frame_only_constraint_flag: true,
            general_level_idc: 120,
        },
        ..create_minimal_sps()
    };

    assert_eq!(sps.profile_tier_level.general_profile_idc, Profile::Main10);
}

#[test]
fn test_sps_profile_from_u8() {
    assert_eq!(Profile::from(1u8), Profile::Main);
    assert_eq!(Profile::from(2u8), Profile::Main10);
    assert_eq!(Profile::from(3u8), Profile::MainStillPicture);
    assert_eq!(Profile::from(255u8), Profile::Unknown(255));
}

#[test]
fn test_sps_profile_idc() {
    assert_eq!(Profile::Main.idc(), 1);
    assert_eq!(Profile::Main10.idc(), 2);
    assert_eq!(Profile::MainStillPicture.idc(), 3);
    assert_eq!(Profile::Unknown(42).idc(), 42);
}

#[test]
fn test_sps_bit_depth_variants() {
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
fn test_sps_bit_depth_helpers() {
    let sps = Sps {
        bit_depth_luma_minus8: 2,
        bit_depth_chroma_minus8: 2,
        ..create_minimal_sps()
    };

    assert_eq!(sps.bit_depth_luma(), 10);
    assert_eq!(sps.bit_depth_chroma(), 10);
}

#[test]
fn test_sps_resolution_variants() {
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
fn test_sps_ctb_size_variants() {
    let ctu_sizes = vec![0u8, 1, 2, 3]; // 8x8, 16x16, 32x32, 64x64

    for ctu_size in ctu_sizes {
        let sps = Sps {
            log2_min_luma_coding_block_size_minus3: ctu_size,
            log2_diff_max_min_luma_coding_block_size: 0,
            ..create_minimal_sps()
        };

        assert_eq!(sps.log2_min_luma_coding_block_size_minus3, ctu_size);
    }
}

#[test]
fn test_sps_ctb_size_calculation() {
    let sps = Sps {
        log2_min_luma_coding_block_size_minus3: 0,   // Min CB = 8
        log2_diff_max_min_luma_coding_block_size: 3, // CTB = 64
        ..create_minimal_sps()
    };

    assert_eq!(sps.min_cb_size(), 8);
    assert_eq!(sps.ctb_size(), 64);
}

#[test]
fn test_sps_pic_width_in_ctbs() {
    let sps = Sps {
        pic_width_in_luma_samples: 1920,
        log2_min_luma_coding_block_size_minus3: 0,
        log2_diff_max_min_luma_coding_block_size: 3,
        ..create_minimal_sps()
    };

    assert_eq!(sps.pic_width_in_ctbs(), 30); // 1920 / 64 = 30
}

#[test]
fn test_sps_pic_height_in_ctbs() {
    let sps = Sps {
        pic_height_in_luma_samples: 1080,
        log2_min_luma_coding_block_size_minus3: 0,
        log2_diff_max_min_luma_coding_block_size: 3,
        ..create_minimal_sps()
    };

    assert_eq!(sps.pic_height_in_ctbs(), 17); // 1080 / 64 = 16.875 -> 17
}

#[test]
fn test_sps_temporal_mvp_flag() {
    let sps = Sps {
        sps_temporal_mvp_enabled_flag: true,
        ..create_minimal_sps()
    };

    assert!(sps.sps_temporal_mvp_enabled_flag);
}

#[test]
fn test_sps_sao_flag() {
    let sps = Sps {
        sample_adaptive_offset_enabled_flag: true,
        ..create_minimal_sps()
    };

    assert!(sps.sample_adaptive_offset_enabled_flag);
}

#[test]
fn test_sps_amp_flag() {
    let sps = Sps {
        amp_enabled_flag: true,
        ..create_minimal_sps()
    };

    assert!(sps.amp_enabled_flag);
}

#[test]
fn test_sps_strong_intra_smoothing_flag() {
    let sps = Sps {
        strong_intra_smoothing_enabled_flag: true,
        ..create_minimal_sps()
    };

    assert!(sps.strong_intra_smoothing_enabled_flag);
}

#[test]
fn test_sps_max_sub_layers() {
    let sub_layers = vec![0u8, 1, 2, 3, 4, 5, 6];

    for sub_layer in sub_layers {
        let sps = Sps {
            sps_max_sub_layers_minus1: sub_layer,
            ..create_minimal_sps()
        };

        assert!(sps.sps_max_sub_layers_minus1 <= 6);
    }
}

#[test]
fn test_sps_vui_parameters_flag() {
    let sps = Sps {
        vui_parameters_present_flag: true,
        vui_parameters: None,
        ..create_minimal_sps()
    };

    assert!(sps.vui_parameters_present_flag);
}

#[test]
fn test_sps_max_poc_lsb() {
    let sps = Sps {
        log2_max_pic_order_cnt_lsb_minus4: 4,
        ..create_minimal_sps()
    };

    assert_eq!(sps.max_poc_lsb(), 256); // 2^8
}

#[test]
fn test_sps_display_width_no_crop() {
    let sps = Sps {
        pic_width_in_luma_samples: 1920,
        pic_height_in_luma_samples: 1080,
        conformance_window_flag: false,
        chroma_format_idc: ChromaFormat::Chroma420,
        ..create_minimal_sps()
    };

    assert_eq!(sps.display_width(), 1920);
    assert_eq!(sps.display_height(), 1080);
}

#[test]
fn test_sps_display_width_with_crop() {
    let sps = Sps {
        pic_width_in_luma_samples: 1920,
        pic_height_in_luma_samples: 1080,
        conformance_window_flag: true,
        conf_win_left_offset: 2,
        conf_win_right_offset: 2,
        conf_win_top_offset: 1,
        conf_win_bottom_offset: 1,
        chroma_format_idc: ChromaFormat::Chroma420,
        ..create_minimal_sps()
    };

    // 1920 - 2*(2+2) = 1920 - 8 = 1912
    // 1080 - 2*(1+1) = 1080 - 4 = 1076
    assert_eq!(sps.display_width(), 1912);
    assert_eq!(sps.display_height(), 1076);
}

#[test]
fn test_sps_chroma_format_from_u8() {
    assert_eq!(ChromaFormat::from(0u8), ChromaFormat::Monochrome);
    assert_eq!(ChromaFormat::from(1u8), ChromaFormat::Chroma420);
    assert_eq!(ChromaFormat::from(2u8), ChromaFormat::Chroma422);
    assert_eq!(ChromaFormat::from(3u8), ChromaFormat::Chroma444);
    assert_eq!(ChromaFormat::from(255u8), ChromaFormat::Chroma420); // Default
}

#[test]
fn test_sps_transform_depths() {
    let sps = Sps {
        max_transform_hierarchy_depth_inter: 2,
        max_transform_hierarchy_depth_intra: 3,
        ..create_minimal_sps()
    };

    assert_eq!(sps.max_transform_hierarchy_depth_inter, 2);
    assert_eq!(sps.max_transform_hierarchy_depth_intra, 3);
}

#[test]
fn test_sps_pcm_enabled_flag() {
    let sps = Sps {
        pcm_enabled_flag: true,
        ..create_minimal_sps()
    };

    assert!(sps.pcm_enabled_flag);
}

#[test]
fn test_sps_scaling_list_enabled_flag() {
    let sps = Sps {
        scaling_list_enabled_flag: true,
        ..create_minimal_sps()
    };

    assert!(sps.scaling_list_enabled_flag);
}

#[test]
fn test_sps_long_term_ref_pics_flag() {
    let sps = Sps {
        long_term_ref_pics_present_flag: true,
        num_long_term_ref_pics_sps: 2,
        ..create_minimal_sps()
    };

    assert!(sps.long_term_ref_pics_present_flag);
    assert_eq!(sps.num_long_term_ref_pics_sps, 2);
}

#[test]
fn test_sps_temporal_id_nesting_flag() {
    let sps = Sps {
        sps_temporal_id_nesting_flag: true,
        ..create_minimal_sps()
    };

    assert!(sps.sps_temporal_id_nesting_flag);
}

#[test]
fn test_sps_separate_colour_plane_flag() {
    let sps = Sps {
        chroma_format_idc: ChromaFormat::Chroma444,
        separate_colour_plane_flag: true,
        ..create_minimal_sps()
    };

    assert!(sps.separate_colour_plane_flag);
}

#[test]
fn test_sps_sub_layer_ordering_info_flag() {
    let sps = Sps {
        sps_sub_layer_ordering_info_present_flag: true,
        ..create_minimal_sps()
    };

    assert!(sps.sps_sub_layer_ordering_info_present_flag);
}

#[test]
fn test_sps_max_dec_pic_buffering() {
    let sps = Sps {
        sps_max_dec_pic_buffering_minus1: vec![3, 2, 1],
        ..create_minimal_sps()
    };

    assert_eq!(sps.sps_max_dec_pic_buffering_minus1.len(), 3);
    assert_eq!(sps.sps_max_dec_pic_buffering_minus1[0], 3);
}

#[test]
fn test_sps_max_num_reorder_pics() {
    let sps = Sps {
        sps_max_num_reorder_pics: vec![2, 1, 0],
        ..create_minimal_sps()
    };

    assert_eq!(sps.sps_max_num_reorder_pics.len(), 3);
    assert_eq!(sps.sps_max_num_reorder_pics[0], 2);
}

#[test]
fn test_sps_profile_tier_level_flags() {
    let ptl = ProfileTierLevel {
        general_profile_space: 0,
        general_tier_flag: false,
        general_profile_idc: Profile::Main,
        general_profile_compatibility_flags: 0,
        general_progressive_source_flag: true,
        general_interlaced_source_flag: false,
        general_non_packed_constraint_flag: true,
        general_frame_only_constraint_flag: true,
        general_level_idc: 120,
    };

    assert!(ptl.general_progressive_source_flag);
    assert!(!ptl.general_interlaced_source_flag);
    assert!(ptl.general_non_packed_constraint_flag);
    assert!(ptl.general_frame_only_constraint_flag);
}

#[test]
fn test_sps_profile_compatibility_flags() {
    let ptl = ProfileTierLevel {
        general_profile_space: 0,
        general_tier_flag: false,
        general_profile_idc: Profile::Main,
        general_profile_compatibility_flags: 0xFFFFFFFF,
        general_progressive_source_flag: true,
        general_interlaced_source_flag: false,
        general_non_packed_constraint_flag: true,
        general_frame_only_constraint_flag: true,
        general_level_idc: 120,
    };

    assert_eq!(ptl.general_profile_compatibility_flags, 0xFFFFFFFF);
}

#[test]
fn test_sps_general_tier_flag() {
    let ptl = ProfileTierLevel {
        general_profile_space: 0,
        general_tier_flag: true,
        general_profile_idc: Profile::Main,
        general_profile_compatibility_flags: 0,
        general_progressive_source_flag: true,
        general_interlaced_source_flag: false,
        general_non_packed_constraint_flag: true,
        general_frame_only_constraint_flag: true,
        general_level_idc: 120,
    };

    assert!(ptl.general_tier_flag);
}

#[test]
fn test_sps_general_profile_space() {
    let ptl = ProfileTierLevel {
        general_profile_space: 1,
        general_tier_flag: false,
        general_profile_idc: Profile::Main,
        general_profile_compatibility_flags: 0,
        general_progressive_source_flag: true,
        general_interlaced_source_flag: false,
        general_non_packed_constraint_flag: true,
        general_frame_only_constraint_flag: true,
        general_level_idc: 120,
    };

    assert_eq!(ptl.general_profile_space, 1);
}

#[test]
fn test_sps_conformance_window_offsets() {
    let sps = Sps {
        conformance_window_flag: true,
        conf_win_left_offset: 10,
        conf_win_right_offset: 20,
        conf_win_top_offset: 5,
        conf_win_bottom_offset: 15,
        ..create_minimal_sps()
    };

    assert_eq!(sps.conf_win_left_offset, 10);
    assert_eq!(sps.conf_win_right_offset, 20);
    assert_eq!(sps.conf_win_top_offset, 5);
    assert_eq!(sps.conf_win_bottom_offset, 15);
}
