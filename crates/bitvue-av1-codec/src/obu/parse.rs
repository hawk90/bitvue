//! Parsing one OBU from a byte slice, and all of them.

use std::sync::Arc;

use bitvue_engine::{BitvueError, Result};

use super::iterator::ObuIterator;
use super::types::{Obu, ObuHeader, ObuType};
use crate::bitreader::BitReader;
use crate::leb128::decode_uleb128;

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
        // temporal_id (3 bits) - valid range is 0-7 per AV1 spec
        temporal_id = reader.read_bits(3)? as u8;
        // Validate temporal_id is within 3-bit range
        if temporal_id > 7 {
            return Err(BitvueError::Parse {
                offset: reader.position(),
                message: format!("Invalid temporal_id: {} (must be 0-7)", temporal_id),
            });
        }

        // spatial_id (2 bits) - valid range is 0-3 per AV1 spec
        spatial_id = reader.read_bits(2)? as u8;
        // Validate spatial_id is within 2-bit range
        if spatial_id > 3 {
            return Err(BitvueError::Parse {
                offset: reader.position(),
                message: format!("Invalid spatial_id: {} (must be 0-3)", spatial_id),
            });
        }

        // extension_header_reserved_3bits (3 bits) - should be 0 per spec
        let ext_reserved = reader.read_bits(3)?;
        if ext_reserved != 0 {
            return Err(BitvueError::Parse {
                offset: reader.position(),
                message: format!(
                    "Invalid extension_header_reserved_3bits: {} (must be 0)",
                    ext_reserved
                ),
            });
        }

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
        // Validate header_bytes doesn't exceed slice length to prevent underflow
        if header_bytes > slice.len() {
            return Err(BitvueError::UnexpectedEof(
                offset as u64 + header_bytes as u64,
            ));
        }
        ((slice.len() - header_bytes) as u64, 0)
    };

    let payload_start = header_bytes + size_field_bytes;

    // Validate payload size is reasonable (DoS protection)
    const MAX_OBU_PAYLOAD_SIZE: u64 = 100 * 1024 * 1024; // 100MB max per OBU
    if payload_size > MAX_OBU_PAYLOAD_SIZE {
        return Err(BitvueError::InvalidData(format!(
            "OBU payload size exceeds maximum: {} bytes (max: {})",
            payload_size, MAX_OBU_PAYLOAD_SIZE
        )));
    }

    // Check for overflow in payload size calculation
    // payload_size is u64, payload_start is usize - convert carefully
    let payload_size_usize = payload_size
        .try_into()
        .map_err(|_| BitvueError::InvalidData("OBU payload size overflow".to_string()))?;

    let total_size = payload_start
        .checked_add(payload_size_usize)
        .ok_or_else(|| BitvueError::InvalidData("OBU payload size overflow".to_string()))?;

    // Validate we have enough data
    if total_size > slice.len() {
        return Err(BitvueError::UnexpectedEof(
            offset as u64 + total_size as u64,
        ));
    }

    // Extract payload as Arc-wrapped slice for efficient sharing
    // Using Arc<[u8]> instead of Vec<u8> enables O(1) cloning
    // and zero-copy sharing across the codebase
    let payload: Arc<[u8]> = Arc::from(&slice[payload_start..total_size]);

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
        total_size: total_size as u64,
        offset: offset as u64,
        payload,
        frame_type,
        frame_header,
    };

    Ok((obu, total_size))
}

/// Parses all OBUs from a byte slice
///
/// Returns a vector of all parsed OBUs.
/// Uses the ObuIterator for efficient iteration.
pub fn parse_all_obus(data: &[u8]) -> Result<Vec<Obu>> {
    ObuIterator::new(data).collect()
}
