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

/// Helper macro to safely lock mutexes with proper error handling
/// Prevents panic on mutex poisoning by returning an error instead
macro_rules! lock_mutex {
    ($mutex:expr) => {
        $mutex.lock().map_err(|e| format!("Mutex poisoned: {}", e))?
    };
}

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
///
/// THREAD SAFETY: cache and lru_queue are protected by a single mutex
/// to prevent race conditions. Access through lock_internal() helper.
pub struct ThumbnailService {
    /// Internal state protected by mutex (cache + LRU queue)
    state: Mutex<ThumbnailState>,
    /// Current file path (to invalidate cache on file change)
    current_file: Mutex<Option<PathBuf>>,
    /// Thumbnail width
    thumb_width: u32,
    /// Thumbnail height
    pub thumb_height: u32,
}

/// Internal state protected by a single mutex for thread safety
struct ThumbnailState {
    /// Cache: frame_index -> cached thumbnail
    cache: HashMap<usize, CacheEntry>,
    /// LRU tracking: frame indices in access order
    lru_queue: VecDeque<usize>,
}

impl ThumbnailService {
    /// Create a new thumbnail service
    pub fn new() -> Self {
        Self {
            state: Mutex::new(ThumbnailState {
                cache: HashMap::new(),
                lru_queue: VecDeque::new(),
            }),
            current_file: Mutex::new(None),
            thumb_width: 80,
            thumb_height: 45,
        }
    }

    /// Set thumbnail dimensions
    #[allow(dead_code)]
    pub fn set_dimensions(&mut self, width: u32, height: u32) -> Result<(), String> {
        // SECURITY: Validate dimensions to prevent DoS with extremely large thumbnails
        const MAX_THUMB_DIMENSION: u32 = 4096; // 4K max for thumbnails
        const MIN_THUMB_DIMENSION: u32 = 16;   // Minimum reasonable size

        if width < MIN_THUMB_DIMENSION || height < MIN_THUMB_DIMENSION {
            return Err(format!(
                "Thumbnail dimensions too small (minimum {}x{}, got {}x{})",
                MIN_THUMB_DIMENSION, MIN_THUMB_DIMENSION, width, height
            ));
        }

        if width > MAX_THUMB_DIMENSION || height > MAX_THUMB_DIMENSION {
            return Err(format!(
                "Thumbnail dimensions too large (maximum {}x{}, got {}x{})",
                MAX_THUMB_DIMENSION, MAX_THUMB_DIMENSION, width, height
            ));
        }

        self.thumb_width = width;
        self.thumb_height = height;
        // Clear cache when dimensions change
        self.clear_cache()
    }

    /// Set the current file (invalidates cache if file changed)
    pub fn set_file(&self, path: PathBuf) -> Result<bool, String> {
        let mut current = lock_mutex!(self.current_file);
        if current.as_ref() != Some(&path) {
            *current = Some(path);
            drop(current);
            self.clear_cache()?;
            return Ok(true);
        }
        Ok(false)
    }

    /// Get cached thumbnail if available
    pub fn get_cached(&self, frame_index: usize) -> Result<Option<String>, String> {
        let mut state = lock_mutex!(self.state);

        // Clone the data first to avoid borrow conflicts
        let data = if let Some(entry) = state.cache.get_mut(&frame_index) {
            entry.last_accessed = std::time::Instant::now();
            Some(entry.thumbnail.data.clone())
        } else {
            None
        };

        // Update LRU queue outside of the if-let
        if data.is_some() {
            state.lru_queue.retain(|x| *x != frame_index);
            state.lru_queue.push_back(frame_index);
        }

        Ok(data)
    }

    /// Cache a thumbnail
    pub fn cache_thumbnail(&self, frame_index: usize, data: String, frame_type: String) -> Result<(), String> {
        let mut state = lock_mutex!(self.state);

        let now = std::time::Instant::now();

        // Check if we need to evict
        if state.cache.len() >= MAX_CACHE_SIZE && !state.cache.contains_key(&frame_index) {
            // Evict least recently used
            if let Some(lru_idx) = state.lru_queue.pop_front() {
                state.cache.remove(&lru_idx);
            }
        }

        // Add/update entry
        state.cache.insert(frame_index, CacheEntry {
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
        state.lru_queue.retain(|x| *x != frame_index);
        state.lru_queue.push_back(frame_index);
        Ok(())
    }

    /// Get multiple cached thumbnails at once
    #[allow(dead_code)]
    pub fn get_cached_batch(&self, indices: &[usize]) -> Result<Vec<Option<String>>, String> {
        indices.iter().map(|&idx| self.get_cached(idx)).collect()
    }

    /// Clear all cached thumbnails
    pub fn clear_cache(&self) -> Result<(), String> {
        let mut state = lock_mutex!(self.state);
        state.cache.clear();
        state.lru_queue.clear();
        Ok(())
    }

    /// Get cache statistics
    #[allow(dead_code)]
    pub fn cache_stats(&self) -> Result<(usize, usize), String> {
        let state = lock_mutex!(self.state);
        Ok((state.cache.len(), MAX_CACHE_SIZE))
    }

    /// Preload thumbnails for a range of frames
    /// Returns indices that need to be generated
    #[allow(dead_code)]
    pub fn get_missing_indices(&self, indices: &[usize]) -> Result<Vec<usize>, String> {
        let state = lock_mutex!(self.state);
        Ok(indices.iter().filter(|&&idx| !state.cache.contains_key(&idx)).copied().collect())
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
            let resized_rgb = resize_rgb(&rgb_data, frame.width, frame.height, width, height)
                .map_err(|e| format!("Failed to resize frame {}: {}", idx, e))?;

            // Create PNG base64 (without data: URL prefix)
            let png_base64 = create_png_base64(&resized_rgb, width, height)
                .map_err(|e| format!("Failed to encode PNG for frame {}: {}", idx, e))?;

            // Convert FrameType to String
            let frame_type_str = match &frame.frame_type {
                FrameType::Key => "KEY",
                FrameType::Inter => "P",
                FrameType::Intra => "I",
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
) -> Result<Vec<u8>, String> {
    // Validate dimensions are not zero
    if dst_width == 0 || dst_height == 0 {
        return Err("Invalid dimensions: width and height must be non-zero".to_string());
    }

    // Check for overflow in buffer size calculation
    let output_size = dst_width
        .checked_mul(dst_height)
        .and_then(|v| v.checked_mul(3))
        .ok_or_else(|| {
            format!("Invalid dimensions: {}x{} causes integer overflow", dst_width, dst_height)
        })?;

    // Limit maximum output size to prevent DoS (100MB = ~31K x 31K image)
    const MAX_OUTPUT_SIZE: u32 = 100 * 1024 * 1024;
    if output_size > MAX_OUTPUT_SIZE {
        return Err(format!(
            "Output image too large: {}x{} ({} bytes). Maximum is 100MB.",
            dst_width, dst_height, output_size
        ));
    }

    let output_size = output_size as usize;

    // Limit maximum reasonable dimensions
    const MAX_DIMENSION: u32 = 32768; // 32K should be more than enough
    if dst_width > MAX_DIMENSION || dst_height > MAX_DIMENSION {
        return Err(format!(
            "Dimensions too large: {}x{}. Maximum is {}x{}.",
            dst_width, dst_height, MAX_DIMENSION, MAX_DIMENSION
        ));
    }

    let mut resized = vec![0u8; output_size];

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

    Ok(resized)
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
