//! Thumbnail Service - Caching and optimized thumbnail generation
//!
//! Features:
//! - In-memory LRU cache for generated thumbnails
//! - Lazy loading (only decode what's needed)
//! - Consistent data format (raw data, no data URL prefix)

use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::Mutex;
use base64::Engine;

/// Maximum number of cached thumbnails (LRU)
const MAX_CACHE_SIZE: usize = 200;

/// Cached thumbnail data
#[derive(Debug, Clone)]
pub struct CachedThumbnail {
    /// Base64 encoded PNG/SVG data (without data: URL prefix)
    pub data: String,
    /// Thumbnail width
    #[allow(dead_code)]
    pub width: u32,
    /// Thumbnail height
    #[allow(dead_code)]
    pub height: u32,
    /// Frame type for placeholder generation
    #[allow(dead_code)]
    pub frame_type: String,
    /// Timestamp when cached (for LRU eviction)
    #[allow(dead_code)]
    pub cached_at: std::time::Instant,
}

/// Thumbnail cache entry with access tracking
#[derive(Debug)]
struct CacheEntry {
    thumbnail: CachedThumbnail,
    last_accessed: std::time::Instant,
}

/// Thumbnail service with LRU caching
pub struct ThumbnailService {
    /// Cache: frame_index -> cached thumbnail
    cache: Mutex<HashMap<usize, CacheEntry>>,
    /// LRU tracking: frame indices in access order
    lru_queue: Mutex<VecDeque<usize>>,
    /// Current file path (to invalidate cache on file change)
    current_file: Mutex<Option<PathBuf>>,
    /// Thumbnail width
    thumb_width: u32,
    /// Thumbnail height
    pub thumb_height: u32,
}

impl ThumbnailService {
    /// Create a new thumbnail service
    pub fn new() -> Self {
        Self {
            cache: Mutex::new(HashMap::new()),
            lru_queue: Mutex::new(VecDeque::new()),
            current_file: Mutex::new(None),
            thumb_width: 80,
            thumb_height: 45,
        }
    }

    /// Set thumbnail dimensions
    #[allow(dead_code)]
    pub fn set_dimensions(&mut self, width: u32, height: u32) {
        self.thumb_width = width;
        self.thumb_height = height;
        // Clear cache when dimensions change
        self.clear_cache();
    }

    /// Set the current file (invalidates cache if file changed)
    pub fn set_file(&self, path: PathBuf) -> bool {
        let mut current = self.current_file.lock().unwrap();
        if current.as_ref() != Some(&path) {
            *current = Some(path);
            drop(current);
            self.clear_cache();
            return true;
        }
        false
    }

    /// Get cached thumbnail if available
    pub fn get_cached(&self, frame_index: usize) -> Option<String> {
        let mut cache = self.cache.lock().unwrap();
        let mut lru = self.lru_queue.lock().unwrap();

        if let Some(entry) = cache.get_mut(&frame_index) {
            entry.last_accessed = std::time::Instant::now();

            // Move to end of LRU queue (most recently used)
            lru.retain(|x| *x != frame_index);
            lru.push_back(frame_index);

            Some(entry.thumbnail.data.clone())
        } else {
            None
        }
    }

    /// Cache a thumbnail
    pub fn cache_thumbnail(&self, frame_index: usize, data: String, frame_type: String) {
        let mut cache = self.cache.lock().unwrap();
        let mut lru = self.lru_queue.lock().unwrap();

        let now = std::time::Instant::now();

        // Check if we need to evict
        if cache.len() >= MAX_CACHE_SIZE && !cache.contains_key(&frame_index) {
            // Evict least recently used
            if let Some(lru_idx) = lru.pop_front() {
                cache.remove(&lru_idx);
            }
        }

        // Add/update entry
        cache.insert(frame_index, CacheEntry {
            thumbnail: CachedThumbnail {
                data,
                width: self.thumb_width,
                height: self.thumb_height,
                frame_type,
                cached_at: now,
            },
            last_accessed: now,
        });

        // Update LRU queue
        lru.retain(|x| *x != frame_index);
        lru.push_back(frame_index);
    }

    /// Get multiple cached thumbnails at once
    #[allow(dead_code)]
    pub fn get_cached_batch(&self, indices: &[usize]) -> Vec<Option<String>> {
        indices.iter().map(|&idx| self.get_cached(idx)).collect()
    }

    /// Clear all cached thumbnails
    pub fn clear_cache(&self) {
        self.cache.lock().unwrap().clear();
        self.lru_queue.lock().unwrap().clear();
    }

    /// Get cache statistics
    #[allow(dead_code)]
    pub fn cache_stats(&self) -> (usize, usize) {
        let cache = self.cache.lock().unwrap();
        (cache.len(), MAX_CACHE_SIZE)
    }

    /// Preload thumbnails for a range of frames
    /// Returns indices that need to be generated
    #[allow(dead_code)]
    pub fn get_missing_indices(&self, indices: &[usize]) -> Vec<usize> {
        let cache = self.cache.lock().unwrap();
        indices.iter().filter(|&&idx| !cache.contains_key(&idx)).copied().collect()
    }
}

impl Default for ThumbnailService {
    fn default() -> Self {
        Self::new()
    }
}

/// Create a placeholder SVG thumbnail
pub fn create_svg_thumbnail(frame_type: &str) -> String {
    let color = match frame_type.to_uppercase().as_str() {
        "KEY" | "I" => "#e03131",
        "INTER" | "P" => "#2da44e",
        "B" => "#1f77b4",
        _ => "#666666",
    };

    let svg = format!(
        r#"<svg width="80" height="45" xmlns="http://www.w3.org/2000/svg">
        <rect width="80" height="45" fill="{}"/>
        <text x="50%" y="50%" text-anchor="middle" dy=".3em" fill="white" font-size="10" font-family="sans-serif">{}</text>
    </svg>"#,
        color, frame_type
    );

    // Return as proper data URL with base64 encoding
    let base64_data = base64::engine::general_purpose::STANDARD.encode(svg);
    format!("data:image/svg+xml;base64,{}", base64_data)
}

/// Decode AV1 frames and generate thumbnails (optimized version)
#[allow(dead_code)]
pub fn decode_av1_thumbnails(
    file_data: &[u8],
    frame_indices: &[usize],
    width: u32,
    height: u32,
) -> Result<Vec<(usize, String, String)>, String> {
    use bitvue_decode::{Av1Decoder, FrameType};

    let mut decoder = Av1Decoder::new().map_err(|e| format!("Failed to create decoder: {}", e))?;
    let decoded_frames = decoder.decode_all(file_data)
        .map_err(|e| format!("Failed to decode: {}", e))?;

    let mut results = Vec::new();

    for &idx in frame_indices {
        if idx < decoded_frames.len() {
            let frame = &decoded_frames[idx];

            // Convert YUV to RGB
            let rgb_data = bitvue_decode::yuv_to_rgb(frame);

            // Resize to thumbnail size
            let resized_rgb = resize_rgb(&rgb_data, frame.width, frame.height, width, height);

            // Create PNG base64 (without data: URL prefix)
            let png_base64 = create_png_base64(&resized_rgb, width, height)?;

            // Convert FrameType to String
            let frame_type_str = match &frame.frame_type {
                FrameType::Key => "KEY",
                FrameType::Inter => "P",
                FrameType::Intra => "I",
                FrameType::Switch => "S",
            }.to_string();

            results.push((idx, png_base64, frame_type_str));
        }
    }

    Ok(results)
}

/// Resize RGB data using nearest neighbor scaling
#[allow(dead_code)]
fn resize_rgb(
    rgb_data: &[u8],
    src_width: u32,
    src_height: u32,
    dst_width: u32,
    dst_height: u32,
) -> Vec<u8> {
    let mut resized = vec![0u8; (dst_width * dst_height * 3) as usize];

    for dy in 0..dst_height {
        for dx in 0..dst_width {
            let sx = (dx * src_width) / dst_width;
            let sy = (dy * src_height) / dst_height;
            let src_idx = (sy * src_width + sx) as usize * 3;
            let dst_idx = (dy * dst_width + dx) as usize * 3;

            if src_idx + 3 <= rgb_data.len() && dst_idx + 3 <= resized.len() {
                resized[dst_idx] = rgb_data[src_idx];
                resized[dst_idx + 1] = rgb_data[src_idx + 1];
                resized[dst_idx + 2] = rgb_data[src_idx + 2];
            }
        }
    }

    resized
}

/// Create PNG base64 (without data: URL prefix)
#[allow(dead_code)]
fn create_png_base64(rgb_data: &[u8], width: u32, height: u32) -> Result<String, String> {
    use image::{ImageBuffer, RgbImage};

    let img: RgbImage = ImageBuffer::from_raw(width, height, rgb_data.to_vec())
        .ok_or("Failed to create image buffer")?;

    let mut png_bytes = Vec::new();
    img.write_to(&mut std::io::Cursor::new(&mut png_bytes), image::ImageFormat::Png)
        .map_err(|e| format!("Failed to encode PNG: {}", e))?;

    Ok(base64::engine::general_purpose::STANDARD.encode(&png_bytes))
}
