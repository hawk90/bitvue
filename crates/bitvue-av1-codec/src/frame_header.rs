//! Frame header parsing for AV1
//!
//! Parses frame header OBUs to extract frame type and other metadata.
//!
//! # Limitations
//!
//! This is a **minimal parser** that extracts only the most basic frame information.
//! It does NOT implement the full AV1 uncompressed header parsing per spec.
//!
//! ## Known Limitations
//!
//! 1. **Approximate Bit Skipping**: The parser uses heuristic bit skips to reach
//!    certain fields (refresh_frame_flags, quantization_params). This is NOT
//!    spec-compliant and may misinterpret data for non-standard bitstreams.
//!
//! 2. **Missing Conditional Fields**: Many conditional fields in the uncompressed
//!    header are not parsed:
//!    - Frame size override
//!    - Screen content tools
//!    - Tile information
//!    - Loop filter parameters
//!    - CDEF parameters
//!    - Loop restoration parameters
//!
//! 3. **Estimated Header Size**: The `header_size_bytes` field is an estimate,
//!    not the actual uncompressed header size. This may cause incorrect tile
//!    data positioning in some cases.
//!
//! ## When to Use
//!
//! This parser is suitable for:
//! - Basic frame type detection
//! - Simple metadata extraction
//! - Prototyping and testing
//!
//! This parser is NOT suitable for:
//! - Bitstream validation
//! - Precise tile data extraction
//! - Decoding implementation
//!
//! ## Future Work
//!
//! A full implementation would require:
//! - Sequence header context
//! - Complete uncompressed_header() parsing per AV1 spec Section 5.9.2
//! - Proper handling of all conditional fields
//! - Accurate header size calculation
//!
//! Estimated effort: 3-4 hours of development + testing.

use crate::bitreader::BitReader;
use bitvue_core::BitvueError;
// Re-export FrameType for other modules in this crate
pub use bitvue_core::FrameType;

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
    let frame_type = FrameType::from_av1_bits(frame_type_bits);

    // show_frame (1 bit)
    let show_frame = reader.read_bit()?;

    // error_resilient_mode (1 bit) - AV1 Spec 5.9.8
    // Key frames shown are always error resilient (no bit in bitstream)
    let error_resilient_mode = match (frame_type, show_frame) {
        (FrameType::Key, true) => true,
        _ => reader.read_bit()?,
    };

    // Skip to refresh_frame_flags and ref_frame_idx
    // The uncompressed header has many conditional fields between here and there.
    //
    // PER AV1 SPEC SECTION 5.9.2 (uncompressed_header):
    // The fields between error_resilient_mode and refresh_frame_flags include:
    // - disable_cdf_update (1 bit, conditional)
    // - disable_frame_end_update_cdf (1 bit, conditional)
    // - tile_cols, tile_rows (variable, depending on sequence header)
    // - render_and_frame_size_different (1 bit)
    // - allow_screen_content_tools (1 bit, conditional)
    // - And many more conditional fields...
    //
    // This implementation uses a heuristic skip which is NOT spec-compliant.
    // TODO: Implement full uncompressed header parsing per AV1 spec (3-4 hours of work)
    //
    // The 20-bit skip is an approximation based on typical bitstream patterns.
    // This may misinterpret data for non-standard bitstreams.

    // Maximum bits to skip before giving up (prevents infinite loops on malformed data)
    let max_skip_bits: u32 = 20;
    let mut bits_skipped = 0;

    while bits_skipped < max_skip_bits {
        match reader.read_bit() {
            Ok(_) => bits_skipped += 1,
            Err(_) => {
                // Reached end of data before finding refresh_frame_flags
                // This is acceptable for truncated bitstreams
                break;
            }
        }
    }

    // refresh_frame_flags (8 bits) - present for all frame types
    let refresh_frame_flags = match reader.read_bits(8) {
        Ok(val) => Some(val as u8),
        Err(_) => None,
    };

    // ref_frame_idx[3] (9 bits) - only for inter frames
    let ref_frame_idx = match frame_type {
        FrameType::Inter => {
            let last = reader.read_bits(3).ok().map(|b| b as u8);
            let golden = reader.read_bits(3).ok().map(|b| b as u8);
            let altref = reader.read_bits(3).ok().map(|b| b as u8);

            match (last, golden, altref) {
                (Some(l), Some(g), Some(a)) => Some([l, g, a]),
                _ => None,
            }
        }
        _ => None,
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
    //
    // Per AV1 spec, maximum uncompressed header size is bounded by:
    // - Frame dimensions (width/height fields)
    // - Tile information (tile_cols * tile_rows)
    // - Reference frame lists (8 slots * 7 bytes each)
    // - Loop filter params
    // - Quantization params
    // - CDEF params
    // - LR params
    //
    // In practice, headers rarely exceed 200 bytes even for complex frames.
    const MAX_HEADER_SIZE_ESTIMATE: usize = 200;
    const CONSERVATIVE_PADDING: usize = 40;

    let header_size_bytes =
        (base_header_bytes + CONSERVATIVE_PADDING).min(MAX_HEADER_SIZE_ESTIMATE);

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
///
/// # AV1 Spec Context
///
/// Per AV1 Spec Section 5.9.17 (Quantization Params Syntax):
/// quantization_params() comes AFTER many conditional fields in the uncompressed header:
/// - loop_filter_params()
/// - cdef_params()
/// - lr_params()
/// - And potentially more...
///
/// The exact bit position depends on:
/// - Sequence header configuration (reduced_tx_set, etc.)
/// - Frame header flags (mode_ref_delta_enabled, etc.)
/// - Frame type (key vs inter)
///
/// This implementation uses a heuristic skip which is NOT spec-compliant.
/// TODO: Implement full uncompressed header parsing for accurate quantization params.
fn parse_quantization_params_simple(
    reader: &mut BitReader,
) -> (Option<u8>, Option<i8>, Option<i8>) {
    // Maximum bits to skip (prevents infinite loops on malformed data)
    // This is a rough estimate - actual position varies significantly
    let max_skip_bits: u32 = 20;
    let mut bits_skipped = 0;

    while bits_skipped < max_skip_bits {
        match reader.read_bit() {
            Ok(_) => bits_skipped += 1,
            Err(_) => {
                // Reached end of data before finding quantization params
                return (None, None, None);
            }
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
        assert_eq!(FrameType::from_av1_bits(0), FrameType::Key);
        assert_eq!(FrameType::from_av1_bits(1), FrameType::Inter);
        assert_eq!(FrameType::from_av1_bits(2), FrameType::IntraOnly);
        assert_eq!(FrameType::from_av1_bits(3), FrameType::Switch);
        assert_eq!(FrameType::from_av1_bits(4), FrameType::Unknown);
    }

    #[test]
    fn test_frame_type_names() {
        assert!(FrameType::Key.description().contains("Key"));
        assert!(FrameType::Inter.description().contains("Inter"));
        assert!(FrameType::IntraOnly.description().contains("Intra"));
        assert!(FrameType::Switch.description().contains("Switch"));
    }

    #[test]
    fn test_is_intra() {
        // Test is_intra() method - Key, IntraOnly, SI, SP are intra frames
        assert!(FrameType::Key.is_intra());
        assert!(!FrameType::Inter.is_intra());
        assert!(FrameType::IntraOnly.is_intra());
        assert!(!FrameType::Switch.is_intra()); // Switch frames are inter frames, not intra
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
