//! Decode Service - Frame decoding business logic
//!
//! Caches file data and decoded frames to avoid repeated operations.

use std::path::PathBuf;
use std::sync::{Arc, Mutex, atomic::{AtomicU64, Ordering}};
use std::collections::{HashMap, VecDeque};

// Import path validation from commands module
use crate::commands::file::validate_file_path;

// Import shared utilities
use crate::services::utils::lock_mutex;

// Import ChromaFormat from bitvue_decode
use bitvue_decode::decoder::ChromaFormat;

/// Cached decoded RGB frame data
#[derive(Debug, Clone)]
struct CachedDecodedFrame {
    width: u32,
    height: u32,
    rgb_data: Arc<Vec<u8>>,  // Arc to avoid expensive clones
    generation: u64,  // Cache generation to prevent stale data
    #[allow(dead_code)]
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
    #[allow(dead_code)]
    cached_at: std::time::Instant,  // When this frame was cached (for LRU eviction)
}

/// Combined RGB cache state protected by single mutex
///
/// This structure groups the RGB cache data, LRU order, and byte counter
/// together to prevent race conditions when accessing the cache.
struct RGBCacheState {
    cache: HashMap<usize, CachedDecodedFrame>,
    lru_order: VecDeque<usize>,
    current_bytes: usize,
}

/// Combined YUV cache state protected by single mutex
///
/// This structure groups the YUV cache data, LRU order, and byte counter
/// together to prevent race conditions when accessing the cache.
struct YUVCacheState {
    cache: HashMap<usize, CachedYUVFrame>,
    lru_order: VecDeque<usize>,
    current_bytes: usize,
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
    /// Combined RGB cache state (single mutex prevents race conditions)
    rgb_cache_state: Mutex<RGBCacheState>,
    /// Combined YUV cache state (single mutex prevents race conditions)
    yuv_cache_state: Mutex<YUVCacheState>,
    /// Maximum cache size in bytes (prevents memory issues with 4K/8K frames)
    max_cache_bytes: usize,
    /// Cache generation counter - increments on file change to prevent stale cache data
    cache_generation: AtomicU64,
    /// Cached MP4 samples (extracted AV1 samples from MP4 container)
    mp4_samples_cache: Mutex<Option<Vec<Vec<u8>>>>,
    /// Cached MKV samples (extracted AV1 samples from MKV container)
    mkv_samples_cache: Mutex<Option<Vec<Vec<u8>>>>,
    /// Cached parsed IVF frames (header + frame offsets)
    ivf_frames_cache: Mutex<Option<(bitvue_av1_codec::IvfHeader, Vec<bitvue_av1_codec::IvfFrame>)>>,
}

impl DecodeService {
    /// Create a new decode service
    pub fn new() -> Self {
        Self {
            file_path: None,
            codec: String::new(),
            cached_data: Mutex::new(None),
            rgb_cache_state: Mutex::new(RGBCacheState {
                cache: HashMap::new(),
                lru_order: VecDeque::new(),
                current_bytes: 0,
            }),
            yuv_cache_state: Mutex::new(YUVCacheState {
                cache: HashMap::new(),
                lru_order: VecDeque::new(),
                current_bytes: 0,
            }),
            // SAFETY: Use usize cast to prevent integer overflow (512 * 1024 * 1024 = 512MB)
            max_cache_bytes: (512 * 1024 * 1024) as usize, // 512MB default cache limit
            mp4_samples_cache: Mutex::new(None),
            mkv_samples_cache: Mutex::new(None),
            ivf_frames_cache: Mutex::new(None),
            cache_generation: AtomicU64::new(0),
        }
    }

    /// Set the file for decoding and cache its data
    #[allow(dead_code)]
    pub fn set_file(&mut self, path: PathBuf, codec: String) -> Result<(), String> {
        // Clear previous cache and reset byte counter
        *lock_mutex!(self.cached_data) = None;
        *lock_mutex!(self.rgb_cache_state) = RGBCacheState {
            cache: HashMap::new(),
            lru_order: VecDeque::new(),
            current_bytes: 0,
        };
        *lock_mutex!(self.yuv_cache_state) = YUVCacheState {
            cache: HashMap::new(),
            lru_order: VecDeque::new(),
            current_bytes: 0,
        };
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
        *lock_mutex!(self.rgb_cache_state) = RGBCacheState {
            cache: HashMap::new(),
            lru_order: VecDeque::new(),
            current_bytes: 0,
        };
        *lock_mutex!(self.yuv_cache_state) = YUVCacheState {
            cache: HashMap::new(),
            lru_order: VecDeque::new(),
            current_bytes: 0,
        };
        *lock_mutex!(self.mp4_samples_cache) = None;
        *lock_mutex!(self.mkv_samples_cache) = None;
        *lock_mutex!(self.ivf_frames_cache) = None;

        self.file_path = Some(path);
        self.codec = codec;

        // Use provided data instead of reading from disk
        *lock_mutex!(self.cached_data) = Some(Arc::new(file_data));

        // Increment cache generation to invalidate stale cache entries
        self.cache_generation.fetch_add(1, Ordering::SeqCst);

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
    ///
    /// Uses combined cache state with single mutex to prevent race conditions
    pub fn get_or_decode_frame(&self, frame_index: usize, decode_fn: impl FnOnce(&[u8], usize) -> Result<(u32, u32, Vec<u8>), String>) -> Result<(u32, u32, Arc<Vec<u8>>), String> {
        // Get current cache generation
        let current_generation = self.cache_generation.load(Ordering::SeqCst);

        // Single lock acquisition for cache state (prevents race conditions)
        let mut state = lock_mutex!(self.rgb_cache_state);

        // Check frame cache first
        if let Some(cached) = state.cache.get(&frame_index) {
            // SECURITY: Check generation matches to prevent stale cache data
            if cached.generation == current_generation {
                // Clone values before mutable borrow
                let width = cached.width;
                let height = cached.height;
                let rgb_data = cached.rgb_data.clone();

                // Update LRU position (move to back = most recently used)
                state.lru_order.retain(|&k| k != frame_index);
                state.lru_order.push_back(frame_index);

                // Return Arc clone (cheap - just increments reference count)
                return Ok((width, height, rgb_data));
            }
            // Stale cache entry, remove it
            log::debug!("Removing stale cache entry for frame {}", frame_index);
            state.cache.remove(&frame_index);
        }

        // Not in cache - need to decode
        // Release lock during decode (expensive operation)
        drop(state);

        // Not in cache, decode the frame
        let file_data = self.get_file_data_arc()?;
        let (width, height, rgb_data) = decode_fn(&file_data, frame_index)?;

        // Calculate frame size in bytes (using actual data size)
        let frame_size = rgb_data.len();

        // Re-acquire lock to update cache
        let mut state = lock_mutex!(self.rgb_cache_state);

        // Evict oldest frames until there's space for the new frame (O(1) per eviction)
        // SECURITY: Use saturating_sub to prevent underflow
        while state.current_bytes + frame_size > self.max_cache_bytes && !state.lru_order.is_empty() {
            if let Some(oldest_key) = state.lru_order.pop_front() {
                if let Some(removed_frame) = state.cache.remove(&oldest_key) {
                    let removed_size = removed_frame.rgb_data.len();
                    // SECURITY: Prevent underflow with saturating_sub
                    state.current_bytes = state.current_bytes.saturating_sub(removed_size);

                    // Sanity check: log if state is inconsistent
                    if removed_size > state.current_bytes + frame_size {
                        log::warn!("Cache state inconsistency: removed {} bytes but only had {} tracked",
                            removed_size, state.current_bytes);
                    }
                }
            } else {
                break;
            }
        }

        // Add new frame to cache and LRU order (back = most recently used)
        // Wrap RGB data in Arc for cheap cloning
        let rgb_arc = Arc::new(rgb_data);
        state.cache.insert(frame_index, CachedDecodedFrame {
            width,
            height,
            rgb_data: rgb_arc.clone(),
            generation: current_generation,
            cached_at: std::time::Instant::now(),
        });
        state.lru_order.push_back(frame_index);
        state.current_bytes += frame_size;

        Ok((width, height, rgb_arc))
    }

    /// Get a cached decoded RGB frame, or decode it if not cached, with prefetching
    ///
    /// Similar to get_or_decode_frame, but also prefetches subsequent frames
    /// to improve performance for sequential access patterns (e.g., video playback).
    ///
    /// # Arguments
    /// * `frame_index` - The frame to retrieve
    /// * `decode_fn` - Function to decode the frame if not cached (must be Fn for multiple calls)
    /// * `prefetch_count` - Number of subsequent frames to prefetch (default: 3)
    ///
    /// # Performance
    /// This is beneficial for video playback where frames are typically accessed
    /// sequentially. By prefetching, the next frames will be ready when needed,
    /// reducing latency.
    pub fn get_or_decode_frame_with_prefetch<F>(
        &self,
        frame_index: usize,
        decode_fn: F,
        prefetch_count: usize,
    ) -> Result<(u32, u32, Arc<Vec<u8>>), String>
    where
        F: Fn(&[u8], usize) -> Result<(u32, u32, Vec<u8>), String>,
    {
        use crate::constants::cache;

        // Get current cache generation
        let current_generation = self.cache_generation.load(Ordering::SeqCst);

        // First, get the requested frame (this may or may not require decoding)
        let result = self.get_or_decode_frame(frame_index, &decode_fn)?;

        // Prefetch subsequent frames in background if enabled
        if prefetch_count > 0 {
            let file_data = self.get_file_data_arc()?;

            // Prefetch up to the specified number of frames
            for idx in (frame_index + 1)..=(frame_index + prefetch_count) {
                // Check if frame is already cached
                let mut state = lock_mutex!(self.rgb_cache_state);

                if state.cache.contains_key(&idx) {
                    // Already cached, skip
                    continue;
                }

                // Frame not cached - need to decode
                // Release lock before expensive decode operation
                drop(state);

                // Attempt to decode this frame (fire-and-forget, ignore errors)
                if let Ok((width, height, rgb_data)) = decode_fn(&file_data, idx) {
                    let frame_size = rgb_data.len();

                    // Re-acquire lock to cache the prefetched frame
                    let mut state = lock_mutex!(self.rgb_cache_state);

                    // Check if we have space for this frame
                    // Evict if necessary, but be more conservative with prefetching
                    while state.current_bytes + frame_size > self.max_cache_bytes && !state.lru_order.is_empty() {
                        if let Some(oldest_key) = state.lru_order.pop_front() {
                            if let Some(removed_frame) = state.cache.remove(&oldest_key) {
                                state.current_bytes -= removed_frame.rgb_data.len();
                            }
                        } else {
                            // Cache is full, stop prefetching
                            break;
                        }
                    }

                    // Only cache if we have enough space (prefer eviction of old frames over eviction of prefetches)
                    if state.current_bytes + frame_size <= self.max_cache_bytes {
                        let rgb_arc = Arc::new(rgb_data);
                        state.cache.insert(idx, CachedDecodedFrame {
                            width,
                            height,
                            rgb_data: rgb_arc.clone(),
                            generation: current_generation,
                            cached_at: std::time::Instant::now(),
                        });
                        state.lru_order.push_back(idx);
                        state.current_bytes += frame_size;
                    }

                    // Log prefetch activity (debug level only, no sensitive info)
                    log::debug!("Prefetched frame {}", idx);
                } else {
                    // Decode failed, stop prefetching
                    log::debug!("Failed to prefetch frame {}, stopping prefetch", idx);
                    break;
                }
            }
        }

        Ok(result)
    }

    /// Get a cached decoded YUV frame, or decode it if not cached
    ///
    /// Uses combined cache state with single mutex to prevent race conditions
    pub fn get_or_decode_frame_yuv(&self, frame_index: usize, decode_fn: impl FnOnce(&[u8], usize) -> Result<bitvue_decode::DecodedFrame, String>) -> Result<bitvue_decode::DecodedFrame, String> {
        // Single lock acquisition for cache state (prevents race conditions)
        let mut state = lock_mutex!(self.yuv_cache_state);

        // Check YUV frame cache first
        if let Some(cached) = state.cache.get(&frame_index) {
            // Clone values before mutable borrow
            let width = cached.width;
            let height = cached.height;
            let bit_depth = cached.bit_depth;
            let y_plane = cached.y_plane.clone();
            let u_plane = cached.u_plane.clone();
            let v_plane = cached.v_plane.clone();
            let y_stride = cached.y_stride;
            let u_stride = cached.u_stride;
            let v_stride = cached.v_stride;
            let timestamp = cached.timestamp;
            let frame_type = cached.frame_type;
            let qp_avg = cached.qp_avg;

            // Update LRU position (move to back = most recently used)
            state.lru_order.retain(|&k| k != frame_index);
            state.lru_order.push_back(frame_index);

            return Ok(bitvue_decode::DecodedFrame {
                width,
                height,
                bit_depth,
                y_plane: std::sync::Arc::new(y_plane),
                u_plane: u_plane.clone().map(std::sync::Arc::new),
                v_plane: v_plane.clone().map(std::sync::Arc::new),
                y_stride,
                u_stride,
                v_stride,
                timestamp,
                frame_type,
                qp_avg,
                chroma_format: ChromaFormat::from_frame_data(
                    width,
                    height,
                    bit_depth,
                    u_plane.as_deref(),
                    v_plane.as_deref(),
                ),
            });
        }

        // Not in cache - need to decode
        // Release lock during decode (expensive operation)
        drop(state);

        // Not in cache, decode the frame
        let file_data = self.get_file_data_arc()?;
        let frame = decode_fn(&file_data, frame_index)?;

        // Calculate YUV frame size in bytes (Y + U + V planes)
        let y_size = frame.y_plane.len();
        let u_size = frame.u_plane.as_ref().map_or(0, |p| p.len());
        let v_size = frame.v_plane.as_ref().map_or(0, |p| p.len());
        let frame_size = y_size + u_size + v_size;

        // Re-acquire lock to update cache
        let mut state = lock_mutex!(self.yuv_cache_state);

        // Evict oldest frames until there's space for the new frame (O(1) per eviction)
        while state.current_bytes + frame_size > self.max_cache_bytes && !state.lru_order.is_empty() {
            if let Some(oldest_key) = state.lru_order.pop_front() {
                if let Some(removed_frame) = state.cache.remove(&oldest_key) {
                    let removed_size = removed_frame.y_plane.len()
                        + removed_frame.u_plane.as_ref().map_or(0, |p| p.len())
                        + removed_frame.v_plane.as_ref().map_or(0, |p| p.len());
                    state.current_bytes -= removed_size;
                }
            } else {
                break;
            }
        }

        // Add new frame to cache and LRU order (back = most recently used)
        state.cache.insert(frame_index, CachedYUVFrame {
            width: frame.width,
            height: frame.height,
            bit_depth: frame.bit_depth,
            y_plane: (*frame.y_plane).clone(),
            u_plane: frame.u_plane.as_ref().map(|p| (**p).clone()),
            v_plane: frame.v_plane.as_ref().map(|p| (**p).clone()),
            y_stride: frame.y_stride,
            u_stride: frame.u_stride,
            v_stride: frame.v_stride,
            timestamp: frame.timestamp,
            frame_type: frame.frame_type,
            qp_avg: frame.qp_avg,
            cached_at: std::time::Instant::now(),
        });
        state.lru_order.push_back(frame_index);
        state.current_bytes += frame_size;

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
                let owned_samples: Vec<Vec<u8>> = samples.into_iter().map(|cow| cow.to_vec()).collect();
                *lock_mutex!(self.mp4_samples_cache) = Some(owned_samples.clone());
                Ok(Some(owned_samples))
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
    #[allow(dead_code)]
    pub fn get_or_parse_ivf_frames(&self) -> Result<Option<(bitvue_av1_codec::IvfHeader, Vec<bitvue_av1_codec::IvfFrame>)>, String> {
        // Check cache first
        if let Some(frames) = lock_mutex!(self.ivf_frames_cache).as_ref() {
            return Ok(Some(frames.clone()));
        }

        // Not cached, try to parse
        let file_data = self.get_file_data_arc()?;
        match bitvue_av1_codec::parse_ivf_frames(&file_data) {
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
        *lock_mutex!(self.rgb_cache_state) = RGBCacheState {
            cache: HashMap::new(),
            lru_order: VecDeque::new(),
            current_bytes: 0,
        };
        *lock_mutex!(self.yuv_cache_state) = YUVCacheState {
            cache: HashMap::new(),
            lru_order: VecDeque::new(),
            current_bytes: 0,
        };
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
