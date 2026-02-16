//! AV3 bitstream parser for bitvue.
//!
//! This crate provides parsing capabilities for AV3 (AV1 successor)
//! bitstreams, extracting OBU units, sequence headers, and frame data.
//!
//! # Features
//!
//! - OBU (Open Bitstream Unit) parsing
//! - Sequence header parsing
//! - Frame header parsing
//! - Metadata OBU parsing
//!
//! # Example
//!
//! ```ignore
//! use bitvue_av3_codec::{parse_av3, Av3Stream};
//!
//! let data: &[u8] = &[/* AV3 bitstream data */];
//! let stream = parse_av3(data)?;
//!
//! for obu in &stream.obu_units {
//!     println!("OBU type: {:?}", obu.obu_type);
//! }
//! ```

pub mod bitreader;
pub mod error;
pub mod frame_header;
pub mod metadata;
pub mod obu;
pub mod overlay_extraction;
pub mod sequence_header;

pub use bitreader::BitReader;
pub use error::{Av3Error, Result};
pub use frame_header::{
    parse_frame_header, FrameHeader, FrameType, PrimaryRefFrame, ReferenceFrame,
};
pub use metadata::{MetadataObu, MetadataType};
pub use obu::{parse_obu_header, parse_obu_units, ObuHeader, ObuType, ObuUnit};
pub use overlay_extraction::{
    extract_mv_grid, extract_partition_grid, extract_qp_grid, CodingUnit, MotionVector, SuperBlock,
};
pub use sequence_header::{
    parse_sequence_header, OperatingParametersInfo, OperatingPoint, SequenceHeader,
};

use std::collections::HashMap;

/// Parsed AV3 bitstream.
#[derive(Debug, Clone)]
pub struct Av3Stream {
    /// All OBU units in the stream.
    pub obu_units: Vec<ObuUnit>,
    /// Sequence headers (indexed by seq_profile).
    pub seq_headers: HashMap<u8, SequenceHeader>,
    /// Frame headers.
    pub frame_headers: Vec<FrameHeader>,
}

impl Av3Stream {
    /// Get video dimensions from first sequence header.
    pub fn dimensions(&self) -> Option<(u32, u32)> {
        self.seq_headers
            .values()
            .next()
            .map(|seq| (seq.max_frame_width, seq.max_frame_height))
    }

    /// Get bit depth from sequence header.
    pub fn bit_depth(&self) -> Option<u8> {
        self.seq_headers.values().next().map(|seq| seq.bit_depth)
    }

    /// Get frame rate from sequence header timing info.
    pub fn frame_rate(&self) -> Option<f64> {
        self.seq_headers.values().next().and_then(|seq| {
            if seq.timing_info_present_flag
                && seq.time_scale > 0
                && seq.num_units_in_display_tick > 0
            {
                return Some(seq.time_scale as f64 / seq.num_units_in_display_tick as f64);
            }
            None
        })
    }

    /// Count frames.
    pub fn frame_count(&self) -> usize {
        self.frame_headers.len()
    }

    /// Get all key frames.
    pub fn key_frames(&self) -> Vec<&FrameHeader> {
        self.frame_headers
            .iter()
            .filter(|f| f.frame_type == FrameType::KeyFrame)
            .collect()
    }

    /// Get all inter frames.
    pub fn inter_frames(&self) -> Vec<&FrameHeader> {
        self.frame_headers
            .iter()
            .filter(|f| f.frame_type == FrameType::InterFrame)
            .collect()
    }

    /// Get all switch frames.
    pub fn switch_frames(&self) -> Vec<&FrameHeader> {
        self.frame_headers
            .iter()
            .filter(|f| f.frame_type == FrameType::SwitchFrame)
            .collect()
    }

    /// Get all show existing frames.
    pub fn show_existing_frames(&self) -> Vec<&FrameHeader> {
        self.frame_headers
            .iter()
            .filter(|f| f.frame_type == FrameType::ShowExistingFrame)
            .collect()
    }
}

/// Parse AV3 bitstream from OBU data.
pub fn parse_av3(data: &[u8]) -> Result<Av3Stream> {
    let obu_units = parse_obu_units(data)?;

    let mut seq_headers = HashMap::new();
    let mut frame_headers = Vec::new();

    for obu in &obu_units {
        match obu.header.obu_type {
            ObuType::SequenceHeader => {
                if let Ok(seq) = sequence_header::parse_sequence_header(&obu.payload) {
                    seq_headers.insert(seq.seq_profile, seq);
                }
            }
            ObuType::FrameHeader | ObuType::Frame => {
                if let Ok(header) = frame_header::parse_frame_header(&obu.payload) {
                    frame_headers.push(header);
                }
            }
            _ => {}
        }
    }

    Ok(Av3Stream {
        obu_units,
        seq_headers,
        frame_headers,
    })
}

/// Quick parse to extract basic stream info.
pub fn parse_av3_quick(data: &[u8]) -> Result<Av3QuickInfo> {
    let obu_units = parse_obu_units(data)?;

    let mut info = Av3QuickInfo {
        obu_count: obu_units.len(),
        seq_header_count: 0,
        frame_count: 0,
        key_frame_count: 0,
        width: None,
        height: None,
        bit_depth: None,
    };

    for obu in &obu_units {
        match obu.header.obu_type {
            ObuType::SequenceHeader => {
                info.seq_header_count += 1;
                if info.width.is_none() {
                    if let Ok(seq) = sequence_header::parse_sequence_header(&obu.payload) {
                        info.width = Some(seq.max_frame_width);
                        info.height = Some(seq.max_frame_height);
                        info.bit_depth = Some(seq.bit_depth);
                    }
                }
            }
            ObuType::FrameHeader | ObuType::Frame => {
                info.frame_count += 1;
                if let Ok(header) = frame_header::parse_frame_header(&obu.payload) {
                    if header.frame_type == FrameType::KeyFrame {
                        info.key_frame_count += 1;
                    }
                }
            }
            _ => {}
        }
    }

    Ok(info)
}

/// Quick stream info without full parsing.
#[derive(Debug, Clone)]
pub struct Av3QuickInfo {
    /// Total OBU unit count.
    pub obu_count: usize,
    /// Sequence header count.
    pub seq_header_count: usize,
    /// Frame count.
    pub frame_count: usize,
    /// Key frame count.
    pub key_frame_count: usize,
    /// Video width.
    pub width: Option<u32>,
    /// Video height.
    pub height: Option<u32>,
    /// Bit depth.
    pub bit_depth: Option<u8>,
}

#[cfg(test)]
mod tests;
