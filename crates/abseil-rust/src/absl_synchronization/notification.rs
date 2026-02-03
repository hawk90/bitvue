//! Notification primitive for one-time event signaling.
//!
//! This module provides a `Notification` type similar to Abseil's `absl::Notification`,
//! which allows a single event to be signaled and waited on.
//!
//! # Example
//!
//! ```rust
//! use abseil::absl_synchronization::notification::Notification;
//!
//! let notification = Notification::new();
//!
//! // In a thread, wait for the notification
//! let handle = std::thread::spawn({
//!     let notif = notification.clone();
//!     move || {
//!         notif.wait();
//!         println!("Notified!");
//!     }
//! });
//!
//! // Signal the notification
//! notification.notify();
//!
//! handle.join().unwrap();
//! ```

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// A notification primitive for one-time event signaling.
///
/// Once notified, all waiting threads are unblocked. Future calls to `wait`
/// return immediately.
#[derive(Clone, Default)]
pub struct Notification {
    notified: Arc<AtomicBool>,
}

impl Notification {
    /// Creates a new unnotified `Notification`.
    #[inline]
    pub fn new() -> Self {
        Self {
            notified: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Returns `true` if this notification has been notified.
    #[inline]
    pub fn has_been_notified(&self) -> bool {
        self.notified.load(Ordering::Acquire)
    }

    /// Marks this notification as notified.
    ///
    /// All threads waiting on this notification will be unblocked.
    #[inline]
    pub fn notify(&self) {
        self.notified.store(true, Ordering::Release);
        // Note: For a more efficient implementation with actual wakeups,
        // we would use a ConditionVariable. This simple version uses
        // busy-waiting for simplicity.
    }

    /// Blocks until this notification is notified.
    ///
    /// If already notified, returns immediately.
    ///
    /// # Performance Note
    ///
    /// Uses exponential backoff to avoid CPU exhaustion while waiting.
    #[inline]
    pub fn wait(&self) {
        let mut backoff = 1;
        while !self.has_been_notified() {
            // Exponential backoff: spin, then yield, then sleep
            if backoff <= 64 {
                for _ in 0..backoff {
                    if self.has_been_notified() {
                        return;
                    }
                    std::hint::spin_loop();
                }
                backoff *= 2;
            } else {
                // After sufficient spinning, yield to other threads
                std::thread::yield_now();
                // Small sleep to prevent busy-waiting
                #[cfg(feature = "std")]
                std::thread::sleep(std::time::Duration::from_micros(100));
            }
        }
    }

    /// Blocks until this notification is notified or the timeout expires.
    ///
    /// Returns `true` if notified, `false` if timeout occurred.
    #[inline]
    pub fn wait_for(&self, timeout: std::time::Duration) -> bool {
        let start = std::time::Instant::now();
        while !self.has_been_notified() {
            if start.elapsed() >= timeout {
                return false;
            }
            std::hint::spin_loop();
        }
        true
    }

    /// Blocks until this notification is notified or the deadline is reached.
    ///
    /// Returns `true` if notified, `false` if deadline passed.
    #[inline]
    pub fn wait_until(&self, deadline: std::time::Instant) -> bool {
        while !self.has_been_notified() {
            if std::time::Instant::now() >= deadline {
                return false;
            }
            std::hint::spin_loop();
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Barrier;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_new_notification() {
        let n = Notification::new();
        assert!(!n.has_been_notified());
    }

    #[test]
    fn test_default_notification() {
        let n = Notification::default();
        assert!(!n.has_been_notified());
    }

    #[test]
    fn test_notify() {
        let n = Notification::new();
        assert!(!n.has_been_notified());
        n.notify();
        assert!(n.has_been_notified());
    }

    #[test]
    fn test_wait_already_notified() {
        let n = Notification::new();
        n.notify();
        n.wait(); // Should return immediately
        assert!(n.has_been_notified());
    }

    #[test]
    fn test_wait_blocks() {
        let n = Notification::new();
        let barrier = Arc::new(Barrier::new(2));

        let handle = thread::spawn({
            let n = n.clone();
            let barrier = barrier.clone();
            move || {
                barrier.wait();
                n.wait();
                assert!(n.has_been_notified());
            }
        });

        barrier.wait();
        thread::sleep(Duration::from_millis(50));
        n.notify();

        handle.join().unwrap();
    }

    #[test]
    fn test_wait_for_timeout() {
        let n = Notification::new();
        let result = n.wait_for(Duration::from_millis(10));
        assert!(!result);
        assert!(!n.has_been_notified());
    }

    #[test]
    fn test_wait_for_success() {
        let n = Notification::new();
        let barrier = Arc::new(Barrier::new(2));

        let handle = thread::spawn({
            let n = n.clone();
            let barrier = barrier.clone();
            move || {
                barrier.wait();
                let result = n.wait_for(Duration::from_secs(1));
                assert!(result);
                assert!(n.has_been_notified());
            }
        });

        barrier.wait();
        thread::sleep(Duration::from_millis(10));
        n.notify();

        handle.join().unwrap();
    }

    #[test]
    fn test_clone() {
        let n1 = Notification::new();
        let n2 = n1.clone();

        n1.notify();
        assert!(n2.has_been_notified());
    }

    #[test]
    fn test_multiple_waiters() {
        let n = Notification::new();
        let barrier = Arc::new(Barrier::new(4)); // 3 waiters + main thread

        let mut handles = vec![];
        for _ in 0..3 {
            let n = n.clone();
            let barrier = barrier.clone();
            handles.push(thread::spawn(move || {
                barrier.wait();
                n.wait();
                assert!(n.has_been_notified());
            }));
        }

        barrier.wait();
        thread::sleep(Duration::from_millis(50));
        n.notify();

        for handle in handles {
            handle.join().unwrap();
        }
    }
}
