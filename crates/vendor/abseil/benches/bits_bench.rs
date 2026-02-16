// Comprehensive benchmarks for bit manipulation operations
#![cfg(bench)]

// Use functions re-exported at crate level from absl_bits
use abseil::{
    count_leading_zeros, count_trailing_zeros, is_power_of_two, next_power_of_two, popcount,
    prev_power_of_two, reverse_bits, reverse_bytes, rotate_left, rotate_right,
};
use test::{black_box, Bencher};

// ========== Popcount ==========

#[bench]
fn bench_popcount_u8_sparse(b: &mut Bencher) {
    b.iter(|| {
        black_box(popcount(black_box(0x01u8)));
    });
}

#[bench]
fn bench_popcount_u8_dense(b: &mut Bencher) {
    b.iter(|| {
        black_box(popcount(black_box(0xFFu8)));
    });
}

#[bench]
fn bench_popcount_u16_sparse(b: &mut Bencher) {
    b.iter(|| {
        black_box(popcount(black_box(0x0101u16)));
    });
}

#[bench]
fn bench_popcount_u16_dense(b: &mut Bencher) {
    b.iter(|| {
        black_box(popcount(black_box(0xFFFFu16)));
    });
}

#[bench]
fn bench_popcount_u32_sparse(b: &mut Bencher) {
    b.iter(|| {
        black_box(popcount(black_box(0x01010101u32)));
    });
}

#[bench]
fn bench_popcount_u32_dense(b: &mut Bencher) {
    b.iter(|| {
        black_box(popcount(black_box(0xFFFFFFFFu32)));
    });
}

#[bench]
fn bench_popcount_u64_sparse(b: &mut Bencher) {
    b.iter(|| {
        black_box(popcount(black_box(0x0101010101010101u64)));
    });
}

#[bench]
fn bench_popcount_u64_dense(b: &mut Bencher) {
    b.iter(|| {
        black_box(popcount(black_box(0xFFFFFFFFFFFFFFFFu64)));
    });
}

// ========== Count Leading Zeros ==========

#[bench]
fn bench_count_leading_zeros_u8(b: &mut Bencher) {
    b.iter(|| {
        black_box(count_leading_zeros(black_box(0x10u8)));
    });
}

#[bench]
fn bench_count_leading_zeros_u16(b: &mut Bencher) {
    b.iter(|| {
        black_box(count_leading_zeros(black_box(0x1000u16)));
    });
}

#[bench]
fn bench_count_leading_zeros_u32(b: &mut Bencher) {
    b.iter(|| {
        black_box(count_leading_zeros(black_box(0x00FF0000u32)));
    });
}

#[bench]
fn bench_count_leading_zeros_u64(b: &mut Bencher) {
    b.iter(|| {
        black_box(count_leading_zeros(black_box(0x00000000FFFFFFFFu64)));
    });
}

// ========== Count Trailing Zeros ==========

#[bench]
fn bench_count_trailing_zeros_u8(b: &mut Bencher) {
    b.iter(|| {
        black_box(count_trailing_zeros(black_box(0x10u8)));
    });
}

#[bench]
fn bench_count_trailing_zeros_u16(b: &mut Bencher) {
    b.iter(|| {
        black_box(count_trailing_zeros(black_box(0x1000u16)));
    });
}

#[bench]
fn bench_count_trailing_zeros_u32(b: &mut Bencher) {
    b.iter(|| {
        black_box(count_trailing_zeros(black_box(0x0000FF00u32)));
    });
}

#[bench]
fn bench_count_trailing_zeros_u64(b: &mut Bencher) {
    b.iter(|| {
        black_box(count_trailing_zeros(black_box(0xFFFFFFFF00000000u64)));
    });
}

// ========== Rotate Operations ==========

#[bench]
fn bench_rotate_left_u8_small(b: &mut Bencher) {
    b.iter(|| {
        black_box(rotate_left(black_box(0x12u8), black_box(2)));
    });
}

#[bench]
fn bench_rotate_left_u8_large(b: &mut Bencher) {
    b.iter(|| {
        black_box(rotate_left(black_box(0x12u8), black_box(6)));
    });
}

#[bench]
fn bench_rotate_left_u16_small(b: &mut Bencher) {
    b.iter(|| {
        black_box(rotate_left(black_box(0x1234u16), black_box(4)));
    });
}

#[bench]
fn bench_rotate_left_u32_small(b: &mut Bencher) {
    b.iter(|| {
        black_box(rotate_left(black_box(0x12345678u32), black_box(8)));
    });
}

#[bench]
fn bench_rotate_left_u64_small(b: &mut Bencher) {
    b.iter(|| {
        black_box(rotate_left(black_box(0x1234567890ABCDEFu64), black_box(16)));
    });
}

#[bench]
fn bench_rotate_right_u8_small(b: &mut Bencher) {
    b.iter(|| {
        black_box(rotate_right(black_box(0x12u8), black_box(2)));
    });
}

#[bench]
fn bench_rotate_right_u16_small(b: &mut Bencher) {
    b.iter(|| {
        black_box(rotate_right(black_box(0x1234u16), black_box(4)));
    });
}

#[bench]
fn bench_rotate_right_u32_small(b: &mut Bencher) {
    b.iter(|| {
        black_box(rotate_right(black_box(0x12345678u32), black_box(8)));
    });
}

#[bench]
fn bench_rotate_right_u64_small(b: &mut Bencher) {
    b.iter(|| {
        black_box(rotate_right(
            black_box(0x1234567890ABCDEFu64),
            black_box(16),
        ));
    });
}

// ========== Power of Two ==========

#[bench]
fn bench_is_power_of_two_u8_true(b: &mut Bencher) {
    b.iter(|| {
        black_box(is_power_of_two(black_box(64u8)));
    });
}

#[bench]
fn bench_is_power_of_two_u8_false(b: &mut Bencher) {
    b.iter(|| {
        black_box(is_power_of_two(black_box(63u8)));
    });
}

#[bench]
fn bench_is_power_of_two_u16_true(b: &mut Bencher) {
    b.iter(|| {
        black_box(is_power_of_two(black_box(1024u16)));
    });
}

#[bench]
fn bench_is_power_of_two_u16_false(b: &mut Bencher) {
    b.iter(|| {
        black_box(is_power_of_two(black_box(1023u16)));
    });
}

#[bench]
fn bench_is_power_of_two_u32_true(b: &mut Bencher) {
    b.iter(|| {
        black_box(is_power_of_two(black_box(1024u32)));
    });
}

#[bench]
fn bench_is_power_of_two_u32_false(b: &mut Bencher) {
    b.iter(|| {
        black_box(is_power_of_two(black_box(1023u32)));
    });
}

#[bench]
fn bench_is_power_of_two_u64_true(b: &mut Bencher) {
    b.iter(|| {
        black_box(is_power_of_two(black_box(1024u64)));
    });
}

#[bench]
fn bench_is_power_of_two_u64_false(b: &mut Bencher) {
    b.iter(|| {
        black_box(is_power_of_two(black_box(1023u64)));
    });
}

// ========== Round to Power of Two ==========

#[bench]
fn bench_next_power_of_two_u8(b: &mut Bencher) {
    b.iter(|| {
        black_box(next_power_of_two(black_box(100u8)));
    });
}

#[bench]
fn bench_next_power_of_two_u16(b: &mut Bencher) {
    b.iter(|| {
        black_box(next_power_of_two(black_box(1000u16)));
    });
}

#[bench]
fn bench_next_power_of_two_u32(b: &mut Bencher) {
    b.iter(|| {
        black_box(next_power_of_two(black_box(1000u32)));
    });
}

#[bench]
fn bench_next_power_of_two_u64(b: &mut Bencher) {
    b.iter(|| {
        black_box(next_power_of_two(black_box(1000u64)));
    });
}

#[bench]
fn bench_prev_power_of_two_u8(b: &mut Bencher) {
    b.iter(|| {
        black_box(prev_power_of_two(black_box(100u8)));
    });
}

#[bench]
fn bench_prev_power_of_two_u16(b: &mut Bencher) {
    b.iter(|| {
        black_box(prev_power_of_two(black_box(1000u16)));
    });
}

#[bench]
fn bench_prev_power_of_two_u32(b: &mut Bencher) {
    b.iter(|| {
        black_box(prev_power_of_two(black_box(1000u32)));
    });
}

#[bench]
fn bench_prev_power_of_two_u64(b: &mut Bencher) {
    b.iter(|| {
        black_box(prev_power_of_two(black_box(1000u64)));
    });
}

// ========== Reverse Bits ==========

#[bench]
fn bench_reverse_bits_u8(b: &mut Bencher) {
    b.iter(|| {
        black_box(reverse_bits(black_box(0b11010000u8)));
    });
}

#[bench]
fn bench_reverse_bits_u16(b: &mut Bencher) {
    b.iter(|| {
        black_box(reverse_bits(black_box(0b1101000000000000u16)));
    });
}

#[bench]
fn bench_reverse_bits_u32(b: &mut Bencher) {
    b.iter(|| {
        black_box(reverse_bits(black_box(0x12345678u32)));
    });
}

#[bench]
fn bench_reverse_bits_u64(b: &mut Bencher) {
    b.iter(|| {
        black_box(reverse_bits(black_box(0x1234567890ABCDEFu64)));
    });
}

// ========== Reverse Bytes ==========

#[bench]
fn bench_reverse_bytes_u16(b: &mut Bencher) {
    b.iter(|| {
        black_box(reverse_bytes(black_box(0x1234u16)));
    });
}

#[bench]
fn bench_reverse_bytes_u32(b: &mut Bencher) {
    b.iter(|| {
        black_box(reverse_bytes(black_box(0x12345678u32)));
    });
}

#[bench]
fn bench_reverse_bytes_u64(b: &mut Bencher) {
    b.iter(|| {
        black_box(reverse_bytes(black_box(0x1234567890ABCDEFu64)));
    });
}
