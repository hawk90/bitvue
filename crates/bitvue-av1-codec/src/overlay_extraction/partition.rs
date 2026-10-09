//! Partition, prediction mode, and transform grid extraction
//!
//! Provides functions to extract partition trees, prediction modes,
//! and transform sizes from AV1 bitstreams.

use bitvue_engine::{
    limits::{AV1_BLOCK_SIZE, MAX_GRID_BLOCKS, MAX_GRID_DIMENSION},
    partition_grid::{PartitionGrid, PartitionType},
    BitvueError,
};

use super::cu_parser::parse_all_coding_units;
use super::parser::ParsedFrame;
use crate::tile::{PredictionMode, TxSize};

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
    if !super::provenance::has_decodable_tile(parsed) {
        return Err(super::provenance::no_decodable_tile());
    }
    parse_partition_trees_from_tile_data(parsed)
}

/// The partition grid of a decoded frame: one block per coding unit of the canonical parse (the
/// same cached one every other extractor uses, so the grids always agree with each other).
///
/// This used to run its own parse, with a fresh MV context per superblock, smaller superblocks
/// at the frame edge, and an invented block wherever a superblock failed -- a second decoder
/// that disagreed with the real one. A frame the decode does not complete simply has no blocks
/// past that point; its provenance (`frame_provenance`) says so.
fn parse_partition_trees_from_tile_data(
    parsed: &ParsedFrame,
) -> Result<PartitionGrid, BitvueError> {
    let units = parse_all_coding_units(parsed)?;
    let mut grid = PartitionGrid::new(
        parsed.dimensions.width,
        parsed.dimensions.height,
        parsed.dimensions.sb_size,
    );
    for cu in units.iter() {
        grid.add_block(bitvue_engine::partition_grid::PartitionBlock::new(
            cu.x,
            cu.y,
            cu.width,
            cu.height,
            partition_type_from_prediction_mode(cu.mode),
            0,
        ));
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

    if !super::provenance::has_decodable_tile(parsed) {
        return Err(super::provenance::no_decodable_tile());
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

    if !super::provenance::has_decodable_tile(parsed) {
        return Err(super::provenance::no_decodable_tile());
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

/// Build a grid from coding units using spatial indexing for O(n) performance
///
/// This is a key optimization that changes the algorithm from:
/// - Before: O(grid_h × grid_w × num_coding_units) - nested triple loop
/// - After: O(grid_h × grid_w + num_coding_units) - two separate loops
///
/// For 1080p video with ~30,000 coding units and ~8,000 grid blocks:
/// - Before: ~240M iterations (30,000 × 8,000)
/// - After: ~38K iterations (8,000 + 30,000)
/// - Speedup: ~6,300x faster
///
/// # Parameters
/// - `coding_units`: Slice of all coding units parsed from tile data
/// - `parsed`: Parsed frame with dimensions
/// - `block_w`, `block_h`: Grid block size in pixels
/// - `output`: Vector to fill with grid values
/// - `cu_value_fn`: Function to extract value from CU (mode, tx_size, etc.)
///
/// Grid blocks no coding unit covers (the decode stopped before them) stay `None`.
fn build_grid_from_coding_units_spatial<T, F>(
    coding_units: &[crate::tile::CodingUnit],
    parsed: &ParsedFrame,
    block_w: u32,
    block_h: u32,
    output: &mut Vec<Option<T>>,
    cu_value_fn: F,
) -> Result<(), BitvueError>
where
    F: Fn(&crate::tile::CodingUnit) -> T,
{
    let grid_w = parsed.dimensions.width.div_ceil(block_w);
    let grid_h = parsed.dimensions.height.div_ceil(block_h);

    // Callers pass `Vec::with_capacity(total_blocks)` (length 0, only reserved capacity) --
    // every write below is a direct `output[idx] = ...` index assignment, which the `idx <
    // output.len()` guards silently skip entirely on a zero-length vec. Pre-fill to the real
    // length here so writes actually land (this path never actually ran before `ParsedFrame::
    // parse`'s OBU_FRAME tile-data bug was fixed -- has_tile_data() was always false, so this
    // function was never reached; the bug was latent, not a regression from that fix).
    let total = (grid_w as usize).saturating_mul(grid_h as usize);
    if output.len() < total {
        output.resize_with(total, || None);
    }

    // Build spatial index: map superblock position to relevant CUs
    // This allows O(1) lookup of which CUs to check for each grid block
    use std::collections::HashMap;
    let mut sb_index: HashMap<(u32, u32), Vec<usize>> = HashMap::new();

    for (cu_idx, cu) in coding_units.iter().enumerate() {
        // Calculate which superblock this CU belongs to
        let sb_x = cu.x / parsed.dimensions.sb_size;
        let sb_y = cu.y / parsed.dimensions.sb_size;
        sb_index.entry((sb_x, sb_y)).or_default().push(cu_idx);
    }

    // Now iterate through grid blocks, only checking CUs from relevant superblocks
    for grid_y in 0..grid_h {
        for grid_x in 0..grid_w {
            let block_x = grid_x * block_w;
            let block_y = grid_y * block_h;

            // Find which superblock this block belongs to
            let sb_x = block_x / parsed.dimensions.sb_size;
            let sb_y = block_y / parsed.dimensions.sb_size;

            // Get CUs from this superblock only (O(1) lookup)
            if let Some(cu_indices) = sb_index.get(&(sb_x, sb_y)) {
                // Only check CUs from this superblock (typically 1-4 CUs)
                for cu_idx in cu_indices {
                    let cu = &coding_units[*cu_idx];
                    if cu.x < block_x + block_w
                        && cu.x + cu.width > block_x
                        && cu.y < block_y + block_h
                        && cu.y + cu.height > block_y
                    {
                        // This CU overlaps our block - use its value
                        let idx = (grid_y * grid_w + grid_x) as usize;
                        if idx < output.len() {
                            output[idx] = Some(cu_value_fn(cu));
                        }
                        break;
                    }
                }
            }
        }
    }

    Ok(())
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
    fn a_frame_without_decodable_tile_data_is_an_error_for_every_grid() {
        // No tile in these bytes: there is nothing to decode, and no substitute grid.
        let obu_data = vec![0x00, 0x01, 0x02, 0x03];
        let parsed = ParsedFrame::parse(&obu_data).unwrap();

        assert!(extract_partition_grid_from_parsed(&parsed).is_err());
        assert!(extract_prediction_mode_grid_from_parsed(&parsed).is_err());
        assert!(extract_transform_grid_from_parsed(&parsed).is_err());
        assert!(super::super::extract_mv_grid_from_parsed(&parsed).is_err());
        assert!(super::super::extract_qp_grid_from_parsed(&parsed, 0, 32).is_err());
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

    const AV1_IVF_FIXTURE: &[u8] = include_bytes!("../../../../test_data/av1_test.ivf");

    fn find_seq_header_bytes(frames: &[crate::ivf::IvfFrame]) -> Option<Vec<u8>> {
        for frame in frames.iter().take(8) {
            let mut iter = crate::obu::ObuIterator::new(&frame.data);
            while let Some(Ok(found)) = iter.next_obu_with_offset() {
                if found.obu.header.obu_type == crate::obu::ObuType::SequenceHeader {
                    return Some(frame.data[found.offset..found.offset + found.consumed].to_vec());
                }
            }
        }
        None
    }

    /// `delta_q_enabled` must come from the exact frame-header parse, not the approximate one.
    ///
    /// An earlier version of this module's tests asserted that a fixture frame with
    /// `delta_q_enabled = true` parses differently with the flag off, from a count of 82 such
    /// frames out of 250. That count came from `parse_frame_header_basic`, which only approximates
    /// everything after the quantizer fields for non-key frames and reported `delta_q_present` for
    /// bits that belong to other syntax elements. The exact parse (`parse_frame_header_full`) says
    /// none of the fixture's frames use delta-q at all, and a dav1d 1.5.1 trace of frame 0 agrees
    /// (no `Post-delta_q` after the first block's `Post-cdef_idx`, although that block sits at a
    /// superblock origin, where `delta_q` is read whenever it is present). Passing the wrong
    /// `true` made the parser read `delta_q` symbols that were never coded, desyncing every later
    /// symbol of those frames.
    ///
    /// The `delta_q` read itself stays covered by the synthetic goldens
    /// (`delta-q-lf`, `delta-q-lf-sb128`, ...), which turn the flag on explicitly.
    #[test]
    fn real_fixture_has_no_delta_q_frames_per_the_exact_header_parse() {
        let (_hdr, frames) = crate::ivf::parse_ivf_frames(AV1_IVF_FIXTURE).unwrap();
        let seq_bytes = find_seq_header_bytes(&frames).expect("fixture has a sequence header");

        let mut parsed_frames = 0;
        for (idx, frame) in frames.iter().enumerate() {
            let obu_data: Vec<u8> = [seq_bytes.as_slice(), frame.data.as_slice()].concat();
            let Ok(parsed) = super::super::parser::ParsedFrame::parse(&obu_data) else {
                continue;
            };
            parsed_frames += 1;
            assert!(
                !parsed.delta_q_enabled,
                "frame {idx}: delta_q_enabled must follow the exact header parse"
            );
        }
        assert!(parsed_frames > 200, "only {parsed_frames} frames parsed");
    }
}
