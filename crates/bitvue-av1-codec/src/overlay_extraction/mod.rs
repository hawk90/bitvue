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
//! ## Error Handling Strategy
//!
//! Extraction functions use a best-effort approach by default:
//! - Try to parse actual data from tile data
//! - Fall back to scaffold/estimated data if parsing fails
//! - Log warnings for debugging
//!
//! For testing/debugging, strict mode can be enabled via:
//! ```ignore
//! use bitvue_av1::overlay_extraction::set_strict_mode;
//! set_strict_mode(true);  // Errors propagate instead of silent fallback
//! ```

mod cache;
mod cu_parser;
mod mv_extractor;
mod parser;
mod partition;
mod qp_extractor;

use std::sync::atomic::{AtomicBool, Ordering};

/// Global strict mode flag for overlay extraction
///
/// When enabled, parse errors will propagate instead of silently
/// falling back to scaffold data. Useful for testing and debugging.
static STRICT_MODE: AtomicBool = AtomicBool::new(false);

/// Enable or disable strict mode for overlay extraction
///
/// When strict mode is enabled:
/// - Parse errors will be returned instead of logged
/// - No fallback to scaffold/estimated data
/// - Useful for testing and debugging
///
/// # Example
/// ```ignore
/// use bitvue_av1::overlay_extraction::set_strict_mode;
/// set_strict_mode(true);
/// let result = extract_prediction_mode_grid_from_parsed(&parsed);
/// // If parsing fails, result will be Err instead of Ok with scaffold
/// ```
pub fn set_strict_mode(enabled: bool) {
    STRICT_MODE.store(enabled, Ordering::SeqCst);
}

/// Check if strict mode is enabled
pub fn strict_mode_enabled() -> bool {
    STRICT_MODE.load(Ordering::Relaxed)
}

// Re-export public API
pub use mv_extractor::{extract_mv_grid, extract_mv_grid_from_parsed};
pub use parser::{
    extract_pixel_info, FrameDimensions, FrameTypeInfo, ObuRef, ParsedFrame, PixelInfo,
};
pub use partition::{
    extract_partition_grid, extract_partition_grid_from_parsed, extract_prediction_mode_grid,
    extract_prediction_mode_grid_from_parsed, extract_transform_grid,
    extract_transform_grid_from_parsed, PredictionModeGrid, TransformGrid,
};
pub use qp_extractor::{extract_qp_grid, extract_qp_grid_from_parsed};

// Test utilities (only available in tests)
#[cfg(test)]
pub use cache::{clear_cu_cache, cu_cache_size};

// Re-export Obu for public API
pub use crate::obu::Obu;
