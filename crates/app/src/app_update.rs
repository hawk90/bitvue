//! eframe::App trait implementation for BitvueApp

use crate::app_async::BitvueAppAsync;
use crate::app_config::BitvueAppConfig;
use crate::app_decode::BitvueAppDecode;
use crate::app_input::BitvueAppInput;
use crate::app_ui;
use crate::app_ui_menus::BitvueAppMenus;
use crate::app_ui_panels::BitvueAppPanels;
use crate::app_ui_toolbar::BitvueAppToolbar;
use crate::bitvue_app::BitvueApp;
use crate::helpers::{count_frames, find_frame_by_index, get_current_frame_index};
use crate::panel_tab::FrameNavRequest;
use crate::panel_tab_viewer::PanelTabViewer;
use bitvue_core::{Command, StreamId};
use eframe::egui;
use egui_dock::{DockArea, Style};
use std::sync::Arc;

impl eframe::App for BitvueApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Poll for async worker results (Phase 2-3 - non-blocking)
        self.poll_decode_results(ctx);
        self.poll_bytecache_results(ctx);
        self.poll_parse_results(ctx);
        self.poll_parse_progress(ctx);
        self.poll_export_results();
        self.poll_config_results();

        // Handle file shortcuts (Ctrl+O/W/Q) - VQAnalyzer parity
        use crate::app_input::FileShortcutAction;
        match self.handle_file_shortcuts(ctx) {
            FileShortcutAction::Open => {
                // Trigger file dialog
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Video Files", &["ivf", "av1", "mp4", "mkv", "webm", "ts"])
                    .add_filter("All Files", &["*"])
                    .pick_file()
                {
                    self.open_file(path, ctx);
                }
            }
            FileShortcutAction::Close => {
                self.close_file(ctx);
            }
            FileShortcutAction::Quit => {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            FileShortcutAction::None => {}
        }

        // Handle mode switching shortcuts (F1-F5) - VQAnalyzer parity Phase 4
        self.handle_mode_shortcuts(ctx);

        // Handle keyboard navigation (Left/Right/Home/End)
        if let Some((stream_id, frame_index)) = self.handle_keyboard_navigation(ctx) {
            // Trigger decode for the new frame
            let stream = self.core.get_stream(stream_id);
            let byte_cache_opt = {
                let state = stream.read();
                state.byte_cache.clone()
            };
            if let Some(byte_cache) = byte_cache_opt {
                if let Ok(all_data) = byte_cache.read_range(0, byte_cache.len() as usize) {
                    self.submit_decode_request(stream_id, frame_index, Arc::new(all_data.to_vec()));
                }
            }
        }

        // Check notification timeouts and display toasts
        self.check_notification_timeouts();

        // Render toasts (success/error notifications)
        let (dismiss_success, dismiss_error) = app_ui::render_toasts(ctx, &mut self.notifications);
        if dismiss_success {
            self.notifications.clear_success();
        }
        if dismiss_error {
            self.notifications.clear_error();
        }

        // R1: Menu Bar (VQAnalyzer parity) - toolbar integrated into Filmstrip
        let _ = self.handle_toolbar(ctx);

        // R5: Status Bar (VQAnalyzer parity) - extracted to method
        self.handle_status_bar(ctx);

        // Filmstrip Panel (VQAnalyzer parity) - above status bar, with integrated controls
        self.handle_filmstrip_panel(ctx);

        // R2, R3, R4: Dock area (center)
        DockArea::new(&mut self.dock_state)
            .style(Style::from_egui(ctx.style().as_ref()))
            .show(
                ctx,
                &mut PanelTabViewer {
                    core: &self.core,
                    decoder: &mut self.decoder,
                    panels: &mut self.panels,
                    workspaces: &mut self.workspaces,
                },
            );
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        // Auto-save layout if enabled
        if self.app_settings.auto_save_layout {
            if let Err(e) = self.save_layout() {
                tracing::warn!("Failed to auto-save layout on exit: {}", e);
            } else {
                tracing::info!("Layout auto-saved on exit");
            }
        }

        // Auto-save recent files on exit
        if let Err(e) = self.save_recent_files() {
            tracing::warn!("Failed to save recent files on exit: {}", e);
        }
    }
}
