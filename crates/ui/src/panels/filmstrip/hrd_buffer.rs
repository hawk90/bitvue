//! HRD Buffer view - Buffer fullness visualization

use super::{FilmstripPanel, FrameInfo};
use bitvue_core::{Command, StreamId};
use egui::{self, Color32, Rect, Rounding, Sense, Stroke, Vec2};

impl FilmstripPanel {
    pub(super) fn render_hrd_buffer_view(
        &self,
        ui: &mut egui::Ui,
        frames: &[FrameInfo],
        selected_frame_index: Option<usize>,
    ) -> Option<Command> {
        let mut result_command: Option<Command> = None;

        // Simulate HRD buffer fullness (simplified model)
        let mut buffer_levels: Vec<f32> = Vec::with_capacity(frames.len());
        let buffer_size = 1_000_000.0; // 1MB buffer
        let bitrate = 5_000_000.0; // 5Mbps
        let frame_rate = 30.0;
        let bits_per_frame = bitrate / frame_rate;
        let mut current_level = buffer_size * 0.5; // Start at 50%

        for (idx, frame) in frames.iter().enumerate() {
            // Add bits from bitrate
            current_level += bits_per_frame;

            // Subtract frame size (using offset differences as proxy)
            let frame_size = if idx + 1 < frames.len() {
                (frames[idx + 1].offset - frame.offset) as f32 * 8.0 // bytes to bits
            } else {
                bits_per_frame
            };
            current_level -= frame_size;

            // Clamp to buffer bounds
            current_level = current_level.clamp(0.0, buffer_size);
            buffer_levels.push(current_level / buffer_size); // Normalize to 0..1
        }

        egui::ScrollArea::horizontal()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let available_height = ui.available_height() - 20.0;
                let point_spacing = 10.0;

                // Draw graph background
                let (rect, _) = ui.allocate_exact_size(
                    Vec2::new(frames.len() as f32 * point_spacing, available_height),
                    Sense::hover(),
                );

                let painter = ui.painter();

                // Background
                painter.rect_filled(rect, Rounding::ZERO, Color32::from_rgb(20, 25, 30));

                // Grid lines at 25%, 50%, 75%
                for pct in [0.25, 0.5, 0.75] {
                    let y = rect.bottom() - pct * available_height;
                    painter.line_segment(
                        [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
                        Stroke::new(1.0, Color32::from_rgb(50, 50, 60)),
                    );
                    painter.text(
                        egui::pos2(rect.left() + 5.0, y),
                        egui::Align2::LEFT_CENTER,
                        format!("{}%", (pct * 100.0) as u32),
                        egui::FontId::proportional(9.0),
                        Color32::from_rgb(100, 100, 120),
                    );
                }

                // Draw buffer level line
                let mut prev_point: Option<egui::Pos2> = None;
                for (idx, &level) in buffer_levels.iter().enumerate() {
                    let x = rect.left() + idx as f32 * point_spacing + point_spacing / 2.0;
                    let y = rect.bottom() - level * available_height;
                    let point = egui::pos2(x, y);

                    // Line color: green if healthy, yellow if low, red if critical
                    let color = if level > 0.5 {
                        Color32::from_rgb(80, 200, 80)
                    } else if level > 0.2 {
                        Color32::from_rgb(200, 200, 80)
                    } else {
                        Color32::from_rgb(200, 80, 80)
                    };

                    // Draw line segment
                    if let Some(prev) = prev_point {
                        painter.line_segment([prev, point], Stroke::new(2.0, color));
                    }

                    // Draw frame type indicator at each point
                    let frame = &frames[idx];
                    let marker_color = Self::frame_type_color(&frame.frame_type);
                    if selected_frame_index == Some(frame.frame_index) {
                        painter.circle_filled(point, 5.0, Color32::WHITE);
                        painter.circle_filled(point, 3.0, marker_color);
                    } else {
                        painter.circle_filled(point, 3.0, marker_color);
                    }

                    prev_point = Some(point);
                }

                // Handle click on points
                for (idx, frame) in frames.iter().enumerate() {
                    let x = rect.left() + idx as f32 * point_spacing + point_spacing / 2.0;
                    let y = rect.bottom() - buffer_levels[idx] * available_height;
                    let click_rect = Rect::from_center_size(egui::pos2(x, y), Vec2::splat(10.0));

                    let point_response = ui.interact(click_rect, ui.id().with(("hrd_point", idx)), Sense::click());
                    if point_response.clicked() {
                        result_command = Some(Command::SelectUnit {
                            stream: StreamId::A,
                            unit_key: frame.unit_key.clone(),
                        });
                    }
                    point_response.on_hover_ui(|ui| {
                        ui.label(format!("Frame {} ({})", frame.frame_index, frame.frame_type));
                        ui.label(format!("Buffer: {:.1}%", buffer_levels[idx] * 100.0));
                    });
                }
            });

        result_command
    }
}
