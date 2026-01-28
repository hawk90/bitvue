//! H.264/AVC Picture Parameter Set (PPS) parsing.

use crate::bitreader::BitReader;
use crate::error::Result;
use serde::{Deserialize, Serialize};

/// Picture Parameter Set.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pps {
    /// pic_parameter_set_id
    pub pic_parameter_set_id: u8,
    /// seq_parameter_set_id
    pub seq_parameter_set_id: u8,
    /// entropy_coding_mode_flag (0=CAVLC, 1=CABAC)
    pub entropy_coding_mode_flag: bool,
    /// bottom_field_pic_order_in_frame_present_flag
    pub bottom_field_pic_order_in_frame_present_flag: bool,
    /// num_slice_groups_minus1
    pub num_slice_groups_minus1: u32,
    /// slice_group_map_type
    pub slice_group_map_type: u32,
    /// num_ref_idx_l0_default_active_minus1
    pub num_ref_idx_l0_default_active_minus1: u32,
    /// num_ref_idx_l1_default_active_minus1
    pub num_ref_idx_l1_default_active_minus1: u32,
    /// weighted_pred_flag
    pub weighted_pred_flag: bool,
    /// weighted_bipred_idc
    pub weighted_bipred_idc: u8,
    /// pic_init_qp_minus26
    pub pic_init_qp_minus26: i32,
    /// pic_init_qs_minus26
    pub pic_init_qs_minus26: i32,
    /// chroma_qp_index_offset
    pub chroma_qp_index_offset: i32,
    /// deblocking_filter_control_present_flag
    pub deblocking_filter_control_present_flag: bool,
    /// constrained_intra_pred_flag
    pub constrained_intra_pred_flag: bool,
    /// redundant_pic_cnt_present_flag
    pub redundant_pic_cnt_present_flag: bool,
    /// transform_8x8_mode_flag (high profile)
    pub transform_8x8_mode_flag: bool,
    /// pic_scaling_matrix_present_flag
    pub pic_scaling_matrix_present_flag: bool,
    /// second_chroma_qp_index_offset
    pub second_chroma_qp_index_offset: i32,
}

impl Pps {
    /// Check if CABAC is used.
    pub fn is_cabac(&self) -> bool {
        self.entropy_coding_mode_flag
    }

    /// Get initial QP.
    pub fn initial_qp(&self) -> i32 {
        26 + self.pic_init_qp_minus26
    }
}

/// Parse PPS from NAL unit payload.
pub fn parse_pps(data: &[u8]) -> Result<Pps> {
    let mut reader = BitReader::new(data);

    let pic_parameter_set_id = reader.read_ue()? as u8;
    let seq_parameter_set_id = reader.read_ue()? as u8;
    let entropy_coding_mode_flag = reader.read_flag()?;
    let bottom_field_pic_order_in_frame_present_flag = reader.read_flag()?;

    let num_slice_groups_minus1 = reader.read_ue()?;
    let mut slice_group_map_type = 0;

    if num_slice_groups_minus1 > 0 {
        slice_group_map_type = reader.read_ue()?;

        match slice_group_map_type {
            0 => {
                for _ in 0..=num_slice_groups_minus1 {
                    let _run_length_minus1 = reader.read_ue()?;
                }
            }
            2 => {
                for _ in 0..num_slice_groups_minus1 {
                    let _top_left = reader.read_ue()?;
                    let _bottom_right = reader.read_ue()?;
                }
            }
            3 | 4 | 5 => {
                let _slice_group_change_direction_flag = reader.read_flag()?;
                let _slice_group_change_rate_minus1 = reader.read_ue()?;
            }
            6 => {
                let pic_size_in_map_units_minus1 = reader.read_ue()?;
                let bits = ((num_slice_groups_minus1 + 1) as f64).log2().ceil() as u8;
                for _ in 0..=pic_size_in_map_units_minus1 {
                    let _slice_group_id = reader.read_bits(bits)?;
                }
            }
            _ => {}
        }
    }

    let num_ref_idx_l0_default_active_minus1 = reader.read_ue()?;
    let num_ref_idx_l1_default_active_minus1 = reader.read_ue()?;
    let weighted_pred_flag = reader.read_flag()?;
    let weighted_bipred_idc = reader.read_bits(2)? as u8;
    let pic_init_qp_minus26 = reader.read_se()?;
    let pic_init_qs_minus26 = reader.read_se()?;
    let chroma_qp_index_offset = reader.read_se()?;
    let deblocking_filter_control_present_flag = reader.read_flag()?;
    let constrained_intra_pred_flag = reader.read_flag()?;
    let redundant_pic_cnt_present_flag = reader.read_flag()?;

    // Extended syntax for high profiles
    let mut transform_8x8_mode_flag = false;
    let mut pic_scaling_matrix_present_flag = false;
    let mut second_chroma_qp_index_offset = chroma_qp_index_offset;

    if reader.more_rbsp_data() {
        transform_8x8_mode_flag = reader.read_flag()?;
        pic_scaling_matrix_present_flag = reader.read_flag()?;

        if pic_scaling_matrix_present_flag {
            // Skip scaling lists
            let num_lists = 6 + if transform_8x8_mode_flag { 2 } else { 0 };
            for i in 0..num_lists {
                let scaling_list_present_flag = reader.read_flag()?;
                if scaling_list_present_flag {
                    let size = if i < 6 { 16 } else { 64 };
                    skip_scaling_list(&mut reader, size)?;
                }
            }
        }

        second_chroma_qp_index_offset = reader.read_se()?;
    }

    Ok(Pps {
        pic_parameter_set_id,
        seq_parameter_set_id,
        entropy_coding_mode_flag,
        bottom_field_pic_order_in_frame_present_flag,
        num_slice_groups_minus1,
        slice_group_map_type,
        num_ref_idx_l0_default_active_minus1,
        num_ref_idx_l1_default_active_minus1,
        weighted_pred_flag,
        weighted_bipred_idc,
        pic_init_qp_minus26,
        pic_init_qs_minus26,
        chroma_qp_index_offset,
        deblocking_filter_control_present_flag,
        constrained_intra_pred_flag,
        redundant_pic_cnt_present_flag,
        transform_8x8_mode_flag,
        pic_scaling_matrix_present_flag,
        second_chroma_qp_index_offset,
    })
}

/// Skip a scaling list.
fn skip_scaling_list(reader: &mut BitReader, size: usize) -> Result<()> {
    let mut last_scale = 8i32;
    let mut next_scale = 8i32;

    for _ in 0..size {
        if next_scale != 0 {
            let delta_scale = reader.read_se()?;
            next_scale = (last_scale + delta_scale + 256) % 256;
        }
        last_scale = if next_scale == 0 {
            last_scale
        } else {
            next_scale
        };
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pps_initial_qp() {
        let pps = Pps {
            pic_parameter_set_id: 0,
            seq_parameter_set_id: 0,
            entropy_coding_mode_flag: true,
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
            deblocking_filter_control_present_flag: true,
            constrained_intra_pred_flag: false,
            redundant_pic_cnt_present_flag: false,
            transform_8x8_mode_flag: false,
            pic_scaling_matrix_present_flag: false,
            second_chroma_qp_index_offset: 0,
        };

        assert_eq!(pps.initial_qp(), 26);
        assert!(pps.is_cabac());
    }
}
