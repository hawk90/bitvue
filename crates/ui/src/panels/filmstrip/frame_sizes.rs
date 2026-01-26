//! Frame Sizes view - VQAnalyzer parity bar chart visualization
//!
//! Dual-graph visualization:
//! - Moving average line (purple, drawn behind)
//! - Frame size bars (colored by type, drawn on top)
//! - Right panel: Statistics, filters, controls
//!
//! FIXME: Remaining issues to match VQAnalyzer exactly:
//! - [ ] Legend positioning: Should snap to panel edges better
//! - [ ] Y-axis title: Verify exact font size and positioning
//! - [ ] QP axis: Add actual QP data visualization (currently placeholder)
//! - [ ] Moving average: Verify window calculation matches VQAnalyzer exactly
//! - [ ] Bar colors: Fine-tune to match VQAnalyzer RGB values exactly
//! - [ ] Grid lines: Verify spacing and color match
//! - [ ] Performance: Optimize for large frame counts (1000+ frames)

use super::{FilmstripPanel, FrameInfo};
use bitvue_core::{Command, StreamId};
use egui::{self, Color32, Rect, Rounding, Sense, Stroke, Vec2};

impl FilmstripPanel {
    /// Render Frame Sizes view (VQAnalyzer parity)
    ///
    /// Dual-graph visualization:
    /// 1. Moving average line graph (purple/blue)
    /// 2. Frame size bars (colored by type)
    /// 3. Right panel: Statistics, filters, controls
    pub(super) fn render_frame_sizes_view(
        &mut self,
        ui: &mut egui::Ui,
        frames: &[FrameInfo],
        selected_frame_index: Option<usize>,
    ) -> Option<Command> {
        let mut result_command: Option<Command> = None;

        if frames.is_empty() {
            tracing::warn!("Frame Sizes view: No frames available");
            ui.centered_and_justified(|ui| {
                ui.label("No frames");
            });
            return None;
        }

        tracing::info!("📊 Frame Sizes view: Rendering {} frames", frames.len());

        // Log frame types for debugging
        for (idx, frame) in frames.iter().enumerate() {
            tracing::info!("📊 Frame {}: type='{}', size={}", idx, frame.frame_type, frame.size);
        }

        // Calculate statistics (VQAnalyzer parity)
        // AV1 frame types: KEY, INTER, INTRA_ONLY, SWITCH
        let total_size: usize = frames.iter().map(|f| f.size).sum();
        let i_frames: Vec<_> = frames.iter().filter(|f| f.frame_type == "KEY" || f.frame_type == "INTRA_ONLY" || f.frame_type == "SWITCH").collect();
        let p_frames: Vec<_> = frames.iter().filter(|f| f.frame_type == "INTER").collect();
        let b_frames: Vec<&FrameInfo> = Vec::new(); // AV1 doesn't have B-frames

        let i_total: usize = i_frames.iter().map(|f| f.size).sum();
        let p_total: usize = p_frames.iter().map(|f| f.size).sum();
        let b_total: usize = b_frames.iter().map(|f| f.size).sum();

        let i_avg = if !i_frames.is_empty() { i_total / i_frames.len() } else { 0 };
        let p_avg = if !p_frames.is_empty() { p_total / p_frames.len() } else { 0 };
        let b_avg = if !b_frames.is_empty() { b_total / b_frames.len() } else { 0 };

        // Calculate moving average (in kbps - VQAnalyzer units)
        // Convert bytes to kbps: bytes * 8 / 1000
        let moving_avg: Vec<f32> = frames.iter().enumerate().map(|(i, _)| {
            let start = i.saturating_sub(self.moving_avg_window / 2);
            let end = (i + self.moving_avg_window / 2 + 1).min(frames.len());
            let window = &frames[start..end];
            let sum: usize = window.iter().map(|f| f.size).sum();
            let avg_bytes = sum as f32 / window.len() as f32;
            avg_bytes * 8.0 / 1000.0  // Convert to kbps
        }).collect();

        // Y-axis in kbps (VQAnalyzer parity)
        let max_size_kbps = frames.iter().map(|f| f.size as f32 * 8.0 / 1000.0).fold(0.0f32, f32::max);
        let max_avg = moving_avg.iter().cloned().fold(0.0f32, f32::max);
        let y_max = max_size_kbps.max(max_avg);

        tracing::info!("📊 Frame Sizes: max_size_kbps={}, max_avg={}, y_max={}", max_size_kbps, max_avg, y_max);
        tracing::info!("📊 Statistics: I={}/{}, P={}/{}, B={}/{}", i_frames.len(), i_total, p_frames.len(), p_total, b_frames.len(), b_total);

        // Main layout: Full-width graph with floating legend (VQAnalyzer style)
        let available_rect = ui.available_rect_before_wrap(); // For legend positioning
        let graph_response = egui::ScrollArea::horizontal()
            .auto_shrink([false, false])
            .id_salt("frame_sizes_graph")
            .show(ui, |ui| {
                    // VQAnalyzer dimensions
                    let y_label_width = 70.0; // Y-axis label area width
                    let qp_axis_width = 50.0; // QP axis width (right side)
                    let x_label_height = 20.0; // X-axis label height
                    let graph_height = ui.available_height() - x_label_height - 5.0;
                    let bar_width = 3.0;  // VQAnalyzer: thin bars
                    let bar_spacing = 1.0; // VQAnalyzer: minimal spacing
                    let total_width = frames.len() as f32 * (bar_width + bar_spacing) + y_label_width + qp_axis_width;

                    let (full_rect, _) = ui.allocate_exact_size(
                        Vec2::new(total_width, graph_height + x_label_height),
                        Sense::hover(),
                    );

                    let painter = ui.painter();

                    // Y-axis area (VQAnalyzer: gray background for entire left area)
                    let y_axis_rect = Rect::from_min_size(
                        full_rect.min,
                        Vec2::new(y_label_width, graph_height),
                    );
                    painter.rect_filled(y_axis_rect, Rounding::ZERO, Color32::from_rgb(225, 225, 225));

                    // Y-axis title box (darker background)
                    let title_rect = Rect::from_min_size(
                        full_rect.min,
                        Vec2::new(y_label_width, 22.0),
                    );
                    painter.rect_filled(title_rect, Rounding::ZERO, Color32::from_rgb(210, 210, 210));

                    // Y-axis title text
                    painter.text(
                        egui::pos2(full_rect.left() + 3.0, full_rect.top() + 11.0),
                        egui::Align2::LEFT_CENTER,
                        "Bitrate, kbps (30 fps)",
                        egui::FontId::proportional(8.5),
                        Color32::from_rgb(40, 40, 40),
                    );

                    // Graph area (light gray background)
                    let graph_rect = Rect::from_min_max(
                        egui::pos2(full_rect.left() + y_label_width, full_rect.top()),
                        egui::pos2(full_rect.right(), full_rect.top() + graph_height),
                    );
                    painter.rect_filled(graph_rect, Rounding::ZERO, Color32::from_rgb(245, 246, 248));

                    // Grid lines (VQAnalyzer: 5 horizontal lines)
                    let num_grid_lines = 5;
                    for i in 0..=num_grid_lines {
                        let y_pct = i as f32 / num_grid_lines as f32;
                        let y = graph_rect.bottom() - y_pct * graph_height;

                        // Grid line
                        painter.line_segment(
                            [egui::pos2(graph_rect.left(), y), egui::pos2(graph_rect.right(), y)],
                            Stroke::new(0.5, Color32::from_rgb(210, 212, 215)),
                        );

                        // Y-axis label (left of graph)
                        let value = (y_pct * y_max) as usize;
                        painter.text(
                            egui::pos2(full_rect.left() + y_label_width - 5.0, y),
                            egui::Align2::RIGHT_CENTER,
                            if value >= 1000 {
                                format!("{}k", value / 1000)
                            } else {
                                format!("{}", value)
                            },
                            egui::FontId::proportional(9.0),
                            Color32::from_rgb(90, 90, 90),
                        );
                    }

                    // X-axis labels (VQAnalyzer: every 50 frames)
                    for i in (0..frames.len()).step_by(50) {
                        let x = graph_rect.left() + i as f32 * (bar_width + bar_spacing) + bar_width / 2.0;
                        painter.text(
                            egui::pos2(x, graph_rect.bottom() + 5.0),
                            egui::Align2::CENTER_TOP,
                            format!("{}", i),
                            egui::FontId::proportional(9.0),
                            Color32::from_rgb(90, 90, 90),
                        );
                    }

                    // **FIX 2: Right QP axis** (VQAnalyzer parity)
                    let qp_axis_rect = Rect::from_min_size(
                        egui::pos2(graph_rect.right(), full_rect.top()),
                        Vec2::new(qp_axis_width, graph_height),
                    );
                    painter.rect_filled(qp_axis_rect, Rounding::ZERO, Color32::from_rgb(225, 225, 225));

                    // QP axis title box
                    let qp_title_rect = Rect::from_min_size(
                        egui::pos2(graph_rect.right(), full_rect.top()),
                        Vec2::new(qp_axis_width, 22.0),
                    );
                    painter.rect_filled(qp_title_rect, Rounding::ZERO, Color32::from_rgb(210, 210, 210));
                    painter.text(
                        egui::pos2(graph_rect.right() + qp_axis_width / 2.0, full_rect.top() + 11.0),
                        egui::Align2::CENTER_CENTER,
                        "QP",
                        egui::FontId::proportional(8.5),
                        Color32::from_rgb(40, 40, 40),
                    );

                    // QP axis labels (0-1000 range, 5 grid lines)
                    let qp_max = 1000.0;
                    for i in 0..=num_grid_lines {
                        let y_pct = i as f32 / num_grid_lines as f32;
                        let y = graph_rect.bottom() - y_pct * graph_height;
                        let qp_value = (y_pct * qp_max) as usize;

                        painter.text(
                            egui::pos2(graph_rect.right() + 5.0, y),
                            egui::Align2::LEFT_CENTER,
                            format!("{}", qp_value),
                            egui::FontId::proportional(9.0),
                            Color32::from_rgb(90, 90, 90),
                        );
                    }

                    // **FIX 3: Draw moving average line only if enabled**
                    if self.show_moving_avg {
                        tracing::info!("📊 Drawing moving average line: {} points, y_max={}", moving_avg.len(), y_max);
                        if y_max > 0.0 {
                            let mut prev_point: Option<egui::Pos2> = None;
                            for (idx, &avg_size) in moving_avg.iter().enumerate() {
                                let x = graph_rect.left() + idx as f32 * (bar_width + bar_spacing) + bar_width / 2.0;
                                let y_ratio = (avg_size / y_max).clamp(0.0, 1.0);
                                let y = graph_rect.bottom() - (y_ratio * graph_height);
                                let point = egui::pos2(x, y);

                                if let Some(prev) = prev_point {
                                    // VQAnalyzer: thick purple line
                                    painter.line_segment(
                                        [prev, point],
                                        Stroke::new(2.5, Color32::from_rgb(130, 70, 200)),
                                    );
                                }
                                prev_point = Some(point);
                            }
                            tracing::info!("📊 Moving average line drawn");
                        }
                    }

                    // Draw frame size bars (VQAnalyzer: on top of line)
                    let mut visible_bars = 0;
                    for (idx, frame) in frames.iter().enumerate() {
                        // Apply frame type filter (VQAnalyzer parity)
                        // AV1 frame types: KEY, INTER, INTRA_ONLY, SWITCH
                        let is_i_frame = frame.frame_type == "KEY" || frame.frame_type == "INTRA_ONLY" || frame.frame_type == "SWITCH";
                        let is_p_frame = frame.frame_type == "INTER";
                        let is_b_frame = false; // AV1 doesn't have B-frames

                        let visible = (is_i_frame && self.show_i_frames)
                            || (is_p_frame && self.show_p_frames)
                            || (is_b_frame && self.show_b_frames);

                        if !visible {
                            continue; // Skip filtered frames
                        }

                        visible_bars += 1;

                        // Convert frame size to kbps (VQAnalyzer units)
                        let size_kbps = frame.size as f32 * 8.0 / 1000.0;
                        let bar_height = (size_kbps / y_max * graph_height).max(1.0);
                        let is_selected = selected_frame_index == Some(frame.frame_index);

                        let x = graph_rect.left() + idx as f32 * (bar_width + bar_spacing);
                        let bar_rect = Rect::from_min_size(
                            egui::pos2(x, graph_rect.bottom() - bar_height),
                            Vec2::new(bar_width, bar_height),
                        );

                        // Bar color (VQAnalyzer colors - slightly muted for bars)
                        let mut color = Self::frame_type_color(&frame.frame_type);
                        color = color.linear_multiply(0.85); // Slightly transparent for overlay effect

                        painter.rect_filled(bar_rect, Rounding::ZERO, color);

                        // Selection highlight (VQAnalyzer: subtle yellow outline)
                        if is_selected {
                            painter.rect_stroke(
                                bar_rect.expand(1.0),
                                Rounding::ZERO,
                                Stroke::new(2.0, Color32::from_rgb(255, 200, 0)),
                            );
                        }

                        // **FIX 3: Larger click area** - expand hit box for easier clicking
                        let click_rect = Rect::from_min_size(
                            egui::pos2(x - 2.0, graph_rect.top()), // Expand horizontally and full height
                            Vec2::new(bar_width + 4.0, graph_height),
                        );
                        let response = ui.interact(click_rect, ui.id().with(idx), Sense::click());
                        if response.clicked() {
                            result_command = Some(Command::SelectUnit {
                                stream: StreamId::A,
                                unit_key: frame.unit_key.clone(),
                            });
                        }

                        response.on_hover_ui(|ui| {
                            ui.label(format!("Frame {}", frame.frame_index));
                            ui.label(format!("Type: {}", frame.frame_type));
                            ui.label(format!("Size: {} bytes ({:.1} kbps)", frame.size, size_kbps));
                            ui.label(format!("Avg: {:.0} kbps", moving_avg[idx]));
                        });
                    }

                    tracing::info!("📊 Drew {} visible bars out of {} total frames", visible_bars, frames.len());
            });  // ScrollArea show() closes here

        // Floating legend (VQAnalyzer exact parity) - positioned relative to panel
        if self.show_legend {
            let legend_pos = egui::pos2(
                available_rect.right() - 165.0,  // Fixed: Use panel rect, not scrolled content
                available_rect.top() + 5.0,
            );

            egui::Area::new(egui::Id::new("frame_sizes_legend"))
                .default_pos(legend_pos)  // **FIX 1: default_pos instead of fixed_pos**
                .movable(true)             // **FIX 1: Allow dragging**
                .order(egui::Order::Foreground)
                .show(ui.ctx(), |ui| {
                    egui::Frame::none()
                        .fill(Color32::from_rgba_premultiplied(250, 250, 250, 240))
                        .stroke(Stroke::new(1.0, Color32::from_rgb(180, 180, 180)))
                        .rounding(Rounding::same(2.0))
                        .inner_margin(egui::Margin::same(6.0))
                        .show(ui, |ui| {
                            ui.set_width(145.0);

                            // Title bar with close button (VQAnalyzer style)
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new("Legend").strong().size(9.5));
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    if ui.small_button("✕").on_hover_text("Hide legend").clicked() {
                                        self.show_legend = false;
                                    }
                                });
                            });
                            ui.add_space(2.0);

                            // **FIX 3: Frame type legend with clickable toggles**
                            ui.horizontal(|ui| {
                                ui.colored_label(Color32::from_rgb(220, 80, 80), "■");
                                if ui.selectable_label(self.show_i_frames, egui::RichText::new("Intra slices").size(9.0)).clicked() {
                                    self.show_i_frames = !self.show_i_frames;
                                }
                            });
                            ui.horizontal(|ui| {
                                ui.colored_label(Color32::from_rgb(80, 180, 80), "■");
                                if ui.selectable_label(self.show_p_frames, egui::RichText::new("P slices").size(9.0)).clicked() {
                                    self.show_p_frames = !self.show_p_frames;
                                }
                            });
                            ui.horizontal(|ui| {
                                ui.colored_label(Color32::from_rgb(130, 70, 200), "■");
                                if ui.selectable_label(self.show_moving_avg, egui::RichText::new("Moving Avg").size(9.0)).clicked() {
                                    self.show_moving_avg = !self.show_moving_avg;
                                }
                            });

                            ui.add_space(6.0);
                            ui.separator();
                            ui.add_space(6.0);

                            // Moving Average parameters (VQAnalyzer exact layout)
                            ui.label(egui::RichText::new("Moving Average parameters").strong().size(9.0));
                            ui.add_space(3.0);
                            ui.horizontal(|ui| {
                                let mut temp_window = self.moving_avg_window.to_string();
                                let text_edit = egui::TextEdit::singleline(&mut temp_window)
                                    .desired_width(40.0);
                                if ui.add(text_edit).changed() {
                                    if let Ok(val) = temp_window.parse::<usize>() {
                                        self.moving_avg_window = val.clamp(1, 100);
                                    }
                                }
                            });
                            ui.horizontal(|ui| {
                                if ui.small_button("  OK  ").clicked() {
                                    // Accept value
                                }
                                if ui.small_button("Cancel").clicked() {
                                    self.moving_avg_window = 21;
                                }
                            });

                            ui.add_space(6.0);
                            ui.separator();
                            ui.add_space(6.0);

                            // Quality section (VQAnalyzer exact)
                            ui.label(egui::RichText::new("Quality").strong().size(9.0));
                            ui.add_space(2.0);
                            ui.label(egui::RichText::new(format!("Quality average length: {}", self.moving_avg_window)).size(8.5));
                            ui.label(egui::RichText::new("QP: --").size(8.5).color(Color32::DARK_GRAY));

                            ui.add_space(6.0);
                            ui.separator();
                            ui.add_space(4.0);

                            // Frame counts (VQAnalyzer exact)
                            ui.label(egui::RichText::new(format!("Total: {} frames", frames.len())).size(8.5));
                            ui.label(egui::RichText::new(format!("I: {} ({:.0}%)", i_frames.len(),
                                i_frames.len() as f32 / frames.len() as f32 * 100.0)).size(8.5));
                            ui.label(egui::RichText::new(format!("P: {} ({:.0}%)", p_frames.len(),
                                p_frames.len() as f32 / frames.len() as f32 * 100.0)).size(8.5));
                        });
                });
        } else {
            // Show legend button when hidden
            let button_pos = egui::pos2(
                available_rect.right() - 90.0,  // Fixed: Use panel rect
                available_rect.top() + 8.0,
            );
            egui::Area::new(egui::Id::new("frame_sizes_legend_button"))
                .fixed_pos(button_pos)
                .order(egui::Order::Foreground)
                .show(ui.ctx(), |ui| {
                    if ui.button("📊 Legend").clicked() {
                        self.show_legend = true;
                    }
                });
        }

        result_command
    }
}
