//! Parser Strategy - Unified parser interface for all codecs
//!
//! This module provides a strategy pattern for codec-specific parsers,
//! allowing a unified interface for parsing different video codecs.
//!
//! # Example
//!
//! ```ignore
//! use bitvue_codecs::{ParserStrategy, ParserFactory, CodecType};
//!
//! let parser = ParserFactory::create(CodecType::AV1)?;
//! let result = parser.parse_header(data)?;
//! ```

use std::fmt;

/// Codec type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CodecType {
    /// AV1 codec
    AV1,
    /// H.264/AVC codec
    AVC,
    /// H.265/HEVC codec
    HEVC,
    /// H.266/VVC codec
    VVC,
    /// VP9 codec
    VP9,
    /// MPEG2 codec
    MPEG2,
}

impl fmt::Display for CodecType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CodecType::AV1 => write!(f, "AV1"),
            CodecType::AVC => write!(f, "AVC"),
            CodecType::HEVC => write!(f, "HEVC"),
            CodecType::VVC => write!(f, "VVC"),
            CodecType::VP9 => write!(f, "VP9"),
            CodecType::MPEG2 => write!(f, "MPEG2"),
        }
    }
}

/// Parse result containing parsed data and metadata
#[derive(Debug, Clone)]
pub struct ParseResult {
    /// Number of bytes consumed
    pub bytes_consumed: usize,
    /// Number of bits consumed (for partial byte parsing)
    pub bits_consumed: u8,
    /// Parsed unit key (if applicable)
    pub unit_key: Option<u64>,
    /// Parsed frame index (if applicable)
    pub frame_index: Option<usize>,
    /// Additional metadata
    pub metadata: ParseMetadata,
}

/// Metadata from parsing operation
#[derive(Debug, Clone, Default)]
pub struct ParseMetadata {
    /// Frame type (I, P, B)
    pub frame_type: Option<String>,
    /// Picture order count
    pub poc: Option<i32>,
    /// Temporal layer ID
    pub temporal_id: Option<u8>,
    /// Spatial layer ID
    pub spatial_id: Option<u8>,
    /// Quality layer ID
    pub quality_id: Option<u8>,
    /// Reference frame indicators
    pub is_reference: Option<bool>,
    /// Display order
    pub display_order: Option<u64>,
    /// Decode order
    pub decode_order: Option<u64>,
    /// PTS (Presentation Timestamp)
    pub pts: Option<i64>,
    /// DTS (Decode Timestamp)
    pub dts: Option<i64>,
}

impl ParseResult {
    /// Create a new parse result
    pub fn new(bytes_consumed: usize) -> Self {
        Self {
            bytes_consumed,
            bits_consumed: 0,
            unit_key: None,
            frame_index: None,
            metadata: ParseMetadata::default(),
        }
    }

    /// Set bits consumed
    pub fn with_bits_consumed(mut self, bits: u8) -> Self {
        self.bits_consumed = bits;
        self
    }

    /// Set unit key
    pub fn with_unit_key(mut self, key: u64) -> Self {
        self.unit_key = Some(key);
        self
    }

    /// Set frame index
    pub fn with_frame_index(mut self, index: usize) -> Self {
        self.frame_index = Some(index);
        self
    }

    /// Set metadata
    pub fn with_metadata(mut self, metadata: ParseMetadata) -> Self {
        self.metadata = metadata;
        self
    }
}

/// Parser capabilities
#[derive(Debug, Clone)]
pub struct ParserCapabilities {
    /// Supported codec type
    pub codec_type: CodecType,
    /// Supports incremental parsing
    pub incremental_parsing: bool,
    /// Supports seeking within stream
    pub seeking: bool,
    /// Maximum frame size supported (0 = unlimited)
    pub max_frame_size: usize,
    /// Supports multiple temporal layers
    pub temporal_layers: bool,
    /// Supports multiple spatial layers
    pub spatial_layers: bool,
    /// Supports multiple quality layers
    pub quality_layers: bool,
}

/// Parse error types
#[derive(Debug, Clone)]
pub enum ParseError {
    /// Invalid data format
    InvalidData { message: String },
    /// Unsupported feature
    UnsupportedFeature { feature: String },
    /// Insufficient data
    InsufficientData { required: usize, available: usize },
    /// Header parsing failed
    HeaderError { message: String },
    /// Frame parsing failed
    FrameError { message: String },
    /// Bitstream syntax error
    SyntaxError { message: String },
    /// Out of range value
    OutOfRange { value: String, range: String },
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidData { message } => write!(f, "Invalid data: {}", message),
            Self::UnsupportedFeature { feature } => write!(f, "Unsupported feature: {}", feature),
            Self::InsufficientData { required, available } => {
                write!(f, "Insufficient data: required {}, available {}", required, available)
            }
            Self::HeaderError { message } => write!(f, "Header error: {}", message),
            Self::FrameError { message } => write!(f, "Frame error: {}", message),
            Self::SyntaxError { message } => write!(f, "Syntax error: {}", message),
            Self::OutOfRange { value, range } => write!(f, "Out of range: {}, range: {}", value, range),
        }
    }
}

impl std::error::Error for ParseError {}

/// Result type for parsing operations
pub type ParseResultType<T> = Result<T, ParseError>;

/// Parser state for incremental parsing
#[derive(Debug, Clone, Default)]
pub struct ParserState {
    /// Current offset in the stream
    pub offset: u64,
    /// Current frame index
    pub frame_index: usize,
    /// Accumulated bytes (for multi-OBU/frame parsing)
    pub accumulated_bytes: Vec<u8>,
    /// Parsing state flags
    pub flags: ParserStateFlags,
}

/// Flags for parser state
#[derive(Debug, Clone, Copy, Default)]
pub struct ParserStateFlags {
    /// End of stream reached
    pub eos: bool,
    /// Header parsed
    pub header_parsed: bool,
    /// In frame parsing
    pub in_frame: bool,
    /// Error state
    pub error: bool,
}

/// Trait for parser strategies
///
/// This trait defines the interface for codec-specific parsers.
/// Each codec implements this trait to provide its parsing logic.
pub trait ParserStrategy: Send + Sync {
    /// Get the codec type
    fn codec_type(&self) -> CodecType;

    /// Get parser capabilities
    fn capabilities(&self) -> ParserCapabilities;

    /// Get current parser state
    fn state(&self) -> &ParserState;

    /// Reset parser state
    fn reset(&mut self);

    /// Parse header from data
    ///
    /// Parses the container/file header to determine stream properties.
    /// Returns the number of bytes consumed.
    fn parse_header(&mut self, data: &[u8]) -> ParseResultType<ParseResult>;

    /// Parse a single frame from data
    ///
    /// Parses one frame/obu from the input data.
    /// Returns the number of bytes consumed and frame metadata.
    fn parse_frame(&mut self, data: &[u8]) -> ParseResultType<ParseResult>;

    /// Parse multiple frames from data
    ///
    /// Convenience method that parses frames until data is exhausted.
    fn parse_frames(&mut self, data: &[u8]) -> ParseResultType<Vec<ParseResult>> {
        let mut results = Vec::new();
        let mut offset = 0;

        while offset < data.len() {
            let result = self.parse_frame(&data[offset..])?;
            offset += result.bytes_consumed;

            // Check for end of stream
            if self.state().flags.eos {
                break;
            }

            results.push(result);
        }

        Ok(results)
    }

    /// Seek to a specific byte offset
    ///
    /// Only works if the parser supports seeking.
    fn seek(&mut self, offset: u64) -> ParseResultType<()>;

    /// Get total number of frames parsed so far
    fn frame_count(&self) -> usize {
        self.state().frame_index
    }

    /// Check if parsing is complete
    fn is_complete(&self) -> bool {
        self.state().flags.eos
    }
}

// =============================================================================
// Base Parser Implementation
// =============================================================================

/// Base parser providing common functionality
#[derive(Debug)]
pub struct BaseParser {
    codec_type: CodecType,
    state: ParserState,
    capabilities: ParserCapabilities,
}

impl BaseParser {
    /// Create a new base parser
    pub fn new(codec_type: CodecType, capabilities: ParserCapabilities) -> Self {
        Self {
            codec_type,
            state: ParserState::default(),
            capabilities,
        }
    }

    /// Create parser with default capabilities for a codec type
    pub fn with_codec_type(codec_type: CodecType) -> Self {
        let capabilities = ParserCapabilities {
            codec_type,
            incremental_parsing: true,
            seeking: false,
            max_frame_size: 0, // Unlimited
            temporal_layers: matches!(codec_type, CodecType::AV1 | CodecType::VP9 | CodecType::VVC),
            spatial_layers: matches!(codec_type, CodecType::AV1 | CodecType::VP9 | CodecType::VVC),
            quality_layers: matches!(codec_type, CodecType::AV1 | CodecType::VP9),
        };

        Self {
            codec_type,
            state: ParserState::default(),
            capabilities,
        }
    }
}

impl ParserStrategy for BaseParser {
    fn codec_type(&self) -> CodecType {
        self.codec_type
    }

    fn capabilities(&self) -> ParserCapabilities {
        self.capabilities.clone()
    }

    fn state(&self) -> &ParserState {
        &self.state
    }

    fn reset(&mut self) {
        self.state = ParserState::default();
    }

    fn parse_header(&mut self, _data: &[u8]) -> ParseResultType<ParseResult> {
        // Base implementation - to be overridden by codec-specific parsers
        Err(ParseError::UnsupportedFeature {
            feature: "parse_header not implemented for BaseParser".to_string(),
        })
    }

    fn parse_frame(&mut self, _data: &[u8]) -> ParseResultType<ParseResult> {
        // Base implementation - to be overridden by codec-specific parsers
        Err(ParseError::UnsupportedFeature {
            feature: "parse_frame not implemented for BaseParser".to_string(),
        })
    }

    fn seek(&mut self, offset: u64) -> ParseResultType<()> {
        if !self.capabilities.seeking {
            return Err(ParseError::UnsupportedFeature {
                feature: "seeking not supported".to_string(),
            });
        }
        self.state.offset = offset;
        Ok(())
    }
}

// =============================================================================
// AV1 Parser Strategy
// =============================================================================

/// AV1 parser strategy
///
/// Implements AV1-specific parsing logic.
#[derive(Debug)]
pub struct Av1ParserStrategy {
    base: BaseParser,
    // AV1-specific fields would go here
    sequence_header_parsed: bool,
}

impl Av1ParserStrategy {
    /// Create a new AV1 parser
    pub fn new() -> Self {
        Self {
            base: BaseParser::with_codec_type(CodecType::AV1),
            sequence_header_parsed: false,
        }
    }
}

impl Default for Av1ParserStrategy {
    fn default() -> Self {
        Self::new()
    }
}

impl ParserStrategy for Av1ParserStrategy {
    fn codec_type(&self) -> CodecType {
        self.base.codec_type()
    }

    fn capabilities(&self) -> ParserCapabilities {
        self.base.capabilities()
    }

    fn state(&self) -> &ParserState {
        self.base.state()
    }

    fn reset(&mut self) {
        self.base.reset();
        self.sequence_header_parsed = false;
    }

    fn parse_header(&mut self, data: &[u8]) -> ParseResultType<ParseResult> {
        // Check for IVF header
        if data.len() < 32 {
            return Err(ParseError::InsufficientData {
                required: 32,
                available: data.len(),
            });
        }

        // IVF signature
        if &data[0..4] == b"DKIF" {
            // Parse IVF header
            let width = u16::from_le_bytes([data[12], data[13]]) as u32;
            let height = u16::from_le_bytes([data[14], data[15]]) as u32;
            let timebase_den = u32::from_le_bytes([data[16], data[17], data[18], data[19]]);
            let timebase_num = u32::from_le_bytes([data[20], data[21], data[22], data[23]]);
            let num_frames = u32::from_le_bytes([data[24], data[25], data[26], data[27]]);

            Ok(ParseResult::new(32).with_metadata(ParseMetadata {
                display_order: Some(0),
                decode_order: Some(0),
                ..Default::default()
            }))
        } else {
            // Assume Annex-B format (OBUs without IVF container)
            Ok(ParseResult::new(0))
        }
    }

    fn parse_frame(&mut self, data: &[u8]) -> ParseResultType<ParseResult> {
        if data.is_empty() {
            self.base.state.flags.eos = true;
            return Ok(ParseResult::new(0));
        }

        // Parse OBU header
        if data.len() < 2 {
            return Err(ParseError::InsufficientData {
                required: 2,
                available: data.len(),
            });
        }

        let obu_type = (data[0] & 0x78) >> 3;
        let obu_extension = (data[0] & 0x04) != 0;
        let obu_has_size_field = (data[0] & 0x02) != 0;

        // Check for sequence header
        if obu_type == 1 {
            // Sequence Header OBU
            self.sequence_header_parsed = true;
            self.base.state.flags.header_parsed = true;
        }

        // For now, return a simple result
        // Full AV1 parsing would be implemented here
        let bytes_consumed = std::cmp::min(data.len(), 100); // Placeholder
        self.base.state.frame_index += 1;
        self.base.state.offset += bytes_consumed as u64;

        Ok(ParseResult::new(bytes_consumed).with_frame_index(self.base.state.frame_index - 1))
    }

    fn seek(&mut self, offset: u64) -> ParseResultType<()> {
        self.base.seek(offset)
    }
}

// =============================================================================
// AVC (H.264) Parser Strategy
// =============================================================================

/// AVC parser strategy
///
/// Implements H.264-specific parsing logic.
#[derive(Debug)]
pub struct AvcParserStrategy {
    base: BaseParser,
    // AVC-specific fields
    nal_prefix_length: usize,
}

impl AvcParserStrategy {
    /// Create a new AVC parser
    pub fn new() -> Self {
        Self {
            base: BaseParser::with_codec_type(CodecType::AVC),
            nal_prefix_length: 4, // Default to 4-byte NAL prefixes
        }
    }

    /// Set NAL prefix length (3 or 4 bytes)
    pub fn with_nal_prefix_length(mut self, length: usize) -> Self {
        self.nal_prefix_length = if length == 3 || length == 4 { length } else { 4 };
        self
    }
}

impl Default for AvcParserStrategy {
    fn default() -> Self {
        Self::new()
    }
}

impl ParserStrategy for AvcParserStrategy {
    fn codec_type(&self) -> CodecType {
        self.base.codec_type()
    }

    fn capabilities(&self) -> ParserCapabilities {
        self.base.capabilities()
    }

    fn state(&self) -> &ParserState {
        self.base.state()
    }

    fn reset(&mut self) {
        self.base.reset();
    }

    fn parse_header(&mut self, data: &[u8]) -> ParseResultType<ParseResult> {
        // Look for SPS (Sequence Parameter Set)
        // SPS NAL unit type is 7
        // For now, just return success
        Ok(ParseResult::new(0))
    }

    fn parse_frame(&mut self, data: &[u8]) -> ParseResultType<ParseResult> {
        if data.is_empty() {
            self.base.state.flags.eos = true;
            return Ok(ParseResult::new(0));
        }

        // Find NAL unit start code
        let start_code_prefix: &[u8] = if self.nal_prefix_length == 4 {
            &[0, 0, 0, 1]
        } else {
            &[0, 0, 1]
        };

        // Find first NAL unit
        let start = data
            .windows(start_code_prefix.len())
            .position(|w| w == start_code_prefix)
            .unwrap_or(0);

        if start + start_code_prefix.len() >= data.len() {
            return Err(ParseError::InsufficientData {
                required: start_code_prefix.len() + 1,
                available: data.len(),
            });
        }

        let nal_data = &data[start + start_code_prefix.len()..];
        let nal_type = nal_data[0] & 0x1F;

        // Check for IDR (type 5) or non-IDR slice (type 1)
        let is_frame = matches!(nal_type, 1 | 5);
        if is_frame {
            self.base.state.frame_index += 1;
        }

        let bytes_consumed = data.len().min(100); // Placeholder
        self.base.state.offset += bytes_consumed as u64;

        Ok(ParseResult::new(bytes_consumed))
    }

    fn seek(&mut self, offset: u64) -> ParseResultType<()> {
        self.base.seek(offset)
    }
}

// =============================================================================
// HEVC (H.265) Parser Strategy
// =============================================================================

/// HEVC parser strategy
///
/// Implements H.265-specific parsing logic.
#[derive(Debug)]
pub struct HevcParserStrategy {
    base: BaseParser,
}

impl HevcParserStrategy {
    /// Create a new HEVC parser
    pub fn new() -> Self {
        Self {
            base: BaseParser::with_codec_type(CodecType::HEVC),
        }
    }
}

impl Default for HevcParserStrategy {
    fn default() -> Self {
        Self::new()
    }
}

impl ParserStrategy for HevcParserStrategy {
    fn codec_type(&self) -> CodecType {
        self.base.codec_type()
    }

    fn capabilities(&self) -> ParserCapabilities {
        self.base.capabilities()
    }

    fn state(&self) -> &ParserState {
        self.base.state()
    }

    fn reset(&mut self) {
        self.base.reset();
    }

    fn parse_header(&mut self, data: &[u8]) -> ParseResultType<ParseResult> {
        Ok(ParseResult::new(0))
    }

    fn parse_frame(&mut self, data: &[u8]) -> ParseResultType<ParseResult> {
        if data.is_empty() {
            self.base.state.flags.eos = true;
            return Ok(ParseResult::new(0));
        }

        // HEVC NAL unit parsing
        // Similar structure to AVC but with different NAL types
        let bytes_consumed = data.len().min(100); // Placeholder
        self.base.state.frame_index += 1;
        self.base.state.offset += bytes_consumed as u64;

        Ok(ParseResult::new(bytes_consumed).with_frame_index(self.base.state.frame_index - 1))
    }

    fn seek(&mut self, offset: u64) -> ParseResultType<()> {
        self.base.seek(offset)
    }
}

// =============================================================================
// VVC (H.266) Parser Strategy
// =============================================================================

/// VVC parser strategy
///
/// Implements H.266-specific parsing logic.
#[derive(Debug)]
pub struct VvcParserStrategy {
    base: BaseParser,
}

impl VvcParserStrategy {
    /// Create a new VVC parser
    pub fn new() -> Self {
        Self {
            base: BaseParser::with_codec_type(CodecType::VVC),
        }
    }
}

impl Default for VvcParserStrategy {
    fn default() -> Self {
        Self::new()
    }
}

impl ParserStrategy for VvcParserStrategy {
    fn codec_type(&self) -> CodecType {
        self.base.codec_type()
    }

    fn capabilities(&self) -> ParserCapabilities {
        self.base.capabilities()
    }

    fn state(&self) -> &ParserState {
        self.base.state()
    }

    fn reset(&mut self) {
        self.base.reset();
    }

    fn parse_header(&mut self, data: &[u8]) -> ParseResultType<ParseResult> {
        Ok(ParseResult::new(0))
    }

    fn parse_frame(&mut self, data: &[u8]) -> ParseResultType<ParseResult> {
        if data.is_empty() {
            self.base.state.flags.eos = true;
            return Ok(ParseResult::new(0));
        }

        let bytes_consumed = data.len().min(100); // Placeholder
        self.base.state.frame_index += 1;
        self.base.state.offset += bytes_consumed as u64;

        Ok(ParseResult::new(bytes_consumed).with_frame_index(self.base.state.frame_index - 1))
    }

    fn seek(&mut self, offset: u64) -> ParseResultType<()> {
        self.base.seek(offset)
    }
}

// =============================================================================
// VP9 Parser Strategy
// =============================================================================

/// VP9 parser strategy
///
/// Implements VP9-specific parsing logic.
#[derive(Debug)]
pub struct Vp9ParserStrategy {
    base: BaseParser,
}

impl Vp9ParserStrategy {
    /// Create a new VP9 parser
    pub fn new() -> Self {
        Self {
            base: BaseParser::with_codec_type(CodecType::VP9),
        }
    }
}

impl Default for Vp9ParserStrategy {
    fn default() -> Self {
        Self::new()
    }
}

impl ParserStrategy for Vp9ParserStrategy {
    fn codec_type(&self) -> CodecType {
        self.base.codec_type()
    }

    fn capabilities(&self) -> ParserCapabilities {
        self.base.capabilities()
    }

    fn state(&self) -> &ParserState {
        self.base.state()
    }

    fn reset(&mut self) {
        self.base.reset();
    }

    fn parse_header(&mut self, data: &[u8]) -> ParseResultType<ParseResult> {
        Ok(ParseResult::new(0))
    }

    fn parse_frame(&mut self, data: &[u8]) -> ParseResultType<ParseResult> {
        if data.is_empty() {
            self.base.state.flags.eos = true;
            return Ok(ParseResult::new(0));
        }

        let bytes_consumed = data.len().min(100); // Placeholder
        self.base.state.frame_index += 1;
        self.base.state.offset += bytes_consumed as u64;

        Ok(ParseResult::new(bytes_consumed).with_frame_index(self.base.state.frame_index - 1))
    }

    fn seek(&mut self, offset: u64) -> ParseResultType<()> {
        self.base.seek(offset)
    }
}

// =============================================================================
// Parser Factory
// =============================================================================

/// Factory for creating codec-specific parsers
pub struct ParserFactory;

impl ParserFactory {
    /// Create a parser for the specified codec type
    pub fn create(codec_type: CodecType) -> ParseResultType<Box<dyn ParserStrategy>> {
        match codec_type {
            CodecType::AV1 => Ok(Box::new(Av1ParserStrategy::new())),
            CodecType::AVC => Ok(Box::new(AvcParserStrategy::new())),
            CodecType::HEVC => Ok(Box::new(HevcParserStrategy::new())),
            CodecType::VVC => Ok(Box::new(VvcParserStrategy::new())),
            CodecType::VP9 => Ok(Box::new(Vp9ParserStrategy::new())),
            CodecType::MPEG2 => Err(ParseError::UnsupportedFeature {
                feature: "MPEG2 parser not yet implemented".to_string(),
            }),
        }
    }

    /// Get list of supported codec types
    pub fn supported_codecs() -> Vec<CodecType> {
        vec![
            CodecType::AV1,
            CodecType::AVC,
            CodecType::HEVC,
            CodecType::VVC,
            CodecType::VP9,
        ]
    }

    /// Check if a codec type is supported
    pub fn is_supported(codec_type: CodecType) -> bool {
        !matches!(codec_type, CodecType::MPEG2)
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_codec_type_display() {
        assert_eq!(CodecType::AV1.to_string(), "AV1");
        assert_eq!(CodecType::AVC.to_string(), "AVC");
        assert_eq!(CodecType::HEVC.to_string(), "HEVC");
        assert_eq!(CodecType::VVC.to_string(), "VVC");
        assert_eq!(CodecType::VP9.to_string(), "VP9");
    }

    #[test]
    fn test_parse_result_builder() {
        let result = ParseResult::new(100)
            .with_bits_consumed(7)
            .with_unit_key(42)
            .with_frame_index(5);

        assert_eq!(result.bytes_consumed, 100);
        assert_eq!(result.bits_consumed, 7);
        assert_eq!(result.unit_key, Some(42));
        assert_eq!(result.frame_index, Some(5));
    }

    #[test]
    fn test_parse_error_display() {
        let error = ParseError::InvalidData {
            message: "test error".to_string(),
        };
        assert_eq!(error.to_string(), "Invalid data: test error");

        let error = ParseError::InsufficientData {
            required: 100,
            available: 50,
        };
        assert_eq!(error.to_string(), "Insufficient data: required 100, available 50");
    }

    #[test]
    fn test_av1_parser_creation() {
        let parser = ParserFactory::create(CodecType::AV1).unwrap();
        assert_eq!(parser.codec_type(), CodecType::AV1);
        assert_eq!(parser.frame_count(), 0);
        assert!(!parser.is_complete());
    }

    #[test]
    fn test_avc_parser_creation() {
        let parser = ParserFactory::create(CodecType::AVC).unwrap();
        assert_eq!(parser.codec_type(), CodecType::AVC);
    }

    #[test]
    fn test_hevc_parser_creation() {
        let parser = ParserFactory::create(CodecType::HEVC).unwrap();
        assert_eq!(parser.codec_type(), CodecType::HEVC);
    }

    #[test]
    fn test_vvc_parser_creation() {
        let parser = ParserFactory::create(CodecType::VVC).unwrap();
        assert_eq!(parser.codec_type(), CodecType::VVC);
    }

    #[test]
    fn test_vp9_parser_creation() {
        let parser = ParserFactory::create(CodecType::VP9).unwrap();
        assert_eq!(parser.codec_type(), CodecType::VP9);
    }

    #[test]
    fn test_unsupported_codec() {
        let result = ParserFactory::create(CodecType::MPEG2);
        assert!(result.is_err());
    }

    #[test]
    fn test_supported_codecs() {
        let codecs = ParserFactory::supported_codecs();
        assert_eq!(codecs.len(), 5);
        assert!(codecs.contains(&CodecType::AV1));
        assert!(codecs.contains(&CodecType::AVC));
        assert!(codecs.contains(&CodecType::HEVC));
        assert!(codecs.contains(&CodecType::VVC));
        assert!(codecs.contains(&CodecType::VP9));
    }

    #[test]
    fn test_parser_reset() {
        let mut parser = Av1ParserStrategy::new();
        // Simulate some parsing
        parser.base.state.frame_index = 10;
        parser.base.state.offset = 1000;

        parser.reset();
        assert_eq!(parser.frame_count(), 0);
        assert_eq!(parser.state().offset, 0);
    }

    #[test]
    fn test_empty_data_handling() {
        let mut parser = Av1ParserStrategy::new();
        let result = parser.parse_frame(&[]).unwrap();
        assert_eq!(result.bytes_consumed, 0);
        assert!(parser.is_complete());
    }

    #[test]
    fn test_insufficient_data_error() {
        let mut parser = Av1ParserStrategy::new();
        let data = [0u8; 1]; // Less than minimum required
        let result = parser.parse_header(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_av1_with_nal_prefix_length() {
        let parser = AvcParserStrategy::new().with_nal_prefix_length(3);
        // Parser created successfully
        assert_eq!(parser.codec_type(), CodecType::AVC);
    }
}
