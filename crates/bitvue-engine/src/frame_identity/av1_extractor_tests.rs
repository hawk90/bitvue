//! `Av1FrameIdentityExtractor::extract_frames` on hand-built OBU streams (the expected number of
//! displayable frames is worked out from the OBU syntax by hand), on the fixture clips, and on
//! noise.

use super::{Av1FrameIdentityExtractor, FrameIdentityExtractor};

const TEMPORAL_DELIMITER: u8 = 2;
const SEQUENCE_HEADER: u8 = 1;
const FRAME_HEADER: u8 = 3;
const FRAME: u8 = 6;

/// `show_existing_frame` (bit 7) and `show_frame` (bit 4) of the first payload byte.
const SHOW_EXISTING: u8 = 0x80;
const SHOWN: u8 = 0x10;
const HIDDEN: u8 = 0x00;

fn leb128(mut value: usize) -> Vec<u8> {
    let mut out = Vec::new();
    loop {
        let byte = (value & 0x7F) as u8;
        value >>= 7;
        if value == 0 {
            out.push(byte);
            return out;
        }
        out.push(byte | 0x80);
    }
}

/// An OBU with a size field.
fn obu(obu_type: u8, payload: &[u8]) -> Vec<u8> {
    let mut out = vec![(obu_type << 3) | 0x02];
    out.extend(leb128(payload.len()));
    out.extend_from_slice(payload);
    out
}

fn count(data: &[u8]) -> usize {
    Av1FrameIdentityExtractor::new()
        .extract_frames(data)
        .unwrap()
        .len()
}

#[test]
fn a_shown_frame_obu_counts_and_a_hidden_one_does_not() {
    assert_eq!(count(&obu(FRAME, &[SHOWN, 0, 0])), 1);
    assert_eq!(count(&obu(FRAME, &[HIDDEN, 0, 0])), 0);
}

#[test]
fn a_show_existing_frame_header_counts_whatever_its_other_bits_are() {
    assert_eq!(count(&obu(FRAME_HEADER, &[SHOW_EXISTING | 0x03])), 1);
    assert_eq!(count(&obu(FRAME_HEADER, &[SHOW_EXISTING])), 1);
}

#[test]
fn a_frame_header_obu_counts_like_a_frame_obu() {
    assert_eq!(count(&obu(FRAME_HEADER, &[SHOWN])), 1);
    assert_eq!(count(&obu(FRAME_HEADER, &[HIDDEN])), 0);
}

#[test]
fn other_obu_types_never_count_even_with_the_show_bits_set() {
    let mut data = obu(TEMPORAL_DELIMITER, &[]);
    data.extend(obu(SEQUENCE_HEADER, &[SHOWN | SHOW_EXISTING]));
    data.extend(obu(4, &[SHOWN])); // tile group
    assert_eq!(count(&data), 0);
}

#[test]
fn a_temporal_unit_counts_its_shown_frames_only() {
    // delimiter, sequence header, hidden frame, shown frame, show-existing header, hidden frame
    let mut data = obu(TEMPORAL_DELIMITER, &[]);
    data.extend(obu(SEQUENCE_HEADER, &[0x00; 4]));
    data.extend(obu(FRAME, &[HIDDEN]));
    data.extend(obu(FRAME, &[SHOWN]));
    data.extend(obu(FRAME_HEADER, &[SHOW_EXISTING]));
    data.extend(obu(FRAME, &[HIDDEN]));
    assert_eq!(count(&data), 2);
}

#[test]
fn an_empty_frame_obu_has_no_flag_to_read_and_does_not_count() {
    assert_eq!(count(&obu(FRAME, &[])), 0);
    let mut data = obu(FRAME, &[]);
    data.extend(obu(FRAME, &[SHOWN]));
    assert_eq!(count(&data), 1);
}

#[test]
fn an_extension_byte_after_the_header_is_skipped() {
    // header with the extension flag (bit 2) and a size field, then the extension byte
    let mut data = vec![(FRAME << 3) | 0x04 | 0x02, 0x00, 0x01, SHOWN];
    data.extend(obu(FRAME, &[SHOWN]));
    assert_eq!(count(&data), 2);
}

#[test]
fn an_obu_without_a_size_field_runs_to_the_end_of_the_data() {
    let data = vec![FRAME << 3, SHOWN, 0xAA, 0xBB];
    assert_eq!(count(&data), 1);
    // nothing can follow it: a second "OBU" is just more payload
    let mut two = vec![FRAME << 3, HIDDEN];
    two.extend(obu(FRAME, &[SHOWN]));
    assert_eq!(count(&two), 0);
}

#[test]
fn a_multi_byte_size_field_is_decoded() {
    let mut payload = vec![SHOWN];
    payload.extend(vec![0u8; 299]); // 300 bytes: a two-byte LEB128 size
    let mut data = obu(FRAME, &payload);
    data.extend(obu(FRAME, &[SHOWN]));
    assert_eq!(count(&data), 2);
}

#[test]
fn a_set_forbidden_bit_skips_one_byte_and_scanning_resumes() {
    let mut data = vec![0x80];
    data.extend(obu(FRAME, &[SHOWN]));
    assert_eq!(count(&data), 1);
    let mut data = vec![0xFF, 0x80, 0x81];
    data.extend(obu(FRAME, &[SHOWN]));
    assert_eq!(count(&data), 1);
}

#[test]
fn a_truncated_obu_ends_the_scan_but_keeps_the_frames_before_it() {
    let mut data = obu(FRAME, &[SHOWN]);
    data.extend([(FRAME << 3) | 0x02, 0x10, SHOWN]); // claims 16 payload bytes, has 1
    assert_eq!(count(&data), 1);
}

#[test]
fn a_header_with_no_room_for_its_size_field_ends_the_scan() {
    let mut data = obu(FRAME, &[SHOWN]);
    data.push((FRAME << 3) | 0x02); // size flag set, nothing after the header
    assert_eq!(count(&data), 1);
}

#[test]
fn a_size_field_longer_than_eight_bytes_ends_the_scan_without_a_frame() {
    let mut data = vec![(FRAME << 3) | 0x02];
    data.extend([0xFF; 9]);
    data.push(SHOWN);
    assert_eq!(count(&data), 0);
}

#[test]
fn empty_data_has_no_frames() {
    assert_eq!(count(&[]), 0);
}

fn noise(seed: u64, len: usize) -> Vec<u8> {
    let mut x = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1;
    (0..len)
        .map(|_| {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            (x >> 24) as u8
        })
        .collect()
}

#[test]
fn noise_never_panics_and_never_finds_more_frames_than_bytes() {
    for seed in 0..2000u64 {
        let data = noise(seed, (seed % 97) as usize);
        assert!(count(&data) <= data.len());
    }
}

/// Every IVF frame payload of a clip, joined: a raw OBU stream.
fn ivf_obu_stream(ivf: &[u8]) -> (Vec<u8>, usize) {
    let mut out = Vec::new();
    let mut at = 32;
    let mut packets = 0;
    while at + 12 <= ivf.len() {
        let size = u32::from_le_bytes(ivf[at..at + 4].try_into().unwrap()) as usize;
        out.extend_from_slice(&ivf[at + 12..at + 12 + size]);
        at += 12 + size;
        packets += 1;
    }
    (out, packets)
}

#[test]
fn the_fixture_clips_find_at_least_one_shown_frame_per_packet_that_carries_one() {
    for (name, ivf) in [
        (
            "av1_test",
            &include_bytes!("../../../../test_data/av1_test.ivf")[..],
        ),
        (
            "aomenc",
            &include_bytes!("../../../../test_data/av1_aomenc_testsrc2.ivf")[..],
        ),
        (
            "rav1e",
            &include_bytes!("../../../../test_data/av1_rav1e_testsrc2.ivf")[..],
        ),
        (
            "svtav1",
            &include_bytes!("../../../../test_data/av1_svtav1_testsrc2.ivf")[..],
        ),
    ] {
        let (stream, packets) = ivf_obu_stream(ivf);
        let frames = count(&stream);
        // A packet is a temporal unit: it holds exactly one shown frame.
        assert_eq!(frames, packets, "{name}");
    }
}
