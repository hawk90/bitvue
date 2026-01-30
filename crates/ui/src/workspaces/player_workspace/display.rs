//! Frame display area for player workspace
//!
//! Extracted from mod.rs to reduce file size. Contains the main frame
//! display, overlay rendering, mouse handling, context menu, and status bar.

use super::OverlayType;

impl super::PlayerWorkspace {
    /// Show frame display area with overlays
    pub fn show_frame_display(
        &mut self,
        ui: &mut egui::Ui,
        selection: Option<&bitvue_core::SelectionState>,
        units: Option<&[bitvue_core::UnitNode]>,
    ) -> Option<bitvue_core::Command> {
        if let Some(texture) = self.texture.texture() {
            let texture_id = texture.id();
            let show_overlays = !self.overlays.active.is_empty();
            let grid_size = self.overlays.grid.size;
            let zoom = self.zoom.zoom();

            // Calculate display size
            let (frame_w, frame_h) = self.texture.frame_size().unwrap_or((640, 480));
            let display_w = frame_w as f32 * zoom;
            let display_h = frame_h as f32 * zoom;

            // Clone active overlays to avoid borrow issues
            let active_overlays = self.overlays.active.clone();

            let scroll_output = egui::ScrollArea::both()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    let (rect, response) = ui.allocate_exact_size(
                        egui::vec2(display_w, display_h),
                        egui::Sense::click_and_drag(),
                    );

                    // Draw the frame
                    if ui.is_rect_visible(rect) {
                        ui.painter().image(
                            texture_id,
                            rect,
                            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                            egui::Color32::WHITE,
                        );

                        // Draw overlays
                        if show_overlays {
                            self.render_overlays(ui, rect, &active_overlays, selection, units, frame_w, frame_h);
                        }
                    }

                    (response, rect)
                });

            let (response, rect) = scroll_output.inner;

            // Handle mouse interactions
            let result_command = self.handle_mouse_interactions(ui, &response, rect, selection, units);

            // Show status bar
            self.show_status_bar(ui);

            result_command
        } else {
            // No frame loaded - show placeholder
            ui.centered_and_justified(|ui| {
                ui.label(
                    egui::RichText::new("No frame decoded\nSelect a frame to decode and display")
                        .color(egui::Color32::GRAY),
                );
            });
            None
        }
    }

    /// Render all active overlays
    fn render_overlays(
        &mut self,
        ui: &mut egui::Ui,
        rect: egui::Rect,
        active_overlays: &[OverlayType],
        selection: Option<&bitvue_core::SelectionState>,
        units: Option<&[bitvue_core::UnitNode]>,
        frame_w: u32,
        frame_h: u32,
    ) {
        use super::overlays::find_unit_by_offset;

        for overlay in active_overlays {
            match overlay {
                OverlayType::Grid => {
                    self.draw_grid_overlay(ui, rect, self.zoom.zoom(), self.overlays.grid.size);
                }
                OverlayType::MotionVectors => {
                    if let Some(frame_size) = self.texture.frame_size() {
                        if selection.is_none() {
                            tracing::warn!("MV Overlay: No selection");
                        }
                        if units.is_none() {
                            tracing::warn!("MV Overlay: No units data");
                        }

                        // Get MV grid from selected frame
                        let mv_grid = selection
                            .and_then(|sel| {
                                sel.unit.as_ref()
                            })
                            .and_then(|uk| {
                                units.and_then(|u| {
                                    let found = find_unit_by_offset(u, uk.offset);
                                    if let Some(node) = found {
                                        tracing::trace!("MV Overlay: Found unit at offset {}: type={}, has_mv={}",
                                                      uk.offset, node.unit_type, node.mv_grid.is_some());
                                    } else {
                                        tracing::warn!("MV Overlay: Unit not found at offset {}", uk.offset);
                                    }
                                    found.and_then(|node| node.mv_grid.as_ref())
                                })
                            });
                        self.draw_mv_overlay(ui, rect, frame_size, mv_grid);
                    }
                }
                OverlayType::QpHeatmap => {
                    // Get QP from selected frame
                    let qp_avg = selection
                        .and_then(|sel| sel.unit.as_ref())
                        .and_then(|uk| {
                            units.and_then(|u| {
                                find_unit_by_offset(u, uk.offset).and_then(|node| node.qp_avg)
                            })
                        });
                    self.draw_qp_heatmap_overlay(ui, rect, qp_avg);
                }
                OverlayType::Partition => {
                    if let Some(frame_size) = self.texture.frame_size() {
                        self.draw_partition_overlay(ui, rect, frame_size);
                    }
                }
                OverlayType::ReferenceFrames => {
                    Self::draw_placeholder_overlay(
                        ui,
                        rect,
                        "Ref",
                        egui::Color32::from_rgb(255, 255, 100),
                    );
                }
                OverlayType::ModeLabels => {
                    if let Some(frame_size) = self.texture.frame_size() {
                        self.draw_mode_labels_overlay(ui, rect, frame_size);
                    }
                }
                OverlayType::BitAllocation => {
                    if let Some(frame_size) = self.texture.frame_size() {
                        self.draw_bit_allocation_overlay(ui, rect, frame_size);
                    }
                }
                OverlayType::MvMagnitude => {
                    if let Some(frame_size) = self.texture.frame_size() {
                        let mv_grid = selection
                            .and_then(|sel| sel.unit.as_ref())
                            .and_then(|uk| {
                                units.and_then(|u| {
                                    find_unit_by_offset(u, uk.offset).and_then(|node| node.mv_grid.as_ref())
                                })
                            });
                        self.draw_mv_magnitude_overlay(ui, rect, frame_size, mv_grid);
                    }
                }
                OverlayType::PuType => {
                    if let Some(frame_size) = self.texture.frame_size() {
                        self.draw_pu_type_overlay(ui, rect, frame_size);
                    }
                }
                OverlayType::None => {}
            }
        }
    }

    /// Handle mouse interactions (zoom, click selection, context menu, hover tooltip)
    fn handle_mouse_interactions(
        &mut self,
        ui: &mut egui::Ui,
        response: &egui::Response,
        rect: egui::Rect,
        selection: Option<&bitvue_core::SelectionState>,
        units: Option<&[bitvue_core::UnitNode]>,
    ) -> Option<bitvue_core::Command> {
        let mut result_command = None;

        // Handle mouse wheel for zoom
        if response.hovered() {
            let scroll_delta = ui.ctx().input(|i| i.smooth_scroll_delta.y);
            if scroll_delta != 0.0 {
                let zoom_factor = 1.0 + scroll_delta * 0.001;
                self.zoom.adjust_zoom(zoom_factor);
            }
        }

        // Handle click for partition selection
        if response.clicked() {
            if let Some(click_pos) = response.interact_pointer_pos() {
                if rect.contains(click_pos) {
                    self.handle_partition_selection(click_pos, rect);
                }
            }
        }

        // Context menu
        result_command = self.show_context_menu(ui, response, selection, result_command);

        // Hover tooltip
        self.show_hover_tooltip(ui, response, rect);

        result_command
    }

    /// Handle partition block selection on click
    fn handle_partition_selection(&mut self, click_pos: egui::Pos2, rect: egui::Rect) {
        let pixel_x = ((click_pos.x - rect.min.x) / self.zoom.zoom()) as u32;
        let pixel_y = ((click_pos.y - rect.min.y) / self.zoom.zoom()) as u32;

        // Select partition block if partition overlay is active
        if self.overlays.active.contains(&OverlayType::Partition) {
            if let Some(ref partition_grid) = self.overlays.partition.grid {
                // Find all blocks at pixel position, then select the smallest (deepest)
                let mut candidates: Vec<(usize, &bitvue_core::PartitionBlock)> =
                    partition_grid
                        .blocks
                        .iter()
                        .enumerate()
                        .filter(|(_, b)| {
                            pixel_x >= b.x
                                && pixel_x < b.x + b.width
                                && pixel_y >= b.y
                                && pixel_y < b.y + b.height
                        })
                        .collect();

                // Sort by size (smallest first) and depth (deepest first)
                candidates.sort_by(|(_, a), (_, b)| {
                    let size_a = a.width * a.height;
                    let size_b = b.width * b.height;
                    size_a.cmp(&size_b).then(b.depth.cmp(&a.depth))
                });

                if let Some((idx, block)) = candidates.first() {
                    self.overlays.partition.selected_block = Some(*idx);
                    tracing::info!(
                        "Selected partition block: {}x{} at ({}, {}) depth={}",
                        block.width,
                        block.height,
                        block.x,
                        block.y,
                        block.depth
                    );
                }
            }
        }
    }

    /// Show context menu (right-click)
    fn show_context_menu(
        &mut self,
        ui: &mut egui::Ui,
        response: &egui::Response,
        selection: Option<&bitvue_core::SelectionState>,
        mut result_command: Option<bitvue_core::Command>,
    ) -> Option<bitvue_core::Command> {
        let has_selection = selection.map(|s| s.unit.is_some()).unwrap_or(false);

        response.clone().context_menu(|ui| {
            // Details toggle - guarded by has_selection
            if ui
                .add_enabled(has_selection, egui::Button::new("Details"))
                .on_disabled_hover_text("No selection")
                .clicked()
            {
                result_command = Some(bitvue_core::Command::ToggleDetailMode);
                ui.close_menu();
            }

            ui.separator();

            // Export Evidence Bundle - always available
            if ui.button("Export Evidence Bundle...").clicked() {
                result_command = Some(bitvue_core::Command::ExportEvidenceBundle {
                    stream: bitvue_core::StreamId::A,
                    path: std::path::PathBuf::from("."),
                });
                ui.close_menu();
            }

            // Copy Selection - guarded by has_selection
            if ui
                .add_enabled(has_selection, egui::Button::new("Copy Selection"))
                .on_disabled_hover_text("No selection")
                .clicked()
            {
                result_command = Some(bitvue_core::Command::CopySelection);
                ui.close_menu();
            }
        });

        result_command
    }

    /// Show hover tooltip with pixel info and partition block info
    fn show_hover_tooltip(&self, _ui: &egui::Ui, response: &egui::Response, rect: egui::Rect) {
        let hover_pos = response.hover_pos();
        let zoom = self.zoom.zoom();
        let active_overlays = self.overlays.active.clone();
        let partition_grid = self.overlays.partition.grid.clone();

        if let Some(hover_pos) = hover_pos {
            if rect.contains(hover_pos) {
                let pixel_x = ((hover_pos.x - rect.min.x) / zoom) as u32;
                let pixel_y = ((hover_pos.y - rect.min.y) / zoom) as u32;

                // Clone response to pass to on_hover_ui
                response.clone().on_hover_ui(|ui| {
                    ui.label(format!("Pixel: ({}, {})", pixel_x, pixel_y));
                    ui.label(format!("Zoom: {:.0}%", zoom * 100.0));

                    // Show partition block info if partition overlay is active
                    if active_overlays.contains(&OverlayType::Partition) {
                        if let Some(ref partition_grid) = partition_grid {
                            if let Some(block) = partition_grid.block_at(pixel_x, pixel_y) {
                                ui.separator();
                                ui.label(format!("Block: {}×{}", block.width, block.height));
                                ui.label(format!("Position: ({}, {})", block.x, block.y));
                                ui.label(format!("Partition: {:?}", block.partition));
                            }
                        }
                    }
                });
            }
        }
    }

    /// Show status bar with frame info and overlay count
    pub fn show_status_bar(&self, ui: &mut egui::Ui) {
        ui.separator();
        ui.horizontal(|ui| {
            if let Some((w, h)) = self.texture.frame_size() {
                ui.label(format!("{}×{}", w, h));
                ui.separator();
            }
            ui.label(self.zoom.zoom_percent());

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if !self.overlays.active.is_empty() {
                    ui.label(format!("{} overlays active", self.overlays.active.len()));
                }
            });
        });
    }
}
