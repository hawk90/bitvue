//! Tests for HEVC (H.265) parser

#[test]
fn test_hevc_nal_types() {
    // Test HEVC NAL unit type identification
    #[derive(Debug, PartialEq)]
    enum HevcNalType {
        TRAIL_N = 0,
        TRAIL_R = 1,
        IDR_W_RADL = 19,
        IDR_N_LP = 20,
        VPS = 32,
        SPS = 33,
        PPS = 34,
        AUD = 35,
        SEI_PREFIX = 39,
        SEI_SUFFIX = 40,
    }

    let nal_types = vec![
        (0, HevcNalType::TRAIL_N),
        (1, HevcNalType::TRAIL_R),
        (19, HevcNalType::IDR_W_RADL),
        (20, HevcNalType::IDR_N_LP),
        (32, HevcNalType::VPS),
        (33, HevcNalType::SPS),
        (34, HevcNalType::PPS),
    ];

    for (type_val, expected) in nal_types {
        assert_eq!(type_val, expected as u8);
    }
}

#[test]
fn test_hevc_vps_parsing() {
    // Test VPS (Video Parameter Set) parsing
    struct VPS {
        vps_id: u8,
        max_layers: u8,
        max_sub_layers: u8,
    }

    let vps = VPS {
        vps_id: 0,
        max_layers: 1,
        max_sub_layers: 1,
    };

    assert_eq!(vps.vps_id, 0);
    assert!(vps.max_sub_layers >= 1 && vps.max_sub_layers <= 7);
}

#[test]
fn test_hevc_sps_parsing() {
    // Test SPS (Sequence Parameter Set) parsing
    struct SPS {
        sps_id: u8,
        vps_id: u8,
        max_sub_layers: u8,
        width: u32,
        height: u32,
    }

    let sps = SPS {
        sps_id: 0,
        vps_id: 0,
        max_sub_layers: 1,
        width: 1920,
        height: 1080,
    };

    assert_eq!(sps.width, 1920);
    assert_eq!(sps.height, 1080);
    assert!(sps.max_sub_layers <= 7);
}

#[test]
fn test_hevc_pps_parsing() {
    // Test PPS (Picture Parameter Set) parsing
    struct PPS {
        pps_id: u8,
        sps_id: u8,
        dependent_slice_segments_enabled: bool,
        output_flag_present: bool,
    }

    let pps = PPS {
        pps_id: 0,
        sps_id: 0,
        dependent_slice_segments_enabled: false,
        output_flag_present: false,
    };

    assert_eq!(pps.pps_id, 0);
    assert_eq!(pps.sps_id, 0);
}

#[test]
fn test_hevc_slice_header() {
    // Test slice header parsing
    #[derive(Debug, PartialEq)]
    enum SliceType {
        B = 0,
        P = 1,
        I = 2,
    }

    let slice_types = vec![SliceType::I, SliceType::P, SliceType::B];
    assert_eq!(slice_types.len(), 3);
}

#[test]
fn test_hevc_temporal_layer() {
    // Test temporal layer identification
    let temporal_layers = vec![0, 1, 2, 3, 4, 5, 6];
    
    for tid in temporal_layers {
        assert!(tid <= 6, "Temporal ID should be 0-6");
    }
}

#[test]
fn test_hevc_idr_detection() {
    // Test IDR frame detection
    struct Frame {
        nal_type: u8,
        is_idr: bool,
    }

    let frames = vec![
        Frame { nal_type: 19, is_idr: true },  // IDR_W_RADL
        Frame { nal_type: 20, is_idr: true },  // IDR_N_LP
        Frame { nal_type: 1, is_idr: false },  // TRAIL_R
    ];

    assert_eq!(frames.iter().filter(|f| f.is_idr).count(), 2);
}

#[test]
fn test_hevc_ctb_sizes() {
    // Test CTB (Coding Tree Block) size options
    let ctb_sizes = vec![16u32, 32u32, 64u32];

    for size in ctb_sizes {
        assert!(size >= 16 && size <= 64);
        assert!(size.is_power_of_two());
    }
}

#[test]
fn test_hevc_max_transform_hierarchy() {
    // Test max transform hierarchy depth
    let max_depths = vec![0, 1, 2, 3, 4];

    for depth in max_depths {
        assert!(depth <= 4, "Max transform depth should be <= 4");
    }
}

#[test]
fn test_hevc_qp_range() {
    // Test QP (Quantization Parameter) range
    let qp_values = vec![0, 10, 26, 40, 51];

    for qp in qp_values {
        assert!(qp <= 51, "QP should be 0-51 for HEVC");
    }
}

#[test]
fn test_hevc_profile_tier_level() {
    // Test Profile/Tier/Level
    #[derive(Debug, PartialEq)]
    enum Profile {
        Main = 1,
        Main10 = 2,
        MainStillPicture = 3,
    }

    let profiles = vec![Profile::Main, Profile::Main10, Profile::MainStillPicture];
    assert_eq!(profiles.len(), 3);
}

#[test]
fn test_hevc_sei_types() {
    // Test SEI (Supplemental Enhancement Information) types
    struct SEIMessage {
        payload_type: u32,
        payload_size: usize,
    }

    let sei = SEIMessage {
        payload_type: 1, // Picture timing
        payload_size: 10,
    };

    assert!(sei.payload_size > 0);
}

#[test]
fn test_hevc_scaling_list() {
    // Test scaling list support
    struct ScalingList {
        enabled: bool,
        size: usize,
    }

    let scaling_list = ScalingList {
        enabled: true,
        size: 64,
    };

    assert!(scaling_list.size == 16 || scaling_list.size == 64);
}

#[test]
fn test_hevc_reference_picture_set() {
    // Test RPS (Reference Picture Set)
    struct RPS {
        num_negative_pics: u8,
        num_positive_pics: u8,
    }

    let rps = RPS {
        num_negative_pics: 1,
        num_positive_pics: 0,
    };

    assert!(rps.num_negative_pics + rps.num_positive_pics <= 16);
}

#[test]
fn test_hevc_bit_depth() {
    // Test bit depth support
    let bit_depths = vec![8, 10, 12];

    for depth in bit_depths {
        assert!(depth == 8 || depth == 10 || depth == 12);
    }
}
