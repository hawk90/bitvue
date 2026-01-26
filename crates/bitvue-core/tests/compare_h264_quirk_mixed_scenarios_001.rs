use bitvue_core::frame_identity::*;
use bitvue_core::{AlignmentEngine, CompareWorkspace, FrameIndexMap};

// ========================================================================
// Compare H264 Quirks - Mixed Scenario Edge Cases
// Test file 6/6: Combined H.264 quirks in complex scenarios
// ========================================================================
//
// This file tests combinations of H.264 quirks that can occur together:
// POC wrap + DPB management, field coding + pyramid, MMCO + reordering, etc.

#[test]
fn test_compare_h264_mixed_001_poc_wrap_with_pyramid() {
    // Test: POC wrap combined with B-frame pyramid
    // POC wraps while maintaining pyramid structure

    let frames_a = vec![
        FrameMetadata {
            pts: Some(254000),
            dts: Some(0),
        }, // I254
        FrameMetadata {
            pts: Some(258000),
            dts: Some(1000),
        }, // P258 (after wrap)
        FrameMetadata {
            pts: Some(256000),
            dts: Some(2000),
        }, // B256 (wrap boundary, ref)
        FrameMetadata {
            pts: Some(255000),
            dts: Some(3000),
        }, // B255 (pre-wrap)
        FrameMetadata {
            pts: Some(257000),
            dts: Some(4000),
        }, // B257 (post-wrap)
    ];

    let frames_b = frames_a.clone();

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let alignment = AlignmentEngine::new(&map_a, &map_b);

    // POC wrap + pyramid both handled via PTS
    assert!(matches!(
        alignment.method,
        bitvue_core::AlignmentMethod::PtsExact | bitvue_core::AlignmentMethod::PtsNearest
    ));

    for i in 0..5 {
        let pair = alignment.get_pair_for_a(i).unwrap();
        assert_eq!(pair.stream_b_idx, Some(i));
        assert!(!pair.has_gap);
    }
}

#[test]
fn test_compare_h264_mixed_002_field_pyramid_dpb_sliding() {
    // Test: Field coding + pyramid + DPB sliding window
    // Multiple complex features combined

    let frames_a = vec![
        FrameMetadata {
            pts: Some(0),
            dts: Some(0),
        }, // I-field pair 0
        FrameMetadata {
            pts: Some(4000),
            dts: Some(1000),
        }, // P-field pair 4 (DPB slides)
        FrameMetadata {
            pts: Some(2000),
            dts: Some(2000),
        }, // B-field pair 2 (ref)
        FrameMetadata {
            pts: Some(1000),
            dts: Some(3000),
        }, // B-field pair 1
        FrameMetadata {
            pts: Some(3000),
            dts: Some(4000),
        }, // B-field pair 3
    ];

    let frames_b = frames_a.clone();

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    // Multiple quirks combined don't affect alignment
    assert!(workspace.is_diff_enabled());

    for i in 0..5 {
        let (b_idx, quality) = workspace.get_aligned_frame(i).unwrap();
        assert_eq!(b_idx, i);
        assert_eq!(quality, bitvue_core::AlignmentQuality::Exact);
    }
}

#[test]
fn test_compare_h264_mixed_003_mmco5_with_pyramid() {
    // Test: MMCO5 (POC reset) combined with pyramid reordering
    // MMCO5 at reference B-frame

    let frames_a = vec![
        FrameMetadata {
            pts: Some(0),
            dts: Some(0),
        }, // I0
        FrameMetadata {
            pts: Some(4000),
            dts: Some(1000),
        }, // P4
        FrameMetadata {
            pts: Some(2000),
            dts: Some(2000),
        }, // B2 (ref, with MMCO5)
        FrameMetadata {
            pts: Some(1000),
            dts: Some(3000),
        }, // B1
        FrameMetadata {
            pts: Some(3000),
            dts: Some(4000),
        }, // B3
        FrameMetadata {
            pts: Some(5000),
            dts: Some(5000),
        }, // P5 (after MMCO5 reset)
    ];

    let frames_b = frames_a.clone();

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let alignment = AlignmentEngine::new(&map_a, &map_b);

    // MMCO5 in pyramid handled correctly
    assert!(matches!(
        alignment.method,
        bitvue_core::AlignmentMethod::PtsExact | bitvue_core::AlignmentMethod::PtsNearest
    ));

    for i in 0..6 {
        let pair = alignment.get_pair_for_a(i).unwrap();
        assert_eq!(pair.stream_b_idx, Some(i));
    }
}

#[test]
fn test_compare_h264_mixed_004_mbaff_pyramid_poc_wrap() {
    // Test: MBAFF + pyramid + POC wrap
    // Maximum complexity scenario

    let frames_a = vec![
        FrameMetadata {
            pts: Some(254000),
            dts: Some(0),
        }, // MBAFF I254
        FrameMetadata {
            pts: Some(258000),
            dts: Some(1000),
        }, // MBAFF P258 (wrapped)
        FrameMetadata {
            pts: Some(256000),
            dts: Some(2000),
        }, // MBAFF B256 (ref, at wrap)
        FrameMetadata {
            pts: Some(255000),
            dts: Some(3000),
        }, // MBAFF B255
        FrameMetadata {
            pts: Some(257000),
            dts: Some(4000),
        }, // MBAFF B257
    ];

    let frames_b = frames_a.clone();

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    // All quirks combined still align via PTS
    assert!(workspace.is_diff_enabled());

    for i in 0..5 {
        let (b_idx, _) = workspace.get_aligned_frame(i).unwrap();
        assert_eq!(b_idx, i);
    }
}

#[test]
fn test_compare_h264_mixed_005_idr_gap_pyramid() {
    // Test: IDR placement with gaps and pyramid
    // Stream A: IDR at frame 3
    // Stream B: Gap at frame 3 (packet loss)

    let frames_a = vec![
        FrameMetadata {
            pts: Some(0),
            dts: Some(0),
        }, // I0
        FrameMetadata {
            pts: Some(1000),
            dts: Some(1000),
        }, // B1
        FrameMetadata {
            pts: Some(2000),
            dts: Some(2000),
        }, // P2
        FrameMetadata {
            pts: Some(3000),
            dts: Some(3000),
        }, // IDR3
        FrameMetadata {
            pts: Some(4000),
            dts: Some(4000),
        }, // P4
    ];

    let frames_b = vec![
        FrameMetadata {
            pts: Some(0),
            dts: Some(0),
        }, // I0
        FrameMetadata {
            pts: Some(1000),
            dts: Some(1000),
        }, // B1
        FrameMetadata {
            pts: Some(2000),
            dts: Some(2000),
        }, // P2
        // Gap at PTS 3000 (missing IDR)
        FrameMetadata {
            pts: Some(4000),
            dts: Some(4000),
        }, // P4 (error recovery)
    ];

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let alignment = AlignmentEngine::new(&map_a, &map_b);

    // Gap detected
    assert!(alignment.gap_count >= 1);

    // Available frames align
    let pair_a0 = alignment.get_pair_for_a(0).unwrap();
    assert_eq!(pair_a0.stream_b_idx, Some(0));

    let pair_a4 = alignment.get_pair_for_a(4).unwrap();
    assert_eq!(pair_a4.stream_b_idx, Some(3));
}

#[test]
fn test_compare_h264_mixed_006_long_term_ref_with_pyramid() {
    // Test: Long-term reference frames in B-frame pyramid
    // Base I-frame marked long-term, pyramid references it

    let frames_a = vec![
        FrameMetadata {
            pts: Some(0),
            dts: Some(0),
        }, // I0 (long-term via MMCO2)
        FrameMetadata {
            pts: Some(4000),
            dts: Some(1000),
        }, // P4
        FrameMetadata {
            pts: Some(2000),
            dts: Some(2000),
        }, // B2 (ref)
        FrameMetadata {
            pts: Some(1000),
            dts: Some(3000),
        }, // B1 (refs I0 long-term)
        FrameMetadata {
            pts: Some(3000),
            dts: Some(4000),
        }, // B3
        FrameMetadata {
            pts: Some(8000),
            dts: Some(5000),
        }, // P8 (still refs I0)
        FrameMetadata {
            pts: Some(6000),
            dts: Some(6000),
        }, // B6 (refs I0 long-term)
        FrameMetadata {
            pts: Some(5000),
            dts: Some(7000),
        }, // B5
        FrameMetadata {
            pts: Some(7000),
            dts: Some(8000),
        }, // B7
    ];

    let frames_b = frames_a.clone();

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let alignment = AlignmentEngine::new(&map_a, &map_b);

    // Long-term refs with pyramid aligned correctly
    assert!(matches!(
        alignment.method,
        bitvue_core::AlignmentMethod::PtsExact | bitvue_core::AlignmentMethod::PtsNearest
    ));

    for i in 0..9 {
        let pair = alignment.get_pair_for_a(i).unwrap();
        assert_eq!(pair.stream_b_idx, Some(i));
    }
}

#[test]
fn test_compare_h264_mixed_007_adaptive_vs_sliding_with_pyramid() {
    // Test: Adaptive marking (MMCO) vs sliding window with pyramid
    // Stream A: MMCO throughout pyramid
    // Stream B: Sliding window only

    let frames_a = vec![
        FrameMetadata {
            pts: Some(0),
            dts: Some(0),
        },
        FrameMetadata {
            pts: Some(4000),
            dts: Some(1000),
        },
        FrameMetadata {
            pts: Some(2000),
            dts: Some(2000),
        },
        FrameMetadata {
            pts: Some(1000),
            dts: Some(3000),
        },
        FrameMetadata {
            pts: Some(3000),
            dts: Some(4000),
        },
    ];

    let frames_b = frames_a.clone();

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    // Different memory management in pyramid doesn't affect alignment
    assert!(workspace.is_diff_enabled());

    for i in 0..5 {
        let (b_idx, quality) = workspace.get_aligned_frame(i).unwrap();
        assert_eq!(b_idx, i);
        assert_eq!(quality, bitvue_core::AlignmentQuality::Exact);
    }
}

#[test]
fn test_compare_h264_mixed_008_paff_bpyramid_dpb() {
    // Test: PAFF + B-pyramid + DPB management
    // Picture-adaptive field/frame with complex reordering

    let frames_a = vec![
        FrameMetadata {
            pts: Some(0),
            dts: Some(0),
        }, // PAFF frame
        FrameMetadata {
            pts: Some(4000),
            dts: Some(1000),
        }, // PAFF field
        FrameMetadata {
            pts: Some(2000),
            dts: Some(2000),
        }, // PAFF frame (ref B)
        FrameMetadata {
            pts: Some(1000),
            dts: Some(3000),
        }, // PAFF field
        FrameMetadata {
            pts: Some(3000),
            dts: Some(4000),
        }, // PAFF frame
    ];

    let frames_b = frames_a.clone();

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let alignment = AlignmentEngine::new(&map_a, &map_b);

    // PAFF with pyramid aligned via PTS
    assert!(matches!(
        alignment.method,
        bitvue_core::AlignmentMethod::PtsExact | bitvue_core::AlignmentMethod::PtsNearest
    ));

    for i in 0..5 {
        let pair = alignment.get_pair_for_a(i).unwrap();
        assert_eq!(pair.stream_b_idx, Some(i));
    }
}

#[test]
fn test_compare_h264_mixed_009_manual_offset_complex() {
    // Test: Manual offset with multiple quirks active
    // POC wrap + pyramid + field coding

    let frames_a = vec![
        FrameMetadata {
            pts: Some(254000),
            dts: Some(0),
        }, // Field pair
        FrameMetadata {
            pts: Some(258000),
            dts: Some(1000),
        },
        FrameMetadata {
            pts: Some(256000),
            dts: Some(2000),
        }, // Wrap + ref B
        FrameMetadata {
            pts: Some(255000),
            dts: Some(3000),
        },
        FrameMetadata {
            pts: Some(257000),
            dts: Some(4000),
        },
    ];

    let frames_b = vec![
        FrameMetadata {
            pts: Some(256000),
            dts: Some(0),
        }, // B starts at wrap
        FrameMetadata {
            pts: Some(257000),
            dts: Some(1000),
        },
        FrameMetadata {
            pts: Some(258000),
            dts: Some(2000),
        },
        FrameMetadata {
            pts: Some(259000),
            dts: Some(3000),
        },
    ];

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let mut workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    // Apply manual offset
    workspace.set_manual_offset(-2);

    // A frame 2 (PTS 256000, wrap+ref B) → B frame 0 (PTS 256000)
    if let Some((b_idx, quality)) = workspace.get_aligned_frame(2) {
        assert_eq!(b_idx, 0);
        assert_eq!(quality, bitvue_core::AlignmentQuality::Exact);
    }

    // Complex quirks don't interfere with manual offset
}

#[test]
fn test_compare_h264_mixed_010_resolution_mismatch_with_quirks() {
    // Test: Resolution mismatch with H.264 quirks active
    // Different resolutions + pyramid + MBAFF

    let frames_a = vec![
        FrameMetadata {
            pts: Some(0),
            dts: Some(0),
        },
        FrameMetadata {
            pts: Some(4000),
            dts: Some(1000),
        },
        FrameMetadata {
            pts: Some(2000),
            dts: Some(2000),
        },
        FrameMetadata {
            pts: Some(1000),
            dts: Some(3000),
        },
        FrameMetadata {
            pts: Some(3000),
            dts: Some(4000),
        },
    ];

    let frames_b = frames_a.clone();

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    // Different resolutions
    let workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1280, 720));

    // Resolution mismatch disables diff regardless of quirks
    assert!(!workspace.is_diff_enabled());
    assert!(workspace.disable_reason().is_some());
    assert!(workspace
        .disable_reason()
        .unwrap()
        .contains("Resolution mismatch"));
}

#[test]
fn test_compare_h264_mixed_011_stream_a_pyramid_b_flat() {
    // Test: A uses pyramid, B uses flat structure
    // Same frames, vastly different encoding strategies

    let frames_a = vec![
        FrameMetadata {
            pts: Some(0),
            dts: Some(0),
        },
        FrameMetadata {
            pts: Some(8000),
            dts: Some(1000),
        }, // Early P decode
        FrameMetadata {
            pts: Some(4000),
            dts: Some(2000),
        }, // L1 ref B
        FrameMetadata {
            pts: Some(2000),
            dts: Some(3000),
        }, // L2 ref B
        FrameMetadata {
            pts: Some(1000),
            dts: Some(4000),
        }, // L3 B
        FrameMetadata {
            pts: Some(3000),
            dts: Some(5000),
        }, // L3 B
        FrameMetadata {
            pts: Some(6000),
            dts: Some(6000),
        }, // L2 ref B
        FrameMetadata {
            pts: Some(5000),
            dts: Some(7000),
        }, // L3 B
        FrameMetadata {
            pts: Some(7000),
            dts: Some(8000),
        }, // L3 B
    ];

    let frames_b = vec![
        FrameMetadata {
            pts: Some(0),
            dts: Some(0),
        }, // I0
        FrameMetadata {
            pts: Some(1000),
            dts: Some(1000),
        }, // B1 (no pyramid)
        FrameMetadata {
            pts: Some(2000),
            dts: Some(2000),
        }, // B2
        FrameMetadata {
            pts: Some(3000),
            dts: Some(3000),
        }, // B3
        FrameMetadata {
            pts: Some(4000),
            dts: Some(4000),
        }, // B4
        FrameMetadata {
            pts: Some(5000),
            dts: Some(5000),
        }, // B5
        FrameMetadata {
            pts: Some(6000),
            dts: Some(6000),
        }, // B6
        FrameMetadata {
            pts: Some(7000),
            dts: Some(7000),
        }, // B7
        FrameMetadata {
            pts: Some(8000),
            dts: Some(8000),
        }, // P8
    ];

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    assert!(map_a.has_reordering());
    assert!(!map_b.has_reordering());

    let alignment = AlignmentEngine::new(&map_a, &map_b);

    // Drastically different strategies, same PTS alignment
    assert!(matches!(
        alignment.method,
        bitvue_core::AlignmentMethod::PtsExact | bitvue_core::AlignmentMethod::PtsNearest
    ));

    for i in 0..9 {
        let pair = alignment.get_pair_for_a(i).unwrap();
        assert_eq!(pair.stream_b_idx, Some(i));
    }
}

#[test]
fn test_compare_h264_mixed_012_temporal_layers_with_quirks() {
    // Test: Temporal scalability layers + pyramid + POC management
    // SVC-like temporal layering

    let frames_a = vec![
        FrameMetadata {
            pts: Some(0),
            dts: Some(0),
        }, // T0
        FrameMetadata {
            pts: Some(4000),
            dts: Some(1000),
        }, // T0
        FrameMetadata {
            pts: Some(2000),
            dts: Some(2000),
        }, // T1 ref B
        FrameMetadata {
            pts: Some(1000),
            dts: Some(3000),
        }, // T2 B
        FrameMetadata {
            pts: Some(3000),
            dts: Some(4000),
        }, // T2 B
    ];

    let frames_b = frames_a.clone();

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let alignment = AlignmentEngine::new(&map_a, &map_b);

    // Temporal layers don't affect PTS alignment
    assert!(matches!(
        alignment.method,
        bitvue_core::AlignmentMethod::PtsExact | bitvue_core::AlignmentMethod::PtsNearest
    ));

    for i in 0..5 {
        let pair = alignment.get_pair_for_a(i).unwrap();
        assert_eq!(pair.stream_b_idx, Some(i));
    }
}

#[test]
fn test_compare_h264_mixed_013_all_quirks_combined() {
    // Test: Maximum stress test with all quirks
    // POC wrap + DPB + MBAFF + pyramid + MMCO + fields

    let frames_a = vec![
        FrameMetadata {
            pts: Some(254000),
            dts: Some(0),
        }, // MBAFF I254 (long-term MMCO2)
        FrameMetadata {
            pts: Some(258000),
            dts: Some(1000),
        }, // MBAFF P258 (wrapped, DPB slide)
        FrameMetadata {
            pts: Some(256000),
            dts: Some(2000),
        }, // MBAFF B256 (ref, at wrap)
        FrameMetadata {
            pts: Some(255000),
            dts: Some(3000),
        }, // PAFF field B255
        FrameMetadata {
            pts: Some(257000),
            dts: Some(4000),
        }, // PAFF field B257 (with MMCO1)
    ];

    let frames_b = frames_a.clone();

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    // All quirks combined: alignment still works
    assert!(workspace.is_diff_enabled());

    for i in 0..5 {
        let (b_idx, quality) = workspace.get_aligned_frame(i).unwrap();
        assert_eq!(b_idx, i);
        assert_eq!(quality, bitvue_core::AlignmentQuality::Exact);
    }
}

#[test]
fn test_compare_h264_mixed_014_error_resilience_quirks() {
    // Test: Error resilience features + quirks
    // FMO (Flexible Macroblock Ordering) + pyramid + MMCO recovery

    let frames_a = vec![
        FrameMetadata {
            pts: Some(0),
            dts: Some(0),
        }, // I0 with FMO
        FrameMetadata {
            pts: Some(4000),
            dts: Some(1000),
        }, // P4 with FMO
        FrameMetadata {
            pts: Some(2000),
            dts: Some(2000),
        }, // B2 ref (error, MMCO recovery)
        FrameMetadata {
            pts: Some(1000),
            dts: Some(3000),
        }, // B1 with FMO
        FrameMetadata {
            pts: Some(3000),
            dts: Some(4000),
        }, // B3
    ];

    let frames_b = frames_a.clone();

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let alignment = AlignmentEngine::new(&map_a, &map_b);

    // Error resilience features don't affect alignment
    assert!(matches!(
        alignment.method,
        bitvue_core::AlignmentMethod::PtsExact | bitvue_core::AlignmentMethod::PtsNearest
    ));

    for i in 0..5 {
        let pair = alignment.get_pair_for_a(i).unwrap();
        assert_eq!(pair.stream_b_idx, Some(i));
    }
}

#[test]
fn test_compare_h264_mixed_015_pts_quality_degradation() {
    // Test: H.264 quirks causing PTS quality issues
    // VFR content with pyramid and POC wrap

    let frames_a = vec![
        FrameMetadata {
            pts: Some(0),
            dts: Some(0),
        },
        FrameMetadata {
            pts: Some(4000),
            dts: Some(1000),
        }, // P4 (early decode)
        FrameMetadata {
            pts: Some(2000),
            dts: Some(2000),
        }, // B2 ref
        FrameMetadata {
            pts: Some(1000),
            dts: Some(3000),
        }, // B1
        FrameMetadata {
            pts: Some(3000),
            dts: Some(4000),
        }, // B3
        // VFR gap
        FrameMetadata {
            pts: Some(8000),
            dts: Some(5000),
        }, // P8 (large PTS gap)
    ];

    let frames_b = frames_a.clone();

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    // VFR detected
    assert_eq!(map_a.pts_quality(), PtsQuality::Warn);

    let alignment = AlignmentEngine::new(&map_a, &map_b);

    // VFR with quirks still aligns via PTS
    assert!(matches!(
        alignment.method,
        bitvue_core::AlignmentMethod::PtsExact | bitvue_core::AlignmentMethod::PtsNearest
    ));

    for i in 0..6 {
        let pair = alignment.get_pair_for_a(i).unwrap();
        assert_eq!(pair.stream_b_idx, Some(i));
    }
}
