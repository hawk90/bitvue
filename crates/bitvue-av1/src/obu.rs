//! AV1 OBU (Open Bitstream Unit) parsing
//!
//! OBU is the fundamental unit of the AV1 bitstream.
//! Reference: AV1 Specification Section 5.3

use serde::{Deserialize, Serialize};

use bitvue_core::{BitvueError, Result};

use crate::bitreader::BitReader;
use crate::leb128::decode_uleb128;

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
    /// Raw payload data
    #[serde(skip)]
    pub payload: Vec<u8>,
    /// Frame type (only for Frame/FrameHeader OBUs)
    pub frame_type: Option<crate::frame_header::FrameType>,
    /// Parsed frame header (only for Frame/FrameHeader OBUs)
    #[serde(skip)]
    pub frame_header: Option<crate::frame_header::FrameHeader>,
}

impl Obu {
    /// Returns a summary string for display
    pub fn summary(&self) -> String {
        format!(
            "{}  offset={}  size={}",
            self.header.obu_type.name(),
            self.offset,
            self.total_size
        )
    }
}

/// Parses an OBU header from a BitReader
///
/// Returns the header and advances the reader past the header bytes.
pub fn parse_obu_header(reader: &mut BitReader) -> Result<ObuHeader> {
    // obu_forbidden_bit (1 bit) - must be 0
    let forbidden = reader.read_bit()?;
    if forbidden {
        return Err(BitvueError::Parse {
            offset: reader.position() - 1,
            message: "obu_forbidden_bit is not 0".to_string(),
        });
    }

    // obu_type (4 bits)
    let obu_type_val = reader.read_bits(4)? as u8;
    let obu_type = ObuType::from_u8(obu_type_val)?;

    // obu_extension_flag (1 bit)
    let has_extension = reader.read_bit()?;

    // obu_has_size_field (1 bit)
    let has_size = reader.read_bit()?;

    // obu_reserved_1bit (1 bit) - must be 0
    let _reserved = reader.read_bit()?;

    let mut temporal_id = 0u8;
    let mut spatial_id = 0u8;
    let mut header_size = 1usize;

    if has_extension {
        // temporal_id (3 bits)
        temporal_id = reader.read_bits(3)? as u8;
        // spatial_id (2 bits)
        spatial_id = reader.read_bits(2)? as u8;
        // extension_header_reserved_3bits (3 bits)
        let _ext_reserved = reader.read_bits(3)?;
        header_size = 2;
    }

    Ok(ObuHeader {
        obu_type,
        has_extension,
        has_size,
        temporal_id,
        spatial_id,
        header_size,
    })
}

/// Parses a single OBU from a byte slice at the given offset
///
/// Returns the parsed OBU and the number of bytes consumed.
pub fn parse_obu(data: &[u8], offset: usize) -> Result<(Obu, usize)> {
    if offset >= data.len() {
        return Err(BitvueError::UnexpectedEof(offset as u64));
    }

    let slice = &data[offset..];
    let mut reader = BitReader::new(slice);

    // Parse header
    let header = parse_obu_header(&mut reader)?;

    // Calculate bytes read so far (should be byte-aligned after header)
    let header_bytes = reader.byte_position();

    // Parse size field if present
    let (payload_size, size_field_bytes) = if header.has_size {
        let (size, len) = decode_uleb128(&slice[header_bytes..])?;
        (size, len)
    } else {
        // If no size field, payload extends to end of data
        ((slice.len() - header_bytes) as u64, 0)
    };

    let payload_start = header_bytes + size_field_bytes;
    let total_size = payload_start as u64 + payload_size;

    // Validate we have enough data
    if payload_start as u64 + payload_size > slice.len() as u64 {
        return Err(BitvueError::UnexpectedEof(offset as u64 + total_size));
    }

    // Extract payload
    let payload = slice[payload_start..payload_start + payload_size as usize].to_vec();

    // Try to extract frame header for Frame/FrameHeader OBUs
    let (frame_type, frame_header) =
        if matches!(header.obu_type, ObuType::Frame | ObuType::FrameHeader) {
            match crate::frame_header::parse_frame_header_basic(&payload) {
                Ok(fh) => {
                    let frame_type = fh.frame_type;
                    (Some(frame_type), Some(fh))
                }
                Err(_) => (None, None),
            }
        } else {
            (None, None)
        };

    let obu = Obu {
        header,
        payload_size,
        total_size,
        offset: offset as u64,
        payload,
        frame_type,
        frame_header,
    };

    Ok((obu, total_size as usize))
}

/// Parses all OBUs from a byte slice
///
/// Returns a vector of all parsed OBUs.
pub fn parse_all_obus(data: &[u8]) -> Result<Vec<Obu>> {
    let mut obus = Vec::new();
    let mut offset = 0usize;

    while offset < data.len() {
        let (obu, consumed) = parse_obu(data, offset)?;
        obus.push(obu);
        offset += consumed;
    }

    Ok(obus)
}

/// Parses all OBUs from a byte slice with error resilience
///
/// Unlike `parse_all_obus()`, this function continues parsing even when errors occur,
/// collecting diagnostics for each error encountered. This is useful for analyzing
/// corrupt or malformed bitstreams.
///
/// Returns:
/// - Vector of successfully parsed OBUs
/// - Vector of diagnostics for any errors encountered
pub fn parse_all_obus_resilient(
    data: &[u8],
    stream_id: bitvue_core::StreamId,
) -> (Vec<Obu>, Vec<bitvue_core::event::Diagnostic>) {
    use bitvue_core::event::{Category, Diagnostic, Severity};

    let mut obus = Vec::new();
    let mut diagnostics = Vec::new();
    let mut offset = 0usize;
    let mut diagnostic_id = 0u64;

    while offset < data.len() {
        match parse_obu(data, offset) {
            Ok((obu, consumed)) => {
                obus.push(obu);
                offset += consumed;
            }
            Err(e) => {
                // Convert parse error to diagnostic
                let (message, severity, impact_score) = match &e {
                    BitvueError::Parse { message, .. } => {
                        (message.clone(), Severity::Error, 85)
                    }
                    BitvueError::UnexpectedEof(_) => {
                        ("Unexpected end of file".to_string(), Severity::Fatal, 100)
                    }
                    BitvueError::InvalidObuType(t) => {
                        (format!("Invalid OBU type: {}", t), Severity::Error, 90)
                    }
                    _ => (format!("{:?}", e), Severity::Error, 80),
                };

                let diagnostic = Diagnostic {
                    id: diagnostic_id,
                    severity,
                    stream_id,
                    message,
                    category: Category::Bitstream,
                    offset_bytes: offset as u64,
                    timestamp_ms: 0, // Will be calculated from frame index later
                    frame_index: Some(obus.len()), // Approximate frame index
                    count: 1,
                    impact_score,
                };

                diagnostics.push(diagnostic);
                diagnostic_id += 1;

                // Try to skip past the error and continue parsing
                // Skip 1 byte at a time looking for next valid OBU header
                offset += 1;

                // After 10 consecutive errors, give up
                if diagnostics.len() >= 10 && diagnostics.len() > obus.len() {
                    let fatal_diagnostic = Diagnostic {
                        id: diagnostic_id,
                        severity: Severity::Fatal,
                        stream_id,
                        message: "Too many parse errors, stopping".to_string(),
                        category: Category::Bitstream,
                        offset_bytes: offset as u64,
                        timestamp_ms: 0,
                        frame_index: None,
                        count: diagnostics.len() as u32,
                        impact_score: 100,
                    };
                    diagnostics.push(fatal_diagnostic);
                    break;
                }
            }
        }
    }

    (obus, diagnostics)
}

/// Iterator over OBUs in a byte slice
pub struct ObuIterator<'a> {
    data: &'a [u8],
    offset: usize,
}

impl<'a> ObuIterator<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, offset: 0 }
    }
}

impl<'a> Iterator for ObuIterator<'a> {
    type Item = Result<Obu>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.offset >= self.data.len() {
            return None;
        }

        match parse_obu(self.data, self.offset) {
            Ok((obu, consumed)) => {
                self.offset += consumed;
                Some(Ok(obu))
            }
            Err(e) => Some(Err(e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_obu_type_from_u8() {
        assert_eq!(ObuType::from_u8(1).unwrap(), ObuType::SequenceHeader);
        assert_eq!(ObuType::from_u8(6).unwrap(), ObuType::Frame);
        assert_eq!(ObuType::from_u8(15).unwrap(), ObuType::Padding);
        assert!(ObuType::from_u8(16).is_err());
    }

    #[test]
    fn test_parse_simple_obu_header() {
        // OBU header: type=1 (sequence), no extension, has size
        // 0 0001 0 1 0 = 0x0A
        let data = [0x0A];
        let mut reader = BitReader::new(&data);
        let header = parse_obu_header(&mut reader).unwrap();

        assert_eq!(header.obu_type, ObuType::SequenceHeader);
        assert!(!header.has_extension);
        assert!(header.has_size);
        assert_eq!(header.header_size, 1);
    }

    #[test]
    fn test_parse_obu_with_extension() {
        // OBU header with extension: type=6 (frame), has extension, has size
        // Byte 1: 0 0110 1 1 0 = 0x36
        // Byte 2: temporal=1, spatial=0, reserved=0 -> 001 00 000 = 0x20
        let data = [0x36, 0x20];
        let mut reader = BitReader::new(&data);
        let header = parse_obu_header(&mut reader).unwrap();

        assert_eq!(header.obu_type, ObuType::Frame);
        assert!(header.has_extension);
        assert!(header.has_size);
        assert_eq!(header.temporal_id, 1);
        assert_eq!(header.spatial_id, 0);
        assert_eq!(header.header_size, 2);
    }

    #[test]
    fn test_parse_complete_obu() {
        // Temporal delimiter OBU (type=2, no extension, has size, size=0)
        // Header: 0 0010 0 1 0 = 0x12
        // Size: 0x00 (LEB128 for 0)
        let data = [0x12, 0x00];
        let (obu, consumed) = parse_obu(&data, 0).unwrap();

        assert_eq!(obu.header.obu_type, ObuType::TemporalDelimiter);
        assert_eq!(obu.payload_size, 0);
        assert_eq!(obu.total_size, 2);
        assert_eq!(consumed, 2);
    }

    #[test]
    fn test_parse_obu_with_payload() {
        // Padding OBU with 3 bytes of payload
        // Header: 0 1111 0 1 0 = 0x7A
        // Size: 0x03 (LEB128 for 3)
        // Payload: 0x00, 0x00, 0x00
        let data = [0x7A, 0x03, 0x00, 0x00, 0x00];
        let (obu, consumed) = parse_obu(&data, 0).unwrap();

        assert_eq!(obu.header.obu_type, ObuType::Padding);
        assert_eq!(obu.payload_size, 3);
        assert_eq!(obu.total_size, 5);
        assert_eq!(obu.payload.len(), 3);
        assert_eq!(consumed, 5);
    }

    #[test]
    fn test_parse_multiple_obus() {
        // Two temporal delimiter OBUs
        let data = [0x12, 0x00, 0x12, 0x00];
        let obus = parse_all_obus(&data).unwrap();

        assert_eq!(obus.len(), 2);
        assert_eq!(obus[0].offset, 0);
        assert_eq!(obus[1].offset, 2);
    }

    #[test]
    fn test_forbidden_bit_error() {
        // Forbidden bit set (first bit = 1)
        let data = [0x92]; // 1 0010 0 1 0
        let mut reader = BitReader::new(&data);
        assert!(parse_obu_header(&mut reader).is_err());
    }
}
