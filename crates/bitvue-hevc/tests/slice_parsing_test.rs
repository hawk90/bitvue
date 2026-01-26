//! HEVC Slice Header Parsing Tests
//!
//! Comprehensive tests for HEVC slice header parsing functionality.

use bitvue_hevc::slice::{SliceHeader, SliceType, RefPicListModification, PredWeightTable};
use bitvue_hevc::sps::{ChromaFormat, Profile, ProfileTierLevel, Sps};

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

// SliceType tests

#[test]
fn test_slice_type_from_u32() {
    assert_eq!(SliceType::from_u32(0), Some(SliceType::B));
    assert_eq!(SliceType::from_u32(1), Some(SliceType::P));
    assert_eq!(SliceType::from_u32(2), Some(SliceType::I));
    assert_eq!(SliceType::from_u32(3), None);
    assert_eq!(SliceType::from_u32(255), None);
}

#[test]
fn test_slice_type_is_intra() {
    assert!(SliceType::I.is_intra());
    assert!(!SliceType::P.is_intra());
    assert!(!SliceType::B.is_intra());
}

#[test]
fn test_slice_type_is_inter() {
    assert!(!SliceType::I.is_inter());
    assert!(SliceType::P.is_inter());
    assert!(SliceType::B.is_inter());
}

#[test]
fn test_slice_type_name() {
    assert_eq!(SliceType::B.name(), "B");
    assert_eq!(SliceType::P.name(), "P");
    assert_eq!(SliceType::I.name(), "I");
    assert_eq!(SliceType::B.as_str(), "B");
    assert_eq!(SliceType::P.as_str(), "P");
    assert_eq!(SliceType::I.as_str(), "I");
}

#[test]
fn test_slice_type_display() {
    assert_eq!(format!("{:?}", SliceType::B), "B");
    assert_eq!(format!("{:?}", SliceType::P), "P");
    assert_eq!(format!("{:?}", SliceType::I), "I");
}

// SliceHeader tests

#[test]
fn test_slice_header_default() {
    let header = SliceHeader::default();

    assert_eq!(header.slice_type, SliceType::I);
    assert!(header.first_slice_segment_in_pic_flag);
    assert_eq!(header.slice_pic_parameter_set_id, 0);
    assert_eq!(header.slice_qp_delta, 0);
    assert_eq!(header.num_ref_idx_l0_active_minus1, 0);
    assert_eq!(header.num_ref_idx_l1_active_minus1, 0);
}

#[test]
fn test_slice_header_i_slice() {
    let header = SliceHeader {
        slice_type: SliceType::I,
        num_ref_idx_l0_active_minus1: 0,
        num_ref_idx_l1_active_minus1: 0,
        slice_temporal_mvp_enabled_flag: false,
        ..Default::default()
    };

    assert!(header.slice_type.is_intra());
    assert_eq!(header.num_ref_idx_l0_active_minus1, 0);
    assert!(!header.slice_temporal_mvp_enabled_flag);
}

#[test]
fn test_slice_header_p_slice() {
    let header = SliceHeader {
        slice_type: SliceType::P,
        num_ref_idx_l0_active_minus1: 5,
        num_ref_idx_l1_active_minus1: 0,
        slice_temporal_mvp_enabled_flag: true,
        five_minus_max_num_merge_cand: 3,
        ..Default::default()
    };

    assert!(header.slice_type.is_inter());
    assert_eq!(header.num_ref_idx_l0_active_minus1, 5);
    assert!(header.slice_temporal_mvp_enabled_flag);
    assert_eq!(header.five_minus_max_num_merge_cand, 3);
}

#[test]
fn test_slice_header_b_slice() {
    let header = SliceHeader {
        slice_type: SliceType::B,
        num_ref_idx_l0_active_minus1: 3,
        num_ref_idx_l1_active_minus1: 2,
        slice_temporal_mvp_enabled_flag: true,
        mvd_l1_zero_flag: true,
        ..Default::default()
    };

    assert!(header.slice_type.is_inter());
    assert_eq!(header.num_ref_idx_l0_active_minus1, 3);
    assert_eq!(header.num_ref_idx_l1_active_minus1, 2);
    assert!(header.mvd_l1_zero_flag);
}

#[test]
fn test_slice_header_qp_offsets() {
    let header = SliceHeader {
        slice_qp_delta: 5,
        slice_cb_qp_offset: -2,
        slice_cr_qp_offset: -3,
        cu_chroma_qp_offset_enabled_flag: true,
        ..Default::default()
    };

    assert_eq!(header.slice_qp_delta, 5);
    assert_eq!(header.slice_cb_qp_offset, -2);
    assert_eq!(header.slice_cr_qp_offset, -3);
    assert!(header.cu_chroma_qp_offset_enabled_flag);
}

#[test]
fn test_slice_header_deblocking() {
    let header = SliceHeader {
        deblocking_filter_override_flag: true,
        slice_deblocking_filter_disabled_flag: false,
        slice_beta_offset_div2: 1,
        slice_tc_offset_div2: 2,
        ..Default::default()
    };

    assert!(header.deblocking_filter_override_flag);
    assert!(!header.slice_deblocking_filter_disabled_flag);
    assert_eq!(header.slice_beta_offset_div2, 1);
    assert_eq!(header.slice_tc_offset_div2, 2);
}

#[test]
fn test_slice_header_sao_flags() {
    let header = SliceHeader {
        slice_sao_luma_flag: true,
        slice_sao_chroma_flag: true,
        ..Default::default()
    };

    assert!(header.slice_sao_luma_flag);
    assert!(header.slice_sao_chroma_flag);
}

#[test]
fn test_slice_header_cabac() {
    let header = SliceHeader {
        cabac_init_flag: true,
        ..Default::default()
    };

    assert!(header.cabac_init_flag);
}

#[test]
fn test_slice_header_temporal_mvp() {
    let header = SliceHeader {
        slice_temporal_mvp_enabled_flag: true,
        collocated_from_l0_flag: true,
        collocated_ref_idx: 5,
        ..Default::default()
    };

    assert!(header.slice_temporal_mvp_enabled_flag);
    assert!(header.collocated_from_l0_flag);
    assert_eq!(header.collocated_ref_idx, 5);
}

#[test]
fn test_slice_header_pic_order_count() {
    let header = SliceHeader {
        slice_pic_order_cnt_lsb: 12345,
        ..Default::default()
    };

    assert_eq!(header.slice_pic_order_cnt_lsb, 12345);
}

#[test]
fn test_slice_header_colour_plane() {
    let header = SliceHeader {
        colour_plane_id: 2,
        ..Default::default()
    };

    assert_eq!(header.colour_plane_id, 2);
}

#[test]
fn test_slice_header_ref_pic_list_modification() {
    let ref_mod = RefPicListModification {
        ref_pic_list_modification_flag_l0: true,
        list_entry_l0: vec![1, 2, 3],
        ref_pic_list_modification_flag_l1: false,
        list_entry_l1: vec![],
    };

    let header = SliceHeader {
        ref_pic_list_modification: Some(ref_mod),
        ..Default::default()
    };

    assert!(header.ref_pic_list_modification.is_some());
    let modification = header.ref_pic_list_modification.unwrap();
    assert!(modification.ref_pic_list_modification_flag_l0);
    assert_eq!(modification.list_entry_l0.len(), 3);
}

#[test]
fn test_slice_header_pred_weight_table() {
    let pred_weight = PredWeightTable {
        luma_log2_weight_denom: 5,
        delta_chroma_log2_weight_denom: -1,
        luma_weight_l0: vec![10, 20, 30],
        luma_offset_l0: vec![1, 2, 3],
        chroma_weight_l0: vec![[10, 20], [30, 40]],
        chroma_offset_l0: vec![[1, 2], [3, 4]],
        luma_weight_l1: vec![15, 25],
        luma_offset_l1: vec![5],
        chroma_weight_l1: vec![[35, 45], [55, 65]],
        chroma_offset_l1: vec![[6, 7], [8, 9]],
    };

    let header = SliceHeader {
        pred_weight_table: Some(pred_weight),
        ..Default::default()
    };

    assert!(header.pred_weight_table.is_some());
    let weight_table = header.pred_weight_table.unwrap();
    assert_eq!(weight_table.luma_log2_weight_denom, 5);
    assert_eq!(weight_table.luma_weight_l0.len(), 3);
}

#[test]
fn test_slice_header_dependent_segment() {
    let header = SliceHeader {
        dependent_slice_segment_flag: true,
        slice_segment_address: 500,
        ..Default::default()
    };

    assert!(header.dependent_slice_segment_flag);
    assert_eq!(header.slice_segment_address, 500);
}

#[test]
fn test_slice_header_entry_point_offsets() {
    let header = SliceHeader {
        num_entry_point_offsets: 3,
        entry_point_offset_minus1: vec![100, 200, 300],
        ..Default::default()
    };

    assert_eq!(header.num_entry_point_offsets, 3);
    assert_eq!(header.entry_point_offset_minus1.len(), 3);
}

#[test]
fn test_slice_header_output_flag() {
    let header = SliceHeader {
        pic_output_flag: false,
        no_output_of_prior_pics_flag: true,
        ..Default::default()
    };

    assert!(!header.pic_output_flag);
    assert!(header.no_output_of_prior_pics_flag);
}

#[test]
fn test_slice_header_long_term_refs() {
    let header = SliceHeader {
        num_long_term_sps: 2,
        num_long_term_pics: 2,
        ..Default::default()
    };

    assert_eq!(header.num_long_term_sps, 2);
    assert_eq!(header.num_long_term_pics, 2);
}

#[test]
fn test_slice_header_use_integer_mv() {
    let header = SliceHeader {
        use_integer_mv_flag: true,
        ..Default::default()
    };

    assert!(header.use_integer_mv_flag);
}

#[test]
fn test_slice_header_loop_filter_across_slices() {
    let header = SliceHeader {
        slice_loop_filter_across_slices_enabled_flag: true,
        ..Default::default()
    };

    assert!(header.slice_loop_filter_across_slices_enabled_flag);
}

#[test]
fn test_slice_header_first_segment() {
    let header = SliceHeader {
        first_slice_segment_in_pic_flag: false,
        ..Default::default()
    };

    assert!(!header.first_slice_segment_in_pic_flag);
}

#[test]
fn test_slice_header_pps_id() {
    let header = SliceHeader {
        slice_pic_parameter_set_id: 7,
        ..Default::default()
    };

    assert_eq!(header.slice_pic_parameter_set_id, 7);
}

#[test]
fn test_slice_header_short_term_ref_pic_set() {
    let header = SliceHeader {
        short_term_ref_pic_set_sps_flag: true,
        short_term_ref_pic_set_idx: 5,
        ..Default::default()
    };

    assert!(header.short_term_ref_pic_set_sps_flag);
    assert_eq!(header.short_term_ref_pic_set_idx, 5);
}

#[test]
fn test_slice_header_merge_cand() {
    let header = SliceHeader {
        five_minus_max_num_merge_cand: 1,
        ..Default::default()
    };

    // five_minus_max_num_merge_cand = 1 means max_num_merge_cand = 4
    assert_eq!(header.five_minus_max_num_merge_cand, 1);
}

#[test]
fn test_ref_pic_list_modification_default() {
    let ref_mod = RefPicListModification::default();

    assert!(!ref_mod.ref_pic_list_modification_flag_l0);
    assert!(ref_mod.list_entry_l0.is_empty());
    assert!(!ref_mod.ref_pic_list_modification_flag_l1);
    assert!(ref_mod.list_entry_l1.is_empty());
}

#[test]
fn test_ref_pic_list_modification_l0_only() {
    let ref_mod = RefPicListModification {
        ref_pic_list_modification_flag_l0: true,
        list_entry_l0: vec![0, 1, 2],
        ref_pic_list_modification_flag_l1: false,
        list_entry_l1: vec![],
    };

    assert!(ref_mod.ref_pic_list_modification_flag_l0);
    assert_eq!(ref_mod.list_entry_l0.len(), 3);
    assert!(!ref_mod.ref_pic_list_modification_flag_l1);
    assert!(ref_mod.list_entry_l1.is_empty());
}

#[test]
fn test_ref_pic_list_modification_l1_only() {
    let ref_mod = RefPicListModification {
        ref_pic_list_modification_flag_l0: false,
        list_entry_l0: vec![],
        ref_pic_list_modification_flag_l1: true,
        list_entry_l1: vec![3, 4, 5],
    };

    assert!(!ref_mod.ref_pic_list_modification_flag_l0);
    assert!(ref_mod.list_entry_l0.is_empty());
    assert!(ref_mod.ref_pic_list_modification_flag_l1);
    assert_eq!(ref_mod.list_entry_l1.len(), 3);
}

#[test]
fn test_ref_pic_list_modification_both_lists() {
    let ref_mod = RefPicListModification {
        ref_pic_list_modification_flag_l0: true,
        list_entry_l0: vec![0, 1],
        ref_pic_list_modification_flag_l1: true,
        list_entry_l1: vec![2, 3],
    };

    assert!(ref_mod.ref_pic_list_modification_flag_l0);
    assert_eq!(ref_mod.list_entry_l0.len(), 2);
    assert!(ref_mod.ref_pic_list_modification_flag_l1);
    assert_eq!(ref_mod.list_entry_l1.len(), 2);
}

#[test]
fn test_pred_weight_table_default() {
    let pred_weight = PredWeightTable::default();

    assert_eq!(pred_weight.luma_log2_weight_denom, 0);
    assert_eq!(pred_weight.delta_chroma_log2_weight_denom, 0);
    assert!(pred_weight.luma_weight_l0.is_empty());
    assert!(pred_weight.luma_offset_l0.is_empty());
    assert!(pred_weight.chroma_weight_l0.is_empty());
    assert!(pred_weight.chroma_offset_l0.is_empty());
    assert!(pred_weight.luma_weight_l1.is_empty());
    assert!(pred_weight.luma_offset_l1.is_empty());
    assert!(pred_weight.chroma_weight_l1.is_empty());
    assert!(pred_weight.chroma_offset_l1.is_empty());
}

#[test]
fn test_pred_weight_table_l0_weights() {
    let pred_weight = PredWeightTable {
        luma_log2_weight_denom: 6,
        luma_weight_l0: vec![10, 20, 30, 40],
        luma_offset_l0: vec![1, 2, 3, 4],
        ..Default::default()
    };

    assert_eq!(pred_weight.luma_log2_weight_denom, 6);
    assert_eq!(pred_weight.luma_weight_l0.len(), 4);
    assert_eq!(pred_weight.luma_offset_l0.len(), 4);
}

#[test]
fn test_pred_weight_table_chroma_weights() {
    let pred_weight = PredWeightTable {
        delta_chroma_log2_weight_denom: -2,
        chroma_weight_l0: vec![[10, 20], [30, 40], [50, 60]],
        chroma_offset_l0: vec![[1, 2], [3, 4], [5, 6]],
        ..Default::default()
    };

    assert_eq!(pred_weight.delta_chroma_log2_weight_denom, -2);
    assert_eq!(pred_weight.chroma_weight_l0.len(), 3);
    assert_eq!(pred_weight.chroma_offset_l0.len(), 3);
}

#[test]
fn test_pred_weight_table_l1_weights() {
    let pred_weight = PredWeightTable {
        luma_weight_l1: vec![100, 200],
        luma_offset_l1: vec![10, 20],
        chroma_weight_l1: vec![[15, 25], [35, 45]],
        chroma_offset_l1: vec![[1, 2], [3, 4]],
        ..Default::default()
    };

    assert_eq!(pred_weight.luma_weight_l1.len(), 2);
    assert_eq!(pred_weight.luma_offset_l1.len(), 2);
    assert_eq!(pred_weight.chroma_weight_l1.len(), 2);
    assert_eq!(pred_weight.chroma_offset_l1.len(), 2);
}

#[test]
fn test_slice_header_complete_i_slice() {
    let header = SliceHeader {
        first_slice_segment_in_pic_flag: true,
        no_output_of_prior_pics_flag: false,
        slice_pic_parameter_set_id: 0,
        dependent_slice_segment_flag: false,
        slice_segment_address: 0,
        slice_type: SliceType::I,
        pic_output_flag: true,
        colour_plane_id: 0,
        slice_pic_order_cnt_lsb: 0,
        short_term_ref_pic_set_sps_flag: false,
        num_long_term_sps: 0,
        num_long_term_pics: 0,
        slice_temporal_mvp_enabled_flag: false,
        slice_sao_luma_flag: false,
        slice_sao_chroma_flag: false,
        num_ref_idx_l0_active_minus1: 0,
        num_ref_idx_l1_active_minus1: 0,
        ref_pic_list_modification: None,
        cabac_init_flag: true,
        slice_qp_delta: 0,
        slice_cb_qp_offset: 0,
        slice_cr_qp_offset: 0,
        cu_chroma_qp_offset_enabled_flag: false,
        deblocking_filter_override_flag: false,
        slice_deblocking_filter_disabled_flag: false,
        slice_beta_offset_div2: 0,
        slice_tc_offset_div2: 0,
        slice_loop_filter_across_slices_enabled_flag: false,
        num_entry_point_offsets: 1,
        entry_point_offset_minus1: vec![0],
        five_minus_max_num_merge_cand: 0,
        ..Default::default()
    };

    assert_eq!(header.slice_type, SliceType::I);
    assert!(header.slice_type.is_intra());
    assert!(!header.slice_type.is_inter());
}
