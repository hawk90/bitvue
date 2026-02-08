//! H.264/AVC Sequence Parameter Set (SPS) parsing.

use crate::bitreader::BitReader;
use crate::error::{AvcError, Result};
use serde::{Deserialize, Serialize};
use std::fmt;

/// H.264/AVC Profile IDC values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum ProfileIdc {
    /// Baseline Profile
    Baseline = 66,
    /// Main Profile
    Main = 77,
    /// Extended Profile
    Extended = 88,
    /// High Profile
    High = 100,
    /// High 10 Profile
    High10 = 110,
    /// High 4:2:2 Profile
    High422 = 122,
    /// High 4:4:4 Predictive Profile
    High444 = 244,
    /// CAVLC 4:4:4 Intra Profile
    Cavlc444 = 44,
    /// Scalable Baseline Profile
    ScalableBaseline = 83,
    /// Scalable High Profile
    ScalableHigh = 86,
    /// Multiview High Profile
    MultiviewHigh = 118,
    /// Stereo High Profile
    StereoHigh = 128,
    /// Unknown profile
    Unknown = 0,
}

impl ProfileIdc {
    /// Create from raw value.
    pub fn from_u8(value: u8) -> Self {
        match value {
            66 => ProfileIdc::Baseline,
            77 => ProfileIdc::Main,
            88 => ProfileIdc::Extended,
            100 => ProfileIdc::High,
            110 => ProfileIdc::High10,
            122 => ProfileIdc::High422,
            244 => ProfileIdc::High444,
            44 => ProfileIdc::Cavlc444,
            83 => ProfileIdc::ScalableBaseline,
            86 => ProfileIdc::ScalableHigh,
            118 => ProfileIdc::MultiviewHigh,
            128 => ProfileIdc::StereoHigh,
            _ => ProfileIdc::Unknown,
        }
    }

    /// Check if this is a high profile (100, 110, 122, 244, 44, etc.)
    pub fn is_high_profile(&self) -> bool {
        matches!(
            self,
            ProfileIdc::High
                | ProfileIdc::High10
                | ProfileIdc::High422
                | ProfileIdc::High444
                | ProfileIdc::Cavlc444
                | ProfileIdc::ScalableHigh
                | ProfileIdc::MultiviewHigh
                | ProfileIdc::StereoHigh
        )
    }
}

impl fmt::Display for ProfileIdc {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProfileIdc::Baseline => write!(f, "Baseline"),
            ProfileIdc::Main => write!(f, "Main"),
            ProfileIdc::Extended => write!(f, "Extended"),
            ProfileIdc::High => write!(f, "High"),
            ProfileIdc::High10 => write!(f, "High 10"),
            ProfileIdc::High422 => write!(f, "High 4:2:2"),
            ProfileIdc::High444 => write!(f, "High 4:4:4"),
            ProfileIdc::Cavlc444 => write!(f, "CAVLC 4:4:4"),
            ProfileIdc::ScalableBaseline => write!(f, "Scalable Baseline"),
            ProfileIdc::ScalableHigh => write!(f, "Scalable High"),
            ProfileIdc::MultiviewHigh => write!(f, "Multiview High"),
            ProfileIdc::StereoHigh => write!(f, "Stereo High"),
            ProfileIdc::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Chroma format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ChromaFormat {
    /// Monochrome (4:0:0)
    Monochrome = 0,
    /// 4:2:0
    #[default]
    Yuv420 = 1,
    /// 4:2:2
    Yuv422 = 2,
    /// 4:4:4
    Yuv444 = 3,
}

impl ChromaFormat {
    /// Create from raw value.
    ///
    /// Note: This function uses Yuv420 as a fallback for invalid values.
    /// The caller should validate the input value is in range 0-3 before calling.
    /// For SPS parsing, validation is done at the call site.
    pub fn from_u8(value: u8) -> Self {
        match value {
            0 => ChromaFormat::Monochrome,
            1 => ChromaFormat::Yuv420,
            2 => ChromaFormat::Yuv422,
            3 => ChromaFormat::Yuv444,
            // SAFETY: Default to Yuv420 for invalid values.
            // This should never be reached if validation is done at call site.
            // Using a default prevents undefined behavior from invalid enum values.
            _ => ChromaFormat::Yuv420,
        }
    }

    /// Get subsampling width factor.
    pub fn sub_width_c(&self) -> u32 {
        match self {
            ChromaFormat::Monochrome => 0,
            ChromaFormat::Yuv420 | ChromaFormat::Yuv422 => 2,
            ChromaFormat::Yuv444 => 1,
        }
    }

    /// Get subsampling height factor.
    pub fn sub_height_c(&self) -> u32 {
        match self {
            ChromaFormat::Monochrome => 0,
            ChromaFormat::Yuv420 => 2,
            ChromaFormat::Yuv422 | ChromaFormat::Yuv444 => 1,
        }
    }
}

/// VUI (Video Usability Information) parameters.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VuiParameters {
    /// aspect_ratio_info_present_flag
    pub aspect_ratio_info_present_flag: bool,
    /// aspect_ratio_idc
    pub aspect_ratio_idc: u8,
    /// sar_width (if aspect_ratio_idc == 255)
    pub sar_width: u16,
    /// sar_height (if aspect_ratio_idc == 255)
    pub sar_height: u16,
    /// overscan_info_present_flag
    pub overscan_info_present_flag: bool,
    /// overscan_appropriate_flag
    pub overscan_appropriate_flag: bool,
    /// video_signal_type_present_flag
    pub video_signal_type_present_flag: bool,
    /// video_format
    pub video_format: u8,
    /// video_full_range_flag
    pub video_full_range_flag: bool,
    /// colour_description_present_flag
    pub colour_description_present_flag: bool,
    /// colour_primaries
    pub colour_primaries: u8,
    /// transfer_characteristics
    pub transfer_characteristics: u8,
    /// matrix_coefficients
    pub matrix_coefficients: u8,
    /// chroma_loc_info_present_flag
    pub chroma_loc_info_present_flag: bool,
    /// chroma_sample_loc_type_top_field
    pub chroma_sample_loc_type_top_field: u32,
    /// chroma_sample_loc_type_bottom_field
    pub chroma_sample_loc_type_bottom_field: u32,
    /// timing_info_present_flag
    pub timing_info_present_flag: bool,
    /// num_units_in_tick
    pub num_units_in_tick: u32,
    /// time_scale
    pub time_scale: u32,
    /// fixed_frame_rate_flag
    pub fixed_frame_rate_flag: bool,
    /// nal_hrd_parameters_present_flag
    pub nal_hrd_parameters_present_flag: bool,
    /// vcl_hrd_parameters_present_flag
    pub vcl_hrd_parameters_present_flag: bool,
    /// pic_struct_present_flag
    pub pic_struct_present_flag: bool,
    /// bitstream_restriction_flag
    pub bitstream_restriction_flag: bool,
    /// max_num_reorder_frames
    pub max_num_reorder_frames: u32,
    /// max_dec_frame_buffering
    pub max_dec_frame_buffering: u32,
}

/// Sequence Parameter Set.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sps {
    /// profile_idc
    pub profile_idc: ProfileIdc,
    /// constraint_set0_flag
    pub constraint_set0_flag: bool,
    /// constraint_set1_flag
    pub constraint_set1_flag: bool,
    /// constraint_set2_flag
    pub constraint_set2_flag: bool,
    /// constraint_set3_flag
    pub constraint_set3_flag: bool,
    /// constraint_set4_flag
    pub constraint_set4_flag: bool,
    /// constraint_set5_flag
    pub constraint_set5_flag: bool,
    /// level_idc
    pub level_idc: u8,
    /// seq_parameter_set_id
    pub seq_parameter_set_id: u8,
    /// chroma_format_idc
    pub chroma_format_idc: ChromaFormat,
    /// separate_colour_plane_flag
    pub separate_colour_plane_flag: bool,
    /// bit_depth_luma_minus8
    pub bit_depth_luma_minus8: u8,
    /// bit_depth_chroma_minus8
    pub bit_depth_chroma_minus8: u8,
    /// qpprime_y_zero_transform_bypass_flag
    pub qpprime_y_zero_transform_bypass_flag: bool,
    /// seq_scaling_matrix_present_flag
    pub seq_scaling_matrix_present_flag: bool,
    /// log2_max_frame_num_minus4
    pub log2_max_frame_num_minus4: u8,
    /// pic_order_cnt_type
    pub pic_order_cnt_type: u8,
    /// log2_max_pic_order_cnt_lsb_minus4
    pub log2_max_pic_order_cnt_lsb_minus4: u8,
    /// delta_pic_order_always_zero_flag
    pub delta_pic_order_always_zero_flag: bool,
    /// offset_for_non_ref_pic
    pub offset_for_non_ref_pic: i32,
    /// offset_for_top_to_bottom_field
    pub offset_for_top_to_bottom_field: i32,
    /// num_ref_frames_in_pic_order_cnt_cycle
    pub num_ref_frames_in_pic_order_cnt_cycle: u8,
    /// offset_for_ref_frame
    pub offset_for_ref_frame: Vec<i32>,
    /// max_num_ref_frames
    pub max_num_ref_frames: u32,
    /// gaps_in_frame_num_value_allowed_flag
    pub gaps_in_frame_num_value_allowed_flag: bool,
    /// pic_width_in_mbs_minus1
    pub pic_width_in_mbs_minus1: u32,
    /// pic_height_in_map_units_minus1
    pub pic_height_in_map_units_minus1: u32,
    /// frame_mbs_only_flag
    pub frame_mbs_only_flag: bool,
    /// mb_adaptive_frame_field_flag
    pub mb_adaptive_frame_field_flag: bool,
    /// direct_8x8_inference_flag
    pub direct_8x8_inference_flag: bool,
    /// frame_cropping_flag
    pub frame_cropping_flag: bool,
    /// frame_crop_left_offset
    pub frame_crop_left_offset: u32,
    /// frame_crop_right_offset
    pub frame_crop_right_offset: u32,
    /// frame_crop_top_offset
    pub frame_crop_top_offset: u32,
    /// frame_crop_bottom_offset
    pub frame_crop_bottom_offset: u32,
    /// vui_parameters_present_flag
    pub vui_parameters_present_flag: bool,
    /// VUI parameters
    pub vui_parameters: Option<VuiParameters>,
}

impl Sps {
    /// Get picture width in luma samples.
    pub fn pic_width(&self) -> u32 {
        (self.pic_width_in_mbs_minus1 + 1) * 16
    }

    /// Get picture height in luma samples.
    pub fn pic_height(&self) -> u32 {
        let frame_height_in_mbs =
            (2 - self.frame_mbs_only_flag as u32) * (self.pic_height_in_map_units_minus1 + 1);
        frame_height_in_mbs * 16
    }

    /// Get display width after cropping.
    pub fn display_width(&self) -> u32 {
        let width = self.pic_width();
        if self.frame_cropping_flag {
            let crop_unit_x = if self.chroma_format_idc == ChromaFormat::Monochrome {
                1
            } else {
                self.chroma_format_idc.sub_width_c()
            };
            width - crop_unit_x * (self.frame_crop_left_offset + self.frame_crop_right_offset)
        } else {
            width
        }
    }

    /// Get display height after cropping.
    pub fn display_height(&self) -> u32 {
        let height = self.pic_height();
        if self.frame_cropping_flag {
            let crop_unit_y = if self.chroma_format_idc == ChromaFormat::Monochrome {
                1
            } else {
                self.chroma_format_idc.sub_height_c()
            } * (2 - self.frame_mbs_only_flag as u32);
            height - crop_unit_y * (self.frame_crop_top_offset + self.frame_crop_bottom_offset)
        } else {
            height
        }
    }

    /// Get bit depth for luma.
    pub fn bit_depth_luma(&self) -> u8 {
        self.bit_depth_luma_minus8 + 8
    }

    /// Get bit depth for chroma.
    pub fn bit_depth_chroma(&self) -> u8 {
        self.bit_depth_chroma_minus8 + 8
    }
}

/// Parse SPS from NAL unit payload.
pub fn parse_sps(data: &[u8]) -> Result<Sps> {
    let mut reader = BitReader::new(data);

    let profile_idc = ProfileIdc::from_u8(reader.read_bits(8)? as u8);
    let constraint_set0_flag = reader.read_flag()?;
    let constraint_set1_flag = reader.read_flag()?;
    let constraint_set2_flag = reader.read_flag()?;
    let constraint_set3_flag = reader.read_flag()?;
    let constraint_set4_flag = reader.read_flag()?;
    let constraint_set5_flag = reader.read_flag()?;
    let _reserved_zero_2bits = reader.read_bits(2)?;
    let level_idc = reader.read_bits(8)? as u8;
    let seq_parameter_set_id = reader.read_ue()? as u8;

    // High profile specific
    let mut chroma_format_idc = ChromaFormat::Yuv420;
    let mut separate_colour_plane_flag = false;
    let mut bit_depth_luma_minus8 = 0u8;
    let mut bit_depth_chroma_minus8 = 0u8;
    let mut qpprime_y_zero_transform_bypass_flag = false;
    let mut seq_scaling_matrix_present_flag = false;

    if profile_idc.is_high_profile() || profile_idc == ProfileIdc::ScalableBaseline {
        // SECURITY: Validate chroma format ID to prevent invalid enum value
        let raw_chroma_format = reader.read_ue()?;
        if raw_chroma_format > 3 {
            return Err(AvcError::InvalidSps(format!(
                "chroma_format_idc {} exceeds maximum 3",
                raw_chroma_format
            )));
        }
        chroma_format_idc = ChromaFormat::from_u8(raw_chroma_format as u8);

        if chroma_format_idc == ChromaFormat::Yuv444 {
            separate_colour_plane_flag = reader.read_flag()?;
        }

        // SECURITY: Validate bit depth to prevent unreasonable values
        const MAX_BIT_DEPTH_MINUS8: u32 = 6; // Max 14-bit (8+6)
        let raw_bit_depth_luma = reader.read_ue()?;
        if raw_bit_depth_luma > MAX_BIT_DEPTH_MINUS8 {
            return Err(AvcError::InvalidSliceHeader(format!(
                "bit_depth_luma_minus8 {} exceeds maximum {}",
                raw_bit_depth_luma, MAX_BIT_DEPTH_MINUS8
            )));
        }
        bit_depth_luma_minus8 = raw_bit_depth_luma as u8;

        let raw_bit_depth_chroma = reader.read_ue()?;
        if raw_bit_depth_chroma > MAX_BIT_DEPTH_MINUS8 {
            return Err(AvcError::InvalidSliceHeader(format!(
                "bit_depth_chroma_minus8 {} exceeds maximum {}",
                raw_bit_depth_chroma, MAX_BIT_DEPTH_MINUS8
            )));
        }
        bit_depth_chroma_minus8 = raw_bit_depth_chroma as u8;

        qpprime_y_zero_transform_bypass_flag = reader.read_flag()?;
        seq_scaling_matrix_present_flag = reader.read_flag()?;

        if seq_scaling_matrix_present_flag {
            let num_scaling_lists = if chroma_format_idc != ChromaFormat::Yuv444 {
                8
            } else {
                12
            };
            for _ in 0..num_scaling_lists {
                let scaling_list_present_flag = reader.read_flag()?;
                if scaling_list_present_flag {
                    // Skip scaling list
                    skip_scaling_list(&mut reader, if num_scaling_lists <= 6 { 16 } else { 64 })?;
                }
            }
        }
    }

    let log2_max_frame_num_minus4 = reader.read_ue()? as u8;
    let pic_order_cnt_type = reader.read_ue()? as u8;

    let mut log2_max_pic_order_cnt_lsb_minus4 = 0u8;
    let mut delta_pic_order_always_zero_flag = false;
    let mut offset_for_non_ref_pic = 0i32;
    let mut offset_for_top_to_bottom_field = 0i32;
    let mut num_ref_frames_in_pic_order_cnt_cycle = 0u8;
    let mut offset_for_ref_frame = Vec::new();

    match pic_order_cnt_type {
        0 => {
            log2_max_pic_order_cnt_lsb_minus4 = reader.read_ue()? as u8;
        }
        1 => {
            delta_pic_order_always_zero_flag = reader.read_flag()?;
            offset_for_non_ref_pic = reader.read_se()?;
            offset_for_top_to_bottom_field = reader.read_se()?;

            // SECURITY: Validate ref frame cycle count to prevent unbounded loop
            const MAX_REF_FRAMES_IN_CYCLE: u32 = 255;
            let raw_ref_cycle_count = reader.read_ue()?;
            if raw_ref_cycle_count > MAX_REF_FRAMES_IN_CYCLE {
                return Err(AvcError::InvalidSliceHeader(format!(
                    "num_ref_frames_in_pic_order_cnt_cycle {} exceeds maximum {}",
                    raw_ref_cycle_count, MAX_REF_FRAMES_IN_CYCLE
                )));
            }
            num_ref_frames_in_pic_order_cnt_cycle = raw_ref_cycle_count as u8;

            for _ in 0..num_ref_frames_in_pic_order_cnt_cycle {
                offset_for_ref_frame.push(reader.read_se()?);
            }
        }
        _ => {}
    }

    // SECURITY: Validate max_num_ref_frames to prevent excessive allocation
    const MAX_NUM_REF_FRAMES: u32 = 32;
    let max_num_ref_frames = reader.read_ue()?;
    if max_num_ref_frames > MAX_NUM_REF_FRAMES {
        return Err(AvcError::InvalidSliceHeader(format!(
            "max_num_ref_frames {} exceeds maximum {}",
            max_num_ref_frames, MAX_NUM_REF_FRAMES
        )));
    }

    let gaps_in_frame_num_value_allowed_flag = reader.read_flag()?;

    // SECURITY: Validate picture dimensions to prevent excessive allocation
    const MAX_PIC_DIMENSION_IN_MBS: u32 = 16384; // Max 16K in macroblocks
    let pic_width_in_mbs_minus1 = reader.read_ue()?;
    if pic_width_in_mbs_minus1 >= MAX_PIC_DIMENSION_IN_MBS {
        return Err(AvcError::InvalidSliceHeader(format!(
            "pic_width_in_mbs_minus1 {} exceeds maximum {}",
            pic_width_in_mbs_minus1, MAX_PIC_DIMENSION_IN_MBS
        )));
    }
    let pic_height_in_map_units_minus1 = reader.read_ue()?;
    if pic_height_in_map_units_minus1 >= MAX_PIC_DIMENSION_IN_MBS {
        return Err(AvcError::InvalidSliceHeader(format!(
            "pic_height_in_map_units_minus1 {} exceeds maximum {}",
            pic_height_in_map_units_minus1, MAX_PIC_DIMENSION_IN_MBS
        )));
    }

    let frame_mbs_only_flag = reader.read_flag()?;

    let mut mb_adaptive_frame_field_flag = false;
    if !frame_mbs_only_flag {
        mb_adaptive_frame_field_flag = reader.read_flag()?;
    }

    let direct_8x8_inference_flag = reader.read_flag()?;
    let frame_cropping_flag = reader.read_flag()?;

    let mut frame_crop_left_offset = 0;
    let mut frame_crop_right_offset = 0;
    let mut frame_crop_top_offset = 0;
    let mut frame_crop_bottom_offset = 0;

    if frame_cropping_flag {
        frame_crop_left_offset = reader.read_ue()?;
        frame_crop_right_offset = reader.read_ue()?;
        frame_crop_top_offset = reader.read_ue()?;
        frame_crop_bottom_offset = reader.read_ue()?;
    }

    let vui_parameters_present_flag = reader.read_flag()?;
    let vui_parameters = if vui_parameters_present_flag {
        Some(parse_vui(&mut reader)?)
    } else {
        None
    };

    Ok(Sps {
        profile_idc,
        constraint_set0_flag,
        constraint_set1_flag,
        constraint_set2_flag,
        constraint_set3_flag,
        constraint_set4_flag,
        constraint_set5_flag,
        level_idc,
        seq_parameter_set_id,
        chroma_format_idc,
        separate_colour_plane_flag,
        bit_depth_luma_minus8,
        bit_depth_chroma_minus8,
        qpprime_y_zero_transform_bypass_flag,
        seq_scaling_matrix_present_flag,
        log2_max_frame_num_minus4,
        pic_order_cnt_type,
        log2_max_pic_order_cnt_lsb_minus4,
        delta_pic_order_always_zero_flag,
        offset_for_non_ref_pic,
        offset_for_top_to_bottom_field,
        num_ref_frames_in_pic_order_cnt_cycle,
        offset_for_ref_frame,
        max_num_ref_frames,
        gaps_in_frame_num_value_allowed_flag,
        pic_width_in_mbs_minus1,
        pic_height_in_map_units_minus1,
        frame_mbs_only_flag,
        mb_adaptive_frame_field_flag,
        direct_8x8_inference_flag,
        frame_cropping_flag,
        frame_crop_left_offset,
        frame_crop_right_offset,
        frame_crop_top_offset,
        frame_crop_bottom_offset,
        vui_parameters_present_flag,
        vui_parameters,
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

/// Parse VUI parameters.
fn parse_vui(reader: &mut BitReader) -> Result<VuiParameters> {
    let mut vui = VuiParameters::default();

    vui.aspect_ratio_info_present_flag = reader.read_flag()?;
    if vui.aspect_ratio_info_present_flag {
        vui.aspect_ratio_idc = reader.read_bits(8)? as u8;
        if vui.aspect_ratio_idc == 255 {
            // Extended_SAR
            vui.sar_width = reader.read_bits(16)? as u16;
            vui.sar_height = reader.read_bits(16)? as u16;
        }
    }

    vui.overscan_info_present_flag = reader.read_flag()?;
    if vui.overscan_info_present_flag {
        vui.overscan_appropriate_flag = reader.read_flag()?;
    }

    vui.video_signal_type_present_flag = reader.read_flag()?;
    if vui.video_signal_type_present_flag {
        vui.video_format = reader.read_bits(3)? as u8;
        vui.video_full_range_flag = reader.read_flag()?;
        vui.colour_description_present_flag = reader.read_flag()?;
        if vui.colour_description_present_flag {
            vui.colour_primaries = reader.read_bits(8)? as u8;
            vui.transfer_characteristics = reader.read_bits(8)? as u8;
            vui.matrix_coefficients = reader.read_bits(8)? as u8;
        }
    }

    vui.chroma_loc_info_present_flag = reader.read_flag()?;
    if vui.chroma_loc_info_present_flag {
        // SECURITY: Validate chroma location types to prevent unreasonable values
        const MAX_CHROMA_LOC_TYPE: u32 = 64;
        vui.chroma_sample_loc_type_top_field = reader.read_ue()?;
        if vui.chroma_sample_loc_type_top_field > MAX_CHROMA_LOC_TYPE {
            return Err(AvcError::InvalidSliceHeader(format!(
                "chroma_sample_loc_type_top_field {} exceeds maximum {}",
                vui.chroma_sample_loc_type_top_field, MAX_CHROMA_LOC_TYPE
            )));
        }
        vui.chroma_sample_loc_type_bottom_field = reader.read_ue()?;
        if vui.chroma_sample_loc_type_bottom_field > MAX_CHROMA_LOC_TYPE {
            return Err(AvcError::InvalidSliceHeader(format!(
                "chroma_sample_loc_type_bottom_field {} exceeds maximum {}",
                vui.chroma_sample_loc_type_bottom_field, MAX_CHROMA_LOC_TYPE
            )));
        }
    }

    vui.timing_info_present_flag = reader.read_flag()?;
    if vui.timing_info_present_flag {
        vui.num_units_in_tick = reader.read_bits(32)?;
        vui.time_scale = reader.read_bits(32)?;
        vui.fixed_frame_rate_flag = reader.read_flag()?;
    }

    vui.nal_hrd_parameters_present_flag = reader.read_flag()?;
    if vui.nal_hrd_parameters_present_flag {
        skip_hrd_parameters(reader)?;
    }

    vui.vcl_hrd_parameters_present_flag = reader.read_flag()?;
    if vui.vcl_hrd_parameters_present_flag {
        skip_hrd_parameters(reader)?;
    }

    if vui.nal_hrd_parameters_present_flag || vui.vcl_hrd_parameters_present_flag {
        let _low_delay_hrd_flag = reader.read_flag()?;
    }

    vui.pic_struct_present_flag = reader.read_flag()?;
    vui.bitstream_restriction_flag = reader.read_flag()?;

    if vui.bitstream_restriction_flag {
        let _motion_vectors_over_pic_boundaries_flag = reader.read_flag()?;
        let _max_bytes_per_pic_denom = reader.read_ue()?;
        let _max_bits_per_mb_denom = reader.read_ue()?;
        let _log2_max_mv_length_horizontal = reader.read_ue()?;
        let _log2_max_mv_length_vertical = reader.read_ue()?;

        // SECURITY: Validate frame buffer parameters to prevent excessive allocation
        const MAX_NUM_REORDER_FRAMES: u32 = 16;
        const MAX_DEC_FRAME_BUFFERING: u32 = 32;
        vui.max_num_reorder_frames = reader.read_ue()?;
        if vui.max_num_reorder_frames > MAX_NUM_REORDER_FRAMES {
            return Err(AvcError::InvalidSliceHeader(format!(
                "max_num_reorder_frames {} exceeds maximum {}",
                vui.max_num_reorder_frames, MAX_NUM_REORDER_FRAMES
            )));
        }
        vui.max_dec_frame_buffering = reader.read_ue()?;
        if vui.max_dec_frame_buffering > MAX_DEC_FRAME_BUFFERING {
            return Err(AvcError::InvalidSliceHeader(format!(
                "max_dec_frame_buffering {} exceeds maximum {}",
                vui.max_dec_frame_buffering, MAX_DEC_FRAME_BUFFERING
            )));
        }
    }

    Ok(vui)
}

/// Skip HRD parameters.
fn skip_hrd_parameters(reader: &mut BitReader) -> Result<()> {
    // SECURITY: Validate cpb_cnt_minus1 to prevent unbounded loop
    const MAX_CPB_COUNT: u32 = 32;
    let cpb_cnt_minus1 = reader.read_ue()?;

    if cpb_cnt_minus1 > MAX_CPB_COUNT {
        return Err(AvcError::InvalidSliceHeader(format!(
            "cpb_cnt_minus1 {} exceeds maximum {}",
            cpb_cnt_minus1, MAX_CPB_COUNT
        )));
    }

    let _bit_rate_scale = reader.read_bits(4)?;
    let _cpb_size_scale = reader.read_bits(4)?;

    for _ in 0..=cpb_cnt_minus1 {
        let _bit_rate_value_minus1 = reader.read_ue()?;
        let _cpb_size_value_minus1 = reader.read_ue()?;
        let _cbr_flag = reader.read_flag()?;
    }

    let _initial_cpb_removal_delay_length_minus1 = reader.read_bits(5)?;
    let _cpb_removal_delay_length_minus1 = reader.read_bits(5)?;
    let _dpb_output_delay_length_minus1 = reader.read_bits(5)?;
    let _time_offset_length = reader.read_bits(5)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profile_idc() {
        assert_eq!(ProfileIdc::from_u8(66), ProfileIdc::Baseline);
        assert_eq!(ProfileIdc::from_u8(77), ProfileIdc::Main);
        assert_eq!(ProfileIdc::from_u8(100), ProfileIdc::High);
        assert!(ProfileIdc::High.is_high_profile());
        assert!(!ProfileIdc::Baseline.is_high_profile());
    }

    #[test]
    fn test_chroma_format() {
        assert_eq!(ChromaFormat::from_u8(0), ChromaFormat::Monochrome);
        assert_eq!(ChromaFormat::from_u8(1), ChromaFormat::Yuv420);
        assert_eq!(ChromaFormat::Yuv420.sub_width_c(), 2);
        assert_eq!(ChromaFormat::Yuv420.sub_height_c(), 2);
    }
}
