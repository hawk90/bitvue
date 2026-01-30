//! UI panel and component handlers for BitvueApp

use crate::bitvue_app::BitvueApp;
use crate::helpers::find_unit_by_key_with_index;
use bitvue_core::{Command, StreamId};
use eframe::egui;
use std::sync::Arc;

/// UI panel/component methods
pub trait BitvueAppPanels {
    fn handle_filmstrip_panel(&mut self, ctx: &egui::Context);
    fn handle_status_bar(&mut self, ctx: &egui::Context);
}

impl BitvueAppPanels for BitvueApp {
    fn handle_filmstrip_panel(&mut self, ctx: &egui::Context) {
        // VQAnalyzer parity: Filmstrip at bottom (above status bar)
        // Consistent height across all view modes
        let (min_h, max_h) = (200.0, 300.0);

        egui::TopBottomPanel::bottom("filmstrip")
            .min_height(min_h)
            .max_height(max_h)
            .show(ctx, |ui| {
                let stream_a = self.core.get_stream(StreamId::A);
                let state = stream_a.read();
                let selection = self.core.get_selection();
                let sel_guard = selection.read();

                let units_opt = state.units.as_ref().map(|u| u.units.as_slice());
                let frames_opt = state.frames.as_ref();
                let diagnostics = &state.diagnostics; // Get all diagnostics

                // Log data availability
                if let Some(units) = units_opt {
                    tracing::debug!("Filmstrip panel: {} units available", units.len());
                } else {
                    tracing::debug!("Filmstrip panel: No units");
                }
                if frames_opt.is_some() {
                    tracing::debug!("Filmstrip panel: FrameCache available");
                } else {
                    tracing::debug!("Filmstrip panel: No frames cache");
                }
                tracing::debug!("Filmstrip panel: {} diagnostics", diagnostics.len());

                if let Some(command) = self
                    .panels.filmstrip
                    .show(ui, ctx, units_opt, frames_opt, &sel_guard, &diagnostics)
                {
                    drop(sel_guard);
                    drop(state);

                    // Handle frame selection
                    let _events = self.core.handle_command(command.clone());

                    // Decode frame if needed (same logic as Timeline)
                    if let Command::SelectUnit { unit_key, .. } = command {
                        let state_read = stream_a.read();
                        if let Some(byte_cache) = state_read.byte_cache.clone() {
                            if let Some(units) = &state_read.units {
                                let units_clone = units.units.clone();
                                drop(state_read);

                                if let Some((unit_index, unit)) =
                                    find_unit_by_key_with_index(&units_clone, &unit_key)
                                {
                                    // Parse syntax
                                    if let Ok(obu_data) =
                                        byte_cache.read_range(unit.offset, unit.size)
                                    {
                                        let global_offset = unit.offset * 8;
                                        if let Ok(syntax) =
                                            crate::syntax_builder::build_syntax_from_obu_data(
                                                obu_data,
                                                unit_index,
                                                global_offset,
                                            )
                                        {
                                            let mut state_mut = stream_a.write();
                                            state_mut.syntax = Some(syntax);
                                        }
                                    }

                                    // Decode frame
                                    if let Some(frame_index) = unit.frame_index {
                                        let state_mut = stream_a.write();
                                        let cache_hit = if let Some(frames) = &state_mut.frames {
                                            frames.peek(frame_index).filter(|c| c.decoded).is_some()
                                        } else {
                                            false
                                        };

                                        if cache_hit {
                                            if let Some(frames) = &state_mut.frames {
                                                if let Some(cached) = frames.peek(frame_index) {
                                                    let color_image = egui::ColorImage::from_rgb(
                                                        [
                                                            cached.width as usize,
                                                            cached.height as usize,
                                                        ],
                                                        &cached.rgb_data,
                                                    );
                                                    self.workspaces.player
                                                        .set_frame(ctx, color_image);

                                                    // Update YUV viewer from cache
                                                    if let (Some(y), Some(u), Some(v)) = (
                                                        &cached.y_plane,
                                                        &cached.u_plane,
                                                        &cached.v_plane,
                                                    ) {
                                                        self.panels.yuv_viewer_mut().set_yuv_data(
                                                            ctx,
                                                            y,
                                                            u,
                                                            v,
                                                            cached.width,
                                                            cached.height,
                                                        );
                                                    }
                                                }
                                            }
                                        } else {
                                            // Async decode (cache miss)
                                            drop(state_mut);

                                            // Check if already pending to avoid cancelling in-progress decode
                                            if !self.decoder.is_pending(StreamId::A, frame_index) {
                                                if let Ok(all_data) =
                                                    byte_cache.read_range(0, byte_cache.len() as usize)
                                                {
                                                    tracing::debug!("📸 Filmstrip selection: Submitting decode for frame {}", frame_index);
                                                    self.submit_decode_request(
                                                        StreamId::A,
                                                        frame_index,
                                                        Arc::new(all_data.to_vec()),
                                                    );
                                                }
                                            } else {
                                                tracing::debug!("📸 Filmstrip selection: Frame {} already pending decode, skipping", frame_index);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            });
    }

    /// Render status bar with file info, metrics, and diagnostics (VQAnalyzer parity)
    fn handle_status_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            let stream_a = self.core.get_stream(StreamId::A);
            let state_a = stream_a.read();

            ui.horizontal(|ui| {
                // Left side: File path or empty state message (VQAnalyzer style)
                if let Some(path) = &state_a.file_path {
                    // Show full file path (VQAnalyzer shows full path)
                    let path_str = path.to_str().unwrap_or("unknown");
                    ui.label(path_str);
                } else {
                    // VQAnalyzer empty state message
                    ui.label("Choose a bitstream file or drag one in.");
                }

                // Info messages (non-intrusive, in status bar instead of toast)
                if let Some(success_msg) = self.notifications.success() {
                    ui.separator();
                    ui.colored_label(
                        egui::Color32::from_rgb(100, 200, 100),
                        format!("ℹ {}", success_msg),
                    );
                }

                // Error/warning counts (VQAnalyzer parity - show prominently)
                let error_count = state_a
                    .diagnostics_by_severity(bitvue_core::event::Severity::Error)
                    .len();
                let warn_count = state_a
                    .diagnostics_by_severity(bitvue_core::event::Severity::Warn)
                    .len();

                if error_count > 0 {
                    ui.separator();
                    ui.colored_label(
                        egui::Color32::from_rgb(255, 80, 80),
                        format!("⚠ {} errors", error_count),
                    );
                }
                if warn_count > 0 {
                    if error_count == 0 {
                        ui.separator();
                    }
                    ui.colored_label(
                        egui::Color32::from_rgb(255, 200, 80),
                        format!("⚠ {} warnings", warn_count),
                    );
                }

                // Right side: Version info (VQAnalyzer style)
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Version info
                    ui.label(
                        egui::RichText::new(format!("Version {}", env!("CARGO_PKG_VERSION")))
                            .color(egui::Color32::GRAY)
                            .size(11.0),
                    );

                    // File size breakdown (VQAnalyzer shows multi-part size info)
                    if let Some(cache) = &state_a.byte_cache {
                        let size_bytes = cache.len();
                        let size_mb = size_bytes as f64 / 1_000_000.0;
                        let size_kb = size_bytes as f64 / 1_000.0;
                        ui.separator();
                        ui.label(
                            egui::RichText::new(format!("{:.1}MB / {:.0}KB", size_mb, size_kb))
                                .color(egui::Color32::GRAY)
                                .size(11.0),
                        );
                    }
                });
            });
        });
    }
}
