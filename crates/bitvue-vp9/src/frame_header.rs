//! VP9 Frame Header parsing.
//!
//! VP9 frame header consists of:
//! - Uncompressed header (parsed directly)
//! - Compressed header (arithmetic coded)
//!
//! This module parses the uncompressed header.

use crate::bitreader::BitReader;
use crate::error::{Result, Vp9Error};
// Re-export FrameType for other modules in this crate
pub use bitvue_core::FrameType;
use serde::{Deserialize, Serialize};

/// VP9 color space.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ColorSpace {
    Unknown = 0,
    Bt601 = 1,
    Bt709 = 2,
    Smpte170 = 3,
    Smpte240 = 4,
    Bt2020 = 5,
    Reserved = 6,
    Srgb = 7,
}

impl From<u8> for ColorSpace {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::Unknown,
            1 => Self::Bt601,
            2 => Self::Bt709,
            3 => Self::Smpte170,
            4 => Self::Smpte240,
            5 => Self::Bt2020,
            6 => Self::Reserved,
            7 => Self::Srgb,
            _ => Self::Unknown,
        }
    }
}

/// VP9 interpolation filter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InterpolationFilter {
    EightTap = 0,
    EightTapSmooth = 1,
    EightTapSharp = 2,
    Bilinear = 3,
    Switchable = 4,
}

impl From<u8> for InterpolationFilter {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::EightTap,
            1 => Self::EightTapSmooth,
            2 => Self::EightTapSharp,
            3 => Self::Bilinear,
            _ => Self::Switchable,
        }
    }
}

/// Reference frame types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RefFrame {
    None = 0,
    Last = 1,
    Golden = 2,
    AltRef = 3,
}

/// VP9 segmentation feature.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SegmentFeature {
    AltQ = 0,
    AltLf = 1,
    RefFrame = 2,
    Skip = 3,
}

/// Segmentation parameters.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Segmentation {
    /// Segmentation enabled.
    pub enabled: bool,
    /// Update segmentation map.
    pub update_map: bool,
    /// Segmentation map temporal update.
    pub temporal_update: bool,
    /// Update segmentation data.
    pub update_data: bool,
    /// Absolute or delta values.
    pub abs_or_delta_update: bool,
    /// Feature enabled for each segment (8 segments x 4 features).
    pub feature_enabled: [[bool; 4]; 8],
    /// Feature data for each segment (8 segments x 4 features).
    pub feature_data: [[i16; 4]; 8],
}

/// Loop filter parameters.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LoopFilter {
    /// Loop filter level.
    pub level: u8,
    /// Loop filter sharpness.
    pub sharpness: u8,
    /// Mode reference delta enabled.
    pub mode_ref_delta_enabled: bool,
    /// Mode reference delta update.
    pub mode_ref_delta_update: bool,
    /// Reference deltas (4 values: INTRA, LAST, GOLDEN, ALTREF).
    pub ref_deltas: [i8; 4],
    /// Mode deltas (2 values: 0, non-0).
    pub mode_deltas: [i8; 2],
}

/// Quantization parameters.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Quantization {
    /// Base Q index.
    pub base_q_idx: u8,
    /// Y DC delta Q.
    pub delta_q_y_dc: i8,
    /// UV DC delta Q.
    pub delta_q_uv_dc: i8,
    /// UV AC delta Q.
    pub delta_q_uv_ac: i8,
    /// Lossless mode.
    pub lossless: bool,
}

/// VP9 frame header.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameHeader {
    /// Frame type (key or inter).
    pub frame_type: FrameType,
    /// Show frame flag.
    pub show_frame: bool,
    /// Error resilient mode.
    pub error_resilient_mode: bool,
    /// Frame width.
    pub width: u32,
    /// Frame height.
    pub height: u32,
    /// Render width (for display).
    pub render_width: u32,
    /// Render height (for display).
    pub render_height: u32,
    /// Intra-only flag (for inter frames that don't reference other frames).
    pub intra_only: bool,
    /// Reset frame context.
    pub reset_frame_context: u8,
    /// Refresh frame flags (8 reference slots).
    pub refresh_frame_flags: u8,
    /// Reference frame indices for LAST, GOLDEN, ALTREF.
    pub ref_frame_idx: [u8; 3],
    /// Reference frame sign bias.
    pub ref_frame_sign_bias: [bool; 4],
    /// Allow high precision motion vectors.
    pub allow_high_precision_mv: bool,
    /// Interpolation filter.
    pub interpolation_filter: InterpolationFilter,
    /// Refresh frame context.
    pub refresh_frame_context: bool,
    /// Frame parallel decoding mode.
    pub frame_parallel_decoding_mode: bool,
    /// Frame context index.
    pub frame_context_idx: u8,
    /// Loop filter parameters.
    pub loop_filter: LoopFilter,
    /// Quantization parameters.
    pub quantization: Quantization,
    /// Segmentation parameters.
    pub segmentation: Segmentation,
    /// Tile columns log2.
    pub tile_cols_log2: u8,
    /// Tile rows log2.
    pub tile_rows_log2: u8,
    /// Header size in bytes.
    pub header_size_in_bytes: u16,
    /// Bit depth (8, 10, or 12).
    pub bit_depth: u8,
    /// Color space.
    pub color_space: ColorSpace,
    /// Subsampling X.
    pub subsampling_x: bool,
    /// Subsampling Y.
    pub subsampling_y: bool,
    /// Color range (0 = studio, 1 = full).
    pub color_range: bool,
}

impl Default for FrameHeader {
    fn default() -> Self {
        Self {
            frame_type: FrameType::Key,
            show_frame: true,
            error_resilient_mode: false,
            width: 0,
            height: 0,
            render_width: 0,
            render_height: 0,
            intra_only: false,
            reset_frame_context: 0,
            refresh_frame_flags: 0,
            ref_frame_idx: [0; 3],
            ref_frame_sign_bias: [false; 4],
            allow_high_precision_mv: false,
            interpolation_filter: InterpolationFilter::EightTap,
            refresh_frame_context: true,
            frame_parallel_decoding_mode: false,
            frame_context_idx: 0,
            loop_filter: LoopFilter::default(),
            quantization: Quantization::default(),
            segmentation: Segmentation::default(),
            tile_cols_log2: 0,
            tile_rows_log2: 0,
            header_size_in_bytes: 0,
            bit_depth: 8,
            color_space: ColorSpace::Unknown,
            subsampling_x: true,
            subsampling_y: true,
            color_range: false,
        }
    }
}

impl FrameHeader {
    /// Check if this is a key frame.
    pub fn is_key_frame(&self) -> bool {
        self.frame_type.is_key()
    }

    /// Check if this is an intra-only frame (key frame or intra-only inter frame).
    pub fn is_intra_only(&self) -> bool {
        self.frame_type.is_key() || self.intra_only
    }

    /// Get number of tile columns.
    pub fn num_tile_cols(&self) -> u32 {
        1 << self.tile_cols_log2
    }

    /// Get number of tile rows.
    pub fn num_tile_rows(&self) -> u32 {
        1 << self.tile_rows_log2
    }

    /// Get total number of tiles.
    pub fn num_tiles(&self) -> u32 {
        self.num_tile_cols() * self.num_tile_rows()
    }
}

/// Parse VP9 frame header from data.
pub fn parse_frame_header(data: &[u8]) -> Result<FrameHeader> {
    let mut reader = BitReader::new(data);
    let mut header = FrameHeader::default();

    // frame_marker (2 bits, must be 0b10)
    let frame_marker = reader.read_literal(2)?;
    if frame_marker != 2 {
        return Err(Vp9Error::InvalidData(format!(
            "Invalid frame marker: {}, expected 2",
            frame_marker
        )));
    }

    // profile_low_bit (1 bit)
    let profile_low = reader.read_literal(1)? as u8;

    // profile_high_bit (1 bit)
    let profile_high = reader.read_literal(1)? as u8;

    let profile = (profile_high << 1) | profile_low;

    // For profile 3, there's a reserved bit
    if profile == 3 {
        let _reserved = reader.read_literal(1)?;
    }

    // show_existing_frame (1 bit)
    let show_existing_frame = reader.read_literal(1)? != 0;

    if show_existing_frame {
        // frame_to_show_map_idx (3 bits)
        let _frame_to_show_map_idx = reader.read_literal(3)?;
        // This frame just shows an existing reference frame
        header.show_frame = true;
        return Ok(header);
    }

    // frame_type (1 bit)
    header.frame_type = if reader.read_literal(1)? == 0 {
        FrameType::Key
    } else {
        FrameType::Inter
    };

    // show_frame (1 bit)
    header.show_frame = reader.read_literal(1)? != 0;

    // error_resilient_mode (1 bit)
    header.error_resilient_mode = reader.read_literal(1)? != 0;

    if header.frame_type == FrameType::Key {
        // frame_sync_code (24 bits, must be 0x498342)
        let sync_code = reader.read_literal(24)?;
        if sync_code != 0x498342 {
            return Err(Vp9Error::InvalidData(format!(
                "Invalid sync code: {:#x}, expected 0x498342",
                sync_code
            )));
        }

        // Parse color config
        parse_color_config(&mut reader, &mut header, profile)?;

        // Parse frame size
        parse_frame_size(&mut reader, &mut header)?;

        // Parse render size
        parse_render_size(&mut reader, &mut header)?;

        // Key frames refresh all reference frames
        header.refresh_frame_flags = 0xFF;
    } else {
        // Inter frame
        if !header.show_frame {
            // intra_only (1 bit)
            header.intra_only = reader.read_literal(1)? != 0;
        }

        if !header.error_resilient_mode {
            // reset_frame_context (2 bits)
            header.reset_frame_context = reader.read_literal(2)? as u8;
        }

        if header.intra_only {
            // frame_sync_code (24 bits)
            let sync_code = reader.read_literal(24)?;
            if sync_code != 0x498342 {
                return Err(Vp9Error::InvalidData(format!(
                    "Invalid sync code: {:#x}",
                    sync_code
                )));
            }

            // Profile 0: only 8-bit
            if profile > 0 {
                parse_color_config(&mut reader, &mut header, profile)?;
            } else {
                header.color_space = ColorSpace::Bt601;
                header.subsampling_x = true;
                header.subsampling_y = true;
                header.bit_depth = 8;
            }

            // refresh_frame_flags (8 bits)
            header.refresh_frame_flags = reader.read_literal(8)? as u8;

            parse_frame_size(&mut reader, &mut header)?;
            parse_render_size(&mut reader, &mut header)?;
        } else {
            // refresh_frame_flags (8 bits)
            header.refresh_frame_flags = reader.read_literal(8)? as u8;

            // Reference frame indices
            for i in 0..3 {
                header.ref_frame_idx[i] = reader.read_literal(3)? as u8;
                header.ref_frame_sign_bias[i + 1] = reader.read_literal(1)? != 0;
            }

            // frame_size_with_refs
            let found_ref = reader.read_literal(1)? != 0;
            if !found_ref {
                let found_ref = reader.read_literal(1)? != 0;
                if !found_ref {
                    let found_ref = reader.read_literal(1)? != 0;
                    if !found_ref {
                        parse_frame_size(&mut reader, &mut header)?;
                    }
                }
            }

            parse_render_size(&mut reader, &mut header)?;

            // allow_high_precision_mv (1 bit)
            header.allow_high_precision_mv = reader.read_literal(1)? != 0;

            // interpolation_filter
            let is_filter_switchable = reader.read_literal(1)? != 0;
            if is_filter_switchable {
                header.interpolation_filter = InterpolationFilter::Switchable;
            } else {
                header.interpolation_filter =
                    InterpolationFilter::from(reader.read_literal(2)? as u8);
            }
        }
    }

    if !header.error_resilient_mode {
        // refresh_frame_context (1 bit)
        header.refresh_frame_context = reader.read_literal(1)? != 0;

        // frame_parallel_decoding_mode (1 bit)
        header.frame_parallel_decoding_mode = reader.read_literal(1)? != 0;
    } else {
        header.refresh_frame_context = false;
        header.frame_parallel_decoding_mode = true;
    }

    // frame_context_idx (2 bits)
    header.frame_context_idx = reader.read_literal(2)? as u8;

    // Parse loop filter
    parse_loop_filter(&mut reader, &mut header)?;

    // Parse quantization
    parse_quantization(&mut reader, &mut header)?;

    // Parse segmentation
    parse_segmentation(&mut reader, &mut header)?;

    // Parse tile info
    parse_tile_info(&mut reader, &mut header)?;

    // header_size_in_bytes (16 bits)
    header.header_size_in_bytes = reader.read_literal(16)? as u16;

    Ok(header)
}

fn parse_color_config(reader: &mut BitReader, header: &mut FrameHeader, profile: u8) -> Result<()> {
    if profile >= 2 {
        // ten_or_twelve_bit (1 bit)
        let ten_or_twelve_bit = reader.read_literal(1)? != 0;
        header.bit_depth = if ten_or_twelve_bit { 12 } else { 10 };
    } else {
        header.bit_depth = 8;
    }

    // color_space (3 bits)
    header.color_space = ColorSpace::from(reader.read_literal(3)? as u8);

    if header.color_space != ColorSpace::Srgb {
        // color_range (1 bit)
        header.color_range = reader.read_literal(1)? != 0;

        if profile == 1 || profile == 3 {
            // subsampling_x (1 bit)
            header.subsampling_x = reader.read_literal(1)? != 0;
            // subsampling_y (1 bit)
            header.subsampling_y = reader.read_literal(1)? != 0;

            // reserved (1 bit)
            let _reserved = reader.read_literal(1)?;
        } else {
            header.subsampling_x = true;
            header.subsampling_y = true;
        }
    } else {
        header.color_range = true;
        if profile == 1 || profile == 3 {
            header.subsampling_x = false;
            header.subsampling_y = false;
            let _reserved = reader.read_literal(1)?;
        }
    }

    Ok(())
}

fn parse_frame_size(reader: &mut BitReader, header: &mut FrameHeader) -> Result<()> {
    // frame_width_minus_1 (16 bits)
    header.width = reader.read_literal(16)? + 1;
    // frame_height_minus_1 (16 bits)
    header.height = reader.read_literal(16)? + 1;

    // Default render size to frame size
    header.render_width = header.width;
    header.render_height = header.height;

    Ok(())
}

fn parse_render_size(reader: &mut BitReader, header: &mut FrameHeader) -> Result<()> {
    // render_and_frame_size_different (1 bit)
    let different = reader.read_literal(1)? != 0;

    if different {
        header.render_width = reader.read_literal(16)? + 1;
        header.render_height = reader.read_literal(16)? + 1;
    }

    Ok(())
}

fn parse_loop_filter(reader: &mut BitReader, header: &mut FrameHeader) -> Result<()> {
    // loop_filter_level (6 bits)
    header.loop_filter.level = reader.read_literal(6)? as u8;

    // loop_filter_sharpness (3 bits)
    header.loop_filter.sharpness = reader.read_literal(3)? as u8;

    // loop_filter_delta_enabled (1 bit)
    header.loop_filter.mode_ref_delta_enabled = reader.read_literal(1)? != 0;

    if header.loop_filter.mode_ref_delta_enabled {
        // loop_filter_delta_update (1 bit)
        header.loop_filter.mode_ref_delta_update = reader.read_literal(1)? != 0;

        if header.loop_filter.mode_ref_delta_update {
            // Reference deltas
            for i in 0..4 {
                let update = reader.read_literal(1)? != 0;
                if update {
                    header.loop_filter.ref_deltas[i] = read_signed_literal(reader, 6)?;
                }
            }

            // Mode deltas
            for i in 0..2 {
                let update = reader.read_literal(1)? != 0;
                if update {
                    header.loop_filter.mode_deltas[i] = read_signed_literal(reader, 6)?;
                }
            }
        }
    }

    Ok(())
}

fn parse_quantization(reader: &mut BitReader, header: &mut FrameHeader) -> Result<()> {
    // base_q_idx (8 bits)
    header.quantization.base_q_idx = reader.read_literal(8)? as u8;

    // delta_q_y_dc
    header.quantization.delta_q_y_dc = read_delta_q(reader)?;

    // delta_q_uv_dc
    header.quantization.delta_q_uv_dc = read_delta_q(reader)?;

    // delta_q_uv_ac
    header.quantization.delta_q_uv_ac = read_delta_q(reader)?;

    // Check for lossless
    header.quantization.lossless = header.quantization.base_q_idx == 0
        && header.quantization.delta_q_y_dc == 0
        && header.quantization.delta_q_uv_dc == 0
        && header.quantization.delta_q_uv_ac == 0;

    Ok(())
}

fn read_delta_q(reader: &mut BitReader) -> Result<i8> {
    let delta_coded = reader.read_literal(1)? != 0;
    if delta_coded {
        read_signed_literal(reader, 4)
    } else {
        Ok(0)
    }
}

fn read_signed_literal(reader: &mut BitReader, bits: u8) -> Result<i8> {
    let value = reader.read_literal(bits)? as i8;
    let sign = reader.read_literal(1)? != 0;
    Ok(if sign { -value } else { value })
}

fn parse_segmentation(reader: &mut BitReader, header: &mut FrameHeader) -> Result<()> {
    // segmentation_enabled (1 bit)
    header.segmentation.enabled = reader.read_literal(1)? != 0;

    if !header.segmentation.enabled {
        return Ok(());
    }

    // segmentation_update_map (1 bit)
    header.segmentation.update_map = reader.read_literal(1)? != 0;

    if header.segmentation.update_map {
        // Skip segment tree probs (7 bits each, 7 probs)
        for _ in 0..7 {
            let prob_coded = reader.read_literal(1)? != 0;
            if prob_coded {
                let _prob = reader.read_literal(8)?;
            }
        }

        // segmentation_temporal_update (1 bit)
        header.segmentation.temporal_update = reader.read_literal(1)? != 0;

        if header.segmentation.temporal_update {
            // Skip prediction probs (3 probs)
            for _ in 0..3 {
                let prob_coded = reader.read_literal(1)? != 0;
                if prob_coded {
                    let _prob = reader.read_literal(8)?;
                }
            }
        }
    }

    // segmentation_update_data (1 bit)
    header.segmentation.update_data = reader.read_literal(1)? != 0;

    if header.segmentation.update_data {
        // abs_or_delta_update (1 bit)
        header.segmentation.abs_or_delta_update = reader.read_literal(1)? != 0;

        // Parse segment features
        for i in 0..8 {
            for j in 0..4 {
                let enabled = reader.read_literal(1)? != 0;
                header.segmentation.feature_enabled[i][j] = enabled;

                if enabled {
                    let bits = [8, 6, 2, 0][j]; // Feature bit counts
                    if bits > 0 {
                        header.segmentation.feature_data[i][j] =
                            read_signed_literal(reader, bits)? as i16;
                    }
                }
            }
        }
    }

    Ok(())
}

fn parse_tile_info(reader: &mut BitReader, header: &mut FrameHeader) -> Result<()> {
    // Calculate max tile cols log2 based on frame width
    let sb64_cols = (header.width + 63) / 64;
    let mut max_log2 = 0u8;
    while (sb64_cols >> (max_log2 + 1)) >= 1 {
        max_log2 += 1;
    }

    // tile_cols_log2
    header.tile_cols_log2 = 0;
    while header.tile_cols_log2 < max_log2 {
        let increment = reader.read_literal(1)? != 0;
        if increment {
            header.tile_cols_log2 += 1;
        } else {
            break;
        }
    }

    // tile_rows_log2
    let tile_rows_increment = reader.read_literal(1)? != 0;
    header.tile_rows_log2 = if tile_rows_increment {
        reader.read_literal(1)? as u8 + 1
    } else {
        0
    };

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_header_defaults() {
        let header = FrameHeader::default();
        assert!(header.is_key_frame());
        assert!(header.is_intra_only());
        assert_eq!(header.num_tiles(), 1);
    }

    #[test]
    fn test_frame_type() {
        assert!(FrameType::Key.is_key());
        assert!(!FrameType::Key.is_inter());
        assert!(!FrameType::Inter.is_key());
        assert!(FrameType::Inter.is_inter());
    }

    #[test]
    fn test_color_space() {
        assert_eq!(ColorSpace::from(0), ColorSpace::Unknown);
        assert_eq!(ColorSpace::from(2), ColorSpace::Bt709);
        assert_eq!(ColorSpace::from(7), ColorSpace::Srgb);
    }
}
