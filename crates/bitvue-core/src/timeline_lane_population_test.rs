// Timeline lane population module tests
//
// Per generate-tests skill: Arrange-Act-Assert pattern with fixtures
// and edge case coverage.

use super::*;
use crate::DiagnosticsManager;
use crate::DiagnosticSeverity;
use crate::diagnostics::Diagnostic;
use crate::diagnostics::DiagnosticCategory;
use crate::timeline::TimelineFrame;
use crate::FrameKey;
use crate::FrameMarker;
use std::collections::HashMap;

// ============================================================================
// Fixtures
// ============================================================================

/// Create test timeline frames
fn create_test_frames(count: usize) -> Vec<TimelineFrame> {
    (0..count)
        .map(|i| TimelineFrame {
            display_idx: i,
            size_bytes: 1000 + i as u64 * 100,
            frame_type: if i % 3 == 0 { "I".to_string() } else { "P".to_string() },
            marker: if i % 3 == 0 { FrameMarker::Key } else { FrameMarker::None },
            pts: Some(i as u64 * 100),
            dts: Some(i as u64 * 100 + if i % 2 == 0 { 0 } else { 50 }), // Some B-frame reordering
            is_selected: false,
        })
        .collect()
}

/// Create test QP stats
fn create_test_qp_stats() -> Vec<FrameQpStats> {
    vec![
        FrameQpStats::new(0, 20.0),
        FrameQpStats::new(1, 22.0),
        FrameQpStats::new(2, 25.0),
        FrameQpStats::new(3, 18.0),
        FrameQpStats::new(4, 30.0),
    ]
}

/// Create test slice stats
fn create_test_slice_stats() -> Vec<FrameSliceStats> {
    vec![
        FrameSliceStats::new(0, 1),
        FrameSliceStats::new(1, 2),
        FrameSliceStats::new(2, 4),
        FrameSliceStats::new(3, 1),
        FrameSliceStats::new(4, 8),
    ]
}

/// Create test diagnostics manager
fn create_test_diagnostics() -> DiagnosticsManager {
    let mut manager = DiagnosticsManager::new();
    // Add some diagnostics for specific frames
    manager.add(Diagnostic {
        id: 0, // Will be reassigned
        severity: DiagnosticSeverity::Error,
        stream_id: crate::StreamId::A,
        message: "Test error".to_string(),
        category: DiagnosticCategory::Bitstream,
        offset_bytes: 0,
        bit_range: None,
        frame_key: Some(FrameKey {
            stream: crate::StreamId::A,
            frame_index: 0,
            pts: Some(0),
        }),
        unit_key: None,
        codec: None,
        timestamp_ms: 0,
        details: HashMap::new(),
    });
    manager.add(Diagnostic {
        id: 0,
        severity: DiagnosticSeverity::Warn,
        stream_id: crate::StreamId::A,
        message: "Test warning".to_string(),
        category: DiagnosticCategory::Bitstream,
        offset_bytes: 0,
        bit_range: None,
        frame_key: Some(FrameKey {
            stream: crate::StreamId::A,
            frame_index: 0,
            pts: Some(0),
        }),
        unit_key: None,
        codec: None,
        timestamp_ms: 0,
        details: HashMap::new(),
    });
    manager.add(Diagnostic {
        id: 0,
        severity: DiagnosticSeverity::Error,
        stream_id: crate::StreamId::A,
        message: "Another error".to_string(),
        category: DiagnosticCategory::Decode,
        offset_bytes: 0,
        bit_range: None,
        frame_key: Some(FrameKey {
            stream: crate::StreamId::A,
            frame_index: 5,
            pts: Some(500),
        }),
        unit_key: None,
        codec: None,
        timestamp_ms: 0,
        details: HashMap::new(),
    });
    // Add non-actionable diagnostic (should be ignored)
    manager.add(Diagnostic {
        id: 0,
        severity: DiagnosticSeverity::Info,
        stream_id: crate::StreamId::A,
        message: "Test info".to_string(),
        category: DiagnosticCategory::Bitstream,
        offset_bytes: 0,
        bit_range: None,
        frame_key: Some(FrameKey {
            stream: crate::StreamId::A,
            frame_index: 5,
            pts: Some(500),
        }),
        unit_key: None,
        codec: None,
        timestamp_ms: 0,
        details: HashMap::new(),
    });
    manager
}

// ============================================================================
// LaneType Tests
// ============================================================================

#[cfg(test)]
mod lane_type_tests {
    use super::*;

    #[test]
    fn test_lane_type_names() {
        // Arrange & Act
        let qp_name = LaneType::QpAvg.name();
        let bpp_name = LaneType::BitsPerPixel.name();
        let slice_name = LaneType::SliceCount.name();
        let diag_name = LaneType::DiagnosticsDensity.name();
        let reorder_name = LaneType::ReorderMismatch.name();

        // Assert
        assert_eq!(qp_name, "QP Average");
        assert_eq!(bpp_name, "Bits per Pixel");
        assert_eq!(slice_name, "Slice Count");
        assert_eq!(diag_name, "Diagnostics");
        assert_eq!(reorder_name, "Reorder Mismatch");
    }

    #[test]
    fn test_lane_type_color_hints() {
        // Arrange & Act
        let qp_color = LaneType::QpAvg.color_hint();
        let bpp_color = LaneType::BitsPerPixel.color_hint();
        let slice_color = LaneType::SliceCount.color_hint();
        let diag_color = LaneType::DiagnosticsDensity.color_hint();
        let reorder_color = LaneType::ReorderMismatch.color_hint();

        // Assert
        assert_eq!(qp_color, "cyan");
        assert_eq!(bpp_color, "magenta");
        assert_eq!(slice_color, "yellow");
        assert_eq!(diag_color, "orange");
        assert_eq!(reorder_color, "red");
    }

    #[test]
    fn test_lane_type_uses_secondary_axis() {
        // Arrange & Act
        let qp_secondary = LaneType::QpAvg.uses_secondary_axis();
        let bpp_secondary = LaneType::BitsPerPixel.uses_secondary_axis();
        let slice_secondary = LaneType::SliceCount.uses_secondary_axis();
        let diag_secondary = LaneType::DiagnosticsDensity.uses_secondary_axis();
        let reorder_secondary = LaneType::ReorderMismatch.uses_secondary_axis();

        // Assert - Only QP and BPP use secondary axis
        assert!(qp_secondary);
        assert!(bpp_secondary);
        assert!(!slice_secondary);
        assert!(!diag_secondary);
        assert!(!reorder_secondary);
    }
}

// ============================================================================
// LaneDataPoint Tests
// ============================================================================

#[cfg(test)]
mod lane_data_point_tests {
    use super::*;

    #[test]
    fn test_lane_data_point_new() {
        // Arrange & Act
        let point = LaneDataPoint::new(10, 25.5);

        // Assert
        assert_eq!(point.display_idx, 10);
        assert_eq!(point.value, 25.5);
    }

    #[test]
    fn test_lane_data_point_zero_value() {
        // Arrange & Act
        let point = LaneDataPoint::new(0, 0.0);

        // Assert
        assert_eq!(point.display_idx, 0);
        assert_eq!(point.value, 0.0);
    }

    #[test]
    fn test_lane_data_point_negative_value() {
        // Arrange & Act
        let point = LaneDataPoint::new(5, -10.0);

        // Assert
        assert_eq!(point.value, -10.0);
    }
}

// ============================================================================
// Lane Tests
// ============================================================================

#[cfg(test)]
mod lane_tests {
    use super::*;

    #[test]
    fn test_lane_new() {
        // Arrange & Act
        let lane = Lane::new(LaneType::QpAvg);

        // Assert
        assert_eq!(lane.lane_type, LaneType::QpAvg);
        assert!(lane.data.is_empty());
        assert!(lane.enabled);
        assert_eq!(lane.opacity, 1.0);
    }

    #[test]
    fn test_lane_add_point() {
        // Arrange
        let mut lane = Lane::new(LaneType::QpAvg);

        // Act
        lane.add_point(0, 20.0);
        lane.add_point(1, 22.0);

        // Assert
        assert_eq!(lane.data.len(), 2);
        assert_eq!(lane.data[0].display_idx, 0);
        assert_eq!(lane.data[0].value, 20.0);
    }

    #[test]
    fn test_lane_get_value_exists() {
        // Arrange
        let mut lane = Lane::new(LaneType::QpAvg);
        lane.add_point(5, 25.0);

        // Act
        let value = lane.get_value(5);

        // Assert
        assert_eq!(value, Some(25.0));
    }

    #[test]
    fn test_lane_get_value_not_exists() {
        // Arrange
        let mut lane = Lane::new(LaneType::QpAvg);
        lane.add_point(5, 25.0);

        // Act
        let value = lane.get_value(10);

        // Assert
        assert!(value.is_none());
    }

    #[test]
    fn test_lane_value_range_empty() {
        // Arrange
        let lane = Lane::new(LaneType::QpAvg);

        // Act
        let (min, max) = lane.value_range();

        // Assert
        assert_eq!(min, 0.0);
        assert_eq!(max, 0.0);
    }

    #[test]
    fn test_lane_value_range_single() {
        // Arrange
        let mut lane = Lane::new(LaneType::QpAvg);
        lane.add_point(0, 25.0);

        // Act
        let (min, max) = lane.value_range();

        // Assert
        assert_eq!(min, 25.0);
        assert_eq!(max, 25.0);
    }

    #[test]
    fn test_lane_value_range_multiple() {
        // Arrange
        let mut lane = Lane::new(LaneType::QpAvg);
        lane.add_point(0, 20.0);
        lane.add_point(1, 30.0);
        lane.add_point(2, 25.0);

        // Act
        let (min, max) = lane.value_range();

        // Assert
        assert_eq!(min, 20.0);
        assert_eq!(max, 30.0);
    }

    #[test]
    fn test_lane_value_range_negative() {
        // Arrange
        let mut lane = Lane::new(LaneType::QpAvg);
        lane.add_point(0, -10.0);
        lane.add_point(1, 10.0);

        // Act
        let (min, max) = lane.value_range();

        // Assert
        assert_eq!(min, -10.0);
        assert_eq!(max, 10.0);
    }
}

// ============================================================================
// Replace Lane Tests
// ============================================================================

#[cfg(test)]
mod replace_lane_tests {
    use super::*;

    #[test]
    fn test_replace_lane_adds_new() {
        // Arrange
        let mut lanes = Vec::new();
        let new_lane = Lane::new(LaneType::QpAvg);

        // Act
        replace_lane(&mut lanes, new_lane);

        // Assert
        assert_eq!(lanes.len(), 1);
        assert_eq!(lanes[0].lane_type, LaneType::QpAvg);
    }

    #[test]
    fn test_replace_lane_replaces_existing() {
        // Arrange
        let mut lanes = vec![Lane::new(LaneType::QpAvg)];
        lanes[0].add_point(0, 20.0);

        let mut new_lane = Lane::new(LaneType::QpAvg);
        new_lane.add_point(0, 25.0);

        // Act
        replace_lane(&mut lanes, new_lane);

        // Assert
        assert_eq!(lanes.len(), 1);
        assert_eq!(lanes[0].get_value(0), Some(25.0));
    }

    #[test]
    fn test_replace_lane_keeps_different_types() {
        // Arrange
        let mut lanes = vec![
            Lane::new(LaneType::QpAvg),
            Lane::new(LaneType::SliceCount),
        ];
        let new_lane = Lane::new(LaneType::QpAvg);

        // Act
        replace_lane(&mut lanes, new_lane);

        // Assert
        assert_eq!(lanes.len(), 2);
        assert_eq!(lanes[0].lane_type, LaneType::SliceCount); // Kept
        assert_eq!(lanes[1].lane_type, LaneType::QpAvg); // Replaced
    }
}

// ============================================================================
// Populate BPP Lane Tests
// ============================================================================

#[cfg(test)]
mod populate_bpp_lane_tests {
    use super::*;

    #[test]
    fn test_populate_bpp_lane_basic() {
        // Arrange
        let frames = create_test_frames(3);

        // Act
        let lane = populate_bpp_lane(&frames, 1920, 1080);

        // Assert
        assert_eq!(lane.lane_type, LaneType::BitsPerPixel);
        assert_eq!(lane.data.len(), 3);
        // BPP = (size_bytes * 8) / (width * height)
        // Frame 0: 1000 * 8 / (1920 * 1080) = 8000 / 2073600 ≈ 0.00386
        assert!(lane.data[0].value > 0.0);
    }

    #[test]
    fn test_populate_bpp_lane_zero_dimensions() {
        // Arrange
        let frames = create_test_frames(3);

        // Act
        let lane = populate_bpp_lane(&frames, 0, 1080);

        // Assert
        assert!(lane.data.is_empty());
    }

    #[test]
    fn test_populate_bpp_lane_empty_frames() {
        // Arrange
        let frames = vec![];

        // Act
        let lane = populate_bpp_lane(&frames, 1920, 1080);

        // Assert
        assert!(lane.data.is_empty());
    }

    #[test]
    fn test_populate_bpp_lane_calculates_correctly() {
        // Arrange
        let frames = vec![
            TimelineFrame {
                display_idx: 0,
                size_bytes: 1000,
                frame_type: "I".to_string(),
                marker: FrameMarker::Key,
                pts: None,
                dts: None,
                is_selected: false,
            },
        ];

        // Act
        let lane = populate_bpp_lane(&frames, 100, 100); // 10000 pixels

        // Assert - BPP = (1000 * 8) / 10000 = 0.8
        assert!((lane.data[0].value - 0.8).abs() < 0.001);
    }
}

// ============================================================================
// Populate Diagnostics Lane Tests
// ============================================================================

#[cfg(test)]
mod populate_diagnostics_lane_tests {
    use super::*;

    #[test]
    fn test_populate_diagnostics_lane() {
        // Arrange
        let diagnostics = create_test_diagnostics();

        // Act
        let lane = populate_diagnostics_lane(&diagnostics);

        // Assert
        assert_eq!(lane.lane_type, LaneType::DiagnosticsDensity);
        // Frame 0 has 2 diagnostics (error + warning), frame 5 has 1 (error)
        assert_eq!(lane.data.len(), 2);
        assert_eq!(lane.get_value(0), Some(2.0));
        assert_eq!(lane.get_value(5), Some(1.0));
    }

    #[test]
    fn test_populate_diagnostics_lane_ignores_info() {
        // Arrange
        let diagnostics = create_test_diagnostics();

        // Act
        let lane = populate_diagnostics_lane(&diagnostics);

        // Assert - Info diagnostic should be ignored
        // Frame 0: 2 (error + warning), Frame 5: 1 (error only, info ignored)
        assert_eq!(lane.get_value(5), Some(1.0));
    }

    #[test]
    fn test_populate_diagnostics_lane_empty() {
        // Arrange
        let diagnostics = DiagnosticsManager::new();

        // Act
        let lane = populate_diagnostics_lane(&diagnostics);

        // Assert
        assert!(lane.data.is_empty());
    }

    #[test]
    fn test_populate_diagnostics_lane_no_actionable() {
        // Arrange
        let mut diagnostics = DiagnosticsManager::new();
        diagnostics.add(Diagnostic {
            id: 0,
            severity: DiagnosticSeverity::Info,
            stream_id: crate::StreamId::A,
            message: "Just info".to_string(),
            category: DiagnosticCategory::Bitstream,
            offset_bytes: 0,
            bit_range: None,
            frame_key: Some(FrameKey {
                stream: crate::StreamId::A,
                frame_index: 0,
                pts: Some(0),
            }),
            unit_key: None,
            codec: None,
            timestamp_ms: 0,
            details: HashMap::new(),
        });

        // Act
        let lane = populate_diagnostics_lane(&diagnostics);

        // Assert - Info only, should be empty
        assert!(lane.data.is_empty());
    }
}

// ============================================================================
// Populate Reorder Lane Tests
// ============================================================================

#[cfg(test)]
mod populate_reorder_lane_tests {
    use super::*;

    #[test]
    fn test_populate_reorder_lane() {
        // Arrange - create_test_frames has some frames with PTS != DTS
        let frames = create_test_frames(5);

        // Act
        let lane = populate_reorder_lane(&frames);

        // Assert
        assert_eq!(lane.lane_type, LaneType::ReorderMismatch);
        assert_eq!(lane.data.len(), 5);
        // Frames 1 and 3 have DTS = PTS + 50 (mismatch)
        assert_eq!(lane.get_value(0), Some(0.0)); // PTS = DTS
        assert_eq!(lane.get_value(1), Some(1.0)); // PTS != DTS
        assert_eq!(lane.get_value(2), Some(0.0));
        assert_eq!(lane.get_value(3), Some(1.0));
        assert_eq!(lane.get_value(4), Some(0.0));
    }

    #[test]
    fn test_populate_reorder_lane_no_pts_dts() {
        // Arrange
        let frames = vec![
            TimelineFrame {
                display_idx: 0,
                size_bytes: 1000,
                frame_type: "I".to_string(),
                marker: FrameMarker::Key,
                pts: None,
                dts: None,
                is_selected: false,
            },
        ];

        // Act
        let lane = populate_reorder_lane(&frames);

        // Assert
        assert_eq!(lane.get_value(0), Some(0.0));
    }

    #[test]
    fn test_populate_reorder_lane_only_pts() {
        // Arrange
        let frames = vec![
            TimelineFrame {
                display_idx: 0,
                size_bytes: 1000,
                frame_type: "I".to_string(),
                marker: FrameMarker::Key,
                pts: Some(100),
                dts: None,
                is_selected: false,
            },
        ];

        // Act
        let lane = populate_reorder_lane(&frames);

        // Assert
        assert_eq!(lane.get_value(0), Some(0.0));
    }

    #[test]
    fn test_populate_reorder_lane_empty() {
        // Arrange
        let frames = vec![];

        // Act
        let lane = populate_reorder_lane(&frames);

        // Assert
        assert!(lane.data.is_empty());
    }
}

// ============================================================================
// Populate QP Lane Tests
// ============================================================================

#[cfg(test)]
mod populate_qp_lane_tests {
    use super::*;

    #[test]
    fn test_populate_qp_lane() {
        // Arrange
        let qp_data = create_test_qp_stats();

        // Act
        let lane = populate_qp_lane(&qp_data);

        // Assert
        assert_eq!(lane.lane_type, LaneType::QpAvg);
        assert_eq!(lane.data.len(), 5);
        assert_eq!(lane.get_value(0), Some(20.0));
        assert_eq!(lane.get_value(1), Some(22.0));
        assert_eq!(lane.get_value(2), Some(25.0));
    }

    #[test]
    fn test_populate_qp_lane_empty() {
        // Arrange
        let qp_data = vec![];

        // Act
        let lane = populate_qp_lane(&qp_data);

        // Assert
        assert!(lane.data.is_empty());
    }

    #[test]
    fn test_populate_qp_lane_single() {
        // Arrange
        let qp_data = vec![FrameQpStats::new(0, 30.0)];

        // Act
        let lane = populate_qp_lane(&qp_data);

        // Assert
        assert_eq!(lane.data.len(), 1);
        assert_eq!(lane.get_value(0), Some(30.0));
    }
}

// ============================================================================
// Populate Slice Lane Tests
// ============================================================================

#[cfg(test)]
mod populate_slice_lane_tests {
    use super::*;

    #[test]
    fn test_populate_slice_lane() {
        // Arrange
        let slice_data = create_test_slice_stats();

        // Act
        let lane = populate_slice_lane(&slice_data);

        // Assert
        assert_eq!(lane.lane_type, LaneType::SliceCount);
        assert_eq!(lane.data.len(), 5);
        assert_eq!(lane.get_value(0), Some(1.0));
        assert_eq!(lane.get_value(1), Some(2.0));
        assert_eq!(lane.get_value(2), Some(4.0));
    }

    #[test]
    fn test_populate_slice_lane_empty() {
        // Arrange
        let slice_data = vec![];

        // Act
        let lane = populate_slice_lane(&slice_data);

        // Assert
        assert!(lane.data.is_empty());
    }

    #[test]
    fn test_populate_slice_lane_large_counts() {
        // Arrange
        let slice_data = vec![FrameSliceStats::new(0, 1000)];

        // Act
        let lane = populate_slice_lane(&slice_data);

        // Assert
        assert_eq!(lane.get_value(0), Some(1000.0));
    }
}

// ============================================================================
// Calculate QP Statistics Tests
// ============================================================================

#[cfg(test)]
mod calculate_qp_statistics_tests {
    use super::*;

    #[test]
    fn test_calculate_qp_statistics() {
        // Arrange
        let lane = populate_qp_lane(&create_test_qp_stats());

        // Act
        let stats = calculate_qp_statistics(&lane);

        // Assert
        assert!(stats.is_some());
        let s = stats.unwrap();
        assert_eq!(s.min_qp, 18.0);
        assert_eq!(s.max_qp, 30.0);
        assert_eq!(s.avg_qp, 23.0); // (20+22+25+18+30)/5 = 115/5
        assert_eq!(s.frame_count, 5);
    }

    #[test]
    fn test_calculate_qp_statistics_empty() {
        // Arrange
        let lane = Lane::new(LaneType::QpAvg);

        // Act
        let stats = calculate_qp_statistics(&lane);

        // Assert
        assert!(stats.is_none());
    }

    #[test]
    fn test_calculate_qp_statistics_single() {
        // Arrange
        let mut lane = Lane::new(LaneType::QpAvg);
        lane.add_point(0, 25.0);

        // Act
        let stats = calculate_qp_statistics(&lane);

        // Assert
        assert!(stats.is_some());
        let s = stats.unwrap();
        assert_eq!(s.min_qp, 25.0);
        assert_eq!(s.max_qp, 25.0);
        assert_eq!(s.avg_qp, 25.0);
        assert_eq!(s.std_dev, 0.0);
    }

    #[test]
    fn test_calculate_qp_statistics_std_dev() {
        // Arrange
        let mut lane = Lane::new(LaneType::QpAvg);
        lane.add_point(0, 20.0);
        lane.add_point(1, 30.0);

        // Act
        let stats = calculate_qp_statistics(&lane);

        // Assert - avg=25, variance=((20-25)^2 + (30-25)^2)/2 = (25+25)/2 = 25
        let s = stats.unwrap();
        assert!((s.std_dev - 5.0).abs() < 0.001); // sqrt(25) = 5
    }
}

// ============================================================================
// Calculate Slice Statistics Tests
// ============================================================================

#[cfg(test)]
mod calculate_slice_statistics_tests {
    use super::*;

    #[test]
    fn test_calculate_slice_statistics() {
        // Arrange
        let lane = populate_slice_lane(&create_test_slice_stats());

        // Act
        let stats = calculate_slice_statistics(&lane);

        // Assert
        assert!(stats.is_some());
        let s = stats.unwrap();
        assert_eq!(s.min_slices, 1);
        assert_eq!(s.max_slices, 8);
        assert_eq!(s.avg_slices, 3.2); // (1+2+4+1+8)/5 = 16/5
        assert_eq!(s.multi_slice_frame_count, 3); // Frames 1, 2, 4 have >1 slice
        assert_eq!(s.frame_count, 5);
    }

    #[test]
    fn test_calculate_slice_statistics_empty() {
        // Arrange
        let lane = Lane::new(LaneType::SliceCount);

        // Act
        let stats = calculate_slice_statistics(&lane);

        // Assert
        assert!(stats.is_none());
    }

    #[test]
    fn test_calculate_slice_statistics_all_single() {
        // Arrange
        let mut lane = Lane::new(LaneType::SliceCount);
        for i in 0..3 {
            lane.add_point(i, 1.0);
        }

        // Act
        let stats = calculate_slice_statistics(&lane);

        // Assert
        assert!(stats.is_some());
        let s = stats.unwrap();
        assert_eq!(s.multi_slice_frame_count, 0);
    }

    #[test]
    fn test_calculate_slice_statistics_all_multi() {
        // Arrange
        let mut lane = Lane::new(LaneType::SliceCount);
        for i in 0..3 {
            lane.add_point(i, (i + 2) as f32);
        }

        // Act
        let stats = calculate_slice_statistics(&lane);

        // Assert
        assert!(stats.is_some());
        let s = stats.unwrap();
        assert_eq!(s.multi_slice_frame_count, 3);
    }
}

// ============================================================================
// Edge Cases
// ============================================================================

#[cfg(test)]
mod edge_case_tests {
    use super::*;

    #[test]
    fn test_lane_enabled_disabled() {
        // Arrange
        let mut lane = Lane::new(LaneType::QpAvg);

        // Act
        lane.enabled = false;

        // Assert
        assert!(!lane.enabled);
    }

    #[test]
    fn test_lane_opacity() {
        // Arrange
        let mut lane = Lane::new(LaneType::QpAvg);

        // Act
        lane.opacity = 0.5;

        // Assert
        assert_eq!(lane.opacity, 0.5);
    }

    #[test]
    fn test_lane_opacity_clamp_high() {
        // Arrange
        let mut lane = Lane::new(LaneType::QpAvg);

        // Act - Not actually clamped, just stored
        lane.opacity = 1.5;

        // Assert
        assert_eq!(lane.opacity, 1.5); // No clamping in implementation
    }

    #[test]
    fn test_lane_opacity_clamp_low() {
        // Arrange
        let mut lane = Lane::new(LaneType::QpAvg);

        // Act
        lane.opacity = -0.5;

        // Assert
        assert_eq!(lane.opacity, -0.5); // No clamping in implementation
    }

    #[test]
    fn test_bpp_calculates_for_small_frame() {
        // Arrange
        let frames = vec![
            TimelineFrame {
                display_idx: 0,
                size_bytes: 1,
                frame_type: "I".to_string(),
                marker: FrameMarker::Key,
                pts: None,
                dts: None,
                is_selected: false,
            },
        ];

        // Act
        let lane = populate_bpp_lane(&frames, 1, 1);

        // Assert - BPP = (1 * 8) / 1 = 8
        assert_eq!(lane.data[0].value, 8.0);
    }
}
