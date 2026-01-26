//! Frame-related types

use serde::{Deserialize, Serialize};

/// Frame data for filmstrip display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameData {
    pub frame_index: usize,
    pub frame_type: String,
    pub offset: u64,
    pub size: usize,
    pub poc: i32,
    pub nal_type: String,
    pub layer: String,
    pub pts: Option<u64>,
    pub dts: Option<u64>,
    pub ref_list: Option<String>,
}

/// Decoded frame data response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecodedFrameResponse {
    pub frame_index: usize,
    pub rgb_data_base64: String,
    pub width: u32,
    pub height: u32,
    pub success: bool,
    pub error: Option<String>,
}

/// Block coding info for overlay visualization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockInfo {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub qp: u8,
    pub prediction_mode: String,
    pub has_mv: bool,
    pub mv_x: i16,
    pub mv_y: i16,
    pub transform_size: String,
}

/// Overlay data response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverlayDataResponse {
    pub frame_index: usize,
    pub width: u32,
    pub height: u32,
    pub blocks: Vec<BlockInfo>,
    pub success: bool,
    pub error: Option<String>,
}

/// Thumbnail data response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThumbnailResponse {
    pub frame_index: usize,
    pub rgb_data_base64: String,
    pub width: u32,
    pub height: u32,
    pub success: bool,
    pub error: Option<String>,
}

/// YUV data response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YuvDataResponse {
    pub frame_index: usize,
    pub width: u32,
    pub height: u32,
    pub y_plane: String,  // base64 encoded
    pub u_plane: String,  // base64 encoded
    pub v_plane: String,  // base64 encoded
    pub success: bool,
    pub error: Option<String>,
}

/// Histogram response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistogramResponse {
    pub frame_index: usize,
    pub y_histogram: Vec<u32>,
    pub u_histogram: Vec<u32>,
    pub v_histogram: Vec<u32>,
    pub y_mean: u32,
    pub u_mean: u32,
    pub v_mean: u32,
    pub success: bool,
    pub error: Option<String>,
}
