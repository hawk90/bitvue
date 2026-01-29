//! Integration Tests for Critical Paths
//!
//! These tests verify end-to-end functionality for the most critical user workflows.

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    /// Test export configuration struct workflow
    ///
    /// Verifies that ExportConfig properly groups related options
    /// and that default values are sensible.
    #[test]
    fn test_export_config_workflow() {
        use bitvue_core::ExportConfig;

        // Test 1: Default configuration should export all frames without pretty print
        let config = ExportConfig::default();
        assert_eq!(config.range, None, "Default should export all frames");
        assert_eq!(config.pretty, false, "Default should not pretty print");

        // Test 2: Custom configuration should override defaults
        let custom = ExportConfig {
            range: Some((0, 100)),
            pretty: true,
        };
        assert_eq!(custom.range, Some((0, 100)), "Custom range should be set");
        assert_eq!(custom.pretty, true, "Pretty print should be enabled");
    }

    /// Test quality metrics struct workflow
    ///
    /// Verifies that QualityMetrics properly groups related metric arrays.
    #[test]
    fn test_quality_metrics_workflow() {
        use bitvue_core::QualityMetrics;
        use bitvue_core::MetricPoint;

        // Create sample metrics
        let psnr = vec![MetricPoint { idx: 0, value: 40.5 }];
        let ssim = vec![MetricPoint { idx: 0, value: 0.98 }];
        let vmaf = vec![MetricPoint { idx: 0, value: 95.0 }];

        let metrics = QualityMetrics {
            psnr_y: &psnr,
            ssim_y: &ssim,
            vmaf: &vmaf,
        };

        // Verify metrics are properly grouped
        assert_eq!(metrics.psnr_y.len(), 1);
        assert_eq!(metrics.ssim_y.len(), 1);
        assert_eq!(metrics.vmaf.len(), 1);
    }

    /// Test container format detection workflow
    ///
    /// Verifies that the system can detect different container formats by extension.
    #[test]
    fn test_container_format_detection_workflow() {
        use bitvue_formats::container::detect_from_extension;
        use bitvue_formats::ContainerFormat;

        // Test 1: IVF format detection by extension
        let ivf_path = PathBuf::from("test.ivf");
        let detected = detect_from_extension(&ivf_path);
        assert_eq!(detected, ContainerFormat::IVF);

        // Test 2: MP4 format detection
        let mp4_path = PathBuf::from("test.mp4");
        let detected = detect_from_extension(&mp4_path);
        assert_eq!(detected, ContainerFormat::MP4);

        // Test 3: MKV format detection
        let mkv_path = PathBuf::from("test.mkv");
        let detected = detect_from_extension(&mkv_path);
        assert_eq!(detected, ContainerFormat::Matroska);

        // Test 4: WebM format (also Matroska)
        let webm_path = PathBuf::from("test.webm");
        let detected = detect_from_extension(&webm_path);
        assert_eq!(detected, ContainerFormat::Matroska);

        // Test 5: Unknown format
        let unknown_path = PathBuf::from("test.unknown");
        let detected = detect_from_extension(&unknown_path);
        assert_eq!(detected, ContainerFormat::Unknown);
    }

    /// Test frame type enumeration workflow
    ///
    /// Verifies that frame type detection and conversion works correctly.
    #[test]
    fn test_frame_type_workflow() {
        use bitvue_core::types::FrameType;

        // Test 1: Key frame detection
        assert!(FrameType::Key.is_key());
        assert!(!FrameType::Key.is_inter());
        assert!(FrameType::Key.is_intra());

        // Test 2: Inter frame detection
        assert!(FrameType::Inter.is_inter());
        assert!(!FrameType::Inter.is_key());
        assert!(!FrameType::Inter.is_intra());

        // Test 3: B-frame detection
        assert!(FrameType::BFrame.is_b_frame());
        assert!(!FrameType::BFrame.is_reference()); // B-frames are typically not reference frames

        // Test 4: Short name mapping
        assert_eq!(FrameType::Key.short_name(), "I");
        assert_eq!(FrameType::Inter.short_name(), "P");
        assert_eq!(FrameType::BFrame.short_name(), "B");
    }

    /// Test cache provenance tracking workflow
    ///
    /// Verifies that cache invalidation works correctly across different triggers.
    #[test]
    fn test_cache_provenance_workflow() {
        use bitvue_core::{CacheProvenanceTracker, InvalidationTrigger};

        let mut tracker = CacheProvenanceTracker::new();

        // Test 1: Initial state - no entries
        let stats = tracker.stats();
        assert_eq!(stats.total_entries, 0);

        // Test 2: Add entry
        let key = bitvue_core::CacheKey::Timeline {
            data_revision: 0,
            zoom_level_x100: 100,
            filter_hash: 0,
        };
        tracker.add_entry(key.clone(), 1024, "test".to_string());
        let stats = tracker.stats();
        assert_eq!(stats.total_entries, 1);

        // Test 3: Record hit
        tracker.record_hit(&key);
        let stats = tracker.stats();
        assert_eq!(stats.hit_count, 1);

        // Test 4: Invalidation
        tracker.invalidate(InvalidationTrigger::DataRevision(1));
        let stats = tracker.stats();
        assert_eq!(stats.invalid_entries, 1);
    }

    /// Test overlay layer enumeration workflow
    ///
    /// Verifies that all overlay layers are properly defined.
    #[test]
    fn test_overlay_layer_workflow() {
        use bitvue_core::types::OverlayLayer;

        // Test 1: All layers are accessible
        let all_layers = OverlayLayer::all();
        assert!(!all_layers.is_empty(), "Should have overlay layers");

        // Test 2: Each layer has a name
        for layer in all_layers {
            let name: &str = layer.name();
            assert!(!name.is_empty(), "Layer {} should have a name", format!("{:?}", layer));
        }
    }
}
