//! QP (Quantization Parameter) grid extraction
//!
//! Provides functions to extract QP heatmap data from AV1 bitstreams.

use std::sync::Arc;

use bitvue_core::qp_heatmap::QPGrid;
use bitvue_core::BitvueError;

use super::cache::{compute_cache_key, get_or_parse_coding_units};
use super::parser::ParsedFrame;

/// Extract QP Grid from AV1 bitstream data
///
/// **Current Implementation**: Uses base_q_idx from frame header for all blocks.
/// Full QP extraction requires parsing quantization_params() and delta Q values
/// from each coding unit.
///
/// # Performance
///
/// - O(1) when using cached ParsedFrame
/// - O(n) grid creation where n = number of blocks
pub fn extract_qp_grid(
    obu_data: &[u8],
    _frame_index: usize,
    base_qp: i16,
) -> Result<QPGrid, BitvueError> {
    let parsed = ParsedFrame::parse(obu_data)?;

    let block_w = 64u32;
    let block_h = 64u32;
    let grid_w = parsed.dimensions.width.div_ceil(block_w);
    let grid_h = parsed.dimensions.height.div_ceil(block_h);

    let qp = vec![base_qp; (grid_w * grid_h) as usize];

    Ok(QPGrid::new(grid_w, grid_h, block_w, block_h, qp, base_qp))
}

/// Extract QP Grid from cached frame data
///
/// **Current Implementation**:
/// - Parses tile data to extract actual QP values from coding units
/// - Falls back to base_q_idx if tile data unavailable or parsing fails
/// - Uses actual QP values from AV1 bitstream
///
/// This is more efficient when extracting multiple overlays
/// from the same frame.
pub fn extract_qp_grid_from_parsed(
    parsed: &ParsedFrame,
    _frame_index: usize,
    base_qp: i16,
) -> Result<QPGrid, BitvueError> {
    let block_w = 64u32;
    let block_h = 64u32;
    let grid_w = parsed.dimensions.width.div_ceil(block_w);
    let grid_h = parsed.dimensions.height.div_ceil(block_h);
    let total_blocks = (grid_w * grid_h) as usize;

    // If we have tile data, try to parse actual QP values
    if parsed.has_tile_data() && parsed.tile_data.len() > 10 {
        match parse_all_coding_units(parsed) {
            Ok(coding_units) => {
                tracing::debug!(
                    "Extracting QP values from {} coding units",
                    coding_units.len()
                );
                let qp = build_qp_grid_from_cus(
                    &coding_units,
                    grid_w,
                    grid_h,
                    block_w,
                    block_h,
                    base_qp,
                );
                return Ok(QPGrid::new(grid_w, grid_h, block_w, block_h, qp, base_qp));
            }
            Err(e) => {
                tracing::warn!("Failed to parse coding units for QP: {}, using base_qp", e);
                // Fall through to scaffold
            }
        }
    }

    // Fallback: Use base_q_idx for all blocks
    let qp = vec![base_qp; total_blocks];

    Ok(QPGrid::new(grid_w, grid_h, block_w, block_h, qp, base_qp))
}

/// Helper: Find QP value for a block from coding units
///
/// Searches through coding units to find one that overlaps with the given block
/// and returns its effective QP value. Returns None if no overlapping CU found.
fn find_overlapping_cu_qp(
    coding_units: &[crate::tile::CodingUnit],
    block_x: u32,
    block_y: u32,
    block_w: u32,
    base_qp: i16,
) -> Option<i16> {
    coding_units
        .iter()
        .find(|cu| {
            cu.x < block_x + block_w
                && cu.x + cu.width > block_x
                && cu.y < block_y + block_w
                && cu.y + cu.height > block_y
        })
        .map(|cu| cu.effective_qp(base_qp))
}

/// Helper: Build QP grid from coding units
///
/// Creates a QP grid by finding overlapping coding units for each block.
/// Reduces nesting depth in extract_qp_grid_from_parsed.
fn build_qp_grid_from_cus(
    coding_units: &[crate::tile::CodingUnit],
    grid_w: u32,
    grid_h: u32,
    block_w: u32,
    block_h: u32,
    base_qp: i16,
) -> Vec<i16> {
    let total_blocks = (grid_w * grid_h) as usize;
    let mut qp = Vec::with_capacity(total_blocks);

    for grid_y in 0..grid_h {
        for grid_x in 0..grid_w {
            let block_x = grid_x * block_w;
            let block_y = grid_y * block_h;

            let cu_qp = find_overlapping_cu_qp(coding_units, block_x, block_y, block_w, base_qp);
            qp.push(cu_qp.unwrap_or(base_qp));
        }
    }

    qp
}

/// Parse all coding units from tile data
///
/// Per optimize-code skill: Uses thread-safe LRU cache to avoid re-parsing
/// the same tile data when extracting multiple overlays.
///
/// Returns a vector of all coding units parsed from the tile group data.
/// This is used by QP, MV, and prediction mode grid extraction.
fn parse_all_coding_units(
    parsed: &ParsedFrame,
) -> Result<Vec<crate::tile::CodingUnit>, BitvueError> {
    let base_qp = parsed.frame_type.base_qp.unwrap_or(128) as i16;
    let cache_key = compute_cache_key(&parsed.tile_data, base_qp);

    // Arc-clone is cheap (just reference count increment, no data copy)
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

    fn create_test_obu_data() -> Vec<u8> {
        // Minimal OBU data with sequence header and frame header
        let mut data = Vec::new();

        // Temporal delimiter OBU (type 2, size 0)
        data.extend_from_slice(&[0x12, 0x00]);

        // Sequence header OBU (type 1, size ~20)
        data.extend_from_slice(&[0x0A, 0x14]); // OBU header
        data.extend_from_slice(&[0x00u8; 20]); // Payload placeholder

        // Frame header OBU (type 3, size ~10)
        data.extend_from_slice(&[0x1A, 0x0A]); // OBU header
        data.extend_from_slice(&[0x00u8; 10]); // Payload placeholder

        data
    }

    #[test]
    fn test_qp_grid_with_valid_data() {
        // Arrange
        let obu_data = create_test_obu_data();
        let base_qp: i16 = 32;

        // Act
        let result = extract_qp_grid(&obu_data, 0, base_qp);

        // Assert: Should create a grid with default dimensions
        assert!(result.is_ok(), "QP grid extraction should succeed");
        let grid = result.unwrap();
        assert_eq!(grid.block_w, 64);
        assert_eq!(grid.block_h, 64);
        assert!(grid.qp.len() > 0, "QP grid should have values");
        assert_eq!(grid.qp[0], base_qp, "First block should have base QP");
    }

    #[test]
    fn test_qp_grid_coverage_calculation() {
        // Arrange: Create grid with some missing values
        let grid_w = 4;
        let grid_h = 3;
        let mut qp = vec![32i16; 12];
        qp[2] = -1; // Missing value
        qp[5] = -1; // Missing value
        qp[8] = -1; // Missing value

        // Act
        let grid = QPGrid::new(grid_w, grid_h, 64, 64, qp, -1);

        // Assert: Coverage should exclude missing values
        let coverage = grid.coverage_percent();
        assert_eq!(coverage, 75.0, "Coverage should be 75% (9/12 valid)");
    }

    #[test]
    fn test_qp_grid_bounds_checking() {
        // Arrange
        let qp = vec![32i16; 12];
        let grid = QPGrid::new(4, 3, 64, 64, qp, -1);

        // Act & Assert: Valid bounds
        assert_eq!(grid.get(0, 0), Some(32));
        assert_eq!(grid.get(3, 2), Some(32));

        // Act & Assert: Out of bounds
        assert_eq!(
            grid.get(4, 0),
            None,
            "Should return None for out of bounds (x)"
        );
        assert_eq!(
            grid.get(0, 3),
            None,
            "Should return None for out of bounds (y)"
        );
    }
}
