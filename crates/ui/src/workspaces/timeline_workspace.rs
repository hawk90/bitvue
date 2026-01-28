//! Timeline Workspace - WS_TIMELINE_TEMPORAL (Monster Pack v9)
//!
//! Multi-lane temporal visualization with:
//! - Base layer: Frame size bars
//! - Overlay lanes: QP, bpp, diagnostics density, reorder band
//! - Scene change markers
//! - Global cursor
//! - Range selection (Shift+drag)

use bitvue_core::{Command, ContainerModel, SelectionState, StreamId, UnitNode};
use egui;

/// Professional color palette - EXACTLY matching Elecard StreamEye
#[allow(dead_code)]
mod colors {
    use egui::Color32;

    // Background & Grid (Light theme like Elecard)
    pub const BACKGROUND: Color32 = Color32::from_rgb(242, 242, 242); // Light gray background
    pub const GRID_MAJOR: Color32 = Color32::from_rgb(200, 200, 200); // Visible gridlines
    pub const GRID_MINOR: Color32 = Color32::from_rgb(220, 220, 220); // Subtle gridlines

    // Elecard Bar Chart Colors (Stacked)
    pub const BAR_TOTAL_SIZE: Color32 = Color32::from_rgb(102, 102, 204); // Blue/purple
    pub const BAR_SPLIT_CU: Color32 = Color32::from_rgb(255, 140, 60); // Orange
    pub const BAR_SPLIT_QT: Color32 = Color32::from_rgb(180, 180, 180); // Gray
    pub const BAR_MTT_SPLIT: Color32 = Color32::from_rgb(255, 200, 60); // Yellow
    pub const BAR_SMTT_SPLIT: Color32 = Color32::from_rgb(140, 180, 220); // Light blue
    pub const BAR_NON_INTER: Color32 = Color32::from_rgb(100, 180, 100); // Green
    pub const BAR_CU_SKIP: Color32 = Color32::from_rgb(140, 100, 140); // Purple

    // Line Graph (Bit Allocation)
    pub const LINE_BIT_ALLOC: Color32 = Color32::from_rgb(60, 60, 220); // Bright blue line
    pub const LINE_AREA_FILL: Color32 = Color32::from_rgba_premultiplied(60, 60, 220, 30); // Light blue fill

    // Selection & Cursor
    pub const SELECTION: Color32 = Color32::from_rgb(255, 100, 100); // Red selection
    pub const SELECTION_HIGHLIGHT: Color32 = Color32::from_rgba_premultiplied(255, 200, 200, 80);
    pub const CURSOR: Color32 = Color32::from_rgb(50, 50, 50); // Dark cursor line
    pub const CURSOR_SHADOW: Color32 = Color32::from_rgba_premultiplied(0, 0, 0, 60);

    // Numbered Markers (Elecard style)
    pub const MARKER_RED: Color32 = Color32::from_rgb(220, 60, 60); // Red marker
    pub const MARKER_GREEN: Color32 = Color32::from_rgb(60, 220, 60); // Green marker
    pub const MARKER_YELLOW: Color32 = Color32::from_rgb(220, 220, 60); // Yellow marker
    pub const MARKER_RING_RED: Color32 = Color32::from_rgb(255, 100, 100);
    pub const MARKER_RING_GREEN: Color32 = Color32::from_rgb(100, 255, 100);
    pub const MARKER_RING_YELLOW: Color32 = Color32::from_rgb(255, 255, 100);

    // Scene change markers
    pub const MARKER_SCENE_CHANGE: Color32 = Color32::from_rgb(60, 220, 60); // Green scene markers

    // Diamond Track
    pub const DIAMOND_REGULAR: Color32 = Color32::from_rgb(100, 60, 0); // Brown diamonds
    pub const DIAMOND_SELECTED: Color32 = Color32::from_rgb(220, 60, 60); // Red selected

    // UI Elements
    pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(40, 40, 40); // Dark text
    pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(100, 100, 100); // Gray text
    pub const BUTTON_ACTIVE: Color32 = Color32::from_rgb(100, 150, 255);
    pub const BUTTON_INACTIVE: Color32 = Color32::from_rgb(220, 220, 220);
}

/// Stacked bar metrics (rendered in one bar, stacked vertically)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StackedMetric {
    TotalSize,    // Blue - base layer
    SplitCuFlag,  // Orange
    SplitQtFlag,  // Gray
    MttSplitCu,   // Yellow
    SmttSplitCu,  // Light blue
    NonInterFlag, // Green
    CuSkipFlag,   // Purple
}

impl StackedMetric {
    pub fn label(&self) -> &'static str {
        match self {
            StackedMetric::TotalSize => "total_size",
            StackedMetric::SplitCuFlag => "split_cu_flag",
            StackedMetric::SplitQtFlag => "split_qt_flag",
            StackedMetric::MttSplitCu => "mtt_split_cu_vertical_flag",
            StackedMetric::SmttSplitCu => "smtt_split_cu_binary_flag",
            StackedMetric::NonInterFlag => "non_inter_flag",
            StackedMetric::CuSkipFlag => "cu_skip_flag",
        }
    }

    pub fn color(&self) -> egui::Color32 {
        match self {
            StackedMetric::TotalSize => colors::BAR_TOTAL_SIZE,
            StackedMetric::SplitCuFlag => colors::BAR_SPLIT_CU,
            StackedMetric::SplitQtFlag => colors::BAR_SPLIT_QT,
            StackedMetric::MttSplitCu => colors::BAR_MTT_SPLIT,
            StackedMetric::SmttSplitCu => colors::BAR_SMTT_SPLIT,
            StackedMetric::NonInterFlag => colors::BAR_NON_INTER,
            StackedMetric::CuSkipFlag => colors::BAR_CU_SKIP,
        }
    }

    /// All metrics in order (bottom to top)
    pub fn all_metrics() -> Vec<StackedMetric> {
        vec![
            StackedMetric::TotalSize,
            StackedMetric::SplitCuFlag,
            StackedMetric::SplitQtFlag,
            StackedMetric::MttSplitCu,
            StackedMetric::SmttSplitCu,
            StackedMetric::NonInterFlag,
            StackedMetric::CuSkipFlag,
        ]
    }
}

/// Visualization mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisualizationMode {
    BarChart,  // Stacked bars
    AreaChart, // Filled area under line
    LineGraph, // Line only
}

/// Timeline workspace state
pub struct TimelineWorkspace {
    /// Zoom factor (1.0 = normal, >1.0 = zoomed in)
    zoom_factor: f32,

    /// Global cursor position (frame index)
    cursor_position: Option<usize>,

    /// Range selection (start frame, end frame)
    selected_range: Option<(usize, usize)>,

    /// Scroll position (horizontal offset in pixels)
    #[allow(dead_code)]
    scroll_x: f32,

    /// Drag state for range selection
    drag_start: Option<(egui::Pos2, usize)>,

    /// Scene change markers (frame indices with confidence 0.0-1.0)
    scene_changes: Vec<(usize, f32)>,

    /// Show scene change markers
    show_scene_changes: bool,

    /// Show global cursor
    show_cursor: bool,

    /// Interest markers (bookmarks/errors) - frame index, label, inner color, ring color
    interest_markers: Vec<(usize, String, egui::Color32, egui::Color32)>,

    /// Visualization mode
    #[allow(dead_code)]
    viz_mode: VisualizationMode,

    /// Show line graph overlay
    show_line_overlay: bool,

    /// Active stacked metrics (visible in legend)
    active_metrics: Vec<StackedMetric>,
}

impl TimelineWorkspace {
    pub fn new() -> Self {
        Self {
            zoom_factor: 1.0,
            cursor_position: None,
            selected_range: None,
            scroll_x: 0.0,
            drag_start: None,
            scene_changes: vec![
                // Mock scene change data (frame_idx, confidence)
                (5, 0.95),
                (12, 0.88),
                (23, 0.92),
                (34, 0.78),
            ],
            show_scene_changes: true,
            show_cursor: true,
            interest_markers: vec![
                // Mock interest markers (frame_idx, label, inner_color, ring_color)
                (
                    8,
                    "1".to_string(),
                    colors::MARKER_GREEN,
                    colors::MARKER_RING_GREEN,
                ),
                (
                    15,
                    "2".to_string(),
                    colors::MARKER_RED,
                    colors::MARKER_RING_RED,
                ),
                (
                    28,
                    "3".to_string(),
                    colors::MARKER_YELLOW,
                    colors::MARKER_RING_YELLOW,
                ),
            ],
            viz_mode: VisualizationMode::AreaChart,
            show_line_overlay: true,
            active_metrics: StackedMetric::all_metrics(), // All metrics visible by default
        }
    }

    /// Show the timeline workspace
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        units: Option<&[UnitNode]>,
        container: Option<&ContainerModel>,
        selection: &SelectionState,
    ) -> Option<Command> {
        let mut clicked_command = None;

        // Simple toolbar
        ui.horizontal(|ui| {
            ui.heading("⏱️ Timeline");
            ui.separator();

            ui.label(egui::RichText::new("Zoom:").color(colors::TEXT_SECONDARY));

            // Zoom buttons
            if ui.button(egui::RichText::new("Fit")).clicked() {
                self.zoom_factor = 0.0; // Auto-fit
            }
            if ui.button(egui::RichText::new("1:1")).clicked() {
                self.zoom_factor = 1.0;
            }
            if ui.button(egui::RichText::new("2:1")).clicked() {
                self.zoom_factor = 2.0;
            }

            ui.separator();

            // Line overlay toggle
            if ui
                .button(if self.show_line_overlay {
                    "Line: ON"
                } else {
                    "Line: OFF"
                })
                .clicked()
            {
                self.show_line_overlay = !self.show_line_overlay;
            }
        });

        ui.separator();

        // Collect frame data
        let frames = if let Some(units) = units {
            collect_frames(units)
        } else {
            Vec::new()
        };

        if frames.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.label(egui::RichText::new("No frames to display").color(egui::Color32::GRAY));
            });
            return None;
        }

        // Calculate bar width based on zoom (Elecard style - narrow bars!)
        let bar_width = if self.zoom_factor <= 0.0 {
            // Auto-fit mode - but cap max width
            let available_width = ui.available_width();
            (available_width / frames.len() as f32).clamp(3.0, 15.0) // 3-15px range
        } else {
            8.0 * self.zoom_factor // Narrower bars (8px instead of 20px)
        };

        let bar_spacing = 2.0;
        let graph_height = 100.0; // Height for line graph (MAIN visualization)
        let _bar_height_max = 20.0; // Height for bars (SMALL - under the line)
        let _lane_height = 24.0; // Height per overlay lane
        let axis_height = 20.0; // Height for time axis at bottom
        let marker_track_height = 12.0; // Height for diamond track

        // Total height = graph area + axis + diamond track (no separate lanes)
        let total_height = graph_height + axis_height + marker_track_height;

        // Find max frame size for scaling
        let max_size = frames.iter().map(|f| f.size).max().unwrap_or(1);

        // Handle mouse interactions before ScrollArea
        ui.input(|i| {
            let scroll_delta = i.smooth_scroll_delta;
            if scroll_delta.y.abs() > 0.1 && !i.modifiers.shift {
                // Wheel: Zoom (not Shift)
                let zoom_delta = scroll_delta.y * 0.002;
                self.zoom_factor = (self.zoom_factor + zoom_delta).clamp(0.5, 10.0);
            }
            // Shift+Wheel: Pan is handled automatically by ScrollArea
        });

        // Professional layout: Y-axis + scrollable content
        let scroll_output = ui
            .horizontal(|ui| {
                // Y-AXIS (Fixed, non-scrolling)
                ui.vertical(|ui| {
                    ui.set_width(52.0);
                    ui.add_space(10.0);

                    // Draw scale labels (top to bottom) - with comma formatting like Elecard
                    let max_bytes = max_size as f32;
                    let label_spacing = graph_height / 4.0;

                    for i in 0..=4 {
                        let value_bytes = (max_bytes * (4 - i) as f32) / 4.0;
                        let value_int = value_bytes as u32;

                        // Add comma formatting for thousands
                        let formatted = if value_int >= 1000 {
                            let thousands = value_int / 1000;
                            let remainder = value_int % 1000;
                            format!("{},{:03}", thousands, remainder)
                        } else {
                            format!("{}", value_int)
                        };

                        ui.label(
                            egui::RichText::new(formatted)
                                .size(9.0)
                                .color(colors::TEXT_PRIMARY),
                        );
                        if i < 4 {
                            ui.add_space(label_spacing - 12.0); // Subtract label height
                        }
                    }

                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new("Size")
                            .size(8.0)
                            .color(colors::TEXT_SECONDARY),
                    );
                });

                // Scrollable timeline canvas
                let scroll_output = egui::ScrollArea::horizontal()
                    .auto_shrink([false, false])
                    .max_height(total_height + 40.0)
                    .drag_to_scroll(!ui.input(|i| i.modifiers.shift))
                    .show(ui, |ui| {
                        // Calculate total content width
                        let content_width = frames.len() as f32 * (bar_width + bar_spacing);

                        // === BACKGROUND (Light gray like Elecard) ===
                        let bg_rect = egui::Rect::from_min_size(
                            ui.cursor().min,
                            egui::vec2(content_width, total_height),
                        );
                        ui.painter().rect_filled(bg_rect, 0.0, colors::BACKGROUND);

                        // === BACKGROUND GRID ===
                        // Horizontal gridlines (more visible like Elecard)
                        let grid_y_start = ui.cursor().top();
                        let grid_step = graph_height / 4.0;
                        for i in 0..=4 {
                            let y_pos = grid_y_start + (i as f32 * grid_step);
                            ui.painter().line_segment(
                                [egui::pos2(0.0, y_pos), egui::pos2(content_width, y_pos)],
                                egui::Stroke::new(
                                    1.0,
                                    if i == 0 || i == 4 {
                                        colors::GRID_MAJOR
                                    } else {
                                        colors::GRID_MINOR
                                    },
                                ),
                            );
                        }

                        // Vertical gridlines at timecode positions
                        let fps = 30.0;
                        let timecode_interval = 680.0 / 1000.0; // ~0.68 seconds like Elecard
                        let frames_per_gridline = (timecode_interval * fps) as usize;
                        for i in (0..frames.len()).step_by(frames_per_gridline.max(1)) {
                            let x_pos = i as f32 * (bar_width + bar_spacing);
                            ui.painter().line_segment(
                                [
                                    egui::pos2(x_pos, grid_y_start),
                                    egui::pos2(x_pos, grid_y_start + graph_height),
                                ],
                                egui::Stroke::new(0.5, colors::GRID_MINOR),
                            );
                        }

                        // Base layer: STACKED BARS (multiple metrics in one bar - Elecard style!)
                        ui.horizontal(|ui| {
                            // Store bar y-position for later rendering
                            let bars_y_base = grid_y_start + graph_height;

                            for frame in frames.iter() {
                                let is_selected = selection
                                    .unit
                                    .as_ref()
                                    .map(|uk| uk.offset == frame.offset)
                                    .unwrap_or(false);

                                // Allocate space for interaction FIRST to get the rect
                                let (rect, response) = ui.allocate_exact_size(
                                    egui::vec2(bar_width, graph_height),
                                    egui::Sense::click(),
                                );

                                // Use the allocated rect for drawing
                                let x_pos = rect.min.x;

                                // Calculate total bar height (scaled to graph)
                                let total_bar_height =
                                    (frame.size as f32 / max_size as f32) * graph_height;

                                // Draw STACKED segments (each metric stacked vertically in one bar!)
                                let num_active = self.active_metrics.len() as f32;
                                let mut current_y = bars_y_base;

                                for _metric in self.active_metrics.iter() {
                                    // Each metric gets an equal portion (mock data - real app uses actual values)
                                    let segment_height = total_bar_height / num_active;

                                    if segment_height > 0.5 {
                                        let segment_rect = egui::Rect::from_min_size(
                                            egui::pos2(x_pos, current_y - segment_height),
                                            egui::vec2(bar_width, segment_height),
                                        );

                                        // Draw stacked segment with its specific color
                                        ui.painter().rect_filled(
                                            segment_rect,
                                            0.0,
                                            _metric.color(),
                                        );

                                        current_y -= segment_height;
                                    }
                                }

                                // Selection: RED BORDER only (색깔 변경 안함!)
                                if is_selected {
                                    let sel_rect = egui::Rect::from_min_size(
                                        egui::pos2(x_pos, grid_y_start),
                                        egui::vec2(bar_width, graph_height),
                                    );
                                    ui.painter().rect_stroke(
                                        sel_rect,
                                        0.0,
                                        egui::Stroke::new(3.0, colors::SELECTION), // 두꺼운 빨간 테두리
                                    );
                                }

                                if response.clicked() {
                                    // Update cursor position
                                    self.cursor_position = Some(frame.frame_index);

                                    clicked_command = Some(Command::SelectUnit {
                                        stream: StreamId::A,
                                        unit_key: frame.unit_key.clone(),
                                    });
                                }

                                response.on_hover_ui(|ui| {
                                    ui.label(format!("Frame #{}", frame.frame_index));
                                    ui.label(format!("Type: {}", frame.frame_type));
                                    ui.label(format!(
                                        "Size: {} bytes ({:.1} KB)",
                                        frame.size,
                                        frame.size as f32 / 1024.0
                                    ));
                                    if let Some(qp) = frame.qp_avg {
                                        ui.label(format!("QP: {}", qp));
                                    }
                                    ui.label(format!("Offset: 0x{:08X}", frame.offset));
                                });

                                ui.add_space(bar_spacing);
                            }
                        });

                        // Extract dimensions for bpp calculation (reserved for future use)
                        let (_width, _height) = if let Some(c) = container {
                            (c.width, c.height)
                        } else {
                            (None, None)
                        };

                        // === DIAMOND TRACK (Bottom timeline - Elecard style) ===
                        ui.add_space(4.0);
                        let diamond_track_top = ui.cursor().top();
                        ui.horizontal(|ui| {
                            for (idx, _frame) in frames.iter().enumerate() {
                                // Allocate space first to get proper position
                                let (rect, _response) = ui.allocate_exact_size(
                                    egui::vec2(bar_width, marker_track_height),
                                    egui::Sense::hover(),
                                );

                                let x_center = rect.min.x + (bar_width / 2.0);
                                let marker_y = diamond_track_top + marker_track_height / 2.0;

                                // Diamond shape (Elecard uses brown diamonds)
                                let diamond_size = 3.0;

                                // Check if this frame is selected
                                let is_frame_selected = selection
                                    .unit
                                    .as_ref()
                                    .map(|uk| uk.offset == frames[idx].offset)
                                    .unwrap_or(false);

                                let diamond_color = if is_frame_selected {
                                    colors::DIAMOND_SELECTED
                                } else {
                                    colors::DIAMOND_REGULAR
                                };

                                // Draw diamond (4 points forming a diamond)
                                let diamond = [
                                    egui::pos2(x_center, marker_y - diamond_size), // Top
                                    egui::pos2(x_center + diamond_size, marker_y), // Right
                                    egui::pos2(x_center, marker_y + diamond_size), // Bottom
                                    egui::pos2(x_center - diamond_size, marker_y), // Left
                                ];
                                ui.painter().add(egui::Shape::convex_polygon(
                                    diamond.to_vec(),
                                    diamond_color,
                                    egui::Stroke::NONE,
                                ));

                                ui.add_space(bar_spacing);
                            }
                        });

                        // === TIME AXIS (Bottom labels - Elecard format) ===
                        ui.add_space(2.0);
                        let timecode_y = ui.cursor().top();
                        ui.horizontal(|ui| {
                            // Draw timecode labels at intervals (Elecard uses HH:MM:SS.mmm format)
                            let fps = 30.0;
                            let timecode_interval = 680.0 / 1000.0; // ~0.68 seconds
                            let frames_per_label = (timecode_interval * fps) as usize;

                            for idx in (0..frames.len()).step_by(frames_per_label.max(1)) {
                                let x_pos = idx as f32 * (bar_width + bar_spacing);

                                // Calculate timecode in HH:MM:SS.mmm format (Elecard style)
                                let seconds_total = idx as f32 / fps;
                                let hours = (seconds_total / 3600.0) as u32;
                                let mins = ((seconds_total % 3600.0) / 60.0) as u32;
                                let secs = (seconds_total % 60.0) as u32;
                                let millis = ((seconds_total % 1.0) * 1000.0) as u32;

                                let timecode =
                                    format!("{:02}:{:02}:{:02}.{:03}", hours, mins, secs, millis);

                                ui.painter().text(
                                    egui::pos2(x_pos, timecode_y),
                                    egui::Align2::LEFT_TOP,
                                    timecode,
                                    egui::FontId::proportional(9.0),
                                    colors::TEXT_PRIMARY,
                                );

                                ui.add_space(frames_per_label as f32 * (bar_width + bar_spacing));
                            }
                        });

                        // === OVERLAY LAYERS (drawn on top) ===

                        // Calculate canvas bounds for overlay layers (graph area only)
                        let canvas_top = grid_y_start;
                        let canvas_bottom = grid_y_start + graph_height;

                        // Layer 1: Scene change markers
                        if self.show_scene_changes {
                            for &(frame_idx, confidence) in &self.scene_changes {
                                if frame_idx < frames.len() {
                                    let x_pos = (frame_idx as f32 * (bar_width + bar_spacing))
                                        + (bar_width / 2.0);

                                    // Vertical line spanning entire canvas
                                    let line_start = egui::pos2(x_pos, canvas_top);
                                    let line_end = egui::pos2(x_pos, canvas_bottom);

                                    // Draw with opacity based on confidence
                                    let alpha = (confidence * 180.0) as u8;
                                    let marker_color = egui::Color32::from_rgba_premultiplied(
                                        120, 200, 120, alpha,
                                    );

                                    ui.painter().line_segment(
                                        [line_start, line_end],
                                        egui::Stroke::new(2.0, marker_color),
                                    );

                                    // Small triangle at top
                                    let triangle_size = 6.0;
                                    let triangle = [
                                        egui::pos2(x_pos, canvas_top),
                                        egui::pos2(
                                            x_pos - triangle_size,
                                            canvas_top + triangle_size,
                                        ),
                                        egui::pos2(
                                            x_pos + triangle_size,
                                            canvas_top + triangle_size,
                                        ),
                                    ];
                                    ui.painter().add(egui::Shape::convex_polygon(
                                        triangle.to_vec(),
                                        colors::MARKER_SCENE_CHANGE,
                                        egui::Stroke::NONE,
                                    ));
                                }
                            }
                        }

                        // Layer 2: Range selection highlight
                        if let Some((start, end)) = self.selected_range {
                            if start < frames.len() && end < frames.len() {
                                let start_x = start as f32 * (bar_width + bar_spacing);
                                let end_x = (end + 1) as f32 * (bar_width + bar_spacing);
                                let width = end_x - start_x;

                                let highlight_rect = egui::Rect::from_min_size(
                                    egui::pos2(start_x, canvas_top),
                                    egui::vec2(width, total_height),
                                );

                                // Semi-transparent overlay
                                ui.painter().rect_filled(
                                    highlight_rect,
                                    0.0,
                                    colors::SELECTION_HIGHLIGHT,
                                );

                                // Border
                                ui.painter().rect_stroke(
                                    highlight_rect,
                                    0.0,
                                    egui::Stroke::new(1.5, colors::SELECTION),
                                );
                            }
                        }

                        // Layer 3: Global cursor
                        if self.show_cursor {
                            if let Some(cursor_idx) = self.cursor_position {
                                if cursor_idx < frames.len() {
                                    let x_pos = (cursor_idx as f32 * (bar_width + bar_spacing))
                                        + (bar_width / 2.0);

                                    // Draw shadow first
                                    let shadow_start = egui::pos2(x_pos + 1.0, canvas_top);
                                    let shadow_end = egui::pos2(x_pos + 1.0, canvas_bottom);
                                    ui.painter().line_segment(
                                        [shadow_start, shadow_end],
                                        egui::Stroke::new(3.0, colors::CURSOR_SHADOW),
                                    );

                                    // Draw main cursor line
                                    let cursor_start = egui::pos2(x_pos, canvas_top);
                                    let cursor_end = egui::pos2(x_pos, canvas_bottom);
                                    ui.painter().line_segment(
                                        [cursor_start, cursor_end],
                                        egui::Stroke::new(2.0, colors::CURSOR),
                                    );

                                    // Cursor handle at top
                                    let handle_size = 8.0;
                                    let handle_rect = egui::Rect::from_center_size(
                                        egui::pos2(x_pos, canvas_top + handle_size),
                                        egui::vec2(handle_size * 2.0, handle_size * 2.0),
                                    );
                                    ui.painter().circle_filled(
                                        handle_rect.center(),
                                        handle_size,
                                        colors::CURSOR,
                                    );
                                    ui.painter().circle_stroke(
                                        handle_rect.center(),
                                        handle_size,
                                        egui::Stroke::new(1.0, colors::CURSOR_SHADOW),
                                    );
                                }
                            }
                        }

                        // Layer 4: Line Graph Overlay (Blue line - PRIMARY element like Elecard)
                        if self.show_line_overlay {
                            let mut line_points = Vec::new();
                            let line_y_base = grid_y_start + graph_height;

                            for (idx, frame) in frames.iter().enumerate() {
                                let x =
                                    (idx as f32 * (bar_width + bar_spacing)) + (bar_width / 2.0);
                                // Line spans FULL graph height
                                let normalized_height =
                                    (frame.size as f32 / max_size as f32) * graph_height;
                                let y = line_y_base - normalized_height;
                                line_points.push(egui::pos2(x, y));
                            }

                            // Draw area fill under line
                            if line_points.len() >= 2 {
                                let mut area_points = line_points.clone();
                                // Close the polygon at bottom
                                if let Some(last) = line_points.last() {
                                    area_points.push(egui::pos2(last.x, line_y_base));
                                }
                                if let Some(first) = line_points.first() {
                                    area_points.push(egui::pos2(first.x, line_y_base));
                                }

                                ui.painter().add(egui::Shape::convex_polygon(
                                    area_points,
                                    colors::LINE_AREA_FILL,
                                    egui::Stroke::NONE,
                                ));

                                // Draw blue line on top
                                ui.painter().add(egui::Shape::line(
                                    line_points,
                                    egui::Stroke::new(2.0, colors::LINE_BIT_ALLOC),
                                ));
                            }
                        }

                        // Layer 5: Numbered interest markers (HOLLOW RINGS - exactly like Elecard)
                        for (frame_idx, label, _inner_color, ring_color) in &self.interest_markers {
                            if *frame_idx < frames.len() {
                                let x_pos = (*frame_idx as f32 * (bar_width + bar_spacing))
                                    + (bar_width / 2.0);
                                let y_pos = canvas_top + 12.0; // Near top of canvas

                                let inner_radius = 8.0;
                                let ring_width = 2.5;
                                let circle_center = egui::pos2(x_pos, y_pos);

                                // White filled circle (background)
                                ui.painter().circle_filled(
                                    circle_center,
                                    inner_radius,
                                    egui::Color32::WHITE,
                                );

                                // Colored ring (thick stroke)
                                ui.painter().circle_stroke(
                                    circle_center,
                                    inner_radius,
                                    egui::Stroke::new(ring_width, *ring_color),
                                );

                                // Black number text
                                ui.painter().text(
                                    circle_center,
                                    egui::Align2::CENTER_CENTER,
                                    label,
                                    egui::FontId::proportional(10.0),
                                    egui::Color32::BLACK,
                                );
                            }
                        }
                    }); // Close ScrollArea.show()

                ui.separator();

                // === RIGHT: LEGEND PANEL (Elecard style) ===
                ui.vertical(|ui| {
                    ui.set_width(160.0);
                    ui.heading(
                        egui::RichText::new("Legend")
                            .size(11.0)
                            .color(colors::TEXT_PRIMARY),
                    );
                    ui.separator();

                    for metric in &self.active_metrics {
                        ui.horizontal(|ui| {
                            // Color box
                            let (color_rect, _) = ui
                                .allocate_exact_size(egui::vec2(14.0, 14.0), egui::Sense::hover());
                            ui.painter().rect_filled(color_rect, 2.0, metric.color());

                            // Label
                            ui.label(
                                egui::RichText::new(metric.label())
                                    .size(9.0)
                                    .color(colors::TEXT_SECONDARY),
                            );
                        });
                    }
                });

                scroll_output
            })
            .inner; // Close horizontal layout (Y-axis + ScrollArea + Legend)

        // Handle Shift+drag range selection on the scroll area
        if scroll_output
            .inner_rect
            .contains(ui.ctx().pointer_latest_pos().unwrap_or_default())
        {
            if ui
                .input(|i| i.modifiers.shift && i.pointer.button_down(egui::PointerButton::Primary))
            {
                // Shift+Drag: Range selection
                if let Some(pos) = ui.ctx().pointer_latest_pos() {
                    let x_offset = pos.x - scroll_output.inner_rect.min.x;
                    let frame_idx = (x_offset / (bar_width + bar_spacing)) as usize;
                    let frame_idx = frame_idx.min(frames.len().saturating_sub(1));

                    if self.drag_start.is_none() {
                        self.drag_start = Some((pos, frame_idx));
                    }
                }
            } else if let Some((_, start_frame)) = self.drag_start.take() {
                // Release: finalize range selection
                if let Some(pos) = ui.ctx().pointer_latest_pos() {
                    let x_offset = pos.x - scroll_output.inner_rect.min.x;
                    let end_frame = (x_offset / (bar_width + bar_spacing)) as usize;
                    let end_frame = end_frame.min(frames.len().saturating_sub(1));

                    if start_frame != end_frame {
                        let (start, end) = if start_frame < end_frame {
                            (start_frame, end_frame)
                        } else {
                            (end_frame, start_frame)
                        };
                        self.selected_range = Some((start, end));
                        tracing::info!("Selected frame range: {} to {}", start, end);
                    }
                }
            }
        }

        // Status bar
        ui.separator();
        ui.horizontal(|ui| {
            ui.label(format!("📊 {} frames", frames.len()));
            if let Some(uk) = &selection.unit {
                if let Some(frame) = frames.iter().find(|f| f.offset == uk.offset) {
                    ui.separator();
                    ui.label(format!("Frame #{}", frame.frame_index));
                }
            }

            // Show selected range
            if let Some((start, end)) = self.selected_range {
                ui.separator();
                ui.label(format!(
                    "Range: {} - {} ({} frames)",
                    start,
                    end,
                    end - start + 1
                ));
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(format!("Zoom: {:.0}%", self.zoom_factor * 100.0));
                ui.separator();
                ui.label(format!("{} frames", frames.len()));
            });
        });

        clicked_command
    }
}

impl Default for TimelineWorkspace {
    fn default() -> Self {
        Self::new()
    }
}

/// Frame information
#[derive(Debug, Clone)]
struct FrameInfo {
    frame_index: usize,
    frame_type: String,
    size: u64,
    offset: u64,
    unit_key: bitvue_core::UnitKey,
    qp_avg: Option<u8>,
}

/// Collect frames from unit tree
fn collect_frames(units: &[UnitNode]) -> Vec<FrameInfo> {
    let mut frames = Vec::new();

    fn collect_recursive(units: &[UnitNode], frames: &mut Vec<FrameInfo>, depth: usize) {
        for unit in units {
            // Log unit info for debugging
            if depth == 0 {
                tracing::debug!(
                    "Timeline: Processing unit type={} offset={} frame_index={:?} has_children={}",
                    unit.unit_type,
                    unit.offset,
                    unit.frame_index,
                    !unit.children.is_empty()
                );
            }

            // Collect frames from units that have frame_index
            if let Some(frame_idx) = unit.frame_index {
                // Use actual frame type from parser, fallback to heuristic
                let frame_type = unit.frame_type.clone().unwrap_or_else(|| {
                    // Fallback heuristic: frame 0 is usually a KEY frame, rest are INTER
                    if frame_idx == 0 {
                        "KEY".to_string()
                    } else {
                        "INTER".to_string()
                    }
                });

                frames.push(FrameInfo {
                    frame_index: frame_idx,
                    frame_type,
                    size: unit.size as u64,
                    offset: unit.offset,
                    unit_key: unit.key.clone(),
                    qp_avg: unit.qp_avg,
                });
            }

            // Recursively collect from children
            if !unit.children.is_empty() {
                collect_recursive(&unit.children, frames, depth + 1);
            }
        }
    }

    collect_recursive(units, &mut frames, 0);

    frames.sort_by_key(|f| f.frame_index);

    // Log collection result (trace level to avoid spam)
    tracing::trace!("Timeline: Collected {} frames from units", frames.len());

    frames
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stacked_metric_label() {
        assert_eq!(StackedMetric::TotalSize.label(), "total_size");
        assert_eq!(StackedMetric::SplitCuFlag.label(), "split_cu_flag");
        assert_eq!(StackedMetric::SplitQtFlag.label(), "split_qt_flag");
        assert_eq!(
            StackedMetric::MttSplitCu.label(),
            "mtt_split_cu_vertical_flag"
        );
        assert_eq!(
            StackedMetric::SmttSplitCu.label(),
            "smtt_split_cu_binary_flag"
        );
        assert_eq!(StackedMetric::NonInterFlag.label(), "non_inter_flag");
        assert_eq!(StackedMetric::CuSkipFlag.label(), "cu_skip_flag");
    }

    #[test]
    fn test_stacked_metric_all_metrics_count() {
        let metrics = StackedMetric::all_metrics();
        assert_eq!(metrics.len(), 7); // 7 metric types
    }

    #[test]
    fn test_stacked_metric_all_metrics_order() {
        let metrics = StackedMetric::all_metrics();
        // TotalSize should be first (base layer)
        assert_eq!(metrics[0], StackedMetric::TotalSize);
        // CuSkipFlag should be last
        assert_eq!(metrics[6], StackedMetric::CuSkipFlag);
    }

    #[test]
    fn test_stacked_metric_equality() {
        assert_eq!(StackedMetric::TotalSize, StackedMetric::TotalSize);
        assert_ne!(StackedMetric::TotalSize, StackedMetric::SplitCuFlag);
    }

    #[test]
    fn test_visualization_mode_equality() {
        assert_eq!(VisualizationMode::BarChart, VisualizationMode::BarChart);
        assert_ne!(VisualizationMode::BarChart, VisualizationMode::LineGraph);
        assert_ne!(VisualizationMode::AreaChart, VisualizationMode::LineGraph);
    }

    #[test]
    fn test_timeline_workspace_new_defaults() {
        let ws = TimelineWorkspace::new();

        // Verify zoom starts at 1.0
        assert!((ws.zoom_factor - 1.0).abs() < f32::EPSILON);

        // Verify no cursor initially
        assert!(ws.cursor_position.is_none());

        // Verify no range selection
        assert!(ws.selected_range.is_none());

        // Verify scene changes are shown by default
        assert!(ws.show_scene_changes);

        // Verify cursor is shown by default
        assert!(ws.show_cursor);

        // Verify viz mode is AreaChart by default
        assert_eq!(ws.viz_mode, VisualizationMode::AreaChart);
    }

    #[test]
    fn test_timeline_workspace_has_mock_scene_changes() {
        let ws = TimelineWorkspace::new();

        // Should have mock scene changes
        assert!(!ws.scene_changes.is_empty());
        // First scene change at frame 5
        assert_eq!(ws.scene_changes[0].0, 5);
    }

    #[test]
    fn test_timeline_workspace_has_mock_interest_markers() {
        let ws = TimelineWorkspace::new();

        // Should have mock interest markers
        assert!(!ws.interest_markers.is_empty());
        // 3 mock markers
        assert_eq!(ws.interest_markers.len(), 3);
    }

    #[test]
    fn test_timeline_workspace_all_metrics_active_by_default() {
        let ws = TimelineWorkspace::new();

        // All 7 metrics should be active
        assert_eq!(ws.active_metrics.len(), 7);
    }

    #[test]
    fn test_timeline_workspace_default() {
        // Verify Default trait works
        let ws: TimelineWorkspace = Default::default();
        assert!((ws.zoom_factor - 1.0).abs() < f32::EPSILON);
    }
}
