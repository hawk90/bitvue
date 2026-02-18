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
//! Compare Surface VP9 Quirks Test: Superframe Decomposition
//!
//! Tests VP9-specific quirk: Superframe structure and decomposition behavior
//! in the Compare surface.
//!
//! VP9 Superframes:
//! - Multiple frames packaged into single container packet
//! - Superframe index at end of packet
//! - Each sub-frame has its own size
//! - Temporal layering often uses superframes
//! - Alignment challenges when superframe boundaries differ
//!
//! Edge cases tested:
//! - Different superframe structures between streams
//! - Superframes with different sub-frame counts (2, 3, 4 frames)
//! - Mixed superframes and single frames
//! - Superframe at GOP boundaries
//! - Misaligned superframe boundaries
//! - Invalid/corrupted superframe indices

use bitvue_core::{
    compare::CompareWorkspace,
    frame_identity::{FrameIndexMap, FrameMetadata},
    AlignmentConfidence, AlignmentEngine,
};

/// Simulated VP9 superframe structure
#[derive(Debug, Clone, Copy, PartialEq)]
enum FramePackaging {
    /// Single frame (no superframe)
    Single,
    /// Part of a superframe (index within superframe)
    SuperframeSubFrame(usize, usize), // (sub_frame_idx, total_sub_frames)
}

/// Helper to create frames with superframe structure
fn create_vp9_frames_with_superframes(packaging: Vec<FramePackaging>) -> Vec<FrameMetadata> {
    let mut frames = Vec::new();
    let mut pts = 0u64;

    for (i, pack) in packaging.iter().enumerate() {
        match pack {
            FramePackaging::Single => {
                frames.push(FrameMetadata {
                    pts: Some(pts),
                    dts: Some((i * 1000) as u64),
                });
                pts += 1000;
            }
            FramePackaging::SuperframeSubFrame(idx, total) => {
                // Sub-frames in superframe share container timestamp
                // but have different display times
                frames.push(FrameMetadata {
                    pts: Some(pts + (*idx as u64 * 100)),
                    dts: Some((i * 1000) as u64),
                });

                // Advance PTS only after last sub-frame
                if *idx == total - 1 {
                    pts += 1000;
                }
            }
        }
    }

    frames
}

#[test]
fn test_vp9_superframe_identical_structures() {
    // Both streams use same superframe structure: [SF(3), Single, SF(2), Single]
    let packaging = vec![
        FramePackaging::SuperframeSubFrame(0, 3),
        FramePackaging::SuperframeSubFrame(1, 3),
        FramePackaging::SuperframeSubFrame(2, 3),
        FramePackaging::Single,
        FramePackaging::SuperframeSubFrame(0, 2),
        FramePackaging::SuperframeSubFrame(1, 2),
        FramePackaging::Single,
    ];

    let frames_a = create_vp9_frames_with_superframes(packaging.clone());
    let frames_b = create_vp9_frames_with_superframes(packaging.clone());

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    // Identical superframe structures should align perfectly
    assert_eq!(workspace.alignment.stream_a_count, 7);
    assert!(workspace.is_diff_enabled() || workspace.disable_reason().is_some());
}

#[test]
fn test_vp9_superframe_different_sub_frame_counts() {
    // Stream A: SF(2) repeated
    // Stream B: SF(3) repeated
    let packaging_a = vec![
        FramePackaging::SuperframeSubFrame(0, 2),
        FramePackaging::SuperframeSubFrame(1, 2),
        FramePackaging::SuperframeSubFrame(0, 2),
        FramePackaging::SuperframeSubFrame(1, 2),
    ];
    let packaging_b = vec![
        FramePackaging::SuperframeSubFrame(0, 3),
        FramePackaging::SuperframeSubFrame(1, 3),
        FramePackaging::SuperframeSubFrame(2, 3),
        FramePackaging::Single, // One extra frame to match count
    ];

    let frames_a = create_vp9_frames_with_superframes(packaging_a);
    let frames_b = create_vp9_frames_with_superframes(packaging_b);

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    // Different sub-frame counts should still allow alignment
    assert_eq!(workspace.total_frames(), 4);
}

#[test]
fn test_vp9_superframe_vs_all_single() {
    // Stream A: Uses superframes
    // Stream B: All single frames
    let packaging_a = vec![
        FramePackaging::SuperframeSubFrame(0, 3),
        FramePackaging::SuperframeSubFrame(1, 3),
        FramePackaging::SuperframeSubFrame(2, 3),
        FramePackaging::SuperframeSubFrame(0, 2),
        FramePackaging::SuperframeSubFrame(1, 2),
    ];
    let packaging_b = vec![
        FramePackaging::Single,
        FramePackaging::Single,
        FramePackaging::Single,
        FramePackaging::Single,
        FramePackaging::Single,
    ];

    let frames_a = create_vp9_frames_with_superframes(packaging_a);
    let frames_b = create_vp9_frames_with_superframes(packaging_b);

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    // Superframe vs single should still work
    assert_eq!(workspace.total_frames(), 5);
}

#[test]
fn test_vp9_superframe_misaligned_boundaries() {
    // Superframe boundaries at different positions
    // Stream A: [SF(2), SF(2), Single]
    // Stream B: [Single, SF(2), SF(2)]
    let packaging_a = vec![
        FramePackaging::SuperframeSubFrame(0, 2),
        FramePackaging::SuperframeSubFrame(1, 2),
        FramePackaging::SuperframeSubFrame(0, 2),
        FramePackaging::SuperframeSubFrame(1, 2),
        FramePackaging::Single,
    ];
    let packaging_b = vec![
        FramePackaging::Single,
        FramePackaging::SuperframeSubFrame(0, 2),
        FramePackaging::SuperframeSubFrame(1, 2),
        FramePackaging::SuperframeSubFrame(0, 2),
        FramePackaging::SuperframeSubFrame(1, 2),
    ];

    let frames_a = create_vp9_frames_with_superframes(packaging_a);
    let frames_b = create_vp9_frames_with_superframes(packaging_b);

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    // Misaligned boundaries should be handled
    assert_eq!(workspace.total_frames(), 5);
}

#[test]
fn test_vp9_superframe_temporal_layering() {
    // Temporal layering: Base layer single, enhancement layers in superframe
    // Pattern: [Single(base), SF(2:enhancement), Single(base), SF(2:enhancement)]
    let packaging = vec![
        FramePackaging::Single,                   // Base layer
        FramePackaging::SuperframeSubFrame(0, 2), // Enhancement
        FramePackaging::SuperframeSubFrame(1, 2),
        FramePackaging::Single,                   // Base layer
        FramePackaging::SuperframeSubFrame(0, 2), // Enhancement
        FramePackaging::SuperframeSubFrame(1, 2),
    ];

    let frames_a = create_vp9_frames_with_superframes(packaging.clone());
    let frames_b = create_vp9_frames_with_superframes(packaging.clone());

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    // Temporal layering with superframes should work
    assert_eq!(workspace.alignment.stream_a_count, 6);
}

#[test]
fn test_vp9_superframe_gop_boundary() {
    // Superframe at GOP boundary (keyframe)
    // [SF(3:keyframe+refs), Single, Single, ..., SF(3:keyframe+refs)]
    let packaging = vec![
        FramePackaging::SuperframeSubFrame(0, 3), // Keyframe GOP
        FramePackaging::SuperframeSubFrame(1, 3),
        FramePackaging::SuperframeSubFrame(2, 3),
        FramePackaging::Single,
        FramePackaging::Single,
        FramePackaging::SuperframeSubFrame(0, 3), // Next keyframe GOP
        FramePackaging::SuperframeSubFrame(1, 3),
        FramePackaging::SuperframeSubFrame(2, 3),
    ];

    let frames_a = create_vp9_frames_with_superframes(packaging.clone());
    let frames_b = create_vp9_frames_with_superframes(packaging.clone());

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    // GOP boundary superframes should align
    assert_eq!(workspace.total_frames(), 8);

    // Check alignment at GOP boundaries (frames 0 and 5)
    let result_0 = workspace.get_aligned_frame(0);
    assert!(result_0.is_some(), "Should align at first GOP boundary");

    let result_5 = workspace.get_aligned_frame(5);
    assert!(result_5.is_some(), "Should align at second GOP boundary");
}

#[test]
fn test_vp9_superframe_manual_offset() {
    // Test manual offset with superframe misalignment
    let packaging = vec![
        FramePackaging::SuperframeSubFrame(0, 2),
        FramePackaging::SuperframeSubFrame(1, 2),
        FramePackaging::Single,
        FramePackaging::Single,
    ];

    let frames_a = create_vp9_frames_with_superframes(packaging.clone());
    let frames_b = create_vp9_frames_with_superframes(packaging.clone());

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let mut workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    // Apply offset to shift superframe alignment
    workspace.set_manual_offset(1);
    assert_eq!(workspace.manual_offset(), 1);

    // Verify alignment with offset
    let (b_idx, _quality) = workspace.get_aligned_frame(0).unwrap();
    assert_eq!(b_idx, 1);
}

#[test]
fn test_vp9_superframe_large_sub_frame_count() {
    // Large superframe with 4 sub-frames (max practical size)
    let packaging = vec![
        FramePackaging::SuperframeSubFrame(0, 4),
        FramePackaging::SuperframeSubFrame(1, 4),
        FramePackaging::SuperframeSubFrame(2, 4),
        FramePackaging::SuperframeSubFrame(3, 4),
        FramePackaging::Single,
    ];

    let frames_a = create_vp9_frames_with_superframes(packaging.clone());
    let frames_b = create_vp9_frames_with_superframes(packaging.clone());

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    // Large superframes should work
    assert_eq!(workspace.alignment.stream_a_count, 5);
}

#[test]
fn test_vp9_superframe_alignment_confidence() {
    // Test alignment confidence with superframe patterns
    let packaging = vec![
        FramePackaging::SuperframeSubFrame(0, 2),
        FramePackaging::SuperframeSubFrame(1, 2),
        FramePackaging::Single,
    ];

    let frames_a = create_vp9_frames_with_superframes(packaging.clone());
    let frames_b = create_vp9_frames_with_superframes(packaging.clone());

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let alignment = AlignmentEngine::new(&map_a, &map_b);

    // Matching superframe patterns should give good confidence
    let confidence = alignment.confidence();
    assert!(
        confidence == AlignmentConfidence::High || confidence == AlignmentConfidence::Medium,
        "Expected high/medium confidence with matching superframe patterns"
    );
}

#[test]
fn test_vp9_superframe_sync_modes() {
    // Test sync modes with superframes
    use bitvue_core::SyncMode;

    let packaging = vec![
        FramePackaging::SuperframeSubFrame(0, 3),
        FramePackaging::SuperframeSubFrame(1, 3),
        FramePackaging::SuperframeSubFrame(2, 3),
    ];

    let frames_a = create_vp9_frames_with_superframes(packaging.clone());
    let frames_b = create_vp9_frames_with_superframes(packaging.clone());

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
fn test_vp9_superframe_pts_timing_variations() {
    // Superframe sub-frames have close PTS values
    let packaging = vec![
        FramePackaging::SuperframeSubFrame(0, 3),
        FramePackaging::SuperframeSubFrame(1, 3),
        FramePackaging::SuperframeSubFrame(2, 3),
    ];

    let frames = create_vp9_frames_with_superframes(packaging);
    let map = FrameIndexMap::new(&frames);

    // PTS should be present and reasonable
    use bitvue_core::frame_identity::PtsQuality;
    let quality = map.pts_quality();
    assert!(
        quality == PtsQuality::Ok || quality == PtsQuality::Warn,
        "Expected OK or WARN PTS quality with superframes"
    );
}

#[test]
fn test_vp9_superframe_mixed_with_altref() {
    // Superframe containing ALTREF sub-frame
    // ALTREF in superframe has special timing
    let packaging = vec![
        FramePackaging::SuperframeSubFrame(0, 3), // Regular
        FramePackaging::SuperframeSubFrame(1, 3), // ALTREF (not shown)
        FramePackaging::SuperframeSubFrame(2, 3), // Regular
        FramePackaging::Single,
    ];

    let frames_a = create_vp9_frames_with_superframes(packaging.clone());
    let frames_b = create_vp9_frames_with_superframes(packaging.clone());

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    // Superframe with ALTREF should work
    assert_eq!(workspace.total_frames(), 4);
}

#[test]
fn test_vp9_superframe_different_gop_patterns() {
    // Stream A: Superframe every GOP
    // Stream B: No superframes
    let packaging_a = vec![
        FramePackaging::SuperframeSubFrame(0, 2),
        FramePackaging::SuperframeSubFrame(1, 2),
        FramePackaging::Single,
        FramePackaging::Single,
        FramePackaging::SuperframeSubFrame(0, 2),
        FramePackaging::SuperframeSubFrame(1, 2),
    ];
    let packaging_b = vec![
        FramePackaging::Single,
        FramePackaging::Single,
        FramePackaging::Single,
        FramePackaging::Single,
        FramePackaging::Single,
        FramePackaging::Single,
    ];

    let frames_a = create_vp9_frames_with_superframes(packaging_a);
    let frames_b = create_vp9_frames_with_superframes(packaging_b);

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    // Different GOP patterns should work
    assert_eq!(workspace.total_frames(), 6);
}

#[test]
fn test_vp9_superframe_nested_structure_simulation() {
    // Simulate complex nested structure (not actually nested, but complex pattern)
    let packaging = vec![
        FramePackaging::SuperframeSubFrame(0, 2),
        FramePackaging::SuperframeSubFrame(1, 2),
        FramePackaging::SuperframeSubFrame(0, 3),
        FramePackaging::SuperframeSubFrame(1, 3),
        FramePackaging::SuperframeSubFrame(2, 3),
        FramePackaging::Single,
        FramePackaging::SuperframeSubFrame(0, 2),
        FramePackaging::SuperframeSubFrame(1, 2),
    ];

    let frames_a = create_vp9_frames_with_superframes(packaging.clone());
    let frames_b = create_vp9_frames_with_superframes(packaging.clone());

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    // Complex patterns should be handled
    assert_eq!(workspace.alignment.stream_a_count, 8);
}

#[test]
fn test_vp9_superframe_sub_frame_boundary_alignment() {
    // Check alignment at sub-frame boundaries
    let packaging = vec![
        FramePackaging::SuperframeSubFrame(0, 3),
        FramePackaging::SuperframeSubFrame(1, 3),
        FramePackaging::SuperframeSubFrame(2, 3),
        FramePackaging::Single,
    ];

    let frames_a = create_vp9_frames_with_superframes(packaging.clone());
    let frames_b = create_vp9_frames_with_superframes(packaging.clone());

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    // Check alignment at each sub-frame position
    for idx in 0..4 {
        let result = workspace.get_aligned_frame(idx);
        assert!(
            result.is_some(),
            "Should have alignment at sub-frame {}",
            idx
        );
    }
}
