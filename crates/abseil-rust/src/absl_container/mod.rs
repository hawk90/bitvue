//! Container utilities.
//!
//! This module provides container utilities similar to Abseil's `absl/container` directory.
//! Rust's standard library already provides many containers through `alloc::vec`,
//! `alloc::collections`, but this module provides additional compatibility helpers
//! and Abseil-specific utilities.
//!
//! # Overview
//!
//! Container utilities provide common container operations and helper functions that
//! enhance Rust's built-in container system. These include:
//!
//! - Fixed-size arrays with runtime bounds checking
//! - Flat hash map utilities
//! - Node hash map utilities
//! - B-tree map utilities
//! - Container comparison helpers
//! - Container transformation utilities
//!
//! # Modules
//!
//! - [`inlined_vector`] - Vector with inline storage for small sizes (avoids heap allocation)
//!
//! # Examples
//!
//! ```rust
//! use abseil::absl_container::{contains, equal, flat_hash_map};
//!
//! // Check if a container contains a value
//! assert!(contains(&[1, 2, 3, 4, 5], &3));
//! assert!(!contains(&[1, 2, 3, 4, 5], &6));
//!
//! // Compare two containers
//! assert!(equal(&[1, 2, 3], &[1, 2, 3]));
//! assert!(!equal(&[1, 2, 3], &[1, 2, 4]));
//! ```

pub mod inlined_vector;

// Container query utilities
pub mod query;

// Container comparison utilities
pub mod comparison;

// Container transformation utilities
pub mod transformation;

// Container accumulation utilities
pub mod accumulation;

// Container modification utilities
pub mod modification;

// Container search utilities
pub mod search;

// Container sorting utilities
pub mod sorting;

// Fixed array
pub mod fixed_array;

// Flat hash map placeholder
pub mod flat_hash_map;

// Node hash map placeholder
pub mod node_hash_map;

// Ring buffer
pub mod ring_buffer;

// Stack adapter
pub mod stack;

// Queue adapter
pub mod queue;

// Array view
pub mod array_view;

// Chunked storage
pub mod chunked_storage;

// BTree utilities
pub mod btree;

// Concatenation utilities
pub mod concat;

// Re-exports
pub use inlined_vector::InlinedVector;

// Re-exports from query module
pub use query::{back, contains, front, is_empty, size};

// Re-exports from comparison module
pub use comparison::{compare, equal, equivalent};

// Re-exports from transformation module
pub use transformation::{filter, filter_map, flatten, transform};

// Re-exports from accumulation module
pub use accumulation::{max_element, min_element, minmax_element, product, sum};

// Re-exports from modification module
pub use modification::{append, clear, reserve, shrink_to_fit};

// Re-exports from search module
pub use search::{count_if, find_all, find_if, find_last_if};

// Re-exports from sorting module
pub use sorting::{is_sorted, reverse, sort, sort_by};

// Re-exports from fixed_array module
pub use fixed_array::FixedArray;

// Re-exports from flat_hash_map module
pub use flat_hash_map::{flat_hash_map, FlatHashMap};

// Re-exports from node_hash_map module
pub use node_hash_map::{node_hash_map, NodeHashMap};

// Re-exports from ring_buffer module
pub use ring_buffer::RingBuffer;

// Re-exports from stack module
pub use stack::Stack;

// Re-exports from queue module
pub use queue::Queue;

// Re-exports from array_view module
pub use array_view::ArrayView;

// Re-exports from chunked_storage module
pub use chunked_storage::ChunkedStorage;

// Re-exports from btree module
pub use btree::{btree_map, btree_set, get_or_insert, get_or_insert_with, BTreeMap, BTreeSet};

// Re-exports from concat module
pub use concat::{concat, concat_vecs};
