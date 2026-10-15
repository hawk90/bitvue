//! `film_grain_params()` (spec 5.9.30).

use super::*;

pub(super) fn parse_film_grain_params(
    reader: &mut BitReader,
    seq: &SequenceHeader,
    show_frame: bool,
    showable_frame: bool,
    frame_type: FrameType,
) -> Result<FilmGrainInfo> {
    if !seq.film_grain_params_present || (!show_frame && !showable_frame) {
        return Ok(FilmGrainInfo::default());
    }
    let apply_grain = reader.read_bit()?;
    if !apply_grain {
        return Ok(FilmGrainInfo::default());
    }
    let seed = reader.read_bits(16)? as u64;
    let update_grain = if frame_type == FrameType::Inter {
        reader.read_bit()?
    } else {
        true
    };
    if !update_grain {
        reader.read_bits(3)?; // film_grain_params_ref_idx -- values would come from a loaded
                              // reference's params, which this parser doesn't track (out of
                              // scope: same "no cross-frame film-grain state" limitation as
                              // PrevGmParams, but here it affects a real value, not just bit
                              // consumption -- flagged, not silently guessed).
        return Ok(FilmGrainInfo {
            enabled: true,
            seed,
            ..Default::default()
        });
    }

    let num_y_points = reader.read_bits(4)?;
    for _ in 0..num_y_points {
        reader.read_bits(8)?; // point_y_value
        reader.read_bits(8)?; // point_y_scaling
    }
    let (chroma_scaling_from_luma, num_cb_points, num_cr_points) =
        read_chroma_scaling_points(reader, seq, num_y_points)?;
    let grain_scaling_shift = (reader.read_bits(2)? + 8) as u8;
    let ar_coeff_lag = reader.read_bits(2)? as u8;
    let (ar_coeffs_y, ar_coeffs_uv) = read_ar_coeffs(
        reader,
        ar_coeff_lag,
        num_y_points,
        chroma_scaling_from_luma,
        num_cb_points,
        num_cr_points,
    )?;
    let ar_coeff_shift = (reader.read_bits(2)? + 6) as u8;
    let grain_scale_shift = reader.read_bits(2)? as u8;
    if num_cb_points > 0 {
        reader.read_bits(8)?; // cb_mult
        reader.read_bits(8)?; // cb_luma_mult
        reader.read_bits(9)?; // cb_offset
    }
    if num_cr_points > 0 {
        reader.read_bits(8)?; // cr_mult
        reader.read_bits(8)?; // cr_luma_mult
        reader.read_bits(9)?; // cr_offset
    }
    let overlap = reader.read_bit()?;
    let clip_to_restricted_range = reader.read_bit()?;

    Ok(FilmGrainInfo {
        enabled: true,
        update_offset: 0, // AV1 spec has no such field; FilmGrainInfo's field predates this
        // parser and appears unused by advanced_features::extract_film_grain_data
        // (only reads enabled/seed/scaling_shift/ar_coeff_lag/chroma_scaling_from_luma/overlap).
        seed,
        scaling_shift: grain_scaling_shift,
        ar_coeff_lag,
        ar_coeffs_y,
        ar_coeffs_uv,
        ar_coeff_shift,
        grain_scale_shift,
        chroma_scaling_from_luma,
        overlap,
        clip_to_restricted_range,
    })
}

/// `chroma_scaling_from_luma` and the Cb / Cr scaling points. Returns
/// `(chroma_scaling_from_luma, num_cb_points, num_cr_points)`.
fn read_chroma_scaling_points(
    reader: &mut BitReader,
    seq: &SequenceHeader,
    num_y_points: u32,
) -> Result<(bool, u32, u32)> {
    let chroma_scaling_from_luma = if seq.color_config.mono_chrome {
        false
    } else {
        reader.read_bit()?
    };
    let (num_cb_points, num_cr_points) = if seq.color_config.mono_chrome
        || chroma_scaling_from_luma
        || (seq.color_config.subsampling_x && seq.color_config.subsampling_y && num_y_points == 0)
    {
        (0, 0)
    } else {
        let cb = reader.read_bits(4)?;
        for _ in 0..cb {
            reader.read_bits(8)?;
            reader.read_bits(8)?;
        }
        let cr = reader.read_bits(4)?;
        for _ in 0..cr {
            reader.read_bits(8)?;
            reader.read_bits(8)?;
        }
        (cb, cr)
    };
    Ok((chroma_scaling_from_luma, num_cb_points, num_cr_points))
}

/// The auto-regressive coefficients. Returns `(ar_coeffs_y, ar_coeffs_uv)`; the Cr values are read
/// but not kept.
fn read_ar_coeffs(
    reader: &mut BitReader,
    ar_coeff_lag: u8,
    num_y_points: u32,
    chroma_scaling_from_luma: bool,
    num_cb_points: u32,
    num_cr_points: u32,
) -> Result<(Vec<i8>, Vec<i8>)> {
    let num_pos_luma = 2 * ar_coeff_lag as u32 * (ar_coeff_lag as u32 + 1);
    let num_pos_chroma = if num_y_points > 0 {
        num_pos_luma + 1
    } else {
        num_pos_luma
    };
    let mut ar_coeffs_y = Vec::new();
    if num_y_points > 0 {
        ar_coeffs_y.reserve(num_pos_luma as usize);
        for _ in 0..num_pos_luma {
            let raw = reader.read_bits(8)?;
            ar_coeffs_y.push((raw as i32 - 128) as i8);
        }
    }
    let mut ar_coeffs_uv = Vec::new();
    if chroma_scaling_from_luma || num_cb_points > 0 {
        for _ in 0..num_pos_chroma {
            let raw = reader.read_bits(8)?;
            ar_coeffs_uv.push((raw as i32 - 128) as i8);
        }
    }
    if chroma_scaling_from_luma || num_cr_points > 0 {
        for _ in 0..num_pos_chroma {
            reader.read_bits(8)?; // ar_coeffs_cr -- appended separately from ar_coeffs_uv's Cb
                                  // values in the spec; FilmGrainInfo has one combined
                                  // ar_coeffs_uv field, so Cr values aren't separately kept
                                  // (matches the existing FilmGrainInfo shape, not a new gap).
        }
    }
    Ok((ar_coeffs_y, ar_coeffs_uv))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Bits MSB first. Tests append a recognisable 8-bit tail after the bits the parser should
    /// consume, so a parser that stops early or late shows up as a wrong read position.
    #[derive(Default)]
    struct Bits(Vec<u8>);

    impl Bits {
        fn put(&mut self, value: u32, width: u8) -> &mut Self {
            for i in (0..width).rev() {
                self.0.push(((value >> i) & 1) as u8);
            }
            self
        }

        fn bytes(&self) -> Vec<u8> {
            let mut out = vec![0u8; self.0.len().div_ceil(8)];
            for (i, bit) in self.0.iter().enumerate() {
                if *bit != 0 {
                    out[i / 8] |= 1 << (7 - i % 8);
                }
            }
            out
        }

        fn len(&self) -> u64 {
            self.0.len() as u64
        }
    }

    fn seq(mono: bool, subsampling: (bool, bool)) -> SequenceHeader {
        let mut s = super::super::tests::base_seq();
        s.film_grain_params_present = true;
        s.color_config.mono_chrome = mono;
        s.color_config.subsampling_x = subsampling.0;
        s.color_config.subsampling_y = subsampling.1;
        s
    }

    fn yuv420() -> SequenceHeader {
        seq(false, (true, true))
    }

    /// Parses `bits` and returns the result plus how many bits were consumed.
    fn parse(
        seq: &SequenceHeader,
        bits: &Bits,
        show: bool,
        showable: bool,
        frame_type: FrameType,
    ) -> (FilmGrainInfo, u64) {
        let bytes = bits.bytes();
        let mut reader = BitReader::new(&bytes);
        let info = parse_film_grain_params(&mut reader, seq, show, showable, frame_type).unwrap();
        (info, reader.position())
    }

    #[test]
    fn nothing_is_read_unless_the_sequence_has_grain_and_the_frame_is_shown_or_showable() {
        let mut b = Bits::default();
        b.put(1, 1).put(0xAB, 8);

        let mut no_grain = yuv420();
        no_grain.film_grain_params_present = false;
        let (info, pos) = parse(&no_grain, &b, true, true, FrameType::Key);
        assert!(!info.enabled);
        assert_eq!(pos, 0);

        let (info, pos) = parse(&yuv420(), &b, false, false, FrameType::Key);
        assert!(!info.enabled);
        assert_eq!(pos, 0);

        // Either flag alone is enough to read `apply_grain`.
        for (show, showable) in [(true, false), (false, true)] {
            let (_, pos) = parse(
                &yuv420(),
                &Bits(vec![0, 0, 0]),
                show,
                showable,
                FrameType::Key,
            );
            assert_eq!(pos, 1, "show={show} showable={showable}");
        }
    }

    #[test]
    fn apply_grain_zero_stops_after_one_bit() {
        let mut b = Bits::default();
        b.put(0, 1).put(0xFF, 8);
        let (info, pos) = parse(&yuv420(), &b, true, true, FrameType::Key);
        assert!(!info.enabled);
        assert_eq!(pos, 1);
    }

    /// Key frame, 4:2:0, lag 1: every field has a different value, so a swapped or mis-sized
    /// read changes something visible.
    #[test]
    fn a_full_key_frame_header_is_read_field_by_field() {
        let mut b = Bits::default();
        b.put(1, 1); // apply_grain
        b.put(0xBEEF, 16); // grain_seed
                           // key frame: no update_grain bit
        b.put(2, 4); // num_y_points
        b.put(10, 8).put(20, 8).put(200, 8).put(90, 8); // point_y_value / point_y_scaling
        b.put(0, 1); // chroma_scaling_from_luma
        b.put(1, 4).put(50, 8).put(60, 8); // num_cb_points, one point
        b.put(2, 4).put(70, 8).put(80, 8).put(90, 8).put(100, 8); // num_cr_points, two points
        b.put(3, 2); // grain_scaling_shift_minus_8
        b.put(1, 2); // ar_coeff_lag
        for v in [130u32, 120, 140, 100] {
            b.put(v, 8); // ar_coeffs_y_plus_128 (4 for lag 1)
        }
        for v in [129u32, 127, 131, 125, 133] {
            b.put(v, 8); // ar_coeffs_cb_plus_128 (4 + 1)
        }
        for _ in 0..5 {
            b.put(128, 8); // ar_coeffs_cr_plus_128
        }
        b.put(2, 2); // ar_coeff_shift_minus_6
        b.put(3, 2); // grain_scale_shift
        b.put(1, 8).put(2, 8).put(3, 9); // cb_mult, cb_luma_mult, cb_offset
        b.put(4, 8).put(5, 8).put(6, 9); // cr_mult, cr_luma_mult, cr_offset
        b.put(1, 1); // overlap_flag
        b.put(0, 1); // clip_to_restricted_range
        let used = b.len();
        b.put(0xA5, 8);

        let (info, pos) = parse(&yuv420(), &b, true, true, FrameType::Key);
        assert_eq!(pos, used);
        assert!(info.enabled);
        assert_eq!(info.seed, 0xBEEF);
        assert_eq!(info.scaling_shift, 11);
        assert_eq!(info.ar_coeff_lag, 1);
        assert_eq!(info.ar_coeffs_y, vec![2, -8, 12, -28]);
        assert_eq!(info.ar_coeffs_uv, vec![1, -1, 3, -3, 5]);
        assert_eq!(info.ar_coeff_shift, 8);
        assert_eq!(info.grain_scale_shift, 3);
        assert!(!info.chroma_scaling_from_luma);
        assert!(info.overlap);
        assert!(!info.clip_to_restricted_range);
    }

    #[test]
    fn an_inter_frame_without_update_grain_reads_only_a_reference_index() {
        let mut b = Bits::default();
        b.put(1, 1).put(0x1234, 16);
        b.put(0, 1); // update_grain = 0
        b.put(5, 3); // film_grain_params_ref_idx
        let used = b.len();
        b.put(0xFF, 8);

        let (info, pos) = parse(&yuv420(), &b, true, true, FrameType::Inter);
        assert_eq!(pos, used);
        assert!(info.enabled);
        assert_eq!(info.seed, 0x1234);
        assert_eq!(info.ar_coeff_lag, 0);
        assert!(info.ar_coeffs_y.is_empty());
    }

    #[test]
    fn an_inter_frame_with_update_grain_reads_the_parameters() {
        let mut b = Bits::default();
        b.put(1, 1).put(7, 16);
        b.put(1, 1); // update_grain = 1
        b.put(0, 4); // num_y_points
        b.put(0, 1); // chroma_scaling_from_luma (4:2:0, no luma points: no chroma points)
        b.put(1, 2); // grain_scaling_shift_minus_8
        b.put(0, 2); // ar_coeff_lag
        b.put(0, 2).put(0, 2); // ar_coeff_shift_minus_6, grain_scale_shift
        b.put(0, 1).put(0, 1); // overlap_flag, clip_to_restricted_range
        let used = b.len();
        b.put(0xFF, 8);

        let (info, pos) = parse(&yuv420(), &b, true, true, FrameType::Inter);
        assert_eq!(pos, used);
        assert_eq!(info.scaling_shift, 9);
    }

    #[test]
    fn a_key_frame_and_an_intra_only_frame_have_no_update_grain_bit() {
        for frame_type in [FrameType::Key, FrameType::IntraOnly] {
            let mut b = Bits::default();
            b.put(1, 1).put(7, 16);
            b.put(0, 4); // num_y_points
            b.put(0, 1)
                .put(0, 2)
                .put(0, 2)
                .put(0, 2)
                .put(0, 2)
                .put(0, 1)
                .put(0, 1);
            let used = b.len();
            b.put(0xFF, 8);
            let (_, pos) = parse(&yuv420(), &b, true, true, frame_type);
            assert_eq!(pos, used, "{frame_type:?}");
        }
    }

    #[test]
    fn monochrome_has_no_chroma_scaling_points_coefficients_or_offsets() {
        let mut b = Bits::default();
        b.put(1, 1).put(1, 16);
        b.put(1, 4).put(0, 8).put(255, 8); // one luma point
                                           // no chroma_scaling_from_luma bit and no chroma points
        b.put(0, 2); // grain_scaling_shift_minus_8
        b.put(1, 2); // ar_coeff_lag
        for _ in 0..4 {
            b.put(128, 8); // ar_coeffs_y
        }
        b.put(0, 2).put(0, 2); // ar_coeff_shift_minus_6, grain_scale_shift
        b.put(0, 1).put(0, 1); // overlap_flag, clip_to_restricted_range
        let used = b.len();
        b.put(0xFF, 8);

        let (info, pos) = parse(&seq(true, (true, true)), &b, true, true, FrameType::Key);
        assert_eq!(pos, used);
        assert!(!info.chroma_scaling_from_luma);
        assert_eq!(info.ar_coeffs_y.len(), 4);
        assert!(info.ar_coeffs_uv.is_empty());
    }

    #[test]
    fn subsampled_chroma_without_luma_points_reads_no_chroma_points() {
        // 4:2:0 and num_y_points == 0: chroma_scaling_from_luma is read (1), the point counts are
        // not, and each chroma coefficient list has num_pos_luma entries (no luma term).
        let mut b = Bits::default();
        b.put(1, 1).put(1, 16);
        b.put(0, 4); // num_y_points
        b.put(1, 1); // chroma_scaling_from_luma
        b.put(0, 2); // grain_scaling_shift_minus_8
        b.put(1, 2); // ar_coeff_lag -> num_pos_luma = 4
        for v in [129u32, 130, 131, 132] {
            b.put(v, 8); // ar_coeffs_cb_plus_128
        }
        for _ in 0..4 {
            b.put(128, 8); // ar_coeffs_cr_plus_128
        }
        b.put(0, 2).put(0, 2);
        b.put(0, 1).put(0, 1);
        let used = b.len();
        b.put(0xFF, 8);

        let (info, pos) = parse(&yuv420(), &b, true, true, FrameType::Key);
        assert_eq!(pos, used);
        assert!(info.chroma_scaling_from_luma);
        assert!(info.ar_coeffs_y.is_empty());
        assert_eq!(info.ar_coeffs_uv, vec![1, 2, 3, 4]);
    }

    #[test]
    fn full_resolution_chroma_reads_chroma_points_even_without_luma_points() {
        let mut b = Bits::default();
        b.put(1, 1).put(1, 16);
        b.put(0, 4); // num_y_points
        b.put(0, 1); // chroma_scaling_from_luma = 0
        b.put(1, 4).put(1, 8).put(2, 8); // num_cb_points = 1
        b.put(0, 4); // num_cr_points = 0
        b.put(0, 2); // grain_scaling_shift_minus_8
        b.put(0, 2); // ar_coeff_lag 0: num_pos_luma = 0, no luma term, so the Cb list is empty
        b.put(0, 2).put(0, 2); // ar_coeff_shift_minus_6, grain_scale_shift
        b.put(9, 8).put(8, 8).put(7, 9); // cb_mult, cb_luma_mult, cb_offset
                                         // no Cr offsets: num_cr_points = 0
        b.put(0, 1).put(0, 1);
        let used = b.len();
        b.put(0xFF, 8);

        let (_, pos) = parse(&seq(false, (false, false)), &b, true, true, FrameType::Key);
        assert_eq!(pos, used);
    }

    #[test]
    fn chroma_scaling_from_luma_skips_points_and_offsets_but_reads_both_coefficient_lists() {
        let mut b = Bits::default();
        b.put(1, 1).put(1, 16);
        b.put(1, 4).put(0, 8).put(64, 8); // one luma point
        b.put(1, 1); // chroma_scaling_from_luma
        b.put(0, 2); // grain_scaling_shift_minus_8
        b.put(1, 2); // ar_coeff_lag
        for _ in 0..4 {
            b.put(128, 8); // ar_coeffs_y
        }
        for _ in 0..5 {
            b.put(128, 8); // ar_coeffs_cb
        }
        for _ in 0..5 {
            b.put(128, 8); // ar_coeffs_cr
        }
        b.put(0, 2).put(0, 2);
        // no offsets: both point counts are 0
        b.put(1, 1).put(1, 1); // overlap_flag, clip_to_restricted_range
        let used = b.len();
        b.put(0xFF, 8);

        let (info, pos) = parse(&yuv420(), &b, true, true, FrameType::Key);
        assert_eq!(pos, used);
        assert!(info.overlap);
        assert!(info.clip_to_restricted_range);
        assert_eq!(info.ar_coeffs_uv.len(), 5);
    }

    #[test]
    fn the_coefficient_count_follows_the_lag() {
        // num_pos_luma = 2 * lag * (lag + 1): 0, 4, 12, 24. One luma point and chroma from luma,
        // so all three lists are present (the chroma ones have one more entry).
        for (lag, num_pos) in [(0u32, 0u32), (1, 4), (2, 12), (3, 24)] {
            let mut b = Bits::default();
            b.put(1, 1).put(1, 16);
            b.put(1, 4).put(0, 8).put(64, 8);
            b.put(1, 1);
            b.put(0, 2);
            b.put(lag, 2);
            for i in 0..(num_pos + 2 * (num_pos + 1)) {
                b.put(128 + (i % 100), 8);
            }
            b.put(0, 2).put(0, 2);
            b.put(0, 1).put(0, 1);
            let used = b.len();
            b.put(0xFF, 8);

            let (info, pos) = parse(&yuv420(), &b, true, true, FrameType::Key);
            assert_eq!(pos, used, "lag {lag}");
            assert_eq!(info.ar_coeffs_y.len() as u32, num_pos, "lag {lag}");
            assert_eq!(info.ar_coeffs_uv.len() as u32, num_pos + 1, "lag {lag}");
        }
    }

    #[test]
    fn cr_coefficients_and_offsets_depend_on_the_cr_points_alone() {
        // Cb has no points, Cr has one: the Cb list is absent, the Cr list and the Cr offsets
        // are present, the Cb offsets are not.
        let mut b = Bits::default();
        b.put(1, 1).put(1, 16);
        b.put(1, 4).put(0, 8).put(64, 8); // one luma point
        b.put(0, 1); // chroma_scaling_from_luma
        b.put(0, 4); // num_cb_points
        b.put(1, 4).put(3, 8).put(4, 8); // num_cr_points
        b.put(0, 2); // grain_scaling_shift_minus_8
        b.put(1, 2); // ar_coeff_lag
        for _ in 0..4 {
            b.put(128, 8); // ar_coeffs_y
        }
        for v in [140u32, 141, 142, 143, 144] {
            b.put(v, 8); // ar_coeffs_cr_plus_128 (no Cb list)
        }
        b.put(0, 2).put(0, 2);
        b.put(1, 8).put(2, 8).put(3, 9); // cr_mult, cr_luma_mult, cr_offset
        b.put(0, 1).put(0, 1);
        let used = b.len();
        b.put(0xFF, 8);

        let (info, pos) = parse(&yuv420(), &b, true, true, FrameType::Key);
        assert_eq!(pos, used);
        assert!(info.ar_coeffs_uv.is_empty(), "Cb list is absent");
    }

    /// Chroma points are skipped for 4:2:0 without luma points only; every other subsampling
    /// (and mono, and chroma-from-luma) decides separately.
    #[test]
    fn without_luma_points_only_4_2_0_skips_the_chroma_point_counts() {
        for (sub, reads_counts) in [
            ((true, true), false),
            ((false, false), true),
            ((true, false), true),
            ((false, true), true),
        ] {
            let mut b = Bits::default();
            b.put(1, 1).put(9, 16); // apply_grain, seed (key frame: no update_grain)
            b.put(0, 4); // num_y_points
            b.put(0, 1); // chroma_scaling_from_luma
            if reads_counts {
                b.put(1, 4).put(40, 8).put(50, 8); // num_cb_points, one point
                b.put(2, 4).put(1, 8).put(2, 8).put(3, 8).put(4, 8); // num_cr_points, two
            }
            b.put(1, 2); // grain_scaling_shift_minus_8
            b.put(1, 2); // ar_coeff_lag 1: 4 positions, none for luma (no luma points)
            if reads_counts {
                for v in [130u32, 126, 131, 127] {
                    b.put(v, 8); // ar_coeffs_cb_plus_128
                }
                for _ in 0..4 {
                    b.put(128, 8); // ar_coeffs_cr_plus_128
                }
            }
            b.put(0, 2).put(0, 2); // ar_coeff_shift_minus_6, grain_scale_shift
            if reads_counts {
                b.put(1, 8).put(2, 8).put(3, 9); // cb_mult, cb_luma_mult, cb_offset
                b.put(4, 8).put(5, 8).put(6, 9); // cr_mult, cr_luma_mult, cr_offset
            }
            b.put(1, 1).put(1, 1); // overlap, clip_to_restricted_range
            let used = b.len();
            b.put(0xA5, 8);
            let (info, pos) = parse(&seq(false, sub), &b, true, true, FrameType::Key);
            assert_eq!(pos, used, "subsampling {sub:?}");
            assert!(info.overlap && info.clip_to_restricted_range, "{sub:?}");
            assert_eq!(info.scaling_shift, 9, "{sub:?}");
            assert!(info.ar_coeffs_y.is_empty(), "{sub:?}");
            let expected_uv: Vec<i8> = if reads_counts {
                vec![2, -2, 3, -1]
            } else {
                vec![]
            };
            assert_eq!(info.ar_coeffs_uv, expected_uv, "{sub:?}");
        }
    }
}
