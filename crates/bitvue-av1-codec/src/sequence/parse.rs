//! `sequence_header_obu()` (spec 5.5): from an OBU payload to a `SequenceHeader`.

use bitvue_engine::Result;

use super::color::parse_color_config;
use super::types::{Av1Profile, DecoderModelInfo, OperatingPoint, SequenceHeader, TimingInfo};
use crate::bitreader::BitReader;

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
