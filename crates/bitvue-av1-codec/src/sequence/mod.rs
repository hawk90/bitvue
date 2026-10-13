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

        // 10-bit professional streams are 4:2:2 without any subsampling bits; twelve_bit = 0
        // also means 10-bit.
        let ten = color_config(
            Av1Profile::Professional,
            &[(0, 1), (0, 1), (0, 1), (0, 1), (0, 1)],
        );
        assert_eq!(ten.bit_depth, 10);
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
}
