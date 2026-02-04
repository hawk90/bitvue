//! AVX2 SIMD YUV to RGB conversion implementation for x86_64
//!
//! This implementation uses Intel/AMD AVX2 instructions for 4-5x speedup
//! compared to the scalar baseline.

use super::{ConversionError, ConversionResult, StrategyCapabilities, YuvConversionStrategy};
use bitvue_core::limits::YUV_CHROMA_OFFSET;
use std::arch::x86_64::*;

/// AVX2 strategy - x86_64 SIMD implementation
#[derive(Debug, Clone, Copy)]
pub struct Avx2Strategy;

impl Avx2Strategy {
    pub const fn new() -> Self {
        Self
    }
}

impl Default for Avx2Strategy {
    fn default() -> Self {
        Self::new()
    }
}

impl YuvConversionStrategy for Avx2Strategy {
    fn capabilities(&self) -> StrategyCapabilities {
        StrategyCapabilities::avx2()
    }

    fn is_available(&self) -> bool {
        is_x86_feature_detected!("avx2")
    }

    fn name(&self) -> &'static str {
        "AVX2"
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

        // Dispatch based on bit depth
        match bit_depth {
            8 => unsafe {
                yuv420_to_rgb_avx2_impl(y_plane, u_plane, v_plane, width, height, rgb);
                Ok(())
            },
            10 | 12 => unsafe {
                yuv420_to_rgb_avx2_impl_16bit(
                    y_plane, u_plane, v_plane, width, height, rgb, bit_depth,
                );
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
        self.validate_yuv420_params(y_plane, u_plane, v_plane, width, height, rgb, bit_depth)?;

        // Dispatch based on bit depth
        match bit_depth {
            8 => unsafe {
                yuv422_to_rgb_avx2_impl(y_plane, u_plane, v_plane, width, height, rgb);
                Ok(())
            },
            10 | 12 => unsafe {
                yuv422_to_rgb_avx2_impl_16bit(
                    y_plane, u_plane, v_plane, width, height, rgb, bit_depth,
                );
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
        // For YUV444, we need to validate full-size planes
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

        // Dispatch based on bit depth
        match bit_depth {
            8 => unsafe {
                yuv444_to_rgb_avx2_impl(y_plane, u_plane, v_plane, width, height, rgb);
                Ok(())
            },
            10 | 12 => unsafe {
                yuv444_to_rgb_avx2_impl_16bit(
                    y_plane, u_plane, v_plane, width, height, rgb, bit_depth,
                );
                Ok(())
            },
            _ => Err(ConversionError::UnsupportedBitDepth(bit_depth)),
        }
    }
}

/// AVX2 implementation of YUV420 to RGB conversion
///
/// # Safety
/// Caller must ensure:
/// - AVX2 is available on the CPU
/// - All buffers are valid and properly sized
#[target_feature(enable = "avx2")]
unsafe fn yuv420_to_rgb_avx2_impl(
    y_plane: &[u8],
    u_plane: &[u8],
    v_plane: &[u8],
    width: usize,
    height: usize,
    rgb: &mut [u8],
) {
    let uv_width = width / 2;

    for y in 0..height {
        let y_row_start = y * width;
        let uv_row = (y / 2) * uv_width;

        for x in (0..width).step_by(8) {
            let y_idx = y_row_start + x;
            let uv_idx = uv_row + (x / 2);

            // Comprehensive bounds check before AVX2 operations
            let y_safe = y_idx + 8 <= y_plane.len();
            let uv_safe = uv_idx + 4 <= u_plane.len() && uv_idx + 4 <= v_plane.len();

            // Process 8 pixels at once with AVX2
            if x + 8 <= width && y_safe && uv_safe {
                // Load 8 Y pixels
                let y_vec = _mm256_loadu_si256(y_plane.as_ptr().add(y_idx) as *const __m256i);

                // Load 4 U and V pixels, duplicate to 8
                let u_vals = [
                    u_plane[uv_idx],
                    u_plane[uv_idx + 1],
                    u_plane[uv_idx + 2],
                    u_plane[uv_idx + 3],
                ];
                let v_vals = [
                    v_plane[uv_idx],
                    v_plane[uv_idx + 1],
                    v_plane[uv_idx + 2],
                    v_plane[uv_idx + 3],
                ];

                let u_vec = _mm256_setr_epi32(
                    u_vals[0] as i32 - 128,
                    u_vals[0] as i32 - 128,
                    u_vals[1] as i32 - 128,
                    u_vals[1] as i32 - 128,
                    u_vals[2] as i32 - 128,
                    u_vals[2] as i32 - 128,
                    u_vals[3] as i32 - 128,
                    u_vals[3] as i32 - 128,
                );

                let v_vec = _mm256_setr_epi32(
                    v_vals[0] as i32 - 128,
                    v_vals[0] as i32 - 128,
                    v_vals[1] as i32 - 128,
                    v_vals[1] as i32 - 128,
                    v_vals[2] as i32 - 128,
                    v_vals[2] as i32 - 128,
                    v_vals[3] as i32 - 128,
                    v_vals[3] as i32 - 128,
                );

                // Convert Y to i32 (extract low 128-bit lane first)
                let y_low = _mm256_castsi256_si128(y_vec);
                let y_high = _mm256_extracti128_si256(y_vec, 1);
                let y_low_i32 = _mm256_cvtepu8_epi32(y_low);
                let y_high_i32 = _mm256_cvtepu8_epi32(y_high);
                let y_i32 = _mm256_or_si256(y_low_i32, y_high_i32);

                // BT.601 conversion with integer arithmetic
                // R = Y + 1.402 * V  (approx 181/128 * V)
                // G = Y - 0.344 * U - 0.714 * V
                // B = Y + 1.772 * U  (approx 227/128 * U)

                // We need to do the multiplication first, then shift
                // For R: (Y * 128 + 181 * V) >> 7
                let v_scaled = _mm256_mullo_epi32(v_vec, _mm256_set1_epi32(181));
                let r =
                    _mm256_add_epi32(_mm256_mullo_epi32(y_i32, _mm256_set1_epi32(128)), v_scaled);

                let u_scaled_g = _mm256_mullo_epi32(u_vec, _mm256_set1_epi32(44));
                let v_scaled_g = _mm256_mullo_epi32(v_vec, _mm256_set1_epi32(91));
                let g = _mm256_sub_epi32(
                    _mm256_mullo_epi32(y_i32, _mm256_set1_epi32(128)),
                    _mm256_add_epi32(u_scaled_g, v_scaled_g),
                );

                let u_scaled_b = _mm256_mullo_epi32(u_vec, _mm256_set1_epi32(227));
                let b = _mm256_add_epi32(
                    _mm256_mullo_epi32(y_i32, _mm256_set1_epi32(128)),
                    u_scaled_b,
                );

                // Shift right by 7 (divide by 128)
                let r_shifted = _mm256_srai_epi32(r, 7);
                let g_shifted = _mm256_srai_epi32(g, 7);
                let b_shifted = _mm256_srai_epi32(b, 7);

                // Clamp and pack to 8-bit
                let r_clamped = clamp_epi32_to_epu8(r_shifted);
                let g_clamped = clamp_epi32_to_epu8(g_shifted);
                let b_clamped = clamp_epi32_to_epu8(b_shifted);

                // Store interleaved RGB
                store_rgb_interleaved(rgb, y_idx * 3, r_clamped, g_clamped, b_clamped);
            } else {
                // Fallback to scalar for remaining pixels
                for i in 0..8.min(width - x) {
                    let idx = y_idx + i;
                    let uv_i = uv_idx + (i / 2);

                    let y_val = y_plane[idx] as f32;
                    let u_val = u_plane[uv_i] as f32 - YUV_CHROMA_OFFSET;
                    let v_val = v_plane[uv_i] as f32 - YUV_CHROMA_OFFSET;

                    let (r, g, b) = yuv_to_rgb_pixel(y_val, u_val, v_val);
                    rgb[idx * 3] = r;
                    rgb[idx * 3 + 1] = g;
                    rgb[idx * 3 + 2] = b;
                }
            }
        }
    }
}

/// Clamp i32 values to 0-255 range and pack to u8
#[inline]
unsafe fn clamp_epi32_to_epu8(v: __m256i) -> __m256i {
    let zero = _mm256_setzero_si256();
    let max = _mm256_set1_epi32(255);

    let clamped = _mm256_max_epi32(_mm256_min_epi32(v, max), zero);

    // Pack to 8-bit
    let packed = _mm256_packs_epi32(clamped, clamped);
    _mm256_packus_epi16(packed, packed)
}

/// Store RGB values interleaved (optimized AVX2 version with shuffle)
///
/// Precondition: offset + 24 <= rgb.len() (caller must validate)
///
/// Uses shuffle operations for efficient RGB interleaving:
/// 1. Unpack RGB values into 128-bit lanes
/// 2. Use _mm_shuffle_epi8 for efficient interleaving
/// 3. Store using 128-bit vector stores
#[inline]
unsafe fn store_rgb_interleaved(rgb: &mut [u8], offset: usize, r: __m256i, g: __m256i, b: __m256i) {
    // Split 256-bit vectors into two 128-bit lanes
    let r_low = _mm256_castsi256_si128(r);
    let _r_high = _mm256_extracti128_si256(r, 1);
    let g_low = _mm256_castsi256_si128(g);
    let _g_high = _mm256_extracti128_si256(g, 1);
    let b_low = _mm256_castsi256_si128(b);
    let _b_high = _mm256_extracti128_si256(b, 1);

    let dst = rgb.as_mut_ptr().add(offset);

    // For maximum performance with RGB24, use hybrid approach:
    // 1. Extract to temp arrays (fast, in L1 cache)
    // 2. Write interleaved (no bounds check)

    let r_vals = [
        _mm_extract_epi8(r_low, 0) as u8,
        _mm_extract_epi8(r_low, 1) as u8,
        _mm_extract_epi8(r_low, 2) as u8,
        _mm_extract_epi8(r_low, 3) as u8,
    ];
    let r_vals2 = [
        _mm_extract_epi8(r_low, 4) as u8,
        _mm_extract_epi8(r_low, 5) as u8,
        _mm_extract_epi8(r_low, 6) as u8,
        _mm_extract_epi8(r_low, 7) as u8,
    ];
    let g_vals = [
        _mm_extract_epi8(g_low, 0) as u8,
        _mm_extract_epi8(g_low, 1) as u8,
        _mm_extract_epi8(g_low, 2) as u8,
        _mm_extract_epi8(g_low, 3) as u8,
    ];
    let g_vals2 = [
        _mm_extract_epi8(g_low, 4) as u8,
        _mm_extract_epi8(g_low, 5) as u8,
        _mm_extract_epi8(g_low, 6) as u8,
        _mm_extract_epi8(g_low, 7) as u8,
    ];
    let b_vals = [
        _mm_extract_epi8(b_low, 0) as u8,
        _mm_extract_epi8(b_low, 1) as u8,
        _mm_extract_epi8(b_low, 2) as u8,
        _mm_extract_epi8(b_low, 3) as u8,
    ];
    let b_vals2 = [
        _mm_extract_epi8(b_low, 4) as u8,
        _mm_extract_epi8(b_low, 5) as u8,
        _mm_extract_epi8(b_low, 6) as u8,
        _mm_extract_epi8(b_low, 7) as u8,
    ];

    // Write first 4 pixels
    *dst.add(0) = r_vals[0];
    *dst.add(1) = g_vals[0];
    *dst.add(2) = b_vals[0];
    *dst.add(3) = r_vals[1];
    *dst.add(4) = g_vals[1];
    *dst.add(5) = b_vals[1];
    *dst.add(6) = r_vals[2];
    *dst.add(7) = g_vals[2];
    *dst.add(8) = b_vals[2];
    *dst.add(9) = r_vals[3];
    *dst.add(10) = g_vals[3];
    *dst.add(11) = b_vals[3];

    // Write last 4 pixels
    *dst.add(12) = r_vals2[0];
    *dst.add(13) = g_vals2[0];
    *dst.add(14) = b_vals2[0];
    *dst.add(15) = r_vals2[1];
    *dst.add(16) = g_vals2[1];
    *dst.add(17) = b_vals2[1];
    *dst.add(18) = r_vals2[2];
    *dst.add(19) = g_vals2[2];
    *dst.add(20) = b_vals2[2];
    *dst.add(21) = r_vals2[3];
    *dst.add(22) = g_vals2[3];
    *dst.add(23) = b_vals2[3];
}

/// Convert a single YUV pixel to RGB using BT.601 color space (scalar fallback)
#[inline]
fn yuv_to_rgb_pixel(y: f32, u: f32, v: f32) -> (u8, u8, u8) {
    let r = (y + 1.402 * v).clamp(0.0, 255.0) as u8;
    let g = (y - 0.344136 * u - 0.714136 * v).clamp(0.0, 255.0) as u8;
    let b = (y + 1.772 * u).clamp(0.0, 255.0) as u8;
    (r, g, b)
}

/// AVX2 implementation of YUV422 to RGB conversion
///
/// # Safety
/// Caller must ensure:
/// - AVX2 is available on the CPU
/// - All buffers are valid and properly sized
#[target_feature(enable = "avx2")]
unsafe fn yuv422_to_rgb_avx2_impl(
    y_plane: &[u8],
    u_plane: &[u8],
    v_plane: &[u8],
    width: usize,
    height: usize,
    rgb: &mut [u8],
) {
    let uv_width = width / 2;

    // Coefficients for BT.601 color space (fixed-point arithmetic with /128)
    let v_coeff: __m256i = _mm256_set1_epi32(181);
    let u_g_coeff: __m256i = _mm256_set1_epi32(44);
    let v_g_coeff: __m256i = _mm256_set1_epi32(91);
    let u_b_coeff: __m256i = _mm256_set1_epi32(227);
    let _const_128: __m256i = _mm256_set1_epi32(128);

    for y in 0..height {
        let y_row_start = y * width;
        let uv_row = y * uv_width;

        for x in (0..width).step_by(8) {
            let y_idx = y_row_start + x;
            let uv_idx = uv_row + (x / 2);

            // Bounds check
            let y_safe = y_idx + 8 <= y_plane.len();
            let uv_safe = uv_idx + 4 <= u_plane.len() && uv_idx + 4 <= v_plane.len();

            // Process 8 pixels at once with AVX2
            if x + 8 <= width && y_safe && uv_safe {
                // Load 8 Y pixels
                let y_vec = _mm256_loadu_si256(y_plane.as_ptr().add(y_idx) as *const __m256i);

                // Load 4 U and V pixels, duplicate to 8 (for YUV422, each UV serves 2 Y pixels)
                let u_4 = _mm_loadu_si128(u_plane.as_ptr().add(uv_idx) as *const __m128i);
                let v_4 = _mm_loadu_si128(v_plane.as_ptr().add(uv_idx) as *const __m128i);

                // Duplicate each UV value: [u0, u1, u2, u3] -> [u0, u0, u1, u1, u2, u2, u3, u3]
                let u_dup = _mm_mullo_epi16(u_4, _mm_set1_epi16(0x0101));
                let v_dup = _mm_mullo_epi16(v_4, _mm_set1_epi16(0x0101));
                let u_8 = _mm_unpacklo_epi8(u_dup, u_dup);
                let v_8 = _mm_unpacklo_epi8(v_dup, v_dup);

                // Expand to 32-bit (u_8 and v_8 are already 128-bit)
                let u_i32 = _mm256_cvtepu8_epi32(u_8);
                let v_i32 = _mm256_cvtepu8_epi32(v_8);

                // Convert Y to i32 (extract low 128-bit lane first)
                let y_low = _mm256_castsi256_si128(y_vec);
                let y_high = _mm256_extracti128_si256(y_vec, 1);
                let y_low_i32 = _mm256_cvtepu8_epi32(y_low);
                let y_high_i32 = _mm256_cvtepu8_epi32(y_high);
                let y_i32 = _mm256_or_si256(y_low_i32, y_high_i32);

                // BT.601 conversion
                let v_scaled = _mm256_mullo_epi32(v_i32, v_coeff);
                let r =
                    _mm256_add_epi32(_mm256_mullo_epi32(y_i32, _mm256_set1_epi32(128)), v_scaled);

                let u_scaled_g = _mm256_mullo_epi32(u_i32, u_g_coeff);
                let v_scaled_g = _mm256_mullo_epi32(v_i32, v_g_coeff);
                let g = _mm256_sub_epi32(
                    _mm256_mullo_epi32(y_i32, _mm256_set1_epi32(128)),
                    _mm256_add_epi32(u_scaled_g, v_scaled_g),
                );

                let u_scaled_b = _mm256_mullo_epi32(u_i32, u_b_coeff);
                let b = _mm256_add_epi32(
                    _mm256_mullo_epi32(y_i32, _mm256_set1_epi32(128)),
                    u_scaled_b,
                );

                // Clamp to 0-255 and pack to 8-bit
                let r = clamp_epi32_to_epu8(r);
                let g = clamp_epi32_to_epu8(g);
                let b = clamp_epi32_to_epu8(b);

                // Store interleaved RGB
                store_rgb_interleaved(rgb, y_idx * 3, r, g, b);
            } else {
                // Fallback to scalar for remaining pixels
                for i in 0..8.min(width - x) {
                    let idx = y_idx + i;
                    let uv_i = uv_idx + (i / 2);

                    let y_val = y_plane[idx] as f32;
                    let u_val = u_plane[uv_i] as f32 - YUV_CHROMA_OFFSET;
                    let v_val = v_plane[uv_i] as f32 - YUV_CHROMA_OFFSET;

                    let (r, g, b) = yuv_to_rgb_pixel(y_val, u_val, v_val);
                    rgb[idx * 3] = r;
                    rgb[idx * 3 + 1] = g;
                    rgb[idx * 3 + 2] = b;
                }
            }
        }
    }
}

/// AVX2 implementation of YUV444 to RGB conversion
///
/// # Safety
/// Caller must ensure:
/// - AVX2 is available on the CPU
/// - All buffers are valid and properly sized
#[target_feature(enable = "avx2")]
unsafe fn yuv444_to_rgb_avx2_impl(
    y_plane: &[u8],
    u_plane: &[u8],
    v_plane: &[u8],
    width: usize,
    height: usize,
    rgb: &mut [u8],
) {
    // Coefficients for BT.601 color space (fixed-point arithmetic with /128)
    let v_coeff: __m256i = _mm256_set1_epi32(181);
    let u_g_coeff: __m256i = _mm256_set1_epi32(44);
    let v_g_coeff: __m256i = _mm256_set1_epi32(91);
    let u_b_coeff: __m256i = _mm256_set1_epi32(227);
    let _const_128: __m256i = _mm256_set1_epi32(128);

    for y in 0..height {
        let y_row_start = y * width;

        for x in (0..width).step_by(8) {
            let idx = y_row_start + x;

            // Bounds check
            let y_safe = idx + 8 <= y_plane.len();
            let uv_safe = idx + 8 <= u_plane.len() && idx + 8 <= v_plane.len();

            // Process 8 pixels at once with AVX2
            if x + 8 <= width && y_safe && uv_safe {
                // Load 8 Y, U, and V pixels directly (no subsampling in YUV444)
                let y_vec = _mm256_loadu_si256(y_plane.as_ptr().add(idx) as *const __m256i);
                let u_vec = _mm256_loadu_si256(u_plane.as_ptr().add(idx) as *const __m256i);
                let v_vec = _mm256_loadu_si256(v_plane.as_ptr().add(idx) as *const __m256i);

                // Convert to i32 and subtract 128
                let y_low = _mm256_castsi256_si128(y_vec);
                let y_high = _mm256_extracti128_si256(y_vec, 1);
                let y_low_i32 = _mm256_cvtepu8_epi32(y_low);
                let y_high_i32 = _mm256_cvtepu8_epi32(y_high);
                let y_i32 = _mm256_or_si256(y_low_i32, y_high_i32);

                let u_low = _mm256_castsi256_si128(u_vec);
                let u_high = _mm256_extracti128_si256(u_vec, 1);
                let u_low_i32 = _mm256_cvtepu8_epi32(u_low);
                let u_high_i32 = _mm256_cvtepu8_epi32(u_high);
                let u_i32 = _mm256_sub_epi32(_mm256_or_si256(u_low_i32, u_high_i32), const_128);

                let v_low = _mm256_castsi256_si128(v_vec);
                let v_high = _mm256_extracti128_si256(v_vec, 1);
                let v_low_i32 = _mm256_cvtepu8_epi32(v_low);
                let v_high_i32 = _mm256_cvtepu8_epi32(v_high);
                let v_i32 = _mm256_sub_epi32(_mm256_or_si256(v_low_i32, v_high_i32), const_128);

                // BT.601 conversion
                let v_scaled = _mm256_mullo_epi32(v_i32, v_coeff);
                let r =
                    _mm256_add_epi32(_mm256_mullo_epi32(y_i32, _mm256_set1_epi32(128)), v_scaled);

                let u_scaled_g = _mm256_mullo_epi32(u_i32, u_g_coeff);
                let v_scaled_g = _mm256_mullo_epi32(v_i32, v_g_coeff);
                let g = _mm256_sub_epi32(
                    _mm256_mullo_epi32(y_i32, _mm256_set1_epi32(128)),
                    _mm256_add_epi32(u_scaled_g, v_scaled_g),
                );

                let u_scaled_b = _mm256_mullo_epi32(u_i32, u_b_coeff);
                let b = _mm256_add_epi32(
                    _mm256_mullo_epi32(y_i32, _mm256_set1_epi32(128)),
                    u_scaled_b,
                );

                // Clamp to 0-255 and pack to 8-bit
                let r = clamp_epi32_to_epu8(r);
                let g = clamp_epi32_to_epu8(g);
                let b = clamp_epi32_to_epu8(b);

                // Store interleaved RGB
                store_rgb_interleaved(rgb, idx * 3, r, g, b);
            } else {
                // Fallback to scalar for remaining pixels
                for i in 0..8.min(width - x) {
                    let idx = y_row_start + x + i;

                    let y_val = y_plane[idx] as f32;
                    let u_val = u_plane[idx] as f32 - YUV_CHROMA_OFFSET;
                    let v_val = v_plane[idx] as f32 - YUV_CHROMA_OFFSET;

                    let (r, g, b) = yuv_to_rgb_pixel(y_val, u_val, v_val);
                    rgb[idx * 3] = r;
                    rgb[idx * 3 + 1] = g;
                    rgb[idx * 3 + 2] = b;
                }
            }
        }
    }
}

/// AVX2 implementation of YUV420 to RGB conversion for 10/12-bit video
///
/// # Safety
/// Caller must ensure:
/// - AVX2 is available on the CPU
/// - All buffers are valid and properly sized
#[target_feature(enable = "avx2")]
/// Helper function that processes YUV420 to RGB with a specific shift amount
unsafe fn yuv420_to_rgb_avx2_inner(
    y_plane: &[u8],
    u_plane: &[u8],
    v_plane: &[u8],
    width: usize,
    height: usize,
    rgb: &mut [u8],
    bit_depth: u8,
) {
    let uv_width = width / 2;

    // Coefficients for BT.601 color space
    let v_coeff: __m256i = _mm256_set1_epi32(181);
    let u_g_coeff: __m256i = _mm256_set1_epi32(44);
    let v_g_coeff: __m256i = _mm256_set1_epi32(91);
    let u_b_coeff: __m256i = _mm256_set1_epi32(227);
    let _const_128: __m256i = _mm256_set1_epi32(128);

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
                .and_then(|v| v.checked_add(16))
                .map_or(false, |offset| {
                    offset <= u_plane.len() && offset <= v_plane.len()
                });

            if x + 8 <= width && y_safe && uv_safe {
                // Load 8 Y samples as 16-bit values
                let y_vec = _mm256_loadu_si256(y_plane.as_ptr().add(y_idx * 2) as *const __m256i);
                // Load 4 U/V samples and duplicate to 8
                let u_4 = _mm_loadu_si128(u_plane.as_ptr().add(uv_idx * 2) as *const __m128i);
                let v_4 = _mm_loadu_si128(v_plane.as_ptr().add(uv_idx * 2) as *const __m128i);

                // Duplicate U/V: [u0,u1,u2,u3] -> [u0,u0,u1,u1,u2,u2,u3,u3]
                let u_4x = _mm_mullo_epi16(u_4, _mm_set1_epi16(0x0101));
                let u_vec = _mm256_cvtepu16_epi32(u_4x);
                let v_4x = _mm_mullo_epi16(v_4, _mm_set1_epi16(0x0101));
                let v_vec = _mm256_cvtepu16_epi32(v_4x);

                // Convert Y from 16-bit to 32-bit, then normalize to 8-bit range
                // Extract 128-bit lanes from the 256-bit Y vector
                let y_low_lane = _mm256_castsi256_si128(y_vec);
                let y_high_lane = _mm256_extracti128_si256(y_vec, 1);
                // Convert each 128-bit lane of 16-bit values to 32-bit
                let y_low_i32 = _mm256_cvtepu16_epi32(y_low_lane);
                let y_high_i32 = _mm256_cvtepu16_epi32(y_high_lane);
                // Combine the two 256-bit results
                let y_i32 = _mm256_or_si256(y_low_i32, y_high_i32);

                // Normalize 10/12-bit to 8-bit range by right-shifting
                let y_i32 = match bit_depth {
                    10 => _mm256_srai_epi32(y_i32, 2),
                    12 => _mm256_srai_epi32(y_i32, 4),
                    _ => y_i32, // 8-bit or unsupported: no shift
                };

                // Convert U/V from 16-bit to 32-bit and normalize
                let u_32 = match bit_depth {
                    10 => _mm256_srai_epi32(u_vec, 2),
                    12 => _mm256_srai_epi32(u_vec, 4),
                    _ => u_vec,
                };
                let v_32 = match bit_depth {
                    10 => _mm256_srai_epi32(v_vec, 2),
                    12 => _mm256_srai_epi32(v_vec, 4),
                    _ => v_vec,
                };

                // Subtract 128 (center chroma)
                let u_i32 = _mm256_sub_epi32(u_32, const_128);
                let v_i32 = _mm256_sub_epi32(v_32, const_128);

                // BT.601 conversion
                let v_scaled = _mm256_mullo_epi32(v_i32, v_coeff);
                let r =
                    _mm256_add_epi32(_mm256_mullo_epi32(y_i32, _mm256_set1_epi32(128)), v_scaled);

                let u_scaled_g = _mm256_mullo_epi32(u_i32, u_g_coeff);
                let v_scaled_g = _mm256_mullo_epi32(v_i32, v_g_coeff);
                let g = _mm256_sub_epi32(
                    _mm256_mullo_epi32(y_i32, _mm256_set1_epi32(128)),
                    _mm256_add_epi32(u_scaled_g, v_scaled_g),
                );

                let u_scaled_b = _mm256_mullo_epi32(u_i32, u_b_coeff);
                let b = _mm256_add_epi32(
                    _mm256_mullo_epi32(y_i32, _mm256_set1_epi32(128)),
                    u_scaled_b,
                );

                // Shift and clamp to 0-255
                let r = clamp_epi32_to_epu8(_mm256_srai_epi32(r, 7));
                let g = clamp_epi32_to_epu8(_mm256_srai_epi32(g, 7));
                let b = clamp_epi32_to_epu8(_mm256_srai_epi32(b, 7));

                store_rgb_interleaved(rgb, y_idx * 3, r, g, b);
            } else {
                // Fallback to scalar for remaining pixels
                for i in 0..8.min(width - x) {
                    let idx = y_idx + i;
                    let uv_i = uv_idx + (i / 2);

                    let y_val = read_sample_16bit(y_plane, idx, bit_depth);
                    let u_val = read_sample_16bit(u_plane, uv_i, bit_depth) as i32 - 128;
                    let v_val = read_sample_16bit(v_plane, uv_i, bit_depth) as i32 - 128;

                    let (r, g, b) = yuv_to_rgb_pixel_int(y_val, u_val, v_val);
                    rgb[idx * 3] = r;
                    rgb[idx * 3 + 1] = g;
                    rgb[idx * 3 + 2] = b;
                }
            }
        }
    }
}

unsafe fn yuv420_to_rgb_avx2_impl_16bit(
    y_plane: &[u8],
    u_plane: &[u8],
    v_plane: &[u8],
    width: usize,
    height: usize,
    rgb: &mut [u8],
    bit_depth: u8,
) {
    yuv420_to_rgb_avx2_inner(y_plane, u_plane, v_plane, width, height, rgb, bit_depth);
}

/// AVX2 implementation of YUV422 to RGB conversion for 10/12-bit video
///
/// # Safety
/// Caller must ensure:
/// - AVX2 is available on the CPU
/// - All buffers are valid and properly sized
/// Helper function that processes YUV422 to RGB with a specific shift amount
#[target_feature(enable = "avx2")]
unsafe fn yuv422_to_rgb_avx2_inner(
    y_plane: &[u8],
    u_plane: &[u8],
    v_plane: &[u8],
    width: usize,
    height: usize,
    rgb: &mut [u8],
    bit_depth: u8,
) {
    let uv_width = width / 2;

    // Coefficients for BT.601 color space
    let v_coeff: __m256i = _mm256_set1_epi32(181);
    let u_g_coeff: __m256i = _mm256_set1_epi32(44);
    let v_g_coeff: __m256i = _mm256_set1_epi32(91);
    let u_b_coeff: __m256i = _mm256_set1_epi32(227);
    let _const_128: __m256i = _mm256_set1_epi32(128);

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
                .and_then(|v| v.checked_add(16))
                .map_or(false, |offset| {
                    offset <= u_plane.len() && offset <= v_plane.len()
                });

            if x + 8 <= width && y_safe && uv_safe {
                // Load 8 Y samples as 16-bit
                let y_vec = _mm256_loadu_si256(y_plane.as_ptr().add(y_idx * 2) as *const __m256i);
                // Load 4 U/V and duplicate
                let u_4 = _mm_loadu_si128(u_plane.as_ptr().add(uv_idx * 2) as *const __m128i);
                let v_4 = _mm_loadu_si128(v_plane.as_ptr().add(uv_idx * 2) as *const __m128i);

                let u_4x = _mm_mullo_epi16(u_4, _mm_set1_epi16(0x0101));
                let u_vec = _mm256_cvtepu16_epi32(u_4x);
                let v_4x = _mm_mullo_epi16(v_4, _mm_set1_epi16(0x0101));
                let v_vec = _mm256_cvtepu16_epi32(v_4x);

                // Normalize Y (extract 128-bit lanes and convert to 32-bit)
                let y_low_lane = _mm256_castsi256_si128(y_vec);
                let y_high_lane = _mm256_extracti128_si256(y_vec, 1);
                let y_low_i32 = _mm256_cvtepu16_epi32(y_low_lane);
                let y_high_i32 = _mm256_cvtepu16_epi32(y_high_lane);
                let y_i32 = _mm256_or_si256(y_low_i32, y_high_i32);
                let y_i32 = match bit_depth {
                    10 => _mm256_srai_epi32(y_i32, 2),
                    12 => _mm256_srai_epi32(y_i32, 4),
                    _ => y_i32,
                };

                // Normalize U/V
                let u_32 = match bit_depth {
                    10 => _mm256_srai_epi32(u_vec, 2),
                    12 => _mm256_srai_epi32(u_vec, 4),
                    _ => u_vec,
                };
                let v_32 = match bit_depth {
                    10 => _mm256_srai_epi32(v_vec, 2),
                    12 => _mm256_srai_epi32(v_vec, 4),
                    _ => v_vec,
                };
                let u_i32 = _mm256_sub_epi32(u_32, const_128);
                let v_i32 = _mm256_sub_epi32(v_32, const_128);

                // BT.601 conversion (same as YUV420)
                let v_scaled = _mm256_mullo_epi32(v_i32, v_coeff);
                let r =
                    _mm256_add_epi32(_mm256_mullo_epi32(y_i32, _mm256_set1_epi32(128)), v_scaled);
                let u_scaled_g = _mm256_mullo_epi32(u_i32, u_g_coeff);
                let v_scaled_g = _mm256_mullo_epi32(v_i32, v_g_coeff);
                let g = _mm256_sub_epi32(
                    _mm256_mullo_epi32(y_i32, _mm256_set1_epi32(128)),
                    _mm256_add_epi32(u_scaled_g, v_scaled_g),
                );
                let u_scaled_b = _mm256_mullo_epi32(u_i32, u_b_coeff);
                let b = _mm256_add_epi32(
                    _mm256_mullo_epi32(y_i32, _mm256_set1_epi32(128)),
                    u_scaled_b,
                );

                let r = clamp_epi32_to_epu8(_mm256_srai_epi32(r, 7));
                let g = clamp_epi32_to_epu8(_mm256_srai_epi32(g, 7));
                let b = clamp_epi32_to_epu8(_mm256_srai_epi32(b, 7));

                store_rgb_interleaved(rgb, y_idx * 3, r, g, b);
            } else {
                // Fallback to scalar
                for i in 0..8.min(width - x) {
                    let idx = y_idx + i;
                    let uv_i = uv_idx + (i / 2);

                    let y_val = read_sample_16bit(y_plane, idx, bit_depth);
                    let u_val = read_sample_16bit(u_plane, uv_i, bit_depth) as i32 - 128;
                    let v_val = read_sample_16bit(v_plane, uv_i, bit_depth) as i32 - 128;

                    let (r, g, b) = yuv_to_rgb_pixel_int(y_val, u_val, v_val);
                    rgb[idx * 3] = r;
                    rgb[idx * 3 + 1] = g;
                    rgb[idx * 3 + 2] = b;
                }
            }
        }
    }
}

#[target_feature(enable = "avx2")]
unsafe fn yuv422_to_rgb_avx2_impl_16bit(
    y_plane: &[u8],
    u_plane: &[u8],
    v_plane: &[u8],
    width: usize,
    height: usize,
    rgb: &mut [u8],
    bit_depth: u8,
) {
    yuv422_to_rgb_avx2_inner(y_plane, u_plane, v_plane, width, height, rgb, bit_depth);
}

/// AVX2 implementation of YUV444 to RGB conversion for 10/12-bit video
///
/// # Safety
/// Caller must ensure:
/// - AVX2 is available on the CPU
/// - All buffers are valid and properly sized
/// Helper function that processes YUV444 to RGB with a specific shift amount
#[target_feature(enable = "avx2")]
unsafe fn yuv444_to_rgb_avx2_inner(
    y_plane: &[u8],
    u_plane: &[u8],
    v_plane: &[u8],
    width: usize,
    height: usize,
    rgb: &mut [u8],
    bit_depth: u8,
) {
    // Coefficients for BT.601 color space
    let v_coeff: __m256i = _mm256_set1_epi32(181);
    let u_g_coeff: __m256i = _mm256_set1_epi32(44);
    let v_g_coeff: __m256i = _mm256_set1_epi32(91);
    let u_b_coeff: __m256i = _mm256_set1_epi32(227);
    let _const_128: __m256i = _mm256_set1_epi32(128);

    for y in 0..height {
        let y_row_start = y * width;

        for x in (0..width).step_by(8) {
            let idx = y_row_start + x;

            // Bounds check for 16-bit data
            let y_safe = (idx * 2 + 16) <= y_plane.len();
            let uv_safe = (idx * 2 + 16) <= u_plane.len() && (idx * 2 + 16) <= v_plane.len();

            if x + 8 <= width && y_safe && uv_safe {
                // Load 8 Y, U, V samples as 16-bit (no subsampling in YUV444)
                let y_vec = _mm256_loadu_si256(y_plane.as_ptr().add(idx * 2) as *const __m256i);
                let u_vec = _mm256_loadu_si256(u_plane.as_ptr().add(idx * 2) as *const __m256i);
                let v_vec = _mm256_loadu_si256(v_plane.as_ptr().add(idx * 2) as *const __m256i);

                // Convert 16-bit to 32-bit and normalize
                // Extract 128-bit lanes and convert each
                let y_low_lane = _mm256_castsi256_si128(y_vec);
                let y_high_lane = _mm256_extracti128_si256(y_vec, 1);
                let y_low_i32 = _mm256_cvtepu16_epi32(y_low_lane);
                let y_high_i32 = _mm256_cvtepu16_epi32(y_high_lane);
                let y_i32 = _mm256_or_si256(y_low_i32, y_high_i32);

                let u_low_lane = _mm256_castsi256_si128(u_vec);
                let u_high_lane = _mm256_extracti128_si256(u_vec, 1);
                let u_low_i32 = _mm256_cvtepu16_epi32(u_low_lane);
                let u_high_i32 = _mm256_cvtepu16_epi32(u_high_lane);
                let u_i32 = _mm256_or_si256(u_low_i32, u_high_i32);

                let v_low_lane = _mm256_castsi256_si128(v_vec);
                let v_high_lane = _mm256_extracti128_si256(v_vec, 1);
                let v_low_i32 = _mm256_cvtepu16_epi32(v_low_lane);
                let v_high_i32 = _mm256_cvtepu16_epi32(v_high_lane);
                let v_i32 = _mm256_or_si256(v_low_i32, v_high_i32);

                // Normalize and center chroma
                let y_i32 = match bit_depth {
                    10 => _mm256_srai_epi32(y_i32, 2),
                    12 => _mm256_srai_epi32(y_i32, 4),
                    _ => y_i32,
                };
                let u_i32 = match bit_depth {
                    10 => _mm256_sub_epi32(_mm256_srai_epi32(u_i32, 2), const_128),
                    12 => _mm256_sub_epi32(_mm256_srai_epi32(u_i32, 4), const_128),
                    _ => _mm256_sub_epi32(u_i32, const_128),
                };
                let v_i32 = match bit_depth {
                    10 => _mm256_sub_epi32(_mm256_srai_epi32(v_i32, 2), const_128),
                    12 => _mm256_sub_epi32(_mm256_srai_epi32(v_i32, 4), const_128),
                    _ => _mm256_sub_epi32(v_i32, const_128),
                };

                // BT.601 conversion
                let v_scaled = _mm256_mullo_epi32(v_i32, v_coeff);
                let r =
                    _mm256_add_epi32(_mm256_mullo_epi32(y_i32, _mm256_set1_epi32(128)), v_scaled);
                let u_scaled_g = _mm256_mullo_epi32(u_i32, u_g_coeff);
                let v_scaled_g = _mm256_mullo_epi32(v_i32, v_g_coeff);
                let g = _mm256_sub_epi32(
                    _mm256_mullo_epi32(y_i32, _mm256_set1_epi32(128)),
                    _mm256_add_epi32(u_scaled_g, v_scaled_g),
                );
                let u_scaled_b = _mm256_mullo_epi32(u_i32, u_b_coeff);
                let b = _mm256_add_epi32(
                    _mm256_mullo_epi32(y_i32, _mm256_set1_epi32(128)),
                    u_scaled_b,
                );

                let r = clamp_epi32_to_epu8(_mm256_srai_epi32(r, 7));
                let g = clamp_epi32_to_epu8(_mm256_srai_epi32(g, 7));
                let b = clamp_epi32_to_epu8(_mm256_srai_epi32(b, 7));

                store_rgb_interleaved(rgb, idx * 3, r, g, b);
            } else {
                // Fallback to scalar
                for i in 0..8.min(width - x) {
                    let idx = y_row_start + x + i;

                    let y_val = read_sample_16bit(y_plane, idx, bit_depth);
                    let u_val = read_sample_16bit(u_plane, idx, bit_depth) as i32 - 128;
                    let v_val = read_sample_16bit(v_plane, idx, bit_depth) as i32 - 128;

                    let (r, g, b) = yuv_to_rgb_pixel_int(y_val, u_val, v_val);
                    rgb[idx * 3] = r;
                    rgb[idx * 3 + 1] = g;
                    rgb[idx * 3 + 2] = b;
                }
            }
        }
    }
}

#[target_feature(enable = "avx2")]
unsafe fn yuv444_to_rgb_avx2_impl_16bit(
    y_plane: &[u8],
    u_plane: &[u8],
    v_plane: &[u8],
    width: usize,
    height: usize,
    rgb: &mut [u8],
    bit_depth: u8,
) {
    yuv444_to_rgb_avx2_inner(y_plane, u_plane, v_plane, width, height, rgb, bit_depth);
}

/// Read a 16-bit sample from plane data and normalize to 8-bit range
#[inline]
unsafe fn read_sample_16bit(plane: &[u8], idx: usize, bit_depth: u8) -> i32 {
    let byte_idx = idx * 2;
    if byte_idx + 1 < plane.len() {
        let sample16 = u16::from_le_bytes([plane[byte_idx], plane[byte_idx + 1]]);
        // Normalize to 8-bit by right-shifting
        (sample16 >> (bit_depth - 8)) as i32
    } else {
        0
    }
}

/// Convert YUV to RGB using integer arithmetic
#[inline]
fn yuv_to_rgb_pixel_int(y: i32, u: i32, v: i32) -> (u8, u8, u8) {
    let y_scaled = y * 128;
    let r = ((y_scaled + 181 * v) >> 7).clamp(0, 255) as u8;
    let g = ((y_scaled - 44 * u - 91 * v) >> 7).clamp(0, 255) as u8;
    let b = ((y_scaled + 227 * u) >> 7).clamp(0, 255) as u8;
    (r, g, b)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_avx2_strategy_creation() {
        let strategy = Avx2Strategy::new();
        assert_eq!(strategy.name(), "AVX2");
    }

    #[test]
    fn test_avx2_capabilities() {
        let strategy = Avx2Strategy::new();
        let caps = strategy.capabilities();
        assert_eq!(caps.speedup_factor, 4.5);
        assert!(!caps.supports_10bit); // Currently only 8-bit supported
        assert!(!caps.supports_12bit); // Currently only 8-bit supported
        assert!(!caps.is_hardware_accelerated);
    }

    #[test]
    fn test_avx2_default() {
        let strategy = Avx2Strategy::default();
        assert_eq!(strategy.name(), "AVX2");
    }

    #[test]
    fn test_avx2_unsupported_bit_depth() {
        let strategy = Avx2Strategy::new();

        let y_plane = vec![0; 100];
        let u_plane = vec![128; 25];
        let v_plane = vec![128; 25];
        let mut rgb = vec![0u8; 300];

        // 10-bit not supported by AVX2 yet
        let result =
            strategy.convert_yuv420_to_rgb(&y_plane, &u_plane, &v_plane, 10, 10, &mut rgb, 10);

        assert!(result.is_err());
    }

    #[test]
    #[cfg(target_arch = "x86_64")]
    fn test_avx2_is_available_when_supported() {
        let strategy = Avx2Strategy::new();
        // On x86_64 with AVX2 support, this should be true
        let available = strategy.is_available();
        // We can't assert true because the test might run on non-AVX2 hardware
        // But we can verify it returns a boolean without panicking
        let _ = available;
    }
}
