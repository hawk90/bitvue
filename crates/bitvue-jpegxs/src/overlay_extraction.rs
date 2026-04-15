//! Overlay data extraction for JPEG XS frames.
//!
//! Produces the data consumed by the five frontend renderers:
//!   precinct  → PrecinctMap
//!   dequant   → DequantMap
//!   transform → TransformMap
//!   mct       → MctInfo
//!   nlt       → NltInfo

use crate::frames::JpegXsFrame;
use serde::{Deserialize, Serialize};

// ─── Precinct map ──────────────────────────────────────────────────────────────

/// Grid of precincts with per-precinct estimated size (bits).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrecinctMap {
    pub cols: u32,
    pub rows: u32,
    pub precinct_w: u32,
    pub precinct_h: u32,
    /// Estimated bits per precinct (cols × rows entries, row-major).
    pub bits: Vec<u32>,
}

pub fn extract_precinct_map(frame: &JpegXsFrame) -> Option<PrecinctMap> {
    if frame.precinct_cols == 0 || frame.precinct_rows == 0 {
        return None;
    }
    let n = (frame.precinct_cols * frame.precinct_rows) as usize;
    // Without decoding the bitstream we estimate a uniform distribution
    let total_bits = (frame.bpp_x1000 as u64 * frame.width as u64 * frame.height as u64) / 1000;
    let bits_per_precinct = if n > 0 {
        (total_bits / n as u64) as u32
    } else {
        0
    };
    let bits = vec![bits_per_precinct; n];
    Some(PrecinctMap {
        cols: frame.precinct_cols,
        rows: frame.precinct_rows,
        precinct_w: frame.precinct_width,
        precinct_h: frame.precinct_height,
        bits,
    })
}

// ─── Dequant map ───────────────────────────────────────────────────────────────

/// Sub-band dequantization energy heatmap.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DequantMap {
    /// Grid width in sub-band blocks.
    pub grid_w: u32,
    /// Grid height in sub-band blocks.
    pub grid_h: u32,
    /// Normalised energy value [0.0, 1.0] per cell.
    pub energy: Vec<f32>,
    /// Sub-band label per cell (e.g., "LL", "LH", "HL", "HH").
    pub labels: Vec<String>,
}

pub fn extract_dequant_map(frame: &JpegXsFrame) -> Option<DequantMap> {
    if frame.width == 0 || frame.height == 0 {
        return None;
    }
    // Build a simple sub-band grid based on the decomposition levels.
    // Each decomposition level produces LH, HL, HH bands; the final LL is one cell.
    let nd = frame.decomp_h.max(frame.decomp_v) as usize;
    let nd = nd.max(1);
    // Grid: 2^nd columns × 2^nd rows
    let gw = 1u32 << nd;
    let gh = 1u32 << nd;
    let n = (gw * gh) as usize;

    let mut energy = vec![0.0f32; n];
    let mut labels = vec![String::from("HH"); n];

    // Mark LL cell (top-left)
    energy[0] = 1.0;
    labels[0] = "LL".to_string();

    // Mark LH and HL bands at each decomposition level
    for level in 0..nd {
        let stride = 1 << (nd - level - 1);
        // LH: row 0, col stride
        if stride < gw as usize {
            let idx = stride;
            energy[idx] = 0.7 - 0.1 * level as f32;
            labels[idx] = format!("LH{}", level + 1);
        }
        // HL: row stride, col 0
        if stride < gh as usize {
            let idx = stride * gw as usize;
            energy[idx] = 0.6 - 0.1 * level as f32;
            labels[idx] = format!("HL{}", level + 1);
        }
        // HH: row stride, col stride
        if stride < gw as usize && stride < gh as usize {
            let idx = stride * gw as usize + stride;
            energy[idx] = 0.4 - 0.08 * level as f32;
            labels[idx] = format!("HH{}", level + 1);
        }
    }

    Some(DequantMap {
        grid_w: gw,
        grid_h: gh,
        energy,
        labels,
    })
}

// ─── Transform sub-band tree ───────────────────────────────────────────────────

/// One node in the wavelet decomposition tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubbandNode {
    pub name: String,
    /// x position in display grid (pixels from left, normalised to [0,1]).
    pub x: f32,
    /// y position.
    pub y: f32,
    /// Width fraction.
    pub w: f32,
    /// Height fraction.
    pub h: f32,
    /// Decomposition level (0 = finest-detail HH, nd-1 = coarsest).
    pub level: u8,
}

/// Wavelet sub-band transform visualisation data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformMap {
    pub decomp_h: u8,
    pub decomp_v: u8,
    pub nodes: Vec<SubbandNode>,
}

pub fn extract_transform_map(frame: &JpegXsFrame) -> TransformMap {
    let dh = frame.decomp_h.max(1) as usize;
    let mut nodes = Vec::new();

    // Build the quad-tree partition
    fn subdivide(
        nodes: &mut Vec<SubbandNode>,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        level: usize,
        max_levels: usize,
    ) {
        if level >= max_levels {
            nodes.push(SubbandNode {
                name: "LL".to_string(),
                x,
                y,
                w,
                h,
                level: level as u8,
            });
            return;
        }
        let hw = w / 2.0;
        let hh = h / 2.0;
        // LH — top-right (horizontal detail)
        nodes.push(SubbandNode {
            name: format!("LH{}", level + 1),
            x: x + hw,
            y,
            w: hw,
            h: hh,
            level: level as u8,
        });
        // HL — bottom-left (vertical detail)
        nodes.push(SubbandNode {
            name: format!("HL{}", level + 1),
            x,
            y: y + hh,
            w: hw,
            h: hh,
            level: level as u8,
        });
        // HH — bottom-right (diagonal detail)
        nodes.push(SubbandNode {
            name: format!("HH{}", level + 1),
            x: x + hw,
            y: y + hh,
            w: hw,
            h: hh,
            level: level as u8,
        });
        // Recurse on top-left quadrant
        subdivide(nodes, x, y, hw, hh, level + 1, max_levels);
    }

    subdivide(&mut nodes, 0.0, 0.0, 1.0, 1.0, 0, dh);
    TransformMap {
        decomp_h: frame.decomp_h,
        decomp_v: frame.decomp_v,
        nodes,
    }
}

// ─── MCT info ─────────────────────────────────────────────────────────────────

/// MCT visualisation info (static per-frame).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MctInfo {
    pub mct_type: String,
    pub num_comps: u8,
    /// Component labels after MCT (e.g., ["Y","Cb","Cr"]).
    pub comp_labels: Vec<String>,
    /// Whether the transform is reversible.
    pub reversible: bool,
}

pub fn extract_mct_info(frame: &JpegXsFrame) -> MctInfo {
    let (comp_labels, reversible) = match frame.mct_type.as_str() {
        "Rct" => (vec!["Y".into(), "Co".into(), "Cg".into()], true),
        "Ict" => (vec!["Y".into(), "Cb".into(), "Cr".into()], false),
        _ => {
            // No MCT or custom
            (
                (0..frame.num_comps).map(|i| format!("C{}", i)).collect(),
                true,
            )
        }
    };
    MctInfo {
        mct_type: frame.mct_type.clone(),
        num_comps: frame.num_comps,
        comp_labels,
        reversible,
    }
}

// ─── NLT info ─────────────────────────────────────────────────────────────────

/// NLT tone-mapping visualisation info.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NltInfo {
    pub present: bool,
    /// Number of components with NLT.
    pub num_comps: u8,
    /// Type string per component.
    pub comp_types: Vec<String>,
}

pub fn extract_nlt_info(frame: &JpegXsFrame) -> NltInfo {
    if !frame.nlt_present {
        return NltInfo {
            present: false,
            num_comps: 0,
            comp_types: Vec::new(),
        };
    }
    // Without full AEC decoding the per-component NLT types are unknown;
    // report as "present" with generic labels.
    let comp_types = (0..frame.num_comps)
        .map(|i| format!("NLT_C{}", i))
        .collect();
    NltInfo {
        present: true,
        num_comps: frame.num_comps,
        comp_types,
    }
}
