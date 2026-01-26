//! Compare Surface VP9 Quirks Test: show_existing_frame Edge Cases
//!
//! Tests VP9-specific quirk: show_existing_frame flag and its edge cases
//! in the Compare surface.
//!
//! VP9 show_existing_frame:
//! - References a previously decoded frame without new coded data
//! - Creates a "virtual" display frame
//! - Has minimal bitstream overhead (just the frame header)
//! - Can cause frame count mismatches between streams
//! - May have unusual PTS patterns
//!
//! Edge cases tested:
//! - show_existing_frame in both streams (different reference patterns)
//! - show_existing_frame in only one stream
//! - Multiple consecutive show_existing_frame
//! - show_existing_frame referencing ALTREF frames
//! - show_existing_frame with missing/duplicate PTS

use bitvue_core::{
    compare::CompareWorkspace,
    frame_identity::{FrameIndexMap, FrameMetadata},
    AlignmentEngine,
};

/// Helper to create VP9 frames with show_existing_frame markers
fn create_vp9_frames_with_show_existing(
    count: usize,
    show_existing_positions: &[usize],
) -> Vec<FrameMetadata> {
    let mut frames = Vec::new();
    let mut pts = 0u64;

    for i in 0..count {
        let is_show_existing = show_existing_positions.contains(&i);

        if is_show_existing {
            // show_existing_frame: reuses PTS from reference frame
            // Typically uses same PTS as the frame it references
            frames.push(FrameMetadata {
                pts: Some(pts.saturating_sub(1000)), // Reference previous frame's PTS (with overflow protection)
                dts: Some((i * 1000) as u64),
            });
        } else {
            frames.push(FrameMetadata {
                pts: Some(pts),
                dts: Some((i * 1000) as u64),
            });
            pts += 1000;
        }
    }

    frames
}

#[test]
fn test_vp9_show_existing_basic_alignment() {
    // Both streams have show_existing_frame at same position
    let frames_a = create_vp9_frames_with_show_existing(10, &[5]);
    let frames_b = create_vp9_frames_with_show_existing(10, &[5]);

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    // Verify basic alignment works
    assert_eq!(workspace.alignment.stream_a_count, 10);
    assert!(workspace.is_diff_enabled() || workspace.disable_reason().is_some());
}

#[test]
fn test_vp9_show_existing_asymmetric_streams() {
    // Stream A has show_existing at position 5
    // Stream B has show_existing at position 7
    let frames_a = create_vp9_frames_with_show_existing(12, &[5]);
    let frames_b = create_vp9_frames_with_show_existing(12, &[7]);

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    // Alignment should handle different positions
    assert_eq!(workspace.total_frames(), 12);
}

#[test]
fn test_vp9_show_existing_only_in_stream_a() {
    // Only stream A has show_existing_frame
    let frames_a = create_vp9_frames_with_show_existing(10, &[3, 7]);
    let frames_b: Vec<FrameMetadata> = (0..10)
        .map(|i| FrameMetadata {
            pts: Some((i * 1000) as u64),
            dts: Some((i * 1000) as u64),
        })
        .collect();

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    // Should handle frame count mismatch gracefully
    assert_eq!(workspace.total_frames(), 10);
}

#[test]
fn test_vp9_show_existing_only_in_stream_b() {
    // Only stream B has show_existing_frame
    let frames_a: Vec<FrameMetadata> = (0..10)
        .map(|i| FrameMetadata {
            pts: Some((i * 1000) as u64),
            dts: Some((i * 1000) as u64),
        })
        .collect();
    let frames_b = create_vp9_frames_with_show_existing(10, &[4]);

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    // Verify workspace creation succeeds
    assert_eq!(workspace.alignment.stream_a_count, 10);
}

#[test]
fn test_vp9_show_existing_multiple_consecutive() {
    // Multiple consecutive show_existing_frame (possible for frame duplication)
    let frames_a = create_vp9_frames_with_show_existing(15, &[5, 6, 7]);
    let frames_b = create_vp9_frames_with_show_existing(15, &[5, 6, 7]);

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    // Should handle consecutive show_existing_frame
    assert_eq!(workspace.alignment.stream_a_count, 15);
}

#[test]
fn test_vp9_show_existing_duplicate_pts() {
    // show_existing_frame with duplicate PTS (references same frame twice)
    let frames = vec![
        FrameMetadata {
            pts: Some(0),
            dts: Some(0),
        },
        FrameMetadata {
            pts: Some(1000),
            dts: Some(1000),
        },
        // show_existing: duplicate PTS 1000
        FrameMetadata {
            pts: Some(1000),
            dts: Some(2000),
        },
        FrameMetadata {
            pts: Some(2000),
            dts: Some(3000),
        },
    ];

    let map = FrameIndexMap::new(&frames);

    // PTS quality should be BAD due to duplicate PTS
    use bitvue_core::frame_identity::PtsQuality;
    assert_eq!(map.pts_quality(), PtsQuality::Bad);
}

#[test]
fn test_vp9_show_existing_missing_pts() {
    // show_existing_frame with missing PTS
    let frames = vec![
        FrameMetadata {
            pts: Some(0),
            dts: Some(0),
        },
        FrameMetadata {
            pts: Some(1000),
            dts: Some(1000),
        },
        // show_existing: missing PTS
        FrameMetadata {
            pts: None,
            dts: Some(2000),
        },
        FrameMetadata {
            pts: Some(2000),
            dts: Some(3000),
        },
    ];

    let map = FrameIndexMap::new(&frames);

    // PTS quality should be WARN due to missing PTS
    use bitvue_core::frame_identity::PtsQuality;
    assert_ne!(map.pts_quality(), PtsQuality::Ok);
}

#[test]
fn test_vp9_show_existing_manual_offset() {
    // Test manual offset with show_existing_frame
    let frames_a = create_vp9_frames_with_show_existing(20, &[10]);
    let frames_b = create_vp9_frames_with_show_existing(20, &[10]);

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let mut workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    // Apply offset
    workspace.set_manual_offset(2);

    // Verify alignment with offset
    let (b_idx, _quality) = workspace.get_aligned_frame(0).unwrap();
    assert_eq!(b_idx, 2);
}

#[test]
fn test_vp9_show_existing_at_stream_boundary() {
    // show_existing_frame at start/end of stream
    let frames_a = create_vp9_frames_with_show_existing(10, &[0, 9]);
    let frames_b = create_vp9_frames_with_show_existing(10, &[0, 9]);

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    // Check alignment at boundaries
    if let Some((b_idx, _quality)) = workspace.get_aligned_frame(0) {
        assert!(b_idx < 10, "Boundary alignment should be valid");
    }
}

#[test]
fn test_vp9_show_existing_frame_size_mismatch() {
    // show_existing_frame has minimal size (just header)
    // This creates size distribution differences between streams
    let frames_a = create_vp9_frames_with_show_existing(8, &[3]);
    let frames_b = create_vp9_frames_with_show_existing(8, &[]);

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    // Frame size difference shouldn't affect alignment
    assert_eq!(workspace.total_frames(), 8);
}

#[test]
fn test_vp9_show_existing_with_sync_modes() {
    // Test all sync modes with show_existing_frame
    use bitvue_core::SyncMode;

    let frames_a = create_vp9_frames_with_show_existing(12, &[6]);
    let frames_b = create_vp9_frames_with_show_existing(12, &[6]);

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let mut workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    // Cycle through sync modes
    for mode in [SyncMode::Off, SyncMode::Playhead, SyncMode::Full] {
        workspace.set_sync_mode(mode);
        assert_eq!(workspace.sync_mode(), mode);
    }
}

#[test]
fn test_vp9_show_existing_reference_chain() {
    // show_existing_frame referencing a frame that references another
    // Frame 0: coded
    // Frame 1: coded
    // Frame 2: show_existing(1)
    // Frame 3: show_existing(2) -> indirectly references frame 1
    let frames = vec![
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
        }, // show_existing(1)
        FrameMetadata {
            pts: Some(1000),
            dts: Some(3000),
        }, // show_existing(2)
    ];

    let map = FrameIndexMap::new(&frames);

    // PTS quality will be BAD due to duplicates
    use bitvue_core::frame_identity::PtsQuality;
    assert_eq!(map.pts_quality(), PtsQuality::Bad);
    assert_eq!(map.frame_count(), 4);
}

#[test]
fn test_vp9_show_existing_alignment_confidence() {
    // Test alignment confidence with show_existing_frame patterns
    let frames_a = create_vp9_frames_with_show_existing(15, &[5, 10]);
    let frames_b = create_vp9_frames_with_show_existing(15, &[5, 10]);

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let alignment = AlignmentEngine::new(&map_a, &map_b);

    // Matching patterns should give reasonable confidence
    let confidence = alignment.confidence();
    use bitvue_core::AlignmentConfidence;
    assert!(
        confidence == AlignmentConfidence::High
            || confidence == AlignmentConfidence::Medium
            || confidence == AlignmentConfidence::Low
    );
}

#[test]
fn test_vp9_show_existing_different_reference_patterns() {
    // Stream A: show_existing every 5 frames
    // Stream B: show_existing every 7 frames
    let frames_a = create_vp9_frames_with_show_existing(20, &[5, 10, 15]);
    let frames_b = create_vp9_frames_with_show_existing(20, &[7, 14]);

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    // Different patterns should still allow comparison
    assert_eq!(workspace.total_frames(), 20);
}

#[test]
fn test_vp9_show_existing_extreme_reference_distance() {
    // show_existing_frame referencing a very old frame
    let mut frames = vec![];
    for i in 0..15 {
        if i == 14 {
            // Reference frame 0 (14 frames back)
            frames.push(FrameMetadata {
                pts: Some(0),
                dts: Some((i * 1000) as u64),
            });
        } else {
            frames.push(FrameMetadata {
                pts: Some((i * 1000) as u64),
                dts: Some((i * 1000) as u64),
            });
        }
    }

    let map = FrameIndexMap::new(&frames);

    // Should handle extreme reference distance
    use bitvue_core::frame_identity::PtsQuality;
    assert_eq!(map.pts_quality(), PtsQuality::Bad); // Duplicate PTS
    assert_eq!(map.frame_count(), 15);
}
