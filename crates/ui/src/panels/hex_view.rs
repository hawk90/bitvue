//! Hex View Panel - hex dump of bitstream bytes with selection highlighting
//!
//! VQAnalyzer parity: Tabbed interface with Unit Info, Hex View, and DPB Info tabs.

use bitvue_core::{ByteCache, Command, SelectionState};
use egui;

const BYTES_PER_LINE: usize = 16;

/// Tab selection for hex view panel (VQAnalyzer parity)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HexViewTab {
    /// Unit metadata and info
    UnitInfo,
    /// Traditional hex dump view
    #[default]
    HexDump,
    /// Decoded Picture Buffer state
    DpbInfo,
}

impl HexViewTab {
    pub fn label(&self) -> &'static str {
        match self {
            HexViewTab::UnitInfo => "Unit Info",
            HexViewTab::HexDump => "Hex",
            HexViewTab::DpbInfo => "DPB Info",
        }
    }
}

pub struct HexViewPanel {
    /// Scroll to this offset (for TC01: Tree→Hex sync)
    scroll_to_offset: Option<usize>,
    /// Current tab selection (VQAnalyzer parity)
    current_tab: HexViewTab,
}

impl HexViewPanel {
    pub fn new() -> Self {
        Self {
            scroll_to_offset: None,
            current_tab: HexViewTab::default(),
        }
    }

    /// Show the hex view panel
    /// Returns optional Command if user clicks a byte (TC04: Hex→Tree sync)
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        byte_cache: Option<&ByteCache>,
        units: Option<&[bitvue_core::UnitNode]>,
        selection: &SelectionState,
    ) -> Option<Command> {
        let mut clicked_command = None;

        ui.heading("🔢 Hex View");

        // Tab bar (VQAnalyzer parity)
        ui.horizontal(|ui| {
            for tab in [HexViewTab::UnitInfo, HexViewTab::HexDump, HexViewTab::DpbInfo] {
                if ui
                    .selectable_label(self.current_tab == tab, tab.label())
                    .clicked()
                {
                    self.current_tab = tab;
                }
            }
        });

        ui.separator();

        // Render content based on current tab
        match self.current_tab {
            HexViewTab::UnitInfo => {
                clicked_command = self.render_unit_info_tab(ui, units, selection);
            }
            HexViewTab::HexDump => {
                clicked_command = self.render_hex_dump_tab(ui, byte_cache, units, selection);
            }
            HexViewTab::DpbInfo => {
                self.render_dpb_info_tab(ui, units, selection);
            }
        }

        clicked_command
    }

    /// Render Unit Info tab (VQAnalyzer parity)
    fn render_unit_info_tab(
        &mut self,
        ui: &mut egui::Ui,
        units: Option<&[bitvue_core::UnitNode]>,
        selection: &SelectionState,
    ) -> Option<Command> {
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if let Some(unit_key) = &selection.unit {
                    // Find the selected unit
                    if let Some(unit) = units.and_then(|u| Self::find_unit_by_offset(u, unit_key.offset)) {
                        // Unit type header
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("Unit Type:").strong());
                            ui.label(egui::RichText::new(&unit.unit_type).color(egui::Color32::from_rgb(100, 180, 255)));
                        });

                        ui.add_space(4.0);

                        // Basic info grid
                        egui::Grid::new("unit_info_grid")
                            .num_columns(2)
                            .spacing([20.0, 4.0])
                            .show(ui, |ui| {
                                ui.label("Offset:");
                                ui.label(format!("0x{:08X} ({})", unit.offset, unit.offset));
                                ui.end_row();

                                ui.label("Size:");
                                ui.label(format!("{} bytes", unit.size));
                                ui.end_row();

                                if let Some(frame_idx) = unit.frame_index {
                                    ui.label("Frame Index:");
                                    ui.label(format!("{}", frame_idx));
                                    ui.end_row();
                                }

                                if let Some(qp) = unit.qp_avg {
                                    ui.label("QP (avg):");
                                    ui.label(format!("{}", qp));
                                    ui.end_row();
                                }
                            });

                        // Show children count if any
                        if !unit.children.is_empty() {
                            ui.add_space(8.0);
                            ui.separator();
                            ui.label(egui::RichText::new(format!("Children: {}", unit.children.len())).small());
                        }
                    } else {
                        ui.label(egui::RichText::new("Unit not found").color(egui::Color32::GRAY));
                    }
                } else {
                    ui.centered_and_justified(|ui| {
                        ui.label(egui::RichText::new("No unit selected\nSelect a unit from the stream tree").color(egui::Color32::GRAY));
                    });
                }
            });
        None
    }

    /// Render DPB Info tab (VQAnalyzer parity)
    fn render_dpb_info_tab(
        &mut self,
        ui: &mut egui::Ui,
        units: Option<&[bitvue_core::UnitNode]>,
        selection: &SelectionState,
    ) {
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Decoded Picture Buffer").strong());
                ui.add_space(4.0);

                // Get current frame info
                let current_frame = selection
                    .temporal
                    .as_ref()
                    .map(|t| t.frame_index())
                    .unwrap_or(0);

                // Show DPB slots (mock data for now - would come from parser)
                egui::Grid::new("dpb_grid")
                    .num_columns(5)
                    .spacing([10.0, 4.0])
                    .striped(true)
                    .show(ui, |ui| {
                        // Header
                        ui.label(egui::RichText::new("Slot").strong());
                        ui.label(egui::RichText::new("POC").strong());
                        ui.label(egui::RichText::new("Type").strong());
                        ui.label(egui::RichText::new("Ref").strong());
                        ui.label(egui::RichText::new("Status").strong());
                        ui.end_row();

                        // Mock DPB slots based on current frame
                        let dpb_size = 8; // Typical DPB size
                        for slot in 0..dpb_size {
                            let poc = if slot < 4 {
                                current_frame.saturating_sub(4 - slot)
                            } else {
                                0
                            };
                            let is_ref = slot < 4;
                            let frame_type = match slot % 3 {
                                0 => "I",
                                1 => "P",
                                _ => "B",
                            };

                            ui.label(format!("{}", slot));
                            ui.label(if is_ref { format!("{}", poc) } else { "-".to_string() });
                            ui.label(if is_ref { frame_type } else { "-" });
                            ui.label(if is_ref && slot < 2 { "L0" } else if is_ref { "L1" } else { "-" });
                            ui.label(if is_ref {
                                egui::RichText::new("Used").color(egui::Color32::from_rgb(100, 200, 100))
                            } else {
                                egui::RichText::new("Empty").color(egui::Color32::GRAY)
                            });
                            ui.end_row();
                        }
                    });

                ui.add_space(8.0);
                ui.separator();
                ui.label(egui::RichText::new("Note: DPB state from parser (mock data shown)").small().color(egui::Color32::GRAY));

                // Suppress unused warning
                let _ = units;
            });
    }

    /// Render Hex Dump tab
    fn render_hex_dump_tab(
        &mut self,
        ui: &mut egui::Ui,
        byte_cache: Option<&ByteCache>,
        units: Option<&[bitvue_core::UnitNode]>,
        selection: &SelectionState,
    ) -> Option<Command> {
        let mut clicked_command = None;

        // Get selected unit's byte range
        let selected_range = selection
            .unit
            .as_ref()
            .map(|unit_key| (unit_key.offset as usize, unit_key.size));

        // Get selected bit range (for syntax node selection - Tri-sync)
        let bit_range_bytes = selection.bit_range.as_ref().map(|br| {
            let start_byte = (br.start_bit / 8) as usize;
            let end_byte = br.end_bit.div_ceil(8) as usize;
            (start_byte, end_byte - start_byte)
        });

        // Sync scroll position to selected unit or bit range (TC01: Tree→Hex, Tri-sync)
        let scroll_target = bit_range_bytes.or(selected_range);
        if let Some((offset, _size)) = scroll_target {
            if self.scroll_to_offset != Some(offset) {
                self.scroll_to_offset = Some(offset);
            }
        }

        if let Some(cache) = byte_cache {
            let total_bytes = cache.len() as usize;

            let scroll_response = egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    // Calculate line range to display
                    let total_lines = total_bytes.div_ceil(BYTES_PER_LINE);

                    // Render hex dump
                    for line_idx in 0..total_lines {
                        let offset = line_idx * BYTES_PER_LINE;
                        let end = (offset + BYTES_PER_LINE).min(total_bytes);

                        // TC01: Scroll to selected offset
                        if let Some(scroll_offset) = self.scroll_to_offset {
                            if offset <= scroll_offset && scroll_offset < end {
                                ui.scroll_to_cursor(Some(egui::Align::Center));
                                self.scroll_to_offset = None; // Clear after scrolling
                            }
                        }

                        if let Ok(line_bytes) = cache.read_range(offset as u64, end - offset) {
                            if let Some(cmd) = self.render_hex_line(
                                ui,
                                offset,
                                line_bytes,
                                selected_range,
                                bit_range_bytes,
                                units,
                            ) {
                                clicked_command = Some(cmd);
                            }
                        }
                    }
                });

            // Context menu (right-click)
            // Per context_menus.json - HexView scope
            let has_byte_range = selection.bit_range.is_some() || selection.unit.is_some();
            scroll_response.inner_rect.is_positive();
            ui.interact(
                scroll_response.inner_rect,
                ui.id().with("hex_ctx"),
                egui::Sense::click(),
            )
            .context_menu(|ui| {
                // Copy Bytes - guarded by has_byte_range
                if ui
                    .add_enabled(has_byte_range, egui::Button::new("Copy Bytes (Hex)"))
                    .on_disabled_hover_text("No byte range selected")
                    .clicked()
                {
                    // Get byte range from bit_range or unit selection
                    let byte_range = bit_range_bytes.or(selected_range);
                    if let Some((offset, size)) = byte_range {
                        if let Ok(bytes) = cache.read_range(offset as u64, size) {
                            // Format as hex string
                            let hex_str: String = bytes
                                .iter()
                                .map(|b| format!("{:02X}", b))
                                .collect::<Vec<_>>()
                                .join(" ");
                            ui.output_mut(|o| o.copied_text = hex_str);
                        }
                    }
                    ui.close_menu();
                }

                // Copy Bytes as raw binary
                if ui
                    .add_enabled(has_byte_range, egui::Button::new("Copy Bytes (Raw)"))
                    .on_disabled_hover_text("No byte range selected")
                    .clicked()
                {
                    let byte_range = bit_range_bytes.or(selected_range);
                    if let Some((offset, size)) = byte_range {
                        if let Ok(bytes) = cache.read_range(offset as u64, size) {
                            // Copy as raw bytes (ASCII representation)
                            let raw_str = String::from_utf8_lossy(bytes).to_string();
                            ui.output_mut(|o| o.copied_text = raw_str);
                        }
                    }
                    ui.close_menu();
                }

                ui.separator();

                // Export Evidence Bundle - always available
                if ui.button("Export Evidence Bundle...").clicked() {
                    clicked_command = Some(Command::ExportEvidenceBundle {
                        stream: bitvue_core::StreamId::A,
                        path: std::path::PathBuf::from("."),
                    });
                    ui.close_menu();
                }
            });
        } else {
            ui.centered_and_justified(|ui| {
                ui.label(egui::RichText::new("No file loaded").color(egui::Color32::GRAY));
            });
        }

        clicked_command
    }

    fn render_hex_line(
        &self,
        ui: &mut egui::Ui,
        offset: usize,
        bytes: &[u8],
        selected_range: Option<(usize, usize)>,
        bit_range_bytes: Option<(usize, usize)>,
        _units: Option<&[bitvue_core::UnitNode]>,
    ) -> Option<Command> {
        let mut clicked_command = None;

        ui.horizontal(|ui| {
            // Offset column
            ui.label(
                egui::RichText::new(format!("{:08X}", offset))
                    .monospace()
                    .color(egui::Color32::GRAY),
            );

            ui.separator();

            // Check for start codes in this line (for highlighting)
            // NAL start codes: 0x000001 (3 bytes) or 0x00000001 (4 bytes)
            let mut start_code_positions = Vec::new();
            for i in 0..bytes.len().saturating_sub(2) {
                if bytes[i] == 0x00 && bytes[i + 1] == 0x00 {
                    if bytes[i + 2] == 0x01 {
                        // 3-byte start code: 00 00 01
                        start_code_positions.extend([i, i + 1, i + 2]);
                    } else if i + 3 < bytes.len() && bytes[i + 2] == 0x00 && bytes[i + 3] == 0x01 {
                        // 4-byte start code: 00 00 00 01
                        start_code_positions.extend([i, i + 1, i + 2, i + 3]);
                    }
                }
            }

            // Hex bytes column (with selection highlighting)
            for (i, &byte) in bytes.iter().enumerate() {
                let byte_offset = offset + i;

                // Check if byte is in selected unit range
                let is_unit_selected = selected_range
                    .map(|(sel_offset, sel_size)| {
                        byte_offset >= sel_offset && byte_offset < sel_offset + sel_size
                    })
                    .unwrap_or(false);

                // Check if byte is in bit range (Tri-sync: Syntax → Hex)
                let is_bit_range_selected = bit_range_bytes
                    .map(|(sel_offset, sel_size)| {
                        byte_offset >= sel_offset && byte_offset < sel_offset + sel_size
                    })
                    .unwrap_or(false);

                // Check if byte is part of a start code
                let is_start_code = start_code_positions.contains(&i);

                // Determine color with priority: bit range > start code > unit selection > default
                let color = if is_bit_range_selected {
                    egui::Color32::from_rgb(255, 180, 80) // Bright orange for syntax selection
                } else if is_start_code {
                    egui::Color32::from_rgb(255, 100, 100) // Red for start codes (VQAnalyzer parity)
                } else if is_unit_selected {
                    egui::Color32::from_rgb(200, 200, 120) // Dim yellow for unit selection
                } else {
                    egui::Color32::from_rgb(220, 220, 220) // Bright light gray for readability
                };

                let response = ui.label(
                    egui::RichText::new(format!("{:02X}", byte))
                        .monospace()
                        .color(color),
                );

                // TC04: Hex→Tree - clicking a byte selects bit range (Tri-sync)
                if response.clicked() {
                    // Convert byte offset to bit range (entire byte)
                    let bit_start = (byte_offset * 8) as u64;
                    let bit_end = bit_start + 8;

                    clicked_command = Some(Command::SelectBitRange {
                        stream: bitvue_core::StreamId::A,
                        bit_range: bitvue_core::BitRange {
                            start_bit: bit_start,
                            end_bit: bit_end,
                        },
                    });
                }

                // Add spacing every 8 bytes
                if i == 7 {
                    ui.add_space(8.0);
                }
            }

            // Pad remaining bytes in line
            for _ in bytes.len()..BYTES_PER_LINE {
                ui.label(
                    egui::RichText::new("  ")
                        .monospace()
                        .color(egui::Color32::DARK_GRAY),
                );
            }

            ui.separator();

            // ASCII column
            let ascii: String = bytes
                .iter()
                .map(|&b| {
                    if (0x20..=0x7E).contains(&b) {
                        b as char
                    } else {
                        '.'
                    }
                })
                .collect();

            ui.label(
                egui::RichText::new(ascii)
                    .monospace()
                    .color(egui::Color32::from_rgb(180, 180, 180)),
            );
        });

        clicked_command
    }

    /// Helper to find a unit by offset
    fn find_unit_by_offset(units: &[bitvue_core::UnitNode], offset: u64) -> Option<&bitvue_core::UnitNode> {
        for unit in units {
            if unit.offset == offset {
                return Some(unit);
            }
            if !unit.children.is_empty() {
                if let Some(found) = Self::find_unit_by_offset(&unit.children, offset) {
                    return Some(found);
                }
            }
        }
        None
    }
}

impl Default for HexViewPanel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hex_view_tab_labels() {
        assert_eq!(HexViewTab::UnitInfo.label(), "Unit Info");
        assert_eq!(HexViewTab::HexDump.label(), "Hex");
        assert_eq!(HexViewTab::DpbInfo.label(), "DPB Info");
    }

    #[test]
    fn test_hex_view_tab_default() {
        let tab = HexViewTab::default();
        assert_eq!(tab, HexViewTab::HexDump);
    }

    #[test]
    fn test_hex_view_panel_new() {
        let panel = HexViewPanel::new();
        assert!(panel.scroll_to_offset.is_none());
        assert_eq!(panel.current_tab, HexViewTab::HexDump);
    }

    #[test]
    fn test_hex_view_panel_default() {
        let panel: HexViewPanel = Default::default();
        assert_eq!(panel.current_tab, HexViewTab::HexDump);
    }
}
