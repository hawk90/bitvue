//! bitvue-metrics: Video quality metrics
//!
//! This crate provides standard video quality metrics for comparing frames:
//! - PSNR (Peak Signal-to-Noise Ratio) - CPU & GPU-accelerated
//! - SSIM (Structural Similarity Index) - CPU & GPU-accelerated
//! - VMAF (Video Multimethod Assessment Fusion) - CPU & CUDA-accelerated (optional)
//!
//! # Features
//!
//! - `vmaf`: Enable VMAF support (CPU-only, requires libvmaf)
//! - `vmaf-cuda`: Enable CUDA-accelerated VMAF (requires libvmaf with CUDA)
//! - `parallel`: Enable multi-threaded CPU metrics using rayon
//!
//! # Example
//!
//! ```no_run
//! use bitvue_metrics::{psnr, ssim};
//!
//! let reference = vec![128u8; 1920 * 1080];
//! let distorted = vec![130u8; 1920 * 1080];
//!
//! let psnr_value = psnr(&reference, &distorted, 1920, 1080).unwrap();
//! let ssim_value = ssim(&reference, &distorted, 1920, 1080).unwrap();
//!
//! println!("PSNR: {:.2} dB", psnr_value);
//! println!("SSIM: {:.4}", ssim_value);
//! ```
//!
//! # VMAF Example (requires `vmaf` feature)
//!
//! ```no_run
//! #[cfg(feature = "vmaf")]
//! use bitvue_metrics::vmaf::compute_vmaf;
//!
//! # #[cfg(feature = "vmaf")]
//! # fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let reference_frames = vec![/* YUV frames */];
//! let distorted_frames = vec![/* YUV frames */];
//!
//! let score = compute_vmaf(&reference_frames, &distorted_frames, 1920, 1080, None)?;
//! println!("VMAF Score: {:.2}", score);
//! # Ok(())
//! # }
//! ```

use bitvue_core::{BitvueError, Result};

#[cfg(feature = "vmaf")]
pub mod vmaf;

pub mod simd;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Calculate Peak Signal-to-Noise Ratio (PSNR) between two images
///
/// PSNR is a measure of the quality of a lossy compression/distortion.
/// Higher values indicate better quality (less distortion).
/// Typical values range from 30 dB (low quality) to 50 dB (high quality).
///
/// # Arguments
///
/// * `reference` - Original/reference image data (grayscale, 8-bit)
/// * `distorted` - Compressed/distorted image data (grayscale, 8-bit)
/// * `width` - Image width in pixels
/// * `height` - Image height in pixels
///
/// # Returns
///
/// PSNR value in decibels (dB). Returns `f64::INFINITY` if images are identical.
///
/// # Formula
///
/// PSNR = 10 * log10(MAX^2 / MSE)
/// where MAX = 255 for 8-bit images
/// and MSE = Mean Squared Error
pub fn psnr(reference: &[u8], distorted: &[u8], width: usize, height: usize) -> Result<f64> {
    // Validate dimensions are reasonable (max 16K to prevent overflow)
    const MAX_DIMENSION: usize = 15360;
    if width > MAX_DIMENSION || height > MAX_DIMENSION {
        return Err(BitvueError::InvalidData(format!(
            "Dimensions exceed maximum: {}x{} (max {}x{})",
            width, height, MAX_DIMENSION, MAX_DIMENSION
        )));
    }

    // Prevent overflow in width * height calculation
    let size = width.checked_mul(height).ok_or_else(|| {
        BitvueError::InvalidData(format!(
            "Width * height overflow: {} * {}", width, height
        ))
    })?;

    if reference.len() != size || distorted.len() != size {
        return Err(BitvueError::InvalidData(format!(
            "Image size mismatch: expected {}, got {} and {}",
            size,
            reference.len(),
            distorted.len()
        )));
    }

    // Use SIMD-optimized PSNR by default with runtime CPU feature detection
    // Falls back to scalar implementation automatically if SIMD not available
    simd::psnr_simd(reference, distorted, width, height)
}

/// Calculate Structural Similarity Index (SSIM) between two images
///
/// SSIM is a perceptual metric that considers changes in structural information,
/// luminance, and contrast. It correlates better with human perception than PSNR.
/// Values range from -1 to 1, where 1 indicates perfect similarity.
///
/// Uses SIMD optimization for window statistics computation (4-6x speedup).
///
/// # Arguments
///
/// * `reference` - Original/reference image data (grayscale, 8-bit)
/// * `distorted` - Compressed/distorted image data (grayscale, 8-bit)
/// * `width` - Image width in pixels
/// * `height` - Image height in pixels
///
/// # Returns
///
/// SSIM value between -1 and 1 (typically 0.8-1.0 for good quality).
///
/// # Formula
///
/// SSIM(x, y) = (2*μx*μy + C1)(2*σxy + C2) / (μx^2 + μy^2 + C1)(σx^2 + σy^2 + C2)
/// where:
/// - μx, μy = mean of x and y
/// - σx^2, σy^2 = variance of x and y
/// - σxy = covariance of x and y
/// - C1, C2 = stabilization constants
pub fn ssim(reference: &[u8], distorted: &[u8], width: usize, height: usize) -> Result<f64> {
    // Validate dimensions are reasonable (max 16K to prevent overflow)
    const MAX_DIMENSION: usize = 15360;
    if width > MAX_DIMENSION || height > MAX_DIMENSION {
        return Err(BitvueError::InvalidData(format!(
            "Dimensions exceed maximum: {}x{} (max {}x{})",
            width, height, MAX_DIMENSION, MAX_DIMENSION
        )));
    }

    // Prevent overflow in width * height calculation
    let size = width.checked_mul(height).ok_or_else(|| {
        BitvueError::InvalidData(format!(
            "Width * height overflow: {} * {}", width, height
        ))
    })?;

    if reference.len() != size || distorted.len() != size {
        return Err(BitvueError::InvalidData(format!(
            "Image size mismatch: expected {}, got {} and {}",
            size,
            reference.len(),
            distorted.len()
        )));
    }

    // SSIM constants
    let k1 = 0.01;
    let k2 = 0.03;
    let l = 255.0; // Dynamic range for 8-bit
    let c1 = (k1 * l) * (k1 * l);
    let c2 = (k2 * l) * (k2 * l);

    // Use sliding window approach (8x8 blocks)
    let window_size = 8;
    let mut ssim_sum = 0.0;
    let mut count = 0;

    for y in (0..height).step_by(window_size) {
        for x in (0..width).step_by(window_size) {
            // Calculate window bounds
            let win_width = (window_size).min(width - x);
            let win_height = (window_size).min(height - y);
            let win_size = win_width * win_height;

            if win_size == 0 {
                continue;
            }

            // Calculate start and end indices for this window with overflow protection
            let start = y.checked_mul(width)
                .and_then(|s| s.checked_add(x))
                .ok_or_else(|| BitvueError::InvalidData(
                    "SSIM window start calculation overflow".to_string()
                ))?;

            let end = start.checked_add(win_size)
                .ok_or_else(|| BitvueError::InvalidData(
                    "SSIM window end calculation overflow".to_string()
                ))?;

            // Validate end doesn't exceed data bounds
            if end > reference.len() || end > distorted.len() {
                return Err(BitvueError::InvalidData(
                    format!("SSIM window end {} exceeds data length (ref: {}, dist: {})",
                            end, reference.len(), distorted.len())
                ));
            }

            // Use SIMD-optimized window statistics computation
            let stats = simd::compute_window_stats_simd(reference, distorted, start, end);

            if stats.count == 0 {
                continue;
            }

            // Calculate means using f64 to avoid precision loss
            let n = stats.count as f64;
            let sum_x = stats.sum_x as f64;
            let sum_y = stats.sum_y as f64;
            let sum_xx = stats.sum_xx as f64;
            let sum_yy = stats.sum_yy as f64;
            let sum_xy = stats.sum_xy as f64;

            let mean_x = sum_x / n;
            let mean_y = sum_y / n;

            // Calculate variances and covariance
            let var_x = (sum_xx / n) - (mean_x * mean_x);
            let var_y = (sum_yy / n) - (mean_y * mean_y);
            let cov_xy = (sum_xy / n) - (mean_x * mean_y);

            // Calculate SSIM for this window
            let numerator = (2.0 * mean_x * mean_y + c1) * (2.0 * cov_xy + c2);
            let denominator = (mean_x * mean_x + mean_y * mean_y + c1) * (var_x + var_y + c2);

            let window_ssim = numerator / denominator;
            ssim_sum += window_ssim;
            count += 1;
        }
    }

    // Return mean SSIM across all windows
    Ok(ssim_sum / count as f64)
}

/// YUV frame data with dimensions
pub struct YuvFrame<'a> {
    /// Y plane data
    pub y: &'a [u8],
    /// U plane data
    pub u: &'a [u8],
    /// V plane data
    pub v: &'a [u8],
    /// Luma width
    pub width: usize,
    /// Luma height
    pub height: usize,
    /// Chroma width
    pub chroma_width: usize,
    /// Chroma height
    pub chroma_height: usize,
}

/// Calculate PSNR for YUV frames (Y, U, V planes separately)
///
/// Returns PSNR values for each plane: (Y_PSNR, U_PSNR, V_PSNR)
pub fn psnr_yuv(reference: &YuvFrame, distorted: &YuvFrame) -> Result<(f64, f64, f64)> {
    let y_psnr = psnr(reference.y, distorted.y, reference.width, reference.height)?;
    let u_psnr = psnr(
        reference.u,
        distorted.u,
        reference.chroma_width,
        reference.chroma_height,
    )?;
    let v_psnr = psnr(
        reference.v,
        distorted.v,
        reference.chroma_width,
        reference.chroma_height,
    )?;

    Ok((y_psnr, u_psnr, v_psnr))
}

/// Calculate SSIM for YUV frames (Y, U, V planes separately)
///
/// Returns SSIM values for each plane: (Y_SSIM, U_SSIM, V_SSIM)
pub fn ssim_yuv(reference: &YuvFrame, distorted: &YuvFrame) -> Result<(f64, f64, f64)> {
    let y_ssim = ssim(reference.y, distorted.y, reference.width, reference.height)?;
    let u_ssim = ssim(
        reference.u,
        distorted.u,
        reference.chroma_width,
        reference.chroma_height,
    )?;
    let v_ssim = ssim(
        reference.v,
        distorted.v,
        reference.chroma_width,
        reference.chroma_height,
    )?;

    Ok((y_ssim, u_ssim, v_ssim))
}

/// Multi-threaded batch PSNR computation (Rayon - Rust alternative to OpenMP)
///
/// Computes PSNR for multiple frame pairs in parallel using all available CPU cores.
/// Uses SIMD optimizations automatically per-core.
///
/// # Performance
///
/// - Uses Rayon for work-stealing parallelism (similar to OpenMP)
/// - SIMD vectorization per thread (AVX2/AVX/SSE2/NEON)
/// - Scales well with CPU core count
///
/// # Example
///
/// ```no_run
/// use bitvue_metrics::batch_psnr_parallel;
///
/// let ref_frames = vec![vec![128u8; 1920*1080]; 100];  // 100 frames
/// let dist_frames = vec![vec![130u8; 1920*1080]; 100];
///
/// let scores = batch_psnr_parallel(&ref_frames, &dist_frames, 1920, 1080).unwrap();
/// println!("Average PSNR: {:.2} dB", scores.iter().sum::<f64>() / scores.len() as f64);
/// ```
#[cfg(feature = "parallel")]
pub fn batch_psnr_parallel(
    reference_frames: &[Vec<u8>],
    distorted_frames: &[Vec<u8>],
    width: usize,
    height: usize,
) -> Result<Vec<f64>> {
    if reference_frames.len() != distorted_frames.len() {
        return Err(BitvueError::InvalidData(format!(
            "Frame count mismatch: {} reference vs {} distorted",
            reference_frames.len(),
            distorted_frames.len()
        )));
    }

    // Parallel processing using Rayon (OpenMP-like)
    let scores: Result<Vec<f64>> = reference_frames
        .par_iter()
        .zip(distorted_frames.par_iter())
        .map(|(ref_frame, dist_frame)| {
            // Each thread uses SIMD optimizations
            simd::psnr_simd(ref_frame, dist_frame, width, height)
        })
        .collect();

    scores
}

/// Multi-threaded batch SSIM computation (Rayon - Rust alternative to OpenMP)
///
/// Computes SSIM for multiple frame pairs in parallel using all available CPU cores.
///
/// # Performance
///
/// - Uses Rayon for work-stealing parallelism
/// - Scales well with CPU core count
/// - Ideal for processing large video sequences
#[cfg(feature = "parallel")]
pub fn batch_ssim_parallel(
    reference_frames: &[Vec<u8>],
    distorted_frames: &[Vec<u8>],
    width: usize,
    height: usize,
) -> Result<Vec<f64>> {
    if reference_frames.len() != distorted_frames.len() {
        return Err(BitvueError::InvalidData(format!(
            "Frame count mismatch: {} reference vs {} distorted",
            reference_frames.len(),
            distorted_frames.len()
        )));
    }

    // Parallel processing using Rayon
    let scores: Result<Vec<f64>> = reference_frames
        .par_iter()
        .zip(distorted_frames.par_iter())
        .map(|(ref_frame, dist_frame)| ssim(ref_frame, dist_frame, width, height))
        .collect();

    scores
}

/// Multi-threaded batch YUV PSNR computation
#[cfg(feature = "parallel")]
pub fn batch_psnr_yuv_parallel(
    reference_frames: &[YuvFrame],
    distorted_frames: &[YuvFrame],
) -> Result<Vec<(f64, f64, f64)>> {
    if reference_frames.len() != distorted_frames.len() {
        return Err(BitvueError::InvalidData(format!(
            "Frame count mismatch: {} reference vs {} distorted",
            reference_frames.len(),
            distorted_frames.len()
        )));
    }

    let scores: Result<Vec<(f64, f64, f64)>> = reference_frames
        .par_iter()
        .zip(distorted_frames.par_iter())
        .map(|(ref_frame, dist_frame)| psnr_yuv(ref_frame, dist_frame))
        .collect();

    scores
}

/// Multi-threaded batch YUV SSIM computation
#[cfg(feature = "parallel")]
pub fn batch_ssim_yuv_parallel(
    reference_frames: &[YuvFrame],
    distorted_frames: &[YuvFrame],
) -> Result<Vec<(f64, f64, f64)>> {
    if reference_frames.len() != distorted_frames.len() {
        return Err(BitvueError::InvalidData(format!(
            "Frame count mismatch: {} reference vs {} distorted",
            reference_frames.len(),
            distorted_frames.len()
        )));
    }

    let scores: Result<Vec<(f64, f64, f64)>> = reference_frames
        .par_iter()
        .zip(distorted_frames.par_iter())
        .map(|(ref_frame, dist_frame)| ssim_yuv(ref_frame, dist_frame))
        .collect();

    scores
}

// Fallback stubs when parallel feature is not enabled
#[cfg(not(feature = "parallel"))]
pub fn batch_psnr_parallel(
    _reference_frames: &[Vec<u8>],
    _distorted_frames: &[Vec<u8>],
    _width: usize,
    _height: usize,
) -> Result<Vec<f64>> {
    Err(BitvueError::InvalidData(
        "Parallel processing not enabled. Rebuild with --features parallel".to_string(),
    ))
}

#[cfg(not(feature = "parallel"))]
pub fn batch_ssim_parallel(
    _reference_frames: &[Vec<u8>],
    _distorted_frames: &[Vec<u8>],
    _width: usize,
    _height: usize,
) -> Result<Vec<f64>> {
    Err(BitvueError::InvalidData(
        "Parallel processing not enabled. Rebuild with --features parallel".to_string(),
    ))
}

#[cfg(not(feature = "parallel"))]
pub fn batch_psnr_yuv_parallel(
    _reference_frames: &[YuvFrame],
    _distorted_frames: &[YuvFrame],
) -> Result<Vec<(f64, f64, f64)>> {
    Err(BitvueError::InvalidData(
        "Parallel processing not enabled. Rebuild with --features parallel".to_string(),
    ))
}

#[cfg(not(feature = "parallel"))]
pub fn batch_ssim_yuv_parallel(
    _reference_frames: &[YuvFrame],
    _distorted_frames: &[YuvFrame],
) -> Result<Vec<(f64, f64, f64)>> {
    Err(BitvueError::InvalidData(
        "Parallel processing not enabled. Rebuild with --features parallel".to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_psnr_identical() {
        let reference = vec![128u8; 100];
        let distorted = vec![128u8; 100];

        let result = psnr(&reference, &distorted, 10, 10).unwrap();
        assert!(result.is_infinite());
    }

    #[test]
    fn test_psnr_different() {
        let reference = vec![128u8; 100];
        let mut distorted = vec![128u8; 100];
        distorted[50] = 130; // Small difference

        let result = psnr(&reference, &distorted, 10, 10).unwrap();
        assert!(result.is_finite());
        assert!(result > 40.0); // Should be high PSNR for small difference
    }

    #[test]
    fn test_psnr_size_mismatch() {
        let reference = vec![128u8; 100];
        let distorted = vec![128u8; 50];

        let result = psnr(&reference, &distorted, 10, 10);
        assert!(result.is_err());
    }

    #[test]
    fn test_ssim_identical() {
        let reference = vec![128u8; 64]; // 8x8
        let distorted = vec![128u8; 64];

        let result = ssim(&reference, &distorted, 8, 8).unwrap();
        assert!((result - 1.0).abs() < 0.001); // Should be very close to 1.0
    }

    #[test]
    fn test_ssim_similar() {
        let reference = vec![128u8; 64]; // 8x8
        let mut distorted = vec![128u8; 64];
        distorted[30] = 130; // Small difference

        let result = ssim(&reference, &distorted, 8, 8).unwrap();
        assert!(result > 0.95); // Should be high SSIM for small difference
        assert!(result < 1.0);
    }

    #[test]
    fn test_ssim_different() {
        let reference = vec![128u8; 256]; // 16x16
        let distorted = vec![200u8; 256]; // Very different

        let result = ssim(&reference, &distorted, 16, 16).unwrap();
        assert!(result < 1.0); // Should be lower SSIM for different images
        assert!(result > 0.0); // But still positive
    }

    #[test]
    fn test_ssim_size_mismatch() {
        let reference = vec![128u8; 100];
        let distorted = vec![128u8; 50];

        let result = ssim(&reference, &distorted, 10, 10);
        assert!(result.is_err());
    }

    #[test]
    fn test_psnr_yuv() {
        // Create simple YUV frames (4:2:0)
        let width = 16;
        let height = 16;
        let chroma_width = 8;
        let chroma_height = 8;

        let ref_y = vec![128u8; width * height];
        let ref_u = vec![128u8; chroma_width * chroma_height];
        let ref_v = vec![128u8; chroma_width * chroma_height];

        let dist_y = vec![130u8; width * height];
        let dist_u = vec![130u8; chroma_width * chroma_height];
        let dist_v = vec![130u8; chroma_width * chroma_height];

        let reference = YuvFrame {
            y: &ref_y,
            u: &ref_u,
            v: &ref_v,
            width,
            height,
            chroma_width,
            chroma_height,
        };

        let distorted = YuvFrame {
            y: &dist_y,
            u: &dist_u,
            v: &dist_v,
            width,
            height,
            chroma_width,
            chroma_height,
        };

        let result = psnr_yuv(&reference, &distorted).unwrap();

        assert!(result.0.is_finite());
        assert!(result.1.is_finite());
        assert!(result.2.is_finite());
    }

    #[test]
    fn test_ssim_yuv() {
        // Create simple YUV frames (4:2:0)
        let width = 16;
        let height = 16;
        let chroma_width = 8;
        let chroma_height = 8;

        let ref_y = vec![128u8; width * height];
        let ref_u = vec![128u8; chroma_width * chroma_height];
        let ref_v = vec![128u8; chroma_width * chroma_height];

        let dist_y = vec![128u8; width * height];
        let dist_u = vec![128u8; chroma_width * chroma_height];
        let dist_v = vec![128u8; chroma_width * chroma_height];

        let reference = YuvFrame {
            y: &ref_y,
            u: &ref_u,
            v: &ref_v,
            width,
            height,
            chroma_width,
            chroma_height,
        };

        let distorted = YuvFrame {
            y: &dist_y,
            u: &dist_u,
            v: &dist_v,
            width,
            height,
            chroma_width,
            chroma_height,
        };

        let result = ssim_yuv(&reference, &distorted).unwrap();

        // Identical frames should have SSIM close to 1.0
        assert!((result.0 - 1.0).abs() < 0.01);
        assert!((result.1 - 1.0).abs() < 0.01);
        assert!((result.2 - 1.0).abs() < 0.01);
    }
}
