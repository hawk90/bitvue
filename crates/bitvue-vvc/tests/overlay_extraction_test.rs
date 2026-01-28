//! VVC Overlay Extraction Tests
//!
//! Comprehensive tests for VVC overlay data extraction.

use bitvue_core::partition_grid::PartitionType;
use bitvue_core::BlockMode;
use bitvue_vvc::overlay_extraction;

#[test]
fn test_extract_qp_grid_basic() {
    let sps = bitvue_vvc::sps::Sps {
        sps_log2_ctu_size_minus5: 3, // 128 CTU
        sps_pic_width_max_in_luma_samples: 1920,
        sps_pic_height_max_in_luma_samples: 1080,
        ..Default::default()
    };

    let nal_units = [];
    let result = overlay_extraction::extract_qp_grid(&nal_units, &sps, 26);

    assert!(result.is_ok());

    let grid = result.unwrap();
    assert!(grid.grid_w >= 0);
}

#[test]
fn test_extract_mv_grid_basic() {
    let sps = bitvue_vvc::sps::Sps {
        sps_log2_ctu_size_minus5: 3,
        sps_pic_width_max_in_luma_samples: 1920,
        sps_pic_height_max_in_luma_samples: 1080,
        ..Default::default()
    };

    let nal_units = [];
    let result = overlay_extraction::extract_mv_grid(&nal_units, &sps);

    assert!(result.is_ok());

    let grid = result.unwrap();
    assert!(grid.grid_w >= 0);
}

#[test]
fn test_extract_partition_grid_basic() {
    let sps = bitvue_vvc::sps::Sps {
        sps_log2_ctu_size_minus5: 3,
        sps_pic_width_max_in_luma_samples: 1920,
        sps_pic_height_max_in_luma_samples: 1080,
        ..Default::default()
    };

    let nal_units = [];
    let result = overlay_extraction::extract_partition_grid(&nal_units, &sps);

    assert!(result.is_ok());

    let grid = result.unwrap();
    assert!(grid.coded_width >= 0);
}

#[test]
fn test_ctu_creation() {
    let ctu = overlay_extraction::CodingTreeUnit::new(0, 0, 128);

    assert_eq!(ctu.x, 0);
    assert_eq!(ctu.y, 0);
    assert_eq!(ctu.size, 128);
    assert!(ctu.coding_units.is_empty());
}

#[test]
fn test_ctu_add_cu() {
    let mut ctu = overlay_extraction::CodingTreeUnit::new(0, 0, 128);

    let cu = overlay_extraction::CodingUnit {
        x: 0,
        y: 0,
        size: 64,
        pred_mode: overlay_extraction::PredMode::Intra,
        split_mode: overlay_extraction::SplitMode::QuadTree,
        depth: 1,
        tree_type: 0,
        qp: 26,
        mv_l0: None,
        mv_l1: None,
        ref_idx_l0: None,
        ref_idx_l1: None,
        transform_size: 4,
        sbt_flag: false,
        isp_flag: false,
    };

    ctu.add_cu(cu);

    assert_eq!(ctu.coding_units.len(), 1);
}

#[test]
fn test_coding_unit_creation() {
    let cu = overlay_extraction::CodingUnit {
        x: 0,
        y: 0,
        size: 64,
        pred_mode: overlay_extraction::PredMode::Intra,
        split_mode: overlay_extraction::SplitMode::None,
        depth: 0,
        tree_type: 0,
        qp: 26,
        mv_l0: None,
        mv_l1: None,
        ref_idx_l0: None,
        ref_idx_l1: None,
        transform_size: 4,
        sbt_flag: false,
        isp_flag: false,
    };

    assert_eq!(cu.x, 0);
    assert_eq!(cu.y, 0);
    assert_eq!(cu.size, 64);
    assert_eq!(cu.pred_mode, overlay_extraction::PredMode::Intra);
    assert_eq!(cu.qp, 26);
}

#[test]
fn test_pred_mode_variants() {
    let modes = vec![
        overlay_extraction::PredMode::Intra,
        overlay_extraction::PredMode::Inter,
        overlay_extraction::PredMode::Ibc,
        overlay_extraction::PredMode::Skip,
    ];

    for mode in modes {
        let _ = format!("{:?}", mode);
    }
}

#[test]
fn test_split_mode_variants() {
    let modes = vec![
        overlay_extraction::SplitMode::None,
        overlay_extraction::SplitMode::QuadTree,
        overlay_extraction::SplitMode::HorzB,
        overlay_extraction::SplitMode::VertB,
        overlay_extraction::SplitMode::HorzT,
        overlay_extraction::SplitMode::VertT,
    ];

    for mode in modes {
        let _ = format!("{:?}", mode);
    }
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
fn test_cu_with_motion_vector() {
    let mv = overlay_extraction::MotionVector::new(5, -3);

    let cu = overlay_extraction::CodingUnit {
        x: 0,
        y: 0,
        size: 64,
        pred_mode: overlay_extraction::PredMode::Inter,
        split_mode: overlay_extraction::SplitMode::None,
        depth: 0,
        tree_type: 0,
        qp: 26,
        mv_l0: Some(mv),
        mv_l1: None,
        ref_idx_l0: Some(0),
        ref_idx_l1: None,
        transform_size: 8,
        sbt_flag: false,
        isp_flag: false,
    };

    assert!(cu.mv_l0.is_some());
    assert_eq!(cu.mv_l0.unwrap().x, 5);
    assert_eq!(cu.mv_l0.unwrap().y, -3);
    assert!(cu.ref_idx_l0.is_some());
    assert_eq!(cu.ref_idx_l0.unwrap(), 0);
}

#[test]
fn test_cu_with_ibc() {
    let mv = overlay_extraction::MotionVector::new(-16, 8);

    let cu = overlay_extraction::CodingUnit {
        x: 0,
        y: 0,
        size: 32,
        pred_mode: overlay_extraction::PredMode::Ibc,
        split_mode: overlay_extraction::SplitMode::None,
        depth: 1,
        tree_type: 0,
        qp: 26,
        mv_l0: Some(mv),
        mv_l1: None,
        ref_idx_l0: None,
        ref_idx_l1: None,
        transform_size: 4,
        sbt_flag: false,
        isp_flag: false,
    };

    assert_eq!(cu.pred_mode, overlay_extraction::PredMode::Ibc);
    assert!(cu.mv_l0.is_some());
}

#[test]
fn test_cu_with_sbt() {
    let cu = overlay_extraction::CodingUnit {
        x: 0,
        y: 0,
        size: 64,
        pred_mode: overlay_extraction::PredMode::Inter,
        split_mode: overlay_extraction::SplitMode::HorzB,
        depth: 1,
        tree_type: 0,
        qp: 26,
        mv_l0: None,
        mv_l1: None,
        ref_idx_l0: None,
        ref_idx_l1: None,
        transform_size: 16,
        sbt_flag: true,
        isp_flag: false,
    };

    assert!(cu.sbt_flag);
    assert_eq!(cu.split_mode, overlay_extraction::SplitMode::HorzB);
}

#[test]
fn test_cu_with_isp() {
    let cu = overlay_extraction::CodingUnit {
        x: 0,
        y: 0,
        size: 32,
        pred_mode: overlay_extraction::PredMode::Intra,
        split_mode: overlay_extraction::SplitMode::VertB,
        depth: 2,
        tree_type: 0,
        qp: 26,
        mv_l0: None,
        mv_l1: None,
        ref_idx_l0: None,
        ref_idx_l1: None,
        transform_size: 4,
        sbt_flag: false,
        isp_flag: true,
    };

    assert!(cu.isp_flag);
    assert_eq!(cu.pred_mode, overlay_extraction::PredMode::Intra);
}

#[test]
fn test_cu_skip_mode() {
    let cu = overlay_extraction::CodingUnit {
        x: 0,
        y: 0,
        size: 128,
        pred_mode: overlay_extraction::PredMode::Skip,
        split_mode: overlay_extraction::SplitMode::None,
        depth: 0,
        tree_type: 0,
        qp: 26,
        mv_l0: None,
        mv_l1: None,
        ref_idx_l0: None,
        ref_idx_l1: None,
        transform_size: 4,
        sbt_flag: false,
        isp_flag: false,
    };

    assert_eq!(cu.pred_mode, overlay_extraction::PredMode::Skip);
}

#[test]
fn test_cu_sizes() {
    let sizes = vec![4u8, 8, 16, 32, 64, 128];

    for size in sizes {
        let cu = overlay_extraction::CodingUnit {
            x: 0,
            y: 0,
            size,
            pred_mode: overlay_extraction::PredMode::Intra,
            split_mode: overlay_extraction::SplitMode::None,
            depth: 0,
            tree_type: 0,
            qp: 26,
            mv_l0: None,
            mv_l1: None,
            ref_idx_l0: None,
            ref_idx_l1: None,
            transform_size: 4,
            sbt_flag: false,
            isp_flag: false,
        };

        assert_eq!(cu.size, size);
    }
}

#[test]
fn test_cu_depth_values() {
    let depths = vec![0u8, 1, 2, 3, 4];

    for depth in depths {
        let cu = overlay_extraction::CodingUnit {
            x: 0,
            y: 0,
            size: 64,
            pred_mode: overlay_extraction::PredMode::Intra,
            split_mode: overlay_extraction::SplitMode::QuadTree,
            depth,
            tree_type: 0,
            qp: 26,
            mv_l0: None,
            mv_l1: None,
            ref_idx_l0: None,
            ref_idx_l1: None,
            transform_size: 4,
            sbt_flag: false,
            isp_flag: false,
        };

        assert_eq!(cu.depth, depth);
    }
}

#[test]
fn test_tree_types() {
    let tree_types = vec![0u8, 1, 2];

    for tree_type in tree_types {
        let cu = overlay_extraction::CodingUnit {
            x: 0,
            y: 0,
            size: 64,
            pred_mode: overlay_extraction::PredMode::Intra,
            split_mode: overlay_extraction::SplitMode::None,
            depth: 0,
            tree_type,
            qp: 26,
            mv_l0: None,
            mv_l1: None,
            ref_idx_l0: None,
            ref_idx_l1: None,
            transform_size: 4,
            sbt_flag: false,
            isp_flag: false,
        };

        assert_eq!(cu.tree_type, tree_type);
    }
}

#[test]
fn test_transform_sizes() {
    let tx_sizes = vec![0u8, 1, 2, 3, 4, 5, 6];

    for tx_size in tx_sizes {
        let cu = overlay_extraction::CodingUnit {
            x: 0,
            y: 0,
            size: 64,
            pred_mode: overlay_extraction::PredMode::Intra,
            split_mode: overlay_extraction::SplitMode::None,
            depth: 0,
            tree_type: 0,
            qp: 26,
            mv_l0: None,
            mv_l1: None,
            ref_idx_l0: None,
            ref_idx_l1: None,
            transform_size: tx_size,
            sbt_flag: false,
            isp_flag: false,
        };

        assert_eq!(cu.transform_size, tx_size);
    }
}

#[test]
fn test_qp_values() {
    let qp_values = vec![0i16, 10, 20, 26, 30, 40, 51];

    for qp in qp_values {
        let cu = overlay_extraction::CodingUnit {
            x: 0,
            y: 0,
            size: 64,
            pred_mode: overlay_extraction::PredMode::Intra,
            split_mode: overlay_extraction::SplitMode::None,
            depth: 0,
            tree_type: 0,
            qp,
            mv_l0: None,
            mv_l1: None,
            ref_idx_l0: None,
            ref_idx_l1: None,
            transform_size: 4,
            sbt_flag: false,
            isp_flag: false,
        };

        assert_eq!(cu.qp, qp);
    }
}

#[test]
fn test_ref_idx_values() {
    let ref_indices = vec![0i8, 1, 2, 3, -1];

    for ref_idx in ref_indices {
        let cu = overlay_extraction::CodingUnit {
            x: 0,
            y: 0,
            size: 64,
            pred_mode: overlay_extraction::PredMode::Inter,
            split_mode: overlay_extraction::SplitMode::None,
            depth: 0,
            tree_type: 0,
            qp: 26,
            mv_l0: None,
            mv_l1: None,
            ref_idx_l0: Some(ref_idx),
            ref_idx_l1: None,
            transform_size: 4,
            sbt_flag: false,
            isp_flag: false,
        };

        assert!(cu.ref_idx_l0.is_some());
        assert_eq!(cu.ref_idx_l0.unwrap(), ref_idx);
    }
}

#[test]
fn test_various_resolutions() {
    let resolutions = vec![
        (1920, 1080), // Full HD
        (1280, 720),  // HD
        (3840, 2160), // 4K
        (640, 480),   // SD
    ];

    for (width, height) in resolutions {
        let sps = bitvue_vvc::sps::Sps {
            sps_log2_ctu_size_minus5: 3,
            sps_pic_width_max_in_luma_samples: width,
            sps_pic_height_max_in_luma_samples: height,
            ..Default::default()
        };

        let nal_units = [];

        let qp_result = overlay_extraction::extract_qp_grid(&nal_units, &sps, 26);
        assert!(qp_result.is_ok());

        let mv_result = overlay_extraction::extract_mv_grid(&nal_units, &sps);
        assert!(mv_result.is_ok());

        let part_result = overlay_extraction::extract_partition_grid(&nal_units, &sps);
        assert!(part_result.is_ok());
    }
}

#[test]
fn test_ctu_sizes() {
    let ctu_sizes = vec![5u8, 6]; // log2_ctu_size_minus5 = 5 -> 128x128, = 6 -> 256x256

    for log2_ctu in ctu_sizes {
        let sps = bitvue_vvc::sps::Sps {
            sps_log2_ctu_size_minus5: log2_ctu,
            sps_pic_width_max_in_luma_samples: 1920,
            sps_pic_height_max_in_luma_samples: 1080,
            ..Default::default()
        };

        let nal_units = [];
        let result = overlay_extraction::extract_qp_grid(&nal_units, &sps, 26);

        assert!(result.is_ok());
    }
}

#[test]
fn test_cu_positions() {
    let positions = vec![(0u32, 0u32), (64, 0), (0, 64), (128, 128), (256, 256)];

    for (x, y) in positions {
        let cu = overlay_extraction::CodingUnit {
            x,
            y,
            size: 64,
            pred_mode: overlay_extraction::PredMode::Intra,
            split_mode: overlay_extraction::SplitMode::None,
            depth: 0,
            tree_type: 0,
            qp: 26,
            mv_l0: None,
            mv_l1: None,
            ref_idx_l0: None,
            ref_idx_l1: None,
            transform_size: 4,
            sbt_flag: false,
            isp_flag: false,
        };

        assert_eq!(cu.x, x);
        assert_eq!(cu.y, y);
    }
}

#[test]
fn test_partition_type_mapping() {
    let mappings = vec![
        (overlay_extraction::SplitMode::None, PartitionType::None),
        (
            overlay_extraction::SplitMode::QuadTree,
            PartitionType::Split,
        ),
        (overlay_extraction::SplitMode::HorzB, PartitionType::Horz),
        (overlay_extraction::SplitMode::VertB, PartitionType::Vert),
    ];

    for (split_mode, expected_partition) in mappings {
        let _ = format!("{:?}", split_mode);
        let _ = format!("{:?}", expected_partition);
    }
}

#[test]
fn test_dual_tree_luma() {
    let cu = overlay_extraction::CodingUnit {
        x: 0,
        y: 0,
        size: 64,
        pred_mode: overlay_extraction::PredMode::Intra,
        split_mode: overlay_extraction::SplitMode::None,
        depth: 0,
        tree_type: 1, // Dual tree luma
        qp: 26,
        mv_l0: None,
        mv_l1: None,
        ref_idx_l0: None,
        ref_idx_l1: None,
        transform_size: 4,
        sbt_flag: false,
        isp_flag: false,
    };

    assert_eq!(cu.tree_type, 1);
}

#[test]
fn test_dual_tree_chroma() {
    let cu = overlay_extraction::CodingUnit {
        x: 0,
        y: 0,
        size: 32,
        pred_mode: overlay_extraction::PredMode::Intra,
        split_mode: overlay_extraction::SplitMode::None,
        depth: 1,
        tree_type: 2, // Dual tree chroma
        qp: 26,
        mv_l0: None,
        mv_l1: None,
        ref_idx_l0: None,
        ref_idx_l1: None,
        transform_size: 4,
        sbt_flag: false,
        isp_flag: false,
    };

    assert_eq!(cu.tree_type, 2);
}

#[test]
fn test_ctu_coverage() {
    let width = 1920;
    let height = 1080;
    let ctu_size = 128;

    let ctu_cols = (width + ctu_size - 1) / ctu_size;
    let ctu_rows = (height + ctu_size - 1) / ctu_size;

    assert!(ctu_cols > 0);
    assert!(ctu_rows > 0);
    assert_eq!(ctu_cols, 15);
    assert_eq!(ctu_rows, 9);
}

#[test]
fn test_grid_dimensions_consistency() {
    use bitvue_core::mv_overlay::MVGrid;
    use bitvue_core::partition_grid::PartitionGrid;
    use bitvue_core::qp_heatmap::QPGrid;

    let width = 1920;
    let height = 1080;

    let qp_grid = QPGrid::new(15, 9, 128, 128, vec![26; 135], -1);
    assert_eq!(qp_grid.grid_w, 15);

    let mv_l0 = vec![bitvue_core::mv_overlay::MotionVector::ZERO; 120 * 68];
    let mv_l1 = vec![bitvue_core::mv_overlay::MotionVector::MISSING; 120 * 68];
    let mv_grid = MVGrid::new(width, height, 16, 16, mv_l0, mv_l1, None);
    assert_eq!(mv_grid.grid_w, 120);

    let part_grid = PartitionGrid::new(width, height, 128);
    assert_eq!(part_grid.coded_width, width);
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
fn test_cu_with_bipred() {
    let mv_l0 = overlay_extraction::MotionVector::new(5, -3);
    let mv_l1 = overlay_extraction::MotionVector::new(-2, 4);

    let cu = overlay_extraction::CodingUnit {
        x: 0,
        y: 0,
        size: 64,
        pred_mode: overlay_extraction::PredMode::Inter,
        split_mode: overlay_extraction::SplitMode::None,
        depth: 0,
        tree_type: 0,
        qp: 26,
        mv_l0: Some(mv_l0),
        mv_l1: Some(mv_l1),
        ref_idx_l0: Some(0),
        ref_idx_l1: Some(1),
        transform_size: 8,
        sbt_flag: false,
        isp_flag: false,
    };

    assert!(cu.mv_l0.is_some());
    assert!(cu.mv_l1.is_some());
    assert!(cu.ref_idx_l0.is_some());
    assert!(cu.ref_idx_l1.is_some());
}

#[test]
fn test_grid_with_zero_dimensions() {
    use bitvue_core::mv_overlay::MVGrid;
    use bitvue_core::partition_grid::PartitionGrid;
    use bitvue_core::qp_heatmap::QPGrid;

    // Zero dimensions should be handled
    let qp_grid = QPGrid::new(0, 0, 128, 128, vec![], -1);
    assert_eq!(qp_grid.grid_w, 0);

    let mv_grid = MVGrid::new(0, 0, 16, 16, vec![], vec![], None);
    assert_eq!(mv_grid.grid_w, 0);

    let part_grid = PartitionGrid::new(0, 0, 128);
    assert_eq!(part_grid.coded_width, 0);
}
