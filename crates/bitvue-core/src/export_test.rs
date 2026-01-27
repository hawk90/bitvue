// Export module tests
#[cfg(test)]
use super::*;

// ============================================================================
// Fixtures
// ============================================================================
fn create_test_frames() -> Vec<crate::timeline::TimelineFrame> {
    vec![
        crate::timeline::TimelineFrame {
            display_idx: 0,
            frame_type: "KEY".to_string(),
            size_bytes: 5000,
            pts: Some(1000),
            dts: Some(900),
            marker: crate::timeline::FrameMarker::Key,
            is_selected: false,
        },
        crate::timeline::TimelineFrame {
            display_idx: 1,
            frame_type: "P".to_string(),
            size_bytes: 1000,
            pts: Some(1041),
            dts: Some(941),
            marker: crate::timeline::FrameMarker::None,
            is_selected: false,
        },
        crate::timeline::TimelineFrame {
            display_idx: 2,
            frame_type: "B".to_string(),
            size_bytes: 500,
            pts: Some(1020),
            dts: Some(920),
            marker: crate::timeline::FrameMarker::None,
            is_selected: false,
        },
    ]
}

fn create_test_metrics() -> (Vec<crate::metrics_distribution::MetricPoint>, Vec<crate::metrics_distribution::MetricPoint>, Vec<crate::metrics_distribution::MetricPoint>) {
    let psnr = vec![
        crate::metrics_distribution::MetricPoint { idx: 0, value: 40.5 },
        crate::metrics_distribution::MetricPoint { idx: 1, value: 38.2 },
        crate::metrics_distribution::MetricPoint { idx: 2, value: 36.8 },
    ];
    let ssim = vec![
        crate::metrics_distribution::MetricPoint { idx: 0, value: 0.98 },
        crate::metrics_distribution::MetricPoint { idx: 1, value: 0.95 },
    ];
    let vmaf = vec![crate::metrics_distribution::MetricPoint { idx: 0, value: 92.0 }];
    (psnr, ssim, vmaf)
}

fn create_test_diagnostic(id: u64, severity: DiagnosticSeverity) -> Diagnostic {
    Diagnostic {
        id,
        severity,
        category: crate::diagnostics::DiagnosticCategory::Bitstream,
        message: "Test diagnostic message".to_string(),
        offset_bytes: 1024,
        bit_range: None,
        frame_key: None,
        unit_key: None,
        codec: None,
        timestamp_ms: 0,
        details: std::collections::HashMap::new(),
        stream_id: crate::StreamId::A,
    }
}

fn create_test_diagnostics() -> Vec<Diagnostic> {
    vec![
        create_test_diagnostic(1, DiagnosticSeverity::Error),
        create_test_diagnostic(2, DiagnosticSeverity::Warn),
        create_test_diagnostic(3, DiagnosticSeverity::Info),
    ]
}

fn create_test_export_request() -> EvidenceBundleExportRequest {
    EvidenceBundleExportRequest {
        output_dir: std::path::PathBuf::from("/tmp/test"),
        include_screenshots: true,
        include_render_snapshots: true,
        include_interaction_trace: false,
        include_logs: false,
        stream_fingerprint: "test_fingerprint".to_string(),
        selection_state: crate::parity_harness::SelectionSnapshot {
            selected_entity: None,
            selected_byte_range: None,
            order_type: crate::parity_harness::OrderType::Display,
        },
        workspace: "player".to_string(),
        mode: "normal".to_string(),
        order_type: crate::parity_harness::OrderType::Display,
    }
}

fn create_test_overlay_data() -> OverlayExportData {
    let mut data = OverlayExportData::new(100, 100, "Test Overlay", 0);
    data.set_pixel(0, 0, 255, 0, 0, 255);
    data.set_pixel(50, 50, 0, 255, 0, 128);
    data
}

fn create_test_context() -> GuardEvalContext {
    GuardEvalContext {
        has_selection: true,
        has_byte_range: true,
    }
}

// ============================================================================
// ExportFormat Tests
// ============================================================================
#[cfg(test)]
mod format_tests {
    use super::*;

    #[test]
    fn test_format_equality() {
        assert_eq!(ExportFormat::Csv, ExportFormat::Csv);
        assert_ne!(ExportFormat::Csv, ExportFormat::Json);
        assert_ne!(ExportFormat::Json, ExportFormat::JsonPretty);
    }
}

// ============================================================================
// FrameExportRow Tests
// ============================================================================
#[cfg(test)]
mod frame_row_tests {
    use super::*;

    #[test]
    fn test_frame_export_row_from_timeline_frame() {
        let frames = create_test_frames();
        let row = FrameExportRow::from_timeline_frame(&frames[0], 0);

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
        let frame = TimelineFrame {
            display_idx: 5,
            frame_type: "P".to_string(),
            size_bytes: 1000,
            pts: Some(2000),
            dts: Some(1900),
            marker: FrameMarker::Error,
            is_selected: false,
        };
        let row = FrameExportRow::from_timeline_frame(&frame, 5);

        assert!(!row.is_key);
        assert!(row.has_error);
    }

    #[test]
    fn test_frame_export_row_none_marker() {
        let frames = create_test_frames();
        let row = FrameExportRow::from_timeline_frame(&frames[1], 1);

        assert!(!row.is_key);
        assert!(!row.has_error);
    }
}

// ============================================================================
// Export CSV Tests
// ============================================================================
#[cfg(test)]
mod csv_export_tests {
    use super::*;

    #[test]
    fn test_export_frames_csv_basic() {
        let frames = create_test_frames();
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
        let frames = create_test_frames();
        let mut output = Vec::new();
        let result = export_frames_csv(&frames, &mut output, Some((1, 2))).unwrap();

        assert_eq!(result.row_count, 2);

        let csv_str = String::from_utf8(output).unwrap();
        assert!(!csv_str.contains("0,KEY"));
        assert!(csv_str.contains("1,P,1000"));
        assert!(csv_str.contains("2,B,500"));
    }

    #[test]
    fn test_export_frames_csv_empty() {
        let frames: Vec<crate::timeline::TimelineFrame> = vec![];
        let mut output = Vec::new();
        let result = export_frames_csv(&frames, &mut output, None).unwrap();

        assert_eq!(result.row_count, 0);

        let csv_str = String::from_utf8(output).unwrap();
        assert!(csv_str.contains("display_idx,frame_type,size_bytes"));
    }

    #[test]
    fn test_export_frames_csv_bytes_written() {
        let frames = create_test_frames();
        let mut output = Vec::new();
        let result = export_frames_csv(&frames, &mut output, None).unwrap();

        assert!(result.bytes_written > 0);
    }

    #[test]
    fn test_export_metrics_csv_basic() {
        let (psnr, ssim, vmaf) = create_test_metrics();
        let mut output = Vec::new();
        let result = export_metrics_csv(&psnr, &ssim, &vmaf, &mut output).unwrap();

        assert_eq!(result.format, ExportFormat::Csv);
        assert_eq!(result.row_count, 3);

        let csv_str = String::from_utf8(output).unwrap();
        assert!(csv_str.contains("display_idx,psnr_y,ssim_y,vmaf"));
        assert!(csv_str.contains("0,40.5000,0.980000,92.00"));
    }

    #[test]
    fn test_export_metrics_csv_empty() {
        let psnr: Vec<crate::metrics_distribution::MetricPoint> = vec![];
        let ssim: Vec<crate::metrics_distribution::MetricPoint> = vec![];
        let vmaf: Vec<crate::metrics_distribution::MetricPoint> = vec![];

        let mut output = Vec::new();
        let result = export_metrics_csv(&psnr, &ssim, &vmaf, &mut output).unwrap();

        assert_eq!(result.row_count, 0);
    }

    #[test]
    fn test_export_diagnostics_csv_basic() {
        let diagnostics = create_test_diagnostics();
        let mut output = Vec::new();
        let result = export_diagnostics_csv(&diagnostics, &mut output, None).unwrap();

        assert_eq!(result.format, ExportFormat::Csv);
        assert_eq!(result.row_count, 3);

        let csv_str = String::from_utf8(output).unwrap();
        assert!(csv_str.contains("id,severity,category,message"));
        assert!(csv_str.contains("1,Error,Bitstream"));
    }

    #[test]
    fn test_export_diagnostics_csv_with_severity_filter() {
        let diagnostics = create_test_diagnostics();
        let mut output = Vec::new();
        let result = export_diagnostics_csv(&diagnostics, &mut output, Some(DiagnosticSeverity::Warn)).unwrap();

        // Only Warn and Error should be included (Info is below Warn)
        assert_eq!(result.row_count, 2);
    }

    #[test]
    fn test_export_diagnostics_csv_with_error_filter() {
        let diagnostics = create_test_diagnostics();
        let mut output = Vec::new();
        let result = export_diagnostics_csv(&diagnostics, &mut output, Some(DiagnosticSeverity::Error)).unwrap();

        // Only Error should be included
        assert_eq!(result.row_count, 1);
    }

    #[test]
    fn test_export_diagnostics_csv_message_escaping() {
        let mut diag = create_test_diagnostic(1, DiagnosticSeverity::Warn);
        diag.message = "Test, with \"quotes\"".to_string();

        let mut output = Vec::new();
        let result = export_diagnostics_csv(&[diag], &mut output, None).unwrap();

        let csv_str = String::from_utf8(output).unwrap();
        // Message should be quoted and escaped
        assert!(csv_str.contains("\"Test, with \"\"quotes\"\"\""));
    }
}

// ============================================================================
// Export JSON Tests
// ============================================================================
#[cfg(test)]
mod json_export_tests {
    use super::*;

    #[test]
    fn test_export_frames_json_basic() {
        let frames = create_test_frames();
        let mut output = Vec::new();
        let result = export_frames_json(&frames, &mut output, None, false).unwrap();

        assert_eq!(result.format, ExportFormat::Json);
        assert_eq!(result.row_count, 3);  // create_test_frames creates 3 frames

        let json_str = String::from_utf8(output).unwrap();
        assert!(json_str.contains("\"frame_type\":\"KEY\""));
        assert!(json_str.contains("\"size_bytes\":5000"));
    }

    #[test]
    fn test_export_frames_json_pretty() {
        let frames = create_test_frames();
        let mut output = Vec::new();
        let result = export_frames_json(&frames, &mut output, None, true).unwrap();

        assert_eq!(result.format, ExportFormat::JsonPretty);

        let json_str = String::from_utf8(output).unwrap();
        assert!(json_str.contains("\n"));
        assert!(json_str.contains("  "));
    }

    #[test]
    fn test_export_diagnostics_json_basic() {
        let diagnostics = create_test_diagnostics();
        let mut output = Vec::new();
        let result = export_diagnostics_json(&diagnostics, &mut output, None, false).unwrap();

        assert_eq!(result.format, ExportFormat::Json);
        assert_eq!(result.row_count, 3);

        let json_str = String::from_utf8(output).unwrap();
        assert!(json_str.contains("\"severity\":\"Error\""));
    }
}

// ============================================================================
// ExportSummary Tests
// ============================================================================
#[cfg(test)]
mod summary_tests {
    use super::*;

    #[test]
    fn test_export_summary_from_frames() {
        let frames = create_test_frames();
        let summary = ExportSummary::from_frames(&frames);

        assert_eq!(summary.total_frames, 3);
        assert_eq!(summary.total_bytes, 6500);
        assert_eq!(summary.key_frame_count, 1);
        assert_eq!(summary.error_count, 0);
        // avg = 6500 / 3 = 2166.67
        assert!((summary.avg_frame_size - 2166.67).abs() < 0.1);
        assert_eq!(summary.min_frame_size, 500);
        assert_eq!(summary.max_frame_size, 5000);
        assert_eq!(summary.duration_pts, Some(20));
    }

    #[test]
    fn test_export_summary_empty_frames() {
        let frames: Vec<TimelineFrame> = vec![];
        let summary = ExportSummary::from_frames(&frames);

        assert_eq!(summary.total_frames, 0);
        assert_eq!(summary.total_bytes, 0);
        assert_eq!(summary.avg_frame_size, 0.0);
    }

    #[test]
    fn test_export_summary_error_frames() {
        let mut frames = create_test_frames();
        frames.push(crate::timeline::TimelineFrame {
            display_idx: 3,
            frame_type: "ERROR".to_string(),
            size_bytes: 0,
            pts: None,
            dts: None,
            marker: crate::timeline::FrameMarker::Error,
            is_selected: false,
        });

        let summary = ExportSummary::from_frames(&frames);
        assert_eq!(summary.error_count, 1);
    }
}

// ============================================================================
// OverlayExportData Tests
// ============================================================================
#[cfg(test)]
mod overlay_data_tests {
    use super::*;

    #[test]
    fn test_overlay_export_data_new() {
        let data = OverlayExportData::new(100, 100, "Test Overlay", 0);

        assert_eq!(data.width, 100);
        assert_eq!(data.height, 100);
        assert_eq!(data.pixel_count(), 10000);
        assert_eq!(data.rgba_data.len(), 40000);
        assert!(data.is_empty()); // All zeros = all transparent
    }

    #[test]
    fn test_overlay_export_data_set_pixel() {
        let mut data = OverlayExportData::new(10, 10, "Test", 0);
        data.set_pixel(5, 5, 255, 128, 64, 255);

        let pixel = data.get_pixel(5, 5);
        assert!(pixel.is_some());
        assert_eq!(pixel.unwrap(), (255, 128, 64, 255));
    }

    #[test]
    fn test_overlay_export_data_get_pixel_out_of_bounds() {
        let data = OverlayExportData::new(10, 10, "Test", 0);
        let pixel = data.get_pixel(15, 15);
        assert!(pixel.is_none());
    }

    #[test]
    fn test_overlay_export_data_set_pixel_out_of_bounds() {
        let mut data = OverlayExportData::new(10, 10, "Test", 0);
        // Should not panic
        data.set_pixel(15, 15, 255, 0, 0, 255);
        // Pixel should not be set
        assert!(data.get_pixel(15, 15).is_none());
    }

    #[test]
    fn test_overlay_export_data_suggested_filename() {
        let mut data = OverlayExportData::new(100, 100, "Test Overlay", 42);
        data.format = OverlayImageFormat::Png;

        let filename = data.suggested_filename();
        assert_eq!(filename, "test_overlay_00042.png");
    }

    #[test]
    fn test_overlay_export_data_is_empty() {
        let mut data = OverlayExportData::new(10, 10, "Test", 0);
        assert!(data.is_empty());

        data.set_pixel(5, 5, 255, 0, 0, 255);
        assert!(!data.is_empty());
    }

    #[test]
    fn test_overlay_export_data_pixel_count() {
        let data = OverlayExportData::new(100, 50, "Test", 0);
        assert_eq!(data.pixel_count(), 5000);
    }
}

// ============================================================================
// OverlayImageFormat Tests
// ============================================================================
#[cfg(test)]
mod overlay_format_tests {
    use super::*;

    #[test]
    fn test_overlay_format_extension() {
        assert_eq!(OverlayImageFormat::RawRgba.extension(), "rgba");
        assert_eq!(OverlayImageFormat::Png.extension(), "png");
        assert_eq!(OverlayImageFormat::Ppm.extension(), "ppm");
    }
}

// ============================================================================
// OverlayType Tests
// ============================================================================
#[cfg(test)]
mod overlay_type_tests {
    use super::*;

    #[test]
    fn test_overlay_type_name() {
        assert_eq!(OverlayType::QpHeatmap.name(), "QP Heatmap");
        assert_eq!(OverlayType::MotionVector.name(), "Motion Vector");
        assert_eq!(OverlayType::PartitionGrid.name(), "Partition Grid");
        assert_eq!(OverlayType::DiffHeatmap.name(), "Diff Heatmap");
    }
}

// ============================================================================
// OverlayExportRequest Tests
// ============================================================================
#[cfg(test)]
mod overlay_request_tests {
    use super::*;

    #[test]
    fn test_overlay_export_request_default() {
        let request = OverlayExportRequest::default();

        assert_eq!(request.overlay_type, OverlayType::QpHeatmap);
        assert_eq!(request.frame_idx, 0);
        assert_eq!(request.format, OverlayImageFormat::Ppm);
        assert!(request.include_alpha);
        assert_eq!(request.scale, 1.0);
    }
}

// ============================================================================
// Context Menu Tests
// ============================================================================
#[cfg(test)]
mod context_menu_tests {
    use super::*;

    #[test]
    fn test_evaluate_guard_always() {
        let context = create_test_context();
        let (enabled, reason) = evaluate_context_menu_guard("always", &context);
        assert!(enabled);
        assert!(reason.is_none());
    }

    #[test]
    fn test_evaluate_guard_has_selection() {
        let mut context = create_test_context();
        context.has_selection = true;
        let (enabled, reason) = evaluate_context_menu_guard("has_selection", &context);
        assert!(enabled);
        assert!(reason.is_none());
    }

    #[test]
    fn test_evaluate_guard_has_selection_false() {
        let mut context = create_test_context();
        context.has_selection = false;
        let (enabled, reason) = evaluate_context_menu_guard("has_selection", &context);
        assert!(!enabled);
        assert!(reason.is_some());
        assert_eq!(reason.unwrap(), "No selection.");
    }

    #[test]
    fn test_evaluate_guard_has_byte_range() {
        let mut context = create_test_context();
        context.has_byte_range = true;
        let (enabled, reason) = evaluate_context_menu_guard("has_byte_range", &context);
        assert!(enabled);
        assert!(reason.is_none());
    }

    #[test]
    fn test_evaluate_guard_unknown() {
        let context = create_test_context();
        let (enabled, reason) = evaluate_context_menu_guard("unknown_guard", &context);
        assert!(!enabled);
        assert!(reason.is_some());
        assert!(reason.unwrap().contains("Unknown guard"));
    }

    #[test]
    fn test_build_context_menu_player() {
        let context = create_test_context();
        let items = build_context_menu(ContextMenuScope::Player, &context);

        assert!(!items.is_empty());
        assert!(items.len() >= 2);

        // Check that "always" guard items are enabled
        let export_bundle = items.iter().find(|i| i.id == "export_bundle");
        assert!(export_bundle.is_some());
        assert!(export_bundle.unwrap().enabled);
    }

    #[test]
    fn test_build_context_menu_player_no_selection() {
        let mut context = create_test_context();
        context.has_selection = false;
        let items = build_context_menu(ContextMenuScope::Player, &context);

        let toggle_detail = items.iter().find(|i| i.id == "toggle_detail");
        assert!(toggle_detail.is_some());
        assert!(!toggle_detail.unwrap().enabled);
        assert!(toggle_detail.unwrap().disabled_reason.is_some());
    }

    #[test]
    fn test_build_context_menu_hex_view() {
        let context = create_test_context();
        let items = build_context_menu(ContextMenuScope::HexView, &context);

        assert!(!items.is_empty());

        // Should have copy_bytes with has_byte_range guard
        let copy_bytes = items.iter().find(|i| i.id == "copy_bytes");
        assert!(copy_bytes.is_some());
    }
}

// ============================================================================
// DiagnosticsExportRow Tests
// ============================================================================
#[cfg(test)]
mod diagnostics_row_tests {
    use super::*;

    #[test]
    fn test_diagnostics_export_row_from_diagnostic() {
        let diag = create_test_diagnostic(42, DiagnosticSeverity::Warn);
        let row = DiagnosticsExportRow::from_diagnostic(&diag);

        assert_eq!(row.id, 42);
        assert_eq!(row.severity, "Warn");
        assert_eq!(row.category, "Bitstream");
        assert_eq!(row.message, "Test diagnostic message");
        assert_eq!(row.byte_offset, 1024);
        assert_eq!(row.frame_idx, None);
        assert_eq!(row.stream_id, "A");
    }
}
