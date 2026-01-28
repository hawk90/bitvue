//! HEVC Video Parameter Set (VPS) parsing.
//!
//! VPS contains video-level parameters that apply to all layers and sub-layers.
//! It is defined in ITU-T H.265 Section 7.3.2.1.

use crate::bitreader::BitReader;
use crate::error::Result;
use serde::{Deserialize, Serialize};

/// Timing information for VPS/SPS.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TimingInfo {
    /// Number of time units in one tick.
    pub num_units_in_tick: u32,
    /// Time scale (ticks per second).
    pub time_scale: u32,
    /// POC proportional to timing flag.
    pub poc_proportional_to_timing_flag: bool,
    /// Number of ticks per POC difference.
    pub num_ticks_poc_diff_one_minus1: u32,
}

/// HRD (Hypothetical Reference Decoder) parameters.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HrdParameters {
    /// NAL HRD parameters present.
    pub nal_hrd_parameters_present_flag: bool,
    /// VCL HRD parameters present.
    pub vcl_hrd_parameters_present_flag: bool,
    /// Sub-picture CPB parameters present.
    pub sub_pic_cpb_params_present_flag: bool,
    /// Bit rate scale.
    pub bit_rate_scale: u8,
    /// CPB size scale.
    pub cpb_size_scale: u8,
    /// Initial CPB removal delay length.
    pub initial_cpb_removal_delay_length_minus1: u8,
    /// AU CPB removal delay length.
    pub au_cpb_removal_delay_length_minus1: u8,
    /// DPB output delay length.
    pub dpb_output_delay_length_minus1: u8,
}

/// Profile, tier, and level for a sub-layer.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SubLayerProfileTierLevel {
    /// Sub-layer profile present.
    pub sub_layer_profile_present_flag: bool,
    /// Sub-layer level present.
    pub sub_layer_level_present_flag: bool,
    /// Sub-layer profile space.
    pub sub_layer_profile_space: u8,
    /// Sub-layer tier flag.
    pub sub_layer_tier_flag: bool,
    /// Sub-layer profile IDC.
    pub sub_layer_profile_idc: u8,
    /// Sub-layer level IDC.
    pub sub_layer_level_idc: u8,
}

/// HEVC Video Parameter Set.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vps {
    /// VPS ID (0-15).
    pub vps_video_parameter_set_id: u8,
    /// Base layer internal flag.
    pub vps_base_layer_internal_flag: bool,
    /// Base layer available flag.
    pub vps_base_layer_available_flag: bool,
    /// Maximum layers minus 1 (0-62).
    pub vps_max_layers_minus1: u8,
    /// Maximum sub-layers minus 1 (0-6).
    pub vps_max_sub_layers_minus1: u8,
    /// Temporal ID nesting flag.
    pub vps_temporal_id_nesting_flag: bool,
    /// Profile, tier, level information.
    pub profile_tier_level: VpsProfileTierLevel,
    /// Sub-layer ordering info present.
    pub vps_sub_layer_ordering_info_present_flag: bool,
    /// Maximum decoder buffer size for each sub-layer.
    pub vps_max_dec_pic_buffering_minus1: Vec<u32>,
    /// Maximum number of pictures reordered for each sub-layer.
    pub vps_max_num_reorder_pics: Vec<u32>,
    /// Maximum latency increase for each sub-layer.
    pub vps_max_latency_increase_plus1: Vec<u32>,
    /// Maximum layer ID.
    pub vps_max_layer_id: u8,
    /// Number of layer sets minus 1.
    pub vps_num_layer_sets_minus1: u16,
    /// Timing info present flag.
    pub vps_timing_info_present_flag: bool,
    /// Timing information.
    pub timing_info: Option<TimingInfo>,
    /// Number of HRD parameters.
    pub vps_num_hrd_parameters: u16,
}

/// Profile, tier, and level from VPS.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VpsProfileTierLevel {
    /// General profile space.
    pub general_profile_space: u8,
    /// General tier flag.
    pub general_tier_flag: bool,
    /// General profile IDC.
    pub general_profile_idc: u8,
    /// General profile compatibility flags (32 bits).
    pub general_profile_compatibility_flag: u32,
    /// General constraint indicator flags.
    pub general_progressive_source_flag: bool,
    pub general_interlaced_source_flag: bool,
    pub general_non_packed_constraint_flag: bool,
    pub general_frame_only_constraint_flag: bool,
    /// General level IDC.
    pub general_level_idc: u8,
    /// Sub-layer profile/tier/level.
    pub sub_layer_profile_tier_level: Vec<SubLayerProfileTierLevel>,
}

impl Default for Vps {
    fn default() -> Self {
        Self {
            vps_video_parameter_set_id: 0,
            vps_base_layer_internal_flag: true,
            vps_base_layer_available_flag: true,
            vps_max_layers_minus1: 0,
            vps_max_sub_layers_minus1: 0,
            vps_temporal_id_nesting_flag: true,
            profile_tier_level: VpsProfileTierLevel::default(),
            vps_sub_layer_ordering_info_present_flag: true,
            vps_max_dec_pic_buffering_minus1: vec![1],
            vps_max_num_reorder_pics: vec![0],
            vps_max_latency_increase_plus1: vec![0],
            vps_max_layer_id: 0,
            vps_num_layer_sets_minus1: 0,
            vps_timing_info_present_flag: false,
            timing_info: None,
            vps_num_hrd_parameters: 0,
        }
    }
}

impl Vps {
    /// Get maximum number of sub-layers.
    pub fn max_sub_layers(&self) -> u8 {
        self.vps_max_sub_layers_minus1 + 1
    }

    /// Get maximum number of layers.
    pub fn max_layers(&self) -> u8 {
        self.vps_max_layers_minus1 + 1
    }

    /// Get profile name.
    pub fn profile_name(&self) -> &'static str {
        match self.profile_tier_level.general_profile_idc {
            1 => "Main",
            2 => "Main 10",
            3 => "Main Still Picture",
            4 => "Range Extensions",
            5 => "High Throughput",
            9 => "Screen Content",
            11 => "High Throughput Screen Content",
            _ => "Unknown",
        }
    }

    /// Get level as a decimal value (e.g., 5.1).
    pub fn level(&self) -> f32 {
        self.profile_tier_level.general_level_idc as f32 / 30.0
    }

    /// Get tier name.
    pub fn tier_name(&self) -> &'static str {
        if self.profile_tier_level.general_tier_flag {
            "High"
        } else {
            "Main"
        }
    }
}

/// Parse VPS from RBSP data.
pub fn parse_vps(data: &[u8]) -> Result<Vps> {
    let mut reader = BitReader::new(data);
    let mut vps = Vps::default();

    // vps_video_parameter_set_id (4 bits)
    vps.vps_video_parameter_set_id = reader.read_bits(4)? as u8;

    // vps_base_layer_internal_flag (1 bit)
    vps.vps_base_layer_internal_flag = reader.read_bit()?;

    // vps_base_layer_available_flag (1 bit)
    vps.vps_base_layer_available_flag = reader.read_bit()?;

    // vps_max_layers_minus1 (6 bits)
    vps.vps_max_layers_minus1 = reader.read_bits(6)? as u8;

    // vps_max_sub_layers_minus1 (3 bits)
    vps.vps_max_sub_layers_minus1 = reader.read_bits(3)? as u8;

    // vps_temporal_id_nesting_flag (1 bit)
    vps.vps_temporal_id_nesting_flag = reader.read_bit()?;

    // vps_reserved_0xffff_16bits (16 bits, should be 0xFFFF)
    let reserved = reader.read_bits(16)?;
    if reserved != 0xFFFF {
        tracing::warn!("VPS reserved bits are not 0xFFFF: {:#06x}", reserved);
    }

    // profile_tier_level
    vps.profile_tier_level =
        parse_profile_tier_level(&mut reader, true, vps.vps_max_sub_layers_minus1)?;

    // vps_sub_layer_ordering_info_present_flag (1 bit)
    vps.vps_sub_layer_ordering_info_present_flag = reader.read_bit()?;

    // Parse sub-layer ordering info
    let start_idx = if vps.vps_sub_layer_ordering_info_present_flag {
        0
    } else {
        vps.vps_max_sub_layers_minus1 as usize
    };

    vps.vps_max_dec_pic_buffering_minus1.clear();
    vps.vps_max_num_reorder_pics.clear();
    vps.vps_max_latency_increase_plus1.clear();

    for _ in start_idx..=vps.vps_max_sub_layers_minus1 as usize {
        vps.vps_max_dec_pic_buffering_minus1.push(reader.read_ue()?);
        vps.vps_max_num_reorder_pics.push(reader.read_ue()?);
        vps.vps_max_latency_increase_plus1.push(reader.read_ue()?);
    }

    // vps_max_layer_id (6 bits)
    vps.vps_max_layer_id = reader.read_bits(6)? as u8;

    // vps_num_layer_sets_minus1 (ue(v))
    vps.vps_num_layer_sets_minus1 = reader.read_ue()? as u16;

    // Skip layer_id_included_flag for each layer set
    for i in 1..=vps.vps_num_layer_sets_minus1 as usize {
        for _ in 0..=vps.vps_max_layer_id as usize {
            let _ = reader.read_bit()?; // layer_id_included_flag[i][j]
        }
        let _ = i; // silence unused warning
    }

    // vps_timing_info_present_flag (1 bit)
    vps.vps_timing_info_present_flag = reader.read_bit()?;

    if vps.vps_timing_info_present_flag {
        let mut timing = TimingInfo::default();

        // vps_num_units_in_tick (32 bits)
        timing.num_units_in_tick = reader.read_bits(32)?;

        // vps_time_scale (32 bits)
        timing.time_scale = reader.read_bits(32)?;

        // vps_poc_proportional_to_timing_flag (1 bit)
        timing.poc_proportional_to_timing_flag = reader.read_bit()?;

        if timing.poc_proportional_to_timing_flag {
            timing.num_ticks_poc_diff_one_minus1 = reader.read_ue()?;
        }

        // vps_num_hrd_parameters (ue(v))
        vps.vps_num_hrd_parameters = reader.read_ue()? as u16;

        // Skip HRD parameters for now
        // Full implementation would parse hrd_layer_set_idx and hrd_parameters

        vps.timing_info = Some(timing);
    }

    Ok(vps)
}

/// Parse profile_tier_level structure.
fn parse_profile_tier_level(
    reader: &mut BitReader,
    profile_present_flag: bool,
    max_sub_layers_minus1: u8,
) -> Result<VpsProfileTierLevel> {
    let mut ptl = VpsProfileTierLevel::default();

    if profile_present_flag {
        // general_profile_space (2 bits)
        ptl.general_profile_space = reader.read_bits(2)? as u8;

        // general_tier_flag (1 bit)
        ptl.general_tier_flag = reader.read_bit()?;

        // general_profile_idc (5 bits)
        ptl.general_profile_idc = reader.read_bits(5)? as u8;

        // general_profile_compatibility_flag[32] (32 bits)
        ptl.general_profile_compatibility_flag = reader.read_bits(32)?;

        // general_progressive_source_flag (1 bit)
        ptl.general_progressive_source_flag = reader.read_bit()?;

        // general_interlaced_source_flag (1 bit)
        ptl.general_interlaced_source_flag = reader.read_bit()?;

        // general_non_packed_constraint_flag (1 bit)
        ptl.general_non_packed_constraint_flag = reader.read_bit()?;

        // general_frame_only_constraint_flag (1 bit)
        ptl.general_frame_only_constraint_flag = reader.read_bit()?;

        // Skip remaining 44 constraint flags (profile-specific)
        reader.skip_bits(44)?;
    }

    // general_level_idc (8 bits)
    ptl.general_level_idc = reader.read_bits(8)? as u8;

    // Parse sub-layer flags
    let mut sub_layer_profile_present_flag = vec![false; max_sub_layers_minus1 as usize];
    let mut sub_layer_level_present_flag = vec![false; max_sub_layers_minus1 as usize];

    for i in 0..max_sub_layers_minus1 as usize {
        sub_layer_profile_present_flag[i] = reader.read_bit()?;
        sub_layer_level_present_flag[i] = reader.read_bit()?;
    }

    // Byte alignment for reserved bits
    if max_sub_layers_minus1 > 0 {
        for _ in max_sub_layers_minus1..8 {
            let _ = reader.read_bits(2)?; // reserved_zero_2bits
        }
    }

    // Parse sub-layer profile_tier_level
    for i in 0..max_sub_layers_minus1 as usize {
        let mut sub = SubLayerProfileTierLevel::default();
        sub.sub_layer_profile_present_flag = sub_layer_profile_present_flag[i];
        sub.sub_layer_level_present_flag = sub_layer_level_present_flag[i];

        if sub.sub_layer_profile_present_flag {
            sub.sub_layer_profile_space = reader.read_bits(2)? as u8;
            sub.sub_layer_tier_flag = reader.read_bit()?;
            sub.sub_layer_profile_idc = reader.read_bits(5)? as u8;

            // Skip profile compatibility and constraint flags
            reader.skip_bits(32)?; // profile_compatibility_flag
            reader.skip_bits(48)?; // constraint flags
        }

        if sub.sub_layer_level_present_flag {
            sub.sub_layer_level_idc = reader.read_bits(8)? as u8;
        }

        ptl.sub_layer_profile_tier_level.push(sub);
    }

    Ok(ptl)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vps_defaults() {
        let vps = Vps::default();
        assert_eq!(vps.max_sub_layers(), 1);
        assert_eq!(vps.max_layers(), 1);
        assert_eq!(vps.profile_name(), "Unknown");
    }

    #[test]
    fn test_profile_names() {
        let mut vps = Vps::default();

        vps.profile_tier_level.general_profile_idc = 1;
        assert_eq!(vps.profile_name(), "Main");

        vps.profile_tier_level.general_profile_idc = 2;
        assert_eq!(vps.profile_name(), "Main 10");
    }
}
