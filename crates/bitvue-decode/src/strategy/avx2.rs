//! AVX2 strategy for x86_64.
//!
//! Eight pixels per step, computed in 32-bit lanes with the scalar code's own integer formula
//! (`>> 7`, clamp, samples cut to 8 bits first), so the result equals `ScalarStrategy`'s exactly.
//! The kernels this replaces loaded 32 bytes per 8 pixels and OR-ed the second half into the
//! luma, duplicated chroma into the wrong lanes and read past the end of a plane (#386, #399).
//!
//! Safety rules the kernel keeps: a vector step is taken only when every byte it loads lies inside
//! its plane (checked against the plane's real length); whatever is left of a row, and any call
//! the scalar code would reject, goes to the scalar code.

use super::scalar::{read_sample, yuv_to_rgb_pixel_u8};
use super::{ConversionResult, ScalarStrategy, StrategyCapabilities, YuvConversionStrategy};
use std::arch::x86_64::*;

/// How the chroma planes relate to the luma plane.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Subsampling {
    /// 4:2:0 - one chroma sample per 2x2 luma samples
    Yuv420,
    /// 4:2:2 - one chroma sample per 2 luma samples of a row
    Yuv422,
    /// 4:4:4 - one chroma sample per luma sample
    Yuv444,
}

impl Subsampling {
    /// Index of the chroma sample belonging to luma pixel `(x, y)`: the scalar code's formula.
    #[inline]
    fn chroma_index(self, x: usize, y: usize, width: usize) -> usize {
        match self {
            Self::Yuv420 => (y / 2) * (width / 2) + x / 2,
            Self::Yuv422 => y * (width / 2) + x / 2,
            Self::Yuv444 => y * width + x,
        }
    }

    /// Chroma samples the scalar code requires for a `width` x `height` picture.
    fn chroma_len(self, width: usize, height: usize) -> Option<usize> {
        match self {
            Self::Yuv420 => (width / 2).checked_mul(height / 2),
            Self::Yuv422 => (width / 2).checked_mul(height),
            Self::Yuv444 => width.checked_mul(height),
        }
    }
}

struct Planes<'a> {
    y: &'a [u8],
    u: &'a [u8],
    v: &'a [u8],
}

/// Converts with AVX2 when the call is one the scalar code accepts; `false` means "not handled".
fn convert(
    sub: Subsampling,
    planes: &Planes<'_>,
    width: usize,
    height: usize,
    rgb: &mut [u8],
    bit_depth: u8,
) -> bool {
    let (Some(luma), Some(chroma)) = (width.checked_mul(height), sub.chroma_len(width, height))
    else {
        return false;
    };
    let accepted = matches!(bit_depth, 8 | 10 | 12)
        && planes.y.len() >= luma
        && planes.u.len() >= chroma
        && planes.v.len() >= chroma
        && luma.checked_mul(3).is_some_and(|n| rgb.len() >= n);
    if !accepted || !is_x86_feature_detected!("avx2") {
        return false;
    }
    // SAFETY: AVX2 was detected just above; every vector load inside is bounds-checked.
    unsafe { kernel(sub, planes, width, height, rgb, bit_depth) };
    true
}

/// Loads 8 samples starting at `ptr` as 32-bit lanes, already cut to 8 bits like the scalar code.
#[target_feature(enable = "avx2")]
unsafe fn load_8(ptr: *const u8, bit_depth: u8) -> __m256i {
    if bit_depth == 8 {
        _mm256_cvtepu8_epi32(_mm_loadl_epi64(ptr as *const __m128i))
    } else {
        let wide = _mm256_cvtepu16_epi32(_mm_loadu_si128(ptr as *const __m128i));
        narrow(wide, bit_depth)
    }
}

/// Loads 4 samples and repeats each twice: `[a, a, b, b, c, c, d, d]`.
#[target_feature(enable = "avx2")]
unsafe fn load_4_doubled(ptr: *const u8, bit_depth: u8) -> __m256i {
    if bit_depth == 8 {
        let four = _mm_cvtsi32_si128(std::ptr::read_unaligned(ptr as *const i32));
        let doubled = _mm_shuffle_epi8(
            four,
            _mm_setr_epi8(0, 0, 1, 1, 2, 2, 3, 3, 0, 0, 0, 0, 0, 0, 0, 0),
        );
        _mm256_cvtepu8_epi32(doubled)
    } else {
        let four = _mm_loadl_epi64(ptr as *const __m128i);
        let doubled = _mm_shuffle_epi8(
            four,
            _mm_setr_epi8(0, 1, 0, 1, 2, 3, 2, 3, 4, 5, 4, 5, 6, 7, 6, 7),
        );
        narrow(_mm256_cvtepu16_epi32(doubled), bit_depth)
    }
}

/// `(sample >> (bit_depth - 8)) as u8`, per lane.
#[target_feature(enable = "avx2")]
unsafe fn narrow(samples: __m256i, bit_depth: u8) -> __m256i {
    let shifted = _mm256_srl_epi32(samples, _mm_cvtsi32_si128(i32::from(bit_depth) - 8));
    _mm256_and_si256(shifted, _mm256_set1_epi32(0xFF))
}

#[target_feature(enable = "avx2")]
unsafe fn clamp_byte(v: __m256i) -> __m256i {
    _mm256_min_epi32(
        _mm256_max_epi32(v, _mm256_setzero_si256()),
        _mm256_set1_epi32(255),
    )
}

#[target_feature(enable = "avx2")]
unsafe fn kernel(
    sub: Subsampling,
    planes: &Planes<'_>,
    width: usize,
    height: usize,
    rgb: &mut [u8],
    bit_depth: u8,
) {
    let bytes = if bit_depth == 8 { 1 } else { 2 };
    let chroma_step = if sub == Subsampling::Yuv444 { 8 } else { 4 };
    let c128 = _mm256_set1_epi32(128);
    for y in 0..height {
        let mut x = 0;
        while x + 8 <= width {
            let luma_at = y * width + x;
            let chroma_at = sub.chroma_index(x, y, width);
            // Every byte a step loads must lie inside its plane.
            if (luma_at + 8) * bytes > planes.y.len()
                || (chroma_at + chroma_step) * bytes > planes.u.len()
                || (chroma_at + chroma_step) * bytes > planes.v.len()
            {
                break;
            }
            let (luma, u, v) = {
                let l = load_8(planes.y.as_ptr().add(luma_at * bytes), bit_depth);
                let (u, v) = if sub == Subsampling::Yuv444 {
                    (
                        load_8(planes.u.as_ptr().add(chroma_at * bytes), bit_depth),
                        load_8(planes.v.as_ptr().add(chroma_at * bytes), bit_depth),
                    )
                } else {
                    (
                        load_4_doubled(planes.u.as_ptr().add(chroma_at * bytes), bit_depth),
                        load_4_doubled(planes.v.as_ptr().add(chroma_at * bytes), bit_depth),
                    )
                };
                (l, _mm256_sub_epi32(u, c128), _mm256_sub_epi32(v, c128))
            };
            let scaled = _mm256_slli_epi32(luma, 7);
            let r = _mm256_add_epi32(scaled, _mm256_mullo_epi32(v, _mm256_set1_epi32(181)));
            let g = _mm256_sub_epi32(
                _mm256_sub_epi32(scaled, _mm256_mullo_epi32(u, _mm256_set1_epi32(44))),
                _mm256_mullo_epi32(v, _mm256_set1_epi32(91)),
            );
            let b = _mm256_add_epi32(scaled, _mm256_mullo_epi32(u, _mm256_set1_epi32(227)));
            let mut out = [[0i32; 8]; 3];
            for (lane, value) in [r, g, b].into_iter().enumerate() {
                let value = clamp_byte(_mm256_srai_epi32(value, 7));
                _mm256_storeu_si256(out[lane].as_mut_ptr() as *mut __m256i, value);
            }
            let pixels = out[0].iter().zip(&out[1]).zip(&out[2]);
            for (i, ((r, g), b)) in pixels.enumerate() {
                let at = (luma_at + i) * 3;
                rgb[at] = *r as u8;
                rgb[at + 1] = *g as u8;
                rgb[at + 2] = *b as u8;
            }
            x += 8;
        }
        // What is left of the row: the scalar code's own pixel conversion.
        for x in x..width {
            let luma_at = y * width + x;
            let chroma_at = sub.chroma_index(x, y, width);
            let (r, g, b) = yuv_to_rgb_pixel_u8(
                read_sample(planes.y, luma_at, bit_depth),
                read_sample(planes.u, chroma_at, bit_depth),
                read_sample(planes.v, chroma_at, bit_depth),
            );
            rgb[luma_at * 3] = r;
            rgb[luma_at * 3 + 1] = g;
            rgb[luma_at * 3 + 2] = b;
        }
    }
}

/// AVX2 strategy - x86_64; converts with the scalar code for now (see the module doc)
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
        let planes = Planes {
            y: y_plane,
            u: u_plane,
            v: v_plane,
        };
        if convert(Subsampling::Yuv420, &planes, width, height, rgb, bit_depth) {
            return Ok(());
        }
        ScalarStrategy::new()
            .convert_yuv420_to_rgb(y_plane, u_plane, v_plane, width, height, rgb, bit_depth)
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
        let planes = Planes {
            y: y_plane,
            u: u_plane,
            v: v_plane,
        };
        if convert(Subsampling::Yuv422, &planes, width, height, rgb, bit_depth) {
            return Ok(());
        }
        ScalarStrategy::new()
            .convert_yuv422_to_rgb(y_plane, u_plane, v_plane, width, height, rgb, bit_depth)
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
        let planes = Planes {
            y: y_plane,
            u: u_plane,
            v: v_plane,
        };
        if convert(Subsampling::Yuv444, &planes, width, height, rgb, bit_depth) {
            return Ok(());
        }
        ScalarStrategy::new()
            .convert_yuv444_to_rgb(y_plane, u_plane, v_plane, width, height, rgb, bit_depth)
    }
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
        assert_eq!(
            caps.speedup_factor, 1.0,
            "speed is not measured yet, so the registry does not prefer it"
        );
        assert!(caps.supports_10bit); // 10-bit support added
        assert!(caps.supports_12bit); // 12-bit support added
        assert!(!caps.is_hardware_accelerated);
    }

    #[test]
    fn test_avx2_default() {
        // This tests the public `Default` impl itself, so it must call `default()` rather than
        // name the unit struct (which is what clippy would suggest).
        #[allow(clippy::default_constructed_unit_structs)]
        let strategy = Avx2Strategy::default();
        assert_eq!(strategy.name(), "AVX2");
    }

    #[test]
    fn test_avx2_supported_bit_depths() {
        let strategy = Avx2Strategy::new();

        let y_plane = vec![0; 100];
        let u_plane = vec![128; 25];
        let v_plane = vec![128; 25];
        let mut rgb = vec![0u8; 300];

        // 8-bit is supported
        let result_8bit =
            strategy.convert_yuv420_to_rgb(&y_plane, &u_plane, &v_plane, 10, 10, &mut rgb, 8);
        assert!(result_8bit.is_ok());

        // 10-bit is now supported
        let result_10bit =
            strategy.convert_yuv420_to_rgb(&y_plane, &u_plane, &v_plane, 10, 10, &mut rgb, 10);
        assert!(result_10bit.is_ok());
    }

    /// xorshift bytes; the same seed gives the same planes on every machine
    fn noise(seed: &mut u64, n: usize) -> Vec<u8> {
        (0..n)
            .map(|_| {
                *seed ^= *seed << 13;
                *seed ^= *seed >> 7;
                *seed ^= *seed << 17;
                (*seed >> 24) as u8
            })
            .collect()
    }

    #[test]
    fn matches_the_scalar_code_exactly_for_every_layout_depth_and_width() {
        let avx2 = Avx2Strategy::new();
        if !avx2.is_available() {
            return; // the CPU lacks AVX2: nothing to compare
        }
        let scalar = ScalarStrategy::new();
        let mut seed = 0x2545_F491_4F6C_DD1Du64;
        // widths around the 8-pixel step, odd ones too; full-range 16-bit samples, so the cut to
        // 8 bits of values above the bit depth is compared as well
        for width in 1..=41usize {
            for height in 1..=5usize {
                for depth in [8u8, 10, 12] {
                    let bytes = if depth > 8 { 2 } else { 1 };
                    for layout in 0..3 {
                        let chroma = match layout {
                            0 => (width / 2) * (height / 2),
                            1 => (width / 2) * height,
                            _ => width * height,
                        };
                        let y = noise(&mut seed, width * height * bytes);
                        let u = noise(&mut seed, chroma * bytes);
                        let v = noise(&mut seed, chroma * bytes);
                        let mut want = vec![0u8; width * height * 3];
                        let mut got = vec![0u8; width * height * 3];
                        let run = |s: &dyn YuvConversionStrategy, out: &mut [u8]| match layout {
                            0 => s.convert_yuv420_to_rgb(&y, &u, &v, width, height, out, depth),
                            1 => s.convert_yuv422_to_rgb(&y, &u, &v, width, height, out, depth),
                            _ => s.convert_yuv444_to_rgb(&y, &u, &v, width, height, out, depth),
                        };
                        run(&scalar, &mut want).unwrap();
                        run(&avx2, &mut got).unwrap();
                        assert_eq!(got, want, "layout {layout}, {width}x{height}, {depth}-bit");
                    }
                }
            }
        }
    }

    #[test]
    fn planes_just_long_enough_for_the_scalar_code_are_never_read_past() {
        let avx2 = Avx2Strategy::new();
        if !avx2.is_available() {
            return;
        }
        let scalar = ScalarStrategy::new();
        let mut seed = 0x9E37_79B9_7F4A_7C15u64;
        // 10-bit planes of only `width * height` bytes pass the scalar check although each
        // sample takes two: the scalar code reads 0 past the end, the kernel must give the same
        // without loading past it (a short plane makes the vector step fall back to scalar)
        for width in [8usize, 16, 17, 40] {
            let height = 3;
            let y = noise(&mut seed, width * height);
            let u = noise(&mut seed, (width / 2) * (height / 2));
            let v = noise(&mut seed, (width / 2) * (height / 2));
            let mut want = vec![0u8; width * height * 3];
            let mut got = vec![0u8; width * height * 3];
            scalar
                .convert_yuv420_to_rgb(&y, &u, &v, width, height, &mut want, 10)
                .unwrap();
            avx2.convert_yuv420_to_rgb(&y, &u, &v, width, height, &mut got, 10)
                .unwrap();
            assert_eq!(got, want, "{width}x{height}");
        }
    }

    #[test]
    fn calls_the_scalar_code_rejects_are_rejected_the_same_way() {
        let avx2 = Avx2Strategy::new();
        let mut rgb = vec![0u8; 16 * 16 * 3];
        let plane = vec![0u8; 16 * 16];
        assert!(avx2
            .convert_yuv444_to_rgb(&plane, &plane, &plane, 16, 16, &mut rgb, 9)
            .is_err());
        assert!(avx2
            .convert_yuv444_to_rgb(&plane[..100], &plane, &plane, 16, 16, &mut rgb, 8)
            .is_err());
        assert!(avx2
            .convert_yuv420_to_rgb(&plane, &plane[..10], &plane, 16, 16, &mut rgb, 8)
            .is_err());
        assert!(avx2
            .convert_yuv422_to_rgb(&plane, &plane, &plane, 16, 16, &mut rgb[..10], 8)
            .is_err());
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
