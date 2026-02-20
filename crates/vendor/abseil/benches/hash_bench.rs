// Benchmarks for hash algorithms
#![cfg(bench)]

use abseil::absl_hash::{
    algorithms::{
        djb2_hash, fnv_hash, fnv_hash_128, fnv_hash_32, highway_hash, murmur3_64, siphash_24,
        wyhash, xxhash3_64, xxhash_64,
    },
    combiner::{combine_hashes_mult, combine_hashes_xor},
    hash::{hash_combine, hash_of, HashState},
    modern_hash::{blake2s_hash, blake3_hash, sha256_hash},
};
use test::{black_box, Bencher};

// Test data
fn small_data() -> Vec<u8> {
    vec![
        0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F,
        0x10,
    ]
}

fn medium_data() -> Vec<u8> {
    (0..256).map(|i| i as u8).collect()
}

fn large_data() -> Vec<u8> {
    (0..4096).map(|i| i as u8).collect()
}

fn xlarge_data() -> Vec<u8> {
    (0..16384).map(|i| i as u8).collect()
}

fn string_data() -> String {
    "The quick brown fox jumps over the lazy dog and then runs away to find more adventures"
        .to_string()
}

fn large_string_data() -> String {
    "Hello world ".repeat(1000)
}

// ========== hash_of ==========

#[bench]
fn bench_hash_of_small(b: &mut Bencher) {
    let data = small_data();
    b.iter(|| {
        let data: &[u8] = black_box(&data);
        black_box(hash_of(data));
    });
}

#[bench]
fn bench_hash_of_medium(b: &mut Bencher) {
    let data = medium_data();
    b.iter(|| {
        let data: &[u8] = black_box(&data);
        black_box(hash_of(data));
    });
}

#[bench]
fn bench_hash_of_large(b: &mut Bencher) {
    let data = large_data();
    b.iter(|| {
        let data: &[u8] = black_box(&data);
        black_box(hash_of(data));
    });
}

#[bench]
fn bench_hash_of_xlarge(b: &mut Bencher) {
    let data = xlarge_data();
    b.iter(|| {
        let data: &[u8] = black_box(&data);
        black_box(hash_of(data));
    });
}

// ========== HashState ==========

#[bench]
fn bench_hash_state_small(b: &mut Bencher) {
    let data = small_data();
    b.iter(|| {
        black_box(HashState::default().update(black_box(&data)).finalize());
    });
}

#[bench]
fn bench_hash_state_medium(b: &mut Bencher) {
    let data = medium_data();
    b.iter(|| {
        black_box(HashState::default().update(black_box(&data)).finalize());
    });
}

#[bench]
fn bench_hash_state_large(b: &mut Bencher) {
    let data = large_data();
    b.iter(|| {
        black_box(HashState::default().update(black_box(&data)).finalize());
    });
}

// ========== Hash Combine ==========
// hash_combine expects &[&T], so we need to create slices of references

#[bench]
fn bench_hash_combine_two(b: &mut Bencher) {
    let values: [u64; 2] = [42, 99];
    let refs: Vec<&u64> = values.iter().collect();
    b.iter(|| {
        black_box(hash_combine(black_box(refs.as_slice())));
    });
}

#[bench]
fn bench_hash_combine_three(b: &mut Bencher) {
    let values: [u64; 3] = [42, 99, 123];
    let refs: Vec<&u64> = values.iter().collect();
    b.iter(|| {
        black_box(hash_combine(black_box(refs.as_slice())));
    });
}

#[bench]
fn bench_hash_combine_four(b: &mut Bencher) {
    let values: [u64; 4] = [42, 99, 123, 456];
    let refs: Vec<&u64> = values.iter().collect();
    b.iter(|| {
        black_box(hash_combine(black_box(refs.as_slice())));
    });
}
