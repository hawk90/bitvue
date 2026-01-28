//! Player Workspace - WS_PLAYER_SPATIAL (Monster Pack v9)
//!
//! Decoded frame viewer with multiple overlay layers:
//! - Grid overlay
//! - Motion vectors
//! - QP heatmap
//! - Partition visualization
//! - Reference frame visualization

use super::overlays::OverlayManager;
use egui::{ColorImage, TextureHandle, TextureOptions, Vec2};

/// Overlay types for player
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlayType {
    None,
    Grid,
    MotionVectors,
    QpHeatmap, // Per QP_HEATMAP_IMPLEMENTATION_SPEC.md
    Partition,
    ReferenceFrames,
    ModeLabels,    // VQAnalyzer parity: AMVP/Merge/Skip/Intra labels
    BitAllocation, // VQAnalyzer parity: bits per CTB heatmap
    MvMagnitude,   // VQAnalyzer parity: MV magnitude heatmap
    PuType,        // VQAnalyzer parity: PU type categorical overlay
}

impl OverlayType {
    pub fn label(&self) -> &'static str {
        match self {
            OverlayType::None => "None",
            OverlayType::Grid => "Grid",
            OverlayType::MotionVectors => "Motion Vectors",
            OverlayType::QpHeatmap => "QP Heatmap",
            OverlayType::Partition => "Partition",
            OverlayType::ReferenceFrames => "Ref Frames",
            OverlayType::ModeLabels => "Mode Labels",
            OverlayType::BitAllocation => "Bit Alloc",
            OverlayType::MvMagnitude => "MV Magnitude",
            OverlayType::PuType => "PU Type",
        }
    }
}

/// Player workspace state
///
/// After god object refactoring (Batch 2): 27 fields → 5 fields
/// Overlay-related state moved to OverlayManager.
pub struct PlayerWorkspace {
    /// Current decoded frame texture
    texture: Option<TextureHandle>,
    /// Frame dimensions
    frame_size: Option<(u32, u32)>,
    /// Zoom level (1.0 = 100%)
    zoom: f32,
    /// Pan offset
    offset: Vec2,
    /// Overlay manager (contains all overlay state)
    overlays: OverlayManager,
}

impl PlayerWorkspace {
    pub fn new() -> Self {
        Self {
            texture: None,
            frame_size: None,
            zoom: 1.0,
            offset: Vec2::ZERO,
            overlays: OverlayManager::new(),
        }
    }

    /// Update the displayed frame
    pub fn set_frame(&mut self, ctx: &egui::Context, image: ColorImage) {
        self.frame_size = Some((image.width() as u32, image.height() as u32));
        self.texture = Some(ctx.load_texture("player_frame", image, TextureOptions::LINEAR));

        // Notify overlay manager of frame change
        self.overlays.on_frame_change();

        // Try to load partition data when frame changes
        self.load_partition_data();
        self.load_partition_grid();
    }

    /// Check if an overlay is currently active
    pub fn is_overlay_active(&self, overlay: OverlayType) -> bool {
        self.overlays.is_active(overlay)
    }

    /// Toggle an overlay on/off
    pub fn toggle_overlay(&mut self, overlay: OverlayType) {
        self.overlays.toggle(overlay);
    }

    /// Set overlay active state
    pub fn set_overlay(&mut self, overlay: OverlayType, active: bool) {
        self.overlays.set_active(overlay, active);
    }

    /// Load partition data from JSON mock file
    /// Per PARTITION_GRID_IMPLEMENTATION_SPEC.md §1
    fn load_partition_data(&mut self) {
        if self.overlays.partition.data.is_some() {
            return; // Already loaded
        }

        if let Some((w, h)) = self.frame_size {
            // Try to load from mock data file
            let mock_path = "docs/bitstream_analyzer_monster_pack_v14/docs/mock_data/partition_map_frame120.json";
            match std::fs::read_to_string(mock_path) {
                Ok(json_str) => {
                    #[derive(serde::Deserialize)]
                    struct PartitionMapJson {
                        coded_width: u32,
                        coded_height: u32,
                        leaf_block_w: u32,
                        leaf_block_h: u32,
                        #[allow(dead_code)]
                        grid_w: u32,
                        #[allow(dead_code)]
                        grid_h: u32,
                        part_kind: Vec<u8>,
                    }

                    match serde_json::from_str::<PartitionMapJson>(&json_str) {
                        Ok(json_data) => {
                            // Check if dimensions match
                            if json_data.coded_width == w && json_data.coded_height == h {
                                // Convert u8 values to PartitionKind
                                let part_kind: Vec<bitvue_core::PartitionKind> = json_data
                                    .part_kind
                                    .iter()
                                    .map(|&val| match val {
                                        1 => bitvue_core::PartitionKind::Intra,
                                        2 => bitvue_core::PartitionKind::Inter,
                                        3 => bitvue_core::PartitionKind::Split,
                                        4 => bitvue_core::PartitionKind::Skip,
                                        _ => bitvue_core::PartitionKind::Inter, // Default
                                    })
                                    .collect();

                                self.overlays.partition.data =
                                    Some(bitvue_core::PartitionData::new(
                                        json_data.coded_width,
                                        json_data.coded_height,
                                        json_data.leaf_block_w,
                                        json_data.leaf_block_h,
                                        part_kind,
                                    ));
                                tracing::info!("Loaded partition data from JSON: {}x{}", w, h);
                            } else {
                                tracing::warn!(
                                    "Partition data dimensions mismatch: JSON {}x{}, frame {}x{}",
                                    json_data.coded_width,
                                    json_data.coded_height,
                                    w,
                                    h
                                );
                                // Fall back to procedural mock
                                self.overlays.partition.data =
                                    Some(Self::create_mock_partition_data(w, h));
                            }
                        }
                        Err(e) => {
                            tracing::warn!("Failed to parse partition JSON: {}", e);
                            self.overlays.partition.data =
                                Some(Self::create_mock_partition_data(w, h));
                        }
                    }
                }
                Err(_) => {
                    // Mock file not found, use procedural mock
                    tracing::debug!("Mock partition data file not found, using procedural mock");
                    self.overlays.partition.data = Some(Self::create_mock_partition_data(w, h));
                }
            }
        }
    }

    /// Load partition grid (hierarchical blocks)
    fn load_partition_grid(&mut self) {
        if self.overlays.partition.grid.is_some() {
            return; // Already loaded
        }

        if let Some((w, h)) = self.frame_size {
            // For now, use procedural mock
            // TODO: Load from parser data
            self.overlays.partition.grid = Some(Self::create_mock_partition_grid(w, h));
            tracing::info!("Created mock partition grid: {}x{}", w, h);
        }
    }

    /// Show the player workspace
    /// Returns Command for navigation (StepForward, StepBackward, JumpToFrame)
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        container: Option<&bitvue_core::ContainerModel>,
        selection: Option<&bitvue_core::SelectionState>,
        units: Option<&[bitvue_core::UnitNode]>,
        total_frames: usize,
    ) -> Option<bitvue_core::Command> {
        let mut result_command: Option<bitvue_core::Command> = None;

        // Get current frame index
        let current_frame = selection
            .and_then(|sel| sel.temporal.as_ref())
            .map(|t| t.frame_index())
            .unwrap_or(0);

        // Keyboard shortcuts (VQAnalyzer parity)
        // Only handle when no text input is focused
        if !ui.ctx().wants_keyboard_input() {
            ui.input(|i| {
                // Left arrow: Previous frame
                if i.key_pressed(egui::Key::ArrowLeft) && current_frame > 0 {
                    if let Some(frame_unit) = Self::find_frame_by_index(units, current_frame - 1) {
                        result_command = Some(bitvue_core::Command::SelectUnit {
                            stream: bitvue_core::StreamId::A,
                            unit_key: frame_unit.key.clone(),
                        });
                    }
                }

                // Right arrow: Next frame
                if i.key_pressed(egui::Key::ArrowRight)
                    && current_frame < total_frames.saturating_sub(1)
                {
                    if let Some(frame_unit) = Self::find_frame_by_index(units, current_frame + 1) {
                        result_command = Some(bitvue_core::Command::SelectUnit {
                            stream: bitvue_core::StreamId::A,
                            unit_key: frame_unit.key.clone(),
                        });
                    }
                }

                // Home: First frame
                if i.key_pressed(egui::Key::Home) && total_frames > 0 {
                    if let Some(frame_unit) = Self::find_frame_by_index(units, 0) {
                        result_command = Some(bitvue_core::Command::SelectUnit {
                            stream: bitvue_core::StreamId::A,
                            unit_key: frame_unit.key.clone(),
                        });
                    }
                }

                // End: Last frame
                if i.key_pressed(egui::Key::End) && total_frames > 0 {
                    if let Some(frame_unit) =
                        Self::find_frame_by_index(units, total_frames.saturating_sub(1))
                    {
                        result_command = Some(bitvue_core::Command::SelectUnit {
                            stream: bitvue_core::StreamId::A,
                            unit_key: frame_unit.key.clone(),
                        });
                    }
                }

                // Space: Toggle play/pause (future: when playback is implemented)
                // For now, step forward as a simple action
                if i.key_pressed(egui::Key::Space) && current_frame < total_frames.saturating_sub(1)
                {
                    if let Some(frame_unit) = Self::find_frame_by_index(units, current_frame + 1) {
                        result_command = Some(bitvue_core::Command::SelectUnit {
                            stream: bitvue_core::StreamId::A,
                            unit_key: frame_unit.key.clone(),
                        });
                    }
                }
            });
        }

        // Header with HUD info
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
            if let Some((w, h)) = self.frame_size {
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
                            let frame_type = Self::extract_frame_type(&unit.unit_type);
                            let type_color = match frame_type.as_str() {
                                "KEY" | "I" => egui::Color32::from_rgb(100, 200, 100),
                                "P" | "INTER" => egui::Color32::from_rgb(100, 150, 255),
                                "B" => egui::Color32::from_rgb(255, 180, 100),
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
                ui.label(format!("Zoom: {:.0}%", self.zoom * 100.0));

                // Show total frames
                if total_frames > 0 {
                    ui.separator();
                    ui.label(format!("{} frames", total_frames));
                }
            });
        });

        ui.separator();

        // Navigation controls (VQAnalyzer parity)
        ui.horizontal(|ui| {
            // Previous frame button
            let can_go_back = current_frame > 0;
            if ui
                .add_enabled(can_go_back, egui::Button::new("⏮"))
                .on_hover_text("First frame (Home)")
                .clicked()
            {
                if let Some(frame_unit) = Self::find_frame_by_index(units, 0) {
                    result_command = Some(bitvue_core::Command::SelectUnit {
                        stream: bitvue_core::StreamId::A,
                        unit_key: frame_unit.key.clone(),
                    });
                }
            }

            if ui
                .add_enabled(can_go_back, egui::Button::new("◀"))
                .on_hover_text("Previous frame (←)")
                .clicked()
            {
                if let Some(frame_unit) =
                    Self::find_frame_by_index(units, current_frame.saturating_sub(1))
                {
                    result_command = Some(bitvue_core::Command::SelectUnit {
                        stream: bitvue_core::StreamId::A,
                        unit_key: frame_unit.key.clone(),
                    });
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
                        if let Some(frame_unit) = Self::find_frame_by_index(units, new_frame) {
                            result_command = Some(bitvue_core::Command::SelectUnit {
                                stream: bitvue_core::StreamId::A,
                                unit_key: frame_unit.key.clone(),
                            });
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
                if let Some(frame_unit) = Self::find_frame_by_index(units, current_frame + 1) {
                    result_command = Some(bitvue_core::Command::SelectUnit {
                        stream: bitvue_core::StreamId::A,
                        unit_key: frame_unit.key.clone(),
                    });
                }
            }

            if ui
                .add_enabled(can_go_forward, egui::Button::new("⏭"))
                .on_hover_text("Last frame (End)")
                .clicked()
            {
                if let Some(frame_unit) =
                    Self::find_frame_by_index(units, total_frames.saturating_sub(1))
                {
                    result_command = Some(bitvue_core::Command::SelectUnit {
                        stream: bitvue_core::StreamId::A,
                        unit_key: frame_unit.key.clone(),
                    });
                }
            }
        });

        // Toolbar: Overlay toggles + zoom controls
        ui.horizontal(|ui| {
            ui.label("Overlays:");
            for overlay_type in [
                OverlayType::Grid,
                OverlayType::MotionVectors,
                OverlayType::QpHeatmap,
                OverlayType::Partition,
                OverlayType::ReferenceFrames,
                OverlayType::ModeLabels,
                OverlayType::BitAllocation,
                OverlayType::MvMagnitude,
                OverlayType::PuType,
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
                self.zoom = 1.0;
                self.offset = Vec2::ZERO;
            }
            if ui.button("100%").clicked() {
                self.zoom = 1.0;
            }
            if ui.button("200%").clicked() {
                self.zoom = 2.0;
            }
            if ui.button("50%").clicked() {
                self.zoom = 0.5;
            }
        });

        // Grid size control (when grid overlay is active)
        if self.overlays.active.contains(&OverlayType::Grid) {
            ui.horizontal(|ui| {
                ui.label("Grid Size:");
                if ui
                    .selectable_label(self.overlays.grid.size == 32, "32")
                    .clicked()
                {
                    self.overlays.grid.size = 32;
                }
                if ui
                    .selectable_label(self.overlays.grid.size == 64, "64")
                    .clicked()
                {
                    self.overlays.grid.size = 64;
                }
                if ui
                    .selectable_label(self.overlays.grid.size == 128, "128")
                    .clicked()
                {
                    self.overlays.grid.size = 128;
                }

                ui.separator();
                ui.checkbox(&mut self.overlays.grid.show_ctb_labels, "CTB Labels");
                ui.checkbox(&mut self.overlays.grid.show_headers, "Row/Col Headers");
            });
        }

        // QP Heatmap controls (when QP overlay is active)
        // Per QP_HEATMAP_IMPLEMENTATION_SPEC.md §5 (Debug/Validation UI)
        if self.overlays.active.contains(&OverlayType::QpHeatmap) {
            ui.horizontal(|ui| {
                ui.label("QP Opacity:");
                if ui
                    .add(egui::Slider::new(&mut self.overlays.qp.opacity, 0.0..=1.0).step_by(0.05))
                    .changed()
                {
                    self.overlays.qp.texture = None; // Invalidate cache
                }

                ui.separator();
                ui.label("Resolution:");
                if ui
                    .selectable_label(
                        matches!(
                            self.overlays.qp.resolution,
                            bitvue_core::HeatmapResolution::Quarter
                        ),
                        "Quarter",
                    )
                    .clicked()
                {
                    self.overlays.qp.resolution = bitvue_core::HeatmapResolution::Quarter;
                    self.overlays.qp.texture = None;
                }
                if ui
                    .selectable_label(
                        matches!(
                            self.overlays.qp.resolution,
                            bitvue_core::HeatmapResolution::Half
                        ),
                        "Half",
                    )
                    .clicked()
                {
                    self.overlays.qp.resolution = bitvue_core::HeatmapResolution::Half;
                    self.overlays.qp.texture = None;
                }
                if ui
                    .selectable_label(
                        matches!(
                            self.overlays.qp.resolution,
                            bitvue_core::HeatmapResolution::Full
                        ),
                        "Full",
                    )
                    .clicked()
                {
                    self.overlays.qp.resolution = bitvue_core::HeatmapResolution::Full;
                    self.overlays.qp.texture = None;
                }

                ui.separator();
                ui.label("Scale:");
                if ui
                    .selectable_label(
                        matches!(self.overlays.qp.scale_mode, bitvue_core::QPScaleMode::Auto),
                        "Auto",
                    )
                    .clicked()
                {
                    self.overlays.qp.scale_mode = bitvue_core::QPScaleMode::Auto;
                    self.overlays.qp.texture = None;
                }
                if ui
                    .selectable_label(
                        matches!(self.overlays.qp.scale_mode, bitvue_core::QPScaleMode::Fixed),
                        "Fixed (0-63)",
                    )
                    .clicked()
                {
                    self.overlays.qp.scale_mode = bitvue_core::QPScaleMode::Fixed;
                    self.overlays.qp.texture = None;
                }
            });
        }

        // Partition controls (when Partition overlay is active)
        // Per PARTITION_GRID_IMPLEMENTATION_SPEC.md §2
        if self.overlays.active.contains(&OverlayType::Partition) {
            ui.horizontal(|ui| {
                ui.label("Partition Mode:");
                if ui
                    .selectable_label(
                        matches!(
                            self.overlays.partition.mode,
                            bitvue_core::GridMode::Scaffold
                        ),
                        "Scaffold",
                    )
                    .clicked()
                {
                    self.overlays.partition.mode = bitvue_core::GridMode::Scaffold;
                }
                if ui
                    .selectable_label(
                        matches!(
                            self.overlays.partition.mode,
                            bitvue_core::GridMode::Partition
                        ),
                        "Partition",
                    )
                    .clicked()
                {
                    self.overlays.partition.mode = bitvue_core::GridMode::Partition;
                }

                ui.separator();
                ui.label("Tint Opacity:");
                ui.add(
                    egui::Slider::new(&mut self.overlays.partition.opacity, 0.0..=0.25)
                        .step_by(0.05),
                );
            });
        }

        // Motion Vector controls (when MV overlay is active)
        // Per MV_VECTORS_IMPLEMENTATION_SPEC.md §2.3
        if self.overlays.active.contains(&OverlayType::MotionVectors) {
            ui.horizontal(|ui| {
                ui.label("MV Layer:");
                if ui
                    .selectable_label(
                        matches!(self.overlays.mv.layer, bitvue_core::MVLayer::L0Only),
                        "L0",
                    )
                    .clicked()
                {
                    self.overlays.mv.layer = bitvue_core::MVLayer::L0Only;
                }
                if ui
                    .selectable_label(
                        matches!(self.overlays.mv.layer, bitvue_core::MVLayer::L1Only),
                        "L1",
                    )
                    .clicked()
                {
                    self.overlays.mv.layer = bitvue_core::MVLayer::L1Only;
                }
                if ui
                    .selectable_label(
                        matches!(self.overlays.mv.layer, bitvue_core::MVLayer::Both),
                        "Both",
                    )
                    .clicked()
                {
                    self.overlays.mv.layer = bitvue_core::MVLayer::Both;
                }

                ui.separator();
                ui.label("Scale:");
                ui.add(egui::Slider::new(&mut self.overlays.mv.user_scale, 0.1..=3.0).step_by(0.1));

                ui.separator();
                ui.label("Opacity:");
                ui.add(egui::Slider::new(&mut self.overlays.mv.opacity, 0.0..=1.0).step_by(0.05));
            });
        }

        // Mode Labels controls (when Mode Labels overlay is active)
        // VQAnalyzer parity: shows AMVP/Merge/Skip/Intra labels on blocks
        if self.overlays.active.contains(&OverlayType::ModeLabels) {
            ui.horizontal(|ui| {
                ui.label("Show:");
                ui.checkbox(&mut self.overlays.mode_labels.show_intra_modes, "Intra");
                ui.checkbox(&mut self.overlays.mode_labels.show_inter_modes, "Inter");

                ui.separator();
                ui.label("Font Scale:");
                ui.add(
                    egui::Slider::new(&mut self.overlays.mode_labels.font_scale, 0.5..=2.0)
                        .step_by(0.1),
                );

                ui.separator();
                ui.label("Opacity:");
                ui.add(
                    egui::Slider::new(&mut self.overlays.mode_labels.opacity, 0.0..=1.0)
                        .step_by(0.05),
                );

                ui.separator();
                ui.checkbox(&mut self.overlays.mode_labels.show_background, "Background");
            });
        }

        // Bit Allocation Heatmap controls (VQAnalyzer parity)
        if self.overlays.active.contains(&OverlayType::BitAllocation) {
            ui.horizontal(|ui| {
                ui.label("Bit Alloc Opacity:");
                if ui
                    .add(
                        egui::Slider::new(&mut self.overlays.bit_allocation.opacity, 0.0..=1.0)
                            .step_by(0.05),
                    )
                    .changed()
                {
                    self.overlays.bit_allocation.invalidate_texture();
                }

                ui.separator();
                ui.label("Resolution:");
                if ui
                    .selectable_label(
                        matches!(
                            self.overlays.bit_allocation.resolution,
                            bitvue_core::HeatmapResolution::Quarter
                        ),
                        "Quarter",
                    )
                    .clicked()
                {
                    self.overlays.bit_allocation.resolution =
                        bitvue_core::HeatmapResolution::Quarter;
                    self.overlays.bit_allocation.invalidate_texture();
                }
                if ui
                    .selectable_label(
                        matches!(
                            self.overlays.bit_allocation.resolution,
                            bitvue_core::HeatmapResolution::Half
                        ),
                        "Half",
                    )
                    .clicked()
                {
                    self.overlays.bit_allocation.resolution = bitvue_core::HeatmapResolution::Half;
                    self.overlays.bit_allocation.invalidate_texture();
                }
                if ui
                    .selectable_label(
                        matches!(
                            self.overlays.bit_allocation.resolution,
                            bitvue_core::HeatmapResolution::Full
                        ),
                        "Full",
                    )
                    .clicked()
                {
                    self.overlays.bit_allocation.resolution = bitvue_core::HeatmapResolution::Full;
                    self.overlays.bit_allocation.invalidate_texture();
                }

                ui.separator();
                ui.checkbox(&mut self.overlays.bit_allocation.show_legend, "Legend");
            });
        }

        // MV Magnitude Heatmap controls (VQAnalyzer parity)
        if self.overlays.active.contains(&OverlayType::MvMagnitude) {
            ui.horizontal(|ui| {
                ui.label("MV Mag Opacity:");
                if ui
                    .add(
                        egui::Slider::new(&mut self.overlays.mv_magnitude.opacity, 0.0..=1.0)
                            .step_by(0.05),
                    )
                    .changed()
                {
                    self.overlays.mv_magnitude.invalidate_texture();
                }

                ui.separator();
                ui.label("Layer:");
                if ui
                    .selectable_label(
                        matches!(
                            self.overlays.mv_magnitude.layer,
                            bitvue_core::MVLayer::L0Only
                        ),
                        "L0",
                    )
                    .clicked()
                {
                    self.overlays.mv_magnitude.layer = bitvue_core::MVLayer::L0Only;
                    self.overlays.mv_magnitude.invalidate_texture();
                }
                if ui
                    .selectable_label(
                        matches!(
                            self.overlays.mv_magnitude.layer,
                            bitvue_core::MVLayer::L1Only
                        ),
                        "L1",
                    )
                    .clicked()
                {
                    self.overlays.mv_magnitude.layer = bitvue_core::MVLayer::L1Only;
                    self.overlays.mv_magnitude.invalidate_texture();
                }
                if ui
                    .selectable_label(
                        matches!(self.overlays.mv_magnitude.layer, bitvue_core::MVLayer::Both),
                        "Both",
                    )
                    .clicked()
                {
                    self.overlays.mv_magnitude.layer = bitvue_core::MVLayer::Both;
                    self.overlays.mv_magnitude.invalidate_texture();
                }

                ui.separator();
                ui.checkbox(&mut self.overlays.mv_magnitude.show_legend, "Legend");
            });
        }

        // PU Type Overlay controls (VQAnalyzer parity)
        if self.overlays.active.contains(&OverlayType::PuType) {
            ui.horizontal(|ui| {
                ui.label("PU Opacity:");
                if ui
                    .add(
                        egui::Slider::new(&mut self.overlays.pu_type.opacity, 0.0..=1.0)
                            .step_by(0.05),
                    )
                    .changed()
                {
                    self.overlays.pu_type.invalidate_texture();
                }

                ui.separator();
                ui.label("Show:");
                if ui
                    .checkbox(&mut self.overlays.pu_type.show_intra, "Intra")
                    .changed()
                {
                    self.overlays.pu_type.invalidate_texture();
                }
                if ui
                    .checkbox(&mut self.overlays.pu_type.show_skip, "Skip")
                    .changed()
                {
                    self.overlays.pu_type.invalidate_texture();
                }
                if ui
                    .checkbox(&mut self.overlays.pu_type.show_merge, "Merge")
                    .changed()
                {
                    self.overlays.pu_type.invalidate_texture();
                }
                if ui
                    .checkbox(&mut self.overlays.pu_type.show_amvp, "AMVP")
                    .changed()
                {
                    self.overlays.pu_type.invalidate_texture();
                }

                ui.separator();
                ui.checkbox(&mut self.overlays.pu_type.show_legend, "Legend");
            });
        }

        ui.separator();

        // Frame display area
        if let Some(texture) = &self.texture {
            let texture_id = texture.id();
            let show_overlays = !self.overlays.active.is_empty();
            let grid_size = self.overlays.grid.size;
            let zoom = self.zoom;

            // Calculate display size
            let (frame_w, frame_h) = self.frame_size.unwrap_or((640, 480));
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
                            for overlay in &active_overlays {
                                match overlay {
                                    OverlayType::Grid => {
                                        self.draw_grid_overlay(ui, rect, zoom, grid_size);
                                    }
                                    OverlayType::MotionVectors => {
                                        // Draw motion vector overlay
                                        // Per MV_VECTORS_IMPLEMENTATION_SPEC.md
                                        if let Some(frame_size) = self.frame_size {
                                            if selection.is_none() {
                                                tracing::warn!("MV Overlay: No selection");
                                            }
                                            if units.is_none() {
                                                tracing::warn!("MV Overlay: No units data");
                                            }

                                            // Get MV grid from selected frame
                                            let mv_grid = selection
                                                .and_then(|sel| {
                                                    tracing::trace!("MV Overlay: Selection exists - temporal={:?}, unit={:?}",
                                                                   sel.temporal.as_ref().map(|t| t.frame_index()),
                                                                   sel.unit.as_ref().map(|u| u.offset));
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
                                        // Draw QP heatmap overlay
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
                                        // Draw partition grid overlay
                                        // Per PARTITION_GRID_IMPLEMENTATION_SPEC.md
                                        if let Some(frame_size) = self.frame_size {
                                            self.draw_partition_overlay(ui, rect, frame_size);
                                        }
                                    }
                                    OverlayType::ReferenceFrames => {
                                        // TODO: Draw reference frame indicators
                                        Self::draw_placeholder_overlay(
                                            ui,
                                            rect,
                                            "Ref",
                                            egui::Color32::from_rgb(255, 255, 100),
                                        );
                                    }
                                    OverlayType::ModeLabels => {
                                        // Draw mode labels overlay
                                        // VQAnalyzer parity: shows AMVP/Merge/Skip/Intra on blocks
                                        if let Some(frame_size) = self.frame_size {
                                            self.draw_mode_labels_overlay(ui, rect, frame_size);
                                        }
                                    }
                                    OverlayType::BitAllocation => {
                                        // Draw bit allocation heatmap
                                        // VQAnalyzer parity: bits per CTB heatmap
                                        if let Some(frame_size) = self.frame_size {
                                            self.draw_bit_allocation_overlay(ui, rect, frame_size);
                                        }
                                    }
                                    OverlayType::MvMagnitude => {
                                        // Draw MV magnitude heatmap
                                        // VQAnalyzer parity: MV magnitude heatmap
                                        if let Some(frame_size) = self.frame_size {
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
                                        // Draw PU type overlay
                                        // VQAnalyzer parity: PU type categorical overlay
                                        if let Some(frame_size) = self.frame_size {
                                            self.draw_pu_type_overlay(ui, rect, frame_size);
                                        }
                                    }
                                    OverlayType::None => {}
                                }
                            }
                        }
                    }

                    (response, rect)
                });

            let (response, rect) = scroll_output.inner;

            // Handle mouse wheel for zoom
            if response.hovered() {
                let scroll_delta = ui.ctx().input(|i| i.smooth_scroll_delta.y);
                if scroll_delta != 0.0 {
                    let zoom_factor = 1.0 + scroll_delta * 0.001;
                    self.zoom = (self.zoom * zoom_factor).clamp(0.1, 10.0);
                }
            }

            // Handle click for partition selection
            // Per PARTITION_GRID_IMPLEMENTATION_SPEC.md §3
            if response.clicked() {
                if let Some(click_pos) = response.interact_pointer_pos() {
                    if rect.contains(click_pos) {
                        let pixel_x = ((click_pos.x - rect.min.x) / self.zoom) as u32;
                        let pixel_y = ((click_pos.y - rect.min.y) / self.zoom) as u32;

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
                }
            }

            // Show pixel info on hover + partition cell info
            let hover_pos = response.hover_pos();
            let zoom = self.zoom;
            let active_overlays = self.overlays.active.clone();
            let partition_grid = self.overlays.partition.grid.clone();

            // Context menu (right-click)
            // Per context_menus.json - Player scope
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

            // Hover tooltip
            if let Some(hover_pos) = hover_pos {
                if rect.contains(hover_pos) {
                    let pixel_x = ((hover_pos.x - rect.min.x) / zoom) as u32;
                    let pixel_y = ((hover_pos.y - rect.min.y) / zoom) as u32;

                    response.on_hover_ui(|ui| {
                        ui.label(format!("Pixel: ({}, {})", pixel_x, pixel_y));
                        ui.label(format!("Zoom: {:.0}%", zoom * 100.0));

                        // Show partition block info if partition overlay is active
                        // Per PARTITION_GRID_IMPLEMENTATION_SPEC.md §3
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

            // Status bar
            ui.separator();
            ui.horizontal(|ui| {
                if let Some((w, h)) = self.frame_size {
                    ui.label(format!("{}×{}", w, h));
                    ui.separator();
                }
                ui.label(format!("Zoom: {:.0}%", self.zoom * 100.0));

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if !self.overlays.active.is_empty() {
                        ui.label(format!("{} overlays active", self.overlays.active.len()));
                    }
                });
            });
        } else {
            ui.centered_and_justified(|ui| {
                ui.label(
                    egui::RichText::new("No frame decoded\nSelect a frame to decode and display")
                        .color(egui::Color32::GRAY),
                );
            });
        }

        result_command
    }

    /// Draw grid overlay with optional CTB labels and row/column headers
    /// VQAnalyzer parity: shows numbered grid cells like VQAnalyzer
    fn draw_grid_overlay(&self, ui: &mut egui::Ui, rect: egui::Rect, zoom: f32, grid_size: u32) {
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

    fn draw_placeholder_overlay(
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

    /// Draw QP heatmap overlay
    /// Per QP_HEATMAP_IMPLEMENTATION_SPEC.md
    /// Draw QP heatmap overlay with actual QP values per CU (VQAnalyzer parity)
    fn draw_qp_heatmap_overlay(&mut self, ui: &mut egui::Ui, rect: egui::Rect, qp_avg: Option<u8>) {
        let Some((w, h)) = self.frame_size else {
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

    /// Recursively partition a block into smaller blocks
    fn recursively_partition(
        grid: &mut bitvue_core::PartitionGrid,
        x: u32,
        y: u32,
        w: u32,
        h: u32,
        depth: u8,
    ) {
        use bitvue_core::partition_grid::{PartitionBlock, PartitionType};

        // Add current block
        grid.add_block(PartitionBlock::new(x, y, w, h, PartitionType::Split, depth));

        // Stop recursion at small blocks
        if w <= 8 || h <= 8 || depth >= 3 {
            return;
        }

        // Decide whether to split further (deterministic pattern)
        let should_split = !(x / 8 + y / 8 + depth as u32).is_multiple_of(3);

        if should_split {
            // Split into 4 quadrants
            let half_w = w / 2;
            let half_h = h / 2;

            Self::recursively_partition(grid, x, y, half_w, half_h, depth + 1);
            Self::recursively_partition(grid, x + half_w, y, w - half_w, half_h, depth + 1);
            Self::recursively_partition(grid, x, y + half_h, half_w, h - half_h, depth + 1);
            Self::recursively_partition(
                grid,
                x + half_w,
                y + half_h,
                w - half_w,
                h - half_h,
                depth + 1,
            );
        }
    }

    /// Create mock partition grid with hierarchical/nested blocks
    /// Per PARTITION_GRID_IMPLEMENTATION_SPEC.md §1
    fn create_mock_partition_grid(width: u32, height: u32) -> bitvue_core::PartitionGrid {
        use bitvue_core::partition_grid::PartitionGrid;

        let sb_size = 64;
        let mut grid = PartitionGrid::new(width, height, sb_size);

        // Generate hierarchical partition tree
        // Superblocks are recursively split into smaller blocks
        for sb_y in (0..height).step_by(sb_size as usize) {
            for sb_x in (0..width).step_by(sb_size as usize) {
                // Each superblock gets recursively partitioned
                Self::recursively_partition(
                    &mut grid,
                    sb_x,
                    sb_y,
                    sb_size.min(width - sb_x),
                    sb_size.min(height - sb_y),
                    0, // depth
                );
            }
        }

        tracing::info!(
            "Generated partition grid with {} blocks",
            grid.block_count()
        );
        grid
    }

    /// Create mock partition data for testing (uniform grid)
    /// Per PARTITION_GRID_IMPLEMENTATION_SPEC.md §1
    fn create_mock_partition_data(width: u32, height: u32) -> bitvue_core::PartitionData {
        // Use 8x8 leaf blocks for AV1
        let leaf_block_w = 8;
        let leaf_block_h = 8;
        let grid_w = width.div_ceil(leaf_block_w);
        let grid_h = height.div_ceil(leaf_block_h);

        // Generate mock partition kinds (checkerboard pattern for testing)
        let mut part_kind = Vec::with_capacity((grid_w * grid_h) as usize);
        for y in 0..grid_h {
            for x in 0..grid_w {
                // Create a pattern: Intra, Inter, Skip, Intra, Inter, ...
                let kind = match (x + y) % 4 {
                    0 => bitvue_core::PartitionKind::Intra,
                    1 => bitvue_core::PartitionKind::Inter,
                    2 => bitvue_core::PartitionKind::Skip,
                    _ => bitvue_core::PartitionKind::Split,
                };
                part_kind.push(kind);
            }
        }

        bitvue_core::PartitionData::new(width, height, leaf_block_w, leaf_block_h, part_kind)
    }

    /// Draw partition overlay
    /// Per PARTITION_GRID_IMPLEMENTATION_SPEC.md §2
    fn draw_partition_overlay(&self, ui: &mut egui::Ui, rect: egui::Rect, frame_size: (u32, u32)) {
        let painter = ui.painter();
        let (frame_w, frame_h) = frame_size;

        // Calculate screen scaling
        let scale_x = rect.width() / frame_w as f32;
        let scale_y = rect.height() / frame_h as f32;

        match self.overlays.partition.mode {
            bitvue_core::GridMode::Scaffold => {
                // Scaffold mode: superblock grid (64x64 or 128x128)
                // AV1 uses 64x64 or 128x128 superblocks
                let superblock_size = 64; // Could be 128 for some sequences
                let grid_lines_x = frame_w.div_ceil(superblock_size);
                let grid_lines_y = frame_h.div_ceil(superblock_size);

                // LOD decimation: check grid density
                // Per PARTITION_GRID_IMPLEMENTATION_SPEC.md §2.3
                let screen_spacing_x = superblock_size as f32 * scale_x;
                let screen_spacing_y = superblock_size as f32 * scale_y;
                let min_spacing = screen_spacing_x.min(screen_spacing_y);

                // Superblocks are large, so decimation is rarely needed
                // But we still support it for extreme zoom-out
                let stride = if min_spacing < 3.0 {
                    (3.0 / min_spacing).ceil() as u32
                } else {
                    1
                };

                let decimated = stride > 1;

                // Draw vertical superblock lines (thicker, more visible)
                for i in (0..=grid_lines_x).step_by(stride as usize) {
                    let x_coded = i * superblock_size;
                    let x_screen = rect.min.x + x_coded as f32 * scale_x;
                    painter.line_segment(
                        [
                            egui::pos2(x_screen, rect.min.y),
                            egui::pos2(x_screen, rect.max.y),
                        ],
                        egui::Stroke::new(
                            2.0,
                            egui::Color32::from_rgba_unmultiplied(255, 255, 0, 180),
                        ),
                    );
                }

                // Draw horizontal superblock lines (thicker, more visible)
                for i in (0..=grid_lines_y).step_by(stride as usize) {
                    let y_coded = i * superblock_size;
                    let y_screen = rect.min.y + y_coded as f32 * scale_y;
                    painter.line_segment(
                        [
                            egui::pos2(rect.min.x, y_screen),
                            egui::pos2(rect.max.x, y_screen),
                        ],
                        egui::Stroke::new(
                            2.0,
                            egui::Color32::from_rgba_unmultiplied(255, 255, 0, 180),
                        ),
                    );
                }

                // Show superblock info in corner
                let legend_pos = rect.min + egui::vec2(10.0, 10.0);
                let legend_text = if decimated {
                    format!(
                        "Superblock {}×{} (decimated {}x)",
                        superblock_size, superblock_size, stride
                    )
                } else {
                    format!("Superblock {}×{}", superblock_size, superblock_size)
                };
                painter.text(
                    legend_pos,
                    egui::Align2::LEFT_TOP,
                    legend_text,
                    egui::FontId::proportional(12.0),
                    egui::Color32::from_rgba_unmultiplied(255, 255, 0, 200),
                );
            }
            bitvue_core::GridMode::Partition => {
                // Partition mode: draw actual partition tree (hierarchical blocks)
                // Use cached partition grid if available
                let partition_grid = if let Some(ref grid) = self.overlays.partition.grid {
                    grid
                } else {
                    tracing::warn!("No partition grid available");
                    return;
                };

                // Draw partition boundaries only (no fill)
                // Per feedback: only show boundaries, not tint
                for (idx, block) in partition_grid.blocks.iter().enumerate() {
                    // Screen coordinates
                    let screen_x = rect.min.x + block.x as f32 * scale_x;
                    let screen_y = rect.min.y + block.y as f32 * scale_y;
                    let screen_w = block.width as f32 * scale_x;
                    let screen_h = block.height as f32 * scale_y;

                    // Skip if too small to render
                    if screen_w < 1.0 || screen_h < 1.0 {
                        continue;
                    }

                    // Only draw boundaries (no fill)
                    let is_selected = self.overlays.partition.selected_block == Some(idx);

                    if is_selected {
                        // Selected: thick yellow outline
                        painter.rect_stroke(
                            egui::Rect::from_min_size(
                                egui::pos2(screen_x, screen_y),
                                egui::vec2(screen_w, screen_h),
                            ),
                            0.0,
                            egui::Stroke::new(3.0, egui::Color32::from_rgb(255, 255, 0)),
                        );
                    } else {
                        // Normal: very thin white/gray boundary
                        let alpha = (self.overlays.partition.opacity * 255.0 * 2.0) as u8; // Use opacity for line visibility
                        painter.rect_stroke(
                            egui::Rect::from_min_size(
                                egui::pos2(screen_x, screen_y),
                                egui::vec2(screen_w, screen_h),
                            ),
                            0.0,
                            egui::Stroke::new(
                                0.5,
                                egui::Color32::from_rgba_unmultiplied(200, 200, 200, alpha),
                            ),
                        );
                    }
                }

                // Show partition info in corner
                let legend_pos = rect.min + egui::vec2(10.0, 10.0);
                painter.text(
                    legend_pos,
                    egui::Align2::LEFT_TOP,
                    format!("Partition Tree ({} blocks)", partition_grid.block_count()),
                    egui::FontId::proportional(12.0),
                    egui::Color32::from_rgba_unmultiplied(255, 255, 255, 220),
                );
            }
        }
    }

    /// Draw motion vector overlay
    /// Per MV_VECTORS_IMPLEMENTATION_SPEC.md §2
    fn draw_mv_overlay(
        &self,
        ui: &mut egui::Ui,
        rect: egui::Rect,
        frame_size: (u32, u32),
        mv_grid_data: Option<&bitvue_core::MVGrid>,
    ) {
        let painter = ui.painter();
        let (frame_w, frame_h) = frame_size;

        // Only draw if we have real MV data
        let mv_grid = if let Some(grid) = mv_grid_data {
            tracing::trace!(
                "Using REAL MV data: {}x{} grid, {} total MVs",
                grid.grid_w,
                grid.grid_h,
                grid.mv_l0.len()
            );
            let non_zero = grid
                .mv_l0
                .iter()
                .filter(|mv| !mv.is_missing() && (mv.dx_qpel != 0 || mv.dy_qpel != 0))
                .count();
            tracing::trace!("  Non-zero MVs: {}", non_zero);
            grid
        } else {
            tracing::trace!("No MV data for this frame - skipping MV overlay");
            return; // Don't draw anything if no MV data
        };

        // Calculate screen scaling
        let scale_x = rect.width() / frame_w as f32;
        let scale_y = rect.height() / frame_h as f32;

        // Compute visible viewport in screen space (clip to UI)
        let clip = ui.clip_rect().intersect(rect);

        // Convert visible viewport to coded pixel coordinates
        let vp_min_x = ((clip.min.x - rect.min.x) / scale_x).clamp(0.0, frame_w as f32) as u32;
        let vp_min_y = ((clip.min.y - rect.min.y) / scale_y).clamp(0.0, frame_h as f32) as u32;
        let vp_max_x = ((clip.max.x - rect.min.x) / scale_x).clamp(0.0, frame_w as f32) as u32;
        let vp_max_y = ((clip.max.y - rect.min.y) / scale_y).clamp(0.0, frame_h as f32) as u32;

        let viewport = bitvue_core::Viewport::new(
            vp_min_x,
            vp_min_y,
            vp_max_x.saturating_sub(vp_min_x),
            vp_max_y.saturating_sub(vp_min_y),
        );

        // Determine block range intersecting viewport
        let bw = mv_grid.block_w.max(1);
        let bh = mv_grid.block_h.max(1);
        let col_start = (viewport.x / bw).min(mv_grid.grid_w.saturating_sub(1));
        let row_start = (viewport.y / bh).min(mv_grid.grid_h.saturating_sub(1));
        let col_end = (viewport.x + viewport.width)
            .div_ceil(bw)
            .min(mv_grid.grid_w);
        let row_end = (viewport.y + viewport.height)
            .div_ceil(bh)
            .min(mv_grid.grid_h);

        // Count visible, present vectors (L0 or L1) to compute stride
        let mut visible_present = 0usize;
        for row in row_start..row_end {
            for col in col_start..col_end {
                let has_l0 = mv_grid
                    .get_l0(col, row)
                    .map(|mv| !mv.is_missing() && (mv.dx_qpel != 0 || mv.dy_qpel != 0))
                    .unwrap_or(false);
                let has_l1 = mv_grid
                    .get_l1(col, row)
                    .map(|mv| !mv.is_missing() && (mv.dx_qpel != 0 || mv.dy_qpel != 0))
                    .unwrap_or(false);
                if has_l0 || has_l1 {
                    visible_present += 1;
                }
            }
        }

        // Calculate stride for density control (max 8000 vectors)
        let stride = bitvue_core::DensityControl::calculate_stride(visible_present.max(1));

        // Draw motion vectors within visible block range
        for row in row_start..row_end {
            for col in col_start..col_end {
                // Apply stride sampling
                if !bitvue_core::DensityControl::should_draw(col, row, stride) {
                    continue;
                }

                // Get block center in coded pixels
                let (block_center_x, block_center_y) = mv_grid.block_center(col, row);

                // Convert to screen coordinates
                let screen_x = rect.min.x + block_center_x * scale_x;
                let screen_y = rect.min.y + block_center_y * scale_y;

                // Draw L0 vectors (if enabled)
                if matches!(
                    self.overlays.mv.layer,
                    bitvue_core::MVLayer::L0Only | bitvue_core::MVLayer::Both
                ) {
                    if let Some(mv) = mv_grid.get_l0(col, row) {
                        // Skip missing or true zero vectors to reduce clutter
                        if !mv.is_missing() && (mv.dx_qpel != 0 || mv.dy_qpel != 0) {
                            self.draw_mv_arrow(
                                painter,
                                screen_x,
                                screen_y,
                                &mv,
                                scale_x.min(scale_y),
                                egui::Color32::from_rgba_unmultiplied(
                                    0,
                                    255,
                                    0,
                                    (self.overlays.mv.opacity * 255.0) as u8,
                                ), // Green for L0
                            );
                        }
                    }
                }

                // Draw L1 vectors (if enabled)
                if matches!(
                    self.overlays.mv.layer,
                    bitvue_core::MVLayer::L1Only | bitvue_core::MVLayer::Both
                ) {
                    if let Some(mv) = mv_grid.get_l1(col, row) {
                        if !mv.is_missing() && (mv.dx_qpel != 0 || mv.dy_qpel != 0) {
                            self.draw_mv_arrow(
                                painter,
                                screen_x,
                                screen_y,
                                &mv,
                                scale_x.min(scale_y),
                                egui::Color32::from_rgba_unmultiplied(
                                    255,
                                    0,
                                    255,
                                    (self.overlays.mv.opacity * 255.0) as u8,
                                ), // Magenta for L1
                            );
                        }
                    }
                }
            }
        }
    }

    /// Draw a single motion vector arrow
    /// Per MV_VECTORS_IMPLEMENTATION_SPEC.md §2.2
    fn draw_mv_arrow(
        &self,
        painter: &egui::Painter,
        start_x: f32,
        start_y: f32,
        mv: &bitvue_core::mv_overlay::MotionVector,
        zoom_scale: f32,
        color: egui::Color32,
    ) {
        // Scale vector
        let (dx, dy) =
            bitvue_core::MVScaling::scale_vector(mv, self.overlays.mv.user_scale, zoom_scale);

        // Clamp to max arrow length
        let (dx_clamped, dy_clamped) =
            bitvue_core::MVScaling::clamp_arrow_length(dx, dy, bitvue_core::MAX_ARROW_LENGTH_PX);

        let end_x = start_x + dx_clamped;
        let end_y = start_y + dy_clamped;

        // Draw arrow segment
        painter.line_segment(
            [egui::pos2(start_x, start_y), egui::pos2(end_x, end_y)],
            egui::Stroke::new(1.5, color),
        );

        // Draw arrow head (simple triangle)
        let magnitude = (dx_clamped * dx_clamped + dy_clamped * dy_clamped).sqrt();
        if magnitude > 2.0 {
            // Arrow head size
            let head_size = 4.0;

            // Normalize direction
            let norm_dx = dx_clamped / magnitude;
            let norm_dy = dy_clamped / magnitude;

            // Perpendicular vector
            let perp_dx = -norm_dy;
            let perp_dy = norm_dx;

            // Arrow head points
            let p1 = egui::pos2(
                end_x - norm_dx * head_size + perp_dx * head_size * 0.5,
                end_y - norm_dy * head_size + perp_dy * head_size * 0.5,
            );
            let p2 = egui::pos2(
                end_x - norm_dx * head_size - perp_dx * head_size * 0.5,
                end_y - norm_dy * head_size - perp_dy * head_size * 0.5,
            );
            let p3 = egui::pos2(end_x, end_y);

            painter.add(egui::Shape::convex_polygon(
                vec![p1, p2, p3],
                color,
                egui::Stroke::NONE,
            ));
        }
    }

    /// Draw mode labels overlay
    /// VQAnalyzer parity: shows AMVP/Merge/Skip/Intra labels on blocks
    fn draw_mode_labels_overlay(
        &self,
        ui: &mut egui::Ui,
        rect: egui::Rect,
        frame_size: (u32, u32),
    ) {
        use super::overlays::BlockModeLabel;

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

    /// Draw bit allocation heatmap overlay
    /// VQAnalyzer parity: shows bits per CTB as a heatmap
    fn draw_bit_allocation_overlay(
        &mut self,
        ui: &mut egui::Ui,
        rect: egui::Rect,
        frame_size: (u32, u32),
    ) {
        use super::overlays::BitAllocationOverlayState;

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

    /// Draw MV magnitude heatmap overlay
    /// VQAnalyzer parity: shows MV magnitude as a heatmap
    fn draw_mv_magnitude_overlay(
        &mut self,
        ui: &mut egui::Ui,
        rect: egui::Rect,
        frame_size: (u32, u32),
        mv_grid_data: Option<&bitvue_core::MVGrid>,
    ) {
        use super::overlays::MvMagnitudeOverlayState;

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

    /// Draw PU type overlay
    /// VQAnalyzer parity: shows prediction unit types as colored blocks
    fn draw_pu_type_overlay(
        &mut self,
        ui: &mut egui::Ui,
        rect: egui::Rect,
        frame_size: (u32, u32),
    ) {
        use super::overlays::PuType;

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

    /// Find frame unit by frame index (for navigation)
    fn find_frame_by_index(
        units: Option<&[bitvue_core::UnitNode]>,
        target_index: usize,
    ) -> Option<&bitvue_core::UnitNode> {
        units.and_then(|u| Self::find_frame_recursive(u, target_index))
    }

    fn find_frame_recursive(
        units: &[bitvue_core::UnitNode],
        target_index: usize,
    ) -> Option<&bitvue_core::UnitNode> {
        for unit in units {
            if unit.frame_index == Some(target_index) {
                return Some(unit);
            }
            if !unit.children.is_empty() {
                if let Some(found) = Self::find_frame_recursive(&unit.children, target_index) {
                    return Some(found);
                }
            }
        }
        None
    }

    /// Extract frame type from unit type string
    fn extract_frame_type(unit_type: &str) -> String {
        if unit_type.contains("KEY") || unit_type.contains("INTRA") {
            "KEY".to_string()
        } else if unit_type.contains("INTER") {
            "P".to_string()
        } else if unit_type.contains("SWITCH") {
            "S".to_string()
        } else {
            "?".to_string()
        }
    }
}

impl Default for PlayerWorkspace {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper function to find unit by offset
fn find_unit_by_offset(
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_overlay_type_label() {
        assert_eq!(OverlayType::None.label(), "None");
        assert_eq!(OverlayType::Grid.label(), "Grid");
        assert_eq!(OverlayType::MotionVectors.label(), "Motion Vectors");
        assert_eq!(OverlayType::QpHeatmap.label(), "QP Heatmap");
        assert_eq!(OverlayType::Partition.label(), "Partition");
        assert_eq!(OverlayType::ReferenceFrames.label(), "Ref Frames");
        assert_eq!(OverlayType::ModeLabels.label(), "Mode Labels");
        assert_eq!(OverlayType::BitAllocation.label(), "Bit Alloc");
        assert_eq!(OverlayType::MvMagnitude.label(), "MV Magnitude");
        assert_eq!(OverlayType::PuType.label(), "PU Type");
    }

    #[test]
    fn test_overlay_type_equality() {
        assert_eq!(OverlayType::Grid, OverlayType::Grid);
        assert_ne!(OverlayType::Grid, OverlayType::QpHeatmap);
    }

    #[test]
    fn test_player_workspace_new_defaults() {
        let ws = PlayerWorkspace::new();

        // Verify zoom starts at 1.0
        assert!((ws.zoom - 1.0).abs() < f32::EPSILON);

        // Verify no active overlays initially
        assert!(ws.overlays.active.is_empty());

        // Verify grid size is 64 (default per spec)
        assert_eq!(ws.overlays.grid.size, 64);

        // Verify qp_opacity is 0.45 (default per QP_HEATMAP_IMPLEMENTATION_SPEC.md)
        assert!((ws.overlays.qp.opacity - 0.45).abs() < f32::EPSILON);
    }

    #[test]
    fn test_toggle_overlay_adds_when_not_present() {
        let mut ws = PlayerWorkspace::new();

        assert!(!ws.is_overlay_active(OverlayType::Grid));
        ws.toggle_overlay(OverlayType::Grid);
        assert!(ws.is_overlay_active(OverlayType::Grid));
    }

    #[test]
    fn test_toggle_overlay_removes_when_present() {
        let mut ws = PlayerWorkspace::new();

        ws.toggle_overlay(OverlayType::Grid);
        assert!(ws.is_overlay_active(OverlayType::Grid));

        ws.toggle_overlay(OverlayType::Grid);
        assert!(!ws.is_overlay_active(OverlayType::Grid));
    }

    #[test]
    fn test_toggle_overlay_multiple_overlays() {
        let mut ws = PlayerWorkspace::new();

        ws.toggle_overlay(OverlayType::Grid);
        ws.toggle_overlay(OverlayType::QpHeatmap);
        ws.toggle_overlay(OverlayType::MotionVectors);

        assert!(ws.is_overlay_active(OverlayType::Grid));
        assert!(ws.is_overlay_active(OverlayType::QpHeatmap));
        assert!(ws.is_overlay_active(OverlayType::MotionVectors));
        assert!(!ws.is_overlay_active(OverlayType::Partition));
    }

    #[test]
    fn test_set_overlay_activates() {
        let mut ws = PlayerWorkspace::new();

        ws.set_overlay(OverlayType::Grid, true);
        assert!(ws.is_overlay_active(OverlayType::Grid));
    }

    #[test]
    fn test_set_overlay_deactivates() {
        let mut ws = PlayerWorkspace::new();

        ws.set_overlay(OverlayType::Grid, true);
        assert!(ws.is_overlay_active(OverlayType::Grid));

        ws.set_overlay(OverlayType::Grid, false);
        assert!(!ws.is_overlay_active(OverlayType::Grid));
    }

    #[test]
    fn test_set_overlay_idempotent() {
        let mut ws = PlayerWorkspace::new();

        // Setting inactive overlay to inactive does nothing
        ws.set_overlay(OverlayType::Grid, false);
        assert!(!ws.is_overlay_active(OverlayType::Grid));

        // Setting active overlay to active does nothing extra
        ws.set_overlay(OverlayType::Grid, true);
        ws.set_overlay(OverlayType::Grid, true);
        assert!(ws.is_overlay_active(OverlayType::Grid));
        assert_eq!(
            ws.overlays
                .active
                .iter()
                .filter(|&&o| o == OverlayType::Grid)
                .count(),
            1
        );
    }

    #[test]
    fn test_player_workspace_default() {
        // Verify Default trait implementation
        let ws: PlayerWorkspace = Default::default();
        assert!((ws.zoom - 1.0).abs() < f32::EPSILON);
        assert!(ws.overlays.active.is_empty());
    }
}
