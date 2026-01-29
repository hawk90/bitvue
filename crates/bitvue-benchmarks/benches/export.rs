//! Benchmarks for export operations
//!
//! Export functions can be slow when dealing with large datasets (100K+ frames).

use bitvue_core::export::{
    ExportConfig, ExportFormat, FrameExportRow, QualityMetrics, MetricPoint,
};
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};

fn create_test_frames(count: usize) -> Vec<FrameExportRow> {
    (0..count)
        .map(|i| FrameExportRow {
            display_idx: i,
            frame_type: if i % 3 == 0 { "KEY" } else { "P" }.to_string(),
            size_bytes: 1000 + (i % 10) * 100,
            pts: Some(i as u64 * 100),
            dts: Some(i as u64 * 100 - 100),
            is_key: i % 3 == 0,
            has_error: false,
        })
        .collect()
}

/// Benchmark CSV export with varying frame counts
fn bench_export_frames_csv(c: &mut Criterion) {
    let mut group = c.benchmark_group("export_frames_csv");

    for frame_count in [100, 1000, 10000, 100000].iter() {
        group.throughput(Throughput::Elements(*frame_count as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(frame_count),
            frame_count,
            |b, &count| {
                let frames = create_test_frames(count);
                b.iter(|| {
                    let mut output = Vec::new();
                    black_box(
                        bitvue_core::export::export_frames_csv(&frames, &mut output, ExportConfig::default())
                            .unwrap(),
                    );
                });
            },
        );
    }

    group.finish();
}

/// Benchmark JSON export with varying frame counts
fn bench_export_frames_json(c: &mut Criterion) {
    let mut group = c.benchmark_group("export_frames_json");

    for frame_count in [100, 1000, 10000].iter() {
        group.throughput(Throughput::Elements(*frame_count as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(frame_count),
            frame_count,
            |b, &count| {
                let frames = create_test_frames(count);
                b.iter(|| {
                    let mut output = Vec::new();
                    black_box(
                        bitvue_core::export::export_frames_json(&frames, &mut output, ExportConfig::default())
                            .unwrap(),
                    );
                });
            },
        );
    }

    group.finish();
}

/// Benchmark JSON pretty-print export (slower than compact)
fn bench_export_frames_json_pretty(c: &mut Criterion) {
    let mut group = c.benchmark_group("export_frames_json_pretty");

    for frame_count in [100, 1000, 5000].iter() {
        group.throughput(Throughput::Elements(*frame_count as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(frame_count),
            frame_count,
            |b, &count| {
                let frames = create_test_frames(count);
                let config = ExportConfig {
                    range: None,
                    pretty: true,
                };
                b.iter(|| {
                    let mut output = Vec::new();
                    black_box(
                        bitvue_core::export::export_frames_json(&frames, &mut output, config).unwrap(),
                    );
                });
            },
        );
    }

    group.finish();
}

fn create_test_metrics(count: usize) -> (Vec<MetricPoint>, Vec<MetricPoint>, Vec<MetricPoint>) {
    let psnr: Vec<MetricPoint> = (0..count)
        .map(|i| MetricPoint {
            idx: i,
            value: 30.0 + (i as f32 % 20.0),
        })
        .collect();

    let ssim: Vec<MetricPoint> = (0..count)
        .map(|i| MetricPoint {
            idx: i,
            value: 0.8 + (i as f32 % 20.0) / 100.0,
        })
        .collect();

    let vmaf: Vec<MetricPoint> = (0..count)
        .map(|i| MetricPoint {
            idx: i,
            value: 70.0 + (i as f32 % 30.0),
        })
        .collect();

    (psnr, ssim, vmaf)
}

/// Benchmark metrics CSV export
fn bench_export_metrics_csv(c: &mut Criterion) {
    let mut group = c.benchmark_group("export_metrics_csv");

    for metric_count in [100, 1000, 10000].iter() {
        group.throughput(Throughput::Elements(*metric_count as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(metric_count),
            metric_count,
            |b, &count| {
                let (psnr, ssim, vmaf) = create_test_metrics(count);
                let metrics = QualityMetrics {
                    psnr_y: &psnr,
                    ssim_y: &ssim,
                    vmaf: &vmaf,
                };
                b.iter(|| {
                    let mut output = Vec::new();
                    black_box(bitvue_core::export::export_metrics_csv(metrics, &mut output).unwrap());
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_export_frames_csv,
    bench_export_frames_json,
    bench_export_frames_json_pretty,
    bench_export_metrics_csv
);

criterion_main!(benches);
