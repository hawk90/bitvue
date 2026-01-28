//! Selection Info Panel - VQAnalyzer-style block details display
//!
//! Shows detailed information about the currently selected block in a compact 3-column layout:
//! - Frame/stream info (Width, Height, Pictures, Color format, Bitdepth)
//! - Position info (CTB addr, Block X/Y, Pixel X/Y, Tile/Slice info)
//! - Coding info (CU mode, TU type, QP, MV L0/L1)

use bitvue_core::SelectionState;
use egui::{self, Color32, RichText};

/// Selection info data (populated from decoder/parser)
/// VQAnalyzer parity: comprehensive block-level analysis data
#[derive(Debug, Clone, Default)]
pub struct BlockInfo {
    // Frame info
    pub width: u32,
    pub height: u32,
    pub frame_count: usize,
    pub chroma: String,
    pub bit_depth: u8,

    // Tile info
    pub tile_col: u32,
    pub tile_row: u32,

    // Superblock/CTB info (VQAnalyzer parity)
    pub sb_col: u32,
    pub sb_row: u32,
    /// CTB address (linear index)
    pub ctb_addr: u32,

    // Block/CU info (VQAnalyzer parity)
    pub block_seg_id: u32,
    pub block_size: String,
    pub block_x: u32,
    pub block_y: u32,
    pub block_col: u32,
    pub block_row: u32,
    pub block_part: String,
    /// CU depth (0 = CTB level, increases for smaller blocks)
    pub cu_depth: u8,

    // Pixel info
    pub pixel_col: u32,
    pub pixel_row: u32,

    // Prediction info
    pub pred_size: String,
    pub pred_mode: String,

    // Transform info
    pub tx_size: String,
    pub tx_type: String,
    /// TU depth (relative to CU)
    pub tu_depth: u8,

    // Motion vectors (VQAnalyzer parity: L0/L1 with reference POC)
    pub mv_first: String,
    pub mv_second: String,
    /// MV L0 components (dx, dy, ref_poc)
    pub mv_l0: Option<(i16, i16, i32)>,
    /// MV L1 components (dx, dy, ref_poc)
    pub mv_l1: Option<(i16, i16, i32)>,

    // QP info (VQAnalyzer parity)
    /// Quantization Parameter for this block
    pub qp: Option<u8>,
    /// QP delta from frame QP
    pub qp_delta: Option<i8>,
}

/// Selection Info Panel state
pub struct SelectionInfoPanel {
    /// Current block info (updated when selection changes)
    block_info: Option<BlockInfo>,
}

impl Default for SelectionInfoPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl SelectionInfoPanel {
    pub fn new() -> Self {
        Self { block_info: None }
    }

    /// Update block info from selection
    pub fn set_block_info(&mut self, info: Option<BlockInfo>) {
        self.block_info = info;
    }

    /// Show the selection info panel
    /// VQAnalyzer parity: Compact 3-column layout without collapsible sections
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        selection: &SelectionState,
        container: Option<&bitvue_core::ContainerModel>,
    ) {
        // Get frame info from container if available
        let (width, height, frame_count, codec, bitrate, max_ctb): (
            u32,
            u32,
            usize,
            String,
            u64,
            u32,
        ) = if let Some(c) = container {
            (
                c.width.unwrap_or(0),
                c.height.unwrap_or(0),
                c.track_count,
                c.codec.clone(),
                c.bitrate_bps.unwrap_or(0),
                128, // Default CTB size
            )
        } else {
            (0, 0, 0, String::new(), 0, 128)
        };

        // Current frame index from selection
        let current_frame = selection
            .temporal
            .as_ref()
            .map(|t| t.frame_index())
            .unwrap_or(0);

        // Get block info or create defaults
        let info = self.block_info.as_ref();

        // Header with title and close button (VQAnalyzer style)
        ui.horizontal(|ui| {
            ui.label(RichText::new("Selection Info").strong());
            if let Some(unit_key) = &selection.unit {
                ui.label("|");
                ui.label(
                    RichText::new(format!("{}", unit_key.offset))
                        .color(Color32::from_rgb(100, 180, 255))
                        .small(),
                );
            }
        });

        ui.separator();

        // Use scroll area for the content
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                // VQAnalyzer-style 6-column layout (3 pairs of label:value)
                egui::Grid::new("selection_info_main_grid")
                    .num_columns(6)
                    .spacing([4.0, 2.0])
                    .min_col_width(40.0)
                    .show(ui, |ui| {
                        // Row 1: Width, Height, Pictures (left col), CTB col/row (mid), TU size (right)
                        self.label(ui, "Width:");
                        self.value(ui, &format!("{}", width));
                        self.label(ui, "CTB col, row:");
                        self.value(
                            ui,
                            &format!(
                                "{},{}",
                                info.map(|i| i.sb_col).unwrap_or(0),
                                info.map(|i| i.sb_row).unwrap_or(0)
                            ),
                        );
                        self.label(ui, "TU size:");
                        self.value(ui, info.map(|i| i.tx_size.as_str()).unwrap_or("N/A"));
                        ui.end_row();

                        // Row 2
                        self.label(ui, "Height:");
                        self.value(ui, &format!("{}", height));
                        self.label(ui, "Block X,Y:");
                        self.value(
                            ui,
                            &format!(
                                "{},{}",
                                info.map(|i| i.block_x).unwrap_or(0),
                                info.map(|i| i.block_y).unwrap_or(0)
                            ),
                        );
                        self.label(ui, "TU depth:");
                        self.value(ui, &format!("{}", info.map(|i| i.tu_depth).unwrap_or(0)));
                        ui.end_row();

                        // Row 3
                        self.label(ui, "Pictures:");
                        self.value(ui, &format!("{}", frame_count));
                        self.label(ui, "Block size, depth:");
                        self.value(
                            ui,
                            &format!(
                                "{}, {}",
                                info.map(|i| i.block_size.as_str()).unwrap_or("N/A"),
                                info.map(|i| i.cu_depth).unwrap_or(0)
                            ),
                        );
                        self.label(ui, "TU type Y H/V:");
                        self.value(ui, info.map(|i| i.tx_type.as_str()).unwrap_or("N/A"));
                        ui.end_row();

                        // Row 4
                        self.label(ui, "Color format:");
                        self.value(ui, info.map(|i| i.chroma.as_str()).unwrap_or("4:2:0"));
                        self.label(ui, "CU mode type:");
                        self.value(ui, "All");
                        self.label(ui, "TU type Cb H/V:");
                        self.value(ui, info.map(|i| i.tx_type.as_str()).unwrap_or("N/A"));
                        ui.end_row();

                        // Row 5
                        self.label(ui, "Bitdepth Y,C:");
                        self.value(
                            ui,
                            &format!(
                                "{}, {}",
                                info.map(|i| i.bit_depth).unwrap_or(8),
                                info.map(|i| i.bit_depth).unwrap_or(8)
                            ),
                        );
                        self.label(ui, "CU tree type:");
                        self.value(ui, "Single");
                        self.label(ui, "TU type Cr H/V:");
                        self.value(ui, info.map(|i| i.tx_type.as_str()).unwrap_or("N/A"));
                        ui.end_row();

                        // Row 6
                        self.label(ui, "Max CTB size:");
                        self.value(ui, &format!("{}x{}", max_ctb, max_ctb));
                        self.label(ui, "CU pred mode:");
                        self.value(ui, info.map(|i| i.pred_mode.as_str()).unwrap_or("N/A"));
                        self.label(ui, "CU QpY:");
                        self.value(ui, &format!("{}", info.and_then(|i| i.qp).unwrap_or(0)));
                        ui.end_row();

                        // Row 7
                        self.label(ui, "Tile col, row:");
                        self.value(
                            ui,
                            &format!(
                                "{},{}",
                                info.map(|i| i.tile_col).unwrap_or(0),
                                info.map(|i| i.tile_row).unwrap_or(0)
                            ),
                        );
                        self.label(ui, "Pixel X,Y:");
                        self.value(
                            ui,
                            &format!(
                                "{},{}",
                                info.map(|i| i.pixel_col).unwrap_or(0),
                                info.map(|i| i.pixel_row).unwrap_or(0)
                            ),
                        );
                        self.label(ui, "MV L0:");
                        if let Some(i) = info {
                            if let Some((dx, dy, poc)) = i.mv_l0 {
                                self.value(ui, &format!("({},{}) r{}", dx, dy, poc));
                            } else {
                                self.value(ui, "N/A");
                            }
                        } else {
                            self.value(ui, "N/A");
                        }
                        ui.end_row();

                        // Row 8
                        self.label(ui, "Slice #:");
                        self.value(
                            ui,
                            &format!(
                                "{} - {}",
                                current_frame,
                                if info.map(|i| i.pred_mode.contains("Intra")).unwrap_or(false) {
                                    "I"
                                } else if info.map(|i| i.mv_l1.is_some()).unwrap_or(false) {
                                    "B"
                                } else {
                                    "P"
                                }
                            ),
                        );
                        self.label(ui, "4x4 X,Y:");
                        self.value(
                            ui,
                            &format!(
                                "{},{}",
                                info.map(|i| i.pixel_col / 4).unwrap_or(0),
                                info.map(|i| i.pixel_row / 4).unwrap_or(0)
                            ),
                        );
                        self.label(ui, "MV L1:");
                        if let Some(i) = info {
                            if let Some((dx, dy, poc)) = i.mv_l1 {
                                self.value(ui, &format!("({},{}) r{}", dx, dy, poc));
                            } else {
                                self.value(ui, "N/A");
                            }
                        } else {
                            self.value(ui, "N/A");
                        }
                        ui.end_row();

                        // Row 9
                        self.label(ui, "Sub picture #:");
                        self.value(ui, "0");
                        self.label(ui, "4x4 Z-scan idx:");
                        self.value(ui, "N/A");
                        self.label(ui, "");
                        self.value(ui, "");
                        ui.end_row();

                        // Row 10
                        self.label(ui, "CTB address:");
                        self.value(ui, &format!("{}", info.map(|i| i.ctb_addr).unwrap_or(0)));
                        self.label(ui, "PU size:");
                        self.value(ui, info.map(|i| i.pred_size.as_str()).unwrap_or("N/A"));
                        self.label(ui, "");
                        self.value(ui, "");
                        ui.end_row();
                    });

                // Additional info if unit is selected
                if let Some(unit_key) = &selection.unit {
                    ui.add_space(4.0);
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Unit:").small().color(Color32::GRAY));
                        ui.label(
                            RichText::new(&unit_key.unit_type)
                                .small()
                                .color(Color32::from_rgb(100, 180, 255)),
                        );
                        ui.label(
                            RichText::new(format!("@ 0x{:X}", unit_key.offset))
                                .small()
                                .color(Color32::GRAY),
                        );
                        ui.label(
                            RichText::new(format!("({} bytes)", unit_key.size))
                                .small()
                                .color(Color32::GRAY),
                        );
                    });
                }

                // Bit range info if selected
                if let Some(bit_range) = &selection.bit_range {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Bits:").small().color(Color32::GRAY));
                        ui.label(
                            RichText::new(format!("{}-{}", bit_range.start_bit, bit_range.end_bit))
                                .small()
                                .color(Color32::from_rgb(255, 180, 80)),
                        );
                        ui.label(
                            RichText::new(format!(
                                "({} bits)",
                                bit_range.end_bit - bit_range.start_bit
                            ))
                            .small()
                            .color(Color32::GRAY),
                        );
                    });
                }

                // Codec badge
                if !codec.is_empty() {
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(&codec)
                                .small()
                                .strong()
                                .color(Color32::from_rgb(100, 200, 100)),
                        );
                        if bitrate > 0 {
                            ui.label(
                                RichText::new(format!("@ {} kbps", bitrate / 1000))
                                    .small()
                                    .color(Color32::GRAY),
                            );
                        }
                    });
                }
            });
    }

    /// Render a label cell
    fn label(&self, ui: &mut egui::Ui, text: &str) {
        ui.label(RichText::new(text).small().color(Color32::GRAY));
    }

    /// Render a value cell
    fn value(&self, ui: &mut egui::Ui, text: &str) {
        ui.label(RichText::new(text).small().color(Color32::WHITE));
    }

    /// Clear block info
    pub fn clear(&mut self) {
        self.block_info = None;
    }
}
