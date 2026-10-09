//! Motion Vector grid extraction
//!
//! Provides functions to extract motion vector data from AV1 bitstreams.

use bitvue_engine::{
    mv_overlay::{BlockMode, MVGrid, MotionVector as CoreMV},
    BitvueError,
};

use super::cu_parser::{parse_all_coding_units, CuSpatialIndex};
use super::parser::ParsedFrame;
use crate::ivf::OVERLAY_BLOCK_SIZE;

/// Converts a decoded 1/8-sample `MotionVector` to the overlay grid's quarter-sample units
/// (rounding half away from zero).
fn overlay_mv(mv: crate::tile::MotionVector) -> CoreMV {
    let to_qpel = |v: i32| (v + v.signum()) / 2;
    CoreMV::new(to_qpel(mv.x), to_qpel(mv.y))
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
/// - Fails if the frame has no decodable tile data (there is no substitute grid); cells past the
///   point where decoding stopped are `BlockMode::None`
/// - Uses quarter-pel precision motion vectors from AV1 bitstream
pub fn extract_mv_grid_from_parsed(parsed: &ParsedFrame) -> Result<MVGrid, BitvueError> {
    let block_w = OVERLAY_BLOCK_SIZE;
    let block_h = OVERLAY_BLOCK_SIZE;
    let grid_w = parsed.dimensions.width.div_ceil(block_w);
    let grid_h = parsed.dimensions.height.div_ceil(block_h);

    // Check for overflow in grid size calculation
    let total_blocks = grid_w.checked_mul(grid_h).ok_or_else(|| {
        BitvueError::Decode(format!("Grid dimensions too large: {}x{}", grid_w, grid_h))
    })? as usize;

    let mut mv_l0 = Vec::with_capacity(total_blocks);
    let mut mv_l1 = Vec::with_capacity(total_blocks);
    let mut mode = Vec::with_capacity(total_blocks);

    if !super::provenance::has_decodable_tile(parsed) {
        return Err(super::provenance::no_decodable_tile());
    }
    let coding_units = parse_all_coding_units(parsed)?;
    tracing::debug!("Extracting MV from {} coding units", coding_units.len());

    // Build spatial index for O(1) CU lookups (eliminates O(n²) bottleneck)
    let spatial_index = CuSpatialIndex::new(&coding_units, grid_w, grid_h, block_w, block_h);

    for sb_y in 0..grid_h {
        for sb_x in 0..grid_w {
            let Some(cu_idx) = spatial_index.get_cu_index(sb_x, sb_y) else {
                // No decoded block covers this cell (the decode stopped before it).
                mv_l0.push(CoreMV::MISSING);
                mv_l1.push(CoreMV::MISSING);
                mode.push(BlockMode::None);
                continue;
            };
            let cu = &coding_units[cu_idx];
            if cu.use_intrabc {
                // Always a subset of intra (ref_frame[0] == Intra) -- see BlockMode::IntraBc's doc.
                mv_l0.push(CoreMV::MISSING);
                mv_l1.push(CoreMV::MISSING);
                mode.push(BlockMode::IntraBc);
            } else if cu.is_inter() {
                mv_l0.push(overlay_mv(cu.mv[0]));
                let is_compound = cu.ref_frames[1] != crate::tile::RefFrame::Intra;
                // L1 (backward reference MV) is only meaningful for compound blocks --
                // `cu.mv[1]` is always zero for single-ref blocks (see `parse_coding_unit`), so
                // reporting it as MISSING there (rather than a misleading real-looking zero)
                // matches how intra/no-CU blocks already report MISSING for both planes.
                mv_l1.push(if is_compound {
                    overlay_mv(cu.mv[1])
                } else {
                    CoreMV::MISSING
                });
                mode.push(if cu.skip {
                    BlockMode::Skip
                } else if is_compound {
                    BlockMode::Compound
                } else {
                    BlockMode::Inter
                });
            } else {
                mv_l0.push(CoreMV::MISSING);
                mv_l1.push(CoreMV::MISSING);
                mode.push(BlockMode::Intra);
            }
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
    fn placeholder_obus_without_tile_data_give_an_error_not_an_invented_grid() {
        let obu_data = create_test_obu_data();

        assert!(extract_mv_grid(&obu_data, 0).is_err());
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
        assert_eq!(stats.total_blocks, grid_w * grid_h);
        assert_eq!(stats.intra_count, grid_w * grid_h - 10);
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
