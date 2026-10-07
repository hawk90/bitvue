//! `--psnr`: decode reference and distorted AV1 streams to luma and report per-frame PSNR.

use super::extract::FrameRecord;
use anyhow::{Context, Result};
use bitvue_av1_codec::parse_ivf_frames;

// ─── PSNR ─────────────────────────────────────────────────────────────────────

pub(super) fn compute_psnr(
    distorted_path: &std::path::Path,
    reference_path: &std::path::Path,
    dist_records: &[FrameRecord],
) -> Result<()> {
    // Only AV1 IVF decode is currently supported
    let ref_data = std::fs::read(reference_path)
        .with_context(|| format!("Cannot read reference: {}", reference_path.display()))?;
    let dist_data = std::fs::read(distorted_path)
        .with_context(|| format!("Cannot read distorted: {}", distorted_path.display()))?;

    let ref_decoded = decode_av1_luma(&ref_data)?;
    let dist_decoded = decode_av1_luma(&dist_data)?;

    if ref_decoded.is_empty() || dist_decoded.is_empty() {
        anyhow::bail!("PSNR: could not decode frames (AV1 IVF required)");
    }

    let pairs = ref_decoded
        .len()
        .min(dist_decoded.len())
        .min(dist_records.len());
    println!("── PSNR (luma, AV1) ───────────────────────────────");
    println!("{:<8} PSNR (dB)", "Frame");
    println!("{}", "-".repeat(18));

    let mut psnr_sum = 0.0f64;
    let mut psnr_min = f64::INFINITY;

    for i in 0..pairs {
        let (ref ref_y, rw, rh) = ref_decoded[i];
        let (ref dist_y, dw, dh) = dist_decoded[i];
        if rw != dw || rh != dh {
            println!("{:<8} dim mismatch", i);
            continue;
        }
        let val = bitvue_metrics::psnr(ref_y, dist_y, rw, rh).unwrap_or(0.0);
        println!("{:<8} {:.2}", i, val);
        psnr_sum += val;
        if val < psnr_min {
            psnr_min = val;
        }
    }

    if pairs > 0 {
        println!();
        println!(
            "Avg PSNR: {:.2} dB  Min PSNR: {:.2} dB",
            psnr_sum / pairs as f64,
            psnr_min
        );
    }
    Ok(())
}

/// Decode AV1 IVF → (luma_8bit, width, height) per frame.
fn decode_av1_luma(data: &[u8]) -> Result<Vec<(Vec<u8>, usize, usize)>> {
    use bitvue_decode::Av1Decoder;
    let (_hdr, frames) = parse_ivf_frames(data).map_err(|e| anyhow::anyhow!("IVF error: {}", e))?;
    let mut dec = Av1Decoder::new().map_err(|e| anyhow::anyhow!("Decoder init: {}", e))?;
    let mut out = Vec::new();

    for f in &frames {
        dec.send_data_owned(f.data.clone(), f.timestamp as i64)
            .map_err(|e| anyhow::anyhow!("Decode send: {}", e))?;
        drain_frames_luma(&mut dec, &mut out);
    }
    // Not dec.flush() -- see bitvue_decode::Av1Decoder::drain_decoder_frames' doc: flush() clears
    // dav1d's internal state (for seeking) instead of draining buffered frames, which silently
    // dropped every frame on streams shorter than dav1d's thread-pipeline depth.
    let mut remaining = Vec::new();
    dec.drain_decoder_frames(&mut remaining)
        .map_err(|e| anyhow::anyhow!("Decode drain: {}", e))?;
    for f in &remaining {
        push_luma_frame(f, &mut out);
    }
    Ok(out)
}

fn push_luma_frame(f: &bitvue_decode::DecodedFrame, out: &mut Vec<(Vec<u8>, usize, usize)>) {
    let y: Vec<u8> = if f.bit_depth == 8 {
        f.y_plane.to_vec()
    } else {
        f.y_plane.chunks(2).map(|c| c[0]).collect()
    };
    out.push((y, f.width as usize, f.height as usize));
}

fn drain_frames_luma(dec: &mut bitvue_decode::Av1Decoder, out: &mut Vec<(Vec<u8>, usize, usize)>) {
    while let Ok(f) = dec.get_frame() {
        push_luma_frame(&f, out);
    }
}
