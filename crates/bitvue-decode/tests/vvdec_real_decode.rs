//! Decodes a real VVC stream through the real libvvdec and compares every output frame with
//! the reference decoders' bytes.
//!
//! Needs libvvdec 3.x at build time (`--features vvdec`). The previous `tests/vvdec_test.rs`
//! defined its own stand-in structs inside the tests and never touched the decoder, which is how
//! a header mismatch (UB in `VvcDecoder::new`) survived.
//!
//! The fixture `tests/fixtures/vvc/foreman_176x144_9f_10bit.266` is a 4 353-byte Annex B stream
//! (9 frames, 10-bit 4:2:0, reordered B-pyramid) encoded with vvenc 1.14 from the first frames of
//! `samples/foreman_h264.mp4`. The expected hashes are SHA-256 of each output frame as 16-bit
//! little-endian planar Y, Cb, Cr, produced by `vvdecapp` 3.2.0; FFmpeg's native VVC decoder
//! produced byte-identical output (equal md5 over the whole file).

#![cfg(feature = "vvdec")]

use bitvue_decode::decoder::DecodeError;
use bitvue_decode::{DecodedFrame, Decoder, FrameType, VvcDecoder};

const STREAM: &[u8] = include_bytes!("fixtures/vvc/foreman_176x144_9f_10bit.266");
const WIDTH: u32 = 176;
const HEIGHT: u32 = 144;

/// SHA-256 of each frame in output (display) order.
const EXPECTED_SHA256: [&str; 9] = [
    "a36de5c1ce83aee1a80732579b2657f530e581f800449e1dfb310701c6d35980",
    "f28386d9b0739cc7cb4b4ffb11a6556f5e961c45f97d5d90d44b7ab8ff63a51c",
    "33577154656f1b405ffb9cf29990f8b162e4be13d1abd70712ae601b38931515",
    "8207f38ca6ce576ba774809c4aa85894d00356afebc54b90d388319785bc71e3",
    "cb80d133e191230a233375c6631720647ba8cbcbde5a871c5345d624d358c18a",
    "8e4e48eb015f80618b7694c00627f41f8a664ba4dd991f389d10c9b89894689d",
    "e85998f6cb2358cefea407d56a0bdc3a6150f75aa41406802a6a8ad7970f8647",
    "434194a11540a490fb0faf8fe6d025bb5b90deb983c06be1ec4c5733f2356d50",
    "63eb383770c61598677a102a3eb3bf91ae0570e240f43b316828baab609a1dbb",
];

// ---- a minimal SHA-256 (the test crate has no hashing dependency; this keeps it that way) ----

fn sha256_hex(data: &[u8]) -> String {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    let mut msg = data.to_vec();
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&((data.len() as u64) * 8).to_be_bytes());
    for chunk in msg.as_chunks::<64>().0 {
        let mut w = [0u32; 64];
        for (i, word) in chunk.as_chunks::<4>().0.iter().enumerate() {
            w[i] = u32::from_be_bytes([word[0], word[1], word[2], word[3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let mut v = h;
        for i in 0..64 {
            let s1 = v[4].rotate_right(6) ^ v[4].rotate_right(11) ^ v[4].rotate_right(25);
            let ch = (v[4] & v[5]) ^ (!v[4] & v[6]);
            let t1 = v[7]
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = v[0].rotate_right(2) ^ v[0].rotate_right(13) ^ v[0].rotate_right(22);
            let maj = (v[0] & v[1]) ^ (v[0] & v[2]) ^ (v[1] & v[2]);
            let t2 = s0.wrapping_add(maj);
            v = [
                t1.wrapping_add(t2),
                v[0],
                v[1],
                v[2],
                v[3].wrapping_add(t1),
                v[4],
                v[5],
                v[6],
            ];
        }
        for (a, b) in h.iter_mut().zip(v) {
            *a = a.wrapping_add(b);
        }
    }
    h.iter().map(|x| format!("{x:08x}")).collect()
}

#[test]
fn sha256_helper_is_correct() {
    // FIPS 180-2 test vectors
    assert_eq!(
        sha256_hex(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    assert_eq!(
        sha256_hex(b""),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
}

/// The frame as 16-bit little-endian planar Y, Cb, Cr: the layout `vvdecapp` writes.
fn frame_bytes(f: &DecodedFrame) -> Vec<u8> {
    let mut out = f.y_plane.to_vec();
    out.extend_from_slice(f.u_plane.as_ref().expect("4:2:0 has Cb"));
    out.extend_from_slice(f.v_plane.as_ref().expect("4:2:0 has Cr"));
    out
}

fn assert_matches_reference(frames: &[DecodedFrame]) {
    assert_eq!(frames.len(), EXPECTED_SHA256.len(), "frame count");
    for (i, (frame, want)) in frames.iter().zip(EXPECTED_SHA256).enumerate() {
        assert_eq!(
            (frame.width, frame.height, frame.bit_depth),
            (WIDTH, HEIGHT, 10),
            "frame {i}"
        );
        assert_eq!(
            sha256_hex(&frame_bytes(frame)),
            want,
            "frame {i} differs from the reference"
        );
    }
}

/// Splits Annex B into units: each VCL NAL ends one access unit (single-slice pictures).
fn access_units(stream: &[u8]) -> Vec<&[u8]> {
    let mut starts = Vec::new();
    let mut i = 0;
    while i + 3 <= stream.len() {
        if stream[i..i + 3] == [0, 0, 1] {
            starts.push(i);
            i += 3;
        } else {
            i += 1;
        }
    }
    let mut aus = Vec::new();
    let mut au_start = starts[0];
    for (k, &s) in starts.iter().enumerate() {
        let end = starts.get(k + 1).copied().unwrap_or(stream.len());
        let nal_type = (stream[s + 4] >> 3) & 0x1f;
        if nal_type <= 11 {
            aus.push(&stream[au_start..end]);
            au_start = end;
        }
    }
    aus
}

#[test]
fn the_whole_stream_in_one_call_matches_the_reference() {
    let mut decoder = VvcDecoder::new().expect("libvvdec must initialise");
    let frames = decoder.decode_all(STREAM).expect("decode");
    assert_matches_reference(&frames);
}

#[test]
fn access_unit_by_access_unit_matches_the_reference() {
    let aus = access_units(STREAM);
    assert_eq!(aus.len(), 9, "fixture splits into 9 access units");

    let mut decoder = VvcDecoder::new().unwrap();
    let mut frames = Vec::new();
    for (i, au) in aus.iter().enumerate() {
        decoder.send_data(au, Some(i as i64)).unwrap();
        while let Ok(f) = decoder.get_frame() {
            frames.push(f);
        }
    }
    decoder.flush();
    while let Ok(f) = decoder.get_frame() {
        frames.push(f);
    }
    assert_matches_reference(&frames);
}

#[test]
fn the_first_output_frame_is_the_idr_and_frame_types_are_sane() {
    let mut decoder = VvcDecoder::new().unwrap();
    let frames = decoder.decode_all(STREAM).unwrap();
    // Output order: POC 0..8. The IDR picture is POC 7 (coded first, displayed 8th); the others
    // are B pictures. Exactly one random-access frame in this stream.
    let keys: Vec<usize> = frames
        .iter()
        .enumerate()
        .filter(|(_, f)| f.frame_type == FrameType::Key)
        .map(|(i, _)| i)
        .collect();
    assert_eq!(
        keys,
        vec![7],
        "the IDR is the 8th frame in output order: {keys:?}"
    );
    assert!(
        frames
            .iter()
            .filter(|f| f.frame_type == FrameType::Inter)
            .count()
            >= 6
    );
}

#[test]
fn timestamps_follow_the_data_that_carried_them() {
    let aus = access_units(STREAM);
    let mut decoder = VvcDecoder::new().unwrap();
    for (i, au) in aus.iter().enumerate() {
        decoder.send_data(au, Some(100 + i as i64)).unwrap();
    }
    decoder.flush();
    let mut stamps = Vec::new();
    while let Ok(f) = decoder.get_frame() {
        stamps.push(f.timestamp);
    }
    stamps.sort_unstable();
    assert_eq!(
        stamps,
        (100..109).collect::<Vec<i64>>(),
        "each AU's timestamp reaches one frame"
    );
}

#[test]
fn bad_input_is_an_error_and_the_decoder_keeps_working() {
    let mut decoder = VvcDecoder::new().unwrap();
    let junk: Vec<u8> = (0..300u32)
        .map(|i| (i.wrapping_mul(2654435761) >> 13) as u8)
        .collect();
    assert!(matches!(
        decoder.send_data(&junk, None),
        Err(DecodeError::Decode(_))
    ));
    let mut with_start_code = vec![0, 0, 1, 0x00, 0x01];
    with_start_code.extend_from_slice(&junk);
    assert!(decoder.send_data(&with_start_code, None).is_err());

    // The same decoder still decodes a valid stream correctly afterwards.
    let frames = decoder.decode_all(STREAM).unwrap();
    assert_matches_reference(&frames);
}

#[test]
fn empty_input_is_a_no_op() {
    let mut decoder = VvcDecoder::new().unwrap();
    decoder.send_data(&[], None).unwrap();
    assert!(matches!(decoder.get_frame(), Err(DecodeError::NoFrame)));
}

#[test]
fn a_decoder_can_be_reused_after_flush_and_after_reset() {
    let mut decoder = VvcDecoder::new().unwrap();
    assert_matches_reference(&decoder.decode_all(STREAM).unwrap());
    // second stream on the same decoder (vvdec stays usable after EOF)
    assert_matches_reference(&decoder.decode_all(STREAM).unwrap());
    // and on a freshly reset one
    decoder.reset().unwrap();
    assert_matches_reference(&decoder.decode_all(STREAM).unwrap());
}

#[test]
fn a_truncated_access_unit_does_not_poison_the_next_stream() {
    // vvdec answers a half access unit with "restart required" at flush time. The wrapper must
    // recover (replace the C decoder) so the next, valid stream decodes correctly.
    let aus = access_units(STREAM);
    let mut decoder = VvcDecoder::new().unwrap();
    let _ = decoder.send_data(&aus[0][..aus[0].len() / 2], None);
    for au in &aus[1..] {
        let _ = decoder.send_data(au, None);
    }
    decoder.flush();
    while decoder.get_frame().is_ok() {}

    decoder.reset().unwrap();
    assert_matches_reference(&decoder.decode_all(STREAM).unwrap());
}

#[test]
fn the_decoder_can_move_to_another_thread() {
    // `Send` must hold without any `unsafe impl`: the decoder owns a channel, not raw pointers.
    fn assert_send<T: Send>() {}
    assert_send::<VvcDecoder>();

    let handle = std::thread::spawn(|| {
        let mut decoder = VvcDecoder::new().unwrap();
        decoder.decode_all(STREAM).unwrap()
    });
    assert_matches_reference(&handle.join().unwrap());
}

#[test]
fn the_library_reports_a_3_x_version() {
    let version = bitvue_decode::vvdec::vvdec_version();
    assert!(
        version.starts_with("3."),
        "ABI pinned to vvdec 3.x, got {version:?}"
    );
}

/// Not part of the normal run (`cargo test -- --ignored soak`): decodes the fixture with many
/// short-lived decoders. Memory use (RSS) must stay flat; a missing `vvdec_decoder_close` or
/// `vvdec_frame_unref` makes it grow linearly. macOS `leaks` does not see vvdec's pooled
/// allocations, so RSS growth is the signal.
#[test]
#[ignore = "soak test: run with --ignored and watch RSS"]
fn soak_many_decoders_do_not_grow_memory() {
    let n: usize = std::env::var("SOAK_ITERATIONS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(200);
    for _ in 0..n {
        let mut decoder = VvcDecoder::new().unwrap();
        assert_eq!(decoder.decode_all(STREAM).unwrap().len(), 9);
    }
}

/// One decoder, many frames: a missing `vvdec_frame_unref` is invisible when decoders are
/// short-lived (vvdec frees its picture pool when the decoder closes) but grows with the number of
/// frames decoded. `SOAK_ITERATIONS` passes over the 9-frame stream = 9 x N frames.
#[test]
#[ignore = "soak test: run with --ignored and watch RSS"]
fn soak_one_decoder_many_frames_do_not_grow_memory() {
    let n: usize = std::env::var("SOAK_ITERATIONS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(200);
    let mut decoder = VvcDecoder::new().unwrap();
    for _ in 0..n {
        assert_eq!(decoder.decode_all(STREAM).unwrap().len(), 9);
    }
}
