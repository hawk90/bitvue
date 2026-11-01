//! The decoder abstraction through the public API: `DecoderFactory`, `CodecRegistry` and the
//! default methods of the `Decoder` trait. (This file used to hold ten tests that defined their own
//! structs and asserted on literals they had just written, #386.)

use bitvue_decode::decoder::{ChromaFormat, DecodeError, DecodedFrame, FrameType};
use bitvue_decode::traits::{
    CodecRegistry, CodecType, Decoder, DecoderCapabilities, DecoderFactory,
};

fn clip() -> Vec<u8> {
    std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../test_data/av1_aomenc_testsrc2.ivf"
    ))
    .unwrap()
}

fn blank_frame(timestamp: i64) -> DecodedFrame {
    DecodedFrame {
        width: 2,
        height: 2,
        bit_depth: 8,
        y_plane: vec![0u8; 4].into_boxed_slice().into(),
        y_stride: 2,
        u_plane: None,
        u_stride: 0,
        v_plane: None,
        v_stride: 0,
        timestamp,
        frame_type: FrameType::Key,
        qp_avg: None,
        chroma_format: ChromaFormat::Monochrome,
    }
}

/// A decoder that hands out scripted results from `get_frame`.
struct Scripted {
    script: Vec<Result<DecodedFrame, DecodeError>>,
    sent: Vec<(usize, Option<i64>)>,
    resets: usize,
}

impl Scripted {
    fn new(script: Vec<Result<DecodedFrame, DecodeError>>) -> Self {
        script.iter().for_each(|_| ());
        Self {
            script: script.into_iter().rev().collect(),
            sent: Vec::new(),
            resets: 0,
        }
    }
}

impl Decoder for Scripted {
    fn codec_type(&self) -> CodecType {
        CodecType::VP9
    }
    fn capabilities(&self) -> DecoderCapabilities {
        DecoderCapabilities {
            codec: CodecType::VP9,
            max_width: 64,
            max_height: 64,
            supported_bit_depths: vec![8],
            hw_accel: false,
        }
    }
    fn send_data(&mut self, data: &[u8], timestamp: Option<i64>) -> Result<(), DecodeError> {
        self.sent.push((data.len(), timestamp));
        Ok(())
    }
    fn get_frame(&mut self) -> Result<DecodedFrame, DecodeError> {
        self.script.pop().unwrap_or(Err(DecodeError::NoFrame))
    }
    fn flush(&mut self) {}
    fn reset(&mut self) -> Result<(), DecodeError> {
        self.resets += 1;
        Ok(())
    }
}

#[test]
fn the_factory_builds_an_av1_decoder_that_decodes_a_real_clip_twice() {
    let mut decoder = DecoderFactory::create(CodecType::AV1).unwrap();
    assert_eq!(decoder.codec_type(), CodecType::AV1);
    let caps = decoder.capabilities();
    assert_eq!(caps.codec, CodecType::AV1);
    assert!(caps.supported_bit_depths.contains(&8));
    assert!(caps.max_width >= 320 && caps.max_height >= 240, "{caps:?}");

    let data = clip();
    let first = decoder.decode_all(&data).unwrap();
    assert_eq!(first.len(), 12);
    assert!(first.iter().all(|f| (f.width, f.height) == (320, 240)));
    decoder.reset().unwrap();
    let second = decoder.decode_all(&data).unwrap();
    assert_eq!(second.len(), 12, "a reset decoder decodes the clip again");
    assert_eq!(first[5].y_plane, second[5].y_plane);
}

#[test]
fn codecs_that_need_a_feature_name_it_when_it_is_off() {
    let message = |codec| match DecoderFactory::create(codec) {
        Ok(_) => String::new(),
        Err(DecodeError::Init(m)) => m,
        Err(other) => panic!("{codec}: {other:?}"),
    };
    #[cfg(not(feature = "ffmpeg"))]
    for codec in [CodecType::H264, CodecType::H265, CodecType::VP9] {
        assert!(message(codec).contains("--features ffmpeg"), "{codec}");
    }
    #[cfg(not(feature = "vvdec"))]
    assert!(message(CodecType::H266).contains("--features vvdec"));
    let _ = message;
}

#[test]
fn available_codecs_always_has_av1_and_matches_what_the_factory_can_build() {
    let available = DecoderFactory::available_codecs();
    assert!(available.contains(&CodecType::AV1));
    for codec in [
        CodecType::AV1,
        CodecType::H264,
        CodecType::H265,
        CodecType::H266,
        CodecType::VP9,
    ] {
        assert_eq!(
            available.contains(&codec),
            DecoderFactory::create(codec).is_ok(),
            "{codec}"
        );
    }
}

#[test]
fn codec_names_are_the_display_strings() {
    for (codec, name) in [
        (CodecType::AV1, "AV1"),
        (CodecType::H264, "H.264/AVC"),
        (CodecType::H265, "H.265/HEVC"),
        (CodecType::H266, "H.266/VVC"),
        (CodecType::VP9, "VP9"),
    ] {
        assert_eq!(codec.to_string(), name);
    }
}

#[test]
fn a_registry_registers_a_codec_once_and_builds_from_it() {
    let mut registry = CodecRegistry::default();
    assert!(!registry.is_registered(CodecType::VP9));
    assert!(registry.registered_codecs().is_empty());
    match registry.create(CodecType::VP9) {
        Err(DecodeError::Init(m)) => assert!(m.contains("No decoder registered"), "{m}"),
        other => panic!("{:?}", other.map(|_| ())),
    }

    registry
        .register(
            CodecType::VP9,
            Box::new(|| Ok(Box::new(Scripted::new(vec![])) as Box<dyn Decoder>)),
        )
        .unwrap();
    assert!(registry.is_registered(CodecType::VP9));
    assert_eq!(registry.registered_codecs(), vec![CodecType::VP9]);
    assert_eq!(
        registry.create(CodecType::VP9).unwrap().codec_type(),
        CodecType::VP9
    );
    // a second factory for the same codec is refused and the first one stays
    let err = registry
        .register(
            CodecType::VP9,
            Box::new(|| Err(DecodeError::Init("second".into()))),
        )
        .unwrap_err();
    assert!(err.contains("already registered"), "{err}");
    assert!(registry.create(CodecType::VP9).is_ok());
}

#[test]
fn a_failing_factory_reports_its_own_error() {
    let mut registry = CodecRegistry::default();
    registry
        .register(
            CodecType::H265,
            Box::new(|| Err(DecodeError::Init("no hardware".into()))),
        )
        .unwrap();
    match registry.create(CodecType::H265) {
        Err(DecodeError::Init(m)) => assert_eq!(m, "no hardware"),
        other => panic!("{:?}", other.map(|_| ())),
    }
}

#[test]
fn decode_all_sends_the_data_once_and_collects_frames_until_none_is_left() {
    let mut d = Scripted::new(vec![
        Ok(blank_frame(0)),
        Ok(blank_frame(1)),
        Ok(blank_frame(2)),
        Err(DecodeError::NoFrame),
        Ok(blank_frame(99)), // never reached
    ]);
    let frames = d.decode_all(&[1, 2, 3]).unwrap();
    assert_eq!(
        frames.iter().map(|f| f.timestamp).collect::<Vec<_>>(),
        vec![0, 1, 2]
    );
    assert_eq!(d.sent, vec![(3, None)]);
}

#[test]
fn collect_frames_stops_quietly_on_a_decode_error_but_passes_other_errors_on() {
    // a Decode error ends the collection and keeps the frames so far
    let mut d = Scripted::new(vec![
        Ok(blank_frame(0)),
        Err(DecodeError::Decode("bad packet".into())),
        Ok(blank_frame(1)),
    ]);
    assert_eq!(d.collect_frames().unwrap().len(), 1);
    // any other error is returned
    let mut d = Scripted::new(vec![
        Ok(blank_frame(0)),
        Err(DecodeError::Init("lost the device".into())),
    ]);
    match d.collect_frames() {
        Err(DecodeError::Init(m)) => assert_eq!(m, "lost the device"),
        other => panic!("{:?}", other.map(|f| f.len())),
    }
    // nothing decoded is an empty list, not an error
    assert!(Scripted::new(vec![]).collect_frames().unwrap().is_empty());
}

bitvue_test_alloc::install!();
