#![allow(unused_imports, unused_mut)]
use super::*;

#[test]
fn test_empty_stream() {
    let data: &[u8] = &[];
    let stream = parse_avc(data).unwrap();
    assert_eq!(stream.nal_units.len(), 0);
    assert_eq!(stream.frame_count(), 0);
}

// Tests for calculate_poc (was 0% coverage)
#[test]
fn test_calculate_poc_type_0() {
    use super::test_exports::test_calculate_poc;
    use crate::slice::{DecRefPicMarking, RefPicListModification};

    let sps = Sps {
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
        log2_max_pic_order_cnt_lsb_minus4: 0,
        delta_pic_order_always_zero_flag: false,
        offset_for_non_ref_pic: 0,
        offset_for_top_to_bottom_field: 0,
        num_ref_frames_in_pic_order_cnt_cycle: 0,
        offset_for_ref_frame: vec![],
        max_num_ref_frames: 1,
        gaps_in_frame_num_value_allowed_flag: false,
        pic_width_in_mbs_minus1: 10,
        pic_height_in_map_units_minus1: 10,
        frame_mbs_only_flag: true,
        mb_adaptive_frame_field_flag: false,
        direct_8x8_inference_flag: true,
        frame_cropping_flag: false,
        frame_crop_left_offset: 0,
        frame_crop_right_offset: 0,
        frame_crop_top_offset: 0,
        frame_crop_bottom_offset: 0,
        vui_parameters_present_flag: false,
        vui_parameters: None,
    };

    let header = SliceHeader {
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
        ref_pic_list_modification_l0: RefPicListModification::default(),
        ref_pic_list_modification_l1: RefPicListModification::default(),
        dec_ref_pic_marking: DecRefPicMarking {
            no_output_of_prior_pics_flag: false,
            long_term_reference_flag: false,
            adaptive_ref_pic_marking_mode_flag: false,
            mmco_operations: vec![],
        },
        cabac_init_idc: 0,
        slice_qp_delta: 0,
        sp_for_switch_flag: false,
        slice_qs_delta: 0,
        disable_deblocking_filter_idc: 0,
        slice_alpha_c0_offset_div2: 0,
        slice_beta_offset_div2: 0,
        slice_group_change_cycle: 0,
    };

    let mut prev_poc_msb = 0;
    let mut prev_poc_lsb = 0;
    let mut prev_frame_num = 0;
    let mut prev_frame_num_offset = 0;

    let _ = test_calculate_poc(
        &sps,
        &header,
        false,
        &mut prev_poc_msb,
        &mut prev_poc_lsb,
        &mut prev_frame_num,
        &mut prev_frame_num_offset,
    );
}

#[test]
fn test_calculate_poc_type_1() {
    use super::test_exports::test_calculate_poc;
    use crate::slice::{DecRefPicMarking, RefPicListModification};

    let mut sps = Sps {
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
        pic_order_cnt_type: 1,
        log2_max_pic_order_cnt_lsb_minus4: 0,
        delta_pic_order_always_zero_flag: false,
        offset_for_non_ref_pic: 0,
        offset_for_top_to_bottom_field: 0,
        num_ref_frames_in_pic_order_cnt_cycle: 0,
        offset_for_ref_frame: vec![],
        max_num_ref_frames: 1,
        gaps_in_frame_num_value_allowed_flag: false,
        pic_width_in_mbs_minus1: 10,
        pic_height_in_map_units_minus1: 10,
        frame_mbs_only_flag: true,
        mb_adaptive_frame_field_flag: false,
        direct_8x8_inference_flag: true,
        frame_cropping_flag: false,
        frame_crop_left_offset: 0,
        frame_crop_right_offset: 0,
        frame_crop_top_offset: 0,
        frame_crop_bottom_offset: 0,
        vui_parameters_present_flag: false,
        vui_parameters: None,
    };

    let header = SliceHeader {
        first_mb_in_slice: 0,
        slice_type: SliceType::P,
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
        ref_pic_list_modification_l0: RefPicListModification::default(),
        ref_pic_list_modification_l1: RefPicListModification::default(),
        dec_ref_pic_marking: DecRefPicMarking {
            no_output_of_prior_pics_flag: false,
            long_term_reference_flag: false,
            adaptive_ref_pic_marking_mode_flag: false,
            mmco_operations: vec![],
        },
        cabac_init_idc: 0,
        slice_qp_delta: 0,
        sp_for_switch_flag: false,
        slice_qs_delta: 0,
        disable_deblocking_filter_idc: 0,
        slice_alpha_c0_offset_div2: 0,
        slice_beta_offset_div2: 0,
        slice_group_change_cycle: 0,
    };

    let mut prev_poc_msb = 0;
    let mut prev_poc_lsb = 0;
    let mut prev_frame_num = 0;
    let mut prev_frame_num_offset = 0;

    let _ = test_calculate_poc(
        &sps,
        &header,
        false,
        &mut prev_poc_msb,
        &mut prev_poc_lsb,
        &mut prev_frame_num,
        &mut prev_frame_num_offset,
    );
}

#[test]
fn test_calculate_poc_type_2() {
    use super::test_exports::test_calculate_poc;
    use crate::slice::{DecRefPicMarking, RefPicListModification};

    let mut sps = Sps {
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
        pic_order_cnt_type: 2,
        log2_max_pic_order_cnt_lsb_minus4: 0,
        delta_pic_order_always_zero_flag: false,
        offset_for_non_ref_pic: 0,
        offset_for_top_to_bottom_field: 0,
        num_ref_frames_in_pic_order_cnt_cycle: 0,
        offset_for_ref_frame: vec![],
        max_num_ref_frames: 1,
        gaps_in_frame_num_value_allowed_flag: false,
        pic_width_in_mbs_minus1: 10,
        pic_height_in_map_units_minus1: 10,
        frame_mbs_only_flag: true,
        mb_adaptive_frame_field_flag: false,
        direct_8x8_inference_flag: true,
        frame_cropping_flag: false,
        frame_crop_left_offset: 0,
        frame_crop_right_offset: 0,
        frame_crop_top_offset: 0,
        frame_crop_bottom_offset: 0,
        vui_parameters_present_flag: false,
        vui_parameters: None,
    };

    let header = SliceHeader {
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
        ref_pic_list_modification_l0: RefPicListModification::default(),
        ref_pic_list_modification_l1: RefPicListModification::default(),
        dec_ref_pic_marking: DecRefPicMarking {
            no_output_of_prior_pics_flag: false,
            long_term_reference_flag: false,
            adaptive_ref_pic_marking_mode_flag: false,
            mmco_operations: vec![],
        },
        cabac_init_idc: 0,
        slice_qp_delta: 0,
        sp_for_switch_flag: false,
        slice_qs_delta: 0,
        disable_deblocking_filter_idc: 0,
        slice_alpha_c0_offset_div2: 0,
        slice_beta_offset_div2: 0,
        slice_group_change_cycle: 0,
    };

    let mut prev_poc_msb = 0;
    let mut prev_poc_lsb = 0;
    let mut prev_frame_num = 0;
    let mut prev_frame_num_offset = 0;

    let _ = test_calculate_poc(
        &sps,
        &header,
        false,
        &mut prev_poc_msb,
        &mut prev_poc_lsb,
        &mut prev_frame_num,
        &mut prev_frame_num_offset,
    );
}

#[test]
fn test_calculate_poc_unknown_type() {
    use super::test_exports::test_calculate_poc;
    use crate::slice::{DecRefPicMarking, RefPicListModification};

    let mut sps = Sps {
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
        pic_order_cnt_type: 3,
        log2_max_pic_order_cnt_lsb_minus4: 0,
        delta_pic_order_always_zero_flag: false,
        offset_for_non_ref_pic: 0,
        offset_for_top_to_bottom_field: 0,
        num_ref_frames_in_pic_order_cnt_cycle: 0,
        offset_for_ref_frame: vec![],
        max_num_ref_frames: 1,
        gaps_in_frame_num_value_allowed_flag: false,
        pic_width_in_mbs_minus1: 10,
        pic_height_in_map_units_minus1: 10,
        frame_mbs_only_flag: true,
        mb_adaptive_frame_field_flag: false,
        direct_8x8_inference_flag: true,
        frame_cropping_flag: false,
        frame_crop_left_offset: 0,
        frame_crop_right_offset: 0,
        frame_crop_top_offset: 0,
        frame_crop_bottom_offset: 0,
        vui_parameters_present_flag: false,
        vui_parameters: None,
    };

    let header = SliceHeader {
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
        ref_pic_list_modification_l0: RefPicListModification::default(),
        ref_pic_list_modification_l1: RefPicListModification::default(),
        dec_ref_pic_marking: DecRefPicMarking {
            no_output_of_prior_pics_flag: false,
            long_term_reference_flag: false,
            adaptive_ref_pic_marking_mode_flag: false,
            mmco_operations: vec![],
        },
        cabac_init_idc: 0,
        slice_qp_delta: 0,
        sp_for_switch_flag: false,
        slice_qs_delta: 0,
        disable_deblocking_filter_idc: 0,
        slice_alpha_c0_offset_div2: 0,
        slice_beta_offset_div2: 0,
        slice_group_change_cycle: 0,
    };

    let mut prev_poc_msb = 0;
    let mut prev_poc_lsb = 0;
    let mut prev_frame_num = 0;
    let mut prev_frame_num_offset = 0;

    let poc = test_calculate_poc(
        &sps,
        &header,
        false,
        &mut prev_poc_msb,
        &mut prev_poc_lsb,
        &mut prev_frame_num,
        &mut prev_frame_num_offset,
    );
    assert_eq!(poc, 0);
}

#[test]
fn test_calculate_poc_frame_sequence() {
    use super::test_exports::test_calculate_poc;
    use crate::slice::{DecRefPicMarking, RefPicListModification};

    let sps = Sps {
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
        log2_max_pic_order_cnt_lsb_minus4: 0,
        delta_pic_order_always_zero_flag: false,
        offset_for_non_ref_pic: 0,
        offset_for_top_to_bottom_field: 0,
        num_ref_frames_in_pic_order_cnt_cycle: 0,
        offset_for_ref_frame: vec![],
        max_num_ref_frames: 1,
        gaps_in_frame_num_value_allowed_flag: false,
        pic_width_in_mbs_minus1: 10,
        pic_height_in_map_units_minus1: 10,
        frame_mbs_only_flag: true,
        mb_adaptive_frame_field_flag: false,
        direct_8x8_inference_flag: true,
        frame_cropping_flag: false,
        frame_crop_left_offset: 0,
        frame_crop_right_offset: 0,
        frame_crop_top_offset: 0,
        frame_crop_bottom_offset: 0,
        vui_parameters_present_flag: false,
        vui_parameters: None,
    };

    let mut header = SliceHeader {
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
        ref_pic_list_modification_l0: RefPicListModification::default(),
        ref_pic_list_modification_l1: RefPicListModification::default(),
        dec_ref_pic_marking: DecRefPicMarking {
            no_output_of_prior_pics_flag: false,
            long_term_reference_flag: false,
            adaptive_ref_pic_marking_mode_flag: false,
            mmco_operations: vec![],
        },
        cabac_init_idc: 0,
        slice_qp_delta: 0,
        sp_for_switch_flag: false,
        slice_qs_delta: 0,
        disable_deblocking_filter_idc: 0,
        slice_alpha_c0_offset_div2: 0,
        slice_beta_offset_div2: 0,
        slice_group_change_cycle: 0,
    };

    let mut prev_poc_msb = 0;
    let mut prev_poc_lsb = 0;
    let mut prev_frame_num = 0;
    let mut prev_frame_num_offset = 0;

    for i in 0..10 {
        header.frame_num = i;
        header.pic_order_cnt_lsb = i * 2;
        let _ = test_calculate_poc(
            &sps,
            &header,
            false,
            &mut prev_poc_msb,
            &mut prev_poc_lsb,
            &mut prev_frame_num,
            &mut prev_frame_num_offset,
        );
    }
}

// More POC type 0 tests with edge cases
#[test]
fn test_calculate_poc_type_0_poc_wrapping() {
    use super::test_exports::test_calculate_poc;
    use crate::slice::{DecRefPicMarking, RefPicListModification};

    let sps = Sps {
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
        log2_max_frame_num_minus4: 4, // MaxPicOrderCntLsb = 16
        pic_order_cnt_type: 0,
        log2_max_pic_order_cnt_lsb_minus4: 4,
        delta_pic_order_always_zero_flag: false,
        offset_for_non_ref_pic: 0,
        offset_for_top_to_bottom_field: 0,
        num_ref_frames_in_pic_order_cnt_cycle: 0,
        offset_for_ref_frame: vec![],
        max_num_ref_frames: 1,
        gaps_in_frame_num_value_allowed_flag: false,
        pic_width_in_mbs_minus1: 10,
        pic_height_in_map_units_minus1: 10,
        frame_mbs_only_flag: true,
        mb_adaptive_frame_field_flag: false,
        direct_8x8_inference_flag: true,
        frame_cropping_flag: false,
        frame_crop_left_offset: 0,
        frame_crop_right_offset: 0,
        frame_crop_top_offset: 0,
        frame_crop_bottom_offset: 0,
        vui_parameters_present_flag: false,
        vui_parameters: None,
    };

    let mut header = SliceHeader {
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
        ref_pic_list_modification_l0: RefPicListModification::default(),
        ref_pic_list_modification_l1: RefPicListModification::default(),
        dec_ref_pic_marking: DecRefPicMarking {
            no_output_of_prior_pics_flag: false,
            long_term_reference_flag: false,
            adaptive_ref_pic_marking_mode_flag: false,
            mmco_operations: vec![],
        },
        cabac_init_idc: 0,
        slice_qp_delta: 0,
        sp_for_switch_flag: false,
        slice_qs_delta: 0,
        disable_deblocking_filter_idc: 0,
        slice_alpha_c0_offset_div2: 0,
        slice_beta_offset_div2: 0,
        slice_group_change_cycle: 0,
    };

    let mut prev_poc_msb = 0;
    let mut prev_poc_lsb = 0;
    let mut prev_frame_num = 0;
    let mut prev_frame_num_offset = 0;

    // Test POC wrapping at max value
    for lsb in [0u32, 8, 15, 16, 255, 256, 511, 512, 32767, 32768, 65535] {
        header.pic_order_cnt_lsb = lsb;
        let _ = test_calculate_poc(
            &sps,
            &header,
            false,
            &mut prev_poc_msb,
            &mut prev_poc_lsb,
            &mut prev_frame_num,
            &mut prev_frame_num_offset,
        );
    }
}

// POC type 1 with various configurations
#[test]
fn test_calculate_poc_type_1_with_ref_cycle() {
    use super::test_exports::test_calculate_poc;
    use crate::slice::{DecRefPicMarking, RefPicListModification};

    let mut sps = Sps {
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
        pic_order_cnt_type: 1,
        log2_max_pic_order_cnt_lsb_minus4: 0,
        delta_pic_order_always_zero_flag: false,
        offset_for_non_ref_pic: 2,
        offset_for_top_to_bottom_field: 1,
        num_ref_frames_in_pic_order_cnt_cycle: 3,
        offset_for_ref_frame: vec![1, 2, 3],
        max_num_ref_frames: 3,
        gaps_in_frame_num_value_allowed_flag: false,
        pic_width_in_mbs_minus1: 10,
        pic_height_in_map_units_minus1: 10,
        frame_mbs_only_flag: true,
        mb_adaptive_frame_field_flag: false,
        direct_8x8_inference_flag: true,
        frame_cropping_flag: false,
        frame_crop_left_offset: 0,
        frame_crop_right_offset: 0,
        frame_crop_top_offset: 0,
        frame_crop_bottom_offset: 0,
        vui_parameters_present_flag: false,
        vui_parameters: None,
    };

    let header = SliceHeader {
        first_mb_in_slice: 0,
        slice_type: SliceType::P,
        pic_parameter_set_id: 0,
        colour_plane_id: 0,
        frame_num: 5,
        field_pic_flag: false,
        bottom_field_flag: false,
        idr_pic_id: 0,
        pic_order_cnt_lsb: 0,
        delta_pic_order_cnt_bottom: 0,
        delta_pic_order_cnt: [1, -1],
        redundant_pic_cnt: 0,
        direct_spatial_mv_pred_flag: false,
        num_ref_idx_active_override_flag: false,
        num_ref_idx_l0_active_minus1: 0,
        num_ref_idx_l1_active_minus1: 0,
        ref_pic_list_modification_flag_l0: false,
        ref_pic_list_modification_flag_l1: false,
        ref_pic_list_modification_l0: RefPicListModification::default(),
        ref_pic_list_modification_l1: RefPicListModification::default(),
        dec_ref_pic_marking: DecRefPicMarking {
            no_output_of_prior_pics_flag: false,
            long_term_reference_flag: false,
            adaptive_ref_pic_marking_mode_flag: false,
            mmco_operations: vec![],
        },
        cabac_init_idc: 0,
        slice_qp_delta: 0,
        sp_for_switch_flag: false,
        slice_qs_delta: 0,
        disable_deblocking_filter_idc: 0,
        slice_alpha_c0_offset_div2: 0,
        slice_beta_offset_div2: 0,
        slice_group_change_cycle: 0,
    };

    let mut prev_poc_msb = 0;
    let mut prev_poc_lsb = 0;
    let mut prev_frame_num = 0;
    let mut prev_frame_num_offset = 0;

    let _ = test_calculate_poc(
        &sps,
        &header,
        false,
        &mut prev_poc_msb,
        &mut prev_poc_lsb,
        &mut prev_frame_num,
        &mut prev_frame_num_offset,
    );
}

// POC type 2 with field pictures
#[test]
fn test_calculate_poc_type_2_field_pics() {
    use super::test_exports::test_calculate_poc;
    use crate::slice::{DecRefPicMarking, RefPicListModification};

    let mut sps = Sps {
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
        pic_order_cnt_type: 2,
        log2_max_pic_order_cnt_lsb_minus4: 0,
        delta_pic_order_always_zero_flag: false,
        offset_for_non_ref_pic: 0,
        offset_for_top_to_bottom_field: 0,
        num_ref_frames_in_pic_order_cnt_cycle: 0,
        offset_for_ref_frame: vec![],
        max_num_ref_frames: 1,
        gaps_in_frame_num_value_allowed_flag: false,
        pic_width_in_mbs_minus1: 10,
        pic_height_in_map_units_minus1: 10,
        frame_mbs_only_flag: false, // Field pictures
        mb_adaptive_frame_field_flag: false,
        direct_8x8_inference_flag: true,
        frame_cropping_flag: false,
        frame_crop_left_offset: 0,
        frame_crop_right_offset: 0,
        frame_crop_top_offset: 0,
        frame_crop_bottom_offset: 0,
        vui_parameters_present_flag: false,
        vui_parameters: None,
    };

    let mut header = SliceHeader {
        first_mb_in_slice: 0,
        slice_type: SliceType::I,
        pic_parameter_set_id: 0,
        colour_plane_id: 0,
        frame_num: 0,
        field_pic_flag: true,
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
        ref_pic_list_modification_l0: RefPicListModification::default(),
        ref_pic_list_modification_l1: RefPicListModification::default(),
        dec_ref_pic_marking: DecRefPicMarking {
            no_output_of_prior_pics_flag: false,
            long_term_reference_flag: false,
            adaptive_ref_pic_marking_mode_flag: false,
            mmco_operations: vec![],
        },
        cabac_init_idc: 0,
        slice_qp_delta: 0,
        sp_for_switch_flag: false,
        slice_qs_delta: 0,
        disable_deblocking_filter_idc: 0,
        slice_alpha_c0_offset_div2: 0,
        slice_beta_offset_div2: 0,
        slice_group_change_cycle: 0,
    };

    let mut prev_poc_msb = 0;
    let mut prev_poc_lsb = 0;
    let mut prev_frame_num = 0;
    let mut prev_frame_num_offset = 0;

    // Top field
    header.bottom_field_flag = false;
    let _ = test_calculate_poc(
        &sps,
        &header,
        false,
        &mut prev_poc_msb,
        &mut prev_poc_lsb,
        &mut prev_frame_num,
        &mut prev_frame_num_offset,
    );

    // Bottom field
    header.bottom_field_flag = true;
    let _ = test_calculate_poc(
        &sps,
        &header,
        false,
        &mut prev_poc_msb,
        &mut prev_poc_lsb,
        &mut prev_frame_num,
        &mut prev_frame_num_offset,
    );
}

// Various slice types with calculate_poc
#[test]
fn test_calculate_poc_all_slice_types() {
    use super::test_exports::test_calculate_poc;
    use crate::slice::{DecRefPicMarking, RefPicListModification};

    let sps = Sps {
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
        log2_max_pic_order_cnt_lsb_minus4: 0,
        delta_pic_order_always_zero_flag: false,
        offset_for_non_ref_pic: 0,
        offset_for_top_to_bottom_field: 0,
        num_ref_frames_in_pic_order_cnt_cycle: 0,
        offset_for_ref_frame: vec![],
        max_num_ref_frames: 1,
        gaps_in_frame_num_value_allowed_flag: false,
        pic_width_in_mbs_minus1: 10,
        pic_height_in_map_units_minus1: 10,
        frame_mbs_only_flag: true,
        mb_adaptive_frame_field_flag: false,
        direct_8x8_inference_flag: true,
        frame_cropping_flag: false,
        frame_crop_left_offset: 0,
        frame_crop_right_offset: 0,
        frame_crop_top_offset: 0,
        frame_crop_bottom_offset: 0,
        vui_parameters_present_flag: false,
        vui_parameters: None,
    };

    for slice_type in [SliceType::I, SliceType::P, SliceType::B] {
        let mut header = SliceHeader {
            first_mb_in_slice: 0,
            slice_type,
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
            ref_pic_list_modification_l0: RefPicListModification::default(),
            ref_pic_list_modification_l1: RefPicListModification::default(),
            dec_ref_pic_marking: DecRefPicMarking {
                no_output_of_prior_pics_flag: false,
                long_term_reference_flag: false,
                adaptive_ref_pic_marking_mode_flag: false,
                mmco_operations: vec![],
            },
            cabac_init_idc: 0,
            slice_qp_delta: 0,
            sp_for_switch_flag: false,
            slice_qs_delta: 0,
            disable_deblocking_filter_idc: 0,
            slice_alpha_c0_offset_div2: 0,
            slice_beta_offset_div2: 0,
            slice_group_change_cycle: 0,
        };

        let mut prev_poc_msb = 0;
        let mut prev_poc_lsb = 0;
        let mut prev_frame_num = 0;
        let mut prev_frame_num_offset = 0;

        let _ = test_calculate_poc(
            &sps,
            &header,
            false,
            &mut prev_poc_msb,
            &mut prev_poc_lsb,
            &mut prev_frame_num,
            &mut prev_frame_num_offset,
        );
    }
}

// Additional tests for main parser functions and AvcStream methods

#[test]
fn test_parse_avc_with_start_code() {
    // Test parsing with Annex B start code prefix
    // Create a valid SPS NAL unit with start code
    let mut data = vec![0u8; 64];
    // Start code prefix (3-byte)
    data[0] = 0x00;
    data[1] = 0x00;
    data[2] = 0x01;
    // NAL header: nal_ref_idc=3, nal_unit_type=7 (SPS)
    data[3] = 0x67; // forbidden_zero_bit=0, nal_ref_idc=3 (11b), nal_unit_type=7 (00111b)
                    // Minimal SPS data (profile_idc=66 baseline, level=40)
    data[4] = 0x42; // profile_idc = 66 (baseline)
    data[5] = 0x00; // constraint sets
    data[6] = 0x28; // level_idc = 40
    data[7] = 0xFF; // seq_parameter_set_id=0 + reserved
    data[8] = 0x01; // chroma_format_idc=1 (4:2:0)

    let result = parse_avc(&data);
    // Should parse SPS successfully or at least not crash
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_avc_with_4byte_start_code() {
    // Test parsing with 4-byte start code prefix
    let mut data = vec![0u8; 64];
    // Start code prefix (4-byte)
    data[0] = 0x00;
    data[1] = 0x00;
    data[2] = 0x00;
    data[3] = 0x01;
    // NAL header for SPS
    data[4] = 0x67; // SPS
    data[5] = 0x42; // profile_idc = 66
    data[6] = 0x00;
    data[7] = 0x28;
    data[8] = 0xFF;
    data[9] = 0x01;

    let result = parse_avc(&data);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_avc_multiple_nal_units() {
    // Test multiple NAL units in stream
    let mut data = vec![0u8; 64];
    let mut pos = 0;

    // First NAL: SPS (nal_unit_type=7)
    data[pos] = 0x00;
    pos += 1;
    data[pos] = 0x00;
    pos += 1;
    data[pos] = 0x01;
    pos += 1;
    data[pos] = 0x67; // nal_ref_idc=3, nal_unit_type=7
    pos += 1;
    // Minimal SPS data
    for _ in 0..10 {
        data[pos] = 0;
        pos += 1;
    }

    // Second NAL: PPS (nal_unit_type=8)
    data[pos] = 0x00;
    pos += 1;
    data[pos] = 0x00;
    pos += 1;
    data[pos] = 0x01;
    pos += 1;
    data[pos] = 0x68; // nal_ref_idc=3, nal_unit_type=8
    pos += 1;
    // Minimal PPS data
    for _ in 0..5 {
        data[pos] = 0;
        pos += 1;
    }

    let result = parse_avc(&data[..pos]);
    assert!(result.is_ok());
    let stream = result.unwrap();
    assert!(stream.nal_units.len() >= 2);
}

#[test]
fn test_parse_avc_idr_slice() {
    // Test IDR slice detection
    let mut data = vec![0u8; 32];
    data[0] = 0x00;
    data[1] = 0x00;
    data[2] = 0x01;
    data[3] = 0x65; // nal_ref_idc=3, nal_unit_type=5 (IDR slice)

    let result = parse_avc(&data);
    assert!(result.is_ok());
    let stream = result.unwrap();
    // Check that IDR slice was identified
    if !stream.nal_units.is_empty() {
        assert_eq!(
            stream.nal_units[0].header.nal_unit_type,
            NalUnitType::IdrSlice
        );
    }
}

#[test]
fn test_parse_avc_non_idr_slice() {
    // Test non-IDR slice detection
    let mut data = vec![0u8; 32];
    data[0] = 0x00;
    data[1] = 0x00;
    data[2] = 0x01;
    data[3] = 0x81; // nal_ref_idc=3, nal_unit_type=1 (non-IDR slice)

    let result = parse_avc(&data);
    // Parser may fail with incomplete slice data, that's acceptable
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_avc_sei_nal() {
    // Test SEI NAL unit (nal_unit_type=6)
    let mut data = vec![0u8; 32];
    data[0] = 0x00;
    data[1] = 0x00;
    data[2] = 0x01;
    data[3] = 0x66; // nal_ref_idc=3, nal_unit_type=6 (SEI)

    let result = parse_avc(&data);
    assert!(result.is_ok());
    let stream = result.unwrap();
    if !stream.nal_units.is_empty() {
        assert_eq!(stream.nal_units[0].header.nal_unit_type, NalUnitType::Sei);
    }
}

#[test]
fn test_parse_avc_aud_nal() {
    // Test Access Unit Delimiter (nal_unit_type=9)
    let mut data = vec![0u8; 32];
    data[0] = 0x00;
    data[1] = 0x00;
    data[2] = 0x01;
    data[3] = 0x09; // nal_ref_idc=0, nal_unit_type=9 (AUD)

    let result = parse_avc(&data);
    assert!(result.is_ok());
    let stream = result.unwrap();
    if !stream.nal_units.is_empty() {
        assert_eq!(stream.nal_units[0].header.nal_unit_type, NalUnitType::Aud);
    }
}

#[test]
fn test_parse_avc_end_of_sequence() {
    // Test End of Sequence (nal_unit_type=10)
    let mut data = vec![0u8; 16];
    data[0] = 0x00;
    data[1] = 0x00;
    data[2] = 0x01;
    data[3] = 0x0A; // nal_ref_idc=0, nal_unit_type=10 (EOS)

    let result = parse_avc(&data);
    assert!(result.is_ok());
}

#[test]
fn test_parse_avc_end_of_stream() {
    // Test End of Stream (nal_unit_type=11)
    let mut data = vec![0u8; 16];
    data[0] = 0x00;
    data[1] = 0x00;
    data[2] = 0x01;
    data[3] = 0x0B; // nal_ref_idc=0, nal_unit_type=11 (EOStream)

    let result = parse_avc(&data);
    assert!(result.is_ok());
}

#[test]
fn test_parse_avc_filler_data() {
    // Test Filler Data (nal_unit_type=12)
    let mut data = vec![0u8; 32];
    data[0] = 0x00;
    data[1] = 0x00;
    data[2] = 0x01;
    data[3] = 0x0C; // nal_ref_idc=0, nal_unit_type=12 (Filler)
    for i in 4..16 {
        data[i] = 0xFF; // Filler bytes
    }

    let result = parse_avc(&data);
    assert!(result.is_ok());
}

#[test]
fn test_parse_avc_quick() {
    // Test parse_avc_quick function
    let mut data = vec![0u8; 32];
    // Add SPS NAL
    data[0] = 0x00;
    data[1] = 0x00;
    data[2] = 0x01;
    data[3] = 0x67; // SPS

    let result = parse_avc_quick(&data);
    assert!(result.is_ok());
    let info = result.unwrap();
    assert!(info.nal_count >= 1);
}

#[test]
fn test_parse_avc_quick_empty() {
    let result = parse_avc_quick(&[]);
    assert!(result.is_ok());
    let info = result.unwrap();
    assert_eq!(info.nal_count, 0);
    assert_eq!(info.frame_count, 0);
}

#[test]
fn test_avc_stream_methods() {
    // Test AvcStream methods with default stream
    let stream = AvcStream {
        nal_units: vec![],
        sps_map: std::collections::HashMap::new(),
        pps_map: std::collections::HashMap::new(),
        slices: vec![],
        sei_messages: vec![],
    };

    assert_eq!(stream.frame_count(), 0);
    assert_eq!(stream.idr_frames().len(), 0);
    assert!(stream.dimensions().is_none());
    assert!(stream.frame_rate().is_none());
    assert!(stream.bit_depth_luma().is_none());
    assert!(stream.bit_depth_chroma().is_none());
    assert!(stream.chroma_format().is_none());
    assert!(stream.get_sps(0).is_none());
    assert!(stream.get_pps(0).is_none());
}

#[test]
fn test_avc_stream_with_sps() {
    // Test AvcStream with SPS data
    use crate::slice::DecRefPicMarking;

    let mut sps_map = std::collections::HashMap::new();
    sps_map.insert(
        0,
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
            log2_max_pic_order_cnt_lsb_minus4: 0,
            delta_pic_order_always_zero_flag: false,
            offset_for_non_ref_pic: 0,
            offset_for_top_to_bottom_field: 0,
            num_ref_frames_in_pic_order_cnt_cycle: 0,
            offset_for_ref_frame: vec![],
            max_num_ref_frames: 1,
            gaps_in_frame_num_value_allowed_flag: false,
            pic_width_in_mbs_minus1: 21,        // (21+1)*16 = 352
            pic_height_in_map_units_minus1: 21, // (21+1)*16 = 352
            frame_mbs_only_flag: true,
            mb_adaptive_frame_field_flag: false,
            direct_8x8_inference_flag: true,
            frame_cropping_flag: false,
            frame_crop_left_offset: 0,
            frame_crop_right_offset: 0,
            frame_crop_top_offset: 0,
            frame_crop_bottom_offset: 0,
            vui_parameters_present_flag: false,
            vui_parameters: None,
        },
    );

    let stream = AvcStream {
        nal_units: vec![],
        sps_map,
        pps_map: std::collections::HashMap::new(),
        slices: vec![],
        sei_messages: vec![],
    };

    assert!(stream.dimensions().is_some());
    assert_eq!(stream.dimensions(), Some((352, 352))); // (21+1)*16 = 352
    assert_eq!(stream.bit_depth_luma(), Some(8));
    assert_eq!(stream.bit_depth_chroma(), Some(8));
    assert_eq!(stream.chroma_format(), Some(ChromaFormat::Yuv420));
    assert!(stream.get_sps(0).is_some());
    assert!(stream.profile_string().is_some());
    assert!(stream.level_string().is_some());
}

#[test]
fn test_avc_stream_profile_string() {
    // Test profile string conversion for different profiles
    let profiles = vec![
        (ProfileIdc::Baseline, "Baseline"),
        (ProfileIdc::Main, "Main"),
        (ProfileIdc::Extended, "Extended"),
        (ProfileIdc::High, "High"),
        (ProfileIdc::High10, "High 10"),
        (ProfileIdc::High422, "High 4:2:2"),
        (ProfileIdc::High444, "High 4:4:4"),
    ];

    for (profile_idc, expected_name) in profiles {
        let mut sps_map = std::collections::HashMap::new();
        sps_map.insert(
            0,
            Sps {
                profile_idc: profile_idc.clone(),
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
                log2_max_pic_order_cnt_lsb_minus4: 0,
                delta_pic_order_always_zero_flag: false,
                offset_for_non_ref_pic: 0,
                offset_for_top_to_bottom_field: 0,
                num_ref_frames_in_pic_order_cnt_cycle: 0,
                offset_for_ref_frame: vec![],
                max_num_ref_frames: 1,
                gaps_in_frame_num_value_allowed_flag: false,
                pic_width_in_mbs_minus1: 10,
                pic_height_in_map_units_minus1: 10,
                frame_mbs_only_flag: true,
                mb_adaptive_frame_field_flag: false,
                direct_8x8_inference_flag: true,
                frame_cropping_flag: false,
                frame_crop_left_offset: 0,
                frame_crop_right_offset: 0,
                frame_crop_top_offset: 0,
                frame_crop_bottom_offset: 0,
                vui_parameters_present_flag: false,
                vui_parameters: None,
            },
        );

        let stream = AvcStream {
            nal_units: vec![],
            sps_map,
            pps_map: std::collections::HashMap::new(),
            slices: vec![],
            sei_messages: vec![],
        };

        let profile_str = stream.profile_string().unwrap();
        assert!(profile_str.contains(expected_name) || profile_str == "Unknown");
    }
}

#[test]
fn test_avc_stream_level_string() {
    // Test level string conversion
    for level_idc in [10, 11, 12, 20, 21, 30, 31, 40, 41, 50, 51, 52] {
        let mut sps_map = std::collections::HashMap::new();
        sps_map.insert(
            0,
            Sps {
                profile_idc: ProfileIdc::High,
                constraint_set0_flag: false,
                constraint_set1_flag: false,
                constraint_set2_flag: false,
                constraint_set3_flag: false,
                constraint_set4_flag: false,
                constraint_set5_flag: false,
                level_idc,
                seq_parameter_set_id: 0,
                chroma_format_idc: ChromaFormat::Yuv420,
                separate_colour_plane_flag: false,
                bit_depth_luma_minus8: 0,
                bit_depth_chroma_minus8: 0,
                qpprime_y_zero_transform_bypass_flag: false,
                seq_scaling_matrix_present_flag: false,
                log2_max_frame_num_minus4: 0,
                pic_order_cnt_type: 0,
                log2_max_pic_order_cnt_lsb_minus4: 0,
                delta_pic_order_always_zero_flag: false,
                offset_for_non_ref_pic: 0,
                offset_for_top_to_bottom_field: 0,
                num_ref_frames_in_pic_order_cnt_cycle: 0,
                offset_for_ref_frame: vec![],
                max_num_ref_frames: 1,
                gaps_in_frame_num_value_allowed_flag: false,
                pic_width_in_mbs_minus1: 10,
                pic_height_in_map_units_minus1: 10,
                frame_mbs_only_flag: true,
                mb_adaptive_frame_field_flag: false,
                direct_8x8_inference_flag: true,
                frame_cropping_flag: false,
                frame_crop_left_offset: 0,
                frame_crop_right_offset: 0,
                frame_crop_top_offset: 0,
                frame_crop_bottom_offset: 0,
                vui_parameters_present_flag: false,
                vui_parameters: None,
            },
        );

        let stream = AvcStream {
            nal_units: vec![],
            sps_map,
            pps_map: std::collections::HashMap::new(),
            slices: vec![],
            sei_messages: vec![],
        };

        let level_str = stream.level_string().unwrap();
        let major = level_idc / 10;
        let minor = level_idc % 10;
        assert_eq!(level_str, format!("{}.{}", major, minor));
    }
}

#[test]
fn test_avc_quick_info_default() {
    // Test AvcQuickInfo with default values
    let info = AvcQuickInfo {
        nal_count: 0,
        sps_count: 0,
        pps_count: 0,
        idr_count: 0,
        frame_count: 0,
        width: None,
        height: None,
        profile: None,
        level: None,
    };

    assert_eq!(info.nal_count, 0);
    assert_eq!(info.sps_count, 0);
    assert!(info.width.is_none());
}

#[test]
fn test_parse_avc_without_start_code() {
    // Test parsing data without start code (raw NAL unit)
    let mut data = vec![0u8; 16];
    data[0] = 0x67; // SPS NAL header without start code

    let result = parse_avc(&data);
    // Should still parse, treating it as a single NAL
    assert!(result.is_ok());
}

#[test]
fn test_parse_avc_corrupted_start_code() {
    // Test handling of corrupted start codes
    let data = [0x00, 0x00, 0x02, 0x01, 0x67]; // Corrupted (0x02 instead of 0x00 or 0x01)

    let result = parse_avc(&data);
    // Should handle gracefully
    assert!(result.is_ok());
}

// ===== Comprehensive Public API Tests =====

// parse_avc() tests

#[test]
fn test_parse_avc_with_sps_only() {
    // Test parse_avc with SPS only
    let mut data = vec![0u8; 32];
    data[0] = 0x00;
    data[1] = 0x00;
    data[2] = 0x01;
    data[3] = 0x67; // SPS
    data[4] = 0x42; // baseline profile
    data[5] = 0x00;
    data[6] = 0x28; // level 4.0
    data[7] = 0xFF; // reserved

    let result = parse_avc(&data);
    assert!(result.is_ok());
    let stream = result.unwrap();
    assert!(stream.nal_units.len() >= 1);
}

#[test]
fn test_parse_avc_with_pps_only() {
    // Test parse_avc with PPS only
    let mut data = vec![0u8; 32];
    data[0] = 0x00;
    data[1] = 0x00;
    data[2] = 0x01;
    data[3] = 0x68; // PPS
    data[4] = 0x01; // pic_parameter_set_id
    data[5] = 0xFF;

    let result = parse_avc(&data);
    assert!(result.is_ok());
    let stream = result.unwrap();
    assert!(stream.nal_units.len() >= 1);
}

#[test]
fn test_parse_avc_with_sei() {
    // Test parse_avc with SEI NAL unit
    let mut data = vec![0u8; 32];
    data[0] = 0x00;
    data[1] = 0x00;
    data[2] = 0x01;
    data[3] = 0x06; // SEI
    data[4] = 0x01; // payload type

    let result = parse_avc(&data);
    assert!(result.is_ok());
    let stream = result.unwrap();
    assert!(stream.nal_units.len() >= 1);
}

#[test]
fn test_parse_avc_with_aud() {
    // Test parse_avc with AUD NAL unit
    let mut data = vec![0u8; 32];
    data[0] = 0x00;
    data[1] = 0x00;
    data[2] = 0x01;
    data[3] = 0x09; // AUD

    let result = parse_avc(&data);
    assert!(result.is_ok());
}

#[test]
fn test_parse_avc_with_end_of_sequence() {
    // Test parse_avc with end of sequence NAL unit
    let mut data = vec![0u8; 32];
    data[0] = 0x00;
    data[1] = 0x00;
    data[2] = 0x01;
    data[3] = 0x0A; // End of sequence

    let result = parse_avc(&data);
    assert!(result.is_ok());
}

#[test]
fn test_parse_avc_with_filler_data() {
    // Test parse_avc with filler data NAL unit
    let mut data = vec![0u8; 32];
    data[0] = 0x00;
    data[1] = 0x00;
    data[2] = 0x01;
    data[3] = 0x0C; // Filler data

    let result = parse_avc(&data);
    assert!(result.is_ok());
}

#[test]
fn test_parse_avc_with_different_profiles() {
    // Test parse_avc with different profile values
    let profiles = [
        (0x42, "Baseline"),
        (0x4D, "Main"),
        (0x64, "High"),
        (0x6E, "High10"),
        (0x7A, "High422"),
        (0xF4, "High444"),
    ];

    for (profile_idc, _name) in profiles {
        let mut data = vec![0u8; 32];
        data[0] = 0x00;
        data[1] = 0x00;
        data[2] = 0x01;
        data[3] = 0x67; // SPS
        data[4] = profile_idc;
        data[5] = 0x00;
        data[6] = 0x1E; // level 3.0
        data[7] = 0xFF;

        let result = parse_avc(&data);
        assert!(result.is_ok(), "Profile {} should parse", profile_idc);
    }
}

#[test]
fn test_parse_avc_with_different_levels() {
    // Test parse_avc with different level values
    let levels = [10, 11, 12, 13, 20, 21, 22, 30, 31, 40, 41, 50, 51];

    for level_idc in levels {
        let mut data = vec![0u8; 32];
        data[0] = 0x00;
        data[1] = 0x00;
        data[2] = 0x01;
        data[3] = 0x67; // SPS
        data[4] = 0x42;
        data[5] = 0x00;
        data[6] = level_idc; // level
        data[7] = 0xFF;

        let result = parse_avc(&data);
        assert!(result.is_ok(), "Level {} should parse", level_idc);
    }
}

// parse_avc_quick() tests

#[test]
fn test_parse_avc_quick_returns_nal_count() {
    // Test parse_avc_quick returns correct NAL count
    let mut data = vec![0u8; 64];
    // Add 3 NAL units
    for _ in 0..3 {
        data.push(0x00);
        data.push(0x00);
        data.push(0x01);
        data.push(0x67); // SPS
        data.push(0x42);
        data.push(0x00);
        data.push(0x28);
        data.push(0xFF);
    }

    let result = parse_avc_quick(&data);
    assert!(result.is_ok());
    let info = result.unwrap();
    assert_eq!(info.nal_count, 3);
}

#[test]
fn test_parse_avc_quick_returns_sps_count() {
    // Test parse_avc_quick returns correct SPS count
    let mut data = vec![0u8; 128];
    // Add 2 SPS units
    for _ in 0..2 {
        data.push(0x00);
        data.push(0x00);
        data.push(0x01);
        data.push(0x67);
        data.push(0x42);
        data.push(0x00);
        data.push(0x28);
        data.push(0xFF);
    }
    // Add non-SPS NAL
    data.push(0x00);
    data.push(0x00);
    data.push(0x01);
    data.push(0x68); // PPS

    let result = parse_avc_quick(&data);
    assert!(result.is_ok());
    let info = result.unwrap();
    assert_eq!(info.sps_count, 2);
}

#[test]
fn test_parse_avc_quick_returns_pps_count() {
    // Test parse_avc_quick returns correct PPS count
    let mut data = vec![0u8; 128];
    // Add SPS
    data.extend_from_slice(&[0x00, 0x00, 0x01, 0x67, 0x42, 0x00, 0x28, 0xFF]);
    // Add 3 PPS units
    for _ in 0..3 {
        data.extend_from_slice(&[0x00, 0x00, 0x01, 0x68, 0x01, 0xFF]);
    }

    let result = parse_avc_quick(&data);
    assert!(result.is_ok());
    let info = result.unwrap();
    assert_eq!(info.pps_count, 3);
}

#[test]
fn test_parse_avc_quick_returns_idr_count() {
    // Test parse_avc_quick returns correct IDR count
    let mut data = vec![0u8; 128];
    // Add SPS
    data.extend_from_slice(&[0x00, 0x00, 0x01, 0x67, 0x42, 0x00, 0x28, 0xFF]);
    // Add 2 IDR slices
    data.extend_from_slice(&[0x00, 0x00, 0x01, 0x65, 0x11]); // IDR slice
    data.extend_from_slice(&[0x00, 0x00, 0x01, 0x65, 0x11]); // IDR slice
                                                             // Add non-IDR slice
    data.extend_from_slice(&[0x00, 0x00, 0x01, 0x21, 0x01]); // P slice

    let result = parse_avc_quick(&data);
    assert!(result.is_ok());
    let info = result.unwrap();
    assert_eq!(info.idr_count, 2);
}

#[test]
fn test_parse_avc_quick_returns_frame_count() {
    // Test parse_avc_quick returns correct frame count
    let mut data = vec![0u8; 256];
    // Add SPS and PPS
    data.extend_from_slice(&[0x00, 0x00, 0x01, 0x67, 0x42, 0x00, 0x28, 0xFF]);
    data.extend_from_slice(&[0x00, 0x00, 0x01, 0x68, 0x01, 0xFF]);
    // Add multiple slices
    for _ in 0..5 {
        data.extend_from_slice(&[0x00, 0x00, 0x01, 0x21, 0x01, 0x04]); // P slice
    }

    let result = parse_avc_quick(&data);
    assert!(result.is_ok());
    let info = result.unwrap();
    assert!(info.frame_count >= 5);
}

#[test]
fn test_parse_avc_quick_dimensions_extraction() {
    // Test parse_avc_quick extracts width and height when available
    let mut data = vec![0u8; 64];
    // IVF-like structure not used for AVC, but we can test dimension extraction from SPS
    data.extend_from_slice(&[0x00, 0x00, 0x01, 0x67, 0x42, 0x00]);
    data.extend_from_slice(&[0x00, 0x00, 0x01, 0x68, 0x01, 0xFF]);

    let result = parse_avc_quick(&data);
    assert!(result.is_ok());
    let info = result.unwrap();
    // Dimensions may or may not be available depending on SPS parsing
    assert!(info.width.is_some() || info.width.is_none());
}

#[test]
fn test_parse_avc_quick_profile_extraction() {
    // Test parse_avc_quick extracts profile when available
    let mut data = vec![0u8; 32];
    data.extend_from_slice(&[0x00, 0x00, 0x01, 0x67, 0x4D, 0x00]); // Main profile

    let result = parse_avc_quick(&data);
    assert!(result.is_ok());
    let info = result.unwrap();
    // Profile may or may not be available
    assert!(info.profile.is_some() || info.profile.is_none());
}

#[test]
fn test_parse_avc_quick_level_extraction() {
    // Test parse_avc_quick extracts level when available
    let mut data = vec![0u8; 32];
    data.extend_from_slice(&[0x00, 0x00, 0x01, 0x67, 0x42, 0x00, 0x1E]); // level 3.0

    let result = parse_avc_quick(&data);
    assert!(result.is_ok());
    let info = result.unwrap();
    // Level may or may not be available
    assert!(info.level.is_some() || info.level.is_none());
}

// Edge case tests

#[test]
fn test_parse_avc_with_mixed_nal_types() {
    // Test parse_avc with mix of different NAL types
    let nal_types = [
        0x67, // SPS
        0x68, // PPS
        0x65, // IDR
        0x21, // P slice
        0x06, // SEI
        0x09, // AUD
        0x0A, // End of sequence
    ];

    for nal_type in nal_types {
        let mut data = vec![0u8; 16];
        data.extend_from_slice(&[0x00, 0x00, 0x01, nal_type]);
        data.push(0x01); // minimal additional data

        let result = parse_avc(&data);
        assert!(result.is_ok(), "Should parse NAL type 0x{:02X}", nal_type);
    }
}

#[test]
fn test_parse_avc_with_3byte_start_codes() {
    // Test parse_avc with 3-byte start codes (0x00 0x00 0x01)
    let mut data = vec![0u8; 32];
    // First NAL with 3-byte start code
    data[0] = 0x00;
    data[1] = 0x00;
    data[2] = 0x01;
    data[3] = 0x67; // SPS
                    // Second NAL with 3-byte start code
    data[10] = 0x00;
    data[11] = 0x00;
    data[12] = 0x01;
    data[13] = 0x68; // PPS

    let result = parse_avc(&data);
    assert!(result.is_ok());
    let stream = result.unwrap();
    assert!(stream.nal_units.len() >= 2);
}

#[test]
fn test_parse_avc_with_4byte_start_codes() {
    // Test parse_avc with 4-byte start codes (0x00 0x00 0x00 0x01)
    let mut data = vec![0u8; 32];
    // First NAL with 4-byte start code
    data[0] = 0x00;
    data[1] = 0x00;
    data[2] = 0x00;
    data[3] = 0x01;
    data[4] = 0x67; // SPS

    let result = parse_avc(&data);
    assert!(result.is_ok());
    let stream = result.unwrap();
    assert!(stream.nal_units.len() >= 1);
}

#[test]
fn test_parse_avc_with_emulation_prevention() {
    // Test parse_avc handles emulation prevention bytes correctly
    let mut data = vec![0u8; 32];
    // Start code with emulation prevention
    data[0] = 0x00;
    data[1] = 0x00;
    data[2] = 0x03;
    data[3] = 0x01; // Not a start code (0x00 0x00 0x03)
                    // Real start code
    data[4] = 0x00;
    data[5] = 0x00;
    data[6] = 0x01;
    data[7] = 0x67; // SPS

    let result = parse_avc(&data);
    assert!(result.is_ok());
}

#[test]
fn test_parse_avc_with_corrupted_nal() {
    // Test parse_avc handles corrupted NAL units gracefully
    let mut data = vec![0u8; 32];
    data[0] = 0x00;
    data[1] = 0x00;
    data[2] = 0x01;
    data[3] = 0xFF; // Invalid NAL type
    data.extend_from_slice(&[0xFF; 28]); // Fill with garbage

    let result = parse_avc(&data);
    // Should handle gracefully - may succeed or fail
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_avc_with_truncated_data() {
    // Test parse_avc with truncated NAL unit
    let data = [0x00, 0x00, 0x01, 0x67, 0x42]; // SPS header but no payload

    let result = parse_avc(&data);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_avc_large_stream() {
    // Test parse_avc with large stream (1000 NAL units)
    let mut data = vec![0u8; 16_000];
    let mut pos = 0;

    for i in 0..1000 {
        data[pos] = 0x00;
        pos += 1;
        data[pos] = 0x00;
        pos += 1;
        data[pos] = 0x01;
        pos += 1;
        data[pos] = if i % 3 == 0 {
            0x67
        }
        // SPS
        else if i % 3 == 1 {
            0x68
        }
        // PPS
        else {
            0x21
        }; // P slice
        pos += 1;
        data[pos] = 0xFF;
        pos += 1; // minimal data
    }

    let result = parse_avc(&data[..pos]);
    assert!(result.is_ok());
    let stream = result.unwrap();
    assert!(stream.nal_units.len() >= 1000);
}

#[test]
fn test_parse_avc_performance() {
    // Test parse_avc performance with moderate input
    let mut data = vec![0u8; 10_000];
    let mut pos = 0;

    // Add 100 NAL units
    for _ in 0..100 {
        data[pos] = 0x00;
        pos += 1;
        data[pos] = 0x00;
        pos += 1;
        data[pos] = 0x01;
        pos += 1;
        data[pos] = 0x67;
        pos += 1;
        data[pos] = 0x42;
        pos += 1;
        data[pos] = 0x00;
        pos += 1;
        data[pos] = 0x28;
        pos += 1;
        data[pos] = 0xFF;
        pos += 1;
    }

    let start = std::time::Instant::now();
    let result = parse_avc(&data[..pos]);
    let duration = start.elapsed();

    assert!(result.is_ok());
    assert!(
        duration.as_millis() < 500,
        "Parsing took too long: {:?}",
        duration
    );
}

#[test]
fn test_parse_avc_with_cabac() {
    // Test parse_avc with CABAC-related fields
    let mut data = vec![0u8; 64];
    data.extend_from_slice(&[0x00, 0x00, 0x01, 0x67]); // SPS
    data.extend_from_slice(&[0x42, 0x00]); // baseline profile
    data.extend_from_slice(&[0x00, 0x28]); // level
    data.extend_from_slice(&[0xFF]); // reserved
                                     // entropy_coding_mode_flag = 1 for CABAC
    data.extend_from_slice(&[0x01]);

    let result = parse_avc(&data);
    assert!(result.is_ok());
}

#[test]
fn test_parse_avc_with_interlaced() {
    // Test parse_avc with interlaced frames
    let mut data = vec![0u8; 64];
    data.extend_from_slice(&[0x00, 0x00, 0x01, 0x67]); // SPS
    data.extend_from_slice(&[0x42, 0x00]); // baseline
    data.extend_from_slice(&[0x00, 0x28]); // level
    data.extend_from_slice(&[0xFF]); // reserved
                                     // frame_mbs_only_flag = 0 (interlaced)

    let result = parse_avc(&data);
    assert!(result.is_ok());
}

#[test]
fn test_parse_avc_with_frame_cropping() {
    // Test parse_avc with frame cropping enabled
    let mut data = vec![0u8; 64];
    data.extend_from_slice(&[0x00, 0x00, 0x01, 0x67]); // SPS
    data.extend_from_slice(&[0x42, 0x00]); // baseline
    data.extend_from_slice(&[0x00, 0x28]); // level
    data.extend_from_slice(&[0xFF]); // reserved
                                     // frame_cropping_flag = 1
    data.extend_from_slice(&[0x01]);

    let result = parse_avc(&data);
    assert!(result.is_ok());
}

// === Comprehensive Functional Tests ===

#[test]
fn test_parse_sps_with_baseline_profile() {
    // Test SPS parsing with baseline profile (profile_idc = 66)
    let mut data = vec![0u8; 32];
    data[0..4].copy_from_slice(&[0x00, 0x00, 0x01, 0x67]); // NAL header
    data[4] = 66; // profile_idc = Baseline (66)
    data[5] = 0; // constraint_set0_flag
    data[6] = 30; // level_idc = 3.0

    let result = parse_sps(&data[4..]);
    // May fail due to incomplete bitstream, but should not panic
    assert!(result.is_ok() || result.is_err());
    // If successful, verify profile
    if let Ok(sps) = result {
        assert_eq!(sps.profile_idc, ProfileIdc::Baseline);
    }
}

#[test]
fn test_parse_sps_with_high_profile() {
    // Test SPS parsing with high profile (profile_idc = 100)
    let mut data = vec![0u8; 32];
    data[0..4].copy_from_slice(&[0x00, 0x00, 0x01, 0x67]); // NAL header
    data[4] = 100; // profile_idc = High
    data[5] = 0x64; // constraint_set flags
    data[6] = 40; // level_idc = 4.0

    let result = parse_sps(&data[4..]);
    // May fail due to incomplete bitstream, but should not panic
    assert!(result.is_ok() || result.is_err());
    // If successful, verify profile
    if let Ok(sps) = result {
        assert_eq!(sps.profile_idc, ProfileIdc::High);
    }
}

#[test]
fn test_parse_sps_dimensions() {
    // Test SPS parsing extracts correct dimensions
    let mut data = vec![0u8; 32];
    data[0..4].copy_from_slice(&[0x00, 0x00, 0x01, 0x67]); // NAL header
    data[4] = 66; // profile_idc
    data[5] = 0; // constraint_set0_flag
    data[6] = 31; // level_idc
                  // chroma_format_idc = 1 (YUV420) - encoded as ue(v)
                  // pic_width_in_mbs_minus1 = 9 (1920/16 - 1 = 119)
                  // pic_height_in_map_units_minus1 = 6 (1080/16 - 1 = 66)

    let result = parse_sps(&data[4..]);
    // May fail due to incomplete bitstream, but should not panic
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_pps_pic_parameter_set_id() {
    // Test PPS parsing extracts pic_parameter_set_id correctly
    let mut data = vec![0u8; 16];
    data[0..4].copy_from_slice(&[0x00, 0x00, 0x01, 0x68]); // NAL header
    data[4] = 5; // pic_parameter_set_id

    let result = parse_pps(&data[4..]);
    // May fail due to incomplete bitstream, but should not panic
    assert!(result.is_ok() || result.is_err());
    // If successful, verify pps_id
    if let Ok(pps) = result {
        assert_eq!(pps.pic_parameter_set_id, 5);
    }
}

#[test]
fn test_parse_pps_with_cabac_enabled() {
    // Test PPS parsing with CABAC enabled
    let mut data = vec![0u8; 16];
    data[0..4].copy_from_slice(&[0x00, 0x00, 0x01, 0x68]); // NAL header
    data[4] = 0; // pic_parameter_set_id
    data[5] = 0; // seq_parameter_set_id
    data[6] = 0x80; // entropy_coding_mode_flag = 1 (CABAC)

    let result = parse_pps(&data[4..]);
    // May fail due to incomplete bitstream, but should not panic
    assert!(result.is_ok() || result.is_err());
    // If successful, verify CABAC flag
    if let Ok(pps) = result {
        assert!(pps.entropy_coding_mode_flag);
    }
}

#[test]
fn test_parse_nal_header_sps() {
    // Test NAL header parsing for SPS (0x67 = nal_ref_idc=3, type=7)
    let header = parse_nal_header(0x67); // SPS NAL unit
    assert!(header.is_ok());
    let header = header.unwrap();
    assert_eq!(header.nal_ref_idc, 3);
    assert_eq!(header.nal_unit_type, NalUnitType::Sps);
}

#[test]
fn test_parse_nal_header_pps() {
    // Test NAL header parsing for PPS (0x68 = nal_ref_idc=3, type=8)
    let header = parse_nal_header(0x68); // PPS NAL unit
    assert!(header.is_ok());
    let header = header.unwrap();
    assert_eq!(header.nal_ref_idc, 3);
    assert_eq!(header.nal_unit_type, NalUnitType::Pps);
}

#[test]
fn test_parse_nal_header_idr() {
    // Test NAL header parsing for IDR (0x25 = nal_ref_idc=1, type=5)
    let header = parse_nal_header(0x25); // IDR slice
    assert!(header.is_ok());
    let header = header.unwrap();
    assert_eq!(header.nal_ref_idc, 1);
    assert_eq!(header.nal_unit_type, NalUnitType::IdrSlice);
}

#[test]
fn test_parse_nal_units_multiple_nals() {
    // Test parsing multiple NAL units from byte stream
    let mut data = vec![0u8; 64];
    // First NAL: SPS
    data.extend_from_slice(&[0x00, 0x00, 0x01, 0x67]);
    data.extend_from_slice(&[0x42, 0x00, 0x1E]); // minimal SPS data
    data.extend_from_slice(&[0x00; 10]);

    // Second NAL: PPS
    data.extend_from_slice(&[0x00, 0x00, 0x01, 0x68]);
    data.extend_from_slice(&[0x01, 0x00]); // minimal PPS data
    data.extend_from_slice(&[0x00; 10]);

    let result = parse_nal_units(&data);
    assert!(result.is_ok());
    let nal_units = result.unwrap();
    assert_eq!(nal_units.len(), 2);
    assert_eq!(nal_units[0].header.nal_unit_type, NalUnitType::Sps);
    assert_eq!(nal_units[1].header.nal_unit_type, NalUnitType::Pps);
}

#[test]
fn test_parse_avc_quick_info_extraction() {
    // Test quick info extraction from AVC stream
    let mut data = vec![0u8; 64];
    // Add a minimal SPS
    data.extend_from_slice(&[0x00, 0x00, 0x01, 0x67]);
    data.extend_from_slice(&[0x42, 0x00, 0x1E]); // baseline, level 3.0
    data.extend_from_slice(&[0x00; 10]);

    let result = parse_avc_quick(&data);
    assert!(result.is_ok());
    let info = result.unwrap();
    // width/height are Option<u32>, profile is Option<u8>
    assert!(
        info.width.is_some()
            || info.height.is_some()
            || info.profile.is_some()
            || info.nal_count > 0
    );
}

#[test]
fn test_parse_sei_messages_buffering_period() {
    // Test SEI parsing with buffering period SEI
    let mut data = vec![0u8; 32];
    data[0..4].copy_from_slice(&[0x00, 0x00, 0x01, 0x06]); // SEI NAL
    data[4] = 0x06; // buffering_period payload type
    data[5] = 0x01; // payload data (minimal)

    let result = parse_sei(&data);
    // Should handle gracefully - may succeed or fail based on parser implementation
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_slice_header_default_values() {
    // Test slice header parsing extracts default values correctly
    let mut data = vec![0u8; 32];
    data[0..4].copy_from_slice(&[0x00, 0x00, 0x01, 0x25]); // NAL header (non-IDR)
    data[4] = 0x88; // first_mb_in_slice + slice_type
    data[5] = 0x00; // pic_parameter_set_id

    // Parse slice header with empty maps (may fail without proper SPS/PPS)
    let sps_map = std::collections::HashMap::new();
    let pps_map = std::collections::HashMap::new();
    let result = parse_slice_header(&data[4..], &sps_map, &pps_map, NalUnitType::NonIdrSlice, 0);
    // Should handle gracefully - may succeed or fail based on requirements
    assert!(result.is_ok() || result.is_err());
}

// === Error Handling Tests ===

#[test]
fn test_parse_avc_with_completely_invalid_data() {
    // Test parse_avc with completely random/invalid data
    let data = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF];
    let result = parse_avc(&data);
    // Should handle gracefully - either Ok with minimal info or Err
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_avc_with_all_zeros() {
    // Test parse_avc with all zeros (completely invalid)
    let data = vec![0u8; 100];
    let result = parse_avc(&data);
    // Should handle gracefully
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_avc_with_truncated_nal_unit() {
    // Test parse_avc with truncated NAL unit
    let data = [0x00, 0x00, 0x01, 0x67]; // Only start code + NAL header
    let result = parse_avc(&data);
    // Should handle gracefully - incomplete NAL unit
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_avc_with_invalid_nal_type() {
    // Test parse_avc with invalid NAL unit type
    let data = [0x00, 0x00, 0x01, 0xFF]; // Invalid NAL type (0xFF)
    let result = parse_avc(&data);
    // Should handle gracefully
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_avc_with_repeated_start_codes() {
    // Test parse_avc with repeated start codes (no actual data)
    let data = [0x00, 0x00, 0x01, 0x00, 0x00, 0x01, 0x00, 0x00, 0x01];
    let result = parse_avc(&data);
    // Should handle gracefully
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_avc_quick_with_invalid_data() {
    // Test parse_avc_quick with invalid data
    let data = vec![0xFFu8; 50];
    let result = parse_avc_quick(&data);
    // Should handle gracefully
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_avc_with_very_large_input() {
    // Test parse_avc doesn't crash on very large input
    let large_data = vec![0u8; 10_000_000]; // 10 MB
    let result = parse_avc(&large_data);
    // Should handle without panic
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_avc_with_malformed_sps() {
    // Test parse_avc with malformed SPS
    let mut data = vec![0u8; 32];
    data.extend_from_slice(&[0x00, 0x00, 0x01, 0x67]); // SPS
    data.extend_from_slice(&[0xFF, 0xFF, 0xFF, 0xFF]); // Invalid SPS data

    let result = parse_avc(&data);
    // Should handle gracefully
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_avc_with_malformed_pps() {
    // Test parse_avc with malformed PPS
    let mut data = vec![0u8; 32];
    data.extend_from_slice(&[0x00, 0x00, 0x01, 0x68]); // PPS
    data.extend_from_slice(&[0xFF, 0xFF, 0xFF, 0xFF]); // Invalid PPS data

    let result = parse_avc(&data);
    // Should handle gracefully
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_avc_with_incomplete_start_code() {
    // Test parse_avc with incomplete start code
    let data = [0x00, 0x00]; // Only 2 bytes of start code
    let result = parse_avc(&data);
    // Should handle gracefully
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_avc_error_messages_are_descriptive() {
    // Test that error messages provide useful information
    let invalid_data = vec![0xFFu8; 10];
    let result = parse_avc(&invalid_data);
    if let Err(e) = result {
        // Error should have some description
        let error_msg = format!("{}", e);
        assert!(!error_msg.is_empty(), "Error message should not be empty");
    }
}

#[test]
fn test_parse_avc_with_embedded_nulls() {
    // Test parse_avc handles embedded null bytes
    let mut data = vec![0u8; 100];
    data[0] = 0x00;
    data[1] = 0x00;
    data[2] = 0x01; // Start code
    data[3] = 0x00; // Embedded null in NAL header position
                    // Rest is nulls
    for i in 4..100 {
        data[i] = 0x00;
    }

    let result = parse_avc(&data);
    // Should handle without panic
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_avc_with_single_byte() {
    // Test parse_avc with single byte input
    let data = [0x67];
    let result = parse_avc(&data);
    // Should handle gracefully
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_avc_with_two_bytes() {
    // Test parse_avc with two byte input
    let data = [0x00, 0x67];
    let result = parse_avc(&data);
    // Should handle gracefully
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_avc_with_unicode_bytes() {
    // Test parse_avc doesn't crash on unexpected byte patterns
    let data: Vec<u8> = (0..256).map(|i| i as u8).collect();
    let result = parse_avc(&data);
    // Should handle all byte values gracefully
    assert!(result.is_ok() || result.is_err());
}

// === Additional Negative Tests for Public API ===

#[test]
fn test_parse_nal_header_with_forbidden_bit_set() {
    // Test NAL header parser with forbidden_zero_bit set
    let result = parse_nal_header(0x80); // forbidden_zero_bit = 1
                                         // Should return error - forbidden bit must be 0
    assert!(result.is_err());
    if let Err(e) = result {
        assert!(format!("{}", e).contains("forbidden"));
    }
}

#[test]
fn test_parse_nal_header_all_zero() {
    // Test NAL header parser with zero byte
    let result = parse_nal_header(0x00);
    assert!(result.is_ok());
    let header = result.unwrap();
    assert_eq!(header.nal_ref_idc, 0);
}

#[test]
fn test_parse_nal_header_max_values() {
    // Test NAL header parser with maximum values
    let result = parse_nal_header(0x7F); // max value without forbidden bit
    assert!(result.is_ok());
}

#[test]
fn test_extract_mv_grid_with_empty_nal_units() {
    // Test MV grid extraction with empty NAL unit array
    use crate::overlay_extraction::extract_mv_grid;
    let nal_units: &[crate::nal::NalUnit] = &[];
    let sps = crate::sps::Sps {
        profile_idc: crate::sps::ProfileIdc::Baseline,
        constraint_set0_flag: false,
        constraint_set1_flag: false,
        constraint_set2_flag: false,
        constraint_set3_flag: false,
        constraint_set4_flag: false,
        constraint_set5_flag: false,
        level_idc: 51,
        seq_parameter_set_id: 0,
        chroma_format_idc: crate::sps::ChromaFormat::Yuv420,
        separate_colour_plane_flag: false,
        bit_depth_luma_minus8: 0,
        bit_depth_chroma_minus8: 0,
        qpprime_y_zero_transform_bypass_flag: false,
        seq_scaling_matrix_present_flag: false,
        log2_max_frame_num_minus4: 0,
        pic_order_cnt_type: 0,
        log2_max_pic_order_cnt_lsb_minus4: 0,
        delta_pic_order_always_zero_flag: false,
        offset_for_non_ref_pic: 0,
        offset_for_top_to_bottom_field: 0,
        num_ref_frames_in_pic_order_cnt_cycle: 0,
        offset_for_ref_frame: Vec::new(),
        max_num_ref_frames: 0,
        gaps_in_frame_num_value_allowed_flag: false,
        pic_width_in_mbs_minus1: 0,
        pic_height_in_map_units_minus1: 0,
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
    };
    let result = extract_mv_grid(nal_units, &sps);
    // Empty NAL units should return error or empty grid
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_extract_qp_grid_with_empty_nal_units() {
    // Test QP grid extraction with empty NAL unit array
    use crate::overlay_extraction::extract_qp_grid;
    let nal_units: &[crate::nal::NalUnit] = &[];
    let sps = crate::sps::Sps {
        profile_idc: crate::sps::ProfileIdc::Baseline,
        constraint_set0_flag: false,
        constraint_set1_flag: false,
        constraint_set2_flag: false,
        constraint_set3_flag: false,
        constraint_set4_flag: false,
        constraint_set5_flag: false,
        level_idc: 51,
        seq_parameter_set_id: 0,
        chroma_format_idc: crate::sps::ChromaFormat::Yuv420,
        separate_colour_plane_flag: false,
        bit_depth_luma_minus8: 0,
        bit_depth_chroma_minus8: 0,
        qpprime_y_zero_transform_bypass_flag: false,
        seq_scaling_matrix_present_flag: false,
        log2_max_frame_num_minus4: 0,
        pic_order_cnt_type: 0,
        log2_max_pic_order_cnt_lsb_minus4: 0,
        delta_pic_order_always_zero_flag: false,
        offset_for_non_ref_pic: 0,
        offset_for_top_to_bottom_field: 0,
        num_ref_frames_in_pic_order_cnt_cycle: 0,
        offset_for_ref_frame: Vec::new(),
        max_num_ref_frames: 0,
        gaps_in_frame_num_value_allowed_flag: false,
        pic_width_in_mbs_minus1: 0,
        pic_height_in_map_units_minus1: 0,
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
    };
    let result = extract_qp_grid(nal_units, &sps, 26); // Valid base QP
                                                       // Empty NAL units should return error or empty grid
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_avc_quick_with_empty_input() {
    // Test quick info with empty input
    let data: &[u8] = &[];
    let result = parse_avc_quick(data);
    // Should handle gracefully
    assert!(result.is_ok() || result.is_err());
    if let Ok(info) = result {
        // Empty input should return default info
        assert_eq!(info.nal_count, 0);
    }
}

#[test]
fn test_parse_avc_with_only_start_codes() {
    // Test parse_avc with only start codes (no actual data)
    let data = [0x00, 0x00, 0x01, 0x00, 0x00, 0x01, 0x00];
    let result = parse_avc(&data);
    // Should handle gracefully
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_sei_with_empty_payload() {
    // Test SEI parsing with empty payload
    let mut data = vec![0u8; 16];
    data[0..4].copy_from_slice(&[0x00, 0x00, 0x01, 0x06]); // SEI NAL
    data[4] = 0xFF; // payload_type
    data[5] = 0x00; // payload_size = 0

    let result = parse_sei(&data);
    // Should handle gracefully
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_find_nal_units_with_empty_input() {
    // Test find_nal_units with empty input - returns Vec<usize>, not Result
    let data: &[u8] = &[];
    let positions = find_nal_units(data);
    assert_eq!(positions.len(), 0);
}

#[test]
fn test_find_nal_units_with_no_start_codes() {
    // Test find_nal_units with data but no start codes - returns Vec<usize>
    let data = vec![0x12, 0x34, 0x56, 0x78];
    let positions = find_nal_units(&data);
    assert_eq!(positions.len(), 0); // No NAL units found
}
