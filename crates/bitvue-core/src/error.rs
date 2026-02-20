//! Error types for bitvue

use std::path::PathBuf;
use thiserror::Error;

/// Main error type for bitvue operations
#[derive(Error, Debug)]
pub enum BitvueError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("IO error at {path}: {source}")]
    IoError {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("Parse error at offset {offset}: {message}")]
    Parse { offset: u64, message: String },

    #[error("Invalid OBU type: {0}")]
    InvalidObuType(u8),

    #[error("Unexpected end of data at offset {0}")]
    UnexpectedEof(u64),

    #[error("Unsupported codec: {0}")]
    UnsupportedCodec(String),

    #[error("Decode error: {0}")]
    Decode(String),

    #[error("Insufficient data: needed {needed} bytes, available {available}")]
    InsufficientData { needed: usize, available: usize },

    #[error("Invalid data: {0}")]
    InvalidData(String),

    #[error("Invalid file: {0}")]
    InvalidFile(String),

    #[error("Invalid range: offset={offset}, length={length}")]
    InvalidRange { offset: u64, length: usize },

    #[error("File modified on disk: {path} (old_size={old_size}, new_size={new_size})")]
    FileModified {
        path: PathBuf,
        old_size: u64,
        new_size: u64,
    },

    #[error("Frame not found at display_idx {0}")]
    FrameNotFound(usize),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, BitvueError>;

/// Comprehensive test suite with Arrange-Act-Assert pattern
#[allow(
    unused_imports,
    unused_variables,
    unused_mut,
    dead_code,
    unused_comparisons,
    unused_must_use,
    hidden_glob_reexports,
    unreachable_code,
    non_camel_case_types,
    unused_parens,
    unused_assignments
)]
#[cfg(test)]
mod tests {
    include!("error_test.rs");
}
