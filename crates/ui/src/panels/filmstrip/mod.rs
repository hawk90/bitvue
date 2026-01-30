//! Filmstrip Panel - VQAnalyzer-style horizontal frame thumbnail strip
//!
//! Features:
//! - Horizontal scrolling row of frame thumbnails
//! - Frame numbers displayed above each thumbnail
//! - Frame type indicators (I/P/B coloring)
//! - Current frame highlighted with selection indicator
//! - Clickable to navigate to frame
//! - Lazy loading/virtualization for large streams
//! - VQAnalyzer parity: Three visualization modes:
//!   1. Thumbnails (default) - horizontal thumbnail strip
//!   2. Frame Sizes - bar chart showing frame sizes
//!   3. B-Pyramid - hierarchical GOP structure view

mod bpyramid;
mod enhanced;
mod frame_sizes;
mod helpers;
mod hrd_buffer;
mod thumbnails;

use helpers::collect_frame_info;

use bitvue_core::{Command, FrameType, SelectionState, StreamId, ThumbnailCache, UnitNode};
use egui::{self, Color32, ColorImage, TextureHandle};
use std::collections::HashMap;

/// Filmstrip visualization mode (VQAnalyzer parity + Bitvue enhancements)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FilmstripViewMode {
    /// Default thumbnail strip view
    #[default]
    Thumbnails,
    /// Frame sizes bar chart view (VQAnalyzer parity)
    FrameSizes,
    /// B-Pyramid hierarchical GOP view (VQAnalyzer parity)
    BPyramid,
    /// HRD buffer fullness view (VQAnalyzer parity)
    HrdBuffer,
    /// Enhanced multi-metric view (Bitvue exclusive)
    /// Features: GOP boundaries, scene changes, multi-metric overlay, smart navigation
    Enhanced,
}

impl std::fmt::Display for FilmstripViewMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FilmstripViewMode::Thumbnails => write!(f, "Thumbnails"),
            FilmstripViewMode::FrameSizes => write!(f, "Frame Sizes"),
            FilmstripViewMode::BPyramid => write!(f, "B-Pyramid"),
            FilmstripViewMode::HrdBuffer => write!(f, "HRD Buffer"),
            FilmstripViewMode::Enhanced => write!(f, "✨ Enhanced"),
        }
    }
}

/// Frame info for filmstrip display
#[derive(Clone)]
struct FrameInfo {
    frame_index: usize,
    frame_type: FrameType,
    unit_key: bitvue_core::UnitKey,
    offset: u64,
    /// Frame/unit size in bytes
    size: usize,
    /// POC (Picture Order Count) - uses frame_index if not available
    poc: i32,
    /// Display order index (for future use)
    _display_order: usize,
    /// Decode order index (for future use)
    _decode_order: usize,
    /// NAL unit type name (e.g., "TRAIL_N", "IDR_W_RADL")
    nal_type: String,
    /// PTS (Presentation Timestamp) if available
    pts: Option<u64>,
    /// DTS (Decode Timestamp) if available
    dts: Option<u64>,
    /// Reference list (L0/L1) indicator
    ref_list: Option<String>,
    /// Diagnostic count for this frame (bitvue unique feature)
    diagnostic_count: usize,
    /// Highest impact score among diagnostics (0-100)
    max_impact: u8,
}

/// Filmstrip panel state
pub struct FilmstripPanel {
    /// Thumbnail textures (GPU-uploaded)
    textures: HashMap<usize, TextureHandle>,
    /// Thumbnail cache (CPU-side)
    thumbnail_cache: ThumbnailCache,
    /// Thumbnail display width
    thumb_width: f32,
    /// Thumbnail display height
    thumb_height: f32,
    /// Spacing between thumbnails
    spacing: f32,
    /// Show colored borders based on frame type (VQAnalyzer parity)
    pub show_type_borders: bool,
    /// Show POC below thumbnails
    pub show_poc: bool,
    /// Show PTS/DTS below thumbnails
    pub show_timestamps: bool,
    /// Show reference arrows (VQAnalyzer parity)
    pub show_ref_arrows: bool,
    /// Border thickness for frame type indicator
    border_thickness: f32,
    /// Reference arrow height
    ref_arrow_height: f32,
    /// Current visualization mode (VQAnalyzer parity)
    pub view_mode: FilmstripViewMode,
    /// Cached frame info (performance optimization)
    cached_frames: Vec<FrameInfo>,
    /// Cache key (unit count to detect changes)
    cache_key: usize,
    /// Moving average window size for Frame Sizes view (VQAnalyzer parity)
    pub moving_avg_window: usize,
    /// Frame type filters for Frame Sizes view (VQAnalyzer parity)
    pub show_i_frames: bool,
    pub show_p_frames: bool,
    pub show_b_frames: bool,
    /// Show moving average line in Frame Sizes view (VQAnalyzer parity)
    pub show_moving_avg: bool,
    /// Show legend panel in Frame Sizes view (VQAnalyzer parity)
    pub show_legend: bool,
}

impl Default for FilmstripPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl FilmstripPanel {
    pub fn new() -> Self {
        Self {
            textures: HashMap::new(),
            thumbnail_cache: ThumbnailCache::new(100, 100), // 100px wide, cache 100 thumbnails
            thumb_width: 100.0,                             // VQAnalyzer parity: compact thumbnails
            thumb_height: 56.0,                             // VQAnalyzer parity: ~16:9 aspect
            spacing: 2.0,                                   // VQAnalyzer parity: tight spacing
            show_type_borders: true,                        // VQAnalyzer parity
            show_poc: false,        // VQAnalyzer parity: NAL type shown instead
            show_timestamps: false, // Off by default to reduce clutter
            show_ref_arrows: false, // VQAnalyzer parity: off by default
            border_thickness: 2.0,  // VQAnalyzer parity: thinner border
            ref_arrow_height: 20.0, // Height of reference arrow area
            view_mode: FilmstripViewMode::Thumbnails, // Default to thumbnails
            cached_frames: Vec::new(),
            cache_key: 0,
            moving_avg_window: 21, // VQAnalyzer default: 21 frames
            show_i_frames: true,   // VQAnalyzer: all on by default
            show_p_frames: true,
            show_b_frames: true,
            show_moving_avg: true, // VQAnalyzer: moving average visible by default
            show_legend: true,     // VQAnalyzer: legend visible by default
        }
    }

    /// Show the filmstrip panel
    /// Returns Command if frame was clicked
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        units: Option<&[UnitNode]>,
        frames_cache: Option<&bitvue_core::stream_state::FrameModel>,
        selection: &SelectionState,
        diagnostics: &[bitvue_core::event::Diagnostic],
    ) -> Option<Command> {
        let mut result_command: Option<Command> = None;

        // Collect frame information (cached for performance)
        if let Some(units) = units {
            let current_key = units.len();

            // Only recollect if units changed
            if self.cache_key != current_key || self.cached_frames.is_empty() {
                let collected = collect_frame_info(units, diagnostics);

                // Log only on cache miss
                if !collected.is_empty() {
                    let total_size: usize = collected.iter().map(|f| f.size).sum();
                    let avg_size = total_size / collected.len();
                    let max_size = collected.iter().map(|f| f.size).max().unwrap_or(0);
                    let min_size = collected.iter().map(|f| f.size).min().unwrap_or(0);
                    let frames_with_diagnostics =
                        collected.iter().filter(|f| f.diagnostic_count > 0).count();
                    let total_diagnostics: usize =
                        collected.iter().map(|f| f.diagnostic_count).sum();

                    tracing::info!(
                        "🎞️ Filmstrip: Collected {} frames from {} units | total={} bytes, avg={}, min={}, max={}",
                        collected.len(), units.len(), total_size, avg_size, min_size, max_size
                    );
                    tracing::info!(
                        "🎞️ Filmstrip: 🏷️ {} frames have diagnostics ({} total errors/warnings)",
                        frames_with_diagnostics,
                        total_diagnostics
                    );

                    // Log first 3 frames only
                    for frame in collected.iter().take(3) {
                        let badge_info = if frame.diagnostic_count > 0 {
                            format!(
                                " [🏷️ {} diagnostics, impact={}]",
                                frame.diagnostic_count, frame.max_impact
                            )
                        } else {
                            String::new()
                        };
                        tracing::debug!(
                            "  Frame #{}: type={:6} size={:6} bytes, offset={}, poc={}, nal={}{}",
                            frame.frame_index,
                            frame.frame_type,
                            frame.size,
                            frame.offset,
                            frame.poc,
                            frame.nal_type,
                            badge_info
                        );
                    }
                    if collected.len() > 3 {
                        tracing::debug!("  ... and {} more frames", collected.len() - 3);
                    }
                }

                self.cached_frames = collected;
                self.cache_key = current_key;
            }
        } else {
            self.cached_frames.clear();
            self.cache_key = 0;
        }

        if self.cached_frames.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.label(
                    egui::RichText::new("No frames - Open a file to see thumbnails")
                        .color(Color32::GRAY),
                );
            });
            return None;
        }

        tracing::debug!(
            "Filmstrip: Rendering {} frames in {:?} mode",
            self.cached_frames.len(),
            self.view_mode
        );

        // PERFORMANCE OPTIMIZATION: Pre-generate thumbnails before rendering
        // This avoids holding both mutable and immutable borrows simultaneously.
        // We collect frame indices that need thumbnails, generate them all at once,
        // then use immutable references during rendering.
        let frame_indices: Vec<usize> = self.cached_frames.iter().map(|f| f.frame_index).collect();
        for &frame_index in &frame_indices {
            self.ensure_thumbnail(ctx, frame_index, frames_cache);
        }

        // Get selected frame index (before creating frames reference)
        let selected_frame_index = selection
            .unit
            .as_ref()
            .and_then(|uk| self.cached_frames.iter().find(|f| f.offset == uk.offset))
            .map(|f| f.frame_index);

        // Header with playback controls, frame count, view mode toggles (VQAnalyzer parity)
        ui.horizontal(|ui| {
            // Playback controls (VQAnalyzer parity - integrated into Filmstrip)
            ui.label(egui::RichText::new("Play:").small());

            // Local scope for frames reference (dropped after this block)
            let frames = &self.cached_frames;

            if ui
                .small_button("⏮")
                .on_hover_text("First frame (Home)")
                .clicked()
            {
                if let Some(first_frame) = frames.first() {
                    result_command = Some(Command::SelectUnit {
                        stream: StreamId::A,
                        unit_key: first_frame.unit_key.clone(),
                    });
                }
            }
            if ui
                .small_button("←")
                .on_hover_text("Previous frame (Left)")
                .clicked()
            {
                if let Some(idx) = selected_frame_index {
                    if idx > 0 {
                        result_command = Some(Command::SelectUnit {
                            stream: StreamId::A,
                            unit_key: frames[idx - 1].unit_key.clone(),
                        });
                    }
                }
            }
            if ui
                .small_button("→")
                .on_hover_text("Next frame (Right)")
                .clicked()
            {
                if let Some(idx) = selected_frame_index {
                    if idx + 1 < frames.len() {
                        result_command = Some(Command::SelectUnit {
                            stream: StreamId::A,
                            unit_key: frames[idx + 1].unit_key.clone(),
                        });
                    }
                }
            }
            if ui
                .small_button("⏭")
                .on_hover_text("Last frame (End)")
                .clicked()
            {
                if let Some(last_frame) = frames.last() {
                    result_command = Some(Command::SelectUnit {
                        stream: StreamId::A,
                        unit_key: last_frame.unit_key.clone(),
                    });
                }
            }

            ui.separator();

            // Frame count and selected frame
            ui.label(
                egui::RichText::new(format!("Frames: {}", frames.len()))
                    .small()
                    .color(Color32::GRAY),
            );
            if let Some(idx) = selected_frame_index {
                ui.label(
                    egui::RichText::new(format!("({}/{})", idx + 1, frames.len()))
                        .small()
                        .color(Color32::from_rgb(255, 200, 100)),
                );
            }

            ui.separator();

            // View mode dropdown (VQAnalyzer parity + Bitvue exclusive)
            ui.label(egui::RichText::new("View:").small().color(Color32::GRAY));
            egui::ComboBox::from_id_salt("filmstrip_view_mode")
                .selected_text(format!("{}", self.view_mode))
                .width(120.0)
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut self.view_mode,
                        FilmstripViewMode::Thumbnails,
                        "Thumbnails",
                    );
                    ui.selectable_value(
                        &mut self.view_mode,
                        FilmstripViewMode::FrameSizes,
                        "Frame Sizes",
                    );
                    ui.selectable_value(
                        &mut self.view_mode,
                        FilmstripViewMode::BPyramid,
                        "B-Pyramid",
                    );
                    ui.selectable_value(
                        &mut self.view_mode,
                        FilmstripViewMode::HrdBuffer,
                        "HRD Buffer",
                    );
                    ui.separator();
                    ui.selectable_value(
                        &mut self.view_mode,
                        FilmstripViewMode::Enhanced,
                        "✨ Enhanced",
                    );
                });

            // Display options (right-aligned) - only for thumbnails view
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if self.view_mode == FilmstripViewMode::Thumbnails {
                    ui.checkbox(&mut self.show_ref_arrows, "Refs");
                    ui.checkbox(&mut self.show_timestamps, "PTS/DTS");
                    ui.checkbox(&mut self.show_poc, "POC");
                    ui.checkbox(&mut self.show_type_borders, "Borders");
                }
            });
        });

        // Calculate positions for reference arrows
        let thumb_total_width = self.thumb_width + self.border_thickness * 2.0 + self.spacing;

        // Dispatch based on view mode (VQAnalyzer parity)
        match self.view_mode {
            FilmstripViewMode::Thumbnails => {
                // Local scope for frames reference (dropped after this block)
                let frames = &self.cached_frames;

                // Horizontal scroll area for thumbnails
                let scroll_output = egui::ScrollArea::horizontal()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            for frame in frames.iter() {
                                let is_selected = selected_frame_index == Some(frame.frame_index);

                                // Get texture ID (thumbnails were pre-generated above)
                                let texture_id =
                                    self.textures.get(&frame.frame_index).map(|t| t.id());

                                // Render thumbnail with click handling
                                if let Some(cmd) =
                                    self.render_thumbnail(ui, frame, texture_id, is_selected)
                                {
                                    result_command = Some(cmd);
                                }
                            }
                        });
                    });

                // VQAnalyzer parity: Draw reference arrows below filmstrip
                if self.show_ref_arrows && frames.len() > 1 {
                    self.render_reference_arrows(
                        ui,
                        frames,
                        thumb_total_width,
                        scroll_output.state.offset.x,
                    );
                }
            }
            FilmstripViewMode::FrameSizes => {
                // Clone needed here because render_frame_sizes_view needs &mut self
                let frames = self.cached_frames.clone();
                result_command = self.render_frame_sizes_view(ui, &frames, selected_frame_index);
            }
            FilmstripViewMode::BPyramid => {
                // Clone needed here because render_bpyramid_view needs &mut self
                let frames = self.cached_frames.clone();
                result_command = self.render_bpyramid_view(ui, &frames, selected_frame_index);
            }
            FilmstripViewMode::HrdBuffer => {
                // Clone needed here because render_hrd_buffer_view needs &mut self
                let frames = self.cached_frames.clone();
                result_command = self.render_hrd_buffer_view(ui, &frames, selected_frame_index);
            }
            FilmstripViewMode::Enhanced => {
                // Clone needed here because render_enhanced_view needs &mut self
                let frames = self.cached_frames.clone();
                result_command = self.render_enhanced_view(ui, &frames, selected_frame_index);
            }
        }

        result_command
    }

    /// Ensure thumbnail is cached (generate if needed)
    fn ensure_thumbnail(
        &mut self,
        ctx: &egui::Context,
        frame_index: usize,
        frames_cache: Option<&bitvue_core::stream_state::FrameModel>,
    ) {
        // Check if texture already exists
        if self.textures.contains_key(&frame_index) {
            self.thumbnail_cache.touch(frame_index);
            return;
        }

        // Try to generate from cached frame (peek to avoid affecting LRU order)
        if let Some(frames) = frames_cache {
            if let Some(cached_frame) = frames.peek(frame_index) {
                if cached_frame.decoded {
                    tracing::info!(
                        "📸 Filmstrip: Generating thumbnail for frame {} ({}x{})",
                        frame_index,
                        cached_frame.width,
                        cached_frame.height
                    );
                    // Generate thumbnail
                    let thumbnail = ThumbnailCache::generate_thumbnail(
                        cached_frame,
                        self.thumbnail_cache.target_width,
                    );

                    // Upload to GPU
                    let color_image = ColorImage::from_rgb(
                        [thumbnail.width as usize, thumbnail.height as usize],
                        &thumbnail.rgb_data,
                    );

                    let texture = ctx.load_texture(
                        format!("filmstrip_thumb_{}", frame_index),
                        color_image,
                        egui::TextureOptions::LINEAR,
                    );

                    self.textures.insert(frame_index, texture);
                    self.thumbnail_cache.insert(thumbnail);
                    tracing::info!("📸 Filmstrip: ✅ Thumbnail {} uploaded to GPU", frame_index);
                } else {
                    tracing::warn!(
                        "📸 Filmstrip: Frame {} in cache but not decoded (decoded=false)",
                        frame_index
                    );
                }
            } else {
                tracing::debug!("📸 Filmstrip: Frame {} not in cache yet", frame_index);
            }
        } else {
            tracing::debug!("📸 Filmstrip: No frames_cache available");
        }
    }

    /// Clear all cached textures (call when file changes)
    pub fn clear_cache(&mut self) {
        self.textures.clear();
        self.thumbnail_cache.clear();
    }
}
