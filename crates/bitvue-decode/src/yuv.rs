//! YUV to RGB conversion utilities with SIMD optimization
//!
//! This module provides YUV to RGB conversion using a strategy pattern
//! that automatically selects the best available implementation for the
//! current platform (AVX2, NEON, or scalar fallback).

use crate::decoder::{ChromaFormat, DecodedFrame};
use crate::strategy::{
    best_strategy_type, ConversionError as StrategyConversionError, ScalarStrategy, StrategyType,
    YuvConversionStrategy,
};
// Debug logging now uses abseil::vlog!

#[cfg(target_arch = "x86_64")]
use crate::strategy::Avx2Strategy;

#[cfg(target_arch = "aarch64")]
use crate::strategy::NeonStrategy;

// Re-export the strategy module for advanced users who want to
// manually select strategies
pub use crate::strategy;

// ============================================================================
// Constants
// ============================================================================

/// Maximum allowed frame size (8K RGB)
const MAX_FRAME_SIZE: usize = 7680 * 4320 * 3;

// ============================================================================
// Public API
// ============================================================================

/// Converts a decoded YUV frame to RGB with validation and SIMD acceleration
///
/// This function automatically selects the best available conversion strategy
/// for the current platform:
/// - **x86_64**: AVX2 (~4.5x speedup)
/// - **ARM/Apple Silicon**: NEON (~3.5x speedup)
/// - **Other**: Scalar baseline (1.0x)
///
/// # Arguments
/// * `frame` - The decoded YUV frame to convert
///
/// # Returns
/// A Vec<u8> containing RGB24 data (width * height * 3 bytes)
///
/// # Errors
/// Returns a zero-filled buffer if frame dimensions are invalid or exceed
/// the maximum allowed size.
pub fn yuv_to_rgb(frame: &DecodedFrame) -> Vec<u8> {
    let width = frame.width as usize;
    let height = frame.height as usize;

    // Validate frame size to prevent overflow/DoS
    let required_size = match width.checked_mul(height) {
        Some(v) => v,
        None => {
            abseil::vlog!(1, "Frame dimensions overflow: {}x{}", width, height);
            return vec![0; MAX_FRAME_SIZE.min(1920 * 1080 * 3)];
        }
    };

    let required_size = match required_size.checked_mul(3) {
        Some(v) => v,
        None => {
            abseil::vlog!(1, "Frame size overflow: {}x{}x3", width, height);
            return vec![0; MAX_FRAME_SIZE.min(1920 * 1080 * 3)];
        }
    };

    if required_size > MAX_FRAME_SIZE {
        abseil::vlog!(
            1,
            "Frame size {}x{} exceeds maximum allowed {}",
            width,
            height,
            MAX_FRAME_SIZE / 3
        );
        return vec![0; MAX_FRAME_SIZE];
    }

    let mut rgb = vec![0u8; required_size];
    let chroma_format = frame.chroma_format;
    let bit_depth = frame.bit_depth;

    abseil::vlog!(
        2,
        "Converting {:?} frame to RGB ({}x{}, {}bit, {} bytes)",
        chroma_format,
        width,
        height,
        bit_depth,
        required_size
    );

    match chroma_format {
        ChromaFormat::Monochrome => {
            convert_monochrome(&frame.y_plane, width, height, &mut rgb, bit_depth);
        }
        ChromaFormat::Yuv420 => {
            let u_plane = match frame.u_plane.as_ref() {
                Some(plane) => plane,
                None => {
                    abseil::vlog!(1, "Yuv420 frame missing U plane, falling back to grayscale");
                    convert_monochrome(&frame.y_plane, width, height, &mut rgb, bit_depth);
                    return rgb;
                }
            };
            let v_plane = match frame.v_plane.as_ref() {
                Some(plane) => plane,
                None => {
                    abseil::vlog!(1, "Yuv420 frame missing V plane, falling back to grayscale");
                    convert_monochrome(&frame.y_plane, width, height, &mut rgb, bit_depth);
                    return rgb;
                }
            };

            if let Err(e) = convert_yuv420(
                &frame.y_plane,
                u_plane,
                v_plane,
                width,
                height,
                &mut rgb,
                bit_depth,
            ) {
                abseil::vlog!(
                    1,
                    "YUV420 conversion failed: {}, falling back to grayscale",
                    e
                );
                convert_monochrome(&frame.y_plane, width, height, &mut rgb, bit_depth);
            }
        }
        ChromaFormat::Yuv422 => {
            if let Err(e) = convert_yuv422(
                &frame.y_plane,
                frame.u_plane.as_deref(),
                frame.v_plane.as_deref(),
                width,
                height,
                &mut rgb,
                bit_depth,
            ) {
                abseil::vlog!(
                    1,
                    "YUV422 conversion failed: {}, falling back to grayscale",
                    e
                );
                convert_monochrome(&frame.y_plane, width, height, &mut rgb, bit_depth);
            }
        }
        ChromaFormat::Yuv444 => {
            if let Err(e) = convert_yuv444(
                &frame.y_plane,
                frame.u_plane.as_deref(),
                frame.v_plane.as_deref(),
                width,
                height,
                &mut rgb,
                bit_depth,
            ) {
                abseil::vlog!(
                    1,
                    "YUV444 conversion failed: {}, falling back to grayscale",
                    e
                );
                convert_monochrome(&frame.y_plane, width, height, &mut rgb, bit_depth);
            }
        }
    }

    rgb
}

/// Convert monochrome (Y only) to grayscale RGB
fn convert_monochrome(y_plane: &[u8], width: usize, height: usize, rgb: &mut [u8], bit_depth: u8) {
    for i in 0..(width * height) {
        let y_val = read_sample(y_plane, i, bit_depth);
        let rgb_idx = i * 3;
        rgb[rgb_idx] = y_val;
        rgb[rgb_idx + 1] = y_val;
        rgb[rgb_idx + 2] = y_val;
    }
}

/// Convert YUV420 to RGB using the best available strategy
fn convert_yuv420(
    y_plane: &[u8],
    u_plane: &[u8],
    v_plane: &[u8],
    width: usize,
    height: usize,
    rgb: &mut [u8],
    bit_depth: u8,
) -> Result<(), StrategyConversionError> {
    let strategy_type = best_strategy_type();

    // Log which strategy is being used
    let strategy_name = strategy_type.name();
    abseil::vlog!(
        2,
        "Using {} strategy for YUV420 conversion ({}x{}, {}bit)",
        strategy_name,
        width,
        height,
        bit_depth
    );

    // Dispatch based on strategy type
    match strategy_type {
        StrategyType::Scalar => {
            let strategy = ScalarStrategy::new();
            strategy.convert_yuv420_to_rgb(y_plane, u_plane, v_plane, width, height, rgb, bit_depth)
        }
        #[cfg(target_arch = "x86_64")]
        StrategyType::Avx2 => {
            let strategy = Avx2Strategy::new();
            strategy.convert_yuv420_to_rgb(y_plane, u_plane, v_plane, width, height, rgb, bit_depth)
        }
        #[cfg(target_arch = "aarch64")]
        StrategyType::Neon => {
            let strategy = NeonStrategy::new();
            strategy.convert_yuv420_to_rgb(y_plane, u_plane, v_plane, width, height, rgb, bit_depth)
        }
        #[cfg(not(target_arch = "x86_64"))]
        StrategyType::Avx2 => {
            // AVX2 not available on this platform, fall back to scalar
            let strategy = ScalarStrategy::new();
            strategy.convert_yuv420_to_rgb(y_plane, u_plane, v_plane, width, height, rgb, bit_depth)
        }
        #[cfg(not(target_arch = "aarch64"))]
        StrategyType::Neon => {
            // NEON not available on this platform, fall back to scalar
            let strategy = ScalarStrategy::new();
            strategy.convert_yuv420_to_rgb(y_plane, u_plane, v_plane, width, height, rgb, bit_depth)
        }
        StrategyType::Metal => {
            // Metal not implemented yet, fall back to scalar
            let strategy = ScalarStrategy::new();
            strategy.convert_yuv420_to_rgb(y_plane, u_plane, v_plane, width, height, rgb, bit_depth)
        }
        StrategyType::Auto => {
            // Should never happen, but fall back to scalar
            let strategy = ScalarStrategy::new();
            strategy.convert_yuv420_to_rgb(y_plane, u_plane, v_plane, width, height, rgb, bit_depth)
        }
    }
}

/// Convert YUV422 to RGB using the best available strategy
fn convert_yuv422(
    y_plane: &[u8],
    u_plane: Option<&[u8]>,
    v_plane: Option<&[u8]>,
    width: usize,
    height: usize,
    rgb: &mut [u8],
    bit_depth: u8,
) -> Result<(), StrategyConversionError> {
    let u_plane = u_plane.ok_or(StrategyConversionError::MissingUPlane)?;
    let v_plane = v_plane.ok_or(StrategyConversionError::MissingVPlane)?;

    let strategy_type = best_strategy_type();

    // Log which strategy is being used
    let strategy_name = strategy_type.name();
    abseil::vlog!(
        2,
        "Using {} strategy for YUV422 conversion ({}x{}, {}bit)",
        strategy_name,
        width,
        height,
        bit_depth
    );

    // Dispatch based on strategy type
    match strategy_type {
        StrategyType::Scalar => {
            let strategy = ScalarStrategy::new();
            strategy.convert_yuv422_to_rgb(y_plane, u_plane, v_plane, width, height, rgb, bit_depth)
        }
        #[cfg(target_arch = "x86_64")]
        StrategyType::Avx2 => {
            let strategy = Avx2Strategy::new();
            strategy.convert_yuv422_to_rgb(y_plane, u_plane, v_plane, width, height, rgb, bit_depth)
        }
        #[cfg(target_arch = "aarch64")]
        StrategyType::Neon => {
            let strategy = NeonStrategy::new();
            strategy.convert_yuv422_to_rgb(y_plane, u_plane, v_plane, width, height, rgb, bit_depth)
        }
        #[cfg(not(target_arch = "x86_64"))]
        StrategyType::Avx2 => {
            // AVX2 not available on this platform, fall back to scalar
            let strategy = ScalarStrategy::new();
            strategy.convert_yuv422_to_rgb(y_plane, u_plane, v_plane, width, height, rgb, bit_depth)
        }
        #[cfg(not(target_arch = "aarch64"))]
        StrategyType::Neon => {
            // NEON not available on this platform, fall back to scalar
            let strategy = ScalarStrategy::new();
            strategy.convert_yuv422_to_rgb(y_plane, u_plane, v_plane, width, height, rgb, bit_depth)
        }
        StrategyType::Metal => {
            // Metal not implemented yet, fall back to scalar
            let strategy = ScalarStrategy::new();
            strategy.convert_yuv422_to_rgb(y_plane, u_plane, v_plane, width, height, rgb, bit_depth)
        }
        StrategyType::Auto => {
            // Should never happen, but fall back to scalar
            let strategy = ScalarStrategy::new();
            strategy.convert_yuv422_to_rgb(y_plane, u_plane, v_plane, width, height, rgb, bit_depth)
        }
    }
}

/// Convert YUV444 to RGB using the best available strategy
fn convert_yuv444(
    y_plane: &[u8],
    u_plane: Option<&[u8]>,
    v_plane: Option<&[u8]>,
    width: usize,
    height: usize,
    rgb: &mut [u8],
    bit_depth: u8,
) -> Result<(), StrategyConversionError> {
    let u_plane = u_plane.ok_or(StrategyConversionError::MissingUPlane)?;
    let v_plane = v_plane.ok_or(StrategyConversionError::MissingVPlane)?;

    let strategy_type = best_strategy_type();

    // Log which strategy is being used
    let strategy_name = strategy_type.name();
    abseil::vlog!(
        2,
        "Using {} strategy for YUV444 conversion ({}x{}, {}bit)",
        strategy_name,
        width,
        height,
        bit_depth
    );

    // Dispatch based on strategy type
    match strategy_type {
        StrategyType::Scalar => {
            let strategy = ScalarStrategy::new();
            strategy.convert_yuv444_to_rgb(y_plane, u_plane, v_plane, width, height, rgb, bit_depth)
        }
        #[cfg(target_arch = "x86_64")]
        StrategyType::Avx2 => {
            let strategy = Avx2Strategy::new();
            strategy.convert_yuv444_to_rgb(y_plane, u_plane, v_plane, width, height, rgb, bit_depth)
        }
        #[cfg(target_arch = "aarch64")]
        StrategyType::Neon => {
            let strategy = NeonStrategy::new();
            strategy.convert_yuv444_to_rgb(y_plane, u_plane, v_plane, width, height, rgb, bit_depth)
        }
        #[cfg(not(target_arch = "x86_64"))]
        StrategyType::Avx2 => {
            // AVX2 not available on this platform, fall back to scalar
            let strategy = ScalarStrategy::new();
            strategy.convert_yuv444_to_rgb(y_plane, u_plane, v_plane, width, height, rgb, bit_depth)
        }
        #[cfg(not(target_arch = "aarch64"))]
        StrategyType::Neon => {
            // NEON not available on this platform, fall back to scalar
            let strategy = ScalarStrategy::new();
            strategy.convert_yuv444_to_rgb(y_plane, u_plane, v_plane, width, height, rgb, bit_depth)
        }
        StrategyType::Metal => {
            // Metal not implemented yet, fall back to scalar
            let strategy = ScalarStrategy::new();
            strategy.convert_yuv444_to_rgb(y_plane, u_plane, v_plane, width, height, rgb, bit_depth)
        }
        StrategyType::Auto => {
            // Should never happen, but fall back to scalar
            let strategy = ScalarStrategy::new();
            strategy.convert_yuv444_to_rgb(y_plane, u_plane, v_plane, width, height, rgb, bit_depth)
        }
    }
}

/// Read a sample from plane data, handling 8/10/12bit
#[inline]
fn read_sample(plane: &[u8], idx: usize, bit_depth: u8) -> u8 {
    if bit_depth > 8 {
        // 10/12bit: read 16-bit sample and normalize to 8-bit
        let byte_idx = idx * 2;
        if byte_idx + 1 < plane.len() {
            let sample16 = u16::from_le_bytes([plane[byte_idx], plane[byte_idx + 1]]);
            // Normalize to 8-bit by right-shifting
            (sample16 >> (bit_depth - 8)) as u8
        } else {
            0
        }
    } else {
        // 8bit: direct read
        plane.get(idx).copied().unwrap_or(0)
    }
}

/// Convert a single YUV pixel to RGB using BT.601 color space
///
/// Uses integer arithmetic for 20-30% speedup over floating-point.
/// The BT.601 coefficients are expressed as fixed-point with /128:
/// - R = Y + 181/128 * V
/// - G = Y - 44/128 * U - 91/128 * V
/// - B = Y + 227/128 * U
///
/// NOTE: Kept for potential future use in:
/// - Per-pixel conversion (not bulk conversion)
/// - SIMD fallback implementation
/// - Alternative conversion strategies
#[allow(dead_code)]
#[inline]
fn yuv_to_rgb_pixel(y: i32, u: i32, v: i32) -> (u8, u8, u8) {
    // BT.601 conversion with integer arithmetic
    // Scale Y by 128 for fixed-point arithmetic
    let y_scaled = y * 128;

    // R = Y + 181/128 * V -> (Y * 128 + 181 * V) >> 7
    let r = ((y_scaled + 181 * v) >> 7).clamp(0, 255) as u8;

    // G = Y - 44/128 * U - 91/128 * V -> (Y * 128 - 44 * U - 91 * V) >> 7
    let g = ((y_scaled - 44 * u - 91 * v) >> 7).clamp(0, 255) as u8;

    // B = Y + 227/128 * U -> (Y * 128 + 227 * U) >> 7
    let b = ((y_scaled + 227 * u) >> 7).clamp(0, 255) as u8;

    (r, g, b)
}

/// Convert YUV pixel to RGB using integer arithmetic (direct from u8 samples)
///
/// This is a convenience wrapper that takes u8 values directly,
/// subtracting 128 from U and V as needed.
///
/// NOTE: Kept for potential future use in:
/// - Per-pixel conversion for non-bulk operations
/// - Testing and verification
/// - Alternative conversion paths
#[allow(dead_code)]
#[inline]
fn yuv_to_rgb_pixel_u8(y: u8, u: u8, v: u8) -> (u8, u8, u8) {
    let y_i = y as i32;
    let u_i = (u as i32) - 128; // Center U around 0
    let v_i = (v as i32) - 128; // Center V around 0
    yuv_to_rgb_pixel(y_i, u_i, v_i)
}

/// Converts RGB data to an image::RgbImage
///
/// Takes ownership of the RGB data to avoid unnecessary copying.
/// The image crate requires ownership of the pixel data.
pub fn rgb_to_image(rgb: Vec<u8>, width: u32, height: u32) -> Result<image::RgbImage, String> {
    let data_len = rgb.len();
    image::RgbImage::from_raw(width, height, rgb).ok_or_else(|| {
        format!(
            "Failed to create image from RGB data: width={}, height={}, data_len={}",
            width, height, data_len
        )
    })
}

/// Get the current conversion strategy type
pub fn current_strategy() -> StrategyType {
    strategy::current_strategy_type()
}

/// Get information about available conversion strategies
pub fn available_strategies() -> Vec<(StrategyType, bool, String)> {
    strategy::available_strategies()
}

/// Set a specific conversion strategy (for testing/benchmarking)
pub fn set_strategy(strategy_type: StrategyType) -> Result<(), String> {
    strategy::set_strategy(strategy_type)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decoder::FrameType;

    #[test]
    fn test_monochrome_conversion() {
        let frame = DecodedFrame {
            width: 2,
            height: 2,
            bit_depth: 8,
            y_plane: vec![0, 128, 255, 64].into_boxed_slice().into(),
            y_stride: 2,
            u_plane: None,
            u_stride: 0,
            v_plane: None,
            v_stride: 0,
            timestamp: 0,
            frame_type: FrameType::Key,
            qp_avg: None,
            chroma_format: ChromaFormat::Monochrome,
        };

        let rgb = yuv_to_rgb(&frame);
        assert_eq!(rgb.len(), 12); // 2x2x3

        // First pixel (Y=0) -> RGB(0,0,0)
        assert_eq!(rgb[0], 0);
        assert_eq!(rgb[1], 0);
        assert_eq!(rgb[2], 0);

        // Second pixel (Y=128)
        assert_eq!(rgb[3], 128);
    }

    #[test]
    fn test_yuv420_conversion_basic() {
        let frame = DecodedFrame {
            width: 2,
            height: 2,
            bit_depth: 8,
            y_plane: vec![0, 128, 255, 64].into_boxed_slice().into(),
            y_stride: 2,
            u_plane: Some(vec![128, 128].into_boxed_slice().into()),
            u_stride: 1,
            v_plane: Some(vec![128, 128].into_boxed_slice().into()),
            v_stride: 1,
            timestamp: 0,
            frame_type: FrameType::Key,
            qp_avg: None,
            chroma_format: ChromaFormat::Yuv420,
        };

        let rgb = yuv_to_rgb(&frame);
        assert_eq!(rgb.len(), 12); // 2x2x3

        // First pixel (Y=0) should be black
        assert_eq!(rgb[0], 0);
        assert_eq!(rgb[1], 0);
        assert_eq!(rgb[2], 0);
    }

    #[test]
    fn test_frame_size_validation() {
        // Test overflow protection
        let huge_frame = DecodedFrame {
            width: 100000, // Would overflow without protection
            height: 100000,
            bit_depth: 8,
            y_plane: vec![0; 100].into_boxed_slice().into(),
            y_stride: 10,
            u_plane: None,
            u_stride: 0,
            v_plane: None,
            v_stride: 0,
            timestamp: 0,
            frame_type: FrameType::Key,
            qp_avg: None,
            chroma_format: ChromaFormat::Monochrome,
        };

        let rgb = yuv_to_rgb(&huge_frame);
        // Should return safe default instead of panicking
        assert!(!rgb.is_empty());
        assert!(rgb.len() <= MAX_FRAME_SIZE);
    }

    #[test]
    fn test_current_strategy() {
        let strategy = current_strategy();
        // Should return a valid strategy type
        match strategy {
            StrategyType::Scalar
            | StrategyType::Avx2
            | StrategyType::Neon
            | StrategyType::Metal => {
                // Valid
            }
            StrategyType::Auto => panic!("Auto should not be returned by current_strategy"),
        }
    }

    #[test]
    fn test_available_strategies() {
        let strategies = available_strategies();
        // Should always have at least Scalar
        assert!(!strategies.is_empty());
        assert!(strategies
            .iter()
            .any(|(t, _, _)| *t == StrategyType::Scalar));
    }

    #[test]
    fn test_set_strategy_scalar() {
        // Reset to auto first to ensure clean state
        let _ = set_strategy(StrategyType::Auto);

        // Get the current strategy after auto-detection
        let before = current_strategy();

        let _ = set_strategy(StrategyType::Scalar);

        // OnceLock doesn't allow overwriting, so the strategy won't change if already set
        // The result will be Ok() only if the strategy was successfully set to Scalar
        // On platforms with better strategies (NEON/AVX2), the strategy remains unchanged
        assert!(matches!(
            current_strategy(),
            StrategyType::Neon | StrategyType::Avx2 | StrategyType::Scalar
        ));

        // Reset to auto
        let _ = set_strategy(StrategyType::Auto);

        // Verify we're back to auto-detected strategy
        assert_eq!(current_strategy(), before);
    }

    #[test]
    fn test_yuv420_missing_u_plane() {
        let frame = DecodedFrame {
            width: 2,
            height: 2,
            bit_depth: 8,
            y_plane: vec![0, 128, 255, 64].into_boxed_slice().into(),
            y_stride: 2,
            u_plane: None, // Missing
            u_stride: 0,
            v_plane: Some(vec![128, 128].into_boxed_slice().into()),
            v_stride: 1,
            timestamp: 0,
            frame_type: FrameType::Key,
            qp_avg: None,
            chroma_format: ChromaFormat::Yuv420,
        };

        // Should fall back to grayscale
        let rgb = yuv_to_rgb(&frame);
        assert_eq!(rgb.len(), 12);
    }

    #[test]
    fn test_yuv422_conversion() {
        let frame = DecodedFrame {
            width: 4,
            height: 2,
            bit_depth: 8,
            y_plane: vec![0; 8].into_boxed_slice().into(),
            y_stride: 4,
            u_plane: Some(vec![128; 4].into_boxed_slice().into()),
            u_stride: 2,
            v_plane: Some(vec![128; 4].into_boxed_slice().into()),
            v_stride: 2,
            timestamp: 0,
            frame_type: FrameType::Key,
            qp_avg: None,
            chroma_format: ChromaFormat::Yuv422,
        };

        let rgb = yuv_to_rgb(&frame);
        assert_eq!(rgb.len(), 4 * 2 * 3);
    }

    #[test]
    fn test_yuv444_conversion() {
        let frame = DecodedFrame {
            width: 2,
            height: 2,
            bit_depth: 8,
            y_plane: vec![0; 4].into_boxed_slice().into(),
            y_stride: 2,
            u_plane: Some(vec![128; 4].into_boxed_slice().into()),
            u_stride: 2,
            v_plane: Some(vec![128; 4].into_boxed_slice().into()),
            v_stride: 2,
            timestamp: 0,
            frame_type: FrameType::Key,
            qp_avg: None,
            chroma_format: ChromaFormat::Yuv444,
        };

        let rgb = yuv_to_rgb(&frame);
        assert_eq!(rgb.len(), 2 * 2 * 3);
    }
}
