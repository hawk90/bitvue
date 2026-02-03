//! Type utilities.
//!
//! This module provides type utilities similar to Abseil's `absl/types` directory.
//! Rust's standard library already provides many of these types through `core` and `std`,
//! but this module provides additional compatibility helpers and Abseil-specific utilities.
//!
//! # Overview
//!
//! Type utilities provide common type operations and wrapper types that enhance
//! Rust's built-in type system. These include:
//!
//! - Optional values ([`Optional`])
//! - Span types ([`Span`], [`SpanMut`])
//! - Type traits and utilities
//! - Wrapper types for special behaviors
//!
//! # Components
//!
//! - [`Optional<T>`] - Optional value wrapper (similar to `Option<T>` but with different semantics)
//! - [`Span<T>`] - Immutable span over a slice
//! - [`SpanMut<T>`] - Mutable span over a slice
//! - [`Wrapper<T>`] - Generic wrapper type
//! - [`NonNull<T>`] - Non-null pointer wrapper
//! - [`Owner<T>`] - Ownership marker type
//! - [`TypedIndex<T>`] - Typed index for preventing mixing indices
//! - [`Either<L, R>`] - Sum type with two variants
//! - [`CopyOnWrite<T>`] - Copy-on-write wrapper
//! - [`Lazy<T>`] - Lazy evaluation wrapper
//! - [`Bool<T>`] - Type-safe boolean wrapper
//! - [`Int<T>`] - Type-safe integer wrapper
//! - [`Pair<A, B>`] - Named tuple pair
//! - [`Triple<A, B, C>`] - Named tuple triple
//! - [`OneOf3<A, B, C>`] - Sum type with three variants
//! - [`OneOf4<A, B, C, D>`] - Sum type with four variants
//! - [`Aligned<N>`] - Type-level alignment marker
//! - [`Padded<T, N>`] - Padded type
//! - [`Pun<L, R>`] - Type-safe punning union
//! - [`Tagged<A, B>`] - Tagged union type
//! - [`Convertible<T>`] - Type conversion trait
//! - [`TypeId`] - Runtime type identifier
//! - [`Cmp<T>`] - Const-expression compatible comparison
//!
//! # Examples
//!
//! ```rust
//! use abseil::absl_types::{Optional, Span, Wrapper, NonNull, TypedIndex};
//!
//! // Create an optional value
//! let opt = Optional::some(42);
//! assert_eq!(opt.value(), &Some(42));
//!
//! // Create a span from a slice
//! let data = [1, 2, 3, 4, 5];
//! let span = Span::from(&data);
//! assert_eq!(span.len(), 5);
//!
//! // Use Wrapper for generic wrapping
//! let wrapped = Wrapper::new(42);
//! assert_eq!(wrapped.get(), &42);
//!
//! // Non-null pointer wrapper
//! let value = 42;
//! let non_null = NonNull::new(&value);
//! assert!(!non_null.is_null());
//!
//! // Typed index prevents mixing
//! struct EntityId;
//! struct ItemId;
//! let entity: TypedIndex<EntityId> = TypedIndex::new(5);
//! let item: TypedIndex<ItemId> = TypedIndex::new(5);
//! // entity != item at compile time
//! ```


extern crate alloc;

use alloc::boxed::Box;
use core::any::TypeId;
use core::fmt;
use core::marker::PhantomData;
use core::mem::ManuallyDrop;

// Submodules
pub mod copy_on_write;
pub mod either;
pub mod lazy;
pub mod misc_types;
pub mod typed_wrappers;
pub mod typed_index;
pub mod type_traits;
pub mod utility_functions;
pub mod wrappers;

// Existing modules
pub mod optional;
pub mod span;

// Re-exports - Existing modules
pub use optional::Optional;
pub use span::{Span, SpanMut};

// Re-exports - Wrapper types
pub use wrappers::{NonNull, Opaque, Owner, Pinned, Transparent, Wrapper};

// Re-exports - Typed index
pub use typed_index::TypedIndex;

// Re-exports - Type traits
pub use type_traits::{Cmp, Convertible, Downcast, TypeId, TypeIdOf};

// Re-exports - Copy on write
pub use copy_on_write::CopyOnWrite;

// Re-exports - Either
pub use either::Either;

// Re-exports - Utility functions
pub use utility_functions::{black_box, clamp, compare, cast, checked_cast, field_offset, offset_of, unreachable};

// Re-exports - Lazy
pub use lazy::Lazy;

// Re-exports - Typed wrappers
pub use typed_wrappers::{Bool, Int};

// Re-exports - Misc types
pub use misc_types::{Aligned, OneOf3, OneOf4, Padded, Pair, Pun, Tagged, Triple};

// Re-export utility functions at module level for convenience
pub use type_traits::{is_pointer, is_reference, is_same_type};

/// A newtype pattern wrapper.
///
/// This is the standard newtype wrapper that provides trait
/// implementations forwarding to the inner type.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_types::Newtype;
///
/// struct Meters(pub f64);
///
/// impl Newtype for Meters {
///     type Inner = f64;
/// }
/// ```
pub trait Newtype: Sized {
    /// The inner type being wrapped.
    type Inner;

    /// Creates a new newtype wrapper.
    #[inline]
    fn new(inner: Self::Inner) -> Self {
        // This would need to be implemented by each type
        // or use a blanket impl
        todo!("Newtype::new")
    }
}
