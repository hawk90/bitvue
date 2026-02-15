//! VP9 frame extraction
//!
//! Functions for extracting individual frames from VP9 bitstreams

use crate::frame_header::{FrameHeader, FrameType};
use crate::parse_vp9;
use bitvue_core::BitvueError;
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

impl Vp9Frame {
    /// Creates a new Vp9FrameBuilder for constructing Vp9Frame instances
    pub fn builder() -> Vp9FrameBuilder {
        Vp9FrameBuilder::default()
    }
}

/// Builder for constructing Vp9Frame instances
///
/// # Example
///
/// ```
/// use bitvue_vp9::frames::{Vp9Frame, Vp9FrameType};
///
/// let frame = Vp9Frame::builder()
///     .frame_index(0)
///     .frame_type(Vp9FrameType::Key)
///     .frame_data(vec![0x00, 0x00, 0x01])
///     .offset(0)
///     .size(3)
///     .show_frame(true)
///     .width(1920)
///     .height(1080)
///     .build();
/// ```
#[derive(Debug, Default)]
pub struct Vp9FrameBuilder {
    frame_index: Option<usize>,
    frame_type: Option<Vp9FrameType>,
    frame_data: Option<Vec<u8>>,
    offset: Option<usize>,
    size: Option<usize>,
    show_frame: Option<bool>,
    width: Option<u32>,
    height: Option<u32>,
    frame_header: Option<FrameHeader>,
}

impl Vp9FrameBuilder {
    /// Set the frame index
    pub fn frame_index(mut self, value: usize) -> Self {
        self.frame_index = Some(value);
        self
    }

    /// Set the frame type
    pub fn frame_type(mut self, value: Vp9FrameType) -> Self {
        self.frame_type = Some(value);
        self
    }

    /// Set the frame data
    pub fn frame_data(mut self, value: Vec<u8>) -> Self {
        self.frame_data = Some(value);
        self
    }

    /// Set the offset in the stream
    pub fn offset(mut self, value: usize) -> Self {
        self.offset = Some(value);
        self
    }

    /// Set the frame size
    pub fn size(mut self, value: usize) -> Self {
        self.size = Some(value);
        self
    }

    /// Set the show frame flag
    pub fn show_frame(mut self, value: bool) -> Self {
        self.show_frame = Some(value);
        self
    }

    /// Set the frame width
    pub fn width(mut self, value: u32) -> Self {
        self.width = Some(value);
        self
    }

    /// Set the frame height
    pub fn height(mut self, value: u32) -> Self {
        self.height = Some(value);
        self
    }

    /// Set the frame header
    pub fn frame_header(mut self, value: FrameHeader) -> Self {
        self.frame_header = Some(value);
        self
    }

    /// Build the Vp9Frame
    ///
    /// Returns an error if required fields are not set.
    pub fn build(self) -> Result<Vp9Frame, String> {
        Ok(Vp9Frame {
            frame_index: self
                .frame_index
                .ok_or_else(|| "frame_index is required".to_string())?,
            frame_type: self
                .frame_type
                .ok_or_else(|| "frame_type is required".to_string())?,
            frame_data: self.frame_data.unwrap_or_default(),
            offset: self
                .offset
                .ok_or_else(|| "offset is required".to_string())?,
            size: self.size.ok_or_else(|| "size is required".to_string())?,
            show_frame: self
                .show_frame
                .ok_or_else(|| "show_frame is required".to_string())?,
            width: self.width.ok_or_else(|| "width is required".to_string())?,
            height: self
                .height
                .ok_or_else(|| "height is required".to_string())?,
            frame_header: self.frame_header,
        })
    }
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
            FrameType::Key => Vp9FrameType::Key,
            FrameType::Inter => Vp9FrameType::Inter,
            _ => Vp9FrameType::Unknown,
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
pub fn extract_vp9_frames(data: &[u8]) -> Result<Vec<Vp9Frame>, BitvueError> {
    // First, parse the full stream to get frame information
    let stream = parse_vp9(data).map_err(|e| BitvueError::Parse {
        offset: 0,
        message: e.to_string(),
    })?;

    // Extract frame data using superframe parser
    let frame_data_list = crate::extract_frames(data).map_err(|e| BitvueError::Parse {
        offset: 0,
        message: e.to_string(),
    })?;

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
    use bitvue_core::qp_extraction::QpData;

    // Extract QP from frame header quantization index
    let qp_avg = frame
        .frame_header
        .as_ref()
        .and_then(|h| QpData::from_vp9_qindex(h.quantization.base_q_idx).qp_avg);

    bitvue_core::UnitNode {
        key: bitvue_core::UnitKey {
            stream: bitvue_core::StreamId::A,
            unit_type: "FRAME".to_string(),
            offset: frame.offset as u64,
            size: frame.size,
        },
        unit_type: std::sync::Arc::from("FRAME"),
        offset: frame.offset as u64,
        size: frame.size,
        frame_index: Some(frame.frame_index),
        frame_type: Some(std::sync::Arc::from(frame.frame_type.as_str())),
        pts: Some(frame.frame_index as u64),
        dts: None,
        display_name: std::sync::Arc::from(format!(
            "Frame {} ({})",
            frame.frame_index,
            frame.frame_type.as_str()
        )),
        children: Vec::new(),
        qp_avg,
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
            Vp9FrameType::from_frame_type(FrameType::Key),
            Vp9FrameType::Key
        );
        assert_eq!(
            Vp9FrameType::from_frame_type(FrameType::Inter),
            Vp9FrameType::Inter
        );
    }
}
