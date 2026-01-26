//! Stream Tree Panel - displays OBU/NAL unit hierarchy
//!
//! Features:
//! - Hierarchical display of NAL/OBU units
//! - Frame type filtering (I/P/B/All)
//! - Search by type or offset

use bitvue_core::command::OrderType;
use bitvue_core::{Command, SelectionState, StreamId, UnitNode};
use egui;

/// Frame type filter options
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameFilter {
    All,
    KeyOnly,     // I-frames / Key frames
    InterOnly,   // P/B frames
    FramesOnly,  // All frames (exclude headers)
    HeadersOnly, // Only headers (SPS, PPS, etc.)
}

impl FrameFilter {
    pub fn label(&self) -> &'static str {
        match self {
            FrameFilter::All => "All",
            FrameFilter::KeyOnly => "Key (I)",
            FrameFilter::InterOnly => "Inter (P/B)",
            FrameFilter::FramesOnly => "Frames",
            FrameFilter::HeadersOnly => "Headers",
        }
    }
}

pub struct StreamTreePanel {
    /// Current frame filter
    filter: FrameFilter,
    /// Search text
    search: String,
    /// Show only matching items
    filter_enabled: bool,
}

impl StreamTreePanel {
    pub fn new() -> Self {
        Self {
            filter: FrameFilter::All,
            search: String::new(),
            filter_enabled: false,
        }
    }

    /// Check if unit passes filter
    fn passes_filter(&self, unit: &UnitNode) -> bool {
        if !self.filter_enabled {
            return true;
        }

        // Search filter
        if !self.search.is_empty() {
            let search_lower = self.search.to_lowercase();
            let type_match = unit.unit_type.to_lowercase().contains(&search_lower);
            let offset_match = format!("{:x}", unit.offset).contains(&search_lower);
            if !type_match && !offset_match {
                return false;
            }
        }

        // Frame type filter
        match self.filter {
            FrameFilter::All => true,
            FrameFilter::KeyOnly => {
                unit.unit_type.contains("KEY")
                    || unit.unit_type.contains("INTRA")
                    || unit.unit_type.contains("IDR")
            }
            FrameFilter::InterOnly => {
                unit.frame_index.is_some()
                    && !unit.unit_type.contains("KEY")
                    && !unit.unit_type.contains("INTRA")
                    && !unit.unit_type.contains("IDR")
            }
            FrameFilter::FramesOnly => unit.frame_index.is_some(),
            FrameFilter::HeadersOnly => {
                unit.unit_type.contains("SEQUENCE")
                    || unit.unit_type.contains("SPS")
                    || unit.unit_type.contains("PPS")
                    || unit.unit_type.contains("VPS")
                    || unit.unit_type.contains("APS")
            }
        }
    }

    /// Render the stream tree panel
    /// Returns optional Command to emit
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        units: &[UnitNode],
        _selection: &SelectionState,
    ) -> Option<Command> {
        let mut command = None;

        ui.heading("Stream Tree");
        ui.separator();

        // Filter toolbar
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.filter_enabled, "Filter");

            if self.filter_enabled {
                ui.separator();

                // Frame type filter dropdown
                egui::ComboBox::from_id_salt("frame_filter")
                    .selected_text(self.filter.label())
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut self.filter,
                            FrameFilter::All,
                            FrameFilter::All.label(),
                        );
                        ui.selectable_value(
                            &mut self.filter,
                            FrameFilter::KeyOnly,
                            FrameFilter::KeyOnly.label(),
                        );
                        ui.selectable_value(
                            &mut self.filter,
                            FrameFilter::InterOnly,
                            FrameFilter::InterOnly.label(),
                        );
                        ui.selectable_value(
                            &mut self.filter,
                            FrameFilter::FramesOnly,
                            FrameFilter::FramesOnly.label(),
                        );
                        ui.selectable_value(
                            &mut self.filter,
                            FrameFilter::HeadersOnly,
                            FrameFilter::HeadersOnly.label(),
                        );
                    });

                // Search box
                ui.separator();
                ui.label("Search:");
                ui.add(egui::TextEdit::singleline(&mut self.search).desired_width(100.0));

                if ui.button("✕").clicked() {
                    self.search.clear();
                }
            }
        });

        ui.separator();

        // Count matching items
        let total_count = units.len();
        let filtered_count = if self.filter_enabled {
            units.iter().filter(|u| self.passes_filter(u)).count()
        } else {
            total_count
        };

        if self.filter_enabled && filtered_count != total_count {
            ui.label(
                egui::RichText::new(format!(
                    "Showing {} of {} units",
                    filtered_count, total_count
                ))
                .small()
                .color(egui::Color32::GRAY),
            );
            ui.separator();
        }

        egui::ScrollArea::vertical().show(ui, |ui| {
            for (idx, unit) in units.iter().enumerate() {
                if self.passes_filter(unit) {
                    if let Some(cmd) = self.render_unit_node(ui, unit, idx) {
                        command = Some(cmd);
                    }
                }
            }
        });

        command
    }

    fn render_unit_node(&self, ui: &mut egui::Ui, unit: &UnitNode, _idx: usize) -> Option<Command> {
        // Color code based on unit type
        let color = match unit.unit_type.as_str() {
            "SEQUENCE_HEADER" => egui::Color32::from_rgb(100, 200, 100),
            "FRAME" | "FRAME_HEADER" => egui::Color32::from_rgb(100, 150, 255),
            "TILE_GROUP" => egui::Color32::from_rgb(200, 150, 100),
            "TEMPORAL_DELIMITER" => egui::Color32::from_rgb(150, 150, 150),
            _ => egui::Color32::WHITE,
        };

        // Icon based on frame type
        let icon = if unit.frame_index.is_some() {
            "[F]"
        } else {
            match unit.unit_type.as_str() {
                "SEQUENCE_HEADER" => "[S]",
                "TEMPORAL_DELIMITER" => "[T]",
                "METADATA" => "[M]",
                "PADDING" => "[P]",
                _ => "*",
            }
        };

        // Build label
        let label = if let Some(frame_idx) = unit.frame_index {
            format!(
                "{} Frame #{} - {} @ 0x{:08X} ({} bytes)",
                icon, frame_idx, unit.unit_type, unit.offset, unit.size
            )
        } else {
            format!(
                "{} {} @ 0x{:08X} ({} bytes)",
                icon, unit.unit_type, unit.offset, unit.size
            )
        };

        let mut command = None;

        // Render as collapsing header if has children, otherwise as selectable label
        if !unit.children.is_empty() {
            egui::CollapsingHeader::new(egui::RichText::new(&label).color(color)).show(ui, |ui| {
                for (_child_idx, child) in unit.children.iter().enumerate() {
                    if let Some(cmd) = self.render_unit_node(ui, child, _child_idx) {
                        command = Some(cmd);
                    }
                }
            });
        } else {
            let response = ui.selectable_label(false, egui::RichText::new(&label).color(color));

            if response.clicked() {
                command = Some(Command::SelectUnit {
                    stream: StreamId::A,
                    unit_key: unit.key.clone(),
                });
            }

            // Context menu (right-click)
            // Per context_menus.json - StreamView scope
            response.context_menu(|ui| {
                // Compare in Display Order - always available
                if ui.button("Compare in Display Order").clicked() {
                    command = Some(Command::SetOrderType {
                        order_type: OrderType::Display,
                    });
                    ui.close_menu();
                }

                // Compare in Decode Order - always available
                if ui.button("Compare in Decode Order").clicked() {
                    command = Some(Command::SetOrderType {
                        order_type: OrderType::Decode,
                    });
                    ui.close_menu();
                }

                ui.separator();

                // Select this unit
                if ui.button("Select").clicked() {
                    command = Some(Command::SelectUnit {
                        stream: StreamId::A,
                        unit_key: unit.key.clone(),
                    });
                    ui.close_menu();
                }
            });

            // Show tooltip with details on hover
            response.on_hover_ui(|ui| {
                ui.label(format!("Type: {}", unit.unit_type));
                ui.label(format!("Offset: 0x{:08X}", unit.offset));
                ui.label(format!("Size: {} bytes", unit.size));
                if let Some(pts) = unit.pts {
                    ui.label(format!("PTS: {}", pts));
                }
            });
        }

        command
    }
}

impl Default for StreamTreePanel {
    fn default() -> Self {
        Self::new()
    }
}
