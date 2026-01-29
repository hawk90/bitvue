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

/// Spatial index for O(1) coding unit lookup by grid position
///
/// Pre-computes which coding unit overlaps each grid cell, eliminating
/// the need for O(n) linear search per block. For 1080p, this reduces
/// 510×1000 = 510,000 comparisons to just 510 lookups.
struct CuSpatialIndex {
    /// Grid of CU indices (one per grid cell)
    /// Vec index = grid_y * grid_w + grid_x
    /// Value = Some(cu_index) or None if no CU covers this cell
    grid: Vec<Option<usize>>,
    grid_w: u32,
}

impl CuSpatialIndex {
    /// Build spatial index from coding units
    ///
    /// Pre-processes all coding units to determine which grid cell(s) they overlap.
    /// Complexity: O(n * k) where n=CU count, k=average cells per CU (typically ~4)
    fn new(
        coding_units: &[crate::tile::CodingUnit],
        grid_w: u32,
        grid_h: u32,
        block_w: u32,
        block_h: u32,
    ) -> Self {
        let total_cells = (grid_w * grid_h) as usize;
        let mut grid = vec![None; total_cells];

        // For each coding unit, mark all grid cells it overlaps
        for (cu_idx, cu) in coding_units.iter().enumerate() {
            // Calculate grid cell range covered by this CU
            let start_grid_x = cu.x / block_w;
            let start_grid_y = cu.y / block_h;
            let end_grid_x = ((cu.x + cu.width - 1) / block_w).min(grid_w - 1);
            let end_grid_y = ((cu.y + cu.height - 1) / block_h).min(grid_h - 1);

            // Mark all overlapping cells
            for grid_y in start_grid_y..=end_grid_y {
                for grid_x in start_grid_x..=end_grid_x {
                    let cell_idx = (grid_y * grid_w + grid_x) as usize;
                    // First CU wins (earlier CUs take precedence)
                    if grid[cell_idx].is_none() {
                        grid[cell_idx] = Some(cu_idx);
                    }
                }
            }
        }

        Self { grid, grid_w }
    }

    /// Get coding unit index for a grid cell (O(1) lookup)
    #[inline]
    fn get_cu_index(&self, grid_x: u32, grid_y: u32) -> Option<usize> {
        let cell_idx = (grid_y * self.grid_w + grid_x) as usize;
        self.grid.get(cell_idx).copied().flatten()
    }
}

/// Helper: Build QP grid from coding units using spatial index
///
/// Creates a QP grid by using a spatial index for O(1) coding unit lookup.
/// Eliminates O(n²) linear search bottleneck (510k→510 lookups for 1080p).
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

    // Build spatial index (O(n×k) where k=cells per CU, typically ~4)
    let spatial_index = CuSpatialIndex::new(coding_units, grid_w, grid_h, block_w, block_h);

    // Populate QP grid using O(1) lookups instead of O(n) searches
    for grid_y in 0..grid_h {
        for grid_x in 0..grid_w {
            let cu_qp = spatial_index
                .get_cu_index(grid_x, grid_y)
                .map(|cu_idx| coding_units[cu_idx].effective_qp(base_qp))
                .unwrap_or(base_qp);

            qp.push(cu_qp);
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
