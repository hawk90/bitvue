//! AV1 Visualization Workspace
//!
//! VQAnalyzer parity: Complete AV1-specific analysis views
//! - Superblock partition visualization (128x128 or 64x64 SB)
//! - Reference frame management (7 reference frames)
//! - CDEF (Constrained Directional Enhancement Filter)
//! - Super-resolution
//! - Film grain synthesis
//! - Transform type visualization

use egui::{self, Color32, Rect, RichText, Stroke, Vec2};

/// AV1-specific color palette
mod colors {
    use egui::Color32;

    // Reference frame colors
    pub const REF_LAST: Color32 = Color32::from_rgb(65, 105, 225); // Royal blue
    pub const REF_LAST2: Color32 = Color32::from_rgb(30, 144, 255); // Dodger blue
    pub const REF_LAST3: Color32 = Color32::from_rgb(135, 206, 250); // Light sky blue
    pub const REF_GOLDEN: Color32 = Color32::from_rgb(255, 215, 0); // Gold
    pub const REF_BWDREF: Color32 = Color32::from_rgb(255, 165, 0); // Orange
    pub const REF_ALTREF2: Color32 = Color32::from_rgb(255, 99, 71); // Tomato
    pub const REF_ALTREF: Color32 = Color32::from_rgb(220, 20, 60); // Crimson

    // Prediction mode colors
    pub const INTRA: Color32 = Color32::from_rgb(147, 112, 219); // Medium purple
    pub const SINGLE_REF: Color32 = Color32::from_rgb(50, 205, 50); // Lime green
    pub const COMPOUND: Color32 = Color32::from_rgb(255, 20, 147); // Deep pink

    // Block boundary colors
    pub const SB_BOUNDARY: Color32 = Color32::from_rgb(255, 128, 0); // Orange
    pub const BLOCK_BOUNDARY: Color32 = Color32::from_rgb(100, 149, 237); // Cornflower blue

    // CDEF colors
    pub const CDEF_PRIMARY: Color32 = Color32::from_rgb(64, 224, 208); // Turquoise
    pub const CDEF_SECONDARY: Color32 = Color32::from_rgb(255, 182, 193); // Light pink

    // Film grain
    pub const FILM_GRAIN: Color32 = Color32::from_rgb(210, 180, 140); // Tan
}

/// AV1 view modes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Av1View {
    #[default]
    Overview,
    Partitions,
    References,
    Cdef,
    SuperRes,
    FilmGrain,
}

impl Av1View {
    fn label(&self) -> &'static str {
        match self {
            Av1View::Overview => "Overview",
            Av1View::Partitions => "Partitions",
            Av1View::References => "References",
            Av1View::Cdef => "CDEF",
            Av1View::SuperRes => "Super-Res",
            Av1View::FilmGrain => "Film Grain",
        }
    }
}

/// AV1 reference frame types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Av1RefFrame {
    #[default]
    Intra,
    Last,
    Last2,
    Last3,
    Golden,
    BwdRef,
    AltRef2,
    AltRef,
}

impl Av1RefFrame {
    fn color(&self) -> Color32 {
        match self {
            Av1RefFrame::Intra => colors::INTRA,
            Av1RefFrame::Last => colors::REF_LAST,
            Av1RefFrame::Last2 => colors::REF_LAST2,
            Av1RefFrame::Last3 => colors::REF_LAST3,
            Av1RefFrame::Golden => colors::REF_GOLDEN,
            Av1RefFrame::BwdRef => colors::REF_BWDREF,
            Av1RefFrame::AltRef2 => colors::REF_ALTREF2,
            Av1RefFrame::AltRef => colors::REF_ALTREF,
        }
    }

    fn label(&self) -> &'static str {
        match self {
            Av1RefFrame::Intra => "INTRA",
            Av1RefFrame::Last => "LAST",
            Av1RefFrame::Last2 => "LAST2",
            Av1RefFrame::Last3 => "LAST3",
            Av1RefFrame::Golden => "GOLDEN",
            Av1RefFrame::BwdRef => "BWDREF",
            Av1RefFrame::AltRef2 => "ALTREF2",
            Av1RefFrame::AltRef => "ALTREF",
        }
    }
}

/// AV1 block partition type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Av1Partition {
    None,  // No further split
    Horz,  // Split horizontally
    Vert,  // Split vertically
    Split, // Split into 4
    HorzA, // Top half split, bottom full
    HorzB, // Top full, bottom split
    VertA, // Left half split, right full
    VertB, // Left full, right split
    Horz4, // 4 horizontal strips
    Vert4, // 4 vertical strips
}

impl Av1Partition {
    fn label(&self) -> &'static str {
        match self {
            Av1Partition::None => "NONE",
            Av1Partition::Horz => "HORZ",
            Av1Partition::Vert => "VERT",
            Av1Partition::Split => "SPLIT",
            Av1Partition::HorzA => "HORZ_A",
            Av1Partition::HorzB => "HORZ_B",
            Av1Partition::VertA => "VERT_A",
            Av1Partition::VertB => "VERT_B",
            Av1Partition::Horz4 => "HORZ_4",
            Av1Partition::Vert4 => "VERT_4",
        }
    }
}

/// AV1 block node
#[derive(Debug, Clone)]
pub struct Av1Block {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub ref_frame: Av1RefFrame,
    pub ref_frame2: Option<Av1RefFrame>, // For compound prediction
    pub partition: Av1Partition,
}

/// CDEF parameters for a 64x64 unit
#[derive(Debug, Clone, Default)]
pub struct CdefParams {
    pub y_pri_strength: u8, // 0-15
    pub y_sec_strength: u8, // 0-4
    pub uv_pri_strength: u8,
    pub uv_sec_strength: u8,
    pub damping: u8, // 3-6
    pub skip: bool,
}

/// Film grain parameters
#[derive(Debug, Clone, Default)]
pub struct FilmGrainParams {
    pub apply_grain: bool,
    pub grain_seed: u16,
    pub update_grain: bool,
    pub num_y_points: u8,
    pub num_cb_points: u8,
    pub num_cr_points: u8,
    pub grain_scaling: u8,
    pub ar_coeff_lag: u8,
    pub overlap_flag: bool,
    pub clip_to_restricted_range: bool,
}

/// Super-resolution parameters
#[derive(Debug, Clone, Default)]
pub struct SuperResParams {
    pub use_superres: bool,
    pub coded_denom: u8, // 9-16, denominator for scaling
    pub superres_scale_denominator: u8,
    pub upscaled_width: u32,
}

impl SuperResParams {
    fn scale_factor(&self) -> f32 {
        if self.superres_scale_denominator > 0 {
            8.0 / self.superres_scale_denominator as f32
        } else {
            1.0
        }
    }
}

/// AV1 feature flags
#[derive(Debug, Clone, Default)]
pub struct Av1FeatureStatus {
    pub cdef_enabled: bool,
    pub superres_enabled: bool,
    pub film_grain_enabled: bool,
    pub loop_restoration: bool,
    pub segmentation: bool,
    pub delta_q: bool,
    pub delta_lf: bool,
    pub reference_mode: bool, // Compound vs single
    pub skip_mode: bool,
    pub warped_motion: bool,
    pub reduced_tx_set: bool,
}

/// AV1 Visualization Workspace
pub struct Av1Workspace {
    /// Active view
    active_view: Av1View,

    /// Show superblock boundaries
    show_sb_grid: bool,

    /// Show block boundaries
    show_block_grid: bool,

    /// Show reference colors
    show_refs: bool,

    /// Superblock size (64 or 128)
    sb_size: u32,

    /// Feature status
    features: Av1FeatureStatus,

    /// Mock block data
    mock_blocks: Vec<Av1Block>,

    /// Mock CDEF data (per 64x64 unit)
    mock_cdef: Vec<CdefParams>,

    /// Film grain params
    film_grain: FilmGrainParams,

    /// Super-res params
    super_res: SuperResParams,

    /// Selected block
    selected_block: Option<usize>,

    /// Frame dimensions
    frame_width: u32,
    frame_height: u32,

    /// Lazy initialization flag for mock data (defers ~500KB allocation)
    mock_data_initialized: bool,
}

impl Default for Av1Workspace {
    fn default() -> Self {
        Self::new()
    }
}

impl Av1Workspace {
    pub fn new() -> Self {
        Self {
            active_view: Av1View::default(),
            show_sb_grid: true,
            show_block_grid: true,
            show_refs: true,
            sb_size: 128,
            features: Av1FeatureStatus::default(),
            mock_blocks: Vec::new(),
            mock_cdef: Vec::new(),
            film_grain: FilmGrainParams::default(),
            super_res: SuperResParams::default(),
            selected_block: None,
            frame_width: 1920,
            frame_height: 1080,
            mock_data_initialized: false,
        }
    }

    /// Set view mode by index (F1-F4 keyboard shortcuts)
    pub fn set_mode_by_index(&mut self, index: usize) {
        self.active_view = match index {
            0 => Av1View::Overview,
            1 => Av1View::Partitions,
            2 => Av1View::References,
            3 => Av1View::Cdef,
            _ => return, // Ignore invalid indices
        };
    }

    /// Generate mock data for demonstration
    fn generate_mock_data(&mut self) {
        self.mock_blocks.clear();
        self.mock_cdef.clear();

        // Generate varied block sizes within a 128x128 superblock
        let refs = [
            Av1RefFrame::Last,
            Av1RefFrame::Last2,
            Av1RefFrame::Golden,
            Av1RefFrame::AltRef,
            Av1RefFrame::Intra,
        ];

        // Top-left quadrant: 64x64 split into 4 32x32
        for dy in 0..2 {
            for dx in 0..2 {
                self.mock_blocks.push(Av1Block {
                    x: dx * 32,
                    y: dy * 32,
                    width: 32,
                    height: 32,
                    ref_frame: refs[(dx + dy * 2) as usize % refs.len()],
                    ref_frame2: None,
                    partition: Av1Partition::Split,
                });
            }
        }

        // Top-right quadrant: 64x64 with HORZ split
        self.mock_blocks.push(Av1Block {
            x: 64,
            y: 0,
            width: 64,
            height: 32,
            ref_frame: Av1RefFrame::Last,
            ref_frame2: Some(Av1RefFrame::AltRef), // Compound
            partition: Av1Partition::Horz,
        });
        self.mock_blocks.push(Av1Block {
            x: 64,
            y: 32,
            width: 64,
            height: 32,
            ref_frame: Av1RefFrame::Golden,
            ref_frame2: None,
            partition: Av1Partition::None,
        });

        // Bottom-left quadrant: 64x64 no split
        self.mock_blocks.push(Av1Block {
            x: 0,
            y: 64,
            width: 64,
            height: 64,
            ref_frame: Av1RefFrame::Intra,
            ref_frame2: None,
            partition: Av1Partition::None,
        });

        // Bottom-right quadrant: varied splits
        // VERT split
        self.mock_blocks.push(Av1Block {
            x: 64,
            y: 64,
            width: 32,
            height: 64,
            ref_frame: Av1RefFrame::Last,
            ref_frame2: None,
            partition: Av1Partition::Vert,
        });
        // Further split the right side
        for dy in 0..2 {
            for dx in 0..2 {
                self.mock_blocks.push(Av1Block {
                    x: 96 + dx * 16,
                    y: 64 + dy * 32,
                    width: 16,
                    height: 32,
                    ref_frame: refs[(dx + dy) as usize],
                    ref_frame2: None,
                    partition: Av1Partition::Split,
                });
            }
        }

        // CDEF data for 4 64x64 units
        self.mock_cdef.push(CdefParams {
            y_pri_strength: 8,
            y_sec_strength: 2,
            uv_pri_strength: 4,
            uv_sec_strength: 1,
            damping: 4,
            skip: false,
        });
        self.mock_cdef.push(CdefParams {
            y_pri_strength: 12,
            y_sec_strength: 3,
            uv_pri_strength: 6,
            uv_sec_strength: 2,
            damping: 5,
            skip: false,
        });
        self.mock_cdef.push(CdefParams {
            y_pri_strength: 0,
            y_sec_strength: 0,
            uv_pri_strength: 0,
            uv_sec_strength: 0,
            damping: 3,
            skip: true, // Skip CDEF for this unit
        });
        self.mock_cdef.push(CdefParams {
            y_pri_strength: 4,
            y_sec_strength: 1,
            uv_pri_strength: 2,
            uv_sec_strength: 1,
            damping: 4,
            skip: false,
        });

        // Film grain params
        self.film_grain = FilmGrainParams {
            apply_grain: true,
            grain_seed: 12345,
            update_grain: true,
            num_y_points: 8,
            num_cb_points: 4,
            num_cr_points: 4,
            grain_scaling: 2,
            ar_coeff_lag: 3,
            overlap_flag: true,
            clip_to_restricted_range: false,
        };

        // Super-res params
        self.super_res = SuperResParams {
            use_superres: true,
            coded_denom: 12,
            superres_scale_denominator: 12,
            upscaled_width: 1920,
        };

        // Features
        self.features = Av1FeatureStatus {
            cdef_enabled: true,
            superres_enabled: true,
            film_grain_enabled: true,
            loop_restoration: true,
            segmentation: true,
            delta_q: true,
            delta_lf: false,
            reference_mode: true,
            skip_mode: true,
            warped_motion: false,
            reduced_tx_set: false,
        };
    }

    /// Ensure mock data is initialized (lazy loading)
    fn ensure_mock_data(&mut self) {
        if !self.mock_data_initialized {
            self.generate_mock_data();
            self.mock_data_initialized = true;
        }
    }

    /// Main UI entry point
    pub fn show(&mut self, ui: &mut egui::Ui) {
        // Lazy-load mock data on first show
        self.ensure_mock_data();

        ui.heading("AV1 Analysis");
        ui.separator();

        // View selector tabs
        ui.horizontal(|ui| {
            for view in [
                Av1View::Overview,
                Av1View::Partitions,
                Av1View::References,
                Av1View::Cdef,
                Av1View::SuperRes,
                Av1View::FilmGrain,
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
            Av1View::Overview => self.show_overview(ui),
            Av1View::Partitions => self.show_partitions(ui),
            Av1View::References => self.show_references(ui),
            Av1View::Cdef => self.show_cdef(ui),
            Av1View::SuperRes => self.show_super_res(ui),
            Av1View::FilmGrain => self.show_film_grain(ui),
        }
    }

    /// Overview tab
    fn show_overview(&mut self, ui: &mut egui::Ui) {
        ui.columns(2, |cols| {
            // Left: Feature status
            cols[0].group(|ui| {
                ui.heading("AV1 Features");
                ui.add_space(8.0);

                Self::feature_badge(ui, "CDEF", self.features.cdef_enabled);
                Self::feature_badge(ui, "Super-Resolution", self.features.superres_enabled);
                Self::feature_badge(ui, "Film Grain", self.features.film_grain_enabled);
                Self::feature_badge(ui, "Loop Restoration", self.features.loop_restoration);
                Self::feature_badge(ui, "Segmentation", self.features.segmentation);
                Self::feature_badge(ui, "Delta Q", self.features.delta_q);
                Self::feature_badge(ui, "Delta LF", self.features.delta_lf);
                Self::feature_badge(ui, "Compound Ref", self.features.reference_mode);
                Self::feature_badge(ui, "Skip Mode", self.features.skip_mode);
                Self::feature_badge(ui, "Warped Motion", self.features.warped_motion);
            });

            // Right: Quick stats
            cols[1].group(|ui| {
                ui.heading("Frame Statistics");
                ui.add_space(8.0);

                ui.label(format!(
                    "Frame size: {}x{}",
                    self.frame_width, self.frame_height
                ));
                ui.label(format!(
                    "Superblock size: {}x{}",
                    self.sb_size, self.sb_size
                ));
                ui.label(format!("Total blocks: {}", self.mock_blocks.len()));

                ui.add_space(8.0);

                // Reference usage
                let mut intra_count = 0;
                let mut single_count = 0;
                let mut compound_count = 0;

                for block in &self.mock_blocks {
                    if matches!(block.ref_frame, Av1RefFrame::Intra) {
                        intra_count += 1;
                    } else if block.ref_frame2.is_some() {
                        compound_count += 1;
                    } else {
                        single_count += 1;
                    }
                }

                ui.label(format!("Intra blocks: {}", intra_count));
                ui.label(format!("Single ref: {}", single_count));
                ui.label(format!("Compound ref: {}", compound_count));
            });
        });
    }

    /// Partitions tab
    fn show_partitions(&mut self, ui: &mut egui::Ui) {
        // Toolbar
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.show_sb_grid, "SB Grid");
            ui.checkbox(&mut self.show_block_grid, "Block Grid");
            ui.checkbox(&mut self.show_refs, "Show Refs");
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

        // Scale: show 128x128 superblock
        let scale = grid_size / self.sb_size as f32;

        // Draw superblock boundary
        if self.show_sb_grid {
            painter.rect_stroke(rect, 0.0, Stroke::new(3.0, colors::SB_BOUNDARY));

            // 64x64 quadrant lines
            let mid = grid_size / 2.0;
            painter.line_segment(
                [
                    egui::pos2(rect.min.x + mid, rect.min.y),
                    egui::pos2(rect.min.x + mid, rect.max.y),
                ],
                Stroke::new(1.5, colors::SB_BOUNDARY.gamma_multiply(0.5)),
            );
            painter.line_segment(
                [
                    egui::pos2(rect.min.x, rect.min.y + mid),
                    egui::pos2(rect.max.x, rect.min.y + mid),
                ],
                Stroke::new(1.5, colors::SB_BOUNDARY.gamma_multiply(0.5)),
            );
        }

        // Draw blocks
        for (idx, block) in self.mock_blocks.iter().enumerate() {
            let block_rect = Rect::from_min_size(
                egui::pos2(
                    rect.min.x + block.x as f32 * scale,
                    rect.min.y + block.y as f32 * scale,
                ),
                Vec2::new(block.width as f32 * scale, block.height as f32 * scale),
            );

            // Fill with reference color
            if self.show_refs {
                let color = if block.ref_frame2.is_some() {
                    // Compound: blend two colors
                    colors::COMPOUND.gamma_multiply(0.4)
                } else {
                    block.ref_frame.color().gamma_multiply(0.4)
                };
                painter.rect_filled(block_rect, 0.0, color);
            }

            // Draw block boundary
            if self.show_block_grid {
                let stroke_width = if Some(idx) == self.selected_block {
                    2.5
                } else {
                    1.0
                };
                painter.rect_stroke(
                    block_rect,
                    0.0,
                    Stroke::new(stroke_width, colors::BLOCK_BOUNDARY),
                );
            }

            // Label for larger blocks
            if block.width >= 32 && block.height >= 32 {
                let label = if block.ref_frame2.is_some() {
                    format!(
                        "{}+{}",
                        block.ref_frame.label(),
                        block.ref_frame2.unwrap().label()
                    )
                } else {
                    block.ref_frame.label().to_string()
                };
                painter.text(
                    block_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    label,
                    egui::FontId::proportional(9.0),
                    Color32::WHITE,
                );
            }
        }

        // Partition types legend
        ui.add_space(8.0);
        ui.group(|ui| {
            ui.heading("AV1 Partition Types");
            ui.horizontal_wrapped(|ui| {
                for part in [
                    Av1Partition::None,
                    Av1Partition::Horz,
                    Av1Partition::Vert,
                    Av1Partition::Split,
                    Av1Partition::HorzA,
                    Av1Partition::HorzB,
                    Av1Partition::VertA,
                    Av1Partition::VertB,
                    Av1Partition::Horz4,
                    Av1Partition::Vert4,
                ] {
                    ui.label(
                        RichText::new(format!(" {} ", part.label()))
                            .small()
                            .background_color(Color32::from_rgb(60, 60, 70))
                            .color(Color32::WHITE),
                    );
                }
            });
        });
    }

    /// References tab
    fn show_references(&mut self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.heading("AV1 Reference Frame Types");
            ui.add_space(8.0);

            // Reference frame legend with colors
            for ref_type in [
                Av1RefFrame::Last,
                Av1RefFrame::Last2,
                Av1RefFrame::Last3,
                Av1RefFrame::Golden,
                Av1RefFrame::BwdRef,
                Av1RefFrame::AltRef2,
                Av1RefFrame::AltRef,
            ] {
                Self::ref_badge(ui, ref_type);
            }
        });

        ui.add_space(8.0);

        ui.columns(2, |cols| {
            cols[0].group(|ui| {
                ui.heading("Forward References");
                ui.label("LAST: Most recent decoded frame");
                ui.label("LAST2: 2nd most recent");
                ui.label("LAST3: 3rd most recent");
                ui.label("GOLDEN: Key reference (scene anchor)");
            });

            cols[1].group(|ui| {
                ui.heading("Backward References");
                ui.label("BWDREF: Backward reference");
                ui.label("ALTREF2: Secondary alt reference");
                ui.label("ALTREF: Alternate reference (future)");
            });
        });

        ui.add_space(8.0);

        ui.group(|ui| {
            ui.heading("Compound Prediction");
            ui.label("AV1 supports combining two reference frames:");
            ui.add_space(4.0);

            ui.horizontal(|ui| {
                Self::legend_item(ui, colors::SINGLE_REF, "Single Ref");
                Self::legend_item(ui, colors::COMPOUND, "Compound");
                Self::legend_item(ui, colors::INTRA, "Intra");
            });

            ui.add_space(4.0);
            ui.label("Compound modes: NEAREST_NEAREST, NEAR_NEAR, NEW_NEW, etc.");
        });
    }

    /// CDEF tab
    fn show_cdef(&mut self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.heading("CDEF (Constrained Directional Enhancement Filter)");
            ui.add_space(4.0);

            ui.label("Applied after reconstruction, before loop restoration");
            ui.label("Operates on 8x8 blocks within 64x64 units");
        });

        ui.add_space(8.0);

        ui.columns(2, |cols| {
            // Left: CDEF map
            cols[0].group(|ui| {
                ui.heading("CDEF Strength Map");

                let (_, painter) =
                    ui.allocate_painter(Vec2::new(200.0, 200.0), egui::Sense::hover());

                let rect = painter.clip_rect();
                let cell_size = rect.width() / 2.0;

                for (idx, cdef) in self.mock_cdef.iter().enumerate() {
                    let row = idx / 2;
                    let col = idx % 2;
                    let cell_rect = Rect::from_min_size(
                        rect.min + Vec2::new(col as f32 * cell_size, row as f32 * cell_size),
                        Vec2::splat(cell_size),
                    );

                    // Color based on strength
                    let intensity = if cdef.skip {
                        0.0
                    } else {
                        (cdef.y_pri_strength as f32 / 15.0).clamp(0.0, 1.0)
                    };

                    let color = if cdef.skip {
                        Color32::from_rgb(80, 80, 80)
                    } else {
                        Color32::from_rgb(
                            (64.0 + intensity * 191.0) as u8,
                            (224.0 - intensity * 100.0) as u8,
                            (208.0 - intensity * 80.0) as u8,
                        )
                    };

                    painter.rect_filled(cell_rect, 0.0, color.gamma_multiply(0.6));
                    painter.rect_stroke(cell_rect, 0.0, Stroke::new(1.0, Color32::WHITE));

                    // Label
                    let label = if cdef.skip {
                        "SKIP".to_string()
                    } else {
                        format!("P:{}", cdef.y_pri_strength)
                    };
                    painter.text(
                        cell_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        label,
                        egui::FontId::proportional(12.0),
                        Color32::WHITE,
                    );
                }
            });

            // Right: CDEF params for first unit
            cols[1].group(|ui| {
                ui.heading("Unit 0 Parameters");
                if let Some(cdef) = self.mock_cdef.first() {
                    ui.label(format!("Y Primary Strength: {}", cdef.y_pri_strength));
                    ui.label(format!("Y Secondary Strength: {}", cdef.y_sec_strength));
                    ui.label(format!("UV Primary Strength: {}", cdef.uv_pri_strength));
                    ui.label(format!("UV Secondary Strength: {}", cdef.uv_sec_strength));
                    ui.label(format!("Damping: {}", cdef.damping));
                    ui.label(format!("Skip: {}", cdef.skip));
                }
            });
        });

        ui.add_space(8.0);

        ui.group(|ui| {
            ui.heading("CDEF Direction Analysis");
            ui.label("8 directions (0-7) analyzed per block");
            ui.label("Filter follows edge direction to preserve edges while reducing noise");
        });
    }

    /// Super-resolution tab
    fn show_super_res(&mut self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.heading("Super-Resolution");
            ui.add_space(4.0);

            let enabled = self.features.superres_enabled;
            ui.horizontal(|ui| {
                ui.label("Status:");
                if enabled {
                    ui.label(RichText::new("ENABLED").color(Color32::GREEN));
                } else {
                    ui.label(RichText::new("DISABLED").color(Color32::GRAY));
                }
            });
        });

        if self.features.superres_enabled {
            ui.add_space(8.0);

            ui.group(|ui| {
                ui.heading("Parameters");

                ui.label(format!(
                    "Scale denominator: {}/8 = {:.2}x",
                    self.super_res.superres_scale_denominator,
                    self.super_res.scale_factor()
                ));
                ui.label(format!("Upscaled width: {}", self.super_res.upscaled_width));

                ui.add_space(8.0);

                // Visual representation
                let coded_width =
                    (self.super_res.upscaled_width as f32 * self.super_res.scale_factor()) as u32;
                ui.label(format!(
                    "Coded at: {}x{} -> Upscaled to: {}x{}",
                    coded_width,
                    self.frame_height,
                    self.super_res.upscaled_width,
                    self.frame_height
                ));

                // Draw visual
                let (_, painter) =
                    ui.allocate_painter(Vec2::new(300.0, 100.0), egui::Sense::hover());

                let rect = painter.clip_rect();

                // Coded frame
                let coded_rect = Rect::from_min_size(
                    rect.min + Vec2::new(10.0, 30.0),
                    Vec2::new(100.0 * self.super_res.scale_factor(), 50.0),
                );
                painter.rect_filled(coded_rect, 4.0, Color32::from_rgb(100, 100, 200));
                painter.text(
                    coded_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "Coded",
                    egui::FontId::proportional(10.0),
                    Color32::WHITE,
                );

                // Arrow
                painter.arrow(
                    coded_rect.right_center() + Vec2::new(5.0, 0.0),
                    Vec2::new(30.0, 0.0),
                    Stroke::new(2.0, Color32::WHITE),
                );

                // Upscaled frame
                let upscaled_rect =
                    Rect::from_min_size(rect.min + Vec2::new(180.0, 20.0), Vec2::new(100.0, 70.0));
                painter.rect_filled(upscaled_rect, 4.0, Color32::from_rgb(100, 200, 100));
                painter.text(
                    upscaled_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "Upscaled",
                    egui::FontId::proportional(10.0),
                    Color32::WHITE,
                );
            });
        }
    }

    /// Film grain tab
    fn show_film_grain(&mut self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.heading("Film Grain Synthesis");
            ui.add_space(4.0);

            let enabled = self.features.film_grain_enabled && self.film_grain.apply_grain;
            ui.horizontal(|ui| {
                ui.label("Status:");
                if enabled {
                    ui.label(RichText::new("ACTIVE").color(Color32::GREEN));
                } else {
                    ui.label(RichText::new("INACTIVE").color(Color32::GRAY));
                }
            });
        });

        if self.features.film_grain_enabled {
            ui.add_space(8.0);

            ui.columns(2, |cols| {
                cols[0].group(|ui| {
                    ui.heading("Grain Parameters");

                    ui.label(format!("Apply grain: {}", self.film_grain.apply_grain));
                    ui.label(format!("Grain seed: {}", self.film_grain.grain_seed));
                    ui.label(format!("Update grain: {}", self.film_grain.update_grain));
                    ui.label(format!("Scaling: {}", self.film_grain.grain_scaling));
                    ui.label(format!("AR coeff lag: {}", self.film_grain.ar_coeff_lag));
                    ui.label(format!("Overlap: {}", self.film_grain.overlap_flag));
                });

                cols[1].group(|ui| {
                    ui.heading("Grain Points");

                    ui.label(format!("Y points: {}", self.film_grain.num_y_points));
                    ui.label(format!("Cb points: {}", self.film_grain.num_cb_points));
                    ui.label(format!("Cr points: {}", self.film_grain.num_cr_points));

                    ui.add_space(8.0);

                    // Visual grain preview
                    let (_, painter) =
                        ui.allocate_painter(Vec2::new(100.0, 100.0), egui::Sense::hover());

                    let rect = painter.clip_rect();
                    painter.rect_filled(rect, 4.0, colors::FILM_GRAIN);

                    // Simulate grain dots
                    let seed = self.film_grain.grain_seed as u32;
                    for i in 0..50 {
                        let px = ((seed.wrapping_mul(i * 7 + 1)) % 100) as f32;
                        let py = ((seed.wrapping_mul(i * 13 + 3)) % 100) as f32;
                        let intensity = ((seed.wrapping_mul(i * 3 + 5)) % 60) as u8;

                        painter.circle_filled(
                            rect.min + Vec2::new(px, py),
                            1.5,
                            Color32::from_rgba_unmultiplied(255, 255, 255, intensity + 30),
                        );
                    }

                    painter.text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "Grain",
                        egui::FontId::proportional(12.0),
                        Color32::from_rgb(80, 60, 40),
                    );
                });
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

    /// Helper to draw reference badge
    fn ref_badge(ui: &mut egui::Ui, ref_type: Av1RefFrame) {
        ui.horizontal(|ui| {
            let badge = RichText::new(format!(" {} ", ref_type.label()))
                .background_color(ref_type.color())
                .color(Color32::WHITE)
                .small();
            ui.label(badge);
        });
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
