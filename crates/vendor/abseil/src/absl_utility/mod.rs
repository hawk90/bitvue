//! Utility functions.
//!
//! This module provides utility functions (similar to Abseil's `absl/utility`)
//! which are general-purpose helper functions and types.
//!
//! Rust's standard library already provides many of these through `core::mem`,
//! but this module provides additional compatibility helpers and Abseil-specific
//! utilities.
//!
//! # Overview
//!
//! Utility functions provide common helpers that don't fit into other categories.
//! These include:
//!
//! - Memory utilities (address operations, size calculations)
//! - Type utilities (casting, identity)
//! - Comparison helpers
//! - Swap and exchange operations
//! - Scope guard utilities
//! - Callable and invoker utilities
//! - Hash combinators
//! - Debug and assertion utilities
//!
//! # Modules
//!
//! - [`utility`] - Original utility functions module
//!
//! # Examples
//!
//! ```rust
//! use abseil::absl_utility::{address_of, swap, clamp};
//!
//! // Get a reference's address as a usize
//! let value = 42;
//! let addr = address_of(&value);
//!
//! // Swap two values
//! let mut a = 1;
//! let mut b = 2;
//! swap(&mut a, &mut b);
//! assert_eq!(a, 2);
//! assert_eq!(b, 1);
//!
//! // Clamp a value
//! assert_eq!(clamp(15, 0, 10), 10);
//! ```

pub mod utility;

// Submodules
pub mod alignment;
pub mod comparison;
pub mod debug_macros;
pub mod error_handling;
pub mod hashing;
pub mod identity;
pub mod lazy_eval;
pub mod memory;
pub mod move_utils;
pub mod scope_guard;
pub mod string_utils;
pub mod swap_exchange;
pub mod traits;
pub mod type_casting;
pub mod testing;

// Re-exports from utility module (original)
pub use utility::{address_of, move_on_copy};

// Memory utilities
pub use memory::{
    address_of, address_of_mut, align_of_val, is_ptr_aligned, memcpy,
    memmove, memset_zero, null_ptr, null_ptr_mut, read_volatile, size_of_val,
    write_volatile,
};

// Type casting utilities
pub use type_casting::{bit_cast, transmute, type_name};

// Comparison utilities
pub use comparison::{clamp, cmp_max, cmp_min, is_between, is_in_range, max, median, min};

// Swap and exchange utilities
pub use swap_exchange::{exchange, replace, swap, take};

// Scope guard utilities
pub use scope_guard::{CleanupStack, DeferGuard, ScopeGuard, on_scope_exit};

// Move utilities
pub use move_utils::{always_false, always_true, MoveOnCopy};

// Alignment utilities
pub use alignment::{
    align_down, align_up, is_aligned, is_ptr_aligned, is_valid_alignment,
    next_power_of_two,
};

// Utility traits
pub use traits::{AsBytes, BitPattern, Callable, Callable1, Callable2, FromBytes};

// Hashing utilities
pub use hashing::{combine_hash, fnv1a_hash, hash_bytes, hash_bytes_32, HashCombiner};

// String utilities
pub use string_utils::{is_ascii, static_str, to_ascii_lower, to_ascii_upper, truncate};

// Error handling utilities
pub use error_handling::{
    err_or_none, flatten_result, map_err, map_ok, ok_or_none, unwrap_or_default,
    unwrap_or_else,
};

// Lazy evaluation
pub use lazy_eval::Lazy;

// Identity utilities
pub use identity::{Borrowed, Owned, Rc, RcRefCell};

// Debug macros
pub use debug_macros::{debug_assert_checked, get_unchecked_checked, unreachable_checked};

// Testing utilities
pub use testing::{approx_eq, approx_eq_rel, is_finite, is_infinite, is_nan};
