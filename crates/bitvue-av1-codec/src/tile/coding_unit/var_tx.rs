//! Inter / IntraBC transform-size tree (`read_block_tx_size()`, spec 5.11.16-5.11.18): the
//! recursive var-tx walk that yields the per-leaf transform blocks `residual()` iterates.

use super::types::TxBlock;
use crate::symbol::cdf::tx_size_class;
use crate::symbol::SymbolDecoder;
use bitvue_engine::Result;

/// Compute the real (or, for non-`Switchable` `TxMode`s, deterministic-no-read) transform block
/// breakdown for one INTER **or IntraBC** coding unit -- spec 5.11.16's `read_block_tx_size()`
/// (real spec gates the recursive var-tx tree on `is_inter`, and IntraBC blocks are classified
/// `is_inter` for this purpose despite being coded within an intra frame -- see this function's
/// call sites' docs). Supports genuinely rectangular coding units (real `Max_Tx_Size_Rect`, not
/// this crate's older square-only `TxSize::from_dimensions` heuristic) -- verified against
/// rav1d's `dav1d_max_txfm_size_for_bs` table (`src/tables.c`) directly: for every real AV1 block
/// size up to 64 in each axis, the natural starting max transform size is simply the block's own
/// size (real var-tx recursion, not this table, is what performs any further splitting); only
/// block sizes wider or taller than 64 (128-wide/tall) cap that axis at 64 (spec: no transform
/// exceeds 64x64). Hence `max_ytx = (width.min(64), height.min(64))` -- no lookup table needed,
/// unlike what an earlier pass expected.
///
/// Mirrors rav1d's `read_vartx_tree` (`src/decode.c`, `memorysafety/rav1d`/`videolan/dav1d`,
/// BSD-2-Clause) dispatch order:
/// 1. `skip` (spec: no bits read regardless of `TxMode` -- but `Switchable` still needs the
///    block's natural max size written into `var_tx_context`'s neighbor arrays for later blocks'
///    context, even though this CU's own leaf list is moot since `residual()` never runs for a
///    skipped CU). Returns `None` (caller's `!cu.skip` gate already skips the residual loop).
/// 2. `coded_lossless` (this crate's frame-wide approximation of spec's per-segment
///    `LosslessArray`) forces uniform 4x4 tiling, no bits read, regardless of `TxMode` -- checked
///    before `TxMode` since lossless overrides even `Switchable`.
/// 3. `TxfmMode::Only4x4`/`Largest`: deterministic uniform tiling (4x4, or the CU's own natural
///    max size), no bits read -- `TxfmMode::Switchable` is the only case needing a real read.
/// 4. `TxfmMode::Switchable`: real recursive `read_var_tx_size` walk.
///
/// For CUs bigger than one max-size transform tile in either axis (>64 wide and/or tall), tiles
/// the walk across each max-size block -- matches rav1d's own `for y_off in 0..bh4/h { for x_off
/// in 0..bw4/w { read_tx_tree(...) } }`.
#[allow(clippy::too_many_arguments)]
pub(super) fn compute_inter_tx_blocks(
    decoder: &mut SymbolDecoder,
    tile_ctx: &mut crate::tile::TileContext,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    skip: bool,
    coded_lossless: bool,
    txfm_mode: crate::frame_header::TxfmMode,
    mi_rows: u32,
    mi_cols: u32,
) -> Result<Option<Vec<TxBlock>>> {
    if !(4..=128).contains(&width) || !(4..=128).contains(&height) {
        return Ok(None);
    }
    let (x4, y4) = (x / 4, y / 4);
    let (width_4x4, height_4x4) = (width / 4, height / 4);
    let (max_ytx_w, max_ytx_h) = (width.min(64), height.min(64));

    if skip {
        tile_ctx.set_var_tx_class(
            x4,
            y4,
            width_4x4,
            height_4x4,
            tx_size_class(max_ytx_w) as u8,
            tx_size_class(max_ytx_h) as u8,
        );
        return Ok(None);
    }

    let uniform_size = if coded_lossless {
        Some((4, 4))
    } else {
        match txfm_mode {
            crate::frame_header::TxfmMode::Only4x4 => Some((4, 4)),
            crate::frame_header::TxfmMode::Largest => Some((max_ytx_w, max_ytx_h)),
            crate::frame_header::TxfmMode::Switchable => None,
        }
    };

    let mut leaves = Vec::new();
    if let Some((uw, uh)) = uniform_size {
        let (uw4, uh4) = (uw / 4, uh / 4);
        let (uw_class, uh_class) = (tx_size_class(uw) as u8, tx_size_class(uh) as u8);
        let mut ly = y4;
        while ly < y4 + height_4x4 {
            let mut lx = x4;
            while lx < x4 + width_4x4 {
                leaves.push(TxBlock {
                    x4: lx,
                    y4: ly,
                    width_px: uw,
                    height_px: uh,
                });
                tile_ctx.set_var_tx_class(lx, ly, uw4, uh4, uw_class, uh_class);
                lx += uw4;
            }
            ly += uh4;
        }
    } else {
        let (tile_w4, tile_h4) = (max_ytx_w / 4, max_ytx_h / 4);
        let mut ty = y4;
        while ty < y4 + height_4x4 {
            let mut tx = x4;
            while tx < x4 + width_4x4 {
                read_var_tx_size(
                    decoder,
                    tile_ctx,
                    tx,
                    ty,
                    max_ytx_w,
                    max_ytx_h,
                    0,
                    mi_rows,
                    mi_cols,
                    &mut leaves,
                )?;
                tx += tile_w4;
            }
            ty += tile_h4;
        }
    }
    Ok(Some(leaves))
}

/// Recursively read `read_var_tx_size()` (spec 5.11.17/18) for one max-size transform tile,
/// genuinely rectangular starting sizes supported -- see `compute_inter_tx_blocks`'s doc. Ports
/// rav1d's `read_tx_tree` (`src/decode.c`, `memorysafety/rav1d`/`videolan/dav1d`, BSD-2-Clause)
/// index-for-index, including its real asymmetric-split branching (verified against the C
/// directly, not assumed -- a naive square-only 4-way quad-split, this crate's original
/// implementation, is provably wrong for non-square starting sizes):
///
/// - Reads `txfm_split` only when `depth < 2 && (from_w, from_h) != (4, 4)` (spec: recursion caps
///   at 2 levels below the tile's own starting size, and 4x4 is always terminal) -- `cat =
///   2*(4-max_class)-depth` selects the CDF row (`SymbolDecoder::read_txfm_split`'s doc, `
///   max_class` = the square-up class of the *larger* dimension, matching real `t_dim->max`),
///   context from `TileContext::var_tx_context` (real per-axis width/height classes, not one
///   shared class).
/// - If split and `max_class > 1` (bigger than an 8x8-equivalent): recurse into 1, 2, or 4
///   children at `sub` -- real spec's `sub` always halves only the *larger* dimension (or both,
///   for a square starting size); the *count* of children read is asymmetric too: child `(0,0)`
///   always, `(1,0)` only when `from_w >= from_h`, `(0,1)` only when `from_h >= from_w`, and
///   `(1,1)` only when *both* hold (i.e. only ever for a square starting size) -- so a wide
///   starting size (`from_w > from_h`) reads exactly 2 children side by side, a tall one reads 2
///   stacked, and only a square one reads all 4. Skips (early return, no read, no leaves) any
///   child whose origin is `>= mi_rows`/`mi_cols` -- spec: transform blocks entirely outside the
///   frame aren't separately coded (the same shape as, but distinct from,
///   `tile::partition::parse_partition_recursive`'s own frame-edge check).
/// - If split and `max_class <= 1` (an 8x8-or-smaller-max-class starting size, e.g. an 8x8, 4x8,
///   or 8x4): no further symbol is read (spec-deterministic, always all-4x4) -- the leaf loop
///   below naturally produces the right leaf count since it always walks `from`'s full footprint
///   at 4x4 granularity in that case.
/// - Otherwise (not split, or `depth`/`from` already forced no-read): `(from_w, from_h)` itself is
///   the one leaf covering this node's whole footprint.
///
/// Every leaf updates `TileContext::set_var_tx_class` across its own footprint before returning,
/// matching rav1d's `case.set_disjoint(&dir.tx, tx)`.
#[allow(clippy::too_many_arguments)]
fn read_var_tx_size(
    decoder: &mut SymbolDecoder,
    tile_ctx: &mut crate::tile::TileContext,
    x4: u32,
    y4: u32,
    from_w: u32,
    from_h: u32,
    depth: u8,
    mi_rows: u32,
    mi_cols: u32,
    out: &mut Vec<TxBlock>,
) -> Result<()> {
    if x4 >= mi_cols || y4 >= mi_rows {
        return Ok(());
    }
    let from_w_class = tx_size_class(from_w) as u8;
    let from_h_class = tx_size_class(from_h) as u8;
    let max_class = from_w_class.max(from_h_class);
    let is_4x4 = from_w == 4 && from_h == 4;
    let is_split = if depth < 2 && !is_4x4 {
        let cat = 2 * (4 - max_class) - depth;
        let (a, l) = tile_ctx.var_tx_context(x4, y4, from_w_class, from_h_class);
        decoder.read_txfm_split(cat, a + l)?
    } else {
        false
    };

    if is_split && max_class > 1 {
        let (sub_w, sub_h) = match from_w.cmp(&from_h) {
            std::cmp::Ordering::Greater => (from_w / 2, from_h),
            std::cmp::Ordering::Less => (from_w, from_h / 2),
            std::cmp::Ordering::Equal => (from_w / 2, from_h / 2),
        };
        let (half_w4, half_h4) = (sub_w / 4, sub_h / 4);
        read_var_tx_size(
            decoder,
            tile_ctx,
            x4,
            y4,
            sub_w,
            sub_h,
            depth + 1,
            mi_rows,
            mi_cols,
            out,
        )?;
        if from_w >= from_h {
            read_var_tx_size(
                decoder,
                tile_ctx,
                x4 + half_w4,
                y4,
                sub_w,
                sub_h,
                depth + 1,
                mi_rows,
                mi_cols,
                out,
            )?;
        }
        if from_h >= from_w {
            read_var_tx_size(
                decoder,
                tile_ctx,
                x4,
                y4 + half_h4,
                sub_w,
                sub_h,
                depth + 1,
                mi_rows,
                mi_cols,
                out,
            )?;
            if from_w >= from_h {
                read_var_tx_size(
                    decoder,
                    tile_ctx,
                    x4 + half_w4,
                    y4 + half_h4,
                    sub_w,
                    sub_h,
                    depth + 1,
                    mi_rows,
                    mi_cols,
                    out,
                )?;
            }
        }
        return Ok(());
    }

    let (leaf_w, leaf_h) = if is_split { (4, 4) } else { (from_w, from_h) };
    let (leaf_w4, leaf_h4) = (leaf_w / 4, leaf_h / 4);
    let (w4, h4) = (from_w / 4, from_h / 4);
    let (leaf_w_class, leaf_h_class) = (tx_size_class(leaf_w) as u8, tx_size_class(leaf_h) as u8);
    let mut ly = y4;
    while ly < y4 + h4 {
        let mut lx = x4;
        while lx < x4 + w4 {
            out.push(TxBlock {
                x4: lx,
                y4: ly,
                width_px: leaf_w,
                height_px: leaf_h,
            });
            tile_ctx.set_var_tx_class(lx, ly, leaf_w4, leaf_h4, leaf_w_class, leaf_h_class);
            lx += leaf_w4;
        }
        ly += leaf_h4;
    }
    Ok(())
}
