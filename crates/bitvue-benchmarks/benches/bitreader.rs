//! Benchmarks for BitReader operations
//!
//! BitReader is one of the most performance-critical components as it's called
//! millions of times during video bitstream parsing.

use bitvue_core::BitReader;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};

/// Benchmark reading 8 bits (single byte) repeatedly
fn bench_read_bits_8(c: &mut Criterion) {
    let mut group = c.benchmark_group("read_bits_8");

    for size in [256, 1024, 4096, 16384].iter() {
        let data = vec![0xAAu8; *size];
        group.throughput(Throughput::Bytes(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &_size| {
            b.iter(|| {
                let mut reader = BitReader::new(&data);
                for _ in 0..(*size / 8) {
                    black_box(reader.read_bits(8).unwrap());
                }
            });
        });
    }

    group.finish();
}

/// Benchmark reading variable bit sizes (1-32 bits)
fn bench_read_bits_variable(c: &mut Criterion) {
    let data = vec![0x55u8; 4096]; // Alternating bits pattern

    c.bench_function("read_bits_variable", |b| {
        b.iter(|| {
            let mut reader = BitReader::new(&data);
            let mut result = 0u32;
            for i in 0..1000 {
                let bits = (i % 31) + 1; // 1-32 bits
                result = result.wrapping_add(reader.read_bits(bits as u8).unwrap());
            }
            black_box(result);
        });
    });
}

/// Benchmark reading individual bits
fn bench_read_bit(c: &mut Criterion) {
    let data = vec![0xFFu8; 1024];

    c.bench_function("read_bit", |b| {
        b.iter(|| {
            let mut reader = BitReader::new(&data);
            for _ in 0..1024 {
                black_box(reader.read_bit().unwrap());
            }
        });
    });
}

/// Benchmark skip_bits operation
fn bench_skip_bits(c: &mut Criterion) {
    let data = vec![0u8; 65536]; // 64KB buffer

    let mut group = c.benchmark_group("skip_bits");

    for skip_bits in [8, 64, 512, 4096].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(skip_bits), skip_bits, |b, &skip| {
            b.iter(|| {
                let mut reader = BitReader::new(&data);
                for _ in 0..(65536 * 8 / skip) {
                    black_box(reader.skip_bits(skip).unwrap());
                }
            });
        });
    }

    group.finish();
}

/// Benchmark reading bytes in bulk
fn bench_read_bytes(c: &mut Criterion) {
    let data = vec![0xABu8; 16384]; // 16KB buffer
    let mut group = c.benchmark_group("read_bytes");

    for buf_size in [64, 256, 1024, 4096].iter() {
        group.throughput(Throughput::Bytes(*buf_size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(buf_size), buf_size, |b, &size| {
            b.iter(|| {
                let mut reader = BitReader::new(&data);
                let mut buf = vec![0u8; size];
                black_box(reader.read_bytes(&mut buf).unwrap());
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_read_bits_8,
    bench_read_bits_variable,
    bench_read_bit,
    bench_skip_bits,
    bench_read_bytes
);

criterion_main!(benches);
