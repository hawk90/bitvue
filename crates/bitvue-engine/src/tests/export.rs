#![allow(hidden_glob_reexports)]
#![allow(unreachable_code)]
#![allow(non_camel_case_types)]
#![allow(unused_assignments)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(unused_comparisons)]
#![allow(unused_doc_comments)]
//! Tests for the Export module
//!
//! Tests for CSV/JSON export, overlay image export, evidence bundle export,
//! context menus, and semantic probe runner.

// Export module public types
use crate::export::{
    build_context_menu, create_diff_heatmap_export, create_hit_test_input,
    create_qp_heatmap_export, create_selection_input, create_tooltip_field, create_tooltip_input,
    evaluate_context_menu_guard, export_frames_csv, export_frames_json, export_overlay_ppm,
    export_overlay_rgba, ContextMenuScope, EvidenceBundleExportRequest, EvidenceBundleExportResult,
    EvidenceBundleManifest, ExportConfig, ExportSummary, FrameExportRow, GuardEvalContext,
    OverlayExportData, OverlayExportRequest, OverlayExportResult, OverlayImageFormat, OverlayType,
    SemanticProbeRunner,
};

// Types from other modules
use crate::parity_harness::{EntityRef, OrderType, ProbeOutcome, SelectionSnapshot, ViewportState};
use crate::timeline::{FrameMarker, TimelineFrame};

fn create_test_frames() -> Vec<TimelineFrame> {
    vec![
        TimelineFrame {
            display_idx: 0,
            size_bytes: 10000,
            frame_type: "I".to_string(),
            marker: FrameMarker::Key,
            pts: Some(0),
            dts: Some(0),
            is_selected: false,
        },
        TimelineFrame {
            display_idx: 1,
            size_bytes: 2000,
            frame_type: "P".to_string(),
            marker: FrameMarker::None,
            pts: Some(1000),
            dts: Some(1000),
            is_selected: false,
        },
        TimelineFrame {
            display_idx: 2,
            size_bytes: 1500,
            frame_type: "B".to_string(),
            marker: FrameMarker::None,
            pts: Some(2000),
            dts: Some(3000),
            is_selected: false,
        },
    ]
}

#[test]
fn test_export_frames_csv() {
    let frames = create_test_frames();
    let mut output = Vec::new();

    let result = export_frames_csv(&frames, &mut output, ExportConfig::default()).unwrap();

    assert_eq!(result.row_count, 3);
    assert!(result.bytes_written > 0);

    let csv = String::from_utf8(output).unwrap();
    assert!(csv.contains("display_idx,frame_type,size_bytes"));
    assert!(csv.contains("0,I,10000"));
    assert!(csv.contains("1,P,2000"));
}

#[test]
fn test_export_frames_csv_with_range() {
    let frames = create_test_frames();
    let mut output = Vec::new();

    let config = ExportConfig {
        range: Some((0, 1)),
        ..Default::default()
    };
    let result = export_frames_csv(&frames, &mut output, config).unwrap();

    assert_eq!(result.row_count, 2); // Only frames 0 and 1
}

#[test]
fn test_export_frames_json() {
    let frames = create_test_frames();
    let mut output = Vec::new();

    let config = ExportConfig {
        range: None,
        pretty: true,
    };
    let result = export_frames_json(&frames, &mut output, config).unwrap();

    assert_eq!(result.row_count, 3);

    let json = String::from_utf8(output).unwrap();
    let parsed: Vec<FrameExportRow> = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.len(), 3);
    assert_eq!(parsed[0].frame_type, "I");
    assert!(parsed[0].is_key);
}

#[test]
fn test_export_summary() {
    let frames = create_test_frames();
    let summary = ExportSummary::from_frames(&frames);

    assert_eq!(summary.total_frames, 3);
    assert_eq!(summary.total_bytes, 13500);
    assert_eq!(summary.key_frame_count, 1);
    assert_eq!(summary.error_count, 0);
    assert_eq!(summary.min_frame_size, 1500);
    assert_eq!(summary.max_frame_size, 10000);
}

// ═══════════════════════════════════════════════════════════════════════════
// Overlay Image Export Tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_overlay_image_format_extension() {
    assert_eq!(OverlayImageFormat::RawRgba.extension(), "rgba");
    assert_eq!(OverlayImageFormat::Png.extension(), "png");
    assert_eq!(OverlayImageFormat::Ppm.extension(), "ppm");
}

#[test]
fn test_overlay_export_data_new() {
    let data = OverlayExportData::new(100, 50, "Test Overlay", 42);

    assert_eq!(data.width, 100);
    assert_eq!(data.height, 50);
    assert_eq!(data.overlay_type, "Test Overlay");
    assert_eq!(data.frame_idx, 42);
    assert_eq!(data.rgba_data.len(), 100 * 50 * 4); // RGBA
    assert_eq!(data.pixel_count(), 5000);
}

#[test]
fn test_overlay_export_data_set_get_pixel() {
    let mut data = OverlayExportData::new(10, 10, "Test", 0);

    // Set pixel at (5, 5)
    data.set_pixel(5, 5, 255, 128, 64, 200);

    // Get same pixel
    let pixel = data.get_pixel(5, 5).unwrap();
    assert_eq!(pixel, (255, 128, 64, 200));

    // Out of bounds should return None
    assert!(data.get_pixel(100, 100).is_none());
}

#[test]
fn test_overlay_export_data_suggested_filename() {
    let data = OverlayExportData::new(100, 100, "QP Heatmap", 123);

    let filename = data.suggested_filename();
    assert_eq!(filename, "qp_heatmap_00123.rgba");
}

#[test]
fn test_overlay_export_data_is_empty() {
    let data = OverlayExportData::new(10, 10, "Test", 0);
    assert!(data.is_empty()); // All pixels are transparent (alpha = 0)

    let mut data2 = OverlayExportData::new(10, 10, "Test", 0);
    data2.set_pixel(0, 0, 255, 0, 0, 255);
    assert!(!data2.is_empty()); // Has at least one non-transparent pixel
}

#[test]
fn test_overlay_type_name() {
    assert_eq!(OverlayType::QpHeatmap.name(), "QP Heatmap");
    assert_eq!(OverlayType::MotionVector.name(), "Motion Vector");
    assert_eq!(OverlayType::PartitionGrid.name(), "Partition Grid");
    assert_eq!(OverlayType::DiffHeatmap.name(), "Diff Heatmap");
}

#[test]
fn test_overlay_export_request_default() {
    let request = OverlayExportRequest::default();

    assert_eq!(request.overlay_type, OverlayType::QpHeatmap);
    assert_eq!(request.frame_idx, 0);
    assert_eq!(request.format, OverlayImageFormat::Ppm);
    assert!(request.include_alpha);
    assert_eq!(request.scale, 1.0);
}

#[test]
fn test_export_overlay_ppm() {
    let mut data = OverlayExportData::new(2, 2, "Test", 0);
    // Set a red pixel at (0, 0)
    data.set_pixel(0, 0, 255, 0, 0, 255);
    // Set a green pixel at (1, 0)
    data.set_pixel(1, 0, 0, 255, 0, 255);
    // Set a blue pixel at (0, 1)
    data.set_pixel(0, 1, 0, 0, 255, 255);
    // Transparent pixel at (1, 1) - should blend to white

    let mut output = Vec::new();
    let result = export_overlay_ppm(&data, &mut output).unwrap();

    assert!(result.bytes_written > 0);
    assert_eq!(result.row_count, 4); // 2x2 = 4 pixels

    // Check PPM header
    let output_str = String::from_utf8_lossy(&output);
    assert!(output_str.starts_with("P6\n2 2\n255\n"));
}

#[test]
fn test_export_overlay_rgba() {
    let mut data = OverlayExportData::new(4, 4, "Test", 0);
    data.set_pixel(0, 0, 255, 0, 0, 255);

    let mut output = Vec::new();
    let result = export_overlay_rgba(&data, &mut output).unwrap();

    assert_eq!(result.bytes_written, 4 * 4 * 4); // 4x4 pixels * 4 bytes
    assert_eq!(output.len(), 64);

    // Check first pixel (red)
    assert_eq!(output[0], 255); // R
    assert_eq!(output[1], 0); // G
    assert_eq!(output[2], 0); // B
    assert_eq!(output[3], 255); // A
}

#[test]
fn test_overlay_export_result_success() {
    let result = OverlayExportResult::success(
        Some("/tmp/test.ppm".to_string()),
        1000,
        640,
        480,
        OverlayImageFormat::Ppm,
    );

    assert!(result.success);
    assert_eq!(result.file_path, Some("/tmp/test.ppm".to_string()));
    assert_eq!(result.bytes_written, 1000);
    assert_eq!(result.width, 640);
    assert_eq!(result.height, 480);
    assert!(result.error.is_none());
}

#[test]
fn test_overlay_export_result_error() {
    let result = OverlayExportResult::error("Failed to write file");

    assert!(!result.success);
    assert!(result.file_path.is_none());
    assert_eq!(result.bytes_written, 0);
    assert_eq!(result.error, Some("Failed to write file".to_string()));
}

#[test]
fn test_create_diff_heatmap_export() {
    use crate::diff_heatmap::{DiffHeatmapData, DiffMode};

    // Create test luma planes
    let luma_a = vec![100u8; 16];
    let luma_b = vec![50u8; 16];

    let diff_data = DiffHeatmapData::from_luma_planes(&luma_a, &luma_b, 4, 4, DiffMode::Abs);
    let export = create_diff_heatmap_export(&diff_data, 5, 1.0);

    assert_eq!(export.width, 2); // Half-res
    assert_eq!(export.height, 2);
    assert_eq!(export.overlay_type, "Diff Heatmap");
    assert_eq!(export.frame_idx, 5);

    // Should have colored pixels (not empty due to diff)
    assert!(!export.is_empty());
}

#[test]
fn test_create_qp_heatmap_export() {
    use crate::qp_heatmap::QPGrid;

    // Create a simple 4x4 QP grid with varying values
    let qp_values = vec![
        20, 24, 28, 32, 22, 26, 30, 34, 24, 28, 32, 36, 26, 30, 34, 38,
    ];

    let qp_grid = QPGrid::new(4, 4, 8, 8, qp_values, -1);
    let export = create_qp_heatmap_export(&qp_grid, 10, 1.0);

    assert_eq!(export.width, 4);
    assert_eq!(export.height, 4);
    assert_eq!(export.overlay_type, "QP Heatmap");
    assert_eq!(export.frame_idx, 10);

    // Should have colored pixels (not empty)
    assert!(!export.is_empty());

    // Check that first pixel has some color (low QP = blue)
    let pixel = export.get_pixel(0, 0).unwrap();
    assert!(pixel.3 > 0); // Has alpha (not transparent)
}

#[test]
fn test_create_qp_heatmap_export_with_missing() {
    use crate::qp_heatmap::QPGrid;

    // Grid with some missing values
    let qp_values = vec![
        20, -1, 28, -1, -1, 26, -1, 34, 24, -1, 32, -1, -1, 30, -1, 38,
    ];

    let qp_grid = QPGrid::new(4, 4, 8, 8, qp_values, -1);
    let export = create_qp_heatmap_export(&qp_grid, 0, 1.0);

    // Check that missing values result in transparent pixels
    let pixel_missing = export.get_pixel(1, 0).unwrap();
    assert_eq!(pixel_missing.3, 0); // Transparent for missing

    let pixel_present = export.get_pixel(0, 0).unwrap();
    assert!(pixel_present.3 > 0); // Has alpha for present QP
}

// ═══════════════════════════════════════════════════════════════════════════
// Evidence Bundle Export Tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_evidence_bundle_manifest_default() {
    let manifest = EvidenceBundleManifest::default();

    assert_eq!(manifest.bundle_version, "1.0");
    assert_eq!(manifest.order_type, OrderType::Display);
    assert_eq!(manifest.workspace, "player");
    assert_eq!(manifest.mode, "normal");
}

#[test]
fn test_evidence_bundle_export_request_default() {
    let request = EvidenceBundleExportRequest::default();

    assert!(request.include_screenshots);
    assert!(request.include_render_snapshots);
    assert!(!request.include_interaction_trace);
    assert!(!request.include_logs);
    assert_eq!(request.order_type, OrderType::Display);
}

#[test]
fn test_evidence_bundle_export_result_success() {
    let result = EvidenceBundleExportResult::success(
        "/tmp/bundle".to_string(),
        vec!["manifest.json".to_string()],
        1000,
    );

    assert!(result.success);
    assert_eq!(result.bundle_path, Some("/tmp/bundle".to_string()));
    assert_eq!(result.files_created.len(), 1);
    assert_eq!(result.total_bytes, 1000);
    assert!(result.error.is_none());
}

#[test]
fn test_evidence_bundle_export_result_error() {
    let result = EvidenceBundleExportResult::error("Failed to write");

    assert!(!result.success);
    assert!(result.bundle_path.is_none());
    assert!(result.files_created.is_empty());
    assert_eq!(result.error, Some("Failed to write".to_string()));
}

// ═══════════════════════════════════════════════════════════════════════════
// Context Menu Tests (per context_menus.json, guard_rules.json)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_context_menu_guard_always() {
    let context = GuardEvalContext::default();
    let (enabled, reason) = evaluate_context_menu_guard("always", &context);

    assert!(enabled);
    assert!(reason.is_none());
}

#[test]
fn test_context_menu_guard_has_selection() {
    // Without selection
    let context = GuardEvalContext::default();
    let (enabled, reason) = evaluate_context_menu_guard("has_selection", &context);

    assert!(!enabled);
    assert_eq!(reason, Some("No selection.".to_string()));

    // With selection
    let context = GuardEvalContext {
        has_selection: true,
        has_byte_range: false,
    };
    let (enabled, reason) = evaluate_context_menu_guard("has_selection", &context);

    assert!(enabled);
    assert!(reason.is_none());
}

#[test]
fn test_context_menu_guard_has_byte_range() {
    // Without byte range
    let context = GuardEvalContext::default();
    let (enabled, reason) = evaluate_context_menu_guard("has_byte_range", &context);

    assert!(!enabled);
    assert_eq!(reason, Some("No byte range selected.".to_string()));

    // With byte range
    let context = GuardEvalContext {
        has_selection: false,
        has_byte_range: true,
    };
    let (enabled, reason) = evaluate_context_menu_guard("has_byte_range", &context);

    assert!(enabled);
    assert!(reason.is_none());
}

#[test]
fn test_build_context_menu_player() {
    let context = GuardEvalContext::default();
    let items = build_context_menu(ContextMenuScope::Player, &context);

    // Player menu should have 3 items
    assert_eq!(items.len(), 3);

    // Check export bundle is always enabled
    let export_item = items.iter().find(|i| i.id == "export_bundle").unwrap();
    assert!(export_item.enabled);
    assert!(export_item.disabled_reason.is_none());

    // Check selection-dependent items are disabled
    let details_item = items.iter().find(|i| i.id == "toggle_detail").unwrap();
    assert!(!details_item.enabled);
    assert_eq!(
        details_item.disabled_reason,
        Some("No selection.".to_string())
    );
}

#[test]
fn test_build_context_menu_player_with_selection() {
    let context = GuardEvalContext {
        has_selection: true,
        has_byte_range: false,
    };
    let items = build_context_menu(ContextMenuScope::Player, &context);

    // All items should be enabled with selection
    let details_item = items.iter().find(|i| i.id == "toggle_detail").unwrap();
    assert!(details_item.enabled);
    assert!(details_item.disabled_reason.is_none());
}

#[test]
fn test_build_context_menu_hex_view() {
    let context = GuardEvalContext::default();
    let items = build_context_menu(ContextMenuScope::HexView, &context);

    // HexView menu should have 2 items
    assert_eq!(items.len(), 2);

    // Copy bytes requires byte range
    let copy_item = items.iter().find(|i| i.id == "copy_bytes").unwrap();
    assert!(!copy_item.enabled);
    assert_eq!(
        copy_item.disabled_reason,
        Some("No byte range selected.".to_string())
    );
}

#[test]
fn test_build_context_menu_stream_view() {
    let context = GuardEvalContext::default();
    let items = build_context_menu(ContextMenuScope::StreamView, &context);

    // StreamView menu should have 2 items (order type options)
    assert_eq!(items.len(), 2);

    // Both should be always enabled
    for item in &items {
        assert!(item.enabled);
        assert!(item.disabled_reason.is_none());
    }
}

#[test]
fn test_context_menu_disabled_shows_reason() {
    // Per guard_rules.json: "disabled_items_must_show_reason_tooltip": true
    let context = GuardEvalContext::default();
    let items = build_context_menu(ContextMenuScope::Player, &context);

    // Find disabled items and verify they have reasons
    for item in &items {
        if !item.enabled {
            assert!(
                item.disabled_reason.is_some(),
                "Disabled item {} must show reason",
                item.id
            );
        }
    }
}

// =========================================================================
// Semantic Probe Runner Tests
// =========================================================================

#[test]
fn test_semantic_probe_runner_new() {
    let runner = SemanticProbeRunner::new();
    assert!(runner.results.is_empty());
    assert!(runner.violations.is_empty());
    assert!(!runner.has_hard_fails());
}

#[test]
fn test_semantic_probe_runner_set_context() {
    let mut runner = SemanticProbeRunner::new();
    runner.set_context("stream_001", "av1", "player", "normal");

    assert_eq!(runner.stream_id, "stream_001");
    assert_eq!(runner.codec, "av1");
    assert_eq!(runner.workspace, "player");
    assert_eq!(runner.mode, "normal");
}

#[test]
fn test_semantic_probe_runner_hit_test() {
    let mut runner = SemanticProbeRunner::new();
    runner.set_context("stream_001", "av1", "player", "normal");

    let viewport = ViewportState {
        x: 0.0,
        y: 0.0,
        width: 1920.0,
        height: 1080.0,
        zoom: 1.0,
        pan_x: 0.0,
        pan_y: 0.0,
    };

    let entity = EntityRef {
        kind: "block".to_string(),
        id: "block_42".to_string(),
        frame_index: Some(10),
        byte_offset: Some(1000),
    };

    let result = runner.run_hit_test_probe("player", viewport, Some(entity), "partition", None);

    assert!(result.probe_id.starts_with("hit_test_player_"));
    assert_eq!(result.stream_id, "stream_001");
    assert_eq!(result.codec, "av1");
    assert_eq!(runner.results.len(), 1);
    assert!(!runner.has_hard_fails());
}

#[test]
fn test_semantic_probe_runner_selection_propagation() {
    let mut runner = SemanticProbeRunner::new();
    runner.set_context("stream_001", "hevc", "compare", "dual");

    let selection = SelectionSnapshot {
        selected_entity: Some(EntityRef {
            kind: "frame".to_string(),
            id: "frame_10".to_string(),
            frame_index: Some(10),
            byte_offset: Some(5000),
        }),
        selected_byte_range: Some((1000, 2000)),
        order_type: OrderType::Display,
    };

    let result = runner.run_selection_propagation_probe(
        "timeline",
        "select",
        "txn_001",
        vec![
            "player".to_string(),
            "syntax".to_string(),
            "hex".to_string(),
        ],
        selection,
    );

    assert!(result.probe_id.starts_with("selection_timeline_"));
    assert_eq!(runner.results.len(), 1);
}

#[test]
fn test_semantic_probe_runner_tooltip_payload() {
    let mut runner = SemanticProbeRunner::new();
    runner.set_context("stream_001", "vvc", "player", "normal");

    let entity = EntityRef {
        kind: "ctu".to_string(),
        id: "ctu_5_3".to_string(),
        frame_index: Some(5),
        byte_offset: Some(12000),
    };

    let fields = vec![
        create_tooltip_field("CTU Address", "(5, 3)", None),
        create_tooltip_field("QP", "32", None),
        create_tooltip_field("Size", "64x64", None),
    ];

    let result = runner.run_tooltip_payload_probe(entity, fields);

    assert!(result.probe_id.starts_with("tooltip_ctu_5_3_"));
    if let ProbeOutcome::TooltipPayload(ref output) = result.result {
        assert_eq!(output.fields.len(), 3);
        assert_eq!(output.fields[0].label, "CTU Address");
        assert_eq!(output.fields[0].value, "(5, 3)");
    } else {
        panic!("Expected TooltipPayload outcome");
    }
}

#[test]
fn test_semantic_probe_runner_hard_fails() {
    let mut runner = SemanticProbeRunner::new();

    assert!(!runner.has_hard_fails());

    runner.record_order_type_mixing("timeline mixed display/decode");
    assert!(runner.has_hard_fails());
    assert_eq!(runner.get_violations().len(), 1);

    runner.record_stale_async("txn_old_001");
    assert_eq!(runner.get_violations().len(), 2);

    runner.record_cache_violation("frame_cache_key_5");
    assert_eq!(runner.get_violations().len(), 3);
}

#[test]
fn test_semantic_probe_runner_export() {
    let mut runner = SemanticProbeRunner::new();
    runner.set_context("stream_001", "av1", "player", "normal");

    let viewport = ViewportState {
        x: 0.0,
        y: 0.0,
        width: 1920.0,
        height: 1080.0,
        zoom: 1.0,
        pan_x: 0.0,
        pan_y: 0.0,
    };

    runner.run_hit_test_probe("player", viewport, None, "base", None);

    let json = runner.export_results().unwrap();
    assert!(json.contains("probe_id"));
    assert!(json.contains("hit_test_player_"));
    assert!(json.contains("stream_001"));
}

#[test]
fn test_semantic_probe_runner_clear() {
    let mut runner = SemanticProbeRunner::new();
    runner.set_context("stream_001", "av1", "player", "normal");

    let viewport = ViewportState {
        x: 0.0,
        y: 0.0,
        width: 1920.0,
        height: 1080.0,
        zoom: 1.0,
        pan_x: 0.0,
        pan_y: 0.0,
    };

    runner.run_hit_test_probe("player", viewport, None, "base", None);
    runner.record_order_type_mixing("test");

    assert!(!runner.results.is_empty());
    assert!(!runner.violations.is_empty());

    runner.clear();

    assert!(runner.results.is_empty());
    assert!(runner.violations.is_empty());
    assert!(!runner.has_hard_fails());
}

#[test]
fn test_create_hit_test_input() {
    let viewport = ViewportState {
        x: 0.0,
        y: 0.0,
        width: 1920.0,
        height: 1080.0,
        zoom: 2.0,
        pan_x: 50.0,
        pan_y: 100.0,
    };

    let input = create_hit_test_input("player", viewport.clone(), 150.0, 250.0);

    assert_eq!(input.panel, "player");
    assert_eq!(input.viewport.zoom, 2.0);
    assert_eq!(input.points.len(), 1);
    assert_eq!(input.points[0].x, 150.0);
    assert_eq!(input.points[0].y, 250.0);
}

#[test]
fn test_create_selection_input() {
    let input = create_selection_input("timeline", "frame_select", "txn_12345");

    assert_eq!(input.origin_panel, "timeline");
    assert_eq!(input.action, "frame_select");
    assert_eq!(input.txn, "txn_12345");
}

#[test]
fn test_create_tooltip_input() {
    let entity = EntityRef {
        kind: "block".to_string(),
        id: "block_7".to_string(),
        frame_index: Some(7),
        byte_offset: Some(3500),
    };

    let input = create_tooltip_input(entity.clone(), "compare", "diff", "hevc");

    assert_eq!(input.entity.id, "block_7");
    assert_eq!(input.context.workspace, "compare");
    assert_eq!(input.context.mode, "diff");
    assert_eq!(input.context.codec, "hevc");
}
