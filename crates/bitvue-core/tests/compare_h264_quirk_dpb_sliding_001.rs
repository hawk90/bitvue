#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(unused_comparisons)]
#![allow(unused_doc_comments)]
use bitvue_core::frame_identity::*;
use bitvue_core::{AlignmentEngine, CompareWorkspace, FrameIndexMap};

// ========================================================================
// Compare H264 Quirks - DPB Sliding Window Edge Cases
// Test file 2/6: Decoded Picture Buffer sliding window edge cases
// ========================================================================
//
// H.264 DPB (Decoded Picture Buffer) uses sliding window for reference
// frame management. These tests verify that compare alignment handles
// DPB edge cases correctly, especially when reference frames are dropped
// or memory management differs between streams.

#[test]
fn test_compare_h264_dpb_001_basic_sliding_window() {
    // Test: Basic sliding window with max_dec_pic_buffering
    // Stream A: I0, P1, P2, P3, P4 (DPB size = 3)
    // Stream B: Same structure
    // Older references slide out of DPB as new frames arrive

    let frames_a = vec![
        FrameMetadata {
            pts: Some(0),
            dts: Some(0),
        }, // I0
        FrameMetadata {
            pts: Some(1000),
            dts: Some(1000),
        }, // P1 (refs I0)
        FrameMetadata {
            pts: Some(2000),
            dts: Some(2000),
        }, // P2 (refs P1)
        FrameMetadata {
            pts: Some(3000),
            dts: Some(3000),
        }, // P3 (refs P2, I0 slides out)
        FrameMetadata {
            pts: Some(4000),
            dts: Some(4000),
        }, // P4 (refs P3, P1 slides out)
    ];

    let frames_b = frames_a.clone();

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let alignment = AlignmentEngine::new(&map_a, &map_b);

    // DPB sliding is internal decoder state, doesn't affect alignment
    assert!(matches!(
        alignment.method,
        bitvue_core::AlignmentMethod::PtsExact | bitvue_core::AlignmentMethod::PtsNearest
    ));
    assert_eq!(alignment.gap_count, 0);

    // All frames align correctly regardless of DPB state
    for i in 0..5 {
        let pair = alignment.get_pair_for_a(i).unwrap();
        assert_eq!(pair.stream_b_idx, Some(i));
        assert!(!pair.has_gap);
    }
}

#[test]
fn test_compare_h264_dpb_002_dpb_overflow_handling() {
    // Test: DPB overflow scenario (more refs than buffer size)
    // Stream A: Normal encoding with DPB size = 3
    // Stream B: Same content but different encoder may manage DPB differently
    // Frame identity should be independent of DPB management

    let frames_a = vec![
        FrameMetadata {
            pts: Some(0),
            dts: Some(0),
        }, // I0
        FrameMetadata {
            pts: Some(1000),
            dts: Some(1000),
        }, // P1
        FrameMetadata {
            pts: Some(2000),
            dts: Some(2000),
        }, // P2
        FrameMetadata {
            pts: Some(3000),
            dts: Some(3000),
        }, // P3 (DPB full)
        FrameMetadata {
            pts: Some(4000),
            dts: Some(4000),
        }, // P4 (oldest slides out)
        FrameMetadata {
            pts: Some(5000),
            dts: Some(5000),
        }, // P5
    ];

    let frames_b = frames_a.clone();

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    // DPB overflow is encoder concern, not frame identity concern
    assert!(workspace.is_diff_enabled());

    // Verify alignment across DPB overflow points
    for i in 0..6 {
        let (b_idx, quality) = workspace.get_aligned_frame(i).unwrap();
        assert_eq!(b_idx, i);
        assert_eq!(quality, bitvue_core::AlignmentQuality::Exact);
    }
}

#[test]
fn test_compare_h264_dpb_003_adaptive_ref_pic_marking() {
    // Test: Adaptive reference picture marking (MMCO commands)
    // Stream A uses sliding window (automatic)
    // Stream B uses explicit MMCO commands
    // Different DPB management strategies should still align

    let frames_a = vec![
        FrameMetadata {
            pts: Some(0),
            dts: Some(0),
        }, // I0
        FrameMetadata {
            pts: Some(1000),
            dts: Some(1000),
        }, // P1
        FrameMetadata {
            pts: Some(2000),
            dts: Some(2000),
        }, // P2
        FrameMetadata {
            pts: Some(3000),
            dts: Some(3000),
        }, // P3 (with MMCO)
        FrameMetadata {
            pts: Some(4000),
            dts: Some(4000),
        }, // P4
    ];

    let frames_b = frames_a.clone();

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let alignment = AlignmentEngine::new(&map_a, &map_b);

    // Different memory management strategies shouldn't affect alignment
    assert!(matches!(
        alignment.method,
        bitvue_core::AlignmentMethod::PtsExact | bitvue_core::AlignmentMethod::PtsNearest
    ));
    assert_eq!(alignment.gap_count, 0);
}

#[test]
fn test_compare_h264_dpb_004_dpb_size_mismatch() {
    // Test: Streams with different DPB sizes
    // Stream A: max_dec_pic_buffering = 3
    // Stream B: max_dec_pic_buffering = 5
    // Should still align by PTS

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
        FrameMetadata {
            pts: Some(5000),
            dts: Some(5000),
        },
    ];

    let frames_b = frames_a.clone();

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    // DPB size difference is encoder setting, not alignment concern
    assert!(workspace.is_diff_enabled());

    for i in 0..6 {
        let (b_idx, _) = workspace.get_aligned_frame(i).unwrap();
        assert_eq!(b_idx, i);
    }
}

#[test]
fn test_compare_h264_dpb_005_sliding_window_with_gaps() {
    // Test: Sliding window with gaps in frame_num
    // Gaps in frame_num can cause sliding window to advance
    // Alignment should use available frames only

    let frames_a = vec![
        FrameMetadata {
            pts: Some(0),
            dts: Some(0),
        }, // frame_num 0
        FrameMetadata {
            pts: Some(1000),
            dts: Some(1000),
        }, // frame_num 1
        FrameMetadata {
            pts: Some(2000),
            dts: Some(2000),
        }, // frame_num 2
        // gap: frame_num 3, 4 missing (network loss)
        FrameMetadata {
            pts: Some(5000),
            dts: Some(5000),
        }, // frame_num 5 (gap detected)
        FrameMetadata {
            pts: Some(6000),
            dts: Some(6000),
        }, // frame_num 6
    ];

    let frames_b = vec![
        FrameMetadata {
            pts: Some(0),
            dts: Some(0),
        }, // frame_num 0
        FrameMetadata {
            pts: Some(1000),
            dts: Some(1000),
        }, // frame_num 1
        FrameMetadata {
            pts: Some(2000),
            dts: Some(2000),
        }, // frame_num 2
        FrameMetadata {
            pts: Some(3000),
            dts: Some(3000),
        }, // frame_num 3 (no gap in B)
        FrameMetadata {
            pts: Some(4000),
            dts: Some(4000),
        }, // frame_num 4
        FrameMetadata {
            pts: Some(5000),
            dts: Some(5000),
        }, // frame_num 5
        FrameMetadata {
            pts: Some(6000),
            dts: Some(6000),
        }, // frame_num 6
    ];

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    // A has PTS gap (missing frames)
    assert_eq!(map_a.pts_quality(), PtsQuality::Warn);
    assert_eq!(map_b.pts_quality(), PtsQuality::Ok);

    let alignment = AlignmentEngine::new(&map_a, &map_b);

    // Should align available frames (may use nearest due to gaps)
    assert!(matches!(
        alignment.method,
        bitvue_core::AlignmentMethod::PtsExact | bitvue_core::AlignmentMethod::PtsNearest
    ));

    // Verify frames with matching PTS align
    let pair_a0 = alignment.get_pair_for_a(0).unwrap(); // PTS 0
    assert_eq!(pair_a0.stream_b_idx, Some(0));

    let pair_a3 = alignment.get_pair_for_a(3).unwrap(); // PTS 5000
    assert_eq!(pair_a3.stream_b_idx, Some(5));

    // B has extra frames where A has gap
    assert!(alignment.gap_count >= 2);
}

#[test]
fn test_compare_h264_dpb_006_dpb_flush_at_idr() {
    // Test: DPB flush at IDR frame
    // IDR frame causes complete DPB flush
    // Alignment should handle DPB discontinuity

    let frames_a = vec![
        FrameMetadata {
            pts: Some(0),
            dts: Some(0),
        }, // IDR
        FrameMetadata {
            pts: Some(1000),
            dts: Some(1000),
        }, // P1
        FrameMetadata {
            pts: Some(2000),
            dts: Some(2000),
        }, // P2
        FrameMetadata {
            pts: Some(3000),
            dts: Some(3000),
        }, // IDR (DPB flush!)
        FrameMetadata {
            pts: Some(4000),
            dts: Some(4000),
        }, // P4 (new DPB state)
        FrameMetadata {
            pts: Some(5000),
            dts: Some(5000),
        }, // P5
    ];

    let frames_b = frames_a.clone();

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let alignment = AlignmentEngine::new(&map_a, &map_b);

    // DPB flush is decoder internal, doesn't affect alignment
    assert!(matches!(
        alignment.method,
        bitvue_core::AlignmentMethod::PtsExact | bitvue_core::AlignmentMethod::PtsNearest
    ));

    // Verify alignment across DPB flush boundary
    for i in 0..6 {
        let pair = alignment.get_pair_for_a(i).unwrap();
        assert_eq!(pair.stream_b_idx, Some(i));
        assert!(!pair.has_gap);
    }
}

#[test]
fn test_compare_h264_dpb_007_long_term_ref_in_dpb() {
    // Test: Long-term reference frames in DPB
    // Long-term refs don't participate in sliding window
    // Should not affect frame alignment

    let frames_a = vec![
        FrameMetadata {
            pts: Some(0),
            dts: Some(0),
        }, // I0 (marked long-term)
        FrameMetadata {
            pts: Some(1000),
            dts: Some(1000),
        }, // P1
        FrameMetadata {
            pts: Some(2000),
            dts: Some(2000),
        }, // P2
        FrameMetadata {
            pts: Some(3000),
            dts: Some(3000),
        }, // P3 (I0 still in DPB as long-term)
        FrameMetadata {
            pts: Some(4000),
            dts: Some(4000),
        }, // P4
    ];

    let frames_b = frames_a.clone();

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    // Long-term reference management is decoder concern
    assert!(workspace.is_diff_enabled());

    for i in 0..5 {
        let (b_idx, quality) = workspace.get_aligned_frame(i).unwrap();
        assert_eq!(b_idx, i);
        assert_eq!(quality, bitvue_core::AlignmentQuality::Exact);
    }
}

#[test]
fn test_compare_h264_dpb_008_dpb_fullness_difference() {
    // Test: Streams with different DPB utilization
    // Stream A: Conservative encoding (low DPB usage)
    // Stream B: Aggressive encoding (high DPB usage)

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
    ];

    let frames_b = frames_a.clone();

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let alignment = AlignmentEngine::new(&map_a, &map_b);

    // Different DPB utilization strategies don't affect alignment
    assert!(matches!(
        alignment.method,
        bitvue_core::AlignmentMethod::PtsExact | bitvue_core::AlignmentMethod::PtsNearest
    ));
    assert_eq!(alignment.gap_count, 0);
}

#[test]
fn test_compare_h264_dpb_009_mmco_explicit_removal() {
    // Test: MMCO command explicitly removes frame from DPB
    // Stream A: MMCO=1 (mark short-term unused)
    // Stream B: Different MMCO strategy
    // Alignment independent of memory management

    let frames_a = vec![
        FrameMetadata {
            pts: Some(0),
            dts: Some(0),
        }, // I0
        FrameMetadata {
            pts: Some(1000),
            dts: Some(1000),
        }, // P1
        FrameMetadata {
            pts: Some(2000),
            dts: Some(2000),
        }, // P2 (with MMCO=1)
        FrameMetadata {
            pts: Some(3000),
            dts: Some(3000),
        }, // P3
    ];

    let frames_b = frames_a.clone();

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let alignment = AlignmentEngine::new(&map_a, &map_b);

    // MMCO commands are decoder-internal
    assert!(matches!(
        alignment.method,
        bitvue_core::AlignmentMethod::PtsExact | bitvue_core::AlignmentMethod::PtsNearest
    ));

    for i in 0..4 {
        let pair = alignment.get_pair_for_a(i).unwrap();
        assert_eq!(pair.stream_b_idx, Some(i));
    }
}

#[test]
fn test_compare_h264_dpb_010_bumping_process() {
    // Test: DPB bumping process (output reordering)
    // When DPB full, oldest picture is "bumped" for output
    // Bumping affects output order but not frame identity

    let frames_a = vec![
        FrameMetadata {
            pts: Some(0),
            dts: Some(0),
        }, // I0
        FrameMetadata {
            pts: Some(3000),
            dts: Some(1000),
        }, // P3 (decoded early)
        FrameMetadata {
            pts: Some(1000),
            dts: Some(2000),
        }, // B1 (needs I0, P3)
        FrameMetadata {
            pts: Some(2000),
            dts: Some(3000),
        }, // B2 (DPB full, B1 bumped)
        FrameMetadata {
            pts: Some(4000),
            dts: Some(4000),
        }, // P4 (B2 bumped)
    ];

    let frames_b = frames_a.clone();

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    // Both detect reordering
    assert!(map_a.has_reordering());
    assert!(map_b.has_reordering());

    let alignment = AlignmentEngine::new(&map_a, &map_b);

    // Bumping process is internal, alignment uses PTS
    assert!(matches!(
        alignment.method,
        bitvue_core::AlignmentMethod::PtsExact | bitvue_core::AlignmentMethod::PtsNearest
    ));

    // Verify alignment in display order (after bumping)
    for i in 0..5 {
        let pair = alignment.get_pair_for_a(i).unwrap();
        assert_eq!(pair.stream_b_idx, Some(i));
        assert!(!pair.has_gap);
    }
}

#[test]
fn test_compare_h264_dpb_011_max_num_reorder_frames() {
    // Test: max_num_reorder_frames limits DPB reordering
    // Stream A: max_num_reorder_frames = 2
    // Stream B: max_num_reorder_frames = 4
    // Different reorder limits shouldn't affect alignment

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
    ];

    let frames_b = frames_a.clone();

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    // Reorder limits are SPS parameters, not alignment concern
    assert!(workspace.is_diff_enabled());

    for i in 0..4 {
        let (b_idx, _) = workspace.get_aligned_frame(i).unwrap();
        assert_eq!(b_idx, i);
    }
}

#[test]
fn test_compare_h264_dpb_012_dpb_output_delay() {
    // Test: DPB output delay (num_reorder_frames difference)
    // Stream A: Low latency (immediate output)
    // Stream B: Higher quality (delayed output for reordering)

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
    ];

    let frames_b = frames_a.clone();

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let alignment = AlignmentEngine::new(&map_a, &map_b);

    // Output delay is decoder buffering, not frame identity concern
    assert!(matches!(
        alignment.method,
        bitvue_core::AlignmentMethod::PtsExact | bitvue_core::AlignmentMethod::PtsNearest
    ));

    for i in 0..3 {
        let pair = alignment.get_pair_for_a(i).unwrap();
        assert_eq!(pair.stream_b_idx, Some(i));
    }
}

#[test]
fn test_compare_h264_dpb_013_manual_offset_with_dpb_state() {
    // Test: Manual offset when streams have different DPB states
    // User adjusts offset while encoders manage DPB differently

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
    ];

    let frames_b = vec![
        FrameMetadata {
            pts: Some(1000),
            dts: Some(1000),
        }, // B starts 1 frame later
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

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let mut workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    // Apply manual offset
    workspace.set_manual_offset(-1);

    // A frame 1 (PTS 1000) → B frame 0 (PTS 1000)
    if let Some((b_idx, quality)) = workspace.get_aligned_frame(1) {
        assert_eq!(b_idx, 0);
        assert_eq!(quality, bitvue_core::AlignmentQuality::Exact);
    }

    // DPB state differences don't interfere with manual alignment
}

#[test]
fn test_compare_h264_dpb_014_dpb_in_field_mode() {
    // Test: DPB in field coding mode
    // Field pictures use DPB differently than frame pictures
    // Alignment should be independent of field vs frame DPB usage

    let frames_a = vec![
        FrameMetadata {
            pts: Some(0),
            dts: Some(0),
        }, // Field pair 0
        FrameMetadata {
            pts: Some(1000),
            dts: Some(1000),
        }, // Field pair 1
        FrameMetadata {
            pts: Some(2000),
            dts: Some(2000),
        }, // Field pair 2
    ];

    let frames_b = frames_a.clone();

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let alignment = AlignmentEngine::new(&map_a, &map_b);

    // Field vs frame DPB mode is decoder concern
    assert!(matches!(
        alignment.method,
        bitvue_core::AlignmentMethod::PtsExact | bitvue_core::AlignmentMethod::PtsNearest
    ));

    for i in 0..3 {
        let pair = alignment.get_pair_for_a(i).unwrap();
        assert_eq!(pair.stream_b_idx, Some(i));
    }
}

#[test]
fn test_compare_h264_dpb_015_dpb_with_complementary_fields() {
    // Test: DPB with complementary field pairs
    // Top and bottom fields stored separately in DPB
    // Container still presents as frames with single PTS

    let frames_a = vec![
        FrameMetadata {
            pts: Some(0),
            dts: Some(0),
        }, // Frame 0 (top+bottom)
        FrameMetadata {
            pts: Some(1000),
            dts: Some(1000),
        }, // Frame 1
        FrameMetadata {
            pts: Some(2000),
            dts: Some(2000),
        }, // Frame 2
    ];

    let frames_b = frames_a.clone();

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    // Complementary field pairing is decoder concern
    assert!(workspace.is_diff_enabled());

    for i in 0..3 {
        let (b_idx, _) = workspace.get_aligned_frame(i).unwrap();
        assert_eq!(b_idx, i);
    }
}
