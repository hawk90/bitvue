//! MPEG-2 Video Group of Pictures (GOP) header parsing.

use crate::bitreader::BitReader;
use crate::error::Result;
use serde::{Deserialize, Serialize};

/// Group of Pictures (GOP) header.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GopHeader {
    /// time_code - drop_frame_flag
    pub drop_frame_flag: bool,
    /// time_code - time_code_hours (5 bits)
    pub time_code_hours: u8,
    /// time_code - time_code_minutes (6 bits)
    pub time_code_minutes: u8,
    /// time_code - marker_bit
    pub marker_bit: bool,
    /// time_code - time_code_seconds (6 bits)
    pub time_code_seconds: u8,
    /// time_code - time_code_pictures (6 bits)
    pub time_code_pictures: u8,
    /// closed_gop
    pub closed_gop: bool,
    /// broken_link
    pub broken_link: bool,
}

impl GopHeader {
    /// Get time code as a formatted string (HH:MM:SS:FF).
    pub fn time_code_string(&self) -> String {
        format!(
            "{:02}:{:02}:{:02}:{:02}{}",
            self.time_code_hours,
            self.time_code_minutes,
            self.time_code_seconds,
            self.time_code_pictures,
            if self.drop_frame_flag { " (drop)" } else { "" }
        )
    }

    /// Calculate total frames from start (at 30fps for reference).
    pub fn total_frames_30fps(&self) -> u32 {
        let hours = self.time_code_hours as u32;
        let minutes = self.time_code_minutes as u32;
        let seconds = self.time_code_seconds as u32;
        let pictures = self.time_code_pictures as u32;

        (hours * 3600 + minutes * 60 + seconds) * 30 + pictures
    }
}

/// Parse GOP header from data after start code.
pub fn parse_gop_header(data: &[u8]) -> Result<GopHeader> {
    let mut reader = BitReader::new(data);

    // time_code (25 bits)
    let drop_frame_flag = reader.read_flag()?;
    let time_code_hours = reader.read_bits(5)? as u8;
    let time_code_minutes = reader.read_bits(6)? as u8;
    let marker_bit = reader.read_flag()?;
    let time_code_seconds = reader.read_bits(6)? as u8;
    let time_code_pictures = reader.read_bits(6)? as u8;

    let closed_gop = reader.read_flag()?;
    let broken_link = reader.read_flag()?;

    Ok(GopHeader {
        drop_frame_flag,
        time_code_hours,
        time_code_minutes,
        marker_bit,
        time_code_seconds,
        time_code_pictures,
        closed_gop,
        broken_link,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_code_string() {
        let gop = GopHeader {
            drop_frame_flag: false,
            time_code_hours: 1,
            time_code_minutes: 30,
            marker_bit: true,
            time_code_seconds: 45,
            time_code_pictures: 12,
            closed_gop: true,
            broken_link: false,
        };

        assert_eq!(gop.time_code_string(), "01:30:45:12");
    }
}
