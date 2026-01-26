//! BitvueApp - Main application struct

use crate::app_async::BitvueAppAsync;
use crate::app_config::BitvueAppConfig;
use crate::app_decode::BitvueAppDecode;
use crate::app_input::BitvueAppInput;
use crate::app_ui_menus::BitvueAppMenus;
use crate::app_ui_panels::BitvueAppPanels;
use crate::app_yuv_diff::BitvueAppYuvDiff;
use crate::bytecache_worker::ByteCacheWorker;
use crate::config_worker::ConfigWorker;
use crate::decode_coordinator::DecodeCoordinator;
use crate::export_worker::ExportWorker;
use crate::notifications::NotificationManager;
use crate::panel_registry::PanelRegistry;
use crate::panel_tab::PanelTab;
use crate::parse_coordinator::ParseCoordinator;
use crate::settings::AppSettings;
use crate::workspace_registry::WorkspaceRegistry;
use crate::yuv_diff::YuvDiffSettings;
use bitvue_core::{Core, StreamId};
use egui_dock::{DockState, NodeIndex};
use std::sync::Arc;

pub struct BitvueApp {
    pub dock_state: DockState<PanelTab>,
    pub default_dock_state: DockState<PanelTab>, // For reset layout (VQAnalyzer parity)
    pub core: Arc<Core>,
    // Async coordinators (worker threads + pending state tracking)
    pub decoder: DecodeCoordinator,
    pub parser: ParseCoordinator,
    pub bytecache_worker: ByteCacheWorker,
    pub export_worker: ExportWorker,
    pub config_worker: ConfigWorker,
    // Notification manager (error/success messages with auto-dismiss)
    pub notifications: NotificationManager,
    // UI panels registry (10 panels)
    pub panels: PanelRegistry,
    // UI workspaces registry (6 workspaces)
    pub workspaces: WorkspaceRegistry,
    // Recent files (VQAnalyzer parity - max 9 files)
    pub recent_files: Vec<std::path::PathBuf>,
    // YUV diff settings (VQAnalyzer parity - Phase 4)
    pub yuv_diff_settings: YuvDiffSettings,
    // Application settings (VQAnalyzer parity - Options Menu, Phase 4)
    pub app_settings: AppSettings,
}

impl BitvueApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        tracing::info!("BitvueApp initialized - Building dock layout");

        // Build initial dock tree matching W0 wireframe (LAYOUT_CONTRACT.md):
        // ┌───────────────┬─────────────────────────────────┬────────────────────────────┐
        // │ R2 Stream Tree│ R3 Player / Charts              │ R4 Inspectors              │
        // │    (20%)      │          (50%)                  │       (30%)                │
        // └───────────────┴─────────────────────────────────┴────────────────────────────┘
        // Note: Timeline integrated into Filmstrip panel at bottom

        let mut dock_state = DockState::new(vec![PanelTab::StreamTree]);

        // Split right to create [R2: 20%] | [R3+R4: 80%]
        let [_r2, right] = dock_state.main_surface_mut().split_right(
            NodeIndex::root(),
            0.2, // R2 (Stream Tree) takes 20%, right side gets 80%
            vec![
                PanelTab::Player,
                PanelTab::BitrateGraph,
                PanelTab::QualityMetrics,
                PanelTab::Metrics,
                PanelTab::Reference,
                // Codec workspaces (VQAnalyzer parity)
                PanelTab::Av1Coding,
                PanelTab::HevcCoding,
                PanelTab::AvcCoding,
                PanelTab::VvcCoding,
                PanelTab::Mpeg2Coding,
            ],
        );

        // Split right again to create [R3] | [R4]
        // R3 should take 50% of total = 50/80 = 0.625 of right side
        let [_r3, _r4] = dock_state.main_surface_mut().split_right(
            right,
            0.625, // R3 takes 62.5% of right side (= 50% of total width)
            vec![
                PanelTab::SyntaxTree,
                PanelTab::HexView,
                PanelTab::BitView,
                PanelTab::BlockInfo,
                PanelTab::SelectionInfo,
                PanelTab::Diagnostics,
                PanelTab::YuvViewer,
                PanelTab::Compare,
            ],
        );

        // Clone the default layout for reset functionality (VQAnalyzer parity)
        let default_dock_state = dock_state.clone();

        let mut app = Self {
            dock_state,
            default_dock_state,
            core: Arc::new(Core::new()),
            decoder: DecodeCoordinator::new(),
            parser: ParseCoordinator::new(),
            bytecache_worker: ByteCacheWorker::new(),
            export_worker: ExportWorker::new(),
            config_worker: ConfigWorker::new(),
            notifications: NotificationManager::new(),
            panels: PanelRegistry::new(),
            workspaces: WorkspaceRegistry::new(),
            recent_files: Vec::new(),
            yuv_diff_settings: YuvDiffSettings::default(),
            app_settings: AppSettings::default(),
        };

        // Load recent files from ~/.bitvue/recent.json (VQAnalyzer parity)
        if let Err(e) = app.load_recent_files() {
            tracing::warn!("Failed to load recent files: {}", e);
        }

        app
    }

    /// Set an error message to display (auto-dismisses after 5 seconds)
    pub fn set_error(&mut self, message: impl Into<String>) {
        self.notifications.set_error(message);
    }

    /// Add a file to recent files list
    pub fn add_recent_file(&mut self, path: std::path::PathBuf) {
        // Remove if already exists
        self.recent_files.retain(|p| p != &path);
        // Add to front
        self.recent_files.insert(0, path);
        // Keep max 9 files
        self.recent_files.truncate(9);
        // Save to disk
        let _ = self.save_recent_files();
    }

    /// Set a success message to display (auto-dismisses after 3 seconds)
    pub fn set_success(&mut self, message: impl Into<String>) {
        self.notifications.set_success(message);
    }

    /// Clear notifications if expired
    pub fn check_notification_timeouts(&mut self) {
        self.notifications.check_timeouts();
    }

    /// Submit async decode request (non-blocking)
    pub fn submit_decode_request(
        &mut self,
        stream_id: StreamId,
        frame_index: usize,
        file_data: Arc<Vec<u8>>,
    ) {
        tracing::debug!("🎬 Submitting decode request: stream={:?}, frame={}, data_size={} bytes",
            stream_id, frame_index, file_data.len());
        self.decoder.submit(stream_id, frame_index, file_data);
    }

    /// Open a file (shared by menu and Ctrl+O shortcut)
    pub fn open_file(&mut self, path: std::path::PathBuf, ctx: &eframe::egui::Context) {
        tracing::info!("Opening file (async): {:?}", path);

        // Cancel any pending requests for Stream A
        self.bytecache_worker.cancel_stream(StreamId::A);
        self.parser.cancel_stream(StreamId::A);

        // Submit ByteCache load request (NON-BLOCKING - runs in background thread)
        let request_id = self.bytecache_worker.next_request_id(StreamId::A);
        let request = crate::bytecache_worker::ByteCacheRequest {
            stream_id: StreamId::A,
            path: path.clone(),
            request_id,
        };

        if self.bytecache_worker.submit(request) {
            // Show loading notification
            self.set_success(format!(
                "Loading {}...",
                path.file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "file".to_string())
            ));

            // Update window title with file name
            let file_name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "Unknown".to_string());
            ctx.send_viewport_cmd(eframe::egui::ViewportCommand::Title(format!(
                "{} - bitvue",
                file_name
            )));

            tracing::info!(
                "Submitted async file load for {:?} (request_id: {})",
                path,
                request_id
            );
        } else {
            self.set_error("Failed to submit file load request (queue full)");
        }
    }

    /// Close current file (shared by menu and Ctrl+W shortcut)
    pub fn close_file(&mut self, ctx: &eframe::egui::Context) {
        tracing::info!("Closing current bitstream");

        // Cancel any pending requests
        self.bytecache_worker.cancel_stream(StreamId::A);
        self.parser.cancel_stream(StreamId::A);
        self.decoder.cancel_stream(StreamId::A);

        // Clear stream state
        let stream_a = self.core.get_stream(StreamId::A);
        let mut state_a = stream_a.write();
        *state_a = bitvue_core::StreamState::new(StreamId::A);
        drop(state_a);

        // Update window title
        ctx.send_viewport_cmd(eframe::egui::ViewportCommand::Title(
            "bitvue - AV1 Bitstream Analyzer".to_string(),
        ));
        self.set_success("Bitstream closed".to_string());
    }
}
