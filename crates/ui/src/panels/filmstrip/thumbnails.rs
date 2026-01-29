//! Thumbnail rendering - Frame thumbnail display with reference arrows

use super::{FilmstripPanel, FrameInfo};
use bitvue_core::{Command, StreamId};
use egui::{self, Color32, Rect, Rounding, Sense, Stroke, Vec2};

impl FilmstripPanel {
    pub(super) fn render_reference_arrows(
        &self,
        ui: &mut egui::Ui,
        frames: &[FrameInfo],
        thumb_total_width: f32,
        scroll_offset: f32,
    ) {
        let arrow_area_height = self.ref_arrow_height;
        let (rect, _response) = ui.allocate_exact_size(
            Vec2::new(ui.available_width(), arrow_area_height),
            Sense::hover(),
        );

        let painter = ui.painter();

        // Draw baseline
        let baseline_y = rect.center().y;
        painter.line_segment(
            [
                egui::pos2(rect.left(), baseline_y),
                egui::pos2(rect.right(), baseline_y),
            ],
            Stroke::new(1.0, Color32::from_rgb(100, 100, 100)),
        );

        // Calculate visible frame index range upfront to avoid processing off-screen frames
        let visible_start = (scroll_offset / thumb_total_width).floor() as usize;
        let visible_end = ((scroll_offset + rect.width()) / thumb_total_width).ceil() as usize + 1;
        let visible_end = visible_end.min(frames.len());

        // Draw reference arrows (simplified: show arrows from B/P frames to previous I/P frames)
        for idx in visible_start..visible_end {
            let frame = &frames[idx];
            if idx == 0 {
                continue;
            } // First frame has no reference

            // Skip intra frames (they don't reference other frames)
            if frame.frame_type == "KEY" || frame.nal_type == "IDR" || frame.nal_type == "KEY" {
                continue;
            }

            // Find the reference frame (simplified: previous I or P frame)
            let mut ref_idx = idx.saturating_sub(1);
            while ref_idx > 0 {
                let ref_frame = &frames[ref_idx];
                if ref_frame.frame_type != "B" && ref_frame.nal_type != "B" {
                    break;
                }
                ref_idx = ref_idx.saturating_sub(1);
            }

            // Calculate arrow positions
            let from_x = rect.left() + idx as f32 * thumb_total_width + thumb_total_width / 2.0
                - scroll_offset;
            let to_x = rect.left() + ref_idx as f32 * thumb_total_width + thumb_total_width / 2.0
                - scroll_offset;

            // Final visibility check (edge case protection)
            if from_x < rect.left() || from_x > rect.right() {
                continue;
            }
            if to_x < rect.left() || to_x > rect.right() {
                continue;
            }

            // Arrow color: orange for forward refs (L0)
            let arrow_color = Color32::from_rgb(255, 180, 80);

            // Draw arrow line
            painter.line_segment(
                [
                    egui::pos2(from_x, baseline_y - 3.0),
                    egui::pos2(to_x, baseline_y - 3.0),
                ],
                Stroke::new(1.5, arrow_color),
            );

            // Draw arrowhead pointing to reference
            let arrow_size = 4.0;
            let dir = if to_x < from_x { -1.0 } else { 1.0 };
            painter.line_segment(
                [
                    egui::pos2(to_x, baseline_y - 3.0),
                    egui::pos2(to_x - dir * arrow_size, baseline_y - 3.0 - arrow_size),
                ],
                Stroke::new(1.5, arrow_color),
            );
            painter.line_segment(
                [
                    egui::pos2(to_x, baseline_y - 3.0),
                    egui::pos2(to_x - dir * arrow_size, baseline_y - 3.0 + arrow_size),
                ],
                Stroke::new(1.5, arrow_color),
            );

            // Draw vertical tick at source frame
            painter.line_segment(
                [
                    egui::pos2(from_x, baseline_y),
                    egui::pos2(from_x, baseline_y - 6.0),
                ],
                Stroke::new(1.0, arrow_color),
            );
        }
    }

    /// Render B-Pyramid hierarchical GOP view (VQAnalyzer parity)
    pub(super) fn render_thumbnail(
        &self,
        ui: &mut egui::Ui,
        frame: &FrameInfo,
        texture_id: Option<egui::TextureId>,
        is_selected: bool,
    ) -> Option<Command> {
        let mut command: Option<Command> = None;

        let thumb_size = Vec2::new(self.thumb_width, self.thumb_height);
        let type_color = Self::frame_type_color(&frame.frame_type);

        ui.vertical(|ui| {
            // VQAnalyzer parity: Show "FRAME_TYPE-LAYER FRAME_NUM" format at top
            // e.g., "INTER-A 5" means Inter frame, layer A, frame number 5
            let layer = "A"; // TODO: Extract from actual layer info
            let header_text = format!("{}-{} {}", frame.frame_type, layer, frame.frame_index);
            ui.label(
                egui::RichText::new(header_text)
                    .small()
                    .strong()
                    .color(type_color),
            );

            // Thumbnail area (including border space if enabled)
            let border_offset = if self.show_type_borders {
                self.border_thickness
            } else {
                0.0
            };
            let total_size = Vec2::new(
                thumb_size.x + border_offset * 2.0,
                thumb_size.y + border_offset * 2.0,
            );
            let (outer_rect, response) = ui.allocate_exact_size(total_size, Sense::click());

            // VQAnalyzer-style: Full colored border around thumbnail based on frame type
            if self.show_type_borders {
                // Draw frame type border (I=red, P=green, B=blue)
                ui.painter()
                    .rect_filled(outer_rect, Rounding::same(3.0), type_color);
            }

            // Inner thumbnail rect (inside the border)
            let inner_rect = if self.show_type_borders {
                outer_rect.shrink(self.border_thickness)
            } else {
                outer_rect
            };

            // Background
            let bg_color = if is_selected {
                Color32::from_rgb(60, 60, 80)
            } else {
                Color32::from_rgb(30, 30, 35)
            };
            ui.painter()
                .rect_filled(inner_rect, Rounding::same(1.0), bg_color);

            // Thumbnail image or placeholder
            if let Some(tex_id) = texture_id {
                ui.painter().image(
                    tex_id,
                    inner_rect,
                    Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    Color32::WHITE,
                );
            } else {
                // Placeholder with frame type indicator
                ui.painter().rect_filled(
                    inner_rect.shrink(2.0),
                    Rounding::same(1.0),
                    type_color.linear_multiply(0.2),
                );
                ui.painter().text(
                    inner_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    &frame.frame_type,
                    egui::FontId::proportional(12.0),
                    type_color,
                );
            }

            // Bitvue unique feature: Error count badge (top-right corner)
            if frame.diagnostic_count > 0 {
                let badge_size = Vec2::new(20.0, 12.0);
                let badge_pos = egui::pos2(
                    inner_rect.right() - badge_size.x - 2.0,
                    inner_rect.top() + 2.0,
                );
                let badge_rect = Rect::from_min_size(badge_pos, badge_size);

                // Badge background color based on max impact
                let badge_bg = if frame.max_impact >= 80 {
                    Color32::from_rgb(220, 60, 60) // Critical - red
                } else if frame.max_impact >= 50 {
                    Color32::from_rgb(255, 160, 60) // Warning - orange
                } else {
                    Color32::from_rgb(255, 200, 80) // OK - yellow
                };

                // Draw badge background
                ui.painter()
                    .rect_filled(badge_rect, Rounding::same(3.0), badge_bg);

                // Draw border
                ui.painter().rect_stroke(
                    badge_rect,
                    Rounding::same(3.0),
                    Stroke::new(1.0, Color32::from_rgb(40, 40, 40)),
                );

                // Draw count text
                let badge_text = if frame.diagnostic_count > 99 {
                    "99+".to_string()
                } else {
                    frame.diagnostic_count.to_string()
                };
                ui.painter().text(
                    badge_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    badge_text,
                    egui::FontId::proportional(9.0),
                    Color32::WHITE,
                );
            }

            // Selection highlight - bright white overlay on top
            if is_selected {
                ui.painter().rect_stroke(
                    outer_rect.expand(1.0),
                    Rounding::same(4.0),
                    Stroke::new(3.0, Color32::from_rgb(255, 255, 255)),
                );
                // Inner glow effect
                ui.painter().rect_stroke(
                    outer_rect,
                    Rounding::same(3.0),
                    Stroke::new(1.0, Color32::from_rgb(255, 200, 100)),
                );
            }

            // Handle click
            if response.clicked() {
                command = Some(Command::SelectUnit {
                    stream: StreamId::A,
                    unit_key: frame.unit_key.clone(),
                });
            }

            // Tooltip on hover
            response.on_hover_ui(|ui| {
                ui.label(format!(
                    "Frame {} ({})",
                    frame.frame_index, frame.frame_type
                ));
                ui.label(format!("POC: {}", frame.poc));
                if let Some(pts) = frame.pts {
                    ui.label(format!("PTS: {}", pts));
                }
                if let Some(dts) = frame.dts {
                    ui.label(format!("DTS: {}", dts));
                }
                ui.label(format!("Offset: 0x{:X}", frame.offset));
                ui.label(format!("Size: {} bytes", frame.size));

                // Show diagnostic info if present
                if frame.diagnostic_count > 0 {
                    ui.separator();
                    ui.colored_label(
                        Color32::from_rgb(255, 180, 80),
                        format!("⚠ {} diagnostic(s)", frame.diagnostic_count),
                    );
                    ui.label(format!("Max impact: {}", frame.max_impact));
                    ui.label("Click to view details");
                }
            });

            // VQAnalyzer parity: Show NAL type below thumbnail (always visible)
            ui.label(
                egui::RichText::new(&frame.nal_type)
                    .small()
                    .color(Color32::from_rgb(180, 180, 180)),
            );

            // VQAnalyzer parity: Show "L0" indicator and "POC = X" format (optional)
            if self.show_poc {
                ui.horizontal(|ui| {
                    // Reference list indicator (L0/L1)
                    if let Some(ref_list) = &frame.ref_list {
                        ui.label(
                            egui::RichText::new(ref_list)
                                .small()
                                .color(Color32::from_rgb(150, 200, 255)),
                        );
                    }
                    // POC value (VQAnalyzer format: "POC = X")
                    ui.label(
                        egui::RichText::new(format!("POC = {}", frame.poc))
                            .small()
                            .color(Color32::from_rgb(200, 200, 200)),
                    );
                });
            }

            // PTS/DTS labels below POC (optional)
            if self.show_timestamps {
                if let Some(pts) = frame.pts {
                    ui.label(
                        egui::RichText::new(format!("P:{}", pts))
                            .small()
                            .color(Color32::from_rgb(150, 180, 150)),
                    );
                }
                if let Some(dts) = frame.dts {
                    ui.label(
                        egui::RichText::new(format!("D:{}", dts))
                            .small()
                            .color(Color32::from_rgb(150, 150, 180)),
                    );
                }
            }
        });

        // Add spacing
        ui.add_space(self.spacing);

        command
    }
}
