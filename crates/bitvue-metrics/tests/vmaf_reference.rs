//! Checks bitvue's VMAF against the scores Netflix's libvmaf produced for the same input.
//!
//! The expected numbers come from FFmpeg's `libvmaf` filter (libvmaf 3.2.0, model
//! `vmaf_v0.6.1`); see `tests/fixtures/vmaf/README.md` for how they were generated. libvmaf picks
//! SIMD kernels at runtime, and across arm64 / AVX2 / AVX-512 machines real content differed by
//! at most 4.2e-4 (verified on macOS, Ubuntu 24.04/26.04 and Windows CI runners), so the
//! comparison uses a tolerance of 0.01, not equality.

#![cfg(feature = "vmaf")]

use bitvue_metrics::vmaf::{compute_vmaf, compute_vmaf_per_frame, VmafFrame};

const WIDTH: usize = 176;
const HEIGHT: usize = 144;
const FRAMES: usize = 5;
const TOLERANCE: f64 = 0.01;

/// libvmaf 3.2.0 via FFmpeg, per frame. The last frame's value depends on it being last (the
/// motion feature looks one frame ahead), so it is only comparable for exactly this input.
const EXPECTED_PER_FRAME: [f64; FRAMES] = [50.728519, 54.357944, 53.641153, 53.486662, 53.609525];
const EXPECTED_POOLED: f64 = 53.164760;

fn load(name: &str) -> Vec<VmafFrame> {
    let path = format!("{}/tests/fixtures/vmaf/{name}", env!("CARGO_MANIFEST_DIR"));
    let data = std::fs::read(&path).unwrap_or_else(|e| panic!("cannot read {path}: {e}"));
    let (luma, chroma) = (WIDTH * HEIGHT, (WIDTH / 2) * (HEIGHT / 2));
    let frame_len = luma + 2 * chroma;
    assert_eq!(
        data.len(),
        frame_len * FRAMES,
        "{name} has an unexpected size"
    );
    data.chunks_exact(frame_len)
        .map(|f| VmafFrame {
            y: f[..luma].to_vec(),
            u: f[luma..luma + chroma].to_vec(),
            v: f[luma + chroma..].to_vec(),
            width: WIDTH,
            height: HEIGHT,
            bit_depth: 8,
        })
        .collect()
}

#[test]
fn per_frame_scores_match_libvmaf() {
    let reference = load("ref_176x144_5f.yuv");
    let distorted = load("dist_176x144_5f.yuv");
    let scores = compute_vmaf_per_frame(&reference, &distorted, WIDTH, HEIGHT, None).unwrap();
    assert_eq!(scores.len(), FRAMES);
    for (i, (got, want)) in scores.iter().zip(EXPECTED_PER_FRAME).enumerate() {
        assert!(
            (got - want).abs() < TOLERANCE,
            "frame {i}: got {got:.6}, libvmaf says {want:.6} (all: {scores:?})"
        );
    }
}

#[test]
fn pooled_score_matches_libvmaf() {
    let reference = load("ref_176x144_5f.yuv");
    let distorted = load("dist_176x144_5f.yuv");
    let score = compute_vmaf(&reference, &distorted, WIDTH, HEIGHT, None).unwrap();
    assert!(
        (score - EXPECTED_POOLED).abs() < TOLERANCE,
        "got {score:.6}, libvmaf says {EXPECTED_POOLED:.6}"
    );
}
