//! Overlay image export (PPM, PNG, Raw RGBA)

use serde::{Deserialize, Serialize};
use std::io::Write;

use super::types::ExportResult;

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
        format: super::types::ExportFormat::Csv, // Reusing ExportFormat (would ideally have Image variant)
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
        format: super::types::ExportFormat::Csv,
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
