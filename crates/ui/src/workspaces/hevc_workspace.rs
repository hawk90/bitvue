//! HEVC (H.265) Visualization Workspace
//!
//! VQAnalyzer parity: Complete HEVC-specific analysis views
//! - CTU partition visualization (quad-tree only)
//! - Intra prediction modes (35 modes)
//! - Inter prediction (AMVP, Merge, Skip)
//! - Transform/residual visualization
//! - SAO (Sample Adaptive Offset) visualization
//! - Deblocking filter edge display

use egui::{self, Color32, Rect, RichText, Stroke, Vec2};

/// HEVC-specific color palette
mod colors {
    use egui::Color32;

    // Partition colors
    pub const CTU_BOUNDARY: Color32 = Color32::from_rgb(255, 128, 0); // Orange
    pub const CU_BOUNDARY: Color32 = Color32::from_rgb(100, 149, 237); // Cornflower blue
    pub const TU_BOUNDARY: Color32 = Color32::from_rgb(144, 238, 144); // Light green
    #[allow(dead_code)]
    pub const _PU_BOUNDARY: Color32 = Color32::from_rgb(255, 182, 193); // Light pink

    // Prediction mode colors
    pub const INTRA_PLANAR: Color32 = Color32::from_rgb(255, 215, 0); // Gold
    pub const INTRA_DC: Color32 = Color32::from_rgb(255, 165, 0); // Orange
    pub const INTRA_ANGULAR: Color32 = Color32::from_rgb(147, 112, 219); // Medium purple
    pub const INTER_SKIP: Color32 = Color32::from_rgb(50, 205, 50); // Lime green
    pub const INTER_MERGE: Color32 = Color32::from_rgb(30, 144, 255); // Dodger blue
    pub const INTER_AMVP: Color32 = Color32::from_rgb(220, 20, 60); // Crimson

    // SAO colors
    pub const SAO_OFF: Color32 = Color32::from_rgb(128, 128, 128); // Gray
    pub const SAO_BAND: Color32 = Color32::from_rgb(255, 99, 71); // Tomato
    pub const SAO_EDGE: Color32 = Color32::from_rgb(64, 224, 208); // Turquoise
}

/// HEVC view modes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HevcView {
    #[default]
    Overview,
    CodingFlow,
    Predictions,
    Transform,
    Sao,
}

impl HevcView {
    fn label(&self) -> &'static str {
        match self {
            HevcView::Overview => "Overview",
            HevcView::CodingFlow => "Coding Flow",
            HevcView::Predictions => "Predictions",
            HevcView::Transform => "Transform",
            HevcView::Sao => "SAO",
        }
    }
}

/// SAO type for a CTB
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SaoType {
    #[default]
    NotApplied,
    BandOffset,
    EdgeOffset,
}

impl SaoType {
    fn color(&self) -> Color32 {
        match self {
            SaoType::NotApplied => colors::SAO_OFF,
            SaoType::BandOffset => colors::SAO_BAND,
            SaoType::EdgeOffset => colors::SAO_EDGE,
        }
    }

    fn label(&self) -> &'static str {
        match self {
            SaoType::NotApplied => "Off",
            SaoType::BandOffset => "Band",
            SaoType::EdgeOffset => "Edge",
        }
    }
}

/// HEVC prediction mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HevcPredMode {
    #[default]
    IntraPlanar,
    IntraDC,
    IntraAngular(u8), // 2-34
    InterSkip,
    InterMerge,
    InterAMVP,
}

impl HevcPredMode {
    fn color(&self) -> Color32 {
        match self {
            HevcPredMode::IntraPlanar => colors::INTRA_PLANAR,
            HevcPredMode::IntraDC => colors::INTRA_DC,
            HevcPredMode::IntraAngular(_) => colors::INTRA_ANGULAR,
            HevcPredMode::InterSkip => colors::INTER_SKIP,
            HevcPredMode::InterMerge => colors::INTER_MERGE,
            HevcPredMode::InterAMVP => colors::INTER_AMVP,
        }
    }

    fn label(&self) -> String {
        match self {
            HevcPredMode::IntraPlanar => "Planar".to_string(),
            HevcPredMode::IntraDC => "DC".to_string(),
            HevcPredMode::IntraAngular(idx) => format!("Ang{}", idx),
            HevcPredMode::InterSkip => "Skip".to_string(),
            HevcPredMode::InterMerge => "Merge".to_string(),
            HevcPredMode::InterAMVP => "AMVP".to_string(),
        }
    }
}

/// HEVC CU partition node (quad-tree only)
#[derive(Debug, Clone)]
pub struct HevcCuNode {
    /// Position in CTU (x, y)
    pub x: u32,
    pub y: u32,
    /// Size (always square in HEVC)
    pub size: u32,
    /// Depth in quad-tree (0 = CTU, max = 3 for 8x8)
    pub depth: u8,
    /// Prediction mode
    pub pred_mode: HevcPredMode,
    /// Is split into 4 sub-CUs?
    pub is_split: bool,
}

/// SAO parameters for a CTB
#[derive(Debug, Clone)]
pub struct SaoParams {
    pub sao_type_luma: SaoType,
    pub sao_type_chroma: SaoType,
    pub sao_offset: [i8; 4],
    pub sao_band_position: u8,
    pub sao_eo_class: u8, // 0-3 for edge offset directions
}

impl Default for SaoParams {
    fn default() -> Self {
        Self {
            sao_type_luma: SaoType::NotApplied,
            sao_type_chroma: SaoType::NotApplied,
            sao_offset: [0; 4],
            sao_band_position: 0,
            sao_eo_class: 0,
        }
    }
}

/// HEVC-specific feature flags
#[derive(Debug, Clone, Default)]
pub struct HevcFeatureStatus {
    pub sao_enabled: bool,
    pub deblocking_enabled: bool,
    pub amp_enabled: bool, // Asymmetric Motion Partitions
    pub pcm_enabled: bool, // PCM mode
    pub transform_skip: bool,
    pub sign_hiding: bool,
    pub strong_intra_smoothing: bool,
    pub tiles_enabled: bool,
    pub wpp_enabled: bool, // Wavefront Parallel Processing
}

/// HEVC Visualization Workspace
pub struct HevcWorkspace {
    /// Active view
    active_view: HevcView,

    /// Show CTU boundaries
    show_ctu_grid: bool,

    /// Show CU boundaries
    show_cu_grid: bool,

    /// Show TU boundaries
    show_tu_grid: bool,

    /// Show prediction modes
    show_pred_modes: bool,

    /// Show SAO regions
    show_sao: bool,

    /// Current CTU size (default 64x64)
    ctu_size: u32,

    /// Feature status
    features: HevcFeatureStatus,

    /// Mock CU data for visualization
    mock_cus: Vec<HevcCuNode>,

    /// Mock SAO data per CTB
    mock_sao: Vec<SaoParams>,

    /// Selected CU index
    selected_cu: Option<usize>,

    /// Frame dimensions
    frame_width: u32,
    frame_height: u32,

    /// Flag to track if mock data has been initialized
    mock_data_initialized: bool,
}

impl Default for HevcWorkspace {
    fn default() -> Self {
        Self::new()
    }
}

impl HevcWorkspace {
    pub fn new() -> Self {
        Self {
            active_view: HevcView::default(),
            show_ctu_grid: true,
            show_cu_grid: true,
            show_tu_grid: false,
            show_pred_modes: true,
            show_sao: false,
            ctu_size: 64,
            features: HevcFeatureStatus::default(),
            mock_cus: Vec::new(),
            mock_sao: Vec::new(),
            selected_cu: None,
            frame_width: 1920,
            frame_height: 1080,
            mock_data_initialized: false,
        }
    }

    /// Set view mode by index (F1-F5 keyboard shortcuts)
    pub fn set_mode_by_index(&mut self, index: usize) {
        self.active_view = match index {
            0 => HevcView::Overview,
            1 => HevcView::CodingFlow,
            2 => HevcView::Predictions,
            3 => HevcView::Transform,
            4 => HevcView::Sao,
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

    /// Generate mock CU quad-tree data for demonstration
    fn generate_mock_data(&mut self) {
        self.mock_cus.clear();
        self.mock_sao.clear();

        // Generate mock quad-tree partitions for 4 CTUs
        let modes = [
            HevcPredMode::IntraPlanar,
            HevcPredMode::IntraDC,
            HevcPredMode::IntraAngular(10),
            HevcPredMode::IntraAngular(26),
            HevcPredMode::InterSkip,
            HevcPredMode::InterMerge,
            HevcPredMode::InterAMVP,
        ];

        let ctu_size = self.ctu_size;
        let mut idx = 0usize;

        // CTU at (0,0) - split to depth 2
        self.add_split_ctu(0, 0, ctu_size, &modes, &mut idx);

        // CTU at (64,0) - split to depth 1
        self.mock_cus.push(HevcCuNode {
            x: 64,
            y: 0,
            size: 32,
            depth: 1,
            pred_mode: modes[4],
            is_split: false,
        });
        self.mock_cus.push(HevcCuNode {
            x: 96,
            y: 0,
            size: 32,
            depth: 1,
            pred_mode: modes[5],
            is_split: false,
        });
        self.mock_cus.push(HevcCuNode {
            x: 64,
            y: 32,
            size: 32,
            depth: 1,
            pred_mode: modes[6],
            is_split: false,
        });
        self.mock_cus.push(HevcCuNode {
            x: 96,
            y: 32,
            size: 32,
            depth: 1,
            pred_mode: modes[4],
            is_split: false,
        });

        // CTU at (0,64) - no split (single 64x64)
        self.mock_cus.push(HevcCuNode {
            x: 0,
            y: 64,
            size: 64,
            depth: 0,
            pred_mode: modes[4], // Skip
            is_split: false,
        });

        // CTU at (64,64) - mixed splits
        self.mock_cus.push(HevcCuNode {
            x: 64,
            y: 64,
            size: 32,
            depth: 1,
            pred_mode: modes[0],
            is_split: false,
        });
        self.add_split_16x16(96, 64, &modes);
        self.mock_cus.push(HevcCuNode {
            x: 64,
            y: 96,
            size: 32,
            depth: 1,
            pred_mode: modes[5],
            is_split: false,
        });
        self.mock_cus.push(HevcCuNode {
            x: 96,
            y: 96,
            size: 32,
            depth: 1,
            pred_mode: modes[6],
            is_split: false,
        });

        // Generate SAO data for 4 CTBs
        self.mock_sao.push(SaoParams {
            sao_type_luma: SaoType::EdgeOffset,
            sao_type_chroma: SaoType::BandOffset,
            sao_offset: [2, -1, 1, -2],
            sao_band_position: 12,
            sao_eo_class: 1,
        });
        self.mock_sao.push(SaoParams {
            sao_type_luma: SaoType::BandOffset,
            sao_type_chroma: SaoType::NotApplied,
            sao_offset: [1, 2, -1, 0],
            sao_band_position: 8,
            sao_eo_class: 0,
        });
        self.mock_sao.push(SaoParams::default());
        self.mock_sao.push(SaoParams {
            sao_type_luma: SaoType::EdgeOffset,
            sao_type_chroma: SaoType::EdgeOffset,
            sao_offset: [-1, 1, 2, -2],
            sao_band_position: 0,
            sao_eo_class: 2,
        });

        // Set some features as enabled
        self.features = HevcFeatureStatus {
            sao_enabled: true,
            deblocking_enabled: true,
            amp_enabled: true,
            pcm_enabled: false,
            transform_skip: true,
            sign_hiding: true,
            strong_intra_smoothing: true,
            tiles_enabled: false,
            wpp_enabled: false,
        };
    }

    fn add_split_ctu(
        &mut self,
        x: u32,
        y: u32,
        size: u32,
        modes: &[HevcPredMode],
        idx: &mut usize,
    ) {
        // Split into 4 32x32
        let half = size / 2;
        for dy in 0..2 {
            for dx in 0..2 {
                let cx = x + dx * half;
                let cy = y + dy * half;
                if *idx % 3 == 0 {
                    // Further split this one
                    self.add_split_16x16(cx, cy, modes);
                } else {
                    self.mock_cus.push(HevcCuNode {
                        x: cx,
                        y: cy,
                        size: half,
                        depth: 1,
                        pred_mode: modes[*idx % modes.len()],
                        is_split: false,
                    });
                }
                *idx += 1;
            }
        }
    }

    fn add_split_16x16(&mut self, x: u32, y: u32, modes: &[HevcPredMode]) {
        // Split 32x32 into 4 16x16
        for dy in 0..2 {
            for dx in 0..2 {
                let cx = x + dx * 16;
                let cy = y + dy * 16;
                let mode_idx = (cx / 16 + cy / 16) as usize % modes.len();
                self.mock_cus.push(HevcCuNode {
                    x: cx,
                    y: cy,
                    size: 16,
                    depth: 2,
                    pred_mode: modes[mode_idx],
                    is_split: false,
                });
            }
        }
    }

    /// Main UI entry point
    pub fn show(&mut self, ui: &mut egui::Ui) {
        // Lazy-load mock data on first show
        self.ensure_mock_data();

        ui.heading("HEVC (H.265) Analysis");
        ui.separator();

        // View selector tabs
        ui.horizontal(|ui| {
            for view in [
                HevcView::Overview,
                HevcView::CodingFlow,
                HevcView::Predictions,
                HevcView::Transform,
                HevcView::Sao,
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
            HevcView::Overview => self.show_overview(ui),
            HevcView::CodingFlow => self.show_coding_flow(ui),
            HevcView::Predictions => self.show_predictions(ui),
            HevcView::Transform => self.show_transform(ui),
            HevcView::Sao => self.show_sao(ui),
        }
    }

    /// Overview tab
    fn show_overview(&mut self, ui: &mut egui::Ui) {
        ui.columns(2, |cols| {
            // Left: Feature status
            cols[0].group(|ui| {
                ui.heading("HEVC Features");
                ui.add_space(8.0);

                Self::feature_badge(ui, "SAO", self.features.sao_enabled);
                Self::feature_badge(ui, "Deblocking", self.features.deblocking_enabled);
                Self::feature_badge(ui, "AMP", self.features.amp_enabled);
                Self::feature_badge(ui, "PCM", self.features.pcm_enabled);
                Self::feature_badge(ui, "Transform Skip", self.features.transform_skip);
                Self::feature_badge(ui, "Sign Hiding", self.features.sign_hiding);
                Self::feature_badge(
                    ui,
                    "Strong Intra Smooth",
                    self.features.strong_intra_smoothing,
                );
                Self::feature_badge(ui, "Tiles", self.features.tiles_enabled);
                Self::feature_badge(ui, "WPP", self.features.wpp_enabled);
            });

            // Right: Quick stats
            cols[1].group(|ui| {
                ui.heading("Frame Statistics");
                ui.add_space(8.0);

                ui.label(format!(
                    "Frame size: {}x{}",
                    self.frame_width, self.frame_height
                ));
                ui.label(format!("CTU size: {}x{}", self.ctu_size, self.ctu_size));
                ui.label(format!("Total CUs: {}", self.mock_cus.len()));

                ui.add_space(8.0);

                // Mode distribution
                let mut intra_count = 0;
                let mut inter_count = 0;
                let mut skip_count = 0;

                for cu in &self.mock_cus {
                    match cu.pred_mode {
                        HevcPredMode::IntraPlanar
                        | HevcPredMode::IntraDC
                        | HevcPredMode::IntraAngular(_) => intra_count += 1,
                        HevcPredMode::InterSkip => skip_count += 1,
                        HevcPredMode::InterMerge | HevcPredMode::InterAMVP => inter_count += 1,
                    }
                }

                ui.label(format!("Intra CUs: {}", intra_count));
                ui.label(format!("Inter CUs: {}", inter_count));
                ui.label(format!("Skip CUs: {}", skip_count));
            });
        });
    }

    /// Coding Flow tab - shows CTU/CU partitions
    fn show_coding_flow(&mut self, ui: &mut egui::Ui) {
        // Toolbar
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.show_ctu_grid, "CTU Grid");
            ui.checkbox(&mut self.show_cu_grid, "CU Grid");
            ui.checkbox(&mut self.show_tu_grid, "TU Grid");
            ui.checkbox(&mut self.show_pred_modes, "Pred Modes");
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

        // Scale factor: we'll show 2x2 CTUs (128x128 logical -> grid_size pixels)
        let display_size = 128.0;
        let scale = grid_size / display_size;

        // Draw CTU boundaries if enabled
        if self.show_ctu_grid {
            for x in (0..=128).step_by(64) {
                let px = rect.min.x + x as f32 * scale;
                painter.line_segment(
                    [egui::pos2(px, rect.min.y), egui::pos2(px, rect.max.y)],
                    Stroke::new(2.0, colors::CTU_BOUNDARY),
                );
            }
            for y in (0..=128).step_by(64) {
                let py = rect.min.y + y as f32 * scale;
                painter.line_segment(
                    [egui::pos2(rect.min.x, py), egui::pos2(rect.max.x, py)],
                    Stroke::new(2.0, colors::CTU_BOUNDARY),
                );
            }
        }

        // Draw CUs
        for (idx, cu) in self.mock_cus.iter().enumerate() {
            let cu_rect = Rect::from_min_size(
                egui::pos2(
                    rect.min.x + cu.x as f32 * scale,
                    rect.min.y + cu.y as f32 * scale,
                ),
                Vec2::splat(cu.size as f32 * scale),
            );

            // Fill with prediction mode color
            if self.show_pred_modes {
                let color = cu.pred_mode.color().gamma_multiply(0.4);
                painter.rect_filled(cu_rect, 0.0, color);
            }

            // Draw CU boundary
            if self.show_cu_grid {
                let stroke_width = if Some(idx) == self.selected_cu {
                    2.0
                } else {
                    1.0
                };
                painter.rect_stroke(cu_rect, 0.0, Stroke::new(stroke_width, colors::CU_BOUNDARY));
            }

            // Draw mode label if space allows
            if self.show_pred_modes && cu.size >= 16 {
                painter.text(
                    cu_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    cu.pred_mode.label(),
                    egui::FontId::proportional(9.0),
                    Color32::WHITE,
                );
            }
        }

        // Legend
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.label("Legend:");
            Self::legend_item(ui, colors::CTU_BOUNDARY, "CTU");
            Self::legend_item(ui, colors::CU_BOUNDARY, "CU");
            Self::legend_item(ui, colors::TU_BOUNDARY, "TU");
        });
    }

    /// Predictions tab
    fn show_predictions(&mut self, ui: &mut egui::Ui) {
        ui.columns(2, |cols| {
            // Left: Intra modes
            cols[0].group(|ui| {
                ui.heading("Intra Prediction (35 modes)");
                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    Self::mode_badge(ui, colors::INTRA_PLANAR, "Planar (0)");
                });
                ui.horizontal(|ui| {
                    Self::mode_badge(ui, colors::INTRA_DC, "DC (1)");
                });
                ui.horizontal(|ui| {
                    Self::mode_badge(ui, colors::INTRA_ANGULAR, "Angular (2-34)");
                });

                ui.add_space(8.0);

                // Angular mode diagram (simplified)
                let (_, painter) = ui.allocate_painter(Vec2::splat(150.0), egui::Sense::hover());

                let center = painter.clip_rect().center();
                let radius = 60.0;

                // Draw direction arrows for some angular modes
                for angle_idx in [2, 10, 18, 26, 34] {
                    let angle_rad = std::f32::consts::PI * (angle_idx as f32 - 2.0) / 32.0
                        - std::f32::consts::FRAC_PI_2;
                    let end = center + Vec2::new(angle_rad.cos(), angle_rad.sin()) * radius;
                    painter.arrow(
                        center,
                        end - center,
                        Stroke::new(1.5, colors::INTRA_ANGULAR),
                    );
                    painter.text(
                        end + Vec2::new(angle_rad.cos(), angle_rad.sin()) * 10.0,
                        egui::Align2::CENTER_CENTER,
                        format!("{}", angle_idx),
                        egui::FontId::proportional(9.0),
                        Color32::WHITE,
                    );
                }
            });

            // Right: Inter modes
            cols[1].group(|ui| {
                ui.heading("Inter Prediction");
                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    Self::mode_badge(ui, colors::INTER_SKIP, "Skip");
                    ui.label("- No residual, merge MV");
                });
                ui.horizontal(|ui| {
                    Self::mode_badge(ui, colors::INTER_MERGE, "Merge");
                    ui.label("- Inherit MV from neighbor");
                });
                ui.horizontal(|ui| {
                    Self::mode_badge(ui, colors::INTER_AMVP, "AMVP");
                    ui.label("- Explicit MV prediction");
                });

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(4.0);

                ui.heading("PU Partitions");
                ui.label("2Nx2N, 2NxN, Nx2N");
                if self.features.amp_enabled {
                    ui.label("AMP: 2NxnU, 2NxnD, nLx2N, nRx2N");
                }
                ui.label("NxN (Intra only for smallest CU)");
            });
        });
    }

    /// Transform tab
    fn show_transform(&mut self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.heading("Transform Units (TU)");
            ui.add_space(4.0);

            ui.label("TU sizes: 4x4, 8x8, 16x16, 32x32");
            ui.label("Residual Quad-Tree (RQT) inside CU");
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                ui.label("Transform Skip:");
                if self.features.transform_skip {
                    ui.label(RichText::new("Enabled").color(Color32::GREEN));
                } else {
                    ui.label(RichText::new("Disabled").color(Color32::GRAY));
                }
            });

            ui.horizontal(|ui| {
                ui.label("Sign Hiding:");
                if self.features.sign_hiding {
                    ui.label(RichText::new("Enabled").color(Color32::GREEN));
                } else {
                    ui.label(RichText::new("Disabled").color(Color32::GRAY));
                }
            });
        });

        ui.add_space(8.0);

        // TU tree visualization
        ui.group(|ui| {
            ui.heading("TU Depth Visualization");

            let (_, painter) = ui.allocate_painter(Vec2::new(200.0, 200.0), egui::Sense::hover());

            let rect = painter.clip_rect();

            // Draw a sample TU quad-tree (32x32 CU with varied TU depths)
            // Depth 0: 32x32
            painter.rect_stroke(rect, 0.0, Stroke::new(2.0, colors::TU_BOUNDARY));

            // Split into 4 16x16
            let half = rect.width() / 2.0;
            for dy in 0..2 {
                for dx in 0..2 {
                    let tu_rect = Rect::from_min_size(
                        rect.min + Vec2::new(dx as f32 * half, dy as f32 * half),
                        Vec2::splat(half),
                    );
                    painter.rect_stroke(tu_rect, 0.0, Stroke::new(1.0, colors::TU_BOUNDARY));

                    // Further split one quadrant
                    if dx == 1 && dy == 1 {
                        let quarter = half / 2.0;
                        for sy in 0..2 {
                            for sx in 0..2 {
                                let sub_rect = Rect::from_min_size(
                                    tu_rect.min
                                        + Vec2::new(sx as f32 * quarter, sy as f32 * quarter),
                                    Vec2::splat(quarter),
                                );
                                painter.rect_stroke(
                                    sub_rect,
                                    0.0,
                                    Stroke::new(0.5, colors::TU_BOUNDARY),
                                );
                            }
                        }
                    }
                }
            }

            // Labels
            painter.text(
                rect.min + Vec2::new(5.0, 5.0),
                egui::Align2::LEFT_TOP,
                "32x32 CU",
                egui::FontId::proportional(10.0),
                Color32::WHITE,
            );
        });
    }

    /// SAO tab
    fn show_sao(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.show_sao, "Show SAO regions");
        });

        ui.separator();

        ui.columns(2, |cols| {
            // Left: SAO types explanation
            cols[0].group(|ui| {
                ui.heading("SAO Types");
                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    Self::legend_item(ui, colors::SAO_OFF, "Off");
                    ui.label("- No SAO applied");
                });
                ui.horizontal(|ui| {
                    Self::legend_item(ui, colors::SAO_BAND, "Band Offset");
                    ui.label("- Amplitude bands");
                });
                ui.horizontal(|ui| {
                    Self::legend_item(ui, colors::SAO_EDGE, "Edge Offset");
                    ui.label("- Edge classes");
                });

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(4.0);

                ui.heading("Edge Offset Classes");
                ui.label("Class 0: Horizontal");
                ui.label("Class 1: Vertical");
                ui.label("Class 2: 135 diagonal");
                ui.label("Class 3: 45 diagonal");
            });

            // Right: SAO visualization
            cols[1].group(|ui| {
                ui.heading("CTB SAO Map");

                if self.show_sao {
                    let (_, painter) =
                        ui.allocate_painter(Vec2::new(200.0, 200.0), egui::Sense::hover());

                    let rect = painter.clip_rect();
                    let cell_size = rect.width() / 2.0;

                    for (idx, sao) in self.mock_sao.iter().enumerate() {
                        let row = idx / 2;
                        let col = idx % 2;
                        let cell_rect = Rect::from_min_size(
                            rect.min + Vec2::new(col as f32 * cell_size, row as f32 * cell_size),
                            Vec2::splat(cell_size),
                        );

                        // Fill with SAO type color
                        let color = sao.sao_type_luma.color().gamma_multiply(0.5);
                        painter.rect_filled(cell_rect, 0.0, color);
                        painter.rect_stroke(cell_rect, 0.0, Stroke::new(1.0, Color32::WHITE));

                        // Label
                        painter.text(
                            cell_rect.center(),
                            egui::Align2::CENTER_CENTER,
                            sao.sao_type_luma.label(),
                            egui::FontId::proportional(12.0),
                            Color32::WHITE,
                        );
                    }
                } else {
                    ui.label("Enable 'Show SAO regions' to visualize");
                }
            });
        });

        // SAO parameters for selected CTB
        if self.show_sao && !self.mock_sao.is_empty() {
            ui.add_space(8.0);
            ui.group(|ui| {
                ui.heading("CTB 0 SAO Parameters");
                let sao = &self.mock_sao[0];

                ui.label(format!("Luma SAO: {:?}", sao.sao_type_luma));
                ui.label(format!("Chroma SAO: {:?}", sao.sao_type_chroma));
                ui.label(format!(
                    "Offsets: [{}, {}, {}, {}]",
                    sao.sao_offset[0], sao.sao_offset[1], sao.sao_offset[2], sao.sao_offset[3]
                ));
                if matches!(sao.sao_type_luma, SaoType::BandOffset) {
                    ui.label(format!("Band position: {}", sao.sao_band_position));
                }
                if matches!(sao.sao_type_luma, SaoType::EdgeOffset) {
                    ui.label(format!("EO class: {}", sao.sao_eo_class));
                }
            });
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
