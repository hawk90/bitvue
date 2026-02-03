//! Fixed buffer module - compile-time sized buffer.

use core::fmt;
use core::mem::MaybeUninit;

/// A fixed-size buffer with compile-time size tracking.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_memory::FixedBuffer;
///
/// let mut buffer: FixedBuffer<16> = FixedBuffer::new();
/// buffer.push(1);
/// buffer.push(2);
/// assert_eq!(buffer.len(), 2);
/// ```
#[derive(Clone, Copy)]
pub struct FixedBuffer<const N: usize> {
    data: [MaybeUninit<u8>; N],
    len: usize,
}

impl<const N: usize> FixedBuffer<N> {
    /// Creates a new empty fixed buffer.
    pub const fn new() -> Self {
        // SAFETY: Zero-initializing the array is safe for MaybeUninit<u8>
        // MaybeUninit<T> has the same size as T and can be zero-initialized
        Self {
            data: unsafe { core::mem::zeroed() },
            len: 0,
        }
    }

    /// Returns the capacity of the buffer.
    pub const fn capacity(&self) -> usize {
        N
    }

    /// Returns the current length.
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Returns true if the buffer is empty.
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns true if the buffer is full.
    pub const fn is_full(&self) -> bool {
        self.len >= N
    }

    /// Pushes a byte onto the buffer.
    ///
    /// Returns false if the buffer is full.
    pub fn push(&mut self, byte: u8) -> bool {
        if self.len >= N {
            return false;
        }
        self.data[self.len].write(byte);
        self.len += 1;
        true
    }

    /// Pops a byte from the buffer.
    ///
    /// Returns None if the buffer is empty.
    pub fn pop(&mut self) -> Option<u8> {
        if self.len == 0 {
            return None;
        }
        self.len -= 1;
        // SAFETY: We just decreased len, so this position is initialized
        Some(unsafe { self.data[self.len].assume_init_read() })
    }

    /// Clears the buffer.
    pub fn clear(&mut self) {
        self.len = 0;
    }

    /// Returns the remaining capacity.
    pub const fn remaining(&self) -> usize {
        N - self.len
    }

    /// Returns a slice of the initialized portion.
    pub fn as_slice(&self) -> &[u8] {
        // SAFETY: Only the first self.len bytes are initialized
        unsafe { core::slice::from_raw_parts(self.data.as_ptr() as *const u8, self.len) }
    }

    /// Returns a mutable slice of the initialized portion.
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        // SAFETY: Only the first self.len bytes are initialized
        unsafe { core::slice::from_raw_parts_mut(self.data.as_mut_ptr() as *mut u8, self.len) }
    }
}

impl<const N: usize> Default for FixedBuffer<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> fmt::Debug for FixedBuffer<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "FixedBuffer<{}/{}>", self.len, N)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fixed_buffer_new() {
        let buffer: FixedBuffer<16> = FixedBuffer::new();
        assert_eq!(buffer.capacity(), 16);
        assert!(buffer.is_empty());
        assert!(!buffer.is_full());
    }

    #[test]
    fn test_fixed_buffer_push() {
        let mut buffer: FixedBuffer<4> = FixedBuffer::new();
        assert!(buffer.push(1));
        assert!(buffer.push(2));
        assert_eq!(buffer.len(), 2);
    }

    #[test]
    fn test_fixed_buffer_full() {
        let mut buffer: FixedBuffer<2> = FixedBuffer::new();
        assert!(buffer.push(1));
        assert!(buffer.push(2));
        assert!(buffer.is_full());
        assert!(!buffer.push(3)); // Should fail
    }

    #[test]
    fn test_fixed_buffer_pop() {
        let mut buffer: FixedBuffer<4> = FixedBuffer::new();
        buffer.push(1);
        buffer.push(2);
        assert_eq!(buffer.pop(), Some(2));
        assert_eq!(buffer.pop(), Some(1));
        assert_eq!(buffer.pop(), None);
    }

    #[test]
    fn test_fixed_buffer_clear() {
        let mut buffer: FixedBuffer<4> = FixedBuffer::new();
        buffer.push(1);
        buffer.push(2);
        buffer.clear();
        assert!(buffer.is_empty());
    }

    #[test]
    fn test_fixed_buffer_remaining() {
        let mut buffer: FixedBuffer<8> = FixedBuffer::new();
        assert_eq!(buffer.remaining(), 8);
        buffer.push(1);
        buffer.push(2);
        assert_eq!(buffer.remaining(), 6);
    }

    #[test]
    fn test_fixed_buffer_as_slice() {
        let mut buffer: FixedBuffer<4> = FixedBuffer::new();
        buffer.push(1);
        buffer.push(2);
        buffer.push(3);
        assert_eq!(buffer.as_slice(), &[1, 2, 3]);
    }
}
