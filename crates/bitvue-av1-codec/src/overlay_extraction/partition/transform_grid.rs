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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tile::{CodingUnit, TxSize};

    const FIXTURE: &[u8] = include_bytes!("../../../../../test_data/av1_aomenc_testsrc2.ivf");

    fn cu(x: u32, y: u32, w: u32, h: u32, tx: TxSize) -> CodingUnit {
        CodingUnit {
            tx_size: tx,
            ..CodingUnit::new(x, y, w, h)
        }
    }

    /// A frame of `width` x `height` whose decode is already cached as `units`.
    fn frame_with(width: u32, height: u32, units: Vec<CodingUnit>) -> ParsedFrame {
        let mut parsed = ParsedFrame::parse(&[]).unwrap();
        parsed.dimensions.width = width;
        parsed.dimensions.height = height;
        parsed.tile_data = std::sync::Arc::from([0u8]);
        parsed.decoded = Some(super::super::super::ParsedCodingUnits {
            units: std::sync::Arc::new(units),
            outcome: Default::default(),
        });
        parsed
    }

    fn grid_cells(grid: &TransformGrid) -> Vec<Option<TxSize>> {
        (0..grid.grid_h)
            .flat_map(|row| (0..grid.grid_w).map(move |col| (col, row)))
            .map(|(col, row)| grid.get(col, row))
            .collect()
    }

    // ---- TransformGrid::new / get ----

    #[test]
    fn the_grid_size_rounds_partial_blocks_up() {
        let exact = TransformGrid::new(32, 16, 16, 16, vec![None; 2]);
        assert_eq!((exact.grid_w, exact.grid_h), (2, 1));

        // One pixel past a block boundary adds a whole block in that direction only.
        let wide = TransformGrid::new(33, 16, 16, 16, vec![None; 3]);
        assert_eq!((wide.grid_w, wide.grid_h), (3, 1));
        let tall = TransformGrid::new(32, 17, 16, 16, vec![None; 4]);
        assert_eq!((tall.grid_w, tall.grid_h), (2, 2));

        // Non-square blocks divide each axis by its own size.
        let rect = TransformGrid::new(64, 48, 32, 8, vec![None; 12]);
        assert_eq!((rect.grid_w, rect.grid_h), (2, 6));
        assert_eq!((rect.block_w, rect.block_h), (32, 8));
        assert_eq!((rect.coded_width, rect.coded_height), (64, 48));
    }

    #[test]
    fn get_indexes_row_major_on_a_non_square_grid() {
        // 3 columns x 2 rows, each cell distinct so a swapped axis is visible.
        let cells = vec![
            Some(TxSize::Tx4x4),
            Some(TxSize::Tx8x8),
            Some(TxSize::Tx16x16),
            Some(TxSize::Tx32x32),
            Some(TxSize::Tx64x64),
            None,
        ];
        let grid = TransformGrid::new(48, 32, 16, 16, cells);
        assert_eq!(grid.get(0, 0), Some(TxSize::Tx4x4));
        assert_eq!(grid.get(1, 0), Some(TxSize::Tx8x8));
        assert_eq!(grid.get(2, 0), Some(TxSize::Tx16x16));
        assert_eq!(grid.get(0, 1), Some(TxSize::Tx32x32));
        assert_eq!(grid.get(1, 1), Some(TxSize::Tx64x64));
        // A cell holding `None` and a cell outside the grid both read as "no transform size".
        assert_eq!(grid.get(2, 1), None);
    }

    #[test]
    fn get_outside_the_grid_is_none_on_each_axis_separately() {
        let grid = TransformGrid::new(48, 32, 16, 16, vec![Some(TxSize::Tx8x8); 6]);
        assert_eq!(grid.get(2, 1), Some(TxSize::Tx8x8));
        // The first column/row past the edge. With a full grid a missing bounds check on one
        // axis would wrap into the next row or run past the vector, so each axis is probed alone.
        assert_eq!(grid.get(3, 0), None);
        assert_eq!(grid.get(3, 1), None);
        assert_eq!(grid.get(0, 2), None);
        assert_eq!(grid.get(2, 2), None);
        assert_eq!(grid.get(u32::MAX, 0), None);
        assert_eq!(grid.get(0, u32::MAX), None);
        assert_eq!(grid.get(u32::MAX, u32::MAX), None);
    }

    #[test]
    fn a_one_cell_grid_and_a_zero_sized_frame() {
        let one = TransformGrid::new(1, 1, 16, 16, vec![Some(TxSize::Tx4x4)]);
        assert_eq!((one.grid_w, one.grid_h), (1, 1));
        assert_eq!(one.get(0, 0), Some(TxSize::Tx4x4));
        assert_eq!(one.get(1, 0), None);

        let empty = TransformGrid::new(0, 0, 16, 16, Vec::new());
        assert_eq!((empty.grid_w, empty.grid_h), (0, 0));
        assert_eq!(empty.get(0, 0), None);
    }

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "tx_sizes length mismatch: expected 4, got 3")]
    fn too_few_cells_for_the_grid_is_caught_in_debug_builds() {
        let _ = TransformGrid::new(32, 32, 16, 16, vec![None; 3]);
    }

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "tx_sizes length mismatch: expected 4, got 5")]
    fn too_many_cells_for_the_grid_is_caught_in_debug_builds() {
        let _ = TransformGrid::new(32, 32, 16, 16, vec![None; 5]);
    }

    /// Past 512*512 blocks on one axis the length is not checked (the product could overflow), and
    /// reads still stay inside the vector.
    #[test]
    fn an_axis_beyond_the_size_cap_skips_the_length_check_and_still_reads_safely() {
        let len = 512 * 512 + 1;
        let mut cells = vec![None; len];
        cells[len - 1] = Some(TxSize::Tx32x32);
        let grid = TransformGrid::new(len as u32, 1, 1, 1, cells);
        assert_eq!((grid.grid_w, grid.grid_h), (len as u32, 1));
        assert_eq!(grid.get(len as u32 - 1, 0), Some(TxSize::Tx32x32));
        assert_eq!(grid.get(len as u32 - 2, 0), None);
        assert_eq!(grid.get(len as u32, 0), None);

        // A vector shorter than the grid says: reads past its end are None, not a panic.
        let short = TransformGrid::new(len as u32, 1, 1, 1, vec![Some(TxSize::Tx4x4); 10]);
        assert_eq!(short.get(9, 0), Some(TxSize::Tx4x4));
        assert_eq!(short.get(10, 0), None);
        assert_eq!(short.get(len as u32 - 1, 0), None);
    }

    const CAP: u32 = 512 * 512;

    /// The cap is "more than 512*512 blocks on one axis": exactly 512*512 is still checked.
    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "tx_sizes length mismatch")]
    fn a_width_of_exactly_the_cap_still_has_its_length_checked() {
        let _ = TransformGrid::new(CAP, 1, 1, 1, vec![None; 5]);
    }

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "tx_sizes length mismatch")]
    fn a_height_of_exactly_the_cap_still_has_its_length_checked() {
        let _ = TransformGrid::new(1, CAP, 1, 1, vec![None; 5]);
    }

    #[test]
    fn a_height_beyond_the_size_cap_skips_the_length_check_too() {
        let mut cells = vec![None; CAP as usize + 1];
        cells[CAP as usize] = Some(TxSize::Tx64x64);
        let grid = TransformGrid::new(1, CAP + 1, 1, 1, cells);
        assert_eq!((grid.grid_w, grid.grid_h), (1, CAP + 1));
        assert_eq!(grid.get(0, CAP), Some(TxSize::Tx64x64));
        assert_eq!(grid.get(0, CAP + 1), None);

        let short = TransformGrid::new(1, CAP + 1, 1, 1, vec![Some(TxSize::Tx4x4); 3]);
        assert_eq!(short.get(0, 2), Some(TxSize::Tx4x4));
        assert_eq!(short.get(0, 3), None);
    }

    /// With the length unchecked past the cap, a vector longer than the grid must not leak
    /// its extra cells through coordinates just outside the grid.
    #[test]
    fn a_row_just_past_the_grid_stays_none_even_if_the_vector_is_longer() {
        let w = CAP + 1;
        let cells = vec![Some(TxSize::Tx8x8); 2 * w as usize];
        let grid = TransformGrid::new(w, 1, 1, 1, cells);
        assert_eq!(grid.get(0, 0), Some(TxSize::Tx8x8));
        assert_eq!(grid.get(0, 1), None);
        assert_eq!(grid.get(w - 1, 1), None);
    }

    // ---- extract_transform_grid_from_parsed over synthetic coding units ----

    #[test]
    fn each_block_takes_the_transform_size_of_the_unit_covering_it() {
        // 40x24 frame -> 3x2 blocks of 16. The second row's units are partial (frame is 24 high).
        let units = vec![
            cu(0, 0, 16, 16, TxSize::Tx4x4),
            cu(16, 0, 16, 16, TxSize::Tx8x8),
            cu(32, 0, 16, 16, TxSize::Tx16x16),
            cu(0, 16, 16, 16, TxSize::Tx32x32),
            cu(16, 16, 32, 16, TxSize::Tx64x64),
        ];
        let grid = extract_transform_grid_from_parsed(&frame_with(40, 24, units)).unwrap();

        assert_eq!((grid.coded_width, grid.coded_height), (40, 24));
        assert_eq!((grid.block_w, grid.block_h), (16, 16));
        assert_eq!((grid.grid_w, grid.grid_h), (3, 2));
        assert_eq!(
            grid_cells(&grid),
            vec![
                Some(TxSize::Tx4x4),
                Some(TxSize::Tx8x8),
                Some(TxSize::Tx16x16),
                Some(TxSize::Tx32x32),
                Some(TxSize::Tx64x64),
                Some(TxSize::Tx64x64),
            ]
        );
    }

    #[test]
    fn a_large_unit_fills_every_block_it_covers_and_no_others() {
        // One 32x32 unit at the frame origin of a 64x48 frame (4x3 blocks); the rest is uncovered.
        let units = vec![cu(0, 0, 32, 32, TxSize::Tx16x16)];
        let grid = extract_transform_grid_from_parsed(&frame_with(64, 48, units)).unwrap();
        assert_eq!((grid.grid_w, grid.grid_h), (4, 3));

        let covered = |col: u32, row: u32| col < 2 && row < 2;
        for row in 0..3 {
            for col in 0..4 {
                let expected = covered(col, row).then_some(TxSize::Tx16x16);
                assert_eq!(grid.get(col, row), expected, "block ({col},{row})");
            }
        }
    }

    #[test]
    fn units_are_found_across_superblock_boundaries() {
        // 192x64 frame with 64x64 superblocks: a unit per superblock, each with its own size.
        let units = vec![
            cu(128, 0, 64, 64, TxSize::Tx64x64),
            cu(0, 0, 64, 64, TxSize::Tx8x8),
            cu(64, 0, 64, 64, TxSize::Tx32x32),
        ];
        let grid = extract_transform_grid_from_parsed(&frame_with(192, 64, units)).unwrap();
        assert_eq!((grid.grid_w, grid.grid_h), (12, 4));
        for row in 0..4 {
            for col in 0..12 {
                let expected = match col / 4 {
                    0 => TxSize::Tx8x8,
                    1 => TxSize::Tx32x32,
                    _ => TxSize::Tx64x64,
                };
                assert_eq!(grid.get(col, row), Some(expected), "block ({col},{row})");
            }
        }
    }

    #[test]
    fn no_units_gives_an_all_empty_grid_of_the_right_size() {
        let grid = extract_transform_grid_from_parsed(&frame_with(48, 32, Vec::new())).unwrap();
        assert_eq!((grid.grid_w, grid.grid_h), (3, 2));
        assert_eq!(grid.tx_sizes.len(), 6);
        assert!(grid.tx_sizes.iter().all(Option::is_none));
    }

    #[test]
    fn small_units_inside_a_block_the_first_in_decode_order_names_the_block() {
        // Four 8x8 units tile one 16x16 block; the grid keeps one value per block.
        let units = vec![
            cu(0, 0, 8, 8, TxSize::Tx4x4),
            cu(8, 0, 8, 8, TxSize::Tx8x8),
            cu(0, 8, 8, 8, TxSize::Tx16x16),
            cu(8, 8, 8, 8, TxSize::Tx32x32),
        ];
        let grid = extract_transform_grid_from_parsed(&frame_with(16, 16, units)).unwrap();
        assert_eq!(grid_cells(&grid), vec![Some(TxSize::Tx4x4)]);
    }

    #[test]
    fn a_unit_outside_the_frame_is_ignored() {
        let units = vec![
            cu(0, 0, 16, 16, TxSize::Tx8x8),
            cu(64, 0, 16, 16, TxSize::Tx64x64),
            cu(0, 64, 16, 16, TxSize::Tx64x64),
        ];
        let grid = extract_transform_grid_from_parsed(&frame_with(32, 16, units)).unwrap();
        assert_eq!(
            grid_cells(&grid),
            vec![Some(TxSize::Tx8x8), None],
            "only the unit inside the frame may appear"
        );
    }

    // ---- extract_transform_grid_from_parsed: size limits and missing tile data ----

    #[test]
    fn a_frame_of_exactly_the_maximum_block_count_is_accepted() {
        // 8192 x 8192 pixels = 512 x 512 blocks = MAX_GRID_BLOCKS.
        let grid = extract_transform_grid_from_parsed(&frame_with(8192, 8192, Vec::new())).unwrap();
        assert_eq!((grid.grid_w, grid.grid_h), (512, 512));
        assert_eq!(grid.tx_sizes.len(), 512 * 512);
    }

    #[test]
    fn one_block_more_than_the_maximum_is_rejected_on_either_axis() {
        for (w, h) in [(8193, 8192), (8192, 8193)] {
            let err =
                extract_transform_grid_from_parsed(&frame_with(w, h, Vec::new())).unwrap_err();
            assert!(
                err.to_string().contains("Grid exceeds maximum size"),
                "{w}x{h}: {err}"
            );
        }
    }

    #[test]
    fn dimensions_whose_block_count_overflows_are_rejected_not_wrapped() {
        // 2^28 x 2^28 blocks: the product does not fit in u32 and would wrap to a small number.
        let err = extract_transform_grid_from_parsed(&frame_with(u32::MAX, u32::MAX, Vec::new()))
            .unwrap_err();
        assert!(
            err.to_string().contains("Grid dimensions too large"),
            "{err}"
        );
    }

    #[test]
    fn the_size_limit_is_checked_before_the_tile_data() {
        // No tile bytes either, but the size error is the one reported.
        let mut parsed = frame_with(16000, 16000, Vec::new());
        parsed.tile_data = std::sync::Arc::from([]);
        let err = extract_transform_grid_from_parsed(&parsed).unwrap_err();
        assert!(
            err.to_string().contains("Grid exceeds maximum size"),
            "{err}"
        );
    }

    #[test]
    fn a_frame_without_tile_bytes_is_an_error_even_if_units_are_cached() {
        let mut parsed = frame_with(32, 32, vec![cu(0, 0, 16, 16, TxSize::Tx8x8)]);
        parsed.tile_data = std::sync::Arc::from([]);
        let err = extract_transform_grid_from_parsed(&parsed).unwrap_err();
        assert!(err.to_string().contains("no decodable tile data"), "{err}");
    }

    #[test]
    fn a_zero_sized_frame_with_tile_bytes_gives_an_empty_grid() {
        let grid = extract_transform_grid_from_parsed(&frame_with(0, 0, Vec::new())).unwrap();
        assert_eq!((grid.grid_w, grid.grid_h), (0, 0));
        assert!(grid.tx_sizes.is_empty());
    }

    #[test]
    fn one_block_past_the_maximum_by_a_single_block_is_rejected() {
        // 262145 x 1 blocks: one over MAX_GRID_BLOCKS.
        let err = extract_transform_grid_from_parsed(&frame_with(16 * (CAP + 1), 16, Vec::new()))
            .unwrap_err();
        assert!(
            err.to_string().contains("Grid exceeds maximum size"),
            "{err}"
        );
    }

    #[test]
    fn a_unit_that_only_touches_a_block_edge_does_not_claim_the_block() {
        // The unit starts exactly where block 0 ends (x = 16): block 0 stays empty.
        let right = extract_transform_grid_from_parsed(&frame_with(
            32,
            16,
            vec![cu(16, 0, 16, 16, TxSize::Tx8x8)],
        ))
        .unwrap();
        assert_eq!(grid_cells(&right), vec![None, Some(TxSize::Tx8x8)]);

        // Same vertically.
        let below = extract_transform_grid_from_parsed(&frame_with(
            16,
            32,
            vec![cu(0, 16, 16, 16, TxSize::Tx8x8)],
        ))
        .unwrap();
        assert_eq!(grid_cells(&below), vec![None, Some(TxSize::Tx8x8)]);

        // And a unit ending exactly at a block's start does not claim the next block.
        let left = extract_transform_grid_from_parsed(&frame_with(
            32,
            16,
            vec![cu(0, 0, 16, 16, TxSize::Tx8x8)],
        ))
        .unwrap();
        assert_eq!(grid_cells(&left), vec![Some(TxSize::Tx8x8), None]);
    }

    // ---- extract_transform_grid over bytes ----

    #[test]
    fn bytes_without_a_tile_are_an_error_for_every_frame_index() {
        for bytes in [&[][..], &[0x00, 0x01, 0x02, 0x03][..], &[0xFF; 64][..]] {
            for frame_index in [0, 7, usize::MAX] {
                assert!(extract_transform_grid(bytes, frame_index).is_err());
            }
        }
    }

    /// The first IVF packet of the aomenc clip with nothing prepended (it carries its own
    /// sequence header).
    fn key_frame_bytes() -> Vec<u8> {
        let (_hdr, frames) = crate::ivf::parse_ivf_frames(FIXTURE).unwrap();
        frames[0].data.clone()
    }

    /// For a real key frame, the grid agrees with a brute-force restatement of "the transform size
    /// of the first unit, in decode order, that overlaps the block".
    #[test]
    fn the_real_key_frame_grid_matches_a_brute_force_overlap_search() {
        let data = key_frame_bytes();
        let parsed = ParsedFrame::parse(&data).unwrap();
        assert_eq!(
            (parsed.dimensions.width, parsed.dimensions.height),
            (320, 240)
        );
        let units = parse_all_coding_units(&parsed).unwrap();

        let grid = extract_transform_grid(&data, 0).unwrap();
        assert_eq!((grid.grid_w, grid.grid_h), (20, 15));
        assert_eq!(grid.tx_sizes.len(), 300);

        let mut distinct = std::collections::BTreeSet::new();
        for row in 0..15u32 {
            for col in 0..20u32 {
                let (bx, by) = (col * 16, row * 16);
                let expected = units
                    .iter()
                    .find(|u| {
                        u.x < bx + 16 && u.x + u.width > bx && u.y < by + 16 && u.y + u.height > by
                    })
                    .map(|u| u.tx_size);
                assert!(
                    expected.is_some(),
                    "a complete decode covers block ({col},{row})"
                );
                assert_eq!(grid.get(col, row), expected, "block ({col},{row})");
                distinct.insert(expected.map(|t| t.size()));
            }
        }
        assert!(
            distinct.len() >= 2,
            "the comparison needs more than one transform size to mean anything: {distinct:?}"
        );
    }

    #[test]
    fn the_frame_index_does_not_change_the_extracted_grid() {
        let data = key_frame_bytes();
        let a = extract_transform_grid(&data, 0).unwrap();
        let b = extract_transform_grid(&data, 999).unwrap();
        assert_eq!(a.tx_sizes, b.tx_sizes);
    }

    #[test]
    fn a_truncated_key_frame_never_panics() {
        let data = key_frame_bytes();
        // Cut at every length from the OBU headers into the tile, plus a few bit flips.
        for cut in (0..data.len()).step_by(37) {
            let _ = extract_transform_grid(&data[..cut], 0);
        }
        for pos in (0..data.len()).step_by(211) {
            let mut flipped = data.clone();
            flipped[pos] ^= 0x10;
            let _ = extract_transform_grid(&flipped, 0);
        }
    }
}
