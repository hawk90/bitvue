//! Helper functions for filmstrip panel

use super::{FilmstripPanel, FrameInfo};
use bitvue_core::UnitNode;
use egui::Color32;

impl FilmstripPanel {
    /// Get frame type color (VQAnalyzer parity: I=red, P=green, B=blue)
    pub(super) fn frame_type_color(frame_type: &str) -> Color32 {
        match frame_type {
            // AV1 frame types
            "KEY" | "INTRA_ONLY" => Color32::from_rgb(220, 80, 80), // Red for I-frames (KEY, INTRA_ONLY)
            "INTER" => Color32::from_rgb(80, 180, 80),              // Green for P-frames (INTER)
            "SWITCH" => Color32::from_rgb(180, 100, 180),           // Purple for switch frames
            // Legacy codec support (H.264/HEVC)
            "I" | "IDR" | "INTRA" => Color32::from_rgb(220, 80, 80), // Red for I-frames
            "P" | "PRED" => Color32::from_rgb(80, 180, 80),          // Green for P-frames
            "B" | "BFRAME" | "BPRED" => Color32::from_rgb(80, 140, 220), // Blue for B-frames
            // Unknown
            "FRAME" => Color32::from_rgb(120, 120, 120), // Gray for unknown
            "UNKNOWN" => Color32::from_rgb(120, 120, 120), // Gray for unknown
            _ => Color32::from_rgb(100, 100, 100),       // Dark gray default
        }
    }
}

/// Collect frame information from unit tree
pub(super) fn collect_frame_info(
    units: &[UnitNode],
    diagnostics: &[bitvue_core::event::Diagnostic],
) -> Vec<FrameInfo> {
    let mut frames = Vec::new();
    collect_frame_info_recursive(units, &mut frames, diagnostics);
    // Sort by frame index
    frames.sort_by_key(|f| f.frame_index);
    frames
}

fn collect_frame_info_recursive(
    units: &[UnitNode],
    frames: &mut Vec<FrameInfo>,
    diagnostics: &[bitvue_core::event::Diagnostic],
) {
    for unit in units {
        if let Some(frame_index) = unit.frame_index {
            // VQAnalyzer parity: Extract NAL type from unit_type
            let nal_type = extract_nal_type(&unit.unit_type);

            // Determine reference list (L0 for forward ref, L1 for backward)
            let ref_list = if unit.unit_type.contains("IDR")
                || unit.unit_type.contains("KEY")
                || unit.unit_type.contains("INTRA")
            {
                None // Intra frames have no references
            } else {
                Some("L0".to_string()) // Default to L0 for inter frames
            };

            // Bitvue unique feature: Count diagnostics for this frame
            let frame_diagnostics: Vec<_> = diagnostics
                .iter()
                .filter(|d| {
                    // Match by frame_index or offset range
                    d.frame_index == Some(frame_index)
                        || (d.offset_bytes >= unit.offset
                            && d.offset_bytes < unit.offset + unit.size as u64)
                })
                .collect();

            let diagnostic_count = frame_diagnostics.len();
            let max_impact = frame_diagnostics
                .iter()
                .map(|d| d.impact_score)
                .max()
                .unwrap_or(0);

            frames.push(FrameInfo {
                frame_index,
                frame_type: unit
                    .frame_type
                    .clone()
                    .unwrap_or_else(|| "UNKNOWN".to_string()),
                unit_key: unit.key.clone(),
                offset: unit.offset,
                size: unit.size, // Actual unit size in bytes
                // POC: use frame_index as approximation when not available
                poc: frame_index as i32,
                // VQAnalyzer parity: decode_order vs display_order
                // In real impl, these come from PTS/DTS or POC
                decode_order: frame_index,  // Decode order = stream order
                display_order: frame_index, // Display order = POC order (simplified)
                nal_type,
                pts: unit.pts,
                dts: unit.dts,
                ref_list,
                diagnostic_count,
                max_impact,
            });
        }
        if !unit.children.is_empty() {
            collect_frame_info_recursive(&unit.children, frames, diagnostics);
        }
    }
}

/// Extract NAL unit type name from unit_type string
fn extract_nal_type(unit_type: &str) -> String {
    // VQAnalyzer shows abbreviated NAL types like "TRAIL_N", "IDR_W_RADL", etc.
    if unit_type.contains("IDR") {
        "IDR".to_string()
    } else if unit_type.contains("KEY") || unit_type.contains("INTRA") {
        "KEY".to_string()
    } else if unit_type.contains("INTER") {
        "INTER".to_string()
    } else if unit_type.contains("TRAIL") {
        "TRAIL_N".to_string()
    } else if unit_type.contains("SWITCH") {
        "SWITCH".to_string()
    } else {
        // Return abbreviated version of unit_type
        let parts: Vec<&str> = unit_type.split(&['_', ' ', '-'][..]).collect();
        parts
            .first()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "FRAME".to_string())
    }
}

/// Extract frame type from unit type string
fn extract_frame_type(unit_type: &str) -> String {
    if unit_type.contains("KEY") || unit_type.contains("INTRA") {
        "KEY".to_string()
    } else if unit_type.contains("INTER") {
        "INTER".to_string()
    } else if unit_type.contains("SWITCH") {
        "SWITCH".to_string()
    } else {
        "FRAME".to_string()
    }
}
