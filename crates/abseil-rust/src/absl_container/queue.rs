//! Queue - A queue container adapter (FIFO)

use alloc::vec::Vec;

/// A queue container adapter (FIFO).
///
/// Provides queue operations on top of a vector.
#[derive(Clone, Debug)]
pub struct Queue<T> {
    data: Vec<T>,
    front_index: usize,
}

impl<T> Queue<T> {
    /// Creates a new empty queue.
    #[inline]
    pub fn new() -> Self {
        Self {
            data: Vec::new(),
            front_index: 0,
        }
    }

    /// Creates a queue with pre-allocated capacity.
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            data: Vec::with_capacity(capacity),
            front_index: 0,
        }
    }

    /// Enqueues a value at the back of the queue.
    #[inline]
    pub fn enqueue(&mut self, value: T) {
        self.data.push(value);
    }

    /// Dequeues a value from the front of the queue.
    #[inline]
    pub fn dequeue(&mut self) -> Option<T> {
        if self.front_index >= self.data.len() {
            return None;
        }

        let value = unsafe { self.data[self.front_index].clone() }; // Safe because we just checked
        self.front_index += 1;

        // Compact if we've consumed more than half the buffer
        if self.front_index > self.data.len() / 2 {
            self.data.drain(0..self.front_index);
            self.front_index = 0;
        }

        Some(value)
    }

    /// Peeks at the front value without removing it.
    #[inline]
    pub fn peek(&self) -> Option<&T> {
        if self.front_index < self.data.len() {
            Some(&self.data[self.front_index])
        } else {
            None
        }
    }

    /// Peeks at the back value without removing it.
    #[inline]
    pub fn peek_back(&self) -> Option<&T> {
        if self.front_index < self.data.len() {
            self.data.last()
        } else {
            None
        }
    }

    /// Returns the number of elements in the queue.
    #[inline]
    pub fn len(&self) -> usize {
        self.data.len() - self.front_index
    }

    /// Returns true if the queue is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.front_index >= self.data.len()
    }

    /// Clears the queue.
    #[inline]
    pub fn clear(&mut self) {
        self.data.clear();
        self.front_index = 0;
    }
}

impl<T: Default> Default for Queue<T> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_queue() {
        let mut queue = Queue::new();
        assert!(queue.is_empty());

        queue.enqueue(1);
        queue.enqueue(2);
        assert_eq!(queue.len(), 2);
        assert_eq!(queue.peek(), Some(&1));
        assert_eq!(queue.peek_back(), Some(&2));

        assert_eq!(queue.dequeue(), Some(1));
        assert_eq!(queue.dequeue(), Some(2));
        assert_eq!(queue.dequeue(), None);
    }

    #[test]
    fn test_queue_compaction() {
        let mut queue = Queue::with_capacity(10);
        for i in 0..10 {
            queue.enqueue(i);
        }
        for _ in 0..6 {
            queue.dequeue();
        }
        // Should compact
        assert_eq!(queue.len(), 4);
    }
}
