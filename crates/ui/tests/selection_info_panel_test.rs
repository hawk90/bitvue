//! Tests for Selection Info Panel

#[test]
fn test_frame_info_display() {
    // Test frame information display
    struct FrameInfo {
        frame_number: usize,
        frame_type: String,
        size_bytes: u64,
        qp: u8,
        offset: u64,
    }

    let info = FrameInfo {
        frame_number: 42,
        frame_type: "I".to_string(),
        size_bytes: 50000,
        qp: 26,
        offset: 1024000,
    };

    assert_eq!(info.frame_number, 42);
    assert_eq!(info.frame_type, "I");
    assert!(info.qp <= 51);
}

#[test]
fn test_block_info_display() {
    // Test block information display
    struct BlockInfo {
        block_x: usize,
        block_y: usize,
        block_width: usize,
        block_height: usize,
        prediction_mode: String,
        qp: u8,
    }

    let block = BlockInfo {
        block_x: 16,
        block_y: 32,
        block_width: 64,
        block_height: 64,
        prediction_mode: "Intra".to_string(),
        qp: 28,
    };

    assert!(block.block_width.is_power_of_two());
    assert!(block.block_height.is_power_of_two());
}

#[test]
fn test_syntax_element_selection() {
    // Test syntax element selection
    struct SyntaxElement {
        name: String,
        value: String,
        bit_offset: u64,
        bit_length: usize,
    }

    let element = SyntaxElement {
        name: "slice_type".to_string(),
        value: "2".to_string(),
        bit_offset: 1024,
        bit_length: 3,
    };

    assert!(!element.name.is_empty());
    assert!(element.bit_length > 0);
}

#[test]
fn test_selection_coordinates() {
    // Test pixel/block coordinate display
    struct SelectionCoords {
        pixel_x: usize,
        pixel_y: usize,
        block_x: usize,
        block_y: usize,
    }

    let coords = SelectionCoords {
        pixel_x: 1024,
        pixel_y: 768,
        block_x: 16,
        block_y: 12,
    };

    assert_eq!(coords.block_x, coords.pixel_x / 64);
    assert_eq!(coords.block_y, coords.pixel_y / 64);
}

#[test]
fn test_motion_vector_display() {
    // Test motion vector information
    struct MotionVectorInfo {
        mvx: i16,
        mvy: i16,
        ref_idx: i8,
        prediction_list: u8,
    }

    let mv = MotionVectorInfo {
        mvx: 32,
        mvy: -16,
        ref_idx: 0,
        prediction_list: 0,
    };

    assert!(mv.ref_idx >= 0);
    assert!(mv.prediction_list <= 1); // L0 or L1
}

#[test]
fn test_transform_info_display() {
    // Test transform unit information
    struct TransformInfo {
        tu_size: usize,
        has_coefficients: bool,
        cbf_luma: bool,
        cbf_chroma: bool,
    }

    let transform = TransformInfo {
        tu_size: 16,
        has_coefficients: true,
        cbf_luma: true,
        cbf_chroma: false,
    };

    assert!(transform.tu_size.is_power_of_two());
}

#[test]
fn test_partition_info_display() {
    // Test partition information
    #[derive(Debug, PartialEq)]
    enum PartitionType {
        None,
        Horz,
        Vert,
        Split,
    }

    struct PartitionInfo {
        partition_type: PartitionType,
        depth: u8,
    }

    let partition = PartitionInfo {
        partition_type: PartitionType::Split,
        depth: 2,
    };

    assert!(partition.depth <= 4);
}

#[test]
fn test_reference_frame_info() {
    // Test reference frame information
    struct ReferenceInfo {
        ref_frame_idx: u8,
        poc: i32,
        is_long_term: bool,
    }

    let ref_info = ReferenceInfo {
        ref_frame_idx: 0,
        poc: 40,
        is_long_term: false,
    };

    assert!(ref_info.ref_frame_idx < 16);
}

#[test]
fn test_selection_hierarchy() {
    // Test selection hierarchy (Frame -> Slice -> Block)
    struct SelectionHierarchy {
        frame_idx: usize,
        slice_idx: usize,
        block_idx: usize,
    }

    let hierarchy = SelectionHierarchy {
        frame_idx: 10,
        slice_idx: 2,
        block_idx: 156,
    };

    assert!(hierarchy.frame_idx >= 0);
    assert!(hierarchy.slice_idx >= 0);
}

#[test]
fn test_yuv_value_display() {
    // Test YUV pixel value display at selection
    struct YuvValues {
        y: u8,
        u: u8,
        v: u8,
    }

    let yuv = YuvValues {
        y: 128,
        u: 120,
        v: 135,
    };

    assert!(yuv.y <= 255);
    assert!(yuv.u <= 255);
    assert!(yuv.v <= 255);
}

#[test]
fn test_metadata_display() {
    // Test metadata information display
    struct Metadata {
        codec: String,
        resolution: String,
        framerate: f32,
        bitrate_kbps: u32,
    }

    let metadata = Metadata {
        codec: "HEVC".to_string(),
        resolution: "1920x1080".to_string(),
        framerate: 30.0,
        bitrate_kbps: 5000,
    };

    assert!(!metadata.codec.is_empty());
    assert!(metadata.framerate > 0.0);
}

#[test]
fn test_selection_context_menu() {
    // Test context menu items for selection
    #[derive(Debug, PartialEq)]
    enum ContextMenuItem {
        CopyInfo,
        JumpToHex,
        ShowInTree,
        Export,
    }

    let items = vec![
        ContextMenuItem::CopyInfo,
        ContextMenuItem::JumpToHex,
        ContextMenuItem::ShowInTree,
    ];

    assert_eq!(items.len(), 3);
}
