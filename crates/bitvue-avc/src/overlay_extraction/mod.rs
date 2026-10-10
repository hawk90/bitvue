//! Overlay data extraction from H.264/AVC bitstreams
//!
//! This module provides functions to extract QP heatmap, motion vector,
//! and macroblock information for visualization overlays.
//!
//! ## Implementation Status (v0.4.x)
//!
//! **Real Data Extraction**:
//! - ✅ Extract macroblock structure from slice data
//! - ✅ Extract motion vectors from INTER macroblocks
//! - ✅ Extract QP values from macroblock layer
//! - ✅ Extract macroblock types (I/P/Skip/B)
//! - ✅ Extract reference frame indices
//!
//! ## Data Flow
//!
//! 1. **NAL Units** → find_nal_units() → Vec<NalUnit>
//! 2. **Slice Data** → parse_macroblocks() → Vec<Macroblock>
//! 3. **Macroblocks** → extract_*_grid() → overlay grids

mod cabac;
mod cavlc;
mod grids;
mod slice_mbs;
mod types;

#[cfg(test)]
mod tests;

pub use grids::{
    extract_mb_type_grid, extract_mv_grid, extract_partition_grid, extract_prediction_mode_grid,
    extract_qp_grid, extract_ref_idx_grid,
};
pub use types::{Macroblock, MbType, MotionVector};
