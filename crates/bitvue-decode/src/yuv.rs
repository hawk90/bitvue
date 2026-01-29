//! YUV to RGB conversion utilities with SIMD optimization

use crate::decoder::{ChromaFormat, DecodedFrame};
use tracing::debug;

// ============================================================================
// Constants
// ============================================================================

/// Maximum allowed frame size (8K RGB)
const MAX_FRAME_SIZE: usize = 7680 * 4320 * 3;

/// BT.601 color conversion coefficients (precomputed for performance)
const BT601_COEFFS: ColorspaceCoeffs = ColorspaceCoeffs {
    yr: 1.0,
    yg: 0.0,
    yb: 0.0,
    ur: 0.0,
    ug: -0.344136,
    ub: 1.772,
    vr: 1.402,
    vg: -0.714136,
    vb: 0.0,
};

struct ColorspaceCoeffs {
    yr: f32,
    yg: f32,
    yb: f32,
    ur: f32,
    ug: f32,
    ub: f32,
    vr: f32,
    vg: f32,
    vb: f32,
}

// ============================================================================
// SIMD Optimized YUV to RGB Conversion
// ============================================================================

#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

/// YUV to RGB conversion with SIMD acceleration
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn yuv420_to_rgb_avx2(
    y_plane: &[u8],
    u_plane: &[u8],
    v_plane: &[u8],
    width: usize,
    height: usize,
    rgb: &mut [u8],
    bit_depth: u8,
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
                let y_vec = _mm256_loadu_si256(
                    y_plane.as_ptr().add(y_idx) as *const __m256i
                );

                // Load 4 U and V pixels, duplicate to 8
                let u_vals = [u_plane[uv_idx], u_plane[uv_idx + 1],
                             u_plane[uv_idx + 2], u_plane[uv_idx + 3]];
                let v_vals = [v_plane[uv_idx], v_plane[uv_idx + 1],
                             v_plane[uv_idx + 2], v_plane[uv_idx + 3]];

                let u_vec = _mm256_setr_epi32(
                    (u_vals[0] as i32 - 128),
                    (u_vals[0] as i32 - 128),
                    (u_vals[1] as i32 - 128),
                    (u_vals[1] as i32 - 128),
                    (u_vals[2] as i32 - 128),
                    (u_vals[2] as i32 - 128),
                    (u_vals[3] as i32 - 128),
                    (u_vals[3] as i32 - 128),
                );

                let v_vec = _mm256_setr_epi32(
                    (v_vals[0] as i32 - 128),
                    (v_vals[0] as i32 - 128),
                    (v_vals[1] as i32 - 128),
                    (v_vals[1] as i32 - 128),
                    (v_vals[2] as i32 - 128),
                    (v_vals[2] as i32 - 128),
                    (v_vals[3] as i32 - 128),
                    (v_vals[3] as i32 - 128),
                );

                // Convert Y to i32
                let y_i32 = _mm256_cvtepu8_epi32(y_vec);

                // BT.601 conversion with integer arithmetic
                // R = Y + 1.402 * V  (approx 181/128 * V)
                // G = Y - 0.344 * U - 0.714 * V
                // B = Y + 1.772 * U  (approx 227/128 * U)

                let r = _mm256_add_epi32(y_i32,
                    _mm256_mullo_epi32(v_vec, _mm256_set1_epi32(181)) >> 7);

                let g_term1 = _mm256_mullo_epi32(u_vec, _mm256_set1_epi32(44)) >> 7;
                let g_term2 = _mm256_mullo_epi32(v_vec, _mm256_set1_epi32(91)) >> 7;
                let g = _mm256_sub_epi32(y_i32, _mm256_add_epi32(g_term1, g_term2));

                let b = _mm256_add_epi32(y_i32,
                    _mm256_mullo_epi32(u_vec, _mm256_set1_epi32(227)) >> 7);

                // Clamp and pack to 8-bit
                let r_clamped = clamp_epi32_to_epu8(r);
                let g_clamped = clamp_epi32_to_epu8(g);
                let b_clamped = clamp_epi32_to_epu8(b);

                // Store interleaved RGB
                store_rgb_interleaved(rgb, y_idx * 3, r_clamped, g_clamped, b_clamped);
            } else {
                // Fallback to scalar for remaining pixels
                for i in 0..8.min(width - x) {
                    let idx = y_idx + i;
                    let uv_i = uv_idx + (i / 2);

                    let y_val = read_sample(y_plane, idx, bit_depth) as f32;
                    let u_val = read_sample(u_plane, uv_i, bit_depth) as f32 - 128.0;
                    let v_val = read_sample(v_plane, uv_i, bit_depth) as f32 - 128.0;

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
#[cfg(target_arch = "x86_64")]
#[inline]
unsafe fn clamp_epi32_to_epu8(v: __m256i) -> __m256i {
    let zero = _mm256_setzero_si256();
    let max = _mm256_set1_epi32(255);

    let clamped = _mm256_max_epi32(
        _mm256_min_epi32(v, max),
        zero
    );

    // Pack to 8-bit
    let packed = _mm256_packs_epi32(clamped, clamped);
    _mm256_packus_epi16(packed, packed)
}

/// Store RGB values interleaved
#[cfg(target_arch = "x86_64")]
#[inline]
unsafe fn store_rgb_interleaved(rgb: &mut [u8], offset: usize, r: __m256i, g: __m256i, b: __m256i) {
    // Extract and interleave RGB values
    for i in 0..8 {
        let r_val = _mm256_extract_epi8(r, i) as u8;
        let g_val = _mm256_extract_epi8(g, i) as u8;
        let b_val = _mm256_extract_epi8(b, i) as u8;

        let idx = offset + i * 3;
        if idx + 2 < rgb.len() {
            rgb[idx] = r_val;
            rgb[idx + 1] = g_val;
            rgb[idx + 2] = b_val;
        }
    }
}

// ============================================================================
// Scalar Fallback Implementation
// ============================================================================

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
#[inline]
fn yuv_to_rgb_pixel(y: f32, u: f32, v: f32) -> (u8, u8, u8) {
    // BT.601 conversion matrix
    let r = (y + 1.402 * v).clamp(0.0, 255.0) as u8;
    let g = (y - 0.344136 * u - 0.714136 * v).clamp(0.0, 255.0) as u8;
    let b = (y + 1.772 * u).clamp(0.0, 255.0) as u8;
    (r, g, b)
}

/// Scalar YUV420 to RGB conversion (fallback)
fn yuv420_to_rgb_scalar(
    y_plane: &[u8],
    u_plane: &[u8],
    v_plane: &[u8],
    width: usize,
    height: usize,
    rgb: &mut [u8],
    bit_depth: u8,
) {
    for y in 0..height {
        for x in 0..width {
            let y_idx = y * width + x;
            let uv_idx = (y / 2) * (width / 2) + (x / 2);

            let y_val = read_sample(y_plane, y_idx, bit_depth) as f32;
            let u_val = read_sample(u_plane, uv_idx, bit_depth) as f32 - 128.0;
            let v_val = read_sample(v_plane, uv_idx, bit_depth) as f32 - 128.0;

            let (r, g, b) = yuv_to_rgb_pixel(y_val, u_val, v_val);

            let rgb_idx = y_idx * 3;
            rgb[rgb_idx] = r;
            rgb[rgb_idx + 1] = g;
            rgb[rgb_idx + 2] = b;
        }
    }
}

// ============================================================================
// Main Conversion Function
// ============================================================================

/// Converts a decoded YUV frame to RGB with validation and SIMD acceleration
pub fn yuv_to_rgb(frame: &DecodedFrame) -> Vec<u8> {
    let width = frame.width as usize;
    let height = frame.height as usize;

    // Validate frame size to prevent overflow/DoS
    let required_size = match width.checked_mul(height) {
        Some(v) => v,
        None => {
            tracing::error!("Frame dimensions overflow: {}x{}", width, height);
            return vec![0; MAX_FRAME_SIZE.min(1920 * 1080 * 3)]; // Return safe default
        }
    };

    let required_size = match required_size.checked_mul(3) {
        Some(v) => v,
        None => {
            tracing::error!("Frame size overflow: {}x{}x3", width, height);
            return vec![0; MAX_FRAME_SIZE.min(1920 * 1080 * 3)];
        }
    };

    if required_size > MAX_FRAME_SIZE {
        tracing::error!(
            "Frame size {}x{} exceeds maximum allowed {}",
            width, height, MAX_FRAME_SIZE / 3
        );
        return vec![0; MAX_FRAME_SIZE];
    }

    let mut rgb = vec![0u8; required_size];
    let chroma_format = frame.chroma_format;
    let bit_depth = frame.bit_depth;

    debug!(
        "Converting {:?} frame to RGB ({}x{}, {}bit, {} bytes)",
        chroma_format, width, height, bit_depth, required_size
    );

    match chroma_format {
        ChromaFormat::Monochrome => {
            // Y only - grayscale
            for i in 0..(width * height) {
                let y_val = read_sample(&frame.y_plane, i, bit_depth);
                let rgb_idx = i * 3;
                rgb[rgb_idx] = y_val;
                rgb[rgb_idx + 1] = y_val;
                rgb[rgb_idx + 2] = y_val;
            }
        }
        ChromaFormat::Yuv420 => {
            let u_plane = match frame.u_plane.as_ref() {
                Some(plane) => plane,
                None => {
                    tracing::error!("Yuv420 frame missing U plane, falling back to grayscale");
                    for i in 0..(width * height) {
                        let y_val = read_sample(&frame.y_plane, i, bit_depth);
                        let rgb_idx = i * 3;
                        rgb[rgb_idx] = y_val;
                        rgb[rgb_idx + 1] = y_val;
                        rgb[rgb_idx + 2] = y_val;
                    }
                    return rgb;
                }
            };
            let v_plane = match frame.v_plane.as_ref() {
                Some(plane) => plane,
                None => {
                    tracing::error!("Yuv420 frame missing V plane, falling back to grayscale");
                    for i in 0..(width * height) {
                        let y_val = read_sample(&frame.y_plane, i, bit_depth);
                        let rgb_idx = i * 3;
                        rgb[rgb_idx] = y_val;
                        rgb[rgb_idx + 1] = y_val;
                        rgb[rgb_idx + 2] = y_val;
                    }
                    return rgb;
                }
            };

            // Try SIMD if available (x86_64 with AVX2)
            #[cfg(target_arch = "x86_64")]
            {
                if is_x86_feature_detected!("avx2") {
                    unsafe {
                        yuv420_to_rgb_avx2(&frame.y_plane, u_plane, v_plane, width, height, &mut rgb, bit_depth);
                        return rgb;
                    }
                }
            }

            // Fallback to scalar
            yuv420_to_rgb_scalar(&frame.y_plane, u_plane, v_plane, width, height, &mut rgb, bit_depth);
        }
        ChromaFormat::Yuv422 => {
            let u_plane = match frame.u_plane.as_ref() {
                Some(plane) => plane,
                None => {
                    tracing::error!("Yuv422 frame missing U plane, falling back to grayscale");
                    for i in 0..(width * height) {
                        let y_val = read_sample(&frame.y_plane, i, bit_depth);
                        let rgb_idx = i * 3;
                        rgb[rgb_idx] = y_val;
                        rgb[rgb_idx + 1] = y_val;
                        rgb[rgb_idx + 2] = y_val;
                    }
                    return rgb;
                }
            };
            let v_plane = match frame.v_plane.as_ref() {
                Some(plane) => plane,
                None => {
                    tracing::error!("Yuv422 frame missing V plane, falling back to grayscale");
                    for i in 0..(width * height) {
                        let y_val = read_sample(&frame.y_plane, i, bit_depth);
                        let rgb_idx = i * 3;
                        rgb[rgb_idx] = y_val;
                        rgb[rgb_idx + 1] = y_val;
                        rgb[rgb_idx + 2] = y_val;
                    }
                    return rgb;
                }
            };

            for y in 0..height {
                for x in 0..width {
                    let y_idx = y * width + x;
                    let uv_idx = y * (width / 2) + (x / 2);

                    let y_val = read_sample(&frame.y_plane, y_idx, bit_depth) as f32;
                    let u_val = read_sample(u_plane, uv_idx, bit_depth) as f32 - 128.0;
                    let v_val = read_sample(v_plane, uv_idx, bit_depth) as f32 - 128.0;

                    let (r, g, b) = yuv_to_rgb_pixel(y_val, u_val, v_val);

                    let rgb_idx = y_idx * 3;
                    rgb[rgb_idx] = r;
                    rgb[rgb_idx + 1] = g;
                    rgb[rgb_idx + 2] = b;
                }
            }
        }
        ChromaFormat::Yuv444 => {
            let u_plane = match frame.u_plane.as_ref() {
                Some(plane) => plane,
                None => {
                    tracing::error!("Yuv444 frame missing U plane, falling back to grayscale");
                    for i in 0..(width * height) {
                        let y_val = read_sample(&frame.y_plane, i, bit_depth);
                        let rgb_idx = i * 3;
                        rgb[rgb_idx] = y_val;
                        rgb[rgb_idx + 1] = y_val;
                        rgb[rgb_idx + 2] = y_val;
                    }
                    return rgb;
                }
            };
            let v_plane = match frame.v_plane.as_ref() {
                Some(plane) => plane,
                None => {
                    tracing::error!("Yuv444 frame missing V plane, falling back to grayscale");
                    for i in 0..(width * height) {
                        let y_val = read_sample(&frame.y_plane, i, bit_depth);
                        let rgb_idx = i * 3;
                        rgb[rgb_idx] = y_val;
                        rgb[rgb_idx + 1] = y_val;
                        rgb[rgb_idx + 2] = y_val;
                    }
                    return rgb;
                }
            };

            for y in 0..height {
                for x in 0..width {
                    let idx = y * width + x;

                    let y_val = read_sample(&frame.y_plane, idx, bit_depth) as f32;
                    let u_val = read_sample(u_plane, idx, bit_depth) as f32 - 128.0;
                    let v_val = read_sample(v_plane, idx, bit_depth) as f32 - 128.0;

                    let (r, g, b) = yuv_to_rgb_pixel(y_val, u_val, v_val);

                    let rgb_idx = idx * 3;
                    rgb[rgb_idx] = r;
                    rgb[rgb_idx + 1] = g;
                    rgb[rgb_idx + 2] = b;
                }
            }
        }
    }

    rgb
}

/// Converts RGB data to an image::RgbImage
pub fn rgb_to_image(rgb: &[u8], width: u32, height: u32) -> image::RgbImage {
    image::RgbImage::from_raw(width, height, rgb.to_vec())
        .expect("Failed to create image from RGB data")
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_monochrome_conversion() {
        let frame = DecodedFrame {
            width: 2,
            height: 2,
            bit_depth: 8,
            y_plane: vec![0, 128, 255, 64],
            y_stride: 2,
            u_plane: None,
            u_stride: 0,
            v_plane: None,
            v_stride: 0,
            timestamp: 0,
            frame_type: crate::decoder::FrameType::Key,
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
    fn test_frame_size_validation() {
        // Test overflow protection
        let huge_frame = DecodedFrame {
            width: 100000, // Would overflow without protection
            height: 100000,
            bit_depth: 8,
            y_plane: vec![0; 100],
            y_stride: 10,
            u_plane: None,
            u_stride: 0,
            v_plane: None,
            v_stride: 0,
            timestamp: 0,
            frame_type: crate::decoder::FrameType::Key,
            qp_avg: None,
            chroma_format: ChromaFormat::Monochrome,
        };

        let rgb = yuv_to_rgb(&huge_frame);
        // Should return safe default instead of panicking
        assert!(!rgb.is_empty());
        assert!(rgb.len() <= MAX_FRAME_SIZE);
    }
}
