#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(unused_comparisons)]
#![allow(unused_doc_comments)]
//! AVC SPS/PPS/Slice Parsing Tests
//!
//! Tests for actual AVC parameter set and slice parsing functionality.

use bitvue_avc::{
    pps::Pps,
    slice::{SliceHeader, SliceType},
    sps::{ChromaFormat, ProfileIdc, Sps},
};
use std::fmt::Write;

/// Create minimal SPS structure for testing
fn create_minimal_sps() -> Sps {
    Sps {
        profile_idc: ProfileIdc::High,
        constraint_set0_flag: false,
        constraint_set1_flag: false,
        constraint_set2_flag: false,
        constraint_set3_flag: false,
        constraint_set4_flag: false,
        constraint_set5_flag: false,
        level_idc: 41,
        seq_parameter_set_id: 0,
        chroma_format_idc: ChromaFormat::Yuv420,
        separate_colour_plane_flag: false,
        bit_depth_luma_minus8: 0,
        bit_depth_chroma_minus8: 0,
        qpprime_y_zero_transform_bypass_flag: false,
        seq_scaling_matrix_present_flag: false,
        log2_max_frame_num_minus4: 0,
        pic_order_cnt_type: 0,
        log2_max_pic_order_cnt_lsb_minus4: 16,
        delta_pic_order_always_zero_flag: true,
        offset_for_non_ref_pic: 0,
        offset_for_top_to_bottom_field: 0,
        num_ref_frames_in_pic_order_cnt_cycle: 0,
        offset_for_ref_frame: vec![],
        max_num_ref_frames: 0,
        gaps_in_frame_num_value_allowed_flag: false,
        pic_width_in_mbs_minus1: 119,       // 1920 / 16 - 1
        pic_height_in_map_units_minus1: 67, // 1080 / 16 - 1
        frame_mbs_only_flag: true,
        mb_adaptive_frame_field_flag: false,
        direct_8x8_inference_flag: false,
        frame_cropping_flag: false,
        frame_crop_left_offset: 0,
        frame_crop_right_offset: 0,
        frame_crop_top_offset: 0,
        frame_crop_bottom_offset: 0,
        vui_parameters_present_flag: false,
        vui_parameters: None,
    }
}

/// Create minimal PPS structure for testing
fn create_minimal_pps() -> Pps {
    Pps {
        pic_parameter_set_id: 0,
        seq_parameter_set_id: 0,
        entropy_coding_mode_flag: true, // CABAC
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
    }
}

/// Create minimal slice header for testing
fn create_minimal_slice_header() -> SliceHeader {
    SliceHeader {
        first_mb_in_slice: 0,
        slice_type: SliceType::I,
        pic_parameter_set_id: 0,
        colour_plane_id: 0,
        frame_num: 0,
        field_pic_flag: false,
        bottom_field_flag: false,
        idr_pic_id: 0,
        pic_order_cnt_lsb: 0,
        delta_pic_order_cnt_bottom: 0,
        delta_pic_order_cnt: [0, 0],
        redundant_pic_cnt: 0,
        direct_spatial_mv_pred_flag: false,
        num_ref_idx_active_override_flag: false,
        num_ref_idx_l0_active_minus1: 0,
        num_ref_idx_l1_active_minus1: 0,
        ref_pic_list_modification_flag_l0: false,
        ref_pic_list_modification_flag_l1: false,
        ref_pic_list_modification_l0: Default::default(),
        ref_pic_list_modification_l1: Default::default(),
        dec_ref_pic_marking: Default::default(),
        cabac_init_idc: 0,
        slice_qp_delta: 0,
        sp_for_switch_flag: false,
        slice_qs_delta: 0,
        disable_deblocking_filter_idc: 0,
        slice_alpha_c0_offset_div2: 0,
        slice_beta_offset_div2: 0,
        slice_group_change_cycle: 0,
    }
}

#[test]
fn test_sps_creation() {
    let sps = create_minimal_sps();

    assert_eq!(sps.profile_idc, ProfileIdc::High);
    assert_eq!(sps.seq_parameter_set_id, 0);
    assert_eq!(sps.pic_width_in_mbs_minus1, 119);
    assert_eq!(sps.pic_height_in_map_units_minus1, 67);
}

#[test]
fn test_sps_resolution_calculation() {
    let sps = create_minimal_sps();

    // Calculate actual resolution using helper methods
    let width = sps.pic_width();
    let height = sps.pic_height();

    assert_eq!(width, 1920);
    // pic_height = (2 - frame_mbs_only_flag) * (pic_height_in_map_units_minus1 + 1) * 16
    //            = (2 - 1) * (67 + 1) * 16 = 1088
    assert_eq!(height, 1088);
}

#[test]
fn test_sps_various_profiles() {
    let profiles = vec![ProfileIdc::Baseline, ProfileIdc::Main, ProfileIdc::High];

    for profile in profiles {
        let mut sps = create_minimal_sps();
        sps.profile_idc = profile;
        assert_eq!(sps.profile_idc, profile);
    }
}

#[test]
fn test_sps_profile_is_high() {
    assert!(ProfileIdc::High.is_high_profile());
    assert!(ProfileIdc::High10.is_high_profile());
    assert!(ProfileIdc::High422.is_high_profile());
    assert!(ProfileIdc::High444.is_high_profile());
    assert!(!ProfileIdc::Baseline.is_high_profile());
    assert!(!ProfileIdc::Main.is_high_profile());
}

#[test]
fn test_sps_chroma_formats() {
    let chroma_formats = vec![
        ChromaFormat::Monochrome,
        ChromaFormat::Yuv420,
        ChromaFormat::Yuv422,
        ChromaFormat::Yuv444,
    ];

    for chroma in chroma_formats {
        let mut sps = create_minimal_sps();
        sps.chroma_format_idc = chroma;
        assert_eq!(sps.chroma_format_idc, chroma);
    }
}

#[test]
fn test_sps_chroma_subsampling() {
    assert_eq!(ChromaFormat::Yuv420.sub_width_c(), 2);
    assert_eq!(ChromaFormat::Yuv420.sub_height_c(), 2);
    assert_eq!(ChromaFormat::Yuv422.sub_width_c(), 2);
    assert_eq!(ChromaFormat::Yuv422.sub_height_c(), 1);
    assert_eq!(ChromaFormat::Yuv444.sub_width_c(), 1);
    assert_eq!(ChromaFormat::Yuv444.sub_height_c(), 1);
    assert_eq!(ChromaFormat::Monochrome.sub_width_c(), 0);
    assert_eq!(ChromaFormat::Monochrome.sub_height_c(), 0);
}

#[test]
fn test_sps_bit_depths() {
    let bit_depths = vec![0u8, 2, 4]; // 8, 10, 12 bit

    for bit_depth in bit_depths {
        let mut sps = create_minimal_sps();
        sps.bit_depth_luma_minus8 = bit_depth;
        sps.bit_depth_chroma_minus8 = bit_depth;
        assert_eq!(sps.bit_depth_luma_minus8, bit_depth);
        assert_eq!(sps.bit_depth_chroma_minus8, bit_depth);
    }
}

#[test]
fn test_sps_bit_depth_helpers() {
    let mut sps = create_minimal_sps();
    sps.bit_depth_luma_minus8 = 2; // 10-bit
    sps.bit_depth_chroma_minus8 = 2;

    assert_eq!(sps.bit_depth_luma(), 10);
    assert_eq!(sps.bit_depth_chroma(), 10);
}

#[test]
fn test_sps_display_size_with_crop() {
    let mut sps = create_minimal_sps();
    sps.frame_cropping_flag = true;
    sps.frame_crop_left_offset = 2;
    sps.frame_crop_right_offset = 2;
    sps.frame_crop_top_offset = 1;
    sps.frame_crop_bottom_offset = 1;

    let width = sps.display_width();
    let height = sps.display_height();

    // pic_width = 1920, crop_unit_x = 2 (Yuv420)
    // 1920 - 2*(2+2) = 1920 - 8 = 1912
    // pic_height = 1088, crop_unit_y = 2 * (2 - 1) = 2 (Yuv420 + frame_mbs_only)
    // 1088 - 2*(1+1) = 1088 - 4 = 1084
    assert_eq!(width, 1912);
    assert_eq!(height, 1084);
}

#[test]
fn test_sps_profile_idc_from_u8() {
    assert_eq!(ProfileIdc::from_u8(66), ProfileIdc::Baseline);
    assert_eq!(ProfileIdc::from_u8(77), ProfileIdc::Main);
    assert_eq!(ProfileIdc::from_u8(100), ProfileIdc::High);
    assert_eq!(ProfileIdc::from_u8(110), ProfileIdc::High10);
    assert_eq!(ProfileIdc::from_u8(255), ProfileIdc::Unknown);
}

#[test]
fn test_pps_creation() {
    let pps = create_minimal_pps();

    assert_eq!(pps.pic_parameter_set_id, 0);
    assert_eq!(pps.seq_parameter_set_id, 0);
    assert_eq!(pps.entropy_coding_mode_flag, true);
}

#[test]
fn test_pps_is_cabac() {
    let pps = create_minimal_pps();
    assert!(pps.is_cabac());

    let mut cavlc_pps = create_minimal_pps();
    cavlc_pps.entropy_coding_mode_flag = false;
    assert!(!cavlc_pps.is_cabac());
}

#[test]
fn test_pps_initial_qp() {
    let pps = create_minimal_pps();
    assert_eq!(pps.initial_qp(), 26);

    let mut pps_with_delta = create_minimal_pps();
    pps_with_delta.pic_init_qp_minus26 = 5;
    assert_eq!(pps_with_delta.initial_qp(), 31);
}

#[test]
fn test_pps_qp_values() {
    let qp_values = vec![0i32, 10, -10, 20, -20];

    for qp in qp_values {
        let mut pps = create_minimal_pps();
        pps.pic_init_qp_minus26 = qp;
        assert_eq!(pps.pic_init_qp_minus26, qp);
    }
}

#[test]
fn test_slice_header_i_slice() {
    let slice = create_minimal_slice_header();

    assert_eq!(slice.slice_type, SliceType::I);
    assert!(!slice.field_pic_flag);
    assert!(slice.is_first_slice());
}

#[test]
fn test_slice_header_p_slice() {
    let mut slice = create_minimal_slice_header();
    slice.slice_type = SliceType::P;
    slice.num_ref_idx_l0_active_minus1 = 5;

    assert_eq!(slice.slice_type, SliceType::P);
    assert_eq!(slice.num_ref_idx_l0_active_minus1, 5);
}

#[test]
fn test_slice_header_b_slice() {
    let mut slice = create_minimal_slice_header();
    slice.slice_type = SliceType::B;
    slice.num_ref_idx_l0_active_minus1 = 3;
    slice.num_ref_idx_l1_active_minus1 = 2;
    slice.direct_spatial_mv_pred_flag = true;

    assert_eq!(slice.slice_type, SliceType::B);
    assert!(slice.direct_spatial_mv_pred_flag);
}

#[test]
fn test_slice_header_qp_delta() {
    let qp_deltas = vec![0i8, 10, -5, 20, -26];

    for qp in qp_deltas {
        let mut slice = create_minimal_slice_header();
        slice.slice_qp_delta = qp as i32;
        assert_eq!(slice.slice_qp_delta, qp as i32);
    }
}

#[test]
fn test_slice_header_qp_calculation() {
    let mut slice = create_minimal_slice_header();
    slice.slice_qp_delta = 5;

    let mut pps = create_minimal_pps();
    pps.pic_init_qp_minus26 = 2;

    // QP = 26 + 2 + 5 = 33
    assert_eq!(slice.qp(&pps), 33);
}

#[test]
fn test_slice_header_deblocking() {
    let mut slice = create_minimal_slice_header();
    slice.disable_deblocking_filter_idc = 1;
    slice.slice_alpha_c0_offset_div2 = 2;
    slice.slice_beta_offset_div2 = 3;

    assert_eq!(slice.disable_deblocking_filter_idc, 1);
    assert_eq!(slice.slice_alpha_c0_offset_div2, 2);
    assert_eq!(slice.slice_beta_offset_div2, 3);
}

#[test]
fn test_slice_header_cabac_init() {
    let cabac_init_ids = vec![0u32, 1, 2];

    for cabac_id in cabac_init_ids {
        let mut slice = create_minimal_slice_header();
        slice.cabac_init_idc = cabac_id;
        assert_eq!(slice.cabac_init_idc, cabac_id);
    }
}

#[test]
fn test_slice_type_from_u32() {
    // Test modulo 5 wrapping
    assert_eq!(SliceType::from_u32(0), SliceType::P);
    assert_eq!(SliceType::from_u32(1), SliceType::B);
    assert_eq!(SliceType::from_u32(2), SliceType::I);
    assert_eq!(SliceType::from_u32(3), SliceType::Sp);
    assert_eq!(SliceType::from_u32(4), SliceType::Si);
    assert_eq!(SliceType::from_u32(5), SliceType::P); // Wraps
    assert_eq!(SliceType::from_u32(9), SliceType::Si); // 9 % 5 = 4
    assert_eq!(SliceType::from_u32(255), SliceType::P); // 255 % 5 = 0
}

#[test]
fn test_slice_type_is_intra() {
    assert!(SliceType::I.is_intra());
    assert!(SliceType::Si.is_intra());
    assert!(!SliceType::P.is_intra());
    assert!(!SliceType::B.is_intra());
    assert!(!SliceType::Sp.is_intra());
}

#[test]
fn test_slice_type_is_b() {
    assert!(SliceType::B.is_b());
    assert!(!SliceType::I.is_b());
    assert!(!SliceType::P.is_b());
}

#[test]
fn test_slice_type_is_p() {
    assert!(SliceType::P.is_p());
    assert!(SliceType::Sp.is_p());
    assert!(!SliceType::I.is_p());
    assert!(!SliceType::B.is_p());
}

#[test]
fn test_slice_type_name() {
    assert_eq!(SliceType::I.name(), "I");
    assert_eq!(SliceType::P.name(), "P");
    assert_eq!(SliceType::B.name(), "B");
    assert_eq!(SliceType::Sp.name(), "SP");
    assert_eq!(SliceType::Si.name(), "SI");
}

#[test]
fn test_slice_header_is_first_slice() {
    let slice = create_minimal_slice_header();
    assert!(slice.is_first_slice());
}

#[test]
fn test_slice_header_not_first_slice() {
    let mut slice = create_minimal_slice_header();
    slice.first_mb_in_slice = 10;
    assert!(!slice.is_first_slice());
}

#[test]
fn test_slice_header_frame_num() {
    let frame_nums = vec![0u32, 100, 1000, 0xFFFFFFFF];

    for frame_num in frame_nums {
        let mut slice = create_minimal_slice_header();
        slice.frame_num = frame_num;
        assert_eq!(slice.frame_num, frame_num);
    }
}

#[test]
fn test_slice_header_pic_order_count() {
    let pocs = vec![0u32, 100, 0xFFFFFFFF];

    for poc in pocs {
        let mut slice = create_minimal_slice_header();
        slice.pic_order_cnt_lsb = poc;
        assert_eq!(slice.pic_order_cnt_lsb, poc);
    }
}

#[test]
fn test_sps_various_resolutions() {
    let resolutions = vec![(640, 480), (1280, 720), (1920, 1080), (3840, 2160)];

    for (width, height) in resolutions {
        let mut sps = create_minimal_sps();
        sps.pic_width_in_mbs_minus1 = (width / 16) - 1;
        // For frame_mbs_only_flag=true, pic_height = (height / 16) * 16
        sps.pic_height_in_map_units_minus1 = (height / 16) - 1;

        assert_eq!(sps.pic_width(), width);
        // pic_height = (2 - frame_mbs_only_flag) * (pic_height_in_map_units_minus1 + 1) * 16
        //            = 1 * (height/16) * 16 = height rounded to multiple of 16
        let expected_height = ((height / 16) * 16);
        assert_eq!(sps.pic_height(), expected_height);
    }
}

#[test]
fn test_pps_various_qp_settings() {
    let qp_settings = vec![
        (0, 26),   // QP 26
        (10, 36),  // QP 36
        (-10, 16), // QP 16
        (25, 51),  // QP 51
        (-26, 0),  // QP 0
    ];

    for (delta, expected_qp) in qp_settings {
        let mut pps = create_minimal_pps();
        pps.pic_init_qp_minus26 = delta;
        assert_eq!(pps.initial_qp(), expected_qp);
    }
}

#[test]
fn test_pps_weighted_pred_settings() {
    let mut pps = create_minimal_pps();
    pps.weighted_pred_flag = true;
    pps.weighted_bipred_idc = 1;

    assert!(pps.weighted_pred_flag);
    assert_eq!(pps.weighted_bipred_idc, 1);
}

#[test]
fn test_pps_deblocking_settings() {
    let mut pps = create_minimal_pps();
    pps.deblocking_filter_control_present_flag = true;

    assert!(pps.deblocking_filter_control_present_flag);
}

#[test]
fn test_pps_constrained_intra_pred() {
    let mut pps = create_minimal_pps();
    pps.constrained_intra_pred_flag = true;

    assert!(pps.constrained_intra_pred_flag);
}

#[test]
fn test_pps_transform_8x8_mode() {
    let mut pps = create_minimal_pps();
    pps.transform_8x8_mode_flag = true;

    assert!(pps.transform_8x8_mode_flag);
}

#[test]
fn test_slice_header_ref_pic_modification() {
    let mut slice = create_minimal_slice_header();
    slice.ref_pic_list_modification_flag_l0 = true;
    slice.ref_pic_list_modification_flag_l1 = false;

    assert!(slice.ref_pic_list_modification_flag_l0);
    assert!(!slice.ref_pic_list_modification_flag_l1);
}

#[test]
fn test_slice_header_num_ref_idx_override() {
    let mut slice = create_minimal_slice_header();
    slice.num_ref_idx_active_override_flag = true;
    slice.num_ref_idx_l0_active_minus1 = 5;
    slice.num_ref_idx_l1_active_minus1 = 3;

    assert!(slice.num_ref_idx_active_override_flag);
    assert_eq!(slice.num_ref_idx_l0_active_minus1, 5);
    assert_eq!(slice.num_ref_idx_l1_active_minus1, 3);
}

#[test]
fn test_slice_header_idr_pic_id() {
    let mut slice = create_minimal_slice_header();
    slice.idr_pic_id = 42;

    assert_eq!(slice.idr_pic_id, 42);
}

#[test]
fn test_slice_header_field_pic() {
    let mut slice = create_minimal_slice_header();
    slice.field_pic_flag = true;
    slice.bottom_field_flag = true;

    assert!(slice.field_pic_flag);
    assert!(slice.bottom_field_flag);
}

#[test]
fn test_slice_header_redundant_pic_cnt() {
    let mut slice = create_minimal_slice_header();
    slice.redundant_pic_cnt = 5;

    assert_eq!(slice.redundant_pic_cnt, 5);
}

#[test]
fn test_slice_header_slice_group_change_cycle() {
    let mut slice = create_minimal_slice_header();
    slice.slice_group_change_cycle = 100;

    assert_eq!(slice.slice_group_change_cycle, 100);
}

#[test]
fn test_chroma_format_from_u8() {
    assert_eq!(ChromaFormat::from_u8(0), ChromaFormat::Monochrome);
    assert_eq!(ChromaFormat::from_u8(1), ChromaFormat::Yuv420);
    assert_eq!(ChromaFormat::from_u8(2), ChromaFormat::Yuv422);
    assert_eq!(ChromaFormat::from_u8(3), ChromaFormat::Yuv444);
    assert_eq!(ChromaFormat::from_u8(255), ChromaFormat::Yuv420); // Default
}

#[test]
fn test_sps_constraint_flags() {
    let mut sps = create_minimal_sps();
    sps.constraint_set0_flag = true;
    sps.constraint_set3_flag = true;

    assert!(sps.constraint_set0_flag);
    assert!(sps.constraint_set3_flag);
}

#[test]
fn test_sps_vui_parameters_flag() {
    let mut sps = create_minimal_sps();
    sps.vui_parameters_present_flag = true;

    assert!(sps.vui_parameters_present_flag);
}

// ============================================================================
// ProfileIdc Display Trait Tests
// ============================================================================

#[test]
fn test_profile_idc_display() {
    let test_cases = vec![
        (ProfileIdc::Baseline, "Baseline"),
        (ProfileIdc::Main, "Main"),
        (ProfileIdc::Extended, "Extended"),
        (ProfileIdc::High, "High"),
        (ProfileIdc::High10, "High 10"),
        (ProfileIdc::High422, "High 4:2:2"),
        (ProfileIdc::High444, "High 4:4:4"),
        (ProfileIdc::Cavlc444, "CAVLC 4:4:4"),
        (ProfileIdc::ScalableBaseline, "Scalable Baseline"),
        (ProfileIdc::ScalableHigh, "Scalable High"),
        (ProfileIdc::MultiviewHigh, "Multiview High"),
        (ProfileIdc::StereoHigh, "Stereo High"),
        (ProfileIdc::Unknown, "Unknown"),
    ];

    for (profile, expected_name) in test_cases {
        let mut buffer = String::new();
        write!(buffer, "{}", profile).unwrap();
        assert_eq!(buffer, expected_name);
    }
}

// ============================================================================
// Additional ProfileIdc Tests
// ============================================================================

#[test]
fn test_profile_idc_from_u8_all_variants() {
    let test_cases = vec![
        (44, ProfileIdc::Cavlc444),
        (83, ProfileIdc::ScalableBaseline),
        (86, ProfileIdc::ScalableHigh),
        (118, ProfileIdc::MultiviewHigh),
        (128, ProfileIdc::StereoHigh),
        (88, ProfileIdc::Extended),
        (99, ProfileIdc::Unknown), // Undefined value
        (0, ProfileIdc::Unknown),
    ];

    for (value, expected) in test_cases {
        assert_eq!(
            ProfileIdc::from_u8(value),
            expected,
            "from_u8({}) should match",
            value
        );
    }
}

#[test]
fn test_profile_idc_is_high_profile_all_variants() {
    // High profile variants should return true
    assert!(ProfileIdc::High.is_high_profile());
    assert!(ProfileIdc::High10.is_high_profile());
    assert!(ProfileIdc::High422.is_high_profile());
    assert!(ProfileIdc::High444.is_high_profile());
    assert!(ProfileIdc::Cavlc444.is_high_profile());
    assert!(ProfileIdc::ScalableHigh.is_high_profile());
    assert!(ProfileIdc::MultiviewHigh.is_high_profile());
    assert!(ProfileIdc::StereoHigh.is_high_profile());

    // Non-high profiles should return false
    assert!(!ProfileIdc::Baseline.is_high_profile());
    assert!(!ProfileIdc::Main.is_high_profile());
    assert!(!ProfileIdc::Extended.is_high_profile());
    assert!(!ProfileIdc::ScalableBaseline.is_high_profile());
    assert!(!ProfileIdc::Unknown.is_high_profile());
}

// ============================================================================
// Additional ChromaFormat Tests
// ============================================================================

#[test]
fn test_chroma_format_sub_width_c_all() {
    assert_eq!(ChromaFormat::Monochrome.sub_width_c(), 0);
    assert_eq!(ChromaFormat::Yuv420.sub_width_c(), 2);
    assert_eq!(ChromaFormat::Yuv422.sub_width_c(), 2);
    assert_eq!(ChromaFormat::Yuv444.sub_width_c(), 1);
}

#[test]
fn test_chroma_format_sub_height_c_all() {
    assert_eq!(ChromaFormat::Monochrome.sub_height_c(), 0);
    assert_eq!(ChromaFormat::Yuv420.sub_height_c(), 2);
    assert_eq!(ChromaFormat::Yuv422.sub_height_c(), 1);
    assert_eq!(ChromaFormat::Yuv444.sub_height_c(), 1);
}

// ============================================================================
// SPS Display Size Tests with Different Chroma Formats
// ============================================================================

#[test]
fn test_sps_display_size_monochrome() {
    let mut sps = create_minimal_sps();
    sps.chroma_format_idc = ChromaFormat::Monochrome;
    sps.frame_cropping_flag = true;
    sps.frame_crop_left_offset = 2;
    sps.frame_crop_right_offset = 2;
    sps.frame_crop_top_offset = 1;
    sps.frame_crop_bottom_offset = 1;

    // Monochrome: crop_unit_x = 1, crop_unit_y = 1 * (2 - 1) = 1
    let width = sps.display_width();
    let height = sps.display_height();

    // 1920 - 1*(2+2) = 1916
    assert_eq!(width, 1916);
    // 1088 - 1*(1+1) = 1086
    assert_eq!(height, 1086);
}

#[test]
fn test_sps_display_size_yuv422() {
    let mut sps = create_minimal_sps();
    sps.chroma_format_idc = ChromaFormat::Yuv422;
    sps.frame_cropping_flag = true;
    sps.frame_crop_left_offset = 2;
    sps.frame_crop_right_offset = 2;
    sps.frame_crop_top_offset = 1;
    sps.frame_crop_bottom_offset = 1;

    // Yuv422: crop_unit_x = 2, crop_unit_y = 1 * (2 - 1) = 1
    let width = sps.display_width();
    let height = sps.display_height();

    // 1920 - 2*(2+2) = 1912
    assert_eq!(width, 1912);
    // 1088 - 1*(1+1) = 1086
    assert_eq!(height, 1086);
}

#[test]
fn test_sps_display_size_yuv444() {
    let mut sps = create_minimal_sps();
    sps.chroma_format_idc = ChromaFormat::Yuv444;
    sps.frame_cropping_flag = true;
    sps.frame_crop_left_offset = 2;
    sps.frame_crop_right_offset = 2;
    sps.frame_crop_top_offset = 1;
    sps.frame_crop_bottom_offset = 1;

    // Yuv444: crop_unit_x = 1, crop_unit_y = 1 * (2 - 1) = 1
    let width = sps.display_width();
    let height = sps.display_height();

    // 1920 - 1*(2+2) = 1916
    assert_eq!(width, 1916);
    // 1088 - 1*(1+1) = 1086
    assert_eq!(height, 1086);
}

#[test]
fn test_sps_display_size_field_mode() {
    let mut sps = create_minimal_sps();
    sps.frame_mbs_only_flag = false; // Field mode
    sps.pic_height_in_map_units_minus1 = 33; // For 1080p in field mode (1088/2/16 - 1)
    sps.chroma_format_idc = ChromaFormat::Yuv420;
    sps.frame_cropping_flag = true;
    sps.frame_crop_left_offset = 1;
    sps.frame_crop_right_offset = 1;
    sps.frame_crop_top_offset = 1;
    sps.frame_crop_bottom_offset = 1;

    // Field mode: pic_height = (2 - 0) * (33 + 1) * 16 = 1088
    // crop_unit_y = 2 * (2 - 0) = 4
    let height = sps.display_height();

    // 1088 - 4*(1+1) = 1080
    assert_eq!(height, 1080);
}

#[test]
fn test_sps_display_size_no_crop() {
    let sps = create_minimal_sps();

    // No crop flag set
    assert_eq!(sps.display_width(), 1920);
    assert_eq!(sps.display_height(), 1088);
}

// ============================================================================
// SPS Bit Depth Edge Cases
// ============================================================================

#[test]
fn test_sps_bit_depth_edge_cases() {
    let bit_depths = vec![0u8, 2, 4, 6]; // 8, 10, 12, 14 bit

    for bit_depth in bit_depths {
        let mut sps = create_minimal_sps();
        sps.bit_depth_luma_minus8 = bit_depth;
        sps.bit_depth_chroma_minus8 = bit_depth;

        assert_eq!(sps.bit_depth_luma(), bit_depth + 8);
        assert_eq!(sps.bit_depth_chroma(), bit_depth + 8);
    }
}

#[test]
fn test_sps_pic_height_field_mode() {
    let mut sps = create_minimal_sps();
    sps.frame_mbs_only_flag = false; // Field mode
    sps.pic_height_in_map_units_minus1 = 33; // For proper field mode calculation

    // pic_height = (2 - 0) * (33 + 1) * 16 = 1088
    assert_eq!(sps.pic_height(), 1088);
}

#[test]
fn test_sps_pic_width_various() {
    let widths = vec![160u32, 320, 640, 1280, 1920, 3840];

    for width in widths {
        let mut sps = create_minimal_sps();
        sps.pic_width_in_mbs_minus1 = (width / 16) - 1;

        assert_eq!(sps.pic_width(), width);
    }
}

// ============================================================================
// Additional SPS Tests (non-duplicates)
// ============================================================================

#[test]
fn test_sps_pic_order_cnt_type_variants() {
    let poc_types = vec![0u8, 1, 2];

    for poc_type in poc_types {
        let mut sps = create_minimal_sps();
        sps.pic_order_cnt_type = poc_type;

        assert_eq!(sps.pic_order_cnt_type, poc_type);
    }
}

#[test]
fn test_sps_log2_max_frame_num_variants() {
    let variants = vec![0u8, 4, 8, 12];

    for variant in variants {
        let mut sps = create_minimal_sps();
        sps.log2_max_frame_num_minus4 = variant;

        assert_eq!(sps.log2_max_frame_num_minus4, variant);
    }
}

#[test]
fn test_sps_max_num_ref_frames_variants() {
    let variants = vec![0u32, 1, 4, 16];

    for variant in variants {
        let mut sps = create_minimal_sps();
        sps.max_num_ref_frames = variant;

        assert_eq!(sps.max_num_ref_frames, variant);
    }
}

#[test]
fn test_sps_pic_order_cnt_lsb_variants() {
    let variants = vec![0u8, 4, 8, 16];

    for variant in variants {
        let mut sps = create_minimal_sps();
        sps.log2_max_pic_order_cnt_lsb_minus4 = variant;

        assert_eq!(sps.log2_max_pic_order_cnt_lsb_minus4, variant);
    }
}

#[test]
fn test_sps_delta_pic_order_always_zero() {
    let mut sps = create_minimal_sps();
    sps.delta_pic_order_always_zero_flag = true;

    assert!(sps.delta_pic_order_always_zero_flag);
}

#[test]
fn test_sps_gaps_in_frame_num_allowed() {
    let mut sps = create_minimal_sps();
    sps.gaps_in_frame_num_value_allowed_flag = true;

    assert!(sps.gaps_in_frame_num_value_allowed_flag);
}

#[test]
fn test_sps_direct_8x8_inference() {
    let mut sps = create_minimal_sps();
    sps.direct_8x8_inference_flag = true;

    assert!(sps.direct_8x8_inference_flag);
}

#[test]
fn test_sps_mb_adaptive_frame_field() {
    let mut sps = create_minimal_sps();
    sps.mb_adaptive_frame_field_flag = true;

    assert!(sps.mb_adaptive_frame_field_flag);
}

#[test]
fn test_sps_separate_colour_plane() {
    let mut sps = create_minimal_sps();
    sps.separate_colour_plane_flag = true;

    assert!(sps.separate_colour_plane_flag);
}

#[test]
fn test_sps_qpprime_y_zero_transform_bypass() {
    let mut sps = create_minimal_sps();
    sps.qpprime_y_zero_transform_bypass_flag = true;

    assert!(sps.qpprime_y_zero_transform_bypass_flag);
}

#[test]
fn test_sps_seq_scaling_matrix_flag() {
    let mut sps = create_minimal_sps();
    sps.seq_scaling_matrix_present_flag = true;

    assert!(sps.seq_scaling_matrix_present_flag);
}

#[test]
fn test_sps_offset_for_non_ref_pic() {
    let offsets = vec![0i32, -2, -1, 1, 2];

    for offset in offsets {
        let mut sps = create_minimal_sps();
        sps.offset_for_non_ref_pic = offset;

        assert_eq!(sps.offset_for_non_ref_pic, offset);
    }
}

#[test]
fn test_sps_offset_for_top_to_bottom_field() {
    let offsets = vec![0i32, -2, -1, 1, 2];

    for offset in offsets {
        let mut sps = create_minimal_sps();
        sps.offset_for_top_to_bottom_field = offset;

        assert_eq!(sps.offset_for_top_to_bottom_field, offset);
    }
}

#[test]
fn test_sps_offset_for_ref_frame_vec() {
    let offsets = vec![0i32, 1, 2, -1, -2];

    let mut sps = create_minimal_sps();
    sps.num_ref_frames_in_pic_order_cnt_cycle = offsets.len() as u8;
    sps.offset_for_ref_frame = offsets.clone();

    assert_eq!(sps.offset_for_ref_frame, offsets);
    assert_eq!(
        sps.num_ref_frames_in_pic_order_cnt_cycle,
        offsets.len() as u8
    );
}

#[test]
fn test_sps_num_ref_frames_in_pic_order_cnt_cycle() {
    let variants = vec![0u8, 1, 2, 4, 8];

    for variant in variants {
        let mut sps = create_minimal_sps();
        sps.num_ref_frames_in_pic_order_cnt_cycle = variant;
        sps.offset_for_ref_frame = vec![0; variant as usize];

        assert_eq!(sps.num_ref_frames_in_pic_order_cnt_cycle, variant);
        assert_eq!(sps.offset_for_ref_frame.len(), variant as usize);
    }
}

#[test]
fn test_sps_frame_cropping_all_offsets() {
    let mut sps = create_minimal_sps();
    sps.frame_cropping_flag = true;
    sps.frame_crop_left_offset = 10;
    sps.frame_crop_right_offset = 20;
    sps.frame_crop_top_offset = 5;
    sps.frame_crop_bottom_offset = 15;

    assert!(sps.frame_cropping_flag);
    assert_eq!(sps.frame_crop_left_offset, 10);
    assert_eq!(sps.frame_crop_right_offset, 20);
    assert_eq!(sps.frame_crop_top_offset, 5);
    assert_eq!(sps.frame_crop_bottom_offset, 15);
}

#[test]
fn test_sps_frame_cropping_partial_offsets() {
    let mut sps = create_minimal_sps();
    sps.frame_cropping_flag = true;
    sps.frame_crop_left_offset = 5;
    sps.frame_crop_right_offset = 0;
    sps.frame_crop_top_offset = 0;
    sps.frame_crop_bottom_offset = 10;

    assert!(sps.frame_cropping_flag);
    assert_eq!(sps.frame_crop_left_offset, 5);
    assert_eq!(sps.frame_crop_right_offset, 0);
    assert_eq!(sps.frame_crop_top_offset, 0);
    assert_eq!(sps.frame_crop_bottom_offset, 10);
}

#[test]
fn test_sps_seq_parameter_set_id_variants() {
    for id in 0u8..=10 {
        let mut sps = create_minimal_sps();
        sps.seq_parameter_set_id = id;

        assert_eq!(sps.seq_parameter_set_id, id);
    }
}

#[test]
fn test_sps_chroma_format_all_formats() {
    let formats = vec![
        ChromaFormat::Monochrome,
        ChromaFormat::Yuv420,
        ChromaFormat::Yuv422,
        ChromaFormat::Yuv444,
    ];

    for format in formats {
        let mut sps = create_minimal_sps();
        sps.chroma_format_idc = format;

        assert_eq!(sps.chroma_format_idc, format);
    }
}

#[test]
fn test_sps_display_size_field_mode_with_crop() {
    let mut sps = create_minimal_sps();
    sps.frame_mbs_only_flag = false; // Field mode
    sps.pic_width_in_mbs_minus1 = 119; // 1920
    sps.pic_height_in_map_units_minus1 = 67; // 1088
    sps.frame_cropping_flag = true;
    sps.frame_crop_left_offset = 10;
    sps.frame_crop_right_offset = 10;
    sps.frame_crop_top_offset = 5;
    sps.frame_crop_bottom_offset = 5;

    let display_width = sps.display_width();
    let display_height = sps.display_height();

    // Verify cropping is applied
    assert!(display_width < sps.pic_width());
    assert!(display_height < sps.pic_height());
}

#[test]
fn test_sps_display_size_monochrome_with_crop() {
    let mut sps = create_minimal_sps();
    sps.chroma_format_idc = ChromaFormat::Monochrome;
    sps.pic_width_in_mbs_minus1 = 39; // 640
    sps.pic_height_in_map_units_minus1 = 29; // 480
    sps.frame_cropping_flag = true;
    sps.frame_crop_left_offset = 5;
    sps.frame_crop_right_offset = 5;

    let display_width = sps.display_width();

    // Monochrome uses crop_unit_x = 1
    assert_eq!(display_width, 640 - 10);
}

#[test]
fn test_sps_profile_variants_all() {
    let profiles = vec![
        ProfileIdc::Baseline,
        ProfileIdc::Main,
        ProfileIdc::Extended,
        ProfileIdc::High,
        ProfileIdc::High10,
        ProfileIdc::High422,
        ProfileIdc::High444,
    ];

    for profile in profiles {
        let mut sps = create_minimal_sps();
        sps.profile_idc = profile.clone();

        assert_eq!(sps.profile_idc, profile);
    }
}

#[test]
fn test_sps_bit_depth_different_values() {
    let luma_depths = vec![0u8, 2, 4, 6]; // 8, 10, 12, 14 bit
    let chroma_depths = vec![0u8, 2, 4];

    for luma_depth in luma_depths {
        for chroma_depth in &chroma_depths {
            let mut sps = create_minimal_sps();
            sps.bit_depth_luma_minus8 = luma_depth;
            sps.bit_depth_chroma_minus8 = *chroma_depth;

            assert_eq!(sps.bit_depth_luma(), luma_depth + 8);
            assert_eq!(sps.bit_depth_chroma(), *chroma_depth + 8);
        }
    }
}

#[test]
fn test_sps_edge_case_minimal_dimensions() {
    let mut sps = create_minimal_sps();
    // Minimum valid size: 16x16 (1x1 macroblocks)
    sps.pic_width_in_mbs_minus1 = 0;
    sps.pic_height_in_map_units_minus1 = 0;

    assert_eq!(sps.pic_width(), 16);
    assert_eq!(sps.pic_height(), 16);
}

#[test]
fn test_sps_edge_case_large_dimensions() {
    let mut sps = create_minimal_sps();
    // Large but valid size
    sps.pic_width_in_mbs_minus1 = 679; // 10880 pixels
    sps.pic_height_in_map_units_minus1 = 679;

    assert_eq!(sps.pic_width(), 10880);
    assert_eq!(sps.pic_height(), 10880);
}
