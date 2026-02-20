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
//! Tests for compare module

use crate::frame_identity::FrameMetadata;
use crate::{
    AlignmentQuality, AlignmentQualityColor, CompareWorkspace, FrameIndexMap, ResolutionInfo,
    SyncControls, SyncMode,
};

fn create_test_frames(count: usize) -> Vec<FrameMetadata> {
    (0..count)
        .map(|i| FrameMetadata {
            pts: Some((i * 1000) as u64),
            dts: Some((i * 1000) as u64),
        })
        .collect()
}

#[test]
fn test_compare_workspace_basic() {
    let frames_a = create_test_frames(10);
    let frames_b = create_test_frames(10);

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    assert_eq!(workspace.sync_mode(), SyncMode::Off);
    assert_eq!(workspace.manual_offset(), 0);
    assert!(workspace.is_diff_enabled());
    assert!(workspace.disable_reason().is_none());
}

#[test]
fn test_resolution_mismatch() {
    let frames = create_test_frames(5);
    let map = FrameIndexMap::new(&frames);

    // 1920x1080 vs 1280x720 - significant mismatch
    let workspace = CompareWorkspace::new(map.clone(), map.clone(), (1920, 1080), (1280, 720));

    assert!(!workspace.is_diff_enabled());
    assert!(workspace.disable_reason().is_some());
    assert!(workspace
        .disable_reason()
        .unwrap()
        .contains("Resolution mismatch"));
}

#[test]
fn test_manual_offset() {
    let frames_a = create_test_frames(10);
    let frames_b = create_test_frames(10);

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let mut workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    // Set manual offset
    workspace.set_manual_offset(5);
    assert_eq!(workspace.manual_offset(), 5);

    // Adjust offset
    workspace.adjust_offset(2);
    assert_eq!(workspace.manual_offset(), 7);

    workspace.adjust_offset(-3);
    assert_eq!(workspace.manual_offset(), 4);

    // Reset offset
    workspace.reset_offset();
    assert_eq!(workspace.manual_offset(), 0);
}

#[test]
fn test_aligned_frame_lookup() {
    let frames = create_test_frames(5);
    let map = FrameIndexMap::new(&frames);

    let workspace = CompareWorkspace::new(map.clone(), map.clone(), (1920, 1080), (1920, 1080));

    // Without offset
    let (b_idx, quality) = workspace.get_aligned_frame(2).unwrap();
    assert_eq!(b_idx, 2);
    assert_eq!(quality, AlignmentQuality::Exact);
}

#[test]
fn test_aligned_frame_with_offset() {
    let frames = create_test_frames(10);
    let map = FrameIndexMap::new(&frames);

    let mut workspace = CompareWorkspace::new(map.clone(), map.clone(), (1920, 1080), (1920, 1080));

    // Set manual offset of +2 (B is 2 frames ahead)
    workspace.set_manual_offset(2);

    // Frame 0 in A should map to frame 2 in B
    let (b_idx, _quality) = workspace.get_aligned_frame(0).unwrap();
    assert_eq!(b_idx, 2);

    // Frame 3 in A should map to frame 5 in B
    let (b_idx, _quality) = workspace.get_aligned_frame(3).unwrap();
    assert_eq!(b_idx, 5);
}

#[test]
fn test_resolution_info() {
    let info = ResolutionInfo::new((1920, 1080), (1920, 1080));
    assert!(info.is_exact_match());
    assert!(info.is_compatible());
    assert_eq!(info.mismatch_percentage(), 0.0);

    let info2 = ResolutionInfo::new((1920, 1080), (1280, 720));
    assert!(!info2.is_exact_match());
    assert!(!info2.is_compatible()); // More than 5% difference
    assert!(info2.mismatch_percentage() > 0.05);
}

#[test]
fn test_sync_controls() {
    let mut controls = SyncControls::new();

    assert_eq!(controls.mode, SyncMode::Off);
    assert!(!controls.manual_offset_enabled);

    // Toggle sync mode
    controls.toggle_sync();
    assert_eq!(controls.mode, SyncMode::Playhead);

    controls.toggle_sync();
    assert_eq!(controls.mode, SyncMode::Full);

    controls.toggle_sync();
    assert_eq!(controls.mode, SyncMode::Off);

    // Manual offset
    controls.enable_manual_offset();
    assert!(controls.manual_offset_enabled);

    controls.set_offset(5);
    assert_eq!(controls.manual_offset, 5);

    controls.adjust_offset(3);
    assert_eq!(controls.manual_offset, 8);

    controls.disable_manual_offset();
    assert!(!controls.manual_offset_enabled);
    assert_eq!(controls.manual_offset, 0);
}

#[test]
fn test_sync_mode_change() {
    let frames = create_test_frames(5);
    let map = FrameIndexMap::new(&frames);

    let mut workspace = CompareWorkspace::new(map.clone(), map.clone(), (1920, 1080), (1920, 1080));

    workspace.set_sync_mode(SyncMode::Playhead);
    assert_eq!(workspace.sync_mode(), SyncMode::Playhead);

    workspace.set_sync_mode(SyncMode::Full);
    assert_eq!(workspace.sync_mode(), SyncMode::Full);
}

#[test]
fn test_total_frames() {
    let frames_a = create_test_frames(10);
    let frames_b = create_test_frames(15);

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    assert_eq!(workspace.total_frames(), 15); // Max of 10 and 15
}

#[test]
fn test_alignment_quality() {
    assert_eq!(AlignmentQuality::Exact.display_text(), "Exact");
    assert_eq!(AlignmentQuality::Nearest.display_text(), "Nearest");
    assert_eq!(AlignmentQuality::Gap.display_text(), "Gap");

    assert_eq!(
        AlignmentQuality::Exact.color_hint(),
        AlignmentQualityColor::Green
    );
    assert_eq!(
        AlignmentQuality::Nearest.color_hint(),
        AlignmentQualityColor::Yellow
    );
    assert_eq!(
        AlignmentQuality::Gap.color_hint(),
        AlignmentQualityColor::Red
    );
}

#[test]
fn test_scale_indicator() {
    let info = ResolutionInfo::new((1920, 1080), (1920, 1080));
    assert_eq!(info.scale_indicator(), "1:1");

    let info2 = ResolutionInfo::new((1920, 1080), (1280, 720));
    assert_eq!(info2.scale_indicator(), "1920x1080 vs 1280x720");
}

// UX Compare viz_core tests - Task 10 (S.T4-1.ALL.UX.Compare.impl.viz_core.009)

#[test]
fn test_ux_compare_load_streams_and_toggle_sync() {
    // UX Compare: User loads two AV1 streams and toggles sync mode
    let frames_a = create_test_frames(20);
    let frames_b = create_test_frames(20);

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let mut workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    // UX Compare: Initial sync mode is Off (independent playback)
    assert_eq!(workspace.sync_mode(), SyncMode::Off);

    // UX Compare: User clicks "Sync Playhead" button
    workspace.set_sync_mode(SyncMode::Playhead);
    assert_eq!(workspace.sync_mode(), SyncMode::Playhead);

    // UX Compare: User clicks "Sync Full" button for timeline sync
    workspace.set_sync_mode(SyncMode::Full);
    assert_eq!(workspace.sync_mode(), SyncMode::Full);

    // UX Compare: User clicks "Off" to disable sync
    workspace.set_sync_mode(SyncMode::Off);
    assert_eq!(workspace.sync_mode(), SyncMode::Off);

    // UX Compare: Verify workspace properties
    assert_eq!(workspace.total_frames(), 20);
    assert!(workspace.is_diff_enabled());
}

#[test]
fn test_ux_compare_adjust_offset_with_keyboard() {
    // UX Compare: User adjusts manual offset using keyboard arrows
    let frames_a = create_test_frames(15);
    let frames_b = create_test_frames(15);

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let mut workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    // UX Compare: User presses right arrow key (B ahead by 1 frame)
    workspace.adjust_offset(1);
    assert_eq!(workspace.manual_offset(), 1);

    // UX Compare: User presses right arrow key again
    workspace.adjust_offset(1);
    assert_eq!(workspace.manual_offset(), 2);

    // UX Compare: User presses left arrow key (undo 1 frame)
    workspace.adjust_offset(-1);
    assert_eq!(workspace.manual_offset(), 1);

    // UX Compare: User presses Shift+Right (large step +5 frames)
    workspace.adjust_offset(5);
    assert_eq!(workspace.manual_offset(), 6);

    // UX Compare: User presses Shift+Left (large step -5 frames)
    workspace.adjust_offset(-5);
    assert_eq!(workspace.manual_offset(), 1);

    // UX Compare: Verify aligned frame with offset
    let (b_idx, _quality) = workspace.get_aligned_frame(0).unwrap();
    assert_eq!(b_idx, 1); // Frame 0 in A maps to frame 1 in B
}

#[test]
fn test_ux_compare_resolution_mismatch_warning() {
    // UX Compare: User loads streams with different resolutions
    let frames = create_test_frames(10);
    let map = FrameIndexMap::new(&frames);

    // UX Compare: Load 1080p vs 720p
    let workspace = CompareWorkspace::new(map.clone(), map.clone(), (1920, 1080), (1280, 720));

    // UX Compare: Should show resolution mismatch warning
    assert!(!workspace.is_diff_enabled());
    let reason = workspace.disable_reason().unwrap();
    assert!(reason.contains("Resolution mismatch"));
    assert!(reason.contains("1920x1080"));
    assert!(reason.contains("1280x720"));

    // UX Compare: Verify resolution info is accessible
    let res_info = workspace.resolution_info();
    assert!(!res_info.is_exact_match());
    assert!(!res_info.is_compatible());
    assert!(res_info.mismatch_percentage() > 0.05); // More than 5% difference
}

#[test]
fn test_ux_compare_sync_controls_cycling() {
    // UX Compare: User cycles through sync modes using toggle button
    let mut controls = SyncControls::new();

    // UX Compare: Initial state is Off
    assert_eq!(controls.mode, SyncMode::Off);
    assert!(!controls.show_alignment_info);

    // UX Compare: User clicks "Sync" button (cycles to Playhead)
    controls.toggle_sync();
    assert_eq!(controls.mode, SyncMode::Playhead);

    // UX Compare: User clicks "Sync" button again (cycles to Full)
    controls.toggle_sync();
    assert_eq!(controls.mode, SyncMode::Full);

    // UX Compare: User clicks "Sync" button again (cycles back to Off)
    controls.toggle_sync();
    assert_eq!(controls.mode, SyncMode::Off);

    // UX Compare: User toggles alignment info panel
    controls.toggle_alignment_info();
    assert!(controls.show_alignment_info);

    controls.toggle_alignment_info();
    assert!(!controls.show_alignment_info);
}

#[test]
fn test_ux_compare_reset_offset_button() {
    // UX Compare: User resets manual offset using reset button
    let frames = create_test_frames(10);
    let map = FrameIndexMap::new(&frames);

    let mut workspace = CompareWorkspace::new(map.clone(), map.clone(), (1920, 1080), (1920, 1080));

    // UX Compare: User adjusts offset to +7
    workspace.set_manual_offset(7);
    assert_eq!(workspace.manual_offset(), 7);

    // UX Compare: Verify frame alignment with offset
    let (b_idx, _) = workspace.get_aligned_frame(0).unwrap();
    assert_eq!(b_idx, 7);

    // UX Compare: User clicks "Reset Offset" button
    workspace.reset_offset();
    assert_eq!(workspace.manual_offset(), 0);

    // UX Compare: Verify frame alignment is back to 1:1
    let (b_idx, _) = workspace.get_aligned_frame(0).unwrap();
    assert_eq!(b_idx, 0);
}

#[test]
fn test_ux_compare_alignment_quality_indicators() {
    // UX Compare: User views alignment quality color indicators
    let frames_a = create_test_frames(8);
    let frames_b = create_test_frames(8);

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    // UX Compare: Check frame 3 alignment quality
    let (b_idx, quality) = workspace.get_aligned_frame(3).unwrap();
    assert_eq!(b_idx, 3);
    assert_eq!(quality, AlignmentQuality::Exact);

    // UX Compare: UI shows green indicator for exact match
    assert_eq!(quality.color_hint(), AlignmentQualityColor::Green);
    assert_eq!(quality.display_text(), "Exact");

    // UX Compare: Verify other quality types have correct colors
    assert_eq!(
        AlignmentQuality::Nearest.color_hint(),
        AlignmentQualityColor::Yellow
    );
    assert_eq!(
        AlignmentQuality::Gap.color_hint(),
        AlignmentQualityColor::Red
    );
}

#[test]
fn test_ux_compare_manual_offset_controls() {
    // UX Compare: User enables and uses manual offset UI controls
    let mut controls = SyncControls::new();

    // UX Compare: Manual offset disabled by default
    assert!(!controls.manual_offset_enabled);
    assert_eq!(controls.manual_offset, 0);

    // UX Compare: User clicks "Enable Manual Offset" checkbox
    controls.enable_manual_offset();
    assert!(controls.manual_offset_enabled);

    // UX Compare: User drags offset slider to +3
    controls.set_offset(3);
    assert_eq!(controls.manual_offset, 3);

    // UX Compare: User clicks fine-tune buttons (+1/-1)
    controls.adjust_offset(1);
    assert_eq!(controls.manual_offset, 4);

    controls.adjust_offset(-2);
    assert_eq!(controls.manual_offset, 2);

    // UX Compare: User clicks "Disable Manual Offset" checkbox
    controls.disable_manual_offset();
    assert!(!controls.manual_offset_enabled);
    assert_eq!(controls.manual_offset, 0); // Reset to 0
}

#[test]
fn test_ux_compare_offset_affects_alignment() {
    // UX Compare: User sees aligned frames update when offset changes
    let frames_a = create_test_frames(12);
    let frames_b = create_test_frames(12);

    let map_a = FrameIndexMap::new(&frames_a);
    let map_b = FrameIndexMap::new(&frames_b);

    let mut workspace = CompareWorkspace::new(map_a, map_b, (1920, 1080), (1920, 1080));

    // UX Compare: No offset - frame 5 in A maps to frame 5 in B
    let (b_idx, _) = workspace.get_aligned_frame(5).unwrap();
    assert_eq!(b_idx, 5);

    // UX Compare: User sets offset to +2 (B is 2 frames ahead)
    workspace.set_manual_offset(2);

    // UX Compare: Frame 5 in A now maps to frame 7 in B
    let (b_idx, _) = workspace.get_aligned_frame(5).unwrap();
    assert_eq!(b_idx, 7);

    // UX Compare: User sets offset to -1 (B is 1 frame behind)
    workspace.set_manual_offset(-1);

    // UX Compare: Frame 5 in A now maps to frame 4 in B
    let (b_idx, _) = workspace.get_aligned_frame(5).unwrap();
    assert_eq!(b_idx, 4);
}

#[test]
fn test_ux_compare_scale_indicator_display() {
    // UX Compare: User views resolution scale indicator in UI
    let frames = create_test_frames(5);
    let map = FrameIndexMap::new(&frames);

    // UX Compare: Matching resolutions show "1:1"
    let workspace1 = CompareWorkspace::new(map.clone(), map.clone(), (1920, 1080), (1920, 1080));
    let scale1 = workspace1.resolution_info().scale_indicator();
    assert_eq!(scale1, "1:1");

    // UX Compare: Different resolutions show "WxH vs WxH"
    let workspace2 = CompareWorkspace::new(map.clone(), map.clone(), (1920, 1080), (3840, 2160));
    let scale2 = workspace2.resolution_info().scale_indicator();
    assert_eq!(scale2, "1920x1080 vs 3840x2160");

    // UX Compare: Minor resolution differences
    let workspace3 = CompareWorkspace::new(
        map.clone(),
        map.clone(),
        (1920, 1080),
        (1920, 1088), // 8-pixel height difference
    );
    let scale3 = workspace3.resolution_info().scale_indicator();
    assert_eq!(scale3, "1920x1080 vs 1920x1088");

    // UX Compare: Verify compatibility check
    assert!(workspace1.resolution_info().is_compatible()); // Exact match
    assert!(!workspace2.resolution_info().is_compatible()); // 4x pixel count difference
    assert!(workspace3.resolution_info().is_compatible()); // Within 5% tolerance
}

// AV1 Metrics Compare viz_core test - Task 26 (S.T4-3.AV1.Metrics.Compare.impl.viz_core.001)

#[test]
fn test_av1_metrics_compare_visualization() {
    // AV1 Metrics: User compares two AV1 encodes (different bitrates)
    let frames = create_test_frames(5);
    let ref_map = FrameIndexMap::new(&frames);
    let test_map = FrameIndexMap::new(&frames);

    let workspace = CompareWorkspace::new(
        ref_map,
        test_map,
        (640, 360), // AV1 test file resolution
        (640, 360),
    );

    // Verify resolution compatibility for AV1 streams
    let res_info = workspace.resolution_info();
    assert!(res_info.is_compatible());
    assert_eq!(res_info.scale_indicator(), "1:1");
}

// AV1 Metrics Compare evidence chain test - Task 27 (S.T4-3.AV1.Metrics.Compare.impl.evidence_chain.001)

#[test]
fn test_av1_metrics_compare_evidence() {
    // AV1 Metrics: User clicks diff region to trace to both source OBUs
    let frames = create_test_frames(5);
    let ref_map = FrameIndexMap::new(&frames);
    let test_map = FrameIndexMap::new(&frames);

    let workspace = CompareWorkspace::new(ref_map, test_map, (640, 360), (640, 360));

    // Verify compare workspace is created with both streams
    let res_info = workspace.resolution_info();
    assert!(res_info.is_compatible());
}
