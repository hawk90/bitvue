//! Export functionality for bitstream data
//!
//! Supports exporting:
//! - Frame sizes to CSV
//! - Syntax tree to JSON
//! - Unit tree to JSON

use bitvue_core::BitvueError;
use bitvue_core::types::SyntaxModel;
use bitvue_core::UnitNode;
use std::path::Path;

/// Export format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Csv,
    Json,
}

/// Export frame sizes to CSV
///
/// Format:
/// ```csv
/// frame_index,frame_type,size_bytes,offset,qp_avg
/// 0,KEY_FRAME,12345,0,28
/// 1,INTER_FRAME,3456,12345,32
/// ```
pub fn export_frames_csv(units: &[UnitNode], path: &Path) -> Result<(), BitvueError> {
    let mut csv = String::from("frame_index,frame_type,size_bytes,offset,qp_avg\n");

    let frames = collect_frames(units);
    for frame in frames {
        let qp_str = frame
            .qp_avg
            .map(|q| q.to_string())
            .unwrap_or_else(|| "".to_string());
        csv.push_str(&format!(
            "{},{},{},{},{}\n",
            frame.frame_index, frame.frame_type, frame.size, frame.offset, qp_str
        ));
    }

    std::fs::write(path, csv)?;
    Ok(())
}

/// Export unit tree to JSON
pub fn export_units_json(units: &[UnitNode], path: &Path) -> Result<(), BitvueError> {
    let json = serde_json::to_string_pretty(units)?;

    std::fs::write(path, json)?;
    Ok(())
}

/// Export syntax tree to JSON
pub fn export_syntax_json(syntax: &SyntaxModel, path: &Path) -> Result<(), BitvueError> {
    let json = serde_json::to_string_pretty(syntax)?;

    std::fs::write(path, json)?;
    Ok(())
}

/// Frame info for CSV export
#[derive(Debug)]
struct FrameInfo {
    frame_index: usize,
    frame_type: String,
    size: usize,
    offset: u64,
    qp_avg: Option<u8>,
}

/// Recursively collect all frames from unit tree
fn collect_frames(units: &[UnitNode]) -> Vec<FrameInfo> {
    let mut frames = Vec::new();

    for unit in units {
        if let Some(frame_idx) = unit.frame_index {
            frames.push(FrameInfo {
                frame_index: frame_idx,
                frame_type: extract_frame_type(&unit.unit_type),
                size: unit.size,
                offset: unit.offset,
                qp_avg: unit.qp_avg,
            });
        }

        if !unit.children.is_empty() {
            frames.extend(collect_frames(&unit.children));
        }
    }

    frames.sort_by_key(|f| f.frame_index);
    frames
}

/// Extract frame type from unit type string
fn extract_frame_type(unit_type: &str) -> String {
    if unit_type.contains("KEY") || unit_type.contains("INTRA") {
        "KEY_FRAME".to_string()
    } else if unit_type.contains("INTER") {
        "INTER_FRAME".to_string()
    } else {
        "FRAME".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_frame_type() {
        assert_eq!(extract_frame_type("FRAME (KEY_FRAME)"), "KEY_FRAME");
        assert_eq!(extract_frame_type("FRAME (INTER_FRAME)"), "INTER_FRAME");
        assert_eq!(extract_frame_type("FRAME"), "FRAME");
    }
}
