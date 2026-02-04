//! Memory statistics module - tracking allocation statistics.

/// Statistics for memory allocations.
#[derive(Clone, Debug, Default)]
pub struct MemStats {
    /// Total number of allocations.
    pub alloc_count: usize,
    /// Total number of deallocations.
    pub dealloc_count: usize,
    /// Total bytes allocated.
    pub total_allocated: usize,
    /// Total bytes deallocated.
    pub total_deallocated: usize,
    /// Current bytes in use.
    pub current_usage: usize,
    /// Peak bytes in use.
    pub peak_usage: usize,
    /// Number of failed allocations (out of memory).
    pub failed_allocs: usize,
}

impl MemStats {
    /// Creates new zeroed stats.
    pub const fn new() -> Self {
        Self {
            alloc_count: 0,
            dealloc_count: 0,
            total_allocated: 0,
            total_deallocated: 0,
            current_usage: 0,
            peak_usage: 0,
            failed_allocs: 0,
        }
    }

    /// Records an allocation.
    pub fn record_alloc(&mut self, size: usize) {
        self.alloc_count += 1;
        self.total_allocated += size;
        self.current_usage += size;
        if self.current_usage > self.peak_usage {
            self.peak_usage = self.current_usage;
        }
    }

    /// Records a deallocation.
    pub fn record_dealloc(&mut self, size: usize) {
        self.dealloc_count += 1;
        self.total_deallocated += size;
        self.current_usage = self.current_usage.saturating_sub(size);
    }

    /// Records a failed allocation.
    pub fn record_failed_alloc(&mut self) {
        self.failed_allocs += 1;
    }

    /// Returns the allocation efficiency (ratio of peak to total allocated).
    pub fn efficiency(&self) -> f64 {
        if self.total_allocated == 0 {
            1.0
        } else {
            self.peak_usage as f64 / self.total_allocated as f64
        }
    }

    /// Returns the number of active allocations.
    pub fn active_allocs(&self) -> usize {
        self.alloc_count.saturating_sub(self.dealloc_count)
    }

    /// Resets the statistics.
    pub fn reset(&mut self) {
        *self = Self::new();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mem_stats_new() {
        let stats = MemStats::new();
        assert_eq!(stats.alloc_count, 0);
        assert_eq!(stats.current_usage, 0);
    }

    #[test]
    fn test_mem_stats_record_alloc() {
        let mut stats = MemStats::new();
        stats.record_alloc(100);
        assert_eq!(stats.alloc_count, 1);
        assert_eq!(stats.total_allocated, 100);
        assert_eq!(stats.current_usage, 100);
        assert_eq!(stats.peak_usage, 100);
    }

    #[test]
    fn test_mem_stats_record_dealloc() {
        let mut stats = MemStats::new();
        stats.record_alloc(100);
        stats.record_alloc(50);
        stats.record_dealloc(100);
        assert_eq!(stats.dealloc_count, 1);
        assert_eq!(stats.current_usage, 50);
    }

    #[test]
    fn test_mem_stats_peak() {
        let mut stats = MemStats::new();
        stats.record_alloc(100);
        stats.record_dealloc(50);
        stats.record_alloc(75);
        assert_eq!(stats.peak_usage, 100);
    }

    #[test]
    fn test_mem_stats_active_allocs() {
        let mut stats = MemStats::new();
        stats.record_alloc(100);
        stats.record_alloc(50);
        stats.record_dealloc(50);
        assert_eq!(stats.active_allocs(), 1);
    }

    #[test]
    fn test_mem_stats_efficiency() {
        let mut stats = MemStats::new();
        stats.record_alloc(100);
        stats.record_alloc(50);
        assert!((stats.efficiency() - 0.66).abs() < 0.01);
    }
}
