//! A blocking counter that waits until it reaches zero.
//!
//! This module provides a `BlockingCounter` similar to Abseil's `absl::BlockingCounter`,
//! which allows threads to wait until a counter reaches zero.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

/// A blocking counter that waits until it reaches zero.
///
/// Threads can wait on the counter, which will unblock when the count reaches zero.
/// Uses a condition variable for efficient blocking instead of busy-waiting.
#[derive(Clone)]
pub struct BlockingCounter {
    count: Arc<AtomicUsize>,
    condvar: Arc<Condvar>,
    // Mutex is needed for Condvar to work properly
    mutex: Arc<Mutex<()>>,
}

impl BlockingCounter {
    /// Creates a new `BlockingCounter` with the given initial count.
    #[inline]
    pub fn new(initial_count: usize) -> Self {
        Self {
            count: Arc::new(AtomicUsize::new(initial_count)),
            condvar: Arc::new(Condvar::new()),
            mutex: Arc::new(Mutex::new(())),
        }
    }

    /// Decrements the counter by one.
    ///
    /// # Panics
    ///
    /// Panics if the counter is already zero (underflow).
    #[inline]
    pub fn decrement(&self) {
        let old = self.count.fetch_sub(1, Ordering::AcqRel);
        if old == 0 {
            panic!("BlockingCounter underflow");
        }
        // Notify one waiting thread that count has decreased
        if old == 1 {
            // Count just reached zero, notify all waiters
            self.condvar.notify_all();
        } else {
            // Count decreased but not zero, notify one waiter
            self.condvar.notify_one();
        }
    }

    /// Returns the current count.
    #[inline]
    pub fn count(&self) -> usize {
        self.count.load(Ordering::Acquire)
    }

    /// Blocks until the counter reaches zero.
    ///
    /// If the counter is already zero, returns immediately.
    ///
    /// This uses a condition variable for efficient blocking instead of busy-waiting.
    #[inline]
    pub fn wait(&self) {
        while self.count.load(Ordering::Acquire) > 0 {
            let guard = self.mutex.lock().unwrap();
            let _guard = self.condvar.wait(guard).unwrap();
        }
    }

    /// Blocks until the counter reaches zero, with a timeout.
    ///
    /// Returns `true` if the counter reached zero, `false` if the timeout elapsed.
    ///
    /// # Example
    ///
    /// ```rust
    /// use abseil::absl_synchronization::blocking_counter::BlockingCounter;
    /// use std::time::Duration;
    ///
    /// let bc = BlockingCounter::new(5);
    /// // Wait up to 1 second for the counter to reach zero
    /// let reached = bc.wait_timeout(Duration::from_secs(1));
    /// ```
    #[inline]
    pub fn wait_timeout(&self, timeout: Duration) -> bool {
        let start = std::time::Instant::now();

        while self.count.load(Ordering::Acquire) > 0 {
            let remaining = timeout.saturating_sub(start.elapsed());
            if remaining.is_zero() {
                return false;
            }

            let guard = self.mutex.lock().unwrap();
            let (_guard, timeout_result) = self.condvar.wait_timeout(guard, remaining).unwrap();

            if timeout_result.timed_out() {
                return self.count.load(Ordering::Acquire) == 0;
            }
        }
        true
    }

    /// Checks if the counter has reached zero.
    #[inline]
    pub fn is_zero(&self) -> bool {
        self.count() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn test_new() {
        let bc = BlockingCounter::new(5);
        assert_eq!(bc.count(), 5);
        assert!(!bc.is_zero());
    }

    #[test]
    fn test_decrement() {
        let bc = BlockingCounter::new(3);
        bc.decrement();
        assert_eq!(bc.count(), 2);
        bc.decrement();
        assert_eq!(bc.count(), 1);
        bc.decrement();
        assert_eq!(bc.count(), 0);
        assert!(bc.is_zero());
    }

    #[test]
    #[should_panic(expected = "BlockingCounter underflow")]
    fn test_decrement_underflow() {
        let bc = BlockingCounter::new(0);
        bc.decrement();
    }

    #[test]
    fn test_wait_already_zero() {
        let bc = BlockingCounter::new(0);
        bc.wait(); // Should return immediately
    }

    #[test]
    fn test_wait_blocks() {
        let bc = Arc::new(BlockingCounter::new(3));
        let bc_clone = bc.clone();

        let handle = thread::spawn(move || {
            bc_clone.wait();
            assert!(bc_clone.is_zero());
        });

        // Give the thread time to start waiting
        thread::sleep(Duration::from_millis(10));

        bc.decrement();
        bc.decrement();
        bc.decrement();

        handle.join().unwrap();
    }

    #[test]
    fn test_wait_timeout_success() {
        let bc = Arc::new(BlockingCounter::new(2));
        let bc_clone = bc.clone();

        let handle = thread::spawn(move || {
            // Should complete within timeout
            assert!(bc_clone.wait_timeout(Duration::from_millis(100)));
        });

        thread::sleep(Duration::from_millis(10));
        bc.decrement();
        bc.decrement();

        handle.join().unwrap();
    }

    #[test]
    fn test_wait_timeout_failure() {
        let bc = BlockingCounter::new(5);
        // Timeout before count reaches zero
        assert!(!bc.wait_timeout(Duration::from_millis(10)));
    }

    #[test]
    fn test_wait_timeout_already_zero() {
        let bc = BlockingCounter::new(0);
        // Should return true immediately
        assert!(bc.wait_timeout(Duration::from_millis(100)));
    }

    #[test]
    fn test_multiple_threads() {
        let bc = Arc::new(BlockingCounter::new(10));
        let mut handles = vec![];

        // Spawn threads that will decrement the counter
        for _ in 0..10 {
            let bc = bc.clone();
            handles.push(thread::spawn(move || {
                bc.decrement();
            }));
        }

        // Wait for all decrements to complete
        for handle in handles {
            handle.join().unwrap();
        }

        assert!(bc.is_zero());
    }

    #[test]
    fn test_clone() {
        let bc1 = BlockingCounter::new(5);
        let bc2 = bc1.clone();

        bc1.decrement();
        assert_eq!(bc2.count(), 4);

        bc2.decrement();
        assert_eq!(bc1.count(), 3);
    }
}
