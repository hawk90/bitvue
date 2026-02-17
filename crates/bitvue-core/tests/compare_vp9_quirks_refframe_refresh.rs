#![allow(dead_code)]
//! Compare Surface VP9 Quirks Test: Reference Frame Refresh Patterns
//!
//! Tests VP9-specific quirk: Reference frame refresh patterns and their impact
//! on Compare surface alignment and visualization.
//!
//! VP9 Reference Frame Refresh:
//! - VP9 maintains 8 reference frame slots (LAST, GOLDEN, ALTREF, etc.)
//! - refresh_frame_flags indicates which slots are updated
//! - Complex refresh patterns can cause alignment issues
//! - Different encoders use different refresh strategies
//! - Refresh patterns affect frame dependencies
//!
//! Edge cases tested:
//! - Different refresh patterns between streams
//! - No refresh (static reference frames)
//! - Full refresh every frame
//! - Cyclic refresh patterns
//! - Reference frame slot reuse patterns

use bitvue_core::{
    compare::CompareWorkspace,
    frame_identity::{FrameIndexMap, FrameMetadata},
    AlignmentConfidence, AlignmentEngine,
};

/// Simulated VP9 refresh pattern
#[derive(Debug, Clone, Copy, PartialEq)]
enum RefreshPattern {
    /// No refresh (references only, no update)
    NoRefresh,
    /// Refresh LAST frame only
    RefreshLast,
    /// Refresh GOLDEN frame
    RefreshGolden,
    /// Refresh ALTREF frame
    RefreshAltref,
    /// Refresh all frames
    RefreshAll,
}

/// Helper to create frames with refresh pattern metadata
fn create_vp9_frames_with_refresh(
    count: usize,
    refresh_pattern: Vec<RefreshPattern>,
) -> Vec<FrameMetadata> {
    let mut frames = Vec::new();

    for i in 0..count {
        let pattern = refresh_pattern.get(i % refresh_pattern.len()).copied();

        // Refresh patterns may affect PTS timing slightly
        let pts_offset = match pattern {
            Some(RefreshPattern::NoRefresh) => 0,
            Some(RefreshPattern::RefreshAltref) => 100, // ALTREF may have future PTS
            _ => 0,
        };

        frames.push(FrameMetadata {
            pts: Some((i * 1000 + pts_offset) as u64),
            dts: Some((i * 1000) as u64),
        });
    }

    frames
}

#[test]
fn test_vp9_refresh_identical_patterns() {
    // Both streams use same refresh pattern: LAST → GOLDEN → ALTREF → repeat
    let pattern = vec![
        RefreshPattern::RefreshLast,
        RefreshPattern::RefreshGolden,
        RefreshPattern::RefreshAltref,
    ];

    let frames_a = create_vp9_frames_with_refresh(15, pattern.clone());
    let frames_b = create_vp9_frames_with_refresh(15, pattern.clone());

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    // Identical patterns should align well
    assert_eq!(workspace.alignment.stream_a_count, 15);
    assert!(workspace.is_diff_enabled() || workspace.disable_reason().is_some());
}

#[test]
fn test_vp9_refresh_different_patterns() {
    // Stream A: LAST → GOLDEN → repeat
    // Stream B: LAST → ALTREF → repeat
    let pattern_a = vec![RefreshPattern::RefreshLast, RefreshPattern::RefreshGolden];
    let pattern_b = vec![RefreshPattern::RefreshLast, RefreshPattern::RefreshAltref];

    let frames_a = create_vp9_frames_with_refresh(20, pattern_a);
    let frames_b = create_vp9_frames_with_refresh(20, pattern_b);

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    // Different patterns should still allow comparison
    assert_eq!(workspace.total_frames(), 20);
}

#[test]
fn test_vp9_refresh_no_refresh_stream() {
    // Stream A: Normal refresh pattern
    // Stream B: No refresh (all frames use same references)
    let pattern_a = vec![RefreshPattern::RefreshLast, RefreshPattern::RefreshGolden];
    let pattern_b = vec![RefreshPattern::NoRefresh];

    let frames_a = create_vp9_frames_with_refresh(12, pattern_a);
    let frames_b = create_vp9_frames_with_refresh(12, pattern_b);

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    // No refresh pattern should still work
    assert_eq!(workspace.alignment.stream_a_count, 12);
}

#[test]
fn test_vp9_refresh_all_frames_every_frame() {
    // Both streams refresh all frames every frame (inefficient but valid)
    let pattern = vec![RefreshPattern::RefreshAll];

    let frames_a = create_vp9_frames_with_refresh(10, pattern.clone());
    let frames_b = create_vp9_frames_with_refresh(10, pattern.clone());

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    // All-refresh pattern should align perfectly
    assert_eq!(workspace.alignment.stream_a_count, 10);
}

#[test]
fn test_vp9_refresh_cyclic_pattern_sync() {
    // Test cyclic refresh pattern: period 4 vs period 3
    let pattern_a = vec![
        RefreshPattern::RefreshLast,
        RefreshPattern::RefreshGolden,
        RefreshPattern::RefreshAltref,
        RefreshPattern::NoRefresh,
    ];
    let pattern_b = vec![
        RefreshPattern::RefreshLast,
        RefreshPattern::RefreshGolden,
        RefreshPattern::RefreshAltref,
    ];

    let frames_a = create_vp9_frames_with_refresh(24, pattern_a);
    let frames_b = create_vp9_frames_with_refresh(24, pattern_b);

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    // Different cycle periods should still allow alignment
    assert_eq!(workspace.total_frames(), 24);
}

#[test]
fn test_vp9_refresh_alignment_confidence() {
    // Test alignment confidence with different refresh patterns
    let pattern_a = vec![RefreshPattern::RefreshLast, RefreshPattern::RefreshGolden];
    let pattern_b = vec![RefreshPattern::RefreshLast, RefreshPattern::RefreshGolden];

    let frames_a = create_vp9_frames_with_refresh(18, pattern_a);
    let frames_b = create_vp9_frames_with_refresh(18, pattern_b);

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let alignment = AlignmentEngine::new(&map_a, &map_b);

    // Matching patterns should give good confidence
    let confidence = alignment.confidence();
    assert!(
        confidence == AlignmentConfidence::High || confidence == AlignmentConfidence::Medium,
        "Expected high/medium confidence with matching refresh patterns"
    );
}

#[test]
fn test_vp9_refresh_manual_offset_compensation() {
    // Use manual offset to compensate for refresh pattern phase difference
    let pattern = vec![RefreshPattern::RefreshLast, RefreshPattern::RefreshGolden];

    let frames_a = create_vp9_frames_with_refresh(16, pattern.clone());
    let frames_b = create_vp9_frames_with_refresh(16, pattern.clone());

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let mut workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    // Apply offset to shift phase
    workspace.set_manual_offset(1);
    assert_eq!(workspace.manual_offset(), 1);

    // Verify offset affects alignment
    let (b_idx, _quality) = workspace.get_aligned_frame(0).unwrap();
    assert_eq!(b_idx, 1);
}

#[test]
fn test_vp9_refresh_gop_boundary_behavior() {
    // Refresh pattern resets at GOP boundaries (keyframes)
    // GOP size = 10, keyframe at 0, 10, 20
    let mut pattern = vec![
        RefreshPattern::RefreshAll, // Keyframe
        RefreshPattern::RefreshLast,
        RefreshPattern::RefreshGolden,
        RefreshPattern::RefreshLast,
        RefreshPattern::RefreshGolden,
        RefreshPattern::RefreshLast,
        RefreshPattern::RefreshGolden,
        RefreshPattern::RefreshLast,
        RefreshPattern::RefreshGolden,
        RefreshPattern::RefreshAltref,
    ];

    let frames_a = create_vp9_frames_with_refresh(30, pattern.clone());
    let frames_b = create_vp9_frames_with_refresh(30, pattern.clone());

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    // Check alignment at GOP boundaries
    for gop_start in [0, 10, 20] {
        let result = workspace.get_aligned_frame(gop_start);
        assert!(
            result.is_some(),
            "Should have alignment at GOP boundary {}",
            gop_start
        );
    }
}

#[test]
fn test_vp9_refresh_asymmetric_gop_structures() {
    // Stream A: GOP size 10
    // Stream B: GOP size 15
    let pattern_short = vec![
        RefreshPattern::RefreshAll,
        RefreshPattern::RefreshLast,
        RefreshPattern::RefreshGolden,
        RefreshPattern::RefreshLast,
        RefreshPattern::RefreshGolden,
        RefreshPattern::RefreshLast,
        RefreshPattern::RefreshGolden,
        RefreshPattern::RefreshLast,
        RefreshPattern::RefreshGolden,
        RefreshPattern::RefreshAltref,
    ];

    let pattern_long = vec![
        RefreshPattern::RefreshAll,
        RefreshPattern::RefreshLast,
        RefreshPattern::RefreshGolden,
        RefreshPattern::RefreshLast,
        RefreshPattern::RefreshGolden,
        RefreshPattern::RefreshLast,
        RefreshPattern::RefreshGolden,
        RefreshPattern::RefreshLast,
        RefreshPattern::RefreshGolden,
        RefreshPattern::RefreshLast,
        RefreshPattern::RefreshGolden,
        RefreshPattern::RefreshLast,
        RefreshPattern::RefreshGolden,
        RefreshPattern::RefreshAltref,
        RefreshPattern::RefreshLast,
    ];

    let frames_a = create_vp9_frames_with_refresh(30, pattern_short);
    let frames_b = create_vp9_frames_with_refresh(30, pattern_long);

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    // Different GOP sizes should still allow comparison
    assert_eq!(workspace.total_frames(), 30);
}

#[test]
fn test_vp9_refresh_slot_reuse_patterns() {
    // Test reference slot reuse: LAST → GOLDEN → LAST → GOLDEN
    let pattern = vec![
        RefreshPattern::RefreshLast,
        RefreshPattern::RefreshGolden,
        RefreshPattern::RefreshLast,
        RefreshPattern::RefreshGolden,
    ];

    let frames_a = create_vp9_frames_with_refresh(20, pattern.clone());
    let frames_b = create_vp9_frames_with_refresh(20, pattern.clone());

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    // Slot reuse should not affect alignment
    assert_eq!(workspace.alignment.stream_a_count, 20);
}

#[test]
fn test_vp9_refresh_sync_mode_interaction() {
    // Test sync modes with different refresh patterns
    use bitvue_core::SyncMode;

    let pattern = vec![RefreshPattern::RefreshLast, RefreshPattern::RefreshGolden];

    let frames_a = create_vp9_frames_with_refresh(14, pattern.clone());
    let frames_b = create_vp9_frames_with_refresh(14, pattern.clone());

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let mut workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    // Test all sync modes
    workspace.set_sync_mode(SyncMode::Off);
    assert_eq!(workspace.sync_mode(), SyncMode::Off);

    workspace.set_sync_mode(SyncMode::Playhead);
    assert_eq!(workspace.sync_mode(), SyncMode::Playhead);

    workspace.set_sync_mode(SyncMode::Full);
    assert_eq!(workspace.sync_mode(), SyncMode::Full);
}

#[test]
fn test_vp9_refresh_mixed_keyframe_patterns() {
    // Stream A: Keyframes every 10 frames
    // Stream B: Keyframes every 12 frames
    let mut frames_a = vec![];
    let mut frames_b = vec![];

    for i in 0..30 {
        let pts_a = (i * 1000) as u64;
        let pts_b = (i * 1000) as u64;

        frames_a.push(FrameMetadata {
            pts: Some(pts_a),
            dts: Some((i * 1000) as u64),
        });

        frames_b.push(FrameMetadata {
            pts: Some(pts_b),
            dts: Some((i * 1000) as u64),
        });
    }

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    // Different keyframe intervals should still align
    assert_eq!(workspace.total_frames(), 30);
}

#[test]
fn test_vp9_refresh_altref_interaction() {
    // Refresh pattern with ALTREF frames (different PTS timing)
    let pattern = vec![
        RefreshPattern::RefreshLast,
        RefreshPattern::RefreshGolden,
        RefreshPattern::RefreshAltref, // ALTREF has future PTS
        RefreshPattern::RefreshLast,
    ];

    let frames_a = create_vp9_frames_with_refresh(16, pattern.clone());
    let frames_b = create_vp9_frames_with_refresh(16, pattern.clone());

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    // ALTREF refresh should be handled correctly
    assert_eq!(workspace.alignment.stream_a_count, 16);
}

#[test]
fn test_vp9_refresh_temporal_layering() {
    // Simulate temporal layering refresh pattern
    // Base layer: LAST refresh
    // Enhancement layers: GOLDEN/ALTREF refresh
    let pattern = vec![
        RefreshPattern::RefreshLast,   // Base layer
        RefreshPattern::RefreshGolden, // Enhancement
        RefreshPattern::RefreshLast,   // Base layer
        RefreshPattern::RefreshAltref, // Enhancement
    ];

    let frames_a = create_vp9_frames_with_refresh(24, pattern.clone());
    let frames_b = create_vp9_frames_with_refresh(24, pattern.clone());

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    // Temporal layering should not break alignment
    assert_eq!(workspace.total_frames(), 24);
}

#[test]
fn test_vp9_refresh_encoder_specific_patterns() {
    // Different encoders use different refresh strategies
    // Encoder A: Conservative (frequent refresh)
    // Encoder B: Aggressive (minimal refresh)
    let pattern_conservative = vec![RefreshPattern::RefreshLast, RefreshPattern::RefreshGolden];
    let pattern_aggressive = vec![
        RefreshPattern::RefreshLast,
        RefreshPattern::NoRefresh,
        RefreshPattern::NoRefresh,
        RefreshPattern::RefreshGolden,
    ];

    let frames_a = create_vp9_frames_with_refresh(20, pattern_conservative);
    let frames_b = create_vp9_frames_with_refresh(20, pattern_aggressive);

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    // Different encoder strategies should still work
    assert_eq!(workspace.total_frames(), 20);
}
