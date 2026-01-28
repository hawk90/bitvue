//! HEVC Sequence Parameter Set (SPS) parsing.
//!
//! The SPS contains sequence-level coding parameters.

use crate::bitreader::BitReader;
use crate::error::Result;
use serde::{Deserialize, Serialize};

/// HEVC Profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Profile {
    Main,
    Main10,
    MainStillPicture,
    RangeExtensions,
    HighThroughput,
    Multiview,
    Scalable,
    Main3d,
    ScreenExtended,
    ScalableRangeExtensions,
    HighThroughputScreenExtended,
    Unknown(u8),
}

impl Profile {
    /// Get the profile IDC value.
    pub fn idc(&self) -> u8 {
        match self {
            Self::Main => 1,
            Self::Main10 => 2,
            Self::MainStillPicture => 3,
            Self::RangeExtensions => 4,
            Self::HighThroughput => 5,
            Self::Multiview => 6,
            Self::Scalable => 7,
            Self::Main3d => 8,
            Self::ScreenExtended => 9,
            Self::ScalableRangeExtensions => 10,
            Self::HighThroughputScreenExtended => 11,
            Self::Unknown(v) => *v,
        }
    }
}

impl From<u8> for Profile {
    fn from(value: u8) -> Self {
        match value {
            1 => Self::Main,
            2 => Self::Main10,
            3 => Self::MainStillPicture,
            4 => Self::RangeExtensions,
            5 => Self::HighThroughput,
            6 => Self::Multiview,
            7 => Self::Scalable,
            8 => Self::Main3d,
            9 => Self::ScreenExtended,
            10 => Self::ScalableRangeExtensions,
            11 => Self::HighThroughputScreenExtended,
            _ => Self::Unknown(value),
        }
    }
}

/// Profile, Tier, Level information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileTierLevel {
    pub general_profile_space: u8,
    pub general_tier_flag: bool,
    pub general_profile_idc: Profile,
    pub general_profile_compatibility_flags: u32,
    pub general_progressive_source_flag: bool,
    pub general_interlaced_source_flag: bool,
    pub general_non_packed_constraint_flag: bool,
    pub general_frame_only_constraint_flag: bool,
    pub general_level_idc: u8,
}

/// Chroma format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChromaFormat {
    Monochrome = 0,
    Chroma420 = 1,
    Chroma422 = 2,
    Chroma444 = 3,
}

impl From<u8> for ChromaFormat {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::Monochrome,
            1 => Self::Chroma420,
            2 => Self::Chroma422,
            3 => Self::Chroma444,
            _ => Self::Chroma420, // Default
        }
    }
}

/// Video Usability Information (VUI).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VuiParameters {
    pub aspect_ratio_info_present_flag: bool,
    pub aspect_ratio_idc: Option<u8>,
    pub sar_width: Option<u16>,
    pub sar_height: Option<u16>,
    pub overscan_info_present_flag: bool,
    pub overscan_appropriate_flag: Option<bool>,
    pub video_signal_type_present_flag: bool,
    pub video_format: Option<u8>,
    pub video_full_range_flag: Option<bool>,
    pub colour_description_present_flag: Option<bool>,
    pub colour_primaries: Option<u8>,
    pub transfer_characteristics: Option<u8>,
    pub matrix_coeffs: Option<u8>,
    pub chroma_loc_info_present_flag: bool,
    pub timing_info_present_flag: bool,
    pub num_units_in_tick: Option<u32>,
    pub time_scale: Option<u32>,
}

/// Sequence Parameter Set.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sps {
    /// SPS ID (0-15).
    pub sps_video_parameter_set_id: u8,
    /// Maximum number of temporal sub-layers.
    pub sps_max_sub_layers_minus1: u8,
    /// Temporal ID nesting flag.
    pub sps_temporal_id_nesting_flag: bool,
    /// Profile, tier, level.
    pub profile_tier_level: ProfileTierLevel,
    /// SPS ID.
    pub sps_seq_parameter_set_id: u8,
    /// Chroma format.
    pub chroma_format_idc: ChromaFormat,
    /// Separate color plane flag (for 4:4:4).
    pub separate_colour_plane_flag: bool,
    /// Picture width in luma samples.
    pub pic_width_in_luma_samples: u32,
    /// Picture height in luma samples.
    pub pic_height_in_luma_samples: u32,
    /// Conformance window flag.
    pub conformance_window_flag: bool,
    /// Conformance window offsets.
    pub conf_win_left_offset: u32,
    pub conf_win_right_offset: u32,
    pub conf_win_top_offset: u32,
    pub conf_win_bottom_offset: u32,
    /// Bit depth for luma samples.
    pub bit_depth_luma_minus8: u8,
    /// Bit depth for chroma samples.
    pub bit_depth_chroma_minus8: u8,
    /// Log2 of max POC LSB.
    pub log2_max_pic_order_cnt_lsb_minus4: u8,
    /// Sub-layer ordering info present.
    pub sps_sub_layer_ordering_info_present_flag: bool,
    /// Max DPB size per sub-layer.
    pub sps_max_dec_pic_buffering_minus1: Vec<u8>,
    /// Max num reorder pics per sub-layer.
    pub sps_max_num_reorder_pics: Vec<u8>,
    /// Max latency increase per sub-layer.
    pub sps_max_latency_increase_plus1: Vec<u32>,
    /// Log2 of min luma coding block size.
    pub log2_min_luma_coding_block_size_minus3: u8,
    /// Log2 diff of max and min luma coding block size.
    pub log2_diff_max_min_luma_coding_block_size: u8,
    /// Log2 of min luma transform block size.
    pub log2_min_luma_transform_block_size_minus2: u8,
    /// Log2 diff of max and min luma transform block size.
    pub log2_diff_max_min_luma_transform_block_size: u8,
    /// Max transform hierarchy depth for inter.
    pub max_transform_hierarchy_depth_inter: u8,
    /// Max transform hierarchy depth for intra.
    pub max_transform_hierarchy_depth_intra: u8,
    /// Scaling list enabled.
    pub scaling_list_enabled_flag: bool,
    /// AMP (asymmetric motion partition) enabled.
    pub amp_enabled_flag: bool,
    /// Sample adaptive offset enabled.
    pub sample_adaptive_offset_enabled_flag: bool,
    /// PCM enabled.
    pub pcm_enabled_flag: bool,
    /// Number of short-term reference picture sets.
    pub num_short_term_ref_pic_sets: u8,
    /// Long-term reference pics present.
    pub long_term_ref_pics_present_flag: bool,
    /// Number of long-term reference pics in SPS.
    pub num_long_term_ref_pics_sps: u8,
    /// Temporal MVP enabled.
    pub sps_temporal_mvp_enabled_flag: bool,
    /// Strong intra smoothing enabled.
    pub strong_intra_smoothing_enabled_flag: bool,
    /// VUI parameters present.
    pub vui_parameters_present_flag: bool,
    /// VUI parameters.
    pub vui_parameters: Option<VuiParameters>,
}

impl Sps {
    /// Get the actual bit depth for luma.
    pub fn bit_depth_luma(&self) -> u8 {
        self.bit_depth_luma_minus8 + 8
    }

    /// Get the actual bit depth for chroma.
    pub fn bit_depth_chroma(&self) -> u8 {
        self.bit_depth_chroma_minus8 + 8
    }

    /// Get the CTB (Coding Tree Block) size.
    pub fn ctb_size(&self) -> u32 {
        1 << (self.log2_min_luma_coding_block_size_minus3
            + 3
            + self.log2_diff_max_min_luma_coding_block_size)
    }

    /// Get min CB size.
    pub fn min_cb_size(&self) -> u32 {
        1 << (self.log2_min_luma_coding_block_size_minus3 + 3)
    }

    /// Get picture width in CTBs.
    pub fn pic_width_in_ctbs(&self) -> u32 {
        let ctb_size = self.ctb_size();
        (self.pic_width_in_luma_samples + ctb_size - 1) / ctb_size
    }

    /// Get picture height in CTBs.
    pub fn pic_height_in_ctbs(&self) -> u32 {
        let ctb_size = self.ctb_size();
        (self.pic_height_in_luma_samples + ctb_size - 1) / ctb_size
    }

    /// Get the display width (accounting for conformance window).
    pub fn display_width(&self) -> u32 {
        let sub_width_c = match self.chroma_format_idc {
            ChromaFormat::Chroma420 | ChromaFormat::Chroma422 => 2,
            _ => 1,
        };
        self.pic_width_in_luma_samples
            - sub_width_c * (self.conf_win_left_offset + self.conf_win_right_offset)
    }

    /// Get the display height (accounting for conformance window).
    pub fn display_height(&self) -> u32 {
        let sub_height_c = match self.chroma_format_idc {
            ChromaFormat::Chroma420 => 2,
            _ => 1,
        };
        self.pic_height_in_luma_samples
            - sub_height_c * (self.conf_win_top_offset + self.conf_win_bottom_offset)
    }

    /// Get max POC LSB value.
    pub fn max_poc_lsb(&self) -> u32 {
        1 << (self.log2_max_pic_order_cnt_lsb_minus4 + 4)
    }
}

/// Parse profile_tier_level structure.
fn parse_profile_tier_level(
    reader: &mut BitReader,
    profile_present_flag: bool,
    max_num_sub_layers_minus1: u8,
) -> Result<ProfileTierLevel> {
    let mut ptl = ProfileTierLevel {
        general_profile_space: 0,
        general_tier_flag: false,
        general_profile_idc: Profile::Main,
        general_profile_compatibility_flags: 0,
        general_progressive_source_flag: false,
        general_interlaced_source_flag: false,
        general_non_packed_constraint_flag: false,
        general_frame_only_constraint_flag: false,
        general_level_idc: 0,
    };

    if profile_present_flag {
        ptl.general_profile_space = reader.read_bits(2)? as u8;
        ptl.general_tier_flag = reader.read_bit()?;
        ptl.general_profile_idc = Profile::from(reader.read_bits(5)? as u8);
        ptl.general_profile_compatibility_flags = reader.read_bits(32)?;
        ptl.general_progressive_source_flag = reader.read_bit()?;
        ptl.general_interlaced_source_flag = reader.read_bit()?;
        ptl.general_non_packed_constraint_flag = reader.read_bit()?;
        ptl.general_frame_only_constraint_flag = reader.read_bit()?;
        // Skip reserved bits (43 or 44 bits depending on profile)
        reader.skip_bits(44)?;
    }

    ptl.general_level_idc = reader.read_bits(8)? as u8;

    // Skip sub-layer flags
    let mut sub_layer_profile_present = vec![false; max_num_sub_layers_minus1 as usize];
    let mut sub_layer_level_present = vec![false; max_num_sub_layers_minus1 as usize];

    for i in 0..max_num_sub_layers_minus1 as usize {
        sub_layer_profile_present[i] = reader.read_bit()?;
        sub_layer_level_present[i] = reader.read_bit()?;
    }

    if max_num_sub_layers_minus1 > 0 {
        for _ in max_num_sub_layers_minus1..8 {
            reader.skip_bits(2)?;
        }
    }

    // Skip sub-layer profile_tier_level (simplified)
    for i in 0..max_num_sub_layers_minus1 as usize {
        if sub_layer_profile_present[i] {
            reader.skip_bits(88)?;
        }
        if sub_layer_level_present[i] {
            reader.skip_bits(8)?;
        }
    }

    Ok(ptl)
}

/// Parse SPS from RBSP data.
pub fn parse_sps(rbsp: &[u8]) -> Result<Sps> {
    let mut reader = BitReader::new(rbsp);

    let sps_video_parameter_set_id = reader.read_bits(4)? as u8;
    let sps_max_sub_layers_minus1 = reader.read_bits(3)? as u8;
    let sps_temporal_id_nesting_flag = reader.read_bit()?;

    let profile_tier_level =
        parse_profile_tier_level(&mut reader, true, sps_max_sub_layers_minus1)?;

    let sps_seq_parameter_set_id = reader.read_ue()? as u8;
    let chroma_format_idc = ChromaFormat::from(reader.read_ue()? as u8);

    let separate_colour_plane_flag = if chroma_format_idc == ChromaFormat::Chroma444 {
        reader.read_bit()?
    } else {
        false
    };

    let pic_width_in_luma_samples = reader.read_ue()?;
    let pic_height_in_luma_samples = reader.read_ue()?;

    let conformance_window_flag = reader.read_bit()?;
    let (conf_win_left_offset, conf_win_right_offset, conf_win_top_offset, conf_win_bottom_offset) =
        if conformance_window_flag {
            (
                reader.read_ue()?,
                reader.read_ue()?,
                reader.read_ue()?,
                reader.read_ue()?,
            )
        } else {
            (0, 0, 0, 0)
        };

    let bit_depth_luma_minus8 = reader.read_ue()? as u8;
    let bit_depth_chroma_minus8 = reader.read_ue()? as u8;
    let log2_max_pic_order_cnt_lsb_minus4 = reader.read_ue()? as u8;

    let sps_sub_layer_ordering_info_present_flag = reader.read_bit()?;
    let start_idx = if sps_sub_layer_ordering_info_present_flag {
        0
    } else {
        sps_max_sub_layers_minus1
    };

    let mut sps_max_dec_pic_buffering_minus1 = vec![0u8; sps_max_sub_layers_minus1 as usize + 1];
    let mut sps_max_num_reorder_pics = vec![0u8; sps_max_sub_layers_minus1 as usize + 1];
    let mut sps_max_latency_increase_plus1 = vec![0u32; sps_max_sub_layers_minus1 as usize + 1];

    for i in start_idx..=sps_max_sub_layers_minus1 {
        sps_max_dec_pic_buffering_minus1[i as usize] = reader.read_ue()? as u8;
        sps_max_num_reorder_pics[i as usize] = reader.read_ue()? as u8;
        sps_max_latency_increase_plus1[i as usize] = reader.read_ue()?;
    }

    let log2_min_luma_coding_block_size_minus3 = reader.read_ue()? as u8;
    let log2_diff_max_min_luma_coding_block_size = reader.read_ue()? as u8;
    let log2_min_luma_transform_block_size_minus2 = reader.read_ue()? as u8;
    let log2_diff_max_min_luma_transform_block_size = reader.read_ue()? as u8;
    let max_transform_hierarchy_depth_inter = reader.read_ue()? as u8;
    let max_transform_hierarchy_depth_intra = reader.read_ue()? as u8;

    let scaling_list_enabled_flag = reader.read_bit()?;
    if scaling_list_enabled_flag {
        let sps_scaling_list_data_present_flag = reader.read_bit()?;
        if sps_scaling_list_data_present_flag {
            // Skip scaling list data (complex structure)
            // For now, we don't fully parse this
        }
    }

    let amp_enabled_flag = reader.read_bit()?;
    let sample_adaptive_offset_enabled_flag = reader.read_bit()?;

    let pcm_enabled_flag = reader.read_bit()?;
    if pcm_enabled_flag {
        let _pcm_sample_bit_depth_luma_minus1 = reader.read_bits(4)?;
        let _pcm_sample_bit_depth_chroma_minus1 = reader.read_bits(4)?;
        let _log2_min_pcm_luma_coding_block_size_minus3 = reader.read_ue()?;
        let _log2_diff_max_min_pcm_luma_coding_block_size = reader.read_ue()?;
        let _pcm_loop_filter_disabled_flag = reader.read_bit()?;
    }

    let num_short_term_ref_pic_sets = reader.read_ue()? as u8;
    // Skip short-term RPS parsing (complex)
    // This would require full st_ref_pic_set parsing

    let long_term_ref_pics_present_flag = reader.read_bit()?;
    let num_long_term_ref_pics_sps = if long_term_ref_pics_present_flag {
        let count = reader.read_ue()? as u8;
        for _ in 0..count {
            let _lt_ref_pic_poc_lsb_sps =
                reader.read_bits(log2_max_pic_order_cnt_lsb_minus4 + 4)?;
            let _used_by_curr_pic_lt_sps_flag = reader.read_bit()?;
        }
        count
    } else {
        0
    };

    let sps_temporal_mvp_enabled_flag = reader.read_bit()?;
    let strong_intra_smoothing_enabled_flag = reader.read_bit()?;

    let vui_parameters_present_flag = reader.read_bit()?;
    let vui_parameters = if vui_parameters_present_flag {
        Some(parse_vui_parameters(&mut reader)?)
    } else {
        None
    };

    Ok(Sps {
        sps_video_parameter_set_id,
        sps_max_sub_layers_minus1,
        sps_temporal_id_nesting_flag,
        profile_tier_level,
        sps_seq_parameter_set_id,
        chroma_format_idc,
        separate_colour_plane_flag,
        pic_width_in_luma_samples,
        pic_height_in_luma_samples,
        conformance_window_flag,
        conf_win_left_offset,
        conf_win_right_offset,
        conf_win_top_offset,
        conf_win_bottom_offset,
        bit_depth_luma_minus8,
        bit_depth_chroma_minus8,
        log2_max_pic_order_cnt_lsb_minus4,
        sps_sub_layer_ordering_info_present_flag,
        sps_max_dec_pic_buffering_minus1,
        sps_max_num_reorder_pics,
        sps_max_latency_increase_plus1,
        log2_min_luma_coding_block_size_minus3,
        log2_diff_max_min_luma_coding_block_size,
        log2_min_luma_transform_block_size_minus2,
        log2_diff_max_min_luma_transform_block_size,
        max_transform_hierarchy_depth_inter,
        max_transform_hierarchy_depth_intra,
        scaling_list_enabled_flag,
        amp_enabled_flag,
        sample_adaptive_offset_enabled_flag,
        pcm_enabled_flag,
        num_short_term_ref_pic_sets,
        long_term_ref_pics_present_flag,
        num_long_term_ref_pics_sps,
        sps_temporal_mvp_enabled_flag,
        strong_intra_smoothing_enabled_flag,
        vui_parameters_present_flag,
        vui_parameters,
    })
}

/// Parse VUI parameters (simplified).
fn parse_vui_parameters(reader: &mut BitReader) -> Result<VuiParameters> {
    let mut vui = VuiParameters::default();

    vui.aspect_ratio_info_present_flag = reader.read_bit()?;
    if vui.aspect_ratio_info_present_flag {
        vui.aspect_ratio_idc = Some(reader.read_bits(8)? as u8);
        if vui.aspect_ratio_idc == Some(255) {
            // Extended_SAR
            vui.sar_width = Some(reader.read_bits(16)? as u16);
            vui.sar_height = Some(reader.read_bits(16)? as u16);
        }
    }

    vui.overscan_info_present_flag = reader.read_bit()?;
    if vui.overscan_info_present_flag {
        vui.overscan_appropriate_flag = Some(reader.read_bit()?);
    }

    vui.video_signal_type_present_flag = reader.read_bit()?;
    if vui.video_signal_type_present_flag {
        vui.video_format = Some(reader.read_bits(3)? as u8);
        vui.video_full_range_flag = Some(reader.read_bit()?);
        vui.colour_description_present_flag = Some(reader.read_bit()?);
        if vui.colour_description_present_flag == Some(true) {
            vui.colour_primaries = Some(reader.read_bits(8)? as u8);
            vui.transfer_characteristics = Some(reader.read_bits(8)? as u8);
            vui.matrix_coeffs = Some(reader.read_bits(8)? as u8);
        }
    }

    vui.chroma_loc_info_present_flag = reader.read_bit()?;
    if vui.chroma_loc_info_present_flag {
        let _chroma_sample_loc_type_top_field = reader.read_ue()?;
        let _chroma_sample_loc_type_bottom_field = reader.read_ue()?;
    }

    // Skip some flags
    let _neutral_chroma_indication_flag = reader.read_bit()?;
    let _field_seq_flag = reader.read_bit()?;
    let _frame_field_info_present_flag = reader.read_bit()?;

    let default_display_window_flag = reader.read_bit()?;
    if default_display_window_flag {
        let _def_disp_win_left_offset = reader.read_ue()?;
        let _def_disp_win_right_offset = reader.read_ue()?;
        let _def_disp_win_top_offset = reader.read_ue()?;
        let _def_disp_win_bottom_offset = reader.read_ue()?;
    }

    vui.timing_info_present_flag = reader.read_bit()?;
    if vui.timing_info_present_flag {
        vui.num_units_in_tick = Some(reader.read_bits(32)?);
        vui.time_scale = Some(reader.read_bits(32)?);
        // More timing info could be parsed here
    }

    Ok(vui)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sps_derived_values() {
        let sps = Sps {
            sps_video_parameter_set_id: 0,
            sps_max_sub_layers_minus1: 0,
            sps_temporal_id_nesting_flag: true,
            profile_tier_level: ProfileTierLevel {
                general_profile_space: 0,
                general_tier_flag: false,
                general_profile_idc: Profile::Main,
                general_profile_compatibility_flags: 0,
                general_progressive_source_flag: true,
                general_interlaced_source_flag: false,
                general_non_packed_constraint_flag: false,
                general_frame_only_constraint_flag: true,
                general_level_idc: 120, // Level 4.0
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
            log2_max_pic_order_cnt_lsb_minus4: 4,
            sps_sub_layer_ordering_info_present_flag: false,
            sps_max_dec_pic_buffering_minus1: vec![4],
            sps_max_num_reorder_pics: vec![2],
            sps_max_latency_increase_plus1: vec![0],
            log2_min_luma_coding_block_size_minus3: 0, // MinCbSize = 8
            log2_diff_max_min_luma_coding_block_size: 3, // CTB = 64
            log2_min_luma_transform_block_size_minus2: 0,
            log2_diff_max_min_luma_transform_block_size: 3,
            max_transform_hierarchy_depth_inter: 2,
            max_transform_hierarchy_depth_intra: 2,
            scaling_list_enabled_flag: false,
            amp_enabled_flag: true,
            sample_adaptive_offset_enabled_flag: true,
            pcm_enabled_flag: false,
            num_short_term_ref_pic_sets: 0,
            long_term_ref_pics_present_flag: false,
            num_long_term_ref_pics_sps: 0,
            sps_temporal_mvp_enabled_flag: true,
            strong_intra_smoothing_enabled_flag: true,
            vui_parameters_present_flag: false,
            vui_parameters: None,
        };

        assert_eq!(sps.bit_depth_luma(), 8);
        assert_eq!(sps.bit_depth_chroma(), 8);
        assert_eq!(sps.ctb_size(), 64);
        assert_eq!(sps.min_cb_size(), 8);
        assert_eq!(sps.pic_width_in_ctbs(), 30); // 1920 / 64 = 30
        assert_eq!(sps.pic_height_in_ctbs(), 17); // 1080 / 64 = 16.875 -> 17
        assert_eq!(sps.max_poc_lsb(), 256); // 2^8
    }
}
