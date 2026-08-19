//! AVS3 Sequence Header parsing (GB/T 33475.3 §6.2.2).
//!
//! The sequence header carries the codec profile/level, frame dimensions,
//! chroma format, bit depth, and high-level filter flags.

use crate::bitreader::BitReader;
use crate::error::{Avs3Error, Result};
use serde::{Deserialize, Serialize};

/// AVS3 profile identifiers (profile_id field).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Avs3Profile {
    /// Main 8-bit profile (0x20)
    Main8 = 0x20,
    /// Main 10-bit profile (0x22)
    Main10 = 0x22,
    /// High 8-bit profile (0x30)
    High8 = 0x30,
    /// High 10-bit profile (0x32)
    High10 = 0x32,
    /// Unknown
    Unknown(u8),
}

impl From<u8> for Avs3Profile {
    fn from(v: u8) -> Self {
        match v {
            0x20 => Self::Main8,
            0x22 => Self::Main10,
            0x30 => Self::High8,
            0x32 => Self::High10,
            other => Self::Unknown(other),
        }
    }
}

impl Avs3Profile {
    pub fn bit_depth(&self) -> u8 {
        match self {
            Self::Main10 | Self::High10 => 10,
            _ => 8,
        }
    }
}

/// Chroma format (chroma_format field, 2 bits).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChromaFormat {
    Monochrome = 0,
    Yuv420 = 1,
    Yuv422 = 2,
    Yuv444 = 3,
}

impl From<u8> for ChromaFormat {
    fn from(v: u8) -> Self {
        match v {
            0 => Self::Monochrome,
            1 => Self::Yuv420,
            2 => Self::Yuv422,
            3 => Self::Yuv444,
            _ => Self::Yuv420, // fallback
        }
    }
}

/// AVS3 Sequence Header.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SequenceHeader {
    /// Profile identifier (0x20 = Main8, 0x22 = Main10, …)
    pub profile: Avs3Profile,
    /// Level identifier (1..=??)
    pub level: u8,
    /// Progressive sequence flag
    pub progressive_sequence: bool,
    /// Coded width in luma samples
    pub width: u32,
    /// Coded height in luma samples
    pub height: u32,
    /// Chroma format
    pub chroma_format: ChromaFormat,
    /// Internal bit depth (display_primitive_flag = 0 → 8 or 10 bit)
    pub bit_depth_luma: u8,
    pub bit_depth_chroma: u8,
    /// Sample aspect ratio (sar_width : sar_height)
    pub sar_width: u16,
    pub sar_height: u16,
    /// Frame rate code (see AVS3 spec Table 7-5)
    pub frame_rate_code: u8,
    /// Bit rate (upper/lower combined, informative)
    pub bit_rate: u32,
    /// Low delay flag
    pub low_delay: bool,
    /// Temporal ID nesting flag
    pub temporal_id_nesting: bool,
    // ── In-loop filter flags ──────────────────────────────────────────────────
    /// CDEF-like deblocking filter enabled
    pub deblocking_filter_flag: bool,
    /// SAO (Sample Adaptive Offset) enabled
    pub sample_adaptive_offset_enabled: bool,
    /// ESAO (Enhanced SAO) enabled — AVS3 specific
    pub adaptive_leveling_filter_enabled: bool,
    /// CCSAO (Cross-Component SAO) enabled — AVS3 specific
    pub cross_component_prediction_enabled: bool,
    /// ALF (Adaptive Loop Filter) enabled
    pub adaptive_loop_filter_enabled: bool,
}

/// Parse AVS3 Sequence Header from the NAL unit payload.
///
/// `data` should start at the SCI byte (0xB0) and contain all bytes up to
/// the next start code.
pub fn parse_sequence_header(data: &[u8]) -> Result<SequenceHeader> {
    if data.len() < 5 {
        return Err(Avs3Error::TooShort {
            need: 5,
            have: data.len(),
        });
    }

    // Skip SCI byte (data[0] = 0xB0)
    let mut r = BitReader::new(&data[1..]);

    let profile_raw = r.read_bits(8)? as u8;
    let profile = Avs3Profile::from(profile_raw);

    let level = r.read_bits(8)? as u8;

    let progressive_sequence = r.read_flag()?;
    let _field_coded_sequence = r.read_flag()?; // ignored for now

    // Horizontal/vertical size (14 bits each in baseline profile)
    let width = r.read_bits(14)?;
    let height = r.read_bits(14)?;

    let chroma_format_raw = r.read_bits(2)? as u8;
    let chroma_format = ChromaFormat::from(chroma_format_raw);

    let sample_precision = r.read_bits(3)? as u8; // 1=8bit, 2=10bit
    let bit_depth_luma = if sample_precision == 2 { 10 } else { 8 };
    let bit_depth_chroma = bit_depth_luma;

    let _encoding_precision = r.read_bits(3)?;

    // Aspect ratio (4 bits SAR code; we decode to explicit ratio for storage)
    let sar_code = r.read_bits(4)? as u8;
    let (sar_width, sar_height) = sar_from_code(sar_code);

    let frame_rate_code = r.read_bits(4)? as u8;

    // Bit rate: lower 18 bits + marker + upper 12 bits
    let bit_rate_lower = r.read_bits(18)?;
    let _marker = r.read_flag()?;
    let bit_rate_upper = r.read_bits(12)?;
    let bit_rate = (bit_rate_upper << 18) | bit_rate_lower;

    let low_delay = r.read_flag()?;
    let _marker2 = r.read_flag()?;

    let temporal_id_nesting = r.read_flag()?;

    // Skip BBV buffer size (18 bits)
    let _ = r.read_bits(18)?;

    // In-loop filter enable flags
    let deblocking_filter_flag = r.read_flag()?;
    let sample_adaptive_offset_enabled = r.read_flag()?;
    let adaptive_leveling_filter_enabled = r.read_flag()?; // ESAO
    let cross_component_prediction_enabled = r.read_flag()?; // CCSAO
    let adaptive_loop_filter_enabled = r.read_flag()?;

    Ok(SequenceHeader {
        profile,
        level,
        progressive_sequence,
        width,
        height,
        chroma_format,
        bit_depth_luma,
        bit_depth_chroma,
        sar_width,
        sar_height,
        frame_rate_code,
        bit_rate,
        low_delay,
        temporal_id_nesting,
        deblocking_filter_flag,
        sample_adaptive_offset_enabled,
        adaptive_leveling_filter_enabled,
        cross_component_prediction_enabled,
        adaptive_loop_filter_enabled,
    })
}

/// Convert AVS3 SAR code to explicit (width:height) pair.
/// AVS3 spec Table 7-4.
fn sar_from_code(code: u8) -> (u16, u16) {
    match code {
        1 => (1, 1),
        2 => (4, 3),
        3 => (16, 9),
        4 => (221, 100),
        _ => (0, 0), // unspecified
    }
}

/// Nominal frame rate from frame_rate_code.
/// AVS3 spec Table 7-5.
pub fn frame_rate_from_code(code: u8) -> Option<f64> {
    match code {
        1 => Some(24000.0 / 1001.0),
        2 => Some(24.0),
        3 => Some(25.0),
        4 => Some(30000.0 / 1001.0),
        5 => Some(30.0),
        6 => Some(50.0),
        7 => Some(60000.0 / 1001.0),
        8 => Some(60.0),
        _ => None,
    }
}
