//! Filmstrip - Thumbnail data structures for filmstrip
//!
//! Monster Pack v14: Feature Parity - Filmstrip Panel

use crate::stream_state::CachedFrame;
use std::collections::HashMap;

/// Thumbnail image data (downsampled from full frame)
#[derive(Debug, Clone)]
pub struct Thumbnail {
    /// Frame index
    pub frame_index: usize,
    /// RGB data (RGB8 packed: r,g,b,r,g,b,...)
    pub rgb_data: Vec<u8>,
    /// Thumbnail width
    pub width: u32,
    /// Thumbnail height
    pub height: u32,
}

/// Thumbnail cache for filmstrip with LRU eviction
#[derive(Debug, Clone)]
pub struct ThumbnailCache {
    /// Cached thumbnails indexed by frame_index
    thumbnails: HashMap<usize, Thumbnail>,
    /// LRU order (most recent at end)
    lru_order: Vec<usize>,
    /// Target thumbnail width (height computed from aspect ratio)
    pub target_width: u32,
    /// Maximum cache size
    pub max_cache_size: usize,
}

impl Default for ThumbnailCache {
    fn default() -> Self {
        Self::new(120, 100)
    }
}

impl ThumbnailCache {
    /// Create new thumbnail cache
    /// - target_width: desired thumbnail width in pixels
    /// - max_cache_size: maximum number of thumbnails to cache
    pub fn new(target_width: u32, max_cache_size: usize) -> Self {
        Self {
            thumbnails: HashMap::new(),
            lru_order: Vec::new(),
            target_width,
            max_cache_size,
        }
    }

    /// Generate thumbnail from CachedFrame using bilinear downsampling
    pub fn generate_thumbnail(frame: &CachedFrame, target_width: u32) -> Thumbnail {
        let src_width = frame.width as usize;
        let src_height = frame.height as usize;

        // Calculate target height maintaining aspect ratio
        let aspect = src_height as f32 / src_width as f32;
        let target_height = (target_width as f32 * aspect) as u32;

        let dst_width = target_width as usize;
        let dst_height = target_height as usize;

        // Downsample using simple box filter (average)
        let mut rgb_data = vec![0u8; dst_width * dst_height * 3];

        let scale_x = src_width as f32 / dst_width as f32;
        let scale_y = src_height as f32 / dst_height as f32;

        for dst_y in 0..dst_height {
            for dst_x in 0..dst_width {
                // Source region
                let src_x = (dst_x as f32 * scale_x) as usize;
                let src_y = (dst_y as f32 * scale_y) as usize;

                // Clamp to bounds
                let src_x = src_x.min(src_width - 1);
                let src_y = src_y.min(src_height - 1);

                let src_idx = (src_y * src_width + src_x) * 3;
                let dst_idx = (dst_y * dst_width + dst_x) * 3;

                if src_idx + 2 < frame.rgb_data.len() && dst_idx + 2 < rgb_data.len() {
                    rgb_data[dst_idx] = frame.rgb_data[src_idx];
                    rgb_data[dst_idx + 1] = frame.rgb_data[src_idx + 1];
                    rgb_data[dst_idx + 2] = frame.rgb_data[src_idx + 2];
                }
            }
        }

        Thumbnail {
            frame_index: frame.index,
            rgb_data,
            width: target_width,
            height: target_height,
        }
    }

    /// Get thumbnail if cached
    pub fn get(&self, frame_index: usize) -> Option<&Thumbnail> {
        self.thumbnails.get(&frame_index)
    }

    /// Check if thumbnail is cached
    pub fn contains(&self, frame_index: usize) -> bool {
        self.thumbnails.contains_key(&frame_index)
    }

    /// Insert thumbnail with LRU eviction
    pub fn insert(&mut self, thumbnail: Thumbnail) {
        let frame_index = thumbnail.frame_index;

        // Remove from LRU order if already present
        if let Some(pos) = self.lru_order.iter().position(|&idx| idx == frame_index) {
            self.lru_order.remove(pos);
        }

        // Evict oldest if at capacity
        while self.thumbnails.len() >= self.max_cache_size && !self.lru_order.is_empty() {
            let oldest = self.lru_order.remove(0);
            self.thumbnails.remove(&oldest);
        }

        // Insert new thumbnail
        self.thumbnails.insert(frame_index, thumbnail);
        self.lru_order.push(frame_index);
    }

    /// Touch a thumbnail to mark it as recently used
    pub fn touch(&mut self, frame_index: usize) {
        if let Some(pos) = self.lru_order.iter().position(|&idx| idx == frame_index) {
            self.lru_order.remove(pos);
            self.lru_order.push(frame_index);
        }
    }

    /// Get the number of cached thumbnails
    pub fn len(&self) -> usize {
        self.thumbnails.len()
    }

    /// Check if cache is empty
    pub fn is_empty(&self) -> bool {
        self.thumbnails.is_empty()
    }

    /// Clear the cache
    pub fn clear(&mut self) {
        self.thumbnails.clear();
        self.lru_order.clear();
    }

    /// Calculate visible frame range for virtualization
    /// Returns (start_index, end_index) inclusive
    pub fn visible_range(
        scroll_offset: f32,
        visible_width: f32,
        thumb_width: f32,
        spacing: f32,
        total_frames: usize,
    ) -> (usize, usize) {
        if total_frames == 0 {
            return (0, 0);
        }

        let item_width = thumb_width + spacing;

        // Start index (with some padding for smooth scrolling)
        let start = ((scroll_offset / item_width).floor() as isize - 2).max(0) as usize;

        // End index (with padding)
        let visible_count = (visible_width / item_width).ceil() as usize + 4;
        let end = (start + visible_count).min(total_frames.saturating_sub(1));

        (start, end)
    }
}

/// Per generate-tests skill: Comprehensive test suite with Arrange-Act-Assert pattern
#[allow(
    unused_imports,
    unused_variables,
    unused_mut,
    dead_code,
    unused_comparisons,
    unused_must_use,
    hidden_glob_reexports,
    unreachable_code,
    non_camel_case_types,
    unused_parens,
    unused_assignments
)]
#[cfg(test)]
mod tests {
    include!("filmstrip_test.rs");
}
