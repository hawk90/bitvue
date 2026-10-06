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
//! Scores match upstream libvmaf: on real content they agree with FFmpeg's `libvmaf` filter to
//! well under 1e-3 across arm64 and x86-64. Pathological synthetic input can differ by up to
//! ~0.6 between AVX2 and AVX-512 machines (libvmaf picks SIMD kernels at runtime), so compare
//! with a tolerance, not for equality.

use std::ffi::{c_uint, CString};
use std::ptr;

use bitvue_engine::{BitvueError, Result};
use vmaf_head_sys as sys;

// `VmafFrame` hands 10/12-bit samples to libvmaf as raw bytes, which it reads as native-endian
// `uint16_t`; the documented layout is little-endian.
const _: () = assert!(cfg!(target_endian = "little"));

/// Built-in model used when [`VmafConfig::model_path`] is `None`.
const DEFAULT_MODEL: &str = "vmaf_v0.6.1";

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

fn invalid(message: impl Into<String>) -> BitvueError {
    BitvueError::InvalidData(message.into())
}

/// libvmaf returns 0 on success and a negative errno on failure.
fn check(rc: i32, what: &str) -> Result<()> {
    if rc == 0 {
        Ok(())
    } else {
        Err(invalid(format!("{what} failed (libvmaf error {rc})")))
    }
}

/// A `VmafPicture` filled from a [`VmafFrame`].
///
/// Ownership: `Drop` always calls `vmaf_picture_unref`, whatever happened to the picture.
/// That is correct in every case because `vmaf_picture_unref` (picture.c) decrements the
/// refcount and then `memset`s the struct to zero, and returns `-EINVAL` without touching
/// anything when `pic->ref` is NULL:
///
/// * `vmaf_read_pictures` succeeded: libvmaf has already unref'd our struct in place (libvmaf.c,
///   `err |= vmaf_picture_unref(ref)` / the threaded path), so it is zeroed and our unref is a
///   no-op. libvmaf keeps its own reference-counted copy for the temporal features.
/// * `vmaf_read_pictures` failed: libvmaf returns without unref'ing our pictures (early
///   `return err` paths), so our unref is the one that frees them.
struct Picture {
    raw: sys::VmafPicture,
}

impl Picture {
    fn from_frame(frame: &VmafFrame) -> Result<Self> {
        if !matches!(frame.bit_depth, 8 | 10 | 12) {
            return Err(invalid(format!(
                "unsupported bit depth {} (expected 8, 10 or 12)",
                frame.bit_depth
            )));
        }
        if frame.width == 0 || frame.height == 0 {
            return Err(invalid("frame has a zero dimension"));
        }
        let (width, height) = (
            u32::try_from(frame.width).map_err(|_| invalid("frame width exceeds u32"))?,
            u32::try_from(frame.height).map_err(|_| invalid("frame height exceeds u32"))?,
        );

        let mut raw = std::mem::MaybeUninit::<sys::VmafPicture>::uninit();
        // SAFETY: `vmaf_picture_alloc` fully initialises `*raw` when it returns 0.
        let rc = unsafe {
            sys::vmaf_picture_alloc(
                raw.as_mut_ptr(),
                sys::VmafPixelFormat_VMAF_PIX_FMT_YUV420P,
                c_uint::from(frame.bit_depth),
                width,
                height,
            )
        };
        check(rc, "vmaf_picture_alloc")?;
        // SAFETY: initialised by the successful call above.
        let mut picture = Picture {
            raw: unsafe { raw.assume_init() },
        };
        // From here on an early return drops `picture`, which releases the allocation.
        picture.fill(frame)?;
        Ok(picture)
    }

    fn fill(&mut self, frame: &VmafFrame) -> Result<()> {
        let bytes_per_sample = if frame.bit_depth > 8 { 2 } else { 1 };
        let planes: [(&str, &[u8]); 3] = [("Y", &frame.y), ("U", &frame.u), ("V", &frame.v)];

        for (index, (name, src)) in planes.into_iter().enumerate() {
            // Source geometry, as documented on `VmafFrame`: full-size luma, and chroma rounded
            // UP (what decoders produce for odd sizes).
            let (src_w, src_h) = if index == 0 {
                (frame.width, frame.height)
            } else {
                (frame.width.div_ceil(2), frame.height.div_ceil(2))
            };
            // libvmaf's own geometry. It rounds odd chroma DOWN (175x143 luma -> 87x71 chroma),
            // so it only reads the top-left part of an odd-sized source plane.
            let (plane_w, plane_h) = (self.raw.w[index] as usize, self.raw.h[index] as usize);
            if plane_w > src_w || plane_h > src_h {
                return Err(invalid("libvmaf plane is larger than the source plane"));
            }

            let src_row_bytes = src_w * bytes_per_sample;
            let expected = src_row_bytes
                .checked_mul(src_h)
                .ok_or_else(|| invalid("plane size overflows usize"))?;
            if src.len() != expected {
                return Err(invalid(format!(
                    "{name} plane has {} bytes, expected {expected} ({src_w}x{src_h} at \
                     {bytes_per_sample} byte(s)/sample)",
                    src.len()
                )));
            }

            let row_bytes = plane_w * bytes_per_sample;
            let stride = usize::try_from(self.raw.stride[index])
                .map_err(|_| invalid("negative plane stride from libvmaf"))?;
            let dst = self.raw.data[index].cast::<u8>();
            if dst.is_null() || stride < row_bytes {
                return Err(invalid("libvmaf returned an unusable plane layout"));
            }

            for row in 0..plane_h {
                // SAFETY: destination: libvmaf allocated `stride * plane_h` bytes for this plane
                // (vmaf_picture_alloc) with `stride >= row_bytes`, so row `row` lies inside it.
                // Source: `src.len() == src_row_bytes * src_h` (checked above) and
                // `row < plane_h <= src_h`, `row_bytes <= src_row_bytes`, so
                // `row * src_row_bytes + row_bytes <= src.len()`. The two buffers are separate
                // allocations, so the ranges cannot overlap.
                unsafe {
                    ptr::copy_nonoverlapping(
                        src.as_ptr().add(row * src_row_bytes),
                        dst.add(row * stride),
                        row_bytes,
                    );
                }
            }
        }
        Ok(())
    }
}

impl Drop for Picture {
    fn drop(&mut self) {
        // SAFETY: see the type-level docs; the call is a no-op on an already-consumed picture.
        // The return value (-EINVAL in that case) is intentionally ignored.
        unsafe {
            sys::vmaf_picture_unref(&mut self.raw);
        }
    }
}

/// A libvmaf context with one loaded model.
///
/// Not `Send`/`Sync` (it holds raw pointers); libvmaf parallelises internally.
struct Session {
    ctx: *mut sys::VmafContext,
    model: *mut sys::VmafModel,
}

impl Session {
    fn new(config: &VmafConfig) -> Result<Self> {
        let n_threads = config
            .n_threads
            .unwrap_or_else(|| {
                std::thread::available_parallelism()
                    .map(|n| n.get())
                    .unwrap_or(1)
            })
            .clamp(1, c_uint::MAX as usize) as c_uint;
        let log_level = match config.log_level {
            0 => sys::VmafLogLevel_VMAF_LOG_LEVEL_NONE,
            1 => sys::VmafLogLevel_VMAF_LOG_LEVEL_ERROR,
            2 => sys::VmafLogLevel_VMAF_LOG_LEVEL_WARNING,
            3 => sys::VmafLogLevel_VMAF_LOG_LEVEL_INFO,
            _ => sys::VmafLogLevel_VMAF_LOG_LEVEL_DEBUG,
        };
        let cfg = sys::VmafConfiguration {
            log_level,
            n_threads,
            n_subsample: 1,
            cpumask: 0,
            gpumask: 0,
        };

        let mut ctx = ptr::null_mut();
        // SAFETY: on success `vmaf_init` stores a valid context in `ctx`.
        check(unsafe { sys::vmaf_init(&mut ctx, cfg) }, "vmaf_init")?;
        // From here `Drop` closes the context even if loading the model fails.
        let mut session = Session {
            ctx,
            model: ptr::null_mut(),
        };
        session.load_model(config)?;
        Ok(session)
    }

    fn load_model(&mut self, config: &VmafConfig) -> Result<()> {
        let name = CString::new("bitvue").expect("literal has no NUL");
        let mut model_cfg = sys::VmafModelConfig {
            name: name.as_ptr(),
            flags: u64::from(sys::VmafModelFlags_VMAF_MODEL_FLAGS_DEFAULT),
        };
        let mut model = ptr::null_mut();

        match &config.model_path {
            Some(path) => {
                let path = CString::new(path.as_str())
                    .map_err(|_| invalid("VMAF model path contains a NUL byte"))?;
                // SAFETY: all pointers are valid for the duration of the call; `model` receives
                // an owned model on success.
                let rc = unsafe {
                    sys::vmaf_model_load_from_path(&mut model, &mut model_cfg, path.as_ptr())
                };
                check(rc, "loading the VMAF model file")?;
            }
            None => {
                let version = CString::new(DEFAULT_MODEL).expect("literal has no NUL");
                // SAFETY: as above.
                let rc =
                    unsafe { sys::vmaf_model_load(&mut model, &mut model_cfg, version.as_ptr()) };
                check(rc, "loading the built-in VMAF model")?;
            }
        }
        self.model = model;

        // SAFETY: `ctx` and `model` are valid and owned by `self`.
        check(
            unsafe { sys::vmaf_use_features_from_model(self.ctx, self.model) },
            "vmaf_use_features_from_model",
        )
    }

    fn push(&mut self, index: usize, reference: &VmafFrame, distorted: &VmafFrame) -> Result<()> {
        let index = u32::try_from(index).map_err(|_| invalid("frame index exceeds u32"))?;
        let mut reference = Picture::from_frame(reference)?;
        let mut distorted = Picture::from_frame(distorted)?;
        // SAFETY: both pictures are initialised; ownership is settled by `Picture::drop`.
        let rc = unsafe {
            sys::vmaf_read_pictures(self.ctx, &mut reference.raw, &mut distorted.raw, index)
        };
        check(rc, &format!("vmaf_read_pictures (frame {index})"))
    }

    /// Signals end of input; required before reading any score.
    fn flush(&mut self) -> Result<()> {
        // SAFETY: two NULL pictures mean "flush" (libvmaf.c: `if (!ref && !dist) return
        // flush_context(vmaf)`).
        let rc = unsafe { sys::vmaf_read_pictures(self.ctx, ptr::null_mut(), ptr::null_mut(), 0) };
        check(rc, "flushing libvmaf")
    }

    fn score_at(&self, index: usize) -> Result<f64> {
        let index = u32::try_from(index).map_err(|_| invalid("frame index exceeds u32"))?;
        let mut score = f64::NAN;
        // SAFETY: valid context/model, `score` is a valid out pointer.
        let rc = unsafe { sys::vmaf_score_at_index(self.ctx, self.model, &mut score, index) };
        check(rc, &format!("vmaf_score_at_index (frame {index})"))?;
        Ok(score)
    }

    /// Arithmetic mean over frames `0..n_frames`.
    fn pooled_mean(&self, n_frames: usize) -> Result<f64> {
        let last = u32::try_from(n_frames - 1).map_err(|_| invalid("frame count exceeds u32"))?;
        let mut score = f64::NAN;
        // SAFETY: as in `score_at`.
        let rc = unsafe {
            sys::vmaf_score_pooled(
                self.ctx,
                self.model,
                sys::VmafPoolingMethod_VMAF_POOL_METHOD_MEAN,
                &mut score,
                0,
                last,
            )
        };
        check(rc, "vmaf_score_pooled")?;
        Ok(score)
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        // Same order as libvmaf's own `vmaf` tool: model first, then the context.
        // SAFETY: both pointers are owned by `self` and not used afterwards; `vmaf_close` waits
        // for outstanding worker threads.
        unsafe {
            if !self.model.is_null() {
                sys::vmaf_model_destroy(self.model);
            }
            sys::vmaf_close(self.ctx);
        }
    }
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
/// let reference = vec![VmafFrame { /* ... */ }];
/// let distorted = vec![VmafFrame { /* ... */ }];
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

    /// Deterministic hash noise in `0..=amplitude`.
    fn noise(x: usize, y: usize, seed: usize, amplitude: u32) -> f64 {
        let h = (x as u32)
            .wrapping_mul(73856093)
            .wrapping_add((y as u32).wrapping_mul(19349663))
            .wrapping_add((seed as u32).wrapping_mul(83492791))
            .wrapping_mul(2654435761);
        f64::from((h >> 24) % (amplitude + 1))
    }

    /// Smooth, slowly moving content with a little fine texture, so VMAF has real structure to
    /// compare (patterns that wrap or alias make libvmaf's AVX2/AVX-512 kernels diverge by
    /// tenths of a point and saturate scores at 0). `distortion == 0` is pristine; otherwise
    /// the luma is 3x3 box-blurred and given hash noise whose amplitude is `distortion`.
    fn frame(
        width: usize,
        height: usize,
        bit_depth: u8,
        seed: usize,
        distortion: u32,
    ) -> VmafFrame {
        let bytes = if bit_depth > 8 { 2 } else { 1 };
        let max = f64::from((1u32 << bit_depth) - 1);
        let pristine = |x: usize, y: usize| -> f64 {
            let (fx, fy, t) = (x as f64, y as f64, seed as f64);
            128.0
                + 55.0 * (fx * 0.045 + t * 0.15).sin() * (fy * 0.05).cos()
                + 25.0 * (fx * 0.011 - fy * 0.013).sin()
                + noise(x, y, 0, 14)
                - 7.0
        };
        let luma = |x: usize, y: usize| -> f64 {
            if distortion == 0 {
                return pristine(x, y);
            }
            let (mut sum, mut count) = (0.0, 0.0);
            for dy in -1i64..=1 {
                for dx in -1i64..=1 {
                    let (nx, ny) = (x as i64 + dx, y as i64 + dy);
                    if nx >= 0 && ny >= 0 && (nx as usize) < width && (ny as usize) < height {
                        sum += pristine(nx as usize, ny as usize);
                        count += 1.0;
                    }
                }
            }
            sum / count + noise(x, y, seed + 1, distortion) - f64::from(distortion) / 2.0
        };
        let chroma = |x: usize, y: usize| -> f64 { 128.0 + 20.0 * ((x + y) as f64 * 0.03).sin() };

        let encode = |plane: &mut Vec<u8>, value: f64| {
            let v = ((value.clamp(0.0, 255.0) / 255.0) * max).round() as u32;
            plane.extend_from_slice(&v.to_le_bytes()[..bytes]);
        };
        let mut y_plane = Vec::with_capacity(width * height * bytes);
        for y in 0..height {
            for x in 0..width {
                encode(&mut y_plane, luma(x, y));
            }
        }
        let (cw, ch) = (width.div_ceil(2), height.div_ceil(2));
        let (mut u_plane, mut v_plane) = (Vec::new(), Vec::new());
        for y in 0..ch {
            for x in 0..cw {
                encode(&mut u_plane, chroma(x, y));
                encode(&mut v_plane, 255.0 - chroma(x, y));
            }
        }
        VmafFrame {
            y: y_plane,
            u: u_plane,
            v: v_plane,
            width,
            height,
            bit_depth,
        }
    }

    fn sequence(n: usize, bit_depth: u8, distortion: u32) -> Vec<VmafFrame> {
        (0..n)
            .map(|i| frame(176, 144, bit_depth, i, distortion))
            .collect()
    }

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

    /// The path that matters most for the ownership rules: libvmaf itself rejects a submission
    /// (`vmaf_read_pictures` returns an error without consuming the pictures), so `Picture::drop`
    /// must free them. Two kinds of rejection: a geometry change after the first frame
    /// (`validate_pic_params`) and a submission after the flush (`vmaf->flushed`). A missing
    /// unref leaks (caught by `leaks` / valgrind); a double unref crashes.
    #[test]
    fn libvmaf_rejecting_a_submission_releases_the_pictures() {
        let f = frame(176, 144, 8, 0, 0);
        let other = frame(160, 128, 8, 0, 0);
        for _ in 0..40 {
            let mut session = Session::new(&VmafConfig::default()).unwrap();
            session.push(0, &f, &f).unwrap();
            assert!(session.push(1, &other, &other).is_err(), "geometry change");
            session.flush().unwrap();
            assert!(session.push(2, &f, &f).is_err(), "push after flush");
        }
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
