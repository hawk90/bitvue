//! Tests for cache_provenance module

use bitvue_core::cache_provenance::{
    CacheKey, CacheProvenance, CacheProvenanceTracker, InvalidationTrigger,
};

#[test]
fn test_cache_key_types() {
    let decode_key = CacheKey::Decode {
        frame_idx: 0,
        decode_params: "default".to_string(),
    };
    assert_eq!(decode_key.type_name(), "Decode");
    assert_eq!(decode_key.frame_idx(), Some(0));
    assert!(decode_key.is_frame_bound());

    let partition_key = CacheKey::PartitionGrid {
        viewport_hash: 12345,
        zoom_tier: 2,
        mode: "overlay".to_string(),
    };
    assert_eq!(partition_key.type_name(), "PartitionGrid");
    assert_eq!(partition_key.frame_idx(), None);
    assert!(!partition_key.is_frame_bound());
}

#[test]
fn test_cache_provenance() {
    let key = CacheKey::QpHeatmap {
        frame_idx: 0,
        hm_res: 1920,
        scale_mode: "linear".to_string(),
        qp_min: 0,
        qp_max: 51,
        opacity: 255,
    };

    let mut provenance = CacheProvenance::new(key, 1024 * 1024, "renderer".to_string());

    assert_eq!(provenance.access_count, 0);
    assert!(provenance.is_valid);

    provenance.record_access();
    assert_eq!(provenance.access_count, 1);

    provenance.invalidate("frame changed".to_string());
    assert!(!provenance.is_valid);
    assert_eq!(provenance.invalidation_reason.unwrap(), "frame changed");
}

#[test]
fn test_cache_tracker_basic() {
    let mut tracker = CacheProvenanceTracker::new();

    let key = CacheKey::Texture {
        frame_idx: 0,
        res_tier: 1,
        colorspace: "rgb".to_string(),
    };

    tracker.add_entry(key.clone(), 2048, "decoder".to_string());

    let stats = tracker.stats();
    assert_eq!(stats.total_entries, 1);
    assert_eq!(stats.valid_entries, 1);
    assert_eq!(stats.total_size_bytes, 2048);

    tracker.record_hit(&key);
    assert_eq!(tracker.hit_count, 1);

    tracker.record_miss(&key);
    assert_eq!(tracker.miss_count, 1);
}

#[test]
fn test_frame_invalidation() {
    let mut tracker = CacheProvenanceTracker::new();

    // Add entries for frame 0 and frame 1
    let key0 = CacheKey::QpHeatmap {
        frame_idx: 0,
        hm_res: 1920,
        scale_mode: "linear".to_string(),
        qp_min: 0,
        qp_max: 51,
        opacity: 255,
    };

    let key1 = CacheKey::QpHeatmap {
        frame_idx: 1,
        hm_res: 1920,
        scale_mode: "linear".to_string(),
        qp_min: 0,
        qp_max: 51,
        opacity: 255,
    };

    tracker.add_entry(key0.clone(), 1024, "renderer".to_string());
    tracker.add_entry(key1.clone(), 1024, "renderer".to_string());

    // Change to frame 0 - should invalidate frame 1
    tracker.invalidate(InvalidationTrigger::FrameChanged(0));

    let stats = tracker.stats();
    assert_eq!(stats.valid_entries, 1);
    assert_eq!(stats.invalid_entries, 1);
    assert_eq!(stats.invalidation_count, 1);
}

#[test]
fn test_resolution_invalidation() {
    let mut tracker = CacheProvenanceTracker::new();

    let texture_key = CacheKey::Texture {
        frame_idx: 0,
        res_tier: 1,
        colorspace: "rgb".to_string(),
    };

    let decode_key = CacheKey::Decode {
        frame_idx: 0,
        decode_params: "default".to_string(),
    };

    tracker.add_entry(texture_key.clone(), 2048, "decoder".to_string());
    tracker.add_entry(decode_key.clone(), 4096, "decoder".to_string());

    // Resolution change should invalidate textures but not decode cache
    tracker.invalidate(InvalidationTrigger::ResolutionChanged);

    let stats = tracker.stats();
    assert_eq!(stats.valid_entries, 1); // decode cache still valid
    assert_eq!(stats.invalid_entries, 1); // texture invalidated
}

#[test]
fn test_lru_eviction() {
    let mut tracker = CacheProvenanceTracker::new();

    // Add 3 entries
    let key1 = CacheKey::Decode {
        frame_idx: 0,
        decode_params: "default".to_string(),
    };
    let key2 = CacheKey::Decode {
        frame_idx: 1,
        decode_params: "default".to_string(),
    };
    let key3 = CacheKey::Decode {
        frame_idx: 2,
        decode_params: "default".to_string(),
    };

    tracker.add_entry(key1.clone(), 1000, "decoder".to_string());
    std::thread::sleep(std::time::Duration::from_millis(10));
    tracker.add_entry(key2.clone(), 1000, "decoder".to_string());
    std::thread::sleep(std::time::Duration::from_millis(10));
    tracker.add_entry(key3.clone(), 1000, "decoder".to_string());

    // Access key3 to make it most recently used
    tracker.record_hit(&key3);

    // Find eviction candidates
    let candidates = tracker.find_lru_eviction_candidates(1500);

    // Should evict key1 first (oldest), then key2
    assert!(candidates.len() >= 2);

    // Evict the candidates
    for key in candidates {
        tracker.evict(&key);
    }

    let stats = tracker.stats();
    assert_eq!(stats.eviction_count, 2);
}

// UX Core cache provenance integration tests
// Deliverable: cache_provenance_01_tracking:UX:Core:ALL:cache_provenance

#[test]
fn test_ux_timeline_frame_selection_cache_tracking() {
    let mut tracker = CacheProvenanceTracker::new();

    // UX Core: User clicks on timeline frame 5
    // This triggers texture load for frame 5
    let texture_key = CacheKey::Texture {
        frame_idx: 5,
        res_tier: 0,
        colorspace: "yuv420p".to_string(),
    };
    tracker.add_entry(texture_key.clone(), 4096, "ux_timeline_click".to_string());

    // UX Core: Verify cache entry was tracked
    let entry = tracker.entries().get(&texture_key).unwrap();
    assert_eq!(entry.source, "ux_timeline_click");
    assert!(entry.is_valid);
    assert_eq!(entry.size_bytes, 4096);

    // UX Core: User clicks different frame (frame 10)
    // Should invalidate frame 5 texture
    tracker.invalidate(InvalidationTrigger::FrameChanged(10));

    let entry = tracker.entries().get(&texture_key).unwrap();
    assert!(!entry.is_valid);
    assert_eq!(
        entry.invalidation_reason.as_ref().unwrap(),
        "FrameChanged(10)"
    );
}

#[test]
fn test_ux_overlay_toggle_cache_tracking() {
    let mut tracker = CacheProvenanceTracker::new();

    // UX Core: User toggles QP heatmap overlay for frame 0
    let qp_key = CacheKey::QpHeatmap {
        frame_idx: 0,
        hm_res: 1920,
        scale_mode: "log".to_string(),
        qp_min: 0,
        qp_max: 51,
        opacity: 200,
    };
    tracker.add_entry(qp_key.clone(), 8192, "ux_overlay_toggle".to_string());

    // UX Core: User toggles motion vector overlay for same frame
    let mv_key = CacheKey::MvOverlay {
        frame_idx: 0,
        viewport_hash: 0x12345678,
        stride: 16,
        scale_x1000: 1500,
        opacity: 180,
    };
    tracker.add_entry(mv_key.clone(), 2048, "ux_overlay_toggle".to_string());

    // UX Core: Verify both overlays are cached
    let stats = tracker.stats();
    assert_eq!(stats.total_entries, 2);
    assert_eq!(stats.valid_entries, 2);
    assert_eq!(stats.total_size_bytes, 8192 + 2048); // ~10KB total

    // UX Core: User navigates to different frame
    tracker.invalidate(InvalidationTrigger::FrameChanged(1));

    // Both frame-bound overlays should be invalidated
    let stats = tracker.stats();
    assert_eq!(stats.valid_entries, 0);
    assert_eq!(stats.invalid_entries, 2);
}

#[test]
fn test_ux_zoom_viewport_cache_tracking() {
    let mut tracker = CacheProvenanceTracker::new();

    // UX Core: User opens partition grid at zoom level 1
    let grid_key = CacheKey::PartitionGrid {
        viewport_hash: 0xAABBCCDD,
        zoom_tier: 1,
        mode: "all_blocks".to_string(),
    };
    tracker.add_entry(grid_key.clone(), 1024, "ux_zoom_change".to_string());

    // UX Core: Record cache access when user pans viewport
    tracker.record_hit(&grid_key);
    tracker.record_hit(&grid_key);

    let entry = tracker.entries().get(&grid_key).unwrap();
    assert_eq!(entry.access_count, 2);

    // UX Core: User zooms in (zoom level changes)
    tracker.invalidate(InvalidationTrigger::ZoomChanged);

    let stats = tracker.stats();
    assert_eq!(stats.valid_entries, 0);
    assert_eq!(stats.invalidation_count, 1);
}

#[test]
fn test_ux_compare_mode_cache_tracking() {
    let mut tracker = CacheProvenanceTracker::new();

    // UX Core: User enters A/B compare mode
    let diff_key = CacheKey::DiffHeatmap {
        frame_idx_a: 10,
        frame_idx_b: 20,
        mode: "psnr".to_string(),
        ab_mapping: "a_left_b_right".to_string(),
        hm_res: 1920,
    };
    tracker.add_entry(diff_key.clone(), 16384, "ux_compare_mode".to_string());

    // UX Core: Verify diff heatmap is cached
    let entry = tracker.entries().get(&diff_key).unwrap();
    assert_eq!(entry.source, "ux_compare_mode");
    assert_eq!(entry.size_bytes, 16384);

    // UX Core: User changes diff mode from PSNR to SSIM
    // This should invalidate the old diff heatmap
    tracker.invalidate(InvalidationTrigger::Manual("diff_mode_changed".to_string()));

    let entry = tracker.entries().get(&diff_key).unwrap();
    assert!(!entry.is_valid);
    assert_eq!(
        entry.invalidation_reason.as_ref().unwrap(),
        "Manual(\"diff_mode_changed\")"
    );
}

#[test]
fn test_ux_cache_hit_miss_stats() {
    let mut tracker = CacheProvenanceTracker::new();

    let key = CacheKey::Decode {
        frame_idx: 0,
        decode_params: "default".to_string(),
    };

    // UX Core: First access - cache miss (entry not yet created)
    tracker.record_miss(&key);

    // UX Core: Create cache entry after miss
    tracker.add_entry(key.clone(), 4096, "ux_frame_load".to_string());

    // UX Core: Second access - cache hit
    tracker.record_hit(&key);
    tracker.record_hit(&key);

    let stats = tracker.stats();
    assert_eq!(stats.hit_count, 2);
    assert_eq!(stats.miss_count, 1);
    assert!((stats.hit_rate - 0.666).abs() < 0.01); // 2 hits / 3 total = ~0.666
}

// UX Player cache provenance integration tests
// Deliverable: cache_provenance_01_tracking:UX:Player:ALL:cache_provenance

#[test]
fn test_ux_player_overlay_visibility_cache_tracking() {
    let mut tracker = CacheProvenanceTracker::new();

    // UX Player: User toggles QP overlay ON for frame 0
    let qp_key = CacheKey::QpHeatmap {
        frame_idx: 0,
        hm_res: 1920,
        scale_mode: "linear".to_string(),
        qp_min: 0,
        qp_max: 51,
        opacity: 200,
    };
    tracker.add_entry(qp_key.clone(), 8192, "ux_player_overlay_toggle".to_string());

    // UX Player: User hovers over overlay (cache hit)
    tracker.record_hit(&qp_key);
    tracker.record_hit(&qp_key);

    let entry = tracker.entries().get(&qp_key).unwrap();
    assert_eq!(entry.access_count, 2);
    assert_eq!(entry.source, "ux_player_overlay_toggle");

    // UX Player: User seeks to different frame
    tracker.invalidate(InvalidationTrigger::FrameChanged(5));

    let entry = tracker.entries().get(&qp_key).unwrap();
    assert!(!entry.is_valid);
}

#[test]
fn test_ux_player_zoom_pan_cache_invalidation() {
    let mut tracker = CacheProvenanceTracker::new();

    // UX Player: Create partition grid at zoom level 1
    let grid_key = CacheKey::PartitionGrid {
        viewport_hash: 0x11223344,
        zoom_tier: 1,
        mode: "all_blocks".to_string(),
    };
    tracker.add_entry(grid_key.clone(), 2048, "ux_player_zoom".to_string());

    // UX Player: Create MV overlay (viewport dependent)
    let mv_key = CacheKey::MvOverlay {
        frame_idx: 0,
        viewport_hash: 0x11223344,
        stride: 16,
        scale_x1000: 1000,
        opacity: 255,
    };
    tracker.add_entry(mv_key.clone(), 4096, "ux_player_viewport".to_string());

    // UX Player: User zooms in (zoom level changes)
    tracker.invalidate(InvalidationTrigger::ZoomChanged);

    // Partition grid should be invalidated (zoom dependent)
    let grid_entry = tracker.entries().get(&grid_key).unwrap();
    assert!(!grid_entry.is_valid);

    // MV overlay is still valid (not zoom dependent, only viewport dependent)
    let mv_entry = tracker.entries().get(&mv_key).unwrap();
    assert!(mv_entry.is_valid);

    // UX Player: User pans viewport
    tracker.invalidate(InvalidationTrigger::ViewportChanged);

    // Now MV overlay should be invalidated
    let mv_entry = tracker.entries().get(&mv_key).unwrap();
    assert!(!mv_entry.is_valid);
}

#[test]
fn test_ux_player_decode_texture_cache_hierarchy() {
    let mut tracker = CacheProvenanceTracker::new();

    // UX Player: Decode frame 0 (base layer)
    let decode_key = CacheKey::Decode {
        frame_idx: 0,
        decode_params: "yuv420p".to_string(),
    };
    tracker.add_entry(decode_key.clone(), 4096000, "ux_player_decode".to_string());

    // UX Player: Create texture at tier 0 (full resolution)
    let tex_tier0 = CacheKey::Texture {
        frame_idx: 0,
        res_tier: 0,
        colorspace: "rgb".to_string(),
    };
    tracker.add_entry(tex_tier0.clone(), 2048000, "ux_player_texture".to_string());

    // UX Player: Create texture at tier 1 (half resolution)
    let tex_tier1 = CacheKey::Texture {
        frame_idx: 0,
        res_tier: 1,
        colorspace: "rgb".to_string(),
    };
    tracker.add_entry(tex_tier1.clone(), 512000, "ux_player_texture".to_string());

    let stats = tracker.stats();
    assert_eq!(stats.total_entries, 3);

    // UX Player: Resolution change invalidates textures but not decode cache
    tracker.invalidate(InvalidationTrigger::ResolutionChanged);

    let decode_entry = tracker.entries().get(&decode_key).unwrap();
    assert!(decode_entry.is_valid); // Decode cache survives resolution change

    let tex0_entry = tracker.entries().get(&tex_tier0).unwrap();
    assert!(!tex0_entry.is_valid); // Texture invalidated

    let tex1_entry = tracker.entries().get(&tex_tier1).unwrap();
    assert!(!tex1_entry.is_valid); // Texture invalidated
}

#[test]
fn test_ux_player_multi_frame_decode_cache() {
    let mut tracker = CacheProvenanceTracker::new();

    // UX Player: User scrubs through frames 0-4 rapidly
    for i in 0..5 {
        let decode_key = CacheKey::Decode {
            frame_idx: i,
            decode_params: "yuv420p".to_string(),
        };
        tracker.add_entry(decode_key.clone(), 4096000, "ux_player_scrub".to_string());
    }

    assert_eq!(tracker.stats().total_entries, 5);

    // UX Player: User stops at frame 2
    tracker.invalidate(InvalidationTrigger::FrameChanged(2));

    // Frame 2 decode cache remains valid
    let frame2_key = CacheKey::Decode {
        frame_idx: 2,
        decode_params: "yuv420p".to_string(),
    };
    let entry = tracker.entries().get(&frame2_key).unwrap();
    assert!(entry.is_valid);

    // Other frames invalidated
    let stats = tracker.stats();
    assert_eq!(stats.valid_entries, 1); // Only frame 2
    assert_eq!(stats.invalid_entries, 4); // Frames 0, 1, 3, 4
}

#[test]
fn test_ux_player_lru_eviction_on_memory_pressure() {
    let mut tracker = CacheProvenanceTracker::new();

    // UX Player: Decode 5 frames
    for i in 0..5 {
        let decode_key = CacheKey::Decode {
            frame_idx: i,
            decode_params: "yuv420p".to_string(),
        };
        tracker.add_entry(decode_key.clone(), 1000000, "ux_player_decode".to_string());

        // Simulate small delay between decodes
        std::thread::sleep(std::time::Duration::from_millis(5));
    }

    // UX Player: Access frames 3 and 4 (make them MRU)
    let key3 = CacheKey::Decode {
        frame_idx: 3,
        decode_params: "yuv420p".to_string(),
    };
    let key4 = CacheKey::Decode {
        frame_idx: 4,
        decode_params: "yuv420p".to_string(),
    };
    tracker.record_hit(&key3);
    tracker.record_hit(&key4);

    // UX Player: Memory pressure - need to evict 3MB
    let candidates = tracker.find_lru_eviction_candidates(3000000);

    // Should evict frames 0, 1, 2 (LRU) and keep 3, 4 (MRU)
    assert!(candidates.len() >= 3);

    for key in candidates {
        tracker.evict(&key);
    }

    let stats = tracker.stats();
    assert_eq!(stats.eviction_count, 3);
    assert_eq!(stats.total_entries, 2); // Frames 3 and 4 remain
}

// UX Timeline cache provenance integration tests
// Deliverable: cache_provenance_01_tracking:UX:Timeline:ALL:cache_provenance

#[test]
fn test_ux_timeline_scrub_cache_invalidation() {
    let mut tracker = CacheProvenanceTracker::new();

    // UX Timeline: Cache timeline rendering at different zoom levels
    for i in 0..5 {
        let key = CacheKey::Timeline {
            data_revision: 1,
            zoom_level_x100: 100 + (i * 50), // Different zoom levels
            filter_hash: 0x12345678,
        };
        tracker.add_entry(key.clone(), 1024, "ux_timeline_scrub".to_string());
    }

    // UX Timeline: Data changes (new frames added)
    tracker.invalidate(InvalidationTrigger::DataRevision(2));

    let stats = tracker.stats();
    assert_eq!(stats.invalid_entries, 5);
}

#[test]
fn test_ux_timeline_zoom_cache_tracking() {
    let mut tracker = CacheProvenanceTracker::new();

    // UX Timeline: Timeline at 1.0x zoom
    let key_1x = CacheKey::Timeline {
        data_revision: 1,
        zoom_level_x100: 100,
        filter_hash: 0,
    };
    tracker.add_entry(key_1x.clone(), 2048, "ux_timeline_zoom".to_string());

    // UX Timeline: User zooms to 2.0x
    let key_2x = CacheKey::Timeline {
        data_revision: 1,
        zoom_level_x100: 200,
        filter_hash: 0,
    };
    tracker.add_entry(key_2x.clone(), 4096, "ux_timeline_zoom".to_string());

    // UX Timeline: Zoom change invalidates timeline viz
    tracker.invalidate(InvalidationTrigger::ZoomChanged);

    let stats = tracker.stats();
    assert_eq!(stats.invalid_entries, 2);
}

#[test]
fn test_ux_timeline_filter_cache_tracking() {
    let mut tracker = CacheProvenanceTracker::new();

    // UX Timeline: Timeline with no filter
    let key_unfiltered = CacheKey::Timeline {
        data_revision: 1,
        zoom_level_x100: 100,
        filter_hash: 0,
    };
    tracker.add_entry(
        key_unfiltered.clone(),
        1024,
        "ux_timeline_filter".to_string(),
    );

    // UX Timeline: User applies "keyframes only" filter
    let key_filtered = CacheKey::Timeline {
        data_revision: 1,
        zoom_level_x100: 100,
        filter_hash: 0xAABBCCDD, // Different hash for filtered view
    };
    tracker.add_entry(key_filtered.clone(), 512, "ux_timeline_filter".to_string());

    // UX Timeline: Both cache entries valid (different filter states)
    let stats = tracker.stats();
    assert_eq!(stats.total_entries, 2);
    assert_eq!(stats.valid_entries, 2);

    // UX Timeline: User toggles filter off (cache hit on unfiltered)
    tracker.record_hit(&key_unfiltered);

    let entry = tracker.entries().get(&key_unfiltered).unwrap();
    assert_eq!(entry.access_count, 1);
}

#[test]
fn test_ux_timeline_frame_range_cache() {
    let mut tracker = CacheProvenanceTracker::new();

    // UX Timeline: User views frames 0-99 (viewport 1)
    let key_range1 = CacheKey::Timeline {
        data_revision: 1,
        zoom_level_x100: 50, // 0.5x zoom (wide view)
        filter_hash: 0,
    };
    tracker.add_entry(key_range1.clone(), 8192, "ux_timeline_viewport".to_string());

    // UX Timeline: User pans to frames 100-199 (viewport 2)
    // Same zoom level, but different viewport = different cache entry
    // Actually, Timeline cache key doesn't have viewport, so panning
    // doesn't create new entry. Let me simulate user selecting different
    // data revision (new frames loaded)

    // UX Timeline: New video loaded (different stream)
    tracker.invalidate(InvalidationTrigger::DataRevision(2));

    let key_range2 = CacheKey::Timeline {
        data_revision: 2,
        zoom_level_x100: 50,
        filter_hash: 0,
    };
    tracker.add_entry(key_range2.clone(), 8192, "ux_timeline_viewport".to_string());

    let stats = tracker.stats();
    assert_eq!(stats.total_entries, 2);
    assert_eq!(stats.valid_entries, 1); // Only new video timeline valid
    assert_eq!(stats.invalid_entries, 1); // Old video timeline invalidated
}

#[test]
fn test_ux_timeline_marker_cache_tracking() {
    let mut tracker = CacheProvenanceTracker::new();

    // UX Timeline: User views timeline with error/bookmark markers
    let key = CacheKey::Timeline {
        data_revision: 1,
        zoom_level_x100: 100,
        filter_hash: 0x11111111, // Hash includes marker state
    };
    tracker.add_entry(key.clone(), 2048, "ux_timeline_markers".to_string());

    // UX Timeline: User adds bookmark to frame (marker state changes)
    tracker.invalidate(InvalidationTrigger::Manual("marker_added".to_string()));

    let new_key = CacheKey::Timeline {
        data_revision: 1,
        zoom_level_x100: 100,
        filter_hash: 0x22222222, // New hash with updated markers
    };
    tracker.add_entry(new_key, 2048, "ux_timeline_markers".to_string());

    let stats = tracker.stats();
    assert_eq!(stats.total_entries, 2);
    assert_eq!(stats.valid_entries, 1);
    assert_eq!(stats.invalidation_count, 1);
}
