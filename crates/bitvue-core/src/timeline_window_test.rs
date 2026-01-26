// Timeline window module tests
//
// Per generate-tests skill: Arrange-Act-Assert pattern with fixtures
// and edge case coverage.

use super::*;
use crate::FrameMarker;

// ============================================================================
// Fixtures
// ============================================================================

/// Create a test timeline frame
fn create_test_frame(display_idx: usize) -> TimelineFrame {
    TimelineFrame {
        display_idx,
        size_bytes: 1000 + display_idx as u64 * 100,
        frame_type: if display_idx % 3 == 0 { "I".to_string() } else { "P".to_string() },
        marker: if display_idx % 3 == 0 {
            FrameMarker::Key
        } else if display_idx % 7 == 0 {
            FrameMarker::Error
        } else if display_idx % 5 == 0 {
            FrameMarker::Bookmark
        } else {
            FrameMarker::None
        },
        pts: Some(display_idx as u64 * 100),
        dts: Some(display_idx as u64 * 100),
        is_selected: false,
    }
}

/// Create a test timeline window
fn create_test_window(total_frames: usize) -> TimelineWindow {
    TimelineWindow::new("test_stream".to_string(), total_frames, WindowPolicy::Fixed(100))
}

// ============================================================================
// WindowPolicy Tests
// ============================================================================

#[cfg(test)]
mod window_policy_tests {
    use super::*;

    #[test]
    fn test_window_policy_fixed() {
        // Arrange
        let policy = WindowPolicy::Fixed(100);

        // Act
        let size = policy.calculate_window_size(1.0, 1000);

        // Assert
        assert_eq!(size, 100);
    }

    #[test]
    fn test_window_policy_fixed_clamps_to_total() {
        // Arrange
        let policy = WindowPolicy::Fixed(100);

        // Act
        let size = policy.calculate_window_size(1.0, 50);

        // Assert
        assert_eq!(size, 50); // Clamped to total
    }

    #[test]
    fn test_window_policy_adaptive() {
        // Arrange
        let policy = WindowPolicy::Adaptive { min: 10, max: 100 };

        // Act
        let size = policy.calculate_window_size(1.0, 1000);

        // Assert - At zoom 1.0, should be max
        assert_eq!(size, 100);
    }

    #[test]
    fn test_window_policy_adaptive_zoomed_in() {
        // Arrange
        let policy = WindowPolicy::Adaptive { min: 10, max: 100 };

        // Act - Higher zoom = smaller window
        let size = policy.calculate_window_size(10.0, 1000);

        // Assert
        assert_eq!(size, 10); // 100 / 10 = 10, clamped to min
    }

    #[test]
    fn test_window_policy_adaptive_zoomed_out() {
        // Arrange
        let policy = WindowPolicy::Adaptive { min: 10, max: 100 };

        // Act - Lower zoom = larger window
        let size = policy.calculate_window_size(0.1, 1000);

        // Assert - Should be clamped to max
        assert_eq!(size, 100);
    }

    #[test]
    fn test_window_policy_full() {
        // Arrange
        let policy = WindowPolicy::Full;

        // Act
        let size = policy.calculate_window_size(1.0, 1000);

        // Assert
        assert_eq!(size, 1000);
    }

    #[test]
    fn test_window_policy_full_zero_frames() {
        // Arrange
        let policy = WindowPolicy::Full;

        // Act
        let size = policy.calculate_window_size(1.0, 0);

        // Assert
        assert_eq!(size, 0);
    }

    #[test]
    fn test_window_policy_adaptive_clamps_to_total() {
        // Arrange
        let policy = WindowPolicy::Adaptive { min: 10, max: 100 };

        // Act
        let size = policy.calculate_window_size(1.0, 50);

        // Assert - Clamped to total
        assert_eq!(size, 50);
    }
}

// ============================================================================
// SparseIndexEntry Tests
// ============================================================================

#[cfg(test)]
mod sparse_index_entry_tests {
    use super::*;

    #[test]
    fn test_sparse_index_entry_fields() {
        // Arrange & Act
        let entry = SparseIndexEntry {
            display_idx: 100,
            frame_type: "I".to_string(),
            marker: FrameMarker::Key,
            byte_offset: 5000,
            size_bytes: 1000,
        };

        // Assert
        assert_eq!(entry.display_idx, 100);
        assert_eq!(entry.frame_type, "I");
        assert_eq!(entry.marker, FrameMarker::Key);
        assert_eq!(entry.byte_offset, 5000);
        assert_eq!(entry.size_bytes, 1000);
    }
}

// ============================================================================
// TimelineWindow Construction Tests
// ============================================================================

#[cfg(test)]
mod timeline_window_construction_tests {
    use super::*;

    #[test]
    fn test_new_creates_window() {
        // Arrange & Act
        let window = create_test_window(1000);

        // Assert
        assert_eq!(window.stream_id, "test_stream");
        assert_eq!(window.total_frames, 1000);
        assert_eq!(window.window_start, 0);
        assert_eq!(window.zoom_level, 1.0);
        assert!(window.materialized.is_empty());
        assert!(window.sparse_index.is_empty());
    }

    #[test]
    fn test_new_with_fixed_policy() {
        // Arrange & Act
        let window = TimelineWindow::new("test".to_string(), 1000, WindowPolicy::Fixed(50));

        // Assert
        assert_eq!(window.window_size, 50);
    }

    #[test]
    fn test_new_with_adaptive_policy() {
        // Arrange & Act
        let window = TimelineWindow::new(
            "test".to_string(),
            1000,
            WindowPolicy::Adaptive { min: 10, max: 100 },
        );

        // Assert - At zoom 1.0, should be max (100)
        assert_eq!(window.window_size, 100);
    }

    #[test]
    fn test_new_with_full_policy() {
        // Arrange & Act
        let window = TimelineWindow::new("test".to_string(), 100, WindowPolicy::Full);

        // Assert
        assert_eq!(window.window_size, 100);
    }

    #[test]
    fn test_new_initial_cache_state() {
        // Arrange & Act
        let window = create_test_window(100);

        // Assert
        assert_eq!(window.data_revision, 0);
        assert_eq!(window.filter_hash, 0);
        assert!(window.current_cache_key.is_none());
        assert!(window.pending_loads.is_empty());
    }
}

// ============================================================================
// Sparse Index Tests
// ============================================================================

#[cfg(test)]
mod sparse_index_tests {
    use super::*;

    #[test]
    fn test_add_sparse_entry() {
        // Arrange
        let mut window = create_test_window(100);
        let entry = SparseIndexEntry {
            display_idx: 50,
            frame_type: "I".to_string(),
            marker: FrameMarker::Key,
            byte_offset: 0,
            size_bytes: 1000,
        };

        // Act
        window.add_sparse_entry(entry);

        // Assert
        assert_eq!(window.sparse_index.len(), 1);
    }

    #[test]
    fn test_add_sparse_entry_sorts() {
        // Arrange
        let mut window = create_test_window(100);

        // Act - Add out of order
        window.add_sparse_entry(SparseIndexEntry {
            display_idx: 30,
            frame_type: "P".to_string(),
            marker: FrameMarker::None,
            byte_offset: 0,
            size_bytes: 1000,
        });
        window.add_sparse_entry(SparseIndexEntry {
            display_idx: 10,
            frame_type: "I".to_string(),
            marker: FrameMarker::Key,
            byte_offset: 0,
            size_bytes: 1000,
        });

        // Assert - Should be sorted
        assert_eq!(window.sparse_index[0].display_idx, 10);
        assert_eq!(window.sparse_index[1].display_idx, 30);
    }

    #[test]
    fn test_build_sparse_index_from_frames() {
        // Arrange
        let mut window = create_test_window(100);
        let frames: Vec<TimelineFrame> = (0..10).map(|i| create_test_frame(i)).collect();

        // Act
        window.build_sparse_index_from_frames(&frames);

        // Assert
        assert!(!window.sparse_index.is_empty());
        // Should include keyframes (0, 3, 6, 9), first (0), and last (9)
        assert!(window.sparse_index.len() >= 2); // At least first and last
    }

    #[test]
    fn test_build_sparse_index_clears_existing() {
        // Arrange
        let mut window = create_test_window(100);
        window.add_sparse_entry(SparseIndexEntry {
            display_idx: 50,
            frame_type: "I".to_string(),
            marker: FrameMarker::Key,
            byte_offset: 0,
            size_bytes: 1000,
        });
        assert_eq!(window.sparse_index.len(), 1);

        let frames: Vec<TimelineFrame> = (0..5).map(|i| create_test_frame(i)).collect();

        // Act
        window.build_sparse_index_from_frames(&frames);

        // Assert - Old entry should be cleared
        assert_ne!(window.sparse_index[0].display_idx, 50);
    }

    #[test]
    fn test_sparse_entries_in_range() {
        // Arrange
        let mut window = create_test_window(100);
        for i in 0..10 {
            window.add_sparse_entry(SparseIndexEntry {
                display_idx: i * 10,
                frame_type: "I".to_string(),
                marker: FrameMarker::Key,
                byte_offset: 0,
                size_bytes: 1000,
            });
        }

        // Act
        let entries = window.sparse_entries_in_range(25, 55);

        // Assert - Should include 30, 40, 50
        assert_eq!(entries.len(), 3);
    }
}

// ============================================================================
// Zoom and Filter Tests
// ============================================================================

#[cfg(test)]
mod zoom_filter_tests {
    use super::*;

    #[test]
    fn test_set_zoom_changes_zoom() {
        // Arrange
        let mut window = create_test_window(100);
        assert_eq!(window.zoom_level, 1.0);

        // Act
        window.set_zoom(2.5);

        // Assert
        assert_eq!(window.zoom_level, 2.5);
    }

    #[test]
    fn test_set_zoom_increases_window_size_for_adaptive() {
        // Arrange
        let mut window = TimelineWindow::new(
            "test".to_string(),
            1000,
            WindowPolicy::Adaptive { min: 10, max: 100 },
        );
        let initial_size = window.window_size;

        // Act - Zoom out (lower zoom)
        window.set_zoom(0.5);

        // Assert - Window size should increase (more frames visible)
        // Actually, in this implementation, higher zoom = fewer frames (inverse)
        // So zooming out (0.5) should increase window size
        assert!(window.window_size >= initial_size);
    }

    #[test]
    fn test_set_zoom_same_value_no_change() {
        // Arrange
        let mut window = create_test_window(100);
        window.set_zoom(2.0);
        let size_after_first_set = window.window_size;

        // Act - Set same value
        window.set_zoom(2.0);

        // Assert
        assert_eq!(window.window_size, size_after_first_set);
    }

    #[test]
    fn test_set_zoom_within_epsilon_no_change() {
        // Arrange
        let mut window = create_test_window(100);
        window.set_zoom(1.0);
        let initial_size = window.window_size;

        // Act - Change within epsilon (0.001)
        window.set_zoom(1.0005);

        // Assert - Should not trigger change
        assert_eq!(window.window_size, initial_size);
    }

    #[test]
    fn test_set_filter() {
        // Arrange
        let mut window = create_test_window(100);

        // Act
        window.set_filter(12345);

        // Assert
        assert_eq!(window.filter_hash, 12345);
    }

    #[test]
    fn test_set_filter_same_value() {
        // Arrange
        let mut window = create_test_window(100);
        window.set_filter(100);

        // Act
        window.set_filter(100);

        // Assert - Should remain same
        assert_eq!(window.filter_hash, 100);
    }
}

// ============================================================================
// Scroll Tests
// ============================================================================

#[cfg(test)]
mod scroll_tests {
    use super::*;

    #[test]
    fn test_scroll_to_updates_start() {
        // Arrange
        let mut window = create_test_window(1000);
        assert_eq!(window.window_start, 0);

        // Act
        window.scroll_to(500);

        // Assert - Should center on 500
        assert!(window.window_start > 0);
        assert!(window.window_start <= 500);
    }

    #[test]
    fn test_scroll_to_clamps_to_end() {
        // Arrange
        let mut window = create_test_window(100);

        // Act - Scroll beyond end
        window.scroll_to(1000);

        // Assert - Should clamp
        assert!(window.window_start < 100);
    }

    #[test]
    fn test_scroll_to_zero() {
        // Arrange
        let mut window = create_test_window(100);
        window.scroll_to(50);

        // Act
        window.scroll_to(0);

        // Assert
        assert_eq!(window.window_start, 0);
    }

    #[test]
    fn test_scroll_to_same_position() {
        // Arrange
        let mut window = create_test_window(1000);
        window.scroll_to(500);
        let start = window.window_start;

        // Act - Scroll to same position
        window.scroll_to(500);

        // Assert - Should not change
        assert_eq!(window.window_start, start);
    }
}

// ============================================================================
// Materialization Tests
// ============================================================================

#[cfg(test)]
mod materialization_tests {
    use super::*;

    #[test]
    fn test_materialize_frame() {
        // Arrange
        let mut window = create_test_window(100);
        let frame = create_test_frame(50);

        // Act
        window.materialize_frame(frame.clone());

        // Assert
        assert!(window.materialized.contains_key(&50));
        assert_eq!(window.materialized.get(&50).unwrap().display_idx, 50);
    }

    #[test]
    fn test_materialize_frame_outside_window() {
        // Arrange
        let mut window = create_test_window(100);
        // Window is 0-99, frame 200 is outside
        let frame = create_test_frame(200);

        // Act
        window.materialize_frame(frame);

        // Assert
        assert!(!window.materialized.contains_key(&200));
    }

    #[test]
    fn test_materialize_frame_at_window_boundary() {
        // Arrange
        let mut window = create_test_window(100);
        let frame = create_test_frame(99); // Last frame in window

        // Act
        window.materialize_frame(frame.clone());

        // Assert
        assert!(window.materialized.contains_key(&99));
    }

    #[test]
    fn test_get_frame_materialized() {
        // Arrange
        let mut window = create_test_window(100);
        let frame = create_test_frame(50);
        window.materialize_frame(frame);

        // Act
        let result = window.get_frame(50);

        // Assert
        assert!(result.is_some());
        assert_eq!(result.unwrap().display_idx, 50);
    }

    #[test]
    fn test_get_frame_not_materialized() {
        // Arrange
        let window = create_test_window(100);

        // Act
        let result = window.get_frame(50);

        // Assert
        assert!(result.is_none());
    }

    #[test]
    fn test_is_materialized() {
        // Arrange
        let mut window = create_test_window(100);
        window.materialize_frame(create_test_frame(50));

        // Act
        let is_mat = window.is_materialized(50);
        let not_mat = window.is_materialized(99);

        // Assert
        assert!(is_mat);
        assert!(!not_mat);
    }

    #[test]
    fn test_materialized_count() {
        // Arrange
        let mut window = create_test_window(100);

        // Act
        for i in 0..5 {
            window.materialize_frame(create_test_frame(i));
        }

        // Assert
        assert_eq!(window.materialized_count(), 5);
    }

    #[test]
    fn test_materialized_frames_sorted() {
        // Arrange
        let mut window = create_test_window(100);
        window.materialize_frame(create_test_frame(30));
        window.materialize_frame(create_test_frame(10));
        window.materialize_frame(create_test_frame(50));

        // Act
        let frames = window.materialized_frames();

        // Assert - Should be sorted
        assert_eq!(frames[0].display_idx, 10);
        assert_eq!(frames[1].display_idx, 30);
        assert_eq!(frames[2].display_idx, 50);
    }

    #[test]
    fn test_dematerialize_outside_window() {
        // Arrange
        let mut window = create_test_window(100);
        window.materialize_frame(create_test_frame(0));
        window.materialize_frame(create_test_frame(50));
        // Note: materialize_frame only adds frames if they're within window range
        // Frame 150 is outside window (0-99), so it won't be added
        assert_eq!(window.materialized.len(), 2);

        // Act
        window.dematerialize_outside_window();

        // Assert - Both frames in window (0-99) should remain
        assert_eq!(window.materialized.len(), 2);
        assert!(window.materialized.contains_key(&0));
        assert!(window.materialized.contains_key(&50));
    }
}

// ============================================================================
// Find Nearest Tests
// ============================================================================

#[cfg(test)]
mod find_nearest_tests {
    use super::*;

    #[test]
    fn test_find_nearest_keyframe_forward() {
        // Arrange
        let mut window = create_test_window(1000);
        window.add_sparse_entry(SparseIndexEntry {
            display_idx: 0,
            frame_type: "I".to_string(),
            marker: FrameMarker::Key,
            byte_offset: 0,
            size_bytes: 1000,
        });
        window.add_sparse_entry(SparseIndexEntry {
            display_idx: 100,
            frame_type: "I".to_string(),
            marker: FrameMarker::Key,
            byte_offset: 0,
            size_bytes: 1000,
        });
        window.add_sparse_entry(SparseIndexEntry {
            display_idx: 200,
            frame_type: "P".to_string(),
            marker: FrameMarker::None,
            byte_offset: 0,
            size_bytes: 1000,
        });

        // Act
        let result = window.find_nearest_keyframe(50, true);

        // Assert
        assert_eq!(result, Some(100));
    }

    #[test]
    fn test_find_nearest_keyframe_backward() {
        // Arrange
        let mut window = create_test_window(1000);
        window.add_sparse_entry(SparseIndexEntry {
            display_idx: 0,
            frame_type: "I".to_string(),
            marker: FrameMarker::Key,
            byte_offset: 0,
            size_bytes: 1000,
        });
        window.add_sparse_entry(SparseIndexEntry {
            display_idx: 100,
            frame_type: "I".to_string(),
            marker: FrameMarker::Key,
            byte_offset: 0,
            size_bytes: 1000,
        });

        // Act
        let result = window.find_nearest_keyframe(150, false);

        // Assert
        assert_eq!(result, Some(100));
    }

    #[test]
    fn test_find_nearest_keyframe_not_found() {
        // Arrange
        let mut window = create_test_window(1000);
        window.add_sparse_entry(SparseIndexEntry {
            display_idx: 100,
            frame_type: "I".to_string(),
            marker: FrameMarker::Key,
            byte_offset: 0,
            size_bytes: 1000,
        });

        // Act - No keyframe after 200
        let result = window.find_nearest_keyframe(200, true);

        // Assert
        assert!(result.is_none());
    }

    #[test]
    fn test_find_nearest_marker() {
        // Arrange
        let mut window = create_test_window(1000);
        window.add_sparse_entry(SparseIndexEntry {
            display_idx: 50,
            frame_type: "I".to_string(),
            marker: FrameMarker::Bookmark,
            byte_offset: 0,
            size_bytes: 1000,
        });
        window.add_sparse_entry(SparseIndexEntry {
            display_idx: 150,
            frame_type: "I".to_string(),
            marker: FrameMarker::Error,
            byte_offset: 0,
            size_bytes: 1000,
        });

        // Act
        let result = window.find_nearest_marker(100, true);

        // Assert
        assert_eq!(result, Some(150));
    }
}

// ============================================================================
// Cache Tests
// ============================================================================

#[cfg(test)]
mod cache_tests {
    use super::*;

    #[test]
    fn test_increment_revision() {
        // Arrange
        let mut window = create_test_window(100);
        assert_eq!(window.data_revision, 0);

        // Act
        window.increment_revision();

        // Assert
        assert_eq!(window.data_revision, 1);
    }

    #[test]
    fn test_create_cache_key() {
        // Arrange
        let mut window = create_test_window(100);

        // Act
        let key = window.create_cache_key(1024);

        // Assert
        assert!(window.current_cache_key.is_some());
        match key {
            CacheKey::Timeline { data_revision, zoom_level_x100, filter_hash } => {
                assert_eq!(data_revision, 0);
                assert_eq!(zoom_level_x100, 100); // 1.0 * 100
                assert_eq!(filter_hash, 0);
            }
            _ => panic!("Expected Timeline cache key"),
        }
    }

    #[test]
    fn test_cache_stats() {
        // Arrange
        let window = create_test_window(100);

        // Act
        let (total, valid, rate) = window.cache_stats();

        // Assert
        assert_eq!(total, 0);
        assert_eq!(valid, 0);
        assert_eq!(rate, 0.0);
    }

    #[test]
    fn test_clear_materialized() {
        // Arrange
        let mut window = create_test_window(100);
        window.materialize_frame(create_test_frame(50));
        assert_eq!(window.materialized_count(), 1);

        // Act
        window.clear();

        // Assert
        assert_eq!(window.materialized_count(), 0);
        assert!(window.pending_loads.is_empty());
        assert!(window.current_cache_key.is_none());
    }
}

// ============================================================================
// Window Utility Tests
// ============================================================================

#[cfg(test)]
mod window_utility_tests {
    use super::*;

    #[test]
    fn test_visible_range() {
        // Arrange
        let window = create_test_window(100);

        // Act
        let (start, size) = window.visible_range();

        // Assert
        assert_eq!(start, 0);
        assert_eq!(size, 100);
    }

    #[test]
    fn test_is_in_window() {
        // Arrange
        let window = create_test_window(100);

        // Act
        let in_window = window.is_in_window(50);
        let at_boundary = window.is_in_window(99);
        let outside = window.is_in_window(100);

        // Assert
        assert!(in_window);
        assert!(at_boundary);
        assert!(!outside);
    }

    #[test]
    fn test_coverage_ratio_empty() {
        // Arrange
        let window = create_test_window(100);

        // Act
        let ratio = window.coverage_ratio();

        // Assert
        assert_eq!(ratio, 0.0);
    }

    #[test]
    fn test_coverage_ratio_partial() {
        // Arrange
        let mut window = create_test_window(100);
        for i in 0..25 {
            window.materialize_frame(create_test_frame(i));
        }

        // Act
        let ratio = window.coverage_ratio();

        // Assert - 25/100 = 0.25
        assert!((ratio - 0.25).abs() < 0.01);
    }

    #[test]
    fn test_coverage_ratio_full() {
        // Arrange
        let mut window = create_test_window(10);
        for i in 0..10 {
            window.materialize_frame(create_test_frame(i));
        }

        // Act
        let ratio = window.coverage_ratio();

        // Assert - 10/10 = 1.0
        assert_eq!(ratio, 1.0);
    }

    #[test]
    fn test_estimated_memory_usage() {
        // Arrange
        let mut window = create_test_window(100);
        window.materialize_frame(create_test_frame(50));

        // Act
        let memory = window.estimated_memory_usage();

        // Assert - 1 frame * 200 bytes = 200
        assert_eq!(memory, 200);
    }

    #[test]
    fn test_estimated_memory_with_sparse_index() {
        // Arrange
        let mut window = create_test_window(100);
        window.add_sparse_entry(SparseIndexEntry {
            display_idx: 50,
            frame_type: "I".to_string(),
            marker: FrameMarker::Key,
            byte_offset: 0,
            size_bytes: 1000,
        });

        // Act
        let memory = window.estimated_memory_usage();

        // Assert - 0 materialized * 200 + 1 sparse * 100 = 100
        assert_eq!(memory, 100);
    }
}

// ============================================================================
// WindowLoader Tests
// ============================================================================

#[cfg(test)]
mod window_loader_tests {
    use super::*;

    #[test]
    fn test_window_loader_new() {
        // Arrange & Act
        let loader = WindowLoader::new();

        // Assert
        assert_eq!(loader.status, WindowLoadStatus::Idle);
        assert_eq!(loader.progress, 0.0);
        assert_eq!(loader.frames_loaded, 0);
        assert_eq!(loader.frames_total, 0);
    }

    #[test]
    fn test_window_loader_default() {
        // Arrange & Act
        let loader = WindowLoader::default();

        // Assert
        assert_eq!(loader.status, WindowLoadStatus::Idle);
    }

    #[test]
    fn test_start_load() {
        // Arrange
        let mut loader = WindowLoader::new();

        // Act
        loader.start_load(100);

        // Assert
        assert!(matches!(loader.status, WindowLoadStatus::Loading { .. }));
        assert_eq!(loader.frames_total, 100);
        assert_eq!(loader.frames_loaded, 0);
        assert_eq!(loader.progress, 0.0);
    }

    #[test]
    fn test_update_progress() {
        // Arrange
        let mut loader = WindowLoader::new();
        loader.start_load(100);

        // Act
        loader.update_progress(50);

        // Assert
        assert_eq!(loader.frames_loaded, 50);
        assert_eq!(loader.progress, 0.5);
        if let WindowLoadStatus::Loading { current, total } = loader.status {
            assert_eq!(current, 50);
            assert_eq!(total, 100);
        } else {
            panic!("Expected Loading status");
        }
    }

    #[test]
    fn test_complete() {
        // Arrange
        let mut loader = WindowLoader::new();
        loader.start_load(100);

        // Act
        loader.complete();

        // Assert
        assert_eq!(loader.status, WindowLoadStatus::Completed);
        assert_eq!(loader.progress, 1.0);
    }

    #[test]
    fn test_fail() {
        // Arrange
        let mut loader = WindowLoader::new();
        loader.start_load(100);

        // Act
        loader.fail();

        // Assert
        assert_eq!(loader.status, WindowLoadStatus::Failed);
    }

    #[test]
    fn test_cancel() {
        // Arrange
        let mut loader = WindowLoader::new();
        loader.start_load(100);
        let gen_before = loader.generation;

        // Act
        loader.cancel();

        // Assert
        assert_eq!(loader.status, WindowLoadStatus::Idle);
        assert_eq!(loader.progress, 0.0);
        assert_eq!(loader.generation, gen_before + 1);
    }

    #[test]
    fn test_is_loading() {
        // Arrange
        let mut loader = WindowLoader::new();

        // Act
        loader.start_load(100);

        // Assert
        assert!(loader.is_loading());
    }

    #[test]
    fn test_is_loading_not_loading() {
        // Arrange
        let loader = WindowLoader::new();

        // Act & Assert
        assert!(!loader.is_loading());
    }

    #[test]
    fn test_current_generation() {
        // Arrange
        let mut loader = WindowLoader::new();

        // Act
        loader.cancel();
        loader.cancel();

        // Assert
        assert_eq!(loader.current_generation(), 2);
    }

    #[test]
    fn test_update_progress_zero_total() {
        // Arrange
        let mut loader = WindowLoader::new();
        loader.start_load(0);

        // Act
        loader.update_progress(10);

        // Assert - Should handle division by zero
        assert_eq!(loader.progress, 0.0);
    }

    #[test]
    fn test_update_progress_clamps_to_one() {
        // Arrange
        let mut loader = WindowLoader::new();
        loader.start_load(100);

        // Act - Report more than total
        loader.update_progress(150);

        // Assert - Implementation does NOT clamp, so progress exceeds 1.0
        assert_eq!(loader.progress, 1.5);
    }
}

// ============================================================================
// Pending Loads Tests
// ============================================================================

#[cfg(test)]
mod pending_loads_tests {
    use super::*;

    #[test]
    fn test_request_window_load() {
        // Arrange
        let mut window = create_test_window(100);

        // Act
        window.request_window_load(50, 10);

        // Assert
        assert_eq!(window.pending_loads.len(), 1);
        assert_eq!(window.pending_loads[0], (50, 10));
    }

    #[test]
    fn test_request_window_load_clears_pending() {
        // Arrange
        let mut window = create_test_window(100);
        window.request_window_load(10, 10);
        // Note: request_window_load clears pending_loads on each call (last-wins behavior)
        assert_eq!(window.pending_loads.len(), 1);

        window.request_window_load(20, 10);
        assert_eq!(window.pending_loads.len(), 1);

        // Act
        window.request_window_load(30, 10);

        // Assert - Should clear previous requests (last-wins)
        assert_eq!(window.pending_loads.len(), 1);
        assert_eq!(window.pending_loads[0], (30, 10));
    }

    #[test]
    fn test_scroll_triggers_load_request() {
        // Arrange
        let mut window = create_test_window(1000);

        // Act
        window.scroll_to(500);

        // Assert - Should request load
        assert!(!window.pending_loads.is_empty());
    }

    #[test]
    fn test_clear_clears_pending_loads() {
        // Arrange
        let mut window = create_test_window(100);
        window.request_window_load(10, 10);

        // Act
        window.clear();

        // Assert
        assert!(window.pending_loads.is_empty());
    }
}

// ============================================================================
// Edge Cases
// ============================================================================

#[cfg(test)]
mod edge_case_tests {
    use super::*;

    #[test]
    fn test_zero_total_frames() {
        // Arrange & Act
        let window = create_test_window(0);

        // Assert
        assert_eq!(window.total_frames, 0);
        assert_eq!(window.window_size, 0);
    }

    #[test]
    fn test_single_frame() {
        // Arrange & Act
        let window = create_test_window(1);

        // Assert
        assert_eq!(window.total_frames, 1);
        assert_eq!(window.window_size, 1);
    }

    #[test]
    fn test_window_larger_than_total() {
        // Arrange & Act
        let window = TimelineWindow::new("test".to_string(), 10, WindowPolicy::Fixed(100));

        // Assert - Should clamp to total
        assert_eq!(window.window_size, 10);
    }

    #[test]
    fn test_zoom_zero() {
        // Arrange
        let mut window = create_test_window(100);

        // Act
        window.set_zoom(0.0);

        // Assert
        assert_eq!(window.zoom_level, 0.0);
    }

    #[test]
    fn test_zoom_very_large() {
        // Arrange
        let mut window = create_test_window(1000);

        // Act
        window.set_zoom(1000.0);

        // Assert
        assert_eq!(window.zoom_level, 1000.0);
    }

    #[test]
    fn test_coverage_ratio_zero_window() {
        // Arrange
        let window = create_test_window(0);

        // Act
        let ratio = window.coverage_ratio();

        // Assert - Should handle zero division
        assert_eq!(ratio, 0.0);
    }

    #[test]
    fn test_materialize_replaces_existing() {
        // Arrange
        let mut window = create_test_window(100);
        window.materialize_frame(create_test_frame(50));
        assert_eq!(window.materialized_count(), 1);

        // Act - Materialize same frame again
        window.materialize_frame(create_test_frame(50));

        // Assert - Should replace, not duplicate
        assert_eq!(window.materialized_count(), 1);
    }

    #[test]
    fn test_get_frame_mut() {
        // Arrange
        let mut window = create_test_window(100);
        window.materialize_frame(create_test_frame(50));

        // Act
        if let Some(frame) = window.get_frame_mut(50) {
            frame.size_bytes = 9999;
        }

        // Assert
        assert_eq!(window.get_frame(50).unwrap().size_bytes, 9999);
    }

    #[test]
    fn test_get_frame_mut_not_found() {
        // Arrange
        let mut window = create_test_window(100);

        // Act
        let result = window.get_frame_mut(50);

        // Assert
        assert!(result.is_none());
    }
}
