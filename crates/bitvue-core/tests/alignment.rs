#![allow(dead_code)]
//! Tests for alignment module

use bitvue_core::frame_identity::FrameMetadata;
use bitvue_core::{
    AlignmentConfidence, AlignmentEngine, AlignmentMethod, FrameIndexMap, FramePair, PtsQuality,
};

#[test]
fn test_alignment_pts_exact() {
    // Two streams with identical PTS
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
    ];

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let alignment = AlignmentEngine::new(&map_a, &map_b);

    assert_eq!(alignment.method, AlignmentMethod::PtsExact);
    assert_eq!(alignment.confidence, AlignmentConfidence::High);
    assert_eq!(alignment.gap_count, 0);
    assert_eq!(alignment.frame_pairs.len(), 3);

    for pair in &alignment.frame_pairs {
        assert!(pair.is_complete());
        assert!(!pair.has_gap);
        assert_eq!(pair.pts_delta, Some(0));
    }
}

#[test]
fn test_alignment_pts_nearest() {
    // Two streams with slightly offset PTS
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
    let frames_b = vec![
        FrameMetadata {
            pts: Some(5),
            dts: Some(0),
        }, // +5 offset
        FrameMetadata {
            pts: Some(1005),
            dts: Some(1000),
        }, // +5 offset
        FrameMetadata {
            pts: Some(2005),
            dts: Some(2000),
        }, // +5 offset
    ];

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let alignment = AlignmentEngine::new(&map_a, &map_b);

    assert_eq!(alignment.method, AlignmentMethod::PtsNearest);
    assert_eq!(alignment.confidence, AlignmentConfidence::High);
    assert_eq!(alignment.gap_count, 0);
    assert_eq!(alignment.frame_pairs.len(), 3);

    for pair in &alignment.frame_pairs {
        assert!(pair.is_complete());
        assert!(!pair.has_gap);
        assert_eq!(pair.pts_delta, Some(-5)); // A - B = -5
    }
}

#[test]
fn test_alignment_with_gaps() {
    // Stream B missing a frame
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
            pts: Some(0),
            dts: Some(0),
        },
        // Missing frame at 1000
        FrameMetadata {
            pts: Some(2000),
            dts: Some(1000),
        },
        FrameMetadata {
            pts: Some(3000),
            dts: Some(2000),
        },
    ];

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let alignment = AlignmentEngine::new(&map_a, &map_b);

    assert_eq!(alignment.method, AlignmentMethod::PtsNearest);
    assert_eq!(alignment.gap_count, 1);
    assert!(alignment.frame_pairs.len() >= 4);

    // First frame should match
    let pair0 = alignment.get_pair_for_a(0).unwrap();
    assert!(pair0.is_complete());
    assert!(!pair0.has_gap);

    // Second frame in A should have gap (no match in B)
    let pair1 = alignment.get_pair_for_a(1).unwrap();
    assert!(pair1.has_gap);
}

#[test]
fn test_alignment_fallback_to_display_idx() {
    // Stream with BAD PTS quality → fallback to display_idx
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
            pts: Some(1000),
            dts: Some(2000),
        }, // Duplicate PTS
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
    ];

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    // Stream A has duplicate PTS → BAD quality
    assert_eq!(map_a.pts_quality(), PtsQuality::Bad);

    let alignment = AlignmentEngine::new(&map_a, &map_b);

    // Should fall back to display_idx
    assert_eq!(alignment.method, AlignmentMethod::DisplayIdx);
    assert_eq!(alignment.frame_pairs.len(), 3);

    for (i, pair) in alignment.frame_pairs.iter().enumerate() {
        assert_eq!(pair.stream_a_idx, Some(i));
        assert_eq!(pair.stream_b_idx, Some(i));
        assert!(!pair.has_gap);
    }
}

#[test]
fn test_alignment_different_lengths() {
    // Stream A longer than B
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
            pts: Some(0),
            dts: Some(0),
        },
        FrameMetadata {
            pts: Some(1000),
            dts: Some(1000),
        },
    ];

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let alignment = AlignmentEngine::new(&map_a, &map_b);

    assert_eq!(alignment.stream_a_count, 4);
    assert_eq!(alignment.stream_b_count, 2);
    assert!(alignment.gap_count >= 2); // At least 2 gaps for unmatched A frames
}

#[test]
fn test_confidence_calculation() {
    // High confidence: PTS exact
    let frames = vec![
        FrameMetadata {
            pts: Some(0),
            dts: Some(0),
        },
        FrameMetadata {
            pts: Some(1000),
            dts: Some(1000),
        },
    ];
    let map = FrameIndexMap::new(&frames);
    let alignment = AlignmentEngine::new(&map, &map);
    assert_eq!(alignment.confidence, AlignmentConfidence::High);

    // Low confidence: display_idx fallback
    let frames_bad = vec![
        FrameMetadata {
            pts: Some(0),
            dts: Some(0),
        },
        FrameMetadata {
            pts: Some(0),
            dts: Some(1000),
        }, // Duplicate
    ];
    let map_bad = FrameIndexMap::new(&frames_bad);
    let alignment_bad = AlignmentEngine::new(&map_bad, &map);
    assert_eq!(alignment_bad.method, AlignmentMethod::DisplayIdx);
    assert_eq!(alignment_bad.confidence, AlignmentConfidence::Medium);
}

#[test]
fn test_gap_percentage() {
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
            pts: Some(0),
            dts: Some(0),
        },
        FrameMetadata {
            pts: Some(2000),
            dts: Some(1000),
        },
    ];

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let alignment = AlignmentEngine::new(&map_a, &map_b);

    let gap_pct = alignment.gap_percentage();
    assert!(gap_pct > 0.0);
    assert!(gap_pct <= 100.0);
}

#[test]
fn test_frame_pair_helpers() {
    let pair_complete = FramePair {
        stream_a_idx: Some(0),
        stream_b_idx: Some(1),
        pts_delta: Some(-5),
        has_gap: false,
    };
    assert!(pair_complete.is_complete());
    assert_eq!(pair_complete.pts_delta_abs(), Some(5));

    let pair_gap = FramePair {
        stream_a_idx: Some(0),
        stream_b_idx: None,
        pts_delta: None,
        has_gap: true,
    };
    assert!(!pair_gap.is_complete());
    assert_eq!(pair_gap.pts_delta_abs(), None);
}
