//! `residual()` (spec 5.11.34) for a non-skipped block: the luma transform blocks, then U, then V,
//! one 64x64 chunk of the block at a time (a block of at most 64x64 is one chunk). Reading every
//! transform block is required for bitstream alignment, not just for producing residual
//! statistics -- see `SymbolDecoder::read_residual_block`'s doc.

use super::types::CodingUnit;
use crate::symbol::{LumaTxType, ResidualBlockStats, SymbolDecoder, TxClass1d};
use crate::tile::{BlockRect, FrameCodingParams, MiRect, TileContext};
use bitvue_engine::Result;

/// Reads the whole `residual()` of `cu` (which must not be `skip`) and stores the summed
/// statistics in `cu.residual`. `y_mode_raw` is the raw intra `y_mode` symbol (0 for inter
/// blocks), needed by the transform-type read.
pub(super) fn read_residual(
    decoder: &mut SymbolDecoder,
    tile_ctx: &mut TileContext,
    rect: BlockRect,
    mi: MiRect,
    frame: &FrameCodingParams,
    cu: &mut CodingUnit,
    y_mode_raw: u8,
) -> Result<()> {
    let mut summary = ResidualBlockStats::default();
    // Luma transform type at every 4x4 of the block (dav1d `txtp_map`), which an inter block's
    // chroma transforms inherit from.
    let mut luma_types = vec![LumaTxType::DCT; (mi.width * mi.height) as usize];
    // Spec 5.11.34: a block wider or taller than 64 is read one 64x64 chunk at a time, raster
    // order, and each chunk is its luma transforms, then its U transform, then its V transform.
    let (chunks_x, chunks_y) = (
        rect.width.div_ceil(64).max(1),
        rect.height.div_ceil(64).max(1),
    );
    for chunk_y in 0..chunks_y {
        for chunk_x in 0..chunks_x {
            let chunk = (chunk_x, chunk_y);
            read_luma_chunk(
                decoder,
                tile_ctx,
                rect,
                mi,
                frame,
                cu,
                y_mode_raw,
                chunk,
                (&mut summary, &mut luma_types),
            )?;
            read_chroma_residual(decoder, tile_ctx, mi, frame, cu, &luma_types, chunk)?;
        }
    }
    cu.residual = Some(summary);
    Ok(())
}

/// All the luma transform blocks of `cu` in one go (a block of at most 64x64 is a single chunk).
#[cfg(test)]
fn read_luma_residual(
    decoder: &mut SymbolDecoder,
    tile_ctx: &mut TileContext,
    rect: BlockRect,
    mi: MiRect,
    frame: &FrameCodingParams,
    cu: &CodingUnit,
    y_mode_raw: u8,
) -> Result<(ResidualBlockStats, Vec<LumaTxType>)> {
    let mut summary = ResidualBlockStats::default();
    let mut luma_types = vec![LumaTxType::DCT; (mi.width * mi.height) as usize];
    let (chunks_x, chunks_y) = (
        rect.width.div_ceil(64).max(1),
        rect.height.div_ceil(64).max(1),
    );
    for chunk_y in 0..chunks_y {
        for chunk_x in 0..chunks_x {
            read_luma_chunk(
                decoder,
                tile_ctx,
                rect,
                mi,
                frame,
                cu,
                y_mode_raw,
                (chunk_x, chunk_y),
                (&mut summary, &mut luma_types),
            )?;
        }
    }
    Ok((summary, luma_types))
}

/// The luma transform blocks of 64x64 chunk `chunk` (`(column, row)` within the block) of `cu`,
/// accumulated into `summary` and `luma_types`.
#[allow(clippy::too_many_arguments)]
fn read_luma_chunk(
    decoder: &mut SymbolDecoder,
    tile_ctx: &mut TileContext,
    rect: BlockRect,
    mi: MiRect,
    frame: &FrameCodingParams,
    cu: &CodingUnit,
    y_mode_raw: u8,
    chunk: (u32, u32),
    (summary, luma_types): (&mut ResidualBlockStats, &mut [LumaTxType]),
) -> Result<()> {
    let MiRect { x4, y4, .. } = mi;
    let FrameCodingParams {
        is_key_frame,
        tx_type_flags,
        ..
    } = *frame;
    let tx_positions = luma_tx_positions(cu, rect, mi);
    // Whether the whole block is one transform block, not just this chunk.
    let is_single_tx_block = tx_positions.len() == 1;
    let in_chunk = |&(tx_x4, tx_y4, _, _): &(u32, u32, u32, u32)| {
        ((tx_x4 - x4) >> 4, (tx_y4 - y4) >> 4) == chunk
    };
    // Real `txb_skip`/`dc_sign` neighbor context is only trustworthy where transform-block
    // boundaries are real (regular key-frame intra via `tx_size()`, or inter/IntraBC via real
    // `tx_blocks` -- both real bitstream-derived boundaries); other CUs keep the
    // fixed-context-0 fallback and never touch `tile_ctx`'s residual arrays, matching
    // `SymbolDecoder::read_residual_block`'s doc.
    let use_real_residual_ctx = (is_key_frame && !cu.use_intrabc) || cu.tx_blocks.is_some();
    // dav1d's `b->intra`: a block's own prediction kind, not the frame type (intra blocks occur in
    // inter frames), and IntraBC blocks count as inter for the transform-set choice.
    let is_intra_block = cu.is_intra() && !cu.use_intrabc;
    for (tx_x4, tx_y4, tx_w_px, tx_h_px) in tx_positions.into_iter().filter(in_chunk) {
        let (tx_w4, tx_h4) = (tx_w_px / 4, tx_h_px / 4);

        let (txb_skip_ctx, dc_sign_ctx) = if use_real_residual_ctx {
            (
                tile_ctx.txb_skip_context(tx_x4, tx_y4, tx_w4, tx_h4, is_single_tx_block),
                tile_ctx.dc_sign_context(tx_x4, tx_y4, tx_w4, tx_h4),
            )
        } else {
            (0, 0)
        };

        // Real spec order (`decode_coefs`, dav1d `src/recon_tmpl.c`): `all_zero` (`txb_skip`)
        // is read FIRST, unconditionally; `transform_type()` (spec 5.11.47) is read only when
        // that comes back `false` -- NOT unconditionally before it. Getting this backwards
        // was a real, confirmed desync bug: every all-zero transform block (common) previously
        // read a phantom `transform_type` symbol the real encoder never wrote. See
        // `SymbolDecoder::read_txb_skip`'s doc for the full story.
        let all_zero = decoder.read_txb_skip(tx_w_px, tx_h_px, txb_skip_ctx)?;
        let block = if all_zero {
            ResidualBlockStats {
                all_zero: true,
                ..Default::default()
            }
        } else {
            let tx_type = decoder.read_transform_type(
                is_intra_block,
                tx_type_flags.coded_lossless,
                tx_type_flags.qidx_is_zero,
                tx_type_flags.reduced_tx_set,
                tx_w_px,
                tx_h_px,
                y_mode_raw,
            )?;
            let block =
                decoder.read_residual_block(tx_w_px, tx_h_px, tx_type.class, dc_sign_ctx)?;
            for ly in tx_y4.saturating_sub(y4)..(tx_y4 + tx_h4).saturating_sub(y4).min(mi.height) {
                for lx in tx_x4.saturating_sub(x4)..(tx_x4 + tx_w4).saturating_sub(x4).min(mi.width)
                {
                    luma_types[(ly * mi.width + lx) as usize] = tx_type;
                }
            }
            block
        };

        if use_real_residual_ctx {
            let cul_level = block.sum_abs_level.min(63) as u8;
            tile_ctx.set_residual_ctx(tx_x4, tx_y4, tx_w4, tx_h4, cul_level, block.dc_sign_value);
        }

        summary.nonzero_count += block.nonzero_count;
        summary.sum_abs_level += block.sum_abs_level;
        summary.max_level = summary.max_level.max(block.max_level);
    }
    Ok(())
}

/// The luma transform blocks of `cu` as `(x4, y4, width_px, height_px)`, in reading order.
fn luma_tx_positions(cu: &CodingUnit, rect: BlockRect, mi: MiRect) -> Vec<(u32, u32, u32, u32)> {
    let BlockRect { width, height, .. } = rect;
    let MiRect { x4, y4, .. } = mi;
    // Real `tx_blocks` (var-tx, inter only -- see `compute_inter_tx_blocks`'s doc) gives the
    // true per-leaf positions/sizes directly, including genuinely rectangular leaves;
    // everything else still tiles uniformly at `cu.tx_size` (a heuristic for those CUs, not a
    // real bitstream-derived size, still square-only).
    if let Some(blocks) = &cu.tx_blocks {
        blocks
            .iter()
            .map(|b| (b.x4, b.y4, b.width_px, b.height_px))
            .collect()
    } else {
        let tx_px = cu.tx_size.size();
        let tx_cols = width.div_ceil(tx_px).max(1);
        let tx_rows = height.div_ceil(tx_px).max(1);
        let tx_wh4 = tx_px / 4;
        (0..tx_rows)
            .flat_map(|tx_row| {
                (0..tx_cols)
                    .map(move |tx_col| (x4 + tx_col * tx_wh4, y4 + tx_row * tx_wh4, tx_px, tx_px))
            })
            .collect()
    }
}

/// How a block's chroma plane is cut into transform blocks: their size in samples and their
/// count per axis.
struct ChromaTiling {
    tx_w: u32,
    tx_h: u32,
    tiles_x: u32,
    tiles_y: u32,
}

/// The chroma block of an 8x8-or-larger luma block is half its size; a 4-wide/tall block (which
/// shares its chroma with its neighbours) still has a 4-sample chroma side. Each axis is capped at
/// 32 samples per transform block, except in a lossless frame, where spec 5.11.34's `residual()`
/// uses `TX_4X4` for every plane (`txSz = Lossless ? TX_4X4 : get_tx_size(plane, TxSize)`): the
/// chroma block is then cut into 4x4 transform blocks (#388).
fn chroma_tiling(mi: MiRect, lossless: bool) -> ChromaTiling {
    let (chroma_w, chroma_h) = (((mi.width + 1) >> 1) * 4, ((mi.height + 1) >> 1) * 4);
    let (tx_w, tx_h) = if lossless {
        (4, 4)
    } else {
        (chroma_w.min(32), chroma_h.min(32))
    };
    ChromaTiling {
        tx_w,
        tx_h,
        tiles_x: chroma_w.div_ceil(tx_w).max(1),
        tiles_y: chroma_h.div_ceil(tx_h).max(1),
    }
}

/// The index into the block's per-4x4 luma transform types of the sample co-located with chroma
/// transform block `(tile_col, tile_row)`; chroma units are two luma 4x4s wide, and a block
/// narrower than that clamps to its last row/column.
/// The tiles of one plane that belong to a 64x64 luma chunk: `per_chunk` tiles per axis starting at
/// the chunk's index, clipped to the `tiles` the block has.
fn chunk_tiles(chunk: u32, per_chunk: u32, tiles: u32) -> std::ops::Range<u32> {
    chunk * per_chunk..((chunk + 1) * per_chunk).min(tiles)
}

fn colocated_luma_index(
    tile_col: u32,
    tile_row: u32,
    chroma_tx_w4: u32,
    chroma_tx_h4: u32,
    mi: MiRect,
) -> usize {
    let lx = (tile_col * chroma_tx_w4 * 2).min(mi.width.saturating_sub(1));
    let ly = (tile_row * chroma_tx_h4 * 2).min(mi.height.saturating_sub(1));
    (ly * mi.width + lx) as usize
}

fn read_chroma_residual(
    decoder: &mut SymbolDecoder,
    tile_ctx: &mut TileContext,
    mi: MiRect,
    frame: &FrameCodingParams,
    cu: &CodingUnit,
    luma_types: &[LumaTxType],
    chunk: (u32, u32),
) -> Result<()> {
    let MiRect { x4, y4, .. } = mi;
    let tx_type_flags = frame.tx_type_flags;
    // Chroma (U/V) residual -- required for bitstream sync (spec 5.11.34's `residual()`
    // reads luma, then U, then V for every `HasChroma` block). Restricted to non-IntraBC luma
    // coding blocks 8x8 through 128x128 in either dimension (real rectangular chroma tiles
    // supported, since the luma CU itself can be non-square -- see `SymbolDecoder::
    // read_chroma_residual_block`'s doc for the desync bug this closed: every non-square
    // `HasChroma` block's chroma bits were previously never read at all once non-square inter
    // var-tx made non-square CUs common). Chroma's real max transform size caps each axis at
    // 32 independently -- spec `Max_Tx_Size_Rect`, confirmed against rav1d's
    // `DAV1D_MAX_TXFM_SIZE_FOR_BS` table -- regardless of luma size, in a 4:2:0 stream. Not
    // restricted to key frames: real fixture-verified on inter frames too (key-frame content
    // here happens to only ever use unpartitioned 128x128 blocks, so an earlier
    // key-frame-only version of this gate was accidentally *never exercised* by this fixture
    // at all -- see `real_fixture_square_chroma_eligible_blocks_exist_and_parse_cleanly`).
    //
    // Position tracking: chroma tile positions are tracked at the luma CU's `x4/2`/`y4/2`
    // origin (a coordinate-scale approximation, not a truly independent chroma-plane grid --
    // see `TileContext`'s chroma field doc) since only above/left *adjacency* matters for
    // context selection here, not absolute physical distance.
    if tx_type_flags.subsampling_x
        && tx_type_flags.subsampling_y
        && super::contexts::has_chroma(mi, &tx_type_flags)
    {
        let ChromaTiling {
            tx_w: chroma_tx_w,
            tx_h: chroma_tx_h,
            tiles_x: chroma_tiles_x,
            tiles_y: chroma_tiles_y,
        } = chroma_tiling(mi, tx_type_flags.coded_lossless);
        let (chroma_tx_w4, chroma_tx_h4) = (chroma_tx_w / 4, chroma_tx_h / 4);
        let not_one_blk = chroma_tiles_x * chroma_tiles_y > 1;
        let (cx4_base, cy4_base) = (x4 / 2, y4 / 2);
        // A 64x64 luma chunk covers 32x32 chroma samples: one tile of each plane when the chroma
        // transform is 32 wide (or the block is smaller), and `32 / size` per axis for the 4x4
        // transforms of a lossless frame. `chunk` picks the tiles of each plane that belong to it.
        let (chunk_col, chunk_row) = chunk;
        let (per_chunk_x, per_chunk_y) = ((32 / chroma_tx_w).max(1), (32 / chroma_tx_h).max(1));
        for plane in 0..2usize {
            for tile_row in chunk_tiles(chunk_row, per_chunk_y, chroma_tiles_y) {
                for tile_col in chunk_tiles(chunk_col, per_chunk_x, chroma_tiles_x) {
                    let cx4 = cx4_base + tile_col * chroma_tx_w4;
                    let cy4 = cy4_base + tile_row * chroma_tx_h4;
                    // Spec 5.11.35: a transform block starting outside the frame (in chroma
                    // samples, `startX >= (MiCols * MI_SIZE) >> subX`) is not coded.
                    if cx4 >= frame.mi_cols >> 1 || cy4 >= frame.mi_rows >> 1 {
                        continue;
                    }
                    let txb_skip_ctx = tile_ctx.txb_skip_context_chroma(
                        plane,
                        cx4,
                        cy4,
                        chroma_tx_w4,
                        chroma_tx_h4,
                        not_one_blk,
                    );
                    let dc_sign_ctx = tile_ctx.dc_sign_context_chroma(
                        plane,
                        cx4,
                        cy4,
                        chroma_tx_w4,
                        chroma_tx_h4,
                    );
                    // Intra chroma types come from the chroma mode (all 2D); inter chroma inherits
                    // the luma type at the co-located 4x4 (dav1d reads `txtp_map` there).
                    let class = if cu.is_intra() && !cu.use_intrabc {
                        TxClass1d::TwoD
                    } else {
                        luma_types[colocated_luma_index(
                            tile_col,
                            tile_row,
                            chroma_tx_w4,
                            chroma_tx_h4,
                            mi,
                        )]
                        .chroma_class(chroma_tx_w, chroma_tx_h)
                    };
                    let block = decoder.read_chroma_residual_block(
                        chroma_tx_w,
                        chroma_tx_h,
                        txb_skip_ctx,
                        dc_sign_ctx,
                        class,
                    )?;
                    let cul_level = block.sum_abs_level.min(63) as u8;
                    tile_ctx.set_residual_ctx_chroma(
                        plane,
                        cx4,
                        cy4,
                        chroma_tx_w4,
                        chroma_tx_h4,
                        cul_level,
                        block.dc_sign_value,
                    );
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tile::coding_unit::types::{TxBlock, TxSize};

    fn noise(seed: u64, len: usize) -> Vec<u8> {
        let mut x = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1;
        (0..len)
            .map(|_| {
                x ^= x << 13;
                x ^= x >> 7;
                x ^= x << 17;
                (x >> 24) as u8
            })
            .collect()
    }

    fn rect(x4: u32, y4: u32, w4: u32, h4: u32) -> (BlockRect, MiRect) {
        (
            BlockRect {
                x: x4 * 4,
                y: y4 * 4,
                width: w4 * 4,
                height: h4 * 4,
            },
            MiRect {
                x4,
                y4,
                width: w4,
                height: h4,
            },
        )
    }

    fn frame(key: bool, subsampling: (bool, bool), mono: bool) -> FrameCodingParams {
        let mut f = FrameCodingParams::for_tests();
        f.is_key_frame = key;
        f.tx_type_flags.subsampling_x = subsampling.0;
        f.tx_type_flags.subsampling_y = subsampling.1;
        f.tx_type_flags.mono_chrome = mono;
        f
    }

    fn fingerprint(decoder: &mut SymbolDecoder) -> u32 {
        decoder.read_bools_n(24).unwrap()
    }

    /// The luma reads of spec 5.11.34 written out by hand for a list of `(x4, y4, w_px, h_px)`
    /// transform blocks of an intra block: `all_zero`, then the transform type and the
    /// coefficients when it is not all-zero. With `real_ctx` the neighbour contexts come from
    /// (and are written to) `ctx`, otherwise both are 0.
    fn manual_luma(
        decoder: &mut SymbolDecoder,
        ctx: &mut TileContext,
        blocks: &[(u32, u32, u32, u32)],
        real_ctx: bool,
    ) -> ResidualBlockStats {
        let mut total = ResidualBlockStats::default();
        for &(x4, y4, w, h) in blocks {
            let (w4, h4) = (w / 4, h / 4);
            let (skip_ctx, sign_ctx) = if real_ctx {
                (
                    ctx.txb_skip_context(x4, y4, w4, h4, blocks.len() == 1),
                    ctx.dc_sign_context(x4, y4, w4, h4),
                )
            } else {
                (0, 0)
            };
            let block = if decoder.read_txb_skip(w, h, skip_ctx).unwrap() {
                ResidualBlockStats {
                    all_zero: true,
                    ..Default::default()
                }
            } else {
                let t = decoder
                    .read_transform_type(true, false, false, false, w, h, 0)
                    .unwrap();
                decoder
                    .read_residual_block(w, h, t.class, sign_ctx)
                    .unwrap()
            };
            if real_ctx {
                ctx.set_residual_ctx(
                    x4,
                    y4,
                    w4,
                    h4,
                    block.sum_abs_level.min(63) as u8,
                    block.dc_sign_value,
                );
            }
            total.nonzero_count += block.nonzero_count;
            total.sum_abs_level += block.sum_abs_level;
            total.max_level = total.max_level.max(block.max_level);
        }
        total
    }

    fn same_stats(a: &ResidualBlockStats, b: &ResidualBlockStats) -> bool {
        (a.nonzero_count, a.sum_abs_level, a.max_level)
            == (b.nonzero_count, b.sum_abs_level, b.max_level)
    }

    #[test]
    fn without_tx_blocks_the_block_is_tiled_at_tx_size_and_neighbour_contexts_stay_unused() {
        // An inter-frame intra block with no transform-block list: uniform 16x16 tiles, fixed
        // contexts, and the residual context arrays are never touched.
        let (r, mi) = rect(0, 0, 8, 4); // 32x16 -> two 16x16 transforms
        let f = frame(false, (true, false), false); // no chroma, to look at luma alone
        let mut varied = std::collections::BTreeSet::new();
        for seed in 1..=48u64 {
            let data = noise(seed, 256);
            let mut cu = CodingUnit::new(0, 0, 32, 16);
            cu.tx_size = TxSize::Tx16x16;
            let mut d = SymbolDecoder::new(&data).unwrap();
            let mut ctx = TileContext::new(32, 32);
            read_residual(&mut d, &mut ctx, r, mi, &f, &mut cu, 0).unwrap();

            let mut want_d = SymbolDecoder::new(&data).unwrap();
            let want = manual_luma(
                &mut want_d,
                &mut TileContext::new(32, 32),
                &[(0, 0, 16, 16), (4, 0, 16, 16)],
                false,
            );
            let got = cu.residual.unwrap();
            assert!(same_stats(&got, &want), "seed {seed}: {got:?} vs {want:?}");
            assert_eq!(fingerprint(&mut d), fingerprint(&mut want_d), "seed {seed}");

            let fresh = TileContext::new(32, 32);
            assert_eq!(
                ctx.dc_sign_context(4, 0, 4, 4),
                fresh.dc_sign_context(4, 0, 4, 4)
            );
            assert_eq!(
                ctx.txb_skip_context(4, 0, 4, 4, false),
                fresh.txb_skip_context(4, 0, 4, 4, false)
            );
            varied.insert(got.nonzero_count.min(3));
        }
        assert!(
            varied.len() >= 2,
            "seeds must mix all-zero and coded blocks: {varied:?}"
        );
    }

    #[test]
    fn a_key_frame_block_without_tx_blocks_uses_and_updates_the_real_contexts() {
        let (r, mi) = rect(0, 0, 8, 4);
        let f = frame(true, (true, false), false);
        let mut context_changed = false;
        for seed in 1..=48u64 {
            let data = noise(seed, 256);
            let mut cu = CodingUnit::new(0, 0, 32, 16);
            cu.tx_size = TxSize::Tx16x16;
            let mut d = SymbolDecoder::new(&data).unwrap();
            let mut ctx = TileContext::new(32, 32);
            read_residual(&mut d, &mut ctx, r, mi, &f, &mut cu, 0).unwrap();

            let mut want_d = SymbolDecoder::new(&data).unwrap();
            let mut want_ctx = TileContext::new(32, 32);
            let want = manual_luma(
                &mut want_d,
                &mut want_ctx,
                &[(0, 0, 16, 16), (4, 0, 16, 16)],
                true,
            );
            assert!(same_stats(&cu.residual.unwrap(), &want), "seed {seed}");
            assert_eq!(fingerprint(&mut d), fingerprint(&mut want_d), "seed {seed}");
            for (x4, y4) in [(0, 0), (4, 0), (8, 0), (0, 4)] {
                assert_eq!(
                    ctx.txb_skip_context(x4, y4, 4, 4, false),
                    want_ctx.txb_skip_context(x4, y4, 4, 4, false),
                    "seed {seed} ({x4},{y4})"
                );
                assert_eq!(
                    ctx.dc_sign_context(x4, y4, 4, 4),
                    want_ctx.dc_sign_context(x4, y4, 4, 4)
                );
            }
            context_changed |= ctx.txb_skip_context(4, 0, 4, 4, false)
                != TileContext::new(32, 32).txb_skip_context(4, 0, 4, 4, false);
        }
        assert!(
            context_changed,
            "coded blocks must leave a non-zero cul_level behind"
        );
    }

    #[test]
    fn an_intrabc_block_on_a_key_frame_without_tx_blocks_keeps_the_fixed_contexts() {
        let (r, mi) = rect(0, 0, 4, 4);
        let f = frame(true, (true, false), false);
        let data = noise(5, 256);
        let mut cu = CodingUnit::new(0, 0, 16, 16);
        cu.use_intrabc = true;
        cu.tx_size = TxSize::Tx16x16;
        let mut d = SymbolDecoder::new(&data).unwrap();
        let mut ctx = TileContext::new(32, 32);
        read_residual(&mut d, &mut ctx, r, mi, &f, &mut cu, 0).unwrap();
        let fresh = TileContext::new(32, 32);
        assert_eq!(
            ctx.txb_skip_context(4, 0, 4, 4, false),
            fresh.txb_skip_context(4, 0, 4, 4, false)
        );
    }

    #[test]
    fn a_tx_block_list_overrides_the_uniform_tiling() {
        // 16x16 block with two 8x16 transforms (var-tx style); tx_size would say 16x16.
        let (r, mi) = rect(0, 0, 4, 4);
        let f = frame(false, (true, false), false);
        let data = noise(11, 256);
        let mut cu = CodingUnit::new(0, 0, 16, 16);
        cu.tx_size = TxSize::Tx16x16;
        cu.tx_blocks = Some(vec![
            TxBlock {
                x4: 0,
                y4: 0,
                width_px: 8,
                height_px: 16,
            },
            TxBlock {
                x4: 2,
                y4: 0,
                width_px: 8,
                height_px: 16,
            },
        ]);
        let mut d = SymbolDecoder::new(&data).unwrap();
        let mut ctx = TileContext::new(32, 32);
        read_residual(&mut d, &mut ctx, r, mi, &f, &mut cu, 0).unwrap();

        let mut want_d = SymbolDecoder::new(&data).unwrap();
        // The block is inter-style (a tx list exists): contexts are real.
        let want = manual_luma(
            &mut want_d,
            &mut TileContext::new(32, 32),
            &[(0, 0, 8, 16), (2, 0, 8, 16)],
            true,
        );
        assert!(same_stats(&cu.residual.unwrap(), &want));
        assert_eq!(fingerprint(&mut d), fingerprint(&mut want_d));
    }

    #[test]
    fn chroma_is_read_only_for_420_colour_blocks_that_own_their_chroma() {
        let data = noise(21, 512);
        let luma_only = |r, mi, f: &FrameCodingParams, cu: &CodingUnit| {
            let mut d = SymbolDecoder::new(&data).unwrap();
            let mut ctx = TileContext::new(32, 32);
            let (summary, _) = read_luma_residual(&mut d, &mut ctx, r, mi, f, cu, 0).unwrap();
            (summary, fingerprint(&mut d))
        };
        let full = |r, mi, f: &FrameCodingParams, cu: &mut CodingUnit| {
            let mut d = SymbolDecoder::new(&data).unwrap();
            let mut ctx = TileContext::new(32, 32);
            read_residual(&mut d, &mut ctx, r, mi, f, cu, 0).unwrap();
            fingerprint(&mut d)
        };
        let mut cu = CodingUnit::new(0, 0, 16, 16);
        cu.tx_size = TxSize::Tx16x16;
        let (r, mi) = rect(0, 0, 4, 4);

        let (_, luma_bits) = luma_only(r, mi, &frame(false, (true, true), false), &cu);
        let with_chroma = full(r, mi, &frame(false, (true, true), false), &mut cu);
        assert_ne!(with_chroma, luma_bits, "4:2:0 reads U and V");
        assert_eq!(
            cu.residual.as_ref().unwrap().nonzero_count,
            luma_only(r, mi, &frame(false, (true, true), false), &cu)
                .0
                .nonzero_count,
            "the stored statistics are luma only"
        );

        for (label, f) in [
            (
                "4:2:2 / 4:4:0 (only x subsampled)",
                frame(false, (true, false), false),
            ),
            ("only y subsampled", frame(false, (false, true), false)),
            ("4:4:4", frame(false, (false, false), false)),
            ("monochrome", frame(false, (true, true), true)),
        ] {
            let (_, want) = luma_only(r, mi, &f, &cu);
            assert_eq!(full(r, mi, &f, &mut cu), want, "{label}: no chroma symbols");
        }
    }

    #[test]
    fn a_narrow_block_codes_chroma_only_at_an_odd_position() {
        // 4x4 blocks share a chroma block: only the one at an odd column and row owns it.
        let f = frame(false, (true, true), false);
        let data = noise(31, 512);
        for (x4, y4, owns) in [
            (0, 0, false),
            (1, 0, false),
            (0, 1, false),
            (1, 1, true),
            (3, 5, true),
        ] {
            let (r, mi) = rect(x4, y4, 1, 1);
            let mut cu = CodingUnit::new(x4 * 4, y4 * 4, 4, 4);
            cu.tx_size = TxSize::Tx4x4;
            let mut d = SymbolDecoder::new(&data).unwrap();
            let mut ctx = TileContext::new(32, 32);
            let (_, _) = read_luma_residual(&mut d, &mut ctx, r, mi, &f, &cu, 0).unwrap();
            let luma_bits = fingerprint(&mut d);

            let mut d = SymbolDecoder::new(&data).unwrap();
            let mut ctx = TileContext::new(32, 32);
            read_residual(&mut d, &mut ctx, r, mi, &f, &mut cu, 0).unwrap();
            assert_eq!(fingerprint(&mut d) != luma_bits, owns, "4x4 at ({x4},{y4})");
        }
    }

    #[test]
    fn a_128x128_block_is_read_chunk_by_chunk_with_four_32x32_chroma_transforms_per_plane() {
        // 128x128 luma -> 64x64 chroma, capped at 32x32 per transform: 2x2 tiles in each of U, V.
        let mut f = frame(false, (true, true), false);
        (f.mi_cols, f.mi_rows) = (32, 32);
        let (r, mi) = rect(0, 0, 32, 32);
        for seed in 1..=8u64 {
            let data = noise(seed, 2048);
            let mut cu = CodingUnit::new(0, 0, 128, 128);
            cu.tx_size = TxSize::Tx64x64;
            let mut d = SymbolDecoder::new(&data).unwrap();
            let mut ctx = TileContext::new(64, 64);
            read_residual(&mut d, &mut ctx, r, mi, &f, &mut cu, 0).unwrap();

            // Spec 5.11.34: one 64x64 chunk at a time in raster order, each being its 64x64 luma
            // transform and then its 32x32 U and V transforms.
            let mut want_d = SymbolDecoder::new(&data).unwrap();
            let mut want_ctx = TileContext::new(64, 64);
            for (luma, (cx4, cy4)) in [
                ((0, 0, 64, 64), (0, 0)),
                ((16, 0, 64, 64), (8, 0)),
                ((0, 16, 64, 64), (0, 8)),
                ((16, 16, 64, 64), (8, 8)),
            ] {
                manual_luma(&mut want_d, &mut want_ctx, &[luma], false);
                for plane in 0..2usize {
                    let skip_ctx = want_ctx.txb_skip_context_chroma(plane, cx4, cy4, 8, 8, true);
                    let sign_ctx = want_ctx.dc_sign_context_chroma(plane, cx4, cy4, 8, 8);
                    let b = want_d
                        .read_chroma_residual_block(32, 32, skip_ctx, sign_ctx, TxClass1d::TwoD)
                        .unwrap();
                    want_ctx.set_residual_ctx_chroma(
                        plane,
                        cx4,
                        cy4,
                        8,
                        8,
                        b.sum_abs_level.min(63) as u8,
                        b.dc_sign_value,
                    );
                }
            }
            assert_eq!(fingerprint(&mut d), fingerprint(&mut want_d), "seed {seed}");
        }
    }

    #[test]
    fn chroma_tile_count_decides_the_not_one_block_context() {
        // (luma w, h, luma tx origins in 4x4 units, luma tx px, chroma tile origins, chroma
        // tx w4/h4, not_one_blk)
        type Case = (
            u32,
            u32,
            Vec<(u32, u32, u32, u32)>,
            Vec<(u32, u32)>,
            (u32, u32),
            bool,
        );
        let cases: Vec<Case> = vec![
            // 128x64 -> chroma 64x32: two 32x32 tiles side by side.
            (
                128,
                64,
                vec![(0, 0, 64, 64), (16, 0, 64, 64)],
                vec![(0, 0), (8, 0)],
                (8, 8),
                true,
            ),
            // 64x128 -> chroma 32x64: two stacked.
            (
                64,
                128,
                vec![(0, 0, 64, 64), (0, 16, 64, 64)],
                vec![(0, 0), (0, 8)],
                (8, 8),
                true,
            ),
            // 64x64 -> chroma 32x32: one tile, and one luma transform.
            (64, 64, vec![(0, 0, 64, 64)], vec![(0, 0)], (8, 8), false),
        ];
        let mut f = frame(false, (true, true), false);
        (f.mi_cols, f.mi_rows) = (32, 32);
        for (w, h, luma, chroma, (tw4, th4), not_one) in cases {
            let (r, mi) = rect(0, 0, w / 4, h / 4);
            for seed in 1..=8u64 {
                let data = noise(seed, 2048);
                let mut cu = CodingUnit::new(0, 0, w, h);
                cu.tx_size = TxSize::Tx64x64;
                let mut d = SymbolDecoder::new(&data).unwrap();
                let mut ctx = TileContext::new(64, 64);
                read_residual(&mut d, &mut ctx, r, mi, &f, &mut cu, 0).unwrap();

                let mut want_d = SymbolDecoder::new(&data).unwrap();
                let mut want_ctx = TileContext::new(64, 64);
                // One 64x64 chunk at a time: its luma transform, then its U and V tile.
                for (&luma, &(cx4, cy4)) in luma.iter().zip(&chroma) {
                    manual_luma(&mut want_d, &mut want_ctx, &[luma], false);
                    for plane in 0..2usize {
                        let skip_ctx =
                            want_ctx.txb_skip_context_chroma(plane, cx4, cy4, tw4, th4, not_one);
                        let sign_ctx = want_ctx.dc_sign_context_chroma(plane, cx4, cy4, tw4, th4);
                        let b = want_d
                            .read_chroma_residual_block(32, 32, skip_ctx, sign_ctx, TxClass1d::TwoD)
                            .unwrap();
                        want_ctx.set_residual_ctx_chroma(
                            plane,
                            cx4,
                            cy4,
                            tw4,
                            th4,
                            b.sum_abs_level.min(63) as u8,
                            b.dc_sign_value,
                        );
                    }
                }
                assert_eq!(
                    fingerprint(&mut d),
                    fingerprint(&mut want_d),
                    "{w}x{h} seed {seed}"
                );
            }
        }
    }

    #[test]
    fn a_chroma_transform_starting_outside_the_frame_is_not_coded() {
        // A 128x64 block has a 32x32 chroma transform per 64-wide chunk. In a frame 16 4x4 columns
        // (64 pixels) wide the second chunk starts at chroma column 8 >= 16 / 2: nothing is read.
        let mut f = frame(false, (true, true), false);
        (f.mi_cols, f.mi_rows) = (16, 32);
        let (_, mi) = rect(0, 0, 32, 16);
        let cu = CodingUnit::new(0, 0, 128, 64);
        let luma_types = vec![LumaTxType::DCT; (mi.width * mi.height) as usize];
        let data = noise(7, 256);
        let mut ctx = TileContext::new(64, 64);

        let mut outside = SymbolDecoder::new(&data).unwrap();
        read_chroma_residual(&mut outside, &mut ctx, mi, &f, &cu, &luma_types, (1, 0)).unwrap();
        let untouched = SymbolDecoder::new(&data).unwrap();
        assert_eq!(fingerprint(&mut outside), {
            let mut u = untouched;
            fingerprint(&mut u)
        });

        // The first chunk is inside and does read.
        let mut inside = SymbolDecoder::new(&data).unwrap();
        read_chroma_residual(&mut inside, &mut ctx, mi, &f, &cu, &luma_types, (0, 0)).unwrap();
        let mut fresh = SymbolDecoder::new(&data).unwrap();
        assert_ne!(fingerprint(&mut inside), fingerprint(&mut fresh));
    }

    #[test]
    fn luma_blocks_without_a_tx_list_tile_the_block_row_by_row_at_the_tx_size() {
        let (rect, mi) = rect(2, 3, 4, 2); // 16x8 pixels at 4x4 position (2, 3)
        let mut cu = CodingUnit::new(8, 12, 16, 8);
        cu.tx_size = TxSize::Tx8x8;
        assert_eq!(
            luma_tx_positions(&cu, rect, mi),
            vec![(2, 3, 8, 8), (4, 3, 8, 8)]
        );
        cu.tx_size = TxSize::Tx4x4;
        let positions = luma_tx_positions(&cu, rect, mi);
        assert_eq!(positions.len(), 8);
        assert_eq!(
            &positions[..5],
            &[
                (2, 3, 4, 4),
                (3, 3, 4, 4),
                (4, 3, 4, 4),
                (5, 3, 4, 4),
                (2, 4, 4, 4)
            ]
        );
    }

    #[test]
    fn a_transform_larger_than_the_block_still_gives_one_position() {
        let (rect, mi) = rect(0, 0, 1, 1);
        let mut cu = CodingUnit::new(0, 0, 4, 4);
        cu.tx_size = TxSize::Tx8x8;
        assert_eq!(luma_tx_positions(&cu, rect, mi), vec![(0, 0, 8, 8)]);
    }

    #[test]
    fn a_tx_list_gives_its_own_positions_whatever_the_tx_size() {
        let (rect, mi) = rect(4, 4, 4, 4);
        let mut cu = CodingUnit::new(16, 16, 16, 16);
        cu.tx_blocks = Some(vec![
            TxBlock {
                x4: 4,
                y4: 4,
                width_px: 16,
                height_px: 8,
            },
            TxBlock {
                x4: 4,
                y4: 6,
                width_px: 8,
                height_px: 8,
            },
        ]);
        assert_eq!(
            luma_tx_positions(&cu, rect, mi),
            vec![(4, 4, 16, 8), (4, 6, 8, 8)]
        );
    }

    #[test]
    fn a_lossless_frame_cuts_chroma_into_4x4_transform_blocks() {
        // #388: spec residual() uses TX_4X4 for every plane when Lossless
        let tile = |w4, h4| {
            let t = chroma_tiling(
                MiRect {
                    x4: 0,
                    y4: 0,
                    width: w4,
                    height: h4,
                },
                true,
            );
            (t.tx_w, t.tx_h, t.tiles_x, t.tiles_y)
        };
        assert_eq!(tile(1, 1), (4, 4, 1, 1)); // 4x4 luma
        assert_eq!(tile(4, 4), (4, 4, 2, 2)); // 16x16 luma -> 8x8 chroma
        assert_eq!(tile(16, 16), (4, 4, 8, 8)); // 64x64 luma -> 32x32 chroma
        assert_eq!(tile(32, 32), (4, 4, 16, 16)); // 128x128 luma
        assert_eq!(tile(3, 5), (4, 4, 2, 3)); // odd sizes round up like the normal case
        assert_eq!(tile(32, 8), (4, 4, 16, 4));
    }

    #[test]
    fn each_64x64_chunk_owns_a_run_of_chroma_tiles() {
        // normal case: one 32-wide tile per chunk (or the block's single tile)
        assert_eq!(chunk_tiles(0, 1, 1), 0..1);
        assert_eq!(chunk_tiles(0, 1, 2), 0..1);
        assert_eq!(chunk_tiles(1, 1, 2), 1..2);
        assert!(
            chunk_tiles(1, 1, 1).is_empty(),
            "a chunk the block does not reach"
        );
        // a tile wider than the chunk's share still counts as one
        assert_eq!(chunk_tiles(0, (32 / 16u32).max(1), 1), 0..1);
        // lossless 128x128 luma: 16 tiles per axis, 8 per chunk
        assert_eq!(chunk_tiles(0, 8, 16), 0..8);
        assert_eq!(chunk_tiles(1, 8, 16), 8..16);
        // lossless block smaller than a chunk: only the tiles it has
        assert_eq!(chunk_tiles(0, 8, 3), 0..3);
        assert!(chunk_tiles(1, 8, 3).is_empty());
    }

    #[test]
    fn chroma_is_half_the_luma_block_rounded_up_to_4_and_capped_at_32_per_axis() {
        let tile = |w4, h4| {
            let t = chroma_tiling(
                MiRect {
                    x4: 0,
                    y4: 0,
                    width: w4,
                    height: h4,
                },
                false,
            );
            (t.tx_w, t.tx_h, t.tiles_x, t.tiles_y)
        };
        assert_eq!(tile(1, 1), (4, 4, 1, 1)); // 4x4 luma: still a 4x4 chroma block
        assert_eq!(tile(2, 2), (4, 4, 1, 1));
        assert_eq!(tile(3, 5), (8, 12, 1, 1)); // odd sizes round up
        assert_eq!(tile(16, 16), (32, 32, 1, 1)); // 64x64 luma
        assert_eq!(tile(32, 32), (32, 32, 2, 2)); // 128x128 luma
        assert_eq!(tile(32, 8), (32, 16, 2, 1)); // each axis is capped on its own
    }

    #[test]
    fn a_block_with_several_tx_rows_and_columns_steps_by_the_tx_size_in_both_axes() {
        let (rect, mi) = rect(0, 0, 4, 4); // 16x16 pixels
        let mut cu = CodingUnit::new(0, 0, 16, 16);
        cu.tx_size = TxSize::Tx8x8;
        assert_eq!(
            luma_tx_positions(&cu, rect, mi),
            vec![(0, 0, 8, 8), (2, 0, 8, 8), (0, 2, 8, 8), (2, 2, 8, 8)]
        );
    }

    #[test]
    fn the_colocated_luma_type_of_a_chroma_block_is_found_at_twice_its_offset_and_clamped() {
        let mi = MiRect {
            x4: 0,
            y4: 0,
            width: 32,
            height: 32,
        }; // 128x128 luma
        assert_eq!(colocated_luma_index(0, 0, 8, 8, mi), 0);
        assert_eq!(colocated_luma_index(1, 0, 8, 8, mi), 16);
        assert_eq!(colocated_luma_index(0, 1, 8, 8, mi), 16 * 32);
        assert_eq!(colocated_luma_index(1, 1, 8, 8, mi), 16 * 32 + 16);
        // A narrow block clamps to its last column / row.
        let narrow = MiRect {
            x4: 0,
            y4: 0,
            width: 3,
            height: 5,
        };
        assert_eq!(colocated_luma_index(1, 1, 1, 1, narrow), 2 * 3 + 2);
        assert_eq!(colocated_luma_index(3, 3, 1, 1, narrow), 4 * 3 + 2);
    }
}
