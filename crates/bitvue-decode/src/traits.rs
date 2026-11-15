//! Decoder trait abstraction for multi-codec support

use crate::decoder::{DecodeError, DecodedFrame, Result};

/// The codec identifier shared by every crate (`bitvue_engine::Codec`).
pub use bitvue_engine::Codec;

/// Codec registry for decoder factory functions
///
/// This registry implements the Open/Closed principle by allowing
/// new codecs to be registered without modifying the factory code.
///
/// # Example
///
/// ```ignore
/// use bitvue_decode::traits::{CodecRegistry, Codec};
/// use std::sync::Mutex;
///
/// // Register a custom decoder
/// CodecRegistry::register(Codec::Custom, Box::new(||
///     Ok(Box::new(MyCustomDecoder::new()) as Box<dyn Decoder>)
/// ));
/// ```
#[derive(Default)]
pub struct CodecRegistry {
    factories:
        std::collections::HashMap<Codec, Box<dyn Fn() -> Result<Box<dyn Decoder>> + Send + Sync>>,
}

impl CodecRegistry {
    /// Get the global codec registry instance
    pub fn global() -> &'static std::sync::Mutex<Self> {
        use std::sync::OnceLock;
        static REGISTRY: OnceLock<std::sync::Mutex<CodecRegistry>> = OnceLock::new();
        REGISTRY.get_or_init(|| std::sync::Mutex::new(Self::default()))
    }

    /// Register a decoder factory function for a codec type
    ///
    /// # Arguments
    ///
    /// * `codec` - The codec type to register
    /// * `factory` - A function that creates a new decoder instance
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if registration succeeded, or an error if a codec
    /// was already registered.
    pub fn register(
        &mut self,
        codec: Codec,
        factory: Box<dyn Fn() -> Result<Box<dyn Decoder>> + Send + Sync>,
    ) -> std::result::Result<(), String> {
        if self.factories.contains_key(&codec) {
            return Err(format!("Codec {:?} is already registered", codec));
        }
        self.factories.insert(codec, factory);
        Ok(())
    }

    /// Create a decoder using a registered factory function
    pub fn create(&self, codec: Codec) -> Result<Box<dyn Decoder>> {
        self.factories.get(&codec).ok_or_else(|| {
            DecodeError::Init(format!("No decoder registered for codec {:?}", codec))
        })?()
    }

    /// Check if a codec is registered
    pub fn is_registered(&self, codec: Codec) -> bool {
        self.factories.contains_key(&codec)
    }

    /// Get all registered codec types
    pub fn registered_codecs(&self) -> Vec<Codec> {
        self.factories.keys().copied().collect()
    }
}

/// Decoder factory function type alias for convenience
pub type DecoderFactoryFn = Box<dyn Fn() -> Result<Box<dyn Decoder>> + Send + Sync>;

/// Decoder capabilities information
#[derive(Debug, Clone)]
pub struct DecoderCapabilities {
    /// Supported codec
    pub codec: Codec,
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
    fn codec_type(&self) -> Codec;

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
    /// Initialize the codec registry with built-in codecs
    ///
    /// This is called automatically on first use, but can be called explicitly
    /// to ensure all built-in codecs are registered.
    fn init_registry() {
        let registry = CodecRegistry::global();
        let mut guard = registry.lock().unwrap();

        // Check if already initialized by checking for AV1
        if guard.is_registered(Codec::Av1) {
            return;
        }

        // Register built-in codecs
        // AV1 is always available
        let _ = guard.register(
            Codec::Av1,
            Box::new(|| {
                let decoder = crate::decoder::Av1Decoder::new()?;
                Ok(Box::new(decoder))
            }),
        );

        // H.264 (requires ffmpeg feature)
        #[cfg(feature = "ffmpeg")]
        {
            let _ = guard.register(
                Codec::Avc,
                Box::new(|| {
                    let decoder = crate::ffmpeg::H264Decoder::new()?;
                    Ok(Box::new(decoder))
                }),
            );
        }

        // H.265 (requires ffmpeg feature)
        #[cfg(feature = "ffmpeg")]
        {
            let _ = guard.register(
                Codec::Hevc,
                Box::new(|| {
                    let decoder = crate::ffmpeg::HevcDecoder::new()?;
                    Ok(Box::new(decoder))
                }),
            );
        }

        // VP9 (requires ffmpeg feature)
        #[cfg(feature = "ffmpeg")]
        {
            let _ = guard.register(
                Codec::Vp9,
                Box::new(|| {
                    let decoder = crate::ffmpeg::Vp9Decoder::new()?;
                    Ok(Box::new(decoder))
                }),
            );
        }

        // H.266/VVC (requires vvdec feature)
        #[cfg(feature = "vvdec")]
        {
            let _ = guard.register(
                Codec::Vvc,
                Box::new(|| {
                    let decoder = crate::vvdec::VvcDecoder::new()?;
                    Ok(Box::new(decoder))
                }),
            );
        }
    }

    /// Register a custom codec decoder
    ///
    /// This allows external code to add new codec decoders without modifying
    /// the bitvue-decode crate, following the Open/Closed principle.
    ///
    /// # Arguments
    ///
    /// * `codec` - The codec type to register
    /// * `factory` - A function that creates a new decoder instance
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if registration succeeded, or an error if a codec
    /// was already registered.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use bitvue_decode::traits::{DecoderFactory, Codec, CodecRegistry};
    ///
    /// DecoderFactory::register_codec(Codec::Custom, || {
    ///     Ok(Box::new(MyCustomDecoder::new()))
    /// });
    /// ```
    pub fn register_codec(
        codec: Codec,
        factory: DecoderFactoryFn,
    ) -> std::result::Result<(), String> {
        let registry = CodecRegistry::global();
        let mut guard = registry.lock().unwrap();
        guard.register(codec, factory)
    }

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
    /// use bitvue_decode::traits::{DecoderFactory, Codec};
    ///
    /// let decoder = DecoderFactory::create(Codec::Av1).unwrap();
    /// ```
    pub fn create(codec: Codec) -> Result<Box<dyn Decoder>> {
        // Ensure registry is initialized with built-in codecs
        Self::init_registry();

        // Try the registry first (for dynamically registered codecs)
        let registry = CodecRegistry::global();
        let guard = registry.lock().unwrap();

        if guard.is_registered(codec) {
            return guard.create(codec);
        }

        // Fall back to builtin implementations
        match codec {
            Codec::Av1 => {
                let decoder = crate::decoder::Av1Decoder::new()?;
                Ok(Box::new(decoder))
            }
            #[cfg(feature = "ffmpeg")]
            Codec::Avc => {
                let decoder = crate::ffmpeg::H264Decoder::new()?;
                Ok(Box::new(decoder))
            }
            #[cfg(not(feature = "ffmpeg"))]
            Codec::Avc => Err(DecodeError::Init(
                "H.264 decoder requires ffmpeg feature (rebuild with --features ffmpeg)"
                    .to_string(),
            )),
            #[cfg(feature = "ffmpeg")]
            Codec::Hevc => {
                let decoder = crate::ffmpeg::HevcDecoder::new()?;
                Ok(Box::new(decoder))
            }
            #[cfg(not(feature = "ffmpeg"))]
            Codec::Hevc => Err(DecodeError::Init(
                "H.265 decoder requires ffmpeg feature (rebuild with --features ffmpeg)"
                    .to_string(),
            )),
            #[cfg(feature = "ffmpeg")]
            Codec::Vp9 => {
                let decoder = crate::ffmpeg::Vp9Decoder::new()?;
                Ok(Box::new(decoder))
            }
            #[cfg(not(feature = "ffmpeg"))]
            Codec::Vp9 => Err(DecodeError::Init(
                "VP9 decoder requires ffmpeg feature (rebuild with --features ffmpeg)".to_string(),
            )),
            #[cfg(feature = "vvdec")]
            Codec::Vvc => {
                let decoder = crate::vvdec::VvcDecoder::new()?;
                Ok(Box::new(decoder))
            }
            #[cfg(not(feature = "vvdec"))]
            Codec::Vvc => Err(DecodeError::Init(
                "VVC decoder requires vvdec feature (rebuild with --features vvdec)".to_string(),
            )),
            other => Err(DecodeError::Init(format!("No decoder for {other}"))),
        }
    }

    /// List all available decoders
    #[allow(unused_mut)]
    pub fn available_codecs() -> Vec<Codec> {
        let mut codecs = vec![Codec::Av1];

        #[cfg(feature = "ffmpeg")]
        {
            codecs.extend([Codec::Avc, Codec::Hevc, Codec::Vp9]);
        }

        #[cfg(feature = "vvdec")]
        {
            codecs.push(Codec::Vvc);
        }

        codecs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_codec_type_display() {
        assert_eq!(format!("{}", Codec::Av1), "AV1");
        assert_eq!(format!("{}", Codec::Avc), "H.264/AVC");
        assert_eq!(format!("{}", Codec::Hevc), "H.265/HEVC");
        assert_eq!(format!("{}", Codec::Vvc), "H.266/VVC");
        assert_eq!(format!("{}", Codec::Vp9), "VP9");
    }

    #[test]
    fn test_decoder_factory_av1() {
        let result = DecoderFactory::create(Codec::Av1);
        assert!(result.is_ok());
        let decoder = result.unwrap();
        assert_eq!(decoder.codec_type(), Codec::Av1);
    }

    #[test]
    #[cfg(feature = "ffmpeg")]
    fn test_decoder_factory_h264() {
        let result = DecoderFactory::create(Codec::Avc);
        assert!(result.is_ok());
        let decoder = result.unwrap();
        assert_eq!(decoder.codec_type(), Codec::Avc);
    }

    #[test]
    #[cfg(feature = "ffmpeg")]
    fn test_decoder_factory_hevc() {
        let result = DecoderFactory::create(Codec::Hevc);
        assert!(result.is_ok());
        let decoder = result.unwrap();
        assert_eq!(decoder.codec_type(), Codec::Hevc);
    }

    #[test]
    #[cfg(not(feature = "ffmpeg"))]
    fn test_decoder_factory_unsupported_without_ffmpeg() {
        let result = DecoderFactory::create(Codec::Avc);
        assert!(result.is_err());

        let result = DecoderFactory::create(Codec::Hevc);
        assert!(result.is_err());
    }

    #[test]
    #[cfg(feature = "ffmpeg")]
    fn test_decoder_factory_vp9() {
        let result = DecoderFactory::create(Codec::Vp9);
        assert!(result.is_ok());
        let decoder = result.unwrap();
        assert_eq!(decoder.codec_type(), Codec::Vp9);
    }

    #[test]
    #[cfg(not(feature = "ffmpeg"))]
    fn test_decoder_factory_vp9_unsupported() {
        let result = DecoderFactory::create(Codec::Vp9);
        assert!(result.is_err());
    }

    #[test]
    #[cfg(feature = "vvdec")]
    fn test_decoder_factory_vvc() {
        let result = DecoderFactory::create(Codec::Vvc);
        assert!(result.is_ok());
        let decoder = result.unwrap();
        assert_eq!(decoder.codec_type(), Codec::Vvc);
    }

    #[test]
    #[cfg(not(feature = "vvdec"))]
    fn test_decoder_factory_vvc_unsupported() {
        let result = DecoderFactory::create(Codec::Vvc);
        assert!(result.is_err());
    }

    #[test]
    fn test_available_codecs() {
        let codecs = DecoderFactory::available_codecs();
        assert!(codecs.contains(&Codec::Av1));

        #[cfg(feature = "ffmpeg")]
        {
            assert!(codecs.contains(&Codec::Avc));
            assert!(codecs.contains(&Codec::Hevc));
            assert!(codecs.contains(&Codec::Vp9));
        }

        #[cfg(feature = "vvdec")]
        {
            assert!(codecs.contains(&Codec::Vvc));
        }
    }

    #[test]
    fn a_codec_without_a_decoder_is_an_error() {
        for codec in [
            Codec::Mpeg2,
            Codec::Avs3,
            Codec::JpegXs,
            Codec::Vc3,
            Codec::Unknown,
        ] {
            let err = DecoderFactory::create(codec).err().expect("no decoder");
            assert!(err.to_string().contains(&codec.to_string()), "{err}");
        }
    }
}
