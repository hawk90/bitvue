//! Overlay data extraction for VC-3/DNxHD frames.

use crate::frames::Vc3Frame;
use serde::{Deserialize, Serialize};

/// Macroblock grid with per-MB estimated bit cost.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MbGrid {
    pub cols: u32,
    pub rows: u32,
    /// Estimated bits per macroblock (uniform distribution).
    pub bits: Vec<u32>,
}

pub fn extract_mb_grid(frame: &Vc3Frame) -> Option<MbGrid> {
    if frame.mb_cols == 0 || frame.mb_rows == 0 {
        return None;
    }
    let n = (frame.mb_cols * frame.mb_rows) as usize;
    let bits = vec![frame.bits_per_mb; n];
    Some(MbGrid {
        cols: frame.mb_cols,
        rows: frame.mb_rows,
        bits,
    })
}
