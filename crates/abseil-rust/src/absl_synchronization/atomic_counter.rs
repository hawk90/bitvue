//! Atomic counter utilities - simple counter with atomic operations.

use core::sync::atomic::{AtomicUsize, Ordering};

/// A simple atomic counter that can be incremented and decremented atomically.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_synchronization::AtomicCounter;
///
/// let counter = AtomicCounter::new(0);
/// counter.increment();
/// assert_eq!(counter.get(), 1);
/// counter.decrement();
/// assert_eq!(counter.get(), 0);
/// ```
#[derive(Debug)]
pub struct AtomicCounter {
    count: AtomicUsize,
}

impl AtomicCounter {
    /// Creates a new atomic counter with the specified initial value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_synchronization::AtomicCounter;
    ///
    /// let counter = AtomicCounter::new(10);
    /// assert_eq!(counter.get(), 10);
    /// ```
    #[inline]
    pub const fn new(initial: usize) -> Self {
        Self {
            count: AtomicUsize::new(initial),
        }
    }

    /// Increments the counter by 1 and returns the previous value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_synchronization::AtomicCounter;
    ///
    /// let counter = AtomicCounter::new(0);
    /// let prev = counter.increment();
    /// assert_eq!(prev, 0);
    /// assert_eq!(counter.get(), 1);
    /// ```
    #[inline]
    pub fn increment(&self) -> usize {
        self.count.fetch_add(1, Ordering::AcqRel)
    }

    /// Decrements the counter by 1 and returns the previous value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_synchronization::AtomicCounter;
    ///
    /// let counter = AtomicCounter::new(10);
    /// let prev = counter.decrement();
    /// assert_eq!(prev, 10);
    /// assert_eq!(counter.get(), 9);
    /// ```
    #[inline]
    pub fn decrement(&self) -> usize {
        self.count.fetch_sub(1, Ordering::AcqRel)
    }

    /// Adds a value to the counter and returns the previous value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_synchronization::AtomicCounter;
    ///
    /// let counter = AtomicCounter::new(0);
    /// let prev = counter.add(5);
    /// assert_eq!(prev, 0);
    /// assert_eq!(counter.get(), 5);
    /// ```
    #[inline]
    pub fn add(&self, value: usize) -> usize {
        self.count.fetch_add(value, Ordering::AcqRel)
    }

    /// Subtracts a value from the counter and returns the previous value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_synchronization::AtomicCounter;
    ///
    /// let counter = AtomicCounter::new(10);
    /// let prev = counter.sub(5);
    /// assert_eq!(prev, 10);
    /// assert_eq!(counter.get(), 5);
    /// ```
    #[inline]
    pub fn sub(&self, value: usize) -> usize {
        self.count.fetch_sub(value, Ordering::AcqRel)
    }

    /// Returns the current value.
    #[inline]
    pub fn get(&self) -> usize {
        self.count.load(Ordering::Acquire)
    }

    /// Sets the counter to a specific value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_synchronization::AtomicCounter;
    ///
    /// let counter = AtomicCounter::new(0);
    /// counter.set(100);
    /// assert_eq!(counter.get(), 100);
    /// ```
    #[inline]
    pub fn set(&self, value: usize) {
        self.count.store(value, Ordering::Release);
    }

    /// Resets the counter to zero and returns the previous value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_synchronization::AtomicCounter;
    ///
    /// let counter = AtomicCounter::new(10);
    /// let prev = counter.reset();
    /// assert_eq!(prev, 10);
    /// assert_eq!(counter.get(), 0);
    /// ```
    #[inline]
    pub fn reset(&self) -> usize {
        self.count.swap(0, Ordering::AcqRel)
    }

    /// Returns true if the counter is at zero.
    #[inline]
    pub fn is_zero(&self) -> bool {
        self.get() == 0
    }

    /// Waits (spins) until the counter reaches the target value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_synchronization::AtomicCounter;
    ///
    /// let counter = AtomicCounter::new(0);
    /// // In another thread:
    /// // counter.add(10);
    /// counter.wait_until(10); // Spins until count >= 10
    /// ```
    pub fn wait_until(&self, target: usize) {
        while self.get() < target {
            core::hint::spin_loop();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_atomic_counter() {
        let counter = AtomicCounter::new(10);
        assert_eq!(counter.get(), 10);

        let prev = counter.increment();
        assert_eq!(prev, 10);
        assert_eq!(counter.get(), 11);

        let prev = counter.decrement();
        assert_eq!(prev, 11);
        assert_eq!(counter.get(), 10);

        let prev = counter.add(5);
        assert_eq!(prev, 10);
        assert_eq!(counter.get(), 15);

        let prev = counter.sub(3);
        assert_eq!(prev, 15);
        assert_eq!(counter.get(), 12);

        counter.set(100);
        assert_eq!(counter.get(), 100);

        let prev = counter.reset();
        assert_eq!(prev, 100);
        assert_eq!(counter.get(), 0);
        assert!(counter.is_zero());
    }
}
