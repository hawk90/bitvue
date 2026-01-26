//! Cache Provenance - T0-2
//!
//! Deliverable: cache_provenance_01_tracking
//!
//! Cache provenance system tracks:
//! - Where each cache entry came from (parameters)
//! - When it was created (timestamp)
//! - When it should be invalidated (trigger conditions)
//! - Eviction events for performance debugging
//!
//! Per CACHE_LEVELS_SPEC and CACHE_INVALIDATION_TABLE:
//! - Frame change invalidates frame-bound overlays
//! - Resolution change invalidates all textures
//! - Cache hit/miss visible in Dev HUD

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime};

/// Cache key type for different cache layers
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CacheKey {
    /// Decode cache key: frame_idx + decode_params
    Decode {
        frame_idx: usize,
        decode_params: String,
    },

    /// Texture cache key: frame_idx + resolution tier + colorspace
    Texture {
        frame_idx: usize,
        res_tier: u8,
        colorspace: String,
    },

    /// QP heatmap overlay key
    QpHeatmap {
        frame_idx: usize,
        hm_res: u32,
        scale_mode: String,
        qp_min: u8,
        qp_max: u8,
        opacity: u8,
    },

    /// Motion vector overlay key
    MvOverlay {
        frame_idx: usize,
        viewport_hash: u64,
        stride: u32,
        scale_x1000: u32, // scale * 1000 to avoid f32 in Hash
        opacity: u8,
    },

    /// Partition grid overlay key
    PartitionGrid {
        viewport_hash: u64,
        zoom_tier: u8,
        mode: String,
    },

    /// Diff heatmap key (A/B compare)
    DiffHeatmap {
        frame_idx_a: usize,
        frame_idx_b: usize,
        mode: String,
        ab_mapping: String,
        hm_res: u32,
    },

    /// Timeline visualization key
    Timeline {
        data_revision: u64,
        zoom_level_x100: u32, // zoom_level * 100 to avoid f32 in Hash
        filter_hash: u64,
    },
}

impl CacheKey {
    /// Get cache key type name
    pub fn type_name(&self) -> &'static str {
        match self {
            CacheKey::Decode { .. } => "Decode",
            CacheKey::Texture { .. } => "Texture",
            CacheKey::QpHeatmap { .. } => "QpHeatmap",
            CacheKey::MvOverlay { .. } => "MvOverlay",
            CacheKey::PartitionGrid { .. } => "PartitionGrid",
            CacheKey::DiffHeatmap { .. } => "DiffHeatmap",
            CacheKey::Timeline { .. } => "Timeline",
        }
    }

    /// Get frame index if applicable
    pub fn frame_idx(&self) -> Option<usize> {
        match self {
            CacheKey::Decode { frame_idx, .. } => Some(*frame_idx),
            CacheKey::Texture { frame_idx, .. } => Some(*frame_idx),
            CacheKey::QpHeatmap { frame_idx, .. } => Some(*frame_idx),
            CacheKey::MvOverlay { frame_idx, .. } => Some(*frame_idx),
            CacheKey::DiffHeatmap { frame_idx_a, .. } => Some(*frame_idx_a),
            _ => None,
        }
    }

    /// Check if this is a frame-bound cache entry
    pub fn is_frame_bound(&self) -> bool {
        self.frame_idx().is_some()
    }
}

/// Cache entry provenance information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheProvenance {
    /// Cache key
    pub key: CacheKey,

    /// When the entry was created
    pub created_at: SystemTime,

    /// Last access time
    pub last_accessed: SystemTime,

    /// Access count
    pub access_count: u64,

    /// Entry size in bytes
    pub size_bytes: usize,

    /// Source information (e.g., "parser", "decoder", "renderer")
    pub source: String,

    /// Whether this entry is still valid
    pub is_valid: bool,

    /// Invalidation reason (if invalidated)
    pub invalidation_reason: Option<String>,
}

impl CacheProvenance {
    /// Create new cache provenance
    pub fn new(key: CacheKey, size_bytes: usize, source: String) -> Self {
        let now = SystemTime::now();
        Self {
            key,
            created_at: now,
            last_accessed: now,
            access_count: 0,
            size_bytes,
            source,
            is_valid: true,
            invalidation_reason: None,
        }
    }

    /// Record an access to this cache entry
    pub fn record_access(&mut self) {
        self.last_accessed = SystemTime::now();
        self.access_count += 1;
    }

    /// Invalidate this cache entry
    pub fn invalidate(&mut self, reason: String) {
        self.is_valid = false;
        self.invalidation_reason = Some(reason);
    }

    /// Get age of this cache entry
    pub fn age(&self) -> Duration {
        SystemTime::now()
            .duration_since(self.created_at)
            .unwrap_or(Duration::from_secs(0))
    }

    /// Get time since last access
    pub fn time_since_access(&self) -> Duration {
        SystemTime::now()
            .duration_since(self.last_accessed)
            .unwrap_or(Duration::from_secs(0))
    }
}

/// Cache invalidation trigger
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InvalidationTrigger {
    /// Frame index changed
    FrameChanged(usize),

    /// Resolution changed
    ResolutionChanged,

    /// Viewport changed
    ViewportChanged,

    /// Zoom level changed
    ZoomChanged,

    /// Data was updated (timeline, etc.)
    DataRevision(u64),

    /// Manual invalidation
    Manual(String),
}

/// Cache provenance tracker
///
/// Tracks all cache entries across all cache layers with full provenance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheProvenanceTracker {
    /// All cache entries by key
    entries: HashMap<CacheKey, CacheProvenance>,

    /// Total cache size in bytes
    total_size: usize,

    /// Cache hit count
    pub hit_count: u64,

    /// Cache miss count
    pub miss_count: u64,

    /// Eviction count
    eviction_count: u64,

    /// Invalidation count
    invalidation_count: u64,
}

impl CacheProvenanceTracker {
    /// Create new cache provenance tracker
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            total_size: 0,
            hit_count: 0,
            miss_count: 0,
            eviction_count: 0,
            invalidation_count: 0,
        }
    }

    /// Add a cache entry
    pub fn add_entry(&mut self, key: CacheKey, size_bytes: usize, source: String) {
        let provenance = CacheProvenance::new(key.clone(), size_bytes, source);
        self.total_size += size_bytes;
        self.entries.insert(key, provenance);
    }

    /// Record a cache hit
    pub fn record_hit(&mut self, key: &CacheKey) {
        self.hit_count += 1;
        if let Some(entry) = self.entries.get_mut(key) {
            entry.record_access();
        }
    }

    /// Record a cache miss
    pub fn record_miss(&mut self, key: &CacheKey) {
        self.miss_count += 1;
        // Log which key was missed (for debugging)
        let _ = key;
    }

    /// Invalidate cache entries based on trigger
    pub fn invalidate(&mut self, trigger: InvalidationTrigger) {
        let reason = format!("{:?}", trigger);

        let keys_to_invalidate: Vec<CacheKey> = self
            .entries
            .iter()
            .filter(|(key, entry)| entry.is_valid && self.should_invalidate(key, &trigger))
            .map(|(key, _)| key.clone())
            .collect();

        for key in keys_to_invalidate {
            if let Some(entry) = self.entries.get_mut(&key) {
                entry.invalidate(reason.clone());
                self.invalidation_count += 1;
            }
        }
    }

    /// Check if a key should be invalidated by trigger
    fn should_invalidate(&self, key: &CacheKey, trigger: &InvalidationTrigger) -> bool {
        match trigger {
            InvalidationTrigger::FrameChanged(new_frame_idx) => {
                // Frame change invalidates all frame-bound caches
                if let Some(frame_idx) = key.frame_idx() {
                    frame_idx != *new_frame_idx
                } else {
                    false
                }
            }
            InvalidationTrigger::ResolutionChanged => {
                // Resolution change invalidates all texture caches
                matches!(
                    key,
                    CacheKey::Texture { .. }
                        | CacheKey::QpHeatmap { .. }
                        | CacheKey::DiffHeatmap { .. }
                )
            }
            InvalidationTrigger::ViewportChanged => {
                // Viewport change invalidates viewport-dependent caches
                matches!(
                    key,
                    CacheKey::MvOverlay { .. } | CacheKey::PartitionGrid { .. }
                )
            }
            InvalidationTrigger::ZoomChanged => {
                // Zoom change invalidates zoom-dependent caches
                matches!(
                    key,
                    CacheKey::PartitionGrid { .. } | CacheKey::Timeline { .. }
                )
            }
            InvalidationTrigger::DataRevision(new_revision) => {
                // Data revision invalidates timeline
                if let CacheKey::Timeline { data_revision, .. } = key {
                    data_revision != new_revision
                } else {
                    false
                }
            }
            InvalidationTrigger::Manual(_) => {
                // Manual invalidation affects all entries
                true
            }
        }
    }

    /// Evict a cache entry
    pub fn evict(&mut self, key: &CacheKey) -> bool {
        if let Some(entry) = self.entries.remove(key) {
            self.total_size -= entry.size_bytes;
            self.eviction_count += 1;
            true
        } else {
            false
        }
    }

    /// Find entries to evict based on LRU
    pub fn find_lru_eviction_candidates(&self, target_bytes: usize) -> Vec<CacheKey> {
        let mut candidates: Vec<_> = self
            .entries
            .iter()
            .filter(|(_, entry)| entry.is_valid)
            .map(|(key, entry)| (key.clone(), entry.time_since_access()))
            .collect();

        // Sort by time since access (oldest first)
        candidates.sort_by_key(|(_, time)| *time);

        // Collect keys until we reach target bytes
        let mut total = 0;
        let mut result = Vec::new();
        for (key, _) in candidates {
            if let Some(entry) = self.entries.get(&key) {
                total += entry.size_bytes;
                result.push(key);
                if total >= target_bytes {
                    break;
                }
            }
        }

        result
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        let valid_entries = self.entries.values().filter(|e| e.is_valid).count();
        let invalid_entries = self.entries.len() - valid_entries;

        let hit_rate = if self.hit_count + self.miss_count > 0 {
            self.hit_count as f64 / (self.hit_count + self.miss_count) as f64
        } else {
            0.0
        };

        CacheStats {
            total_entries: self.entries.len(),
            valid_entries,
            invalid_entries,
            total_size_bytes: self.total_size,
            hit_count: self.hit_count,
            miss_count: self.miss_count,
            eviction_count: self.eviction_count,
            invalidation_count: self.invalidation_count,
            hit_rate,
        }
    }

    /// Get all entries (for debugging)
    pub fn entries(&self) -> &HashMap<CacheKey, CacheProvenance> {
        &self.entries
    }

    /// Clear all entries
    pub fn clear(&mut self) {
        self.entries.clear();
        self.total_size = 0;
    }
}

impl Default for CacheProvenanceTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub total_entries: usize,
    pub valid_entries: usize,
    pub invalid_entries: usize,
    pub total_size_bytes: usize,
    pub hit_count: u64,
    pub miss_count: u64,
    pub eviction_count: u64,
    pub invalidation_count: u64,
    pub hit_rate: f64,
}

#[cfg(test)]
include!("cache_provenance_test.rs");
