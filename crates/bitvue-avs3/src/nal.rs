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
/// Each unit's `data` slice spans from the SCI byte (inclusive) up to, but not including, the
/// zero bytes in front of the next start code (the optional `zero_byte` of a 4-byte start code and
/// any trailing zero bytes), or the end of the buffer. `offset` is the position of the
/// `00 00 01` of the unit's own start code. A slice unit that is nothing but the SCI `0x00` and
/// zero bytes holds no data and is not reported.
pub fn scan_nal_units(data: &[u8]) -> Vec<NalUnit<'_>> {
    bitvue_engine::annex_b::find_nal_ranges(data)
        .into_iter()
        .map(|unit| NalUnit {
            sci: Sci::from(data[unit.start]),
            data: &data[unit.start..unit.end],
            offset: unit.start - 3,
        })
        .collect()
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

    #[test]
    fn zero_bytes_in_front_of_the_next_start_code_are_not_part_of_the_unit() {
        // 4-byte start code (zero_byte) and trailing zero bytes before the next unit
        let data = [
            0x00, 0x00, 0x01, 0xB0, 0xAA, 0x00, 0x00, // seq header + 2 trailing zeros
            0x00, 0x00, 0x00, 0x01, 0xB3, 0xCC, 0x00, // zero_byte + 3-byte code, I-frame
        ];
        let units = scan_nal_units(&data);
        assert_eq!(units.len(), 2);
        assert_eq!(units[0].data, &[0xB0, 0xAA]);
        assert_eq!(units[0].offset, 0);
        assert_eq!(units[1].data, &[0xB3, 0xCC]);
        assert_eq!(
            units[1].offset, 8,
            "offset stays on the 00 00 01 of the start code"
        );
    }
}
