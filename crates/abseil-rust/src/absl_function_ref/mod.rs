//! Function reference utilities.
//!
//! This module provides function reference utilities similar to Abseil's `absl/functional` directory.
//! Rust's standard library already provides function pointer and closure support through `Fn`,
//! `FnMut`, and `Fn` traits, but this module provides additional compatibility helpers and
//! Abseil-specific utilities.
//!
//! # Overview
//!
//! Function reference utilities provide common function operations and helper functions that
//! enhance Rust's built-in function system. These include:
//!
//! - Type-erased function references (similar to `std::function`)
//! - Callback utilities for event handling
//! - Function composition helpers
//! - Memoization utilities
//! - Currying and partial application
//!
//! # Modules
//!
//! - [`function_ref`] - Type-erased function reference
//!
//! # Examples
//!
//! ```rust
//! use abseil::absl_function_ref::{compose, curry, memoize};
//!
//! // Compose two functions
//! let add_one = |x: i32| x + 1;
//! let double = |x: i32| x * 2;
//! let add_one_then_double = compose(double, add_one);
//! assert_eq!(add_one_then_double(5), 12);
//!
//! // Memoize a function
//! let fib = memoize(|n: u32| -> u32 {
//!     if n <= 1 { n } else { /* recursive calls */ 0 }
//! });
//! ```

pub mod function_ref;

// Function composition
pub mod composition;

// Currying and partial application
pub mod curry;

// Memoization
pub mod memoize;

// Identity utilities
pub mod identity;

// Comparison utilities
pub mod comparison;

// Logical utilities
pub mod logical;

// Conversion utilities
pub mod conversion;

// Function wrappers
pub mod wrappers;

// Conditional execution
pub mod conditional;

// Lazy evaluation
pub mod lazy;

// Function chaining
pub mod chaining;

// Tap/side effects
pub mod tap;

// Retry logic
pub mod retry;

// Adapters and wrappers
pub mod adapters;

// Predicate utilities
pub mod predicates;

// Slice operations and traits
pub mod slice_ops;

// Re-exports
pub use function_ref::{Callback, CallbackRegistry, FunctionCallback, FunctionRef};

// Re-exports from composition module
pub use composition::{complement, compose, constant, pipe};

// Re-exports from curry module
pub use curry::{apply_partial, curry, flip};

// Re-exports from memoize module
pub use memoize::{memoize, memoize_with};

// Re-exports from identity module
pub use identity::{function_name, id};

// Re-exports from comparison module
pub use comparison::{compare_by, compare_by_desc};

// Re-exports from logical module
pub use logical::{all_of, any_of, none_of};

// Re-exports from conversion module
pub use conversion::{to_fn, to_fn_mut};

// Re-exports from wrappers module
pub use wrappers::{catch_panic, zip_with};

// Re-exports from conditional module
pub use conditional::{branch, when};

// Re-exports from lazy module
pub use lazy::{lazy, Lazy};

// Re-exports from chaining module
pub use chaining::{apply_chain, chain};

// Re-exports from tap module
pub use tap::{tap, tap_mut};

// Re-exports from retry module
pub use retry::{retry, retry_n, RetryConfig};

// Re-exports from adapters module
pub use adapters::{map_error, map_result, unwrap_or, unwrap_or_else};

// Re-exports from predicates module
pub use predicates::{eq, gt, in_range, lt, ne};

// Re-exports from slice_ops module
pub use slice_ops::{
    all_slice, any_slice, callable, count_slice, find_slice, fold_slice, for_each_slice,
    map_slice, partition_slice, Apply, Callable,
};
