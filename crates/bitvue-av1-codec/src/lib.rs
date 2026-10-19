//! bitvue-av1-codec: AV1 OBU bitstream parser
//!
//! This crate provides parsing functionality for AV1 bitstreams at the OBU level.
//!
//! # Example
//!
//! ```no_run
//! use bitvue_av1_codec::{parse_all_obus, parse_sequence_header, ObuType};
//! use std::fs;
//!
//! let data = fs::read("video.av1").unwrap();
//! let obus = parse_all_obus(&data).unwrap();
//!
//! for obu in &obus {
//!     println!("{}", obu.summary());
//!
//!     if obu.header.obu_type == ObuType::SequenceHeader {
//!         let seq = parse_sequence_header(&obu.payload).unwrap();
//!         println!("  {}x{} @ {} bit", seq.width(), seq.height(), seq.bit_depth());
//!     }
//! }
//! ```

// Allow clippy warnings common in codec parser code
#![allow(clippy::too_many_arguments)]
#![allow(clippy::ptr_arg)]

pub mod advanced_features;
pub mod bitreader;
pub mod container;
pub mod dependency;
pub mod frame_header;
pub mod frame_header_full;
pub mod ivf;
pub mod leb128;
pub mod obu;
pub mod overlay_extraction;
pub mod sequence;
pub mod symbol;
pub mod syntax_parser;
pub mod tile;
pub mod types;

// Re-export main types
pub use bitreader::BitReader;
pub use container::{
    extract_obu_data_from_mkv, extract_obu_data_from_mp4, extract_obu_data_from_ts, is_mkv, is_mov,
    is_mp4, is_ts, is_webm,
};
pub use dependency::{
    extract_required_obus, DependencyGraph, ExtractionRequest, ExtractionResult, FrameNode,
};
pub use frame_header::{parse_frame_header_basic, FrameHeader, FrameType};
pub use ivf::{
    extract_obu_data, is_av1_ivf, is_ivf, parse_ivf_frames, parse_ivf_header, IvfFrame, IvfHeader,
};
pub use leb128::{decode_uleb128, encode_uleb128, leb128_size};
pub use obu::{
    parse_all_obus, parse_all_obus_resilient, parse_obu, parse_obu_header, Obu, ObuHeader,
    ObuIterator, ObuType,
};
pub use overlay_extraction::{
    extract_mv_grid, extract_mv_grid_from_parsed, extract_partition_grid,
    extract_partition_grid_from_parsed, extract_prediction_mode_grid,
    extract_prediction_mode_grid_from_parsed, extract_qp_grid, extract_qp_grid_from_parsed,
    extract_transform_grid, extract_transform_grid_from_parsed,
};
pub use sequence::{parse_sequence_header, Av1Profile, ColorConfig, SequenceHeader};
pub use symbol::{update_cdf, ArithmeticDecoder, CdfContext, PartitionCdf, SymbolDecoder};
pub use syntax_parser::{
    parse_bitstream_syntax, parse_frame_header_syntax, parse_obu_syntax,
    parse_sequence_header_syntax, TrackedBitReader,
};
pub use tile::{
    parse_coding_unit, parse_partition_tree, parse_superblock, partition_tree_to_grid, BlockSize,
    CodingUnit, MotionVector, PartitionNode, PartitionType, PredictionMode, RefFrame, Superblock,
    SuperblockSize, Tile,
};
pub use types::{Qp, QuarterPel, TimestampPts};

/// Parses an AV1 bitstream and returns basic information
///
/// This is a convenience function that parses all OBUs and extracts
/// the sequence header if present. Supports raw OBU format, IVF, MP4,
/// MOV, MKV, WebM, and TS container formats.
pub fn parse_av1(data: &[u8]) -> bitvue_engine::Result<Av1Info> {
    // Check format and extract OBU data (order matters for detection)
    let obu_data = if is_ivf(data) {
        // IVF container
        std::borrow::Cow::Owned(extract_obu_data(data)?)
    } else if is_ts(data) {
        // TS container (check before MP4/MOV to avoid false positives)
        std::borrow::Cow::Owned(extract_obu_data_from_ts(data)?)
    } else if is_mp4(data) || is_mov(data) {
        // MP4/MOV container (same format)
        std::borrow::Cow::Owned(extract_obu_data_from_mp4(data)?)
    } else if is_mkv(data) {
        // MKV/WebM container (same format)
        std::borrow::Cow::Owned(extract_obu_data_from_mkv(data)?)
    } else {
        // Raw OBU format
        std::borrow::Cow::Borrowed(data)
    };

    let obus = parse_all_obus(&obu_data)?;

    let mut sequence_header = None;
    let mut frame_count = 0;

    for obu in &obus {
        match obu.header.obu_type {
            ObuType::SequenceHeader => {
                sequence_header = Some(parse_sequence_header(&obu.payload)?);
            }
            ObuType::Frame | ObuType::FrameHeader => {
                frame_count += 1;
            }
            _ => {}
        }
    }

    Ok(Av1Info {
        obu_count: obus.len(),
        frame_count,
        sequence_header,
        obus,
    })
}

/// High-level AV1 bitstream information
#[derive(Debug)]
pub struct Av1Info {
    /// Total number of OBUs
    pub obu_count: usize,
    /// Number of frames
    pub frame_count: usize,
    /// Parsed sequence header (if present)
    pub sequence_header: Option<SequenceHeader>,
    /// All parsed OBUs
    pub obus: Vec<Obu>,
}

impl Av1Info {
    /// Returns the width if sequence header is present
    pub fn width(&self) -> Option<u32> {
        self.sequence_header.as_ref().map(|s| s.width())
    }

    /// Returns the height if sequence header is present
    pub fn height(&self) -> Option<u32> {
        self.sequence_header.as_ref().map(|s| s.height())
    }

    /// Returns the bit depth if sequence header is present
    pub fn bit_depth(&self) -> Option<u8> {
        self.sequence_header.as_ref().map(|s| s.bit_depth())
    }

    /// Returns the profile if sequence header is present
    pub fn profile(&self) -> Option<&Av1Profile> {
        self.sequence_header.as_ref().map(|s| &s.profile)
    }
}

#[cfg(test)]
mod tests;
