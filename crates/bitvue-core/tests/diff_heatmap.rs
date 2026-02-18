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
//! Tests for diff_heatmap module

use bitvue_core::{
    DiffCacheKeyParams, DiffCompareContext, DiffHeatmapData, DiffHeatmapOverlay, DiffHoverInfo,
    DiffMode, DiffStatistics, ResolutionCheckResult,
};

#[test]
fn test_diff_mode() {
    assert_eq!(DiffMode::Abs.display_name(), "Absolute");
    assert_eq!(DiffMode::Signed.display_name(), "Signed");
    assert_eq!(DiffMode::Metric.display_name(), "Metric");

    assert_eq!(DiffMode::Abs.cache_key(), "abs");
    assert_eq!(DiffMode::default(), DiffMode::Abs);
}

#[test]
fn test_diff_heatmap_from_luma() {
    // Create test luma planes (4x4)
    let luma_a = vec![100u8; 16];
    let luma_b = vec![50u8; 16];

    let heatmap = DiffHeatmapData::from_luma_planes(&luma_a, &luma_b, 4, 4, DiffMode::Abs);

    // Half-res: 2x2
    assert_eq!(heatmap.heatmap_width, 2);
    assert_eq!(heatmap.heatmap_height, 2);
    assert_eq!(heatmap.values.len(), 4);

    // All values should be 50.0 (abs difference)
    for value in &heatmap.values {
        assert!((value - 50.0).abs() < 0.1);
    }
}

#[test]
fn test_diff_modes() {
    let luma_a = vec![100u8; 16];
    let luma_b = vec![50u8; 16];

    // Abs mode
    let heatmap_abs = DiffHeatmapData::from_luma_planes(&luma_a, &luma_b, 4, 4, DiffMode::Abs);
    assert!((heatmap_abs.values[0] - 50.0).abs() < 0.1);

    // Signed mode
    let heatmap_signed =
        DiffHeatmapData::from_luma_planes(&luma_a, &luma_b, 4, 4, DiffMode::Signed);
    assert!((heatmap_signed.values[0] - 50.0).abs() < 0.1); // 100 - 50 = 50
}

#[test]
fn test_get_value() {
    let luma_a = vec![100u8; 16];
    let luma_b = vec![50u8; 16];

    let heatmap = DiffHeatmapData::from_luma_planes(&luma_a, &luma_b, 4, 4, DiffMode::Abs);

    assert!(heatmap.get_value(0, 0).is_some());
    assert!(heatmap.get_value(1, 1).is_some());
    assert!(heatmap.get_value(2, 2).is_none()); // Out of bounds (2x2 heatmap)
}

#[test]
fn test_get_alpha() {
    let luma_a = vec![100u8; 16];
    let luma_b = vec![50u8; 16];

    let heatmap = DiffHeatmapData::from_luma_planes(&luma_a, &luma_b, 4, 4, DiffMode::Abs);

    // Max diff should give high alpha
    let alpha = heatmap.get_alpha(0, 0, 1.0);
    assert!(alpha > 100); // Should be close to 180

    // With reduced opacity
    let alpha_half = heatmap.get_alpha(0, 0, 0.5);
    assert!(alpha_half < alpha);
}

#[test]
fn test_zero_diff_transparent() {
    // DF01: Zero diff regions are transparent
    let luma_a = vec![100u8; 16];
    let luma_b = vec![100u8; 16]; // Same values

    let heatmap = DiffHeatmapData::from_luma_planes(&luma_a, &luma_b, 4, 4, DiffMode::Abs);

    // Zero diff should be transparent (alpha = 0)
    let alpha = heatmap.get_alpha(0, 0, 1.0);
    assert_eq!(alpha, 0);
}

#[test]
fn test_cache_key() {
    let params = DiffCacheKeyParams {
        codec: "av1",
        file_hash_a: "hash_a",
        file_hash_b: "hash_b",
        frame_idx: 42,
        heatmap_width: 320,
        heatmap_height: 180,
        mode: DiffMode::Abs,
        opacity_bucket: 7,
    };
    let key = DiffHeatmapData::cache_key(&params);

    assert!(key.contains("overlay_diff"));
    assert!(key.contains("av1"));
    assert!(key.contains("hash_a"));
    assert!(key.contains("hash_b"));
    assert!(key.contains("f42"));
    assert!(key.contains("hm320x180"));
    assert!(key.contains("modeabs"));
    assert!(key.contains("op7"));
}

#[test]
fn test_diff_overlay() {
    let mut overlay = DiffHeatmapOverlay::new(640, 360);

    assert!(overlay.is_enabled());
    assert_eq!(overlay.mode, DiffMode::Abs);

    // Set mode
    overlay.set_mode(DiffMode::Signed);
    assert_eq!(overlay.mode, DiffMode::Signed);

    // Set opacity
    overlay.set_opacity(0.5);
    assert!((overlay.user_opacity - 0.5).abs() < 0.01);

    // Opacity bucket
    assert_eq!(overlay.opacity_bucket(), 5); // 0.5 * 10 = 5
}

#[test]
fn test_disable_with_reason() {
    let mut overlay = DiffHeatmapOverlay::new(640, 360);

    overlay.disable("Resolution mismatch: 640x360 vs 1920x1080".to_string());

    assert!(!overlay.is_enabled());
    assert!(overlay.disable_reason().is_some());
    assert!(overlay
        .disable_reason()
        .unwrap()
        .contains("Resolution mismatch"));
}

#[test]
fn test_hover_and_freeze() {
    let mut overlay = DiffHeatmapOverlay::new(640, 360);

    // Set hover
    overlay.set_hover(100, 200, 42.5);
    assert!(overlay.hover_info.is_some());
    assert_eq!(overlay.hover_info.as_ref().unwrap().pixel_x, 100);

    // Freeze tooltip
    overlay.freeze_tooltip();
    assert!(overlay.frozen_tooltip.is_some());

    // Clear hover (frozen should remain)
    overlay.clear_hover();
    assert!(overlay.hover_info.is_none());
    assert!(overlay.frozen_tooltip.is_some());

    // Unfreeze
    overlay.unfreeze_tooltip();
    assert!(overlay.frozen_tooltip.is_none());
}

#[test]
fn test_hover_tooltip_format() {
    let info = DiffHoverInfo {
        pixel_x: 123,
        pixel_y: 456,
        diff_value: 42.7,
    };

    let tooltip = info.format_tooltip(DiffMode::Abs);
    assert!(tooltip.contains("123"));
    assert!(tooltip.contains("456"));
    assert!(tooltip.contains("42.7"));
    assert!(tooltip.contains("Absolute"));
}

#[test]
fn test_experimental_resample() {
    let mut overlay = DiffHeatmapOverlay::new(640, 360);

    assert!(!overlay.experimental_resample);
    overlay.toggle_resample();
    assert!(overlay.experimental_resample);
    overlay.toggle_resample();
    assert!(!overlay.experimental_resample);
}

// Resolution check tests
#[test]
fn test_resolution_check_compatible() {
    // Same resolution
    let check = ResolutionCheckResult::check(1920, 1080, 1920, 1080);
    assert!(check.is_compatible);
    assert!(check.disable_reason.is_none());
    assert_eq!(check.width_mismatch_pct, 0.0);
    assert_eq!(check.height_mismatch_pct, 0.0);
}

#[test]
fn test_resolution_check_small_mismatch() {
    // 4% mismatch (within 5% threshold)
    let check = ResolutionCheckResult::check(1920, 1080, 1843, 1037);
    assert!(check.is_compatible);
    assert!(check.disable_reason.is_none());
    assert!(check.width_mismatch_pct < 5.0);
    assert!(check.height_mismatch_pct < 5.0);
}

#[test]
fn test_resolution_check_large_mismatch() {
    // 50% mismatch (exceeds 5% threshold)
    let check = ResolutionCheckResult::check(1920, 1080, 960, 540);
    assert!(!check.is_compatible);
    assert!(check.disable_reason.is_some());
    assert!(check
        .disable_reason
        .as_ref()
        .unwrap()
        .contains("Resolution mismatch"));
    assert!(check.width_mismatch_pct > 5.0);
}

#[test]
fn test_resolution_check_zero_dimension() {
    let check = ResolutionCheckResult::check(1920, 1080, 0, 0);
    assert!(!check.is_compatible);
    assert_eq!(check.width_mismatch_pct, 100.0);
    assert_eq!(check.height_mismatch_pct, 100.0);
}

#[test]
fn test_resolution_check_summary() {
    let compatible = ResolutionCheckResult::check(1920, 1080, 1920, 1080);
    assert!(compatible.summary().contains("compatible"));

    let incompatible = ResolutionCheckResult::check(1920, 1080, 640, 360);
    assert!(
        incompatible.summary().contains("mismatch")
            || incompatible.summary().contains("Resolution")
    );
}

// DiffCompareContext tests
#[test]
fn test_diff_compare_context_new() {
    let ctx = DiffCompareContext::new(1920, 1080, 1920, 1080);
    assert!(ctx.resolution_check.is_compatible);
    assert!(ctx.frame_a_idx.is_none());
    assert!(ctx.frame_b_idx.is_none());
    assert!(!ctx.has_gap);
    assert!(ctx.diff_stats.is_none());
}

#[test]
fn test_diff_compare_context_set_frame_pair() {
    let mut ctx = DiffCompareContext::new(1920, 1080, 1920, 1080);
    ctx.set_frame_pair(Some(10), Some(10), Some(0), false);

    assert_eq!(ctx.frame_a_idx, Some(10));
    assert_eq!(ctx.frame_b_idx, Some(10));
    assert_eq!(ctx.pts_delta, Some(0));
    assert!(!ctx.has_gap);
}

#[test]
fn test_diff_compare_context_is_diff_available() {
    let mut ctx = DiffCompareContext::new(1920, 1080, 1920, 1080);

    // Initially not available (no frame pair)
    assert!(!ctx.is_diff_available());

    // Set frame pair
    ctx.set_frame_pair(Some(5), Some(5), None, false);
    assert!(ctx.is_diff_available());

    // With gap
    ctx.has_gap = true;
    assert!(!ctx.is_diff_available());
}

#[test]
fn test_diff_compare_context_incompatible_resolution() {
    let ctx = DiffCompareContext::new(1920, 1080, 640, 360);
    assert!(!ctx.resolution_check.is_compatible);
    assert!(!ctx.is_diff_available());
}

#[test]
fn test_diff_compare_context_status_text() {
    // Resolution mismatch
    let ctx_mismatch = DiffCompareContext::new(1920, 1080, 640, 360);
    assert!(
        ctx_mismatch.status_text().contains("mismatch")
            || ctx_mismatch.status_text().contains("Resolution")
    );

    // Gap
    let mut ctx_gap = DiffCompareContext::new(1920, 1080, 1920, 1080);
    ctx_gap.has_gap = true;
    assert!(ctx_gap.status_text().contains("Gap"));

    // Valid frame pair
    let mut ctx_valid = DiffCompareContext::new(1920, 1080, 1920, 1080);
    ctx_valid.set_frame_pair(Some(10), Some(10), Some(0), false);
    ctx_valid.set_alignment_confidence("High");
    let status = ctx_valid.status_text();
    assert!(status.contains("10"));
    assert!(status.contains("High"));
}

#[test]
fn test_diff_compare_context_partial_match() {
    let mut ctx = DiffCompareContext::new(1920, 1080, 1920, 1080);

    // Only A frame
    ctx.set_frame_pair(Some(5), None, None, false);
    assert!(ctx.status_text().contains("no match in B"));

    // Only B frame
    ctx.set_frame_pair(None, Some(5), None, false);
    assert!(ctx.status_text().contains("no match in A"));

    // No frames
    ctx.set_frame_pair(None, None, None, false);
    assert!(ctx.status_text().contains("No frames selected"));
}

// DiffStatistics tests
#[test]
fn test_diff_statistics_from_heatmap() {
    // Create heatmap with known diff values
    let luma_a: Vec<u8> = (0..16).map(|i| 100 + i * 5).collect();
    let luma_b: Vec<u8> = vec![100u8; 16];

    let heatmap = DiffHeatmapData::from_luma_planes(&luma_a, &luma_b, 4, 4, DiffMode::Abs);
    let stats = DiffStatistics::from_heatmap(&heatmap);

    assert!(stats.mean_diff >= 0.0);
    assert!(stats.max_diff >= stats.min_diff);
    assert!(stats.std_dev >= 0.0);
    assert!(stats.diff_area_pct >= 0.0 && stats.diff_area_pct <= 100.0);
}

#[test]
fn test_diff_statistics_identical_frames() {
    let luma = vec![100u8; 16];
    let heatmap = DiffHeatmapData::from_luma_planes(&luma, &luma, 4, 4, DiffMode::Abs);
    let stats = DiffStatistics::from_heatmap(&heatmap);

    // Identical frames should have zero diff
    assert_eq!(stats.mean_diff, 0.0);
    assert_eq!(stats.max_diff, 0.0);
    assert_eq!(stats.diff_area_pct, 0.0);

    // PSNR should be infinity for identical frames
    assert!(stats.psnr.is_some());
    assert!(stats.psnr.unwrap().is_infinite());
}

#[test]
fn test_diff_statistics_large_diff() {
    let luma_a = vec![255u8; 16];
    let luma_b = vec![0u8; 16];

    let heatmap = DiffHeatmapData::from_luma_planes(&luma_a, &luma_b, 4, 4, DiffMode::Abs);
    let stats = DiffStatistics::from_heatmap(&heatmap);

    // Max diff should be 255
    assert!((stats.max_diff - 255.0).abs() < 1.0);
    assert!((stats.mean_diff - 255.0).abs() < 1.0);

    // All pixels have diff > 10
    assert!((stats.diff_area_pct - 100.0).abs() < 1.0);
}

#[test]
fn test_diff_statistics_summary_text() {
    let luma_a = vec![100u8; 16];
    let luma_b = vec![50u8; 16];

    let heatmap = DiffHeatmapData::from_luma_planes(&luma_a, &luma_b, 4, 4, DiffMode::Abs);
    let stats = DiffStatistics::from_heatmap(&heatmap);

    let summary = stats.summary_text();
    assert!(summary.contains("Mean"));
    assert!(summary.contains("Max"));
    assert!(summary.contains("PSNR"));
    assert!(summary.contains("dB"));
}

#[test]
fn test_diff_statistics_empty_heatmap() {
    let stats = DiffStatistics::from_heatmap(&DiffHeatmapData {
        frame_width: 0,
        frame_height: 0,
        heatmap_width: 0,
        heatmap_height: 0,
        values: vec![],
        mode: DiffMode::Abs,
        min_value: 0.0,
        max_value: 0.0,
    });

    assert_eq!(stats.mean_diff, 0.0);
    assert_eq!(stats.max_diff, 0.0);
    assert_eq!(stats.diff_area_pct, 0.0);
}
