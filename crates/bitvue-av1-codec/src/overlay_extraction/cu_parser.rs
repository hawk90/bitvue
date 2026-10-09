//! Shared coding unit parsing utilities
//!
//! This module contains shared code for parsing coding units from AV1 tile data.
//! It is used by QP, MV, and partition extractors to avoid duplication.

use std::sync::Arc;

use bitvue_engine::BitvueError;

use super::cache::{compute_frame_cache_key, get_or_parse_coding_units, ParsedCodingUnits};
use super::parser::ParsedFrame;
use super::provenance::DecodeOutcome;

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
    Ok(parse_coding_units_checked(parsed)?.units)
}

/// [`parse_all_coding_units`] plus how the decode ended ([`DecodeOutcome`]), from the same cache.
pub fn parse_coding_units_checked(parsed: &ParsedFrame) -> Result<ParsedCodingUnits, BitvueError> {
    if let Some(decoded) = &parsed.decoded {
        return Ok(decoded.clone());
    }
    let base_qp = parsed.frame_type.base_qp.unwrap_or(128) as i16;
    // Everything besides the tile bytes that the parse reads (see `compute_frame_cache_key`).
    let context = format!(
        "{:?}|{}|{}|{}",
        parsed.coding_params(),
        parsed.dimensions.sb_size,
        parsed.dimensions.sb_cols,
        parsed.dimensions.sb_rows
    );
    let cache_key = compute_frame_cache_key(&parsed.tile_data, base_qp, &context);
    get_or_parse_coding_units(cache_key, || parse_coding_units_with_outcome(parsed, None))
}

/// Uncached parse returning only the coding units, for tests that feed real temporal candidates.
#[cfg(test)]
fn parse_all_coding_units_with_temporal(
    parsed: &ParsedFrame,
    temporal: Option<(&crate::tile::ProjectedMotionField, [i32; 7])>,
) -> Result<Vec<crate::tile::CodingUnit>, BitvueError> {
    parse_coding_units_with_outcome(parsed, temporal).map(|(units, _)| units)
}

/// The entropy-context tracker for `parsed`'s tile: sized to the tile's full extent in 4x4 units
/// and told the frame's real size and its references' sign biases.
fn tile_context_for(parsed: &ParsedFrame) -> crate::tile::TileContext {
    let dims = &parsed.dimensions;
    let mut tile_ctx = crate::tile::TileContext::new(
        (dims.sb_cols * dims.sb_size).div_ceil(4),
        (dims.sb_rows * dims.sb_size).div_ceil(4),
    );
    tile_ctx.set_frame_extent(dims.width, dims.height);
    tile_ctx.set_sign_bias(parsed.ref_frame_sign_bias);
    tile_ctx.set_ref_order_distance(parsed.ref_order_distance);
    tile_ctx
}

/// What a frame's decode starts from beyond its own bytes: state carried over from earlier frames.
#[derive(Default)]
pub(crate) struct FrameDecodeInputs<'a> {
    /// Real temporal MV candidates (spec 7.9/7.10, [`crate::tile::motion_field`]) and the frame's
    /// `pocdiff`s.
    pub temporal: Option<(&'a crate::tile::ProjectedMotionField, [i32; 7])>,
    /// The CDFs the tile starts from (the primary reference frame's saved CDFs). `None`: the
    /// defaults for the frame's quantizer.
    pub initial_cdf: Option<crate::symbol::CdfContext>,
}

/// One decoded frame.
pub(crate) struct FrameDecode {
    pub units: Vec<crate::tile::CodingUnit>,
    pub outcome: DecodeOutcome,
    /// The tile's CDFs when decoding stopped (spec: what the frame saves when it updates its CDFs
    /// at the end).
    pub final_cdf: crate::symbol::CdfContext,
    /// `(rng, cnt, dif)` after every symbol, for comparison with an instrumented dav1d.
    #[cfg(test)]
    pub trace: Vec<(u32, i32, usize)>,
}

/// The parse behind [`parse_coding_units_checked`]: no state from earlier frames. The cache key
/// has no room for the inputs of [`decode_frame`], so only this stateless parse goes through it.
fn parse_coding_units_with_outcome(
    parsed: &ParsedFrame,
    temporal: Option<(&crate::tile::ProjectedMotionField, [i32; 7])>,
) -> Result<(Vec<crate::tile::CodingUnit>, DecodeOutcome), BitvueError> {
    let decode = decode_frame(
        parsed,
        FrameDecodeInputs {
            temporal,
            initial_cdf: None,
        },
    )?;
    Ok((decode.units, decode.outcome))
}

/// Decodes `parsed`'s tile from `inputs`. The first superblock that fails ends the frame: the
/// decoder has lost its place, so whatever it read after that would be noise presented as coding
/// units.
pub(crate) fn decode_frame(
    parsed: &ParsedFrame,
    inputs: FrameDecodeInputs<'_>,
) -> Result<FrameDecode, BitvueError> {
    let base_qp = parsed.frame_type.base_qp.unwrap_or(128) as i16;
    let tile_data = Arc::clone(&parsed.tile_data);
    let sb_size = parsed.dimensions.sb_size;
    let sb_cols = parsed.dimensions.sb_cols;
    let sb_rows = parsed.dimensions.sb_rows;
    let frame_params = parsed.coding_params();

    let mut all_cus = Vec::new();

    // Pre-allocate capacity based on superblock count
    let estimated_cus = (sb_cols * sb_rows) as usize * 4;
    all_cus.reserve(estimated_cus);

    // The tile starts from the saved CDFs of the primary reference frame, or from the defaults
    // seeded with the real per-frame qindex-bucket (`qcat`) residual-coefficient CDFs -- see
    // `crate::symbol::cdf::CdfContext::new_with_qcat`'s doc for the dav1d selection formula.
    let qcat = (base_qp > 20) as u8 + (base_qp > 60) as u8 + (base_qp > 120) as u8;
    let decoder = match inputs.initial_cdf {
        Some(cdf) => crate::SymbolDecoder::with_cdf_context(&tile_data, cdf)?,
        None => crate::SymbolDecoder::new_with_qcat(&tile_data, qcat)?,
    };

    // Track running QP value across superblocks
    let mut current_qp = base_qp;

    let mut tile_ctx = tile_context_for(parsed);
    if let Some((projected, pocdiff)) = inputs.temporal {
        tile_ctx.set_temporal_context(projected.clone(), pocdiff);
        tile_ctx.set_mv_precision(parsed.allow_high_precision_mv, parsed.force_integer_mv);
    }

    let mut state = crate::tile::TileState {
        decoder,
        mv_ctx: crate::tile::MvPredictorContext::new(sb_cols, sb_rows),
        tile_ctx,
    };
    #[cfg(test)]
    {
        state.decoder.decoder.range_trace = Some(Vec::new());
    }

    let superblocks_total = sb_cols * sb_rows;
    let mut superblocks_decoded = 0;
    'frame: for sb_y in 0..sb_rows {
        state.tile_ctx.start_superblock_row();
        for sb_x in 0..sb_cols {
            let sb_pixel_x = sb_x * sb_size;
            let sb_pixel_y = sb_y * sb_size;

            match crate::parse_superblock(
                &mut state,
                sb_pixel_x,
                sb_pixel_y,
                sb_size,
                &frame_params,
                current_qp,
            ) {
                Ok((sb, new_qp)) => {
                    all_cus.extend(sb.coding_units);
                    current_qp = new_qp;
                    superblocks_decoded += 1;
                }
                Err(e) => {
                    tracing::debug!(
                        "Failed to parse superblock ({}, {}): {}, stopping",
                        sb_pixel_x,
                        sb_pixel_y,
                        e
                    );
                    break 'frame;
                }
            }
        }
    }

    let outcome = DecodeOutcome {
        superblocks_total,
        superblocks_decoded,
        padding_conformant: state.decoder.padding_is_conformant(),
    };
    tracing::debug!(
        "Parsed {} coding units from tile data (final QP: {}, {:?})",
        all_cus.len(),
        current_qp,
        outcome
    );
    #[cfg(test)]
    let trace = state.decoder.decoder.range_trace.take().unwrap_or_default();
    Ok(FrameDecode {
        units: all_cus,
        outcome,
        final_cdf: state.decoder.cdf_context,
        #[cfg(test)]
        trace,
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

    /// Regression test: the coding-unit cache used to key on `(tile_data, base_qp)` only, so two
    /// frames with identical tile bytes but different header flags got each other's cached result
    /// (which one depended on cache eviction order, i.e. on the run). Builds such pairs from real
    /// fixture frames -- same bytes, one header flag flipped -- keeps those whose fresh parses
    /// actually differ, and requires the cached lookup of each to equal its own fresh parse.
    #[test]
    fn frames_with_identical_tile_bytes_but_different_headers_do_not_share_a_cache_entry() {
        let (_hdr, frames) = crate::ivf::parse_ivf_frames(AV1_IVF_FIXTURE).unwrap();
        let seq_bytes = find_seq_header_bytes(&frames).expect("fixture has a sequence header");
        let render = |p: &super::super::parser::ParsedFrame| {
            format!(
                "{:?}",
                parse_all_coding_units_with_temporal(p, None).unwrap()
            )
        };

        let mut checked = 0;
        for frame in frames.iter().take(12) {
            let obu_data: Vec<u8> = [seq_bytes.as_slice(), frame.data.as_slice()].concat();
            let original = super::super::parser::ParsedFrame::parse(&obu_data).unwrap();
            if !original.has_tile_data() {
                continue;
            }
            type Flip = fn(&mut super::super::parser::ParsedFrame);
            let flips: [Flip; 4] = [
                |p| p.reference_select = !p.reference_select,
                |p| p.delta_q_enabled = !p.delta_q_enabled,
                |p| p.allow_screen_content_tools = !p.allow_screen_content_tools,
                |p| p.enable_filter_intra = !p.enable_filter_intra,
            ];
            for flip in flips {
                let mut variant = original.clone();
                flip(&mut variant);
                let (fresh_a, fresh_b) = (render(&original), render(&variant));
                if fresh_a == fresh_b {
                    continue; // this flag does not change what this frame parses to
                }
                // Same tile bytes and QP, different parses: the cache must tell them apart, in
                // either request order.
                for pair in [[&original, &variant], [&variant, &original]] {
                    for p in pair {
                        let want = if std::ptr::eq(p, &original) {
                            &fresh_a
                        } else {
                            &fresh_b
                        };
                        let got = format!("{:?}", parse_all_coding_units(p).unwrap());
                        assert_eq!(&got, want, "cached parse differs from a fresh parse");
                    }
                }
                checked += 1;
            }
        }
        assert!(
            checked > 0,
            "no frame/flag pair parsed differently; test is vacuous"
        );
    }

    /// Ground truth from a dav1d 1.5.1 build with `DEBUG_BLOCK_INFO` enabled (single-threaded,
    /// decoding `test_data/av1_test.ivf`): the arithmetic decoder's `rng` right after each
    /// `partition` symbol of the first 128x128 superblock of frame 0 (`poc=0`), and right after
    /// the *first block's* `skip` symbol.
    ///
    /// ```text
    /// poc=0,y=0,x=0,bl=0,ctx=0,bp=3: r=63552
    /// poc=0,y=0,x=0,bl=1,ctx=0,bp=3: r=50112
    /// poc=0,y=0,x=0,bl=2,ctx=0,bp=3: r=54632
    /// poc=0,y=0,x=0,bl=3,ctx=0,bp=7: r=51232
    /// Post-skip[0]: r=49528
    /// ```
    ///
    /// The point is the order: AV1 decodes a block as soon as its partition is known, so the first
    /// block must be visited right after the 4th partition symbol (`r=51232`) and its `skip` must
    /// come next, before any further partition symbol. The parser used to read the superblock's
    /// whole partition tree first and every block afterwards.
    #[test]
    fn frame_0_blocks_are_decoded_between_partition_symbols_like_dav1d() {
        let (_hdr, frames) = crate::ivf::parse_ivf_frames(AV1_IVF_FIXTURE).unwrap();
        let seq_bytes = find_seq_header_bytes(&frames).expect("fixture has a sequence header");
        let obu_data: Vec<u8> = [seq_bytes.as_slice(), frames[0].data.as_slice()].concat();
        let parsed = super::super::parser::ParsedFrame::parse(&obu_data).unwrap();
        let params = parsed.coding_params();

        // dav1d reads no delta_q for this frame (it would show up as `Post-delta_q` right after
        // `Post-cdef_idx` on the first block, which sits at a superblock origin).
        assert!(!params.delta_q_enabled, "frame 0 has delta_q_present = 0");

        let base_qp = parsed.frame_type.base_qp.unwrap() as i16;
        let qcat = (base_qp > 20) as u8 + (base_qp > 60) as u8 + (base_qp > 120) as u8;
        let mut state = crate::tile::TileState {
            decoder: crate::SymbolDecoder::new_with_qcat(&parsed.tile_data, qcat).unwrap(),
            mv_ctx: crate::tile::MvPredictorContext::new(
                parsed.dimensions.sb_cols,
                parsed.dimensions.sb_rows,
            ),
            tile_ctx: crate::tile::TileContext::new(
                (parsed.dimensions.sb_cols * parsed.dimensions.sb_size).div_ceil(4),
                (parsed.dimensions.sb_rows * parsed.dimensions.sb_size).div_ceil(4),
            ),
        };
        state.tile_ctx.start_superblock_row();

        // Stop at the first leaf and report where the decoder stood when it was reached.
        let mut first_leaf = None;
        let result = crate::tile::partition::parse_partition_recursive(
            &mut state,
            0,
            0,
            crate::tile::BlockSize::Block128x128,
            params.mi_rows,
            params.mi_cols,
            0,
            &mut |state, leaf| {
                first_leaf = Some((leaf.x, leaf.y, leaf.size, state.decoder.decoder.range));
                Err(bitvue_engine::BitvueError::InvalidData(
                    "stop at the first leaf".to_string(),
                ))
            },
        );
        assert!(result.is_err());
        let (x, y, size, range) = first_leaf.expect("a leaf was reached");
        assert_eq!((x, y), (0, 0));
        assert_eq!(
            (size.width(), size.height()),
            (8, 16),
            "VERT_B's first block is the full-height left one"
        );
        assert_eq!(
            range, 51232,
            "the first leaf follows the 4th partition symbol"
        );
    }

    /// Ground truth from the same dav1d 1.5.1 trace (see
    /// `frame_0_blocks_are_decoded_between_partition_symbols_like_dav1d`): the decoder's `rng`
    /// after each syntax element of frame 0's first block, up to its first coefficient token.
    ///
    /// ```text
    /// poc=0,y=0,x=0,bl=0..3 ...                   r=63552, 50112, 54632, 51232   (partition x4)
    /// Post-skip[0]: r=49528          Post-cdef_idx[0]: r=49864
    /// Post-ymode[0]: r=47640         Post-uvmode[0]: r=60524
    /// Post-y_pal[1]: r=49216         Post-pal[pl=0,sz=2,...]: r=43528
    /// Post-uv_pal[0]: r=57128        Post-y-pal-indices: r=33029   (128 symbols)
    /// Post-tx[7]: r=49524            Post-non-zero[2][0][0]: r=48266
    /// Post-txtp-intra[7->1][0][6->2]: r=59440
    /// Post-eob_bin_128[0][0][4]: r=59456  Post-eob_hi_bit: r=56960  Post-eob[9]: r=56840
    /// Post-lo_tok[2][0][1][9=3=1]: r=55611
    /// ```
    ///
    /// The block is an 8x16 palette block with `tx=7` (`RTX_8X16`): one rectangular transform
    /// (not a 16x16 one, which would read a 256-coefficient `eob_bin` alphabet), a transform type
    /// chosen from the 7-symbol intra set because the *smaller* side is 8, then `eob` and the
    /// end-of-block token. Each value pins a decision that was once wrong here: `tx_size` for a
    /// rectangular block and its per-axis context, and the `get_tx_set` side that picks the CDF.
    #[test]
    fn frame_0_first_block_matches_the_dav1d_trace_through_its_end_of_block_token() {
        const ORACLE: &[u32] = &[
            63552, 50112, 54632, 51232, // partition symbols
            49528, 49864, 47640, 60524, // skip, cdef_idx, ymode, uvmode
            49216, 43528, 57128, 33029, // y_pal, palette colours, uv_pal, palette indices
            49524, 48266, 59440, // tx_size, all_zero, transform type
            59456, 56960, 56840, 55611, // eob_bin, eob_hi_bit, eob, first token
        ];
        let (_hdr, frames) = crate::ivf::parse_ivf_frames(AV1_IVF_FIXTURE).unwrap();
        let seq_bytes = find_seq_header_bytes(&frames).expect("fixture has a sequence header");
        let obu_data: Vec<u8> = [seq_bytes.as_slice(), frames[0].data.as_slice()].concat();
        let parsed = super::super::parser::ParsedFrame::parse(&obu_data).unwrap();
        let params = parsed.coding_params();

        let base_qp = parsed.frame_type.base_qp.unwrap() as i16;
        let qcat = (base_qp > 20) as u8 + (base_qp > 60) as u8 + (base_qp > 120) as u8;
        let mut state = crate::tile::TileState {
            decoder: crate::SymbolDecoder::new_with_qcat(&parsed.tile_data, qcat).unwrap(),
            mv_ctx: crate::tile::MvPredictorContext::new(
                parsed.dimensions.sb_cols,
                parsed.dimensions.sb_rows,
            ),
            tile_ctx: crate::tile::TileContext::new(
                (parsed.dimensions.sb_cols * parsed.dimensions.sb_size).div_ceil(4),
                (parsed.dimensions.sb_rows * parsed.dimensions.sb_size).div_ceil(4),
            ),
        };
        state.tile_ctx.start_superblock_row();
        state.decoder.decoder.range_trace = Some(Vec::new());

        let mut sb_ctx = crate::tile::SuperblockCtx::new(0, 0, parsed.dimensions.sb_size);
        let _ = crate::tile::partition::parse_partition_recursive(
            &mut state,
            0,
            0,
            crate::tile::BlockSize::Block128x128,
            params.mi_rows,
            params.mi_cols,
            0,
            &mut |state, leaf| {
                let rect = crate::tile::BlockRect {
                    x: leaf.x,
                    y: leaf.y,
                    width: leaf.size.width(),
                    height: leaf.size.height(),
                };
                // Decode the first block only; whatever follows its end-of-block token is
                // covered by later oracle checkpoints.
                let _ = crate::tile::parse_coding_unit(state, &mut sb_ctx, rect, &params, base_qp);
                Err(bitvue_engine::BitvueError::InvalidData(
                    "stop after the first block".to_string(),
                ))
            },
        );
        let trace: Vec<u32> = state
            .decoder
            .decoder
            .range_trace
            .take()
            .unwrap()
            .into_iter()
            .map(|(rng, _, _)| rng)
            .collect();

        // The four partition symbols come first, back to back.
        assert_eq!(&trace[..4], &ORACLE[..4]);
        // Everything after is an in-order subsequence; the palette index map is the longest gap
        // (127 context-coded symbols between `uv_pal` and `tx_size`).
        let mut at = 4;
        for (i, want) in ORACLE.iter().enumerate().skip(4) {
            let found = trace[at..]
                .iter()
                .take(140)
                .position(|r| r == want)
                .unwrap_or_else(|| {
                    panic!("oracle checkpoint #{i} (r={want}) not reached after symbol {at}")
                });
            at += found + 1;
        }
    }

    /// The whole key frame (frame 0, 320x240, 82,966 coded symbols) decodes bit-exactly like
    /// dav1d 1.5.1: after every symbol of a multi-symbol alphabet the arithmetic decoder's
    /// `(rng, cnt, dif)` equals the reference decoder's.
    ///
    /// The reference values come from a dav1d build with a one-line trace in `ctx_norm`
    /// (`MS rng=%u cnt=%d dif=%llx` after each decode, `--threads 1 --limit 1`). They are folded into
    /// an FNV-1a-64 of the lines `"{rng} {cnt} {dif}\n"` (`dif` in decimal) so the test needs no
    /// 82,966-line fixture. Any wrong CDF, context, symbol order or missing/extra read changes the
    /// digest; unlike the first-block test above, this covers every block of the frame --
    /// palette maps, rectangular and 64-wide transforms, 1D transform classes, chroma, filter
    /// intra, golomb tails.
    ///
    /// Symbols of one-symbol alphabets (the placeholder `partition` read of a 4x4 child) consume no
    /// bits and are not part of the trace.
    #[test]
    fn frame_0_decodes_bit_exactly_like_dav1d() {
        const ORACLE_SYMBOLS: usize = 82_966;
        const ORACLE_DIGEST: u64 = 0xe7c9_0862_8b33_a6a4;

        let (_hdr, frames) = crate::ivf::parse_ivf_frames(AV1_IVF_FIXTURE).unwrap();
        let seq_bytes = find_seq_header_bytes(&frames).expect("fixture has a sequence header");
        let obu_data: Vec<u8> = [seq_bytes.as_slice(), frames[0].data.as_slice()].concat();
        let parsed = super::super::parser::ParsedFrame::parse(&obu_data).unwrap();
        let params = parsed.coding_params();

        let base_qp = parsed.frame_type.base_qp.unwrap() as i16;
        let qcat = (base_qp > 20) as u8 + (base_qp > 60) as u8 + (base_qp > 120) as u8;
        let dims = &parsed.dimensions;
        let mut state = crate::tile::TileState {
            decoder: crate::SymbolDecoder::new_with_qcat(&parsed.tile_data, qcat).unwrap(),
            mv_ctx: crate::tile::MvPredictorContext::new(dims.sb_cols, dims.sb_rows),
            tile_ctx: crate::tile::TileContext::new(
                (dims.sb_cols * dims.sb_size).div_ceil(4),
                (dims.sb_rows * dims.sb_size).div_ceil(4),
            ),
        };
        state.decoder.decoder.range_trace = Some(Vec::new());

        let mut qp = base_qp;
        for sb_y in 0..dims.sb_rows {
            state.tile_ctx.start_superblock_row();
            for sb_x in 0..dims.sb_cols {
                let (_sb, new_qp) = crate::parse_superblock(
                    &mut state,
                    sb_x * dims.sb_size,
                    sb_y * dims.sb_size,
                    dims.sb_size,
                    &params,
                    qp,
                )
                .expect("frame 0 parses without error");
                qp = new_qp;
            }
        }

        let trace = state.decoder.decoder.range_trace.take().unwrap();
        let mut digest = 0xcbf2_9ce4_8422_2325u64;
        for (rng, cnt, dif) in &trace {
            for byte in format!("{rng} {cnt} {dif}\n").bytes() {
                digest ^= u64::from(byte);
                digest = digest.wrapping_mul(0x0000_0100_0000_01b3);
            }
        }
        assert_eq!(trace.len(), ORACLE_SYMBOLS, "number of coded symbols");
        assert_eq!(digest, ORACLE_DIGEST, "decoder state diverges from dav1d");
    }

    /// The fixture's chunk 1 holds a hidden frame (refreshes slot 6) and the shown frame
    /// (refreshes slot 2), both with order hint 1: state threaded across it must contain both
    /// refreshes, not only the first frame's.
    #[test]
    fn state_threading_applies_every_frame_of_a_temporal_unit() {
        let (_hdr, frames) = crate::ivf::parse_ivf_frames(AV1_IVF_FIXTURE).unwrap();
        let seq_bytes = find_seq_header_bytes(&frames).expect("fixture has a sequence header");
        let seq = crate::parse_sequence_header(
            &crate::obu::ObuIterator::new(&seq_bytes)
                .next_obu_with_offset()
                .unwrap()
                .unwrap()
                .obu
                .payload,
        )
        .unwrap();
        let state = crate::frame_header_full::thread_ref_state_before(&frames, &seq, 2).unwrap();
        let hints = state.ref_order_hint();
        assert_eq!(hints[6], 1, "hidden frame's refresh of slot 6");
        assert_eq!(hints[2], 1, "shown frame's refresh of slot 2");
        assert_eq!(hints[0], 0, "slot 0 still holds the key frame");
    }

    /// Decodes the first `count` frames of the fixture in decode order (one entry per Frame OBU,
    /// so a hidden frame is its own entry), threading reference state and the motion field, and
    /// returns each frame's `(rng, cnt, dif)` trace.
    fn decode_fixture_traces(count: usize) -> Vec<Vec<(u32, i32, usize)>> {
        decode_traces(AV1_IVF_FIXTURE, count)
    }

    /// [`decode_fixture_traces`] for any IVF.
    fn decode_traces(ivf: &[u8], count: usize) -> Vec<Vec<(u32, i32, usize)>> {
        let (_hdr, frames) = crate::ivf::parse_ivf_frames(ivf).unwrap();
        let seq_bytes = find_seq_header_bytes(&frames).expect("stream has a sequence header");
        let seq = crate::parse_sequence_header(
            &crate::obu::ObuIterator::new(&seq_bytes)
                .next_obu_with_offset()
                .unwrap()
                .unwrap()
                .obu
                .payload,
        )
        .unwrap();
        let mut state = crate::overlay_extraction::StreamDecodeState::new();
        let mut traces = Vec::new();
        for chunk in &frames {
            let mut iter = crate::obu::ObuIterator::new(&chunk.data);
            let mut has_frame = false;
            while let Some(Ok(found)) = iter.next_obu_with_offset() {
                if found.obu.header.obu_type != crate::obu::ObuType::Frame {
                    continue;
                }
                has_frame = true;
                if traces.len() == count {
                    return traces;
                }
                let raw = &chunk.data[found.offset..found.offset + found.consumed];
                let obu_data = [seq_bytes.as_slice(), raw].concat();
                state.decode_next(&obu_data, &seq).unwrap();
                traces.push(state.last_trace.clone());
            }
            // A unit without a frame (a `show_existing_frame` header) still updates the state.
            if !has_frame {
                let obu_data = [seq_bytes.as_slice(), chunk.data.as_slice()].concat();
                state.skip_unit(&obu_data).unwrap();
            }
        }
        traces
    }

    /// Every symbol the decoder reads for the first five frames of the fixture (key frame, hidden
    /// ARF, shown inter frame, two more inter frames) is identical to dav1d's: the arithmetic
    /// decoder's `(rng, cnt, dif)` after each symbol, from a dav1d 1.5.1 debug build with its
    /// `msac` instrumented. Comparing the full state (not only the decoded value) catches reads
    /// with a wrong probability even when they decode to the same bit. Each tuple is the symbol
    /// count and the FNV-1a digest of the `"{rng} {cnt} {dif}\n"` lines of the whole frame.
    ///
    /// Frames 1..=4 exercise: a temporal unit holding two frames (reference state threaded
    /// through both), CDFs loaded from a reference (`primary_ref_frame`), MV candidate stacks and
    /// contexts, temporal projection, the compound syntax, and chroma transform types inherited
    /// from luma.
    #[test]
    fn first_five_frames_decode_symbol_for_symbol_like_dav1d() {
        const ORACLE: [(usize, u64); 5] = [
            (82_966, 0xe7c9_0862_8b33_a6a4),
            (41_785, 0xf637_9543_fc90_f2b5),
            (4_844, 0x7f2d_df0b_b44d_b942),
            (4_149, 0x083f_d2d3_9c64_6428),
            (2_194, 0x73ce_16d6_2ef2_1c03),
        ];
        let traces = decode_fixture_traces(ORACLE.len());
        assert_eq!(traces.len(), ORACLE.len());
        for (frame, (trace, &(symbols, digest))) in traces.iter().zip(&ORACLE).enumerate() {
            let mut hash = 0xcbf2_9ce4_8422_2325u64;
            for (rng, cnt, dif) in trace {
                for byte in format!("{rng} {cnt} {dif}\n").bytes() {
                    hash ^= u64::from(byte);
                    hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
                }
            }
            assert_eq!(trace.len(), symbols, "frame {frame}: number of symbols");
            assert_eq!(
                hash, digest,
                "frame {frame}: decoder state diverges from dav1d"
            );
        }
    }

    /// Decodes every frame of `clip` and requires each frame's symbol count and FNV-1a digest of
    /// the `"{rng} {cnt} {dif}\n"` lines to equal `oracle`, which comes from dav1d 1.5.1
    /// (instrumented `msac`).
    fn assert_clip_decodes_like_dav1d(clip: &[u8], oracle: &[(usize, u64)]) {
        let traces = decode_traces(clip, oracle.len());
        assert_eq!(traces.len(), oracle.len());
        let matches = |trace: &[(u32, i32, usize)], &(symbols, digest): &(usize, u64)| {
            let mut hash = 0xcbf2_9ce4_8422_2325u64;
            for (rng, cnt, dif) in trace {
                for byte in format!("{rng} {cnt} {dif}\n").bytes() {
                    hash ^= u64::from(byte);
                    hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
                }
            }
            trace.len() == symbols && hash == digest
        };
        for (frame, (trace, expected)) in traces.iter().zip(oracle).enumerate() {
            assert!(
                matches(trace, expected),
                "frame {frame} diverges from dav1d ({} symbols, expected {})",
                trace.len(),
                expected.0
            );
        }
    }

    /// All 274 frames of the fixture (every Frame OBU in decode order, hidden frames included)
    /// decode symbol for symbol like dav1d 1.5.1. This covers what the first-five-frames test
    /// cannot: long reference chains, temporal MV projection when a reference slot holds an intra
    /// frame (dav1d keeps no motion vectors for those), the tiny 3-byte tiles, and frames that
    /// load CDFs from far back. Each entry is the frame's symbol count and the FNV-1a digest of
    /// its `"{rng} {cnt} {dif}\n"` lines.
    #[test]
    fn whole_fixture_decodes_symbol_for_symbol_like_dav1d() {
        const ORACLE: [(usize, u64); 274] = [
            (82_966, 0xe7c9_0862_8b33_a6a4),
            (41_785, 0xf637_9543_fc90_f2b5),
            (4_844, 0x7f2d_df0b_b44d_b942),
            (4_149, 0x083f_d2d3_9c64_6428),
            (2_194, 0x73ce_16d6_2ef2_1c03),
            (2_466, 0x3e1d_d9cf_06bc_5b6b),
            (2_389, 0x5800_aa55_771b_28a0),
            (1_638, 0x6ef4_dbe4_6c4a_7e36),
            (2_378, 0x514c_3f3f_4119_abae),
            (1_470, 0x1290_f7e6_b86b_2904),
            (1_531, 0x3dc8_7a18_4db8_341a),
            (53, 0xd917_b23f_be83_ec37),
            (7_573, 0xb9b7_f172_4565_b9da),
            (2_856, 0x06cb_0a00_bf47_3238),
            (2_277, 0xbd56_bd67_cd29_db6d),
            (2_076, 0x3d3c_9d8b_f155_c613),
            (2_608, 0x6fe8_a078_4128_2e31),
            (3_228, 0x9911_9c8c_4337_5df3),
            (2_506, 0x243d_e85e_30ee_38e1),
            (2_328, 0xa0b4_cd7c_cd16_820f),
            (2_494, 0x8642_bf98_d2a8_739e),
            (2_097, 0x9a3a_3345_3edb_98ae),
            (54, 0x2bb4_1364_35d1_1e72),
            (13_885, 0x753a_7ad2_87ef_3969),
            (3_128, 0x508e_3425_b38d_6082),
            (3_038, 0xa176_0921_02d1_53b1),
            (4_133, 0xa826_7102_dd0a_8e03),
            (3_264, 0xd3b7_d493_0bf2_a1b8),
            (30_637, 0xbd30_3955_2fba_baec),
            (2_495, 0xacbf_101b_20d1_2218),
            (2_039, 0x3898_bf9b_7442_6bac),
            (2_302, 0xc911_ee02_a185_a1b0),
            (2_684, 0x7da3_4b5f_69b9_d35a),
            (54, 0x2bb4_1364_35d1_1e72),
            (11_846, 0x9641_53b8_0e18_67d4),
            (3_338, 0x7792_eb46_e352_52cd),
            (2_320, 0xd998_ca4c_b9dc_cff1),
            (3_758, 0x1454_de33_f2da_bdc4),
            (2_505, 0xfa83_2b5e_6113_a3d0),
            (3_017, 0xff78_58c8_13a1_3d08),
            (3_116, 0xa68e_ac26_4b87_48e6),
            (2_606, 0x1af6_aa4b_0c2c_1ca2),
            (2_318, 0x15bb_a4e3_8ae9_0bf3),
            (2_767, 0x3015_bde3_0370_039b),
            (54, 0x2bb4_1364_35d1_1e72),
            (38_504, 0xe074_e0a0_347f_0836),
            (2_781, 0x09a6_2650_9896_b7d8),
            (3_251, 0x5318_cd06_70e2_89c8),
            (3_114, 0x7e70_a3ad_c5ed_bbd5),
            (2_267, 0x379d_493c_4dfb_1d48),
            (4_031, 0xaaba_84b8_dd3c_47f0),
            (3_363, 0x527f_8010_3e1d_d041),
            (3_042, 0x3dec_2c00_38b4_f520),
            (2_651, 0xb291_4c29_a113_c92c),
            (2_926, 0xd894_1e20_143a_e743),
            (62, 0x0554_0322_5ed1_7605),
            (12_920, 0x8347_5b42_c770_67dc),
            (4_358, 0xccba_2c27_b88a_0422),
            (3_824, 0xad74_2adb_ed6a_f135),
            (5_316, 0x3643_a955_48a3_38fd),
            (3_776, 0x5d58_4bc9_c69c_1a76),
            (3_981, 0xa4aa_31f9_7f10_d098),
            (2_935, 0x2488_6101_7f8f_42cf),
            (2_865, 0x2956_bafd_6aab_4a29),
            (2_925, 0xc4fc_4a7a_1e71_bc64),
            (3_102, 0x41f4_00be_cb93_b274),
            (55, 0x3add_fc6c_ce8a_08dd),
            (33_754, 0xfb27_67b8_9a62_cf23),
            (4_151, 0x6394_02e8_bcf2_66ce),
            (3_750, 0x5437_c747_cd4b_2a54),
            (2_621, 0xe45c_2d47_ce11_7c9a),
            (6_311, 0x1d38_a2e4_fbd1_2110),
            (3_915, 0x0534_a164_19bb_2141),
            (3_284, 0x401a_f6c3_0a40_1090),
            (3_698, 0xe843_36cf_0945_e209),
            (3_196, 0xff31_fcb4_4d4d_a60a),
            (3_297, 0x701a_4212_5937_112d),
            (56, 0x653f_eac3_a2d4_de26),
            (11_376, 0xb71b_dc4d_b9a0_4cc6),
            (4_291, 0xe730_7a77_1ca5_07c3),
            (2_957, 0xf026_6cff_1c34_e26a),
            (4_570, 0xc8fe_2a4d_2ac3_c12b),
            (3_323, 0x0c2c_b134_247a_81b6),
            (38_935, 0x428a_0b8b_1fa8_9074),
            (3_390, 0xe099_931f_84a6_ea31),
            (2_426, 0x57b3_4b19_1e43_29bd),
            (2_539, 0x3eb0_3fd3_37d2_072d),
            (3_228, 0xb0ad_acac_b628_dc4f),
            (53, 0xd917_b23f_be83_ec37),
            (18_813, 0xff35_6513_b7fd_c65c),
            (3_321, 0xf2c6_e9df_a903_14ef),
            (2_687, 0xadff_a1c1_3388_efb5),
            (3_996, 0x9e04_8174_1642_ca1c),
            (3_223, 0xedec_db08_5e4b_d387),
            (3_320, 0x30a2_ee3c_b5ba_a9ad),
            (3_398, 0x26a9_1259_adb1_af0b),
            (2_915, 0x784a_b2d4_db79_6c3b),
            (2_337, 0xdd31_c7ce_0a9b_7395),
            (3_253, 0x0858_094c_8758_d861),
            (54, 0x2bb4_1364_35d1_1e72),
            (39_311, 0x9d03_42c2_738d_f522),
            (3_688, 0x66ef_4700_5dc2_370c),
            (2_992, 0x348e_6e3e_3098_8643),
            (4_064, 0xc2a8_9f49_8b83_578f),
            (2_924, 0x2cbe_1b67_0fa8_47bd),
            (4_336, 0xbd9d_0436_5ada_d2b0),
            (3_451, 0xe7e8_3fc5_cac5_4bc8),
            (2_946, 0x8ea3_11f5_ffea_f3e1),
            (2_804, 0x7b25_b0eb_c2a8_73d2),
            (3_785, 0x4fb1_209a_8c0a_a959),
            (62, 0x0554_0322_5ed1_7605),
            (14_088, 0x4c4c_d3f4_37ff_311e),
            (4_958, 0xba71_5f2d_125d_6269),
            (4_534, 0x9913_c4d4_d51b_c435),
            (6_903, 0xa97d_98ce_97e2_dbea),
            (4_138, 0xed69_be73_b694_7f5c),
            (4_910, 0xdb63_9d3f_8a89_b466),
            (3_507, 0x3529_dadf_c344_4c4c),
            (2_997, 0x20de_50f2_f710_1b1f),
            (3_305, 0x147e_cab8_6c79_b0d9),
            (3_856, 0x6a3f_27a4_74ce_eadc),
            (55, 0x3add_fc6c_ce8a_08dd),
            (8_248, 0x8770_781b_2990_5cd8),
            (4_645, 0x23f5_8406_8a08_8840),
            (2_934, 0x2e59_a7fb_10ba_32bb),
            (3_216, 0x6f9e_3caa_7ac1_7550),
            (2_994, 0x6cdb_f7ed_5e8a_2a68),
            (4_385, 0x9c2d_e857_3f76_2454),
            (3_511, 0x2267_02fb_9e17_c7ea),
            (4_120, 0x480d_bfb0_6a43_0b34),
            (3_754, 0x1dd9_3a64_4e9a_7a63),
            (4_300, 0x7a55_79b3_40cc_b818),
            (56, 0x0726_72dc_cd18_5d2e),
            (35_063, 0x24eb_990f_19dc_cb7a),
            (4_838, 0xcce7_d81e_ba93_3840),
            (4_243, 0x1560_6a7d_9888_f9ef),
            (7_623, 0xbbf9_c851_2077_e12d),
            (4_018, 0x70b1_9222_553e_13ed),
            (43_211, 0xa5c7_4bf5_04cc_12ac),
            (4_274, 0x5570_c1e4_aa26_8db9),
            (2_897, 0xbaf9_778b_0074_899e),
            (4_769, 0xba22_86fb_ad93_e911),
            (3_476, 0xc7d5_7e4f_1a05_70f6),
            (53, 0xd917_b23f_be83_ec37),
            (22_551, 0xbc9a_5faa_7246_a410),
            (2_612, 0x820b_3c62_e9f9_5757),
            (2_595, 0x4c8a_a839_a629_f99c),
            (3_424, 0x305a_5ee2_7f2f_d7f3),
            (3_211, 0x42e9_a3f8_1556_f74d),
            (3_926, 0xd800_55c7_b2b1_9681),
            (3_518, 0x14c0_a02c_2b6b_92fc),
            (3_205, 0x32a7_629f_4113_42c9),
            (2_751, 0x045c_7950_d33b_24b3),
            (3_655, 0x7705_96a8_4421_45f1),
            (55, 0x1ac1_8227_1339_2db6),
            (43_772, 0x4c85_dc60_fbd2_4811),
            (3_679, 0x8fcc_8e65_2470_f1a4),
            (3_453, 0x0b98_9347_4b11_448e),
            (4_257, 0x800e_46ee_76d3_e3d4),
            (2_893, 0x32e3_6df7_113b_5bae),
            (4_490, 0xb097_cac7_8fbf_9ab4),
            (4_115, 0x137a_3e26_f967_de83),
            (3_613, 0x5681_004b_48b3_1086),
            (3_239, 0x509b_e8dd_f86a_3566),
            (3_894, 0x8bc3_8343_bb01_c8c3),
            (62, 0x0554_0322_5ed1_7605),
            (13_818, 0xf21c_0be0_daab_fd99),
            (6_079, 0x0ac7_bdc4_f883_3b07),
            (3_897, 0x3e42_1102_7220_b2b5),
            (7_638, 0x0305_59a9_d453_31aa),
            (4_189, 0x2667_8304_0227_0d05),
            (5_493, 0xeac2_a811_d54d_7921),
            (3_115, 0x94ab_17e9_0993_9725),
            (3_147, 0xd894_ab5b_6746_93b3),
            (3_153, 0xf26e_a34e_6429_3e61),
            (3_723, 0x296d_21b1_c5cb_d34a),
            (55, 0x4c0a_0324_edfe_5611),
            (8_510, 0x4d72_b043_25ec_3e59),
            (4_682, 0xc3b8_d2be_2f80_7064),
            (3_605, 0x1600_22c7_5df4_4ebe),
            (3_363, 0x3ad1_2c58_ea7c_ca8f),
            (3_446, 0x73dd_6f47_d8d2_cc0a),
            (4_701, 0x71ff_3fdd_21b3_635e),
            (3_404, 0x2fe3_6b7c_f41d_0d15),
            (4_439, 0xed48_5196_65e9_cb26),
            (4_458, 0xe1ac_16e7_88ba_05ca),
            (4_283, 0x32f6_e205_f9f2_a8e8),
            (129, 0x0840_6362_f2b5_3f3f),
            (11_320, 0x201b_c1aa_0776_e508),
            (5_860, 0x60df_9fd4_3fcd_70e2),
            (5_082, 0x67a3_9d2f_c63b_95ce),
            (5_673, 0x7bbc_a327_edbc_95fe),
            (3_808, 0x2e4f_d66f_ccef_cdab),
            (45_673, 0x8fe8_489c_005f_7188),
            (3_465, 0xc39e_6546_21c0_d3ab),
            (2_898, 0x0c91_104e_7ea3_bfc3),
            (2_884, 0x0d40_86c9_9b5a_cb4b),
            (3_875, 0xa6c7_ad23_ee29_e725),
            (55, 0x3add_fc6c_ce8a_08dd),
            (27_756, 0x467a_e092_db48_c245),
            (3_473, 0xa030_a0c4_e9c0_735d),
            (3_120, 0x468e_256d_0a30_e30f),
            (4_809, 0x96e5_98f2_8fd6_7fc9),
            (3_432, 0xd6ad_57bb_9914_fcc3),
            (3_897, 0xc7e7_9eef_1448_08b4),
            (3_689, 0x70ac_b598_c40e_aa05),
            (3_636, 0xca36_dba0_a6d1_878a),
            (2_554, 0x8dae_7d76_63bf_abb4),
            (3_486, 0xcb42_0dcc_c780_fb7f),
            (218, 0x41b1_ba89_7365_ab6d),
            (63_113, 0xcf09_64ec_3e8c_c1fb),
            (3_390, 0xe7a2_c59f_86d3_fd92),
            (10_438, 0x79ca_eb2e_12c3_fef4),
            (4_290, 0xe025_0852_8717_50df),
            (3_272, 0x8d62_7330_c156_d1aa),
            (5_278, 0x9653_65a3_17a3_49b9),
            (4_025, 0x4431_94e7_7d16_01bc),
            (3_380, 0xca50_8770_1dcb_ad67),
            (3_261, 0x0941_204b_f6e6_2ae3),
            (4_633, 0xa5a0_de7b_1d2d_72a8),
            (82, 0x0eaa_9f80_83ff_affb),
            (13_750, 0x45f0_c270_b6cd_10c4),
            (8_172, 0xf0e9_5b4e_c6bd_8c3f),
            (4_319, 0x8ad8_cc84_ff02_8972),
            (8_307, 0x1e0c_ef1c_60c1_950c),
            (4_391, 0x4f98_9253_53d9_a70f),
            (4_788, 0x4e61_49ed_5700_edad),
            (3_054, 0x09f0_e0e3_c128_d7b8),
            (3_591, 0x25ea_d4af_88e0_44dd),
            (3_350, 0x4688_40e5_f28b_c3bf),
            (3_852, 0xb363_3058_c79b_175a),
            (101, 0x0e1e_c9fb_635e_54f8),
            (7_804, 0xdece_4bca_251b_3ac8),
            (3_789, 0x667b_c03d_7b05_aef2),
            (3_059, 0x208a_3167_ebb6_e43e),
            (3_755, 0xeb2d_c249_1ea5_7bf4),
            (3_484, 0x48c8_f981_8244_aaef),
            (4_732, 0x964f_f44e_5e65_e36a),
            (3_927, 0x428e_5c1c_22a2_239e),
            (4_615, 0xa1b4_290e_42fa_3b1d),
            (4_110, 0x45d0_57f7_4df1_2119),
            (5_140, 0x84a6_cbb0_d945_515b),
            (127, 0xba7f_34b4_a3b1_668a),
            (11_506, 0x8832_17a8_f426_8371),
            (6_274, 0x970d_b8ec_d917_f0c0),
            (4_599, 0x1419_5cef_453f_bc15),
            (5_834, 0x1231_f7c3_0844_a880),
            (4_293, 0xf87d_e8f1_57c6_d703),
            (45_839, 0x242d_091e_f91f_7577),
            (4_071, 0x6455_fe04_5c5f_ee9e),
            (2_846, 0x6d70_2fb4_5420_7336),
            (2_623, 0x7f9f_f1b3_38ef_59d5),
            (3_580, 0x0a41_9dca_6d8c_33aa),
            (54, 0x2bb4_1364_35d1_1e72),
            (33_996, 0xdc6e_cc66_f70e_5976),
            (4_158, 0xd781_47af_d129_9bc8),
            (2_888, 0xe56b_155b_07b7_cea4),
            (4_890, 0x0d74_a565_cd35_2492),
            (3_332, 0x7ab0_dcc7_0df6_df1c),
            (3_401, 0x9086_ef8f_4247_0f98),
            (3_398, 0x5d30_21b8_2de5_2016),
            (3_064, 0xc653_54f2_19c0_5b34),
            (2_682, 0xab32_e416_0596_47bb),
            (3_815, 0x9123_462f_a70d_da49),
            (54, 0x2bb4_1364_35d1_1e72),
            (3_740, 0x1822_f919_8fce_5973),
            (3_594, 0xf529_6976_3389_7125),
            (3_821, 0x8441_858a_bb44_ebc5),
            (2_899, 0x30e4_672d_084a_48c3),
            (5_714, 0x7429_dfbe_f1e8_5b37),
            (4_882, 0xf1fc_22f0_3ff6_4642),
            (3_695, 0x9618_bb3c_02ac_e5ee),
            (4_090, 0xa5b4_8d51_fa97_e63c),
            (3_836, 0xb92f_d650_f448_a2c3),
        ];
        assert_clip_decodes_like_dav1d(AV1_IVF_FIXTURE, &ORACLE);
    }

    /// 12 frames of a synthetic test pattern encoded with rav1e
    /// (`test_data/av1_rav1e_testsrc2.ivf`, see `docs/PARITY_CHECKLIST.md`). Unlike the fixture it
    /// is a typical encoder output: every frame updates its CDFs at the end of the frame
    /// (`disable_frame_end_update_cdf = 0`), frames load their CDFs from a reference, segmentation
    /// features carry over between frames, and its key frame uses loop restoration.
    #[test]
    fn rav1e_clip_decodes_symbol_for_symbol_like_dav1d() {
        const ORACLE: [(usize, u64); 12] = [
            (35_205, 0xf4d9_4bf6_a511_c1c9),
            (22_555, 0x42d5_5223_c818_2a18),
            (11_473, 0x2aba_ef6c_d37e_9aed),
            (8_381, 0x7973_73b8_e2e5_2884),
            (7_941, 0x1819_8407_bce6_6e1c),
            (20_374, 0xcdb0_c9eb_d52b_09d8),
            (10_645, 0x6431_871b_8538_b750),
            (8_469, 0x7b6e_fc92_c08b_1b1a),
            (7_945, 0x13ad_24e2_75f6_7348),
            (12_429, 0x65b6_a13a_35da_e0ef),
            (8_843, 0xab45_9ff6_76ca_d372),
            (7_863, 0x777e_d8ef_8959_bc8f),
        ];
        assert_clip_decodes_like_dav1d(
            include_bytes!("../../../../test_data/av1_rav1e_testsrc2.ivf"),
            &ORACLE,
        );
    }

    /// 13 frames (one hidden) encoded with aomenc: IntraBC and delta-q on the key frame,
    /// compound prediction with jnt_comp and masked compound, one interpolation filter shared by
    /// both axes (`enable_dual_filter = 0`).
    #[test]
    fn aomenc_clip_decodes_symbol_for_symbol_like_dav1d() {
        const ORACLE: [(usize, u64); 13] = [
            (36_976, 0x8ad9_2b7c_9793_aec0),
            (19_122, 0xcafe_83d0_6a62_253f),
            (12_248, 0xe7cd_6f61_5a51_e547),
            (11_875, 0x5039_7c82_99d0_01eb),
            (8_005, 0x04b3_a8ea_64c4_e504),
            (7_801, 0x6534_6ff8_eeb5_2875),
            (7_760, 0xd58f_2223_14d3_c2af),
            (10_303, 0xe424_7dbb_a1de_5b36),
            (9_710, 0x47ef_35be_d6d3_eba9),
            (8_740, 0x5769_e594_84ed_da30),
            (7_697, 0x3780_efae_761a_0647),
            (7_396, 0xd3e4_edc0_1224_567d),
            (53, 0x0573_dc75_c66c_f904),
        ];
        assert_clip_decodes_like_dav1d(
            include_bytes!("../../../../test_data/av1_aomenc_testsrc2.ivf"),
            &ORACLE,
        );
    }

    /// 12 frames encoded with SVT-AV1 4.2.0: IntraBC on the key frame, hierarchical prediction
    /// with compound references, interintra, a single shared interpolation filter.
    #[test]
    fn svt_av1_clip_decodes_symbol_for_symbol_like_dav1d() {
        const ORACLE: [(usize, u64); 12] = [
            (38_011, 0xc2d7_a4e8_4882_c9c3),
            (16_872, 0x6fa0_6593_810f_b287),
            (14_111, 0x4138_60de_4b02_849b),
            (10_287, 0x40e2_4554_944b_70c5),
            (6_905, 0x75e2_1291_f50b_be32),
            (7_013, 0x278c_b5b9_05f5_2a31),
            (10_393, 0x2cea_3cf9_261d_ddf5),
            (9_541, 0x22c6_a392_f6e6_0bb2),
            (7_660, 0x18e9_3670_c2e3_3d8f),
            (12_720, 0x80b8_3fba_1760_50a4),
            (6_213, 0x2aac_3c40_a08b_177a),
            (9_211, 0x17fe_18f1_94a0_2a25),
        ];
        assert_clip_decodes_like_dav1d(
            include_bytes!("../../../../test_data/av1_svtav1_testsrc2.ivf"),
            &ORACLE,
        );
    }

    /// Real temporal MV candidates (spec 7.9/7.10, `crate::tile::motion_field`) -- sequential
    /// full-fixture regression, threading one `MotionFieldState` across all 250 frames in decode
    /// order (mirroring `bitvue-sidecar/src/av1_features.rs`'s own sequential
    /// `parse_frame_header_full` scan pattern, but for real per-frame MV/ref data instead of just
    /// header flags). No independent value oracle exists for decoded MV values in this environment
    /// (every earlier phase of this multi-session effort shares this caveat -- see
    /// `docs/DEVELOPMENT_PHASES.md`), so correctness here rests on: (1) self-consistency -- the
    /// full sequential 250-frame decode with real temporal candidates wired into
    /// `single_ref_mv_stack`/`inter_mode_context` completes without panics or hard errors: any
    /// desync in the new source-selection/projection/storage logic would very likely surface as an
    /// entropy-decoder crash the same way the residual()/ref_frame()/delta_q_enabled bugs earlier
    /// in this effort did; (2) a non-degenerate check that real temporal sources actually get found
    /// at least once (this fixture's inter frames are `use_ref_frame_mvs=true` per the DRL commit,
    /// so a construction bug that always yields zero sources -- e.g. an inverted priority/sign
    /// check -- would silently make the whole feature a no-op without ever failing a test).
    #[test]
    fn real_fixture_temporal_mv_candidates_are_wired_and_non_degenerate() {
        let (_hdr, frames) = crate::ivf::parse_ivf_frames(AV1_IVF_FIXTURE).unwrap();
        let seq_bytes = find_seq_header_bytes(&frames).expect("fixture has a sequence header");
        let seq_header = {
            let mut iter = crate::obu::ObuIterator::new(&seq_bytes);
            let found = iter
                .next_obu_with_offset()
                .expect("seq_bytes starts with the sequence header OBU")
                .expect("sequence header OBU parses");
            crate::parse_sequence_header(&found.obu.payload)
                .expect("fixture sequence header parses")
        };
        let enable_order_hint = seq_header.enable_order_hint;
        let order_hint_bits = seq_header
            .order_hint_bits_minus_1
            .map(|v| v as u32 + 1)
            .unwrap_or(0);

        let mut mf_state = crate::tile::MotionFieldState::new();
        let mut total_cus = 0usize;
        let mut frames_with_temporal_sources = 0usize;

        for (idx, frame) in frames.iter().enumerate() {
            let obu_data: Vec<u8> = [seq_bytes.as_slice(), frame.data.as_slice()].concat();
            let parsed = super::super::parser::ParsedFrame::parse(&obu_data).unwrap();
            if !parsed.has_tile_data() {
                continue;
            }

            let prev_ref_order_hint = *mf_state.ref_state.ref_order_hint();
            let ref_frame_idx_if_temporal = if parsed.use_ref_frame_mvs {
                parsed.ref_frame_idx
            } else {
                None
            };
            let temporal_input = ref_frame_idx_if_temporal.map(|ref_frame_idx| {
                let sources = crate::tile::select_motion_field_sources(
                    &mf_state,
                    &prev_ref_order_hint,
                    &ref_frame_idx,
                    parsed.order_hint,
                    enable_order_hint,
                    order_hint_bits,
                );
                if !sources.is_empty() {
                    frames_with_temporal_sources += 1;
                }
                let cols_8x8 = parsed.dimensions.width.div_ceil(8).max(1);
                let rows_8x8 = parsed.dimensions.height.div_ceil(8).max(1);
                let projected =
                    crate::tile::project_motion_field(&sources, &mf_state, cols_8x8, rows_8x8);
                let pocdiff: [i32; 7] = std::array::from_fn(|i| {
                    let ref_poc = prev_ref_order_hint[ref_frame_idx[i] as usize];
                    crate::frame_header_full::relative_dist(
                        parsed.order_hint,
                        ref_poc,
                        enable_order_hint,
                        order_hint_bits,
                    )
                    .clamp(-31, 31) as i32
                });
                (projected, pocdiff)
            });

            let result = parse_all_coding_units_with_temporal(
                &parsed,
                temporal_input.as_ref().map(|(p, d)| (p, *d)),
            );
            let cus = result.unwrap_or_else(|e| panic!("frame {idx} failed to parse: {e}"));
            total_cus += cus.len();

            // spec 7.9 storage, using this frame's real decoded CUs -- feeds future frames'
            // temporal candidates.
            let mfmv_sign: [bool; 7] = match parsed.ref_frame_idx {
                Some(ref_frame_idx) => std::array::from_fn(|i| {
                    let ref_poc = prev_ref_order_hint[ref_frame_idx[i] as usize];
                    crate::frame_header_full::relative_dist(
                        ref_poc,
                        parsed.order_hint,
                        enable_order_hint,
                        order_hint_bits,
                    ) < 0
                }),
                None => [false; 7],
            };
            let grid = crate::tile::store_motion_field(
                &cus,
                parsed.dimensions.width,
                parsed.dimensions.height,
                &mfmv_sign,
            );
            mf_state.update(
                &prev_ref_order_hint,
                parsed.refresh_frame_flags,
                parsed.ref_frame_idx.as_ref(),
                grid,
                !parsed.frame_type.is_intra_only || parsed.allow_intrabc,
            );
        }

        assert!(
            total_cus > 0,
            "expected at least some coding units across the fixture"
        );
        // Real measured value on this fixture: 119/250 frames find a source (~48%) -- early
        // frames before any ref has a saved grid yet never do, matching real dav1d's own
        // n_mfmvs==0 case. A >=10% bar catches "the priority/sign logic is essentially always
        // wrong" (e.g. an inverted `dist(...) > 0` check) without being so tight that unrelated
        // future changes to this exact ratio spuriously fail the test.
        assert!(
            frames_with_temporal_sources * 10 >= frames.len(),
            "expected at least 10% of frames to find a valid temporal MV source in this fixture \
             (all inter frames are use_ref_frame_mvs=true per the DRL commit; got {frames_with_temporal_sources}/{}) \
             -- a near-zero count suggests the source-selection priority/sign logic is wrong, not \
             just approximate",
            frames.len()
        );
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
        // NOT a "majority must split" bar -- a real dav1d oracle cross-check (`DEBUG_BLOCK_INFO`,
        // 2026-08-13, see `docs/DEVELOPMENT_PHASES.md`'s Phase 4 entry) confirms real encoders
        // favor NOT splitting for this fixture's content: frame 13's own superblock(0,0) has 3
        // real `vartxtree` reads in the oracle trace, only 1 of which splits (~33%), matching this
        // fixture-wide ratio almost exactly once the tile_data-offset/global_motion_params/
        // Horz4-Vert4 bugs this session found were fixed. A prior "majority" threshold here was a
        // guess calibrated against a since-fixed buggy decode, not a real spec/encoder property --
        // the actual non-degenerate bar is just "some CUs split and some don't", not "most do".
        assert!(
            cus_with_a_real_split > 0 && cus_with_a_real_split < eligible_cu_count,
            "expected both split and not-split real txfm_split outcomes among eligible CUs \
             (neither all-split nor all-not-split), got {cus_with_a_real_split}/{eligible_cu_count} \
             -- the txfm_split reads may be degenerating to a fixed outcome"
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
        // See `real_fixture_inter_var_tx_is_not_degenerate`'s doc for why this isn't a "majority"
        // bar -- a real dav1d oracle cross-check confirms ~1/3 split is the actual real ratio for
        // this fixture's content, not a bug.
        assert!(
            cus_with_a_real_split > 0 && cus_with_a_real_split < eligible_cu_count,
            "expected both split and not-split real txfm_split outcomes among eligible non-square \
             CUs, got {cus_with_a_real_split}/{eligible_cu_count}"
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

    /// Regression test for the real per-frame qindex-bucket (`qcat`) residual-coefficient CDF
    /// selection (previously every frame's residual entropy decode started from dav1d's qindex
    /// bucket 0 regardless of the frame's real `base_q_idx`; buckets 1-3 are now real, and
    /// `parse_all_coding_units_with_temporal` derives `qcat` from `base_qp` via the same formula
    /// as `CdfContext::new_with_qcat`'s doc). Threads the real per-frame `qcat` through the full
    /// 250-frame fixture (mirroring `real_fixture_every_frame_parses_coding_units_without_
    /// panicking_or_erroring`'s sequential scan) and reports the parse-success ratio as the
    /// primary correctness signal (this crate's established precedent for changes to entropy-CDF
    /// selection/context, since no independent value oracle exists for decoded coefficients in
    /// this environment -- see `real_fixture_residual_energy_is_not_degenerate`'s doc).
    ///
    /// Unlike the earlier residual()/ref_frame()/delta_q_enabled bugs this session found (which
    /// were *skipped syntax reads* -- true entropy-decoder desyncs that reliably crash), seeding
    /// the *wrong-but-still-valid* qindex bucket doesn't by itself break arithmetic-decoder
    /// framing: any complete, monotonic CDF keeps `read_symbol` returning some in-range symbol
    /// and consuming a well-defined number of bits, so a parse that used to succeed with the
    /// (wrong) bucket-0-always CDF was already expected to keep succeeding with the (right)
    /// real-qcat CDF -- only the *values* decoded change, not whether parsing completes. The
    /// real regression guard here is therefore two-fold: (1) the parse-success ratio must not
    /// regress (100% before and after on this fixture, asserted below), and (2) the fixture must
    /// actually exercise more than one `qcat` bucket, or this wiring would be untested by every
    /// other test in this file too (they all go through the same `parse_all_coding_units`).
    #[test]
    fn real_fixture_real_qcat_selection_is_exercised_and_parses_cleanly() {
        let (_hdr, frames) = crate::ivf::parse_ivf_frames(AV1_IVF_FIXTURE).unwrap();
        let seq_bytes = find_seq_header_bytes(&frames).expect("fixture has a sequence header");

        let mut ok_count = 0usize;
        let mut err_count = 0usize;
        let mut distinct_qcats: std::collections::HashSet<u8> = Default::default();
        let mut checked_frames = 0usize;

        for (idx, frame) in frames.iter().enumerate() {
            let obu_data: Vec<u8> = [seq_bytes.as_slice(), frame.data.as_slice()].concat();
            let parsed = super::super::parser::ParsedFrame::parse(&obu_data).unwrap();
            if !parsed.has_tile_data() {
                continue;
            }
            checked_frames += 1;

            // Same formula `parse_all_coding_units_with_temporal` uses internally -- recomputed
            // here (rather than exposed as a return value) purely to observe which buckets this
            // fixture actually reaches; production behavior is exercised by the `parse_all_
            // coding_units` call below either way.
            let base_qp = parsed.frame_type.base_qp.unwrap_or(128) as i16;
            let qcat = (base_qp > 20) as u8 + (base_qp > 60) as u8 + (base_qp > 120) as u8;
            distinct_qcats.insert(qcat);

            match parse_all_coding_units(&parsed) {
                Ok(_) => ok_count += 1,
                Err(e) => {
                    err_count += 1;
                    eprintln!("frame {idx} (qcat={qcat}) failed to parse: {e}");
                }
            }
        }

        assert!(checked_frames > 0, "expected real frames with tile data");
        assert_eq!(
            err_count, 0,
            "expected 0 parse errors across {checked_frames} real frames with real per-frame \
             qcat wired in (ok={ok_count}, err={err_count}) -- a wrong-but-still-valid CDF bucket \
             shouldn't desync the arithmetic decoder; any error here suggests qcat computation or \
             threading regressed, not just accuracy"
        );
        assert!(
            distinct_qcats.len() > 1,
            "expected this fixture to exercise more than one qcat bucket (got only {distinct_qcats:?} \
             across {checked_frames} frames) -- if every frame's base_q_idx falls in the same \
             bucket, buckets 1-3 are wired but never actually reached by this regression suite"
        );
    }

    /// Regression test for the real compound MV-stack/DRL port (`SpatialRefContext::
    /// compound_mv_stack`, replacing the old `MvPredictorContext` placeholder + zero DRL bits for
    /// every compound CU). `real_fixture_ref_frame_values_are_not_degenerate` already confirms
    /// this fixture has real compound blocks with non-zero L1 MVs; this test's own value is the
    /// **0-error assertion**: compound DRL is now a real bit-consuming read (previously zero bits
    /// were ever read for compound blocks at all), so a wrong weight/context/branch-selection
    /// implementation would desync the shared arithmetic decoder and surface as a parse error on
    /// a *later* CU/frame, not necessarily the compound block itself -- same class of "silent
    /// desync only visible via a full-fixture parse-error count" this session has repeatedly
    /// relied on (e.g. `real_fixture_real_qcat_selection_is_exercised_and_parses_cleanly` above).
    /// Also reports (doesn't assert -- this fixture may simply never trigger `skip_mode`, same
    /// documented caveat as the segmentation/GmType/DeltaQUAc work) whether any CU ever used the
    /// new `skip_mode` forced-path.
    #[test]
    fn real_fixture_compound_drl_wiring_parses_cleanly() {
        let (_hdr, frames) = crate::ivf::parse_ivf_frames(AV1_IVF_FIXTURE).unwrap();
        let seq_bytes = find_seq_header_bytes(&frames).expect("fixture has a sequence header");

        let mut ok_count = 0usize;
        let mut err_count = 0usize;
        let mut compound_count = 0usize;
        let mut skip_mode_count = 0usize;

        for (idx, frame) in frames.iter().enumerate() {
            let obu_data: Vec<u8> = [seq_bytes.as_slice(), frame.data.as_slice()].concat();
            let parsed = super::super::parser::ParsedFrame::parse(&obu_data).unwrap();
            if !parsed.has_tile_data() {
                continue;
            }
            match parse_all_coding_units(&parsed) {
                Ok(cus) => {
                    ok_count += 1;
                    for cu in cus.iter() {
                        if cu.skip_mode {
                            skip_mode_count += 1;
                        }
                        if cu.is_inter() && cu.ref_frames[1] != crate::tile::RefFrame::Intra {
                            compound_count += 1;
                        }
                    }
                }
                Err(e) => {
                    err_count += 1;
                    eprintln!("frame {idx} failed to parse: {e}");
                }
            }
        }

        assert_eq!(
            err_count, 0,
            "expected 0 parse errors across all frames with real compound DRL bits wired in \
             (ok={ok_count}, err={err_count}, {compound_count} compound CUs seen) -- a wrong DRL \
             weight/context/branch computation would desync the shared arithmetic decoder"
        );
        assert!(
            compound_count > 0,
            "expected at least one compound CU to exercise the new compound_mv_stack path"
        );
        eprintln!(
            "real_fixture_compound_drl_wiring_parses_cleanly: {skip_mode_count}/{compound_count} \
             compound CUs had skip_mode=true"
        );
    }
}
