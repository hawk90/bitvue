//! AV1 Sequence Header parsing
//!
//! Reference: AV1 Specification Section 5.5

mod color;
mod parse;
mod types;

pub use color::*;
pub use parse::parse_sequence_header;
pub use types::{Av1Profile, DecoderModelInfo, OperatingPoint, SequenceHeader, TimingInfo};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bitreader::BitReader;
    use color::parse_color_config;

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
        header_sized(
            display_delay,
            frame_ids,
            screen_content,
            integer_mv,
            (15, 15),
            0,
        )
    }

    /// [`header`] with the coded `max_frame_width_minus_1` / `max_frame_height_minus_1` (4 bits
    /// each) and `high_bitdepth` chosen.
    fn header_sized(
        display_delay: Option<u32>,
        frame_ids: Option<(u32, u32)>,
        screen_content: (u32, u32),
        integer_mv: (u32, u32),
        (max_w_minus_1, max_h_minus_1): (u32, u32),
        high_bitdepth: u32,
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
        f.extend([(3, 4), (3, 4), (max_w_minus_1, 4), (max_h_minus_1, 4)]); // frame size
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
        f.extend([(high_bitdepth, 1), (0, 1), (0, 1), (0, 1), (0, 2), (0, 1)]); // color_config
        f.push((0, 1)); // film_grain_params_present_flag
        pack(&f)
    }

    /// Like [`header`] with `initial_display_delay_present_flag` set, but the operating point's own
    /// presence bit clear: no delay value follows.
    fn header_with_clear_per_op_delay_flag() -> Vec<u8> {
        let f: Vec<(u32, u8)> = vec![
            (0, 3),  // seq_profile
            (0, 1),  // still_picture
            (0, 1),  // reduced_still_picture_header
            (0, 1),  // timing_info_present_flag
            (1, 1),  // initial_display_delay_present_flag
            (0, 5),  // operating_points_cnt_minus_1
            (0, 12), // operating_point_idc[0]
            (0, 5),  // seq_level_idx[0]
            (0, 1),  // initial_display_delay_present_for_this_op = 0
            (3, 4),
            (3, 4),
            (15, 4),
            (15, 4), // 16x16
            (0, 1),  // frame_id_numbers_present_flag
            (0, 1),
            (0, 1),
            (0, 1), // 128x128 sb, filter_intra, intra_edge_filter
            (0, 1),
            (0, 1),
            (0, 1),
            (0, 1), // interintra, masked, warped, dual filter
            (0, 1), // enable_order_hint
            (1, 1), // seq_choose_screen_content_tools
            (0, 1),
            (0, 1),
            (0, 1), // superres, cdef, restoration
            (0, 1),
            (0, 1),
            (0, 1),
            (0, 1),
            (0, 2),
            (0, 1), // color_config
            (0, 1), // film_grain_params_present_flag
        ];
        pack(&f)
    }

    #[test]
    fn an_operating_point_can_opt_out_of_the_display_delay_the_sequence_announces() {
        let seq = parse_sequence_header(&header_with_clear_per_op_delay_flag()).unwrap();
        assert_eq!(seq.operating_points[0].initial_display_delay, None);
        // No value was consumed: the frame size that follows is still 16x16.
        assert_eq!((seq.max_frame_width, seq.max_frame_height), (16, 16));
        // Same header, but with the per-op bit set, reads the 4-bit value and shifts everything.
        let with_value = parse_sequence_header(&header(Some(0), None, (1, 0), (1, 0))).unwrap();
        assert_eq!(
            with_value.operating_points[0].initial_display_delay,
            Some(1)
        );
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

    fn color_config(profile: Av1Profile, fields: &[(u32, u8)]) -> ColorConfig {
        let bytes = pack(fields);
        parse_color_config(&mut BitReader::new(&bytes), &profile).unwrap()
    }

    #[test]
    fn a_main_profile_high_bit_depth_stream_is_10_bit_420_with_its_colour_description() {
        let config = color_config(
            Av1Profile::Main,
            &[
                (1, 1),  // high_bitdepth
                (0, 1),  // mono_chrome
                (1, 1),  // color_description_present_flag
                (9, 8),  // color_primaries: BT.2020
                (16, 8), // transfer_characteristics: SMPTE ST 2084
                (9, 8),  // matrix_coefficients: BT.2020 non-constant
                (1, 1),  // color_range
                (2, 2),  // chroma_sample_position: colocated
                (1, 1),  // separate_uv_delta_q
            ],
        );
        assert_eq!(config.bit_depth, 10);
        assert_eq!(config.num_planes, 3);
        assert_eq!(config.color_primaries, ColorPrimaries::Bt2020);
        assert_eq!(
            config.transfer_characteristics,
            TransferCharacteristics::Smpte2084
        );
        assert_eq!(config.matrix_coefficients, MatrixCoefficients::Bt2020Ncl);
        assert!(config.color_range);
        assert_eq!(config.chroma_subsampling_str(), "4:2:0");
        assert_eq!(
            config.chroma_sample_position,
            ChromaSamplePosition::Colocated
        );
        assert!(config.separate_uv_delta_q);
    }

    #[test]
    fn a_main_profile_8_bit_stream_without_a_colour_description_is_unspecified() {
        let config = color_config(
            Av1Profile::Main,
            &[(0, 1), (0, 1), (0, 1), (0, 1), (1, 2), (0, 1)],
        );
        assert_eq!(config.bit_depth, 8);
        assert_eq!(config.color_primaries, ColorPrimaries::Unspecified);
        assert_eq!(
            config.transfer_characteristics,
            TransferCharacteristics::Unspecified
        );
        assert_eq!(config.matrix_coefficients, MatrixCoefficients::Unspecified);
        assert!(!config.color_range);
        assert_eq!(
            config.chroma_sample_position,
            ChromaSamplePosition::Vertical
        );
        assert!(!config.separate_uv_delta_q);
    }

    #[test]
    fn the_high_profile_has_no_mono_bit_and_is_444() {
        // high_bitdepth, (no mono_chrome bit), no colour description, color_range, separate_uv.
        let config = color_config(Av1Profile::High, &[(0, 1), (0, 1), (1, 1), (1, 1)]);
        assert!(!config.mono_chrome);
        assert_eq!(config.chroma_subsampling_str(), "4:4:4");
        assert!(config.color_range);
        assert!(config.separate_uv_delta_q);
        assert_eq!(config.chroma_sample_position, ChromaSamplePosition::Unknown);
    }

    #[test]
    fn the_professional_profile_reads_its_depth_and_subsampling_bits() {
        // 12-bit 4:2:2: high_bitdepth, twelve_bit, mono, desc, range, subsampling_x = 1, _y = 0.
        let c422 = color_config(
            Av1Profile::Professional,
            &[
                (1, 1),
                (1, 1),
                (0, 1),
                (0, 1),
                (0, 1),
                (1, 1),
                (0, 1),
                (0, 1),
            ],
        );
        assert_eq!(c422.bit_depth, 12);
        assert_eq!(c422.chroma_subsampling_str(), "4:2:2");
        assert_eq!(c422.chroma_sample_position, ChromaSamplePosition::Unknown);

        // 12-bit 4:2:0 reads chroma_sample_position, 12-bit 4:4:4 reads neither y bit.
        let c420 = color_config(
            Av1Profile::Professional,
            &[
                (1, 1),
                (1, 1),
                (0, 1),
                (0, 1),
                (0, 1),
                (1, 1),
                (1, 1),
                (1, 2),
                (0, 1),
            ],
        );
        assert_eq!(c420.chroma_subsampling_str(), "4:2:0");
        assert_eq!(c420.chroma_sample_position, ChromaSamplePosition::Vertical);
        let c444 = color_config(
            Av1Profile::Professional,
            &[(1, 1), (1, 1), (0, 1), (0, 1), (0, 1), (0, 1), (0, 1)],
        );
        assert_eq!(c444.chroma_subsampling_str(), "4:4:4");

        // Profile 2 below 12 bit is 4:2:2 without any subsampling bits.
        let ten = color_config(
            Av1Profile::Professional,
            &[(0, 1), (0, 1), (0, 1), (0, 1), (0, 1)],
        );
        // (`ten.bit_depth` is deliberately not asserted: the spec says 8 here, see #227.)
        assert_eq!(ten.chroma_subsampling_str(), "4:2:2");
        let ten_high = color_config(
            Av1Profile::Professional,
            &[
                (1, 1),
                (0, 1),
                (0, 1),
                (0, 1),
                (0, 1),
                (1, 1),
                (0, 1),
                (0, 1),
            ],
        );
        assert_eq!(ten_high.bit_depth, 10);
    }

    #[test]
    fn a_monochrome_stream_reads_only_the_color_range() {
        let config = color_config(Av1Profile::Main, &[(0, 1), (1, 1), (0, 1), (1, 1)]);
        assert!(config.mono_chrome);
        assert_eq!(config.num_planes, 1);
        assert!(config.color_range);
        assert_eq!(config.chroma_subsampling_str(), "4:0:0");
        // The spec fixes a monochrome stream's subsampling at 1 in both directions.
        assert!(config.subsampling_x && config.subsampling_y);
        assert_eq!(config.chroma_sample_position, ChromaSamplePosition::Unknown);
        assert!(!config.separate_uv_delta_q);
    }

    #[test]
    fn bt709_srgb_identity_is_full_range_444_without_range_and_subsampling_bits() {
        let config = color_config(
            Av1Profile::Main,
            &[
                (0, 1),  // high_bitdepth
                (0, 1),  // mono_chrome
                (1, 1),  // color_description_present_flag
                (1, 8),  // BT.709
                (13, 8), // sRGB
                (0, 8),  // identity
                (1, 1),  // separate_uv_delta_q (no color_range / subsampling bits before it)
            ],
        );
        assert!(config.color_range);
        assert_eq!(config.chroma_subsampling_str(), "4:4:4");
        assert!(config.separate_uv_delta_q);
    }

    #[test]
    fn chroma_subsampling_str_names_422() {
        let config = ColorConfig {
            subsampling_x: true,
            subsampling_y: false,
            ..color_config(Av1Profile::High, &[(0, 1), (0, 1), (0, 1), (0, 1)])
        };
        assert_eq!(config.chroma_subsampling_str(), "4:2:2");
    }

    /// A 16x16, 8-bit 4:2:0 sequence header whose operating section (everything between
    /// `reduced_still_picture_header` and the frame size) is spelled out by the caller.
    fn header_with_operating_section(operating_section: &[(u32, u8)]) -> Vec<u8> {
        let mut f: Vec<(u32, u8)> = vec![(0, 3), (0, 1), (0, 1)]; // profile, still, reduced
        f.extend_from_slice(operating_section);
        f.extend([(3, 4), (3, 4), (15, 4), (15, 4)]); // frame size: 4 bits each, 16x16
        f.push((0, 1)); // frame_id_numbers_present_flag
        f.extend([(0, 1), (0, 1), (0, 1)]); // 128x128 sb, filter_intra, intra_edge_filter
        f.extend([(0, 1), (0, 1), (0, 1), (0, 1)]); // interintra, masked, warped, dual filter
        f.push((0, 1)); // enable_order_hint
        f.extend([(1, 1), (1, 1)]); // seq_choose_screen_content_tools, seq_choose_integer_mv
        f.extend([(0, 1), (0, 1), (0, 1)]); // superres, cdef, restoration
        f.extend([(0, 1), (0, 1), (0, 1), (0, 1), (0, 2), (0, 1)]); // color_config
        f.push((1, 1)); // film_grain_params_present_flag
        pack(&f)
    }

    /// The fields after the operating section, as `parse_sequence_header` reports them: if the
    /// operating section consumed the wrong number of bits these come out wrong.
    fn assert_tail_is_intact(seq: &SequenceHeader) {
        assert_eq!((seq.max_frame_width, seq.max_frame_height), (16, 16));
        assert!(seq.film_grain_params_present);
    }

    #[test]
    fn timing_info_without_an_equal_picture_interval_has_no_tick_count() {
        let seq = parse_sequence_header(&header_with_operating_section(&[
            (1, 1),      // timing_info_present_flag
            (1001, 32),  // num_units_in_display_tick
            (30000, 32), // time_scale
            (0, 1),      // equal_picture_interval
            (0, 1),      // decoder_model_info_present_flag
            (0, 1),      // initial_display_delay_present_flag
            (0, 5),      // operating_points_cnt_minus_1
            (0, 12),     // operating_point_idc[0]
            (4, 5),      // seq_level_idx[0]
                         // no decoder_model_present_for_this_op: there is no decoder model info
        ]))
        .unwrap();

        let timing = seq.timing_info.as_ref().expect("timing info");
        assert_eq!(timing.num_units_in_display_tick, 1001);
        assert_eq!(timing.time_scale, 30000);
        assert!(!timing.equal_picture_interval);
        assert_eq!(timing.num_ticks_per_picture, None);
        assert!(seq.decoder_model_info.is_none());
        assert_eq!(seq.operating_points[0].seq_level_idx, 4);
        assert_tail_is_intact(&seq);
    }

    #[test]
    fn an_equal_picture_interval_reads_its_tick_count_as_uvlc_plus_one() {
        // uvlc(4): 5 = 0b101 -> two leading zeros, the 1, then the two low bits "01" = 00101.
        // uvlc(0): a single 1 bit.
        for (uvlc_bits, uvlc_width, ticks) in [(0b00101, 5, 5), (0b1, 1, 1)] {
            let seq = parse_sequence_header(&header_with_operating_section(&[
                (1, 1),
                (1, 32),
                (25, 32),
                (1, 1), // equal_picture_interval
                (uvlc_bits, uvlc_width),
                (0, 1), // decoder_model_info_present_flag
                (0, 1),
                (0, 5),
                (0, 12),
                (0, 5),
            ]))
            .unwrap();
            let timing = seq.timing_info.as_ref().unwrap();
            assert!(timing.equal_picture_interval);
            assert_eq!(timing.num_ticks_per_picture, Some(ticks));
            assert_tail_is_intact(&seq);
        }
    }

    #[test]
    fn decoder_model_info_sets_the_delay_width_of_each_operating_point_that_uses_it() {
        // buffer_delay_length_minus_1 = 7 -> 8-bit decoder / encoder buffer delays. The second
        // operating point has no decoder model, and a level above 7 (so it has a tier bit).
        let seq = parse_sequence_header(&header_with_operating_section(&[
            (1, 1),      // timing_info_present_flag
            (1, 32),     // num_units_in_display_tick
            (60, 32),    // time_scale
            (0, 1),      // equal_picture_interval
            (1, 1),      // decoder_model_info_present_flag
            (7, 5),      // buffer_delay_length_minus_1
            (12345, 32), // num_units_in_decoding_tick
            (9, 5),      // buffer_removal_time_length_minus_1
            (11, 5),     // frame_presentation_time_length_minus_1
            (0, 1),      // initial_display_delay_present_flag
            (1, 5),      // operating_points_cnt_minus_1: two operating points
            // operating point 0
            (0x0F, 12), // operating_point_idc
            (3, 5),     // seq_level_idx
            (1, 1),     // decoder_model_present_for_this_op
            (0xAA, 8),  // decoder_buffer_delay
            (0x55, 8),  // encoder_buffer_delay
            (1, 1),     // low_delay_mode_flag
            // operating point 1
            (0x103, 12), // operating_point_idc
            (9, 5),      // seq_level_idx (> 7)
            (1, 1),      // seq_tier
            (0, 1),      // decoder_model_present_for_this_op
        ]))
        .unwrap();

        let model = seq.decoder_model_info.as_ref().expect("decoder model info");
        assert_eq!(model.buffer_delay_length_minus_1, 7);
        assert_eq!(model.num_units_in_decoding_tick, 12345);
        assert_eq!(model.buffer_removal_time_length_minus_1, 9);
        assert_eq!(model.frame_presentation_time_length_minus_1, 11);

        let [op0, op1] = &seq.operating_points[..] else {
            panic!("two operating points");
        };
        assert_eq!((op0.idc, op0.seq_level_idx, op0.seq_tier), (0x0F, 3, false));
        assert!(op0.decoder_model_present);
        assert_eq!((op1.idc, op1.seq_level_idx, op1.seq_tier), (0x103, 9, true));
        assert!(!op1.decoder_model_present);
        assert_tail_is_intact(&seq);
    }

    #[test]
    fn the_tier_bit_is_present_from_level_index_8_and_not_at_7() {
        let seq = parse_sequence_header(&header_with_operating_section(&[
            (0, 1), // timing_info_present_flag
            (0, 1), // initial_display_delay_present_flag
            (2, 5), // operating_points_cnt_minus_1: three operating points
            (0x001, 12),
            (7, 5), // seq_level_idx 7: no tier bit
            (0x002, 12),
            (8, 5), // seq_level_idx 8: tier bit follows
            (1, 1),
            (0x004, 12),
            (0, 5), // seq_level_idx 0
        ]))
        .unwrap();

        let ops: Vec<_> = seq
            .operating_points
            .iter()
            .map(|op| (op.idc, op.seq_level_idx, op.seq_tier))
            .collect();
        assert_eq!(ops, [(1, 7, false), (2, 8, true), (4, 0, false)]);
        assert_tail_is_intact(&seq);
    }

    // ---- colour enums and subsampling names ----

    #[test]
    fn colour_primaries_map_every_defined_code_and_keep_the_rest_as_reserved() {
        let known = [
            (1, ColorPrimaries::Bt709),
            (2, ColorPrimaries::Unspecified),
            (4, ColorPrimaries::Bt470M),
            (5, ColorPrimaries::Bt470Bg),
            (6, ColorPrimaries::Bt601),
            (7, ColorPrimaries::Smpte240),
            (8, ColorPrimaries::GenericFilm),
            (9, ColorPrimaries::Bt2020),
            (10, ColorPrimaries::Xyz),
            (11, ColorPrimaries::Smpte431),
            (12, ColorPrimaries::Smpte432),
            (22, ColorPrimaries::Ebu3213),
        ];
        for value in 0..=255u8 {
            let expected = known
                .iter()
                .find(|(code, _)| *code == value)
                .map_or(ColorPrimaries::Reserved(value), |(_, v)| *v);
            assert_eq!(ColorPrimaries::from_u8(value), expected, "code {value}");
        }
    }

    #[test]
    fn transfer_characteristics_map_every_defined_code_and_keep_the_rest_as_reserved() {
        use TransferCharacteristics as T;
        let known = [
            (1, T::Bt709),
            (2, T::Unspecified),
            (4, T::Bt470M),
            (5, T::Bt470Bg),
            (6, T::Bt601),
            (7, T::Smpte240),
            (8, T::Linear),
            (9, T::Log100),
            (10, T::Log100Sqrt10),
            (11, T::Iec61966),
            (12, T::Bt1361),
            (13, T::Srgb),
            (14, T::Bt202010Bit),
            (15, T::Bt202012Bit),
            (16, T::Smpte2084),
            (17, T::Smpte428),
            (18, T::Hlg),
        ];
        for value in 0..=255u8 {
            let expected = known
                .iter()
                .find(|(code, _)| *code == value)
                .map_or(T::Reserved(value), |(_, v)| *v);
            assert_eq!(T::from_u8(value), expected, "code {value}");
        }
    }

    #[test]
    fn matrix_coefficients_map_every_defined_code_and_keep_the_rest_as_reserved() {
        use MatrixCoefficients as M;
        let known = [
            (0, M::Identity),
            (1, M::Bt709),
            (2, M::Unspecified),
            (4, M::Fcc),
            (5, M::Bt470Bg),
            (6, M::Bt601),
            (7, M::Smpte240),
            (8, M::YCgCo),
            (9, M::Bt2020Ncl),
            (10, M::Bt2020Cl),
            (11, M::Smpte2085),
            (12, M::ChromaDerivedNcl),
            (13, M::ChromaDerivedCl),
            (14, M::ICtCp),
        ];
        for value in 0..=255u8 {
            let expected = known
                .iter()
                .find(|(code, _)| *code == value)
                .map_or(M::Reserved(value), |(_, v)| *v);
            assert_eq!(M::from_u8(value), expected, "code {value}");
        }
    }

    #[test]
    fn chroma_sample_positions_map_0_to_2_and_everything_else_is_reserved() {
        assert_eq!(
            ChromaSamplePosition::from_u8(0),
            ChromaSamplePosition::Unknown
        );
        assert_eq!(
            ChromaSamplePosition::from_u8(1),
            ChromaSamplePosition::Vertical
        );
        assert_eq!(
            ChromaSamplePosition::from_u8(2),
            ChromaSamplePosition::Colocated
        );
        for value in 3..=255u8 {
            assert_eq!(
                ChromaSamplePosition::from_u8(value),
                ChromaSamplePosition::Reserved
            );
        }
    }

    #[test]
    fn subsampling_names_cover_every_flag_combination_and_monochrome_wins() {
        let base = color_config(Av1Profile::High, &[(0, 1), (0, 1), (0, 1), (0, 1)]);
        let name = |mono, x, y| {
            ColorConfig {
                mono_chrome: mono,
                subsampling_x: x,
                subsampling_y: y,
                ..base.clone()
            }
            .chroma_subsampling_str()
        };
        assert_eq!(name(false, true, true), "4:2:0");
        assert_eq!(name(false, true, false), "4:2:2");
        assert_eq!(name(false, false, false), "4:4:4");
        for (x, y) in [(true, true), (true, false), (false, false)] {
            assert_eq!(name(true, x, y), "4:0:0", "mono with x={x} y={y}");
        }
    }

    // ---- bit-exact reads and truncation ----

    /// Parses `fields` and checks the reader stopped exactly after the last field; every shorter
    /// byte prefix must be an error rather than a partial or defaulted config.
    fn parse_exact(profile: Av1Profile, fields: &[(u32, u8)]) -> ColorConfig {
        let bytes = pack(fields);
        let total: u64 = fields.iter().map(|&(_, width)| width as u64).sum();
        let mut reader = BitReader::new(&bytes);
        let config = parse_color_config(&mut reader, &profile).unwrap();
        assert_eq!(reader.position(), total, "bits consumed for {fields:?}");
        for len in 0..bytes.len() {
            if (len as u64) * 8 < total {
                assert!(
                    parse_color_config(&mut BitReader::new(&bytes[..len]), &profile).is_err(),
                    "{len}-byte prefix of {fields:?} should be an error"
                );
            }
        }
        config
    }

    #[test]
    fn the_srgb_shortcut_needs_all_three_of_bt709_srgb_and_identity() {
        // Full triple: no color_range and no subsampling bits, only separate_uv_delta_q follows.
        let full = [(0, 1), (0, 1), (1, 1), (1, 8), (13, 8), (0, 8)];
        let mut with_uv = full.to_vec();
        with_uv.push((0, 1));
        let config = parse_exact(Av1Profile::Main, &with_uv);
        assert!(config.color_range);
        assert_eq!(config.chroma_subsampling_str(), "4:4:4");
        assert_eq!(config.chroma_sample_position, ChromaSamplePosition::Unknown);

        // Any one of the three different: the general path reads color_range (here 0, not the
        // shortcut's forced 1), and for Main the 4:2:0 chroma_sample_position, then the uv bit.
        for (primaries, transfer, matrix) in [(2, 13, 0), (1, 14, 0), (1, 13, 1)] {
            let fields = [
                (0, 1),
                (0, 1),
                (1, 1),
                (primaries, 8),
                (transfer, 8),
                (matrix, 8),
                (0, 1), // color_range
                (0, 2), // chroma_sample_position
                (0, 1), // separate_uv_delta_q
            ];
            let config = parse_exact(Av1Profile::Main, &fields);
            assert!(!config.color_range, "{primaries}/{transfer}/{matrix}");
            assert_eq!(config.chroma_subsampling_str(), "4:2:0");
        }
    }

    #[test]
    fn a_monochrome_stream_ignores_the_srgb_shortcut_and_still_reads_color_range() {
        let fields = [
            (0, 1),
            (1, 1), // mono_chrome
            (1, 1),
            (1, 8),
            (13, 8),
            (0, 8),
            (0, 1), // color_range, read even though the triple matches
        ];
        let config = parse_exact(Av1Profile::Main, &fields);
        assert!(config.mono_chrome);
        assert!(!config.color_range);
        assert_eq!(config.chroma_subsampling_str(), "4:0:0");
    }

    #[test]
    fn a_monochrome_stream_keeps_its_colour_description_and_reads_nothing_after_color_range() {
        let fields = [(0, 1), (1, 1), (1, 1), (9, 8), (16, 8), (9, 8), (1, 1)];
        let config = parse_exact(Av1Profile::Main, &fields);
        assert_eq!(config.num_planes, 1);
        assert_eq!(config.color_primaries, ColorPrimaries::Bt2020);
        assert_eq!(
            config.transfer_characteristics,
            TransferCharacteristics::Smpte2084
        );
        assert_eq!(config.matrix_coefficients, MatrixCoefficients::Bt2020Ncl);
        assert!(config.color_range);
    }

    #[test]
    fn a_12_bit_professional_monochrome_stream_reads_the_twelve_bit_flag_then_only_color_range() {
        let config = parse_exact(
            Av1Profile::Professional,
            &[(1, 1), (1, 1), (1, 1), (0, 1), (1, 1)],
        );
        assert_eq!(config.bit_depth, 12);
        assert!(config.mono_chrome);
        assert!(config.color_range);
    }

    #[test]
    fn reserved_colour_codes_are_kept_not_rejected() {
        let fields = [
            (0, 1),
            (0, 1),
            (1, 1),
            (3, 8),
            (3, 8),
            (3, 8),
            (0, 1),
            (0, 2),
            (0, 1),
        ];
        let config = parse_exact(Av1Profile::Main, &fields);
        assert_eq!(config.color_primaries, ColorPrimaries::Reserved(3));
        assert_eq!(
            config.transfer_characteristics,
            TransferCharacteristics::Reserved(3)
        );
        assert_eq!(config.matrix_coefficients, MatrixCoefficients::Reserved(3));
        // Chroma sample position 3 is reserved but still read as two bits.
        let fields = [(0, 1), (0, 1), (0, 1), (0, 1), (3, 2), (0, 1)];
        let config = parse_exact(Av1Profile::Main, &fields);
        assert_eq!(
            config.chroma_sample_position,
            ChromaSamplePosition::Reserved
        );
    }

    #[test]
    fn every_profile_and_layout_reads_exactly_the_bits_the_spec_lists() {
        // Main 8-bit 4:2:0 with description.
        parse_exact(
            Av1Profile::Main,
            &[
                (0, 1),
                (0, 1),
                (1, 1),
                (6, 8),
                (6, 8),
                (6, 8),
                (0, 1),
                (1, 2),
                (1, 1),
            ],
        );
        // High: no mono bit, no subsampling, no chroma position; high_bitdepth gives 10-bit.
        let high = parse_exact(Av1Profile::High, &[(1, 1), (0, 1), (0, 1), (0, 1)]);
        assert_eq!(high.bit_depth, 10);
        assert_eq!(high.chroma_subsampling_str(), "4:4:4");
        // High with the sRGB triple: the shortcut skips color_range.
        let high_srgb = parse_exact(
            Av1Profile::High,
            &[(0, 1), (1, 1), (1, 8), (13, 8), (0, 8), (1, 1)],
        );
        assert!(high_srgb.color_range);
        assert!(high_srgb.separate_uv_delta_q);
        // Professional 12-bit: 4:2:2 (x=1, y=0) reads two subsampling bits and no position.
        parse_exact(
            Av1Profile::Professional,
            &[
                (1, 1),
                (1, 1),
                (0, 1),
                (0, 1),
                (0, 1),
                (1, 1),
                (0, 1),
                (1, 1),
            ],
        );
        // Professional 12-bit 4:4:4 (x=0) skips the y bit.
        parse_exact(
            Av1Profile::Professional,
            &[(1, 1), (1, 1), (0, 1), (0, 1), (0, 1), (0, 1), (1, 1)],
        );
        // Professional 12-bit 4:2:0 reads the chroma sample position.
        parse_exact(
            Av1Profile::Professional,
            &[
                (1, 1),
                (1, 1),
                (0, 1),
                (0, 1),
                (0, 1),
                (1, 1),
                (1, 1),
                (2, 2),
                (0, 1),
            ],
        );
    }

    #[test]
    fn random_bytes_never_panic_for_any_profile() {
        let mut state = 0x2545_f491_4f6c_dd1du64;
        for round in 0..4000 {
            let mut bytes = [0u8; 8];
            for byte in &mut bytes {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                *byte = state as u8;
            }
            let profile = match round % 4 {
                0 => Av1Profile::Main,
                1 => Av1Profile::High,
                2 => Av1Profile::Professional,
                _ => Av1Profile::Reserved(3),
            };
            let len = (state % 9) as usize;
            let _ = parse_color_config(&mut BitReader::new(&bytes[..len]), &profile);
        }
    }

    #[test]
    fn the_444_layouts_have_both_subsampling_flags_false() {
        // The name "4:4:4" also covers x=false,y=true, so check the flags themselves.
        let srgb = parse_exact(
            Av1Profile::Main,
            &[(0, 1), (0, 1), (1, 1), (1, 8), (13, 8), (0, 8), (0, 1)],
        );
        assert!(!srgb.subsampling_x && !srgb.subsampling_y);
        let professional_444 = parse_exact(
            Av1Profile::Professional,
            &[(1, 1), (1, 1), (0, 1), (0, 1), (0, 1), (0, 1), (1, 1)],
        );
        assert!(!professional_444.subsampling_x && !professional_444.subsampling_y);
        let high = parse_exact(Av1Profile::High, &[(0, 1), (0, 1), (0, 1), (0, 1)]);
        assert!(!high.subsampling_x && !high.subsampling_y);
        let main = parse_exact(
            Av1Profile::Main,
            &[(0, 1), (0, 1), (0, 1), (0, 1), (0, 2), (0, 1)],
        );
        assert!(main.subsampling_x && main.subsampling_y);
        let mono = parse_exact(Av1Profile::Main, &[(0, 1), (1, 1), (0, 1), (0, 1)]);
        assert!(mono.subsampling_x && mono.subsampling_y);
    }

    #[test]
    fn the_header_accessors_report_the_parsed_fields() {
        let seq = parse_sequence_header(&header(None, None, (1, 0), (1, 0))).unwrap();
        assert_eq!((seq.width(), seq.height(), seq.bit_depth()), (16, 16, 8));

        // Distinct width and height, and a 10-bit stream.
        let wide = header_sized(None, None, (1, 0), (1, 0), (11, 5), 1);
        let wide = parse_sequence_header(&wide).unwrap();
        assert_eq!((wide.width(), wide.height()), (12, 6));
        assert_eq!(wide.bit_depth(), 10);
        assert_eq!(seq.level(), seq.operating_points[0].seq_level_idx);
        assert_eq!(seq.level(), 0, "the header was built with seq_level_idx 0");

        // The level is the first operating point's; without any it falls back to 0.
        let mut none = seq.clone();
        none.operating_points.clear();
        assert_eq!(none.level(), 0);
        let mut two = seq.clone();
        let mut second = two.operating_points[0].clone();
        two.operating_points[0].seq_level_idx = 5;
        second.seq_level_idx = 9;
        two.operating_points.push(second);
        assert_eq!(two.level(), 5, "operating point 0 decides");
    }
}
