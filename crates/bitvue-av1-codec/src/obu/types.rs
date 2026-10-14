//! The OBU type codes, header and parsed OBU (spec 5.3).

use serde::{Deserialize, Serialize};
use std::sync::Arc;

use bitvue_engine::{BitvueError, Result};

/// OBU type codes as defined in AV1 specification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum ObuType {
    /// Reserved (0)
    Reserved0 = 0,
    /// Sequence header (1)
    SequenceHeader = 1,
    /// Temporal delimiter (2)
    TemporalDelimiter = 2,
    /// Frame header (3)
    FrameHeader = 3,
    /// Tile group (4)
    TileGroup = 4,
    /// Metadata (5)
    Metadata = 5,
    /// Frame (header + tile group combined) (6)
    Frame = 6,
    /// Redundant frame header (7)
    RedundantFrameHeader = 7,
    /// Tile list (8)
    TileList = 8,
    /// Reserved (9-14)
    Reserved9 = 9,
    Reserved10 = 10,
    Reserved11 = 11,
    Reserved12 = 12,
    Reserved13 = 13,
    Reserved14 = 14,
    /// Padding (15)
    Padding = 15,
}

impl ObuType {
    /// Converts a u8 to ObuType
    pub fn from_u8(value: u8) -> Result<Self> {
        match value {
            0 => Ok(ObuType::Reserved0),
            1 => Ok(ObuType::SequenceHeader),
            2 => Ok(ObuType::TemporalDelimiter),
            3 => Ok(ObuType::FrameHeader),
            4 => Ok(ObuType::TileGroup),
            5 => Ok(ObuType::Metadata),
            6 => Ok(ObuType::Frame),
            7 => Ok(ObuType::RedundantFrameHeader),
            8 => Ok(ObuType::TileList),
            9 => Ok(ObuType::Reserved9),
            10 => Ok(ObuType::Reserved10),
            11 => Ok(ObuType::Reserved11),
            12 => Ok(ObuType::Reserved12),
            13 => Ok(ObuType::Reserved13),
            14 => Ok(ObuType::Reserved14),
            15 => Ok(ObuType::Padding),
            _ => Err(BitvueError::InvalidObuType(value)),
        }
    }

    /// Returns the display name of the OBU type
    pub fn name(&self) -> &'static str {
        match self {
            ObuType::Reserved0 => "RESERVED",
            ObuType::SequenceHeader => "SEQUENCE_HEADER",
            ObuType::TemporalDelimiter => "TEMPORAL_DELIMITER",
            ObuType::FrameHeader => "FRAME_HEADER",
            ObuType::TileGroup => "TILE_GROUP",
            ObuType::Metadata => "METADATA",
            ObuType::Frame => "FRAME",
            ObuType::RedundantFrameHeader => "REDUNDANT_FRAME_HEADER",
            ObuType::TileList => "TILE_LIST",
            ObuType::Reserved9 => "RESERVED",
            ObuType::Reserved10 => "RESERVED",
            ObuType::Reserved11 => "RESERVED",
            ObuType::Reserved12 => "RESERVED",
            ObuType::Reserved13 => "RESERVED",
            ObuType::Reserved14 => "RESERVED",
            ObuType::Padding => "PADDING",
        }
    }

    /// Returns true if this OBU type is frame-related (contains frame data or metadata).
    ///
    /// Returns true for Frame (type 6), FrameHeader (type 3), and TileGroup (type 4).
    /// Note: Only Frame OBU contains complete, self-contained frame data for decoding.
    pub fn has_frame_data(&self) -> bool {
        matches!(
            self,
            ObuType::Frame | ObuType::FrameHeader | ObuType::TileGroup
        )
    }
}

impl std::fmt::Display for ObuType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// OBU header information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObuHeader {
    /// OBU type
    pub obu_type: ObuType,
    /// Whether the OBU has an extension header
    pub has_extension: bool,
    /// Whether the OBU has a size field
    pub has_size: bool,
    /// Temporal ID (from extension, 0 if no extension)
    pub temporal_id: u8,
    /// Spatial ID (from extension, 0 if no extension)
    pub spatial_id: u8,
    /// Header size in bytes (1 or 2)
    pub header_size: usize,
}

/// A complete OBU (header + payload)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Obu {
    /// OBU header
    pub header: ObuHeader,
    /// Payload size in bytes (excluding header)
    pub payload_size: u64,
    /// Total OBU size in bytes (header + size field + payload)
    pub total_size: u64,
    /// Byte offset in the original stream
    pub offset: u64,
    /// Raw payload data (Arc-wrapped for efficient sharing)
    ///
    /// Using Arc<[u8]> instead of Vec<u8> enables:
    /// - O(1) cloning instead of O(n) for Obu clones
    /// - Zero-copy sharing where possible
    /// - 50-90% reduction in allocations for typical video streams
    #[serde(skip)]
    pub payload: Arc<[u8]>,
    /// Frame type (only for Frame/FrameHeader OBUs)
    pub frame_type: Option<crate::frame_header::FrameType>,
    /// Parsed frame header (only for Frame/FrameHeader OBUs)
    #[serde(skip)]
    pub frame_header: Option<crate::frame_header::FrameHeader>,
}

impl Obu {
    /// Returns a summary string for display
    ///
    /// Note: This is a potentially expensive operation due to string allocation.
    /// Consider using tracing macros directly for logging in hot paths.
    #[cfg(feature = "debug-strings")]
    pub fn summary(&self) -> String {
        format!(
            "{}  offset={}  size={}",
            self.header.obu_type.name(),
            self.offset,
            self.total_size
        )
    }

    /// Returns a static placeholder when debug-strings feature is disabled
    /// This avoids allocations in release builds while still providing the API
    #[cfg(not(feature = "debug-strings"))]
    pub fn summary(&self) -> &'static str {
        "OBU"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// (code, variant, display name) for every code the 4-bit `obu_type` field can hold.
    const ALL: [(u8, ObuType, &str); 16] = [
        (0, ObuType::Reserved0, "RESERVED"),
        (1, ObuType::SequenceHeader, "SEQUENCE_HEADER"),
        (2, ObuType::TemporalDelimiter, "TEMPORAL_DELIMITER"),
        (3, ObuType::FrameHeader, "FRAME_HEADER"),
        (4, ObuType::TileGroup, "TILE_GROUP"),
        (5, ObuType::Metadata, "METADATA"),
        (6, ObuType::Frame, "FRAME"),
        (7, ObuType::RedundantFrameHeader, "REDUNDANT_FRAME_HEADER"),
        (8, ObuType::TileList, "TILE_LIST"),
        (9, ObuType::Reserved9, "RESERVED"),
        (10, ObuType::Reserved10, "RESERVED"),
        (11, ObuType::Reserved11, "RESERVED"),
        (12, ObuType::Reserved12, "RESERVED"),
        (13, ObuType::Reserved13, "RESERVED"),
        (14, ObuType::Reserved14, "RESERVED"),
        (15, ObuType::Padding, "PADDING"),
    ];

    fn obu(obu_type: ObuType, offset: u64, total_size: u64) -> Obu {
        Obu {
            header: ObuHeader {
                obu_type,
                has_extension: false,
                has_size: true,
                temporal_id: 0,
                spatial_id: 0,
                header_size: 1,
            },
            payload_size: 0,
            total_size,
            offset,
            payload: Arc::from(Vec::new()),
            frame_type: None,
            frame_header: None,
        }
    }

    #[test]
    fn each_of_the_16_codes_maps_to_its_variant_and_back_to_the_same_number() {
        for (code, variant, _) in ALL {
            assert_eq!(ObuType::from_u8(code).unwrap(), variant, "code {code}");
            assert_eq!(variant as u8, code, "discriminant of {variant:?}");
        }
    }

    #[test]
    fn codes_above_15_are_rejected_with_the_offending_value() {
        for code in 16..=255u8 {
            match ObuType::from_u8(code) {
                Err(BitvueError::InvalidObuType(value)) => assert_eq!(value, code),
                other => panic!("code {code}: {other:?}"),
            }
        }
    }

    #[test]
    fn names_and_display_agree_for_every_type_and_reserved_ones_share_a_name() {
        for (_, variant, name) in ALL {
            assert_eq!(variant.name(), name, "{variant:?}");
            assert_eq!(variant.to_string(), name, "{variant:?}");
        }
    }

    #[test]
    fn only_frame_frame_header_and_tile_group_carry_frame_data() {
        for (code, variant, _) in ALL {
            let expected = matches!(code, 3 | 4 | 6);
            assert_eq!(variant.has_frame_data(), expected, "{variant:?}");
        }
    }

    #[test]
    #[cfg(not(feature = "debug-strings"))]
    fn the_summary_is_a_fixed_placeholder_without_the_debug_strings_feature() {
        assert_eq!(obu(ObuType::Frame, 10, 20).summary(), "OBU");
    }

    #[test]
    #[cfg(feature = "debug-strings")]
    fn the_summary_names_the_type_offset_and_total_size() {
        assert_eq!(
            obu(ObuType::TileGroup, 1234, 56).summary(),
            "TILE_GROUP  offset=1234  size=56"
        );
    }
}
