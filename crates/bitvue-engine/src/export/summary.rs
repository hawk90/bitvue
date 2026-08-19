//! Summary statistics export

use serde::{Deserialize, Serialize};
use std::io::Write;

use super::types::{ExportFormat, ExportResult};
use crate::timeline::TimelineFrame;

/// Summary statistics for export
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportSummary {
    pub total_frames: usize,
    pub total_bytes: u64,
    pub key_frame_count: usize,
    pub error_count: usize,
    pub avg_frame_size: f64,
    pub min_frame_size: u64,
    pub max_frame_size: u64,
    pub duration_pts: Option<u64>,
    pub metrics_available: bool,
}

impl ExportSummary {
    pub fn from_frames(frames: &[TimelineFrame]) -> Self {
        let total_frames = frames.len();
        let total_bytes: u64 = frames.iter().map(|f| f.size_bytes).sum();

        let key_frame_count = frames
            .iter()
            .filter(|f| matches!(f.marker, crate::timeline::FrameMarker::Key))
            .count();

        let error_count = frames
            .iter()
            .filter(|f| matches!(f.marker, crate::timeline::FrameMarker::Error))
            .count();

        let min_frame_size = frames.iter().map(|f| f.size_bytes).min().unwrap_or(0);
        let max_frame_size = frames.iter().map(|f| f.size_bytes).max().unwrap_or(0);
        let avg_frame_size = if total_frames > 0 {
            total_bytes as f64 / total_frames as f64
        } else {
            0.0
        };

        // Duration from PTS
        let duration_pts = if let (Some(first), Some(last)) = (
            frames.first().and_then(|f| f.pts),
            frames.last().and_then(|f| f.pts),
        ) {
            Some(last - first)
        } else {
            None
        };

        Self {
            total_frames,
            total_bytes,
            key_frame_count,
            error_count,
            avg_frame_size,
            min_frame_size,
            max_frame_size,
            duration_pts,
            metrics_available: false,
        }
    }
}

/// Export summary to JSON
pub fn export_summary_json<W: Write>(
    summary: &ExportSummary,
    writer: &mut W,
    pretty: bool,
) -> std::io::Result<ExportResult> {
    let json_str = if pretty {
        serde_json::to_string_pretty(summary).map_err(std::io::Error::other)?
    } else {
        serde_json::to_string(summary).map_err(std::io::Error::other)?
    };

    let bytes_written = writer.write(json_str.as_bytes())?;

    Ok(ExportResult {
        format: if pretty {
            ExportFormat::JsonPretty
        } else {
            ExportFormat::Json
        },
        bytes_written,
        row_count: 1,
    })
}
