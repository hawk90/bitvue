//! Metrics Workspace - WS_METRICS_QUALITY (Monster Pack v14)
//!
//! Mixed visualization workspace with:
//! - Metric series plot (PSNR/SSIM/VMAF over time)
//! - Distribution histogram (whole vs selection)
//! - Summary panel (min/max/avg/p95)
//! - Worst frames list
//! - Delta lane (A/B compare)

use bitvue_core::{Command, FrameKey, SelectionState, StreamId};
use egui::{self, Color32};

/// Professional color palette for metrics
#[allow(dead_code)]
mod colors {
    use egui::Color32;

    pub const BACKGROUND: Color32 = Color32::from_rgb(250, 250, 250);
    pub const GRID: Color32 = Color32::from_rgb(220, 220, 220);
    pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(40, 40, 40);
    pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(100, 100, 100);

    // Metric colors
    pub const PSNR_LINE: Color32 = Color32::from_rgb(30, 144, 255); // Dodger blue
    pub const SSIM_LINE: Color32 = Color32::from_rgb(50, 205, 50); // Lime green
    pub const VMAF_LINE: Color32 = Color32::from_rgb(255, 140, 0); // Dark orange

    // A/B compare
    pub const STREAM_A: Color32 = Color32::from_rgb(30, 144, 255);
    pub const STREAM_B: Color32 = Color32::from_rgb(255, 99, 71); // Tomato
    pub const DELTA_POSITIVE: Color32 = Color32::from_rgb(50, 205, 50);
    pub const DELTA_NEGATIVE: Color32 = Color32::from_rgb(255, 69, 0);

    // Histogram
    pub const HISTOGRAM_WHOLE: Color32 = Color32::from_rgb(100, 149, 237);
    pub const HISTOGRAM_SELECTION: Color32 = Color32::from_rgb(255, 165, 0);

    // Threshold
    pub const THRESHOLD_LINE: Color32 = Color32::from_rgb(255, 0, 0);
    pub const THRESHOLD_EXCEED: Color32 = Color32::from_rgba_premultiplied(255, 0, 0, 40);

    // Selection
    pub const CURSOR: Color32 = Color32::from_rgb(50, 50, 50);
    pub const SELECTION_HIGHLIGHT: Color32 = Color32::from_rgba_premultiplied(255, 200, 200, 80);
}

/// Metric type selector
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MetricType {
    #[default]
    Psnr,
    Ssim,
    Vmaf,
}

impl MetricType {
    pub fn label(&self) -> &'static str {
        match self {
            MetricType::Psnr => "PSNR (dB)",
            MetricType::Ssim => "SSIM",
            MetricType::Vmaf => "VMAF",
        }
    }

    pub fn color(&self) -> Color32 {
        match self {
            MetricType::Psnr => colors::PSNR_LINE,
            MetricType::Ssim => colors::SSIM_LINE,
            MetricType::Vmaf => colors::VMAF_LINE,
        }
    }

    pub fn unit(&self) -> &'static str {
        match self {
            MetricType::Psnr => "dB",
            MetricType::Ssim => "",
            MetricType::Vmaf => "",
        }
    }

    pub fn range(&self) -> (f64, f64) {
        match self {
            MetricType::Psnr => (20.0, 60.0),
            MetricType::Ssim => (0.8, 1.0),
            MetricType::Vmaf => (0.0, 100.0),
        }
    }
}

/// Mock metric data point
#[derive(Debug, Clone)]
pub struct MetricPoint {
    pub frame_idx: usize,
    pub psnr: f64,
    pub ssim: f64,
    pub vmaf: f64,
}

/// Summary statistics
#[derive(Debug, Clone, Default)]
pub struct MetricSummary {
    pub min: f64,
    pub max: f64,
    pub avg: f64,
    pub p95: f64,
    pub count: usize,
}

impl MetricSummary {
    pub fn from_values(values: &[f64]) -> Self {
        if values.is_empty() {
            return Self::default();
        }

        let mut sorted = values.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let min = sorted.first().copied().unwrap_or(0.0);
        let max = sorted.last().copied().unwrap_or(0.0);
        let avg = values.iter().sum::<f64>() / values.len() as f64;
        let p95_idx = ((values.len() as f64 * 0.05) as usize).max(0);
        let p95 = sorted.get(p95_idx).copied().unwrap_or(min);

        Self {
            min,
            max,
            avg,
            p95,
            count: values.len(),
        }
    }
}

/// Worst frame entry
#[derive(Debug, Clone)]
pub struct WorstFrame {
    pub frame_idx: usize,
    pub value: f64,
    pub rank: usize,
}

/// Metrics workspace state
pub struct MetricsWorkspace {
    /// Active metric type
    active_metric: MetricType,

    /// Show A/B comparison
    show_ab_compare: bool,

    /// Zoom factor
    #[allow(dead_code)]
    zoom_factor: f32,

    /// Cursor position (frame index)
    cursor_position: Option<usize>,

    /// Selected range
    selected_range: Option<(usize, usize)>,

    /// Threshold value for alerts
    threshold: Option<f64>,

    /// Show threshold line
    show_threshold: bool,

    /// Mock metric data
    mock_data: Vec<MetricPoint>,

    /// Histogram bin count
    histogram_bins: usize,
}

impl MetricsWorkspace {
    pub fn new() -> Self {
        // Generate mock metric data
        let mock_data: Vec<MetricPoint> = (0..100)
            .map(|i| {
                let base_psnr = 35.0 + (i as f64 * 0.1).sin() * 5.0;
                let base_ssim = 0.92 + (i as f64 * 0.15).sin() * 0.05;
                let base_vmaf = 75.0 + (i as f64 * 0.08).sin() * 15.0;

                // Add some random variation
                let noise = ((i * 17) % 100) as f64 / 100.0 - 0.5;

                MetricPoint {
                    frame_idx: i,
                    psnr: base_psnr + noise * 2.0,
                    ssim: (base_ssim + noise * 0.02).clamp(0.0, 1.0),
                    vmaf: (base_vmaf + noise * 5.0).clamp(0.0, 100.0),
                }
            })
            .collect();

        Self {
            active_metric: MetricType::Psnr,
            show_ab_compare: false,
            zoom_factor: 1.0,
            cursor_position: None,
            selected_range: None,
            threshold: Some(30.0), // PSNR threshold
            show_threshold: true,
            mock_data,
            histogram_bins: 20,
        }
    }

    /// Get metric value for a point
    fn get_metric_value(&self, point: &MetricPoint) -> f64 {
        match self.active_metric {
            MetricType::Psnr => point.psnr,
            MetricType::Ssim => point.ssim,
            MetricType::Vmaf => point.vmaf,
        }
    }

    /// Get all metric values
    fn get_all_values(&self) -> Vec<f64> {
        self.mock_data
            .iter()
            .map(|p| self.get_metric_value(p))
            .collect()
    }

    /// Get values for selected range
    fn get_selected_values(&self) -> Vec<f64> {
        if let Some((start, end)) = self.selected_range {
            self.mock_data
                .iter()
                .filter(|p| p.frame_idx >= start && p.frame_idx <= end)
                .map(|p| self.get_metric_value(p))
                .collect()
        } else {
            self.get_all_values()
        }
    }

    /// Get worst frames
    fn get_worst_frames(&self, count: usize) -> Vec<WorstFrame> {
        let mut indexed: Vec<_> = self
            .mock_data
            .iter()
            .map(|p| (p.frame_idx, self.get_metric_value(p)))
            .collect();

        indexed.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        indexed
            .into_iter()
            .take(count)
            .enumerate()
            .map(|(rank, (frame_idx, value))| WorstFrame {
                frame_idx,
                value,
                rank: rank + 1,
            })
            .collect()
    }

    /// Show the metrics workspace
    pub fn show(&mut self, ui: &mut egui::Ui, selection: &SelectionState) -> Option<Command> {
        let mut clicked_command = None;

        // Header toolbar
        ui.horizontal(|ui| {
            ui.heading("📈 Metrics");
            ui.separator();

            // Metric selector
            ui.label("Metric:");
            egui::ComboBox::from_id_salt("metric_selector")
                .selected_text(self.active_metric.label())
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.active_metric, MetricType::Psnr, "PSNR (dB)");
                    ui.selectable_value(&mut self.active_metric, MetricType::Ssim, "SSIM");
                    ui.selectable_value(&mut self.active_metric, MetricType::Vmaf, "VMAF");
                });

            ui.separator();

            // A/B toggle
            if ui
                .selectable_label(self.show_ab_compare, "A/B Compare")
                .clicked()
            {
                self.show_ab_compare = !self.show_ab_compare;
            }

            ui.separator();

            // Threshold toggle
            if ui
                .selectable_label(self.show_threshold, "Threshold")
                .clicked()
            {
                self.show_threshold = !self.show_threshold;
            }
        });

        ui.separator();

        if self.mock_data.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.label(
                    egui::RichText::new("No metrics data available. Run analysis first.")
                        .color(Color32::GRAY),
                );
            });
            return None;
        }

        // Main content: Split into top (series) and bottom (histogram + summary)
        let available_height = ui.available_height();
        let series_height = available_height * 0.55;
        let bottom_height = available_height * 0.40;

        // === TOP: Metric Series Plot ===
        ui.allocate_ui(egui::vec2(ui.available_width(), series_height), |ui| {
            self.render_series_plot(ui, selection, &mut clicked_command);
        });

        ui.separator();

        // === BOTTOM: Histogram + Summary side by side ===
        ui.allocate_ui(egui::vec2(ui.available_width(), bottom_height), |ui| {
            ui.horizontal(|ui| {
                // Left: Histogram (70%)
                let hist_width = ui.available_width() * 0.65;
                ui.allocate_ui(egui::vec2(hist_width, bottom_height - 20.0), |ui| {
                    self.render_histogram(ui);
                });

                ui.separator();

                // Right: Summary + Worst frames (30%)
                ui.vertical(|ui| {
                    self.render_summary_panel(ui);
                    ui.separator();
                    self.render_worst_frames(ui, &mut clicked_command);
                });
            });
        });

        clicked_command
    }

    /// Render the series plot
    fn render_series_plot(
        &mut self,
        ui: &mut egui::Ui,
        _selection: &SelectionState,
        clicked_command: &mut Option<Command>,
    ) {
        let (range_min, range_max) = self.active_metric.range();
        let values = self.get_all_values();

        if values.is_empty() {
            return;
        }

        ui.heading(
            egui::RichText::new(format!("{} Series", self.active_metric.label()))
                .size(12.0)
                .color(colors::TEXT_PRIMARY),
        );

        let plot_rect = ui.available_rect_before_wrap();
        let plot_height = plot_rect.height() - 30.0;
        let plot_width = plot_rect.width() - 60.0;

        // Background
        ui.painter().rect_filled(plot_rect, 4.0, colors::BACKGROUND);

        // Draw grid
        let grid_lines = 5;
        for i in 0..=grid_lines {
            let y = plot_rect.top() + 20.0 + (i as f32 / grid_lines as f32) * plot_height;
            ui.painter().line_segment(
                [
                    egui::pos2(plot_rect.left() + 50.0, y),
                    egui::pos2(plot_rect.right() - 10.0, y),
                ],
                egui::Stroke::new(0.5, colors::GRID),
            );

            // Y-axis labels
            let value = range_max - (i as f64 / grid_lines as f64) * (range_max - range_min);
            ui.painter().text(
                egui::pos2(plot_rect.left() + 5.0, y),
                egui::Align2::LEFT_CENTER,
                format!("{:.1}", value),
                egui::FontId::proportional(9.0),
                colors::TEXT_SECONDARY,
            );
        }

        // Draw threshold line
        if self.show_threshold {
            if let Some(threshold) = self.threshold {
                let t_normalized = (threshold - range_min) / (range_max - range_min);
                let t_y = plot_rect.top() + 20.0 + (1.0 - t_normalized as f32) * plot_height;

                ui.painter().line_segment(
                    [
                        egui::pos2(plot_rect.left() + 50.0, t_y),
                        egui::pos2(plot_rect.right() - 10.0, t_y),
                    ],
                    egui::Stroke::new(1.5, colors::THRESHOLD_LINE),
                );

                // Shade area below threshold
                let threshold_rect = egui::Rect::from_min_max(
                    egui::pos2(plot_rect.left() + 50.0, t_y),
                    egui::pos2(
                        plot_rect.right() - 10.0,
                        plot_rect.top() + 20.0 + plot_height,
                    ),
                );
                ui.painter()
                    .rect_filled(threshold_rect, 0.0, colors::THRESHOLD_EXCEED);
            }
        }

        // Draw series line
        let mut points = Vec::new();
        let x_start = plot_rect.left() + 50.0;
        let x_scale = plot_width / values.len() as f32;

        for (i, &value) in values.iter().enumerate() {
            let x = x_start + i as f32 * x_scale;
            let normalized = ((value - range_min) / (range_max - range_min)) as f32;
            let y = plot_rect.top() + 20.0 + (1.0 - normalized) * plot_height;
            points.push(egui::pos2(x, y));
        }

        if points.len() >= 2 {
            ui.painter().add(egui::Shape::line(
                points.clone(),
                egui::Stroke::new(2.0, self.active_metric.color()),
            ));
        }

        // Draw cursor
        if let Some(cursor_idx) = self.cursor_position {
            if cursor_idx < values.len() {
                let x = x_start + cursor_idx as f32 * x_scale;
                ui.painter().line_segment(
                    [
                        egui::pos2(x, plot_rect.top() + 20.0),
                        egui::pos2(x, plot_rect.top() + 20.0 + plot_height),
                    ],
                    egui::Stroke::new(2.0, colors::CURSOR),
                );
            }
        }

        // Handle clicks
        let response = ui.allocate_rect(plot_rect, egui::Sense::click());
        if response.clicked() {
            if let Some(pos) = response.interact_pointer_pos() {
                let x_offset = pos.x - x_start;
                let frame_idx = (x_offset / x_scale) as usize;
                let frame_idx = frame_idx.min(values.len().saturating_sub(1));
                self.cursor_position = Some(frame_idx);

                *clicked_command = Some(Command::SelectFrame {
                    stream: StreamId::A,
                    frame_key: FrameKey {
                        stream: StreamId::A,
                        frame_index: frame_idx,
                        pts: None,
                    },
                });
            }
        }

        // Tooltip on hover
        response.on_hover_ui_at_pointer(|ui| {
            if let Some(pos) = ui.ctx().pointer_latest_pos() {
                let x_offset = pos.x - x_start;
                let frame_idx = ((x_offset / x_scale) as usize).min(values.len().saturating_sub(1));
                if let Some(value) = values.get(frame_idx) {
                    ui.label(format!("Frame: {}", frame_idx));
                    ui.label(format!(
                        "{}: {:.3}{}",
                        self.active_metric.label(),
                        value,
                        self.active_metric.unit()
                    ));
                }
            }
        });
    }

    /// Render the histogram
    fn render_histogram(&self, ui: &mut egui::Ui) {
        let values = self.get_all_values();
        let selected_values = self.get_selected_values();

        if values.is_empty() {
            return;
        }

        ui.heading(
            egui::RichText::new("Distribution")
                .size(12.0)
                .color(colors::TEXT_PRIMARY),
        );

        let (range_min, range_max) = self.active_metric.range();
        let bin_width = (range_max - range_min) / self.histogram_bins as f64;

        // Calculate histogram bins
        let mut whole_bins = vec![0usize; self.histogram_bins];
        let mut selection_bins = vec![0usize; self.histogram_bins];

        for &v in &values {
            let bin = ((v - range_min) / bin_width) as usize;
            let bin = bin.min(self.histogram_bins - 1);
            whole_bins[bin] += 1;
        }

        for &v in &selected_values {
            let bin = ((v - range_min) / bin_width) as usize;
            let bin = bin.min(self.histogram_bins - 1);
            selection_bins[bin] += 1;
        }

        let max_count = whole_bins.iter().max().copied().unwrap_or(1);

        let plot_rect = ui.available_rect_before_wrap();
        let plot_height = plot_rect.height() - 40.0;
        let bar_width = (plot_rect.width() - 60.0) / self.histogram_bins as f32;

        // Background
        ui.painter().rect_filled(plot_rect, 4.0, colors::BACKGROUND);

        // Draw bars
        for (i, (&whole, &sel)) in whole_bins.iter().zip(selection_bins.iter()).enumerate() {
            let x = plot_rect.left() + 50.0 + i as f32 * bar_width;
            let whole_height = (whole as f32 / max_count as f32) * plot_height;
            let sel_height = (sel as f32 / max_count as f32) * plot_height;

            // Whole stream bar
            let bar_rect = egui::Rect::from_min_size(
                egui::pos2(x + 2.0, plot_rect.top() + 10.0 + plot_height - whole_height),
                egui::vec2(bar_width - 4.0, whole_height),
            );
            ui.painter()
                .rect_filled(bar_rect, 2.0, colors::HISTOGRAM_WHOLE);

            // Selection overlay bar
            if self.selected_range.is_some() && sel > 0 {
                let sel_rect = egui::Rect::from_min_size(
                    egui::pos2(x + 2.0, plot_rect.top() + 10.0 + plot_height - sel_height),
                    egui::vec2(bar_width - 4.0, sel_height),
                );
                ui.painter()
                    .rect_filled(sel_rect, 2.0, colors::HISTOGRAM_SELECTION);
            }
        }

        // X-axis labels
        for i in 0..=4 {
            let value = range_min + (i as f64 / 4.0) * (range_max - range_min);
            let x = plot_rect.left() + 50.0 + (i as f32 / 4.0) * (plot_rect.width() - 60.0);
            ui.painter().text(
                egui::pos2(x, plot_rect.bottom() - 5.0),
                egui::Align2::CENTER_BOTTOM,
                format!("{:.1}", value),
                egui::FontId::proportional(9.0),
                colors::TEXT_SECONDARY,
            );
        }
    }

    /// Render the summary panel
    fn render_summary_panel(&self, ui: &mut egui::Ui) {
        ui.heading(
            egui::RichText::new("Summary")
                .size(12.0)
                .color(colors::TEXT_PRIMARY),
        );

        let whole_summary = MetricSummary::from_values(&self.get_all_values());
        let selection_summary = MetricSummary::from_values(&self.get_selected_values());

        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(
                    egui::RichText::new("Whole Stream")
                        .size(10.0)
                        .color(colors::TEXT_SECONDARY),
                );
                ui.label(format!("Min: {:.3}", whole_summary.min));
                ui.label(format!("Max: {:.3}", whole_summary.max));
                ui.label(format!("Avg: {:.3}", whole_summary.avg));
                ui.label(format!("P5: {:.3}", whole_summary.p95));
                ui.label(format!("Count: {}", whole_summary.count));
            });

            if self.selected_range.is_some() {
                ui.separator();
                ui.vertical(|ui| {
                    ui.label(
                        egui::RichText::new("Selection")
                            .size(10.0)
                            .color(colors::HISTOGRAM_SELECTION),
                    );
                    ui.label(format!("Min: {:.3}", selection_summary.min));
                    ui.label(format!("Max: {:.3}", selection_summary.max));
                    ui.label(format!("Avg: {:.3}", selection_summary.avg));
                    ui.label(format!("P5: {:.3}", selection_summary.p95));
                    ui.label(format!("Count: {}", selection_summary.count));
                });
            }
        });
    }

    /// Render worst frames list
    fn render_worst_frames(&self, ui: &mut egui::Ui, clicked_command: &mut Option<Command>) {
        ui.heading(
            egui::RichText::new("Worst Frames")
                .size(12.0)
                .color(colors::TEXT_PRIMARY),
        );

        let worst = self.get_worst_frames(5);

        for frame in worst {
            let response = ui.horizontal(|ui| {
                ui.label(format!("#{}", frame.rank));
                ui.label(format!("Frame {}", frame.frame_idx));
                ui.label(
                    egui::RichText::new(format!("{:.3}", frame.value))
                        .color(colors::THRESHOLD_LINE),
                );
            });

            if response.response.interact(egui::Sense::click()).clicked() {
                *clicked_command = Some(Command::SelectFrame {
                    stream: StreamId::A,
                    frame_key: FrameKey {
                        stream: StreamId::A,
                        frame_index: frame.frame_idx,
                        pts: None,
                    },
                });
            }
        }
    }
}

impl Default for MetricsWorkspace {
    fn default() -> Self {
        Self::new()
    }
}
