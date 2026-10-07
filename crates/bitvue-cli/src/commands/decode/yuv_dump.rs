//! `-o`: decode AV1 to raw YUV/Y4M and write it to disk.

use super::extract::FrameRecord;
use super::{codec_name, DecodeConfig, ForceCodec};
use anyhow::{Context, Result};
use bitvue_av1_codec::parse_ivf_frames;
use std::fs::File;
use std::io::Write as IoWrite;

// ─── YUV / Y4M dump ───────────────────────────────────────────────────────────

pub(super) fn do_yuv_dump(
    cfg: &DecodeConfig,
    file_data: &[u8],
    records: &[FrameRecord],
    codec: ForceCodec,
) -> Result<()> {
    if !matches!(codec, ForceCodec::AV1) {
        println!(
            "Note: YUV dump is only available for AV1 (decoded via dav1d). \
             Got {}.",
            codec_name(codec)
        );
        return Ok(());
    }

    // --film-grain: output both pre-grain and post-grain YUV streams.
    if cfg.film_grain {
        let base = cfg
            .output
            .clone()
            .unwrap_or_else(|| cfg.file.with_extension(""));
        let stem = base.file_stem().unwrap_or_default().to_string_lossy();
        let parent = base.parent().unwrap_or_else(|| std::path::Path::new("."));

        let pre_path = parent.join(format!("{}.pre_grain.yuv", stem));
        let post_path = parent.join(format!("{}.post_grain.yuv", stem));

        println!("Film grain — decoding pre-grain frames…");
        let pre_decoded = decode_av1_yuv_with_grain(file_data, records.len(), false)?;
        write_yuv_frames(&pre_decoded, &pre_path, cfg)?;
        println!(
            "Wrote {} pre-grain frame(s) to {}",
            pre_decoded.len(),
            pre_path.display()
        );

        println!("Film grain — decoding post-grain frames…");
        let post_decoded = decode_av1_yuv_with_grain(file_data, records.len(), true)?;
        write_yuv_frames(&post_decoded, &post_path, cfg)?;
        println!(
            "Wrote {} post-grain frame(s) to {}",
            post_decoded.len(),
            post_path.display()
        );
        return Ok(());
    }

    let decoded = decode_av1_yuv(file_data, records.len())?;
    if decoded.is_empty() {
        anyhow::bail!("No frames decoded");
    }

    let output_path = cfg
        .output
        .clone()
        .unwrap_or_else(|| cfg.file.with_extension(if cfg.y4m { "y4m" } else { "yuv" }));

    let mut out = File::create(&output_path)
        .with_context(|| format!("Cannot create output: {}", output_path.display()))?;

    // Y4M header
    if cfg.y4m {
        if let Some((_, w, h, _)) = decoded.first() {
            writeln!(
                out,
                "YUV4MPEG2 W{} H{} F30:1 Ip A0:0 C420mpeg2 XYSCSS=420MPEG2",
                w, h
            )?;
        }
    }

    let target_bd = cfg.dump_bitdepth.unwrap_or(8);

    for (i, (yuv, w, h, src_bd)) in decoded.iter().enumerate() {
        if cfg.y4m {
            out.write_all(b"FRAME\n")?;
        }

        // Write Y, U, V planes
        let y_size = w * h;
        let uv_size = (w / 2) * (h / 2);
        let plane_sizes = [y_size, uv_size, uv_size];

        let mut offset = 0;
        for &plane_sz in &plane_sizes {
            let plane = &yuv[offset..offset + plane_sz * if *src_bd > 8 { 2 } else { 1 }];
            if *src_bd > 8 && target_bd == 8 {
                // Downscale: take high byte of each LE 16-bit sample
                let out_plane: Vec<u8> = plane.chunks(2).map(|c| c[1]).collect();
                out.write_all(&out_plane)?;
            } else {
                out.write_all(plane)?;
            }
            offset += plane.len();
        }

        if (i + 1) % 100 == 0 {
            eprintln!("  Dumped {} frames…", i + 1);
        }
    }

    println!(
        "Wrote {} frame(s) to {}",
        decoded.len(),
        output_path.display()
    );
    Ok(())
}

// ─── AV1 decode helpers ───────────────────────────────────────────────────────

/// Per-frame decoded YUV: (full_yuv_bytes, width, height, bit_depth).
type YuvFrames = Vec<(Vec<u8>, usize, usize, u8)>;

/// Decode AV1 IVF → (full_yuv_bytes, width, height, bit_depth) per frame.
fn decode_av1_yuv(data: &[u8], limit: usize) -> Result<YuvFrames> {
    use bitvue_decode::Av1Decoder;
    let (_hdr, frames) = parse_ivf_frames(data).map_err(|e| anyhow::anyhow!("IVF error: {}", e))?;
    let mut dec = Av1Decoder::new().map_err(|e| anyhow::anyhow!("Decoder init: {}", e))?;
    let mut out: YuvFrames = Vec::new();

    for f in &frames {
        dec.send_data_owned(f.data.clone(), f.timestamp as i64)
            .map_err(|e| anyhow::anyhow!("Decode send: {}", e))?;
        drain_frames_yuv(&mut dec, &mut out);
        if out.len() >= limit {
            break;
        }
    }
    if out.len() < limit {
        drain_frames_yuv_at_eos(&mut dec, &mut out)?;
    }
    // Each send drains every frame the decoder has ready, so `out` can overshoot `limit`.
    out.truncate(limit);
    Ok(out)
}

fn push_yuv_frame(f: &bitvue_decode::DecodedFrame, out: &mut YuvFrames) {
    let mut combined = f.y_plane.to_vec();
    if let Some(ref u) = f.u_plane {
        combined.extend_from_slice(u);
    }
    if let Some(ref v) = f.v_plane {
        combined.extend_from_slice(v);
    }
    out.push((combined, f.width as usize, f.height as usize, f.bit_depth));
}

fn drain_frames_yuv(dec: &mut bitvue_decode::Av1Decoder, out: &mut YuvFrames) {
    while let Ok(f) = dec.get_frame() {
        push_yuv_frame(&f, out);
    }
}

/// Not dec.flush() -- see bitvue_decode::Av1Decoder::drain_decoder_frames' doc: flush() clears
/// dav1d's internal state (for seeking) instead of draining buffered frames, which silently
/// dropped every frame on streams shorter than dav1d's thread-pipeline depth.
fn drain_frames_yuv_at_eos(dec: &mut bitvue_decode::Av1Decoder, out: &mut YuvFrames) -> Result<()> {
    let mut remaining = Vec::new();
    dec.drain_decoder_frames(&mut remaining)
        .map_err(|e| anyhow::anyhow!("Decode drain: {}", e))?;
    for f in &remaining {
        push_yuv_frame(f, out);
    }
    Ok(())
}

// ─── Film-grain helpers ───────────────────────────────────────────────────────

/// Decode AV1 IVF with explicit film-grain control.
fn decode_av1_yuv_with_grain(data: &[u8], limit: usize, apply_grain: bool) -> Result<YuvFrames> {
    use bitvue_decode::Av1Decoder;
    let (_hdr, frames) = parse_ivf_frames(data).map_err(|e| anyhow::anyhow!("IVF error: {}", e))?;
    let mut dec = Av1Decoder::new_with_apply_grain(apply_grain)
        .map_err(|e| anyhow::anyhow!("Decoder init: {}", e))?;
    let mut out: YuvFrames = Vec::new();

    for f in &frames {
        dec.send_data_owned(f.data.clone(), f.timestamp as i64)
            .map_err(|e| anyhow::anyhow!("Decode send: {}", e))?;
        drain_frames_yuv(&mut dec, &mut out);
        if out.len() >= limit {
            break;
        }
    }
    if out.len() < limit {
        drain_frames_yuv_at_eos(&mut dec, &mut out)?;
    }
    // Each send drains every frame the decoder has ready, so `out` can overshoot `limit`.
    out.truncate(limit);
    Ok(out)
}

/// Write a decoded YUV frame list to `path`, honouring the y4m and
/// dump_bitdepth settings from `cfg`.
fn write_yuv_frames(
    decoded: &[(Vec<u8>, usize, usize, u8)],
    path: &std::path::Path,
    cfg: &DecodeConfig,
) -> Result<()> {
    if decoded.is_empty() {
        return Ok(());
    }
    let mut out =
        File::create(path).with_context(|| format!("Cannot create output: {}", path.display()))?;

    if cfg.y4m {
        if let Some((_, w, h, _)) = decoded.first() {
            writeln!(
                out,
                "YUV4MPEG2 W{} H{} F30:1 Ip A0:0 C420mpeg2 XYSCSS=420MPEG2",
                w, h
            )?;
        }
    }

    let target_bd = cfg.dump_bitdepth.unwrap_or(8);
    for (i, (yuv, w, h, src_bd)) in decoded.iter().enumerate() {
        if cfg.y4m {
            out.write_all(b"FRAME\n")?;
        }
        let y_size = w * h;
        let uv_size = (w / 2) * (h / 2);
        let mut offset = 0;
        for &plane_sz in &[y_size, uv_size, uv_size] {
            let plane = &yuv[offset..offset + plane_sz * if *src_bd > 8 { 2 } else { 1 }];
            if *src_bd > 8 && target_bd == 8 {
                let out_plane: Vec<u8> = plane.chunks(2).map(|c| c[1]).collect();
                out.write_all(&out_plane)?;
            } else {
                out.write_all(plane)?;
            }
            offset += plane.len();
        }
        if (i + 1) % 100 == 0 {
            eprintln!("  Dumped {} frames…", i + 1);
        }
    }
    Ok(())
}
