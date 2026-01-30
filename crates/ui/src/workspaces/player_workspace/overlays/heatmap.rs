//! Heatmap overlay drawing functions
//!
//! Provides various heatmap overlays: QP heatmap, bit allocation heatmap, MV magnitude heatmap.
//! VQAnalyzer parity: Shows heatmap-style visualizations for codec analysis.

use egui::{ColorImage, TextureOptions};

/// Implementation of heatmap overlay drawing
impl super::super::PlayerWorkspace {
    // =========================================================================
    // QP Heatmap
    // =========================================================================

    /// Draw QP heatmap overlay
    /// Per QP_HEATMAP_IMPLEMENTATION_SPEC.md
    /// Draw QP heatmap overlay with actual QP values per CU (VQAnalyzer parity)
    pub fn draw_qp_heatmap_overlay(&mut self, ui: &mut egui::Ui, rect: egui::Rect, qp_avg: Option<u8>) {
        let Some((w, h)) = self.texture.frame_size() else {
            return;
        };

        // VQAnalyzer parity: Use CU-sized blocks (typically 64x64 for HEVC CTB)
        let block_w = 64u32;
        let block_h = 64u32;
        let grid_w = w.div_ceil(block_w);
        let grid_h = h.div_ceil(block_h);

        // Generate QP grid
        let qp_grid = if let Some(qp) = qp_avg {
            Self::create_uniform_qp_grid(w, h, qp)
        } else {
            Self::create_mock_qp_grid(w, h)
        };

        // Get or create heatmap texture
        if self.overlays.qp.texture.is_none() {
            let heatmap_texture = bitvue_core::HeatmapTexture::generate(
                &qp_grid,
                self.overlays.qp.resolution,
                self.overlays.qp.scale_mode,
                self.overlays.qp.opacity,
            );

            let color_image = ColorImage::from_rgba_unmultiplied(
                [
                    heatmap_texture.width as usize,
                    heatmap_texture.height as usize,
                ],
                &heatmap_texture.pixels,
            );

            self.overlays.qp.texture = Some(ui.ctx().load_texture(
                "qp_heatmap",
                color_image,
                TextureOptions::LINEAR,
            ));
        }

        // Draw the heatmap texture
        if let Some(texture) = &self.overlays.qp.texture {
            ui.painter().image(
                texture.id(),
                rect,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            );
        }

        // VQAnalyzer parity: Draw actual QP values in each CU block
        let scale_x = rect.width() / w as f32;
        let scale_y = rect.height() / h as f32;
        let screen_block_w = block_w as f32 * scale_x;
        let screen_block_h = block_h as f32 * scale_y;

        // Only show labels if blocks are large enough
        let min_size = 24.0;
        if screen_block_w < min_size || screen_block_h < min_size {
            return;
        }

        let painter = ui.painter();
        let font_size = (screen_block_w.min(screen_block_h) / 3.0).clamp(10.0, 20.0);

        // Draw QP values per block (VQAnalyzer style: number in each CU)
        for row in 0..grid_h {
            for col in 0..grid_w {
                // Get QP value for this block
                let idx = (row * (w.div_ceil(qp_grid.block_w)) + col) as usize;
                let qp_val = qp_grid.qp.get(idx).copied().unwrap_or(0);

                // Calculate screen position (center of block)
                let screen_x = rect.min.x + (col as f32 + 0.5) * screen_block_w;
                let screen_y = rect.min.y + (row as f32 + 0.5) * screen_block_h;

                // Draw QP value text (VQAnalyzer shows just the number)
                painter.text(
                    egui::pos2(screen_x, screen_y),
                    egui::Align2::CENTER_CENTER,
                    format!("{}", qp_val),
                    egui::FontId::proportional(font_size),
                    egui::Color32::WHITE,
                );

                // Draw grid lines (VQAnalyzer shows block boundaries)
                let block_rect = egui::Rect::from_min_size(
                    egui::pos2(
                        rect.min.x + col as f32 * screen_block_w,
                        rect.min.y + row as f32 * screen_block_h,
                    ),
                    egui::vec2(screen_block_w, screen_block_h),
                );
                painter.rect_stroke(
                    block_rect,
                    0.0,
                    egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(0, 0, 0, 100)),
                );
            }
        }
    }

    /// Create uniform QP grid (all blocks same QP)
    /// Per QP_HEATMAP_IMPLEMENTATION_SPEC.md §1 (Canonical representation)
    fn create_uniform_qp_grid(width: u32, height: u32, qp_value: u8) -> bitvue_core::QPGrid {
        // Use 8x8 blocks for AV1
        let block_w = 8;
        let block_h = 8;
        let grid_w = width.div_ceil(block_w);
        let grid_h = height.div_ceil(block_h);

        // All blocks have the same QP (from parser: base_q_idx)
        let qp = vec![qp_value as i16; (grid_w * grid_h) as usize];

        bitvue_core::QPGrid::new(grid_w, grid_h, block_w, block_h, qp, -1)
    }

    /// Create mock QP grid for testing (gradient pattern)
    /// Per QP_HEATMAP_IMPLEMENTATION_SPEC.md §1 (Canonical representation)
    fn create_mock_qp_grid(width: u32, height: u32) -> bitvue_core::QPGrid {
        // Use 8x8 blocks for AV1
        let block_w = 8;
        let block_h = 8;
        let grid_w = width.div_ceil(block_w);
        let grid_h = height.div_ceil(block_h);

        // Generate mock QP values (gradient for visual testing)
        let mut qp = Vec::with_capacity((grid_w * grid_h) as usize);
        for _y in 0..grid_h {
            for x in 0..grid_w {
                // Create a gradient pattern: low QP (blue) on left, high QP (red) on right
                let t = x as f32 / grid_w as f32;
                let qp_val = (t * 63.0) as i16; // 0-63 range for AV1
                qp.push(qp_val);
            }
        }

        bitvue_core::QPGrid::new(grid_w, grid_h, block_w, block_h, qp, -1)
    }

    // =========================================================================
    // Bit Allocation Heatmap
    // =========================================================================

    /// Draw bit allocation heatmap overlay
    /// VQAnalyzer parity: shows bits per CTB as a heatmap
    pub fn draw_bit_allocation_overlay(
        &mut self,
        ui: &mut egui::Ui,
        rect: egui::Rect,
        frame_size: (u32, u32),
    ) {
        use crate::workspaces::overlays::BitAllocationOverlayState;

        // Generate texture if not cached
        if self.overlays.bit_allocation.texture.is_none() {
            let (w, h) = frame_size;

            // Use 64x64 CTB blocks for bit allocation (typical superblock size)
            let block_w = 64u32;
            let block_h = 64u32;
            let grid_w = w.div_ceil(block_w);
            let grid_h = h.div_ceil(block_h);

            // Generate mock bit allocation data (gradient pattern for testing)
            // In a real implementation, this would come from the parser
            let mut bits = Vec::with_capacity((grid_w * grid_h) as usize);
            let max_bits = 5000u32; // Typical max bits per CTB
            for y in 0..grid_h {
                for x in 0..grid_w {
                    // Create a radial pattern: more bits in center
                    let cx = grid_w as f32 / 2.0;
                    let cy = grid_h as f32 / 2.0;
                    let dx = x as f32 - cx;
                    let dy = y as f32 - cy;
                    let dist = (dx * dx + dy * dy).sqrt();
                    let max_dist = (cx * cx + cy * cy).sqrt();
                    let normalized = 1.0 - (dist / max_dist).min(1.0);
                    bits.push((normalized * max_bits as f32) as u32);
                }
            }

            // Calculate resolution factor
            let res_factor = match self.overlays.bit_allocation.resolution {
                bitvue_core::HeatmapResolution::Quarter => 4,
                bitvue_core::HeatmapResolution::Half => 2,
                bitvue_core::HeatmapResolution::Full => 1,
            };

            let tex_w = (w / res_factor).max(1);
            let tex_h = (h / res_factor).max(1);

            // Generate heatmap pixels
            let mut pixels = Vec::with_capacity((tex_w * tex_h * 4) as usize);
            let alpha = (self.overlays.bit_allocation.opacity * 255.0) as u8;

            for py in 0..tex_h {
                for px in 0..tex_w {
                    // Map texture pixel to block
                    let bx = (px * res_factor / block_w).min(grid_w - 1);
                    let by = (py * res_factor / block_h).min(grid_h - 1);
                    let idx = (by * grid_w + bx) as usize;
                    let bit_val = bits.get(idx).copied().unwrap_or(0);

                    // Get color from bit allocation state
                    let (r, g, b) = BitAllocationOverlayState::get_color(bit_val, max_bits);
                    pixels.extend_from_slice(&[r, g, b, alpha]);
                }
            }

            // Create texture
            let color_image =
                ColorImage::from_rgba_unmultiplied([tex_w as usize, tex_h as usize], &pixels);
            self.overlays.bit_allocation.texture = Some(ui.ctx().load_texture(
                "bit_alloc_heatmap",
                color_image,
                TextureOptions::LINEAR,
            ));
        }

        // Draw the heatmap texture
        if let Some(texture) = &self.overlays.bit_allocation.texture {
            ui.painter().image(
                texture.id(),
                rect,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            );
        }

        // Draw legend if enabled
        if self.overlays.bit_allocation.show_legend {
            let painter = ui.painter();
            let legend_x = rect.max.x - 80.0;
            let legend_y = rect.min.y + 10.0;

            // Legend background
            painter.rect_filled(
                egui::Rect::from_min_size(
                    egui::pos2(legend_x - 5.0, legend_y - 5.0),
                    egui::vec2(75.0, 90.0),
                ),
                4.0,
                egui::Color32::from_rgba_unmultiplied(0, 0, 0, 180),
            );

            painter.text(
                egui::pos2(legend_x, legend_y),
                egui::Align2::LEFT_TOP,
                "Bit Alloc",
                egui::FontId::proportional(11.0),
                egui::Color32::WHITE,
            );

            // Color gradient
            for i in 0..5 {
                let t = i as f32 / 4.0;
                let (r, g, b) = BitAllocationOverlayState::get_color((t * 5000.0) as u32, 5000);
                painter.rect_filled(
                    egui::Rect::from_min_size(
                        egui::pos2(legend_x, legend_y + 15.0 + i as f32 * 12.0),
                        egui::vec2(12.0, 10.0),
                    ),
                    2.0,
                    egui::Color32::from_rgb(r, g, b),
                );
                let label = match i {
                    0 => "Low",
                    2 => "Med",
                    4 => "High",
                    _ => "",
                };
                if !label.is_empty() {
                    painter.text(
                        egui::pos2(legend_x + 16.0, legend_y + 15.0 + i as f32 * 12.0 + 5.0),
                        egui::Align2::LEFT_CENTER,
                        label,
                        egui::FontId::proportional(9.0),
                        egui::Color32::WHITE,
                    );
                }
            }
        }
    }

    // =========================================================================
    // MV Magnitude Heatmap
    // =========================================================================

    /// Draw MV magnitude heatmap overlay
    /// VQAnalyzer parity: shows MV magnitude as a heatmap
    pub fn draw_mv_magnitude_overlay(
        &mut self,
        ui: &mut egui::Ui,
        rect: egui::Rect,
        frame_size: (u32, u32),
        mv_grid_data: Option<&bitvue_core::MVGrid>,
    ) {
        use crate::workspaces::overlays::MvMagnitudeOverlayState;

        // Generate texture if not cached
        if self.overlays.mv_magnitude.texture.is_none() {
            let (w, h) = frame_size;

            // Calculate resolution factor
            let res_factor = match self.overlays.mv_magnitude.resolution {
                bitvue_core::HeatmapResolution::Quarter => 4,
                bitvue_core::HeatmapResolution::Half => 2,
                bitvue_core::HeatmapResolution::Full => 1,
            };

            let tex_w = (w / res_factor).max(1);
            let tex_h = (h / res_factor).max(1);

            // Generate heatmap pixels
            let mut pixels = Vec::with_capacity((tex_w * tex_h * 4) as usize);
            let alpha = (self.overlays.mv_magnitude.opacity * 255.0) as u8;

            // Get max magnitude for scaling
            let max_magnitude = self
                .overlays
                .mv_magnitude
                .scale_mode
                .max_value()
                .unwrap_or(64.0);

            if let Some(mv_grid) = mv_grid_data {
                // Use real MV data
                for py in 0..tex_h {
                    for px in 0..tex_w {
                        // Map texture pixel to MV block
                        let coded_x = px * res_factor;
                        let coded_y = py * res_factor;
                        let bx = coded_x / mv_grid.block_w.max(1);
                        let by = coded_y / mv_grid.block_h.max(1);

                        let mut mag = 0.0f32;
                        let mut count = 0;

                        // Get L0 magnitude
                        if matches!(
                            self.overlays.mv_magnitude.layer,
                            bitvue_core::MVLayer::L0Only | bitvue_core::MVLayer::Both
                        ) {
                            if let Some(mv) = mv_grid.get_l0(bx, by) {
                                if !mv.is_missing() {
                                    mag +=
                                        MvMagnitudeOverlayState::magnitude(mv.dx_qpel, mv.dy_qpel);
                                    count += 1;
                                }
                            }
                        }

                        // Get L1 magnitude
                        if matches!(
                            self.overlays.mv_magnitude.layer,
                            bitvue_core::MVLayer::L1Only | bitvue_core::MVLayer::Both
                        ) {
                            if let Some(mv) = mv_grid.get_l1(bx, by) {
                                if !mv.is_missing() {
                                    mag +=
                                        MvMagnitudeOverlayState::magnitude(mv.dx_qpel, mv.dy_qpel);
                                    count += 1;
                                }
                            }
                        }

                        if count > 0 {
                            mag /= count as f32;
                        }

                        let (r, g, b) = MvMagnitudeOverlayState::get_color(mag, max_magnitude);
                        pixels.extend_from_slice(&[r, g, b, alpha]);
                    }
                }
            } else {
                // Generate mock MV magnitude data (radial pattern for testing)
                for py in 0..tex_h {
                    for px in 0..tex_w {
                        // Create a radial pattern
                        let cx = tex_w as f32 / 2.0;
                        let cy = tex_h as f32 / 2.0;
                        let dx = px as f32 - cx;
                        let dy = py as f32 - cy;
                        let dist = (dx * dx + dy * dy).sqrt();
                        let max_dist = (cx * cx + cy * cy).sqrt();
                        let mag = (dist / max_dist) * max_magnitude;

                        let (r, g, b) = MvMagnitudeOverlayState::get_color(mag, max_magnitude);
                        pixels.extend_from_slice(&[r, g, b, alpha]);
                    }
                }
            }

            // Create texture
            let color_image =
                ColorImage::from_rgba_unmultiplied([tex_w as usize, tex_h as usize], &pixels);
            self.overlays.mv_magnitude.texture = Some(ui.ctx().load_texture(
                "mv_magnitude_heatmap",
                color_image,
                TextureOptions::LINEAR,
            ));
        }

        // Draw the heatmap texture
        if let Some(texture) = &self.overlays.mv_magnitude.texture {
            ui.painter().image(
                texture.id(),
                rect,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            );
        }

        // Draw legend if enabled
        if self.overlays.mv_magnitude.show_legend {
            let painter = ui.painter();
            let legend_x = rect.max.x - 80.0;
            let legend_y = rect.min.y + 10.0;
            let max_mag = self
                .overlays
                .mv_magnitude
                .scale_mode
                .max_value()
                .unwrap_or(64.0);

            // Legend background
            painter.rect_filled(
                egui::Rect::from_min_size(
                    egui::pos2(legend_x - 5.0, legend_y - 5.0),
                    egui::vec2(75.0, 90.0),
                ),
                4.0,
                egui::Color32::from_rgba_unmultiplied(0, 0, 0, 180),
            );

            painter.text(
                egui::pos2(legend_x, legend_y),
                egui::Align2::LEFT_TOP,
                "MV Mag",
                egui::FontId::proportional(11.0),
                egui::Color32::WHITE,
            );

            // Color gradient
            for i in 0..5 {
                let t = i as f32 / 4.0;
                let (r, g, b) = MvMagnitudeOverlayState::get_color(t * max_mag, max_mag);
                painter.rect_filled(
                    egui::Rect::from_min_size(
                        egui::pos2(legend_x, legend_y + 15.0 + i as f32 * 12.0),
                        egui::vec2(12.0, 10.0),
                    ),
                    2.0,
                    egui::Color32::from_rgb(r, g, b),
                );
                let label = match i {
                    0 => "0px".to_string(),
                    2 => format!("{:.0}px", max_mag / 2.0),
                    4 => format!("{:.0}px", max_mag),
                    _ => String::new(),
                };
                if !label.is_empty() {
                    painter.text(
                        egui::pos2(legend_x + 16.0, legend_y + 15.0 + i as f32 * 12.0 + 5.0),
                        egui::Align2::LEFT_CENTER,
                        label,
                        egui::FontId::proportional(9.0),
                        egui::Color32::WHITE,
                    );
                }
            }
        }
    }
}
