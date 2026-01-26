//! Decode Service - Frame decoding business logic
//!
//! Simplified stub for now - full decoding will be implemented later.

use std::path::PathBuf;

/// Decode service for handling frame operations
#[allow(dead_code)]
pub struct DecodeService {
    /// Current file path
    file_path: Option<PathBuf>,
    /// Codec type
    codec: String,
}

impl DecodeService {
    /// Create a new decode service
    pub fn new() -> Self {
        Self {
            file_path: None,
            codec: String::new(),
        }
    }

    /// Set the file for decoding
    #[allow(dead_code)]
    pub fn set_file(&mut self, path: PathBuf, codec: String) -> Result<(), String> {
        self.file_path = Some(path);
        self.codec = codec;
        Ok(())
    }

    /// Get the current codec
    #[allow(dead_code)]
    pub fn codec(&self) -> &str {
        &self.codec
    }

    /// Get the current file path
    #[allow(dead_code)]
    pub fn file_path(&self) -> Option<&PathBuf> {
        self.file_path.as_ref()
    }
}

impl Default for DecodeService {
    fn default() -> Self {
        Self::new()
    }
}
