//! Coding Unit Parsing
//!
//! Per AV1 Specification Section 5.11 (Coding Block Syntax)
//!
//! A Coding Unit contains:
//! - Prediction information (INTRA/INTER mode)
//! - Motion vectors (for INTER blocks)
//! - Transform information
//! - Quantization parameters
//! - Residual data
//!
//! ## Implementation Strategy
//!
//! **Phase 1** (Current):
//! - Parse skip flag
//! - Parse prediction mode
//! - Parse reference frames (for INTER)
//!
//! **Phase 2**:
//! - Parse motion vectors
//! - Calculate MV predictors
//! - Reconstruct final MVs
//!
//! **Phase 3**:
//! - Parse transform sizes
//! - Parse quantization info
//! - Parse residuals (optional for visualization)
//!
//! ## Residual reading is not optional -- it was a real desync bug, not a scope choice
//!
//! `parse_coding_unit` used to return immediately after `delta_q`, never reading the AV1 spec's
//! `residual()` syntax element for non-skip coding units. Because a tile's coefficient data is
//! arithmetic-coded with no byte-aligned skip points, this silently desynced the shared
//! `SymbolDecoder`'s position from every subsequent syntax element in the tile as soon as any CU
//! had `skip == false` -- confirmed via a real crash (`get_codec_extended_info` on frame 100 of
//! the real fixture panicked in `ArithmeticDecoder::refill` once the drift exhausted the tile's
//! real bytes). `SymbolDecoder::read_residual_block` (see its own doc for the CDF/context
//! simplifications) now reads a real (if non-spec-exact) residual read sequence for every
//! non-skip CU's transform blocks, closing the gap that caused this.

mod compound;
mod contexts;
mod delta;
mod interp_filter;
mod intra;
mod is_inter;
mod palette;
mod ref_frames;
mod segment;
mod single_ref;
mod skip;
mod types;
mod var_tx;

pub use palette::PaletteInfo;
pub use types::*;

use crate::tile::{BlockRect, FrameCodingParams, MiRect, SuperblockCtx, TileState};

use crate::symbol::{ResidualBlockStats, SymbolDecoder};
use bitvue_engine::Result;

/// Parse coding unit from symbol decoder
///
/// Reads block-level syntax elements from the bitstream.
///
/// # Arguments
///
/// * `state` - The tile's shared mutable state: symbol decoder, MV predictor context and the
///   above/left entropy-context tracker (currently only `skip` uses it) -- see [`TileState`]
/// * `sb` - The enclosing superblock's origin/size and `cdef_idx()` tracker (see
///   [`SuperblockCtx`]); real spec's `delta_q`/`delta_lf` are read only once per superblock
/// * `rect` - Block position and dimensions in pixels
/// * `frame` - Frame-level, read-only flags (see [`FrameCodingParams`] for what each gates)
/// * `current_qp` - Current quantization parameter value
///
/// # Returns
///
/// Parsed coding unit with prediction info, motion vectors (if INTER), and QP value
pub fn parse_coding_unit(
    state: &mut TileState<'_>,
    sb: &mut SuperblockCtx,
    rect: BlockRect,
    frame: &FrameCodingParams,
    current_qp: i16,
) -> Result<(CodingUnit, i16)> {
    let BlockRect {
        x,
        y,
        width,
        height,
    } = rect;
    let TileState {
        decoder,
        mv_ctx,
        tile_ctx,
    } = state;
    let FrameCodingParams {
        is_key_frame,
        allow_intrabc,
        segmentation,
        tx_type_flags,
        inter_mode_flags,
        mi_rows,
        mi_cols,
        cdef_bits,
        skip_mode_present,
        ..
    } = *frame;
    let mut cu = CodingUnit::new(x, y, width, height);
    let (x4, y4) = (x / 4, y / 4);
    let (width_4x4, height_4x4) = (width.div_ceil(4).max(1), height.div_ceil(4).max(1));
    let mi = MiRect {
        x4,
        y4,
        width: width_4x4,
        height: height_4x4,
    };

    // segment_id(), pre-skip position (spec 5.11.9/5.11.10) -- see `segment::read_pre_skip`.
    if let Some(id) = segment::read_pre_skip(decoder, tile_ctx, mi, segmentation)? {
        cu.segment_id = id;
    }

    // skip_mode then skip (spec 5.11.5) -- see `skip`.
    cu.skip_mode = skip::read_skip_mode(decoder, tile_ctx, mi, !is_key_frame && skip_mode_present)?;
    cu.skip = skip::read_skip(
        decoder,
        tile_ctx,
        mi,
        cu.skip_mode,
        segmentation,
        cu.segment_id,
    )?;

    // segment_id(), post-skip position -- see `segment::read_post_skip`.
    if let Some(id) = segment::read_post_skip(decoder, tile_ctx, mi, segmentation, cu.skip)? {
        cu.segment_id = id;
    }

    // cdef_idx() then delta_q/delta_lf (spec 5.11.56, 5.11.38) -- see `delta`.
    delta::read_cdef_idx(decoder, sb, mi, cu.skip, cdef_bits)?;
    let new_qp = delta::read_delta_q_lf(decoder, sb, mi, cu.skip, frame, current_qp);
    cu.qp = Some(new_qp);

    // Raw intra mode symbol (0..=12), captured below for `is_intra` CUs -- only meaningful for
    // `SymbolDecoder::read_transform_type_is_1d`'s `y_mode_raw` param.
    let mut y_mode_raw: u8 = 0;

    // is_inter (spec 5.11.5) -- see `is_inter`.
    let is_inter = is_inter::read_is_inter(
        decoder,
        tile_ctx,
        mi,
        is_key_frame,
        cu.skip_mode,
        segmentation,
        cu.segment_id,
    )?;

    // Determine if INTRA or INTER
    if !is_inter {
        // Real INTRA CU -- either a key-frame CU (the only case before 2026-08-13) or a genuine
        // intra-coded CU within an inter frame (new). Everything below (`y_mode` through the
        // real per-pixel palette-token read and `tx_size()`) is the SAME unified code path real
        // dav1d uses for both cases -- see `SymbolDecoder::read_intra_mode_inter_frame`'s doc for
        // the one real difference (which CDF/context source `y_mode` draws from).
        cu.ref_frames = [RefFrame::Intra, RefFrame::Intra];

        // use_intrabc (spec 5.11.6) -- rare (screen-content-coding), only read at all when the
        // frame header allows it. Real spec: `allow_intrabc` is only ever true for an intra-only
        // frame's own header (never for a genuine inter frame), so gating on `is_key_frame` here
        // too is redundant with a well-formed `allow_intrabc` but kept explicit rather than
        // assumed.
        cu.use_intrabc = if is_key_frame && allow_intrabc {
            decoder.read_use_intrabc()?
        } else {
            false
        };

        // tx_size() (spec 5.11.15/16) -- real per-context CDF + adaptation, see
        // `SymbolDecoder::read_tx_size`'s doc. IntraBC is excluded from *this* single-size read:
        // real spec's `read_block_tx_size()` gates the recursive `read_var_tx_size()` tree on
        // `is_inter`, and dav1d's own block-mode dispatch (`b->intra = !intrabc_flag`, verified
        // directly against `src/decode.c`, not assumed) confirms IntraBC blocks are classified
        // `is_inter` for this purpose despite being coded within an intra frame -- real
        // `read_vartx_tree` is called for them identically to real inter blocks (`compute_inter_
        // tx_blocks`, below), not this heuristic-single-size path. The ENTIRE `b->intra` mode-info
        // tail below (`y_mode` through the real per-pixel palette-token read) is likewise excluded
        // for IntraBC: real dav1d dispatches `b->intra = !intrabc_flag`, so a true `use_intrabc`
        // flag makes `b->intra == 0` and skips this whole block -- verified directly against
        // `src/decode.c`'s `if (b->intra) { ... }` wrapper (2026-08-13, found while implementing
        // palette: this crate previously read `y_mode` here UNCONDITIONALLY, a real desync bug on
        // every IntraBC CU that predates this fix).
        if !cu.use_intrabc {
            y_mode_raw = intra::read_intra_mode_info(decoder, tile_ctx, rect, mi, frame, &mut cu)?;
        } else {
            // Real-fixture-verified (2026-08-12): this crate's only committed fixture
            // (`test_data/av1_test.ivf`) has zero `use_intrabc` CUs, so this path was verified
            // separately against a real screen-content encode (official libaom test asset
            // `screendata.y4m`, `storage.googleapis.com/aom-test-data`, encoded locally with
            // `aomenc --tune-content=screen --enable-intrabc=1` -- scratchpad-only, never
            // committed, per this repo's third-party-test-data policy) -- 10 real IntraBC CUs
            // observed, 100% got a real `tx_blocks` breakdown, zero parse errors across the
            // clip. See `DEVELOPMENT_PHASES.md` for the full verification record.
            cu.tx_blocks = var_tx::compute_inter_tx_blocks(
                decoder,
                tile_ctx,
                x,
                y,
                width,
                height,
                cu.skip,
                tx_type_flags.coded_lossless,
                tx_type_flags.txfm_mode,
                mi_rows,
                mi_cols,
            )?;
        }
    } else {
        // ref_frame() (spec 5.11.25) -- see `ref_frames`.
        cu.ref_frames = ref_frames::read_ref_frames(decoder, tile_ctx, rect, mi, frame, &cu)?;
        let is_compound = cu.ref_frames[1] != RefFrame::Intra;
        let rav1d_ref0 = cu.ref_frames[0] as i8 - 1;

        if is_compound {
            compound::read_compound_mode_info(decoder, tile_ctx, mv_ctx, rect, mi, frame, &mut cu)?;
        } else {
            // INTER frame - read prediction mode
            single_ref::read_single_ref_mode_info(
                decoder, tile_ctx, mv_ctx, rect, mi, frame, &mut cu,
            )?;
        }

        // filter (spec 5.11.30) -- see `interp_filter`.
        interp_filter::read_interp_filter(decoder, tile_ctx, mi, frame, &cu)?;

        // read_block_tx_size() (spec 5.11.16/17/18) for INTER blocks -- real recursive var-tx
        // read, see `read_var_tx_size`'s doc for the exact scope (square CUs only) and why.
        cu.tx_blocks = var_tx::compute_inter_tx_blocks(
            decoder,
            tile_ctx,
            x,
            y,
            width,
            height,
            cu.skip,
            tx_type_flags.coded_lossless,
            tx_type_flags.txfm_mode,
            mi_rows,
            mi_cols,
        )?;
    }

    // Add this CU to the MV predictor context for future blocks
    // Now uses zero-copy reference instead of cloning the entire CU
    mv_ctx.add_cu(&cu);

    // Read residual() for every transform block tiling this CU -- required for correct bitstream
    // alignment whenever skip == false, not just for producing residual statistics. See this
    // module's doc and `SymbolDecoder::read_residual_block`'s doc.
    if !cu.skip {
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
                    (0..tx_cols).map(move |tx_col| {
                        (x4 + tx_col * tx_wh4, y4 + tx_row * tx_wh4, tx_px, tx_px)
                    })
                })
                .collect()
        };
        // Real `txb_skip`/`dc_sign` neighbor context is only trustworthy where transform-block
        // boundaries are real (regular key-frame intra via `tx_size()`, or inter/IntraBC via real
        // `tx_blocks` -- both real bitstream-derived boundaries); other CUs keep the
        // fixed-context-0 fallback and never touch `tile_ctx`'s residual arrays, matching
        // `SymbolDecoder::read_residual_block`'s doc.
        let use_real_residual_ctx = (is_key_frame && !cu.use_intrabc) || cu.tx_blocks.is_some();
        let is_single_tx_block = tx_positions.len() == 1;
        let mut summary = ResidualBlockStats::default();
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
            let all_zero = decoder.read_txb_skip(tx_w_px.max(tx_h_px), txb_skip_ctx)?;
            let block = if all_zero {
                ResidualBlockStats {
                    all_zero: true,
                    ..Default::default()
                }
            } else {
                let tx_class_1d = decoder.read_transform_type_is_1d(
                    is_key_frame,
                    tx_type_flags.coded_lossless,
                    tx_type_flags.qidx_is_zero,
                    tx_type_flags.reduced_tx_set,
                    tx_w_px.max(tx_h_px),
                    y_mode_raw,
                )?;
                decoder.read_residual_block(tx_w_px, tx_h_px, tx_class_1d, dc_sign_ctx)?
            };

            if use_real_residual_ctx {
                let cul_level = block.sum_abs_level.min(63) as u8;
                tile_ctx.set_residual_ctx(
                    tx_x4,
                    tx_y4,
                    tx_w4,
                    tx_h4,
                    cul_level,
                    block.dc_sign_value,
                );
            }

            summary.nonzero_count += block.nonzero_count;
            summary.sum_abs_level += block.sum_abs_level;
            summary.max_level = summary.max_level.max(block.max_level);
        }

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
        if !cu.use_intrabc
            && !tx_type_flags.mono_chrome
            && tx_type_flags.subsampling_x
            && tx_type_flags.subsampling_y
            && (8..=128).contains(&width)
            && (8..=128).contains(&height)
        {
            let (chroma_w, chroma_h) = (width / 2, height / 2);
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
                        let block = decoder.read_chroma_residual_block(
                            chroma_tx_w,
                            chroma_tx_h,
                            txb_skip_ctx,
                            dc_sign_ctx,
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

        cu.residual = Some(summary);
    } else {
        cu.residual = None;
    }

    Ok((cu, new_qp))
}

/// Read one explicit MV delta (horizontal + vertical component) from the bitstream, per AV1 spec
/// 5.11.32 `read_mv(ref)`. Used for every `MvKind::New` reference-list slot -- single-ref
/// `NewMv`'s L0, and compound modes' L0 and/or L1 (spec 5.11.26 `assign_mv()`).
///
/// The `mv_joint` symbol gates which axis actually has a coded component -- an axis mv_joint
/// marks "zero" is NOT read from the bitstream at all (it's implicitly 0), it doesn't just
/// happen to decode to a small value. The previous implementation unconditionally read both
/// components for every MV, which desynced the shared `SymbolDecoder` against any real
/// bitstream whenever mv_joint indicated a zero axis -- the same "syntax element not read at
/// all" pattern as this session's earlier residual()/ref_frame() bugs, just not crash-visible
/// here since a `SymbolDecoder` never panics on merely-wrong-but-in-range values.
fn read_explicit_mv(decoder: &mut SymbolDecoder) -> Result<MotionVector> {
    let joint = decoder.read_mv_joint()?;
    // MV_JOINT_HZVNZ(2)/MV_JOINT_HNZVNZ(3): vertical component is non-zero, read it (spec reads
    // diffMv[0], the row/vertical component, first).
    let mv_y = if matches!(joint, 2 | 3) {
        decoder.read_mv_component()?
    } else {
        0
    };
    // MV_JOINT_HNZVZ(1)/MV_JOINT_HNZVNZ(3): horizontal component is non-zero, read it.
    let mv_x = if matches!(joint, 1 | 3) {
        decoder.read_mv_component()?
    } else {
        0
    };
    Ok(MotionVector::new(mv_x, mv_y))
}
