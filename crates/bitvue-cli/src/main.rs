//! Bitvue CLI - AV1 Bitstream Analyzer Command Line Interface
//!
//! Professional command-line tool for analyzing AV1, H.264, H.265, VP9, and VVC bitstreams.

use anyhow::Result;
use bitvue_cli::commands;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// Bitvue - Professional AV1 Bitstream Analyzer
#[derive(Parser, Debug)]
#[command(name = "bitvue")]
#[command(about = "Analyze video bitstreams (AV1, H.264, H.265, VP9, VVC)", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Enable verbose logging (-v, -vv, -vvv)
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

    /// Input file path (IVF, MP4, MKV, raw bitstream)
    #[arg(short, long)]
    input: Option<PathBuf>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Decode a bitstream with VQ Analyzer-compatible flags
    ///
    /// Examples:
    ///   bitvue decode input.ivf --av1 --frames 100 --stats --md5
    ///   bitvue decode input.hevc --hevc --stats -o decoded.yuv --y4m
    ///   bitvue decode input.ivf --psnr --reference ref.ivf
    ///   bitvue decode input.ivf --regress
    Decode {
        /// Input bitstream file
        #[arg(value_name = "FILE")]
        file: PathBuf,

        /// Force codec: AV1
        #[arg(long = "av1", group = "force_codec_group")]
        codec_av1: bool,
        /// Force codec: HEVC/H.265
        #[arg(long = "hevc", group = "force_codec_group")]
        codec_hevc: bool,
        /// Force codec: AVC/H.264
        #[arg(long = "avc", group = "force_codec_group")]
        codec_avc: bool,
        /// Force codec: VP9
        #[arg(long = "vp9", group = "force_codec_group")]
        codec_vp9: bool,
        /// Force codec: VVC/H.266
        #[arg(long = "vvc", group = "force_codec_group")]
        codec_vvc: bool,
        /// Force codec: MPEG-2 Video
        #[arg(long = "mpeg2", group = "force_codec_group")]
        codec_mpeg2: bool,
        /// Force codec: AVS3
        #[arg(long = "avs3", group = "force_codec_group")]
        codec_avs3: bool,

        /// Maximum number of frames to process (0 = all)
        #[arg(long = "frames", default_value = "0")]
        max_frames: usize,

        /// Compute per-frame MD5 checksums (for regression testing)
        #[arg(long)]
        md5: bool,

        /// Print stream statistics (frame type distribution, sizes)
        #[arg(long)]
        stats: bool,

        /// Print detailed stream statistics including size distribution
        #[arg(long)]
        stream_stats: bool,

        /// Output decoded YUV to file
        #[arg(short = 'o', long)]
        output: Option<PathBuf>,

        /// Write output in Y4M format (adds YUV4MPEG2 header)
        #[arg(long)]
        y4m: bool,

        /// Dump decoded YUV to output (same as -o with auto-generated filename)
        #[arg(long)]
        dump: bool,

        /// Output bit depth (8 or 10; default: same as input)
        #[arg(long)]
        dump_bitdepth: Option<u8>,

        /// Headless decode — silent, exit 0 on success, exit 1 on any error
        #[arg(long)]
        regress: bool,

        /// Output frames in display order (vs. decode order)
        #[arg(long)]
        display_order: bool,

        /// Disable crop (output full coded dimensions)
        #[arg(long)]
        no_crop: bool,

        /// Performance mode: 0=quality, 1=speed, 2=max-speed
        #[arg(long, default_value = "0", value_parser = clap::value_parser!(u8).range(0..=2))]
        fast: u8,

        /// AV1: output pre-grain and post-grain frames separately
        #[arg(long)]
        film_grain: bool,

        /// Write parse errors to a log file
        #[arg(long)]
        errors: Option<PathBuf>,

        /// Compute PSNR (requires --reference)
        #[arg(long)]
        psnr: bool,

        /// Reference file for PSNR/SSIM calculation
        #[arg(long)]
        reference: Option<PathBuf>,
    },

    /// Analyze a video file and display stream information
    Info {
        /// Video file path
        #[arg(short, long)]
        file: PathBuf,
    },

    /// List all frames in the video
    Frames {
        /// Video file path
        #[arg(short, long)]
        file: PathBuf,

        /// Maximum number of frames to list
        #[arg(short = 'n', long, default_value = "100")]
        limit: usize,

        /// Output format (text, json, csv)
        #[arg(short = 'F', long, default_value = "text")]
        format: String,
    },

    /// Decode and analyze a specific frame
    Analyze {
        /// Video file path
        #[arg(short, long)]
        file: PathBuf,

        /// Frame index (0-based)
        #[arg(short = 'f', long)]
        frame: usize,

        /// Show detailed syntax information
        #[arg(long)]
        syntax: bool,

        /// Show residual data
        #[arg(long)]
        residual: bool,

        /// Show coding flow
        #[arg(long)]
        coding_flow: bool,
    },

    /// Calculate quality metrics between two files
    Quality {
        /// Reference (original) file path
        #[arg(long)]
        reference: PathBuf,

        /// Distorted (encoded) file path
        #[arg(long)]
        distorted: PathBuf,

        /// Frame indices to analyze (comma-separated, or "all")
        #[arg(short = 'f', long, default_value = "0")]
        frames: String,

        /// Metrics to calculate (psnr, ssim, vmaf)
        #[arg(short = 'm', long, default_value = "psnr,ssim")]
        metrics: String,
    },

    /// Export analysis results to file
    Export {
        /// Video file path
        #[arg(short, long)]
        file: PathBuf,

        /// Output file path
        #[arg(short, long)]
        output: PathBuf,

        /// Export format (json, csv, markdown)
        #[arg(long, default_value = "json")]
        format: String,
    },

    /// Batch process multiple files
    Batch {
        /// Directory containing video files
        #[arg(short, long)]
        directory: PathBuf,

        /// File pattern to match (e.g., "*.ivf", "*.mp4")
        #[arg(short, long)]
        pattern: String,

        /// Output directory for results
        #[arg(short, long)]
        output: PathBuf,
    },

    /// Validate bitstream syntax
    Validate {
        /// Video file path
        #[arg(short, long)]
        file: PathBuf,

        /// Exit with error code on first failure
        #[arg(short, long)]
        strict: bool,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging based on verbosity
    let log_level = match cli.verbose {
        0 => "warn",
        1 => "info",
        2 => "debug",
        _ => "trace",
    };
    tracing_subscriber::fmt()
        .with_max_level(log_level.parse().unwrap_or(tracing::Level::WARN))
        .init();

    // Initialize abseil logging (VLOG_LEVEL env var)
    abseil::init_from_env();

    // Execute command
    match cli.command {
        Commands::Decode {
            file,
            codec_av1,
            codec_hevc,
            codec_avc,
            codec_vp9,
            codec_vvc,
            codec_mpeg2,
            codec_avs3,
            max_frames,
            md5,
            stats,
            stream_stats,
            output,
            y4m,
            dump,
            dump_bitdepth,
            regress,
            display_order,
            no_crop,
            fast,
            film_grain,
            errors,
            psnr,
            reference,
        } => {
            use commands::decode::{DecodeConfig, ForceCodec};
            let force_codec = if codec_av1 {
                Some(ForceCodec::AV1)
            } else if codec_hevc {
                Some(ForceCodec::HEVC)
            } else if codec_avc {
                Some(ForceCodec::AVC)
            } else if codec_vp9 {
                Some(ForceCodec::VP9)
            } else if codec_vvc {
                Some(ForceCodec::VVC)
            } else if codec_mpeg2 {
                Some(ForceCodec::MPEG2)
            } else if codec_avs3 {
                Some(ForceCodec::AVS3)
            } else {
                None
            };
            commands::decode::run(DecodeConfig {
                file,
                force_codec,
                max_frames,
                md5,
                stats,
                stream_stats,
                output,
                y4m,
                dump,
                dump_bitdepth,
                regress,
                display_order,
                no_crop,
                fast,
                film_grain,
                errors_file: errors,
                psnr,
                reference,
            })?;
        }
        Commands::Info { file } => {
            commands::info::run(file)?;
        }
        Commands::Frames {
            file,
            limit,
            format,
        } => {
            commands::frames::run(file, limit, &format)?;
        }
        Commands::Analyze {
            file,
            frame,
            syntax,
            residual,
            coding_flow,
        } => {
            commands::analyze::run(file, frame, syntax, residual, coding_flow)?;
        }
        Commands::Quality {
            reference,
            distorted,
            frames,
            metrics,
        } => {
            commands::quality::run(reference, distorted, &frames, &metrics)?;
        }
        Commands::Export {
            file,
            output,
            format,
        } => {
            commands::export::run(file, output, &format)?;
        }
        Commands::Batch {
            directory,
            pattern,
            output,
        } => {
            commands::batch::run(directory, &pattern, output)?;
        }
        Commands::Validate { file, strict } => {
            commands::validate::run(file, strict)?;
        }
    }

    Ok(())
}
