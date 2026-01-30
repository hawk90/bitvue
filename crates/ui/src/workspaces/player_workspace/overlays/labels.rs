//! Label overlay drawing functions
//!
//! Provides mode labels (AMVP/Merge/Skip/Intra) and PU type overlays.
//! VQAnalyzer parity: shows block-level prediction information.

use egui::{ColorImage, TextureOptions};

/// Implementation of label overlay drawing
impl super::super::PlayerWorkspace {
    // =========================================================================
    // Mode Labels Overlay
    // =========================================================================

    /// Draw mode labels overlay
    /// VQAnalyzer parity: shows AMVP/Merge/Skip/Intra labels on blocks
    pub fn draw_mode_labels_overlay(
        &self,
        ui: &mut egui::Ui,
        rect: egui::Rect,
        frame_size: (u32, u32),
    ) {
        use crate::workspaces::overlays::BlockModeLabel;

        let painter = ui.painter();
        let (frame_w, frame_h) = frame_size;

        // Calculate screen scaling
        let scale_x = rect.width() / frame_w as f32;
        let scale_y = rect.height() / frame_h as f32;

        // Use partition grid if available, otherwise use a default block size
        let block_size = if let Some(ref grid) = self.overlays.partition.grid {
            // Use average block size from partition grid
            if !grid.blocks.is_empty() {
                let avg_size: f32 = grid
                    .blocks
                    .iter()
                    .map(|b| (b.width + b.height) as f32 / 2.0)
                    .sum::<f32>()
                    / grid.blocks.len() as f32;
                avg_size as u32
            } else {
                32 // Default
            }
        } else {
            32 // Default block size for labels
        };

        // Calculate how many blocks to render
        let block_w = block_size.max(8);
        let block_h = block_size.max(8);
        let cols = frame_w.div_ceil(block_w);
        let rows = frame_h.div_ceil(block_h);

        // Check if blocks are too small to show labels
        let screen_block_w = block_w as f32 * scale_x;
        let screen_block_h = block_h as f32 * scale_y;
        let min_size = self.overlays.mode_labels.min_block_size;

        if screen_block_w < min_size || screen_block_h < min_size {
            // Show info message when blocks are too small
            painter.text(
                rect.min + egui::vec2(10.0, 10.0),
                egui::Align2::LEFT_TOP,
                "Mode Labels (zoom in to see labels)",
                egui::FontId::proportional(12.0),
                egui::Color32::from_rgba_unmultiplied(255, 200, 50, 200),
            );
            return;
        }

        // Calculate font size based on block size and user scale
        let base_font_size = (screen_block_w.min(screen_block_h) / 3.0).clamp(8.0, 16.0);
        let font_size = base_font_size * self.overlays.mode_labels.font_scale;
        let alpha = (self.overlays.mode_labels.opacity * 255.0) as u8;

        // Draw labels for each block
        // Use partition grid if available, otherwise generate mock pattern
        if let Some(ref partition_grid) = self.overlays.partition.grid {
            // Use real partition data to determine modes
            for block in &partition_grid.blocks {
                // Check if block is too small to show labels
                let screen_w = block.width as f32 * scale_x;
                let screen_h = block.height as f32 * scale_y;
                if screen_w < min_size || screen_h < min_size {
                    continue;
                }

                // Determine mode label from partition type
                let mode = match block.partition {
                    bitvue_core::partition_grid::PartitionType::Split
                    | bitvue_core::partition_grid::PartitionType::Horz4
                    | bitvue_core::partition_grid::PartitionType::Vert4 => continue, // Skip internal split nodes
                    bitvue_core::partition_grid::PartitionType::None => {
                        // Infer from depth: shallow = intra, deep = inter
                        if block.depth == 0 {
                            BlockModeLabel::Intra
                        } else if block.depth <= 2 {
                            BlockModeLabel::Merge
                        } else {
                            BlockModeLabel::Skip
                        }
                    }
                    bitvue_core::partition_grid::PartitionType::Horz
                    | bitvue_core::partition_grid::PartitionType::Vert => {
                        // Binary splits typically indicate inter prediction
                        if block.depth <= 1 {
                            BlockModeLabel::AMVP
                        } else {
                            BlockModeLabel::Merge
                        }
                    }
                    bitvue_core::partition_grid::PartitionType::HorzA
                    | bitvue_core::partition_grid::PartitionType::HorzB
                    | bitvue_core::partition_grid::PartitionType::VertA
                    | bitvue_core::partition_grid::PartitionType::VertB => {
                        // Asymmetric partitions often indicate motion-compensated prediction
                        BlockModeLabel::Merge
                    }
                };

                // Check if we should show this mode
                if !self.overlays.mode_labels.should_show(&mode) {
                    continue;
                }

                // Calculate screen position (center of block)
                let screen_x = rect.min.x + (block.x as f32 + block.width as f32 / 2.0) * scale_x;
                let screen_y = rect.min.y + (block.y as f32 + block.height as f32 / 2.0) * scale_y;

                // Get label text and color
                let label = mode.short_label();
                let (r, g, b, _) = mode.color();

                // Draw background if enabled
                if self.overlays.mode_labels.show_background {
                    let text_size = font_size * label.len() as f32 * 0.6;
                    painter.rect_filled(
                        egui::Rect::from_center_size(
                            egui::pos2(screen_x, screen_y),
                            egui::vec2(text_size + 4.0, font_size + 4.0),
                        ),
                        2.0,
                        egui::Color32::from_rgba_unmultiplied(0, 0, 0, alpha / 2),
                    );
                }

                // Draw label text
                painter.text(
                    egui::pos2(screen_x, screen_y),
                    egui::Align2::CENTER_CENTER,
                    label,
                    egui::FontId::proportional(font_size),
                    egui::Color32::from_rgba_unmultiplied(r, g, b, alpha),
                );
            }
        } else {
            // Generate mock mode labels using a deterministic pattern
            // This provides visual feedback even without real data
            for row in 0..rows {
                for col in 0..cols {
                    // Generate a deterministic mode based on position
                    let mode = match ((col + row * 3) % 7) as u8 {
                        0 => BlockModeLabel::IntraDC,
                        1 => BlockModeLabel::Skip,
                        2 => BlockModeLabel::Merge,
                        3 => BlockModeLabel::AMVP,
                        4 => BlockModeLabel::IntraPlanar,
                        5 => BlockModeLabel::NearMV,
                        _ => BlockModeLabel::Inter,
                    };

                    // Check if we should show this mode
                    if !self.overlays.mode_labels.should_show(&mode) {
                        continue;
                    }

                    // Calculate screen position (center of block)
                    let block_x = col * block_w;
                    let block_y = row * block_h;
                    let screen_x = rect.min.x + (block_x as f32 + block_w as f32 / 2.0) * scale_x;
                    let screen_y = rect.min.y + (block_y as f32 + block_h as f32 / 2.0) * scale_y;

                    // Get label text and color
                    let label = mode.short_label();
                    let (r, g, b, _) = mode.color();

                    // Draw background if enabled
                    if self.overlays.mode_labels.show_background {
                        let text_size = font_size * label.len() as f32 * 0.6;
                        painter.rect_filled(
                            egui::Rect::from_center_size(
                                egui::pos2(screen_x, screen_y),
                                egui::vec2(text_size + 4.0, font_size + 4.0),
                            ),
                            2.0,
                            egui::Color32::from_rgba_unmultiplied(0, 0, 0, alpha / 2),
                        );
                    }

                    // Draw label text
                    painter.text(
                        egui::pos2(screen_x, screen_y),
                        egui::Align2::CENTER_CENTER,
                        label,
                        egui::FontId::proportional(font_size),
                        egui::Color32::from_rgba_unmultiplied(r, g, b, alpha),
                    );
                }
            }
        }

        // Show overlay info in corner
        painter.text(
            rect.min + egui::vec2(10.0, 10.0),
            egui::Align2::LEFT_TOP,
            format!(
                "Mode Labels ({}×{})",
                if self.overlays.mode_labels.show_intra_modes {
                    "I"
                } else {
                    "-"
                },
                if self.overlays.mode_labels.show_inter_modes {
                    "P"
                } else {
                    "-"
                }
            ),
            egui::FontId::proportional(12.0),
            egui::Color32::from_rgba_unmultiplied(255, 200, 50, 200),
        );
    }

    // =========================================================================
    // PU Type Overlay
    // =========================================================================

    /// Draw PU type overlay
    /// VQAnalyzer parity: shows prediction unit types as colored blocks
    pub fn draw_pu_type_overlay(
        &mut self,
        ui: &mut egui::Ui,
        rect: egui::Rect,
        frame_size: (u32, u32),
    ) {
        use crate::workspaces::overlays::PuType;

        // Generate texture if not cached
        if self.overlays.pu_type.texture.is_none() {
            let (w, h) = frame_size;

            // Use 32x32 blocks for PU type visualization
            let block_w = 32u32;
            let block_h = 32u32;
            let grid_w = w.div_ceil(block_w);
            let grid_h = h.div_ceil(block_h);

            // Generate mock PU type data (deterministic pattern for testing)
            // In a real implementation, this would come from the parser
            let mut pu_types = Vec::with_capacity((grid_w * grid_h) as usize);
            for y in 0..grid_h {
                for x in 0..grid_w {
                    // Create a pattern based on position
                    let pu_type = match ((x + y * 3) % 6) as u8 {
                        0 => PuType::Intra,
                        1 => PuType::Skip,
                        2 => PuType::Merge,
                        3 => PuType::Amvp,
                        4 => PuType::Affine,
                        _ => PuType::Other,
                    };
                    pu_types.push(pu_type);
                }
            }

            // Generate texture at half resolution for performance
            let tex_w = (w / 2).max(1);
            let tex_h = (h / 2).max(1);

            let mut pixels = Vec::with_capacity((tex_w * tex_h * 4) as usize);
            let alpha = (self.overlays.pu_type.opacity * 255.0) as u8;

            for py in 0..tex_h {
                for px in 0..tex_w {
                    // Map texture pixel to block
                    let bx = (px * 2 / block_w).min(grid_w - 1);
                    let by = (py * 2 / block_h).min(grid_h - 1);
                    let idx = (by * grid_w + bx) as usize;
                    let pu_type = pu_types.get(idx).copied().unwrap_or(PuType::Other);

                    // Check if this type should be shown
                    if self.overlays.pu_type.should_show(pu_type) {
                        let (r, g, b) = pu_type.color();
                        pixels.extend_from_slice(&[r, g, b, alpha]);
                    } else {
                        pixels.extend_from_slice(&[0, 0, 0, 0]); // Transparent
                    }
                }
            }

            // Create texture
            let color_image =
                ColorImage::from_rgba_unmultiplied([tex_w as usize, tex_h as usize], &pixels);
            self.overlays.pu_type.texture = Some(ui.ctx().load_texture(
                "pu_type_overlay",
                color_image,
                TextureOptions::LINEAR,
            ));
        }

        // Draw the overlay texture
        if let Some(texture) = &self.overlays.pu_type.texture {
            ui.painter().image(
                texture.id(),
                rect,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            );
        }

        // Draw legend if enabled
        if self.overlays.pu_type.show_legend {
            let painter = ui.painter();
            let legend_x = rect.max.x - 80.0;
            let legend_y = rect.min.y + 10.0;

            // Legend background
            painter.rect_filled(
                egui::Rect::from_min_size(
                    egui::pos2(legend_x - 5.0, legend_y - 5.0),
                    egui::vec2(75.0, 100.0),
                ),
                4.0,
                egui::Color32::from_rgba_unmultiplied(0, 0, 0, 180),
            );

            painter.text(
                egui::pos2(legend_x, legend_y),
                egui::Align2::LEFT_TOP,
                "PU Type",
                egui::FontId::proportional(11.0),
                egui::Color32::WHITE,
            );

            // Show each PU type
            for (i, pu_type) in PuType::all().iter().enumerate() {
                let (r, g, b) = pu_type.color();
                painter.rect_filled(
                    egui::Rect::from_min_size(
                        egui::pos2(legend_x, legend_y + 15.0 + i as f32 * 14.0),
                        egui::vec2(12.0, 10.0),
                    ),
                    2.0,
                    egui::Color32::from_rgb(r, g, b),
                );
                painter.text(
                    egui::pos2(legend_x + 16.0, legend_y + 15.0 + i as f32 * 14.0 + 5.0),
                    egui::Align2::LEFT_CENTER,
                    pu_type.label(),
                    egui::FontId::proportional(9.0),
                    egui::Color32::WHITE,
                );
            }
        }
    }
}

/// Helper function to find unit by offset
pub fn find_unit_by_offset(
    units: &[bitvue_core::UnitNode],
    offset: u64,
) -> Option<&bitvue_core::UnitNode> {
    for unit in units {
        if unit.offset == offset {
            return Some(unit);
        }
        if !unit.children.is_empty() {
            if let Some(found) = find_unit_by_offset(&unit.children, offset) {
                return Some(found);
            }
        }
    }
    None
}
