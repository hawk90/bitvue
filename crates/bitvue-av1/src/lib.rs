//! bitvue-av1: AV1 OBU bitstream parser
//!
//! This crate provides parsing functionality for AV1 bitstreams at the OBU level.
//!
//! # Example
//!
//! ```no_run
//! use bitvue_av1::{parse_all_obus, parse_sequence_header, ObuType};
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

pub mod bitreader;
pub mod dependency;
pub mod frame_header;
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
pub use symbol::{ArithmeticDecoder, CdfContext, PartitionCdf, SymbolDecoder};
pub use syntax_parser::{
    parse_bitstream_syntax, parse_frame_header_syntax, parse_obu_syntax,
    parse_sequence_header_syntax, TrackedBitReader,
};
pub use tile::{
    parse_coding_unit, parse_partition_tree, parse_superblock, parse_tile_group,
    partition_tree_to_grid, BlockSize, CodingUnit, MotionVector, PartitionNode, PartitionType,
    PredictionMode, RefFrame, Superblock, SuperblockSize, Tile, TileGroup, TileInfo,
};
pub use types::Qp;

/// Parses an AV1 bitstream and returns basic information
///
/// This is a convenience function that parses all OBUs and extracts
/// the sequence header if present. Supports raw OBU format, IVF, MP4,
/// MOV, MKV, WebM, and TS container formats.
pub fn parse_av1(data: &[u8]) -> bitvue_core::Result<Av1Info> {
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

/// Check if data is an MP4 file
pub fn is_mp4(data: &[u8]) -> bool {
    // MP4 files start with a box (typically ftyp)
    // First 4 bytes are size, next 4 bytes are type
    data.get(4..8).is_some_and(|box_type| {
        matches!(box_type, b"ftyp" | b"moov" | b"mdat" | b"free" | b"skip")
    })
}

/// Check if data is an MKV file
pub fn is_mkv(data: &[u8]) -> bool {
    // MKV files start with EBML header
    // First byte is typically 0x1A, and EBML element ID is 0x1A45DFA3
    data.get(0..4).is_some_and(|sig| sig == [0x1A, 0x45, 0xDF, 0xA3])
}

/// Check if data is a WebM file
/// WebM is a subset of MKV with DocType "webm"
pub fn is_webm(data: &[u8]) -> bool {
    // WebM uses the same EBML header as MKV
    // For simplicity, we treat WebM as MKV (they use the same parser)
    is_mkv(data)
}

/// Check if data is a MOV (QuickTime) file
/// MOV uses the same ISO Base Media format as MP4
pub fn is_mov(data: &[u8]) -> bool {
    // MOV files use the same box structure as MP4
    // Check for 'qt  ' or 'moov' box type
    data.get(4..8).is_some_and(|box_type| {
        matches!(box_type, b"ftyp" | b"moov" | b"mdat" | b"wide" | b"free")
    })
}

/// Check if data is a TS (MPEG-2 Transport Stream) file
pub fn is_ts(data: &[u8]) -> bool {
    bitvue_formats::ts::is_ts(data)
}

/// Extract OBU data from MP4 container
pub fn extract_obu_data_from_mp4(data: &[u8]) -> bitvue_core::Result<Vec<u8>> {
    use bitvue_formats::mp4;

    // Extract AV1 samples from MP4
    let samples = mp4::extract_av1_samples(data)?;

    // Concatenate all samples into a single OBU stream
    let mut obu_data = Vec::new();
    for sample in samples {
        obu_data.extend_from_slice(&sample);
    }

    Ok(obu_data)
}

/// Extract OBU data from MKV container
pub fn extract_obu_data_from_mkv(data: &[u8]) -> bitvue_core::Result<Vec<u8>> {
    use bitvue_formats::mkv;

    // Extract AV1 samples from MKV
    let samples = mkv::extract_av1_samples(data)?;

    // Concatenate all samples into a single OBU stream
    let mut obu_data = Vec::new();
    for sample in samples {
        obu_data.extend_from_slice(&sample);
    }

    Ok(obu_data)
}

/// Extract OBU data from TS container
pub fn extract_obu_data_from_ts(data: &[u8]) -> bitvue_core::Result<Vec<u8>> {
    use bitvue_formats::ts;

    // Extract AV1 samples from TS
    let samples = ts::extract_av1_samples(data)?;

    // Concatenate all samples into a single OBU stream
    let mut obu_data = Vec::new();
    for sample in samples {
        obu_data.extend_from_slice(&sample);
    }

    Ok(obu_data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_exports() {
        // Verify all types are accessible
        let _ = ObuType::SequenceHeader;
        let _ = Av1Profile::Main;
    }

    #[test]
    fn test_is_mp4() {
        // Valid MP4 ftyp header
        let mut mp4_data = Vec::new();
        mp4_data.extend_from_slice(&20u32.to_be_bytes());
        mp4_data.extend_from_slice(b"ftyp");
        mp4_data.extend_from_slice(b"isom");
        assert!(is_mp4(&mp4_data));

        // Not MP4
        assert!(!is_mp4(b"DKIF")); // IVF signature
        assert!(!is_mp4(&[]));
        assert!(!is_mp4(b"random"));
    }

    #[test]
    fn test_is_mkv() {
        // Valid MKV EBML header
        let mkv_data = [0x1A, 0x45, 0xDF, 0xA3, 0x00, 0x00, 0x00, 0x00];
        assert!(is_mkv(&mkv_data));

        // Not MKV
        assert!(!is_mkv(b"DKIF")); // IVF signature
        assert!(!is_mkv(&[]));
        assert!(!is_mkv(b"random"));

        // MP4 is not MKV
        let mut mp4_data = Vec::new();
        mp4_data.extend_from_slice(&20u32.to_be_bytes());
        mp4_data.extend_from_slice(b"ftyp");
        assert!(!is_mkv(&mp4_data));
    }

    #[test]
    fn test_is_webm() {
        // WebM uses same EBML header as MKV
        let webm_data = [0x1A, 0x45, 0xDF, 0xA3, 0x00, 0x00, 0x00, 0x00];
        assert!(is_webm(&webm_data));

        // Not WebM
        assert!(!is_webm(b"DKIF"));
        assert!(!is_webm(&[]));
    }

    #[test]
    fn test_is_mov() {
        // Valid MOV with ftyp
        let mut mov_data = Vec::new();
        mov_data.extend_from_slice(&20u32.to_be_bytes());
        mov_data.extend_from_slice(b"ftyp");
        assert!(is_mov(&mov_data));

        // Valid MOV with moov
        let mut mov_data = Vec::new();
        mov_data.extend_from_slice(&20u32.to_be_bytes());
        mov_data.extend_from_slice(b"moov");
        assert!(is_mov(&mov_data));

        // Not MOV
        assert!(!is_mov(b"DKIF"));
        assert!(!is_mov(&[]));
    }

    #[test]
    fn test_is_ts() {
        // Valid TS file
        let mut ts_data = vec![0x47; 188 * 2];
        ts_data[188] = 0x47;
        assert!(is_ts(&ts_data));

        // Not TS
        assert!(!is_ts(b"DKIF"));
        assert!(!is_ts(&[]));
    }

    #[test]
    fn test_obu_type_values() {
        // Verify OBU type enum values match spec
        assert_eq!(ObuType::Reserved0 as u8, 0);
        assert_eq!(ObuType::SequenceHeader as u8, 1);
        assert_eq!(ObuType::TemporalDelimiter as u8, 2);
        assert_eq!(ObuType::FrameHeader as u8, 3);
        assert_eq!(ObuType::TileGroup as u8, 4);
        assert_eq!(ObuType::Metadata as u8, 5);
        assert_eq!(ObuType::Frame as u8, 6);
        assert_eq!(ObuType::RedundantFrameHeader as u8, 7);
        assert_eq!(ObuType::TileList as u8, 8);
        assert_eq!(ObuType::Padding as u8, 15);
    }

    #[test]
    fn test_frame_type_values() {
        // Verify frame type enum
        use FrameType::*;
        assert_eq!(Key as u8, 0);
        assert_eq!(Inter as u8, 1);
        assert_eq!(BFrame as u8, 2);
        assert_eq!(IntraOnly as u8, 3);
        assert_eq!(Switch as u8, 4);
    }

    #[test]
    fn test_leb128_encode_decode() {
        let test_cases = vec![0, 1, 127, 128, 255, 1024, 65535, 1_000_000];

        for value in test_cases {
            let encoded = encode_uleb128(value);
            let (decoded, size) = decode_uleb128(&encoded).unwrap();

            assert_eq!(decoded, value, "Mismatch for value {}", value);
            assert_eq!(size, encoded.len(), "Size mismatch for value {}", value);
            assert_eq!(
                size,
                leb128_size(value),
                "Size calculation mismatch for value {}",
                value
            );
        }
    }

    #[test]
    fn test_leb128_edge_cases() {
        // Test maximum value (8 bytes max in AV1)
        let max_value = (1u64 << 56) - 1; // Max 8-byte LEB128
        let encoded = encode_uleb128(max_value);
        let (decoded, _) = decode_uleb128(&encoded).unwrap();
        assert_eq!(decoded, max_value);
    }
}
