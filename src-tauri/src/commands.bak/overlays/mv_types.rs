//! Motion Vector overlay types (Monster Pack v14 T3-2)

use bitvue_core::mv_overlay::Viewport;
use serde::{Deserialize, Serialize};

/// Motion Vector overlay request parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MVOverlayRequest {
    pub frame_index: usize,
    pub viewport: ViewportParams,
    pub layer: String,  // "L0", "L1", "both"
    pub user_scale: f32,
    pub opacity: f32,
}

/// Viewport parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewportParams {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl From<ViewportParams> for Viewport {
    fn from(params: ViewportParams) -> Self {
        Viewport::new(params.x, params.y, params.width, params.height)
    }
}

/// Motion vector data for rendering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MVRenderData {
    pub block_x: u32,
    pub block_y: u32,
    pub center_x: f32,
    pub center_y: f32,
    pub mv_l0_dx: Option<f32>,  // pixels
    pub mv_l0_dy: Option<f32>,
    pub mv_l1_dx: Option<f32>,
    pub mv_l1_dy: Option<f32>,
    pub is_intra: bool,
}

/// MV Overlay response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MVOverlayResponse {
    pub frame_index: usize,
    pub vectors: Vec<MVRenderData>,
    pub stride: u32,
    pub total_blocks: usize,
    pub visible_blocks: usize,
    pub drawn_blocks: usize,
    pub statistics: MVStatsResponse,
    pub cache_key: String,
    pub success: bool,
    pub error: Option<String>,
}

/// MV Statistics response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MVStatsResponse {
    pub total_blocks: usize,
    pub l0_present: usize,
    pub l1_present: usize,
    pub l0_avg_magnitude: f32,
    pub l1_avg_magnitude: f32,
    pub l0_max_magnitude: f32,
    pub l1_max_magnitude: f32,
}
