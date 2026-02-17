#![allow(dead_code)]
use bitvue_core::metrics_distribution::*;

fn create_test_series() -> MetricSeries {
    let mut series = MetricSeries::new(MetricType::PsnrY);
    for i in 0..10 {
        series.add_point(MetricPoint::new(i, 30.0 + i as f32));
    }
    series
}

#[test]
fn test_metric_type_info() {
    assert_eq!(MetricType::PsnrY.name(), "PSNR-Y");
    assert_eq!(MetricType::PsnrY.unit(), "dB");
    assert_eq!(MetricType::SsimY.typical_range(), (0.0, 1.0));
}

#[test]
fn test_metric_series() {
    let series = create_test_series();
    assert_eq!(series.data.len(), 10);
    assert_eq!(series.get_value(5), Some(35.0));
    assert_eq!(series.get_value(20), None);
}

#[test]
fn test_metric_series_values_for_selection() {
    let series = create_test_series();
    let selected = vec![2, 5, 8];
    let values = series.values_for_selection(&selected);

    assert_eq!(values.len(), 3);
    assert_eq!(values[0], 32.0);
    assert_eq!(values[1], 35.0);
    assert_eq!(values[2], 38.0);
}

#[test]
fn test_summary_stats() {
    let values = vec![10.0, 20.0, 30.0, 40.0, 50.0];
    let stats = SummaryStats::calculate(values).unwrap();

    assert_eq!(stats.count, 5);
    assert_eq!(stats.min, 10.0);
    assert_eq!(stats.max, 50.0);
    assert_eq!(stats.mean, 30.0);
    assert_eq!(stats.median, 30.0);
}

#[test]
fn test_summary_stats_empty() {
    let values: Vec<f32> = vec![];
    let stats = SummaryStats::calculate(values);
    assert!(stats.is_none());
}

#[test]
fn test_histogram_creation() {
    let values = vec![10.0, 15.0, 20.0, 25.0, 30.0, 35.0, 40.0, 45.0];
    let histogram = MetricsHistogram::new(MetricType::PsnrY, values, 4);

    assert_eq!(histogram.bins.len(), 4);
    assert_eq!(histogram.range, (10.0, 45.0));

    // Check total count
    let total: usize = histogram.bins.iter().map(|b| b.count).sum();
    assert_eq!(total, 8);
}

#[test]
fn test_histogram_get_bin() {
    let values = vec![10.0, 20.0, 30.0, 40.0];
    let histogram = MetricsHistogram::new(MetricType::PsnrY, values, 2);

    let bin = histogram.get_bin(15.0);
    assert!(bin.is_some());
    assert_eq!(bin.unwrap().start, 10.0);
}

#[test]
fn test_histogram_max_frequency() {
    let values = vec![10.0, 10.0, 10.0, 40.0]; // 3 in first bin, 1 in second
    let histogram = MetricsHistogram::new(MetricType::PsnrY, values, 2);

    assert_eq!(histogram.max_frequency(), 0.75); // 3/4
}

#[test]
fn test_worst_frames() {
    let series = create_test_series(); // Values: 30.0, 31.0, ..., 39.0
    let worst = WorstFrames::new(MetricType::PsnrY, &series, 3);

    assert_eq!(worst.frame_indices.len(), 3);
    assert_eq!(worst.frame_indices[0], 0); // Worst (lowest value)
    assert_eq!(worst.frame_indices[1], 1);
    assert_eq!(worst.frame_indices[2], 2);

    assert!(worst.contains(0));
    assert!(!worst.contains(9));

    assert_eq!(worst.get_rank(0), Some(0)); // Rank 0 = worst
    assert_eq!(worst.get_rank(1), Some(1));
}

#[test]
fn test_metrics_distribution_panel() {
    let mut panel = MetricsDistributionPanel::new(10);
    let series = create_test_series();
    panel.add_series(series);

    assert!(panel.series.contains_key(&MetricType::PsnrY));
}

#[test]
fn test_metrics_distribution_histogram_caching() {
    let mut panel = MetricsDistributionPanel::new(10);
    panel.add_series(create_test_series());

    let hist1 = panel.get_histogram();
    assert!(hist1.is_some());

    // Should return cached version
    let hist2 = panel.get_histogram();
    assert!(hist2.is_some());
}

#[test]
fn test_metrics_distribution_selection_aware() {
    let mut panel = MetricsDistributionPanel::new(10);
    panel.add_series(create_test_series());

    // Full stats
    let full_stats = panel.get_stats().unwrap();
    assert_eq!(full_stats.count, 10);

    // Selection stats
    panel.set_selection(vec![0, 1, 2]);
    let sel_stats = panel.get_selection_stats().unwrap();
    assert_eq!(sel_stats.count, 3);
    assert_eq!(sel_stats.min, 30.0);
    assert_eq!(sel_stats.max, 32.0);
}

#[test]
fn test_metrics_distribution_cache_invalidation() {
    let mut panel = MetricsDistributionPanel::new(10);
    panel.add_series(create_test_series());

    panel.get_histogram();
    assert!(panel.cached_histogram.is_some());

    // Change metric type -> invalidate
    panel.set_metric(MetricType::SsimY);
    assert!(panel.cached_histogram.is_none());

    // Re-populate
    panel.set_metric(MetricType::PsnrY);
    panel.get_histogram();
    assert!(panel.cached_histogram.is_some());

    // Change selection -> invalidate
    panel.set_selection(vec![0, 1]);
    assert!(panel.cached_histogram.is_none());
}

#[test]
fn test_metrics_distribution_worst_frames() {
    let mut panel = MetricsDistributionPanel::new(10);
    panel.add_series(create_test_series());

    let worst = panel.get_worst_frames(5).unwrap();
    assert_eq!(worst.frame_indices.len(), 5);
    assert_eq!(worst.frame_indices[0], 0); // Frame 0 has value 30.0 (worst)
}

// UX MetricsPanel viz_core tests - Task 13 (S.T4-1.ALL.UX.MetricsPanel.impl.viz_core.005)

#[test]
fn test_ux_metrics_panel_select_metric_type_dropdown() {
    // UX MetricsPanel: User selects metric type from dropdown
    let mut panel = MetricsDistributionPanel::new(10);

    // UX MetricsPanel: Add PSNR and SSIM series
    let mut psnr_series = MetricSeries::new(MetricType::PsnrY);
    for i in 0..20 {
        psnr_series.add_point(MetricPoint::new(i, 30.0 + i as f32 * 0.5));
    }
    panel.add_series(psnr_series);

    let mut ssim_series = MetricSeries::new(MetricType::SsimY);
    for i in 0..20 {
        ssim_series.add_point(MetricPoint::new(i, 0.9 + i as f32 * 0.005));
    }
    panel.add_series(ssim_series);

    // UX MetricsPanel: Initial metric is PSNR-Y
    assert_eq!(panel.current_metric, MetricType::PsnrY);

    // UX MetricsPanel: Get histogram for PSNR
    let hist_psnr = panel.get_histogram().unwrap();
    assert_eq!(hist_psnr.metric_type, MetricType::PsnrY);
    assert!(hist_psnr.range.0 >= 30.0);

    // UX MetricsPanel: User selects SSIM-Y from dropdown
    panel.set_metric(MetricType::SsimY);
    assert_eq!(panel.current_metric, MetricType::SsimY);

    // UX MetricsPanel: Cache is invalidated, histogram recalculates
    let hist_ssim = panel.get_histogram().unwrap();
    assert_eq!(hist_ssim.metric_type, MetricType::SsimY);
    assert!(hist_ssim.range.0 >= 0.9);
    assert!(hist_ssim.range.1 <= 1.0);

    // UX MetricsPanel: Verify metric info displayed in UI
    assert_eq!(MetricType::PsnrY.name(), "PSNR-Y");
    assert_eq!(MetricType::PsnrY.unit(), "dB");
    assert_eq!(MetricType::SsimY.name(), "SSIM-Y");
    assert_eq!(MetricType::SsimY.unit(), "");
}

#[test]
fn test_ux_metrics_panel_selection_aware_stats() {
    // UX MetricsPanel: User selects frames on timeline to see selection-aware stats
    let mut panel = MetricsDistributionPanel::new(10);

    // UX MetricsPanel: Load PSNR series for 15 frames
    let mut series = MetricSeries::new(MetricType::PsnrY);
    for i in 0..15 {
        let value = 30.0 + (i as f32 * 1.5);
        series.add_point(MetricPoint::new(i, value));
    }
    panel.add_series(series);

    // UX MetricsPanel: User views full dataset stats
    let full_mean = {
        let full_stats = panel.get_stats().unwrap();
        assert_eq!(full_stats.count, 15);
        assert_eq!(full_stats.min, 30.0);
        assert_eq!(full_stats.max, 30.0 + 14.0 * 1.5);
        full_stats.mean
    };

    // UX MetricsPanel: User drags on timeline to select frames 0-4 (low quality frames)
    panel.set_selection(vec![0, 1, 2, 3, 4]);

    // UX MetricsPanel: Selection stats panel updates
    let sel_mean = {
        let sel_stats = panel.get_selection_stats().unwrap();
        assert_eq!(sel_stats.count, 5);
        assert_eq!(sel_stats.min, 30.0); // Frame 0: 30.0
        assert_eq!(sel_stats.max, 30.0 + 4.0 * 1.5); // Frame 4: 36.0
        sel_stats.mean
    };

    // UX MetricsPanel: UI shows "Selection: 5 frames" vs "All: 15 frames"
    // Selection mean (33.0) is lower than full mean (40.5)
    assert!(sel_mean < full_mean);

    // UX MetricsPanel: User clears selection with Escape key
    panel.clear_selection();

    // UX MetricsPanel: Selection stats panel disappears
    let sel_stats_after = panel.get_selection_stats();
    assert!(sel_stats_after.is_none());

    // UX MetricsPanel: Full stats remain visible
    let full_stats_after = panel.get_stats().unwrap();
    assert_eq!(full_stats_after.count, 15);
}

#[test]
fn test_ux_metrics_panel_histogram_bin_hover() {
    // UX MetricsPanel: User hovers over histogram bin to see value range
    let mut panel = MetricsDistributionPanel::new(4); // 4 bins

    // UX MetricsPanel: Load PSNR series
    let mut series = MetricSeries::new(MetricType::PsnrY);
    for i in 0..12 {
        series.add_point(MetricPoint::new(i, 30.0 + i as f32 * 2.0));
    }
    panel.add_series(series);

    // UX MetricsPanel: User hovers over histogram
    let hist = panel.get_histogram().unwrap();
    assert_eq!(hist.bins.len(), 4);

    // UX MetricsPanel: Hover over first bin
    let bin_0 = &hist.bins[0];
    assert_eq!(bin_0.start, 30.0);
    assert!(bin_0.end > 30.0);

    // UX MetricsPanel: UI shows tooltip: "Range: 30.0 - X.X dB, Count: N, Frequency: X%"
    assert!(bin_0.count > 0);
    assert!(bin_0.frequency > 0.0);
    assert!(bin_0.frequency <= 1.0);

    // UX MetricsPanel: User hovers over bin containing value 40.0
    let bin_containing_40 = hist.get_bin(40.0).unwrap();
    assert!(40.0 >= bin_containing_40.start);
    assert!(40.0 < bin_containing_40.end);

    // UX MetricsPanel: Get max frequency for bar height scaling
    let max_freq = hist.max_frequency();
    assert!(max_freq > 0.0);
    assert!(max_freq <= 1.0);

    // UX MetricsPanel: All bins use max_freq to scale to 100% height
    for bin in &hist.bins {
        let bar_height_percent = (bin.frequency / max_freq) * 100.0;
        assert!(bar_height_percent <= 100.0);
    }
}

#[test]
fn test_ux_metrics_panel_worst_frames_list_click() {
    // UX MetricsPanel: User clicks on worst frames list to jump to frame
    let mut panel = MetricsDistributionPanel::new(10);

    // UX MetricsPanel: Load PSNR series with varying quality
    let mut series = MetricSeries::new(MetricType::PsnrY);
    let values = vec![45.0, 32.0, 48.0, 28.0, 42.0, 35.0, 50.0, 30.0, 44.0, 38.0];
    for (i, &value) in values.iter().enumerate() {
        series.add_point(MetricPoint::new(i, value));
    }
    panel.add_series(series);

    // UX MetricsPanel: User views "Worst 5 Frames" list
    let worst = panel.get_worst_frames(5).unwrap();
    assert_eq!(worst.frame_indices.len(), 5);

    // UX MetricsPanel: List shows worst frames in order
    // Sorted values: 28.0 (idx 3), 30.0 (idx 7), 32.0 (idx 1), 35.0 (idx 5), 38.0 (idx 9)
    assert_eq!(worst.frame_indices[0], 3); // Worst: 28.0 dB
    assert_eq!(worst.values[0], 28.0);

    assert_eq!(worst.frame_indices[1], 7); // 2nd worst: 30.0 dB
    assert_eq!(worst.values[1], 30.0);

    assert_eq!(worst.frame_indices[2], 1); // 3rd worst: 32.0 dB
    assert_eq!(worst.values[2], 32.0);

    // UX MetricsPanel: User clicks on frame 3 in the list
    assert!(worst.contains(3));
    assert_eq!(worst.get_rank(3), Some(0)); // Rank 0 = worst

    // UX MetricsPanel: User clicks on frame 7
    assert_eq!(worst.get_rank(7), Some(1)); // Rank 1 = 2nd worst

    // UX MetricsPanel: Frame 0 is not in worst list (has good quality 45.0 dB)
    assert!(!worst.contains(0));
    assert_eq!(worst.get_rank(0), None);

    // UX MetricsPanel: Verify UI badge colors
    // Rank 0 (worst) -> Red badge
    // Rank 1-2 -> Orange badge
    // Rank 3-4 -> Yellow badge
    for (rank, &idx) in worst.frame_indices.iter().enumerate() {
        assert_eq!(worst.get_rank(idx), Some(rank));
    }
}

#[test]
fn test_ux_metrics_panel_selection_histogram_comparison() {
    // UX MetricsPanel: User compares full vs selection histograms side by side
    let mut panel = MetricsDistributionPanel::new(5); // 5 bins

    // UX MetricsPanel: Load VMAF series
    let mut series = MetricSeries::new(MetricType::Vmaf);
    for i in 0..30 {
        // Bimodal distribution: some high, some low
        let value = if i < 15 {
            60.0 + i as f32
        } else {
            90.0 + (i - 15) as f32
        };
        series.add_point(MetricPoint::new(i, value));
    }
    panel.add_series(series);
    panel.set_metric(MetricType::Vmaf);

    // UX MetricsPanel: User views full histogram
    let full_hist = panel.get_histogram().unwrap();
    assert_eq!(full_hist.metric_type, MetricType::Vmaf);
    assert_eq!(full_hist.bins.len(), 5);

    // UX MetricsPanel: Full dataset spans 60-104 range
    let full_range = full_hist.range;
    assert_eq!(full_range.0, 60.0);
    assert_eq!(full_range.1, 104.0);

    // UX MetricsPanel: User selects only high-quality frames (15-29)
    panel.set_selection((15..30).collect());

    // UX MetricsPanel: Selection histogram shows different distribution
    let sel_hist = panel.get_selection_histogram().unwrap();
    assert_eq!(sel_hist.bins.len(), 5);

    // UX MetricsPanel: Selection range is narrower (90-104)
    let sel_range = sel_hist.range;
    assert_eq!(sel_range.0, 90.0);
    assert_eq!(sel_range.1, 104.0);

    // UX MetricsPanel: UI shows both histograms overlaid or side-by-side
    // Selection histogram should have different distribution than full
    assert_ne!(full_range, sel_range);

    // UX MetricsPanel: User can see selection covers high-quality region
    let sel_min = {
        let sel_stats = panel.get_selection_stats().unwrap();
        assert!(sel_stats.min >= 90.0);
        sel_stats.min
    };

    let full_min = {
        let full_stats = panel.get_stats().unwrap();
        full_stats.min
    };

    assert!(full_min < sel_min); // Full includes low values
}

// AV1 Metrics Panel viz_core test - Task 23 (S.T4-3.AV1.Metrics.MetricsPanel.impl.viz_core.001)

#[test]
fn test_av1_metrics_panel_visualization() {
    // AV1 Metrics: User views PSNR-Y metrics for AV1 640x360 stream
    let mut panel = MetricsDistributionPanel::new(5); // 5 histogram bins

    // AV1 Metrics: Add PSNR-Y series for AV1 stream
    let mut psnr_series = MetricSeries::new(MetricType::PsnrY);

    // Frame 0: KEY_FRAME (high PSNR due to I-frame)
    psnr_series.add_point(MetricPoint::new(0, 42.5));

    // Frames 1-9: INTER_FRAMEs (lower PSNR)
    for i in 1..10 {
        psnr_series.add_point(MetricPoint::new(i, 38.0 + (i as f32 * 0.5)));
    }

    panel.add_series(psnr_series);

    // AV1 Metrics: Verify PSNR statistics
    let stats = panel.get_stats().unwrap();
    assert_eq!(stats.count, 10);
    assert_eq!(stats.min, 38.5); // Frame 1 (lowest PSNR)
    assert_eq!(stats.max, 42.5); // Frame 0 (KEY_FRAME)
    assert!((stats.mean - 40.0).abs() < 1.5); // Average ~40 dB

    // AV1 Metrics: Verify PSNR range via MetricType
    assert_eq!(MetricType::PsnrY.name(), "PSNR-Y");
    assert_eq!(MetricType::PsnrY.unit(), "dB");

    // AV1 Metrics: Typical PSNR range for AV1
    let typical = MetricType::PsnrY.typical_range();
    assert_eq!(typical, (20.0, 50.0));
    assert!(stats.min >= typical.0);
    assert!(stats.max <= typical.1);

    // AV1 Metrics: Histogram visualization
    let hist = panel.get_histogram().unwrap();
    assert_eq!(hist.bins.len(), 5);
    assert_eq!(hist.metric_type, MetricType::PsnrY);
    assert_eq!(hist.range.0, 38.5); // Min from frame 1
    assert_eq!(hist.range.1, 42.5); // Max from frame 0

    // AV1 Metrics: Worst frames identification (bottom 3 frames)
    let worst = panel.get_worst_frames(3).unwrap();
    assert_eq!(worst.frame_indices.len(), 3);
    assert_eq!(worst.frame_indices[0], 1); // Frame 1 (lowest PSNR)
    assert_eq!(worst.metric_type, MetricType::PsnrY);

    // AV1 Metrics: User selects KEY_FRAME and nearby frames (0-2)
    panel.set_selection(vec![0, 1, 2]);

    // AV1 Metrics: Selection statistics show subset
    let sel_stats = panel.get_selection_stats().unwrap();
    assert_eq!(sel_stats.count, 3);
    assert!(sel_stats.min >= 38.0);
    assert!(sel_stats.max <= 42.5);

    // AV1 Metrics: Clear selection
    panel.clear_selection();
    assert!(panel.get_selection_stats().is_none());

    // AV1 Metrics: Add SSIM-Y series for comparison
    let mut ssim_series = MetricSeries::new(MetricType::SsimY);

    for i in 0..10 {
        ssim_series.add_point(MetricPoint::new(i, 0.90 + (i as f32 * 0.005)));
    }

    panel.add_series(ssim_series);

    // AV1 Metrics: Switch to SSIM metric
    panel.set_metric(MetricType::SsimY);

    // AV1 Metrics: Verify SSIM statistics
    let ssim_stats = panel.get_stats().unwrap();
    assert_eq!(ssim_stats.count, 10);

    // AV1 Metrics: Verify SSIM metric type properties
    assert_eq!(MetricType::SsimY.name(), "SSIM-Y");
    assert_eq!(MetricType::SsimY.unit(), "");

    // AV1 Metrics: SSIM range 0.0-1.0
    let ssim_range = MetricType::SsimY.typical_range();
    assert_eq!(ssim_range, (0.0, 1.0));
    assert!(ssim_stats.min >= ssim_range.0);
    assert!(ssim_stats.max <= ssim_range.1);
}
