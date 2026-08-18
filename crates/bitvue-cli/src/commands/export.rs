//! Export analysis results to file

use anyhow::{Context, Result};
use bitvue_av1_codec::{parse_ivf_frames, ObuIterator, ObuType};
use bitvue_formats::container::{detect_container_format, ContainerFormat};
use bitvue_formats::mkv::parse_mkv;
use bitvue_formats::mp4::parse_mp4;
use serde::Serialize;
use std::path::PathBuf;

/// Serializable frame record exported to file
#[derive(Debug, Serialize)]
struct ExportFrame {
    index: usize,
    frame_type: String,
    size: usize,
    pts: Option<u64>,
    offset: u64,
    key_frame: bool,
    temporal_id: Option<u8>,
}

/// Top-level export document
#[derive(Debug, Serialize)]
struct ExportDocument {
    file: String,
    container: String,
    frame_count: usize,
    frames: Vec<ExportFrame>,
}

pub fn run(file_path: PathBuf, output: PathBuf, format: &str) -> Result<()> {
    if !file_path.exists() {
        anyhow::bail!("File not found: {}", file_path.display());
    }

    let file_data = std::fs::read(&file_path)
        .with_context(|| format!("Failed to read file: {}", file_path.display()))?;

    let container = detect_container_format(&file_path).unwrap_or(ContainerFormat::Unknown);

    let frames = match container {
        ContainerFormat::IVF => collect_ivf_frames(&file_data)?,
        ContainerFormat::MP4 => collect_mp4_frames(&file_data)?,
        ContainerFormat::Matroska => collect_mkv_frames(&file_data)?,
        ContainerFormat::AnnexB => collect_annex_b_frames(&file_data)?,
        _ => {
            anyhow::bail!(
                "Export is not yet supported for container format {:?}",
                container
            );
        }
    };

    let doc = ExportDocument {
        file: file_path.display().to_string(),
        container: format!("{:?}", container),
        frame_count: frames.len(),
        frames,
    };

    let output_str = match format {
        "csv" => render_csv(&doc),
        "markdown" | "md" => render_markdown(&doc),
        _ => {
            // Default: JSON
            serde_json::to_string_pretty(&doc).context("JSON serialization failed")?
        }
    };

    std::fs::write(&output, &output_str)
        .with_context(|| format!("Failed to write output: {}", output.display()))?;

    println!(
        "Exported {} frames to {} (format: {})",
        doc.frame_count,
        output.display(),
        format
    );

    Ok(())
}

fn collect_ivf_frames(data: &[u8]) -> Result<Vec<ExportFrame>> {
    let (_header, frames) =
        parse_ivf_frames(data).map_err(|e| anyhow::anyhow!("IVF parse error: {}", e))?;

    let mut offset: u64 = 32; // file header
    let mut records = Vec::with_capacity(frames.len());

    for (idx, frame) in frames.iter().enumerate() {
        let frame_offset = offset + 12; // past per-frame header
        let (frame_type, temporal_id) = resolve_av1_frame_type(&frame.data);
        let key_frame = frame_type == "I";

        records.push(ExportFrame {
            index: idx,
            frame_type,
            size: frame.size as usize,
            pts: Some(frame.timestamp),
            offset: frame_offset,
            key_frame,
            temporal_id,
        });

        offset += 12 + frame.size as u64;
    }

    Ok(records)
}

fn collect_mp4_frames(data: &[u8]) -> Result<Vec<ExportFrame>> {
    let info = parse_mp4(data).map_err(|e| anyhow::anyhow!("MP4 parse error: {}", e))?;

    let key_frame_set: std::collections::HashSet<u32> = info.key_frames.iter().copied().collect();
    let mut records = Vec::with_capacity(info.sample_count);

    for idx in 0..info.sample_count {
        let size = info.sample_sizes.get(idx).copied().unwrap_or(0) as usize;
        let pts = info.presentation_timestamps.get(idx).copied();
        let offset = info.sample_offsets.get(idx).copied().unwrap_or(0);
        let key_frame = key_frame_set.contains(&((idx as u32) + 1));
        let frame_type = if key_frame { "I" } else { "P" }.to_string();

        records.push(ExportFrame {
            index: idx,
            frame_type,
            size,
            pts,
            offset,
            key_frame,
            temporal_id: None,
        });
    }

    Ok(records)
}

fn collect_mkv_frames(data: &[u8]) -> Result<Vec<ExportFrame>> {
    let info = parse_mkv(data).map_err(|e| anyhow::anyhow!("MKV parse error: {}", e))?;

    let key_frame_set: std::collections::HashSet<u32> = info.key_frames.iter().copied().collect();

    // Determine frame types based on codec and key-frame flag
    let codec = info.codec_id.as_deref().unwrap_or("unknown");
    let mut offset: u64 = 0;
    let mut records = Vec::with_capacity(info.sample_count);

    for (idx, sample) in info.samples.iter().enumerate() {
        let key_frame = key_frame_set.contains(&(idx as u32));
        let frame_type = resolve_mkv_frame_type(sample, codec, key_frame);
        let pts = info.timestamps.get(idx).copied();

        records.push(ExportFrame {
            index: idx,
            frame_type,
            size: sample.len(),
            pts,
            offset,
            key_frame,
            temporal_id: None,
        });

        offset += sample.len() as u64;
    }

    Ok(records)
}

fn resolve_mkv_frame_type(data: &[u8], codec: &str, key_frame: bool) -> String {
    // For AV1, inspect OBU frame type
    if codec == "V_AV1" {
        let (ft, _) = resolve_av1_frame_type(data);
        return ft;
    }

    // For H.264 / HEVC: first byte after any length prefix contains NAL type
    // MKV stores in AVCC/HVCC (length-prefixed) format with 4-byte length field
    if (codec.contains("AVC") || codec.contains("H264")) && data.len() >= 5 {
        let nal_type = data[4] & 0x1F; // H.264 NAL type (5 bits)
        return match nal_type {
            5 => "I".to_string(), // IDR
            1 => "P".to_string(), // Non-IDR
            _ => {
                if key_frame {
                    "I".to_string()
                } else {
                    "P".to_string()
                }
            }
        };
    }
    if (codec.contains("HEVC") || codec.contains("H265")) && data.len() >= 6 {
        let nal_type = (data[4] >> 1) & 0x3F; // HEVC NAL type (6 bits)
        return match nal_type {
            19..=21 => "I".to_string(), // IDR
            1..=3 => "P".to_string(),
            _ => {
                if key_frame {
                    "I".to_string()
                } else {
                    "P".to_string()
                }
            }
        };
    }

    if key_frame {
        "I".to_string()
    } else {
        "P".to_string()
    }
}

/// Collect frames from an Annex B byte-stream (H.264 or HEVC raw bitstream).
///
/// Scans for 3-byte (00 00 01) and 4-byte (00 00 00 01) start codes to find
/// slice NAL units. Each access unit (group of NALs sharing a PTS) is one frame.
fn collect_annex_b_frames(data: &[u8]) -> Result<Vec<ExportFrame>> {
    let mut records = Vec::new();
    let mut pos = 0usize;
    let mut frame_idx = 0usize;

    // Locate NAL start codes and derive access unit boundaries
    while pos < data.len() {
        // Find next start code (00 00 01 or 00 00 00 01)
        let Some(sc_pos) = find_start_code(data, pos) else {
            break;
        };

        let sc_len = if sc_pos >= 1 && data[sc_pos - 1] == 0x00 {
            4
        } else {
            3
        };
        let nal_start = sc_pos - (sc_len - 3); // start of start code prefix
        let nal_data_start = sc_pos + 1; // byte after 00 01

        // Find end of NAL (next start code or EOF)
        let nal_end = find_start_code(data, nal_data_start + 1)
            .map(|p| {
                if p >= 1 && data.get(p - 1) == Some(&0x00) {
                    p - 1
                } else {
                    p
                }
            })
            .unwrap_or(data.len());

        // Determine if this is H.264 or HEVC from the NAL header byte
        let nal_byte = data.get(nal_data_start).copied().unwrap_or(0);
        // H.264 NAL type = nal_byte & 0x1F; HEVC NAL type = (nal_byte >> 1) & 0x3F
        let h264_type = nal_byte & 0x1F;
        let is_slice_h264 = matches!(h264_type, 1 | 5); // non-IDR or IDR
        let is_idr_h264 = h264_type == 5;

        let hevc_type = (nal_byte >> 1) & 0x3F;
        let is_slice_hevc = matches!(hevc_type, 1 | 19 | 20 | 21); // TRAIL_R, IDR types
        let is_idr_hevc = matches!(hevc_type, 19..=21);

        if is_slice_h264 || is_slice_hevc {
            let key_frame = is_idr_h264 || is_idr_hevc;
            let frame_type = if key_frame { "I" } else { "P" }.to_string();
            records.push(ExportFrame {
                index: frame_idx,
                frame_type,
                size: nal_end - nal_start,
                pts: None, // Annex B has no PTS
                offset: nal_start as u64,
                key_frame,
                temporal_id: None,
            });
            frame_idx += 1;
        }

        pos = nal_data_start + 1;
    }

    if records.is_empty() {
        anyhow::bail!("No slice NAL units found in Annex B stream");
    }

    Ok(records)
}

/// Find the position of the `01` byte in the next `00 00 01` pattern.
fn find_start_code(data: &[u8], start: usize) -> Option<usize> {
    if start + 2 >= data.len() {
        return None;
    }
    for i in start..data.len() - 2 {
        if data[i] == 0x00 && data[i + 1] == 0x00 && data[i + 2] == 0x01 {
            return Some(i + 2); // position of the 01 byte
        }
    }
    None
}

fn resolve_av1_frame_type(frame_data: &[u8]) -> (String, Option<u8>) {
    let mut frame_type = "?".to_string();
    let mut temporal_id: Option<u8> = None;

    for obu in ObuIterator::new(frame_data).flatten() {
        if matches!(obu.header.obu_type, ObuType::Frame | ObuType::FrameHeader) {
            if let Some(ft) = obu.frame_type {
                frame_type = ft.as_str().to_string();
            }
            // temporal_id is in the OBU extension header (0 means no extension / base layer)
            temporal_id = Some(obu.header.temporal_id);
        }
    }

    (frame_type, temporal_id)
}

fn render_csv(doc: &ExportDocument) -> String {
    let mut out = String::from("index,frame_type,size,pts,offset,key_frame,temporal_id\n");
    for f in &doc.frames {
        out.push_str(&format!(
            "{},{},{},{},{},{},{}\n",
            f.index,
            f.frame_type,
            f.size,
            f.pts.map(|v| v.to_string()).unwrap_or_default(),
            f.offset,
            f.key_frame,
            f.temporal_id.map(|v| v.to_string()).unwrap_or_default(),
        ));
    }
    out
}

fn render_markdown(doc: &ExportDocument) -> String {
    let mut out = format!(
        "# Bitvue Export\n\n**File:** {}\n**Container:** {}\n**Frames:** {}\n\n",
        doc.file, doc.container, doc.frame_count
    );
    out.push_str("| Index | Type | Size | PTS | Offset | KeyFrame |\n");
    out.push_str("|-------|------|------|-----|--------|----------|\n");
    for f in &doc.frames {
        out.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} |\n",
            f.index,
            f.frame_type,
            f.size,
            f.pts
                .map(|v| v.to_string())
                .unwrap_or_else(|| "-".to_string()),
            f.offset,
            f.key_frame,
        ));
    }
    out
}
