//! MKV file parser

use bitvue_core::{StreamId, UnitNode};
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

/// Parse MKV file and extract frame information
pub fn parse_mkv_file(path: &PathBuf, stream_id: StreamId) -> Result<Vec<UnitNode>, String> {
    tracing::info!("parse_mkv_file: Starting to parse MKV file: {:?}", path);

    use bitvue_formats::mkv::parse_mkv;

    let mut file = File::open(path).map_err(|e| {
        tracing::error!("parse_mkv_file: Failed to open file: {}", e);
        format!("Failed to open file: {}", e)
    })?;
    let mut data = Vec::new();
    file.read_to_end(&mut data).map_err(|e| {
        tracing::error!("parse_mkv_file: Failed to read file: {}", e);
        format!("Failed to read file: {}", e)
    })?;

    tracing::info!("parse_mkv_file: Read {} bytes from file", data.len());

    let mkv_info = parse_mkv(&data).map_err(|e| {
        tracing::error!("parse_mkv_file: Failed to parse MKV: {:?}", e);
        format!("Failed to parse MKV: {:?}", e)
    })?;

    tracing::info!("MKV info: codec_id={:?}, sample_count={}", mkv_info.codec_id, mkv_info.sample_count);

    // Convert MKV samples to UnitNodes
    let mut units = Vec::new();
    for idx in 0..mkv_info.sample_count.min(mkv_info.samples.len()) {
        let pts = mkv_info.timestamps.get(idx).copied();
        let is_key = mkv_info.key_frames.contains(&(idx as u32));
        let sample_data = &mkv_info.samples[idx];
        let size = sample_data.len();

        tracing::debug!("MKV sample {}: size={}, pts={:?}, is_key={}", idx, size, pts, is_key);

        let mut unit = UnitNode::new(stream_id, "FRAME".to_string(), 0, size);
        unit.frame_index = Some(idx);
        unit.pts = pts;
        unit.frame_type = Some(if is_key { "I".to_string() } else { "P".to_string() });
        unit.display_name = format!("Frame {} ({} bytes)", idx, size);
        units.push(unit);
    }

    tracing::info!("Parsed {} samples from MKV file", units.len());
    Ok(units)
}
