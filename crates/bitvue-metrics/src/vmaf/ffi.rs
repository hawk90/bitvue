//! The `unsafe` boundary to libvmaf: owning wrappers for a picture and a scoring session.
//!
//! Nothing else in this crate touches `vmaf_head_sys`. The wrappers are not public API; `mod.rs`
//! builds the safe functions on top of them.

use std::ffi::{c_uint, CString};
use std::ptr;

use bitvue_engine::{BitvueError, Result};
use vmaf_head_sys as sys;

use super::{VmafConfig, VmafFrame};

// `VmafFrame` hands 10/12-bit samples to libvmaf as raw bytes, which it reads as native-endian
// `uint16_t`; the documented layout is little-endian.
const _: () = assert!(cfg!(target_endian = "little"));

/// Built-in model used when [`VmafConfig::model_path`] is `None`.
const DEFAULT_MODEL: &str = "vmaf_v0.6.1";

pub(super) fn invalid(message: impl Into<String>) -> BitvueError {
    BitvueError::InvalidData(message.into())
}

/// libvmaf returns 0 on success and a negative errno on failure.
pub(super) fn check(rc: i32, what: &str) -> Result<()> {
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
pub(super) struct Session {
    ctx: *mut sys::VmafContext,
    model: *mut sys::VmafModel,
}

impl Session {
    pub(super) fn new(config: &VmafConfig) -> Result<Self> {
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

    pub(super) fn push(
        &mut self,
        index: usize,
        reference: &VmafFrame,
        distorted: &VmafFrame,
    ) -> Result<()> {
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
    pub(super) fn flush(&mut self) -> Result<()> {
        // SAFETY: two NULL pictures mean "flush" (libvmaf.c: `if (!ref && !dist) return
        // flush_context(vmaf)`).
        let rc = unsafe { sys::vmaf_read_pictures(self.ctx, ptr::null_mut(), ptr::null_mut(), 0) };
        check(rc, "flushing libvmaf")
    }

    pub(super) fn score_at(&self, index: usize) -> Result<f64> {
        let index = u32::try_from(index).map_err(|_| invalid("frame index exceeds u32"))?;
        let mut score = f64::NAN;
        // SAFETY: valid context/model, `score` is a valid out pointer.
        let rc = unsafe { sys::vmaf_score_at_index(self.ctx, self.model, &mut score, index) };
        check(rc, &format!("vmaf_score_at_index (frame {index})"))?;
        Ok(score)
    }

    /// Arithmetic mean over frames `0..n_frames`.
    pub(super) fn pooled_mean(&self, n_frames: usize) -> Result<f64> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vmaf::test_support::frame;

    /// The score cannot check this: the default model only looks at luma, so a wrong chroma
    /// plane (U/V swapped, bad crop, wrong stride) would pass every score comparison. Read the
    /// planes back out of the libvmaf picture and compare them with the source, row by row.
    #[test]
    fn picture_planes_hold_exactly_the_source_samples() {
        for (w, h, depth) in [(176usize, 144usize, 8u8), (175, 143, 8), (176, 144, 10)] {
            let f = frame(w, h, depth, 3, 4);
            assert_ne!(f.u, f.v, "test frame must have distinct U and V planes");
            let pic = Picture::from_frame(&f).unwrap();
            let bps = if depth > 8 { 2 } else { 1 };

            for (i, src) in [&f.y, &f.u, &f.v].into_iter().enumerate() {
                let src_w = if i == 0 { w } else { w.div_ceil(2) };
                let (pw, ph) = (pic.raw.w[i] as usize, pic.raw.h[i] as usize);
                let stride = pic.raw.stride[i] as usize;
                let base = pic.raw.data[i] as *const u8;
                for row in 0..ph {
                    // SAFETY: libvmaf allocated `stride * ph` bytes for plane `i`.
                    let got =
                        unsafe { std::slice::from_raw_parts(base.add(row * stride), pw * bps) };
                    let start = row * src_w * bps;
                    assert_eq!(
                        got,
                        &src[start..start + pw * bps],
                        "plane {i} row {row} ({w}x{h}, {depth}-bit)"
                    );
                }
            }
        }
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
}
