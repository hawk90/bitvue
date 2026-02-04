//! Type erasure utilities.
//!
//! This module provides type erasure utilities (similar to Abseil's `absl/types/any.h`
//! or C++'s `std::any`). Rust's standard library already provides `core::any::Any`
//! for type erasure, but this module provides additional compatibility helpers.
//!
//! # Overview
//!
//! Type erasure allows you to work with values of unknown types through a common
//! interface. Rust's `core::any::Any` trait already provides this functionality.
//!
//! # Modules
//!
//! - [`any`] - Type erasure wrapper
//!
//! # Examples
//!
//! ```rust
//! use abseil::absl_any::any::Any;
//!
//! // Store any value
//! let any_value: Any = Any::new(42i32);
//! assert!(any_value.is::<i32>());
//! assert_eq!(*any_value.downcast_ref::<i32>().unwrap(), 42);
//! ```

pub mod any;

// Core type-erased containers
pub mod any_box;
pub mod clone_any;
pub mod any_map;

// Comparison and hashing
pub mod any_comparison;
pub mod any_hash;

// Debug and display
pub mod any_debug;

// Type name utilities
pub mod type_name;

// Visitor pattern
pub mod visitor;

// Builder pattern
pub mod any_builder;

// Function wrapper
pub mod any_function;

// Iterator
pub mod any_iterator;

// Channel
pub mod any_channel;

// Downcast utilities
pub mod downcast;

// Type utilities
pub mod type_utils;

// Conversion utilities
pub mod conversion;

// Re-exports from any module
pub use any::Any;

// Re-exports from any_box module
pub use any_box::{AnyBox, AnyBoxBuilder};

// Re-exports from clone_any module
pub use clone_any::CloneAny;

// Re-exports from any_map module
pub use any_map::AnyMap;

// Re-exports from any_comparison module
pub use any_comparison::{any_cmp, any_eq, AnyEq};

// Re-exports from any_hash module
pub use any_hash::{any_hash, AnyHash};

// Re-exports from any_debug module
pub use any_debug::{any_debug, any_display, AnyDebug, AnyDisplay};

// Re-exports from type_name module
pub use type_name::TypeName;

// Re-exports from visitor module
pub use visitor::{TypeCheckVisitor, TypeConstraintVisitor, TypePrinterVisitor, TypeNameCollector,
    TransformVisitor, AnyVisitor};

// Re-exports from any_builder module
pub use any_builder::AnyBuilder;

// Re-exports from any_function module
pub use any_function::AnyFunction;

// Re-exports from any_iterator module
pub use any_iterator::AnyIterator;

// Re-exports from any_channel module
pub use any_channel::{any_channel, AnyReceiver, AnySender};

// Re-exports from downcast module
pub use downcast::{downcast_mut, downcast_ref};

// Re-exports from type_utils module
pub use type_utils::{is_same_type, type_id_of, type_name_of};

// Re-exports from conversion module
pub use conversion::{from_any_box, to_any_box, to_clone_any, TypeErasable};
