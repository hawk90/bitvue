//! YUV Component Viewer Panel - displays Y, U, V planes separately
//!
//! Features:
//! - View individual Y, U, V planes
//! - Toggle between planes or show all three
//! - Histogram display for each component
//! - Value readout on hover
//! - VQAnalyzer parity: YUV diff modes for comparing two streams
//!   - Absolute difference
//!   - Signed difference (green=positive, red=negative)
//!   - Amplified difference (10x gain)
//! - Debug YUV overlay: show coding info on blocks (CU depth, pred mode, MV, QP)

use bitvue_core::{BlockInfo, SelectionState};
use egui::{self, Color32, ColorImage, TextureHandle, Vec2};

/// Which component(s) to display
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum YuvViewMode {
    /// Show all three planes side by side
    All,
    /// Show only Y (luma)
    YOnly,
    /// Show only U (Cb)
    UOnly,
    /// Show only V (Cr)
    VOnly,
    /// Show UV combined (chroma)
    UVOnly,
}

/// YUV diff mode for comparing two streams (VQAnalyzer parity)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum YuvDiffMode {
    /// No diff - show current stream only
    #[default]
    None,
    /// Absolute difference |A - B|
    Absolute,
    /// Signed difference (green = A > B, red = A < B)
    Signed,
    /// Amplified difference (10x gain for subtle differences)
    Amplified,
    /// Side by side comparison
    SideBySide,
}

impl YuvDiffMode {
    pub fn label(&self) -> &'static str {
        match self {
            YuvDiffMode::None => "No Diff",
            YuvDiffMode::Absolute => "Absolute |A-B|",
            YuvDiffMode::Signed => "Signed (G+/R-)",
            YuvDiffMode::Amplified => "Amplified 10x",
            YuvDiffMode::SideBySide => "Side by Side",
        }
    }
}

impl YuvViewMode {
    pub fn label(&self) -> &'static str {
        match self {
            YuvViewMode::All => "Y | U | V",
            YuvViewMode::YOnly => "Y (Luma)",
            YuvViewMode::UOnly => "U (Cb)",
            YuvViewMode::VOnly => "V (Cr)",
            YuvViewMode::UVOnly => "UV (Chroma)",
        }
    }
}

/// YUV Component Viewer Panel
pub struct YuvViewerPanel {
    /// Current view mode
    view_mode: YuvViewMode,
    /// Show histogram
    show_histogram: bool,
    /// Zoom level
    zoom: f32,
    /// Cached textures for Y, U, V planes (Stream A)
    y_texture: Option<TextureHandle>,
    u_texture: Option<TextureHandle>,
    v_texture: Option<TextureHandle>,
    /// Frame dimensions
    frame_size: Option<(u32, u32)>,
    /// Cached histograms [Y, U, V] - 256 bins each
    histograms: Option<[[u32; 256]; 3]>,
    /// VQAnalyzer parity: YUV diff mode
    pub diff_mode: YuvDiffMode,
    /// Stream B Y plane data (for diff)
    y_data_b: Option<Vec<u8>>,
    /// Stream B U plane data (for diff)
    u_data_b: Option<Vec<u8>>,
    /// Stream B V plane data (for diff)
    v_data_b: Option<Vec<u8>>,
    /// Stream B frame size
    frame_size_b: Option<(u32, u32)>,
    /// Diff texture (computed difference)
    diff_texture: Option<TextureHandle>,
    /// Stream A raw Y plane data (for diff computation)
    y_data_a: Option<Vec<u8>>,
    /// Stream B textures for side-by-side
    y_texture_b: Option<TextureHandle>,
    /// Debug YUV overlay: show coding info on blocks
    pub show_debug_overlay: bool,
    /// Block info for current frame (for debug overlay)
    block_info: Option<Vec<BlockInfo>>,
}

impl Default for YuvViewerPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl YuvViewerPanel {
    pub fn new() -> Self {
        Self {
            view_mode: YuvViewMode::All,
            show_histogram: false,
            zoom: 1.0,
            y_texture: None,
            u_texture: None,
            v_texture: None,
            frame_size: None,
            histograms: None,
            diff_mode: YuvDiffMode::None,
            y_data_b: None,
            u_data_b: None,
            v_data_b: None,
            frame_size_b: None,
            diff_texture: None,
            y_data_a: None,
            y_texture_b: None,
            show_debug_overlay: false,
            block_info: None,
        }
    }

    /// Update textures from YUV data (Stream A - primary)
    pub fn set_yuv_data(
        &mut self,
        ctx: &egui::Context,
        y_plane: &[u8],
        u_plane: &[u8],
        v_plane: &[u8],
        width: u32,
        height: u32,
    ) {
        self.frame_size = Some((width, height));

        // Store raw Y data for diff computation
        self.y_data_a = Some(y_plane.to_vec());

        // Create Y plane texture (grayscale)
        let y_rgb: Vec<u8> = y_plane.iter().flat_map(|&y| [y, y, y]).collect();
        let y_image = ColorImage::from_rgb([width as usize, height as usize], &y_rgb);
        self.y_texture = Some(ctx.load_texture("yuv_y", y_image, egui::TextureOptions::LINEAR));

        // Create U plane texture (blue-ish for Cb)
        let uv_width = width / 2;
        let uv_height = height / 2;
        let u_rgb: Vec<u8> = u_plane
            .iter()
            .flat_map(|&u| {
                let val = u as i32 - 128;
                let b = (128 + val).clamp(0, 255) as u8;
                let g = (128 - val / 2).clamp(0, 255) as u8;
                [g, g, b]
            })
            .collect();
        let u_image = ColorImage::from_rgb([uv_width as usize, uv_height as usize], &u_rgb);
        self.u_texture = Some(ctx.load_texture("yuv_u", u_image, egui::TextureOptions::LINEAR));

        // Create V plane texture (red-ish for Cr)
        let v_rgb: Vec<u8> = v_plane
            .iter()
            .flat_map(|&v| {
                let val = v as i32 - 128;
                let r = (128 + val).clamp(0, 255) as u8;
                let g = (128 - val / 2).clamp(0, 255) as u8;
                [r, g, g]
            })
            .collect();
        let v_image = ColorImage::from_rgb([uv_width as usize, uv_height as usize], &v_rgb);
        self.v_texture = Some(ctx.load_texture("yuv_v", v_image, egui::TextureOptions::LINEAR));

        // Compute histograms
        let mut y_hist = [0u32; 256];
        let mut u_hist = [0u32; 256];
        let mut v_hist = [0u32; 256];

        for &val in y_plane {
            y_hist[val as usize] += 1;
        }
        for &val in u_plane {
            u_hist[val as usize] += 1;
        }
        for &val in v_plane {
            v_hist[val as usize] += 1;
        }

        self.histograms = Some([y_hist, u_hist, v_hist]);
    }

    /// Update YUV data for Stream B (comparison target)
    pub fn set_yuv_data_b(
        &mut self,
        ctx: &egui::Context,
        y_plane: &[u8],
        u_plane: &[u8],
        v_plane: &[u8],
        width: u32,
        height: u32,
    ) {
        self.frame_size_b = Some((width, height));

        // Store raw plane data for diff computation
        self.y_data_b = Some(y_plane.to_vec());
        self.u_data_b = Some(u_plane.to_vec());
        self.v_data_b = Some(v_plane.to_vec());

        // Create Y texture for side-by-side mode
        let y_rgb: Vec<u8> = y_plane.iter().flat_map(|&y| [y, y, y]).collect();
        let y_image = ColorImage::from_rgb([width as usize, height as usize], &y_rgb);
        self.y_texture_b = Some(ctx.load_texture("yuv_y_b", y_image, egui::TextureOptions::LINEAR));
    }

    /// Compute diff texture based on current diff mode
    fn compute_diff_texture(&mut self, ctx: &egui::Context) {
        let (y_a, y_b) = match (&self.y_data_a, &self.y_data_b) {
            (Some(a), Some(b)) => (a, b),
            _ => return,
        };

        let (width, height) = match self.frame_size {
            Some(s) => s,
            None => return,
        };

        // Ensure same dimensions
        if self.frame_size != self.frame_size_b {
            return;
        }

        let len = (width * height) as usize;
        if y_a.len() < len || y_b.len() < len {
            return;
        }

        let diff_rgb: Vec<u8> = match self.diff_mode {
            YuvDiffMode::None => return,

            YuvDiffMode::Absolute => {
                // |A - B| as grayscale
                y_a.iter()
                    .zip(y_b.iter())
                    .flat_map(|(&a, &b)| {
                        let diff = (a as i16 - b as i16).unsigned_abs() as u8;
                        [diff, diff, diff]
                    })
                    .collect()
            }

            YuvDiffMode::Signed => {
                // Green = A > B (positive), Red = A < B (negative)
                y_a.iter()
                    .zip(y_b.iter())
                    .flat_map(|(&a, &b)| {
                        let diff = a as i16 - b as i16;
                        if diff > 0 {
                            // Positive difference: green
                            let intensity = (diff.min(255) as u8).saturating_mul(2);
                            [0, intensity, 0]
                        } else if diff < 0 {
                            // Negative difference: red
                            let intensity = ((-diff).min(255) as u8).saturating_mul(2);
                            [intensity, 0, 0]
                        } else {
                            // No difference: black
                            [0, 0, 0]
                        }
                    })
                    .collect()
            }

            YuvDiffMode::Amplified => {
                // 10x amplified difference as grayscale
                y_a.iter()
                    .zip(y_b.iter())
                    .flat_map(|(&a, &b)| {
                        let diff = (a as i16 - b as i16).unsigned_abs();
                        let amplified = (diff * 10).min(255) as u8;
                        [amplified, amplified, amplified]
                    })
                    .collect()
            }

            YuvDiffMode::SideBySide => {
                // Side-by-side doesn't need a diff texture
                return;
            }
        };

        let diff_image = ColorImage::from_rgb([width as usize, height as usize], &diff_rgb);
        self.diff_texture = Some(ctx.load_texture("yuv_diff", diff_image, egui::TextureOptions::LINEAR));
    }

    /// Check if Stream B data is available for comparison
    pub fn has_stream_b(&self) -> bool {
        self.y_data_b.is_some()
    }

    /// Set block info for debug overlay
    pub fn set_block_info(&mut self, blocks: Vec<BlockInfo>) {
        self.block_info = Some(blocks);
    }

    /// Get block info at a pixel position
    fn get_block_at(&self, pixel_x: u32, pixel_y: u32) -> Option<&BlockInfo> {
        self.block_info.as_ref().and_then(|blocks| {
            blocks.iter().find(|b| {
                pixel_x >= b.x && pixel_x < b.x + b.width && pixel_y >= b.y && pixel_y < b.y + b.height
            })
        })
    }

    /// Clear Stream B data
    pub fn clear_stream_b(&mut self) {
        self.y_data_b = None;
        self.u_data_b = None;
        self.v_data_b = None;
        self.frame_size_b = None;
        self.y_texture_b = None;
        self.diff_texture = None;
        self.diff_mode = YuvDiffMode::None;
    }

    /// Show the YUV viewer panel
    pub fn show(&mut self, ui: &mut egui::Ui, _selection: &SelectionState) {
        ui.heading("YUV Viewer");
        ui.separator();

        // Toolbar
        ui.horizontal(|ui| {
            ui.label("Mode:");
            egui::ComboBox::from_id_salt("yuv_mode")
                .selected_text(self.view_mode.label())
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut self.view_mode,
                        YuvViewMode::All,
                        YuvViewMode::All.label(),
                    );
                    ui.selectable_value(
                        &mut self.view_mode,
                        YuvViewMode::YOnly,
                        YuvViewMode::YOnly.label(),
                    );
                    ui.selectable_value(
                        &mut self.view_mode,
                        YuvViewMode::UOnly,
                        YuvViewMode::UOnly.label(),
                    );
                    ui.selectable_value(
                        &mut self.view_mode,
                        YuvViewMode::VOnly,
                        YuvViewMode::VOnly.label(),
                    );
                    ui.selectable_value(
                        &mut self.view_mode,
                        YuvViewMode::UVOnly,
                        YuvViewMode::UVOnly.label(),
                    );
                });

            ui.separator();
            ui.checkbox(&mut self.show_histogram, "Histogram");

            ui.separator();
            ui.checkbox(&mut self.show_debug_overlay, "Debug")
                .on_hover_text("Show block coding info on hover");

            ui.separator();
            ui.label("Zoom:");
            if ui.button("-").clicked() {
                self.zoom = (self.zoom - 0.25).max(0.25);
            }
            ui.label(format!("{:.0}%", self.zoom * 100.0));
            if ui.button("+").clicked() {
                self.zoom = (self.zoom + 0.25).min(4.0);
            }

            // Diff mode selector (only show if Stream B is available)
            if self.has_stream_b() {
                ui.separator();
                ui.label("Diff:");
                let prev_mode = self.diff_mode;
                egui::ComboBox::from_id_salt("yuv_diff_mode")
                    .selected_text(self.diff_mode.label())
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut self.diff_mode,
                            YuvDiffMode::None,
                            YuvDiffMode::None.label(),
                        );
                        ui.selectable_value(
                            &mut self.diff_mode,
                            YuvDiffMode::Absolute,
                            YuvDiffMode::Absolute.label(),
                        );
                        ui.selectable_value(
                            &mut self.diff_mode,
                            YuvDiffMode::Signed,
                            YuvDiffMode::Signed.label(),
                        );
                        ui.selectable_value(
                            &mut self.diff_mode,
                            YuvDiffMode::Amplified,
                            YuvDiffMode::Amplified.label(),
                        );
                        ui.selectable_value(
                            &mut self.diff_mode,
                            YuvDiffMode::SideBySide,
                            YuvDiffMode::SideBySide.label(),
                        );
                    });

                // Clear diff texture when mode changes to recompute
                if prev_mode != self.diff_mode && self.diff_mode != YuvDiffMode::None {
                    self.diff_texture = None;
                }
            }
        });

        // Compute diff texture if needed (outside the closure)
        if self.diff_mode != YuvDiffMode::None
            && self.diff_mode != YuvDiffMode::SideBySide
            && self.diff_texture.is_none()
            && self.has_stream_b()
        {
            self.compute_diff_texture(ui.ctx());
        }

        ui.separator();

        // Check if we have textures
        let has_data = self.y_texture.is_some();

        if !has_data {
            ui.centered_and_justified(|ui| {
                ui.label(
                    egui::RichText::new("No YUV data available\nDecode a frame to view components")
                        .color(Color32::GRAY),
                );
            });
            return;
        }

        // Display planes based on view mode and diff mode
        egui::ScrollArea::both()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                // Handle diff modes first
                match self.diff_mode {
                    YuvDiffMode::SideBySide => {
                        // Show Stream A and Stream B side by side
                        ui.horizontal(|ui| {
                            self.render_plane(ui, "Stream A (Y)", &self.y_texture.clone(), 1.0);
                            ui.separator();
                            self.render_plane(ui, "Stream B (Y)", &self.y_texture_b.clone(), 1.0);
                        });
                        return;
                    }
                    YuvDiffMode::Absolute | YuvDiffMode::Signed | YuvDiffMode::Amplified => {
                        // Show diff texture
                        if self.diff_texture.is_some() {
                            let mode_label = match self.diff_mode {
                                YuvDiffMode::Absolute => "Diff |A-B|",
                                YuvDiffMode::Signed => "Diff (G+/R-)",
                                YuvDiffMode::Amplified => "Diff 10x",
                                _ => "Diff",
                            };
                            self.render_plane(ui, mode_label, &self.diff_texture.clone(), 1.0);
                            return;
                        }
                    }
                    YuvDiffMode::None => {}
                }

                // Normal view modes (no diff or diff not ready)
                match self.view_mode {
                    YuvViewMode::All => {
                        ui.horizontal(|ui| {
                            self.render_plane(ui, "Y (Luma)", &self.y_texture.clone(), 1.0);
                            ui.separator();
                            self.render_plane(ui, "U (Cb)", &self.u_texture.clone(), 2.0);
                            ui.separator();
                            self.render_plane(ui, "V (Cr)", &self.v_texture.clone(), 2.0);
                        });
                    }
                    YuvViewMode::YOnly => {
                        self.render_plane(ui, "Y (Luma)", &self.y_texture.clone(), 1.0);
                    }
                    YuvViewMode::UOnly => {
                        self.render_plane(ui, "U (Cb)", &self.u_texture.clone(), 1.0);
                    }
                    YuvViewMode::VOnly => {
                        self.render_plane(ui, "V (Cr)", &self.v_texture.clone(), 1.0);
                    }
                    YuvViewMode::UVOnly => {
                        ui.horizontal(|ui| {
                            self.render_plane(ui, "U (Cb)", &self.u_texture.clone(), 1.0);
                            ui.separator();
                            self.render_plane(ui, "V (Cr)", &self.v_texture.clone(), 1.0);
                        });
                    }
                }
            });

        // Show histogram if enabled
        if self.show_histogram {
            ui.separator();
            if let Some(histograms) = &self.histograms {
                ui.horizontal(|ui| {
                    self.render_histogram(
                        ui,
                        "Y",
                        &histograms[0],
                        Color32::from_rgb(200, 200, 200),
                    );
                    ui.separator();
                    self.render_histogram(
                        ui,
                        "U",
                        &histograms[1],
                        Color32::from_rgb(100, 100, 255),
                    );
                    ui.separator();
                    self.render_histogram(
                        ui,
                        "V",
                        &histograms[2],
                        Color32::from_rgb(255, 100, 100),
                    );
                });
            } else {
                ui.label(
                    egui::RichText::new("No histogram data - decode a frame first")
                        .small()
                        .color(Color32::GRAY),
                );
            }
        }
    }

    /// Render a histogram
    fn render_histogram(
        &self,
        ui: &mut egui::Ui,
        label: &str,
        histogram: &[u32; 256],
        color: Color32,
    ) {
        let hist_width = 256.0;
        let hist_height = 80.0;

        ui.vertical(|ui| {
            ui.label(egui::RichText::new(label).strong().small());

            let (rect, _response) =
                ui.allocate_exact_size(Vec2::new(hist_width, hist_height), egui::Sense::hover());

            if ui.is_rect_visible(rect) {
                let painter = ui.painter();

                // Background
                painter.rect_filled(rect, 0.0, Color32::from_rgb(30, 30, 35));

                // Find max for normalization
                let max_count = histogram.iter().max().copied().unwrap_or(1).max(1);

                // Draw histogram bars
                for (i, &count) in histogram.iter().enumerate() {
                    if count > 0 {
                        let x = rect.min.x + i as f32;
                        let bar_height = (count as f32 / max_count as f32) * hist_height;
                        let bar_rect = egui::Rect::from_min_max(
                            egui::pos2(x, rect.max.y - bar_height),
                            egui::pos2(x + 1.0, rect.max.y),
                        );
                        painter.rect_filled(bar_rect, 0.0, color);
                    }
                }

                // Draw border
                painter.rect_stroke(
                    rect,
                    0.0,
                    egui::Stroke::new(1.0, Color32::from_rgb(60, 60, 70)),
                );
            }

            // Stats
            let total: u64 = histogram.iter().map(|&c| c as u64).sum();
            let mean: f32 = if total > 0 {
                histogram
                    .iter()
                    .enumerate()
                    .map(|(i, &c)| i as u64 * c as u64)
                    .sum::<u64>() as f32
                    / total as f32
            } else {
                0.0
            };
            ui.label(
                egui::RichText::new(format!("μ={:.1}", mean))
                    .small()
                    .color(Color32::GRAY),
            );
        });
    }

    /// Render a single plane
    fn render_plane(
        &self,
        ui: &mut egui::Ui,
        label: &str,
        texture: &Option<TextureHandle>,
        scale_factor: f32,
    ) {
        ui.vertical(|ui| {
            ui.label(egui::RichText::new(label).strong());

            if let Some(tex) = texture {
                let size = tex.size();
                let display_size = Vec2::new(
                    size[0] as f32 * self.zoom * scale_factor,
                    size[1] as f32 * self.zoom * scale_factor,
                );

                let (rect, response) = ui.allocate_exact_size(display_size, egui::Sense::hover());

                if ui.is_rect_visible(rect) {
                    let painter = ui.painter();

                    // Draw the texture
                    painter.image(
                        tex.id(),
                        rect,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        Color32::WHITE,
                    );

                    // Draw debug overlay (block boundaries) if enabled
                    if self.show_debug_overlay {
                        if let Some(blocks) = &self.block_info {
                            let zoom_scale = self.zoom * scale_factor;
                            for block in blocks {
                                let block_rect = egui::Rect::from_min_size(
                                    egui::pos2(
                                        rect.min.x + block.x as f32 * zoom_scale,
                                        rect.min.y + block.y as f32 * zoom_scale,
                                    ),
                                    Vec2::new(
                                        block.width as f32 * zoom_scale,
                                        block.height as f32 * zoom_scale,
                                    ),
                                );

                                // Draw block boundary
                                painter.rect_stroke(
                                    block_rect,
                                    0.0,
                                    egui::Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 0, 128)),
                                );
                            }
                        }
                    }
                }

                // Show pixel value and block info on hover
                if let Some(pos) = response.hover_pos() {
                    let local_pos = pos - rect.min;
                    let pixel_x = (local_pos.x / self.zoom / scale_factor) as u32;
                    let pixel_y = (local_pos.y / self.zoom / scale_factor) as u32;

                    // Highlight hovered block if debug overlay is enabled
                    if self.show_debug_overlay {
                        if let Some(block) = self.get_block_at(pixel_x, pixel_y) {
                            let zoom_scale = self.zoom * scale_factor;
                            let block_rect = egui::Rect::from_min_size(
                                egui::pos2(
                                    rect.min.x + block.x as f32 * zoom_scale,
                                    rect.min.y + block.y as f32 * zoom_scale,
                                ),
                                Vec2::new(
                                    block.width as f32 * zoom_scale,
                                    block.height as f32 * zoom_scale,
                                ),
                            );

                            // Highlight current block
                            ui.painter().rect_stroke(
                                block_rect,
                                0.0,
                                egui::Stroke::new(2.0, Color32::from_rgb(0, 255, 255)),
                            );
                        }
                    }

                    response.on_hover_ui(|ui| {
                        ui.label(format!("Position: ({}, {})", pixel_x, pixel_y));

                        // Show block info if debug overlay is enabled
                        if self.show_debug_overlay {
                            if let Some(block) = self.get_block_at(pixel_x, pixel_y) {
                                ui.separator();
                                ui.label(egui::RichText::new("Block Info").strong());
                                ui.label(format!(
                                    "Size: {}x{} @ ({}, {})",
                                    block.width, block.height, block.x, block.y
                                ));
                                ui.label(format!("Pred Mode: {:?}", block.prediction_mode));
                                if let Some(qp) = block.qp {
                                    ui.label(format!("QP: {}", qp));
                                }
                                if let Some(bits) = block.bits {
                                    ui.label(format!("Bits: {}", bits));
                                }
                                if let Some(ref mv) = &block.motion_vector {
                                    ui.label(format!("MV: ({:.1}, {:.1})", mv.x, mv.y));
                                    ui.label(format!("Ref Frame: {}", mv.ref_frame));
                                }
                            }
                        }
                    });
                }
            } else {
                ui.label(
                    egui::RichText::new("No data")
                        .color(Color32::GRAY)
                        .italics(),
                );
            }
        });
    }

    /// Clear cached data
    pub fn clear(&mut self) {
        self.y_texture = None;
        self.u_texture = None;
        self.v_texture = None;
        self.frame_size = None;
        self.histograms = None;
        // Also clear diff-related data
        self.y_data_a = None;
        self.clear_stream_b();
        // Clear block info
        self.block_info = None;
    }
}
