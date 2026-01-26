//! Application UI Rendering Module
//!
//! Handles all UI rendering logic, extracted from the massive update() method.
//! This module contains all egui rendering code to keep main.rs clean.

use crate::notifications::NotificationManager;
use eframe::egui;

/// Render error toast notifications at the top of the screen
///
/// Success/info messages are shown in status bar only (non-intrusive)
pub fn render_toasts(ctx: &egui::Context, notifications: &mut NotificationManager) -> (bool, bool) {
    let mut dismiss_success = false;
    let mut dismiss_error = false;

    // Success messages now shown only in status bar (non-intrusive)
    // No more green toast popups for "Parse Complete", "Loading...", etc.

    // Error toast (red) - still shown as popup for critical issues
    if let Some(error_msg) = notifications.error() {
        let error_msg = error_msg.to_string(); // Clone for closure
        egui::TopBottomPanel::top("error_toast")
            .frame(egui::Frame::none().fill(egui::Color32::from_rgb(60, 30, 30)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("⚠")
                            .color(egui::Color32::from_rgb(255, 100, 100))
                            .size(16.0),
                    );
                    ui.label(
                        egui::RichText::new(&error_msg)
                            .color(egui::Color32::from_rgb(255, 200, 200)),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button("✕").clicked() {
                            dismiss_error = true;
                        }
                    });
                });
            });
    }

    (dismiss_success, dismiss_error)
}
