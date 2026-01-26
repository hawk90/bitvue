// Compare module tests
//
// Per generate-tests skill: Arrange-Act-Assert pattern with fixtures
// and edge case coverage.

use super::*;
use crate::{FrameIndexMap, SyncMode};
use crate::frame_identity::FrameMetadata;

// ============================================================================
// Fixtures
// ============================================================================

/// Create test FrameIndexMap for stream A
fn create_test_frame_map_a() -> FrameIndexMap {
    let mut frames = Vec::new();
    for i in 0..100 {
        frames.push(FrameMetadata {
            pts: Some(1000 + i as u64 * 33), // ~30fps
            dts: Some(i as u64 * 33),
        });
    }
    FrameIndexMap::new(&frames)
}

/// Create test FrameIndexMap for stream B
fn create_test_frame_map_b() -> FrameIndexMap {
    let mut frames = Vec::new();
    for i in 0..100 {
        frames.push(FrameMetadata {
            pts: Some(1000 + i as u64 * 33),
            dts: Some(i as u64 * 33),
        });
    }
    FrameIndexMap::new(&frames)
}

/// Create test FrameIndexMap with offset
fn create_test_frame_map_b_offset(offset_ticks: u64) -> FrameIndexMap {
    let mut frames = Vec::new();
    for i in 0..100 {
        frames.push(FrameMetadata {
            pts: Some(1000 + i as u64 * 33 + offset_ticks),
            dts: Some(i as u64 * 33),
        });
    }
    FrameIndexMap::new(&frames)
}

/// Create test FrameIndexMap with gaps
fn create_test_frame_map_with_gaps() -> FrameIndexMap {
    let mut frames = Vec::new();
    // Frames 0-9
    for i in 0..10 {
        frames.push(FrameMetadata {
            pts: Some(i as u64 * 33),
            dts: Some(i as u64 * 33),
        });
    }
    // Frames 20-29 (gap at 10-19)
    for i in 20..30 {
        frames.push(FrameMetadata {
            pts: Some(i as u64 * 33),
            dts: Some(i as u64 * 33),
        });
    }
    FrameIndexMap::new(&frames)
}

/// Create shorter FrameIndexMap (50 frames)
fn create_test_frame_map_b_short() -> FrameIndexMap {
    let mut frames = Vec::new();
    for i in 0..50 {
        frames.push(FrameMetadata {
            pts: Some(1000 + i as u64 * 33),
            dts: Some(i as u64 * 33),
        });
    }
    FrameIndexMap::new(&frames)
}

// ============================================================================
// ResolutionInfo Tests
// ============================================================================

#[cfg(test)]
mod resolution_info_tests {
    use super::*;

    #[test]
    fn test_resolution_info_new() {
        // Arrange & Act
        let info = ResolutionInfo::new((1920, 1080), (1920, 1080));

        // Assert
        assert_eq!(info.stream_a, (1920, 1080));
        assert_eq!(info.stream_b, (1920, 1080));
        assert_eq!(info.tolerance, 0.05);
    }

    #[test]
    fn test_resolution_info_exact_match() {
        // Arrange
        let info = ResolutionInfo::new((1920, 1080), (1920, 1080));

        // Act & Assert
        assert!(info.is_exact_match());
        assert!(info.is_compatible());
        assert_eq!(info.mismatch_percentage(), 0.0);
        assert_eq!(info.scale_indicator(), "1:1");
    }

    #[test]
    fn test_resolution_info_within_tolerance() {
        // Arrange - 1920x1080 vs 1920x1079 (1 pixel diff, < 5%)
        let info = ResolutionInfo::new((1920, 1080), (1920, 1079));

        // Act
        let mismatch = info.mismatch_percentage();

        // Assert
        assert!(mismatch < 0.05); // Less than 5%
        assert!(info.is_compatible());
        assert!(!info.is_exact_match());
    }

    #[test]
    fn test_resolution_info_outside_tolerance() {
        // Arrange - 1920x1080 vs 1280x720 (much larger difference)
        let info = ResolutionInfo::new((1920, 1080), (1280, 720));

        // Act & Assert
        assert!(!info.is_compatible());
        assert!(info.mismatch_percentage() > 0.05);
    }

    #[test]
    fn test_resolution_info_zero_resolution() {
        // Arrange
        let info = ResolutionInfo::new((0, 0), (0, 0));

        // Act & Assert
        assert_eq!(info.mismatch_percentage(), 0.0);
        assert!(info.is_compatible());
    }

    #[test]
    fn test_resolution_info_scale_indicator() {
        // Arrange
        let info = ResolutionInfo::new((1920, 1080), (1280, 720));

        // Act
        let indicator = info.scale_indicator();

        // Assert
        assert!(indicator.contains("1920x1080"));
        assert!(indicator.contains("1280x720"));
        assert!(indicator.contains("vs"));
    }
}

// ============================================================================
// AlignmentQuality Tests
// ============================================================================

#[cfg(test)]
mod alignment_quality_tests {
    use super::*;

    #[test]
    fn test_alignment_quality_exact() {
        // Arrange & Act
        let quality = AlignmentQuality::Exact;

        // Assert
        assert_eq!(quality.display_text(), "Exact");
        assert_eq!(quality.color_hint(), AlignmentQualityColor::Green);
    }

    #[test]
    fn test_alignment_quality_nearest() {
        // Arrange & Act
        let quality = AlignmentQuality::Nearest;

        // Assert
        assert_eq!(quality.display_text(), "Nearest");
        assert_eq!(quality.color_hint(), AlignmentQualityColor::Yellow);
    }

    #[test]
    fn test_alignment_quality_gap() {
        // Arrange & Act
        let quality = AlignmentQuality::Gap;

        // Assert
        assert_eq!(quality.display_text(), "Gap");
        assert_eq!(quality.color_hint(), AlignmentQualityColor::Red);
    }
}

// ============================================================================
// SyncControls Tests
// ============================================================================

#[cfg(test)]
mod sync_controls_tests {
    use super::*;

    #[test]
    fn test_sync_controls_new() {
        // Arrange & Act
        let controls = SyncControls::new();

        // Assert
        assert_eq!(controls.mode, SyncMode::Off);
        assert!(!controls.manual_offset_enabled);
        assert_eq!(controls.manual_offset, 0);
        assert!(!controls.show_alignment_info);
    }

    #[test]
    fn test_sync_controls_default() {
        // Arrange & Act
        let controls = SyncControls::default();

        // Assert
        assert_eq!(controls.mode, SyncMode::Off);
    }

    #[test]
    fn test_sync_controls_toggle_sync() {
        // Arrange
        let mut controls = SyncControls::new();

        // Act - Off to Playhead
        controls.toggle_sync();

        // Assert
        assert_eq!(controls.mode, SyncMode::Playhead);

        // Act - Playhead to Full
        controls.toggle_sync();

        // Assert
        assert_eq!(controls.mode, SyncMode::Full);

        // Act - Full to Off
        controls.toggle_sync();

        // Assert
        assert_eq!(controls.mode, SyncMode::Off);
    }

    #[test]
    fn test_sync_controls_set_mode() {
        // Arrange
        let mut controls = SyncControls::new();

        // Act
        controls.set_mode(SyncMode::Full);

        // Assert
        assert_eq!(controls.mode, SyncMode::Full);
    }

    #[test]
    fn test_sync_controls_manual_offset() {
        // Arrange
        let mut controls = SyncControls::new();

        // Act - Enable manual offset
        controls.enable_manual_offset();
        assert!(controls.manual_offset_enabled);

        // Act - Set offset
        controls.set_offset(5);
        assert_eq!(controls.manual_offset, 5);

        // Act - Adjust offset
        controls.adjust_offset(2);
        assert_eq!(controls.manual_offset, 7);

        // Act - Disable
        controls.disable_manual_offset();

        // Assert
        assert!(!controls.manual_offset_enabled);
        assert_eq!(controls.manual_offset, 0);
    }

    #[test]
    fn test_sync_controls_reset_offset() {
        // Arrange
        let mut controls = SyncControls::new();
        controls.set_offset(10);

        // Act
        controls.reset_offset();

        // Assert
        assert_eq!(controls.manual_offset, 0);
    }

    #[test]
    fn test_sync_controls_toggle_alignment_info() {
        // Arrange
        let mut controls = SyncControls::new();
        assert!(!controls.show_alignment_info);

        // Act
        controls.toggle_alignment_info();

        // Assert
        assert!(controls.show_alignment_info);

        // Act - Toggle again
        controls.toggle_alignment_info();

        // Assert
        assert!(!controls.show_alignment_info);
    }

    #[test]
    fn test_sync_controls_offset_saturation() {
        // Arrange
        let mut controls = SyncControls::new();
        controls.manual_offset = i32::MAX;

        // Act - Should saturate
        controls.adjust_offset(1);

        // Assert
        assert_eq!(controls.manual_offset, i32::MAX);
    }
}

// ============================================================================
// CompareWorkspace Tests
// ============================================================================

#[cfg(test)]
mod compare_workspace_tests {
    use super::*;

    #[test]
    fn test_compare_workspace_new() {
        // Arrange
        let stream_a = create_test_frame_map_a();
        let stream_b = create_test_frame_map_b();

        // Act
        let workspace = CompareWorkspace::new(stream_a, stream_b, (1920, 1080), (1920, 1080));

        // Assert
        assert_eq!(workspace.manual_offset, 0);
        assert_eq!(workspace.sync_mode, SyncMode::Off);
        assert!(workspace.resolution_info.is_compatible());
    }

    #[test]
    fn test_compare_workspace_resolution_mismatch() {
        // Arrange
        let stream_a = create_test_frame_map_a();
        let stream_b = create_test_frame_map_b();

        // Act
        let workspace = CompareWorkspace::new(stream_a, stream_b, (1920, 1080), (1280, 720));

        // Assert
        assert!(!workspace.resolution_info.is_compatible());
        assert!(!workspace.is_diff_enabled());
        assert!(workspace.disable_reason.is_some());
        assert!(workspace.disable_reason.unwrap().contains("Resolution mismatch"));
    }

    #[test]
    fn test_compare_workspace_set_sync_mode() {
        // Arrange
        let stream_a = create_test_frame_map_a();
        let stream_b = create_test_frame_map_b();
        let mut workspace = CompareWorkspace::new(stream_a.clone(), stream_b.clone(), (1920, 1080), (1920, 1080));

        // Act
        workspace.set_sync_mode(SyncMode::Full);

        // Assert
        assert_eq!(workspace.sync_mode(), SyncMode::Full);
    }

    #[test]
    fn test_compare_workspace_manual_offset() {
        // Arrange
        let stream_a = create_test_frame_map_a();
        let stream_b = create_test_frame_map_b();
        let mut workspace = CompareWorkspace::new(stream_a.clone(), stream_b.clone(), (1920, 1080), (1920, 1080));

        // Act
        workspace.set_manual_offset(5);

        // Assert
        assert_eq!(workspace.manual_offset(), 5);

        // Act - Adjust
        workspace.adjust_offset(2);

        // Assert
        assert_eq!(workspace.manual_offset(), 7);

        // Act - Reset
        workspace.reset_offset();

        // Assert
        assert_eq!(workspace.manual_offset(), 0);
    }

    #[test]
    fn test_compare_workspace_total_frames() {
        // Arrange
        let stream_a = create_test_frame_map_a(); // 100 frames
        let stream_b_short = create_test_frame_map_b_short(); // 50 frames

        // Act
        let workspace = CompareWorkspace::new(stream_a, stream_b_short, (1920, 1080), (1920, 1080));

        // Assert - Should be max of both
        assert_eq!(workspace.total_frames(), 100);
    }

    #[test]
    fn test_compare_workspace_get_aligned_frame() {
        // Arrange
        let stream_a = create_test_frame_map_a();
        let stream_b = create_test_frame_map_b();
        let workspace = CompareWorkspace::new(stream_a, stream_b, (1920, 1080), (1920, 1080));

        // Act
        let result = workspace.get_aligned_frame(0);

        // Assert - Should find matching frame
        assert!(result.is_some());
        let (b_idx, quality) = result.unwrap();
        assert_eq!(b_idx, 0);
        assert_eq!(quality, AlignmentQuality::Exact);
    }

    #[test]
    fn test_compare_workspace_get_aligned_frame_with_offset() {
        // Arrange
        let stream_a = create_test_frame_map_a();
        let stream_b = create_test_frame_map_b();
        let mut workspace = CompareWorkspace::new(stream_a, stream_b, (1920, 1080), (1920, 1080));
        workspace.set_manual_offset(5);

        // Act
        let result = workspace.get_aligned_frame(0);

        // Assert - Should align to frame 5 in B
        assert!(result.is_some());
        let (b_idx, _) = result.unwrap();
        assert_eq!(b_idx, 5);
    }

    #[test]
    fn test_compare_workspace_get_aligned_frame_not_found() {
        // Arrange
        let stream_a = create_test_frame_map_a(); // 100 frames
        let stream_b = create_test_frame_map_b();
        let workspace = CompareWorkspace::new(stream_a, stream_b, (1920, 1080), (1920, 1080));

        // Act - Frame beyond range
        let result = workspace.get_aligned_frame(200);

        // Assert
        assert!(result.is_none());
    }

    #[test]
    fn test_compare_workspace_resolution_info() {
        // Arrange
        let stream_a = create_test_frame_map_a();
        let stream_b = create_test_frame_map_b();
        let workspace = CompareWorkspace::new(stream_a, stream_b, (1920, 1080), (1920, 1080));

        // Act
        let info = workspace.resolution_info();

        // Assert
        assert_eq!(info.stream_a, (1920, 1080));
        assert_eq!(info.stream_b, (1920, 1080));
    }

    #[test]
    fn test_compare_workspace_diff_enabled() {
        // Arrange
        let stream_a = create_test_frame_map_a();
        let stream_b = create_test_frame_map_b();

        // Act - Same resolution
        let workspace = CompareWorkspace::new(stream_a, stream_b, (1920, 1080), (1920, 1080));

        // Assert
        assert!(workspace.is_diff_enabled());
        assert!(workspace.disable_reason().is_none());
    }
}

// ============================================================================
// Edge Cases
// ============================================================================

#[cfg(test)]
mod edge_case_tests {
    use super::*;

    #[test]
    fn test_resolution_info_both_zero() {
        // Arrange & Act
        let info = ResolutionInfo::new((0, 0), (0, 0));

        // Assert
        assert_eq!(info.mismatch_percentage(), 0.0);
        assert!(info.is_compatible());
    }

    #[test]
    fn test_resolution_info_one_zero() {
        // Arrange & Act
        let info = ResolutionInfo::new((1920, 1080), (0, 0));

        // Assert
        assert!(info.mismatch_percentage() > 0.99); // Almost 100% mismatch
        assert!(!info.is_compatible());
    }

    #[test]
    fn test_sync_controls_negative_offset() {
        // Arrange
        let mut controls = SyncControls::new();

        // Act
        controls.set_offset(-10);

        // Assert
        assert_eq!(controls.manual_offset, -10);

        // Act - Adjust negative
        controls.adjust_offset(-5);

        // Assert
        assert_eq!(controls.manual_offset, -15);
    }

    #[test]
    fn test_compare_workspace_negative_offset() {
        // Arrange
        let stream_a = create_test_frame_map_a();
        let stream_b = create_test_frame_map_b();
        let mut workspace = CompareWorkspace::new(stream_a, stream_b, (1920, 1080), (1920, 1080));

        // Act
        workspace.set_manual_offset(-5);

        // Assert
        assert_eq!(workspace.manual_offset(), -5);

        // Try to get aligned frame with negative offset
        // Frame 0 - 5 = -5, which should be clamped to 0
        let result = workspace.get_aligned_frame(0);

        // Assert - Should still find frame 0
        assert!(result.is_some());
    }

    #[test]
    fn test_compare_workspace_different_framerates() {
        // Arrange - Different timing between streams with large offset
        let stream_a = create_test_frame_map_a();
        let stream_b = create_test_frame_map_b_offset(3000); // ~3 second offset
        let workspace = CompareWorkspace::new(stream_a, stream_b, (1920, 1080), (1920, 1080));

        // Act - Try to get aligned frame for stream A frame 0 (PTS=1000)
        // Stream B starts at PTS=4000, so no close match exists
        let result = workspace.get_aligned_frame(0);

        // Assert - With such large offset, no alignment is found
        // This is correct behavior: 3000 ticks (~3 seconds) is too large for matching
        assert!(result.is_none());
    }
}

