//! Memory region and profiling support.

use alloc::string::String;
use alloc::vec::Vec;

/// Memory permissions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MemoryPermissions {
    /// Readable.
    pub read: bool,
    /// Writable.
    pub write: bool,
    /// Executable.
    pub execute: bool,
}

impl MemoryPermissions {
    /// Creates new permissions.
    pub const fn new(read: bool, write: bool, execute: bool) -> Self {
        Self {
            read,
            write,
            execute,
        }
    }

    /// Read-only memory.
    pub const fn read_only() -> Self {
        Self {
            read: true,
            write: false,
            execute: false,
        }
    }

    /// Read-write memory.
    pub const fn read_write() -> Self {
        Self {
            read: true,
            write: true,
            execute: false,
        }
    }

    /// Read-execute memory.
    pub const fn read_execute() -> Self {
        Self {
            read: true,
            write: false,
            execute: true,
        }
    }

    /// All permissions.
    pub const fn all() -> Self {
        Self {
            read: true,
            write: true,
            execute: true,
        }
    }

    /// No permissions.
    pub const fn none() -> Self {
        Self {
            read: false,
            write: false,
            execute: false,
        }
    }
}

/// Memory region information.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemoryRegion {
    /// Start address.
    pub start: usize,
    /// End address.
    pub end: usize,
    /// Memory permissions.
    pub permissions: MemoryPermissions,
    /// Region name or label.
    pub name: Option<String>,
}

impl MemoryRegion {
    /// Creates a new memory region.
    pub fn new(start: usize, end: usize, permissions: MemoryPermissions) -> Self {
        Self {
            start,
            end,
            permissions,
            name: None,
        }
    }

    /// Sets the region name.
    pub fn with_name(mut self, name: String) -> Self {
        self.name = Some(name);
        self
    }

    /// Returns the size of this region.
    pub fn size(&self) -> usize {
        self.end.saturating_sub(self.start)
    }

    /// Returns true if the given address is within this region.
    pub fn contains(&self, address: usize) -> bool {
        address >= self.start && address < self.end
    }

    /// Returns true if this region overlaps with another.
    pub fn overlaps(&self, other: &Self) -> bool {
        self.start < other.end && self.end > other.start
    }
}

/// Memory map for debugging.
#[derive(Clone, Debug, Default)]
pub struct MemoryMap {
    regions: Vec<MemoryRegion>,
}

impl MemoryMap {
    /// Creates a new memory map.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a region to the map.
    pub fn add(&mut self, region: MemoryRegion) {
        self.regions.push(region);
    }

    /// Finds the region containing the given address.
    pub fn find_region(&self, address: usize) -> Option<&MemoryRegion> {
        self.regions.iter().find(|r| r.contains(address))
    }

    /// Returns all regions.
    pub fn regions(&self) -> &[MemoryRegion] {
        &self.regions
    }

    /// Sorts regions by start address.
    pub fn sort(&mut self) {
        self.regions.sort_by_key(|r| r.start);
    }

    /// Merges adjacent regions with the same permissions.
    pub fn merge(&mut self) {
        if self.regions.is_empty() {
            return;
        }

        self.regions.sort_by_key(|r| r.start);
        let mut merged = Vec::new();
        let mut current = self.regions[0].clone();

        for region in &self.regions[1..] {
            if region.start == current.end && region.permissions == current.permissions {
                // Merge adjacent regions with same permissions
                current.end = region.end;
            } else {
                merged.push(current);
                current = region.clone();
            }
        }
        merged.push(current);
        self.regions = merged;
    }

    /// Returns the total memory size.
    pub fn total_size(&self) -> usize {
        self.regions.iter().map(|r| r.size()).sum()
    }
}

/// Performance profiling data.
#[derive(Clone, Debug)]
pub struct ProfilingData {
    /// Function name or label.
    pub name: String,
    /// Number of calls.
    pub call_count: u64,
    /// Total time in nanoseconds.
    pub total_time_ns: u64,
    /// Self time (excluding children) in nanoseconds.
    pub self_time_ns: u64,
    /// Average time per call.
    pub avg_time_ns: u64,
}

impl ProfilingData {
    /// Creates new profiling data.
    pub fn new(name: String) -> Self {
        Self {
            name,
            call_count: 0,
            total_time_ns: 0,
            self_time_ns: 0,
            avg_time_ns: 0,
        }
    }

    /// Records a call.
    pub fn record_call(&mut self, duration_ns: u64) {
        self.call_count = self.call_count.saturating_add(1);
        self.total_time_ns = self.total_time_ns.saturating_add(duration_ns);
        self.avg_time_ns = self.total_time_ns / self.call_count.max(1);
    }
}

/// A performance profiler.
#[derive(Clone, Debug, Default)]
pub struct Profiler {
    entries: Vec<ProfilingData>,
    enabled: bool,
}

impl Profiler {
    /// Creates a new profiler.
    pub fn new() -> Self {
        Self::default()
    }

    /// Enables or disables profiling.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Returns true if profiling is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Records a function call.
    pub fn record(&mut self, name: &str, duration_ns: u64) {
        if !self.enabled {
            return;
        }

        let entry = self.entries.iter_mut().find(|e| e.name == name);
        if let Some(entry) = entry {
            entry.record_call(duration_ns);
        } else {
            let mut new_entry = ProfilingData::new(name.to_string());
            new_entry.record_call(duration_ns);
            self.entries.push(new_entry);
        }
    }

    /// Returns all profiling entries.
    pub fn entries(&self) -> &[ProfilingData] {
        &self.entries
    }

    /// Clears all profiling data.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Resets profiling data (keeps entries but resets counters).
    pub fn reset(&mut self) {
        for entry in &mut self.entries {
            entry.call_count = 0;
            entry.total_time_ns = 0;
            entry.self_time_ns = 0;
            entry.avg_time_ns = 0;
        }
    }

    /// Returns profiling data for a specific function.
    pub fn get(&self, name: &str) -> Option<&ProfilingData> {
        self.entries.iter().find(|e| e.name == name)
    }

    /// Sorts entries by total time (descending).
    pub fn sort_by_total_time(&mut self) {
        self.entries.sort_by(|a, b| b.total_time_ns.cmp(&a.total_time_ns));
    }

    /// Sorts entries by call count (descending).
    pub fn sort_by_call_count(&mut self) {
        self.entries.sort_by(|a, b| b.call_count.cmp(&a.call_count));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tests for MemoryPermissions
    #[test]
    fn test_memory_permissions_new() {
        let perms = MemoryPermissions::new(true, false, true);
        assert!(perms.read);
        assert!(!perms.write);
        assert!(perms.execute);
    }

    #[test]
    fn test_memory_permissions_read_only() {
        let perms = MemoryPermissions::read_only();
        assert!(perms.read);
        assert!(!perms.write);
        assert!(!perms.execute);
    }

    #[test]
    fn test_memory_permissions_read_write() {
        let perms = MemoryPermissions::read_write();
        assert!(perms.read);
        assert!(perms.write);
        assert!(!perms.execute);
    }

    #[test]
    fn test_memory_permissions_read_execute() {
        let perms = MemoryPermissions::read_execute();
        assert!(perms.read);
        assert!(!perms.write);
        assert!(perms.execute);
    }

    #[test]
    fn test_memory_permissions_all() {
        let perms = MemoryPermissions::all();
        assert!(perms.read);
        assert!(perms.write);
        assert!(perms.execute);
    }

    #[test]
    fn test_memory_permissions_none() {
        let perms = MemoryPermissions::none();
        assert!(!perms.read);
        assert!(!perms.write);
        assert!(!perms.execute);
    }

    // Tests for MemoryRegion
    #[test]
    fn test_memory_region_new() {
        let region = MemoryRegion::new(0x1000, 0x2000, MemoryPermissions::read_only());
        assert_eq!(region.start, 0x1000);
        assert_eq!(region.end, 0x2000);
    }

    #[test]
    fn test_memory_region_with_name() {
        let region = MemoryRegion::new(0x1000, 0x2000, MemoryPermissions::read_write())
            .with_name("test_region".to_string());
        assert_eq!(region.name, Some("test_region".to_string()));
    }

    #[test]
    fn test_memory_region_size() {
        let region = MemoryRegion::new(0x1000, 0x2000, MemoryPermissions::read_only());
        assert_eq!(region.size(), 0x1000);
    }

    #[test]
    fn test_memory_region_contains() {
        let region = MemoryRegion::new(0x1000, 0x2000, MemoryPermissions::read_only());
        assert!(region.contains(0x1000));
        assert!(region.contains(0x1500));
        assert!(!region.contains(0x2000));
        assert!(!region.contains(0x0FFF));
    }

    #[test]
    fn test_memory_region_overlaps() {
        let region1 = MemoryRegion::new(0x1000, 0x2000, MemoryPermissions::read_only());
        let region2 = MemoryRegion::new(0x1500, 0x2500, MemoryPermissions::read_write());
        assert!(region1.overlaps(&region2));
    }

    #[test]
    fn test_memory_region_not_overlaps() {
        let region1 = MemoryRegion::new(0x1000, 0x2000, MemoryPermissions::read_only());
        let region2 = MemoryRegion::new(0x2000, 0x3000, MemoryPermissions::read_write());
        assert!(!region1.overlaps(&region2));
    }

    // Tests for MemoryMap
    #[test]
    fn test_memory_map_new() {
        let map = MemoryMap::new();
        assert!(map.regions().is_empty());
    }

    #[test]
    fn test_memory_map_add() {
        let mut map = MemoryMap::new();
        map.add(MemoryRegion::new(0x1000, 0x2000, MemoryPermissions::read_only()));
        assert_eq!(map.regions().len(), 1);
    }

    #[test]
    fn test_memory_map_find_region() {
        let mut map = MemoryMap::new();
        map.add(MemoryRegion::new(0x1000, 0x2000, MemoryPermissions::read_only()));
        let region = map.find_region(0x1500);
        assert!(region.is_some());
        assert_eq!(region.unwrap().start, 0x1000);
    }

    #[test]
    fn test_memory_map_sort() {
        let mut map = MemoryMap::new();
        map.add(MemoryRegion::new(0x2000, 0x3000, MemoryPermissions::read_only()));
        map.add(MemoryRegion::new(0x1000, 0x2000, MemoryPermissions::read_write()));
        map.sort();
        assert_eq!(map.regions()[0].start, 0x1000);
        assert_eq!(map.regions()[1].start, 0x2000);
    }

    #[test]
    fn test_memory_map_merge() {
        let mut map = MemoryMap::new();
        map.add(MemoryRegion::new(0x1000, 0x2000, MemoryPermissions::read_only()));
        map.add(MemoryRegion::new(0x2000, 0x3000, MemoryPermissions::read_only()));
        map.merge();
        assert_eq!(map.regions().len(), 1);
        assert_eq!(map.regions()[0].end, 0x3000);
    }

    #[test]
    fn test_memory_map_total_size() {
        let mut map = MemoryMap::new();
        map.add(MemoryRegion::new(0x1000, 0x2000, MemoryPermissions::read_only()));
        map.add(MemoryRegion::new(0x2000, 0x3000, MemoryPermissions::read_write()));
        assert_eq!(map.total_size(), 0x2000);
    }

    // Tests for ProfilingData
    #[test]
    fn test_profiling_data_new() {
        let data = ProfilingData::new("test_func".to_string());
        assert_eq!(data.name, "test_func");
        assert_eq!(data.call_count, 0);
    }

    #[test]
    fn test_profiling_data_record_call() {
        let mut data = ProfilingData::new("test_func".to_string());
        data.record_call(100);
        assert_eq!(data.call_count, 1);
        assert_eq!(data.total_time_ns, 100);
        assert_eq!(data.avg_time_ns, 100);
    }

    #[test]
    fn test_profiling_data_record_multiple_calls() {
        let mut data = ProfilingData::new("test_func".to_string());
        data.record_call(100);
        data.record_call(200);
        assert_eq!(data.call_count, 2);
        assert_eq!(data.total_time_ns, 300);
        assert_eq!(data.avg_time_ns, 150);
    }

    // Tests for Profiler
    #[test]
    fn test_profiler_new() {
        let profiler = Profiler::new();
        assert!(!profiler.is_enabled());
    }

    #[test]
    fn test_profiler_set_enabled() {
        let mut profiler = Profiler::new();
        profiler.set_enabled(true);
        assert!(profiler.is_enabled());
    }

    #[test]
    fn test_profiler_record() {
        let mut profiler = Profiler::new();
        profiler.set_enabled(true);
        profiler.record("test_func", 100);
        assert_eq!(profiler.entries().len(), 1);
        assert_eq!(profiler.entries()[0].call_count, 1);
    }

    #[test]
    fn test_profiler_record_disabled() {
        let mut profiler = Profiler::new();
        profiler.record("test_func", 100);
        assert_eq!(profiler.entries().len(), 0);
    }

    #[test]
    fn test_profiler_get() {
        let mut profiler = Profiler::new();
        profiler.set_enabled(true);
        profiler.record("test_func", 100);
        let entry = profiler.get("test_func");
        assert!(entry.is_some());
        assert_eq!(entry.unwrap().name, "test_func");
    }

    #[test]
    fn test_profiler_clear() {
        let mut profiler = Profiler::new();
        profiler.set_enabled(true);
        profiler.record("test_func", 100);
        profiler.clear();
        assert_eq!(profiler.entries().len(), 0);
    }

    #[test]
    fn test_profiler_reset() {
        let mut profiler = Profiler::new();
        profiler.set_enabled(true);
        profiler.record("test_func", 100);
        profiler.reset();
        assert_eq!(profiler.entries().len(), 1);
        assert_eq!(profiler.entries()[0].call_count, 0);
    }

    #[test]
    fn test_profiler_sort_by_total_time() {
        let mut profiler = Profiler::new();
        profiler.set_enabled(true);
        profiler.record("func1", 100);
        profiler.record("func2", 200);
        profiler.sort_by_total_time();
        assert_eq!(profiler.entries()[0].name, "func2");
    }

    #[test]
    fn test_profiler_sort_by_call_count() {
        let mut profiler = Profiler::new();
        profiler.set_enabled(true);
        profiler.record("func1", 100);
        profiler.record("func1", 100);
        profiler.record("func2", 100);
        profiler.sort_by_call_count();
        assert_eq!(profiler.entries()[0].name, "func1");
    }
}
