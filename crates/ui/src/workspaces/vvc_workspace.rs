//! VVC Visualization Workspace - VVC/H.266 specific analysis (Monster Pack v14)
//!
//! Provides visualization for VVC-specific features:
//! - Dual Tree (QTMT/BTT splitting for luma/chroma)
//! - Predictions (intra/inter modes, MVs, reference frames)
//! - Transform & Reconstruction stages
//! - VVC-specific tools: ALF, LMCS, GDR

use bitvue_core::{Command, SelectionState};
use egui::{self, Color32, Vec2};

/// Professional color palette for VVC visualization
mod colors {
    use egui::Color32;

    pub const BACKGROUND: Color32 = Color32::from_rgb(250, 250, 250);
    pub const PANEL_BG: Color32 = Color32::from_rgb(245, 245, 248);
    pub const GRID: Color32 = Color32::from_rgb(220, 220, 220);
    pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(40, 40, 40);
    pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(100, 100, 100);

    // Dual Tree colors
    pub const LUMA_TREE: Color32 = Color32::from_rgb(30, 144, 255); // Dodger blue
    pub const CHROMA_TREE: Color32 = Color32::from_rgb(255, 140, 0); // Orange
    pub const JOINT_TREE: Color32 = Color32::from_rgb(50, 205, 50); // Lime green

    // Partition colors (VVC QTMT)
    pub const QUAD_SPLIT: Color32 = Color32::from_rgb(100, 149, 237); // Cornflower blue
    pub const BINARY_VERT: Color32 = Color32::from_rgb(144, 238, 144); // Light green
    pub const BINARY_HORZ: Color32 = Color32::from_rgb(255, 182, 193); // Light pink
    pub const TERNARY_VERT: Color32 = Color32::from_rgb(255, 218, 185); // Peach
    pub const TERNARY_HORZ: Color32 = Color32::from_rgb(221, 160, 221); // Plum
    pub const NO_SPLIT: Color32 = Color32::from_rgb(200, 200, 200);

    // Prediction mode colors
    pub const INTRA_DC: Color32 = Color32::from_rgb(255, 99, 71); // Tomato
    pub const INTRA_PLANAR: Color32 = Color32::from_rgb(255, 165, 0); // Orange
    pub const INTRA_ANGULAR: Color32 = Color32::from_rgb(255, 215, 0); // Gold
    pub const INTER_MERGE: Color32 = Color32::from_rgb(100, 149, 237); // Cornflower blue
    pub const INTER_AMVP: Color32 = Color32::from_rgb(30, 144, 255); // Dodger blue
    pub const INTER_AFFINE: Color32 = Color32::from_rgb(138, 43, 226); // Blue violet
    pub const INTER_GPM: Color32 = Color32::from_rgb(186, 85, 211); // Medium orchid
    pub const SKIP: Color32 = Color32::from_rgb(128, 128, 128); // Gray

    // Feature status colors
    pub const FEATURE_ENABLED: Color32 = Color32::from_rgb(50, 205, 50);
    pub const FEATURE_DISABLED: Color32 = Color32::from_rgb(180, 180, 180);
}

/// VVC partition split type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VvcSplitType {
    NoSplit,
    QuadTree,
    BinaryVertical,
    BinaryHorizontal,
    TernaryVertical,
    TernaryHorizontal,
}

impl VvcSplitType {
    pub fn label(&self) -> &'static str {
        match self {
            VvcSplitType::NoSplit => "No Split",
            VvcSplitType::QuadTree => "Quad (QT)",
            VvcSplitType::BinaryVertical => "Binary V (BT)",
            VvcSplitType::BinaryHorizontal => "Binary H (BT)",
            VvcSplitType::TernaryVertical => "Ternary V (TT)",
            VvcSplitType::TernaryHorizontal => "Ternary H (TT)",
        }
    }

    pub fn color(&self) -> Color32 {
        match self {
            VvcSplitType::NoSplit => colors::NO_SPLIT,
            VvcSplitType::QuadTree => colors::QUAD_SPLIT,
            VvcSplitType::BinaryVertical => colors::BINARY_VERT,
            VvcSplitType::BinaryHorizontal => colors::BINARY_HORZ,
            VvcSplitType::TernaryVertical => colors::TERNARY_VERT,
            VvcSplitType::TernaryHorizontal => colors::TERNARY_HORZ,
        }
    }
}

/// Dual tree mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DualTreeMode {
    #[default]
    Joint,
    LumaOnly,
    ChromaOnly,
    SeparateTrees,
}

impl DualTreeMode {
    pub fn label(&self) -> &'static str {
        match self {
            DualTreeMode::Joint => "Joint (Single Tree)",
            DualTreeMode::LumaOnly => "Luma Only",
            DualTreeMode::ChromaOnly => "Chroma Only",
            DualTreeMode::SeparateTrees => "Separate Trees",
        }
    }
}

/// VVC feature status (from SPS)
#[derive(Debug, Clone, Default)]
pub struct VvcFeatureStatus {
    pub dual_tree_enabled: bool,
    pub alf_enabled: bool,
    pub lmcs_enabled: bool,
    pub gdr_enabled: bool,
    pub affine_enabled: bool,
    pub gpm_enabled: bool,
    pub ibc_enabled: bool,
    pub mts_enabled: bool,
    pub lfnst_enabled: bool,
    pub sbt_enabled: bool,
}

/// Mock partition node for visualization
#[derive(Debug, Clone)]
pub struct PartitionNode {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub depth: u8,
    pub split_type: VvcSplitType,
    pub is_luma: bool,
}

/// VVC Visualization Workspace
pub struct VvcWorkspace {
    /// Active panel view
    active_view: VvcView,

    /// Dual tree display mode
    dual_tree_mode: DualTreeMode,

    /// Show partition boundaries
    show_partitions: bool,

    /// Show prediction modes
    show_predictions: bool,

    /// VVC feature status
    features: VvcFeatureStatus,

    /// Mock partition data (would come from decoder)
    mock_partitions: Vec<PartitionNode>,

    /// Selected block index
    selected_block: Option<usize>,

    /// Zoom level
    zoom: f32,

    /// Flag to track if mock data has been initialized
    mock_data_initialized: bool,
}

/// Active visualization view
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VvcView {
    #[default]
    Overview,
    DualTree,
    Predictions,
    Transform,
}

impl VvcView {
    pub fn label(&self) -> &'static str {
        match self {
            VvcView::Overview => "Overview",
            VvcView::DualTree => "Dual Tree",
            VvcView::Predictions => "Predictions",
            VvcView::Transform => "Transform",
        }
    }
}

impl VvcWorkspace {
    pub fn new() -> Self {
        Self {
            active_view: VvcView::Overview,
            dual_tree_mode: DualTreeMode::Joint,
            show_partitions: true,
            show_predictions: true,
            features: VvcFeatureStatus {
                dual_tree_enabled: true,
                alf_enabled: true,
                lmcs_enabled: true,
                gdr_enabled: false,
                affine_enabled: true,
                gpm_enabled: true,
                ibc_enabled: false,
                mts_enabled: true,
                lfnst_enabled: true,
                sbt_enabled: true,
            },
            mock_partitions: Vec::new(),
            selected_block: None,
            zoom: 1.0,
            mock_data_initialized: false,
        }
    }

    /// Set view mode by index (F1-F4 keyboard shortcuts)
    pub fn set_mode_by_index(&mut self, index: usize) {
        self.active_view = match index {
            0 => VvcView::Overview,
            1 => VvcView::DualTree,
            2 => VvcView::Predictions,
            3 => VvcView::Transform,
            _ => return, // Ignore invalid indices
        };
    }

    /// Ensure mock data is initialized (lazy loading)
    fn ensure_mock_data(&mut self) {
        if !self.mock_data_initialized {
            self.mock_partitions = Self::generate_mock_partitions();
            self.mock_data_initialized = true;
        }
    }

    /// Generate mock partition data for visualization demo
    fn generate_mock_partitions() -> Vec<PartitionNode> {
        let mut partitions = Vec::new();
        let ctu_size = 128u32;

        // Generate a 4x4 CTU grid with various partition patterns
        for ctu_y in 0..4 {
            for ctu_x in 0..4 {
                let base_x = ctu_x * ctu_size;
                let base_y = ctu_y * ctu_size;

                // Vary the partition pattern based on position
                let pattern = (ctu_x + ctu_y * 4) % 6;

                match pattern {
                    0 => {
                        // Quad split into 4
                        let half = ctu_size / 2;
                        for dy in 0..2 {
                            for dx in 0..2 {
                                partitions.push(PartitionNode {
                                    x: base_x + dx * half,
                                    y: base_y + dy * half,
                                    width: half,
                                    height: half,
                                    depth: 1,
                                    split_type: VvcSplitType::QuadTree,
                                    is_luma: true,
                                });
                            }
                        }
                    }
                    1 => {
                        // Binary vertical split
                        let half = ctu_size / 2;
                        partitions.push(PartitionNode {
                            x: base_x,
                            y: base_y,
                            width: half,
                            height: ctu_size,
                            depth: 1,
                            split_type: VvcSplitType::BinaryVertical,
                            is_luma: true,
                        });
                        partitions.push(PartitionNode {
                            x: base_x + half,
                            y: base_y,
                            width: half,
                            height: ctu_size,
                            depth: 1,
                            split_type: VvcSplitType::BinaryVertical,
                            is_luma: true,
                        });
                    }
                    2 => {
                        // Binary horizontal split
                        let half = ctu_size / 2;
                        partitions.push(PartitionNode {
                            x: base_x,
                            y: base_y,
                            width: ctu_size,
                            height: half,
                            depth: 1,
                            split_type: VvcSplitType::BinaryHorizontal,
                            is_luma: true,
                        });
                        partitions.push(PartitionNode {
                            x: base_x,
                            y: base_y + half,
                            width: ctu_size,
                            height: half,
                            depth: 1,
                            split_type: VvcSplitType::BinaryHorizontal,
                            is_luma: true,
                        });
                    }
                    3 => {
                        // Ternary vertical split
                        let third = ctu_size / 4;
                        partitions.push(PartitionNode {
                            x: base_x,
                            y: base_y,
                            width: third,
                            height: ctu_size,
                            depth: 1,
                            split_type: VvcSplitType::TernaryVertical,
                            is_luma: true,
                        });
                        partitions.push(PartitionNode {
                            x: base_x + third,
                            y: base_y,
                            width: third * 2,
                            height: ctu_size,
                            depth: 1,
                            split_type: VvcSplitType::TernaryVertical,
                            is_luma: true,
                        });
                        partitions.push(PartitionNode {
                            x: base_x + third * 3,
                            y: base_y,
                            width: third,
                            height: ctu_size,
                            depth: 1,
                            split_type: VvcSplitType::TernaryVertical,
                            is_luma: true,
                        });
                    }
                    4 => {
                        // Ternary horizontal split
                        let third = ctu_size / 4;
                        partitions.push(PartitionNode {
                            x: base_x,
                            y: base_y,
                            width: ctu_size,
                            height: third,
                            depth: 1,
                            split_type: VvcSplitType::TernaryHorizontal,
                            is_luma: true,
                        });
                        partitions.push(PartitionNode {
                            x: base_x,
                            y: base_y + third,
                            width: ctu_size,
                            height: third * 2,
                            depth: 1,
                            split_type: VvcSplitType::TernaryHorizontal,
                            is_luma: true,
                        });
                        partitions.push(PartitionNode {
                            x: base_x,
                            y: base_y + third * 3,
                            width: ctu_size,
                            height: third,
                            depth: 1,
                            split_type: VvcSplitType::TernaryHorizontal,
                            is_luma: true,
                        });
                    }
                    _ => {
                        // No split
                        partitions.push(PartitionNode {
                            x: base_x,
                            y: base_y,
                            width: ctu_size,
                            height: ctu_size,
                            depth: 0,
                            split_type: VvcSplitType::NoSplit,
                            is_luma: true,
                        });
                    }
                }
            }
        }

        partitions
    }

    /// Show the VVC workspace
    pub fn show(&mut self, ui: &mut egui::Ui, _selection: &SelectionState) -> Option<Command> {
        // Lazy-load mock data on first show
        self.ensure_mock_data();

        let mut _clicked_command: Option<Command> = None;

        // Header toolbar
        ui.horizontal(|ui| {
            ui.heading("VVC/H.266 Analysis");
            ui.separator();

            // View selector
            ui.label("View:");
            egui::ComboBox::from_id_salt("vvc_view")
                .selected_text(self.active_view.label())
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.active_view, VvcView::Overview, "Overview");
                    ui.selectable_value(&mut self.active_view, VvcView::DualTree, "Dual Tree");
                    ui.selectable_value(&mut self.active_view, VvcView::Predictions, "Predictions");
                    ui.selectable_value(&mut self.active_view, VvcView::Transform, "Transform");
                });

            ui.separator();

            // Zoom
            ui.label("Zoom:");
            if ui.button("-").clicked() {
                self.zoom = (self.zoom - 0.25).max(0.25);
            }
            ui.label(format!("{:.0}%", self.zoom * 100.0));
            if ui.button("+").clicked() {
                self.zoom = (self.zoom + 0.25).min(4.0);
            }
        });

        ui.separator();

        // Main content based on active view
        match self.active_view {
            VvcView::Overview => self.render_overview(ui),
            VvcView::DualTree => self.render_dual_tree(ui),
            VvcView::Predictions => self.render_predictions(ui),
            VvcView::Transform => self.render_transform(ui),
        }

        None
    }

    /// Render the overview panel
    fn render_overview(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            // Left: Feature status panel
            ui.vertical(|ui| {
                ui.heading(egui::RichText::new("VVC Features").size(14.0));
                ui.add_space(10.0);

                self.render_feature_badge(ui, "Dual Tree (QTBTT)", self.features.dual_tree_enabled);
                self.render_feature_badge(
                    ui,
                    "ALF (Adaptive Loop Filter)",
                    self.features.alf_enabled,
                );
                self.render_feature_badge(ui, "LMCS (Luma Mapping)", self.features.lmcs_enabled);
                self.render_feature_badge(ui, "GDR (Gradual Decoding)", self.features.gdr_enabled);
                self.render_feature_badge(ui, "Affine Motion", self.features.affine_enabled);
                self.render_feature_badge(ui, "GPM (Geo Partition)", self.features.gpm_enabled);
                self.render_feature_badge(ui, "IBC (Intra Block Copy)", self.features.ibc_enabled);
                self.render_feature_badge(
                    ui,
                    "MTS (Multiple Transforms)",
                    self.features.mts_enabled,
                );
                self.render_feature_badge(ui, "LFNST", self.features.lfnst_enabled);
                self.render_feature_badge(
                    ui,
                    "SBT (Sub-Block Transform)",
                    self.features.sbt_enabled,
                );
            });

            ui.separator();

            // Right: Partition preview
            ui.vertical(|ui| {
                ui.heading(egui::RichText::new("QTMT Partition Preview").size(14.0));
                ui.add_space(10.0);

                self.render_partition_grid(ui);
            });
        });

        // Legend
        ui.separator();
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Partition Types:").strong());
            self.render_legend_item(ui, "Quad", colors::QUAD_SPLIT);
            self.render_legend_item(ui, "Binary V", colors::BINARY_VERT);
            self.render_legend_item(ui, "Binary H", colors::BINARY_HORZ);
            self.render_legend_item(ui, "Ternary V", colors::TERNARY_VERT);
            self.render_legend_item(ui, "Ternary H", colors::TERNARY_HORZ);
            self.render_legend_item(ui, "No Split", colors::NO_SPLIT);
        });
    }

    /// Render a feature status badge
    fn render_feature_badge(&self, ui: &mut egui::Ui, name: &str, enabled: bool) {
        ui.horizontal(|ui| {
            let (symbol, color) = if enabled {
                ("\u{2713}", colors::FEATURE_ENABLED)
            } else {
                ("\u{2717}", colors::FEATURE_DISABLED)
            };

            ui.label(egui::RichText::new(symbol).color(color).strong());
            ui.label(egui::RichText::new(name).color(if enabled {
                colors::TEXT_PRIMARY
            } else {
                colors::TEXT_SECONDARY
            }));
        });
    }

    /// Render legend item
    fn render_legend_item(&self, ui: &mut egui::Ui, label: &str, color: Color32) {
        ui.horizontal(|ui| {
            let (rect, _) = ui.allocate_exact_size(Vec2::new(12.0, 12.0), egui::Sense::hover());
            ui.painter().rect_filled(rect, 2.0, color);
            ui.label(egui::RichText::new(label).small());
        });
    }

    /// Render the partition grid visualization
    fn render_partition_grid(&self, ui: &mut egui::Ui) {
        let grid_size = 512.0 * self.zoom;
        let scale = grid_size / 512.0; // 4x128 CTUs = 512 pixels

        let (rect, response) =
            ui.allocate_exact_size(Vec2::new(grid_size, grid_size), egui::Sense::hover());

        if ui.is_rect_visible(rect) {
            let painter = ui.painter();

            // Background
            painter.rect_filled(rect, 4.0, colors::BACKGROUND);

            // Draw partitions
            for partition in &self.mock_partitions {
                let block_rect = egui::Rect::from_min_size(
                    egui::pos2(
                        rect.min.x + partition.x as f32 * scale,
                        rect.min.y + partition.y as f32 * scale,
                    ),
                    Vec2::new(
                        partition.width as f32 * scale,
                        partition.height as f32 * scale,
                    ),
                );

                // Fill with partition type color
                painter.rect_filled(
                    block_rect,
                    0.0,
                    Color32::from_rgba_unmultiplied(
                        partition.split_type.color().r(),
                        partition.split_type.color().g(),
                        partition.split_type.color().b(),
                        100,
                    ),
                );

                // Draw boundary
                painter.rect_stroke(
                    block_rect,
                    0.0,
                    egui::Stroke::new(1.0, partition.split_type.color()),
                );
            }

            // Draw CTU grid overlay
            let ctu_size = 128.0 * scale;
            for i in 0..=4 {
                let offset = i as f32 * ctu_size;
                // Vertical
                painter.line_segment(
                    [
                        egui::pos2(rect.min.x + offset, rect.min.y),
                        egui::pos2(rect.min.x + offset, rect.max.y),
                    ],
                    egui::Stroke::new(2.0, colors::GRID),
                );
                // Horizontal
                painter.line_segment(
                    [
                        egui::pos2(rect.min.x, rect.min.y + offset),
                        egui::pos2(rect.max.x, rect.min.y + offset),
                    ],
                    egui::Stroke::new(2.0, colors::GRID),
                );
            }
        }

        // Tooltip on hover
        if let Some(pos) = response.hover_pos() {
            let local_x = ((pos.x - rect.min.x) / self.zoom) as u32;
            let local_y = ((pos.y - rect.min.y) / self.zoom) as u32;

            // Find partition at this position
            if let Some(partition) = self.mock_partitions.iter().find(|p| {
                local_x >= p.x
                    && local_x < p.x + p.width
                    && local_y >= p.y
                    && local_y < p.y + p.height
            }) {
                response.on_hover_ui(|ui| {
                    ui.label(format!("Position: ({}, {})", partition.x, partition.y));
                    ui.label(format!("Size: {}x{}", partition.width, partition.height));
                    ui.label(format!("Split: {}", partition.split_type.label()));
                    ui.label(format!("Depth: {}", partition.depth));
                });
            }
        }
    }

    /// Render the dual tree view
    fn render_dual_tree(&mut self, ui: &mut egui::Ui) {
        // Dual tree mode selector
        ui.horizontal(|ui| {
            ui.label("Display Mode:");
            egui::ComboBox::from_id_salt("dual_tree_mode")
                .selected_text(self.dual_tree_mode.label())
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut self.dual_tree_mode,
                        DualTreeMode::Joint,
                        "Joint Tree",
                    );
                    ui.selectable_value(
                        &mut self.dual_tree_mode,
                        DualTreeMode::LumaOnly,
                        "Luma Only",
                    );
                    ui.selectable_value(
                        &mut self.dual_tree_mode,
                        DualTreeMode::ChromaOnly,
                        "Chroma Only",
                    );
                    ui.selectable_value(
                        &mut self.dual_tree_mode,
                        DualTreeMode::SeparateTrees,
                        "Side by Side",
                    );
                });
        });

        ui.separator();

        match self.dual_tree_mode {
            DualTreeMode::SeparateTrees => {
                // Side by side view
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(
                            egui::RichText::new("Luma Tree")
                                .color(colors::LUMA_TREE)
                                .strong(),
                        );
                        self.render_partition_grid(ui);
                    });
                    ui.separator();
                    ui.vertical(|ui| {
                        ui.label(
                            egui::RichText::new("Chroma Tree")
                                .color(colors::CHROMA_TREE)
                                .strong(),
                        );
                        self.render_partition_grid(ui);
                    });
                });
            }
            _ => {
                let tree_label = match self.dual_tree_mode {
                    DualTreeMode::Joint => "Joint Tree (Luma + Chroma)",
                    DualTreeMode::LumaOnly => "Luma Tree",
                    DualTreeMode::ChromaOnly => "Chroma Tree",
                    _ => "",
                };
                ui.label(egui::RichText::new(tree_label).strong());
                self.render_partition_grid(ui);
            }
        }
    }

    /// Render predictions view
    fn render_predictions(&self, ui: &mut egui::Ui) {
        ui.heading(egui::RichText::new("Prediction Modes").size(14.0));
        ui.add_space(10.0);

        // Legend for prediction modes
        ui.horizontal(|ui| {
            ui.label("Intra:");
            self.render_legend_item(ui, "DC", colors::INTRA_DC);
            self.render_legend_item(ui, "Planar", colors::INTRA_PLANAR);
            self.render_legend_item(ui, "Angular", colors::INTRA_ANGULAR);
            ui.separator();
            ui.label("Inter:");
            self.render_legend_item(ui, "Merge", colors::INTER_MERGE);
            self.render_legend_item(ui, "AMVP", colors::INTER_AMVP);
            self.render_legend_item(ui, "Affine", colors::INTER_AFFINE);
            self.render_legend_item(ui, "GPM", colors::INTER_GPM);
            self.render_legend_item(ui, "Skip", colors::SKIP);
        });

        ui.separator();

        ui.centered_and_justified(|ui| {
            ui.label(
                egui::RichText::new(
                    "Prediction mode visualization requires decoded CU data.\n\n\
                     This view will show:\n\
                     - Intra prediction modes (DC, Planar, Angular 2-66)\n\
                     - Inter prediction modes (Merge, AMVP, Affine, GPM)\n\
                     - Motion vectors and reference frames\n\
                     - Skip/direct mode indicators",
                )
                .color(Color32::GRAY),
            );
        });
    }

    /// Render transform view
    fn render_transform(&self, ui: &mut egui::Ui) {
        ui.heading(egui::RichText::new("Transform & Reconstruction").size(14.0));
        ui.add_space(10.0);

        ui.horizontal(|ui| {
            // Transform info panel
            ui.vertical(|ui| {
                ui.label(egui::RichText::new("VVC Transform Tools").strong());
                self.render_feature_badge(ui, "DCT-2 (Primary)", true);
                self.render_feature_badge(ui, "DST-7", self.features.mts_enabled);
                self.render_feature_badge(ui, "DCT-8", self.features.mts_enabled);
                self.render_feature_badge(ui, "LFNST (Secondary)", self.features.lfnst_enabled);
                self.render_feature_badge(ui, "SBT", self.features.sbt_enabled);
            });

            ui.separator();

            ui.vertical(|ui| {
                ui.label(egui::RichText::new("Reconstruction Pipeline").strong());
                ui.label("1. Inverse Transform");
                ui.label("2. Add Prediction");
                ui.label("3. LMCS (if enabled)");
                ui.label("4. Deblocking Filter");
                ui.label("5. SAO");
                ui.label("6. ALF");
            });
        });

        ui.separator();

        ui.centered_and_justified(|ui| {
            ui.label(
                egui::RichText::new(
                    "Transform visualization requires decoded coefficient data.\n\n\
                     This view will show:\n\
                     - Transform type per block (DCT-2, DST-7, DCT-8)\n\
                     - LFNST application regions\n\
                     - SBT half-transform regions\n\
                     - Coefficient magnitude heatmap",
                )
                .color(Color32::GRAY),
            );
        });
    }
}

impl Default for VvcWorkspace {
    fn default() -> Self {
        Self::new()
    }
}
