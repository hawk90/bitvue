//! QP (Quantization Parameter) grid extraction
//!
//! Provides functions to extract QP heatmap data from AV1 bitstreams.

use bitvue_engine::qp_heatmap::QPGrid;
use bitvue_engine::BitvueError;

use super::cu_parser::parse_all_coding_units;
use super::cu_spatial_index::CuSpatialIndex;
use super::parser::ParsedFrame;
use crate::ivf::OVERLAY_BLOCK_SIZE;
use crate::Qp;

/// `QPGrid::missing`'s default: a cell no decoded block covers.
const MISSING_QP: i16 = -1;

/// Extract QP Grid from AV1 bitstream data
///
/// Parses `obu_data` and extracts the per-block QP of its decoded coding units; see
/// [`extract_qp_grid_from_parsed`]. A frame without decodable tile data is an error.
pub fn extract_qp_grid(
    obu_data: &[u8],
    frame_index: usize,
    base_qp: i16,
) -> Result<QPGrid, BitvueError> {
    let parsed = ParsedFrame::parse(obu_data)?;
    extract_qp_grid_from_parsed(&parsed, frame_index, base_qp)
}

/// Extract QP Grid from cached frame data
///
/// **Current Implementation**:
/// - Parses tile data to extract actual QP values from coding units
/// - Fails if the frame has no decodable tile data; cells past the point where decoding stopped
///   hold the grid's missing-value marker
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

    // Check for overflow in grid size calculation
    grid_w.checked_mul(grid_h).ok_or_else(|| {
        BitvueError::Decode(format!("Grid dimensions too large: {}x{}", grid_w, grid_h))
    })?;

    let base_qp_value = base_qp.value();

    if !super::provenance::has_decodable_tile(parsed) {
        return Err(super::provenance::no_decodable_tile());
    }
    let coding_units = parse_all_coding_units(parsed)?;
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
        base_qp_value,
    );
    Ok(QPGrid::new(
        grid_w, grid_h, block_w, block_h, qp, MISSING_QP,
    ))
}

/// Helper: Build QP grid from coding units using spatial index
///
/// Creates a QP grid by using a spatial index for O(1) coding unit lookup.
/// Eliminates O(n²) linear search bottleneck (510k→510 lookups for 1080p).
///
/// # Safety
///
/// Grid dimensions are expected to be validated by the caller before this function is invoked.
/// This function uses saturating arithmetic for capacity allocation to prevent panic on overflow.
fn build_qp_grid_from_cus(
    coding_units: &[crate::tile::CodingUnit],
    grid_w: u32,
    grid_h: u32,
    block_w: u32,
    block_h: u32,
    base_qp: i16,
) -> Vec<i16> {
    // Use saturating multiplication for capacity to prevent overflow panic
    // If overflow occurs, use a reasonable maximum capacity (8K video with 4x4 blocks)
    const MAX_REASONABLE_CAPACITY: u32 = 512 * 512; // ~262K blocks
    let total_blocks = grid_w.saturating_mul(grid_h).min(MAX_REASONABLE_CAPACITY) as usize;
    let mut qp = Vec::with_capacity(total_blocks);

    // Build spatial index (O(n×k) where k=cells per CU, typically ~4)
    let spatial_index = CuSpatialIndex::new(coding_units, grid_w, grid_h, block_w, block_h);

    // Populate QP grid using O(1) lookups instead of O(n) searches
    for grid_y in 0..grid_h {
        for grid_x in 0..grid_w {
            let cu_qp = spatial_index
                .get_cu_index(grid_x, grid_y)
                .map(|cu_idx| coding_units[cu_idx].effective_qp(base_qp))
                .unwrap_or(MISSING_QP);

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
    fn placeholder_obus_give_an_error_and_an_out_of_range_qp_is_rejected() {
        let obu_data = create_test_obu_data();

        // No tile data in the placeholder bytes: nothing to extract, nothing invented.
        assert!(extract_qp_grid(&obu_data, 0, 32).is_err());
        // Out-of-range base QPs are rejected before anything else.
        assert!(extract_qp_grid(&obu_data, 0, 256).is_err());
        assert!(extract_qp_grid(&obu_data, 0, -1).is_err());
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
