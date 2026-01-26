//! MPEG-TS file parser

use bitvue_core::{StreamId, UnitNode};
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

/// Parse MPEG-TS file and extract frame information
pub fn parse_ts_file(path: &PathBuf, stream_id: StreamId) -> Result<Vec<UnitNode>, String> {
    tracing::info!("parse_ts_file: Starting to parse TS file: {:?}", path);

    use bitvue_formats::ts::parse_ts;

    let mut file = File::open(path).map_err(|e| {
        tracing::error!("parse_ts_file: Failed to open file: {}", e);
        format!("Failed to open file: {}", e)
    })?;
    let mut data = Vec::new();
    file.read_to_end(&mut data).map_err(|e| {
        tracing::error!("parse_ts_file: Failed to read file: {}", e);
        format!("Failed to read file: {}", e)
    })?;

    tracing::info!("parse_ts_file: Read {} bytes from file", data.len());

    let ts_info = parse_ts(&data).map_err(|e| {
        tracing::error!("parse_ts_file: Failed to parse TS: {:?}", e);
        format!("Failed to parse TS: {:?}", e)
    })?;

    tracing::info!("TS info: video_pid={:?}, sample_count={}",
        ts_info.video_pid, ts_info.sample_count);

    // Convert TS samples to UnitNodes
    let mut units = Vec::new();
    for idx in 0..ts_info.sample_count.min(ts_info.samples.len()) {
        let pts = ts_info.timestamps.get(idx).copied();
        let sample_data = &ts_info.samples[idx];
        let size = sample_data.len();

        tracing::debug!("TS sample {}: size={}, pts={:?}", idx, size, pts);

        let mut unit = UnitNode::new(stream_id, "FRAME".to_string(), 0, size);
        unit.frame_index = Some(idx);
        unit.pts = pts;
        unit.frame_type = Some("I".to_string());
        unit.display_name = format!("Frame {} ({} bytes)", idx, size);
        units.push(unit);
    }

    tracing::info!("Parsed {} samples from TS file", units.len());
    Ok(units)
}
