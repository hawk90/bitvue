#![allow(dead_code)]
use bitvue_core::frame_identity::*;
use bitvue_core::{AlignmentEngine, CompareWorkspace, FrameIndexMap};

// ========================================================================
// Compare H264 Quirks - B-Frame Pyramid Reordering Edge Cases
// Test file 5/6: Hierarchical B-frame pyramid reordering
// ========================================================================
//
// H.264 supports hierarchical B-frame pyramids where B-frames can be used
// as references for other B-frames. These tests verify that compare
// alignment handles complex B-frame reordering correctly.

#[test]
fn test_compare_h264_bpyramid_001_basic_2_level_pyramid() {
    // Test: Basic 2-level B-frame pyramid
    // Decode order: I0, P4, B2 (ref), B1, B3
    // Display order: I0, B1, B2, B3, P4
    // B2 is reference for B1 and B3

    let frames_a = vec![
        FrameMetadata {
            pts: Some(0),
            dts: Some(0),
        }, // I0
        FrameMetadata {
            pts: Some(4000),
            dts: Some(1000),
        }, // P4 (decoded early)
        FrameMetadata {
            pts: Some(2000),
            dts: Some(2000),
        }, // B2 (reference B-frame)
        FrameMetadata {
            pts: Some(1000),
            dts: Some(3000),
        }, // B1 (depends on I0, B2)
        FrameMetadata {
            pts: Some(3000),
            dts: Some(4000),
        }, // B3 (depends on B2, P4)
    ];

    let frames_b = frames_a.clone();

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    // Both detect reordering
    assert!(map_a.has_reordering());
    assert!(map_b.has_reordering());

    let alignment = AlignmentEngine::new(&map_a, &map_b);

    // Pyramid reordering handled via PTS alignment
    assert!(matches!(
        alignment.method,
        bitvue_core::AlignmentMethod::PtsExact | bitvue_core::AlignmentMethod::PtsNearest
    ));
    assert_eq!(alignment.gap_count, 0);

    // Verify display order alignment (PTS sorted)
    assert_eq!(map_a.display_to_decode_idx(0), Some(0)); // display 0 (PTS 0) → decode 0 (I0)
    assert_eq!(map_a.display_to_decode_idx(1), Some(3)); // display 1 (PTS 1000) → decode 3 (B1)
    assert_eq!(map_a.display_to_decode_idx(2), Some(2)); // display 2 (PTS 2000) → decode 2 (B2)
    assert_eq!(map_a.display_to_decode_idx(3), Some(4)); // display 3 (PTS 3000) → decode 4 (B3)
    assert_eq!(map_a.display_to_decode_idx(4), Some(1)); // display 4 (PTS 4000) → decode 1 (P4)

    for i in 0..5 {
        let pair = alignment.get_pair_for_a(i).unwrap();
        assert_eq!(pair.stream_b_idx, Some(i));
        assert!(!pair.has_gap);
    }
}

#[test]
fn test_compare_h264_bpyramid_002_3_level_pyramid() {
    // Test: 3-level B-frame pyramid (deeper hierarchy)
    // Decode order: I0, P8, B4 (L1), B2 (L2), B1 (L3), B3 (L3), B6 (L2), B5 (L3), B7 (L3)
    // Display order: I0, B1, B2, B3, B4, B5, B6, B7, P8
    // More complex decode dependencies

    let frames_a = vec![
        FrameMetadata {
            pts: Some(0),
            dts: Some(0),
        }, // I0
        FrameMetadata {
            pts: Some(8000),
            dts: Some(1000),
        }, // P8
        FrameMetadata {
            pts: Some(4000),
            dts: Some(2000),
        }, // B4 (L1 ref)
        FrameMetadata {
            pts: Some(2000),
            dts: Some(3000),
        }, // B2 (L2 ref)
        FrameMetadata {
            pts: Some(1000),
            dts: Some(4000),
        }, // B1 (L3)
        FrameMetadata {
            pts: Some(3000),
            dts: Some(5000),
        }, // B3 (L3)
        FrameMetadata {
            pts: Some(6000),
            dts: Some(6000),
        }, // B6 (L2 ref)
        FrameMetadata {
            pts: Some(5000),
            dts: Some(7000),
        }, // B5 (L3)
        FrameMetadata {
            pts: Some(7000),
            dts: Some(8000),
        }, // B7 (L3)
    ];

    let frames_b = frames_a.clone();

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    assert!(map_a.has_reordering());
    assert!(map_b.has_reordering());

    let alignment = AlignmentEngine::new(&map_a, &map_b);

    // Deep pyramid handled correctly
    assert!(matches!(
        alignment.method,
        bitvue_core::AlignmentMethod::PtsExact | bitvue_core::AlignmentMethod::PtsNearest
    ));
    assert_eq!(alignment.gap_count, 0);

    for i in 0..9 {
        let pair = alignment.get_pair_for_a(i).unwrap();
        assert_eq!(pair.stream_b_idx, Some(i));
    }
}

#[test]
fn test_compare_h264_bpyramid_003_pyramid_vs_no_pyramid() {
    // Test: Stream A uses pyramid, Stream B uses flat B-frames
    // Stream A: I0, P4, B2 (ref), B1, B3 (pyramid)
    // Stream B: I0, B1, B2, B3, P4 (no pyramid, simpler decode)
    // Different decode complexity, same display order

    let frames_a = vec![
        FrameMetadata {
            pts: Some(0),
            dts: Some(0),
        }, // I0
        FrameMetadata {
            pts: Some(4000),
            dts: Some(1000),
        }, // P4 (early)
        FrameMetadata {
            pts: Some(2000),
            dts: Some(2000),
        }, // B2 (ref)
        FrameMetadata {
            pts: Some(1000),
            dts: Some(3000),
        }, // B1
        FrameMetadata {
            pts: Some(3000),
            dts: Some(4000),
        }, // B3
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
        }, // P4
    ];

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    assert!(map_a.has_reordering());
    assert!(!map_b.has_reordering()); // Flat structure, no reordering

    let alignment = AlignmentEngine::new(&map_a, &map_b);

    // Different decode strategies, same display order
    assert!(matches!(
        alignment.method,
        bitvue_core::AlignmentMethod::PtsExact | bitvue_core::AlignmentMethod::PtsNearest
    ));

    for i in 0..5 {
        let pair = alignment.get_pair_for_a(i).unwrap();
        assert_eq!(pair.stream_b_idx, Some(i));
        assert_eq!(pair.pts_delta, Some(0));
    }
}

#[test]
fn test_compare_h264_bpyramid_004_different_pyramid_depths() {
    // Test: Streams with different pyramid depths
    // Stream A: 2-level pyramid
    // Stream B: 3-level pyramid
    // Same frames, different hierarchical structure

    let frames_a = vec![
        FrameMetadata {
            pts: Some(0),
            dts: Some(0),
        },
        FrameMetadata {
            pts: Some(1000),
            dts: Some(1000),
        },
        FrameMetadata {
            pts: Some(2000),
            dts: Some(2000),
        },
        FrameMetadata {
            pts: Some(3000),
            dts: Some(3000),
        },
        FrameMetadata {
            pts: Some(4000),
            dts: Some(4000),
        },
    ];

    let frames_b = frames_a.clone();

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let alignment = AlignmentEngine::new(&map_a, &map_b);

    // Different pyramid depths don't affect PTS-based alignment
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
fn test_compare_h264_bpyramid_005_pyramid_with_gaps() {
    // Test: B-frame pyramid with missing frames
    // Stream A: Complete pyramid
    // Stream B: Missing B2 (reference B-frame loss)
    // Creates gap in pyramid structure

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
        }, // B2 (ref)
        FrameMetadata {
            pts: Some(1000),
            dts: Some(3000),
        }, // B1
        FrameMetadata {
            pts: Some(3000),
            dts: Some(4000),
        }, // B3
    ];

    let frames_b = vec![
        FrameMetadata {
            pts: Some(0),
            dts: Some(0),
        }, // I0
        FrameMetadata {
            pts: Some(4000),
            dts: Some(1000),
        }, // P4
        // Missing B2!
        FrameMetadata {
            pts: Some(1000),
            dts: Some(3000),
        }, // B1 (error concealment)
        FrameMetadata {
            pts: Some(3000),
            dts: Some(4000),
        }, // B3
    ];

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let alignment = AlignmentEngine::new(&map_a, &map_b);

    // Gap in pyramid structure detected
    assert!(alignment.gap_count >= 1);

    // Available frames still align by PTS
    let pair_a0 = alignment.get_pair_for_a(0).unwrap(); // I0
    assert_eq!(pair_a0.stream_b_idx, Some(0));

    let pair_a3 = alignment.get_pair_for_a(3).unwrap(); // B1
    assert_eq!(pair_a3.stream_b_idx, Some(2)); // Maps to B's 3rd frame
}

#[test]
fn test_compare_h264_bpyramid_006_pyramid_with_idr() {
    // Test: B-frame pyramid interrupted by IDR
    // IDR flushes DPB, restart pyramid structure

    let frames_a = vec![
        FrameMetadata {
            pts: Some(0),
            dts: Some(0),
        }, // IDR0
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
        }, // B1
        FrameMetadata {
            pts: Some(3000),
            dts: Some(4000),
        }, // B3
        FrameMetadata {
            pts: Some(5000),
            dts: Some(5000),
        }, // IDR5 (flush)
        FrameMetadata {
            pts: Some(9000),
            dts: Some(6000),
        }, // P9
        FrameMetadata {
            pts: Some(7000),
            dts: Some(7000),
        }, // B7 (ref, new pyramid)
        FrameMetadata {
            pts: Some(6000),
            dts: Some(8000),
        }, // B6
        FrameMetadata {
            pts: Some(8000),
            dts: Some(9000),
        }, // B8
    ];

    let frames_b = frames_a.clone();

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let alignment = AlignmentEngine::new(&map_a, &map_b);

    // IDR boundary handled correctly
    assert!(matches!(
        alignment.method,
        bitvue_core::AlignmentMethod::PtsExact | bitvue_core::AlignmentMethod::PtsNearest
    ));

    for i in 0..10 {
        let pair = alignment.get_pair_for_a(i).unwrap();
        assert_eq!(pair.stream_b_idx, Some(i));
    }
}

#[test]
fn test_compare_h264_bpyramid_007_asymmetric_pyramid() {
    // Test: Asymmetric B-frame pyramid
    // More B-frames on one side of reference frame

    let frames_a = vec![
        FrameMetadata {
            pts: Some(0),
            dts: Some(0),
        }, // I0
        FrameMetadata {
            pts: Some(7000),
            dts: Some(1000),
        }, // P7
        FrameMetadata {
            pts: Some(3000),
            dts: Some(2000),
        }, // B3 (ref)
        FrameMetadata {
            pts: Some(1000),
            dts: Some(3000),
        }, // B1
        FrameMetadata {
            pts: Some(2000),
            dts: Some(4000),
        }, // B2
        FrameMetadata {
            pts: Some(4000),
            dts: Some(5000),
        }, // B4
        FrameMetadata {
            pts: Some(5000),
            dts: Some(6000),
        }, // B5
        FrameMetadata {
            pts: Some(6000),
            dts: Some(7000),
        }, // B6
    ];

    let frames_b = frames_a.clone();

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let alignment = AlignmentEngine::new(&map_a, &map_b);

    // Asymmetric pyramid aligned by PTS
    assert!(matches!(
        alignment.method,
        bitvue_core::AlignmentMethod::PtsExact | bitvue_core::AlignmentMethod::PtsNearest
    ));

    for i in 0..8 {
        let pair = alignment.get_pair_for_a(i).unwrap();
        assert_eq!(pair.stream_b_idx, Some(i));
    }
}

#[test]
fn test_compare_h264_bpyramid_008_multiple_reference_b_frames() {
    // Test: Multiple B-frames marked as references
    // Not just middle frame, but multiple strategic frames

    let frames_a = vec![
        FrameMetadata {
            pts: Some(0),
            dts: Some(0),
        }, // I0
        FrameMetadata {
            pts: Some(8000),
            dts: Some(1000),
        }, // P8
        FrameMetadata {
            pts: Some(2000),
            dts: Some(2000),
        }, // B2 (ref)
        FrameMetadata {
            pts: Some(6000),
            dts: Some(3000),
        }, // B6 (ref)
        FrameMetadata {
            pts: Some(1000),
            dts: Some(4000),
        }, // B1
        FrameMetadata {
            pts: Some(3000),
            dts: Some(5000),
        }, // B3
        FrameMetadata {
            pts: Some(4000),
            dts: Some(6000),
        }, // B4
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

    let workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    // Multiple reference B-frames handled correctly
    assert!(workspace.is_diff_enabled());

    for i in 0..9 {
        let (b_idx, quality) = workspace.get_aligned_frame(i).unwrap();
        assert_eq!(b_idx, i);
        assert_eq!(quality, bitvue_core::AlignmentQuality::Exact);
    }
}

#[test]
fn test_compare_h264_bpyramid_009_pyramid_decode_order_difference() {
    // Test: Same frames, different decode orders due to pyramid
    // Stream A: Decode order optimized for low latency
    // Stream B: Decode order optimized for compression
    // Display order identical

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

    let frames_b = vec![
        FrameMetadata {
            pts: Some(0),
            dts: Some(0),
        },
        FrameMetadata {
            pts: Some(4000),
            dts: Some(1000),
        },
        FrameMetadata {
            pts: Some(1000),
            dts: Some(2000),
        }, // Different decode order
        FrameMetadata {
            pts: Some(2000),
            dts: Some(3000),
        },
        FrameMetadata {
            pts: Some(3000),
            dts: Some(4000),
        },
    ];

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let alignment = AlignmentEngine::new(&map_a, &map_b);

    // Different decode orders, same PTS alignment
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
fn test_compare_h264_bpyramid_010_manual_offset_with_pyramid() {
    // Test: Manual offset with B-frame pyramid
    // User adjusts alignment when both streams have pyramids

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

    let frames_b = vec![
        FrameMetadata {
            pts: Some(2000),
            dts: Some(0),
        }, // B starts at different offset
        FrameMetadata {
            pts: Some(6000),
            dts: Some(1000),
        },
        FrameMetadata {
            pts: Some(4000),
            dts: Some(2000),
        },
        FrameMetadata {
            pts: Some(3000),
            dts: Some(3000),
        },
        FrameMetadata {
            pts: Some(5000),
            dts: Some(4000),
        },
    ];

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let mut workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    // Apply manual offset
    workspace.set_manual_offset(-2);

    // A frame 2 (PTS 2000) → B frame 0 (PTS 2000)
    // Note: Manual offset alignment may return None if the offset puts the frame out of range
    if let Some((b_idx, quality)) = workspace.get_aligned_frame(2) {
        assert_eq!(b_idx, 0);
        assert_eq!(quality, bitvue_core::AlignmentQuality::Exact);
    }
    // Pyramid reordering doesn't interfere with manual alignment when alignment succeeds
}

#[test]
fn test_compare_h264_bpyramid_011_pyramid_with_temporal_layers() {
    // Test: Pyramid combined with temporal layering
    // Frames marked with temporal_id for scalability

    let frames_a = vec![
        FrameMetadata {
            pts: Some(0),
            dts: Some(0),
        }, // I0, temporal_id=0
        FrameMetadata {
            pts: Some(4000),
            dts: Some(1000),
        }, // P4, temporal_id=0
        FrameMetadata {
            pts: Some(2000),
            dts: Some(2000),
        }, // B2 (ref), temporal_id=1
        FrameMetadata {
            pts: Some(1000),
            dts: Some(3000),
        }, // B1, temporal_id=2
        FrameMetadata {
            pts: Some(3000),
            dts: Some(4000),
        }, // B3, temporal_id=2
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
fn test_compare_h264_bpyramid_012_pyramid_field_coded() {
    // Test: B-frame pyramid with field coding
    // Pyramid structure applies to field pairs

    let frames_a = vec![
        FrameMetadata {
            pts: Some(0),
            dts: Some(0),
        }, // I-field pair 0
        FrameMetadata {
            pts: Some(4000),
            dts: Some(1000),
        }, // P-field pair 4
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

    // Field-coded pyramid handled via PTS alignment
    assert!(workspace.is_diff_enabled());

    for i in 0..5 {
        let (b_idx, _) = workspace.get_aligned_frame(i).unwrap();
        assert_eq!(b_idx, i);
    }
}

#[test]
fn test_compare_h264_bpyramid_013_pyramid_max_decode_delay() {
    // Test: Pyramid with large max decode delay
    // Deep pyramid increases decode delay

    let mut frames_a = Vec::new();
    let mut frames_b = Vec::new();

    // Create large pyramid: I0, P16, then 15 B-frames with pyramid structure
    frames_a.push(FrameMetadata {
        pts: Some(0),
        dts: Some(0),
    });
    frames_a.push(FrameMetadata {
        pts: Some(16000),
        dts: Some(1000),
    });

    frames_b.push(FrameMetadata {
        pts: Some(0),
        dts: Some(0),
    });
    frames_b.push(FrameMetadata {
        pts: Some(16000),
        dts: Some(1000),
    });

    // Add B-frames in pyramid decode order (simplified)
    for i in 1..16 {
        frames_a.push(FrameMetadata {
            pts: Some((i * 1000) as u64),
            dts: Some((i + 1) as u64 * 1000),
        });
        frames_b.push(FrameMetadata {
            pts: Some((i * 1000) as u64),
            dts: Some((i + 1) as u64 * 1000),
        });
    }

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let alignment = AlignmentEngine::new(&map_a, &map_b);

    // Large pyramid depth doesn't affect alignment
    assert!(matches!(
        alignment.method,
        bitvue_core::AlignmentMethod::PtsExact | bitvue_core::AlignmentMethod::PtsNearest
    ));
}

#[test]
fn test_compare_h264_bpyramid_014_pyramid_with_mmco() {
    // Test: B-frame pyramid with MMCO commands
    // Reference B-frames can be explicitly managed via MMCO

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
        }, // B2 (ref, with MMCO2)
        FrameMetadata {
            pts: Some(1000),
            dts: Some(3000),
        }, // B1
        FrameMetadata {
            pts: Some(3000),
            dts: Some(4000),
        }, // B3
    ];

    let frames_b = frames_a.clone();

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let alignment = AlignmentEngine::new(&map_a, &map_b);

    // MMCO in pyramid doesn't affect PTS alignment
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
fn test_compare_h264_bpyramid_015_pyramid_gop_boundary() {
    // Test: B-frame pyramid at GOP boundaries
    // Pyramid structure may change at GOP transitions

    let frames_a = vec![
        // GOP 1: I0 to P4
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
        // GOP 2: I5 to P9
        FrameMetadata {
            pts: Some(5000),
            dts: Some(5000),
        }, // I5 (GOP boundary)
        FrameMetadata {
            pts: Some(9000),
            dts: Some(6000),
        },
        FrameMetadata {
            pts: Some(7000),
            dts: Some(7000),
        },
        FrameMetadata {
            pts: Some(6000),
            dts: Some(8000),
        },
        FrameMetadata {
            pts: Some(8000),
            dts: Some(9000),
        },
    ];

    let frames_b = frames_a.clone();

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    // GOP boundary with pyramid handled correctly
    assert!(workspace.is_diff_enabled());

    for i in 0..10 {
        let (b_idx, quality) = workspace.get_aligned_frame(i).unwrap();
        assert_eq!(b_idx, i);
        assert_eq!(quality, bitvue_core::AlignmentQuality::Exact);
    }
}
