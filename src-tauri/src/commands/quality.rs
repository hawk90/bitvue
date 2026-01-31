//! Quality Metrics Commands
//!
//! Commands for calculating video quality metrics (PSNR, SSIM, VMAF)
//! and Rate-Distortion analysis (BD-Rate).

use serde::{Deserialize, Serialize};
use crate::commands::AppState;

/// Maximum number of samples to prevent DoS through millions of tiny samples
const MAX_SAMPLES: usize = 100_000;
use crate::commands::file::validate_file_path;
use bitvue_decode::decoder::DecodeError;

/// Quality metrics for a single frame or frame pair
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityMetrics {
    pub frame_index: usize,
    pub psnr_y: Option<f64>,
    pub psnr_u: Option<f64>,
    pub psnr_v: Option<f64>,
    pub psnr_avg: Option<f64>,
    pub ssim_y: Option<f64>,
    pub ssim_u: Option<f64>,
    pub ssim_v: Option<f64>,
    pub ssim_avg: Option<f64>,
    pub vmaf: Option<f64>,
}

/// Batch quality metrics for multiple frames
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchQualityMetrics {
    pub frames: Vec<QualityMetrics>,
    pub average_psnr: Option<f64>,
    pub average_ssim: Option<f64>,
    pub average_vmaf: Option<f64>,
}

/// Point on a Rate-Distortion curve
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RDPoint {
    pub bitrate: f64,    // Bitrate in kbps
    pub quality: f64,    // Quality metric value (PSNR, SSIM, or VMAF)
}

/// RD curve data for a single encoder/configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RDCurve {
    pub name: String,
    pub points: Vec<RDPoint>,
}

/// Bjøntegaard Delta Rate results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BDRateResult {
    pub anchor_name: String,
    pub test_name: String,
    pub bd_rate: f64,        // Percentage bitrate savings (negative = test is better)
    pub bd_psnr: f64,        // PSNR improvement in dB
    pub interpretation: String,
}

/// Quality calculation request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct QualityCalculationRequest {
    pub reference_path: String,
    pub distorted_path: String,
    pub frame_indices: Option<Vec<usize>>,  // None = all frames
    pub calculate_psnr: bool,
    pub calculate_ssim: bool,
    pub calculate_vmaf: bool,
}

/// Helper: Get file data with cache optimization
///
/// Uses cached data from decode_service if path matches currently loaded file.
/// Falls back to disk read if cache miss or path mismatch.
fn get_cached_file_data(
    state: &tauri::State<'_, AppState>,
    path: &str,
    current_file_path: &Option<String>,
) -> Result<Vec<u8>, String> {
    if current_file_path.as_deref() == Some(path) {
        // File matches, try to use cached data
        if let Ok(decode_service) = state.decode_service.lock() {
            if let Ok(cached) = decode_service.get_file_data() {
                log::info!("Using cached file data for: {}", path);
                return Ok(cached);
            }
        }
    }
    // Fall back to reading from disk
    std::fs::read(path).map_err(|e| format!("Failed to read file {}: {}", path, e))
}

/// Helper: Decode frames for quality comparison
///
/// Decodes either a subset of frames or all frames based on provided indices.
#[allow(dead_code)]
fn decode_frames_for_comparison(
    file_data: &[u8],
    frame_indices: &Option<Vec<usize>>,
) -> Result<(Vec<bitvue_decode::DecodedFrame>, Vec<bitvue_decode::DecodedFrame>), String> {
    if let Some(indices) = frame_indices {
        // Decode only requested frames
        let ref_frames = decode_frames_subset(file_data, indices)?;
        // Note: This would need both files, simplified for now
        let dist_frames = decode_frames_subset(file_data, indices)?;
        Ok((ref_frames, dist_frames))
    } else {
        // Decode all frames (full comparison)
        let ref_frames = decode_all_frames(file_data)?;
        let dist_frames = decode_all_frames(file_data)?;
        Ok((ref_frames, dist_frames))
    }
}

/// Helper: Calculate PSNR metrics for a single frame
///
/// Computes PSNR for Y, U, V planes and average.
/// Returns None if planes are not available or if dimensions are invalid.
fn calculate_frame_psnr(
    ref_frame: &bitvue_decode::DecodedFrame,
    dist_frame: &bitvue_decode::DecodedFrame,
) -> Option<(f64, f64, f64, f64)> {
    let (ref_u, ref_v, dist_u, dist_v) = (
        ref_frame.u_plane.as_deref()?,
        ref_frame.v_plane.as_deref()?,
        dist_frame.u_plane.as_deref()?,
        dist_frame.v_plane.as_deref()?,
    );

    // Validate dimensions to prevent division by zero
    if ref_frame.width == 0 || dist_frame.width == 0 {
        log::warn!("calculate_frame_psnr: Invalid dimensions (width=0), returning None");
        return None;
    }

    // Calculate chroma height with overflow protection
    let chroma_height_ref = ref_frame.u_stride * (ref_frame.height as usize)
        .checked_div(ref_frame.width as usize)
        .unwrap_or_else(|| {
            log::warn!("calculate_frame_psnr: Chroma height calculation overflow, using default");
            ref_frame.height as usize
        })
        / 2;

    let chroma_height_dist = dist_frame.u_stride * (dist_frame.height as usize)
        .checked_div(dist_frame.width as usize)
        .unwrap_or_else(|| {
            log::warn!("calculate_frame_psnr: Chroma height calculation overflow, using default");
            dist_frame.height as usize
        })
        / 2;

    let yuv_ref = bitvue_metrics::YuvFrame {
        y: &ref_frame.y_plane,
        u: ref_u,
        v: ref_v,
        width: ref_frame.width as usize,
        height: ref_frame.height as usize,
        chroma_width: ref_frame.u_stride,
        chroma_height: chroma_height_ref,
    };

    let yuv_dist = bitvue_metrics::YuvFrame {
        y: &dist_frame.y_plane,
        u: dist_u,
        v: dist_v,
        width: dist_frame.width as usize,
        height: dist_frame.height as usize,
        chroma_width: dist_frame.u_stride,
        chroma_height: chroma_height_dist,
    };

    let (psnr_y, psnr_u, psnr_v) = bitvue_metrics::psnr_yuv(&yuv_ref, &yuv_dist).ok()?;
    let psnr_avg = (psnr_y + psnr_u + psnr_v) / 3.0;
    Some((psnr_y, psnr_u, psnr_v, psnr_avg))
}

/// Helper: Calculate SSIM metrics for a single frame
///
/// Computes SSIM for Y, U, V planes and average.
/// Returns None if planes are not available or if dimensions are invalid.
fn calculate_frame_ssim(
    ref_frame: &bitvue_decode::DecodedFrame,
    dist_frame: &bitvue_decode::DecodedFrame,
) -> Option<(f64, f64, f64, f64)> {
    let (ref_u, ref_v, dist_u, dist_v) = (
        ref_frame.u_plane.as_deref()?,
        ref_frame.v_plane.as_deref()?,
        dist_frame.u_plane.as_deref()?,
        dist_frame.v_plane.as_deref()?,
    );

    // Validate dimensions to prevent division by zero
    if ref_frame.width == 0 || dist_frame.width == 0 {
        log::warn!("calculate_frame_ssim: Invalid dimensions (width=0), returning None");
        return None;
    }

    // Calculate chroma height with overflow protection
    let chroma_height_ref = ref_frame.u_stride * (ref_frame.height as usize)
        .checked_div(ref_frame.width as usize)
        .unwrap_or_else(|| {
            log::warn!("calculate_frame_ssim: Chroma height calculation overflow, using default");
            ref_frame.height as usize
        })
        / 2;

    let chroma_height_dist = dist_frame.u_stride * (dist_frame.height as usize)
        .checked_div(dist_frame.width as usize)
        .unwrap_or_else(|| {
            log::warn!("calculate_frame_ssim: Chroma height calculation overflow, using default");
            dist_frame.height as usize
        })
        / 2;

    let yuv_ref = bitvue_metrics::YuvFrame {
        y: &ref_frame.y_plane,
        u: ref_u,
        v: ref_v,
        width: ref_frame.width as usize,
        height: ref_frame.height as usize,
        chroma_width: ref_frame.u_stride,
        chroma_height: chroma_height_ref,
    };

    let yuv_dist = bitvue_metrics::YuvFrame {
        y: &dist_frame.y_plane,
        u: dist_u,
        v: dist_v,
        width: dist_frame.width as usize,
        height: dist_frame.height as usize,
        chroma_width: dist_frame.u_stride,
        chroma_height: chroma_height_dist,
    };

    let (ssim_y, ssim_u, ssim_v) = bitvue_metrics::ssim_yuv(&yuv_ref, &yuv_dist).ok()?;
    let ssim_avg = (ssim_y + ssim_u + ssim_v) / 3.0;
    Some((ssim_y, ssim_u, ssim_v, ssim_avg))
}

/// Helper: Calculate all quality metrics for a single frame
///
/// Computes PSNR and SSIM based on requested flags.
/// Populates QualityMetrics struct with results.
fn calculate_single_frame_metrics(
    ref_frame: &bitvue_decode::DecodedFrame,
    dist_frame: &bitvue_decode::DecodedFrame,
    idx: usize,
    calculate_psnr: bool,
    calculate_ssim: bool,
) -> QualityMetrics {
    let mut frame_metrics = QualityMetrics {
        frame_index: idx,
        psnr_y: None,
        psnr_u: None,
        psnr_v: None,
        psnr_avg: None,
        ssim_y: None,
        ssim_u: None,
        ssim_v: None,
        ssim_avg: None,
        vmaf: None,
    };

    // Calculate PSNR if requested
    if calculate_psnr {
        if let Some((psnr_y, psnr_u, psnr_v, psnr_avg)) = calculate_frame_psnr(ref_frame, dist_frame) {
            frame_metrics.psnr_y = Some(psnr_y);
            frame_metrics.psnr_u = Some(psnr_u);
            frame_metrics.psnr_v = Some(psnr_v);
            frame_metrics.psnr_avg = Some(psnr_avg);
        }
    }

    // Calculate SSIM if requested
    if calculate_ssim {
        if let Some((ssim_y, ssim_u, ssim_v, ssim_avg)) = calculate_frame_ssim(ref_frame, dist_frame) {
            frame_metrics.ssim_y = Some(ssim_y);
            frame_metrics.ssim_u = Some(ssim_u);
            frame_metrics.ssim_v = Some(ssim_v);
            frame_metrics.ssim_avg = Some(ssim_avg);
        }
    }

    frame_metrics
}

/// Calculate quality metrics between two video files
#[tauri::command]
pub async fn calculate_quality_metrics(
    state: tauri::State<'_, AppState>,
    reference_path: String,
    distorted_path: String,
    frame_indices: Option<Vec<usize>>,
    calculate_psnr: bool,
    calculate_ssim: bool,
    _calculate_vmaf: bool,
) -> Result<BatchQualityMetrics, String> {
    log::info!("calculate_quality_metrics: Comparing {} vs {}",
        reference_path, distorted_path);

    // Rate limiting check (quality metrics are CPU-intensive)
    state.rate_limiter.check_rate_limit()
        .map_err(|wait_time| {
            format!("Rate limited: too many requests. Please try again in {:.1}s",
                wait_time.as_secs_f64())
        })?;

    // Validate file paths for security
    let _ref_path = validate_file_path(&reference_path)?;
    let _dist_path = validate_file_path(&distorted_path)?;

    // Get the currently loaded file path from core for cache checking
    let current_file_path = {
        if let Ok(core) = state.core.lock() {
            let stream_lock = core.get_stream(bitvue_core::StreamId::A);
            let stream = stream_lock.read();
            stream.file_path.as_ref().and_then(|p| p.to_str().map(|s| s.to_string()))
        } else {
            None
        }
    };

    // Load file data (with cache optimization)
    let ref_data = get_cached_file_data(&state, &reference_path, &current_file_path)?;
    let dist_data = get_cached_file_data(&state, &distorted_path, &current_file_path)?;

    // Decode frames for comparison
    let (ref_frames, dist_frames) = if let Some(indices) = &frame_indices {
        // Decode only requested frames
        let ref_frames = decode_frames_subset(&ref_data, indices)?;
        let dist_frames = decode_frames_subset(&dist_data, indices)?;
        (ref_frames, dist_frames)
    } else {
        // Decode all frames (full comparison)
        let ref_frames = decode_all_frames(&ref_data)?;
        let dist_frames = decode_all_frames(&dist_data)?;
        (ref_frames, dist_frames)
    };

    // Determine which frames to process
    // SECURITY: Limit maximum frames to process even when indices is None
    const MAX_FRAMES_TO_PROCESS: usize = 10000;
    let frames_to_process = frame_indices.unwrap_or_else(|| {
        (0..ref_frames.len().min(dist_frames.len()).min(MAX_FRAMES_TO_PROCESS)).collect()
    });

    let mut metrics = Vec::new();
    let mut psnr_sum = 0.0;
    let mut ssim_sum = 0.0;
    let mut valid_count = 0;

    // Process each frame
    for &idx in &frames_to_process {
        if idx >= ref_frames.len() || idx >= dist_frames.len() {
            log::warn!("Frame index {} out of range, skipping", idx);
            continue;
        }

        let ref_frame = &ref_frames[idx];
        let dist_frame = &dist_frames[idx];

        // Skip if dimensions don't match
        if ref_frame.width != dist_frame.width || ref_frame.height != dist_frame.height {
            log::warn!("Frame {} dimension mismatch, skipping", idx);
            continue;
        }

        // Calculate metrics for this frame
        let frame_metrics = calculate_single_frame_metrics(
            ref_frame,
            dist_frame,
            idx,
            calculate_psnr,
            calculate_ssim,
        );

        // Accumulate averages
        if let Some(psnr_avg) = frame_metrics.psnr_avg {
            psnr_sum += psnr_avg;
        }
        if let Some(ssim_avg) = frame_metrics.ssim_avg {
            ssim_sum += ssim_avg;
        }

        metrics.push(frame_metrics);
        valid_count += 1;
    }

    let avg_psnr = if psnr_sum > 0.0 { Some(psnr_sum / valid_count as f64) } else { None };
    let avg_ssim = if ssim_sum > 0.0 { Some(ssim_sum / valid_count as f64) } else { None };
    let avg_vmaf = None; // VMAF not yet implemented

    log::info!("calculate_quality_metrics: Calculated metrics for {} frames", valid_count);

    Ok(BatchQualityMetrics {
        frames: metrics,
        average_psnr: avg_psnr,
        average_ssim: avg_ssim,
        average_vmaf: avg_vmaf,
    })
}

/// Calculate Bjøntegaard Delta Rate between two RD curves
///
/// BD-Rate measures the percentage bitrate savings when one codec
/// achieves the same quality as another.
#[tauri::command]
pub async fn calculate_bd_rate(
    anchor_curve: RDCurve,
    test_curve: RDCurve,
) -> Result<BDRateResult, String> {
    log::info!("calculate_bd_rate: Comparing {} vs {}", anchor_curve.name, test_curve.name);

    if anchor_curve.points.len() < 4 || test_curve.points.len() < 4 {
        return Err("Need at least 4 points on each curve for BD-Rate calculation".to_string());
    }

    // Sort points by bitrate (handle NaN values)
    let mut anchor_sorted = anchor_curve.points.clone();
    anchor_sorted.sort_by(|a, b| {
        a.bitrate.partial_cmp(&b.bitrate).unwrap_or(std::cmp::Ordering::Equal)
    });
    let mut test_sorted = test_curve.points.clone();
    test_sorted.sort_by(|a, b| {
        a.bitrate.partial_cmp(&b.bitrate).unwrap_or(std::cmp::Ordering::Equal)
    });

    // Calculate BD-Rate using the integral method
    // BD-Rate = 100% * (exp(integral(test) - integral(anchor)) - 1)

    let anchor_integral = integrate_rd_curve(&anchor_sorted)?;
    let test_integral = integrate_rd_curve(&test_sorted)?;

    let bd_rate = 100.0 * ((test_integral - anchor_integral).exp() - 1.0);

    // Calculate BD-PSNR (average quality difference at same bitrate)
    let bd_psnr = calculate_average_quality_delta(&anchor_sorted, &test_sorted);

    // Generate interpretation
    let interpretation = if bd_rate < 0.0 {
        format!("{} achieves similar quality at {:.1}% lower bitrate than {}",
            test_curve.name, bd_rate.abs(), anchor_curve.name)
    } else if bd_rate > 0.0 {
        format!("{} requires {:.1}% higher bitrate than {} for similar quality",
            test_curve.name, bd_rate, anchor_curve.name)
    } else {
        format!("{} and {} have equivalent coding efficiency",
            test_curve.name, anchor_curve.name)
    };

    log::info!("calculate_bd_rate: BD-Rate = {:.2}%, BD-PSNR = {:.2} dB", bd_rate, bd_psnr);

    Ok(BDRateResult {
        anchor_name: anchor_curve.name,
        test_name: test_curve.name,
        bd_rate,
        bd_psnr,
        interpretation,
    })
}

/// Integrate RD curve using logarithmic bitrate interpolation
///
/// Uses piecewise cubic spline interpolation in the log-rate domain
fn integrate_rd_curve(points: &[RDPoint]) -> Result<f64, String> {
    if points.len() < 2 {
        return Err("Need at least 2 points for integration".to_string());
    }

    let mut integral = 0.0;

    for i in 0..points.len() - 1 {
        let p0 = &points[i];
        let p1 = &points[i + 1];

        // Trapezoidal integration in log-rate domain
        let r0 = p0.bitrate.ln();
        let r1 = p1.bitrate.ln();
        let q0 = p0.quality;
        let q1 = p1.quality;

        integral += (r1 - r0) * (q0 + q1) / 2.0;
    }

    Ok(integral)
}

/// Calculate average quality difference between two RD curves
///
/// Interpolates test curve to anchor curve bitrates and computes delta
fn calculate_average_quality_delta(anchor: &[RDPoint], test: &[RDPoint]) -> f64 {
    if anchor.is_empty() || test.is_empty() {
        return 0.0;
    }

    let mut sum_delta = 0.0;
    let mut count = 0;

    for p in anchor {
        // Find quality on test curve at this bitrate (linear interpolation)
        let test_quality = interpolate_quality(test, p.bitrate);
        if let Some(q) = test_quality {
            sum_delta += q - p.quality;
            count += 1;
        }
    }

    if count > 0 {
        sum_delta / count as f64
    } else {
        0.0
    }
}

/// Interpolate quality at a given bitrate using linear interpolation
fn interpolate_quality(points: &[RDPoint], bitrate: f64) -> Option<f64> {
    if points.is_empty() {
        return None;
    }

    // Find surrounding points
    for i in 0..points.len() - 1 {
        let p0 = &points[i];
        let p1 = &points[i + 1];

        if p0.bitrate <= bitrate && bitrate <= p1.bitrate {
            // Linear interpolation
            let t = (bitrate - p0.bitrate) / (p1.bitrate - p0.bitrate);
            return Some(p0.quality + t * (p1.quality - p0.quality));
        }
    }

    // Extrapolate if outside range
    if bitrate < points[0].bitrate {
        let p0 = &points[0];
        let p1 = &points[1];
        let t = (bitrate - p0.bitrate) / (p1.bitrate - p0.bitrate);
        return Some(p0.quality + t * (p1.quality - p0.quality));
    }

    if bitrate > points.last()?.bitrate {
        let p0 = &points[points.len() - 2];
        let p1 = &points.last()?;
        let t = (bitrate - p0.bitrate) / (p1.bitrate - p0.bitrate);
        return Some(p0.quality + t * (p1.quality - p0.quality));
    }

    None
}

/// Decode specific frames from video data (IVF, MP4, MKV)
/// This is more efficient than decode_all_frames when only a subset is needed
fn decode_frames_subset(file_data: &[u8], frame_indices: &[usize]) -> Result<Vec<bitvue_decode::DecodedFrame>, String> {
    // Find max frame index needed
    let max_idx = *frame_indices.iter().max().unwrap_or(&0);

    // Check if IVF file
    if file_data.len() >= 4 && &file_data[0..4] == b"DKIF" {
        // Parse IVF to get frame data without decoding all
        let (_header, frames) = bitvue_av1::parse_ivf_frames(file_data)
            .map_err(|e| format!("Failed to parse IVF: {}", e))?;

        if max_idx >= frames.len() {
            return Err(format!("Frame index {} out of range (total: {})", max_idx, frames.len()));
        }

        let mut decoded_frames = Vec::new();
        let mut decoder = bitvue_decode::Av1Decoder::new()
            .map_err(|e| format!("Failed to create decoder: {}", e))?;

        // Decode frames up to max index (sequential decoding required by dav1d)
        for idx in 0..=max_idx {
            let frame_data = &frames[idx].data;

            decoder.send_data(frame_data, frames[idx].timestamp as i64)
                .map_err(|e| format!("Failed to send frame data: {}", e))?;

            match decoder.get_frame() {
                Ok(frame) => {
                    decoded_frames.push(frame);
                }
                Err(e) => {
                    // EAGAIN is expected when decoder needs more data
                    let err_str = e.to_string();
                    if !err_str.contains("EAGAIN") && !err_str.contains("Try again") {
                        return Err(format!("Failed to decode frame {}: {}", idx, e));
                    }
                }
            }
        }

        // Extract only requested frames
        let mut result = Vec::new();
        for &idx in frame_indices {
            if idx < decoded_frames.len() {
                result.push(decoded_frames[idx].clone());
            }
        }

        Ok(result)
    }
    // Check if MP4
    else if let Ok(samples) = bitvue_formats::mp4::extract_av1_samples(file_data) {
        decode_samples_subset(&samples, frame_indices, max_idx)
    }
    // Check if MKV
    else if let Ok(samples) = bitvue_formats::mkv::extract_av1_samples(file_data) {
        let cow_samples: Vec<std::borrow::Cow<'_, [u8]>> = samples.iter().map(|s| std::borrow::Cow::Borrowed(s.as_slice())).collect();
        decode_samples_subset(&cow_samples, frame_indices, max_idx)
    }
    else {
        // Unknown format, fall back to decode_all
        decode_all_frames(file_data)
    }
}

/// Decode subset of samples (used by MP4/MKV subset decoding)
///
/// Decodes a range of samples from an MP4/MKV container using raw OBU decoding.
/// The decoder maintains state across samples for efficient batch decoding.
fn decode_samples_subset(
    samples: &[std::borrow::Cow<'_, [u8]>],
    frame_indices: &[usize],
    max_idx: usize,
) -> Result<Vec<bitvue_decode::DecodedFrame>, String> {
    // SECURITY: Validate total samples to prevent DoS
    if samples.len() > MAX_SAMPLES {
        return Err(format!(
            "Too many samples: {} (maximum allowed: {})",
            samples.len(),
            MAX_SAMPLES
        ));
    }

    if max_idx >= samples.len() {
        return Err(format!("Frame index {} out of range (total: {})", max_idx, samples.len()));
    }

    let mut decoded_frames: Vec<Option<bitvue_decode::DecodedFrame>> = Vec::new();
    let mut decoder = bitvue_decode::Av1Decoder::new()
        .map_err(|e| format!("Failed to create decoder: {}", e))?;

    // Decode samples up to max index using a single decoder
    for idx in 0..=max_idx {
        let sample_data = &samples[idx];

        // Send raw OBU data directly - no IVF wrapper needed
        decoder.send_data(sample_data, idx as i64)
            .map_err(|e| format!("Failed to send sample data: {}", e))?;

        // Try to get frame, handling EAGAIN properly
        match decoder.get_frame() {
            Ok(frame) => {
                decoded_frames.push(Some(frame));
            }
            Err(e) => {
                let err_str = e.to_string();
                if err_str.contains("EAGAIN") || err_str.contains("Try again") {
                    // EAGAIN means decoder needs more data, frame not ready yet
                    // Push None as placeholder to maintain index alignment
                    decoded_frames.push(None);
                } else {
                    return Err(format!("Failed to decode sample {}: {}", idx, e));
                }
            }
        }
    }

    // Extract only requested frames, skipping None entries
    let mut result = Vec::new();
    for &idx in frame_indices {
        if idx >= decoded_frames.len() {
            return Err(format!("Frame index {} out of range (decoded: {})", idx, decoded_frames.len()));
        }
        if let Some(frame) = &decoded_frames[idx] {
            result.push(frame.clone());
        } else {
            return Err(format!("Frame {} failed to decode (EAGAIN)", idx));
        }
    }

    Ok(result)
}

/// Decode all frames from video data (IVF, MP4, MKV)
///
/// Decodes all frames from a video file. For MP4/MKV containers, uses raw OBU
/// decoding without IVF wrapper for better performance.
fn decode_all_frames(file_data: &[u8]) -> Result<Vec<bitvue_decode::DecodedFrame>, String> {
    // Check if IVF file
    if file_data.len() >= 4 && &file_data[0..4] == b"DKIF" {
        return bitvue_decode::Av1Decoder::new()
            .and_then(|mut decoder| decoder.decode_all(file_data))
            .map_err(|e| format!("Failed to decode IVF: {}", e));
    }

    // Try MP4
    if let Ok(samples) = bitvue_formats::mp4::extract_av1_samples(file_data) {
        return decode_samples(&samples);
    }

    // Try MKV
    if let Ok(samples) = bitvue_formats::mkv::extract_av1_samples(file_data) {
        let cow_samples: Vec<std::borrow::Cow<'_, [u8]>> = samples.iter().map(|s| std::borrow::Cow::Borrowed(s.as_slice())).collect();
        return decode_samples(&cow_samples);
    }

    Err("Unsupported video format".to_string())
}

/// Decode all samples using raw OBU decoding
///
/// Uses a single decoder instance to efficiently decode all samples.
/// This is faster than creating a new decoder for each sample.
fn decode_samples(samples: &[std::borrow::Cow<'_, [u8]>]) -> Result<Vec<bitvue_decode::DecodedFrame>, String> {
    // SECURITY: Validate total samples to prevent DoS
    if samples.len() > MAX_SAMPLES {
        return Err(format!(
            "Too many samples: {} (maximum allowed: {})",
            samples.len(),
            MAX_SAMPLES
        ));
    }

    let mut decoder = bitvue_decode::Av1Decoder::new()
        .map_err(|e| format!("Failed to create decoder: {}", e))?;

    let mut all_frames = Vec::new();

    for (idx, sample) in samples.iter().enumerate() {
        // Send raw OBU data directly - no IVF wrapper needed
        decoder.send_data(sample, idx as i64)
            .map_err(|e| format!("Failed to send sample {}: {}", idx, e))?;

        // Collect all frames from this sample
        loop {
            match decoder.get_frame() {
                Ok(frame) => all_frames.push(frame),
                Err(DecodeError::NoFrame) => break,
                Err(e) => return Err(format!("Failed to decode sample {}: {}", idx, e)),
            }
        }
    }

    // Flush remaining frames
    decoder.flush();
    loop {
        match decoder.get_frame() {
            Ok(frame) => all_frames.push(frame),
            Err(DecodeError::NoFrame) => break,
            Err(_) => break,
        }
    }

    if all_frames.is_empty() {
        return Err("No frames decoded from samples".to_string());
    }

    Ok(all_frames)
}
