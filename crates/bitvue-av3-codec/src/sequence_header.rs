//! Sequence Header parsing for AV3.

use crate::bitreader::BitReader;
use crate::error::Result;
use serde::{Deserialize, Serialize};

/// Sequence Header for AV3.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SequenceHeader {
    /// AV3 profile (0-3)
    pub seq_profile: u8,
    /// AV3 level (0-31)
    pub seq_level_idx: u8,
    /// Sequence tier (0=main, 1=high)
    pub seq_tier: u8,
    /// Initial display delay
    pub initial_display_delay_minus_1: u8,
    /// Video bit depth (8, 10, 12)
    pub bit_depth: u8,
    /// Maximum frame width
    pub max_frame_width: u32,
    /// Maximum frame height
    pub max_frame_height: u32,
    /// Frame ID numbers are present
    pub frame_id_numbers_present_flag: bool,
    /// Frame ID length (minus 1)
    pub delta_frame_id_length_minus_2: u8,
    /// Additional frame ID length
    pub additional_frame_id_length_minus_1: u8,
    /// Use 128x128 super blocks
    pub use_128x128_superblock_flag: bool,
    /// Enable filter intra
    pub enable_filter_intra: bool,
    /// Enable intra edge filter
    pub enable_intra_edge_filter: bool,
    /// Enable interintra compound
    pub enable_interintra_compound: bool,
    /// Enable masked compound
    pub enable_masked_compound: bool,
    /// Enable dual filter
    pub enable_dual_filter: bool,
    /// Enable order hint
    pub enable_order_hint: bool,
    /// Order hint bits (minus 1)
    pub order_hint_bits_minus_1: u8,
    /// Enable jnt comp (joint compound modes)
    pub enable_jnt_comp: bool,
    /// Enable super resolution
    pub enable_superres: bool,
    /// Enable cdef (constrained directional enhancement filter)
    pub enable_cdef: bool,
    /// enable restoration loop filtering
    pub enable_restoration: bool,
    /// Enable post processing overlay
    pub enable_post_process_overlay: bool, // AV3 addition
    /// Enable large scale tile
    pub enable_large_scale_tile: bool,
    /// Timing info present
    pub timing_info_present_flag: bool,
    /// Time scale
    pub time_scale: u32,
    /// Number of units in display tick
    pub num_units_in_display_tick: u32,
    /// Buffer removal delay
    pub buffer_removal_delay: u32,
    /// Operating points
    pub operating_points: Vec<OperatingPoint>,
    /// Color config
    pub color_config: ColorConfig,
    /// Film grain params present
    pub film_grain_params_present: bool,
}

/// Operating point info for scalable streams.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperatingPoint {
    /// Operating point ID
    pub op_id: u8,
    /// Operating point codec level
    pub seq_level_idx: u8,
    /// Operating point tier
    pub seq_tier: u8,
    /// Decoder model present
    pub decoder_model_present_for_this_op: bool,
    /// Operating parameters info
    pub operating_parameters_info: Option<OperatingParametersInfo>,
}

/// Operating parameters info.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperatingParametersInfo {
    /// Buffer delay in decoder
    pub decoder_buffer_delay: u32,
    /// Encoder buffer delay
    pub encoder_buffer_delay: u32,
    /// Low delay mode flag
    pub low_delay_mode_flag: bool,
}

/// Color configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorConfig {
    /// Color space (0 = BT.601, 1 = BT.709, 2 = BT.2020, etc.)
    pub color_space: u8,
    /// Color range (0 = limited, 1 = full)
    pub color_range: u8,
    /// Subsampling x (0 = 444, 1 = 422, 2 = 420)
    pub subsampling_x: u8,
    /// Subsampling y
    pub subsampling_y: u8,
    /// Chroma sample position
    pub chroma_sample_position: u8,
    /// Separate UV delta Q
    pub separate_uv_delta_q: bool,
}

/// Read unsigned variable-length code (uvlc) per AV3/AV1 spec §4.10.3.
///
/// Reads leading zeros followed by a '1' stop bit, then that many
/// additional value bits. Returns the decoded value.
fn read_uvlc(reader: &mut BitReader<'_>) -> Result<u64> {
    let mut leading_bits: u8 = 0;
    loop {
        let done = reader.read_bit()?;
        if done {
            break;
        }
        leading_bits += 1;
        if leading_bits >= 32 {
            // All-ones sentinel: value = 2^32 - 1
            return Ok((1u64 << 32) - 1);
        }
    }
    if leading_bits == 0 {
        return Ok(0);
    }
    let value = reader.read_bits(leading_bits)?;
    Ok((1u64 << leading_bits) - 1 + value)
}

/// Parse sequence header from OBU payload.
pub fn parse_sequence_header(data: &[u8]) -> Result<SequenceHeader> {
    let mut reader = BitReader::new(data);

    let seq_profile = reader.read_bits(3)? as u8;
    let _still_picture = reader.read_bit()?;
    let reduced_still_picture_header = reader.read_bit()?;

    let mut timing_info_present = false;
    let mut parsed_num_units: u32 = 0;
    let mut parsed_time_scale: u32 = 0;

    if reduced_still_picture_header {
        reader.read_bits(5)?; // seq_level_idx
    } else {
        reader.read_bits(5)?; // seq_level_idx
        timing_info_present = reader.read_bit()?;
        if timing_info_present {
            // timing_info() per AV3/AV1 spec §5.5.3
            parsed_num_units = reader.read_bits(32)? as u32;
            parsed_time_scale = reader.read_bits(32)? as u32;
            let equal_picture_interval = reader.read_bit()?;
            if equal_picture_interval {
                // num_ticks_per_picture_minus_1: uvlc()
                read_uvlc(&mut reader)?;
            }
        }
    }

    let seq_tier = reader.read_bits(1)? as u8;
    let initial_display_delay_minus_1 = reader.read_bits(4)? as u8;
    let bit_depth = reader.read_bits(2)? as u8;
    let _seq_force_screen_content_tools = reader.read_bits(1)? as u8;

    // Parse frame size
    let max_frame_width_minus_1 = reader.read_bits(16)?;
    let max_frame_height_minus_1 = reader.read_bits(16)?;

    // Calculate dimensions
    let max_frame_width = max_frame_width_minus_1 as u32 + 1;
    let max_frame_height = max_frame_height_minus_1 as u32 + 1;

    // Validate dimensions to prevent DoS via excessive allocations
    // Maximum 8K resolution (7680x4320) with safety margin
    const MAX_WIDTH: u32 = 7680;
    const MAX_HEIGHT: u32 = 4320;

    if max_frame_width > MAX_WIDTH || max_frame_height > MAX_HEIGHT {
        return Err(crate::error::Av3Error::InvalidData(format!(
            "Sequence header dimensions {}x{} exceed maximum {}x{}",
            max_frame_width, max_frame_height, MAX_WIDTH, MAX_HEIGHT
        )));
    }

    // Also validate minimum dimensions
    if max_frame_width == 0 || max_frame_height == 0 {
        return Err(crate::error::Av3Error::InvalidData(format!(
            "Sequence header dimensions {}x{} are invalid (must be non-zero)",
            max_frame_width, max_frame_height
        )));
    }

    // Use defaults for remaining fields (simplified)
    Ok(SequenceHeader {
        seq_profile,
        seq_level_idx: 0,
        seq_tier,
        initial_display_delay_minus_1,
        bit_depth: match bit_depth {
            0 => 8,
            1 => 10,
            _ => 12,
        },
        max_frame_width,
        max_frame_height,
        frame_id_numbers_present_flag: false,
        delta_frame_id_length_minus_2: 0,
        additional_frame_id_length_minus_1: 0,
        use_128x128_superblock_flag: true,
        enable_filter_intra: false,
        enable_intra_edge_filter: false,
        enable_interintra_compound: false,
        enable_masked_compound: false,
        enable_dual_filter: false,
        enable_order_hint: true,
        order_hint_bits_minus_1: 0,
        enable_jnt_comp: false,
        enable_superres: false,
        enable_cdef: true,
        enable_restoration: true,
        enable_post_process_overlay: false, // AV3 addition
        enable_large_scale_tile: false,
        timing_info_present_flag: timing_info_present,
        time_scale: parsed_time_scale,
        num_units_in_display_tick: parsed_num_units,
        buffer_removal_delay: 0,
        operating_points: Vec::new(),
        color_config: ColorConfig {
            color_space: 1, // BT.709
            color_range: 0,
            subsampling_x: 1,
            subsampling_y: 1,
            chroma_sample_position: 0,
            separate_uv_delta_q: false,
        },
        film_grain_params_present: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty() {
        let data: &[u8] = &[];
        assert!(parse_sequence_header(data).is_err());
    }
}
