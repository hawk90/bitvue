//! Syntax Detail Panel - VQAnalyzer-style tabbed syntax display
//!
//! Features:
//! - Tabbed interface: NAL | SPS | PPS | Slice | CU | Refs | Stats
//! - Syntax tree rendering with collapsible nodes
//! - Bit range highlighting integration
//! - Reference list display (L0/L1)
//! - Frame statistics (QP distribution, mode counts)

use bitvue_core::{Command, SelectionState, SyntaxModel};
use egui::{self, Color32, RichText};

/// Available tabs in the syntax detail panel
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SyntaxTab {
    #[default]
    /// NAL unit list and details
    Nal,
    /// Sequence Parameter Set
    Sps,
    /// Picture Parameter Set
    Pps,
    /// Slice header information
    Slice,
    /// Coding Unit details
    Cu,
    /// Reference frame lists (L0/L1)
    Refs,
    /// Frame/stream statistics
    Stats,
}

impl SyntaxTab {
    /// Get tab label for display
    pub fn label(&self) -> &'static str {
        match self {
            Self::Nal => "NAL",
            Self::Sps => "SPS",
            Self::Pps => "PPS",
            Self::Slice => "Slice",
            Self::Cu => "CU",
            Self::Refs => "Refs",
            Self::Stats => "Stats",
        }
    }

    /// All available tabs in display order
    pub fn all() -> &'static [SyntaxTab] {
        &[
            Self::Nal,
            Self::Sps,
            Self::Pps,
            Self::Slice,
            Self::Cu,
            Self::Refs,
            Self::Stats,
        ]
    }
}

/// Syntax Detail Panel with VQAnalyzer-style tabs
pub struct SyntaxDetailPanel {
    /// Currently selected tab
    current_tab: SyntaxTab,
}

impl SyntaxDetailPanel {
    pub fn new() -> Self {
        Self {
            current_tab: SyntaxTab::Nal,
        }
    }

    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        syntax: Option<&SyntaxModel>,
        selection: &SelectionState,
    ) -> Option<Command> {
        let mut command = None;

        // VQAnalyzer-style tab bar
        ui.horizontal(|ui| {
            for tab in SyntaxTab::all() {
                let is_selected = self.current_tab == *tab;
                let label = if is_selected {
                    RichText::new(tab.label()).strong().color(Color32::WHITE)
                } else {
                    RichText::new(tab.label()).color(Color32::GRAY)
                };

                if ui.selectable_label(is_selected, label).clicked() {
                    self.current_tab = *tab;
                }
            }
        });
        ui.separator();

        // Tab content
        match self.current_tab {
            SyntaxTab::Nal => {
                if let Some(cmd) = self.show_nal_tab(ui, syntax, selection) {
                    command = Some(cmd);
                }
            }
            SyntaxTab::Sps => {
                self.show_sps_tab(ui, syntax);
            }
            SyntaxTab::Pps => {
                self.show_pps_tab(ui, syntax);
            }
            SyntaxTab::Slice => {
                self.show_slice_tab(ui, syntax);
            }
            SyntaxTab::Cu => {
                if let Some(cmd) = self.show_cu_tab(ui, syntax, selection) {
                    command = Some(cmd);
                }
            }
            SyntaxTab::Refs => {
                self.show_refs_tab(ui, selection);
            }
            SyntaxTab::Stats => {
                self.show_stats_tab(ui, syntax, selection);
            }
        }

        command
    }

    /// NAL tab - shows NAL unit list and full syntax tree
    fn show_nal_tab(
        &self,
        ui: &mut egui::Ui,
        syntax: Option<&SyntaxModel>,
        selection: &SelectionState,
    ) -> Option<Command> {
        let mut command = None;

        if let Some(syntax) = syntax {
            // Show which unit is selected
            ui.horizontal(|ui| {
                ui.label(RichText::new("Unit:").color(Color32::GRAY));
                ui.label(RichText::new(&syntax.unit_key).color(Color32::from_rgb(100, 180, 255)));
            });

            ui.label(
                RichText::new(format!("Nodes: {}", syntax.nodes.len()))
                    .small()
                    .color(Color32::GRAY),
            );
            ui.separator();

            // Scrollable syntax tree - render from root
            egui::ScrollArea::vertical().show(ui, |ui| {
                if let Some(root_node) = syntax.nodes.get(&syntax.root_id) {
                    if let Some(cmd) = Self::render_syntax_node(ui, syntax, root_node, 0, selection)
                    {
                        command = Some(cmd);
                    }
                } else {
                    ui.label("Root node not found");
                }
            });
        } else if selection.unit.is_some() {
            ui.label("Parsing syntax...");
        } else {
            ui.colored_label(Color32::GRAY, "Select a unit to see NAL syntax");
        }

        command
    }

    /// SPS tab - Sequence Parameter Set
    fn show_sps_tab(&self, ui: &mut egui::Ui, syntax: Option<&SyntaxModel>) {
        ui.label(RichText::new("Sequence Parameter Set").strong());
        ui.separator();

        if let Some(syntax) = syntax {
            // Look for SPS-related nodes in syntax tree
            let sps_fields = Self::extract_sps_fields(syntax);
            if sps_fields.is_empty() {
                ui.colored_label(Color32::GRAY, "No SPS data in current unit");
                ui.label("Select a sequence header unit to see SPS details");
            } else {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    egui::Grid::new("sps_grid")
                        .num_columns(2)
                        .spacing([8.0, 4.0])
                        .striped(true)
                        .show(ui, |ui| {
                            for (name, value) in &sps_fields {
                                ui.label(RichText::new(name).color(Color32::GRAY));
                                ui.label(RichText::new(value).color(Color32::WHITE));
                                ui.end_row();
                            }
                        });
                });
            }
        } else {
            ui.colored_label(Color32::GRAY, "Select a unit to see SPS");
        }
    }

    /// PPS tab - Picture Parameter Set
    fn show_pps_tab(&self, ui: &mut egui::Ui, syntax: Option<&SyntaxModel>) {
        ui.label(RichText::new("Picture Parameter Set").strong());
        ui.separator();

        if let Some(syntax) = syntax {
            let pps_fields = Self::extract_pps_fields(syntax);
            if pps_fields.is_empty() {
                ui.colored_label(Color32::GRAY, "No PPS data in current unit");
            } else {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    egui::Grid::new("pps_grid")
                        .num_columns(2)
                        .spacing([8.0, 4.0])
                        .striped(true)
                        .show(ui, |ui| {
                            for (name, value) in &pps_fields {
                                ui.label(RichText::new(name).color(Color32::GRAY));
                                ui.label(RichText::new(value).color(Color32::WHITE));
                                ui.end_row();
                            }
                        });
                });
            }
        } else {
            ui.colored_label(Color32::GRAY, "Select a unit to see PPS");
        }
    }

    /// Slice tab - Slice header information
    fn show_slice_tab(&self, ui: &mut egui::Ui, syntax: Option<&SyntaxModel>) {
        ui.label(RichText::new("Slice Header").strong());
        ui.separator();

        if let Some(syntax) = syntax {
            let slice_fields = Self::extract_slice_fields(syntax);
            if slice_fields.is_empty() {
                ui.colored_label(Color32::GRAY, "No slice data in current unit");
            } else {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    egui::Grid::new("slice_grid")
                        .num_columns(2)
                        .spacing([8.0, 4.0])
                        .striped(true)
                        .show(ui, |ui| {
                            for (name, value) in &slice_fields {
                                ui.label(RichText::new(name).color(Color32::GRAY));
                                ui.label(RichText::new(value).color(Color32::WHITE));
                                ui.end_row();
                            }
                        });
                });
            }
        } else {
            ui.colored_label(Color32::GRAY, "Select a frame unit to see slice info");
        }
    }

    /// CU tab - Coding Unit details (same as NAL tree for now)
    fn show_cu_tab(
        &self,
        ui: &mut egui::Ui,
        syntax: Option<&SyntaxModel>,
        selection: &SelectionState,
    ) -> Option<Command> {
        ui.label(RichText::new("Coding Unit").strong());
        ui.separator();

        if syntax.is_some() {
            ui.colored_label(
                Color32::GRAY,
                "Select a block in the player to see CU details",
            );
            ui.separator();

            // Show selected block info if available
            if selection.temporal.is_some() {
                ui.label("Block selection available - CU details shown in Selection Info panel");
            }
        } else {
            ui.colored_label(Color32::GRAY, "Select a unit to see CU details");
        }
        None
    }

    /// Refs tab - Reference frame lists (L0/L1)
    fn show_refs_tab(&self, ui: &mut egui::Ui, selection: &SelectionState) {
        ui.label(RichText::new("Reference Lists").strong());
        ui.separator();

        // VQAnalyzer shows L0 and L1 reference lists with POC values
        ui.collapsing("L0 (List 0 - Forward)", |ui| {
            if selection.temporal.is_some() {
                // Mock data - would come from decoder in real implementation
                egui::Grid::new("l0_refs")
                    .num_columns(3)
                    .spacing([8.0, 4.0])
                    .show(ui, |ui| {
                        ui.label(RichText::new("Idx").color(Color32::GRAY));
                        ui.label(RichText::new("POC").color(Color32::GRAY));
                        ui.label(RichText::new("Type").color(Color32::GRAY));
                        ui.end_row();

                        // Placeholder entries
                        ui.label("0");
                        ui.label(RichText::new("N/A").color(Color32::from_rgb(100, 180, 100)));
                        ui.label("Short-term");
                        ui.end_row();
                    });
            } else {
                ui.colored_label(Color32::GRAY, "Select a frame to see references");
            }
        });

        ui.add_space(4.0);

        ui.collapsing("L1 (List 1 - Backward)", |ui| {
            if selection.temporal.is_some() {
                egui::Grid::new("l1_refs")
                    .num_columns(3)
                    .spacing([8.0, 4.0])
                    .show(ui, |ui| {
                        ui.label(RichText::new("Idx").color(Color32::GRAY));
                        ui.label(RichText::new("POC").color(Color32::GRAY));
                        ui.label(RichText::new("Type").color(Color32::GRAY));
                        ui.end_row();

                        ui.label("0");
                        ui.label(RichText::new("N/A").color(Color32::from_rgb(100, 140, 220)));
                        ui.label("Short-term");
                        ui.end_row();
                    });
            } else {
                ui.colored_label(Color32::GRAY, "Select a frame to see references");
            }
        });
    }

    /// Stats tab - Frame/stream statistics
    fn show_stats_tab(
        &self,
        ui: &mut egui::Ui,
        syntax: Option<&SyntaxModel>,
        selection: &SelectionState,
    ) {
        ui.label(RichText::new("Statistics").strong());
        ui.separator();

        // QP Distribution
        ui.collapsing("QP Distribution", |ui| {
            ui.label(
                RichText::new("Frame QP statistics")
                    .small()
                    .color(Color32::GRAY),
            );
            egui::Grid::new("qp_stats")
                .num_columns(2)
                .spacing([8.0, 4.0])
                .show(ui, |ui| {
                    ui.label(RichText::new("Min QP:").color(Color32::GRAY));
                    ui.label("N/A");
                    ui.end_row();

                    ui.label(RichText::new("Max QP:").color(Color32::GRAY));
                    ui.label("N/A");
                    ui.end_row();

                    ui.label(RichText::new("Avg QP:").color(Color32::GRAY));
                    ui.label("N/A");
                    ui.end_row();
                });
        });

        ui.add_space(4.0);

        // Prediction Mode Distribution
        ui.collapsing("Prediction Modes", |ui| {
            ui.label(
                RichText::new("Mode usage statistics")
                    .small()
                    .color(Color32::GRAY),
            );
            egui::Grid::new("mode_stats")
                .num_columns(2)
                .spacing([8.0, 4.0])
                .show(ui, |ui| {
                    ui.label(RichText::new("Intra:").color(Color32::GRAY));
                    ui.label("N/A");
                    ui.end_row();

                    ui.label(RichText::new("Skip:").color(Color32::GRAY));
                    ui.label("N/A");
                    ui.end_row();

                    ui.label(RichText::new("Merge:").color(Color32::GRAY));
                    ui.label("N/A");
                    ui.end_row();

                    ui.label(RichText::new("AMVP:").color(Color32::GRAY));
                    ui.label("N/A");
                    ui.end_row();
                });
        });

        ui.add_space(4.0);

        // Syntax Stats
        if let Some(syntax) = syntax {
            ui.collapsing("Syntax Statistics", |ui| {
                egui::Grid::new("syntax_stats")
                    .num_columns(2)
                    .spacing([8.0, 4.0])
                    .show(ui, |ui| {
                        ui.label(RichText::new("Total Nodes:").color(Color32::GRAY));
                        ui.label(format!("{}", syntax.nodes.len()));
                        ui.end_row();

                        ui.label(RichText::new("Root ID:").color(Color32::GRAY));
                        ui.label(&syntax.root_id);
                        ui.end_row();
                    });
            });
        }

        // Frame Info
        if selection.temporal.is_some() {
            ui.add_space(4.0);
            ui.collapsing("Frame Statistics", |ui| {
                ui.label(
                    RichText::new("Frame-level analysis")
                        .small()
                        .color(Color32::GRAY),
                );
                // Would show frame size, bit count, etc.
                ui.label("Select block for detailed stats");
            });
        }
    }

    /// Extract SPS-related fields from syntax model
    fn extract_sps_fields(syntax: &SyntaxModel) -> Vec<(String, String)> {
        let mut fields = Vec::new();

        // Walk syntax tree looking for SPS-related fields
        for node in syntax.nodes.values() {
            let name_lower = node.field_name.to_lowercase();
            if name_lower.contains("width")
                || name_lower.contains("height")
                || name_lower.contains("profile")
                || name_lower.contains("level")
                || name_lower.contains("chroma")
                || name_lower.contains("bit_depth")
                || name_lower.contains("max_frame")
            {
                if let Some(ref value) = node.value {
                    fields.push((node.field_name.clone(), value.clone()));
                }
            }
        }

        fields
    }

    /// Extract PPS-related fields from syntax model
    fn extract_pps_fields(syntax: &SyntaxModel) -> Vec<(String, String)> {
        let mut fields = Vec::new();

        for node in syntax.nodes.values() {
            let name_lower = node.field_name.to_lowercase();
            if name_lower.contains("pps")
                || name_lower.contains("pic_init")
                || name_lower.contains("entropy")
                || name_lower.contains("weighted")
                || name_lower.contains("deblock")
            {
                if let Some(ref value) = node.value {
                    fields.push((node.field_name.clone(), value.clone()));
                }
            }
        }

        fields
    }

    /// Extract slice-related fields from syntax model
    fn extract_slice_fields(syntax: &SyntaxModel) -> Vec<(String, String)> {
        let mut fields = Vec::new();

        for node in syntax.nodes.values() {
            let name_lower = node.field_name.to_lowercase();
            if name_lower.contains("slice")
                || name_lower.contains("frame_type")
                || name_lower.contains("qp")
                || name_lower.contains("temporal")
                || name_lower.contains("ref_")
            {
                if let Some(ref value) = node.value {
                    fields.push((node.field_name.clone(), value.clone()));
                }
            }
        }

        fields
    }

    fn render_syntax_node(
        ui: &mut egui::Ui,
        model: &SyntaxModel,
        node: &bitvue_core::SyntaxNode,
        indent_level: usize,
        selection: &SelectionState,
    ) -> Option<Command> {
        let mut command = None;
        let indent = "  ".repeat(indent_level);

        // Icon based on whether it's a container or leaf
        let icon = if node.children.is_empty() {
            "▪"
        } else {
            "📦"
        };

        // Build label
        let label = if let Some(ref value) = node.value {
            format!("{}{} {} = {}", indent, icon, node.field_name, value)
        } else {
            format!("{}{} {}", indent, icon, node.field_name)
        };

        // Highlight if selected
        let is_selected = selection
            .syntax_node
            .as_ref()
            .map(|id| id == &node.node_id)
            .unwrap_or(false);

        let text_color = if is_selected {
            egui::Color32::BLACK // Black text on bright background
        } else {
            egui::Color32::from_rgb(160, 160, 160) // Darker gray for better contrast
        };

        if !node.children.is_empty() {
            // Container node - use collapsing header
            let id = egui::Id::new(&node.node_id);

            let header = egui::CollapsingHeader::new(egui::RichText::new(&label).color(text_color))
                .id_salt(id)
                .default_open(indent_level < 3); // Open first 3 levels by default

            let response = header.show(ui, |ui| {
                for child_id in &node.children {
                    if let Some(child_node) = model.nodes.get(child_id) {
                        if let Some(cmd) = Self::render_syntax_node(
                            ui,
                            model,
                            child_node,
                            indent_level + 1,
                            selection,
                        ) {
                            command = Some(cmd);
                        }
                    }
                }
            });

            // Make container clickable too
            if response.header_response.clicked() {
                command = Some(Command::SelectSyntax {
                    stream: selection.stream_id,
                    node_id: node.node_id.clone(),
                    bit_range: node.bit_range,
                });
            }
        } else {
            // Leaf node - clickable label
            if is_selected {
                // Selected: highlight with background
                let response = ui.add(
                    egui::Label::new(
                        egui::RichText::new(&label)
                            .color(egui::Color32::BLACK)
                            .background_color(egui::Color32::from_rgb(255, 180, 80)),
                    )
                    .sense(egui::Sense::click()),
                );

                if response.clicked() {
                    command = Some(Command::SelectSyntax {
                        stream: selection.stream_id,
                        node_id: node.node_id.clone(),
                        bit_range: node.bit_range,
                    });
                }

                response.on_hover_text(format!(
                    "Bits {}-{} ({} bits)\nClick to highlight in Hex/Bit View",
                    node.bit_range.start_bit,
                    node.bit_range.end_bit,
                    node.bit_range.size_bits()
                ));
            } else {
                // Unselected: normal clickable label
                let response = ui.add(
                    egui::Label::new(egui::RichText::new(&label).color(text_color))
                        .sense(egui::Sense::click()),
                );

                if response.clicked() {
                    command = Some(Command::SelectSyntax {
                        stream: selection.stream_id,
                        node_id: node.node_id.clone(),
                        bit_range: node.bit_range,
                    });
                }

                response.on_hover_text(format!(
                    "Bits {}-{} ({} bits)\nClick to highlight in Hex/Bit View",
                    node.bit_range.start_bit,
                    node.bit_range.end_bit,
                    node.bit_range.size_bits()
                ));
            }
        }

        command
    }
}

impl Default for SyntaxDetailPanel {
    fn default() -> Self {
        Self::new()
    }
}
