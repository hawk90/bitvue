//! Error types for H.264/AVC parsing.

use thiserror::Error;

/// AVC parsing error types.
#[derive(Debug, Error)]
pub enum AvcError {
    /// Not enough data to parse.
    #[error("not enough data: expected {expected} bytes, got {got}")]
    NotEnoughData { expected: usize, got: usize },

    /// Invalid NAL unit.
    #[error("invalid NAL unit: {0}")]
    InvalidNalUnit(String),

    /// Invalid SPS.
    #[error("invalid SPS: {0}")]
    InvalidSps(String),

    /// Invalid PPS.
    #[error("invalid PPS: {0}")]
    InvalidPps(String),

    /// Invalid slice header.
    #[error("invalid slice header: {0}")]
    InvalidSliceHeader(String),

    /// Invalid SEI.
    #[error("invalid SEI: {0}")]
    InvalidSei(String),

    /// Missing required parameter set.
    #[error("missing parameter set: {0}")]
    MissingParameterSet(String),

    /// Bitstream error.
    #[error("bitstream error: {0}")]
    BitstreamError(String),

    /// Unsupported feature.
    #[error("unsupported feature: {0}")]
    Unsupported(String),

    /// Generic parse error.
    #[error("parse error: {0}")]
    ParseError(String),
}

/// Result type alias for AVC operations.
pub type Result<T> = std::result::Result<T, AvcError>;

impl From<bitvue_core::CodecError> for AvcError {
    fn from(err: bitvue_core::CodecError) -> Self {
        match err {
            bitvue_core::CodecError::UnexpectedEof { codec: _, position } => {
                AvcError::NotEnoughData {
                    expected: position as usize + 1,
                    got: 0,
                }
            }
            bitvue_core::CodecError::InvalidData { codec: _, message } => {
                AvcError::ParseError(message)
            }
            bitvue_core::CodecError::InsufficientData {
                codec: _,
                expected,
                actual,
            } => AvcError::NotEnoughData {
                expected,
                got: actual,
            },
            bitvue_core::CodecError::Io { codec: _, source } => {
                AvcError::BitstreamError(source.to_string())
            }
            bitvue_core::CodecError::Parse {
                codec: _,
                offset,
                message,
            } => AvcError::ParseError(format!("at offset {}: {}", offset, message)),
            bitvue_core::CodecError::Unsupported { codec: _, feature } => {
                AvcError::Unsupported(feature)
            }
            bitvue_core::CodecError::MissingParameter {
                codec: _,
                parameter,
            } => AvcError::MissingParameterSet(parameter),
            bitvue_core::CodecError::CodecSpecific { codec: _, message } => {
                AvcError::ParseError(message)
            }
        }
    }
}
