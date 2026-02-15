//! MPEG-2 Video sequence header and extension parsing.

use crate::bitreader::BitReader;
use crate::error::{Mpeg2Error, Result};
use serde::{Deserialize, Serialize};

/// Chroma format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ChromaFormat {
    /// Reserved
    Reserved = 0,
    /// 4:2:0
    #[default]
    Yuv420 = 1,
    /// 4:2:2
    Yuv422 = 2,
    /// 4:4:4
    Yuv444 = 3,
}

impl ChromaFormat {
    /// Create from raw value.
    pub fn from_u8(value: u8) -> Self {
        match value {
            0 => ChromaFormat::Reserved,
            1 => ChromaFormat::Yuv420,
            2 => ChromaFormat::Yuv422,
            3 => ChromaFormat::Yuv444,
            _ => ChromaFormat::Yuv420,
        }
    }
}

/// Frame rate code to actual frame rate mapping.
const FRAME_RATES: [(f64, &str); 9] = [
    (0.0, "forbidden"),
    (23.976, "23.976 (24000/1001)"),
    (24.0, "24"),
    (25.0, "25"),
    (29.97, "29.97 (30000/1001)"),
    (30.0, "30"),
    (50.0, "50"),
    (59.94, "59.94 (60000/1001)"),
    (60.0, "60"),
];

/// Aspect ratio code to string mapping.
const ASPECT_RATIOS: [&str; 5] = ["forbidden", "1:1 (Square)", "4:3", "16:9", "2.21:1"];

/// Sequence header.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SequenceHeader {
    /// horizontal_size_value (12 bits)
    pub horizontal_size_value: u16,
    /// vertical_size_value (12 bits)
    pub vertical_size_value: u16,
    /// aspect_ratio_information (4 bits)
    pub aspect_ratio_information: u8,
    /// frame_rate_code (4 bits)
    pub frame_rate_code: u8,
    /// bit_rate_value (18 bits)
    pub bit_rate_value: u32,
    /// vbv_buffer_size_value (10 bits)
    pub vbv_buffer_size_value: u16,
    /// constrained_parameters_flag
    pub constrained_parameters_flag: bool,
    /// load_intra_quantiser_matrix
    pub load_intra_quantiser_matrix: bool,
    /// intra_quantiser_matrix (optional, 64 bytes)
    pub intra_quantiser_matrix: Option<Vec<u8>>,
    /// load_non_intra_quantiser_matrix
    pub load_non_intra_quantiser_matrix: bool,
    /// non_intra_quantiser_matrix (optional, 64 bytes)
    pub non_intra_quantiser_matrix: Option<Vec<u8>>,
}

impl SequenceHeader {
    /// Get frame rate from frame_rate_code.
    pub fn frame_rate(&self) -> f64 {
        let idx = (self.frame_rate_code as usize).min(FRAME_RATES.len() - 1);
        FRAME_RATES[idx].0
    }

    /// Get frame rate string.
    pub fn frame_rate_string(&self) -> &'static str {
        let idx = (self.frame_rate_code as usize).min(FRAME_RATES.len() - 1);
        FRAME_RATES[idx].1
    }

    /// Get aspect ratio string.
    pub fn aspect_ratio_string(&self) -> &'static str {
        let idx = (self.aspect_ratio_information as usize).min(ASPECT_RATIOS.len() - 1);
        ASPECT_RATIOS[idx]
    }
}

/// Sequence extension (MPEG-2 only).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SequenceExtension {
    /// profile_and_level_indication (8 bits)
    pub profile_and_level_indication: u8,
    /// progressive_sequence
    pub progressive_sequence: bool,
    /// chroma_format (2 bits)
    pub chroma_format: ChromaFormat,
    /// horizontal_size_extension (2 bits)
    pub horizontal_size_extension: u8,
    /// vertical_size_extension (2 bits)
    pub vertical_size_extension: u8,
    /// bit_rate_extension (12 bits)
    pub bit_rate_extension: u16,
    /// vbv_buffer_size_extension (8 bits)
    pub vbv_buffer_size_extension: u8,
    /// low_delay
    pub low_delay: bool,
    /// frame_rate_extension_n (2 bits)
    pub frame_rate_extension_n: u8,
    /// frame_rate_extension_d (5 bits)
    pub frame_rate_extension_d: u8,
}

impl SequenceExtension {
    /// Get profile name.
    pub fn profile_name(&self) -> &'static str {
        match self.profile_and_level_indication >> 4 {
            1 => "High",
            2 => "Spatially Scalable",
            3 => "SNR Scalable",
            4 => "Main",
            5 => "Simple",
            _ => "Unknown",
        }
    }

    /// Get level name.
    pub fn level_name(&self) -> &'static str {
        match self.profile_and_level_indication & 0x0F {
            4 => "High",
            6 => "High 1440",
            8 => "Main",
            10 => "Low",
            _ => "Unknown",
        }
    }
}

/// Parse sequence header from data after start code.
pub fn parse_sequence_header(data: &[u8]) -> Result<SequenceHeader> {
    let mut reader = BitReader::new(data);

    // SECURITY: Validate dimensions to prevent excessive allocation
    // MPEG-2 spec uses 12-bit fields for dimensions (max 4095)
    const MAX_MPEG2_DIMENSION: u16 = 4095;
    const MIN_MPEG2_DIMENSION: u16 = 1;

    let horizontal_size_value = reader.read_bits(12)? as u16;
    if horizontal_size_value < MIN_MPEG2_DIMENSION || horizontal_size_value > MAX_MPEG2_DIMENSION {
        return Err(Mpeg2Error::InvalidSequenceHeader(format!(
            "horizontal_size_value {} must be between {} and {}",
            horizontal_size_value, MIN_MPEG2_DIMENSION, MAX_MPEG2_DIMENSION
        )));
    }

    let vertical_size_value = reader.read_bits(12)? as u16;
    if vertical_size_value < MIN_MPEG2_DIMENSION || vertical_size_value > MAX_MPEG2_DIMENSION {
        return Err(Mpeg2Error::InvalidSequenceHeader(format!(
            "vertical_size_value {} must be between {} and {}",
            vertical_size_value, MIN_MPEG2_DIMENSION, MAX_MPEG2_DIMENSION
        )));
    }

    let aspect_ratio_information = reader.read_bits(4)? as u8;
    let frame_rate_code = reader.read_bits(4)? as u8;
    let bit_rate_value = reader.read_bits(18)?;
    let _marker_bit = reader.read_bit()?;
    let vbv_buffer_size_value = reader.read_bits(10)? as u16;
    let constrained_parameters_flag = reader.read_flag()?;

    let load_intra_quantiser_matrix = reader.read_flag()?;
    let intra_quantiser_matrix = if load_intra_quantiser_matrix {
        let mut matrix = Vec::with_capacity(64);
        for _ in 0..64 {
            matrix.push(reader.read_bits(8)? as u8);
        }
        Some(matrix)
    } else {
        None
    };

    let load_non_intra_quantiser_matrix = reader.read_flag()?;
    let non_intra_quantiser_matrix = if load_non_intra_quantiser_matrix {
        let mut matrix = Vec::with_capacity(64);
        for _ in 0..64 {
            matrix.push(reader.read_bits(8)? as u8);
        }
        Some(matrix)
    } else {
        None
    };

    Ok(SequenceHeader {
        horizontal_size_value,
        vertical_size_value,
        aspect_ratio_information,
        frame_rate_code,
        bit_rate_value,
        vbv_buffer_size_value,
        constrained_parameters_flag,
        load_intra_quantiser_matrix,
        intra_quantiser_matrix,
        load_non_intra_quantiser_matrix,
        non_intra_quantiser_matrix,
    })
}

/// Parse sequence extension from data after extension start code.
pub fn parse_sequence_extension(data: &[u8]) -> Result<SequenceExtension> {
    let mut reader = BitReader::new(data);

    // extension_start_code_identifier (4 bits) - should be 1
    let _ext_id = reader.read_bits(4)?;

    let profile_and_level_indication = reader.read_bits(8)? as u8;
    let progressive_sequence = reader.read_flag()?;
    let chroma_format = ChromaFormat::from_u8(reader.read_bits(2)? as u8);
    let horizontal_size_extension = reader.read_bits(2)? as u8;
    let vertical_size_extension = reader.read_bits(2)? as u8;
    let bit_rate_extension = reader.read_bits(12)? as u16;
    let _marker_bit = reader.read_bit()?;
    let vbv_buffer_size_extension = reader.read_bits(8)? as u8;
    let low_delay = reader.read_flag()?;
    let frame_rate_extension_n = reader.read_bits(2)? as u8;
    let frame_rate_extension_d = reader.read_bits(5)? as u8;

    Ok(SequenceExtension {
        profile_and_level_indication,
        progressive_sequence,
        chroma_format,
        horizontal_size_extension,
        vertical_size_extension,
        bit_rate_extension,
        vbv_buffer_size_extension,
        low_delay,
        frame_rate_extension_n,
        frame_rate_extension_d,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_rate() {
        let header = SequenceHeader {
            horizontal_size_value: 720,
            vertical_size_value: 480,
            aspect_ratio_information: 2,
            frame_rate_code: 4, // 29.97
            bit_rate_value: 0,
            vbv_buffer_size_value: 0,
            constrained_parameters_flag: false,
            load_intra_quantiser_matrix: false,
            intra_quantiser_matrix: None,
            load_non_intra_quantiser_matrix: false,
            non_intra_quantiser_matrix: None,
        };

        assert!((header.frame_rate() - 29.97).abs() < 0.01);
    }
}
