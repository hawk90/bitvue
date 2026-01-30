//! Metrics export (CSV)

use serde::{Deserialize, Serialize};
use std::io::Write;

use super::types::{ExportFormat, ExportResult, QualityMetrics};

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
    metrics: QualityMetrics,
    writer: &mut W,
) -> std::io::Result<ExportResult> {
    writeln!(writer, "display_idx,psnr_y,ssim_y,vmaf")?;

    // Build frame map
    let max_idx = [
        metrics.psnr_y.iter().map(|p| p.idx).max().unwrap_or(0),
        metrics.ssim_y.iter().map(|p| p.idx).max().unwrap_or(0),
        metrics.vmaf.iter().map(|p| p.idx).max().unwrap_or(0),
    ]
    .into_iter()
    .max()
    .unwrap_or(0);

    let mut row_count = 0;
    let mut bytes_written = 0;

    for idx in 0..=max_idx {
        let psnr_val = metrics.psnr_y.iter().find(|p| p.idx == idx).map(|p| p.value);
        let ssim_val = metrics.ssim_y.iter().find(|p| p.idx == idx).map(|p| p.value);
        let vmaf_val = metrics.vmaf.iter().find(|p| p.idx == idx).map(|p| p.value);

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
