//! HEVC Slice Header parsing.
//!
//! Slice header contains per-slice parameters.
//! It is defined in ITU-T H.265 Section 7.3.6.

use crate::bitreader::BitReader;
use crate::error::{HevcError, Result};
use crate::nal::NalUnitType;
use crate::pps::Pps;
use crate::sps::Sps;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// HEVC slice type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SliceType {
    /// B slice (bi-directional prediction).
    B = 0,
    /// P slice (uni-directional prediction).
    P = 1,
    /// I slice (intra prediction only).
    I = 2,
}

impl SliceType {
    /// Create from raw value.
    pub fn from_u32(value: u32) -> Option<Self> {
        match value {
            0 => Some(Self::B),
            1 => Some(Self::P),
            2 => Some(Self::I),
            _ => None,
        }
    }

    /// Check if this is an intra slice.
    pub fn is_intra(&self) -> bool {
        matches!(self, Self::I)
    }

    /// Check if this slice uses inter prediction.
    pub fn is_inter(&self) -> bool {
        matches!(self, Self::B | Self::P)
    }

    /// Get human-readable name.
    pub fn name(&self) -> &'static str {
        match self {
            Self::B => "B",
            Self::P => "P",
            Self::I => "I",
        }
    }

    /// Get human-readable name (alias for compatibility).
    pub fn as_str(&self) -> &'static str {
        self.name()
    }
}

/// Reference picture list modification.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RefPicListModification {
    /// List 0 modification flag.
    pub ref_pic_list_modification_flag_l0: bool,
    /// List 0 modification indices.
    pub list_entry_l0: Vec<u8>,
    /// List 1 modification flag.
    pub ref_pic_list_modification_flag_l1: bool,
    /// List 1 modification indices.
    pub list_entry_l1: Vec<u8>,
}

/// Prediction weight table for weighted prediction.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PredWeightTable {
    /// Luma log2 weight denominator.
    pub luma_log2_weight_denom: u8,
    /// Delta chroma log2 weight denominator.
    pub delta_chroma_log2_weight_denom: i8,
    /// Luma weights for L0.
    pub luma_weight_l0: Vec<i16>,
    /// Luma offsets for L0.
    pub luma_offset_l0: Vec<i16>,
    /// Chroma weights for L0.
    pub chroma_weight_l0: Vec<[i16; 2]>,
    /// Chroma offsets for L0.
    pub chroma_offset_l0: Vec<[i16; 2]>,
    /// Luma weights for L1.
    pub luma_weight_l1: Vec<i16>,
    /// Luma offsets for L1.
    pub luma_offset_l1: Vec<i16>,
    /// Chroma weights for L1.
    pub chroma_weight_l1: Vec<[i16; 2]>,
    /// Chroma offsets for L1.
    pub chroma_offset_l1: Vec<[i16; 2]>,
}

/// HEVC Slice Header.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SliceHeader {
    /// First slice segment in picture.
    pub first_slice_segment_in_pic_flag: bool,
    /// No output of prior pictures (for IRAP).
    pub no_output_of_prior_pics_flag: bool,
    /// Referenced PPS ID.
    pub slice_pic_parameter_set_id: u8,
    /// Dependent slice segment flag.
    pub dependent_slice_segment_flag: bool,
    /// Slice segment address.
    pub slice_segment_address: u32,
    /// Slice type (B, P, I).
    pub slice_type: SliceType,
    /// Picture output flag.
    pub pic_output_flag: bool,
    /// Colour plane ID (for separate colour planes).
    pub colour_plane_id: u8,
    /// Picture order count LSB.
    pub slice_pic_order_cnt_lsb: u32,
    /// Short-term reference picture set SPS flag.
    pub short_term_ref_pic_set_sps_flag: bool,
    /// Short-term reference picture set index.
    pub short_term_ref_pic_set_idx: u8,
    /// Number of long-term SPS pictures.
    pub num_long_term_sps: u8,
    /// Number of long-term pictures.
    pub num_long_term_pics: u8,
    /// Slice temporal MVP enabled.
    pub slice_temporal_mvp_enabled_flag: bool,
    /// Slice SAO luma flag.
    pub slice_sao_luma_flag: bool,
    /// Slice SAO chroma flag.
    pub slice_sao_chroma_flag: bool,
    /// Number of reference pictures in list 0 active.
    pub num_ref_idx_l0_active_minus1: u8,
    /// Number of reference pictures in list 1 active.
    pub num_ref_idx_l1_active_minus1: u8,
    /// Reference picture list modification.
    pub ref_pic_list_modification: Option<RefPicListModification>,
    /// MVP L0 flag.
    pub mvd_l1_zero_flag: bool,
    /// CABAC init flag.
    pub cabac_init_flag: bool,
    /// Collocated from L0 flag.
    pub collocated_from_l0_flag: bool,
    /// Collocated reference index.
    pub collocated_ref_idx: u8,
    /// Prediction weight table.
    pub pred_weight_table: Option<PredWeightTable>,
    /// Five minus max num merge cand.
    pub five_minus_max_num_merge_cand: u8,
    /// Use integer MV flag.
    pub use_integer_mv_flag: bool,
    /// Slice QP delta.
    pub slice_qp_delta: i8,
    /// Slice CB QP offset.
    pub slice_cb_qp_offset: i8,
    /// Slice CR QP offset.
    pub slice_cr_qp_offset: i8,
    /// CU chroma QP offset enabled.
    pub cu_chroma_qp_offset_enabled_flag: bool,
    /// Deblocking filter override flag.
    pub deblocking_filter_override_flag: bool,
    /// Slice deblocking filter disabled.
    pub slice_deblocking_filter_disabled_flag: bool,
    /// Slice beta offset div 2.
    pub slice_beta_offset_div2: i8,
    /// Slice TC offset div 2.
    pub slice_tc_offset_div2: i8,
    /// Slice loop filter across slices enabled.
    pub slice_loop_filter_across_slices_enabled_flag: bool,
    /// Number of entry point offsets.
    pub num_entry_point_offsets: u32,
    /// Entry point offset minus 1.
    pub entry_point_offset_minus1: Vec<u32>,
}

impl Default for SliceHeader {
    fn default() -> Self {
        Self {
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
            short_term_ref_pic_set_idx: 0,
            num_long_term_sps: 0,
            num_long_term_pics: 0,
            slice_temporal_mvp_enabled_flag: false,
            slice_sao_luma_flag: false,
            slice_sao_chroma_flag: false,
            num_ref_idx_l0_active_minus1: 0,
            num_ref_idx_l1_active_minus1: 0,
            ref_pic_list_modification: None,
            mvd_l1_zero_flag: false,
            cabac_init_flag: false,
            collocated_from_l0_flag: true,
            collocated_ref_idx: 0,
            pred_weight_table: None,
            five_minus_max_num_merge_cand: 0,
            use_integer_mv_flag: false,
            slice_qp_delta: 0,
            slice_cb_qp_offset: 0,
            slice_cr_qp_offset: 0,
            cu_chroma_qp_offset_enabled_flag: false,
            deblocking_filter_override_flag: false,
            slice_deblocking_filter_disabled_flag: false,
            slice_beta_offset_div2: 0,
            slice_tc_offset_div2: 0,
            slice_loop_filter_across_slices_enabled_flag: false,
            num_entry_point_offsets: 0,
            entry_point_offset_minus1: Vec::new(),
        }
    }
}

impl SliceHeader {
    /// Get slice QP value.
    pub fn qp(&self, pps: &Pps) -> i8 {
        26 + pps.init_qp_minus26 + self.slice_qp_delta
    }

    /// Get maximum number of merge candidates.
    pub fn max_num_merge_cand(&self) -> u8 {
        5 - self.five_minus_max_num_merge_cand
    }

    /// Check if this is an intra slice.
    pub fn is_intra(&self) -> bool {
        self.slice_type.is_intra()
    }

    /// Check if this slice uses inter prediction.
    pub fn is_inter(&self) -> bool {
        self.slice_type.is_inter()
    }

    /// Get number of active references in L0.
    pub fn num_ref_idx_l0_active(&self) -> u8 {
        self.num_ref_idx_l0_active_minus1 + 1
    }

    /// Get number of active references in L1.
    pub fn num_ref_idx_l1_active(&self) -> u8 {
        if self.slice_type == SliceType::B {
            self.num_ref_idx_l1_active_minus1 + 1
        } else {
            0
        }
    }
}

/// Parse slice header from RBSP data.
pub fn parse_slice_header(
    data: &[u8],
    sps_map: &HashMap<u8, Sps>,
    pps_map: &HashMap<u8, Pps>,
    nal_type: NalUnitType,
) -> Result<SliceHeader> {
    let mut reader = BitReader::new(data);
    let mut header = SliceHeader::default();

    // first_slice_segment_in_pic_flag (1 bit)
    header.first_slice_segment_in_pic_flag = reader.read_bit()?;

    // no_output_of_prior_pics_flag (1 bit) - only for IRAP
    if nal_type.is_irap() {
        header.no_output_of_prior_pics_flag = reader.read_bit()?;
    }

    // slice_pic_parameter_set_id (ue(v))
    header.slice_pic_parameter_set_id = reader.read_ue()? as u8;

    // Get PPS and SPS
    let pps = pps_map
        .get(&header.slice_pic_parameter_set_id)
        .ok_or_else(|| {
            HevcError::InvalidData(format!(
                "PPS {} not found",
                header.slice_pic_parameter_set_id
            ))
        })?;

    let sps = sps_map.get(&pps.pps_seq_parameter_set_id).ok_or_else(|| {
        HevcError::InvalidData(format!("SPS {} not found", pps.pps_seq_parameter_set_id))
    })?;

    // dependent_slice_segment_flag
    if !header.first_slice_segment_in_pic_flag {
        if pps.dependent_slice_segments_enabled_flag {
            header.dependent_slice_segment_flag = reader.read_bit()?;
        }

        // slice_segment_address - need to calculate number of CTBs
        let pic_size_in_ctbs = sps.pic_width_in_ctbs() as u32 * sps.pic_height_in_ctbs() as u32;
        let bits_needed = (32 - pic_size_in_ctbs.leading_zeros()) as u8;
        header.slice_segment_address = reader.read_bits(bits_needed)?;
    }

    // Skip extra slice header bits
    if !header.dependent_slice_segment_flag {
        for _ in 0..pps.num_extra_slice_header_bits {
            let _ = reader.read_bit()?;
        }

        // slice_type (ue(v))
        let slice_type_raw = reader.read_ue()?;
        header.slice_type = SliceType::from_u32(slice_type_raw).ok_or_else(|| {
            HevcError::InvalidData(format!("Invalid slice type: {}", slice_type_raw))
        })?;

        // pic_output_flag
        if pps.output_flag_present_flag {
            header.pic_output_flag = reader.read_bit()?;
        }

        // colour_plane_id
        if sps.separate_colour_plane_flag {
            header.colour_plane_id = reader.read_bits(2)? as u8;
        }

        // POC and reference picture handling (non-IDR only)
        if !nal_type.is_idr() {
            // slice_pic_order_cnt_lsb
            let poc_bits = sps.log2_max_pic_order_cnt_lsb_minus4 + 4;
            header.slice_pic_order_cnt_lsb = reader.read_bits(poc_bits)?;

            // short_term_ref_pic_set_sps_flag
            header.short_term_ref_pic_set_sps_flag = reader.read_bit()?;

            if header.short_term_ref_pic_set_sps_flag {
                // short_term_ref_pic_set_idx
                if sps.num_short_term_ref_pic_sets > 1 {
                    let bits_needed =
                        (32 - (sps.num_short_term_ref_pic_sets as u32).leading_zeros()) as u8;
                    header.short_term_ref_pic_set_idx = reader.read_bits(bits_needed)? as u8;
                }
            } else {
                // Parse st_ref_pic_set in slice header
                // This is complex - skip for now
                // Full implementation would parse short_term_ref_pic_set()
            }

            // Long-term reference pictures
            if sps.long_term_ref_pics_present_flag {
                if sps.num_long_term_ref_pics_sps > 0 {
                    header.num_long_term_sps = reader.read_ue()? as u8;
                }
                header.num_long_term_pics = reader.read_ue()? as u8;

                // Skip long-term ref pic parsing for now
            }

            // slice_temporal_mvp_enabled_flag
            if sps.sps_temporal_mvp_enabled_flag {
                header.slice_temporal_mvp_enabled_flag = reader.read_bit()?;
            }
        }

        // SAO flags
        if sps.sample_adaptive_offset_enabled_flag {
            header.slice_sao_luma_flag = reader.read_bit()?;
            if sps.chroma_format_idc != crate::sps::ChromaFormat::Monochrome {
                header.slice_sao_chroma_flag = reader.read_bit()?;
            }
        }

        // Reference picture list handling for inter slices
        if header.slice_type.is_inter() {
            // num_ref_idx_active_override_flag
            let num_ref_idx_active_override_flag = reader.read_bit()?;

            if num_ref_idx_active_override_flag {
                header.num_ref_idx_l0_active_minus1 = reader.read_ue()? as u8;
                if header.slice_type == SliceType::B {
                    header.num_ref_idx_l1_active_minus1 = reader.read_ue()? as u8;
                }
            } else {
                header.num_ref_idx_l0_active_minus1 = pps.num_ref_idx_l0_default_active_minus1;
                header.num_ref_idx_l1_active_minus1 = pps.num_ref_idx_l1_default_active_minus1;
            }

            // ref_pic_lists_modification() - skip for now
            if pps.lists_modification_present_flag {
                // Parse modification lists
            }

            // mvd_l1_zero_flag
            if header.slice_type == SliceType::B {
                header.mvd_l1_zero_flag = reader.read_bit()?;
            }

            // cabac_init_flag
            if pps.cabac_init_present_flag {
                header.cabac_init_flag = reader.read_bit()?;
            }

            // Temporal MVP
            if header.slice_temporal_mvp_enabled_flag {
                if header.slice_type == SliceType::B {
                    header.collocated_from_l0_flag = reader.read_bit()?;
                }

                let num_ref = if header.collocated_from_l0_flag {
                    header.num_ref_idx_l0_active_minus1
                } else {
                    header.num_ref_idx_l1_active_minus1
                };

                if num_ref > 0 {
                    header.collocated_ref_idx = reader.read_ue()? as u8;
                }
            }

            // Weighted prediction
            if (pps.weighted_pred_flag && header.slice_type == SliceType::P)
                || (pps.weighted_bipred_flag && header.slice_type == SliceType::B)
            {
                // Parse pred_weight_table() - skip for now
            }

            // five_minus_max_num_merge_cand
            header.five_minus_max_num_merge_cand = reader.read_ue()? as u8;

            // use_integer_mv_flag (for SCC extension)
            // Skip for now
        }

        // slice_qp_delta
        header.slice_qp_delta = reader.read_se()? as i8;

        // Chroma QP offsets
        if pps.pps_slice_chroma_qp_offsets_present_flag {
            header.slice_cb_qp_offset = reader.read_se()? as i8;
            header.slice_cr_qp_offset = reader.read_se()? as i8;
        }

        // cu_chroma_qp_offset_enabled_flag (for range extension)
        // Skip for now

        // Deblocking filter
        if pps.deblocking_filter_override_enabled_flag {
            header.deblocking_filter_override_flag = reader.read_bit()?;
        }

        if header.deblocking_filter_override_flag {
            header.slice_deblocking_filter_disabled_flag = reader.read_bit()?;
            if !header.slice_deblocking_filter_disabled_flag {
                header.slice_beta_offset_div2 = reader.read_se()? as i8;
                header.slice_tc_offset_div2 = reader.read_se()? as i8;
            }
        } else {
            header.slice_deblocking_filter_disabled_flag = pps.pps_deblocking_filter_disabled_flag;
            header.slice_beta_offset_div2 = pps.pps_beta_offset_div2;
            header.slice_tc_offset_div2 = pps.pps_tc_offset_div2;
        }

        // Loop filter across slices
        if pps.pps_loop_filter_across_slices_enabled_flag
            && (header.slice_sao_luma_flag
                || header.slice_sao_chroma_flag
                || !header.slice_deblocking_filter_disabled_flag)
        {
            header.slice_loop_filter_across_slices_enabled_flag = reader.read_bit()?;
        }
    }

    // Entry points for tiles or WPP
    if pps.tiles_enabled_flag || pps.entropy_coding_sync_enabled_flag {
        // SECURITY: Limit entry point offsets to prevent DoS via memory exhaustion
        const MAX_ENTRY_POINT_OFFSETS: u32 = 1000; // Reasonable limit
        const MAX_OFFSET_BITS: u8 = 32; // Maximum bits for offset values

        header.num_entry_point_offsets = reader.read_ue()?;

        if header.num_entry_point_offsets > MAX_ENTRY_POINT_OFFSETS {
            return Err(HevcError::InvalidData(format!(
                "Entry point offsets {} exceeds maximum {}",
                header.num_entry_point_offsets, MAX_ENTRY_POINT_OFFSETS
            )));
        }

        if header.num_entry_point_offsets > 0 {
            let offset_len_minus1 = reader.read_ue()?;
            let offset_bits = (offset_len_minus1 + 1) as u8;

            if offset_bits > MAX_OFFSET_BITS {
                return Err(HevcError::InvalidData(format!(
                    "Entry point offset bits {} exceeds maximum {}",
                    offset_bits, MAX_OFFSET_BITS
                )));
            }

            for _ in 0..header.num_entry_point_offsets {
                header
                    .entry_point_offset_minus1
                    .push(reader.read_bits(offset_bits)?);
            }
        }
    }

    Ok(header)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slice_type() {
        assert!(SliceType::I.is_intra());
        assert!(!SliceType::I.is_inter());

        assert!(SliceType::P.is_inter());
        assert!(SliceType::B.is_inter());

        assert_eq!(SliceType::I.name(), "I");
        assert_eq!(SliceType::P.name(), "P");
        assert_eq!(SliceType::B.name(), "B");
    }

    #[test]
    fn test_slice_header_defaults() {
        let header = SliceHeader::default();
        assert!(header.first_slice_segment_in_pic_flag);
        assert!(header.is_intra());
        assert_eq!(header.max_num_merge_cand(), 5);
    }
}
