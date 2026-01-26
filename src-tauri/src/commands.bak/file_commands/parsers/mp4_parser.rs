//! MP4 file parser

use bitvue_core::{StreamId, UnitNode};
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

/// Parse MP4 file and extract frame information
pub fn parse_mp4_file(path: &PathBuf, stream_id: StreamId) -> Result<Vec<UnitNode>, String> {
    tracing::info!("parse_mp4_file: Starting to parse MP4 file: {:?}", path);

    use bitvue_formats::mp4::parse_mp4;

    let mut file = File::open(path).map_err(|e| {
        tracing::error!("parse_mp4_file: Failed to open file: {}", e);
        format!("Failed to open file: {}", e)
    })?;
    let mut data = Vec::new();
    file.read_to_end(&mut data).map_err(|e| {
        tracing::error!("parse_mp4_file: Failed to read file: {}", e);
        format!("Failed to read file: {}", e)
    })?;

    tracing::info!("parse_mp4_file: Read {} bytes from file", data.len());

    let mp4_info = parse_mp4(&data).map_err(|e| {
        tracing::error!("parse_mp4_file: Failed to parse MP4: {:?}", e);
        format!("Failed to parse MP4: {:?}", e)
    })?;

    tracing::info!("MP4 info: codec={:?}, sample_count={}", mp4_info.codec, mp4_info.sample_count);

    // Convert MP4 samples to UnitNodes
    let mut units = Vec::new();
    for idx in 0..mp4_info.sample_count {
        let offset = mp4_info.sample_offsets.get(idx).copied().unwrap_or(0);
        let size = mp4_info.sample_sizes.get(idx).copied().unwrap_or(0) as usize;
        let pts = mp4_info.presentation_timestamps.get(idx).copied();
        let is_key = mp4_info.key_frames.contains(&(idx as u32));

        tracing::debug!("MP4 sample {}: offset={}, size={}, pts={:?}, is_key={}",
            idx, offset, size, pts, is_key);

        let mut unit = UnitNode::new(stream_id, "FRAME".to_string(), offset, size);
        unit.frame_index = Some(idx);
        unit.pts = pts;
        unit.frame_type = Some(if is_key { "I".to_string() } else { "P".to_string() });
        unit.display_name = format!("Sample {} @ 0x{:08X} ({} bytes)", idx, offset, size);
        units.push(unit);
    }

    tracing::info!("Parsed {} samples from MP4 file", units.len());
    Ok(units)
}
