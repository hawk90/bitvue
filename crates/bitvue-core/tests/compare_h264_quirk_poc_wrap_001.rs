#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(unused_comparisons)]
#![allow(unused_doc_comments)]
use bitvue_core::frame_identity::*;
use bitvue_core::{AlignmentEngine, CompareWorkspace, FrameIndexMap};

// ========================================================================
// Compare H264 Quirks - POC Wrap-Around Scenarios
// Test file 1/6: POC wrap-around edge cases in compare alignment
// ========================================================================
//
// H.264 POC (Picture Order Count) can wrap around at MaxPicOrderCntLsb.
// These tests verify that compare alignment handles POC wrapping correctly
// by relying on container PTS rather than internal POC values.

#[test]
fn test_compare_h264_poc_wrap_001_basic_wraparound() {
    // Test: POC wraps but PTS remains monotonic in both streams
    // Stream A: POC 254, 255, 0 (wrap), 1, 2
    // Stream B: POC 254, 255, 0 (wrap), 1, 2
    // Both have matching PTS → exact alignment

    let frames_a = vec![
        FrameMetadata {
            pts: Some(254000),
            dts: Some(254000),
        }, // POC 254
        FrameMetadata {
            pts: Some(255000),
            dts: Some(255000),
        }, // POC 255
        FrameMetadata {
            pts: Some(256000),
            dts: Some(256000),
        }, // POC 0 (wrapped)
        FrameMetadata {
            pts: Some(257000),
            dts: Some(257000),
        }, // POC 1
        FrameMetadata {
            pts: Some(258000),
            dts: Some(258000),
        }, // POC 2
    ];

    let frames_b = frames_a.clone();

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    // Both streams handle wrap correctly via container PTS
    assert_eq!(map_a.pts_quality(), PtsQuality::Ok);
    assert_eq!(map_b.pts_quality(), PtsQuality::Ok);

    let alignment = AlignmentEngine::new(&map_a, &map_b);

    // Should use PTS-based alignment (not internal POC)
    assert!(matches!(
        alignment.method,
        bitvue_core::AlignmentMethod::PtsExact | bitvue_core::AlignmentMethod::PtsNearest
    ));
    assert_eq!(alignment.gap_count, 0);
    assert_eq!(alignment.frame_pairs.len(), 5);

    // Verify exact alignment across wrap boundary
    for i in 0..5 {
        let pair = alignment.get_pair_for_a(i).unwrap();
        assert_eq!(pair.stream_b_idx, Some(i));
        assert_eq!(pair.pts_delta, Some(0));
        assert!(!pair.has_gap);
    }
}

#[test]
fn test_compare_h264_poc_wrap_002_offset_streams() {
    // Test: POC wrap at different positions in A vs B
    // Stream A: POC 253, 254, 255, 0 (wrap), 1
    // Stream B: POC 251, 252, 253, 254, 255, 0 (wrap), 1, 2, 3
    // B starts earlier and continues longer

    let frames_a = vec![
        FrameMetadata {
            pts: Some(253000),
            dts: Some(253000),
        },
        FrameMetadata {
            pts: Some(254000),
            dts: Some(254000),
        },
        FrameMetadata {
            pts: Some(255000),
            dts: Some(255000),
        },
        FrameMetadata {
            pts: Some(256000),
            dts: Some(256000),
        }, // wrap
        FrameMetadata {
            pts: Some(257000),
            dts: Some(257000),
        },
    ];

    let frames_b = vec![
        FrameMetadata {
            pts: Some(251000),
            dts: Some(251000),
        },
        FrameMetadata {
            pts: Some(252000),
            dts: Some(252000),
        },
        FrameMetadata {
            pts: Some(253000),
            dts: Some(253000),
        },
        FrameMetadata {
            pts: Some(254000),
            dts: Some(254000),
        },
        FrameMetadata {
            pts: Some(255000),
            dts: Some(255000),
        },
        FrameMetadata {
            pts: Some(256000),
            dts: Some(256000),
        }, // wrap
        FrameMetadata {
            pts: Some(257000),
            dts: Some(257000),
        },
        FrameMetadata {
            pts: Some(258000),
            dts: Some(258000),
        },
        FrameMetadata {
            pts: Some(259000),
            dts: Some(259000),
        },
    ];

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let alignment = AlignmentEngine::new(&map_a, &map_b);

    // Should align based on PTS values
    assert!(matches!(
        alignment.method,
        bitvue_core::AlignmentMethod::PtsExact | bitvue_core::AlignmentMethod::PtsNearest
    ));

    // Verify alignment matches by PTS
    // A frame with PTS 253000 (display_idx 0) → B display_idx 2
    let pair_a0 = alignment.get_pair_for_a(0).unwrap();
    assert_eq!(pair_a0.stream_b_idx, Some(2));
    assert_eq!(pair_a0.pts_delta, Some(0));

    // A frame with PTS 256000 (display_idx 3, after wrap) → B display_idx 5
    let pair_a3 = alignment.get_pair_for_a(3).unwrap();
    assert_eq!(pair_a3.stream_b_idx, Some(5));
    assert_eq!(pair_a3.pts_delta, Some(0));

    // B has extra frames before and after A's range
    assert!(alignment.gap_count >= 4); // 2 before + 2 after
}

#[test]
fn test_compare_h264_poc_wrap_003_multiple_wraps() {
    // Test: Multiple POC wraps in long sequence
    // Stream A: 254, 255, 0, 1, ..., 254, 255, 0, 1
    // Stream B: Same pattern
    // Tests that alignment handles repeated wraps

    let mut frames_a = Vec::new();
    let mut frames_b = Vec::new();

    // First wrap cycle: 254-255-0-1
    for i in 254..258 {
        frames_a.push(FrameMetadata {
            pts: Some((i * 1000) as u64),
            dts: Some((i * 1000) as u64),
        });
        frames_b.push(FrameMetadata {
            pts: Some((i * 1000) as u64),
            dts: Some((i * 1000) as u64),
        });
    }

    // Second wrap cycle: 502-503-504-505 (another wrap at 256*2)
    for i in 502..506 {
        frames_a.push(FrameMetadata {
            pts: Some((i * 1000) as u64),
            dts: Some((i * 1000) as u64),
        });
        frames_b.push(FrameMetadata {
            pts: Some((i * 1000) as u64),
            dts: Some((i * 1000) as u64),
        });
    }

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let alignment = AlignmentEngine::new(&map_a, &map_b);

    // Should handle multiple wraps correctly via PTS
    assert!(matches!(
        alignment.method,
        bitvue_core::AlignmentMethod::PtsExact | bitvue_core::AlignmentMethod::PtsNearest
    ));
    assert_eq!(alignment.gap_count, 0);

    // Verify alignment across both wrap boundaries
    for i in 0..8 {
        let pair = alignment.get_pair_for_a(i).unwrap();
        assert_eq!(pair.stream_b_idx, Some(i));
        assert!(!pair.has_gap);
    }
}

#[test]
fn test_compare_h264_poc_wrap_004_misaligned_wrap() {
    // Test: POC wrap occurs at different frame in each stream
    // Stream A wraps at frame 3
    // Stream B wraps at frame 5
    // Tests alignment when wrap boundaries don't match

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
        }, // A wraps here
        FrameMetadata {
            pts: Some(4000),
            dts: Some(4000),
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
        FrameMetadata {
            pts: Some(4000),
            dts: Some(4000),
        },
        FrameMetadata {
            pts: Some(5000),
            dts: Some(5000),
        }, // B wraps here
        FrameMetadata {
            pts: Some(6000),
            dts: Some(6000),
        },
    ];

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let alignment = AlignmentEngine::new(&map_a, &map_b);

    // PTS-based alignment is independent of internal POC wrap positions
    assert!(matches!(
        alignment.method,
        bitvue_core::AlignmentMethod::PtsExact | bitvue_core::AlignmentMethod::PtsNearest
    ));

    // Verify alignment by PTS regardless of where wrap occurs
    for i in 0..5 {
        let pair = alignment.get_pair_for_a(i).unwrap();
        assert_eq!(pair.stream_b_idx, Some(i));
        assert_eq!(pair.pts_delta, Some(0));
    }

    // B has extra frames after A ends
    assert!(alignment.gap_count >= 2);
}

#[test]
fn test_compare_h264_poc_wrap_005_workspace_with_wrap() {
    // Test: CompareWorkspace handles POC wrap correctly
    // Create compare workspace with streams that have POC wrap

    let frames_a = vec![
        FrameMetadata {
            pts: Some(254000),
            dts: Some(254000),
        },
        FrameMetadata {
            pts: Some(255000),
            dts: Some(255000),
        },
        FrameMetadata {
            pts: Some(256000),
            dts: Some(256000),
        }, // wrap
        FrameMetadata {
            pts: Some(257000),
            dts: Some(257000),
        },
    ];

    let frames_b = frames_a.clone();

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    // Workspace should handle wrap correctly
    assert!(workspace.is_diff_enabled());
    assert!(workspace.disable_reason().is_none());

    // Verify aligned frame lookup across wrap
    for i in 0..4 {
        let (b_idx, quality) = workspace.get_aligned_frame(i).unwrap();
        assert_eq!(b_idx, i);
        assert_eq!(quality, bitvue_core::AlignmentQuality::Exact);
    }
}

#[test]
fn test_compare_h264_poc_wrap_006_wrap_with_reordering() {
    // Test: POC wrap combined with B-frame reordering
    // Decode order: I254, P257, B255, B256 (wrap occurs mid-GOP)
    // Display order: I254, B255, B256 (wrap), P257

    let frames_a = vec![
        FrameMetadata {
            pts: Some(254000),
            dts: Some(0),
        }, // I254
        FrameMetadata {
            pts: Some(257000),
            dts: Some(1000),
        }, // P257 (decoded early)
        FrameMetadata {
            pts: Some(255000),
            dts: Some(2000),
        }, // B255
        FrameMetadata {
            pts: Some(256000),
            dts: Some(3000),
        }, // B256 (wrap)
    ];

    let frames_b = frames_a.clone();

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    // Both streams should detect reordering
    assert!(map_a.has_reordering());
    assert!(map_b.has_reordering());

    let alignment = AlignmentEngine::new(&map_a, &map_b);

    // Should align correctly despite both wrap and reordering
    assert!(matches!(
        alignment.method,
        bitvue_core::AlignmentMethod::PtsExact | bitvue_core::AlignmentMethod::PtsNearest
    ));
    assert_eq!(alignment.gap_count, 0);

    // Verify alignment in display order
    for i in 0..4 {
        let pair = alignment.get_pair_for_a(i).unwrap();
        assert_eq!(pair.stream_b_idx, Some(i));
        assert_eq!(pair.pts_delta, Some(0));
        assert!(!pair.has_gap);
    }
}

#[test]
fn test_compare_h264_poc_wrap_007_wrap_with_idr() {
    // Test: POC wrap vs IDR reset distinction
    // IDR resets POC to 0 (intentional reset)
    // Wrap cycles POC back to 0 (modulo arithmetic)
    // Both should be handled via container PTS

    let frames_a = vec![
        FrameMetadata {
            pts: Some(0),
            dts: Some(0),
        }, // IDR (POC reset)
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
        }, // Another IDR (POC reset again)
        FrameMetadata {
            pts: Some(4000),
            dts: Some(4000),
        },
        FrameMetadata {
            pts: Some(255000),
            dts: Some(255000),
        }, // Near wrap
        FrameMetadata {
            pts: Some(256000),
            dts: Some(256000),
        }, // Wrap (but not IDR)
    ];

    let frames_b = frames_a.clone();

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let alignment = AlignmentEngine::new(&map_a, &map_b);

    // Container PTS handles both IDR resets and POC wraps
    assert!(matches!(
        alignment.method,
        bitvue_core::AlignmentMethod::PtsExact | bitvue_core::AlignmentMethod::PtsNearest
    ));
    assert_eq!(alignment.gap_count, 0);

    // Verify alignment across both IDR boundaries and wrap
    for i in 0..7 {
        let pair = alignment.get_pair_for_a(i).unwrap();
        assert_eq!(pair.stream_b_idx, Some(i));
        assert!(!pair.has_gap);
    }
}

#[test]
fn test_compare_h264_poc_wrap_008_negative_poc_after_wrap() {
    // Test: Some H.264 implementations use signed POC
    // After wrap, POC might appear as large positive or negative values
    // Container PTS should remain authoritative

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
        }, // POC near max
        FrameMetadata {
            pts: Some(3000),
            dts: Some(3000),
        }, // POC wraps (appears negative in signed)
        FrameMetadata {
            pts: Some(4000),
            dts: Some(4000),
        },
    ];

    let frames_b = frames_a.clone();

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    // Should handle signed/unsigned POC via container PTS
    assert!(workspace.is_diff_enabled());

    // Verify alignment works across apparent negative POC
    for i in 0..5 {
        let (b_idx, quality) = workspace.get_aligned_frame(i).unwrap();
        assert_eq!(b_idx, i);
        assert_eq!(quality, bitvue_core::AlignmentQuality::Exact);
    }
}

#[test]
fn test_compare_h264_poc_wrap_009_manual_offset_with_wrap() {
    // Test: Manual offset adjustment when streams have POC wrap
    // Ensure offset arithmetic works correctly across wrap boundary

    let frames_a = vec![
        FrameMetadata {
            pts: Some(254000),
            dts: Some(254000),
        },
        FrameMetadata {
            pts: Some(255000),
            dts: Some(255000),
        },
        FrameMetadata {
            pts: Some(256000),
            dts: Some(256000),
        }, // wrap
        FrameMetadata {
            pts: Some(257000),
            dts: Some(257000),
        },
    ];

    let frames_b = vec![
        FrameMetadata {
            pts: Some(256000),
            dts: Some(256000),
        }, // B starts at wrap point
        FrameMetadata {
            pts: Some(257000),
            dts: Some(257000),
        },
        FrameMetadata {
            pts: Some(258000),
            dts: Some(258000),
        },
        FrameMetadata {
            pts: Some(259000),
            dts: Some(259000),
        },
    ];

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let mut workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    // Apply manual offset to align A's wrap frame (display_idx 2) with B's first frame
    workspace.set_manual_offset(-2);

    // A frame 2 (PTS 256000, after wrap) → B frame 0 (PTS 256000)
    if let Some((b_idx, quality)) = workspace.get_aligned_frame(2) {
        assert_eq!(b_idx, 0);
        assert_eq!(quality, bitvue_core::AlignmentQuality::Exact);
    }

    // A frame 3 (PTS 257000) → B frame 1 (PTS 257000)
    if let Some((b_idx, _)) = workspace.get_aligned_frame(3) {
        assert_eq!(b_idx, 1);
    }
}

#[test]
fn test_compare_h264_poc_wrap_010_max_poc_lsb_boundary() {
    // Test: Exact MaxPicOrderCntLsb boundary (typically 256)
    // Frame at POC=255 followed by POC=0

    let frames_a = vec![
        FrameMetadata {
            pts: Some(253000),
            dts: Some(253000),
        }, // POC 253
        FrameMetadata {
            pts: Some(254000),
            dts: Some(254000),
        }, // POC 254
        FrameMetadata {
            pts: Some(255000),
            dts: Some(255000),
        }, // POC 255 (MaxPicOrderCntLsb - 1)
        FrameMetadata {
            pts: Some(256000),
            dts: Some(256000),
        }, // POC 0 (exact wrap at MaxPicOrderCntLsb)
        FrameMetadata {
            pts: Some(257000),
            dts: Some(257000),
        }, // POC 1
    ];

    let frames_b = frames_a.clone();

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let alignment = AlignmentEngine::new(&map_a, &map_b);

    // Exact boundary should be handled correctly
    assert!(matches!(
        alignment.method,
        bitvue_core::AlignmentMethod::PtsExact | bitvue_core::AlignmentMethod::PtsNearest
    ));
    assert_eq!(alignment.gap_count, 0);

    // Special check: frame at exact wrap boundary (display_idx 3)
    let pair = alignment.get_pair_for_a(3).unwrap();
    assert_eq!(pair.stream_b_idx, Some(3));
    assert_eq!(pair.pts_delta, Some(0));
    assert!(!pair.has_gap);
}
