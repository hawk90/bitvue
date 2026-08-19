//! Benchmarks for frame parsing and PSNR calculation

use bitvue_av1_codec::{parse_ivf_frames, ObuIterator};
use bitvue_metrics::psnr;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};

/// Benchmark parsing IVF header (just the header, no frame data)
fn bench_ivf_header_parse(c: &mut Criterion) {
    use bitvue_av1_codec::parse_ivf_header;

    // Minimal valid IVF header (32 bytes):
    // DKIF signature + version + header_size + fourcc + width + height + timebase_num/den + frame_count + unused
    let ivf_header: Vec<u8> = {
        let mut h = Vec::with_capacity(32);
        h.extend_from_slice(b"DKIF"); // signature
        h.extend_from_slice(&0u16.to_le_bytes()); // version
        h.extend_from_slice(&32u16.to_le_bytes()); // header size
        h.extend_from_slice(b"AV01"); // fourcc
        h.extend_from_slice(&1920u16.to_le_bytes()); // width
        h.extend_from_slice(&1080u16.to_le_bytes()); // height
        h.extend_from_slice(&30u32.to_le_bytes()); // timebase_num
        h.extend_from_slice(&1u32.to_le_bytes()); // timebase_den
        h.extend_from_slice(&100u32.to_le_bytes()); // frame_count
        h.extend_from_slice(&0u32.to_le_bytes()); // unused
        h
    };

    c.bench_function("ivf_header_parse", |b| {
        b.iter(|| black_box(parse_ivf_header(black_box(&ivf_header))));
    });
}

/// Benchmark PSNR calculation on synthetic frames
fn bench_psnr_calculation(c: &mut Criterion) {
    let mut group = c.benchmark_group("psnr");

    for &(w, h, label) in &[
        (640u32, 360u32, "360p"),
        (1280u32, 720u32, "720p"),
        (1920u32, 1080u32, "1080p"),
    ] {
        let pixels = (w * h) as usize;
        let ref_frame: Vec<u8> = (0..pixels).map(|i| (i % 255) as u8).collect();
        let dist_frame: Vec<u8> = (0..pixels).map(|i| ((i + 10) % 255) as u8).collect();

        group.throughput(Throughput::Elements(pixels as u64));
        group.bench_with_input(BenchmarkId::new("luma", label), &(w, h), |b, &(w, h)| {
            b.iter(|| {
                black_box(psnr(
                    black_box(&ref_frame),
                    black_box(&dist_frame),
                    w as usize,
                    h as usize,
                ))
            });
        });
    }
    group.finish();
}

/// Benchmark OBU iterator over a minimal synthetic OBU payload
fn bench_av1_obu_iterator(c: &mut Criterion) {
    // Minimal sequence header OBU bytes (type=1, no extension, no size field).
    // Real sequence headers vary; use a small synthetic blob that the iterator
    // will attempt to parse (it may return an error, but we benchmark iteration).
    let obu_data: Vec<u8> = vec![
        0x0A, // OBU header: forbidden=0, type=1 (sequence_header), ext=0, has_size=1, reserved=0
        0x08, // leb128 size = 8 bytes
        0x00, 0x00, 0x04, 0x45, 0x9E, 0x3A, 0x00, 0x00, // 8-byte payload
    ];

    let mut group = c.benchmark_group("av1_obu");
    group.bench_function("iterate_empty", |b| {
        b.iter(|| {
            let iter = ObuIterator::new(black_box(&obu_data));
            for obu in iter {
                black_box(obu.ok());
            }
        });
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_ivf_header_parse,
    bench_psnr_calculation,
    bench_av1_obu_iterator
);
criterion_main!(benches);
