//! Timeline frame export (CSV and JSON)

use serde::{Deserialize, Serialize};
use std::io::Write;

use super::types::{ExportConfig, ExportFormat, ExportResult};
use crate::timeline::TimelineFrame;

/// Timeline frame export row
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameExportRow {
    pub display_idx: u64,
    pub frame_type: String,
    pub size_bytes: u64,
    pub pts: Option<u64>,
    pub dts: Option<u64>,
    pub is_key: bool,
    pub has_error: bool,
    pub qp_avg: Option<f32>,
    pub qp_min: Option<i8>,
    pub qp_max: Option<i8>,
}

impl FrameExportRow {
    pub fn from_timeline_frame(frame: &TimelineFrame, display_idx: u64) -> Self {
        Self {
            display_idx,
            frame_type: frame.frame_type.clone(),
            size_bytes: frame.size_bytes,
            pts: frame.pts,
            dts: frame.dts,
            is_key: matches!(frame.marker, crate::timeline::FrameMarker::Key),
            has_error: matches!(frame.marker, crate::timeline::FrameMarker::Error),
            qp_avg: None, // Populated from lane data if available
            qp_min: None,
            qp_max: None,
        }
    }
}

/// Export timeline frames to CSV
pub fn export_frames_csv<W: Write>(
    frames: &[TimelineFrame],
    writer: &mut W,
    config: ExportConfig,
) -> std::io::Result<ExportResult> {
    // Header
    writeln!(
        writer,
        "display_idx,frame_type,size_bytes,pts,dts,is_key,has_error,qp_avg,qp_min,qp_max"
    )?;

    let mut row_count = 0;
    let mut bytes_written = 0;

    for (idx, frame) in frames.iter().enumerate() {
        let display_idx = idx as u64;

        // Apply range filter from config
        if let Some((start, end)) = config.range {
            if display_idx < start || display_idx > end {
                continue;
            }
        }

        let row = FrameExportRow::from_timeline_frame(frame, display_idx);

        let line = format!(
            "{},{},{},{},{},{},{},{},{},{}\n",
            row.display_idx,
            row.frame_type,
            row.size_bytes,
            row.pts.map(|v| v.to_string()).unwrap_or_default(),
            row.dts.map(|v| v.to_string()).unwrap_or_default(),
            row.is_key,
            row.has_error,
            row.qp_avg.map(|v| format!("{:.1}", v)).unwrap_or_default(),
            row.qp_min.map(|v| v.to_string()).unwrap_or_default(),
            row.qp_max.map(|v| v.to_string()).unwrap_or_default(),
        );

        bytes_written += writer.write(line.as_bytes())?;
        row_count += 1;
    }

    Ok(ExportResult {
        format: ExportFormat::Csv,
        bytes_written,
        row_count,
    })
}

/// Export timeline frames to JSON
pub fn export_frames_json<W: Write>(
    frames: &[TimelineFrame],
    writer: &mut W,
    config: ExportConfig,
) -> std::io::Result<ExportResult> {
    let rows: Vec<FrameExportRow> = frames
        .iter()
        .enumerate()
        .filter_map(|(idx, frame)| {
            let display_idx = idx as u64;
            if let Some((start, end)) = config.range {
                if display_idx < start || display_idx > end {
                    return None;
                }
            }
            Some(FrameExportRow::from_timeline_frame(frame, display_idx))
        })
        .collect();

    let row_count = rows.len();

    let json_str = if config.pretty {
        serde_json::to_string_pretty(&rows).map_err(std::io::Error::other)?
    } else {
        serde_json::to_string(&rows).map_err(std::io::Error::other)?
    };

    let bytes_written = writer.write(json_str.as_bytes())?;

    Ok(ExportResult {
        format: if config.pretty {
            ExportFormat::JsonPretty
        } else {
            ExportFormat::Json
        },
        bytes_written,
        row_count,
    })
}
