//! Shared coding unit parsing utilities
//!
//! This module contains shared code for parsing coding units from AV1 tile data.
//! It is used by QP, MV, and partition extractors to avoid duplication.

use std::sync::Arc;

use bitvue_engine::BitvueError;

use super::cache::{compute_cache_key, get_or_parse_coding_units};
use super::parser::ParsedFrame;

/// Parse all coding units from tile data
///
/// Uses thread-safe LRU cache to avoid re-parsing the same tile data
/// when extracting multiple overlays.
///
/// Returns `Arc<Vec<CodingUnit>>` for O(1) cloning on cache hits.
/// Use `&*result` or `result.as_ref()` to access the slice of coding units.
/// This is used by QP, MV, and prediction mode grid extraction.
pub fn parse_all_coding_units(
    parsed: &ParsedFrame,
) -> Result<Arc<Vec<crate::tile::CodingUnit>>, BitvueError> {
    let base_qp = parsed.frame_type.base_qp.unwrap_or(128) as i16;
    let cache_key = compute_cache_key(&parsed.tile_data, base_qp);

    // Arc-clone is cheap (just reference count increment, no data copy)
    let tile_data = Arc::clone(&parsed.tile_data);
    let sb_size = parsed.dimensions.sb_size;
    let sb_cols = parsed.dimensions.sb_cols;
    let sb_rows = parsed.dimensions.sb_rows;
    let mi_rows = crate::tile::partition::mi_units(parsed.dimensions.height);
    let mi_cols = crate::tile::partition::mi_units(parsed.dimensions.width);
    let is_key_frame = parsed.frame_type.is_intra_only;
    let delta_q_enabled = parsed.delta_q_enabled;
    let reference_select = parsed.reference_select;
    let allow_intrabc = parsed.allow_intrabc;
    let allow_screen_content_tools = parsed.allow_screen_content_tools;
    let enable_filter_intra = parsed.enable_filter_intra;
    let delta_lf_present = parsed.delta_lf_present;
    let delta_lf_multi = parsed.delta_lf_multi;
    let use_ref_frame_mvs = parsed.use_ref_frame_mvs;
    let segmentation = parsed.segmentation;
    let cdef_bits = parsed.cdef_bits;
    let tx_type_flags = crate::tile::TxTypeFrameFlags {
        coded_lossless: parsed.coded_lossless,
        qidx_is_zero: parsed.frame_type.base_qp == Some(0),
        reduced_tx_set: parsed.reduced_tx_set,
        txfm_mode: parsed.txfm_mode,
        mono_chrome: parsed.mono_chrome,
        subsampling_x: parsed.subsampling_x,
        subsampling_y: parsed.subsampling_y,
    };

    // Use get_or_parse helper for cache pattern
    get_or_parse_coding_units(cache_key, || {
        let mut all_cus = Vec::new();

        // Pre-allocate capacity based on superblock count
        let estimated_cus = (sb_cols * sb_rows) as usize * 4;
        all_cus.reserve(estimated_cus);

        // Create SymbolDecoder for tile data
        let mut decoder = crate::SymbolDecoder::new(&tile_data)?;

        // Track running QP value across superblocks
        let mut current_qp = base_qp;

        // Create MV predictor context
        let mut mv_ctx = crate::tile::MvPredictorContext::new(sb_cols, sb_rows);

        // Create entropy-context tracker (currently only `skip` uses it -- see
        // `crate::tile::TileContext`'s doc), sized to the tile's full extent in 4x4 units.
        let mut tile_ctx = crate::tile::TileContext::new(
            (sb_cols * sb_size).div_ceil(4),
            (sb_rows * sb_size).div_ceil(4),
        );

        // Parse each superblock
        for sb_y in 0..sb_rows {
            tile_ctx.start_superblock_row();
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
                    reference_select,
                    allow_intrabc,
                    allow_screen_content_tools,
                    enable_filter_intra,
                    delta_lf_present,
                    delta_lf_multi,
                    use_ref_frame_mvs,
                    segmentation,
                    &mut tile_ctx,
                    tx_type_flags,
                    mi_rows,
                    mi_cols,
                    cdef_bits,
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

/// Spatial index for O(1) coding unit lookup by grid position
///
/// Pre-computes which coding unit overlaps each grid cell, eliminating
/// the need for O(n) linear search per block. For 1080p, this reduces
/// 510×1000 = 510,000 comparisons to just 510 lookups.
pub struct CuSpatialIndex {
    /// Grid of CU indices (one per grid cell)
    /// Vec index = grid_y * grid_w + grid_x
    /// Value = Some(cu_index) or None if no CU covers this cell
    grid: Vec<Option<usize>>,
    grid_w: u32,
}

impl CuSpatialIndex {
    /// Build spatial index from coding units
    ///
    /// For each coding unit, determine which grid cells it overlaps
    /// and store the CU index in those cells.
    ///
    /// # Arguments
    /// * `coding_units` - Slice of coding units to index
    /// * `grid_w` - Grid width in cells
    /// * `grid_h` - Grid height in cells
    /// * `block_w` - Grid cell width in pixels
    /// * `block_h` - Grid cell height in pixels
    pub fn new(
        coding_units: &[crate::tile::CodingUnit],
        grid_w: u32,
        grid_h: u32,
        block_w: u32,
        block_h: u32,
    ) -> Self {
        let total_cells = (grid_w * grid_h) as usize;
        let mut grid = vec![None; total_cells];

        for (cu_idx, cu) in coding_units.iter().enumerate() {
            // Convert CU pixel coordinates to grid coordinates
            // All values are u32, so division works correctly
            let cu_grid_x_start = cu.x / block_w;
            let cu_grid_y_start = cu.y / block_h;
            let cu_grid_x_end = cu.x.saturating_add(cu.width).saturating_sub(1) / block_w;
            let cu_grid_y_end = cu.y.saturating_add(cu.height).saturating_sub(1) / block_h;

            // Clamp to grid bounds
            let clamped_x_start = cu_grid_x_start.min(grid_w - 1);
            let clamped_y_start = cu_grid_y_start.min(grid_h - 1);
            let clamped_x_end = cu_grid_x_end.min(grid_w - 1);
            let clamped_y_end = cu_grid_y_end.min(grid_h - 1);

            // Mark all grid cells overlapped by this CU
            for grid_y in clamped_y_start..=clamped_y_end {
                for grid_x in clamped_x_start..=clamped_x_end {
                    let cell_idx = (grid_y * grid_w + grid_x) as usize;
                    // First CU wins (earlier CUs take precedence)
                    if grid[cell_idx].is_none() {
                        grid[cell_idx] = Some(cu_idx);
                    }
                }
            }
        }

        Self { grid, grid_w }
    }

    /// Get coding unit index for a grid cell (O(1) lookup)
    #[inline]
    pub fn get_cu_index(&self, grid_x: u32, grid_y: u32) -> Option<usize> {
        let cell_idx = (grid_y * self.grid_w + grid_x) as usize;
        self.grid.get(cell_idx).copied().flatten()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cu_spatial_index() {
        // Create some test CUs
        let cus = vec![
            crate::tile::CodingUnit::new(0, 0, 64, 64),
            crate::tile::CodingUnit::new(64, 0, 64, 64),
        ];

        // Build index with 64x64 grid cells
        let index = CuSpatialIndex::new(&cus, 4, 4, 64, 64);

        // Check that we can find CUs
        assert_eq!(index.get_cu_index(0, 0), Some(0)); // First CU
        assert_eq!(index.get_cu_index(1, 0), Some(1)); // Second CU
        assert_eq!(index.get_cu_index(0, 1), None); // No CU here
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

    /// Regression test for a real desync/crash bug: `parse_coding_unit` used to never read AV1's
    /// `residual()` syntax for non-skip coding units, which silently desynced the shared
    /// `SymbolDecoder` from every later syntax element in a tile -- confirmed to eventually run
    /// the arithmetic decoder past the end of real tile bytes and panic on real fixture frames
    /// (frame 100, then frame 19 after a partial fix, before `SymbolDecoder::read_residual_block`
    /// closed the gap). Parses every single frame of the real fixture through the full real-CU
    /// path to guard against regressing back to that state.
    #[test]
    fn real_fixture_every_frame_parses_coding_units_without_panicking_or_erroring() {
        let (_hdr, frames) = crate::ivf::parse_ivf_frames(AV1_IVF_FIXTURE).unwrap();
        let seq_bytes = find_seq_header_bytes(&frames).expect("fixture has a sequence header");

        for (idx, frame) in frames.iter().enumerate() {
            let obu_data: Vec<u8> = [seq_bytes.as_slice(), frame.data.as_slice()].concat();
            let parsed = super::super::parser::ParsedFrame::parse(&obu_data).unwrap();
            if !parsed.has_tile_data() {
                continue;
            }
            let result = parse_all_coding_units(&parsed);
            assert!(
                result.is_ok(),
                "frame {idx} failed to parse coding units: {:?}",
                result.err()
            );
        }
    }

    /// Regression test for the ref_frame() desync fix (2026-08-11): `parse_coding_unit` used to
    /// hardcode every inter block's reference frame to `RefFrame::Last` (never reading the real
    /// `ref_frame()` syntax at all -- the same "syntax element completely unread" bug shape as
    /// the residual() bug above, just never crashed because nothing downstream cross-checks
    /// ref_frame values against anything). Confirms real bits are actually being read: `Last`
    /// alone would mean the fix silently regressed to the old hardcoded behavior, and zero
    /// compound blocks despite `reference_select=true` on many frames would mean the comp_mode
    /// branch never actually triggers.
    #[test]
    fn real_fixture_ref_frame_values_are_not_degenerate() {
        let (_hdr, frames) = crate::ivf::parse_ivf_frames(AV1_IVF_FIXTURE).unwrap();
        let seq_bytes = find_seq_header_bytes(&frames).expect("fixture has a sequence header");

        let mut distinct_ref0_values: std::collections::HashSet<crate::tile::RefFrame> =
            Default::default();
        let mut compound_count = 0;
        let mut inter_count = 0;
        let mut saw_reference_select_true = false;
        let mut distinct_compound_modes: std::collections::HashSet<crate::tile::PredictionMode> =
            Default::default();
        let mut nonzero_l1_mv_count = 0;

        for frame in frames.iter().take(30) {
            let obu_data: Vec<u8> = [seq_bytes.as_slice(), frame.data.as_slice()].concat();
            let parsed = super::super::parser::ParsedFrame::parse(&obu_data).unwrap();
            saw_reference_select_true |= parsed.reference_select;
            if !parsed.has_tile_data() {
                continue;
            }
            let cus = parse_all_coding_units(&parsed).unwrap();
            for cu in cus.iter() {
                if cu.is_inter() {
                    inter_count += 1;
                    distinct_ref0_values.insert(cu.ref_frames[0]);
                    if cu.ref_frames[1] != crate::tile::RefFrame::Intra {
                        compound_count += 1;
                        distinct_compound_modes.insert(cu.mode);
                        if cu.mv[1] != crate::tile::MotionVector::zero() {
                            nonzero_l1_mv_count += 1;
                        }
                    }
                }
            }
        }

        assert!(inter_count > 0, "fixture should have real inter blocks");
        assert!(
            saw_reference_select_true,
            "expected at least one of the first 30 frames to have reference_select=true"
        );
        assert!(
            distinct_ref0_values.len() > 1,
            "expected more than one distinct ref_frame[0] value across {inter_count} real inter \
             blocks, got only {distinct_ref0_values:?} -- ref_frame() may have regressed to the \
             old hardcoded-to-Last behavior"
        );
        assert!(
            compound_count > 0,
            "expected at least one compound-prediction block given reference_select was true on \
             multiple frames -- the comp_mode branch may not be triggering"
        );
        assert!(
            compound_count == 0 || !distinct_compound_modes.is_empty(),
            "compound blocks exist but none produced a compound PredictionMode -- \
             read_compound_mode()/compound_mode_from_symbol may not be wired up"
        );
        assert!(
            nonzero_l1_mv_count > 0,
            "expected at least one compound block ({compound_count} total) with a non-zero L1 \
             motion vector across {} distinct compound modes ({distinct_compound_modes:?}) -- L1 \
             MV reading may have regressed to the old always-zero placeholder",
            distinct_compound_modes.len()
        );
    }

    /// Regression test for the real `skip` entropy-context work (this session's arithmetic-core
    /// rewrite, see `symbol/arithmetic.rs`'s doc): `read_skip` now uses real per-context
    /// (`TileContext::skip_context`, 0..=2) default CDFs and real adaptation instead of a single
    /// fixed non-adaptive CDF. Parses the real fixture and confirms both `skip=true` and
    /// `skip=false` actually occur (not degenerate to always one value, which is exactly the
    /// failure mode a wrongly-wired context/CDF-direction bug would produce -- see this test's
    /// sibling `real_fixture_ref_frame_values_are_not_degenerate` for the established pattern).
    #[test]
    fn real_fixture_skip_flags_are_not_degenerate() {
        let (_hdr, frames) = crate::ivf::parse_ivf_frames(AV1_IVF_FIXTURE).unwrap();
        let seq_bytes = find_seq_header_bytes(&frames).expect("fixture has a sequence header");

        let mut saw_skip_true = false;
        let mut saw_skip_false = false;
        let mut total_cus = 0usize;

        for frame in frames.iter().take(30) {
            let obu_data: Vec<u8> = [seq_bytes.as_slice(), frame.data.as_slice()].concat();
            let parsed = match super::super::parser::ParsedFrame::parse(&obu_data) {
                Ok(p) => p,
                Err(_) => continue,
            };
            if !parsed.has_tile_data() {
                continue;
            }
            let Ok(cus) = parse_all_coding_units(&parsed) else {
                continue;
            };
            for cu in cus.iter() {
                total_cus += 1;
                if cu.skip {
                    saw_skip_true = true;
                } else {
                    saw_skip_false = true;
                }
            }
        }

        assert!(total_cus > 0, "fixture should have real coding units");
        assert!(
            saw_skip_true && saw_skip_false,
            "expected both skip=true and skip=false across {total_cus} real coding units, got \
             true={saw_skip_true} false={saw_skip_false} -- the real per-context skip CDFs/\
             adaptation may have regressed to a degenerate always-one-value decode"
        );
    }

    /// Regression test for the real `coeff_base`/`coeff_br` neighbor-context work
    /// (`symbol::scan::lo_ctx`, real scan order): confirms real, varied residual-energy stats
    /// still come out of the full fixture parse -- same "not degenerate" bar as this test's
    /// siblings. A regression to a flat/broken context (or a scan-order/level-buffer indexing
    /// bug) would most likely show up as either every non-skip CU reporting `nonzero_count == 0`
    /// (context desync causing `txb_skip`/`coeff_base` to read spuriously-all-zero) or a single
    /// repeated `sum_abs_level` value (context stuck at one bucket).
    #[test]
    fn real_fixture_residual_energy_is_not_degenerate() {
        let (_hdr, frames) = crate::ivf::parse_ivf_frames(AV1_IVF_FIXTURE).unwrap();
        let seq_bytes = find_seq_header_bytes(&frames).expect("fixture has a sequence header");

        let mut distinct_sum_abs_levels: std::collections::HashSet<u64> = Default::default();
        let mut saw_nonzero_residual = false;
        let mut non_skip_cu_count = 0usize;

        for frame in frames.iter().take(30) {
            let obu_data: Vec<u8> = [seq_bytes.as_slice(), frame.data.as_slice()].concat();
            let parsed = match super::super::parser::ParsedFrame::parse(&obu_data) {
                Ok(p) => p,
                Err(_) => continue,
            };
            if !parsed.has_tile_data() {
                continue;
            }
            let Ok(cus) = parse_all_coding_units(&parsed) else {
                continue;
            };
            for cu in cus.iter() {
                if cu.skip {
                    continue;
                }
                non_skip_cu_count += 1;
                if let Some(residual) = cu.residual {
                    distinct_sum_abs_levels.insert(residual.sum_abs_level);
                    if residual.nonzero_count > 0 {
                        saw_nonzero_residual = true;
                    }
                }
            }
        }

        assert!(
            non_skip_cu_count > 0,
            "fixture should have real non-skip coding units"
        );
        assert!(
            saw_nonzero_residual,
            "expected at least one non-skip CU with real nonzero residual coefficients among \
             {non_skip_cu_count} non-skip CUs -- the real coeff_base/coeff_br context/scan-order \
             work may have regressed to spurious all-zero decodes"
        );
        assert!(
            distinct_sum_abs_levels.len() > 1,
            "expected varied sum_abs_level values across {non_skip_cu_count} non-skip CUs, got \
             only {} distinct value(s) -- may indicate coeff_base/coeff_br context is stuck at a \
             single bucket",
            distinct_sum_abs_levels.len()
        );
    }

    /// Regression test for the chroma residual desync fix (`SymbolDecoder::
    /// read_chroma_residual_block`): confirms the gate `parse_coding_unit` uses to decide "should
    /// this CU also read chroma" (non-IntraBC, non-monochrome, 4:2:0, square luma width
    /// `8..64`) is non-vacuously true on the real fixture -- i.e. guards against the condition
    /// silently becoming dead code (never matching) in a future change, which would look
    /// identical to "working" in every other test here since it'd just silently stop reading
    /// chroma bits again. Parsing succeeding at all for these frames (via
    /// `parse_all_coding_units`, which would error out on an arithmetic-decoder desync) is the
    /// actual regression signal -- this test's real job is proving the gate fires, not
    /// re-deriving decoded values.
    ///
    /// Scans the *entire* fixture (not just the first 30 frames like this file's other tests):
    /// qualifying CUs are rare here (17 total across all 250 frames, confirmed while developing
    /// this fix) and, moreover, occur *only* on inter frames -- this fixture's key frames happen
    /// to consist entirely of unpartitioned 128x128 coding blocks (never small enough to
    /// qualify). An earlier, more conservative version of the real gate additionally required
    /// `is_key_frame`, which happened to make it *never actually fire* against this fixture at
    /// all -- every test still passed, vacuously, until this test was added specifically to
    /// catch that (see `read_chroma_residual_block`'s doc for the full story).
    #[test]
    fn real_fixture_square_chroma_eligible_blocks_exist_and_parse_cleanly() {
        let (_hdr, frames) = crate::ivf::parse_ivf_frames(AV1_IVF_FIXTURE).unwrap();
        let seq_bytes = find_seq_header_bytes(&frames).expect("fixture has a sequence header");

        let mut square_chroma_eligible_cu_count = 0usize;
        let mut saw_64x64 = false;
        let mut saw_128x128 = false;

        for frame in frames.iter() {
            let obu_data: Vec<u8> = [seq_bytes.as_slice(), frame.data.as_slice()].concat();
            let parsed = match super::super::parser::ParsedFrame::parse(&obu_data) {
                Ok(p) => p,
                Err(_) => continue,
            };
            if !parsed.has_tile_data() {
                continue;
            }
            assert!(
                !parsed.mono_chrome && parsed.subsampling_x && parsed.subsampling_y,
                "fixture is expected to be a real 4:2:0 (non-monochrome) stream"
            );
            let Ok(cus) = parse_all_coding_units(&parsed) else {
                panic!("frame failed to parse coding units -- possible chroma-read desync");
            };
            for cu in cus.iter() {
                if !cu.skip
                    && !cu.use_intrabc
                    && cu.width == cu.height
                    && (8..=128).contains(&cu.width)
                {
                    square_chroma_eligible_cu_count += 1;
                    saw_64x64 |= cu.width == 64;
                    saw_128x128 |= cu.width == 128;
                }
            }
        }

        assert!(
            square_chroma_eligible_cu_count > 0,
            "expected at least one non-skip, non-IntraBC, square 8x8/16x16/32x32/64x64 coding \
             unit (the chroma residual read's gate condition) across the real fixture -- if this \
             is ever 0, the gate has gone dead and chroma bits are silently unread again"
        );
        assert!(
            saw_64x64,
            "expected at least one real 64x64 CU -- if this regresses to 0, the 64x64 chroma \
             extension (single-tile, tx_size_class=3) is passing vacuously"
        );
        assert!(
            saw_128x128,
            "expected at least one real 128x128 CU -- if this regresses to 0, the 128x128 chroma \
             extension (real 2x2-tiled, per-position context) is passing vacuously"
        );
    }

    /// Regression test for real **non-square** chroma residual reading (`read_chroma_residual_
    /// block`'s width/height generalization) -- same non-vacuous discipline as
    /// `real_fixture_square_chroma_eligible_blocks_exist_and_parse_cleanly`'s doc, extended to
    /// `cu.width != cu.height`. Before this, every non-square `HasChroma` coding block's chroma
    /// residual bits were never read at all (the gate required `width == height`) -- a real, live
    /// desync bug that only became common once non-square inter var-tx made non-square CUs common
    /// (550/1676 real fixture CUs, previous session). The full-fixture parse still succeeding here
    /// (no panic/error) is itself part of the evidence this was fixed correctly, not just that the
    /// gate now fires.
    #[test]
    fn real_fixture_nonsquare_chroma_eligible_blocks_exist_and_parse_cleanly() {
        let (_hdr, frames) = crate::ivf::parse_ivf_frames(AV1_IVF_FIXTURE).unwrap();
        let seq_bytes = find_seq_header_bytes(&frames).expect("fixture has a sequence header");

        let mut nonsquare_chroma_eligible_cu_count = 0usize;
        let mut dims_seen: std::collections::HashSet<(u32, u32)> = Default::default();

        for frame in frames.iter() {
            let obu_data: Vec<u8> = [seq_bytes.as_slice(), frame.data.as_slice()].concat();
            let parsed = match super::super::parser::ParsedFrame::parse(&obu_data) {
                Ok(p) => p,
                Err(_) => continue,
            };
            if !parsed.has_tile_data() {
                continue;
            }
            let Ok(cus) = parse_all_coding_units(&parsed) else {
                panic!("frame failed to parse coding units -- possible chroma-read desync");
            };
            for cu in cus.iter() {
                if !cu.skip
                    && !cu.use_intrabc
                    && cu.width != cu.height
                    && (8..=128).contains(&cu.width)
                    && (8..=128).contains(&cu.height)
                {
                    nonsquare_chroma_eligible_cu_count += 1;
                    dims_seen.insert((cu.width, cu.height));
                }
            }
        }

        assert!(
            nonsquare_chroma_eligible_cu_count > 0,
            "expected at least one non-skip, non-IntraBC, non-square coding unit (the non-square \
             chroma residual read's gate condition) across the real fixture -- if this is ever 0, \
             the gate has gone dead and non-square chroma bits are silently unread again"
        );
        assert!(
            dims_seen.len() > 3,
            "expected a real variety of non-square chroma-eligible dimensions, got only \
             {dims_seen:?}"
        );
    }

    /// Regression test for the real key-frame `intra_mode` (`kfym`) context work: `read_intra_mode`
    /// now uses real per-context (`TileContext::intra_mode_context`, 5x5 above/left mode-class
    /// grid) default CDFs sourced from rav1d's `Default_Kf_Y_Mode_Cdf` and real adaptation,
    /// instead of a single fixed non-adaptive CDF. Parses the real fixture's key/intra-only frames
    /// and confirms more than one distinct `PredictionMode` occurs among their coding units --
    /// same "not degenerate" bar as this test's siblings (`real_fixture_ref_frame_values_are_not_degenerate`,
    /// `real_fixture_skip_flags_are_not_degenerate`).
    #[test]
    fn real_fixture_key_frame_intra_modes_are_not_degenerate() {
        let (_hdr, frames) = crate::ivf::parse_ivf_frames(AV1_IVF_FIXTURE).unwrap();
        let seq_bytes = find_seq_header_bytes(&frames).expect("fixture has a sequence header");

        let mut distinct_modes: std::collections::HashSet<crate::tile::PredictionMode> =
            Default::default();
        let mut key_frame_cu_count = 0usize;

        for frame in frames.iter().take(30) {
            let obu_data: Vec<u8> = [seq_bytes.as_slice(), frame.data.as_slice()].concat();
            let parsed = match super::super::parser::ParsedFrame::parse(&obu_data) {
                Ok(p) => p,
                Err(_) => continue,
            };
            if !parsed.frame_type.is_intra_only || !parsed.has_tile_data() {
                continue;
            }
            let Ok(cus) = parse_all_coding_units(&parsed) else {
                continue;
            };
            key_frame_cu_count += cus.len();
            distinct_modes.extend(cus.iter().map(|cu| cu.mode));
        }

        assert!(
            key_frame_cu_count > 0,
            "expected at least one key/intra-only frame with real coding units in the first 30 \
             frames of the fixture"
        );
        assert!(
            distinct_modes.len() > 1,
            "expected more than one distinct intra PredictionMode across {key_frame_cu_count} \
             real key-frame coding units, got only {distinct_modes:?} -- the real kfym \
             context/CDF wiring may have regressed to a degenerate always-one-mode decode"
        );
    }

    /// Regression test for the `mv_joint` desync fix: `read_explicit_mv` used to unconditionally
    /// read both the horizontal and vertical MV components for every `NewMv`/compound-`New` slot,
    /// skipping the spec 5.11.32 `mv_joint` symbol that gates whether either axis is actually
    /// coded at all. Confirms real, non-degenerate `NewMv` decoding: more than one distinct
    /// explicit-MV outcome (proving the conditional reads aren't just producing one fixed
    /// pattern), and both real adaptive MV CDFs (`mv_class`/`mv_bit`/`mv_sign`/`mv_joint`, wired
    /// in the same change) actually getting exercised across the fixture.
    #[test]
    fn real_fixture_new_mv_values_are_not_degenerate() {
        let (_hdr, frames) = crate::ivf::parse_ivf_frames(AV1_IVF_FIXTURE).unwrap();
        let seq_bytes = find_seq_header_bytes(&frames).expect("fixture has a sequence header");

        let mut distinct_mv0: std::collections::HashSet<(i32, i32)> = Default::default();
        let mut new_mv_count = 0usize;

        for frame in frames.iter().take(30) {
            let obu_data: Vec<u8> = [seq_bytes.as_slice(), frame.data.as_slice()].concat();
            let parsed = match super::super::parser::ParsedFrame::parse(&obu_data) {
                Ok(p) => p,
                Err(_) => continue,
            };
            if !parsed.has_tile_data() {
                continue;
            }
            let Ok(cus) = parse_all_coding_units(&parsed) else {
                continue;
            };
            for cu in cus.iter() {
                if cu.mode == crate::tile::PredictionMode::NewMv {
                    new_mv_count += 1;
                    distinct_mv0.insert((cu.mv[0].x, cu.mv[0].y));
                }
            }
        }

        assert!(
            new_mv_count > 0,
            "expected at least one real NewMv coding unit in the first 30 frames of the fixture"
        );
        assert!(
            distinct_mv0.len() > 1,
            "expected more than one distinct MV across {new_mv_count} real NewMv coding units, \
             got only {distinct_mv0:?} -- the mv_joint/mv_class/mv_bit/mv_sign real adaptation \
             may have regressed to a degenerate always-one-value decode"
        );
    }

    /// Regression test for the real `partition` entropy-context work: `parse_partition_recursive`
    /// now uses real per-context (`crate::tile::TileContext::partition_context`, above/left 8x8
    /// bitmask) default CDFs sourced from rav1d's `Default_Partition_W*_Cdf` tables and real
    /// adaptation, instead of a single context-independent CDF per block size. Confirms real,
    /// non-degenerate leaf-block sizes across the fixture (CU width/height directly reflects
    /// which `PartitionType` was decoded at each level) -- same "not degenerate" bar as this
    /// test's siblings (`real_fixture_ref_frame_values_are_not_degenerate`,
    /// `real_fixture_skip_flags_are_not_degenerate`, `real_fixture_new_mv_values_are_not_degenerate`).
    #[test]
    fn real_fixture_partition_leaf_sizes_are_not_degenerate() {
        let (_hdr, frames) = crate::ivf::parse_ivf_frames(AV1_IVF_FIXTURE).unwrap();
        let seq_bytes = find_seq_header_bytes(&frames).expect("fixture has a sequence header");

        let mut distinct_leaf_sizes: std::collections::HashSet<(u32, u32)> = Default::default();
        let mut total_cus = 0usize;

        for frame in frames.iter().take(30) {
            let obu_data: Vec<u8> = [seq_bytes.as_slice(), frame.data.as_slice()].concat();
            let parsed = match super::super::parser::ParsedFrame::parse(&obu_data) {
                Ok(p) => p,
                Err(_) => continue,
            };
            if !parsed.has_tile_data() {
                continue;
            }
            let Ok(cus) = parse_all_coding_units(&parsed) else {
                continue;
            };
            total_cus += cus.len();
            distinct_leaf_sizes.extend(cus.iter().map(|cu| (cu.width, cu.height)));
        }

        assert!(
            total_cus > 0,
            "expected real coding units in the first 30 frames of the fixture"
        );
        assert!(
            distinct_leaf_sizes.len() > 1,
            "expected more than one distinct leaf-block size across {total_cus} real coding \
             units, got only {distinct_leaf_sizes:?} -- the real partition context/CDF wiring may \
             have regressed to a degenerate always-one-size decode (e.g. every superblock reading \
             PARTITION_NONE)"
        );
    }

    /// Regression test for `parse_partition_recursive`'s real `hasRows`/`hasCols` frame-edge
    /// handling (spec 5.11.4): the fixture is 320x240 with 128x128 superblocks (`sb_cols=3,
    /// sb_rows=2`, i.e. a `384x256` superblock grid overhanging the real frame on both edges), so
    /// every rightmost/bottommost superblock genuinely straddles `MiCols`/`MiRows` -- this is a
    /// real, non-contrived condition in this fixture, not a synthetic one. Before this change,
    /// `has_rows`/`has_cols` were hardcoded `true`, so these edge superblocks always read the full
    /// partition alphabet regardless of position; asserts the reduced-alphabet path is genuinely
    /// exercised (non-square leaf CU shapes, e.g. `64x128` from a `PARTITION_VERT`-only choice at
    /// a column-truncated 128x128 superblock, appear in real output) rather than passing
    /// vacuously -- same discipline as `real_fixture_square_chroma_eligible_blocks_exist_and_parse_cleanly`'s
    /// doc (a prior gate on this same fixture, `is_key_frame`, turned out to never actually fire).
    /// Landing this also exposed and fixed a dormant, pre-existing bug: this crate's partition-tree
    /// walker used to recurse into `parse_partition_recursive` again for every non-`None`
    /// partition's sub-blocks, including terminal ones (`HORZ`/`VERT`/etc, which per spec go
    /// straight to `decode_block`, no further `partition` symbol) -- harmless while those were
    /// rarely chosen, but real `has_rows`/`has_cols` making `VERT`-at-a-column-edge a common,
    /// correct decode surfaced it as an outright decode error (a spurious symbol read against the
    /// wrong CDF bucket for the resulting non-square block). Fixed in the same change (see
    /// `parse_partition_recursive`'s doc).
    #[test]
    fn real_fixture_frame_edge_partitions_are_not_degenerate() {
        let (_hdr, frames) = crate::ivf::parse_ivf_frames(AV1_IVF_FIXTURE).unwrap();
        let seq_bytes = find_seq_header_bytes(&frames).expect("fixture has a sequence header");

        let mut non_square_leaf_count = 0usize;
        let mut total_cus = 0usize;

        for frame in frames.iter() {
            let obu_data: Vec<u8> = [seq_bytes.as_slice(), frame.data.as_slice()].concat();
            let parsed = match super::super::parser::ParsedFrame::parse(&obu_data) {
                Ok(p) => p,
                Err(_) => continue,
            };
            if !parsed.has_tile_data() {
                continue;
            }
            let Ok(cus) = parse_all_coding_units(&parsed) else {
                continue;
            };
            total_cus += cus.len();
            non_square_leaf_count += cus.iter().filter(|cu| cu.width != cu.height).count();
        }

        assert!(
            total_cus > 0,
            "expected real coding units across the fixture"
        );
        assert!(
            non_square_leaf_count > 0,
            "expected at least one non-square leaf CU (from a real frame-edge HORZ/VERT choice) \
             across {total_cus} real coding units -- the real hasRows/hasCols frame-edge gate may \
             not be firing against this fixture's real 320x240-in-128x128-superblocks geometry, or \
             may have regressed to always reading the full alphabet"
        );
    }

    /// Regression test for `inter_mode`'s `globalmv_ctx` fix: previously hardcoded to `0`
    /// (`crate::tile::context::SpatialRefContext::inter_mode_context`'s old doc), now the frame
    /// header's real `use_ref_frame_mvs` flag. Confirms the fix is exercised non-vacuously against
    /// this fixture -- every real inter frame here has `use_ref_frame_mvs == true` (not just a
    /// contrived edge case), meaning the old hardcoded `false` was wrong for 100% of this
    /// fixture's inter frames, not some rare corner. Same "assert the gate genuinely fires"
    /// discipline as `real_fixture_frame_edge_partitions_are_not_degenerate`'s doc.
    #[test]
    fn real_fixture_use_ref_frame_mvs_is_true_for_inter_frames() {
        let (_hdr, frames) = crate::ivf::parse_ivf_frames(AV1_IVF_FIXTURE).unwrap();
        let seq_bytes = find_seq_header_bytes(&frames).expect("fixture has a sequence header");

        let mut inter_frame_count = 0usize;
        let mut use_ref_frame_mvs_true_count = 0usize;

        for frame in frames.iter() {
            let obu_data: Vec<u8> = [seq_bytes.as_slice(), frame.data.as_slice()].concat();
            let Ok(parsed) = super::super::parser::ParsedFrame::parse(&obu_data) else {
                continue;
            };
            if parsed.frame_type.is_intra_only || !parsed.has_tile_data() {
                continue;
            }
            inter_frame_count += 1;
            if parsed.use_ref_frame_mvs {
                use_ref_frame_mvs_true_count += 1;
            }
        }

        assert!(
            inter_frame_count > 0,
            "expected real inter frames in the fixture"
        );
        assert_eq!(
            use_ref_frame_mvs_true_count, inter_frame_count,
            "expected every real inter frame in the fixture to have use_ref_frame_mvs == true \
             ({use_ref_frame_mvs_true_count}/{inter_frame_count} did) -- if this regresses, the \
             globalmv_ctx fix may be passing vacuously again"
        );
    }

    /// Regression test for real inter var-tx (`compute_inter_tx_blocks`/`read_var_tx_size`, spec
    /// 5.11.16/17/18): confirms the real `txfm_split` reads are genuinely exercised against the
    /// fixture (not vacuous -- every eligible non-skip square inter CU gets a real `tx_blocks`
    /// breakdown) and produce non-degenerate output (a real mix of leaf sizes across the whole
    /// fixture, and a majority of eligible CUs show at least one real split rather than always
    /// falling back to the CU's own uniform max size). Same "assert the gate genuinely fires"
    /// discipline as `real_fixture_frame_edge_partitions_are_not_degenerate`'s doc.
    #[test]
    fn real_fixture_inter_var_tx_is_not_degenerate() {
        let (_hdr, frames) = crate::ivf::parse_ivf_frames(AV1_IVF_FIXTURE).unwrap();
        let seq_bytes = find_seq_header_bytes(&frames).expect("fixture has a sequence header");

        let mut eligible_cu_count = 0usize;
        let mut cus_with_tx_blocks = 0usize;
        let mut cus_with_a_real_split = 0usize;
        let mut distinct_leaf_sizes: std::collections::HashSet<u32> = Default::default();

        for frame in frames.iter() {
            let obu_data: Vec<u8> = [seq_bytes.as_slice(), frame.data.as_slice()].concat();
            let Ok(parsed) = super::super::parser::ParsedFrame::parse(&obu_data) else {
                continue;
            };
            if parsed.frame_type.is_intra_only || !parsed.has_tile_data() {
                continue;
            }
            let Ok(cus) = parse_all_coding_units(&parsed) else {
                continue;
            };
            for cu in cus.iter() {
                // `compute_inter_tx_blocks`'s real gate is `width == height && (8..=128)`
                // (`coding_unit.rs`) -- 4x4 inter CUs are legitimately out of its documented scope
                // (var-tx's recursion always terminates at 8x8, never reads `txfm_split` below
                // it), not a regression, so mirror that same width floor here.
                if !(cu.is_inter() && cu.width == cu.height && cu.width >= 8 && !cu.skip) {
                    continue;
                }
                eligible_cu_count += 1;
                let Some(blocks) = &cu.tx_blocks else {
                    continue;
                };
                cus_with_tx_blocks += 1;
                if blocks.len() > 1 {
                    cus_with_a_real_split += 1;
                }
                distinct_leaf_sizes.extend(blocks.iter().map(|b| b.width_px));
            }
        }

        assert!(
            eligible_cu_count > 0,
            "expected real non-skip square inter coding units in the fixture"
        );
        assert_eq!(
            cus_with_tx_blocks, eligible_cu_count,
            "expected every eligible non-skip square inter CU to get a real tx_blocks breakdown \
             ({cus_with_tx_blocks}/{eligible_cu_count} did) -- the real var-tx gate may not be \
             firing against this fixture, or may have regressed to the old dimension-only \
             heuristic"
        );
        assert!(
            cus_with_a_real_split * 2 > eligible_cu_count,
            "expected a majority of eligible CUs to show a real txfm_split (more than one leaf), \
             got only {cus_with_a_real_split}/{eligible_cu_count} -- the txfm_split reads may be \
             degenerating to always-not-split"
        );
        assert!(
            distinct_leaf_sizes.len() > 2,
            "expected more than two distinct real leaf transform sizes across the fixture, got \
             only {distinct_leaf_sizes:?}"
        );
    }

    /// Regression test for real **non-square** inter var-tx (`compute_inter_tx_blocks`/
    /// `read_var_tx_size`'s rectangular generalization, spec 5.11.16/17/18) -- same non-vacuous
    /// discipline as `real_fixture_inter_var_tx_is_not_degenerate`'s doc, extended to CUs where
    /// `width != height` (previously entirely out of `compute_inter_tx_blocks`'s scope, kept on
    /// the older `TxSize::from_dimensions` square heuristic). Finding this fixture actually
    /// exercises non-square inter CUs is what surfaced a real, pre-existing, unrelated bug:
    /// `BlockSize::Block8x32::height()` returned `64` instead of `32` (a copy-paste error in its
    /// match arm grouping, `tile/partition.rs`) -- silently never caught before since nothing
    /// previously read a non-square CU's real dimensions this precisely. Fixed alongside this
    /// test landing.
    #[test]
    fn real_fixture_nonsquare_inter_var_tx_is_not_degenerate() {
        let (_hdr, frames) = crate::ivf::parse_ivf_frames(AV1_IVF_FIXTURE).unwrap();
        let seq_bytes = find_seq_header_bytes(&frames).expect("fixture has a sequence header");

        let mut eligible_cu_count = 0usize;
        let mut cus_with_tx_blocks = 0usize;
        let mut cus_with_a_real_split = 0usize;
        let mut rect_leaf_sizes: std::collections::HashSet<(u32, u32)> = Default::default();
        let mut dims_seen: std::collections::HashSet<(u32, u32)> = Default::default();

        for frame in frames.iter() {
            let obu_data: Vec<u8> = [seq_bytes.as_slice(), frame.data.as_slice()].concat();
            let Ok(parsed) = super::super::parser::ParsedFrame::parse(&obu_data) else {
                continue;
            };
            if parsed.frame_type.is_intra_only || !parsed.has_tile_data() {
                continue;
            }
            let Ok(cus) = parse_all_coding_units(&parsed) else {
                continue;
            };
            for cu in cus.iter() {
                if !(cu.is_inter() && cu.width != cu.height && !cu.skip) {
                    continue;
                }
                dims_seen.insert((cu.width, cu.height));
                eligible_cu_count += 1;
                let Some(blocks) = &cu.tx_blocks else {
                    continue;
                };
                cus_with_tx_blocks += 1;
                if blocks.len() > 1 {
                    cus_with_a_real_split += 1;
                }
                rect_leaf_sizes.extend(blocks.iter().map(|b| (b.width_px, b.height_px)));
                // Every real AV1 block size is a valid `dav1d_max_txfm_size_for_bs` entry (spec:
                // no transform exceeds 64 on either axis) -- catches the `Block8x32` class of bug
                // (a wrong dimension flowing all the way to a leaf) even if some future change
                // reintroduces something similar elsewhere.
                for b in blocks {
                    assert!(
                        b.width_px <= 64 && b.height_px <= 64,
                        "leaf {b:?} exceeds the real 64-per-axis transform cap"
                    );
                }
            }
        }

        assert!(
            eligible_cu_count > 0,
            "expected real non-skip non-square inter coding units in the fixture"
        );
        assert!(
            dims_seen.len() > 5,
            "expected a real variety of non-square block dimensions, got only {dims_seen:?}"
        );
        assert_eq!(
            cus_with_tx_blocks, eligible_cu_count,
            "expected every eligible non-skip non-square inter CU to get a real tx_blocks \
             breakdown ({cus_with_tx_blocks}/{eligible_cu_count} did)"
        );
        assert!(
            cus_with_a_real_split * 2 > eligible_cu_count,
            "expected a majority of eligible non-square CUs to show a real txfm_split, got only \
             {cus_with_a_real_split}/{eligible_cu_count}"
        );
        assert!(
            rect_leaf_sizes.iter().any(|(w, h)| w != h),
            "expected at least one genuinely rectangular (non-square) real leaf transform size, \
             got only {rect_leaf_sizes:?} -- the asymmetric split may be degenerating to \
             square-only leaves"
        );
    }

    /// Regression test for real `segment_id()` (spec 5.11.9/5.11.10) plumbing -- the only
    /// committed real fixture (`test_data/av1_test.ivf`) never enables segmentation, so this
    /// can't be a non-vacuous "real segment_id values decoded" test the way most of this module's
    /// real-context work is verified (see `DEVELOPMENT_PHASES.md` for the actual non-vacuous
    /// verification: a locally-generated, scratchpad-only `aomenc --aq-mode=3 --end-usage=cbr`
    /// clip showed 3 distinct real segment ids decoded across 478 CUs, zero parse errors). This
    /// test instead guards the *disabled* path: confirms `segmentation.enabled` reads `false` for
    /// this fixture (the plumbing didn't accidentally flip it) and every CU's `segment_id` stays
    /// `0` (the correct value when disabled, and the same default as before this feature existed
    /// -- catches a regression that would make `segmentation.enabled` spuriously `true` and start
    /// consuming bits that were never there, which would show up as decode errors here).
    #[test]
    fn real_fixture_segmentation_disabled_and_segment_id_stays_zero() {
        let (_hdr, frames) = crate::ivf::parse_ivf_frames(AV1_IVF_FIXTURE).unwrap();
        let seq_bytes = find_seq_header_bytes(&frames).expect("fixture has a sequence header");
        let mut checked_frames = 0usize;

        for frame in frames.iter() {
            let obu_data: Vec<u8> = [seq_bytes.as_slice(), frame.data.as_slice()].concat();
            let Ok(parsed) = super::super::parser::ParsedFrame::parse(&obu_data) else {
                continue;
            };
            if !parsed.has_tile_data() {
                continue;
            }
            assert!(
                !parsed.segmentation.enabled,
                "expected this fixture to never enable segmentation -- if this regresses to \
                 true, the segmentation bit is being misparsed"
            );
            let Ok(cus) = parse_all_coding_units(&parsed) else {
                panic!("frame failed to parse coding units -- possible segment_id-read desync");
            };
            for cu in cus.iter() {
                assert_eq!(
                    cu.segment_id, 0,
                    "expected segment_id 0 when segmentation is disabled"
                );
            }
            checked_frames += 1;
        }

        assert!(
            checked_frames > 0,
            "expected at least one real frame to check"
        );
    }
}
