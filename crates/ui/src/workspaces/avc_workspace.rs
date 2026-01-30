//! AVC (H.264) Visualization Workspace
//!
//! VQAnalyzer parity: Complete H.264/AVC-specific analysis views
//! - Macroblock partitions (16x16 fixed MB size)
//! - Intra prediction modes (I_4x4, I_16x16, I_PCM)
//! - Inter prediction (P/B Skip, Merge, 16x16 to 4x4 partitions)
//! - Transform visualization (4x4, 8x8)
//! - Deblocking filter edge display
//! - CAVLC/CABAC entropy mode indication

use egui::{self, Color32, Rect, RichText, Stroke, Vec2};

use super::workspace_strategy::{
    ColorScheme, PartitionRenderer, PartitionData, ViewRenderer, ViewContext, ViewRenderResult,
};

/// AVC-specific color palette
mod colors {
    use egui::Color32;

    // Partition colors
    pub const MB_BOUNDARY: Color32 = Color32::from_rgb(255, 128, 0); // Orange
    pub const SUB_MB_BOUNDARY: Color32 = Color32::from_rgb(100, 149, 237); // Cornflower blue
    pub const TRANSFORM_BOUNDARY: Color32 = Color32::from_rgb(144, 238, 144); // Light green

    // Intra mode colors
    pub const INTRA_4X4: Color32 = Color32::from_rgb(147, 112, 219); // Medium purple
    pub const INTRA_16X16: Color32 = Color32::from_rgb(255, 215, 0); // Gold
    pub const INTRA_PCM: Color32 = Color32::from_rgb(255, 99, 71); // Tomato

    // Inter mode colors
    pub const INTER_SKIP: Color32 = Color32::from_rgb(50, 205, 50); // Lime green
    pub const INTER_P16X16: Color32 = Color32::from_rgb(30, 144, 255); // Dodger blue
    pub const INTER_P16X8: Color32 = Color32::from_rgb(65, 105, 225); // Royal blue
    pub const INTER_P8X16: Color32 = Color32::from_rgb(0, 191, 255); // Deep sky blue
    pub const INTER_P8X8: Color32 = Color32::from_rgb(70, 130, 180); // Steel blue
    pub const INTER_P8X4: Color32 = Color32::from_rgb(135, 206, 235); // Sky blue
    pub const INTER_P4X8: Color32 = Color32::from_rgb(176, 196, 222); // Light steel blue
    pub const INTER_P4X4: Color32 = Color32::from_rgb(176, 224, 230); // Powder blue

    pub const INTER_B_DIRECT: Color32 = Color32::from_rgb(220, 20, 60); // Crimson
    pub const INTER_B16X16: Color32 = Color32::from_rgb(255, 69, 0); // Orange red

    // Slice type colors
    pub const SLICE_I: Color32 = Color32::from_rgb(255, 0, 0); // Red
    pub const SLICE_P: Color32 = Color32::from_rgb(0, 255, 0); // Green
    pub const SLICE_B: Color32 = Color32::from_rgb(0, 0, 255); // Blue
}

/// AVC view modes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AvcView {
    #[default]
    Overview,
    Partitions,
    Predictions,
    Transform,
    Deblocking,
}

impl AvcView {
    fn label(&self) -> &'static str {
        match self {
            AvcView::Overview => "Overview",
            AvcView::Partitions => "Partitions",
            AvcView::Predictions => "Predictions",
            AvcView::Transform => "Transform",
            AvcView::Deblocking => "Deblocking",
        }
    }
}

/// H.264 slice type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AvcSliceType {
    #[default]
    I,
    P,
    B,
    SI,
    SP,
}

impl AvcSliceType {
    fn color(&self) -> Color32 {
        match self {
            AvcSliceType::I | AvcSliceType::SI => colors::SLICE_I,
            AvcSliceType::P | AvcSliceType::SP => colors::SLICE_P,
            AvcSliceType::B => colors::SLICE_B,
        }
    }

    fn label(&self) -> &'static str {
        match self {
            AvcSliceType::I => "I",
            AvcSliceType::P => "P",
            AvcSliceType::B => "B",
            AvcSliceType::SI => "SI",
            AvcSliceType::SP => "SP",
        }
    }
}

/// H.264 macroblock type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AvcMbType {
    // Intra modes
    #[default]
    I4x4,
    I16x16,
    IPCM,
    // P slice modes
    PSkip,
    P16x16,
    P16x8,
    P8x16,
    P8x8,
    P8x8ref0,
    // B slice modes
    BDirect16x16,
    B16x16,
    B16x8,
    B8x16,
    B8x8,
    BSkip,
}

impl AvcMbType {
    fn color(&self) -> Color32 {
        match self {
            AvcMbType::I4x4 => colors::INTRA_4X4,
            AvcMbType::I16x16 => colors::INTRA_16X16,
            AvcMbType::IPCM => colors::INTRA_PCM,
            AvcMbType::PSkip | AvcMbType::BSkip => colors::INTER_SKIP,
            AvcMbType::P16x16 => colors::INTER_P16X16,
            AvcMbType::P16x8 => colors::INTER_P16X8,
            AvcMbType::P8x16 => colors::INTER_P8X16,
            AvcMbType::P8x8 | AvcMbType::P8x8ref0 => colors::INTER_P8X8,
            AvcMbType::BDirect16x16 => colors::INTER_B_DIRECT,
            AvcMbType::B16x16 => colors::INTER_B16X16,
            AvcMbType::B16x8 | AvcMbType::B8x16 | AvcMbType::B8x8 => colors::INTER_P8X8,
        }
    }

    fn label(&self) -> &'static str {
        match self {
            AvcMbType::I4x4 => "I_4x4",
            AvcMbType::I16x16 => "I_16x16",
            AvcMbType::IPCM => "I_PCM",
            AvcMbType::PSkip => "P_Skip",
            AvcMbType::P16x16 => "P_16x16",
            AvcMbType::P16x8 => "P_16x8",
            AvcMbType::P8x16 => "P_8x16",
            AvcMbType::P8x8 => "P_8x8",
            AvcMbType::P8x8ref0 => "P_8x8ref0",
            AvcMbType::BDirect16x16 => "B_Direct",
            AvcMbType::B16x16 => "B_16x16",
            AvcMbType::B16x8 => "B_16x8",
            AvcMbType::B8x16 => "B_8x16",
            AvcMbType::B8x8 => "B_8x8",
            AvcMbType::BSkip => "B_Skip",
        }
    }

    fn is_intra(&self) -> bool {
        matches!(self, AvcMbType::I4x4 | AvcMbType::I16x16 | AvcMbType::IPCM)
    }

    fn is_skip(&self) -> bool {
        matches!(self, AvcMbType::PSkip | AvcMbType::BSkip)
    }
}

/// Intra 4x4 prediction mode (9 modes)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Intra4x4Mode {
    #[default]
    Vertical, // 0
    Horizontal,    // 1
    DC,            // 2
    DiagDownLeft,  // 3
    DiagDownRight, // 4
    VertRight,     // 5
    HorzDown,      // 6
    VertLeft,      // 7
    HorzUp,        // 8
}

impl Intra4x4Mode {
    fn from_u8(value: u8) -> Self {
        match value {
            0 => Intra4x4Mode::Vertical,
            1 => Intra4x4Mode::Horizontal,
            2 => Intra4x4Mode::DC,
            3 => Intra4x4Mode::DiagDownLeft,
            4 => Intra4x4Mode::DiagDownRight,
            5 => Intra4x4Mode::VertRight,
            6 => Intra4x4Mode::HorzDown,
            7 => Intra4x4Mode::VertLeft,
            8 => Intra4x4Mode::HorzUp,
            _ => Intra4x4Mode::DC,
        }
    }

    fn label(&self) -> &'static str {
        match self {
            Intra4x4Mode::Vertical => "V",
            Intra4x4Mode::Horizontal => "H",
            Intra4x4Mode::DC => "DC",
            Intra4x4Mode::DiagDownLeft => "DDL",
            Intra4x4Mode::DiagDownRight => "DDR",
            Intra4x4Mode::VertRight => "VR",
            Intra4x4Mode::HorzDown => "HD",
            Intra4x4Mode::VertLeft => "VL",
            Intra4x4Mode::HorzUp => "HU",
        }
    }
}

/// Intra 16x16 prediction mode (4 modes)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Intra16x16Mode {
    Vertical,   // 0
    Horizontal, // 1
    #[default]
    DC, // 2
    Plane,      // 3
}

impl Intra16x16Mode {
    fn label(&self) -> &'static str {
        match self {
            Intra16x16Mode::Vertical => "Vertical",
            Intra16x16Mode::Horizontal => "Horizontal",
            Intra16x16Mode::DC => "DC",
            Intra16x16Mode::Plane => "Plane",
        }
    }
}

/// H.264 macroblock data
#[derive(Debug, Clone)]
pub struct AvcMacroblock {
    /// MB index
    pub mb_idx: u32,
    /// Position (x, y) in pixels
    pub x: u32,
    pub y: u32,
    /// MB type
    pub mb_type: AvcMbType,
    /// QP value
    pub qp: u8,
    /// Intra 4x4 modes (16 blocks if I_4x4)
    pub intra4x4_modes: Option<[Intra4x4Mode; 16]>,
    /// Intra 16x16 mode (if I_16x16)
    pub intra16x16_mode: Option<Intra16x16Mode>,
    /// Transform 8x8 flag
    pub transform_8x8: bool,
    /// Coded Block Pattern
    pub cbp: u8,
}

impl Default for AvcMacroblock {
    fn default() -> Self {
        Self {
            mb_idx: 0,
            x: 0,
            y: 0,
            mb_type: AvcMbType::default(),
            qp: 26,
            intra4x4_modes: None,
            intra16x16_mode: None,
            transform_8x8: false,
            cbp: 0,
        }
    }
}

/// AVC-specific feature flags
#[derive(Debug, Clone, Default)]
pub struct AvcFeatureStatus {
    pub transform_8x8: bool,
    pub cabac_enabled: bool, // vs CAVLC
    pub deblocking_enabled: bool,
    pub weighted_pred: bool,
    pub weighted_bipred: bool,
    pub direct_8x8_inference: bool,
    pub frame_mbs_only: bool,
    pub mbaff: bool, // MB-adaptive frame-field
    pub fmo: bool,   // Flexible MB ordering
    pub aso: bool,   // Arbitrary slice ordering
}

/// AVC Visualization Workspace
pub struct AvcWorkspace {
    /// Active view
    active_view: AvcView,

    /// Show MB grid
    show_mb_grid: bool,

    /// Show sub-MB partitions
    show_sub_mb: bool,

    /// Show transform blocks
    show_transform: bool,

    /// Show prediction modes
    show_pred_modes: bool,

    /// Show deblocking edges
    show_deblocking: bool,

    /// Feature status
    features: AvcFeatureStatus,

    /// Mock macroblock data
    mock_mbs: Vec<AvcMacroblock>,

    /// Selected MB index
    selected_mb: Option<usize>,

    /// Current slice type
    slice_type: AvcSliceType,

    /// Frame dimensions
    frame_width: u32,
    frame_height: u32,

    /// Profile/Level info
    profile_idc: u8,
    level_idc: u8,

    /// Flag to track if mock data has been initialized
    mock_data_initialized: bool,
}

impl Default for AvcWorkspace {
    fn default() -> Self {
        Self::new()
    }
}

impl AvcWorkspace {
    pub fn new() -> Self {
        Self {
            active_view: AvcView::default(),
            show_mb_grid: true,
            show_sub_mb: false,
            show_transform: false,
            show_pred_modes: true,
            show_deblocking: false,
            features: AvcFeatureStatus::default(),
            mock_mbs: Vec::new(),
            selected_mb: None,
            slice_type: AvcSliceType::P,
            frame_width: 1920,
            frame_height: 1080,
            profile_idc: 100, // High profile
            level_idc: 41,    // Level 4.1
            mock_data_initialized: false,
        }
    }

    /// Set view mode by index (F1-F4 keyboard shortcuts)
    pub fn set_mode_by_index(&mut self, index: usize) {
        self.active_view = match index {
            0 => AvcView::Overview,
            1 => AvcView::Partitions,
            2 => AvcView::Predictions,
            3 => AvcView::Transform,
            _ => return, // Ignore invalid indices
        };
    }

    /// Ensure mock data is initialized (lazy loading)
    fn ensure_mock_data(&mut self) {
        if !self.mock_data_initialized {
            self.generate_mock_data();
            self.mock_data_initialized = true;
        }
    }

    /// Generate mock macroblock data for demonstration
    fn generate_mock_data(&mut self) {
        self.mock_mbs.clear();

        // Generate 4x4 grid of MBs (64x64 pixels)
        let mb_types = [
            AvcMbType::PSkip,
            AvcMbType::P16x16,
            AvcMbType::P16x8,
            AvcMbType::P8x16,
            AvcMbType::P8x8,
            AvcMbType::I4x4,
            AvcMbType::I16x16,
            AvcMbType::BDirect16x16,
        ];

        let mut mb_idx = 0u32;
        for row in 0..4u32 {
            for col in 0..4u32 {
                let mb_type = mb_types[(row * 4 + col) as usize % mb_types.len()];
                let mut mb = AvcMacroblock {
                    mb_idx,
                    x: col * 16,
                    y: row * 16,
                    mb_type,
                    qp: 22 + (mb_idx as u8 % 10),
                    transform_8x8: mb_idx % 3 == 0 && self.features.transform_8x8,
                    cbp: if mb_type.is_skip() { 0 } else { 0x2F },
                    ..Default::default()
                };

                // Add intra modes for intra MBs
                if matches!(mb_type, AvcMbType::I4x4) {
                    let mut modes = [Intra4x4Mode::DC; 16];
                    for (i, mode) in modes.iter_mut().enumerate() {
                        *mode = Intra4x4Mode::from_u8((i as u8 + mb_idx as u8) % 9);
                    }
                    mb.intra4x4_modes = Some(modes);
                } else if matches!(mb_type, AvcMbType::I16x16) {
                    mb.intra16x16_mode = Some(match mb_idx % 4 {
                        0 => Intra16x16Mode::DC,
                        1 => Intra16x16Mode::Horizontal,
                        2 => Intra16x16Mode::Vertical,
                        _ => Intra16x16Mode::Plane,
                    });
                }

                self.mock_mbs.push(mb);
                mb_idx += 1;
            }
        }

        // Set feature flags for High profile
        self.features = AvcFeatureStatus {
            transform_8x8: true,
            cabac_enabled: true,
            deblocking_enabled: true,
            weighted_pred: true,
            weighted_bipred: true,
            direct_8x8_inference: true,
            frame_mbs_only: true,
            mbaff: false,
            fmo: false,
            aso: false,
        };
    }

    /// Main UI entry point
    pub fn show(&mut self, ui: &mut egui::Ui) {
        // Lazy-load mock data on first show
        self.ensure_mock_data();

        ui.heading("AVC (H.264) Analysis");
        ui.separator();

        // View selector tabs
        ui.horizontal(|ui| {
            for view in [
                AvcView::Overview,
                AvcView::Partitions,
                AvcView::Predictions,
                AvcView::Transform,
                AvcView::Deblocking,
            ] {
                if ui
                    .selectable_label(self.active_view == view, view.label())
                    .clicked()
                {
                    self.active_view = view;
                }
            }
        });

        ui.separator();

        // Render active view
        match self.active_view {
            AvcView::Overview => self.show_overview(ui),
            AvcView::Partitions => self.show_partitions(ui),
            AvcView::Predictions => self.show_predictions(ui),
            AvcView::Transform => self.show_transform(ui),
            AvcView::Deblocking => self.show_deblocking(ui),
        }
    }

    /// Overview tab
    fn show_overview(&mut self, ui: &mut egui::Ui) {
        ui.columns(2, |cols| {
            // Left: Profile/Level and Features
            cols[0].group(|ui| {
                ui.heading("AVC Features");
                ui.add_space(8.0);

                ui.label(format!(
                    "Profile: {} ({})",
                    Self::profile_name(self.profile_idc),
                    self.profile_idc
                ));
                ui.label(format!(
                    "Level: {}.{}",
                    self.level_idc / 10,
                    self.level_idc % 10
                ));
                ui.add_space(8.0);

                Self::feature_badge(ui, "8x8 Transform", self.features.transform_8x8);
                Self::feature_badge(ui, "CABAC", self.features.cabac_enabled);
                Self::feature_badge(ui, "CAVLC", !self.features.cabac_enabled);
                Self::feature_badge(ui, "Deblocking", self.features.deblocking_enabled);
                Self::feature_badge(ui, "Weighted Pred", self.features.weighted_pred);
                Self::feature_badge(ui, "Weighted BiPred", self.features.weighted_bipred);
                Self::feature_badge(ui, "Direct 8x8", self.features.direct_8x8_inference);
                Self::feature_badge(ui, "Frame MBs Only", self.features.frame_mbs_only);
                Self::feature_badge(ui, "MBAFF", self.features.mbaff);
                Self::feature_badge(ui, "FMO", self.features.fmo);
            });

            // Right: Frame stats
            cols[1].group(|ui| {
                ui.heading("Frame Statistics");
                ui.add_space(8.0);

                ui.label(format!(
                    "Frame size: {}x{}",
                    self.frame_width, self.frame_height
                ));
                ui.label(format!(
                    "MBs: {} x {} = {}",
                    (self.frame_width + 15) / 16,
                    (self.frame_height + 15) / 16,
                    ((self.frame_width + 15) / 16) * ((self.frame_height + 15) / 16)
                ));

                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label("Slice Type:");
                    let badge = RichText::new(format!(" {} ", self.slice_type.label()))
                        .background_color(self.slice_type.color())
                        .color(Color32::WHITE);
                    ui.label(badge);
                });

                ui.add_space(8.0);

                // MB type distribution
                let mut intra_count = 0;
                let mut inter_count = 0;
                let mut skip_count = 0;

                for mb in &self.mock_mbs {
                    if mb.mb_type.is_intra() {
                        intra_count += 1;
                    } else if mb.mb_type.is_skip() {
                        skip_count += 1;
                    } else {
                        inter_count += 1;
                    }
                }

                ui.label(format!("Intra MBs: {}", intra_count));
                ui.label(format!("Inter MBs: {}", inter_count));
                ui.label(format!("Skip MBs: {}", skip_count));
            });
        });
    }

    /// Partitions tab - shows MB partitions
    fn show_partitions(&mut self, ui: &mut egui::Ui) {
        // Toolbar
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.show_mb_grid, "MB Grid");
            ui.checkbox(&mut self.show_sub_mb, "Sub-MB");
            ui.checkbox(&mut self.show_pred_modes, "Types");
        });

        ui.separator();

        // Partition visualization
        let available = ui.available_size();
        let grid_size = available.x.min(available.y - 100.0).min(400.0);

        let (response, painter) = ui.allocate_painter(
            Vec2::new(grid_size, grid_size),
            egui::Sense::click_and_drag(),
        );

        let rect = response.rect;

        // Background
        painter.rect_filled(rect, 4.0, Color32::from_rgb(30, 30, 35));

        // Scale factor: show 4x4 MBs (64x64 logical -> grid_size pixels)
        let display_size = 64.0;
        let scale = grid_size / display_size;

        // Draw MB boundaries if enabled
        if self.show_mb_grid {
            for x in (0..=64).step_by(16) {
                let px = rect.min.x + x as f32 * scale;
                painter.line_segment(
                    [egui::pos2(px, rect.min.y), egui::pos2(px, rect.max.y)],
                    Stroke::new(2.0, colors::MB_BOUNDARY),
                );
            }
            for y in (0..=64).step_by(16) {
                let py = rect.min.y + y as f32 * scale;
                painter.line_segment(
                    [egui::pos2(rect.min.x, py), egui::pos2(rect.max.x, py)],
                    Stroke::new(2.0, colors::MB_BOUNDARY),
                );
            }
        }

        // Draw MBs
        for (idx, mb) in self.mock_mbs.iter().enumerate() {
            if mb.x >= 64 || mb.y >= 64 {
                continue;
            }

            let mb_rect = Rect::from_min_size(
                egui::pos2(
                    rect.min.x + mb.x as f32 * scale,
                    rect.min.y + mb.y as f32 * scale,
                ),
                Vec2::splat(16.0 * scale),
            );

            // Fill with MB type color
            if self.show_pred_modes {
                let color = mb.mb_type.color().gamma_multiply(0.4);
                painter.rect_filled(mb_rect, 0.0, color);
            }

            // Draw sub-MB partitions for P8x8
            if self.show_sub_mb && matches!(mb.mb_type, AvcMbType::P8x8 | AvcMbType::P8x8ref0) {
                let half = 8.0 * scale;
                for sy in 0..2 {
                    for sx in 0..2 {
                        let sub_rect = Rect::from_min_size(
                            mb_rect.min + Vec2::new(sx as f32 * half, sy as f32 * half),
                            Vec2::splat(half),
                        );
                        painter.rect_stroke(
                            sub_rect,
                            0.0,
                            Stroke::new(1.0, colors::SUB_MB_BOUNDARY),
                        );
                    }
                }
            }

            // Draw type label
            if self.show_pred_modes {
                let label = mb.mb_type.label();
                // Truncate label if needed
                let display_label = if label.len() > 6 { &label[..6] } else { label };
                painter.text(
                    mb_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    display_label,
                    egui::FontId::proportional(8.0),
                    Color32::WHITE,
                );
            }

            // Highlight selected
            if Some(idx) == self.selected_mb {
                painter.rect_stroke(mb_rect, 0.0, Stroke::new(2.0, Color32::YELLOW));
            }
        }

        // Handle click for selection
        if response.clicked() {
            if let Some(pos) = response.interact_pointer_pos() {
                let rel_x = ((pos.x - rect.min.x) / scale) as u32;
                let rel_y = ((pos.y - rect.min.y) / scale) as u32;
                let mb_x = (rel_x / 16) * 16;
                let mb_y = (rel_y / 16) * 16;

                self.selected_mb = self
                    .mock_mbs
                    .iter()
                    .position(|mb| mb.x == mb_x && mb.y == mb_y);
            }
        }

        // Legend
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.label("Legend:");
            Self::legend_item(ui, colors::MB_BOUNDARY, "MB");
            Self::legend_item(ui, colors::SUB_MB_BOUNDARY, "Sub-MB");
        });

        // Selected MB info
        if let Some(idx) = self.selected_mb {
            if let Some(mb) = self.mock_mbs.get(idx) {
                ui.add_space(8.0);
                ui.group(|ui| {
                    ui.heading(format!("MB #{}", mb.mb_idx));
                    ui.label(format!("Position: ({}, {})", mb.x, mb.y));
                    ui.label(format!("Type: {}", mb.mb_type.label()));
                    ui.label(format!("QP: {}", mb.qp));
                    ui.label(format!("CBP: 0x{:02X}", mb.cbp));
                    if mb.transform_8x8 {
                        ui.label("Transform: 8x8");
                    } else {
                        ui.label("Transform: 4x4");
                    }
                });
            }
        }
    }

    /// Predictions tab
    fn show_predictions(&mut self, ui: &mut egui::Ui) {
        ui.columns(2, |cols| {
            // Left: Intra modes
            cols[0].group(|ui| {
                ui.heading("Intra Prediction");
                ui.add_space(4.0);

                ui.label(RichText::new("I_4x4 (9 modes)").strong());
                ui.horizontal(|ui| {
                    for mode in [
                        Intra4x4Mode::Vertical,
                        Intra4x4Mode::Horizontal,
                        Intra4x4Mode::DC,
                    ] {
                        Self::mode_badge(ui, colors::INTRA_4X4, mode.label());
                    }
                });
                ui.horizontal(|ui| {
                    for mode in [
                        Intra4x4Mode::DiagDownLeft,
                        Intra4x4Mode::DiagDownRight,
                        Intra4x4Mode::VertRight,
                    ] {
                        Self::mode_badge(ui, colors::INTRA_4X4, mode.label());
                    }
                });
                ui.horizontal(|ui| {
                    for mode in [
                        Intra4x4Mode::HorzDown,
                        Intra4x4Mode::VertLeft,
                        Intra4x4Mode::HorzUp,
                    ] {
                        Self::mode_badge(ui, colors::INTRA_4X4, mode.label());
                    }
                });

                ui.add_space(8.0);

                ui.label(RichText::new("I_16x16 (4 modes)").strong());
                ui.horizontal(|ui| {
                    Self::mode_badge(ui, colors::INTRA_16X16, "V");
                    Self::mode_badge(ui, colors::INTRA_16X16, "H");
                    Self::mode_badge(ui, colors::INTRA_16X16, "DC");
                    Self::mode_badge(ui, colors::INTRA_16X16, "Plane");
                });

                ui.add_space(8.0);

                ui.label(RichText::new("I_PCM").strong());
                ui.label("Raw sample values, no prediction");
            });

            // Right: Inter modes
            cols[1].group(|ui| {
                ui.heading("Inter Prediction");
                ui.add_space(4.0);

                ui.label(RichText::new("P Slice Modes").strong());
                ui.horizontal(|ui| {
                    Self::mode_badge(ui, colors::INTER_SKIP, "Skip");
                    Self::mode_badge(ui, colors::INTER_P16X16, "16x16");
                });
                ui.horizontal(|ui| {
                    Self::mode_badge(ui, colors::INTER_P16X8, "16x8");
                    Self::mode_badge(ui, colors::INTER_P8X16, "8x16");
                });
                ui.horizontal(|ui| {
                    Self::mode_badge(ui, colors::INTER_P8X8, "8x8");
                });

                ui.add_space(8.0);

                ui.label(RichText::new("B Slice Modes").strong());
                ui.horizontal(|ui| {
                    Self::mode_badge(ui, colors::INTER_SKIP, "Skip");
                    Self::mode_badge(ui, colors::INTER_B_DIRECT, "Direct");
                });
                ui.horizontal(|ui| {
                    Self::mode_badge(ui, colors::INTER_B16X16, "16x16");
                    Self::mode_badge(ui, colors::INTER_P16X8, "16x8");
                });

                ui.add_space(8.0);

                ui.label(RichText::new("Sub-MB Partitions (8x8 block)").strong());
                ui.horizontal(|ui| {
                    Self::mode_badge(ui, colors::INTER_P8X8, "8x8");
                    Self::mode_badge(ui, colors::INTER_P8X4, "8x4");
                });
                ui.horizontal(|ui| {
                    Self::mode_badge(ui, colors::INTER_P4X8, "4x8");
                    Self::mode_badge(ui, colors::INTER_P4X4, "4x4");
                });
            });
        });
    }

    /// Transform tab
    fn show_transform(&mut self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.heading("Transform Sizes");
            ui.add_space(4.0);

            ui.label("H.264 supports two transform sizes:");
            ui.add_space(4.0);

            ui.horizontal(|ui| {
                ui.label("4x4 DCT:");
                ui.label("Always available (Baseline, Main, High)");
            });
            ui.horizontal(|ui| {
                ui.label("8x8 DCT:");
                if self.features.transform_8x8 {
                    ui.label(RichText::new("Enabled (High profile+)").color(Color32::GREEN));
                } else {
                    ui.label(RichText::new("Disabled").color(Color32::GRAY));
                }
            });
        });

        ui.add_space(8.0);

        // Transform visualization
        ui.group(|ui| {
            ui.heading("Transform Block Visualization");

            let (_, painter) = ui.allocate_painter(Vec2::new(200.0, 200.0), egui::Sense::hover());

            let rect = painter.clip_rect();

            // Show a 16x16 MB with different transform patterns
            let mb_size = rect.width();

            // Background
            painter.rect_filled(rect, 0.0, Color32::from_rgb(40, 40, 45));

            // Left half: 4x4 transforms (4x4 grid)
            let block_4x4 = mb_size / 8.0;
            for y in 0..4 {
                for x in 0..4 {
                    let block_rect = Rect::from_min_size(
                        rect.min + Vec2::new(x as f32 * block_4x4, y as f32 * block_4x4),
                        Vec2::splat(block_4x4),
                    );
                    painter.rect_stroke(
                        block_rect,
                        0.0,
                        Stroke::new(1.0, colors::TRANSFORM_BOUNDARY),
                    );
                }
            }

            // Right half: 8x8 transforms (2x2 grid)
            let block_8x8 = mb_size / 4.0;
            let offset_x = mb_size / 2.0;
            for y in 0..2 {
                for x in 0..2 {
                    let block_rect = Rect::from_min_size(
                        rect.min + Vec2::new(offset_x + x as f32 * block_8x8, y as f32 * block_8x8),
                        Vec2::splat(block_8x8),
                    );
                    painter.rect_stroke(block_rect, 0.0, Stroke::new(2.0, Color32::YELLOW));
                }
            }

            // Labels
            painter.text(
                rect.min + Vec2::new(mb_size / 4.0, mb_size + 15.0),
                egui::Align2::CENTER_CENTER,
                "4x4 DCT",
                egui::FontId::proportional(11.0),
                Color32::WHITE,
            );
            painter.text(
                rect.min + Vec2::new(mb_size * 3.0 / 4.0, mb_size + 15.0),
                egui::Align2::CENTER_CENTER,
                "8x8 DCT",
                egui::FontId::proportional(11.0),
                Color32::YELLOW,
            );
        });

        ui.add_space(8.0);

        ui.group(|ui| {
            ui.heading("Entropy Coding");
            ui.add_space(4.0);

            if self.features.cabac_enabled {
                ui.horizontal(|ui| {
                    let badge = RichText::new(" CABAC ")
                        .background_color(Color32::from_rgb(50, 205, 50))
                        .color(Color32::WHITE);
                    ui.label(badge);
                    ui.label("Context-Adaptive Binary Arithmetic Coding");
                });
                ui.label("- Better compression, higher complexity");
                ui.label("- Required for High profile");
            } else {
                ui.horizontal(|ui| {
                    let badge = RichText::new(" CAVLC ")
                        .background_color(Color32::from_rgb(255, 165, 0))
                        .color(Color32::WHITE);
                    ui.label(badge);
                    ui.label("Context-Adaptive Variable Length Coding");
                });
                ui.label("- Lower complexity, faster decode");
                ui.label("- Used in Baseline profile");
            }
        });
    }

    /// Deblocking tab
    fn show_deblocking(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.show_deblocking, "Show Deblocking Edges");
        });

        ui.separator();

        ui.columns(2, |cols| {
            // Left: Deblocking info
            cols[0].group(|ui| {
                ui.heading("Deblocking Filter");
                ui.add_space(4.0);

                if self.features.deblocking_enabled {
                    ui.label(RichText::new("Status: Enabled").color(Color32::GREEN));
                } else {
                    ui.label(RichText::new("Status: Disabled").color(Color32::GRAY));
                }

                ui.add_space(8.0);

                ui.label("Filter applied at:");
                ui.label("- MB boundaries (16-pixel edges)");
                ui.label("- 4x4 block boundaries");

                ui.add_space(8.0);

                ui.label("Boundary Strength (bS):");
                ui.label("  bS=4: Strong filtering (I-MB boundary)");
                ui.label("  bS=3: Medium (intra/inter boundary)");
                ui.label("  bS=2: Reference frame differs");
                ui.label("  bS=1: MV difference > 4");
                ui.label("  bS=0: No filtering");
            });

            // Right: Edge visualization
            cols[1].group(|ui| {
                ui.heading("Edge Visualization");

                if self.show_deblocking {
                    let (_, painter) =
                        ui.allocate_painter(Vec2::new(150.0, 150.0), egui::Sense::hover());

                    let rect = painter.clip_rect();

                    // Draw a 2x2 MB grid
                    let mb_size = rect.width() / 2.0;

                    painter.rect_filled(rect, 0.0, Color32::from_rgb(40, 40, 45));

                    // MB boundaries - strong (bS=4)
                    painter.line_segment(
                        [
                            egui::pos2(rect.min.x + mb_size, rect.min.y),
                            egui::pos2(rect.min.x + mb_size, rect.max.y),
                        ],
                        Stroke::new(3.0, Color32::RED),
                    );
                    painter.line_segment(
                        [
                            egui::pos2(rect.min.x, rect.min.y + mb_size),
                            egui::pos2(rect.max.x, rect.min.y + mb_size),
                        ],
                        Stroke::new(3.0, Color32::RED),
                    );

                    // Internal 4x4 boundaries - weaker
                    let block_size = mb_size / 4.0;
                    for i in 1..8 {
                        if i != 4 {
                            // Skip MB boundary
                            let pos = i as f32 * block_size;
                            painter.line_segment(
                                [
                                    egui::pos2(rect.min.x + pos, rect.min.y),
                                    egui::pos2(rect.min.x + pos, rect.max.y),
                                ],
                                Stroke::new(1.0, Color32::from_rgb(100, 200, 100)),
                            );
                            painter.line_segment(
                                [
                                    egui::pos2(rect.min.x, rect.min.y + pos),
                                    egui::pos2(rect.max.x, rect.min.y + pos),
                                ],
                                Stroke::new(1.0, Color32::from_rgb(100, 200, 100)),
                            );
                        }
                    }
                } else {
                    ui.label("Enable checkbox to visualize deblocking edges");
                }
            });
        });

        // Legend
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.label("Legend:");
            Self::legend_item(ui, Color32::RED, "MB Edge (bS=4)");
            Self::legend_item(ui, Color32::from_rgb(100, 200, 100), "4x4 Edge");
        });
    }

    /// Get profile name from profile_idc
    fn profile_name(profile_idc: u8) -> &'static str {
        match profile_idc {
            66 => "Baseline",
            77 => "Main",
            88 => "Extended",
            100 => "High",
            110 => "High 10",
            122 => "High 4:2:2",
            244 => "High 4:4:4 Pred",
            _ => "Unknown",
        }
    }

    /// Helper to draw feature badge
    fn feature_badge(ui: &mut egui::Ui, name: &str, enabled: bool) {
        ui.horizontal(|ui| {
            let (color, text) = if enabled {
                (Color32::from_rgb(50, 205, 50), "ON")
            } else {
                (Color32::from_rgb(128, 128, 128), "OFF")
            };

            let badge = RichText::new(format!(" {} ", text))
                .background_color(color)
                .color(Color32::WHITE)
                .small();

            ui.label(badge);
            ui.label(name);
        });
    }

    /// Helper to draw mode badge
    fn mode_badge(ui: &mut egui::Ui, color: Color32, label: &str) {
        let badge = RichText::new(format!(" {} ", label))
            .background_color(color)
            .color(Color32::WHITE)
            .small();
        ui.label(badge);
    }

    /// Helper to draw legend item
    fn legend_item(ui: &mut egui::Ui, color: Color32, label: &str) {
        ui.horizontal(|ui| {
            let (rect, _) = ui.allocate_exact_size(Vec2::splat(12.0), egui::Sense::hover());
            ui.painter().rect_filled(rect, 2.0, color);
            ui.label(label);
        });
    }
}

// =============================================================================
// STRATEGY PATTERN IMPLEMENTATION
// =============================================================================

/// AVC color scheme strategy
pub struct AvcWorkspaceColorScheme;

impl ColorScheme for AvcWorkspaceColorScheme {
    fn block_boundary(&self) -> Color32 {
        colors::SUB_MB_BOUNDARY
    }

    fn superblock_boundary(&self) -> Color32 {
        colors::MB_BOUNDARY
    }

    fn intra_prediction(&self) -> Color32 {
        colors::INTRA_16X16
    }

    fn inter_prediction(&self) -> Color32 {
        colors::INTER_P16X16
    }

    fn skip_mode(&self) -> Color32 {
        colors::INTER_SKIP
    }

    fn iframe(&self) -> Color32 {
        colors::SLICE_I
    }

    fn pframe(&self) -> Color32 {
        colors::SLICE_P
    }

    fn bframe(&self) -> Color32 {
        colors::SLICE_B
    }

    fn transform_boundary(&self) -> Option<Color32> {
        Some(colors::TRANSFORM_BOUNDARY)
    }

    fn deblocking_boundary(&self) -> Option<Color32> {
        Some(colors::MB_BOUNDARY.gamma_multiply(0.5))
    }
}

/// AVC partition renderer strategy
pub struct AvcPartitionRendererStrategy;

impl PartitionRenderer for AvcPartitionRendererStrategy {
    fn render_partitions(
        &self,
        frame_width: u32,
        frame_height: u32,
        _zoom: f32,
    ) -> Vec<PartitionData> {
        let mut partitions = Vec::new();
        let mb_size = 16; // H.264 uses 16x16 macroblocks
        let mb_count_x = (frame_width + mb_size - 1) / mb_size;
        let mb_count_y = (frame_height + mb_size - 1) / mb_size;

        for y in 0..mb_count_y {
            for x in 0..mb_count_x {
                partitions.push(PartitionData {
                    x: x * mb_size,
                    y: y * mb_size,
                    width: mb_size,
                    height: mb_size,
                    partition_type: "MB".to_string(),
                    reference_frame: Some("L0".to_string()),
                    prediction_mode: Some("P".to_string()),
                    color: colors::MB_BOUNDARY,
                    is_selected: false,
                });
            }
        }

        partitions
    }

    fn max_depth(&self) -> usize {
        3 // H.264 supports up to 3 levels of sub-partitioning
    }

    fn base_block_size(&self) -> u32 {
        16 // H.264 macroblock size
    }
}

/// View renderer for AVC Overview
pub struct AvcOverviewRenderer {
    pub features: AvcFeatureStatus,
    pub profile_idc: u8,
    pub level_idc: u8,
}

impl ViewRenderer for AvcOverviewRenderer {
    fn label(&self) -> &str {
        "Overview"
    }

    fn render(&self, ui: &mut egui::Ui, _ctx: &ViewContext) -> Option<ViewRenderResult> {
        ui.columns(2, |cols| {
            // Left: Features
            cols[0].group(|ui| {
                ui.heading("AVC Features");
                ui.add_space(8.0);

                ui.label(format!(
                    "Profile: {} ({})",
                    AvcWorkspace::profile_name(self.profile_idc),
                    self.profile_idc
                ));
                ui.label(format!(
                    "Level: {}.{}",
                    self.level_idc / 10,
                    self.level_idc % 10
                ));
                ui.add_space(8.0);

                AvcWorkspace::feature_badge(ui, "Transform 8x8", self.features.transform_8x8);
                AvcWorkspace::feature_badge(ui, "CABAC", self.features.cabac_enabled);
                AvcWorkspace::feature_badge(ui, "Deblocking", self.features.deblocking_enabled);
                AvcWorkspace::feature_badge(ui, "Weighted Pred", self.features.weighted_pred);
                AvcWorkspace::feature_badge(ui, "Weighted Bipred", self.features.weighted_bipred);
                AvcWorkspace::feature_badge(
                    "Direct 8x8",
                    self.features.direct_8x8_inference,
                );
                AvcWorkspace::feature_badge(ui, "MBAFF", self.features.mbaff);
                AvcWorkspace::feature_badge(ui, "FMO", self.features.fmo);
                AvcWorkspace::feature_badge(ui, "ASO", self.features.aso);
            });

            // Right: Entropy mode
            cols[1].group(|ui| {
                ui.heading("Entropy Encoding");
                ui.add_space(8.0);

                ui.label("CAVLC: Context-Adaptive Variable Length Coding");
                ui.label("CABAC: Context-Adaptive Binary Arithmetic Coding");
                ui.add_space(8.0);

                if self.features.cabac_enabled {
                    ui.label(RichText::new("CABAC Enabled").color(Color32::GREEN));
                } else {
                    ui.label(RichText::new("CAVLC Mode").color(Color32::GRAY));
                }

                ui.add_space(8.0);

                ui.group(|ui| {
                    ui.heading("Slice Types");
                    for slice_type in [
                        AvcSliceType::I,
                        AvcSliceType::P,
                        AvcSliceType::B,
                    ] {
                        AvcWorkspace::mode_badge(ui, slice_type.color(), slice_type.label());
                    }
                });
            });
        });

        None
    }
}

/// View renderer for AVC Partitions
pub struct AvcPartitionsRenderer {
    pub show_mb_grid: bool,
    pub show_sub_mb_grid: bool,
    pub show_transform: bool,
    pub show_refs: bool,
    pub mock_mbs: Vec<AvcMacroblock>,
    pub selected_mb: Option<usize>,
}

impl ViewRenderer for AvcPartitionsRenderer {
    fn label(&self) -> &str {
        "Partitions"
    }

    fn render(&self, ui: &mut egui::Ui, _ctx: &ViewContext) -> Option<ViewRenderResult> {
        // Toolbar
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("MB Grid")
                    .color(if self.show_mb_grid {
                        Color32::WHITE
                    } else {
                        Color32::GRAY
                    }),
            );
            ui.label(
                RichText::new("Sub-MB")
                    .color(if self.show_sub_mb_grid {
                        Color32::WHITE
                    } else {
                        Color32::GRAY
                    }),
            );
            ui.label(
                RichText::new("Transform")
                    .color(if self.show_transform {
                        Color32::WHITE
                    } else {
                        Color32::GRAY
                    }),
            );
        });

        ui.separator();

        // Partition visualization
        let available = ui.available_size();
        let grid_size = available.x.min(available.y - 120.0).min(400.0);

        let (response, painter) = ui.allocate_painter(
            Vec2::new(grid_size, grid_size),
            egui::Sense::click_and_drag(),
        );

        let rect = response.rect;

        // Background
        painter.rect_filled(rect, 4.0, Color32::from_rgb(30, 30, 35));

        // Scale: show up to 20x20 macroblocks (320x320 pixels)
        let mb_count = (self.mock_mbs.len() as f32).sqrt().ceil() as u32;
        let scale = grid_size / (mb_count * 16) as f32;

        // Draw macroblocks
        for (idx, mb) in self.mock_mbs.iter().enumerate() {
            let mb_rect = Rect::from_min_size(
                egui::pos2(
                    rect.min.x + mb.x as f32 * scale,
                    rect.min.y + mb.y as f32 * scale,
                ),
                Vec2::new(16.0 * scale, 16.0 * scale),
            );

            // Fill with MB type color
            painter.rect_filled(mb_rect, 0.0, mb.mb_type.color().gamma_multiply(0.4));

            // Draw MB boundary
            let stroke_width = if Some(idx) == self.selected_mb {
                2.5
            } else {
                1.0
            };
            painter.rect_stroke(
                mb_rect,
                0.0,
                Stroke::new(stroke_width, colors::MB_BOUNDARY),
            );

            // Label for MBs
            if mb.mb_type.is_intra() {
                painter.text(
                    mb_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    mb.mb_type.label(),
                    egui::FontId::proportional(9.0),
                    Color32::WHITE,
                );
            } else if mb.mb_type.is_inter() {
                painter.text(
                    mb_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    mb.mb_type.label(),
                    egui::FontId::proportional(8.0),
                    Color32::from_rgb(200, 255, 200),
                );
            }

            // Draw transform boundary if enabled and 8x8 transform
            if self.show_transform && mb.transform_8x8 {
                let mid_x = mb_rect.center().x;
                let mid_y = mb_rect.center().y;

                painter.line_segment(
                    [
                        egui::pos2(mb_rect.min.x, mid_y),
                        egui::pos2(mb_rect.max.x, mid_y),
                    ],
                    Stroke::new(0.5, colors::TRANSFORM_BOUNDARY.gamma_multiply(0.5)),
                );
                painter.line_segment(
                    [
                        egui::pos2(mid_x, mb_rect.min.y),
                        egui::pos2(mid_x, mb_rect.max.y),
                    ],
                    Stroke::new(0.5, colors::TRANSFORM_BOUNDARY.gamma_multiply(0.5)),
                );
            }
        }

        None
    }
}

/// View renderer for AVC Predictions
pub struct AvcPredictionsRenderer;

impl ViewRenderer for AvcPredictionsRenderer {
    fn label(&self) -> &str {
        "Predictions"
    }

    fn render(&self, ui: &mut egui::Ui, _ctx: &ViewContext) -> Option<ViewRenderResult> {
        ui.group(|ui| {
            ui.heading("H.264 Prediction Modes");
            ui.add_space(8.0);

            ui.columns(2, |cols| {
                cols[0].group(|ui| {
                    ui.heading("Intra Prediction");
                    ui.label("I_4x4: 9 modes (vertical, horizontal, DC, plane)");
                    ui.label("I_16x16: 4 modes (vertical, horizontal, DC, plane)");
                    ui.label("I_PCM: Raw sample encoding");
                });

                cols[1].group(|ui| {
                    ui.heading("Inter Prediction");
                    ui.label("P_Skip: No motion vectors, copy from ref");
                    ui.label("P_L0/L1: Direct mode");
                    ui.label("B_Skip: Direct mode, no MV data");
                    ui.label("B_Direct: L0 and L1 refs same");
                });
            });

            ui.add_space(8.0);

            // Prediction mode legend
            ui.group(|ui| {
                ui.heading("P Slice Partition Sizes");
                ui.horizontal_wrapped(|ui| {
                    AvcWorkspace::mode_badge(ui, colors::INTER_P16X16, "16x16");
                    AvcWorkspace::mode_badge(ui, colors::INTER_P16X8, "16x8");
                    AvcWorkspace::mode_badge(ui, colors::INTER_P8X16, "8x16");
                    AvcWorkspace::mode_badge(ui, colors::INTER_P8X8, "8x8");
                    AvcWorkspace::mode_badge(ui, colors::INTER_P8X4, "8x4");
                    AvcWorkspace::mode_badge(ui, colors::INTER_P4X8, "4x8");
                    AvcWorkspace::mode_badge(ui, colors::INTER_P4X4, "4x4");
                });
            });
        });

        None
    }
}

/// View renderer for AVC Transform
pub struct AvcTransformRenderer;

impl ViewRenderer for AvcTransformRenderer {
    fn label(&self) -> &str {
        "Transform"
    }

    fn render(&self, ui: &mut egui::Ui, _ctx: &ViewContext) -> Option<ViewRenderResult> {
        ui.group(|ui| {
            ui.heading("Transform Size Selection");
            ui.add_space(8.0);

            ui.label("H.264 supports two transform sizes:");
            ui.add_space(4.0);

            ui.group(|ui| {
                ui.horizontal(|ui| {
                    AvcWorkspace::mode_badge(ui, Color32::from_rgb(100, 100, 200), "4x4 DCT");
                });
                ui.label("Used for residual coding in most blocks");
            });

            ui.add_space(4.0);

            ui.group(|ui| {
                ui.horizontal(|ui| {
                    AvcWorkspace::mode_badge(ui, Color32::from_rgb(100, 200, 100), "8x8 DCT");
                });
                ui.label("High profile feature, optional for some blocks");
            });

            ui.add_space(8.0);

            ui.label("Transform choice is signaled per block and depends on:");
            ui.label("• Prediction mode (intra uses 4x4, inter can use 8x8)");
            ui.label("• Residual energy (8x8 for more uniform residuals)");
            ui.label("• Picture profile (High profile enables 8x8)");
        });

        None
    }
}

/// View renderer for AVC Deblocking
pub struct AvcDeblockingRenderer;

impl ViewRenderer for AvcDeblockingRenderer {
    fn label(&self) -> &str {
        "Deblocking"
    }

    fn render(&self, ui: &mut egui::Ui, _ctx: &ViewContext) -> Option<ViewRenderResult> {
        ui.group(|ui| {
            ui.heading("Deblocking Filter");
            ui.add_space(8.0);

            ui.label("H.264 includes an in-loop deblocking filter that reduces");
            ui.label("blocking artifacts at block boundaries.");

            ui.add_space(8.0);

            ui.columns(2, |cols| {
                cols[0].group(|ui| {
                    ui.heading("Filter Parameters");
                    ui.label("Strength: 0-2 (derived from QP)");
                    ui.label("Boundary: 4x4 and 8x8 edges");
                    ui.label("Beta offset: -6 to 6");
                    ui.label("Alpha C0: Chroma default offset");
                    ui.label("Alpha C1: Chroma offset slope");
                });

                cols[1].group(|ui| {
                    ui.heading("Deblocking Process");
                    ui.label("1. Filter vertical edges");
                    ui.label("2. Filter horizontal edges");
                    ui.label("3. Chroma filtering");
                    ui.label("Strength based on slice QP");
                });
            });

            ui.add_space(8.0);

            ui.group(|ui| {
                ui.heading("Boundary Strength");
                ui.label("Strong: QP > 36 (bslice threshold)");
                ui.label("Weak:   QP ≤ 36");
            });
        });

        None
    }
}
