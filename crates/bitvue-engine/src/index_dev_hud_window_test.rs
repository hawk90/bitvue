// Index dev HUD window module tests
//
// Per generate-tests skill: Arrange-Act-Assert pattern with fixtures
// and edge case coverage.

use super::*;
use crate::{IndexSessionWindow, IndexWindowPolicy};
use crate::indexing::SeekPoint;
use std::collections::VecDeque;

// ============================================================================
// Fixtures
// ============================================================================

/// Create a test TimelineWindowHUD
fn create_test_hud() -> TimelineWindowHUD {
    TimelineWindowHUD::new("test_session".to_string())
}

/// Create a test window
fn create_test_window() -> IndexSessionWindow {
    let sparse_keyframes = vec![
        SeekPoint {
            display_idx: 0,
            byte_offset: 1000,
            is_keyframe: true,
            pts: Some(0u64),
        },
        SeekPoint {
            display_idx: 100,
            byte_offset: 100000,
            is_keyframe: true,
            pts: Some(10000u64),
        },
    ];
    IndexSessionWindow::new(
        "test".to_string(),
        1000,
        IndexWindowPolicy::Fixed(100),
        sparse_keyframes,
    )
}

/// Create a test window with materialized frames
fn create_test_window_with_materialized() -> IndexSessionWindow {
    let mut window = create_test_window();
    // Materialize some frames
    for i in 0..10 {
        window.materialize_frame(crate::indexing::FrameMetadata {
            display_idx: i,
            decode_idx: i,
            byte_offset: i as u64 * 1000,
            size: 100u64,
            is_keyframe: i % 3 == 0,
            pts: Some(i as u64 * 100),
            dts: Some(i as u64 * 100),
            frame_type: Some(if i % 3 == 0 { "I".to_string() } else { "P".to_string() }),
        });
    }
    window
}

// ============================================================================
// TimelineWindowHUD Construction Tests
// ============================================================================

#[cfg(test)]
mod construction_tests {
    use super::*;

    #[test]
    fn test_new_creates_hud() {
        // Arrange & Act
        let hud = create_test_hud();

        // Assert
        assert_eq!(hud.session_id(), "test_session");
        assert_eq!(hud.update_count(), 0);
    }

    #[test]
    fn test_new_initial_window_viz() {
        // Arrange & Act
        let hud = create_test_hud();

        // Assert
        assert_eq!(hud.window_viz().total_frames, 0);
        assert_eq!(hud.window_viz().window_start, 0);
        assert_eq!(hud.window_viz().window_end, 0);
    }

    #[test]
    fn test_new_initial_materialization_tracker() {
        // Arrange & Act
        let hud = create_test_hud();

        // Assert
        assert_eq!(hud.materialization_tracker().total_requests, 0);
        assert_eq!(hud.materialization_tracker().cache_hits, 0);
        assert_eq!(hud.materialization_tracker().cache_misses, 0);
    }

    #[test]
    fn test_new_initial_performance_metrics() {
        // Arrange & Act
        let hud = create_test_hud();

        // Assert
        assert_eq!(hud.performance().avg_access_time_us, 0.0);
        assert_eq!(hud.performance().max_access_time_us, 0);
        assert_eq!(hud.performance().blocking_operations, 0);
    }

    #[test]
    fn test_new_initial_recent_pattern_empty() {
        // Arrange & Act
        let hud = create_test_hud();

        // Assert
        assert!(hud.materialization_tracker().recent_pattern.is_empty());
    }
}

// ============================================================================
// Update From Window Tests
// ============================================================================

#[cfg(test)]
mod update_from_window_tests {
    use super::*;

    #[test]
    fn test_update_from_window_increments_count() {
        // Arrange
        let mut hud = create_test_hud();
        let window = create_test_window();

        // Act
        hud.update_from_window(&window);

        // Assert
        assert_eq!(hud.update_count(), 1);
    }

    #[test]
    fn test_update_from_window_updates_viz() {
        // Arrange
        let mut hud = create_test_hud();
        let window = create_test_window();

        // Act
        hud.update_from_window(&window);

        // Assert
        assert_eq!(hud.window_viz().total_frames, 1000);
        assert_eq!(hud.window_viz().window_start, 0);
        assert_eq!(hud.window_viz().window_end, 100);
    }

    #[test]
    fn test_update_from_window_with_materialized() {
        // Arrange
        let mut hud = create_test_hud();
        let window = create_test_window_with_materialized();

        // Act
        hud.update_from_window(&window);

        // Assert
        assert!(!hud.window_viz().materialized_indices.is_empty());
    }

    #[test]
    fn test_update_from_window_sparse_keyframes() {
        // Arrange
        let mut hud = create_test_hud();
        let window = create_test_window();

        // Act
        hud.update_from_window(&window);

        // Assert
        assert_eq!(hud.window_viz().sparse_keyframes.len(), 2);
    }

    #[test]
    fn test_update_from_window_coverage_percent() {
        // Arrange
        let mut hud = create_test_hud();
        let window = create_test_window_with_materialized();

        // Act
        hud.update_from_window(&window);

        // Assert
        assert!(hud.window_viz().coverage_percent > 0.0);
    }
}

// ============================================================================
// Record Access Tests
// ============================================================================

#[cfg(test)]
mod record_access_tests {
    use super::*;

    #[test]
    fn test_record_access_hit() {
        // Arrange
        let mut hud = create_test_hud();

        // Act
        hud.record_access(100, true, 500);

        // Assert
        assert_eq!(hud.materialization_tracker().total_requests, 1);
        assert_eq!(hud.materialization_tracker().cache_hits, 1);
        assert_eq!(hud.materialization_tracker().cache_misses, 0);
    }

    #[test]
    fn test_record_access_miss() {
        // Arrange
        let mut hud = create_test_hud();

        // Act
        hud.record_access(100, false, 500);

        // Assert
        assert_eq!(hud.materialization_tracker().total_requests, 1);
        assert_eq!(hud.materialization_tracker().cache_hits, 0);
        assert_eq!(hud.materialization_tracker().cache_misses, 1);
    }

    #[test]
    fn test_record_access_adds_to_pattern() {
        // Arrange
        let mut hud = create_test_hud();

        // Act
        hud.record_access(100, true, 500);
        hud.record_access(101, false, 500);

        // Assert
        assert_eq!(hud.materialization_tracker().recent_pattern.len(), 2);
    }

    #[test]
    fn test_record_access_updates_performance() {
        // Arrange
        let mut hud = create_test_hud();

        // Act
        hud.record_access(100, true, 1000);

        // Assert
        assert_eq!(hud.performance().total_operations, 1);
        assert_eq!(hud.performance().max_access_time_us, 1000);
    }

    #[test]
    fn test_record_access_non_blocking() {
        // Arrange
        let mut hud = create_test_hud();

        // Act
        hud.record_access(100, true, 500); // 0.5ms, not blocking

        // Assert
        assert_eq!(hud.performance().blocking_operations, 0);
    }

    #[test]
    fn test_record_access_blocking() {
        // Arrange
        let mut hud = create_test_hud();

        // Act
        hud.record_access(100, true, 20000); // 20ms, blocking for 60fps

        // Assert
        assert_eq!(hud.performance().blocking_operations, 1);
    }

    #[test]
    fn test_record_access_bounded_pattern() {
        // Arrange
        let mut hud = create_test_hud();

        // Act - Add more than 100 events
        for i in 0..150 {
            hud.record_access(i, true, 100);
        }

        // Assert - Should be bounded to 100
        assert!(hud.materialization_tracker().recent_pattern.len() <= 100);
    }
}

// ============================================================================
// Record Window Move Tests
// ============================================================================

#[cfg(test)]
mod record_window_move_tests {
    use super::*;

    #[test]
    fn test_record_window_move_increments_count() {
        // Arrange
        let mut hud = create_test_hud();

        // Act
        hud.record_window_move(5000);

        // Assert
        assert_eq!(hud.materialization_tracker().window_moves, 1);
    }

    #[test]
    fn test_record_window_move_adds_to_pattern() {
        // Arrange
        let mut hud = create_test_hud();

        // Act
        hud.record_window_move(5000);

        // Assert
        assert_eq!(hud.materialization_tracker().recent_pattern.len(), 1);
        assert_eq!(
            hud.materialization_tracker().recent_pattern[0],
            AccessEvent::WindowMove
        );
    }

    #[test]
    fn test_record_window_move_tracks_latency() {
        // Arrange
        let mut hud = create_test_hud();

        // Act
        hud.record_window_move(7500);

        // Assert
        assert_eq!(hud.performance().window_adjust_latency_us.len(), 1);
        assert_eq!(hud.performance().window_adjust_latency_us[0], 7500);
    }

    #[test]
    fn test_record_window_move_bounded_latency() {
        // Arrange
        let mut hud = create_test_hud();

        // Act - Add more than 1000 latencies
        for i in 0..1500 {
            hud.record_window_move(i);
        }

        // Assert - Should be bounded to 1000
        assert!(hud.performance().window_adjust_latency_us.len() <= 1000);
    }
}

// ============================================================================
// Record Eviction Tests
// ============================================================================

#[cfg(test)]
mod record_eviction_tests {
    use super::*;

    #[test]
    fn test_record_eviction_increments_count() {
        // Arrange
        let mut hud = create_test_hud();

        // Act
        hud.record_eviction(100);

        // Assert
        assert_eq!(hud.materialization_tracker().frames_evicted, 1);
    }

    #[test]
    fn test_record_eviction_adds_to_pattern() {
        // Arrange
        let mut hud = create_test_hud();

        // Act
        hud.record_eviction(100);

        // Assert
        assert_eq!(hud.materialization_tracker().recent_pattern.len(), 1);
        assert_eq!(
            hud.materialization_tracker().recent_pattern[0],
            AccessEvent::Eviction
        );
    }

    #[test]
    fn test_record_eviction_bounded_pattern() {
        // Arrange
        let mut hud = create_test_hud();

        // Act - Add more than 100 evictions
        for i in 0..150 {
            hud.record_eviction(i);
        }

        // Assert
        assert!(hud.materialization_tracker().recent_pattern.len() <= 100);
    }
}

// ============================================================================
// Format Window Viz Tests
// ============================================================================

#[cfg(test)]
mod format_window_viz_tests {
    use super::*;

    #[test]
    fn test_format_window_viz_no_frames() {
        // Arrange
        let hud = create_test_hud();

        // Act
        let viz = hud.format_window_viz(60);

        // Assert
        assert!(viz.contains("[No frames]"));
    }

    #[test]
    fn test_format_window_viz_with_frames() {
        // Arrange
        let mut hud = create_test_hud();
        let window = create_test_window_with_materialized();
        hud.update_from_window(&window);

        // Act
        let viz = hud.format_window_viz(60);

        // Assert
        assert!(viz.contains('['));
        assert!(viz.contains(']'));
        assert!(viz.contains('\n'));
    }

    #[test]
    fn test_format_window_viz_includes_legend() {
        // Arrange
        let mut hud = create_test_hud();
        let window = create_test_window_with_materialized();
        hud.update_from_window(&window);

        // Act
        let viz = hud.format_window_viz(60);

        // Assert
        assert!(viz.contains("░=window"));
        assert!(viz.contains("█=materialized"));
        assert!(viz.contains("▲=position"));
        assert!(viz.contains("·=keyframe"));
    }

    #[test]
    fn test_format_window_viz_width() {
        // Arrange
        let mut hud = create_test_hud();
        let window = create_test_window_with_materialized();
        hud.update_from_window(&window);

        // Act
        let viz = hud.format_window_viz(40);

        // Assert - Line with brackets should be width + 2
        let first_line = viz.lines().next().unwrap();
        assert_eq!(first_line.chars().count(), 42); // 40 + 2 brackets
    }
}

// ============================================================================
// Format Access Pattern Tests
// ============================================================================

#[cfg(test)]
mod format_access_pattern_tests {
    use super::*;

    #[test]
    fn test_format_access_pattern_empty() {
        // Arrange
        let hud = create_test_hud();

        // Act
        let pattern = hud.format_access_pattern();

        // Assert
        assert!(pattern.contains("Recent: []"));
    }

    #[test]
    fn test_format_access_pattern_with_events() {
        // Arrange
        let mut hud = create_test_hud();
        hud.record_access(0, true, 100);
        hud.record_access(1, false, 100);
        hud.record_window_move(100);

        // Act
        let pattern = hud.format_access_pattern();

        // Assert
        assert!(pattern.contains('H')); // Hit
        assert!(pattern.contains('M')); // Miss
        assert!(pattern.contains('W')); // Window move
    }

    #[test]
    fn test_format_access_pattern_includes_legend() {
        // Arrange
        let mut hud = create_test_hud();
        hud.record_access(0, true, 100);

        // Act
        let pattern = hud.format_access_pattern();

        // Assert
        assert!(pattern.contains("H=hit"));
        assert!(pattern.contains("M=miss"));
        assert!(pattern.contains("W=window-move"));
        assert!(pattern.contains("E=eviction"));
    }

    #[test]
    fn test_format_access_pattern_wrapping() {
        // Arrange
        let mut hud = create_test_hud();
        // Add 25 events (should wrap at 20)
        for i in 0..25 {
            hud.record_access(i, true, 100);
        }

        // Act
        let pattern = hud.format_access_pattern();

        // Assert - Should have a space for wrapping
        assert!(pattern.contains(' '));
    }
}

// ============================================================================
// Format Density Histogram Tests
// ============================================================================

#[cfg(test)]
mod format_density_histogram_tests {
    use super::*;

    #[test]
    fn test_format_density_histogram_empty() {
        // Arrange
        let hud = create_test_hud();

        // Act
        let histogram = hud.format_density_histogram();

        // Assert
        assert!(histogram.contains("Density:"));
    }

    #[test]
    fn test_format_density_histogram_with_data() {
        // Arrange
        let mut hud = create_test_hud();
        let window = create_test_window_with_materialized();
        hud.update_from_window(&window);

        // Act
        let histogram = hud.format_density_histogram();

        // Assert
        assert!(histogram.contains("Density:"));
        // Should have 10 bucket characters
        let line = histogram.lines().next().unwrap();
        assert!(line.len() >= "Density: ".len() + 10);
    }

    #[test]
    fn test_format_density_histogram_includes_bars() {
        // Arrange
        let mut hud = create_test_hud();
        let window = create_test_window_with_materialized();
        hud.update_from_window(&window);

        // Act
        let histogram = hud.format_density_histogram();

        // Assert - Should contain some bar characters
        assert!(histogram.contains('█') || histogram.contains('▇')
            || histogram.contains('▅') || histogram.contains('_'));
    }
}

// ============================================================================
// Format Text Tests
// ============================================================================

#[cfg(test)]
mod format_text_tests {
    use super::*;

    #[test]
    fn test_format_text_includes_session_id() {
        // Arrange
        let hud = create_test_hud();

        // Act
        let text = hud.format_text();

        // Assert
        assert!(text.contains("test_session"));
    }

    #[test]
    fn test_format_text_includes_update_count() {
        // Arrange
        let hud = create_test_hud();

        // Act
        let text = hud.format_text();

        // Assert
        assert!(text.contains("Updates: 0"));
    }

    #[test]
    fn test_format_text_includes_window_state() {
        // Arrange
        let mut hud = create_test_hud();
        let window = create_test_window();
        hud.update_from_window(&window);

        // Act
        let text = hud.format_text();

        // Assert
        assert!(text.contains("Window State"));
        assert!(text.contains("Total Frames: 1000"));
    }

    #[test]
    fn test_format_text_includes_materialization() {
        // Arrange
        let mut hud = create_test_hud();
        hud.record_access(0, true, 100);
        hud.record_access(1, false, 100);

        // Act
        let text = hud.format_text();

        // Assert
        assert!(text.contains("Materialization"));
        assert!(text.contains("Total Requests: 2"));
    }

    #[test]
    fn test_format_text_includes_performance() {
        // Arrange
        let mut hud = create_test_hud();
        hud.record_access(0, true, 1000);

        // Act
        let text = hud.format_text();

        // Assert
        assert!(text.contains("Performance"));
        assert!(text.contains("Avg Access Time"));
        assert!(text.contains("Max Access Time"));
    }
}

// ============================================================================
// AccessEvent Tests
// ============================================================================

#[cfg(test)]
mod access_event_tests {
    use super::*;

    #[test]
    fn test_access_event_hit() {
        // Arrange & Act
        let event = AccessEvent::Hit;

        // Assert
        assert_eq!(event, AccessEvent::Hit);
    }

    #[test]
    fn test_access_event_miss() {
        // Arrange & Act
        let event = AccessEvent::Miss;

        // Assert
        assert_eq!(event, AccessEvent::Miss);
    }

    #[test]
    fn test_access_event_window_move() {
        // Arrange & Act
        let event = AccessEvent::WindowMove;

        // Assert
        assert_eq!(event, AccessEvent::WindowMove);
    }

    #[test]
    fn test_access_event_eviction() {
        // Arrange & Act
        let event = AccessEvent::Eviction;

        // Assert
        assert_eq!(event, AccessEvent::Eviction);
    }

    #[test]
    fn test_access_event_partial_eq() {
        // Arrange
        let hit1 = AccessEvent::Hit;
        let hit2 = AccessEvent::Hit;
        let miss = AccessEvent::Miss;

        // Assert
        assert_eq!(hit1, hit2);
        assert_ne!(hit1, miss);
    }
}

// ============================================================================
// WindowVisualization Tests
// ============================================================================

#[cfg(test)]
mod window_visualization_tests {
    use super::*;

    #[test]
    fn test_window_visualization_default() {
        // Arrange & Act
        let viz = WindowVisualization {
            total_frames: 0,
            window_start: 0,
            window_end: 0,
            current_position: 0,
            materialized_indices: vec![],
            sparse_keyframes: vec![],
            coverage_percent: 0.0,
        };

        // Assert
        assert_eq!(viz.total_frames, 0);
        assert_eq!(viz.coverage_percent, 0.0);
    }
}

// ============================================================================
// MaterializationTracker Tests
// ============================================================================

#[cfg(test)]
mod materialization_tracker_tests {
    use super::*;

    #[test]
    fn test_materialization_tracker_default() {
        // Arrange & Act
        let tracker = MaterializationTracker {
            total_requests: 0,
            cache_hits: 0,
            cache_misses: 0,
            frames_evicted: 0,
            window_moves: 0,
            recent_pattern: VecDeque::new(),
            density_histogram: vec![],
        };

        // Assert
        assert_eq!(tracker.total_requests, 0);
        assert!(tracker.recent_pattern.is_empty());
    }
}

// ============================================================================
// WindowPerformanceMetrics Tests
// ============================================================================

#[cfg(test)]
mod window_performance_metrics_tests {
    use super::*;

    #[test]
    fn test_window_performance_metrics_default() {
        // Arrange & Act
        let metrics = WindowPerformanceMetrics {
            avg_access_time_us: 0.0,
            max_access_time_us: 0,
            p95_access_time_us: 0,
            p99_access_time_us: 0,
            blocking_operations: 0,
            total_operations: 0,
            window_adjust_latency_us: vec![],
        };

        // Assert
        assert_eq!(metrics.avg_access_time_us, 0.0);
        assert_eq!(metrics.blocking_operations, 0);
    }
}

// ============================================================================
// Edge Cases
// ============================================================================

#[cfg(test)]
mod edge_case_tests {
    use super::*;

    #[test]
    fn test_density_histogram_no_frames() {
        // Arrange
        let hud = create_test_hud();

        // Act - Window with 0 total frames
        let histogram = hud.format_density_histogram();

        // Assert
        assert!(histogram.contains("Density:"));
    }

    #[test]
    fn test_coverage_zero_window_size() {
        // Arrange
        let hud = create_test_hud();

        // Act - Window with 0 total frames (simulated via WindowVisualization)
        // The actual coverage will be 0.0 when total_frames is 0

        // Assert - Coverage should be 0.0 when no frames
        assert_eq!(hud.window_viz().coverage_percent, 0.0);
    }

    #[test]
    fn test_performance_percentiles_with_single_sample() {
        // Arrange
        let mut hud = create_test_hud();

        // Act
        hud.record_access(0, true, 1000);

        // Assert
        // P95/P99 should be calculated from avg and max
        assert!(hud.performance().p95_access_time_us > 0);
        assert!(hud.performance().p99_access_time_us > 0);
    }

    #[test]
    fn test_record_access_timing_updates_rolling_average() {
        // Arrange
        let mut hud = create_test_hud();

        // Act - Record multiple accesses with different times
        hud.record_access(0, true, 1000);
        hud.record_access(1, true, 2000);
        hud.record_access(2, true, 3000);

        // Assert - Average should be (1000 + 2000 + 3000) / 3 = 2000
        assert_eq!(hud.performance().avg_access_time_us, 2000.0);
    }

    #[test]
    fn test_update_from_window_updates_tracker() {
        // Arrange
        let mut hud = create_test_hud();
        let mut window = create_test_window();
        // Manually move window to increment stats
        window.set_position(500);

        // Act
        hud.update_from_window(&window);

        // Assert
        assert_eq!(hud.materialization_tracker().window_moves, window.stats().window_moves);
    }

    #[test]
    fn test_multiple_window_updates() {
        // Arrange
        let mut hud = create_test_hud();
        let window = create_test_window();

        // Act
        for _ in 0..5 {
            hud.update_from_window(&window);
        }

        // Assert
        assert_eq!(hud.update_count(), 5);
    }

    #[test]
    fn test_blocking_threshold() {
        // Arrange
        let mut hud = create_test_hud();

        // Act - Exactly at threshold (16ms = 16000us)
        hud.record_access(0, true, 16001);

        // Assert
        assert_eq!(hud.performance().blocking_operations, 1);
    }

    #[test]
    fn test_non_blocking_threshold() {
        // Arrange
        let mut hud = create_test_hud();

        // Act - Just below threshold
        hud.record_access(0, true, 15999);

        // Assert
        assert_eq!(hud.performance().blocking_operations, 0);
    }
}
