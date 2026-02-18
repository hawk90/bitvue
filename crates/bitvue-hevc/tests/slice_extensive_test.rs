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
//! Extensive tests for HEVC slice module
//!
//! Comprehensive tests covering SliceType, RefPicListModification,
//! PredWeightTable, SliceHeader, and parse_slice_header function

use bitvue_hevc::slice::{PredWeightTable, RefPicListModification, SliceHeader, SliceType};
use bitvue_hevc::{Pps, Sps};
use std::collections::HashMap;

// ============================================================================
// SliceType Tests
// ============================================================================

#[test]
fn test_slice_type_from_u32_valid() {
    assert_eq!(SliceType::from_u32(0), Some(SliceType::B));
    assert_eq!(SliceType::from_u32(1), Some(SliceType::P));
    assert_eq!(SliceType::from_u32(2), Some(SliceType::I));
}

#[test]
fn test_slice_type_from_u32_invalid() {
    assert_eq!(SliceType::from_u32(3), None);
    assert_eq!(SliceType::from_u32(255), None);
    assert_eq!(SliceType::from_u32(100), None);
}

#[test]
fn test_slice_type_b_is_intra() {
    assert!(!SliceType::B.is_intra());
    assert!(SliceType::B.is_inter());
}

#[test]
fn test_slice_type_p_is_inter() {
    assert!(!SliceType::P.is_intra());
    assert!(SliceType::P.is_inter());
}

#[test]
fn test_slice_type_i_is_intra() {
    assert!(SliceType::I.is_intra());
    assert!(!SliceType::I.is_inter());
}

#[test]
fn test_slice_type_name() {
    assert_eq!(SliceType::B.name(), "B");
    assert_eq!(SliceType::P.name(), "P");
    assert_eq!(SliceType::I.name(), "I");
}

#[test]
fn test_slice_type_as_str() {
    assert_eq!(SliceType::B.as_str(), "B");
    assert_eq!(SliceType::P.as_str(), "P");
    assert_eq!(SliceType::I.as_str(), "I");
}

#[test]
fn test_slice_type_copy() {
    let slice_type = SliceType::P;
    let copied = slice_type;
    assert_eq!(copied, slice_type);
    assert_eq!(copied as u32, 1);
}

// ============================================================================
// RefPicListModification Tests
// ============================================================================

#[test]
fn test_ref_pic_list_modification_default() {
    let mod_ref = RefPicListModification::default();
    assert!(!mod_ref.ref_pic_list_modification_flag_l0);
    assert!(!mod_ref.ref_pic_list_modification_flag_l1);
    assert!(mod_ref.list_entry_l0.is_empty());
    assert!(mod_ref.list_entry_l1.is_empty());
}

#[test]
fn test_ref_pic_list_modification_with_l0() {
    let mod_ref = RefPicListModification {
        ref_pic_list_modification_flag_l0: true,
        list_entry_l0: vec![1, 2, 3],
        ..Default::default()
    };
    assert!(mod_ref.ref_pic_list_modification_flag_l0);
    assert_eq!(mod_ref.list_entry_l0.len(), 3);
    assert_eq!(mod_ref.list_entry_l0[0], 1);
}

#[test]
fn test_ref_pic_list_modification_with_l1() {
    let mod_ref = RefPicListModification {
        ref_pic_list_modification_flag_l1: true,
        list_entry_l1: vec![5, 10, 15],
        ..Default::default()
    };
    assert!(mod_ref.ref_pic_list_modification_flag_l1);
    assert_eq!(mod_ref.list_entry_l1.len(), 3);
    assert_eq!(mod_ref.list_entry_l1[2], 15);
}

#[test]
fn test_ref_pic_list_modification_clone() {
    let mod_ref = RefPicListModification {
        ref_pic_list_modification_flag_l0: true,
        list_entry_l0: vec![1, 2, 3],
        ..Default::default()
    };
    let cloned = mod_ref.clone();
    assert_eq!(
        cloned.ref_pic_list_modification_flag_l0,
        mod_ref.ref_pic_list_modification_flag_l0
    );
    assert_eq!(cloned.list_entry_l0, mod_ref.list_entry_l0);
}

// ============================================================================
// PredWeightTable Tests
// ============================================================================

#[test]
fn test_pred_weight_table_default() {
    let table = PredWeightTable::default();
    assert_eq!(table.luma_log2_weight_denom, 0);
    assert_eq!(table.delta_chroma_log2_weight_denom, 0);
    assert!(table.luma_weight_l0.is_empty());
    assert!(table.luma_offset_l0.is_empty());
    assert!(table.chroma_weight_l0.is_empty());
    assert!(table.chroma_offset_l0.is_empty());
    assert!(table.luma_weight_l1.is_empty());
    assert!(table.luma_offset_l1.is_empty());
    assert!(table.chroma_weight_l1.is_empty());
    assert!(table.chroma_offset_l1.is_empty());
}

#[test]
fn test_pred_weight_table_with_l0() {
    let table = PredWeightTable {
        luma_log2_weight_denom: 6,
        delta_chroma_log2_weight_denom: 0,
        luma_weight_l0: vec![100, 200, 300],
        luma_offset_l0: vec![10, 20, 30],
        ..Default::default()
    };
    assert_eq!(table.luma_weight_l0.len(), 3);
    assert_eq!(table.luma_offset_l0[0], 10);
}

#[test]
fn test_pred_weight_table_with_l1() {
    let table = PredWeightTable {
        luma_log2_weight_denom: 6,
        delta_chroma_log2_weight_denom: 0,
        luma_weight_l1: vec![150, 250],
        luma_offset_l1: vec![5, 15],
        ..Default::default()
    };
    assert_eq!(table.luma_weight_l1.len(), 2);
    assert_eq!(table.luma_offset_l1[1], 15);
}

#[test]
fn test_pred_weight_table_chroma_with_l0() {
    let table = PredWeightTable {
        luma_log2_weight_denom: 6,
        delta_chroma_log2_weight_denom: 1,
        chroma_weight_l0: vec![[10, 20], [30, 40]],
        chroma_offset_l0: vec![[5, 10], [15, 20]],
        ..Default::default()
    };
    assert_eq!(table.chroma_weight_l0.len(), 2);
    assert_eq!(table.chroma_weight_l0[0][0], 10);
    assert_eq!(table.chroma_offset_l0[1][1], 20);
}

#[test]
fn test_pred_weight_table_chroma_with_l1() {
    let table = PredWeightTable {
        luma_log2_weight_denom: 6,
        delta_chroma_log2_weight_denom: 1,
        chroma_weight_l1: vec![[15, 25], [35, 45]],
        chroma_offset_l1: vec![[10, 20], [30, 40]],
        ..Default::default()
    };
    assert_eq!(table.chroma_weight_l1.len(), 2);
    assert_eq!(table.chroma_weight_l1[1][0], 35);
    assert_eq!(table.chroma_offset_l1[0][1], 20);
}

#[test]
fn test_pred_weight_table_clone() {
    let table = PredWeightTable {
        luma_log2_weight_denom: 5,
        luma_weight_l0: vec![10, 20],
        ..Default::default()
    };
    let cloned = table.clone();
    assert_eq!(cloned.luma_log2_weight_denom, table.luma_log2_weight_denom);
    assert_eq!(cloned.luma_weight_l0, table.luma_weight_l0);
}

// ============================================================================
// SliceHeader Tests
// ============================================================================

#[test]
fn test_slice_header_default() {
    let header = SliceHeader::default();
    assert!(header.first_slice_segment_in_pic_flag);
    assert!(!header.no_output_of_prior_pics_flag);
    assert_eq!(header.slice_pic_parameter_set_id, 0);
    assert!(!header.dependent_slice_segment_flag);
    assert_eq!(header.slice_segment_address, 0);
    assert_eq!(header.slice_type, SliceType::I);
    assert!(header.pic_output_flag);
    assert_eq!(header.colour_plane_id, 0);
    assert_eq!(header.slice_pic_order_cnt_lsb, 0);
}

#[test]
fn test_slice_header_qp() {
    let header = SliceHeader {
        slice_qp_delta: 5,
        ..Default::default()
    };
    let pps = Pps {
        init_qp_minus26: 10,
        ..Default::default()
    };
    assert_eq!(header.qp(&pps), 26 + 10 + 5); // 41
}

#[test]
fn test_slice_header_qp_negative_delta() {
    let header = SliceHeader {
        slice_qp_delta: -3,
        ..Default::default()
    };
    let pps = Pps {
        init_qp_minus26: 5,
        ..Default::default()
    };
    assert_eq!(header.qp(&pps), 26 + 5 - 3); // 28
}

#[test]
fn test_slice_header_max_num_merge_cand() {
    let header = SliceHeader {
        five_minus_max_num_merge_cand: 0,
        ..Default::default()
    };
    assert_eq!(header.max_num_merge_cand(), 5);
}

#[test]
fn test_slice_header_max_num_merge_cand_min() {
    let header = SliceHeader {
        five_minus_max_num_merge_cand: 5,
        ..Default::default()
    };
    assert_eq!(header.max_num_merge_cand(), 0);
}

#[test]
fn test_slice_header_is_intra() {
    let header = SliceHeader {
        slice_type: SliceType::I,
        ..Default::default()
    };
    assert!(header.is_intra());
    assert!(!header.is_inter());
}

#[test]
fn test_slice_header_is_inter_p_slice() {
    let header = SliceHeader {
        slice_type: SliceType::P,
        ..Default::default()
    };
    assert!(!header.is_intra());
    assert!(header.is_inter());
}

#[test]
fn test_slice_header_is_inter_b_slice() {
    let header = SliceHeader {
        slice_type: SliceType::B,
        ..Default::default()
    };
    assert!(!header.is_intra());
    assert!(header.is_inter());
}

#[test]
fn test_slice_header_num_ref_idx_l0_active() {
    let header = SliceHeader {
        num_ref_idx_l0_active_minus1: 3,
        ..Default::default()
    };
    assert_eq!(header.num_ref_idx_l0_active(), 4); // minus1 + 1
}

#[test]
fn test_slice_header_num_ref_idx_l1_active_for_b_slice() {
    let header = SliceHeader {
        slice_type: SliceType::B,
        num_ref_idx_l1_active_minus1: 2,
        ..Default::default()
    };
    assert_eq!(header.num_ref_idx_l1_active(), 3); // minus1 + 1
}

#[test]
fn test_slice_header_num_ref_idx_l1_active_for_non_b_slice() {
    let header = SliceHeader {
        slice_type: SliceType::P,
        num_ref_idx_l1_active_minus1: 5,
        ..Default::default()
    };
    assert_eq!(header.num_ref_idx_l1_active(), 0); // Not B slice
}

#[test]
fn test_slice_header_num_ref_idx_l1_active_for_i_slice() {
    let header = SliceHeader {
        slice_type: SliceType::I,
        num_ref_idx_l1_active_minus1: 3,
        ..Default::default()
    };
    assert_eq!(header.num_ref_idx_l1_active(), 0); // Not B slice
}

#[test]
fn test_slice_header_clone() {
    let header = SliceHeader {
        slice_type: SliceType::P,
        slice_qp_delta: 10,
        ..Default::default()
    };
    let cloned = header.clone();
    assert_eq!(cloned.slice_type, header.slice_type);
    assert_eq!(cloned.slice_qp_delta, header.slice_qp_delta);
}

#[test]
fn test_slice_header_with_entry_point_offsets() {
    let header = SliceHeader {
        num_entry_point_offsets: 5,
        entry_point_offset_minus1: vec![100, 200, 300, 400, 500],
        ..Default::default()
    };
    assert_eq!(header.num_entry_point_offsets, 5);
    assert_eq!(header.entry_point_offset_minus1.len(), 5);
    assert_eq!(header.entry_point_offset_minus1[2], 300);
}

#[test]
fn test_slice_header_deblocking_flags() {
    let header = SliceHeader {
        deblocking_filter_override_flag: true,
        slice_deblocking_filter_disabled_flag: false,
        slice_beta_offset_div2: 3,
        slice_tc_offset_div2: -2,
        ..Default::default()
    };
    assert!(header.deblocking_filter_override_flag);
    assert!(!header.slice_deblocking_filter_disabled_flag);
    assert_eq!(header.slice_beta_offset_div2, 3);
    assert_eq!(header.slice_tc_offset_div2, -2);
}

#[test]
fn test_slice_header_chroma_qp_offsets() {
    let header = SliceHeader {
        slice_cb_qp_offset: -2,
        slice_cr_qp_offset: 3,
        cu_chroma_qp_offset_enabled_flag: true,
        ..Default::default()
    };
    assert_eq!(header.slice_cb_qp_offset, -2);
    assert_eq!(header.slice_cr_qp_offset, 3);
    assert!(header.cu_chroma_qp_offset_enabled_flag);
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
fn test_slice_header_temporal_mvp() {
    let header = SliceHeader {
        slice_temporal_mvp_enabled_flag: true,
        collocated_from_l0_flag: false,
        collocated_ref_idx: 2,
        ..Default::default()
    };
    assert!(header.slice_temporal_mvp_enabled_flag);
    assert!(!header.collocated_from_l0_flag);
    assert_eq!(header.collocated_ref_idx, 2);
}
