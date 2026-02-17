#![allow(dead_code)]
//! Tests for compare_cache module

use bitvue_core::cache_provenance::CacheKey;
use bitvue_core::compare_cache::{CompareCacheManager, CompareStreamId};

#[test]
fn test_create_compare_cache_manager() {
    let manager = CompareCacheManager::new();

    assert_eq!(manager.total_entries(), 0);
    assert_eq!(manager.total_size_bytes(), 0);
    assert_eq!(manager.current_frame_a(), None);
    assert_eq!(manager.current_frame_b(), None);
    assert_eq!(manager.manual_offset(), 0);
    assert_eq!(manager.alignment_revision(), 0);
}

#[test]
fn test_add_decode_cache() {
    let mut manager = CompareCacheManager::new();

    let key_a = manager.add_decode_cache_a(0, "default".to_string(), 100000);
    let key_b = manager.add_decode_cache_b(0, "default".to_string(), 100000);

    assert_eq!(manager.total_entries(), 2);
    assert_eq!(manager.total_size_bytes(), 200000);

    let stats_a = manager.stats_a();
    let stats_b = manager.stats_b();

    assert_eq!(stats_a.total_entries, 1);
    assert_eq!(stats_b.total_entries, 1);

    // Keys are identical but tracked separately in different CacheProvenanceTrackers
    assert_eq!(key_a, key_b); // Same frame_idx and decode_params
}

#[test]
fn test_add_texture_cache() {
    let mut manager = CompareCacheManager::new();

    manager.add_texture_cache(CompareStreamId::A, 0, 0, "yuv420".to_string(), 50000);
    manager.add_texture_cache(CompareStreamId::B, 0, 0, "yuv420".to_string(), 50000);

    assert_eq!(manager.total_entries(), 2);
    assert_eq!(manager.total_size_bytes(), 100000);
}

#[test]
fn test_add_diff_heatmap_cache() {
    let mut manager = CompareCacheManager::new();

    let key =
        manager.add_diff_heatmap_cache(0, 0, "psnr".to_string(), "exact".to_string(), 64, 25000);

    assert_eq!(manager.stats_diff().total_entries, 1);
    assert_eq!(manager.stats_diff().total_size_bytes, 25000);

    let entries = manager.diff_entries_for_pair(0, 0);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0], &key);
}

#[test]
fn test_set_frame_invalidation() {
    let mut manager = CompareCacheManager::new();

    // Add cache entries
    manager.add_decode_cache_a(0, "default".to_string(), 100000);
    manager.set_frame_a(0);

    let stats_before = manager.stats_a();
    assert_eq!(stats_before.valid_entries, 1);

    // Change frame
    manager.set_frame_a(1);

    let stats_after = manager.stats_a();
    assert_eq!(stats_after.valid_entries, 0); // Invalidated
}

#[test]
fn test_manual_offset_invalidation() {
    let mut manager = CompareCacheManager::new();

    manager.add_diff_heatmap_cache(0, 0, "psnr".to_string(), "exact".to_string(), 64, 25000);

    let stats_before = manager.stats_diff();
    assert_eq!(stats_before.valid_entries, 1);

    // Change manual offset
    manager.set_manual_offset(5);

    let stats_after = manager.stats_diff();
    assert_eq!(stats_after.valid_entries, 0); // Invalidated
}

#[test]
fn test_alignment_update_invalidation() {
    let mut manager = CompareCacheManager::new();

    manager.add_diff_heatmap_cache(0, 0, "psnr".to_string(), "exact".to_string(), 64, 25000);

    let rev_before = manager.alignment_revision();
    assert_eq!(rev_before, 0);

    let stats_before = manager.stats_diff();
    assert_eq!(stats_before.valid_entries, 1);

    // Update alignment
    manager.update_alignment();

    assert_eq!(manager.alignment_revision(), 1);

    let stats_after = manager.stats_diff();
    assert_eq!(stats_after.valid_entries, 0); // Invalidated
}

#[test]
fn test_resolution_change_invalidation() {
    let mut manager = CompareCacheManager::new();

    // Set initial resolution before adding caches
    manager.set_resolution(1920, 1080, 1920, 1080);

    // Add caches AFTER setting initial resolution
    manager.add_texture_cache(CompareStreamId::A, 0, 0, "yuv420".to_string(), 50000);
    manager.add_texture_cache(CompareStreamId::B, 0, 0, "yuv420".to_string(), 50000);
    manager.add_diff_heatmap_cache(0, 0, "psnr".to_string(), "exact".to_string(), 64, 25000);

    let stats_a_before = manager.stats_a();
    let stats_b_before = manager.stats_b();
    let stats_diff_before = manager.stats_diff();

    assert_eq!(stats_a_before.valid_entries, 1);
    assert_eq!(stats_b_before.valid_entries, 1);
    assert_eq!(stats_diff_before.valid_entries, 1);

    // Change resolution
    manager.set_resolution(1280, 720, 1280, 720);

    let stats_a_after = manager.stats_a();
    let stats_b_after = manager.stats_b();
    let stats_diff_after = manager.stats_diff();

    assert_eq!(stats_a_after.valid_entries, 0); // Invalidated
    assert_eq!(stats_b_after.valid_entries, 0); // Invalidated
    assert_eq!(stats_diff_after.valid_entries, 0); // Invalidated
}

#[test]
fn test_cache_hit_miss_tracking() {
    let mut manager = CompareCacheManager::new();

    let key_a = manager.add_decode_cache_a(0, "default".to_string(), 100000);
    let key_b = manager.add_decode_cache_b(0, "default".to_string(), 100000);

    // Record hits and misses
    manager.record_hit(CompareStreamId::A, &key_a);
    manager.record_hit(CompareStreamId::A, &key_a);
    manager.record_miss(CompareStreamId::A, &key_a);

    manager.record_hit(CompareStreamId::B, &key_b);
    manager.record_miss(CompareStreamId::B, &key_b);
    manager.record_miss(CompareStreamId::B, &key_b);

    let stats_a = manager.stats_a();
    let stats_b = manager.stats_b();

    assert_eq!(stats_a.hit_count, 2);
    assert_eq!(stats_a.miss_count, 1);
    assert!((stats_a.hit_rate - 0.666).abs() < 0.01);

    assert_eq!(stats_b.hit_count, 1);
    assert_eq!(stats_b.miss_count, 2);
    assert!((stats_b.hit_rate - 0.333).abs() < 0.01);
}

#[test]
fn test_combined_stats() {
    let mut manager = CompareCacheManager::new();

    manager.add_decode_cache_a(0, "default".to_string(), 100000);
    manager.add_decode_cache_b(0, "default".to_string(), 100000);
    manager.add_diff_heatmap_cache(0, 0, "psnr".to_string(), "exact".to_string(), 64, 25000);

    let combined = manager.stats_combined();

    assert_eq!(combined.total_entries, 3);
    assert_eq!(combined.total_size_bytes, 225000);
    assert_eq!(combined.stream_a.total_entries, 1);
    assert_eq!(combined.stream_b.total_entries, 1);
    assert_eq!(combined.diff.total_entries, 1);
}

#[test]
fn test_evict_lru_stream() {
    let mut manager = CompareCacheManager::new();

    // Add multiple entries
    for i in 0..5 {
        manager.add_decode_cache_a(i, "default".to_string(), 20000);
    }

    assert_eq!(manager.stats_a().total_entries, 5);

    // Evict 50000 bytes worth (should evict ~2-3 entries)
    let evicted = manager.evict_lru_stream(CompareStreamId::A, 50000);
    assert!(evicted >= 2);

    let stats_after = manager.stats_a();
    assert!(stats_after.total_entries < 5);
}

#[test]
fn test_evict_lru_diff() {
    let mut manager = CompareCacheManager::new();

    // Add multiple diff entries
    for i in 0..5 {
        manager.add_diff_heatmap_cache(i, i, "psnr".to_string(), "exact".to_string(), 64, 10000);
    }

    assert_eq!(manager.stats_diff().total_entries, 5);

    // Evict 25000 bytes worth
    let evicted = manager.evict_lru_diff(25000);
    assert!(evicted >= 2);

    let stats_after = manager.stats_diff();
    assert!(stats_after.total_entries < 5);
}

#[test]
fn test_clear() {
    let mut manager = CompareCacheManager::new();

    manager.add_decode_cache_a(0, "default".to_string(), 100000);
    manager.add_decode_cache_b(0, "default".to_string(), 100000);
    manager.add_diff_heatmap_cache(0, 0, "psnr".to_string(), "exact".to_string(), 64, 25000);
    manager.set_frame_a(5);
    manager.set_frame_b(5);
    manager.set_manual_offset(3);
    manager.update_alignment();

    assert_eq!(manager.total_entries(), 3);

    manager.clear();

    assert_eq!(manager.total_entries(), 0);
    assert_eq!(manager.total_size_bytes(), 0);
    assert_eq!(manager.current_frame_a(), None);
    assert_eq!(manager.current_frame_b(), None);
    assert_eq!(manager.manual_offset(), 0);
    assert_eq!(manager.alignment_revision(), 0);
}

#[test]
fn test_stream_id_label() {
    assert_eq!(CompareStreamId::A.label(), "Stream A");
    assert_eq!(CompareStreamId::B.label(), "Stream B");
}

#[test]
fn test_diff_entries_for_pair_empty() {
    let manager = CompareCacheManager::new();
    let entries = manager.diff_entries_for_pair(0, 0);
    assert_eq!(entries.len(), 0);
}

#[test]
fn test_multiple_diff_entries_same_pair() {
    let mut manager = CompareCacheManager::new();

    manager.add_diff_heatmap_cache(0, 0, "psnr".to_string(), "exact".to_string(), 64, 10000);
    manager.add_diff_heatmap_cache(0, 0, "ssim".to_string(), "exact".to_string(), 64, 10000);

    let entries = manager.diff_entries_for_pair(0, 0);
    assert_eq!(entries.len(), 2);
}

#[test]
fn test_resolution_same_no_invalidation() {
    let mut manager = CompareCacheManager::new();

    manager.set_resolution(1920, 1080, 1920, 1080);

    // Add caches AFTER setting initial resolution
    manager.add_texture_cache(CompareStreamId::A, 0, 0, "yuv420".to_string(), 50000);

    let stats_before = manager.stats_a();
    assert_eq!(stats_before.valid_entries, 1);

    // Set same resolution again - should NOT invalidate
    manager.set_resolution(1920, 1080, 1920, 1080);

    let stats_after = manager.stats_a();
    assert_eq!(stats_after.valid_entries, 1); // Still valid
}

#[test]
fn test_resolution_different_triggers_invalidation() {
    let mut manager = CompareCacheManager::new();

    manager.set_resolution(1920, 1080, 1920, 1080);

    // Add caches AFTER setting initial resolution
    manager.add_texture_cache(CompareStreamId::A, 0, 0, "yuv420".to_string(), 50000);

    let stats_before = manager.stats_a();
    assert_eq!(stats_before.valid_entries, 1);

    // Set different resolution - should invalidate
    manager.set_resolution(1280, 720, 1280, 720);

    let stats_after = manager.stats_a();
    assert_eq!(stats_after.valid_entries, 0); // Invalidated
}

// UX Compare cache provenance test - Task 12 (S.T4-1.ALL.UX.Compare.impl.cache_provenance.001)

#[test]
fn test_ux_compare_dual_stream_cache_workflow() {
    // UX Compare: User loads two streams and navigates with cache tracking
    let mut manager = CompareCacheManager::new();

    // UX Compare: User loads Stream A and B
    manager.set_resolution(1920, 1080, 1920, 1080);

    // UX Compare: User plays frame 0, decodes both streams
    let key_a0 = manager.add_decode_cache_a(0, "yuv420p".to_string(), 3110400);
    let key_b0 = manager.add_decode_cache_b(0, "yuv420p".to_string(), 3110400);
    manager.set_frame_a(0);
    manager.set_frame_b(0);

    // UX Compare: User generates diff heatmap for frame pair (0,0)
    let diff_key_0_0 = manager.add_diff_heatmap_cache(
        0,
        0,
        "psnr".to_string(),
        "exact_aligned".to_string(),
        64,
        262144,
    );

    // UX Compare: Verify initial cache state
    assert_eq!(manager.total_entries(), 3); // 2 decode + 1 diff
    assert_eq!(manager.stats_a().total_entries, 1);
    assert_eq!(manager.stats_b().total_entries, 1);
    assert_eq!(manager.stats_diff().total_entries, 1);

    // UX Compare: User navigates to frame 1
    let key_a1 = manager.add_decode_cache_a(1, "yuv420p".to_string(), 3110400);
    let key_b1 = manager.add_decode_cache_b(1, "yuv420p".to_string(), 3110400);
    manager.set_frame_a(1);
    manager.set_frame_b(1);

    // UX Compare: Frame change should invalidate frame 0 caches but not the ones we just added
    let stats_a = manager.stats_a();
    let stats_b = manager.stats_b();
    assert_eq!(stats_a.total_entries, 2); // Both frame 0 and 1
    assert_eq!(stats_b.total_entries, 2);

    // UX Compare: User generates diff heatmap for frame pair (1,1)
    let diff_key_1_1 = manager.add_diff_heatmap_cache(
        1,
        1,
        "psnr".to_string(),
        "exact_aligned".to_string(),
        64,
        262144,
    );

    // UX Compare: User navigates back to frame 0 (cache hit)
    manager.set_frame_a(0);
    manager.set_frame_b(0);
    manager.record_hit(CompareStreamId::A, &key_a0);
    manager.record_hit(CompareStreamId::B, &key_b0);
    manager.record_diff_hit(&diff_key_0_0);

    // UX Compare: Forward to frame 1 again (cache hit)
    manager.set_frame_a(1);
    manager.set_frame_b(1);
    manager.record_hit(CompareStreamId::A, &key_a1);
    manager.record_hit(CompareStreamId::B, &key_b1);
    manager.record_diff_hit(&diff_key_1_1);

    // UX Compare: User plays to frame 2 (cache miss, needs decode)
    let key_a2_miss = CacheKey::Decode {
        frame_idx: 2,
        decode_params: "yuv420p".to_string(),
    };
    let key_b2_miss = CacheKey::Decode {
        frame_idx: 2,
        decode_params: "yuv420p".to_string(),
    };
    manager.record_miss(CompareStreamId::A, &key_a2_miss);
    manager.record_miss(CompareStreamId::B, &key_b2_miss);

    // UX Compare: User decodes frame 2
    let key_a2 = manager.add_decode_cache_a(2, "yuv420p".to_string(), 3110400);
    let key_b2 = manager.add_decode_cache_b(2, "yuv420p".to_string(), 3110400);
    manager.set_frame_a(2);
    manager.set_frame_b(2);

    // UX Compare: Verify cache hit/miss statistics
    let stats_a = manager.stats_a();
    let stats_b = manager.stats_b();

    assert_eq!(stats_a.hit_count, 2); // Frame 0 and 1 hits
    assert_eq!(stats_a.miss_count, 1); // Frame 2 miss
    assert!((stats_a.hit_rate - 0.666).abs() < 0.01); // 2/3 hit rate

    assert_eq!(stats_b.hit_count, 2);
    assert_eq!(stats_b.miss_count, 1);
    assert!((stats_b.hit_rate - 0.666).abs() < 0.01);

    // UX Compare: User adjusts manual offset by +1 (Stream B shifts ahead)
    manager.set_manual_offset(1);

    // UX Compare: Manual offset change invalidates all diff heatmaps
    let stats_diff_after_offset = manager.stats_diff();
    assert_eq!(stats_diff_after_offset.valid_entries, 0); // All diff heatmaps invalidated
    assert_eq!(stats_diff_after_offset.total_entries, 2); // Still 2 entries, just invalid

    // UX Compare: User regenerates diff heatmap for new alignment (0 in A → 1 in B)
    let diff_key_0_1 = manager.add_diff_heatmap_cache(
        0,
        1,
        "psnr".to_string(),
        "offset_plus_1".to_string(),
        64,
        262144,
    );

    assert_eq!(manager.stats_diff().total_entries, 3); // Old 2 invalid + new 1 valid

    // UX Compare: User resets offset back to 0
    manager.set_manual_offset(0);

    // UX Compare: Offset change again invalidates all diff heatmaps
    let stats_diff_after_reset = manager.stats_diff();
    assert_eq!(stats_diff_after_reset.valid_entries, 0);

    // UX Compare: User views combined cache statistics in dev panel
    let combined = manager.stats_combined();

    assert_eq!(combined.stream_a.total_entries, 3); // Frames 0, 1, 2
    assert_eq!(combined.stream_b.total_entries, 3); // Frames 0, 1, 2
    assert_eq!(combined.diff.total_entries, 3); // 3 diff heatmaps (all invalid)
    assert_eq!(combined.total_entries, 9);

    // Decode size: 3110400 * 6 (3 frames * 2 streams) = 18662400
    // Diff size: 262144 * 3 = 786432
    // Total: 19448832
    assert_eq!(combined.total_size_bytes, 19448832);

    // Combined hit rate: (2+2+2) / (3+3+2) = 6/8 = 0.75
    assert!((combined.combined_hit_rate - 0.75).abs() < 0.01);

    // UX Compare: User changes resolution (loads new streams at 720p)
    manager.set_resolution(1280, 720, 1280, 720);

    // UX Compare: Resolution change invalidates textures and diff overlays
    // Note: Previous set_frame calls already invalidated old frame caches
    // Only current frame (2) decode caches remain valid
    let stats_a_after_res = manager.stats_a();
    let stats_b_after_res = manager.stats_b();
    let stats_diff_after_res = manager.stats_diff();

    // Only current frame decode caches are still valid (frames 0,1 were invalidated by frame changes)
    assert_eq!(stats_a_after_res.total_entries, 3);
    assert_eq!(stats_b_after_res.total_entries, 3);
    assert_eq!(stats_diff_after_res.valid_entries, 0); // Diff heatmaps invalidated by resolution

    // UX Compare: Verify current state tracking
    assert_eq!(manager.current_frame_a(), Some(2));
    assert_eq!(manager.current_frame_b(), Some(2));
    assert_eq!(manager.manual_offset(), 0);
    assert_eq!(manager.alignment_revision(), 0);

    // UX Compare: User triggers alignment update (new algorithm run)
    manager.update_alignment();

    // UX Compare: Alignment update invalidates all diff heatmaps
    assert_eq!(manager.alignment_revision(), 1);
    let stats_diff_final = manager.stats_diff();
    assert_eq!(stats_diff_final.valid_entries, 0);

    // UX Compare: User can query diff entries for specific frame pairs
    let diff_entries_0_0 = manager.diff_entries_for_pair(0, 0);
    assert_eq!(diff_entries_0_0.len(), 1);

    let diff_entries_0_1 = manager.diff_entries_for_pair(0, 1);
    assert_eq!(diff_entries_0_1.len(), 1);

    let diff_entries_1_1 = manager.diff_entries_for_pair(1, 1);
    assert_eq!(diff_entries_1_1.len(), 1);
}
