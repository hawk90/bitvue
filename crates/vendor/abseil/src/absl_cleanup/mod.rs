//! Cleanup and scope guard utilities.
//!
//! This module provides cleanup utilities similar to Abseil's `absl/cleanup` directory.
//!
//! # Overview
//!
//! The cleanup utilities provide RAII-style cleanup guards that run functions
//! when they go out of scope. This is similar to Go's `defer` statement or
//! C++'s `ScopeGuard`.
//!
//! # Modules
//!
//! - [`cleanup`] - Basic cleanup guard that runs on drop
//! - [`cleanup_stack`] - CleanupStack for LIFO ordered cleanup actions
//! - [`resource_guard`] - ResourceGuard for managing resource lifetimes with access
//! - [`rollback`] - RollbackGuard for transaction-style rollback on failure
//! - [`deferred`] - DeferredCleanup for deferred execution
//! - [`cleanup_queue`] - CleanupQueue for FIFO ordered cleanup actions
//! - [`finally`] - FinallyGuard for try-finally pattern
//! - [`conditional`] - ConditionalCleanup for condition-based cleanup
//! - [`macros`] - Convenience macros for creating cleanup guards
//!
//! # Examples
//!
//! ```rust
//! use abseil::absl_cleanup::Cleanup;
//!
//! fn do_something() -> Result<(), String> {
//!     let resource = acquire_resource();
//!     let _cleanup = Cleanup::new(|| {
//!         release_resource(resource);
//!     });
//!
//!     // Do work with resource...
//!     // cleanup runs automatically when _cleanup goes out of scope
//!     Ok(())
//! }
//!
//! # fn acquire_resource() -> i32 { 42 }
//! # fn release_resource(_: i32) {}
//! ```
//!
//! # Failure-Only Cleanup
//!
//! ```rust
//! use abseil::absl_cleanup::RollbackGuard;
//!
//! fn transaction() -> Result<(), String> {
//!     let mut rollback = RollbackGuard::new(|| {
//!         println!("Rolling back transaction");
//!     });
//!
//!     // Do work...
//!     if error_condition() {
//!         return Err("error".to_string()); // rollback runs here
//!     }
//!
//!     rollback.commit(); // Success - don't rollback
//!     Ok(())
//! }
//! # fn error_condition() -> bool { false }
//! ```


extern crate alloc;

use alloc::sync::Arc;
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

pub mod cleanup;

// Re-exports from cleanup module
pub use cleanup::{
    cleanup, failure_cleanup, Cleanup, FailureCleanup,
};

// New modules
pub mod cleanup_stack;
pub mod resource_guard;
pub mod rollback;
pub mod deferred;
pub mod cleanup_queue;
pub mod finally;
pub mod macros;
pub mod conditional;

// Re-exports from cleanup_stack module
pub use cleanup_stack::CleanupStack;

// Re-exports from resource_guard module
pub use resource_guard::ResourceGuard;

// Re-exports from rollback module
pub use rollback::RollbackGuard;

// Re-exports from deferred module
pub use deferred::DeferredCleanup;

// Re-exports from cleanup_queue module
pub use cleanup_queue::CleanupQueue;

// Re-exports from finally module
pub use finally::FinallyGuard;

// Re-exports from conditional module
pub use conditional::ConditionalCleanup;

// Re-exports from macros module
pub use macros::{cleanup_stack, cleanup_queue, finally};

// Test helper - exported for use by other modules' tests
pub struct TestCounter {
    count: AtomicUsize,
}

impl TestCounter {
    pub fn new() -> Self {
        Self {
            count: AtomicUsize::new(0),
        }
    }

    pub fn inc(&self) {
        self.count.fetch_add(1, Ordering::SeqCst);
    }

    pub fn get(&self) -> usize {
        self.count.load(Ordering::SeqCst)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nested_cleanup_stacks() {
        let outer = TestCounter::new();
        let inner = TestCounter::new();

        {
            let mut outer_stack = CleanupStack::new();
            outer_stack.push(|| outer.inc());

            {
                let mut inner_stack = CleanupStack::new();
                inner_stack.push(|| inner.inc());
            }

            assert_eq!(inner.get(), 1);
            assert_eq!(outer.get(), 0);
        }

        assert_eq!(inner.get(), 1);
        assert_eq!(outer.get(), 1);
    }
}
