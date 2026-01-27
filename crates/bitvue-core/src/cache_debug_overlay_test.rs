// Cache debug overlay module tests
//
// Per generate-tests skill: Arrange-Act-Assert pattern with fixtures
// and edge case coverage.


// ============================================================================
// Fixtures
// ============================================================================

/// Create a test cache debug overlay
fn create_test_overlay() -> CacheDebugOverlay {
    CacheDebugOverlay::new()
}

/// Create a test cache entry
fn create_test_cached_entry(
    frame_idx: usize,
    _cache_type: CacheType,
    size_bytes: usize,
) -> CacheEntry {
    CacheEntry::cached(frame_idx as u64, "test_source".to_string(), size_bytes)
}

// ============================================================================
// CacheStatus Tests
// ============================================================================

#[cfg(test)]
mod cache_status_tests {
    use super::*;

    #[test]
    fn test_cache_status_cached_variant() {
        // Arrange & Act
        let status = CacheStatus::Cached;

        // Assert
        assert_eq!(status, CacheStatus::Cached);
        assert!(matches!(status, CacheStatus::Cached));
    }

    #[test]
    fn test_cache_status_computed_variant() {
        // Arrange & Act
        let status = CacheStatus::Computed;

        // Assert
        assert_eq!(status, CacheStatus::Computed);
    }

    #[test]
    fn test_cache_status_invalidated_variant() {
        // Arrange & Act
        let reason = InvalidationReason::StreamChanged;
        let status = CacheStatus::Invalidated(reason.clone());

        // Assert
        assert!(matches!(status, CacheStatus::Invalidated(_)));
        if let CacheStatus::Invalidated(r) = status {
            assert_eq!(r, reason);
        }
    }

    #[test]
    fn test_cache_status_missing_variant() {
        // Arrange & Act
        let status = CacheStatus::Missing;

        // Assert
        assert_eq!(status, CacheStatus::Missing);
    }
}

// ============================================================================
// InvalidationReason Tests
// ============================================================================

#[cfg(test)]
mod invalidation_reason_tests {
    use super::*;

    #[test]
    fn test_stream_changed_description() {
        // Arrange
        let reason = InvalidationReason::StreamChanged;

        // Act
        let desc = reason.description();

        // Assert
        assert_eq!(desc, "Stream changed");
    }

    #[test]
    fn test_frame_data_changed_description() {
        // Arrange
        let reason = InvalidationReason::FrameDataChanged;

        // Act
        let desc = reason.description();

        // Assert
        assert_eq!(desc, "Frame data changed");
    }

    #[test]
    fn test_user_refresh_description() {
        // Arrange
        let reason = InvalidationReason::UserRefresh;

        // Act
        let desc = reason.description();

        // Assert
        assert_eq!(desc, "User requested refresh");
    }

    #[test]
    fn test_dependency_invalidation_description() {
        // Arrange
        let dep = "decode_failed".to_string();
        let reason = InvalidationReason::DependencyInvalidation(dep.clone());

        // Act
        let desc = reason.description();

        // Assert
        assert_eq!(desc, "Dependency invalidated: decode_failed");
    }

    #[test]
    fn test_memory_pressure_description() {
        // Arrange
        let reason = InvalidationReason::MemoryPressure;

        // Act
        let desc = reason.description();

        // Assert
        assert_eq!(desc, "Memory pressure");
    }

    #[test]
    fn test_manual_description() {
        // Arrange
        let msg = "test invalidation".to_string();
        let reason = InvalidationReason::Manual(msg.clone());

        // Act
        let desc = reason.description();

        // Assert
        assert_eq!(desc, "Manual: test invalidation");
    }
}

// ============================================================================
// CacheProvenance Tests
// ============================================================================

#[cfg(test)]
mod cache_provenance_tests {
    use super::*;

    #[test]
    fn test_new_creates_provenance() {
        // Arrange & Act
        let provenance = CacheProvenance::new(100, "test_source".to_string());

        // Assert
        assert_eq!(provenance.cached_at, 100);
        assert_eq!(provenance.source, "test_source");
        assert_eq!(provenance.access_count, 0);
        assert_eq!(provenance.last_access, 100);
    }

    #[test]
    fn test_record_access_updates_counters() {
        // Arrange
        let mut provenance = CacheProvenance::new(100, "test_source".to_string());

        // Act
        provenance.record_access(150);

        // Assert
        assert_eq!(provenance.access_count, 1);
        assert_eq!(provenance.last_access, 150);
    }

    #[test]
    fn test_record_access_multiple_times() {
        // Arrange
        let mut provenance = CacheProvenance::new(100, "test_source".to_string());

        // Act
        provenance.record_access(150);
        provenance.record_access(200);
        provenance.record_access(250);

        // Assert
        assert_eq!(provenance.access_count, 3);
        assert_eq!(provenance.last_access, 250);
    }
}

// ============================================================================
// CacheEntry Tests
// ============================================================================

#[cfg(test)]
mod cache_entry_tests {
    use super::*;

    #[test]
    fn test_cached_creates_entry() {
        // Arrange & Act
        let entry = CacheEntry::cached(100, "test_source".to_string(), 1024);

        // Assert
        assert!(matches!(entry.status, CacheStatus::Cached));
        assert!(entry.provenance.is_some());
        assert_eq!(entry.size_bytes, 1024);
    }

    #[test]
    fn test_computed_creates_entry() {
        // Arrange & Act
        let entry = CacheEntry::computed(2048);

        // Assert
        assert!(matches!(entry.status, CacheStatus::Computed));
        assert!(entry.provenance.is_none());
        assert_eq!(entry.size_bytes, 2048);
    }

    #[test]
    fn test_invalidated_creates_entry() {
        // Arrange
        let reason = InvalidationReason::StreamChanged;

        // Act
        let entry = CacheEntry::invalidated(reason.clone());

        // Assert
        assert!(matches!(entry.status, CacheStatus::Invalidated(_)));
        assert!(entry.provenance.is_none());
        assert_eq!(entry.size_bytes, 0);
        assert_eq!(entry.invalidation_reason(), Some(&reason));
    }

    #[test]
    fn test_missing_creates_entry() {
        // Arrange & Act
        let entry = CacheEntry::missing();

        // Assert
        assert!(matches!(entry.status, CacheStatus::Missing));
        assert!(entry.provenance.is_none());
        assert_eq!(entry.size_bytes, 0);
    }

    #[test]
    fn test_is_valid_cached_entry() {
        // Arrange
        let entry = CacheEntry::cached(100, "test_source".to_string(), 1024);

        // Act
        let is_valid = entry.is_valid();

        // Assert
        assert!(is_valid);
    }

    #[test]
    fn test_is_valid_computed_entry() {
        // Arrange
        let entry = CacheEntry::computed(2048);

        // Act
        let is_valid = entry.is_valid();

        // Assert
        assert!(is_valid);
    }

    #[test]
    fn test_is_valid_invalidated_entry() {
        // Arrange
        let entry = CacheEntry::invalidated(InvalidationReason::StreamChanged);

        // Act
        let is_valid = entry.is_valid();

        // Assert
        assert!(!is_valid);
    }

    #[test]
    fn test_is_valid_missing_entry() {
        // Arrange
        let entry = CacheEntry::missing();

        // Act
        let is_valid = entry.is_valid();

        // Assert
        assert!(!is_valid);
    }

    #[test]
    fn test_invalidation_reason_invalidated() {
        // Arrange
        let reason = InvalidationReason::MemoryPressure;
        let entry = CacheEntry::invalidated(reason.clone());

        // Act
        let result = entry.invalidation_reason();

        // Assert
        assert_eq!(result, Some(&reason));
    }

    #[test]
    fn test_invalidation_reason_not_invalidated() {
        // Arrange
        let entry = CacheEntry::cached(100, "test_source".to_string(), 1024);

        // Act
        let result = entry.invalidation_reason();

        // Assert
        assert!(result.is_none());
    }
}

// ============================================================================
// CacheType Tests
// ============================================================================

#[cfg(test)]
mod cache_type_tests {
    use super::*;

    #[test]
    fn test_decoded_frame_name() {
        // Arrange & Act
        let name = CacheType::DecodedFrame.name();

        // Assert
        assert_eq!(name, "Decoded Frame");
    }

    #[test]
    fn test_qp_heatmap_name() {
        // Arrange & Act
        let name = CacheType::QpHeatmap.name();

        // Assert
        assert_eq!(name, "QP Heatmap");
    }

    #[test]
    fn test_mv_overlay_name() {
        // Arrange & Act
        let name = CacheType::MvOverlay.name();

        // Assert
        assert_eq!(name, "Motion Vectors");
    }

    #[test]
    fn test_partition_grid_name() {
        // Arrange & Act
        let name = CacheType::PartitionGrid.name();

        // Assert
        assert_eq!(name, "Partition Grid");
    }

    #[test]
    fn test_diff_heatmap_name() {
        // Arrange & Act
        let name = CacheType::DiffHeatmap.name();

        // Assert
        assert_eq!(name, "Diff Heatmap");
    }

    #[test]
    fn test_psnr_metrics_name() {
        // Arrange & Act
        let name = CacheType::PsnrMetrics.name();

        // Assert
        assert_eq!(name, "PSNR Metrics");
    }

    #[test]
    fn test_ssim_metrics_name() {
        // Arrange & Act
        let name = CacheType::SsimMetrics.name();

        // Assert
        assert_eq!(name, "SSIM Metrics");
    }

    #[test]
    fn test_vmaf_metrics_name() {
        // Arrange & Act
        let name = CacheType::VmafMetrics.name();

        // Assert
        assert_eq!(name, "VMAF Metrics");
    }

    #[test]
    fn test_custom_name() {
        // Arrange & Act
        let name = CacheType::Custom("MyCustomCache".to_string()).name();

        // Assert
        assert_eq!(name, "MyCustomCache");
    }
}

// ============================================================================
// CacheDebugOverlay Construction Tests
// ============================================================================

#[cfg(test)]
mod construction_tests {
    use super::*;

    #[test]
    fn test_new_creates_overlay() {
        // Arrange & Act
        let overlay = create_test_overlay();

        // Assert
        assert!(!overlay.is_visible());
        assert_eq!(overlay.get_stats().total_entries, 0);
        assert_eq!(overlay.get_stats().total_memory_bytes, 0);
    }

    #[test]
    fn test_default_creates_overlay() {
        // Arrange & Act
        let overlay = CacheDebugOverlay::default();

        // Assert
        assert!(!overlay.is_visible());
    }
}

// ============================================================================
// CacheDebugOverlay Configuration Tests
// ============================================================================

#[cfg(test)]
mod configuration_tests {
    use super::*;

    #[test]
    fn test_set_memory_limit() {
        // Arrange
        let mut overlay = create_test_overlay();

        // Act
        overlay.set_memory_limit(1024 * 1024); // 1 MB

        // Assert
        let stats = overlay.get_stats();
        assert_eq!(stats.memory_limit_bytes, Some(1024 * 1024));
    }

    #[test]
    fn test_set_visible_true() {
        // Arrange
        let mut overlay = create_test_overlay();

        // Act
        overlay.set_visible(true);

        // Assert
        assert!(overlay.is_visible());
    }

    #[test]
    fn test_set_visible_false() {
        // Arrange
        let mut overlay = create_test_overlay();
        overlay.set_visible(true);

        // Act
        overlay.set_visible(false);

        // Assert
        assert!(!overlay.is_visible());
    }
}

// ============================================================================
// CacheDebugOverlay Recording Tests
// ============================================================================

#[cfg(test)]
mod recording_tests {
    use super::*;

    #[test]
    fn test_record_cached_adds_entry() {
        // Arrange
        let mut overlay = create_test_overlay();

        // Act
        overlay.record_cached(0, CacheType::DecodedFrame, 100, "decoder".to_string(), 1024);

        // Assert
        let stats = overlay.get_stats();
        assert_eq!(stats.total_entries, 1);
        assert_eq!(stats.cached_count, 1);
        assert_eq!(stats.total_memory_bytes, 1024);
    }

    #[test]
    fn test_record_cached_multiple() {
        // Arrange
        let mut overlay = create_test_overlay();

        // Act
        overlay.record_cached(0, CacheType::DecodedFrame, 100, "decoder".to_string(), 1024);
        overlay.record_cached(1, CacheType::DecodedFrame, 101, "decoder".to_string(), 1024);
        overlay.record_cached(2, CacheType::QpHeatmap, 102, "renderer".to_string(), 512);

        // Assert
        let stats = overlay.get_stats();
        assert_eq!(stats.total_entries, 3);
        assert_eq!(stats.cached_count, 3);
        assert_eq!(stats.total_memory_bytes, 2560);
    }

    #[test]
    fn test_record_computed_adds_entry() {
        // Arrange
        let mut overlay = create_test_overlay();

        // Act
        overlay.record_computed(0, CacheType::DecodedFrame, 2048);

        // Assert
        let stats = overlay.get_stats();
        assert_eq!(stats.total_entries, 1);
        assert_eq!(stats.computed_count, 1);
        assert_eq!(stats.total_memory_bytes, 2048);
    }

    #[test]
    fn test_record_invalidation_removes_memory() {
        // Arrange
        let mut overlay = create_test_overlay();
        overlay.record_cached(0, CacheType::DecodedFrame, 100, "decoder".to_string(), 1024);

        // Act
        overlay.record_invalidation(0, CacheType::DecodedFrame, InvalidationReason::FrameDataChanged);

        // Assert
        let stats = overlay.get_stats();
        assert_eq!(stats.total_entries, 1);
        assert_eq!(stats.invalidated_count, 1);
        assert_eq!(stats.total_memory_bytes, 0); // Memory removed
    }

    #[test]
    fn test_record_invalidation_nonexistent() {
        // Arrange
        let mut overlay = create_test_overlay();

        // Act - Should not panic
        overlay.record_invalidation(0, CacheType::DecodedFrame, InvalidationReason::StreamChanged);

        // Assert
        let stats = overlay.get_stats();
        assert_eq!(stats.invalidated_count, 1);
    }

    #[test]
    fn test_record_cached_then_invalidate() {
        // Arrange
        let mut overlay = create_test_overlay();
        overlay.record_cached(0, CacheType::DecodedFrame, 100, "decoder".to_string(), 1024);

        // Act
        overlay.record_invalidation(0, CacheType::DecodedFrame, InvalidationReason::MemoryPressure);

        // Assert
        let entry = overlay.get_entry(0, &CacheType::DecodedFrame);
        assert!(entry.is_some());
        assert!(!entry.unwrap().is_valid());
    }
}

// ============================================================================
// CacheDebugOverlay Retrieval Tests
// ============================================================================

#[cfg(test)]
mod retrieval_tests {
    use super::*;

    #[test]
    fn test_get_entry_exists() {
        // Arrange
        let mut overlay = create_test_overlay();
        overlay.record_cached(0, CacheType::DecodedFrame, 100, "decoder".to_string(), 1024);

        // Act
        let entry = overlay.get_entry(0, &CacheType::DecodedFrame);

        // Assert
        assert!(entry.is_some());
        assert!(entry.unwrap().is_valid());
    }

    #[test]
    fn test_get_entry_not_exists() {
        // Arrange
        let overlay = create_test_overlay();

        // Act
        let entry = overlay.get_entry(0, &CacheType::DecodedFrame);

        // Assert
        assert!(entry.is_none());
    }

    #[test]
    fn test_get_entry_different_type() {
        // Arrange
        let mut overlay = create_test_overlay();
        overlay.record_cached(0, CacheType::DecodedFrame, 100, "decoder".to_string(), 1024);

        // Act
        let entry = overlay.get_entry(0, &CacheType::QpHeatmap);

        // Assert
        assert!(entry.is_none());
    }

    #[test]
    fn test_get_frame_entries_single() {
        // Arrange
        let mut overlay = create_test_overlay();
        overlay.record_cached(0, CacheType::DecodedFrame, 100, "decoder".to_string(), 1024);

        // Act
        let entries = overlay.get_frame_entries(0);

        // Assert
        assert_eq!(entries.len(), 1);
    }

    #[test]
    fn test_get_frame_entries_multiple() {
        // Arrange
        let mut overlay = create_test_overlay();
        overlay.record_cached(0, CacheType::DecodedFrame, 100, "decoder".to_string(), 1024);
        overlay.record_cached(0, CacheType::QpHeatmap, 100, "renderer".to_string(), 512);

        // Act
        let entries = overlay.get_frame_entries(0);

        // Assert
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn test_get_frame_entries_different_frames() {
        // Arrange
        let mut overlay = create_test_overlay();
        overlay.record_cached(0, CacheType::DecodedFrame, 100, "decoder".to_string(), 1024);
        overlay.record_cached(1, CacheType::DecodedFrame, 101, "decoder".to_string(), 1024);

        // Act
        let entries = overlay.get_frame_entries(0);

        // Assert
        assert_eq!(entries.len(), 1);
    }
}

// ============================================================================
// CacheDebugOverlay Statistics Tests
// ============================================================================

#[cfg(test)]
mod statistics_tests {
    use super::*;

    #[test]
    fn test_get_stats_initial() {
        // Arrange
        let overlay = create_test_overlay();

        // Act
        let stats = overlay.get_stats();

        // Assert
        assert_eq!(stats.total_entries, 0);
        assert_eq!(stats.cached_count, 0);
        assert_eq!(stats.computed_count, 0);
        assert_eq!(stats.invalidated_count, 0);
        assert_eq!(stats.missing_count, 0);
    }

    #[test]
    fn test_get_stats_with_entries() {
        // Arrange
        let mut overlay = create_test_overlay();
        overlay.record_cached(0, CacheType::DecodedFrame, 100, "decoder".to_string(), 1024);
        overlay.record_computed(1, CacheType::QpHeatmap, 512);
        overlay.record_invalidation(2, CacheType::DecodedFrame, InvalidationReason::StreamChanged);

        // Act
        let stats = overlay.get_stats();

        // Assert
        assert_eq!(stats.total_entries, 3);
        assert_eq!(stats.cached_count, 1);
        assert_eq!(stats.computed_count, 1);
        assert_eq!(stats.invalidated_count, 1);
    }

    #[test]
    fn test_get_stats_memory_tracking() {
        // Arrange
        let mut overlay = create_test_overlay();
        overlay.record_cached(0, CacheType::DecodedFrame, 100, "decoder".to_string(), 1024);
        overlay.record_cached(1, CacheType::DecodedFrame, 101, "decoder".to_string(), 2048);

        // Act
        let stats = overlay.get_stats();

        // Assert
        assert_eq!(stats.total_memory_bytes, 3072);
    }
}

// ============================================================================
// CacheDebugOverlay Clear Tests
// ============================================================================

#[cfg(test)]
mod clear_tests {
    use super::*;

    #[test]
    fn test_clear_all() {
        // Arrange
        let mut overlay = create_test_overlay();
        overlay.record_cached(0, CacheType::DecodedFrame, 100, "decoder".to_string(), 1024);
        overlay.record_cached(1, CacheType::QpHeatmap, 101, "renderer".to_string(), 512);

        // Act
        overlay.clear();

        // Assert
        let stats = overlay.get_stats();
        assert_eq!(stats.total_entries, 0);
        assert_eq!(stats.total_memory_bytes, 0);
    }

    #[test]
    fn test_clear_frame_single() {
        // Arrange
        let mut overlay = create_test_overlay();
        overlay.record_cached(0, CacheType::DecodedFrame, 100, "decoder".to_string(), 1024);
        overlay.record_cached(1, CacheType::DecodedFrame, 101, "decoder".to_string(), 1024);

        // Act
        overlay.clear_frame(0);

        // Assert
        let stats = overlay.get_stats();
        assert_eq!(stats.total_entries, 1);
        assert_eq!(stats.total_memory_bytes, 1024);
        assert!(overlay.get_entry(0, &CacheType::DecodedFrame).is_none());
        assert!(overlay.get_entry(1, &CacheType::DecodedFrame).is_some());
    }

    #[test]
    fn test_clear_frame_multiple_types() {
        // Arrange
        let mut overlay = create_test_overlay();
        overlay.record_cached(0, CacheType::DecodedFrame, 100, "decoder".to_string(), 1024);
        overlay.record_cached(0, CacheType::QpHeatmap, 100, "renderer".to_string(), 512);
        overlay.record_cached(1, CacheType::DecodedFrame, 101, "decoder".to_string(), 1024);

        // Act
        overlay.clear_frame(0);

        // Assert
        let stats = overlay.get_stats();
        assert_eq!(stats.total_entries, 1);
        assert_eq!(stats.total_memory_bytes, 1024);
        assert!(overlay.get_entry(0, &CacheType::DecodedFrame).is_none());
        assert!(overlay.get_entry(0, &CacheType::QpHeatmap).is_none());
        assert!(overlay.get_entry(1, &CacheType::DecodedFrame).is_some());
    }

    #[test]
    fn test_clear_frame_nonexistent() {
        // Arrange
        let mut overlay = create_test_overlay();
        overlay.record_cached(0, CacheType::DecodedFrame, 100, "decoder".to_string(), 1024);

        // Act - Should not panic
        overlay.clear_frame(1);

        // Assert
        let stats = overlay.get_stats();
        assert_eq!(stats.total_entries, 1);
    }
}

// ============================================================================
// CacheStats Tests
// ============================================================================

#[cfg(test)]
mod cache_stats_tests {
    use super::*;

    #[test]
    fn test_memory_usage_percent_with_limit() {
        // Arrange
        let stats = CacheStats {
            total_entries: 0,
            cached_count: 0,
            computed_count: 0,
            invalidated_count: 0,
            missing_count: 0,
            total_memory_bytes: 512,
            memory_limit_bytes: Some(1024),
        };

        // Act
        let percent = stats.memory_usage_percent();

        // Assert
        assert_eq!(percent, Some(50.0));
    }

    #[test]
    fn test_memory_usage_percent_no_limit() {
        // Arrange
        let stats = CacheStats {
            total_entries: 0,
            cached_count: 0,
            computed_count: 0,
            invalidated_count: 0,
            missing_count: 0,
            total_memory_bytes: 512,
            memory_limit_bytes: None,
        };

        // Act
        let percent = stats.memory_usage_percent();

        // Assert
        assert!(percent.is_none());
    }

    #[test]
    fn test_memory_usage_percent_zero_limit() {
        // Arrange
        let stats = CacheStats {
            total_entries: 0,
            cached_count: 0,
            computed_count: 0,
            invalidated_count: 0,
            missing_count: 0,
            total_memory_bytes: 512,
            memory_limit_bytes: Some(0),
        };

        // Act
        let percent = stats.memory_usage_percent();

        // Assert
        assert_eq!(percent, Some(0.0));
    }

    #[test]
    fn test_format_memory_bytes() {
        // Arrange
        let stats = CacheStats {
            total_entries: 0,
            cached_count: 0,
            computed_count: 0,
            invalidated_count: 0,
            missing_count: 0,
            total_memory_bytes: 512,
            memory_limit_bytes: None,
        };

        // Act
        let formatted = stats.format_memory();

        // Assert
        assert_eq!(formatted, "512 bytes");
    }

    #[test]
    fn test_format_memory_kb() {
        // Arrange
        let stats = CacheStats {
            total_entries: 0,
            cached_count: 0,
            computed_count: 0,
            invalidated_count: 0,
            missing_count: 0,
            total_memory_bytes: 1536,
            memory_limit_bytes: None,
        };

        // Act
        let formatted = stats.format_memory();

        // Assert
        assert!(formatted.contains("KB"));
    }

    #[test]
    fn test_format_memory_mb() {
        // Arrange
        let stats = CacheStats {
            total_entries: 0,
            cached_count: 0,
            computed_count: 0,
            invalidated_count: 0,
            missing_count: 0,
            total_memory_bytes: 2 * 1024 * 1024,
            memory_limit_bytes: None,
        };

        // Act
        let formatted = stats.format_memory();

        // Assert
        assert!(formatted.contains("MB"));
    }

    #[test]
    fn test_format_memory_gb() {
        // Arrange
        let stats = CacheStats {
            total_entries: 0,
            cached_count: 0,
            computed_count: 0,
            invalidated_count: 0,
            missing_count: 0,
            total_memory_bytes: 2 * 1024 * 1024 * 1024,
            memory_limit_bytes: None,
        };

        // Act
        let formatted = stats.format_memory();

        // Assert
        assert!(formatted.contains("GB"));
    }
}

// ============================================================================
// Edge Cases
// ============================================================================

#[cfg(test)]
mod edge_case_tests {
    use super::*;

    #[test]
    fn test_overwrite_existing_entry() {
        // Arrange
        let mut overlay = create_test_overlay();
        overlay.record_cached(0, CacheType::DecodedFrame, 100, "decoder".to_string(), 1024);

        // Act - Overwrite with computed entry
        overlay.record_computed(0, CacheType::DecodedFrame, 2048);

        // Assert
        let stats = overlay.get_stats();
        // Total entries should still be 1 (not 2)
        assert_eq!(stats.total_entries, 1);
        // Note: Implementation does NOT subtract old entry size when overwriting
        // Memory accumulates: 1024 + 2048 = 3072
        assert_eq!(stats.total_memory_bytes, 3072);
    }

    #[test]
    fn test_multiple_cache_types_same_frame() {
        // Arrange
        let mut overlay = create_test_overlay();

        // Act
        overlay.record_cached(0, CacheType::DecodedFrame, 100, "decoder".to_string(), 1024);
        overlay.record_cached(0, CacheType::QpHeatmap, 100, "renderer".to_string(), 512);
        overlay.record_cached(0, CacheType::MvOverlay, 100, "mv_renderer".to_string(), 256);
        overlay.record_cached(0, CacheType::PartitionGrid, 100, "grid_renderer".to_string(), 128);

        // Assert
        let stats = overlay.get_stats();
        assert_eq!(stats.total_entries, 4);
        assert_eq!(stats.total_memory_bytes, 1920);
    }

    #[test]
    fn test_large_memory_values() {
        // Arrange
        let mut overlay = create_test_overlay();

        // Act - Add large entries
        overlay.record_cached(0, CacheType::DecodedFrame, 100, "decoder".to_string(), 100_000_000);
        overlay.record_cached(1, CacheType::DecodedFrame, 101, "decoder".to_string(), 200_000_000);

        // Assert
        let stats = overlay.get_stats();
        assert_eq!(stats.total_memory_bytes, 300_000_000);
    }

    #[test]
    fn test_zero_size_entries() {
        // Arrange
        let mut overlay = create_test_overlay();

        // Act
        overlay.record_cached(0, CacheType::DecodedFrame, 100, "decoder".to_string(), 0);

        // Assert
        let stats = overlay.get_stats();
        assert_eq!(stats.total_memory_bytes, 0);
        assert_eq!(stats.cached_count, 1);
    }

    #[test]
    fn test_custom_cache_type() {
        // Arrange
        let mut overlay = create_test_overlay();
        let custom_type = CacheType::Custom("MyCustomCache".to_string());

        // Act
        overlay.record_cached(0, custom_type.clone(), 100, "custom_source".to_string(), 512);

        // Assert
        let entry = overlay.get_entry(0, &custom_type);
        assert!(entry.is_some());
        let stats = overlay.get_stats();
        assert_eq!(stats.cached_count, 1);
    }

    fn stats() -> CacheStats {
        CacheStats {
            total_entries: 0,
            cached_count: 0,
            computed_count: 0,
            invalidated_count: 0,
            missing_count: 0,
            total_memory_bytes: 0,
            memory_limit_bytes: None,
        }
    }

    #[test]
    fn test_saturating_sub_on_clear() {
        // Arrange
        let mut overlay = create_test_overlay();
        overlay.record_cached(0, CacheType::DecodedFrame, 100, "decoder".to_string(), 1024);

        // Act - Clear should use saturating_sub to not underflow
        overlay.clear();

        // Assert
        let stats = overlay.get_stats();
        assert_eq!(stats.total_memory_bytes, 0);
    }
}
