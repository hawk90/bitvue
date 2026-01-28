//! HEVC Overlay Extraction Tests
//!
//! Comprehensive tests for HEVC overlay data extraction.

use bitvue_core::{partition_grid::PartitionType, BlockMode};
use bitvue_hevc::overlay_extraction;
use bitvue_hevc::sps::{ChromaFormat, Profile, ProfileTierLevel, Sps};

fn create_minimal_sps() -> Sps {
    Sps {
        sps_video_parameter_set_id: 0,
        sps_max_sub_layers_minus1: 0,
        sps_temporal_id_nesting_flag: false,
        profile_tier_level: ProfileTierLevel {
            general_profile_space: 0,
            general_tier_flag: false,
            general_profile_idc: Profile::Main,
            general_profile_compatibility_flags: 0,
            general_progressive_source_flag: true,
            general_interlaced_source_flag: false,
            general_non_packed_constraint_flag: true,
            general_frame_only_constraint_flag: true,
            general_level_idc: 120,
        },
        sps_seq_parameter_set_id: 0,
        chroma_format_idc: ChromaFormat::Chroma420,
        separate_colour_plane_flag: false,
        pic_width_in_luma_samples: 1920,
        pic_height_in_luma_samples: 1080,
        conformance_window_flag: false,
        conf_win_left_offset: 0,
        conf_win_right_offset: 0,
        conf_win_top_offset: 0,
        conf_win_bottom_offset: 0,
        bit_depth_luma_minus8: 0,
        bit_depth_chroma_minus8: 0,
        log2_max_pic_order_cnt_lsb_minus4: 0,
        sps_sub_layer_ordering_info_present_flag: false,
        sps_max_dec_pic_buffering_minus1: vec![0],
        sps_max_num_reorder_pics: vec![0],
        sps_max_latency_increase_plus1: vec![0],
        log2_min_luma_coding_block_size_minus3: 0,
        log2_diff_max_min_luma_coding_block_size: 0,
        log2_min_luma_transform_block_size_minus2: 0,
        log2_diff_max_min_luma_transform_block_size: 0,
        max_transform_hierarchy_depth_inter: 0,
        max_transform_hierarchy_depth_intra: 0,
        scaling_list_enabled_flag: false,
        amp_enabled_flag: false,
        sample_adaptive_offset_enabled_flag: false,
        pcm_enabled_flag: false,
        num_short_term_ref_pic_sets: 0,
        long_term_ref_pics_present_flag: false,
        num_long_term_ref_pics_sps: 0,
        sps_temporal_mvp_enabled_flag: false,
        strong_intra_smoothing_enabled_flag: false,
        vui_parameters_present_flag: false,
        vui_parameters: None,
    }
}

#[test]
fn test_extract_qp_grid_basic() {
    // Test QP grid extraction with minimal SPS
    let sps = create_minimal_sps();

    let nal_units = [];
    let result = overlay_extraction::extract_qp_grid(&nal_units, &sps, 26);

    assert!(result.is_ok());

    let grid = result.unwrap();
    assert!(grid.grid_w > 0);
}

#[test]
fn test_extract_mv_grid_basic() {
    let sps = create_minimal_sps();

    let nal_units = [];
    let result = overlay_extraction::extract_mv_grid(&nal_units, &sps);

    assert!(result.is_ok());

    let grid = result.unwrap();
    assert!(grid.grid_w > 0);
}

#[test]
fn test_extract_partition_grid_basic() {
    let sps = create_minimal_sps();

    let nal_units = [];
    let result = overlay_extraction::extract_partition_grid(&nal_units, &sps);

    assert!(result.is_ok());

    let grid = result.unwrap();
    assert!(grid.coded_width > 0);
}

#[test]
fn test_ctu_creation() {
    let ctu = bitvue_hevc::overlay_extraction::CodingTreeUnit {
        x: 0,
        y: 0,
        size: 64,
        coding_units: vec![],
    };

    assert_eq!(ctu.x, 0);
    assert_eq!(ctu.y, 0);
    assert_eq!(ctu.size, 64);
}

#[test]
fn test_pred_mode_variants() {
    let modes = vec![
        overlay_extraction::PredMode::Intra,
        overlay_extraction::PredMode::Inter,
        overlay_extraction::PredMode::Skip,
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
fn test_block_mode_variants() {
    let modes = vec![BlockMode::Intra, BlockMode::Inter, BlockMode::Skip];

    for mode in modes {
        let _ = format!("{:?}", mode);
    }
}

#[test]
fn test_partition_type_variants() {
    let types = vec![
        PartitionType::None,
        PartitionType::Horz,
        PartitionType::Vert,
        PartitionType::Split,
    ];

    for ptype in types {
        let _ = format!("{:?}", ptype);
    }
}

#[test]
fn test_qp_grid_with_keyframe() {
    let sps = create_minimal_sps();

    let nal_units = [];
    let result = overlay_extraction::extract_qp_grid(&nal_units, &sps, 80);
    assert!(result.is_ok());
}

#[test]
fn test_qp_grid_with_interframe() {
    let sps = create_minimal_sps();

    let nal_units = [];
    let result = overlay_extraction::extract_qp_grid(&nal_units, &sps, 120);
    assert!(result.is_ok());
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
        let mut sps = create_minimal_sps();
        sps.pic_width_in_luma_samples = width;
        sps.pic_height_in_luma_samples = height;

        let nal_units = [];

        let qp_result = overlay_extraction::extract_qp_grid(&nal_units, &sps, 100);
        assert!(qp_result.is_ok());

        let mv_result = overlay_extraction::extract_mv_grid(&nal_units, &sps);
        assert!(mv_result.is_ok());

        let part_result = overlay_extraction::extract_partition_grid(&nal_units, &sps);
        assert!(part_result.is_ok());
    }
}

#[test]
fn test_qp_range() {
    let qp_values = vec![0i16, 50, 100, 150, 200, 255];

    for base_qp in qp_values {
        let sps = create_minimal_sps();
        let nal_units = [];

        let result = overlay_extraction::extract_qp_grid(&nal_units, &sps, base_qp);
        assert!(result.is_ok());
    }
}

#[test]
fn test_ctu_with_motion_vector() {
    let mv = overlay_extraction::MotionVector::new(5, -3);
    let ctu = bitvue_hevc::overlay_extraction::CodingTreeUnit {
        x: 64,
        y: 64,
        size: 64,
        coding_units: vec![],
    };

    assert_eq!(ctu.size, 64);
}

#[test]
fn test_ctu_positions() {
    let positions = vec![(0u32, 0u32), (64, 0), (0, 64), (64, 64), (128, 128)];

    for (x, y) in positions {
        let ctu = bitvue_hevc::overlay_extraction::CodingTreeUnit {
            x,
            y,
            size: 64,
            coding_units: vec![],
        };

        assert_eq!(ctu.x, x);
        assert_eq!(ctu.y, y);
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
        let _ = format!("{:?}", partition);
    }
}

#[test]
fn test_super_block_modes() {
    let modes = vec![BlockMode::Intra, BlockMode::Inter, BlockMode::Skip];

    for mode in modes {
        let _ = format!("{:?}", mode);
    }
}

#[test]
fn test_grid_dimensions() {
    use bitvue_core::mv_overlay::{MVGrid, MotionVector};
    use bitvue_core::qp_heatmap::QPGrid;

    // Test various dimensions
    let dimensions = vec![(640, 480), (1280, 720), (1920, 1080)];

    for (width, height) in dimensions {
        let ctu_size = 64u32;
        let grid_w = (width + ctu_size - 1) / ctu_size;
        let grid_h = (height + ctu_size - 1) / ctu_size;

        let qp_grid = QPGrid::new(
            grid_w,
            grid_h,
            ctu_size,
            ctu_size,
            vec![26; (grid_w * grid_h) as usize],
            -1,
        );
        assert_eq!(qp_grid.grid_w, grid_w);

        let mv_l0 = vec![MotionVector::ZERO; (grid_w * grid_h) as usize];
        let mv_l1 = vec![MotionVector::MISSING; (grid_w * grid_h) as usize];
        let mv_grid = MVGrid::new(width, height, ctu_size, ctu_size, mv_l0, mv_l1, None);
        assert_eq!(mv_grid.grid_w, grid_w);
    }
}

#[test]
fn test_grid_with_zero_dimensions() {
    use bitvue_core::mv_overlay::{MVGrid, MotionVector};
    use bitvue_core::qp_heatmap::QPGrid;

    // Zero dimensions should be handled
    let qp_grid = QPGrid::new(0, 0, 64, 64, vec![], -1);
    assert_eq!(qp_grid.grid_w, 0);

    let mv_grid = MVGrid::new(0, 0, 64, 64, vec![], vec![], None);
    assert_eq!(mv_grid.grid_w, 0);
}

#[test]
fn test_large_grid() {
    use bitvue_core::qp_heatmap::QPGrid;

    // Test 4K resolution
    let width: u32 = 3840 / 64; // 60 CTUs
    let height: u32 = 2160 / 64; // 33 CTUs
    let qp_values = vec![26i16; (width * height) as usize];

    let grid = QPGrid::new(width, height, 64, 64, qp_values, -1);

    assert_eq!(grid.grid_w, 60);
    assert_eq!(grid.grid_h, 33);
}

#[test]
fn test_mv_grid_large() {
    use bitvue_core::mv_overlay::{MVGrid, MotionVector};

    // Test 4K resolution with dimensions that divide evenly
    let coded_width = 3840u32;
    let coded_height = 2160u32;
    let block_w = 64u32;
    let block_h = 64u32;

    let grid_w = (coded_width + block_w - 1) / block_w;
    let grid_h = (coded_height + block_h - 1) / block_h;

    let mv_l0 = vec![MotionVector::ZERO; (grid_w * grid_h) as usize];
    let mv_l1 = vec![MotionVector::MISSING; (grid_w * grid_h) as usize];

    let grid = MVGrid::new(
        coded_width,
        coded_height,
        block_w,
        block_h,
        mv_l0,
        mv_l1,
        None,
    );
    assert_eq!(grid.grid_w, grid_w);
    assert_eq!(grid.grid_h, grid_h);
}

#[test]
fn test_qp_values_valid_range() {
    let qp_values = vec![0i16, 10, 20, 30, 40, 50, 100, 150, 200, 255];

    for qp in qp_values {
        let sps = create_minimal_sps();
        let nal_units = [];

        let result = overlay_extraction::extract_qp_grid(&nal_units, &sps, qp);
        assert!(result.is_ok());
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
fn test_ctu_coverage() {
    let width = 1920;
    let height = 1080;
    let ctu_size = 64;

    let ctu_cols = (width + ctu_size - 1) / ctu_size;
    let ctu_rows = (height + ctu_size - 1) / ctu_size;

    assert!(ctu_cols > 0);
    assert!(ctu_rows > 0);
    assert_eq!(ctu_cols, 30);
    assert_eq!(ctu_rows, 17);
}

#[test]
fn test_ctu_coverage_128() {
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
    use bitvue_core::mv_overlay::{MVGrid, MotionVector};
    use bitvue_core::partition_grid::PartitionGrid;
    use bitvue_core::qp_heatmap::QPGrid;

    let width = 1920;
    let height = 1080;

    let qp_grid = QPGrid::new(30, 17, 64, 64, vec![26; 510], -1);
    assert_eq!(qp_grid.grid_w, 30);

    let mv_l0 = vec![MotionVector::ZERO; 510];
    let mv_l1 = vec![MotionVector::MISSING; 510];
    let mv_grid = MVGrid::new(width, height, 64, 64, mv_l0, mv_l1, None);
    assert_eq!(mv_grid.grid_w, 30);

    let part_grid = PartitionGrid::new(width, height, 64);
    assert_eq!(part_grid.coded_width, width);
}

#[test]
fn test_partition_type_from_u8() {
    use bitvue_core::partition_grid::PartitionType;

    // Test conversion from u8
    assert_eq!(PartitionType::from(0u8), PartitionType::None);
    assert_eq!(PartitionType::from(1u8), PartitionType::Horz);
    assert_eq!(PartitionType::from(2u8), PartitionType::Vert);
    assert_eq!(PartitionType::from(3u8), PartitionType::Split);

    // Invalid value should default to None
    assert_eq!(PartitionType::from(255u8), PartitionType::None);
}

#[test]
fn test_cu_size_variants() {
    let sizes = vec![4u8, 8, 16, 32, 64];

    for size in sizes {
        let cu = bitvue_hevc::overlay_extraction::CodingUnit {
            x: 0,
            y: 0,
            size,
            pred_mode: overlay_extraction::PredMode::Intra,
            part_mode: overlay_extraction::PartMode::Part2Nx2N,
            intra_mode: None,
            qp: 26,
            mv_l0: None,
            mv_l1: None,
            ref_idx_l0: None,
            ref_idx_l1: None,
            transform_size: 4,
            depth: 0,
        };

        assert_eq!(cu.size, size);
    }
}

#[test]
fn test_cu_depth_values() {
    let depths = vec![0u8, 1, 2, 3, 4];

    for depth in depths {
        let cu = bitvue_hevc::overlay_extraction::CodingUnit {
            x: 0,
            y: 0,
            size: 64,
            pred_mode: overlay_extraction::PredMode::Intra,
            part_mode: overlay_extraction::PartMode::Part2Nx2N,
            intra_mode: None,
            qp: 26,
            mv_l0: None,
            mv_l1: None,
            ref_idx_l0: None,
            ref_idx_l1: None,
            transform_size: 4,
            depth,
        };

        assert_eq!(cu.depth, depth);
    }
}

#[test]
fn test_cu_positions() {
    let positions = vec![(0u32, 0u32), (64, 0), (0, 64), (64, 64), (128, 128)];

    for (x, y) in positions {
        let cu = bitvue_hevc::overlay_extraction::CodingUnit {
            x,
            y,
            size: 64,
            pred_mode: overlay_extraction::PredMode::Intra,
            part_mode: overlay_extraction::PartMode::Part2Nx2N,
            intra_mode: None,
            qp: 26,
            mv_l0: None,
            mv_l1: None,
            ref_idx_l0: None,
            ref_idx_l1: None,
            transform_size: 4,
            depth: 0,
        };

        assert_eq!(cu.x, x);
        assert_eq!(cu.y, y);
    }
}

#[test]
fn test_cu_with_motion_vector() {
    let mv = overlay_extraction::MotionVector::new(5, -3);

    let cu = bitvue_hevc::overlay_extraction::CodingUnit {
        x: 0,
        y: 0,
        size: 64,
        pred_mode: overlay_extraction::PredMode::Inter,
        part_mode: overlay_extraction::PartMode::Part2Nx2N,
        intra_mode: None,
        qp: 26,
        mv_l0: Some(mv),
        mv_l1: None,
        ref_idx_l0: Some(0),
        ref_idx_l1: None,
        transform_size: 8,
        depth: 0,
    };

    assert!(cu.mv_l0.is_some());
    assert_eq!(cu.mv_l0.unwrap().x, 5);
}

#[test]
fn test_cu_skip_mode() {
    let cu = bitvue_hevc::overlay_extraction::CodingUnit {
        x: 0,
        y: 0,
        size: 64,
        pred_mode: overlay_extraction::PredMode::Skip,
        part_mode: overlay_extraction::PartMode::Part2Nx2N,
        intra_mode: None,
        qp: 26,
        mv_l0: None,
        mv_l1: None,
        ref_idx_l0: None,
        ref_idx_l1: None,
        transform_size: 4,
        depth: 0,
    };

    assert_eq!(cu.pred_mode, overlay_extraction::PredMode::Skip);
}

#[test]
fn test_cu_intra_mode() {
    let intra_mode = overlay_extraction::IntraMode::Dc;
    let cu = bitvue_hevc::overlay_extraction::CodingUnit {
        x: 0,
        y: 0,
        size: 32,
        pred_mode: overlay_extraction::PredMode::Intra,
        part_mode: overlay_extraction::PartMode::NxN,
        intra_mode: Some(intra_mode),
        qp: 26,
        mv_l0: None,
        mv_l1: None,
        ref_idx_l0: None,
        ref_idx_l1: None,
        transform_size: 4,
        depth: 1,
    };

    assert_eq!(cu.pred_mode, overlay_extraction::PredMode::Intra);
    assert!(cu.intra_mode.is_some());
}

#[test]
fn test_ctu_add_cu() {
    let mut ctu = bitvue_hevc::overlay_extraction::CodingTreeUnit {
        x: 0,
        y: 0,
        size: 64,
        coding_units: vec![],
    };

    let cu = bitvue_hevc::overlay_extraction::CodingUnit {
        x: 0,
        y: 0,
        size: 64,
        pred_mode: overlay_extraction::PredMode::Intra,
        part_mode: overlay_extraction::PartMode::Part2Nx2N,
        intra_mode: None,
        qp: 26,
        mv_l0: None,
        mv_l1: None,
        ref_idx_l0: None,
        ref_idx_l1: None,
        transform_size: 4,
        depth: 0,
    };

    ctu.coding_units.push(cu);

    assert_eq!(ctu.coding_units.len(), 1);
}
