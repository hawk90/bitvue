//! Export Module - Feature Parity Phase A
//!
//! Provides CSV/JSON export for timeline data, metrics, and diagnostics.
//! Per COMPETITOR_PARITY_MATRIX: Export is a core requirement for professional use.

mod context_menu;
mod diagnostics;
mod evidence;
mod frames;
mod metrics;
mod overlay;
mod probes;
mod summary;
mod types;

pub use context_menu::*;
pub use diagnostics::*;
pub use evidence::*;
pub use frames::*;
pub use metrics::*;
pub use overlay::*;
pub use probes::*;
pub use summary::*;
pub use types::*;

// ═══════════════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════════════

#[allow(
    unused_imports,
    unused_variables,
    unused_mut,
    dead_code,
    unused_comparisons,
    unused_must_use,
    hidden_glob_reexports,
    unreachable_code,
    non_camel_case_types,
    unused_parens,
    unused_assignments
)]
#[cfg(test)]
mod tests {
    use super::types::ExportFormat;
    use super::*;
    use crate::timeline::{FrameMarker, TimelineFrame};

    fn create_test_frame(
        display_idx: usize,
        frame_type: &str,
        size: u64,
        marker: FrameMarker,
    ) -> TimelineFrame {
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
        let result = export_frames_csv(&frames, &mut output, ExportConfig::default()).unwrap();

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

        let config = ExportConfig {
            range: Some((1, 2)),
            ..Default::default()
        };
        let mut output = Vec::new();
        let result = export_frames_csv(&frames, &mut output, config).unwrap();

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
        let result = export_frames_csv(&frames, &mut output, ExportConfig::default()).unwrap();

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
        let result = export_frames_json(&frames, &mut output, ExportConfig::default()).unwrap();

        assert_eq!(result.format, ExportFormat::Json);
        assert_eq!(result.row_count, 2);

        let json_str = String::from_utf8(output).unwrap();
        assert!(json_str.contains("\"frame_type\":\"KEY\""));
        assert!(json_str.contains("\"size_bytes\":5000"));
    }

    #[test]
    fn test_export_frames_json_pretty() {
        let frames = vec![create_test_frame(0, "KEY", 5000, FrameMarker::Key)];

        let config = ExportConfig {
            range: None,
            pretty: true,
        };
        let mut output = Vec::new();
        let result = export_frames_json(&frames, &mut output, config).unwrap();

        assert_eq!(result.format, ExportFormat::JsonPretty);

        let json_str = String::from_utf8(output).unwrap();
        // Pretty JSON should have newlines and indentation
        assert!(json_str.contains("\n"));
    }

    #[test]
    fn test_export_metrics_csv_basic() {
        let psnr = vec![
            crate::metrics_distribution::MetricPoint {
                idx: 0,
                value: 40.5,
            },
            crate::metrics_distribution::MetricPoint {
                idx: 1,
                value: 38.2,
            },
        ];
        let ssim = vec![
            crate::metrics_distribution::MetricPoint {
                idx: 0,
                value: 0.98,
            },
            crate::metrics_distribution::MetricPoint {
                idx: 1,
                value: 0.95,
            },
        ];
        let vmaf: Vec<crate::metrics_distribution::MetricPoint> = vec![];

        let metrics = QualityMetrics {
            psnr_y: &psnr,
            ssim_y: &ssim,
            vmaf: &vmaf,
        };
        let mut output = Vec::new();
        let result = export_metrics_csv(metrics, &mut output).unwrap();

        assert_eq!(result.format, ExportFormat::Csv);
        assert_eq!(result.row_count, 2);

        let csv_str = String::from_utf8(output).unwrap();
        assert!(csv_str.contains("display_idx,psnr_y,ssim_y,vmaf"));
        assert!(csv_str.contains("0,40.5000,0.980000,"));
    }

    #[test]
    fn test_export_metrics_csv_empty() {
        let psnr: Vec<crate::metrics_distribution::MetricPoint> = vec![];
        let ssim: Vec<crate::metrics_distribution::MetricPoint> = vec![];
        let vmaf: Vec<crate::metrics_distribution::MetricPoint> = vec![];

        let metrics = QualityMetrics {
            psnr_y: &psnr,
            ssim_y: &ssim,
            vmaf: &vmaf,
        };
        let mut output = Vec::new();
        let result = export_metrics_csv(metrics, &mut output).unwrap();

        assert_eq!(result.row_count, 0);
    }

    #[test]
    fn test_diagnostics_export_row_from_diagnostic() {
        use crate::diagnostics::{Diagnostic, DiagnosticCategory, DiagnosticSeverity};
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
