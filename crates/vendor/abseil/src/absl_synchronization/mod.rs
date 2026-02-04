//! Synchronization utilities.
//!
//! This module provides synchronization utilities similar to Abseil's `absl/synchronization` directory.
//!
//! # Overview
//!
//! The synchronization utilities provide:
//! - Mutex wrappers with additional utilities
//! - One-time event signaling (Notification)
//! - Blocking counter for waiting on multiple conditions
//! - Spinlock for low-level synchronization
//! - Barrier for synchronizing multiple threads
//! - Lock guard utilities
//! - Atomic operations helpers
//!
//! # Modules
//!
//! - [`mutex`] - Mutex wrapper with additional utilities
//! - [`notification`] - One-time event signaling
//! - [`blocking_counter`] - Counter that blocks until reaching zero
//! - [`spinlock`] - Spinlock primitive using busy-waiting
//! - [`barrier`] - Synchronize multiple threads at a barrier point
//! - [`reentrant_mutex`] - Reentrant mutex for recursive locking
//! - [`scope_guard`] - RAII guard for cleanup operations
//! - [`latch`] - Latch for countdown operations
//! - [`rwlock`] - Reader-writer lock
//! - [`atomic_counter`] - Atomic counter utilities
//! - [`alignment`] - Memory alignment utilities
//!
//! # Examples
//!
//! ```rust
//! use abseil::absl_synchronization::*;
//!
//! // Use notification for one-time signaling
//! let notification = Notification::new();
//! // In another thread:
//! // notification.notify();
//!
//! // Wait for the notification
//! notification.wait();
//!
//! // Use blocking counter to wait for multiple tasks
//! let counter = BlockingCounter::new(3);
//! // Each task calls counter.decrement() when done
//! // Wait for all tasks:
//! counter.wait();
//! ```

pub mod mutex;
pub mod notification;
pub mod blocking_counter;
pub mod spinlock;
pub mod barrier;
pub mod reentrant_mutex;
pub mod scope_guard;
pub mod latch;
pub mod rwlock;
pub mod atomic_counter;
pub mod alignment;
pub mod semaphore;

// Re-exports from mutex module
pub use mutex::{Mutex, MutexGuard};

// Re-exports from notification module
pub use notification::Notification;

// Re-exports from blocking_counter module
pub use blocking_counter::BlockingCounter;

// Re-exports from spinlock module
pub use spinlock::{Spinlock, SpinlockGuard};

// Re-exports from barrier module
pub use barrier::Barrier;

// Re-exports from reentrant_mutex module
pub use reentrant_mutex::{ReentrantMutex, ReentrantMutexGuard};

// Re-exports from scope_guard module
pub use scope_guard::{ScopeGuard, scope_guard};

// Re-exports from latch module
pub use latch::Latch;

// Re-exports from rwlock module
pub use rwlock::{RwLock, ReadGuard, WriteGuard};

// Re-exports from atomic_counter module
pub use atomic_counter::AtomicCounter;

// Re-exports from alignment module
pub use alignment::{is_aligned, align_up, align_down, align_up_diff};

// Re-exports from semaphore module
pub use semaphore::{Semaphore, SemaphorePermit, CountingSemaphore};
