//! `residual()` (spec 5.11.34) for a non-skipped block: luma transform blocks first, then U, then
//! V. Reading every transform block is required for bitstream alignment, not just for producing
//! residual statistics -- see `SymbolDecoder::read_residual_block`'s doc.

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
    let (summary, luma_types) =
        read_luma_residual(decoder, tile_ctx, rect, mi, frame, cu, y_mode_raw)?;
    read_chroma_residual(decoder, tile_ctx, mi, frame, cu, &luma_types)?;
    cu.residual = Some(summary);
    Ok(())
}

fn read_luma_residual(
    decoder: &mut SymbolDecoder,
    tile_ctx: &mut TileContext,
    rect: BlockRect,
    mi: MiRect,
    frame: &FrameCodingParams,
    cu: &CodingUnit,
    y_mode_raw: u8,
) -> Result<(ResidualBlockStats, Vec<LumaTxType>)> {
    let BlockRect { width, height, .. } = rect;
    let MiRect { x4, y4, .. } = mi;
    let FrameCodingParams {
        is_key_frame,
        tx_type_flags,
        ..
    } = *frame;
    // Real `tx_blocks` (var-tx, inter only -- see `compute_inter_tx_blocks`'s doc) gives the
    // true per-leaf positions/sizes directly, including genuinely rectangular leaves;
    // everything else still tiles uniformly at `cu.tx_size` (a heuristic for those CUs, not a
    // real bitstream-derived size, still square-only).
    let tx_positions: Vec<(u32, u32, u32, u32)> = if let Some(blocks) = &cu.tx_blocks {
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
    let is_single_tx_block = tx_positions.len() == 1;
    let mut summary = ResidualBlockStats::default();
    // Luma transform type at every 4x4 of the block (dav1d `txtp_map`), which an inter block's
    // chroma transforms inherit from.
    let mut luma_types = vec![LumaTxType::DCT; (mi.width * mi.height) as usize];
    for (tx_x4, tx_y4, tx_w_px, tx_h_px) in tx_positions {
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
    Ok((summary, luma_types))
}

fn read_chroma_residual(
    decoder: &mut SymbolDecoder,
    tile_ctx: &mut TileContext,
    mi: MiRect,
    frame: &FrameCodingParams,
    cu: &CodingUnit,
    luma_types: &[LumaTxType],
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
        // The chroma block of an 8x8-or-larger luma block is half its size; a 4-wide/tall block
        // (which shares its chroma with its neighbours) still has a 4-sample chroma side.
        let (chroma_w, chroma_h) = (((mi.width + 1) >> 1) * 4, ((mi.height + 1) >> 1) * 4);
        let (chroma_tx_w, chroma_tx_h) = (chroma_w.min(32), chroma_h.min(32));
        let (chroma_tx_w4, chroma_tx_h4) = (chroma_tx_w / 4, chroma_tx_h / 4);
        let chroma_tiles_x = chroma_w.div_ceil(chroma_tx_w).max(1);
        let chroma_tiles_y = chroma_h.div_ceil(chroma_tx_h).max(1);
        let not_one_blk = chroma_tiles_x * chroma_tiles_y > 1;
        let (cx4_base, cy4_base) = (x4 / 2, y4 / 2);
        for plane in 0..2usize {
            for tile_row in 0..chroma_tiles_y {
                for tile_col in 0..chroma_tiles_x {
                    let cx4 = cx4_base + tile_col * chroma_tx_w4;
                    let cy4 = cy4_base + tile_row * chroma_tx_h4;
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
                        let lx = (tile_col * chroma_tx_w4 * 2).min(mi.width.saturating_sub(1));
                        let ly = (tile_row * chroma_tx_h4 * 2).min(mi.height.saturating_sub(1));
                        luma_types[(ly * mi.width + lx) as usize]
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
