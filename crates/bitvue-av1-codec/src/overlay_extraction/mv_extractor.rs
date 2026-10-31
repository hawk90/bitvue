//! Motion Vector grid extraction
//!
//! Provides functions to extract motion vector data from AV1 bitstreams.

use bitvue_engine::{
    mv_overlay::{BlockMode, MVGrid, MotionVector as CoreMV},
    BitvueError,
};

use super::cu_parser::parse_all_coding_units;
use super::cu_spatial_index::CuSpatialIndex;
use super::parser::ParsedFrame;
use super::OVERLAY_BLOCK_SIZE;

/// Converts a decoded 1/8-sample `MotionVector` to the overlay grid's quarter-sample units
/// (rounding half away from zero).
fn overlay_mv(mv: crate::tile::MotionVector) -> CoreMV {
    // i64 so that i32::MAX does not overflow when the rounding half is added (#235)
    let to_qpel = |v: i32| ((v as i64 + v.signum() as i64) / 2) as i32;
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

    // ---- overlay_mv: 1/8-sample decoded vectors to quarter-sample grid units ----

    #[test]
    fn overlay_mv_handles_the_extreme_values_of_a_hand_built_vector() {
        // #235: v + signum overflowed at i32::MAX
        let mv = overlay_mv(crate::tile::MotionVector::new(i32::MAX, i32::MIN));
        assert_eq!((mv.dx_qpel, mv.dy_qpel), (1 << 30, -(1 << 30)));
        let mv = overlay_mv(crate::tile::MotionVector::new(i32::MAX - 1, i32::MIN + 1));
        assert_eq!((mv.dx_qpel, mv.dy_qpel), ((1 << 30) - 1, -(1 << 30)));
    }

    #[test]
    fn overlay_mv_halves_and_rounds_half_away_from_zero_for_each_axis_separately() {
        let table = [
            (0, 0),
            (1, 1),
            (2, 1),
            (3, 2),
            (4, 2),
            (5, 3),
            (6, 3),
            (7, 4),
            (8, 4),
            (1000, 500),
            (1001, 501),
            (-1, -1),
            (-2, -1),
            (-3, -2),
            (-4, -2),
            (-5, -3),
            (-1001, -501),
        ];
        for (eighth, quarter) in table {
            let mv = overlay_mv(crate::tile::MotionVector::new(eighth, 0));
            assert_eq!((mv.dx_qpel, mv.dy_qpel), (quarter, 0), "x {eighth}");
            let mv = overlay_mv(crate::tile::MotionVector::new(0, eighth));
            assert_eq!((mv.dx_qpel, mv.dy_qpel), (0, quarter), "y {eighth}");
        }
        let mv = overlay_mv(crate::tile::MotionVector::new(3, -5));
        assert_eq!((mv.dx_qpel, mv.dy_qpel), (2, -3));
    }

    // ---- extract_mv_grid_from_parsed over synthetic decoded coding units ----

    use crate::tile::{CodingUnit, MotionVector, PredictionMode, RefFrame, TxSize};

    fn cu(x: u32, y: u32, w: u32, h: u32) -> CodingUnit {
        CodingUnit {
            x,
            y,
            width: w,
            height: h,
            skip: false,
            skip_mode: false,
            segment_id: 0,
            mode: PredictionMode::DcPred,
            ref_frames: [RefFrame::Intra, RefFrame::Intra],
            use_intrabc: false,
            mv: [MotionVector::zero(), MotionVector::zero()],
            tx_size: TxSize::from_dimensions(w, h),
            tx_blocks: None,
            qp: None,
            residual: None,
            palette: crate::tile::PaletteInfo::default(),
        }
    }

    fn inter(x: u32, y: u32, mv0: (i32, i32)) -> CodingUnit {
        CodingUnit {
            mode: PredictionMode::NewMv,
            ref_frames: [RefFrame::Last, RefFrame::Intra],
            mv: [MotionVector::new(mv0.0, mv0.1), MotionVector::zero()],
            ..cu(x, y, 64, 64)
        }
    }

    fn compound(x: u32, y: u32, mv0: (i32, i32), mv1: (i32, i32)) -> CodingUnit {
        CodingUnit {
            mode: PredictionMode::NearestNearestMv,
            ref_frames: [RefFrame::Last, RefFrame::AltRef2],
            mv: [
                MotionVector::new(mv0.0, mv0.1),
                MotionVector::new(mv1.0, mv1.1),
            ],
            ..cu(x, y, 64, 64)
        }
    }

    fn frame_with(width: u32, height: u32, units: Vec<CodingUnit>) -> ParsedFrame {
        let mut parsed = ParsedFrame::parse(&[]).unwrap();
        parsed.dimensions.width = width;
        parsed.dimensions.height = height;
        parsed.tile_data = std::sync::Arc::from([0u8]);
        parsed.decoded = Some(super::super::ParsedCodingUnits {
            units: std::sync::Arc::new(units),
            outcome: Default::default(),
        });
        parsed
    }

    fn cell(grid: &MVGrid, col: u32, row: u32) -> (CoreMV, CoreMV, BlockMode) {
        (
            grid.get_l0(col, row).unwrap(),
            grid.get_l1(col, row).unwrap(),
            grid.get_mode(col, row).unwrap(),
        )
    }

    #[test]
    fn a_frame_without_tile_data_is_an_error_even_if_coding_units_are_cached() {
        let mut parsed = frame_with(64, 64, vec![inter(0, 0, (2, 2))]);
        parsed.tile_data = std::sync::Arc::from([]);
        match extract_mv_grid_from_parsed(&parsed) {
            Err(BitvueError::Decode(message)) => assert!(message.contains("no decodable tile")),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn every_block_kind_gets_its_mode_and_its_vectors_in_quarter_samples() {
        let units = vec![
            cu(0, 0, 64, 64), // intra
            CodingUnit {
                use_intrabc: true,
                mv: [MotionVector::new(8, 8), MotionVector::zero()],
                ..cu(64, 0, 64, 64)
            },
            inter(128, 0, (3, -5)), // single reference
            CodingUnit {
                skip: true,
                ..inter(0, 64, (4, 4))
            }, // skipped inter
            compound(64, 64, (2, 6), (-1, 7)), // compound
            CodingUnit {
                skip: true,
                ..compound(128, 64, (2, 2), (2, 2))
            }, // skipped compound
        ];
        let grid = extract_mv_grid_from_parsed(&frame_with(192, 128, units)).unwrap();
        assert_eq!(grid.block_count(), 6);

        let intra = cell(&grid, 0, 0);
        assert!(intra.0.is_missing() && intra.1.is_missing());
        assert_eq!(intra.2, BlockMode::Intra);

        // Intra block copy: its own mode, and no vectors reported.
        let bc = cell(&grid, 1, 0);
        assert!(bc.0.is_missing() && bc.1.is_missing());
        assert_eq!(bc.2, BlockMode::IntraBc);

        let single = cell(&grid, 2, 0);
        assert_eq!((single.0.dx_qpel, single.0.dy_qpel), (2, -3));
        assert!(single.1.is_missing(), "no second reference, no L1 vector");
        assert_eq!(single.2, BlockMode::Inter);

        let skipped = cell(&grid, 0, 1);
        assert_eq!((skipped.0.dx_qpel, skipped.0.dy_qpel), (2, 2));
        assert!(skipped.1.is_missing());
        assert_eq!(skipped.2, BlockMode::Skip);

        let both = cell(&grid, 1, 1);
        assert_eq!((both.0.dx_qpel, both.0.dy_qpel), (1, 3));
        assert_eq!((both.1.dx_qpel, both.1.dy_qpel), (-1, 4));
        assert_eq!(both.2, BlockMode::Compound);

        // Skip wins over compound for the mode, and both vectors are still reported.
        let skipped_both = cell(&grid, 2, 1);
        assert_eq!(skipped_both.2, BlockMode::Skip);
        assert_eq!((skipped_both.0.dx_qpel, skipped_both.1.dx_qpel), (1, 1));
    }

    #[test]
    fn an_intra_block_stays_intra_whatever_its_skip_flag_and_stored_vectors() {
        let unit = CodingUnit {
            skip: true,
            mv: [MotionVector::new(40, 40), MotionVector::new(40, 40)],
            ..cu(0, 0, 64, 64)
        };
        let grid = extract_mv_grid_from_parsed(&frame_with(64, 64, vec![unit])).unwrap();
        let c = cell(&grid, 0, 0);
        assert_eq!(c.2, BlockMode::Intra);
        assert!(c.0.is_missing() && c.1.is_missing());
    }

    #[test]
    fn cells_no_decoded_block_covers_are_missing_with_mode_none() {
        // Decoding stopped after the first block: the other three cells have no block.
        let grid =
            extract_mv_grid_from_parsed(&frame_with(128, 128, vec![inter(0, 0, (2, 2))])).unwrap();
        assert_eq!(cell(&grid, 0, 0).2, BlockMode::Inter);
        for (col, row) in [(1, 0), (0, 1), (1, 1)] {
            let c = cell(&grid, col, row);
            assert!(c.0.is_missing() && c.1.is_missing(), "({col},{row})");
            assert_eq!(c.2, BlockMode::None, "({col},{row})");
        }
        // No blocks at all.
        let grid = extract_mv_grid_from_parsed(&frame_with(64, 64, Vec::new())).unwrap();
        assert_eq!(cell(&grid, 0, 0).2, BlockMode::None);
    }

    #[test]
    fn a_large_block_fills_every_cell_it_covers_and_the_grid_rounds_partial_cells_up() {
        // 130x70 needs 3x2 cells of 64; one 128x64 inter block covers the first two cells of row 0.
        let wide = CodingUnit {
            width: 128,
            ..inter(0, 0, (6, 6))
        };
        let grid = extract_mv_grid_from_parsed(&frame_with(130, 70, vec![wide])).unwrap();
        assert_eq!(grid.block_count(), 6);
        assert_eq!(cell(&grid, 0, 0).2, BlockMode::Inter);
        assert_eq!(cell(&grid, 1, 0).2, BlockMode::Inter);
        assert_eq!(cell(&grid, 1, 0).0.dx_qpel, 3);
        assert_eq!(cell(&grid, 2, 0).2, BlockMode::None);
        assert_eq!(cell(&grid, 0, 1).2, BlockMode::None);
        assert!(grid.get_mode(3, 0).is_none() && grid.get_mode(0, 2).is_none());
        // The grid keeps the frame size.
        let (w, h) = (grid.coded_width, grid.coded_height);
        assert_eq!((w, h), (130, 70));
    }

    #[test]
    fn the_partial_last_column_and_row_each_get_their_own_cell() {
        // 130x70: 3 columns x 2 rows of 64, the last column is 2 pixels wide and the last row 6.
        let units = vec![
            cu(0, 0, 64, 64),
            inter(64, 0, (2, 2)),
            CodingUnit {
                skip: true,
                ..inter(128, 0, (4, 4))
            },
            inter(0, 64, (6, 6)),
            compound(64, 64, (8, 8), (10, 10)),
            cu(128, 64, 64, 64),
        ];
        let grid = extract_mv_grid_from_parsed(&frame_with(130, 70, units)).unwrap();
        let modes: Vec<BlockMode> = (0..2)
            .flat_map(|row| (0..3).map(move |col| (col, row)))
            .map(|(col, row)| grid.get_mode(col, row).unwrap())
            .collect();
        assert_eq!(
            modes,
            vec![
                BlockMode::Intra,
                BlockMode::Inter,
                BlockMode::Skip,
                BlockMode::Inter,
                BlockMode::Compound,
                BlockMode::Intra
            ]
        );
        assert_eq!(grid.get_l0(2, 0).unwrap().dx_qpel, 2);
        assert_eq!(grid.get_l0(0, 1).unwrap().dx_qpel, 3);
        assert_eq!(grid.get_l1(1, 1).unwrap().dx_qpel, 5);
    }

    #[test]
    fn a_tall_frame_has_one_cell_per_row_of_its_height() {
        // 64x200: one column, four rows (the last one 8 pixels tall).
        let units = vec![
            cu(0, 0, 64, 64),
            inter(0, 64, (2, 2)),
            compound(0, 128, (2, 2), (2, 2)),
            CodingUnit {
                skip: true,
                ..inter(0, 192, (2, 2))
            },
        ];
        let grid = extract_mv_grid_from_parsed(&frame_with(64, 200, units)).unwrap();
        assert_eq!(grid.block_count(), 4);
        let modes: Vec<BlockMode> = (0..4).map(|row| grid.get_mode(0, row).unwrap()).collect();
        assert_eq!(
            modes,
            vec![
                BlockMode::Intra,
                BlockMode::Inter,
                BlockMode::Compound,
                BlockMode::Skip
            ]
        );
        assert!(grid.get_mode(0, 4).is_none());
    }

    #[test]
    fn a_frame_size_whose_grid_does_not_fit_in_u32_is_an_error_not_a_panic() {
        let parsed = frame_with(u32::MAX, u32::MAX, Vec::new());
        match extract_mv_grid_from_parsed(&parsed) {
            Err(BitvueError::Decode(message)) => {
                assert!(message.contains("too large"), "{message}")
            }
            other => panic!("{other:?}"),
        }
        // Just inside the limit is not rejected by that check (it fails later or succeeds).
        let ok = frame_with(64 * 1000, 64 * 1000, Vec::new());
        assert!(
            !matches!(extract_mv_grid_from_parsed(&ok), Err(BitvueError::Decode(m)) if m.contains("too large"))
        );
    }

    #[test]
    fn extract_mv_grid_without_obus_that_hold_tile_data_is_an_error() {
        assert!(extract_mv_grid(&[], 0).is_err());
        assert!(extract_mv_grid(&[0x12, 0x00], 3).is_err());
    }
}
