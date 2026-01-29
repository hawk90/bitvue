//! Decoder trait abstraction for multi-codec support

use crate::decoder::{DecodeError, DecodedFrame, Result};

/// Codec type enumeration for decoder selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

/// Codec registry for decoder factory functions
///
/// This registry implements the Open/Closed principle by allowing
/// new codecs to be registered without modifying the factory code.
///
/// # Example
///
/// ```ignore
/// use bitvue_decode::traits::{CodecRegistry, CodecType};
/// use std::sync::Mutex;
///
/// // Register a custom decoder
/// CodecRegistry::register(CodecType::Custom, Box::new(||
///     Ok(Box::new(MyCustomDecoder::new()) as Box<dyn Decoder>)
/// ));
/// ```
pub struct CodecRegistry {
    factories: std::collections::HashMap<CodecType, Box<dyn Fn() -> Result<Box<dyn Decoder>> + Send + Sync>>,
}

impl CodecRegistry {
    /// Get the global codec registry instance
    pub fn global() -> &'static std::sync::Mutex<Self> {
        use std::sync::OnceLock;
        static REGISTRY: OnceLock<std::sync::Mutex<CodecRegistry>> = OnceLock::new();
        REGISTRY.get_or_init(|| {
            std::sync::Mutex::new(Self::default())
        })
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
        codec: CodecType,
        factory: Box<dyn Fn() -> Result<Box<dyn Decoder>> + Send + Sync>,
    ) -> std::result::Result<(), String> {
        if self.factories.contains_key(&codec) {
            return Err(format!("Codec {:?} is already registered", codec));
        }
        self.factories.insert(codec, factory);
        Ok(())
    }

    /// Create a decoder using a registered factory function
    pub fn create(&self, codec: CodecType) -> Result<Box<dyn Decoder>> {
        self.factories
            .get(&codec)
            .ok_or_else(|| {
                DecodeError::Init(format!("No decoder registered for codec {:?}", codec))
            })?
            ()
    }

    /// Check if a codec is registered
    pub fn is_registered(&self, codec: CodecType) -> bool {
        self.factories.contains_key(&codec)
    }

    /// Get all registered codec types
    pub fn registered_codecs(&self) -> Vec<CodecType> {
        self.factories.keys().copied().collect()
    }
}

impl Default for CodecRegistry {
    fn default() -> Self {
        Self {
            factories: std::collections::HashMap::new(),
        }
    }
}

/// Decoder factory function type alias for convenience
pub type DecoderFactoryFn = Box<dyn Fn() -> Result<Box<dyn Decoder>> + Send + Sync>;

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
    /// Initialize the codec registry with built-in codecs
    ///
    /// This is called automatically on first use, but can be called explicitly
    /// to ensure all built-in codecs are registered.
    fn init_registry() {
        let registry = CodecRegistry::global();
        let mut guard = registry.lock().unwrap();

        // Check if already initialized by checking for AV1
        if guard.is_registered(CodecType::AV1) {
            return;
        }

        // Register built-in codecs
        // AV1 is always available
        let _ = guard.register(CodecType::AV1, Box::new(|| {
            let decoder = crate::decoder::Av1Decoder::new()?;
            Ok(Box::new(decoder))
        }));

        // H.264 (requires ffmpeg feature)
        #[cfg(feature = "ffmpeg")]
        {
            let _ = guard.register(CodecType::H264, Box::new(|| {
                let decoder = crate::ffmpeg::H264Decoder::new()?;
                Ok(Box::new(decoder))
            }));
        }

        // H.265 (requires ffmpeg feature)
        #[cfg(feature = "ffmpeg")]
        {
            let _ = guard.register(CodecType::H265, Box::new(|| {
                let decoder = crate::ffmpeg::HevcDecoder::new()?;
                Ok(Box::new(decoder))
            }));
        }

        // VP9 (requires ffmpeg feature)
        #[cfg(feature = "ffmpeg")]
        {
            let _ = guard.register(CodecType::VP9, Box::new(|| {
                let decoder = crate::ffmpeg::Vp9Decoder::new()?;
                Ok(Box::new(decoder))
            }));
        }

        // H.266/VVC (requires vvdec feature)
        #[cfg(feature = "vvdec")]
        {
            let _ = guard.register(CodecType::H266, Box::new(|| {
                let decoder = crate::vvdec::VvcDecoder::new()?;
                Ok(Box::new(decoder))
            }));
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
    /// use bitvue_decode::traits::{DecoderFactory, CodecType, CodecRegistry};
    ///
    /// DecoderFactory::register_codec(CodecType::Custom, || {
    ///     Ok(Box::new(MyCustomDecoder::new()))
    /// });
    /// ```
    pub fn register_codec(codec: CodecType, factory: DecoderFactoryFn) -> std::result::Result<(), String> {
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
    /// use bitvue_decode::traits::{DecoderFactory, CodecType};
    ///
    /// let decoder = DecoderFactory::create(CodecType::AV1).unwrap();
    /// ```
    pub fn create(codec: CodecType) -> Result<Box<dyn Decoder>> {
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
