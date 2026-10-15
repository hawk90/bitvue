//! The prediction mode of each grid block of a frame.

use super::super::cu_parser::parse_all_coding_units;
use super::super::parser::ParsedFrame;
use super::grid_build::build_grid_from_coding_units_spatial;
use crate::tile::PredictionMode;
use bitvue_engine::{
    limits::AV1_BLOCK_SIZE, limits::MAX_GRID_BLOCKS, limits::MAX_GRID_DIMENSION, BitvueError,
};

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

    if !super::super::provenance::has_decodable_tile(parsed) {
        return Err(super::super::provenance::no_decodable_tile());
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tile::CodingUnit;

    const FIXTURE: &[u8] = include_bytes!("../../../../../test_data/av1_aomenc_testsrc2.ivf");

    fn cu(x: u32, y: u32, w: u32, h: u32, mode: PredictionMode) -> CodingUnit {
        CodingUnit {
            mode,
            ..CodingUnit::new(x, y, w, h)
        }
    }

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

    fn cells(grid: &PredictionModeGrid) -> Vec<Option<PredictionMode>> {
        (0..grid.grid_h)
            .flat_map(|row| (0..grid.grid_w).map(move |col| (col, row)))
            .map(|(col, row)| grid.get(col, row))
            .collect()
    }

    const CAP: u32 = 512;

    // ---- PredictionModeGrid::new / get ----

    #[test]
    fn the_grid_size_rounds_partial_blocks_up_per_axis() {
        let g = PredictionModeGrid::new(33, 16, 16, 16, vec![None; 3]);
        assert_eq!((g.grid_w, g.grid_h), (3, 1));
        let g = PredictionModeGrid::new(32, 17, 16, 16, vec![None; 4]);
        assert_eq!((g.grid_w, g.grid_h), (2, 2));
        let g = PredictionModeGrid::new(64, 48, 32, 8, vec![None; 12]);
        assert_eq!((g.grid_w, g.grid_h), (2, 6));
        assert_eq!(
            (g.block_w, g.block_h, g.coded_width, g.coded_height),
            (32, 8, 64, 48)
        );
        let g = PredictionModeGrid::new(0, 0, 16, 16, Vec::new());
        assert_eq!((g.grid_w, g.grid_h), (0, 0));
        assert_eq!(g.get(0, 0), None);
    }

    #[test]
    fn get_is_row_major_and_none_outside_on_each_axis() {
        let modes = [
            PredictionMode::DcPred,
            PredictionMode::VPred,
            PredictionMode::HPred,
            PredictionMode::NewMv,
            PredictionMode::GlobalMv,
            PredictionMode::PaethPred,
        ];
        let grid =
            PredictionModeGrid::new(48, 32, 16, 16, modes.iter().map(|m| Some(*m)).collect());
        for (i, m) in modes.iter().enumerate() {
            assert_eq!(grid.get(i as u32 % 3, i as u32 / 3), Some(*m), "cell {i}");
        }
        for (c, r) in [(3, 0), (3, 1), (0, 2), (2, 2), (u32::MAX, 0), (0, u32::MAX)] {
            assert_eq!(grid.get(c, r), None, "({c},{r})");
        }
        // A `None` cell reads as None too.
        let grid = PredictionModeGrid::new(16, 16, 16, 16, vec![None]);
        assert_eq!(grid.get(0, 0), None);
    }

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "modes length mismatch: expected 4, got 3")]
    fn too_few_cells_is_caught_in_debug_builds() {
        let _ = PredictionModeGrid::new(32, 32, 16, 16, vec![None; 3]);
    }

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "modes length mismatch: expected 4, got 5")]
    fn too_many_cells_is_caught_in_debug_builds() {
        let _ = PredictionModeGrid::new(32, 32, 16, 16, vec![None; 5]);
    }

    /// The length check is skipped when an axis has more than 512 blocks; exactly 512 is checked.
    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "modes length mismatch")]
    fn a_width_of_exactly_the_cap_still_has_its_length_checked() {
        let _ = PredictionModeGrid::new(CAP, 1, 1, 1, vec![None; 5]);
    }

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "modes length mismatch")]
    fn a_height_of_exactly_the_cap_still_has_its_length_checked() {
        let _ = PredictionModeGrid::new(1, CAP, 1, 1, vec![None; 5]);
    }

    #[test]
    fn an_axis_beyond_the_cap_skips_the_length_check_and_reads_safely() {
        let n = CAP + 1;
        let mut modes = vec![None; n as usize];
        modes[n as usize - 1] = Some(PredictionMode::SmoothPred);
        let wide = PredictionModeGrid::new(n, 1, 1, 1, modes.clone());
        assert_eq!(wide.get(n - 1, 0), Some(PredictionMode::SmoothPred));
        assert_eq!(wide.get(n - 2, 0), None);
        let tall = PredictionModeGrid::new(1, n, 1, 1, modes);
        assert_eq!(tall.get(0, n - 1), Some(PredictionMode::SmoothPred));
        assert_eq!(tall.get(0, n), None);
        // Shorter than the grid says: past the end is None, not a panic.
        let short = PredictionModeGrid::new(n, 1, 1, 1, vec![Some(PredictionMode::DcPred); 3]);
        assert_eq!(short.get(2, 0), Some(PredictionMode::DcPred));
        assert_eq!(short.get(3, 0), None);
        // The same on the height axis: a vector shorter than the grid says is not checked.
        let tall_short = PredictionModeGrid::new(1, n, 1, 1, vec![Some(PredictionMode::DcPred); 3]);
        assert_eq!(tall_short.get(0, 2), Some(PredictionMode::DcPred));
        assert_eq!(tall_short.get(0, 3), None);
        // Longer than the grid: the extra cells are not reachable through row n.
        let long = PredictionModeGrid::new(
            n,
            1,
            1,
            1,
            vec![Some(PredictionMode::DcPred); 2 * n as usize],
        );
        assert_eq!(long.get(0, 1), None);
    }

    // ---- extract_prediction_mode_grid_from_parsed ----

    #[test]
    fn each_block_takes_the_mode_of_the_unit_covering_it() {
        let units = vec![
            cu(0, 0, 16, 16, PredictionMode::DcPred),
            cu(16, 0, 16, 16, PredictionMode::NewMv),
            cu(32, 0, 16, 16, PredictionMode::GlobalMv),
            cu(0, 16, 16, 16, PredictionMode::HPred),
            cu(16, 16, 32, 16, PredictionMode::NearestNearestMv),
        ];
        let grid = extract_prediction_mode_grid_from_parsed(&frame_with(40, 24, units)).unwrap();
        assert_eq!((grid.coded_width, grid.coded_height), (40, 24));
        assert_eq!(
            (grid.block_w, grid.block_h, grid.grid_w, grid.grid_h),
            (16, 16, 3, 2)
        );
        assert_eq!(
            cells(&grid),
            vec![
                Some(PredictionMode::DcPred),
                Some(PredictionMode::NewMv),
                Some(PredictionMode::GlobalMv),
                Some(PredictionMode::HPred),
                Some(PredictionMode::NearestNearestMv),
                Some(PredictionMode::NearestNearestMv),
            ]
        );
    }

    #[test]
    fn uncovered_blocks_are_none_and_a_large_unit_fills_what_it_covers() {
        let units = vec![cu(0, 0, 32, 32, PredictionMode::SmoothPred)];
        let grid = extract_prediction_mode_grid_from_parsed(&frame_with(64, 48, units)).unwrap();
        for row in 0..3 {
            for col in 0..4 {
                let expected = (col < 2 && row < 2).then_some(PredictionMode::SmoothPred);
                assert_eq!(grid.get(col, row), expected, "block ({col},{row})");
            }
        }
    }

    #[test]
    fn units_in_different_superblocks_are_found_and_the_first_unit_in_a_block_wins() {
        let units = vec![
            cu(64, 0, 64, 64, PredictionMode::NewMv),
            cu(0, 0, 8, 8, PredictionMode::VPred),
            cu(8, 0, 8, 8, PredictionMode::HPred),
        ];
        let grid = extract_prediction_mode_grid_from_parsed(&frame_with(128, 64, units)).unwrap();
        // Block 0 holds the two small units; the first listed (VPred) names it.
        assert_eq!(grid.get(0, 0), Some(PredictionMode::VPred));
        assert_eq!(
            grid.get(1, 0),
            None,
            "no unit starts in the second block of superblock 0"
        );
        for row in 0..4 {
            for col in 4..8 {
                assert_eq!(grid.get(col, row), Some(PredictionMode::NewMv));
            }
        }
    }

    #[test]
    fn a_unit_touching_a_block_edge_does_not_claim_it() {
        let right = extract_prediction_mode_grid_from_parsed(&frame_with(
            32,
            16,
            vec![cu(16, 0, 16, 16, PredictionMode::NewMv)],
        ))
        .unwrap();
        assert_eq!(cells(&right), vec![None, Some(PredictionMode::NewMv)]);
        let below = extract_prediction_mode_grid_from_parsed(&frame_with(
            16,
            32,
            vec![cu(0, 16, 16, 16, PredictionMode::NewMv)],
        ))
        .unwrap();
        assert_eq!(cells(&below), vec![None, Some(PredictionMode::NewMv)]);
    }

    #[test]
    fn exactly_the_maximum_block_count_is_accepted_and_one_more_is_rejected() {
        let grid =
            extract_prediction_mode_grid_from_parsed(&frame_with(8192, 8192, Vec::new())).unwrap();
        assert_eq!((grid.grid_w, grid.grid_h), (512, 512));
        assert_eq!(grid.modes.len(), 512 * 512);
        for (w, h) in [(8193, 8192), (8192, 8193), (16 * (512 * 512 + 1), 16)] {
            let err = extract_prediction_mode_grid_from_parsed(&frame_with(w, h, Vec::new()))
                .unwrap_err();
            assert!(
                err.to_string().contains("Grid exceeds maximum size"),
                "{w}x{h}: {err}"
            );
        }
    }

    #[test]
    fn a_block_count_that_overflows_is_rejected_and_is_checked_before_the_tile() {
        let err =
            extract_prediction_mode_grid_from_parsed(&frame_with(u32::MAX, u32::MAX, Vec::new()))
                .unwrap_err();
        assert!(
            err.to_string().contains("Grid dimensions too large"),
            "{err}"
        );

        let mut parsed = frame_with(16000, 16000, Vec::new());
        parsed.tile_data = std::sync::Arc::from([]);
        let err = extract_prediction_mode_grid_from_parsed(&parsed).unwrap_err();
        assert!(
            err.to_string().contains("Grid exceeds maximum size"),
            "{err}"
        );
    }

    #[test]
    fn a_frame_without_tile_bytes_is_an_error_even_with_cached_units() {
        let mut parsed = frame_with(32, 32, vec![cu(0, 0, 16, 16, PredictionMode::DcPred)]);
        parsed.tile_data = std::sync::Arc::from([]);
        let err = extract_prediction_mode_grid_from_parsed(&parsed).unwrap_err();
        assert!(err.to_string().contains("no decodable tile data"), "{err}");
    }

    #[test]
    fn bytes_without_a_tile_are_an_error_for_every_frame_index() {
        for bytes in [&[][..], &[0x00, 0x01, 0x02, 0x03][..], &[0xFF; 64][..]] {
            for frame_index in [0, 7, usize::MAX] {
                assert!(extract_prediction_mode_grid(bytes, frame_index).is_err());
            }
        }
    }

    #[test]
    fn the_real_key_frame_grid_matches_a_brute_force_overlap_search() {
        let (_hdr, frames) = crate::ivf::parse_ivf_frames(FIXTURE).unwrap();
        let data = frames[0].data.clone();
        let parsed = ParsedFrame::parse(&data).unwrap();
        let units = parse_all_coding_units(&parsed).unwrap();
        let grid = extract_prediction_mode_grid(&data, 0).unwrap();
        assert_eq!((grid.grid_w, grid.grid_h), (20, 15));
        let mut seen = std::collections::BTreeSet::new();
        for row in 0..15u32 {
            for col in 0..20u32 {
                let (bx, by) = (col * 16, row * 16);
                let expected = units
                    .iter()
                    .find(|u| {
                        u.x < bx + 16 && u.x + u.width > bx && u.y < by + 16 && u.y + u.height > by
                    })
                    .map(|u| u.mode);
                assert!(expected.is_some(), "block ({col},{row}) is covered");
                assert_eq!(grid.get(col, row), expected, "block ({col},{row})");
                seen.insert(format!("{:?}", expected));
            }
        }
        assert!(
            seen.len() >= 2,
            "more than one mode in a real key frame: {seen:?}"
        );
    }
}
