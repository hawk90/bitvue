//! OBU (Open Bitstream Unit) parsing for AV3.

use crate::bitreader::BitReader;
use crate::error::{Av3Error, Result};
use serde::{Deserialize, Serialize};

/// OBU types for AV3 (similar to AV1 with some additions).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum ObuType {
    /// Reserved
    Reserved = 0,
    /// Sequence Header OBU
    SequenceHeader = 1,
    /// Temporal Delimiter OBU
    TemporalDelimiter = 2,
    /// Overhead Info OBU (new in AV3)
    OverheadInfo = 3,
    /// Frame Header OBU
    FrameHeader = 4,
    /// Frame OBU (includes frame header and data)
    Frame = 5,
    /// Tile Group OBU
    TileGroup = 6,
    /// Metadata OBU
    Metadata = 7,
    /// Redundant Frame Header OBU
    RedundantFrameHeader = 8,
    /// Tile List OBU
    TileList = 9,
    /// Padding OBU
    Padding = 15,
}

impl ObuType {
    /// Create from u8 value.
    pub fn from_u8(value: u8) -> Self {
        match value {
            1 => ObuType::SequenceHeader,
            2 => ObuType::TemporalDelimiter,
            3 => ObuType::OverheadInfo,
            4 => ObuType::FrameHeader,
            5 => ObuType::Frame,
            6 => ObuType::TileGroup,
            7 => ObuType::Metadata,
            8 => ObuType::RedundantFrameHeader,
            9 => ObuType::TileList,
            15 => ObuType::Padding,
            _ => ObuType::Reserved,
        }
    }

    /// Check if this OBU type has a payload.
    pub fn has_payload(&self) -> bool {
        !matches!(
            self,
            ObuType::Reserved | ObuType::TemporalDelimiter | ObuType::Padding
        )
    }
}

/// OBU header for AV3.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObuHeader {
    /// OBU type
    pub obu_type: ObuType,
    /// OBU extension flag
    pub obu_extension_flag: bool,
    /// OBU has size field
    pub obu_has_size_field: bool,
    /// OBU temporal ID (if extension present)
    pub temporal_id: u8,
    /// OBU spatial ID (if extension present)
    pub spatial_id: u8,
}

/// OBU unit with header and payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObuUnit {
    /// OBU header
    pub header: ObuHeader,
    /// Byte offset in the original stream
    pub offset: u64,
    /// Raw payload bytes
    pub payload: Vec<u8>,
}

impl ObuUnit {
    /// Get OBU type
    pub fn obu_type(&self) -> ObuType {
        self.header.obu_type
    }
}

/// Find OBU units in data (looks for OBU header patterns).
pub fn find_obu_units(data: &[u8]) -> Vec<(usize, usize)> {
    let mut units = Vec::new();
    let mut i = 0;

    while i < data.len() {
        // Look for valid OBU header
        if i + 1 < data.len() {
            let byte0 = data[i];
            let _byte1 = data[i + 1];

            let obu_type = ObuType::from_u8((byte0 >> 3) & 0x0F);
            let _obu_extension_flag = (byte0 & 0x04) != 0;
            let obu_has_size_field = (byte0 & 0x02) != 0;

            // Check if this looks like a valid OBU
            if obu_type != ObuType::Reserved {
                let obu_start = i;
                let mut obu_end = i + 2; // At least header bytes

                if obu_has_size_field {
                    // Parse leb128 size
                    // SECURITY: Limit OBU payload size to prevent memory exhaustion
                    const MAX_OBU_PAYLOAD_SIZE: u64 = 100 * 1024 * 1024; // 100MB
                    let mut reader = BitReader::new(&data[i + 1..]);
                    if let Ok(size) = reader.read_leb128() {
                        // Validate size is within reasonable bounds
                        if size > MAX_OBU_PAYLOAD_SIZE {
                            // Skip oversized OBU to prevent DoS
                            i += 1;
                            continue;
                        }

                        // Safe conversion to usize (handles 32-bit systems)
                        let size_usize = if size > usize::MAX as u64 {
                            // On 32-bit systems, skip values that don't fit
                            i += 1;
                            continue;
                        } else {
                            size as usize
                        };

                        let payload_start = 1 + reader.byte_pos();
                        obu_end = obu_start + payload_start + size_usize;

                        // Make sure we don't go past the end
                        if obu_end > data.len() {
                            obu_end = data.len();
                        }
                    }
                } else {
                    // No size field, extends to end
                    obu_end = data.len();
                }

                units.push((obu_start, obu_end));
                i = obu_end;
                continue;
            }
        }

        i += 1;
    }

    units
}

/// Parse all OBU units from data.
pub fn parse_obu_units(data: &[u8]) -> Result<Vec<ObuUnit>> {
    let positions = find_obu_units(data);
    let mut units = Vec::with_capacity(positions.len());

    for (start, end) in positions {
        if end <= start || start >= data.len() {
            continue;
        }

        let obu_data = &data[start..end.min(data.len())];
        let header = parse_obu_header(obu_data)?;

        // Calculate payload start and size
        let mut reader = BitReader::new(&obu_data[1..]);
        let size = if header.obu_has_size_field {
            reader.read_leb128().ok()
        } else {
            None
        };

        let payload_offset = 1 + reader.byte_pos() + (if reader.bit_offset() > 0 { 1 } else { 0 });
        let payload_end = if let Some(s) = size {
            (payload_offset + s as usize).min(obu_data.len())
        } else {
            obu_data.len()
        };

        let payload = if payload_offset < obu_data.len() {
            obu_data[payload_offset..payload_end].to_vec()
        } else {
            Vec::new()
        };

        units.push(ObuUnit {
            header,
            offset: start as u64,
            payload,
        });
    }

    Ok(units)
}

/// Parse OBU header from data.
pub fn parse_obu_header(data: &[u8]) -> Result<ObuHeader> {
    if data.is_empty() {
        return Err(Av3Error::InsufficientData {
            expected: 1,
            actual: 0,
        });
    }

    let byte0 = data[0];
    let obu_type = ObuType::from_u8((byte0 >> 3) & 0x0F);
    let obu_extension_flag = (byte0 & 0x04) != 0;
    let obu_has_size_field = (byte0 & 0x02) != 0;

    let mut temporal_id = 0;
    let mut spatial_id = 0;

    if obu_extension_flag && data.len() >= 2 {
        let byte1 = data[1];
        // SECURITY: Validate temporal_id_plus1 is in valid range [1, 8] per AV1/AV3 spec
        // The lower 3 bits contain temporal_id_plus1 (not temporal_id directly)
        let temporal_id_plus1 = byte1 & 0x07;
        if temporal_id_plus1 == 0 {
            return Err(Av3Error::InvalidData(
                "OBU extension temporal_id_plus1 must be >= 1".to_string()
            ));
        }
        temporal_id = temporal_id_plus1 - 1; // Convert to temporal_id
        spatial_id = (byte1 >> 3) & 0x03;
    }

    Ok(ObuHeader {
        obu_type,
        obu_extension_flag,
        obu_has_size_field,
        temporal_id,
        spatial_id,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_obu_type_from_u8() {
        assert_eq!(ObuType::from_u8(1), ObuType::SequenceHeader);
        assert_eq!(ObuType::from_u8(4), ObuType::FrameHeader);
        assert_eq!(ObuType::from_u8(5), ObuType::Frame);
        assert_eq!(ObuType::from_u8(0xFF), ObuType::Reserved);
    }

    #[test]
    fn test_obu_type_has_payload() {
        assert!(ObuType::SequenceHeader.has_payload());
        assert!(ObuType::Frame.has_payload());
        assert!(!ObuType::TemporalDelimiter.has_payload());
        assert!(!ObuType::Reserved.has_payload());
    }
}
