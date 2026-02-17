#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(unused_comparisons)]
#![allow(unused_doc_comments)]
//! Tests for cache_validation module

use bitvue_core::cache_validation::{
    CacheStats, CacheStatsHUD, CacheStatus, CacheType, CacheValidator, EvictionReason,
    ViolationSeverity, ViolationType,
};

#[test]
fn test_cache_stats_new() {
    let stats = CacheStats::new(CacheType::Decode);
    assert_eq!(stats.cache_type, CacheType::Decode);
    assert_eq!(stats.requests, 0);
    assert_eq!(stats.hits, 0);
    assert_eq!(stats.misses, 0);
    assert_eq!(stats.cap_bytes, 64 * 1024 * 1024);
}

#[test]
fn test_cache_stats_hit_rate() {
    let mut stats = CacheStats::new(CacheType::Texture);

    // No requests = 0% hit rate
    assert_eq!(stats.hit_rate(), 0.0);

    // 7 hits, 3 misses = 70% hit rate
    for _ in 0..7 {
        stats.record_hit();
    }
    for _ in 0..3 {
        stats.record_miss();
    }

    assert_eq!(stats.requests, 10);
    assert_eq!(stats.hits, 7);
    assert_eq!(stats.misses, 3);
    assert!((stats.hit_rate() - 0.7).abs() < 0.001);
}

#[test]
fn test_cache_stats_usage_percent() {
    let mut stats = CacheStats::new(CacheType::QpHeatmap);
    stats.add_bytes(64 * 1024 * 1024); // 64MB

    // 64MB / 128MB = 50%
    assert!((stats.usage_percent() - 0.5).abs() < 0.001);

    // Add more to exceed 80%
    stats.add_bytes(40 * 1024 * 1024); // Total 104MB
    assert!(stats.should_evict_aggressively());
}

#[test]
fn test_cache_stats_over_capacity() {
    let mut stats = CacheStats::new(CacheType::GridLine);
    assert!(!stats.is_over_capacity());

    // Add more than cap
    stats.add_bytes(20 * 1024 * 1024); // Over 16MB cap
    assert!(stats.is_over_capacity());
}

#[test]
fn test_validator_record_hit_miss() {
    let mut validator = CacheValidator::new();

    validator.record_hit(CacheType::Decode);
    validator.record_hit(CacheType::Decode);
    validator.record_miss(CacheType::Decode);

    let stats = validator.get_stats(CacheType::Decode).unwrap();
    assert_eq!(stats.requests, 3);
    assert_eq!(stats.hits, 2);
    assert_eq!(stats.misses, 1);
}

#[test]
fn test_validator_record_eviction() {
    let mut validator = CacheValidator::new();

    // Add some bytes first
    validator.add_bytes(CacheType::Texture, 100 * 1024 * 1024);

    // Record eviction
    validator.record_eviction(
        CacheType::Texture,
        EvictionReason::Lru,
        50 * 1024 * 1024,
        10,
        1000,
    );

    let stats = validator.get_stats(CacheType::Texture).unwrap();
    assert_eq!(stats.evictions, 1);
    assert_eq!(stats.current_size_bytes, 50 * 1024 * 1024);

    assert_eq!(validator.eviction_log.len(), 1);
    assert_eq!(validator.eviction_log[0].entries_evicted, 10);
}

#[test]
fn test_validator_generate_report_no_violations() {
    let validator = CacheValidator::new();
    let report = validator.generate_report(1000);

    assert_eq!(report.violations.len(), 0);
    assert_eq!(report.cache_stats.len(), 6);
    assert_eq!(report.overall_hit_rate, 0.0);
}

#[test]
fn test_validator_generate_report_capacity_violation() {
    let mut validator = CacheValidator::new();

    // Exceed capacity
    validator.add_bytes(CacheType::GridLine, 20 * 1024 * 1024);

    let report = validator.generate_report(1000);
    assert_eq!(report.violations.len(), 1);
    assert_eq!(report.violations[0].cache_type, CacheType::GridLine);
    assert_eq!(
        report.violations[0].violation_type,
        ViolationType::CapacityExceeded
    );
    assert_eq!(report.violations[0].severity, ViolationSeverity::Error);
}

#[test]
fn test_validator_generate_report_aggressive_zone() {
    let mut validator = CacheValidator::new();

    // 85% usage (over 80% threshold)
    validator.add_bytes(CacheType::Texture, 217 * 1024 * 1024);

    let report = validator.generate_report(1000);
    assert_eq!(report.violations.len(), 1);
    assert_eq!(
        report.violations[0].violation_type,
        ViolationType::AggressiveZone
    );
    assert_eq!(report.violations[0].severity, ViolationSeverity::Warning);
}

#[test]
fn test_validator_generate_report_low_hit_rate() {
    let mut validator = CacheValidator::new();

    // 150 requests with 40% hit rate (< 50%)
    for _ in 0..60 {
        validator.record_hit(CacheType::Decode);
    }
    for _ in 0..90 {
        validator.record_miss(CacheType::Decode);
    }

    let report = validator.generate_report(1000);
    assert_eq!(report.violations.len(), 1);
    assert_eq!(
        report.violations[0].violation_type,
        ViolationType::LowHitRate
    );
    assert_eq!(report.violations[0].severity, ViolationSeverity::Info);
}

#[test]
fn test_eviction_log_trimming() {
    let mut validator = CacheValidator::new();
    validator.max_eviction_log_entries = 100;

    // Add 150 eviction events
    for i in 0..150 {
        validator.record_eviction(CacheType::Decode, EvictionReason::Lru, 1024, 1, i);
    }

    // Should be trimmed to 50 (100 oldest removed)
    assert_eq!(validator.eviction_log.len(), 50);

    // First event should have timestamp 100 (0-99 were removed)
    assert_eq!(validator.eviction_log[0].timestamp_ms, 100);
}

#[test]
fn test_cache_stats_hud_from_validator() {
    let mut validator = CacheValidator::new();

    // Add some stats
    validator.record_hit(CacheType::Decode);
    validator.record_hit(CacheType::Decode);
    validator.record_miss(CacheType::Decode);
    validator.add_bytes(CacheType::Decode, 32 * 1024 * 1024);

    let hud = CacheStatsHUD::from_validator(&validator);

    assert_eq!(hud.cache_rows.len(), 6);

    let decode_row = hud
        .cache_rows
        .iter()
        .find(|r| r.cache_type == CacheType::Decode)
        .unwrap();

    assert_eq!(decode_row.requests, 3);
    assert_eq!(decode_row.hits, 2);
    assert_eq!(decode_row.status, CacheStatus::Healthy);
    assert!((decode_row.hit_rate - 0.666).abs() < 0.01);
}

#[test]
fn test_cache_stats_hud_status() {
    let mut validator = CacheValidator::new();

    // Healthy
    let hud = CacheStatsHUD::from_validator(&validator);
    assert!(hud
        .cache_rows
        .iter()
        .all(|r| r.status == CacheStatus::Healthy));

    // Warning (>80%)
    validator.add_bytes(CacheType::QpHeatmap, 110 * 1024 * 1024);
    let hud = CacheStatsHUD::from_validator(&validator);
    let qp_row = hud
        .cache_rows
        .iter()
        .find(|r| r.cache_type == CacheType::QpHeatmap)
        .unwrap();
    assert_eq!(qp_row.status, CacheStatus::Warning);

    // Critical (over cap)
    validator.add_bytes(CacheType::QpHeatmap, 30 * 1024 * 1024);
    let hud = CacheStatsHUD::from_validator(&validator);
    let qp_row = hud
        .cache_rows
        .iter()
        .find(|r| r.cache_type == CacheType::QpHeatmap)
        .unwrap();
    assert_eq!(qp_row.status, CacheStatus::Critical);
}

#[test]
fn test_get_recent_evictions() {
    let mut validator = CacheValidator::new();

    // Add evictions for different cache types
    for i in 0..10 {
        let cache_type = if i % 2 == 0 {
            CacheType::Decode
        } else {
            CacheType::Texture
        };
        validator.record_eviction(cache_type, EvictionReason::Lru, 1024, 1, i);
    }

    let decode_evictions = validator.get_recent_evictions(CacheType::Decode, 100);
    assert_eq!(decode_evictions.len(), 5);

    let texture_evictions = validator.get_recent_evictions(CacheType::Texture, 3);
    assert_eq!(texture_evictions.len(), 3);
}

#[test]
fn test_cache_type_default_caps() {
    assert_eq!(CacheType::Decode.default_cap_bytes(), 64 * 1024 * 1024);
    assert_eq!(CacheType::Texture.default_cap_bytes(), 256 * 1024 * 1024);
    assert_eq!(CacheType::QpHeatmap.default_cap_bytes(), 128 * 1024 * 1024);
    assert_eq!(
        CacheType::DiffHeatmap.default_cap_bytes(),
        128 * 1024 * 1024
    );
    assert_eq!(
        CacheType::MvVisibleList.default_cap_bytes(),
        32 * 1024 * 1024
    );
    assert_eq!(CacheType::GridLine.default_cap_bytes(), 16 * 1024 * 1024);
}
