//! Async operations trait for BitvueApp
//!
//! Consolidates all async result polling in one place for clean separation of concerns.
//!
//! # Architecture
//!
//! This module provides the `BitvueAppAsync` trait that handles:
//! - ByteCache loading results
//! - File parsing results and progress
//! - Decode results (already implemented)
//!
//! All polling methods are non-blocking and safe to call every frame.

use crate::bitvue_app::BitvueApp;
use crate::config_worker::ConfigResultData;
use bitvue_core::Command;

/// Trait for async operations in BitvueApp
///
/// All methods are non-blocking and should be called in the main UI loop.
pub trait BitvueAppAsync {
    /// Poll for ByteCache load results
    ///
    /// When a ByteCache finishes loading, this:
    /// 1. Stores the ByteCache in StreamState
    /// 2. Submits a parse request for the file
    /// 3. Shows "Parsing..." notification
    fn poll_bytecache_results(&mut self, ctx: &egui::Context);

    /// Poll for parse results
    ///
    /// When parsing finishes, this:
    /// 1. Stores container and units in StreamState
    /// 2. Shows success notification
    /// 3. Requests repaint
    fn poll_parse_results(&mut self, ctx: &egui::Context);

    /// Poll for parse progress updates
    ///
    /// Updates the progress notification with current parse status.
    fn poll_parse_progress(&mut self, ctx: &egui::Context);

    /// Poll for export results
    ///
    /// When an export finishes, this shows success/error notification.
    fn poll_export_results(&mut self);

    /// Poll for config results
    ///
    /// When config operations finish (load/save), this handles the results.
    fn poll_config_results(&mut self);
}

impl BitvueAppAsync for BitvueApp {
    fn poll_bytecache_results(&mut self, ctx: &egui::Context) {
        let results = self.bytecache_worker.poll_results();

        for result in results {
            match result.result {
                Ok(byte_cache) => {
                    tracing::info!(
                        "ByteCache loaded successfully: {:?} ({} bytes)",
                        result.path,
                        byte_cache.len()
                    );

                    // Store ByteCache in StreamState
                    let stream = self.core.get_stream(result.stream_id);
                    {
                        let mut state = stream.write();
                        state.file_path = Some(result.path.clone());
                        state.byte_cache = Some(byte_cache.clone());
                    }

                    // Submit parse request
                    let request_id = self.parser.next_request_id(result.stream_id);
                    let parse_request = crate::parse_worker::ParseRequest {
                        stream_id: result.stream_id,
                        path: result.path.clone(),
                        byte_cache,
                        request_id,
                    };

                    if self.parser.submit(parse_request).is_some() {
                        self.notifications.set_success(format!(
                            "Parsing {}...",
                            result
                                .path
                                .file_name()
                                .unwrap_or_default()
                                .to_string_lossy()
                        ));
                    } else {
                        self.notifications
                            .set_error("Failed to submit parse request (queue full)");
                    }
                }
                Err(e) => {
                    tracing::error!("ByteCache load failed: {:?} - {}", result.path, e);
                    self.notifications
                        .set_error(format!("Failed to load file: {}", e));
                }
            }
        }

        // Request repaint if we have pending work
        if self.bytecache_worker.has_pending_work() {
            ctx.request_repaint();
        }
    }

    fn poll_parse_results(&mut self, ctx: &egui::Context) {
        let results = self.parser.poll_results();

        if !results.is_empty() {
            tracing::info!(
                "📥 poll_parse_results: Processing {} result(s)",
                results.len()
            );
        }

        for result in results {
            match result.result {
                Ok((container, units, diagnostics)) => {
                    tracing::info!(
                        "✅ Parse completed: {:?} - {} units, {} frames, {} diagnostics",
                        result.path,
                        units.unit_count,
                        units.frame_count,
                        diagnostics.len()
                    );

                    // Store in StreamState
                    let stream = self.core.get_stream(result.stream_id);
                    {
                        let mut state = stream.write();
                        state.container = Some(container);
                        state.units = Some(units);

                        // Add diagnostics to StreamState
                        for diagnostic in diagnostics {
                            state.add_diagnostic(diagnostic);
                        }
                    }

                    // Add to recent files
                    self.add_recent_file(result.path.clone());

                    self.notifications.set_success(format!(
                        "Loaded {}",
                        result
                            .path
                            .file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                    ));

                    // Clear pending state
                    self.parser
                        .clear_pending_if_matches(result.stream_id, &result.path);

                    // Auto-decode first N frames for thumbnails (VQAnalyzer parity)
                    // IMPORTANT: Do this BEFORE auto-select to avoid decode conflicts
                    tracing::info!("🎬 AUTO-DECODE: Starting thumbnail pre-generation...");
                    let byte_cache = stream.read().byte_cache.clone();
                    if let Some(byte_cache) = byte_cache {
                        tracing::info!(
                            "🎬 AUTO-DECODE: ByteCache available, size={} bytes",
                            byte_cache.len()
                        );
                        let all_data = byte_cache.read_range(0, byte_cache.len() as usize).ok();
                        if let Some(file_data) = all_data {
                            let file_data = std::sync::Arc::new(file_data.to_vec());

                            // Decode first 20 frames for filmstrip thumbnails
                            let frame_count = stream
                                .read()
                                .units
                                .as_ref()
                                .map(|u| crate::helpers::count_frames(&u.units))
                                .unwrap_or(0);
                            let decode_count = frame_count.min(20);

                            tracing::info!(
                                "🎬 AUTO-DECODE: Requesting first {} of {} frames",
                                decode_count,
                                frame_count
                            );

                            // Log frame indices from UnitNodes for debugging
                            if let Some(units) = &stream.read().units {
                                let mut frame_indices = Vec::new();
                                for unit in &units.units {
                                    if let Some(idx) = unit.frame_index {
                                        frame_indices.push(idx);
                                    }
                                }
                                tracing::warn!(
                                    "🎬 AUTO-DECODE: Unit frame_indices = {:?}",
                                    frame_indices
                                );
                            }

                            for decode_index in 0..decode_count {
                                tracing::info!(
                                    "🎬 AUTO-DECODE: Submitting decode for frame array index {}",
                                    decode_index
                                );
                                self.submit_decode_request(
                                    result.stream_id,
                                    decode_index,
                                    file_data.clone(),
                                );
                            }
                            tracing::info!(
                                "🎬 AUTO-DECODE: ✅ Submitted {} decode requests successfully!",
                                decode_count
                            );
                        } else {
                            tracing::error!("🎬 AUTO-DECODE: ❌ Failed to read byte_cache data");
                        }
                    } else {
                        tracing::error!("🎬 AUTO-DECODE: ❌ No byte_cache available");
                    }

                    // Trigger selection to first frame if no selection
                    // Do this AFTER auto-decode so frame 0 is already in decode queue
                    let selection = self.core.get_selection();
                    let has_temporal_selection = {
                        let sel_guard = selection.read();
                        sel_guard.temporal.is_some()
                    };

                    if !has_temporal_selection {
                        tracing::info!("🎯 Auto-selecting first frame...");
                        // Find first frame and select it
                        if let Some(first_frame_node) = crate::helpers::find_first_frame(
                            &stream.read().units.as_ref().unwrap().units,
                        ) {
                            let _events = self.core.handle_command(Command::SelectUnit {
                                stream: result.stream_id,
                                unit_key: first_frame_node.key.clone(),
                            });
                            tracing::info!("🎯 Auto-selected frame 0");
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Parse failed: {:?} - {:?}", result.path, e);
                    self.notifications.set_error(format!("Parse failed: {}", e));

                    // Clear pending state even on error
                    self.parser
                        .clear_pending_if_matches(result.stream_id, &result.path);
                }
            }
        }

        // Request repaint if we have pending work
        if self.parser.has_pending_work() {
            ctx.request_repaint();
        }
    }

    fn poll_parse_progress(&mut self, ctx: &egui::Context) {
        let progress_updates = self.parser.poll_progress();

        for progress in progress_updates {
            // Update progress notification
            if let Some(prog_value) = progress.progress {
                self.notifications
                    .set_progress(progress.message, prog_value);
            } else {
                self.notifications.set_success(progress.message);
            }
        }

        // Request repaint if we have pending work
        if self.parser.has_pending_work() {
            ctx.request_repaint();
        }
    }

    fn poll_export_results(&mut self) {
        let results = self.export_worker.poll_results();

        for result in results {
            match result.result {
                Ok(()) => {
                    self.notifications.set_success(format!(
                        "Exported to {}",
                        result
                            .path
                            .file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                    ));
                }
                Err(e) => {
                    self.notifications
                        .set_error(format!("Export failed: {}", e));
                }
            }
        }
    }

    fn poll_config_results(&mut self) {
        let results = self.config_worker.poll_results();

        for result in results {
            match result.result {
                ConfigResultData::RecentFilesLoaded(Ok(files)) => {
                    self.recent_files = files;
                    tracing::debug!("Loaded {} recent files", self.recent_files.len());
                }
                ConfigResultData::RecentFilesLoaded(Err(e)) => {
                    tracing::warn!("Failed to load recent files: {}", e);
                }
                ConfigResultData::RecentFilesSaved(Ok(())) => {
                    tracing::debug!("Saved recent files");
                }
                ConfigResultData::RecentFilesSaved(Err(e)) => {
                    tracing::warn!("Failed to save recent files: {}", e);
                }
                ConfigResultData::LayoutLoaded(Ok(_layout_json)) => {
                    tracing::debug!("Layout loaded (TODO: apply to dock_state)");
                    // TODO: Deserialize and apply to dock_state when ready
                }
                ConfigResultData::LayoutLoaded(Err(e)) => {
                    tracing::debug!("No saved layout: {}", e);
                }
                ConfigResultData::LayoutSaved(Ok(())) => {
                    tracing::debug!("Layout saved");
                }
                ConfigResultData::LayoutSaved(Err(e)) => {
                    tracing::warn!("Failed to save layout: {}", e);
                }
                ConfigResultData::SettingsLoaded(Ok(_settings_json)) => {
                    tracing::debug!("Settings loaded (TODO: apply to app_settings)");
                    // TODO: Deserialize and apply to app_settings when ready
                }
                ConfigResultData::SettingsLoaded(Err(e)) => {
                    tracing::debug!("No saved settings: {}", e);
                }
                ConfigResultData::SettingsSaved(Ok(())) => {
                    tracing::debug!("Settings saved");
                }
                ConfigResultData::SettingsSaved(Err(e)) => {
                    tracing::warn!("Failed to save settings: {}", e);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: Full integration tests require mocking the async workers
    // These would be tested through the actual UI update loop

    #[test]
    fn test_trait_exists() {
        // Verify trait can be used as a bound
        fn _requires_async<T: BitvueAppAsync>(_app: &T) {}
    }
}
