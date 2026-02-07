//! MPEG-2 Video slice header parsing.

use crate::bitreader::BitReader;
use crate::error::Result;
use serde::{Deserialize, Serialize};

/// Slice header.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SliceHeader {
    /// slice_vertical_position_extension (for HD, 3 bits)
    pub slice_vertical_position_extension: Option<u8>,
    /// priority_breakpoint (if priority_breakpoint is used)
    pub priority_breakpoint: Option<u8>,
    /// quantiser_scale_code (5 bits)
    pub quantiser_scale_code: u8,
    /// intra_slice_flag
    pub intra_slice_flag: bool,
    /// intra_slice
    pub intra_slice: bool,
    /// reserved_bits (7 bits)
    pub reserved_bits: u8,
    /// extra_bit_slice count
    pub extra_information_count: u32,
}

impl SliceHeader {
    /// Get quantiser scale value based on q_scale_type.
    /// If q_scale_type is false (linear), scale = 2 * quantiser_scale_code
    /// If q_scale_type is true (non-linear), uses lookup table
    pub fn quantiser_scale(&self, q_scale_type: bool) -> u8 {
        if !q_scale_type {
            // Linear scale
            self.quantiser_scale_code * 2
        } else {
            // Non-linear scale
            NON_LINEAR_QSCALE[self.quantiser_scale_code as usize]
        }
    }
}

/// Non-linear quantiser scale lookup table.
const NON_LINEAR_QSCALE: [u8; 32] = [
    0, 1, 2, 3, 4, 5, 6, 7, 8, 10, 12, 14, 16, 18, 20, 22, 24, 28, 32, 36, 40, 44, 48, 52, 56, 64,
    72, 80, 88, 96, 104, 112,
];

/// Parse slice header from data after slice start code.
pub fn parse_slice_header(data: &[u8]) -> Result<SliceHeader> {
    let mut reader = BitReader::new(data);

    // Check for slice_vertical_position_extension (for HD content)
    // This is present when vertical_size > 2800
    let slice_vertical_position_extension = None; // Would need context to determine

    // Check for priority_breakpoint (sequence_scalable_extension)
    let priority_breakpoint = None; // Would need context

    let quantiser_scale_code = reader.read_bits(5)? as u8;

    let mut intra_slice_flag = false;
    let mut intra_slice = false;
    let mut reserved_bits = 0u8;

    // Check for slice extension
    if reader.peek_bits(1)? == 1 {
        intra_slice_flag = reader.read_flag()?;
        intra_slice = reader.read_flag()?;
        reserved_bits = reader.read_bits(7)? as u8;
    }

    // Skip extra_bit_slice data
    // SECURITY: Limit iterations to prevent DoS via malicious files
    const MAX_SLICE_EXTRA_COUNT: u32 = 1000;
    let mut extra_information_count = 0;

    while extra_information_count < MAX_SLICE_EXTRA_COUNT && reader.has_more_data() {
        if let Ok(extra_bit) = reader.read_flag() {
            if extra_bit {
                let _ = reader.read_bits(8)?;
                extra_information_count += 1;
            } else {
                break;
            }
        } else {
            break;
        }
    }

    Ok(SliceHeader {
        slice_vertical_position_extension,
        priority_breakpoint,
        quantiser_scale_code,
        intra_slice_flag,
        intra_slice,
        reserved_bits,
        extra_information_count,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quantiser_scale() {
        let header = SliceHeader {
            slice_vertical_position_extension: None,
            priority_breakpoint: None,
            quantiser_scale_code: 10,
            intra_slice_flag: false,
            intra_slice: false,
            reserved_bits: 0,
            extra_information_count: 0,
        };

        // Linear: 10 * 2 = 20
        assert_eq!(header.quantiser_scale(false), 20);

        // Non-linear: lookup table index 10 = 12
        assert_eq!(header.quantiser_scale(true), 12);
    }
}
