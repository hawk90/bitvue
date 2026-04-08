//! Display stream information about a video file

use anyhow::{Context, Result};
use bitvue_av1_codec::{
    parse_ivf_frames, parse_ivf_header, parse_sequence_header, ObuIterator, ObuType,
};
use bitvue_formats::container::{detect_container_format, ContainerFormat};
use bitvue_formats::mp4::parse_mp4;
use std::path::PathBuf;

pub fn run(file_path: PathBuf) -> Result<()> {
    println!("Bitvue CLI - Video File Analyzer");
    println!("================================");

    if !file_path.exists() {
        anyhow::bail!("File not found: {}", file_path.display());
    }

    let file_data = std::fs::read(&file_path)
        .with_context(|| format!("Failed to read file: {}", file_path.display()))?;

    let container = detect_container_format(&file_path).unwrap_or(ContainerFormat::Unknown);

    println!("File:   {}", file_path.display());
    println!(
        "Size:   {} bytes ({:.1} MB)",
        file_data.len(),
        file_data.len() as f64 / 1_048_576.0
    );
    println!("Format: {:?}", container);

    match container {
        ContainerFormat::IVF => print_ivf_info(&file_data),
        ContainerFormat::MP4 => print_mp4_info(&file_data),
        _ => {
            println!("(Detailed info not yet available for this container format)");
        }
    }

    Ok(())
}

fn print_ivf_info(data: &[u8]) {
    match parse_ivf_header(data) {
        Ok(header) => {
            let fourcc = std::str::from_utf8(&header.fourcc).unwrap_or("????");
            let codec = match &header.fourcc {
                b"AV01" => "AV1",
                b"VP90" => "VP9",
                b"VP80" => "VP8",
                _ => "Unknown",
            };
            println!("Codec:  {} (FourCC: {})", codec, fourcc);
            println!("Width:  {}", header.width);
            println!("Height: {}", header.height);
            println!(
                "Frame rate: {}/{}",
                header.framerate_num, header.framerate_den
            );
            println!("Frames (header): {}", header.frame_count);

            // Count actual frames parsed
            if let Ok((_hdr, frames)) = parse_ivf_frames(data) {
                println!("Frames (parsed): {}", frames.len());

                // Try to extract sequence header info from AV1 OBUs
                if &header.fourcc == b"AV01" {
                    for ivf_frame in frames.iter().take(10) {
                        for obu in ObuIterator::new(&ivf_frame.data) {
                            if let Ok(obu) = obu {
                                if obu.header.obu_type == ObuType::SequenceHeader {
                                    if let Ok(seq) = parse_sequence_header(&obu.payload) {
                                        println!("Profile: {:?}", seq.profile);
                                        println!("Bit depth: {}", seq.bit_depth());
                                    }
                                    break;
                                }
                            }
                        }
                    }
                }

                // Frame type distribution
                let mut intra = 0usize;
                let mut inter = 0usize;
                let mut other = 0usize;
                for ivf_frame in &frames {
                    let mut found = false;
                    for obu in ObuIterator::new(&ivf_frame.data) {
                        if let Ok(obu) = obu {
                            if matches!(obu.header.obu_type, ObuType::Frame | ObuType::FrameHeader)
                            {
                                if let Some(ft) = obu.frame_type {
                                    match ft.as_str() {
                                        "I" => intra += 1,
                                        "P" => inter += 1,
                                        _ => other += 1,
                                    }
                                    found = true;
                                    break;
                                }
                            }
                        }
                    }
                    if !found {
                        other += 1;
                    }
                }
                println!("Frame types: I={} P={} Other={}", intra, inter, other);

                // Bitrate estimate (requires timestamps)
                if frames.len() > 1 {
                    let total_bytes: usize = frames.iter().map(|f| f.size as usize).sum();
                    let first_ts = frames.first().map(|f| f.timestamp).unwrap_or(0);
                    let last_ts = frames.last().map(|f| f.timestamp).unwrap_or(0);
                    if last_ts > first_ts && header.framerate_den > 0 {
                        // timestamps are in framerate_num/framerate_den units
                        let duration_secs = (last_ts - first_ts) as f64
                            * (header.framerate_den as f64 / header.framerate_num as f64);
                        if duration_secs > 0.0 {
                            let bitrate_kbps =
                                (total_bytes as f64 * 8.0 / duration_secs / 1000.0) as u64;
                            println!("Bitrate (est.): {} kbps", bitrate_kbps);
                        }
                    }
                }
            }
        }
        Err(e) => {
            println!("(IVF header parse error: {})", e);
        }
    }
}

fn print_mp4_info(data: &[u8]) {
    match parse_mp4(data) {
        Ok(info) => {
            if let Some(codec) = &info.codec {
                println!("Codec:  {}", codec);
            }
            if let Some(brand) = &info.brand {
                println!("Brand:  {}", brand);
            }
            println!("Frames: {}", info.sample_count);
            if info.timescale > 0 && !info.timestamps.is_empty() {
                let last_ts = *info.timestamps.last().unwrap_or(&0);
                let last_dur = info.sample_durations.last().copied().unwrap_or(0);
                let total_units = last_ts + last_dur as u64;
                let duration_secs = total_units as f64 / info.timescale as f64;
                println!("Duration: {:.2}s", duration_secs);
                if duration_secs > 0.0 {
                    let total_bytes: u64 = info.sample_sizes.iter().map(|&s| s as u64).sum();
                    let bitrate_kbps = (total_bytes as f64 * 8.0 / duration_secs / 1000.0) as u64;
                    println!("Bitrate (est.): {} kbps", bitrate_kbps);
                }
            }
            if !info.key_frames.is_empty() {
                println!("Key frames: {}", info.key_frames.len());
            }
        }
        Err(e) => {
            println!("(MP4 parse error: {})", e);
        }
    }
}
