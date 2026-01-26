//! Metrics Distribution Panel - T5-2
//!
//! Per CACHE_INVALIDATION_TABLE and PERFORMANCE_DEGRADATION_RULES:
//! - Histogram visualization for metrics (PSNR, SSIM, etc.)
//! - Summary statistics with selection-aware recalculation
//! - Worst frames identification
//! - Caching for performance

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Metric type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MetricType {
    /// PSNR Y (Peak Signal-to-Noise Ratio for luma)
    PsnrY,
    /// SSIM Y (Structural Similarity Index for luma)
    SsimY,
    /// VMAF (Video Multi-Method Assessment Fusion)
    Vmaf,
    /// Custom metric
    Custom,
}

impl MetricType {
    pub fn name(&self) -> &'static str {
        match self {
            MetricType::PsnrY => "PSNR-Y",
            MetricType::SsimY => "SSIM-Y",
            MetricType::Vmaf => "VMAF",
            MetricType::Custom => "Custom",
        }
    }

    pub fn unit(&self) -> &'static str {
        match self {
            MetricType::PsnrY => "dB",
            MetricType::SsimY => "",
            MetricType::Vmaf => "",
            MetricType::Custom => "",
        }
    }

    /// Get typical range for this metric type
    pub fn typical_range(&self) -> (f32, f32) {
        match self {
            MetricType::PsnrY => (20.0, 50.0),
            MetricType::SsimY => (0.0, 1.0),
            MetricType::Vmaf => (0.0, 100.0),
            MetricType::Custom => (0.0, 100.0),
        }
    }
}

/// Metric data point
///
/// Per mock_data/metrics_series.json: { idx, value }
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MetricPoint {
    /// Frame display_idx
    pub idx: usize,
    /// Metric value
    pub value: f32,
}

impl MetricPoint {
    pub fn new(idx: usize, value: f32) -> Self {
        Self { idx, value }
    }
}

/// Metric series (time series data)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricSeries {
    /// Metric type
    pub metric_type: MetricType,
    /// Data points (indexed by frame display_idx)
    pub data: Vec<MetricPoint>,
}

impl MetricSeries {
    pub fn new(metric_type: MetricType) -> Self {
        Self {
            metric_type,
            data: Vec::new(),
        }
    }

    /// Add a data point
    pub fn add_point(&mut self, point: MetricPoint) {
        self.data.push(point);
    }

    /// Get value at specific frame
    pub fn get_value(&self, idx: usize) -> Option<f32> {
        self.data.iter().find(|p| p.idx == idx).map(|p| p.value)
    }

    /// Get all values (for histogram calculation)
    pub fn values(&self) -> Vec<f32> {
        self.data.iter().map(|p| p.value).collect()
    }

    /// Get values for selected frames only
    pub fn values_for_selection(&self, selected_indices: &[usize]) -> Vec<f32> {
        self.data
            .iter()
            .filter(|p| selected_indices.contains(&p.idx))
            .map(|p| p.value)
            .collect()
    }
}

/// Summary statistics
///
/// Per T5-2 deliverable: "Summary stats with selection-aware recalculation"
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SummaryStats {
    /// Number of samples
    pub count: usize,
    /// Minimum value
    pub min: f32,
    /// Maximum value
    pub max: f32,
    /// Mean (average)
    pub mean: f32,
    /// Standard deviation
    pub std_dev: f32,
    /// Median (50th percentile)
    pub median: f32,
    /// 5th percentile
    pub p5: f32,
    /// 95th percentile
    pub p95: f32,
}

impl SummaryStats {
    /// Calculate summary statistics from values
    pub fn calculate(mut values: Vec<f32>) -> Option<Self> {
        if values.is_empty() {
            return None;
        }

        let count = values.len();

        // Sort for percentiles
        values.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let min = values[0];
        let max = values[count - 1];

        // Mean
        let sum: f32 = values.iter().sum();
        let mean = sum / count as f32;

        // Standard deviation
        let variance: f32 = values.iter().map(|v| (v - mean).powi(2)).sum::<f32>() / count as f32;
        let std_dev = variance.sqrt();

        // Percentiles
        let median = percentile(&values, 50.0);
        let p5 = percentile(&values, 5.0);
        let p95 = percentile(&values, 95.0);

        Some(Self {
            count,
            min,
            max,
            mean,
            std_dev,
            median,
            p5,
            p95,
        })
    }
}

/// Calculate percentile from sorted values
fn percentile(sorted_values: &[f32], p: f32) -> f32 {
    let n = sorted_values.len();
    if n == 0 {
        return 0.0;
    }

    let index = (p / 100.0 * (n - 1) as f32).round() as usize;
    sorted_values[index.min(n - 1)]
}

/// Histogram bin
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct HistogramBin {
    /// Bin start value (inclusive)
    pub start: f32,
    /// Bin end value (exclusive)
    pub end: f32,
    /// Count of values in this bin
    pub count: usize,
    /// Frequency (count / total)
    pub frequency: f32,
}

/// Metrics Histogram
///
/// Per T5-2 deliverable: "MetricsHistogram"
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsHistogram {
    /// Metric type
    pub metric_type: MetricType,
    /// Histogram bins
    pub bins: Vec<HistogramBin>,
    /// Number of bins
    pub bin_count: usize,
    /// Value range (min, max)
    pub range: (f32, f32),
}

impl MetricsHistogram {
    /// Create histogram from values
    pub fn new(metric_type: MetricType, values: Vec<f32>, bin_count: usize) -> Self {
        if values.is_empty() {
            return Self {
                metric_type,
                bins: Vec::new(),
                bin_count,
                range: (0.0, 0.0),
            };
        }

        let min = values.iter().copied().fold(f32::INFINITY, f32::min);
        let max = values.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        let range = (min, max);

        let bin_width = (max - min) / bin_count as f32;
        let total_count = values.len();

        let mut bins = Vec::new();
        for i in 0..bin_count {
            let start = min + i as f32 * bin_width;
            let end = if i == bin_count - 1 {
                max + 0.001 // Include max value
            } else {
                min + (i + 1) as f32 * bin_width
            };

            let count = values.iter().filter(|&&v| v >= start && v < end).count();
            let frequency = count as f32 / total_count as f32;

            bins.push(HistogramBin {
                start,
                end,
                count,
                frequency,
            });
        }

        Self {
            metric_type,
            bins,
            bin_count,
            range,
        }
    }

    /// Get bin containing a specific value
    pub fn get_bin(&self, value: f32) -> Option<&HistogramBin> {
        self.bins.iter().find(|b| value >= b.start && value < b.end)
    }

    /// Get max frequency (for scaling visualization)
    pub fn max_frequency(&self) -> f32 {
        self.bins.iter().map(|b| b.frequency).fold(0.0, f32::max)
    }
}

/// Worst frames list
///
/// Per T5-2 deliverable: "Worst frames list"
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorstFrames {
    /// Metric type
    pub metric_type: MetricType,
    /// Frame indices sorted by metric value (worst first)
    pub frame_indices: Vec<usize>,
    /// Corresponding metric values
    pub values: Vec<f32>,
    /// Maximum number of frames to track
    pub max_count: usize,
}

impl WorstFrames {
    /// Create worst frames list from metric series
    ///
    /// For PSNR/SSIM/VMAF: lower is worse
    pub fn new(metric_type: MetricType, series: &MetricSeries, max_count: usize) -> Self {
        let mut indexed: Vec<(usize, f32)> = series.data.iter().map(|p| (p.idx, p.value)).collect();

        // Sort by value ascending (worst first for quality metrics)
        indexed.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

        let frame_indices: Vec<usize> = indexed.iter().take(max_count).map(|(i, _)| *i).collect();
        let values: Vec<f32> = indexed.iter().take(max_count).map(|(_, v)| *v).collect();

        Self {
            metric_type,
            frame_indices,
            values,
            max_count,
        }
    }

    /// Check if a frame is in the worst frames list
    pub fn contains(&self, idx: usize) -> bool {
        self.frame_indices.contains(&idx)
    }

    /// Get rank of a frame (0 = worst, 1 = second worst, etc.)
    pub fn get_rank(&self, idx: usize) -> Option<usize> {
        self.frame_indices.iter().position(|&i| i == idx)
    }
}

/// Metrics distribution cache key
///
/// Per CACHE_INVALIDATION_TABLE: Cache invalidates on selection change
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MetricsCacheKey {
    pub metric_type_name: String,
    pub bin_count: usize,
    pub selection_hash: u64, // Hash of selected frame indices
}

impl MetricsCacheKey {
    pub fn new(metric_type: MetricType, bin_count: usize, selection: &[usize]) -> Self {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        selection.hash(&mut hasher);
        let selection_hash = hasher.finish();

        Self {
            metric_type_name: metric_type.name().to_string(),
            bin_count,
            selection_hash,
        }
    }
}

/// Metrics Distribution Panel
///
/// Per T5-2 deliverable: "Selection-aware stats" with caching
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsDistributionPanel {
    /// Metric series data
    pub series: HashMap<MetricType, MetricSeries>,

    /// Current metric being displayed
    pub current_metric: MetricType,

    /// Number of histogram bins
    pub bin_count: usize,

    /// Selected frame indices (for selection-aware stats)
    pub selected_frames: Vec<usize>,

    /// Cached histogram (full dataset)
    pub cached_histogram: Option<MetricsHistogram>,

    /// Cached histogram (selection only)
    pub cached_selection_histogram: Option<MetricsHistogram>,

    /// Cached summary stats (full dataset)
    pub cached_stats: Option<SummaryStats>,

    /// Cached summary stats (selection only)
    pub cached_selection_stats: Option<SummaryStats>,

    /// Cached worst frames
    pub cached_worst_frames: Option<WorstFrames>,

    /// Cache key for invalidation
    pub cache_key: Option<MetricsCacheKey>,
}

impl MetricsDistributionPanel {
    /// Create a new metrics distribution panel
    pub fn new(bin_count: usize) -> Self {
        Self {
            series: HashMap::new(),
            current_metric: MetricType::PsnrY,
            bin_count,
            selected_frames: Vec::new(),
            cached_histogram: None,
            cached_selection_histogram: None,
            cached_stats: None,
            cached_selection_stats: None,
            cached_worst_frames: None,
            cache_key: None,
        }
    }

    /// Add a metric series
    pub fn add_series(&mut self, series: MetricSeries) {
        self.series.insert(series.metric_type, series);
        self.invalidate_cache();
    }

    /// Set current metric
    pub fn set_metric(&mut self, metric_type: MetricType) {
        if self.current_metric != metric_type {
            self.current_metric = metric_type;
            self.invalidate_cache();
        }
    }

    /// Set selected frames
    ///
    /// Per T5-2 deliverable: "Selection-aware stats"
    pub fn set_selection(&mut self, indices: Vec<usize>) {
        if self.selected_frames != indices {
            self.selected_frames = indices;
            self.invalidate_cache();
        }
    }

    /// Clear selection
    pub fn clear_selection(&mut self) {
        if !self.selected_frames.is_empty() {
            self.selected_frames.clear();
            self.invalidate_cache();
        }
    }

    /// Get histogram (with caching)
    pub fn get_histogram(&mut self) -> Option<&MetricsHistogram> {
        let new_key = MetricsCacheKey::new(
            self.current_metric,
            self.bin_count,
            &[], // Full dataset
        );

        if self.cache_key.as_ref() != Some(&new_key) || self.cached_histogram.is_none() {
            // Recalculate
            if let Some(series) = self.series.get(&self.current_metric) {
                let values = series.values();
                let histogram = MetricsHistogram::new(self.current_metric, values, self.bin_count);
                self.cached_histogram = Some(histogram);
                self.cache_key = Some(new_key);
            }
        }

        self.cached_histogram.as_ref()
    }

    /// Get histogram for selection only (with caching)
    pub fn get_selection_histogram(&mut self) -> Option<&MetricsHistogram> {
        if self.selected_frames.is_empty() {
            return None;
        }

        let new_key =
            MetricsCacheKey::new(self.current_metric, self.bin_count, &self.selected_frames);

        if self.cache_key.as_ref() != Some(&new_key) || self.cached_selection_histogram.is_none() {
            // Recalculate
            if let Some(series) = self.series.get(&self.current_metric) {
                let values = series.values_for_selection(&self.selected_frames);
                if !values.is_empty() {
                    let histogram =
                        MetricsHistogram::new(self.current_metric, values, self.bin_count);
                    self.cached_selection_histogram = Some(histogram);
                }
            }
        }

        self.cached_selection_histogram.as_ref()
    }

    /// Get summary statistics (with caching)
    pub fn get_stats(&mut self) -> Option<&SummaryStats> {
        if self.cached_stats.is_none() {
            if let Some(series) = self.series.get(&self.current_metric) {
                let values = series.values();
                self.cached_stats = SummaryStats::calculate(values);
            }
        }
        self.cached_stats.as_ref()
    }

    /// Get summary statistics for selection only (with caching)
    pub fn get_selection_stats(&mut self) -> Option<&SummaryStats> {
        if self.selected_frames.is_empty() {
            return None;
        }

        if self.cached_selection_stats.is_none() {
            if let Some(series) = self.series.get(&self.current_metric) {
                let values = series.values_for_selection(&self.selected_frames);
                if !values.is_empty() {
                    self.cached_selection_stats = SummaryStats::calculate(values);
                }
            }
        }
        self.cached_selection_stats.as_ref()
    }

    /// Get worst frames list (with caching)
    ///
    /// Per T5-2 deliverable: "Worst frames list"
    pub fn get_worst_frames(&mut self, max_count: usize) -> Option<&WorstFrames> {
        if self.cached_worst_frames.is_none() {
            if let Some(series) = self.series.get(&self.current_metric) {
                let worst = WorstFrames::new(self.current_metric, series, max_count);
                self.cached_worst_frames = Some(worst);
            }
        }
        self.cached_worst_frames.as_ref()
    }

    /// Invalidate cache
    fn invalidate_cache(&mut self) {
        self.cached_histogram = None;
        self.cached_selection_histogram = None;
        self.cached_stats = None;
        self.cached_selection_stats = None;
        self.cached_worst_frames = None;
        self.cache_key = None;
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Metric Series Calculator (Reference Video Comparison)
// ═══════════════════════════════════════════════════════════════════════════

/// Frame-level metric calculation result
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FrameMetricResult {
    /// Frame display index
    pub display_idx: usize,
    /// PSNR Y (if calculated)
    pub psnr_y: Option<f32>,
    /// SSIM Y (if calculated)
    pub ssim_y: Option<f32>,
    /// VMAF (if calculated)
    pub vmaf: Option<f32>,
    /// Calculation time in microseconds
    pub calc_time_us: u64,
}

impl FrameMetricResult {
    pub fn new(display_idx: usize) -> Self {
        Self {
            display_idx,
            psnr_y: None,
            ssim_y: None,
            vmaf: None,
            calc_time_us: 0,
        }
    }

    /// Get metric value by type
    pub fn get(&self, metric_type: MetricType) -> Option<f32> {
        match metric_type {
            MetricType::PsnrY => self.psnr_y,
            MetricType::SsimY => self.ssim_y,
            MetricType::Vmaf => self.vmaf,
            MetricType::Custom => None,
        }
    }
}

/// Reference comparison context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReferenceComparisonContext {
    /// Reference stream path
    pub reference_path: String,
    /// Distorted stream path
    pub distorted_path: String,
    /// Frame width
    pub width: u32,
    /// Frame height
    pub height: u32,
    /// Bit depth
    pub bit_depth: u8,
    /// Chroma subsampling (e.g. "420", "422", "444")
    pub chroma: String,
    /// Total frames
    pub total_frames: usize,
    /// Computed frame metrics
    pub frame_metrics: Vec<FrameMetricResult>,
}

impl ReferenceComparisonContext {
    pub fn new(
        reference_path: String,
        distorted_path: String,
        width: u32,
        height: u32,
        bit_depth: u8,
        chroma: String,
        total_frames: usize,
    ) -> Self {
        Self {
            reference_path,
            distorted_path,
            width,
            height,
            bit_depth,
            chroma,
            total_frames,
            frame_metrics: Vec::with_capacity(total_frames),
        }
    }

    /// Add frame metric result
    pub fn add_frame_result(&mut self, result: FrameMetricResult) {
        self.frame_metrics.push(result);
    }

    /// Get progress (0.0 - 1.0)
    pub fn progress(&self) -> f32 {
        if self.total_frames > 0 {
            self.frame_metrics.len() as f32 / self.total_frames as f32
        } else {
            0.0
        }
    }

    /// Is calculation complete?
    pub fn is_complete(&self) -> bool {
        self.frame_metrics.len() >= self.total_frames
    }

    /// Convert to metric series
    pub fn to_series(&self, metric_type: MetricType) -> MetricSeries {
        let mut series = MetricSeries::new(metric_type);
        for result in &self.frame_metrics {
            if let Some(value) = result.get(metric_type) {
                series.add_point(MetricPoint::new(result.display_idx, value));
            }
        }
        series
    }
}

/// Metric calculator for computing PSNR/SSIM from frame data
#[derive(Debug, Clone)]
pub struct MetricCalculator {
    /// Bit depth
    pub bit_depth: u8,
    /// Maximum pixel value
    max_value: f32,
}

impl Default for MetricCalculator {
    fn default() -> Self {
        Self::new(8)
    }
}

impl MetricCalculator {
    pub fn new(bit_depth: u8) -> Self {
        let max_value = ((1u32 << bit_depth) - 1) as f32;
        Self {
            bit_depth,
            max_value,
        }
    }

    /// Calculate PSNR for Y plane
    pub fn calculate_psnr_y(&self, reference: &[u8], distorted: &[u8]) -> Option<f32> {
        if reference.len() != distorted.len() || reference.is_empty() {
            return None;
        }

        let mse = self.calculate_mse(reference, distorted);
        if mse <= 0.0 {
            return Some(100.0); // Perfect match
        }

        Some(10.0 * (self.max_value * self.max_value / mse).log10())
    }

    /// Calculate MSE
    pub fn calculate_mse(&self, reference: &[u8], distorted: &[u8]) -> f32 {
        if reference.len() != distorted.len() || reference.is_empty() {
            return 0.0;
        }

        let sum_sq_diff: f64 = reference
            .iter()
            .zip(distorted.iter())
            .map(|(&r, &d)| {
                let diff = r as f64 - d as f64;
                diff * diff
            })
            .sum();

        (sum_sq_diff / reference.len() as f64) as f32
    }

    /// Calculate simplified SSIM for Y plane
    pub fn calculate_ssim_y(&self, reference: &[u8], distorted: &[u8]) -> Option<f32> {
        if reference.len() != distorted.len() || reference.is_empty() {
            return None;
        }

        let n = reference.len() as f64;

        // Calculate means
        let mean_r: f64 = reference.iter().map(|&x| x as f64).sum::<f64>() / n;
        let mean_d: f64 = distorted.iter().map(|&x| x as f64).sum::<f64>() / n;

        // Calculate variances and covariance
        let mut var_r = 0.0;
        let mut var_d = 0.0;
        let mut covar = 0.0;

        for (&r, &d) in reference.iter().zip(distorted.iter()) {
            let r_diff = r as f64 - mean_r;
            let d_diff = d as f64 - mean_d;
            var_r += r_diff * r_diff;
            var_d += d_diff * d_diff;
            covar += r_diff * d_diff;
        }

        var_r /= n - 1.0;
        var_d /= n - 1.0;
        covar /= n - 1.0;

        // SSIM constants
        let c1 = (0.01 * self.max_value as f64).powi(2);
        let c2 = (0.03 * self.max_value as f64).powi(2);

        let ssim = ((2.0 * mean_r * mean_d + c1) * (2.0 * covar + c2))
            / ((mean_r.powi(2) + mean_d.powi(2) + c1) * (var_r + var_d + c2));

        Some(ssim as f32)
    }

    /// Calculate all metrics for a frame
    pub fn calculate_frame(
        &self,
        display_idx: usize,
        reference_y: &[u8],
        distorted_y: &[u8],
    ) -> FrameMetricResult {
        let start = std::time::Instant::now();

        let psnr_y = self.calculate_psnr_y(reference_y, distorted_y);
        let ssim_y = self.calculate_ssim_y(reference_y, distorted_y);

        let calc_time_us = start.elapsed().as_micros() as u64;

        FrameMetricResult {
            display_idx,
            psnr_y,
            ssim_y,
            vmaf: None, // VMAF requires external library
            calc_time_us,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Metric Series Lane (Timeline Integration)
// ═══════════════════════════════════════════════════════════════════════════

/// Metric series lane data point for timeline
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MetricLanePoint {
    /// Frame display index
    pub display_idx: usize,
    /// Metric value
    pub value: f32,
    /// Is this a "bad" frame (below threshold)?
    pub is_warning: bool,
}

/// Metric series lane for timeline visualization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricSeriesLane {
    /// Metric type
    pub metric_type: MetricType,
    /// Data points
    pub points: Vec<MetricLanePoint>,
    /// Warning threshold
    pub warning_threshold: f32,
    /// Value range for visualization
    pub range: (f32, f32),
}

impl MetricSeriesLane {
    /// Create from metric series
    pub fn from_series(series: &MetricSeries, warning_threshold: f32) -> Self {
        let (range_min, range_max) = series.metric_type.typical_range();

        let points: Vec<MetricLanePoint> = series
            .data
            .iter()
            .map(|p| MetricLanePoint {
                display_idx: p.idx,
                value: p.value,
                is_warning: p.value < warning_threshold,
            })
            .collect();

        let actual_range = if points.is_empty() {
            (range_min, range_max)
        } else {
            let min = points.iter().map(|p| p.value).fold(f32::INFINITY, f32::min);
            let max = points
                .iter()
                .map(|p| p.value)
                .fold(f32::NEG_INFINITY, f32::max);
            (min.min(range_min), max.max(range_max))
        };

        Self {
            metric_type: series.metric_type,
            points,
            warning_threshold,
            range: actual_range,
        }
    }

    /// Get normalized Y position (0.0 = bottom, 1.0 = top)
    pub fn normalized_y(&self, value: f32) -> f32 {
        let (min, max) = self.range;
        if max > min {
            ((value - min) / (max - min)).clamp(0.0, 1.0)
        } else {
            0.5
        }
    }

    /// Get warning frames
    pub fn warning_frames(&self) -> Vec<usize> {
        self.points
            .iter()
            .filter(|p| p.is_warning)
            .map(|p| p.display_idx)
            .collect()
    }

    /// Get point at display index
    pub fn get_point(&self, display_idx: usize) -> Option<&MetricLanePoint> {
        self.points.iter().find(|p| p.display_idx == display_idx)
    }
}

/// Metric comparison summary for A/B comparison
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MetricComparisonSummary {
    /// Metric type
    pub metric_type: MetricType,
    /// Stream A average
    pub avg_a: f32,
    /// Stream B average
    pub avg_b: f32,
    /// Difference (B - A)
    pub diff: f32,
    /// Frames where A is better
    pub frames_a_better: usize,
    /// Frames where B is better
    pub frames_b_better: usize,
    /// Frames with equal quality
    pub frames_equal: usize,
    /// Total frames compared
    pub total_frames: usize,
}

impl MetricComparisonSummary {
    /// Compare two metric series
    pub fn compare(
        metric_type: MetricType,
        series_a: &MetricSeries,
        series_b: &MetricSeries,
        tolerance: f32,
    ) -> Self {
        let values_a = series_a.values();
        let values_b = series_b.values();

        let avg_a = if values_a.is_empty() {
            0.0
        } else {
            values_a.iter().sum::<f32>() / values_a.len() as f32
        };

        let avg_b = if values_b.is_empty() {
            0.0
        } else {
            values_b.iter().sum::<f32>() / values_b.len() as f32
        };

        // Frame-by-frame comparison
        let mut frames_a_better = 0;
        let mut frames_b_better = 0;
        let mut frames_equal = 0;

        for point_a in &series_a.data {
            if let Some(point_b) = series_b.data.iter().find(|p| p.idx == point_a.idx) {
                let diff = point_b.value - point_a.value;
                if diff.abs() < tolerance {
                    frames_equal += 1;
                } else if diff > 0.0 {
                    frames_b_better += 1;
                } else {
                    frames_a_better += 1;
                }
            }
        }

        Self {
            metric_type,
            avg_a,
            avg_b,
            diff: avg_b - avg_a,
            frames_a_better,
            frames_b_better,
            frames_equal,
            total_frames: frames_a_better + frames_b_better + frames_equal,
        }
    }

    /// Format as display string
    pub fn format_display(&self) -> String {
        format!(
            "{}: A={:.2}{} B={:.2}{} (Δ={:+.2}{})\n  A better: {} | B better: {} | Equal: {}",
            self.metric_type.name(),
            self.avg_a,
            self.metric_type.unit(),
            self.avg_b,
            self.metric_type.unit(),
            self.diff,
            self.metric_type.unit(),
            self.frames_a_better,
            self.frames_b_better,
            self.frames_equal
        )
    }
}

#[cfg(test)]
mod tests {
    include!("metrics_distribution_test.rs");
}
