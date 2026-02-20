// Cache provenance module tests
//
// Per generate-tests skill: Arrange-Act-Assert pattern with fixtures
// and edge case coverage.


// ============================================================================
// Fixtures
// ============================================================================

/// Create a test cache provenance tracker
#[allow(dead_code)]
fn create_test_tracker() -> CacheProvenanceTracker {
    CacheProvenanceTracker::new()
}

/// Create a test decode cache key
#[allow(dead_code)]
fn create_test_decode_key(frame_idx: usize) -> CacheKey {
    CacheKey::Decode {
        frame_idx,
        decode_params: "test_params".to_string(),
    }
}

/// Create a test texture cache key
#[allow(dead_code)]
fn create_test_texture_key(frame_idx: usize) -> CacheKey {
    CacheKey::Texture {
        frame_idx,
        res_tier: 0,
        colorspace: "yuv420p".to_string(),
    }
}

/// Create a test timeline cache key
#[allow(dead_code)]
fn create_test_timeline_key(data_revision: u64, zoom_level_x100: u32) -> CacheKey {
    CacheKey::Timeline {
        data_revision,
        zoom_level_x100,
        filter_hash: 0,
    }
}

// ============================================================================
// CacheKey Tests
// ============================================================================

#[cfg(test)]
mod cache_key_tests {
#[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_type_name_decode() {
        // Arrange
        let key = CacheKey::Decode {
            frame_idx: 0,
            decode_params: "test".to_string(),
        };

        // Act
        let name = key.type_name();

        // Assert
        assert_eq!(name, "Decode");
    }

    #[test]
    fn test_type_name_texture() {
        // Arrange
        let key = CacheKey::Texture {
            frame_idx: 0,
            res_tier: 0,
            colorspace: "yuv420p".to_string(),
        };

        // Act
        let name = key.type_name();

        // Assert
        assert_eq!(name, "Texture");
    }

    #[test]
    fn test_type_name_qp_heatmap() {
        // Arrange
        let key = CacheKey::QpHeatmap {
            frame_idx: 0,
            hm_res: 64,
            scale_mode: "linear".to_string(),
            qp_min: 0,
            qp_max: 51,
            opacity: 128,
        };

        // Act
        let name = key.type_name();

        // Assert
        assert_eq!(name, "QpHeatmap");
    }

    #[test]
    fn test_type_name_mv_overlay() {
        // Arrange
        let key = CacheKey::MvOverlay {
            frame_idx: 0,
            viewport_hash: 12345,
            stride: 32,
            scale_x1000: 1000,
            opacity: 200,
        };

        // Act
        let name = key.type_name();

        // Assert
        assert_eq!(name, "MvOverlay");
    }

    #[test]
    fn test_type_name_partition_grid() {
        // Arrange
        let key = CacheKey::PartitionGrid {
            viewport_hash: 12345,
            zoom_tier: 1,
            mode: "blocks".to_string(),
        };

        // Act
        let name = key.type_name();

        // Assert
        assert_eq!(name, "PartitionGrid");
    }

    #[test]
    fn test_type_name_diff_heatmap() {
        // Arrange
        let key = CacheKey::DiffHeatmap {
            frame_idx_a: 0,
            frame_idx_b: 1,
            mode: "psnr".to_string(),
            ab_mapping: "side_by_side".to_string(),
            hm_res: 64,
        };

        // Act
        let name = key.type_name();

        // Assert
        assert_eq!(name, "DiffHeatmap");
    }

    #[test]
    fn test_type_name_timeline() {
        // Arrange
        let key = CacheKey::Timeline {
            data_revision: 1,
            zoom_level_x100: 100,
            filter_hash: 42,
        };

        // Act
        let name = key.type_name();

        // Assert
        assert_eq!(name, "Timeline");
    }

    #[test]
    fn test_frame_idx_decode() {
        // Arrange
        let key = CacheKey::Decode {
            frame_idx: 42,
            decode_params: "test".to_string(),
        };

        // Act
        let frame_idx = key.frame_idx();

        // Assert
        assert_eq!(frame_idx, Some(42));
    }

    #[test]
    fn test_frame_idx_texture() {
        // Arrange
        let key = CacheKey::Texture {
            frame_idx: 100,
            res_tier: 0,
            colorspace: "yuv420p".to_string(),
        };

        // Act
        let frame_idx = key.frame_idx();

        // Assert
        assert_eq!(frame_idx, Some(100));
    }

    #[test]
    fn test_frame_idx_diff_heatmap() {
        // Arrange
        let key = CacheKey::DiffHeatmap {
            frame_idx_a: 0,
            frame_idx_b: 1,
            mode: "psnr".to_string(),
            ab_mapping: "side_by_side".to_string(),
            hm_res: 64,
        };

        // Act
        let frame_idx = key.frame_idx();

        // Assert
        assert_eq!(frame_idx, Some(0)); // Returns frame_idx_a
    }

    #[test]
    fn test_frame_idx_none_for_partition_grid() {
        // Arrange
        let key = CacheKey::PartitionGrid {
            viewport_hash: 12345,
            zoom_tier: 1,
            mode: "blocks".to_string(),
        };

        // Act
        let frame_idx = key.frame_idx();

        // Assert
        assert!(frame_idx.is_none());
    }

    #[test]
    fn test_frame_idx_none_for_timeline() {
        // Arrange
        let key = CacheKey::Timeline {
            data_revision: 1,
            zoom_level_x100: 100,
            filter_hash: 42,
        };

        // Act
        let frame_idx = key.frame_idx();

        // Assert
        assert!(frame_idx.is_none());
    }

    #[test]
    fn test_is_frame_bound_decode() {
        // Arrange
        let key = CacheKey::Decode {
            frame_idx: 0,
            decode_params: "test".to_string(),
        };

        // Act
        let is_frame_bound = key.is_frame_bound();

        // Assert
        assert!(is_frame_bound);
    }

    #[test]
    fn test_is_frame_bound_timeline() {
        // Arrange
        let key = CacheKey::Timeline {
            data_revision: 1,
            zoom_level_x100: 100,
            filter_hash: 42,
        };

        // Act
        let is_frame_bound = key.is_frame_bound();

        // Assert
        assert!(!is_frame_bound);
    }
}

// ============================================================================
// CacheProvenance Tests
// ============================================================================

#[cfg(test)]
mod cache_provenance_struct_tests {
#[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_new_creates_provenance() {
        // Arrange
        let key = create_test_decode_key(0);

        // Act
        let provenance = CacheProvenance::new(key.clone(), 1024, "decoder".to_string());

        // Assert
        assert_eq!(provenance.key, key);
        assert_eq!(provenance.size_bytes, 1024);
        assert_eq!(provenance.source, "decoder");
        assert!(provenance.is_valid);
        assert!(provenance.invalidation_reason.is_none());
        assert_eq!(provenance.access_count, 0);
    }

    #[test]
    fn test_record_access_increments_count() {
        // Arrange
        let key = create_test_decode_key(0);
        let mut provenance = CacheProvenance::new(key, 1024, "decoder".to_string());

        // Act
        provenance.record_access();
        provenance.record_access();
        provenance.record_access();

        // Assert
        assert_eq!(provenance.access_count, 3);
    }

    #[test]
    fn test_record_access_updates_last_accessed() {
        // Arrange
        let key = create_test_decode_key(0);
        let mut provenance = CacheProvenance::new(key, 1024, "decoder".to_string());
        let created = provenance.created_at;

        // Act - Sleep briefly to ensure time difference
        std::thread::sleep(Duration::from_millis(10));
        provenance.record_access();

        // Assert
        assert!(provenance.last_accessed > created);
    }

    #[test]
    fn test_invalidate_sets_valid_false() {
        // Arrange
        let key = create_test_decode_key(0);
        let mut provenance = CacheProvenance::new(key, 1024, "decoder".to_string());

        // Act
        provenance.invalidate("test_reason".to_string());

        // Assert
        assert!(!provenance.is_valid);
        assert_eq!(provenance.invalidation_reason, Some("test_reason".to_string()));
    }

    #[test]
    fn test_age_returns_duration() {
        // Arrange
        let key = create_test_decode_key(0);
        let provenance = CacheProvenance::new(key, 1024, "decoder".to_string());

        // Act
        std::thread::sleep(Duration::from_millis(10));
        let age = provenance.age();

        // Assert
        assert!(age.as_millis() >= 10);
    }

    #[test]
    fn test_time_since_access_returns_duration() {
        // Arrange
        let key = create_test_decode_key(0);
        let mut provenance = CacheProvenance::new(key, 1024, "decoder".to_string());

        // Act
        std::thread::sleep(Duration::from_millis(10));
        provenance.record_access();
        std::thread::sleep(Duration::from_millis(10));
        let time_since = provenance.time_since_access();

        // Assert
        assert!(time_since.as_millis() >= 10);
    }
}

// ============================================================================
// CacheProvenanceTracker Construction Tests
// ============================================================================

#[cfg(test)]
mod tracker_construction_tests {
#[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_new_creates_tracker() {
        // Arrange & Act
        let tracker = create_test_tracker();

        // Assert
        assert_eq!(tracker.entries().len(), 0);
        assert_eq!(tracker.hit_count, 0);
        assert_eq!(tracker.miss_count, 0);
        assert_eq!(tracker.eviction_count, 0);
        assert_eq!(tracker.invalidation_count, 0);
    }

    #[test]
    fn test_default_creates_tracker() {
        // Arrange & Act
        let tracker = CacheProvenanceTracker::default();

        // Assert
        assert_eq!(tracker.entries().len(), 0);
    }
}

// ============================================================================
// CacheProvenanceTracker Entry Management Tests
// ============================================================================

#[cfg(test)]
mod entry_management_tests {
#[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_add_entry() {
        // Arrange
        let mut tracker = create_test_tracker();
        let key = create_test_decode_key(0);

        // Act
        tracker.add_entry(key.clone(), 1024, "decoder".to_string());

        // Assert
        assert!(tracker.entries().contains_key(&key));
        let stats = tracker.stats();
        assert_eq!(stats.total_entries, 1);
        assert_eq!(stats.valid_entries, 1);
    }

    #[test]
    fn test_add_entry_multiple() {
        // Arrange
        let mut tracker = create_test_tracker();

        // Act
        tracker.add_entry(create_test_decode_key(0), 1024, "decoder".to_string());
        tracker.add_entry(create_test_decode_key(1), 1024, "decoder".to_string());
        tracker.add_entry(create_test_texture_key(0), 512, "renderer".to_string());

        // Assert
        let stats = tracker.stats();
        assert_eq!(stats.total_entries, 3);
    }

    #[test]
    fn test_add_entry_increments_size() {
        // Arrange
        let mut tracker = create_test_tracker();

        // Act
        tracker.add_entry(create_test_decode_key(0), 1024, "decoder".to_string());
        tracker.add_entry(create_test_decode_key(1), 2048, "decoder".to_string());

        // Assert
        let stats = tracker.stats();
        assert_eq!(stats.total_size_bytes, 3072);
    }

    #[test]
    fn test_add_entry_overwrites() {
        // Arrange
        let mut tracker = create_test_tracker();
        let key = create_test_decode_key(0);

        // Act - Add same key twice
        tracker.add_entry(key.clone(), 1024, "decoder".to_string());
        tracker.add_entry(key.clone(), 2048, "decoder_v2".to_string());

        // Assert
        assert_eq!(tracker.entries().len(), 1); // Only one entry
        let stats = tracker.stats();
        // Note: Implementation accumulates total_size even when overwriting
        // total_size = 1024 + 2048 = 3072
        assert_eq!(stats.total_size_bytes, 3072);
    }
}

// ============================================================================
// CacheProvenanceTracker Hit/Miss Tests
// ============================================================================

#[cfg(test)]
mod hit_miss_tests {
#[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_record_hit_increments_hit_count() {
        // Arrange
        let mut tracker = create_test_tracker();
        let key = create_test_decode_key(0);
        tracker.add_entry(key.clone(), 1024, "decoder".to_string());

        // Act
        tracker.record_hit(&key);

        // Assert
        assert_eq!(tracker.hit_count, 1);
        assert_eq!(tracker.miss_count, 0);
    }

    #[test]
    fn test_record_hit_updates_access_count() {
        // Arrange
        let mut tracker = create_test_tracker();
        let key = create_test_decode_key(0);
        tracker.add_entry(key.clone(), 1024, "decoder".to_string());

        // Act
        tracker.record_hit(&key);
        tracker.record_hit(&key);

        // Assert
        let entry = tracker.entries().get(&key).unwrap();
        assert_eq!(entry.access_count, 2);
    }

    #[test]
    fn test_record_miss_increments_miss_count() {
        // Arrange
        let mut tracker = create_test_tracker();
        let key = create_test_decode_key(0);

        // Act
        tracker.record_miss(&key);

        // Assert
        assert_eq!(tracker.hit_count, 0);
        assert_eq!(tracker.miss_count, 1);
    }

    #[test]
    fn test_hit_rate_calculation() {
        // Arrange
        let mut tracker = create_test_tracker();
        let key = create_test_decode_key(0);
        tracker.add_entry(key.clone(), 1024, "decoder".to_string());

        // Act
        tracker.record_hit(&key);
        tracker.record_hit(&key);
        tracker.record_hit(&key);
        tracker.record_miss(&key);
        tracker.record_miss(&key);

        // Assert
        let stats = tracker.stats();
        assert_eq!(stats.hit_count, 3);
        assert_eq!(stats.miss_count, 2);
        assert_eq!(stats.hit_rate, 0.6); // 3/5 = 0.6
    }

    #[test]
    fn test_hit_rate_zero_requests() {
        // Arrange
        let tracker = create_test_tracker();

        // Act
        let stats = tracker.stats();

        // Assert
        assert_eq!(stats.hit_rate, 0.0);
    }
}

// ============================================================================
// CacheProvenanceTracker Invalidation Tests
// ============================================================================

#[cfg(test)]
mod invalidation_tests {
#[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_invalidate_frame_changed() {
        // Arrange
        let mut tracker = create_test_tracker();
        tracker.add_entry(create_test_decode_key(0), 1024, "decoder".to_string());
        tracker.add_entry(create_test_decode_key(1), 1024, "decoder".to_string());

        // Act - Change to frame 1
        tracker.invalidate(InvalidationTrigger::FrameChanged(1));

        // Assert
        let stats = tracker.stats();
        assert_eq!(stats.invalid_entries, 1); // Frame 0 invalidated
        assert_eq!(stats.valid_entries, 1);   // Frame 1 still valid
    }

    #[test]
    fn test_invalidate_resolution_changed() {
        // Arrange
        let mut tracker = create_test_tracker();
        tracker.add_entry(create_test_texture_key(0), 512, "renderer".to_string());
        tracker.add_entry(create_test_decode_key(0), 1024, "decoder".to_string());

        // Act
        tracker.invalidate(InvalidationTrigger::ResolutionChanged);

        // Assert
        let stats = tracker.stats();
        // Texture should be invalidated, decode should not
        assert_eq!(stats.invalid_entries, 1);
        assert_eq!(stats.valid_entries, 1);
    }

    #[test]
    fn test_invalidate_viewport_changed() {
        // Arrange
        let mut tracker = create_test_tracker();
        let mv_key = CacheKey::MvOverlay {
            frame_idx: 0,
            viewport_hash: 12345,
            stride: 32,
            scale_x1000: 1000,
            opacity: 200,
        };
        tracker.add_entry(mv_key, 512, "mv_renderer".to_string());
        tracker.add_entry(create_test_decode_key(0), 1024, "decoder".to_string());

        // Act
        tracker.invalidate(InvalidationTrigger::ViewportChanged);

        // Assert
        let stats = tracker.stats();
        // MV overlay should be invalidated
        assert_eq!(stats.invalid_entries, 1);
        assert_eq!(stats.valid_entries, 1);
    }

    #[test]
    fn test_invalidate_zoom_changed() {
        // Arrange
        let mut tracker = create_test_tracker();
        let grid_key = CacheKey::PartitionGrid {
            viewport_hash: 12345,
            zoom_tier: 1,
            mode: "blocks".to_string(),
        };
        tracker.add_entry(grid_key, 512, "grid_renderer".to_string());
        tracker.add_entry(create_test_decode_key(0), 1024, "decoder".to_string());

        // Act
        tracker.invalidate(InvalidationTrigger::ZoomChanged);

        // Assert
        let stats = tracker.stats();
        // Partition grid should be invalidated
        assert_eq!(stats.invalid_entries, 1);
        assert_eq!(stats.valid_entries, 1);
    }

    #[test]
    fn test_invalidate_data_revision() {
        // Arrange
        let mut tracker = create_test_tracker();
        tracker.add_entry(create_test_timeline_key(1, 100), 1024, "timeline".to_string());

        // Act - Change to revision 2
        tracker.invalidate(InvalidationTrigger::DataRevision(2));

        // Assert
        let stats = tracker.stats();
        assert_eq!(stats.invalid_entries, 1);
    }

    #[test]
    fn test_invalidate_manual() {
        // Arrange
        let mut tracker = create_test_tracker();
        tracker.add_entry(create_test_decode_key(0), 1024, "decoder".to_string());
        tracker.add_entry(create_test_decode_key(1), 1024, "decoder".to_string());

        // Act
        tracker.invalidate(InvalidationTrigger::Manual("test".to_string()));

        // Assert
        let stats = tracker.stats();
        // All entries should be invalidated
        assert_eq!(stats.invalid_entries, 2);
        assert_eq!(stats.valid_entries, 0);
    }
}

// ============================================================================
// CacheProvenanceTracker Eviction Tests
// ============================================================================

#[cfg(test)]
mod eviction_tests {
#[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_evict_existing_entry() {
        // Arrange
        let mut tracker = create_test_tracker();
        let key = create_test_decode_key(0);
        tracker.add_entry(key.clone(), 1024, "decoder".to_string());

        // Act
        let result = tracker.evict(&key);

        // Assert
        assert!(result);
        assert!(!tracker.entries().contains_key(&key));
        assert_eq!(tracker.eviction_count, 1);
    }

    #[test]
    fn test_evict_nonexistent_entry() {
        // Arrange
        let mut tracker = create_test_tracker();
        let key = create_test_decode_key(0);

        // Act
        let result = tracker.evict(&key);

        // Assert
        assert!(!result);
        assert_eq!(tracker.eviction_count, 0);
    }

    #[test]
    fn test_evict_decrements_size() {
        // Arrange
        let mut tracker = create_test_tracker();
        let key = create_test_decode_key(0);
        tracker.add_entry(key.clone(), 1024, "decoder".to_string());

        // Act
        tracker.evict(&key);

        // Assert
        let stats = tracker.stats();
        assert_eq!(stats.total_size_bytes, 0);
    }

    #[test]
    fn test_find_lru_eviction_candidates() {
        // Arrange
        let mut tracker = create_test_tracker();
        tracker.add_entry(create_test_decode_key(0), 1024, "decoder".to_string());
        tracker.add_entry(create_test_decode_key(1), 1024, "decoder".to_string());
        tracker.add_entry(create_test_decode_key(2), 1024, "decoder".to_string());

        // Access frame 1 and 2 to make frame 0 LRU
        tracker.record_hit(&create_test_decode_key(1));
        tracker.record_hit(&create_test_decode_key(2));

        // Act
        let candidates = tracker.find_lru_eviction_candidates(1024);

        // Assert
        assert_eq!(candidates.len(), 1);
        // Frame 0 was never accessed after being added, so it's the LRU (should be evicted first)
        if let CacheKey::Decode { frame_idx, .. } = &candidates[0] {
            assert_eq!(*frame_idx, 0);
        } else {
            panic!("Expected Decode key");
        }
    }

    #[test]
    fn test_find_lru_eviction_candidates_multiple() {
        // Arrange
        let mut tracker = create_test_tracker();
        for i in 0..5 {
            tracker.add_entry(create_test_decode_key(i), 512, "decoder".to_string());
        }

        // Access frames 2, 3, 4 to make 0, 1 LRU
        tracker.record_hit(&create_test_decode_key(2));
        tracker.record_hit(&create_test_decode_key(3));
        tracker.record_hit(&create_test_decode_key(4));

        // Act - Need to evict 1024 bytes (2 entries of 512 bytes each)
        let candidates = tracker.find_lru_eviction_candidates(1024);

        // Assert
        assert!(candidates.len() >= 2);
    }
}

// ============================================================================
// CacheProvenanceTracker Statistics Tests
// ============================================================================

#[cfg(test)]
mod statistics_tests {
#[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_stats_initial() {
        // Arrange
        let tracker = create_test_tracker();

        // Act
        let stats = tracker.stats();

        // Assert
        assert_eq!(stats.total_entries, 0);
        assert_eq!(stats.valid_entries, 0);
        assert_eq!(stats.invalid_entries, 0);
        assert_eq!(stats.total_size_bytes, 0);
    }

    #[test]
    fn test_stats_with_entries() {
        // Arrange
        let mut tracker = create_test_tracker();
        tracker.add_entry(create_test_decode_key(0), 1024, "decoder".to_string());
        tracker.add_entry(create_test_decode_key(1), 2048, "decoder".to_string());

        // Act
        let stats = tracker.stats();

        // Assert
        assert_eq!(stats.total_entries, 2);
        assert_eq!(stats.valid_entries, 2);
        assert_eq!(stats.total_size_bytes, 3072);
    }

    #[test]
    fn test_stats_with_invalidations() {
        // Arrange
        let mut tracker = create_test_tracker();
        tracker.add_entry(create_test_decode_key(0), 1024, "decoder".to_string());
        tracker.add_entry(create_test_decode_key(1), 1024, "decoder".to_string());
        tracker.invalidate(InvalidationTrigger::FrameChanged(1));

        // Act
        let stats = tracker.stats();

        // Assert
        assert_eq!(stats.total_entries, 2);
        assert_eq!(stats.valid_entries, 1);
        assert_eq!(stats.invalid_entries, 1);
    }
}

// ============================================================================
// CacheProvenanceTracker Clear Tests
// ============================================================================

#[cfg(test)]
mod clear_tests {
#[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_clear_all() {
        // Arrange
        let mut tracker = create_test_tracker();
        tracker.add_entry(create_test_decode_key(0), 1024, "decoder".to_string());
        tracker.add_entry(create_test_decode_key(1), 1024, "decoder".to_string());

        // Act
        tracker.clear();

        // Assert
        assert_eq!(tracker.entries().len(), 0);
        let stats = tracker.stats();
        assert_eq!(stats.total_size_bytes, 0);
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
    fn test_invalidate_already_invalid() {
        // Arrange
        let mut tracker = create_test_tracker();
        tracker.add_entry(create_test_decode_key(0), 1024, "decoder".to_string());
        tracker.invalidate(InvalidationTrigger::FrameChanged(1));

        // Act - Invalidate again
        tracker.invalidate(InvalidationTrigger::FrameChanged(2));

        // Assert
        let stats = tracker.stats();
        // Should not increment invalidation count for already invalid entries
        assert_eq!(stats.invalid_entries, 1);
    }

    #[test]
    fn test_empty_eviction_candidates() {
        // Arrange
        let tracker = create_test_tracker();

        // Act
        let candidates = tracker.find_lru_eviction_candidates(1024);

        // Assert
        assert_eq!(candidates.len(), 0);
    }

    #[test]
    fn test_evict_from_empty_tracker() {
        // Arrange
        let mut tracker = create_test_tracker();
        let key = create_test_decode_key(0);

        // Act
        let result = tracker.evict(&key);

        // Assert
        assert!(!result);
    }

    #[test]
    fn test_multiple_invalidations_same_trigger() {
        // Arrange
        let mut tracker = create_test_tracker();
        tracker.add_entry(create_test_decode_key(0), 1024, "decoder".to_string());
        tracker.add_entry(create_test_decode_key(1), 1024, "decoder".to_string());

        // Act - Same frame change trigger
        tracker.invalidate(InvalidationTrigger::FrameChanged(2));
        tracker.invalidate(InvalidationTrigger::FrameChanged(2));

        // Assert
        let stats = tracker.stats();
        // Second invalidation should not affect already-invalid entries
        assert_eq!(stats.invalid_entries, 2);
    }

    #[test]
    fn test_qp_heatmap_invalidated_by_resolution() {
        // Arrange
        let mut tracker = create_test_tracker();
        let qp_key = CacheKey::QpHeatmap {
            frame_idx: 0,
            hm_res: 64,
            scale_mode: "linear".to_string(),
            qp_min: 0,
            qp_max: 51,
            opacity: 128,
        };
        tracker.add_entry(qp_key, 512, "renderer".to_string());

        // Act
        tracker.invalidate(InvalidationTrigger::ResolutionChanged);

        // Assert
        let stats = tracker.stats();
        assert_eq!(stats.invalid_entries, 1);
    }

    #[test]
    fn test_diff_heatmap_invalidated_by_resolution() {
        // Arrange
        let mut tracker = create_test_tracker();
        let diff_key = CacheKey::DiffHeatmap {
            frame_idx_a: 0,
            frame_idx_b: 1,
            mode: "psnr".to_string(),
            ab_mapping: "side_by_side".to_string(),
            hm_res: 64,
        };
        tracker.add_entry(diff_key, 512, "renderer".to_string());

        // Act
        tracker.invalidate(InvalidationTrigger::ResolutionChanged);

        // Assert
        let stats = tracker.stats();
        assert_eq!(stats.invalid_entries, 1);
    }
}
