#![allow(hidden_glob_reexports)]
#![allow(unreachable_code)]
#![allow(non_camel_case_types)]
#![allow(unused_assignments)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(unused_comparisons)]
#![allow(unused_doc_comments)]
//! Tests for timeline cache management

use bitvue_core::timeline_cache::TimelineCacheManager;
use bitvue_core::CacheKey;

#[test]
fn test_timeline_cache_creation() {
    let manager = TimelineCacheManager::new();
    assert_eq!(manager.data_revision(), 0);
    assert_eq!(manager.zoom_level(), 1.0);
}

#[test]
fn test_add_timeline_cache() {
    let mut manager = TimelineCacheManager::new();

    let key = manager.add_timeline_cache(1024);
    assert!(matches!(key, CacheKey::Timeline { .. }));

    let stats = manager.stats();
    assert_eq!(stats.total_entries, 1);
    assert_eq!(stats.total_size_bytes, 1024);
}

#[test]
fn test_add_lane_cache() {
    let mut manager = TimelineCacheManager::new();

    let key1 = manager.add_lane_cache("qp_lane", 512);
    let key2 = manager.add_lane_cache("mv_lane", 768);

    assert!(matches!(key1, CacheKey::Timeline { .. }));
    assert!(matches!(key2, CacheKey::Timeline { .. }));

    let stats = manager.stats();
    assert_eq!(stats.total_entries, 2);
    assert_eq!(stats.total_size_bytes, 1280);
    assert_eq!(stats.active_lanes, 2);
}

#[test]
fn test_cache_hit_miss() {
    let mut manager = TimelineCacheManager::new();

    let key = manager.add_timeline_cache(1024);

    manager.record_hit(&key);
    manager.record_hit(&key);
    manager.record_miss(&key);

    let stats = manager.stats();
    assert_eq!(stats.hit_count, 2);
    assert_eq!(stats.miss_count, 1);
    assert!((stats.hit_rate - 0.6666).abs() < 0.01);
}

#[test]
fn test_data_revision_invalidation() {
    let mut manager = TimelineCacheManager::new();

    // Add cache entries
    manager.add_timeline_cache(1024);
    manager.add_lane_cache("lane1", 512);

    let stats_before = manager.stats();
    assert_eq!(stats_before.valid_entries, 2);
    assert_eq!(stats_before.data_revision, 0);

    // Update data revision - should invalidate all
    manager.update_data_revision();

    let stats_after = manager.stats();
    assert_eq!(stats_after.valid_entries, 0);
    assert_eq!(stats_after.invalid_entries, 2);
    assert_eq!(stats_after.data_revision, 1);
}

#[test]
fn test_zoom_level_invalidation() {
    let mut manager = TimelineCacheManager::new();

    // Add cache entries
    manager.add_timeline_cache(1024);

    let stats_before = manager.stats();
    assert_eq!(stats_before.valid_entries, 1);

    // Update zoom level - should invalidate zoom-dependent caches
    manager.update_zoom_level(2.0);

    let stats_after = manager.stats();
    assert_eq!(stats_after.valid_entries, 0);
    assert_eq!(manager.zoom_level(), 2.0);
}

#[test]
fn test_zoom_level_no_change() {
    let mut manager = TimelineCacheManager::new();

    manager.add_timeline_cache(1024);

    // Update to same zoom level - should not invalidate
    manager.update_zoom_level(1.0);

    let stats = manager.stats();
    assert_eq!(stats.valid_entries, 1);
}

#[test]
fn test_filter_invalidation() {
    let mut manager = TimelineCacheManager::new();

    manager.add_timeline_cache(1024);

    let stats_before = manager.stats();
    assert_eq!(stats_before.valid_entries, 1);

    // Update filter - should invalidate
    manager.update_filter(12345);

    let stats_after = manager.stats();
    assert_eq!(stats_after.valid_entries, 0);
}

#[test]
fn test_filter_no_change() {
    let mut manager = TimelineCacheManager::new();

    manager.add_timeline_cache(1024);

    // Update to same filter - should not invalidate
    manager.update_filter(0);

    let stats = manager.stats();
    assert_eq!(stats.valid_entries, 1);
}

#[test]
fn test_invalidate_specific_lane() {
    let mut manager = TimelineCacheManager::new();

    manager.add_lane_cache("lane1", 512);
    manager.add_lane_cache("lane2", 768);

    let stats_before = manager.stats();
    assert_eq!(stats_before.total_entries, 2);
    assert_eq!(stats_before.active_lanes, 2);

    // Invalidate only lane1
    manager.invalidate_lane("lane1");

    let stats_after = manager.stats();
    assert_eq!(stats_after.total_entries, 1);
    assert_eq!(stats_after.active_lanes, 1);
}

#[test]
fn test_clear_all() {
    let mut manager = TimelineCacheManager::new();

    manager.add_timeline_cache(1024);
    manager.add_lane_cache("lane1", 512);
    manager.add_lane_cache("lane2", 768);

    let stats_before = manager.stats();
    assert_eq!(stats_before.total_entries, 3);
    assert_eq!(stats_before.valid_entries, 3);

    manager.clear_all();

    let stats_after = manager.stats();
    assert_eq!(stats_after.valid_entries, 0);
    assert_eq!(stats_after.active_lanes, 0);
}

#[test]
fn test_timeline_cache_key_bucketing() {
    let mut manager = TimelineCacheManager::new();

    // Keys at same revision/zoom/filter should be identical
    let key1 = manager.timeline_cache_key();
    let key2 = manager.timeline_cache_key();
    assert_eq!(key1, key2);

    // After data revision change, key should differ
    manager.update_data_revision();
    let key3 = manager.timeline_cache_key();
    assert_ne!(key1, key3);

    // After zoom change, key should differ
    manager.update_zoom_level(2.0);
    let key4 = manager.timeline_cache_key();
    assert_ne!(key3, key4);
}

#[test]
fn test_multiple_revisions() {
    let mut manager = TimelineCacheManager::new();

    manager.add_timeline_cache(1024);
    assert_eq!(manager.data_revision(), 0);

    manager.update_data_revision();
    assert_eq!(manager.data_revision(), 1);

    manager.add_timeline_cache(512);

    manager.update_data_revision();
    assert_eq!(manager.data_revision(), 2);

    // Should have 2 entries at revision 1, and 1 entry at revision 2
    // But after revision 2 invalidation, only the last entry is valid
    let stats = manager.stats();
    assert_eq!(stats.data_revision, 2);
}
