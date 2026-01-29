//! Bitrate Graph Panel - frame size and bitrate visualization

use bitvue_core::{FrameType, SelectionState, UnitNode};
use egui;
use egui_plot::{Bar, BarChart, Legend, Plot};

pub struct BitrateGraphPanel;

impl BitrateGraphPanel {
    pub fn new() -> Self {
        Self
    }

    /// Show the bitrate graph panel
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        units: Option<&[UnitNode]>,
        selection: &SelectionState,
    ) {
        ui.heading("📊 Bitrate Graph");
        ui.separator();

        // Collect frame information
        let frames = if let Some(units) = units {
            collect_frame_sizes(units)
        } else {
            Vec::new()
        };

        if frames.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.label(egui::RichText::new("No frames to display").color(egui::Color32::GRAY));
            });
            return;
        }

        // Calculate statistics
        let total_bytes: usize = frames.iter().map(|f| f.size).sum();
        let avg_bytes = total_bytes / frames.len();
        let max_bytes = frames.iter().map(|f| f.size).max().unwrap_or(0);

        // Show statistics
        ui.horizontal(|ui| {
            ui.label(format!("Frames: {}", frames.len()));
            ui.separator();
            ui.label(format!("Total: {} KB", total_bytes / 1024));
            ui.separator();
            ui.label(format!("Avg: {} KB", avg_bytes / 1024));
            ui.separator();
            ui.label(format!("Max: {} KB", max_bytes / 1024));
        });

        ui.separator();

        // Create bar chart data
        let bars: Vec<Bar> = frames
            .iter()
            .enumerate()
            .map(|(idx, frame)| {
                let color = match frame.frame_type {
                    FrameType::Key => egui::Color32::from_rgb(100, 200, 100),
                    FrameType::Inter => egui::Color32::from_rgb(100, 150, 255),
                    FrameType::BFrame => egui::Color32::from_rgb(100, 150, 255),
                    FrameType::IntraOnly => egui::Color32::from_rgb(100, 200, 100),
                    _ => egui::Color32::from_rgb(150, 150, 150),
                };

                Bar::new(idx as f64, frame.size as f64 / 1024.0)
                    .width(0.8)
                    .fill(color)
            })
            .collect();

        let chart = BarChart::new(bars)
            .name("Frame Size (KB)")
            .color(egui::Color32::from_rgb(100, 150, 255));

        // Plot the chart
        Plot::new("bitrate_graph")
            .legend(Legend::default())
            .height(ui.available_height() - 40.0)
            .show_axes([true, true])
            .show_grid([true, true])
            .label_formatter(|_name, value| {
                format!("Frame {}: {:.1} KB", value.x as usize, value.y)
            })
            .show(ui, |plot_ui| {
                plot_ui.bar_chart(chart);

                // Highlight selected frame
                if let Some(uk) = &selection.unit {
                    if let Some(frame_idx) = frames.iter().position(|f| f.offset == uk.offset) {
                        // Draw vertical line at selected frame
                        plot_ui.vline(
                            egui_plot::VLine::new(frame_idx as f64)
                                .color(egui::Color32::from_rgb(255, 200, 100))
                                .width(2.0),
                        );
                    }
                }
            });
    }
}

impl Default for BitrateGraphPanel {
    fn default() -> Self {
        Self::new()
    }
}

/// Frame size information
#[derive(Debug, Clone)]
struct FrameSizeInfo {
    frame_index: usize,
    frame_type: FrameType,
    size: usize,
    offset: u64,
}

/// Recursively collect all frame sizes from unit tree
fn collect_frame_sizes(units: &[UnitNode]) -> Vec<FrameSizeInfo> {
    let mut frames = Vec::new();

    for unit in units {
        if let Some(frame_idx) = unit.frame_index {
            frames.push(FrameSizeInfo {
                frame_index: frame_idx,
                frame_type: FrameType::from_str(&unit.unit_type).unwrap_or(FrameType::Unknown),
                size: unit.size,
                offset: unit.offset,
            });
        }

        if !unit.children.is_empty() {
            frames.extend(collect_frame_sizes(&unit.children));
        }
    }

    frames.sort_by_key(|f| f.frame_index);
    frames
}
