//! VC-3 / DNxHD segment structure parser.
//!
//! DNxHD frames begin with a fixed-size 640-byte header. The header starts
//! with a 4-byte signature (0x000002 0x80) followed by a frame header.
//!
//! DNxHR uses a variable-length header; we detect it by the extended syntax
//! bit in the compression ID.
//!
//! Reference: SMPTE ST 2019-4, VC-3 Part 3 (DNxHD/DNxHR specification).

use crate::error::{Result, Vc3Error};
use serde::{Deserialize, Serialize};

/// DNxHD/DNxHR compression IDs (partial list).
///
/// The IDs encode resolution, bit-rate class and chroma sampling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompId {
    /// DNxHD 1080p/1080i 220Mbps 8-bit
    Dnxhd1080p220,
    /// DNxHD 1080p/1080i 145Mbps 8-bit
    Dnxhd1080p145,
    /// DNxHD 1080p/1080i 36Mbps 8-bit
    Dnxhd1080p36,
    /// DNxHR SQ (Standard Quality)
    DnxhrSq,
    /// DNxHR HQ (High Quality)
    DnxhrHq,
    /// DNxHR HQX (10-bit)
    DnxhrHqx,
    /// DNxHR 444 (12-bit RGB)
    Dnxhr444,
    /// DNxHR LB (Low Bandwidth)
    DnxhrLb,
    Other(u32),
}

impl CompId {
    pub fn from_u32(v: u32) -> Self {
        match v {
            1235 => Self::Dnxhd1080p220,
            1237 => Self::Dnxhd1080p145,
            1238 => Self::Dnxhd1080p36,
            1256 => Self::DnxhrLb,
            1257 => Self::DnxhrSq,
            1258 => Self::DnxhrHq,
            1259 => Self::DnxhrHqx,
            1260 => Self::Dnxhr444,
            other => Self::Other(other),
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Dnxhd1080p220 => "DNxHD 220 1080",
            Self::Dnxhd1080p145 => "DNxHD 145 1080",
            Self::Dnxhd1080p36 => "DNxHD 36 1080",
            Self::DnxhrLb => "DNxHR LB",
            Self::DnxhrSq => "DNxHR SQ",
            Self::DnxhrHq => "DNxHR HQ",
            Self::DnxhrHqx => "DNxHR HQX",
            Self::Dnxhr444 => "DNxHR 444",
            Self::Other(_) => "DNxHD/HR (other)",
        }
    }

    /// Returns true if this is a DNxHR variant (variable-size header).
    pub fn is_dnxhr(self) -> bool {
        matches!(
            self,
            Self::DnxhrLb | Self::DnxhrSq | Self::DnxhrHq | Self::DnxhrHqx | Self::Dnxhr444
        )
    }
}

/// Parsed DNxHD/DNxHR frame header.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameHeader {
    /// Byte offset of the header in the input buffer.
    pub offset: usize,
    /// Compression ID.
    pub comp_id: CompId,
    /// Frame width in pixels.
    pub width: u32,
    /// Frame height in pixels.
    pub height: u32,
    /// Bits per component (8, 10, or 12).
    pub bits_per_component: u8,
    /// Chroma sampling (420, 422, or 444).
    pub chroma_sampling: u16,
    /// Total frame size in bytes (from header).
    pub frame_size: u32,
    /// Number of horizontal macroblocks per row.
    pub mb_cols: u32,
    /// Number of macroblock rows.
    pub mb_rows: u32,
    /// Average bits per macroblock (estimated).
    pub bits_per_mb: u32,
}

/// Magic bytes for a DNxHD/DNxHR frame header.
pub const DNXHD_MAGIC: [u8; 4] = [0x00, 0x00, 0x02, 0x80];

/// Parse a single DNxHD/DNxHR frame header starting at `offset` in `data`.
pub fn parse_frame_header(data: &[u8], offset: usize) -> Result<FrameHeader> {
    if offset + 640 > data.len() {
        return Err(Vc3Error::UnexpectedEof(offset));
    }

    // Check magic
    if data[offset..offset + 4] != DNXHD_MAGIC {
        return Err(Vc3Error::InvalidMagic(offset));
    }

    // Header layout (fixed 640-byte region):
    // Bytes  0-3:  Magic (0x00 0x00 0x02 0x80)
    // Bytes  4-7:  Frame size (big-endian uint32)
    // Bytes  8-11: Compression ID (big-endian uint32) — at byte 40 in some versions
    // Actual offsets per SMPTE ST 2019-4 Table 1:
    let frame_size = u32::from_be_bytes([
        data[offset + 4],
        data[offset + 5],
        data[offset + 6],
        data[offset + 7],
    ]);

    // Compression ID is at offset 40 within the 640-byte header
    let comp_id_raw = u32::from_be_bytes([
        data[offset + 40],
        data[offset + 41],
        data[offset + 42],
        data[offset + 43],
    ]);
    let comp_id = CompId::from_u32(comp_id_raw);

    // Width and height at offsets 24 and 26 (16-bit big-endian)
    let width = u16::from_be_bytes([data[offset + 24], data[offset + 25]]) as u32;
    let height = u16::from_be_bytes([data[offset + 26], data[offset + 27]]) as u32;

    // Bits per component at offset 43
    let bits_per_component = match data[offset + 43] & 0x03 {
        0 => 8,
        1 => 10,
        2 => 12,
        _ => 8,
    };

    // Chroma sampling — bit 2 of byte 44: 0 = 4:2:2, 1 = 4:4:4
    let chroma_sampling = if data[offset + 44] & 0x04 != 0 {
        444u16
    } else if data[offset + 44] & 0x02 != 0 {
        420u16
    } else {
        422u16
    };

    let mb_w = 16u32; // macroblock width in pixels
    let mb_h = 16u32;
    let mb_cols = if width > 0 { width.div_ceil(mb_w) } else { 0 };
    let mb_rows = if height > 0 { height.div_ceil(mb_h) } else { 0 };
    let num_mb = (mb_cols * mb_rows).max(1);
    let bits_per_mb = if frame_size > 0 {
        (frame_size as u64 * 8 / num_mb as u64) as u32
    } else {
        0
    };

    Ok(FrameHeader {
        offset,
        comp_id,
        width,
        height,
        bits_per_component,
        chroma_sampling,
        frame_size,
        mb_cols,
        mb_rows,
        bits_per_mb,
    })
}

/// Scan `data` for all DNxHD/DNxHR frame boundaries and parse their headers.
pub fn scan_segments(data: &[u8]) -> Vec<FrameHeader> {
    let mut headers = Vec::new();
    let mut i = 0;
    while i + 640 <= data.len() {
        if data[i..i + 4] == DNXHD_MAGIC {
            if let Ok(h) = parse_frame_header(data, i) {
                let advance = if h.frame_size > 640 {
                    h.frame_size as usize
                } else {
                    // Corrupt or unknown size — advance by minimum stride to avoid loop
                    640
                };
                headers.push(h);
                i += advance;
                continue;
            }
        }
        // Scan byte by byte for next magic
        i += 1;
    }
    headers
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_returns_no_segments() {
        assert!(scan_segments(&[]).is_empty());
    }

    #[test]
    fn no_magic_returns_no_segments() {
        let data = [0xAA; 1280];
        assert!(scan_segments(&data).is_empty());
    }
}
