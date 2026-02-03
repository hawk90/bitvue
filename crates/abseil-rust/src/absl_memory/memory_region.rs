//! Memory region module - bounds-checked memory regions.

use core::fmt;

/// A memory region with bounds checking.
///
/// This represents a region of memory with start and end addresses,
/// providing bounds-safe operations.
///
/// # Examples
///
/// ```rust
//! use abseil::absl_memory::MemoryRegion;
//!
//! let region = MemoryRegion::new(0x1000, 0x2000);
//! assert_eq!(region.start(), 0x1000);
//! assert_eq!(region.end(), 0x2000);
//! assert_eq!(region.size(), 0x1000);
//! ```
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct MemoryRegion {
    start: usize,
    end: usize,
}

impl MemoryRegion {
    /// Creates a new memory region.
    ///
    /// # Panics
    ///
    /// Panics if `start > end`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_memory::MemoryRegion;
    ///
    /// let region = MemoryRegion::new(0x1000, 0x2000);
    /// ```
    pub const fn new(start: usize, end: usize) -> Self {
        assert!(start <= end, "MemoryRegion start must be <= end");
        Self { start, end }
    }

    /// Creates a region from a base address and size.
    ///
    /// # Panics
    ///
    /// Panics if the size would overflow.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_memory::MemoryRegion;
    ///
    /// let region = MemoryRegion::from_size(0x1000, 0x1000);
    /// ```
    pub const fn from_size(base: usize, size: usize) -> Self {
        let end = match base.checked_add(size) {
            Some(e) => e,
            None => panic!("MemoryRegion size overflow"),
        };
        Self::new(base, end)
    }

    /// Returns the start address of the region.
    pub const fn start(&self) -> usize {
        self.start
    }

    /// Returns the end address (exclusive) of the region.
    pub const fn end(&self) -> usize {
        self.end
    }

    /// Returns the size of the region in bytes.
    pub const fn size(&self) -> usize {
        self.end - self.start
    }

    /// Returns true if the region is empty.
    pub const fn is_empty(&self) -> bool {
        self.start == self.end
    }

    /// Checks if an address is within this region.
    pub const fn contains(&self, addr: usize) -> bool {
        addr >= self.start && addr < self.end
    }

    /// Checks if this region completely contains another region.
    pub const fn contains_region(&self, other: &MemoryRegion) -> bool {
        other.start >= self.start && other.end <= self.end
    }

    /// Checks if two regions overlap.
    pub const fn overlaps(&self, other: &MemoryRegion) -> bool {
        self.start < other.end && other.start < self.end
    }

    /// Returns the intersection of two regions.
    ///
    /// Returns None if they don't overlap.
    pub const fn intersection(&self, other: &MemoryRegion) -> Option<MemoryRegion> {
        let start = self.start.max(other.start);
        let end = self.end.min(other.end);
        if start < end {
            Some(MemoryRegion::new(start, end))
        } else {
            None
        }
    }

    /// Returns a sub-region of this region.
    ///
    /// # Panics
    ///
    /// Panics if the sub-region would exceed bounds.
    pub const fn sub_region(&self, offset: usize, size: usize) -> MemoryRegion {
        // Use checked arithmetic to prevent overflow in offset calculation
        let new_start = match self.start.checked_add(offset) {
            Some(s) => s,
            None => panic!("sub_region offset would overflow"),
        };
        let new_end = match new_start.checked_add(size) {
            Some(e) => e,
            None => panic!("sub_region would overflow"),
        };
        assert!(new_end <= self.end, "sub_region exceeds bounds");
        MemoryRegion::new(new_start, new_end)
    }

    /// Aligns this region up to the given alignment.
    ///
    /// # Panics
    ///
    /// Panics if `alignment` is not a power of two.
    pub const fn align_up(&self, alignment: usize) -> MemoryRegion {
        let aligned_start = (self.start + alignment - 1) & !(alignment - 1);
        MemoryRegion::new(aligned_start, self.end)
    }
}

impl fmt::Debug for MemoryRegion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "MemoryRegion(0x{:x}-0x{:x}, {} bytes)",
            self.start,
            self.end,
            self.size()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_region_new() {
        let region = MemoryRegion::new(0x1000, 0x2000);
        assert_eq!(region.start(), 0x1000);
        assert_eq!(region.end(), 0x2000);
        assert_eq!(region.size(), 0x1000);
    }

    #[test]
    fn test_memory_region_contains() {
        let region = MemoryRegion::new(0x1000, 0x2000);
        assert!(region.contains(0x1000));
        assert!(region.contains(0x1500));
        assert!(!region.contains(0x2000));
        assert!(!region.contains(0x0FFF));
    }

    #[test]
    fn test_memory_region_overlaps() {
        let region1 = MemoryRegion::new(0x1000, 0x2000);
        let region2 = MemoryRegion::new(0x1500, 0x2500);
        assert!(region1.overlaps(&region2));
    }

    #[test]
    fn test_memory_region_intersection() {
        let region1 = MemoryRegion::new(0x1000, 0x2000);
        let region2 = MemoryRegion::new(0x1500, 0x2500);
        let intersection = region1.intersection(&region2);
        assert!(intersection.is_some());
        assert_eq!(intersection.unwrap().start(), 0x1500);
    }

    // Tests for HIGH security fix - integer overflow in sub_region

    #[test]
    fn test_sub_region_normal() {
        let region = MemoryRegion::new(0x1000, 0x2000);
        let sub = region.sub_region(0x100, 0x500);
        assert_eq!(sub.start(), 0x1100);
        assert_eq!(sub.end(), 0x1600);
    }

    #[test]
    #[should_panic(expected = "sub_region offset would overflow")]
    fn test_sub_region_offset_overflow() {
        // Test that offset overflow is detected
        let region = MemoryRegion::new(usize::MAX - 0x100, usize::MAX);
        // This offset would overflow: (usize::MAX - 0x100) + 0x200
        region.sub_region(0x200, 0x100);
    }

    #[test]
    #[should_panic(expected = "sub_region would overflow")]
    fn test_sub_region_size_overflow() {
        // Test that size overflow is detected
        let region = MemoryRegion::new(0x1000, 0x2000);
        // This size would overflow: 0x1500 + usize::MAX
        region.sub_region(0x500, usize::MAX);
    }

    #[test]
    #[should_panic(expected = "sub_region exceeds bounds")]
    fn test_sub_region_exceeds_bounds() {
        // Test that bounds checking works after overflow protection
        let region = MemoryRegion::new(0x1000, 0x2000);
        // This would exceed the region bounds
        region.sub_region(0x1000, 0x100);
    }

    #[test]
    fn test_sub_region_from_size_no_overflow() {
        // Test that from_size correctly handles overflow
        let region = MemoryRegion::from_size(usize::MAX - 0x100, 0x100);
        assert_eq!(region.start(), usize::MAX - 0x100);
        assert_eq!(region.end(), usize::MAX);
    }

    #[test]
    #[should_panic(expected = "MemoryRegion size overflow")]
    fn test_from_size_overflow() {
        // Test that from_size correctly detects overflow
        MemoryRegion::from_size(usize::MAX - 0x100, 0x200);
    }
}
