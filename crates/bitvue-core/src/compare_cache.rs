//! Compare Cache Provenance - T0-2 Cache Provenance Implementation
//!
//! Deliverable: cache_proof_01:Foundations:Compare:AV1:cache_provenance
//!
//! Manages cache provenance for A/B comparison view with:
//! - Dual-stream decode cache tracking (Stream A and B)
//! - Diff heatmap cache provenance
//! - Alignment-dependent invalidation
//! - Manual offset invalidation
//!
//! Per CACHE_INVALIDATION_TABLE:
//! - Frame change invalidates frame-bound caches
//! - Alignment change invalidates diff heatmaps
//! - Manual offset change invalidates diff heatmaps
//! - Resolution change invalidates all textures and diff overlays

use crate::cache_provenance::{CacheKey, CacheProvenanceTracker, CacheStats, InvalidationTrigger};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Stream identifier for dual-stream comparison
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CompareStreamId {
    /// Stream A (reference)
    A,
    /// Stream B (comparison)
    B,
}

impl CompareStreamId {
    pub fn label(&self) -> &'static str {
        match self {
            CompareStreamId::A => "Stream A",
            CompareStreamId::B => "Stream B",
        }
    }
}

/// Compare cache manager
///
/// Manages cache provenance for A/B comparison with dual-stream tracking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompareCacheManager {
    /// Cache tracker for stream A
    cache_a: CacheProvenanceTracker,

    /// Cache tracker for stream B
    cache_b: CacheProvenanceTracker,

    /// Cache tracker for diff overlays
    diff_cache: CacheProvenanceTracker,

    /// Current frame index for stream A
    current_frame_a: Option<usize>,

    /// Current frame index for stream B
    current_frame_b: Option<usize>,

    /// Current manual offset
    manual_offset: i32,

    /// Alignment revision (increments when alignment changes)
    alignment_revision: u64,

    /// Resolution hash (for invalidation on resolution change)
    resolution_hash: u64,

    /// Active diff cache keys by frame pair
    diff_cache_map: HashMap<(usize, usize), Vec<CacheKey>>,
}

impl CompareCacheManager {
    /// Create a new compare cache manager
    pub fn new() -> Self {
        Self {
            cache_a: CacheProvenanceTracker::new(),
            cache_b: CacheProvenanceTracker::new(),
            diff_cache: CacheProvenanceTracker::new(),
            current_frame_a: None,
            current_frame_b: None,
            manual_offset: 0,
            alignment_revision: 0,
            resolution_hash: 0,
            diff_cache_map: HashMap::new(),
        }
    }

    /// Add decode cache entry for stream A
    pub fn add_decode_cache_a(
        &mut self,
        frame_idx: usize,
        decode_params: String,
        size_bytes: usize,
    ) -> CacheKey {
        let key = CacheKey::Decode {
            frame_idx,
            decode_params,
        };

        self.cache_a.add_entry(
            key.clone(),
            size_bytes,
            format!("decode_stream_a_frame_{}", frame_idx),
        );

        key
    }

    /// Add decode cache entry for stream B
    pub fn add_decode_cache_b(
        &mut self,
        frame_idx: usize,
        decode_params: String,
        size_bytes: usize,
    ) -> CacheKey {
        let key = CacheKey::Decode {
            frame_idx,
            decode_params,
        };

        self.cache_b.add_entry(
            key.clone(),
            size_bytes,
            format!("decode_stream_b_frame_{}", frame_idx),
        );

        key
    }

    /// Add texture cache entry for stream
    pub fn add_texture_cache(
        &mut self,
        stream: CompareStreamId,
        frame_idx: usize,
        res_tier: u8,
        colorspace: String,
        size_bytes: usize,
    ) -> CacheKey {
        let key = CacheKey::Texture {
            frame_idx,
            res_tier,
            colorspace,
        };

        let cache = match stream {
            CompareStreamId::A => &mut self.cache_a,
            CompareStreamId::B => &mut self.cache_b,
        };

        cache.add_entry(
            key.clone(),
            size_bytes,
            format!("texture_{}_frame_{}", stream.label(), frame_idx),
        );

        key
    }

    /// Add diff heatmap cache entry
    ///
    /// Tracks dependencies: frame_a, frame_b, alignment, offset, resolution
    pub fn add_diff_heatmap_cache(
        &mut self,
        frame_idx_a: usize,
        frame_idx_b: usize,
        mode: String,
        ab_mapping: String,
        hm_res: u32,
        size_bytes: usize,
    ) -> CacheKey {
        let key = CacheKey::DiffHeatmap {
            frame_idx_a,
            frame_idx_b,
            mode,
            ab_mapping,
            hm_res,
        };

        self.diff_cache.add_entry(
            key.clone(),
            size_bytes,
            format!("diff_heatmap_{}_{}", frame_idx_a, frame_idx_b),
        );

        // Track in diff map for targeted invalidation
        self.diff_cache_map
            .entry((frame_idx_a, frame_idx_b))
            .or_default()
            .push(key.clone());

        key
    }

    /// Set current frame for stream A
    ///
    /// Per CACHE_INVALIDATION_TABLE: Frame change invalidates frame-bound caches
    pub fn set_frame_a(&mut self, frame_idx: usize) {
        if self.current_frame_a != Some(frame_idx) {
            if let Some(old_frame) = self.current_frame_a {
                // Only invalidate if frame actually changed
                if old_frame != frame_idx {
                    self.cache_a
                        .invalidate(InvalidationTrigger::FrameChanged(frame_idx));
                }
            }
            self.current_frame_a = Some(frame_idx);
        }
    }

    /// Set current frame for stream B
    pub fn set_frame_b(&mut self, frame_idx: usize) {
        if self.current_frame_b != Some(frame_idx) {
            if let Some(old_frame) = self.current_frame_b {
                if old_frame != frame_idx {
                    self.cache_b
                        .invalidate(InvalidationTrigger::FrameChanged(frame_idx));
                }
            }
            self.current_frame_b = Some(frame_idx);
        }
    }

    /// Set manual offset
    ///
    /// Invalidates diff heatmaps that depend on frame alignment.
    pub fn set_manual_offset(&mut self, offset: i32) {
        if self.manual_offset != offset {
            self.manual_offset = offset;
            // Invalidate all diff heatmaps (alignment changed)
            self.diff_cache.invalidate(InvalidationTrigger::Manual(
                "manual_offset_changed".to_string(),
            ));
        }
    }

    /// Update alignment
    ///
    /// Call when alignment algorithm re-runs or changes.
    /// Invalidates all diff heatmaps.
    pub fn update_alignment(&mut self) {
        self.alignment_revision += 1;
        self.diff_cache
            .invalidate(InvalidationTrigger::Manual(format!(
                "alignment_updated_rev_{}",
                self.alignment_revision
            )));
    }

    /// Set resolution hash
    ///
    /// Call when stream resolution changes.
    /// Per CACHE_INVALIDATION_TABLE: Resolution change invalidates all textures and diff overlays.
    pub fn set_resolution(&mut self, width_a: u32, height_a: u32, width_b: u32, height_b: u32) {
        let new_hash = Self::hash_resolution(width_a, height_a, width_b, height_b);

        if self.resolution_hash != new_hash {
            self.resolution_hash = new_hash;

            // Invalidate textures in both streams
            self.cache_a
                .invalidate(InvalidationTrigger::ResolutionChanged);
            self.cache_b
                .invalidate(InvalidationTrigger::ResolutionChanged);

            // Invalidate all diff heatmaps
            self.diff_cache
                .invalidate(InvalidationTrigger::ResolutionChanged);
        }
    }

    /// Hash resolution for change detection
    fn hash_resolution(width_a: u32, height_a: u32, width_b: u32, height_b: u32) -> u64 {
        // Simple hash combining all dimensions
        ((width_a as u64) << 48)
            | ((height_a as u64) << 32)
            | ((width_b as u64) << 16)
            | (height_b as u64)
    }

    /// Record cache hit for stream
    pub fn record_hit(&mut self, stream: CompareStreamId, key: &CacheKey) {
        match stream {
            CompareStreamId::A => self.cache_a.record_hit(key),
            CompareStreamId::B => self.cache_b.record_hit(key),
        }
    }

    /// Record cache miss for stream
    pub fn record_miss(&mut self, stream: CompareStreamId, key: &CacheKey) {
        match stream {
            CompareStreamId::A => self.cache_a.record_miss(key),
            CompareStreamId::B => self.cache_b.record_miss(key),
        }
    }

    /// Record diff cache hit
    pub fn record_diff_hit(&mut self, key: &CacheKey) {
        self.diff_cache.record_hit(key);
    }

    /// Record diff cache miss
    pub fn record_diff_miss(&mut self, key: &CacheKey) {
        self.diff_cache.record_miss(key);
    }

    /// Get cache statistics for stream A
    pub fn stats_a(&self) -> CacheStats {
        self.cache_a.stats()
    }

    /// Get cache statistics for stream B
    pub fn stats_b(&self) -> CacheStats {
        self.cache_b.stats()
    }

    /// Get cache statistics for diff overlays
    pub fn stats_diff(&self) -> CacheStats {
        self.diff_cache.stats()
    }

    /// Get combined statistics across all caches
    pub fn stats_combined(&self) -> CombinedCacheStats {
        let stats_a = self.cache_a.stats();
        let stats_b = self.cache_b.stats();
        let stats_diff = self.diff_cache.stats();

        let total_entries =
            stats_a.total_entries + stats_b.total_entries + stats_diff.total_entries;
        let total_size_bytes =
            stats_a.total_size_bytes + stats_b.total_size_bytes + stats_diff.total_size_bytes;

        let total_hits = stats_a.hit_count + stats_b.hit_count + stats_diff.hit_count;
        let total_attempts = stats_a.hit_count
            + stats_a.miss_count
            + stats_b.hit_count
            + stats_b.miss_count
            + stats_diff.hit_count
            + stats_diff.miss_count;

        let combined_hit_rate = if total_attempts > 0 {
            total_hits as f64 / total_attempts as f64
        } else {
            0.0
        };

        CombinedCacheStats {
            stream_a: stats_a,
            stream_b: stats_b,
            diff: stats_diff,
            total_entries,
            total_size_bytes,
            combined_hit_rate,
        }
    }

    /// Get current frame for stream A
    pub fn current_frame_a(&self) -> Option<usize> {
        self.current_frame_a
    }

    /// Get current frame for stream B
    pub fn current_frame_b(&self) -> Option<usize> {
        self.current_frame_b
    }

    /// Get current manual offset
    pub fn manual_offset(&self) -> i32 {
        self.manual_offset
    }

    /// Get alignment revision
    pub fn alignment_revision(&self) -> u64 {
        self.alignment_revision
    }

    /// Get diff cache entries for frame pair
    pub fn diff_entries_for_pair(&self, frame_a: usize, frame_b: usize) -> Vec<&CacheKey> {
        self.diff_cache_map
            .get(&(frame_a, frame_b))
            .map(|keys| keys.iter().collect())
            .unwrap_or_default()
    }

    /// Evict LRU entries from stream cache
    pub fn evict_lru_stream(&mut self, stream: CompareStreamId, target_bytes: usize) -> usize {
        let cache = match stream {
            CompareStreamId::A => &mut self.cache_a,
            CompareStreamId::B => &mut self.cache_b,
        };

        let candidates = cache.find_lru_eviction_candidates(target_bytes);
        let count = candidates.len();

        for key in candidates {
            cache.evict(&key);
        }

        count
    }

    /// Evict LRU diff cache entries
    pub fn evict_lru_diff(&mut self, target_bytes: usize) -> usize {
        let candidates = self.diff_cache.find_lru_eviction_candidates(target_bytes);
        let count = candidates.len();

        for key in candidates {
            self.diff_cache.evict(&key);

            // Clean up diff_cache_map
            if let CacheKey::DiffHeatmap {
                frame_idx_a,
                frame_idx_b,
                ..
            } = &key
            {
                if let Some(keys) = self.diff_cache_map.get_mut(&(*frame_idx_a, *frame_idx_b)) {
                    keys.retain(|k| k != &key);
                }
            }
        }

        count
    }

    /// Clear all caches
    pub fn clear(&mut self) {
        self.cache_a.clear();
        self.cache_b.clear();
        self.diff_cache.clear();
        self.diff_cache_map.clear();
        self.current_frame_a = None;
        self.current_frame_b = None;
        self.manual_offset = 0;
        self.alignment_revision = 0;
        self.resolution_hash = 0;
    }

    /// Get total cache size in bytes
    pub fn total_size_bytes(&self) -> usize {
        let stats_a = self.cache_a.stats();
        let stats_b = self.cache_b.stats();
        let stats_diff = self.diff_cache.stats();
        stats_a.total_size_bytes + stats_b.total_size_bytes + stats_diff.total_size_bytes
    }

    /// Get total entry count across all caches
    pub fn total_entries(&self) -> usize {
        let stats_a = self.cache_a.stats();
        let stats_b = self.cache_b.stats();
        let stats_diff = self.diff_cache.stats();
        stats_a.total_entries + stats_b.total_entries + stats_diff.total_entries
    }
}

impl Default for CompareCacheManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Combined cache statistics for all compare caches
#[derive(Debug, Clone)]
pub struct CombinedCacheStats {
    /// Statistics for stream A
    pub stream_a: CacheStats,
    /// Statistics for stream B
    pub stream_b: CacheStats,
    /// Statistics for diff overlays
    pub diff: CacheStats,
    /// Total entries across all caches
    pub total_entries: usize,
    /// Total size in bytes across all caches
    pub total_size_bytes: usize,
    /// Combined hit rate
    pub combined_hit_rate: f64,
}

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
    include!("compare_cache_test.rs");
}
