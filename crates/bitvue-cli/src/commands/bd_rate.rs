//! Build two RD (rate-distortion) curves from real AV1 IVF files and compute BD-rate/BD-quality.
//!
//! Each anchor/test file is compared against the same `--reference` file to get an average
//! quality value (PSNR by default), and its bitrate is computed from file size / duration (via
//! the IVF header's frame count + framerate) -- no QP-based approximation, matching this
//! session's "real measured distortion, not a heuristic" bar (see `docs/PARITY_CHECKLIST.md`
//! CMP-05's history). The actual BD-rate math lives in `bitvue_metrics::bd_rate`.

use anyhow::{Context, Result};
use bitvue_av1_codec::parse_ivf_frames;
use bitvue_metrics::bd_rate::{calculate_bd_rate, RdCurve, RdPoint};
use std::path::{Path, PathBuf};

use super::quality::compute_frame_metrics;

#[allow(clippy::too_many_arguments)]
pub fn run(
    reference: PathBuf,
    anchor_files: Vec<PathBuf>,
    test_files: Vec<PathBuf>,
    anchor_name: String,
    test_name: String,
    metric: String,
) -> Result<()> {
    let want_psnr = metric == "psnr";
    let want_ssim = metric == "ssim";
    if !want_psnr && !want_ssim {
        anyhow::bail!("Unknown --metric '{metric}'. Supported: psnr, ssim");
    }

    println!(
        "Building RD curves against reference {}",
        reference.display()
    );
    let anchor = build_curve(
        &anchor_name,
        &reference,
        &anchor_files,
        want_psnr,
        want_ssim,
    )?;
    let test = build_curve(&test_name, &reference, &test_files, want_psnr, want_ssim)?;

    for curve in [&anchor, &test] {
        println!("\n{} ({} point(s)):", curve.name, curve.points.len());
        for p in &curve.points {
            println!(
                "  bitrate={:>10.1} kbps  quality={:.3}",
                p.bitrate_kbps, p.quality
            );
        }
    }

    let result = calculate_bd_rate(&anchor, &test)
        .with_context(|| format!("computing BD-rate between '{anchor_name}' and '{test_name}'"))?;

    println!(
        "\nBD-rate: {:+.2}%  (negative = '{}' needs less bitrate for equal quality)",
        result.bd_rate_percent, result.test_name
    );
    println!(
        "BD-quality: {:+.3}  (positive = '{}' has higher quality at equal bitrate)",
        result.bd_quality, result.test_name
    );

    Ok(())
}

fn build_curve(
    name: &str,
    reference: &Path,
    files: &[PathBuf],
    want_psnr: bool,
    want_ssim: bool,
) -> Result<RdCurve> {
    let mut points = Vec::with_capacity(files.len());
    for file in files {
        let metrics = compute_frame_metrics(reference, file, "all", want_psnr, want_ssim)
            .with_context(|| format!("computing quality for {}", file.display()))?;

        let values: Vec<f64> = if want_psnr {
            metrics.iter().filter_map(|m| m.psnr_db).collect()
        } else {
            metrics.iter().filter_map(|m| m.ssim).collect()
        };
        if values.is_empty() {
            anyhow::bail!("no valid quality samples computed for {}", file.display());
        }
        let quality = values.iter().sum::<f64>() / values.len() as f64;
        let bitrate_kbps = bitrate_kbps_of(file)?;
        points.push(RdPoint {
            bitrate_kbps,
            quality,
        });
    }
    Ok(RdCurve {
        name: name.to_string(),
        points,
    })
}

/// `file_size_bits / duration_seconds / 1000`, where duration is the *actually parsed* frame
/// count (not the IVF header's own `frame_count` field, which some encoders leave wrong) times
/// the framerate. Falls back to 1 second if framerate is missing/zero, matching the
/// pre-migration Tauri implementation's fallback.
fn bitrate_kbps_of(file: &Path) -> Result<f64> {
    let data = std::fs::read(file).with_context(|| format!("reading {}", file.display()))?;
    let (header, frames) = parse_ivf_frames(&data)
        .map_err(|e| anyhow::anyhow!("parsing IVF frames for {}: {e}", file.display()))?;

    // NOTE: `IvfHeader::framerate_den`/`framerate_num` are named backwards from their actual
    // on-disk meaning -- verified empirically against a known-25fps real (ffmpeg-generated)
    // file: framerate_den holds the raw "rate" byte value, framerate_num holds "scale", so
    // fps = framerate_den/framerate_num and duration = frame_count/fps = frame_count *
    // framerate_num/framerate_den. See the identical note in `commands/info.rs`.
    let duration_secs = if header.framerate_num > 0 && header.framerate_den > 0 {
        frames.len() as f64 * header.framerate_num as f64 / header.framerate_den as f64
    } else {
        1.0
    };

    Ok((data.len() as f64 * 8.0) / (duration_secs * 1000.0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitvue_formats::IvfWriter;

    /// Regression test for the framerate_num/framerate_den mixup: writes a real 25fps, 25-frame
    /// IVF file, and checks the resulting bitrate against a manually-computed expected value.
    /// Before the fix, this file's computed duration was 625s instead of 1s (off by 625x), which
    /// is exactly the framerate_den^2/framerate_num == 25*25/1 signature of the num/den swap.
    #[test]
    fn bitrate_kbps_of_a_known_25fps_file_is_correct() {
        let frame_payload = vec![0u8; 100]; // 100 bytes/frame, dummy OBU-shaped content
        let frame_count = 25;
        let mut writer = IvfWriter::new(
            176, 144, /* framerate_num */ 1, /* framerate_den */ 25,
        );
        for i in 0..frame_count {
            writer.write_frame(&frame_payload, i as u64).unwrap();
        }
        let data = writer.finalize();

        let dir = std::env::temp_dir().join(format!(
            "bitvue_bdrate_fps_test_{:?}",
            std::thread::current().id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("known_25fps.ivf");
        std::fs::write(&path, &data).unwrap();

        // 25 frames @ 25fps = 1.0s duration. file_size_bits / 1.0s / 1000 = expected kbps.
        let expected_kbps = (data.len() as f64 * 8.0) / 1000.0;
        let actual_kbps = bitrate_kbps_of(&path).unwrap();

        assert!(
            (actual_kbps - expected_kbps).abs() < 1e-6,
            "expected {expected_kbps:.3} kbps (1.0s duration), got {actual_kbps:.3} kbps"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }
}
