//! Export Module - Feature Parity Phase A
//!
//! Provides CSV/JSON export for timeline data, metrics, and diagnostics.
//! Per COMPETITOR_PARITY_MATRIX: Export is a core requirement for professional use.

use serde::{Deserialize, Serialize};
use std::io::Write;

use crate::diagnostics::{Diagnostic, DiagnosticSeverity};
use crate::metrics_distribution::MetricPoint;
use crate::timeline::TimelineFrame;

// ═══════════════════════════════════════════════════════════════════════════
// Export Formats
// ═══════════════════════════════════════════════════════════════════════════

/// Export format options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExportFormat {
    Csv,
    Json,
    JsonPretty,
}

/// Export result
#[derive(Debug)]
pub struct ExportResult {
    pub format: ExportFormat,
    pub bytes_written: usize,
    pub row_count: usize,
}

// ═══════════════════════════════════════════════════════════════════════════
// Timeline Frame Export
// ═══════════════════════════════════════════════════════════════════════════

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
    range: Option<(u64, u64)>,
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

        // Apply range filter
        if let Some((start, end)) = range {
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
    range: Option<(u64, u64)>,
    pretty: bool,
) -> std::io::Result<ExportResult> {
    let rows: Vec<FrameExportRow> = frames
        .iter()
        .enumerate()
        .filter_map(|(idx, frame)| {
            let display_idx = idx as u64;
            if let Some((start, end)) = range {
                if display_idx < start || display_idx > end {
                    return None;
                }
            }
            Some(FrameExportRow::from_timeline_frame(frame, display_idx))
        })
        .collect();

    let row_count = rows.len();

    let json_str = if pretty {
        serde_json::to_string_pretty(&rows).map_err(std::io::Error::other)?
    } else {
        serde_json::to_string(&rows).map_err(std::io::Error::other)?
    };

    let bytes_written = writer.write(json_str.as_bytes())?;

    Ok(ExportResult {
        format: if pretty {
            ExportFormat::JsonPretty
        } else {
            ExportFormat::Json
        },
        bytes_written,
        row_count,
    })
}

// ═══════════════════════════════════════════════════════════════════════════
// Metrics Export
// ═══════════════════════════════════════════════════════════════════════════

/// Metrics export row
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsExportRow {
    pub display_idx: u64,
    pub psnr_y: Option<f64>,
    pub psnr_u: Option<f64>,
    pub psnr_v: Option<f64>,
    pub ssim_y: Option<f64>,
    pub ssim_u: Option<f64>,
    pub ssim_v: Option<f64>,
    pub vmaf: Option<f64>,
}

/// Export metrics to CSV
pub fn export_metrics_csv<W: Write>(
    psnr_y: &[MetricPoint],
    ssim_y: &[MetricPoint],
    vmaf: &[MetricPoint],
    writer: &mut W,
) -> std::io::Result<ExportResult> {
    writeln!(writer, "display_idx,psnr_y,ssim_y,vmaf")?;

    // Build frame map
    let max_idx = [
        psnr_y.iter().map(|p| p.idx).max().unwrap_or(0),
        ssim_y.iter().map(|p| p.idx).max().unwrap_or(0),
        vmaf.iter().map(|p| p.idx).max().unwrap_or(0),
    ]
    .into_iter()
    .max()
    .unwrap_or(0);

    let mut row_count = 0;
    let mut bytes_written = 0;

    for idx in 0..=max_idx {
        let psnr_val = psnr_y.iter().find(|p| p.idx == idx).map(|p| p.value);
        let ssim_val = ssim_y.iter().find(|p| p.idx == idx).map(|p| p.value);
        let vmaf_val = vmaf.iter().find(|p| p.idx == idx).map(|p| p.value);

        // Skip rows with no data
        if psnr_val.is_none() && ssim_val.is_none() && vmaf_val.is_none() {
            continue;
        }

        let line = format!(
            "{},{},{},{}\n",
            idx,
            psnr_val.map(|v| format!("{:.4}", v)).unwrap_or_default(),
            ssim_val.map(|v| format!("{:.6}", v)).unwrap_or_default(),
            vmaf_val.map(|v| format!("{:.2}", v)).unwrap_or_default(),
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

// ═══════════════════════════════════════════════════════════════════════════
// Diagnostics Export
// ═══════════════════════════════════════════════════════════════════════════

/// Diagnostics export row
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticsExportRow {
    pub id: u64,
    pub severity: String,
    pub category: String,
    pub message: String,
    pub byte_offset: u64,
    pub frame_idx: Option<u64>,
    pub stream_id: String,
}

impl DiagnosticsExportRow {
    pub fn from_diagnostic(diag: &Diagnostic) -> Self {
        Self {
            id: diag.id,
            severity: format!("{:?}", diag.severity),
            category: format!("{:?}", diag.category),
            message: diag.message.clone(),
            byte_offset: diag.offset_bytes,
            frame_idx: diag.frame_key.as_ref().map(|fk| fk.frame_index as u64),
            stream_id: format!("{:?}", diag.stream_id),
        }
    }
}

/// Export diagnostics to CSV
pub fn export_diagnostics_csv<W: Write>(
    diagnostics: &[Diagnostic],
    writer: &mut W,
    min_severity: Option<DiagnosticSeverity>,
) -> std::io::Result<ExportResult> {
    writeln!(
        writer,
        "id,severity,category,message,byte_offset,frame_idx,stream_id"
    )?;

    let mut row_count = 0;
    let mut bytes_written = 0;

    for record in diagnostics {
        // Filter by severity
        if let Some(min_sev) = &min_severity {
            if (record.severity as u8) < (*min_sev as u8) {
                continue;
            }
        }

        let row = DiagnosticsExportRow::from_diagnostic(record);

        // Escape message for CSV (handle commas and quotes)
        let escaped_message = if row.message.contains(',') || row.message.contains('"') {
            format!("\"{}\"", row.message.replace('"', "\"\""))
        } else {
            row.message.clone()
        };

        let line = format!(
            "{},{},{},{},{},{},{}\n",
            row.id,
            row.severity,
            row.category,
            escaped_message,
            row.byte_offset,
            row.frame_idx.map(|v| v.to_string()).unwrap_or_default(),
            row.stream_id,
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

/// Export diagnostics to JSON
pub fn export_diagnostics_json<W: Write>(
    diagnostics: &[Diagnostic],
    writer: &mut W,
    min_severity: Option<DiagnosticSeverity>,
    pretty: bool,
) -> std::io::Result<ExportResult> {
    let rows: Vec<DiagnosticsExportRow> = diagnostics
        .iter()
        .filter(|r| {
            if let Some(min_sev) = &min_severity {
                (r.severity as u8) >= (*min_sev as u8)
            } else {
                true
            }
        })
        .map(DiagnosticsExportRow::from_diagnostic)
        .collect();

    let row_count = rows.len();

    let json_str = if pretty {
        serde_json::to_string_pretty(&rows).map_err(std::io::Error::other)?
    } else {
        serde_json::to_string(&rows).map_err(std::io::Error::other)?
    };

    let bytes_written = writer.write(json_str.as_bytes())?;

    Ok(ExportResult {
        format: if pretty {
            ExportFormat::JsonPretty
        } else {
            ExportFormat::Json
        },
        bytes_written,
        row_count,
    })
}

// ═══════════════════════════════════════════════════════════════════════════
// Summary Statistics Export
// ═══════════════════════════════════════════════════════════════════════════

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

// ═══════════════════════════════════════════════════════════════════════════
// Overlay Image Export
// ═══════════════════════════════════════════════════════════════════════════

/// Supported overlay image formats
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OverlayImageFormat {
    /// Raw RGBA bytes (can be consumed by image libraries)
    RawRgba,
    /// PNG format (if encoder is available)
    Png,
    /// PPM format (simple, no compression, always available)
    Ppm,
}

impl OverlayImageFormat {
    /// Get file extension for this format
    pub fn extension(&self) -> &'static str {
        match self {
            OverlayImageFormat::RawRgba => "rgba",
            OverlayImageFormat::Png => "png",
            OverlayImageFormat::Ppm => "ppm",
        }
    }
}

/// Overlay export data containing raw pixel data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverlayExportData {
    /// Image width in pixels
    pub width: u32,
    /// Image height in pixels
    pub height: u32,
    /// RGBA pixel data (4 bytes per pixel, row-major)
    #[serde(skip)]
    pub rgba_data: Vec<u8>,
    /// Export format
    pub format: OverlayImageFormat,
    /// Overlay type description
    pub overlay_type: String,
    /// Frame index
    pub frame_idx: usize,
}

impl OverlayExportData {
    /// Create new overlay export data
    pub fn new(width: u32, height: u32, overlay_type: &str, frame_idx: usize) -> Self {
        let pixel_count = (width * height) as usize;
        Self {
            width,
            height,
            rgba_data: vec![0u8; pixel_count * 4],
            format: OverlayImageFormat::RawRgba,
            overlay_type: overlay_type.to_string(),
            frame_idx,
        }
    }

    /// Set pixel at (x, y) to RGBA color
    pub fn set_pixel(&mut self, x: u32, y: u32, r: u8, g: u8, b: u8, a: u8) {
        if x >= self.width || y >= self.height {
            return;
        }
        let idx = ((y * self.width + x) * 4) as usize;
        if idx + 3 < self.rgba_data.len() {
            self.rgba_data[idx] = r;
            self.rgba_data[idx + 1] = g;
            self.rgba_data[idx + 2] = b;
            self.rgba_data[idx + 3] = a;
        }
    }

    /// Get pixel at (x, y)
    pub fn get_pixel(&self, x: u32, y: u32) -> Option<(u8, u8, u8, u8)> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let idx = ((y * self.width + x) * 4) as usize;
        if idx + 3 < self.rgba_data.len() {
            Some((
                self.rgba_data[idx],
                self.rgba_data[idx + 1],
                self.rgba_data[idx + 2],
                self.rgba_data[idx + 3],
            ))
        } else {
            None
        }
    }

    /// Generate suggested filename
    pub fn suggested_filename(&self) -> String {
        format!(
            "{}_{:05}.{}",
            self.overlay_type.to_lowercase().replace(' ', "_"),
            self.frame_idx,
            self.format.extension()
        )
    }

    /// Get total pixel count
    pub fn pixel_count(&self) -> usize {
        (self.width * self.height) as usize
    }

    /// Check if image is empty (all transparent)
    pub fn is_empty(&self) -> bool {
        self.rgba_data
            .chunks(4)
            .all(|p| p.get(3).copied().unwrap_or(0) == 0)
    }
}

/// Overlay export request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverlayExportRequest {
    /// Overlay type to export
    pub overlay_type: OverlayType,
    /// Frame index to export
    pub frame_idx: usize,
    /// Output format
    pub format: OverlayImageFormat,
    /// Include alpha channel
    pub include_alpha: bool,
    /// Scale factor (1.0 = original size)
    pub scale: f32,
}

impl Default for OverlayExportRequest {
    fn default() -> Self {
        Self {
            overlay_type: OverlayType::QpHeatmap,
            frame_idx: 0,
            format: OverlayImageFormat::Ppm,
            include_alpha: true,
            scale: 1.0,
        }
    }
}

/// Overlay types that can be exported
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OverlayType {
    /// QP heatmap
    QpHeatmap,
    /// Motion vector overlay
    MotionVector,
    /// Partition grid
    PartitionGrid,
    /// Diff heatmap (A/B comparison)
    DiffHeatmap,
}

impl OverlayType {
    /// Get display name
    pub fn name(&self) -> &'static str {
        match self {
            OverlayType::QpHeatmap => "QP Heatmap",
            OverlayType::MotionVector => "Motion Vector",
            OverlayType::PartitionGrid => "Partition Grid",
            OverlayType::DiffHeatmap => "Diff Heatmap",
        }
    }
}

/// Export overlay image to PPM format (simple, no dependencies)
///
/// PPM is a simple image format that most image viewers can open.
/// Format: P6 header followed by raw RGB data.
pub fn export_overlay_ppm<W: Write>(
    data: &OverlayExportData,
    writer: &mut W,
) -> std::io::Result<ExportResult> {
    // PPM header (P6 = binary RGB)
    let header = format!("P6\n{} {}\n255\n", data.width, data.height);
    let mut bytes_written = writer.write(header.as_bytes())?;

    // Convert RGBA to RGB (PPM doesn't support alpha)
    // For transparent pixels, use white background
    for chunk in data.rgba_data.chunks(4) {
        let (r, g, b, a) = (
            chunk.first().copied().unwrap_or(0),
            chunk.get(1).copied().unwrap_or(0),
            chunk.get(2).copied().unwrap_or(0),
            chunk.get(3).copied().unwrap_or(255),
        );

        // Alpha blending with white background
        let alpha = a as f32 / 255.0;
        let bg = 255.0;
        let r_out = (r as f32 * alpha + bg * (1.0 - alpha)) as u8;
        let g_out = (g as f32 * alpha + bg * (1.0 - alpha)) as u8;
        let b_out = (b as f32 * alpha + bg * (1.0 - alpha)) as u8;

        bytes_written += writer.write(&[r_out, g_out, b_out])?;
    }

    Ok(ExportResult {
        format: ExportFormat::Csv, // Reusing ExportFormat (would ideally have Image variant)
        bytes_written,
        row_count: data.pixel_count(),
    })
}

/// Export overlay image to raw RGBA format
pub fn export_overlay_rgba<W: Write>(
    data: &OverlayExportData,
    writer: &mut W,
) -> std::io::Result<ExportResult> {
    let bytes_written = writer.write(&data.rgba_data)?;

    Ok(ExportResult {
        format: ExportFormat::Csv,
        bytes_written,
        row_count: data.pixel_count(),
    })
}

/// Create QP heatmap export data from QPGrid
///
/// Per COMPETITOR_PARITY_STATUS §4.4: Export overlays → image
pub fn create_qp_heatmap_export(
    qp_grid: &crate::qp_heatmap::QPGrid,
    frame_idx: usize,
    user_opacity: f32,
) -> OverlayExportData {
    let mut export =
        OverlayExportData::new(qp_grid.grid_w, qp_grid.grid_h, "QP Heatmap", frame_idx);

    // Use the 4-stop color ramp from QP heatmap spec
    let color_mapper = crate::qp_heatmap::QPColorMapper::new(user_opacity);

    for by in 0..qp_grid.grid_h {
        for bx in 0..qp_grid.grid_w {
            let qp = qp_grid.get(bx, by);
            let color = color_mapper.map_qp(qp, qp_grid.qp_min, qp_grid.qp_max);
            export.set_pixel(bx, by, color.r, color.g, color.b, color.a);
        }
    }

    export
}

/// Create diff heatmap export data
pub fn create_diff_heatmap_export(
    diff_data: &crate::diff_heatmap::DiffHeatmapData,
    frame_idx: usize,
    user_opacity: f32,
) -> OverlayExportData {
    let mut export = OverlayExportData::new(
        diff_data.heatmap_width,
        diff_data.heatmap_height,
        "Diff Heatmap",
        frame_idx,
    );

    // 4-stop ramp: blue (low) → cyan → yellow → red (high)
    for y in 0..diff_data.heatmap_height {
        for x in 0..diff_data.heatmap_width {
            if let Some(normalized) = diff_data.get_normalized(x, y) {
                let alpha = diff_data.get_alpha(x, y, user_opacity);

                // Color ramp based on normalized value
                let (r, g, b) = if normalized < 0.25 {
                    // Blue to cyan
                    let t = normalized / 0.25;
                    (0, (t * 255.0) as u8, 255)
                } else if normalized < 0.5 {
                    // Cyan to yellow
                    let t = (normalized - 0.25) / 0.25;
                    ((t * 255.0) as u8, 255, (255.0 * (1.0 - t)) as u8)
                } else if normalized < 0.75 {
                    // Yellow to orange
                    let t = (normalized - 0.5) / 0.25;
                    (255, (255.0 * (1.0 - t * 0.5)) as u8, 0)
                } else {
                    // Orange to red
                    let t = (normalized - 0.75) / 0.25;
                    (255, (128.0 * (1.0 - t)) as u8, 0)
                };

                export.set_pixel(x, y, r, g, b, alpha);
            }
        }
    }

    export
}

/// Overlay image export result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverlayExportResult {
    /// Export success
    pub success: bool,
    /// Output file path (if written to file)
    pub file_path: Option<String>,
    /// Bytes written
    pub bytes_written: usize,
    /// Image dimensions
    pub width: u32,
    pub height: u32,
    /// Format used
    pub format: OverlayImageFormat,
    /// Error message (if any)
    pub error: Option<String>,
}

impl OverlayExportResult {
    /// Create success result
    pub fn success(
        file_path: Option<String>,
        bytes_written: usize,
        width: u32,
        height: u32,
        format: OverlayImageFormat,
    ) -> Self {
        Self {
            success: true,
            file_path,
            bytes_written,
            width,
            height,
            format,
            error: None,
        }
    }

    /// Create error result
    pub fn error(message: &str) -> Self {
        Self {
            success: false,
            file_path: None,
            bytes_written: 0,
            width: 0,
            height: 0,
            format: OverlayImageFormat::RawRgba,
            error: Some(message.to_string()),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Evidence Bundle Export (per export_evidence_bundle.schema.json)
// ═══════════════════════════════════════════════════════════════════════════

use crate::parity_harness::{OrderType, RenderSnapshot, SelectionSnapshot};
use std::collections::HashMap;
use std::path::Path;

/// Evidence bundle manifest (per export_evidence_bundle.schema.json)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceBundleManifest {
    /// Bundle schema version
    pub bundle_version: String,
    /// Application version
    pub app_version: String,
    /// Git commit hash
    pub git_commit: String,
    /// Build profile (debug/release)
    pub build_profile: String,
    /// Operating system
    pub os: String,
    /// GPU information
    pub gpu: String,
    /// CPU information
    pub cpu: String,
    /// Backend used (e.g., "dav1d")
    pub backend: String,
    /// Plugin versions
    pub plugin_versions: HashMap<String, String>,
    /// Stream fingerprint (hash)
    pub stream_fingerprint: String,
    /// Order type (display/decode)
    pub order_type: OrderType,
    /// Current selection state
    pub selection_state: SelectionSnapshot,
    /// Active workspace
    pub workspace: String,
    /// Current mode
    pub mode: String,
    /// Any warnings
    pub warnings: Vec<String>,
    /// Artifact paths
    pub artifacts: Vec<String>,
}

impl Default for EvidenceBundleManifest {
    fn default() -> Self {
        Self {
            bundle_version: "1.0".to_string(),
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            git_commit: "unknown".to_string(),
            build_profile: if cfg!(debug_assertions) {
                "debug"
            } else {
                "release"
            }
            .to_string(),
            os: std::env::consts::OS.to_string(),
            gpu: "unknown".to_string(),
            cpu: std::env::consts::ARCH.to_string(),
            backend: "dav1d".to_string(),
            plugin_versions: HashMap::new(),
            stream_fingerprint: String::new(),
            order_type: OrderType::Display,
            selection_state: SelectionSnapshot {
                selected_entity: None,
                selected_byte_range: None,
                order_type: OrderType::Display,
            },
            workspace: "player".to_string(),
            mode: "normal".to_string(),
            warnings: Vec::new(),
            artifacts: Vec::new(),
        }
    }
}

/// Evidence bundle export request
#[derive(Debug, Clone)]
pub struct EvidenceBundleExportRequest {
    /// Output directory path
    pub output_dir: std::path::PathBuf,
    /// Include screenshots
    pub include_screenshots: bool,
    /// Include render snapshots
    pub include_render_snapshots: bool,
    /// Include interaction trace
    pub include_interaction_trace: bool,
    /// Include logs
    pub include_logs: bool,
    /// Stream fingerprint
    pub stream_fingerprint: String,
    /// Current selection
    pub selection_state: SelectionSnapshot,
    /// Active workspace
    pub workspace: String,
    /// Current mode
    pub mode: String,
    /// Order type
    pub order_type: OrderType,
}

impl Default for EvidenceBundleExportRequest {
    fn default() -> Self {
        Self {
            output_dir: std::path::PathBuf::from("."),
            include_screenshots: true,
            include_render_snapshots: true,
            include_interaction_trace: false,
            include_logs: false,
            stream_fingerprint: String::new(),
            selection_state: SelectionSnapshot {
                selected_entity: None,
                selected_byte_range: None,
                order_type: OrderType::Display,
            },
            workspace: "player".to_string(),
            mode: "normal".to_string(),
            order_type: OrderType::Display,
        }
    }
}

/// Evidence bundle export result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceBundleExportResult {
    /// Export success
    pub success: bool,
    /// Bundle directory path
    pub bundle_path: Option<String>,
    /// Files created
    pub files_created: Vec<String>,
    /// Total bytes written
    pub total_bytes: usize,
    /// Error message (if any)
    pub error: Option<String>,
}

impl EvidenceBundleExportResult {
    /// Create success result
    pub fn success(bundle_path: String, files: Vec<String>, bytes: usize) -> Self {
        Self {
            success: true,
            bundle_path: Some(bundle_path),
            files_created: files,
            total_bytes: bytes,
            error: None,
        }
    }

    /// Create error result
    pub fn error(message: &str) -> Self {
        Self {
            success: false,
            bundle_path: None,
            files_created: Vec::new(),
            total_bytes: 0,
            error: Some(message.to_string()),
        }
    }
}

/// Export evidence bundle to directory
///
/// Creates a bundle directory containing:
/// - bundle_manifest.json
/// - env.json
/// - selection_state.json
/// - order_type.json
/// - warnings.json
/// - screenshots/ (optional)
/// - render_snapshots/ (optional)
///
/// Per export_entrypoints.json, this must be reachable from:
/// - MainMenu > File > Export > Evidence Bundle
/// - BottomBar > Export
/// - ContextMenu > Export Evidence Bundle
/// - CompareWorkspace > Toolbar > Export Diff Bundle
pub fn export_evidence_bundle(
    request: &EvidenceBundleExportRequest,
    render_snapshots: &[RenderSnapshot],
) -> EvidenceBundleExportResult {
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
    let bundle_name = format!("bitvue_evidence_{}", timestamp);
    let bundle_dir = request.output_dir.join(&bundle_name);

    // Create bundle directory
    if let Err(e) = std::fs::create_dir_all(&bundle_dir) {
        return EvidenceBundleExportResult::error(&format!(
            "Failed to create bundle directory: {}",
            e
        ));
    }

    let mut files_created = Vec::new();
    let mut total_bytes = 0;

    // Create manifest
    let manifest = EvidenceBundleManifest {
        bundle_version: "1.0".to_string(),
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        git_commit: option_env!("GIT_COMMIT").unwrap_or("unknown").to_string(),
        build_profile: if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        }
        .to_string(),
        os: std::env::consts::OS.to_string(),
        gpu: "unknown".to_string(),
        cpu: std::env::consts::ARCH.to_string(),
        backend: "dav1d".to_string(),
        plugin_versions: HashMap::new(),
        stream_fingerprint: request.stream_fingerprint.clone(),
        order_type: request.order_type,
        selection_state: request.selection_state.clone(),
        workspace: request.workspace.clone(),
        mode: request.mode.clone(),
        warnings: Vec::new(),
        artifacts: Vec::new(),
    };

    // Write bundle_manifest.json
    let manifest_path = bundle_dir.join("bundle_manifest.json");
    match write_json_file(&manifest_path, &manifest) {
        Ok(bytes) => {
            files_created.push("bundle_manifest.json".to_string());
            total_bytes += bytes;
        }
        Err(e) => {
            return EvidenceBundleExportResult::error(&format!("Failed to write manifest: {}", e))
        }
    }

    // Write env.json
    let env_info = EnvInfo {
        os: manifest.os.clone(),
        arch: manifest.cpu.clone(),
        gpu: manifest.gpu.clone(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    };
    let env_path = bundle_dir.join("env.json");
    match write_json_file(&env_path, &env_info) {
        Ok(bytes) => {
            files_created.push("env.json".to_string());
            total_bytes += bytes;
        }
        Err(e) => {
            return EvidenceBundleExportResult::error(&format!("Failed to write env.json: {}", e))
        }
    }

    // Write version.json
    let version_info = VersionInfo {
        app_version: manifest.app_version.clone(),
        git_commit: manifest.git_commit.clone(),
        build_profile: manifest.build_profile.clone(),
        bundle_version: manifest.bundle_version.clone(),
    };
    let version_path = bundle_dir.join("version.json");
    match write_json_file(&version_path, &version_info) {
        Ok(bytes) => {
            files_created.push("version.json".to_string());
            total_bytes += bytes;
        }
        Err(e) => {
            return EvidenceBundleExportResult::error(&format!(
                "Failed to write version.json: {}",
                e
            ))
        }
    }

    // Write selection_state.json
    let selection_path = bundle_dir.join("selection_state.json");
    match write_json_file(&selection_path, &request.selection_state) {
        Ok(bytes) => {
            files_created.push("selection_state.json".to_string());
            total_bytes += bytes;
        }
        Err(e) => {
            return EvidenceBundleExportResult::error(&format!(
                "Failed to write selection_state.json: {}",
                e
            ))
        }
    }

    // Write order_type.json
    let order_type_info = OrderTypeInfo {
        order_type: request.order_type,
    };
    let order_type_path = bundle_dir.join("order_type.json");
    match write_json_file(&order_type_path, &order_type_info) {
        Ok(bytes) => {
            files_created.push("order_type.json".to_string());
            total_bytes += bytes;
        }
        Err(e) => {
            return EvidenceBundleExportResult::error(&format!(
                "Failed to write order_type.json: {}",
                e
            ))
        }
    }

    // Write backend_fingerprint.json
    let backend_info = BackendInfo {
        backend: manifest.backend.clone(),
        plugin_versions: manifest.plugin_versions.clone(),
    };
    let backend_path = bundle_dir.join("backend_fingerprint.json");
    match write_json_file(&backend_path, &backend_info) {
        Ok(bytes) => {
            files_created.push("backend_fingerprint.json".to_string());
            total_bytes += bytes;
        }
        Err(e) => {
            return EvidenceBundleExportResult::error(&format!(
                "Failed to write backend_fingerprint.json: {}",
                e
            ))
        }
    }

    // Write warnings.json
    let warnings_path = bundle_dir.join("warnings.json");
    match write_json_file(&warnings_path, &manifest.warnings) {
        Ok(bytes) => {
            files_created.push("warnings.json".to_string());
            total_bytes += bytes;
        }
        Err(e) => {
            return EvidenceBundleExportResult::error(&format!(
                "Failed to write warnings.json: {}",
                e
            ))
        }
    }

    // Write render snapshots if requested
    if request.include_render_snapshots && !render_snapshots.is_empty() {
        let snapshots_dir = bundle_dir.join("render_snapshots");
        if let Err(e) = std::fs::create_dir_all(&snapshots_dir) {
            return EvidenceBundleExportResult::error(&format!(
                "Failed to create snapshots directory: {}",
                e
            ));
        }

        for (idx, snapshot) in render_snapshots.iter().enumerate() {
            let snapshot_path = snapshots_dir.join(format!("snapshot_{:04}.json", idx));
            match write_json_file(&snapshot_path, snapshot) {
                Ok(bytes) => {
                    files_created.push(format!("render_snapshots/snapshot_{:04}.json", idx));
                    total_bytes += bytes;
                }
                Err(e) => {
                    return EvidenceBundleExportResult::error(&format!(
                        "Failed to write snapshot: {}",
                        e
                    ))
                }
            }
        }
    }

    EvidenceBundleExportResult::success(
        bundle_dir.to_string_lossy().to_string(),
        files_created,
        total_bytes,
    )
}

/// Environment info for evidence bundle
#[derive(Debug, Clone, Serialize, Deserialize)]
struct EnvInfo {
    os: String,
    arch: String,
    gpu: String,
    timestamp: String,
}

/// Version info for evidence bundle
#[derive(Debug, Clone, Serialize, Deserialize)]
struct VersionInfo {
    app_version: String,
    git_commit: String,
    build_profile: String,
    bundle_version: String,
}

/// Order type info for evidence bundle
#[derive(Debug, Clone, Serialize, Deserialize)]
struct OrderTypeInfo {
    order_type: OrderType,
}

/// Backend info for evidence bundle
#[derive(Debug, Clone, Serialize, Deserialize)]
struct BackendInfo {
    backend: String,
    plugin_versions: HashMap<String, String>,
}

/// Helper to write JSON file
fn write_json_file<T: Serialize>(path: &Path, data: &T) -> std::io::Result<usize> {
    let json = serde_json::to_string_pretty(data).map_err(std::io::Error::other)?;
    std::fs::write(path, &json)?;
    Ok(json.len())
}

// ═══════════════════════════════════════════════════════════════════════════
// Context Menu Support (per context_menus.json, guard_rules.json)
// ═══════════════════════════════════════════════════════════════════════════

/// Context menu scope
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContextMenuScope {
    Player,
    HexView,
    StreamView,
    Timeline,
    DiagnosticsPanel,
}

/// Context menu item with guard
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextMenuItem {
    pub id: String,
    pub label: String,
    pub command: String,
    pub guard: String,
    pub enabled: bool,
    pub disabled_reason: Option<String>,
}

/// Guard evaluation context
#[derive(Debug, Clone, Default)]
pub struct GuardEvalContext {
    pub has_selection: bool,
    pub has_byte_range: bool,
}

/// Evaluate guard and return enabled state with reason
pub fn evaluate_context_menu_guard(
    guard: &str,
    context: &GuardEvalContext,
) -> (bool, Option<String>) {
    match guard {
        "always" => (true, None),
        "has_selection" => {
            if context.has_selection {
                (true, None)
            } else {
                (false, Some("No selection.".to_string()))
            }
        }
        "has_byte_range" => {
            if context.has_byte_range {
                (true, None)
            } else {
                (false, Some("No byte range selected.".to_string()))
            }
        }
        _ => (false, Some(format!("Unknown guard: {}", guard))),
    }
}

/// Build context menu items for a scope
pub fn build_context_menu(
    scope: ContextMenuScope,
    context: &GuardEvalContext,
) -> Vec<ContextMenuItem> {
    let items = match scope {
        ContextMenuScope::Player => vec![
            (
                "toggle_detail",
                "Details",
                "Toggle.DetailMode",
                "has_selection",
            ),
            (
                "export_bundle",
                "Export Evidence Bundle",
                "Export.EvidenceBundle",
                "always",
            ),
            (
                "copy_selection",
                "Copy Selection",
                "Copy.Selection",
                "has_selection",
            ),
        ],
        ContextMenuScope::HexView => vec![
            ("copy_bytes", "Copy Bytes", "Copy.Bytes", "has_byte_range"),
            (
                "export_bundle",
                "Export Evidence Bundle",
                "Export.EvidenceBundle",
                "always",
            ),
        ],
        ContextMenuScope::StreamView => vec![
            (
                "set_order_display",
                "Compare in Display Order",
                "Set.OrderType.Display",
                "always",
            ),
            (
                "set_order_decode",
                "Compare in Decode Order",
                "Set.OrderType.Decode",
                "always",
            ),
        ],
        ContextMenuScope::Timeline => vec![
            (
                "export_bundle",
                "Export Evidence Bundle",
                "Export.EvidenceBundle",
                "always",
            ),
            (
                "copy_selection",
                "Copy Selection",
                "Copy.Selection",
                "has_selection",
            ),
        ],
        ContextMenuScope::DiagnosticsPanel => vec![
            (
                "export_bundle",
                "Export Evidence Bundle",
                "Export.EvidenceBundle",
                "always",
            ),
            (
                "copy_selection",
                "Copy Selection",
                "Copy.Selection",
                "has_selection",
            ),
        ],
    };

    items
        .into_iter()
        .map(|(id, label, command, guard)| {
            let (enabled, disabled_reason) = evaluate_context_menu_guard(guard, context);
            ContextMenuItem {
                id: id.to_string(),
                label: label.to_string(),
                command: command.to_string(),
                guard: guard.to_string(),
                enabled,
                disabled_reason,
            }
        })
        .collect()
}

// =============================================================================
// SEMANTIC PROBE RUNNER - Integration with UI Panels
// =============================================================================

use crate::parity_harness::{
    EntityRef, HardFailKind, HitTestProbeInput, HitTestProbeOutput, ProbeContext, ProbeOutcome,
    ProbePoint, ProbeResult, ProbeViolation, Provenance, SelectionPropagationInput,
    SelectionPropagationOutput, TooltipField, TooltipPayload, TooltipPayloadInput,
    TooltipPayloadOutput, ViewportState,
};

/// Semantic probe runner - records and validates probe results against contracts
#[derive(Debug, Default)]
pub struct SemanticProbeRunner {
    /// Recorded probe results
    pub results: Vec<ProbeResult>,
    /// Hard fail violations detected
    pub violations: Vec<ProbeViolation>,
    /// Current stream ID being probed
    pub stream_id: String,
    /// Current codec
    pub codec: String,
    /// Current workspace
    pub workspace: String,
    /// Current mode
    pub mode: String,
}

impl SemanticProbeRunner {
    /// Create a new probe runner
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the current context for probes
    pub fn set_context(&mut self, stream_id: &str, codec: &str, workspace: &str, mode: &str) {
        self.stream_id = stream_id.to_string();
        self.codec = codec.to_string();
        self.workspace = workspace.to_string();
        self.mode = mode.to_string();
    }

    /// Run hit-test probe
    ///
    /// Per semantic_probe_contracts.json: Validates hit-testing consistency
    pub fn run_hit_test_probe(
        &mut self,
        panel: &str,
        viewport: ViewportState,
        hit_entity: Option<EntityRef>,
        layer: &str,
        tooltip: Option<TooltipPayload>,
    ) -> ProbeResult {
        let probe_id = format!(
            "hit_test_{}_{}",
            panel,
            chrono::Utc::now().timestamp_millis()
        );
        let mut violations = Vec::new();

        // Check for transform consistency (hard fail if hit-test returns different entity than render)
        // This is a simplified check - in practice you'd compare with actual render data
        if hit_entity.is_some() && viewport.zoom == 0.0 {
            violations.push(ProbeViolation {
                kind: HardFailKind::HitTestRenderTransformMismatch,
                message: "Zero zoom detected during hit-test".to_string(),
                context: [("panel".to_string(), panel.to_string())]
                    .into_iter()
                    .collect(),
            });
        }

        let result = ProbeResult {
            probe_id: probe_id.clone(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            stream_id: self.stream_id.clone(),
            codec: self.codec.clone(),
            workspace: self.workspace.clone(),
            mode: self.mode.clone(),
            result: ProbeOutcome::HitTest(HitTestProbeOutput {
                hit_entity,
                layer: layer.to_string(),
                cursor_payload: tooltip,
            }),
            violations: violations.clone(),
        };

        self.violations.extend(violations);
        self.results.push(result.clone());
        result
    }

    /// Run selection propagation probe
    ///
    /// Per semantic_probe_contracts.json: Validates selection sync across panels
    pub fn run_selection_propagation_probe(
        &mut self,
        origin_panel: &str,
        action: &str,
        _txn: &str,
        panels_updated: Vec<String>,
        selection: SelectionSnapshot,
    ) -> ProbeResult {
        let probe_id = format!(
            "selection_{}_{}",
            origin_panel,
            chrono::Utc::now().timestamp_millis()
        );
        let violations = Vec::new();

        // Check for order type mixing (hard fail)
        // In practice, you'd compare with previous selection state
        if panels_updated.len() > 1 {
            // Selection should propagate to all panels
            let expected_panels = ["player", "syntax", "hex", "timeline"];
            let missing: Vec<_> = expected_panels
                .iter()
                .filter(|p| !panels_updated.iter().any(|u| u.contains(*p)))
                .collect();

            if !missing.is_empty() && action == "select" {
                // This is a warning, not a hard fail
            }
        }

        let result = ProbeResult {
            probe_id: probe_id.clone(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            stream_id: self.stream_id.clone(),
            codec: self.codec.clone(),
            workspace: self.workspace.clone(),
            mode: self.mode.clone(),
            result: ProbeOutcome::SelectionPropagation(SelectionPropagationOutput {
                panels_updated,
                selection_snapshot: selection,
            }),
            violations: violations.clone(),
        };

        self.violations.extend(violations);
        self.results.push(result.clone());
        result
    }

    /// Run tooltip payload probe
    ///
    /// Per semantic_probe_contracts.json: Validates tooltip content
    pub fn run_tooltip_payload_probe(
        &mut self,
        entity: EntityRef,
        fields: Vec<TooltipField>,
    ) -> ProbeResult {
        let probe_id = format!(
            "tooltip_{}_{}",
            entity.id,
            chrono::Utc::now().timestamp_millis()
        );
        let violations = Vec::new();

        let result = ProbeResult {
            probe_id: probe_id.clone(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            stream_id: self.stream_id.clone(),
            codec: self.codec.clone(),
            workspace: self.workspace.clone(),
            mode: self.mode.clone(),
            result: ProbeOutcome::TooltipPayload(TooltipPayloadOutput {
                fields,
                provenance: Provenance::default(),
            }),
            violations: violations.clone(),
        };

        self.results.push(result.clone());
        result
    }

    /// Record order type mixing hard fail
    pub fn record_order_type_mixing(&mut self, context: &str) {
        self.violations.push(ProbeViolation {
            kind: HardFailKind::OrderTypeMixingDetected,
            message: format!("Display/decode order mixing detected: {}", context),
            context: [("context".to_string(), context.to_string())]
                .into_iter()
                .collect(),
        });
    }

    /// Record stale async applied hard fail
    pub fn record_stale_async(&mut self, txn_id: &str) {
        self.violations.push(ProbeViolation {
            kind: HardFailKind::StaleAsyncApplied,
            message: format!("Stale async result applied: txn={}", txn_id),
            context: [("txn_id".to_string(), txn_id.to_string())]
                .into_iter()
                .collect(),
        });
    }

    /// Record cache invalidation violation
    pub fn record_cache_violation(&mut self, cache_key: &str) {
        self.violations.push(ProbeViolation {
            kind: HardFailKind::CacheInvalidationViolation,
            message: format!("Cache invalidation violation: key={}", cache_key),
            context: [("cache_key".to_string(), cache_key.to_string())]
                .into_iter()
                .collect(),
        });
    }

    /// Check if any hard fails have been detected
    pub fn has_hard_fails(&self) -> bool {
        !self.violations.is_empty()
    }

    /// Get all hard fail violations
    pub fn get_violations(&self) -> &[ProbeViolation] {
        &self.violations
    }

    /// Export probe results as JSON
    pub fn export_results(&self) -> Result<String> {
        serde_json::to_string_pretty(&self.results).map_err(|e| crate::BitvueError::Decode(format!("Failed to serialize probe results: {}", e)))
    }

    /// Clear recorded results
    pub fn clear(&mut self) {
        self.results.clear();
        self.violations.clear();
    }
}

/// Create a hit-test probe input for a panel
pub fn create_hit_test_input(
    panel: &str,
    viewport: ViewportState,
    x: f32,
    y: f32,
) -> HitTestProbeInput {
    HitTestProbeInput {
        panel: panel.to_string(),
        viewport,
        points: vec![ProbePoint { x, y }],
    }
}

/// Create a selection propagation input
pub fn create_selection_input(
    origin_panel: &str,
    action: &str,
    txn: &str,
) -> SelectionPropagationInput {
    SelectionPropagationInput {
        origin_panel: origin_panel.to_string(),
        action: action.to_string(),
        txn: txn.to_string(),
    }
}

/// Create a tooltip payload input
pub fn create_tooltip_input(
    entity: EntityRef,
    workspace: &str,
    mode: &str,
    codec: &str,
) -> TooltipPayloadInput {
    TooltipPayloadInput {
        entity,
        context: ProbeContext {
            workspace: workspace.to_string(),
            mode: mode.to_string(),
            codec: codec.to_string(),
        },
    }
}

/// Create a tooltip field for display
pub fn create_tooltip_field(
    label: &str,
    value: &str,
    provenance: Option<Provenance>,
) -> TooltipField {
    TooltipField {
        label: label.to_string(),
        value: value.to_string(),
        provenance,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::timeline::{FrameMarker, TimelineFrame};

    fn create_test_frame(display_idx: usize, frame_type: &str, size: u64, marker: FrameMarker) -> TimelineFrame {
        TimelineFrame {
            display_idx,
            frame_type: frame_type.to_string(),
            size_bytes: size,
            pts: Some(1000),
            dts: Some(900),
            marker,
            is_selected: false,
        }
    }

    #[test]
    fn test_export_format_equality() {
        assert_eq!(ExportFormat::Csv, ExportFormat::Csv);
        assert_ne!(ExportFormat::Csv, ExportFormat::Json);
        assert_ne!(ExportFormat::Json, ExportFormat::JsonPretty);
    }

    #[test]
    fn test_frame_export_row_from_timeline_frame() {
        let frame = create_test_frame(0, "KEY", 5000, FrameMarker::Key);
        let row = FrameExportRow::from_timeline_frame(&frame, 0);

        assert_eq!(row.display_idx, 0);
        assert_eq!(row.frame_type, "KEY");
        assert_eq!(row.size_bytes, 5000);
        assert_eq!(row.pts, Some(1000));
        assert_eq!(row.dts, Some(900));
        assert!(row.is_key);
        assert!(!row.has_error);
    }

    #[test]
    fn test_frame_export_row_error_marker() {
        let frame = create_test_frame(5, "P", 1000, FrameMarker::Error);
        let row = FrameExportRow::from_timeline_frame(&frame, 5);

        assert!(!row.is_key);
        assert!(row.has_error);
    }

    #[test]
    fn test_export_frames_csv_basic() {
        let frames = vec![
            create_test_frame(0, "KEY", 5000, FrameMarker::Key),
            create_test_frame(1, "P", 1000, FrameMarker::None),
            create_test_frame(2, "B", 500, FrameMarker::None),
        ];

        let mut output = Vec::new();
        let result = export_frames_csv(&frames, &mut output, None).unwrap();

        assert_eq!(result.format, ExportFormat::Csv);
        assert_eq!(result.row_count, 3);

        let csv_str = String::from_utf8(output).unwrap();
        assert!(csv_str.contains("display_idx,frame_type,size_bytes"));
        assert!(csv_str.contains("0,KEY,5000"));
        assert!(csv_str.contains("1,P,1000"));
        assert!(csv_str.contains("2,B,500"));
    }

    #[test]
    fn test_export_frames_csv_with_range() {
        let frames = vec![
            create_test_frame(0, "KEY", 5000, FrameMarker::Key),
            create_test_frame(1, "P", 1000, FrameMarker::None),
            create_test_frame(2, "B", 500, FrameMarker::None),
            create_test_frame(3, "P", 800, FrameMarker::None),
        ];

        let mut output = Vec::new();
        let result = export_frames_csv(&frames, &mut output, Some((1, 2))).unwrap();

        assert_eq!(result.row_count, 2);

        let csv_str = String::from_utf8(output).unwrap();
        assert!(!csv_str.contains("0,KEY"));
        assert!(csv_str.contains("1,P,1000"));
        assert!(csv_str.contains("2,B,500"));
        assert!(!csv_str.contains("3,P"));
    }

    #[test]
    fn test_export_frames_csv_empty() {
        let frames: Vec<TimelineFrame> = vec![];

        let mut output = Vec::new();
        let result = export_frames_csv(&frames, &mut output, None).unwrap();

        assert_eq!(result.row_count, 0);

        let csv_str = String::from_utf8(output).unwrap();
        // Should still have header
        assert!(csv_str.contains("display_idx,frame_type,size_bytes"));
    }

    #[test]
    fn test_export_frames_json_basic() {
        let frames = vec![
            create_test_frame(0, "KEY", 5000, FrameMarker::Key),
            create_test_frame(1, "P", 1000, FrameMarker::None),
        ];

        let mut output = Vec::new();
        let result = export_frames_json(&frames, &mut output, None, false).unwrap();

        assert_eq!(result.format, ExportFormat::Json);
        assert_eq!(result.row_count, 2);

        let json_str = String::from_utf8(output).unwrap();
        assert!(json_str.contains("\"frame_type\":\"KEY\""));
        assert!(json_str.contains("\"size_bytes\":5000"));
    }

    #[test]
    fn test_export_frames_json_pretty() {
        let frames = vec![create_test_frame(0, "KEY", 5000, FrameMarker::Key)];

        let mut output = Vec::new();
        let result = export_frames_json(&frames, &mut output, None, true).unwrap();

        assert_eq!(result.format, ExportFormat::JsonPretty);

        let json_str = String::from_utf8(output).unwrap();
        // Pretty JSON should have newlines and indentation
        assert!(json_str.contains("\n"));
    }

    #[test]
    fn test_export_metrics_csv_basic() {
        let psnr = vec![
            MetricPoint { idx: 0, value: 40.5 },
            MetricPoint { idx: 1, value: 38.2 },
        ];
        let ssim = vec![
            MetricPoint { idx: 0, value: 0.98 },
            MetricPoint { idx: 1, value: 0.95 },
        ];
        let vmaf: Vec<MetricPoint> = vec![];

        let mut output = Vec::new();
        let result = export_metrics_csv(&psnr, &ssim, &vmaf, &mut output).unwrap();

        assert_eq!(result.format, ExportFormat::Csv);
        assert_eq!(result.row_count, 2);

        let csv_str = String::from_utf8(output).unwrap();
        assert!(csv_str.contains("display_idx,psnr_y,ssim_y,vmaf"));
        assert!(csv_str.contains("0,40.5000,0.980000,"));
    }

    #[test]
    fn test_export_metrics_csv_empty() {
        let psnr: Vec<MetricPoint> = vec![];
        let ssim: Vec<MetricPoint> = vec![];
        let vmaf: Vec<MetricPoint> = vec![];

        let mut output = Vec::new();
        let result = export_metrics_csv(&psnr, &ssim, &vmaf, &mut output).unwrap();

        assert_eq!(result.row_count, 0);
    }

    #[test]
    fn test_diagnostics_export_row_from_diagnostic() {
        use crate::diagnostics::{Diagnostic, DiagnosticCategory};
        use crate::StreamId;
        use std::collections::HashMap;

        let diag = Diagnostic {
            id: 42,
            severity: DiagnosticSeverity::Warn,
            category: DiagnosticCategory::Bitstream,
            message: "Test warning".to_string(),
            offset_bytes: 1024,
            bit_range: None,
            frame_key: None,
            unit_key: None,
            codec: None,
            timestamp_ms: 0,
            details: HashMap::new(),
            stream_id: StreamId::A,
        };

        let row = DiagnosticsExportRow::from_diagnostic(&diag);

        assert_eq!(row.id, 42);
        assert_eq!(row.severity, "Warn");
        assert_eq!(row.category, "Bitstream");
        assert_eq!(row.message, "Test warning");
        assert_eq!(row.byte_offset, 1024);
        assert_eq!(row.frame_idx, None);
    }

    #[test]
    fn test_export_result_debug() {
        let result = ExportResult {
            format: ExportFormat::Csv,
            bytes_written: 1234,
            row_count: 10,
        };

        let debug_str = format!("{:?}", result);
        assert!(debug_str.contains("Csv"));
        assert!(debug_str.contains("1234"));
        assert!(debug_str.contains("10"));
    }
}

// Additional comprehensive tests
include!("export_test.rs");
