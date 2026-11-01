//! Per-block residual "energy" grid extraction -- real, data-driven replacement for the
//! QP-only Efficiency Map proxy that used to live entirely in the frontend
//! (`Av1EfficiencyMapRenderer.tsx`: `bpp ≈ (64 - QP) * scale_factor`).
//!
//! `energy_bpp` per cell is `sum_abs_level / block_area` (see
//! `crate::symbol::ResidualBlockStats`'s doc) -- still not literal entropy-coded bit count
//! (this crate's residual decode uses context-independent representative CDFs, not the real
//! neighbor-context-adaptive ones, per `SymbolDecoder::read_residual_block`'s doc), but unlike
//! the QP proxy it reflects each block's actual decoded residual magnitude rather than just the
//! frame's quantization step, so "hot"/"cold" comparisons between blocks are now data-driven.
//! Reuses the exact `parse_all_coding_units`/`CuSpatialIndex` machinery `qp_extractor.rs` already
//! uses for the same grid-cell-to-CU mapping problem.

use super::cu_parser::parse_all_coding_units;
use super::cu_spatial_index::CuSpatialIndex;
use super::parser::ParsedFrame;
use super::OVERLAY_BLOCK_SIZE;
use bitvue_engine::BitvueError;

/// Dense grid of per-block residual-energy values, same cell layout as `QPGrid`
/// (`bitvue_engine::qp_heatmap::QPGrid`) so the frontend can reuse the same grid-indexing code.
#[derive(Debug, Clone, PartialEq)]
pub struct EnergyGrid {
    pub grid_w: u32,
    pub grid_h: u32,
    pub block_w: u32,
    pub block_h: u32,
    /// Bits-per-pixel-like energy value per cell. 0.0 is a real "no residual" value (skip
    /// blocks, or no tile data to parse), not a missing-data marker -- every cell always has a
    /// value.
    pub energy_bpp: Vec<f64>,
}

/// Extract an energy grid from an already-parsed frame. Falls back to all-zero cells (rather
/// than erroring) when tile data is absent or CU parsing fails, matching
/// `extract_qp_grid_from_parsed`'s fallback-to-scaffold behavior for the same conditions.
pub fn extract_energy_grid_from_parsed(parsed: &ParsedFrame) -> Result<EnergyGrid, BitvueError> {
    let block_w = OVERLAY_BLOCK_SIZE;
    let block_h = OVERLAY_BLOCK_SIZE;
    let grid_w = parsed.dimensions.width.div_ceil(block_w);
    let grid_h = parsed.dimensions.height.div_ceil(block_h);

    let total_blocks = grid_w.checked_mul(grid_h).ok_or_else(|| {
        BitvueError::Decode(format!("Grid dimensions too large: {}x{}", grid_w, grid_h))
    })? as usize;

    // Any tile that exists is decoded: a small one can still hold residual (a 10-byte guess here
    // reported 0.0 for tiles of 4 and 9 bytes that decode to non-zero energy, #239). The other
    // extractors use `provenance::has_decodable_tile` for the same reason.
    if !parsed.has_tile_data() {
        return Ok(EnergyGrid {
            grid_w,
            grid_h,
            block_w,
            block_h,
            energy_bpp: vec![0.0; total_blocks],
        });
    }

    let coding_units = match parse_all_coding_units(parsed) {
        Ok(cus) => cus,
        Err(e) => {
            tracing::warn!(
                "Failed to parse coding units for energy grid: {}, using zeros",
                e
            );
            return Ok(EnergyGrid {
                grid_w,
                grid_h,
                block_w,
                block_h,
                energy_bpp: vec![0.0; total_blocks],
            });
        }
    };

    let spatial_index = CuSpatialIndex::new(&coding_units, grid_w, grid_h, block_w, block_h);
    let mut energy_bpp = Vec::with_capacity(total_blocks);
    for grid_y in 0..grid_h {
        for grid_x in 0..grid_w {
            let val = spatial_index
                .get_cu_index(grid_x, grid_y)
                .map(|idx| {
                    let cu = &coding_units[idx];
                    let area = cu.width as f64 * cu.height as f64;
                    if area <= 0.0 {
                        0.0
                    } else {
                        cu.residual
                            .map(|r| r.sum_abs_level as f64 / area)
                            .unwrap_or(0.0)
                    }
                })
                .unwrap_or(0.0);
            energy_bpp.push(val);
        }
    }

    Ok(EnergyGrid {
        grid_w,
        grid_h,
        block_w,
        block_h,
        energy_bpp,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Minimal OBU stream (temporal delimiter + placeholder sequence/frame headers, no real
    /// tile data) -- mirrors `qp_extractor.rs`'s `create_test_obu_data` helper.
    fn create_test_obu_data() -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&[0x12, 0x00]); // Temporal delimiter OBU
        data.extend_from_slice(&[0x0A, 0x14]); // Sequence header OBU header
        data.extend_from_slice(&[0x00u8; 20]); // Payload placeholder
        data.extend_from_slice(&[0x1A, 0x0A]); // Frame header OBU header
        data.extend_from_slice(&[0x00u8; 10]); // Payload placeholder
        data
    }

    #[test]
    fn energy_grid_falls_back_to_zeros_without_tile_data() {
        // No real tile data in this minimal stream -- should produce an all-zero grid of the
        // scaffold dimensions rather than erroring.
        let obu_data = create_test_obu_data();
        let parsed = ParsedFrame::parse(&obu_data).expect("scaffold parse should succeed");
        let grid = extract_energy_grid_from_parsed(&parsed).unwrap();
        assert_eq!(grid.grid_w, grid.energy_bpp.len() as u32 / grid.grid_h);
        assert!(!grid.energy_bpp.is_empty());
        assert!(grid.energy_bpp.iter().all(|&v| v == 0.0));
    }

    /// One-frame aomenc clips (`--end-usage=q --cq-level=63`) of a 16x16 and a 32x32 gradient,
    /// generated for this test (temporal delimiter, sequence header, frame OBU): their tiles are
    /// 4 and 5 bytes and hold one coding unit with a non-zero residual.
    const TINY_16X16: [u8; 24] = [
        0x12, 0x00, 0x0a, 0x06, 0x18, 0x0c, 0xff, 0xdb, 0x00, 0x80, 0x32, 0x0c, 0x1f, 0xf0, 0x00,
        0x00, 0x58, 0x10, 0x00, 0x40, 0x22, 0xaa, 0x99, 0xb0,
    ];
    const TINY_32X32: [u8; 25] = [
        0x12, 0x00, 0x0a, 0x06, 0x18, 0x11, 0x3f, 0xf6, 0xc0, 0x20, 0x32, 0x0d, 0x47, 0xfc, 0x00,
        0x00, 0x16, 0x5c, 0x00, 0x00, 0x03, 0x75, 0x1a, 0xb4, 0xf2,
    ];

    #[test]
    fn a_tile_of_ten_bytes_or_fewer_is_decoded_not_reported_as_zero_energy() {
        for (name, packet, expected_tile_bytes) in
            [("16x16", &TINY_16X16[..], 4), ("32x32", &TINY_32X32[..], 5)]
        {
            let parsed = ParsedFrame::parse(packet).unwrap();
            assert_eq!(parsed.tile_data.len(), expected_tile_bytes, "{name}");
            let units = parse_all_coding_units(&parsed).unwrap();
            assert_eq!(units.len(), 1, "{name}");
            let unit = &units[0];
            let sum = unit
                .residual
                .expect("the unit has a residual")
                .sum_abs_level;
            assert!(sum > 0, "{name}: the clip must carry residual");
            let expected = sum as f64 / (unit.width as f64 * unit.height as f64);

            let grid = extract_energy_grid_from_parsed(&parsed).unwrap();
            assert_eq!(grid.energy_bpp, vec![expected], "{name}");
        }
    }

    // ---- extract_energy_grid_from_parsed over synthetic decoded coding units ----

    use crate::symbol::ResidualBlockStats;
    use crate::tile::CodingUnit;

    const FIXTURE: &[u8] = include_bytes!("../../../../test_data/av1_aomenc_testsrc2.ivf");

    fn cu(x: u32, y: u32, w: u32, h: u32, sum_abs_level: Option<u64>) -> CodingUnit {
        CodingUnit {
            residual: sum_abs_level.map(|sum_abs_level| ResidualBlockStats {
                all_zero: sum_abs_level == 0,
                nonzero_count: 1,
                sum_abs_level,
                max_level: 1,
                dc_sign_value: None,
            }),
            ..CodingUnit::new(x, y, w, h)
        }
    }

    /// A frame of `width` x `height` with `tile_len` tile bytes whose decode is cached as `units`.
    fn frame_with(width: u32, height: u32, tile_len: usize, units: Vec<CodingUnit>) -> ParsedFrame {
        let mut parsed = ParsedFrame::parse(&[]).unwrap();
        parsed.dimensions.width = width;
        parsed.dimensions.height = height;
        parsed.tile_data = std::sync::Arc::from(vec![0u8; tile_len]);
        parsed.decoded = Some(super::super::ParsedCodingUnits {
            units: std::sync::Arc::new(units),
            outcome: Default::default(),
        });
        parsed
    }

    fn grid_of(width: u32, height: u32, units: Vec<CodingUnit>) -> EnergyGrid {
        extract_energy_grid_from_parsed(&frame_with(width, height, 64, units)).unwrap()
    }

    #[test]
    fn each_cell_is_the_residual_sum_over_the_area_of_its_unit() {
        // 128x128 frame -> 2x2 cells of 64. Four units with different sums and areas.
        let grid = grid_of(
            128,
            128,
            vec![
                cu(0, 0, 64, 64, Some(4096)),   // 1.0
                cu(64, 0, 64, 64, Some(1024)),  // 0.25
                cu(0, 64, 64, 64, Some(0)),     // all-zero residual
                cu(64, 64, 64, 64, Some(2048)), // 0.5
            ],
        );
        assert_eq!((grid.grid_w, grid.grid_h), (2, 2));
        assert_eq!((grid.block_w, grid.block_h), (64, 64));
        assert_eq!(grid.energy_bpp, vec![1.0, 0.25, 0.0, 0.5]);
    }

    #[test]
    fn the_area_is_the_units_not_the_cells() {
        // A 32x32 unit inside a 64x64 cell: 512 / (32*32), not 512 / (64*64).
        let grid = grid_of(64, 64, vec![cu(0, 0, 32, 32, Some(512))]);
        assert_eq!(grid.energy_bpp, vec![0.5]);

        // Rectangular units use width * height.
        let grid = grid_of(64, 64, vec![cu(0, 0, 64, 16, Some(512))]);
        assert_eq!(grid.energy_bpp, vec![0.5]);
        let grid = grid_of(64, 64, vec![cu(0, 0, 16, 64, Some(256))]);
        assert_eq!(grid.energy_bpp, vec![0.25]);
    }

    #[test]
    fn a_large_unit_gives_every_cell_it_covers_the_same_value() {
        // 128x128 unit with 8192 / 16384 = 0.5 over a 192x128 frame (3x2 cells): left 2x2 cells.
        let grid = grid_of(192, 128, vec![cu(0, 0, 128, 128, Some(8192))]);
        assert_eq!((grid.grid_w, grid.grid_h), (3, 2));
        assert_eq!(grid.energy_bpp, vec![0.5, 0.5, 0.0, 0.5, 0.5, 0.0]);
    }

    #[test]
    fn skipped_units_and_uncovered_cells_are_zero_and_every_cell_has_a_value() {
        // A skipped unit has no residual stats at all; the cell right of it has no unit.
        let grid = grid_of(
            192,
            64,
            vec![cu(0, 0, 64, 64, None), cu(128, 0, 64, 64, Some(4096))],
        );
        assert_eq!(grid.energy_bpp, vec![0.0, 0.0, 1.0]);
    }

    #[test]
    fn the_first_unit_covering_a_cell_names_it() {
        let grid = grid_of(
            64,
            64,
            vec![cu(0, 0, 32, 32, Some(1024)), cu(32, 32, 32, 32, Some(4096))],
        );
        assert_eq!(grid.energy_bpp, vec![1.0]);
    }

    #[test]
    fn partial_frames_round_the_grid_up_and_read_edge_units() {
        // 100x70 -> 2x2 cells; the units in the last column/row are cut by the frame edge.
        let grid = grid_of(
            100,
            70,
            vec![
                cu(0, 0, 64, 64, Some(4096)),
                cu(64, 0, 64, 64, Some(2048)),
                cu(0, 64, 64, 64, Some(1024)),
                cu(64, 64, 64, 64, Some(8192)),
            ],
        );
        assert_eq!((grid.grid_w, grid.grid_h), (2, 2));
        assert_eq!(grid.energy_bpp, vec![1.0, 0.5, 0.25, 2.0]);
    }

    #[test]
    fn cells_are_in_row_major_order_on_a_wide_and_a_tall_frame() {
        // 3 columns x 1 row, then 1 column x 3 rows, with distinct values per cell.
        let wide = grid_of(
            192,
            64,
            vec![
                cu(0, 0, 64, 64, Some(4096)),
                cu(64, 0, 64, 64, Some(8192)),
                cu(128, 0, 64, 64, Some(12288)),
            ],
        );
        assert_eq!(wide.energy_bpp, vec![1.0, 2.0, 3.0]);
        let tall = grid_of(
            64,
            192,
            vec![
                cu(0, 0, 64, 64, Some(4096)),
                cu(0, 64, 64, 64, Some(8192)),
                cu(0, 128, 64, 64, Some(12288)),
            ],
        );
        assert_eq!(tall.energy_bpp, vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn a_zero_area_unit_gives_zero_not_nan_or_infinity() {
        for (w, h) in [(0, 0), (0, 64), (64, 0)] {
            let grid = grid_of(64, 64, vec![cu(0, 0, w, h, Some(100))]);
            assert_eq!(grid.energy_bpp, vec![0.0], "{w}x{h}");
        }
    }

    #[test]
    fn the_largest_residual_sum_stays_finite() {
        let grid = grid_of(64, 64, vec![cu(0, 0, 4, 4, Some(u64::MAX))]);
        assert!(grid.energy_bpp[0].is_finite() && grid.energy_bpp[0] > 1e18);
    }

    #[test]
    fn a_unit_beyond_the_frame_is_clamped_into_the_last_cell() {
        // The index clamps out-of-frame units to the last cell instead of dropping them.
        let grid = grid_of(64, 64, vec![cu(640, 640, 64, 64, Some(4096))]);
        assert_eq!(grid.energy_bpp, vec![1.0]);
    }

    // ---- fallbacks to an all-zero grid ----

    #[test]
    fn a_frame_without_tile_bytes_is_not_decoded() {
        let units = vec![cu(0, 0, 64, 64, Some(4096))];
        let grid = extract_energy_grid_from_parsed(&frame_with(64, 128, 0, units.clone())).unwrap();
        assert_eq!(grid.energy_bpp, vec![0.0, 0.0]);
        // The same units with tile bytes present are read (64 bytes: well above any size cut-off).
        let grid = extract_energy_grid_from_parsed(&frame_with(64, 128, 64, units)).unwrap();
        assert_eq!(grid.energy_bpp, vec![1.0, 0.0]);
    }

    #[test]
    fn the_fallback_grid_has_the_frames_dimensions() {
        let grid = extract_energy_grid_from_parsed(&frame_with(130, 65, 0, Vec::new())).unwrap();
        assert_eq!((grid.grid_w, grid.grid_h), (3, 2));
        assert_eq!((grid.block_w, grid.block_h), (64, 64));
        assert_eq!(grid.energy_bpp, vec![0.0; 6]);
    }

    #[test]
    fn a_zero_sized_frame_gives_an_empty_grid_with_or_without_tile_bytes() {
        for len in [0, 64] {
            let grid = extract_energy_grid_from_parsed(&frame_with(0, 0, len, Vec::new())).unwrap();
            assert_eq!((grid.grid_w, grid.grid_h), (0, 0));
            assert!(grid.energy_bpp.is_empty());
        }
    }

    #[test]
    fn dimensions_whose_cell_count_overflows_are_an_error() {
        // 2^26 x 2^26 cells of 64 pixels: the product does not fit in u32.
        let err = extract_energy_grid_from_parsed(&frame_with(u32::MAX, u32::MAX, 0, Vec::new()))
            .unwrap_err();
        assert!(
            err.to_string().contains("Grid dimensions too large"),
            "{err}"
        );
    }

    /// A tile layout that does not fit its bytes makes the coding-unit parse fail; the grid is
    /// then all zeros of the frame's size, not an error.
    #[test]
    fn a_failed_unit_parse_gives_an_all_zero_grid() {
        let mut parsed = ParsedFrame::parse(&[]).unwrap();
        parsed.dimensions.width = 128;
        parsed.dimensions.height = 64;
        parsed.tile_data = std::sync::Arc::from(vec![0u8; 16]);
        // Two tile columns, each size field 255 bytes wide: the first tile's size is out of reach.
        parsed.tiles = Some(crate::frame_header::TileLayout {
            col_starts_sb: vec![0, 1, 2],
            row_starts_sb: vec![0, 1],
            tile_cols_log2: 1,
            tile_rows_log2: 0,
            context_update_tile_id: 0,
            tile_size_bytes: 255,
        });
        assert!(
            parse_all_coding_units(&parsed).is_err(),
            "setup must fail to parse"
        );

        let grid = extract_energy_grid_from_parsed(&parsed).unwrap();
        assert_eq!((grid.grid_w, grid.grid_h), (2, 1));
        assert_eq!(grid.energy_bpp, vec![0.0, 0.0]);
    }

    // ---- real clip ----

    fn clip_packet(index: usize) -> ParsedFrame {
        use crate::obu::{ObuIterator, ObuType};
        let (_hdr, frames) = crate::ivf::parse_ivf_frames(FIXTURE).unwrap();
        let mut seq = Vec::new();
        let mut it = ObuIterator::new(&frames[0].data);
        while let Some(Ok(found)) = it.next_obu_with_offset() {
            if found.obu.header.obu_type == ObuType::SequenceHeader {
                seq = frames[0].data[found.offset..found.offset + found.consumed].to_vec();
            }
        }
        ParsedFrame::parse(&[seq.as_slice(), frames[index].data.as_slice()].concat()).unwrap()
    }

    /// The real key frame's grid equals a brute-force "first unit overlapping the cell" search.
    #[test]
    fn the_real_key_frame_grid_matches_a_brute_force_overlap_search() {
        let parsed = clip_packet(0);
        let units = parse_all_coding_units(&parsed).unwrap();
        let grid = extract_energy_grid_from_parsed(&parsed).unwrap();
        assert_eq!((grid.grid_w, grid.grid_h), (5, 4));
        assert_eq!(grid.energy_bpp.len(), 20);

        for row in 0..4u32 {
            for col in 0..5u32 {
                let (bx, by) = (col * 64, row * 64);
                let first = units
                    .iter()
                    .find(|u| {
                        u.x < bx + 64 && u.x + u.width > bx && u.y < by + 64 && u.y + u.height > by
                    })
                    .expect("a complete decode covers every cell");
                let expected = first.residual.map_or(0.0, |r| {
                    r.sum_abs_level as f64 / (first.width * first.height) as f64
                });
                assert_eq!(
                    grid.energy_bpp[(row * 5 + col) as usize],
                    expected,
                    "cell ({col},{row})"
                );
            }
        }
        let distinct: std::collections::BTreeSet<u64> =
            grid.energy_bpp.iter().map(|v| v.to_bits()).collect();
        assert!(distinct.len() > 3, "a real frame should vary: {distinct:?}");
        assert!(grid.energy_bpp.iter().all(|v| v.is_finite() && *v >= 0.0));
    }

    #[test]
    fn a_real_inter_frame_gives_a_full_finite_grid() {
        let grid = extract_energy_grid_from_parsed(&clip_packet(1)).unwrap();
        assert_eq!(grid.energy_bpp.len(), (grid.grid_w * grid.grid_h) as usize);
        assert!(grid.energy_bpp.iter().all(|v| v.is_finite() && *v >= 0.0));
    }

    #[test]
    fn a_corrupted_key_frame_never_panics() {
        let (_hdr, frames) = crate::ivf::parse_ivf_frames(FIXTURE).unwrap();
        let data = &frames[0].data;
        for cut in (0..data.len()).step_by(53) {
            if let Ok(parsed) = ParsedFrame::parse(&data[..cut]) {
                let _ = extract_energy_grid_from_parsed(&parsed);
            }
        }
        for pos in (0..data.len()).step_by(97) {
            let mut flipped = data.clone();
            flipped[pos] ^= 0x20;
            if let Ok(parsed) = ParsedFrame::parse(&flipped) {
                let _ = extract_energy_grid_from_parsed(&parsed);
            }
        }
    }
}
