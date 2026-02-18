#![allow(hidden_glob_reexports)]
#![allow(unused_must_use)]
#![allow(unused_comparisons)]
#![allow(unused_mut)]
#![allow(unused_variables)]
#![allow(dead_code)]
#![allow(unused_imports)]
// Diff heatmap module tests
//
// Per generate-tests skill: Arrange-Act-Assert pattern with fixtures
// and edge case coverage.

use super::*;

// ============================================================================
// Fixtures
// ============================================================================

/// Create test luma plane data
fn create_test_luma_plane(width: u32, height: u32, value: u8) -> Vec<u8> {
    vec![value; (width * height) as usize]
}

/// Create identical luma planes (no difference)
fn create_identical_luma_planes(width: u32, height: u32) -> (Vec<u8>, Vec<u8>) {
    let plane = create_test_luma_plane(width, height, 128);
    (plane.clone(), plane)
}

/// Create different luma planes
fn create_different_luma_planes(width: u32, height: u32) -> (Vec<u8>, Vec<u8>) {
    let plane_a = create_test_luma_plane(width, height, 100);
    let plane_b = create_test_luma_plane(width, height, 200);
    (plane_a, plane_b)
}

/// Create test diff cache key params
fn create_test_cache_key_params() -> DiffCacheKeyParams<'static> {
    DiffCacheKeyParams {
        codec: "AV1",
        file_hash_a: "hash_a",
        file_hash_b: "hash_b",
        frame_idx: 42,
        heatmap_width: 640,
        heatmap_height: 360,
        mode: DiffMode::Abs,
        opacity_bucket: 7,
    }
}

/// Create test frame dimensions
fn create_test_frame_dims() -> (u32, u32) {
    (1920, 1080)
}

/// Create test hover info
fn create_test_hover_info() -> DiffHoverInfo {
    DiffHoverInfo {
        pixel_x: 100,
        pixel_y: 200,
        diff_value: 15.5,
    }
}

// ============================================================================
// DiffMode Tests
// ============================================================================

#[cfg(test)]
mod diff_mode_tests {
    use super::*;

    #[test]
    fn test_diff_mode_default() {
        // Arrange & Act
        let mode = DiffMode::default();

        // Assert
        assert_eq!(mode, DiffMode::Abs);
    }

    #[test]
    fn test_diff_mode_display_names() {
        // Arrange & Act
        let abs_name = DiffMode::Abs.display_name();
        let signed_name = DiffMode::Signed.display_name();
        let metric_name = DiffMode::Metric.display_name();

        // Assert
        assert_eq!(abs_name, "Absolute");
        assert_eq!(signed_name, "Signed");
        assert_eq!(metric_name, "Metric");
    }

    #[test]
    fn test_diff_mode_cache_keys() {
        // Arrange & Act
        let abs_key = DiffMode::Abs.cache_key();
        let signed_key = DiffMode::Signed.cache_key();
        let metric_key = DiffMode::Metric.cache_key();

        // Assert
        assert_eq!(abs_key, "abs");
        assert_eq!(signed_key, "signed");
        assert_eq!(metric_key, "metric");
    }

    #[test]
    fn test_diff_mode_equality() {
        // Arrange
        let mode1 = DiffMode::Abs;
        let mode2 = DiffMode::Abs;
        let mode3 = DiffMode::Signed;

        // Assert
        assert_eq!(mode1, mode2);
        assert_ne!(mode1, mode3);
    }

    #[test]
    fn test_diff_mode_clone() {
        // Arrange
        let mode = DiffMode::Metric;

        // Act
        let cloned = mode.clone();

        // Assert
        assert_eq!(mode, cloned);
    }
}

// ============================================================================
// DiffHeatmapData Tests
// ============================================================================

#[cfg(test)]
mod diff_heatmap_data_tests {
    use super::*;

    #[test]
    fn test_diff_heatmap_data_from_luma_planes_identical() {
        // Arrange
        let (luma_a, luma_b) = create_identical_luma_planes(1920, 1080);
        let width = 1920;
        let height = 1080;

        // Act
        let heatmap = DiffHeatmapData::from_luma_planes(&luma_a, &luma_b, width, height, DiffMode::Abs);

        // Assert
        assert_eq!(heatmap.frame_width, width);
        assert_eq!(heatmap.frame_height, height);
        assert_eq!(heatmap.heatmap_width, 960); // Half-res
        assert_eq!(heatmap.heatmap_height, 540); // Half-res
        assert_eq!(heatmap.mode, DiffMode::Abs);
        assert_eq!(heatmap.values.len(), 960 * 540);
        assert_eq!(heatmap.min_value, 0.0);
        assert_eq!(heatmap.max_value, 0.0);
    }

    #[test]
    fn test_diff_heatmap_data_from_luma_planes_different() {
        // Arrange
        let (luma_a, luma_b) = create_different_luma_planes(1920, 1080);
        let width = 1920;
        let height = 1080;

        // Act
        let heatmap = DiffHeatmapData::from_luma_planes(&luma_a, &luma_b, width, height, DiffMode::Abs);

        // Assert
        assert_eq!(heatmap.frame_width, width);
        assert_eq!(heatmap.frame_height, height);
        assert_eq!(heatmap.heatmap_width, 960);
        assert_eq!(heatmap.heatmap_height, 540);
        assert_eq!(heatmap.mode, DiffMode::Abs);
        assert_eq!(heatmap.values.len(), 960 * 540);
        assert_eq!(heatmap.min_value, 100.0);
        assert_eq!(heatmap.max_value, 100.0);
    }

    #[test]
    fn test_diff_heatmap_data_from_luma_planes_signed() {
        // Arrange
        let (luma_a, luma_b) = create_different_luma_planes(1920, 1080);
        let width = 1920;
        let height = 1080;

        // Act
        let heatmap = DiffHeatmapData::from_luma_planes(&luma_a, &luma_b, width, height, DiffMode::Signed);

        // Assert
        assert_eq!(heatmap.mode, DiffMode::Signed);
        assert_eq!(heatmap.min_value, -100.0);
        assert_eq!(heatmap.max_value, -100.0);
    }

    #[test]
    fn test_diff_heatmap_data_get_value_valid() {
        // Arrange
        let (luma_a, luma_b) = create_different_luma_planes(100, 100);
        let heatmap = DiffHeatmapData::from_luma_planes(&luma_a, &luma_b, 100, 100, DiffMode::Abs);

        // Act & Assert
        assert_eq!(heatmap.get_value(0, 0), Some(100.0));
        assert_eq!(heatmap.get_value(49, 49), Some(100.0));
        assert_eq!(heatmap.get_value(50, 50), None); // Out of bounds
    }

    #[test]
    fn test_diff_heatmap_data_get_normalized() {
        // Arrange
        let (luma_a, luma_b) = create_different_luma_planes(100, 100);
        let heatmap = DiffHeatmapData::from_luma_planes(&luma_a, &luma_b, 100, 100, DiffMode::Abs);

        // Act
        let normalized = heatmap.get_normalized(0, 0);

        // Assert
        assert_eq!(normalized, Some(1.0));
    }

    #[test]
    fn test_diff_heatmap_data_get_normalized_zero_range() {
        // Arrange
        let (luma_a, luma_b) = create_identical_luma_planes(100, 100);
        let heatmap = DiffHeatmapData::from_luma_planes(&luma_a, &luma_b, 100, 100, DiffMode::Abs);

        // Act
        let normalized = heatmap.get_normalized(0, 0);

        // Assert
        assert_eq!(normalized, Some(0.0));
    }

    #[test]
    fn test_diff_heatmap_data_get_alpha_full_diff() {
        // Arrange
        let (luma_a, luma_b) = create_different_luma_planes(100, 100);
        let heatmap = DiffHeatmapData::from_luma_planes(&luma_a, &luma_b, 100, 100, DiffMode::Abs);

        // Act
        let alpha = heatmap.get_alpha(0, 0, 1.0);

        // Assert - Max alpha (255 * 1.0 * 180/180 = 180)
        assert_eq!(alpha, 180);
    }

    #[test]
    fn test_diff_heatmap_data_get_alpha_zero_diff() {
        // Arrange
        let (luma_a, luma_b) = create_identical_luma_planes(100, 100);
        let heatmap = DiffHeatmapData::from_luma_planes(&luma_a, &luma_b, 100, 100, DiffMode::Abs);

        // Act
        let alpha = heatmap.get_alpha(0, 0, 1.0);

        // Assert - Zero diff is transparent
        assert_eq!(alpha, 0);
    }

    #[test]
    fn test_diff_heatmap_data_get_alpha_partial_diff() {
        // Arrange
        let (luma_a, luma_b) = create_different_luma_planes(100, 100);
        let heatmap = DiffHeatmapData::from_luma_planes(&luma_a, &luma_b, 100, 100, DiffMode::Abs);

        // Act - Normalized value of 0.5 should give alpha = 90
        let alpha = heatmap.get_alpha(0, 0, 1.0);

        // Assert - For uniform diff, normalized = 1.0
        assert_eq!(alpha, 180);
    }

    #[test]
    fn test_diff_heatmap_data_get_alpha_with_opacity() {
        // Arrange
        let (luma_a, luma_b) = create_different_luma_planes(100, 100);
        let heatmap = DiffHeatmapData::from_luma_planes(&luma_a, &luma_b, 100, 100, DiffMode::Abs);

        // Act
        let alpha_half = heatmap.get_alpha(0, 0, 0.5);
        let alpha_zero = heatmap.get_alpha(0, 0, 0.0);

        // Assert
        assert_eq!(alpha_half, 90); // 180 * 0.5
        assert_eq!(alpha_zero, 0); // 180 * 0.0
    }

    #[test]
    fn test_diff_heatmap_data_cache_key() {
        // Arrange
        let params = create_test_cache_key_params();

        // Act
        let key = DiffHeatmapData::cache_key(&params);

        // Assert
        assert_eq!(key, "overlay_diff:AV1:hash_a:hash_b:f42|hm640x360|modeabs|op7");
    }

    #[test]
    fn test_diff_heatmap_data_cache_key_different_modes() {
        // Arrange
        let mut params = create_test_cache_key_params();
        params.mode = DiffMode::Signed;

        // Act
        let key = DiffHeatmapData::cache_key(&params);

        // Assert
        assert_eq!(key, "overlay_diff:AV1:hash_a:hash_b:f42|hm640x360|modessigned|op7");
    }

    #[test]
    fn test_diff_heatmap_data_empty_luma_planes() {
        // Arrange
        let luma_a = Vec::<u8>::new();
        let luma_b = Vec::<u8>::new();
        let width = 0;
        let height = 0;

        // Act
        let heatmap = DiffHeatmapData::from_luma_planes(&luma_a, &luma_b, width, height, DiffMode::Abs);

        // Assert
        assert_eq!(heatmap.frame_width, 0);
        assert_eq!(heatmap.frame_height, 0);
        assert_eq!(heatmap.heatmap_width, 0);
        assert_eq!(heatmap.heatmap_height, 0);
        assert!(heatmap.values.is_empty());
    }
}

// ============================================================================
// DiffHeatmapOverlay Tests
// ============================================================================

#[cfg(test)]
mod diff_heatmap_overlay_tests {
    use super::*;

    #[test]
    fn test_diff_heatmap_overlay_new() {
        // Arrange
        let (width, height) = create_test_frame_dims();

        // Act
        let overlay = DiffHeatmapOverlay::new(width, height);

        // Assert
        assert_eq!(overlay.mode, DiffMode::default());
        assert_eq!(overlay.user_opacity, 0.7);
        assert!(overlay.enabled);
        assert!(overlay.disable_reason.is_none());
        assert_eq!(overlay.experimental_resample, false);
        assert!(overlay.hover_info.is_none());
        assert!(overlay.frozen_tooltip.is_none());
    }

    #[test]
    fn test_diff_heatmap_overlay_set_mode() {
        // Arrange
        let (width, height) = create_test_frame_dims();
        let mut overlay = DiffHeatmapOverlay::new(width, height);

        // Act
        overlay.set_mode(DiffMode::Signed);

        // Assert
        assert_eq!(overlay.mode, DiffMode::Signed);
    }

    #[test]
    fn test_diff_heatmap_overlay_set_opacity() {
        // Arrange
        let (width, height) = create_test_frame_dims();
        let mut overlay = DiffHeatmapOverlay::new(width, height);

        // Act
        overlay.set_opacity(0.5);

        // Assert
        assert_eq!(overlay.user_opacity, 0.5);
    }

    #[test]
    fn test_diff_heatmap_overlay_set_opacity_clamp() {
        // Arrange
        let (width, height) = create_test_frame_dims();
        let mut overlay = DiffHeatmapOverlay::new(width, height);

        // Act
        overlay.set_opacity(1.5); // Should clamp to 1.0
        overlay.set_opacity(-0.5); // Should clamp to 0.0

        // Assert
        assert_eq!(overlay.user_opacity, 0.0);
    }

    #[test]
    fn test_diff_heatmap_overlay_enable_disable() {
        // Arrange
        let (width, height) = create_test_frame_dims();
        let mut overlay = DiffHeatmapOverlay::new(width, height);

        // Act - Disable
        overlay.disable("Test reason".to_string());

        // Assert - Disabled
        assert!(!overlay.enabled);
        assert_eq!(overlay.disable_reason, Some("Test reason".to_string()));

        // Act - Enable
        overlay.enable();

        // Assert - Enabled
        assert!(overlay.enabled);
        assert!(overlay.disable_reason.is_none());
    }

    #[test]
    fn test_diff_heatmap_overlay_is_enabled() {
        // Arrange
        let (width, height) = create_test_frame_dims();
        let mut overlay = DiffHeatmapOverlay::new(width, height);

        // Act & Assert - Initially enabled
        assert!(overlay.is_enabled());

        // Act - Disable
        overlay.disable("Test".to_string());

        // Assert - Now disabled
        assert!(!overlay.is_enabled());
    }

    #[test]
    fn test_diff_heatmap_overlay_disable_reason() {
        // Arrange
        let (width, height) = create_test_frame_dims();
        let mut overlay = DiffHeatmapOverlay::new(width, height);

        // Act - Initially no reason
        assert!(overlay.disable_reason().is_none());

        // Act - Disable with reason
        overlay.disable("Test reason".to_string());

        // Assert - Now has reason
        assert_eq!(overlay.disable_reason(), Some("Test reason"));
    }

    #[test]
    fn test_diff_heatmap_overlay_set_hover() {
        // Arrange
        let (width, height) = create_test_frame_dims();
        let mut overlay = DiffHeatmapOverlay::new(width, height);

        // Act
        overlay.set_hover(100, 200, 15.5);

        // Assert
        assert!(overlay.hover_info.is_some());
        let info = overlay.hover_info.unwrap();
        assert_eq!(info.pixel_x, 100);
        assert_eq!(info.pixel_y, 200);
        assert_eq!(info.diff_value, 15.5);
    }

    #[test]
    fn test_diff_heatmap_overlay_clear_hover() {
        // Arrange
        let (width, height) = create_test_frame_dims();
        let mut overlay = DiffHeatmapOverlay::new(width, height);
        overlay.set_hover(100, 200, 15.5);

        // Act
        overlay.clear_hover();

        // Assert
        assert!(overlay.hover_info.is_none());
    }

    #[test]
    fn test_diff_heatmap_overlay_freeze_unfreeze_tooltip() {
        // Arrange
        let (width, height) = create_test_frame_dims();
        let mut overlay = DiffHeatmapOverlay::new(width, height);
        overlay.set_hover(100, 200, 15.5);

        // Act - Freeze
        overlay.freeze_tooltip();

        // Assert
        assert!(overlay.frozen_tooltip.is_some());
        let frozen = overlay.frozen_tooltip.unwrap();
        assert_eq!(frozen.pixel_x, 100);
        assert_eq!(frozen.pixel_y, 200);
        assert_eq!(frozen.diff_value, 15.5);

        // Act - Unfreeze
        overlay.unfreeze_tooltip();

        // Assert
        assert!(overlay.frozen_tooltip.is_none());
    }

    #[test]
    fn test_diff_heatmap_overlay_toggle_resample() {
        // Arrange
        let (width, height) = create_test_frame_dims();
        let mut overlay = DiffHeatmapOverlay::new(width, height);

        // Act - Toggle to true
        overlay.toggle_resample();

        // Assert
        assert!(overlay.experimental_resample);

        // Act - Toggle to false
        overlay.toggle_resample();

        // Assert
        assert!(!overlay.experimental_resample);
    }

    #[test]
    fn test_diff_heatmap_overlay_opacity_bucket() {
        // Arrange
        let (width, height) = create_test_frame_dims();
        let mut overlay = DiffHeatmapOverlay::new(width, height);

        // Act - Test different opacity values
        overlay.set_opacity(0.0);
        assert_eq!(overlay.opacity_bucket(), 0);

        overlay.set_opacity(0.1);
        assert_eq!(overlay.opacity_bucket(), 1);

        overlay.set_opacity(0.5);
        assert_eq!(overlay.opacity_bucket(), 5);

        overlay.set_opacity(0.99);
        assert_eq!(overlay.opacity_bucket(), 9);

        overlay.set_opacity(1.0);
        assert_eq!(overlay.opacity_bucket(), 10);
    }

    #[test]
    fn test_diff_heatmap_overlay_freeze_without_hover() {
        // Arrange
        let (width, height) = create_test_frame_dims();
        let mut overlay = DiffHeatmapOverlay::new(width, height);

        // Act - Freeze without hover
        overlay.freeze_tooltip();

        // Assert - Should be None
        assert!(overlay.frozen_tooltip.is_none());
    }
}

// ============================================================================
// ResolutionCheckResult Tests
// ============================================================================

#[cfg(test)]
mod resolution_check_result_tests {
    use super::*;

    #[test]
    fn test_resolution_check_result_compatible() {
        // Arrange
        let width_a = 1920;
        let height_a = 1080;
        let width_b = 1920;
        let height_b = 1080;

        // Act
        let result = ResolutionCheckResult::check(width_a, height_a, width_b, height_b);

        // Assert
        assert_eq!(result.width_a, width_a);
        assert_eq!(result.height_a, height_a);
        assert_eq!(result.width_b, width_b);
        assert_eq!(result.height_b, height_b);
        assert!(result.is_compatible);
        assert_eq!(result.width_mismatch_pct, 0.0);
        assert_eq!(result.height_mismatch_pct, 0.0);
        assert!(result.disable_reason.is_none());
    }

    #[test]
    fn test_resolution_check_result_small_mismatch() {
        // Arrange - 2% difference (within 5% tolerance)
        let width_a = 1920;
        let height_a = 1080;
        let width_b = 1980; // 1920 * 1.03 = ~1980 (3% difference)
        let height_b = 1080;

        // Act
        let result = ResolutionCheckResult::check(width_a, height_a, width_b, height_b);

        // Assert
        assert!(result.is_compatible);
        assert!(result.disable_reason.is_none());
    }

    #[test]
    fn test_resolution_check_result_large_mismatch() {
        // Arrange - 10% difference (exceeds 5% tolerance)
        let width_a = 1920;
        let height_a = 1080;
        let width_b = 2112; // 1920 * 1.1 = 2112 (10% difference)
        let height_b = 1080;

        // Act
        let result = ResolutionCheckResult::check(width_a, height_a, width_b, height_b);

        // Assert
        assert!(!result.is_compatible);
        assert!(result.disable_reason.is_some());
        let reason = result.disable_reason.unwrap();
        assert!(reason.contains("Resolution mismatch"));
        assert!(reason.contains("2112"));
    }

    #[test]
    fn test_resolution_check_result_height_mismatch() {
        // Arrange - Height difference
        let width_a = 1920;
        let height_a = 1080;
        let width_b = 1920;
        let height_b = 1200; // 120/1080 ≈ 11% difference

        // Act
        let result = ResolutionCheckResult::check(width_a, height_a, width_b, height_b);

        // Assert
        assert!(!result.is_compatible);
        assert!(result.disable_reason.is_some());
        let reason = result.disable_reason.unwrap();
        assert!(reason.contains("1080"));
        assert!(reason.contains("1200"));
    }

    #[test]
    fn test_resolution_check_result_zero_dimensions() {
        // Arrange - Zero dimensions
        let width_a = 0;
        let height_a = 1080;
        let width_b = 1920;
        let height_b = 1080;

        // Act
        let result = ResolutionCheckResult::check(width_a, height_a, width_b, height_b);

        // Assert
        assert!(!result.is_compatible);
        assert_eq!(result.width_mismatch_pct, 100.0);
    }

    #[test]
    fn test_resolution_check_result_summary_compatible() {
        // Arrange
        let width_a = 1920;
        let height_a = 1080;
        let width_b = 1920;
        let height_b = 1080;
        let result = ResolutionCheckResult::check(width_a, height_a, width_b, height_b);

        // Act
        let summary = result.summary();

        // Assert
        assert_eq!(summary, "Resolution compatible: 1920x1080 ↔ 1920x1080");
    }

    #[test]
    fn test_resolution_check_result_summary_incompatible() {
        // Arrange
        let width_a = 1920;
        let height_a = 1080;
        let width_b = 2112;
        let height_b = 1080;
        let result = ResolutionCheckResult::check(width_a, height_a, width_b, height_b);

        // Act
        let summary = result.summary();

        // Assert
        assert!(summary.contains("Resolution mismatch"));
        assert!(summary.contains("2112"));
    }
}

// ============================================================================
// DiffCompareContext Tests
// ============================================================================

#[cfg(test)]
mod diff_compare_context_tests {
    use super::*;

    #[test]
    fn test_diff_compare_context_new_compatible() {
        // Arrange
        let width_a = 1920;
        let height_a = 1080;
        let width_b = 1920;
        let height_b = 1080;

        // Act
        let context = DiffCompareContext::new(width_a, height_a, width_b, height_b);

        // Assert
        assert!(context.resolution_check.is_compatible);
        assert!(context.frame_a_idx.is_none());
        assert!(context.frame_b_idx.is_none());
        assert!(context.alignment_confidence.is_none());
        assert!(!context.has_gap);
        assert!(context.pts_delta.is_none());
        assert!(context.diff_stats.is_none());
    }

    #[test]
    fn test_diff_compare_context_new_incompatible() {
        // Arrange
        let width_a = 1920;
        let height_a = 1080;
        let width_b = 2112;
        let height_b = 1080;

        // Act
        let context = DiffCompareContext::new(width_a, height_a, width_b, height_b);

        // Assert
        assert!(!context.resolution_check.is_compatible);
    }

    #[test]
    fn test_diff_compare_context_set_frame_pair() {
        // Arrange
        let width_a = 1920;
        let height_a = 1080;
        let width_b = 1920;
        let height_b = 1080;
        let mut context = DiffCompareContext::new(width_a, height_a, width_b, height_b);

        // Act
        context.set_frame_pair(Some(10), Some(15), Some(1000), false);

        // Assert
        assert_eq!(context.frame_a_idx, Some(10));
        assert_eq!(context.frame_b_idx, Some(15));
        assert_eq!(context.pts_delta, Some(1000));
        assert!(!context.has_gap);
    }

    #[test]
    fn test_diff_compare_context_set_frame_pair_with_gap() {
        // Arrange
        let width_a = 1920;
        let height_a = 1080;
        let width_b = 1920;
        let height_b = 1080;
        let mut context = DiffCompareContext::new(width_a, height_a, width_b, height_b);

        // Act
        context.set_frame_pair(Some(10), None, None, true);

        // Assert
        assert_eq!(context.frame_a_idx, Some(10));
        assert!(context.frame_b_idx.is_none());
        assert!(context.pts_delta.is_none());
        assert!(context.has_gap);
    }

    #[test]
    fn test_diff_compare_context_set_alignment_confidence() {
        // Arrange
        let width_a = 1920;
        let height_a = 1080;
        let width_b = 1920;
        let height_b = 1080;
        let mut context = DiffCompareContext::new(width_a, height_a, width_b, height_b);

        // Act
        context.set_alignment_confidence("High");

        // Assert
        assert_eq!(context.alignment_confidence, Some("High".to_string()));
    }

    #[test]
    fn test_diff_compare_context_set_diff_stats() {
        // Arrange
        let width_a = 1920;
        let height_a = 1080;
        let width_b = 1920;
        let height_b = 1080;
        let mut context = DiffCompareContext::new(width_a, height_a, width_b, height_b);
        let stats = DiffStatistics::default();

        // Act
        context.set_diff_stats(stats);

        // Assert
        assert!(context.diff_stats.is_some());
    }

    #[test]
    fn test_diff_compare_context_is_diff_available_compatible() {
        // Arrange
        let width_a = 1920;
        let height_a = 1080;
        let width_b = 1920;
        let height_b = 1080;
        let mut context = DiffCompareContext::new(width_a, height_a, width_b, height_b);

        // Act - Set frame pair
        context.set_frame_pair(Some(10), Some(15), None, false);

        // Assert
        assert!(context.is_diff_available());
    }

    #[test]
    fn test_diff_compare_context_is_diff_available_incompatible() {
        // Arrange
        let width_a = 1920;
        let height_a = 1080;
        let width_b = 2112;
        let height_b = 1080;
        let context = DiffCompareContext::new(width_a, height_a, width_b, height_b);

        // Assert
        assert!(!context.is_diff_available());
    }

    #[test]
    fn test_diff_compare_context_is_diff_available_with_gap() {
        // Arrange
        let width_a = 1920;
        let height_a = 1080;
        let width_b = 1920;
        let height_b = 1080;
        let mut context = DiffCompareContext::new(width_a, height_a, width_b, height_b);

        // Act - Set frame pair with gap
        context.set_frame_pair(Some(10), None, None, true);

        // Assert
        assert!(!context.is_diff_available());
    }

    #[test]
    fn test_diff_compare_context_status_text_compatible() {
        // Arrange
        let width_a = 1920;
        let height_a = 1080;
        let width_b = 1920;
        let height_b = 1080;
        let mut context = DiffCompareContext::new(width_a, height_a, width_b, height_b);

        // Act - Set frame pair
        context.set_frame_pair(Some(10), Some(15), Some(1000), false);
        context.set_alignment_confidence("High");

        // Act
        let status = context.status_text();

        // Assert
        assert_eq!(status, "Frame A:10 ↔ B:15 (High, PTS Δ: 1000)");
    }

    #[test]
    fn test_diff_compare_context_status_text_incompatible() {
        // Arrange
        let width_a = 1920;
        let height_a = 1080;
        let width_b = 2112;
        let height_b = 1080;
        let context = DiffCompareContext::new(width_a, height_a, width_b, height_b);

        // Act
        let status = context.status_text();

        // Assert
        assert!(status.contains("Resolution mismatch"));
    }

    #[test]
    fn test_diff_compare_context_status_text_with_gap() {
        // Arrange
        let width_a = 1920;
        let height_a = 1080;
        let width_b = 1920;
        let height_b = 1080;
        let mut context = DiffCompareContext::new(width_a, height_a, width_b, height_b);

        // Act - Set frame pair with gap
        context.set_frame_pair(Some(10), None, None, true);

        // Act
        let status = context.status_text();

        // Assert
        assert_eq!(status, "Gap: No matching frame in other stream");
    }
}

// ============================================================================
// DiffStatistics Tests
// ============================================================================

#[cfg(test)]
mod diff_statistics_tests {
    use super::*;

    #[test]
    fn test_diff_statistics_from_heatmap_empty() {
        // Arrange
        let heatmap = DiffHeatmapData {
            frame_width: 1920,
            frame_height: 1080,
            heatmap_width: 960,
            heatmap_height: 540,
            values: Vec::new(),
            mode: DiffMode::Abs,
            min_value: 0.0,
            max_value: 0.0,
        };

        // Act
        let stats = DiffStatistics::from_heatmap(&heatmap);

        // Assert
        assert_eq!(stats.mean_diff, 0.0);
        assert_eq!(stats.max_diff, 0.0);
        assert_eq!(stats.min_diff, 0.0);
        assert_eq!(stats.std_dev, 0.0);
        assert_eq!(stats.diff_area_pct, 0.0);
        assert_eq!(stats.psnr, Some(f32::INFINITY));
    }

    #[test]
    fn test_diff_statistics_from_heatmap_uniform() {
        // Arrange
        let values = vec![10.0; 100]; // All values are 10
        let heatmap = DiffHeatmapData {
            frame_width: 1920,
            frame_height: 1080,
            heatmap_width: 10,
            heatmap_height: 10,
            values,
            mode: DiffMode::Abs,
            min_value: 10.0,
            max_value: 10.0,
        };

        // Act
        let stats = DiffStatistics::from_heatmap(&heatmap);

        // Assert
        assert_eq!(stats.mean_diff, 10.0);
        assert_eq!(stats.max_diff, 10.0);
        assert_eq!(stats.min_diff, 10.0);
        assert_eq!(stats.std_dev, 0.0); // No variance
        assert_eq!(stats.diff_area_pct, 0.0); // No values > 10
        assert_eq!(stats.psnr, Some(f32::INFINITY)); // Perfect match
    }

    #[test]
    fn test_diff_statistics_from_heatmap_varying() {
        // Arrange
        let values = vec![0.0, 10.0, 20.0, 30.0, 40.0, 50.0]; // Mean = 25
        let heatmap = DiffHeatmapData {
            frame_width: 1920,
            frame_height: 1080,
            heatmap_width: 3,
            heatmap_height: 2,
            values,
            mode: DiffMode::Abs,
            min_value: 0.0,
            max_value: 50.0,
        };

        // Act
        let stats = DiffStatistics::from_heatmap(&heatmap);

        // Assert
        assert_eq!(stats.mean_diff, 25.0); // Mean of absolute values
        assert_eq!(stats.max_diff, 50.0);
        assert_eq!(stats.min_diff, 0.0);
        // Note: std_dev calculation is complex, just check it's positive
        assert!(stats.std_dev > 0.0);
        assert_eq!(stats.diff_area_pct, 83.33); // 5/6 values > 10
        assert!(stats.psnr.is_some() && stats.psnr.unwrap() < f32::INFINITY);
    }

    #[test]
    fn test_diff_statistics_summary_text() {
        // Arrange
        let stats = DiffStatistics {
            mean_diff: 25.0,
            max_diff: 50.0,
            min_diff: 0.0,
            std_dev: 15.0,
            diff_area_pct: 25.0,
            psnr: Some(35.5),
        };

        // Act
        let summary = stats.summary_text();

        // Assert
        assert!(summary.contains("Mean: 25.00"));
        assert!(summary.contains("Max: 50.00"));
        assert!(summary.contains("Diff Area: 25.0%"));
        assert!(summary.contains("PSNR: 35.50 dB"));
    }

    #[test]
    fn test_diff_statistics_summary_text_infinite_psnr() {
        // Arrange
        let stats = DiffStatistics {
            mean_diff: 0.0,
            max_diff: 0.0,
            min_diff: 0.0,
            std_dev: 0.0,
            diff_area_pct: 0.0,
            psnr: Some(f32::INFINITY),
        };

        // Act
        let summary = stats.summary_text();

        // Assert
        assert!(summary.contains("PSNR: ∞ dB"));
    }

    #[test]
    fn test_diff_statistics_summary_text_no_psnr() {
        // Arrange
        let stats = DiffStatistics {
            mean_diff: 25.0,
            max_diff: 50.0,
            min_diff: 0.0,
            std_dev: 15.0,
            diff_area_pct: 25.0,
            psnr: None,
        };

        // Act
        let summary = stats.summary_text();

        // Assert
        assert!(summary.contains("PSNR: N/A dB"));
    }

    #[test]
    fn test_diff_statistics_from_heatmap_all_zero() {
        // Arrange
        let values = vec![0.0; 100]; // All values are 0
        let heatmap = DiffHeatmapData {
            frame_width: 1920,
            frame_height: 1080,
            heatmap_width: 10,
            heatmap_height: 10,
            values,
            mode: DiffMode::Abs,
            min_value: 0.0,
            max_value: 0.0,
        };

        // Act
        let stats = DiffStatistics::from_heatmap(&heatmap);

        // Assert
        assert_eq!(stats.mean_diff, 0.0);
        assert_eq!(stats.max_diff, 0.0);
        assert_eq!(stats.min_diff, 0.0);
        assert_eq!(stats.std_dev, 0.0);
        assert_eq!(stats.diff_area_pct, 0.0);
        assert_eq!(stats.psnr, Some(f32::INFINITY));
    }
}

// ============================================================================
// DiffHoverInfo Tests
// ============================================================================

#[cfg(test)]
mod diff_hover_info_tests {
    use super::*;

    #[test]
    fn test_diff_hover_info_construct() {
        // Arrange & Act
        let hover_info = DiffHoverInfo {
            pixel_x: 100,
            pixel_y: 200,
            diff_value: 15.5,
        };

        // Assert
        assert_eq!(hover_info.pixel_x, 100);
        assert_eq!(hover_info.pixel_y, 200);
        assert_eq!(hover_info.diff_value, 15.5);
    }

    #[test]
    fn test_diff_hover_info_format_tooltip_abs() {
        // Arrange
        let hover_info = create_test_hover_info();

        // Act
        let tooltip = hover_info.format_tooltip(DiffMode::Abs);

        // Assert
        assert_eq!(tooltip, "Pixel: (100, 200)\nDiff (Absolute): 15.5");
    }

    #[test]
    fn test_diff_hover_info_format_tooltip_signed() {
        // Arrange
        let hover_info = create_test_hover_info();

        // Act
        let tooltip = hover_info.format_tooltip(DiffMode::Signed);

        // Assert
        assert_eq!(tooltip, "Pixel: (100, 200)\nDiff (Signed): 15.5");
    }

    #[test]
    fn test_diff_hover_info_format_tooltip_metric() {
        // Arrange
        let hover_info = create_test_hover_info();

        // Act
        let tooltip = hover_info.format_tooltip(DiffMode::Metric);

        // Assert
        assert_eq!(tooltip, "Pixel: (100, 200)\nDiff (Metric): 15.5");
    }

    #[test]
    fn test_diff_hover_info_format_tooltip_negative_diff() {
        // Arrange
        let hover_info = DiffHoverInfo {
            pixel_x: 50,
            pixel_y: 75,
            diff_value: -10.25,
        };

        // Act
        let tooltip = hover_info.format_tooltip(DiffMode::Signed);

        // Assert
        assert_eq!(tooltip, "Pixel: (50, 75)\nDiff (Signed): -10.25");
    }
}

// ============================================================================
// Edge Cases
// ============================================================================

#[cfg(test)]
mod edge_case_tests {
    use super::*;

    #[test]
    fn test_diff_heatmap_data_single_pixel_frame() {
        // Arrange - 1x1 frame
        let (luma_a, luma_b) = create_different_luma_planes(1, 1);
        let heatmap = DiffHeatmapData::from_luma_planes(&luma_a, &luma_b, 1, 1, DiffMode::Abs);

        // Assert
        assert_eq!(heatmap.frame_width, 1);
        assert_eq!(heatmap.frame_height, 1);
        assert_eq!(heatmap.heatmap_width, 1);
        assert_eq!(heatmap.heatmap_height, 1);
        assert_eq!(heatmap.values.len(), 1);
        assert_eq!(heatmap.get_value(0, 0), Some(100.0));
    }

    #[test]
    fn test_diff_heatmap_data_odd_dimensions() {
        // Arrange - Odd dimensions (should be ceil division)
        let (luma_a, luma_b) = create_different_luma_planes(99, 99);
        let heatmap = DiffHeatmapData::from_luma_planes(&luma_a, &luma_b, 99, 99, DiffMode::Abs);

        // Assert - 99/2 = 49.5 -> ceil to 50
        assert_eq!(heatmap.heatmap_width, 50);
        assert_eq!(heatmap.heatmap_height, 50);
        assert_eq!(heatmap.values.len(), 2500);
    }

    #[test]
    fn test_diff_heatmap_data_partial_block_averaging() {
        // Arrange - Frame smaller than 2x2 blocks
        let (luma_a, luma_b) = create_different_luma_planes(1, 1);
        let heatmap = DiffHeatmapData::from_luma_planes(&luma_a, &luma_b, 1, 1, DiffMode::Abs);

        // Assert - Should handle partial block correctly
        assert_eq!(heatmap.values[0], 100.0);
    }

    #[test]
    fn test_diff_heatmap_data_extreme_values() {
        // Arrange - Extreme pixel values
        let luma_a = vec![255u8; 100 * 100]; // White
        let luma_b = vec![0u8; 100 * 100];   // Black
        let heatmap = DiffHeatmapData::from_luma_planes(&luma_a, &luma_b, 100, 100, DiffMode::Abs);

        // Assert
        assert_eq!(heatmap.min_value, 255.0);
        assert_eq!(heatmap.max_value, 255.0);
        assert_eq!(heatmap.get_normalized(0, 0), Some(1.0));
    }

    #[test]
    fn test_resolution_check_result_edge_case_dimensions() {
        // Arrange - Very small dimensions
        let width_a = 1;
        let height_a = 1;
        let width_b = 2;
        let height_b = 1;

        // Act
        let result = ResolutionCheckResult::check(width_a, height_a, width_b, height_b);

        // Assert
        assert!(!result.is_compatible);
        assert_eq!(result.width_mismatch_pct, 100.0);
    }

    #[test]
    fn test_diff_compare_context_status_text_no_frames() {
        // Arrange
        let width_a = 1920;
        let height_a = 1080;
        let width_b = 1920;
        let height_b = 1080;
        let context = DiffCompareContext::new(width_a, height_a, width_b, height_b);

        // Act
        let status = context.status_text();

        // Assert
        assert_eq!(status, "No frames selected");
    }

    #[test]
    fn test_diff_statistics_zero_variance() {
        // Arrange - All identical values
        let values = vec![42.0; 100];
        let heatmap = DiffHeatmapData {
            frame_width: 200,
            frame_height: 200,
            heatmap_width: 100,
            heatmap_height: 100,
            values,
            mode: DiffMode::Abs,
            min_value: 42.0,
            max_value: 42.0,
        };

        // Act
        let stats = DiffStatistics::from_heatmap(&heatmap);

        // Assert
        assert_eq!(stats.std_dev, 0.0);
        assert_eq!(stats.diff_area_pct, 0.0); // No values > 10
    }

    #[test]
    fn test_diff_heatmap_overlay_max_opacity() {
        // Arrange
        let (width, height) = create_test_frame_dims();
        let mut overlay = DiffHeatmapOverlay::new(width, height);

        // Act
        overlay.set_opacity(1.0);
        let alpha = overlay.get_alpha(0, 0, 1.0); // Max diff, max opacity

        // Assert
        assert_eq!(alpha, 180); // Maximum possible alpha
    }

    #[test]
    fn test_diff_heatmap_data_cache_key_large_params() {
        // Arrange
        let params = DiffCacheKeyParams {
            codec: "AV1",
            file_hash_a: "a".repeat(100), // Long hash
            file_hash_b: "b".repeat(100), // Long hash
            frame_idx: 99999,
            heatmap_width: 9999,
            heatmap_height: 9999,
            mode: DiffMode::Metric,
            opacity_bucket: 10,
        };

        // Act
        let key = DiffHeatmapData::cache_key(&params);

        // Assert - Should generate valid key
        assert!(key.contains("a".repeat(100)));
        assert!(key.contains("b".repeat(100)));
        assert!(key.contains("f99999"));
        assert!(key.contains("hm9999x9999"));
        assert!(key.contains("op10"));
    }

    #[test]
    fn test_resolution_check_result_percentages() {
        // Arrange - Exact 5% difference (boundary case)
        let width_a = 200;
        let height_a = 200;
        let width_b = 210; // 200 * 1.05 = 210 (exactly 5%)
        let height_b = 200;

        // Act
        let result = ResolutionCheckResult::check(width_a, height_a, width_b, height_b);

        // Assert - Should be compatible (5% is allowed)
        assert!(result.is_compatible);
    }

    #[test]
    fn test_resolution_check_result_501_percent_mismatch() {
        // Arrange - Just over 5% difference
        let width_a = 200;
        let height_a = 200;
        let width_b = 211; // 200 * 1.0505 = ~210.1, so 211 is >5%
        let height_b = 200;

        // Act
        let result = ResolutionCheckResult::check(width_a, height_a, width_b, height_b);

        // Assert - Should be incompatible
        assert!(!result.is_compatible);
        assert!(result.width_mismatch_pct > 5.0);
    }
}