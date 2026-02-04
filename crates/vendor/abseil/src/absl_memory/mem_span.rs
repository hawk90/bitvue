//! Memory span module - span with start and length.

/// A span of memory with start and length, useful for tracking allocations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MemSpan {
    /// Start address of the span.
    pub start: usize,
    /// Length of the span in bytes.
    pub length: usize,
}

impl MemSpan {
    /// Creates a new memory span.
    ///
    /// # Panics
    ///
    /// Panics if `start + length` would overflow.
    pub const fn new(start: usize, length: usize) -> Self {
        // Check for overflow at construction time
        let _ = match start.checked_add(length) {
            Some(_) => (),
            None => panic!("MemSpan: start + length would overflow"),
        };
        Self { start, length }
    }

    /// Returns the end address (exclusive).
    ///
    /// # Panics
    ///
    /// Panics if `start + length` would overflow.
    pub const fn end(&self) -> usize {
        // Use checked arithmetic to prevent overflow
        // In release mode, this will panic on overflow
        match self.start.checked_add(self.length) {
            Some(end) => end,
            None => panic!("MemSpan::end: overflow in start + length"),
        }
    }

    /// Returns the end address (exclusive), saturating at usize::MAX on overflow.
    ///
    /// This is a safer alternative to `end()` that never overflows, but may
    /// return an incorrect value on overflow.
    pub const fn end_saturating(&self) -> usize {
        self.start.saturating_add(self.length)
    }

    /// Returns true if this span contains the given address.
    pub const fn contains(&self, addr: usize) -> bool {
        addr >= self.start && addr < self.end()
    }

    /// Returns true if this span overlaps with another.
    pub const fn overlaps(&self, other: &MemSpan) -> bool {
        self.start < other.end() && other.start < self.end()
    }

    /// Returns the gap between two spans.
    ///
    /// Returns None if they overlap.
    pub const fn gap(&self, other: &MemSpan) -> Option<usize> {
        if self.overlaps(other) {
            return None;
        }
        if self.end() <= other.start {
            Some(other.start - self.end())
        } else {
            Some(self.start - other.end())
        }
    }

    /// Merges two adjacent or overlapping spans.
    ///
    /// Returns None if they are not adjacent or overlapping.
    pub fn merge(&self, other: &MemSpan) -> Option<MemSpan> {
        if !self.overlaps(other) && self.end() != other.start && other.end() != self.start {
            return None;
        }
        Some(MemSpan::new(
            self.start.min(other.start),
            self.end().max(other.end()) - self.start.min(other.start),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mem_span_new() {
        let span = MemSpan::new(0x1000, 0x100);
        assert_eq!(span.start, 0x1000);
        assert_eq!(span.length, 0x100);
        assert_eq!(span.end(), 0x1100);
    }

    #[test]
    fn test_mem_span_contains() {
        let span = MemSpan::new(0x1000, 0x100);
        assert!(span.contains(0x1000));
        assert!(span.contains(0x1050));
        assert!(!span.contains(0x1100));
        assert!(!span.contains(0x0FFF));
    }

    #[test]
    fn test_mem_span_overlaps() {
        let span1 = MemSpan::new(0x1000, 0x200);
        let span2 = MemSpan::new(0x1500, 0x200);
        assert!(span1.overlaps(&span2));
    }

    #[test]
    fn test_mem_span_gap() {
        let span1 = MemSpan::new(0x1000, 0x100);
        let span2 = MemSpan::new(0x1200, 0x100);
        assert_eq!(span1.gap(&span2), Some(0x100));
    }

    #[test]
    fn test_mem_span_merge() {
        let span1 = MemSpan::new(0x1000, 0x100);
        let span2 = MemSpan::new(0x1100, 0x100);
        let merged = span1.merge(&span2);
        assert!(merged.is_some());
        assert_eq!(merged.unwrap().length, 0x200);
    }
}
