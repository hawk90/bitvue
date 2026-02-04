//! Variant type - Type-safe union.
//!
//! This module provides comprehensive variant types similar to Abseil's `absl::variant`
//! and C++17's `std::variant`. It includes type-safe unions, pattern matching,
//! monad-style operations, and various utility functions.
//!
//! # Overview
//!
//! The variant module provides several types for working with type-safe unions:
//!
//! - `Variant<T>` - Type-safe wrapper for single values
//! - `MultiVariant` - Enum that can hold one of many predefined types
//! - `OptionalVariant` - Variant that can be empty
//! - `Variant2<A, B>` to `Variant8<A, B, C, D, E, F, G, H>` - Generic variant types
//! - `ResultVariant<T, E>` - Variant for results with error info
//!
//! # Modules
//!
//! - [`variant`] - Variant type implementation
//!
//! # Examples
//!
//! ```rust
//! use abseil::absl_variant::{MultiVariant, Variant2, variant};
//!
//! // Using MultiVariant for common types
//! let v: MultiVariant = 42i32.into();
//! assert_eq!(v.type_name(), "i32");
//!
//! // Using Variant2 for custom type pairs
//! let v2: Variant2<i32, String> = Variant2::First(42);
//!
//! // Monad-style operations
//! let mapped = v2.map_first(|i| i * 2);
//! assert_eq!(mapped.as_first(), Some(&84));
//! ```

pub mod variant;

// Re-exports
pub use variant::{Variant, VariantMatchError};

// Submodules
pub mod builder;
pub mod error;
pub mod extended_variant;
pub mod generic_variants;
pub mod macros;
pub mod monad;
pub mod multi_variant;
pub mod optional;
pub mod result_variant;
pub mod utils;
pub mod visitor;

// MultiVariant re-exports
pub use multi_variant::MultiVariant;

// Visitor re-exports
pub use visitor::{ConvertVisitor, TypeCollector, Visitor, match_variant, same_variant_type, variant_convert, variant_type};

// Builder re-exports
pub use builder::VariantBuilder;

// OptionalVariant re-exports
pub use optional::OptionalVariant;

// Generic variants re-exports
pub use generic_variants::{Variant2, Variant3, Variant4, Variant5, Variant6, Variant7, Variant8};

// ResultVariant re-exports
pub use result_variant::ResultVariant;

// ExtendedVariant re-exports
pub use extended_variant::ExtendedVariant;

// Error re-exports
pub use error::VariantError;

// Utility function re-exports
pub use utils::{variant_cast, variant_clone, variant_compare, variant_eq_coerce, variant_hash, variant_parse, variant_validate};

// Macros
pub use macros::{variant, variant_match};
