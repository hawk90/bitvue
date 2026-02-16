// Vendored code from Abseil C++ library - suppress Clippy warnings
#![allow(clippy::empty_line_after_doc_comments)]
#![allow(clippy::multiple_bound_locations)]
#![allow(clippy::op_ref)]
#![allow(clippy::doc_overindented_list_items)]
#![allow(clippy::useless_conversion)]
#![allow(clippy::bool_to_int_with_if)]
#![allow(clippy::redundant_closure)]
#![allow(clippy::get_first)]
#![allow(clippy::manual_saturating_arithmetic)]
#![allow(clippy::needless_ifs)]
#![allow(clippy::unnecessary_cast)]
#![allow(clippy::or_fun_call)]
#![allow(clippy::deref_addrof)]
#![allow(clippy::extra_unused_lifetimes)]
#![allow(clippy::unnecessary_map_or)]
#![allow(clippy::manual_range_contains)]
#![allow(clippy::manual_is_multiple_of)]
#![allow(clippy::manual_find)]
#![allow(clippy::should_implement_trait)]
#![allow(clippy::ptr_eq)]
#![allow(clippy::manual_strip)]
#![allow(clippy::len_without_is_empty)]
#![allow(clippy::needless_range_loop)]
#![allow(clippy::collapsible_if)]
#![allow(clippy::option_as_ref_deref)]
#![allow(clippy::filter_map_next)]
#![allow(clippy::collapsible_str_replace)]
#![allow(clippy::extra_unused_type_parameters)]
#![allow(clippy::manual_is_ascii_check)]
#![allow(clippy::sliced_string_as_bytes)]
#![allow(clippy::needless_borrow)]
#![allow(clippy::nonminimal_bool)]
#![allow(clippy::unnecessary_lazy_evaluations)]
#![allow(clippy::needless_lifetimes)]
#![allow(clippy::wrong_self_convention)]
#![allow(unused_variables)]
#![allow(unused_assignments)]
#![allow(unused_imports)]

//! Abseil Rust - Rust port of Google's Abseil Common Libraries (C++)
//!
//! This crate provides Rust equivalents of the [Abseil C++ libraries](https://abseil.io/),
//! adapted to Rust idioms while maintaining API compatibility where possible.
//!
//! # Feature Flags
//!
//! ## Core Features
//! - `std` - Enable standard library support (default)
//! - `alloc` - Enable alloc crate support (for Vec, String, etc. on no_std)
//!
//! ## Module Features
//! - `algorithm` - Search algorithms (binary search, lower/upper bound)
//! - `any` - Type erasure wrapper
//! - `base` - Core utilities (call_once, OnceFlag)
//! - `bits` - Bit manipulation (popcount, bit_width, rotate, etc.)
//! - `cleanup` - Cleanup/ScopeGuard implementation
//! - `container` - InlinedVector
//! - `crc` - CRC-32 and CRC-64 checksums
//! - `debugging` - Stack trace, symbolize, failure handler (requires std)
//! - `flags` - Flag parsing (requires std)
//! - `function_ref` - Type-erased function reference
//! - `graph` - Graph data structures & algorithms (BFS, DFS, Dijkstra, A*, maxflow)
//! - `hash` - Hash utilities (HashState, FNV, Murmur3, xxHash)
//! - `hash-blake2`, `hash-blake3`, `hash-sha256` - Modern hash algorithms
//! - `hash-all` - All hash algorithms
//! - `log` - Logging utilities (requires std)
//! - `memory` - Memory alignment utilities
//! - `meta` - Type traits
//! - `numeric` - Numeric utilities
//! - `numeric-int128`, `numeric-fixed-point`, `numeric-rational` - Specific numeric types
//! - `numeric-all` - All numeric utilities
//! - `profiling` - Profiling utilities
//! - `random` - BitGen, Bernoulli, Uniform distributions
//! - `sorting` - 10+ sorting algorithms
//! - `status` - Status and StatusOr for error handling
//! - `strings` - String utilities
//! - `strings-cord`, `strings-format`, `strings-charset` - Specific string utilities
//! - `strings-all` - All string utilities
//! - `synchronization` - Mutex, Notification, BlockingCounter
//! - `time` - Civil time, Duration
//! - `types` - Optional, Span
//! - `types-optional`, `types-span` - Specific type utilities
//! - `types-all` - All type utilities
//! - `utility` - MoveOnCopy, address_of, etc.
//! - `variant` - Variant type (type-safe union)
//!
//! ## Convenience Feature Groups
//! - `full` - All features (default)
//! - `full-no-std` - All features except std-dependent ones
//! - `minimal-no-std` - Core utilities only (base, bits, meta, utility)
//! - `web` - Common web/backend use case
//! - `embedded` - Common embedded/no-std use case
//! - `data-structures` - Container, graph, hash, sorting, types, variant
//!
//! # Quick Start
//!
//! ```rust
//! use abseil::{call_once, is_done, OnceFlag};
//!
//! fn main() {
//!     // One-time initialization
//!     static INIT: OnceFlag = OnceFlag::new();
//!     call_once(&INIT, || {
//!         println!("Initialized!");
//!     });
//! }
//! ```
//!
//! # Examples
//!
//! ## Using Specific Features
//!
//! ```toml
//! [dependencies]
//! abseil = { version = "0.1", features = ["hash", "strings"] }
//! ```
//!
//! ## Minimal no_std Build
//!
//! ```toml
//! [dependencies]
//! abseil = { version = "0.1", default-features = false, features = ["minimal-no-std"] }
//! ```

// ============================================================================
// Public API Modules
// ============================================================================

/// absl_base - Base utilities from Abseil's absl/base directory
#[cfg(feature = "base")]
pub mod absl_base {
    /// call_once - One-time function invocation
    pub mod call_once;

    /// attributes - Compiler attribute utilities
    pub mod attributes;

    /// optimization - Branch prediction hints
    pub mod optimization;

    /// macros - Useful utility macros
    pub mod macros;
}

/// absl_log - Logging utilities from Abseil's absl/log directory
#[cfg(feature = "log")]
pub mod absl_log {
    /// log - LOG macros (INFO, WARNING, ERROR, FATAL)
    pub mod log;

    /// check - CHECK macros for assertions
    pub mod check;

    /// vlog - VLOG (verbose logging) system
    pub mod vlog;

    /// severity - LogSeverity enum
    pub mod severity;

    /// die_if_null - DIE_IF_NULL macro for null checks
    pub mod die_if_null;

    /// rate_limit - Rate-limited logging macros
    pub mod rate_limit;

    /// config - Log configuration (VLOG levels, etc.)
    pub mod config;
}

/// absl_strings - String utilities from Abseil's absl/strings directory
#[cfg(feature = "strings")]
pub mod absl_strings {
    /// cord - Cord (rope data structure for large strings)
    #[cfg(feature = "strings-cord")]
    pub mod cord;

    /// str_split - String splitting utilities
    pub mod str_split;

    /// str_cat - Optimized string concatenation
    pub mod str_cat;

    /// ascii - ASCII-specific string utilities
    pub mod ascii;

    /// charset - Character set matching utilities
    #[cfg(feature = "strings-charset")]
    pub mod charset;

    /// internal - Internal string implementation details
    pub mod internal;

    /// numbers - String to number conversion utilities
    pub mod numbers;

    /// escaping - String escaping utilities
    pub mod escaping;

    /// str_format - String formatting utilities
    #[cfg(feature = "strings-format")]
    pub mod str_format;
}

/// absl_numeric - Numeric utilities from Abseil's absl/numeric directory
#[cfg(feature = "numeric")]
pub mod absl_numeric {
    /// int128 - 128-bit integer types with additional utilities
    #[cfg(feature = "numeric-int128")]
    pub mod int128;
}

/// absl_container - Container utilities from Abseil's absl/container directory
#[cfg(feature = "container")]
pub mod absl_container {
    /// inlined_vector - Vector with inline storage for small sizes
    pub mod inlined_vector;
}

/// absl_time - Time utilities from Abseil's absl/time directory
#[cfg(feature = "time")]
pub mod absl_time {
    /// civil_time - Civil time (date/time) for calendar operations
    pub mod civil_time;

    /// duration - Duration for representing time spans
    pub mod duration;
}

/// absl_types - Type utilities from Abseil's absl/types directory
#[cfg(feature = "types")]
pub mod absl_types {
    /// optional - Optional wrapper type
    #[cfg(feature = "types-optional")]
    pub mod optional;

    /// span - Span type for contiguous sequences
    #[cfg(feature = "types-span")]
    pub mod span;
}

/// absl_algorithm - Algorithm utilities from Abseil's absl/algorithm directory
#[cfg(feature = "algorithm")]
pub mod absl_algorithm;

/// absl_synchronization - Synchronization utilities from Abseil's absl/synchronization directory
#[cfg(feature = "synchronization")]
pub mod absl_synchronization {
    /// mutex - Mutex wrapper with additional utilities
    #[cfg(feature = "synchronization-mutex")]
    pub mod mutex;

    /// notification - One-time event signaling
    #[cfg(feature = "synchronization-notification")]
    pub mod notification;

    /// blocking_counter - Counter that blocks until reaching zero
    #[cfg(feature = "synchronization-blocking-counter")]
    pub mod blocking_counter;
}

/// absl_status - Status utilities from Abseil's absl/status directory
#[cfg(feature = "status")]
pub mod absl_status {
    /// status - Status type for error codes and messages
    pub mod status;

    /// statusor - StatusOr<T> type for returning status or a value
    pub mod statusor;
}

/// absl_hash - Hash utilities from Abseil's absl/hash directory
#[cfg(feature = "hash")]
pub mod absl_hash {
    /// hash - Hash state for combining hash values
    pub mod hash;
}

/// absl_memory - Memory utilities from Abseil's absl/memory directory
#[cfg(feature = "memory")]
pub mod absl_memory {
    /// memory - Memory alignment and related utilities
    pub mod memory;
}

/// absl_random - Random utilities from Abseil's absl/random directory
#[cfg(feature = "random")]
pub mod absl_random {
    /// bit_gen - Random bit generator
    pub mod bit_gen;

    /// distributions - Random number distributions
    pub mod distributions;
}

/// absl_flags - Flag utilities from Abseil's absl/flags directory
#[cfg(feature = "flags")]
pub mod absl_flags {
    /// flags - Flag parsing and common flags
    pub mod flags;
}

/// absl_function_ref - Function reference utilities from Abseil's absl/functional directory
#[cfg(feature = "function_ref")]
pub mod absl_function_ref {
    /// function_ref - Type-erased function reference
    pub mod function_ref;
}

/// absl_cleanup - Cleanup utilities from Abseil's absl/cleanup directory
#[cfg(feature = "cleanup")]
pub mod absl_cleanup {
    /// cleanup - Cleanup/ScopeGuard implementation
    pub mod cleanup;
}

/// absl_bits - Bit manipulation utilities from Abseil's absl/numeric/bits directory
#[cfg(feature = "bits")]
pub mod absl_bits {
    /// bits - Bit manipulation functions
    pub mod bits;
}

/// absl_variant - Variant type (type-safe union) from Abseil's absl/types directory
#[cfg(feature = "variant")]
pub mod absl_variant {
    /// variant - Variant type implementation
    pub mod variant;
}

/// absl_meta - Type traits and compile-time utilities from Abseil's absl/meta directory
#[cfg(feature = "meta")]
pub mod absl_meta {
    /// type_traits - Type trait utilities
    pub mod type_traits;
}

/// absl_utility - Utility functions from Abseil's absl/utility directory
#[cfg(feature = "utility")]
pub mod absl_utility {
    /// utility - Utility functions
    pub mod utility;
}

/// absl_any - Type erasure utilities from Abseil's absl/types directory
#[cfg(feature = "any")]
pub mod absl_any {
    /// any - Type erasure wrapper
    pub mod any;
}

/// absl_debugging - Debugging utilities from Abseil's absl/debugging directory
#[cfg(feature = "debugging")]
pub mod absl_debugging {
    /// failure_signal_handler - Failure signal handling
    pub mod failure_signal_handler;
    /// stacktrace - Stack trace utilities
    pub mod stacktrace;
    /// symbolize - Symbol/address lookup utilities
    pub mod symbolize;
}

/// absl_crc - CRC checksum utilities from Abseil's absl/crc directory
#[cfg(feature = "crc")]
pub mod absl_crc {
    /// crc32 - CRC-32 checksums
    pub mod crc32;
    /// crc64 - CRC-64 checksums
    pub mod crc64;
}

/// absl_graph - Graph data structures and algorithms
#[cfg(feature = "graph")]
pub mod absl_graph;

/// absl_profiling - Profiling and performance measurement utilities
#[cfg(feature = "profiling")]
pub mod absl_profiling;

/// absl_sorting - Advanced sorting algorithms
#[cfg(feature = "sorting")]
pub mod absl_sorting;

// ============================================================================
// Re-exports (public API)
// ============================================================================

// absl_base re-exports
#[cfg(feature = "base")]
pub use absl_base::call_once::{call_once, is_done, OnceFlag};

// absl_strings re-exports
#[cfg(feature = "strings")]
pub use absl_strings::escaping::UnescapeError;
#[cfg(feature = "strings")]
pub use absl_strings::escaping::{
    escape_c, escape_html, escape_url, unescape_c, unescape_html, unescape_url,
};

#[cfg(all(feature = "strings", feature = "strings-cord"))]
pub use absl_strings::cord::Cord;

#[cfg(feature = "strings")]
pub use absl_strings::str_cat::{str_join, AlphaNum, StrCat};

#[cfg(all(feature = "strings", feature = "strings-format"))]
pub use absl_strings::str_format::{
    append, appendv, format_delimited, format_with, formatted_len, human_readable_duration,
    human_readable_size, Spec, Stream,
};

// absl_numeric re-exports
#[cfg(all(feature = "numeric", feature = "numeric-int128"))]
pub use absl_numeric::int128::{int128, uint128};

// absl_container re-exports
#[cfg(feature = "container")]
pub use absl_container::inlined_vector::InlinedVector;

// absl_time re-exports
#[cfg(feature = "time")]
pub use absl_time::civil_time::{
    CivilDay, CivilHour, CivilMinute, CivilMonth, CivilSecond, CivilYear,
};
#[cfg(feature = "time")]
pub use absl_time::duration::Duration;

// absl_types re-exports
#[cfg(all(feature = "types", feature = "types-optional"))]
pub use absl_types::optional::Optional;
#[cfg(all(feature = "types", feature = "types-span"))]
pub use absl_types::span::{Span, SpanMut};

// absl_algorithm re-exports
#[cfg(feature = "algorithm")]
pub use absl_algorithm::search::{binary_search, binary_search_by, lower_bound, upper_bound};

// absl_synchronization re-exports
#[cfg(all(
    feature = "synchronization",
    feature = "synchronization-blocking-counter"
))]
pub use absl_synchronization::blocking_counter::BlockingCounter;
#[cfg(all(feature = "synchronization", feature = "synchronization-mutex"))]
pub use absl_synchronization::mutex::{Mutex, MutexGuard};
#[cfg(all(feature = "synchronization", feature = "synchronization-notification"))]
pub use absl_synchronization::notification::Notification;

// absl_status re-exports
#[cfg(feature = "status")]
pub use absl_status::status::{Status, StatusCode};
#[cfg(feature = "status")]
pub use absl_status::statusor::StatusOr;

// absl_hash re-exports
#[cfg(feature = "hash")]
pub use absl_hash::hash::HashState;

// absl_memory re-exports
#[cfg(feature = "memory")]
pub use absl_memory::memory::Alignment;

// absl_random re-exports
#[cfg(feature = "random")]
pub use absl_random::bit_gen::BitGen;
#[cfg(feature = "random")]
pub use absl_random::distributions::{Bernoulli, Uniform};

// absl_flags re-exports
#[cfg(feature = "flags")]
pub use absl_flags::flags::{BoolFlag, IntFlag, StringFlag, USAGE, VERBOSE};

// absl_function_ref re-exports
#[cfg(feature = "function_ref")]
pub use absl_function_ref::function_ref::{
    Callback, CallbackRegistry, FunctionCallback, FunctionRef,
};

// absl_cleanup re-exports
#[cfg(feature = "cleanup")]
pub use absl_cleanup::cleanup::{cleanup, failure_cleanup, Cleanup, FailureCleanup};

// absl_bits re-exports
#[cfg(feature = "bits")]
pub use absl_bits::bits::{
    bit_width, count_leading_zeros, count_trailing_zeros, highest_bit, is_power_of_two, lowest_bit,
    next_power_of_two, popcount, prev_power_of_two, reverse_bits, reverse_bytes, rotate_left,
    rotate_right,
};

// absl_variant re-exports
#[cfg(feature = "variant")]
pub use absl_variant::variant::{Variant, VariantMatchError};

// absl_meta re-exports
#[cfg(feature = "meta")]
pub use absl_meta::type_traits::{is_signed, is_unsigned, TypeIdentity};

// absl_utility re-exports
#[cfg(feature = "utility")]
pub use absl_utility::utility::{address_of, move_on_copy, MoveOnCopy};

// absl_any re-exports
#[cfg(feature = "any")]
pub use absl_any::any::Any;

// absl_debugging re-exports
#[cfg(feature = "debugging")]
pub use absl_debugging::stacktrace::{print_stack_trace, StackTrace};
#[cfg(feature = "debugging")]
pub use absl_debugging::symbolize::{demangle, symbolize};

// absl_crc re-exports
#[cfg(feature = "crc")]
pub use absl_crc::crc32::{crc32, crc32c};
#[cfg(feature = "crc")]
pub use absl_crc::crc64::crc64;

// absl_graph re-exports
#[cfg(feature = "graph")]
pub use absl_graph::{astar, bellman_ford, bfs, dfs, dijkstra, topological_sort};
#[cfg(feature = "graph")]
pub use absl_graph::{EdgeId, Graph, UndirectedGraph, VertexId, WeightedGraph};

// absl_profiling re-exports
#[cfg(feature = "profiling")]
pub use absl_profiling::{Counter, CounterGuard, Histogram, SampleRecorder};
#[cfg(feature = "profiling")]
pub use absl_profiling::{ProfileStats, Profiler, Timer, TimerGuard};

// absl_sorting re-exports
#[cfg(feature = "sorting")]
pub use absl_sorting::{heapsort, introsort, mergesort, quicksort, radix_sort, timsort};
#[cfg(feature = "sorting")]
pub use absl_sorting::{is_sorted, is_sorted_by, max, min, min_max, sort, sort_by};
#[cfg(feature = "sorting")]
pub use absl_sorting::{stable_sort, unstable_sort};

// absl_log re-exports
// Note: LOG, CHECK, and other macros are available at crate root via #[macro_export]

#[cfg(feature = "log")]
pub use absl_log::severity::LogSeverity;

#[cfg(feature = "log")]
pub use absl_log::config::{init_from_env, LogConfig};

// ============================================================================
// Prelude Module
// ============================================================================

/// Prelude module for convenient imports.
///
/// ```rust
/// use abseil::prelude::*;
/// ```
pub mod prelude {
    // absl_base
    #[cfg(feature = "base")]
    pub use crate::absl_base::call_once::{call_once, is_done, OnceFlag};

    // absl_log
    // Note: LOG, CHECK, VLOG macros are available at crate root via #[macro_export]

    #[cfg(feature = "log")]
    pub use crate::absl_log::severity::LogSeverity;

    #[cfg(feature = "log")]
    pub use crate::absl_log::config::{init_from_env, LogConfig};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prelude_imports() {
        // Test that prelude compiles with various features
        #[cfg(feature = "base")]
        {
            let flag = OnceFlag::new();
            assert!(!is_done(&flag));
        }
    }
}
