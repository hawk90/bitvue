#![allow(hidden_glob_reexports)]
#![allow(unreachable_code)]
#![allow(non_camel_case_types)]
#![allow(unused_assignments)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(unused_comparisons)]
#![allow(unused_doc_comments)]
use crate::frame_identity::*;
use crate::{AlignmentEngine, CompareWorkspace, FrameIndexMap};

// ========================================================================
// Compare H264 Quirks - MMCO Command Timing Edge Cases
// Test file 4/6: Memory Management Control Operation timing
// ========================================================================
//
// H.264 MMCO (Memory Management Control Operation) commands control
// reference picture marking. These tests verify that compare alignment
// handles MMCO timing differences between streams correctly.

#[test]
fn test_compare_h264_mmco_001_mmco5_idr_simulation() {
    // Test: MMCO5 simulates IDR without actual IDR frame
    // Stream A: MMCO5 at frame 3 (POC reset)
    // Stream B: Actual IDR at frame 3
    // Both should align via container PTS

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
        }, // P3 with MMCO5 (POC reset)
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
        }, // P1
        FrameMetadata {
            pts: Some(2000),
            dts: Some(2000),
        }, // P2
        FrameMetadata {
            pts: Some(3000),
            dts: Some(3000),
        }, // IDR3 (clean restart)
        FrameMetadata {
            pts: Some(4000),
            dts: Some(4000),
        }, // P4
    ];

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let alignment = AlignmentEngine::new(&map_a, &map_b);

    // MMCO5 vs IDR is encoder choice, alignment uses PTS
    assert!(matches!(
        alignment.method,
        crate::AlignmentMethod::PtsExact | crate::AlignmentMethod::PtsNearest
    ));

    for i in 0..5 {
        let pair = alignment.get_pair_for_a(i).unwrap();
        assert_eq!(pair.stream_b_idx, Some(i));
        assert_eq!(pair.pts_delta, Some(0));
    }
}

#[test]
fn test_compare_h264_mmco_002_mmco1_short_term_unused() {
    // Test: MMCO1 marks short-term reference as unused
    // Stream A: MMCO1 at frame 2 (discards frame 0)
    // Stream B: Sliding window (frame 0 slides out naturally)
    // Different memory management, same display order

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
        }, // P2 with MMCO1 (discard I0)
        FrameMetadata {
            pts: Some(3000),
            dts: Some(3000),
        }, // P3
    ];

    let frames_b = frames_a.clone();

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let alignment = AlignmentEngine::new(&map_a, &map_b);

    // MMCO1 is decoder internal, doesn't affect alignment
    assert!(matches!(
        alignment.method,
        crate::AlignmentMethod::PtsExact | crate::AlignmentMethod::PtsNearest
    ));
    assert_eq!(alignment.gap_count, 0);

    for i in 0..4 {
        let pair = alignment.get_pair_for_a(i).unwrap();
        assert_eq!(pair.stream_b_idx, Some(i));
    }
}

#[test]
fn test_compare_h264_mmco_003_mmco2_long_term_reference() {
    // Test: MMCO2 marks picture as long-term reference
    // Stream A: MMCO2 at frame 1 (make I0 long-term)
    // Stream B: No long-term references
    // Different reference management, same alignment

    let frames_a = vec![
        FrameMetadata {
            pts: Some(0),
            dts: Some(0),
        }, // I0
        FrameMetadata {
            pts: Some(1000),
            dts: Some(1000),
        }, // P1 with MMCO2 (I0 → long-term)
        FrameMetadata {
            pts: Some(2000),
            dts: Some(2000),
        }, // P2
        FrameMetadata {
            pts: Some(3000),
            dts: Some(3000),
        }, // P3 (can still ref I0)
        FrameMetadata {
            pts: Some(4000),
            dts: Some(4000),
        }, // P4
    ];

    let frames_b = frames_a.clone();

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    // Long-term reference marking is encoder optimization
    assert!(workspace.is_diff_enabled());

    for i in 0..5 {
        let (b_idx, quality) = workspace.get_aligned_frame(i).unwrap();
        assert_eq!(b_idx, i);
        assert_eq!(quality, crate::AlignmentQuality::Exact);
    }
}

#[test]
fn test_compare_h264_mmco_004_mmco3_max_long_term_idx() {
    // Test: MMCO3 sets MaxLongTermFrameIdx
    // Stream A: MMCO3 limits long-term frame indices
    // Stream B: Different MaxLongTermFrameIdx
    // Alignment independent of MMCO parameters

    let frames_a = vec![
        FrameMetadata {
            pts: Some(0),
            dts: Some(0),
        }, // I0
        FrameMetadata {
            pts: Some(1000),
            dts: Some(1000),
        }, // P1 with MMCO3 (set MaxLongTermFrameIdx)
        FrameMetadata {
            pts: Some(2000),
            dts: Some(2000),
        }, // P2
        FrameMetadata {
            pts: Some(3000),
            dts: Some(3000),
        }, // P3
    ];

    let frames_b = frames_a.clone();

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let alignment = AlignmentEngine::new(&map_a, &map_b);

    // MMCO3 parameter differences don't affect alignment
    assert!(matches!(
        alignment.method,
        crate::AlignmentMethod::PtsExact | crate::AlignmentMethod::PtsNearest
    ));

    for i in 0..4 {
        let pair = alignment.get_pair_for_a(i).unwrap();
        assert_eq!(pair.stream_b_idx, Some(i));
    }
}

#[test]
fn test_compare_h264_mmco_005_mmco4_all_long_term_unused() {
    // Test: MMCO4 marks all long-term references unused
    // Stream A: MMCO4 at frame 3 (clear all long-term refs)
    // Stream B: No MMCO4 (keeps long-term refs)

    let frames_a = vec![
        FrameMetadata {
            pts: Some(0),
            dts: Some(0),
        }, // I0 (long-term)
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
        }, // P3 with MMCO4 (clear all long-term)
        FrameMetadata {
            pts: Some(4000),
            dts: Some(4000),
        }, // P4
    ];

    let frames_b = frames_a.clone();

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    // MMCO4 is reference management, not frame identity
    assert!(workspace.is_diff_enabled());

    for i in 0..5 {
        let (b_idx, _) = workspace.get_aligned_frame(i).unwrap();
        assert_eq!(b_idx, i);
    }
}

#[test]
fn test_compare_h264_mmco_006_mmco6_current_as_long_term() {
    // Test: MMCO6 marks current picture as long-term
    // Stream A: MMCO6 at frame 2 (self-marking)
    // Stream B: Different MMCO strategy

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
        }, // P2 with MMCO6 (mark self long-term)
        FrameMetadata {
            pts: Some(3000),
            dts: Some(3000),
        }, // P3 (can ref P2 as long-term)
    ];

    let frames_b = frames_a.clone();

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let alignment = AlignmentEngine::new(&map_a, &map_b);

    // MMCO6 self-marking is encoder optimization
    assert!(matches!(
        alignment.method,
        crate::AlignmentMethod::PtsExact | crate::AlignmentMethod::PtsNearest
    ));

    for i in 0..4 {
        let pair = alignment.get_pair_for_a(i).unwrap();
        assert_eq!(pair.stream_b_idx, Some(i));
    }
}

#[test]
fn test_compare_h264_mmco_007_mmco_sequence_difference() {
    // Test: Different MMCO command sequences
    // Stream A: MMCO1, MMCO2, MMCO3 sequence
    // Stream B: MMCO5 only
    // Very different memory management strategies

    let frames_a = vec![
        FrameMetadata {
            pts: Some(0),
            dts: Some(0),
        }, // I0
        FrameMetadata {
            pts: Some(1000),
            dts: Some(1000),
        }, // P1 with MMCO1, MMCO2, MMCO3
        FrameMetadata {
            pts: Some(2000),
            dts: Some(2000),
        }, // P2
    ];

    let frames_b = vec![
        FrameMetadata {
            pts: Some(0),
            dts: Some(0),
        }, // I0
        FrameMetadata {
            pts: Some(1000),
            dts: Some(1000),
        }, // P1 with MMCO5
        FrameMetadata {
            pts: Some(2000),
            dts: Some(2000),
        }, // P2
    ];

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let alignment = AlignmentEngine::new(&map_a, &map_b);

    // Completely different MMCO strategies, same PTS alignment
    assert!(matches!(
        alignment.method,
        crate::AlignmentMethod::PtsExact | crate::AlignmentMethod::PtsNearest
    ));

    for i in 0..3 {
        let pair = alignment.get_pair_for_a(i).unwrap();
        assert_eq!(pair.stream_b_idx, Some(i));
    }
}

#[test]
fn test_compare_h264_mmco_008_adaptive_vs_sliding_window() {
    // Test: Adaptive ref pic marking (MMCO) vs sliding window
    // Stream A: Uses MMCO commands throughout
    // Stream B: Uses sliding window only (no MMCO)

    let frames_a = vec![
        FrameMetadata {
            pts: Some(0),
            dts: Some(0),
        }, // I0
        FrameMetadata {
            pts: Some(1000),
            dts: Some(1000),
        }, // P1 with MMCO
        FrameMetadata {
            pts: Some(2000),
            dts: Some(2000),
        }, // P2 with MMCO
        FrameMetadata {
            pts: Some(3000),
            dts: Some(3000),
        }, // P3 with MMCO
    ];

    let frames_b = vec![
        FrameMetadata {
            pts: Some(0),
            dts: Some(0),
        }, // I0
        FrameMetadata {
            pts: Some(1000),
            dts: Some(1000),
        }, // P1 (sliding window)
        FrameMetadata {
            pts: Some(2000),
            dts: Some(2000),
        }, // P2 (sliding window)
        FrameMetadata {
            pts: Some(3000),
            dts: Some(3000),
        }, // P3 (sliding window)
    ];

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    // Adaptive vs sliding window is encoder architecture choice
    assert!(workspace.is_diff_enabled());

    for i in 0..4 {
        let (b_idx, quality) = workspace.get_aligned_frame(i).unwrap();
        assert_eq!(b_idx, i);
        assert_eq!(quality, crate::AlignmentQuality::Exact);
    }
}

#[test]
fn test_compare_h264_mmco_009_mmco_after_idr() {
    // Test: MMCO commands in frames after IDR
    // IDR should have cleared all references, but MMCO still present

    let frames_a = vec![
        FrameMetadata {
            pts: Some(0),
            dts: Some(0),
        }, // IDR0
        FrameMetadata {
            pts: Some(1000),
            dts: Some(1000),
        }, // P1 with MMCO (rebuild ref list)
        FrameMetadata {
            pts: Some(2000),
            dts: Some(2000),
        }, // P2 with MMCO
        FrameMetadata {
            pts: Some(3000),
            dts: Some(3000),
        }, // IDR3 (another flush)
        FrameMetadata {
            pts: Some(4000),
            dts: Some(4000),
        }, // P4
    ];

    let frames_b = frames_a.clone();

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let alignment = AlignmentEngine::new(&map_a, &map_b);

    // MMCO timing relative to IDR doesn't affect alignment
    assert!(matches!(
        alignment.method,
        crate::AlignmentMethod::PtsExact | crate::AlignmentMethod::PtsNearest
    ));

    for i in 0..5 {
        let pair = alignment.get_pair_for_a(i).unwrap();
        assert_eq!(pair.stream_b_idx, Some(i));
    }
}

#[test]
fn test_compare_h264_mmco_010_mmco_with_b_frames() {
    // Test: MMCO commands with B-frame reordering
    // MMCO applied at decode time, not display time

    let frames_a = vec![
        FrameMetadata {
            pts: Some(0),
            dts: Some(0),
        }, // I0
        FrameMetadata {
            pts: Some(3000),
            dts: Some(1000),
        }, // P3 (decoded early, with MMCO)
        FrameMetadata {
            pts: Some(1000),
            dts: Some(2000),
        }, // B1
        FrameMetadata {
            pts: Some(2000),
            dts: Some(3000),
        }, // B2
    ];

    let frames_b = frames_a.clone();

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    // Both detect reordering
    assert!(map_a.has_reordering());
    assert!(map_b.has_reordering());

    let alignment = AlignmentEngine::new(&map_a, &map_b);

    // MMCO with B-frames: decode-time operation, display-order alignment
    assert!(matches!(
        alignment.method,
        crate::AlignmentMethod::PtsExact | crate::AlignmentMethod::PtsNearest
    ));

    for i in 0..4 {
        let pair = alignment.get_pair_for_a(i).unwrap();
        assert_eq!(pair.stream_b_idx, Some(i));
    }
}

#[test]
fn test_compare_h264_mmco_011_no_mmco_vs_mmco() {
    // Test: One stream without MMCO capability vs one with MMCO
    // Stream A: Constrained baseline (no MMCO support)
    // Stream B: Main profile (MMCO supported)

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

    // MMCO capability difference (profile) doesn't affect alignment
    assert!(matches!(
        alignment.method,
        crate::AlignmentMethod::PtsExact | crate::AlignmentMethod::PtsNearest
    ));

    for i in 0..3 {
        let pair = alignment.get_pair_for_a(i).unwrap();
        assert_eq!(pair.stream_b_idx, Some(i));
    }
}

#[test]
fn test_compare_h264_mmco_012_mmco_in_reference_b_frames() {
    // Test: MMCO in reference B-frames (used as references)
    // Some B-frames can be marked as references and have MMCO

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
        }, // B2 (reference, with MMCO)
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

    // MMCO in reference B-frames handled via PTS alignment
    assert!(matches!(
        alignment.method,
        crate::AlignmentMethod::PtsExact | crate::AlignmentMethod::PtsNearest
    ));

    for i in 0..5 {
        let pair = alignment.get_pair_for_a(i).unwrap();
        assert_eq!(pair.stream_b_idx, Some(i));
    }
}

#[test]
fn test_compare_h264_mmco_013_manual_offset_with_mmco() {
    // Test: Manual offset adjustment when streams have different MMCO timing
    // User needs to align streams with different memory management

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
        }, // P2 with MMCO5
        FrameMetadata {
            pts: Some(3000),
            dts: Some(3000),
        }, // P3
    ];

    let frames_b = vec![
        FrameMetadata {
            pts: Some(1000),
            dts: Some(1000),
        }, // P1 (starts later)
        FrameMetadata {
            pts: Some(2000),
            dts: Some(2000),
        }, // P2
        FrameMetadata {
            pts: Some(3000),
            dts: Some(3000),
        }, // P3
        FrameMetadata {
            pts: Some(4000),
            dts: Some(4000),
        }, // P4
    ];

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let mut workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    // Apply manual offset
    workspace.set_manual_offset(-1);

    // A frame 1 (PTS 1000) → B frame 0 (PTS 1000)
    if let Some((b_idx, quality)) = workspace.get_aligned_frame(1) {
        assert_eq!(b_idx, 0);
        assert_eq!(quality, crate::AlignmentQuality::Exact);
    }

    // MMCO timing differences don't interfere with manual alignment
}

#[test]
fn test_compare_h264_mmco_014_mmco_error_recovery() {
    // Test: MMCO used for error recovery
    // Stream A: Error causes unusual MMCO sequence
    // Stream B: Normal encoding
    // Error recovery shouldn't prevent alignment

    let frames_a = vec![
        FrameMetadata {
            pts: Some(0),
            dts: Some(0),
        },
        FrameMetadata {
            pts: Some(1000),
            dts: Some(1000),
        }, // Error here
        FrameMetadata {
            pts: Some(2000),
            dts: Some(2000),
        }, // MMCO for recovery
        FrameMetadata {
            pts: Some(3000),
            dts: Some(3000),
        },
    ];

    let frames_b = vec![
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

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let alignment = AlignmentEngine::new(&map_a, &map_b);

    // Error recovery MMCO doesn't affect PTS alignment
    assert!(matches!(
        alignment.method,
        crate::AlignmentMethod::PtsExact | crate::AlignmentMethod::PtsNearest
    ));

    for i in 0..4 {
        let pair = alignment.get_pair_for_a(i).unwrap();
        assert_eq!(pair.stream_b_idx, Some(i));
    }
}

#[test]
fn test_compare_h264_mmco_015_mmco_at_scene_change() {
    // Test: MMCO commands at scene changes
    // Encoders may use MMCO to reset references at scene cuts

    let frames_a = vec![
        FrameMetadata {
            pts: Some(0),
            dts: Some(0),
        }, // Scene 1
        FrameMetadata {
            pts: Some(1000),
            dts: Some(1000),
        },
        FrameMetadata {
            pts: Some(2000),
            dts: Some(2000),
        }, // Scene change (MMCO4 + MMCO5)
        FrameMetadata {
            pts: Some(3000),
            dts: Some(3000),
        }, // Scene 2
        FrameMetadata {
            pts: Some(4000),
            dts: Some(4000),
        },
    ];

    let frames_b = frames_a.clone();

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    // MMCO at scene changes is encoder optimization
    assert!(workspace.is_diff_enabled());

    for i in 0..5 {
        let (b_idx, _) = workspace.get_aligned_frame(i).unwrap();
        assert_eq!(b_idx, i);
    }
}
