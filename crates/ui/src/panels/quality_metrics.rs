//! Quality Metrics Panel - PSNR/SSIM visualization with heatmap overlays
//!
//! VQAnalyzer parity: Visual metric heatmaps for per-block PSNR/SSIM display

use egui::{self, Color32, Vec2};

/// Quality metric type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MetricType {
    #[default]
    Psnr,
    Ssim,
    Vmaf,
}

/// Per-block metric value for heatmap visualization
#[derive(Debug, Clone)]
pub struct BlockMetric {
    /// Block position (x, y) in pixels
    pub x: u32,
    pub y: u32,
    /// Block size (width, height) in pixels
    pub width: u32,
    pub height: u32,
    /// PSNR value (dB) for this block
    pub psnr: Option<f64>,
    /// SSIM value (0.0 - 1.0) for this block
    pub ssim: Option<f64>,
}

/// Metric heatmap data
#[derive(Debug, Clone, Default)]
pub struct MetricHeatmap {
    /// Frame dimensions
    pub frame_width: u32,
    pub frame_height: u32,
    /// Per-block metrics
    pub blocks: Vec<BlockMetric>,
    /// Min/Max PSNR for color scaling
    pub psnr_min: f64,
    pub psnr_max: f64,
    /// Min/Max SSIM for color scaling
    pub ssim_min: f64,
    pub ssim_max: f64,
}

impl MetricHeatmap {
    /// Compute min/max values from block data
    pub fn update_ranges(&mut self) {
        self.psnr_min = f64::MAX;
        self.psnr_max = f64::MIN;
        self.ssim_min = f64::MAX;
        self.ssim_max = f64::MIN;

        for block in &self.blocks {
            if let Some(psnr) = block.psnr {
                self.psnr_min = self.psnr_min.min(psnr);
                self.psnr_max = self.psnr_max.max(psnr);
            }
            if let Some(ssim) = block.ssim {
                self.ssim_min = self.ssim_min.min(ssim);
                self.ssim_max = self.ssim_max.max(ssim);
            }
        }

        // Set defaults if no data
        if self.psnr_min == f64::MAX {
            self.psnr_min = 20.0;
            self.psnr_max = 50.0;
        }
        if self.ssim_min == f64::MAX {
            self.ssim_min = 0.8;
            self.ssim_max = 1.0;
        }
    }
}

pub struct QualityMetricsPanel {
    /// Active metric to display
    active_metric: MetricType,
    /// Show Y component
    show_y: bool,
    /// Show U component
    show_u: bool,
    /// Show V component
    show_v: bool,
    /// Enable heatmap overlay
    pub show_heatmap: bool,
    /// Heatmap data for current frame
    heatmap: Option<MetricHeatmap>,
    /// Heatmap opacity (0.0 - 1.0)
    pub heatmap_opacity: f32,
}

impl QualityMetricsPanel {
    pub fn new() -> Self {
        Self {
            active_metric: MetricType::Psnr,
            show_y: true,
            show_u: true,
            show_v: true,
            show_heatmap: false,
            heatmap: None,
            heatmap_opacity: 0.7,
        }
    }

    /// Set heatmap data for the current frame
    pub fn set_heatmap(&mut self, mut heatmap: MetricHeatmap) {
        heatmap.update_ranges();
        self.heatmap = Some(heatmap);
    }

    /// Clear heatmap data
    pub fn clear_heatmap(&mut self) {
        self.heatmap = None;
    }

    /// Get the active metric type
    pub fn active_metric(&self) -> MetricType {
        self.active_metric
    }

    /// Map a metric value to a color (red = bad, yellow = medium, green = good)
    fn value_to_color(&self, value: f64, min_val: f64, max_val: f64) -> Color32 {
        let range = max_val - min_val;
        if range <= 0.0 {
            return Color32::from_rgba_unmultiplied(
                128,
                128,
                128,
                (self.heatmap_opacity * 255.0) as u8,
            );
        }

        let normalized = ((value - min_val) / range).clamp(0.0, 1.0);

        // Red (bad) -> Yellow (medium) -> Green (good)
        let (r, g) = if normalized < 0.5 {
            // Red to Yellow
            let t = normalized * 2.0;
            (255, (255.0 * t) as u8)
        } else {
            // Yellow to Green
            let t = (normalized - 0.5) * 2.0;
            ((255.0 * (1.0 - t)) as u8, 255)
        };

        Color32::from_rgba_unmultiplied(r, g, 0, (self.heatmap_opacity * 255.0) as u8)
    }

    /// Render heatmap overlay on a given rect
    pub fn render_heatmap_overlay(&self, ui: &mut egui::Ui, rect: egui::Rect, zoom: f32) {
        let heatmap = match &self.heatmap {
            Some(h) => h,
            None => return,
        };

        if heatmap.blocks.is_empty() {
            return;
        }

        let painter = ui.painter();
        let scale_x = rect.width() / heatmap.frame_width as f32;
        let scale_y = rect.height() / heatmap.frame_height as f32;

        for block in &heatmap.blocks {
            let value = match self.active_metric {
                MetricType::Psnr => block.psnr,
                MetricType::Ssim => block.ssim,
                MetricType::Vmaf => None, // VMAF not implemented per-block
            };

            if let Some(val) = value {
                let (min_val, max_val) = match self.active_metric {
                    MetricType::Psnr => (heatmap.psnr_min, heatmap.psnr_max),
                    MetricType::Ssim => (heatmap.ssim_min, heatmap.ssim_max),
                    MetricType::Vmaf => (0.0, 100.0),
                };

                let color = self.value_to_color(val, min_val, max_val);

                let block_rect = egui::Rect::from_min_size(
                    egui::pos2(
                        rect.min.x + block.x as f32 * scale_x * zoom,
                        rect.min.y + block.y as f32 * scale_y * zoom,
                    ),
                    Vec2::new(
                        block.width as f32 * scale_x * zoom,
                        block.height as f32 * scale_y * zoom,
                    ),
                );

                painter.rect_filled(block_rect, 0.0, color);
            }
        }

        // Draw color legend
        self.render_legend(ui, rect);
    }

    /// Render color legend for the heatmap
    fn render_legend(&self, ui: &mut egui::Ui, rect: egui::Rect) {
        let legend_width = 20.0;
        let legend_height = 100.0;
        let legend_x = rect.max.x - legend_width - 10.0;
        let legend_y = rect.min.y + 10.0;

        let painter = ui.painter();

        // Draw gradient
        let steps = 20;
        let step_height = legend_height / steps as f32;

        for i in 0..steps {
            let t = 1.0 - (i as f64 / steps as f64); // Top = good, bottom = bad
            let color = self.value_to_color(t, 0.0, 1.0);

            let step_rect = egui::Rect::from_min_size(
                egui::pos2(legend_x, legend_y + i as f32 * step_height),
                Vec2::new(legend_width, step_height + 1.0),
            );

            painter.rect_filled(step_rect, 0.0, color);
        }

        // Draw border
        let legend_rect = egui::Rect::from_min_size(
            egui::pos2(legend_x, legend_y),
            Vec2::new(legend_width, legend_height),
        );
        painter.rect_stroke(legend_rect, 0.0, egui::Stroke::new(1.0, Color32::BLACK));

        // Labels
        let (label_top, label_bottom, unit) = match self.active_metric {
            MetricType::Psnr => {
                if let Some(h) = &self.heatmap {
                    (
                        format!("{:.1}", h.psnr_max),
                        format!("{:.1}", h.psnr_min),
                        "dB",
                    )
                } else {
                    ("50".to_string(), "20".to_string(), "dB")
                }
            }
            MetricType::Ssim => {
                if let Some(h) = &self.heatmap {
                    (
                        format!("{:.3}", h.ssim_max),
                        format!("{:.3}", h.ssim_min),
                        "",
                    )
                } else {
                    ("1.0".to_string(), "0.8".to_string(), "")
                }
            }
            MetricType::Vmaf => ("100".to_string(), "0".to_string(), ""),
        };

        painter.text(
            egui::pos2(legend_x + legend_width + 4.0, legend_y),
            egui::Align2::LEFT_TOP,
            format!("{} {}", label_top, unit),
            egui::FontId::proportional(10.0),
            Color32::WHITE,
        );

        painter.text(
            egui::pos2(legend_x + legend_width + 4.0, legend_y + legend_height),
            egui::Align2::LEFT_BOTTOM,
            format!("{} {}", label_bottom, unit),
            egui::FontId::proportional(10.0),
            Color32::WHITE,
        );
    }

    /// Show the quality metrics panel
    pub fn show(&mut self, ui: &mut egui::Ui) {
        ui.heading("📈 Quality Metrics");
        ui.separator();

        // Toolbar: metric selection
        ui.horizontal(|ui| {
            ui.label("Metric:");
            if ui
                .selectable_label(self.active_metric == MetricType::Psnr, "PSNR")
                .clicked()
            {
                self.active_metric = MetricType::Psnr;
            }
            if ui
                .selectable_label(self.active_metric == MetricType::Ssim, "SSIM")
                .clicked()
            {
                self.active_metric = MetricType::Ssim;
            }
            if ui
                .selectable_label(self.active_metric == MetricType::Vmaf, "VMAF")
                .clicked()
            {
                self.active_metric = MetricType::Vmaf;
            }

            ui.separator();

            ui.label("Components:");
            ui.checkbox(&mut self.show_y, "Y");
            ui.checkbox(&mut self.show_u, "U");
            ui.checkbox(&mut self.show_v, "V");

            ui.separator();

            // Heatmap controls
            ui.checkbox(&mut self.show_heatmap, "Heatmap")
                .on_hover_text("Show per-block quality heatmap overlay");

            if self.show_heatmap {
                ui.label("Opacity:");
                ui.add(egui::Slider::new(&mut self.heatmap_opacity, 0.2..=1.0).show_value(false));
            }
        });

        ui.separator();

        // Show heatmap status
        if self.show_heatmap {
            if let Some(heatmap) = &self.heatmap {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(format!(
                            "Heatmap: {} blocks, {}",
                            heatmap.blocks.len(),
                            match self.active_metric {
                                MetricType::Psnr => format!(
                                    "PSNR range: {:.1}-{:.1} dB",
                                    heatmap.psnr_min, heatmap.psnr_max
                                ),
                                MetricType::Ssim => format!(
                                    "SSIM range: {:.3}-{:.3}",
                                    heatmap.ssim_min, heatmap.ssim_max
                                ),
                                MetricType::Vmaf => "VMAF (frame-level only)".to_string(),
                            }
                        ))
                        .small()
                        .color(Color32::from_rgb(100, 200, 100)),
                    );
                });
            } else {
                ui.label(
                    egui::RichText::new("No heatmap data - requires reference comparison")
                        .small()
                        .color(Color32::GRAY),
                );
            }
        }

        ui.separator();

        // Placeholder for when no data is available
        if self.heatmap.is_none() {
            ui.centered_and_justified(|ui| {
                ui.label(
                    egui::RichText::new(format!(
                        "{} metrics require reference video comparison\n\nTo use:\n1. Load two streams (A and B)\n2. Enable metric computation\n3. Heatmap will overlay on player",
                        match self.active_metric {
                            MetricType::Psnr => "PSNR",
                            MetricType::Ssim => "SSIM",
                            MetricType::Vmaf => "VMAF",
                        }
                    ))
                    .color(Color32::GRAY),
                );
            });
        }
    }
}

impl Default for QualityMetricsPanel {
    fn default() -> Self {
        Self::new()
    }
}
