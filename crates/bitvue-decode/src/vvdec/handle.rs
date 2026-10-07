//! Safe RAII layer over the vvdec C API.
//!
//! This file and `ffi.rs` are the only places that touch raw pointers. Everything above it works
//! with owned Rust values: [`Decoder`] owns the vvdec handles, [`Frame`] is a borrowed view of an
//! output picture that is released on drop, and failures are classified by [`ErrorKind`].
//!
//! The call contract below was established by probing vvdec 3.2.0 directly on a real 9-frame
//! stream (output checked byte-for-byte against `vvdecapp` and FFmpeg's VVC decoder):
//!
//! * `vvdec_decode` consumes the whole payload. It may hold the pictures back and answers
//!   `VVDEC_TRY_AGAIN`; on a short stream nothing is output until the flush. The payload may hold
//!   several access units (the whole bitstream in one call works).
//! * `vvdec_flush` returns one held-back picture per call and `VVDEC_EOF` once drained. The
//!   decoder stays usable afterwards (a second stream decodes correctly).
//! * `VVDEC_ERR_DEC_INPUT` (garbage, empty payload) leaves the decoder usable.
//!   `VVDEC_ERR_RESTART_REQUIRED` (seen after a truncated access unit) means it must be recreated.

use std::ffi::{c_int, CStr};
use std::marker::PhantomData;
use std::ptr::{self, NonNull};

use super::convert::{Picture, PictureKind, PlaneView};
use super::ffi;
use crate::decoder::ChromaFormat;

/// How the caller must react to a failed vvdec call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ErrorKind {
    /// The input was bad; the decoder is still usable.
    Input,
    /// The decoder is in a state it cannot continue from; recreate it.
    RestartRequired,
    /// Anything else (allocation, unsupported CPU, bad parameters, ...).
    Fatal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct VvdecError {
    pub code: i32,
    pub kind: ErrorKind,
    pub message: String,
}

impl std::fmt::Display for VvdecError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} (vvdec error {})", self.message, self.code)
    }
}

impl VvdecError {
    fn from_code(code: c_int) -> Self {
        let kind = match ffi::vvdecErrorCodes(code) {
            ffi::vvdecErrorCodes::VVDEC_ERR_DEC_INPUT => ErrorKind::Input,
            ffi::vvdecErrorCodes::VVDEC_ERR_RESTART_REQUIRED => ErrorKind::RestartRequired,
            _ => ErrorKind::Fatal,
        };
        Self {
            code,
            kind,
            message: error_message(code),
        }
    }

    fn init(message: &str) -> Self {
        Self {
            code: ffi::vvdecErrorCodes::VVDEC_ERR_INITIALIZE.0,
            kind: ErrorKind::Fatal,
            message: message.to_string(),
        }
    }
}

fn error_message(code: c_int) -> String {
    // SAFETY: `vvdec_get_error_msg` returns a pointer to a static NUL-terminated string (or NULL).
    unsafe {
        let msg = ffi::vvdec_get_error_msg(code);
        if msg.is_null() {
            format!("unknown vvdec error {code}")
        } else {
            CStr::from_ptr(msg).to_string_lossy().into_owned()
        }
    }
}

/// `"3.2.0"`-style version of the loaded library.
pub(super) fn version() -> String {
    // SAFETY: returns a static NUL-terminated string.
    unsafe {
        let v = ffi::vvdec_get_version();
        if v.is_null() {
            String::new()
        } else {
            CStr::from_ptr(v).to_string_lossy().into_owned()
        }
    }
}

/// Frees the payload of `au` **and clears the fields that point at it**.
///
/// `vvdec_accessUnit_free_payload` only frees: it leaves `payload` dangling. vvdec then frees that
/// same pointer again, either in the next `vvdec_accessUnit_alloc_payload` (which releases an
/// existing payload first) or in `vvdec_accessUnit_free` ("the payload memory is also released if
/// not done yet"), which aborts the process with a double free. This is not documented in
/// `vvdec.h`; it was found by crashing every decode test.
///
/// # Safety
///
/// `au` must be a valid access unit whose payload, if any, came from
/// `vvdec_accessUnit_alloc_payload`.
unsafe fn release_payload(au: *mut ffi::vvdecAccessUnit) {
    ffi::vvdec_accessUnit_free_payload(au);
    (*au).payload = ptr::null_mut();
    (*au).payloadSize = 0;
    (*au).payloadUsedSize = 0;
}

/// One decoder instance: the vvdec decoder handle plus the access unit used to feed it.
///
/// Both are owned exclusively by this value and freed in `Drop`. vvdec handles have no thread
/// affinity but must not be used concurrently; `Decoder` takes `&mut self` for every call, and
/// the worker thread (see `worker.rs`) is the only thing that ever holds one, so it is neither
/// `Sync` nor shared.
pub(super) struct Decoder {
    decoder: NonNull<ffi::vvdecDecoder>,
    access_unit: NonNull<ffi::vvdecAccessUnit>,
}

impl Decoder {
    /// Opens a decoder. `threads`: `None` lets vvdec pick (its default is `-1`).
    pub(super) fn open(threads: Option<i32>) -> Result<Self, VvdecError> {
        // SAFETY: `vvdecParams` is plain data; `vvdec_params_default` fills the whole struct
        // (its size comes from the real header via bindgen, so this cannot overrun it).
        let mut params: ffi::vvdecParams = unsafe { std::mem::zeroed() };
        unsafe { ffi::vvdec_params_default(&mut params) };
        params.logLevel = ffi::vvdecLogLevel::VVDEC_SILENT;
        if let Some(threads) = threads {
            params.threads = threads;
        }

        // SAFETY: `params` is valid for the call; the decoder copies what it needs.
        let decoder = NonNull::new(unsafe { ffi::vvdec_decoder_open(&mut params) })
            .ok_or_else(|| VvdecError::init("vvdec_decoder_open returned NULL"))?;

        // SAFETY: plain allocation. If it fails the decoder is closed again before returning.
        let access_unit = match NonNull::new(unsafe { ffi::vvdec_accessUnit_alloc() }) {
            Some(au) => au,
            None => {
                unsafe { ffi::vvdec_decoder_close(decoder.as_ptr()) };
                return Err(VvdecError::init("vvdec_accessUnit_alloc returned NULL"));
            }
        };
        Ok(Self {
            decoder,
            access_unit,
        })
    }

    /// Feeds `data` (Annex B, one or more access units) to the decoder.
    ///
    /// `Ok(None)` is `VVDEC_TRY_AGAIN`: the data was consumed but no picture is ready yet.
    pub(super) fn decode(
        &mut self,
        data: &[u8],
        cts: Option<i64>,
    ) -> Result<Option<Frame<'_>>, VvdecError> {
        let size = c_int::try_from(data.len())
            .map_err(|_| VvdecError::init("payload larger than i32::MAX bytes"))?;
        if size == 0 {
            // vvdec answers an empty payload with an input error; do not even ask.
            return Ok(None);
        }

        let au = self.access_unit.as_ptr();
        // SAFETY: `au` is a valid access unit owned by `self`. `vvdec_accessUnit_alloc_payload`
        // (returns `void` in the real header) allocates `size` bytes into `(*au).payload`, which
        // we fill completely before the payload is used; it is freed again right after the call.
        unsafe {
            ffi::vvdec_accessUnit_alloc_payload(au, size);
            if (*au).payload.is_null() {
                return Err(VvdecError::from_code(
                    ffi::vvdecErrorCodes::VVDEC_ERR_ALLOCATE.0,
                ));
            }
            ptr::copy_nonoverlapping(data.as_ptr(), (*au).payload, data.len());
            (*au).payloadUsedSize = size;
            (*au).cts = cts.unwrap_or(0) as u64;
            (*au).ctsValid = cts.is_some();
            (*au).dtsValid = false;
        }

        let mut frame: *mut ffi::vvdecFrame = ptr::null_mut();
        // SAFETY: valid decoder and access unit; `frame` is an out pointer.
        let rc = unsafe { ffi::vvdec_decode(self.decoder.as_ptr(), au, &mut frame) };
        // SAFETY: the payload was allocated above and is no longer needed.
        unsafe { release_payload(au) };

        self.interpret(rc, frame)
    }

    /// Drains one held-back picture. `Ok(None)` is `VVDEC_EOF`: nothing left.
    pub(super) fn flush(&mut self) -> Result<Option<Frame<'_>>, VvdecError> {
        let mut frame: *mut ffi::vvdecFrame = ptr::null_mut();
        // SAFETY: valid decoder; `frame` is an out pointer.
        let rc = unsafe { ffi::vvdec_flush(self.decoder.as_ptr(), &mut frame) };
        self.interpret(rc, frame)
    }

    fn interpret(
        &self,
        rc: c_int,
        frame: *mut ffi::vvdecFrame,
    ) -> Result<Option<Frame<'_>>, VvdecError> {
        match ffi::vvdecErrorCodes(rc) {
            ffi::vvdecErrorCodes::VVDEC_OK => match NonNull::new(frame) {
                Some(raw) => Ok(Some(Frame {
                    raw,
                    decoder: self.decoder,
                    _decoder: PhantomData,
                })),
                None => Ok(None),
            },
            ffi::vvdecErrorCodes::VVDEC_TRY_AGAIN | ffi::vvdecErrorCodes::VVDEC_EOF => Ok(None),
            _ => Err(VvdecError::from_code(rc)),
        }
    }
}

impl Drop for Decoder {
    fn drop(&mut self) {
        // SAFETY: both handles are owned by `self` and not used afterwards. Any `Frame` borrowed
        // from this decoder has already been dropped (it borrows `&self`).
        unsafe {
            ffi::vvdec_accessUnit_free(self.access_unit.as_ptr());
            ffi::vvdec_decoder_close(self.decoder.as_ptr());
        }
    }
}

/// An output picture, valid until it is dropped. Borrows the [`Decoder`] so it cannot outlive it.
pub(super) struct Frame<'d> {
    raw: NonNull<ffi::vvdecFrame>,
    decoder: NonNull<ffi::vvdecDecoder>,
    _decoder: PhantomData<&'d Decoder>,
}

impl Frame<'_> {
    fn raw(&self) -> &ffi::vvdecFrame {
        // SAFETY: vvdec keeps the frame alive until `vvdec_frame_unref` (our `Drop`).
        unsafe { self.raw.as_ref() }
    }

    /// Translates the frame into neutral types, borrowing its pixel data. Fails on a colour
    /// format the header does not define, so a newer library cannot smuggle in an unknown value.
    pub(super) fn picture(&self) -> Result<Picture<'_>, VvdecError> {
        let f = self.raw();
        let chroma = match f.colorFormat {
            ffi::vvdecColorFormat::VVDEC_CF_YUV400_PLANAR => ChromaFormat::Monochrome,
            ffi::vvdecColorFormat::VVDEC_CF_YUV420_PLANAR => ChromaFormat::Yuv420,
            ffi::vvdecColorFormat::VVDEC_CF_YUV422_PLANAR => ChromaFormat::Yuv422,
            ffi::vvdecColorFormat::VVDEC_CF_YUV444_PLANAR => ChromaFormat::Yuv444,
            other => {
                return Err(VvdecError {
                    code: ffi::vvdecErrorCodes::VVDEC_ERR_NOT_SUPPORTED.0,
                    kind: ErrorKind::Fatal,
                    message: format!("unsupported colour format {}", other.0),
                })
            }
        };
        let planes = [self.plane(0), self.plane(1), self.plane(2)];
        Ok(Picture {
            width: f.width,
            height: f.height,
            bit_depth: f.bitDepth,
            chroma,
            planes,
            cts: f.ctsValid.then_some(f.cts as i64),
            kind: self.kind(),
        })
    }

    fn kind(&self) -> PictureKind {
        // SAFETY: `picAttributes` is NULL or points to attributes owned by the frame.
        let Some(attrs) = (unsafe { self.raw().picAttributes.as_ref() }) else {
            return PictureKind::Inter;
        };
        match (attrs.nalType, attrs.sliceType) {
            (
                ffi::vvdecNalType::VVC_NAL_UNIT_CODED_SLICE_IDR_W_RADL
                | ffi::vvdecNalType::VVC_NAL_UNIT_CODED_SLICE_IDR_N_LP
                | ffi::vvdecNalType::VVC_NAL_UNIT_CODED_SLICE_CRA,
                _,
            ) => PictureKind::RandomAccess,
            (ffi::vvdecNalType::VVC_NAL_UNIT_CODED_SLICE_GDR, _) => PictureKind::GradualRefresh,
            (_, ffi::vvdecSliceType::VVDEC_SLICETYPE_I) => PictureKind::Intra,
            _ => PictureKind::Inter,
        }
    }

    /// Plane `index` (0 = Y, 1 = Cb, 2 = Cr), or `None` if absent or malformed. `data` covers
    /// exactly the bytes the plane occupies: `(height - 1) * stride + width * bytes_per_sample`,
    /// so the final row is not assumed to be padded out to a full stride.
    fn plane(&self, index: usize) -> Option<PlaneView<'_>> {
        let f = self.raw();
        if index >= (f.numPlanes as usize).min(3) {
            return None;
        }
        let p = &f.planes[index];
        let (width, height) = (p.width as usize, p.height as usize);
        let (stride, bytes_per_sample) = (p.stride as usize, p.bytesPerSample as usize);
        if p.ptr.is_null() || width == 0 || height == 0 || !matches!(bytes_per_sample, 1 | 2) {
            return None;
        }
        let row_bytes = width.checked_mul(bytes_per_sample)?;
        if stride < row_bytes {
            return None;
        }
        let len = (height - 1).checked_mul(stride)?.checked_add(row_bytes)?;
        // SAFETY: vvdec guarantees the plane buffer holds `height` rows of `stride` bytes starting
        // at `ptr` (the last row needs only `row_bytes` of them), valid until the frame is
        // unref'd; the slice borrows `self`, so it cannot outlive the frame.
        let data = unsafe { std::slice::from_raw_parts(p.ptr, len) };
        Some(PlaneView {
            data,
            width,
            height,
            stride,
            bytes_per_sample,
        })
    }
}

impl Drop for Frame<'_> {
    fn drop(&mut self) {
        // SAFETY: `raw` came from `vvdec_decode`/`vvdec_flush` of this decoder and is released
        // exactly once, here. The decoder outlives the frame (lifetime `'d`).
        unsafe {
            ffi::vvdec_frame_unref(self.decoder.as_ptr(), self.raw.as_ptr());
        }
    }
}
