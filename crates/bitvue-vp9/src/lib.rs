//! VP9 bitstream parser for bitvue.
//!
//! This crate provides parsing capabilities for VP9 bitstreams,
//! extracting frame headers, superframe indices, and syntax trees.
//!
//! # Features
//!
//! - Superframe index parsing
//! - Frame header parsing (uncompressed header)
//! - Syntax tree extraction for visualization
//!
//! # Example
//!
//! ```ignore
//! use bitvue_vp9::{parse_vp9, Vp9Stream};
//!
//! let data: &[u8] = &[/* VP9 bitstream data */];
//! let stream = parse_vp9(data)?;
//!
//! for frame in &stream.frames {
//!     println!("Frame type: {:?}", frame.frame_type);
//! }
//! ```

pub mod bitreader;
pub mod error;
pub mod frame_header;
pub mod frames;
pub mod overlay_extraction;
pub mod superframe;
pub mod syntax;

pub use bitreader::BitReader;
pub use error::{Result, Vp9Error};
pub use frame_header::{
    ColorSpace, FrameHeader, FrameType, InterpolationFilter, LoopFilter, Quantization, RefFrame,
    SegmentFeature, Segmentation,
};
pub use frames::{
    extract_frame_at_index, extract_vp9_frames, vp9_frame_to_unit_node, vp9_frames_to_unit_nodes,
    Vp9Frame, Vp9FrameType,
};
pub use overlay_extraction::{
    extract_mv_grid, extract_partition_grid, extract_qp_grid, MotionVector, SuperBlock,
};
pub use superframe::{
    extract_frames, has_superframe_index, parse_superframe_index, SuperframeIndex,
};

use serde::{Deserialize, Serialize};

/// Parsed VP9 stream.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vp9Stream {
    /// Superframe index (if present).
    pub superframe_index: SuperframeIndex,
    /// Parsed frame headers.
    pub frames: Vec<FrameHeader>,
}

impl Vp9Stream {
    /// Get video dimensions from first frame.
    pub fn dimensions(&self) -> Option<(u32, u32)> {
        self.frames.first().map(|f| (f.width, f.height))
    }

    /// Get render dimensions from first frame.
    pub fn render_dimensions(&self) -> Option<(u32, u32)> {
        self.frames
            .first()
            .map(|f| (f.render_width, f.render_height))
    }

    /// Get bit depth from first frame.
    pub fn bit_depth(&self) -> Option<u8> {
        self.frames.first().map(|f| f.bit_depth)
    }

    /// Get color space from first frame.
    pub fn color_space(&self) -> Option<ColorSpace> {
        self.frames.first().map(|f| f.color_space)
    }

    /// Get all key frames.
    pub fn key_frames(&self) -> Vec<&FrameHeader> {
        self.frames.iter().filter(|f| f.is_key_frame()).collect()
    }

    /// Get all inter frames.
    pub fn inter_frames(&self) -> Vec<&FrameHeader> {
        self.frames.iter().filter(|f| !f.is_key_frame()).collect()
    }

    /// Get visible frames (show_frame = true).
    pub fn visible_frames(&self) -> Vec<&FrameHeader> {
        self.frames.iter().filter(|f| f.show_frame).collect()
    }

    /// Get hidden frames (show_frame = false).
    pub fn hidden_frames(&self) -> Vec<&FrameHeader> {
        self.frames.iter().filter(|f| !f.show_frame).collect()
    }

    /// Count total frames.
    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }

    /// Count visible frames.
    pub fn visible_frame_count(&self) -> usize {
        self.frames.iter().filter(|f| f.show_frame).count()
    }
}

/// Parse VP9 bitstream (single frame or superframe).
pub fn parse_vp9(data: &[u8]) -> Result<Vp9Stream> {
    // Parse superframe index
    let superframe_index = parse_superframe_index(data)?;

    // Extract individual frames
    let frame_data = extract_frames(data)?;

    // Parse each frame header
    let mut frames = Vec::with_capacity(frame_data.len());

    for (i, frame_bytes) in frame_data.iter().enumerate() {
        match frame_header::parse_frame_header(frame_bytes) {
            Ok(header) => frames.push(header),
            Err(e) => {
                abseil::vlog!(1, "Failed to parse frame {}: {}", i, e);
                // Continue with other frames
            }
        }
    }

    Ok(Vp9Stream {
        superframe_index,
        frames,
    })
}

/// Quick parse to extract basic stream info without full parsing.
pub fn parse_vp9_quick(data: &[u8]) -> Result<Vp9QuickInfo> {
    let superframe_index = parse_superframe_index(data)?;
    let frame_data = extract_frames(data)?;

    let mut info = Vp9QuickInfo {
        frame_count: superframe_index.frame_count as usize,
        is_superframe: superframe_index.is_superframe(),
        width: None,
        height: None,
        bit_depth: None,
        key_frame_count: 0,
        show_frame_count: 0,
    };

    // Parse first frame for dimensions
    if let Some(first_frame) = frame_data.first() {
        if let Ok(header) = frame_header::parse_frame_header(first_frame) {
            info.width = Some(header.width);
            info.height = Some(header.height);
            info.bit_depth = Some(header.bit_depth);
        }
    }

    // Count frame types
    for frame_bytes in frame_data {
        if let Ok(header) = frame_header::parse_frame_header(frame_bytes) {
            if header.is_key_frame() {
                info.key_frame_count += 1;
            }
            if header.show_frame {
                info.show_frame_count += 1;
            }
        }
    }

    Ok(info)
}

/// Quick stream info without full parsing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vp9QuickInfo {
    /// Total frame count.
    pub frame_count: usize,
    /// Whether this is a superframe.
    pub is_superframe: bool,
    /// Video width.
    pub width: Option<u32>,
    /// Video height.
    pub height: Option<u32>,
    /// Bit depth.
    pub bit_depth: Option<u8>,
    /// Key frame count.
    pub key_frame_count: usize,
    /// Visible frame count.
    pub show_frame_count: usize,
}

#[cfg(test)]
mod tests;
