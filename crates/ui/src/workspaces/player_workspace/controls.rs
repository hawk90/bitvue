//! UI control panels for player workspace overlays
//!
//! Extracted from mod.rs to reduce file size. Each overlay type has its control panel.

impl super::PlayerWorkspace {
    /// Grid size control (when grid overlay is active)
    fn show_grid_control(&mut self, ui: &mut egui::Ui) {
        if !self.overlays.active.contains(&super::OverlayType::Grid) {
            return;
        }

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

    /// QP Heatmap controls (when QP overlay is active)
    fn show_qp_heatmap_control(&mut self, ui: &mut egui::Ui) {
        if !self.overlays.active.contains(&super::OverlayType::QpHeatmap) {
            return;
        }

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

    /// Partition controls (when Partition overlay is active)
    fn show_partition_control(&mut self, ui: &mut egui::Ui) {
        if !self.overlays.active.contains(&super::OverlayType::Partition) {
            return;
        }

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

    /// Motion Vector controls (when MV overlay is active)
    fn show_mv_control(&mut self, ui: &mut egui::Ui) {
        if !self.overlays.active.contains(&super::OverlayType::MotionVectors) {
            return;
        }

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

    /// Mode Labels controls (when Mode Labels overlay is active)
    fn show_mode_labels_control(&mut self, ui: &mut egui::Ui) {
        if !self.overlays.active.contains(&super::OverlayType::ModeLabels) {
            return;
        }

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

    /// Bit Allocation Heatmap controls (VQAnalyzer parity)
    fn show_bit_allocation_control(&mut self, ui: &mut egui::Ui) {
        if !self.overlays.active.contains(&super::OverlayType::BitAllocation) {
            return;
        }

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

    /// MV Magnitude Heatmap controls (VQAnalyzer parity)
    fn show_mv_magnitude_control(&mut self, ui: &mut egui::Ui) {
        if !self.overlays.active.contains(&super::OverlayType::MvMagnitude) {
            return;
        }

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

    /// PU Type Overlay controls (VQAnalyzer parity)
    fn show_pu_type_control(&mut self, ui: &mut egui::Ui) {
        if !self.overlays.active.contains(&super::OverlayType::PuType) {
            return;
        }

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

    /// Show all overlay control panels
    pub fn show_all_controls(&mut self, ui: &mut egui::Ui) {
        self.show_grid_control(ui);
        self.show_qp_heatmap_control(ui);
        self.show_partition_control(ui);
        self.show_mv_control(ui);
        self.show_mode_labels_control(ui);
        self.show_bit_allocation_control(ui);
        self.show_mv_magnitude_control(ui);
        self.show_pu_type_control(ui);
    }
}
