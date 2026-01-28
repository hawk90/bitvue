//! VVC/H.266 decoder using vvdec
//!
//! This module provides VVC decoding capabilities using the Fraunhofer vvdec library.
//! vvdec is an open-source VVC decoder optimized for performance.
//!
//! # Requirements
//!
//! The vvdec library must be installed on the system:
//! - macOS: `brew install vvdec`
//! - Linux: Build from source at https://github.com/fraunhoferhhi/vvdec
//! - Windows: Download prebuilt binaries from vvdec releases
//!
//! # Features
//!
//! - Full VVC/H.266 Main 10 profile support
//! - 8/10/12 bit depth
//! - All chroma formats (4:2:0, 4:2:2, 4:4:4)
//! - Multi-threaded decoding

use crate::decoder::{DecodeError, DecodedFrame, FrameType, Result};
use crate::traits::{CodecType, Decoder, DecoderCapabilities};
use std::ffi::c_void;
use std::ptr;
use tracing::{debug, error, warn};

// vvdec FFI bindings
mod ffi {
    use std::ffi::c_void;
    use std::os::raw::{c_char, c_int, c_uint};

    /// vvdec decoder handle
    pub type VvdecDecoder = c_void;

    /// vvdec access unit (encoded data container)
    #[repr(C)]
    pub struct VvdecAccessUnit {
        pub payload: *mut u8,
        pub payload_size: c_int,
        pub payload_used_size: c_int,
        pub cts: i64,
        pub dts: i64,
        pub ctsValid: bool,
        pub dtsValid: bool,
        pub rap: bool,
    }

    /// vvdec picture plane
    #[repr(C)]
    pub struct VvdecPlane {
        pub ptr: *mut u8,
        pub width: c_uint,
        pub height: c_uint,
        pub stride: c_uint,
        pub bytes_per_sample: c_uint,
    }

    /// vvdec picture component
    #[repr(C)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum VvdecComponentType {
        Y = 0,
        Cb = 1,
        Cr = 2,
    }

    /// vvdec frame information
    #[repr(C)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum VvdecFrameType {
        Auto = 0,
        I = 1,
        P = 2,
        B = 3,
        Idr = 4,
        Cra = 5,
        Gdr = 6,
    }

    /// vvdec color format
    #[repr(C)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum VvdecColorFormat {
        Invalid = -1,
        Yuv400Planar = 0,
        Yuv420Planar = 1,
        Yuv422Planar = 2,
        Yuv444Planar = 3,
    }

    /// vvdec picture (decoded frame)
    #[repr(C)]
    pub struct VvdecFrame {
        pub planes: [VvdecPlane; 3],
        pub num_planes: c_uint,
        pub width: c_uint,
        pub height: c_uint,
        pub bit_depth: c_uint,
        pub frame_type: VvdecFrameType,
        pub color_format: VvdecColorFormat,
        pub cts: i64,
        pub ctsValid: bool,
        pub picAttributes: *mut c_void,
    }

    /// vvdec parameters
    #[repr(C)]
    pub struct VvdecParams {
        pub threads: c_int,
        pub parseThreads: c_int,
        pub simd: c_int,
        pub logLevel: c_int,
        pub verifyPictureHash: c_int,
        pub removePadding: c_int,
        pub opaque: *mut c_void,
    }

    /// vvdec return codes
    pub const VVDEC_OK: c_int = 0;
    pub const VVDEC_ERR_UNSPECIFIED: c_int = -1;
    pub const VVDEC_ERR_INITIALIZE: c_int = -2;
    pub const VVDEC_ERR_ALLOCATE: c_int = -3;
    pub const VVDEC_ERR_DEC_INPUT: c_int = -4;
    pub const VVDEC_NOT_ENOUGH_MEM: c_int = -5;
    pub const VVDEC_ERR_PARAMETER: c_int = -6;
    pub const VVDEC_ERR_NOT_SUPPORTED: c_int = -7;
    pub const VVDEC_ERR_RESTART_REQUIRED: c_int = -8;
    pub const VVDEC_ERR_CPU: c_int = -9;
    pub const VVDEC_TRY_AGAIN: c_int = -10;
    pub const VVDEC_EOF: c_int = -11;

    #[link(name = "vvdec")]
    extern "C" {
        pub fn vvdec_params_default(params: *mut VvdecParams);
        pub fn vvdec_decoder_open(params: *const VvdecParams) -> *mut VvdecDecoder;
        pub fn vvdec_decoder_close(decoder: *mut VvdecDecoder) -> c_int;
        pub fn vvdec_decode(
            decoder: *mut VvdecDecoder,
            access_unit: *mut VvdecAccessUnit,
            frame: *mut *mut VvdecFrame,
        ) -> c_int;
        pub fn vvdec_flush(decoder: *mut VvdecDecoder, frame: *mut *mut VvdecFrame) -> c_int;
        pub fn vvdec_frame_unref(decoder: *mut VvdecDecoder, frame: *mut VvdecFrame) -> c_int;
        pub fn vvdec_accessUnit_alloc() -> *mut VvdecAccessUnit;
        pub fn vvdec_accessUnit_free(access_unit: *mut VvdecAccessUnit);
        pub fn vvdec_accessUnit_alloc_payload(
            access_unit: *mut VvdecAccessUnit,
            size: c_int,
        ) -> c_int;
        pub fn vvdec_accessUnit_free_payload(access_unit: *mut VvdecAccessUnit);
        pub fn vvdec_get_error_msg(error_code: c_int) -> *const c_char;
        pub fn vvdec_get_version() -> *const c_char;
    }
}

/// VVC/H.266 decoder using vvdec library
///
/// # Thread Safety
///
/// **NOT thread-safe!** The underlying vvdec library uses internal state that is not
/// protected by mutexes. Each thread should create its own VvcDecoder instance.
///
/// The `Send` impl is deliberately omitted because vvdec may have race conditions
/// when used concurrently. See: https://github.com/fraunhoferhhi/vvdec/issues
pub struct VvcDecoder {
    decoder: *mut c_void,
    access_unit: *mut ffi::VvdecAccessUnit,
    flushing: bool,
}

impl VvcDecoder {
    /// Create a new VVC decoder
    pub fn new() -> Result<Self> {
        unsafe {
            // Initialize parameters with defaults
            let mut params: ffi::VvdecParams = std::mem::zeroed();
            ffi::vvdec_params_default(&mut params);

            // Configure for multi-threaded decoding
            params.threads = 0; // 0 = auto-detect
            params.parseThreads = -1; // -1 = auto
            params.logLevel = 0; // Quiet

            // Open decoder
            let decoder = ffi::vvdec_decoder_open(&params);
            if decoder.is_null() {
                return Err(DecodeError::Init(
                    "Failed to open vvdec decoder".to_string(),
                ));
            }

            // Allocate access unit for input
            let access_unit = ffi::vvdec_accessUnit_alloc();
            if access_unit.is_null() {
                ffi::vvdec_decoder_close(decoder);
                return Err(DecodeError::Init(
                    "Failed to allocate vvdec access unit".to_string(),
                ));
            }

            debug!("VVC decoder initialized successfully");

            Ok(Self {
                decoder,
                access_unit,
                flushing: false,
            })
        }
    }

    /// Convert vvdec frame to DecodedFrame
    fn convert_frame(&self, frame: *mut ffi::VvdecFrame) -> Result<DecodedFrame> {
        unsafe {
            if frame.is_null() {
                return Err(DecodeError::NoFrame);
            }

            let vf = &*frame;

            let width = vf.width;
            let height = vf.height;
            let bit_depth = vf.bit_depth as u8;

            // Extract Y plane
            let y_plane = &vf.planes[0];
            let y_data = self.extract_plane(y_plane, height as usize);
            let y_stride = y_plane.stride as usize;

            // Extract U and V planes (if present)
            let (u_data, u_stride, v_data, v_stride) = if vf.num_planes > 1 {
                let u_plane = &vf.planes[1];
                let v_plane = &vf.planes[2];

                let chroma_height = match vf.color_format {
                    ffi::VvdecColorFormat::Yuv420Planar => height as usize / 2,
                    _ => height as usize,
                };

                (
                    Some(self.extract_plane(u_plane, chroma_height)),
                    u_plane.stride as usize,
                    Some(self.extract_plane(v_plane, chroma_height)),
                    v_plane.stride as usize,
                )
            } else {
                (None, 0, None, 0)
            };

            // Convert frame type
            let frame_type = match vf.frame_type {
                ffi::VvdecFrameType::I | ffi::VvdecFrameType::Idr | ffi::VvdecFrameType::Cra => {
                    FrameType::Key
                }
                ffi::VvdecFrameType::P | ffi::VvdecFrameType::B => FrameType::Inter,
                ffi::VvdecFrameType::Gdr => FrameType::Intra, // GDR is gradual intra refresh
                _ => FrameType::Inter,
            };

            let timestamp = if vf.ctsValid { vf.cts } else { 0 };

            Ok(DecodedFrame {
                width,
                height,
                bit_depth,
                y_plane: y_data,
                y_stride,
                u_plane: u_data,
                u_stride,
                v_plane: v_data,
                v_stride,
                timestamp,
                frame_type,
                qp_avg: None, // vvdec doesn't expose QP
            })
        }
    }

    /// Extract plane data from vvdec plane
    fn extract_plane(&self, plane: &ffi::VvdecPlane, height: usize) -> Vec<u8> {
        if plane.ptr.is_null() {
            return Vec::new();
        }

        let bytes_per_sample = plane.bytes_per_sample as usize;
        let row_bytes = plane.width as usize * bytes_per_sample;
        let stride = plane.stride as usize;

        let mut data = Vec::with_capacity(row_bytes * height);

        unsafe {
            for row in 0..height {
                let src = plane.ptr.add(row * stride);
                let slice = std::slice::from_raw_parts(src, row_bytes);
                data.extend_from_slice(slice);
            }
        }

        data
    }

    /// Get error message from vvdec error code
    fn error_message(code: i32) -> String {
        unsafe {
            let msg = ffi::vvdec_get_error_msg(code);
            if msg.is_null() {
                format!("Unknown error ({})", code)
            } else {
                std::ffi::CStr::from_ptr(msg).to_string_lossy().into_owned()
            }
        }
    }
}

impl Decoder for VvcDecoder {
    fn codec_type(&self) -> CodecType {
        CodecType::H266
    }

    fn capabilities(&self) -> DecoderCapabilities {
        DecoderCapabilities {
            codec: CodecType::H266,
            max_width: 8192,
            max_height: 8192,
            supported_bit_depths: vec![8, 10, 12],
            hw_accel: false, // vvdec is software-only
        }
    }

    fn send_data(&mut self, data: &[u8], timestamp: Option<i64>) -> Result<()> {
        if data.is_empty() {
            return Ok(());
        }

        unsafe {
            // Allocate payload buffer
            let ret = ffi::vvdec_accessUnit_alloc_payload(self.access_unit, data.len() as i32);
            if ret != ffi::VVDEC_OK {
                return Err(DecodeError::Decode(format!(
                    "Failed to allocate payload: {}",
                    Self::error_message(ret)
                )));
            }

            // Copy data to access unit
            let au = &mut *self.access_unit;
            ptr::copy_nonoverlapping(data.as_ptr(), au.payload, data.len());
            au.payload_used_size = data.len() as i32;
            au.cts = timestamp.unwrap_or(0);
            au.ctsValid = timestamp.is_some();
            au.dtsValid = false;
        }

        Ok(())
    }

    fn get_frame(&mut self) -> Result<DecodedFrame> {
        unsafe {
            let mut frame_ptr: *mut ffi::VvdecFrame = ptr::null_mut();

            let ret = if self.flushing {
                ffi::vvdec_flush(self.decoder, &mut frame_ptr)
            } else {
                ffi::vvdec_decode(self.decoder, self.access_unit, &mut frame_ptr)
            };

            match ret {
                ffi::VVDEC_OK => {
                    let frame = self.convert_frame(frame_ptr)?;
                    ffi::vvdec_frame_unref(self.decoder, frame_ptr);

                    // Clear access unit after successful decode
                    ffi::vvdec_accessUnit_free_payload(self.access_unit);

                    Ok(frame)
                }
                ffi::VVDEC_TRY_AGAIN => {
                    debug!("vvdec: need more data");
                    Err(DecodeError::NoFrame)
                }
                ffi::VVDEC_EOF => {
                    debug!("vvdec: end of stream");
                    Err(DecodeError::NoFrame)
                }
                _ => {
                    let msg = Self::error_message(ret);
                    error!("vvdec decode error: {}", msg);
                    Err(DecodeError::Decode(msg))
                }
            }
        }
    }

    fn flush(&mut self) {
        self.flushing = true;
        // Drain remaining frames
        loop {
            match self.get_frame() {
                Ok(_) => continue,
                Err(_) => break,
            }
        }
        self.flushing = false;
    }

    fn reset(&mut self) -> Result<()> {
        // Close and reopen decoder
        unsafe {
            ffi::vvdec_accessUnit_free_payload(self.access_unit);
            ffi::vvdec_accessUnit_free(self.access_unit);
            ffi::vvdec_decoder_close(self.decoder);
        }

        let new_decoder = Self::new()?;
        *self = new_decoder;
        Ok(())
    }
}

impl Drop for VvcDecoder {
    fn drop(&mut self) {
        unsafe {
            if !self.access_unit.is_null() {
                ffi::vvdec_accessUnit_free_payload(self.access_unit);
                ffi::vvdec_accessUnit_free(self.access_unit);
            }
            if !self.decoder.is_null() {
                ffi::vvdec_decoder_close(self.decoder);
            }
        }
        debug!("VVC decoder closed");
    }
}

/// Get vvdec library version
pub fn vvdec_version() -> String {
    unsafe {
        let version = ffi::vvdec_get_version();
        if version.is_null() {
            "unknown".to_string()
        } else {
            std::ffi::CStr::from_ptr(version)
                .to_string_lossy()
                .into_owned()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vvc_decoder_creation() {
        // This test will only pass if vvdec is installed
        match VvcDecoder::new() {
            Ok(decoder) => {
                assert_eq!(decoder.codec_type(), CodecType::H266);
                let caps = decoder.capabilities();
                assert!(caps.supported_bit_depths.contains(&10));
            }
            Err(e) => {
                // Expected if vvdec is not installed
                eprintln!("VVC decoder not available: {}", e);
            }
        }
    }

    #[test]
    fn test_vvdec_version() {
        // Only run if vvdec is available
        if std::process::Command::new("pkg-config")
            .args(["--exists", "vvdec"])
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
        {
            let version = vvdec_version();
            println!("vvdec version: {}", version);
            assert!(!version.is_empty());
        }
    }
}
