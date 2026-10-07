//! decode — VQ Analyzer-compatible analysis flags
//!
//! Mirrors the VQ Analyzer CLI interface:
//!   bitvue decode <file> [-hevc|-av1|-vp9|-avc] [-frames N] [--md5] [--stats]
//!                        [-o out.yuv] [--y4m] [--psnr --reference ref.ivf]
//!                        [--regress] [--dump-bitdepth N] [--fast N]

use anyhow::{Context, Result};
use bitvue_formats::container::{detect_container_format, ContainerFormat};
use memmap2::MmapOptions;
use rayon::prelude::*;
use std::fs::File;
use std::io::Write as IoWrite;
use std::path::PathBuf;

mod extract;
mod psnr;
mod report;
mod yuv_dump;

use extract::{extract_frames, md5_hex};
use psnr::compute_psnr;
use report::{print_frame_table, print_hevc_nal_stats, print_stream_stats};
use yuv_dump::do_yuv_dump;

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
    /// Limit CPU instruction-set extensions used during decoding.
    /// Recognised tokens: "none", "sse2", "sse4", "avx2", "avx512".
    /// Passed through to the underlying decoder where supported.
    pub cpu_max_feature: Option<String>,
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
        if let Some(ref feat) = cfg.cpu_max_feature {
            println!("CPU:    max feature = {}", feat);
        }
    }

    // ── Extract frames ──────────────────────────────────────────────────────
    let limit = if cfg.max_frames == 0 {
        usize::MAX
    } else {
        cfg.max_frames
    };
    // Pass want_md5=false; we compute MD5 in parallel below after extraction.
    let (mut records, parse_errors) = extract_frames(effective_codec, &file_data, limit, false)?;

    // ── Display-order sort ──────────────────────────────────────────────────
    // When --display-order is set, reorder frames by their PTS/POC so that
    // the frame table and statistics reflect presentation order rather than
    // the bitstream decode order (relevant for B-frame reordering in HEVC/AVC).
    if cfg.display_order {
        records.sort_by_key(|r| r.pts.unwrap_or(r.index as u64));
        // Re-assign sequential indices to reflect the new order.
        for (new_idx, r) in records.iter_mut().enumerate() {
            r.index = new_idx;
        }
    }

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
        // HEVC-specific NAL unit type breakdown (--stream-stats only)
        if cfg.stream_stats && matches!(effective_codec, ForceCodec::HEVC) {
            print_hevc_nal_stats(&file_data);
        }
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
    if cfg.dump || cfg.output.is_some() || cfg.film_grain {
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

pub(super) fn codec_name(c: ForceCodec) -> &'static str {
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
