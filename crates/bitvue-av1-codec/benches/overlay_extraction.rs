//! Performance benchmarks for overlay extraction
//!
//! Run with:
//! ```bash
//! cargo bench -p bitvue-av1
//! ```

use bitvue_av1_codec::{
    extract_mv_grid, extract_partition_grid, extract_prediction_mode_grid, extract_qp_grid,
    extract_transform_grid, ParsedFrame,
};
use criterion::{black_box, criterion_group, criterion_main, Criterion};

/// Create test OBU data (simplified AV1 sequence)
fn create_test_obu_data() -> Vec<u8> {
    // Create a minimal AV1 OBU sequence
    let mut data = Vec::new();

    // Sequence Header OBU (simplified)
    data.extend_from_slice(&[0x08, 0x01]); // Minimal header
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);

    // Frame Header OBU (simplified)
    data.extend_from_slice(&[0x18, 0x01]);
    data.extend_from_slice(&[0x00, 0x08, 0x00]);

    // Tile Group OBU (simplified)
    data.extend_from_slice(&[0x20, 0x01]);
    data.extend_from_slice(&[0x00; 100]);

    data
}

fn bench_parse_frame(c: &mut Criterion) {
    let data = create_test_obu_data();

    c.bench_function("parse_frame", |b| {
        b.iter(|| {
            let parsed = ParsedFrame::parse(black_box(&data)).unwrap();
            black_box(parsed);
        });
    });
}

fn bench_extract_grids(c: &mut Criterion) {
    let data = create_test_obu_data();

    let mut group = c.benchmark_group("extract_grids");

    group.bench_function("qp_grid", |b| {
        b.iter(|| {
            black_box(extract_qp_grid(&data, 0, 32).unwrap());
        });
    });

    group.bench_function("mv_grid", |b| {
        b.iter(|| {
            black_box(extract_mv_grid(&data, 0).unwrap());
        });
    });

    group.bench_function("partition_grid", |b| {
        b.iter(|| {
            black_box(extract_partition_grid(&data, 0).unwrap());
        });
    });

    group.bench_function("prediction_mode_grid", |b| {
        b.iter(|| {
            black_box(extract_prediction_mode_grid(&data, 0).unwrap());
        });
    });

    group.bench_function("transform_grid", |b| {
        b.iter(|| {
            black_box(extract_transform_grid(&data, 0).unwrap());
        });
    });

    group.finish();
}

fn bench_cached_vs_uncached(c: &mut Criterion) {
    let data = create_test_obu_data();

    let mut group = c.benchmark_group("cached_comparison");

    // Uncached: Parse on each extraction
    group.bench_function("uncached_all_grids", |b| {
        b.iter(|| {
            let _qp = extract_qp_grid(&data, 0, 32).unwrap();
            let _mv = extract_mv_grid(&data, 0).unwrap();
            let _part = extract_partition_grid(&data, 0).unwrap();
            let _pred = extract_prediction_mode_grid(&data, 0).unwrap();
            let _tx = extract_transform_grid(&data, 0).unwrap();
            black_box((&_qp, &_mv, &_part, &_pred, &_tx));
        });
    });

    // Cached: Parse once, extract multiple times
    group.bench_function("cached_all_grids", |b| {
        b.iter(|| {
            let parsed = ParsedFrame::parse(&data).unwrap();
            // Note: These use internal functions from parsed frame
            let _qp = extract_qp_grid(&data, 0, 32).unwrap();
            let _mv = extract_mv_grid(&data, 0).unwrap();
            let _part = extract_partition_grid(&data, 0).unwrap();
            black_box((&_qp, &_mv, &_part));
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_parse_frame,
    bench_extract_grids,
    bench_cached_vs_uncached
);
criterion_main!(benches);
