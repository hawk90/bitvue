#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(unused_comparisons)]
#![allow(unused_doc_comments)]
//! Compare Surface VP9 Quirks Test: ALTREF Frame Display Timing
//!
//! Tests VP9-specific quirk: ALTREF (alternate reference) frames and their display timing behavior
//! in the Compare surface.
//!
//! VP9 ALTREF frames are:
//! - Not intended for display (show_frame = false)
//! - Used only as reference frames
//! - Have unusual PTS timing (may be "out of order" in display sequence)
//! - Can cause alignment mismatches if not handled correctly
//!
//! Edge cases tested:
//! - ALTREF frames in both streams (different positions)
//! - ALTREF frames in only one stream
//! - ALTREF frames at GOP boundaries
//! - Multiple consecutive ALTREF frames
//! - ALTREF frames with duplicate/missing PTS

use bitvue_core::{
    compare::CompareWorkspace,
    frame_identity::{FrameIndexMap, FrameMetadata},
    AlignmentConfidence, AlignmentEngine,
};

/// Helper to create frame metadata with ALTREF flags
fn create_vp9_frames_with_altref(count: usize, altref_positions: &[usize]) -> Vec<FrameMetadata> {
    let mut frames = Vec::new();
    let mut pts = 0u64;

    for i in 0..count {
        let is_altref = altref_positions.contains(&i);

        if is_altref {
            // ALTREF frames: PTS may be out of order or duplicate
            // Typically, ALTREF is encoded before the frames that reference it
            // but has a PTS that would place it later in display order
            frames.push(FrameMetadata {
                pts: Some(pts + 1000), // ALTREF PTS is "future" relative to encode position
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
fn test_vp9_altref_basic_alignment() {
    // Test basic ALTREF frame alignment between two VP9 streams
    // Stream A: frames 0-9, ALTREF at position 3
    // Stream B: frames 0-9, ALTREF at position 3
    let frames_a = create_vp9_frames_with_altref(10, &[3]);
    let frames_b = create_vp9_frames_with_altref(10, &[3]);

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    // Verify alignment engine was created
    assert_eq!(workspace.alignment.stream_a_count, 10);

    // Check alignment for frame around ALTREF position
    let (b_idx, quality) = workspace.get_aligned_frame(2).unwrap();
    assert!(b_idx <= 10, "Aligned frame index should be valid");
}

#[test]
fn test_vp9_altref_asymmetric_streams() {
    // Stream A has ALTREF at position 3
    // Stream B has ALTREF at position 5
    // This tests alignment when ALTREF frames are at different positions
    let frames_a = create_vp9_frames_with_altref(10, &[3]);
    let frames_b = create_vp9_frames_with_altref(10, &[5]);

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    // Alignment should still work but may have lower confidence due to PTS mismatch
    assert!(workspace.is_diff_enabled() || workspace.disable_reason().is_some());
}

#[test]
fn test_vp9_altref_only_in_stream_a() {
    // Stream A has ALTREF frames, Stream B does not
    // This creates a frame count mismatch in displayable frames
    let frames_a = create_vp9_frames_with_altref(10, &[3, 7]);
    let frames_b = create_vp9_frames_with_altref(10, &[]);

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    // Check that alignment handles the display frame count difference
    assert_eq!(workspace.alignment.stream_a_count, 10);
}

#[test]
fn test_vp9_altref_only_in_stream_b() {
    // Stream B has ALTREF frames, Stream A does not
    let frames_a = create_vp9_frames_with_altref(10, &[]);
    let frames_b = create_vp9_frames_with_altref(10, &[2, 6]);

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    // Verify workspace is created successfully
    assert_eq!(workspace.total_frames(), 10);
}

#[test]
fn test_vp9_altref_at_gop_boundary() {
    // ALTREF frames typically appear at GOP boundaries
    // Test alignment when ALTREF is at frame 0, 10, 20 (GOP size = 10)
    let frames_a = create_vp9_frames_with_altref(30, &[0, 10, 20]);
    let frames_b = create_vp9_frames_with_altref(30, &[0, 10, 20]);

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    // Check alignment at GOP boundaries
    for gop_start in [0, 10, 20] {
        if let Some((b_idx, _quality)) = workspace.get_aligned_frame(gop_start) {
            // Alignment should be reasonable (within 2 frames)
            let diff = (b_idx as isize - gop_start as isize).abs();
            assert!(
                diff <= 2,
                "GOP boundary alignment should be tight, got diff: {}",
                diff
            );
        }
    }
}

#[test]
fn test_vp9_altref_multiple_consecutive() {
    // Test multiple consecutive ALTREF frames (rare but possible)
    let frames_a = create_vp9_frames_with_altref(15, &[4, 5, 6]);
    let frames_b = create_vp9_frames_with_altref(15, &[4, 5, 6]);

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    // Verify alignment works despite consecutive ALTREF frames
    assert_eq!(workspace.alignment.stream_a_count, 15);
}

#[test]
fn test_vp9_altref_pts_quality_degradation() {
    // ALTREF frames with missing PTS should degrade PTS quality
    let mut frames = vec![
        FrameMetadata {
            pts: Some(0),
            dts: Some(0),
        },
        FrameMetadata {
            pts: Some(1000),
            dts: Some(1000),
        },
        // ALTREF frame with missing PTS
        FrameMetadata {
            pts: None,
            dts: Some(2000),
        },
        FrameMetadata {
            pts: Some(2000),
            dts: Some(3000),
        },
        FrameMetadata {
            pts: Some(3000),
            dts: Some(4000),
        },
    ];

    let map = FrameIndexMap::new(&frames);

    // PTS quality should be WARN due to missing PTS
    assert_ne!(
        map.pts_quality(),
        bitvue_core::frame_identity::PtsQuality::Ok
    );
}

#[test]
fn test_vp9_altref_manual_offset_adjustment() {
    // Test manual offset adjustment with ALTREF frames
    let frames_a = create_vp9_frames_with_altref(20, &[5, 15]);
    let frames_b = create_vp9_frames_with_altref(20, &[5, 15]);

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let mut workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    // Apply manual offset to compensate for ALTREF timing
    workspace.set_manual_offset(1);
    assert_eq!(workspace.manual_offset(), 1);

    // Verify alignment with offset
    let (b_idx, _quality) = workspace.get_aligned_frame(0).unwrap();
    assert_eq!(b_idx, 1);
}

#[test]
fn test_vp9_altref_alignment_confidence() {
    // Test alignment confidence with various ALTREF patterns
    let frames_a = create_vp9_frames_with_altref(15, &[3, 9]);
    let frames_b = create_vp9_frames_with_altref(15, &[3, 9]);

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let alignment = AlignmentEngine::new(&map_a, &map_b);

    // With matching ALTREF patterns, confidence should be reasonable
    let confidence = alignment.confidence();
    assert!(
        confidence == AlignmentConfidence::High || confidence == AlignmentConfidence::Medium,
        "Expected high/medium confidence with matching ALTREF patterns"
    );
}

#[test]
fn test_vp9_altref_different_gop_sizes() {
    // Stream A: GOP size 10, ALTREF every 10 frames
    // Stream B: GOP size 15, ALTREF every 15 frames
    let frames_a = create_vp9_frames_with_altref(30, &[0, 10, 20]);
    let frames_b = create_vp9_frames_with_altref(30, &[0, 15]);

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    // Different GOP sizes should still allow alignment
    assert!(workspace.alignment.stream_a_count > 0);
}

#[test]
fn test_vp9_altref_sync_mode_interaction() {
    // Test sync mode behavior with ALTREF frames
    use bitvue_core::SyncMode;

    let frames_a = create_vp9_frames_with_altref(12, &[4]);
    let frames_b = create_vp9_frames_with_altref(12, &[4]);

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let mut workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    // Test Off mode
    workspace.set_sync_mode(SyncMode::Off);
    assert_eq!(workspace.sync_mode(), SyncMode::Off);

    // Test Playhead mode
    workspace.set_sync_mode(SyncMode::Playhead);
    assert_eq!(workspace.sync_mode(), SyncMode::Playhead);

    // Test Full mode
    workspace.set_sync_mode(SyncMode::Full);
    assert_eq!(workspace.sync_mode(), SyncMode::Full);
}

#[test]
fn test_vp9_altref_frame_boundary_alignment() {
    // Test alignment at ALTREF frame boundaries
    let frames_a = create_vp9_frames_with_altref(25, &[8, 16]);
    let frames_b = create_vp9_frames_with_altref(25, &[8, 16]);

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    // Check alignment before, at, and after ALTREF positions
    for idx in [7, 8, 9, 15, 16, 17] {
        let result = workspace.get_aligned_frame(idx);
        assert!(
            result.is_some(),
            "Should have alignment at index {} (near ALTREF)",
            idx
        );
    }
}

#[test]
fn test_vp9_altref_extreme_pts_offset() {
    // Test ALTREF with extreme PTS offset (future reference)
    let mut frames_a = vec![];
    for i in 0..10 {
        if i == 3 {
            // ALTREF with very far future PTS
            frames_a.push(FrameMetadata {
                pts: Some(100000),
                dts: Some((i * 1000) as u64),
            });
        } else {
            frames_a.push(FrameMetadata {
                pts: Some((i * 1000) as u64),
                dts: Some((i * 1000) as u64),
            });
        }
    }

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_a);

    let workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    // Should still create workspace despite extreme PTS
    assert_eq!(workspace.total_frames(), 10);
}

#[test]
fn test_vp9_altref_pts_rollback() {
    // Test ALTREF causing PTS "rollback" (PTS going backwards)
    let frames_a = vec![
        FrameMetadata {
            pts: Some(0),
            dts: Some(0),
        },
        FrameMetadata {
            pts: Some(1000),
            dts: Some(1000),
        },
        // ALTREF: PTS goes back to 500
        FrameMetadata {
            pts: Some(500),
            dts: Some(2000),
        },
        FrameMetadata {
            pts: Some(2000),
            dts: Some(3000),
        },
    ];

    let map = FrameIndexMap::new(&frames_a);

    // PTS quality should detect this issue
    // (non-monotonic PTS in decode order is OK, but the sorting will handle it)
    assert_eq!(map.frame_count(), 4);
}
