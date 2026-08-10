//! Calculate quality metrics between two files
//!
//! Compares reference and distorted video files frame-by-frame using
//! PSNR and SSIM on decoded luma (Y) planes.

use anyhow::{Context, Result};
use bitvue_av1_codec::parse_ivf_frames;
use bitvue_decode::Av1Decoder;
use bitvue_formats::container::{detect_container_format, ContainerFormat};
use bitvue_metrics::{psnr, ssim};
use std::path::PathBuf;

#[derive(Debug)]
pub struct FrameMetrics {
    pub frame: usize,
    pub psnr_db: Option<f64>,
    pub ssim: Option<f64>,
}

/// Decodes `reference`/`distorted` (AV1 IVF only) and computes per-frame PSNR/SSIM for the
/// frames selected by `frames` ("0", "0,1,2", or "all"). Shared by `quality`'s CLI output and
/// `bd-rate`'s per-file average-quality extraction -- both need the same decode+compare, just a
/// different summary of the result.
pub fn compute_frame_metrics(
    reference: &std::path::Path,
    distorted: &std::path::Path,
    frames: &str,
    want_psnr: bool,
    want_ssim: bool,
) -> Result<Vec<FrameMetrics>> {
    if !reference.exists() {
        anyhow::bail!("Reference file not found: {}", reference.display());
    }
    if !distorted.exists() {
        anyhow::bail!("Distorted file not found: {}", distorted.display());
    }
    if !want_psnr && !want_ssim {
        anyhow::bail!("At least one of psnr/ssim must be requested");
    }

    let ref_data = std::fs::read(reference)
        .with_context(|| format!("Failed to read reference: {}", reference.display()))?;
    let dist_data = std::fs::read(distorted)
        .with_context(|| format!("Failed to read distorted: {}", distorted.display()))?;

    // Only IVF (AV1) is currently supported for frame-level decode
    let ref_fmt = detect_container_format(reference).unwrap_or(ContainerFormat::Unknown);
    let dist_fmt = detect_container_format(distorted).unwrap_or(ContainerFormat::Unknown);

    if !matches!(ref_fmt, ContainerFormat::IVF) || !matches!(dist_fmt, ContainerFormat::IVF) {
        anyhow::bail!(
            "Quality metrics currently require AV1 IVF files. \
             Reference format: {:?}, Distorted format: {:?}",
            ref_fmt,
            dist_fmt
        );
    }

    // Decode reference frames
    let ref_decoded = decode_ivf_frames(&ref_data).context("Failed to decode reference frames")?;
    let dist_decoded =
        decode_ivf_frames(&dist_data).context("Failed to decode distorted frames")?;

    if ref_decoded.is_empty() {
        anyhow::bail!("No frames decoded from reference file");
    }
    if dist_decoded.is_empty() {
        anyhow::bail!("No frames decoded from distorted file");
    }

    // Determine which frame indices to compare
    let total_pairs = ref_decoded.len().min(dist_decoded.len());
    let frame_indices: Vec<usize> = parse_frame_spec(frames, total_pairs)?;

    let mut results: Vec<FrameMetrics> = Vec::new();

    for &idx in &frame_indices {
        if idx >= ref_decoded.len() || idx >= dist_decoded.len() {
            eprintln!("Warning: frame {} out of range, skipping", idx);
            continue;
        }
        let (ref_frame, ref_w, ref_h) = &ref_decoded[idx];
        let (dist_frame, dist_w, dist_h) = &dist_decoded[idx];

        if ref_w != dist_w || ref_h != dist_h {
            eprintln!(
                "Warning: frame {} dimension mismatch ({}x{} vs {}x{}), skipping",
                idx, ref_w, ref_h, dist_w, dist_h
            );
            continue;
        }

        let width = *ref_w;
        let height = *ref_h;

        let psnr_val = if want_psnr {
            psnr(ref_frame, dist_frame, width, height).ok()
        } else {
            None
        };

        let ssim_val = if want_ssim {
            ssim(ref_frame, dist_frame, width, height).ok()
        } else {
            None
        };

        results.push(FrameMetrics {
            frame: idx,
            psnr_db: psnr_val,
            ssim: ssim_val,
        });
    }

    Ok(results)
}

pub fn run(reference: PathBuf, distorted: PathBuf, frames: &str, metrics: &str) -> Result<()> {
    let want_psnr = metrics.contains("psnr");
    let want_ssim = metrics.contains("ssim");
    if !want_psnr && !want_ssim {
        anyhow::bail!("Unknown metrics '{}'. Supported: psnr, ssim", metrics);
    }

    println!(
        "Comparing {} vs {}",
        reference.display(),
        distorted.display()
    );
    println!("Metrics: {}", metrics);
    println!();

    let results = compute_frame_metrics(&reference, &distorted, frames, want_psnr, want_ssim)?;

    // Print per-frame results
    if want_psnr && want_ssim {
        println!("{:<8} {:<12} {:<12}", "Frame", "PSNR (dB)", "SSIM");
        println!("{}", "-".repeat(34));
        for r in &results {
            println!(
                "{:<8} {:<12} {:<12}",
                r.frame,
                r.psnr_db.map_or("N/A".to_string(), |v| format!("{:.2}", v)),
                r.ssim.map_or("N/A".to_string(), |v| format!("{:.4}", v)),
            );
        }
    } else if want_psnr {
        println!("{:<8} {}", "Frame", "PSNR (dB)");
        println!("{}", "-".repeat(20));
        for r in &results {
            println!(
                "{:<8} {}",
                r.frame,
                r.psnr_db.map_or("N/A".to_string(), |v| format!("{:.2}", v)),
            );
        }
    } else {
        println!("{:<8} {}", "Frame", "SSIM");
        println!("{}", "-".repeat(16));
        for r in &results {
            println!(
                "{:<8} {}",
                r.frame,
                r.ssim.map_or("N/A".to_string(), |v| format!("{:.4}", v)),
            );
        }
    }

    // Summary
    println!();
    let psnr_vals: Vec<f64> = results.iter().filter_map(|r| r.psnr_db).collect();
    let ssim_vals: Vec<f64> = results.iter().filter_map(|r| r.ssim).collect();

    if !psnr_vals.is_empty() {
        let avg = psnr_vals.iter().sum::<f64>() / psnr_vals.len() as f64;
        let min = psnr_vals.iter().cloned().fold(f64::INFINITY, f64::min);
        println!("PSNR  avg={:.2} dB  min={:.2} dB", avg, min);
    }
    if !ssim_vals.is_empty() {
        let avg = ssim_vals.iter().sum::<f64>() / ssim_vals.len() as f64;
        let min = ssim_vals.iter().cloned().fold(f64::INFINITY, f64::min);
        println!("SSIM  avg={:.4}  min={:.4}", avg, min);
    }

    Ok(())
}

/// Decode all frames from an AV1 IVF file, returning (y_plane_8bit, width, height) per frame.
///
/// Only 8-bit luma planes are returned; 10/12-bit frames are downsampled by
/// taking the high byte from each 16-bit sample.
fn decode_ivf_frames(data: &[u8]) -> Result<Vec<(Vec<u8>, usize, usize)>> {
    let (_header, ivf_frames) =
        parse_ivf_frames(data).map_err(|e| anyhow::anyhow!("IVF parse error: {}", e))?;

    let mut decoder =
        Av1Decoder::new().map_err(|e| anyhow::anyhow!("Decoder init error: {}", e))?;

    let mut decoded: Vec<(Vec<u8>, usize, usize)> = Vec::with_capacity(ivf_frames.len());

    for frame in &ivf_frames {
        decoder
            .send_data_owned(frame.data.clone(), frame.timestamp as i64)
            .map_err(|e| anyhow::anyhow!("Decode send error: {}", e))?;

        loop {
            match decoder.get_frame() {
                Ok(f) => {
                    decoded.push(decoded_frame_to_luma(f));
                }
                Err(bitvue_decode::decoder::DecodeError::NoFrame) => break,
                Err(e) => return Err(anyhow::anyhow!("Decode error: {}", e)),
            }
        }
    }

    // Not decoder.flush() -- see bitvue_decode::Av1Decoder::drain_decoder_frames' doc: flush()
    // clears dav1d's internal state (for seeking) instead of draining buffered frames, which
    // silently dropped every frame on streams shorter than dav1d's thread-pipeline depth.
    let mut remaining = Vec::new();
    decoder
        .drain_decoder_frames(&mut remaining)
        .map_err(|e| anyhow::anyhow!("Decode drain error: {}", e))?;
    for f in remaining {
        decoded.push(decoded_frame_to_luma(f));
    }

    Ok(decoded)
}

/// Convert a DecodedFrame to a densely-packed 8-bit luma plane with dimensions.
fn decoded_frame_to_luma(f: bitvue_decode::DecodedFrame) -> (Vec<u8>, usize, usize) {
    let y = if f.bit_depth == 8 {
        f.y_plane.iter().copied().collect()
    } else {
        // For >8-bit: take every other byte (low byte of LE 16-bit samples)
        f.y_plane.chunks(2).map(|c| c[0]).collect()
    };
    (y, f.width as usize, f.height as usize)
}

/// Parse the frame specification string into a list of frame indices.
/// Supports: "0", "0,1,2", "all"
fn parse_frame_spec(spec: &str, total: usize) -> Result<Vec<usize>> {
    if spec.eq_ignore_ascii_case("all") {
        return Ok((0..total).collect());
    }
    let mut indices = Vec::new();
    for part in spec.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let idx: usize = part
            .parse()
            .with_context(|| format!("Invalid frame index '{}' in spec '{}'", part, spec))?;
        indices.push(idx);
    }
    if indices.is_empty() {
        anyhow::bail!("No frame indices specified in '{}'", spec);
    }
    Ok(indices)
}
