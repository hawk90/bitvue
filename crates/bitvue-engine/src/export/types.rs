//! Common export types and configurations

use crate::metrics_distribution::MetricPoint;
use serde::{Deserialize, Serialize};

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

/// Export configuration options
///
/// Groups related export options to reduce function parameter count.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ExportConfig {
    /// Optional frame range to export (start, end indices)
    pub range: Option<(u64, u64)>,
    /// Whether to format JSON output with pretty printing
    pub pretty: bool,
}

/// Quality metrics data
///
/// Groups related quality metric arrays for export.
pub struct QualityMetrics<'a> {
    /// PSNR values for Y component
    pub psnr_y: &'a [MetricPoint],
    /// SSIM values for Y component
    pub ssim_y: &'a [MetricPoint],
    /// VMAF values
    pub vmaf: &'a [MetricPoint],
}

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
