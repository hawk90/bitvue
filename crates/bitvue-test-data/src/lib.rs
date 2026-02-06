//! Test data generation utilities for bitvue
//!
//! This module provides utilities for generating test video data
//! including valid, corrupted, and edge case variants.

pub mod generators;
pub mod corruption;

pub use generators::*;
pub use corruption::*;

use bitvue_decode::DecodedFrame;

/// Create a minimal test frame with specified dimensions and chroma format
pub fn create_test_frame_with_params(
    width: u32,
    height: u32,
    bit_depth: u8,
    chroma_format: bitvue_decode::ChromaFormat,
) -> DecodedFrame {
    use bitvue_decode::ChromaFormat;

    let bytes_per_sample = if bit_depth > 8 { 2 } else { 1 };
    let y_size = (width * height) as usize * bytes_per_sample;

    let (u_plane, v_plane) = match chroma_format {
        ChromaFormat::Monochrome => (None, None),
        ChromaFormat::Yuv420 => {
            let uv_size = ((width / 2) * (height / 2)) as usize * bytes_per_sample;
            (
                Some(vec![128u8; uv_size].into_boxed_slice().into()),
                Some(vec![128u8; uv_size].into_boxed_slice().into()),
            )
        }
        ChromaFormat::Yuv422 => {
            let uv_size = ((width / 2) * height) as usize * bytes_per_sample;
            (
                Some(vec![128u8; uv_size].into_boxed_slice().into()),
                Some(vec![128u8; uv_size].into_boxed_slice().into()),
            )
        }
        ChromaFormat::Yuv444 => {
            let uv_size = (width * height) as usize * bytes_per_sample;
            (
                Some(vec![128u8; uv_size].into_boxed_slice().into()),
                Some(vec![128u8; uv_size].into_boxed_slice().into()),
            )
        }
    };

    let u_stride = match chroma_format {
        ChromaFormat::Monochrome => 0,
        ChromaFormat::Yuv420 => (width / 2) as usize * bytes_per_sample,
        ChromaFormat::Yuv422 => (width / 2) as usize * bytes_per_sample,
        ChromaFormat::Yuv444 => width as usize * bytes_per_sample,
    };

    let v_stride = u_stride;

    DecodedFrame {
        width,
        height,
        bit_depth,
        y_plane: vec![128u8; y_size].into_boxed_slice().into(),
        y_stride: width as usize * bytes_per_sample,
        u_plane,
        u_stride,
        v_plane,
        v_stride,
        timestamp: 0,
        frame_type: bitvue_decode::FrameType::Key,
        qp_avg: None,
        chroma_format,
    }
}

/// Create a standard test frame (320x240 YUV420)
pub fn create_test_frame() -> DecodedFrame {
    create_test_frame_with_params(320, 240, 8, bitvue_decode::ChromaFormat::Yuv420)
}

/// Create a test frame with specific bit depth
pub fn create_frame_with_bit_depth(bit_depth: u8) -> DecodedFrame {
    create_test_frame_with_params(320, 240, bit_depth, bitvue_decode::ChromaFormat::Monochrome)
}
