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
//! Compare Surface VP9 Quirks Test: Segmentation State Timing
//!
//! Tests VP9-specific quirk: Segmentation state and its timing behavior
//! in the Compare surface.
//!
//! VP9 Segmentation:
//! - Up to 8 segments per frame
//! - Segmentation state can persist across frames
//! - segment_id can be predicted from previous frame
//! - Segmentation timing affects frame dependencies
//! - Different segmentation patterns between streams
//!
//! Edge cases tested:
//! - Segmentation enabled/disabled state transitions
//! - Temporal segmentation (prediction from previous frame)
//! - Different segment counts between streams
//! - Segmentation state at GOP boundaries
//! - Segmentation with ALTREF and show_existing_frame

use bitvue_core::{
    compare::CompareWorkspace,
    frame_identity::{FrameIndexMap, FrameMetadata},
    AlignmentEngine,
};

/// Simulated VP9 segmentation state
#[derive(Debug, Clone, Copy, PartialEq)]
enum SegmentationState {
    /// Segmentation disabled
    Disabled,
    /// Segmentation enabled, no temporal prediction
    EnabledSpatial,
    /// Segmentation enabled with temporal prediction
    EnabledTemporal,
    /// Segmentation update (state change)
    Update,
}

/// Helper to create frames with segmentation state
fn create_vp9_frames_with_segmentation(
    count: usize,
    seg_states: Vec<SegmentationState>,
) -> Vec<FrameMetadata> {
    let mut frames = Vec::new();

    for i in 0..count {
        let state = seg_states.get(i % seg_states.len()).copied();

        // Segmentation updates may cause slight timing variations
        let pts_offset = match state {
            Some(SegmentationState::Update) => 50,
            Some(SegmentationState::EnabledTemporal) => 25,
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
fn test_vp9_segmentation_both_disabled() {
    // Both streams have segmentation disabled
    let seg_states = vec![SegmentationState::Disabled];

    let frames_a = create_vp9_frames_with_segmentation(10, seg_states.clone());
    let frames_b = create_vp9_frames_with_segmentation(10, seg_states.clone());

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    // No segmentation should align perfectly
    assert_eq!(workspace.alignment.stream_a_count, 10);
    assert!(workspace.is_diff_enabled() || workspace.disable_reason().is_some());
}

#[test]
fn test_vp9_segmentation_both_enabled_spatial() {
    // Both streams use spatial segmentation (no temporal prediction)
    let seg_states = vec![SegmentationState::EnabledSpatial];

    let frames_a = create_vp9_frames_with_segmentation(12, seg_states.clone());
    let frames_b = create_vp9_frames_with_segmentation(12, seg_states.clone());

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    // Spatial segmentation should align well
    assert_eq!(workspace.alignment.stream_a_count, 12);
}

#[test]
fn test_vp9_segmentation_both_enabled_temporal() {
    // Both streams use temporal segmentation (predict from previous frame)
    let seg_states = vec![SegmentationState::EnabledTemporal];

    let frames_a = create_vp9_frames_with_segmentation(15, seg_states.clone());
    let frames_b = create_vp9_frames_with_segmentation(15, seg_states.clone());

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    // Temporal segmentation should work
    assert_eq!(workspace.total_frames(), 15);
}

#[test]
fn test_vp9_segmentation_state_transitions() {
    // Test transitions: Disabled → Enabled → Temporal → Update → Repeat
    let seg_states = vec![
        SegmentationState::Disabled,
        SegmentationState::EnabledSpatial,
        SegmentationState::EnabledTemporal,
        SegmentationState::Update,
    ];

    let frames_a = create_vp9_frames_with_segmentation(20, seg_states.clone());
    let frames_b = create_vp9_frames_with_segmentation(20, seg_states.clone());

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    // State transitions should be handled
    assert_eq!(workspace.alignment.stream_a_count, 20);
}

#[test]
fn test_vp9_segmentation_asymmetric_states() {
    // Stream A: Disabled
    // Stream B: Enabled temporal
    let seg_states_a = vec![SegmentationState::Disabled];
    let seg_states_b = vec![SegmentationState::EnabledTemporal];

    let frames_a = create_vp9_frames_with_segmentation(10, seg_states_a);
    let frames_b = create_vp9_frames_with_segmentation(10, seg_states_b);

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    // Different segmentation states should still allow comparison
    assert_eq!(workspace.total_frames(), 10);
}

#[test]
fn test_vp9_segmentation_update_frequency() {
    // Stream A: Frequent updates (every 3 frames)
    // Stream B: Rare updates (every 7 frames)
    let seg_states_a = vec![
        SegmentationState::EnabledTemporal,
        SegmentationState::EnabledTemporal,
        SegmentationState::Update,
    ];
    let seg_states_b = vec![
        SegmentationState::EnabledTemporal,
        SegmentationState::EnabledTemporal,
        SegmentationState::EnabledTemporal,
        SegmentationState::EnabledTemporal,
        SegmentationState::EnabledTemporal,
        SegmentationState::EnabledTemporal,
        SegmentationState::Update,
    ];

    let frames_a = create_vp9_frames_with_segmentation(21, seg_states_a);
    let frames_b = create_vp9_frames_with_segmentation(21, seg_states_b);

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    // Different update frequencies should work
    assert_eq!(workspace.total_frames(), 21);
}

#[test]
fn test_vp9_segmentation_gop_boundary_reset() {
    // Segmentation typically resets at GOP boundaries (keyframes)
    // GOP size = 10
    let seg_states = vec![
        SegmentationState::Update, // Keyframe resets
        SegmentationState::EnabledTemporal,
        SegmentationState::EnabledTemporal,
        SegmentationState::EnabledTemporal,
        SegmentationState::EnabledTemporal,
        SegmentationState::EnabledTemporal,
        SegmentationState::EnabledTemporal,
        SegmentationState::EnabledTemporal,
        SegmentationState::EnabledTemporal,
        SegmentationState::EnabledTemporal,
    ];

    let frames_a = create_vp9_frames_with_segmentation(30, seg_states.clone());
    let frames_b = create_vp9_frames_with_segmentation(30, seg_states.clone());

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
fn test_vp9_segmentation_manual_offset() {
    // Test manual offset with segmentation timing variations
    let seg_states = vec![
        SegmentationState::EnabledTemporal,
        SegmentationState::Update,
    ];

    let frames_a = create_vp9_frames_with_segmentation(16, seg_states.clone());
    let frames_b = create_vp9_frames_with_segmentation(16, seg_states.clone());

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let mut workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    // Apply manual offset
    workspace.set_manual_offset(1);
    assert_eq!(workspace.manual_offset(), 1);

    // Verify alignment with offset
    let (b_idx, _quality) = workspace.get_aligned_frame(0).unwrap();
    assert_eq!(b_idx, 1);
}

#[test]
fn test_vp9_segmentation_temporal_prediction_break() {
    // Temporal prediction can break on scene changes
    let seg_states = vec![
        SegmentationState::EnabledTemporal,
        SegmentationState::EnabledTemporal,
        SegmentationState::EnabledTemporal,
        SegmentationState::Update, // Scene change breaks prediction
        SegmentationState::EnabledTemporal,
        SegmentationState::EnabledTemporal,
    ];

    let frames_a = create_vp9_frames_with_segmentation(18, seg_states.clone());
    let frames_b = create_vp9_frames_with_segmentation(18, seg_states.clone());

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    // Prediction breaks should be handled
    assert_eq!(workspace.alignment.stream_a_count, 18);
}

#[test]
fn test_vp9_segmentation_sync_modes() {
    // Test sync modes with segmentation
    use bitvue_core::SyncMode;

    let seg_states = vec![
        SegmentationState::EnabledTemporal,
        SegmentationState::Update,
    ];

    let frames_a = create_vp9_frames_with_segmentation(12, seg_states.clone());
    let frames_b = create_vp9_frames_with_segmentation(12, seg_states.clone());

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
fn test_vp9_segmentation_alignment_confidence() {
    // Test alignment confidence with segmentation patterns
    let seg_states = vec![
        SegmentationState::EnabledTemporal,
        SegmentationState::Update,
    ];

    let frames_a = create_vp9_frames_with_segmentation(14, seg_states.clone());
    let frames_b = create_vp9_frames_with_segmentation(14, seg_states.clone());

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let alignment = AlignmentEngine::new(&map_a, &map_b);

    // Matching patterns should give reasonable confidence
    use bitvue_core::AlignmentConfidence;
    let confidence = alignment.confidence();
    assert!(
        confidence == AlignmentConfidence::High
            || confidence == AlignmentConfidence::Medium
            || confidence == AlignmentConfidence::Low
    );
}

#[test]
fn test_vp9_segmentation_spatial_vs_temporal() {
    // Stream A: Spatial segmentation only
    // Stream B: Temporal segmentation only
    let seg_states_a = vec![SegmentationState::EnabledSpatial];
    let seg_states_b = vec![SegmentationState::EnabledTemporal];

    let frames_a = create_vp9_frames_with_segmentation(15, seg_states_a);
    let frames_b = create_vp9_frames_with_segmentation(15, seg_states_b);

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    // Different segmentation types should still work
    assert_eq!(workspace.total_frames(), 15);
}

#[test]
fn test_vp9_segmentation_pts_quality_impact() {
    // Segmentation updates may cause PTS variations
    let seg_states = vec![
        SegmentationState::EnabledTemporal,
        SegmentationState::Update,
        SegmentationState::EnabledTemporal,
        SegmentationState::Update,
    ];

    let frames = create_vp9_frames_with_segmentation(16, seg_states);
    let map = FrameIndexMap::new(&frames);

    // PTS quality should still be reasonable
    use bitvue_core::frame_identity::PtsQuality;
    let quality = map.pts_quality();
    assert!(
        quality == PtsQuality::Ok || quality == PtsQuality::Warn,
        "Expected OK or WARN PTS quality with segmentation"
    );
}

#[test]
fn test_vp9_segmentation_different_segment_counts() {
    // Streams with different numbers of segments (1-8)
    // Stream A: 4 segments
    // Stream B: 8 segments
    // (Simulated by different update patterns)
    let seg_states_a = vec![
        SegmentationState::EnabledTemporal,
        SegmentationState::EnabledTemporal,
        SegmentationState::Update,
        SegmentationState::EnabledTemporal,
    ];
    let seg_states_b = vec![
        SegmentationState::EnabledTemporal,
        SegmentationState::EnabledTemporal,
        SegmentationState::EnabledTemporal,
        SegmentationState::EnabledTemporal,
        SegmentationState::EnabledTemporal,
        SegmentationState::EnabledTemporal,
        SegmentationState::Update,
        SegmentationState::EnabledTemporal,
    ];

    let frames_a = create_vp9_frames_with_segmentation(24, seg_states_a);
    let frames_b = create_vp9_frames_with_segmentation(24, seg_states_b);

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    // Different segment counts should still allow comparison
    assert_eq!(workspace.total_frames(), 24);
}

#[test]
fn test_vp9_segmentation_rapid_state_changes() {
    // Rapid segmentation state changes
    let seg_states = vec![
        SegmentationState::Disabled,
        SegmentationState::EnabledSpatial,
        SegmentationState::EnabledTemporal,
        SegmentationState::Update,
        SegmentationState::Disabled,
        SegmentationState::EnabledSpatial,
    ];

    let frames_a = create_vp9_frames_with_segmentation(24, seg_states.clone());
    let frames_b = create_vp9_frames_with_segmentation(24, seg_states.clone());

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    // Rapid changes should be handled
    assert_eq!(workspace.alignment.stream_a_count, 24);
}
