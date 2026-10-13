//! The transform size of each grid block of a frame.

use super::super::cu_parser::parse_all_coding_units;
use super::super::parser::ParsedFrame;
use super::grid_build::build_grid_from_coding_units_spatial;
use crate::tile::TxSize;
use bitvue_engine::{limits::AV1_BLOCK_SIZE, limits::MAX_GRID_BLOCKS, BitvueError};

/// Transform Grid for visualization
#[derive(Debug, Clone)]
pub struct TransformGrid {
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
    /// Transform size for each block
    pub tx_sizes: Vec<Option<TxSize>>,
}

impl TransformGrid {
    /// Create a new transform grid
    pub fn new(
        coded_width: u32,
        coded_height: u32,
        block_w: u32,
        block_h: u32,
        tx_sizes: Vec<Option<TxSize>>,
    ) -> Self {
        let grid_w = coded_width.div_ceil(block_w);
        let grid_h = coded_height.div_ceil(block_h);

        // Check for overflow in grid size calculation
        const MAX_GRID_SIZE: u32 = 512 * 512;
        let expected_len = if grid_w > MAX_GRID_SIZE || grid_h > MAX_GRID_SIZE {
            tx_sizes.len()
        } else {
            (grid_w * grid_h) as usize
        };

        debug_assert_eq!(
            tx_sizes.len(),
            expected_len,
            "TransformGrid: tx_sizes length mismatch: expected {}, got {}",
            expected_len,
            tx_sizes.len()
        );

        Self {
            coded_width,
            coded_height,
            block_w,
            block_h,
            grid_w,
            grid_h,
            tx_sizes,
        }
    }

    /// Get transform size at block position
    pub fn get(&self, col: u32, row: u32) -> Option<TxSize> {
        if col >= self.grid_w || row >= self.grid_h {
            return None;
        }
        // Check for overflow in index calculation before casting
        let idx = (row as usize)
            .checked_mul(self.grid_w as usize)
            .and_then(|v| v.checked_add(col as usize))?;
        self.tx_sizes.get(idx).copied().flatten()
    }
}

/// Extract Transform Grid from AV1 bitstream data
///
/// **Current Implementation**:
/// - Parses tile data to extract actual transform sizes from coding units
/// - Falls back to scaffold if tile data unavailable or parsing fails
/// - Uses actual transform sizes from AV1 bitstream
pub fn extract_transform_grid(
    obu_data: &[u8],
    _frame_index: usize,
) -> Result<TransformGrid, BitvueError> {
    let parsed = ParsedFrame::parse(obu_data)?;

    extract_transform_grid_from_parsed(&parsed)
}

/// Extract Transform Grid from cached frame data
///
/// **Current Implementation**:
/// - Parses tile data to extract actual transform sizes from coding units
/// - Falls back to scaffold if tile data unavailable or parsing fails
/// - Uses actual transform sizes from AV1 bitstream
pub fn extract_transform_grid_from_parsed(
    parsed: &ParsedFrame,
) -> Result<TransformGrid, BitvueError> {
    let block_w = AV1_BLOCK_SIZE;
    let block_h = AV1_BLOCK_SIZE;
    let grid_w = parsed.dimensions.width.div_ceil(block_w);
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
        "Extracting transform sizes from {} coding units",
        coding_units.len()
    );
    let mut tx_sizes = Vec::with_capacity(total_blocks);
    build_grid_from_coding_units_spatial(
        &coding_units,
        parsed,
        block_w,
        block_h,
        &mut tx_sizes,
        |cu| cu.tx_size,
    )?;
    Ok(TransformGrid::new(
        parsed.dimensions.width,
        parsed.dimensions.height,
        block_w,
        block_h,
        tx_sizes,
    ))
}
