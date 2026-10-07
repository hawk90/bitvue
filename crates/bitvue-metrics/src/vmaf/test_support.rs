//! Synthetic frames shared by the tests of `mod.rs` and `ffi.rs`.

use super::VmafFrame;

/// Deterministic hash noise in `0..=amplitude`.
pub(crate) fn noise(x: usize, y: usize, seed: usize, amplitude: u32) -> f64 {
    let h = (x as u32)
        .wrapping_mul(73856093)
        .wrapping_add((y as u32).wrapping_mul(19349663))
        .wrapping_add((seed as u32).wrapping_mul(83492791))
        .wrapping_mul(2654435761);
    f64::from((h >> 24) % (amplitude + 1))
}

/// Smooth, slowly moving content with a little fine texture, so VMAF has real structure to
/// compare (patterns that wrap or alias make libvmaf's AVX2/AVX-512 kernels diverge by
/// tenths of a point and saturate scores at 0). `distortion == 0` is pristine; otherwise
/// the luma is 3x3 box-blurred and given hash noise whose amplitude is `distortion`.
pub(crate) fn frame(
    width: usize,
    height: usize,
    bit_depth: u8,
    seed: usize,
    distortion: u32,
) -> VmafFrame {
    let bytes = if bit_depth > 8 { 2 } else { 1 };
    let max = f64::from((1u32 << bit_depth) - 1);
    let pristine = |x: usize, y: usize| -> f64 {
        let (fx, fy, t) = (x as f64, y as f64, seed as f64);
        128.0
            + 55.0 * (fx * 0.045 + t * 0.15).sin() * (fy * 0.05).cos()
            + 25.0 * (fx * 0.011 - fy * 0.013).sin()
            + noise(x, y, 0, 14)
            - 7.0
    };
    let luma = |x: usize, y: usize| -> f64 {
        if distortion == 0 {
            return pristine(x, y);
        }
        let (mut sum, mut count) = (0.0, 0.0);
        for dy in -1i64..=1 {
            for dx in -1i64..=1 {
                let (nx, ny) = (x as i64 + dx, y as i64 + dy);
                if nx >= 0 && ny >= 0 && (nx as usize) < width && (ny as usize) < height {
                    sum += pristine(nx as usize, ny as usize);
                    count += 1.0;
                }
            }
        }
        sum / count + noise(x, y, seed + 1, distortion) - f64::from(distortion) / 2.0
    };
    let chroma = |x: usize, y: usize| -> f64 { 128.0 + 20.0 * ((x + y) as f64 * 0.03).sin() };

    let encode = |plane: &mut Vec<u8>, value: f64| {
        let v = ((value.clamp(0.0, 255.0) / 255.0) * max).round() as u32;
        plane.extend_from_slice(&v.to_le_bytes()[..bytes]);
    };
    let mut y_plane = Vec::with_capacity(width * height * bytes);
    for y in 0..height {
        for x in 0..width {
            encode(&mut y_plane, luma(x, y));
        }
    }
    let (cw, ch) = (width.div_ceil(2), height.div_ceil(2));
    let (mut u_plane, mut v_plane) = (Vec::new(), Vec::new());
    for y in 0..ch {
        for x in 0..cw {
            encode(&mut u_plane, chroma(x, y));
            encode(&mut v_plane, 255.0 - chroma(x, y));
        }
    }
    VmafFrame {
        y: y_plane,
        u: u_plane,
        v: v_plane,
        width,
        height,
        bit_depth,
    }
}

pub(crate) fn sequence(n: usize, bit_depth: u8, distortion: u32) -> Vec<VmafFrame> {
    (0..n)
        .map(|i| frame(176, 144, bit_depth, i, distortion))
        .collect()
}
