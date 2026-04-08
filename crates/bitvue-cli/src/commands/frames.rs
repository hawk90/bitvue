//! List all frames in the video

use anyhow::{Context, Result};
use bitvue_av1_codec::{parse_ivf_frames, ObuIterator, ObuType};
use bitvue_formats::container::{detect_container_format, ContainerFormat};
use bitvue_formats::mp4::parse_mp4;
use serde::Serialize;
use std::path::PathBuf;

/// Serializable frame record for JSON / CSV output
#[derive(Debug, Serialize)]
struct FrameRecord {
    index: usize,
    frame_type: String,
    size: usize,
    pts: Option<u64>,
    offset: u64,
    key_frame: bool,
}

pub fn run(file_path: PathBuf, limit: usize, format: &str) -> Result<()> {
    if !file_path.exists() {
        anyhow::bail!("File not found: {}", file_path.display());
    }

    let file_data = std::fs::read(&file_path)
        .with_context(|| format!("Failed to read file: {}", file_path.display()))?;

    let container = detect_container_format(&file_path).unwrap_or(ContainerFormat::Unknown);

    let records = match container {
        ContainerFormat::IVF => collect_ivf_frames(&file_data, limit)?,
        ContainerFormat::MP4 => collect_mp4_frames(&file_data, limit)?,
        _ => {
            anyhow::bail!(
                "Frame listing is not yet supported for container format {:?}",
                container
            );
        }
    };

    match format {
        "json" => {
            println!("{}", serde_json::to_string_pretty(&records)?);
        }
        "csv" => {
            println!("index,frame_type,size,pts,offset,key_frame");
            for r in &records {
                println!(
                    "{},{},{},{},{},{}",
                    r.index,
                    r.frame_type,
                    r.size,
                    r.pts.map(|v| v.to_string()).unwrap_or_default(),
                    r.offset,
                    r.key_frame
                );
            }
        }
        _ => {
            // Text table
            println!(
                "{:<6} {:<8} {:<10} {:<16} {:<12} {}",
                "Index", "Type", "Size", "PTS", "Offset", "KeyFrame"
            );
            println!("{}", "-".repeat(66));
            for r in &records {
                println!(
                    "{:<6} {:<8} {:<10} {:<16} {:<12} {}",
                    r.index,
                    r.frame_type,
                    r.size,
                    r.pts
                        .map(|v| v.to_string())
                        .unwrap_or_else(|| "-".to_string()),
                    r.offset,
                    r.key_frame
                );
            }
            println!("\nTotal: {} frames shown", records.len());
        }
    }

    Ok(())
}

fn collect_ivf_frames(data: &[u8], limit: usize) -> Result<Vec<FrameRecord>> {
    let (_header, frames) =
        parse_ivf_frames(data).map_err(|e| anyhow::anyhow!("IVF parse error: {}", e))?;

    // IVF frame header is 12 bytes (4 size + 8 timestamp), plus the 32-byte file header.
    // We re-derive the byte offset from a running count.
    let mut offset: u64 = 32; // skip file header
    let mut records = Vec::with_capacity(frames.len().min(limit));

    for (idx, frame) in frames.iter().enumerate() {
        if records.len() >= limit {
            break;
        }

        let frame_offset = offset + 12; // after per-frame header
        let frame_type = resolve_av1_frame_type(&frame.data);
        let key_frame = frame_type == "I";

        records.push(FrameRecord {
            index: idx,
            frame_type,
            size: frame.size as usize,
            pts: Some(frame.timestamp),
            offset: frame_offset,
            key_frame,
        });

        // Advance: 12-byte per-frame header + frame data
        offset += 12 + frame.size as u64;
    }

    Ok(records)
}

fn collect_mp4_frames(data: &[u8], limit: usize) -> Result<Vec<FrameRecord>> {
    let info = parse_mp4(data).map_err(|e| anyhow::anyhow!("MP4 parse error: {}", e))?;

    let key_frame_set: std::collections::HashSet<u32> = info.key_frames.iter().copied().collect();
    let mut records = Vec::new();

    let count = info.sample_count.min(limit);
    for idx in 0..count {
        let size = info.sample_sizes.get(idx).copied().unwrap_or(0) as usize;
        let pts = info.presentation_timestamps.get(idx).copied();
        let offset = info.sample_offsets.get(idx).copied().unwrap_or(0);
        // key_frames in MP4 are 1-based sample numbers
        let key_frame = key_frame_set.contains(&((idx as u32) + 1));
        let frame_type = if key_frame { "I" } else { "P" }.to_string();

        records.push(FrameRecord {
            index: idx,
            frame_type,
            size,
            pts,
            offset,
            key_frame,
        });
    }

    Ok(records)
}

fn resolve_av1_frame_type(frame_data: &[u8]) -> String {
    for obu in ObuIterator::new(frame_data) {
        if let Ok(obu) = obu {
            if matches!(obu.header.obu_type, ObuType::Frame | ObuType::FrameHeader) {
                if let Some(ft) = obu.frame_type {
                    return ft.as_str().to_string();
                }
            }
        }
    }
    "?".to_string()
}
