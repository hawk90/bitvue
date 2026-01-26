//! H.264/AVC Slice header parsing.

use crate::bitreader::BitReader;
use crate::error::{AvcError, Result};
use crate::nal::NalUnitType;
use crate::pps::Pps;
use crate::sps::Sps;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Slice type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SliceType {
    /// P slice (predictive)
    P = 0,
    /// B slice (bi-predictive)
    B = 1,
    /// I slice (intra)
    I = 2,
    /// SP slice (switching P)
    Sp = 3,
    /// SI slice (switching I)
    Si = 4,
}

impl SliceType {
    /// Create from raw value.
    pub fn from_u32(value: u32) -> Self {
        match value % 5 {
            0 => SliceType::P,
            1 => SliceType::B,
            2 => SliceType::I,
            3 => SliceType::Sp,
            4 => SliceType::Si,
            _ => SliceType::P,
        }
    }

    /// Check if this is an I slice.
    pub fn is_intra(&self) -> bool {
        matches!(self, SliceType::I | SliceType::Si)
    }

    /// Check if this is a B slice.
    pub fn is_b(&self) -> bool {
        matches!(self, SliceType::B)
    }

    /// Check if this is a P slice.
    pub fn is_p(&self) -> bool {
        matches!(self, SliceType::P | SliceType::Sp)
    }

    /// Get name.
    pub fn name(&self) -> &'static str {
        match self {
            SliceType::P => "P",
            SliceType::B => "B",
            SliceType::I => "I",
            SliceType::Sp => "SP",
            SliceType::Si => "SI",
        }
    }
}

/// Reference picture list modification.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RefPicListModification {
    /// modification_of_pic_nums_idc values
    pub modifications: Vec<(u32, u32)>, // (idc, value)
}

/// Decoded reference picture marking.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DecRefPicMarking {
    /// no_output_of_prior_pics_flag
    pub no_output_of_prior_pics_flag: bool,
    /// long_term_reference_flag
    pub long_term_reference_flag: bool,
    /// adaptive_ref_pic_marking_mode_flag
    pub adaptive_ref_pic_marking_mode_flag: bool,
    /// Memory management control operations
    pub mmco_operations: Vec<(u32, u32, u32)>, // (op, diff_of_pic_nums, long_term_idx)
}

/// Slice header.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SliceHeader {
    /// first_mb_in_slice
    pub first_mb_in_slice: u32,
    /// slice_type
    pub slice_type: SliceType,
    /// pic_parameter_set_id
    pub pic_parameter_set_id: u8,
    /// colour_plane_id (if separate_colour_plane_flag)
    pub colour_plane_id: u8,
    /// frame_num
    pub frame_num: u32,
    /// field_pic_flag
    pub field_pic_flag: bool,
    /// bottom_field_flag
    pub bottom_field_flag: bool,
    /// idr_pic_id (for IDR slices)
    pub idr_pic_id: u32,
    /// pic_order_cnt_lsb (for poc_type == 0)
    pub pic_order_cnt_lsb: u32,
    /// delta_pic_order_cnt_bottom
    pub delta_pic_order_cnt_bottom: i32,
    /// delta_pic_order_cnt[2]
    pub delta_pic_order_cnt: [i32; 2],
    /// redundant_pic_cnt
    pub redundant_pic_cnt: u32,
    /// direct_spatial_mv_pred_flag (B slices)
    pub direct_spatial_mv_pred_flag: bool,
    /// num_ref_idx_active_override_flag
    pub num_ref_idx_active_override_flag: bool,
    /// num_ref_idx_l0_active_minus1
    pub num_ref_idx_l0_active_minus1: u32,
    /// num_ref_idx_l1_active_minus1
    pub num_ref_idx_l1_active_minus1: u32,
    /// ref_pic_list_modification_flag_l0
    pub ref_pic_list_modification_flag_l0: bool,
    /// ref_pic_list_modification_flag_l1
    pub ref_pic_list_modification_flag_l1: bool,
    /// Reference picture list modification L0
    pub ref_pic_list_modification_l0: RefPicListModification,
    /// Reference picture list modification L1
    pub ref_pic_list_modification_l1: RefPicListModification,
    /// Decoded reference picture marking
    pub dec_ref_pic_marking: DecRefPicMarking,
    /// cabac_init_idc
    pub cabac_init_idc: u32,
    /// slice_qp_delta
    pub slice_qp_delta: i32,
    /// sp_for_switch_flag
    pub sp_for_switch_flag: bool,
    /// slice_qs_delta
    pub slice_qs_delta: i32,
    /// disable_deblocking_filter_idc
    pub disable_deblocking_filter_idc: u32,
    /// slice_alpha_c0_offset_div2
    pub slice_alpha_c0_offset_div2: i32,
    /// slice_beta_offset_div2
    pub slice_beta_offset_div2: i32,
    /// slice_group_change_cycle
    pub slice_group_change_cycle: u32,
}

impl SliceHeader {
    /// Get slice QP.
    pub fn qp(&self, pps: &Pps) -> i32 {
        26 + pps.pic_init_qp_minus26 + self.slice_qp_delta
    }

    /// Check if this is the first slice in picture.
    pub fn is_first_slice(&self) -> bool {
        self.first_mb_in_slice == 0
    }
}

/// Parse slice header.
pub fn parse_slice_header(
    data: &[u8],
    sps_map: &HashMap<u8, Sps>,
    pps_map: &HashMap<u8, Pps>,
    nal_type: NalUnitType,
    nal_ref_idc: u8,
) -> Result<SliceHeader> {
    let mut reader = BitReader::new(data);

    let first_mb_in_slice = reader.read_ue()?;
    let slice_type_raw = reader.read_ue()?;
    let slice_type = SliceType::from_u32(slice_type_raw);
    let pic_parameter_set_id = reader.read_ue()? as u8;

    let pps = pps_map.get(&pic_parameter_set_id).ok_or_else(|| {
        AvcError::MissingParameterSet(format!("PPS {} not found", pic_parameter_set_id))
    })?;

    let sps = sps_map.get(&pps.seq_parameter_set_id).ok_or_else(|| {
        AvcError::MissingParameterSet(format!("SPS {} not found", pps.seq_parameter_set_id))
    })?;

    let mut colour_plane_id = 0;
    if sps.separate_colour_plane_flag {
        colour_plane_id = reader.read_bits(2)? as u8;
    }

    let frame_num_bits = sps.log2_max_frame_num_minus4 + 4;
    let frame_num = reader.read_bits(frame_num_bits)?;

    let mut field_pic_flag = false;
    let mut bottom_field_flag = false;

    if !sps.frame_mbs_only_flag {
        field_pic_flag = reader.read_flag()?;
        if field_pic_flag {
            bottom_field_flag = reader.read_flag()?;
        }
    }

    let mut idr_pic_id = 0;
    if nal_type == NalUnitType::IdrSlice {
        idr_pic_id = reader.read_ue()?;
    }

    let mut pic_order_cnt_lsb = 0;
    let mut delta_pic_order_cnt_bottom = 0;
    let mut delta_pic_order_cnt = [0i32; 2];

    match sps.pic_order_cnt_type {
        0 => {
            let poc_lsb_bits = sps.log2_max_pic_order_cnt_lsb_minus4 + 4;
            pic_order_cnt_lsb = reader.read_bits(poc_lsb_bits)?;

            if pps.bottom_field_pic_order_in_frame_present_flag && !field_pic_flag {
                delta_pic_order_cnt_bottom = reader.read_se()?;
            }
        }
        1 if !sps.delta_pic_order_always_zero_flag => {
            delta_pic_order_cnt[0] = reader.read_se()?;

            if pps.bottom_field_pic_order_in_frame_present_flag && !field_pic_flag {
                delta_pic_order_cnt[1] = reader.read_se()?;
            }
        }
        _ => {}
    }

    let mut redundant_pic_cnt = 0;
    if pps.redundant_pic_cnt_present_flag {
        redundant_pic_cnt = reader.read_ue()?;
    }

    let mut direct_spatial_mv_pred_flag = false;
    if slice_type.is_b() {
        direct_spatial_mv_pred_flag = reader.read_flag()?;
    }

    let mut num_ref_idx_active_override_flag = false;
    let mut num_ref_idx_l0_active_minus1 = pps.num_ref_idx_l0_default_active_minus1;
    let mut num_ref_idx_l1_active_minus1 = pps.num_ref_idx_l1_default_active_minus1;

    if slice_type.is_p() || slice_type.is_b() {
        num_ref_idx_active_override_flag = reader.read_flag()?;
        if num_ref_idx_active_override_flag {
            num_ref_idx_l0_active_minus1 = reader.read_ue()?;
            if slice_type.is_b() {
                num_ref_idx_l1_active_minus1 = reader.read_ue()?;
            }
        }
    }

    // Reference picture list modification
    let mut ref_pic_list_modification_flag_l0 = false;
    let mut ref_pic_list_modification_flag_l1 = false;
    let mut ref_pic_list_modification_l0 = RefPicListModification::default();
    let mut ref_pic_list_modification_l1 = RefPicListModification::default();

    if !slice_type.is_intra() {
        ref_pic_list_modification_flag_l0 = reader.read_flag()?;
        if ref_pic_list_modification_flag_l0 {
            ref_pic_list_modification_l0 = parse_ref_pic_list_modification(&mut reader)?;
        }
    }

    if slice_type.is_b() {
        ref_pic_list_modification_flag_l1 = reader.read_flag()?;
        if ref_pic_list_modification_flag_l1 {
            ref_pic_list_modification_l1 = parse_ref_pic_list_modification(&mut reader)?;
        }
    }

    // Pred weight table
    if (pps.weighted_pred_flag && (slice_type.is_p() || matches!(slice_type, SliceType::Sp)))
        || (pps.weighted_bipred_idc == 1 && slice_type.is_b())
    {
        skip_pred_weight_table(&mut reader, slice_type, num_ref_idx_l0_active_minus1, num_ref_idx_l1_active_minus1, sps)?;
    }

    // Dec ref pic marking
    let mut dec_ref_pic_marking = DecRefPicMarking::default();
    if nal_ref_idc != 0 {
        dec_ref_pic_marking = parse_dec_ref_pic_marking(&mut reader, nal_type)?;
    }

    let mut cabac_init_idc = 0;
    if pps.entropy_coding_mode_flag && !slice_type.is_intra() {
        cabac_init_idc = reader.read_ue()?;
    }

    let slice_qp_delta = reader.read_se()?;

    let mut sp_for_switch_flag = false;
    let mut slice_qs_delta = 0;
    if matches!(slice_type, SliceType::Sp | SliceType::Si) {
        if matches!(slice_type, SliceType::Sp) {
            sp_for_switch_flag = reader.read_flag()?;
        }
        slice_qs_delta = reader.read_se()?;
    }

    let mut disable_deblocking_filter_idc = 0;
    let mut slice_alpha_c0_offset_div2 = 0;
    let mut slice_beta_offset_div2 = 0;

    if pps.deblocking_filter_control_present_flag {
        disable_deblocking_filter_idc = reader.read_ue()?;
        if disable_deblocking_filter_idc != 1 {
            slice_alpha_c0_offset_div2 = reader.read_se()?;
            slice_beta_offset_div2 = reader.read_se()?;
        }
    }

    let mut slice_group_change_cycle = 0;
    if pps.num_slice_groups_minus1 > 0 && pps.slice_group_map_type >= 3 && pps.slice_group_map_type <= 5 {
        // Calculate number of bits needed
        let pic_size_in_map_units = (sps.pic_width_in_mbs_minus1 + 1)
            * (sps.pic_height_in_map_units_minus1 + 1);
        let bits = ((pic_size_in_map_units as f64).log2().ceil() + 1.0) as u8;
        slice_group_change_cycle = reader.read_bits(bits)?;
    }

    Ok(SliceHeader {
        first_mb_in_slice,
        slice_type,
        pic_parameter_set_id,
        colour_plane_id,
        frame_num,
        field_pic_flag,
        bottom_field_flag,
        idr_pic_id,
        pic_order_cnt_lsb,
        delta_pic_order_cnt_bottom,
        delta_pic_order_cnt,
        redundant_pic_cnt,
        direct_spatial_mv_pred_flag,
        num_ref_idx_active_override_flag,
        num_ref_idx_l0_active_minus1,
        num_ref_idx_l1_active_minus1,
        ref_pic_list_modification_flag_l0,
        ref_pic_list_modification_flag_l1,
        ref_pic_list_modification_l0,
        ref_pic_list_modification_l1,
        dec_ref_pic_marking,
        cabac_init_idc,
        slice_qp_delta,
        sp_for_switch_flag,
        slice_qs_delta,
        disable_deblocking_filter_idc,
        slice_alpha_c0_offset_div2,
        slice_beta_offset_div2,
        slice_group_change_cycle,
    })
}

/// Parse reference picture list modification.
fn parse_ref_pic_list_modification(reader: &mut BitReader) -> Result<RefPicListModification> {
    let mut modifications = Vec::new();

    loop {
        let modification_of_pic_nums_idc = reader.read_ue()?;
        if modification_of_pic_nums_idc == 3 {
            break;
        }

        let value = match modification_of_pic_nums_idc {
            0 | 1 => reader.read_ue()?, // abs_diff_pic_num_minus1
            2 => reader.read_ue()?,      // long_term_pic_num
            _ => break,
        };

        modifications.push((modification_of_pic_nums_idc, value));
    }

    Ok(RefPicListModification { modifications })
}

/// Parse decoded reference picture marking.
fn parse_dec_ref_pic_marking(reader: &mut BitReader, nal_type: NalUnitType) -> Result<DecRefPicMarking> {
    let mut marking = DecRefPicMarking::default();

    if nal_type == NalUnitType::IdrSlice {
        marking.no_output_of_prior_pics_flag = reader.read_flag()?;
        marking.long_term_reference_flag = reader.read_flag()?;
    } else {
        marking.adaptive_ref_pic_marking_mode_flag = reader.read_flag()?;

        if marking.adaptive_ref_pic_marking_mode_flag {
            loop {
                let memory_management_control_operation = reader.read_ue()?;
                if memory_management_control_operation == 0 {
                    break;
                }

                let mut diff_of_pic_nums = 0;
                let mut long_term_idx = 0;

                match memory_management_control_operation {
                    1 | 3 => {
                        diff_of_pic_nums = reader.read_ue()?;
                    }
                    2 => {
                        long_term_idx = reader.read_ue()?;
                    }
                    4 => {
                        let _max_long_term_frame_idx_plus1 = reader.read_ue()?;
                    }
                    6 => {
                        long_term_idx = reader.read_ue()?;
                    }
                    _ => {}
                }

                if memory_management_control_operation == 3 {
                    long_term_idx = reader.read_ue()?;
                }

                marking.mmco_operations.push((
                    memory_management_control_operation,
                    diff_of_pic_nums,
                    long_term_idx,
                ));
            }
        }
    }

    Ok(marking)
}

/// Skip prediction weight table.
fn skip_pred_weight_table(
    reader: &mut BitReader,
    slice_type: SliceType,
    num_ref_idx_l0_active_minus1: u32,
    num_ref_idx_l1_active_minus1: u32,
    sps: &Sps,
) -> Result<()> {
    let _luma_log2_weight_denom = reader.read_ue()?;

    if sps.chroma_format_idc != crate::sps::ChromaFormat::Monochrome {
        let _chroma_log2_weight_denom = reader.read_ue()?;
    }

    for _ in 0..=num_ref_idx_l0_active_minus1 {
        let luma_weight_flag = reader.read_flag()?;
        if luma_weight_flag {
            let _luma_weight = reader.read_se()?;
            let _luma_offset = reader.read_se()?;
        }

        if sps.chroma_format_idc != crate::sps::ChromaFormat::Monochrome {
            let chroma_weight_flag = reader.read_flag()?;
            if chroma_weight_flag {
                for _ in 0..2 {
                    let _chroma_weight = reader.read_se()?;
                    let _chroma_offset = reader.read_se()?;
                }
            }
        }
    }

    if slice_type.is_b() {
        for _ in 0..=num_ref_idx_l1_active_minus1 {
            let luma_weight_flag = reader.read_flag()?;
            if luma_weight_flag {
                let _luma_weight = reader.read_se()?;
                let _luma_offset = reader.read_se()?;
            }

            if sps.chroma_format_idc != crate::sps::ChromaFormat::Monochrome {
                let chroma_weight_flag = reader.read_flag()?;
                if chroma_weight_flag {
                    for _ in 0..2 {
                        let _chroma_weight = reader.read_se()?;
                        let _chroma_offset = reader.read_se()?;
                    }
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slice_type() {
        assert_eq!(SliceType::from_u32(0), SliceType::P);
        assert_eq!(SliceType::from_u32(1), SliceType::B);
        assert_eq!(SliceType::from_u32(2), SliceType::I);
        assert_eq!(SliceType::from_u32(5), SliceType::P); // Mod 5
        assert_eq!(SliceType::from_u32(7), SliceType::I); // Mod 5

        assert!(SliceType::I.is_intra());
        assert!(SliceType::B.is_b());
        assert!(SliceType::P.is_p());
    }
}
