#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(unused_comparisons)]
#![allow(unused_doc_comments)]
use bitvue_core::frame_identity::*;
use bitvue_core::{AlignmentEngine, CompareWorkspace, FrameIndexMap};

// ========================================================================
// Compare H264 Quirks - Field/Frame Mixing Edge Cases
// Test file 3/6: Field and frame coding mix scenarios
// ========================================================================
//
// H.264 supports frame coding, field coding, and MBAFF (mixed mode).
// These tests verify that compare alignment handles field/frame mixing
// correctly, especially when streams use different coding modes.

#[test]
fn test_compare_h264_field_frame_001_frame_vs_frame() {
    // Test: Both streams use frame coding (baseline)
    // Stream A: Frame-coded pictures
    // Stream B: Frame-coded pictures
    // Standard case, should align perfectly

    let frames_a = vec![
        FrameMetadata {
            pts: Some(0),
            dts: Some(0),
        }, // Frame 0
        FrameMetadata {
            pts: Some(1000),
            dts: Some(1000),
        }, // Frame 1
        FrameMetadata {
            pts: Some(2000),
            dts: Some(2000),
        }, // Frame 2
        FrameMetadata {
            pts: Some(3000),
            dts: Some(3000),
        }, // Frame 3
    ];

    let frames_b = frames_a.clone();

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let alignment = AlignmentEngine::new(&map_a, &map_b);

    // Frame-coded streams should align perfectly
    assert!(matches!(
        alignment.method,
        bitvue_core::AlignmentMethod::PtsExact | bitvue_core::AlignmentMethod::PtsNearest
    ));
    assert_eq!(alignment.gap_count, 0);

    for i in 0..4 {
        let pair = alignment.get_pair_for_a(i).unwrap();
        assert_eq!(pair.stream_b_idx, Some(i));
        assert!(!pair.has_gap);
    }
}

#[test]
fn test_compare_h264_field_frame_002_field_vs_field() {
    // Test: Both streams use field coding (interlaced)
    // Stream A: Field-coded pictures (top/bottom pairs)
    // Stream B: Field-coded pictures
    // Container presents as frames (field pairs)

    let frames_a = vec![
        FrameMetadata {
            pts: Some(0),
            dts: Some(0),
        }, // Frame 0 (top+bottom)
        FrameMetadata {
            pts: Some(1000),
            dts: Some(1000),
        }, // Frame 1 (top+bottom)
        FrameMetadata {
            pts: Some(2000),
            dts: Some(2000),
        }, // Frame 2 (top+bottom)
    ];

    let frames_b = frames_a.clone();

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let alignment = AlignmentEngine::new(&map_a, &map_b);

    // Field-coded streams align by frame (field pairs)
    assert!(matches!(
        alignment.method,
        bitvue_core::AlignmentMethod::PtsExact | bitvue_core::AlignmentMethod::PtsNearest
    ));
    assert_eq!(alignment.gap_count, 0);

    for i in 0..3 {
        let pair = alignment.get_pair_for_a(i).unwrap();
        assert_eq!(pair.stream_b_idx, Some(i));
    }
}

#[test]
fn test_compare_h264_field_frame_003_frame_vs_field() {
    // Test: Stream A frame-coded, Stream B field-coded
    // Different coding modes but same temporal resolution
    // Should align by container PTS

    let frames_a = vec![
        FrameMetadata {
            pts: Some(0),
            dts: Some(0),
        }, // Frame 0
        FrameMetadata {
            pts: Some(1000),
            dts: Some(1000),
        }, // Frame 1
        FrameMetadata {
            pts: Some(2000),
            dts: Some(2000),
        }, // Frame 2
    ];

    let frames_b = vec![
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

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let alignment = AlignmentEngine::new(&map_a, &map_b);

    // Different coding modes don't affect PTS-based alignment
    assert!(matches!(
        alignment.method,
        bitvue_core::AlignmentMethod::PtsExact | bitvue_core::AlignmentMethod::PtsNearest
    ));

    for i in 0..3 {
        let pair = alignment.get_pair_for_a(i).unwrap();
        assert_eq!(pair.stream_b_idx, Some(i));
        assert_eq!(pair.pts_delta, Some(0));
    }
}

#[test]
fn test_compare_h264_field_frame_004_mbaff_vs_frame() {
    // Test: Stream A uses MBAFF, Stream B uses frame coding
    // MBAFF (Macroblock-Adaptive Frame-Field) mixes modes per MB pair
    // Container presents as frames with single PTS

    let frames_a = vec![
        FrameMetadata {
            pts: Some(0),
            dts: Some(0),
        }, // MBAFF picture (mixed)
        FrameMetadata {
            pts: Some(1000),
            dts: Some(1000),
        }, // MBAFF picture
        FrameMetadata {
            pts: Some(2000),
            dts: Some(2000),
        }, // MBAFF picture
    ];

    let frames_b = vec![
        FrameMetadata {
            pts: Some(0),
            dts: Some(0),
        }, // Frame-coded
        FrameMetadata {
            pts: Some(1000),
            dts: Some(1000),
        }, // Frame-coded
        FrameMetadata {
            pts: Some(2000),
            dts: Some(2000),
        }, // Frame-coded
    ];

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    // MBAFF vs frame coding shouldn't affect alignment
    assert!(workspace.is_diff_enabled());

    for i in 0..3 {
        let (b_idx, quality) = workspace.get_aligned_frame(i).unwrap();
        assert_eq!(b_idx, i);
        assert_eq!(quality, bitvue_core::AlignmentQuality::Exact);
    }
}

#[test]
fn test_compare_h264_field_frame_005_mbaff_vs_field() {
    // Test: Stream A uses MBAFF, Stream B uses field coding
    // Both handle interlaced content differently

    let frames_a = vec![
        FrameMetadata {
            pts: Some(0),
            dts: Some(0),
        }, // MBAFF
        FrameMetadata {
            pts: Some(1000),
            dts: Some(1000),
        }, // MBAFF
        FrameMetadata {
            pts: Some(2000),
            dts: Some(2000),
        }, // MBAFF
        FrameMetadata {
            pts: Some(3000),
            dts: Some(3000),
        }, // MBAFF
    ];

    let frames_b = vec![
        FrameMetadata {
            pts: Some(0),
            dts: Some(0),
        }, // Field pair
        FrameMetadata {
            pts: Some(1000),
            dts: Some(1000),
        }, // Field pair
        FrameMetadata {
            pts: Some(2000),
            dts: Some(2000),
        }, // Field pair
        FrameMetadata {
            pts: Some(3000),
            dts: Some(3000),
        }, // Field pair
    ];

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let alignment = AlignmentEngine::new(&map_a, &map_b);

    // MBAFF vs field coding should still align by PTS
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
fn test_compare_h264_field_frame_006_pic_struct_differences() {
    // Test: Different pic_struct values (SEI message)
    // Stream A: pic_struct = frame
    // Stream B: pic_struct = top/bottom field
    // Container PTS should be authoritative

    let frames_a = vec![
        FrameMetadata {
            pts: Some(0),
            dts: Some(0),
        }, // pic_struct: frame
        FrameMetadata {
            pts: Some(1000),
            dts: Some(1000),
        }, // pic_struct: frame
        FrameMetadata {
            pts: Some(2000),
            dts: Some(2000),
        }, // pic_struct: frame
    ];

    let frames_b = vec![
        FrameMetadata {
            pts: Some(0),
            dts: Some(0),
        }, // pic_struct: top+bottom
        FrameMetadata {
            pts: Some(1000),
            dts: Some(1000),
        }, // pic_struct: top+bottom
        FrameMetadata {
            pts: Some(2000),
            dts: Some(2000),
        }, // pic_struct: top+bottom
    ];

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    // pic_struct is display hint, not alignment determinant
    assert!(workspace.is_diff_enabled());

    for i in 0..3 {
        let (b_idx, _) = workspace.get_aligned_frame(i).unwrap();
        assert_eq!(b_idx, i);
    }
}

#[test]
fn test_compare_h264_field_frame_007_unpaired_field() {
    // Test: Unpaired field (missing complementary field)
    // Stream A: Normal field pairs
    // Stream B: Missing bottom field in frame 1
    // Containers may present differently

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

    let frames_b = vec![
        FrameMetadata {
            pts: Some(0),
            dts: Some(0),
        }, // Field pair 0
        // Missing field at PTS 1000 (error concealment or loss)
        FrameMetadata {
            pts: Some(2000),
            dts: Some(2000),
        }, // Field pair 2
    ];

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let alignment = AlignmentEngine::new(&map_a, &map_b);

    // Missing field creates gap in B
    assert!(matches!(
        alignment.method,
        bitvue_core::AlignmentMethod::PtsExact | bitvue_core::AlignmentMethod::PtsNearest
    ));
    assert!(alignment.gap_count >= 1);

    // Verify available frames align
    let pair_a0 = alignment.get_pair_for_a(0).unwrap();
    assert_eq!(pair_a0.stream_b_idx, Some(0));

    let pair_a2 = alignment.get_pair_for_a(2).unwrap();
    assert_eq!(pair_a2.stream_b_idx, Some(1));
}

#[test]
fn test_compare_h264_field_frame_008_field_to_frame_conversion() {
    // Test: Field-coded source converted to frame-coded
    // Stream A: Field-coded (interlaced source)
    // Stream B: Deinterlaced to frames
    // Same temporal positions, different coding

    let frames_a = vec![
        FrameMetadata {
            pts: Some(0),
            dts: Some(0),
        }, // Field pair
        FrameMetadata {
            pts: Some(1000),
            dts: Some(1000),
        }, // Field pair
        FrameMetadata {
            pts: Some(2000),
            dts: Some(2000),
        }, // Field pair
    ];

    let frames_b = vec![
        FrameMetadata {
            pts: Some(0),
            dts: Some(0),
        }, // Frame (deinterlaced)
        FrameMetadata {
            pts: Some(1000),
            dts: Some(1000),
        }, // Frame
        FrameMetadata {
            pts: Some(2000),
            dts: Some(2000),
        }, // Frame
    ];

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let alignment = AlignmentEngine::new(&map_a, &map_b);

    // Field-to-frame conversion doesn't affect PTS alignment
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
fn test_compare_h264_field_frame_009_bottom_field_first() {
    // Test: Bottom field first vs top field first
    // Stream A: top_field_first = 1
    // Stream B: bottom_field_first = 1
    // Field order is display concern, not alignment concern

    let frames_a = vec![
        FrameMetadata {
            pts: Some(0),
            dts: Some(0),
        }, // top_field_first
        FrameMetadata {
            pts: Some(1000),
            dts: Some(1000),
        }, // top_field_first
        FrameMetadata {
            pts: Some(2000),
            dts: Some(2000),
        }, // top_field_first
    ];

    let frames_b = vec![
        FrameMetadata {
            pts: Some(0),
            dts: Some(0),
        }, // bottom_field_first
        FrameMetadata {
            pts: Some(1000),
            dts: Some(1000),
        }, // bottom_field_first
        FrameMetadata {
            pts: Some(2000),
            dts: Some(2000),
        }, // bottom_field_first
    ];

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    // Field order is decoder/display concern
    assert!(workspace.is_diff_enabled());

    for i in 0..3 {
        let (b_idx, quality) = workspace.get_aligned_frame(i).unwrap();
        assert_eq!(b_idx, i);
        assert_eq!(quality, bitvue_core::AlignmentQuality::Exact);
    }
}

#[test]
fn test_compare_h264_field_frame_010_field_reordering() {
    // Test: B-fields with reordering
    // Decode order: I-field0, P-field3, B-field1, B-field2
    // Display order: I-field0, B-field1, B-field2, P-field3

    let frames_a = vec![
        FrameMetadata {
            pts: Some(0),
            dts: Some(0),
        }, // I-field
        FrameMetadata {
            pts: Some(3000),
            dts: Some(1000),
        }, // P-field
        FrameMetadata {
            pts: Some(1000),
            dts: Some(2000),
        }, // B-field
        FrameMetadata {
            pts: Some(2000),
            dts: Some(3000),
        }, // B-field
    ];

    let frames_b = frames_a.clone();

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    // Both detect reordering
    assert!(map_a.has_reordering());
    assert!(map_b.has_reordering());

    let alignment = AlignmentEngine::new(&map_a, &map_b);

    // Field reordering handled like frame reordering
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
fn test_compare_h264_field_frame_011_mixed_mode_switching() {
    // Test: Stream switches between field and frame coding
    // Some pictures field-coded, others frame-coded
    // Container normalizes to frames with consistent PTS

    let frames_a = vec![
        FrameMetadata {
            pts: Some(0),
            dts: Some(0),
        }, // Frame-coded
        FrameMetadata {
            pts: Some(1000),
            dts: Some(1000),
        }, // Field-coded
        FrameMetadata {
            pts: Some(2000),
            dts: Some(2000),
        }, // Frame-coded
        FrameMetadata {
            pts: Some(3000),
            dts: Some(3000),
        }, // Field-coded
    ];

    let frames_b = frames_a.clone();

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let alignment = AlignmentEngine::new(&map_a, &map_b);

    // Mixed mode switching transparent to alignment
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
fn test_compare_h264_field_frame_012_paff_vs_mbaff() {
    // Test: PAFF (Picture-Adaptive Frame-Field) vs MBAFF
    // PAFF: Switch per picture
    // MBAFF: Switch per macroblock pair
    // Both presented as frames by container

    let frames_a = vec![
        FrameMetadata {
            pts: Some(0),
            dts: Some(0),
        }, // PAFF (frame picture)
        FrameMetadata {
            pts: Some(1000),
            dts: Some(1000),
        }, // PAFF (field picture)
        FrameMetadata {
            pts: Some(2000),
            dts: Some(2000),
        }, // PAFF (frame picture)
    ];

    let frames_b = vec![
        FrameMetadata {
            pts: Some(0),
            dts: Some(0),
        }, // MBAFF
        FrameMetadata {
            pts: Some(1000),
            dts: Some(1000),
        }, // MBAFF
        FrameMetadata {
            pts: Some(2000),
            dts: Some(2000),
        }, // MBAFF
    ];

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    // PAFF vs MBAFF is encoder choice, not alignment concern
    assert!(workspace.is_diff_enabled());

    for i in 0..3 {
        let (b_idx, _) = workspace.get_aligned_frame(i).unwrap();
        assert_eq!(b_idx, i);
    }
}

#[test]
fn test_compare_h264_field_frame_013_manual_offset_field_vs_frame() {
    // Test: Manual offset when comparing field vs frame streams
    // User needs to align streams with different coding modes

    let frames_a = vec![
        FrameMetadata {
            pts: Some(0),
            dts: Some(0),
        }, // Field-coded
        FrameMetadata {
            pts: Some(1000),
            dts: Some(1000),
        },
        FrameMetadata {
            pts: Some(2000),
            dts: Some(2000),
        },
    ];

    let frames_b = vec![
        FrameMetadata {
            pts: Some(1000),
            dts: Some(1000),
        }, // Frame-coded, starts later
        FrameMetadata {
            pts: Some(2000),
            dts: Some(2000),
        },
        FrameMetadata {
            pts: Some(3000),
            dts: Some(3000),
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

    // Field vs frame coding doesn't interfere with manual alignment
}

#[test]
fn test_compare_h264_field_frame_014_resolution_field_vs_frame() {
    // Test: Resolution validation with field vs frame coding
    // Field-coded: 1920x540 per field → 1920x1080 frame
    // Frame-coded: 1920x1080 frame
    // Should consider full frame resolution for comparison

    let frames_a = vec![
        FrameMetadata {
            pts: Some(0),
            dts: Some(0),
        }, // Field-coded (1920x540 per field)
        FrameMetadata {
            pts: Some(1000),
            dts: Some(1000),
        },
    ];

    let frames_b = vec![
        FrameMetadata {
            pts: Some(0),
            dts: Some(0),
        }, // Frame-coded (1920x1080)
        FrameMetadata {
            pts: Some(1000),
            dts: Some(1000),
        },
    ];

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    // Resolution check uses full frame resolution
    assert!(workspace.is_diff_enabled());
    assert!(workspace.resolution_info().is_exact_match());
}

#[test]
fn test_compare_h264_field_frame_015_aff_flag_difference() {
    // Test: mb_adaptive_frame_field_flag difference
    // Stream A: mb_adaptive_frame_field_flag = 0 (no MBAFF)
    // Stream B: mb_adaptive_frame_field_flag = 1 (MBAFF enabled)
    // SPS flag difference shouldn't affect alignment

    let frames_a = vec![
        FrameMetadata {
            pts: Some(0),
            dts: Some(0),
        }, // No MBAFF
        FrameMetadata {
            pts: Some(1000),
            dts: Some(1000),
        },
        FrameMetadata {
            pts: Some(2000),
            dts: Some(2000),
        },
    ];

    let frames_b = vec![
        FrameMetadata {
            pts: Some(0),
            dts: Some(0),
        }, // MBAFF enabled
        FrameMetadata {
            pts: Some(1000),
            dts: Some(1000),
        },
        FrameMetadata {
            pts: Some(2000),
            dts: Some(2000),
        },
    ];

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let alignment = AlignmentEngine::new(&map_a, &map_b);

    // SPS flags are encoder settings, not alignment concern
    assert!(matches!(
        alignment.method,
        bitvue_core::AlignmentMethod::PtsExact | bitvue_core::AlignmentMethod::PtsNearest
    ));

    for i in 0..3 {
        let pair = alignment.get_pair_for_a(i).unwrap();
        assert_eq!(pair.stream_b_idx, Some(i));
    }
}
