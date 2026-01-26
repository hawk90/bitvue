// Filmstrip module tests
//
// Per generate-tests skill: Arrange-Act-Assert pattern with fixtures
// and edge case coverage.

use super::*;
use crate::stream_state::CachedFrame;

/// Create a test cached frame with RGB data
fn create_test_frame(index: usize, width: u32, height: u32) -> CachedFrame {
    let rgb_data = vec![0u8; (width * height * 3) as usize];
    CachedFrame {
        index,
        width,
        height,
        rgb_data,
        decoded: true,
        error: None,
        y_plane: None,
        u_plane: None,
        v_plane: None,
        chroma_width: None,
        chroma_height: None,
    }
}

/// Create a test thumbnail
fn create_test_thumbnail(frame_index: usize, rgb_size: usize) -> Thumbnail {
    Thumbnail {
        frame_index,
        rgb_data: vec![0u8; rgb_size],
        width: 100,
        height: 56,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ============================================================================
    // Thumbnail Tests
    // ============================================================================

    #[test]
    fn test_thumbnail_new() {
        // Arrange & Act
        let thumbnail = Thumbnail {
            frame_index: 5,
            rgb_data: vec![255, 0, 0, 0, 255, 0],
            width: 120,
            height: 68,
        };

        // Assert
        assert_eq!(thumbnail.frame_index, 5);
        assert_eq!(thumbnail.width, 120);
        assert_eq!(thumbnail.height, 68);
        assert_eq!(thumbnail.rgb_data.len(), 6);
    }

    // ============================================================================
    // ThumbnailCache::new Tests
    // ============================================================================

    #[test]
    fn test_thumbnail_cache_new() {
        // Arrange & Act
        let cache = ThumbnailCache::new(160, 50);

        // Assert
        assert_eq!(cache.target_width, 160);
        assert_eq!(cache.max_cache_size, 50);
        assert_eq!(cache.len(), 0);
        assert!(cache.is_empty());
    }

    #[test]
    fn test_thumbnail_cache_default() {
        // Arrange & Act
        let cache = ThumbnailCache::default();

        // Assert - Default values from impl Default
        assert_eq!(cache.target_width, 120);
        assert_eq!(cache.max_cache_size, 100);
        assert!(cache.is_empty());
    }

    // ============================================================================
    // ThumbnailCache::generate_thumbnail Tests
    // ============================================================================

    #[test]
    fn test_generate_thumbnail_16x9_aspect() {
        // Arrange - 1920x1080 frame (16:9)
        let frame = create_test_frame(0, 1920, 1080);

        // Act
        let thumbnail = ThumbnailCache::generate_thumbnail(&frame, 160);

        // Assert - Height should maintain 16:9 ratio
        // 160 * (1080/1920) = 90
        assert_eq!(thumbnail.frame_index, 0);
        assert_eq!(thumbnail.width, 160);
        assert_eq!(thumbnail.height, 90);
        assert_eq!(thumbnail.rgb_data.len(), 160 * 90 * 3);
    }

    #[test]
    fn test_generate_thumbnail_4x3_aspect() {
        // Arrange - 640x480 frame (4:3)
        let frame = create_test_frame(0, 640, 480);

        // Act
        let thumbnail = ThumbnailCache::generate_thumbnail(&frame, 120);

        // Assert - Height should maintain 4:3 ratio
        // 120 * (480/640) = 90
        assert_eq!(thumbnail.width, 120);
        assert_eq!(thumbnail.height, 90);
    }

    #[test]
    fn test_generate_thumbnail_square() {
        // Arrange - 512x512 frame (1:1)
        let frame = create_test_frame(5, 512, 512);

        // Act
        let thumbnail = ThumbnailCache::generate_thumbnail(&frame, 100);

        // Assert - Square frame stays square
        assert_eq!(thumbnail.frame_index, 5);
        assert_eq!(thumbnail.width, 100);
        assert_eq!(thumbnail.height, 100);
    }

    #[test]
    fn test_generate_thumbnail_preserves_frame_index() {
        // Arrange
        let frame = create_test_frame(42, 1920, 1080);

        // Act
        let thumbnail = ThumbnailCache::generate_thumbnail(&frame, 160);

        // Assert
        assert_eq!(thumbnail.frame_index, 42);
    }

    // ============================================================================
    // ThumbnailCache::get Tests
    // ============================================================================

    #[test]
    fn test_thumbnail_cache_get_empty() {
        // Arrange
        let cache = ThumbnailCache::new(100, 10);

        // Act
        let result = cache.get(0);

        // Assert
        assert!(result.is_none());
    }

    #[test]
    fn test_thumbnail_cache_get_exists() {
        // Arrange
        let mut cache = ThumbnailCache::new(100, 10);
        let thumbnail = create_test_thumbnail(5, 100);
        cache.insert(thumbnail);

        // Act
        let result = cache.get(5);

        // Assert
        assert!(result.is_some());
        assert_eq!(result.unwrap().frame_index, 5);
    }

    #[test]
    fn test_thumbnail_cache_get_not_exists() {
        // Arrange
        let mut cache = ThumbnailCache::new(100, 10);
        cache.insert(create_test_thumbnail(5, 100));

        // Act
        let result = cache.get(10);

        // Assert
        assert!(result.is_none());
    }

    // ============================================================================
    // ThumbnailCache::contains Tests
    // ============================================================================

    #[test]
    fn test_thumbnail_cache_contains_true() {
        // Arrange
        let mut cache = ThumbnailCache::new(100, 10);
        cache.insert(create_test_thumbnail(3, 100));

        // Act & Assert
        assert!(cache.contains(3));
    }

    #[test]
    fn test_thumbnail_cache_contains_false() {
        // Arrange
        let mut cache = ThumbnailCache::new(100, 10);
        cache.insert(create_test_thumbnail(3, 100));

        // Act & Assert
        assert!(!cache.contains(5));
    }

    #[test]
    fn test_thumbnail_cache_contains_empty() {
        // Arrange & Act
        let cache = ThumbnailCache::new(100, 10);

        // Assert
        assert!(!cache.contains(0));
    }

    // ============================================================================
    // ThumbnailCache::insert Tests
    // ============================================================================

    #[test]
    fn test_thumbnail_cache_insert_single() {
        // Arrange
        let mut cache = ThumbnailCache::new(100, 10);

        // Act
        cache.insert(create_test_thumbnail(0, 100));

        // Assert
        assert_eq!(cache.len(), 1);
        assert!(cache.contains(0));
    }

    #[test]
    fn test_thumbnail_cache_insert_multiple() {
        // Arrange
        let mut cache = ThumbnailCache::new(100, 10);

        // Act
        for i in 0..5 {
            cache.insert(create_test_thumbnail(i, 100));
        }

        // Assert
        assert_eq!(cache.len(), 5);
        for i in 0..5 {
            assert!(cache.contains(i));
        }
    }

    #[test]
    fn test_thumbnail_cache_insert_replaces_existing() {
        // Arrange
        let mut cache = ThumbnailCache::new(100, 10);
        cache.insert(create_test_thumbnail(5, 100));

        // Act - Insert same frame_index again
        cache.insert(Thumbnail {
            frame_index: 5,
            rgb_data: vec![1, 2, 3],
            width: 200,
            height: 112,
        });

        // Assert - Should replace, not duplicate
        assert_eq!(cache.len(), 1);
        let thumb = cache.get(5).unwrap();
        assert_eq!(thumb.width, 200); // New value
        assert_eq!(thumb.rgb_data[0], 1);
    }

    // ============================================================================
    // ThumbnailCache LRU Eviction Tests
    // ============================================================================

    #[test]
    fn test_thumbnail_cache_lru_eviction_oldest() {
        // Arrange
        let mut cache = ThumbnailCache::new(100, 3);

        // Act - Insert 4 thumbnails (cache size = 3)
        cache.insert(create_test_thumbnail(0, 100));
        cache.insert(create_test_thumbnail(1, 100));
        cache.insert(create_test_thumbnail(2, 100));
        cache.insert(create_test_thumbnail(3, 100)); // Should evict 0

        // Assert
        assert_eq!(cache.len(), 3);
        assert!(!cache.contains(0)); // Oldest evicted
        assert!(cache.contains(1));
        assert!(cache.contains(2));
        assert!(cache.contains(3));
    }

    #[test]
    fn test_thumbnail_cache_lru_eviction_fifo() {
        // Arrange
        let mut cache = ThumbnailCache::new(100, 3);
        cache.insert(create_test_thumbnail(0, 100));
        cache.insert(create_test_thumbnail(1, 100));
        cache.insert(create_test_thumbnail(2, 100));

        // Act - Insert more, verify eviction order
        cache.insert(create_test_thumbnail(3, 100)); // Evicts 0
        assert!(!cache.contains(0));

        cache.insert(create_test_thumbnail(4, 100)); // Evicts 1
        assert!(!cache.contains(1));

        cache.insert(create_test_thumbnail(5, 100)); // Evicts 2
        assert!(!cache.contains(2));

        // Assert - Only newest 3 remain
        assert_eq!(cache.len(), 3);
        assert!(cache.contains(3));
        assert!(cache.contains(4));
        assert!(cache.contains(5));
    }

    // ============================================================================
    // ThumbnailCache::touch Tests
    // ============================================================================

    #[test]
    fn test_thumbnail_cache_touch_updates_lru() {
        // Arrange
        let mut cache = ThumbnailCache::new(100, 3);
        cache.insert(create_test_thumbnail(0, 100));
        cache.insert(create_test_thumbnail(1, 100));
        cache.insert(create_test_thumbnail(2, 100));

        // Act - Touch frame 0 to make it recent
        cache.touch(0);

        // Insert one more - should evict 1 (oldest untouched), not 0
        cache.insert(create_test_thumbnail(3, 100));

        // Assert
        assert!(cache.contains(0)); // Still present (was touched)
        assert!(!cache.contains(1)); // Evicted (oldest untouched)
        assert!(cache.contains(2));
        assert!(cache.contains(3));
    }

    #[test]
    fn test_thumbnail_cache_touch_nonexistent() {
        // Arrange
        let mut cache = ThumbnailCache::new(100, 10);
        cache.insert(create_test_thumbnail(0, 100));

        // Act - Touch non-existent frame (should not panic)
        cache.touch(99);

        // Assert - No change
        assert_eq!(cache.len(), 1);
        assert!(cache.contains(0));
    }

    #[test]
    fn test_thumbnail_cache_touch_empty() {
        // Arrange & Act
        let mut cache = ThumbnailCache::new(100, 10);
        cache.touch(0); // Should not panic on empty cache

        // Assert
        assert!(cache.is_empty());
    }

    // ============================================================================
    // ThumbnailCache::clear Tests
    // ============================================================================

    #[test]
    fn test_thumbnail_cache_clear() {
        // Arrange
        let mut cache = ThumbnailCache::new(100, 10);
        cache.insert(create_test_thumbnail(0, 100));
        cache.insert(create_test_thumbnail(1, 100));
        cache.insert(create_test_thumbnail(2, 100));
        assert_eq!(cache.len(), 3);

        // Act
        cache.clear();

        // Assert
        assert_eq!(cache.len(), 0);
        assert!(cache.is_empty());
        assert!(!cache.contains(0));
        assert!(!cache.contains(1));
        assert!(!cache.contains(2));
    }

    #[test]
    fn test_thumbnail_cache_clear_empty() {
        // Arrange
        let mut cache = ThumbnailCache::new(100, 10);
        assert!(cache.is_empty());

        // Act - Clear empty cache (should not panic)
        cache.clear();

        // Assert
        assert!(cache.is_empty());
    }

    // ============================================================================
    // ThumbnailCache::len and ::is_empty Tests
    // ============================================================================

    #[test]
    fn test_thumbnail_cache_len_empty() {
        // Arrange & Act
        let cache = ThumbnailCache::new(100, 10);

        // Assert
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn test_thumbnail_cache_len_after_inserts() {
        // Arrange
        let mut cache = ThumbnailCache::new(100, 10);

        // Act
        for i in 0..7 {
            cache.insert(create_test_thumbnail(i, 100));
        }

        // Assert
        assert_eq!(cache.len(), 7);
    }

    #[test]
    fn test_thumbnail_cache_is_empty_true() {
        // Arrange & Act
        let cache = ThumbnailCache::new(100, 10);

        // Assert
        assert!(cache.is_empty());
    }

    #[test]
    fn test_thumbnail_cache_is_empty_false() {
        // Arrange
        let mut cache = ThumbnailCache::new(100, 10);
        cache.insert(create_test_thumbnail(0, 100));

        // Act & Assert
        assert!(!cache.is_empty());
    }

    // ============================================================================
    // ThumbnailCache::visible_range Tests
    // ============================================================================

    #[test]
    fn test_visible_range_zero_frames() {
        // Arrange & Act
        let (start, end) = ThumbnailCache::visible_range(0.0, 400.0, 80.0, 4.0, 0);

        // Assert
        assert_eq!(start, 0);
        assert_eq!(end, 0);
    }

    #[test]
    fn test_visible_range_no_scroll() {
        // Arrange - 10 frames, 80px thumbnails, 4px spacing, 400px visible
        // item_width = 84px, ~5 items visible

        // Act
        let (start, end) = ThumbnailCache::visible_range(0.0, 400.0, 80.0, 4.0, 10);

        // Assert - Start at 0 with padding, end covers visible area
        assert_eq!(start, 0);
        assert!(end >= 4); // At least 5 frames visible
        assert!(end <= 9); // Not beyond total
    }

    #[test]
    fn test_visible_range_scrolled() {
        // Arrange - Scroll 500px (past ~6 frames)

        // Act
        let (start, end) = ThumbnailCache::visible_range(500.0, 400.0, 80.0, 4.0, 100);

        // Assert
        assert!(start > 0, "start should be > 0 when scrolled");
        assert!(end > start, "end should be > start");
    }

    #[test]
    fn test_visible_range_clamps_to_total() {
        // Arrange - Small total, try to get large range

        // Act
        let (start, end) = ThumbnailCache::visible_range(10000.0, 400.0, 80.0, 4.0, 5);

        // Assert - end should be clamped to total_frames - 1
        assert!(end < 5); // end is clamped to 4 (total_frames - 1)
        assert!(start >= 0); // start can exceed total when scrolling far
    }

    #[test]
    fn test_visible_range_no_spacing() {
        // Arrange - Zero spacing

        // Act
        let (start, end) = ThumbnailCache::visible_range(0.0, 400.0, 80.0, 0.0, 20);

        // Assert
        assert_eq!(start, 0);
        assert!(end >= 4); // At least 5 items (400/80)
    }

    #[test]
    fn test_visible_range_large_spacing() {
        // Arrange - Large spacing (100px)

        // Act
        let (start, end) = ThumbnailCache::visible_range(0.0, 500.0, 80.0, 100.0, 10);

        // Assert - item_width = 180px, ~3 items visible
        assert_eq!(start, 0);
        assert!(end >= 2);
        assert!(end < 10);
    }

    // ============================================================================
    // Edge Cases
    // ============================================================================

    #[test]
    fn test_thumbnail_cache_max_size_one() {
        // Arrange
        let mut cache = ThumbnailCache::new(100, 1);

        // Act - Insert multiple
        cache.insert(create_test_thumbnail(0, 100));
        cache.insert(create_test_thumbnail(1, 100));
        cache.insert(create_test_thumbnail(2, 100));

        // Assert - Only last one should remain
        assert_eq!(cache.len(), 1);
        assert!(!cache.contains(0));
        assert!(!cache.contains(1));
        assert!(cache.contains(2));
    }

    #[test]
    fn test_generate_thumbnail_small_frame() {
        // Arrange - Very small frame (32x32)
        let frame = create_test_frame(0, 32, 32);

        // Act
        let thumbnail = ThumbnailCache::generate_thumbnail(&frame, 16);

        // Assert - Should scale down correctly
        assert_eq!(thumbnail.width, 16);
        assert_eq!(thumbnail.height, 16);
        assert_eq!(thumbnail.rgb_data.len(), 16 * 16 * 3);
    }

    #[test]
    fn test_visible_range_scroll_negative_clamped() {
        // Arrange - Negative scroll (should clamp to 0)

        // Act
        let (start, end) = ThumbnailCache::visible_range(-100.0, 400.0, 80.0, 4.0, 10);

        // Assert - Start should be clamped to 0
        assert_eq!(start, 0);
        assert!(end >= 0);
    }

    #[test]
    fn test_visible_range_single_frame() {
        // Arrange - Only 1 frame total

        // Act
        let (start, end) = ThumbnailCache::visible_range(0.0, 400.0, 80.0, 4.0, 1);

        // Assert
        assert_eq!(start, 0);
        assert_eq!(end, 0); // Clamped to 0 (total - 1)
    }
}
