//! Partition, prediction mode, and transform grid extraction
//!
//! Provides functions to extract partition trees, prediction modes,
//! and transform sizes from AV1 bitstreams.

use std::sync::Arc;

use bitvue_core::{
    partition_grid::{PartitionGrid, PartitionType},
    BitvueError,
};

use super::cache::{compute_cache_key, get_or_parse_coding_units};
use super::parser::ParsedFrame;
use crate::tile::{BlockSize, PredictionMode, TxSize};

/// Extract Partition Grid from AV1 bitstream data
///
/// **Current Implementation**:
/// - Attempts to parse actual partition trees from tile data
/// - Falls back to scaffold grid if parsing fails
/// - Uses SymbolDecoder for entropy decoding
///
/// # Performance
///
/// - O(n) where n = number of superblocks
/// - Falls back to O(1) scaffold if tile data unavailable
pub fn extract_partition_grid(
    obu_data: &[u8],
    _frame_index: usize,
) -> Result<PartitionGrid, BitvueError> {
    let parsed = ParsedFrame::parse(obu_data)?;

    extract_partition_grid_from_parsed(&parsed)
}

/// Extract Partition Grid from cached frame data
///
/// This is more efficient when extracting multiple overlays
/// from the same frame.
///
/// Attempts real partition parsing first, falls back to scaffold.
pub fn extract_partition_grid_from_parsed(
    parsed: &ParsedFrame,
) -> Result<PartitionGrid, BitvueError> {
    // If we have tile data, try to parse actual partitions
    if parsed.has_tile_data() && parsed.tile_data.len() > 10 {
        // Try to parse actual partition trees using SymbolDecoder
        match parse_partition_trees_from_tile_data(parsed) {
            Ok(grid) => {
                tracing::debug!(
                    "Successfully parsed {} actual partition blocks",
                    grid.blocks.len()
                );
                return Ok(grid);
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to parse partitions: {}, falling back to scaffold",
                    e
                );
                // Fall through to scaffold
            }
        }
    }

    // Fallback: Create scaffold partition grid based on superblock layout
    let mut grid = PartitionGrid::new(
        parsed.dimensions.width,
        parsed.dimensions.height,
        parsed.dimensions.sb_size,
    );

    for sb_y in 0..parsed.dimensions.sb_rows {
        for sb_x in 0..parsed.dimensions.sb_cols {
            let sb_pixel_x = sb_x * parsed.dimensions.sb_size;
            let sb_pixel_y = sb_y * parsed.dimensions.sb_size;

            let remaining_w = parsed
                .dimensions
                .sb_size
                .saturating_sub(parsed.dimensions.width.saturating_sub(sb_pixel_x));
            let remaining_h = parsed
                .dimensions
                .sb_size
                .saturating_sub(parsed.dimensions.height.saturating_sub(sb_pixel_y));

            grid.add_block(bitvue_core::partition_grid::PartitionBlock::new(
                sb_pixel_x,
                sb_pixel_y,
                remaining_w,
                remaining_h,
                PartitionType::None,
                0,
            ));
        }
    }

    Ok(grid)
}

/// Parse partition trees from tile data using SymbolDecoder
///
/// Attempts to parse actual partition structures from tile group payload.
fn parse_partition_trees_from_tile_data(
    parsed: &ParsedFrame,
) -> Result<PartitionGrid, BitvueError> {
    let mut grid = PartitionGrid::new(
        parsed.dimensions.width,
        parsed.dimensions.height,
        parsed.dimensions.sb_size,
    );

    // Create SymbolDecoder for tile data
    let mut decoder = crate::SymbolDecoder::new(&parsed.tile_data)?;

    let sb_size = parsed.dimensions.sb_size;
    let block_size = if sb_size == 128 {
        BlockSize::Block128x128
    } else {
        BlockSize::Block64x64
    };

    let is_key_frame = parsed.frame_type.is_intra_only;

    // Parse each superblock
    for sb_y in 0..parsed.dimensions.sb_rows {
        for sb_x in 0..parsed.dimensions.sb_cols {
            let sb_pixel_x = sb_x * sb_size;
            let sb_pixel_y = sb_y * sb_size;

            // Ensure we don't go out of bounds
            let remaining_w = sb_size.min(parsed.dimensions.width.saturating_sub(sb_pixel_x));
            let remaining_h = sb_size.min(parsed.dimensions.height.saturating_sub(sb_pixel_y));

            // Adjust for edge superblocks
            let actual_block_size =
                if remaining_w < block_size.width() || remaining_h < block_size.height() {
                    // Adjust to smaller block size at edges
                    let w = remaining_w.max(block_size.width() / 2);
                    let h = remaining_h.max(block_size.height() / 2);
                    match (w, h) {
                        (w, h) if w <= 32 && h <= 32 => BlockSize::Block32x32,
                        (w, h) if w <= 16 && h <= 16 => BlockSize::Block16x16,
                        (w, h) if w <= 8 && h <= 8 => BlockSize::Block8x8,
                        _ => BlockSize::Block4x4,
                    }
                } else {
                    block_size
                };

            // Try to parse the superblock
            // Note: For MVP, we use default QP=128 and delta_q_enabled=false
            let base_qp = parsed.frame_type.base_qp.unwrap_or(128) as i16;

            // Create MV predictor context (local for partition extraction)
            let mut mv_ctx = crate::tile::MvPredictorContext::new(
                parsed.dimensions.sb_cols,
                parsed.dimensions.sb_rows,
            );

            let sb_result = crate::parse_superblock(
                &mut decoder,
                sb_pixel_x,
                sb_pixel_y,
                actual_block_size.width(),
                is_key_frame,
                base_qp,
                false, // delta_q_enabled - not implemented for MVP
                &mut mv_ctx,
            );

            match sb_result {
                Ok((sb, _final_qp)) => {
                    // Convert partition tree to grid blocks
                    for cu in &sb.coding_units {
                        grid.add_block(bitvue_core::partition_grid::PartitionBlock::new(
                            cu.x,
                            cu.y,
                            cu.width,
                            cu.height,
                            partition_type_from_prediction_mode(cu.mode),
                            0,
                        ));
                    }
                }
                Err(e) => {
                    // On parse error, add scaffold block
                    tracing::warn!(
                        "Failed to parse superblock ({}, {}): {}, using scaffold",
                        sb_pixel_x,
                        sb_pixel_y,
                        e
                    );
                    grid.add_block(bitvue_core::partition_grid::PartitionBlock::new(
                        sb_pixel_x,
                        sb_pixel_y,
                        remaining_w,
                        remaining_h,
                        PartitionType::None,
                        0,
                    ));
                }
            }
        }
    }

    Ok(grid)
}

/// Convert prediction mode to partition type for visualization
fn partition_type_from_prediction_mode(mode: PredictionMode) -> PartitionType {
    match mode {
        PredictionMode::DcPred => PartitionType::None,
        PredictionMode::VPred => PartitionType::Vert,
        PredictionMode::HPred => PartitionType::Horz,
        _ => PartitionType::None,
    }
}

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
        let expected_len = (grid_w * grid_h) as usize;

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
        let idx = (row * self.grid_w + col) as usize;
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
    let block_w = 16u32;
    let block_h = 16u32;
    let grid_w = parsed.dimensions.width.div_ceil(block_w);
    let grid_h = parsed.dimensions.height.div_ceil(block_h);
    let total_blocks = (grid_w * grid_h) as usize;

    let mut modes = Vec::with_capacity(total_blocks);

    // If we have tile data, try to parse actual prediction modes
    if parsed.has_tile_data() && parsed.tile_data.len() > 10 {
        match parse_all_coding_units(parsed) {
            Ok(coding_units) => {
                tracing::debug!(
                    "Extracting prediction modes from {} coding units",
                    coding_units.len()
                );

                // Build a grid of prediction modes from coding units
                for grid_y in 0..grid_h {
                    for grid_x in 0..grid_w {
                        let block_x = grid_x * block_w;
                        let block_y = grid_y * block_h;

                        // Find coding units that overlap with this block
                        let mut found_mode = false;
                        for cu in &coding_units {
                            if cu.x < block_x + block_w
                                && cu.x + cu.width > block_x
                                && cu.y < block_y + block_h
                                && cu.y + cu.height > block_y
                            {
                                // This CU overlaps our block - use its mode
                                modes.push(Some(cu.mode));
                                found_mode = true;
                                break;
                            }
                        }

                        if !found_mode {
                            // No CU found - use default based on frame type
                            let mode = if parsed.frame_type.is_intra_only {
                                get_intra_mode_for_position(grid_x, grid_y)
                            } else {
                                get_inter_mode_for_position(grid_x, grid_y)
                            };
                            modes.push(Some(mode));
                        }
                    }
                }

                return Ok(PredictionModeGrid::new(
                    parsed.dimensions.width,
                    parsed.dimensions.height,
                    block_w,
                    block_h,
                    modes,
                ));
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to parse coding units for prediction modes: {}, using scaffold",
                    e
                );
                // Fall through to scaffold
            }
        }
    }

    // Fallback: Create scaffold prediction mode grid
    let is_intra = parsed.frame_type.is_intra_only;

    for row in 0..grid_h {
        for col in 0..grid_w {
            let mode = if is_intra {
                get_intra_mode_for_position(col, row)
            } else {
                get_inter_mode_for_position(col, row)
            };
            modes.push(Some(mode));
        }
    }

    Ok(PredictionModeGrid::new(
        parsed.dimensions.width,
        parsed.dimensions.height,
        block_w,
        block_h,
        modes,
    ))
}

/// Get INTRA prediction mode for block position
fn get_intra_mode_for_position(col: u32, row: u32) -> PredictionMode {
    const INTRA_MODES: [PredictionMode; 10] = [
        PredictionMode::DcPred,
        PredictionMode::VPred,
        PredictionMode::HPred,
        PredictionMode::D45Pred,
        PredictionMode::D135Pred,
        PredictionMode::D113Pred,
        PredictionMode::D157Pred,
        PredictionMode::D203Pred,
        PredictionMode::SmoothPred,
        PredictionMode::PaethPred,
    ];

    let idx = ((col as usize) + (row as usize) * 3) % INTRA_MODES.len();
    INTRA_MODES[idx]
}

/// Get INTER prediction mode for block position
fn get_inter_mode_for_position(col: u32, row: u32) -> PredictionMode {
    const INTER_MODES: [PredictionMode; 4] = [
        PredictionMode::NewMv,
        PredictionMode::NearestMv,
        PredictionMode::NearMv,
        PredictionMode::GlobalMv,
    ];

    let idx = ((col as usize) + (row as usize)) % INTER_MODES.len();
    INTER_MODES[idx]
}

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
        let expected_len = (grid_w * grid_h) as usize;

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
        let idx = (row * self.grid_w + col) as usize;
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
    let block_w = 16u32;
    let block_h = 16u32;
    let grid_w = parsed.dimensions.width.div_ceil(block_w);
    let grid_h = parsed.dimensions.height.div_ceil(block_h);
    let total_blocks = (grid_w * grid_h) as usize;

    let mut tx_sizes = Vec::with_capacity(total_blocks);

    // If we have tile data, try to parse actual transform sizes
    if parsed.has_tile_data() && parsed.tile_data.len() > 10 {
        match parse_all_coding_units(parsed) {
            Ok(coding_units) => {
                tracing::debug!(
                    "Extracting transform sizes from {} coding units",
                    coding_units.len()
                );

                // Build a grid of transform sizes from coding units
                for grid_y in 0..grid_h {
                    for grid_x in 0..grid_w {
                        let block_x = grid_x * block_w;
                        let block_y = grid_y * block_h;

                        // Find coding units that overlap with this block
                        let mut found_tx = false;
                        for cu in &coding_units {
                            if cu.x < block_x + block_w
                                && cu.x + cu.width > block_x
                                && cu.y < block_y + block_h
                                && cu.y + cu.height > block_y
                            {
                                // This CU overlaps our block - use its transform size
                                tx_sizes.push(Some(cu.tx_size));
                                found_tx = true;
                                break;
                            }
                        }

                        if !found_tx {
                            // No CU found - use default based on block size
                            tx_sizes.push(Some(get_transform_size_for_position(grid_x, grid_y)));
                        }
                    }
                }

                return Ok(TransformGrid::new(
                    parsed.dimensions.width,
                    parsed.dimensions.height,
                    block_w,
                    block_h,
                    tx_sizes,
                ));
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to parse coding units for transform sizes: {}, using scaffold",
                    e
                );
                // Fall through to scaffold
            }
        }
    }

    // Fallback: Create scaffold transform size grid
    for row in 0..grid_h {
        for col in 0..grid_w {
            tx_sizes.push(Some(get_transform_size_for_position(col, row)));
        }
    }

    Ok(TransformGrid::new(
        parsed.dimensions.width,
        parsed.dimensions.height,
        block_w,
        block_h,
        tx_sizes,
    ))
}

/// Get transform size for block position
fn get_transform_size_for_position(col: u32, row: u32) -> TxSize {
    // Bias towards 16x16 and 8x8 (most common in practice)
    let sum = (col + row) as usize;
    match sum % 4 {
        0 => TxSize::Tx16x16,
        1 => TxSize::Tx8x8,
        2 => TxSize::Tx16x16,
        _ => TxSize::Tx4x4,
    }
}

/// Parse all coding units from tile data
///
/// Per optimize-code skill: Uses thread-safe LRU cache to avoid re-parsing
/// the same tile data when extracting multiple overlays.
///
/// Returns a vector of all coding units parsed from the tile group data.
/// This is used by prediction mode and transform grid extraction.
fn parse_all_coding_units(
    parsed: &ParsedFrame,
) -> Result<Vec<crate::tile::CodingUnit>, BitvueError> {
    let base_qp = parsed.frame_type.base_qp.unwrap_or(128) as i16;
    let cache_key = compute_cache_key(&parsed.tile_data, base_qp);

    // Clone data needed for parsing (move into closure)
    let tile_data = Arc::clone(&parsed.tile_data);
    let sb_size = parsed.dimensions.sb_size;
    let sb_cols = parsed.dimensions.sb_cols;
    let sb_rows = parsed.dimensions.sb_rows;
    let is_key_frame = parsed.frame_type.is_intra_only;
    let delta_q_enabled = parsed.delta_q_enabled;

    // Per optimize-code skill: Use get_or_parse helper for cache pattern
    get_or_parse_coding_units(cache_key, || {
        let mut all_cus = Vec::new();

        // Pre-allocate capacity based on superblock count (per optimize-code)
        let estimated_cus = (sb_cols * sb_rows) as usize * 4;
        all_cus.reserve(estimated_cus);

        // Create SymbolDecoder for tile data
        let mut decoder = crate::SymbolDecoder::new(&tile_data)?;

        // Track running QP value across superblocks
        let mut current_qp = base_qp;

        // Create MV predictor context
        let mut mv_ctx = crate::tile::MvPredictorContext::new(sb_cols, sb_rows);

        // Parse each superblock
        for sb_y in 0..sb_rows {
            for sb_x in 0..sb_cols {
                let sb_pixel_x = sb_x * sb_size;
                let sb_pixel_y = sb_y * sb_size;

                // Try to parse the superblock
                match crate::parse_superblock(
                    &mut decoder,
                    sb_pixel_x,
                    sb_pixel_y,
                    sb_size,
                    is_key_frame,
                    current_qp,
                    delta_q_enabled,
                    &mut mv_ctx,
                ) {
                    Ok((sb, new_qp)) => {
                        // Collect all coding units from this superblock
                        all_cus.extend(sb.coding_units);
                        current_qp = new_qp;
                    }
                    Err(e) => {
                        tracing::debug!(
                            "Failed to parse superblock ({}, {}): {}, skipping",
                            sb_pixel_x,
                            sb_pixel_y,
                            e
                        );
                        // Continue parsing other superblocks
                    }
                }
            }
        }

        tracing::debug!(
            "Parsed {} coding units from tile data (final QP: {})",
            all_cus.len(),
            current_qp
        );
        Ok(all_cus)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tx_size_values() {
        // Assert TxSize enum values match expected sizes
        assert_eq!(TxSize::Tx4x4.size(), 4);
        assert_eq!(TxSize::Tx8x8.size(), 8);
        assert_eq!(TxSize::Tx16x16.size(), 16);
        assert_eq!(TxSize::Tx32x32.size(), 32);
        assert_eq!(TxSize::Tx64x64.size(), 64);
    }

    #[test]
    fn test_get_intra_mode_deterministic() {
        // Act
        let mode1 = get_intra_mode_for_position(5, 10);
        let mode2 = get_intra_mode_for_position(5, 10);

        // Assert: Same position should give same mode
        assert_eq!(mode1, mode2, "Should return same mode for same position");
    }

    #[test]
    fn test_get_inter_mode_deterministic() {
        // Act
        let mode1 = get_inter_mode_for_position(3, 7);
        let mode2 = get_inter_mode_for_position(3, 7);

        // Assert: Same position should give same mode
        assert_eq!(mode1, mode2, "Should return same mode for same position");
    }

    #[test]
    fn test_get_transform_size_deterministic() {
        let tx1 = get_transform_size_for_position(2, 4);
        let tx2 = get_transform_size_for_position(2, 4);
        assert_eq!(tx1, tx2, "Should return same size for same position");
    }

    #[test]
    fn test_partition_grid_fallback() {
        // Arrange: Empty OBU data (should use scaffold)
        let obu_data = vec![0x00, 0x01, 0x02, 0x03];

        // Act
        let result = extract_partition_grid(&obu_data, 0);

        // Assert: Should create scaffold grid
        assert!(
            result.is_ok(),
            "Partition grid extraction should succeed with fallback"
        );
        let grid = result.unwrap();
        assert!(grid.blocks.len() > 0, "Grid should have scaffold blocks");
    }

    #[test]
    fn test_prediction_mode_grid_bounds() {
        let grid = PredictionModeGrid::new(
            1920,
            1080,
            16,
            16,
            vec![Some(PredictionMode::DcPred); (120 * 68) as usize],
        );

        // Valid bounds
        assert!(grid.get(0, 0).is_some());
        assert!(grid.get(119, 67).is_some());

        // Out of bounds
        assert!(grid.get(120, 0).is_none());
        assert!(grid.get(0, 68).is_none());
    }
}
