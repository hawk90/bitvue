//! Decode Service - Frame decoding business logic
//!
//! Caches file data and decoded frames to avoid repeated operations.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::collections::{HashMap, VecDeque};

// Import path validation from commands module
use crate::commands::file::validate_file_path;

/// Helper macro to safely lock mutexes with proper error handling
/// Prevents panic on mutex poisoning by returning an error instead
macro_rules! lock_mutex {
    ($mutex:expr) => {
        $mutex.lock().map_err(|e| format!("Mutex poisoned: {}", e))?
    };
}

/// Cached decoded RGB frame data
#[derive(Debug, Clone)]
struct CachedDecodedFrame {
    width: u32,
    height: u32,
    rgb_data: Arc<Vec<u8>>,  // Arc to avoid expensive clones
    cached_at: std::time::Instant,  // When this frame was cached (for LRU eviction)
}

/// Cached decoded YUV frame data (separate cache for efficiency)
#[derive(Debug, Clone)]
struct CachedYUVFrame {
    width: u32,
    height: u32,
    bit_depth: u8,
    y_plane: Vec<u8>,
    u_plane: Option<Vec<u8>>,
    v_plane: Option<Vec<u8>>,
    y_stride: usize,
    u_stride: usize,
    v_stride: usize,
    timestamp: i64,  // Frame timestamp from video
    frame_type: bitvue_decode::FrameType,
    qp_avg: Option<u8>,
    cached_at: std::time::Instant,  // When this frame was cached (for LRU eviction)
}

/// Decode service for handling frame operations with file and frame caching
pub struct DecodeService {
    /// Current file path
    file_path: Option<PathBuf>,
    /// Codec type
    codec: String,
    /// Cached file data (to avoid repeated disk reads)
    /// Using Arc to avoid expensive Vec clones
    cached_data: Mutex<Option<Arc<Vec<u8>>>>,
    /// Cached decoded RGB frames (frame_index -> decoded frame)
    decoded_frames_cache: Mutex<HashMap<usize, CachedDecodedFrame>>,
    /// Cached decoded YUV frames (separate cache for YUV requests)
    yuv_frames_cache: Mutex<HashMap<usize, CachedYUVFrame>>,
    /// LRU tracking for RGB frames (front = oldest, back = newest)
    rgb_lru_order: Mutex<VecDeque<usize>>,
    /// LRU tracking for YUV frames (front = oldest, back = newest)
    yuv_lru_order: Mutex<VecDeque<usize>>,
    /// Maximum cache size in bytes (prevents memory issues with 4K/8K frames)
    max_cache_bytes: usize,
    /// Current cache size in bytes
    current_cache_bytes: Mutex<usize>,
    /// Cached MP4 samples (extracted AV1 samples from MP4 container)
    mp4_samples_cache: Mutex<Option<Vec<Vec<u8>>>>,
    /// Cached MKV samples (extracted AV1 samples from MKV container)
    mkv_samples_cache: Mutex<Option<Vec<Vec<u8>>>>,
    /// Cached parsed IVF frames (header + frame offsets)
    ivf_frames_cache: Mutex<Option<(bitvue_av1::IvfHeader, Vec<bitvue_av1::IvfFrame>)>>,
}

impl DecodeService {
    /// Create a new decode service
    pub fn new() -> Self {
        Self {
            file_path: None,
            codec: String::new(),
            cached_data: Mutex::new(None),
            decoded_frames_cache: Mutex::new(HashMap::new()),
            yuv_frames_cache: Mutex::new(HashMap::new()),
            rgb_lru_order: Mutex::new(VecDeque::new()),
            yuv_lru_order: Mutex::new(VecDeque::new()),
            max_cache_bytes: 512 * 1024 * 1024, // 512MB default cache limit
            current_cache_bytes: Mutex::new(0),
            mp4_samples_cache: Mutex::new(None),
            mkv_samples_cache: Mutex::new(None),
            ivf_frames_cache: Mutex::new(None),
        }
    }

    /// Set the file for decoding and cache its data
    pub fn set_file(&mut self, path: PathBuf, codec: String) -> Result<(), String> {
        // Clear previous cache and reset byte counter
        *lock_mutex!(self.cached_data) = None;
        lock_mutex!(self.decoded_frames_cache).clear();
        lock_mutex!(self.yuv_frames_cache).clear();
        lock_mutex!(self.rgb_lru_order).clear();
        lock_mutex!(self.yuv_lru_order).clear();
        *lock_mutex!(self.current_cache_bytes) = 0;
        *lock_mutex!(self.mp4_samples_cache) = None;
        *lock_mutex!(self.mkv_samples_cache) = None;
        *lock_mutex!(self.ivf_frames_cache) = None;

        self.file_path = Some(path.clone());
        self.codec = codec;

        // Pre-load file data into cache for faster access
        let file_data = std::fs::read(&path)
            .map_err(|e| format!("Failed to read file: {}", e))?;

        *lock_mutex!(self.cached_data) = Some(Arc::new(file_data));

        Ok(())
    }

    /// Set the file for decoding with already-read data (avoids re-reading from disk)
    pub fn set_file_with_data(&mut self, path: PathBuf, codec: String, file_data: Vec<u8>) -> Result<(), String> {
        // Clear previous cache and reset byte counter
        *lock_mutex!(self.cached_data) = None;
        lock_mutex!(self.decoded_frames_cache).clear();
        lock_mutex!(self.yuv_frames_cache).clear();
        lock_mutex!(self.rgb_lru_order).clear();
        lock_mutex!(self.yuv_lru_order).clear();
        *lock_mutex!(self.current_cache_bytes) = 0;
        *lock_mutex!(self.mp4_samples_cache) = None;
        *lock_mutex!(self.mkv_samples_cache) = None;
        *lock_mutex!(self.ivf_frames_cache) = None;

        self.file_path = Some(path);
        self.codec = codec;

        // Use provided data instead of reading from disk
        *lock_mutex!(self.cached_data) = Some(Arc::new(file_data));

        Ok(())
    }

    /// Get cached file data as Arc slice (cheap clone)
    /// Returns Arc<Vec<u8>> which can be cheaply cloned and dereferenced to &[u8]
    pub fn get_file_data_arc(&self) -> Result<Arc<Vec<u8>>, String> {
        // Check cache first
        if let Some(data) = lock_mutex!(self.cached_data).as_ref() {
            return Ok(data.clone());
        }

        // If not cached, try to read from disk (for backward compatibility)
        let path = self.file_path.as_ref()
            .ok_or("No file loaded")?;

        // SECURITY: Validate path before reading to prevent path traversal
        let path_str = path.to_str()
            .ok_or("Invalid path: unable to convert to string")?;
        let validated_path = validate_file_path(path_str)?;

        std::fs::read(&validated_path)
            .map_err(|e| format!("Failed to read file: {}", e))
            .map(|data| Arc::new(data))
    }

    /// Get cached file data, or read from disk if not cached
    /// Note: This clones the data. For better performance, use get_file_data_arc()
    pub fn get_file_data(&self) -> Result<Vec<u8>, String> {
        self.get_file_data_arc().map(|arc| (*arc).clone())
    }

    /// Get a cached decoded RGB frame, or decode it if not cached
    /// Returns Arc<Vec<u8>> for cheap cloning - caller can deref to &[u8] for most use cases
    pub fn get_or_decode_frame(&self, frame_index: usize, decode_fn: impl FnOnce(&[u8], usize) -> Result<(u32, u32, Vec<u8>), String>) -> Result<(u32, u32, Arc<Vec<u8>>), String> {
        // Check frame cache first
        if let Some(cached) = lock_mutex!(self.decoded_frames_cache).get(&frame_index) {
            // Update LRU position (move to back = most recently used)
            let mut lru_order = lock_mutex!(self.rgb_lru_order);
            if let Some(pos) = lru_order.iter().position(|&k| k == frame_index) {
                lru_order.remove(pos);
            }
            lru_order.push_back(frame_index);
            // Return Arc clone (cheap - just increments reference count)
            return Ok((cached.width, cached.height, cached.rgb_data.clone()));
        }

        // Not in cache, decode the frame
        let file_data = self.get_file_data_arc()?;
        let (width, height, rgb_data) = decode_fn(&file_data, frame_index)?;

        // Calculate frame size in bytes (using actual data size)
        let frame_size = rgb_data.len();

        // Cache the decoded frame with O(1) LRU eviction
        let mut cache = lock_mutex!(self.decoded_frames_cache);
        let mut lru_order = lock_mutex!(self.rgb_lru_order);
        let mut current_bytes = lock_mutex!(self.current_cache_bytes);

        // Evict oldest frames until there's space for the new frame (O(1) per eviction)
        while *current_bytes + frame_size > self.max_cache_bytes && !lru_order.is_empty() {
            if let Some(oldest_key) = lru_order.pop_front() {
                if let Some(removed_frame) = cache.remove(&oldest_key) {
                    *current_bytes -= removed_frame.rgb_data.len();
                }
            } else {
                break;
            }
        }

        // Add new frame to cache and LRU order (back = most recently used)
        // Wrap RGB data in Arc for cheap cloning
        let rgb_arc = Arc::new(rgb_data);
        cache.insert(frame_index, CachedDecodedFrame {
            width,
            height,
            rgb_data: rgb_arc.clone(),
            cached_at: std::time::Instant::now(),
        });
        lru_order.push_back(frame_index);
        *current_bytes += frame_size;

        Ok((width, height, rgb_arc))
    }

    /// Get a cached decoded YUV frame, or decode it if not cached
    pub fn get_or_decode_frame_yuv(&self, frame_index: usize, decode_fn: impl FnOnce(&[u8], usize) -> Result<bitvue_decode::DecodedFrame, String>) -> Result<bitvue_decode::DecodedFrame, String> {
        // Check YUV frame cache first
        if let Some(cached) = lock_mutex!(self.yuv_frames_cache).get(&frame_index) {
            // Update LRU position (move to back = most recently used)
            let mut lru_order = lock_mutex!(self.yuv_lru_order);
            if let Some(pos) = lru_order.iter().position(|&k| k == frame_index) {
                lru_order.remove(pos);
            }
            lru_order.push_back(frame_index);
            return Ok(bitvue_decode::DecodedFrame {
                width: cached.width,
                height: cached.height,
                bit_depth: cached.bit_depth,
                y_plane: cached.y_plane.clone(),
                u_plane: cached.u_plane.clone(),
                v_plane: cached.v_plane.clone(),
                y_stride: cached.y_stride,
                u_stride: cached.u_stride,
                v_stride: cached.v_stride,
                timestamp: cached.timestamp,
                frame_type: cached.frame_type,
                qp_avg: cached.qp_avg,
            });
        }

        // Not in cache, decode the frame
        let file_data = self.get_file_data_arc()?;
        let frame = decode_fn(&file_data, frame_index)?;

        // Calculate YUV frame size in bytes (Y + U + V planes)
        let y_size = frame.y_plane.len();
        let u_size = frame.u_plane.as_ref().map_or(0, |p| p.len());
        let v_size = frame.v_plane.as_ref().map_or(0, |p| p.len());
        let frame_size = y_size + u_size + v_size;

        // Cache the decoded YUV frame with O(1) LRU eviction
        let mut cache = lock_mutex!(self.yuv_frames_cache);
        let mut lru_order = lock_mutex!(self.yuv_lru_order);
        let mut current_bytes = lock_mutex!(self.current_cache_bytes);

        // Evict oldest frames until there's space for the new frame (O(1) per eviction)
        while *current_bytes + frame_size > self.max_cache_bytes && !lru_order.is_empty() {
            if let Some(oldest_key) = lru_order.pop_front() {
                if let Some(removed_frame) = cache.remove(&oldest_key) {
                    let removed_size = removed_frame.y_plane.len()
                        + removed_frame.u_plane.as_ref().map_or(0, |p| p.len())
                        + removed_frame.v_plane.as_ref().map_or(0, |p| p.len());
                    *current_bytes -= removed_size;
                }
            } else {
                break;
            }
        }

        // Add new frame to cache and LRU order (back = most recently used)
        cache.insert(frame_index, CachedYUVFrame {
            width: frame.width,
            height: frame.height,
            bit_depth: frame.bit_depth,
            y_plane: frame.y_plane.clone(),
            u_plane: frame.u_plane.clone(),
            v_plane: frame.v_plane.clone(),
            y_stride: frame.y_stride,
            u_stride: frame.u_stride,
            v_stride: frame.v_stride,
            timestamp: frame.timestamp,
            frame_type: frame.frame_type,
            qp_avg: frame.qp_avg,
            cached_at: std::time::Instant::now(),
        });
        lru_order.push_back(frame_index);
        *current_bytes += frame_size;

        Ok(frame)
    }

    /// Get cached MP4 samples, or extract them if not cached
    pub fn get_or_extract_mp4_samples(&self) -> Result<Option<Vec<Vec<u8>>>, String> {
        // Check cache first
        if let Some(samples) = lock_mutex!(self.mp4_samples_cache).as_ref() {
            return Ok(Some(samples.clone()));
        }

        // Not cached, try to extract
        let file_data = self.get_file_data_arc()?;
        match bitvue_formats::mp4::extract_av1_samples(&file_data) {
            Ok(samples) => {
                *lock_mutex!(self.mp4_samples_cache) = Some(samples.clone());
                Ok(Some(samples))
            }
            Err(_) => Ok(None), // Not an MP4 file or extraction failed
        }
    }

    /// Get cached MKV samples, or extract them if not cached
    pub fn get_or_extract_mkv_samples(&self) -> Result<Option<Vec<Vec<u8>>>, String> {
        // Check cache first
        if let Some(samples) = lock_mutex!(self.mkv_samples_cache).as_ref() {
            return Ok(Some(samples.clone()));
        }

        // Not cached, try to extract
        let file_data = self.get_file_data_arc()?;
        match bitvue_formats::mkv::extract_av1_samples(&file_data) {
            Ok(samples) => {
                *lock_mutex!(self.mkv_samples_cache) = Some(samples.clone());
                Ok(Some(samples))
            }
            Err(_) => Ok(None), // Not an MKV file or extraction failed
        }
    }

    /// Get cached IVF frames, or parse them if not cached
    /// Returns the parsed IVF header and frame information
    pub fn get_or_parse_ivf_frames(&self) -> Result<Option<(bitvue_av1::IvfHeader, Vec<bitvue_av1::IvfFrame>)>, String> {
        // Check cache first
        if let Some(frames) = lock_mutex!(self.ivf_frames_cache).as_ref() {
            return Ok(Some(frames.clone()));
        }

        // Not cached, try to parse
        let file_data = self.get_file_data_arc()?;
        match bitvue_av1::parse_ivf_frames(&file_data) {
            Ok(parsed) => {
                *lock_mutex!(self.ivf_frames_cache) = Some(parsed.clone());
                Ok(Some(parsed))
            }
            Err(_) => Ok(None), // Not an IVF file or parsing failed
        }
    }

    /// Clear all cached data
    pub fn clear_cache(&self) -> Result<(), String> {
        *lock_mutex!(self.cached_data) = None;
        lock_mutex!(self.decoded_frames_cache).clear();
        lock_mutex!(self.yuv_frames_cache).clear();
        lock_mutex!(self.rgb_lru_order).clear();
        lock_mutex!(self.yuv_lru_order).clear();
        *lock_mutex!(self.current_cache_bytes) = 0;
        *lock_mutex!(self.mp4_samples_cache) = None;
        *lock_mutex!(self.mkv_samples_cache) = None;
        *lock_mutex!(self.ivf_frames_cache) = None;
        Ok(())
    }
}

impl Default for DecodeService {
    fn default() -> Self {
        Self::new()
    }
}
