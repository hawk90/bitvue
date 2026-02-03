//! Random number generation utilities.
//!
//! This module provides random utilities similar to Abseil's `absl/random` directory.
//!
//! # Overview
//!
//! The random utilities provide:
//! - Random bit generation with multiple algorithms
//! - Various probability distributions
//! - Random sampling and selection
//! - Shuffling utilities
//! - Seeding and initialization
//! - Cryptographic RNG interface
//!
//! # Modules
//!
//! - [`bit_gen`] - Random bit generator
//! - [`distributions`] - Random number distributions
//!
//! # Examples
//!
//! ```rust
//! use abseil::absl_random::*;
//!
//! // Generate random values
//! let mut rng = BitGen::new(12345);
//! let random_i32 = rng.gen_i32();
//! let random_u32 = rng.gen_u32();
//! let random_bool = rng.gen_bool();
//!
//! // Shuffle a slice
//! let mut data = vec![1, 2, 3, 4, 5];
//! shuffle(&mut data, &mut rng);
//!
//! // Sample from a collection
//! let items = vec!['a', 'b', 'c', 'd', 'e'];
//! let chosen = sample(&items, 3, &mut rng);
//! ```

// Core modules
pub mod bit_gen;
pub mod distributions;

// Seed and RNG algorithms
pub mod seed;
pub mod seed_seq;
pub mod xorshift64;
pub mod pcg32;
pub mod wyrand;

// Extra distributions
pub mod distributions_extra;

// Random generation utilities
pub mod random;

// Sampling utilities
pub mod sampling;
pub mod sampling_advanced;

// String and bytes
pub mod string_random;

// Range-based random
pub mod range_random;

// Advanced sampling
pub mod permutation;
pub mod reservoir;

// UUID
pub mod uuid;

// Thread-local RNG
pub mod thread_local;

// Re-exports from core modules
pub use bit_gen::BitGen;
pub use distributions::{Bernoulli, Uniform};

// Re-exports from seed module
pub use seed::{seed_from, seed_from_bytes, seed_from_time, Seed};

// Re-exports from seed_seq module
pub use seed_seq::SeedSeq;

// Re-exports from RNG algorithms
pub use xorshift64::XorShift64;
pub use pcg32::Pcg32;
pub use wyrand::WyRand;

// Re-exports from extra distributions
pub use distributions_extra::{
    Beta, Cauchy, ChiSquared, Exponential, Gamma, Laplace, LogNormal, Normal, Poisson, Weibull,
};

// Re-exports from random module
pub use random::{random_alphanumeric, random_ascii, random_bool, random_char, random_digit,
    random_lowercase, random_uppercase};

// Re-exports from sampling module
pub use sampling::{partial_shuffle, sample, sample_one, sample_with_replacement, shuffle,
    random_index};

// Re-exports from sampling_advanced module
pub use sampling_advanced::{coin_flip, random_duration, roll_dice, roll_die, uniform_sample,
    weighted_sample};

// Re-exports from string_random module
pub use string_random::{fill_bytes, random_bytes, random_string, random_string_from};

// Re-exports from range_random module
pub use range_random::{random_f32, random_f64, random_range_f32, random_range_f64,
    random_range_i16, random_range_i32, random_range_i64, random_range_i8, random_range_isize,
    random_range_u16, random_range_u32, random_range_u64, random_range_u8, random_range_usize};

// Re-exports from permutation module
pub use permutation::random_permutation;

// Re-exports from reservoir module
pub use reservoir::reservoir_sample;

// Re-exports from uuid module
pub use uuid::{format_uuid, random_uuid};

// Re-exports from thread_local module
pub use thread_local::{thread_rng, ThreadLocalRng};
