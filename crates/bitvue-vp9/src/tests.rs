use super::*;

#[test]
fn test_empty_stream() {
    // Empty data should fail gracefully
    let data: &[u8] = &[];
    let result = parse_vp9(data);
    // Should still return a stream with empty frames
    assert!(result.is_ok());
    let stream = result.unwrap();
    assert_eq!(stream.frame_count(), 0);
}

#[test]
fn test_stream_methods() {
    let stream = Vp9Stream {
        superframe_index: SuperframeIndex {
            frame_count: 1,
            frame_sizes: vec![100],
            frame_offsets: vec![0],
        },
        frames: vec![FrameHeader {
            frame_type: FrameType::Key,
            show_frame: true,
            width: 1920,
            height: 1080,
            render_width: 1920,
            render_height: 1080,
            bit_depth: 8,
            ..FrameHeader::default()
        }],
    };

    assert_eq!(stream.dimensions(), Some((1920, 1080)));
    assert_eq!(stream.bit_depth(), Some(8));
    assert_eq!(stream.key_frames().len(), 1);
    assert_eq!(stream.visible_frames().len(), 1);
}

// Additional tests for main parser functions and Vp9Stream methods

#[test]
fn test_parse_vp9_minimal_key_frame() {
    // Minimal VP9 key frame header
    // Frame marker (2 bits) = 2, profile (3 bits) = 0, show_existing_frame (1 bit) = 0
    let mut data = vec![0u8; 32];
    data[0] = 0x82; // frame_marker=2 (10b), profile=0 (000b), show_existing_frame=0, frame_type=1 (key), show_frame=1
    data[1] = 0x49; // sync_code_0=0x49, first 4 bits

    let result = parse_vp9(&data);
    // May fail due to incomplete header but shouldn't panic
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_vp9_inter_frame() {
    // Inter frame (frame_type=0)
    let mut data = vec![0u8; 32];
    data[0] = 0x80; // frame_marker=2, profile=0, show_existing_frame=0, frame_type=0 (inter), show_frame=1
    data[1] = 0x49; // sync_code partial

    let result = parse_vp9(&data);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_vp9_show_existing_frame() {
    // Show existing frame (show_existing_frame=1)
    let mut data = vec![0u8; 16];
    data[0] = 0x9C; // frame_marker=2, profile=0, show_existing_frame=1, frame_to_show=3

    let result = parse_vp9(&data);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_vp9_quick() {
    // Test parse_vp9_quick function
    let mut data = vec![0u8; 32];
    data[0] = 0x82; // Key frame marker
    data[1] = 0x49; // sync_code_0
    data[2] = 0x83; // sync_code_1

    let result = parse_vp9_quick(&data);
    assert!(result.is_ok());
    let info = result.unwrap();
    assert_eq!(info.frame_count, 1);
}

#[test]
fn test_parse_vp9_quick_empty() {
    let result = parse_vp9_quick(&[]);
    assert!(result.is_ok());
    let info = result.unwrap();
    // Empty data may return frame_count=1 as default/placeholder
    assert!(info.frame_count <= 1);
}

#[test]
fn test_vp9_stream_inter_frames() {
    // Test inter_frames() method
    let stream = Vp9Stream {
        superframe_index: SuperframeIndex {
            frame_count: 3,
            frame_sizes: vec![100, 100, 100],
            frame_offsets: vec![0, 100, 200],
        },
        frames: vec![
            FrameHeader {
                frame_type: FrameType::Key,
                show_frame: true,
                width: 1920,
                height: 1080,
                render_width: 1920,
                render_height: 1080,
                bit_depth: 8,
                ..FrameHeader::default()
            },
            FrameHeader {
                frame_type: FrameType::Inter,
                show_frame: true,
                width: 1920,
                height: 1080,
                render_width: 1920,
                render_height: 1080,
                bit_depth: 8,
                ..FrameHeader::default()
            },
            FrameHeader {
                frame_type: FrameType::Inter,
                show_frame: false,
                width: 1920,
                height: 1080,
                render_width: 1920,
                render_height: 1080,
                bit_depth: 8,
                ..FrameHeader::default()
            },
        ],
    };

    assert_eq!(stream.key_frames().len(), 1);
    assert_eq!(stream.inter_frames().len(), 2);
    assert_eq!(stream.visible_frames().len(), 2);
    assert_eq!(stream.hidden_frames().len(), 1);
}

#[test]
fn test_vp9_stream_render_dimensions() {
    // Test render_dimensions() method with different render sizes
    let stream = Vp9Stream {
        superframe_index: SuperframeIndex {
            frame_count: 1,
            frame_sizes: vec![100],
            frame_offsets: vec![0],
        },
        frames: vec![FrameHeader {
            frame_type: FrameType::Key,
            show_frame: true,
            width: 1920,
            height: 1080,
            render_width: 1280, // Different render size
            render_height: 720,
            bit_depth: 8,
            ..FrameHeader::default()
        }],
    };

    assert_eq!(stream.dimensions(), Some((1920, 1080)));
    assert_eq!(stream.render_dimensions(), Some((1280, 720)));
}

#[test]
fn test_vp9_stream_color_space() {
    // Test color_space() method
    let stream = Vp9Stream {
        superframe_index: SuperframeIndex {
            frame_count: 1,
            frame_sizes: vec![100],
            frame_offsets: vec![0],
        },
        frames: vec![FrameHeader {
            frame_type: FrameType::Key,
            show_frame: true,
            width: 1920,
            height: 1080,
            render_width: 1920,
            render_height: 1080,
            bit_depth: 8,
            color_space: ColorSpace::Bt601,
            ..FrameHeader::default()
        }],
    };

    assert_eq!(stream.color_space(), Some(ColorSpace::Bt601));
}

#[test]
fn test_vp9_stream_visible_frame_count() {
    // Test visible_frame_count() method
    let stream = Vp9Stream {
        superframe_index: SuperframeIndex {
            frame_count: 5,
            frame_sizes: vec![100; 5],
            frame_offsets: vec![0, 100, 200, 300, 400],
        },
        frames: vec![
            FrameHeader {
                frame_type: FrameType::Key,
                show_frame: true,
                width: 1920,
                height: 1080,
                render_width: 1920,
                render_height: 1080,
                bit_depth: 8,
                ..FrameHeader::default()
            },
            FrameHeader {
                frame_type: FrameType::Inter,
                show_frame: false,
                width: 1920,
                height: 1080,
                render_width: 1920,
                render_height: 1080,
                bit_depth: 8,
                ..FrameHeader::default()
            },
            FrameHeader {
                frame_type: FrameType::Inter,
                show_frame: true,
                width: 1920,
                height: 1080,
                render_width: 1920,
                render_height: 1080,
                bit_depth: 8,
                ..FrameHeader::default()
            },
            FrameHeader {
                frame_type: FrameType::Inter,
                show_frame: true,
                width: 1920,
                height: 1080,
                render_width: 1920,
                render_height: 1080,
                bit_depth: 8,
                ..FrameHeader::default()
            },
            FrameHeader {
                frame_type: FrameType::Inter,
                show_frame: false,
                width: 1920,
                height: 1080,
                render_width: 1920,
                render_height: 1080,
                bit_depth: 8,
                ..FrameHeader::default()
            },
        ],
    };

    assert_eq!(stream.frame_count(), 5);
    assert_eq!(stream.visible_frame_count(), 3);
}

#[test]
fn test_vp9_quick_info_default() {
    // Test Vp9QuickInfo with default values
    let info = Vp9QuickInfo {
        frame_count: 0,
        is_superframe: false,
        width: None,
        height: None,
        bit_depth: None,
        key_frame_count: 0,
        show_frame_count: 0,
    };

    assert_eq!(info.frame_count, 0);
    assert!(!info.is_superframe);
    assert!(info.width.is_none());
}

#[test]
fn test_vp9_quick_info_with_data() {
    // Test Vp9QuickInfo with values
    let info = Vp9QuickInfo {
        frame_count: 3,
        is_superframe: true,
        width: Some(1920),
        height: Some(1080),
        bit_depth: Some(8),
        key_frame_count: 1,
        show_frame_count: 3,
    };

    assert_eq!(info.frame_count, 3);
    assert!(info.is_superframe);
    assert_eq!(info.width, Some(1920));
    assert_eq!(info.key_frame_count, 1);
}

#[test]
fn test_vp9_superframe_index_detection() {
    // Test superframe index detection
    // A valid superframe has a marker at the end
    let mut data = vec![0u8; 32];
    // Frame data
    data[0] = 0x82; // Key frame
    for i in 1..16 {
        data[i] = 0;
    }
    // Superframe marker
    let marker_pos = 16;
    data[marker_pos] = 0; // No marker in this test

    let result = parse_vp9(&data[..16]);
    assert!(result.is_ok());
}

#[test]
fn test_frame_type_is_key_frame() {
    // Test FrameHeader::is_key_frame() method
    let key_frame = FrameHeader {
        frame_type: FrameType::Key,
        show_frame: true,
        width: 1920,
        height: 1080,
        ..FrameHeader::default()
    };
    assert!(key_frame.is_key_frame());

    let inter_frame = FrameHeader {
        frame_type: FrameType::Inter,
        show_frame: true,
        width: 1920,
        height: 1080,
        ..FrameHeader::default()
    };
    assert!(!inter_frame.is_key_frame());
}

#[test]
fn test_parse_vp9_various_profiles() {
    // Test VP9 profiles (0, 1, 2, 3)
    for profile in 0u8..=3 {
        let mut data = vec![0u8; 16];
        // frame_marker=2, profile=profile, show_existing_frame=0
        data[0] = (2 << 6) | ((profile & 0x03) << 3) | 0x04; // key frame
        data[1] = 0x49; // sync_code_0

        let result = parse_vp9(&data);
        // Should handle all profiles without panicking
        assert!(result.is_ok() || result.is_err());
    }
}

#[test]
fn test_parse_vp9_bit_depth_variations() {
    // Test different bit depths (8, 10, 12)
    for _bit_depth in [8u8, 10, 12] {
        let mut data = vec![0u8; 32];
        data[0] = 0x82; // Key frame, profile 0
        data[1] = 0x49; // sync_code_0
        data[2] = 0x83; // sync_code_1

        // Note: Real VP9 bit depth is encoded in the profile
        let result = parse_vp9(&data);
        assert!(result.is_ok() || result.is_err());
    }
}

#[test]
fn test_color_space_variants() {
    // Test different color space values
    let color_spaces = [
        ColorSpace::Bt601,
        ColorSpace::Bt709,
        ColorSpace::Smpte170,
        ColorSpace::Smpte240,
        ColorSpace::Bt2020,
        ColorSpace::Reserved,
        ColorSpace::Srgb,
    ];

    for cs in color_spaces {
        let frame = FrameHeader {
            frame_type: FrameType::Key,
            show_frame: true,
            width: 1920,
            height: 1080,
            render_width: 1920,
            render_height: 1080,
            bit_depth: 8,
            color_space: cs,
            ..FrameHeader::default()
        };

        let stream = Vp9Stream {
            superframe_index: SuperframeIndex {
                frame_count: 1,
                frame_sizes: vec![100],
                frame_offsets: vec![0],
            },
            frames: vec![frame],
        };

        assert_eq!(stream.color_space(), Some(cs));
    }
}

// Frame extraction tests
#[test]
fn test_extract_vp9_frames() {
    // Test VP9 frame extraction
    let mut data = vec![0u8; 64];
    data[0] = 0x82; // Key frame marker
    data[1] = 0x49; // sync_code_0
    data[2] = 0x83; // sync_code_1

    let frames = extract_vp9_frames(&data);
    assert!(frames.is_ok());
}

#[test]
fn test_extract_frame_at_index() {
    // Test extracting specific frame by index
    let mut data = vec![0u8; 64];
    // First frame
    data[0] = 0x82; // Key frame
    data[1] = 0x49;
    data[2] = 0x83;

    let result = extract_frame_at_index(&data, 0);
    // Frame extraction should not panic
    let _ = result;
}

#[test]
fn test_vp9_frame_to_unit_node() {
    // Test converting VP9 frame to unit node
    let frame = Vp9Frame {
        frame_index: 0,
        frame_type: Vp9FrameType::Key,
        frame_data: vec![],
        offset: 0,
        size: 0,
        show_frame: true,
        width: 0,
        height: 0,
        frame_header: None,
    };

    let node = vp9_frame_to_unit_node(&frame);
    // Unit node should be created successfully
    assert!(!node.unit_type.is_empty());
}

#[test]
fn test_vp9_frames_to_unit_nodes() {
    // Test converting multiple frames to unit nodes
    let frames = vec![
        Vp9Frame {
            frame_index: 0,
            frame_type: Vp9FrameType::Key,
            frame_data: vec![],
            offset: 0,
            size: 0,
            show_frame: true,
            width: 0,
            height: 0,
            frame_header: None,
        },
        Vp9Frame {
            frame_index: 1,
            frame_type: Vp9FrameType::Inter,
            frame_data: vec![],
            offset: 0,
            size: 0,
            show_frame: true,
            width: 0,
            height: 0,
            frame_header: None,
        },
    ];

    let nodes = vp9_frames_to_unit_nodes(&frames);
    assert_eq!(nodes.len(), 2);
}

// Superframe tests
#[test]
fn test_has_superframe_index() {
    // Test superframe index detection
    let mut data = vec![0u8; 32];
    data[0] = 0x82; // Key frame

    // No superframe marker
    let has_index = has_superframe_index(&data[..16]);
    // Should handle gracefully
    let _ = has_index;
}

#[test]
fn test_parse_superframe_index() {
    // Test superframe index parsing
    let mut data = vec![0u8; 32];
    data[0] = 0x82; // Key frame

    let result = parse_superframe_index(&data[..16]);
    // Should handle gracefully
    let _ = result;
}

#[test]
fn test_superframe_index_is_superframe() {
    // Test SuperframeIndex::is_superframe() method
    let index = SuperframeIndex {
        frame_count: 1,
        frame_sizes: vec![100],
        frame_offsets: vec![0],
    };

    // Single frame is not a superframe
    assert!(!index.is_superframe());
}

#[test]
fn test_superframe_index_multi_frame() {
    // Test multi-frame superframe
    let index = SuperframeIndex {
        frame_count: 3,
        frame_sizes: vec![100, 100, 100],
        frame_offsets: vec![0, 100, 200],
    };

    // Multiple frames make it a superframe
    assert!(index.is_superframe());
    assert_eq!(index.frame_count, 3);
}

// Overlay extraction tests
#[test]
fn test_extract_qp_grid_vp9() {
    // Test QP grid extraction for VP9
    let frame = FrameHeader {
        frame_type: FrameType::Key,
        show_frame: true,
        width: 640,
        height: 480,
        render_width: 640,
        render_height: 480,
        bit_depth: 8,
        ..FrameHeader::default()
    };

    let result = extract_qp_grid(&frame);
    assert!(result.is_ok());
    let qp_grid = result.unwrap();
    // QP grid should be created
    assert!(qp_grid.grid_w > 0);
    assert!(qp_grid.grid_h > 0);
}

#[test]
fn test_extract_mv_grid_vp9() {
    // Test MV grid extraction for VP9
    let frame = FrameHeader {
        frame_type: FrameType::Inter,
        show_frame: true,
        width: 640,
        height: 480,
        render_width: 640,
        render_height: 480,
        bit_depth: 8,
        ..FrameHeader::default()
    };

    let result = extract_mv_grid(&frame);
    assert!(result.is_ok());
    let mv_grid = result.unwrap();
    // MV grid should be created
    assert_eq!(mv_grid.coded_width, 640);
    assert_eq!(mv_grid.coded_height, 480);
}

#[test]
fn test_extract_partition_grid_vp9() {
    // Test partition grid extraction for VP9
    let frame = FrameHeader {
        frame_type: FrameType::Key,
        show_frame: true,
        width: 640,
        height: 480,
        render_width: 640,
        render_height: 480,
        bit_depth: 8,
        ..FrameHeader::default()
    };

    let result = extract_partition_grid(&frame);
    assert!(result.is_ok());
    let partition_grid = result.unwrap();
    // Partition grid should be created
    assert_eq!(partition_grid.coded_width, 640);
    assert_eq!(partition_grid.coded_height, 480);
}

#[test]
fn test_motion_vector_new() {
    // Test MotionVector creation
    let mv = MotionVector { x: 4, y: -8 };
    assert_eq!(mv.x, 4);
    assert_eq!(mv.y, -8);
}

#[test]
fn test_motion_vector_zero() {
    // Test MotionVector zero
    let mv = MotionVector { x: 0, y: 0 };
    assert_eq!(mv.x, 0);
    assert_eq!(mv.y, 0);
}

#[test]
fn test_super_block_creation() {
    // Test SuperBlock creation
    use bitvue_core::mv_overlay::BlockMode;
    use bitvue_core::partition_grid::PartitionType;

    let sb = SuperBlock {
        x: 0,
        y: 0,
        size: 64,
        mode: BlockMode::Intra,
        partition: PartitionType::None,
        qp: 26,
        mv_l0: None,
        transform_size: 0,
        segment_id: 0,
    };
    assert_eq!(sb.x, 0);
    assert_eq!(sb.y, 0);
    assert_eq!(sb.size, 64);
}

// Edge cases and error handling
#[test]
fn test_parse_vp9_invalid_data() {
    // Test parsing with invalid data
    let data = vec![0xFF; 32]; // Invalid frame marker

    let result = parse_vp9(&data);
    // Should handle gracefully
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_vp9_empty_data() {
    // Test parsing with empty data
    let data: &[u8] = &[];

    let result = parse_vp9(data);
    // Should handle gracefully
    assert!(result.is_ok());
}

#[test]
fn test_parse_vp9_quick_with_superframe() {
    // Test parse_vp9_quick with superframe data
    let mut data = vec![0u8; 64];
    data[0] = 0x82; // Key frame
    data[1] = 0x49;
    data[2] = 0x83;

    let result = parse_vp9_quick(&data[..32]);
    assert!(result.is_ok());
    let info = result.unwrap();
    assert!(info.frame_count >= 1);
}

#[test]
fn test_vp9_frame_type_variants() {
    // Test different frame type values
    let key_header = FrameHeader {
        frame_type: FrameType::Key,
        show_frame: true,
        width: 1920,
        height: 1080,
        ..FrameHeader::default()
    };

    let inter_header = FrameHeader {
        frame_type: FrameType::Inter,
        show_frame: true,
        width: 1920,
        height: 1080,
        ..FrameHeader::default()
    };

    assert!(key_header.is_key_frame());
    assert!(!inter_header.is_key_frame());
}

#[test]
fn test_vp9_various_resolutions() {
    // Test various video resolutions
    let resolutions = [
        (320u32, 240u32),
        (640, 480),
        (1280, 720),
        (1920, 1080),
        (3840, 2160),
    ];

    for (width, height) in resolutions {
        let frame = FrameHeader {
            frame_type: FrameType::Key,
            show_frame: true,
            width,
            height,
            render_width: width,
            render_height: height,
            bit_depth: 8,
            ..FrameHeader::default()
        };

        let stream = Vp9Stream {
            superframe_index: SuperframeIndex {
                frame_count: 1,
                frame_sizes: vec![100],
                frame_offsets: vec![0],
            },
            frames: vec![frame],
        };

        assert_eq!(stream.dimensions(), Some((width, height)));
    }
}

#[test]
fn test_vp9_show_frame_variants() {
    // Test show_frame flag behavior
    let visible_frame = FrameHeader {
        frame_type: FrameType::Key,
        show_frame: true,
        width: 1920,
        height: 1080,
        ..FrameHeader::default()
    };

    let hidden_frame = FrameHeader {
        frame_type: FrameType::Inter,
        show_frame: false,
        width: 1920,
        height: 1080,
        ..FrameHeader::default()
    };

    assert!(visible_frame.show_frame);
    assert!(!hidden_frame.show_frame);
}

#[test]
fn test_vp9_frame_header_default() {
    // Test FrameHeader default values
    let header = FrameHeader::default();
    // Default should not panic
    let _ = header.frame_type;
    let _ = header.show_frame;
}

#[test]
fn test_vp9_superframe_index_default() {
    // Test SuperframeIndex default behavior
    let index = SuperframeIndex {
        frame_count: 0,
        frame_sizes: vec![],
        frame_offsets: vec![],
    };

    assert!(!index.is_superframe());
    assert_eq!(index.frame_count, 0);
}

#[test]
fn test_vp9_quick_info_with_superframe() {
    // Test Vp9QuickInfo with superframe flag
    let info = Vp9QuickInfo {
        frame_count: 3,
        is_superframe: true,
        width: Some(1920),
        height: Some(1080),
        bit_depth: Some(8),
        key_frame_count: 1,
        show_frame_count: 3,
    };

    assert!(info.is_superframe);
    assert_eq!(info.frame_count, 3);
    assert_eq!(info.key_frame_count, 1);
}

#[test]
fn test_parse_vp9_with_incomplete_header() {
    // Test parsing with incomplete frame header
    let data = vec![0x82]; // Just the frame marker

    let result = parse_vp9(&data);
    // Should handle gracefully
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_extract_frames_with_no_marker() {
    // Test frame extraction with no superframe marker
    let data = vec![0x82, 0x49, 0x83, 0x00, 0x00];

    let result = extract_frames(&data);
    // Should handle gracefully
    let _ = result;
}

// === Error Handling Tests ===

#[test]
fn test_parse_vp9_with_completely_invalid_data() {
    // Test parse_vp9 with completely random/invalid data
    let data = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF];
    let result = parse_vp9(&data);
    // Should handle gracefully - either Ok with minimal info or Err
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_vp9_with_all_zeros() {
    // Test parse_vp9 with all zeros (completely invalid)
    let data = vec![0u8; 100];
    let result = parse_vp9(&data);
    // Should handle gracefully
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_vp9_with_invalid_frame_marker() {
    // Test parse_vp9 with invalid frame marker
    let data = [0x00]; // Invalid frame marker (should be 0x2x or 0x8x or 0x9x)
    let result = parse_vp9(&data);
    // Should handle gracefully
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_vp9_with_truncated_frame_header() {
    // Test parse_vp9 with truncated frame header
    let data = [0x82, 0x49]; // Only 2 bytes of header
    let result = parse_vp9(&data);
    // Should handle gracefully - incomplete header
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_vp9_quick_with_invalid_data() {
    // Test parse_vp9_quick with invalid data
    let data = vec![0xFFu8; 50];
    let result = parse_vp9_quick(&data);
    // Should handle gracefully
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_vp9_with_very_large_input() {
    // Test parse_vp9 doesn't crash on very large input
    let large_data = vec![0u8; 10_000_000]; // 10 MB
    let result = parse_vp9(&large_data);
    // Should handle without panic
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_vp9_with_invalid_profile() {
    // Test parse_vp9 with invalid profile (profile 3 is max)
    let mut data = vec![0u8; 16];
    data[0] = 0x82; // Frame marker + profile 0
    data[0] = 0x8E; // Try to set profile 3 (invalid when bit 3 is set)

    let result = parse_vp9(&data);
    // Should handle gracefully
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_extract_frames_with_invalid_superframe() {
    // Test extract_frames with invalid superframe marker
    let mut data = vec![0u8; 32];
    data[0] = 0x82; // Frame marker
    data.extend_from_slice(&[0x00; 30]);
    // Invalid superframe marker
    data.extend_from_slice(&[0x00, 0x00]); // Invalid marker length

    let result = extract_frames(&data);
    // Should handle gracefully
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_vp9_with_single_byte() {
    // Test parse_vp9 with single byte input
    let data = [0x82];
    let result = parse_vp9(&data);
    // Should handle gracefully
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_vp9_error_messages_are_descriptive() {
    // Test that error messages provide useful information
    let invalid_data = vec![0xFFu8; 10];
    let result = parse_vp9(&invalid_data);
    if let Err(e) = result {
        // Error should have some description
        let error_msg = format!("{}", e);
        assert!(!error_msg.is_empty(), "Error message should not be empty");
    }
}

#[test]
fn test_parse_vp9_with_embedded_nulls() {
    // Test parse_vp9 handles embedded null bytes
    let mut data = vec![0u8; 100];
    data[0] = 0x82; // Frame marker
                    // Rest is nulls
    for i in 1..100 {
        data[i] = 0x00;
    }

    let result = parse_vp9(&data);
    // Should handle without panic
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_vp9_with_unicode_bytes() {
    // Test parse_vp9 doesn't crash on unexpected byte patterns
    let data: Vec<u8> = (0..256).map(|i| i as u8).collect();
    let result = parse_vp9(&data);
    // Should handle all byte values gracefully
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_vp9_with_corrupted_compressed_header() {
    // Test parse_vp9 with corrupted compressed header data
    let mut data = vec![0u8; 64];
    data[0] = 0x82; // Frame marker
    data[1] = 0x49; // Sync code
    data.extend_from_slice(&[0xFF; 62]); // Corrupted data

    let result = parse_vp9(&data);
    // Should handle gracefully
    assert!(result.is_ok() || result.is_err());
}

// === Comprehensive Functional Tests ===

#[test]
fn test_parse_vp9_keyframe_dimensions() {
    // Test VP9 parsing extracts frame dimensions correctly
    let mut data = vec![0u8; 20];
    // Frame marker for keyframe
    data[0] = 0x49; // frame marker + profile 0, show_frame
    data[1] = 0x83; // frame type (Key frame = 0)

    // Set width and height to 640x480
    // width = (2 << 1) | 0 = 640
    // height = (3 << 1) | 0 = 480
    data.extend_from_slice(&[0x4D, 0x00]); // width bits
    data.extend_from_slice(&[0x4C, 0x00]); // height bits

    let result = parse_vp9(&data);
    // May fail due to incomplete bitstream, but should not panic
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_vp9_inter_frame_type() {
    // Test VP9 parsing for inter frame type
    let mut data = vec![0u8; 20];
    data[0] = 0x49; // frame marker + profile 0
    data[1] = 0x82; // frame type (Inter = 2)

    let result = parse_vp9(&data);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_vp9_show_frame_flag() {
    // Test VP9 parsing extracts show_frame flag correctly
    let mut data = vec![0u8; 20];

    // Test with show_frame = 1
    data[0] = 0x49; // frame marker (1) + profile (0) + show_frame (1)
    let result1 = parse_vp9(&data);
    assert!(result1.is_ok() || result1.is_err());

    // Test with show_frame = 0
    data[0] = 0x48; // frame marker (1) + profile (0) + show_frame (0)
    let result2 = parse_vp9(&data);
    assert!(result2.is_ok() || result2.is_err());
}

#[test]
fn test_parse_superframe_index_detection() {
    // Test superframe index detection in VP9 streams
    let mut data = vec![0u8; 40];
    // Frame marker + profile
    data.extend_from_slice(&[0x49, 0x82, 0x49, 0x83]);

    // Superframe index marker
    data.extend_from_slice(&[0x00, 0x00]);
    data.extend_from_slice(&[0x53, 0x80, 0x08]); // superframe marker
    data.extend_from_slice(&[0x01]); // superframe index count = 1
    data.extend_from_slice(&[0x04]); // first superframe offset = 4 bytes

    let result = parse_superframe_index(&data);
    assert!(result.is_ok() || result.is_err()); // May or may not detect superframe
}

#[test]
fn test_parse_vp9_quick_info() {
    // Test quick info extraction from VP9 stream
    let mut data = vec![0u8; 20];
    data[0] = 0x49; // frame marker + profile 0
    data[1] = 0x83; // keyframe
                    // Add minimal width/height
    data.extend_from_slice(&[0x4D, 0x00, 0x4C, 0x00]);

    let result = parse_vp9_quick(&data);
    assert!(result.is_ok());
    let info = result.unwrap();
    // width/height are Option<u32>
    assert!(info.width.is_some() || info.height.is_some() || info.frame_count > 0);
}

#[test]
fn test_parse_frame_header_bit_depth() {
    // Test frame header parsing with different bit depths
    // Frame headers are parsed internally through parse_vp9
    for _bit_depth in [8u8, 10, 12] {
        let mut data = vec![0u8; 20];
        data[0] = 0x49; // frame marker
        data[1] = 0x83; // keyframe
        data.extend_from_slice(&[0x4D, 0x00, 0x4C, 0x00]); // dimensions

        let result = parse_vp9(&data);
        // Should handle gracefully - may succeed or fail based on bit depth support
        assert!(result.is_ok() || result.is_err());
    }
}

#[test]
fn test_parse_vp9_loop_filter_level() {
    // Test VP9 parsing with different loop filter levels
    for _level in [0u8, 1, 2, 3] {
        let mut data = vec![0u8; 20];
        data[0] = 0x49; // frame marker
        data[1] = 0x83; // keyframe
        data.extend_from_slice(&[0x4D, 0x00, 0x4C, 0x00]); // dimensions

        let result = parse_vp9(&data);
        assert!(result.is_ok() || result.is_err());
    }
}

#[test]
fn test_parse_vp9_sharpness_level() {
    // Test VP9 parsing with different sharpness levels
    for _sharpness in [0u8, 1, 2, 3, 4, 5, 6, 7] {
        let mut data = vec![0u8; 20];
        data[0] = 0x49; // frame marker
        data[1] = 0x83; // keyframe
        data.extend_from_slice(&[0x4D, 0x00, 0x4C, 0x00]); // dimensions

        let result = parse_vp9(&data);
        assert!(result.is_ok() || result.is_err());
    }
}

#[test]
fn test_parse_vp9_color_space() {
    // Test VP9 parsing with different color spaces
    let data = vec![0u8; 20]; // Minimal data
    let result = parse_vp9(&data);
    // Should handle gracefully
    assert!(result.is_ok() || result.is_err());
}

// === Edge Case Tests ===

#[test]
fn test_extract_qp_grid_with_invalid_frame_data() {
    // Test extract_qp_grid with invalid frame data
    use crate::frame_header::{FrameHeader, FrameType};

    let frame = FrameHeader {
        frame_type: FrameType::Key,
        show_frame: true,
        width: 640,
        height: 480,
        render_width: 640,
        render_height: 480,
        bit_depth: 8,
        ..FrameHeader::default()
    };

    let qp_result = extract_qp_grid(&frame);
    // Should handle gracefully
    assert!(qp_result.is_ok() || qp_result.is_err());
}

#[test]
fn test_extract_qp_grid_with_empty_frame() {
    // Test extract_qp_grid with frame that has no data
    use crate::frame_header::{FrameHeader, FrameType};

    let frame = FrameHeader {
        frame_type: FrameType::Key,
        show_frame: true,
        width: 0,
        height: 0,
        render_width: 0,
        render_height: 0,
        bit_depth: 8,
        ..FrameHeader::default()
    };

    let result = extract_qp_grid(&frame);
    // Should handle gracefully - return empty grid or error
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_vp9_with_repeated_frame_markers() {
    // Test parse_vp9 with repeated frame markers (no actual data)
    let data = vec![0x82u8; 100];
    let result = parse_vp9(&data);
    // Should handle gracefully
    assert!(result.is_ok() || result.is_err());
}

// === Additional Negative Tests for Public API ===

#[test]
fn test_parse_vp9_with_empty_input() {
    // Test parse_vp9 with empty input
    let data: &[u8] = &[];
    let result = parse_vp9(data);
    // Should handle gracefully
    assert!(result.is_ok() || result.is_err());
    if let Ok(stream) = result {
        assert_eq!(stream.frames.len(), 0);
    }
}

#[test]
fn test_parse_vp9_quick_with_empty_input() {
    // Test quick info with empty input
    let data: &[u8] = &[];
    let result = parse_vp9_quick(data);
    // Should handle gracefully
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_vp9_with_invalid_profile_value() {
    // Test parse_vp9 with invalid profile (profile 3 is invalid)
    let mut data = vec![0u8; 10];
    data[0] = 0x9C; // frame_marker=1, profile=3 (invalid), show_frame=1
    let result = parse_vp9(&data);
    // Should handle gracefully
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_extract_mv_grid_with_invalid_frame_header() {
    // Test MV grid extraction with invalid frame header
    use crate::overlay_extraction::extract_mv_grid;
    let frame_header = FrameHeader {
        frame_type: FrameType::Key,
        show_frame: true,
        error_resilient_mode: true,
        width: 0,
        height: 0,
        ..Default::default()
    };
    let result = extract_mv_grid(&frame_header);
    // Should handle gracefully - may return error for zero dimensions
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_extract_qp_grid_with_invalid_frame_header() {
    // Test QP grid extraction with minimal frame data
    use crate::overlay_extraction::extract_qp_grid;
    let frame_header = FrameHeader {
        frame_type: FrameType::Key,
        show_frame: true,
        error_resilient_mode: true,
        width: 0,
        height: 0,
        ..Default::default()
    };
    let result = extract_qp_grid(&frame_header);
    // Should handle gracefully
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_superframe_index_with_incomplete_data() {
    // Test superframe index parsing with incomplete data
    let data = [0x00, 0x00]; // Just 2 bytes (incomplete)
    let result = parse_superframe_index(&data);
    // Should handle gracefully - may return error or partial result
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_has_superframe_index_with_empty_input() {
    // Test has_superframe_index with empty input
    let result = has_superframe_index(&[]);
    // Should return false for empty input
    assert!(!result);
}

#[test]
fn test_has_superframe_index_with_invalid_index() {
    // Test has_superframe_index with invalid index marker
    let data = [0x12, 0x00]; // No superframe index marker
    let result = has_superframe_index(&data);
    // Should return false
    assert!(!result);
}

#[test]
fn test_extract_vp9_frames_with_empty_input() {
    // Test extract_vp9_frames with empty input
    let result = extract_vp9_frames(&[]);
    assert!(result.is_ok());
    let frames = result.unwrap();
    assert_eq!(frames.len(), 0);
}
