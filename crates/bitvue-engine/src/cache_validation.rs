//! Cache Validation & HUD - T9-2
//!
//! Per CACHE_LEVELS_SPEC.md:
//! - Decode cache: 64 frames (ring/LRU)
//! - Texture cache: 256MB per stream
//! - QP heatmap: 128MB per stream
//! - Diff heatmap: 128MB for AB
//! - MV visible list: 32MB per stream
//! - Grid line: 16MB per stream
//!
//! Per CACHE_INVALIDATION_TABLE.md:
//! - Frame change ALWAYS invalidates frame-bound overlays
//! - Resolution change invalidates all textures
//! - Cache hit/miss must be visible in Dev HUD
//!
//! Deliverables:
//! - CacheStatsHUD: Display hit/miss stats for all caches
//! - Eviction logs: Track and log eviction events
//! - Cap enforcement checks: Validate cache capacity limits

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Cache type identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CacheType {
    /// Decode cache (per stream)
    Decode,
    /// Texture cache (per stream)
    Texture,
    /// QP heatmap overlay texture
    QpHeatmap,
    /// Diff heatmap overlay texture
    DiffHeatmap,
    /// MV overlay visible list
    MvVisibleList,
    /// Grid overlay line cache
    GridLine,
}

impl CacheType {
    /// Get display name
    pub fn display_name(&self) -> &'static str {
        match self {
            CacheType::Decode => "Decode",
            CacheType::Texture => "Texture",
            CacheType::QpHeatmap => "QP Heatmap",
            CacheType::DiffHeatmap => "Diff Heatmap",
            CacheType::MvVisibleList => "MV Visible List",
            CacheType::GridLine => "Grid Line",
        }
    }

    /// Get default capacity limit
    ///
    /// Per CACHE_LEVELS_SPEC.md:
    /// - Decode: 64 frames
    /// - Texture: 256MB per stream
    /// - QP Heatmap: 128MB per stream
    /// - Diff Heatmap: 128MB for AB
    /// - MV Visible List: 32MB per stream
    /// - Grid Line: 16MB per stream
    pub fn default_cap_bytes(&self) -> u64 {
        match self {
            // 64 frames * ~1MB per frame (assuming 1080p YUV)
            CacheType::Decode => 64 * 1024 * 1024,
            CacheType::Texture => 256 * 1024 * 1024,
            CacheType::QpHeatmap => 128 * 1024 * 1024,
            CacheType::DiffHeatmap => 128 * 1024 * 1024,
            CacheType::MvVisibleList => 32 * 1024 * 1024,
            CacheType::GridLine => 16 * 1024 * 1024,
        }
    }
}

/// Cache statistics for a single cache
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub cache_type: CacheType,
    pub requests: u64,
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub current_size_bytes: u64,
    pub cap_bytes: u64,
}

impl CacheStats {
    /// Create new cache stats
    pub fn new(cache_type: CacheType) -> Self {
        Self {
            cache_type,
            requests: 0,
            hits: 0,
            misses: 0,
            evictions: 0,
            current_size_bytes: 0,
            cap_bytes: cache_type.default_cap_bytes(),
        }
    }

    /// Calculate hit rate (0.0 to 1.0)
    pub fn hit_rate(&self) -> f64 {
        if self.requests == 0 {
            return 0.0;
        }
        self.hits as f64 / self.requests as f64
    }

    /// Calculate usage percentage (0.0 to 1.0)
    pub fn usage_percent(&self) -> f64 {
        if self.cap_bytes == 0 {
            return 0.0;
        }
        (self.current_size_bytes as f64 / self.cap_bytes as f64).min(1.0)
    }

    /// Check if cache is over capacity
    pub fn is_over_capacity(&self) -> bool {
        self.current_size_bytes > self.cap_bytes
    }

    /// Check if aggressive eviction should trigger (>80% usage)
    pub fn should_evict_aggressively(&self) -> bool {
        self.usage_percent() > 0.8
    }

    /// Record a cache hit
    pub fn record_hit(&mut self) {
        self.requests += 1;
        self.hits += 1;
    }

    /// Record a cache miss
    pub fn record_miss(&mut self) {
        self.requests += 1;
        self.misses += 1;
    }

    /// Record an eviction
    pub fn record_eviction(&mut self, bytes_freed: u64) {
        self.evictions += 1;
        self.current_size_bytes = self.current_size_bytes.saturating_sub(bytes_freed);
    }

    /// Add bytes to cache
    pub fn add_bytes(&mut self, bytes: u64) {
        self.current_size_bytes += bytes;
    }

    /// Set custom cap
    pub fn set_cap(&mut self, cap_bytes: u64) {
        self.cap_bytes = cap_bytes;
    }
}

/// Eviction event record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvictionEvent {
    pub timestamp_ms: u64,
    pub cache_type: CacheType,
    pub reason: EvictionReason,
    pub bytes_freed: u64,
    pub entries_evicted: u32,
    pub cache_usage_before: f64, // percentage
    pub cache_usage_after: f64,  // percentage
}

/// Reason for cache eviction
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvictionReason {
    /// Capacity exceeded
    CapacityExceeded,
    /// Aggressive eviction triggered (>80% usage)
    AggressiveEviction,
    /// Manual invalidation (frame change, resolution change, etc.)
    ManualInvalidation,
    /// LRU eviction
    Lru,
}

impl EvictionReason {
    pub fn display_name(&self) -> &'static str {
        match self {
            EvictionReason::CapacityExceeded => "Capacity Exceeded",
            EvictionReason::AggressiveEviction => "Aggressive Eviction (>80%)",
            EvictionReason::ManualInvalidation => "Manual Invalidation",
            EvictionReason::Lru => "LRU",
        }
    }
}

/// Cache validation report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheValidationReport {
    pub timestamp_ms: u64,
    pub cache_stats: HashMap<CacheType, CacheStats>,
    pub violations: Vec<CacheViolation>,
    pub recent_evictions: Vec<EvictionEvent>,
    pub overall_hit_rate: f64,
}

/// Cache capacity violation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheViolation {
    pub cache_type: CacheType,
    pub violation_type: ViolationType,
    pub current_bytes: u64,
    pub cap_bytes: u64,
    pub severity: ViolationSeverity,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ViolationType {
    /// Cache size exceeds capacity
    CapacityExceeded,
    /// Cache usage > 80% (aggressive eviction zone)
    AggressiveZone,
    /// Hit rate below threshold
    LowHitRate,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ViolationSeverity {
    Info,
    Warning,
    Error,
}

/// Cache validation manager
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheValidator {
    pub cache_stats: HashMap<CacheType, CacheStats>,
    pub eviction_log: Vec<EvictionEvent>,
    pub max_eviction_log_entries: usize,
}

impl Default for CacheValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl CacheValidator {
    /// Create new cache validator with all cache types initialized
    pub fn new() -> Self {
        let mut cache_stats = HashMap::new();
        cache_stats.insert(CacheType::Decode, CacheStats::new(CacheType::Decode));
        cache_stats.insert(CacheType::Texture, CacheStats::new(CacheType::Texture));
        cache_stats.insert(CacheType::QpHeatmap, CacheStats::new(CacheType::QpHeatmap));
        cache_stats.insert(
            CacheType::DiffHeatmap,
            CacheStats::new(CacheType::DiffHeatmap),
        );
        cache_stats.insert(
            CacheType::MvVisibleList,
            CacheStats::new(CacheType::MvVisibleList),
        );
        cache_stats.insert(CacheType::GridLine, CacheStats::new(CacheType::GridLine));

        Self {
            cache_stats,
            eviction_log: Vec::new(),
            max_eviction_log_entries: 1000,
        }
    }

    /// Get cache stats for a specific cache type
    pub fn get_stats(&self, cache_type: CacheType) -> Option<&CacheStats> {
        self.cache_stats.get(&cache_type)
    }

    /// Get mutable cache stats for a specific cache type
    pub fn get_stats_mut(&mut self, cache_type: CacheType) -> Option<&mut CacheStats> {
        self.cache_stats.get_mut(&cache_type)
    }

    /// Record a cache hit
    pub fn record_hit(&mut self, cache_type: CacheType) {
        if let Some(stats) = self.get_stats_mut(cache_type) {
            stats.record_hit();
        }
    }

    /// Record a cache miss
    pub fn record_miss(&mut self, cache_type: CacheType) {
        if let Some(stats) = self.get_stats_mut(cache_type) {
            stats.record_miss();
        }
    }

    /// Record an eviction event
    pub fn record_eviction(
        &mut self,
        cache_type: CacheType,
        reason: EvictionReason,
        bytes_freed: u64,
        entries_evicted: u32,
        timestamp_ms: u64,
    ) {
        let usage_before = self
            .get_stats(cache_type)
            .map(|s| s.usage_percent())
            .unwrap_or(0.0);

        if let Some(stats) = self.get_stats_mut(cache_type) {
            stats.record_eviction(bytes_freed);
        }

        let usage_after = self
            .get_stats(cache_type)
            .map(|s| s.usage_percent())
            .unwrap_or(0.0);

        let event = EvictionEvent {
            timestamp_ms,
            cache_type,
            reason,
            bytes_freed,
            entries_evicted,
            cache_usage_before: usage_before,
            cache_usage_after: usage_after,
        };

        self.eviction_log.push(event);

        // Trim log if it exceeds max entries
        if self.eviction_log.len() > self.max_eviction_log_entries {
            self.eviction_log.drain(0..100); // Remove oldest 100 entries
        }
    }

    /// Add bytes to cache
    pub fn add_bytes(&mut self, cache_type: CacheType, bytes: u64) {
        if let Some(stats) = self.get_stats_mut(cache_type) {
            stats.add_bytes(bytes);
        }
    }

    /// Generate validation report
    pub fn generate_report(&self, timestamp_ms: u64) -> CacheValidationReport {
        let mut violations = Vec::new();

        // Check for violations
        for (cache_type, stats) in &self.cache_stats {
            // Check capacity violations
            if stats.is_over_capacity() {
                violations.push(CacheViolation {
                    cache_type: *cache_type,
                    violation_type: ViolationType::CapacityExceeded,
                    current_bytes: stats.current_size_bytes,
                    cap_bytes: stats.cap_bytes,
                    severity: ViolationSeverity::Error,
                });
            } else if stats.should_evict_aggressively() {
                violations.push(CacheViolation {
                    cache_type: *cache_type,
                    violation_type: ViolationType::AggressiveZone,
                    current_bytes: stats.current_size_bytes,
                    cap_bytes: stats.cap_bytes,
                    severity: ViolationSeverity::Warning,
                });
            }

            // Check hit rate violations (< 50% is concerning)
            if stats.requests > 100 && stats.hit_rate() < 0.5 {
                violations.push(CacheViolation {
                    cache_type: *cache_type,
                    violation_type: ViolationType::LowHitRate,
                    current_bytes: stats.current_size_bytes,
                    cap_bytes: stats.cap_bytes,
                    severity: ViolationSeverity::Info,
                });
            }
        }

        // Calculate overall hit rate
        let total_requests: u64 = self.cache_stats.values().map(|s| s.requests).sum();
        let total_hits: u64 = self.cache_stats.values().map(|s| s.hits).sum();
        let overall_hit_rate = if total_requests > 0 {
            total_hits as f64 / total_requests as f64
        } else {
            0.0
        };

        // Get recent evictions (last 100)
        let recent_evictions = self.eviction_log.iter().rev().take(100).cloned().collect();

        CacheValidationReport {
            timestamp_ms,
            cache_stats: self.cache_stats.clone(),
            violations,
            recent_evictions,
            overall_hit_rate,
        }
    }

    /// Get recent evictions for a specific cache type
    pub fn get_recent_evictions(&self, cache_type: CacheType, limit: usize) -> Vec<&EvictionEvent> {
        self.eviction_log
            .iter()
            .rev()
            .filter(|e| e.cache_type == cache_type)
            .take(limit)
            .collect()
    }

    /// Clear all stats (for testing)
    pub fn clear_stats(&mut self) {
        for stats in self.cache_stats.values_mut() {
            stats.requests = 0;
            stats.hits = 0;
            stats.misses = 0;
            stats.evictions = 0;
            stats.current_size_bytes = 0;
        }
        self.eviction_log.clear();
    }
}

/// Cache stats HUD display data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStatsHUD {
    pub cache_rows: Vec<CacheStatsRow>,
    pub overall_hit_rate: f64,
    pub total_memory_mb: f64,
    pub recent_violations: Vec<CacheViolation>,
}

/// Single row in cache stats HUD
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStatsRow {
    pub cache_type: CacheType,
    pub display_name: String,
    pub hit_rate: f64,
    pub usage_mb: f64,
    pub cap_mb: f64,
    pub usage_percent: f64,
    pub requests: u64,
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub status: CacheStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CacheStatus {
    /// Cache is healthy
    Healthy,
    /// Cache usage > 80%
    Warning,
    /// Cache over capacity
    Critical,
}

impl CacheStatsHUD {
    /// Create HUD display data from validator
    pub fn from_validator(validator: &CacheValidator) -> Self {
        let mut cache_rows = Vec::new();
        let mut total_memory_bytes = 0u64;

        for cache_type in [
            CacheType::Decode,
            CacheType::Texture,
            CacheType::QpHeatmap,
            CacheType::DiffHeatmap,
            CacheType::MvVisibleList,
            CacheType::GridLine,
        ] {
            if let Some(stats) = validator.get_stats(cache_type) {
                total_memory_bytes += stats.current_size_bytes;

                let status = if stats.is_over_capacity() {
                    CacheStatus::Critical
                } else if stats.should_evict_aggressively() {
                    CacheStatus::Warning
                } else {
                    CacheStatus::Healthy
                };

                cache_rows.push(CacheStatsRow {
                    cache_type,
                    display_name: cache_type.display_name().to_string(),
                    hit_rate: stats.hit_rate(),
                    usage_mb: stats.current_size_bytes as f64 / (1024.0 * 1024.0),
                    cap_mb: stats.cap_bytes as f64 / (1024.0 * 1024.0),
                    usage_percent: stats.usage_percent(),
                    requests: stats.requests,
                    hits: stats.hits,
                    misses: stats.misses,
                    evictions: stats.evictions,
                    status,
                });
            }
        }

        // Get recent violations from latest report
        let report = validator.generate_report(0);

        Self {
            cache_rows,
            overall_hit_rate: report.overall_hit_rate,
            total_memory_mb: total_memory_bytes as f64 / (1024.0 * 1024.0),
            recent_violations: report.violations,
        }
    }
}

#[cfg(test)]
include!("cache_validation_test.rs");
