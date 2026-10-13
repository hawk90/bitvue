//! The prediction mode of each grid block of a frame.

use super::super::cu_parser::parse_all_coding_units;
use super::super::parser::ParsedFrame;
use super::grid_build::build_grid_from_coding_units_spatial;
use crate::tile::PredictionMode;
use bitvue_engine::{
    limits::AV1_BLOCK_SIZE, limits::MAX_GRID_BLOCKS, limits::MAX_GRID_DIMENSION, BitvueError,
};

/// Prediction Mode Grid for visualization
#[derive(Debug, Clone)]
pub struct PredictionModeGrid {
    /// Coded frame width in pixels
    pub coded_width: u32,
    /// Coded frame height in pixels
    pub coded_height: u32,
    /// Block width in pixels
    pub block_w: u32,
    /// Block height in pixels
    pub block_h: u32,
    /// Grid width in blocks
    pub grid_w: u32,
    /// Grid height in blocks
    pub grid_h: u32,
    /// Prediction mode for each block (row-major order)
    pub modes: Vec<Option<PredictionMode>>,
}

impl PredictionModeGrid {
    /// Create a new prediction mode grid
    pub fn new(
        coded_width: u32,
        coded_height: u32,
        block_w: u32,
        block_h: u32,
        modes: Vec<Option<PredictionMode>>,
    ) -> Self {
        let grid_w = coded_width.div_ceil(block_w);
        let grid_h = coded_height.div_ceil(block_h);

        // Check for overflow in grid size calculation
        // Max reasonable grid is 8Kx8K with 16x16 blocks = 512x512 blocks
        let expected_len = if grid_w > MAX_GRID_DIMENSION || grid_h > MAX_GRID_DIMENSION {
            // Grid is too large, use modes length as-is
            modes.len()
        } else {
            // Safe to multiply (both values checked above)
            (grid_w * grid_h) as usize
        };

        debug_assert_eq!(
            modes.len(),
            expected_len,
            "PredictionModeGrid: modes length mismatch: expected {}, got {}",
            expected_len,
            modes.len()
        );

        Self {
            coded_width,
            coded_height,
            block_w,
            block_h,
            grid_w,
            grid_h,
            modes,
        }
    }

    /// Get prediction mode at block position
    pub fn get(&self, col: u32, row: u32) -> Option<PredictionMode> {
        if col >= self.grid_w || row >= self.grid_h {
            return None;
        }
        // Check for overflow in index calculation before casting
        let idx = (row as usize)
            .checked_mul(self.grid_w as usize)
            .and_then(|v| v.checked_add(col as usize))?;
        self.modes.get(idx).copied().flatten()
    }
}

/// Extract Prediction Mode Grid from AV1 bitstream data
///
/// **Current Implementation**: Uses frame type to generate modes.
/// Full implementation would parse actual modes from tile data.
pub fn extract_prediction_mode_grid(
    obu_data: &[u8],
    _frame_index: usize,
) -> Result<PredictionModeGrid, BitvueError> {
    let parsed = ParsedFrame::parse(obu_data)?;

    extract_prediction_mode_grid_from_parsed(&parsed)
}

/// Extract Prediction Mode Grid from cached frame data
///
/// **Current Implementation**:
/// - Parses tile data to extract actual prediction modes from coding units
/// - Falls back to scaffold if tile data unavailable or parsing fails
/// - Uses actual INTRA/INTER modes from AV1 bitstream
pub fn extract_prediction_mode_grid_from_parsed(
    parsed: &ParsedFrame,
) -> Result<PredictionModeGrid, BitvueError> {
    let block_w = AV1_BLOCK_SIZE;
    let block_h = AV1_BLOCK_SIZE;
    let grid_w = parsed.dimensions.width.div_ceil(block_h);
    let grid_h = parsed.dimensions.height.div_ceil(block_h);

    // Check for overflow and validate grid dimensions
    let total_blocks = match grid_w.checked_mul(grid_h) {
        Some(product) => product as usize,
        None => {
            return Err(BitvueError::Decode(format!(
                "Grid dimensions too large: {}x{}",
                grid_w, grid_h
            )))
        }
    };

    if total_blocks > MAX_GRID_BLOCKS {
        return Err(BitvueError::Decode(format!(
            "Grid exceeds maximum size: {}x{} = {} blocks",
            grid_w, grid_h, total_blocks
        )));
    }

    if !super::super::provenance::has_decodable_tile(parsed) {
        return Err(super::super::provenance::no_decodable_tile());
    }
    let coding_units = parse_all_coding_units(parsed)?;
    tracing::debug!(
        "Extracting prediction modes from {} coding units",
        coding_units.len()
    );
    let mut modes = Vec::with_capacity(total_blocks);
    build_grid_from_coding_units_spatial(
        &coding_units,
        parsed,
        block_w,
        block_h,
        &mut modes,
        |cu| cu.mode,
    )?;
    Ok(PredictionModeGrid::new(
        parsed.dimensions.width,
        parsed.dimensions.height,
        block_w,
        block_h,
        modes,
    ))
}
