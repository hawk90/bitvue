//! bitvue - Professional AV1 Bitstream Analyzer
//!
//! Main entry point for the GUI application

use app::bitvue_app::BitvueApp;
use eframe::egui;

fn main() -> eframe::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    bitvue_log::init_from_env(); // Initialize VLOG from VLOG_LEVEL and VLOG_MODULE env vars
    tracing::info!("bitvue starting (Monster Pack v9 architecture)");

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([1024.0, 768.0]), // Per UI_VISUAL_DETAIL.md: Professional tool minimum
        ..Default::default()
    };

    eframe::run_native(
        "bitvue - AV1 Bitstream Analyzer",
        options,
        Box::new(|cc| Ok(Box::new(BitvueApp::new(cc)))),
    )
}
