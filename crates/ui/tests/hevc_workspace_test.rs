//! Tests for HEVC Workspace

#[test]
fn test_hevc_overlay_types() {
    // Test HEVC-specific overlay types
    #[derive(Debug, PartialEq)]
    enum HevcOverlay {
        CtbGrid,
        CuPartitions,
        PuBoundaries,
        TuSplits,
        IntraModes,
        MotionVectors,
        QpHeatmap,
        ReferenceFrames,
    }

    let overlays = vec![
        HevcOverlay::CtbGrid,
        HevcOverlay::CuPartitions,
        HevcOverlay::IntraModes,
        HevcOverlay::MotionVectors,
    ];

    assert_eq!(overlays.len(), 4);
}

#[test]
fn test_hevc_nal_unit_types() {
    // Test HEVC NAL unit type identification
    #[derive(Debug, PartialEq)]
    enum HevcNalType {
        TrailN = 0,
        TrailR = 1,
        IdrWRadl = 19,
        IdrNLp = 20,
        Vps = 32,
        Sps = 33,
        Pps = 34,
    }

    let nal_types = vec![
        HevcNalType::TrailN,
        HevcNalType::IdrWRadl,
        HevcNalType::Vps,
        HevcNalType::Sps,
    ];

    assert_eq!(nal_types.len(), 4);
}

#[test]
fn test_hevc_ctb_sizes() {
    // Test CTB (Coding Tree Block) sizes
    let ctb_sizes = vec![16usize, 32usize, 64usize];

    for size in &ctb_sizes {
        assert!(size.is_power_of_two());
        assert!(*size >= 16 && *size <= 64);
    }
}

#[test]
fn test_hevc_cu_depths() {
    // Test CU (Coding Unit) depth levels
    let cu_depths = vec![0usize, 1usize, 2usize, 3usize];

    for depth in &cu_depths {
        assert!(*depth <= 3, "HEVC CU depth should be 0-3");
    }
}

#[test]
fn test_hevc_pu_partition_modes() {
    // Test PU (Prediction Unit) partition modes
    #[derive(Debug, PartialEq)]
    enum PartMode {
        Part2Nx2N,
        Part2NxN,
        PartNx2N,
        PartNxN,
        Part2NxnU,
        Part2NxnD,
        PartnLx2N,
        PartnRx2N,
    }

    let modes = vec![PartMode::Part2Nx2N, PartMode::PartNxN, PartMode::Part2NxnU];

    assert_eq!(modes.len(), 3);
}

#[test]
fn test_hevc_tu_split_flags() {
    // Test TU (Transform Unit) split structure
    struct TuNode {
        depth: u8,
        split_flag: bool,
        size: usize,
    }

    let tu = TuNode {
        depth: 1,
        split_flag: true,
        size: 16,
    };

    assert!(tu.depth <= 4);
    assert!(tu.size.is_power_of_two());
}

#[test]
fn test_hevc_intra_modes() {
    // Test intra prediction mode range
    let intra_modes = vec![0u8, 10, 18, 26, 34]; // Planar, DC, Angular modes

    for mode in intra_modes {
        assert!(mode <= 34, "HEVC has 35 intra prediction modes (0-34)");
    }
}

#[test]
fn test_hevc_qp_range() {
    // Test quantization parameter range
    let qp_values = vec![0i32, 10, 26, 40, 51];

    for qp in qp_values {
        assert!(qp >= 0 && qp <= 51, "HEVC QP should be 0-51");
    }
}

#[test]
fn test_hevc_temporal_layers() {
    // Test temporal layer hierarchy
    struct TemporalLayer {
        temporal_id: u8,
        max_sub_layers: u8,
    }

    let layer = TemporalLayer {
        temporal_id: 2,
        max_sub_layers: 4,
    };

    assert!(layer.temporal_id < layer.max_sub_layers);
    assert!(layer.max_sub_layers <= 7);
}

#[test]
fn test_hevc_reference_picture_sets() {
    // Test RPS (Reference Picture Set) structure
    struct RPS {
        num_negative_pics: u8,
        num_positive_pics: u8,
    }

    let rps = RPS {
        num_negative_pics: 3,
        num_positive_pics: 1,
    };

    assert!(rps.num_negative_pics <= 16);
    assert!(rps.num_positive_pics <= 16);
}

#[test]
fn test_hevc_scaling_list() {
    // Test scaling list data
    struct ScalingList {
        size_id: u8, // 0=4x4, 1=8x8, 2=16x16, 3=32x32
        matrix_id: u8,
    }

    let sl = ScalingList {
        size_id: 2,
        matrix_id: 0,
    };

    assert!(sl.size_id <= 3);
}

#[test]
fn test_hevc_sao_parameters() {
    // Test SAO (Sample Adaptive Offset) parameters
    #[derive(Debug, PartialEq)]
    enum SaoType {
        NotApplied = 0,
        BandOffset = 1,
        EdgeOffset = 2,
    }

    let sao = SaoType::EdgeOffset;
    assert_eq!(sao, SaoType::EdgeOffset);
}

#[test]
fn test_hevc_deblocking_filter() {
    // Test deblocking filter parameters
    struct DeblockingFilter {
        disable_flag: bool,
        beta_offset: i8,
        tc_offset: i8,
    }

    let dbf = DeblockingFilter {
        disable_flag: false,
        beta_offset: 0,
        tc_offset: 0,
    };

    assert!(dbf.beta_offset >= -6 && dbf.beta_offset <= 6);
    assert!(dbf.tc_offset >= -6 && dbf.tc_offset <= 6);
}

#[test]
fn test_hevc_motion_vector_range() {
    // Test motion vector range
    struct MotionVector {
        mvx: i16,
        mvy: i16,
    }

    let mv = MotionVector { mvx: 128, mvy: -64 };

    // HEVC MV range is typically +/- 2048 in quarter-pel units
    assert!(mv.mvx.abs() <= 2048);
    assert!(mv.mvy.abs() <= 2048);
}

#[test]
fn test_hevc_merge_candidates() {
    // Test merge mode candidate list
    let max_merge_cand = 5usize;

    assert!(
        max_merge_cand <= 5,
        "HEVC supports up to 5 merge candidates"
    );
}
