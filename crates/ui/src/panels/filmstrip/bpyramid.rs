//! B-Pyramid view - Hierarchical GOP structure visualization
//!
//! VQAnalyzer parity: Node-edge graph showing temporal layer hierarchy

use super::{FilmstripPanel, FrameInfo};
use bitvue_core::{Command, StreamId};
use egui::{self, Color32, Pos2, Rect, Rounding, Sense, Stroke, Vec2};

impl FilmstripPanel {
    /// Render B-Pyramid hierarchical GOP view (VQAnalyzer parity)
    pub(super) fn render_bpyramid_view(
        &self,
        ui: &mut egui::Ui,
        frames: &[FrameInfo],
        selected_frame_index: Option<usize>,
    ) -> Option<Command> {
        let mut result_command: Option<Command> = None;

        if frames.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.label("No frames");
            });
            return None;
        }

        tracing::info!("📊 B-Pyramid: Rendering {} frames", frames.len());

        // VQAnalyzer dimensions
        let node_radius = 15.0;
        let horizontal_spacing = 60.0;
        let vertical_spacing = 50.0;
        let max_temporal_layer = 4; // Typical GOP structure

        egui::ScrollArea::both()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let available_height = ui.available_height();
                let total_width = frames.len() as f32 * horizontal_spacing + 100.0;
                let total_height = (max_temporal_layer as f32 + 1.0) * vertical_spacing + 100.0;

                let (rect, _) = ui.allocate_exact_size(
                    Vec2::new(total_width, total_height.max(available_height)),
                    Sense::hover(),
                );

                let painter = ui.painter();

                // Background
                painter.rect_filled(rect, Rounding::ZERO, Color32::from_rgb(250, 250, 250));

                // Calculate node positions based on temporal layer
                // VQAnalyzer: I/P frames at bottom (layer 0), B-frames in pyramid above
                let mut node_positions: Vec<Pos2> = Vec::new();
                let base_y = rect.bottom() - 50.0; // Bottom of graph

                for (idx, frame) in frames.iter().enumerate() {
                    // Determine temporal layer based on frame type
                    // I-frame / P-frame: layer 0 (bottom)
                    // B-frame: layer 1+ (based on GOP structure)
                    let temporal_layer =
                        if frame.frame_type == bitvue_core::FrameType::Key
                            || frame.frame_type == bitvue_core::FrameType::IntraOnly
                        {
                            0
                        } else if frame.frame_type == bitvue_core::FrameType::Inter {
                            0 // P-frames at bottom
                        } else {
                            // B-frames: estimate layer based on position in GOP
                            // Simple heuristic: alternate layers
                            (idx % 3).min(max_temporal_layer)
                        };

                    let x = rect.left() + 50.0 + idx as f32 * horizontal_spacing;
                    let y = base_y - temporal_layer as f32 * vertical_spacing;
                    node_positions.push(Pos2::new(x, y));
                }

                // Draw reference arrows (VQAnalyzer: blue arrows from B/P to reference frames)
                for (idx, frame) in frames.iter().enumerate() {
                    if idx == 0 {
                        continue; // First frame has no reference
                    }

                    // Skip I-frames (they don't reference other frames)
                    if frame.frame_type == bitvue_core::FrameType::Key
                        || frame.frame_type == bitvue_core::FrameType::IntraOnly
                    {
                        continue;
                    }

                    // Find reference frame (simple: previous I/P frame)
                    let mut ref_idx = idx.saturating_sub(1);
                    while ref_idx > 0 {
                        let ref_frame = &frames[ref_idx];
                        if ref_frame.frame_type != bitvue_core::FrameType::BFrame {
                            break;
                        }
                        ref_idx = ref_idx.saturating_sub(1);
                    }

                    // Draw arrow from current frame to reference
                    let from_pos = node_positions[idx];
                    let to_pos = node_positions[ref_idx];

                    // Arrow line
                    painter.line_segment(
                        [from_pos, to_pos],
                        Stroke::new(1.5, Color32::from_rgb(100, 150, 200)),
                    );

                    // Arrowhead at reference end
                    let arrow_dir = (to_pos - from_pos).normalized();
                    let arrow_base = to_pos - arrow_dir * node_radius;
                    let arrow_size = 6.0;
                    let perp = Vec2::new(-arrow_dir.y, arrow_dir.x);

                    painter.line_segment(
                        [
                            arrow_base,
                            arrow_base - arrow_dir * arrow_size + perp * arrow_size * 0.5,
                        ],
                        Stroke::new(1.5, Color32::from_rgb(100, 150, 200)),
                    );
                    painter.line_segment(
                        [
                            arrow_base,
                            arrow_base - arrow_dir * arrow_size - perp * arrow_size * 0.5,
                        ],
                        Stroke::new(1.5, Color32::from_rgb(100, 150, 200)),
                    );
                }

                // Draw nodes (VQAnalyzer: colored circles with frame numbers)
                for (idx, frame) in frames.iter().enumerate() {
                    let pos = node_positions[idx];
                    let is_selected = selected_frame_index == Some(frame.frame_index);

                    // Node color based on frame type
                    let node_color = match frame.frame_type.as_str() {
                        "KEY" | "INTRA_ONLY" => Color32::from_rgb(220, 100, 100), // Red for I-frames
                        "INTER" => Color32::from_rgb(100, 200, 100), // Green for P-frames
                        _ => Color32::from_rgb(150, 150, 220),       // Blue for B-frames
                    };

                    // Draw circle
                    painter.circle_filled(pos, node_radius, node_color);

                    // Selection highlight
                    if is_selected {
                        painter.circle_stroke(
                            pos,
                            node_radius + 3.0,
                            Stroke::new(3.0, Color32::from_rgb(255, 200, 0)),
                        );
                    }

                    // Border
                    painter.circle_stroke(
                        pos,
                        node_radius,
                        Stroke::new(1.5, Color32::from_rgb(80, 80, 80)),
                    );

                    // Frame number inside circle
                    painter.text(
                        pos,
                        egui::Align2::CENTER_CENTER,
                        format!("{}", frame.frame_index),
                        egui::FontId::proportional(11.0),
                        Color32::WHITE,
                    );

                    // Click interaction
                    let click_rect = Rect::from_center_size(pos, Vec2::splat(node_radius * 2.0));
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
                        ui.label(format!("POC: {}", frame.poc));
                        ui.label(format!("Size: {} bytes", frame.size));
                    });
                }
            });

        result_command
    }
}
