//! MPEG-2 Video start code detection and parsing.

use serde::{Deserialize, Serialize};

/// MPEG-2 start code types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StartCodeType {
    /// Picture start code (0x00).
    Picture,
    /// Slice start codes (0x01-0xAF).
    Slice(u8),
    /// Reserved (0xB0).
    Reserved0,
    /// Reserved (0xB1).
    Reserved1,
    /// User data start code (0xB2).
    UserData,
    /// Sequence header start code (0xB3).
    SequenceHeader,
    /// Sequence error start code (0xB4).
    SequenceError,
    /// Extension start code (0xB5).
    Extension,
    /// Reserved (0xB6).
    Reserved6,
    /// Sequence end code (0xB7).
    SequenceEnd,
    /// Group of pictures start code (0xB8).
    GroupOfPictures,
    /// System start codes (0xB9-0xFF).
    System(u8),
}

impl StartCodeType {
    /// Create from raw byte value.
    pub fn from_u8(value: u8) -> Self {
        match value {
            0x00 => StartCodeType::Picture,
            0x01..=0xAF => StartCodeType::Slice(value),
            0xB0 => StartCodeType::Reserved0,
            0xB1 => StartCodeType::Reserved1,
            0xB2 => StartCodeType::UserData,
            0xB3 => StartCodeType::SequenceHeader,
            0xB4 => StartCodeType::SequenceError,
            0xB5 => StartCodeType::Extension,
            0xB6 => StartCodeType::Reserved6,
            0xB7 => StartCodeType::SequenceEnd,
            0xB8 => StartCodeType::GroupOfPictures,
            _ => StartCodeType::System(value),
        }
    }

    /// Get human-readable name.
    pub fn name(&self) -> &'static str {
        match self {
            StartCodeType::Picture => "Picture",
            StartCodeType::Slice(_) => "Slice",
            StartCodeType::Reserved0 | StartCodeType::Reserved1 | StartCodeType::Reserved6 => {
                "Reserved"
            }
            StartCodeType::UserData => "User Data",
            StartCodeType::SequenceHeader => "Sequence Header",
            StartCodeType::SequenceError => "Sequence Error",
            StartCodeType::Extension => "Extension",
            StartCodeType::SequenceEnd => "Sequence End",
            StartCodeType::GroupOfPictures => "GOP",
            StartCodeType::System(_) => "System",
        }
    }
}

/// Parsed start code.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartCode {
    /// Start code type.
    pub code_type: StartCodeType,
    /// Raw byte value (the byte after 0x000001).
    pub code_value: u8,
}

/// Find all start codes in MPEG-2 bitstream.
/// Returns tuples of (byte offset, StartCode).
///
/// SECURITY: Limits scan distance to prevent DoS via malicious files with
/// long sequences of non-start-code bytes.
pub fn find_start_codes(data: &[u8]) -> Vec<(usize, StartCode)> {
    // SECURITY: Limit scan distance to prevent DoS via unbounded loops
    // Similar to other NAL parsers - max 100MB scan per start code
    const MAX_START_CODE_SCAN_DISTANCE: usize = 100 * 1024 * 1024;

    let mut codes = Vec::new();
    let mut i = 0;
    let mut last_code_pos = 0;

    while i + 3 < data.len() {
        // Limit scan distance to prevent DoS
        if i > last_code_pos && i - last_code_pos > MAX_START_CODE_SCAN_DISTANCE {
            break; // Give up after scanning 100MB without finding start code
        }

        // Look for start code prefix: 0x000001
        if data[i] == 0x00 && data[i + 1] == 0x00 && data[i + 2] == 0x01 {
            let code_value = data[i + 3];
            codes.push((
                i,
                StartCode {
                    code_type: StartCodeType::from_u8(code_value),
                    code_value,
                },
            ));
            last_code_pos = i;
            i += 4;
        } else {
            i += 1;
        }
    }

    codes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_start_code_types() {
        assert_eq!(StartCodeType::from_u8(0x00), StartCodeType::Picture);
        assert_eq!(StartCodeType::from_u8(0x01), StartCodeType::Slice(1));
        assert_eq!(StartCodeType::from_u8(0xB3), StartCodeType::SequenceHeader);
        assert_eq!(StartCodeType::from_u8(0xB8), StartCodeType::GroupOfPictures);
    }

    #[test]
    fn test_find_start_codes() {
        let data = [
            0x00, 0x00, 0x01, 0xB3, 0x00, 0x00, 0x01, 0xB8, 0x00, 0x00, 0x01, 0x00,
        ];
        let codes = find_start_codes(&data);

        assert_eq!(codes.len(), 3);
        assert_eq!(codes[0].0, 0);
        assert_eq!(codes[0].1.code_type, StartCodeType::SequenceHeader);
        assert_eq!(codes[1].0, 4);
        assert_eq!(codes[1].1.code_type, StartCodeType::GroupOfPictures);
        assert_eq!(codes[2].0, 8);
        assert_eq!(codes[2].1.code_type, StartCodeType::Picture);
    }
}
