//! Benchmark suite for Bitvue performance-critical code
//!
//! This crate provides Criterion-based benchmarks for measuring performance
//! of the most frequently called and optimization-sensitive functions in
//! the Bitvue video analysis pipeline.

#![cfg(test)]

extern crate criterion;

pub mod bitreader;
pub mod export;
pub mod magic_bytes;
