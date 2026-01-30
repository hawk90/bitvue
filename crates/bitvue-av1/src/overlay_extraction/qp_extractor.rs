//! QP (Quantization Parameter) grid extraction
//!
//! Provides functions to extract QP heatmap data from AV1 bitstreams.

use bitvue_core::qp_heatmap::QPGrid;
use bitvue_core::BitvueError;

use super::cu_parser::{CuSpatialIndex, parse_all_coding_units};
use super::parser::ParsedFrame;
use crate::ivf::OVERLAY_BLOCK_SIZE;
use crate::Qp;

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
    // Validate QP range for type safety
    let qp = Qp::new(base_qp)?;

    extract_qp_grid_typed(obu_data, _frame_index, qp)
}

/// Extract QP Grid with type-safe Qp parameter
///
/// Internal function that uses the Qp newtype for type safety.
/// This validates that base_qp is in the valid range [0, 255].
fn extract_qp_grid_typed(
    obu_data: &[u8],
    _frame_index: usize,
    base_qp: Qp,
) -> Result<QPGrid, BitvueError> {
    let parsed = ParsedFrame::parse(obu_data)?;

    let block_w = OVERLAY_BLOCK_SIZE;
    let block_h = OVERLAY_BLOCK_SIZE;
    let grid_w = parsed.dimensions.width.div_ceil(block_w);
    let grid_h = parsed.dimensions.height.div_ceil(block_h);

    let qp_value = base_qp.value();
    let qp = vec![qp_value; (grid_w * grid_h) as usize];

    Ok(QPGrid::new(grid_w, grid_h, block_w, block_h, qp, qp_value))
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
    // Validate QP range for type safety
    let qp = Qp::new(base_qp)?;

    extract_qp_grid_from_parsed_typed(parsed, _frame_index, qp)
}

/// Extract QP Grid from cached frame data with type-safe Qp parameter
///
/// Internal function that uses the Qp newtype for type safety.
fn extract_qp_grid_from_parsed_typed(
    parsed: &ParsedFrame,
    _frame_index: usize,
    base_qp: Qp,
) -> Result<QPGrid, BitvueError> {
    let block_w = OVERLAY_BLOCK_SIZE;
    let block_h = OVERLAY_BLOCK_SIZE;
    let grid_w = parsed.dimensions.width.div_ceil(block_w);
    let grid_h = parsed.dimensions.height.div_ceil(block_h);
    let total_blocks = (grid_w * grid_h) as usize;
    let base_qp_value = base_qp.value();

    // If we have tile data, try to parse actual QP values
    if parsed.has_tile_data() && parsed.tile_data.len() > 10 {
        match parse_all_coding_units(parsed) {
            Ok(coding_units) => {
                tracing::debug!(
                    "Extracting QP values from {} coding units",
                    coding_units.len()
                );
                let qp = build_qp_grid_from_cus(
                    &*coding_units,
                    grid_w,
                    grid_h,
                    block_w,
                    block_h,
                    base_qp_value,
                );
                return Ok(QPGrid::new(grid_w, grid_h, block_w, block_h, qp, base_qp_value));
            }
            Err(e) => {
                tracing::warn!("Failed to parse coding units for QP: {}, using base_qp", e);
                // Fall through to scaffold
            }
        }
    }

    // Fallback: Use base_q_idx for all blocks
    let qp = vec![base_qp_value; total_blocks];

    Ok(QPGrid::new(grid_w, grid_h, block_w, block_h, qp, base_qp_value))
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

        // Also test with invalid QP to ensure validation works
        let invalid_result = extract_qp_grid(&obu_data, 0, 256); // Invalid: > 255
        assert!(invalid_result.is_err(), "Should reject QP > 255");

        let invalid_result2 = extract_qp_grid(&obu_data, 0, -1); // Invalid: < 0
        assert!(invalid_result2.is_err(), "Should reject QP < 0");
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
