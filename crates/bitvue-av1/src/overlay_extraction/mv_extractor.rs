//! Motion Vector grid extraction
//!
//! Provides functions to extract motion vector data from AV1 bitstreams.

use std::sync::Arc;

use bitvue_core::{
    mv_overlay::{BlockMode, MVGrid, MotionVector as CoreMV},
    BitvueError,
};

use super::cache::{compute_cache_key, get_or_parse_coding_units};
use super::parser::ParsedFrame;

/// Spatial index for O(1) coding unit lookup by grid position
///
/// Pre-computes which coding unit overlaps each grid cell, eliminating
/// the need for O(n) linear search per block.
struct CuSpatialIndex {
    grid: Vec<Option<usize>>,
    grid_w: u32,
}

impl CuSpatialIndex {
    #[inline]
    fn get_cu_index(&self, grid_x: u32, grid_y: u32) -> Option<usize> {
        let cell_idx = (grid_y * self.grid_w + grid_x) as usize;
        self.grid.get(cell_idx).copied().flatten()
    }
}

/// Build spatial index from coding units
///
/// Complexity: O(n×k) where n=CU count, k=average cells per CU (~4)
fn build_cu_spatial_index(
    coding_units: &[crate::tile::CodingUnit],
    grid_w: u32,
    grid_h: u32,
    block_w: u32,
    block_h: u32,
) -> CuSpatialIndex {
    let total_cells = (grid_w * grid_h) as usize;
    let mut grid = vec![None; total_cells];

    for (cu_idx, cu) in coding_units.iter().enumerate() {
        let start_grid_x = cu.x / block_w;
        let start_grid_y = cu.y / block_h;
        let end_grid_x = ((cu.x + cu.width - 1) / block_w).min(grid_w - 1);
        let end_grid_y = ((cu.y + cu.height - 1) / block_h).min(grid_h - 1);

        for grid_y in start_grid_y..=end_grid_y {
            for grid_x in start_grid_x..=end_grid_x {
                let cell_idx = (grid_y * grid_w + grid_x) as usize;
                if grid[cell_idx].is_none() {
                    grid[cell_idx] = Some(cu_idx);
                }
            }
        }
    }

    CuSpatialIndex { grid, grid_w }
}

/// Extract MV Grid from AV1 bitstream data
///
/// **Current Implementation**: Parses tile group data and extracts
/// motion vectors from coding units using the symbol decoder.
///
/// # Performance
///
/// - O(n) where n = number of blocks
pub fn extract_mv_grid(obu_data: &[u8], _frame_index: usize) -> Result<MVGrid, BitvueError> {
    let parsed = ParsedFrame::parse(obu_data)?;

    extract_mv_grid_from_parsed(&parsed)
}

/// Extract MV Grid from cached frame data
///
/// **Current Implementation**:
/// - Parses tile data to extract actual motion vectors from coding units
/// - Falls back to scaffold if tile data unavailable or parsing fails
/// - Uses quarter-pel precision motion vectors from AV1 bitstream
pub fn extract_mv_grid_from_parsed(parsed: &ParsedFrame) -> Result<MVGrid, BitvueError> {
    let block_w = 64u32;
    let block_h = 64u32;
    let grid_w = parsed.dimensions.width.div_ceil(block_w);
    let grid_h = parsed.dimensions.height.div_ceil(block_h);
    let total_blocks = (grid_w * grid_h) as usize;

    let mut mv_l0 = Vec::with_capacity(total_blocks);
    let mut mv_l1 = Vec::with_capacity(total_blocks);
    let mut mode = Vec::with_capacity(total_blocks);

    // If we have tile data, try to parse actual motion vectors
    if parsed.has_tile_data() && parsed.tile_data.len() > 10 {
        match parse_all_coding_units(parsed) {
            Ok(coding_units) => {
                tracing::debug!("Extracting MV from {} coding units", coding_units.len());

                // Build spatial index for O(1) CU lookups (eliminates O(n²) bottleneck)
                let spatial_index =
                    build_cu_spatial_index(&coding_units, grid_w, grid_h, block_w, block_h);

                // Build a grid of MVs from coding units using spatial index
                for sb_y in 0..grid_h {
                    for sb_x in 0..grid_w {
                        // O(1) lookup instead of O(n) linear search
                        if let Some(cu_idx) = spatial_index.get_cu_index(sb_x, sb_y) {
                            let cu = &coding_units[cu_idx];

                            // This CU overlaps our block - use its MV
                            if cu.is_inter() {
                                // Use quarter-pel precision motion vector directly
                                mv_l0.push(CoreMV::new(cu.mv[0].x, cu.mv[0].y));
                                mv_l1.push(CoreMV::MISSING);
                                mode.push(BlockMode::Inter);
                            } else {
                                mv_l0.push(CoreMV::MISSING);
                                mv_l1.push(CoreMV::MISSING);
                                mode.push(BlockMode::Intra);
                            }
                        } else {
                            // No CU found - use default based on frame type
                            if parsed.frame_type.is_intra_only {
                                mv_l0.push(CoreMV::MISSING);
                                mv_l1.push(CoreMV::MISSING);
                                mode.push(BlockMode::Intra);
                            } else {
                                mv_l0.push(CoreMV::ZERO);
                                mv_l1.push(CoreMV::MISSING);
                                mode.push(BlockMode::Inter);
                            }
                        }
                    }
                }

                return Ok(MVGrid::new(
                    parsed.dimensions.width,
                    parsed.dimensions.height,
                    block_w,
                    block_h,
                    mv_l0,
                    mv_l1,
                    Some(mode),
                ));
            }
            Err(e) => {
                tracing::warn!("Failed to parse coding units for MV: {}, using scaffold", e);
                // Fall through to scaffold
            }
        }
    }

    // Fallback: Create scaffold MV grid
    let is_intra = parsed.frame_type.is_intra_only;
    let has_tiles = parsed.has_tile_data();

    for _ in 0..total_blocks {
        if is_intra {
            mv_l0.push(CoreMV::MISSING);
            mv_l1.push(CoreMV::MISSING);
            mode.push(BlockMode::Intra);
        } else if has_tiles {
            mv_l0.push(CoreMV::ZERO);
            mv_l1.push(CoreMV::ZERO);
            mode.push(BlockMode::Inter);
        } else {
            mv_l0.push(CoreMV::ZERO);
            mv_l1.push(CoreMV::ZERO);
            mode.push(BlockMode::Inter);
        }
    }

    Ok(MVGrid::new(
        parsed.dimensions.width,
        parsed.dimensions.height,
        block_w,
        block_h,
        mv_l0,
        mv_l1,
        Some(mode),
    ))
}

/// Parse all coding units from tile data
///
/// Per optimize-code skill: Uses thread-safe LRU cache to avoid re-parsing
/// the same tile data when extracting multiple overlays.
///
/// Returns a vector of all coding units parsed from the tile group data.
/// This is used by MV and prediction mode grid extraction.
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
    fn test_mv_grid_with_valid_data() {
        // Arrange
        let obu_data = create_test_obu_data();

        // Act
        let result = extract_mv_grid(&obu_data, 0);

        // Assert: Should create a grid with default dimensions
        assert!(result.is_ok(), "MV grid extraction should succeed");
        let grid = result.unwrap();
        assert_eq!(grid.block_w, 64);
        assert_eq!(grid.block_h, 64);
        assert!(grid.mv_l0.len() > 0, "MV grid should have L0 vectors");
        assert!(grid.mv_l1.len() > 0, "MV grid should have L1 vectors");
    }

    #[test]
    fn test_mv_grid_inter_vs_intra() {
        // Arrange: Create grid with mixed modes
        let coded_width = 1920;
        let coded_height = 1080;
        let block_w = 64;
        let block_h = 64;
        let grid_w = 30;
        let grid_h = 17;

        let mut mv_l0 = vec![CoreMV::MISSING; grid_w * grid_h];
        let mv_l1 = vec![CoreMV::MISSING; grid_w * grid_h];
        let mut mode = vec![BlockMode::Intra; grid_w * grid_h];

        // Set some blocks to Inter mode
        for i in 10..20 {
            mv_l0[i] = CoreMV::ZERO;
            mode[i] = BlockMode::Inter;
        }

        // Act
        let grid = MVGrid::new(
            coded_width,
            coded_height,
            block_w,
            block_h,
            mv_l0,
            mv_l1,
            Some(mode),
        );

        // Assert
        let stats = grid.statistics();
        assert_eq!(stats.total_blocks, (grid_w * grid_h) as usize);
        assert_eq!(stats.intra_count, (grid_w * grid_h - 10) as usize);
        assert_eq!(stats.inter_count, 10);
    }

    #[test]
    fn test_mv_grid_bounds_checking() {
        // Arrange: Create grid with correct dimensions (1920x1080 / 64x64 = 30x17)
        let grid_w = 30;
        let grid_h = 17;
        let mv_l0 = vec![CoreMV::ZERO; grid_w * grid_h];
        let mv_l1 = vec![CoreMV::MISSING; grid_w * grid_h];
        let grid = MVGrid::new(1920, 1080, 64, 64, mv_l0, mv_l1, None);

        // Act & Assert: Valid bounds
        assert!(grid.get_l0(0, 0).is_some());
        assert!(grid.get_l0(3, 2).is_some());

        // Act & Assert: Out of bounds
        assert!(
            grid.get_l0(30, 0).is_none(),
            "Should return None for out of bounds (x)"
        );
        assert!(
            grid.get_l0(0, 17).is_none(),
            "Should return None for out of bounds (y)"
        );
    }
}
