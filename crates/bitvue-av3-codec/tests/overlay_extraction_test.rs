//! AV3 Overlay Extraction Tests
//!
//! Comprehensive tests for AV3 overlay data extraction.

use bitvue_av3_codec::frame_header::FrameType;
use bitvue_av3_codec::overlay_extraction;
use bitvue_core::partition_grid::PartitionType;
use bitvue_core::BlockMode;

#[test]
fn test_extract_qp_grid_basic() {
    let frame_header = bitvue_av3_codec::frame_header::FrameHeader {
        width: 1920,
        height: 1080,
        sb_size: 64,
        base_q_idx: 100,
        frame_type: FrameType::KeyFrame,
        ..Default::default()
    };

    let result = overlay_extraction::extract_qp_grid(&frame_header);
    assert!(result.is_ok());

    let grid = result.unwrap();
    assert!(grid.grid_w > 0);
    assert!(grid.grid_h > 0);
}

#[test]
fn test_extract_mv_grid_basic() {
    let frame_header = bitvue_av3_codec::frame_header::FrameHeader {
        width: 1920,
        height: 1080,
        sb_size: 64,
        base_q_idx: 100,
        frame_type: FrameType::InterFrame,
        ..Default::default()
    };

    let result = overlay_extraction::extract_mv_grid(&frame_header);
    assert!(result.is_ok());

    let grid = result.unwrap();
    assert!(grid.grid_w > 0);
    assert!(grid.grid_h > 0);
}

#[test]
fn test_extract_partition_grid_basic() {
    let frame_header = bitvue_av3_codec::frame_header::FrameHeader {
        width: 1920,
        height: 1080,
        sb_size: 64,
        base_q_idx: 100,
        frame_type: FrameType::KeyFrame,
        ..Default::default()
    };

    let result = overlay_extraction::extract_partition_grid(&frame_header);
    assert!(result.is_ok());

    let grid = result.unwrap();
    assert!(grid.coded_width > 0);
}

#[test]
fn test_super_block_creation() {
    let sb = overlay_extraction::SuperBlock {
        x: 0,
        y: 0,
        size: 64,
        mode: BlockMode::Intra,
        partition: PartitionType::None,
        qp: 26,
        mv_l0: None,
        transform_size: 4,
        segment_id: 0,
    };

    assert_eq!(sb.x, 0);
    assert_eq!(sb.y, 0);
    assert_eq!(sb.size, 64);
    assert_eq!(sb.mode, BlockMode::Intra);
    assert_eq!(sb.qp, 26);
}

#[test]
fn test_motion_vector_creation() {
    let mv = overlay_extraction::MotionVector::new(10, -5);

    assert_eq!(mv.x, 10);
    assert_eq!(mv.y, -5);
}

#[test]
fn test_motion_vector_zero() {
    let mv = overlay_extraction::MotionVector::zero();

    assert_eq!(mv.x, 0);
    assert_eq!(mv.y, 0);
}

#[test]
fn test_coding_unit_creation() {
    let cu = overlay_extraction::CodingUnit {
        x: 0,
        y: 0,
        size: 64,
        pred_mode: overlay_extraction::PredMode::Intra,
        skip: false,
        qp: 26,
        mv_l0: None,
        transform_size: 4,
        depth: 0,
    };

    assert_eq!(cu.x, 0);
    assert_eq!(cu.y, 0);
    assert_eq!(cu.size, 64);
    assert_eq!(cu.pred_mode, overlay_extraction::PredMode::Intra);
}

#[test]
fn test_pred_mode_variants() {
    let modes = vec![
        overlay_extraction::PredMode::Intra,
        overlay_extraction::PredMode::Inter,
        overlay_extraction::PredMode::Compound,
        overlay_extraction::PredMode::Skip,
    ];

    for mode in modes {
        let _ = format!("{:?}", mode);
    }
}

#[test]
fn test_qp_grid_with_keyframe() {
    let frame_header = bitvue_av3_codec::frame_header::FrameHeader {
        width: 640,
        height: 480,
        sb_size: 64,
        base_q_idx: 80,
        frame_type: FrameType::KeyFrame,
        ..Default::default()
    };

    let result = overlay_extraction::extract_qp_grid(&frame_header);
    assert!(result.is_ok());

    let grid = result.unwrap();
    assert_eq!(grid.qp_min, 80);
    assert_eq!(grid.qp_max, 80);
}

#[test]
fn test_qp_grid_with_interframe() {
    let frame_header = bitvue_av3_codec::frame_header::FrameHeader {
        width: 1280,
        height: 720,
        sb_size: 64,
        base_q_idx: 120,
        frame_type: FrameType::InterFrame,
        ..Default::default()
    };

    let result = overlay_extraction::extract_qp_grid(&frame_header);
    assert!(result.is_ok());

    let grid = result.unwrap();
    assert_eq!(grid.qp_min, 120);
    assert_eq!(grid.qp_max, 120);
}

#[test]
fn test_various_resolutions() {
    let resolutions = vec![
        (640, 480),   // SD
        (1280, 720),  // HD
        (1920, 1080), // Full HD
        (3840, 2160), // 4K
    ];

    for (width, height) in resolutions {
        let frame_header = bitvue_av3_codec::frame_header::FrameHeader {
            width,
            height,
            sb_size: 64,
            base_q_idx: 100,
            frame_type: FrameType::KeyFrame,
            ..Default::default()
        };

        let qp_result = overlay_extraction::extract_qp_grid(&frame_header);
        assert!(qp_result.is_ok());

        let mv_result = overlay_extraction::extract_mv_grid(&frame_header);
        assert!(mv_result.is_ok());

        let part_result = overlay_extraction::extract_partition_grid(&frame_header);
        assert!(part_result.is_ok());
    }
}

#[test]
fn test_qp_range() {
    let qp_values = vec![0i16, 50, 100, 150, 200, 255];

    for base_qp in qp_values {
        let frame_header = bitvue_av3_codec::frame_header::FrameHeader {
            width: 640,
            height: 480,
            sb_size: 64,
            base_q_idx: base_qp as u8,
            frame_type: FrameType::KeyFrame,
            ..Default::default()
        };

        let result = overlay_extraction::extract_qp_grid(&frame_header);
        assert!(result.is_ok());

        let grid = result.unwrap();
        assert_eq!(grid.qp_min, base_qp);
        assert_eq!(grid.qp_max, base_qp);
    }
}

#[test]
fn test_super_block_with_motion_vector() {
    let mv = overlay_extraction::MotionVector::new(5, -3);
    let sb = overlay_extraction::SuperBlock {
        x: 64,
        y: 64,
        size: 64,
        mode: BlockMode::Inter,
        partition: PartitionType::None,
        qp: 26,
        mv_l0: Some(mv),
        transform_size: 8,
        segment_id: 1,
    };

    assert_eq!(sb.size, 64);
    assert_eq!(sb.segment_id, 1);
    assert!(sb.mv_l0.is_some());
    assert_eq!(sb.mv_l0.unwrap().x, 5);
}

#[test]
fn test_super_block_sizes() {
    let sizes = vec![64u8, 128];

    for size in sizes {
        let sb = overlay_extraction::SuperBlock {
            x: 0,
            y: 0,
            size,
            mode: BlockMode::Intra,
            partition: PartitionType::None,
            qp: 26,
            mv_l0: None,
            transform_size: 4,
            segment_id: 0,
        };

        assert_eq!(sb.size, size);
    }
}

#[test]
fn test_transform_sizes() {
    let tx_sizes = vec![4u8, 8, 16, 32];

    for tx_size in tx_sizes {
        let sb = overlay_extraction::SuperBlock {
            x: 0,
            y: 0,
            size: 64,
            mode: BlockMode::Intra,
            partition: PartitionType::None,
            qp: 26,
            mv_l0: None,
            transform_size: tx_size,
            segment_id: 0,
        };

        assert_eq!(sb.transform_size, tx_size);
    }
}

#[test]
fn test_segment_ids() {
    let segment_ids = vec![0u8, 1, 2, 3, 4, 5, 6, 7];

    for seg_id in segment_ids {
        let sb = overlay_extraction::SuperBlock {
            x: 0,
            y: 0,
            size: 64,
            mode: BlockMode::Intra,
            partition: PartitionType::None,
            qp: 26,
            mv_l0: None,
            transform_size: 4,
            segment_id: seg_id,
        };

        assert_eq!(sb.segment_id, seg_id);
    }
}

#[test]
fn test_super_block_positions() {
    let positions = vec![(0u32, 0u32), (64, 0), (0, 64), (64, 64), (128, 128)];

    for (x, y) in positions {
        let sb = overlay_extraction::SuperBlock {
            x,
            y,
            size: 64,
            mode: BlockMode::Intra,
            partition: PartitionType::None,
            qp: 26,
            mv_l0: None,
            transform_size: 4,
            segment_id: 0,
        };

        assert_eq!(sb.x, x);
        assert_eq!(sb.y, y);
    }
}

#[test]
fn test_partition_variants() {
    let partitions = vec![
        PartitionType::None,
        PartitionType::Horz,
        PartitionType::Vert,
        PartitionType::Split,
    ];

    for partition in partitions {
        let sb = overlay_extraction::SuperBlock {
            x: 0,
            y: 0,
            size: 64,
            mode: BlockMode::Intra,
            partition,
            qp: 26,
            mv_l0: None,
            transform_size: 4,
            segment_id: 0,
        };

        assert_eq!(sb.partition, partition);
    }
}

#[test]
fn test_super_block_modes() {
    let modes = vec![BlockMode::Intra, BlockMode::Inter, BlockMode::Skip];

    for mode in modes {
        let sb = overlay_extraction::SuperBlock {
            x: 0,
            y: 0,
            size: 64,
            mode,
            partition: PartitionType::None,
            qp: 26,
            mv_l0: None,
            transform_size: 4,
            segment_id: 0,
        };

        assert_eq!(sb.mode, mode);
    }
}

#[test]
fn test_coding_unit_skip() {
    let cu = overlay_extraction::CodingUnit {
        x: 0,
        y: 0,
        size: 64,
        pred_mode: overlay_extraction::PredMode::Skip,
        skip: true,
        qp: 26,
        mv_l0: None,
        transform_size: 4,
        depth: 0,
    };

    assert!(cu.skip);
    assert_eq!(cu.pred_mode, overlay_extraction::PredMode::Skip);
}

#[test]
fn test_coding_unit_compound() {
    let mv = overlay_extraction::MotionVector::new(5, -3);

    let cu = overlay_extraction::CodingUnit {
        x: 0,
        y: 0,
        size: 64,
        pred_mode: overlay_extraction::PredMode::Compound,
        skip: false,
        qp: 26,
        mv_l0: Some(mv),
        transform_size: 8,
        depth: 0,
    };

    assert_eq!(cu.pred_mode, overlay_extraction::PredMode::Compound);
    assert!(cu.mv_l0.is_some());
}

#[test]
fn test_coding_unit_sizes() {
    let sizes = vec![4u8, 8, 16, 32, 64, 128];

    for size in sizes {
        let cu = overlay_extraction::CodingUnit {
            x: 0,
            y: 0,
            size,
            pred_mode: overlay_extraction::PredMode::Intra,
            skip: false,
            qp: 26,
            mv_l0: None,
            transform_size: 4,
            depth: 0,
        };

        assert_eq!(cu.size, size);
    }
}

#[test]
fn test_coding_unit_depth() {
    let depths = vec![0u8, 1, 2, 3, 4];

    for depth in depths {
        let cu = overlay_extraction::CodingUnit {
            x: 0,
            y: 0,
            size: 64,
            pred_mode: overlay_extraction::PredMode::Intra,
            skip: false,
            qp: 26,
            mv_l0: None,
            transform_size: 4,
            depth,
        };

        assert_eq!(cu.depth, depth);
    }
}

#[test]
fn test_coding_unit_qp_values() {
    let qp_values = vec![0i16, 10, 20, 26, 30, 40, 51];

    for qp in qp_values {
        let cu = overlay_extraction::CodingUnit {
            x: 0,
            y: 0,
            size: 64,
            pred_mode: overlay_extraction::PredMode::Intra,
            skip: false,
            qp,
            mv_l0: None,
            transform_size: 4,
            depth: 0,
        };

        assert_eq!(cu.qp, qp);
    }
}

#[test]
fn test_coding_unit_positions() {
    let positions = vec![(0u32, 0u32), (64, 0), (0, 64), (64, 64), (128, 128)];

    for (x, y) in positions {
        let cu = overlay_extraction::CodingUnit {
            x,
            y,
            size: 64,
            pred_mode: overlay_extraction::PredMode::Intra,
            skip: false,
            qp: 26,
            mv_l0: None,
            transform_size: 4,
            depth: 0,
        };

        assert_eq!(cu.x, x);
        assert_eq!(cu.y, y);
    }
}

#[test]
fn test_grid_dimensions() {
    use bitvue_core::mv_overlay::MVGrid;
    use bitvue_core::partition_grid::PartitionGrid;
    use bitvue_core::qp_heatmap::QPGrid;

    // Test various dimensions
    let dimensions = vec![(640, 480), (1280, 720), (1920, 1080)];

    for (width, height) in dimensions {
        // Use ceiling division to match overlay_extraction implementation
        let grid_w = width / 64;
        let grid_h = (height + 64 - 1) / 64;
        let qp_grid = QPGrid::new(
            grid_w,
            grid_h,
            64,
            64,
            vec![26; (grid_w * grid_h) as usize],
            -1,
        );
        assert_eq!(qp_grid.grid_w * 64, width);

        // Use ceiling division for MV grid to match implementation
        let mv_grid_w = width / 16;
        let mv_grid_h = (height + 16 - 1) / 16;
        let mv_l0 =
            vec![bitvue_core::mv_overlay::MotionVector::ZERO; (mv_grid_w * mv_grid_h) as usize];
        let mv_l1 =
            vec![bitvue_core::mv_overlay::MotionVector::MISSING; (mv_grid_w * mv_grid_h) as usize];
        let mv_grid = MVGrid::new(width, height, 16, 16, mv_l0, mv_l1, None);
        assert_eq!(mv_grid.grid_w, mv_grid_w);

        let part_grid = PartitionGrid::new(width, height, 64);
        assert_eq!(part_grid.coded_width, width);
    }
}

#[test]
fn test_grid_with_zero_dimensions() {
    use bitvue_core::mv_overlay::MVGrid;
    use bitvue_core::partition_grid::PartitionGrid;
    use bitvue_core::qp_heatmap::QPGrid;

    // Zero dimensions should be handled
    let qp_grid = QPGrid::new(0, 0, 64, 64, vec![], -1);
    assert_eq!(qp_grid.grid_w, 0);

    let mv_grid = MVGrid::new(0, 0, 64, 64, vec![], vec![], None);
    assert_eq!(mv_grid.grid_w, 0);

    let part_grid = PartitionGrid::new(0, 0, 64);
    assert_eq!(part_grid.coded_width, 0);
}

#[test]
fn test_large_grid() {
    use bitvue_core::qp_heatmap::QPGrid;

    // Test 4K resolution
    // Use ceiling division for height to match overlay_extraction implementation
    let width: u32 = 3840 / 64; // 60 SBs
    let height: u32 = (2160 + 64 - 1) / 64; // 34 SBs (ceiling division)
    let qp_values = vec![26i16; (width * height) as usize];

    let grid = QPGrid::new(width, height, 64, 64, qp_values, -1);

    assert_eq!(grid.grid_w, 60);
    assert_eq!(grid.grid_h, 34); // Ceiling division for 2160
}

#[test]
fn test_mv_grid_large() {
    use bitvue_core::mv_overlay::MVGrid;

    // Test 4K resolution
    // Use ceiling division for height to match overlay_extraction implementation
    let grid_w = 3840 / 16;
    let grid_h = (2160 + 16 - 1) / 16; // Ceiling division

    let mv_l0 = vec![bitvue_core::mv_overlay::MotionVector::ZERO; (grid_w * grid_h) as usize];
    let mv_l1 = vec![bitvue_core::mv_overlay::MotionVector::MISSING; (grid_w * grid_h) as usize];
    let grid = MVGrid::new(3840, 2160, 16, 16, mv_l0, mv_l1, None);
    assert_eq!(grid.grid_w, grid_w);
    assert_eq!(grid.grid_h, grid_h);
}

#[test]
fn test_qp_values_valid_range() {
    let qp_values = vec![0i16, 10, 20, 30, 40, 50, 100, 150, 200, 255];

    for qp in qp_values {
        let sb = overlay_extraction::SuperBlock {
            x: 0,
            y: 0,
            size: 64,
            mode: BlockMode::Intra,
            partition: PartitionType::None,
            qp,
            mv_l0: None,
            transform_size: 4,
            segment_id: 0,
        };

        assert_eq!(sb.qp, qp);
    }
}

#[test]
fn test_motion_vector_range() {
    let mv_values = vec![
        (0, 0),
        (10, -5),
        (-10, 5),
        (100, -100),
        (-127, 127),
        (255, -255),
    ];

    for (x, y) in mv_values {
        let mv = overlay_extraction::MotionVector::new(x, y);
        assert_eq!(mv.x, x);
        assert_eq!(mv.y, y);
    }
}

#[test]
fn test_superblock_coverage() {
    let width = 1920;
    let height = 1080;
    let sb_size = 64;

    let sb_cols = (width + sb_size - 1) / sb_size;
    let sb_rows = (height + sb_size - 1) / sb_size;

    assert!(sb_cols > 0);
    assert!(sb_rows > 0);
    assert_eq!(sb_cols, 30);
    assert_eq!(sb_rows, 17);
}

#[test]
fn test_superblock_coverage_128() {
    let width = 1920;
    let height = 1080;
    let sb_size = 128;

    let sb_cols = (width + sb_size - 1) / sb_size;
    let sb_rows = (height + sb_size - 1) / sb_size;

    assert!(sb_cols > 0);
    assert!(sb_rows > 0);
    assert_eq!(sb_cols, 15);
    assert_eq!(sb_rows, 9);
}
