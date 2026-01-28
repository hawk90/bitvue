//! Diagnostics Workspace - WS_DIAGNOSTICS_ERROR (Monster Pack v9)
//!
//! Error and warning visualization (VQAnalyzer parity):
//! - Table view with columns: Severity, Pos, Type, Message
//! - Severity filtering
//! - Search filter
//! - Click to navigate

use bitvue_core::event::{Category, Diagnostic, Severity};
use egui;
use egui_extras::{Column, TableBuilder};

/// Diagnostics workspace state
pub struct DiagnosticsWorkspace {
    /// Show errors
    show_errors: bool,
    /// Show warnings
    show_warnings: bool,
    /// Show info messages
    show_info: bool,
    /// Search filter
    search_filter: String,
}

impl DiagnosticsWorkspace {
    pub fn new() -> Self {
        Self {
            show_errors: true,
            show_warnings: true,
            show_info: false,
            search_filter: String::new(),
        }
    }

    /// Show the diagnostics workspace (VQAnalyzer table style)
    pub fn show(&mut self, ui: &mut egui::Ui, diagnostics: &[Diagnostic]) -> Option<Diagnostic> {
        let mut clicked_diagnostic = None;

        // Toolbar: Severity filters + search (VQAnalyzer style)
        ui.horizontal(|ui| {
            ui.label("Show:");
            ui.checkbox(&mut self.show_errors, "Errors");
            ui.checkbox(&mut self.show_warnings, "Warnings");
            ui.checkbox(&mut self.show_info, "Info");

            ui.separator();
            ui.label("Search:");
            ui.text_edit_singleline(&mut self.search_filter);
        });

        ui.separator();

        // Summary stats (VQAnalyzer style)
        let error_count = diagnostics
            .iter()
            .filter(|d| matches!(d.severity, Severity::Error | Severity::Fatal))
            .count();
        let warn_count = diagnostics
            .iter()
            .filter(|d| matches!(d.severity, Severity::Warn))
            .count();
        let info_count = diagnostics
            .iter()
            .filter(|d| matches!(d.severity, Severity::Info))
            .count();

        ui.horizontal(|ui| {
            if error_count > 0 {
                ui.colored_label(
                    egui::Color32::from_rgb(255, 80, 80),
                    format!("⚠ {} errors", error_count),
                );
            }
            if warn_count > 0 {
                if error_count > 0 {
                    ui.separator();
                }
                ui.colored_label(
                    egui::Color32::from_rgb(255, 200, 80),
                    format!("⚠ {} warnings", warn_count),
                );
            }
            if info_count > 0 {
                if error_count > 0 || warn_count > 0 {
                    ui.separator();
                }
                ui.colored_label(
                    egui::Color32::from_rgb(100, 150, 255),
                    format!("ℹ {} info", info_count),
                );
            }
        });

        ui.separator();

        // Filter diagnostics
        let filtered: Vec<&Diagnostic> = diagnostics
            .iter()
            .filter(|d| {
                // Severity filter
                let severity_match = match d.severity {
                    Severity::Error | Severity::Fatal => self.show_errors,
                    Severity::Warn => self.show_warnings,
                    Severity::Info => self.show_info,
                };

                // Search filter
                let search_match = if self.search_filter.is_empty() {
                    true
                } else {
                    d.message
                        .to_lowercase()
                        .contains(&self.search_filter.to_lowercase())
                };

                severity_match && search_match
            })
            .collect();

        // Bitvue ENHANCED table - 11 columns (VQAnalyzer 7 + bitvue 4)
        egui::ScrollArea::both()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                TableBuilder::new(ui)
                    .striped(true)
                    .resizable(true)
                    .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                    .column(Column::auto().at_least(70.0)) // Severity
                    .column(Column::auto().at_least(60.0)) // Frame # ⭐ NEW
                    .column(Column::auto().at_least(80.0)) // Timestamp ⭐ NEW
                    .column(Column::auto().at_least(70.0)) // Pos
                    .column(Column::auto().at_least(60.0)) // NAL idx
                    .column(Column::auto().at_least(50.0)) // Field
                    .column(Column::auto().at_least(60.0)) // CTB idx
                    .column(Column::auto().at_least(50.0)) // Type
                    .column(Column::auto().at_least(50.0)) // Count ⭐ NEW
                    .column(Column::auto().at_least(60.0)) // Impact ⭐ NEW
                    .column(Column::remainder().at_least(250.0)) // Message
                    .header(20.0, |mut header| {
                        // Bitvue ENHANCED column headers
                        header.col(|ui| {
                            ui.strong("Severity");
                        });
                        header.col(|ui| {
                            ui.strong("Frame #")
                                .on_hover_text("Frame number (bitvue extension)");
                        });
                        header.col(|ui| {
                            ui.strong("Time")
                                .on_hover_text("Timestamp (bitvue extension)");
                        });
                        header.col(|ui| {
                            ui.strong("Pos");
                        });
                        header.col(|ui| {
                            ui.strong("NAL idx");
                        });
                        header.col(|ui| {
                            ui.strong("Field");
                        });
                        header.col(|ui| {
                            ui.strong("CTB idx");
                        });
                        header.col(|ui| {
                            ui.strong("Type");
                        });
                        header.col(|ui| {
                            ui.strong("Count")
                                .on_hover_text("Repetition count (bitvue extension)");
                        });
                        header.col(|ui| {
                            ui.strong("Impact")
                                .on_hover_text("Impact score 0-100 (bitvue extension)");
                        });
                        header.col(|ui| {
                            ui.strong("Message");
                        });
                    })
                    .body(|mut body| {
                        if filtered.is_empty() {
                            body.row(30.0, |mut row| {
                                for _ in 0..5 {
                                    row.col(|ui| {
                                        ui.label("");
                                    });
                                }
                                row.col(|ui| {
                                    ui.label(
                                        egui::RichText::new("No diagnostics")
                                            .color(egui::Color32::GRAY),
                                    );
                                });
                                for _ in 0..5 {
                                    row.col(|ui| {
                                        ui.label("");
                                    });
                                }
                            });
                        } else {
                            for diagnostic in filtered.iter() {
                                let row_height = 20.0;
                                body.row(row_height, |mut row| {
                                    // 1. Severity column (VQAnalyzer)
                                    row.col(|ui| {
                                        let (text, color) = match diagnostic.severity {
                                            Severity::Fatal => {
                                                ("Fatal", egui::Color32::from_rgb(200, 50, 50))
                                            }
                                            Severity::Error => {
                                                ("Error", egui::Color32::from_rgb(255, 80, 80))
                                            }
                                            Severity::Warn => {
                                                ("Warning", egui::Color32::from_rgb(255, 200, 80))
                                            }
                                            Severity::Info => {
                                                ("Info", egui::Color32::from_rgb(100, 150, 255))
                                            }
                                        };
                                        ui.colored_label(color, text);
                                    });

                                    // 2. Frame # column ⭐ BITVUE EXTENSION
                                    row.col(|ui| {
                                        if let Some(frame_idx) = diagnostic.frame_index {
                                            ui.label(format!("{}", frame_idx));
                                        } else {
                                            ui.label("-");
                                        }
                                    });

                                    // 3. Timestamp column ⭐ BITVUE EXTENSION
                                    row.col(|ui| {
                                        let time_sec = diagnostic.timestamp_ms as f64 / 1000.0;
                                        ui.label(format!("{:.2}s", time_sec));
                                    });

                                    // 4. Pos column (VQAnalyzer)
                                    row.col(|ui| {
                                        ui.label(format!("{}", diagnostic.offset_bytes));
                                    });

                                    // 5. NAL idx column (VQAnalyzer)
                                    row.col(|ui| {
                                        // TODO: Extract actual NAL index from unit data
                                        let nal_idx = (diagnostic.offset_bytes / 1000) % 1000;
                                        ui.label(format!("{}", nal_idx));
                                    });

                                    // 6. Field column (VQAnalyzer)
                                    row.col(|ui| {
                                        // TODO: Add field information to Diagnostic struct
                                        ui.label("");
                                    });

                                    // 7. CTB idx column (VQAnalyzer)
                                    row.col(|ui| {
                                        // TODO: Extract actual CTB index from block data
                                        let ctb_idx = (diagnostic.offset_bytes / 100) % 100;
                                        if ctb_idx > 0 {
                                            ui.label(format!("{}", ctb_idx));
                                        } else {
                                            ui.label("");
                                        }
                                    });

                                    // 8. Type column (VQAnalyzer)
                                    row.col(|ui| {
                                        let type_str = match diagnostic.category {
                                            Category::Container => "CTB",
                                            Category::Bitstream => "NAL",
                                            Category::Decode => "HRD",
                                            Category::Metric => "QP",
                                            Category::IO => "IO",
                                            Category::Worker => "SYS",
                                        };
                                        ui.label(type_str);
                                    });

                                    // 9. Count column ⭐ BITVUE EXTENSION
                                    row.col(|ui| {
                                        if diagnostic.count > 1 {
                                            ui.colored_label(
                                                egui::Color32::from_rgb(255, 150, 0),
                                                format!("{}x", diagnostic.count),
                                            );
                                        } else {
                                            ui.label("1x");
                                        }
                                    });

                                    // 10. Impact column ⭐ BITVUE EXTENSION
                                    row.col(|ui| {
                                        let (icon, color) = if diagnostic.impact_score >= 80 {
                                            ("●", egui::Color32::from_rgb(255, 80, 80))
                                        // Red dot - Critical
                                        } else if diagnostic.impact_score >= 50 {
                                            ("▲", egui::Color32::from_rgb(255, 200, 80))
                                        // Orange triangle - Warning
                                        } else {
                                            ("○", egui::Color32::from_rgb(100, 200, 100))
                                            // Green circle - OK
                                        };
                                        ui.horizontal(|ui| {
                                            ui.colored_label(
                                                color,
                                                egui::RichText::new(icon).strong(),
                                            );
                                            ui.label(format!("{}", diagnostic.impact_score));
                                        });
                                    });

                                    // 11. Message column (clickable for tri-sync navigation)
                                    row.col(|ui| {
                                        let response =
                                            ui.selectable_label(false, &diagnostic.message);
                                        if response.clicked() {
                                            clicked_diagnostic = Some((*diagnostic).clone());
                                        }
                                        response
                                            .on_hover_text("Click to navigate to error location");
                                    });
                                });
                            }
                        }
                    });
            });

        clicked_diagnostic
    }
}

impl Default for DiagnosticsWorkspace {
    fn default() -> Self {
        Self::new()
    }
}
