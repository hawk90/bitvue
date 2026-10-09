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
    let sb_size = parsed.dimensions.sb_size;
    let sb_cols = parsed.dimensions.sb_cols;
    let sb_rows = parsed.dimensions.sb_rows;
    let frame_params = parsed.coding_params();

    let mut all_cus = Vec::new();

    // Pre-allocate capacity based on superblock count
    let estimated_cus = (sb_cols * sb_rows) as usize * 4;
    all_cus.reserve(estimated_cus);

    // Every tile starts from the saved CDFs of the primary reference frame, or from the defaults
    // seeded with the real per-frame qindex-bucket (`qcat`) residual-coefficient CDFs -- see
    // `crate::symbol::cdf::CdfContext::new_with_qcat`'s doc for the dav1d selection formula.
    let qcat = (base_qp > 20) as u8 + (base_qp > 60) as u8 + (base_qp > 120) as u8;
    let initial_cdf = inputs
        .initial_cdf
        .unwrap_or_else(|| crate::symbol::CdfContext::new_with_qcat(qcat));

    let tiles = tile_slices(parsed)?;
    let layout = parsed.tiles.as_ref().filter(|l| l.tile_count() > 1);
    let cols = layout.map_or(1, |l| l.tile_cols());
    let update_tile = layout.map_or(0, |l| l.context_update_tile_id);

    let superblocks_total = sb_cols * sb_rows;
    let mut superblocks_decoded = 0;
    let mut padding_conformant = true;
    let mut final_cdf = None;
    // dav1d decodes one superblock row of every tile column in turn, so its trace interleaves
    // the tiles: segments are keyed `(tile row, superblock row, tile column)` to merge alike.
    #[cfg(test)]
    let mut trace_segments: Vec<TraceSegment> = Vec::new();

    for (tile_num, bytes) in &tiles {
        let (col_sbs, row_sbs) = match layout {
            Some(l) => {
                let (col, row) = ((tile_num % cols) as usize, (tile_num / cols) as usize);
                (
                    l.col_starts_sb[col]..l.col_starts_sb[col + 1],
                    l.row_starts_sb[row]..l.row_starts_sb[row + 1],
                )
            }
            None => (0..sb_cols, 0..sb_rows),
        };

        let decoder = crate::SymbolDecoder::with_cdf_context(bytes, initial_cdf.clone())?;
        let mut tile_ctx = tile_context_for(parsed);
        let sb_4x4 = sb_size / 4;
        tile_ctx.set_tile_extent(
            col_sbs.start * sb_4x4,
            col_sbs.end * sb_4x4,
            row_sbs.start * sb_4x4,
            row_sbs.end * sb_4x4,
        );
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

        // Track running QP value across superblocks
        let mut current_qp = base_qp;
        #[cfg(test)]
        let tile_pos = layout.map_or((0, 0), |_| (tile_num / cols, tile_num % cols));
        'tile: for sb_y in row_sbs.clone() {
            #[cfg(test)]
            {
                let done = state.decoder.decoder.range_trace.take().unwrap_or_default();
                if let Some(last) = trace_segments.last_mut() {
                    last.1.extend(done);
                }
                trace_segments.push(((tile_pos.0, sb_y, tile_pos.1), Vec::new()));
                state.decoder.decoder.range_trace = Some(Vec::new());
            }
            state.tile_ctx.start_superblock_row();
            for sb_x in col_sbs.clone() {
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
                        break 'tile;
                    }
                }
            }
        }
        padding_conformant &= state.decoder.padding_is_conformant();
        #[cfg(test)]
        if let Some(last) = trace_segments.last_mut() {
            last.1
                .extend(state.decoder.decoder.range_trace.take().unwrap_or_default());
        }
        if *tile_num == update_tile || final_cdf.is_none() {
            final_cdf = Some(state.decoder.cdf_context);
        }
    }

    let outcome = DecodeOutcome {
        superblocks_total,
        superblocks_decoded,
        padding_conformant,
    };
    tracing::debug!(
        "Parsed {} coding units from tile data ({:?})",
        all_cus.len(),
        outcome
    );
    Ok(FrameDecode {
        units: all_cus,
        outcome,
        final_cdf: final_cdf.unwrap_or(initial_cdf),
        #[cfg(test)]
        trace: {
            trace_segments.sort_by_key(|(key, _)| *key);
            trace_segments.into_iter().flat_map(|(_, t)| t).collect()
        },
    })
}

/// `(tile row, superblock row, tile column)`: the order dav1d decodes (and traces) tiles in.
#[cfg(test)]
type TileRowKey = (u32, u32, u32);

/// The `(rng, cnt, dif)` trace of one superblock row of one tile.
#[cfg(test)]
type TraceSegment = (TileRowKey, Vec<(u32, i32, usize)>);

/// The tiles of the frame as `(tile number, bytes)`, in decode order. A frame without a known
/// layout, or with a single tile, is all of `tile_data`; otherwise every tile group is split
/// into its tiles (spec 5.11.1).
fn tile_slices(parsed: &ParsedFrame) -> Result<Vec<(u32, &[u8])>, BitvueError> {
    let data: &[u8] = &parsed.tile_data;
    let Some(layout) = parsed.tiles.as_ref().filter(|l| l.tile_count() > 1) else {
        return Ok(vec![(0, data)]);
    };
    let whole = std::iter::once(0..data.len());
    let groups: Vec<std::ops::Range<usize>> = if parsed.tile_groups.is_empty() {
        whole.collect()
    } else {
        parsed.tile_groups.clone()
    };
    let mut tiles = Vec::new();
    for group in &groups {
        let payload = &data[group.clone()];
        for (tile, range) in crate::tile::layout::split_tile_group(payload, layout)? {
            tiles.push((tile, &payload[range]));
        }
    }
    Ok(tiles)
}

#[cfg(test)]
mod fixture_tests;
#[cfg(test)]
mod oracle_tests;
#[cfg(test)]
mod test_support;
