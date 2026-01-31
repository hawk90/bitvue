//! Export commands for Bitvue
//!
//! Commands for exporting frame data, analysis results, and reports.

use crate::commands::AppState;
use crate::commands::file::check_system_directory_access;
use bitvue_core::StreamId;
use serde_json::json;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

/// Validate output path to prevent path traversal and ensure safe file operations
///
/// SECURITY: Canonicalize FIRST before any validation to fully resolve all path
/// traversal attempts. This ensures that paths like `/safe/../../../etc/passwd`
/// are properly caught before any extension or other validation occurs.
///
/// Validation order (critical for security):
/// 1. Canonicalize (resolves all .., symlinks, relative paths)
/// 2. Check system directory access
/// 3. Check extension (on canonicalized path)
/// 4. Check parent directory exists
fn validate_output_path(path: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(path);

    // SECURITY CRITICAL: Canonicalize FIRST before any validation
    // This handles:
    // - All `..` components (path traversal)
    // - Symlinks (including nested symlinks)
    // - Relative path resolution
    // - Path separator normalization
    let canonical = path.canonicalize()
        .map_err(|e| format!("Invalid path: cannot resolve path '{}': {}", path.display(), e))?;

    // Validate the canonical path against system directory restrictions
    // This must happen on the canonicalized path to catch traversal attempts
    check_system_directory_access(&canonical.to_string_lossy())
        .map_err(|e| format!("Path validation failed: {}", e))?;

    // Check if the canonical path has a valid extension (prevents writing to system files)
    // Check extension AFTER canonicalization to ensure we validate the actual destination
    let extension = canonical.extension()
        .and_then(|e| e.to_str())
        .ok_or_else(|| {
            // SECURITY: Don't reveal the file path in error message
            "Invalid path: file has no extension".to_string()
        })?;

    // Only allow specific file extensions
    let allowed_extensions = ["csv", "json", "txt", "md"];
    if !allowed_extensions.contains(&extension.to_lowercase().as_str()) {
        // SECURITY: Reveal allowed extensions but not the specific file path
        return Err(format!(
            "Invalid path: extension '{}' not allowed. Allowed extensions: {:?}",
            extension,
            allowed_extensions
        ));
    }

    // Check if the parent directory exists
    if let Some(parent) = canonical.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            // SECURITY: Don't reveal the parent directory path
            return Err("Invalid path: parent directory does not exist".to_string());
        }
    }

    Ok(canonical)
}

/// Export frame data to CSV format
#[tauri::command]
pub async fn export_frames_csv(
    state: tauri::State<'_, AppState>,
    output_path: String,
) -> Result<String, String> {
    // SECURITY: Don't log output path to prevent information disclosure
    log::info!("export_frames_csv: Exporting frames");

    // Validate output path for security
    let path = validate_output_path(&output_path)?;

    let core = state.core.lock().map_err(|e| e.to_string())?;
    let stream_a = core.get_stream(StreamId::A);
    let stream_a = stream_a.read();

    let units = stream_a.units.as_ref().ok_or("No data loaded")?;
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
    // SECURITY: Don't log output path to prevent information disclosure
    log::info!("export_frames_json: Exporting frames");

    // Validate output path for security
    let path = validate_output_path(&output_path)?;

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
        // SECURITY: Don't include file_path in export to prevent information disclosure
    });

    let mut file = File::create(&path)
        .map_err(|e| format!("Failed to create file: {}", e))?;

    file.write_all(serde_json::to_string_pretty(&output)
        .map_err(|e| format!("Failed to serialize JSON: {}", e))?.as_bytes())
        .map_err(|e| format!("Failed to write file: {}", e))?;

    log::info!("export_frames_json: Successfully exported {} frames", units.units.len());
    // SECURITY: Don't reveal output path in response to prevent information disclosure
    Ok(format!("Exported {} frames", units.units.len()))
}

/// Export analysis report (text format)
#[tauri::command]
pub async fn export_analysis_report(
    state: tauri::State<'_, AppState>,
    output_path: String,
    _include_syntax: bool,
) -> Result<String, String> {
    // SECURITY: Don't log output path to prevent information disclosure
    log::info!("export_analysis_report: Exporting analysis report");

    // Validate output path for security
    let path = validate_output_path(&output_path)?;

    let core = state.core.lock().map_err(|e| e.to_string())?;
    let stream_a = core.get_stream(StreamId::A);
    let stream_a = stream_a.read();

    let units = stream_a.units.as_ref().ok_or("No data loaded")?;

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

    // Get total frame count once for reuse
    let total_frames = units.units.len();

    writeln!(file, "Frame Type Distribution:").map_err(|e| format!("Failed to write: {}", e))?;
    // Guard against division by zero
    if total_frames > 0 {
        writeln!(file, "  I-Frames: {} ({:.1}%)", i_frames, (i_frames as f64 / total_frames as f64) * 100.0)
            .map_err(|e| format!("Failed to write: {}", e))?;
        writeln!(file, "  P-Frames: {} ({:.1}%)", p_frames, (p_frames as f64 / total_frames as f64) * 100.0)
            .map_err(|e| format!("Failed to write: {}", e))?;
        writeln!(file, "  B-Frames: {} ({:.1}%)", b_frames, (b_frames as f64 / total_frames as f64) * 100.0)
            .map_err(|e| format!("Failed to write: {}", e))?;
    } else {
        writeln!(file, "  I-Frames: {} (0.0%)", i_frames)
            .map_err(|e| format!("Failed to write: {}", e))?;
        writeln!(file, "  P-Frames: {} (0.0%)", p_frames)
            .map_err(|e| format!("Failed to write: {}", e))?;
        writeln!(file, "  B-Frames: {} (0.0%)", b_frames)
            .map_err(|e| format!("Failed to write: {}", e))?;
    }
    writeln!(file).map_err(|e| format!("Failed to write: {}", e))?;

    // Size statistics
    let total_size: usize = units.units.iter().map(|u| u.size).sum();
    // Guard against division by zero
    let avg_size = if total_frames > 0 { total_size / total_frames } else { 0 };
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
        // Guard against division by zero
        let avg_gop = if !gop_sizes.is_empty() {
            gop_sizes.iter().sum::<usize>() / gop_sizes.len()
        } else {
            0
        };
        writeln!(file, "  Average GOP size: {}", avg_gop)
            .map_err(|e| format!("Failed to write: {}", e))?;
    }

    writeln!(file).map_err(|e| format!("Failed to write: {}", e))?;
    writeln!(file, "Generated by Bitvue 1.0.0").map_err(|e| format!("Failed to write: {}", e))?;

    log::info!("export_analysis_report: Successfully exported report");
    Ok(format!("Exported analysis report to {}", output_path))
}
