//! decode — VQ Analyzer-compatible analysis flags
//!
//! Mirrors the VQ Analyzer CLI interface:
//!   bitvue decode <file> [-hevc|-av1|-vp9|-avc] [-frames N] [--md5] [--stats]
//!                        [-o out.yuv] [--y4m] [--psnr --reference ref.ivf]
//!                        [--regress] [--dump-bitdepth N] [--fast N]

use anyhow::{Context, Result};
use bitvue_av1_codec::{parse_ivf_frames, ObuIterator, ObuType};
use bitvue_formats::container::{detect_container_format, ContainerFormat};
use memmap2::MmapOptions;
use rayon::prelude::*;
use std::fs::File;
use std::io::Write as IoWrite;
use std::path::PathBuf;

// ─── Public types ─────────────────────────────────────────────────────────────

/// Codec forced by the user (overrides auto-detect).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForceCodec {
    AV1,
    HEVC,
    AVC,
    VP9,
    VVC,
    MPEG2,
    AVS3,
    JpegXs,
    Vc3,
}

/// Full configuration for the `decode` subcommand.
#[derive(Debug, Default)]
pub struct DecodeConfig {
    pub file: PathBuf,
    pub force_codec: Option<ForceCodec>,
    pub max_frames: usize,
    pub md5: bool,
    pub stats: bool,
    pub stream_stats: bool,
    pub output: Option<PathBuf>,
    pub y4m: bool,
    pub dump: bool,
    pub dump_bitdepth: Option<u8>,
    pub regress: bool,
    pub display_order: bool,
    pub no_crop: bool,
    pub fast: u8,
    pub film_grain: bool,
    pub errors_file: Option<PathBuf>,
    pub psnr: bool,
    pub reference: Option<PathBuf>,
}

// ─── Unified frame record ─────────────────────────────────────────────────────

#[derive(Debug)]
struct FrameRecord {
    index: usize,
    frame_type: String,
    size: usize,
    pts: Option<u64>,
    offset: u64,
    key_frame: bool,
    /// Raw compressed data — populated only when --md5 is requested.
    md5_hex: Option<String>,
}

// ─── Entry point ─────────────────────────────────────────────────────────────

pub fn run(cfg: DecodeConfig) -> Result<()> {
    if !cfg.file.exists() {
        anyhow::bail!("File not found: {}", cfg.file.display());
    }

    // Memory-map the file — the OS pages in data on demand rather than
    // copying the entire file into the process heap upfront.  For multi-GB
    // files this avoids both the initial read latency and the peak RSS spike.
    //
    // SAFETY: The file is opened read-only.  We hold `_fh` alive alongside
    // `file_data` so that the mmap remains valid for the duration of `run`.
    let _fh =
        File::open(&cfg.file).with_context(|| format!("Cannot open: {}", cfg.file.display()))?;
    let file_data = unsafe { MmapOptions::new().map(&_fh) }
        .with_context(|| format!("Cannot mmap: {}", cfg.file.display()))?;

    // Determine effective codec
    let effective_codec = resolve_codec(&cfg, &file_data);

    if !cfg.regress {
        println!(
            "File:   {}  ({:.2} MB)",
            cfg.file.display(),
            file_data.len() as f64 / 1_048_576.0
        );
        println!("Codec:  {}", codec_name(effective_codec));
    }

    // ── Extract frames ──────────────────────────────────────────────────────
    let limit = if cfg.max_frames == 0 {
        usize::MAX
    } else {
        cfg.max_frames
    };
    // Pass want_md5=false; we compute MD5 in parallel below after extraction.
    let (mut records, parse_errors) = extract_frames(effective_codec, &file_data, limit, false)?;

    // ── Parallel MD5 computation ────────────────────────────────────────────
    // rayon splits the slice across available CPU cores.  Each record already
    // carries (offset, size) so no synchronisation on the mmap'd file_data
    // is needed — reads are purely read-only and Mmap is Sync.
    if cfg.md5 {
        records.par_iter_mut().for_each(|r| {
            let start = r.offset as usize;
            let end = (start + r.size).min(file_data.len());
            r.md5_hex = Some(md5_hex(&file_data[start..end]));
        });
    }

    if cfg.regress {
        // Silent decode — report error count and exit with appropriate code
        if parse_errors > 0 {
            eprintln!(
                "REGRESS: {} parse error(s) in {}",
                parse_errors,
                cfg.file.display()
            );
            std::process::exit(1);
        } else {
            println!("REGRESS OK: {} frames, 0 errors", records.len());
            return Ok(());
        }
    }

    // ── Print frame table ───────────────────────────────────────────────────
    print_frame_table(&records, cfg.md5);

    // ── Stream stats ────────────────────────────────────────────────────────
    if cfg.stats || cfg.stream_stats {
        println!();
        print_stream_stats(&records, cfg.stream_stats);
    }

    // ── PSNR ────────────────────────────────────────────────────────────────
    if cfg.psnr {
        if let Some(ref ref_path) = cfg.reference {
            println!();
            compute_psnr(&cfg.file, ref_path, &records)?;
        } else {
            eprintln!("Warning: --psnr requires --reference <file>");
        }
    }

    // ── YUV / Y4M dump ──────────────────────────────────────────────────────
    if cfg.dump || cfg.output.is_some() {
        println!();
        do_yuv_dump(&cfg, &file_data, &records, effective_codec)?;
    }

    // ── Error log ───────────────────────────────────────────────────────────
    if let Some(ref err_path) = cfg.errors_file {
        if parse_errors > 0 {
            let mut f = File::create(err_path)
                .with_context(|| format!("Cannot create error log: {}", err_path.display()))?;
            writeln!(
                f,
                "{} parse error(s) in {}",
                parse_errors,
                cfg.file.display()
            )?;
        }
    }

    if parse_errors > 0 {
        eprintln!("\nWarning: {} parse error(s) encountered", parse_errors);
    }

    Ok(())
}

// ─── Codec resolution ─────────────────────────────────────────────────────────

fn resolve_codec(cfg: &DecodeConfig, data: &[u8]) -> ForceCodec {
    if let Some(forced) = cfg.force_codec {
        return forced;
    }
    // Auto-detect from file
    let container = detect_container_format(&cfg.file).unwrap_or(ContainerFormat::Unknown);
    match container {
        ContainerFormat::IVF => {
            // Peek FourCC from IVF header
            if data.len() >= 12 {
                match &data[8..12] {
                    b"AV01" => ForceCodec::AV1,
                    b"VP90" | b"VP9 " => ForceCodec::VP9,
                    _ => ForceCodec::AV1, // default
                }
            } else {
                ForceCodec::AV1
            }
        }
        ContainerFormat::AnnexB => ForceCodec::HEVC,
        _ => ForceCodec::AV1, // fallback
    }
}

fn codec_name(c: ForceCodec) -> &'static str {
    match c {
        ForceCodec::AV1 => "AV1",
        ForceCodec::HEVC => "HEVC/H.265",
        ForceCodec::AVC => "AVC/H.264",
        ForceCodec::VP9 => "VP9",
        ForceCodec::VVC => "VVC/H.266",
        ForceCodec::MPEG2 => "MPEG-2 Video",
        ForceCodec::AVS3 => "AVS3",
        ForceCodec::JpegXs => "JPEG XS",
        ForceCodec::Vc3 => "VC-3/DNxHD",
    }
}

// ─── Frame extraction ─────────────────────────────────────────────────────────

/// Returns (frames, parse_error_count).
fn extract_frames(
    codec: ForceCodec,
    data: &[u8],
    limit: usize,
    want_md5: bool,
) -> Result<(Vec<FrameRecord>, usize)> {
    match codec {
        ForceCodec::AV1 => extract_av1_frames(data, limit, want_md5),
        ForceCodec::HEVC => extract_hevc_frames(data, limit, want_md5),
        ForceCodec::AVC => extract_avc_frames(data, limit, want_md5),
        ForceCodec::VP9 => extract_vp9_frames(data, limit, want_md5),
        ForceCodec::AVS3 => extract_avs3_frames(data, limit, want_md5),
        ForceCodec::JpegXs => extract_jpegxs_frames(data, limit, want_md5),
        ForceCodec::Vc3 => extract_vc3_frames_cli(data, limit, want_md5),
        _ => {
            eprintln!(
                "Note: Frame extraction not yet implemented for {}.",
                codec_name(codec)
            );
            Ok((Vec::new(), 0))
        }
    }
}

fn md5_hex(data: &[u8]) -> String {
    let digest = md5::compute(data);
    format!("{:x}", digest)
}

fn maybe_md5(data: &[u8], want: bool) -> Option<String> {
    if want {
        Some(md5_hex(data))
    } else {
        None
    }
}

// AV1 ──────────────────────────────────────────────────────────────────────────

fn extract_av1_frames(
    data: &[u8],
    limit: usize,
    want_md5: bool,
) -> Result<(Vec<FrameRecord>, usize)> {
    let (_header, frames) =
        parse_ivf_frames(data).map_err(|e| anyhow::anyhow!("IVF parse error: {}", e))?;

    let mut records = Vec::with_capacity(frames.len().min(limit));
    let mut errors = 0usize;
    let mut offset: u64 = 32; // IVF file header

    for (idx, frame) in frames.iter().enumerate() {
        if records.len() >= limit {
            break;
        }

        let frame_offset = offset + 12;
        let (frame_type, had_error) = av1_frame_type(&frame.data);
        if had_error {
            errors += 1;
        }

        records.push(FrameRecord {
            index: idx,
            frame_type,
            size: frame.size as usize,
            pts: Some(frame.timestamp),
            offset: frame_offset,
            key_frame: false, // filled from frame_type
            md5_hex: maybe_md5(&frame.data, want_md5),
        });
        // Fix key_frame
        let last = records.last_mut().unwrap();
        last.key_frame = last.frame_type == "I" || last.frame_type == "KEY";

        offset += 12 + frame.size as u64;
    }

    Ok((records, errors))
}

fn av1_frame_type(data: &[u8]) -> (String, bool) {
    let mut error = false;
    for obu in ObuIterator::new(data) {
        match obu {
            Ok(obu) if matches!(obu.header.obu_type, ObuType::Frame | ObuType::FrameHeader) => {
                if let Some(ft) = obu.frame_type {
                    return (ft.as_str().to_string(), error);
                }
            }
            Err(_) => {
                error = true;
            }
            _ => {}
        }
    }
    ("?".to_string(), error)
}

// HEVC ─────────────────────────────────────────────────────────────────────────

fn extract_hevc_frames(
    data: &[u8],
    limit: usize,
    want_md5: bool,
) -> Result<(Vec<FrameRecord>, usize)> {
    use bitvue_hevc::{parse_hevc, NalUnitType};

    let stream = parse_hevc(data).map_err(|e| anyhow::anyhow!("HEVC parse error: {}", e))?;
    let mut records = Vec::new();

    // Use slice records to associate NAL offset/size/type
    for (idx, slice) in stream.slices.iter().enumerate() {
        if records.len() >= limit {
            break;
        }

        let nal = &stream.nal_units[slice.nal_index];
        let nal_type = nal.header.nal_unit_type;
        let key_frame = nal_type.is_idr();
        let frame_type = if key_frame {
            "IDR".to_string()
        } else if nal_type.is_irap() {
            "CRA".to_string()
        } else {
            format!("{:?}", nal_type)
        };
        let offset = nal.offset;
        let size = nal.size as usize;
        let end = (offset as usize + size).min(data.len());
        let frame_data = &data[offset as usize..end];

        records.push(FrameRecord {
            index: idx,
            frame_type,
            size,
            pts: Some(slice.poc as u64),
            offset,
            key_frame,
            md5_hex: maybe_md5(frame_data, want_md5),
        });
    }

    Ok((records, 0))
}

// AVC ──────────────────────────────────────────────────────────────────────────

fn extract_avc_frames(
    data: &[u8],
    limit: usize,
    want_md5: bool,
) -> Result<(Vec<FrameRecord>, usize)> {
    use bitvue_avc::{parse_avc, NalUnitType};

    let stream = parse_avc(data).map_err(|e| anyhow::anyhow!("AVC parse error: {}", e))?;
    let mut records = Vec::new();

    for (idx, slice) in stream.slices.iter().enumerate() {
        if records.len() >= limit {
            break;
        }

        let nal = &stream.nal_units[slice.nal_index];
        let is_idr = nal.header.nal_unit_type == NalUnitType::IdrSlice;
        let frame_type = format!("{:?}", slice.header.slice_type);
        let offset = nal.offset;
        let size = nal.size;
        let end = (offset + size).min(data.len());
        let frame_data = &data[offset..end];

        records.push(FrameRecord {
            index: idx,
            frame_type,
            size,
            pts: Some(slice.poc as u64),
            offset: offset as u64,
            key_frame: is_idr,
            md5_hex: maybe_md5(frame_data, want_md5),
        });
    }

    Ok((records, 0))
}

// VP9 ──────────────────────────────────────────────────────────────────────────

fn extract_vp9_frames(
    data: &[u8],
    limit: usize,
    want_md5: bool,
) -> Result<(Vec<FrameRecord>, usize)> {
    use bitvue_vp9::{extract_vp9_frames as vp9_extract, Vp9FrameType};

    let frames = vp9_extract(data).map_err(|e| anyhow::anyhow!("VP9 parse error: {}", e))?;
    let mut records = Vec::new();

    for frame in frames.iter().take(limit) {
        let key_frame = matches!(frame.frame_type, Vp9FrameType::Key);
        let frame_type = if key_frame { "KEY" } else { "INTER" }.to_string();
        let end = (frame.offset + frame.size).min(data.len());
        let frame_data = &data[frame.offset..end];

        records.push(FrameRecord {
            index: frame.frame_index,
            frame_type,
            size: frame.size,
            pts: None,
            offset: frame.offset as u64,
            key_frame,
            md5_hex: maybe_md5(frame_data, want_md5),
        });
    }

    Ok((records, 0))
}

// ─── Frame table output ───────────────────────────────────────────────────────

fn print_frame_table(records: &[FrameRecord], show_md5: bool) {
    if show_md5 {
        println!(
            "{:<6} {:<8} {:<10} {:<16} {:<12} {:<6} {}",
            "Index", "Type", "Size", "PTS", "Offset", "Key", "MD5"
        );
        println!("{}", "-".repeat(85));
    } else {
        println!(
            "{:<6} {:<8} {:<10} {:<16} {:<12} {}",
            "Index", "Type", "Size", "PTS", "Offset", "Key"
        );
        println!("{}", "-".repeat(66));
    }

    for r in records {
        let pts_str = r
            .pts
            .map(|v| v.to_string())
            .unwrap_or_else(|| "-".to_string());
        if show_md5 {
            println!(
                "{:<6} {:<8} {:<10} {:<16} {:<12} {:<6} {}",
                r.index,
                r.frame_type,
                r.size,
                pts_str,
                r.offset,
                if r.key_frame { "Y" } else { "" },
                r.md5_hex.as_deref().unwrap_or("-"),
            );
        } else {
            println!(
                "{:<6} {:<8} {:<10} {:<16} {:<12} {}",
                r.index,
                r.frame_type,
                r.size,
                pts_str,
                r.offset,
                if r.key_frame { "Y" } else { "" },
            );
        }
    }

    println!("\nTotal: {} frame(s)", records.len());
}

// ─── Statistics ───────────────────────────────────────────────────────────────

fn print_stream_stats(records: &[FrameRecord], detailed: bool) {
    if records.is_empty() {
        println!("Statistics: no frames");
        return;
    }

    // Frame type counts
    let mut type_counts: std::collections::BTreeMap<String, usize> =
        std::collections::BTreeMap::new();
    let mut key_count = 0usize;
    let mut total_bytes = 0usize;

    for r in records {
        *type_counts.entry(r.frame_type.clone()).or_insert(0) += 1;
        if r.key_frame {
            key_count += 1;
        }
        total_bytes += r.size;
    }

    let avg_bytes = total_bytes / records.len();
    let max_frame = records.iter().max_by_key(|r| r.size).unwrap();
    let min_frame = records.iter().min_by_key(|r| r.size).unwrap();

    println!("── Stream Statistics ──────────────────────────────");
    println!("  Frames:     {}", records.len());
    println!("  Key frames: {}", key_count);
    println!(
        "  Total size: {} bytes ({:.2} MB)",
        total_bytes,
        total_bytes as f64 / 1_048_576.0
    );
    println!(
        "  Avg size:   {} bytes ({:.2} KB)",
        avg_bytes,
        avg_bytes as f64 / 1024.0
    );
    println!(
        "  Max frame:  #{} {} bytes",
        max_frame.index, max_frame.size
    );
    println!(
        "  Min frame:  #{} {} bytes",
        min_frame.index, min_frame.size
    );

    println!("  Frame type distribution:");
    for (t, n) in &type_counts {
        println!(
            "    {:8} {:>5}  ({:.1}%)",
            t,
            n,
            (*n as f64 / records.len() as f64) * 100.0
        );
    }

    if detailed {
        // Size buckets (logarithmic)
        println!("  Size distribution:");
        let buckets = [1024, 4096, 16384, 65536, usize::MAX];
        let labels = ["<1KB", "<4KB", "<16KB", "<64KB", "≥64KB"];
        let mut counts = [0usize; 5];
        for r in records {
            for (i, &threshold) in buckets.iter().enumerate() {
                if r.size < threshold {
                    counts[i] += 1;
                    break;
                }
            }
        }
        for (label, count) in labels.iter().zip(counts.iter()) {
            if *count > 0 {
                println!(
                    "    {:8} {:>5}  ({:.1}%)",
                    label,
                    count,
                    (*count as f64 / records.len() as f64) * 100.0
                );
            }
        }
    }
}

// ─── PSNR ─────────────────────────────────────────────────────────────────────

fn compute_psnr(
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
    println!("{:<8} {}", "Frame", "PSNR (dB)");
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

// ─── YUV / Y4M dump ───────────────────────────────────────────────────────────

fn do_yuv_dump(
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
    dec.flush();
    drain_frames_luma(&mut dec, &mut out);
    Ok(out)
}

fn drain_frames_luma(dec: &mut bitvue_decode::Av1Decoder, out: &mut Vec<(Vec<u8>, usize, usize)>) {
    loop {
        match dec.get_frame() {
            Ok(f) => {
                let y: Vec<u8> = if f.bit_depth == 8 {
                    f.y_plane.to_vec()
                } else {
                    f.y_plane.chunks(2).map(|c| c[0]).collect()
                };
                out.push((y, f.width as usize, f.height as usize));
            }
            Err(_) => break,
        }
    }
}

/// Decode AV1 IVF → (full_yuv_bytes, width, height, bit_depth) per frame.
fn decode_av1_yuv(data: &[u8], limit: usize) -> Result<Vec<(Vec<u8>, usize, usize, u8)>> {
    use bitvue_decode::Av1Decoder;
    let (_hdr, frames) = parse_ivf_frames(data).map_err(|e| anyhow::anyhow!("IVF error: {}", e))?;
    let mut dec = Av1Decoder::new().map_err(|e| anyhow::anyhow!("Decoder init: {}", e))?;
    let mut out: Vec<(Vec<u8>, usize, usize, u8)> = Vec::new();

    for f in &frames {
        dec.send_data_owned(f.data.clone(), f.timestamp as i64)
            .map_err(|e| anyhow::anyhow!("Decode send: {}", e))?;
        drain_frames_yuv(&mut dec, &mut out);
        if out.len() >= limit {
            break;
        }
    }
    if out.len() < limit {
        dec.flush();
        drain_frames_yuv(&mut dec, &mut out);
    }
    Ok(out)
}

fn drain_frames_yuv(
    dec: &mut bitvue_decode::Av1Decoder,
    out: &mut Vec<(Vec<u8>, usize, usize, u8)>,
) {
    loop {
        match dec.get_frame() {
            Ok(f) => {
                let mut combined = f.y_plane.to_vec();
                if let Some(ref u) = f.u_plane {
                    combined.extend_from_slice(u);
                }
                if let Some(ref v) = f.v_plane {
                    combined.extend_from_slice(v);
                }
                out.push((combined, f.width as usize, f.height as usize, f.bit_depth));
            }
            Err(_) => break,
        }
    }
}

// AVS3 ─────────────────────────────────────────────────────────────────────────

fn extract_avs3_frames(
    data: &[u8],
    limit: usize,
    want_md5: bool,
) -> Result<(Vec<FrameRecord>, usize)> {
    use bitvue_avs3::extract_avs3_frames as avs3_extract;

    let result =
        avs3_extract(data, limit).map_err(|e| anyhow::anyhow!("AVS3 parse error: {}", e))?;

    let mut records = Vec::new();
    for frame in &result.frames {
        let end = (frame.offset + frame.size).min(data.len());
        let frame_data = &data[frame.offset..end];
        records.push(FrameRecord {
            index: frame.frame_index,
            frame_type: frame.frame_type_str().to_string(),
            size: frame.size,
            pts: None,
            offset: frame.offset as u64,
            key_frame: frame.is_key_frame(),
            md5_hex: maybe_md5(frame_data, want_md5),
        });
    }

    Ok((records, result.parse_errors))
}

fn extract_jpegxs_frames(
    data: &[u8],
    limit: usize,
    want_md5: bool,
) -> Result<(Vec<FrameRecord>, usize)> {
    use bitvue_jpegxs::extract_jpegxs_frames as jxs_extract;

    let result =
        jxs_extract(data, limit).map_err(|e| anyhow::anyhow!("JPEG XS parse error: {}", e))?;

    let mut records = Vec::new();
    for frame in &result.frames {
        let end = (frame.offset + frame.size).min(data.len());
        let frame_data = &data[frame.offset..end];
        records.push(FrameRecord {
            index: frame.frame_index,
            frame_type: "JXS".to_string(),
            size: frame.size,
            pts: None,
            offset: frame.offset as u64,
            key_frame: true, // JPEG XS is intra-only
            md5_hex: maybe_md5(frame_data, want_md5),
        });
    }

    Ok((records, result.parse_errors))
}

fn extract_vc3_frames_cli(
    data: &[u8],
    limit: usize,
    want_md5: bool,
) -> Result<(Vec<FrameRecord>, usize)> {
    use bitvue_vc3::extract_vc3_frames;

    let result =
        extract_vc3_frames(data, limit).map_err(|e| anyhow::anyhow!("VC-3 parse error: {}", e))?;

    let mut records = Vec::new();
    for frame in &result.frames {
        let end = (frame.offset + frame.frame_size as usize).min(data.len());
        let frame_data = &data[frame.offset..end];
        records.push(FrameRecord {
            index: frame.frame_index,
            frame_type: frame.frame_type_str().to_string(),
            size: frame.frame_size as usize,
            pts: None,
            offset: frame.offset as u64,
            key_frame: true, // VC-3 frames are all intra
            md5_hex: maybe_md5(frame_data, want_md5),
        });
    }

    Ok((records, result.parse_errors))
}
