// Metadata Inspector module tests

// ============================================================================
// Fixtures
// ============================================================================
fn create_test_metadata() -> StreamMetadata {
    StreamMetadata::new()
}

fn create_test_mastering_display() -> MasteringDisplayMetadata {
    MasteringDisplayMetadata {
        red_x: 34000,
        red_y: 16000,
        green_x: 13250,
        green_y: 34500,
        blue_x: 7500,
        blue_y: 3000,
        white_point_x: 15635,
        white_point_y: 16350,
        max_luminance: 10000000,
        min_luminance: 500,
    }
}

fn create_test_c_ll() -> ContentLightLevel {
    ContentLightLevel::new(1000, 400)
}

fn create_test_inspector() -> MetadataInspector {
    MetadataInspector::new()
}

// ============================================================================
// MasteringDisplayMetadata Tests
// ============================================================================
#[cfg(test)]
mod mastering_display_tests {
    use super::*;

    #[test]
    fn test_red_normalized() {
        let md = create_test_mastering_display();
        let (x, y) = md.red_normalized();
        assert!((x - 0.68).abs() < 0.01);
        assert!((y - 0.32).abs() < 0.01);
    }

    #[test]
    fn test_max_luminance_cdm2() {
        let md = create_test_mastering_display();
        assert!((md.max_luminance_cdm2() - 1000.0).abs() < 0.1);
    }

    #[test]
    fn test_format_display() {
        let md = create_test_mastering_display();
        let text = md.format_display();
        assert!(text.contains("Mastering Display"));
    }
}

// ============================================================================
// ContentLightLevel Tests
// ============================================================================
#[cfg(test)]
mod content_light_level_tests {
    use super::*;

    #[test]
    fn test_new_creates_cll() {
        let cll = ContentLightLevel::new(1000, 400);
        assert_eq!(cll.max_cll, 1000);
        assert_eq!(cll.max_fall, 400);
    }

    #[test]
    fn test_format_display() {
        let cll = ContentLightLevel::new(1000, 400);
        let text = cll.format_display();
        assert!(text.contains("1000"));
        assert!(text.contains("400"));
    }
}

// ============================================================================
// HdrFormat Tests
// ============================================================================
#[cfg(test)]
mod hdr_format_tests {
    use super::*;

    #[test]
    fn test_name() {
        assert_eq!(HdrFormat::Sdr.name(), "SDR");
        assert_eq!(HdrFormat::Hdr10.name(), "HDR10");
        assert_eq!(HdrFormat::DolbyVision.name(), "Dolby Vision");
    }

    #[test]
    fn test_is_hdr() {
        assert!(!HdrFormat::Sdr.is_hdr());
        assert!(HdrFormat::Hdr10.is_hdr());
        assert!(HdrFormat::DolbyVision.is_hdr());
    }
}

// ============================================================================
// ColorPrimaries Tests
// ============================================================================
#[cfg(test)]
mod color_primaries_tests {
    use super::*;

    #[test]
    fn test_from_code() {
        assert_eq!(ColorPrimaries::from_code(1), ColorPrimaries::Bt709);
        assert_eq!(ColorPrimaries::from_code(9), ColorPrimaries::Bt2020);
    }

    #[test]
    fn test_name() {
        assert_eq!(ColorPrimaries::Bt709.name(), "BT.709");
        assert_eq!(ColorPrimaries::Bt2020.name(), "BT.2020");
    }
}

// ============================================================================
// TransferCharacteristics Tests
// ============================================================================
#[cfg(test)]
mod transfer_tests {
    use super::*;

    #[test]
    fn test_from_code() {
        assert_eq!(TransferCharacteristics::from_code(1), TransferCharacteristics::Bt709);
        assert_eq!(TransferCharacteristics::from_code(16), TransferCharacteristics::Pq);
    }

    #[test]
    fn test_is_hdr() {
        assert!(!TransferCharacteristics::Bt709.is_hdr());
        assert!(TransferCharacteristics::Pq.is_hdr());
        assert!(TransferCharacteristics::Hlg.is_hdr());
    }
}

// ============================================================================
// MatrixCoefficients Tests
// ============================================================================
#[cfg(test)]
mod matrix_tests {
    use super::*;

    #[test]
    fn test_from_code() {
        assert_eq!(MatrixCoefficients::from_code(0), MatrixCoefficients::Identity);
        assert_eq!(MatrixCoefficients::from_code(1), MatrixCoefficients::Bt709);
    }
}

// ============================================================================
// SeiMessageType Tests
// ============================================================================
#[cfg(test)]
mod sei_type_tests {
    use super::*;

    #[test]
    fn test_hdr_related() {
        assert!(SeiMessageType::MasteringDisplayColourVolume.is_hdr_related());
        assert!(SeiMessageType::ContentLightLevelInfo.is_hdr_related());
        assert!(!SeiMessageType::BufferingPeriod.is_hdr_related());
    }
}

// ============================================================================
// StreamMetadata Tests
// ============================================================================
#[cfg(test)]
mod metadata_tests {
    use super::*;

    #[test]
    fn test_new_creates_metadata() {
        let metadata = create_test_metadata();
        assert!(metadata.hdr_format.is_none());
        assert!(metadata.sei_messages.is_empty());
    }

    #[test]
    fn test_has_hdr() {
        let mut metadata = create_test_metadata();
        assert!(!metadata.has_hdr());
        metadata.hdr_format = Some(HdrFormat::Hdr10);
        assert!(metadata.has_hdr());
    }

    #[test]
    fn test_color_info_display() {
        let metadata = create_test_metadata();
        let info = metadata.color_info_display();
        assert!(info.contains("No color info"));
    }

    #[test]
    fn test_sei_count_by_type() {
        let mut metadata = create_test_metadata();
        metadata.sei_messages.push(SeiMessage {
            message_type: SeiMessageType::BufferingPeriod,
            payload_size: 10,
            byte_offset: 100,
            frame_idx: Some(0),
            data: SeiData::Raw(vec![]),
        });
        let counts = metadata.sei_count_by_type();
        assert_eq!(*counts.get(&SeiMessageType::BufferingPeriod).unwrap_or(&0), 1);
    }
}

// ============================================================================
// MetadataInspector Tests
// ============================================================================
#[cfg(test)]
mod inspector_tests {
    use super::*;

    #[test]
    fn test_toggle_hdr() {
        let mut inspector = create_test_inspector();
        inspector.toggle_hdr();
        assert!(!inspector.expanded_hdr);
    }

    #[test]
    fn test_select_sei() {
        let mut inspector = create_test_inspector();
        inspector.select_sei(Some(5));
        assert_eq!(inspector.selected_sei_idx, Some(5));
    }

    #[test]
    fn test_summary_text() {
        let inspector = create_test_inspector();
        let metadata = create_test_metadata();
        let text = inspector.summary_text(&metadata);
        assert!(text.contains("No metadata"));
    }
}

// ============================================================================
// MetadataFilter Tests
// ============================================================================
#[cfg(test)]
mod filter_tests {
    use super::*;

    #[test]
    fn test_all_shows_all() {
        let filter = MetadataFilter::all();
        assert!(filter.show_hdr);
        assert!(filter.show_sei);
        assert!(filter.show_color);
    }

    #[test]
    fn test_hdr_only() {
        let filter = MetadataFilter::hdr_only();
        assert!(filter.show_hdr);
        assert!(!filter.show_sei);
        assert!(!filter.show_color);
    }
}
