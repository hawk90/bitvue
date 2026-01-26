//! QP Heatmap types (Monster Pack v14 T3-1)

use serde::{Deserialize, Serialize};

/// QP Heatmap request parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QPHeatmapRequest {
    pub frame_index: usize,
    pub resolution: String,  // "quarter", "half", "full"
    pub scale_mode: String,  // "auto", "fixed"
    pub opacity: f32,
}

/// QP Heatmap response with texture data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QPHeatmapResponse {
    pub frame_index: usize,
    pub width: u32,
    pub height: u32,
    pub pixels_base64: String,  // RGBA8 texture data
    pub qp_min: i16,
    pub qp_max: i16,
    pub coverage_percent: f32,
    pub cache_key: String,
    pub success: bool,
    pub error: Option<String>,
}
