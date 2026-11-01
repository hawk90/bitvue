//! The FFmpeg-backed decoders (H.264, H.265, VP9) on real clips. Only built with
//! `--features ffmpeg`, which needs FFmpeg's libraries; CI does not build that feature today.
//! (This file used to hold ten tests that defined their own structs and asserted on literals they
//! had just written, and ran in every build, #386.)

#![cfg(feature = "ffmpeg")]

use bitvue_decode::decoder::{validate_frame, ChromaFormat};
use bitvue_decode::traits::{CodecType, Decoder, DecoderFactory};
use bitvue_decode::{H264Decoder, HevcDecoder, Vp9Decoder};

fn clip(name: &str) -> Vec<u8> {
    std::fs::read(format!(
        "{}/../../test_data/{name}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap_or_else(|e| panic!("test_data/{name}: {e}"))
}

#[test]
fn each_decoder_reports_its_codec_and_capabilities() {
    let decoders: [(Box<dyn Decoder>, CodecType); 3] = [
        (Box::new(H264Decoder::new().unwrap()), CodecType::H264),
        (Box::new(HevcDecoder::new().unwrap()), CodecType::H265),
        (Box::new(Vp9Decoder::new().unwrap()), CodecType::VP9),
    ];
    for (decoder, codec) in decoders {
        assert_eq!(decoder.codec_type(), codec);
        let caps = decoder.capabilities();
        assert_eq!(caps.codec, codec);
        assert!(caps.supported_bit_depths.contains(&8), "{codec}");
        assert!(caps.max_width >= 1920 && caps.max_height >= 1080, "{codec}");
    }
}

#[test]
fn the_factory_builds_the_ffmpeg_decoders() {
    for codec in [CodecType::H264, CodecType::H265, CodecType::VP9] {
        assert_eq!(DecoderFactory::create(codec).unwrap().codec_type(), codec);
        assert!(DecoderFactory::available_codecs().contains(&codec));
    }
}

#[test]
fn vp9_packets_decode_to_every_frame_of_the_clip() {
    // ffprobe: 60 frames, 352x288
    let data = clip("sample_vp9.ivf");
    let mut decoder = Vp9Decoder::new().unwrap();
    let (mut pos, mut frames) = (32usize, Vec::new());
    while pos + 12 <= data.len() {
        let size = u32::from_le_bytes(data[pos..pos + 4].try_into().unwrap()) as usize;
        decoder
            .send_data(&data[pos + 12..pos + 12 + size], None)
            .unwrap();
        while let Ok(f) = decoder.get_frame() {
            frames.push(f);
        }
        pos += 12 + size;
    }
    decoder.flush();
    while let Ok(f) = decoder.get_frame() {
        frames.push(f);
    }
    assert_eq!(frames.len(), 60);
    for (i, f) in frames.iter().enumerate() {
        assert_eq!((f.width, f.height, f.bit_depth), (352, 288, 8), "frame {i}");
        assert_eq!(f.chroma_format, ChromaFormat::Yuv420, "frame {i}");
        validate_frame(f).unwrap_or_else(|e| panic!("frame {i}: {e}"));
    }
}

/// A raw Annex B stream sent whole decodes every picture, in order (#402): the decoders split it
/// into access units, because libavcodec takes one packet to be one picture.
#[test]
fn h264_and_hevc_streams_decode_every_picture() {
    let cases: [(&str, usize, Box<dyn Decoder>); 3] = [
        ("avc_test.h264", 30, Box::new(H264Decoder::new().unwrap())),
        ("hevc_test.hevc", 30, Box::new(HevcDecoder::new().unwrap())),
        ("sample_h264.264", 60, Box::new(H264Decoder::new().unwrap())),
    ];
    for (file, pictures, mut decoder) in cases {
        let mut frames = decoder.decode_all(&clip(file)).unwrap();
        decoder.flush();
        frames.extend(decoder.collect_frames().unwrap());
        assert_eq!(frames.len(), pictures, "{file}");
        let (w, h) = (frames[0].width, frames[0].height);
        for (i, frame) in frames.iter().enumerate() {
            assert_eq!(
                (frame.width, frame.height, frame.bit_depth),
                (w, h, 8),
                "{file} #{i}"
            );
            assert_eq!(frame.chroma_format, ChromaFormat::Yuv420, "{file} #{i}");
            validate_frame(frame).unwrap_or_else(|e| panic!("{file} #{i}: {e}"));
        }
    }
}

#[test]
fn garbage_is_not_a_panic_and_a_reset_decoder_works_again() {
    let mut decoder = Vp9Decoder::new().unwrap();
    let _ = decoder.send_data(&[0xde, 0xad, 0xbe, 0xef, 1, 2, 3, 4, 5, 6], None);
    decoder.reset().unwrap();
    let data = clip("sample_vp9.ivf");
    let size = u32::from_le_bytes(data[32..36].try_into().unwrap()) as usize;
    decoder.send_data(&data[44..44 + size], None).unwrap();
    decoder.flush();
    assert!(
        decoder.get_frame().is_ok(),
        "the first key frame decodes after a reset"
    );
}

bitvue_test_alloc::install!();
