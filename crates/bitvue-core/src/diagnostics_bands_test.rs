// Diagnostics Bands module tests
//
// Per generate-tests skill: Arrange-Act-Assert pattern with fixtures
// and edge case coverage.


// ============================================================================
// Fixtures
// ============================================================================

/// Create a test scene change
#[allow(dead_code)]
fn create_test_scene_change(idx: usize, confidence: f32) -> SceneChange {
    SceneChange::new(idx, confidence)
}

/// Create a test reorder entry
#[allow(dead_code)]
fn create_test_reorder_entry(idx: usize, pts: u64, dts: u64) -> ReorderEntry {
    ReorderEntry::new(idx, pts, dts)
}

/// Create a test error burst
#[allow(dead_code)]
fn create_test_error_burst(start: usize, end: usize, count: usize) -> ErrorBurst {
    ErrorBurst::new(start, end, count)
}

/// Create a test diagnostics bands
#[allow(dead_code)]
fn create_test_diagnostics_bands() -> DiagnosticsBands {
    DiagnosticsBands::new()
}

// ============================================================================
// DiagnosticBandType Tests
// ============================================================================

#[cfg(test)]
mod diagnostic_band_type_tests {
#[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_scene_change_name() {
        // Arrange & Act
        let name = DiagnosticBandType::SceneChange.name();

        // Assert
        assert_eq!(name, "Scene Changes");
    }

    #[test]
    fn test_reorder_mismatch_name() {
        // Arrange & Act
        let name = DiagnosticBandType::ReorderMismatch.name();

        // Assert
        assert_eq!(name, "Reorder Mismatch");
    }

    #[test]
    fn test_error_burst_name() {
        // Arrange & Act
        let name = DiagnosticBandType::ErrorBurst.name();

        // Assert
        assert_eq!(name, "Error Bursts");
    }

    #[test]
    fn test_scene_change_color_hint() {
        // Arrange & Act
        let color = DiagnosticBandType::SceneChange.color_hint();

        // Assert
        assert_eq!(color, "green");
    }

    #[test]
    fn test_reorder_mismatch_color_hint() {
        // Arrange & Act
        let color = DiagnosticBandType::ReorderMismatch.color_hint();

        // Assert
        assert_eq!(color, "orange");
    }

    #[test]
    fn test_error_burst_color_hint() {
        // Arrange & Act
        let color = DiagnosticBandType::ErrorBurst.color_hint();

        // Assert
        assert_eq!(color, "red");
    }
}

// ============================================================================
// SceneChange Tests
// ============================================================================

#[cfg(test)]
mod scene_change_tests {
#[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_new_creates_scene_change() {
        // Arrange & Act
        let scene = SceneChange::new(100, 0.85);

        // Assert
        assert_eq!(scene.display_idx, 100);
        assert_eq!(scene.confidence, 0.85);
        assert!(scene.description.is_none());
    }

    #[test]
    fn test_with_description() {
        // Arrange
        let scene = SceneChange::new(100, 0.85);

        // Act
        let scene = scene.with_description("Cut to next scene".to_string());

        // Assert
        assert_eq!(scene.description, Some("Cut to next scene".to_string()));
    }

    #[test]
    fn test_confidence_zero() {
        // Arrange & Act
        let scene = SceneChange::new(50, 0.0);

        // Assert
        assert_eq!(scene.confidence, 0.0);
    }

    #[test]
    fn test_confidence_one() {
        // Arrange & Act
        let scene = SceneChange::new(50, 1.0);

        // Assert
        assert_eq!(scene.confidence, 1.0);
    }
}

// ============================================================================
// ReorderEntry Tests
// ============================================================================

#[cfg(test)]
mod reorder_entry_tests {
#[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_new_creates_reorder_entry() {
        // Arrange & Act
        let entry = ReorderEntry::new(10, 1000, 900);

        // Assert
        assert_eq!(entry.display_idx, 10);
        assert_eq!(entry.pts, 1000);
        assert_eq!(entry.dts, 900);
        assert_eq!(entry.depth, 100); // abs_diff(1000, 900)
    }

    #[test]
    fn test_depth_calculation_positive() {
        // Arrange & Act
        let entry = ReorderEntry::new(5, 2000, 1000);

        // Assert
        assert_eq!(entry.depth, 1000);
    }

    #[test]
    fn test_depth_calculation_negative() {
        // Arrange & Act
        let entry = ReorderEntry::new(5, 1000, 2000);

        // Assert
        assert_eq!(entry.depth, 1000); // abs_diff
    }

    #[test]
    fn test_depth_frames_zero_duration() {
        // Arrange
        let entry = create_test_reorder_entry(10, 1000, 900);

        // Act
        let depth = entry.depth_frames(0);

        // Assert
        assert_eq!(depth, 0); // Should return 0 to avoid division by zero
    }

    #[test]
    fn test_depth_frames_calculated() {
        // Arrange
        let entry = create_test_reorder_entry(10, 1000, 900); // depth = 100ms

        // Act
        let depth = entry.depth_frames(33); // 33ms per frame

        // Assert
        assert_eq!(depth, 3); // 100 / 33 = 3
    }

    #[test]
    fn test_equal_pts_dts() {
        // Arrange & Act
        let entry = ReorderEntry::new(10, 1000, 1000);

        // Assert
        assert_eq!(entry.depth, 0); // No reorder
    }
}

// ============================================================================
// ErrorBurst Tests
// ============================================================================

#[cfg(test)]
mod error_burst_tests {
#[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_new_creates_error_burst() {
        // Arrange & Act
        let burst = ErrorBurst::new(100, 110, 5);

        // Assert
        assert_eq!(burst.start_idx, 100);
        assert_eq!(burst.end_idx, 110);
        assert_eq!(burst.error_count, 5);
        // severity = error_count / (end_idx - start_idx + 1) = 5 / (110 - 100 + 1) = 5/11 ≈ 0.4545
        assert!((burst.severity - 0.4545).abs() < 0.001);
        assert!(burst.error_types.is_empty());
    }

    #[test]
    fn test_with_error_types() {
        // Arrange
        let burst = ErrorBurst::new(100, 110, 5);

        // Act
        let burst = burst.with_error_types(vec![
            "ParseError".to_string(),
            "DecodeError".to_string(),
        ]);

        // Assert
        assert_eq!(burst.error_types.len(), 2);
        assert_eq!(burst.error_types[0], "ParseError");
    }

    #[test]
    fn test_length_calculation() {
        // Arrange
        let burst = ErrorBurst::new(100, 110, 5);

        // Act
        let length = burst.length();

        // Assert
        assert_eq!(length, 11); // 110 - 100 + 1
    }

    #[test]
    fn test_length_single_frame() {
        // Arrange
        let burst = ErrorBurst::new(100, 100, 1);

        // Act
        let length = burst.length();

        // Assert
        assert_eq!(length, 1);
    }

    #[test]
    fn test_density_calculation() {
        // Arrange
        let burst = ErrorBurst::new(100, 110, 5);

        // Act
        let density = burst.density();

        // Assert
        assert!((density - 0.4545).abs() < 0.01); // 5 / 11 ≈ 0.4545
    }

    #[test]
    fn test_density_zero_errors() {
        // Arrange
        let burst = ErrorBurst::new(100, 110, 0);

        // Act
        let density = burst.density();

        // Assert
        assert_eq!(density, 0.0);
    }

    #[test]
    fn test_severity_calculation() {
        // Arrange
        let burst1 = ErrorBurst::new(0, 10, 10); // 10/11 = 0.91
        let burst2 = ErrorBurst::new(0, 100, 10); // 10/101 = 0.099

        // Assert
        assert!(burst1.severity > burst2.severity);
    }
}

// ============================================================================
// ErrorBurstDetection Tests
// ============================================================================

#[cfg(test)]
mod error_burst_detection_tests {
#[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_detect_bursts_empty() {
        // Arrange
        let errors = vec![];

        // Act
        let bursts = ErrorBurstDetection::detect_bursts(&errors, 5);

        // Assert
        assert!(bursts.is_empty());
    }

    #[test]
    fn test_detect_bursts_single_error() {
        // Arrange
        let errors = vec![100];

        // Act
        let bursts = ErrorBurstDetection::detect_bursts(&errors, 5);

        // Assert
        assert_eq!(bursts.len(), 1);
        assert_eq!(bursts[0].start_idx, 100);
        assert_eq!(bursts[0].end_idx, 100);
        assert_eq!(bursts[0].error_count, 1);
    }

    #[test]
    fn test_detect_bursts_consecutive_errors() {
        // Arrange
        let errors = vec![100, 101, 102, 103, 104];

        // Act
        let bursts = ErrorBurstDetection::detect_bursts(&errors, 2);

        // Assert
        assert_eq!(bursts.len(), 1);
        assert_eq!(bursts[0].start_idx, 100);
        assert_eq!(bursts[0].end_idx, 104);
        assert_eq!(bursts[0].error_count, 5);
    }

    #[test]
    fn test_detect_bursts_within_threshold() {
        // Arrange
        let errors = vec![100, 102, 104, 106]; // Gap of 2, threshold is 3

        // Act
        let bursts = ErrorBurstDetection::detect_bursts(&errors, 3);

        // Assert
        assert_eq!(bursts.len(), 1); // All within threshold
        assert_eq!(bursts[0].start_idx, 100);
        assert_eq!(bursts[0].end_idx, 106);
    }

    #[test]
    fn test_detect_bursts_multiple_bursts() {
        // Arrange
        let errors = vec![10, 11, 12, 100, 101, 200];

        // Act
        let bursts = ErrorBurstDetection::detect_bursts(&errors, 5);

        // Assert
        assert_eq!(bursts.len(), 3);
        assert_eq!(bursts[0].start_idx, 10);
        assert_eq!(bursts[1].start_idx, 100);
        assert_eq!(bursts[2].start_idx, 200);
    }

    #[test]
    fn test_detect_bursts_large_gap() {
        // Arrange
        let errors = vec![10, 1000]; // Large gap

        // Act
        let bursts = ErrorBurstDetection::detect_bursts(&errors, 5);

        // Assert
        assert_eq!(bursts.len(), 2);
    }

    #[test]
    fn test_top_bursts_empty() {
        // Arrange
        let bursts = vec![];

        // Act
        let top = ErrorBurstDetection::top_bursts(&bursts, 3);

        // Assert
        assert!(top.is_empty());
    }

    #[test]
    fn test_top_bursts_less_than_n() {
        // Arrange
        let bursts = vec![
            ErrorBurst::new(0, 10, 5),
            ErrorBurst::new(20, 30, 3),
        ];

        // Act
        let top = ErrorBurstDetection::top_bursts(&bursts, 5);

        // Assert
        assert_eq!(top.len(), 2); // Only 2 bursts exist
    }

    #[test]
    fn test_top_bursts_sorts_by_severity() {
        // Arrange
        let bursts = vec![
            ErrorBurst::new(0, 10, 2),  // Low severity
            ErrorBurst::new(20, 30, 10), // High severity
            ErrorBurst::new(40, 50, 5),  // Medium severity
        ];

        // Act
        let top = ErrorBurstDetection::top_bursts(&bursts, 2);

        // Assert
        assert_eq!(top.len(), 2);
        // Should return indices of most severe bursts
        assert!(top.contains(&1)); // High severity burst at index 1
    }

    #[test]
    fn test_top_bursts_single() {
        // Arrange
        let bursts = vec![
            ErrorBurst::new(0, 10, 5),
            ErrorBurst::new(20, 30, 3),
            ErrorBurst::new(40, 50, 7),
        ];

        // Act
        let top = ErrorBurstDetection::top_bursts(&bursts, 1);

        // Assert
        assert_eq!(top.len(), 1);
        assert_eq!(top[0], 2); // Index of most severe burst
    }
}

// ============================================================================
// DiagnosticsBands Construction Tests
// ============================================================================

#[cfg(test)]
mod diagnostics_bands_construction_tests {
#[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_new_creates_diagnostics_bands() {
        // Arrange & Act
        let bands = create_test_diagnostics_bands();

        // Assert
        assert!(bands.scene_changes.is_empty());
        assert!(bands.reorder_entries.is_empty());
        assert!(bands.error_bursts.is_empty());
        assert!(bands.selected_burst.is_none());
        assert!(bands.scene_change_visible);
        assert!(bands.reorder_visible);
        assert!(bands.error_burst_visible);
    }

    #[test]
    fn test_default_creates_diagnostics_bands() {
        // Arrange & Act
        let bands = DiagnosticsBands::default();

        // Assert
        assert!(bands.scene_changes.is_empty());
    }
}

// ============================================================================
// DiagnosticsBands Add Tests
// ============================================================================

#[cfg(test)]
mod diagnostics_bands_add_tests {
#[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_add_scene_change() {
        // Arrange
        let mut bands = create_test_diagnostics_bands();
        let scene = create_test_scene_change(100, 0.85);

        // Act
        bands.add_scene_change(scene);

        // Assert
        assert_eq!(bands.scene_changes.len(), 1);
    }

    #[test]
    fn test_add_multiple_scene_changes() {
        // Arrange
        let mut bands = create_test_diagnostics_bands();

        // Act
        bands.add_scene_change(create_test_scene_change(100, 0.85));
        bands.add_scene_change(create_test_scene_change(200, 0.90));
        bands.add_scene_change(create_test_scene_change(300, 0.75));

        // Assert
        assert_eq!(bands.scene_changes.len(), 3);
    }

    #[test]
    fn test_add_reorder_entry() {
        // Arrange
        let mut bands = create_test_diagnostics_bands();
        let entry = create_test_reorder_entry(10, 1000, 900);

        // Act
        bands.add_reorder_entry(entry);

        // Assert
        assert_eq!(bands.reorder_entries.len(), 1);
    }

    #[test]
    fn test_add_multiple_reorder_entries() {
        // Arrange
        let mut bands = create_test_diagnostics_bands();

        // Act
        bands.add_reorder_entry(create_test_reorder_entry(10, 1000, 900));
        bands.add_reorder_entry(create_test_reorder_entry(20, 2000, 1900));
        bands.add_reorder_entry(create_test_reorder_entry(30, 3000, 2900));

        // Assert
        assert_eq!(bands.reorder_entries.len(), 3);
    }
}

// ============================================================================
// DiagnosticsBands Error Burst Detection Tests
// ============================================================================

#[cfg(test)]
mod diagnostics_bands_error_tests {
#[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_detect_error_bursts() {
        // Arrange
        let mut bands = create_test_diagnostics_bands();
        let errors = vec![100, 101, 102, 200, 201];

        // Act
        bands.detect_error_bursts(&errors, 5);

        // Assert
        assert_eq!(bands.error_bursts.len(), 2);
    }

    #[test]
    fn test_detect_error_bursts_empty() {
        // Arrange
        let mut bands = create_test_diagnostics_bands();
        let errors = vec![];

        // Act
        bands.detect_error_bursts(&errors, 5);

        // Assert
        assert!(bands.error_bursts.is_empty());
    }

    #[test]
    fn test_detect_error_bursts_replaces_existing() {
        // Arrange
        let mut bands = create_test_diagnostics_bands();
        bands.detect_error_bursts(&[100, 101], 5);

        // Act
        bands.detect_error_bursts(&[200, 201], 5);

        // Assert
        assert_eq!(bands.error_bursts.len(), 1); // Replaced
        assert_eq!(bands.error_bursts[0].start_idx, 200);
    }
}

// ============================================================================
// DiagnosticsBands Selection Tests
// ============================================================================

#[cfg(test)]
mod diagnostics_bands_selection_tests {
#[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_auto_select_worst_burst() {
        // Arrange
        let mut bands = create_test_diagnostics_bands();
        bands.detect_error_bursts(&[0, 1, 2, 100, 101], 5);

        // Act
        bands.auto_select_worst_burst();

        // Assert
        assert!(bands.selected_burst.is_some());
    }

    #[test]
    fn test_auto_select_worst_burst_empty() {
        // Arrange
        let mut bands = create_test_diagnostics_bands();

        // Act
        bands.auto_select_worst_burst();

        // Assert
        assert!(bands.selected_burst.is_none());
    }

    #[test]
    fn test_select_burst_valid() {
        // Arrange
        let mut bands = create_test_diagnostics_bands();
        bands.detect_error_bursts(&[100, 101, 200], 5);

        // Act
        bands.select_burst(1);

        // Assert
        assert_eq!(bands.selected_burst, Some(1));
    }

    #[test]
    fn test_select_burst_invalid() {
        // Arrange
        let mut bands = create_test_diagnostics_bands();
        bands.detect_error_bursts(&[100], 5);

        // Act
        bands.select_burst(999); // Out of range

        // Assert
        assert!(bands.selected_burst.is_none()); // Should not change
    }

    #[test]
    fn test_clear_burst_selection() {
        // Arrange
        let mut bands = create_test_diagnostics_bands();
        bands.detect_error_bursts(&[100], 5);
        bands.auto_select_worst_burst();

        // Act
        bands.clear_burst_selection();

        // Assert
        assert!(bands.selected_burst.is_none());
    }

    #[test]
    fn test_get_selected_burst() {
        // Arrange
        let mut bands = create_test_diagnostics_bands();
        bands.detect_error_bursts(&[100, 101], 5);
        bands.select_burst(0);

        // Act
        let selected = bands.get_selected_burst();

        // Assert
        assert!(selected.is_some());
        assert_eq!(selected.unwrap().start_idx, 100);
    }

    #[test]
    fn test_get_selected_burst_none() {
        // Arrange
        let bands = create_test_diagnostics_bands();

        // Act
        let selected = bands.get_selected_burst();

        // Assert
        assert!(selected.is_none());
    }
}

// ============================================================================
// DiagnosticsBands Visibility Tests
// ============================================================================

#[cfg(test)]
mod diagnostics_bands_visibility_tests {
#[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_toggle_scene_change() {
        // Arrange
        let mut bands = create_test_diagnostics_bands();
        let initial = bands.scene_change_visible;

        // Act
        bands.toggle_band(DiagnosticBandType::SceneChange);

        // Assert
        assert_eq!(bands.scene_change_visible, !initial);
    }

    #[test]
    fn test_toggle_reorder_mismatch() {
        // Arrange
        let mut bands = create_test_diagnostics_bands();
        let initial = bands.reorder_visible;

        // Act
        bands.toggle_band(DiagnosticBandType::ReorderMismatch);

        // Assert
        assert_eq!(bands.reorder_visible, !initial);
    }

    #[test]
    fn test_toggle_error_burst() {
        // Arrange
        let mut bands = create_test_diagnostics_bands();
        let initial = bands.error_burst_visible;

        // Act
        bands.toggle_band(DiagnosticBandType::ErrorBurst);

        // Assert
        assert_eq!(bands.error_burst_visible, !initial);
    }

    #[test]
    fn test_multiple_toggles() {
        // Arrange
        let mut bands = create_test_diagnostics_bands();

        // Act
        bands.toggle_band(DiagnosticBandType::SceneChange);
        bands.toggle_band(DiagnosticBandType::SceneChange);

        // Assert
        assert!(bands.scene_change_visible); // Back to original
    }
}

// ============================================================================
// DiagnosticsBands Query Tests
// ============================================================================

#[cfg(test)]
mod diagnostics_bands_query_tests {
#[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_reorder_count() {
        // Arrange
        let mut bands = create_test_diagnostics_bands();
        bands.add_reorder_entry(create_test_reorder_entry(10, 1000, 900));
        bands.add_reorder_entry(create_test_reorder_entry(20, 2000, 1900));
        bands.add_reorder_entry(create_test_reorder_entry(30, 3000, 2900));

        // Act
        let count = bands.reorder_count();

        // Assert
        assert_eq!(count, 3);
    }

    #[test]
    fn test_reorder_count_empty() {
        // Arrange
        let bands = create_test_diagnostics_bands();

        // Act
        let count = bands.reorder_count();

        // Assert
        assert_eq!(count, 0);
    }

    #[test]
    fn test_max_reorder_depth() {
        // Arrange
        let mut bands = create_test_diagnostics_bands();
        bands.add_reorder_entry(create_test_reorder_entry(10, 1000, 900)); // depth=100
        bands.add_reorder_entry(create_test_reorder_entry(20, 2000, 1700)); // depth=300
        bands.add_reorder_entry(create_test_reorder_entry(30, 3000, 2900)); // depth=100

        // Act
        let max_depth = bands.max_reorder_depth();

        // Assert
        assert_eq!(max_depth, 300);
    }

    #[test]
    fn test_max_reorder_depth_empty() {
        // Arrange
        let bands = create_test_diagnostics_bands();

        // Act
        let max_depth = bands.max_reorder_depth();

        // Assert
        assert_eq!(max_depth, 0);
    }

    #[test]
    fn test_total_error_count() {
        // Arrange
        let mut bands = create_test_diagnostics_bands();
        bands.detect_error_bursts(&[100, 101, 102, 200, 201], 5);

        // Act
        let total = bands.total_error_count();

        // Assert
        assert_eq!(total, 5);
    }

    #[test]
    fn test_total_error_count_empty() {
        // Arrange
        let bands = create_test_diagnostics_bands();

        // Act
        let total = bands.total_error_count();

        // Assert
        assert_eq!(total, 0);
    }
}

// ============================================================================
// Edge Cases
// ============================================================================

#[cfg(test)]
mod edge_case_tests {
#[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_zero_confidence_scene_change() {
        // Arrange
        let mut bands = create_test_diagnostics_bands();

        // Act
        bands.add_scene_change(SceneChange::new(100, 0.0));

        // Assert
        assert_eq!(bands.scene_changes.len(), 1);
        assert_eq!(bands.scene_changes[0].confidence, 0.0);
    }

    #[test]
    fn test_perfect_confidence_scene_change() {
        // Arrange
        let mut bands = create_test_diagnostics_bands();

        // Act
        bands.add_scene_change(SceneChange::new(100, 1.0));

        // Assert
        assert_eq!(bands.scene_changes[0].confidence, 1.0);
    }

    #[test]
    fn test_negative_confidence_clamped() {
        // Arrange & Act
        let scene = SceneChange::new(100, -0.5);

        // Assert - f32 can be negative, may want to clamp
        assert_eq!(scene.confidence, -0.5);
    }

    #[test]
    fn test_confidence_above_one() {
        // Arrange & Act
        let scene = SceneChange::new(100, 1.5);

        // Assert - f32 can be > 1.0, may want to clamp
        assert_eq!(scene.confidence, 1.5);
    }

    #[test]
    fn test_zero_length_burst() {
        // Arrange
        let burst = ErrorBurst::new(100, 100, 0);

        // Assert
        assert_eq!(burst.length(), 1);
        assert_eq!(burst.density(), 0.0);
    }

    #[test]
    fn test_single_frame_burst() {
        // Arrange
        let burst = ErrorBurst::new(100, 100, 5);

        // Assert
        assert_eq!(burst.length(), 1);
        assert_eq!(burst.density(), 5.0);
    }

    #[test]
    fn test_empty_error_types() {
        // Arrange
        let burst = ErrorBurst::new(100, 110, 5);

        // Assert
        assert!(burst.error_types.is_empty());
    }

    #[test]
    fn test_duplicate_scene_changes() {
        // Arrange
        let mut bands = create_test_diagnostics_bands();

        // Act
        bands.add_scene_change(SceneChange::new(100, 0.8));
        bands.add_scene_change(SceneChange::new(100, 0.9));

        // Assert
        assert_eq!(bands.scene_changes.len(), 2); // Allows duplicates
    }

    #[test]
    fn test_zero_gap_threshold() {
        // Arrange
        let errors = vec![100, 101, 103, 104];

        // Act
        let bursts = ErrorBurstDetection::detect_bursts(&errors, 0);

        // Assert - With gap_threshold=0, only gaps <= 0 are grouped
        // Since consecutive indices have gaps of at least 1, none are grouped
        // Each error becomes its own burst: [100], [101], [103], [104]
        assert_eq!(bursts.len(), 4);
    }

    #[test]
    fn test_very_large_gap_threshold() {
        // Arrange
        let errors = vec![100, 1000, 10000];

        // Act
        let bursts = ErrorBurstDetection::detect_bursts(&errors, 100000);

        // Assert
        assert_eq!(bursts.len(), 1); // All in single burst
    }

    #[test]
    fn test_unsorted_error_indices() {
        // Arrange - Errors not in order
        let mut errors = vec![200, 100, 300, 150];

        // Act - Implementation assumes sorted input, so sort first
        errors.sort();
        let bursts = ErrorBurstDetection::detect_bursts(&errors, 10);

        // Assert - With sorted input [100, 150, 200, 300] and gap_threshold=10,
        // each gap > 10, so each is its own burst
        assert_eq!(bursts.len(), 4);
    }
}
