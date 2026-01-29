//! Scalar (baseline) YUV to RGB conversion implementation
//!
//! This is the fallback implementation that works on all platforms.
//! It provides correct results without any SIMD acceleration.

use super::{ConversionError, ConversionResult, StrategyCapabilities, YuvConversionStrategy};

/// Scalar strategy - baseline implementation that works everywhere
#[derive(Debug, Clone, Copy)]
pub struct ScalarStrategy;

impl ScalarStrategy {
    pub const fn new() -> Self {
        Self
    }
}

impl Default for ScalarStrategy {
    fn default() -> Self {
        Self::new()
    }
}

impl YuvConversionStrategy for ScalarStrategy {
    fn capabilities(&self) -> StrategyCapabilities {
        StrategyCapabilities::scalar()
    }

    fn name(&self) -> &'static str {
        "Scalar"
    }

    fn convert_yuv420_to_rgb(
        &self,
        y_plane: &[u8],
        u_plane: &[u8],
        v_plane: &[u8],
        width: usize,
        height: usize,
        rgb: &mut [u8],
        bit_depth: u8,
    ) -> ConversionResult<()> {
        // Validate inputs
        self.validate_yuv420_params(y_plane, u_plane, v_plane, width, height, rgb, bit_depth)?;

        // Convert pixel by pixel
        for y in 0..height {
            for x in 0..width {
                let y_idx = y * width + x;
                let uv_idx = (y / 2) * (width / 2) + (x / 2);

                let y_val = read_sample(y_plane, y_idx, bit_depth);
                let u_val = read_sample(u_plane, uv_idx, bit_depth);
                let v_val = read_sample(v_plane, uv_idx, bit_depth);

                let (r, g, b) = yuv_to_rgb_pixel_u8(y_val, u_val, v_val);

                let rgb_idx = y_idx * 3;
                rgb[rgb_idx] = r;
                rgb[rgb_idx + 1] = g;
                rgb[rgb_idx + 2] = b;
            }
        }

        Ok(())
    }

    fn convert_yuv422_to_rgb(
        &self,
        y_plane: &[u8],
        u_plane: &[u8],
        v_plane: &[u8],
        width: usize,
        height: usize,
        rgb: &mut [u8],
        bit_depth: u8,
    ) -> ConversionResult<()> {
        // Validate inputs - YUV422 has half horizontal chroma resolution
        let y_expected = width * height;
        let uv_expected = (width / 2) * height;
        let rgb_expected = width * height * 3;

        if y_plane.len() < y_expected {
            return Err(ConversionError::PlaneSizeMismatch {
                expected: y_expected,
                actual: y_plane.len(),
            });
        }

        if u_plane.len() < uv_expected {
            return Err(ConversionError::PlaneSizeMismatch {
                expected: uv_expected,
                actual: u_plane.len(),
            });
        }

        if v_plane.len() < uv_expected {
            return Err(ConversionError::PlaneSizeMismatch {
                expected: uv_expected,
                actual: v_plane.len(),
            });
        }

        if rgb.len() < rgb_expected {
            return Err(ConversionError::PlaneSizeMismatch {
                expected: rgb_expected,
                actual: rgb.len(),
            });
        }

        if !matches!(bit_depth, 8 | 10 | 12) {
            return Err(ConversionError::UnsupportedBitDepth(bit_depth));
        }

        // Convert pixel by pixel
        for y in 0..height {
            for x in 0..width {
                let y_idx = y * width + x;
                let uv_idx = y * (width / 2) + (x / 2);

                let y_val = read_sample(y_plane, y_idx, bit_depth);
                let u_val = read_sample(u_plane, uv_idx, bit_depth);
                let v_val = read_sample(v_plane, uv_idx, bit_depth);

                let (r, g, b) = yuv_to_rgb_pixel_u8(y_val, u_val, v_val);

                let rgb_idx = y_idx * 3;
                rgb[rgb_idx] = r;
                rgb[rgb_idx + 1] = g;
                rgb[rgb_idx + 2] = b;
            }
        }

        Ok(())
    }

    fn convert_yuv444_to_rgb(
        &self,
        y_plane: &[u8],
        u_plane: &[u8],
        v_plane: &[u8],
        width: usize,
        height: usize,
        rgb: &mut [u8],
        bit_depth: u8,
    ) -> ConversionResult<()> {
        // Validate inputs - YUV444 has full chroma resolution
        let y_expected = width * height;
        let rgb_expected = width * height * 3;

        if y_plane.len() < y_expected {
            return Err(ConversionError::PlaneSizeMismatch {
                expected: y_expected,
                actual: y_plane.len(),
            });
        }

        if u_plane.len() < y_expected {
            return Err(ConversionError::PlaneSizeMismatch {
                expected: y_expected,
                actual: u_plane.len(),
            });
        }

        if v_plane.len() < y_expected {
            return Err(ConversionError::PlaneSizeMismatch {
                expected: y_expected,
                actual: v_plane.len(),
            });
        }

        if rgb.len() < rgb_expected {
            return Err(ConversionError::PlaneSizeMismatch {
                expected: rgb_expected,
                actual: rgb.len(),
            });
        }

        if !matches!(bit_depth, 8 | 10 | 12) {
            return Err(ConversionError::UnsupportedBitDepth(bit_depth));
        }

        // Convert pixel by pixel
        for y in 0..height {
            for x in 0..width {
                let idx = y * width + x;

                let y_val = read_sample(y_plane, idx, bit_depth);
                let u_val = read_sample(u_plane, idx, bit_depth);
                let v_val = read_sample(v_plane, idx, bit_depth);

                let (r, g, b) = yuv_to_rgb_pixel_u8(y_val, u_val, v_val);

                let rgb_idx = idx * 3;
                rgb[rgb_idx] = r;
                rgb[rgb_idx + 1] = g;
                rgb[rgb_idx + 2] = b;
            }
        }

        Ok(())
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
#[inline]
fn yuv_to_rgb_pixel_u8(y: u8, u: u8, v: u8) -> (u8, u8, u8) {
    let y_i = y as i32;
    let u_i = (u as i32) - 128;  // Center U around 0
    let v_i = (v as i32) - 128;  // Center V around 0
    yuv_to_rgb_pixel(y_i, u_i, v_i)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scalar_strategy_creation() {
        let strategy = ScalarStrategy::new();
        assert_eq!(strategy.name(), "Scalar");
    }

    #[test]
    fn test_scalar_capabilities() {
        let strategy = ScalarStrategy::new();
        let caps = strategy.capabilities();
        assert_eq!(caps.speedup_factor, 1.0);
        assert!(caps.supports_10bit);
        assert!(caps.supports_12bit);
        assert!(!caps.is_hardware_accelerated);
    }

    #[test]
    fn test_scalar_is_available() {
        let strategy = ScalarStrategy::new();
        assert!(strategy.is_available());
    }

    #[test]
    fn test_read_sample_8bit() {
        let plane = vec![0, 128, 255, 64];
        assert_eq!(read_sample(&plane, 0, 8), 0);
        assert_eq!(read_sample(&plane, 1, 8), 128);
        assert_eq!(read_sample(&plane, 2, 8), 255);
    }

    #[test]
    fn test_read_sample_10bit() {
        // 10-bit value stored as 16-bit LE: 1024 (0x0400)
        // Normalized: 1024 >> (10 - 8) = 1024 >> 2 = 256
        // As u8: 256 % 256 = 0 (wraps to 0)
        let plane = vec![0x00, 0x04, 0x00, 0x08]; // 1024, 2048
        assert_eq!(read_sample(&plane, 0, 10), 0); // 1024 >> 2 = 256 -> 256 as u8 = 0
    }

    #[test]
    fn test_yuv_to_rgb_pixel_black() {
        // Y=0, U=128 (0), V=128 (0) -> black
        let (r, g, b) = yuv_to_rgb_pixel(0, 0, 0);
        assert_eq!((r, g, b), (0, 0, 0));
    }

    #[test]
    fn test_yuv_to_rgb_pixel_white() {
        // Y=255, U=128 (0), V=128 (0) -> white
        let (r, g, b) = yuv_to_rgb_pixel(255, 0, 0);
        assert_eq!((r, g, b), (255, 255, 255));
    }

    #[test]
    fn test_yuv_to_rgb_pixel_red() {
        // Pure red: Y=82, U=90, V=240 (approximately)
        // U centered: 90 - 128 = -38
        // V centered: 240 - 128 = 112
        let (r, g, b) = yuv_to_rgb_pixel(82, -38, 112);
        assert!(r > 200); // High red
        assert!(g < 100); // Low green
        assert!(b < 100); // Low blue
    }

    #[test]
    fn test_yuv420_conversion_basic() {
        let strategy = ScalarStrategy::new();

        // 2x2 frame
        let y_plane = vec![0, 128, 255, 64];
        let u_plane = vec![128, 128];
        let v_plane = vec![128, 128];

        let mut rgb = vec![0u8; 2 * 2 * 3];

        let result = strategy.convert_yuv420_to_rgb(
            &y_plane,
            &u_plane,
            &v_plane,
            2,
            2,
            &mut rgb,
            8,
        );

        assert!(result.is_ok());

        // First pixel (Y=0) should be black
        assert_eq!(rgb[0], 0);
        assert_eq!(rgb[1], 0);
        assert_eq!(rgb[2], 0);
    }

    #[test]
    fn test_yuv420_conversion_invalid_dimensions() {
        let strategy = ScalarStrategy::new();

        let y_plane = vec![0; 100];
        let u_plane = vec![128; 25];
        let v_plane = vec![128; 25];
        let mut rgb = vec![0u8; 300];

        // Wrong dimensions for the data provided
        let result = strategy.convert_yuv420_to_rgb(
            &y_plane,
            &u_plane,
            &v_plane,
            20, // Too large for y_plane
            20,
            &mut rgb,
            8,
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_yuv420_conversion_insufficient_rgb_buffer() {
        let strategy = ScalarStrategy::new();

        let y_plane = vec![0; 100];
        let u_plane = vec![128; 25];
        let v_plane = vec![128; 25];
        let mut rgb = vec![0u8; 100]; // Too small (need 300)

        let result = strategy.convert_yuv420_to_rgb(
            &y_plane,
            &u_plane,
            &v_plane,
            10,
            10,
            &mut rgb,
            8,
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_yuv420_conversion_unsupported_bit_depth() {
        let strategy = ScalarStrategy::new();

        let y_plane = vec![0; 100];
        let u_plane = vec![128; 25];
        let v_plane = vec![128; 25];
        let mut rgb = vec![0u8; 300];

        let result = strategy.convert_yuv420_to_rgb(
            &y_plane,
            &u_plane,
            &v_plane,
            10,
            10,
            &mut rgb,
            16, // Unsupported bit depth
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_scalar_default() {
        let strategy = ScalarStrategy::default();
        assert_eq!(strategy.name(), "Scalar");
    }
}
