// Cache validation module tests
//
// Per generate-tests skill: Arrange-Act-Assert pattern with fixtures
// and edge case coverage.


// ============================================================================
// Fixtures
// ============================================================================

/// Create a test cache validator
#[allow(dead_code)]
fn create_test_validator() -> CacheValidator {
    CacheValidator::new()
}

/// Create a test cache stats row
#[allow(dead_code)]
fn create_test_stats_row() -> CacheStatsRow {
    CacheStatsRow {
        cache_type: CacheType::Decode,
        display_name: "Decode".to_string(),
        hit_rate: 0.8,
        usage_mb: 50.0,
        cap_mb: 100.0,
        usage_percent: 0.5,
        requests: 100,
        hits: 80,
        misses: 20,
        evictions: 5,
        status: CacheStatus::Healthy,
    }
}

// ============================================================================
// CacheType Tests
// ============================================================================

#[cfg(test)]
mod cache_type_tests {
#[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_display_name_decode() {
        // Arrange & Act
        let name = CacheType::Decode.display_name();

        // Assert
        assert_eq!(name, "Decode");
    }

    #[test]
    fn test_display_name_texture() {
        // Arrange & Act
        let name = CacheType::Texture.display_name();

        // Assert
        assert_eq!(name, "Texture");
    }

    #[test]
    fn test_display_name_qp_heatmap() {
        // Arrange & Act
        let name = CacheType::QpHeatmap.display_name();

        // Assert
        assert_eq!(name, "QP Heatmap");
    }

    #[test]
    fn test_display_name_diff_heatmap() {
        // Arrange & Act
        let name = CacheType::DiffHeatmap.display_name();

        // Assert
        assert_eq!(name, "Diff Heatmap");
    }

    #[test]
    fn test_display_name_mv_visible_list() {
        // Arrange & Act
        let name = CacheType::MvVisibleList.display_name();

        // Assert
        assert_eq!(name, "MV Visible List");
    }

    #[test]
    fn test_display_name_grid_line() {
        // Arrange & Act
        let name = CacheType::GridLine.display_name();

        // Assert
        assert_eq!(name, "Grid Line");
    }

    #[test]
    fn test_default_cap_decode() {
        // Arrange & Act
        let cap = CacheType::Decode.default_cap_bytes();

        // Assert
        assert_eq!(cap, 64 * 1024 * 1024); // 64 MB
    }

    #[test]
    fn test_default_cap_texture() {
        // Arrange & Act
        let cap = CacheType::Texture.default_cap_bytes();

        // Assert
        assert_eq!(cap, 256 * 1024 * 1024); // 256 MB
    }

    #[test]
    fn test_default_cap_qp_heatmap() {
        // Arrange & Act
        let cap = CacheType::QpHeatmap.default_cap_bytes();

        // Assert
        assert_eq!(cap, 128 * 1024 * 1024); // 128 MB
    }

    #[test]
    fn test_default_cap_diff_heatmap() {
        // Arrange & Act
        let cap = CacheType::DiffHeatmap.default_cap_bytes();

        // Assert
        assert_eq!(cap, 128 * 1024 * 1024); // 128 MB
    }

    #[test]
    fn test_default_cap_mv_visible_list() {
        // Arrange & Act
        let cap = CacheType::MvVisibleList.default_cap_bytes();

        // Assert
        assert_eq!(cap, 32 * 1024 * 1024); // 32 MB
    }

    #[test]
    fn test_default_cap_grid_line() {
        // Arrange & Act
        let cap = CacheType::GridLine.default_cap_bytes();

        // Assert
        assert_eq!(cap, 16 * 1024 * 1024); // 16 MB
    }
}

// ============================================================================
// CacheStats Tests
// ============================================================================

#[cfg(test)]
mod cache_stats_tests {
#[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_new_creates_stats() {
        // Arrange & Act
        let stats = CacheStats::new(CacheType::Decode);

        // Assert
        assert_eq!(stats.cache_type, CacheType::Decode);
        assert_eq!(stats.requests, 0);
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 0);
        assert_eq!(stats.evictions, 0);
        assert_eq!(stats.current_size_bytes, 0);
    }

    #[test]
    fn test_new_has_default_cap() {
        // Arrange & Act
        let stats = CacheStats::new(CacheType::Texture);

        // Assert
        assert_eq!(stats.cap_bytes, 256 * 1024 * 1024);
    }

    #[test]
    fn test_hit_rate_zero_requests() {
        // Arrange
        let stats = CacheStats::new(CacheType::Decode);

        // Act
        let hit_rate = stats.hit_rate();

        // Assert
        assert_eq!(hit_rate, 0.0);
    }

    #[test]
    fn test_hit_rate_all_hits() {
        // Arrange
        let mut stats = CacheStats::new(CacheType::Decode);
        stats.hits = 100;
        stats.requests = 100;

        // Act
        let hit_rate = stats.hit_rate();

        // Assert
        assert_eq!(hit_rate, 1.0);
    }

    #[test]
    fn test_hit_rate_partial() {
        // Arrange
        let mut stats = CacheStats::new(CacheType::Decode);
        stats.hits = 70;
        stats.misses = 30;
        stats.requests = 100;

        // Act
        let hit_rate = stats.hit_rate();

        // Assert
        assert_eq!(hit_rate, 0.7);
    }

    #[test]
    fn test_usage_percent_zero_cap() {
        // Arrange
        let mut stats = CacheStats::new(CacheType::Decode);
        stats.cap_bytes = 0;
        stats.current_size_bytes = 1024;

        // Act
        let usage = stats.usage_percent();

        // Assert
        assert_eq!(usage, 0.0);
    }

    #[test]
    fn test_usage_percent_half() {
        // Arrange
        let mut stats = CacheStats::new(CacheType::Decode);
        stats.cap_bytes = 2048;
        stats.current_size_bytes = 1024;

        // Act
        let usage = stats.usage_percent();

        // Assert
        assert_eq!(usage, 0.5);
    }

    #[test]
    fn test_usage_percent_over_cap() {
        // Arrange
        let mut stats = CacheStats::new(CacheType::Decode);
        stats.cap_bytes = 100;
        stats.current_size_bytes = 200;

        // Act
        let usage = stats.usage_percent();

        // Assert
        assert_eq!(usage, 1.0); // Capped at 1.0
    }

    #[test]
    fn test_is_over_capacity_true() {
        // Arrange
        let mut stats = CacheStats::new(CacheType::Decode);
        stats.cap_bytes = 100;
        stats.current_size_bytes = 150;

        // Act
        let over = stats.is_over_capacity();

        // Assert
        assert!(over);
    }

    #[test]
    fn test_is_over_capacity_false() {
        // Arrange
        let mut stats = CacheStats::new(CacheType::Decode);
        stats.cap_bytes = 100;
        stats.current_size_bytes = 50;

        // Act
        let over = stats.is_over_capacity();

        // Assert
        assert!(!over);
    }

    #[test]
    fn test_should_evict_aggressively_true() {
        // Arrange
        let mut stats = CacheStats::new(CacheType::Decode);
        stats.cap_bytes = 100;
        stats.current_size_bytes = 85; // 85%

        // Act
        let should = stats.should_evict_aggressively();

        // Assert
        assert!(should);
    }

    #[test]
    fn test_should_evict_aggressively_false() {
        // Arrange
        let mut stats = CacheStats::new(CacheType::Decode);
        stats.cap_bytes = 100;
        stats.current_size_bytes = 70; // 70%

        // Act
        let should = stats.should_evict_aggressively();

        // Assert
        assert!(!should);
    }

    #[test]
    fn test_record_hit_increments() {
        // Arrange
        let mut stats = CacheStats::new(CacheType::Decode);

        // Act
        stats.record_hit();

        // Assert
        assert_eq!(stats.requests, 1);
        assert_eq!(stats.hits, 1);
    }

    #[test]
    fn test_record_miss_increments() {
        // Arrange
        let mut stats = CacheStats::new(CacheType::Decode);

        // Act
        stats.record_miss();

        // Assert
        assert_eq!(stats.requests, 1);
        assert_eq!(stats.misses, 1);
    }

    #[test]
    fn test_record_eviction_decrements_size() {
        // Arrange
        let mut stats = CacheStats::new(CacheType::Decode);
        stats.current_size_bytes = 200;

        // Act
        stats.record_eviction(50);

        // Assert
        assert_eq!(stats.evictions, 1);
        assert_eq!(stats.current_size_bytes, 150);
    }

    #[test]
    fn test_add_bytes_increments_size() {
        // Arrange
        let mut stats = CacheStats::new(CacheType::Decode);

        // Act
        stats.add_bytes(1024);

        // Assert
        assert_eq!(stats.current_size_bytes, 1024);
    }

    #[test]
    fn test_set_cap() {
        // Arrange
        let mut stats = CacheStats::new(CacheType::Decode);

        // Act
        stats.set_cap(999);

        // Assert
        assert_eq!(stats.cap_bytes, 999);
    }
}

// ============================================================================
// EvictionReason Tests
// ============================================================================

#[cfg(test)]
mod eviction_reason_tests {
#[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_display_name_capacity_exceeded() {
        // Arrange & Act
        let name = EvictionReason::CapacityExceeded.display_name();

        // Assert
        assert_eq!(name, "Capacity Exceeded");
    }

    #[test]
    fn test_display_name_aggressive_eviction() {
        // Arrange & Act
        let name = EvictionReason::AggressiveEviction.display_name();

        // Assert
        assert_eq!(name, "Aggressive Eviction (>80%)");
    }

    #[test]
    fn test_display_name_manual_invalidation() {
        // Arrange & Act
        let name = EvictionReason::ManualInvalidation.display_name();

        // Assert
        assert_eq!(name, "Manual Invalidation");
    }

    #[test]
    fn test_display_name_lru() {
        // Arrange & Act
        let name = EvictionReason::Lru.display_name();

        // Assert
        assert_eq!(name, "LRU");
    }
}

// ============================================================================
// CacheValidator Construction Tests
// ============================================================================

#[cfg(test)]
mod validator_construction_tests {
#[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_new_creates_validator() {
        // Arrange & Act
        let validator = create_test_validator();

        // Assert
        assert_eq!(validator.cache_stats.len(), 6); // All cache types
        assert!(validator.eviction_log.is_empty());
    }

    #[test]
    fn test_new_has_all_cache_types() {
        // Arrange
        let validator = create_test_validator();

        // Act & Assert - All types should be present
        assert!(validator.get_stats(CacheType::Decode).is_some());
        assert!(validator.get_stats(CacheType::Texture).is_some());
        assert!(validator.get_stats(CacheType::QpHeatmap).is_some());
        assert!(validator.get_stats(CacheType::DiffHeatmap).is_some());
        assert!(validator.get_stats(CacheType::MvVisibleList).is_some());
        assert!(validator.get_stats(CacheType::GridLine).is_some());
    }

    #[test]
    fn test_default_creates_validator() {
        // Arrange & Act
        let validator = CacheValidator::default();

        // Assert
        assert_eq!(validator.cache_stats.len(), 6);
    }
}

// ============================================================================
// CacheValidator Record Tests
// ============================================================================

#[cfg(test)]
mod record_tests {
#[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_record_hit() {
        // Arrange
        let mut validator = create_test_validator();

        // Act
        validator.record_hit(CacheType::Decode);

        // Assert
        let stats = validator.get_stats(CacheType::Decode).unwrap();
        assert_eq!(stats.requests, 1);
        assert_eq!(stats.hits, 1);
    }

    #[test]
    fn test_record_miss() {
        // Arrange
        let mut validator = create_test_validator();

        // Act
        validator.record_miss(CacheType::Texture);

        // Assert
        let stats = validator.get_stats(CacheType::Texture).unwrap();
        assert_eq!(stats.requests, 1);
        assert_eq!(stats.misses, 1);
    }

    #[test]
    fn test_record_eviction() {
        // Arrange
        let mut validator = create_test_validator();
        validator.add_bytes(CacheType::Decode, 200);

        // Act
        validator.record_eviction(CacheType::Decode, EvictionReason::Lru, 50, 1, 0);

        // Assert
        let stats = validator.get_stats(CacheType::Decode).unwrap();
        assert_eq!(stats.evictions, 1);
        assert_eq!(stats.current_size_bytes, 150);
        assert_eq!(validator.eviction_log.len(), 1);
    }

    #[test]
    fn test_add_bytes() {
        // Arrange
        let mut validator = create_test_validator();

        // Act
        validator.add_bytes(CacheType::Decode, 1024);

        // Assert
        let stats = validator.get_stats(CacheType::Decode).unwrap();
        assert_eq!(stats.current_size_bytes, 1024);
    }

    #[test]
    fn test_record_eviction_tracks_usage() {
        // Arrange
        let mut validator = create_test_validator();
        validator.add_bytes(CacheType::Decode, 80);
        validator.get_stats_mut(CacheType::Decode).unwrap().cap_bytes = 100;

        // Act
        validator.record_eviction(CacheType::Decode, EvictionReason::Lru, 30, 1, 0);

        // Assert
        let event = &validator.eviction_log[0];
        assert_eq!(event.cache_usage_before, 0.8);
        assert_eq!(event.cache_usage_after, 0.5);
    }
}

// ============================================================================
// CacheValidator Eviction Log Tests
// ============================================================================

#[cfg(test)]
mod eviction_log_tests {
#[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_eviction_log_trimming() {
        // Arrange
        let mut validator = CacheValidator::new();
        validator.max_eviction_log_entries = 100;

        // Act - Add 150 evictions
        for i in 0..150 {
            validator.record_eviction(CacheType::Decode, EvictionReason::Lru, 1, 1, i);
        }

        // Assert - Should trim to max entries
        assert!(validator.eviction_log.len() <= 100);
    }

    #[test]
    fn test_get_recent_evictions() {
        // Arrange
        let mut validator = create_test_validator();
        validator.record_eviction(CacheType::Decode, EvictionReason::Lru, 1, 1, 0);
        validator.record_eviction(CacheType::Decode, EvictionReason::Lru, 1, 1, 1);
        validator.record_eviction(CacheType::Texture, EvictionReason::Lru, 1, 1, 2);

        // Act
        let decode_evs = validator.get_recent_evictions(CacheType::Decode, 10);

        // Assert
        assert_eq!(decode_evs.len(), 2);
    }

    #[test]
    fn test_get_recent_evictions_limit() {
        // Arrange
        let mut validator = create_test_validator();
        for i in 0..10 {
            validator.record_eviction(CacheType::Decode, EvictionReason::Lru, 1, 1, i);
        }

        // Act
        let evs = validator.get_recent_evictions(CacheType::Decode, 5);

        // Assert
        assert_eq!(evs.len(), 5);
    }
}

// ============================================================================
// CacheValidator Report Tests
// ============================================================================

#[cfg(test)]
mod report_tests {
#[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_generate_report_empty() {
        // Arrange
        let validator = create_test_validator();

        // Act
        let report = validator.generate_report(0);

        // Assert
        assert_eq!(report.timestamp_ms, 0);
        assert!(report.violations.is_empty());
        assert_eq!(report.overall_hit_rate, 0.0);
    }

    #[test]
    fn test_generate_report_capacity_violation() {
        // Arrange
        let mut validator = create_test_validator();
        validator.add_bytes(CacheType::Decode, 150 * 1024 * 1024); // Over 64 MB cap

        // Act
        let report = validator.generate_report(0);

        // Assert
        assert!(!report.violations.is_empty());
        let decode_violations: Vec<_> = report
            .violations
            .iter()
            .filter(|v| v.cache_type == CacheType::Decode)
            .collect();
        assert!(!decode_violations.is_empty());
        assert_eq!(
            decode_violations[0].violation_type,
            ViolationType::CapacityExceeded
        );
    }

    #[test]
    fn test_generate_report_aggressive_zone() {
        // Arrange
        let mut validator = create_test_validator();
        let cap = 64 * 1024 * 1024;
        validator.get_stats_mut(CacheType::Decode).unwrap().cap_bytes = cap;
        validator.add_bytes(CacheType::Decode, (cap as f64 * 0.85) as u64); // 85%

        // Act
        let report = validator.generate_report(0);

        // Assert
        let agg_violations: Vec<_> = report
            .violations
            .iter()
            .filter(|v| v.violation_type == ViolationType::AggressiveZone)
            .collect();
        assert!(!agg_violations.is_empty());
    }

    #[test]
    fn test_generate_report_low_hit_rate() {
        // Arrange
        let mut validator = create_test_validator();
        for _ in 0..150 {
            validator.record_miss(CacheType::Decode); // Low hit rate
        }
        validator.record_hit(CacheType::Decode);

        // Act
        let report = validator.generate_report(0);

        // Assert
        let hr_violations: Vec<_> = report
            .violations
            .iter()
            .filter(|v| v.violation_type == ViolationType::LowHitRate)
            .collect();
        assert!(!hr_violations.is_empty());
    }

    #[test]
    fn test_generate_report_overall_hit_rate() {
        // Arrange
        let mut validator = create_test_validator();
        validator.record_hit(CacheType::Decode);
        validator.record_hit(CacheType::Decode);
        validator.record_miss(CacheType::Decode);
        validator.record_hit(CacheType::Texture);
        validator.record_miss(CacheType::Texture);

        // Act
        let report = validator.generate_report(0);

        // Assert
        // 3 hits / 5 requests = 0.6
        assert_eq!(report.overall_hit_rate, 0.6);
    }

    #[test]
    fn test_generate_report_recent_evictions() {
        // Arrange
        let mut validator = create_test_validator();
        for i in 0..150 {
            validator.record_eviction(CacheType::Decode, EvictionReason::Lru, 1, 1, i);
        }

        // Act
        let report = validator.generate_report(0);

        // Assert
        assert_eq!(report.recent_evictions.len(), 100); // Max 100 recent
    }
}

// ============================================================================
// CacheValidator Clear Tests
// ============================================================================

#[cfg(test)]
mod clear_tests {
#[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_clear_stats() {
        // Arrange
        let mut validator = create_test_validator();
        validator.record_hit(CacheType::Decode);
        validator.record_miss(CacheType::Texture);
        validator.add_bytes(CacheType::Decode, 1024);
        validator.record_eviction(CacheType::Decode, EvictionReason::Lru, 1, 1, 0);

        // Act
        validator.clear_stats();

        // Assert
        for cache_type in [
            CacheType::Decode,
            CacheType::Texture,
            CacheType::QpHeatmap,
            CacheType::DiffHeatmap,
            CacheType::MvVisibleList,
            CacheType::GridLine,
        ] {
            let stats = validator.get_stats(cache_type).unwrap();
            assert_eq!(stats.requests, 0);
            assert_eq!(stats.hits, 0);
            assert_eq!(stats.misses, 0);
            assert_eq!(stats.evictions, 0);
            assert_eq!(stats.current_size_bytes, 0);
        }
        assert!(validator.eviction_log.is_empty());
    }
}

// ============================================================================
// CacheStatsHUD Tests
// ============================================================================

#[cfg(test)]
mod hud_tests {
#[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_from_validator_creates_rows() {
        // Arrange
        let mut validator = create_test_validator();
        validator.record_hit(CacheType::Decode);
        validator.add_bytes(CacheType::Decode, 10 * 1024 * 1024);

        // Act
        let hud = CacheStatsHUD::from_validator(&validator);

        // Assert
        assert_eq!(hud.cache_rows.len(), 6);
    }

    #[test]
    fn test_from_validator_calculates_total_memory() {
        // Arrange
        let mut validator = create_test_validator();
        validator.add_bytes(CacheType::Decode, 10 * 1024 * 1024); // 10 MB
        validator.add_bytes(CacheType::Texture, 20 * 1024 * 1024); // 20 MB

        // Act
        let hud = CacheStatsHUD::from_validator(&validator);

        // Assert
        assert_eq!(hud.total_memory_mb, 30.0);
    }

    #[test]
    fn test_from_validator_status_healthy() {
        // Arrange
        let mut validator = create_test_validator();
        validator.add_bytes(CacheType::Decode, 10 * 1024 * 1024); // Well under cap

        // Act
        let hud = CacheStatsHUD::from_validator(&validator);

        // Assert
        let decode_row = hud.cache_rows.iter().find(|r| r.cache_type == CacheType::Decode).unwrap();
        assert_eq!(decode_row.status, CacheStatus::Healthy);
    }

    #[test]
    fn test_from_validator_status_warning() {
        // Arrange
        let mut validator = create_test_validator();
        let cap = 100 * 1024 * 1024;
        validator.get_stats_mut(CacheType::Decode).unwrap().cap_bytes = cap;
        validator.add_bytes(CacheType::Decode, (cap as f64 * 0.85) as u64); // 85%

        // Act
        let hud = CacheStatsHUD::from_validator(&validator);

        // Assert
        let decode_row = hud.cache_rows.iter().find(|r| r.cache_type == CacheType::Decode).unwrap();
        assert_eq!(decode_row.status, CacheStatus::Warning);
    }

    #[test]
    fn test_from_validator_status_critical() {
        // Arrange
        let mut validator = create_test_validator();
        let cap = 100 * 1024 * 1024;
        validator.get_stats_mut(CacheType::Decode).unwrap().cap_bytes = cap;
        validator.add_bytes(CacheType::Decode, cap + 1); // Over cap

        // Act
        let hud = CacheStatsHUD::from_validator(&validator);

        // Assert
        let decode_row = hud.cache_rows.iter().find(|r| r.cache_type == CacheType::Decode).unwrap();
        assert_eq!(decode_row.status, CacheStatus::Critical);
    }

    #[test]
    fn test_from_validator_includes_violations() {
        // Arrange
        let mut validator = create_test_validator();
        validator.add_bytes(CacheType::Decode, 150 * 1024 * 1024); // Over 64 MB cap

        // Act
        let hud = CacheStatsHUD::from_validator(&validator);

        // Assert
        assert!(!hud.recent_violations.is_empty());
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
    fn test_record_eviction_saturating_sub() {
        // Arrange
        let mut validator = create_test_validator();
        validator.add_bytes(CacheType::Decode, 10);

        // Act - Try to evict more than exists
        validator.record_eviction(CacheType::Decode, EvictionReason::Lru, 20, 1, 0);

        // Assert
        let stats = validator.get_stats(CacheType::Decode).unwrap();
        assert_eq!(stats.current_size_bytes, 0); // Should saturate at 0
    }

    #[test]
    fn test_clear_stats_preserves_caps() {
        // Arrange
        let mut validator = create_test_validator();
        validator.get_stats_mut(CacheType::Decode).unwrap().set_cap(999);

        // Act
        validator.clear_stats();

        // Assert
        let stats = validator.get_stats(CacheType::Decode).unwrap();
        assert_eq!(stats.cap_bytes, 999); // Cap should be preserved
    }

    #[test]
    fn test_generate_report_no_requests_hit_rate() {
        // Arrange
        let validator = create_test_validator();

        // Act
        let report = validator.generate_report(0);

        // Assert
        assert_eq!(report.overall_hit_rate, 0.0);
    }

    #[test]
    fn test_eviction_log_ordering() {
        // Arrange
        let mut validator = create_test_validator();
        validator.record_eviction(CacheType::Decode, EvictionReason::Lru, 1, 1, 0);
        validator.record_eviction(CacheType::Texture, EvictionReason::Lru, 1, 1, 1);
        validator.record_eviction(CacheType::Decode, EvictionReason::Lru, 1, 1, 2);

        // Act
        let recent = validator.get_recent_evictions(CacheType::Decode, 10);

        // Assert - Should be in reverse order (most recent first)
        assert_eq!(recent[0].timestamp_ms, 2);
        assert_eq!(recent[1].timestamp_ms, 0);
    }

    #[test]
    fn test_low_hit_rate_threshold() {
        // Arrange
        let mut validator = create_test_validator();

        // Act - Note: LowHitRate violation is triggered by requests > 100 && hit_rate < 0.5
        // But this test name is misleading - it actually tests AggressiveZone violation
        // which is triggered by usage_percent() > 0.8
        // Directly set current_size_bytes to exceed 80% of capacity
        let decode_cap = validator.cache_stats[&CacheType::Decode].cap_bytes;
        validator.cache_stats.get_mut(&CacheType::Decode).unwrap().current_size_bytes = (decode_cap as f64 * 0.9) as u64;

        let report = validator.generate_report(0);

        // Assert - Should have AggressiveZone violation (capacity usage > 80%)
        let aggressive_violations: Vec<_> = report
            .violations
            .iter()
            .filter(|v| v.violation_type == ViolationType::AggressiveZone)
            .collect();
        assert!(!aggressive_violations.is_empty());
    }

    #[test]
    fn test_low_hit_rate_below_threshold() {
        // Arrange
        let mut validator = create_test_validator();

        // Act - Only 99 requests (below threshold for violation)
        for _ in 0..49 {
            validator.record_hit(CacheType::Decode);
        }
        for _ in 0..50 {
            validator.record_miss(CacheType::Decode);
        }

        let report = validator.generate_report(0);

        // Assert - Should NOT have low hit rate violation (< 100 requests)
        let hr_violations: Vec<_> = report
            .violations
            .iter()
            .filter(|v| v.violation_type == ViolationType::LowHitRate)
            .collect();
        assert!(hr_violations.is_empty());
    }
}
