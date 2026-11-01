//! `DecoderFactory::register_codec` goes through the process-wide registry, so it gets its own
//! test binary: registering a codec here must not change what the other trait tests see.
//! H.266 is only unbuildable without the `vvdec` feature, so the whole file needs it off.

#![cfg(not(feature = "vvdec"))]

use bitvue_decode::decoder::{DecodeError, DecodedFrame};
use bitvue_decode::traits::{CodecType, Decoder, DecoderCapabilities, DecoderFactory};

struct Nothing;

impl Decoder for Nothing {
    fn codec_type(&self) -> CodecType {
        CodecType::H266
    }
    fn capabilities(&self) -> DecoderCapabilities {
        DecoderCapabilities {
            codec: CodecType::H266,
            max_width: 16,
            max_height: 16,
            supported_bit_depths: vec![8],
            hw_accel: false,
        }
    }
    fn send_data(&mut self, _: &[u8], _: Option<i64>) -> Result<(), DecodeError> {
        Ok(())
    }
    fn get_frame(&mut self) -> Result<DecodedFrame, DecodeError> {
        Err(DecodeError::NoFrame)
    }
    fn flush(&mut self) {}
    fn reset(&mut self) -> Result<(), DecodeError> {
        Ok(())
    }
}

#[test]
fn registering_through_the_factory_makes_the_codec_buildable_once() {
    // the default build cannot create H.266 ...
    assert!(DecoderFactory::create(CodecType::H266).is_err());
    DecoderFactory::register_codec(CodecType::H266, Box::new(|| Ok(Box::new(Nothing)))).unwrap();
    // ... and can after the registration
    let decoder = DecoderFactory::create(CodecType::H266).unwrap();
    assert_eq!(decoder.codec_type(), CodecType::H266);
    // a second registration of the same codec is refused
    let err = DecoderFactory::register_codec(
        CodecType::H266,
        Box::new(|| Err(DecodeError::Init("again".into()))),
    )
    .unwrap_err();
    assert!(err.contains("already registered"), "{err}");
    assert!(DecoderFactory::create(CodecType::H266).is_ok());
}

bitvue_test_alloc::install!();
