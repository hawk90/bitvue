//! RingBuffer - A fixed-size ring buffer (circular buffer)

use alloc::vec::Vec;

/// A fixed-size ring buffer (circular buffer).
///
/// Elements are added in a circular pattern, overwriting old elements when full.
#[derive(Clone, Debug)]
pub struct RingBuffer<T> {
    data: Vec<Option<T>>,
    read_index: usize,
    write_index: usize,
    size: usize,
}

impl<T> RingBuffer<T> {
    /// Creates a new ring buffer with the given capacity.
    ///
    /// # Panics
    ///
    /// Panics if capacity is 0.
    #[inline]
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0, "Capacity must be greater than 0");
        RingBuffer {
            data: (0..capacity).map(|_| None).collect(),
            read_index: 0,
            write_index: 0,
            size: 0,
        }
    }

    /// Returns the capacity of the ring buffer.
    #[inline]
    pub const fn capacity(&self) -> usize {
        self.data.len()
    }

    /// Returns the number of elements in the ring buffer.
    #[inline]
    pub const fn len(&self) -> usize {
        self.size
    }

    /// Returns true if the ring buffer is empty.
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.size == 0
    }

    /// Returns true if the ring buffer is full.
    #[inline]
    pub const fn is_full(&self) -> bool {
        self.size == self.data.len()
    }

    /// Pushes a value into the ring buffer.
    ///
    /// If the buffer is full, this overwrites the oldest element.
    #[inline]
    pub fn push(&mut self, value: T) -> Option<T> {
        let old_value = self.data[self.write_index].replace(value);
        self.write_index = (self.write_index + 1) % self.data.len();

        if self.size < self.data.len() {
            self.size += 1;
            None
        } else {
            self.read_index = self.write_index;
            old_value
        }
    }

    /// Pops a value from the ring buffer.
    #[inline]
    pub fn pop(&mut self) -> Option<T> {
        if self.size == 0 {
            return None;
        }

        let value = self.data[self.read_index].take();
        self.read_index = (self.read_index + 1) % self.data.len();
        self.size -= 1;
        value
    }

    /// Peeks at the front element without removing it.
    #[inline]
    pub fn front(&self) -> Option<&T> {
        if self.size == 0 {
            return None;
        }
        self.data[self.read_index].as_ref()
    }

    /// Peeks at the back element without removing it.
    #[inline]
    pub fn back(&self) -> Option<&T> {
        if self.size == 0 {
            return None;
        }
        let index = if self.write_index == 0 {
            self.data.len() - 1
        } else {
            self.write_index - 1
        };
        self.data[index].as_ref()
    }

    /// Clears the ring buffer.
    #[inline]
    pub fn clear(&mut self) {
        for item in &mut self.data {
            *item = None;
        }
        self.read_index = 0;
        self.write_index = 0;
        self.size = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ring_buffer() {
        let mut rb = RingBuffer::new(3);
        assert_eq!(rb.capacity(), 3);
        assert!(rb.is_empty());

        assert_eq!(rb.push(1), None);
        assert_eq!(rb.push(2), None);
        assert_eq!(rb.push(3), None);
        assert!(rb.is_full());

        // Overwrites oldest
        assert_eq!(rb.push(4), Some(1));
        assert_eq!(rb.front(), Some(&2));
        assert_eq!(rb.len(), 3);

        assert_eq!(rb.pop(), Some(2));
        assert_eq!(rb.len(), 2);
    }

    #[test]
    fn test_ring_buffer_empty() {
        let mut rb: RingBuffer<i32> = RingBuffer::new(3);
        assert_eq!(rb.pop(), None);
        assert_eq!(rb.front(), None);
        assert_eq!(rb.back(), None);
    }
}
