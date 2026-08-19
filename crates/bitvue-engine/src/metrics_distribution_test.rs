// Metrics distribution module tests
//
// Per generate-tests skill: Arrange-Act-Assert pattern with fixtures
// and edge case coverage.

use super::*;

// ============================================================================
// Fixtures
// ============================================================================

/// Create test metric series
fn create_test_series(metric_type: MetricType, count: usize, start_value: f32, step: f32) -> MetricSeries {
    let mut series = MetricSeries::new(metric_type);
    for i in 0..count {
        series.add_point(MetricPoint::new(i, start_value + i as f32 * step));
    }
    series
}

/// Create test metric series with random-ish values
fn create_test_series_values(metric_type: MetricType, values: Vec<f32>) -> MetricSeries {
    let mut series = MetricSeries::new(metric_type);
    for (i, &value) in values.iter().enumerate() {
        series.add_point(MetricPoint::new(i, value));
    }
    series
}

// ============================================================================
// MetricType Tests
// ============================================================================

#[cfg(test)]
mod metric_type_tests {
    use super::*;

    #[test]
    fn test_metric_type_psnr_y() {
        // Arrange & Act
        let metric_type = MetricType::PsnrY;

        // Assert
        assert_eq!(metric_type.name(), "PSNR-Y");
        assert_eq!(metric_type.unit(), "dB");
        assert_eq!(metric_type.typical_range(), (20.0, 50.0));
    }

    #[test]
    fn test_metric_type_ssim_y() {
        // Arrange & Act
        let metric_type = MetricType::SsimY;

        // Assert
        assert_eq!(metric_type.name(), "SSIM-Y");
        assert_eq!(metric_type.unit(), "");
        assert_eq!(metric_type.typical_range(), (0.0, 1.0));
    }

    #[test]
    fn test_metric_type_vmaf() {
        // Arrange & Act
        let metric_type = MetricType::Vmaf;

        // Assert
        assert_eq!(metric_type.name(), "VMAF");
        assert_eq!(metric_type.unit(), "");
        assert_eq!(metric_type.typical_range(), (0.0, 100.0));
    }

    #[test]
    fn test_metric_type_custom() {
        // Arrange & Act
        let metric_type = MetricType::Custom;

        // Assert
        assert_eq!(metric_type.name(), "Custom");
        assert_eq!(metric_type.unit(), "");
        assert_eq!(metric_type.typical_range(), (0.0, 100.0));
    }
}

// ============================================================================
// MetricPoint Tests
// ============================================================================

#[cfg(test)]
mod metric_point_tests {
    use super::*;

    #[test]
    fn test_metric_point_new() {
        // Arrange & Act
        let point = MetricPoint::new(5, 35.5);

        // Assert
        assert_eq!(point.idx, 5);
        assert_eq!(point.value, 35.5);
    }
}

// ============================================================================
// MetricSeries Tests
// ============================================================================

#[cfg(test)]
mod metric_series_tests {
    use super::*;

    #[test]
    fn test_metric_series_new() {
        // Arrange & Act
        let series = MetricSeries::new(MetricType::PsnrY);

        // Assert
        assert_eq!(series.metric_type, MetricType::PsnrY);
        assert_eq!(series.data.len(), 0);
    }

    #[test]
    fn test_metric_series_add_point() {
        // Arrange
        let mut series = MetricSeries::new(MetricType::PsnrY);

        // Act
        series.add_point(MetricPoint::new(0, 35.0));

        // Assert
        assert_eq!(series.data.len(), 1);
        assert_eq!(series.data[0].idx, 0);
        assert_eq!(series.data[0].value, 35.0);
    }

    #[test]
    fn test_metric_series_get_value_existing() {
        // Arrange
        let series = create_test_series(MetricType::PsnrY, 10, 30.0, 1.0);

        // Act
        let value = series.get_value(5);

        // Assert
        assert_eq!(value, Some(35.0));
    }

    #[test]
    fn test_metric_series_get_value_not_found() {
        // Arrange
        let series = create_test_series(MetricType::PsnrY, 10, 30.0, 1.0);

        // Act
        let value = series.get_value(99);

        // Assert
        assert!(value.is_none());
    }

    #[test]
    fn test_metric_series_values() {
        // Arrange
        let series = create_test_series(MetricType::PsnrY, 5, 30.0, 2.5);

        // Act
        let values = series.values();

        // Assert
        assert_eq!(values.len(), 5);
        assert_eq!(values[0], 30.0);
        assert_eq!(values[4], 40.0);
    }

    #[test]
    fn test_metric_series_values_for_selection() {
        // Arrange
        let series = create_test_series(MetricType::PsnrY, 10, 30.0, 1.0);
        let selection = vec![0, 5, 9];

        // Act
        let values = series.values_for_selection(&selection);

        // Assert
        assert_eq!(values.len(), 3);
        assert_eq!(values[0], 30.0); // Frame 0
        assert_eq!(values[1], 35.0); // Frame 5
        assert_eq!(values[2], 39.0); // Frame 9
    }
}

// ============================================================================
// SummaryStats Tests
// ============================================================================

#[cfg(test)]
mod summary_stats_tests {
    use super::*;

    #[test]
    fn test_summary_stats_calculate_basic() {
        // Arrange
        let values = vec![30.0, 35.0, 40.0, 45.0, 50.0];

        // Act
        let stats = SummaryStats::calculate(values);

        // Assert
        assert!(stats.is_some());
        let s = stats.unwrap();
        assert_eq!(s.count, 5);
        assert_eq!(s.min, 30.0);
        assert_eq!(s.max, 50.0);
        assert_eq!(s.mean, 40.0);
        assert!((s.std_dev - 7.071).abs() < 0.01); // ~7.07
        assert_eq!(s.median, 40.0);
    }

    #[test]
    fn test_summary_stats_calculate_empty() {
        // Arrange
        let values: Vec<f32> = vec![];

        // Act
        let stats = SummaryStats::calculate(values);

        // Assert
        assert!(stats.is_none());
    }

    #[test]
    fn test_summary_stats_calculate_single() {
        // Arrange
        let values = vec![42.0];

        // Act
        let stats = SummaryStats::calculate(values);

        // Assert
        assert!(stats.is_some());
        let s = stats.unwrap();
        assert_eq!(s.count, 1);
        assert_eq!(s.min, 42.0);
        assert_eq!(s.max, 42.0);
        assert_eq!(s.mean, 42.0);
        assert_eq!(s.std_dev, 0.0);
    }

    #[test]
    fn test_summary_stats_percentiles() {
        // Arrange - Values 0-99
        let values: Vec<f32> = (0..100).map(|i| i as f32).collect();

        // Act
        let stats = SummaryStats::calculate(values);

        // Assert
        let s = stats.unwrap();
        assert_eq!(s.p5, 5.0);  // 5th percentile
        assert_eq!(s.p95, 94.0); // 95th percentile (note: index calculation)
    }
}

// ============================================================================
// MetricsHistogram Tests
// ============================================================================

#[cfg(test)]
mod metrics_histogram_tests {
    use super::*;

    #[test]
    fn test_metrics_histogram_new() {
        // Arrange
        let values = vec![30.0, 35.0, 40.0, 45.0, 50.0];

        // Act
        let hist = MetricsHistogram::new(MetricType::PsnrY, values, 5);

        // Assert
        assert_eq!(hist.metric_type, MetricType::PsnrY);
        assert_eq!(hist.bin_count, 5);
        assert_eq!(hist.bins.len(), 5);
        assert_eq!(hist.range, (30.0, 50.0));
    }

    #[test]
    fn test_metrics_histogram_empty() {
        // Arrange
        let values: Vec<f32> = vec![];

        // Act
        let hist = MetricsHistogram::new(MetricType::PsnrY, values, 5);

        // Assert
        assert_eq!(hist.bins.len(), 0);
        assert_eq!(hist.range, (0.0, 0.0));
    }

    #[test]
    fn test_metrics_histogram_get_bin() {
        // Arrange
        let values = vec![30.0, 35.0, 40.0, 45.0, 50.0];
        let hist = MetricsHistogram::new(MetricType::PsnrY, values, 5);

        // Act
        let bin = hist.get_bin(37.5);

        // Assert
        assert!(bin.is_some());
        let b = bin.unwrap();
        assert!(b.start <= 37.5 && 37.5 < b.end);
    }

    #[test]
    fn test_metrics_histogram_get_bin_out_of_range() {
        // Arrange
        let values = vec![30.0, 35.0, 40.0];
        let hist = MetricsHistogram::new(MetricType::PsnrY, values, 5);

        // Act
        let bin = hist.get_bin(100.0);

        // Assert
        assert!(bin.is_none());
    }

    #[test]
    fn test_metrics_histogram_max_frequency() {
        // Arrange
        let values = vec![30.0, 30.0, 35.0, 40.0]; // 30.0 appears twice
        let hist = MetricsHistogram::new(MetricType::PsnrY, values, 3);

        // Act
        let max_freq = hist.max_frequency();

        // Assert
        assert_eq!(max_freq, 0.5); // 2/4 = 0.5
    }
}

// ============================================================================
// WorstFrames Tests
// ============================================================================

#[cfg(test)]
mod worst_frames_tests {
    use super::*;

    #[test]
    fn test_worst_frames_new() {
        // Arrange
        let series = create_test_series_values(MetricType::PsnrY, vec![40.0, 35.0, 30.0, 45.0, 25.0]);

        // Act
        let worst = WorstFrames::new(MetricType::PsnrY, &series, 3);

        // Assert
        assert_eq!(worst.metric_type, MetricType::PsnrY);
        assert_eq!(worst.max_count, 3);
        assert_eq!(worst.frame_indices.len(), 3);
        assert_eq!(worst.values.len(), 3);
        // Should be sorted by value (worst first)
        assert_eq!(worst.values[0], 25.0); // Worst
        assert_eq!(worst.values[1], 30.0);
        assert_eq!(worst.values[2], 35.0);
    }

    #[test]
    fn test_worst_frames_contains() {
        // Arrange
        let series = create_test_series_values(MetricType::PsnrY, vec![40.0, 35.0, 30.0]);
        let worst = WorstFrames::new(MetricType::PsnrY, &series, 10);

        // Act
        let contains = worst.contains(1); // Frame 1 has value 35.0

        // Assert
        assert!(contains);
    }

    #[test]
    fn test_worst_frames_contains_not_found() {
        // Arrange
        let series = create_test_series(MetricType::PsnrY, 10, 30.0, 1.0);
        let worst = WorstFrames::new(MetricType::PsnrY, &series, 3);

        // Act
        let contains = worst.contains(99);

        // Assert
        assert!(!contains);
    }

    #[test]
    fn test_worst_frames_get_rank() {
        // Arrange
        let series = create_test_series_values(MetricType::PsnrY, vec![40.0, 35.0, 30.0, 25.0]);
        let worst = WorstFrames::new(MetricType::PsnrY, &series, 10);

        // Act
        let rank = worst.get_rank(2); // Frame 2 has value 30.0

        // Assert
        assert_eq!(rank, Some(1)); // Second worst (0-indexed), after 25.0
    }

    #[test]
    fn test_worst_frames_get_rank_not_in_list() {
        // Arrange
        let series = create_test_series(MetricType::PsnrY, 10, 30.0, 1.0);
        let worst = WorstFrames::new(MetricType::PsnrY, &series, 3);

        // Act
        let rank = worst.get_rank(9); // Not in worst 3

        // Assert
        assert!(rank.is_none());
    }
}

// ============================================================================
// MetricsDistributionPanel Tests
// ============================================================================

#[cfg(test)]
mod metrics_distribution_panel_tests {
    use super::*;

    #[test]
    fn test_metrics_distribution_panel_new() {
        // Arrange & Act
        let panel = MetricsDistributionPanel::new(10);

        // Assert
        assert_eq!(panel.current_metric, MetricType::PsnrY);
        assert_eq!(panel.bin_count, 10);
        assert_eq!(panel.series.len(), 0);
        assert!(panel.selected_frames.is_empty());
        assert!(panel.cached_histogram.is_none());
    }

    #[test]
    fn test_metrics_distribution_panel_add_series() {
        // Arrange
        let mut panel = MetricsDistributionPanel::new(10);
        let series = create_test_series(MetricType::PsnrY, 10, 30.0, 1.0);

        // Act
        panel.add_series(series);

        // Assert
        assert_eq!(panel.series.len(), 1);
        assert!(panel.series.contains_key(&MetricType::PsnrY));
    }

    #[test]
    fn test_metrics_distribution_panel_set_metric() {
        // Arrange
        let mut panel = MetricsDistributionPanel::new(10);
        let series = create_test_series(MetricType::SsimY, 10, 0.8, 0.01);
        panel.add_series(series);

        // Act
        panel.set_metric(MetricType::SsimY);

        // Assert
        assert_eq!(panel.current_metric, MetricType::SsimY);
    }

    #[test]
    fn test_metrics_distribution_panel_set_selection() {
        // Arrange
        let mut panel = MetricsDistributionPanel::new(10);
        let series = create_test_series(MetricType::PsnrY, 10, 30.0, 1.0);
        panel.add_series(series);
        let selection = vec![0, 1, 2];

        // Act
        panel.set_selection(selection.clone());

        // Assert
        assert_eq!(panel.selected_frames, selection);
    }

    #[test]
    fn test_metrics_distribution_panel_clear_selection() {
        // Arrange
        let mut panel = MetricsDistributionPanel::new(10);
        panel.set_selection(vec![0, 1, 2]);

        // Act
        panel.clear_selection();

        // Assert
        assert!(panel.selected_frames.is_empty());
    }

    #[test]
    fn test_metrics_distribution_panel_get_histogram() {
        // Arrange
        let mut panel = MetricsDistributionPanel::new(5);
        let series = create_test_series(MetricType::PsnrY, 10, 30.0, 1.0);
        panel.add_series(series);

        // Act
        let hist = panel.get_histogram();

        // Assert
        assert!(hist.is_some());
        let h = hist.unwrap();
        assert_eq!(h.bin_count, 5);
        assert_eq!(h.bins.len(), 5);
    }

    #[test]
    fn test_metrics_distribution_panel_get_histogram_cached() {
        // Arrange
        let mut panel = MetricsDistributionPanel::new(5);
        let series = create_test_series(MetricType::PsnrY, 10, 30.0, 1.0);
        panel.add_series(series);

        // Act - Get histogram (first call computes, subsequent calls use cache)
        let hist1 = panel.get_histogram();
        // Drop the first borrow
        let _ = hist1;
        let hist2 = panel.get_histogram();

        // Assert
        assert!(hist2.is_some());
        // Should have cached histogram
        assert_eq!(hist2.unwrap().bins.len(), 5);
    }

    #[test]
    fn test_metrics_distribution_panel_get_stats() {
        // Arrange
        let mut panel = MetricsDistributionPanel::new(5);
        let series = create_test_series(MetricType::PsnrY, 10, 30.0, 1.0);
        panel.add_series(series);

        // Act
        let stats = panel.get_stats();

        // Assert
        assert!(stats.is_some());
        let s = stats.unwrap();
        assert_eq!(s.count, 10);
    }

    #[test]
    fn test_metrics_distribution_panel_get_worst_frames() {
        // Arrange
        let mut panel = MetricsDistributionPanel::new(5);
        let series = create_test_series(MetricType::PsnrY, 10, 30.0, 1.0);
        panel.add_series(series);

        // Act
        let worst = panel.get_worst_frames(3);

        // Assert
        assert!(worst.is_some());
        let w = worst.unwrap();
        assert_eq!(w.max_count, 3);
    }
}

// ============================================================================
// FrameMetricResult Tests
// ============================================================================

#[cfg(test)]
mod frame_metric_result_tests {
    use super::*;

    #[test]
    fn test_frame_metric_result_new() {
        // Arrange & Act
        let result = FrameMetricResult::new(5);

        // Assert
        assert_eq!(result.display_idx, 5);
        assert!(result.psnr_y.is_none());
        assert!(result.ssim_y.is_none());
        assert!(result.vmaf.is_none());
        assert_eq!(result.calc_time_us, 0);
    }

    #[test]
    fn test_frame_metric_result_get_psnr() {
        // Arrange
        let mut result = FrameMetricResult::new(0);
        result.psnr_y = Some(35.5);

        // Act
        let value = result.get(MetricType::PsnrY);

        // Assert
        assert_eq!(value, Some(35.5));
    }

    #[test]
    fn test_frame_metric_result_get_ssim() {
        // Arrange
        let mut result = FrameMetricResult::new(0);
        result.ssim_y = Some(0.95);

        // Act
        let value = result.get(MetricType::SsimY);

        // Assert
        assert_eq!(value, Some(0.95));
    }

    #[test]
    fn test_frame_metric_result_get_vmaf() {
        // Arrange
        let mut result = FrameMetricResult::new(0);
        result.vmaf = Some(85.0);

        // Act
        let value = result.get(MetricType::Vmaf);

        // Assert
        assert_eq!(value, Some(85.0));
    }

    #[test]
    fn test_frame_metric_result_get_custom() {
        // Arrange
        let result = FrameMetricResult::new(0);

        // Act
        let value = result.get(MetricType::Custom);

        // Assert
        assert!(value.is_none());
    }
}

// ============================================================================
// ReferenceComparisonContext Tests
// ============================================================================

#[cfg(test)]
mod reference_comparison_context_tests {
    use super::*;

    #[test]
    fn test_reference_comparison_context_new() {
        // Arrange & Act
        let context = ReferenceComparisonContext::new(
            "ref.yuv".to_string(),
            "dist.yuv".to_string(),
            1920,
            1080,
            8,
            "420".to_string(),
            100,
        );

        // Assert
        assert_eq!(context.reference_path, "ref.yuv");
        assert_eq!(context.distorted_path, "dist.yuv");
        assert_eq!(context.width, 1920);
        assert_eq!(context.height, 1080);
        assert_eq!(context.bit_depth, 8);
        assert_eq!(context.chroma, "420");
        assert_eq!(context.total_frames, 100);
        assert_eq!(context.frame_metrics.len(), 0);
    }

    #[test]
    fn test_reference_comparison_context_add_frame_result() {
        // Arrange
        let mut context = ReferenceComparisonContext::new(
            "ref.yuv".to_string(),
            "dist.yuv".to_string(),
            1920,
            1080,
            8,
            "420".to_string(),
            10,
        );

        // Act
        let result = FrameMetricResult::new(0);
        context.add_frame_result(result);

        // Assert
        assert_eq!(context.frame_metrics.len(), 1);
    }

    #[test]
    fn test_reference_comparison_context_progress() {
        // Arrange
        let mut context = ReferenceComparisonContext::new(
            "ref.yuv".to_string(),
            "dist.yuv".to_string(),
            1920,
            1080,
            8,
            "420".to_string(),
            100,
        );

        // Add 50 frames
        for i in 0..50 {
            context.add_frame_result(FrameMetricResult::new(i));
        }

        // Act
        let progress = context.progress();

        // Assert
        assert_eq!(progress, 0.5);
    }

    #[test]
    fn test_reference_comparison_context_is_complete() {
        // Arrange
        let mut context = ReferenceComparisonContext::new(
            "ref.yuv".to_string(),
            "dist.yuv".to_string(),
            1920,
            1080,
            8,
            "420".to_string(),
            10,
        );

        // Act - Add 9 frames
        for i in 0..9 {
            context.add_frame_result(FrameMetricResult::new(i));
        }

        // Assert
        assert!(!context.is_complete());

        // Act - Add 10th frame
        context.add_frame_result(FrameMetricResult::new(9));

        // Assert
        assert!(context.is_complete());
    }

    #[test]
    fn test_reference_comparison_context_to_series() {
        // Arrange
        let mut context = ReferenceComparisonContext::new(
            "ref.yuv".to_string(),
            "dist.yuv".to_string(),
            1920,
            1080,
            8,
            "420".to_string(),
            5,
        );

        // Add some results
        for i in 0..5 {
            let mut result = FrameMetricResult::new(i);
            result.psnr_y = Some(30.0 + i as f32);
            context.add_frame_result(result);
        }

        // Act
        let series = context.to_series(MetricType::PsnrY);

        // Assert
        assert_eq!(series.metric_type, MetricType::PsnrY);
        assert_eq!(series.data.len(), 5);
        assert_eq!(series.data[0].value, 30.0);
        assert_eq!(series.data[4].value, 34.0);
    }
}

// ============================================================================
// MetricCalculator Tests
// ============================================================================

#[cfg(test)]
mod metric_calculator_tests {
    use super::*;

    #[test]
    fn test_metric_calculator_new() {
        // Arrange & Act
        let calc = MetricCalculator::new(8);

        // Assert
        assert_eq!(calc.bit_depth, 8);
        assert_eq!(calc.max_value, 255.0);
    }

    #[test]
    fn test_metric_calculator_default() {
        // Arrange & Act
        let calc = MetricCalculator::default();

        // Assert
        assert_eq!(calc.bit_depth, 8);
        assert_eq!(calc.max_value, 255.0);
    }

    #[test]
    fn test_metric_calculator_calculate_psnr_y_perfect() {
        // Arrange
        let calc = MetricCalculator::new(8);
        let reference = vec![128u8; 100];
        let distorted = vec![128u8; 100];

        // Act
        let psnr = calc.calculate_psnr_y(&reference, &distorted);

        // Assert
        assert_eq!(psnr, Some(100.0)); // Perfect match
    }

    #[test]
    fn test_metric_calculator_calculate_psnr_y_different() {
        // Arrange
        let calc = MetricCalculator::new(8);
        let reference = vec![128u8; 100];
        let distorted = vec![130u8; 100]; // 2 units different

        // Act
        let psnr = calc.calculate_psnr_y(&reference, &distorted);

        // Assert
        assert!(psnr.is_some());
        let value = psnr.unwrap();
        assert!(value < 100.0); // Not perfect
        assert!(value > 40.0); // But decent
    }

    #[test]
    fn test_metric_calculator_calculate_psnr_y_mismatched_length() {
        // Arrange
        let calc = MetricCalculator::new(8);
        let reference = vec![128u8; 100];
        let distorted = vec![130u8; 50]; // Different length

        // Act
        let psnr = calc.calculate_psnr_y(&reference, &distorted);

        // Assert
        assert!(psnr.is_none());
    }

    #[test]
    fn test_metric_calculator_calculate_psnr_y_empty() {
        // Arrange
        let calc = MetricCalculator::new(8);
        let reference = vec![];
        let distorted = vec![];

        // Act
        let psnr = calc.calculate_psnr_y(&reference, &distorted);

        // Assert
        assert!(psnr.is_none());
    }

    #[test]
    fn test_metric_calculator_calculate_mse() {
        // Arrange
        let calc = MetricCalculator::new(8);
        let reference = vec![100u8, 150u8, 200u8];
        let distorted = vec![110u8, 145u8, 210u8];

        // Act
        let mse = calc.calculate_mse(&reference, &distorted);

        // Assert
        // MSE = ((100-110)^2 + (150-145)^2 + (200-210)^2) / 3
        // = (100 + 25 + 100) / 3 = 225 / 3 = 75
        assert!((mse - 75.0).abs() < 0.01);
    }

    #[test]
    fn test_metric_calculator_calculate_mse_empty() {
        // Arrange
        let calc = MetricCalculator::new(8);
        let reference = vec![];
        let distorted = vec![];

        // Act
        let mse = calc.calculate_mse(&reference, &distorted);

        // Assert
        assert_eq!(mse, 0.0);
    }

    #[test]
    fn test_metric_calculator_calculate_ssim_y_perfect() {
        // Arrange
        let calc = MetricCalculator::new(8);
        let reference = vec![128u8; 100];
        let distorted = vec![128u8; 100];

        // Act
        let ssim = calc.calculate_ssim_y(&reference, &distorted);

        // Assert
        assert!(ssim.is_some());
        let value = ssim.unwrap();
        assert!(value > 0.99); // Near-perfect match
    }

    #[test]
    fn test_metric_calculator_calculate_frame() {
        // Arrange
        let calc = MetricCalculator::new(8);
        let reference = vec![128u8; 100];
        let distorted = vec![130u8; 100];

        // Act
        let result = calc.calculate_frame(5, &reference, &distorted);

        // Assert
        assert_eq!(result.display_idx, 5);
        assert!(result.psnr_y.is_some());
        assert!(result.ssim_y.is_some());
        assert!(result.vmaf.is_none());
        assert!(result.calc_time_us > 0); // Should take some time
    }
}

// ============================================================================
// MetricSeriesLane Tests
// ============================================================================

#[cfg(test)]
mod metric_series_lane_tests {
    use super::*;

    #[test]
    fn test_metric_series_lane_from_series() {
        // Arrange
        let series = create_test_series_values(MetricType::PsnrY, vec![30.0, 35.0, 40.0, 45.0, 50.0]);

        // Act
        let lane = MetricSeriesLane::from_series(&series, 35.0);

        // Assert
        assert_eq!(lane.metric_type, MetricType::PsnrY);
        assert_eq!(lane.warning_threshold, 35.0);
        assert_eq!(lane.points.len(), 5);
        // Check warning frames (below 35)
        let warning_frames = lane.warning_frames();
        assert_eq!(warning_frames, vec![0]); // Only frame 0 has value 30.0 < 35
    }

    #[test]
    fn test_metric_series_lane_normalized_y() {
        // Arrange
        let series = create_test_series(MetricType::PsnrY, 5, 20.0, 10.0);
        let lane = MetricSeriesLane::from_series(&series, 30.0);

        // Act
        let norm = lane.normalized_y(20.0);

        // Assert
        assert!((norm - 0.0).abs() < 0.01); // Min value
    }

    #[test]
    fn test_metric_series_lane_normalized_y_clamped() {
        // Arrange
        let series = create_test_series(MetricType::PsnrY, 5, 20.0, 10.0);
        let lane = MetricSeriesLane::from_series(&series, 30.0);

        // Act
        let norm_max = lane.normalized_y(60.0); // Above range

        // Assert
        assert_eq!(norm_max, 1.0); // Clamped to max
    }

    #[test]
    fn test_metric_series_lane_get_point() {
        // Arrange
        let series = create_test_series(MetricType::PsnrY, 10, 30.0, 1.0);
        let lane = MetricSeriesLane::from_series(&series, 30.0);

        // Act
        let point = lane.get_point(5);

        // Assert
        assert!(point.is_some());
        let p = point.unwrap();
        assert_eq!(p.display_idx, 5);
        assert_eq!(p.value, 35.0);
    }
}

// ============================================================================
// MetricComparisonSummary Tests
// ============================================================================

#[cfg(test)]
mod metric_comparison_summary_tests {
    use super::*;

    #[test]
    fn test_metric_comparison_summary_compare() {
        // Arrange
        let series_a = create_test_series_values(MetricType::PsnrY, vec![40.0, 35.0, 30.0]);
        let series_b = create_test_series_values(MetricType::PsnrY, vec![38.0, 37.0, 32.0]);

        // Act
        let summary = MetricComparisonSummary::compare(MetricType::PsnrY, &series_a, &series_b, 0.5);

        // Assert
        assert_eq!(summary.metric_type, MetricType::PsnrY);
        assert_eq!(summary.avg_a, 35.0);
        assert_eq!(summary.avg_b, 35.666_668); // (38+37+32)/3
        assert_eq!(summary.total_frames, 3);
    }

    #[test]
    fn test_metric_comparison_summary_format_display() {
        // Arrange
        let series_a = create_test_series_values(MetricType::PsnrY, vec![40.0, 35.0]);
        let series_b = create_test_series_values(MetricType::PsnrY, vec![38.0, 37.0]);
        let summary = MetricComparisonSummary::compare(MetricType::PsnrY, &series_a, &series_b, 0.5);

        // Act
        let display = summary.format_display();

        // Assert
        assert!(display.contains("PSNR-Y"));
        assert!(display.contains("dB"));
        assert!(display.contains("A better"));
        assert!(display.contains("B better"));
    }
}

// ============================================================================
// Edge Cases
// ============================================================================

#[cfg(test)]
mod edge_case_tests {
    use super::*;

    #[test]
    fn test_metric_calculator_16bit() {
        // Arrange & Act
        let calc = MetricCalculator::new(16);

        // Assert
        assert_eq!(calc.bit_depth, 16);
        assert_eq!(calc.max_value, 65535.0);
    }

    #[test]
    fn test_metric_calculate_psnr_extreme_difference() {
        // Arrange
        let calc = MetricCalculator::new(8);
        let reference = vec![0u8; 10];
        let distorted = vec![255u8; 10];

        // Act
        let psnr = calc.calculate_psnr_y(&reference, &distorted);

        // Assert
        assert!(psnr.is_some());
        let value = psnr.unwrap();
        assert!(value < 10.0); // Very low PSNR
    }

    #[test]
    fn test_summary_stats_large_values() {
        // Arrange
        let values = vec![1000.0, 2000.0, 3000.0];

        // Act
        let stats = SummaryStats::calculate(values);

        // Assert
        assert!(stats.is_some());
        let s = stats.unwrap();
        assert_eq!(s.mean, 2000.0);
    }

    #[test]
    fn test_histogram_single_value() {
        // Arrange
        let values = vec![42.0];

        // Act
        let hist = MetricsHistogram::new(MetricType::PsnrY, values, 5);

        // Assert
        // All bins should have the single value
        let total_count: usize = hist.bins.iter().map(|b| b.count).sum();
        assert_eq!(total_count, 1);
    }

    #[test]
    fn test_histogram_with_nan_range() {
        // Arrange
        let values = vec![30.0, 35.0, 40.0];
        let hist = MetricsHistogram::new(MetricType::PsnrY, values, 3);

        // Act - Try to get bin for value outside range
        let bin = hist.get_bin(100.0);

        // Assert
        assert!(bin.is_none());
    }

    #[test]
    fn test_series_lane_empty_series() {
        // Arrange
        let series = MetricSeries::new(MetricType::PsnrY);

        // Act
        let lane = MetricSeriesLane::from_series(&series, 30.0);

        // Assert
        assert_eq!(lane.points.len(), 0);
        assert_eq!(lane.warning_frames().len(), 0);
    }

    #[test]
    fn test_worst_frames_empty_series() {
        // Arrange
        let series = MetricSeries::new(MetricType::PsnrY);

        // Act
        let worst = WorstFrames::new(MetricType::PsnrY, &series, 10);

        // Assert
        assert_eq!(worst.frame_indices.len(), 0);
        assert_eq!(worst.values.len(), 0);
    }

    #[test]
    fn test_comparison_summary_empty_series() {
        // Arrange
        let series_a = MetricSeries::new(MetricType::PsnrY);
        let series_b = MetricSeries::new(MetricType::PsnrY);

        // Act
        let summary = MetricComparisonSummary::compare(MetricType::PsnrY, &series_a, &series_b, 0.5);

        // Assert
        assert_eq!(summary.avg_a, 0.0);
        assert_eq!(summary.avg_b, 0.0);
        assert_eq!(summary.total_frames, 0);
    }
}
