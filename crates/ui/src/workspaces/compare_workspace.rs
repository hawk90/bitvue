//! Compare Workspace - WS_COMPARE_AB (Monster Pack v14)
//!
//! Mixed visualization workspace with:
//! - Dual timelines (A + B stacked)
//! - Delta lane (A-B for size/QP/metrics)
//! - Sync modes (Off/Playhead/Full)
//! - Regression rules + violations list
//! - Player compare strip

use bitvue_core::{Command, FrameKey, SelectionState, StreamId};
use egui::{self, Color32};

/// Professional color palette for compare
#[allow(dead_code)]
mod colors {
    use egui::Color32;

    pub const BACKGROUND: Color32 = Color32::from_rgb(250, 250, 250);
    pub const GRID: Color32 = Color32::from_rgb(220, 220, 220);
    pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(40, 40, 40);
    pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(100, 100, 100);

    // Stream colors
    pub const STREAM_A: Color32 = Color32::from_rgb(30, 144, 255); // Dodger blue
    pub const STREAM_B: Color32 = Color32::from_rgb(255, 140, 0); // Orange
    pub const STREAM_A_LIGHT: Color32 = Color32::from_rgba_premultiplied(30, 144, 255, 80);
    pub const STREAM_B_LIGHT: Color32 = Color32::from_rgba_premultiplied(255, 140, 0, 80);

    // Delta colors
    pub const DELTA_POSITIVE: Color32 = Color32::from_rgb(50, 205, 50); // Green (A > B)
    pub const DELTA_NEGATIVE: Color32 = Color32::from_rgb(255, 69, 0); // Red (A < B)
    pub const DELTA_ZERO: Color32 = Color32::from_rgb(128, 128, 128); // Gray (equal)

    // Threshold
    pub const THRESHOLD_LINE: Color32 = Color32::from_rgb(255, 0, 0);
    pub const THRESHOLD_EXCEED: Color32 = Color32::from_rgba_premultiplied(255, 0, 0, 40);

    // Violations
    pub const VIOLATION_BG: Color32 = Color32::from_rgb(255, 240, 240);
    pub const VIOLATION_BORDER: Color32 = Color32::from_rgb(255, 100, 100);

    // Selection
    pub const CURSOR: Color32 = Color32::from_rgb(50, 50, 50);
    pub const SYNC_CURSOR: Color32 = Color32::from_rgb(100, 100, 255);
}

/// Sync mode for A/B comparison
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SyncMode {
    Off, // Independent navigation
    #[default]
    Playhead, // Cursors synced
    Full, // Full lock (zoom + pan + cursor)
}

impl SyncMode {
    pub fn label(&self) -> &'static str {
        match self {
            SyncMode::Off => "Off",
            SyncMode::Playhead => "Playhead",
            SyncMode::Full => "Full",
        }
    }
}

/// Delta metric type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DeltaMetric {
    #[default]
    Size,
    Qp,
    Psnr,
    Ssim,
}

impl DeltaMetric {
    pub fn label(&self) -> &'static str {
        match self {
            DeltaMetric::Size => "Size",
            DeltaMetric::Qp => "QP",
            DeltaMetric::Psnr => "PSNR",
            DeltaMetric::Ssim => "SSIM",
        }
    }
}

/// Frame data for comparison (built from real streams)
#[derive(Debug, Clone)]
pub struct CompareFrame {
    pub frame_idx: usize,
    pub size_a: u64,
    pub size_b: u64,
    pub qp_a: u8,
    pub qp_b: u8,
    pub psnr_a: f64,
    pub psnr_b: f64,
}

use bitvue_core::UnitNode;

/// Regression rule
#[derive(Debug, Clone)]
pub struct RegressionRule {
    pub id: usize,
    pub name: String,
    pub metric: DeltaMetric,
    pub threshold: f64,
    pub direction: ThresholdDirection,
    pub enabled: bool,
}

/// Threshold direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThresholdDirection {
    AboveIsViolation,  // A > B by threshold is violation
    BelowIsViolation,  // A < B by threshold is violation
    EitherIsViolation, // |A - B| > threshold is violation
}

/// Violation entry
#[derive(Debug, Clone)]
pub struct Violation {
    pub rule_id: usize,
    pub rule_name: String,
    pub frame_idx: usize,
    pub value_a: f64,
    pub value_b: f64,
    pub delta: f64,
}

/// First difference result
#[derive(Debug, Clone)]
pub struct FirstDiffResult {
    /// Byte offset where the first difference was found
    pub offset: u64,
    /// Value in stream A at that offset
    pub byte_a: u8,
    /// Value in stream B at that offset
    pub byte_b: u8,
}

/// Compare workspace state
pub struct CompareWorkspace {
    /// Sync mode
    sync_mode: SyncMode,

    /// Active delta metric
    delta_metric: DeltaMetric,

    /// Cursor positions
    cursor_a: Option<usize>,
    cursor_b: Option<usize>,

    /// Zoom factors
    #[allow(dead_code)]
    zoom_a: f32,
    #[allow(dead_code)]
    zoom_b: f32,

    /// Show delta lane
    show_delta_lane: bool,

    /// Frame comparison data (built from real streams)
    frames: Vec<CompareFrame>,

    /// Whether we have real data loaded
    has_real_data: bool,

    /// Regression rules
    rules: Vec<RegressionRule>,

    /// Current violations
    violations: Vec<Violation>,

    /// Selected violation
    selected_violation: Option<usize>,

    /// Raw bytes for stream A (for byte-level comparison)
    stream_bytes_a: Option<Vec<u8>>,

    /// Raw bytes for stream B (for byte-level comparison)
    stream_bytes_b: Option<Vec<u8>>,

    /// First difference result (cached after find operation)
    first_diff: Option<FirstDiffResult>,
}

impl CompareWorkspace {
    pub fn new() -> Self {
        // Default regression rules
        let rules = vec![
            RegressionRule {
                id: 1,
                name: "Size increase > 10%".to_string(),
                metric: DeltaMetric::Size,
                threshold: 10.0,
                direction: ThresholdDirection::AboveIsViolation,
                enabled: true,
            },
            RegressionRule {
                id: 2,
                name: "PSNR drop > 1dB".to_string(),
                metric: DeltaMetric::Psnr,
                threshold: 1.0,
                direction: ThresholdDirection::BelowIsViolation,
                enabled: true,
            },
            RegressionRule {
                id: 3,
                name: "QP delta > 3".to_string(),
                metric: DeltaMetric::Qp,
                threshold: 3.0,
                direction: ThresholdDirection::EitherIsViolation,
                enabled: false,
            },
        ];

        Self {
            sync_mode: SyncMode::Playhead,
            delta_metric: DeltaMetric::Size,
            cursor_a: Some(0),
            cursor_b: Some(0),
            zoom_a: 1.0,
            zoom_b: 1.0,
            show_delta_lane: true,
            frames: Vec::new(),
            has_real_data: false,
            rules,
            violations: Vec::new(),
            selected_violation: None,
            stream_bytes_a: None,
            stream_bytes_b: None,
            first_diff: None,
        }
    }

    /// Set raw bytes for stream A
    pub fn set_stream_bytes_a(&mut self, bytes: Vec<u8>) {
        self.stream_bytes_a = Some(bytes);
        self.first_diff = None; // Clear cached result
    }

    /// Set raw bytes for stream B
    pub fn set_stream_bytes_b(&mut self, bytes: Vec<u8>) {
        self.stream_bytes_b = Some(bytes);
        self.first_diff = None; // Clear cached result
    }

    /// Find the first differing byte between the two streams
    pub fn find_first_difference(&mut self) -> Option<FirstDiffResult> {
        let bytes_a = self.stream_bytes_a.as_ref()?;
        let bytes_b = self.stream_bytes_b.as_ref()?;

        // Find first difference
        let min_len = bytes_a.len().min(bytes_b.len());

        for i in 0..min_len {
            if bytes_a[i] != bytes_b[i] {
                let result = FirstDiffResult {
                    offset: i as u64,
                    byte_a: bytes_a[i],
                    byte_b: bytes_b[i],
                };
                self.first_diff = Some(result.clone());
                return Some(result);
            }
        }

        // If lengths differ, the first difference is at the end of the shorter stream
        if bytes_a.len() != bytes_b.len() {
            let offset = min_len as u64;
            let result = FirstDiffResult {
                offset,
                byte_a: bytes_a.get(min_len).copied().unwrap_or(0),
                byte_b: bytes_b.get(min_len).copied().unwrap_or(0),
            };
            self.first_diff = Some(result.clone());
            return Some(result);
        }

        // Streams are identical
        self.first_diff = None;
        None
    }

    /// Get the cached first difference result
    pub fn get_first_diff(&self) -> Option<&FirstDiffResult> {
        self.first_diff.as_ref()
    }

    /// Check if both streams have byte data loaded
    pub fn has_byte_data(&self) -> bool {
        self.stream_bytes_a.is_some() && self.stream_bytes_b.is_some()
    }

    /// Update comparison data from real stream units
    pub fn update_from_streams(&mut self, units_a: &[UnitNode], units_b: &[UnitNode]) {
        // Extract frames from both streams
        let frames_a: Vec<_> = units_a.iter().filter(|u| u.frame_index.is_some()).collect();
        let frames_b: Vec<_> = units_b.iter().filter(|u| u.frame_index.is_some()).collect();

        if frames_a.is_empty() || frames_b.is_empty() {
            return;
        }

        // Build comparison frames (aligned by frame index)
        let max_frames = frames_a.len().max(frames_b.len());
        self.frames = (0..max_frames)
            .map(|i| {
                let frame_a = frames_a.get(i);
                let frame_b = frames_b.get(i);

                CompareFrame {
                    frame_idx: i,
                    size_a: frame_a.map(|f| f.size as u64).unwrap_or(0),
                    size_b: frame_b.map(|f| f.size as u64).unwrap_or(0),
                    qp_a: frame_a.and_then(|f| f.qp_avg).unwrap_or(0),
                    qp_b: frame_b.and_then(|f| f.qp_avg).unwrap_or(0),
                    psnr_a: 0.0, // PSNR requires decoded comparison
                    psnr_b: 0.0,
                }
            })
            .collect();

        self.has_real_data = true;

        // Recalculate violations
        self.recalculate_violations();
    }

    /// Recalculate violations based on current rules
    fn recalculate_violations(&mut self) {
        self.violations.clear();

        for frame in &self.frames {
            for rule in &self.rules {
                if !rule.enabled {
                    continue;
                }

                let (value_a, value_b, delta) = match rule.metric {
                    DeltaMetric::Size => {
                        if frame.size_b == 0 {
                            continue;
                        }
                        let delta = (frame.size_a as f64 - frame.size_b as f64)
                            / frame.size_b as f64
                            * 100.0;
                        (frame.size_a as f64, frame.size_b as f64, delta)
                    }
                    DeltaMetric::Qp => {
                        let delta = frame.qp_a as f64 - frame.qp_b as f64;
                        (frame.qp_a as f64, frame.qp_b as f64, delta)
                    }
                    DeltaMetric::Psnr => {
                        let delta = frame.psnr_a - frame.psnr_b;
                        (frame.psnr_a, frame.psnr_b, delta)
                    }
                    DeltaMetric::Ssim => continue, // Not implemented
                };

                let is_violation = match rule.direction {
                    ThresholdDirection::AboveIsViolation => delta > rule.threshold,
                    ThresholdDirection::BelowIsViolation => delta < -rule.threshold,
                    ThresholdDirection::EitherIsViolation => delta.abs() > rule.threshold,
                };

                if is_violation {
                    self.violations.push(Violation {
                        rule_id: rule.id,
                        rule_name: rule.name.clone(),
                        frame_idx: frame.frame_idx,
                        value_a,
                        value_b,
                        delta,
                    });
                }
            }
        }
    }

    /// Get delta value for a frame
    fn get_delta(&self, frame: &CompareFrame) -> f64 {
        match self.delta_metric {
            DeltaMetric::Size => {
                (frame.size_a as f64 - frame.size_b as f64) / frame.size_b as f64 * 100.0
            }
            DeltaMetric::Qp => frame.qp_a as f64 - frame.qp_b as f64,
            DeltaMetric::Psnr => frame.psnr_a - frame.psnr_b,
            DeltaMetric::Ssim => 0.0, // Not in mock data
        }
    }

    /// Show the compare workspace
    pub fn show(&mut self, ui: &mut egui::Ui, _selection: &SelectionState) -> Option<Command> {
        let mut clicked_command = None;

        // Header toolbar
        ui.horizontal(|ui| {
            ui.heading("⚖️ Compare A/B");
            ui.separator();

            // Sync mode selector
            ui.label("Sync:");
            egui::ComboBox::from_id_salt("sync_mode")
                .selected_text(self.sync_mode.label())
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.sync_mode, SyncMode::Off, "Off");
                    ui.selectable_value(&mut self.sync_mode, SyncMode::Playhead, "Playhead");
                    ui.selectable_value(&mut self.sync_mode, SyncMode::Full, "Full");
                });

            ui.separator();

            // Delta metric selector
            ui.label("Delta:");
            egui::ComboBox::from_id_salt("delta_metric")
                .selected_text(self.delta_metric.label())
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.delta_metric, DeltaMetric::Size, "Size");
                    ui.selectable_value(&mut self.delta_metric, DeltaMetric::Qp, "QP");
                    ui.selectable_value(&mut self.delta_metric, DeltaMetric::Psnr, "PSNR");
                    ui.selectable_value(&mut self.delta_metric, DeltaMetric::Ssim, "SSIM");
                });

            ui.separator();

            // Delta lane toggle
            if ui
                .selectable_label(self.show_delta_lane, "Delta Lane")
                .clicked()
            {
                self.show_delta_lane = !self.show_delta_lane;
            }

            ui.separator();

            // Find First Difference button
            let find_diff_enabled = self.has_byte_data();
            ui.add_enabled_ui(find_diff_enabled, |ui| {
                if ui
                    .button("Find First Diff")
                    .on_hover_text("Find the first differing byte between streams")
                    .on_disabled_hover_text("Load both streams to enable byte comparison")
                    .clicked()
                {
                    if let Some(diff) = self.find_first_difference() {
                        // Navigate to the difference in both streams
                        clicked_command = Some(Command::JumpToOffset {
                            stream: StreamId::A,
                            offset: diff.offset,
                        });
                    }
                }
            });

            // Show first diff result if available
            if let Some(diff) = &self.first_diff {
                ui.separator();
                ui.label(
                    egui::RichText::new(format!(
                        "First diff @ 0x{:X}: A=0x{:02X} B=0x{:02X}",
                        diff.offset, diff.byte_a, diff.byte_b
                    ))
                    .small()
                    .color(colors::VIOLATION_BORDER),
                );
            }
        });

        ui.separator();

        if self.frames.is_empty() && !self.has_byte_data() {
            ui.centered_and_justified(|ui| {
                ui.label(
                    egui::RichText::new("Load two streams for comparison").color(Color32::GRAY),
                );
            });
            return None;
        }

        // Main layout: Timelines (left) + Controls (right)
        ui.horizontal(|ui| {
            // Left column: Stacked timelines
            let timeline_width = ui.available_width() * 0.70;
            ui.allocate_ui(egui::vec2(timeline_width, ui.available_height()), |ui| {
                ui.vertical(|ui| {
                    // Timeline A
                    let timeline_height = if self.show_delta_lane { 120.0 } else { 160.0 };
                    ui.allocate_ui(egui::vec2(timeline_width, timeline_height), |ui| {
                        self.render_timeline(ui, StreamId::A, &mut clicked_command);
                    });

                    // Timeline B
                    ui.allocate_ui(egui::vec2(timeline_width, timeline_height), |ui| {
                        self.render_timeline(ui, StreamId::B, &mut clicked_command);
                    });

                    // Delta lane
                    if self.show_delta_lane {
                        ui.allocate_ui(egui::vec2(timeline_width, 100.0), |ui| {
                            self.render_delta_lane(ui);
                        });
                    }
                });
            });

            ui.separator();

            // Right column: Controls + Violations
            ui.vertical(|ui| {
                self.render_rules_panel(ui);
                ui.separator();
                self.render_violations_list(ui, &mut clicked_command);
            });
        });

        clicked_command
    }

    /// Render a single timeline
    fn render_timeline(
        &mut self,
        ui: &mut egui::Ui,
        stream: StreamId,
        clicked_command: &mut Option<Command>,
    ) {
        let (color, label, cursor) = match stream {
            StreamId::A => (colors::STREAM_A, "Stream A", &mut self.cursor_a),
            StreamId::B => (colors::STREAM_B, "Stream B", &mut self.cursor_b),
        };

        // Header
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(label).color(color).strong());
        });

        let plot_rect = ui.available_rect_before_wrap();
        let plot_height = plot_rect.height() - 20.0;
        let plot_width = plot_rect.width() - 20.0;

        // Background
        ui.painter().rect_filled(plot_rect, 4.0, colors::BACKGROUND);

        // Find max size for scaling
        let max_size = self
            .frames
            .iter()
            .map(|f| f.size_a.max(f.size_b))
            .max()
            .unwrap_or(1);

        let bar_width = plot_width / self.frames.len() as f32;

        // Draw bars
        for (i, frame) in self.frames.iter().enumerate() {
            let size = match stream {
                StreamId::A => frame.size_a,
                StreamId::B => frame.size_b,
            };

            let x = plot_rect.left() + 10.0 + i as f32 * bar_width;
            let bar_height = (size as f32 / max_size as f32) * plot_height;
            let y = plot_rect.top() + 10.0 + plot_height - bar_height;

            let bar_rect = egui::Rect::from_min_size(
                egui::pos2(x, y),
                egui::vec2(bar_width - 2.0, bar_height),
            );

            ui.painter().rect_filled(bar_rect, 1.0, color);
        }

        // Draw cursor
        if let Some(cursor_idx) = *cursor {
            if cursor_idx < self.frames.len() {
                let x = plot_rect.left() + 10.0 + cursor_idx as f32 * bar_width + bar_width / 2.0;
                ui.painter().line_segment(
                    [
                        egui::pos2(x, plot_rect.top() + 10.0),
                        egui::pos2(x, plot_rect.top() + 10.0 + plot_height),
                    ],
                    egui::Stroke::new(2.0, colors::CURSOR),
                );
            }
        }

        // Handle clicks
        let response = ui.allocate_rect(plot_rect, egui::Sense::click());
        if response.clicked() {
            if let Some(pos) = response.interact_pointer_pos() {
                let x_offset = pos.x - plot_rect.left() - 10.0;
                let frame_idx = (x_offset / bar_width) as usize;
                let frame_idx = frame_idx.min(self.frames.len().saturating_sub(1));

                *cursor = Some(frame_idx);

                // Sync if needed
                if self.sync_mode != SyncMode::Off {
                    match stream {
                        StreamId::A => self.cursor_b = Some(frame_idx),
                        StreamId::B => self.cursor_a = Some(frame_idx),
                    }
                }

                *clicked_command = Some(Command::SelectFrame {
                    stream,
                    frame_key: FrameKey {
                        stream,
                        frame_index: frame_idx,
                        pts: None,
                    },
                });
            }
        }
    }

    /// Render delta lane
    fn render_delta_lane(&self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(format!("Delta ({})", self.delta_metric.label()))
                    .size(11.0)
                    .color(colors::TEXT_PRIMARY),
            );
        });

        let plot_rect = ui.available_rect_before_wrap();
        let plot_height = plot_rect.height() - 20.0;
        let plot_width = plot_rect.width() - 20.0;

        // Background
        ui.painter().rect_filled(plot_rect, 4.0, colors::BACKGROUND);

        // Zero line
        let zero_y = plot_rect.top() + 10.0 + plot_height / 2.0;
        ui.painter().line_segment(
            [
                egui::pos2(plot_rect.left() + 10.0, zero_y),
                egui::pos2(plot_rect.right() - 10.0, zero_y),
            ],
            egui::Stroke::new(1.0, colors::GRID),
        );

        // Find max delta for scaling
        let max_delta = self
            .frames
            .iter()
            .map(|f| self.get_delta(f).abs())
            .fold(0.0f64, |a: f64, b| a.max(b));

        if max_delta < 0.001 {
            return;
        }

        let bar_width = plot_width / self.frames.len() as f32;

        // Draw delta bars
        for (i, frame) in self.frames.iter().enumerate() {
            let delta = self.get_delta(frame);
            let bar_height = (delta.abs() / max_delta) as f32 * (plot_height / 2.0 - 5.0);

            let x = plot_rect.left() + 10.0 + i as f32 * bar_width;
            let color = if delta > 0.0 {
                colors::DELTA_POSITIVE
            } else if delta < 0.0 {
                colors::DELTA_NEGATIVE
            } else {
                colors::DELTA_ZERO
            };

            let y = if delta > 0.0 {
                zero_y - bar_height
            } else {
                zero_y
            };

            let bar_rect = egui::Rect::from_min_size(
                egui::pos2(x, y),
                egui::vec2(bar_width - 2.0, bar_height),
            );

            ui.painter().rect_filled(bar_rect, 1.0, color);
        }
    }

    /// Render regression rules panel
    fn render_rules_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading(
            egui::RichText::new("Regression Rules")
                .size(12.0)
                .color(colors::TEXT_PRIMARY),
        );

        for rule in &mut self.rules {
            ui.horizontal(|ui| {
                if ui.checkbox(&mut rule.enabled, "").changed() {
                    // Recalculate violations when rules change
                }
                ui.label(&rule.name);
            });
        }

        ui.separator();

        if ui.button("+ Add Rule from Selection").clicked() {
            // Would create rule from current selection
        }
    }

    /// Render violations list
    fn render_violations_list(&mut self, ui: &mut egui::Ui, clicked_command: &mut Option<Command>) {
        let violation_count = self.violations.len();
        ui.heading(
            egui::RichText::new(format!("Violations ({})", violation_count))
                .size(12.0)
                .color(if violation_count > 0 {
                    colors::VIOLATION_BORDER
                } else {
                    colors::TEXT_PRIMARY
                }),
        );

        if self.violations.is_empty() {
            ui.label(egui::RichText::new("No violations").color(colors::TEXT_SECONDARY));
            return;
        }

        egui::ScrollArea::vertical()
            .max_height(200.0)
            .show(ui, |ui| {
                for (idx, violation) in self.violations.iter().enumerate() {
                    let is_selected = self.selected_violation == Some(idx);

                    let response = ui
                        .horizontal(|ui| {
                            let bg_color = if is_selected {
                                colors::VIOLATION_BG
                            } else {
                                Color32::TRANSPARENT
                            };

                            egui::Frame::none().fill(bg_color).show(ui, |ui| {
                                ui.vertical(|ui| {
                                    ui.label(format!("Frame {}", violation.frame_idx));
                                    ui.label(
                                        egui::RichText::new(&violation.rule_name)
                                            .size(10.0)
                                            .color(colors::TEXT_SECONDARY),
                                    );
                                    ui.label(
                                        egui::RichText::new(format!("Δ = {:.2}", violation.delta))
                                            .color(colors::VIOLATION_BORDER),
                                    );
                                });
                            });
                        })
                        .response;

                    if response.interact(egui::Sense::click()).clicked() {
                        self.selected_violation = Some(idx);

                        // Jump to violation frame
                        self.cursor_a = Some(violation.frame_idx);
                        self.cursor_b = Some(violation.frame_idx);

                        *clicked_command = Some(Command::SelectFrame {
                            stream: StreamId::A,
                            frame_key: FrameKey {
                                stream: StreamId::A,
                                frame_index: violation.frame_idx,
                                pts: None,
                            },
                        });
                    }

                    ui.separator();
                }
            });

        ui.separator();

        if ui.button("Export Diff Bundle").clicked() {
            // Emit command to export evidence bundle with compare data
            // Per export_entrypoints.json: CompareWorkspace > Toolbar > Export Diff Bundle
            *clicked_command = Some(Command::ExportEvidenceBundle {
                stream: StreamId::A,
                path: std::path::PathBuf::from("."),
            });
        }
    }
}

impl Default for CompareWorkspace {
    fn default() -> Self {
        Self::new()
    }
}
