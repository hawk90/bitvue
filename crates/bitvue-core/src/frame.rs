//! Codec-agnostic video frame structure
//!
//! This module provides a unified frame representation that can be used
//! across all video codecs (AV1, AVC, HEVC, VP9, etc.), eliminating
//! code duplication and providing a consistent interface.

use crate::types::FrameType;
use serde::{Deserialize, Serialize};

/// Codec-agnostic video frame
///
/// This structure captures the common fields shared across all video codecs,
/// providing a unified interface for frame operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoFrame {
    /// Frame index in the stream (0-based)
    pub frame_index: usize,

    /// Frame type (I, P, B, etc.)
    pub frame_type: FrameType,

    /// Raw encoded frame data
    pub data: Vec<u8>,

    /// Starting byte position in the stream
    pub offset: usize,

    /// Frame size in bytes
    pub size: usize,

    /// Presentation timestamp (PTS)
    pub pts: Option<u64>,

    /// Decoding timestamp (DTS)
    pub dts: Option<u64>,

    /// Temporal layer ID (for scalable coding)
    pub temporal_id: Option<u8>,

    /// Whether this is a key/random access frame
    pub is_key_frame: bool,

    /// Whether this is a reference frame
    pub is_ref: bool,
}

impl VideoFrame {
    /// Creates a new VideoFrameBuilder for constructing VideoFrame instances
    pub fn builder() -> VideoFrameBuilder {
        VideoFrameBuilder::default()
    }

    /// Get the display name for this frame
    pub fn display_name(&self) -> String {
        format!("Frame {} ({})", self.frame_index, self.frame_type.as_str())
    }

    /// Check if this frame is an IDR (Instantaneous Decoder Refresh) frame
    pub fn is_idr(&self) -> bool {
        self.is_key_frame
    }

    /// Convert to UnitNode format
    pub fn to_unit_node(&self, stream_id: crate::StreamId) -> crate::UnitNode {
        crate::UnitNode {
            key: crate::UnitKey {
                stream: stream_id,
                unit_type: "FRAME".to_string(),
                offset: self.offset as u64,
                size: self.size,
            },
            unit_type: std::sync::Arc::from("FRAME"),
            offset: self.offset as u64,
            size: self.size,
            frame_index: Some(self.frame_index),
            frame_type: Some(std::sync::Arc::from(self.frame_type.as_str())),
            pts: self.pts,
            dts: self.dts,
            display_name: std::sync::Arc::from(self.display_name()),
            children: Vec::new(),
            qp_avg: None, // Codec-specific metadata not included
            mv_grid: None,
            temporal_id: self.temporal_id,
            ref_frames: None,
            ref_slots: None,
        }
    }
}

impl Default for VideoFrame {
    fn default() -> Self {
        Self {
            frame_index: 0,
            frame_type: FrameType::Unknown,
            data: Vec::new(),
            offset: 0,
            size: 0,
            pts: None,
            dts: None,
            temporal_id: None,
            is_key_frame: false,
            is_ref: false,
        }
    }
}

/// Builder for constructing VideoFrame instances
///
/// # Example
///
/// ```rust
/// use bitvue_core::frame::{VideoFrame, VideoFrameBuilder};
/// use bitvue_core::FrameType;
///
/// let frame = VideoFrame::builder()
///     .frame_index(0)
///     .frame_type(FrameType::Key)
///     .data(vec![0x00, 0x01, 0x02])
///     .offset(0)
///     .size(3)
///     .is_key_frame(true)
///     .is_ref(true)
///     .build();
/// ```
#[derive(Debug, Default)]
pub struct VideoFrameBuilder {
    frame_index: Option<usize>,
    frame_type: Option<FrameType>,
    data: Option<Vec<u8>>,
    offset: Option<usize>,
    size: Option<usize>,
    pts: Option<u64>,
    dts: Option<u64>,
    temporal_id: Option<u8>,
    is_key_frame: Option<bool>,
    is_ref: Option<bool>,
}

impl VideoFrameBuilder {
    /// Set the frame index
    pub fn frame_index(mut self, value: usize) -> Self {
        self.frame_index = Some(value);
        self
    }

    /// Set the frame type
    pub fn frame_type(mut self, value: FrameType) -> Self {
        self.frame_type = Some(value);
        self
    }

    /// Set the frame data
    pub fn data(mut self, value: Vec<u8>) -> Self {
        self.data = Some(value);
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

    /// Set the presentation timestamp
    pub fn pts(mut self, value: u64) -> Self {
        self.pts = Some(value);
        self
    }

    /// Set the decoding timestamp
    pub fn dts(mut self, value: u64) -> Self {
        self.dts = Some(value);
        self
    }

    /// Set the temporal layer ID
    pub fn temporal_id(mut self, value: u8) -> Self {
        self.temporal_id = Some(value);
        self
    }

    /// Set whether this is a key frame
    pub fn is_key_frame(mut self, value: bool) -> Self {
        self.is_key_frame = Some(value);
        self
    }

    /// Set whether this is a reference frame
    pub fn is_ref(mut self, value: bool) -> Self {
        self.is_ref = Some(value);
        self
    }

    /// Build the VideoFrame
    ///
    /// # Panics
    ///
    /// Panics if required fields (frame_index, frame_type, offset, size) are not set.
    pub fn build(self) -> VideoFrame {
        VideoFrame {
            frame_index: self.frame_index.expect("frame_index is required"),
            frame_type: self.frame_type.expect("frame_type is required"),
            data: self.data.unwrap_or_default(),
            offset: self.offset.expect("offset is required"),
            size: self.size.expect("size is required"),
            pts: self.pts,
            dts: self.dts,
            temporal_id: self.temporal_id,
            is_key_frame: self.is_key_frame.unwrap_or(false),
            is_ref: self.is_ref.unwrap_or(false),
        }
    }
}

/// Codec-specific frame metadata extensions
///
/// Different codecs have different additional metadata. This enum
/// allows codec-specific information to be attached to a VideoFrame.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CodecMetadata {
    /// No additional metadata
    None,

    /// AVC/H.264 specific metadata
    Avc {
        /// Picture Order Count
        poc: i32,
        /// Frame number
        frame_num: u32,
        /// Slice header data (simplified)
        slice_header: Option<AvcSliceInfo>,
    },

    /// HEVC/H.265 specific metadata
    Hevc {
        /// Picture Order Count
        poc: i32,
        /// Frame number
        frame_num: u32,
        /// Whether this is an IRAP frame
        is_irap: bool,
        /// Slice header data (simplified)
        slice_header: Option<HevcSliceInfo>,
    },

    /// VP9 specific metadata
    Vp9 {
        /// Frame width
        width: u32,
        /// Frame height
        height: u32,
        /// Show frame flag
        show_frame: bool,
    },

    /// AV1 specific metadata
    Av1 {
        /// Frame width
        width: u32,
        /// Frame height
        height: u32,
        /// Show existing frame flag
        show_frame: bool,
    },
}

/// Simplified AVC slice information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvcSliceInfo {
    pub slice_type: String,
    pub first_mb_in_slice: u32,
}

/// Simplified HEVC slice information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HevcSliceInfo {
    pub slice_type: String,
    pub first_slice_segment_in_pic: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_video_frame_builder() {
        let frame = VideoFrame::builder()
            .frame_index(0)
            .frame_type(FrameType::Key)
            .data(vec![0x00, 0x01])
            .offset(0)
            .size(2)
            .is_key_frame(true)
            .is_ref(true)
            .build();

        assert_eq!(frame.frame_index, 0);
        assert_eq!(frame.frame_type, FrameType::Key);
        assert_eq!(frame.data, vec![0x00, 0x01]);
        assert_eq!(frame.offset, 0);
        assert_eq!(frame.size, 2);
        assert!(frame.is_key_frame);
        assert!(frame.is_ref);
    }

    #[test]
    fn test_video_frame_display_name() {
        let frame = VideoFrame::builder()
            .frame_index(42)
            .frame_type(FrameType::Inter)
            .offset(0)
            .size(100)
            .build();

        assert_eq!(frame.display_name(), "Frame 42 (P)");
    }

    #[test]
    fn test_video_frame_default() {
        let frame = VideoFrame::default();
        assert_eq!(frame.frame_index, 0);
        assert_eq!(frame.frame_type, FrameType::Unknown);
        assert!(frame.data.is_empty());
        assert!(!frame.is_key_frame);
        assert!(!frame.is_ref);
    }
}
