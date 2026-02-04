//! Hash utilities.
//!
//! This module provides hash utilities similar to Abseil's `absl/hash` directory.
//!
//! # Overview
//!
//! The hash utilities provide functions and types for computing and combining
//! hash values, similar to C++'s `<functional>` hash or Abseil's hash framework.
//!
//! # Modules
//!
//! - [`hash`] - Core hash types: HashState, Fingerprint, PiecewiseHasher, HashBuilder, TestHasher
//! - [`symmetric`] - SymmetricHash for order-independent hashing
//! - [`value_hash`] - ValueHash wrapper for value-based hashing
//! - [`hashable`] - Hashable trait and HashCache
//! - [`algorithms`] - Hash algorithms: FNV, MurmurHash3, xxHash, DJB2, SipHash
//! - [`combiner`] - Hash combination utilities
//! - [`rolling`] - RollingHash for sliding window hashing
//! - [`bloom`] - BloomFilter for approximate set membership
//! - [`set_ops`] - HashSetOps for set operations
//! - [`perfect`] - PerfectHash for small static sets
//! - [`collections`] - HashMap and HashSet marker types
//! - [`type_hash`] - Type-specific hash functions for Option, Result, BTreeSet
//!
//! # Examples
//!
//! ```rust
//! use abseil::absl_hash::{hash_of, hash_combine};
//!
//! // Single value hash
//! let hash = hash_of(&42);
//!
//! // Combined hash of multiple values
//! let combined = hash_combine(&[&1, &2, &3]);
//!
//! // Using HashState
//! let state = HashState::default()
//!     .update(&42)
//!     .update(&"hello")
//!     .finalize();
//! ```


extern crate alloc;

pub mod hash;

// Re-exports from hash module
pub use hash::{
    HashState, Fingerprint, PiecewiseHasher, HashBuilder, TestHasher,
    hash_of, hash_combine, mix_hashes, hash_slice, hash_pair, hash_triple,
};

// New modules
pub mod symmetric;
pub mod value_hash;
pub mod hashable;
pub mod algorithms;
pub mod combiner;
pub mod rolling;
pub mod bloom;
pub mod set_ops;
pub mod perfect;
pub mod collections;
pub mod type_hash;
pub mod modern_hash;

// Re-exports from symmetric module
pub use symmetric::SymmetricHash;

// Re-exports from value_hash module
pub use value_hash::ValueHash;

// Re-exports from hashable module
pub use hashable::{Hashable, HashCache};

// Re-exports from algorithms module
pub use algorithms::{
    deterministic_hash, fnv_hash, fnv_hash_32, fnv_hash_128,
    murmur3_mix, murmur3_64, xxhash_64, xxhash3_64, highway_hash, wyhash,
    djb2_hash, siphash_24,
};

// Re-exports from combiner module
pub use combiner::{
    combine_hashes_mult, combine_hashes_xor, rotate_left, rotate_right, hash_random,
};

// Re-exports from rolling module
pub use rolling::RollingHash;

// Re-exports from bloom module
pub use bloom::BloomFilter;

// Re-exports from set_ops module
pub use set_ops::HashSetOps;

// Re-exports from perfect module
pub use perfect::PerfectHash;

// Re-exports from collections module
pub use collections::{HashMap, HashSet};

// Re-exports from type_hash module
pub use type_hash::{hash_option, hash_result, hash_btree_set, hash_btree_set_symmetric};

// Re-exports from modern_hash module
pub use modern_hash::{blake2s_hash, blake3_hash, sha256_hash};

/// Macro for creating a HashBuilder with values.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_hash::hash_build;
///
/// let hash = hash_build!(42, "hello", true);
/// ```
#[macro_export]
macro_rules! hash_build {
    () => {
        $crate::absl_hash::HashBuilder::new().build()
    };
    ($($value:expr),+ $(,)?) => {{
        let mut builder = $crate::absl_hash::HashBuilder::new();
        $(
            builder = builder.add(&$value);
        )+
        builder.build()
    }};
}

/// Macro for computing hash of multiple values.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_hash::hash_values;
///
/// let hash = hash_values!(1, 2, 3, 4);
/// ```
#[macro_export]
macro_rules! hash_values {
    ($($value:expr),+ $(,)?) => {{
        $crate::absl_hash::hash_combine(&[$(&$value),+])
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_different_values_different_hashes() {
        let hash1 = hash_of(&42);
        let hash2 = hash_of(&43);
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_hash_build_macro() {
        let hash = hash_build!(1, 2, 3);
        assert_ne!(hash, 0);
    }

    #[test]
    fn test_hash_build_macro_empty() {
        let hash = hash_build!();
        assert_eq!(hash, 0); // Default HashState is 0
    }

    #[test]
    fn test_hash_values_macro() {
        let hash = hash_values!(1, 2, 3);
        assert_ne!(hash, 0);
    }
}
