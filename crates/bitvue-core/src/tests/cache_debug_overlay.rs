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
//! Tests for cache_debug_overlay module

use crate::cache_debug_overlay::{
    CacheDebugOverlay, CacheEntry, CacheStatus, CacheType, InvalidationReason,
};

#[test]
fn test_cache_entry_cached() {
    let entry = CacheEntry::cached(100, "decoder".to_string(), 1024);

    assert!(entry.is_valid());
    assert!(matches!(entry.status, CacheStatus::Cached));
    assert!(entry.provenance.is_some());
    assert_eq!(entry.size_bytes, 1024);
}

#[test]
fn test_cache_entry_computed() {
    let entry = CacheEntry::computed(2048);

    assert!(entry.is_valid());
    assert!(matches!(entry.status, CacheStatus::Computed));
    assert!(entry.provenance.is_none());
    assert_eq!(entry.size_bytes, 2048);
}

#[test]
fn test_cache_entry_invalidated() {
    let entry = CacheEntry::invalidated(InvalidationReason::StreamChanged);

    assert!(!entry.is_valid());
    assert!(matches!(entry.status, CacheStatus::Invalidated(_)));
    assert!(entry.invalidation_reason().is_some());
}

#[test]
fn test_cache_debug_overlay_record() {
    let mut overlay = CacheDebugOverlay::new();

    overlay.record_cached(5, CacheType::DecodedFrame, 100, "dav1d".to_string(), 1024);

    let entry = overlay.get_entry(5, &CacheType::DecodedFrame).unwrap();
    assert!(entry.is_valid());
    assert_eq!(overlay.get_stats().total_memory_bytes, 1024);
}

#[test]
fn test_cache_stats() {
    let mut overlay = CacheDebugOverlay::new();

    overlay.record_cached(0, CacheType::DecodedFrame, 0, "decoder".to_string(), 1024);
    overlay.record_computed(1, CacheType::QpHeatmap, 512);
    overlay.record_invalidation(2, CacheType::MvOverlay, InvalidationReason::UserRefresh);

    let stats = overlay.get_stats();
    assert_eq!(stats.cached_count, 1);
    assert_eq!(stats.computed_count, 1);
    assert_eq!(stats.invalidated_count, 1);
    assert_eq!(stats.total_memory_bytes, 1024 + 512);
}
