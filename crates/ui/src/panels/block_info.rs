//! Block Info Panel - VQAnalyzer-style block details display
//!
//! Shows detailed information about the currently selected coding block:
//! - Block position and size
//! - Partition type
//! - Prediction mode (Intra/Inter)
//! - Transform info (size, type)
//! - Quantization parameter
//! - Motion vectors and reference frames
//! - Coefficients summary

use bitvue_core::SelectionState;
use egui::{self, Color32, Grid, RichText};

/// Block-level information (populated from decoder/parser)
#[derive(Debug, Clone, Default)]
pub struct BlockData {
    // Position
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,

    // Partition
    pub partition_type: String,
    pub depth: u8,

    // Prediction
    pub pred_mode: String,
    pub is_intra: bool,
    pub intra_mode: String,

    // Transform
    pub tx_size: String,
    pub tx_type: String,
    pub tx_depth: u8,

    // Quantization
    pub qp: i32,
    pub qp_y: i32,
    pub qp_u: i32,
    pub qp_v: i32,

    // Motion (for inter blocks)
    pub mv0: String,
    pub mv1: String,
    pub ref_frame0: String,
    pub ref_frame1: String,

    // Flags
    pub skip: bool,
    pub segment_id: u8,
    pub cdef_level: u8,

    // Coefficients
    pub has_coeffs: bool,
    pub coeff_count: u32,
    pub eob: u32,
}

/// Block Info Panel state
pub struct BlockInfoPanel {
    /// Current block data (updated when selection changes)
    block_data: Option<BlockData>,
    /// Expand sections (for future use)
    #[allow(dead_code)]
    show_motion: bool,
    #[allow(dead_code)]
    show_coeffs: bool,
}

impl Default for BlockInfoPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl BlockInfoPanel {
    pub fn new() -> Self {
        Self {
            block_data: None,
            show_motion: true,
            show_coeffs: true,
        }
    }

    /// Update block data from selection
    pub fn set_block_data(&mut self, data: Option<BlockData>) {
        self.block_data = data;
    }

    /// Show the block info panel
    pub fn show(&mut self, ui: &mut egui::Ui, selection: &SelectionState) {
        ui.heading("Block Info");
        ui.separator();

        // Check if we have a block selection
        let has_block = self.block_data.is_some();

        if !has_block && selection.unit.is_none() {
            ui.label(
                RichText::new("No block selected")
                    .color(Color32::GRAY)
                    .italics(),
            );
            ui.add_space(8.0);
            ui.label(
                RichText::new("Click on a block in the Player view")
                    .color(Color32::DARK_GRAY)
                    .small(),
            );
            return;
        }

        egui::ScrollArea::vertical().show(ui, |ui| {
            // Position & Size section
            ui.collapsing("Position & Size", |ui| {
                Grid::new("block_position_grid")
                    .num_columns(4)
                    .spacing([8.0, 4.0])
                    .striped(true)
                    .show(ui, |ui| {
                        if let Some(data) = &self.block_data {
                            self.info_row(ui, "X:", &format!("{}", data.x));
                            self.info_row(ui, "Y:", &format!("{}", data.y));
                            ui.end_row();

                            self.info_row(ui, "Width:", &format!("{}", data.width));
                            self.info_row(ui, "Height:", &format!("{}", data.height));
                            ui.end_row();
                        } else {
                            self.info_row(ui, "X:", "N/A");
                            self.info_row(ui, "Y:", "N/A");
                            ui.end_row();

                            self.info_row(ui, "Width:", "N/A");
                            self.info_row(ui, "Height:", "N/A");
                            ui.end_row();
                        }
                    });
            });

            ui.add_space(4.0);

            // Partition section
            ui.collapsing("Partition", |ui| {
                Grid::new("block_partition_grid")
                    .num_columns(4)
                    .spacing([8.0, 4.0])
                    .striped(true)
                    .show(ui, |ui| {
                        if let Some(data) = &self.block_data {
                            self.info_row(ui, "Type:", &data.partition_type);
                            self.info_row(ui, "Depth:", &format!("{}", data.depth));
                            ui.end_row();
                        } else {
                            self.info_row(ui, "Type:", "N/A");
                            self.info_row(ui, "Depth:", "N/A");
                            ui.end_row();
                        }
                    });
            });

            ui.add_space(4.0);

            // Prediction section
            ui.collapsing("Prediction", |ui| {
                Grid::new("block_prediction_grid")
                    .num_columns(4)
                    .spacing([8.0, 4.0])
                    .striped(true)
                    .show(ui, |ui| {
                        if let Some(data) = &self.block_data {
                            self.info_row(ui, "Mode:", &data.pred_mode);
                            let type_str = if data.is_intra { "INTRA" } else { "INTER" };
                            self.info_row(ui, "Type:", type_str);
                            ui.end_row();

                            if data.is_intra {
                                self.info_row(ui, "Intra Mode:", &data.intra_mode);
                                self.info_row(ui, "", "");
                                ui.end_row();
                            }
                        } else {
                            self.info_row(ui, "Mode:", "N/A");
                            self.info_row(ui, "Type:", "N/A");
                            ui.end_row();
                        }
                    });
            });

            ui.add_space(4.0);

            // Transform section
            ui.collapsing("Transform", |ui| {
                Grid::new("block_transform_grid")
                    .num_columns(4)
                    .spacing([8.0, 4.0])
                    .striped(true)
                    .show(ui, |ui| {
                        if let Some(data) = &self.block_data {
                            self.info_row(ui, "TX Size:", &data.tx_size);
                            self.info_row(ui, "TX Type:", &data.tx_type);
                            ui.end_row();

                            self.info_row(ui, "TX Depth:", &format!("{}", data.tx_depth));
                            self.info_row(ui, "", "");
                            ui.end_row();
                        } else {
                            self.info_row(ui, "TX Size:", "N/A");
                            self.info_row(ui, "TX Type:", "N/A");
                            ui.end_row();
                        }
                    });
            });

            ui.add_space(4.0);

            // Quantization section
            ui.collapsing("Quantization", |ui| {
                Grid::new("block_qp_grid")
                    .num_columns(4)
                    .spacing([8.0, 4.0])
                    .striped(true)
                    .show(ui, |ui| {
                        if let Some(data) = &self.block_data {
                            self.info_row(ui, "QP:", &format!("{}", data.qp));
                            self.info_row(ui, "Segment:", &format!("{}", data.segment_id));
                            ui.end_row();

                            self.info_row(ui, "QP Y:", &format!("{}", data.qp_y));
                            self.info_row(ui, "QP U:", &format!("{}", data.qp_u));
                            ui.end_row();

                            self.info_row(ui, "QP V:", &format!("{}", data.qp_v));
                            self.info_row(ui, "CDEF:", &format!("{}", data.cdef_level));
                            ui.end_row();
                        } else {
                            self.info_row(ui, "QP:", "N/A");
                            self.info_row(ui, "Segment:", "N/A");
                            ui.end_row();
                        }
                    });
            });

            ui.add_space(4.0);

            // Motion section (for inter blocks)
            ui.collapsing("Motion Vectors", |ui| {
                Grid::new("block_motion_grid")
                    .num_columns(4)
                    .spacing([8.0, 4.0])
                    .striped(true)
                    .show(ui, |ui| {
                        if let Some(data) = &self.block_data {
                            if !data.is_intra {
                                self.info_row(ui, "MV0:", &data.mv0);
                                self.info_row(ui, "Ref0:", &data.ref_frame0);
                                ui.end_row();

                                self.info_row(ui, "MV1:", &data.mv1);
                                self.info_row(ui, "Ref1:", &data.ref_frame1);
                                ui.end_row();
                            } else {
                                ui.label(RichText::new("N/A (Intra block)").color(Color32::GRAY));
                                ui.end_row();
                            }
                        } else {
                            self.info_row(ui, "MV0:", "N/A");
                            self.info_row(ui, "Ref0:", "N/A");
                            ui.end_row();
                        }
                    });
            });

            ui.add_space(4.0);

            // Coefficients section
            ui.collapsing("Coefficients", |ui| {
                Grid::new("block_coeff_grid")
                    .num_columns(4)
                    .spacing([8.0, 4.0])
                    .striped(true)
                    .show(ui, |ui| {
                        if let Some(data) = &self.block_data {
                            let has_str = if data.has_coeffs { "Yes" } else { "No" };
                            self.info_row(ui, "Has Coeffs:", has_str);
                            self.info_row(ui, "Skip:", if data.skip { "Yes" } else { "No" });
                            ui.end_row();

                            self.info_row(ui, "Count:", &format!("{}", data.coeff_count));
                            self.info_row(ui, "EOB:", &format!("{}", data.eob));
                            ui.end_row();
                        } else {
                            self.info_row(ui, "Has Coeffs:", "N/A");
                            self.info_row(ui, "Skip:", "N/A");
                            ui.end_row();
                        }
                    });
            });
        });
    }

    /// Helper to render a label-value pair
    fn info_row(&self, ui: &mut egui::Ui, label: &str, value: &str) {
        ui.label(RichText::new(label).color(Color32::GRAY));
        ui.label(RichText::new(value).color(Color32::WHITE));
    }

    /// Clear block data
    pub fn clear(&mut self) {
        self.block_data = None;
    }
}
