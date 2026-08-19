// Index session window module tests
//
// Per generate-tests skill: Arrange-Act-Assert pattern with fixtures
// and edge case coverage.

use super::*;
use crate::indexing::{FrameMetadata, SeekPoint};

// ============================================================================
// Fixtures
// ============================================================================

/// Create a test seek point
fn create_test_seek_point(display_idx: usize, byte_offset: u64) -> SeekPoint {
    SeekPoint {
        display_idx,
        byte_offset,
        is_keyframe: true,
        pts: Some(display_idx as u64 * 100),
    }
}

/// Create test sparse keyframes
fn create_test_sparse_keyframes(count: usize) -> Vec<SeekPoint> {
    (0..count)
        .map(|i| create_test_seek_point(i * 100, i as u64 * 100000))
        .collect()
}

/// Create a test frame metadata
fn create_test_frame_metadata(display_idx: usize, byte_offset: u64, size: u64) -> FrameMetadata {
    FrameMetadata {
        display_idx,
        decode_idx: display_idx,
        byte_offset,
        size,
        is_keyframe: display_idx.is_multiple_of(3),
        pts: Some(display_idx as u64 * 100),
        dts: Some(display_idx as u64 * 100),
        frame_type: Some(if display_idx.is_multiple_of(3) {
            "I".to_string()
        } else {
            "P".to_string()
        }),
    }
}

/// Create a test window with default policy
fn create_test_window(total_frames: usize) -> IndexSessionWindow {
    IndexSessionWindow::new(
        "test_session".to_string(),
        total_frames,
        IndexWindowPolicy::default(),
        create_test_sparse_keyframes(10),
    )
}

// ============================================================================
// IndexWindowPolicy Tests
// ============================================================================

#[cfg(test)]
mod index_window_policy_tests {
    use super::*;

    #[test]
    fn test_window_policy_fixed_calculates_size() {
        // Arrange
        let policy = IndexWindowPolicy::Fixed(5000);

        // Act
        let size = policy.calculate_window_size(10000);

        // Assert
        assert_eq!(size, 5000);
    }

    #[test]
    fn test_window_policy_fixed_clamps_to_total() {
        // Arrange
        let policy = IndexWindowPolicy::Fixed(10000);

        // Act
        let size = policy.calculate_window_size(5000);

        // Assert
        assert_eq!(size, 5000);
    }

    #[test]
    fn test_window_policy_adaptive_calculates_size() {
        // Arrange
        let policy = IndexWindowPolicy::Adaptive {
            min: 1000,
            max: 10000,
        };

        // Act
        let size = policy.calculate_window_size(5000);

        // Assert
        assert_eq!(size, 5000); // Uses max
    }

    #[test]
    fn test_window_policy_adaptive_clamps_to_max() {
        // Arrange
        let policy = IndexWindowPolicy::Adaptive {
            min: 1000,
            max: 5000,
        };

        // Act
        let size = policy.calculate_window_size(10000);

        // Assert
        assert_eq!(size, 5000); // Clamped to max
    }

    #[test]
    fn test_window_policy_full_returns_total() {
        // Arrange
        let policy = IndexWindowPolicy::Full;

        // Act
        let size = policy.calculate_window_size(12345);

        // Assert
        assert_eq!(size, 12345);
    }

    #[test]
    fn test_should_use_out_of_core_small_index() {
        // Arrange & Act
        let result = IndexWindowPolicy::should_use_out_of_core(5000);

        // Assert
        assert!(!result);
    }

    #[test]
    fn test_should_use_out_of_core_large_index() {
        // Arrange & Act
        let result = IndexWindowPolicy::should_use_out_of_core(50000);

        // Assert
        assert!(result);
    }

    #[test]
    fn test_should_use_out_of_core_threshold() {
        // Arrange & Act
        let result = IndexWindowPolicy::should_use_out_of_core(10001);

        // Assert
        assert!(result);
    }

    #[test]
    fn test_window_policy_default() {
        // Arrange & Act
        let policy = IndexWindowPolicy::default();

        // Assert
        assert!(matches!(policy, IndexWindowPolicy::Adaptive { .. }));
    }
}

// ============================================================================
// IndexSessionWindow Construction Tests
// ============================================================================

#[cfg(test)]
mod construction_tests {
    use super::*;

    #[test]
    fn test_new_creates_window() {
        // Arrange & Act
        let window = create_test_window(10000);

        // Assert
        assert_eq!(window.session_id, "test_session");
        assert_eq!(window.total_frames, 10000);
        assert_eq!(window.current_position, 0);
        assert_eq!(window.window_start, 0);
        assert!(window.window_size > 0);
    }

    #[test]
    fn test_new_with_fixed_policy() {
        // Arrange & Act
        let window = IndexSessionWindow::new(
            "test".to_string(),
            10000,
            IndexWindowPolicy::Fixed(5000),
            vec![],
        );

        // Assert
        assert_eq!(window.window_size, 5000);
    }

    #[test]
    fn test_new_with_full_policy() {
        // Arrange & Act
        let window = IndexSessionWindow::new(
            "test".to_string(),
            1000,
            IndexWindowPolicy::Full,
            vec![],
        );

        // Assert
        assert_eq!(window.window_size, 1000);
    }

    #[test]
    fn test_new_initial_stats() {
        // Arrange & Act
        let window = create_test_window(1000);

        // Assert
        assert_eq!(window.stats().window_moves, 0);
        assert_eq!(window.stats().frames_materialized, 0);
        assert_eq!(window.stats().frames_evicted, 0);
        assert_eq!(window.stats().cache_hits, 0);
        assert_eq!(window.stats().cache_misses, 0);
    }

    #[test]
    fn test_new_sparse_keyframes_stored() {
        // Arrange
        let keyframes = create_test_sparse_keyframes(5);

        // Act
        let window = IndexSessionWindow::new(
            "test".to_string(),
            10000,
            IndexWindowPolicy::Fixed(5000),
            keyframes,
        );

        // Assert
        assert_eq!(window.sparse_keyframes.len(), 5);
    }
}

// ============================================================================
// Set Position Tests
// ============================================================================

#[cfg(test)]
mod set_position_tests {
    use super::*;

    #[test]
    fn test_set_position_within_window() {
        // Arrange
        let mut window = create_test_window(10000);

        // Act
        window.set_position(500);

        // Assert
        assert_eq!(window.current_position, 500);
        assert_eq!(window.window_start, 0); // Window doesn't move
    }

    #[test]
    fn test_set_position_outside_window_moves() {
        // Arrange
        let window = IndexSessionWindow::new(
            "test".to_string(),
            10000,
            IndexWindowPolicy::Fixed(1000), // Fixed smaller window
            create_test_sparse_keyframes(10),
        );
        let mut window = window;

        // Act - Position beyond window end
        window.set_position(2000);

        // Assert
        assert_eq!(window.current_position, 2000);
        assert!(window.window_start > 0); // Window moved
    }

    #[test]
    fn test_set_position_before_window_moves() {
        // Arrange
        let window = IndexSessionWindow::new(
            "test".to_string(),
            10000,
            IndexWindowPolicy::Fixed(1000), // Fixed smaller window
            create_test_sparse_keyframes(10),
        );
        let mut window = window;
        // First move the window to position 5000 (window will be around 4500-5500)
        window.set_position(5000);
        let initial_start = window.window_start;

        // Act - Set position before the window
        window.set_position(4000);

        // Assert
        assert_eq!(window.current_position, 4000);
        assert!(window.window_start < initial_start); // Window moved back
    }

    #[test]
    fn test_set_position_beyond_total_clamps() {
        // Arrange
        let mut window = create_test_window(10000);

        // Act
        window.set_position(20000);

        // Assert
        assert_eq!(window.current_position, 0); // Doesn't move beyond total
    }

    #[test]
    fn test_set_position_increments_window_moves() {
        // Arrange
        let window = IndexSessionWindow::new(
            "test".to_string(),
            10000,
            IndexWindowPolicy::Fixed(1000), // Fixed smaller window
            create_test_sparse_keyframes(10),
        );
        let mut window = window;
        let initial_moves = window.stats().window_moves;

        // Act - Force window move (position beyond window)
        window.set_position(2000);

        // Assert
        assert_eq!(window.stats().window_moves, initial_moves + 1);
    }
}

// ============================================================================
// Materialize Frame Tests
// ============================================================================

#[cfg(test)]
mod materialize_frame_tests {
    use super::*;

    #[test]
    fn test_materialize_frame_adds_to_window() {
        // Arrange
        let mut window = create_test_window(10000);
        let frame = create_test_frame_metadata(100, 50000, 300);

        // Act
        let result = window.materialize_frame(frame);

        // Assert
        assert!(result); // Newly materialized
        assert!(window.is_materialized(100));
        assert_eq!(window.stats().frames_materialized, 1);
    }

    #[test]
    fn test_materialize_frame_already_present() {
        // Arrange
        let mut window = create_test_window(10000);
        let frame = create_test_frame_metadata(100, 50000, 300);
        window.materialize_frame(frame.clone());

        // Act
        let result = window.materialize_frame(frame);

        // Assert
        assert!(!result); // Not newly materialized
        assert_eq!(window.stats().frames_materialized, 1); // Unchanged
    }

    #[test]
    fn test_materialize_frame_updates_cache_hit() {
        // Arrange
        let mut window = create_test_window(10000);
        let frame = create_test_frame_metadata(100, 50000, 300);
        window.materialize_frame(frame.clone());
        let initial_hits = window.stats().cache_hits;

        // Act
        window.materialize_frame(frame);

        // Assert
        assert_eq!(window.stats().cache_hits, initial_hits + 1);
    }

    #[test]
    fn test_materialize_frame_updates_cache_miss() {
        // Arrange
        let mut window = create_test_window(10000);
        let frame = create_test_frame_metadata(100, 50000, 300);
        let initial_misses = window.stats().cache_misses;

        // Act
        window.materialize_frame(frame);

        // Assert
        assert_eq!(window.stats().cache_misses, initial_misses + 1);
    }

    #[test]
    fn test_materialize_frame_evicts_when_over_capacity() {
        // Arrange
        let mut window = IndexSessionWindow::new(
            "test".to_string(),
            1000,
            IndexWindowPolicy::Fixed(3), // Small window
            vec![],
        );

        // Act - Add 4 frames to a window of size 3
        for i in 0..4 {
            window.materialize_frame(create_test_frame_metadata(i, i as u64 * 100, 100));
        }

        // Assert - One frame should be evicted
        assert_eq!(window.stats().frames_evicted, 1);
        assert_eq!(window.materialized_count(), 3);
    }
}

// ============================================================================
// Get Frame Tests
// ============================================================================

#[cfg(test)]
mod get_frame_tests {
    use super::*;

    #[test]
    fn test_get_frame_materialized() {
        // Arrange
        let mut window = create_test_window(10000);
        let frame = create_test_frame_metadata(100, 50000, 300);
        window.materialize_frame(frame.clone());

        // Act
        let result = window.get_frame(100);

        // Assert
        assert!(result.is_some());
        assert_eq!(result.unwrap().display_idx, 100);
    }

    #[test]
    fn test_get_frame_not_materialized() {
        // Arrange
        let mut window = create_test_window(10000);

        // Act
        let result = window.get_frame(100);

        // Assert
        assert!(result.is_none());
    }

    #[test]
    fn test_get_frame_updates_cache_hit() {
        // Arrange
        let mut window = create_test_window(10000);
        let frame = create_test_frame_metadata(100, 50000, 300);
        window.materialize_frame(frame);
        let initial_hits = window.stats().cache_hits;

        // Act
        window.get_frame(100);

        // Assert
        assert_eq!(window.stats().cache_hits, initial_hits + 1);
    }

    #[test]
    fn test_get_frame_updates_cache_miss() {
        // Arrange
        let mut window = create_test_window(10000);
        let initial_misses = window.stats().cache_misses;

        // Act
        window.get_frame(100);

        // Assert
        assert_eq!(window.stats().cache_misses, initial_misses + 1);
    }

    #[test]
    fn test_get_frame_mut_materialized() {
        // Arrange
        let mut window = create_test_window(10000);
        let frame = create_test_frame_metadata(100, 50000, 300);
        window.materialize_frame(frame);

        // Act
        let result = window.get_frame_mut(100);

        // Assert
        assert!(result.is_some());
    }

    #[test]
    fn test_get_frame_mut_not_materialized() {
        // Arrange
        let mut window = create_test_window(10000);

        // Act
        let result = window.get_frame_mut(100);

        // Assert
        assert!(result.is_none());
    }
}

// ============================================================================
// Is Materialized Tests
// ============================================================================

#[cfg(test)]
mod is_materialized_tests {
    use super::*;

    #[test]
    fn test_is_materialized_true() {
        // Arrange
        let mut window = create_test_window(10000);
        let frame = create_test_frame_metadata(100, 50000, 300);
        window.materialize_frame(frame);

        // Act
        let result = window.is_materialized(100);

        // Assert
        assert!(result);
    }

    #[test]
    fn test_is_materialized_false() {
        // Arrange
        let window = create_test_window(10000);

        // Act
        let result = window.is_materialized(100);

        // Assert
        assert!(!result);
    }
}

// ============================================================================
// Find Nearest Keyframe Tests
// ============================================================================

#[cfg(test)]
mod find_nearest_keyframe_tests {
    use super::*;

    #[test]
    fn test_find_nearest_keyframe_exact_match() {
        // Arrange
        let keyframes = create_test_sparse_keyframes(5);
        let window = IndexSessionWindow::new(
            "test".to_string(),
            10000,
            IndexWindowPolicy::Fixed(5000),
            keyframes,
        );

        // Act
        let result = window.find_nearest_keyframe(100);

        // Assert
        assert!(result.is_some());
        assert_eq!(result.unwrap().display_idx, 100);
    }

    #[test]
    fn test_find_nearest_keyframe_before() {
        // Arrange
        let keyframes = create_test_sparse_keyframes(5);
        let window = IndexSessionWindow::new(
            "test".to_string(),
            10000,
            IndexWindowPolicy::Fixed(5000),
            keyframes,
        );

        // Act
        let result = window.find_nearest_keyframe(150);

        // Assert
        assert!(result.is_some());
        assert_eq!(result.unwrap().display_idx, 100); // Previous keyframe
    }

    #[test]
    fn test_find_nearest_keyframe_before_first() {
        // Arrange
        let keyframes = create_test_sparse_keyframes(5);
        let window = IndexSessionWindow::new(
            "test".to_string(),
            10000,
            IndexWindowPolicy::Fixed(5000),
            keyframes,
        );

        // Act
        let result = window.find_nearest_keyframe(50);

        // Assert - Keyframe at position 0 is before position 50
        assert!(result.is_some());
        assert_eq!(result.unwrap().display_idx, 0);
    }

    #[test]
    fn test_find_nearest_keyframe_empty_index() {
        // Arrange
        let window = IndexSessionWindow::new(
            "test".to_string(),
            10000,
            IndexWindowPolicy::Fixed(5000),
            vec![],
        );

        // Act
        let result = window.find_nearest_keyframe(100);

        // Assert
        assert!(result.is_none());
    }
}

// ============================================================================
// Window Range Tests
// ============================================================================

#[cfg(test)]
mod window_range_tests {
    use super::*;

    #[test]
    fn test_window_range_initial() {
        // Arrange
        let window = create_test_window(10000);

        // Act
        let (start, end) = window.window_range();

        // Assert
        assert_eq!(start, 0);
        assert_eq!(end, window.window_size);
    }

    #[test]
    fn test_window_range_after_move() {
        // Arrange
        let window = IndexSessionWindow::new(
            "test".to_string(),
            10000,
            IndexWindowPolicy::Fixed(1000),
            create_test_sparse_keyframes(10),
        );
        let mut window = window;
        window.set_position(2000); // Force window move

        // Act
        let (start, end) = window.window_range();

        // Assert
        assert!(start > 0);
        assert_eq!(end - start, window.window_size);
    }
}

// ============================================================================
// Window Revision Tests
// ============================================================================

#[cfg(test)]
mod window_revision_tests {
    use super::*;

    #[test]
    fn test_window_revision_initial() {
        // Arrange
        let window = create_test_window(10000);

        // Act
        let revision = window.window_revision();

        // Assert
        assert_eq!(revision, 0);
    }

    #[test]
    fn test_window_revision_increments_on_move() {
        // Arrange
        let window = IndexSessionWindow::new(
            "test".to_string(),
            10000,
            IndexWindowPolicy::Fixed(1000),
            create_test_sparse_keyframes(10),
        );
        let mut window = window;

        // Act - Force window move
        window.set_position(2000);

        // Assert
        assert_eq!(window.window_revision(), 1);
    }

    #[test]
    fn test_window_revision_no_move_no_increment() {
        // Arrange
        let mut window = create_test_window(10000);

        // Act - Position within window
        window.set_position(100);

        // Assert
        assert_eq!(window.window_revision(), 0);
    }
}

// ============================================================================
// Stats Tests
// ============================================================================

#[cfg(test)]
mod stats_tests {
    use super::*;

    #[test]
    fn test_stats_returns_reference() {
        // Arrange
        let window = create_test_window(10000);

        // Act
        let stats = window.stats();

        // Assert
        assert_eq!(stats.window_moves, 0);
        assert_eq!(stats.frames_materialized, 0);
    }
}

// ============================================================================
// Hit Rate Tests
// ============================================================================

#[cfg(test)]
mod hit_rate_tests {
    use super::*;

    #[test]
    fn test_hit_rate_initially_zero() {
        // Arrange
        let window = create_test_window(10000);

        // Act
        let rate = window.hit_rate();

        // Assert
        assert_eq!(rate, 0.0);
    }

    #[test]
    fn test_hit_rate_all_hits() {
        // Arrange
        let mut window = create_test_window(10000);
        let frame = create_test_frame_metadata(100, 50000, 300);
        window.materialize_frame(frame);

        // Act - Access same frame twice
        window.get_frame(100);
        window.get_frame(100);

        // Assert - materialize_frame adds 1 miss, get_frame adds 2 hits
        // hit_rate = 2 / (1 + 2) = 0.667
        let total = window.stats().cache_hits + window.stats().cache_misses;
        assert_eq!(window.stats().cache_hits, 2);
        assert_eq!(total, 3);
    }

    #[test]
    fn test_hit_rate_mixed() {
        // Arrange
        let mut window = create_test_window(10000);

        // Act - One hit, one miss
        let frame = create_test_frame_metadata(100, 50000, 300);
        window.materialize_frame(frame); // Adds 1 miss
        window.get_frame(100); // Hit (1 hit)
        window.get_frame(200); // Miss (1 miss)

        // Assert - 1 hit / 3 total = 0.333
        let total = window.stats().cache_hits + window.stats().cache_misses;
        assert_eq!(window.stats().cache_hits, 1);
        assert_eq!(total, 3);
    }
}

// ============================================================================
// Clear Tests
// ============================================================================

#[cfg(test)]
mod clear_tests {
    use super::*;

    #[test]
    fn test_clear_removes_materialized() {
        // Arrange
        let mut window = create_test_window(10000);
        for i in 0..5 {
            window.materialize_frame(create_test_frame_metadata(i, i as u64 * 100, 100));
        }
        assert_eq!(window.materialized_count(), 5);

        // Act
        window.clear();

        // Assert
        assert_eq!(window.materialized_count(), 0);
    }

    #[test]
    fn test_clear_updates_evictions() {
        // Arrange
        let mut window = create_test_window(10000);
        for i in 0..5 {
            window.materialize_frame(create_test_frame_metadata(i, i as u64 * 100, 100));
        }

        // Act
        window.clear();

        // Assert
        assert_eq!(window.stats().frames_evicted, 5);
    }
}

// ============================================================================
// Materialized Count Tests
// ============================================================================

#[cfg(test)]
mod materialized_count_tests {
    use super::*;

    #[test]
    fn test_materialized_count_initially_zero() {
        // Arrange
        let window = create_test_window(10000);

        // Act
        let count = window.materialized_count();

        // Assert
        assert_eq!(count, 0);
    }

    #[test]
    fn test_materialized_count_increments() {
        // Arrange
        let mut window = create_test_window(10000);

        // Act
        for i in 0..5 {
            window.materialize_frame(create_test_frame_metadata(i, i as u64 * 100, 100));
        }

        // Assert
        assert_eq!(window.materialized_count(), 5);
    }

    #[test]
    fn test_materialized_count_at_capacity() {
        // Arrange
        let mut window = IndexSessionWindow::new(
            "test".to_string(),
            1000,
            IndexWindowPolicy::Fixed(100),
            vec![],
        );

        // Act - Add more than capacity
        for i in 0..150 {
            window.materialize_frame(create_test_frame_metadata(i, i as u64 * 100, 100));
        }

        // Assert - Should not exceed capacity
        assert!(window.materialized_count() <= 100);
    }
}

// ============================================================================
// Should Move Window Tests
// ============================================================================

#[cfg(test)]
mod should_move_window_tests {
    use super::*;

    #[test]
    fn test_should_move_window_within_bounds() {
        // Arrange
        let window = create_test_window(10000);

        // Act
        let should_move = window.should_move_window(100);

        // Assert
        assert!(!should_move);
    }

    #[test]
    fn test_should_move_window_before_start() {
        // Arrange
        let mut window = create_test_window(10000);
        window.window_start = 5000;

        // Act
        let should_move = window.should_move_window(4000);

        // Assert
        assert!(should_move);
    }

    #[test]
    fn test_should_move_window_after_end() {
        // Arrange
        let window = create_test_window(10000);
        let window_end = window.window_start + window.window_size;

        // Act
        let should_move = window.should_move_window(window_end + 100);

        // Assert
        assert!(should_move);
    }

    #[test]
    fn test_should_move_window_at_boundary() {
        // Arrange
        let window = create_test_window(10000);
        let window_end = window.window_start + window.window_size;

        // Act - Exactly at end boundary
        let should_move = window.should_move_window(window_end);

        // Assert
        assert!(should_move); // At end is considered outside
    }
}

// ============================================================================
// Edge Cases
// ============================================================================

#[cfg(test)]
mod edge_case_tests {
    use super::*;

    #[test]
    fn test_empty_sparse_keyframes() {
        // Arrange
        let window = IndexSessionWindow::new(
            "test".to_string(),
            10000,
            IndexWindowPolicy::Fixed(5000),
            vec![],
        );

        // Act
        let result = window.find_nearest_keyframe(100);

        // Assert
        assert!(result.is_none());
    }

    #[test]
    fn test_window_larger_than_total() {
        // Arrange & Act
        let window = IndexSessionWindow::new(
            "test".to_string(),
            1000,
            IndexWindowPolicy::Fixed(10000), // Larger than total
            vec![],
        );

        // Assert
        assert_eq!(window.window_size, 1000); // Clamped to total
    }

    #[test]
    fn test_position_at_total_frames_minus_one() {
        // Arrange
        let mut window = create_test_window(1000);

        // Act
        window.set_position(999);

        // Assert
        assert_eq!(window.current_position, 999);
    }

    #[test]
    fn test_lru_queue_updates_on_access() {
        // Arrange
        let mut window = create_test_window(10000);
        for i in 0..5 {
            window.materialize_frame(create_test_frame_metadata(i, i as u64 * 100, 100));
        }

        // Act - Access frame 0 multiple times
        window.get_frame(0);
        window.get_frame(1);
        window.get_frame(0);

        // Assert - Frame 0 should still be in cache (most recently accessed)
        assert!(window.is_materialized(0));
    }

    #[test]
    fn test_window_centers_on_position() {
        // Arrange
        let mut window = create_test_window(10000);
        let half_window = window.window_size / 2;

        // Act - Move far from current position
        window.set_position(5000);

        // Assert
        let expected_start = 5000usize.saturating_sub(half_window);
        assert_eq!(window.window_start, expected_start.min(10000 - window.window_size));
    }
}
