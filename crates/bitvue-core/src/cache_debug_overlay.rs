//! Cache Debug Overlay - cache_debug_overlay.001
//!
//! Per FRAME_IDENTITY_CONTRACT:
//! - Visualize cache status (cached vs computed)
//! - Show invalidation reasons and provenance
//! - Intended for engineers and QA debugging

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Cache entry status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CacheStatus {
    /// Data is cached and valid
    Cached,

    /// Data was computed this frame
    Computed,

    /// Cache was invalidated (with reason)
    Invalidated(InvalidationReason),

    /// Data not available
    Missing,
}

/// Reason why cache was invalidated
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InvalidationReason {
    /// Stream changed (new file loaded)
    StreamChanged,

    /// Frame data changed
    FrameDataChanged,

    /// User requested refresh
    UserRefresh,

    /// Dependency invalidation (e.g., decode failed)
    DependencyInvalidation(String),

    /// Memory pressure
    MemoryPressure,

    /// Manual invalidation (for testing)
    Manual(String),
}

impl InvalidationReason {
    /// Get human-readable description
    pub fn description(&self) -> String {
        match self {
            InvalidationReason::StreamChanged => "Stream changed".to_string(),
            InvalidationReason::FrameDataChanged => "Frame data changed".to_string(),
            InvalidationReason::UserRefresh => "User requested refresh".to_string(),
            InvalidationReason::DependencyInvalidation(dep) => {
                format!("Dependency invalidated: {}", dep)
            }
            InvalidationReason::MemoryPressure => "Memory pressure".to_string(),
            InvalidationReason::Manual(msg) => format!("Manual: {}", msg),
        }
    }
}

/// Cache provenance tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheProvenance {
    /// When was this data cached (frame number or timestamp)
    pub cached_at: u64,

    /// Source of the cached data
    pub source: String,

    /// How many times has this been accessed
    pub access_count: u32,

    /// Last access time
    pub last_access: u64,
}

impl CacheProvenance {
    /// Create new provenance record
    pub fn new(cached_at: u64, source: String) -> Self {
        Self {
            cached_at,
            source,
            access_count: 0,
            last_access: cached_at,
        }
    }

    /// Record an access
    pub fn record_access(&mut self, time: u64) {
        self.access_count += 1;
        self.last_access = time;
    }
}

/// Cache entry with debug information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    /// Current status
    pub status: CacheStatus,

    /// Provenance (if cached)
    pub provenance: Option<CacheProvenance>,

    /// Size in bytes
    pub size_bytes: usize,
}

impl CacheEntry {
    /// Create a new cached entry
    pub fn cached(cached_at: u64, source: String, size_bytes: usize) -> Self {
        Self {
            status: CacheStatus::Cached,
            provenance: Some(CacheProvenance::new(cached_at, source)),
            size_bytes,
        }
    }

    /// Create a new computed entry
    pub fn computed(size_bytes: usize) -> Self {
        Self {
            status: CacheStatus::Computed,
            provenance: None,
            size_bytes,
        }
    }

    /// Create an invalidated entry
    pub fn invalidated(reason: InvalidationReason) -> Self {
        Self {
            status: CacheStatus::Invalidated(reason),
            provenance: None,
            size_bytes: 0,
        }
    }

    /// Create a missing entry
    pub fn missing() -> Self {
        Self {
            status: CacheStatus::Missing,
            provenance: None,
            size_bytes: 0,
        }
    }

    /// Check if entry is valid (cached or computed)
    pub fn is_valid(&self) -> bool {
        matches!(self.status, CacheStatus::Cached | CacheStatus::Computed)
    }

    /// Get invalidation reason if invalidated
    pub fn invalidation_reason(&self) -> Option<&InvalidationReason> {
        match &self.status {
            CacheStatus::Invalidated(reason) => Some(reason),
            _ => None,
        }
    }
}

/// Cache type identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CacheType {
    /// Decoded frame (YUV data)
    DecodedFrame,

    /// QP heatmap overlay
    QpHeatmap,

    /// Motion vector overlay
    MvOverlay,

    /// Partition grid overlay
    PartitionGrid,

    /// Diff heatmap (comparison)
    DiffHeatmap,

    /// PSNR metrics
    PsnrMetrics,

    /// SSIM metrics
    SsimMetrics,

    /// VMAF metrics
    VmafMetrics,

    /// Custom cache type
    Custom(String),
}

impl CacheType {
    /// Get display name
    pub fn name(&self) -> String {
        match self {
            CacheType::DecodedFrame => "Decoded Frame".to_string(),
            CacheType::QpHeatmap => "QP Heatmap".to_string(),
            CacheType::MvOverlay => "Motion Vectors".to_string(),
            CacheType::PartitionGrid => "Partition Grid".to_string(),
            CacheType::DiffHeatmap => "Diff Heatmap".to_string(),
            CacheType::PsnrMetrics => "PSNR Metrics".to_string(),
            CacheType::SsimMetrics => "SSIM Metrics".to_string(),
            CacheType::VmafMetrics => "VMAF Metrics".to_string(),
            CacheType::Custom(name) => name.clone(),
        }
    }
}

/// Cache debug overlay state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheDebugOverlay {
    /// Map of (frame_idx, cache_type) -> cache entry
    /// Note: frame_idx is display_idx per FRAME_IDENTITY_CONTRACT
    cache_entries: HashMap<(usize, CacheType), CacheEntry>,

    /// Total memory used by cache (bytes)
    total_memory_bytes: usize,

    /// Memory limit (bytes)
    memory_limit_bytes: Option<usize>,

    /// Whether overlay is visible
    visible: bool,
}

impl CacheDebugOverlay {
    /// Create new cache debug overlay
    pub fn new() -> Self {
        Self {
            cache_entries: HashMap::new(),
            total_memory_bytes: 0,
            memory_limit_bytes: None,
            visible: false,
        }
    }

    /// Set memory limit
    pub fn set_memory_limit(&mut self, limit_bytes: usize) {
        self.memory_limit_bytes = Some(limit_bytes);
    }

    /// Show/hide overlay
    pub fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }

    /// Check if overlay is visible
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Record a cached entry
    pub fn record_cached(
        &mut self,
        frame_idx: usize,
        cache_type: CacheType,
        cached_at: u64,
        source: String,
        size_bytes: usize,
    ) {
        let entry = CacheEntry::cached(cached_at, source, size_bytes);
        self.total_memory_bytes += size_bytes;
        self.cache_entries.insert((frame_idx, cache_type), entry);
    }

    /// Record a computed entry
    pub fn record_computed(&mut self, frame_idx: usize, cache_type: CacheType, size_bytes: usize) {
        let entry = CacheEntry::computed(size_bytes);
        self.total_memory_bytes += size_bytes;
        self.cache_entries.insert((frame_idx, cache_type), entry);
    }

    /// Record an invalidation
    pub fn record_invalidation(
        &mut self,
        frame_idx: usize,
        cache_type: CacheType,
        reason: InvalidationReason,
    ) {
        // Remove old entry from memory count
        if let Some(old_entry) = self.cache_entries.get(&(frame_idx, cache_type.clone())) {
            self.total_memory_bytes = self.total_memory_bytes.saturating_sub(old_entry.size_bytes);
        }

        let entry = CacheEntry::invalidated(reason);
        self.cache_entries.insert((frame_idx, cache_type), entry);
    }

    /// Get cache entry
    pub fn get_entry(&self, frame_idx: usize, cache_type: &CacheType) -> Option<&CacheEntry> {
        self.cache_entries.get(&(frame_idx, cache_type.clone()))
    }

    /// Get all entries for a frame
    pub fn get_frame_entries(&self, frame_idx: usize) -> Vec<(&CacheType, &CacheEntry)> {
        self.cache_entries
            .iter()
            .filter_map(|((idx, cache_type), entry)| {
                if *idx == frame_idx {
                    Some((cache_type, entry))
                } else {
                    None
                }
            })
            .collect()
    }

    /// Get cache statistics
    pub fn get_stats(&self) -> CacheStats {
        let mut stats = CacheStats {
            total_entries: self.cache_entries.len(),
            cached_count: 0,
            computed_count: 0,
            invalidated_count: 0,
            missing_count: 0,
            total_memory_bytes: self.total_memory_bytes,
            memory_limit_bytes: self.memory_limit_bytes,
        };

        for entry in self.cache_entries.values() {
            match entry.status {
                CacheStatus::Cached => stats.cached_count += 1,
                CacheStatus::Computed => stats.computed_count += 1,
                CacheStatus::Invalidated(_) => stats.invalidated_count += 1,
                CacheStatus::Missing => stats.missing_count += 1,
            }
        }

        stats
    }

    /// Clear all cache entries
    pub fn clear(&mut self) {
        self.cache_entries.clear();
        self.total_memory_bytes = 0;
    }

    /// Clear entries for a specific frame
    pub fn clear_frame(&mut self, frame_idx: usize) {
        let keys_to_remove: Vec<_> = self
            .cache_entries
            .keys()
            .filter(|(idx, _)| *idx == frame_idx)
            .cloned()
            .collect();

        for key in keys_to_remove {
            if let Some(entry) = self.cache_entries.remove(&key) {
                self.total_memory_bytes = self.total_memory_bytes.saturating_sub(entry.size_bytes);
            }
        }
    }
}

impl Default for CacheDebugOverlay {
    fn default() -> Self {
        Self::new()
    }
}

/// Cache statistics summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub total_entries: usize,
    pub cached_count: usize,
    pub computed_count: usize,
    pub invalidated_count: usize,
    pub missing_count: usize,
    pub total_memory_bytes: usize,
    pub memory_limit_bytes: Option<usize>,
}

impl CacheStats {
    /// Get memory usage percentage (if limit is set)
    pub fn memory_usage_percent(&self) -> Option<f32> {
        self.memory_limit_bytes.map(|limit| {
            if limit > 0 {
                (self.total_memory_bytes as f32 / limit as f32) * 100.0
            } else {
                0.0
            }
        })
    }

    /// Format memory size as human-readable string
    pub fn format_memory(&self) -> String {
        format_bytes(self.total_memory_bytes)
    }
}

/// Format bytes as human-readable string
fn format_bytes(bytes: usize) -> String {
    const KB: usize = 1024;
    const MB: usize = KB * 1024;
    const GB: usize = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} bytes", bytes)
    }
}

#[cfg(test)]
include!("cache_debug_overlay_test.rs");
