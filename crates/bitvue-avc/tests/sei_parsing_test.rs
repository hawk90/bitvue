#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(unused_comparisons)]
#![allow(unused_doc_comments)]
//! AVC SEI Parsing Tests
//!
//! Tests for SEI (Supplemental Enhancement Information) parsing with real data.

use bitvue_avc::sei::{parse_sei, SeiMessage, SeiParsedData, SeiPayloadType};
use bitvue_avc::AvcError;

/// Create SEI payload byte encoding
fn encode_sei_payload_type(payload_type: u32) -> Vec<u8> {
    let mut data = Vec::new();

    // Encode payload type using 0xFF extension mechanism
    let mut remaining = payload_type;
    while remaining >= 255 {
        data.push(0xFF);
        remaining -= 255;
    }
    data.push(remaining as u8);

    data
}

/// Create SEI payload size byte encoding
fn encode_sei_payload_size(payload_size: u32) -> Vec<u8> {
    let mut data = Vec::new();

    // Encode payload size using 0xFF extension mechanism
    let mut remaining = payload_size;
    while remaining >= 255 {
        data.push(0xFF);
        remaining -= 255;
    }
    data.push(remaining as u8);

    data
}

#[test]
fn test_parse_sei_empty() {
    let data: &[u8] = &[];
    let result = parse_sei(data);

    assert!(result.is_ok());
    let messages = result.unwrap();
    assert_eq!(messages.len(), 0);
}

#[test]
fn test_parse_sei_single_message() {
    let mut data = Vec::new();

    // Payload type = 6 (RecoveryPoint)
    data.extend_from_slice(&encode_sei_payload_type(6));

    // Payload size = 5
    data.extend_from_slice(&encode_sei_payload_size(5));

    // Payload data (5 bytes of recovery point data)
    data.extend_from_slice(&[0x80, 0x00, 0x00, 0x00, 0x00]);

    let result = parse_sei(&data);
    assert!(result.is_ok());

    let messages = result.unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].payload_type, SeiPayloadType::RecoveryPoint);
    assert_eq!(messages[0].payload_size, 5);
}

#[test]
fn test_parse_sei_payload_type_small() {
    let payload_types = vec![0u32, 1, 2, 5, 10, 100];

    for pt in payload_types {
        let data = encode_sei_payload_type(pt);
        assert!(!data.is_empty());
        // First byte should be the value (no extension needed for < 255)
        if pt < 255 {
            assert_eq!(data[0] as u32, pt);
            assert_eq!(data.len(), 1);
        }
    }
}

#[test]
fn test_parse_sei_payload_type_large() {
    // Test payload types >= 255
    let payload_types = vec![255u32, 256, 510, 1000];

    for pt in payload_types {
        let data = encode_sei_payload_type(pt);
        assert!(!data.is_empty());

        // Should have at least one 0xFF for values >= 255
        if pt >= 255 {
            let ff_count = pt / 255;
            let remainder = pt % 255;
            assert_eq!(
                data.iter().filter(|&&b| b == 0xFF).count(),
                ff_count as usize
            );
            assert_eq!(*data.last().unwrap() as u32, remainder);
        }
    }
}

#[test]
fn test_parse_sei_payload_size_encoding() {
    let payload_sizes = vec![0u32, 1, 100, 254, 255, 256, 511, 1000];

    for ps in payload_sizes {
        let data = encode_sei_payload_size(ps);
        assert!(!data.is_empty());

        if ps >= 255 {
            let ff_count = ps / 255;
            let remainder = ps % 255;
            assert_eq!(
                data.iter().filter(|&&b| b == 0xFF).count(),
                ff_count as usize
            );
            assert_eq!(*data.last().unwrap() as u32, remainder);
        } else {
            assert_eq!(data[0] as u32, ps);
            assert_eq!(data.len(), 1);
        }
    }
}

#[test]
fn test_parse_sei_multiple_messages() {
    let mut data = Vec::new();

    // First message: Recovery point
    data.extend_from_slice(&encode_sei_payload_type(6));
    data.extend_from_slice(&encode_sei_payload_size(5));
    data.extend_from_slice(&[0x80, 0x00, 0x00, 0x00, 0x00]);

    // Second message: User data unregistered
    data.extend_from_slice(&encode_sei_payload_type(5));
    data.extend_from_slice(&encode_sei_payload_size(20));
    // UUID (16 bytes) + 4 bytes data
    data.extend_from_slice(&[
        0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F,
        0x10, 0x11, 0x12, 0x13, 0x14,
    ]);

    let result = parse_sei(&data);
    assert!(result.is_ok());

    let messages = result.unwrap();
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0].payload_type, SeiPayloadType::RecoveryPoint);
    assert_eq!(
        messages[1].payload_type,
        SeiPayloadType::UserDataUnregistered
    );
}

#[test]
fn test_parse_sei_with_rbsp_stop() {
    let mut data = Vec::new();

    // Message: Recovery point
    data.extend_from_slice(&encode_sei_payload_type(6));
    data.extend_from_slice(&encode_sei_payload_size(2));
    data.extend_from_slice(&[0x80, 0x00]);

    // RBSP trailing bits (0x80)
    data.push(0x80);

    let result = parse_sei(&data);
    assert!(result.is_ok());

    let messages = result.unwrap();
    // Should stop at RBSP trailing bit
    assert!(!messages.is_empty());
}

#[test]
fn test_sei_payload_type_from_u32() {
    let test_cases = vec![
        (0, SeiPayloadType::BufferingPeriod),
        (1, SeiPayloadType::PicTiming),
        (2, SeiPayloadType::PanScanRect),
        (3, SeiPayloadType::FillerPayload),
        (4, SeiPayloadType::UserDataRegisteredItuTT35),
        (5, SeiPayloadType::UserDataUnregistered),
        (6, SeiPayloadType::RecoveryPoint),
        (19, SeiPayloadType::FilmGrainCharacteristics),
        (45, SeiPayloadType::FramePackingArrangement),
        (137, SeiPayloadType::MasteringDisplayColourVolume),
        (144, SeiPayloadType::ContentLightLevelInfo),
        (147, SeiPayloadType::AlternativeTransferCharacteristics),
        (999, SeiPayloadType::Unknown),
    ];

    for (value, expected) in test_cases {
        let result = SeiPayloadType::from_u32(value);
        assert_eq!(result, expected);
    }
}

#[test]
fn test_sei_payload_type_name() {
    assert_eq!(SeiPayloadType::BufferingPeriod.name(), "Buffering Period");
    assert_eq!(SeiPayloadType::PicTiming.name(), "Picture Timing");
    assert_eq!(SeiPayloadType::RecoveryPoint.name(), "Recovery Point");
    assert_eq!(
        SeiPayloadType::UserDataUnregistered.name(),
        "User Data (Unregistered)"
    );
    assert_eq!(SeiPayloadType::Unknown.name(), "Unknown");
}

#[test]
fn test_parse_sei_user_data_unregistered() {
    let mut data = Vec::new();

    // Payload type = 5 (User data unregistered)
    data.extend_from_slice(&encode_sei_payload_type(5));

    // Payload size = 20 (16 bytes UUID + 4 bytes data)
    data.extend_from_slice(&encode_sei_payload_size(20));

    // UUID
    data.extend_from_slice(&[
        0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F,
        0x10,
    ]);

    // User data
    data.extend_from_slice(&[0xAA, 0xBB, 0xCC, 0xDD]);

    let result = parse_sei(&data);
    assert!(result.is_ok());

    let messages = result.unwrap();
    assert_eq!(messages.len(), 1);

    if let Some(SeiParsedData::UserDataUnregistered { uuid, data }) = &messages[0].parsed {
        assert_eq!(uuid[0], 0x01);
        assert_eq!(uuid[15], 0x10);
        assert_eq!(data.len(), 4);
        assert_eq!(data[0], 0xAA);
    } else {
        panic!("Expected UserDataUnregistered parsed data");
    }
}

#[test]
fn test_parse_sei_user_data_unregistered_too_short() {
    let mut data = Vec::new();

    // Payload type = 5 (User data unregistered)
    data.extend_from_slice(&encode_sei_payload_type(5));

    // Payload size = 10 (less than 16 bytes for UUID)
    data.extend_from_slice(&encode_sei_payload_size(10));

    // Incomplete UUID
    data.extend_from_slice(&[0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A]);

    let result = parse_sei(&data);
    assert!(result.is_ok());

    let messages = result.unwrap();
    // Should still parse but without UserDataUnregistered data
    assert_eq!(messages.len(), 1);
    assert!(messages[0].parsed.is_none());
}

#[test]
fn test_parse_sei_mastering_display_colour_volume() {
    let mut data = Vec::new();

    // Payload type = 137 (Mastering display colour volume)
    data.extend_from_slice(&encode_sei_payload_type(137));

    // Payload size = 24 bytes
    data.extend_from_slice(&encode_sei_payload_size(24));

    // Display primaries (RGB - 6 values, 2 bytes each = 12 bytes)
    data.extend_from_slice(&[
        0x00, 0x20, 0x00, 0x30, 0x00, 0x40, // R
        0x00, 0x21, 0x00, 0x31, 0x00, 0x41, // G
        0x00, 0x22, 0x00, 0x32, 0x00, 0x42,
    ]); // B

    // White point (2 bytes each = 4 bytes)
    data.extend_from_slice(&[0x00, 0x25, 0x00, 0x35]);

    // Max/min luminance (4 bytes each = 8 bytes)
    data.extend_from_slice(&[0x00, 0x00, 0x10, 0x00]); // max
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]); // min

    // Add RBSP stop to prevent parsing further data
    data.push(0x80);

    let result = parse_sei(&data);
    assert!(result.is_ok());

    let messages = result.unwrap();
    assert!(messages.len() >= 1);

    // Check the first message has the correct type
    assert_eq!(messages[0].payload_type_raw, 137);
}

#[test]
fn test_parse_sei_content_light_level() {
    let mut data = Vec::new();

    // Payload type = 144 (Content light level info)
    data.extend_from_slice(&encode_sei_payload_type(144));

    // Payload size = 4 bytes
    data.extend_from_slice(&encode_sei_payload_size(4));

    // Max content light level (2 bytes)
    data.extend_from_slice(&[0x10, 0x00]); // 4096

    // Max pic average light level (2 bytes)
    data.extend_from_slice(&[0x05, 0x00]); // 1280

    let result = parse_sei(&data);
    assert!(result.is_ok());

    let messages = result.unwrap();
    assert_eq!(messages.len(), 1);

    if let Some(SeiParsedData::ContentLightLevelInfo {
        max_content_light_level,
        max_pic_average_light_level,
    }) = &messages[0].parsed
    {
        assert_eq!(*max_content_light_level, 4096);
        assert_eq!(*max_pic_average_light_level, 1280);
    } else {
        panic!("Expected ContentLightLevelInfo parsed data");
    }
}

#[test]
fn test_parse_sei_content_light_level_too_short() {
    let mut data = Vec::new();

    // Payload type = 144
    data.extend_from_slice(&encode_sei_payload_type(144));

    // Payload size = 2 bytes (too short)
    data.extend_from_slice(&encode_sei_payload_size(2));

    data.extend_from_slice(&[0x10, 0x00]);

    let result = parse_sei(&data);
    assert!(result.is_ok());

    let messages = result.unwrap();
    assert_eq!(messages.len(), 1);
    // Should not have parsed data due to insufficient length
    assert!(messages[0].parsed.is_none());
}

#[test]
fn test_parse_sei_zero_payload_size() {
    let mut data = Vec::new();

    // Payload type = 6
    data.extend_from_slice(&encode_sei_payload_type(6));

    // Payload size = 0
    data.extend_from_slice(&encode_sei_payload_size(0));

    let result = parse_sei(&data);
    assert!(result.is_ok());

    let messages = result.unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].payload_size, 0);
    assert!(messages[0].payload.is_empty());
}

#[test]
fn test_parse_sei_empty_payload() {
    let mut data = Vec::new();

    // Payload type = 3 (Filler payload)
    data.extend_from_slice(&encode_sei_payload_type(3));

    // Payload size = 0
    data.extend_from_slice(&encode_sei_payload_size(0));

    // RBSP stop
    data.push(0x80);

    let result = parse_sei(&data);
    assert!(result.is_ok());

    let messages = result.unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].payload_type, SeiPayloadType::FillerPayload);
}

#[test]
fn test_sei_message_structure() {
    let msg = SeiMessage {
        payload_type: SeiPayloadType::RecoveryPoint,
        payload_type_raw: 6,
        payload_size: 10,
        payload: vec![0u8; 10],
        parsed: None,
    };

    assert_eq!(msg.payload_type, SeiPayloadType::RecoveryPoint);
    assert_eq!(msg.payload_type_raw, 6);
    assert_eq!(msg.payload_size, 10);
    assert_eq!(msg.payload.len(), 10);
    assert!(msg.parsed.is_none());
}

#[test]
fn test_parse_sei_recovery_point_minimal() {
    let mut data = Vec::new();

    // Payload type = 6
    data.extend_from_slice(&encode_sei_payload_type(6));

    // Minimal recovery point data (1 byte: recovery_frame_cnt = 0)
    data.extend_from_slice(&encode_sei_payload_size(1));
    data.push(0x80); // UE(0) encoded as 0x80

    let result = parse_sei(&data);
    assert!(result.is_ok());

    let messages = result.unwrap();
    assert_eq!(messages.len(), 1);
}

#[test]
fn test_parse_sei_large_payload_type() {
    let mut data = Vec::new();

    // Large payload type = 600 (255 + 255 + 90)
    let mut large_pt = Vec::new();
    large_pt.extend_from_slice(&[0xFFu8, 0xFFu8, 90u8]);
    data.extend_from_slice(&large_pt);

    // Small payload
    data.extend_from_slice(&encode_sei_payload_size(5));
    data.extend_from_slice(&[0u8; 5]);

    let result = parse_sei(&data);
    // SECURITY: Large payloads with many 0xFF bytes should be rejected
    assert!(result.is_err());
    assert!(matches!(result, Err(AvcError::InvalidSei(_))));
}

#[test]
fn test_parse_sei_large_payload_size() {
    let mut data = Vec::new();

    // Payload type = 5
    data.extend_from_slice(&encode_sei_payload_type(5));

    // Large payload size = 600 (255 + 255 + 90)
    let mut large_size = Vec::new();
    large_size.extend_from_slice(&[0xFFu8, 0xFFu8, 90u8]);
    data.extend_from_slice(&large_size);

    // Payload data (90 bytes)
    data.extend_from_slice(&vec![0xAAu8; 90]);

    let result = parse_sei(&data);
    // SECURITY: Large payloads with many 0xFF bytes should be rejected
    assert!(result.is_err());
    assert!(matches!(result, Err(AvcError::InvalidSei(_))));
}

#[test]
fn test_sei_parsed_data_recovery_point() {
    let recovery_data = SeiParsedData::RecoveryPoint {
        recovery_frame_cnt: 10,
        exact_match_flag: true,
        broken_link_flag: false,
        changing_slice_group_idc: 2,
    };

    if let SeiParsedData::RecoveryPoint {
        recovery_frame_cnt,
        exact_match_flag,
        broken_link_flag,
        changing_slice_group_idc,
    } = recovery_data
    {
        assert_eq!(recovery_frame_cnt, 10);
        assert!(exact_match_flag);
        assert!(!broken_link_flag);
        assert_eq!(changing_slice_group_idc, 2);
    } else {
        panic!("Expected RecoveryPoint variant");
    }
}

#[test]
fn test_parse_sei_unknown_payload_type() {
    let mut data = Vec::new();

    // Unknown payload type = 200
    data.extend_from_slice(&encode_sei_payload_type(200));

    // Payload size = 5
    data.extend_from_slice(&encode_sei_payload_size(5));

    data.extend_from_slice(&[0u8; 5]);

    let result = parse_sei(&data);
    assert!(result.is_ok());

    let messages = result.unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].payload_type, SeiPayloadType::Unknown);
    // Unknown types should not have parsed data
    assert!(messages[0].parsed.is_none());
}
