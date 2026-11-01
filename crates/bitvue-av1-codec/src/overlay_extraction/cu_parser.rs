//! Shared coding unit parsing utilities
//!
//! This module contains shared code for parsing coding units from AV1 tile data.
//! It is used by QP, MV, and partition extractors to avoid duplication.

use std::sync::Arc;

use bitvue_engine::BitvueError;

use super::cache::{compute_frame_cache_key, get_or_parse_coding_units, ParsedCodingUnits};
use super::parser::{FrameDimensions, ParsedFrame};
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

/// Most luma samples a frame may have for the decode to allocate its per-tile context: AV1
/// level 6.3's `MaxPicSize` (spec A.3). The frame header's 16-bit width and height allow 65536 x
/// 65536, whose context grid alone is over 2 GB (found by the `frame_analysis` fuzz target).
pub const MAX_FRAME_LUMA_SAMPLES: u64 = 35_651_584;

/// Whether a frame of `dims` is small enough for anything sized by the frame (the tile context,
/// the saved motion field, the projected one) to be allocated. A 5 KB file can declare 65536 x
/// 65536, and the saved motion field alone was 3 GB for it (found by the `stream_analysis` fuzz
/// target).
pub(crate) fn frame_within_decode_limit(dims: &FrameDimensions) -> bool {
    u64::from(dims.width) * u64::from(dims.height) <= MAX_FRAME_LUMA_SAMPLES
}

fn check_frame_size(dims: &FrameDimensions) -> Result<(), BitvueError> {
    if !frame_within_decode_limit(dims) {
        return Err(BitvueError::InvalidData(format!(
            "frame size {}x{} exceeds the {} luma samples of AV1 level 6.3",
            dims.width, dims.height, MAX_FRAME_LUMA_SAMPLES
        )));
    }
    Ok(())
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
    tile_ctx.set_global_motion(
        parsed.gm_type,
        parsed.gm_params,
        parsed.allow_high_precision_mv,
        parsed.force_integer_mv,
    );
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
    check_frame_size(&parsed.dimensions)?;
    let base_qp = parsed.frame_type.base_qp.unwrap_or(128) as i16;
    let sb_cols = parsed.dimensions.sb_cols;
    let sb_rows = parsed.dimensions.sb_rows;
    let frame_params = parsed.coding_params();

    let mut all_cus = Vec::new();

    // Pre-allocate capacity based on superblock count (a guess: only the allocation depends on it)
    let estimated_cus = (sb_cols * sb_rows) as usize * 4;
    all_cus.reserve(estimated_cus);

    // Every tile starts from the saved CDFs of the primary reference frame, or from the defaults
    // seeded with the real per-frame qindex-bucket (`qcat`) residual-coefficient CDFs -- see
    // `crate::symbol::cdf::CdfContext::new_with_qcat`'s doc for the dav1d selection formula.
    let initial_cdf = inputs.initial_cdf.unwrap_or_else(|| {
        crate::symbol::CdfContext::new_with_qcat(super::stream_state::quantizer_category(base_qp))
    });

    let tiles = tile_slices(parsed)?;
    // `> 1` or `>= 1` decodes the same: a one-tile layout covers the whole frame and its
    // `context_update_tile_id` can only name that tile (#277).
    let layout = parsed.tiles.as_ref().filter(|l| l.tile_count() > 1);
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
        let (col_sbs, row_sbs) = tile_superblock_ranges(layout, *tile_num, sb_cols, sb_rows);

        let mut state = start_tile_state(
            parsed,
            bytes,
            &initial_cdf,
            &col_sbs,
            &row_sbs,
            inputs.temporal,
        )?;

        // (running QP is tracked across superblocks inside `decode_superblock_rows`)
        #[cfg(test)]
        let tile_pos = layout.map_or((0, 0), |l| {
            (tile_num / l.tile_cols(), tile_num % l.tile_cols())
        });
        decode_superblock_rows(
            &mut state,
            &row_sbs,
            &col_sbs,
            parsed,
            &frame_params,
            base_qp,
            &mut all_cus,
            &mut superblocks_decoded,
            #[cfg(test)]
            tile_pos,
            #[cfg(test)]
            &mut trace_segments,
        );
        padding_conformant &= state.decoder.padding_is_conformant();
        #[cfg(test)]
        if let Some(last) = trace_segments.last_mut() {
            last.1
                .extend(state.decoder.decoder.range_trace.take().unwrap_or_default());
        }
        if takes_final_cdf(*tile_num, update_tile, final_cdf.is_some()) {
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

/// Whether a tile's final CDFs replace the frame's saved ones: the `context_update_tile_id` tile's
/// do, and the first tile decoded provides them when that tile is missing from the data.
fn takes_final_cdf(tile_num: u32, update_tile: u32, have_one: bool) -> bool {
    tile_num == update_tile || !have_one
}

/// The superblock columns and rows tile `tile_num` covers. No layout (or a single tile) means the
/// whole frame.
fn tile_superblock_ranges(
    layout: Option<&crate::frame_header::TileLayout>,
    tile_num: u32,
    sb_cols: u32,
    sb_rows: u32,
) -> (std::ops::Range<u32>, std::ops::Range<u32>) {
    match layout {
        Some(l) => {
            let cols = l.tile_cols();
            let (col, row) = ((tile_num % cols) as usize, (tile_num / cols) as usize);
            (
                l.col_starts_sb[col]..l.col_starts_sb[col + 1],
                l.row_starts_sb[row]..l.row_starts_sb[row + 1],
            )
        }
        None => (0..sb_cols, 0..sb_rows),
    }
}

/// `(tile row, superblock row, tile column)`: the order dav1d decodes (and traces) tiles in.
#[cfg(test)]
type TileRowKey = (u32, u32, u32);

/// The `(rng, cnt, dif)` trace of one superblock row of one tile.
#[cfg(test)]
type TraceSegment = (TileRowKey, Vec<(u32, i32, usize)>);

/// The symbol decoder and prediction contexts for one tile, positioned at its first superblock.
fn start_tile_state<'a>(
    parsed: &ParsedFrame,
    bytes: &'a [u8],
    initial_cdf: &crate::symbol::CdfContext,
    col_sbs: &std::ops::Range<u32>,
    row_sbs: &std::ops::Range<u32>,
    temporal: Option<(&crate::tile::ProjectedMotionField, [i32; 7])>,
) -> Result<crate::tile::TileState<'a>, BitvueError> {
    let sb_size = parsed.dimensions.sb_size;
    let sb_cols = parsed.dimensions.sb_cols;
    let sb_rows = parsed.dimensions.sb_rows;
    let decoder = crate::SymbolDecoder::with_cdf_context(bytes, initial_cdf.clone())?;
    let mut tile_ctx = tile_context_for(parsed);
    let sb_4x4 = sb_size / 4;
    tile_ctx.set_tile_extent(
        col_sbs.start * sb_4x4,
        col_sbs.end * sb_4x4,
        row_sbs.start * sb_4x4,
        row_sbs.end * sb_4x4,
    );
    if let Some((projected, pocdiff)) = temporal {
        tile_ctx.set_temporal_context(projected.clone(), pocdiff);
        tile_ctx.set_mv_precision(parsed.allow_high_precision_mv, parsed.force_integer_mv);
    }
    #[cfg_attr(not(test), allow(unused_mut))]
    let mut state = crate::tile::TileState {
        decoder,
        mv_ctx: crate::tile::MvPredictorContext::new(sb_cols, sb_rows),
        tile_ctx,
    };
    #[cfg(test)]
    {
        state.decoder.decoder.range_trace = Some(Vec::new());
    }
    Ok(state)
}

/// Decodes the superblocks of one tile in raster order, stopping at the first one that fails.
#[allow(clippy::too_many_arguments)]
fn decode_superblock_rows(
    state: &mut crate::tile::TileState<'_>,
    row_sbs: &std::ops::Range<u32>,
    col_sbs: &std::ops::Range<u32>,
    parsed: &ParsedFrame,
    frame_params: &crate::tile::FrameCodingParams,
    base_qp: i16,
    all_cus: &mut Vec<crate::tile::CodingUnit>,
    superblocks_decoded: &mut u32,
    #[cfg(test)] tile_pos: (u32, u32),
    #[cfg(test)] trace_segments: &mut Vec<TraceSegment>,
) {
    let sb_size = parsed.dimensions.sb_size;
    // Track running QP value across superblocks
    let mut current_qp = base_qp;
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
        // The left contexts span the tile's full height and are addressed by absolute 4x4 row, so
        // a new superblock row only reads entries nothing has written yet and this reset changes
        // nothing in the decode (dropping it leaves every test green, #176). It mirrors the
        // spec's per-row left-context reset and keeps that true if the addressing changes.
        state.tile_ctx.start_superblock_row();
        for sb_x in col_sbs.clone() {
            let sb_pixel_x = sb_x * sb_size;
            let sb_pixel_y = sb_y * sb_size;

            match crate::parse_superblock(
                state,
                sb_pixel_x,
                sb_pixel_y,
                sb_size,
                frame_params,
                current_qp,
            ) {
                Ok((sb, new_qp)) => {
                    all_cus.extend(sb.coding_units);
                    current_qp = new_qp;
                    *superblocks_decoded += 1;
                }
                Err(e) => {
                    tracing::debug!(
                        "Failed to parse superblock ({}, {}): {}, stopping",
                        sb_pixel_x,
                        sb_pixel_y,
                        e
                    );
                    // Whatever follows a failure is noise. In practice it also never decodes: with
                    // `continue` here, 35,318 single-byte corruptions (6 fixture frames, every
                    // byte, 3 flip patterns) gave the same superblock and unit counts, because
                    // every later superblock fails again (#176). The `break` stays so that this
                    // does not depend on that.
                    break 'tile;
                }
            }
        }
    }
}

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
mod fixture_test;
#[cfg(test)]
mod oracle_test;
#[cfg(test)]
mod test_support;
#[cfg(test)]
mod tile_range_test;
