//! Bit View Panel - bit-level visualization of bitstream

use bitvue_core::{ByteCache, Command, SelectionState};
use egui;

const BYTES_PER_LINE: usize = 8;

pub struct BitViewPanel {
    /// Scroll to this offset (for TC01: Tree→Bit sync)
    scroll_to_offset: Option<usize>,
}

impl BitViewPanel {
    pub fn new() -> Self {
        Self {
            scroll_to_offset: None,
        }
    }

    /// Show the bit view panel
    /// Returns optional Command if user clicks a bit (TC04: Bit→Tree sync)
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        byte_cache: Option<&ByteCache>,
        units: Option<&[bitvue_core::UnitNode]>,
        selection: &SelectionState,
    ) -> Option<Command> {
        let mut clicked_command = None;

        ui.heading("🔢 Bit View");
        ui.separator();

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

        // Get exact bit range for bit-level highlighting
        let bit_range_bits = selection
            .bit_range
            .as_ref()
            .map(|br| (br.start_bit as usize, br.end_bit as usize));

        // Sync scroll position to selected unit or bit range (TC01: Tree→Bit, Tri-sync)
        let scroll_target = bit_range_bytes.or(selected_range);
        if let Some((offset, _size)) = scroll_target {
            if self.scroll_to_offset != Some(offset) {
                self.scroll_to_offset = Some(offset);
            }
        }

        if let Some(cache) = byte_cache {
            let total_bytes = cache.len() as usize;

            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    // Calculate line range to display
                    let total_lines = total_bytes.div_ceil(BYTES_PER_LINE);

                    // Render bit view
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
                            if let Some(cmd) = self.render_bit_line(
                                ui,
                                offset,
                                line_bytes,
                                selected_range,
                                bit_range_bits,
                                units,
                            ) {
                                clicked_command = Some(cmd);
                            }
                        }
                    }
                });
        } else {
            ui.centered_and_justified(|ui| {
                ui.label(egui::RichText::new("No file loaded").color(egui::Color32::GRAY));
            });
        }

        clicked_command
    }

    fn render_bit_line(
        &self,
        ui: &mut egui::Ui,
        offset: usize,
        bytes: &[u8],
        selected_range: Option<(usize, usize)>,
        bit_range_bits: Option<(usize, usize)>,
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

            // Bit columns (8 bits per byte)
            for (byte_idx, &byte) in bytes.iter().enumerate() {
                let byte_offset = offset + byte_idx;
                let is_byte_selected = selected_range
                    .map(|(sel_offset, sel_size)| {
                        byte_offset >= sel_offset && byte_offset < sel_offset + sel_size
                    })
                    .unwrap_or(false);

                // Render each bit in the byte (MSB first)
                for bit_idx in (0..8).rev() {
                    let bit = (byte >> bit_idx) & 1;
                    let bit_char = if bit == 1 { "1" } else { "0" };

                    // Calculate absolute bit position
                    let absolute_bit_pos = (byte_offset * 8) + (7 - bit_idx);

                    // Check if this specific bit is in the selected bit range (Tri-sync)
                    let is_bit_in_range = bit_range_bits
                        .map(|(start_bit, end_bit)| {
                            absolute_bit_pos >= start_bit && absolute_bit_pos < end_bit
                        })
                        .unwrap_or(false);

                    let color = if is_bit_in_range {
                        // Tri-sync: Syntax node bit range highlight (brightest)
                        if bit == 1 {
                            egui::Color32::from_rgb(255, 180, 80) // Bright orange
                        } else {
                            egui::Color32::from_rgb(200, 100, 0) // Dim orange
                        }
                    } else if is_byte_selected {
                        // Unit selection highlight (dim)
                        if bit == 1 {
                            egui::Color32::from_rgb(200, 200, 120) // Dim yellow
                        } else {
                            egui::Color32::from_rgb(150, 150, 90)
                        }
                    } else {
                        // Normal display - bright and readable
                        if bit == 1 {
                            egui::Color32::from_rgb(240, 240, 240) // Bright white
                        } else {
                            egui::Color32::from_rgb(120, 120, 120) // Brighter gray
                        }
                    };

                    let response = ui.label(egui::RichText::new(bit_char).monospace().color(color));

                    // TC04: Bit→Tree - clicking a bit selects that specific bit (Tri-sync)
                    if response.clicked() {
                        // Single bit selection for maximum precision
                        let bit_start = absolute_bit_pos as u64;
                        let bit_end = bit_start + 1;

                        clicked_command = Some(Command::SelectBitRange {
                            stream: bitvue_core::StreamId::A,
                            bit_range: bitvue_core::BitRange {
                                start_bit: bit_start,
                                end_bit: bit_end,
                            },
                        });
                    }
                }

                // Add spacing between bytes
                ui.add_space(4.0);

                // Add larger spacing every 4 bytes
                if byte_idx == 3 {
                    ui.add_space(8.0);
                }
            }

            ui.separator();

            // Hex representation for reference
            let hex_str: String = bytes
                .iter()
                .map(|b| format!("{:02X}", b))
                .collect::<Vec<_>>()
                .join(" ");

            ui.label(
                egui::RichText::new(hex_str)
                    .monospace()
                    .color(egui::Color32::from_rgb(160, 160, 160))
                    .size(10.0),
            );
        });

        clicked_command
    }
}

impl Default for BitViewPanel {
    fn default() -> Self {
        Self::new()
    }
}
