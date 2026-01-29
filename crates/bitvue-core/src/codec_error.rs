//! Shared error types for video codec parsing
//!
//! This module provides a unified `CodecError` type that can be used across
//! all video codec parsers (AVC, HEVC, VP9, VVC, AV1, etc.) to eliminate
//! code duplication and provide consistent error handling.

use thiserror::Error;

/// Video codec identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Codec {
    /// H.264/AVC
    Avc,
    /// H.265/HEVC
    Hevc,
    /// VP9
    Vp9,
    /// H.266/VVC
    Vvc,
    /// AV1
    Av1,
    /// MPEG-2
    Mpeg2,
    /// Unknown codec
    Unknown,
}

impl std::fmt::Display for Codec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Codec::Avc => write!(f, "H.264/AVC"),
            Codec::Hevc => write!(f, "H.265/HEVC"),
            Codec::Vp9 => write!(f, "VP9"),
            Codec::Vvc => write!(f, "H.266/VVC"),
            Codec::Av1 => write!(f, "AV1"),
            Codec::Mpeg2 => write!(f, "MPEG-2"),
            Codec::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Unified codec parsing error type
///
/// This error type encompasses common error scenarios across all video codecs,
/// reducing duplication and providing consistent error handling.
#[derive(Error, Debug)]
pub enum CodecError {
    /// Unexpected end of data at the given position
    #[error("Unexpected end of data at position {position} in {codec}")]
    UnexpectedEof {
        /// Codec that encountered the error
        codec: Codec,
        /// Byte offset where EOF occurred
        position: u64,
    },

    /// Invalid data encountered
    #[error("Invalid data in {codec}: {message}")]
    InvalidData {
        /// Codec that encountered the error
        codec: Codec,
        /// Error message
        message: String,
    },

    /// Insufficient data for the requested operation
    #[error("Insufficient data in {codec}: expected {expected} bytes, got {actual}")]
    InsufficientData {
        /// Codec that encountered the error
        codec: Codec,
        /// Expected number of bytes
        expected: usize,
        /// Actual number of bytes available
        actual: usize,
    },

    /// IO error during codec operations
    #[error("IO error in {codec}")]
    Io {
        /// Codec that encountered the error
        codec: Codec,
        /// Underlying IO error
        #[source]
        source: std::io::Error,
    },

    /// Parse error at a specific offset
    #[error("Parse error in {codec} at offset {offset}: {message}")]
    Parse {
        /// Codec that encountered the error
        codec: Codec,
        /// Byte offset where the error occurred
        offset: u64,
        /// Error message
        message: String,
    },

    /// Unsupported feature or parameter
    #[error("Unsupported feature in {codec}: {feature}")]
    Unsupported {
        /// Codec that encountered the error
        codec: Codec,
        /// Description of the unsupported feature
        feature: String,
    },

    /// Missing required parameter set or reference
    #[error("Missing parameter in {codec}: {parameter}")]
    MissingParameter {
        /// Codec that encountered the error
        codec: Codec,
        /// Description of the missing parameter
        parameter: String,
    },

    /// Generic codec-specific error
    #[error("{codec} error: {message}")]
    CodecSpecific {
        /// Codec that encountered the error
        codec: Codec,
        /// Error message
        message: String,
    },
}

impl CodecError {
    /// Create an unexpected EOF error for a specific codec
    pub fn unexpected_eof(codec: Codec, position: u64) -> Self {
        Self::UnexpectedEof { codec, position }
    }

    /// Create an invalid data error for a specific codec
    pub fn invalid_data(codec: Codec, message: impl Into<String>) -> Self {
        Self::InvalidData {
            codec,
            message: message.into(),
        }
    }

    /// Create an insufficient data error for a specific codec
    pub fn insufficient_data(codec: Codec, expected: usize, actual: usize) -> Self {
        Self::InsufficientData {
            codec,
            expected,
            actual,
        }
    }

    /// Create a parse error for a specific codec
    pub fn parse_error(codec: Codec, offset: u64, message: impl Into<String>) -> Self {
        Self::Parse {
            codec,
            offset,
            message: message.into(),
        }
    }

    /// Create an unsupported feature error for a specific codec
    pub fn unsupported(codec: Codec, feature: impl Into<String>) -> Self {
        Self::Unsupported {
            codec,
            feature: feature.into(),
        }
    }

    /// Get the codec associated with this error
    pub fn codec(&self) -> Codec {
        match self {
            Self::UnexpectedEof { codec, .. }
            | Self::InvalidData { codec, .. }
            | Self::InsufficientData { codec, .. }
            | Self::Io { codec, .. }
            | Self::Parse { codec, .. }
            | Self::Unsupported { codec, .. }
            | Self::MissingParameter { codec, .. }
            | Self::CodecSpecific { codec, .. } => *codec,
        }
    }
}

/// Result type alias for codec operations
///
/// Use `codec_error::Result` or `bitvue_core::CodecResult` to avoid ambiguity
/// with `bitvue_core::error::Result` (BitvueError).
pub type CodecResult<T> = std::result::Result<T, CodecError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_codec_display() {
        assert_eq!(Codec::Avc.to_string(), "H.264/AVC");
        assert_eq!(Codec::Hevc.to_string(), "H.265/HEVC");
        assert_eq!(Codec::Vp9.to_string(), "VP9");
        assert_eq!(Codec::Vvc.to_string(), "H.266/VVC");
        assert_eq!(Codec::Av1.to_string(), "AV1");
        assert_eq!(Codec::Mpeg2.to_string(), "MPEG-2");
    }

    #[test]
    fn test_error_creation() {
        let err = CodecError::unexpected_eof(Codec::Avc, 100);
        assert_eq!(err.codec(), Codec::Avc);
        assert!(err.to_string().contains("H.264/AVC"));
        assert!(err.to_string().contains("100"));

        let err = CodecError::invalid_data(Codec::Hevc, "invalid NAL unit");
        assert_eq!(err.codec(), Codec::Hevc);
        assert!(err.to_string().contains("invalid NAL unit"));

        let err = CodecError::insufficient_data(Codec::Vp9, 10, 5);
        assert_eq!(err.codec(), Codec::Vp9);
        assert!(err.to_string().contains("expected 10"));
        assert!(err.to_string().contains("got 5"));

        let err = CodecError::parse_error(Codec::Vvc, 200, "syntax error");
        assert_eq!(err.codec(), Codec::Vvc);
        assert!(err.to_string().contains("offset 200"));
        assert!(err.to_string().contains("syntax error"));

        let err = CodecError::unsupported(Codec::Av1, "4:4:4 chroma subsampling");
        assert_eq!(err.codec(), Codec::Av1);
        assert!(err.to_string().contains("4:4:4 chroma subsampling"));
    }
}
