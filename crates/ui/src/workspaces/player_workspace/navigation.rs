//! Navigation UI for player workspace
//!
//! Extracted from mod.rs to reduce file size. Contains keyboard shortcuts,
//! header with HUD info, navigation controls, and toolbar.

impl super::PlayerWorkspace {
    /// Handle keyboard shortcuts (VQAnalyzer parity)
    /// Returns a command if navigation action was triggered
    pub fn handle_keyboard_shortcuts(
        &mut self,
        ui: &egui::Ui,
        units: Option<&[bitvue_core::UnitNode]>,
        current_frame: usize,
        total_frames: usize,
    ) -> Option<bitvue_core::Command> {
        // Only handle when no text input is focused
        if !ui.ctx().wants_keyboard_input() {
            let mut result_command = None;

            ui.input(|i| {
                // Left arrow: Previous frame
                if i.key_pressed(egui::Key::ArrowLeft) {
                    if let Some(cmd) = self.navigation.previous_frame_command(units, current_frame) {
                        result_command = Some(cmd);
                    }
                }

                // Right arrow: Next frame
                if i.key_pressed(egui::Key::ArrowRight) {
                    if let Some(cmd) = self.navigation.next_frame_command(units, current_frame, total_frames) {
                        result_command = Some(cmd);
                    }
                }

                // Home: First frame
                if i.key_pressed(egui::Key::Home) && total_frames > 0 {
                    if let Some(cmd) = self.navigation.first_frame_command(units) {
                        result_command = Some(cmd);
                    }
                }

                // End: Last frame
                if i.key_pressed(egui::Key::End) && total_frames > 0 {
                    if let Some(cmd) = self.navigation.last_frame_command(units, total_frames) {
                        result_command = Some(cmd);
                    }
                }

                // Space: Toggle play/pause (future: when playback is implemented)
                // For now, step forward as a simple action
                if i.key_pressed(egui::Key::Space) {
                    if let Some(cmd) = self.navigation.next_frame_command(units, current_frame, total_frames) {
                        result_command = Some(cmd);
                    }
                }
            });

            result_command
        } else {
            None
        }
    }

    /// Show header with HUD info (codec, dimensions, frame index, type)
    pub fn show_header(
        &mut self,
        ui: &mut egui::Ui,
        container: Option<&bitvue_core::ContainerModel>,
        selection: Option<&bitvue_core::SelectionState>,
        units: Option<&[bitvue_core::UnitNode]>,
        total_frames: usize,
    ) {
        ui.horizontal(|ui| {
            ui.heading("Player");

            // Display codec
            if let Some(c) = container {
                ui.separator();
                ui.label(
                    egui::RichText::new(&c.codec)
                        .color(egui::Color32::from_rgb(100, 180, 255))
                        .strong(),
                );
            }

            // Display frame dimensions
            if let Some((w, h)) = self.texture.frame_size() {
                ui.separator();
                ui.label(format!("{}×{}", w, h));
            }

            // Display frame index and type
            if let Some(sel) = selection {
                if let Some(temporal) = &sel.temporal {
                    let frame_index = temporal.frame_index();
                    ui.separator();
                    ui.label(format!("Frame #{}", frame_index));

                    // Get frame type from units
                    if let Some(u) = units {
                        if let Some(unit) = Self::find_frame_by_index(Some(u), frame_index) {
                            let frame_type = super::NavigationManager::extract_frame_type(&unit.unit_type);
                            let type_color = match frame_type {
                                bitvue_core::FrameType::Key | bitvue_core::FrameType::IntraOnly => {
                                    egui::Color32::from_rgb(100, 200, 100)
                                }
                                bitvue_core::FrameType::Inter => egui::Color32::from_rgb(100, 150, 255),
                                bitvue_core::FrameType::BFrame => egui::Color32::from_rgb(255, 180, 100),
                                _ => egui::Color32::GRAY,
                            };
                            ui.label(
                                egui::RichText::new(format!("[{}]", frame_type))
                                    .color(type_color)
                                    .strong(),
                            );
                        }
                    }
                }
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(self.zoom.zoom_percent());

                // Show total frames
                if total_frames > 0 {
                    ui.separator();
                    ui.label(format!("{} frames", total_frames));
                }
            });
        });
    }

    /// Show navigation controls (first, prev, next, last buttons with frame input)
    pub fn show_navigation_controls(
        &mut self,
        ui: &mut egui::Ui,
        units: Option<&[bitvue_core::UnitNode]>,
        current_frame: usize,
        total_frames: usize,
    ) -> Option<bitvue_core::Command> {
        let mut result_command = None;

        ui.horizontal(|ui| {
            // Previous frame button
            let can_go_back = current_frame > 0;
            if ui
                .add_enabled(can_go_back, egui::Button::new("⏮"))
                .on_hover_text("First frame (Home)")
                .clicked()
            {
                if let Some(cmd) = self.navigation.first_frame_command(units) {
                    result_command = Some(cmd);
                }
            }

            if ui
                .add_enabled(can_go_back, egui::Button::new("◀"))
                .on_hover_text("Previous frame (←)")
                .clicked()
            {
                if let Some(cmd) = self.navigation.previous_frame_command(units, current_frame) {
                    result_command = Some(cmd);
                }
            }

            // Frame number display with input
            ui.label("Frame:");
            let mut frame_text = current_frame.to_string();
            let response = ui
                .add(
                    egui::TextEdit::singleline(&mut frame_text)
                        .desired_width(50.0)
                        .horizontal_align(egui::Align::Center),
                )
                .on_hover_text("Enter frame number, press Enter to jump");
            if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                if let Ok(new_frame) = frame_text.parse::<usize>() {
                    if new_frame < total_frames {
                        if let Some(cmd) = self.navigation.goto_frame_command(units, new_frame) {
                            result_command = Some(cmd);
                        }
                    }
                }
            }
            ui.label(format!("/ {}", total_frames.saturating_sub(1)));

            // Next frame button
            let can_go_forward = total_frames > 0 && current_frame < total_frames.saturating_sub(1);
            if ui
                .add_enabled(can_go_forward, egui::Button::new("▶"))
                .on_hover_text("Next frame (→ or Space)")
                .clicked()
            {
                if let Some(cmd) = self.navigation.next_frame_command(units, current_frame, total_frames) {
                    result_command = Some(cmd);
                }
            }

            if ui
                .add_enabled(can_go_forward, egui::Button::new("⏭"))
                .on_hover_text("Last frame (End)")
                .clicked()
            {
                if let Some(cmd) = self.navigation.last_frame_command(units, total_frames) {
                    result_command = Some(cmd);
                }
            }
        });

        result_command
    }

    /// Show toolbar: Overlay toggles + zoom controls
    pub fn show_toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Overlays:");
            for overlay_type in [
                super::OverlayType::Grid,
                super::OverlayType::MotionVectors,
                super::OverlayType::QpHeatmap,
                super::OverlayType::Partition,
                super::OverlayType::ReferenceFrames,
                super::OverlayType::ModeLabels,
                super::OverlayType::BitAllocation,
                super::OverlayType::MvMagnitude,
                super::OverlayType::PuType,
            ] {
                let mut is_active = self.overlays.active.contains(&overlay_type);
                if ui.checkbox(&mut is_active, overlay_type.label()).changed() {
                    if is_active {
                        if !self.overlays.active.contains(&overlay_type) {
                            self.overlays.active.push(overlay_type);
                        }
                    } else {
                        self.overlays.active.retain(|&o| o != overlay_type);
                    }
                }
            }

            ui.separator();
            ui.label("Zoom:");
            if ui.button("Fit").clicked() {
                self.zoom.reset();
            }
            if ui.button("100%").clicked() {
                self.zoom.set_zoom(1.0);
            }
            if ui.button("200%").clicked() {
                self.zoom.set_zoom(2.0);
            }
            if ui.button("50%").clicked() {
                self.zoom.set_zoom(0.5);
            }
        });
    }
}
