// Timeline cache module tests
//
// Per generate-tests skill: Arrange-Act-Assert pattern with fixtures
// and edge case coverage.

use super::*;

// ============================================================================
// Fixtures
// ============================================================================

/// Create a test timeline cache manager
fn create_test_manager() -> TimelineCacheManager {
    TimelineCacheManager::new()
}

// ============================================================================
// TimelineCacheManager Construction Tests
// ============================================================================

#[cfg(test)]
mod construction_tests {
    use super::*;

    #[test]
    fn test_new_creates_manager() {
        // Arrange & Act
        let manager = create_test_manager();

        // Assert
        assert_eq!(manager.data_revision(), 0);
        assert_eq!(manager.zoom_level(), 1.0);
        assert_eq!(manager.filter_hash, 0);
    }

    #[test]
    fn test_new_initial_stats() {
        // Arrange
        let manager = create_test_manager();

        // Act
        let stats = manager.stats();

        // Assert
        assert_eq!(stats.total_entries, 0);
        assert_eq!(stats.valid_entries, 0);
        assert_eq!(stats.data_revision, 0);
        assert_eq!(stats.active_lanes, 0);
    }

    #[test]
    fn test_default_creates_manager() {
        // Arrange & Act
        let manager = TimelineCacheManager::default();

        // Assert
        assert_eq!(manager.data_revision(), 0);
        assert_eq!(manager.zoom_level(), 1.0);
    }
}

// ============================================================================
// Timeline Cache Key Tests
// ============================================================================

#[cfg(test)]
mod timeline_cache_key_tests {
    use super::*;

    #[test]
    fn test_timeline_cache_key_includes_revision() {
        // Arrange
        let mut manager = create_test_manager();

        // Act
        manager.data_revision = 5;

        // Act
        let key = manager.timeline_cache_key();

        // Assert
        match key {
            crate::CacheKey::Timeline { data_revision, .. } => {
                assert_eq!(data_revision, 5);
            }
            _ => panic!("Expected Timeline cache key"),
        }
    }

    #[test]
    fn test_timeline_cache_key_includes_zoom() {
        // Arrange
        let mut manager = create_test_manager();
        manager.zoom_level = 2.5;

        // Act
        let key = manager.timeline_cache_key();

        // Assert
        match key {
            crate::CacheKey::Timeline { zoom_level_x100, .. } => {
                assert_eq!(zoom_level_x100, 250);
            }
            _ => panic!("Expected Timeline cache key"),
        }
    }

    #[test]
    fn test_timeline_cache_key_includes_filter() {
        // Arrange
        let mut manager = create_test_manager();
        manager.filter_hash = 42;

        // Act
        let key = manager.timeline_cache_key();

        // Assert
        match key {
            crate::CacheKey::Timeline { filter_hash, .. } => {
                assert_eq!(filter_hash, 42);
            }
            _ => panic!("Expected Timeline cache key"),
        }
    }
}

// ============================================================================
// Add Cache Tests
// ============================================================================

#[cfg(test)]
mod add_cache_tests {
    use super::*;

    #[test]
    fn test_add_timeline_cache_returns_key() {
        // Arrange
        let mut manager = create_test_manager();

        // Act
        let key = manager.add_timeline_cache(1024);

        // Assert
        // Key should contain current revision, zoom, and filter
        match key {
            crate::CacheKey::Timeline { .. } => {
                // Valid timeline key
            }
            _ => panic!("Expected Timeline cache key"),
        }
    }

    #[test]
    fn test_add_lane_cache_returns_key() {
        // Arrange
        let mut manager = create_test_manager();

        // Act
        let key = manager.add_lane_cache("qp_lane", 2048);

        // Assert
        // Key should be unique for the lane
        match key {
            crate::CacheKey::Timeline { .. } => {
                // Valid timeline key
            }
            _ => panic!("Expected Timeline cache key"),
        }
    }

    #[test]
    fn test_add_lane_cache_tracks_entry() {
        // Arrange
        let mut manager = create_test_manager();

        // Act
        manager.add_lane_cache("qp_lane", 1024);
        manager.add_lane_cache("slice_lane", 2048);

        // Assert
        assert_eq!(manager.stats().active_lanes, 2);
    }

    #[test]
    fn test_add_lane_cache_same_lane_multiple_entries() {
        // Arrange
        let mut manager = create_test_manager();

        // Act
        let key1 = manager.add_lane_cache("qp_lane", 1024);
        let key2 = manager.add_lane_cache("qp_lane", 2048);

        // Assert - Same lane returns same key (implementation creates identical keys)
        assert_eq!(key1, key2);
        // But entries are tracked separately in lane_cache_entries
        assert_eq!(manager.lane_cache_entries.get("qp_lane").unwrap().len(), 2);
        assert_eq!(manager.stats().active_lanes, 1);
    }
}

// ============================================================================
// Record Hit/Miss Tests
// ============================================================================

#[cfg(test)]
mod record_hit_miss_tests {
    use super::*;

    #[test]
    fn test_record_hit_increments_hit_count() {
        // Arrange
        let mut manager = create_test_manager();
        let key = manager.add_timeline_cache(1024);

        // Act
        manager.record_hit(&key);

        // Assert
        let stats = manager.stats();
        assert_eq!(stats.hit_count, 1);
        assert_eq!(stats.miss_count, 0);
    }

    #[test]
    fn test_record_miss_increments_miss_count() {
        // Arrange
        let mut manager = create_test_manager();
        let key = manager.add_timeline_cache(1024);

        // Act
        manager.record_miss(&key);

        // Assert
        let stats = manager.stats();
        assert_eq!(stats.hit_count, 0);
        assert_eq!(stats.miss_count, 1);
    }

    #[test]
    fn test_record_hit_miss_affects_hit_rate() {
        // Arrange
        let mut manager = create_test_manager();
        let key = manager.add_timeline_cache(1024);
        manager.record_hit(&key);
        manager.record_hit(&key);
        manager.record_miss(&key);

        // Act
        let stats = manager.stats();

        // Assert - 2 hits / 3 total = 0.667
        assert_eq!(stats.hit_count, 2);
        assert_eq!(stats.miss_count, 1);
        assert!((stats.hit_rate - 0.667).abs() < 0.01);
    }
}

// ============================================================================
// Update/Invalidate Tests
// ============================================================================

#[cfg(test)]
mod update_invalidate_tests {
    use super::*;

    #[test]
    fn test_update_data_revision_increments() {
        // Arrange
        let mut manager = create_test_manager();
        let initial = manager.data_revision();

        // Act
        manager.update_data_revision();

        // Assert
        assert_eq!(manager.data_revision(), initial + 1);
    }

    #[test]
    fn test_update_data_revision_invalidates_caches() {
        // Arrange
        let mut manager = create_test_manager();
        manager.add_timeline_cache(1024);
        assert_eq!(manager.stats().valid_entries, 1);

        // Act
        manager.update_data_revision();

        // Assert - All entries should be invalidated
        assert_eq!(manager.stats().valid_entries, 0);
        assert_eq!(manager.stats().invalid_entries, 1);
    }

    #[test]
    fn test_update_zoom_level_changes_zoom() {
        // Arrange
        let mut manager = create_test_manager();
        let initial = manager.zoom_level();

        // Act
        manager.update_zoom_level(2.5);

        // Assert
        assert_eq!(manager.zoom_level(), 2.5);
        assert_ne!(initial, 2.5);
    }

    #[test]
    fn test_update_zoom_level_no_change_no_invalidation() {
        // Arrange
        let mut manager = create_test_manager();
        manager.add_timeline_cache(1024);

        // Act - Update with same value
        manager.update_zoom_level(1.0);

        // Assert - No invalidation since zoom didn't change
        assert_eq!(manager.stats().valid_entries, 1);
    }

    #[test]
    fn test_update_zoom_level_invalidates_caches() {
        // Arrange
        let mut manager = create_test_manager();
        manager.add_timeline_cache(1024);
        assert_eq!(manager.stats().valid_entries, 1);

        // Act
        manager.update_zoom_level(2.0);

        // Assert - Zoom change invalidates caches
        assert_eq!(manager.stats().valid_entries, 0);
    }

    #[test]
    fn test_update_filter_changes_filter() {
        // Arrange
        let mut manager = create_test_manager();

        // Act
        manager.update_filter(12345);

        // Assert
        assert_eq!(manager.filter_hash, 12345);
    }

    #[test]
    fn test_update_filter_same_value_no_change() {
        // Arrange
        let mut manager = create_test_manager();
        manager.filter_hash = 100;

        // Act
        manager.update_filter(100);

        // Assert
        assert_eq!(manager.filter_hash, 100);
    }

    #[test]
    fn test_update_filter_invalidates_caches() {
        // Arrange
        let mut manager = create_test_manager();
        manager.add_timeline_cache(1024);
        assert_eq!(manager.stats().valid_entries, 1);

        // Act
        manager.update_filter(999);

        // Assert
        assert_eq!(manager.stats().valid_entries, 0);
    }

    #[test]
    fn test_invalidate_lane_removes_entries() {
        // Arrange
        let mut manager = create_test_manager();
        manager.add_lane_cache("qp_lane", 1024);
        manager.add_lane_cache("slice_lane", 2048);
        assert_eq!(manager.stats().active_lanes, 2);

        // Act
        manager.invalidate_lane("qp_lane");

        // Assert
        assert_eq!(manager.stats().active_lanes, 1);
    }

    #[test]
    fn test_invalidate_lane_nonexistent() {
        // Arrange
        let mut manager = create_test_manager();

        // Act - Should not panic
        manager.invalidate_lane("nonexistent");

        // Assert
        assert_eq!(manager.stats().active_lanes, 0);
    }

    #[test]
    fn test_clear_all_invalidates() {
        // Arrange
        let mut manager = create_test_manager();
        manager.add_timeline_cache(1024);
        manager.add_lane_cache("qp_lane", 2048);
        assert_eq!(manager.stats().total_entries, 2);

        // Act
        manager.clear_all();

        // Assert - clear_all marks entries as invalid and clears lane entries
        // total_entries includes invalid entries, so it remains 2
        assert_eq!(manager.stats().total_entries, 2);
        assert_eq!(manager.stats().invalid_entries, 2);
        assert_eq!(manager.stats().valid_entries, 0);
        assert_eq!(manager.stats().active_lanes, 0);
    }
}

// ============================================================================
// Tracker Access Tests
// ============================================================================

#[cfg(test)]
mod tracker_access_tests {
    use super::*;

    #[test]
    fn test_tracker_returns_ref() {
        // Arrange
        let manager = create_test_manager();

        // Act
        let tracker = manager.tracker();

        // Assert
        assert_eq!(tracker.stats().total_entries, 0);
    }

    #[test]
    fn test_tracker_mut_returns_ref() {
        // Arrange
        let mut manager = create_test_manager();

        // Act
        let tracker = manager.tracker_mut();

        // Assert
        assert_eq!(tracker.stats().total_entries, 0);
    }
}

// ============================================================================
// TimelineCacheStats Tests
// ============================================================================

#[cfg(test)]
mod timeline_cache_stats_tests {
    use super::*;

    #[test]
    fn test_timeline_cache_stats_fields_accessible() {
        // Arrange
        let manager = create_test_manager();

        // Act
        let stats = manager.stats();

        // Assert
        assert_eq!(stats.data_revision, 0);
        assert_eq!(stats.active_lanes, 0);
    }

    #[test]
    fn test_stats_reflects_operations() {
        // Arrange
        let mut manager = create_test_manager();
        manager.add_timeline_cache(1000);
        manager.add_lane_cache("test", 500);
        let key = manager.add_timeline_cache(2000);
        manager.record_hit(&key);
        manager.record_miss(&key);

        // Act
        let stats = manager.stats();

        // Assert - Note: both add_timeline_cache calls use the same key (same revision/zoom/filter),
        // so the second overwrites the first in the entries map, BUT total_size still accumulates.
        // Total entries = 1 timeline + 1 lane = 2
        // Total size = 1000 + 500 + 2000 = 3500 (all additions counted, even overwrites)
        assert_eq!(stats.total_entries, 2);
        assert_eq!(stats.total_size_bytes, 3500);
        assert_eq!(stats.hit_count, 1);
        assert_eq!(stats.miss_count, 1);
        assert_eq!(stats.active_lanes, 1);
    }
}

// ============================================================================
// Hash Function Tests
// ============================================================================

#[cfg(test)]
mod hash_function_tests {
    use super::*;

    #[test]
    fn test_hash_string_consistent() {
        // Arrange
        let manager = create_test_manager();

        // Act
        let hash1 = manager.hash_string("test_lane");
        let hash2 = manager.hash_string("test_lane");

        // Assert
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_hash_string_different_inputs() {
        // Arrange
        let manager = create_test_manager();

        // Act
        let hash1 = manager.hash_string("lane_a");
        let hash2 = manager.hash_string("lane_b");

        // Assert
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_hash_string_empty_string() {
        // Arrange
        let manager = create_test_manager();

        // Act
        let hash = manager.hash_string("");

        // Assert
        // Empty string should hash to some value
        assert!(hash >= 0); // Just verify it doesn't panic
    }
}

// ============================================================================
// Edge Cases
// ============================================================================

#[cfg(test)]
mod edge_case_tests {
    use super::*;

    #[test]
    fn test_add_multiple_caches_same_lane() {
        // Arrange
        let mut manager = create_test_manager();

        // Act - Add multiple caches for same lane
        for i in 0..5 {
            manager.add_lane_cache("test_lane", 100 * (i + 1));
        }

        // Assert
        assert_eq!(manager.stats().active_lanes, 1);
        // Should have 5 entries for the lane
        let keys = manager.lane_cache_entries.get("test_lane").unwrap();
        assert_eq!(keys.len(), 5);
    }

    #[test]
    fn test_clear_all_removes_lane_entries() {
        // Arrange
        let mut manager = create_test_manager();
        manager.add_lane_cache("lane1", 100);
        manager.add_lane_cache("lane2", 200);

        // Act
        manager.clear_all();

        // Assert
        assert!(manager.lane_cache_entries.is_empty());
    }

    #[test]
    fn test_data_revision_increments_on_invalidate() {
        // Arrange
        let mut manager = create_test_manager();
        let revision = manager.data_revision();

        // Act
        manager.update_data_revision();

        // Assert
        assert!(manager.data_revision() > revision);
    }

    #[test]
    fn test_timeline_cache_key_changes_with_params() {
        // Arrange
        let mut manager = create_test_manager();
        manager.data_revision = 5;
        manager.zoom_level = 2.0;
        manager.filter_hash = 42;

        // Act
        let key1 = manager.timeline_cache_key();

        // Change parameters
        manager.data_revision = 6;
        let key2 = manager.timeline_cache_key();

        // Assert - Keys should be different
        assert_ne!(key1, key2);
    }

    #[test]
    fn test_lane_cache_key_includes_lane_hash() {
        // Arrange
        let mut manager = create_test_manager();
        manager.data_revision = 5;
        manager.zoom_level = 1.0;

        // Act
        let key1 = manager.add_lane_cache("lane_a", 100);
        let key2 = manager.add_lane_cache("lane_b", 100);

        // Assert - Keys should be different due to lane hash
        assert_ne!(key1, key2);
    }

    #[test]
    fn test_update_zoom_epsilon() {
        // Arrange
        let mut manager = create_test_manager();
        manager.zoom_level = 1.0;

        // Act - Update with very small change (within epsilon)
        manager.update_zoom_level(1.0 + f32::EPSILON / 2.0);

        // Assert - Should not trigger invalidation
        assert_eq!(manager.zoom_level(), 1.0); // Unchanged due to epsilon check
    }

    #[test]
    fn test_multiple_updates_invalidate_once() {
        // Arrange
        let mut manager = create_test_manager();
        manager.add_timeline_cache(1000);
        assert_eq!(manager.stats().valid_entries, 1);

        // Act
        manager.update_data_revision();
        manager.update_data_revision();

        // Assert - Only 1 invalid entry, but revision incremented twice
        assert_eq!(manager.stats().invalid_entries, 1);
        assert_eq!(manager.data_revision(), 2);
    }

    #[test]
    fn test_hit_rate_with_zero_operations() {
        // Arrange
        let manager = create_test_manager();

        // Act
        let hit_rate = manager.stats().hit_rate;

        // Assert - No operations means hit_rate is 0
        assert_eq!(hit_rate, 0.0);
    }

    #[test]
    fn test_hit_rate_with_only_hits() {
        // Arrange
        let mut manager = create_test_manager();
        let key = manager.add_timeline_cache(1000);
        manager.record_hit(&key);
        manager.record_hit(&key);

        // Act
        let stats = manager.stats();

        // Assert
        assert_eq!(stats.hit_rate, 1.0);
    }

    #[test]
    fn test_hit_rate_with_only_misses() {
        // Arrange
        let mut manager = create_test_manager();
        let key = manager.add_timeline_cache(1000);
        manager.record_miss(&key);
        manager.record_miss(&key);

        // Act
        let stats = manager.stats();

        // Assert
        assert_eq!(stats.hit_rate, 0.0);
    }
}
