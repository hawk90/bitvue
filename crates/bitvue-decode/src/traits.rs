//! Decoder trait abstraction for multi-codec support

use crate::decoder::{DecodeError, DecodedFrame, Result};

/// Codec type enumeration for decoder selection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodecType {
    /// AV1 codec
    AV1,
    /// H.264/AVC codec
    H264,
    /// H.265/HEVC codec
    H265,
    /// H.266/VVC codec
    H266,
    /// VP9 codec
    VP9,
}

impl std::fmt::Display for CodecType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CodecType::AV1 => write!(f, "AV1"),
            CodecType::H264 => write!(f, "H.264/AVC"),
            CodecType::H265 => write!(f, "H.265/HEVC"),
            CodecType::H266 => write!(f, "H.266/VVC"),
            CodecType::VP9 => write!(f, "VP9"),
        }
    }
}

/// Decoder capabilities information
#[derive(Debug, Clone)]
pub struct DecoderCapabilities {
    /// Supported codec
    pub codec: CodecType,
    /// Maximum supported resolution width
    pub max_width: u32,
    /// Maximum supported resolution height
    pub max_height: u32,
    /// Supported bit depths
    pub supported_bit_depths: Vec<u8>,
    /// Hardware acceleration available
    pub hw_accel: bool,
}

/// Universal decoder trait for all video codecs
///
/// This trait provides a unified interface for decoding video frames
/// across different codecs (AV1, H.264, H.265, VP9, etc.).
pub trait Decoder: Send {
    /// Get the codec type this decoder handles
    fn codec_type(&self) -> CodecType;

    /// Get decoder capabilities
    fn capabilities(&self) -> DecoderCapabilities;

    /// Send encoded data to the decoder
    ///
    /// # Arguments
    ///
    /// * `data` - Encoded bitstream data
    /// * `timestamp` - Optional presentation timestamp
    ///
    /// # Returns
    ///
    /// Returns Ok(()) if data was successfully queued for decoding.
    fn send_data(&mut self, data: &[u8], timestamp: Option<i64>) -> Result<()>;

    /// Get the next decoded frame
    ///
    /// # Returns
    ///
    /// Returns the next available decoded frame, or an error if no frame
    /// is ready or decoding failed.
    fn get_frame(&mut self) -> Result<DecodedFrame>;

    /// Decode all frames from data (convenience method)
    ///
    /// # Arguments
    ///
    /// * `data` - Encoded bitstream data (may include container format)
    ///
    /// # Returns
    ///
    /// Returns all decoded frames.
    fn decode_all(&mut self, data: &[u8]) -> Result<Vec<DecodedFrame>> {
        self.send_data(data, None)?;
        self.collect_frames()
    }

    /// Collect all available decoded frames from the decoder
    fn collect_frames(&mut self) -> Result<Vec<DecodedFrame>> {
        let mut frames = Vec::new();
        loop {
            match self.get_frame() {
                Ok(frame) => frames.push(frame),
                Err(DecodeError::Decode(_)) | Err(DecodeError::NoFrame) => break,
                Err(e) => return Err(e),
            }
        }
        Ok(frames)
    }

    /// Flush the decoder (finish decoding and output remaining frames)
    fn flush(&mut self);

    /// Reset the decoder state
    fn reset(&mut self) -> Result<()>;
}

/// Decoder factory for creating codec-specific decoders
pub struct DecoderFactory;

impl DecoderFactory {
    /// Create a decoder for the specified codec
    ///
    /// # Arguments
    ///
    /// * `codec` - The codec type to create a decoder for
    ///
    /// # Returns
    ///
    /// Returns a boxed decoder implementation, or an error if the codec
    /// is not supported or decoder initialization failed.
    ///
    /// # Example
    ///
    /// ```
    /// use bitvue_decode::traits::{DecoderFactory, CodecType};
    ///
    /// let decoder = DecoderFactory::create(CodecType::AV1).unwrap();
    /// ```
    pub fn create(codec: CodecType) -> Result<Box<dyn Decoder>> {
        match codec {
            CodecType::AV1 => {
                let decoder = crate::decoder::Av1Decoder::new()?;
                Ok(Box::new(decoder))
            }
            #[cfg(feature = "ffmpeg")]
            CodecType::H264 => {
                let decoder = crate::ffmpeg::H264Decoder::new()?;
                Ok(Box::new(decoder))
            }
            #[cfg(not(feature = "ffmpeg"))]
            CodecType::H264 => Err(DecodeError::Init(
                "H.264 decoder requires ffmpeg feature (rebuild with --features ffmpeg)"
                    .to_string(),
            )),
            #[cfg(feature = "ffmpeg")]
            CodecType::H265 => {
                let decoder = crate::ffmpeg::HevcDecoder::new()?;
                Ok(Box::new(decoder))
            }
            #[cfg(not(feature = "ffmpeg"))]
            CodecType::H265 => Err(DecodeError::Init(
                "H.265 decoder requires ffmpeg feature (rebuild with --features ffmpeg)"
                    .to_string(),
            )),
            #[cfg(feature = "ffmpeg")]
            CodecType::VP9 => {
                let decoder = crate::ffmpeg::Vp9Decoder::new()?;
                Ok(Box::new(decoder))
            }
            #[cfg(not(feature = "ffmpeg"))]
            CodecType::VP9 => Err(DecodeError::Init(
                "VP9 decoder requires ffmpeg feature (rebuild with --features ffmpeg)".to_string(),
            )),
            #[cfg(feature = "vvdec")]
            CodecType::H266 => {
                let decoder = crate::vvdec::VvcDecoder::new()?;
                Ok(Box::new(decoder))
            }
            #[cfg(not(feature = "vvdec"))]
            CodecType::H266 => Err(DecodeError::Init(
                "VVC decoder requires vvdec feature (rebuild with --features vvdec)".to_string(),
            )),
        }
    }

    /// List all available decoders
    #[allow(unused_mut)]
    pub fn available_codecs() -> Vec<CodecType> {
        let mut codecs = vec![CodecType::AV1];

        #[cfg(feature = "ffmpeg")]
        {
            codecs.extend([CodecType::H264, CodecType::H265, CodecType::VP9]);
        }

        #[cfg(feature = "vvdec")]
        {
            codecs.push(CodecType::H266);
        }

        codecs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_codec_type_display() {
        assert_eq!(format!("{}", CodecType::AV1), "AV1");
        assert_eq!(format!("{}", CodecType::H264), "H.264/AVC");
        assert_eq!(format!("{}", CodecType::H265), "H.265/HEVC");
        assert_eq!(format!("{}", CodecType::H266), "H.266/VVC");
        assert_eq!(format!("{}", CodecType::VP9), "VP9");
    }

    #[test]
    fn test_decoder_factory_av1() {
        let result = DecoderFactory::create(CodecType::AV1);
        assert!(result.is_ok());
        let decoder = result.unwrap();
        assert_eq!(decoder.codec_type(), CodecType::AV1);
    }

    #[test]
    #[cfg(feature = "ffmpeg")]
    fn test_decoder_factory_h264() {
        let result = DecoderFactory::create(CodecType::H264);
        assert!(result.is_ok());
        let decoder = result.unwrap();
        assert_eq!(decoder.codec_type(), CodecType::H264);
    }

    #[test]
    #[cfg(feature = "ffmpeg")]
    fn test_decoder_factory_hevc() {
        let result = DecoderFactory::create(CodecType::H265);
        assert!(result.is_ok());
        let decoder = result.unwrap();
        assert_eq!(decoder.codec_type(), CodecType::H265);
    }

    #[test]
    #[cfg(not(feature = "ffmpeg"))]
    fn test_decoder_factory_unsupported_without_ffmpeg() {
        let result = DecoderFactory::create(CodecType::H264);
        assert!(result.is_err());

        let result = DecoderFactory::create(CodecType::H265);
        assert!(result.is_err());
    }

    #[test]
    #[cfg(feature = "ffmpeg")]
    fn test_decoder_factory_vp9() {
        let result = DecoderFactory::create(CodecType::VP9);
        assert!(result.is_ok());
        let decoder = result.unwrap();
        assert_eq!(decoder.codec_type(), CodecType::VP9);
    }

    #[test]
    #[cfg(not(feature = "ffmpeg"))]
    fn test_decoder_factory_vp9_unsupported() {
        let result = DecoderFactory::create(CodecType::VP9);
        assert!(result.is_err());
    }

    #[test]
    #[cfg(feature = "vvdec")]
    fn test_decoder_factory_vvc() {
        let result = DecoderFactory::create(CodecType::H266);
        assert!(result.is_ok());
        let decoder = result.unwrap();
        assert_eq!(decoder.codec_type(), CodecType::H266);
    }

    #[test]
    #[cfg(not(feature = "vvdec"))]
    fn test_decoder_factory_vvc_unsupported() {
        let result = DecoderFactory::create(CodecType::H266);
        assert!(result.is_err());
    }

    #[test]
    fn test_available_codecs() {
        let codecs = DecoderFactory::available_codecs();
        assert!(codecs.contains(&CodecType::AV1));

        #[cfg(feature = "ffmpeg")]
        {
            assert!(codecs.contains(&CodecType::H264));
            assert!(codecs.contains(&CodecType::H265));
            assert!(codecs.contains(&CodecType::VP9));
        }

        #[cfg(feature = "vvdec")]
        {
            assert!(codecs.contains(&CodecType::H266));
        }
    }
}
