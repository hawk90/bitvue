//! Open dependent file command (for Stream B comparison)

use super::types::FileInfo;
use super::parsers::{parse_by_extension, detect_codec_from_extension};
use crate::commands::AppState;
use bitvue_core::{Command, Event, StreamId, UnitModel};
use std::path::PathBuf;

/// Open a dependent file (Stream B) for comparison
pub async fn open_dependent_file_impl(
    path: String,
    state: tauri::State<'_, AppState>,
) -> Result<FileInfo, String> {
    tracing::info!("open_dependent_file: Opening file: {}", path);

    let path_buf = PathBuf::from(&path);

    // Check if file exists
    if !path_buf.exists() {
        tracing::error!("open_dependent_file: File does not exist: {}", path);
        return Ok(FileInfo {
            path,
            size: 0,
            codec: "Unknown".to_string(),
            success: false,
            error: Some("File does not exist".to_string()),
        });
    }

    // Get file size
    let size = path_buf.metadata()
        .and_then(|m| Ok(m.len()))
        .unwrap_or(0);

    let ext = path_buf.extension()
        .and_then(|e| e.to_str())
        .unwrap_or("unknown");

    let codec = detect_codec_from_extension(ext);
    tracing::info!("open_dependent_file: Detected codec: {}, file size: {} bytes", codec, size);

    // Use bitvue-core to open the file for Stream B
    let core = state.core.lock().map_err(|e| e.to_string())?;
    let events = core.handle_command(Command::OpenFile {
        stream: StreamId::B,
        path: path_buf.clone(),
    });

    // Check for errors in events
    let mut success = false;
    let mut error = None;

    for event in events {
        match event {
            Event::ModelUpdated { kind: _, stream: _ } => {
                success = true;
                tracing::info!("open_dependent_file: ModelUpdated event received");
            }
            Event::DiagnosticAdded { diagnostic } => {
                error = Some(diagnostic.message.clone());
                tracing::error!("open_dependent_file: DiagnosticAdded: {}", diagnostic.message);
            }
            _ => {}
        }
    }

    // Parse the file and populate units for Stream B
    if success {
        tracing::info!("open_dependent_file: File opened successfully, parsing frames...");

        let parse_result = parse_by_extension(&path_buf, ext, StreamId::B);

        match parse_result {
            Ok(units) => {
                tracing::info!("open_dependent_file: Parsed {} units for Stream B", units.len());

                // Populate units in StreamState for Stream B
                let stream_b_lock = core.get_stream(StreamId::B);
                let mut stream_b = stream_b_lock.write();

                let unit_count = units.len();
                let frame_count = units.iter().filter(|u| u.frame_index.is_some()).count();

                stream_b.units = Some(UnitModel {
                    units,
                    unit_count,
                    frame_count,
                });

                tracing::info!("open_dependent_file: Stream B populated with {} units ({} frames)",
                    unit_count, frame_count);
            }
            Err(e) => {
                tracing::error!("open_dependent_file: Failed to parse file: {}", e);
                error = Some(format!("Parse error: {}", e));
            }
        }
    }

    Ok(FileInfo {
        path,
        size,
        codec,
        success: error.is_none(),
        error,
    })
}
