//! VVC Sequence Parameter Set (SPS) parsing.
//!
//! VVC SPS contains sequence-level coding parameters including
//! new VVC features like dual tree, ALF, LMCS, and extended partitioning.

use crate::bitreader::BitReader;
use crate::error::Result;
use serde::{Deserialize, Serialize};

// Re-export ChromaFormat from bitvue_core for backward compatibility
pub use bitvue_core::ChromaFormat;

/// VVC Profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Profile {
    Main10,
    Main10StillPicture,
    Main10444,
    Main10444StillPicture,
    Multilayer,
    MultilayerMain10,
    Unknown(u8),
}

impl Profile {
    /// Get the profile IDC value.
    pub fn idc(&self) -> u8 {
        match self {
            Self::Main10 => 1,
            Self::Main10StillPicture => 2,
            Self::Main10444 => 3,
            Self::Main10444StillPicture => 4,
            Self::Multilayer => 17,
            Self::MultilayerMain10 => 65,
            Self::Unknown(v) => *v,
        }
    }
}

impl From<u8> for Profile {
    fn from(value: u8) -> Self {
        match value {
            1 => Self::Main10,
            2 => Self::Main10StillPicture,
            3 => Self::Main10444,
            4 => Self::Main10444StillPicture,
            17 => Self::Multilayer,
            65 => Self::MultilayerMain10,
            _ => Self::Unknown(value),
        }
    }
}

/// Profile, Tier, Level information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileTierLevel {
    /// Profile IDC.
    pub general_profile_idc: Profile,
    /// Tier flag (0 = Main, 1 = High).
    pub general_tier_flag: bool,
    /// Level IDC.
    pub general_level_idc: u8,
    /// Frame only constraint flag.
    pub ptl_frame_only_constraint_flag: bool,
    /// Multilayer enabled flag.
    pub ptl_multilayer_enabled_flag: bool,
}

impl Default for ProfileTierLevel {
    fn default() -> Self {
        Self {
            general_profile_idc: Profile::Main10,
            general_tier_flag: false,
            general_level_idc: 0,
            ptl_frame_only_constraint_flag: true,
            ptl_multilayer_enabled_flag: false,
        }
    }
}

/// Dual tree configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DualTreeConfig {
    /// Dual tree for intra slices.
    pub qtbtt_dual_tree_intra_flag: bool,
    /// Max MTT depth for intra luma.
    pub max_mtt_hierarchy_depth_intra_slice_luma: u8,
    /// Max MTT depth for intra chroma.
    pub max_mtt_hierarchy_depth_intra_slice_chroma: u8,
    /// Max MTT depth for inter.
    pub max_mtt_hierarchy_depth_inter_slice: u8,
}

/// ALF (Adaptive Loop Filter) configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AlfConfig {
    /// ALF enabled.
    pub alf_enabled_flag: bool,
    /// Cross-component ALF for Cb.
    pub ccalf_enabled_flag: bool,
}

/// LMCS (Luma Mapping with Chroma Scaling) configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LmcsConfig {
    /// LMCS enabled.
    pub lmcs_enabled_flag: bool,
}

/// VVC Sequence Parameter Set.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sps {
    /// SPS ID (0-15).
    pub sps_seq_parameter_set_id: u8,
    /// VPS ID.
    pub sps_video_parameter_set_id: u8,
    /// Maximum sub-layers.
    pub sps_max_sublayers_minus1: u8,
    /// Chroma format.
    pub sps_chroma_format_idc: ChromaFormat,
    /// Log2 CTU size.
    pub sps_log2_ctu_size_minus5: u8,
    /// Subpictures present.
    pub sps_subpic_info_present_flag: bool,
    /// Number of subpictures.
    pub sps_num_subpics_minus1: u16,
    /// Picture width in luma samples.
    pub sps_pic_width_max_in_luma_samples: u32,
    /// Picture height in luma samples.
    pub sps_pic_height_max_in_luma_samples: u32,
    /// Conformance window present.
    pub sps_conformance_window_flag: bool,
    /// Conformance window offsets.
    pub sps_conf_win_left_offset: u32,
    pub sps_conf_win_right_offset: u32,
    pub sps_conf_win_top_offset: u32,
    pub sps_conf_win_bottom_offset: u32,
    /// Bit depth.
    pub sps_bitdepth_minus8: u8,
    /// Log2 min luma coding block size.
    pub sps_log2_min_luma_coding_block_size_minus2: u8,
    /// POC MSB cycle present.
    pub sps_poc_msb_cycle_flag: bool,
    /// Log2 max POC LSB.
    pub sps_log2_max_pic_order_cnt_lsb_minus4: u8,
    /// Profile, tier, level.
    pub profile_tier_level: ProfileTierLevel,
    /// GDR enabled.
    pub sps_gdr_enabled_flag: bool,
    /// Reference picture resampling.
    pub sps_ref_pic_resampling_enabled_flag: bool,
    /// Dual tree configuration.
    pub dual_tree: DualTreeConfig,
    /// ALF configuration.
    pub alf: AlfConfig,
    /// LMCS configuration.
    pub lmcs: LmcsConfig,
    /// Transform skip enabled.
    pub sps_transform_skip_enabled_flag: bool,
    /// BDPCM enabled.
    pub sps_bdpcm_enabled_flag: bool,
    /// MTS intra enabled.
    pub sps_mts_enabled_flag: bool,
    /// LFNST enabled.
    pub sps_lfnst_enabled_flag: bool,
    /// Joint Cb-Cr residual enabled.
    pub sps_joint_cbcr_enabled_flag: bool,
    /// Same QP table for Cb and Cr.
    pub sps_same_qp_table_for_chroma_flag: bool,
    /// SAO enabled.
    pub sps_sao_enabled_flag: bool,
    /// Deblocking filter control present.
    pub sps_deblocking_filter_control_present_flag: bool,
    /// Temporal MVP enabled.
    pub sps_temporal_mvp_enabled_flag: bool,
    /// MMVD enabled.
    pub sps_mmvd_enabled_flag: bool,
    /// Affine enabled.
    pub sps_affine_enabled_flag: bool,
    /// BCW (Bi-prediction with CU-level Weights) enabled.
    pub sps_bcw_enabled_flag: bool,
    /// IBC (Intra Block Copy) enabled.
    pub sps_ibc_enabled_flag: bool,
    /// CIIP (Combined Inter-Intra Prediction) enabled.
    pub sps_ciip_enabled_flag: bool,
    /// GPM (Geometric Partition Mode) enabled.
    pub sps_gpm_enabled_flag: bool,
}

impl Default for Sps {
    fn default() -> Self {
        Self {
            sps_seq_parameter_set_id: 0,
            sps_video_parameter_set_id: 0,
            sps_max_sublayers_minus1: 0,
            sps_chroma_format_idc: ChromaFormat::Chroma420,
            sps_log2_ctu_size_minus5: 2,
            sps_subpic_info_present_flag: false,
            sps_num_subpics_minus1: 0,
            sps_pic_width_max_in_luma_samples: 0,
            sps_pic_height_max_in_luma_samples: 0,
            sps_conformance_window_flag: false,
            sps_conf_win_left_offset: 0,
            sps_conf_win_right_offset: 0,
            sps_conf_win_top_offset: 0,
            sps_conf_win_bottom_offset: 0,
            sps_bitdepth_minus8: 2, // 10-bit default
            sps_log2_min_luma_coding_block_size_minus2: 0,
            sps_poc_msb_cycle_flag: false,
            sps_log2_max_pic_order_cnt_lsb_minus4: 4,
            profile_tier_level: ProfileTierLevel::default(),
            sps_gdr_enabled_flag: false,
            sps_ref_pic_resampling_enabled_flag: false,
            dual_tree: DualTreeConfig::default(),
            alf: AlfConfig::default(),
            lmcs: LmcsConfig::default(),
            sps_transform_skip_enabled_flag: false,
            sps_bdpcm_enabled_flag: false,
            sps_mts_enabled_flag: false,
            sps_lfnst_enabled_flag: false,
            sps_joint_cbcr_enabled_flag: false,
            sps_same_qp_table_for_chroma_flag: true,
            sps_sao_enabled_flag: true,
            sps_deblocking_filter_control_present_flag: false,
            sps_temporal_mvp_enabled_flag: true,
            sps_mmvd_enabled_flag: false,
            sps_affine_enabled_flag: false,
            sps_bcw_enabled_flag: false,
            sps_ibc_enabled_flag: false,
            sps_ciip_enabled_flag: false,
            sps_gpm_enabled_flag: false,
        }
    }
}

impl Sps {
    /// Get bit depth.
    pub fn bit_depth(&self) -> u8 {
        self.sps_bitdepth_minus8 + 8
    }

    /// Get CTU size.
    pub fn ctu_size(&self) -> u32 {
        1 << (self.sps_log2_ctu_size_minus5 + 5)
    }

    /// Get min coding block size.
    pub fn min_cb_size(&self) -> u32 {
        1 << (self.sps_log2_min_luma_coding_block_size_minus2 + 2)
    }

    /// Get picture width in CTUs.
    pub fn pic_width_in_ctus(&self) -> u32 {
        let ctu_size = self.ctu_size();
        self.sps_pic_width_max_in_luma_samples.div_ceil(ctu_size)
    }

    /// Get picture height in CTUs.
    pub fn pic_height_in_ctus(&self) -> u32 {
        let ctu_size = self.ctu_size();
        self.sps_pic_height_max_in_luma_samples.div_ceil(ctu_size)
    }

    /// Get display width (accounting for conformance window).
    pub fn display_width(&self) -> u32 {
        let sub_width_c = match self.sps_chroma_format_idc {
            ChromaFormat::Chroma420 | ChromaFormat::Chroma422 => 2,
            _ => 1,
        };
        self.sps_pic_width_max_in_luma_samples
            - sub_width_c * (self.sps_conf_win_left_offset + self.sps_conf_win_right_offset)
    }

    /// Get display height (accounting for conformance window).
    pub fn display_height(&self) -> u32 {
        let sub_height_c = match self.sps_chroma_format_idc {
            ChromaFormat::Chroma420 => 2,
            _ => 1,
        };
        self.sps_pic_height_max_in_luma_samples
            - sub_height_c * (self.sps_conf_win_top_offset + self.sps_conf_win_bottom_offset)
    }

    /// Get max POC LSB value.
    pub fn max_poc_lsb(&self) -> u32 {
        1 << (self.sps_log2_max_pic_order_cnt_lsb_minus4 + 4)
    }

    /// Check if dual tree is enabled for intra.
    pub fn has_dual_tree_intra(&self) -> bool {
        self.dual_tree.qtbtt_dual_tree_intra_flag
    }

    /// Get profile name.
    pub fn profile_name(&self) -> &'static str {
        match self.profile_tier_level.general_profile_idc {
            Profile::Main10 => "Main 10",
            Profile::Main10StillPicture => "Main 10 Still Picture",
            Profile::Main10444 => "Main 10 4:4:4",
            Profile::Main10444StillPicture => "Main 10 4:4:4 Still Picture",
            Profile::Multilayer => "Multilayer",
            Profile::MultilayerMain10 => "Multilayer Main 10",
            Profile::Unknown(_) => "Unknown",
        }
    }

    /// Get tier name.
    pub fn tier_name(&self) -> &'static str {
        if self.profile_tier_level.general_tier_flag {
            "High"
        } else {
            "Main"
        }
    }

    /// Get level as decimal (e.g., 5.1).
    pub fn level(&self) -> f32 {
        self.profile_tier_level.general_level_idc as f32 / 16.0
    }
}

/// Parse SPS from RBSP data.
pub fn parse_sps(data: &[u8]) -> Result<Sps> {
    let mut reader = BitReader::new(data);
    let mut sps = Sps::default();

    // sps_seq_parameter_set_id (4 bits)
    sps.sps_seq_parameter_set_id = reader.read_bits(4)? as u8;

    // sps_video_parameter_set_id (4 bits)
    sps.sps_video_parameter_set_id = reader.read_bits(4)? as u8;

    // sps_max_sublayers_minus1 (3 bits)
    sps.sps_max_sublayers_minus1 = reader.read_bits(3)? as u8;

    // sps_chroma_format_idc (2 bits)
    sps.sps_chroma_format_idc = ChromaFormat::from(reader.read_bits(2)? as u8);

    // sps_log2_ctu_size_minus5 (2 bits)
    sps.sps_log2_ctu_size_minus5 = reader.read_bits(2)? as u8;

    // sps_ptl_dpb_hrd_params_present_flag (1 bit)
    let ptl_present = reader.read_bit()?;

    if ptl_present {
        // Parse profile_tier_level (simplified)
        sps.profile_tier_level =
            parse_profile_tier_level(&mut reader, sps.sps_max_sublayers_minus1)?;
    }

    // sps_gdr_enabled_flag (1 bit)
    sps.sps_gdr_enabled_flag = reader.read_bit()?;

    // sps_ref_pic_resampling_enabled_flag (1 bit)
    sps.sps_ref_pic_resampling_enabled_flag = reader.read_bit()?;

    if sps.sps_ref_pic_resampling_enabled_flag {
        // sps_res_change_in_clvs_allowed_flag (1 bit)
        let _ = reader.read_bit()?;
    }

    // sps_pic_width_max_in_luma_samples (ue(v))
    sps.sps_pic_width_max_in_luma_samples = reader.read_ue()?;

    // sps_pic_height_max_in_luma_samples (ue(v))
    sps.sps_pic_height_max_in_luma_samples = reader.read_ue()?;

    // Validate dimensions to prevent DoS via excessive allocations
    // Maximum 8K resolution (7680x4320) with safety margin
    const MAX_WIDTH: u32 = 7680;
    const MAX_HEIGHT: u32 = 4320;

    if sps.sps_pic_width_max_in_luma_samples > MAX_WIDTH
        || sps.sps_pic_height_max_in_luma_samples > MAX_HEIGHT
    {
        return Err(crate::error::VvcError::InvalidData(format!(
            "SPS dimensions {}x{} exceed maximum {}x{}",
            sps.sps_pic_width_max_in_luma_samples,
            sps.sps_pic_height_max_in_luma_samples,
            MAX_WIDTH,
            MAX_HEIGHT
        )));
    }

    // Also validate minimum dimensions
    if sps.sps_pic_width_max_in_luma_samples == 0 || sps.sps_pic_height_max_in_luma_samples == 0 {
        return Err(crate::error::VvcError::InvalidData(format!(
            "SPS dimensions {}x{} are invalid (must be non-zero)",
            sps.sps_pic_width_max_in_luma_samples, sps.sps_pic_height_max_in_luma_samples
        )));
    }

    // sps_conformance_window_flag (1 bit)
    sps.sps_conformance_window_flag = reader.read_bit()?;

    if sps.sps_conformance_window_flag {
        sps.sps_conf_win_left_offset = reader.read_ue()?;
        sps.sps_conf_win_right_offset = reader.read_ue()?;
        sps.sps_conf_win_top_offset = reader.read_ue()?;
        sps.sps_conf_win_bottom_offset = reader.read_ue()?;
    }

    // sps_subpic_info_present_flag (1 bit)
    sps.sps_subpic_info_present_flag = reader.read_bit()?;

    if sps.sps_subpic_info_present_flag {
        // sps_num_subpics_minus1 (ue(v))
        sps.sps_num_subpics_minus1 = reader.read_ue()? as u16;
        // Skip subpic parsing for now
    }

    // sps_bitdepth_minus8 (ue(v))
    sps.sps_bitdepth_minus8 = reader.read_ue()? as u8;

    // More fields would be parsed here...
    // For now, we have the essential fields

    Ok(sps)
}

fn parse_profile_tier_level(
    reader: &mut BitReader,
    #[allow(unused_variables)] max_sublayers_minus1: u8,
) -> Result<ProfileTierLevel> {
    let mut ptl = ProfileTierLevel::default();

    // general_profile_idc (7 bits)
    ptl.general_profile_idc = Profile::from(reader.read_bits(7)? as u8);

    // general_tier_flag (1 bit)
    ptl.general_tier_flag = reader.read_bit()?;

    // general_level_idc (8 bits)
    ptl.general_level_idc = reader.read_bits(8)? as u8;

    // ptl_frame_only_constraint_flag (1 bit)
    ptl.ptl_frame_only_constraint_flag = reader.read_bit()?;

    // ptl_multilayer_enabled_flag (1 bit)
    ptl.ptl_multilayer_enabled_flag = reader.read_bit()?;

    // Skip general constraint info and sublayer info
    // This is a simplified parse

    Ok(ptl)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sps_defaults() {
        let sps = Sps::default();
        assert_eq!(sps.bit_depth(), 10);
        assert_eq!(sps.ctu_size(), 128); // 2^(2+5) = 128
    }

    #[test]
    fn test_profile_names() {
        let mut sps = Sps::default();

        sps.profile_tier_level.general_profile_idc = Profile::Main10;
        assert_eq!(sps.profile_name(), "Main 10");

        sps.profile_tier_level.general_profile_idc = Profile::Main10444;
        assert_eq!(sps.profile_name(), "Main 10 4:4:4");
    }

    #[test]
    fn test_ctu_calculations() {
        let mut sps = Sps::default();
        sps.sps_pic_width_max_in_luma_samples = 1920;
        sps.sps_pic_height_max_in_luma_samples = 1080;

        // CTU size = 128
        assert_eq!(sps.pic_width_in_ctus(), 15); // ceil(1920/128)
        assert_eq!(sps.pic_height_in_ctus(), 9); // ceil(1080/128)
    }
}
