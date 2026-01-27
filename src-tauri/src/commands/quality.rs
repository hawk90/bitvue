//! Quality Metrics Commands
//!
//! Commands for calculating video quality metrics (PSNR, SSIM, VMAF)
//! and Rate-Distortion analysis (BD-Rate).

use serde::{Deserialize, Serialize};
use crate::commands::AppState;

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

/// Calculate quality metrics between two video files
#[tauri::command]
pub async fn calculate_quality_metrics(
    _state: tauri::State<'_, AppState>,
    reference_path: String,
    distorted_path: String,
    frame_indices: Option<Vec<usize>>,
    calculate_psnr: bool,
    calculate_ssim: bool,
    _calculate_vmaf: bool,
) -> Result<BatchQualityMetrics, String> {
    log::info!("calculate_quality_metrics: Comparing {} vs {}",
        reference_path, distorted_path);

    // Read both files
    let ref_data = std::fs::read(&reference_path)
        .map_err(|e| format!("Failed to read reference file: {}", e))?;
    let dist_data = std::fs::read(&distorted_path)
        .map_err(|e| format!("Failed to read distorted file: {}", e))?;

    // Decode both videos
    let ref_frames = decode_all_frames(&ref_data)?;
    let dist_frames = decode_all_frames(&dist_data)?;

    // Determine which frames to process
    let frames_to_process = frame_indices.unwrap_or_else(|| {
        (0..ref_frames.len().min(dist_frames.len())).collect()
    });

    let mut metrics = Vec::new();
    let mut psnr_sum = 0.0;
    let mut ssim_sum = 0.0;
    let vmaf_sum: f64 = 0.0;
    let mut valid_count = 0;

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
            if let (Some(ref_u), Some(ref_v),
                 Some(dist_u), Some(dist_v)) = (
                ref_frame.u_plane.as_deref(), ref_frame.v_plane.as_deref(),
                dist_frame.u_plane.as_deref(), dist_frame.v_plane.as_deref(),
            ) {
                let yuv_ref = bitvue_metrics::YuvFrame {
                    y: &ref_frame.y_plane,
                    u: ref_u,
                    v: ref_v,
                    width: ref_frame.width as usize,
                    height: ref_frame.height as usize,
                    chroma_width: ref_frame.u_stride,
                    chroma_height: ref_frame.u_stride * ref_frame.height as usize / ref_frame.width as usize / 2,
                };

                let yuv_dist = bitvue_metrics::YuvFrame {
                    y: &dist_frame.y_plane,
                    u: dist_u,
                    v: dist_v,
                    width: dist_frame.width as usize,
                    height: dist_frame.height as usize,
                    chroma_width: dist_frame.u_stride,
                    chroma_height: dist_frame.u_stride * dist_frame.height as usize / dist_frame.width as usize / 2,
                };

                if let Ok((psnr_y, psnr_u, psnr_v)) = bitvue_metrics::psnr_yuv(&yuv_ref, &yuv_dist) {
                    frame_metrics.psnr_y = Some(psnr_y);
                    frame_metrics.psnr_u = Some(psnr_u);
                    frame_metrics.psnr_v = Some(psnr_v);
                    frame_metrics.psnr_avg = Some((psnr_y + psnr_u + psnr_v) / 3.0);
                    psnr_sum += frame_metrics.psnr_avg.unwrap();
                }
            }
        }

        // Calculate SSIM if requested
        if calculate_ssim {
            if let (Some(ref_u), Some(ref_v),
                 Some(dist_u), Some(dist_v)) = (
                ref_frame.u_plane.as_deref(), ref_frame.v_plane.as_deref(),
                dist_frame.u_plane.as_deref(), dist_frame.v_plane.as_deref(),
            ) {
                let yuv_ref = bitvue_metrics::YuvFrame {
                    y: &ref_frame.y_plane,
                    u: ref_u,
                    v: ref_v,
                    width: ref_frame.width as usize,
                    height: ref_frame.height as usize,
                    chroma_width: ref_frame.u_stride,
                    chroma_height: ref_frame.u_stride * ref_frame.height as usize / ref_frame.width as usize / 2,
                };

                let yuv_dist = bitvue_metrics::YuvFrame {
                    y: &dist_frame.y_plane,
                    u: dist_u,
                    v: dist_v,
                    width: dist_frame.width as usize,
                    height: dist_frame.height as usize,
                    chroma_width: dist_frame.u_stride,
                    chroma_height: dist_frame.u_stride * dist_frame.height as usize / dist_frame.width as usize / 2,
                };

                if let Ok((ssim_y, ssim_u, ssim_v)) = bitvue_metrics::ssim_yuv(&yuv_ref, &yuv_dist) {
                    frame_metrics.ssim_y = Some(ssim_y);
                    frame_metrics.ssim_u = Some(ssim_u);
                    frame_metrics.ssim_v = Some(ssim_v);
                    frame_metrics.ssim_avg = Some((ssim_y + ssim_u + ssim_v) / 3.0);
                    ssim_sum += frame_metrics.ssim_avg.unwrap();
                }
            }
        }

        // VMAF calculation (if feature enabled)
        // TODO: Add vmaf feature when libvmaf integration is ready
        if _calculate_vmaf {
            // VMAF requires both frames to be passed
            // This is a placeholder - actual VMAF calculation requires libvmaf
            frame_metrics.vmaf = None;  // TODO: Implement VMAF
        }

        metrics.push(frame_metrics);
        valid_count += 1;
    }

    let avg_psnr = if psnr_sum > 0.0 { Some(psnr_sum / valid_count as f64) } else { None };
    let avg_ssim = if ssim_sum > 0.0 { Some(ssim_sum / valid_count as f64) } else { None };
    let avg_vmaf = if vmaf_sum > 0.0 { Some(vmaf_sum / valid_count as f64) } else { None };

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

    // Sort points by bitrate
    let mut anchor_sorted = anchor_curve.points.clone();
    anchor_sorted.sort_by(|a, b| a.bitrate.partial_cmp(&b.bitrate).unwrap());
    let mut test_sorted = test_curve.points.clone();
    test_sorted.sort_by(|a, b| a.bitrate.partial_cmp(&b.bitrate).unwrap());

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

/// Decode all frames from video data (IVF, MP4, MKV)
fn decode_all_frames(file_data: &[u8]) -> Result<Vec<bitvue_decode::DecodedFrame>, String> {
    // Check if IVF file
    if file_data.len() >= 4 && &file_data[0..4] == b"DKIF" {
        return bitvue_decode::Av1Decoder::new()
            .and_then(|mut decoder| decoder.decode_all(file_data))
            .map_err(|e| format!("Failed to decode IVF: {}", e));
    }

    // Try MP4
    if let Ok(samples) = bitvue_formats::mp4::extract_av1_samples(file_data) {
        let mut frames = Vec::new();
        for sample in samples {
            let ivf_data = create_ivf_wrapper(&sample);
            if let Ok(mut decoder) = bitvue_decode::Av1Decoder::new() {
                if let Ok(mut decoded) = decoder.decode_all(&ivf_data) {
                    frames.append(&mut decoded);
                }
            }
        }
        return Ok(frames);
    }

    // Try MKV
    if let Ok(samples) = bitvue_formats::mkv::extract_av1_samples(file_data) {
        let mut frames = Vec::new();
        for sample in samples {
            let ivf_data = create_ivf_wrapper(&sample);
            if let Ok(mut decoder) = bitvue_decode::Av1Decoder::new() {
                if let Ok(mut decoded) = decoder.decode_all(&ivf_data) {
                    frames.append(&mut decoded);
                }
            }
        }
        return Ok(frames);
    }

    Err("Unsupported video format".to_string())
}

/// Create minimal IVF wrapper for a single AV1 sample
fn create_ivf_wrapper(sample_data: &[u8]) -> Vec<u8> {
    let mut ivf = Vec::new();

    // IVF header
    ivf.extend_from_slice(b"DKIF");
    ivf.extend_from_slice(&0u16.to_le_bytes());
    ivf.extend_from_slice(&1u16.to_le_bytes());
    ivf.extend_from_slice(b"AV01");
    ivf.extend_from_slice(&1920u16.to_le_bytes());
    ivf.extend_from_slice(&1080u16.to_le_bytes());
    ivf.extend_from_slice(&30u32.to_le_bytes());
    ivf.extend_from_slice(&1u32.to_le_bytes());
    ivf.extend_from_slice(&1u32.to_le_bytes());
    ivf.extend_from_slice(&[0u8; 4]);

    // Frame header
    ivf.extend_from_slice(&(sample_data.len() as u32).to_le_bytes());
    ivf.extend_from_slice(&0u64.to_le_bytes());
    ivf.extend_from_slice(sample_data);

    ivf
}
