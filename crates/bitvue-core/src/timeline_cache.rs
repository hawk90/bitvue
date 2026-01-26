//! Timeline Cache Provenance - T4-1 Cache Integration
//!
//! Deliverable: cache_proof_01:Foundations:Timeline:AV1:cache_provenance
//!
//! Timeline-specific cache provenance tracking for:
//! - Lane overlay rendering caches
//! - Marker cluster caches
//! - Viewport-dependent visualizations
//!
//! Per CACHE_INVALIDATION_TABLE:
//! - Frame change invalidates frame-bound caches
//! - Zoom change invalidates zoom-dependent caches
//! - Data revision invalidates computed overlays

use crate::{CacheKey, CacheProvenanceTracker, InvalidationTrigger};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Timeline cache manager
///
/// Manages all timeline-specific caches with full provenance tracking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineCacheManager {
    /// Cache provenance tracker
    tracker: CacheProvenanceTracker,

    /// Current data revision (incremented when timeline data changes)
    data_revision: u64,

    /// Current zoom level (for cache bucketing)
    zoom_level: f32,

    /// Current filter hash (for filtered views)
    filter_hash: u64,

    /// Per-lane cache entries
    lane_cache_entries: HashMap<String, Vec<CacheKey>>,
}

impl TimelineCacheManager {
    /// Create a new timeline cache manager
    pub fn new() -> Self {
        Self {
            tracker: CacheProvenanceTracker::new(),
            data_revision: 0,
            zoom_level: 1.0,
            filter_hash: 0,
            lane_cache_entries: HashMap::new(),
        }
    }

    /// Get cache key for timeline rendering
    pub fn timeline_cache_key(&self) -> CacheKey {
        CacheKey::Timeline {
            data_revision: self.data_revision,
            zoom_level_x100: (self.zoom_level * 100.0) as u32,
            filter_hash: self.filter_hash,
        }
    }

    /// Add a timeline cache entry
    pub fn add_timeline_cache(&mut self, size_bytes: usize) -> CacheKey {
        let key = self.timeline_cache_key();
        self.tracker
            .add_entry(key.clone(), size_bytes, "timeline_renderer".to_string());
        key
    }

    /// Add a lane-specific cache entry
    pub fn add_lane_cache(&mut self, lane_id: &str, size_bytes: usize) -> CacheKey {
        // Create lane-specific cache key by hashing lane_id into filter
        let lane_hash = self.hash_string(lane_id);
        let key = CacheKey::Timeline {
            data_revision: self.data_revision,
            zoom_level_x100: (self.zoom_level * 100.0) as u32,
            filter_hash: self.filter_hash ^ lane_hash, // XOR with lane hash for uniqueness
        };

        self.tracker.add_entry(
            key.clone(),
            size_bytes,
            format!("timeline_lane_{}", lane_id),
        );

        // Track lane cache entries
        self.lane_cache_entries
            .entry(lane_id.to_string())
            .or_default()
            .push(key.clone());

        key
    }

    /// Simple hash function for strings
    fn hash_string(&self, s: &str) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        s.hash(&mut hasher);
        hasher.finish()
    }

    /// Record a cache hit
    pub fn record_hit(&mut self, key: &CacheKey) {
        self.tracker.record_hit(key);
    }

    /// Record a cache miss
    pub fn record_miss(&mut self, key: &CacheKey) {
        self.tracker.record_miss(key);
    }

    /// Update data revision (invalidates all timeline caches)
    ///
    /// Per CACHE_INVALIDATION_TABLE: Data revision change invalidates all timeline caches
    pub fn update_data_revision(&mut self) {
        self.data_revision += 1;
        self.tracker
            .invalidate(InvalidationTrigger::DataRevision(self.data_revision));
    }

    /// Update zoom level (invalidates zoom-dependent caches)
    pub fn update_zoom_level(&mut self, new_zoom: f32) {
        if (self.zoom_level - new_zoom).abs() > f32::EPSILON {
            self.zoom_level = new_zoom;
            self.tracker.invalidate(InvalidationTrigger::ZoomChanged);
        }
    }

    /// Update filter hash (invalidates filter-dependent caches)
    pub fn update_filter(&mut self, new_filter_hash: u64) {
        if self.filter_hash != new_filter_hash {
            self.filter_hash = new_filter_hash;
            // Invalidate all caches since filter change affects all timeline rendering
            self.tracker
                .invalidate(InvalidationTrigger::Manual("filter_changed".to_string()));
        }
    }

    /// Invalidate all caches for a specific lane
    pub fn invalidate_lane(&mut self, lane_id: &str) {
        if let Some(cache_keys) = self.lane_cache_entries.get(lane_id) {
            for key in cache_keys {
                self.tracker.evict(key);
            }
        }
        self.lane_cache_entries.remove(lane_id);
    }

    /// Clear all timeline caches
    pub fn clear_all(&mut self) {
        self.tracker
            .invalidate(InvalidationTrigger::Manual("clear_all".to_string()));
        self.lane_cache_entries.clear();
    }

    /// Get cache statistics
    pub fn stats(&self) -> TimelineCacheStats {
        let base_stats = self.tracker.stats();

        TimelineCacheStats {
            total_entries: base_stats.total_entries,
            valid_entries: base_stats.valid_entries,
            invalid_entries: base_stats.invalid_entries,
            total_size_bytes: base_stats.total_size_bytes,
            hit_count: base_stats.hit_count,
            miss_count: base_stats.miss_count,
            hit_rate: base_stats.hit_rate,
            data_revision: self.data_revision,
            active_lanes: self.lane_cache_entries.len(),
        }
    }

    /// Get the cache provenance tracker
    pub fn tracker(&self) -> &CacheProvenanceTracker {
        &self.tracker
    }

    /// Get the cache provenance tracker (mutable)
    pub fn tracker_mut(&mut self) -> &mut CacheProvenanceTracker {
        &mut self.tracker
    }

    /// Get current data revision
    pub fn data_revision(&self) -> u64 {
        self.data_revision
    }

    /// Get current zoom level
    pub fn zoom_level(&self) -> f32 {
        self.zoom_level
    }
}

impl Default for TimelineCacheManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Timeline cache statistics
#[derive(Debug, Clone)]
pub struct TimelineCacheStats {
    pub total_entries: usize,
    pub valid_entries: usize,
    pub invalid_entries: usize,
    pub total_size_bytes: usize,
    pub hit_count: u64,
    pub miss_count: u64,
    pub hit_rate: f64,
    pub data_revision: u64,
    pub active_lanes: usize,
}

#[cfg(test)]
mod tests {
    include!("timeline_cache_test.rs");
}
