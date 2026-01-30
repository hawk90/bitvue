//! Grid overlay drawing functions
//!
//! Provides grid overlay with optional CTB labels and row/column headers.
//! VQAnalyzer parity: shows numbered grid cells like VQAnalyzer.

/// Implementation of grid overlay drawing
impl super::super::PlayerWorkspace {
    /// Draw grid overlay with optional CTB labels and row/column headers
    /// VQAnalyzer parity: shows numbered grid cells like VQAnalyzer
    pub fn draw_grid_overlay(&self, ui: &mut egui::Ui, rect: egui::Rect, zoom: f32, grid_size: u32) {
        let painter = ui.painter();
        let grid_size_scaled = grid_size as f32 * zoom;

        // Calculate grid dimensions
        let cols = ((rect.width() / grid_size_scaled).ceil() as u32).max(1);
        let rows = ((rect.height() / grid_size_scaled).ceil() as u32).max(1);

        // Header offset for row/col headers
        let header_offset = if self.overlays.grid.show_headers {
            20.0
        } else {
            0.0
        };

        // Draw row headers (left side)
        if self.overlays.grid.show_headers {
            for row in 0..rows {
                let y = rect.min.y + row as f32 * grid_size_scaled + grid_size_scaled / 2.0;
                let header_rect = egui::Rect::from_min_size(
                    egui::pos2(
                        rect.min.x - header_offset,
                        rect.min.y + row as f32 * grid_size_scaled,
                    ),
                    egui::vec2(header_offset - 2.0, grid_size_scaled),
                );

                // Background for header
                painter.rect_filled(
                    header_rect,
                    0.0,
                    egui::Color32::from_rgba_unmultiplied(40, 40, 50, 200),
                );

                // Row number
                painter.text(
                    egui::pos2(rect.min.x - header_offset / 2.0, y),
                    egui::Align2::CENTER_CENTER,
                    format!("{}", row),
                    egui::FontId::proportional(10.0),
                    egui::Color32::from_rgb(200, 200, 200),
                );
            }
        }

        // Draw column headers (top)
        if self.overlays.grid.show_headers {
            for col in 0..cols {
                let x = rect.min.x + col as f32 * grid_size_scaled + grid_size_scaled / 2.0;
                let header_rect = egui::Rect::from_min_size(
                    egui::pos2(
                        rect.min.x + col as f32 * grid_size_scaled,
                        rect.min.y - header_offset,
                    ),
                    egui::vec2(grid_size_scaled, header_offset - 2.0),
                );

                // Background for header
                painter.rect_filled(
                    header_rect,
                    0.0,
                    egui::Color32::from_rgba_unmultiplied(40, 40, 50, 200),
                );

                // Column number
                painter.text(
                    egui::pos2(x, rect.min.y - header_offset / 2.0),
                    egui::Align2::CENTER_CENTER,
                    format!("{}", col),
                    egui::FontId::proportional(10.0),
                    egui::Color32::from_rgb(200, 200, 200),
                );
            }
        }

        // Draw vertical lines
        let mut x = rect.min.x;
        while x <= rect.max.x {
            painter.line_segment(
                [egui::pos2(x, rect.min.y), egui::pos2(x, rect.max.y)],
                egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(255, 255, 0, 128)),
            );
            x += grid_size_scaled;
        }

        // Draw horizontal lines
        let mut y = rect.min.y;
        while y <= rect.max.y {
            painter.line_segment(
                [egui::pos2(rect.min.x, y), egui::pos2(rect.max.x, y)],
                egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(255, 255, 0, 128)),
            );
            y += grid_size_scaled;
        }

        // Draw CTB index labels inside each cell
        // VQAnalyzer parity: shows "CTB Idx X / CTB Addr Y / Subnet Z" format
        if self.overlays.grid.show_ctb_labels && grid_size_scaled >= 40.0 {
            // Only show labels if cells are large enough to read
            let font_size = (grid_size_scaled / 6.0).clamp(8.0, 12.0);
            let line_height = font_size + 2.0;
            let mut ctb_index = 0u32;

            for row in 0..rows {
                for col in 0..cols {
                    let cell_x = rect.min.x + col as f32 * grid_size_scaled;
                    let cell_y = rect.min.y + row as f32 * grid_size_scaled;
                    let center_x = cell_x + grid_size_scaled / 2.0;

                    // VQAnalyzer format: 3 lines of text
                    // Line 1: CTB Idx X
                    // Line 2: CTB Addr Y
                    // Line 3: Subnet Z
                    let ctb_addr = ctb_index; // In real impl, this comes from bitstream
                    let subnet = 0u32; // Subnet/slice index

                    // Semi-transparent background for readability
                    let bg_width = grid_size_scaled * 0.9;
                    let bg_height = line_height * 3.0 + 4.0;
                    let bg_y = cell_y + (grid_size_scaled - bg_height) / 2.0;
                    painter.rect_filled(
                        egui::Rect::from_min_size(
                            egui::pos2(cell_x + (grid_size_scaled - bg_width) / 2.0, bg_y),
                            egui::vec2(bg_width, bg_height),
                        ),
                        2.0,
                        egui::Color32::from_rgba_unmultiplied(0, 0, 0, 180),
                    );

                    // Line 1: CTB Idx
                    painter.text(
                        egui::pos2(center_x, bg_y + line_height * 0.5 + 2.0),
                        egui::Align2::CENTER_CENTER,
                        format!("CTB Idx {}", ctb_index),
                        egui::FontId::proportional(font_size),
                        egui::Color32::from_rgb(255, 255, 100),
                    );

                    // Line 2: CTB Addr
                    painter.text(
                        egui::pos2(center_x, bg_y + line_height * 1.5 + 2.0),
                        egui::Align2::CENTER_CENTER,
                        format!("CTB Addr {}", ctb_addr),
                        egui::FontId::proportional(font_size),
                        egui::Color32::from_rgb(200, 200, 255),
                    );

                    // Line 3: Subnet
                    painter.text(
                        egui::pos2(center_x, bg_y + line_height * 2.5 + 2.0),
                        egui::Align2::CENTER_CENTER,
                        format!("Subnet {}", subnet),
                        egui::FontId::proportional(font_size),
                        egui::Color32::from_rgb(200, 255, 200),
                    );

                    ctb_index += 1;
                }
            }
        } else if self.overlays.grid.show_ctb_labels && grid_size_scaled >= 24.0 {
            // Smaller cells: show only CTB index
            let font_size = (grid_size_scaled / 4.0).clamp(8.0, 14.0);
            let mut ctb_index = 0u32;

            for row in 0..rows {
                for col in 0..cols {
                    let cell_x = rect.min.x + col as f32 * grid_size_scaled;
                    let cell_y = rect.min.y + row as f32 * grid_size_scaled;
                    let center_x = cell_x + grid_size_scaled / 2.0;
                    let center_y = cell_y + grid_size_scaled / 2.0;

                    painter.rect_filled(
                        egui::Rect::from_center_size(
                            egui::pos2(center_x, center_y),
                            egui::vec2(font_size * 2.0, font_size + 4.0),
                        ),
                        2.0,
                        egui::Color32::from_rgba_unmultiplied(0, 0, 0, 160),
                    );

                    painter.text(
                        egui::pos2(center_x, center_y),
                        egui::Align2::CENTER_CENTER,
                        format!("{}", ctb_index),
                        egui::FontId::proportional(font_size),
                        egui::Color32::from_rgb(255, 255, 100),
                    );

                    ctb_index += 1;
                }
            }
        }
    }

    /// Draw placeholder overlay for unimplemented features
    pub fn draw_placeholder_overlay(
        ui: &mut egui::Ui,
        rect: egui::Rect,
        label: &str,
        color: egui::Color32,
    ) {
        // Placeholder: Draw text in corner to indicate overlay is active but not fully implemented
        let painter = ui.painter();
        let text_pos = rect.min + egui::vec2(10.0, 10.0);
        painter.text(
            text_pos,
            egui::Align2::LEFT_TOP,
            label,
            egui::FontId::proportional(14.0),
            color,
        );
    }
}
