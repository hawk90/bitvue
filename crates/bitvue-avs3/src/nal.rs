//! AVS3 NAL unit / start-code scanning.
//!
//! AVS3 uses 3-byte start codes: 0x00 0x00 0x01 followed by a 1-byte
//! start_code_identifier (SCI).
//!
//! Key SCI values (AVS3 P3 — GB/T 33475.3):
//!   0xB0  Sequence Header
//!   0xB1  Sequence End
//!   0xB5  Extension (user-data, picture timing, etc.)
//!   0xB3  I-frame / Intra picture
//!   0xB6  P/B-frame / Inter picture
//!   0x00–0xAF  Slice start codes (slice_vertical_position)

/// AVS3 start-code identifier constants.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sci {
    Slice(u8),      // 0x00..=0xAF
    SequenceHeader, // 0xB0
    SequenceEnd,    // 0xB1
    UserData,       // 0xB2
    IFrame,         // 0xB3
    Extension,      // 0xB5
    PBFrame,        // 0xB6
    Unknown(u8),
}

impl From<u8> for Sci {
    fn from(v: u8) -> Self {
        match v {
            0x00..=0xAF => Sci::Slice(v),
            0xB0 => Sci::SequenceHeader,
            0xB1 => Sci::SequenceEnd,
            0xB2 => Sci::UserData,
            0xB3 => Sci::IFrame,
            0xB5 => Sci::Extension,
            0xB6 => Sci::PBFrame,
            other => Sci::Unknown(other),
        }
    }
}

/// A raw NAL-like unit extracted from a byte stream.
#[derive(Debug, Clone)]
pub struct NalUnit<'a> {
    /// Start-code identifier.
    pub sci: Sci,
    /// Raw bytes of this unit (not including the 3-byte start code prefix).
    pub data: &'a [u8],
    /// Byte offset of the first byte of the start code (0x00 0x00 0x01).
    pub offset: usize,
}

/// Scan `data` and return all NAL units found.
///
/// Each unit's `data` slice spans from the SCI byte (inclusive) up to, but
/// not including, the next start code (or end of buffer).
pub fn scan_nal_units(data: &[u8]) -> Vec<NalUnit<'_>> {
    let mut units = Vec::new();
    let mut i = 0usize;

    while i + 3 < data.len() {
        if data[i] == 0x00 && data[i + 1] == 0x00 && data[i + 2] == 0x01 {
            let sci = Sci::from(data[i + 3]);
            // The data starts at SCI byte
            let start = i + 3;
            // Find the next start code
            let end = find_next_start_code(data, i + 4).unwrap_or(data.len());
            units.push(NalUnit {
                sci,
                data: &data[start..end],
                offset: i,
            });
            i = end;
        } else {
            i += 1;
        }
    }

    units
}

/// Find the next 0x00 0x00 0x01 pattern starting at `from`.
fn find_next_start_code(data: &[u8], from: usize) -> Option<usize> {
    if from + 2 >= data.len() {
        return None;
    }
    for i in from..data.len() - 2 {
        if data[i] == 0x00 && data[i + 1] == 0x00 && data[i + 2] == 0x01 {
            return Some(i);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_two_units() {
        // Two units: seq header then I-frame
        let data = [
            0x00, 0x00, 0x01, 0xB0, 0xAA, 0xBB, // seq header, 2 bytes
            0x00, 0x00, 0x01, 0xB3, 0xCC, 0xDD, 0xEE, // I-frame, 3 bytes
        ];
        let units = scan_nal_units(&data);
        assert_eq!(units.len(), 2);
        assert_eq!(units[0].sci, Sci::SequenceHeader);
        assert_eq!(units[0].data, &[0xB0, 0xAA, 0xBB]);
        assert_eq!(units[1].sci, Sci::IFrame);
        assert_eq!(units[1].data, &[0xB3, 0xCC, 0xDD, 0xEE]);
    }
}
