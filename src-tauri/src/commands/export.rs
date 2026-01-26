//! Export commands for Bitvue
//!
//! Commands for exporting frame data, analysis results, and reports.

use crate::commands::AppState;
use bitvue_core::StreamId;
use serde_json::json;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

/// Export frame data to CSV format
#[tauri::command]
pub async fn export_frames_csv(
    state: tauri::State<'_, AppState>,
    output_path: String,
) -> Result<String, String> {
    log::info!("export_frames_csv: Exporting to {}", output_path);

    let core = state.core.lock().map_err(|e| e.to_string())?;
    let stream_a = core.get_stream(StreamId::A);
    let stream_a = stream_a.read();

    let units = stream_a.units.as_ref().ok_or("No data loaded")?;

    let path = PathBuf::from(&output_path);
    let mut file = File::create(&path)
        .map_err(|e| format!("Failed to create file: {}", e))?;

    // Write CSV header
    writeln!(
        file,
        "Frame,Type,Size,PTS,DTS,FrameTypeKey,TemporalLayer,RefFrames,RefSlots"
    )
    .map_err(|e| format!("Failed to write header: {}", e))?;

    // Write frame data
    for (idx, unit) in units.units.iter().enumerate() {
        let ref_frames_str = unit.ref_frames
            .as_ref()
            .map(|refs| refs.iter().map(|r| r.to_string()).collect::<Vec<_>>().join(";"))
            .unwrap_or_default();

        let ref_slots_str = unit.ref_slots
            .as_ref()
            .map(|slots| slots.iter().map(|s| s.to_string()).collect::<Vec<_>>().join(";"))
            .unwrap_or_default();

        writeln!(
            file,
            "{},{},{},{},{},{},{},{},{}",
            idx,
            unit.frame_type.as_deref().unwrap_or("UNKNOWN"),
            unit.size,
            unit.pts.map(|p| p.to_string()).unwrap_or_default(),
            unit.dts.map(|d| d.to_string()).unwrap_or_default(),
            unit.unit_type,
            unit.temporal_id.map(|t| t.to_string()).unwrap_or_default(),
            ref_frames_str,
            ref_slots_str
        )
        .map_err(|e| format!("Failed to write row: {}", e))?;
    }

    log::info!("export_frames_csv: Successfully exported {} frames", units.units.len());
    Ok(format!("Exported {} frames to {}", units.units.len(), output_path))
}

/// Export frame data to JSON format
#[tauri::command]
pub async fn export_frames_json(
    state: tauri::State<'_, AppState>,
    output_path: String,
) -> Result<String, String> {
    log::info!("export_frames_json: Exporting to {}", output_path);

    let core = state.core.lock().map_err(|e| e.to_string())?;
    let stream_a = core.get_stream(StreamId::A);
    let stream_a = stream_a.read();

    let units = stream_a.units.as_ref().ok_or("No data loaded")?;

    let frames_json: Vec<serde_json::Value> = units.units.iter().enumerate().map(|(idx, unit)| {
        json!({
            "frame_index": idx,
            "frame_type": unit.frame_type,
            "unit_type": unit.unit_type,
            "size": unit.size,
            "pts": unit.pts,
            "dts": unit.dts,
            "temporal_id": unit.temporal_id,
            "ref_frames": unit.ref_frames,
            "ref_slots": unit.ref_slots,
            "qp_avg": unit.qp_avg,
        })
    }).collect();

    let output = json!({
        "frames": frames_json,
        "total_frames": units.units.len(),
        "file_path": stream_a.file_path,
    });

    let path = PathBuf::from(&output_path);
    let mut file = File::create(&path)
        .map_err(|e| format!("Failed to create file: {}", e))?;

    file.write_all(serde_json::to_string_pretty(&output)
        .map_err(|e| format!("Failed to serialize JSON: {}", e))?.as_bytes())
        .map_err(|e| format!("Failed to write file: {}", e))?;

    log::info!("export_frames_json: Successfully exported {} frames", units.units.len());
    Ok(format!("Exported {} frames to {}", units.units.len(), output_path))
}

/// Export analysis report (text format)
#[tauri::command]
pub async fn export_analysis_report(
    state: tauri::State<'_, AppState>,
    output_path: String,
    _include_syntax: bool,
) -> Result<String, String> {
    log::info!("export_analysis_report: Exporting to {}", output_path);

    let core = state.core.lock().map_err(|e| e.to_string())?;
    let stream_a = core.get_stream(StreamId::A);
    let stream_a = stream_a.read();

    let units = stream_a.units.as_ref().ok_or("No data loaded")?;

    let path = PathBuf::from(&output_path);
    let mut file = File::create(&path)
        .map_err(|e| format!("Failed to create file: {}", e))?;

    // Write header
    writeln!(file, "Bitvue Analysis Report").map_err(|e| format!("Failed to write: {}", e))?;
    writeln!(file, "{}", "=".repeat(40)).map_err(|e| format!("Failed to write: {}", e))?;
    writeln!(file).map_err(|e| format!("Failed to write: {}", e))?;

    // Stream info
    writeln!(file, "File: {:?}", stream_a.file_path)
        .map_err(|e| format!("Failed to write: {}", e))?;
    writeln!(file, "Total Frames: {}", units.units.len())
        .map_err(|e| format!("Failed to write: {}", e))?;
    writeln!(file).map_err(|e| format!("Failed to write: {}", e))?;

    // Frame type statistics
    let i_frames = units.units.iter().filter(|u| u.frame_type.as_deref() == Some("I") || u.frame_type.as_deref() == Some("KEY")).count();
    let p_frames = units.units.iter().filter(|u| u.frame_type.as_deref() == Some("P") || u.frame_type.as_deref() == Some("INTER")).count();
    let b_frames = units.units.iter().filter(|u| u.frame_type.as_deref() == Some("B")).count();

    writeln!(file, "Frame Type Distribution:").map_err(|e| format!("Failed to write: {}", e))?;
    writeln!(file, "  I-Frames: {} ({:.1}%)", i_frames, (i_frames as f64 / units.units.len() as f64) * 100.0)
        .map_err(|e| format!("Failed to write: {}", e))?;
    writeln!(file, "  P-Frames: {} ({:.1}%)", p_frames, (p_frames as f64 / units.units.len() as f64) * 100.0)
        .map_err(|e| format!("Failed to write: {}", e))?;
    writeln!(file, "  B-Frames: {} ({:.1}%)", b_frames, (b_frames as f64 / units.units.len() as f64) * 100.0)
        .map_err(|e| format!("Failed to write: {}", e))?;
    writeln!(file).map_err(|e| format!("Failed to write: {}", e))?;

    // Size statistics
    let total_size: usize = units.units.iter().map(|u| u.size).sum();
    let avg_size = total_size / units.units.len();
    let max_size = units.units.iter().map(|u| u.size).max().unwrap_or(0);
    let min_size = units.units.iter().map(|u| u.size).min().unwrap_or(0);

    writeln!(file, "Frame Size Statistics:").map_err(|e| format!("Failed to write: {}", e))?;
    writeln!(file, "  Total: {} bytes ({:.2} MB)", total_size, total_size as f64 / 1024.0 / 1024.0)
        .map_err(|e| format!("Failed to write: {}", e))?;
    writeln!(file, "  Average: {} bytes", avg_size)
        .map_err(|e| format!("Failed to write: {}", e))?;
    writeln!(file, "  Max: {} bytes", max_size)
        .map_err(|e| format!("Failed to write: {}", e))?;
    writeln!(file, "  Min: {} bytes", min_size)
        .map_err(|e| format!("Failed to write: {}", e))?;
    writeln!(file).map_err(|e| format!("Failed to write: {}", e))?;

    // GOP structure
    writeln!(file, "GOP Structure:").map_err(|e| format!("Failed to write: {}", e))?;
    let mut gop_starts = Vec::new();
    for (idx, unit) in units.units.iter().enumerate() {
        if unit.frame_type.as_deref() == Some("I") || unit.frame_type.as_deref() == Some("KEY") {
            gop_starts.push(idx);
        }
    }
    writeln!(file, "  Number of GOPs: {}", gop_starts.len())
        .map_err(|e| format!("Failed to write: {}", e))?;

    if gop_starts.len() > 1 {
        let gop_sizes: Vec<usize> = gop_starts.windows(2).map(|w| w[1] - w[0]).collect();
        let avg_gop = gop_sizes.iter().sum::<usize>() / gop_sizes.len();
        writeln!(file, "  Average GOP size: {}", avg_gop)
            .map_err(|e| format!("Failed to write: {}", e))?;
    }

    writeln!(file).map_err(|e| format!("Failed to write: {}", e))?;
    writeln!(file, "Generated by Bitvue 1.0.0").map_err(|e| format!("Failed to write: {}", e))?;

    log::info!("export_analysis_report: Successfully exported report");
    Ok(format!("Exported analysis report to {}", output_path))
}
