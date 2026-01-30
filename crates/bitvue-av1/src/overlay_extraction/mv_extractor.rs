//! Motion Vector grid extraction
//!
//! Provides functions to extract motion vector data from AV1 bitstreams.

use bitvue_core::{
    mv_overlay::{BlockMode, MVGrid, MotionVector as CoreMV},
    BitvueError,
};

use super::cu_parser::{CuSpatialIndex, parse_all_coding_units};
use super::parser::ParsedFrame;
use crate::ivf::OVERLAY_BLOCK_SIZE;

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
    let block_w = OVERLAY_BLOCK_SIZE;
    let block_h = OVERLAY_BLOCK_SIZE;
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
                    CuSpatialIndex::new(&*coding_units, grid_w, grid_h, block_w, block_h);

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

    for _ in 0..total_blocks {
        if is_intra {
            mv_l0.push(CoreMV::MISSING);
            mv_l1.push(CoreMV::MISSING);
            mode.push(BlockMode::Intra);
        } else {
            // Inter frames (with or without tiles) use default MV
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
        assert_eq!(grid.block_w, OVERLAY_BLOCK_SIZE);
        assert_eq!(grid.block_h, OVERLAY_BLOCK_SIZE);
        assert!(grid.mv_l0.len() > 0, "MV grid should have L0 vectors");
        assert!(grid.mv_l1.len() > 0, "MV grid should have L1 vectors");
    }

    #[test]
    fn test_mv_grid_inter_vs_intra() {
        // Arrange: Create grid with mixed modes
        let coded_width = 1920;
        let coded_height = 1080;
        let block_w = OVERLAY_BLOCK_SIZE;
        let block_h = OVERLAY_BLOCK_SIZE;
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
