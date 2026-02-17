#![allow(dead_code)]
//! Tests for metadata module

use bitvue_core::{
    ColorPrimaries, ContentLightLevel, HdrFormat, MasteringDisplayMetadata, MatrixCoefficients,
    MetadataFilter, MetadataInspector, SeiData, SeiMessage, SeiMessageType, StreamMetadata,
    TransferCharacteristics,
};

#[test]
fn test_mastering_display_metadata() {
    let mdm = MasteringDisplayMetadata {
        red_x: 34000,            // 0.68
        red_y: 16000,            // 0.32
        green_x: 13250,          // 0.265
        green_y: 34500,          // 0.69
        blue_x: 7500,            // 0.15
        blue_y: 3000,            // 0.06
        white_point_x: 15635,    // 0.3127
        white_point_y: 16450,    // 0.329
        max_luminance: 10000000, // 1000 cd/m²
        min_luminance: 50,       // 0.005 cd/m²
    };

    let (rx, ry) = mdm.red_normalized();
    assert!((rx - 0.68).abs() < 0.001);
    assert!((ry - 0.32).abs() < 0.001);

    assert!((mdm.max_luminance_cdm2() - 1000.0).abs() < 0.1);
    assert!((mdm.min_luminance_cdm2() - 0.005).abs() < 0.001);
}

#[test]
fn test_content_light_level() {
    let cll = ContentLightLevel::new(1000, 400);
    assert_eq!(cll.max_cll, 1000);
    assert_eq!(cll.max_fall, 400);

    let display = cll.format_display();
    assert!(display.contains("1000"));
    assert!(display.contains("400"));
}

#[test]
fn test_hdr_format() {
    assert!(!HdrFormat::Sdr.is_hdr());
    assert!(HdrFormat::Hdr10.is_hdr());
    assert!(HdrFormat::Hlg.is_hdr());
    assert!(HdrFormat::DolbyVision.is_hdr());

    assert_eq!(HdrFormat::Hdr10.name(), "HDR10");
    assert_eq!(HdrFormat::Hlg.name(), "HLG");
}

#[test]
fn test_color_primaries() {
    assert_eq!(ColorPrimaries::from_code(1), ColorPrimaries::Bt709);
    assert_eq!(ColorPrimaries::from_code(9), ColorPrimaries::Bt2020);
    assert_eq!(ColorPrimaries::from_code(99), ColorPrimaries::Unknown(99));

    assert_eq!(ColorPrimaries::Bt709.name(), "BT.709");
    assert_eq!(ColorPrimaries::Bt2020.name(), "BT.2020");
}

#[test]
fn test_transfer_characteristics() {
    assert_eq!(
        TransferCharacteristics::from_code(16),
        TransferCharacteristics::Pq
    );
    assert_eq!(
        TransferCharacteristics::from_code(18),
        TransferCharacteristics::Hlg
    );

    assert!(TransferCharacteristics::Pq.is_hdr());
    assert!(TransferCharacteristics::Hlg.is_hdr());
    assert!(!TransferCharacteristics::Bt709.is_hdr());
}

#[test]
fn test_sei_message_type() {
    assert!(SeiMessageType::MasteringDisplayColourVolume.is_hdr_related());
    assert!(SeiMessageType::ContentLightLevelInfo.is_hdr_related());
    assert!(!SeiMessageType::PicTiming.is_hdr_related());
}

#[test]
fn test_stream_metadata() {
    let mut metadata = StreamMetadata::new();
    assert!(!metadata.has_hdr());

    metadata.transfer_characteristics = Some(TransferCharacteristics::Pq);
    assert!(metadata.has_hdr());

    metadata.content_light_level = Some(ContentLightLevel::new(1000, 400));
    assert!(metadata.has_hdr());
}

#[test]
fn test_stream_metadata_color_info() {
    let mut metadata = StreamMetadata::new();
    metadata.color_primaries = Some(ColorPrimaries::Bt2020);
    metadata.transfer_characteristics = Some(TransferCharacteristics::Pq);
    metadata.bit_depth = Some(10);

    let display = metadata.color_info_display();
    assert!(display.contains("BT.2020"));
    assert!(display.contains("PQ"));
    assert!(display.contains("10"));
}

#[test]
fn test_metadata_filter() {
    let filter = MetadataFilter::all();
    assert!(filter.show_hdr);
    assert!(filter.show_sei);
    assert!(filter.show_color);

    let hdr_filter = MetadataFilter::hdr_only();
    assert!(hdr_filter.show_hdr);
    assert!(!hdr_filter.show_sei);
}

#[test]
fn test_metadata_inspector() {
    let mut inspector = MetadataInspector::new();
    assert!(inspector.expanded_hdr);
    assert!(inspector.expanded_sei);

    inspector.toggle_hdr();
    assert!(!inspector.expanded_hdr);

    inspector.select_sei(Some(5));
    assert_eq!(inspector.selected_sei_idx, Some(5));
}

#[test]
fn test_metadata_inspector_filtered_sei() {
    let mut metadata = StreamMetadata::new();
    metadata.sei_messages.push(SeiMessage {
        message_type: SeiMessageType::MasteringDisplayColourVolume,
        payload_size: 24,
        byte_offset: 100,
        frame_idx: Some(0),
        data: SeiData::Raw(vec![]),
    });
    metadata.sei_messages.push(SeiMessage {
        message_type: SeiMessageType::PicTiming,
        payload_size: 8,
        byte_offset: 200,
        frame_idx: Some(1),
        data: SeiData::Raw(vec![]),
    });

    let inspector = MetadataInspector::new();
    let filtered = inspector.filtered_sei(&metadata);
    assert_eq!(filtered.len(), 2);

    // Filter by type
    let mut inspector2 = MetadataInspector::new();
    inspector2.filter.sei_type_filter = Some(SeiMessageType::MasteringDisplayColourVolume);
    let filtered2 = inspector2.filtered_sei(&metadata);
    assert_eq!(filtered2.len(), 1);
}

#[test]
fn test_metadata_inspector_summary() {
    let mut metadata = StreamMetadata::new();
    metadata.hdr_format = Some(HdrFormat::Hdr10);
    metadata.sei_messages.push(SeiMessage {
        message_type: SeiMessageType::PicTiming,
        payload_size: 8,
        byte_offset: 100,
        frame_idx: None,
        data: SeiData::Raw(vec![]),
    });

    let inspector = MetadataInspector::new();
    let summary = inspector.summary_text(&metadata);
    assert!(summary.contains("HDR10"));
    assert!(summary.contains("1 messages"));
}

#[test]
fn test_sei_count_by_type() {
    let mut metadata = StreamMetadata::new();
    for _ in 0..3 {
        metadata.sei_messages.push(SeiMessage {
            message_type: SeiMessageType::PicTiming,
            payload_size: 8,
            byte_offset: 0,
            frame_idx: None,
            data: SeiData::Raw(vec![]),
        });
    }
    metadata.sei_messages.push(SeiMessage {
        message_type: SeiMessageType::MasteringDisplayColourVolume,
        payload_size: 24,
        byte_offset: 0,
        frame_idx: None,
        data: SeiData::Raw(vec![]),
    });

    let counts = metadata.sei_count_by_type();
    assert_eq!(counts.get(&SeiMessageType::PicTiming), Some(&3));
    assert_eq!(
        counts.get(&SeiMessageType::MasteringDisplayColourVolume),
        Some(&1)
    );
}
