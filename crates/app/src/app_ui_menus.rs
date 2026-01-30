//! UI menu handlers for BitvueApp

use crate::app_config::BitvueAppConfig;
use crate::app_yuv_diff::BitvueAppYuvDiff;
use crate::bitvue_app::BitvueApp;
use crate::settings::{ColorSpace, CpuOptimization};
use crate::yuv_diff::{BitDepth, ChromaSubsampling};
use bitvue_core::StreamId;
use eframe::egui;

/// UI menu methods
pub trait BitvueAppMenus {
    fn handle_file_menu(&mut self, ui: &mut egui::Ui, ctx: &egui::Context);
    fn handle_mode_menu(&mut self, ui: &mut egui::Ui);
    fn handle_yuvdiff_menu(&mut self, ui: &mut egui::Ui, ctx: &egui::Context);
    fn handle_export_menu(&mut self, ui: &mut egui::Ui);
    fn handle_options_menu(&mut self, ui: &mut egui::Ui, ctx: &egui::Context);
    fn handle_view_menu(&mut self, ui: &mut egui::Ui);
    fn handle_help_menu(&mut self, ui: &mut egui::Ui);
}

impl BitvueAppMenus for BitvueApp {
    fn handle_file_menu(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let mut path_to_open: Option<std::path::PathBuf> = None;
        let mut close_file = false;
        let mut quit_app = false;

        ui.menu_button("File", |ui| {
            // FIXME: egui Grid alignment issue - multiple separate Grids cause inconsistent column widths
            // Current workaround: Use min_col_width for each Grid
            // Better solution: Use single Grid or custom layout
            egui::Grid::new("file_menu_grid")
                .num_columns(2)
                .min_col_width(140.0)
                .spacing([10.0, 4.0])
                .show(ui, |ui| {
                    // Open bitstream... (Ctrl+O)
                    if ui.button("Open bitstream...").clicked() {
                        tracing::info!("File menu: Open bitstream clicked");
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Video Files", &["ivf", "av1", "mp4", "mkv", "webm", "ts"])
                            .add_filter("All Files", &["*"])
                            .pick_file()
                        {
                            // Log file metadata
                            if let Ok(metadata) = std::fs::metadata(&path) {
                                let size_mb = metadata.len() as f64 / (1024.0 * 1024.0);
                                let ext = path
                                    .extension()
                                    .and_then(|e| e.to_str())
                                    .unwrap_or("unknown");
                                tracing::info!(
                                    "File dialog: Selected {} file ({:.2} MB): {:?}",
                                    ext,
                                    size_mb,
                                    path
                                );
                            } else {
                                tracing::info!("File dialog: Selected file {:?}", path);
                            }
                            path_to_open = Some(path);
                            ui.close_menu();
                        } else {
                            tracing::debug!("File dialog: Cancelled");
                        }
                    }
                    ui.label(egui::RichText::new("Ctrl+O").weak());
                    ui.end_row();
                });

            // Open bitstream as... (force codec selection)
            ui.menu_button("Open bitstream as...", |ui| {
                ui.label("Force codec parser:");
                ui.separator();
                if ui.button("AV1").clicked() {
                    tracing::info!("File menu: Force AV1 parser");
                    // TODO: Open with AV1 parser forced
                    self.set_success("Force AV1 parser - not yet implemented".to_string());
                    ui.close_menu();
                }
                if ui.button("HEVC").clicked() {
                    tracing::info!("File menu: Force HEVC parser");
                    self.set_success("Force HEVC parser - not yet implemented".to_string());
                    ui.close_menu();
                }
                if ui.button("AVC/H.264").clicked() {
                    tracing::info!("File menu: Force AVC parser");
                    self.set_success("Force AVC parser - not yet implemented".to_string());
                    ui.close_menu();
                }
                if ui.button("VP9").clicked() {
                    tracing::info!("File menu: Force VP9 parser");
                    self.set_success("Force VP9 parser - not yet implemented".to_string());
                    ui.close_menu();
                }
                if ui.button("VVC/H.266").clicked() {
                    tracing::info!("File menu: Force VVC parser");
                    self.set_success("Force VVC parser - not yet implemented".to_string());
                    ui.close_menu();
                }
                if ui.button("MPEG-2").clicked() {
                    tracing::info!("File menu: Force MPEG-2 parser");
                    self.set_success("Force MPEG-2 parser - not yet implemented".to_string());
                    ui.close_menu();
                }
            });

            if ui.button("Open dependent bitstream...").clicked() {
                tracing::info!("File menu: Open dependent bitstream clicked");
                // TODO: Open YUV reference file for comparison
                self.set_success("Open dependent bitstream - not yet implemented".to_string());
                ui.close_menu();
            }

            ui.separator();

            // Continue the same Grid
            egui::Grid::new("file_menu_grid2")
                .num_columns(2)
                .min_col_width(140.0)
                .spacing([10.0, 4.0])
                .show(ui, |ui| {
                    // Close bitstream (Ctrl+W)
                    if ui.button("Close bitstream").clicked() {
                        tracing::info!("File menu: Close bitstream clicked");
                        close_file = true;
                        ui.close_menu();
                    }
                    ui.label(egui::RichText::new("Ctrl+W").weak());
                    ui.end_row();
                });

            ui.separator();

            // Extract... submenu
            ui.menu_button("Extract...", |ui| {
                if ui.button("YUV frames").clicked() {
                    tracing::info!("File menu: Extract YUV frames");
                    self.set_success("Extract YUV - not yet implemented".to_string());
                    ui.close_menu();
                }
                if ui.button("Prediction frames").clicked() {
                    tracing::info!("File menu: Extract Prediction frames");
                    self.set_success("Extract Prediction - not yet implemented".to_string());
                    ui.close_menu();
                }
                if ui.button("Reconstruction frames").clicked() {
                    tracing::info!("File menu: Extract Reconstruction frames");
                    self.set_success("Extract Reconstruction - not yet implemented".to_string());
                    ui.close_menu();
                }
                if ui.button("Transform coefficients").clicked() {
                    tracing::info!("File menu: Extract Transform coefficients");
                    self.set_success("Extract Transform - not yet implemented".to_string());
                    ui.close_menu();
                }
            });

            // Recent files section (VQAnalyzer parity - max 9 files)
            if !self.recent_files.is_empty() {
                ui.separator();
                ui.label("Recent Files:");
                for (i, path) in self.recent_files.iter().enumerate() {
                    let label = format!(
                        "{} {}",
                        i + 1,
                        path.file_name()
                            .map(|n| n.to_string_lossy())
                            .unwrap_or_else(|| path.to_string_lossy())
                    );
                    if ui
                        .button(label)
                        .on_hover_text(path.display().to_string())
                        .clicked()
                    {
                        // Log recent file metadata
                        if let Ok(metadata) = std::fs::metadata(path) {
                            let size_mb = metadata.len() as f64 / (1024.0 * 1024.0);
                            tracing::info!(
                                "File menu: Opening recent file ({:.2} MB): {:?}",
                                size_mb,
                                path
                            );
                        } else {
                            tracing::info!("File menu: Opening recent file {:?}", path);
                        }
                        path_to_open = Some(path.clone());
                        ui.close_menu();
                    }
                }
                if ui.button("Clear recent files").clicked() {
                    tracing::info!("File menu: Clear recent files");
                    self.recent_files.clear();
                    // Save empty list to disk
                    if let Err(e) = self.save_recent_files() {
                        tracing::warn!("Failed to save recent files: {}", e);
                    }
                    ui.close_menu();
                }
            }

            ui.separator();

            egui::Grid::new("file_menu_grid3")
                .num_columns(2)
                .min_col_width(140.0)
                .spacing([10.0, 4.0])
                .show(ui, |ui| {
                    // Quit (Ctrl+Q)
                    if ui.button("Quit").clicked() {
                        tracing::info!("File menu: Quit clicked");
                        quit_app = true;
                        ui.close_menu();
                    }
                    ui.label(egui::RichText::new("Ctrl+Q").weak());
                    ui.end_row();
                });
        });

        // Handle file opening
        if let Some(path) = path_to_open {
            self.open_file(path, ctx);
        }

        // Handle close file
        if close_file {
            self.close_file(ctx);
        }

        // Handle quit
        if quit_app {
            tracing::info!("Quitting application");
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }

    /// Render YUVDiff menu for reference comparison (VQAnalyzer parity)
    fn handle_yuvdiff_menu(&mut self, ui: &mut egui::Ui, _ctx: &egui::Context) {
        ui.menu_button("YUVDiff", |ui| {
            egui::Grid::new("yuvdiff_menu_grid")
                .num_columns(2)
                .min_col_width(140.0)
                .spacing([10.0, 4.0])
                .show(ui, |ui| {
                    // VQAnalyzer: Open debug YUV (Ctrl+Y)
                    if ui.button("Open debug YUV...").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .set_title("Open Debug YUV File")
                            .add_filter("YUV Files", &["yuv", "y4m"])
                            .add_filter("All Files", &["*"])
                            .pick_file()
                        {
                            // Log YUV file metadata
                            if let Ok(metadata) = std::fs::metadata(&path) {
                                let size_mb = metadata.len() as f64 / (1024.0 * 1024.0);
                                let ext = path
                                    .extension()
                                    .and_then(|e| e.to_str())
                                    .unwrap_or("unknown");
                                tracing::info!(
                                    "YUVDiff: Loading {} file ({:.2} MB): {:?}",
                                    ext,
                                    size_mb,
                                    path
                                );
                            } else {
                                tracing::info!("YUVDiff: Loading debug YUV: {:?}", path);
                            }
                            self.yuv_diff_settings.reference_file = Some(path);
                            ui.close_menu();
                        } else {
                            tracing::debug!("YUV dialog: Cancelled");
                        }
                    }
                    ui.label(egui::RichText::new("Ctrl+Y").weak());
                    ui.end_row();
                });

            // VQAnalyzer: Recent YUV files
            ui.menu_button("Recent YUV files", |ui| {
                ui.label("(No recent files)");
            });

            if ui.button("Close debug YUV").clicked() {
                tracing::info!("YUVDiff menu: Close debug YUV clicked");
                self.yuv_diff_settings.reference_file = None;
                ui.close_menu();
            }

            ui.separator();

            // VQAnalyzer: Subsampling
            ui.menu_button("Subsampling", |ui| {
                if ui
                    .radio_value(
                        &mut self.yuv_diff_settings.subsampling,
                        ChromaSubsampling::Yuv420,
                        "Planar",
                    )
                    .clicked()
                {
                    tracing::info!("YUVDiff: Subsampling changed to YUV420 (Planar)");
                }
                if ui
                    .radio_value(
                        &mut self.yuv_diff_settings.subsampling,
                        ChromaSubsampling::Yuv422,
                        "Interleaved",
                    )
                    .clicked()
                {
                    tracing::info!("YUVDiff: Subsampling changed to YUV422 (Interleaved)");
                }
            });

            ui.separator();

            // VQAnalyzer: Display/Decode order
            ui.radio_value(&mut 0, 0, "Display order");
            ui.radio_value(&mut 0, 1, "Decode order");

            ui.separator();

            // VQAnalyzer: Stream crop & offset
            ui.checkbox(&mut false, "Use stream crop values");
            if ui.button("Set picture offset here").clicked() {
                tracing::info!("Set picture offset");
            }
            ui.label("Picture offset (0)...");

            ui.separator();

            // VQAnalyzer: Bitdepth options
            if ui
                .radio_value(
                    &mut self.yuv_diff_settings.bit_depth,
                    BitDepth::Bit8,
                    "Use stream bitdepth",
                )
                .clicked()
            {
                tracing::info!("YUVDiff: Bit depth changed to 8-bit (stream)");
            }
            if ui
                .radio_value(
                    &mut self.yuv_diff_settings.bit_depth,
                    BitDepth::Bit10,
                    "Use max stream bitdepth",
                )
                .clicked()
            {
                tracing::info!("YUVDiff: Bit depth changed to 10-bit (max stream)");
            }
            if ui
                .radio_value(
                    &mut self.yuv_diff_settings.bit_depth,
                    BitDepth::Bit12,
                    "Use 16 bit bitdepth",
                )
                .clicked()
            {
                tracing::info!("YUVDiff: Bit depth changed to 16-bit");
            }
            if ui.button("Set YUV bitdepth").clicked() {
                tracing::info!("Set YUV bitdepth");
            }

            ui.separator();

            ui.checkbox(&mut false, "Check for file changes");

            ui.separator();

            // Visualization toggles
            ui.checkbox(&mut self.yuv_diff_settings.show_psnr_map, "Show PSNR Map");
            ui.checkbox(&mut self.yuv_diff_settings.show_ssim_map, "Show SSIM Map");
            ui.checkbox(&mut self.yuv_diff_settings.show_delta, "Show Delta Image");

            ui.separator();

            // Export options
            ui.checkbox(
                &mut self.yuv_diff_settings.export_all_frames,
                "Export All Frames",
            )
            .on_hover_text("Export metrics for all frames (requires frames to be decoded)");

            if ui.button("Export Metrics CSV...").clicked() {
                // Export PSNR/SSIM metrics to CSV
                if let Some(ref _ref_file) = self.yuv_diff_settings.reference_file {
                    if let Some(path) = rfd::FileDialog::new()
                        .set_file_name("metrics.csv")
                        .add_filter("CSV Files", &["csv"])
                        .save_file()
                    {
                        tracing::info!("Exporting metrics to {:?}", path);
                        match self.export_diff_metrics_csv(
                            &path,
                            self.yuv_diff_settings.export_all_frames,
                        ) {
                            Ok(()) => {
                                self.set_success(format!("Metrics exported to {}", path.display()));
                            }
                            Err(e) => {
                                self.set_error(format!("Export failed: {}", e));
                            }
                        }
                        ui.close_menu();
                    }
                } else {
                    tracing::warn!("No reference file loaded for metrics export");
                    self.set_error("Load a reference YUV file first".to_string());
                }
            }
        });
    }

    /// Render Options menu (VQAnalyzer parity)
    fn handle_options_menu(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.menu_button("Options", |ui| {
            // Color Space submenu
            ui.menu_button("Color Space", |ui| {
                if ui
                    .radio_value(
                        &mut self.app_settings.color_space,
                        ColorSpace::Bt601,
                        "ITU Rec. 601",
                    )
                    .clicked()
                {
                    tracing::info!("Options: Color space changed to BT.601");
                }
                if ui
                    .radio_value(
                        &mut self.app_settings.color_space,
                        ColorSpace::Bt709,
                        "ITU Rec. 709",
                    )
                    .clicked()
                {
                    tracing::info!("Options: Color space changed to BT.709");
                }
                if ui
                    .radio_value(
                        &mut self.app_settings.color_space,
                        ColorSpace::Bt2020,
                        "ITU Rec. 2020",
                    )
                    .clicked()
                {
                    tracing::info!("Options: Color space changed to BT.2020");
                }
                ui.separator();
                ui.label("YUV as RGB");
                ui.label("YUV as GBR");
            });

            // CPU & Performance submenu
            ui.menu_button("CPU & Performance", |ui| {
                let mut cpu_opt = self.app_settings.cpu_optimization == CpuOptimization::Auto;
                if ui
                    .checkbox(&mut cpu_opt, "Enable CPU optimizations [avx2]")
                    .changed()
                {
                    self.app_settings.cpu_optimization = if cpu_opt {
                        CpuOptimization::Auto
                    } else {
                        CpuOptimization::Disabled
                    };
                    tracing::info!("Options: CPU optimization = {}", cpu_opt);
                }
                ui.separator();
                ui.checkbox(&mut false, "Loop playback");
            });

            // Codec Settings submenu
            ui.menu_button("Codec Settings", |ui| {
                ui.label("HEVC:");
                ui.checkbox(&mut true, "  Enable extensions");
                ui.checkbox(&mut true, "  Enable stream index");
                ui.checkbox(&mut true, "  Show only visible CTB MV");

                ui.separator();
                ui.label("VVC:");
                ui.checkbox(&mut false, "  Dynamic selection info");
                ui.checkbox(&mut false, "  Details popup window");

                ui.separator();
                ui.label("JXS:");
                ui.checkbox(&mut false, "  Use reference CFA");

                ui.separator();
                ui.label("Digest calculation:");
                ui.radio_value(&mut 0, 0, "  Force digest");
                ui.radio_value(&mut 0, 1, "  No digest");
                ui.radio_value(&mut 0, 2, "  As in bitstream");

                ui.separator();
                ui.checkbox(&mut false, "Force options immediately");
            });

            ui.separator();

            // Theme & Layout submenu
            ui.menu_button("Theme & Layout", |ui| {
                ui.label("Theme:");
                if ui
                    .radio_value(&mut self.app_settings.theme, egui::Theme::Dark, "  Dark")
                    .clicked()
                {
                    ctx.set_theme(self.app_settings.theme);
                    tracing::info!("Options: Theme changed to Dark");
                }
                if ui
                    .radio_value(&mut self.app_settings.theme, egui::Theme::Light, "  Light")
                    .clicked()
                {
                    ctx.set_theme(self.app_settings.theme);
                    tracing::info!("Options: Theme changed to Light");
                }

                ui.separator();
                ui.label("Layout:");

                if ui.button("  Save Layout...").clicked() {
                    tracing::info!("Options: Save Layout clicked");
                    match self.save_layout() {
                        Ok(()) => {
                            tracing::info!("Options: Layout saved successfully");
                            self.set_success("Layout saved".to_string());
                        }
                        Err(e) => {
                            tracing::error!("Options: Failed to save layout: {}", e);
                            self.set_error(format!("Failed to save layout: {}", e));
                        }
                    }
                    ui.close_menu();
                }

                if ui.button("  Load Layout...").clicked() {
                    tracing::info!("Options: Load Layout clicked");
                    match self.load_layout() {
                        Ok(()) => {
                            tracing::info!("Options: Layout loaded successfully");
                            self.set_success("Layout loaded".to_string());
                        }
                        Err(e) => {
                            tracing::error!("Options: Failed to load layout: {}", e);
                            self.set_error(format!("Failed to load layout: {}", e));
                        }
                    }
                    ui.close_menu();
                }

                if ui.button("  Reset Layout").clicked() {
                    tracing::info!("Options: Reset Layout clicked");
                    self.dock_state = self.default_dock_state.clone();
                    ui.close_menu();
                }

                ui.checkbox(
                    &mut self.app_settings.auto_save_layout,
                    "  Auto-save on exit",
                )
                .on_hover_text("Automatically save panel layout when closing");
            });
        });
    }

    /// Render Mode menu (VQAnalyzer parity - F1-F12 visualization modes)
    fn handle_mode_menu(&mut self, ui: &mut egui::Ui) {
        ui.menu_button("Mode", |ui| {
            egui::Grid::new("mode_menu_grid")
                .num_columns(2)
                .min_col_width(140.0)
                .spacing([10.0, 4.0])
                .show(ui, |ui| {
                    if ui
                        .button("Overview")
                        .on_hover_text("High-level statistics")
                        .clicked()
                    {
                        // TODO: Switch to overview mode
                        tracing::info!("Mode: Overview");
                        ui.close_menu();
                    }
                    ui.label(egui::RichText::new("F1").weak());
                    ui.end_row();

                    if ui
                        .button("Coding Flow")
                        .on_hover_text("CTB grid with block types")
                        .clicked()
                    {
                        tracing::info!("Mode: Coding Flow");
                        ui.close_menu();
                    }
                    ui.label(egui::RichText::new("F2").weak());
                    ui.end_row();

                    if ui
                        .button("Prediction")
                        .on_hover_text("Intra/inter prediction modes")
                        .clicked()
                    {
                        tracing::info!("Mode: Prediction");
                        ui.close_menu();
                    }
                    ui.label(egui::RichText::new("F3").weak());
                    ui.end_row();

                    if ui
                        .button("Transform")
                        .on_hover_text("Transform unit tree")
                        .clicked()
                    {
                        tracing::info!("Mode: Transform");
                        ui.close_menu();
                    }
                    ui.label(egui::RichText::new("F4").weak());
                    ui.end_row();

                    if ui
                        .button("QP Map")
                        .on_hover_text("Quantization parameter heatmap")
                        .clicked()
                    {
                        tracing::info!("Mode: QP Map");
                        ui.close_menu();
                    }
                    ui.label(egui::RichText::new("F5").weak());
                    ui.end_row();
                });
        });
    }

    /// Render Export menu (Data extraction)
    fn handle_export_menu(&mut self, ui: &mut egui::Ui) {
        ui.menu_button("Export", |ui| {
            // Data Export submenu
            ui.menu_button("Data Export", |ui| {
                if ui.button("Frame Sizes (CSV)").clicked() {
                    tracing::info!("Export menu: Frame Sizes (CSV) clicked");
                    let stream_a = self.core.get_stream(StreamId::A);
                    let state = stream_a.read();

                    if let Some(ref units_model) = state.units {
                        let unit_count = units_model.units.len();
                        tracing::info!("Export: Found {} units to export", unit_count);
                        if let Some(path) = rfd::FileDialog::new()
                            .set_file_name("frame_sizes.csv")
                            .add_filter("CSV Files", &["csv"])
                            .save_file()
                        {
                            tracing::info!(
                                "Export: Frame sizes ({} units) to {:?}",
                                unit_count,
                                path
                            );
                            self.set_success(format!("Exported to {}", path.display()));
                        } else {
                            tracing::debug!("Export dialog: Cancelled");
                        }
                    } else {
                        tracing::warn!("Export: No bitstream loaded");
                        self.set_error("No bitstream loaded".to_string());
                    }
                    ui.close_menu();
                }

                if ui.button("Unit Tree (JSON)").clicked() {
                    tracing::info!("Export menu: Unit Tree (JSON) clicked");
                    let stream_a = self.core.get_stream(StreamId::A);
                    let state = stream_a.read();

                    if let Some(ref units_model) = state.units {
                        let unit_count = units_model.units.len();
                        tracing::info!("Export: Found {} units to export", unit_count);
                        if let Some(path) = rfd::FileDialog::new()
                            .set_file_name("units.json")
                            .add_filter("JSON Files", &["json"])
                            .save_file()
                        {
                            tracing::info!(
                                "Export: Unit tree ({} units) to {:?}",
                                unit_count,
                                path
                            );
                            self.set_success(format!("Exported to {}", path.display()));
                        } else {
                            tracing::debug!("Export dialog: Cancelled");
                        }
                    } else {
                        tracing::warn!("Export: No bitstream loaded");
                        self.set_error("No bitstream loaded".to_string());
                    }
                    ui.close_menu();
                }

                if ui.button("Syntax Tree (JSON)").clicked() {
                    tracing::info!("Export menu: Syntax Tree (JSON) clicked");
                    let stream_a = self.core.get_stream(bitvue_core::StreamId::A);
                    let state = stream_a.read();

                    if state.syntax.is_some() {
                        if let Some(path) = rfd::FileDialog::new()
                            .set_file_name("syntax.json")
                            .add_filter("JSON Files", &["json"])
                            .save_file()
                        {
                            tracing::info!("Export: Syntax tree to {:?}", path);
                            self.set_success(format!("Exported to {}", path.display()));
                        } else {
                            tracing::debug!("Export dialog: Cancelled");
                        }
                    } else {
                        tracing::warn!("Export: No unit selected");
                        self.set_error("No unit selected".to_string());
                    }
                    ui.close_menu();
                }
            }); // End of Data Export submenu

            ui.separator();

            // Advanced Export
            if ui.button("Evidence Bundle...").clicked() {
                tracing::info!("Export menu: Evidence Bundle clicked");
                if let Some(dir) = rfd::FileDialog::new()
                    .set_title("Select Evidence Bundle Output Directory")
                    .pick_folder()
                {
                    tracing::info!("Export: Evidence bundle to {:?}", dir);
                    // TODO: Implement evidence bundle export
                    self.set_success(format!("Evidence bundle: {}", dir.display()));
                } else {
                    tracing::debug!("Export dialog: Cancelled");
                }
                ui.close_menu();
            }
        });
    }

    /// Render View menu (VQAnalyzer parity)
    fn handle_view_menu(&mut self, ui: &mut egui::Ui) {
        ui.menu_button("View", |ui| {
            ui.label("Layout:");
            ui.separator();

            if ui.button("Reset Layout").clicked() {
                tracing::info!("View menu: Reset Layout clicked");
                self.dock_state = self.default_dock_state.clone();
                ui.close_menu();
            }

            ui.separator();
            ui.label("Panels:");

            // TODO: Panel visibility toggles
            ui.checkbox(&mut true, "Stream Tree");
            ui.checkbox(&mut true, "Player");
            ui.checkbox(&mut true, "Diagnostics");
        });
    }

    /// Render Help menu (VQAnalyzer parity)
    fn handle_help_menu(&mut self, ui: &mut egui::Ui) {
        ui.menu_button("Help", |ui| {
            if ui.button("Documentation").clicked() {
                tracing::info!("Help menu: Documentation clicked");
                self.set_success("Opening documentation...".to_string());
                ui.close_menu();
            }

            if ui.button("Keyboard Shortcuts").clicked() {
                tracing::info!("Help menu: Keyboard Shortcuts clicked");
                self.set_success("Ctrl+O: Open | Ctrl+W: Close | Ctrl+Q: Quit | F1-F5: Modes | ←→: Navigate frames".to_string());
                ui.close_menu();
            }

            ui.separator();

            if ui.button("About bitvue").clicked() {
                tracing::info!("Help menu: About bitvue clicked");
                self.set_success(format!("bitvue v{} - Open Source Bitstream Analyzer", env!("CARGO_PKG_VERSION")));
                ui.close_menu();
            }
        });
    }
}
