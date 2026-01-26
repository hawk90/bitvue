//! PanelTabViewer - Implements TabViewer trait for dock tabs
//!
//! Handles rendering of all panel tabs in the dock area

use crate::decode_coordinator::DecodeCoordinator;
use crate::helpers::{count_frames, find_unit_by_key_with_index, find_unit_containing_offset_helper, format_bytes};
use crate::panel_registry::PanelRegistry;
use crate::panel_tab::PanelTab;
use crate::syntax_builder;
use crate::workspace_registry::WorkspaceRegistry;
use bitvue_core::{Command, Core, StreamId};
use eframe::egui;
use egui_dock::TabViewer;
use std::sync::Arc;

pub struct PanelTabViewer<'a> {
    pub core: &'a Arc<Core>,
    pub decoder: &'a mut DecodeCoordinator,
    pub panels: &'a mut PanelRegistry,
    pub workspaces: &'a mut WorkspaceRegistry,
}


impl<'a> TabViewer for PanelTabViewer<'a> {
    type Tab = PanelTab;

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        tab.to_string().into()
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        match tab {
            PanelTab::StreamTree => {
                // Display Stream A state
                let stream_a = self.core.get_stream(StreamId::A);
                let state = stream_a.read();

                if let Some(ref path) = state.file_path {
                    // File info header
                    ui.horizontal(|ui| {
                        ui.label("📁");
                        ui.label(
                            path.file_name()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .to_string(),
                        );
                    });

                    if let Some(ref byte_cache) = state.byte_cache {
                        ui.label(format!("Size: {} bytes", format_bytes(byte_cache.len())));
                    }

                    // Show container info
                    if let Some(ref container) = state.container {
                        ui.label(format!("Format: {:?}", container.format));
                        ui.label(format!("Codec: {}", container.codec));
                    }

                    // Get diagnostics counts early
                    let error_count = state
                        .diagnostics_by_severity(bitvue_core::event::Severity::Error)
                        .len();
                    let warn_count = state
                        .diagnostics_by_severity(bitvue_core::event::Severity::Warn)
                        .len();

                    ui.separator();

                    // Show unit tree
                    if let Some(ref units) = state.units {
                        ui.label(format!(
                            "📊 {} units, {} frames",
                            units.unit_count, units.frame_count
                        ));
                        ui.separator();

                        // Use panel to render tree
                        let selection = self.core.get_selection();
                        let sel_guard = selection.read();
                        if let Some(command) =
                            self.panels.stream_tree.show(ui, &units.units, &sel_guard)
                        {
                            drop(sel_guard); // Release selection lock

                            // Clone what we need before handling command
                            let byte_cache_opt = state.byte_cache.clone();
                            let units_clone = units.units.clone();
                            drop(state); // Release read lock

                            // Handle command and trigger syntax parsing if needed
                            let _events = self.core.handle_command(command.clone());

                            // Parse syntax for selected unit (business logic)
                            if let Command::SelectUnit { unit_key, .. } = command {
                                if let Some(byte_cache) = byte_cache_opt {
                                    // Find the unit to get its size and index
                                    if let Some((unit_index, unit)) =
                                        find_unit_by_key_with_index(&units_clone, &unit_key)
                                    {
                                        if let Ok(obu_data) =
                                            byte_cache.read_range(unit.offset, unit.size)
                                        {
                                            // Use new syntax parser with bit-level tracking
                                            let global_offset = unit.offset * 8; // Convert byte to bit offset
                                            if let Ok(syntax) =
                                                syntax_builder::build_syntax_from_obu_data(
                                                    obu_data,
                                                    unit_index,
                                                    global_offset,
                                                )
                                            {
                                                let mut state_mut = stream_a.write();
                                                state_mut.syntax = Some(syntax);
                                                tracing::debug!(
                                                    "Syntax parsed for {} with bit-level tracking",
                                                    unit.unit_type
                                                );
                                            } else {
                                                tracing::warn!(
                                                    "Failed to parse syntax for {}",
                                                    unit.unit_type
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    } else {
                        ui.label("⏳ Parsing...");
                    }

                    // Show diagnostics summary at bottom

                    if error_count > 0 || warn_count > 0 {
                        ui.separator();
                        ui.horizontal(|ui| {
                            if error_count > 0 {
                                ui.label(format!("❌ {}", error_count));
                            }
                            if warn_count > 0 {
                                ui.label(format!("⚠️ {}", warn_count));
                            }
                        });
                    }
                } else {
                    ui.label("No file loaded");
                    ui.separator();
                    ui.colored_label(egui::Color32::GRAY, "Click 'Open' to load a file");
                }
            }

            /* PanelTab::Timeline removed - integrated into Filmstrip panel
            PanelTab::Timeline => {
                let stream_a = self.core.get_stream(StreamId::A);
                let state = stream_a.read();
                let selection = self.core.get_selection();
                let sel_guard = selection.read();

                // Get units and container if available
                let units_opt = state.units.as_ref().map(|u| u.units.as_slice());
                let container_opt = state.container.as_ref();

                if let Some(command) =
                    self.workspaces.timeline
                        .show(ui, units_opt, container_opt, &sel_guard)
                {
                    drop(sel_guard);
                    drop(state);

                    tracing::info!("Timeline command: {:?}", command);

                    // Handle Timeline→All sync (TC05)
                    let _events = self.core.handle_command(command.clone());

                    // Parse syntax for the selected frame
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
                                            syntax_builder::build_syntax_from_obu_data(
                                                obu_data,
                                                unit_index,
                                                global_offset,
                                            )
                                        {
                                            let mut state_mut = stream_a.write();
                                            state_mut.syntax = Some(syntax);
                                        }
                                    }

                                    // Decode frame (Phase 2)
                                    if let Some(frame_index) = unit.frame_index {
                                        let state_mut = stream_a.write();

                                        // Check cache first
                                        let cache_hit = if let Some(frames) = &state_mut.frames {
                                            frames.peek(frame_index).filter(|c| c.decoded).is_some()
                                        } else {
                                            false
                                        };

                                        if cache_hit {
                                            // Display cached frame
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
                                                        .set_frame(ui.ctx(), color_image);

                                                    // Update YUV viewer from cache
                                                    if let (Some(y), Some(u), Some(v)) = (
                                                        &cached.y_plane,
                                                        &cached.u_plane,
                                                        &cached.v_plane,
                                                    ) {
                                                        self.panels.yuv_viewer_mut().set_yuv_data(
                                                            ui.ctx(),
                                                            y,
                                                            u,
                                                            v,
                                                            cached.width,
                                                            cached.height,
                                                        );
                                                    }

                                                    tracing::info!(
                                                        "Frame {} cache hit",
                                                        frame_index
                                                    );
                                                }
                                            }
                                        } else {
                                            // Async decode (cache miss)
                                            tracing::info!(
                                                "Frame {} cache miss - submitting async decode",
                                                frame_index
                                            );
                                            drop(state_mut);

                                            if let Ok(all_data) =
                                                byte_cache.read_range(0, byte_cache.len() as usize)
                                            {
                                                if self.decoder.submit(
                                                    StreamId::A,
                                                    frame_index,
                                                    Arc::new(all_data.to_vec()),
                                                ).is_some() {
                                                    tracing::info!(
                                                        "Submitted async decode: frame {}",
                                                        frame_index
                                                    );
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            */ // End of Timeline removal

            PanelTab::Player => {
                let stream_a = self.core.get_stream(StreamId::A);
                let state = stream_a.read();
                let selection = self.core.get_selection();
                let sel_guard = selection.read();

                let units_opt = state.units.as_ref().map(|u| u.units.as_slice());

                // Count total frames for navigation controls
                let total_frames = state
                    .units
                    .as_ref()
                    .map(|u| count_frames(&u.units))
                    .unwrap_or(0);

                if let Some(command) = self.workspaces.player.show(
                    ui,
                    state.container.as_ref(),
                    Some(&sel_guard),
                    units_opt,
                    total_frames,
                ) {
                    // Handle navigation command (SelectUnit for frame navigation)
                    drop(sel_guard);
                    drop(state);

                    let _events = self.core.handle_command(command);
                }
            }

            PanelTab::BitrateGraph => {
                let stream_a = self.core.get_stream(StreamId::A);
                let state = stream_a.read();
                let selection = self.core.get_selection();
                let sel_guard = selection.read();

                let units_opt = state.units.as_ref().map(|u| u.units.as_slice());
                self.panels.bitrate_graph.show(ui, units_opt, &sel_guard);
            }

            PanelTab::QualityMetrics => {
                self.panels.quality_metrics_mut().show(ui);
            }

            PanelTab::SyntaxTree => {
                let stream_a = self.core.get_stream(StreamId::A);
                let state = stream_a.read();
                let selection = self.core.get_selection();
                let sel_guard = selection.read();

                if let Some(command) = self
                    .panels
                    .syntax_detail_mut()
                    .show(ui, state.syntax.as_ref(), &sel_guard)
                {
                    // Tri-sync: Syntax node clicked → update selection
                    drop(sel_guard);
                    drop(state);

                    let _events = self.core.handle_command(command);
                }
            }

            PanelTab::HexView => {
                let stream_a = self.core.get_stream(StreamId::A);
                let state = stream_a.read();
                let selection = self.core.get_selection();
                let sel_guard = selection.read();

                let units_opt = state.units.as_ref().map(|u| u.units.as_slice());

                if let Some(command) = self.panels.hex_view_mut().show(
                    ui,
                    state.byte_cache.as_ref().map(|arc| arc.as_ref()),
                    units_opt,
                    &sel_guard,
                ) {
                    // TC04: Hex→Tree sync - handle command
                    drop(sel_guard);
                    drop(state);

                    let _events = self.core.handle_command(command.clone());

                    // Parse syntax for selected unit
                    if let Command::SelectUnit { unit_key, .. } = command {
                        let state_read = stream_a.read();
                        if let Some(byte_cache) = state_read.byte_cache.clone() {
                            if let Some(units) = &state_read.units {
                                let units_clone = units.units.clone();
                                drop(state_read);

                                if let Some((unit_index, unit)) =
                                    find_unit_by_key_with_index(&units_clone, &unit_key)
                                {
                                    if let Ok(obu_data) =
                                        byte_cache.read_range(unit.offset, unit.size)
                                    {
                                        let global_offset = unit.offset * 8;
                                        if let Ok(syntax) =
                                            syntax_builder::build_syntax_from_obu_data(
                                                obu_data,
                                                unit_index,
                                                global_offset,
                                            )
                                        {
                                            let mut state_mut = stream_a.write();
                                            state_mut.syntax = Some(syntax);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            PanelTab::BitView => {
                let stream_a = self.core.get_stream(StreamId::A);
                let state = stream_a.read();
                let selection = self.core.get_selection();
                let sel_guard = selection.read();

                let units_opt = state.units.as_ref().map(|u| u.units.as_slice());

                if let Some(command) = self.panels.bit_view_mut().show(
                    ui,
                    state.byte_cache.as_ref().map(|arc| arc.as_ref()),
                    units_opt,
                    &sel_guard,
                ) {
                    // TC04: Bit→Tree sync - handle command
                    drop(sel_guard);
                    drop(state);

                    let _events = self.core.handle_command(command.clone());

                    // Parse syntax for selected unit
                    if let Command::SelectUnit { unit_key, .. } = command {
                        let state_read = stream_a.read();
                        if let Some(byte_cache) = state_read.byte_cache.clone() {
                            if let Some(units) = &state_read.units {
                                let units_clone = units.units.clone();
                                drop(state_read);

                                if let Some((unit_index, unit)) =
                                    find_unit_by_key_with_index(&units_clone, &unit_key)
                                {
                                    if let Ok(obu_data) =
                                        byte_cache.read_range(unit.offset, unit.size)
                                    {
                                        let global_offset = unit.offset * 8;
                                        if let Ok(syntax) =
                                            syntax_builder::build_syntax_from_obu_data(
                                                obu_data,
                                                unit_index,
                                                global_offset,
                                            )
                                        {
                                            let mut state_mut = stream_a.write();
                                            state_mut.syntax = Some(syntax);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            PanelTab::BlockInfo => {
                let selection = self.core.get_selection();
                let sel_guard = selection.read();

                self.panels.block_info_mut().show(ui, &sel_guard);
            }

            PanelTab::SelectionInfo => {
                let stream_a = self.core.get_stream(StreamId::A);
                let state = stream_a.read();
                let selection = self.core.get_selection();
                let sel_guard = selection.read();

                self.panels.selection_info
                    .show(ui, &sel_guard, state.container.as_ref());
            }

            PanelTab::Diagnostics => {
                // Get diagnostics from stream state
                let stream_a = self.core.get_stream(StreamId::A);
                let state = stream_a.read();
                let diagnostics = &state.diagnostics;

                if let Some(clicked_diagnostic) = self.workspaces.diagnostics.show(ui, diagnostics) {
                    // TC06: Diagnostics→All sync (enhanced with frame_index)
                    tracing::info!("Diagnostic clicked: {:?} at frame {:?}",
                        clicked_diagnostic.message, clicked_diagnostic.frame_index);

                    // Try frame-based navigation first (more accurate)
                    let mut navigated = false;
                    if let (Some(frame_idx), Some(units)) = (clicked_diagnostic.frame_index, &state.units) {
                        // Find frame by index
                        use crate::helpers::find_frame_by_index;
                        if let Some(frame_unit) = find_frame_by_index(&units.units, frame_idx) {
                            let unit_key = frame_unit.key.clone();
                            drop(state);

                            tracing::info!("Navigating to frame {} via frame_index", frame_idx);
                            let _events = self.core.handle_command(Command::SelectUnit {
                                stream: StreamId::A,
                                unit_key: unit_key.clone(),
                            });
                            navigated = true;

                            // Parse syntax and potentially trigger decode
                            let state_read = stream_a.read();
                            if let Some(byte_cache) = state_read.byte_cache.clone() {
                                if let Some(units) = &state_read.units {
                                    let units_clone = units.units.clone();
                                    drop(state_read);

                                    use crate::helpers::find_unit_by_key_with_index;
                                    if let Some((unit_index, unit)) =
                                        find_unit_by_key_with_index(&units_clone, &unit_key)
                                    {
                                        if let Ok(obu_data) =
                                            byte_cache.read_range(unit.offset, unit.size)
                                        {
                                            let global_offset = unit.offset * 8;
                                            if let Ok(syntax) =
                                                syntax_builder::build_syntax_from_obu_data(
                                                    obu_data,
                                                    unit_index,
                                                    global_offset,
                                                )
                                            {
                                                let mut state_mut = stream_a.write();
                                                state_mut.syntax = Some(syntax);
                                            }
                                        }

                                        // Trigger decode if it's a frame
                                        if unit.frame_index.is_some() {
                                            drop(byte_cache);
                                            // Decode request would go here
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Fallback to offset-based navigation if frame_index not available
                    if !navigated {
                        let offset = clicked_diagnostic.offset_bytes;
                        let state_ref = stream_a.read();
                        if let Some(units) = &state_ref.units {
                            if let Some(unit_node) =
                                find_unit_containing_offset_helper(&units.units, offset)
                            {
                                let unit_key = unit_node.key.clone();
                                drop(state_ref);

                                tracing::info!("Navigating to offset {} (fallback)", offset);
                                let _events = self.core.handle_command(Command::SelectUnit {
                                    stream: StreamId::A,
                                    unit_key: unit_key.clone(),
                                });

                                // Parse syntax for the unit
                                let state_read = stream_a.read();
                                if let Some(byte_cache) = state_read.byte_cache.clone() {
                                    if let Some(units) = &state_read.units {
                                        let units_clone = units.units.clone();
                                        drop(state_read);

                                        if let Some((unit_index, unit)) =
                                            find_unit_by_key_with_index(&units_clone, &unit_key)
                                        {
                                            if let Ok(obu_data) =
                                                byte_cache.read_range(unit.offset, unit.size)
                                            {
                                                let global_offset = unit.offset * 8;
                                                if let Ok(syntax) =
                                                    syntax_builder::build_syntax_from_obu_data(
                                                        obu_data,
                                                        unit_index,
                                                        global_offset,
                                                    )
                                                {
                                                    let mut state_mut = stream_a.write();
                                                    state_mut.syntax = Some(syntax);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            PanelTab::Metrics => {
                let selection = self.core.get_selection();
                let sel_guard = selection.read();

                if let Some(command) = self.workspaces.metrics_mut().show(ui, &sel_guard) {
                    drop(sel_guard);
                    let _events = self.core.handle_command(command);
                }
            }

            PanelTab::Reference => {
                let selection = self.core.get_selection();
                let sel_guard = selection.read();

                if let Some(command) = self.workspaces.reference_mut().show(ui, &sel_guard) {
                    drop(sel_guard);
                    let _events = self.core.handle_command(command);
                }
            }

            PanelTab::Compare => {
                // Update compare workspace with real stream data
                {
                    let stream_a = self.core.get_stream(StreamId::A);
                    let stream_b = self.core.get_stream(StreamId::B);
                    let state_a = stream_a.read();
                    let state_b = stream_b.read();

                    if let (Some(units_a), Some(units_b)) = (&state_a.units, &state_b.units) {
                        self.workspaces.compare_mut()
                            .update_from_streams(&units_a.units, &units_b.units);
                    }
                }

                let selection = self.core.get_selection();
                let sel_guard = selection.read();

                if let Some(command) = self.workspaces.compare_mut().show(ui, &sel_guard) {
                    drop(sel_guard);

                    // Handle ExportEvidenceBundle specially (opens file dialog)
                    if let Command::ExportEvidenceBundle { .. } = &command {
                        use bitvue_core::export::export_evidence_bundle;
                        use bitvue_core::export::EvidenceBundleExportRequest;
                        use bitvue_core::parity_harness::{
                            EntityRef, OrderType, SelectionSnapshot,
                        };

                        if let Some(dir) = rfd::FileDialog::new()
                            .set_title("Select Evidence Bundle Output Directory")
                            .pick_folder()
                        {
                            let stream_a = self.core.get_stream(StreamId::A);
                            let state = stream_a.read();
                            let selection = self.core.get_selection();
                            let sel_guard = selection.read();

                            let stream_fingerprint = state
                                .file_path
                                .as_ref()
                                .map(|p| format!("{:?}", p))
                                .unwrap_or_default();

                            let request = EvidenceBundleExportRequest {
                                output_dir: dir.clone(),
                                include_screenshots: true,
                                include_render_snapshots: true,
                                include_interaction_trace: false,
                                include_logs: false,
                                stream_fingerprint,
                                selection_state: SelectionSnapshot {
                                    selected_entity: sel_guard.unit.as_ref().map(|u| EntityRef {
                                        kind: "unit".to_string(),
                                        id: format!("{}:{}", u.unit_type, u.offset),
                                        frame_index: None,
                                        byte_offset: Some(u.offset),
                                    }),
                                    selected_byte_range: sel_guard
                                        .bit_range
                                        .as_ref()
                                        .map(|r| (r.start_bit / 8, r.end_bit / 8)),
                                    order_type: OrderType::Display,
                                },
                                workspace: "compare".to_string(),
                                mode: "diff".to_string(),
                                order_type: OrderType::Display,
                            };

                            let result = export_evidence_bundle(&request, &[]);

                            if result.success {
                                tracing::info!(
                                    "Evidence bundle exported to {:?}",
                                    result.bundle_path
                                );
                            } else {
                                tracing::error!(
                                    "Evidence bundle export failed: {:?}",
                                    result.error
                                );
                            }
                        }
                    } else {
                        let _events = self.core.handle_command(command);
                    }
                }
            }

            // Codec workspaces (VQAnalyzer parity - Coding Flow visualizations)
            PanelTab::Av1Coding => {
                self.workspaces.av1_mut().show(ui);
            }

            PanelTab::AvcCoding => {
                self.workspaces.avc_mut().show(ui);
            }

            PanelTab::HevcCoding => {
                self.workspaces.hevc_mut().show(ui);
            }

            PanelTab::Mpeg2Coding => {
                self.workspaces.mpeg2_mut().show(ui);
            }

            PanelTab::VvcCoding => {
                let selection = self.core.get_selection();
                let sel_guard = selection.read();

                if let Some(command) = self.workspaces.vvc_mut().show(ui, &sel_guard) {
                    drop(sel_guard);
                    let _events = self.core.handle_command(command);
                }
            }

            PanelTab::YuvViewer => {
                let selection = self.core.get_selection();
                let sel_guard = selection.read();

                self.panels.yuv_viewer_mut().show(ui, &sel_guard);
            }
        }
    }
}
