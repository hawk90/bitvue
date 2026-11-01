//! `Av1Decoder` on real clips: packet-by-packet decoding, flushing, bad input, film grain and the
//! file entry point. (This file used to hold ten tests that defined their own structs and asserted
//! on literals they had just written, #386.)

use bitvue_decode::decoder::{validate_frame, DecodeError, DecodedFrame};
use bitvue_decode::Av1Decoder;
use tempfile::TempDir;

fn ivf(name: &str) -> Vec<u8> {
    std::fs::read(format!(
        "{}/../../test_data/{name}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap_or_else(|e| panic!("test_data/{name}: {e}"))
}

/// The packets of an IVF file (12-byte frame headers after the 32-byte file header).
fn packets(data: &[u8]) -> Vec<Vec<u8>> {
    let (mut pos, mut out) = (32usize, Vec::new());
    while pos + 12 <= data.len() {
        let size = u32::from_le_bytes(data[pos..pos + 4].try_into().unwrap()) as usize;
        out.push(data[pos + 12..pos + 12 + size].to_vec());
        pos += 12 + size;
    }
    out
}

fn drain(decoder: &mut Av1Decoder, into: &mut Vec<DecodedFrame>) {
    while let Ok(frame) = decoder.get_frame() {
        into.push(frame);
    }
}

#[test]
fn a_decoder_with_nothing_sent_has_no_frame() {
    let mut decoder = Av1Decoder::new().unwrap();
    assert!(matches!(decoder.get_frame(), Err(DecodeError::NoFrame)));
    // and empty input is refused rather than treated as zero frames
    assert!(matches!(
        decoder.decode_all(&[]),
        Err(DecodeError::Decode(_))
    ));
}

#[test]
fn packets_sent_one_by_one_come_back_with_their_timestamps() {
    let data = ivf("av1_aomenc_testsrc2.ivf");
    let mut decoder = Av1Decoder::new().unwrap();
    let mut frames = Vec::new();
    for (i, packet) in packets(&data).iter().enumerate() {
        decoder.send_data(packet, 100 + i as i64).unwrap();
        drain(&mut decoder, &mut frames);
    }
    decoder.flush();
    drain(&mut decoder, &mut frames);
    assert_eq!(frames.len(), 12);
    assert_eq!(
        frames.iter().map(|f| f.timestamp).collect::<Vec<_>>(),
        (100..112).collect::<Vec<_>>()
    );
    for (i, f) in frames.iter().enumerate() {
        assert_eq!((f.width, f.height, f.bit_depth), (320, 240, 8), "frame {i}");
        validate_frame(f).unwrap_or_else(|e| panic!("frame {i}: {e}"));
    }
}

#[test]
fn packet_by_packet_and_whole_file_decoding_give_the_same_pixels() {
    let data = ivf("av1_aomenc_testsrc2.ivf");
    let whole = Av1Decoder::new().unwrap().decode_all(&data).unwrap();
    let mut decoder = Av1Decoder::new().unwrap();
    let mut by_packet = Vec::new();
    for (i, packet) in packets(&data).iter().enumerate() {
        decoder.send_data_owned(packet.clone(), i as i64).unwrap();
        drain(&mut decoder, &mut by_packet);
    }
    assert_eq!(whole.len(), by_packet.len());
    for (i, (a, b)) in whole.iter().zip(&by_packet).enumerate() {
        assert_eq!(a.y_plane, b.y_plane, "frame {i} luma");
        assert_eq!(a.u_plane, b.u_plane, "frame {i} U");
    }
}

#[test]
fn bad_input_is_an_error_and_the_decoder_keeps_working() {
    let mut decoder = Av1Decoder::new().unwrap();
    assert!(matches!(
        decoder.send_data(&[0xde, 0xad, 0xbe, 0xef, 1, 2, 3], 5),
        Err(DecodeError::Decode(_))
    ));
    assert!(matches!(decoder.get_frame(), Err(DecodeError::NoFrame)));
    // the same decoder decodes a real clip afterwards
    let frames = decoder.decode_all(&ivf("av1_aomenc_testsrc2.ivf")).unwrap();
    assert_eq!(frames.len(), 12);
}

#[test]
fn a_truncated_clip_is_an_error() {
    let data = ivf("av1_aomenc_testsrc2.ivf");
    let mut decoder = Av1Decoder::new().unwrap();
    assert!(decoder.decode_all(&data[..data.len() / 2]).is_err());
}

#[test]
fn film_grain_is_applied_or_left_out_as_asked() {
    // a clip with film grain: the two settings give different luma on every shown frame
    let grain = ivf("av1_aomenc_filmgrain.ivf");
    let with = Av1Decoder::new_with_apply_grain(true)
        .unwrap()
        .decode_all(&grain)
        .unwrap();
    let without = Av1Decoder::new_with_apply_grain(false)
        .unwrap()
        .decode_all(&grain)
        .unwrap();
    assert_eq!((with.len(), without.len()), (10, 10));
    assert!(with
        .iter()
        .zip(&without)
        .all(|(a, b)| a.y_plane != b.y_plane));
    // and a clip without grain is not touched by the setting
    let plain = ivf("av1_aomenc_testsrc2.ivf");
    let a = Av1Decoder::new_with_apply_grain(true)
        .unwrap()
        .decode_all(&plain)
        .unwrap();
    let b = Av1Decoder::new_with_apply_grain(false)
        .unwrap()
        .decode_all(&plain)
        .unwrap();
    assert!(a.iter().zip(&b).all(|(x, y)| x.y_plane == y.y_plane));
}

#[test]
fn decode_from_file_reads_a_file_below_the_working_directory_and_refuses_others() {
    // tests run with the crate directory as working directory
    let dir = TempDir::new_in(".").unwrap();
    let path = dir.path().join("clip.ivf");
    std::fs::write(&path, ivf("av1_aomenc_testsrc2.ivf")).unwrap();
    let frames = Av1Decoder::new().unwrap().decode_from_file(&path).unwrap();
    assert_eq!(frames.len(), 12);

    // a file outside the working directory
    let outside = format!(
        "{}/../../test_data/av1_aomenc_testsrc2.ivf",
        env!("CARGO_MANIFEST_DIR")
    );
    match Av1Decoder::new().unwrap().decode_from_file(&outside) {
        Err(DecodeError::Decode(m)) => assert!(m.contains("outside working directory"), "{m}"),
        other => panic!("{:?}", other.map(|f| f.len())),
    }
    // a file that does not exist
    match Av1Decoder::new()
        .unwrap()
        .decode_from_file("no_such_clip.ivf")
    {
        Err(DecodeError::Decode(m)) => assert!(m.contains("Invalid path"), "{m}"),
        other => panic!("{:?}", other.map(|f| f.len())),
    }
}

#[test]
fn validate_frame_rejects_frames_that_cannot_be_right() {
    let good = Av1Decoder::new()
        .unwrap()
        .decode_all(&ivf("av1_aomenc_testsrc2.ivf"))
        .unwrap()
        .remove(0);
    assert!(validate_frame(&good).is_ok());

    let mut zero_width = good.clone();
    zero_width.width = 0;
    assert!(validate_frame(&zero_width).is_err());

    let mut zero_height = good.clone();
    zero_height.height = 0;
    assert!(validate_frame(&zero_height).is_err());

    let mut short_plane = good.clone();
    short_plane.y_plane = vec![0u8; 10].into_boxed_slice().into();
    assert!(validate_frame(&short_plane).is_err());
}

bitvue_test_alloc::install!();
