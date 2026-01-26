//! Reference Workspace - WS_REFERENCE_DPB (Monster Pack v14)
//!
//! Mixed visualization workspace with:
//! - Reference graph (nodes=frames, edges=reference links)
//! - Reference list table (L0/L1 for selected frame)
//! - DPB inspector
//! - Risk indicators (depth, missing refs)

use bitvue_core::{Command, FrameKey, SelectionState, StreamId};
use egui::{self, Color32};

/// Professional color palette for reference graph
#[allow(dead_code)]
mod colors {
    use egui::Color32;

    pub const BACKGROUND: Color32 = Color32::from_rgb(250, 250, 250);
    pub const GRID: Color32 = Color32::from_rgb(235, 235, 235);
    pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(40, 40, 40);
    pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(100, 100, 100);

    // Frame type colors
    pub const NODE_I: Color32 = Color32::from_rgb(255, 100, 100); // Red for I-frames
    pub const NODE_P: Color32 = Color32::from_rgb(100, 200, 100); // Green for P-frames
    pub const NODE_B: Color32 = Color32::from_rgb(100, 150, 255); // Blue for B-frames

    // Edge colors
    pub const EDGE_L0: Color32 = Color32::from_rgb(50, 150, 50); // Green for L0 refs
    pub const EDGE_L1: Color32 = Color32::from_rgb(150, 50, 150); // Purple for L1 refs
    pub const EDGE_LONG_TERM: Color32 = Color32::from_rgb(255, 140, 0); // Orange for long-term

    // Selection
    pub const NODE_SELECTED: Color32 = Color32::from_rgb(255, 200, 50);
    pub const NODE_REF_HIGHLIGHT: Color32 = Color32::from_rgb(200, 255, 200);
    pub const NODE_DEP_HIGHLIGHT: Color32 = Color32::from_rgb(255, 200, 200);

    // Risk indicators
    pub const RISK_LOW: Color32 = Color32::from_rgb(50, 200, 50);
    pub const RISK_MEDIUM: Color32 = Color32::from_rgb(255, 200, 50);
    pub const RISK_HIGH: Color32 = Color32::from_rgb(255, 50, 50);
}

/// Frame type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameType {
    I,
    P,
    B,
}

impl FrameType {
    pub fn color(&self) -> Color32 {
        match self {
            FrameType::I => colors::NODE_I,
            FrameType::P => colors::NODE_P,
            FrameType::B => colors::NODE_B,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            FrameType::I => "I",
            FrameType::P => "P",
            FrameType::B => "B",
        }
    }
}

/// Reference entry
#[derive(Debug, Clone)]
pub struct RefEntry {
    pub list: u8,         // 0 for L0, 1 for L1
    pub ref_idx: usize,   // Index in ref list
    pub frame_idx: usize, // Referenced frame
    pub poc: i32,         // POC of referenced frame
    pub is_long_term: bool,
    pub weight: Option<f32>,
}

/// Graph node
#[derive(Debug, Clone)]
pub struct GraphNode {
    pub frame_idx: usize,
    pub frame_type: FrameType,
    pub poc: i32,
    pub refs: Vec<usize>,       // Frames this references
    pub dependents: Vec<usize>, // Frames that reference this
    pub position: egui::Pos2,   // Layout position
}

/// DPB entry
#[derive(Debug, Clone)]
pub struct DpbEntry {
    pub slot: usize,
    pub frame_idx: usize,
    pub poc: i32,
    pub is_long_term: bool,
    pub is_output: bool,
}

/// Risk level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
}

impl RiskLevel {
    pub fn color(&self) -> Color32 {
        match self {
            RiskLevel::Low => colors::RISK_LOW,
            RiskLevel::Medium => colors::RISK_MEDIUM,
            RiskLevel::High => colors::RISK_HIGH,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            RiskLevel::Low => "Low",
            RiskLevel::Medium => "Medium",
            RiskLevel::High => "High",
        }
    }
}

/// Risk indicator
#[derive(Debug, Clone)]
pub struct RiskIndicator {
    pub name: String,
    pub level: RiskLevel,
    pub value: f32,
    pub description: String,
}

/// Inspector tab
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InspectorTab {
    #[default]
    RefList,
    Dpb,
    Risk,
}

/// Reference workspace state
pub struct ReferenceWorkspace {
    /// Graph nodes
    nodes: Vec<GraphNode>,

    /// Selected frame index
    selected_frame: Option<usize>,

    /// Zoom factor
    zoom_factor: f32,

    /// Pan offset
    pan_offset: egui::Vec2,

    /// Show only refs/deps filter
    show_refs_only: bool,
    show_deps_only: bool,

    /// Active inspector tab
    inspector_tab: InspectorTab,

    /// Mock DPB state
    mock_dpb: Vec<DpbEntry>,

    /// Mock risk indicators
    mock_risks: Vec<RiskIndicator>,
}

impl ReferenceWorkspace {
    pub fn new() -> Self {
        // Generate mock graph data (typical GOP structure)
        let mut nodes = Vec::new();

        // Create nodes with positions
        for i in 0..30 {
            let frame_type = if i % 8 == 0 {
                FrameType::I
            } else if i % 4 == 0 {
                FrameType::P
            } else {
                FrameType::B
            };

            // Simple timeline layout
            let x = 50.0 + (i as f32 * 40.0);
            let y = match frame_type {
                FrameType::I => 100.0,
                FrameType::P => 150.0,
                FrameType::B => 200.0,
            };

            nodes.push(GraphNode {
                frame_idx: i,
                frame_type,
                poc: i as i32,
                refs: Vec::new(),
                dependents: Vec::new(),
                position: egui::pos2(x, y),
            });
        }

        // Add reference relationships
        for i in 1..nodes.len() {
            let frame_type = nodes[i].frame_type;
            match frame_type {
                FrameType::P => {
                    // P-frames reference previous I or P
                    for j in (0..i).rev() {
                        if nodes[j].frame_type != FrameType::B {
                            nodes[i].refs.push(j);
                            nodes[j].dependents.push(i);
                            break;
                        }
                    }
                }
                FrameType::B => {
                    // B-frames reference both directions
                    // Backward ref
                    for j in (0..i).rev() {
                        if nodes[j].frame_type != FrameType::B {
                            nodes[i].refs.push(j);
                            nodes[j].dependents.push(i);
                            break;
                        }
                    }
                    // Forward ref
                    for j in (i + 1)..nodes.len() {
                        if nodes[j].frame_type != FrameType::B {
                            nodes[i].refs.push(j);
                            nodes[j].dependents.push(i);
                            break;
                        }
                    }
                }
                FrameType::I => {}
            }
        }

        // Mock DPB
        let mock_dpb = vec![
            DpbEntry {
                slot: 0,
                frame_idx: 0,
                poc: 0,
                is_long_term: false,
                is_output: true,
            },
            DpbEntry {
                slot: 1,
                frame_idx: 4,
                poc: 4,
                is_long_term: false,
                is_output: true,
            },
            DpbEntry {
                slot: 2,
                frame_idx: 8,
                poc: 8,
                is_long_term: false,
                is_output: false,
            },
        ];

        // Mock risk indicators
        let mock_risks = vec![
            RiskIndicator {
                name: "Ref Chain Depth".to_string(),
                level: RiskLevel::Low,
                value: 3.0,
                description: "Maximum reference chain depth".to_string(),
            },
            RiskIndicator {
                name: "DPB Utilization".to_string(),
                level: RiskLevel::Medium,
                value: 75.0,
                description: "DPB slots in use (%)".to_string(),
            },
            RiskIndicator {
                name: "Missing Refs".to_string(),
                level: RiskLevel::Low,
                value: 0.0,
                description: "Frames with missing references".to_string(),
            },
            RiskIndicator {
                name: "Long-term Refs".to_string(),
                level: RiskLevel::Low,
                value: 1.0,
                description: "Active long-term references".to_string(),
            },
        ];

        Self {
            nodes,
            selected_frame: Some(5),
            zoom_factor: 1.0,
            pan_offset: egui::Vec2::ZERO,
            show_refs_only: false,
            show_deps_only: false,
            inspector_tab: InspectorTab::RefList,
            mock_dpb,
            mock_risks,
        }
    }

    /// Show the reference workspace
    pub fn show(&mut self, ui: &mut egui::Ui, _selection: &SelectionState) -> Option<Command> {
        let mut clicked_command = None;

        // Header toolbar
        ui.horizontal(|ui| {
            ui.heading("🔗 References");
            ui.separator();

            // Filter toggles
            if ui
                .selectable_label(self.show_refs_only, "Refs Only")
                .clicked()
            {
                self.show_refs_only = !self.show_refs_only;
                if self.show_refs_only {
                    self.show_deps_only = false;
                }
            }

            if ui
                .selectable_label(self.show_deps_only, "Deps Only")
                .clicked()
            {
                self.show_deps_only = !self.show_deps_only;
                if self.show_deps_only {
                    self.show_refs_only = false;
                }
            }

            ui.separator();

            // Zoom controls
            if ui.button("Fit").clicked() {
                self.zoom_factor = 1.0;
                self.pan_offset = egui::Vec2::ZERO;
            }

            ui.label(format!("Zoom: {:.0}%", self.zoom_factor * 100.0));
        });

        ui.separator();

        // Main content: Graph (65%) + Inspector (35%)
        ui.horizontal(|ui| {
            // Left: Graph canvas
            let graph_width = ui.available_width() * 0.65;
            ui.allocate_ui(egui::vec2(graph_width, ui.available_height()), |ui| {
                self.render_graph(ui, &mut clicked_command);
            });

            ui.separator();

            // Right: Inspector tabs
            ui.vertical(|ui| {
                self.render_inspector_tabs(ui);
                ui.separator();

                match self.inspector_tab {
                    InspectorTab::RefList => self.render_ref_list(ui, &mut clicked_command),
                    InspectorTab::Dpb => self.render_dpb_inspector(ui),
                    InspectorTab::Risk => self.render_risk_panel(ui),
                }
            });
        });

        clicked_command
    }

    /// Render the reference graph
    fn render_graph(&mut self, ui: &mut egui::Ui, clicked_command: &mut Option<Command>) {
        let plot_rect = ui.available_rect_before_wrap();

        // Background
        ui.painter().rect_filled(plot_rect, 4.0, colors::BACKGROUND);

        // Draw grid
        let grid_spacing = 50.0 * self.zoom_factor;
        let mut x = plot_rect.left() + self.pan_offset.x % grid_spacing;
        while x < plot_rect.right() {
            ui.painter().line_segment(
                [
                    egui::pos2(x, plot_rect.top()),
                    egui::pos2(x, plot_rect.bottom()),
                ],
                egui::Stroke::new(0.5, colors::GRID),
            );
            x += grid_spacing;
        }
        let mut y = plot_rect.top() + self.pan_offset.y % grid_spacing;
        while y < plot_rect.bottom() {
            ui.painter().line_segment(
                [
                    egui::pos2(plot_rect.left(), y),
                    egui::pos2(plot_rect.right(), y),
                ],
                egui::Stroke::new(0.5, colors::GRID),
            );
            y += grid_spacing;
        }

        // Collect visible nodes and transform positions
        let transform = |pos: egui::Pos2| -> egui::Pos2 {
            egui::pos2(
                plot_rect.left() + self.pan_offset.x + pos.x * self.zoom_factor,
                plot_rect.top() + self.pan_offset.y + pos.y * self.zoom_factor,
            )
        };

        // Draw edges first (under nodes)
        for node in &self.nodes {
            let start_pos = transform(node.position);

            // Skip if not visible
            if !plot_rect.contains(start_pos) {
                continue;
            }

            // Draw reference edges
            for &ref_idx in &node.refs {
                if ref_idx >= self.nodes.len() {
                    continue;
                }

                let end_pos = transform(self.nodes[ref_idx].position);

                // Determine edge color based on ref direction
                let edge_color = if ref_idx < node.frame_idx {
                    colors::EDGE_L0 // Backward ref
                } else {
                    colors::EDGE_L1 // Forward ref
                };

                // Highlight if selected
                let is_selected_ref = self.selected_frame == Some(node.frame_idx);
                let stroke_width = if is_selected_ref { 2.5 } else { 1.0 };

                ui.painter().line_segment(
                    [start_pos, end_pos],
                    egui::Stroke::new(stroke_width, edge_color),
                );

                // Arrow head
                let dir = (end_pos - start_pos).normalized();
                let arrow_size = 6.0 * self.zoom_factor;
                let arrow_pos = end_pos - dir * 15.0;
                let perp = egui::vec2(-dir.y, dir.x);

                let arrow = [
                    arrow_pos + dir * arrow_size,
                    arrow_pos + perp * arrow_size * 0.5,
                    arrow_pos - perp * arrow_size * 0.5,
                ];
                ui.painter().add(egui::Shape::convex_polygon(
                    arrow.to_vec(),
                    edge_color,
                    egui::Stroke::NONE,
                ));
            }
        }

        // Draw nodes
        let node_radius = 12.0 * self.zoom_factor;

        for node in &self.nodes {
            let pos = transform(node.position);

            // Skip if not visible
            if !plot_rect.contains(pos) {
                continue;
            }

            // Determine node color
            let mut node_color = node.frame_type.color();

            // Highlight based on selection
            if let Some(sel) = self.selected_frame {
                if node.frame_idx == sel {
                    node_color = colors::NODE_SELECTED;
                } else if self.nodes[sel].refs.contains(&node.frame_idx) {
                    // This node is referenced by selected
                    if !self.show_deps_only {
                        node_color = colors::NODE_REF_HIGHLIGHT;
                    }
                } else if self.nodes[sel].dependents.contains(&node.frame_idx) {
                    // This node references selected
                    if !self.show_refs_only {
                        node_color = colors::NODE_DEP_HIGHLIGHT;
                    }
                }
            }

            // Draw node circle
            ui.painter().circle_filled(pos, node_radius, node_color);
            ui.painter().circle_stroke(
                pos,
                node_radius,
                egui::Stroke::new(1.0, colors::TEXT_PRIMARY),
            );

            // Draw frame index label
            ui.painter().text(
                pos,
                egui::Align2::CENTER_CENTER,
                format!("{}", node.frame_idx),
                egui::FontId::proportional(9.0 * self.zoom_factor),
                colors::TEXT_PRIMARY,
            );
        }

        // Handle interactions
        let response = ui.allocate_rect(plot_rect, egui::Sense::click_and_drag());

        // Click to select node
        if response.clicked() {
            if let Some(pos) = response.interact_pointer_pos() {
                for node in &self.nodes {
                    let node_pos = transform(node.position);
                    if (pos - node_pos).length() < node_radius {
                        self.selected_frame = Some(node.frame_idx);
                        *clicked_command = Some(Command::SelectFrame {
                            stream: StreamId::A,
                            frame_key: FrameKey {
                                stream: StreamId::A,
                                frame_index: node.frame_idx,
                                pts: None,
                            },
                        });
                        break;
                    }
                }
            }
        }

        // Drag to pan
        if response.dragged() {
            self.pan_offset += response.drag_delta();
        }

        // Scroll to zoom
        if response.hovered() {
            ui.input(|i| {
                let scroll = i.smooth_scroll_delta.y;
                if scroll.abs() > 0.1 {
                    let zoom_delta = scroll * 0.002;
                    self.zoom_factor = (self.zoom_factor + zoom_delta).clamp(0.3, 3.0);
                }
            });
        }
    }

    /// Render inspector tab buttons
    fn render_inspector_tabs(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui
                .selectable_label(self.inspector_tab == InspectorTab::RefList, "Ref List")
                .clicked()
            {
                self.inspector_tab = InspectorTab::RefList;
            }
            if ui
                .selectable_label(self.inspector_tab == InspectorTab::Dpb, "DPB")
                .clicked()
            {
                self.inspector_tab = InspectorTab::Dpb;
            }
            if ui
                .selectable_label(self.inspector_tab == InspectorTab::Risk, "Risk")
                .clicked()
            {
                self.inspector_tab = InspectorTab::Risk;
            }
        });
    }

    /// Render reference list table
    fn render_ref_list(&self, ui: &mut egui::Ui, clicked_command: &mut Option<Command>) {
        ui.heading(
            egui::RichText::new("Reference List")
                .size(12.0)
                .color(colors::TEXT_PRIMARY),
        );

        if let Some(sel_idx) = self.selected_frame {
            if sel_idx < self.nodes.len() {
                let node = &self.nodes[sel_idx];

                ui.label(format!(
                    "Frame {} ({}) - POC {}",
                    node.frame_idx,
                    node.frame_type.label(),
                    node.poc
                ));

                ui.separator();

                // L0 references
                ui.label(
                    egui::RichText::new("L0 (Backward)")
                        .size(10.0)
                        .color(colors::EDGE_L0),
                );

                let l0_refs: Vec<_> = node.refs.iter().filter(|&&r| r < node.frame_idx).collect();

                if l0_refs.is_empty() {
                    ui.label("  (none)");
                } else {
                    for (idx, &&ref_frame) in l0_refs.iter().enumerate() {
                        let ref_node = &self.nodes[ref_frame];
                        let response = ui.horizontal(|ui| {
                            ui.label(format!(
                                "  [{}] Frame {} ({}) POC {}",
                                idx,
                                ref_frame,
                                ref_node.frame_type.label(),
                                ref_node.poc
                            ));
                        });

                        if response.response.interact(egui::Sense::click()).clicked() {
                            *clicked_command = Some(Command::SelectFrame {
                                stream: StreamId::A,
                                frame_key: FrameKey {
                                    stream: StreamId::A,
                                    frame_index: ref_frame,
                                    pts: None,
                                },
                            });
                        }
                    }
                }

                ui.separator();

                // L1 references
                ui.label(
                    egui::RichText::new("L1 (Forward)")
                        .size(10.0)
                        .color(colors::EDGE_L1),
                );

                let l1_refs: Vec<_> = node.refs.iter().filter(|&&r| r > node.frame_idx).collect();

                if l1_refs.is_empty() {
                    ui.label("  (none)");
                } else {
                    for (idx, &&ref_frame) in l1_refs.iter().enumerate() {
                        let ref_node = &self.nodes[ref_frame];
                        let response = ui.horizontal(|ui| {
                            ui.label(format!(
                                "  [{}] Frame {} ({}) POC {}",
                                idx,
                                ref_frame,
                                ref_node.frame_type.label(),
                                ref_node.poc
                            ));
                        });

                        if response.response.interact(egui::Sense::click()).clicked() {
                            *clicked_command = Some(Command::SelectFrame {
                                stream: StreamId::A,
                                frame_key: FrameKey {
                                    stream: StreamId::A,
                                    frame_index: ref_frame,
                                    pts: None,
                                },
                            });
                        }
                    }
                }

                ui.separator();

                // Dependents
                ui.label(
                    egui::RichText::new(format!("Dependents ({})", node.dependents.len()))
                        .size(10.0)
                        .color(colors::TEXT_SECONDARY),
                );

                for &dep_frame in node.dependents.iter().take(5) {
                    let dep_node = &self.nodes[dep_frame];
                    ui.label(format!(
                        "  Frame {} ({})",
                        dep_frame,
                        dep_node.frame_type.label()
                    ));
                }

                if node.dependents.len() > 5 {
                    ui.label(format!("  ... and {} more", node.dependents.len() - 5));
                }
            }
        } else {
            ui.label("Select a frame to see references");
        }
    }

    /// Render DPB inspector
    fn render_dpb_inspector(&self, ui: &mut egui::Ui) {
        ui.heading(
            egui::RichText::new("DPB State")
                .size(12.0)
                .color(colors::TEXT_PRIMARY),
        );

        ui.label(format!("Slots used: {}/16", self.mock_dpb.len()));
        ui.separator();

        egui::Grid::new("dpb_grid")
            .num_columns(5)
            .striped(true)
            .show(ui, |ui| {
                ui.label("Slot");
                ui.label("Frame");
                ui.label("POC");
                ui.label("LT");
                ui.label("Out");
                ui.end_row();

                for entry in &self.mock_dpb {
                    ui.label(format!("{}", entry.slot));
                    ui.label(format!("{}", entry.frame_idx));
                    ui.label(format!("{}", entry.poc));
                    ui.label(if entry.is_long_term { "✓" } else { "" });
                    ui.label(if entry.is_output { "✓" } else { "" });
                    ui.end_row();
                }
            });
    }

    /// Render risk panel
    fn render_risk_panel(&self, ui: &mut egui::Ui) {
        ui.heading(
            egui::RichText::new("Risk Indicators")
                .size(12.0)
                .color(colors::TEXT_PRIMARY),
        );

        for risk in &self.mock_risks {
            ui.horizontal(|ui| {
                // Risk level indicator
                let (rect, _) =
                    ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
                ui.painter()
                    .circle_filled(rect.center(), 5.0, risk.level.color());

                ui.label(&risk.name);
                ui.label(
                    egui::RichText::new(format!("{:.1}", risk.value)).color(risk.level.color()),
                );
            });

            ui.label(
                egui::RichText::new(&risk.description)
                    .size(9.0)
                    .color(colors::TEXT_SECONDARY),
            );

            ui.add_space(4.0);
        }
    }
}

impl Default for ReferenceWorkspace {
    fn default() -> Self {
        Self::new()
    }
}
