//! Toolbar UI handler for BitvueApp

use crate::app_ui_menus::BitvueAppMenus;
use crate::bitvue_app::BitvueApp;
use crate::helpers::{count_frames, get_current_frame_index, get_memory_usage_mb};
use crate::panel_tab::FrameNavRequest;
use bitvue_core::{Command, StreamId};
use eframe::egui;
use ui::OverlayType;

/// Toolbar methods
pub trait BitvueAppToolbar {
    fn handle_toolbar(&mut self, ctx: &egui::Context) -> Option<FrameNavRequest>;
}

impl BitvueAppToolbar for BitvueApp {
    /// Render menu bar only (VQAnalyzer parity - toolbar integrated into Filmstrip)
    /// Returns None (toolbar moved to Filmstrip panel)
    fn handle_toolbar(&mut self, ctx: &egui::Context) -> Option<FrameNavRequest> {
        // R1: Menu Bar (VQAnalyzer parity)
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                // Menu items in VQAnalyzer order
                self.handle_file_menu(ui, ctx);
                self.handle_mode_menu(ui);
                self.handle_yuvdiff_menu(ui, ctx);
                self.handle_export_menu(ui);
                self.handle_options_menu(ui, ctx);
                self.handle_view_menu(ui);
                self.handle_help_menu(ui);
            });
        });

        // Toolbar controls moved to Filmstrip panel (VQAnalyzer parity)
        None
    }
}
