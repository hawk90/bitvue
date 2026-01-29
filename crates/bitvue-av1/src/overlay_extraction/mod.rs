//! Overlay data extraction from AV1 bitstreams
//!
//! This module provides functions to extract QP heatmap, motion vector,
//! and pixel information data for visualization overlays and tooltips.
//!
//! ## Implementation Status (v0.3.x) ✅ COMPLETE
//!
//! **Real Data Extraction**:
//! - ✅ Parse tile groups from OBU data (cached)
//! - ✅ Parse partition trees using symbol decoder
//! - ✅ Extract prediction modes from bitstream (actual modes from coding units)
//! - ✅ Extract motion vectors for INTER blocks (quarter-pel precision from coding units)
//! - ✅ Extract QP values from coding units (with fallback to base_q_idx)
//! - ✅ Extract transform sizes from coding units (Tx4x4 to Tx64x64)
//!
//! ## Performance Optimizations (v0.3.1)
//!
//! - **Single-pass OBU parsing**: Parse OBUs once and cache results
//! - **Arc-based sharing**: Avoid unnecessary data copies
//! - **Lazy evaluation**: Only parse what's needed
//! - **Efficient lookups**: Use iterators for OBA traversal
//! - **Thread-safe LRU caching**: Cache parsed coding units per frame
//!
//! ## Data Flow
//!
//! 1. **OBU Data** → parse_frame_data() → ParsedFrame (cached)
//! 2. **ParsedFrame** → extract_*_grid() → overlay grids
//! 3. **Tile Data** → parse_partition_tree → partition structure
//! 4. **Superblock** → CodingUnit → actual prediction mode, MV, QP, TxSize

mod cache;
mod parser;
mod qp_extractor;
mod mv_extractor;
mod partition;

// Re-export public API
pub use parser::{extract_pixel_info, FrameDimensions, FrameTypeInfo, ObuRef, ParsedFrame, PixelInfo};
pub use qp_extractor::{extract_qp_grid, extract_qp_grid_from_parsed};
pub use mv_extractor::{extract_mv_grid, extract_mv_grid_from_parsed};
pub use partition::{
    extract_partition_grid, extract_partition_grid_from_parsed,
    extract_prediction_mode_grid, extract_prediction_mode_grid_from_parsed,
    extract_transform_grid, extract_transform_grid_from_parsed,
    PredictionModeGrid, TransformGrid,
};

// Test utilities (only available in tests)
#[cfg(test)]
pub use cache::{clear_cu_cache, cu_cache_size};

// Re-export Obu for public API
pub use crate::obu::Obu;
