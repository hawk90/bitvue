//! Algorithm utilities.
//!
//! This module provides algorithm utilities similar to Abseil's `absl/algorithm` directory.
//! Rust's standard library already provides many algorithms through `core::slice` and
//! `core::iter`, but this module provides additional compatibility helpers and
//! Abseil-specific utilities.
//!
//! # Overview
//!
//! Algorithm utilities provide common algorithms that enhance Rust's built-in
//! algorithmic capabilities. These include:
//!
//! - Search algorithms (binary search, linear search)
//! - Sorting utilities
//! - Permutation and combination operations
//! - Comparison helpers
//! - Partitioning operations
//! - Heap utilities
//!
//! # Modules
//!
//! - [`search`] - Binary search and other search algorithms
//!
//! # Examples
//!
//! ```rust
//! use abseil::absl_algorithm::{clamp, is_sorted, rotate_left};
//!
//! // Check if a slice is sorted
//! assert!(is_sorted(&[1, 2, 3, 4, 5]));
//! assert!(!is_sorted(&[1, 3, 2, 4]));
//!
//! // Clamp a value to a range
//! assert_eq!(clamp(5, 0, 10), 5);
//! assert_eq!(clamp(-5, 0, 10), 0);
//! ```

// Internal utilities shared across modules
pub(crate) mod internal;

pub mod search;

// Sorting utilities
pub mod sorting;

// Linear search
pub mod linear_search;

// Rotation utilities
pub mod rotation;

// Comparison utilities
pub mod comparison;

// Partition utilities
pub mod partition;

// Permutation utilities
pub mod permutation;

// Fill utilities
pub mod fill;

// Unique utilities
pub mod unique;

// Swap utilities
pub mod swap;

// Heap utilities
pub mod heap;

// Selection utilities
pub mod selection;

// Binary search bounds
pub mod bounds;

// Lexicographical comparison
pub mod lexicographic;

// Set operations and merging
pub mod set_ops;

// Shuffle and sampling
pub mod sample;

// Accumulate and reduce
pub mod reduce;

// Transform utilities
pub mod transform;

// Searching
pub mod searching;

// Adjacent find
pub mod adjacent;

// Re-exports from search module
pub use search::{binary_search, binary_search_by};

// Re-exports from sorting module
pub use sorting::{is_sorted, is_sorted_by, is_sorted_descending};

// Re-exports from linear_search module
pub use linear_search::{linear_search, linear_search_by};

// Re-exports from rotation module
pub use rotation::{reverse, rotate_left, rotate_right};

// Re-exports from comparison module
pub use comparison::{clamp, max_element, min_element, minmax_element};

// Re-exports from partition module
pub use partition::{is_partitioned, partition, partition_point};

// Re-exports from permutation module
pub use permutation::{is_permutation, next_permutation, prev_permutation};

// Re-exports from fill module
pub use fill::{fill, fill_with};

// Re-exports from unique module
pub use unique::{is_unique, unique};

// Re-exports from swap module
pub use swap::{reverse_range, swap_elements};

// Re-exports from heap module
pub use heap::{is_heap, is_heap_by};

// Re-exports from selection module
pub use selection::nth_element;

// Re-exports from bounds module
pub use bounds::{equal_range, lower_bound, upper_bound};

// Re-exports from lexicographic module
pub use lexicographic::lexicographical_compare;

// Re-exports from set_ops module
pub use set_ops::{
    merge, merge_in_place, set_difference, set_intersection, set_symmetric_difference, set_union,
};

// Re-exports from sample module
pub use sample::{sample, shuffle};

// Re-exports from reduce module
pub use reduce::{accumulate, reduce};

// Re-exports from transform module
pub use transform::{equal_by, for_each_pair, transform, transform_copy};

// Re-exports from searching module
pub use searching::{count, find_all, find_last, search_subsequence};

// Re-exports from adjacent module
pub use adjacent::{adjacent_find, adjacent_find_by};
