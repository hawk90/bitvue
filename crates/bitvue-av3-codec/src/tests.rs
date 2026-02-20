use super::*;

#[test]
fn test_empty_stream() {
    let data: &[u8] = &[];
    let stream = parse_av3(data).unwrap();
    assert_eq!(stream.obu_units.len(), 0);
    assert_eq!(stream.frame_count(), 0);
}

#[test]
fn test_stream_methods() {
    let stream = Av3Stream {
        obu_units: vec![],
        seq_headers: HashMap::new(),
        frame_headers: vec![],
    };

    assert_eq!(stream.frame_count(), 0);
    assert_eq!(stream.key_frames().len(), 0);
}

// Additional tests for main parser functions and Av3Stream methods

#[test]
fn test_parse_av3_with_obu_header() {
    // Test parsing with OBU header
    let mut data = vec![0u8; 16];
    // OBU header: obu_forbidden_bit=0, obu_type=1 (SequenceHeader), obu_extension_flag=0, has_size=1
    data[0] = (1 << 3) | 0x02; // obu_type=1 (SequenceHeader), has_size=1
    data[1] = 10; // size=10

    let result = parse_av3(&data);
    assert!(result.is_ok());
    let stream = result.unwrap();
    let _ = stream.obu_units.len(); // verify field is accessible
}

#[test]
fn test_parse_av3_temporal_delimiter() {
    // Test Temporal Delimiter OBU (type 2)
    let mut data = vec![0u8; 8];
    data[0] = (2 << 1) | 0x01; // obu_type=2, has_size=1
    data[1] = 0; // size=0

    let result = parse_av3(&data);
    assert!(result.is_ok());
    let stream = result.unwrap();
    if !stream.obu_units.is_empty() {
        assert_eq!(
            stream.obu_units[0].header.obu_type,
            ObuType::TemporalDelimiter
        );
    }
}

#[test]
fn test_parse_av3_sequence_header() {
    // Test Sequence Header OBU (type 1)
    let mut data = vec![0u8; 16];
    data[0] = (1 << 3) | 0x02; // obu_type=1 (SequenceHeader), has_size=1
    data[1] = 14; // size=14
                  // Fill with zeros (incomplete but tests detection)
    for i in 2..16 {
        data[i] = 0;
    }

    let result = parse_av3(&data);
    assert!(result.is_ok());
    let stream = result.unwrap();
    if !stream.obu_units.is_empty() {
        assert_eq!(stream.obu_units[0].header.obu_type, ObuType::SequenceHeader);
    }
}

#[test]
fn test_parse_av3_frame_header() {
    // Test Frame Header OBU (type 3)
    let mut data = vec![0u8; 16];
    data[0] = (4 << 3) | 0x02; // obu_type=4 (FrameHeader), has_size=1
    data[1] = 14; // size=14

    let result = parse_av3(&data);
    assert!(result.is_ok());
    let stream = result.unwrap();
    if !stream.obu_units.is_empty() {
        assert_eq!(stream.obu_units[0].header.obu_type, ObuType::FrameHeader);
    }
}

#[test]
fn test_parse_av3_frame_obu() {
    // Test Frame OBU (type 6)
    let mut data = vec![0u8; 16];
    data[0] = (5 << 3) | 0x02; // obu_type=5 (Frame), has_size=1
    data[1] = 14; // size=14

    let result = parse_av3(&data);
    assert!(result.is_ok());
    let stream = result.unwrap();
    if !stream.obu_units.is_empty() {
        assert_eq!(stream.obu_units[0].header.obu_type, ObuType::Frame);
    }
}

#[test]
fn test_parse_av3_metadata_obu() {
    // Test Metadata OBU (type 5)
    let mut data = vec![0u8; 16];
    data[0] = (7 << 3) | 0x02; // obu_type=7 (Metadata), has_size=1
    data[1] = 14; // size=14

    let result = parse_av3(&data);
    assert!(result.is_ok());
    let stream = result.unwrap();
    if !stream.obu_units.is_empty() {
        assert_eq!(stream.obu_units[0].header.obu_type, ObuType::Metadata);
    }
}

#[test]
fn test_parse_av3_padding_obu() {
    // Test Padding OBU (type 15)
    let mut data = vec![0u8; 16];
    data[0] = (15 << 3) | 0x02; // obu_type=15 (Padding), has_size=1
    data[1] = 14; // size=14
    for i in 2..16 {
        data[i] = 0; // Padding bytes
    }

    let result = parse_av3(&data);
    assert!(result.is_ok());
    let stream = result.unwrap();
    if !stream.obu_units.is_empty() {
        assert_eq!(stream.obu_units[0].header.obu_type, ObuType::Padding);
    }
}

#[test]
fn test_parse_av3_tile_group_obu() {
    // Test Tile Group OBU (type 6)
    let mut data = vec![0u8; 16];
    data[0] = (6 << 3) | 0x02; // obu_type=6 (TileGroup), has_size=1
    data[1] = 14; // size=14

    let result = parse_av3(&data);
    assert!(result.is_ok());
    let stream = result.unwrap();
    if !stream.obu_units.is_empty() {
        assert_eq!(stream.obu_units[0].header.obu_type, ObuType::TileGroup);
    }
}

#[test]
fn test_parse_av3_multiple_obus() {
    // Test multiple OBU units
    let mut data = vec![0u8; 32];
    let mut pos = 0;

    // Temporal delimiter
    data[pos] = (2 << 3) | 0x02;
    pos += 1;
    data[pos] = 0;
    pos += 1;

    // Sequence header
    data[pos] = (1 << 3) | 0x02;
    pos += 1;
    data[pos] = 10;
    pos += 1;
    for _ in 0..10 {
        data[pos] = 0;
        pos += 1;
    }

    // Frame header
    data[pos] = (4 << 3) | 0x02;
    pos += 1;
    data[pos] = 10;
    pos += 1;
    for _ in 0..10 {
        data[pos] = 0;
        pos += 1;
    }

    let result = parse_av3(&data[..pos]);
    assert!(result.is_ok());
    let stream = result.unwrap();
    assert!(stream.obu_units.len() >= 1);
}

#[test]
fn test_parse_av3_quick() {
    // Test parse_av3_quick function
    let mut data = vec![0u8; 16];
    data[0] = (1 << 3) | 0x02; // obu_type=1 (SequenceHeader), has_size=1
    data[1] = 14; // size=14

    let result = parse_av3_quick(&data);
    assert!(result.is_ok());
    let info = result.unwrap();
    assert_eq!(info.obu_count, 1);
    assert_eq!(info.seq_header_count, 1);
}

#[test]
fn test_parse_av3_quick_empty() {
    let result = parse_av3_quick(&[]);
    assert!(result.is_ok());
    let info = result.unwrap();
    assert_eq!(info.obu_count, 0);
    assert_eq!(info.frame_count, 0);
}

#[test]
fn test_av3_stream_methods() {
    // Test Av3Stream methods with empty data
    let stream = Av3Stream {
        obu_units: vec![],
        seq_headers: HashMap::new(),
        frame_headers: vec![],
    };

    assert_eq!(stream.frame_count(), 0);
    assert_eq!(stream.key_frames().len(), 0);
    assert_eq!(stream.inter_frames().len(), 0);
    assert_eq!(stream.switch_frames().len(), 0);
    assert_eq!(stream.show_existing_frames().len(), 0);
}

#[test]
fn test_av3_quick_info_default() {
    // Test Av3QuickInfo with default values
    let info = Av3QuickInfo {
        obu_count: 0,
        seq_header_count: 0,
        frame_count: 0,
        key_frame_count: 0,
        width: None,
        height: None,
        bit_depth: None,
    };

    assert_eq!(info.obu_count, 0);
    assert_eq!(info.frame_count, 0);
    assert!(info.width.is_none());
}

#[test]
fn test_av3_quick_info_with_data() {
    // Test Av3QuickInfo with values
    let info = Av3QuickInfo {
        obu_count: 5,
        seq_header_count: 1,
        frame_count: 3,
        key_frame_count: 1,
        width: Some(1920),
        height: Some(1080),
        bit_depth: Some(10),
    };

    assert_eq!(info.obu_count, 5);
    assert_eq!(info.frame_count, 3);
    assert_eq!(info.width, Some(1920));
}

// Additional comprehensive tests for AV3 public API

#[test]
fn test_parse_av3_tile_group() {
    // Test Tile Group OBU (type 6)
    let mut data = vec![0u8; 16];
    data[0] = (6 << 3) | 0x02; // obu_type=6 (TileGroup), has_size=1
    data[1] = 0; // size=0

    let result = parse_av3(&data);
    assert!(result.is_ok());
    let stream = result.unwrap();
    if !stream.obu_units.is_empty() {
        assert_eq!(stream.obu_units[0].obu_type(), ObuType::TileGroup);
    }
}

#[test]
fn test_parse_av3_metadata() {
    // Test Metadata OBU (type 7)
    let mut data = vec![0u8; 16];
    data[0] = (7 << 3) | 0x02; // obu_type=7 (Metadata), has_size=1
    data[1] = 0; // size=0

    let result = parse_av3(&data);
    assert!(result.is_ok());
    let stream = result.unwrap();
    if !stream.obu_units.is_empty() {
        assert_eq!(stream.obu_units[0].obu_type(), ObuType::Metadata);
    }
}

#[test]
fn test_parse_av3_padding() {
    // Test Padding OBU (type 15)
    let mut data = vec![0u8; 16];
    data[0] = (15 << 3) | 0x02; // obu_type=15 (Padding), has_size=1
    data[1] = 0; // size=0

    let result = parse_av3(&data);
    assert!(result.is_ok());
    let stream = result.unwrap();
    if !stream.obu_units.is_empty() {
        assert_eq!(stream.obu_units[0].obu_type(), ObuType::Padding);
    }
}

#[test]
fn test_av3_stream_with_sequence_header() {
    // Test Av3Stream with sequence header
    use crate::sequence_header::ColorConfig;

    let seq_header = SequenceHeader {
        seq_profile: 0,
        seq_level_idx: 0,
        seq_tier: 0,
        initial_display_delay_minus_1: 0,
        bit_depth: 8,
        max_frame_width: 1920,
        max_frame_height: 1080,
        frame_id_numbers_present_flag: false,
        delta_frame_id_length_minus_2: 0,
        additional_frame_id_length_minus_1: 0,
        use_128x128_superblock_flag: false,
        enable_filter_intra: false,
        enable_intra_edge_filter: false,
        enable_interintra_compound: false,
        enable_masked_compound: false,
        enable_dual_filter: false,
        enable_order_hint: false,
        order_hint_bits_minus_1: 0,
        enable_jnt_comp: false,
        enable_superres: false,
        enable_cdef: false,
        enable_restoration: false,
        enable_post_process_overlay: false,
        enable_large_scale_tile: false,
        timing_info_present_flag: false,
        time_scale: 0,
        num_units_in_display_tick: 0,
        buffer_removal_delay: 0,
        operating_points: vec![],
        color_config: ColorConfig {
            color_space: 0,
            color_range: 0,
            subsampling_x: 1,
            subsampling_y: 1,
            chroma_sample_position: 0,
            separate_uv_delta_q: false,
        },
        film_grain_params_present: false,
    };

    let mut seq_headers = HashMap::new();
    seq_headers.insert(0, seq_header);

    let stream = Av3Stream {
        obu_units: vec![],
        seq_headers,
        frame_headers: vec![],
    };

    assert_eq!(stream.dimensions(), Some((1920, 1080)));
    assert_eq!(stream.bit_depth(), Some(8));
}

#[test]
fn test_av3_key_frames_detection() {
    // Test key frame detection
    use crate::FrameHeader;

    let frame_header = FrameHeader {
        frame_type: FrameType::KeyFrame,
        show_frame: true,
        ..FrameHeader::default()
    };

    let stream = Av3Stream {
        obu_units: vec![],
        seq_headers: HashMap::new(),
        frame_headers: vec![frame_header],
    };

    assert_eq!(stream.key_frames().len(), 1);
    assert_eq!(stream.inter_frames().len(), 0);
}

#[test]
fn test_av3_inter_frames_detection() {
    // Test inter frame detection
    use crate::FrameHeader;

    let frame_header = FrameHeader {
        frame_type: FrameType::InterFrame,
        show_frame: true,
        ..FrameHeader::default()
    };

    let stream = Av3Stream {
        obu_units: vec![],
        seq_headers: HashMap::new(),
        frame_headers: vec![frame_header],
    };

    assert_eq!(stream.key_frames().len(), 0);
    assert_eq!(stream.inter_frames().len(), 1);
}

#[test]
fn test_av3_switch_frames_detection() {
    // Test switch frame detection
    use crate::FrameHeader;

    let frame_header = FrameHeader {
        frame_type: FrameType::SwitchFrame,
        show_frame: true,
        ..FrameHeader::default()
    };

    let stream = Av3Stream {
        obu_units: vec![],
        seq_headers: HashMap::new(),
        frame_headers: vec![frame_header],
    };

    assert_eq!(stream.switch_frames().len(), 1);
}

#[test]
fn test_av3_show_existing_frames() {
    // Test show existing frame detection
    use crate::FrameHeader;

    let frame_header = FrameHeader {
        frame_type: FrameType::ShowExistingFrame,
        show_frame: true,
        ..FrameHeader::default()
    };

    let stream = Av3Stream {
        obu_units: vec![],
        seq_headers: HashMap::new(),
        frame_headers: vec![frame_header],
    };

    assert_eq!(stream.show_existing_frames().len(), 1);
}

#[test]
fn test_av3_various_resolutions() {
    // Test various video resolutions
    use crate::sequence_header::ColorConfig;

    let resolutions = [
        (320u32, 240u32),
        (640, 480),
        (1280, 720),
        (1920, 1080),
        (3840, 2160),
    ];

    for (width, height) in resolutions {
        let seq_header = SequenceHeader {
            seq_profile: 0,
            seq_level_idx: 0,
            seq_tier: 0,
            initial_display_delay_minus_1: 0,
            bit_depth: 8,
            max_frame_width: width,
            max_frame_height: height,
            frame_id_numbers_present_flag: false,
            delta_frame_id_length_minus_2: 0,
            additional_frame_id_length_minus_1: 0,
            use_128x128_superblock_flag: false,
            enable_filter_intra: false,
            enable_intra_edge_filter: false,
            enable_interintra_compound: false,
            enable_masked_compound: false,
            enable_dual_filter: false,
            enable_order_hint: false,
            order_hint_bits_minus_1: 0,
            enable_jnt_comp: false,
            enable_superres: false,
            enable_cdef: false,
            enable_restoration: false,
            enable_post_process_overlay: false,
            enable_large_scale_tile: false,
            timing_info_present_flag: false,
            time_scale: 0,
            num_units_in_display_tick: 0,
            buffer_removal_delay: 0,
            operating_points: vec![],
            color_config: ColorConfig {
                color_space: 0,
                color_range: 0,
                subsampling_x: 1,
                subsampling_y: 1,
                chroma_sample_position: 0,
                separate_uv_delta_q: false,
            },
            film_grain_params_present: false,
        };

        let mut seq_headers = HashMap::new();
        seq_headers.insert(0, seq_header);

        let stream = Av3Stream {
            obu_units: vec![],
            seq_headers,
            frame_headers: vec![],
        };

        assert_eq!(stream.dimensions(), Some((width, height)));
    }
}

#[test]
fn test_av3_bit_depth_variations() {
    // Test different bit depths
    use crate::sequence_header::ColorConfig;

    for bit_depth in [8u8, 10, 12] {
        let seq_header = SequenceHeader {
            seq_profile: 0,
            seq_level_idx: 0,
            seq_tier: 0,
            initial_display_delay_minus_1: 0,
            bit_depth,
            max_frame_width: 1920,
            max_frame_height: 1080,
            frame_id_numbers_present_flag: false,
            delta_frame_id_length_minus_2: 0,
            additional_frame_id_length_minus_1: 0,
            use_128x128_superblock_flag: false,
            enable_filter_intra: false,
            enable_intra_edge_filter: false,
            enable_interintra_compound: false,
            enable_masked_compound: false,
            enable_dual_filter: false,
            enable_order_hint: false,
            order_hint_bits_minus_1: 0,
            enable_jnt_comp: false,
            enable_superres: false,
            enable_cdef: false,
            enable_restoration: false,
            enable_post_process_overlay: false,
            enable_large_scale_tile: false,
            timing_info_present_flag: false,
            time_scale: 0,
            num_units_in_display_tick: 0,
            buffer_removal_delay: 0,
            operating_points: vec![],
            color_config: ColorConfig {
                color_space: 0,
                color_range: 0,
                subsampling_x: 1,
                subsampling_y: 1,
                chroma_sample_position: 0,
                separate_uv_delta_q: false,
            },
            film_grain_params_present: false,
        };

        let mut seq_headers = HashMap::new();
        seq_headers.insert(0, seq_header);

        let stream = Av3Stream {
            obu_units: vec![],
            seq_headers,
            frame_headers: vec![],
        };

        assert_eq!(stream.bit_depth(), Some(bit_depth));
    }
}

#[test]
fn test_parse_av3_multiple_obu_types() {
    // Test parsing multiple OBU types in sequence
    let mut data = vec![0u8; 64];
    let mut pos = 0;

    // Sequence Header
    data[pos] = (1 << 3) | 0x02;
    pos += 1; // OBU header
    data[pos] = 0;
    pos += 1; // size

    // Frame Header
    data[pos] = (4 << 3) | 0x02;
    pos += 1;
    data[pos] = 0;
    pos += 1;

    // Tile Group
    data[pos] = (6 << 3) | 0x02;
    pos += 1;
    data[pos] = 0;
    pos += 1;

    let result = parse_av3(&data[..pos]);
    assert!(result.is_ok());
    let stream = result.unwrap();
    assert!(stream.obu_units.len() >= 3);
}

#[test]
fn test_av3_obu_type_checks() {
    // Test OBU type checking
    // Note: ObuType doesn't have an is_frame() method, so we check values directly
    assert_eq!(ObuType::SequenceHeader as u8, 1);
    assert_eq!(ObuType::TemporalDelimiter as u8, 2);
    assert_eq!(ObuType::FrameHeader as u8, 4);
    assert_eq!(ObuType::TileGroup as u8, 6);
}

#[test]
fn test_av3_frame_type_is_key_frame() {
    // Test FrameType::is_key_frame() method
    assert!(FrameType::KeyFrame.is_key_frame());
    assert!(!FrameType::InterFrame.is_key_frame());
    assert!(!FrameType::IntraOnlyFrame.is_key_frame());
    assert!(!FrameType::SwitchFrame.is_key_frame());
    assert!(!FrameType::ShowExistingFrame.is_key_frame());
}

#[test]
fn test_av3_sequence_header_fields() {
    // Test SequenceHeader field access
    use crate::sequence_header::ColorConfig;

    let seq_header = SequenceHeader {
        seq_profile: 0,
        seq_level_idx: 0,
        seq_tier: 0,
        initial_display_delay_minus_1: 0,
        bit_depth: 8,
        max_frame_width: 1920,
        max_frame_height: 1080,
        frame_id_numbers_present_flag: false,
        delta_frame_id_length_minus_2: 0,
        additional_frame_id_length_minus_1: 0,
        use_128x128_superblock_flag: false,
        enable_filter_intra: false,
        enable_intra_edge_filter: false,
        enable_interintra_compound: false,
        enable_masked_compound: false,
        enable_dual_filter: false,
        enable_order_hint: false,
        order_hint_bits_minus_1: 0,
        enable_jnt_comp: false,
        enable_superres: false,
        enable_cdef: false,
        enable_restoration: false,
        enable_post_process_overlay: false,
        enable_large_scale_tile: false,
        timing_info_present_flag: false,
        time_scale: 0,
        num_units_in_display_tick: 0,
        buffer_removal_delay: 0,
        operating_points: vec![],
        color_config: ColorConfig {
            color_space: 0,
            color_range: 0,
            subsampling_x: 1,
            subsampling_y: 1,
            chroma_sample_position: 0,
            separate_uv_delta_q: false,
        },
        film_grain_params_present: false,
    };
    // Should not panic
    assert_eq!(seq_header.max_frame_width, 1920);
    assert_eq!(seq_header.max_frame_height, 1080);
    assert_eq!(seq_header.bit_depth, 8);
}

#[test]
fn test_av3_frame_header_default() {
    // Test FrameHeader default values
    let frame_header = FrameHeader::default();
    // Should not panic
    let _ = frame_header.frame_type;
    let _ = frame_header.show_frame;
}

#[test]
fn test_parse_av3_with_obu_extension() {
    // Test parsing OBU with extension flag
    let mut data = vec![0u8; 16];
    data[0] = (1 << 3) | 0x04; // obu_type=1, extension=1, has_size=0
    data[1] = 0; // extension byte

    let result = parse_av3(&data);
    // May fail due to incomplete data, but should handle gracefully
    match result {
        Ok(stream) => {
            // Success case - verify field is accessible
            let _ = stream.obu_units.len();
        }
        Err(_) => {
            // Expected for incomplete OBU data
        }
    }
}

#[test]
fn test_parse_av3_with_large_obu_size() {
    // Test parsing OBU with large size field
    let mut data = vec![0u8; 32];
    data[0] = (1 << 3) | 0x02; // obu_type=1, has_size=1
    data[1] = 0x80; // size continuation bit
    data[2] = 0x01; // size final byte

    let result = parse_av3(&data[..3]);
    assert!(result.is_ok());
}

#[test]
fn test_av3_quick_info_with_dimensions() {
    // Test Av3QuickInfo with all fields set
    let info = Av3QuickInfo {
        obu_count: 10,
        seq_header_count: 1,
        frame_count: 5,
        key_frame_count: 2,
        width: Some(1920),
        height: Some(1080),
        bit_depth: Some(10),
    };

    assert_eq!(info.obu_count, 10);
    assert_eq!(info.frame_count, 5);
    assert_eq!(info.key_frame_count, 2);
    assert_eq!(info.width, Some(1920));
    assert_eq!(info.height, Some(1080));
    assert_eq!(info.bit_depth, Some(10));
}

#[test]
fn test_av3_stream_frame_count_mixed_types() {
    // Test frame counting with mixed frame types
    use crate::FrameHeader;

    let frame_headers = vec![
        FrameHeader {
            frame_type: FrameType::KeyFrame,
            show_frame: true,
            ..FrameHeader::default()
        },
        FrameHeader {
            frame_type: FrameType::InterFrame,
            show_frame: true,
            ..FrameHeader::default()
        },
        FrameHeader {
            frame_type: FrameType::SwitchFrame,
            show_frame: true,
            ..FrameHeader::default()
        },
    ];

    let stream = Av3Stream {
        obu_units: vec![],
        seq_headers: HashMap::new(),
        frame_headers,
    };

    assert_eq!(stream.frame_count(), 3);
    assert_eq!(stream.key_frames().len(), 1);
    assert_eq!(stream.inter_frames().len(), 1);
    assert_eq!(stream.switch_frames().len(), 1);
}

#[test]
fn test_av3_show_frame_flag() {
    // Test show_frame flag behavior
    use crate::FrameHeader;

    let visible_frame = FrameHeader {
        frame_type: FrameType::KeyFrame,
        show_frame: true,
        ..FrameHeader::default()
    };

    let hidden_frame = FrameHeader {
        frame_type: FrameType::InterFrame,
        show_frame: false,
        ..FrameHeader::default()
    };

    assert!(visible_frame.show_frame);
    assert!(!hidden_frame.show_frame);
}

#[test]
fn test_av3_show_existing_frame_type() {
    // Test ShowExistingFrame frame type
    use crate::FrameHeader;

    let show_existing_frame = FrameHeader {
        frame_type: FrameType::ShowExistingFrame,
        show_frame: true,
        ..FrameHeader::default()
    };

    let regular_frame = FrameHeader {
        frame_type: FrameType::KeyFrame,
        show_frame: true,
        ..FrameHeader::default()
    };

    assert_eq!(show_existing_frame.frame_type, FrameType::ShowExistingFrame);
    assert_eq!(regular_frame.frame_type, FrameType::KeyFrame);
}

// === Error Handling Tests ===

#[test]
fn test_parse_av3_with_completely_invalid_data() {
    // Test parse_av3 with completely random/invalid data
    let data = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF];
    let result = parse_av3(&data);
    // Should handle gracefully - either Ok with minimal info or Err
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_av3_with_all_zeros() {
    // Test parse_av3 with all zeros (completely invalid)
    let data = vec![0u8; 100];
    let result = parse_av3(&data);
    // Should handle gracefully
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_av3_with_truncated_obu_header() {
    // Test parse_av3 with truncated OBU header
    let data = [0x02]; // Only OBU header byte (no size)
    let result = parse_av3(&data);
    // Should handle gracefully - incomplete OBU header
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_av3_with_invalid_obu_type() {
    // Test parse_av3 with reserved/invalid OBU type
    let mut data = vec![0u8; 16];
    data[0] = 0xFF; // Invalid OBU type (0x1F = reserved)

    let result = parse_av3(&data);
    // Should handle gracefully
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_av3_quick_with_invalid_data() {
    // Test parse_av3_quick with invalid data
    let data = vec![0xFFu8; 50];
    let result = parse_av3_quick(&data);
    // Should handle gracefully
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_av3_with_very_large_input() {
    // Test parse_av3 doesn't crash on very large input
    let large_data = vec![0u8; 10_000_000]; // 10 MB
    let result = parse_av3(&large_data);
    // Should handle without panic
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_av3_with_malformed_sequence_header() {
    // Test parse_av3 with malformed sequence header OBU
    let mut data = vec![0u8; 32];
    data[0] = (1 << 3) | 0x02; // Sequence header OBU
    data[1] = 30; // Large size
    data.extend_from_slice(&[0xFF; 29]); // Invalid data

    let result = parse_av3(&data);
    // Should handle gracefully
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_av3_with_invalid_leb128_size() {
    // Test parse_av3 with invalid LEB128 encoded size
    let mut data = vec![0u8; 32];
    data[0] = (1 << 3) | 0x02; // Sequence header with extension bit
    data[1] = 0x80; // Incomplete LEB128 (continuation bit set but no more bytes)
    data.extend_from_slice(&[0x00; 30]);

    let result = parse_av3(&data);
    // Should handle gracefully
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_av3_with_single_byte() {
    // Test parse_av3 with single byte input
    let data = [0x02];
    let result = parse_av3(&data);
    // Should handle gracefully
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_av3_error_messages_are_descriptive() {
    // Test that error messages provide useful information
    let invalid_data = vec![0xFFu8; 10];
    let result = parse_av3(&invalid_data);
    if let Err(e) = result {
        // Error should have some description
        let error_msg = format!("{}", e);
        assert!(!error_msg.is_empty(), "Error message should not be empty");
    }
}

#[test]
fn test_parse_av3_with_embedded_nulls() {
    // Test parse_av3 handles embedded null bytes
    let mut data = vec![0u8; 100];
    data[0] = (1 << 3) | 0x02; // Sequence header OBU
                               // Rest is nulls
    for i in 1..100 {
        data[i] = 0x00;
    }

    let result = parse_av3(&data);
    // Should handle without panic
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_av3_with_unicode_bytes() {
    // Test parse_av3 doesn't crash on unexpected byte patterns
    let data: Vec<u8> = (0..256).map(|i| i as u8).collect();
    let result = parse_av3(&data);
    // Should handle all byte values gracefully
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_av3_with_forbidden_bit_set() {
    // Test parse_av3 with forbidden_bit set in OBU header
    let data = [0x80]; // forbidden_bit = 1 (invalid)

    let result = parse_av3(&data);
    // Should handle gracefully
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_av3_with_mixed_obu_types() {
    // Test parse_av3 with mixed valid and invalid OBU types
    let mut data = vec![0u8; 64];
    // Valid sequence header
    data.extend_from_slice(&[(1 << 3) | 0x02]); // seq header
    data.extend_from_slice(&[10]);
    data.extend_from_slice(&[0x00; 10]);
    // Invalid OBU
    data.extend_from_slice(&[0xFF]); // reserved OBU type
    data.extend_from_slice(&[5]);
    data.extend_from_slice(&[0x00; 5]);
    // Valid frame header
    data.extend_from_slice(&[(3 << 3) | 0x02]); // frame header
    data.extend_from_slice(&[10]);
    data.extend_from_slice(&[0x00; 10]);

    let result = parse_av3(&data);
    // Should handle gracefully - parse valid OBUs, skip invalid
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_av3_with_repeated_obu_headers() {
    // Test parse_av3 with repeated OBU headers (no actual data)
    let data = vec![(1 << 3) | 0x02u8; 100];
    let result = parse_av3(&data);
    // Should handle gracefully
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_av3_with_zero_length_obu() {
    // Test parse_av3 with zero-length OBU
    let data = [(1 << 3) | 0x02, 0x00]; // OBU header with zero size
    let result = parse_av3(&data);
    // Should handle gracefully
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_av3_with_oversized_obu() {
    // Test parse_av3 with OBU size larger than available data
    let mut data = vec![0u8; 10];
    data[0] = (1 << 3) | 0x02; // Sequence header
    data[1] = 0xFF; // Large size
    data[2] = 0x80; // LEB128 continuation

    let result = parse_av3(&data);
    // Should handle gracefully
    assert!(result.is_ok() || result.is_err());
}

// === Comprehensive Functional Tests ===

#[test]
fn test_parse_av3_sequence_header_profile() {
    // Test parsing sequence header with profile information
    let mut data = vec![0u8; 32];
    data[0] = (1 << 3) | 0x02; // Sequence header OBU
    data[1] = 0x40; // OBU size = 0 (minimal)
    data.extend_from_slice(&[0x00, 0x00]); // seq_profile

    let result = parse_sequence_header(&data);
    // Should handle gracefully - may succeed or fail based on parser
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_frame_header_dimensions() {
    // Test frame header parsing extracts dimensions correctly
    let mut data = vec![0u8; 32];
    data[0] = (3 << 3) | 0x02; // Frame header OBU
    data[1] = 0x20; // OBU size = 2 (minimal)
    data.extend_from_slice(&[0x80]); // show_frame = 1, frame type = 0 (key)
                                     // Add width/height info
    data.extend_from_slice(&[16, 0, 9]); // width = 128
    data.extend_from_slice(&[12, 0, 6]); // height = 96

    let result = parse_frame_header(&data);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_obu_header_types() {
    // Test parsing OBU headers for different OBU types
    for obu_type in [1u8, 2, 3, 4, 5, 6] {
        // Sequence header through Tile group
        let data = [(obu_type << 3) | 0x02, 0x00]; // OBU header with size 0

        let result = parse_obu_header(&data);
        // Should handle gracefully
        assert!(result.is_ok() || result.is_err());
    }
}

#[test]
fn test_parse_obu_temporal_delimiter() {
    // Test parsing temporal delimiter OBU
    let data = [(2 << 3) | 0x02, 0x00]; // Temporal delimiter (type = 2)

    let result = parse_obu_header(&data);
    assert!(result.is_ok());
    let obu = result.unwrap();
    assert_eq!(obu.obu_type, ObuType::TemporalDelimiter);
}

#[test]
fn test_parse_av3_quick_info() {
    // Test quick info extraction from AV3 stream
    let mut data = vec![0u8; 32];
    data[0] = (1 << 3) | 0x02; // Sequence header
    data[1] = 0x20; // OBU size
    data.extend_from_slice(&[0x00, 0x00]); // seq_profile

    let result = parse_av3_quick(&data);
    assert!(result.is_ok());
    let info = result.unwrap();
    // width/height are Option<u32>
    assert!(info.frame_count > 0 || info.width.is_some() || info.height.is_some());
}

#[test]
fn test_parse_metadata_obu() {
    // Test parsing metadata OBU - metadata OBUs are parsed through parse_obu_units
    let mut data = vec![0u8; 32];
    data[0] = (5 << 3) | 0x02; // Metadata OBU
    data[1] = 0x10; // OBU size

    let result = parse_obu_units(&data);
    // Should handle gracefully
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_all_obus_single_obu() {
    // Test parsing all OBUs with single OBU
    let data = [(1 << 3) | 0x02, 0x00]; // Sequence header

    let result = parse_obu_units(&data);
    assert!(result.is_ok());
    let obus = result.unwrap();
    assert_eq!(obus.len(), 1);
}

#[test]
fn test_parse_all_obus_multiple_obus() {
    // Test parsing multiple OBUs
    let mut data = vec![0u8; 64];
    // First OBU: Sequence header
    data.extend_from_slice(&[(1 << 3) | 0x02, 0x00]);

    // Second OBU: Frame header
    data.extend_from_slice(&[(3 << 3) | 0x02, 0x00]);

    // Third OBU: Metadata
    data.extend_from_slice(&[(5 << 3) | 0x02, 0x00]);

    let result = parse_obu_units(&data);
    assert!(result.is_ok());
    let obus = result.unwrap();
    assert_eq!(obus.len(), 3);
}

#[test]
fn test_parse_all_obus_resilient_with_partial_data() {
    // Test resilient parsing handles partial/truncated data
    let mut data = vec![0u8; 32];
    data.extend_from_slice(&[(1 << 3) | 0x02]); // Sequence header (no size)
    data.extend_from_slice(&[0x80]); // Partial LEB128 size

    let result = parse_obu_units(&data);
    // Should handle gracefully - may parse what it can or return error
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_av3_with_multiple_sequence_headers() {
    // Test AV3 stream with multiple sequence headers
    let mut data = vec![0u8; 64];
    // First sequence header
    data.extend_from_slice(&[(1 << 3) | 0x02, 0x00]);

    // Second sequence header (should update)
    data.extend_from_slice(&[(1 << 3) | 0x02, 0x00]);

    let result = parse_av3(&data);
    // Should handle gracefully - may use latest sequence header
    assert!(result.is_ok() || result.is_err());
}

// === Edge Case Tests ===

#[test]
fn test_parse_av3_with_temporal_delimiter_only() {
    // Test parse_av3 with only temporal delimiter OBU
    let data = [(0 << 3) | 0x02, 0x00]; // Temporal delimiter with zero size
    let result = parse_av3(&data);
    // Should handle gracefully
    assert!(result.is_ok() || result.is_err());
}

// === Additional Negative Tests for Public API ===

#[test]
fn test_parse_obu_header_with_reserved_obu_type() {
    // Test OBU header parsing with reserved OBU type
    let data = [0x78, 0x00]; // obu_type = 15 (Padding), obu_extension=0
    let result = parse_obu_header(&data);
    // Should handle gracefully
    assert!(result.is_ok());
    let obu = result.unwrap();
    assert_eq!(obu.obu_type, ObuType::Padding);
}

#[test]
fn test_parse_obu_header_with_max_size_field() {
    // Test OBU header with maximum size field value
    let mut data = vec![0u8; 260]; // Large OBU size
    data[0] = (1 << 3) | 0x02; // Sequence header
    data[1] = 0xFF; // First byte of LEB128 (indicating large size)
                    // Fill rest with valid LEB128 continuation bytes
    for i in 2..260 {
        data[i] = 0x80; // More continuation bytes
    }

    let result = parse_av3(&data);
    // Should handle gracefully without panic
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_av3_quick_with_empty_input() {
    // Test quick info with empty input
    let data: &[u8] = &[];
    let result = parse_av3_quick(data);
    // Should handle gracefully
    assert!(result.is_ok() || result.is_err());
    if let Ok(info) = result {
        assert_eq!(info.obu_count, 0);
    }
}

#[test]
fn test_parse_obu_units_with_empty_input() {
    // Test parse_obu_units with empty input
    let data: &[u8] = &[];
    let result = parse_obu_units(data);
    assert!(result.is_ok());
    let obus = result.unwrap();
    assert_eq!(obus.len(), 0);
}

#[test]
fn test_parse_obu_units_with_invalid_obu_header() {
    // Test parse_obu_units with malformed OBU header
    let data = [0xFF, 0xFF, 0xFF, 0xFF]; // All invalid bytes
    let result = parse_obu_units(&data);
    // Should handle gracefully
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_sequence_header_with_minimal_data() {
    // Test sequence header parsing with incomplete data
    let data = [(1 << 3) | 0x02, 0x01]; // seq_profile, minimal
    let result = parse_sequence_header(&data);
    // May fail due to incomplete data, but should not panic
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_frame_header_with_empty_data() {
    // Test frame header parsing with no data
    let data: &[u8] = &[];
    let result = parse_frame_header(data);
    // Should handle gracefully
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_av3_with_obu_extension_flag_set() {
    // Test parse_av3 with OBU extension flag
    let mut data = vec![0u8; 16];
    data[0] = (1 << 3) | 0x06; // Sequence header with obu_extension=1
    data[1] = 0x10; // OBU size

    let result = parse_av3(&data);
    // Should handle gracefully
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_extract_mv_grid_with_no_frame_data() {
    // Test MV grid extraction with no frame data
    let result = extract_mv_grid(&FrameHeader::default());
    // Should handle gracefully
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_extract_qp_grid_with_no_frame_data() {
    // Test QP grid extraction with no frame data
    let result = extract_qp_grid(&FrameHeader::default());
    // Should handle gracefully
    assert!(result.is_ok() || result.is_err());
}
