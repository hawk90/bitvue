//! Benchmarks for MagicBytes const generic implementation
//!
//! Tests the performance of type-safe magic byte matching vs traditional byte array comparisons.

use bitvue_formats::container::MagicBytes;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};

/// Benchmark MagicBytes::matches vs manual byte slice comparison
fn bench_magic_bytes_matches(c: &mut Criterion) {
    let test_data: &[&[u8]] = &[
        b"DKIF\x00\x00\x00\x00",
        b"ftypmp42",
        b"RIFF\x00\x00\x00\x00AVI ",
        &[0x1A, 0x45, 0xDF, 0xA3, 0x00], // EBML
    ];

    let mut group = c.benchmark_group("magic_bytes_match");

    // Test MagicBytes matching
    group.bench_function("magic_bytes_type_safe", |b| {
        b.iter(|| {
            let dkif = MagicBytes::DKIF;
            let ftyp = MagicBytes::FTYP;
            let riff = MagicBytes::RIFF;
            let ebml = MagicBytes::EBML;

            for data in test_data {
                black_box(dkif.matches(data));
                black_box(ftyp.matches(data));
                black_box(riff.matches(data));
                black_box(ebml.matches(data));
            }
        });
    });

    // Test traditional byte slice comparison
    group.bench_function("magic_bytes_traditional", |b| {
        b.iter(|| {
            for data in test_data {
                black_box(data.len() >= 4 && &data[..4] == b"DKIF");
                black_box(data.len() >= 4 && &data[..4] == b"ftyp");
                black_box(data.len() >= 4 && &data[..4] == b"RIFF");
                black_box(data.len() >= 4 && &data[..4] == &[0x1A, 0x45, 0xDF, 0xA3]);
            }
        });
    });

    group.finish();
}

/// Benchmark MagicBytes::matches_at for offset matching
fn bench_magic_bytes_matches_at(c: &mut Criterion) {
    let buffer = {
        let mut buf = vec![0u8; 1024];
        // Insert magic bytes at various offsets
        buf[4..8].copy_from_slice(b"ftyp");
        buf[100..104].copy_from_slice(b"DKIF");
        buf[500..504].copy_from_slice(b"RIFF");
        buf
    };

    let mut group = c.benchmark_group("magic_bytes_offset");

    group.bench_function("type_safe_offset", |b| {
        b.iter(|| {
            let ftyp = MagicBytes::FTYP;
            let dkif = MagicBytes::DKIF;
            let riff = MagicBytes::RIFF;

            black_box(ftyp.matches_at(&buffer, 4));
            black_box(dkif.matches_at(&buffer, 100));
            black_box(riff.matches_at(&buffer, 500));
            black_box(!dkif.matches_at(&buffer, 4)); // Negative case
        });
    });

    group.bench_function("traditional_offset", |b| {
        b.iter(|| {
            black_box(buffer.len() >= 8 && &buffer[4..8] == b"ftyp");
            black_box(buffer.len() >= 104 && &buffer[100..104] == b"DKIF");
            black_box(buffer.len() >= 504 && &buffer[500..504] == b"RIFF");
            black_box(!(buffer.len() >= 104 && &buffer[4..8] == b"DKIF"));
        });
    });

    group.finish();
}

/// Benchmark creating MagicBytes constants vs byte arrays
fn bench_magic_bytes_creation(c: &mut Criterion) {
    c.bench_function("create_magic_bytes", |b| {
        b.iter(|| {
            black_box(MagicBytes::new(*b"TEST"));
            black_box(MagicBytes::new([0x1A, 0x45, 0xDF, 0xA3]));
            black_box(MagicBytes::DKIF);
            black_box(MagicBytes::FTYP);
        });
    });

    c.bench_function("create_byte_array", |b| {
        b.iter(|| {
            black_box(b"TEST" as &[u8]);
            black_box([0x1A, 0x45, 0xDF, 0xA3] as &[u8]);
        });
    });
}

criterion_group!(
    benches,
    bench_magic_bytes_matches,
    bench_magic_bytes_matches_at,
    bench_magic_bytes_creation
);

criterion_main!(benches);
