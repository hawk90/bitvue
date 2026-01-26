//! VVC Picture Parameter Set (PPS) parsing.
//!
//! VVC PPS contains picture-level parameters.

use crate::bitreader::BitReader;
use crate::error::{Result};
use serde::{Deserialize, Serialize};

/// VVC Picture Parameter Set.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pps {
    /// PPS ID (0-63).
    pub pps_pic_parameter_set_id: u8,
    /// Referenced SPS ID.
    pub pps_seq_parameter_set_id: u8,
    /// Mixed NAL unit types allowed.
    pub pps_mixed_nalu_types_in_pic_flag: bool,
    /// Picture width.
    pub pps_pic_width_in_luma_samples: u32,
    /// Picture height.
    pub pps_pic_height_in_luma_samples: u32,
    /// Conformance window flag.
    pub pps_conformance_window_flag: bool,
    /// Scaling window explicit flag.
    pub pps_scaling_window_explicit_signalling_flag: bool,
    /// Output flag present.
    pub pps_output_flag_present_flag: bool,
    /// No picture partition.
    pub pps_no_pic_partition_flag: bool,
    /// Subpicture ID mapping present.
    pub pps_subpic_id_mapping_present_flag: bool,
    /// Loop filter across tiles enabled.
    pub pps_loop_filter_across_tiles_enabled_flag: bool,
    /// Loop filter across slices enabled.
    pub pps_loop_filter_across_slices_enabled_flag: bool,
    /// Cabac init present.
    pub pps_cabac_init_present_flag: bool,
    /// Number of ref idx L0 default active.
    pub pps_num_ref_idx_default_active_minus1: [u8; 2],
    /// RPL1 idx present.
    pub pps_rpl1_idx_present_flag: bool,
    /// Weighted prediction.
    pub pps_weighted_pred_flag: bool,
    /// Weighted biprediction.
    pub pps_weighted_bipred_flag: bool,
    /// Reference wraparound enabled.
    pub pps_ref_wraparound_enabled_flag: bool,
    /// Initial QP.
    pub pps_init_qp_minus26: i8,
    /// CU QP delta enabled.
    pub pps_cu_qp_delta_enabled_flag: bool,
    /// Chroma tool offsets present.
    pub pps_chroma_tool_offsets_present_flag: bool,
    /// CB QP offset.
    pub pps_cb_qp_offset: i8,
    /// CR QP offset.
    pub pps_cr_qp_offset: i8,
    /// Joint CB-CR QP offset present.
    pub pps_joint_cbcr_qp_offset_present_flag: bool,
    /// Joint CB-CR QP offset value.
    pub pps_joint_cbcr_qp_offset_value: i8,
    /// Slice chroma QP offsets present.
    pub pps_slice_chroma_qp_offsets_present_flag: bool,
    /// Deblocking filter control present.
    pub pps_deblocking_filter_control_present_flag: bool,
    /// Deblocking filter disabled.
    pub pps_deblocking_filter_disabled_flag: bool,
    /// Picture header extension present.
    pub pps_picture_header_extension_present_flag: bool,
    /// Slice header extension present.
    pub pps_slice_header_extension_present_flag: bool,
}

impl Default for Pps {
    fn default() -> Self {
        Self {
            pps_pic_parameter_set_id: 0,
            pps_seq_parameter_set_id: 0,
            pps_mixed_nalu_types_in_pic_flag: false,
            pps_pic_width_in_luma_samples: 0,
            pps_pic_height_in_luma_samples: 0,
            pps_conformance_window_flag: false,
            pps_scaling_window_explicit_signalling_flag: false,
            pps_output_flag_present_flag: false,
            pps_no_pic_partition_flag: true,
            pps_subpic_id_mapping_present_flag: false,
            pps_loop_filter_across_tiles_enabled_flag: true,
            pps_loop_filter_across_slices_enabled_flag: true,
            pps_cabac_init_present_flag: false,
            pps_num_ref_idx_default_active_minus1: [0, 0],
            pps_rpl1_idx_present_flag: false,
            pps_weighted_pred_flag: false,
            pps_weighted_bipred_flag: false,
            pps_ref_wraparound_enabled_flag: false,
            pps_init_qp_minus26: 0,
            pps_cu_qp_delta_enabled_flag: false,
            pps_chroma_tool_offsets_present_flag: false,
            pps_cb_qp_offset: 0,
            pps_cr_qp_offset: 0,
            pps_joint_cbcr_qp_offset_present_flag: false,
            pps_joint_cbcr_qp_offset_value: 0,
            pps_slice_chroma_qp_offsets_present_flag: false,
            pps_deblocking_filter_control_present_flag: false,
            pps_deblocking_filter_disabled_flag: false,
            pps_picture_header_extension_present_flag: false,
            pps_slice_header_extension_present_flag: false,
        }
    }
}

impl Pps {
    /// Get initial QP value.
    pub fn init_qp(&self) -> i8 {
        26 + self.pps_init_qp_minus26
    }
}

/// Parse PPS from RBSP data.
pub fn parse_pps(data: &[u8]) -> Result<Pps> {
    let mut reader = BitReader::new(data);
    let mut pps = Pps::default();

    // pps_pic_parameter_set_id (6 bits)
    pps.pps_pic_parameter_set_id = reader.read_bits(6)? as u8;

    // pps_seq_parameter_set_id (4 bits)
    pps.pps_seq_parameter_set_id = reader.read_bits(4)? as u8;

    // pps_mixed_nalu_types_in_pic_flag (1 bit)
    pps.pps_mixed_nalu_types_in_pic_flag = reader.read_bit()?;

    // pps_pic_width_in_luma_samples (ue(v))
    pps.pps_pic_width_in_luma_samples = reader.read_ue()?;

    // pps_pic_height_in_luma_samples (ue(v))
    pps.pps_pic_height_in_luma_samples = reader.read_ue()?;

    // pps_conformance_window_flag (1 bit)
    pps.pps_conformance_window_flag = reader.read_bit()?;

    if pps.pps_conformance_window_flag {
        // Skip conformance window offsets
        let _ = reader.read_ue()?; // left
        let _ = reader.read_ue()?; // right
        let _ = reader.read_ue()?; // top
        let _ = reader.read_ue()?; // bottom
    }

    // pps_scaling_window_explicit_signalling_flag (1 bit)
    pps.pps_scaling_window_explicit_signalling_flag = reader.read_bit()?;

    if pps.pps_scaling_window_explicit_signalling_flag {
        // Skip scaling window
        let _ = reader.read_se()?; // left
        let _ = reader.read_se()?; // right
        let _ = reader.read_se()?; // top
        let _ = reader.read_se()?; // bottom
    }

    // pps_output_flag_present_flag (1 bit)
    pps.pps_output_flag_present_flag = reader.read_bit()?;

    // pps_no_pic_partition_flag (1 bit)
    pps.pps_no_pic_partition_flag = reader.read_bit()?;

    // Skip remaining fields for simplified parsing

    Ok(pps)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pps_defaults() {
        let pps = Pps::default();
        assert_eq!(pps.init_qp(), 26);
        assert!(pps.pps_no_pic_partition_flag);
    }
}
