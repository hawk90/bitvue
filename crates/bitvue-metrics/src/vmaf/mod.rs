//! VMAF (Video Multimethod Assessment Fusion) via Netflix's libvmaf.
//!
//! Enabled by the `vmaf` feature. libvmaf is compiled from source and statically linked by the
//! `vmaf-head-sys` crate, so neither a system libvmaf nor FFmpeg is needed; the build needs
//! meson, ninja and a C/C++ compiler (plus nasm on x86). CPU only (`vmaf-head-sys` builds
//! without CUDA).
//!
//! The public API is entirely safe. All FFI lives in this file behind three small RAII types
//! ([`Picture`], [`Session`]) whose ownership rules are taken from the vendored libvmaf sources
//! (`libvmaf/src/libvmaf.c`, `picture.c`), cited where they matter.
//!
//! Scores match upstream libvmaf: they are identical to FFmpeg's `libvmaf` filter on arm64, and
//! within 4e-4 (a 352x288 clip) to 3e-2 (a small, noisy 176x144 clip) between arm64 and x86-64.
//! libvmaf picks SIMD kernels at runtime (NEON / AVX2 / AVX-512), and pathological synthetic
//! input can differ by ~0.6 between AVX2 and AVX-512 machines, so compare with a tolerance, not
//! for equality.

mod ffi;
#[cfg(test)]
mod test_support;

use bitvue_engine::Result;

use ffi::{invalid, Session};

/// VMAF configuration options
pub struct VmafConfig {
    /// Path to a libvmaf model file (JSON). `None` = the built-in `vmaf_v0.6.1` model.
    pub model_path: Option<String>,
    /// Number of worker threads (`None` = all available cores)
    pub n_threads: Option<usize>,
    /// Log level (0 = none, 1 = error, 2 = warning, 3 = info, 4 = debug)
    pub log_level: u8,
}

impl Default for VmafConfig {
    fn default() -> Self {
        Self {
            model_path: None,
            n_threads: None,
            log_level: 1, // Error only by default
        }
    }
}

/// YUV 4:2:0 frame data for VMAF computation
///
/// Planes are tightly packed (no row padding). For `bit_depth` 8 each sample is one byte; for 10
/// and 12 each sample is a 16-bit little-endian value (two bytes). Chroma planes are
/// `ceil(width / 2) x ceil(height / 2)`, the layout decoders produce for odd sizes (libvmaf
/// itself scores the `floor` region and ignores the extra chroma row/column). Plane lengths are
/// validated exactly; a wrong length is an error, never an out-of-bounds read.
pub struct VmafFrame {
    /// Y plane data
    pub y: Vec<u8>,
    /// U plane data
    pub u: Vec<u8>,
    /// V plane data
    pub v: Vec<u8>,
    /// Luma width
    pub width: usize,
    /// Luma height
    pub height: usize,
    /// Bit depth (8, 10, or 12)
    pub bit_depth: u8,
}

/// Validates the inputs and runs every frame pair through a fresh [`Session`].
fn run_session(
    reference_frames: &[VmafFrame],
    distorted_frames: &[VmafFrame],
    width: usize,
    height: usize,
    config: Option<VmafConfig>,
) -> Result<Session> {
    if reference_frames.len() != distorted_frames.len() {
        return Err(invalid(format!(
            "Frame count mismatch: {} reference vs {} distorted",
            reference_frames.len(),
            distorted_frames.len()
        )));
    }
    if reference_frames.is_empty() {
        return Err(invalid("Cannot compute VMAF on empty frame sequence"));
    }

    let mut session = Session::new(&config.unwrap_or_default())?;
    for (i, (reference, distorted)) in reference_frames.iter().zip(distorted_frames).enumerate() {
        if reference.width != width
            || reference.height != height
            || distorted.width != width
            || distorted.height != height
        {
            return Err(invalid(format!(
                "Frame {} dimension mismatch: expected {}x{}, got {}x{} (ref) and {}x{} (dist)",
                i,
                width,
                height,
                reference.width,
                reference.height,
                distorted.width,
                distorted.height
            )));
        }
        session.push(i, reference, distorted)?;
    }
    session.flush()?;
    Ok(session)
}

/// Compute VMAF score for a pair of video sequences
///
/// # Arguments
///
/// * `reference_frames` - Original/reference video frames
/// * `distorted_frames` - Compressed/distorted video frames
/// * `width` - Video width in pixels
/// * `height` - Video height in pixels
/// * `config` - VMAF configuration (None = use defaults)
///
/// # Returns
///
/// The arithmetic mean of the per-frame VMAF scores (0-100, higher is better)
/// - 0-20: Poor quality
/// - 20-40: Fair quality
/// - 40-60: Good quality
/// - 60-80: Very good quality
/// - 80-100: Excellent quality
///
/// # Example
///
/// ```no_run
/// use bitvue_metrics::vmaf::{compute_vmaf, VmafFrame};
///
/// // Decode two clips into 4:2:0 frames (tightly packed Y, U, V planes) however you like.
/// fn load(_path: &str) -> Vec<VmafFrame> {
///     unimplemented!("decode the clip")
/// }
///
/// let reference = load("reference.ivf");
/// let distorted = load("distorted.ivf");
///
/// let score = compute_vmaf(&reference, &distorted, 1920, 1080, None).unwrap();
/// println!("VMAF Score: {:.2}", score);
/// ```
pub fn compute_vmaf(
    reference_frames: &[VmafFrame],
    distorted_frames: &[VmafFrame],
    width: usize,
    height: usize,
    config: Option<VmafConfig>,
) -> Result<f64> {
    let session = run_session(reference_frames, distorted_frames, width, height, config)?;
    session.pooled_mean(reference_frames.len())
}

/// Compute per-frame VMAF scores
///
/// Returns one score per frame pair. The temporal features look at the *next* frame, so the
/// last frame's score depends on it being the last (this is libvmaf's behaviour, not an
/// artefact of this wrapper).
pub fn compute_vmaf_per_frame(
    reference_frames: &[VmafFrame],
    distorted_frames: &[VmafFrame],
    width: usize,
    height: usize,
    config: Option<VmafConfig>,
) -> Result<Vec<f64>> {
    let session = run_session(reference_frames, distorted_frames, width, height, config)?;
    (0..reference_frames.len())
        .map(|i| session.score_at(i))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_support::{frame, sequence};

    #[test]
    fn identical_sequences_score_near_100() {
        let r = sequence(4, 8, 0);
        let score = compute_vmaf(&r, &r, 176, 144, None).unwrap();
        assert!(score > 97.0, "identical pair scored {score}");
    }

    #[test]
    fn degraded_sequence_scores_lower() {
        let r = sequence(4, 8, 0);
        let d = sequence(4, 8, 6);
        let same = compute_vmaf(&r, &r, 176, 144, None).unwrap();
        let diff = compute_vmaf(&r, &d, 176, 144, None).unwrap();
        assert!(diff < same - 5.0, "degraded {diff} vs identical {same}");
        assert!(diff > 1.0, "degraded pair saturated at the floor: {diff}");
    }

    /// Regression test for the old fake per-frame implementation, which returned the overall
    /// score repeated once per frame. Distortion grows with the frame index, so a real
    /// per-frame score must fall.
    #[test]
    fn per_frame_scores_are_real_not_a_repeated_overall_score() {
        let n = 6;
        let r = sequence(n, 8, 0);
        let d: Vec<VmafFrame> = (0..n)
            .map(|i| frame(176, 144, 8, i, 2 + 6 * i as u32))
            .collect();

        let per = compute_vmaf_per_frame(&r, &d, 176, 144, None).unwrap();
        assert_eq!(per.len(), n);
        assert!(per.iter().all(|s| s.is_finite() && *s > 1.0), "{per:?}");
        // First vs the frame before the last (the last frame's temporal terms differ).
        assert!(
            per[0] > per[n - 2] + 3.0,
            "scores do not fall as the distortion grows: {per:?}"
        );

        let pooled = compute_vmaf(&r, &d, 176, 144, None).unwrap();
        let mean = per.iter().sum::<f64>() / per.len() as f64;
        assert!(
            (pooled - mean).abs() < 1e-6,
            "pooled {pooled} != mean {mean}"
        );
    }

    #[test]
    fn ten_bit_input_is_accepted() {
        let r = sequence(3, 10, 0);
        let score = compute_vmaf(&r, &r, 176, 144, None).unwrap();
        assert!(score > 90.0, "10-bit identical pair scored {score}");
    }

    #[test]
    fn odd_dimensions_take_ceil_chroma_planes() {
        // A decoder's 4:2:0 output for 175x143 has 88x72 chroma. libvmaf only uses the 87x71
        // floor region; the wrapper must accept the ceil-sized plane and crop it.
        let r: Vec<_> = (0..3).map(|i| frame(175, 143, 8, i, 0)).collect();
        assert_eq!(r[0].u.len(), 88 * 72);
        let score = compute_vmaf(&r, &r, 175, 143, None).unwrap();
        assert!(score.is_finite() && score > 90.0, "scored {score}");

        // A floor-sized chroma plane is not the documented layout and is rejected.
        let mut floor_sized: Vec<_> = (0..3).map(|i| frame(175, 143, 8, i, 0)).collect();
        for f in &mut floor_sized {
            f.u.truncate(87 * 71);
            f.v.truncate(87 * 71);
        }
        assert!(compute_vmaf(&floor_sized, &floor_sized, 175, 143, None).is_err());
    }

    #[test]
    fn n_threads_does_not_change_the_score() {
        let r = sequence(4, 8, 0);
        let d = sequence(4, 8, 6);
        let cfg = |n| VmafConfig {
            n_threads: Some(n),
            ..Default::default()
        };
        let one = compute_vmaf(&r, &d, 176, 144, Some(cfg(1))).unwrap();
        let four = compute_vmaf(&r, &d, 176, 144, Some(cfg(4))).unwrap();
        assert!(
            (one - four).abs() < 1e-9,
            "1 thread {one} vs 4 threads {four}"
        );
    }

    #[test]
    fn rejects_empty_and_mismatched_input() {
        let r = sequence(2, 8, 0);
        assert!(compute_vmaf(&[], &[], 176, 144, None).is_err());
        assert!(compute_vmaf(&r, &r[..1], 176, 144, None).is_err());
        // wrong declared dimensions
        assert!(compute_vmaf(&r, &r, 160, 144, None).is_err());
    }

    #[test]
    fn rejects_malformed_planes_instead_of_reading_out_of_bounds() {
        let good = sequence(2, 8, 0);

        let mut short_chroma = sequence(2, 8, 0);
        short_chroma[1].u.pop();
        assert!(compute_vmaf(&good, &short_chroma, 176, 144, None).is_err());

        let mut long_luma = sequence(2, 8, 0);
        long_luma[0].y.push(0);
        assert!(compute_vmaf(&long_luma, &good, 176, 144, None).is_err());

        // 10-bit declared but 8-bit sized planes
        let mut wrong_depth = sequence(2, 8, 0);
        wrong_depth[0].bit_depth = 10;
        assert!(compute_vmaf(&wrong_depth, &good, 176, 144, None).is_err());

        let mut bad_depth = sequence(2, 8, 0);
        bad_depth[0].bit_depth = 9;
        assert!(compute_vmaf(&bad_depth, &good, 176, 144, None).is_err());
    }

    #[test]
    fn bad_model_path_is_an_error() {
        let r = sequence(2, 8, 0);
        let cfg = VmafConfig {
            model_path: Some("/definitely/not/a/model.json".to_string()),
            ..Default::default()
        };
        assert!(compute_vmaf(&r, &r, 176, 144, Some(cfg)).is_err());
        let cfg = VmafConfig {
            model_path: Some("with\0nul".to_string()),
            ..Default::default()
        };
        assert!(compute_vmaf(&r, &r, 176, 144, Some(cfg)).is_err());
    }

    /// Exercises the ownership paths repeatedly: success (libvmaf consumes the pictures), a
    /// plane-validation failure on a later frame (earlier pictures were already consumed, the
    /// failing one was never submitted). A double free or a leak-induced crash would show up
    /// here (run with MallocScribble=1 on macOS, or under valgrind/ASan, for a stricter check).
    #[test]
    fn repeated_success_and_error_paths_do_not_crash() {
        let good = sequence(3, 8, 0);
        let mut bad_mid = sequence(3, 8, 0);
        bad_mid[2].v.pop();
        for _ in 0..40 {
            assert!(compute_vmaf(&good, &good, 176, 144, None).is_ok());
            assert!(compute_vmaf(&good, &bad_mid, 176, 144, None).is_err());
        }
    }
}
