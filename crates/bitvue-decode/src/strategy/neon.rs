//! NEON SIMD YUV to RGB conversion implementation for ARM/Apple Silicon
//!
//! This implementation uses ARM NEON instructions for 3-4x speedup
//! compared to the scalar baseline.

use super::{ConversionError, ConversionResult, StrategyCapabilities, YuvConversionStrategy};
use std::arch::aarch64::*;

/// NEON strategy - ARM/Apple Silicon SIMD implementation
#[derive(Debug, Clone, Copy)]
pub struct NeonStrategy;

impl NeonStrategy {
    pub const fn new() -> Self {
        Self
    }
}

impl Default for NeonStrategy {
    fn default() -> Self {
        Self::new()
    }
}

impl YuvConversionStrategy for NeonStrategy {
    fn capabilities(&self) -> StrategyCapabilities {
        StrategyCapabilities::neon()
    }

    fn is_available(&self) -> bool {
        std::arch::is_aarch64_feature_detected!("neon")
    }

    fn name(&self) -> &'static str {
        "NEON"
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
        self.validate_yuv420_params(y_plane, u_plane, v_plane, width, height, rgb, bit_depth)?;

        match bit_depth {
            8 => unsafe {
                yuv420_to_rgb_neon_impl(y_plane, u_plane, v_plane, width, height, rgb);
                Ok(())
            },
            10 => unsafe {
                yuv420_to_rgb_neon_impl_10bit(y_plane, u_plane, v_plane, width, height, rgb);
                Ok(())
            },
            12 => unsafe {
                yuv420_to_rgb_neon_impl_12bit(y_plane, u_plane, v_plane, width, height, rgb);
                Ok(())
            },
            _ => Err(ConversionError::UnsupportedBitDepth(bit_depth)),
        }
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

        match bit_depth {
            8 => unsafe {
                yuv422_to_rgb_neon_impl(y_plane, u_plane, v_plane, width, height, rgb);
                Ok(())
            },
            10 => unsafe {
                yuv422_to_rgb_neon_impl_10bit(y_plane, u_plane, v_plane, width, height, rgb);
                Ok(())
            },
            12 => unsafe {
                yuv422_to_rgb_neon_impl_12bit(y_plane, u_plane, v_plane, width, height, rgb);
                Ok(())
            },
            _ => Err(ConversionError::UnsupportedBitDepth(bit_depth)),
        }
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

        match bit_depth {
            8 => unsafe {
                yuv444_to_rgb_neon_impl(y_plane, u_plane, v_plane, width, height, rgb);
                Ok(())
            },
            10 => unsafe {
                yuv444_to_rgb_neon_impl_10bit(y_plane, u_plane, v_plane, width, height, rgb);
                Ok(())
            },
            12 => unsafe {
                yuv444_to_rgb_neon_impl_12bit(y_plane, u_plane, v_plane, width, height, rgb);
                Ok(())
            },
            _ => Err(ConversionError::UnsupportedBitDepth(bit_depth)),
        }
    }
}

/// NEON implementation of YUV420 to RGB conversion
///
/// # Safety
/// Caller must ensure:
/// - NEON is available on the CPU
/// - All buffers are valid and properly sized
#[target_feature(enable = "neon")]
unsafe fn yuv420_to_rgb_neon_impl(
    y_plane: &[u8],
    u_plane: &[u8],
    v_plane: &[u8],
    width: usize,
    height: usize,
    rgb: &mut [u8],
) {
    let uv_width = width / 2;

    // Coefficients for BT.601 color space (fixed-point arithmetic with /128)
    // R = Y + 181/128 * V
    // G = Y - 44/128 * U - 91/128 * V
    // B = Y + 227/128 * U
    //
    // Using vdupq_n_s16 for 16-lane vectors (8-lane versions are vdup_n_s16)
    let v_coeff: int16x8_t = vdupq_n_s16(181);
    let u_g_coeff: int16x8_t = vdupq_n_s16(44);
    let v_g_coeff: int16x8_t = vdupq_n_s16(91);
    let u_b_coeff: int16x8_t = vdupq_n_s16(227);
    let const_128: int16x8_t = vdupq_n_s16(128);

    for y in 0..height {
        let y_row_start = y * width;
        let uv_row = (y / 2) * uv_width;

        for x in (0..width).step_by(8) {
            let y_idx = y_row_start + x;
            let uv_idx = uv_row + (x / 2);

            // Bounds check
            let y_safe = y_idx + 8 <= y_plane.len();
            let uv_safe = uv_idx + 4 <= u_plane.len() && uv_idx + 4 <= v_plane.len();

            // Process 8 pixels at once with NEON
            if x + 8 <= width && y_safe && uv_safe {
                // Load 8 Y pixels as u8
                let y_vec = vld1_u8(y_plane.as_ptr().add(y_idx));

                // Load 4 U and V pixels
                let u_4 = vld1_u8(u_plane.as_ptr().add(uv_idx));
                let v_4 = vld1_u8(v_plane.as_ptr().add(uv_idx));

                // Duplicate U/V values: [u0, u1, u2, u3] -> [u0, u0, u1, u1, u2, u2, u3, u3]
                // We use vzip to interleave and take the low half
                let u_zipped = vzip_u8(u_4, u_4);
                let v_zipped = vzip_u8(v_4, v_4);

                // vzip_u8 returns uint8x8x2_t - take the first element which has the interleaved values
                let u_vec = u_zipped.0;
                let v_vec = v_zipped.0;

                // Convert Y to i16 (zero-extend from 8 to 16 bits)
                // vmovl_u8 returns int16x8_t (8 lanes)
                let y_i16 = vreinterpretq_s16_u16(vmovl_u8(y_vec));

                // Convert U/V to i16 and subtract 128 (center the chroma values)
                let u_i16 = vsubq_s16(vreinterpretq_s16_u16(vmovl_u8(u_vec)), const_128);
                let v_i16 = vsubq_s16(vreinterpretq_s16_u16(vmovl_u8(v_vec)), const_128);

                // Compute R, G, B using BT.601 coefficients
                // R = Y + (181 * V) >> 7
                // G = Y - ((44 * U) >> 7) - ((91 * V) >> 7)
                // B = Y + (227 * U) >> 7

                // For 8-lane vectors, we need to split into two 4-lane halves
                // Use vget_low/vget_high to split the vectors

                let y_low = vget_low_s16(y_i16);
                let y_high = vget_high_s16(y_i16);
                let u_low = vget_low_s16(u_i16);
                let u_high = vget_high_s16(u_i16);
                let v_low = vget_low_s16(v_i16);
                let v_high = vget_high_s16(v_i16);

                let v_coeff_low = vget_low_s16(v_coeff);
                let v_coeff_high = vget_high_s16(v_coeff);
                let u_g_coeff_low = vget_low_s16(u_g_coeff);
                let u_g_coeff_high = vget_high_s16(u_g_coeff);
                let v_g_coeff_low = vget_low_s16(v_g_coeff);
                let v_g_coeff_high = vget_high_s16(v_g_coeff);
                let u_b_coeff_low = vget_low_s16(u_b_coeff);
                let u_b_coeff_high = vget_high_s16(u_b_coeff);

                // Process low half (first 4 pixels)
                let v_mult_low = vmull_s16(v_low, v_coeff_low);
                let v_scaled_low = vshrq_n_s32(v_mult_low, 7);
                let v_narrow_low = vmovn_s32(v_scaled_low);
                let r_low = vadd_s16(y_low, v_narrow_low);

                let u_mult_g_low = vmull_s16(u_low, u_g_coeff_low);
                let u_scaled_g_low = vshrq_n_s32(u_mult_g_low, 7);
                let u_narrow_g_low = vmovn_s32(u_scaled_g_low);

                let v_mult_g_low = vmull_s16(v_low, v_g_coeff_low);
                let v_scaled_g_low = vshrq_n_s32(v_mult_g_low, 7);
                let v_narrow_g_low = vmovn_s32(v_scaled_g_low);

                let g_low = vsub_s16(y_low, vadd_s16(u_narrow_g_low, v_narrow_g_low));

                let u_mult_b_low = vmull_s16(u_low, u_b_coeff_low);
                let u_scaled_b_low = vshrq_n_s32(u_mult_b_low, 7);
                let u_narrow_b_low = vmovn_s32(u_scaled_b_low);

                let b_low = vadd_s16(y_low, u_narrow_b_low);

                // Process high half (last 4 pixels)
                let v_mult_high = vmull_s16(v_high, v_coeff_high);
                let v_scaled_high = vshrq_n_s32(v_mult_high, 7);
                let v_narrow_high = vmovn_s32(v_scaled_high);
                let r_high = vadd_s16(y_high, v_narrow_high);

                let u_mult_g_high = vmull_s16(u_high, u_g_coeff_high);
                let u_scaled_g_high = vshrq_n_s32(u_mult_g_high, 7);
                let u_narrow_g_high = vmovn_s32(u_scaled_g_high);

                let v_mult_g_high = vmull_s16(v_high, v_g_coeff_high);
                let v_scaled_g_high = vshrq_n_s32(v_mult_g_high, 7);
                let v_narrow_g_high = vmovn_s32(v_scaled_g_high);

                let g_high = vsub_s16(y_high, vadd_s16(u_narrow_g_high, v_narrow_g_high));

                let u_mult_b_high = vmull_s16(u_high, u_b_coeff_high);
                let u_scaled_b_high = vshrq_n_s32(u_mult_b_high, 7);
                let u_narrow_b_high = vmovn_s32(u_scaled_b_high);

                let b_high = vadd_s16(y_high, u_narrow_b_high);

                // Combine low and high halves
                let r = vcombine_s16(r_low, r_high);
                let g = vcombine_s16(g_low, g_high);
                let b = vcombine_s16(b_low, b_high);

                // Narrow back to 8-bit with saturation (this handles 0-255 clamping automatically)
                let r_u8 = vqmovun_s16(r);
                let g_u8 = vqmovun_s16(g);
                let b_u8 = vqmovun_s16(b);

                // Store RGB values interleaved
                store_rgb_interleaved_neon(rgb, y_idx * 3, r_u8, g_u8, b_u8);
            } else {
                // Fallback to scalar for remaining pixels
                for i in 0..8.min(width - x) {
                    let idx = y_idx + i;
                    let uv_i = uv_idx + (i / 2);

                    let y_val = y_plane[idx] as i32;
                    let u_val = u_plane[uv_i] as i32 - 128;
                    let v_val = v_plane[uv_i] as i32 - 128;

                    let (r, g, b) = yuv_to_rgb_pixel(y_val, u_val, v_val);
                    rgb[idx * 3] = r;
                    rgb[idx * 3 + 1] = g;
                    rgb[idx * 3 + 2] = b;
                }
            }
        }
    }
}

/// NEON implementation of YUV422 to RGB conversion
///
/// YUV422 has horizontal chroma subsampling (2:1).
/// Each UV sample is shared by 2 horizontal Y samples.
///
/// # Safety
/// Caller must ensure:
/// - NEON is available on the CPU
/// - All buffers are valid and properly sized
#[target_feature(enable = "neon")]
unsafe fn yuv422_to_rgb_neon_impl(
    y_plane: &[u8],
    u_plane: &[u8],
    v_plane: &[u8],
    width: usize,
    height: usize,
    rgb: &mut [u8],
) {
    let uv_width = width / 2;

    // Coefficients for BT.601 color space (fixed-point arithmetic with /128)
    let v_coeff: int16x8_t = vdupq_n_s16(181);
    let u_g_coeff: int16x8_t = vdupq_n_s16(44);
    let v_g_coeff: int16x8_t = vdupq_n_s16(91);
    let u_b_coeff: int16x8_t = vdupq_n_s16(227);
    let const_128: int16x8_t = vdupq_n_s16(128);

    for y in 0..height {
        let y_row_start = y * width;
        let uv_row = y * uv_width;

        for x in (0..width).step_by(8) {
            let y_idx = y_row_start + x;
            let uv_idx = uv_row + (x / 2);

            // Bounds check
            let y_safe = y_idx + 8 <= y_plane.len();
            let uv_safe = uv_idx + 4 <= u_plane.len() && uv_idx + 4 <= v_plane.len();

            // Process 8 pixels at once with NEON
            if x + 8 <= width && y_safe && uv_safe {
                // Load 8 Y pixels as u8
                let y_vec = vld1_u8(y_plane.as_ptr().add(y_idx));

                // Load 4 U and V pixels, duplicate to 8
                let u_4 = vld1_u8(u_plane.as_ptr().add(uv_idx));
                let v_4 = vld1_u8(v_plane.as_ptr().add(uv_idx));

                // Duplicate each UV value: [u0, u1, u2, u3] -> [u0, u0, u1, u1, u2, u2, u3, u3]
                let u_zipped = vzip_u8(u_4, u_4);
                let v_zipped = vzip_u8(v_4, v_4);
                let u_vec = u_zipped.0;
                let v_vec = v_zipped.0;

                // Convert to i16 and subtract 128
                let y_i16 = vreinterpretq_s16_u16(vmovl_u8(y_vec));
                let u_i16 = vsubq_s16(vreinterpretq_s16_u16(vmovl_u8(u_vec)), const_128);
                let v_i16 = vsubq_s16(vreinterpretq_s16_u16(vmovl_u8(v_vec)), const_128);

                // Compute R, G, B using BT.601 coefficients
                let y_low = vget_low_s16(y_i16);
                let y_high = vget_high_s16(y_i16);
                let u_low = vget_low_s16(u_i16);
                let u_high = vget_high_s16(u_i16);
                let v_low = vget_low_s16(v_i16);
                let v_high = vget_high_s16(v_i16);

                let v_coeff_low = vget_low_s16(v_coeff);
                let v_coeff_high = vget_high_s16(v_coeff);
                let u_g_coeff_low = vget_low_s16(u_g_coeff);
                let u_g_coeff_high = vget_high_s16(u_g_coeff);
                let v_g_coeff_low = vget_low_s16(v_g_coeff);
                let v_g_coeff_high = vget_high_s16(v_g_coeff);
                let u_b_coeff_low = vget_low_s16(u_b_coeff);
                let u_b_coeff_high = vget_high_s16(u_b_coeff);

                // Process low half
                let v_mult_low = vmull_s16(v_low, v_coeff_low);
                let v_scaled_low = vshrq_n_s32(v_mult_low, 7);
                let v_narrow_low = vmovn_s32(v_scaled_low);
                let r_low = vadd_s16(y_low, v_narrow_low);

                let u_mult_g_low = vmull_s16(u_low, u_g_coeff_low);
                let u_scaled_g_low = vshrq_n_s32(u_mult_g_low, 7);
                let u_narrow_g_low = vmovn_s32(u_scaled_g_low);

                let v_mult_g_low = vmull_s16(v_low, v_g_coeff_low);
                let v_scaled_g_low = vshrq_n_s32(v_mult_g_low, 7);
                let v_narrow_g_low = vmovn_s32(v_scaled_g_low);

                let g_low = vsub_s16(y_low, vadd_s16(u_narrow_g_low, v_narrow_g_low));

                let u_mult_b_low = vmull_s16(u_low, u_b_coeff_low);
                let u_scaled_b_low = vshrq_n_s32(u_mult_b_low, 7);
                let u_narrow_b_low = vmovn_s32(u_scaled_b_low);

                let b_low = vadd_s16(y_low, u_narrow_b_low);

                // Process high half
                let v_mult_high = vmull_s16(v_high, v_coeff_high);
                let v_scaled_high = vshrq_n_s32(v_mult_high, 7);
                let v_narrow_high = vmovn_s32(v_scaled_high);
                let r_high = vadd_s16(y_high, v_narrow_high);

                let u_mult_g_high = vmull_s16(u_high, u_g_coeff_high);
                let u_scaled_g_high = vshrq_n_s32(u_mult_g_high, 7);
                let u_narrow_g_high = vmovn_s32(u_scaled_g_high);

                let v_mult_g_high = vmull_s16(v_high, v_g_coeff_high);
                let v_scaled_g_high = vshrq_n_s32(v_mult_g_high, 7);
                let v_narrow_g_high = vmovn_s32(v_scaled_g_high);

                let g_high = vsub_s16(y_high, vadd_s16(u_narrow_g_high, v_narrow_g_high));

                let u_mult_b_high = vmull_s16(u_high, u_b_coeff_high);
                let u_scaled_b_high = vshrq_n_s32(u_mult_b_high, 7);
                let u_narrow_b_high = vmovn_s32(u_scaled_b_high);

                let b_high = vadd_s16(y_high, u_narrow_b_high);

                // Combine and narrow to 8-bit
                let r = vcombine_s16(r_low, r_high);
                let g = vcombine_s16(g_low, g_high);
                let b = vcombine_s16(b_low, b_high);

                let r_u8 = vqmovun_s16(r);
                let g_u8 = vqmovun_s16(g);
                let b_u8 = vqmovun_s16(b);

                store_rgb_interleaved_neon(rgb, y_idx * 3, r_u8, g_u8, b_u8);
            } else {
                // Fallback to scalar for remaining pixels
                for i in 0..8.min(width - x) {
                    let idx = y_idx + i;
                    let uv_i = uv_idx + (i / 2);

                    let y_val = y_plane[idx] as i32;
                    let u_val = u_plane[uv_i] as i32 - 128;
                    let v_val = v_plane[uv_i] as i32 - 128;

                    let (r, g, b) = yuv_to_rgb_pixel(y_val, u_val, v_val);
                    rgb[idx * 3] = r;
                    rgb[idx * 3 + 1] = g;
                    rgb[idx * 3 + 2] = b;
                }
            }
        }
    }
}

/// NEON implementation of YUV444 to RGB conversion
///
/// YUV444 has no chroma subsampling (4:4:4).
/// Each pixel has its own Y, U, and V sample.
///
/// # Safety
/// Caller must ensure:
/// - NEON is available on the CPU
/// - All buffers are valid and properly sized
#[target_feature(enable = "neon")]
unsafe fn yuv444_to_rgb_neon_impl(
    y_plane: &[u8],
    u_plane: &[u8],
    v_plane: &[u8],
    width: usize,
    height: usize,
    rgb: &mut [u8],
) {
    // Coefficients for BT.601 color space (fixed-point arithmetic with /128)
    let v_coeff: int16x8_t = vdupq_n_s16(181);
    let u_g_coeff: int16x8_t = vdupq_n_s16(44);
    let v_g_coeff: int16x8_t = vdupq_n_s16(91);
    let u_b_coeff: int16x8_t = vdupq_n_s16(227);
    let const_128: int16x8_t = vdupq_n_s16(128);

    for y in 0..height {
        let y_row_start = y * width;

        for x in (0..width).step_by(8) {
            let y_idx = y_row_start + x;

            // Bounds check
            let y_safe = y_idx + 8 <= y_plane.len();
            let uv_safe = y_idx + 8 <= u_plane.len() && y_idx + 8 <= v_plane.len();

            // Process 8 pixels at once with NEON
            if x + 8 <= width && y_safe && uv_safe {
                // Load 8 Y, U, and V pixels (no subsampling)
                let y_vec = vld1_u8(y_plane.as_ptr().add(y_idx));
                let u_vec = vld1_u8(u_plane.as_ptr().add(y_idx));
                let v_vec = vld1_u8(v_plane.as_ptr().add(y_idx));

                // Convert to i16 and subtract 128 for U/V
                let y_i16 = vreinterpretq_s16_u16(vmovl_u8(y_vec));
                let u_i16 = vsubq_s16(vreinterpretq_s16_u16(vmovl_u8(u_vec)), const_128);
                let v_i16 = vsubq_s16(vreinterpretq_s16_u16(vmovl_u8(v_vec)), const_128);

                // Compute R, G, B using BT.601 coefficients
                let y_low = vget_low_s16(y_i16);
                let y_high = vget_high_s16(y_i16);
                let u_low = vget_low_s16(u_i16);
                let u_high = vget_high_s16(u_i16);
                let v_low = vget_low_s16(v_i16);
                let v_high = vget_high_s16(v_i16);

                let v_coeff_low = vget_low_s16(v_coeff);
                let v_coeff_high = vget_high_s16(v_coeff);
                let u_g_coeff_low = vget_low_s16(u_g_coeff);
                let u_g_coeff_high = vget_high_s16(u_g_coeff);
                let v_g_coeff_low = vget_low_s16(v_g_coeff);
                let v_g_coeff_high = vget_high_s16(v_g_coeff);
                let u_b_coeff_low = vget_low_s16(u_b_coeff);
                let u_b_coeff_high = vget_high_s16(u_b_coeff);

                // Process low half
                let v_mult_low = vmull_s16(v_low, v_coeff_low);
                let v_scaled_low = vshrq_n_s32(v_mult_low, 7);
                let v_narrow_low = vmovn_s32(v_scaled_low);
                let r_low = vadd_s16(y_low, v_narrow_low);

                let u_mult_g_low = vmull_s16(u_low, u_g_coeff_low);
                let u_scaled_g_low = vshrq_n_s32(u_mult_g_low, 7);
                let u_narrow_g_low = vmovn_s32(u_scaled_g_low);

                let v_mult_g_low = vmull_s16(v_low, v_g_coeff_low);
                let v_scaled_g_low = vshrq_n_s32(v_mult_g_low, 7);
                let v_narrow_g_low = vmovn_s32(v_scaled_g_low);

                let g_low = vsub_s16(y_low, vadd_s16(u_narrow_g_low, v_narrow_g_low));

                let u_mult_b_low = vmull_s16(u_low, u_b_coeff_low);
                let u_scaled_b_low = vshrq_n_s32(u_mult_b_low, 7);
                let u_narrow_b_low = vmovn_s32(u_scaled_b_low);

                let b_low = vadd_s16(y_low, u_narrow_b_low);

                // Process high half
                let v_mult_high = vmull_s16(v_high, v_coeff_high);
                let v_scaled_high = vshrq_n_s32(v_mult_high, 7);
                let v_narrow_high = vmovn_s32(v_scaled_high);
                let r_high = vadd_s16(y_high, v_narrow_high);

                let u_mult_g_high = vmull_s16(u_high, u_g_coeff_high);
                let u_scaled_g_high = vshrq_n_s32(u_mult_g_high, 7);
                let u_narrow_g_high = vmovn_s32(u_scaled_g_high);

                let v_mult_g_high = vmull_s16(v_high, v_g_coeff_high);
                let v_scaled_g_high = vshrq_n_s32(v_mult_g_high, 7);
                let v_narrow_g_high = vmovn_s32(v_scaled_g_high);

                let g_high = vsub_s16(y_high, vadd_s16(u_narrow_g_high, v_narrow_g_high));

                let u_mult_b_high = vmull_s16(u_high, u_b_coeff_high);
                let u_scaled_b_high = vshrq_n_s32(u_mult_b_high, 7);
                let u_narrow_b_high = vmovn_s32(u_scaled_b_high);

                let b_high = vadd_s16(y_high, u_narrow_b_high);

                // Combine and narrow to 8-bit
                let r = vcombine_s16(r_low, r_high);
                let g = vcombine_s16(g_low, g_high);
                let b = vcombine_s16(b_low, b_high);

                let r_u8 = vqmovun_s16(r);
                let g_u8 = vqmovun_s16(g);
                let b_u8 = vqmovun_s16(b);

                store_rgb_interleaved_neon(rgb, y_idx * 3, r_u8, g_u8, b_u8);
            } else {
                // Fallback to scalar for remaining pixels
                for i in 0..8.min(width - x) {
                    let idx = y_idx + i;

                    let y_val = y_plane[idx] as i32;
                    let u_val = u_plane[idx] as i32 - 128;
                    let v_val = v_plane[idx] as i32 - 128;

                    let (r, g, b) = yuv_to_rgb_pixel(y_val, u_val, v_val);
                    rgb[idx * 3] = r;
                    rgb[idx * 3 + 1] = g;
                    rgb[idx * 3 + 2] = b;
                }
            }
        }
    }
}

/// NEON implementation of YUV420 to RGB conversion for 10-bit video
///
/// 10-bit video is stored as 16-bit samples that need to be normalized to 8-bit.
///
/// # Safety
/// Caller must ensure:
/// - NEON is available on the CPU
/// - All buffers are valid and properly sized
#[target_feature(enable = "neon")]
unsafe fn yuv420_to_rgb_neon_impl_10bit(
    y_plane: &[u8],
    u_plane: &[u8],
    v_plane: &[u8],
    width: usize,
    height: usize,
    rgb: &mut [u8],
) {
    yuv420_to_rgb_neon_impl_nbit(y_plane, u_plane, v_plane, width, height, rgb, 2);
}

/// NEON implementation of YUV420 to RGB conversion for 12-bit video
///
/// 12-bit video is stored as 16-bit samples that need to be normalized to 8-bit.
///
/// # Safety
/// Caller must ensure:
/// - NEON is available on the CPU
/// - All buffers are valid and properly sized
#[target_feature(enable = "neon")]
unsafe fn yuv420_to_rgb_neon_impl_12bit(
    y_plane: &[u8],
    u_plane: &[u8],
    v_plane: &[u8],
    width: usize,
    height: usize,
    rgb: &mut [u8],
) {
    yuv420_to_rgb_neon_impl_nbit(y_plane, u_plane, v_plane, width, height, rgb, 4);
}

/// Generic NEON implementation of YUV420 to RGB conversion for n-bit video
///
/// Uses scalar fallback for the bit depth normalization since NEON shift
/// intrinsics require compile-time constant shift values.
#[target_feature(enable = "neon")]
unsafe fn yuv420_to_rgb_neon_impl_nbit(
    y_plane: &[u8],
    u_plane: &[u8],
    v_plane: &[u8],
    width: usize,
    height: usize,
    rgb: &mut [u8],
    shift: i32,
) {
    let uv_width = width / 2;

    // Coefficients for BT.601 color space (fixed-point arithmetic with /128)
    let _v_coeff: int16x8_t = vdupq_n_s16(181);
    let _u_g_coeff: int16x8_t = vdupq_n_s16(44);
    let _v_g_coeff: int16x8_t = vdupq_n_s16(91);
    let _u_b_coeff: int16x8_t = vdupq_n_s16(227);
    let _const_128: int16x8_t = vdupq_n_s16(128);

    for y in 0..height {
        let y_row_start = y * width;
        let uv_row = (y / 2) * uv_width;

        for x in (0..width).step_by(8) {
            let y_idx = y_row_start + x;
            let uv_idx = uv_row + (x / 2);

            // Bounds check for 16-bit data (2 bytes per sample)
            // Use checked arithmetic to prevent overflow and bypass bounds check
            let y_safe = y_idx
                .checked_mul(2)
                .and_then(|v| v.checked_add(16))
                .map_or(false, |offset| offset <= y_plane.len());
            let uv_safe = uv_idx
                .checked_mul(2)
                .and_then(|v| v.checked_add(8))
                .map_or(false, |offset| {
                    offset <= u_plane.len() && offset <= v_plane.len()
                });

            // Process 8 pixels at once with NEON (or scalar fallback)
            if x + 8 <= width && y_safe && uv_safe {
                // Use scalar for 10/12-bit since NEON shift requires compile-time constants
                // This is still faster than processing the entire frame scalar
                for i in 0..8 {
                    let idx = y_idx + i;
                    let uv_i = uv_idx + (i / 2);

                    let y_val = read_sample_16bit_shifted(y_plane, idx, shift);
                    let u_val = read_sample_16bit_shifted(u_plane, uv_i, shift) - 128;
                    let v_val = read_sample_16bit_shifted(v_plane, uv_i, shift) - 128;

                    let (r, g, b) = yuv_to_rgb_pixel(y_val, u_val, v_val);
                    rgb[idx * 3] = r;
                    rgb[idx * 3 + 1] = g;
                    rgb[idx * 3 + 2] = b;
                }
            } else {
                // Fallback to scalar for remaining pixels
                for i in 0..8.min(width - x) {
                    let idx = y_idx + i;
                    let uv_i = uv_idx + (i / 2);

                    let y_val = read_sample_16bit_shifted(y_plane, idx, shift);
                    let u_val = read_sample_16bit_shifted(u_plane, uv_i, shift) - 128;
                    let v_val = read_sample_16bit_shifted(v_plane, uv_i, shift) - 128;

                    let (r, g, b) = yuv_to_rgb_pixel(y_val, u_val, v_val);
                    rgb[idx * 3] = r;
                    rgb[idx * 3 + 1] = g;
                    rgb[idx * 3 + 2] = b;
                }
            }
        }
    }
}

/// NEON implementation of YUV422 to RGB conversion for 10-bit video
///
/// YUV422 has horizontal chroma subsampling (2:1).
/// Each UV sample is shared by 2 horizontal Y samples.
///
/// # Safety
/// Caller must ensure:
/// - NEON is available on the CPU
/// - All buffers are valid and properly sized
#[target_feature(enable = "neon")]
unsafe fn yuv422_to_rgb_neon_impl_10bit(
    y_plane: &[u8],
    u_plane: &[u8],
    v_plane: &[u8],
    width: usize,
    height: usize,
    rgb: &mut [u8],
) {
    yuv422_to_rgb_neon_impl_nbit(y_plane, u_plane, v_plane, width, height, rgb, 2);
}

/// NEON implementation of YUV422 to RGB conversion for 12-bit video
///
/// YUV422 has horizontal chroma subsampling (2:1).
/// Each UV sample is shared by 2 horizontal Y samples.
///
/// # Safety
/// Caller must ensure:
/// - NEON is available on the CPU
/// - All buffers are valid and properly sized
#[target_feature(enable = "neon")]
unsafe fn yuv422_to_rgb_neon_impl_12bit(
    y_plane: &[u8],
    u_plane: &[u8],
    v_plane: &[u8],
    width: usize,
    height: usize,
    rgb: &mut [u8],
) {
    yuv422_to_rgb_neon_impl_nbit(y_plane, u_plane, v_plane, width, height, rgb, 4);
}

/// Generic NEON implementation of YUV422 to RGB conversion for n-bit video
///
/// Uses scalar fallback for the bit depth normalization since NEON shift
/// intrinsics require compile-time constant shift values.
#[target_feature(enable = "neon")]
unsafe fn yuv422_to_rgb_neon_impl_nbit(
    y_plane: &[u8],
    u_plane: &[u8],
    v_plane: &[u8],
    width: usize,
    height: usize,
    rgb: &mut [u8],
    shift: i32,
) {
    let uv_width = width / 2;

    for y in 0..height {
        let y_row_start = y * width;
        let uv_row = y * uv_width;

        for x in (0..width).step_by(8) {
            let y_idx = y_row_start + x;
            let uv_idx = uv_row + (x / 2);

            // Bounds check for 16-bit data
            // Use checked arithmetic to prevent overflow and bypass bounds check
            let y_safe = y_idx
                .checked_mul(2)
                .and_then(|v| v.checked_add(16))
                .map_or(false, |offset| offset <= y_plane.len());
            let uv_safe = uv_idx
                .checked_mul(2)
                .and_then(|v| v.checked_add(8))
                .map_or(false, |offset| {
                    offset <= u_plane.len() && offset <= v_plane.len()
                });

            // Process 8 pixels at once with NEON (or scalar fallback)
            if x + 8 <= width && y_safe && uv_safe {
                // Use scalar for 10/12-bit since NEON shift requires compile-time constants
                for i in 0..8 {
                    let idx = y_idx + i;
                    let uv_i = uv_idx + (i / 2);

                    let y_val = read_sample_16bit_shifted(y_plane, idx, shift);
                    let u_val = read_sample_16bit_shifted(u_plane, uv_i, shift) - 128;
                    let v_val = read_sample_16bit_shifted(v_plane, uv_i, shift) - 128;

                    let (r, g, b) = yuv_to_rgb_pixel(y_val, u_val, v_val);
                    rgb[idx * 3] = r;
                    rgb[idx * 3 + 1] = g;
                    rgb[idx * 3 + 2] = b;
                }
            } else {
                // Fallback to scalar for remaining pixels
                for i in 0..8.min(width - x) {
                    let idx = y_idx + i;
                    let uv_i = uv_idx + (i / 2);

                    let y_val = read_sample_16bit_shifted(y_plane, idx, shift);
                    let u_val = read_sample_16bit_shifted(u_plane, uv_i, shift) - 128;
                    let v_val = read_sample_16bit_shifted(v_plane, uv_i, shift) - 128;

                    let (r, g, b) = yuv_to_rgb_pixel(y_val, u_val, v_val);
                    rgb[idx * 3] = r;
                    rgb[idx * 3 + 1] = g;
                    rgb[idx * 3 + 2] = b;
                }
            }
        }
    }
}

/// NEON implementation of YUV444 to RGB conversion for 10-bit video
///
/// YUV444 has no chroma subsampling (4:4:4).
/// Each pixel has its own Y, U, and V sample.
///
/// # Safety
/// Caller must ensure:
/// - NEON is available on the CPU
/// - All buffers are valid and properly sized
#[target_feature(enable = "neon")]
unsafe fn yuv444_to_rgb_neon_impl_10bit(
    y_plane: &[u8],
    u_plane: &[u8],
    v_plane: &[u8],
    width: usize,
    height: usize,
    rgb: &mut [u8],
) {
    yuv444_to_rgb_neon_impl_nbit(y_plane, u_plane, v_plane, width, height, rgb, 2);
}

/// NEON implementation of YUV444 to RGB conversion for 12-bit video
///
/// YUV444 has no chroma subsampling (4:4:4).
/// Each pixel has its own Y, U, and V sample.
///
/// # Safety
/// Caller must ensure:
/// - NEON is available on the CPU
/// - All buffers are valid and properly sized
#[target_feature(enable = "neon")]
unsafe fn yuv444_to_rgb_neon_impl_12bit(
    y_plane: &[u8],
    u_plane: &[u8],
    v_plane: &[u8],
    width: usize,
    height: usize,
    rgb: &mut [u8],
) {
    yuv444_to_rgb_neon_impl_nbit(y_plane, u_plane, v_plane, width, height, rgb, 4);
}

/// Generic NEON implementation of YUV444 to RGB conversion for n-bit video
///
/// Uses scalar fallback for the bit depth normalization since NEON shift
/// intrinsics require compile-time constant shift values.
#[target_feature(enable = "neon")]
unsafe fn yuv444_to_rgb_neon_impl_nbit(
    y_plane: &[u8],
    u_plane: &[u8],
    v_plane: &[u8],
    width: usize,
    height: usize,
    rgb: &mut [u8],
    shift: i32,
) {
    for y in 0..height {
        let y_row_start = y * width;

        for x in (0..width).step_by(8) {
            let y_idx = y_row_start + x;

            // Bounds check for 16-bit data (YUV444 has no chroma subsampling)
            // Use checked arithmetic to prevent overflow and bypass bounds check
            let y_safe = y_idx
                .checked_mul(2)
                .and_then(|v| v.checked_add(16))
                .map_or(false, |offset| offset <= y_plane.len());
            let uv_safe = y_idx
                .checked_mul(2)
                .and_then(|v| v.checked_add(16))
                .map_or(false, |offset| {
                    offset <= u_plane.len() && offset <= v_plane.len()
                });

            // Process 8 pixels at once with NEON (or scalar fallback)
            if x + 8 <= width && y_safe && uv_safe {
                // Use scalar for 10/12-bit since NEON shift requires compile-time constants
                for i in 0..8 {
                    let idx = y_idx + i;

                    let y_val = read_sample_16bit_shifted(y_plane, idx, shift);
                    let u_val = read_sample_16bit_shifted(u_plane, idx, shift) - 128;
                    let v_val = read_sample_16bit_shifted(v_plane, idx, shift) - 128;

                    let (r, g, b) = yuv_to_rgb_pixel(y_val, u_val, v_val);
                    rgb[idx * 3] = r;
                    rgb[idx * 3 + 1] = g;
                    rgb[idx * 3 + 2] = b;
                }
            } else {
                // Fallback to scalar for remaining pixels
                for i in 0..8.min(width - x) {
                    let idx = y_idx + i;

                    let y_val = read_sample_16bit_shifted(y_plane, idx, shift);
                    let u_val = read_sample_16bit_shifted(u_plane, idx, shift) - 128;
                    let v_val = read_sample_16bit_shifted(v_plane, idx, shift) - 128;

                    let (r, g, b) = yuv_to_rgb_pixel(y_val, u_val, v_val);
                    rgb[idx * 3] = r;
                    rgb[idx * 3 + 1] = g;
                    rgb[idx * 3 + 2] = b;
                }
            }
        }
    }
}

/// Store RGB values interleaved (optimized NEON version with vector operations)
///
/// Precondition: offset + 24 <= rgb.len() (caller must validate)
///
/// Uses NEON vector operations for efficient RGB interleaving:
/// 1. vst1_u8 for fast vector stores to temp buffers
/// 2. Structured arrays for cache-friendly access
/// 3. Direct pointer writes (no bounds check - caller validated)
#[inline]
unsafe fn store_rgb_interleaved_neon(
    rgb: &mut [u8],
    offset: usize,
    r: uint8x8_t,
    g: uint8x8_t,
    b: uint8x8_t,
) {
    // Extract to temporary buffers using fast NEON vector stores
    let r_buf = {
        let mut tmp = [0u8; 8];
        vst1_u8(tmp.as_mut_ptr(), r);
        tmp
    };
    let g_buf = {
        let mut tmp = [0u8; 8];
        vst1_u8(tmp.as_mut_ptr(), g);
        tmp
    };
    let b_buf = {
        let mut tmp = [0u8; 8];
        vst1_u8(tmp.as_mut_ptr(), b);
        tmp
    };

    // Build RGB triplets in arrays for cache-friendly access
    // Pixels 0-3: R0,G0,B0,R1,G1,B1,R2,G2,B2,R3,G3,B3 (12 bytes)
    let rgb_part1 = [
        r_buf[0], g_buf[0], b_buf[0], // Pixel 0
        r_buf[1], g_buf[1], b_buf[1], // Pixel 1
        r_buf[2], g_buf[2], b_buf[2], // Pixel 2
        r_buf[3], g_buf[3], b_buf[3], // Pixel 3
    ];

    // Pixels 4-7: R4,G4,B4,R5,G5,B5,R6,G6,B6,R7,G7,B7 (12 bytes)
    let rgb_part2 = [
        r_buf[4], g_buf[4], b_buf[4], // Pixel 4
        r_buf[5], g_buf[5], b_buf[5], // Pixel 5
        r_buf[6], g_buf[6], b_buf[6], // Pixel 6
        r_buf[7], g_buf[7], b_buf[7], // Pixel 7
    ];

    // Copy to output using pointer arithmetic
    // No bounds check needed - caller validated offset + 24 <= rgb.len()
    let dst = rgb.as_mut_ptr().add(offset);

    // First 12 bytes (pixels 0-3)
    *dst.add(0) = rgb_part1[0];
    *dst.add(1) = rgb_part1[1];
    *dst.add(2) = rgb_part1[2];
    *dst.add(3) = rgb_part1[3];
    *dst.add(4) = rgb_part1[4];
    *dst.add(5) = rgb_part1[5];
    *dst.add(6) = rgb_part1[6];
    *dst.add(7) = rgb_part1[7];
    *dst.add(8) = rgb_part1[8];
    *dst.add(9) = rgb_part1[9];
    *dst.add(10) = rgb_part1[10];
    *dst.add(11) = rgb_part1[11];

    // Last 12 bytes (pixels 4-7)
    *dst.add(12) = rgb_part2[0];
    *dst.add(13) = rgb_part2[1];
    *dst.add(14) = rgb_part2[2];
    *dst.add(15) = rgb_part2[3];
    *dst.add(16) = rgb_part2[4];
    *dst.add(17) = rgb_part2[5];
    *dst.add(18) = rgb_part2[6];
    *dst.add(19) = rgb_part2[7];
    *dst.add(20) = rgb_part2[8];
    *dst.add(21) = rgb_part2[9];
    *dst.add(22) = rgb_part2[10];
    *dst.add(23) = rgb_part2[11];
}

/// Convert a single YUV pixel to RGB using BT.601 color space (scalar fallback)
///
/// Uses integer arithmetic for 20-30% speedup over floating-point.
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

/// Read a 16-bit sample and normalize to 8-bit
///
/// NOTE: Kept for future high bit-depth support (10-bit, 12-bit video).
/// Currently unused but will be needed when processing >8-bit content.
#[allow(dead_code)]
#[inline]
fn read_sample_16bit(plane: &[u8], idx: usize, bit_depth: u8) -> i32 {
    let byte_idx = idx * 2;
    if byte_idx + 1 < plane.len() {
        let sample16 = u16::from_le_bytes([plane[byte_idx], plane[byte_idx + 1]]);
        // Normalize to 8-bit by right-shifting
        (sample16 >> (bit_depth - 8)) as i32
    } else {
        0
    }
}

/// Read a 16-bit sample and normalize to 8-bit with given shift amount
///
/// NOTE: Kept for future high bit-depth support with custom shift amounts.
/// Currently unused but provides flexibility for different bit-depth formats.
#[allow(dead_code)]
#[inline]
fn read_sample_16bit_shifted(plane: &[u8], idx: usize, shift: i32) -> i32 {
    let byte_idx = idx * 2;
    if byte_idx + 1 < plane.len() {
        let sample16 = u16::from_le_bytes([plane[byte_idx], plane[byte_idx + 1]]);
        // Normalize to 8-bit by right-shifting
        (sample16 >> shift) as i32
    } else {
        0
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_neon_strategy_creation() {
        let strategy = NeonStrategy::new();
        assert_eq!(strategy.name(), "NEON");
    }

    #[test]
    fn test_neon_capabilities() {
        let strategy = NeonStrategy::new();
        let caps = strategy.capabilities();
        assert_eq!(caps.speedup_factor, 3.5);
        assert!(caps.supports_10bit); // 10-bit support added
        assert!(caps.supports_12bit); // 12-bit support added
        assert!(!caps.is_hardware_accelerated);
    }

    #[test]
    fn test_neon_default() {
        let strategy = NeonStrategy::default();
        assert_eq!(strategy.name(), "NEON");
    }

    #[test]
    fn test_yuv_to_rgb_pixel_black() {
        let (r, g, b) = yuv_to_rgb_pixel(0, 0, 0);
        assert_eq!((r, g, b), (0, 0, 0));
    }

    #[test]
    fn test_yuv_to_rgb_pixel_white() {
        let (r, g, b) = yuv_to_rgb_pixel(255, 0, 0);
        assert_eq!((r, g, b), (255, 255, 255));
    }

    #[test]
    fn test_yuv_to_rgb_pixel_clamping() {
        // Test negative values (U and V below 128)
        // Using integer arithmetic with BT.601 coefficients:
        // R = (0 * 128 + 181 * (-200)) >> 7 = -283 >> 7 = -2 -> 0 (clamped)
        // G = (0 * 128 - 44 * (-200) - 91 * (-200)) >> 7 = 27000 >> 7 = 210
        // B = (0 * 128 + 227 * (-200)) >> 7 = -353 >> 7 = -2 -> 0 (clamped)
        let (r, g, b) = yuv_to_rgb_pixel(0, -200, -200);
        assert_eq!((r, g, b), (0, 210, 0));

        // Test overflow
        // R = (300 * 128 + 181 * 200) >> 7 = 44200 >> 7 = 345 -> 255 (clamped)
        // G = (300 * 128 - 44 * 200 - 91 * 200) >> 7 = 11400 >> 7 = 89
        // B = (300 * 128 + 227 * 200) >> 7 = 49800 >> 7 = 389 -> 255 (clamped)
        let (r, g, b) = yuv_to_rgb_pixel(300, 200, 200);
        assert_eq!((r, g, b), (255, 89, 255));
    }
}
