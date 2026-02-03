//! Bytes module - strongly-typed wrapper for byte slices.

use core::fmt;

/// A strongly-typed wrapper for byte slices.
///
/// This provides type safety when working with raw bytes and helps
/// prevent accidentally interpreting bytes as the wrong type.
///
/// # Examples
///
/// ```rust
//! use abseil::absl_memory::Bytes;
//!
//! let data = vec![0x01, 0x02, 0x03, 0x04];
//! let bytes = Bytes::new(&data);
//! assert_eq!(bytes.len(), 4);
//! ```
#[derive(Clone, Copy)]
pub struct Bytes<'a> {
    data: &'a [u8],
}

impl<'a> Bytes<'a> {
    /// Creates a new Bytes wrapper from a byte slice.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_memory::Bytes;
    ///
    /// let data = vec![1, 2, 3, 4];
    /// let bytes = Bytes::new(&data);
    /// ```
    pub const fn new(data: &'a [u8]) -> Self {
        Self { data }
    }

    /// Returns the underlying byte slice.
    pub const fn as_slice(&self) -> &'a [u8] {
        self.data
    }

    /// Returns the length of the byte slice.
    pub const fn len(&self) -> usize {
        self.data.len()
    }

    /// Returns true if the byte slice is empty.
    pub const fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Returns a pointer to the first byte.
    pub const fn as_ptr(&self) -> *const u8 {
        self.data.as_ptr()
    }

    /// Splits the bytes at a position.
    ///
    /// # Panics
    ///
    /// Panics if `mid` is greater than the length.
    pub const fn split_at(&self, mid: usize) -> (Bytes<'a>, Bytes<'a>) {
        let (first, second) = self.data.split_at(mid);
        (Bytes::new(first), Bytes::new(second))
    }

    /// Takes the first `n` bytes.
    ///
    /// # Panics
    ///
    /// Panics if `n` is greater than the length.
    pub const fn take(&self, n: usize) -> Bytes<'a> {
        Bytes::new(if n <= self.data.len() {
            // SAFETY: We just checked the bounds
            unsafe { self.data.get_unchecked(..n) }
        } else {
            &[]
        })
    }

    /// Skips the first `n` bytes.
    ///
    /// # Panics
    ///
    /// Panics if `n` is greater than the length.
    pub const fn skip(&self, n: usize) -> Bytes<'a> {
        Bytes::new(if n <= self.data.len() {
            // SAFETY: We just checked the bounds
            unsafe { self.data.get_unchecked(n..) }
        } else {
            &[]
        })
    }
}

impl<'a> fmt::Debug for Bytes<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Bytes({} bytes)", self.data.len())
    }
}

impl<'a> PartialEq for Bytes<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.data == other.data
    }
}

impl<'a> Eq for Bytes<'a> {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bytes_new() {
        let data = vec![1, 2, 3, 4];
        let bytes = Bytes::new(&data);
        assert_eq!(bytes.len(), 4);
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_bytes_take() {
        let data = vec![1, 2, 3, 4, 5];
        let bytes = Bytes::new(&data);
        let taken = bytes.take(3);
        assert_eq!(taken.len(), 3);
    }

    #[test]
    fn test_bytes_skip() {
        let data = vec![1, 2, 3, 4, 5];
        let bytes = Bytes::new(&data);
        let skipped = bytes.skip(2);
        assert_eq!(skipped.len(), 3);
    }

    #[test]
    fn test_bytes_split_at() {
        let data = vec![1, 2, 3, 4, 5];
        let bytes = Bytes::new(&data);
        let (first, second) = bytes.split_at(2);
        assert_eq!(first.len(), 2);
        assert_eq!(second.len(), 3);
    }
}
