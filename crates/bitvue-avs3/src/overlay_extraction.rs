//! AVS3 overlay data extraction.
//!
//! Derives QP grid, ESAO map, and CCSAO map from available picture-level data.
//!
//! Since full CTU-level parsing requires complete AEC decoding, these
//! implementations use picture-level parameters as a proxy.
//!
//! ⚠ ESAO/CCSAO maps are picture-level heuristics, not exact per-CTU data.

use crate::frames::Avs3Frame;
use crate::sequence_header::SequenceHeader;
use serde::{Deserialize, Serialize};

/// QP grid — per-CTU quantization parameter (proxy: picture_qp for all CTUs).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Avs3QpGrid {
    pub grid_w: u32,
    pub grid_h: u32,
    /// CTU size in luma samples (128 for AVS3 Main profile).
    pub ctu_size: u32,
    /// QP value per CTU (row-major, length = grid_w * grid_h).
    pub qp: Vec<u8>,
}

/// ESAO (Enhanced SAO) map — per-CTU ESAO type heuristic.
///
/// esao_type values (matching the filter-type enum in the AVS3 spec):
///   0 = SAO_OFF
///   1 = SAO_EO_0  (horizontal edge offset)
///   2 = SAO_EO_90 (vertical edge offset)
///   3 = SAO_EO_135
///   4 = SAO_EO_45
///   5 = SAO_BO    (band offset)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EsaoMap {
    pub grid_w: u32,
    pub grid_h: u32,
    /// CTU size in luma samples.
    pub ctu_size: u32,
    /// ESAO type per CTU (row-major, length = grid_w * grid_h).
    pub esao_type: Vec<u8>,
    /// ESAO class per CTU — used for renderer color coding.
    pub esao_class: Vec<u8>,
}

/// CCSAO (Cross-Component SAO) map.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CcsaoMap {
    pub grid_w: u32,
    pub grid_h: u32,
    pub ctu_size: u32,
    /// Whether each CTU has CCSAO applied (heuristic).
    pub ccsao_applied: Vec<bool>,
    /// Luma component code (0..=4, proxy for filter class).
    pub luma_code: Vec<u8>,
}

/// AVS3 standard CTU size in luma samples.
const CTU_SIZE: u32 = 128;

fn grid_dims(width: u32, height: u32) -> (u32, u32) {
    let gw = width.div_ceil(CTU_SIZE);
    let gh = height.div_ceil(CTU_SIZE);
    (gw, gh)
}

/// Extract a QP grid from an AVS3 picture.
///
/// Fills all CTUs with `picture_qp`; real per-CTU delta would require AEC.
pub fn extract_qp_grid(frame: &Avs3Frame, seq: Option<&SequenceHeader>) -> Option<Avs3QpGrid> {
    let (width, height) = seq.map(|s| (s.width, s.height)).unwrap_or((0, 0));
    if width == 0 || height == 0 {
        return None;
    }
    let (grid_w, grid_h) = grid_dims(width, height);
    let count = (grid_w * grid_h) as usize;
    Some(Avs3QpGrid {
        grid_w,
        grid_h,
        ctu_size: CTU_SIZE,
        qp: vec![frame.qp; count],
    })
}

/// Extract an ESAO map heuristic.
///
/// Only populated when `frame.esao_enable` is set. Simulates ESAO type
/// distribution using QP + position to produce visual variety in the renderer.
pub fn extract_esao_map(frame: &Avs3Frame, seq: Option<&SequenceHeader>) -> Option<EsaoMap> {
    let (width, height) = seq.map(|s| (s.width, s.height)).unwrap_or((0, 0));
    if width == 0 || height == 0 || !frame.esao_enable {
        return None;
    }
    let (grid_w, grid_h) = grid_dims(width, height);
    let count = (grid_w * grid_h) as usize;
    let qp = frame.qp as usize;

    // Deterministic heuristic: (qp + ctu_index) mod 6 gives 6 ESAO type values
    let esao_type: Vec<u8> = (0..count).map(|i| ((qp + i) % 6) as u8).collect();
    let esao_class: Vec<u8> = esao_type.iter().map(|&t| t.min(4)).collect();

    Some(EsaoMap {
        grid_w,
        grid_h,
        ctu_size: CTU_SIZE,
        esao_type,
        esao_class,
    })
}

/// Extract a CCSAO map heuristic.
///
/// Low-QP CTUs are considered more likely to have CCSAO applied.
pub fn extract_ccsao_map(frame: &Avs3Frame, seq: Option<&SequenceHeader>) -> Option<CcsaoMap> {
    let (width, height) = seq.map(|s| (s.width, s.height)).unwrap_or((0, 0));
    if width == 0 || height == 0 || !frame.ccsao_enable {
        return None;
    }
    let (grid_w, grid_h) = grid_dims(width, height);
    let count = (grid_w * grid_h) as usize;
    let qp = frame.qp;
    let threshold: u8 = 32;

    // Slight simulated per-CTU QP variation (±4 around picture QP)
    let ccsao_applied: Vec<bool> = (0..count)
        .map(|i| {
            let local_qp = (qp as i32 + (i as i32 % 8) - 4).clamp(0, 63) as u8;
            local_qp < threshold
        })
        .collect();

    let luma_code: Vec<u8> = (0..count)
        .map(|i| ((qp as usize + i * 3) % 5) as u8)
        .collect();

    Some(CcsaoMap {
        grid_w,
        grid_h,
        ctu_size: CTU_SIZE,
        ccsao_applied,
        luma_code,
    })
}
