//! Tauri Commands
//!
//! This module contains all Tauri commands for IPC between Rust backend and TypeScript frontend.
//!
//! ## Module Structure
//!
//! - `file`: File operations (open, close, get stream info, get frames)
//! - `frame`: Frame data (decoded frames, hex data, analysis)
//! - `thumbnails`: Thumbnail generation
//! - `recent`: Recent files management
//! - `window`: Window management

use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

use bitvue_core::{Core, CompareWorkspace};
use crate::services::{DecodeService, ThumbnailService, RateLimiter};

// Re-export module contents
pub mod analysis;
pub mod compare;
pub mod export;
pub mod file;
pub mod frame;
pub mod log;
pub mod quality;
pub mod syntax;
pub mod thumbnails;
pub mod recent_files;
pub mod window;

// Re-export commonly used types and commands
// Note: These are re-exported for convenience, but lib.rs uses full paths
#[allow(unused_imports)]
pub use analysis::get_frame_analysis;
#[allow(unused_imports)]
pub use file::{open_file, close_file, get_stream_info, get_frames, get_frames_chunk, ChunkedFramesResponse};
#[allow(unused_imports)]
pub use frame::{get_decoded_frame, get_decoded_frame_yuv, get_frame_hex_data, DecodedFrameData, FrameHexData, YUVFrameData};
#[allow(unused_imports)]
pub use thumbnails::get_thumbnails;
#[allow(unused_imports)]
pub use recent_files::{get_recent_files, add_recent_file, clear_recent_files};
#[allow(unused_imports)]
pub use window::close_window;
#[allow(unused_imports)]
pub use compare::{create_compare_workspace, get_aligned_frame, set_sync_mode, set_manual_offset, reset_offset};
#[allow(unused_imports)]
pub use syntax::{get_frame_syntax, SyntaxNode, SyntaxValue};

// =============================================================================
// Shared Types
// =============================================================================

/// File information returned by the open_file command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub path: String,
    pub size: u64,
    pub codec: String,
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Frame data for filmstrip display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameData {
    pub frame_index: usize,
    pub frame_type: String,
    pub size: usize,
    pub poc: Option<i32>,
    pub pts: Option<u64>,
    pub key_frame: Option<bool>,
    /// Display order (POC-based or frame index for simple streams)
    pub display_order: Option<usize>,
    /// Coding order (frame index in bitstream)
    pub coding_order: usize,
    /// Temporal layer ID (for scalable streams)
    pub temporal_id: Option<u8>,
    /// Spatial layer ID (for scalable streams)
    pub spatial_id: Option<u8>,
    /// Reference frame indices (e.g., [0, 3] means references frames 0 and 3)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ref_frames: Option<Vec<usize>>,
    /// Reference slot indices (for scalable coding)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ref_slots: Option<Vec<u8>>,
    /// Duration in time units (pts difference from previous frame)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<u64>,
}

/// Shared application state
pub struct AppState {
    pub core: Arc<Mutex<Core>>,
    #[allow(dead_code)]
    pub decode_service: Arc<Mutex<DecodeService>>,
    pub thumbnail_service: Arc<Mutex<ThumbnailService>>,
    /// Rate limiter for expensive operations (prevents DoS)
    pub rate_limiter: Arc<RateLimiter>,
    /// Compare workspace for A/B comparison
    pub compare_workspace: Arc<Mutex<Option<CompareWorkspace>>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            core: Arc::new(Mutex::new(Core::new())),
            decode_service: Arc::new(Mutex::new(DecodeService::new())),
            thumbnail_service: Arc::new(Mutex::new(ThumbnailService::new())),
            rate_limiter: Arc::new(RateLimiter::new()),
            compare_workspace: Arc::new(Mutex::new(None)),
        }
    }
}

// =============================================================================
// Shared Validation Utilities
// =============================================================================

/// Validate frame index bounds
///
/// This is a shared utility for consistent frame index validation across all commands.
/// Use this after retrieving the total frame count from the stream/units.
///
/// # Arguments
/// * `frame_index` - The frame index to validate
/// * `total_frames` - The total number of frames available
///
/// # Returns
/// * `Ok(())` if the frame index is valid
/// * `Err(String)` with descriptive error message if out of bounds
///
/// # Example
/// ```ignore
/// let total_frames = units.units.len();
/// validate_frame_index_bounds(frame_index, total_frames)?;
/// ```
pub fn validate_frame_index_bounds(frame_index: usize, total_frames: usize) -> Result<(), String> {
    if frame_index >= total_frames {
        return Err(format!(
            "Frame index {} out of bounds (total: {})",
            frame_index, total_frames
        ));
    }
    Ok(())
}

// =============================================================================
// Frame Analysis Types (used by frame module)
// =============================================================================

/// Motion vector data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MotionVectorData {
    pub dx_qpel: i32,  // X component in quarter-pel units
    pub dy_qpel: i32,  // Y component in quarter-pel units
}

/// Block mode for motion vectors
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct BlockModeData {
    pub mode: u8,  // 0=Intra, 1=Inter, 2=Skip
}

/// QP Grid data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QPGridData {
    pub grid_w: u32,
    pub grid_h: u32,
    pub block_w: u32,
    pub block_h: u32,
    pub qp: Vec<i16>,
    pub qp_min: i16,
    pub qp_max: i16,
}

/// MV Grid data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MVGridData {
    pub coded_width: u32,
    pub coded_height: u32,
    pub block_w: u32,
    pub block_h: u32,
    pub grid_w: u32,
    pub grid_h: u32,
    pub mv_l0: Vec<MotionVectorData>,
    pub mv_l1: Vec<MotionVectorData>,
    pub mode: Option<Vec<u8>>,
}

/// Partition block data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartitionBlockData {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub partition: u8,  // PartitionType as u8
    pub depth: u8,
}

/// Partition Grid data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartitionGridData {
    pub coded_width: u32,
    pub coded_height: u32,
    pub sb_size: u32,
    pub blocks: Vec<PartitionBlockData>,
}

/// Prediction Mode Grid data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictionModeGridData {
    pub coded_width: u32,
    pub coded_height: u32,
    pub block_w: u32,
    pub block_h: u32,
    pub grid_w: u32,
    pub grid_h: u32,
    pub modes: Vec<Option<u8>>,
}

/// Transform Grid data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformGridData {
    pub coded_width: u32,
    pub coded_height: u32,
    pub block_w: u32,
    pub block_h: u32,
    pub grid_w: u32,
    pub grid_h: u32,
    pub tx_sizes: Vec<Option<u8>>,
}

/// Frame analysis data response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameAnalysisData {
    pub frame_index: usize,
    pub width: u32,
    pub height: u32,
    pub qp_grid: Option<QPGridData>,
    pub mv_grid: Option<MVGridData>,
    pub partition_grid: Option<PartitionGridData>,
    pub prediction_mode_grid: Option<PredictionModeGridData>,
    pub transform_grid: Option<TransformGridData>,
}

/// Thumbnail data for a single frame
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct ThumbnailData {
    pub frame_index: usize,
    pub thumbnail: String,  // SVG data URL
    pub width: u32,
    pub height: u32,
}

// =============================================================================
// Greeting Command (for testing)
// =============================================================================

/// Maximum length for the name parameter to prevent DoS
const MAX_NAME_LENGTH: usize = 100;

/// Characters to sanitize from the name parameter
const SANITIZE_CHARS: &[char] = &['\r', '\n', '\t', '\x00', '\x1b'];

#[tauri::command]
pub fn greet(name: &str) -> String {
    // SECURITY: Sanitize input to prevent log injection and limit length
    let sanitized = if name.len() > MAX_NAME_LENGTH {
        &name[..MAX_NAME_LENGTH.min(name.len())]
    } else {
        name
    };

    let sanitized: String = sanitized
        .chars()
        .take(MAX_NAME_LENGTH)
        .map(|ch| if SANITIZE_CHARS.contains(&ch) { ' ' } else { ch })
        .collect();

    format!("Hello, {}! You've been greeted from Rust!", sanitized)
}
