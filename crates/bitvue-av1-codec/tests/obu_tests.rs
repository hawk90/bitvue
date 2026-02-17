#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(unused_comparisons)]
#![allow(unused_doc_comments)]
//! Unit tests for OBU parsing
//!
//! TDD: Test cases for AV1 OBU header and payload parsing

use bitvue_av1_codec::{parse_all_obus, parse_obu, ObuIterator, ObuType};

mod obu_type {
    use super::*;

    #[test]
    fn all_valid_types() {
        let types = [
            (0, ObuType::Reserved0),
            (1, ObuType::SequenceHeader),
            (2, ObuType::TemporalDelimiter),
            (3, ObuType::FrameHeader),
            (4, ObuType::TileGroup),
            (5, ObuType::Metadata),
            (6, ObuType::Frame),
            (7, ObuType::RedundantFrameHeader),
            (8, ObuType::TileList),
            (15, ObuType::Padding),
        ];

        for (value, expected) in types {
            assert_eq!(ObuType::from_u8(value).unwrap(), expected);
        }
    }

    #[test]
    fn invalid_type_errors() {
        assert!(ObuType::from_u8(16).is_err());
        assert!(ObuType::from_u8(255).is_err());
    }

    #[test]
    fn type_names() {
        assert_eq!(ObuType::SequenceHeader.name(), "SEQUENCE_HEADER");
        assert_eq!(ObuType::Frame.name(), "FRAME");
        assert_eq!(ObuType::Padding.name(), "PADDING");
    }

    #[test]
    fn has_frame_data() {
        assert!(ObuType::Frame.has_frame_data());
        assert!(ObuType::FrameHeader.has_frame_data());
        assert!(ObuType::TileGroup.has_frame_data());
        assert!(!ObuType::SequenceHeader.has_frame_data());
        assert!(!ObuType::Padding.has_frame_data());
    }
}

mod parse_header {
    use super::*;

    #[test]
    fn temporal_delimiter_no_extension() {
        // type=2, no extension, has size: 0 0010 0 1 0 = 0x12
        let data = [0x12, 0x00];
        let (obu, len) = parse_obu(&data, 0).unwrap();

        assert_eq!(obu.header.obu_type, ObuType::TemporalDelimiter);
        assert!(!obu.header.has_extension);
        assert!(obu.header.has_size);
        assert_eq!(obu.header.temporal_id, 0);
        assert_eq!(obu.header.spatial_id, 0);
        assert_eq!(len, 2);
    }

    #[test]
    fn sequence_header_no_extension() {
        // type=1, no extension, has size: 0 0001 0 1 0 = 0x0A
        let data = [0x0A, 0x00];
        let (obu, _) = parse_obu(&data, 0).unwrap();

        assert_eq!(obu.header.obu_type, ObuType::SequenceHeader);
        assert!(!obu.header.has_extension);
    }

    #[test]
    fn frame_with_extension() {
        // type=6, has extension, has size: 0 0110 1 1 0 = 0x36
        // Extension: temporal=2, spatial=1, reserved=0: 010 01 000 = 0x48
        let data = [0x36, 0x48, 0x00];
        let (obu, _) = parse_obu(&data, 0).unwrap();

        assert_eq!(obu.header.obu_type, ObuType::Frame);
        assert!(obu.header.has_extension);
        assert_eq!(obu.header.temporal_id, 2);
        assert_eq!(obu.header.spatial_id, 1);
        assert_eq!(obu.header.header_size, 2);
    }

    #[test]
    fn forbidden_bit_set_errors() {
        // Forbidden bit = 1: 1 0010 0 1 0 = 0x92
        let data = [0x92, 0x00];
        assert!(parse_obu(&data, 0).is_err());
    }
}

mod parse_size {
    use super::*;

    #[test]
    fn zero_size_payload() {
        // type=2, has size, size=0
        let data = [0x12, 0x00];
        let (obu, len) = parse_obu(&data, 0).unwrap();

        assert_eq!(obu.payload_size, 0);
        assert_eq!(obu.total_size, 2);
        assert_eq!(len, 2);
    }

    #[test]
    fn small_payload() {
        // type=15 (padding), has size, size=5
        // Header: 0 1111 0 1 0 = 0x7A
        let data = [0x7A, 0x05, 0x00, 0x00, 0x00, 0x00, 0x00];
        let (obu, len) = parse_obu(&data, 0).unwrap();

        assert_eq!(obu.payload_size, 5);
        assert_eq!(obu.total_size, 7);
        assert_eq!(obu.payload.len(), 5);
        assert_eq!(len, 7);
    }

    #[test]
    fn leb128_multi_byte_size() {
        // type=15, has size, size=128 (requires 2 LEB128 bytes)
        // Header: 0x7A
        // Size: 0x80 0x01 (LEB128 for 128)
        let mut data = vec![0x7A, 0x80, 0x01];
        data.extend(vec![0x00; 128]); // 128 bytes of padding

        let (obu, len) = parse_obu(&data, 0).unwrap();

        assert_eq!(obu.payload_size, 128);
        assert_eq!(obu.total_size, 131); // 1 header + 2 size + 128 payload
        assert_eq!(len, 131);
    }
}

mod parse_payload {
    use super::*;

    #[test]
    fn payload_content_preserved() {
        // Padding OBU with specific payload
        let data = [0x7A, 0x04, 0xDE, 0xAD, 0xBE, 0xEF];
        let (obu, _) = parse_obu(&data, 0).unwrap();

        assert_eq!(&*obu.payload, &[0xDE, 0xAD, 0xBE, 0xEF][..]);
    }

    #[test]
    fn offset_preserved() {
        // Parse at offset 10
        let mut data = vec![0x00; 10];
        data.extend([0x12, 0x00]); // Temporal delimiter

        let (obu, _) = parse_obu(&data, 10).unwrap();
        assert_eq!(obu.offset, 10);
    }
}

mod parse_multiple {
    use super::*;

    #[test]
    fn two_obus() {
        let data = [
            0x12, 0x00, // Temporal delimiter
            0x7A, 0x02, 0x00, 0x00, // Padding with 2 bytes
        ];

        let obus = parse_all_obus(&data).unwrap();

        assert_eq!(obus.len(), 2);
        assert_eq!(obus[0].header.obu_type, ObuType::TemporalDelimiter);
        assert_eq!(obus[0].offset, 0);
        assert_eq!(obus[1].header.obu_type, ObuType::Padding);
        assert_eq!(obus[1].offset, 2);
    }

    #[test]
    fn multiple_obus_various_types() {
        // type=3 (FrameHeader): 0 0011 0 1 0 = 0x1A
        let data = [
            0x12, 0x00, // Temporal delimiter (type=2)
            0x0A, 0x00, // Sequence header (type=1)
            0x1A, 0x00, // Frame header (type=3)
            0x7A, 0x01, 0xFF, // Padding (type=15)
        ];

        let obus = parse_all_obus(&data).unwrap();

        assert_eq!(obus.len(), 4);
        assert_eq!(obus[0].header.obu_type, ObuType::TemporalDelimiter);
        assert_eq!(obus[1].header.obu_type, ObuType::SequenceHeader);
        assert_eq!(obus[2].header.obu_type, ObuType::FrameHeader);
        assert_eq!(obus[3].header.obu_type, ObuType::Padding);
    }

    #[test]
    fn empty_data() {
        let obus = parse_all_obus(&[]).unwrap();
        assert!(obus.is_empty());
    }
}

mod iterator {
    use super::*;

    #[test]
    fn iterator_yields_all_obus() {
        let data = [
            0x12, 0x00, // Temporal delimiter
            0x7A, 0x01, 0x00, // Padding 1 byte
        ];

        let obus: Vec<_> = ObuIterator::new(&data).collect::<Result<_, _>>().unwrap();

        assert_eq!(obus.len(), 2);
    }

    #[test]
    fn iterator_empty_data() {
        let obus: Vec<_> = ObuIterator::new(&[]).collect::<Result<_, _>>().unwrap();
        assert!(obus.is_empty());
    }

    #[test]
    fn iterator_stops_on_error() {
        // Valid OBU followed by invalid data
        let data = [
            0x12, 0x00, // Valid temporal delimiter
            0x92, 0x00, // Invalid (forbidden bit set)
        ];

        let mut iter = ObuIterator::new(&data);
        assert!(iter.next().unwrap().is_ok()); // First succeeds
        assert!(iter.next().unwrap().is_err()); // Second fails
    }
}

mod edge_cases {
    use super::*;

    #[test]
    fn truncated_header_errors() {
        // Only half a header
        let data = [0x12]; // No size field
        assert!(parse_obu(&data, 0).is_err());
    }

    #[test]
    fn truncated_payload_errors() {
        // Header says 10 bytes but only 5 available
        let data = [0x7A, 0x0A, 0x00, 0x00, 0x00, 0x00, 0x00];
        assert!(parse_obu(&data, 0).is_err());
    }

    #[test]
    fn offset_past_end_errors() {
        let data = [0x12, 0x00];
        assert!(parse_obu(&data, 10).is_err());
    }
}
