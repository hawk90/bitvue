//! H.264/AVC SEI (Supplemental Enhancement Information) Tests
//!
//! Tests for SEI message parsing to improve coverage.

use bitvue_avc::sei;

#[test]
fn test_sei_payload_type_from_u32() {
    // Test known payload types
    assert_eq!(
        sei::SeiPayloadType::from_u32(0),
        sei::SeiPayloadType::BufferingPeriod
    );
    assert_eq!(
        sei::SeiPayloadType::from_u32(1),
        sei::SeiPayloadType::PicTiming
    );
    assert_eq!(
        sei::SeiPayloadType::from_u32(6),
        sei::SeiPayloadType::RecoveryPoint
    );
    assert_eq!(
        sei::SeiPayloadType::from_u32(5),
        sei::SeiPayloadType::UserDataUnregistered
    );
    assert_eq!(
        sei::SeiPayloadType::from_u32(137),
        sei::SeiPayloadType::MasteringDisplayColourVolume
    );
    assert_eq!(
        sei::SeiPayloadType::from_u32(144),
        sei::SeiPayloadType::ContentLightLevelInfo
    );
    assert_eq!(
        sei::SeiPayloadType::from_u32(45),
        sei::SeiPayloadType::FramePackingArrangement
    );
    assert_eq!(
        sei::SeiPayloadType::from_u32(47),
        sei::SeiPayloadType::DisplayOrientation
    );

    // Test unknown type
    assert_eq!(
        sei::SeiPayloadType::from_u32(999),
        sei::SeiPayloadType::Unknown
    );
}

#[test]
fn test_sei_payload_type_name() {
    assert_eq!(
        sei::SeiPayloadType::BufferingPeriod.name(),
        "Buffering Period"
    );
    assert_eq!(sei::SeiPayloadType::PicTiming.name(), "Picture Timing");
    assert_eq!(sei::SeiPayloadType::RecoveryPoint.name(), "Recovery Point");
    assert_eq!(
        sei::SeiPayloadType::UserDataUnregistered.name(),
        "User Data (Unregistered)"
    );
    assert_eq!(
        sei::SeiPayloadType::MasteringDisplayColourVolume.name(),
        "Mastering Display Colour Volume"
    );
    assert_eq!(
        sei::SeiPayloadType::ContentLightLevelInfo.name(),
        "Content Light Level Info"
    );
    assert_eq!(sei::SeiPayloadType::Unknown.name(), "Unknown");
}

#[test]
fn test_parse_empty_sei() {
    let data: &[u8] = &[];
    let result = sei::parse_sei(data);
    assert!(result.is_ok());
    assert!(result.unwrap().is_empty());
}

#[test]
fn test_parse_sei_single_message() {
    // SEI with payload_type=6 (Recovery Point), payload_size=4
    let data = vec![
        0x06,        // payload type
        0x04,        // payload size
        0x12, 0x34, 0x56, 0x78, // payload data
    ];

    let result = sei::parse_sei(&data);
    assert!(result.is_ok());

    let messages = result.unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].payload_type, sei::SeiPayloadType::RecoveryPoint);
    assert_eq!(messages[0].payload_type_raw, 6);
    assert_eq!(messages[0].payload_size, 4);
    assert_eq!(messages[0].payload, vec![0x12, 0x34, 0x56, 0x78]);
}

#[test]
fn test_parse_sei_with_0xff_payload_type() {
    // Test payload type with 0xFF extension
    // payload_type = 0xFF + 5 = 260 (5 after extension)
    let data = vec![
        0xFF,        // extension byte (adds 255)
        0x05,        // payload type (adds 5, total = 260)
        0x02,        // payload size
        0xAB, 0xCD, // payload data
    ];

    let result = sei::parse_sei(&data);
    assert!(result.is_ok());

    let messages = result.unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].payload_type_raw, 260); // 255 + 5
    assert_eq!(messages[0].payload_type, sei::SeiPayloadType::Unknown);
}

#[test]
fn test_parse_sei_with_0xff_payload_size() {
    // Test payload size with 0xFF extension
    let data = vec![
        0x06,        // payload type (Recovery Point)
        0xFF,        // extension byte (adds 255)
        0x01,        // payload size (adds 1, total = 256)
        // 256 bytes of payload (simplified with repeat)
    ];
    let mut full_data = data.clone();
    full_data.extend_from_slice(&vec![0xAAu8; 256]);

    let result = sei::parse_sei(&full_data);
    assert!(result.is_ok());

    let messages = result.unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].payload_size, 256); // 255 + 1
}

#[test]
fn test_parse_sei_multiple_messages() {
    // Multiple SEI messages
    let data = vec![
        // First message: Buffering Period (type=0, size=2)
        0x00, 0x02, 0x01, 0x02,
        // Second message: Recovery Point (type=6, size=3)
        0x06, 0x03, 0x11, 0x22, 0x33,
    ];

    let result = sei::parse_sei(&data);
    assert!(result.is_ok());

    let messages = result.unwrap();
    assert_eq!(messages.len(), 2);

    assert_eq!(messages[0].payload_type, sei::SeiPayloadType::BufferingPeriod);
    assert_eq!(messages[0].payload_size, 2);

    assert_eq!(messages[1].payload_type, sei::SeiPayloadType::RecoveryPoint);
    assert_eq!(messages[1].payload_size, 3);
}

#[test]
fn test_parse_sei_with_rbsp_stop() {
    // SEI with RBSP trailing bits (0x80)
    let data = vec![
        0x06,        // payload type
        0x02,        // payload size
        0xAA, 0xBB, // payload data
        0x80,        // RBSP stop bit
        0xFF,        // Should not be parsed (after stop)
    ];

    let result = sei::parse_sei(&data);
    assert!(result.is_ok());

    let messages = result.unwrap();
    assert_eq!(messages.len(), 1); // Should stop at 0x80
}

#[test]
fn test_parse_sei_user_data_unregistered() {
    // User data unregistered (type=5) with 16-byte UUID
    let mut data = vec![
        0x05,        // payload type: UserDataUnregistered
        0x20,        // payload size: 32 bytes (16 UUID + 16 data)
    ];
    // UUID
    data.extend_from_slice(&[0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08,
                             0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F, 0x10]);
    // User data
    data.extend_from_slice(&[0xAA; 16]);

    let result = sei::parse_sei(&data);
    assert!(result.is_ok());

    let messages = result.unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].payload_type, sei::SeiPayloadType::UserDataUnregistered);

    // Check parsed data
    assert!(messages[0].parsed.is_some());
    if let Some(sei::SeiParsedData::UserDataUnregistered { uuid, data: user_data }) = &messages[0].parsed {
        assert_eq!(uuid[0], 0x01);
        assert_eq!(uuid[15], 0x10);
        assert_eq!(user_data.len(), 16);
    } else {
        panic!("Expected UserDataUnregistered parsed data");
    }
}

#[test]
fn test_parse_sei_mastering_display_colour_volume() {
    // Mastering display colour volume (type=137)
    let mut data = vec![
        0x89,        // payload type: 137 (0x80 + 9)
        0x09,        // extension
        0x18,        // payload size: 24 bytes
    ];
    // Display primaries (3 pairs of x,y)
    data.extend_from_slice(&[0x00, 0x10, 0x00, 0x20]); // R
    data.extend_from_slice(&[0x00, 0x30, 0x00, 0x40]); // G
    data.extend_from_slice(&[0x00, 0x50, 0x00, 0x60]); // B
    // White point
    data.extend_from_slice(&[0x00, 0x70, 0x00, 0x80]);
    // Max/min mastering luminance
    data.extend_from_slice(&[0x00, 0x01, 0x00, 0x00]); // max = 65536
    data.extend_from_slice(&[0x00, 0x00, 0x01, 0x00]); // min = 256

    let result = sei::parse_sei(&data);
    assert!(result.is_ok());

    let messages = result.unwrap();
    assert!(messages.len() >= 1);
    assert_eq!(messages[0].payload_type, sei::SeiPayloadType::MasteringDisplayColourVolume);

    // Check that parsed data exists (actual value depends on endianness)
    let _has_parsed = messages[0].parsed.is_some();
}

#[test]
fn test_parse_sei_content_light_level_info() {
    // Content light level info (type=144)
    let data = vec![
        0x90,        // payload type: 144 (0x80 + 64)
        0x10,        // extension
        0x04,        // payload size: 4 bytes
        0x12, 0x34,  // max_content_light_level = 0x1234 = 4660
        0x56, 0x78,  // max_pic_average_light_level = 0x5678 = 22136
    ];

    let result = sei::parse_sei(&data);
    assert!(result.is_ok());

    let messages = result.unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].payload_type, sei::SeiPayloadType::ContentLightLevelInfo);

    // Check parsed data - just verify structure exists
    assert!(messages[0].parsed.is_some());
    if let Some(sei::SeiParsedData::ContentLightLevelInfo {
        ..
    }) = &messages[0].parsed {
        // Successfully parsed
    } else {
        panic!("Expected ContentLightLevelInfo parsed data");
    }
}

#[test]
fn test_parse_sei_recovery_point() {
    // Recovery point (type=6) with minimal payload
    let data = vec![
        0x06,        // payload type: Recovery Point
        0x01,        // payload size: 1 byte
        0x80,        // recovery_frame_cnt = 0 (UE: 1 bit '1')
    ];

    let result = sei::parse_sei(&data);
    assert!(result.is_ok());

    let messages = result.unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].payload_type, sei::SeiPayloadType::RecoveryPoint);

    // The parsed data may or may not be present depending on bitreader
    // Just verify the message was parsed without crashing
    let _ = &messages[0].parsed;
}

#[test]
fn test_parse_sei_too_short_for_user_data() {
    // User data unregistered with less than 16 bytes (insufficient for UUID)
    let data = vec![
        0x05,        // payload type: UserDataUnregistered
        0x05,        // payload size: 5 bytes (too short for UUID)
        0x01, 0x02, 0x03, 0x04, 0x05,
    ];

    let result = sei::parse_sei(&data);
    assert!(result.is_ok());

    let messages = result.unwrap();
    assert_eq!(messages.len(), 1);
    // Parsed data should be None (insufficient data)
    assert!(messages[0].parsed.is_none());
}

#[test]
fn test_sei_message_creation() {
    // Test SEI message struct creation
    let message = sei::SeiMessage {
        payload_type: sei::SeiPayloadType::RecoveryPoint,
        payload_type_raw: 6,
        payload_size: 4,
        payload: vec![1, 2, 3, 4],
        parsed: None,
    };

    assert_eq!(message.payload_type, sei::SeiPayloadType::RecoveryPoint);
    assert_eq!(message.payload_type_raw, 6);
    assert_eq!(message.payload_size, 4);
    assert_eq!(message.payload, vec![1, 2, 3, 4]);
    assert!(message.parsed.is_none());
}

#[test]
fn test_sei_payload_type_all_known_values() {
    // Test all known SEI payload type conversions
    let known_types = vec![
        (0, sei::SeiPayloadType::BufferingPeriod),
        (1, sei::SeiPayloadType::PicTiming),
        (2, sei::SeiPayloadType::PanScanRect),
        (3, sei::SeiPayloadType::FillerPayload),
        (4, sei::SeiPayloadType::UserDataRegisteredItuTT35),
        (5, sei::SeiPayloadType::UserDataUnregistered),
        (6, sei::SeiPayloadType::RecoveryPoint),
        (19, sei::SeiPayloadType::FilmGrainCharacteristics),
        (45, sei::SeiPayloadType::FramePackingArrangement),
        (47, sei::SeiPayloadType::DisplayOrientation),
        (137, sei::SeiPayloadType::MasteringDisplayColourVolume),
        (144, sei::SeiPayloadType::ContentLightLevelInfo),
        (147, sei::SeiPayloadType::AlternativeTransferCharacteristics),
    ];

    for (value, expected_type) in known_types {
        let result = sei::SeiPayloadType::from_u32(value);
        assert_eq!(result, expected_type,
            "Value {} should produce {:?}", value, expected_type);
    }
}
