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
