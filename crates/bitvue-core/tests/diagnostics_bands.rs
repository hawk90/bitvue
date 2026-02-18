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
//! Tests for diagnostics_bands module

use bitvue_core::{
    DiagnosticBandType, DiagnosticsBands, ErrorBurst, ErrorBurstDetection, ReorderEntry,
    SceneChange,
};

#[test]
fn test_diagnostic_band_type_name() {
    assert_eq!(DiagnosticBandType::SceneChange.name(), "Scene Changes");
    assert_eq!(
        DiagnosticBandType::ReorderMismatch.name(),
        "Reorder Mismatch"
    );
    assert_eq!(DiagnosticBandType::ErrorBurst.name(), "Error Bursts");
}

#[test]
fn test_diagnostic_band_type_color() {
    assert_eq!(DiagnosticBandType::SceneChange.color_hint(), "green");
    assert_eq!(DiagnosticBandType::ReorderMismatch.color_hint(), "orange");
    assert_eq!(DiagnosticBandType::ErrorBurst.color_hint(), "red");
}

#[test]
fn test_scene_change_creation() {
    let sc = SceneChange::new(10, 0.85);
    assert_eq!(sc.display_idx, 10);
    assert_eq!(sc.confidence, 0.85);
    assert!(sc.description.is_none());

    let sc = sc.with_description("Cut to interior".to_string());
    assert_eq!(sc.description, Some("Cut to interior".to_string()));
}

#[test]
fn test_reorder_entry_depth() {
    let entry = ReorderEntry::new(5, 2000, 1000);
    assert_eq!(entry.display_idx, 5);
    assert_eq!(entry.pts, 2000);
    assert_eq!(entry.dts, 1000);
    assert_eq!(entry.depth, 1000);

    // Reverse case
    let entry = ReorderEntry::new(6, 1000, 2000);
    assert_eq!(entry.depth, 1000);
}

#[test]
fn test_reorder_entry_depth_frames() {
    let entry = ReorderEntry::new(5, 2000, 1000);
    let frames = entry.depth_frames(33); // 30fps
    assert_eq!(frames, 30); // 1000ms / 33ms ≈ 30 frames
}

#[test]
fn test_error_burst_creation() {
    let burst = ErrorBurst::new(10, 15, 4);
    assert_eq!(burst.start_idx, 10);
    assert_eq!(burst.end_idx, 15);
    assert_eq!(burst.error_count, 4);
    assert_eq!(burst.length(), 6); // 15 - 10 + 1
    assert!((burst.density() - 0.666).abs() < 0.01); // 4/6
}

#[test]
fn test_error_burst_severity() {
    let burst1 = ErrorBurst::new(10, 15, 4); // 4 errors in 6 frames
    let burst2 = ErrorBurst::new(20, 25, 2); // 2 errors in 6 frames

    assert!(burst1.severity > burst2.severity);
}

#[test]
fn test_error_burst_detection_single() {
    let errors = vec![10];
    let bursts = ErrorBurstDetection::detect_bursts(&errors, 5);

    assert_eq!(bursts.len(), 1);
    assert_eq!(bursts[0].start_idx, 10);
    assert_eq!(bursts[0].end_idx, 10);
    assert_eq!(bursts[0].error_count, 1);
}

#[test]
fn test_error_burst_detection_continuous() {
    let errors = vec![10, 11, 12, 13];
    let bursts = ErrorBurstDetection::detect_bursts(&errors, 5);

    assert_eq!(bursts.len(), 1);
    assert_eq!(bursts[0].start_idx, 10);
    assert_eq!(bursts[0].end_idx, 13);
    assert_eq!(bursts[0].error_count, 4);
}

#[test]
fn test_error_burst_detection_gap() {
    let errors = vec![10, 12, 14]; // Gap of 2
    let bursts = ErrorBurstDetection::detect_bursts(&errors, 5);

    // Should be one burst (gaps <= 5)
    assert_eq!(bursts.len(), 1);
    assert_eq!(bursts[0].start_idx, 10);
    assert_eq!(bursts[0].end_idx, 14);
    assert_eq!(bursts[0].error_count, 3);
}

#[test]
fn test_error_burst_detection_multiple_bursts() {
    let errors = vec![10, 11, 12, 50, 51, 52]; // Large gap
    let bursts = ErrorBurstDetection::detect_bursts(&errors, 5);

    assert_eq!(bursts.len(), 2);
    assert_eq!(bursts[0].start_idx, 10);
    assert_eq!(bursts[0].end_idx, 12);
    assert_eq!(bursts[1].start_idx, 50);
    assert_eq!(bursts[1].end_idx, 52);
}

#[test]
fn test_error_burst_detection_empty() {
    let errors: Vec<usize> = vec![];
    let bursts = ErrorBurstDetection::detect_bursts(&errors, 5);
    assert_eq!(bursts.len(), 0);
}

#[test]
fn test_top_bursts() {
    let bursts = vec![
        ErrorBurst::new(10, 12, 2), // severity = 2/3 = 0.66
        ErrorBurst::new(20, 23, 4), // severity = 4/4 = 1.0
        ErrorBurst::new(30, 35, 3), // severity = 3/6 = 0.5
    ];

    let top = ErrorBurstDetection::top_bursts(&bursts, 2);
    assert_eq!(top.len(), 2);
    assert_eq!(top[0], 1); // burst[1] has highest severity
    assert_eq!(top[1], 0); // burst[0] has second highest
}

#[test]
fn test_diagnostics_bands_creation() {
    let bands = DiagnosticsBands::new();
    assert_eq!(bands.scene_changes.len(), 0);
    assert_eq!(bands.reorder_entries.len(), 0);
    assert_eq!(bands.error_bursts.len(), 0);
    assert!(bands.scene_change_visible);
    assert!(bands.reorder_visible);
    assert!(bands.error_burst_visible);
}

#[test]
fn test_diagnostics_bands_add_entries() {
    let mut bands = DiagnosticsBands::new();

    bands.add_scene_change(SceneChange::new(10, 0.9));
    bands.add_reorder_entry(ReorderEntry::new(5, 2000, 1000));

    assert_eq!(bands.scene_changes.len(), 1);
    assert_eq!(bands.reorder_entries.len(), 1);
}

#[test]
fn test_diagnostics_bands_detect_bursts() {
    let mut bands = DiagnosticsBands::new();
    let errors = vec![10, 11, 12, 50, 51];

    bands.detect_error_bursts(&errors, 5);

    assert_eq!(bands.error_bursts.len(), 2);
}

#[test]
fn test_diagnostics_bands_auto_select() {
    let mut bands = DiagnosticsBands::new();
    // Burst 1: 3 errors spread over 5 frames (10, 12, 14) = 3/5 = 0.6 severity
    // Burst 2: 4 errors in 4 consecutive frames (50-53) = 4/4 = 1.0 severity
    let errors = vec![10, 12, 14, 50, 51, 52, 53];

    bands.detect_error_bursts(&errors, 5);
    bands.auto_select_worst_burst();

    assert!(bands.selected_burst.is_some());
    let selected = bands.get_selected_burst().unwrap();
    assert_eq!(selected.start_idx, 50); // Second burst has higher severity (1.0 > 0.6)
}

#[test]
fn test_diagnostics_bands_toggle() {
    let mut bands = DiagnosticsBands::new();
    assert!(bands.scene_change_visible);

    bands.toggle_band(DiagnosticBandType::SceneChange);
    assert!(!bands.scene_change_visible);

    bands.toggle_band(DiagnosticBandType::SceneChange);
    assert!(bands.scene_change_visible);
}

#[test]
fn test_diagnostics_bands_reorder_stats() {
    let mut bands = DiagnosticsBands::new();
    bands.add_reorder_entry(ReorderEntry::new(5, 2000, 1000));
    bands.add_reorder_entry(ReorderEntry::new(6, 3000, 1000));
    bands.add_reorder_entry(ReorderEntry::new(7, 2500, 1000));

    assert_eq!(bands.reorder_count(), 3);
    assert_eq!(bands.max_reorder_depth(), 2000);
}

#[test]
fn test_diagnostics_bands_error_count() {
    let mut bands = DiagnosticsBands::new();
    let errors = vec![10, 11, 12, 50, 51];

    bands.detect_error_bursts(&errors, 5);

    assert_eq!(bands.total_error_count(), 5);
}

#[test]
fn test_diagnostics_bands_select_burst() {
    let mut bands = DiagnosticsBands::new();
    bands.detect_error_bursts(&vec![10, 11, 50, 51], 5);

    bands.select_burst(1);
    assert_eq!(bands.selected_burst, Some(1));

    bands.clear_burst_selection();
    assert!(bands.selected_burst.is_none());
}

// AV1 TimelineBands viz_core test - Task 19 (S.T4-2.AV1.Timeline.TimelineBands.impl.viz_core.001)

#[test]
fn test_av1_timeline_bands_diagnostic_features() {
    // AV1 TimelineBands: User views diagnostic bands for AV1 stream
    let mut bands = DiagnosticsBands::new();

    // AV1 TimelineBands: Scene changes at KEY_FRAME boundaries
    // Frame 0: KEY_FRAME (scene change with high confidence)
    bands.add_scene_change(SceneChange::new(0, 0.95).with_description("Opening scene".to_string()));

    // Frame 30: KEY_FRAME (scene change, medium confidence)
    bands.add_scene_change(
        SceneChange::new(30, 0.75).with_description("Cut to exterior".to_string()),
    );

    // Frame 60: KEY_FRAME (scene change, high confidence)
    bands.add_scene_change(
        SceneChange::new(60, 0.92).with_description("Fade to interior".to_string()),
    );

    // AV1 TimelineBands: Verify scene change entries
    assert_eq!(bands.scene_changes.len(), 3);
    assert_eq!(bands.scene_changes[0].display_idx, 0);
    assert_eq!(bands.scene_changes[0].confidence, 0.95);
    assert_eq!(bands.scene_changes[1].display_idx, 30);
    assert_eq!(bands.scene_changes[2].display_idx, 60);

    // AV1 TimelineBands: Reorder mismatches for AV1 temporal layering
    // AV1 supports B-frames with display reordering (PTS ≠ DTS)
    // Frame 1: B-frame displayed after frame 3 (PTS=1000, DTS=3000)
    bands.add_reorder_entry(ReorderEntry::new(1, 1000, 3000));

    // Frame 2: B-frame displayed after frame 4 (PTS=2000, DTS=4000)
    bands.add_reorder_entry(ReorderEntry::new(2, 2000, 4000));

    // Frame 5: B-frame with smaller reorder (PTS=5000, DTS=6000)
    bands.add_reorder_entry(ReorderEntry::new(5, 5000, 6000));

    // AV1 TimelineBands: Verify reorder entries
    assert_eq!(bands.reorder_count(), 3);
    assert_eq!(bands.reorder_entries[0].depth, 2000);
    assert_eq!(bands.reorder_entries[1].depth, 2000);
    assert_eq!(bands.reorder_entries[2].depth, 1000);
    assert_eq!(bands.max_reorder_depth(), 2000);

    // AV1 TimelineBands: Verify reorder depth in frames (assuming 33ms frame duration)
    assert_eq!(bands.reorder_entries[0].depth_frames(33), 60); // 2000ms / 33ms ≈ 60 frames

    // AV1 TimelineBands: Error bursts (OBU parsing errors, tile group errors)
    // Burst 1: Tile group errors in frames 10-14 (3 errors spread over 5 frames)
    // Burst 2: Severe OBU corruption in frames 50-53 (4 consecutive errors)
    let error_indices = vec![10, 12, 14, 50, 51, 52, 53];
    bands.detect_error_bursts(&error_indices, 5);

    // AV1 TimelineBands: Verify error burst detection
    assert_eq!(bands.error_bursts.len(), 2);
    assert_eq!(bands.error_bursts[0].start_idx, 10);
    assert_eq!(bands.error_bursts[0].end_idx, 14);
    assert_eq!(bands.error_bursts[0].error_count, 3);
    assert_eq!(bands.error_bursts[1].start_idx, 50);
    assert_eq!(bands.error_bursts[1].end_idx, 53);
    assert_eq!(bands.error_bursts[1].error_count, 4);

    // AV1 TimelineBands: Auto-select worst burst (highest severity)
    bands.auto_select_worst_burst();
    assert!(bands.selected_burst.is_some());
    let selected = bands.get_selected_burst().unwrap();
    assert_eq!(selected.start_idx, 50); // Burst 2 has higher severity (4/4 = 1.0 > 3/5 = 0.6)
    assert_eq!(selected.error_count, 4);
    assert_eq!(selected.severity, 1.0);

    // AV1 TimelineBands: Verify total error count
    assert_eq!(bands.total_error_count(), 7);

    // AV1 TimelineBands: Verify band visibility toggles
    assert!(bands.scene_change_visible);
    assert!(bands.reorder_visible);
    assert!(bands.error_burst_visible);

    bands.toggle_band(DiagnosticBandType::SceneChange);
    assert!(!bands.scene_change_visible);
    bands.toggle_band(DiagnosticBandType::SceneChange);
    assert!(bands.scene_change_visible);

    // AV1 TimelineBands: Verify band type properties
    assert_eq!(DiagnosticBandType::SceneChange.name(), "Scene Changes");
    assert_eq!(DiagnosticBandType::SceneChange.color_hint(), "green");
    assert_eq!(
        DiagnosticBandType::ReorderMismatch.name(),
        "Reorder Mismatch"
    );
    assert_eq!(DiagnosticBandType::ReorderMismatch.color_hint(), "orange");
    assert_eq!(DiagnosticBandType::ErrorBurst.name(), "Error Bursts");
    assert_eq!(DiagnosticBandType::ErrorBurst.color_hint(), "red");
}
