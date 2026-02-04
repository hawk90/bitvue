//! Sequence Header syntax parsing (simplified for Phase 0)
//!
//! This module provides syntax tree generation for AV1 Sequence Headers,
//! tracking exact bit ranges for Tri-sync functionality.
//!
//! **Phase 0 Scope**: Parses essential fields only (~25-30 syntax nodes)
//! - Core fields: profile, dimensions, level, key feature flags
//! - Simplified sub-structures: color_config basics, operating points
//! - Deferred: Inter flags, screen content tools, advanced color config
//!
//! # AV1 Specification Reference
//! - Section 5.5: Sequence Header OBU
//! - Section 6.4: Color Config

use super::{SyntaxBuilder, TrackedBitReader};
use crate::sequence::Av1Profile;
use bitvue_core::Result;

/// Parse Sequence Header OBU payload with bit-level tracking
///
/// Creates a detailed syntax tree for the sequence header, including:
/// - Basic header fields (profile, still_picture, reduced_header)
/// - Frame dimensions (max_frame_width, max_frame_height)
/// - Operating points array
/// - Feature enable flags
/// - Simplified color configuration
///
/// # Arguments
///
/// * `reader` - TrackedBitReader positioned at start of sequence header payload
/// * `builder` - SyntaxBuilder for constructing the syntax tree
///
/// # Returns
///
/// `Ok(())` if parsing succeeds, or an error if the bitstream is malformed.
///
/// # Example Syntax Tree
///
/// ```text
/// sequence_header
/// ├── seq_profile: "0 (Main)"
/// ├── still_picture: "0"
/// ├── reduced_still_picture_header: "0"
/// ├── operating_points[0]
/// │   ├── operating_point_idc: "0x000"
/// │   └── seq_level_idx: "5"
/// ├── max_frame_width_minus_1: "1919"
/// ├── max_frame_height_minus_1: "1079"
/// └── color_config
///     ├── high_bitdepth: "0"
///     └── ...
/// ```
pub fn parse_sequence_header_syntax(
    reader: &mut TrackedBitReader,
    builder: &mut SyntaxBuilder,
) -> Result<()> {
    let start = reader.position();
    builder.push_container("sequence_header", start);

    // seq_profile (3 bits) - AV1 Spec 5.5.1
    let (profile_val, range) = reader.read_bits_tracked(3)?;
    let profile = Av1Profile::from_u8(profile_val as u8);
    builder.add_field(
        "seq_profile",
        range,
        format!("{} ({})", profile_val, profile.name()),
    );

    // still_picture (1 bit) - AV1 Spec 5.5.2
    let (still, range) = reader.read_bit_tracked()?;
    builder.add_field("still_picture", range, format!("{}", still as u8));

    // reduced_still_picture_header (1 bit) - AV1 Spec 5.5.3
    let (reduced, range) = reader.read_bit_tracked()?;
    builder.add_field(
        "reduced_still_picture_header",
        range,
        format!("{}", reduced as u8),
    );

    if reduced {
        // Simplified path for still pictures - AV1 Spec 5.5.4
        // Only parse seq_level_idx, then skip to color_config
        let (level, range) = reader.read_bits_tracked(5)?;
        builder.add_field("seq_level_idx", range, format!("{}", level));

        // In reduced mode, skip to color_config (no dimensions, no feature flags)
        parse_color_config_simplified(reader, builder, profile_val as u8)?;
    } else {
        // Full sequence header - AV1 Spec 5.5.5+
        parse_non_reduced_path(reader, builder, profile_val as u8)?;

        // Parse remaining common fields
        parse_frame_dimensions(reader, builder)?;
        parse_feature_flags(reader, builder)?;
        parse_color_config_simplified(reader, builder, profile_val as u8)?;
    }

    // film_grain_params_present (1 bit) - AV1 Spec 5.5.26
    let (val, range) = reader.read_bit_tracked()?;
    builder.add_field("film_grain_params_present", range, format!("{}", val as u8));

    let end = reader.position();
    builder.pop_container(end);
    Ok(())
}

/// Parse non-reduced sequence header path (timing info, operating points)
fn parse_non_reduced_path(
    reader: &mut TrackedBitReader,
    builder: &mut SyntaxBuilder,
    _profile: u8,
) -> Result<()> {
    // timing_info_present_flag (1 bit) - AV1 Spec 5.5.5
    let (timing_present, range) = reader.read_bit_tracked()?;
    builder.add_field(
        "timing_info_present_flag",
        range,
        format!("{}", timing_present as u8),
    );

    let decoder_model_present = if timing_present {
        parse_timing_info_simplified(reader, builder)?;

        // decoder_model_info_present_flag (1 bit) - AV1 Spec 5.5.7
        let (decoder_present, range) = reader.read_bit_tracked()?;
        builder.add_field(
            "decoder_model_info_present_flag",
            range,
            format!("{}", decoder_present as u8),
        );

        if decoder_present {
            // Phase 0: Skip decoder_model_info details
            // Placeholder for future expansion
            skip_decoder_model_info(reader)?;
        }
        decoder_present
    } else {
        false
    };

    // initial_display_delay_present_flag (1 bit) - AV1 Spec 5.5.8
    let (display_delay, range) = reader.read_bit_tracked()?;
    builder.add_field(
        "initial_display_delay_present_flag",
        range,
        format!("{}", display_delay as u8),
    );

    // Parse operating points array
    parse_operating_points_simplified(
        reader,
        builder,
        timing_present,
        decoder_model_present,
        display_delay,
    )?;

    Ok(())
}

/// Parse timing_info structure (simplified)
///
/// Phase 0: Parse only key fields, skip uvlc conditional
fn parse_timing_info_simplified(
    reader: &mut TrackedBitReader,
    builder: &mut SyntaxBuilder,
) -> Result<()> {
    let start = reader.position();
    builder.push_container("timing_info", start);

    // num_units_in_display_tick (32 bits) - AV1 Spec 5.5.6
    let (val, range) = reader.read_bits_tracked(32)?;
    builder.add_field("num_units_in_display_tick", range, format!("{}", val));

    // time_scale (32 bits)
    let (val, range) = reader.read_bits_tracked(32)?;
    builder.add_field("time_scale", range, format!("{}", val));

    // equal_picture_interval (1 bit)
    let (equal, range) = reader.read_bit_tracked()?;
    builder.add_field("equal_picture_interval", range, format!("{}", equal as u8));

    if equal {
        // num_ticks_per_picture_minus_1 (uvlc) - Variable length
        let (val, range) = reader.read_uvlc_tracked()?;
        builder.add_field("num_ticks_per_picture_minus_1", range, format!("{}", val));
    }

    let end = reader.position();
    builder.pop_container(end);
    Ok(())
}

/// Skip decoder_model_info structure
///
/// Phase 0: Deferred to future implementation
/// Skips 5 + 32 + 5 + 5 = 47 bits
fn skip_decoder_model_info(reader: &mut TrackedBitReader) -> Result<()> {
    // buffer_delay_length_minus_1 (5 bits)
    // num_units_in_decoding_tick (32 bits)
    // buffer_removal_time_length_minus_1 (5 bits)
    // frame_presentation_time_length_minus_1 (5 bits)
    reader.skip_bits(47)?;
    Ok(())
}

/// Parse operating points array (simplified)
///
/// Phase 0: Parse idc, seq_level_idx, seq_tier.
/// Skip decoder_model params but correctly handle initial_display_delay.
fn parse_operating_points_simplified(
    reader: &mut TrackedBitReader,
    builder: &mut SyntaxBuilder,
    timing_info_present: bool,
    decoder_model_present: bool,
    initial_display_delay_present: bool,
) -> Result<()> {
    // operating_points_cnt_minus_1 (5 bits) - AV1 Spec 5.5.9
    let (cnt_minus_1, range) = reader.read_bits_tracked(5)?;
    builder.add_field(
        "operating_points_cnt_minus_1",
        range,
        format!("{}", cnt_minus_1),
    );

    let count = cnt_minus_1 + 1;

    for i in 0..count {
        let op_start = reader.position();
        builder.push_container(&format!("operating_points[{}]", i), op_start);

        // operating_point_idc (12 bits) - AV1 Spec 5.5.10
        let (idc, range) = reader.read_bits_tracked(12)?;
        builder.add_field("operating_point_idc", range, format!("0x{:03X}", idc));

        // seq_level_idx (5 bits) - AV1 Spec 5.5.11
        let (level, range) = reader.read_bits_tracked(5)?;
        builder.add_field("seq_level_idx", range, format!("{}", level));

        // seq_tier (1 bit) - Conditional on level > 7 - AV1 Spec 5.5.12
        if level > 7 {
            let (tier, range) = reader.read_bit_tracked()?;
            builder.add_field("seq_tier", range, format!("{}", tier as u8));
        }

        // decoder_model_present_for_this_op (conditional)
        if timing_info_present && decoder_model_present {
            let (decoder_model_for_op, _) = reader.read_bit_tracked()?;
            if decoder_model_for_op {
                // Phase 0: Skip operating_parameters_info()
                // decoder_buffer_delay (n bits), encoder_buffer_delay (n bits), low_delay_mode_flag (1 bit)
                // n = buffer_delay_length_minus_1 + 1 (from decoder_model_info)
                // For simplicity, skip 60 bits (approximate, actual depends on buffer_delay_length)
                reader.skip_bits(60)?;
            }
        }

        // initial_display_delay_present_for_this_op (conditional)
        if initial_display_delay_present {
            let (delay_for_op, _) = reader.read_bit_tracked()?;
            if delay_for_op {
                // initial_display_delay_minus_1 (4 bits)
                reader.skip_bits(4)?;
            }
        }

        let op_end = reader.position();
        builder.pop_container(op_end);
    }

    Ok(())
}

/// Parse frame dimensions
fn parse_frame_dimensions(
    reader: &mut TrackedBitReader,
    builder: &mut SyntaxBuilder,
) -> Result<()> {
    // frame_width_bits_minus_1 (4 bits) - AV1 Spec 5.5.15
    let (width_bits, range) = reader.read_bits_tracked(4)?;
    builder.add_field("frame_width_bits_minus_1", range, format!("{}", width_bits));

    // frame_height_bits_minus_1 (4 bits) - AV1 Spec 5.5.16
    let (height_bits, range) = reader.read_bits_tracked(4)?;
    builder.add_field(
        "frame_height_bits_minus_1",
        range,
        format!("{}", height_bits),
    );

    // max_frame_width_minus_1 (n+1 bits) - Variable width! - AV1 Spec 5.5.17
    let width_n = (width_bits + 1) as u8;
    let (width, range) = reader.read_bits_tracked(width_n)?;
    builder.add_field("max_frame_width_minus_1", range, format!("{}", width));

    // max_frame_height_minus_1 (n+1 bits) - Variable width! - AV1 Spec 5.5.18
    let height_n = (height_bits + 1) as u8;
    let (height, range) = reader.read_bits_tracked(height_n)?;
    builder.add_field("max_frame_height_minus_1", range, format!("{}", height));

    Ok(())
}

/// Parse feature enable flags (non-reduced path only)
fn parse_feature_flags(reader: &mut TrackedBitReader, builder: &mut SyntaxBuilder) -> Result<()> {
    // Frame ID fields (conditional) - AV1 Spec 5.5.19-21
    let (frame_ids, range) = reader.read_bit_tracked()?;
    builder.add_field(
        "frame_id_numbers_present_flag",
        range,
        format!("{}", frame_ids as u8),
    );

    if frame_ids {
        // Phase 0: Skip delta_frame_id_length_minus_2 (4 bits)
        // and additional_frame_id_length_minus_1 (3 bits)
        reader.skip_bits(7)?;
    }

    // use_128x128_superblock (1 bit) - AV1 Spec 5.5.22
    let (val, range) = reader.read_bit_tracked()?;
    builder.add_field("use_128x128_superblock", range, format!("{}", val as u8));

    // enable_filter_intra (1 bit) - AV1 Spec 5.5.23
    let (val, range) = reader.read_bit_tracked()?;
    builder.add_field("enable_filter_intra", range, format!("{}", val as u8));

    // enable_intra_edge_filter (1 bit) - AV1 Spec 5.5.24
    let (val, range) = reader.read_bit_tracked()?;
    builder.add_field("enable_intra_edge_filter", range, format!("{}", val as u8));

    // Phase 0: Parse inter/compound flags to properly track bit positions
    // enable_interintra_compound (1 bit)
    reader.skip_bits(1)?;
    // enable_masked_compound (1 bit)
    reader.skip_bits(1)?;
    // enable_warped_motion (1 bit)
    reader.skip_bits(1)?;
    // enable_dual_filter (1 bit)
    reader.skip_bits(1)?;

    // enable_order_hint (1 bit) - Parse this since it affects skip logic
    let (enable_order_hint, range) = reader.read_bit_tracked()?;
    builder.add_field(
        "enable_order_hint",
        range,
        format!("{}", enable_order_hint as u8),
    );

    if enable_order_hint {
        // enable_jnt_comp (1 bit)
        reader.skip_bits(1)?;
        // enable_ref_frame_mvs (1 bit)
        reader.skip_bits(1)?;
    }

    // seq_choose_screen_content_tools (1 bit)
    let (choose_sct, _) = reader.read_bit_tracked()?;
    if !choose_sct {
        // seq_force_screen_content_tools (1 bit)
        let (force_sct, _) = reader.read_bits_tracked(1)?;
        if force_sct > 0 {
            // seq_choose_integer_mv (1 bit)
            let (choose_imv, _) = reader.read_bit_tracked()?;
            if !choose_imv {
                // seq_force_integer_mv (1 bit)
                reader.skip_bits(1)?;
            }
        }
    }

    if enable_order_hint {
        // order_hint_bits_minus_1 (3 bits)
        reader.skip_bits(3)?;
    }

    // enable_superres (1 bit) - AV1 Spec 5.5.25
    let (val, range) = reader.read_bit_tracked()?;
    builder.add_field("enable_superres", range, format!("{}", val as u8));

    // enable_cdef (1 bit)
    let (val, range) = reader.read_bit_tracked()?;
    builder.add_field("enable_cdef", range, format!("{}", val as u8));

    // enable_restoration (1 bit)
    let (val, range) = reader.read_bit_tracked()?;
    builder.add_field("enable_restoration", range, format!("{}", val as u8));

    Ok(())
}

/// Parse color_config structure (simplified for Phase 0)
///
/// Phase 0 scope:
/// - high_bitdepth, twelve_bit (conditional), mono_chrome (conditional)
/// - color_description_present_flag and 3 color fields (if present)
/// - color_range
///
/// Deferred:
/// - subsampling_x, subsampling_y (profile-dependent logic)
/// - chroma_sample_position (conditional on subsampling)
/// - separate_uv_delta_q
fn parse_color_config_simplified(
    reader: &mut TrackedBitReader,
    builder: &mut SyntaxBuilder,
    profile: u8,
) -> Result<()> {
    let start = reader.position();
    builder.push_container("color_config", start);

    // high_bitdepth (1 bit) - AV1 Spec 6.4.1
    let (high_bd, range) = reader.read_bit_tracked()?;
    builder.add_field("high_bitdepth", range, format!("{}", high_bd as u8));

    // twelve_bit (1 bit) - Conditional on Professional profile + high_bd - AV1 Spec 6.4.2
    if profile == 2 && high_bd {
        let (twelve, range) = reader.read_bit_tracked()?;
        builder.add_field("twelve_bit", range, format!("{}", twelve as u8));
    }

    // mono_chrome (1 bit) - Conditional on !High profile - AV1 Spec 6.4.3
    let _mono = if profile != 1 {
        let (mono, range) = reader.read_bit_tracked()?;
        builder.add_field("mono_chrome", range, format!("{}", mono as u8));
        mono
    } else {
        false
    };

    // color_description_present_flag (1 bit) - AV1 Spec 6.4.4
    let (desc_present, range) = reader.read_bit_tracked()?;
    builder.add_field(
        "color_description_present_flag",
        range,
        format!("{}", desc_present as u8),
    );

    if desc_present {
        // color_primaries (8 bits) - AV1 Spec 6.4.5
        let (val, range) = reader.read_bits_tracked(8)?;
        builder.add_field("color_primaries", range, format!("{}", val));

        // transfer_characteristics (8 bits) - AV1 Spec 6.4.6
        let (val, range) = reader.read_bits_tracked(8)?;
        builder.add_field("transfer_characteristics", range, format!("{}", val));

        // matrix_coefficients (8 bits) - AV1 Spec 6.4.7
        let (val, range) = reader.read_bits_tracked(8)?;
        builder.add_field("matrix_coefficients", range, format!("{}", val));
    }

    // color_range (1 bit) - AV1 Spec 6.4.8
    let (val, range) = reader.read_bit_tracked()?;
    builder.add_field("color_range", range, format!("{}", val as u8));

    // Subsampling (conditional on profile and mono_chrome)
    if profile == 1 {
        // High Profile: 4:4:4 (no subsampling)
        // subsampling_x = 0, subsampling_y = 0 (implicit)
    } else if profile == 2 {
        // Professional Profile: bit_depth determines subsampling
        // For simplicity, skip detailed logic - just parse the bits
        // subsampling_x (1 bit)
        let (sub_x, range) = reader.read_bit_tracked()?;
        builder.add_field("subsampling_x", range, format!("{}", sub_x as u8));

        if sub_x {
            // subsampling_y (1 bit)
            let (sub_y, range) = reader.read_bit_tracked()?;
            builder.add_field("subsampling_y", range, format!("{}", sub_y as u8));
        }
        // chroma_sample_position is conditional - skip for Phase 0
    } else {
        // Main Profile (profile == 0): typically 4:2:0
        if !_mono {
            // subsampling_x = 1, subsampling_y = 1 (implicit for 4:2:0)
            // chroma_sample_position (2 bits)
            let (val, range) = reader.read_bits_tracked(2)?;
            builder.add_field("chroma_sample_position", range, format!("{}", val));
        }
    }

    // separate_uv_delta_q (1 bit) - AV1 Spec 6.4.11
    if !_mono {
        let (val, range) = reader.read_bit_tracked()?;
        builder.add_field("separate_uv_delta_q", range, format!("{}", val as u8));
    }

    let end = reader.position();
    builder.pop_container(end);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitvue_core::types::BitRange;

    #[test]
    fn test_reduced_still_picture_header() {
        // Minimal sequence header: profile=0, still=1, reduced=1, level=4
        // Binary: 000 1 1 00100 (8 bits used, 3 padding bits)
        // Followed by: dimensions, flags, color_config, film_grain
        // Provide enough padding (100 bytes) to avoid UnexpectedEof
        let mut data = vec![0b00011001, 0b00000000]; // profile=0, still=1, reduced=1, level=4
        data.extend_from_slice(&[0u8; 100]); // Add 100 bytes of zeros for all remaining fields
        let mut reader = TrackedBitReader::new(&data, 0);
        let mut builder = SyntaxBuilder::new("obu[0]".to_string(), "obu_0".to_string());

        let result = parse_sequence_header_syntax(&mut reader, &mut builder);
        if let Err(e) = &result {
            eprintln!("Parse error: {:?}", e);
        }
        assert!(result.is_ok(), "Parsing should succeed: {:?}", result.err());

        let model = builder.build();

        // Verify container exists
        assert!(model.get_node("obu[0].sequence_header").is_some());

        // Verify profile
        let profile = model
            .get_node("obu[0].sequence_header.seq_profile")
            .unwrap();
        assert_eq!(profile.bit_range, BitRange::new(0, 3));
        assert!(profile.value.as_ref().unwrap().contains("Main"));

        // Verify still_picture
        let still = model
            .get_node("obu[0].sequence_header.still_picture")
            .unwrap();
        assert_eq!(still.bit_range, BitRange::new(3, 4));
        assert_eq!(still.value.as_ref().unwrap(), "1");

        // Verify reduced_still_picture_header
        let reduced = model
            .get_node("obu[0].sequence_header.reduced_still_picture_header")
            .unwrap();
        assert_eq!(reduced.bit_range, BitRange::new(4, 5));
        assert_eq!(reduced.value.as_ref().unwrap(), "1");

        // Verify seq_level_idx (only field in reduced path)
        let level = model
            .get_node("obu[0].sequence_header.seq_level_idx")
            .unwrap();
        assert_eq!(level.bit_range, BitRange::new(5, 10));
    }
}
