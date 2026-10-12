//! AV1 Sequence Header parsing
//!
//! Reference: AV1 Specification Section 5.5

use serde::{Deserialize, Serialize};

use bitvue_engine::Result;

use crate::bitreader::BitReader;

mod color;

use color::parse_color_config;
pub use color::*;

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

    let OperatingSection {
        timing_info,
        decoder_model_info,
        operating_points,
    } = read_operating_section(&mut reader, reduced_still_picture_header)?;

    // frame_width_bits_minus_1 (4 bits)
    let frame_width_bits_minus_1 = reader.read_bits(4)? as u8;
    // frame_height_bits_minus_1 (4 bits)
    let frame_height_bits_minus_1 = reader.read_bits(4)? as u8;

    // max_frame_width_minus_1 (n+1 bits)
    let max_frame_width = reader.read_bits(frame_width_bits_minus_1 + 1)? + 1;
    // max_frame_height_minus_1 (n+1 bits)
    let max_frame_height = reader.read_bits(frame_height_bits_minus_1 + 1)? + 1;
    let FrameIdSection {
        frame_id_numbers_present,
        delta_frame_id_length_minus_2,
        additional_frame_id_length_minus_1,
    } = read_frame_id_section(&mut reader, reduced_still_picture_header)?;

    // use_128x128_superblock (1 bit)
    let use_128x128_superblock = reader.read_bit()?;
    // enable_filter_intra (1 bit)
    let enable_filter_intra = reader.read_bit()?;
    // enable_intra_edge_filter (1 bit)
    let enable_intra_edge_filter = reader.read_bit()?;

    let CodingToolFlags {
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
    } = read_coding_tool_flags(&mut reader, reduced_still_picture_header)?;

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

/// The timing, decoder-model and operating-point part of the sequence header.
struct OperatingSection {
    timing_info: Option<TimingInfo>,
    decoder_model_info: Option<DecoderModelInfo>,
    operating_points: Vec<OperatingPoint>,
}

fn read_operating_section(
    reader: &mut BitReader,
    reduced_still_picture_header: bool,
) -> Result<OperatingSection> {
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
            timing_info = Some(parse_timing_info(reader)?);

            // decoder_model_info_present_flag (1 bit)
            let decoder_model_info_present = reader.read_bit()?;
            if decoder_model_info_present {
                decoder_model_info = Some(parse_decoder_model_info(reader)?);
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
    Ok(OperatingSection {
        timing_info,
        decoder_model_info,
        operating_points,
    })
}

/// `frame_id_numbers_present_flag` and the id lengths that follow it.
struct FrameIdSection {
    frame_id_numbers_present: bool,
    delta_frame_id_length_minus_2: Option<u8>,
    additional_frame_id_length_minus_1: Option<u8>,
}

fn read_frame_id_section(
    reader: &mut BitReader,
    reduced_still_picture_header: bool,
) -> Result<FrameIdSection> {
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
    Ok(FrameIdSection {
        frame_id_numbers_present,
        delta_frame_id_length_minus_2,
        additional_frame_id_length_minus_1,
    })
}

/// The block-level coding tool switches (`enable_interintra_compound` .. `order_hint_bits_minus_1`).
struct CodingToolFlags {
    enable_interintra_compound: bool,
    enable_masked_compound: bool,
    enable_warped_motion: bool,
    enable_dual_filter: bool,
    enable_order_hint: bool,
    enable_jnt_comp: bool,
    enable_ref_frame_mvs: bool,
    seq_choose_screen_content_tools: bool,
    seq_force_screen_content_tools: u8,
    seq_choose_integer_mv: bool,
    seq_force_integer_mv: u8,
    order_hint_bits_minus_1: Option<u8>,
}

fn read_coding_tool_flags(
    reader: &mut BitReader,
    reduced_still_picture_header: bool,
) -> Result<CodingToolFlags> {
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
    Ok(CodingToolFlags {
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

    /// Packs `(value, width)` pairs MSB first, as the spec's `f(n)` elements are laid out.
    fn pack(fields: &[(u32, u8)]) -> Vec<u8> {
        let mut bits = Vec::new();
        for &(value, width) in fields {
            for i in (0..width).rev() {
                bits.push(((value >> i) & 1) as u8);
            }
        }
        let mut out = vec![0u8; bits.len().div_ceil(8)];
        for (i, bit) in bits.iter().enumerate() {
            out[i / 8] |= bit << (7 - (i % 8));
        }
        out
    }

    /// A 16x16, 8-bit 4:2:0 sequence header with the three optional groups under test spelled
    /// out: `initial_display_delay_present_flag` with one operating point, frame ids, and the
    /// screen-content / integer-MV selection.
    fn header(
        display_delay: Option<u32>,
        frame_ids: Option<(u32, u32)>,
        screen_content: (u32, u32),
        integer_mv: (u32, u32),
    ) -> Vec<u8> {
        let mut f: Vec<(u32, u8)> = vec![
            (0, 3),                              // seq_profile
            (0, 1),                              // still_picture
            (0, 1),                              // reduced_still_picture_header
            (0, 1),                              // timing_info_present_flag
            (display_delay.is_some() as u32, 1), // initial_display_delay_present_flag
            (0, 5),                              // operating_points_cnt_minus_1
            (0, 12),                             // operating_point_idc[0]
            (0, 5),                              // seq_level_idx[0]
        ];
        if let Some(delay) = display_delay {
            f.push((1, 1)); // initial_display_delay_present_for_this_op
            f.push((delay, 4)); // initial_display_delay_minus_1
        }
        f.extend([(3, 4), (3, 4), (15, 4), (15, 4)]); // frame size: 4 bits each, 16x16
        match frame_ids {
            Some((delta_minus_2, additional_minus_1)) => {
                f.extend([(1, 1), (delta_minus_2, 4), (additional_minus_1, 3)])
            }
            None => f.push((0, 1)),
        }
        f.extend([(0, 1), (0, 1), (0, 1)]); // 128x128 sb, filter_intra, intra_edge_filter
        f.extend([(0, 1), (0, 1), (0, 1), (0, 1)]); // interintra, masked, warped, dual filter
        f.push((0, 1)); // enable_order_hint
        f.push((screen_content.0, 1)); // seq_choose_screen_content_tools
        if screen_content.0 == 0 {
            f.push((screen_content.1, 1)); // seq_force_screen_content_tools
        }
        let force_screen = if screen_content.0 == 1 {
            2
        } else {
            screen_content.1
        };
        if force_screen > 0 {
            f.push((integer_mv.0, 1)); // seq_choose_integer_mv
            if integer_mv.0 == 0 {
                f.push((integer_mv.1, 1)); // seq_force_integer_mv
            }
        }
        f.extend([(0, 1), (0, 1), (0, 1)]); // superres, cdef, restoration
        f.extend([(0, 1), (0, 1), (0, 1), (0, 1), (0, 2), (0, 1)]); // color_config
        f.push((0, 1)); // film_grain_params_present_flag
        pack(&f)
    }

    #[test]
    fn the_initial_display_delay_of_an_operating_point_is_the_coded_value_plus_one() {
        let seq = parse_sequence_header(&header(Some(5), None, (1, 0), (1, 0))).unwrap();
        assert_eq!(seq.operating_points[0].initial_display_delay, Some(6));
        assert_eq!((seq.max_frame_width, seq.max_frame_height), (16, 16));

        let without = parse_sequence_header(&header(None, None, (1, 0), (1, 0))).unwrap();
        assert_eq!(without.operating_points[0].initial_display_delay, None);
    }

    #[test]
    fn frame_id_lengths_are_read_in_4_and_3_bits() {
        let seq = parse_sequence_header(&header(None, Some((9, 5)), (1, 0), (1, 0))).unwrap();
        assert!(seq.frame_id_numbers_present);
        assert_eq!(seq.delta_frame_id_length_minus_2, Some(9));
        assert_eq!(seq.additional_frame_id_length_minus_1, Some(5));
        // Fields after the ids still line up.
        assert_eq!((seq.max_frame_width, seq.max_frame_height), (16, 16));
        assert!(seq.seq_choose_screen_content_tools);
    }

    #[test]
    fn forced_screen_content_and_integer_mv_flags_are_taken_from_the_bits() {
        // choose = 0 reads the forced value; the integer-MV pair is only present with
        // screen content tools on.
        let on = parse_sequence_header(&header(None, None, (0, 1), (0, 1))).unwrap();
        assert!(!on.seq_choose_screen_content_tools);
        assert_eq!(on.seq_force_screen_content_tools, 1);
        assert!(!on.seq_choose_integer_mv);
        assert_eq!(on.seq_force_integer_mv, 1);

        let integer_mv_off = parse_sequence_header(&header(None, None, (0, 1), (0, 0))).unwrap();
        assert_eq!(integer_mv_off.seq_force_integer_mv, 0);

        // Screen content tools forced off: no integer-MV bits, SELECT_INTEGER_MV (2).
        let off = parse_sequence_header(&header(None, None, (0, 0), (1, 0))).unwrap();
        assert_eq!(off.seq_force_screen_content_tools, 0);
        assert_eq!(off.seq_force_integer_mv, 2);
        assert_eq!((off.max_frame_width, off.max_frame_height), (16, 16));

        // Chosen per frame: SELECT (2) for both.
        let select = parse_sequence_header(&header(None, None, (1, 0), (1, 0))).unwrap();
        assert_eq!(select.seq_force_screen_content_tools, 2);
        assert_eq!(select.seq_force_integer_mv, 2);
    }
}
