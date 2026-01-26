//! Selection-related types

use serde::{Deserialize, Serialize};

/// Selection state response (matches SelectionState structure)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectionStateResponse {
    pub stream_id: String,  // "A" or "B"
    pub temporal: Option<TemporalSelectionResponse>,
    pub cursor: Option<DerivedCursorResponse>,
    pub unit: Option<UnitKeyResponse>,
    pub syntax_node: Option<String>,  // JSON string of SyntaxNodeId
    pub bit_range: Option<BitRangeResponse>,
    pub source_view: Option<String>,
}

/// Temporal selection response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum TemporalSelectionResponse {
    Block { frame_index: usize, block: SpatialBlockResponse },
    Point { frame_index: usize },
    Range { start: usize, end: usize },
    Marker { frame_index: usize },
}

/// Spatial block response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpatialBlockResponse {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

/// Derived cursor response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DerivedCursorResponse {
    pub frame_index: usize,
    pub spatial_pos: Option<(u32, u32)>,
}

/// Unit key response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnitKeyResponse {
    pub stream: String,
    pub unit_type: String,
    pub offset: u64,
    pub size: usize,
}

/// Bit range response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BitRangeResponse {
    pub offset: u64,
    pub length: u64,
}

/// Selection action request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum SelectionActionRequest {
    SelectBlock { frame_index: usize, block: SpatialBlockResponse },
    SelectPoint { frame_index: usize },
    SelectRange { start: usize, end: usize },
    SelectMarker { frame_index: usize },
    SelectFrame { stream: String, frame_index: usize, pts: Option<u64> },
    SelectUnit { stream: String, unit_type: String, offset: u64, size: usize },
    SelectSyntax { node_id: String, bit_range: BitRangeResponse },
    SelectBitRange { offset: u64, length: u64 },
    ClearTemporal,
    ClearAll,
}
