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
    let is_key_frame = parsed.frame_type.is_intra_only;
    let delta_q_enabled = parsed.delta_q_enabled;
    let reference_select = parsed.reference_select;
    let allow_intrabc = parsed.allow_intrabc;
    let tx_type_flags = crate::tile::TxTypeFrameFlags {
        coded_lossless: parsed.coded_lossless,
        qidx_is_zero: parsed.frame_type.base_qp == Some(0),
        reduced_tx_set: parsed.reduced_tx_set,
        txfm_mode: parsed.txfm_mode,
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
                    &mut tile_ctx,
                    tx_type_flags,
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
}
