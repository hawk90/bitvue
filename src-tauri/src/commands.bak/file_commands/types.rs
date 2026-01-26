//! File-related types

use serde::{Deserialize, Serialize};

/// File information response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub path: String,
    pub size: u64,
    pub codec: String,
    pub success: bool,
    pub error: Option<String>,
}

/// Hex data response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HexDataResponse {
    pub offset: u64,
    pub size: usize,
    pub hex_data: String,
    pub ascii_data: String,
    pub success: bool,
    pub error: Option<String>,
}
