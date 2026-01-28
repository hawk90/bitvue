//! VP9 frame extraction
//!
//! Functions for extracting individual frames from VP9 bitstreams

use crate::frame_header::{FrameHeader, FrameType};
use crate::parse_vp9;
use serde::{Deserialize, Serialize};

/// VP9 frame data extracted from the bitstream
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vp9Frame {
    /// Frame index in the stream
    pub frame_index: usize,
    /// Frame type (Key/Inter)
    pub frame_type: Vp9FrameType,
    /// Raw frame data
    pub frame_data: Vec<u8>,
    /// Starting byte position in the stream
    pub offset: usize,
    /// Frame size in bytes
    pub size: usize,
    /// Show frame flag
    pub show_frame: bool,
    /// Frame width
    pub width: u32,
    /// Frame height
    pub height: u32,
    /// Frame header (if available)
    pub frame_header: Option<FrameHeader>,
}

/// VP9 frame type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Vp9FrameType {
    /// Key frame (intra)
    Key,
    /// Inter frame
    Inter,
    /// Unknown frame type
    Unknown,
}

impl Vp9FrameType {
    /// Convert from FrameType
    pub fn from_frame_type(frame_type: FrameType) -> Self {
        match frame_type {
            FrameType::KeyFrame => Vp9FrameType::Key,
            FrameType::InterFrame => Vp9FrameType::Inter,
        }
    }

    /// Get display string
    pub fn as_str(&self) -> &'static str {
        match self {
            Vp9FrameType::Key => "I",
            Vp9FrameType::Inter => "P",
            Vp9FrameType::Unknown => "Unknown",
        }
    }
}

/// Extract frames from VP9 byte stream
///
/// This function parses the byte stream and extracts individual frames.
/// For VP9, frames are either standalone or packed in superframes.
pub fn extract_vp9_frames(data: &[u8]) -> Result<Vec<Vp9Frame>, String> {
    // First, parse the full stream to get frame information
    let stream = parse_vp9(data).map_err(|e| format!("Failed to parse VP9 stream: {:?}", e))?;

    // Extract frame data using superframe parser
    let frame_data_list =
        crate::extract_frames(data).map_err(|e| format!("Failed to extract frames: {:?}", e))?;

    let mut frames = Vec::new();

    let mut current_offset = 0;
    for (idx, (frame_data, frame_header)) in
        frame_data_list.iter().zip(stream.frames.iter()).enumerate()
    {
        let frame_type = Vp9FrameType::from_frame_type(frame_header.frame_type);

        let frame = Vp9Frame {
            frame_index: idx,
            frame_type,
            frame_data: frame_data.to_vec(),
            offset: current_offset,
            size: frame_data.len(),
            show_frame: frame_header.show_frame,
            width: frame_header.width,
            height: frame_header.height,
            frame_header: Some(frame_header.clone()),
        };

        frames.push(frame);
        current_offset += frame_data.len();
    }

    Ok(frames)
}

/// Extract a single frame by index from VP9 byte stream
pub fn extract_frame_at_index(data: &[u8], frame_index: usize) -> Option<Vp9Frame> {
    let frames = extract_vp9_frames(data).ok()?;
    frames.get(frame_index).cloned()
}

/// Convert Vp9Frame to UnitNode format for bitvue-core
pub fn vp9_frame_to_unit_node(frame: &Vp9Frame) -> bitvue_core::UnitNode {
    bitvue_core::UnitNode {
        key: bitvue_core::UnitKey {
            stream: bitvue_core::StreamId::A,
            unit_type: "FRAME".to_string(),
            offset: frame.offset as u64,
            size: frame.size,
        },
        unit_type: "FRAME".to_string(),
        offset: frame.offset as u64,
        size: frame.size,
        frame_index: Some(frame.frame_index),
        frame_type: Some(frame.frame_type.as_str().to_string()),
        pts: Some(frame.frame_index as u64),
        dts: None,
        display_name: format!(
            "Frame {} ({})",
            frame.frame_index,
            frame.frame_type.as_str()
        ),
        children: Vec::new(),
        qp_avg: frame
            .frame_header
            .as_ref()
            .map(|h| h.quantization.base_q_idx),
        mv_grid: None,
        temporal_id: None,
        ref_frames: None,
        ref_slots: None,
    }
}

/// Convert multiple Vp9Frames to UnitNode format
pub fn vp9_frames_to_unit_nodes(frames: &[Vp9Frame]) -> Vec<bitvue_core::UnitNode> {
    frames.iter().map(vp9_frame_to_unit_node).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_empty() {
        let data: &[u8] = &[];
        let frames = extract_vp9_frames(data);
        assert!(frames.is_ok());
        assert!(frames.unwrap().is_empty());
    }

    #[test]
    fn test_frame_type_display() {
        assert_eq!(Vp9FrameType::Key.as_str(), "I");
        assert_eq!(Vp9FrameType::Inter.as_str(), "P");
        assert_eq!(Vp9FrameType::Unknown.as_str(), "Unknown");
    }

    #[test]
    fn test_frame_type_from_frame_type() {
        assert_eq!(
            Vp9FrameType::from_frame_type(FrameType::KeyFrame),
            Vp9FrameType::Key
        );
        assert_eq!(
            Vp9FrameType::from_frame_type(FrameType::InterFrame),
            Vp9FrameType::Inter
        );
    }
}
