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
use bitvue_engine::Result;

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
    use bitvue_engine::types::{BitRange, SyntaxModel};

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

    // --- helpers for the tests below ---------------------------------------

    /// MSB-first bit writer with the trailing-one tail of an OBU payload.
    struct Bits(Vec<bool>);

    impl Bits {
        fn new() -> Self {
            Bits(Vec::new())
        }
        fn put(&mut self, value: u64, width: u32) -> &mut Self {
            for i in (0..width).rev() {
                self.0.push((value >> i) & 1 == 1);
            }
            self
        }
        fn len(&self) -> usize {
            self.0.len()
        }
        fn bytes(&self) -> Vec<u8> {
            let mut bits = self.0.clone();
            bits.push(true);
            while !bits.len().is_multiple_of(8) {
                bits.push(false);
            }
            bits.chunks(8)
                .map(|c| c.iter().fold(0u8, |a, &b| (a << 1) | b as u8))
                .collect()
        }
    }

    /// Everything after the operating points of a non-reduced, profile 0 header:
    /// 8-bit width/height fields, no frame ids, order hint off, plain colour config.
    fn tail_after_operating_points(w: &mut Bits) {
        w.put(7, 4).put(7, 4); // frame_width_bits_minus_1, frame_height_bits_minus_1
        w.put(0xAB, 8).put(0xCD, 8); // max_frame_width/height_minus_1
        w.put(0, 1); // frame_id_numbers_present_flag
        w.put(0b101, 3); // use_128x128, filter_intra, intra_edge
        w.put(0, 4); // interintra, masked, warped, dual_filter
        w.put(0, 1); // enable_order_hint
        w.put(0, 1).put(0, 1); // choose_sct = 0, force_sct = 0
        w.put(0b011, 3); // superres, cdef, restoration
        w.put(0, 1).put(0, 1).put(0, 1); // high_bitdepth, mono_chrome, colour description
        w.put(1, 1); // color_range
        w.put(0b10, 2); // chroma_sample_position
        w.put(1, 1); // separate_uv_delta_q
        w.put(1, 1); // film_grain_params_present
    }

    fn parse(data: &[u8]) -> (Result<()>, SyntaxModel) {
        let mut reader = TrackedBitReader::new(data, 0);
        let mut builder = SyntaxBuilder::new("o".to_string(), "o".to_string());
        let result = parse_sequence_header_syntax(&mut reader, &mut builder);
        (result, builder.build())
    }

    fn value(model: &SyntaxModel, path: &str) -> Option<String> {
        model
            .get_node(&format!("o.sequence_header.{path}"))
            .and_then(|n| n.value.clone())
    }

    fn range(model: &SyntaxModel, path: &str) -> BitRange {
        model
            .get_node(&format!("o.sequence_header.{path}"))
            .unwrap_or_else(|| panic!("missing node {path}"))
            .bit_range
    }

    // --- tests ---------------------------------------------------------------

    #[test]
    fn a_plain_header_reports_every_field_with_its_bit_range() {
        let mut w = Bits::new();
        w.put(0, 3).put(0, 1).put(0, 1); // profile 0, still 0, reduced 0
        w.put(0, 1); // timing_info_present_flag
        w.put(0, 1); // initial_display_delay_present_flag
        w.put(0, 5); // operating_points_cnt_minus_1
        w.put(0x123, 12).put(5, 5); // idc, seq_level_idx
        tail_after_operating_points(&mut w);
        let (result, m) = parse(&w.bytes());
        result.unwrap();

        assert_eq!(
            range(&m, "operating_points[0].operating_point_idc"),
            BitRange::new(12, 24)
        );
        assert_eq!(
            value(&m, "operating_points[0].operating_point_idc").unwrap(),
            "0x123"
        );
        assert_eq!(value(&m, "operating_points[0].seq_level_idx").unwrap(), "5");
        // level 5 <= 7: no tier bit
        assert!(m
            .get_node("o.sequence_header.operating_points[0].seq_tier")
            .is_none());
        assert_eq!(value(&m, "max_frame_width_minus_1").unwrap(), "171"); // 0xAB
        assert_eq!(value(&m, "max_frame_height_minus_1").unwrap(), "205"); // 0xCD
        assert_eq!(
            range(&m, "max_frame_width_minus_1").end_bit
                - range(&m, "max_frame_width_minus_1").start_bit,
            8
        );
        assert_eq!(value(&m, "use_128x128_superblock").unwrap(), "1");
        assert_eq!(value(&m, "enable_filter_intra").unwrap(), "0");
        assert_eq!(value(&m, "enable_intra_edge_filter").unwrap(), "1");
        assert_eq!(value(&m, "enable_superres").unwrap(), "0");
        assert_eq!(value(&m, "enable_cdef").unwrap(), "1");
        assert_eq!(value(&m, "enable_restoration").unwrap(), "1");
        assert_eq!(value(&m, "color_config.color_range").unwrap(), "1");
        assert_eq!(
            value(&m, "color_config.chroma_sample_position").unwrap(),
            "2"
        );
        assert_eq!(value(&m, "color_config.separate_uv_delta_q").unwrap(), "1");
        assert_eq!(value(&m, "film_grain_params_present").unwrap(), "1");
        // the container spans the whole header
        let whole = range(&m, "film_grain_params_present");
        assert_eq!(whole.end_bit, w.len() as u64);
    }

    #[test]
    fn timing_info_reads_two_32_bit_words_and_the_uvlc_only_when_the_interval_is_equal() {
        for (equal, uvlc_bits, expect_field) in [(false, 0u32, false), (true, 0b00101, true)] {
            let mut w = Bits::new();
            w.put(0, 5); // profile, still, reduced
            w.put(1, 1); // timing_info_present_flag
            w.put(0xDEAD_BEEF, 32).put(30_000, 32).put(equal as u64, 1);
            if equal {
                w.put(uvlc_bits as u64, 5); // uvlc(4): two leading zeros, 1, two bits
            }
            w.put(0, 1); // decoder_model_info_present_flag
            w.put(0, 1).put(0, 5).put(0, 12).put(4, 5);
            tail_after_operating_points(&mut w);
            let (result, m) = parse(&w.bytes());
            result.unwrap();

            assert_eq!(
                value(&m, "timing_info.num_units_in_display_tick").unwrap(),
                "3735928559"
            );
            assert_eq!(value(&m, "timing_info.time_scale").unwrap(), "30000");
            assert_eq!(
                value(&m, "timing_info.equal_picture_interval").unwrap(),
                (equal as u8).to_string()
            );
            assert_eq!(
                value(&m, "timing_info.num_ticks_per_picture_minus_1").is_some(),
                expect_field
            );
            if expect_field {
                assert_eq!(
                    value(&m, "timing_info.num_ticks_per_picture_minus_1").unwrap(),
                    "4"
                );
            }
            // the bits after the uvlc are still in step
            assert_eq!(value(&m, "operating_points[0].seq_level_idx").unwrap(), "4");
            assert_eq!(value(&m, "film_grain_params_present").unwrap(), "1");
        }
    }

    #[test]
    fn decoder_model_info_is_skipped_as_exactly_47_bits() {
        let mut w = Bits::new();
        w.put(0, 5).put(1, 1);
        w.put(1, 32).put(2, 32).put(0, 1); // timing, not equal
        w.put(1, 1); // decoder_model_info_present_flag
        w.put(0x1F, 5)
            .put(0xFFFF_FFFF, 32)
            .put(0x1F, 5)
            .put(0x1F, 5); // 47 bits of ones
        w.put(0, 1); // initial_display_delay_present_flag
        w.put(0, 5); // one operating point
        w.put(0x0AA, 12).put(4, 5);
        w.put(0, 1); // decoder_model_present_for_this_op = 0
        tail_after_operating_points(&mut w);
        let (result, m) = parse(&w.bytes());
        result.unwrap();
        assert_eq!(value(&m, "decoder_model_info_present_flag").unwrap(), "1");
        assert_eq!(
            value(&m, "operating_points[0].operating_point_idc").unwrap(),
            "0x0AA"
        );
        assert_eq!(value(&m, "film_grain_params_present").unwrap(), "1");
    }

    #[test]
    fn the_tier_bit_is_read_from_level_index_8_and_not_at_7() {
        for (level, tier_present) in [(0u64, false), (7, false), (8, true), (31, true)] {
            let mut w = Bits::new();
            w.put(0, 5).put(0, 1).put(0, 1).put(0, 5);
            w.put(0, 12).put(level, 5);
            if tier_present {
                w.put(1, 1);
            }
            tail_after_operating_points(&mut w);
            let (result, m) = parse(&w.bytes());
            result.unwrap();
            assert_eq!(
                value(&m, "operating_points[0].seq_tier").is_some(),
                tier_present,
                "level {level}"
            );
            assert_eq!(
                value(&m, "film_grain_params_present").unwrap(),
                "1",
                "level {level}"
            );
        }
    }

    #[test]
    fn every_operating_point_gets_its_own_container_and_display_delay_bits() {
        let mut w = Bits::new();
        w.put(0, 5).put(0, 1);
        w.put(1, 1); // initial_display_delay_present_flag
        w.put(2, 5); // three operating points
                     // op 0: no delay, op 1: delay present with 4 bits, op 2: level 9 (tier) and delay present
        w.put(0x001, 12).put(2, 5).put(0, 1);
        w.put(0x002, 12).put(3, 5).put(1, 1).put(0b1010, 4);
        w.put(0x003, 12)
            .put(9, 5)
            .put(0, 1)
            .put(1, 1)
            .put(0b0111, 4);
        tail_after_operating_points(&mut w);
        let (result, m) = parse(&w.bytes());
        result.unwrap();
        assert_eq!(value(&m, "operating_points_cnt_minus_1").unwrap(), "2");
        for (i, idc) in ["0x001", "0x002", "0x003"].iter().enumerate() {
            assert_eq!(
                value(&m, &format!("operating_points[{i}].operating_point_idc")).unwrap(),
                *idc
            );
        }
        assert_eq!(value(&m, "operating_points[2].seq_tier").unwrap(), "0");
        assert_eq!(value(&m, "film_grain_params_present").unwrap(), "1");
    }

    #[test]
    fn the_largest_operating_point_count_is_read_in_full() {
        let mut w = Bits::new();
        w.put(0, 5).put(0, 1).put(0, 1).put(31, 5); // 32 operating points
        for i in 0..32u64 {
            w.put(i, 12).put(1, 5);
        }
        tail_after_operating_points(&mut w);
        let (result, m) = parse(&w.bytes());
        result.unwrap();
        assert_eq!(
            value(&m, "operating_points[31].operating_point_idc").unwrap(),
            "0x01F"
        );
        assert_eq!(value(&m, "film_grain_params_present").unwrap(), "1");
    }

    #[test]
    fn frame_dimensions_use_the_declared_widths() {
        for (wb, hb) in [(0u64, 0u64), (3, 12), (15, 15)] {
            let mut w = Bits::new();
            w.put(0, 5)
                .put(0, 1)
                .put(0, 1)
                .put(0, 5)
                .put(0, 12)
                .put(1, 5);
            w.put(wb, 4).put(hb, 4);
            w.put((1u64 << (wb + 1)) - 1, wb as u32 + 1);
            w.put(0, hb as u32 + 1);
            w.put(0, 1); // frame ids
            w.put(0, 3).put(0, 4).put(0, 1).put(0, 2).put(0, 3); // flags
            w.put(0, 3).put(1, 1).put(0, 2).put(0, 1); // colour config
            w.put(1, 1);
            let (result, m) = parse(&w.bytes());
            result.unwrap();
            let width = range(&m, "max_frame_width_minus_1");
            let height = range(&m, "max_frame_height_minus_1");
            assert_eq!(width.end_bit - width.start_bit, wb + 1);
            assert_eq!(height.end_bit - height.start_bit, hb + 1);
            assert_eq!(
                value(&m, "max_frame_width_minus_1").unwrap(),
                ((1u64 << (wb + 1)) - 1).to_string()
            );
            assert_eq!(value(&m, "max_frame_height_minus_1").unwrap(), "0");
            assert_eq!(value(&m, "film_grain_params_present").unwrap(), "1");
        }
    }

    #[test]
    fn every_proper_prefix_of_a_full_header_is_an_error() {
        let mut w = Bits::new();
        w.put(0, 5).put(1, 1);
        w.put(1, 32).put(2, 32).put(1, 1).put(0b1, 1); // equal, uvlc(0)
        w.put(1, 1).put(0, 47);
        w.put(1, 1).put(1, 5);
        w.put(0x010, 12)
            .put(9, 5)
            .put(1, 1)
            .put(0, 1)
            .put(1, 1)
            .put(0, 4);
        w.put(0x020, 12).put(2, 5).put(0, 1).put(0, 1);
        tail_after_operating_points(&mut w);
        let full = w.bytes();
        let (result, _) = parse(&full);
        result.unwrap();
        for cut in 0..full.len() - 1 {
            let (result, _) = parse(&full[..cut]);
            assert!(
                result.is_err(),
                "a {cut}-byte prefix of {} bytes parsed",
                full.len()
            );
        }
    }

    #[test]
    fn arbitrary_bytes_never_panic() {
        let mut state = 0x9E37_79B9_7F4A_7C15u64;
        for len in 0..48usize {
            for _ in 0..200 {
                let data: Vec<u8> = (0..len)
                    .map(|_| {
                        state ^= state << 13;
                        state ^= state >> 7;
                        state ^= state << 17;
                        (state >> 24) as u8
                    })
                    .collect();
                let _ = parse(&data);
            }
        }
    }

    /// Header up to the end of the operating points: profile `profile`, one operating point.
    fn head(w: &mut Bits, profile: u64) {
        w.put(profile, 3).put(0, 1).put(0, 1);
        w.put(0, 1).put(0, 1).put(0, 5).put(0, 12).put(4, 5);
        w.put(7, 4).put(7, 4).put(0xAB, 8).put(0xCD, 8);
    }

    #[test]
    fn every_optional_feature_branch_keeps_the_bits_in_step() {
        // (frame ids, order hint, screen-content force path) in all combinations
        for frame_ids in [false, true] {
            for order_hint in [false, true] {
                for force_sct in [false, true] {
                    let mut w = Bits::new();
                    head(&mut w, 0);
                    w.put(frame_ids as u64, 1);
                    if frame_ids {
                        w.put(0b1010101, 7);
                    }
                    w.put(0b110, 3); // 128x128, filter_intra, intra_edge
                    w.put(0b0000, 4);
                    w.put(order_hint as u64, 1);
                    if order_hint {
                        w.put(0b11, 2); // enable_jnt_comp, enable_ref_frame_mvs
                    }
                    w.put(0, 1); // seq_choose_screen_content_tools = 0
                    w.put(force_sct as u64, 1); // seq_force_screen_content_tools
                    if force_sct {
                        w.put(0, 1); // seq_choose_integer_mv = 0
                        w.put(1, 1); // seq_force_integer_mv
                    }
                    if order_hint {
                        w.put(0b101, 3); // order_hint_bits_minus_1
                    }
                    w.put(0b101, 3); // superres, cdef, restoration
                    w.put(0, 3).put(1, 1).put(0b01, 2).put(0, 1); // colour config
                    w.put(1, 1);
                    let (result, m) = parse(&w.bytes());
                    result.unwrap();
                    let tag = format!(
                        "frame_ids={frame_ids} order_hint={order_hint} force_sct={force_sct}"
                    );
                    assert_eq!(
                        value(&m, "frame_id_numbers_present_flag").unwrap(),
                        (frame_ids as u8).to_string(),
                        "{tag}"
                    );
                    assert_eq!(
                        value(&m, "enable_order_hint").unwrap(),
                        (order_hint as u8).to_string(),
                        "{tag}"
                    );
                    assert_eq!(value(&m, "enable_superres").unwrap(), "1", "{tag}");
                    assert_eq!(value(&m, "enable_cdef").unwrap(), "0", "{tag}");
                    assert_eq!(value(&m, "enable_restoration").unwrap(), "1", "{tag}");
                    assert_eq!(
                        value(&m, "film_grain_params_present").unwrap(),
                        "1",
                        "{tag}"
                    );
                }
            }
        }
    }

    #[test]
    fn a_high_bitdepth_flag_alone_does_not_add_the_twelve_bit_bit() {
        // profile 0 with high_bitdepth = 1: mono_chrome follows directly
        let mut w = Bits::new();
        head(&mut w, 0);
        w.put(0, 1)
            .put(0, 3)
            .put(0, 4)
            .put(0, 1)
            .put(0, 2)
            .put(0, 3);
        w.put(1, 1).put(0, 1); // high_bitdepth = 1, mono_chrome = 0
        w.put(0, 1).put(1, 1).put(0b11, 2).put(0, 1).put(1, 1);
        let (result, m) = parse(&w.bytes());
        result.unwrap();
        assert!(m
            .get_node("o.sequence_header.color_config.twelve_bit")
            .is_none());
        assert_eq!(value(&m, "color_config.high_bitdepth").unwrap(), "1");
        assert_eq!(value(&m, "color_config.mono_chrome").unwrap(), "0");
        assert_eq!(
            value(&m, "color_config.chroma_sample_position").unwrap(),
            "3"
        );
        assert_eq!(value(&m, "film_grain_params_present").unwrap(), "1");
    }

    #[test]
    fn the_professional_profile_reads_twelve_bit_after_high_bitdepth() {
        let mut w = Bits::new();
        head(&mut w, 2);
        w.put(0, 1)
            .put(0, 3)
            .put(0, 4)
            .put(0, 1)
            .put(0, 2)
            .put(0, 3);
        w.put(1, 1).put(1, 1); // high_bitdepth = 1, twelve_bit = 1
        let (_, m) = parse(&w.bytes());
        assert_eq!(value(&m, "color_config.twelve_bit").unwrap(), "1");
        assert_eq!(value(&m, "seq_profile").unwrap(), "2 (Professional)");
    }
}
