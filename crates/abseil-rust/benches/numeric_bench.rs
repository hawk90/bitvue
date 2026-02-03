// Minimal benchmarks for numeric operations
#![feature(test)]
extern crate test;

use test::{black_box, Bencher};

// Use functions that are actually exported from the library
// int128 and uint128 are functions that create int128/uint128 values
use abseil::{int128, uint128};

#[bench]
fn bench_int128_add(b: &mut Bencher) {
    let a_val = int128(1000);
    let b_val = int128(2000);
    b.iter(|| {
        black_box(black_box(a_val) + black_box(b_val));
    });
}

#[bench]
fn bench_int128_mul(b: &mut Bencher) {
    let a_val = int128(100);
    let b_val = int128(200);
    b.iter(|| {
        black_box(black_box(a_val) * black_box(b_val));
    });
}

#[bench]
fn bench_uint128_add(b: &mut Bencher) {
    let a_val = uint128(1000);
    let b_val = uint128(2000);
    b.iter(|| {
        black_box(black_box(a_val) + black_box(b_val));
    });
}

#[bench]
fn bench_uint128_mul(b: &mut Bencher) {
    let a_val = uint128(100);
    let b_val = uint128(200);
    b.iter(|| {
        black_box(black_box(a_val) * black_box(b_val));
    });
}
