// MCP integration module tests
use super::*;
use std::collections::HashMap;

// ============================================================================
// Fixtures
// ============================================================================
fn create_test_resources() -> McpResources {
    McpResources {
        stream_info: McpStreamInfo {
            codec: "AV1".to_string(),
            width: 1920,
            height: 1080,
            num_frames: 1000,
            fps: 30.0,
            bitrate: 5000000,
        },
        frame_data: McpFrameData {
            frame_idx: 42,
            frame_type: "KEY".to_string(),
            size_bytes: 5000,
            qp_avg: Some(32.0),
            is_key: true,
        },
        metadata: HashMap::new(),
    }
}

fn create_test_state() -> McpSelectionState {
    McpSelectionState {
        frame_idx: Some(42),
        byte_offset: Some(1024),
        syntax_path: Some("root.child".to_string()),
    }
}

fn create_test_diagnostics() -> McpDiagnostics {
    McpDiagnostics {
        error_count: 1,
        warning_count: 2,
        info_count: 3,
        latest_errors: vec!["Error 1".to_string(), "Error 2".to_string()],
        performance_warnings: vec!["Slow decode".to_string()],
    }
}

fn create_test_metrics_summary() -> McpMetricsSummary {
    McpMetricsSummary {
        avg_psnr: Some(40.5),
        avg_ssim: Some(0.98),
        avg_vmaf: Some(92.0),
        min_psnr: Some(35.0),
        max_psnr: Some(45.0),
        bitrate: 5000000,
        avg_qp: Some(32.0),
    }
}

fn create_test_integration() -> McpIntegration {
    let mut integration = McpIntegration::new();
    integration.resources = create_test_resources();
    integration
}

// ============================================================================
// McpStreamInfo Tests
// ============================================================================
#[cfg(test)]
mod stream_info_tests {
    use super::*;

    #[test]
    fn test_stream_info_fields() {
        let info = McpStreamInfo {
            codec: "AV1".to_string(),
            width: 1920,
            height: 1080,
            num_frames: 1000,
            fps: 30.0,
            bitrate: 5000000,
        };

        assert_eq!(info.codec, "AV1");
        assert_eq!(info.width, 1920);
        assert_eq!(info.height, 1080);
        assert_eq!(info.num_frames, 1000);
        assert_eq!(info.fps, 30.0);
        assert_eq!(info.bitrate, 5000000);
    }
}

// ============================================================================
// McpFrameData Tests
// ============================================================================
#[cfg(test)]
mod frame_data_tests {
    use super::*;

    #[test]
    fn test_frame_data_fields() {
        let data = McpFrameData {
            frame_idx: 42,
            frame_type: "KEY".to_string(),
            size_bytes: 5000,
            qp_avg: Some(32.0),
            is_key: true,
        };

        assert_eq!(data.frame_idx, 42);
        assert_eq!(data.frame_type, "KEY");
        assert_eq!(data.size_bytes, 5000);
        assert_eq!(data.qp_avg, Some(32.0));
        assert!(data.is_key);
    }

    #[test]
    fn test_frame_data_no_qp() {
        let data = McpFrameData {
            frame_idx: 0,
            frame_type: "B".to_string(),
            size_bytes: 500,
            qp_avg: None,
            is_key: false,
        };

        assert_eq!(data.qp_avg, None);
        assert!(!data.is_key);
    }
}

// ============================================================================
// McpResources Tests
// ============================================================================
#[cfg(test)]
mod resources_tests {
    use super::*;

    #[test]
    fn test_resources_new() {
        let resources = McpResources::new();
        assert_eq!(resources.codec, "");
        assert_eq!(resources.width, 0);
        assert_eq!(resources.height, 0);
    }

    #[test]
    fn test_resources_complete() {
        let resources = create_test_resources();

        assert_eq!(resources.codec, "AV1");
        assert_eq!(resources.width, 1920);
        assert_eq!(resources.height, 1080);
        assert_eq!(resources.num_frames, 1000);
        assert_eq!(resources.fps, 30.0);
        assert_eq!(resources.bitrate, 5000000);
    }

    #[test]
    fn test_resources_with_metadata() {
        let mut resources = create_test_resources();
        resources.metadata.insert("key1".to_string(), "value1".to_string());
        resources.metadata.insert("key2".to_string(), "value2".to_string());

        assert_eq!(resources.metadata.len(), 2);
    }

    #[test]
    fn test_resources_stream_info() {
        let resources = create_test_resources();
        let info = resources.stream_info();

        assert_eq!(info.codec, "AV1");
        assert_eq!(info.width, 1920);
        assert_eq!(info.height, 1080);
    }

    #[test]
    fn test_resources_frame_data() {
        let resources = create_test_resources();
        let data = resources.frame_data();

        assert_eq!(data.frame_idx, 42);
        assert_eq!(data.frame_type, "KEY");
    }
}

// ============================================================================
// McpSelectionState Tests
// ============================================================================
#[cfg(test)]
mod selection_state_tests {
    use super::*;

    #[test]
    fn test_selection_state_new() {
        let state = McpSelectionState::new();
        assert!(state.frame_idx.is_none());
        assert!(state.byte_offset.is_none());
        assert!(state.syntax_path.is_none());
    }

    #[test]
    fn test_selection_state_complete() {
        let state = create_test_state();

        assert_eq!(state.frame_idx, Some(42));
        assert_eq!(state.byte_offset, Some(1024));
        assert_eq!(state.syntax_path, Some("root.child".to_string()));
    }

    #[test]
    fn test_selection_state_has_frame() {
        let state = create_test_state();
        assert!(state.has_frame());

        let empty = McpSelectionState::new();
        assert!(!empty.has_frame());
    }

    #[test]
    fn test_selection_state_has_byte_offset() {
        let state = create_test_state();
        assert!(state.has_byte_offset());

        let empty = McpSelectionState::new();
        assert!(!empty.has_byte_offset());
    }

    #[test]
    fn test_selection_state_has_syntax_path() {
        let state = create_test_state();
        assert!(state.has_syntax_path());

        let empty = McpSelectionState::new();
        assert!(!empty.has_syntax_path());
    }
}

// ============================================================================
// McpDiagnostics Tests
// ============================================================================
#[cfg(test)]
mod diagnostics_tests {
    use super::*;

    #[test]
    fn test_diagnostics_new() {
        let diag = McpDiagnostics::new();
        assert_eq!(diag.error_count, 0);
        assert_eq!(diag.warning_count, 0);
        assert_eq!(diag.info_count, 0);
        assert!(diag.latest_errors.is_empty());
        assert!(diag.performance_warnings.is_empty());
    }

    #[test]
    fn test_diagnostics_complete() {
        let diag = create_test_diagnostics();

        assert_eq!(diag.error_count, 1);
        assert_eq!(diag.warning_count, 2);
        assert_eq!(diag.info_count, 3);
        assert_eq!(diag.latest_errors.len(), 2);
        assert_eq!(diag.performance_warnings.len(), 1);
    }

    #[test]
    fn test_diagnostics_has_errors() {
        let diag = create_test_diagnostics();
        assert!(diag.has_errors());

        let empty = McpDiagnostics::new();
        assert!(!empty.has_errors());
    }

    #[test]
    fn test_diagnostics_has_warnings() {
        let diag = create_test_diagnostics();
        assert!(diag.has_warnings());

        let empty = McpDiagnostics::new();
        assert!(!empty.has_warnings());
    }

    #[test]
    fn test_diagnostics_total_count() {
        let diag = create_test_diagnostics();
        assert_eq!(diag.total_count(), 6);
    }

    #[test]
    fn test_diagnostics_add_error() {
        let mut diag = McpDiagnostics::new();
        diag.add_error("New error".to_string());

        assert_eq!(diag.error_count, 1);
        assert_eq!(diag.latest_errors.len(), 1);
    }

    #[test]
    fn test_diagnostics_add_warning() {
        let mut diag = McpDiagnostics::new();
        diag.add_warning();

        assert_eq!(diag.warning_count, 1);
    }

    #[test]
    fn test_diagnostics_add_info() {
        let mut diag = McpDiagnostics::new();
        diag.add_info();

        assert_eq!(diag.info_count, 1);
    }

    #[test]
    fn test_diagnostics_add_performance_warning() {
        let mut diag = McpDiagnostics::new();
        diag.add_performance_warning("Slow operation".to_string());

        assert_eq!(diag.performance_warnings.len(), 1);
    }

    #[test]
    fn test_diagnostics_clear() {
        let mut diag = create_test_diagnostics();
        diag.clear();

        assert_eq!(diag.error_count, 0);
        assert_eq!(diag.warning_count, 0);
        assert_eq!(diag.info_count, 0);
        assert!(diag.latest_errors.is_empty());
        assert!(diag.performance_warnings.is_empty());
    }
}

// ============================================================================
// McpMetricsSummary Tests
// ============================================================================
#[cfg(test)]
mod metrics_summary_tests {
    use super::*;

    #[test]
    fn test_metrics_summary_new() {
        let summary = McpMetricsSummary::new();
        assert!(summary.avg_psnr.is_none());
        assert!(summary.avg_ssim.is_none());
        assert!(summary.avg_vmaf.is_none());
        assert_eq!(summary.bitrate, 0);
    }

    #[test]
    fn test_metrics_summary_complete() {
        let summary = create_test_metrics_summary();

        assert_eq!(summary.avg_psnr, Some(40.5));
        assert_eq!(summary.avg_ssim, Some(0.98));
        assert_eq!(summary.avg_vmaf, Some(92.0));
        assert_eq!(summary.min_psnr, Some(35.0));
        assert_eq!(summary.max_psnr, Some(45.0));
        assert_eq!(summary.bitrate, 5000000);
        assert_eq!(summary.avg_qp, Some(32.0));
    }

    #[test]
    fn test_metrics_summary_has_psnr() {
        let summary = create_test_metrics_summary();
        assert!(summary.has_psnr());

        let empty = McpMetricsSummary::new();
        assert!(!empty.has_psnr());
    }

    #[test]
    fn test_metrics_summary_has_ssim() {
        let summary = create_test_metrics_summary();
        assert!(summary.has_ssim());

        let empty = McpMetricsSummary::new();
        assert!(!empty.has_ssim());
    }

    #[test]
    fn test_metrics_summary_has_vmaf() {
        let summary = create_test_metrics_summary();
        assert!(summary.has_vmaf());

        let empty = McpMetricsSummary::new();
        assert!(!empty.has_vmaf());
    }

    #[test]
    fn test_metrics_summary_psnr_range() {
        let summary = create_test_metrics_summary();
        let range = summary.psnr_range();
        assert_eq!(range, Some((35.0, 45.0)));

        let empty = McpMetricsSummary::new();
        assert_eq!(empty.psnr_range(), None);
    }
}

// ============================================================================
// McpIntegration Tests
// ============================================================================
#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_integration_new() {
        let integration = McpIntegration::new();
        assert!(integration.resources.codec.is_empty());
        assert!(integration.selection_state.frame_idx.is_none());
    }

    #[test]
    fn test_integration_complete() {
        let mut integration = create_test_integration();
        integration.selection_state = create_test_state();

        assert_eq!(integration.resources.codec, "AV1");
        assert_eq!(integration.selection_state.frame_idx, Some(42));
    }

    #[test]
    fn test_integration_with_diagnostics() {
        let mut integration = create_test_integration();
        integration.diagnostics = create_test_diagnostics();

        assert!(integration.diagnostics.has_errors());
        assert_eq!(integration.diagnostics.error_count, 1);
    }

    #[test]
    fn test_integration_with_metrics() {
        let mut integration = create_test_integration();
        integration.metrics = create_test_metrics_summary();

        assert!(integration.metrics.has_psnr());
        assert_eq!(integration.metrics.avg_psnr, Some(40.5));
    }

    #[test]
    fn test_integration_set_resources() {
        let mut integration = McpIntegration::new();
        integration.set_resources(create_test_resources());

        assert_eq!(integration.resources.codec, "AV1");
        assert_eq!(integration.resources.width, 1920);
    }

    #[test]
    fn test_integration_set_selection() {
        let mut integration = McpIntegration::new();
        integration.set_selection(create_test_state());

        assert_eq!(integration.selection_state.frame_idx, Some(42));
    }

    #[test]
    fn test_integration_set_diagnostics() {
        let mut integration = McpIntegration::new();
        integration.set_diagnostics(create_test_diagnostics());

        assert!(integration.diagnostics.has_errors());
    }

    #[test]
    fn test_integration_set_metrics() {
        let mut integration = McpIntegration::new();
        integration.set_metrics(create_test_metrics_summary());

        assert!(integration.metrics.has_psnr());
    }

    #[test]
    fn test_integration_get_stream_info() {
        let integration = create_test_integration();
        let info = integration.get_stream_info();

        assert_eq!(info.codec, "AV1");
        assert_eq!(info.width, 1920);
    }

    #[test]
    fn test_integration_get_frame_data() {
        let integration = create_test_integration();
        let data = integration.get_frame_data();

        assert_eq!(data.frame_idx, 42);
        assert_eq!(data.frame_type, "KEY");
    }

    #[test]
    fn test_integration_has_selection() {
        let mut integration = create_test_integration();
        assert!(!integration.has_selection());

        integration.selection_state = create_test_state();
        assert!(integration.has_selection());
    }

    #[test]
    fn test_integration_has_diagnostics() {
        let mut integration = create_test_integration();
        assert!(!integration.has_diagnostics());

        integration.diagnostics = create_test_diagnostics();
        assert!(integration.has_diagnostics());
    }

    #[test]
    fn test_integration_has_metrics() {
        let mut integration = create_test_integration();
        assert!(!integration.has_metrics());

        integration.metrics = create_test_metrics_summary();
        assert!(integration.has_metrics());
    }

    #[test]
    fn test_integration_clear() {
        let mut integration = create_test_integration();
        integration.selection_state = create_test_state();
        integration.diagnostics = create_test_diagnostics();
        integration.metrics = create_test_metrics_summary();

        integration.clear();

        assert!(!integration.has_selection());
        assert!(!integration.has_diagnostics());
        assert!(!integration.has_metrics());
    }

    #[test]
    fn test_integration_read_only() {
        let integration = create_test_integration();

        // Verify we can get read-only access to all data
        let _info = integration.get_stream_info();
        let _data = integration.get_frame_data();
        let _state = &integration.selection_state;
        let _diag = &integration.diagnostics;
        let _metrics = &integration.metrics;

        // Should not be able to modify through getters
        // (This is enforced by the API design)
    }

    #[test]
    fn test_integration_to_json() {
        let integration = create_test_integration();

        // Should be serializable to JSON
        let json = serde_json::to_string(&integration);
        assert!(json.is_ok());

        let json_str = json.unwrap();
        assert!(json_str.contains("\"codec\":\"AV1\""));
        assert!(json_str.contains("\"frame_idx\":42"));
    }

    #[test]
    fn test_integration_from_json() {
        let integration = create_test_integration();
        let json = serde_json::to_string(&integration).unwrap();

        // Should be deserializable from JSON
        let parsed: Result<McpIntegration, _> = serde_json::from_str(&json);
        assert!(parsed.is_ok());

        let parsed_integration = parsed.unwrap();
        assert_eq!(parsed_integration.resources.codec, "AV1");
        assert_eq!(parsed_integration.resources.frame_data.frame_idx, 42);
    }

    #[test]
    fn test_integration_roundtrip() {
        let original = create_test_integration();
        original.selection_state = create_test_state();

        let json = serde_json::to_string(&original).unwrap();
        let restored: McpIntegration = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.resources.codec, original.resources.codec);
        assert_eq!(restored.selection_state.frame_idx, original.selection_state.frame_idx);
    }

    #[test]
    fn test_integration_empty_state() {
        let integration = McpIntegration::new();

        assert!(!integration.has_selection());
        assert!(!integration.has_diagnostics());
        assert!(!integration.has_metrics());

        // Should still be serializable
        let json = serde_json::to_string(&integration);
        assert!(json.is_ok());
    }
}
