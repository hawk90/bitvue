//! Splits an Annex B H.264 / HEVC byte stream into access units (one coded picture each).
//!
//! libavcodec's H.264 and HEVC decoders (used without a parser) take one packet to be one access
//! unit, so a whole file sent as a single packet decodes one picture (#402). This module finds the
//! boundaries the way a stream parser does, from the NAL unit headers alone:
//!
//! - a NAL unit that can only open an access unit (parameter sets, SEI, delimiter) after the
//!   picture's slices starts the next one;
//! - a slice whose "first slice of the picture" marker is set (`first_mb_in_slice == 0` for H.264,
//!   `first_slice_segment_in_pic_flag` for HEVC) starts the next one.

/// Which NAL unit header layout to read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnnexBCodec {
    H264,
    Hevc,
}

/// What a NAL unit means for access unit boundaries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NalRole {
    /// A slice that is the first of its picture.
    FirstSlice,
    /// A slice that continues the picture.
    LaterSlice,
    /// A NAL unit that precedes the slices of its access unit.
    LeadingNonVcl,
    /// Anything else (suffix SEI, end of sequence, ...): stays with the current access unit.
    Other,
}

fn classify(codec: AnnexBCodec, nal: &[u8]) -> NalRole {
    let Some(&b0) = nal.first() else {
        return NalRole::Other;
    };
    match codec {
        AnnexBCodec::H264 => match b0 & 0x1F {
            // First macroblock 0 is ue(v) = 1, the top bit of the byte after the header.
            1..=5 if nal.get(1).is_some_and(|b| b & 0x80 != 0) => NalRole::FirstSlice,
            1..=5 => NalRole::LaterSlice,
            // SEI, SPS, PPS, access unit delimiter, prefix NAL, subset SPS.
            6..=9 | 14 | 15 => NalRole::LeadingNonVcl,
            _ => NalRole::Other,
        },
        AnnexBCodec::Hevc => match (b0 >> 1) & 0x3F {
            // first_slice_segment_in_pic_flag is the top bit of the byte after the 2-byte header.
            0..=31 if nal.get(2).is_some_and(|b| b & 0x80 != 0) => NalRole::FirstSlice,
            0..=31 => NalRole::LaterSlice,
            // VPS, SPS, PPS, access unit delimiter, prefix SEI, reserved 41..=44.
            32..=35 | 39 | 41..=44 => NalRole::LeadingNonVcl,
            _ => NalRole::Other,
        },
    }
}

/// `(offset of the start code, offset of the NAL unit)` for every NAL unit, in order. A 3-byte
/// start code preceded by a zero byte counts as the 4-byte form.
fn nal_offsets(data: &[u8]) -> Vec<(usize, usize)> {
    let mut found = Vec::new();
    let mut i = 0;
    while i + 3 <= data.len() {
        if data[i] == 0 && data[i + 1] == 0 && data[i + 2] == 1 {
            let start = if i > 0 && data[i - 1] == 0 { i - 1 } else { i };
            found.push((start, i + 3));
            i += 3;
        } else {
            i += 1;
        }
    }
    found
}

/// Splits `data` into access units. The pieces are consecutive, cover all of `data`, and each
/// begins at a start code (bytes before the first start code stay with the first piece). Data
/// with no start code, or with a single access unit, comes back whole.
pub fn split_access_units(codec: AnnexBCodec, data: &[u8]) -> Vec<&[u8]> {
    let nals = nal_offsets(data);
    let mut units = Vec::new();
    let mut unit_start = 0;
    let mut has_slice = false;

    for (idx, &(start_code, nal_start)) in nals.iter().enumerate() {
        let end = nals.get(idx + 1).map_or(data.len(), |&(next, _)| next);
        let role = classify(codec, &data[nal_start..end]);
        let opens_unit = has_slice && matches!(role, NalRole::FirstSlice | NalRole::LeadingNonVcl);
        if opens_unit {
            units.push(&data[unit_start..start_code]);
            unit_start = start_code;
            has_slice = false;
        }
        if matches!(role, NalRole::FirstSlice | NalRole::LaterSlice) {
            has_slice = true;
        }
    }
    units.push(&data[unit_start..]);
    units
}

#[cfg(test)]
mod tests {
    use super::*;

    const SC4: [u8; 4] = [0, 0, 0, 1];
    const SC3: [u8; 3] = [0, 0, 1];

    fn cat(parts: &[&[u8]]) -> Vec<u8> {
        parts.concat()
    }

    // H.264 NAL units (header byte then payload).
    const AUD: [u8; 2] = [0x09, 0xF0];
    const SPS: [u8; 3] = [0x67, 0x42, 0x00];
    const PPS: [u8; 2] = [0x68, 0xCE];
    const SEI: [u8; 2] = [0x06, 0x05];
    const IDR_FIRST: [u8; 3] = [0x65, 0x88, 0x84]; // first_mb_in_slice = 0
    const P_FIRST: [u8; 3] = [0x41, 0x9A, 0x10];
    const P_SECOND_SLICE: [u8; 3] = [0x41, 0x40, 0x10]; // first_mb_in_slice != 0

    #[test]
    fn h264_parameter_sets_open_the_next_unit_and_a_first_slice_closes_the_previous_one() {
        let data = cat(&[
            &SC4, &SPS, &SC4, &PPS, &SC4, &IDR_FIRST, // unit 0
            &SC4, &P_FIRST, // unit 1
            &SC3, &P_FIRST, // unit 2
        ]);
        let units = split_access_units(AnnexBCodec::H264, &data);
        assert_eq!(units.len(), 3);
        assert_eq!(units.concat(), data);
        assert_eq!(units[0], cat(&[&SC4, &SPS, &SC4, &PPS, &SC4, &IDR_FIRST]));
        assert_eq!(units[1], cat(&[&SC4, &P_FIRST]));
        assert_eq!(units[2], cat(&[&SC3, &P_FIRST]));
    }

    #[test]
    fn h264_delimiter_and_sei_start_a_unit_and_later_slices_stay_in_theirs() {
        let data = cat(&[
            &SC4,
            &AUD,
            &SC4,
            &IDR_FIRST,
            &SC4,
            &P_SECOND_SLICE, // unit 0, two slices
            &SC4,
            &AUD,
            &SC4,
            &SEI,
            &SC4,
            &P_FIRST, // unit 1
        ]);
        let units = split_access_units(AnnexBCodec::H264, &data);
        assert_eq!(units.len(), 2);
        assert_eq!(units.concat(), data);
        assert_eq!(
            units[0],
            cat(&[&SC4, &AUD, &SC4, &IDR_FIRST, &SC4, &P_SECOND_SLICE])
        );
        assert_eq!(units[1], cat(&[&SC4, &AUD, &SC4, &SEI, &SC4, &P_FIRST]));
    }

    #[test]
    fn hevc_units_split_on_parameter_sets_and_first_slice_segments() {
        let vps = [0x40, 0x01, 0x0C];
        let sps = [0x42, 0x01, 0x01];
        let idr_first = [0x26, 0x01, 0xAF]; // IDR_W_RADL, first_slice_segment_in_pic_flag = 1
        let trail_first = [0x02, 0x01, 0xBF];
        let trail_second = [0x02, 0x01, 0x3F]; // flag = 0
        let suffix_sei = [0x50, 0x01, 0x00];
        let data = cat(&[
            &SC4,
            &vps,
            &SC4,
            &sps,
            &SC4,
            &idr_first,
            &SC4,
            &trail_second,
            &SC4,
            &suffix_sei, // 0
            &SC4,
            &trail_first, // 1
            &SC4,
            &trail_first, // 2
        ]);
        let units = split_access_units(AnnexBCodec::Hevc, &data);
        assert_eq!(units.len(), 3);
        assert_eq!(units.concat(), data);
        assert_eq!(
            units[0],
            cat(&[
                &SC4,
                &vps,
                &SC4,
                &sps,
                &SC4,
                &idr_first,
                &SC4,
                &trail_second,
                &SC4,
                &suffix_sei
            ])
        );
        assert_eq!(units[1], cat(&[&SC4, &trail_first]));
    }

    #[test]
    fn data_without_a_start_code_or_with_one_unit_comes_back_whole() {
        for data in [&[][..], &[1, 2, 3][..], &[0, 0, 1][..]] {
            for codec in [AnnexBCodec::H264, AnnexBCodec::Hevc] {
                assert_eq!(split_access_units(codec, data), vec![data]);
            }
        }
        let one = cat(&[&SC4, &SPS, &SC4, &PPS, &SC4, &IDR_FIRST]);
        assert_eq!(split_access_units(AnnexBCodec::H264, &one), vec![&one[..]]);
    }

    #[test]
    fn bytes_before_the_first_start_code_stay_with_the_first_unit() {
        let data = cat(&[&[0xAA, 0xBB], &SC4, &IDR_FIRST, &SC4, &P_FIRST]);
        let units = split_access_units(AnnexBCodec::H264, &data);
        assert_eq!(units.len(), 2);
        assert_eq!(units[0], cat(&[&[0xAA, 0xBB], &SC4, &IDR_FIRST]));
    }

    #[test]
    fn the_real_clips_split_into_one_unit_per_picture() {
        for (codec, file, pictures) in [
            (AnnexBCodec::H264, "avc_test.h264", 30),
            (AnnexBCodec::Hevc, "hevc_test.hevc", 30),
            (AnnexBCodec::H264, "sample_h264.264", 60),
        ] {
            let path = format!("{}/../../test_data/{file}", env!("CARGO_MANIFEST_DIR"));
            let data = std::fs::read(&path).unwrap();
            let units = split_access_units(codec, &data);
            assert_eq!(units.len(), pictures, "{file}");
            assert_eq!(
                units.concat(),
                data,
                "{file}: the pieces cover the file exactly"
            );
        }
    }
}
