//! Bitvue CLI - AV1 Bitstream Analyzer Command Line Interface
//!
//! Professional command-line tool for analyzing AV1, H.264, H.265, VP9, and VVC bitstreams.

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod commands;

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
