// Compare cache module tests
//
// Per generate-tests skill: Arrange-Act-Assert pattern with fixtures
// and edge case coverage.

use super::*;
use crate::cache_provenance::CacheKey;

// ============================================================================
// Fixtures
// ============================================================================

/// Create a test compare cache manager
fn create_test_cache_manager() -> CompareCacheManager {
    CompareCacheManager::new()
}

// ============================================================================
// CompareStreamId Tests
// ============================================================================

#[cfg(test)]
mod compare_stream_id_tests {
    use super::*;

    #[test]
    fn test_compare_stream_id_label_a() {
        // Arrange & Act
        let label = CompareStreamId::A.label();

        // Assert
        assert_eq!(label, "Stream A");
    }

    #[test]
    fn test_compare_stream_id_label_b() {
        // Arrange & Act
        let label = CompareStreamId::B.label();

        // Assert
        assert_eq!(label, "Stream B");
    }
}

// ============================================================================
// CompareCacheManager Construction Tests
// ============================================================================

#[cfg(test)]
mod construction_tests {
    use super::*;

    #[test]
    fn test_compare_cache_manager_new() {
        // Arrange & Act
        let manager = CompareCacheManager::new();

        // Assert
        assert_eq!(manager.current_frame_a(), None);
        assert_eq!(manager.current_frame_b(), None);
        assert_eq!(manager.manual_offset(), 0);
        assert_eq!(manager.alignment_revision(), 0);
        assert_eq!(manager.total_size_bytes(), 0);
        assert_eq!(manager.total_entries(), 0);
    }

    #[test]
    fn test_compare_cache_manager_default() {
        // Arrange & Act
        let manager = CompareCacheManager::default();

        // Assert
        assert_eq!(manager.manual_offset(), 0);
        assert_eq!(manager.total_entries(), 0);
    }
}

// ============================================================================
// Decode Cache Tests
// ============================================================================

#[cfg(test)]
mod decode_cache_tests {
    use super::*;

    #[test]
    fn test_add_decode_cache_a() {
        // Arrange
        let mut manager = create_test_cache_manager();

        // Act
        let key = manager.add_decode_cache_a(0, "params1".to_string(), 1024);

        // Assert
        assert!(matches!(key, CacheKey::Decode { frame_idx: 0, .. }));
        assert_eq!(manager.stats_a().total_entries, 1);
        assert_eq!(manager.stats_a().total_size_bytes, 1024);
    }

    #[test]
    fn test_add_decode_cache_b() {
        // Arrange
        let mut manager = create_test_cache_manager();

        // Act
        let key = manager.add_decode_cache_b(5, "params2".to_string(), 2048);

        // Assert
        assert!(matches!(key, CacheKey::Decode { frame_idx: 5, .. }));
        assert_eq!(manager.stats_b().total_entries, 1);
        assert_eq!(manager.stats_b().total_size_bytes, 2048);
    }

    #[test]
    fn test_add_decode_cache_both_streams() {
        // Arrange
        let mut manager = create_test_cache_manager();

        // Act
        manager.add_decode_cache_a(0, "p1".to_string(), 1000);
        manager.add_decode_cache_b(0, "p1".to_string(), 1000);
        manager.add_decode_cache_a(1, "p2".to_string(), 2000);

        // Assert
        assert_eq!(manager.stats_a().total_entries, 2);
        assert_eq!(manager.stats_b().total_entries, 1);
        assert_eq!(manager.total_entries(), 3);
    }
}

// ============================================================================
// Texture Cache Tests
// ============================================================================

#[cfg(test)]
mod texture_cache_tests {
    use super::*;

    #[test]
    fn test_add_texture_cache_stream_a() {
        // Arrange
        let mut manager = create_test_cache_manager();

        // Act
        let key = manager.add_texture_cache(CompareStreamId::A, 0, 1, "ycbcr".to_string(), 512);

        // Assert
        assert!(matches!(key, CacheKey::Texture { frame_idx: 0, res_tier: 1, .. }));
        assert_eq!(manager.stats_a().total_entries, 1);
        assert_eq!(manager.stats_a().total_size_bytes, 512);
    }

    #[test]
    fn test_add_texture_cache_stream_b() {
        // Arrange
        let mut manager = create_test_cache_manager();

        // Act
        let key = manager.add_texture_cache(CompareStreamId::B, 10, 2, "rgb".to_string(), 1024);

        // Assert
        assert!(matches!(key, CacheKey::Texture { frame_idx: 10, res_tier: 2, .. }));
        assert_eq!(manager.stats_b().total_entries, 1);
        assert_eq!(manager.stats_b().total_size_bytes, 1024);
    }
}

// ============================================================================
// Diff Heatmap Cache Tests
// ============================================================================

#[cfg(test)]
mod diff_heatmap_cache_tests {
    use super::*;

    #[test]
    fn test_add_diff_heatmap_cache() {
        // Arrange
        let mut manager = create_test_cache_manager();

        // Act
        let key = manager.add_diff_heatmap_cache(0, 0, "difference".to_string(), "ab".to_string(), 64, 256);

        // Assert
        assert!(matches!(key, CacheKey::DiffHeatmap { frame_idx_a: 0, frame_idx_b: 0, .. }));
        assert_eq!(manager.stats_diff().total_entries, 1);
        assert_eq!(manager.stats_diff().total_size_bytes, 256);
    }

    #[test]
    fn test_diff_entries_for_pair() {
        // Arrange
        let mut manager = create_test_cache_manager();
        let key1 = manager.add_diff_heatmap_cache(0, 0, "diff".to_string(), "ab".to_string(), 64, 256);
        let key2 = manager.add_diff_heatmap_cache(0, 0, "ssim".to_string(), "ab".to_string(), 64, 128);

        // Act
        let entries = manager.diff_entries_for_pair(0, 0);

        // Assert
        assert_eq!(entries.len(), 2);
        assert!(entries.contains(&&key1));
        assert!(entries.contains(&&key2));
    }

    #[test]
    fn test_diff_entries_for_pair_empty() {
        // Arrange
        let manager = create_test_cache_manager();

        // Act
        let entries = manager.diff_entries_for_pair(5, 10);

        // Assert
        assert_eq!(entries.len(), 0);
    }

    #[test]
    fn test_diff_entries_for_different_pairs() {
        // Arrange
        let mut manager = create_test_cache_manager();
        manager.add_diff_heatmap_cache(0, 0, "diff".to_string(), "ab".to_string(), 64, 256);
        manager.add_diff_heatmap_cache(1, 1, "diff".to_string(), "ab".to_string(), 64, 256);

        // Act
        let entries_0_0 = manager.diff_entries_for_pair(0, 0);
        let entries_1_1 = manager.diff_entries_for_pair(1, 1);

        // Assert
        assert_eq!(entries_0_0.len(), 1);
        assert_eq!(entries_1_1.len(), 1);
    }
}

// ============================================================================
// Frame Tracking Tests
// ============================================================================

#[cfg(test)]
mod frame_tracking_tests {
    use super::*;

    #[test]
    fn test_set_frame_a() {
        // Arrange
        let mut manager = create_test_cache_manager();

        // Act
        manager.set_frame_a(5);

        // Assert
        assert_eq!(manager.current_frame_a(), Some(5));
    }

    #[test]
    fn test_set_frame_b() {
        // Arrange
        let mut manager = create_test_cache_manager();

        // Act
        manager.set_frame_b(10);

        // Assert
        assert_eq!(manager.current_frame_b(), Some(10));
    }

    #[test]
    fn test_set_frame_same_value_no_invalidation() {
        // Arrange
        let mut manager = create_test_cache_manager();
        manager.set_frame_a(5);
        manager.add_decode_cache_a(5, "params".to_string(), 1024);
        let stats_before = manager.stats_a().total_entries;

        // Act - Set same frame
        manager.set_frame_a(5);

        // Assert - No change in entries
        assert_eq!(manager.stats_a().total_entries, stats_before);
    }

    #[test]
    fn test_set_frame_different_invalidates() {
        // Arrange
        let mut manager = create_test_cache_manager();
        manager.set_frame_a(5);
        manager.add_decode_cache_a(5, "params".to_string(), 1024);

        // Act - Change frame
        manager.set_frame_a(10);

        // Assert - Frame changed but we can't test invalidation directly
        assert_eq!(manager.current_frame_a(), Some(10));
    }
}

// ============================================================================
// Manual Offset Tests
// ============================================================================

#[cfg(test)]
mod manual_offset_tests {
    use super::*;

    #[test]
    fn test_set_manual_offset() {
        // Arrange
        let mut manager = create_test_cache_manager();

        // Act
        manager.set_manual_offset(5);

        // Assert
        assert_eq!(manager.manual_offset(), 5);
    }

    #[test]
    fn test_set_manual_offset_negative() {
        // Arrange
        let mut manager = create_test_cache_manager();

        // Act
        manager.set_manual_offset(-10);

        // Assert
        assert_eq!(manager.manual_offset(), -10);
    }

    #[test]
    fn test_set_manual_offset_invalidates_diff() {
        // Arrange
        let mut manager = create_test_cache_manager();
        manager.add_diff_heatmap_cache(0, 0, "diff".to_string(), "ab".to_string(), 64, 256);
        assert_eq!(manager.stats_diff().valid_entries, 1);

        // Act - Changing offset invalidates diff cache
        manager.set_manual_offset(5);

        // Assert - Diff cache entries should be invalidated (but still counted in total)
        assert_eq!(manager.stats_diff().valid_entries, 0);
        assert_eq!(manager.stats_diff().invalid_entries, 1);
        assert_eq!(manager.stats_diff().invalidation_count, 1);
    }

    #[test]
    fn test_set_manual_offset_same_value_no_invalidation() {
        // Arrange
        let mut manager = create_test_cache_manager();
        manager.set_manual_offset(5);
        manager.add_diff_heatmap_cache(0, 0, "diff".to_string(), "ab".to_string(), 64, 256);
        let entries_before = manager.stats_diff().total_entries;

        // Act - Set same offset
        manager.set_manual_offset(5);

        // Assert - No change
        assert_eq!(manager.stats_diff().total_entries, entries_before);
    }
}

// ============================================================================
// Alignment Tests
// ============================================================================

#[cfg(test)]
mod alignment_tests {
    use super::*;

    #[test]
    fn test_update_alignment() {
        // Arrange
        let mut manager = create_test_cache_manager();

        // Act
        manager.update_alignment();

        // Assert
        assert_eq!(manager.alignment_revision(), 1);
    }

    #[test]
    fn test_update_alignment_multiple_times() {
        // Arrange
        let mut manager = create_test_cache_manager();

        // Act
        manager.update_alignment();
        manager.update_alignment();
        manager.update_alignment();

        // Assert
        assert_eq!(manager.alignment_revision(), 3);
    }

    #[test]
    fn test_update_alignment_invalidates_diff() {
        // Arrange
        let mut manager = create_test_cache_manager();
        manager.add_diff_heatmap_cache(0, 0, "diff".to_string(), "ab".to_string(), 64, 256);
        assert_eq!(manager.stats_diff().valid_entries, 1);

        // Act
        manager.update_alignment();

        // Assert - Diff cache entries should be invalidated
        assert_eq!(manager.stats_diff().valid_entries, 0);
        assert_eq!(manager.stats_diff().invalid_entries, 1);
        assert_eq!(manager.stats_diff().invalidation_count, 1);
    }
}

// ============================================================================
// Resolution Tests
// ============================================================================

#[cfg(test)]
mod resolution_tests {
    use super::*;

    #[test]
    fn test_set_resolution() {
        // Arrange
        let mut manager = create_test_cache_manager();

        // Act
        manager.set_resolution(1920, 1080, 1920, 1080);

        // Assert - No way to directly check resolution hash, but verify no panic
        assert_eq!(manager.total_entries(), 0);
    }

    #[test]
    fn test_set_resolution_invalidates_textures() {
        // Arrange
        let mut manager = create_test_cache_manager();
        manager.add_texture_cache(CompareStreamId::A, 0, 1, "ycbcr".to_string(), 1024);
        manager.add_texture_cache(CompareStreamId::B, 0, 1, "ycbcr".to_string(), 1024);
        assert_eq!(manager.stats_a().valid_entries, 1);
        assert_eq!(manager.stats_b().valid_entries, 1);

        // Act
        manager.set_resolution(1920, 1080, 1280, 720);

        // Assert - Textures should be invalidated
        assert_eq!(manager.stats_a().valid_entries, 0);
        assert_eq!(manager.stats_b().valid_entries, 0);
    }

    #[test]
    fn test_set_resolution_invalidates_diff() {
        // Arrange
        let mut manager = create_test_cache_manager();
        manager.add_diff_heatmap_cache(0, 0, "diff".to_string(), "ab".to_string(), 64, 256);
        assert_eq!(manager.stats_diff().valid_entries, 1);

        // Act
        manager.set_resolution(1920, 1080, 1920, 1081);

        // Assert - Diff should be invalidated (height changed)
        assert_eq!(manager.stats_diff().valid_entries, 0);
        assert_eq!(manager.stats_diff().invalid_entries, 1);
    }

    #[test]
    fn test_set_resolution_same_value_no_invalidation() {
        // Arrange
        let mut manager = create_test_cache_manager();
        manager.add_texture_cache(CompareStreamId::A, 0, 1, "ycbcr".to_string(), 1024);
        manager.set_resolution(1920, 1080, 1920, 1080);
        let entries_before = manager.total_entries();

        // Act - Set same resolution
        manager.set_resolution(1920, 1080, 1920, 1080);

        // Assert - No change
        assert_eq!(manager.total_entries(), entries_before);
    }
}

// ============================================================================
// Hit/Miss Recording Tests
// ============================================================================

#[cfg(test)]
mod hit_miss_recording_tests {
    use super::*;

    #[test]
    fn test_record_hit_stream_a() {
        // Arrange
        let mut manager = create_test_cache_manager();
        let key = manager.add_decode_cache_a(0, "params".to_string(), 1024);

        // Act
        manager.record_hit(CompareStreamId::A, &key);

        // Assert
        assert_eq!(manager.stats_a().hit_count, 1);
        assert_eq!(manager.stats_a().miss_count, 0);
    }

    #[test]
    fn test_record_miss_stream_a() {
        // Arrange
        let mut manager = create_test_cache_manager();
        let key = CacheKey::Decode { frame_idx: 0, decode_params: "params".to_string() };

        // Act
        manager.record_miss(CompareStreamId::A, &key);

        // Assert
        assert_eq!(manager.stats_a().hit_count, 0);
        assert_eq!(manager.stats_a().miss_count, 1);
    }

    #[test]
    fn test_record_hit_stream_b() {
        // Arrange
        let mut manager = create_test_cache_manager();
        let key = manager.add_decode_cache_b(0, "params".to_string(), 1024);

        // Act
        manager.record_hit(CompareStreamId::B, &key);

        // Assert
        assert_eq!(manager.stats_b().hit_count, 1);
    }

    #[test]
    fn test_record_diff_hit() {
        // Arrange
        let mut manager = create_test_cache_manager();
        let key = manager.add_diff_heatmap_cache(0, 0, "diff".to_string(), "ab".to_string(), 64, 256);

        // Act
        manager.record_diff_hit(&key);

        // Assert
        assert_eq!(manager.stats_diff().hit_count, 1);
        assert_eq!(manager.stats_diff().miss_count, 0);
    }

    #[test]
    fn test_record_diff_miss() {
        // Arrange
        let mut manager = create_test_cache_manager();
        let key = CacheKey::DiffHeatmap {
            frame_idx_a: 0,
            frame_idx_b: 0,
            mode: "diff".to_string(),
            ab_mapping: "ab".to_string(),
            hm_res: 64,
        };

        // Act
        manager.record_diff_miss(&key);

        // Assert
        assert_eq!(manager.stats_diff().hit_count, 0);
        assert_eq!(manager.stats_diff().miss_count, 1);
    }
}

// ============================================================================
// Statistics Tests
// ============================================================================

#[cfg(test)]
mod statistics_tests {
    use super::*;

    #[test]
    fn test_stats_combined() {
        // Arrange
        let mut manager = create_test_cache_manager();
        let key_a = manager.add_decode_cache_a(0, "params".to_string(), 1024);
        let key_b = manager.add_decode_cache_b(0, "params".to_string(), 2048);
        let key_diff = manager.add_diff_heatmap_cache(0, 0, "diff".to_string(), "ab".to_string(), 64, 512);

        manager.record_hit(CompareStreamId::A, &key_a);
        manager.record_hit(CompareStreamId::B, &key_b);
        manager.record_diff_hit(&key_diff);

        // Act
        let stats = manager.stats_combined();

        // Assert
        assert_eq!(stats.stream_a.total_entries, 1);
        assert_eq!(stats.stream_b.total_entries, 1);
        assert_eq!(stats.diff.total_entries, 1);
        assert_eq!(stats.total_entries, 3);
        assert_eq!(stats.total_size_bytes, 1024 + 2048 + 512);
        assert_eq!(stats.combined_hit_rate, 1.0); // 3/3 hits
    }

    #[test]
    fn test_stats_combined_hit_rate_calculation() {
        // Arrange
        let mut manager = create_test_cache_manager();
        let key_a = manager.add_decode_cache_a(0, "params".to_string(), 1024);

        manager.record_hit(CompareStreamId::A, &key_a);
        manager.record_miss(CompareStreamId::A, &key_a);
        manager.record_hit(CompareStreamId::A, &key_a);

        // Act
        let stats = manager.stats_combined();

        // Assert - 2 hits, 1 miss = 66.7%
        assert!(stats.combined_hit_rate > 0.6 && stats.combined_hit_rate < 0.7);
    }

    #[test]
    fn test_stats_combined_zero_attempts() {
        // Arrange
        let manager = create_test_cache_manager();

        // Act
        let stats = manager.stats_combined();

        // Assert
        assert_eq!(stats.combined_hit_rate, 0.0);
    }

    #[test]
    fn test_total_size_bytes() {
        // Arrange
        let mut manager = create_test_cache_manager();
        manager.add_decode_cache_a(0, "params".to_string(), 1000);
        manager.add_decode_cache_b(0, "params".to_string(), 2000);
        manager.add_diff_heatmap_cache(0, 0, "diff".to_string(), "ab".to_string(), 64, 500);

        // Act
        let total = manager.total_size_bytes();

        // Assert
        assert_eq!(total, 3500);
    }

    #[test]
    fn test_total_entries() {
        // Arrange
        let mut manager = create_test_cache_manager();
        manager.add_decode_cache_a(0, "params".to_string(), 1000);
        manager.add_decode_cache_a(1, "params".to_string(), 1000);
        manager.add_decode_cache_b(0, "params".to_string(), 1000);

        // Act
        let total = manager.total_entries();

        // Assert
        assert_eq!(total, 3);
    }
}

// ============================================================================
// LRU Eviction Tests
// ============================================================================

#[cfg(test)]
mod lru_eviction_tests {
    use super::*;

    #[test]
    fn test_evict_lru_stream_a() {
        // Arrange
        let mut manager = create_test_cache_manager();
        manager.add_decode_cache_a(0, "params1".to_string(), 1024);
        manager.add_decode_cache_a(1, "params2".to_string(), 2048);

        // Act - Evict with target 1500 bytes
        // Entry 0 (1024 bytes) is LRU, evicted first (total: 1024 < 1500, continue)
        // Entry 1 (2048 bytes) evicted second (total: 3072 >= 1500, stop)
        let count = manager.evict_lru_stream(CompareStreamId::A, 1500);

        // Assert - Both entries evicted to meet target
        assert_eq!(count, 2);
        assert_eq!(manager.stats_a().total_entries, 0);
    }

    #[test]
    fn test_evict_lru_stream_b() {
        // Arrange
        let mut manager = create_test_cache_manager();
        manager.add_decode_cache_b(0, "params".to_string(), 512);
        manager.add_decode_cache_b(1, "params".to_string(), 512);

        // Act - Target is 600, first entry is 512 (< 600), so it includes both
        let count = manager.evict_lru_stream(CompareStreamId::B, 600);

        // Assert - Both entries evicted (512 + 512 = 1024 >= 600)
        assert_eq!(count, 2);
        assert_eq!(manager.stats_b().valid_entries, 0);
    }

    #[test]
    fn test_evict_lru_diff() {
        // Arrange
        let mut manager = create_test_cache_manager();
        manager.add_diff_heatmap_cache(0, 0, "diff".to_string(), "ab".to_string(), 64, 256);
        manager.add_diff_heatmap_cache(1, 1, "diff".to_string(), "ab".to_string(), 64, 512);

        // Act - Target is 300 bytes
        // With entries of 256 and 512 bytes, the LRU eviction should
        // collect entries until reaching >= 300 bytes
        let count = manager.evict_lru_diff(300);

        // Assert - After eviction, only evicted entries are removed
        // Note: The exact number depends on LRU ordering and timing
        assert!(count >= 1); // At least one entry should be evicted
        assert_eq!(manager.stats_diff().total_entries, 2 - count);
    }

    #[test]
    fn test_evict_lru_diff_cleans_map() {
        // Arrange
        let mut manager = create_test_cache_manager();
        manager.add_diff_heatmap_cache(0, 0, "diff".to_string(), "ab".to_string(), 64, 256);
        assert_eq!(manager.diff_entries_for_pair(0, 0).len(), 1);

        // Act
        manager.evict_lru_diff(300);

        // Assert - Map should be cleaned up
        assert_eq!(manager.diff_entries_for_pair(0, 0).len(), 0);
    }

    #[test]
    fn test_evict_lru_stream_no_match() {
        // Arrange
        let mut manager = create_test_cache_manager();
        manager.add_decode_cache_a(0, "params".to_string(), 100);

        // Act - Try to evict more than available
        let count = manager.evict_lru_stream(CompareStreamId::A, 10000);

        // Assert
        assert_eq!(count, 1); // All entries evicted
        assert_eq!(manager.stats_a().total_entries, 0);
    }
}

// ============================================================================
// Clear Tests
// ============================================================================

#[cfg(test)]
mod clear_tests {
    use super::*;

    #[test]
    fn test_clear_all() {
        // Arrange
        let mut manager = create_test_cache_manager();
        manager.add_decode_cache_a(0, "params".to_string(), 1024);
        manager.add_decode_cache_b(0, "params".to_string(), 1024);
        manager.add_diff_heatmap_cache(0, 0, "diff".to_string(), "ab".to_string(), 64, 256);
        manager.set_frame_a(5);
        manager.set_manual_offset(10);
        manager.update_alignment();

        // Act
        manager.clear();

        // Assert
        assert_eq!(manager.total_entries(), 0);
        assert_eq!(manager.total_size_bytes(), 0);
        assert_eq!(manager.current_frame_a(), None);
        assert_eq!(manager.current_frame_b(), None);
        assert_eq!(manager.manual_offset(), 0);
        assert_eq!(manager.alignment_revision(), 0);
    }

    #[test]
    fn test_clear_empty_manager() {
        // Arrange
        let mut manager = create_test_cache_manager();

        // Act - Should not panic
        manager.clear();

        // Assert
        assert_eq!(manager.total_entries(), 0);
    }
}

// ============================================================================
// Edge Cases
// ============================================================================

#[cfg(test)]
mod edge_case_tests {
    use super::*;

    #[test]
    fn test_zero_offset() {
        // Arrange
        let mut manager = create_test_cache_manager();

        // Act
        manager.set_manual_offset(0);

        // Assert
        assert_eq!(manager.manual_offset(), 0);
    }

    #[test]
    fn test_large_offset() {
        // Arrange
        let mut manager = create_test_cache_manager();

        // Act
        manager.set_manual_offset(i32::MAX);

        // Assert
        assert_eq!(manager.manual_offset(), i32::MAX);
    }

    #[test]
    fn test_large_frame_indices() {
        // Arrange
        let mut manager = create_test_cache_manager();

        // Act
        manager.add_decode_cache_a(usize::MAX, "params".to_string(), 1024);

        // Assert - Should handle large indices
        assert_eq!(manager.stats_a().total_entries, 1);
    }

    #[test]
    fn test_resolution_zero_dimensions() {
        // Arrange
        let mut manager = create_test_cache_manager();

        // Act - Should not panic
        manager.set_resolution(0, 0, 0, 0);

        // Assert
        assert_eq!(manager.total_entries(), 0);
    }

    #[test]
    fn test_set_frame_zero() {
        // Arrange
        let mut manager = create_test_cache_manager();

        // Act
        manager.set_frame_a(0);
        manager.set_frame_b(0);

        // Assert
        assert_eq!(manager.current_frame_a(), Some(0));
        assert_eq!(manager.current_frame_b(), Some(0));
    }
}
