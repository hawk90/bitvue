//! bitvue CLI - Bitstream analyzer command line interface

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

use bitvue_av1::{extract_required_obus, parse_av1, DependencyGraph, ExtractionRequest, ObuType};
// use bitvue_container::IvfWriter;  // TODO: Re-enable when bitvue-container is implemented

#[derive(Parser)]
#[command(name = "bitvue")]
#[command(author, version, about = "AV1 bitstream analyzer", long_about = None)]
struct Cli {
    /// Enable verbose output
    #[arg(short, long, global = true)]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Display basic information about an AV1 bitstream
    Info {
        /// Path to the AV1 file
        file: PathBuf,
    },

    /// List all OBUs in the bitstream
    Obu {
        /// Path to the AV1 file
        file: PathBuf,

        /// Output as JSON
        #[arg(short, long)]
        json: bool,

        /// Show only specific OBU types (e.g., "sequence,frame")
        #[arg(short = 't', long)]
        types: Option<String>,
    },

    /// Display detailed sequence header information
    Sequence {
        /// Path to the AV1 file
        file: PathBuf,

        /// Output as JSON
        #[arg(short, long)]
        json: bool,
    },

    /// Show statistics about the bitstream
    Stats {
        /// Path to the AV1 file
        file: PathBuf,

        /// Output as JSON
        #[arg(short, long)]
        json: bool,
    },

    /// Extract minimal reproducible clip around a target frame
    Extract {
        /// Path to the AV1 file
        file: PathBuf,

        /// Target frame index (0-based)
        #[arg(short = 'f', long)]
        frame: usize,

        /// Number of context frames before target (default: 0)
        #[arg(short = 'b', long, default_value = "0")]
        before: usize,

        /// Number of context frames after target (default: 0)
        #[arg(short = 'a', long, default_value = "0")]
        after: usize,

        /// Output IVF file path
        #[arg(short, long)]
        output: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Info { file } => cmd_info(&file),
        Commands::Obu { file, json, types } => cmd_obu(&file, json, types),
        Commands::Sequence { file, json } => cmd_sequence(&file, json),
        Commands::Stats { file, json } => cmd_stats(&file, json),
        Commands::Extract {
            file,
            frame,
            before,
            after,
            output,
        } => cmd_extract(&file, frame, before, after, &output),
    }
}

fn cmd_info(file: &PathBuf) -> Result<()> {
    let data = fs::read(file).context("Failed to read file")?;
    let info = parse_av1(&data).context("Failed to parse AV1 bitstream")?;

    println!("File: {}", file.display());
    println!("Size: {} bytes", data.len());
    println!("OBUs: {}", info.obu_count);
    println!("Frames: {}", info.frame_count);

    if let Some(seq) = &info.sequence_header {
        println!();
        println!("Sequence Header:");
        println!("  Profile: {}", seq.profile);
        println!("  Resolution: {}x{}", seq.width(), seq.height());
        println!("  Bit Depth: {}", seq.bit_depth());
        println!("  Chroma: {}", seq.color_config.chroma_subsampling_str());
        println!("  Level: {}.{}", seq.level() / 4, seq.level() % 4);
        println!(
            "  Superblock: {}",
            if seq.use_128x128_superblock {
                "128x128"
            } else {
                "64x64"
            }
        );
    }

    Ok(())
}

fn cmd_obu(file: &PathBuf, json: bool, types: Option<String>) -> Result<()> {
    let data = fs::read(file).context("Failed to read file")?;
    let info = parse_av1(&data).context("Failed to parse AV1 bitstream")?;

    // Parse type filter if provided
    let type_filter: Option<Vec<ObuType>> = types.map(|t| {
        t.split(',')
            .filter_map(|s| match s.trim().to_lowercase().as_str() {
                "sequence" | "sequence_header" | "1" => Some(ObuType::SequenceHeader),
                "temporal" | "temporal_delimiter" | "2" => Some(ObuType::TemporalDelimiter),
                "frame_header" | "3" => Some(ObuType::FrameHeader),
                "tile_group" | "tile" | "4" => Some(ObuType::TileGroup),
                "metadata" | "5" => Some(ObuType::Metadata),
                "frame" | "6" => Some(ObuType::Frame),
                "redundant" | "7" => Some(ObuType::RedundantFrameHeader),
                "tile_list" | "8" => Some(ObuType::TileList),
                "padding" | "15" => Some(ObuType::Padding),
                _ => None,
            })
            .collect()
    });

    let filtered_obus: Vec<_> = info
        .obus
        .iter()
        .filter(|obu| {
            type_filter
                .as_ref()
                .map(|f| f.contains(&obu.header.obu_type))
                .unwrap_or(true)
        })
        .collect();

    if json {
        // JSON output
        let output: Vec<_> = filtered_obus
            .iter()
            .map(|obu| {
                serde_json::json!({
                    "type": obu.header.obu_type.name(),
                    "type_id": obu.header.obu_type as u8,
                    "offset": obu.offset,
                    "size": obu.total_size,
                    "payload_size": obu.payload_size,
                    "has_extension": obu.header.has_extension,
                    "temporal_id": obu.header.temporal_id,
                    "spatial_id": obu.header.spatial_id,
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        // Table output
        println!(
            "{:<4} {:<24} {:>10} {:>12} {:>8}",
            "#", "Type", "Offset", "Size", "Payload"
        );
        println!("{:-<4} {:-<24} {:-<10} {:-<12} {:-<8}", "", "", "", "", "");

        for (i, obu) in filtered_obus.iter().enumerate() {
            println!(
                "{:<4} {:<24} {:>10} {:>12} {:>8}",
                i,
                obu.header.obu_type.name(),
                obu.offset,
                obu.total_size,
                obu.payload_size
            );
        }

        println!();
        println!("Total: {} OBUs", filtered_obus.len());
    }

    Ok(())
}

fn cmd_sequence(file: &PathBuf, json: bool) -> Result<()> {
    let data = fs::read(file).context("Failed to read file")?;
    let info = parse_av1(&data).context("Failed to parse AV1 bitstream")?;

    let seq = info
        .sequence_header
        .context("No sequence header found in bitstream")?;

    if json {
        println!("{}", serde_json::to_string_pretty(&seq)?);
    } else {
        println!("Sequence Header");
        println!("===============");
        println!();
        println!("Profile: {}", seq.profile);
        println!("Still Picture: {}", seq.still_picture);
        println!("Reduced Header: {}", seq.reduced_still_picture_header);
        println!();
        println!("Dimensions:");
        println!("  Max Width: {}", seq.max_frame_width);
        println!("  Max Height: {}", seq.max_frame_height);
        println!();
        println!("Color Config:");
        println!("  Bit Depth: {}", seq.color_config.bit_depth);
        println!("  Mono Chrome: {}", seq.color_config.mono_chrome);
        println!(
            "  Chroma Subsampling: {}",
            seq.color_config.chroma_subsampling_str()
        );
        println!(
            "  Color Range: {}",
            if seq.color_config.color_range {
                "Full"
            } else {
                "Limited"
            }
        );
        println!();
        println!("Tools:");
        println!(
            "  Superblock: {}",
            if seq.use_128x128_superblock {
                "128x128"
            } else {
                "64x64"
            }
        );
        println!("  Filter Intra: {}", seq.enable_filter_intra);
        println!("  Intra Edge Filter: {}", seq.enable_intra_edge_filter);
        println!("  Interintra Compound: {}", seq.enable_interintra_compound);
        println!("  Masked Compound: {}", seq.enable_masked_compound);
        println!("  Warped Motion: {}", seq.enable_warped_motion);
        println!("  Dual Filter: {}", seq.enable_dual_filter);
        println!("  Order Hint: {}", seq.enable_order_hint);
        println!("  JNT Comp: {}", seq.enable_jnt_comp);
        println!("  Ref Frame MVs: {}", seq.enable_ref_frame_mvs);
        println!("  Superres: {}", seq.enable_superres);
        println!("  CDEF: {}", seq.enable_cdef);
        println!("  Restoration: {}", seq.enable_restoration);
        println!("  Film Grain: {}", seq.film_grain_params_present);
        println!();
        println!("Operating Points:");
        for (i, op) in seq.operating_points.iter().enumerate() {
            println!(
                "  [{}] Level {}.{} (idx={})",
                i,
                op.seq_level_idx / 4,
                op.seq_level_idx % 4,
                op.seq_level_idx
            );
        }
    }

    Ok(())
}

fn cmd_stats(file: &PathBuf, json: bool) -> Result<()> {
    let data = fs::read(file).context("Failed to read file")?;
    let info = parse_av1(&data).context("Failed to parse AV1 bitstream")?;

    // Count OBUs by type
    let mut obu_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut obu_sizes: std::collections::HashMap<String, u64> = std::collections::HashMap::new();

    for obu in &info.obus {
        let type_name = obu.header.obu_type.name().to_string();
        *obu_counts.entry(type_name.clone()).or_default() += 1;
        *obu_sizes.entry(type_name).or_default() += obu.total_size;
    }

    if json {
        let stats = serde_json::json!({
            "file_size": data.len(),
            "obu_count": info.obu_count,
            "frame_count": info.frame_count,
            "obu_types": obu_counts,
            "obu_sizes": obu_sizes,
            "sequence": info.sequence_header,
        });
        println!("{}", serde_json::to_string_pretty(&stats)?);
    } else {
        println!("Statistics for: {}", file.display());
        println!();
        println!("File Size: {} bytes", data.len());
        println!("Total OBUs: {}", info.obu_count);
        println!("Frame Count: {}", info.frame_count);
        println!();
        println!("OBU Distribution:");
        println!("{:<24} {:>8} {:>12} {:>8}", "Type", "Count", "Bytes", "%");
        println!("{:-<24} {:-<8} {:-<12} {:-<8}", "", "", "", "");

        let mut sorted: Vec<_> = obu_counts.iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(a.1));

        for (type_name, count) in sorted {
            let size = obu_sizes.get(type_name).unwrap_or(&0);
            let percentage = if data.is_empty() {
                0.0
            } else {
                (*size as f64 / data.len() as f64) * 100.0
            };
            println!(
                "{:<24} {:>8} {:>12} {:>7.2}%",
                type_name, count, size, percentage
            );
        }

        if let Some(seq) = &info.sequence_header {
            println!();
            println!("Sequence Info:");
            println!(
                "  {}x{} @ {} bit ({})",
                seq.width(),
                seq.height(),
                seq.bit_depth(),
                seq.color_config.chroma_subsampling_str()
            );
        }
    }

    Ok(())
}

fn cmd_extract(
    file: &PathBuf,
    target_frame: usize,
    context_before: usize,
    context_after: usize,
    output: &std::path::Path,
) -> Result<()> {
    let data = fs::read(file).context("Failed to read file")?;
    let info = parse_av1(&data).context("Failed to parse AV1 bitstream")?;

    // Build dependency graph
    let graph = DependencyGraph::build(&info.obus);

    println!("Analyzing bitstream: {} frames total", graph.frame_count());

    // Validate target frame
    if target_frame >= graph.frame_count() {
        anyhow::bail!(
            "Target frame {} is out of range (0-{})",
            target_frame,
            graph.frame_count() - 1
        );
    }

    // Create extraction request
    let request = ExtractionRequest {
        target_frame,
        context_before,
        context_after,
        include_sequence_header: true,
    };

    // Extract required OBUs
    let result = extract_required_obus(&request, &graph, &info.obus);

    println!("Extraction plan:");
    println!("  Target frame: {}", target_frame);
    println!("  Context: -{} / +{}", context_before, context_after);
    println!("  Frames to include: {}", result.frame_count);
    println!("  OBUs to include: {}", result.obu_indices.len());
    println!("  Estimated size: {} bytes", result.estimated_size);

    // Get video dimensions from sequence header
    let _seq = info
        .sequence_header
        .as_ref()
        .context("No sequence header found")?;

    // TODO: Re-implement when bitvue-container is available
    // Create IVF writer
    // let mut writer = IvfWriter::new(seq.width() as u16, seq.height() as u16, 30, 1);
    //
    // // Write extracted OBUs to IVF
    // let mut timestamp = 0u64;
    // for &obu_idx in &result.obu_indices {
    //     if let Some(obu) = info.obus.get(obu_idx) {
    //         // Only write frame OBUs (Frame and FrameHeader)
    //         if matches!(obu.header.obu_type, ObuType::Frame | ObuType::FrameHeader) {
    //             writer
    //                 .write_frame(&obu.payload, timestamp)
    //                 .context("Failed to write frame to IVF")?;
    //             timestamp += 1;
    //         }
    //     }
    // }
    //
    // // Save frame count before finalizing
    // let frame_count = writer.frame_count();
    //
    // // Finalize and write to file
    // let output_data = writer.finalize();
    // fs::write(output, &output_data).context("Failed to write output file")?;
    //
    // println!();
    // println!("✓ Extraction complete");
    // println!("  Output: {}", output.display());
    // println!("  Size: {} bytes", output_data.len());
    // println!("  Frames: {}", frame_count);

    println!();
    println!("✗ Extraction not yet implemented (bitvue-container required)");
    println!("  Output would be: {}", output.display());
    println!("  Frames to extract: {}", result.frame_count);

    Ok(())
}
