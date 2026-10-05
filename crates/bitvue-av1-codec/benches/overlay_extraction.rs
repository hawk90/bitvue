//! Performance benchmarks for overlay extraction, run against real frames of the
//! `test_data/av1_test.ivf` fixture (a synthetic OBU blob would only measure the scaffold path).
//!
//! Run with:
//! ```bash
//! cargo bench -p bitvue-av1-codec --bench overlay_extraction
//! ```

use bitvue_av1_codec::overlay_extraction::ParsedFrame;
use bitvue_av1_codec::{
    extract_mv_grid_from_parsed, extract_partition_grid_from_parsed,
    extract_prediction_mode_grid_from_parsed, extract_qp_grid_from_parsed,
    extract_transform_grid_from_parsed, parse_ivf_frames, ObuIterator, ObuType,
};
use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;

const FIXTURE: &[u8] = include_bytes!("../../../test_data/av1_test.ivf");
const BASE_QP: i16 = 32;

/// Frame data blobs that each carry their own sequence header, so `ParsedFrame::parse` sees real
/// dimensions: frame 0 (key frame) and the largest inter frame among frames 1..30 with frame 0's sequence header
/// prepended.
fn real_frames() -> Vec<(&'static str, Vec<u8>)> {
    let (_, frames) = parse_ivf_frames(FIXTURE).expect("fixture parses");
    let key = frames[0].data.clone();

    let mut seq_header = Vec::new();
    let mut iter = ObuIterator::new(&key);
    while let Some(Ok(found)) = iter.next_obu_with_offset() {
        if found.obu.header.obu_type == ObuType::SequenceHeader {
            seq_header = key[found.offset..found.offset + found.consumed].to_vec();
            break;
        }
    }
    let mut inter = seq_header;
    // Most frames of this clip are a few dozen skip-only bytes; the biggest early one is the
    // representative "real inter frame" workload.
    let biggest = frames[1..30]
        .iter()
        .max_by_key(|f| f.data.len())
        .expect("fixture has inter frames");
    inter.extend_from_slice(&biggest.data);

    vec![("key_frame", key), ("inter_frame", inter)]
}

fn bench_parse_frame(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse_frame");
    for (name, data) in real_frames() {
        group.bench_function(name, |b| {
            b.iter(|| black_box(ParsedFrame::parse(black_box(&data)).unwrap()));
        });
    }
    group.finish();
}

fn bench_extract_grids(c: &mut Criterion) {
    for (name, data) in real_frames() {
        let parsed = ParsedFrame::parse(&data).unwrap();
        let mut group = c.benchmark_group(format!("extract_grids/{name}"));

        group.bench_function("qp_grid", |b| {
            b.iter(|| black_box(extract_qp_grid_from_parsed(&parsed, 0, BASE_QP).unwrap()));
        });
        group.bench_function("mv_grid", |b| {
            b.iter(|| black_box(extract_mv_grid_from_parsed(&parsed).unwrap()));
        });
        group.bench_function("partition_grid", |b| {
            b.iter(|| black_box(extract_partition_grid_from_parsed(&parsed).unwrap()));
        });
        group.bench_function("prediction_mode_grid", |b| {
            b.iter(|| black_box(extract_prediction_mode_grid_from_parsed(&parsed).unwrap()));
        });
        group.bench_function("transform_grid", |b| {
            b.iter(|| black_box(extract_transform_grid_from_parsed(&parsed).unwrap()));
        });

        group.finish();
    }
}

fn bench_parse_once_vs_per_grid(c: &mut Criterion) {
    let (_, data) = real_frames().remove(0);
    let mut group = c.benchmark_group("parse_once_vs_per_grid");

    group.bench_function("parse_per_grid", |b| {
        b.iter(|| {
            for _ in 0..3 {
                let parsed = ParsedFrame::parse(&data).unwrap();
                black_box(extract_qp_grid_from_parsed(&parsed, 0, BASE_QP).unwrap());
            }
        });
    });
    group.bench_function("parse_once_shared", |b| {
        b.iter(|| {
            let parsed = ParsedFrame::parse(&data).unwrap();
            for _ in 0..3 {
                black_box(extract_qp_grid_from_parsed(&parsed, 0, BASE_QP).unwrap());
            }
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_parse_frame,
    bench_extract_grids,
    bench_parse_once_vs_per_grid
);
criterion_main!(benches);
