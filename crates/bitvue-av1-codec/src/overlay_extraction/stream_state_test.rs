//! `StreamDecodeState::decode_next` over whole fixture streams: after every frame the state it
//! leaves behind (slot contents, order hints, saved CDFs, motion fields, symbol trace and decoded
//! units) is folded into one pinned FNV-1a hash per stream.

use super::*;
use crate::obu::{ObuIterator, ObuType};

fn fnv(hash: &mut u64, bytes: &[u8]) {
    for b in bytes {
        *hash ^= u64::from(*b);
        *hash = hash.wrapping_mul(0x100000001b3);
    }
}

/// The first sequence header OBU of the stream, parsed, and its raw bytes.
fn sequence_of(packets: &[crate::ivf::IvfFrame]) -> (SequenceHeader, Vec<u8>) {
    for packet in packets {
        let mut iter = ObuIterator::new(&packet.data);
        while let Some(Ok(found)) = iter.next_obu_with_offset() {
            if found.obu.header.obu_type == ObuType::SequenceHeader {
                let raw = packet.data[found.offset..found.offset + found.consumed].to_vec();
                return (
                    crate::parse_sequence_header(&found.obu.payload).unwrap(),
                    raw,
                );
            }
        }
    }
    panic!("stream has no sequence header");
}

/// Folds what `state` holds into `hash`: per slot the identity group of its frame, its order
/// hint, whether it has a motion field and its saved CDFs.
fn fold_state(hash: &mut u64, state: &StreamDecodeState) {
    let mut seen: Vec<*const ParsedFrame> = Vec::new();
    for slot in 0..8u8 {
        let group = match &state.slots[usize::from(slot)] {
            Some(frame) => {
                let ptr = Arc::as_ptr(frame);
                let at = seen.iter().position(|p| *p == ptr).unwrap_or_else(|| {
                    seen.push(ptr);
                    seen.len() - 1
                });
                at as i64
            }
            None => -1,
        };
        fnv(hash, &group.to_le_bytes());
        fnv(
            hash,
            &state.motion.ref_state.ref_order_hint()[usize::from(slot)].to_le_bytes(),
        );
        fnv(hash, &[state.motion.has_motion_field(slot) as u8]);
        fnv(
            hash,
            format!("{:?}", state.motion.saved_cdf(slot)).as_bytes(),
        );
    }
}

fn run(ivf: &[u8], max_frames: usize) -> (u64, usize) {
    let (_hdr, packets) = crate::ivf::parse_ivf_frames(ivf).unwrap();
    let (seq, seq_bytes) = sequence_of(&packets);
    let mut state = StreamDecodeState::new();
    let mut hash = 0xcbf29ce484222325u64;
    let mut decoded = 0;
    'stream: for packet in &packets {
        let units = frame_units(&packet.data, seq.reduced_still_picture_header);
        for unit in &units {
            if decoded == max_frames {
                break 'stream;
            }
            let obu_data = [seq_bytes.as_slice(), unit.as_slice()].concat();
            let frame = state.decode_next(&obu_data, &seq).unwrap();
            decoded += 1;
            fnv(&mut hash, &[frame.refresh_frame_flags]);
            fnv(&mut hash, &frame.order_hint.to_le_bytes());
            fnv(&mut hash, format!("{:?}", frame.decoded).as_bytes());
            fnv(&mut hash, format!("{:?}", state.last_trace).as_bytes());
            fold_state(&mut hash, &state);
        }
        if units.is_empty() {
            let obu_data = [seq_bytes.as_slice(), packet.data.as_slice()].concat();
            state.skip_unit(&obu_data).unwrap();
            fold_state(&mut hash, &state);
        }
    }
    (hash, decoded)
}

macro_rules! stream_case {
    ($name:ident, $file:literal, $frames:expr, $hash:expr) => {
        #[test]
        fn $name() {
            let data = include_bytes!(concat!("../../../../test_data/", $file));
            let (hash, decoded) = run(data, $frames);
            assert_eq!(decoded, $frames, "fixture has enough frames");
            assert_eq!(hash, $hash);
        }
    };
}

stream_case!(
    av1_test_stream_state,
    "av1_test.ivf",
    10,
    3567507827026309127
);
stream_case!(
    aomenc_stream_state,
    "av1_aomenc_testsrc2.ivf",
    8,
    11886248314148368264
);
stream_case!(
    rav1e_stream_state,
    "av1_rav1e_testsrc2.ivf",
    8,
    7191996451845630663
);
stream_case!(
    svtav1_stream_state,
    "av1_svtav1_testsrc2.ivf",
    8,
    16904493856448060124
);
stream_case!(
    aomenc_tiles_stream_state,
    "av1_aomenc_tiles5.ivf",
    4,
    62663505108442518
);

#[test]
fn quantizer_category_steps_up_just_above_20_60_and_120() {
    let cats: Vec<u8> = [i16::MIN, 0, 20, 21, 60, 61, 120, 121, 255]
        .into_iter()
        .map(quantizer_category)
        .collect();
    assert_eq!(cats, [0, 0, 0, 1, 1, 2, 2, 3, 3]);
}

/// `test_data/av1_fuzz_oversized_frame.ivf` (5.6 KB, minimized from a `stream_analysis` fuzz
/// input that was itself a mutation of our own clips): one key frame whose sequence header allows
/// 65536 x 65536 and whose frame is 65536 x 65536. It is past the decode limit, so it is not
/// decoded; its saved motion field used to be sized by the declared extent anyway (3 GB, 455 ms).
/// The test binaries run under a 1 GiB live-heap cap that aborts the process, so going back to
/// the old sizing fails this test.
#[test]
fn frames_past_the_decode_limit_are_not_decoded_and_save_no_motion_field() {
    const CLIP: &[u8] = include_bytes!("../../../../test_data/av1_fuzz_oversized_frame.ivf");
    let mut analyzer = crate::overlay_extraction::StreamAnalyzer::with_limits(CLIP, 2, 2).unwrap();
    let mut oversized = 0;
    for index in 0..analyzer.frame_count() {
        let frame = analyzer.analyze(index).unwrap();
        let dims = frame.dimensions;
        if !frame_within_decode_limit(&dims) {
            oversized += 1;
            let decoded = frame.decoded.as_ref().expect("the frame has tile data");
            assert!(decoded.units.is_empty(), "frame {index} was decoded");
        }
    }
    assert_eq!(oversized, 1, "the clip no longer has its oversized frame");
}

#[test]
fn the_decode_limit_is_level_6_3s_max_pic_size() {
    use crate::overlay_extraction::parser::FrameDimensions;
    let dims = |width, height| FrameDimensions {
        width,
        height,
        ..Default::default()
    };
    assert!(frame_within_decode_limit(&dims(16384, 2176))); // exactly 35,651,584
    assert!(!frame_within_decode_limit(&dims(16384, 2177)));
    assert!(!frame_within_decode_limit(&dims(65536, 65536)));
    assert!(frame_within_decode_limit(&dims(0, 0)));
}
