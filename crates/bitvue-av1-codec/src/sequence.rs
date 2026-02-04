//! AV1 Sequence Header parsing
//!
//! Reference: AV1 Specification Section 5.5

use serde::{Deserialize, Serialize};

use bitvue_core::Result;

use crate::bitreader::BitReader;

/// AV1 Profile
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Av1Profile {
    /// Main profile (0)
    Main,
    /// High profile (1)
    High,
    /// Professional profile (2)
    Professional,
    /// Reserved profile
    Reserved(u8),
}

impl Av1Profile {
    pub fn from_u8(value: u8) -> Self {
        match value {
            0 => Av1Profile::Main,
            1 => Av1Profile::High,
            2 => Av1Profile::Professional,
            v => Av1Profile::Reserved(v),
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Av1Profile::Main => "Main",
            Av1Profile::High => "High",
            Av1Profile::Professional => "Professional",
            Av1Profile::Reserved(_) => "Reserved",
        }
    }
}

impl std::fmt::Display for Av1Profile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// Color primaries
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum ColorPrimaries {
    Bt709 = 1,
    Unspecified = 2,
    Bt470M = 4,
    Bt470Bg = 5,
    Bt601 = 6,
    Smpte240 = 7,
    GenericFilm = 8,
    Bt2020 = 9,
    Xyz = 10,
    Smpte431 = 11,
    Smpte432 = 12,
    Ebu3213 = 22,
    Reserved(u8),
}

impl ColorPrimaries {
    pub fn from_u8(value: u8) -> Self {
        match value {
            1 => ColorPrimaries::Bt709,
            2 => ColorPrimaries::Unspecified,
            4 => ColorPrimaries::Bt470M,
            5 => ColorPrimaries::Bt470Bg,
            6 => ColorPrimaries::Bt601,
            7 => ColorPrimaries::Smpte240,
            8 => ColorPrimaries::GenericFilm,
            9 => ColorPrimaries::Bt2020,
            10 => ColorPrimaries::Xyz,
            11 => ColorPrimaries::Smpte431,
            12 => ColorPrimaries::Smpte432,
            22 => ColorPrimaries::Ebu3213,
            v => ColorPrimaries::Reserved(v),
        }
    }
}

/// Transfer characteristics
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum TransferCharacteristics {
    Bt709 = 1,
    Unspecified = 2,
    Bt470M = 4,
    Bt470Bg = 5,
    Bt601 = 6,
    Smpte240 = 7,
    Linear = 8,
    Log100 = 9,
    Log100Sqrt10 = 10,
    Iec61966 = 11,
    Bt1361 = 12,
    Srgb = 13,
    Bt202010Bit = 14,
    Bt202012Bit = 15,
    Smpte2084 = 16,
    Smpte428 = 17,
    Hlg = 18,
    Reserved(u8),
}

impl TransferCharacteristics {
    pub fn from_u8(value: u8) -> Self {
        match value {
            1 => TransferCharacteristics::Bt709,
            2 => TransferCharacteristics::Unspecified,
            4 => TransferCharacteristics::Bt470M,
            5 => TransferCharacteristics::Bt470Bg,
            6 => TransferCharacteristics::Bt601,
            7 => TransferCharacteristics::Smpte240,
            8 => TransferCharacteristics::Linear,
            9 => TransferCharacteristics::Log100,
            10 => TransferCharacteristics::Log100Sqrt10,
            11 => TransferCharacteristics::Iec61966,
            12 => TransferCharacteristics::Bt1361,
            13 => TransferCharacteristics::Srgb,
            14 => TransferCharacteristics::Bt202010Bit,
            15 => TransferCharacteristics::Bt202012Bit,
            16 => TransferCharacteristics::Smpte2084,
            17 => TransferCharacteristics::Smpte428,
            18 => TransferCharacteristics::Hlg,
            v => TransferCharacteristics::Reserved(v),
        }
    }
}

/// Matrix coefficients
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum MatrixCoefficients {
    Identity = 0,
    Bt709 = 1,
    Unspecified = 2,
    Fcc = 4,
    Bt470Bg = 5,
    Bt601 = 6,
    Smpte240 = 7,
    YCgCo = 8,
    Bt2020Ncl = 9,
    Bt2020Cl = 10,
    Smpte2085 = 11,
    ChromaDerivedNcl = 12,
    ChromaDerivedCl = 13,
    ICtCp = 14,
    Reserved(u8),
}

impl MatrixCoefficients {
    pub fn from_u8(value: u8) -> Self {
        match value {
            0 => MatrixCoefficients::Identity,
            1 => MatrixCoefficients::Bt709,
            2 => MatrixCoefficients::Unspecified,
            4 => MatrixCoefficients::Fcc,
            5 => MatrixCoefficients::Bt470Bg,
            6 => MatrixCoefficients::Bt601,
            7 => MatrixCoefficients::Smpte240,
            8 => MatrixCoefficients::YCgCo,
            9 => MatrixCoefficients::Bt2020Ncl,
            10 => MatrixCoefficients::Bt2020Cl,
            11 => MatrixCoefficients::Smpte2085,
            12 => MatrixCoefficients::ChromaDerivedNcl,
            13 => MatrixCoefficients::ChromaDerivedCl,
            14 => MatrixCoefficients::ICtCp,
            v => MatrixCoefficients::Reserved(v),
        }
    }
}

/// Chroma sample position
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChromaSamplePosition {
    Unknown = 0,
    Vertical = 1,
    Colocated = 2,
    Reserved = 3,
}

impl ChromaSamplePosition {
    pub fn from_u8(value: u8) -> Self {
        match value {
            0 => ChromaSamplePosition::Unknown,
            1 => ChromaSamplePosition::Vertical,
            2 => ChromaSamplePosition::Colocated,
            _ => ChromaSamplePosition::Reserved,
        }
    }
}

/// Color configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorConfig {
    pub bit_depth: u8,
    pub mono_chrome: bool,
    pub num_planes: u8,
    pub color_primaries: ColorPrimaries,
    pub transfer_characteristics: TransferCharacteristics,
    pub matrix_coefficients: MatrixCoefficients,
    pub color_range: bool,
    pub subsampling_x: bool,
    pub subsampling_y: bool,
    pub chroma_sample_position: ChromaSamplePosition,
    pub separate_uv_delta_q: bool,
}

impl ColorConfig {
    /// Returns the chroma subsampling as a string (e.g., "4:2:0", "4:4:4")
    pub fn chroma_subsampling_str(&self) -> &'static str {
        if self.mono_chrome {
            "4:0:0"
        } else if self.subsampling_x && self.subsampling_y {
            "4:2:0"
        } else if self.subsampling_x {
            "4:2:2"
        } else {
            "4:4:4"
        }
    }
}

/// Timing information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimingInfo {
    pub num_units_in_display_tick: u32,
    pub time_scale: u32,
    pub equal_picture_interval: bool,
    pub num_ticks_per_picture: Option<u32>,
}

/// Decoder model info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecoderModelInfo {
    pub buffer_delay_length_minus_1: u8,
    pub num_units_in_decoding_tick: u32,
    pub buffer_removal_time_length_minus_1: u8,
    pub frame_presentation_time_length_minus_1: u8,
}

/// Operating point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperatingPoint {
    pub idc: u16,
    pub seq_level_idx: u8,
    pub seq_tier: bool,
    pub decoder_model_present: bool,
    pub initial_display_delay: Option<u8>,
}

/// Parsed Sequence Header
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SequenceHeader {
    pub profile: Av1Profile,
    pub still_picture: bool,
    pub reduced_still_picture_header: bool,

    pub timing_info: Option<TimingInfo>,
    pub decoder_model_info: Option<DecoderModelInfo>,
    pub operating_points: Vec<OperatingPoint>,

    pub frame_width_bits_minus_1: u8,
    pub frame_height_bits_minus_1: u8,
    pub max_frame_width: u32,
    pub max_frame_height: u32,

    pub frame_id_numbers_present: bool,
    pub delta_frame_id_length_minus_2: Option<u8>,
    pub additional_frame_id_length_minus_1: Option<u8>,

    pub use_128x128_superblock: bool,
    pub enable_filter_intra: bool,
    pub enable_intra_edge_filter: bool,
    pub enable_interintra_compound: bool,
    pub enable_masked_compound: bool,
    pub enable_warped_motion: bool,
    pub enable_dual_filter: bool,
    pub enable_order_hint: bool,
    pub enable_jnt_comp: bool,
    pub enable_ref_frame_mvs: bool,
    pub seq_choose_screen_content_tools: bool,
    pub seq_force_screen_content_tools: u8,
    pub seq_choose_integer_mv: bool,
    pub seq_force_integer_mv: u8,
    pub order_hint_bits_minus_1: Option<u8>,

    pub enable_superres: bool,
    pub enable_cdef: bool,
    pub enable_restoration: bool,

    pub color_config: ColorConfig,
    pub film_grain_params_present: bool,
}

impl SequenceHeader {
    /// Returns the sequence level index from the first operating point
    pub fn level(&self) -> u8 {
        self.operating_points
            .first()
            .map(|op| op.seq_level_idx)
            .unwrap_or(0)
    }

    /// Returns the width
    pub fn width(&self) -> u32 {
        self.max_frame_width
    }

    /// Returns the height
    pub fn height(&self) -> u32 {
        self.max_frame_height
    }

    /// Returns the bit depth
    pub fn bit_depth(&self) -> u8 {
        self.color_config.bit_depth
    }
}

/// Parses a Sequence Header OBU payload
pub fn parse_sequence_header(data: &[u8]) -> Result<SequenceHeader> {
    let mut reader = BitReader::new(data);

    // seq_profile (3 bits)
    let profile = Av1Profile::from_u8(reader.read_bits(3)? as u8);

    // still_picture (1 bit)
    let still_picture = reader.read_bit()?;

    // reduced_still_picture_header (1 bit)
    let reduced_still_picture_header = reader.read_bit()?;

    let mut timing_info = None;
    let mut decoder_model_info = None;
    let mut operating_points = Vec::new();
    let initial_display_delay_present;

    if reduced_still_picture_header {
        // Simplified header for still pictures
        operating_points.push(OperatingPoint {
            idc: 0,
            seq_level_idx: reader.read_bits(5)? as u8,
            seq_tier: false,
            decoder_model_present: false,
            initial_display_delay: None,
        });
    } else {
        // timing_info_present_flag (1 bit)
        let timing_info_present = reader.read_bit()?;

        if timing_info_present {
            timing_info = Some(parse_timing_info(&mut reader)?);

            // decoder_model_info_present_flag (1 bit)
            let decoder_model_info_present = reader.read_bit()?;
            if decoder_model_info_present {
                decoder_model_info = Some(parse_decoder_model_info(&mut reader)?);
            }
        }

        // initial_display_delay_present_flag (1 bit)
        initial_display_delay_present = reader.read_bit()?;

        // operating_points_cnt_minus_1 (5 bits)
        let operating_points_cnt = reader.read_bits(5)? as usize + 1;

        for _ in 0..operating_points_cnt {
            let idc = reader.read_bits(12)? as u16;
            let seq_level_idx = reader.read_bits(5)? as u8;
            let seq_tier = if seq_level_idx > 7 {
                reader.read_bit()?
            } else {
                false
            };

            let decoder_model_present = if decoder_model_info.is_some() {
                reader.read_bit()?
            } else {
                false
            };

            if decoder_model_present {
                // Skip decoder model info for now
                let n = decoder_model_info
                    .as_ref()
                    .map(|d| d.buffer_delay_length_minus_1 + 1)
                    .unwrap_or(0);
                reader.skip_bits(n as u64)?; // decoder_buffer_delay
                reader.skip_bits(n as u64)?; // encoder_buffer_delay
                reader.read_bit()?; // low_delay_mode_flag
            }

            let initial_display_delay = if initial_display_delay_present {
                if reader.read_bit()? {
                    // initial_display_delay_present_for_this_op
                    Some(reader.read_bits(4)? as u8 + 1)
                } else {
                    None
                }
            } else {
                None
            };

            operating_points.push(OperatingPoint {
                idc,
                seq_level_idx,
                seq_tier,
                decoder_model_present,
                initial_display_delay,
            });
        }
    }

    // frame_width_bits_minus_1 (4 bits)
    let frame_width_bits_minus_1 = reader.read_bits(4)? as u8;
    // frame_height_bits_minus_1 (4 bits)
    let frame_height_bits_minus_1 = reader.read_bits(4)? as u8;

    // max_frame_width_minus_1 (n+1 bits)
    let max_frame_width = reader.read_bits(frame_width_bits_minus_1 + 1)? + 1;
    // max_frame_height_minus_1 (n+1 bits)
    let max_frame_height = reader.read_bits(frame_height_bits_minus_1 + 1)? + 1;

    let mut frame_id_numbers_present = false;
    let mut delta_frame_id_length_minus_2 = None;
    let mut additional_frame_id_length_minus_1 = None;

    if !reduced_still_picture_header {
        // frame_id_numbers_present_flag (1 bit)
        frame_id_numbers_present = reader.read_bit()?;

        if frame_id_numbers_present {
            // delta_frame_id_length_minus_2 (4 bits)
            delta_frame_id_length_minus_2 = Some(reader.read_bits(4)? as u8);
            // additional_frame_id_length_minus_1 (3 bits)
            additional_frame_id_length_minus_1 = Some(reader.read_bits(3)? as u8);
        }
    }

    // use_128x128_superblock (1 bit)
    let use_128x128_superblock = reader.read_bit()?;
    // enable_filter_intra (1 bit)
    let enable_filter_intra = reader.read_bit()?;
    // enable_intra_edge_filter (1 bit)
    let enable_intra_edge_filter = reader.read_bit()?;

    let mut enable_interintra_compound = false;
    let mut enable_masked_compound = false;
    let mut enable_warped_motion = false;
    let mut enable_dual_filter = false;
    let mut enable_order_hint = false;
    let mut enable_jnt_comp = false;
    let mut enable_ref_frame_mvs = false;
    let mut seq_choose_screen_content_tools = true;
    let mut seq_force_screen_content_tools = 2; // SELECT_SCREEN_CONTENT_TOOLS
    let mut seq_choose_integer_mv = true;
    let mut seq_force_integer_mv = 2; // SELECT_INTEGER_MV
    let mut order_hint_bits_minus_1 = None;

    if !reduced_still_picture_header {
        // enable_interintra_compound (1 bit)
        enable_interintra_compound = reader.read_bit()?;
        // enable_masked_compound (1 bit)
        enable_masked_compound = reader.read_bit()?;
        // enable_warped_motion (1 bit)
        enable_warped_motion = reader.read_bit()?;
        // enable_dual_filter (1 bit)
        enable_dual_filter = reader.read_bit()?;
        // enable_order_hint (1 bit)
        enable_order_hint = reader.read_bit()?;

        if enable_order_hint {
            // enable_jnt_comp (1 bit)
            enable_jnt_comp = reader.read_bit()?;
            // enable_ref_frame_mvs (1 bit)
            enable_ref_frame_mvs = reader.read_bit()?;
        }

        // seq_choose_screen_content_tools (1 bit)
        seq_choose_screen_content_tools = reader.read_bit()?;
        if seq_choose_screen_content_tools {
            seq_force_screen_content_tools = 2; // SELECT_SCREEN_CONTENT_TOOLS
        } else {
            // seq_force_screen_content_tools (1 bit)
            seq_force_screen_content_tools = reader.read_bits(1)? as u8;
        }

        if seq_force_screen_content_tools > 0 {
            // seq_choose_integer_mv (1 bit)
            seq_choose_integer_mv = reader.read_bit()?;
            if seq_choose_integer_mv {
                seq_force_integer_mv = 2; // SELECT_INTEGER_MV
            } else {
                // seq_force_integer_mv (1 bit)
                seq_force_integer_mv = reader.read_bits(1)? as u8;
            }
        } else {
            seq_force_integer_mv = 2;
        }

        if enable_order_hint {
            // order_hint_bits_minus_1 (3 bits)
            order_hint_bits_minus_1 = Some(reader.read_bits(3)? as u8);
        }
    }

    // enable_superres (1 bit)
    let enable_superres = reader.read_bit()?;
    // enable_cdef (1 bit)
    let enable_cdef = reader.read_bit()?;
    // enable_restoration (1 bit)
    let enable_restoration = reader.read_bit()?;

    // color_config
    let color_config = parse_color_config(&mut reader, &profile)?;

    // film_grain_params_present (1 bit)
    let film_grain_params_present = reader.read_bit()?;

    Ok(SequenceHeader {
        profile,
        still_picture,
        reduced_still_picture_header,
        timing_info,
        decoder_model_info,
        operating_points,
        frame_width_bits_minus_1,
        frame_height_bits_minus_1,
        max_frame_width,
        max_frame_height,
        frame_id_numbers_present,
        delta_frame_id_length_minus_2,
        additional_frame_id_length_minus_1,
        use_128x128_superblock,
        enable_filter_intra,
        enable_intra_edge_filter,
        enable_interintra_compound,
        enable_masked_compound,
        enable_warped_motion,
        enable_dual_filter,
        enable_order_hint,
        enable_jnt_comp,
        enable_ref_frame_mvs,
        seq_choose_screen_content_tools,
        seq_force_screen_content_tools,
        seq_choose_integer_mv,
        seq_force_integer_mv,
        order_hint_bits_minus_1,
        enable_superres,
        enable_cdef,
        enable_restoration,
        color_config,
        film_grain_params_present,
    })
}

fn parse_timing_info(reader: &mut BitReader) -> Result<TimingInfo> {
    let num_units_in_display_tick = reader.read_bits(32)?;
    let time_scale = reader.read_bits(32)?;
    let equal_picture_interval = reader.read_bit()?;

    let num_ticks_per_picture = if equal_picture_interval {
        Some(reader.read_uvlc()? + 1)
    } else {
        None
    };

    Ok(TimingInfo {
        num_units_in_display_tick,
        time_scale,
        equal_picture_interval,
        num_ticks_per_picture,
    })
}

fn parse_decoder_model_info(reader: &mut BitReader) -> Result<DecoderModelInfo> {
    let buffer_delay_length_minus_1 = reader.read_bits(5)? as u8;
    let num_units_in_decoding_tick = reader.read_bits(32)?;
    let buffer_removal_time_length_minus_1 = reader.read_bits(5)? as u8;
    let frame_presentation_time_length_minus_1 = reader.read_bits(5)? as u8;

    Ok(DecoderModelInfo {
        buffer_delay_length_minus_1,
        num_units_in_decoding_tick,
        buffer_removal_time_length_minus_1,
        frame_presentation_time_length_minus_1,
    })
}

fn parse_color_config(reader: &mut BitReader, profile: &Av1Profile) -> Result<ColorConfig> {
    // high_bitdepth (1 bit)
    let high_bitdepth = reader.read_bit()?;

    let bit_depth = if matches!(profile, Av1Profile::Professional) && high_bitdepth {
        // twelve_bit (1 bit)
        if reader.read_bit()? {
            12
        } else {
            10
        }
    } else if matches!(profile, Av1Profile::Professional) || high_bitdepth {
        10
    } else {
        8
    };

    let mono_chrome = if !matches!(profile, Av1Profile::High) {
        reader.read_bit()?
    } else {
        false
    };

    let num_planes = if mono_chrome { 1 } else { 3 };

    // color_description_present_flag (1 bit)
    let color_description_present = reader.read_bit()?;

    let (color_primaries, transfer_characteristics, matrix_coefficients) =
        if color_description_present {
            (
                ColorPrimaries::from_u8(reader.read_bits(8)? as u8),
                TransferCharacteristics::from_u8(reader.read_bits(8)? as u8),
                MatrixCoefficients::from_u8(reader.read_bits(8)? as u8),
            )
        } else {
            (
                ColorPrimaries::Unspecified,
                TransferCharacteristics::Unspecified,
                MatrixCoefficients::Unspecified,
            )
        };

    let (color_range, subsampling_x, subsampling_y, chroma_sample_position) = if mono_chrome {
        // color_range (1 bit)
        let color_range = reader.read_bit()?;
        (color_range, true, true, ChromaSamplePosition::Unknown)
    } else if matches!(color_primaries, ColorPrimaries::Bt709)
        && matches!(transfer_characteristics, TransferCharacteristics::Srgb)
        && matches!(matrix_coefficients, MatrixCoefficients::Identity)
    {
        (true, false, false, ChromaSamplePosition::Unknown)
    } else {
        // color_range (1 bit)
        let color_range = reader.read_bit()?;

        let (subsampling_x, subsampling_y) = if matches!(profile, Av1Profile::Main) {
            (true, true)
        } else if matches!(profile, Av1Profile::High) {
            (false, false)
        } else if bit_depth == 12 {
            let subsampling_x = reader.read_bit()?;
            let subsampling_y = if subsampling_x {
                reader.read_bit()?
            } else {
                false
            };
            (subsampling_x, subsampling_y)
        } else {
            (true, false)
        };

        let chroma_sample_position = if subsampling_x && subsampling_y {
            ChromaSamplePosition::from_u8(reader.read_bits(2)? as u8)
        } else {
            ChromaSamplePosition::Unknown
        };

        (
            color_range,
            subsampling_x,
            subsampling_y,
            chroma_sample_position,
        )
    };

    let separate_uv_delta_q = if !mono_chrome {
        reader.read_bit()?
    } else {
        false
    };

    Ok(ColorConfig {
        bit_depth,
        mono_chrome,
        num_planes,
        color_primaries,
        transfer_characteristics,
        matrix_coefficients,
        color_range,
        subsampling_x,
        subsampling_y,
        chroma_sample_position,
        separate_uv_delta_q,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profile_from_u8() {
        assert_eq!(Av1Profile::from_u8(0), Av1Profile::Main);
        assert_eq!(Av1Profile::from_u8(1), Av1Profile::High);
        assert_eq!(Av1Profile::from_u8(2), Av1Profile::Professional);
    }

    #[test]
    fn test_chroma_subsampling_str() {
        let config = ColorConfig {
            bit_depth: 8,
            mono_chrome: false,
            num_planes: 3,
            color_primaries: ColorPrimaries::Unspecified,
            transfer_characteristics: TransferCharacteristics::Unspecified,
            matrix_coefficients: MatrixCoefficients::Unspecified,
            color_range: false,
            subsampling_x: true,
            subsampling_y: true,
            chroma_sample_position: ChromaSamplePosition::Unknown,
            separate_uv_delta_q: false,
        };
        assert_eq!(config.chroma_subsampling_str(), "4:2:0");

        let config_444 = ColorConfig {
            subsampling_x: false,
            subsampling_y: false,
            ..config.clone()
        };
        assert_eq!(config_444.chroma_subsampling_str(), "4:4:4");

        let config_mono = ColorConfig {
            mono_chrome: true,
            ..config
        };
        assert_eq!(config_mono.chroma_subsampling_str(), "4:0:0");
    }
}
