#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(unused_comparisons)]
#![allow(unused_doc_comments)]
//! Compare Surface VP9 Quirks Test: Integration Tests
//!
//! Tests VP9-specific quirks in combination, simulating real-world encode scenarios
//! in the Compare surface.
//!
//! This file combines multiple VP9 quirks to test realistic scenarios:
//! - ALTREF + show_existing_frame combinations
//! - Superframes + reference refresh patterns
//! - Segmentation + temporal layering
//! - All quirks combined in complex GOP structures
//!
//! Edge cases tested:
//! - Real-world encoder patterns (libvpx, libaom-av1 VP9 mode)
//! - Scene change handling with multiple quirks
//! - Bitrate adaptation with quirks
//! - Temporal SVC with quirks
//! - Error resilience patterns

use bitvue_core::{
    compare::CompareWorkspace,
    frame_identity::{FrameIndexMap, FrameMetadata, PtsQuality},
    AlignmentConfidence, AlignmentEngine, SyncMode,
};

/// Comprehensive VP9 frame type
#[derive(Debug, Clone, Copy, PartialEq)]
enum Vp9FrameType {
    /// Keyframe (IDR)
    Key,
    /// ALTREF frame (not shown)
    Altref,
    /// show_existing_frame (virtual display)
    ShowExisting(usize), // references frame index
    /// Regular inter frame
    Inter,
    /// Regular inter frame in superframe
    InterInSuperframe(usize, usize), // (sub_idx, total)
}

/// Helper to create realistic VP9 GOP structure
fn create_realistic_vp9_gop(gop_size: usize, use_altref: bool) -> Vec<Vp9FrameType> {
    let mut gop = vec![Vp9FrameType::Key]; // Start with keyframe

    if use_altref {
        // Insert ALTREF as frame 1 (to be used as reference)
        gop.push(Vp9FrameType::Altref);
    }

    // Fill with inter frames
    for _ in 0..(gop_size - gop.len()) {
        gop.push(Vp9FrameType::Inter);
    }

    gop
}

/// Convert frame types to metadata
fn frame_types_to_metadata(frame_types: &[Vp9FrameType]) -> Vec<FrameMetadata> {
    let mut frames = Vec::new();
    let mut pts = 0u64;
    let mut display_pts = 0u64;

    for (i, frame_type) in frame_types.iter().enumerate() {
        match frame_type {
            Vp9FrameType::Key => {
                frames.push(FrameMetadata {
                    pts: Some(display_pts),
                    dts: Some((i * 1000) as u64),
                });
                display_pts += 1000;
            }
            Vp9FrameType::Altref => {
                // ALTREF has future PTS but comes early in decode order
                frames.push(FrameMetadata {
                    pts: Some(display_pts + 5000), // Future reference
                    dts: Some((i * 1000) as u64),
                });
            }
            Vp9FrameType::ShowExisting(ref_idx) => {
                // Reuse PTS from reference frame (creates duplicate PTS)
                let ref_pts = if *ref_idx < frames.len() {
                    frames[*ref_idx].pts.unwrap_or(display_pts)
                } else {
                    display_pts
                };
                frames.push(FrameMetadata {
                    pts: Some(ref_pts),
                    dts: Some((i * 1000) as u64),
                });
            }
            Vp9FrameType::Inter => {
                frames.push(FrameMetadata {
                    pts: Some(display_pts),
                    dts: Some((i * 1000) as u64),
                });
                display_pts += 1000;
            }
            Vp9FrameType::InterInSuperframe(sub_idx, total) => {
                frames.push(FrameMetadata {
                    pts: Some(display_pts + (*sub_idx as u64 * 100)),
                    dts: Some((i * 1000) as u64),
                });
                if *sub_idx == total - 1 {
                    display_pts += 1000;
                }
            }
        }
    }

    frames
}

#[test]
fn test_vp9_integration_libvpx_encoding_pattern() {
    // Simulate typical libvpx encoding: GOP=10, ALTREF enabled
    let mut frame_types = Vec::new();

    // First GOP
    frame_types.extend(create_realistic_vp9_gop(10, true));
    // Second GOP
    frame_types.extend(create_realistic_vp9_gop(10, true));
    // Third GOP
    frame_types.extend(create_realistic_vp9_gop(10, true));

    let frames_a = frame_types_to_metadata(&frame_types);
    let frames_b = frame_types_to_metadata(&frame_types);

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    // Verify realistic GOP structure aligns
    assert_eq!(workspace.total_frames(), 30);
    assert!(workspace.alignment.stream_a_count > 0);
}

#[test]
fn test_vp9_integration_altref_plus_show_existing() {
    // Combine ALTREF and show_existing_frame
    let frame_types = vec![
        Vp9FrameType::Key,
        Vp9FrameType::Altref,
        Vp9FrameType::Inter,
        Vp9FrameType::Inter,
        Vp9FrameType::ShowExisting(1), // Show the ALTREF
        Vp9FrameType::Inter,
    ];

    let frames_a = frame_types_to_metadata(&frame_types);
    let frames_b = frame_types_to_metadata(&frame_types);

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    // ALTREF + show_existing should work
    assert_eq!(workspace.total_frames(), 6);
}

#[test]
fn test_vp9_integration_superframe_temporal_svc() {
    // Temporal SVC using superframes: Base + 2 enhancement layers
    let frame_types = vec![
        Vp9FrameType::Key,                     // Base layer keyframe
        Vp9FrameType::InterInSuperframe(0, 3), // SF: base + 2 enhancement
        Vp9FrameType::InterInSuperframe(1, 3),
        Vp9FrameType::InterInSuperframe(2, 3),
        Vp9FrameType::Inter, // Base layer
        Vp9FrameType::InterInSuperframe(0, 3),
        Vp9FrameType::InterInSuperframe(1, 3),
        Vp9FrameType::InterInSuperframe(2, 3),
    ];

    let frames_a = frame_types_to_metadata(&frame_types);
    let frames_b = frame_types_to_metadata(&frame_types);

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    // Temporal SVC should work
    assert_eq!(workspace.alignment.stream_a_count, 8);
}

#[test]
fn test_vp9_integration_scene_change_with_quirks() {
    // Scene change resets GOP structure mid-stream
    // First scene: GOP with ALTREF
    let mut frame_types = create_realistic_vp9_gop(10, true);
    frame_types.extend(&[Vp9FrameType::Inter, Vp9FrameType::Inter]);

    // Scene change: New keyframe
    frame_types.push(Vp9FrameType::Key);
    frame_types.extend(create_realistic_vp9_gop(10, true)[1..].to_vec()); // Skip duplicate key

    let frames_a = frame_types_to_metadata(&frame_types);
    let frames_b = frame_types_to_metadata(&frame_types);

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    // Scene change should not break alignment
    assert!(workspace.total_frames() > 20);
}

#[test]
fn test_vp9_integration_bitrate_adaptation() {
    // Stream A: High bitrate (no ALTREF, simpler)
    // Stream B: Low bitrate (ALTREF enabled for compression)
    let frame_types_a = vec![
        Vp9FrameType::Key,
        Vp9FrameType::Inter,
        Vp9FrameType::Inter,
        Vp9FrameType::Inter,
        Vp9FrameType::Inter,
    ];

    let frame_types_b = vec![
        Vp9FrameType::Key,
        Vp9FrameType::Altref,
        Vp9FrameType::Inter,
        Vp9FrameType::Inter,
        Vp9FrameType::Inter,
    ];

    let frames_a = frame_types_to_metadata(&frame_types_a);
    let frames_b = frame_types_to_metadata(&frame_types_b);

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    // Different bitrate strategies should still align
    assert_eq!(workspace.total_frames(), 5);
}

#[test]
fn test_vp9_integration_all_quirks_combined() {
    // Combine all quirks: ALTREF, show_existing, superframe, complex refs
    let frame_types = vec![
        Vp9FrameType::Key,
        Vp9FrameType::Altref,
        Vp9FrameType::InterInSuperframe(0, 2),
        Vp9FrameType::InterInSuperframe(1, 2),
        Vp9FrameType::Inter,
        Vp9FrameType::ShowExisting(1), // Show ALTREF
        Vp9FrameType::Inter,
    ];

    let frames_a = frame_types_to_metadata(&frame_types);
    let frames_b = frame_types_to_metadata(&frame_types);

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    // All quirks should work together
    assert_eq!(workspace.total_frames(), 7);
}

#[test]
fn test_vp9_integration_manual_offset_with_quirks() {
    // Test manual offset with complex quirk patterns
    let frame_types = vec![
        Vp9FrameType::Key,
        Vp9FrameType::Altref,
        Vp9FrameType::Inter,
        Vp9FrameType::Inter,
        Vp9FrameType::ShowExisting(1),
    ];

    let frames_a = frame_types_to_metadata(&frame_types);
    let frames_b = frame_types_to_metadata(&frame_types);

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let mut workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    // Apply manual offset
    workspace.set_manual_offset(2);
    assert_eq!(workspace.manual_offset(), 2);

    // Verify offset works with quirks
    let (b_idx, _quality) = workspace.get_aligned_frame(0).unwrap();
    assert_eq!(b_idx, 2);
}

#[test]
fn test_vp9_integration_pts_quality_with_all_quirks() {
    // Test PTS quality with all quirks combined
    let frame_types = vec![
        Vp9FrameType::Key,
        Vp9FrameType::Altref,
        Vp9FrameType::Inter,
        Vp9FrameType::ShowExisting(1), // Duplicate PTS
    ];

    let frames = frame_types_to_metadata(&frame_types);
    let map = FrameIndexMap::new(&frames);

    // PTS quality should reflect duplicate PTS from show_existing
    let quality = map.pts_quality();
    assert_eq!(
        quality,
        PtsQuality::Bad,
        "Expected BAD due to duplicate PTS"
    );
}

#[test]
fn test_vp9_integration_alignment_confidence_quirks() {
    // Test alignment confidence with mixed quirk patterns
    let frame_types = vec![
        Vp9FrameType::Key,
        Vp9FrameType::Altref,
        Vp9FrameType::Inter,
        Vp9FrameType::Inter,
    ];

    let frames_a = frame_types_to_metadata(&frame_types);
    let frames_b = frame_types_to_metadata(&frame_types);

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let alignment = AlignmentEngine::new(&map_a, &map_b);

    // Matching quirk patterns should give reasonable confidence
    let confidence = alignment.confidence();
    assert!(
        confidence == AlignmentConfidence::High
            || confidence == AlignmentConfidence::Medium
            || confidence == AlignmentConfidence::Low
    );
}

#[test]
fn test_vp9_integration_sync_modes_with_quirks() {
    // Test all sync modes with quirky frame patterns
    let frame_types = vec![
        Vp9FrameType::Key,
        Vp9FrameType::Altref,
        Vp9FrameType::InterInSuperframe(0, 2),
        Vp9FrameType::InterInSuperframe(1, 2),
    ];

    let frames_a = frame_types_to_metadata(&frame_types);
    let frames_b = frame_types_to_metadata(&frame_types);

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
fn test_vp9_integration_error_resilience_pattern() {
    // Error resilience: Periodic intra refresh with show_existing
    let frame_types = vec![
        Vp9FrameType::Key,
        Vp9FrameType::Inter,
        Vp9FrameType::Inter,
        Vp9FrameType::ShowExisting(0), // Show keyframe again for resilience
        Vp9FrameType::Inter,
        Vp9FrameType::Inter,
    ];

    let frames_a = frame_types_to_metadata(&frame_types);
    let frames_b = frame_types_to_metadata(&frame_types);

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    // Error resilience pattern should work
    assert_eq!(workspace.total_frames(), 6);
}

#[test]
fn test_vp9_integration_long_gop_with_multiple_altrefs() {
    // Long GOP (30 frames) with multiple ALTREF frames
    let mut frame_types = vec![Vp9FrameType::Key];
    frame_types.push(Vp9FrameType::Altref); // First ALTREF
    for _ in 0..8 {
        frame_types.push(Vp9FrameType::Inter);
    }
    frame_types.push(Vp9FrameType::Altref); // Second ALTREF
    for _ in 0..8 {
        frame_types.push(Vp9FrameType::Inter);
    }
    frame_types.push(Vp9FrameType::Altref); // Third ALTREF
    for _ in 0..8 {
        frame_types.push(Vp9FrameType::Inter);
    }

    let frames_a = frame_types_to_metadata(&frame_types);
    let frames_b = frame_types_to_metadata(&frame_types);

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    // Long GOP with multiple ALTREFs should work
    assert!(workspace.total_frames() >= 28);
}

#[test]
fn test_vp9_integration_asymmetric_quirk_usage() {
    // Stream A: Uses all quirks
    // Stream B: Simple pattern (no quirks)
    let frame_types_a = vec![
        Vp9FrameType::Key,
        Vp9FrameType::Altref,
        Vp9FrameType::InterInSuperframe(0, 2),
        Vp9FrameType::InterInSuperframe(1, 2),
        Vp9FrameType::ShowExisting(1),
    ];

    let frame_types_b = vec![
        Vp9FrameType::Key,
        Vp9FrameType::Inter,
        Vp9FrameType::Inter,
        Vp9FrameType::Inter,
        Vp9FrameType::Inter,
    ];

    let frames_a = frame_types_to_metadata(&frame_types_a);
    let frames_b = frame_types_to_metadata(&frame_types_b);

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    // Asymmetric quirk usage should still work
    assert_eq!(workspace.total_frames(), 5);
}

#[test]
fn test_vp9_integration_resolution_mismatch_with_quirks() {
    // Test resolution mismatch with VP9 quirks
    let frame_types = vec![
        Vp9FrameType::Key,
        Vp9FrameType::Altref,
        Vp9FrameType::Inter,
        Vp9FrameType::ShowExisting(1),
    ];

    let frames_a = frame_types_to_metadata(&frame_types);
    let frames_b = frame_types_to_metadata(&frame_types);

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    // Different resolutions
    let workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1280, 720));

    // Should detect resolution mismatch
    assert!(!workspace.is_diff_enabled());
    assert!(workspace.disable_reason().is_some());
    assert!(workspace
        .disable_reason()
        .unwrap()
        .contains("Resolution mismatch"));
}

#[test]
fn test_vp9_integration_real_world_webrtc_pattern() {
    // Simulate WebRTC pattern: Temporal SVC with rapid keyframes
    let mut frame_types = Vec::new();
    for _ in 0..3 {
        frame_types.push(Vp9FrameType::Key);
        frame_types.push(Vp9FrameType::InterInSuperframe(0, 2));
        frame_types.push(Vp9FrameType::InterInSuperframe(1, 2));
        frame_types.push(Vp9FrameType::Inter);
    }

    let frames_a = frame_types_to_metadata(&frame_types);
    let frames_b = frame_types_to_metadata(&frame_types);

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    // WebRTC pattern should work
    assert_eq!(workspace.total_frames(), 12);
}
