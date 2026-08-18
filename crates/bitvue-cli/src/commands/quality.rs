//! Calculate quality metrics between two files
//!
//! Compares reference and distorted video files frame-by-frame using
//! PSNR and SSIM on decoded luma (Y) planes.

use anyhow::{Context, Result};
use bitvue_av1_codec::parse_ivf_frames;
use bitvue_decode::Av1Decoder;
use bitvue_engine::alignment::AlignmentEngine;
use bitvue_engine::frame_identity::{FrameIndexMap, FrameMetadata};
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

    // Align reference and distorted frames by PTS rather than raw array position:
    // if either stream ever drops a frame, index-based pairing silently compares
    // frame N of one stream against frame N±1 of the other for every frame after
    // the drop. `AlignmentEngine` (bitvue-engine) does real PTS-based nearest-
    // neighbor matching with gap detection; we only feed metrics from pairs it
    // reports as complete (i.e. both sides matched within the gap threshold).
    let matched_pairs = align_frame_pairs(&ref_decoded, &dist_decoded);

    // Determine which pairs (by position in the matched-pair sequence) to compare
    let total_pairs = matched_pairs.len();
    let frame_indices: Vec<usize> = parse_frame_spec(frames, total_pairs)?;

    let mut results: Vec<FrameMetrics> = Vec::new();

    for &idx in &frame_indices {
        let Some(&(ref_i, dist_i)) = matched_pairs.get(idx) else {
            eprintln!("Warning: frame {} out of range, skipping", idx);
            continue;
        };
        let (ref_frame, ref_w, ref_h, _) = &ref_decoded[ref_i];
        let (dist_frame, dist_w, dist_h, _) = &dist_decoded[dist_i];

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

/// PTS-align two decoded-frame sequences and return the list of matched
/// `(ref_vec_index, dist_vec_index)` pairs, in ascending-PTS order.
///
/// Frames that `AlignmentEngine` could not confidently match (gaps/drops on
/// either side) are excluded entirely rather than paired with a neighbor --
/// silently pairing a dropped frame with the wrong neighbor is exactly the
/// bug this replaces. A warning summarizing how many frames were dropped is
/// printed to stderr so the caller knows the comparison isn't 1:1 complete.
fn align_frame_pairs(
    ref_decoded: &[(Vec<u8>, usize, usize, i64)],
    dist_decoded: &[(Vec<u8>, usize, usize, i64)],
) -> Vec<(usize, usize)> {
    let to_frame_meta = |frames: &[(Vec<u8>, usize, usize, i64)]| -> Vec<FrameMetadata> {
        frames
            .iter()
            .map(|&(_, _, _, ts)| FrameMetadata {
                pts: u64::try_from(ts).ok(),
                dts: None,
            })
            .collect()
    };

    let ref_meta = to_frame_meta(ref_decoded);
    let dist_meta = to_frame_meta(dist_decoded);

    let ref_map = FrameIndexMap::new(&ref_meta);
    let dist_map = FrameIndexMap::new(&dist_meta);

    let alignment = AlignmentEngine::new(&ref_map, &dist_map);

    let mut matched = Vec::new();
    let mut dropped_ref = 0usize;
    let mut dropped_dist = 0usize;

    for pair in &alignment.frame_pairs {
        match (pair.stream_a_idx, pair.stream_b_idx) {
            (Some(a_display), Some(b_display)) if !pair.has_gap => {
                let a_idx = ref_map.display_to_decode_idx(a_display);
                let b_idx = dist_map.display_to_decode_idx(b_display);
                if let (Some(a_idx), Some(b_idx)) = (a_idx, b_idx) {
                    matched.push((a_idx, b_idx));
                }
            }
            (Some(_), _) => dropped_ref += 1,
            (_, Some(_)) => dropped_dist += 1,
            (None, None) => {}
        }
    }

    if dropped_ref > 0 || dropped_dist > 0 {
        eprintln!(
            "Warning: frame alignment ({}) dropped {} unmatched reference frame(s) and {} \
             unmatched distorted frame(s) out of {}/{} total; {} pair(s) will be compared. \
             Confidence: {}",
            alignment.method.display_text(),
            dropped_ref,
            dropped_dist,
            ref_decoded.len(),
            dist_decoded.len(),
            matched.len(),
            alignment.confidence().display_text(),
        );
    }

    matched
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
        println!("{:<8} PSNR (dB)", "Frame");
        println!("{}", "-".repeat(20));
        for r in &results {
            println!(
                "{:<8} {}",
                r.frame,
                r.psnr_db.map_or("N/A".to_string(), |v| format!("{:.2}", v)),
            );
        }
    } else {
        println!("{:<8} SSIM", "Frame");
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

/// Decoded luma plane: (y_plane_8bit, width, height, timestamp).
type LumaFrame = (Vec<u8>, usize, usize, i64);

/// Decode all frames from an AV1 IVF file, returning (y_plane_8bit, width, height, timestamp)
/// per frame.
///
/// Only 8-bit luma planes are returned; 10/12-bit frames are downsampled by
/// taking the top 8 bits of each 16-bit sample (see `downconvert_high_bitdepth_luma`).
fn decode_ivf_frames(data: &[u8]) -> Result<Vec<LumaFrame>> {
    let (_header, ivf_frames) =
        parse_ivf_frames(data).map_err(|e| anyhow::anyhow!("IVF parse error: {}", e))?;

    let mut decoder =
        Av1Decoder::new().map_err(|e| anyhow::anyhow!("Decoder init error: {}", e))?;

    let mut decoded: Vec<LumaFrame> = Vec::with_capacity(ivf_frames.len());

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

/// Convert a DecodedFrame to a densely-packed 8-bit luma plane with dimensions and timestamp.
fn decoded_frame_to_luma(f: bitvue_decode::DecodedFrame) -> (Vec<u8>, usize, usize, i64) {
    let y = if f.bit_depth == 8 {
        f.y_plane.iter().copied().collect()
    } else {
        downconvert_high_bitdepth_luma(&f.y_plane, f.bit_depth)
    };
    (y, f.width as usize, f.height as usize, f.timestamp)
}

/// Downconvert a >8-bit luma plane (2 bytes per sample, little-endian, sample
/// right-justified in the low `bit_depth` bits -- dav1d/rav1d's high-bit-depth
/// plane packing, e.g. a 10-bit value occupies bits [0,10) of the u16 word) to
/// an approximate 8-bit plane by taking the top 8 bits of each sample.
///
/// This mirrors `bitvue_decode::strategy::scalar::read_sample`'s already-correct
/// convention (`u16::from_le_bytes(...) >> (bit_depth - 8)`) elsewhere in this
/// codebase for the same plane format. The previous implementation took the
/// *low* byte (`c[0]`) of each LE sample pair, which for a right-justified
/// 10/12-bit sample is mostly noise (e.g. only bits 8-9 of a 10-bit sample land
/// in the high byte, so the low byte alone carries none of the meaningful
/// high-order magnitude) -- comparing frames using that byte compared near-
/// random low bits instead of real luma.
fn downconvert_high_bitdepth_luma(plane: &[u8], bit_depth: u8) -> Vec<u8> {
    debug_assert!(bit_depth > 8, "8-bit planes should not be downconverted");
    let shift = bit_depth.saturating_sub(8);
    plane
        .chunks_exact(2)
        .map(|c| {
            let sample16 = u16::from_le_bytes([c[0], c[1]]);
            (sample16 >> shift) as u8
        })
        .collect()
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

#[cfg(test)]
mod tests {
    use super::*;

    // ------------------------------------------------------------------
    // Bug 1 (COLOR-005): high-bit-depth luma downconversion
    // ------------------------------------------------------------------

    /// 10-bit mid-gray (value 512 = 0x0200) stored as an LE u16 sample:
    /// low byte 0x00, high byte 0x02. The *correct* 8-bit approximation is
    /// `512 >> (10 - 8) == 128` (8-bit mid-gray). The old buggy
    /// implementation took `c[0]` (the low byte) directly, which for this
    /// value is 0 -- i.e. it would report pure black for mid-gray input.
    #[test]
    fn downconvert_10bit_mid_gray_takes_top_bits_not_low_byte() {
        let plane = vec![0x00, 0x02]; // one 10-bit sample: value 512
        let out = downconvert_high_bitdepth_luma(&plane, 10);
        assert_eq!(out, vec![128]);

        // Sanity: confirm this is NOT what the old (buggy) c[0]-only
        // extraction would have produced for the same input.
        let old_buggy_result = plane[0];
        assert_ne!(out[0], old_buggy_result);
    }

    /// 12-bit mid-gray (value 2048 = 0x0800): low byte 0x00, high byte 0x08.
    /// Correct extraction: `2048 >> (12 - 8) == 128`.
    #[test]
    fn downconvert_12bit_mid_gray() {
        let plane = vec![0x00, 0x08];
        let out = downconvert_high_bitdepth_luma(&plane, 12);
        assert_eq!(out, vec![128]);
    }

    /// A value whose low byte is non-trivial makes the old c[0]-only bug
    /// obvious: value 300 (10-bit) -> LE bytes [0x2C, 0x01]. Old buggy
    /// output would be 44 (0x2C, the low byte); correct output is
    /// `300 >> 2 == 75`.
    #[test]
    fn downconvert_10bit_arbitrary_value_matches_shift_not_low_byte() {
        let value: u16 = 300;
        let bytes = value.to_le_bytes();
        let plane = vec![bytes[0], bytes[1]];

        let out = downconvert_high_bitdepth_luma(&plane, 10);
        assert_eq!(out, vec![(value >> 2) as u8]);
        assert_eq!(out, vec![75]);

        let old_buggy_result = plane[0]; // what `c[0]` alone would give
        assert_eq!(old_buggy_result, 44);
        assert_ne!(out[0], old_buggy_result);
    }

    /// Multiple samples in a row decode independently and preserve order.
    #[test]
    fn downconvert_multiple_samples() {
        let samples: [u16; 3] = [0, 1023, 512]; // 10-bit: black, white, mid-gray
        let mut plane = Vec::new();
        for s in samples {
            plane.extend_from_slice(&s.to_le_bytes());
        }
        let out = downconvert_high_bitdepth_luma(&plane, 10);
        assert_eq!(out, vec![0, 255, 128]);
    }

    /// 8-bit content must be unaffected by any bit-depth-aware path (it never
    /// reaches `downconvert_high_bitdepth_luma`, but `decoded_frame_to_luma`'s
    /// bit_depth==8 branch should be a plain byte-for-byte copy).
    #[test]
    fn decoded_frame_to_luma_8bit_passthrough_unaffected() {
        let y_plane: std::sync::Arc<[u8]> = std::sync::Arc::from(vec![10u8, 20, 30, 255]);
        let frame = bitvue_decode::DecodedFrame {
            width: 2,
            height: 2,
            bit_depth: 8,
            y_plane,
            y_stride: 2,
            u_plane: None,
            u_stride: 0,
            v_plane: None,
            v_stride: 0,
            timestamp: 42,
            frame_type: bitvue_decode::decoder::FrameType::Key,
            qp_avg: None,
            chroma_format: bitvue_decode::decoder::ChromaFormat::Yuv420,
        };
        let (y, w, h, ts) = decoded_frame_to_luma(frame);
        assert_eq!(y, vec![10, 20, 30, 255]);
        assert_eq!((w, h, ts), (2, 2, 42));
    }

    // ------------------------------------------------------------------
    // Bug 2 (ALIGN-001/005, STAT-007): PTS-based frame alignment
    // ------------------------------------------------------------------

    /// Build a fake decoded-frame list with only timestamps populated (the
    /// luma/width/height fields are irrelevant to `align_frame_pairs`).
    fn fake_frames(timestamps: &[i64]) -> Vec<(Vec<u8>, usize, usize, i64)> {
        timestamps
            .iter()
            .map(|&ts| (Vec::new(), 1, 1, ts))
            .collect()
    }

    /// Reference has 5 evenly-spaced frames; distorted is missing the
    /// pts=2000 frame entirely (a real dropped frame, as happens with real
    /// encoders/streams). Under the old `idx.min(len)` + index-zip pairing,
    /// every pair from position 2 onward would silently compare the wrong
    /// frames (ref pts=2000 vs dist pts=3000, ref pts=3000 vs dist pts=4000,
    /// and ref pts=4000 would be dropped from comparison entirely since the
    /// dist vec is shorter). PTS alignment must instead skip the dropped
    /// frame and correctly re-sync every frame after it.
    #[test]
    fn align_frame_pairs_skips_dropped_frame_and_resyncs_after_it() {
        let reference = fake_frames(&[0, 1000, 2000, 3000, 4000]);
        let distorted = fake_frames(&[0, 1000, 3000, 4000]); // pts=2000 dropped

        let matched = align_frame_pairs(&reference, &distorted);

        // The dropped frame (ref vec index 2, pts=2000) must not appear in
        // any pair -- neither silently merged into a neighboring pair nor
        // paired with the wrong dist frame.
        assert!(
            matched.iter().all(|&(ref_i, _)| ref_i != 2),
            "ref frame at pts=2000 (dropped from distorted) must be excluded, not misaligned: {:?}",
            matched
        );

        // Every remaining pair must be a genuine PTS match (same underlying
        // timestamp on both sides), not merely a matching array index.
        for (ref_i, dist_i) in &matched {
            assert_eq!(
                reference[*ref_i].3, distorted[*dist_i].3,
                "matched pair must share the same PTS, not just position"
            );
        }

        // Frames after the drop point must still be correctly re-synced by
        // PTS: ref pts=3000 -> dist pts=3000 (dist vec index 2), and
        // ref pts=4000 -> dist pts=4000 (dist vec index 3) -- NOT the
        // index-zip results (dist index 3 / out-of-range) that the old
        // buggy code would have produced.
        let pair_for_ref = |ref_idx: usize| {
            matched
                .iter()
                .find(|&&(r, _)| r == ref_idx)
                .copied()
                .unwrap_or_else(|| panic!("expected a match for ref index {}", ref_idx))
        };
        assert_eq!(pair_for_ref(0), (0, 0)); // pts 0 <-> pts 0
        assert_eq!(pair_for_ref(1), (1, 1)); // pts 1000 <-> pts 1000
        assert_eq!(pair_for_ref(3), (3, 2)); // pts 3000 <-> dist's pts 3000 (index 2, not 3)
        assert_eq!(pair_for_ref(4), (4, 3)); // pts 4000 <-> dist's pts 4000 (index 3, not OOB)

        assert_eq!(
            matched.len(),
            4,
            "exactly 4 of the 5 reference frames should match"
        );
    }

    /// When PTS sequences match exactly with no drops, alignment should
    /// reduce to the same pairing an index-zip would have produced (no
    /// regression for the common/clean case).
    #[test]
    fn align_frame_pairs_identity_when_no_drops() {
        let reference = fake_frames(&[0, 1000, 2000, 3000]);
        let distorted = fake_frames(&[0, 1000, 2000, 3000]);

        let matched = align_frame_pairs(&reference, &distorted);

        assert_eq!(matched, vec![(0, 0), (1, 1), (2, 2), (3, 3)]);
    }
}
