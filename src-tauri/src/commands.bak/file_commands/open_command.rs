//! Open file command

use super::types::FileInfo;
use super::parsers::{parse_by_extension, detect_codec_from_extension};
use crate::commands::AppState;
use bitvue_core::{Command, Event, StreamId, UnitModel};
use std::path::PathBuf;

/// Open and parse a video bitstream file
pub async fn open_file_impl(
    path: String,
    state: tauri::State<'_, AppState>,
) -> Result<FileInfo, String> {
    let path_buf = PathBuf::from(&path);

    tracing::info!("open_file: Opening file at path: {}", path);

    // Check if file exists
    if !path_buf.exists() {
        tracing::error!("open_file: File not found: {}", path);
        return Ok(FileInfo {
            path: path.clone(),
            size: 0,
            codec: "unknown".to_string(),
            success: false,
            error: Some("File not found".to_string()),
        });
    }

    // Get file size and detect codec from extension
    let size = std::fs::metadata(&path_buf)
        .map(|m| m.len())
        .unwrap_or(0);

    let ext = path_buf.extension()
        .and_then(|e| e.to_str())
        .unwrap_or("unknown");

    let codec = detect_codec_from_extension(ext);
    tracing::info!("open_file: Detected codec: {}, file size: {} bytes", codec, size);

    // Use bitvue-core to open the file
    let core = state.core.lock().map_err(|e| e.to_string())?;
    let events = core.handle_command(Command::OpenFile {
        stream: StreamId::A,
        path: path_buf.clone(),
    });

    // Check for errors in events
    let mut success = false;
    let mut error = None;

    for event in events {
        match event {
            Event::ModelUpdated { kind: _, stream: _ } => {
                success = true;
                tracing::info!("open_file: ModelUpdated event received");
            }
            Event::DiagnosticAdded { diagnostic } => {
                error = Some(diagnostic.message.clone());
                tracing::error!("open_file: DiagnosticAdded: {}", diagnostic.message);
            }
            _ => {}
        }
    }

    // Parse the file and populate units
    if success {
        tracing::info!("open_file: File opened successfully, parsing frames...");

        let parse_result = parse_by_extension(&path_buf, ext, StreamId::A);

        match parse_result {
            Ok(units) => {
                tracing::info!("open_file: Parsed {} units, populating StreamState", units.len());

                // Populate units in StreamState
                let stream_a_lock = core.get_stream(StreamId::A);
                let mut stream_a = stream_a_lock.write();

                let unit_count = units.len();
                let frame_count = units.iter().filter(|u| u.frame_index.is_some()).count();

                stream_a.units = Some(UnitModel {
                    units,
                    unit_count,
                    frame_count,
                });

                tracing::info!("open_file: StreamState populated with {} units ({} frames)",
                    unit_count, frame_count);

                // Initialize decode service with the file
                let mut decode_service = state.decode_service.lock()
                    .map_err(|e| format!("Lock error: {}", e))?;
                if let Err(e) = decode_service.set_file(path_buf.clone(), codec.clone()) {
                    tracing::warn!("open_file: Failed to initialize decode service: {}", e);
                    // Don't fail the whole operation if decode service fails
                } else {
                    tracing::info!("open_file: Decode service initialized with codec {}", codec);
                }
            }
            Err(e) => {
                tracing::error!("open_file: Failed to parse file: {}", e);
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
