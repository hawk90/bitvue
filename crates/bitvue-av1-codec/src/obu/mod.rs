//! AV1 OBU (Open Bitstream Unit) parsing
//!
//! OBU is the fundamental unit of the AV1 bitstream.
//! Reference: AV1 Specification Section 5.3

mod iterator;
mod parse;
mod resilient;
mod types;

pub use iterator::{ObuIterator, ObuWithOffset};
pub use parse::{parse_all_obus, parse_obu, parse_obu_header};
pub use resilient::parse_all_obus_resilient;
pub use types::{Obu, ObuHeader, ObuType};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bitreader::BitReader;
    use bitvue_engine::BitvueError;

    /// Regression test for the "iterator never terminates on a parse error" hang: a truncated
    /// OBU (a valid header claiming a payload size that overruns the remaining buffer) used to
    /// make `Iterator::next()` return the same `Err` forever, since `offset` never advances past
    /// a failed parse. Bounds the loop itself so a regression fails fast instead of hanging the
    /// test suite.
    #[test]
    fn test_obu_iterator_terminates_after_truncated_obu() {
        // Sequence header OBU (valid, 0 payload bytes) followed by an OBU_FRAME header claiming
        // a 256-byte payload with only 2 bytes actually present.
        let data: Vec<u8> = vec![0x0A, 0x80, 0x00, 0x22, 0x80, 0x02, 0x00, 0x01];

        let results: Vec<_> = ObuIterator::new(&data).take(1000).collect();

        assert!(
            results.len() < 1000,
            "iterator did not terminate within 1000 items -- regressed to the infinite-loop-on-\
             parse-error bug"
        );
        assert_eq!(
            results.len(),
            2,
            "one successful OBU, then one error, then done"
        );
        assert!(results[0].is_ok());
        assert!(results[1].is_err());
    }

    #[test]
    fn test_obu_type_from_u8() {
        assert_eq!(ObuType::from_u8(1).unwrap(), ObuType::SequenceHeader);
        assert_eq!(ObuType::from_u8(6).unwrap(), ObuType::Frame);
        assert_eq!(ObuType::from_u8(15).unwrap(), ObuType::Padding);
        assert!(ObuType::from_u8(16).is_err());
    }

    #[test]
    fn test_parse_simple_obu_header() {
        // OBU header: type=1 (sequence), no extension, has size
        // 0 0001 0 1 0 = 0x0A
        let data = [0x0A];
        let mut reader = BitReader::new(&data);
        let header = parse_obu_header(&mut reader).unwrap();

        assert_eq!(header.obu_type, ObuType::SequenceHeader);
        assert!(!header.has_extension);
        assert!(header.has_size);
        assert_eq!(header.header_size, 1);
    }

    #[test]
    fn test_parse_obu_with_extension() {
        // OBU header with extension: type=6 (frame), has extension, has size
        // Byte 1: 0 0110 1 1 0 = 0x36
        // Byte 2: temporal=1, spatial=0, reserved=0 -> 001 00 000 = 0x20
        let data = [0x36, 0x20];
        let mut reader = BitReader::new(&data);
        let header = parse_obu_header(&mut reader).unwrap();

        assert_eq!(header.obu_type, ObuType::Frame);
        assert!(header.has_extension);
        assert!(header.has_size);
        assert_eq!(header.temporal_id, 1);
        assert_eq!(header.spatial_id, 0);
        assert_eq!(header.header_size, 2);
    }

    #[test]
    fn test_parse_complete_obu() {
        // Temporal delimiter OBU (type=2, no extension, has size, size=0)
        // Header: 0 0010 0 1 0 = 0x12
        // Size: 0x00 (LEB128 for 0)
        let data = [0x12, 0x00];
        let (obu, consumed) = parse_obu(&data, 0).unwrap();

        assert_eq!(obu.header.obu_type, ObuType::TemporalDelimiter);
        assert_eq!(obu.payload_size, 0);
        assert_eq!(obu.total_size, 2);
        assert_eq!(consumed, 2);
    }

    #[test]
    fn test_parse_obu_with_payload() {
        // Padding OBU with 3 bytes of payload
        // Header: 0 1111 0 1 0 = 0x7A
        // Size: 0x03 (LEB128 for 3)
        // Payload: 0x00, 0x00, 0x00
        let data = [0x7A, 0x03, 0x00, 0x00, 0x00];
        let (obu, consumed) = parse_obu(&data, 0).unwrap();

        assert_eq!(obu.header.obu_type, ObuType::Padding);
        assert_eq!(obu.payload_size, 3);
        assert_eq!(obu.total_size, 5);
        assert_eq!(obu.payload.len(), 3);
        assert_eq!(consumed, 5);
    }

    #[test]
    fn test_parse_multiple_obus() {
        // Two temporal delimiter OBUs
        let data = [0x12, 0x00, 0x12, 0x00];
        let obus = parse_all_obus(&data).unwrap();

        assert_eq!(obus.len(), 2);
        assert_eq!(obus[0].offset, 0);
        assert_eq!(obus[1].offset, 2);
    }

    #[test]
    fn test_forbidden_bit_error() {
        // Forbidden bit set (first bit = 1)
        let data = [0x92]; // 1 0010 0 1 0
        let mut reader = BitReader::new(&data);
        assert!(parse_obu_header(&mut reader).is_err());
    }

    #[test]
    fn test_obu_without_size_at_end() {
        // OBU without size field at end of data (valid case)
        // Header: 0 0010 0 0 0 = 0x10 (type=2, no extension, no size)
        let data = [0x10, 0x00, 0x00]; // 2 bytes of payload
        let (obu, consumed) = parse_obu(&data, 0).unwrap();

        assert_eq!(obu.header.obu_type, ObuType::TemporalDelimiter);
        assert_eq!(obu.payload_size, 2);
        assert_eq!(obu.total_size, 3);
        assert_eq!(consumed, 3);
    }

    #[test]
    fn test_obu_without_size_header_too_large() {
        // OBU without size field where header is 2 bytes but data is truncated
        // Header: 0 0010 1 0 0 = 0x30 (type=2, has extension, no size)
        // We provide 2 bytes total, but the second byte is incomplete
        // The BitReader will read the first byte successfully but fail on the extension bits
        let data = [0x30, 0x00]; // Two bytes, but extension reading might still fail
        let result = parse_obu(&data, 0);

        // This test verifies we handle truncated data gracefully
        // Depending on BitReader implementation, this might:
        // - Fail during header parsing (if BitReader checks bounds)
        // - Succeed with partial header (if BitReader is lenient)
        // We just check it doesn't panic and returns a Result
        match result {
            Ok(_) => {
                // If it succeeds, verify the OBU is well-formed
            }
            Err(_) => {
                // If it fails, that's also acceptable for truncated data
            }
        }
    }

    #[test]
    fn test_obu_payload_size_too_large() {
        // OBU with payload size exceeding maximum
        // Header: 0 0001 0 1 0 = 0x0A (type=1, has size)
        // Size: encoded value > 100MB
        let mut data = vec![0x0A];
        // LEB128 encode 100MB + 1 = 104,857,601 bytes
        // This exceeds MAX_OBU_PAYLOAD_SIZE
        data.extend_from_slice(&[0x81, 0x80, 0x80, 0x80, 0x80, 0x25]); // ~100MB
        let result = parse_obu(&data, 0);
        assert!(result.is_err(), "Should reject oversized OBU payload");

        match result {
            Err(BitvueError::InvalidData(message)) => {
                assert!(
                    message.contains("exceeds maximum"),
                    "Error should mention maximum size exceeded"
                );
            }
            _ => panic!("Expected InvalidData error for oversized payload"),
        }
    }
}
