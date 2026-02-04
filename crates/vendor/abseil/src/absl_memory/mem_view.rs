//! Memory view module - read-only view into memory.

use core::fmt;
use core::ptr;

use crate::absl_memory::mem_ops::memcmp_ord;

/// A read-only view into memory with bounds checking.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_memory::MemView;
///
/// let data = vec![1, 2, 3, 4, 5];
/// let view = MemView::new(&data);
/// assert_eq!(view.len(), 5);
/// ```
pub struct MemView<'a> {
    data: &'a [u8],
}

impl<'a> MemView<'a> {
    /// Creates a new memory view.
    pub fn new(data: &'a [u8]) -> Self {
        Self { data }
    }

    /// Returns the length of the view.
    pub const fn len(&self) -> usize {
        self.data.len()
    }

    /// Returns true if the view is empty.
    pub const fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Reads a value at the given offset.
    ///
    /// Returns None if the read would be out of bounds.
    pub fn read<T: Copy>(&self, offset: usize) -> Option<T> {
        let size = core::mem::size_of::<T>();
        let end = offset.checked_add(size)?;
        if end > self.data.len() {
            return None;
        }
        // SAFETY: We checked the bounds
        unsafe { Some(ptr::read(self.data.as_ptr().add(offset) as *const T)) }
    }

    /// Gets a byte at the given offset.
    pub fn get(&self, offset: usize) -> Option<u8> {
        self.data.get(offset).copied()
    }

    /// Creates a sub-view starting at the given offset.
    pub fn sub_view(&self, offset: usize) -> Option<MemView<'a>> {
        self.data.get(offset..).map(|slice| MemView::new(slice))
    }

    /// Creates a sub-view with the given range.
    pub fn sub_view_range(&self, start: usize, end: usize) -> Option<MemView<'a>> {
        self.data.get(start..end).map(|slice| MemView::new(slice))
    }

    /// Searches for a byte sequence in the view.
    pub fn find(&self, pattern: &[u8]) -> Option<usize> {
        if pattern.is_empty() {
            return Some(0);
        }
        if pattern.len() > self.data.len() {
            return None;
        }
        for i in 0..=(self.data.len() - pattern.len()) {
            if &self.data[i..i + pattern.len()] == pattern {
                return Some(i);
            }
        }
        None
    }

    /// Compares this view with another.
    pub fn compare(&self, other: &MemView<'a>) -> i32 {
        memcmp_ord(self.data, other.data)
    }
}

impl<'a> fmt::Debug for MemView<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MemView({} bytes)", self.data.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mem_view_new() {
        let data = vec![1, 2, 3, 4, 5];
        let view = MemView::new(&data);
        assert_eq!(view.len(), 5);
        assert!(!view.is_empty());
    }

    #[test]
    fn test_mem_view_read() {
        let data = vec![1, 2, 3, 4, 5];
        let view = MemView::new(&data);
        assert_eq!(view.read::<u32>(0), Some(u32::from_le_bytes([1, 2, 3, 4])));
    }

    #[test]
    fn test_mem_view_read_out_of_bounds() {
        let data = vec![1, 2, 3, 4, 5];
        let view = MemView::new(&data);
        assert_eq!(view.read::<u32>(3), None); // Would read past end
    }

    #[test]
    fn test_mem_view_get() {
        let data = vec![1, 2, 3, 4, 5];
        let view = MemView::new(&data);
        assert_eq!(view.get(2), Some(3));
        assert_eq!(view.get(10), None);
    }

    #[test]
    fn test_mem_view_sub_view() {
        let data = vec![1, 2, 3, 4, 5];
        let view = MemView::new(&data);
        let sub = view.sub_view(2);
        assert!(sub.is_some());
        assert_eq!(sub.unwrap().len(), 3);
    }

    #[test]
    fn test_mem_view_find() {
        let data = b"hello world";
        let view = MemView::new(data);
        assert_eq!(view.find(b"world"), Some(6));
        assert_eq!(view.find(b"xyz"), None);
    }

    #[test]
    fn test_mem_view_compare() {
        let a = b"abc";
        let b = b"abd";
        let view_a = MemView::new(a);
        let view_b = MemView::new(b);
        assert!(view_a.compare(&view_b) < 0);
    }
}
