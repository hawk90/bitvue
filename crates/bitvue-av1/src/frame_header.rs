//! Frame header parsing for AV1
//!
//! Parses frame header OBUs to extract frame type and other metadata.

use crate::bitreader::BitReader;
use bitvue_core::BitvueError;
use serde::{Deserialize, Serialize};

/// Frame type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FrameType {
    /// Key frame (intra-only, no dependencies)
    Key = 0,
    /// Inter frame (may use references)
    Inter = 1,
    /// Intra-only frame (intra-only but not a key frame)
    IntraOnly = 2,
    /// Switch frame (intra-only, allows switching between streams)
    Switch = 3,
}

impl FrameType {
    /// Parse frame type from 2-bit value
    pub fn from_bits(bits: u32) -> Result<Self, BitvueError> {
        match bits {
            0 => Ok(FrameType::Key),
            1 => Ok(FrameType::Inter),
            2 => Ok(FrameType::IntraOnly),
            3 => Ok(FrameType::Switch),
            _ => Err(BitvueError::InvalidData(format!(
                "Invalid frame type: {}",
                bits
            ))),
        }
    }

    /// Get human-readable name
    pub fn name(&self) -> &'static str {
        match self {
            FrameType::Key => "KEY_FRAME",
            FrameType::Inter => "INTER_FRAME",
            FrameType::IntraOnly => "INTRA_ONLY_FRAME",
            FrameType::Switch => "SWITCH_FRAME",
        }
    }

    /// Check if this is an intra-only frame (Key, IntraOnly, or Switch)
    pub fn is_intra_only(&self) -> bool {
        matches!(
            self,
            FrameType::Key | FrameType::IntraOnly | FrameType::Switch
        )
    }
}

/// Minimal frame header information
#[derive(Debug, Clone)]
pub struct FrameHeader {
    /// Frame type
    pub frame_type: FrameType,
    /// Show frame flag
    pub show_frame: bool,
    /// Show existing frame flag
    pub show_existing_frame: bool,
    /// Frame to show (if show_existing_frame is true)
    pub frame_to_show_map_idx: Option<u8>,
    /// Error resilient mode
    pub error_resilient_mode: bool,
    /// Base quantization index (0-255)
    pub base_q_idx: Option<u8>,
    /// Y DC delta Q
    pub y_dc_delta_q: Option<i8>,
    /// UV DC delta Q
    pub uv_dc_delta_q: Option<i8>,
    /// Delta Q params (whether delta Q is enabled for this frame)
    /// Per AV1 Spec Section 5.9.17 (Delta Q Params Syntax)
    pub delta_q_present: bool,
    /// Delta Q residue (1 bit if delta_q_residue_enabled in sequence header)
    pub delta_q_residue: Option<bool>,
    /// Uncompressed header size in bytes (for finding tile data start)
    pub header_size_bytes: usize,
    /// Refresh frame flags (8 bits) - which reference slots to refresh
    pub refresh_frame_flags: Option<u8>,
    /// Reference frame indices [LAST, GOLDEN, ALTREF] (3 bits each)
    pub ref_frame_idx: Option<[u8; 3]>,
}

/// Parse frame header from payload
///
/// This is a minimal parser that only extracts frame type and basic flags.
/// Full frame header parsing would require sequence header context.
pub fn parse_frame_header_basic(payload: &[u8]) -> Result<FrameHeader, BitvueError> {
    let mut reader = BitReader::new(payload);

    // show_existing_frame (1 bit)
    let show_existing_frame = reader.read_bit()?;

    if show_existing_frame {
        // If showing existing frame, read which frame to show
        let frame_to_show_map_idx = reader.read_bits(3)? as u8;

        // For show_existing_frame, we don't have frame_type in the bitstream
        // but we know it's showing a previously decoded frame

        // Calculate header size (align to byte boundary)
        let header_size_bytes = reader.byte_position()
            + if !reader.position().is_multiple_of(8) {
                1
            } else {
                0
            };

        return Ok(FrameHeader {
            frame_type: FrameType::Key, // Default to KEY for show_existing_frame
            show_frame: true,
            show_existing_frame: true,
            frame_to_show_map_idx: Some(frame_to_show_map_idx),
            error_resilient_mode: false,
            base_q_idx: None,
            y_dc_delta_q: None,
            uv_dc_delta_q: None,
            delta_q_present: false,
            delta_q_residue: None,
            header_size_bytes,
            refresh_frame_flags: None,
            ref_frame_idx: None,
        });
    }

    // frame_type (2 bits)
    let frame_type_bits = reader.read_bits(2)?;
    let frame_type = FrameType::from_bits(frame_type_bits)?;

    // show_frame (1 bit)
    let show_frame = reader.read_bit()?;

    // error_resilient_mode (1 bit) - only if not Key or IntraOnly
    let error_resilient_mode = if show_frame && frame_type == FrameType::Key {
        true // Key frames shown are always error resilient in this context
    } else {
        reader.read_bit()?
    };

    // Skip to refresh_frame_flags and ref_frame_idx
    // The uncompressed header has many conditional fields between here and there
    // For simplicity, we'll use a heuristic approach

    // Skip some fields to reach refresh_frame_flags
    // This is approximate - in a full implementation we'd parse all conditional fields
    let bits_to_skip = 20; // Approximate bits to skip to reach refresh_frame_flags
    for _ in 0..bits_to_skip {
        if reader.read_bit().is_err() {
            break;
        }
    }

    // refresh_frame_flags (8 bits) - present for all frame types
    let refresh_frame_flags = match reader.read_bits(8) {
        Ok(val) => Some(val as u8),
        Err(_) => None,
    };

    // ref_frame_idx[3] (9 bits) - only for inter frames
    let ref_frame_idx = if frame_type == FrameType::Inter {
        let last = reader.read_bits(3).ok().map(|b| b as u8);
        let golden = reader.read_bits(3).ok().map(|b| b as u8);
        let altref = reader.read_bits(3).ok().map(|b| b as u8);

        match (last, golden, altref) {
            (Some(l), Some(g), Some(a)) => Some([l, g, a]),
            _ => None,
        }
    } else {
        None
    };

    // TODO: Parse full uncompressed header to get exact size
    // For now, use a conservative estimate based on what we've parsed so far
    // Typical AV1 frame headers range from 6 to 60+ bytes depending on features
    // We'll use the current byte position + estimated remaining fields
    let base_header_bytes = reader.byte_position()
        + if !reader.position().is_multiple_of(8) {
            1
        } else {
            0
        };

    // Try to parse quantization parameters
    // This is a simplified version that may not work for all cases
    // Full parsing requires sequence header context
    let (base_q_idx, y_dc_delta_q, uv_dc_delta_q) = parse_quantization_params_simple(&mut reader);

    // Estimate header size: base parsed fields + typical remaining fields
    // Conservatively add 40 bytes for unparsed fields (ref frames, MV params, quantization, loop filter, etc.)
    let header_size_bytes = base_header_bytes + 40;

    Ok(FrameHeader {
        frame_type,
        show_frame,
        show_existing_frame: false,
        frame_to_show_map_idx: None,
        error_resilient_mode,
        base_q_idx,
        y_dc_delta_q,
        uv_dc_delta_q,
        delta_q_present: false, // TODO: Parse delta_q_params from frame header
        delta_q_residue: None,
        header_size_bytes,
        refresh_frame_flags,
        ref_frame_idx,
    })
}

/// Simplified quantization parameter parsing
///
/// This attempts to skip to and parse quantization parameters.
/// Note: This is a heuristic approach and may not work for all bitstreams.
fn parse_quantization_params_simple(
    reader: &mut BitReader,
) -> (Option<u8>, Option<i8>, Option<i8>) {
    // Skip many conditional fields in frame header to reach quantization_params
    // This is highly simplified and will only work for specific bitstream patterns

    // For a more robust implementation, we would need:
    // - Sequence header context
    // - Full uncompressed_header() parsing
    // - Conditional field handling

    // For now, attempt to read at a heuristic position
    // In practice, quantization_params comes after many conditional fields

    // Try to skip ahead (this is a rough estimate)
    // Actual position varies based on frame type and flags
    for _ in 0..20 {
        if reader.read_bit().is_err() {
            return (None, None, None);
        }
    }

    // Try to read base_q_idx (8 bits)
    let base_q_idx = match reader.read_bits(8) {
        Ok(val) => Some(val as u8),
        Err(_) => None,
    };

    // Skip delta Q parsing for now (too complex without full context)
    // In real implementation, we'd check DeltaQYDc flag and read signed delta

    (base_q_idx, None, None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_type_from_bits() {
        assert_eq!(FrameType::from_bits(0).unwrap(), FrameType::Key);
        assert_eq!(FrameType::from_bits(1).unwrap(), FrameType::Inter);
        assert_eq!(FrameType::from_bits(2).unwrap(), FrameType::IntraOnly);
        assert_eq!(FrameType::from_bits(3).unwrap(), FrameType::Switch);
        assert!(FrameType::from_bits(4).is_err());
    }

    #[test]
    fn test_frame_type_names() {
        assert_eq!(FrameType::Key.name(), "KEY_FRAME");
        assert_eq!(FrameType::Inter.name(), "INTER_FRAME");
        assert_eq!(FrameType::IntraOnly.name(), "INTRA_ONLY_FRAME");
        assert_eq!(FrameType::Switch.name(), "SWITCH_FRAME");
    }

    #[test]
    fn test_is_intra_only() {
        assert!(FrameType::Key.is_intra_only());
        assert!(!FrameType::Inter.is_intra_only());
        assert!(FrameType::IntraOnly.is_intra_only());
        assert!(FrameType::Switch.is_intra_only());
    }

    #[test]
    fn test_parse_key_frame() {
        // show_existing_frame=0, frame_type=00 (KEY), show_frame=1
        // Binary: 0 00 1 0000 = 0x10
        let payload = [0b0001_0000];
        let header = parse_frame_header_basic(&payload).unwrap();
        assert_eq!(header.frame_type, FrameType::Key);
        assert!(header.show_frame);
        assert!(!header.show_existing_frame);
    }

    #[test]
    fn test_parse_inter_frame() {
        // show_existing_frame=0, frame_type=01 (INTER), show_frame=1, error_resilient=0
        // Binary: 0 01 1 0 000 = 0x30
        let payload = [0b0011_0000];
        let header = parse_frame_header_basic(&payload).unwrap();
        assert_eq!(header.frame_type, FrameType::Inter);
        assert!(header.show_frame);
        assert!(!header.error_resilient_mode);
    }

    #[test]
    fn test_parse_show_existing_frame() {
        // show_existing_frame=1, frame_to_show_map_idx=101 (5)
        // Binary: 1 101 0000 = 0xD0
        let payload = [0b1101_0000];
        let header = parse_frame_header_basic(&payload).unwrap();
        assert!(header.show_existing_frame);
        assert_eq!(header.frame_to_show_map_idx, Some(5));
    }
}
