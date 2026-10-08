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

mod contexts;
mod delta;
mod intra;
mod is_inter;
mod palette;
mod segment;
mod skip;
mod types;

pub use palette::PaletteInfo;
pub use types::*;

use crate::tile::{BlockRect, FrameCodingParams, MiRect, SuperblockCtx, TileState};

use crate::symbol::cdf::tx_size_class;
use crate::symbol::{ResidualBlockStats, SymbolDecoder};
use bitvue_engine::Result;
use contexts::{
    compound_mode_from_symbol, global_motion_forces_simple, has_overlappable_neighbors,
    inter_mode_from_symbol, motion_mode_size_index, needs_interp_filter, wedge_ctx,
    y_mode_size_context,
};

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
        reference_select,
        allow_intrabc,
        use_ref_frame_mvs,
        segmentation,
        tx_type_flags,
        inter_mode_flags,
        mi_rows,
        mi_cols,
        cdef_bits,
        skip_mode_present,
        skip_mode_refs,
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
            cu.tx_blocks = compute_inter_tx_blocks(
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
        // ref_frame() (spec 5.11.25) -- real per-context CDF + adaptation, see
        // `SymbolDecoder::read_ref_frames`'s doc. Real spec priority order (dav1d `decode.c:1401`
        // vs `1424`): `skip_mode` beats everything else, forcing `RefFrame` from the real
        // `skip_mode_refs` (spec's `SkipModeFrame[0]/[1]`, `read_skip_mode_params`'s doc) with no
        // bits read -- always a genuine compound pair (`skip_mode` can only be true when
        // `skip_mode_present`, which itself requires deriving 2 distinct refs, `read_skip_mode_
        // params`'s doc). Segmentation's two real overrides come next when not `skip_mode`:
        // `SEG_LVL_REF_FRAME` forces `RefFrame[0]` from its `FeatureData` (`RefFrame[1] = Intra`,
        // i.e. never compound); when that's inactive, `SEG_LVL_SKIP` or `SEG_LVL_GLOBALMV`
        // (either one) forces `RefFrame[0] = Last`, `RefFrame[1] = Intra` (real spec: same
        // `LAST_FRAME` fallback for both).
        let seg_ref_frame_feature = crate::frame_header_full::SEG_LVL_REF_FRAME;
        cu.ref_frames = if cu.skip_mode {
            [
                RefFrame::from_u8(skip_mode_refs[0]).unwrap_or(RefFrame::Last),
                RefFrame::from_u8(skip_mode_refs[1]).unwrap_or(RefFrame::Intra),
            ]
        } else if segmentation.seg_feature_active(cu.segment_id, seg_ref_frame_feature) {
            let raw = segmentation.seg_feature_data(cu.segment_id, seg_ref_frame_feature);
            [
                RefFrame::from_u8(raw.clamp(0, 7) as u8).unwrap_or(RefFrame::Last),
                RefFrame::Intra,
            ]
        } else if segmentation
            .seg_feature_active(cu.segment_id, crate::frame_header_full::SEG_LVL_SKIP)
            || segmentation
                .seg_feature_active(cu.segment_id, crate::frame_header_full::SEG_LVL_GLOBALMV)
        {
            [RefFrame::Last, RefFrame::Intra]
        } else {
            decoder.read_ref_frames(tile_ctx, x4, y4, reference_select, width.min(height))?
        };
        let is_compound = cu.ref_frames[1] != RefFrame::Intra;
        tile_ctx.set_ref_frames(
            x4,
            y4,
            width_4x4,
            height_4x4,
            false, // real inter CU -- this branch is only reached when `is_inter` (see above)
            is_compound,
            cu.ref_frames[0] as i8 - 1,
            if is_compound {
                cu.ref_frames[1] as i8 - 1
            } else {
                -1
            },
        );
        let rav1d_ref0 = cu.ref_frames[0] as i8 - 1;
        let rav1d_ref1 = if is_compound {
            cu.ref_frames[1] as i8 - 1
        } else {
            -1
        };

        if is_compound {
            // skip_mode (spec 5.11.5/5.11.24, dav1d `decode.c:1401-1423`) forces the ENTIRE
            // mode-info tail with zero bits read: `inter_mode = NEARESTMV_NEARESTMV`, `drl_idx =
            // NEAREST` (index 0, no DRL bits), `mv[]` straight from `compound_mv_stack`'s
            // `stack[0]`, `comp_type = AVG`. Real spec never reads `compound_mode()`/DRL/MV-
            // residual/`compound_type()` bits for a skip_mode CU at all -- this crate previously
            // (before this branch existed) fell through to the real-read path below even for
            // skip_mode CUs, a real desync (reading bits a real encoder never wrote).
            let comp_type = if cu.skip_mode {
                cu.mode = PredictionMode::NearestNearestMv;
                let (stack, _n_mvs) = tile_ctx.compound_mv_stack(
                    x4,
                    y4,
                    width_4x4,
                    height_4x4,
                    rav1d_ref0,
                    rav1d_ref1,
                    use_ref_frame_mvs,
                );
                cu.mv = stack[0].mv;
                2 // AVG, matches dav1d's `b->comp_type = COMP_INTER_AVG`
            } else {
                // compound_mode() (spec 5.11.24) -- a distinct 8-symbol alphabet from the
                // single-ref 4-way `inter_mode`, see `SymbolDecoder::read_compound_mode`'s doc.
                let ctx = tile_ctx
                    .compound_mode_context(x4, y4, width_4x4, height_4x4, rav1d_ref0, rav1d_ref1);
                let mode_symbol = decoder.read_compound_mode(ctx)?;
                cu.mode = compound_mode_from_symbol(mode_symbol)?;

                // Real compound DRL (spec 7.10.2.10, joint L0/L1 candidate stack) -- see
                // `SpatialRefContext::compound_mv_stack`'s doc for the real weighted
                // spatial+temporal search this replaces (previously: independent, zero-fallback
                // per-direction lookups via `MvPredictorContext`, and no DRL bits read at all --
                // a real desync for any compound CU whose candidate list has more than 1 entry,
                // not a rare edge case). Real spec's exact 3-way branch (dav1d
                // `decode.c:1502-1532`): `NewNewMv` reads up to 2 bits from `stack[0]`/`[1]`;
                // else if either direction is `Near`, drl starts at index 1 with up to 1 more
                // bit from `stack[1]`; otherwise (`NearestNearestMv`/`GlobalGlobalMv`/any
                // Nearest+New/Nearest+Global/etc. combination not involving `Near`) index 0, no
                // bits at all.
                let (stack, n_mvs) = tile_ctx.compound_mv_stack(
                    x4,
                    y4,
                    width_4x4,
                    height_4x4,
                    rav1d_ref0,
                    rav1d_ref1,
                    use_ref_frame_mvs,
                );
                let l0_kind = cu.mode.l0_mv_kind();
                let l1_kind = cu.mode.l1_mv_kind();
                let mut drl_idx = 0usize;
                if l0_kind == Some(MvKind::New) && l1_kind == Some(MvKind::New) {
                    if n_mvs > 1 {
                        if decoder.read_drl_bit(crate::tile::context::get_compound_drl_context(
                            &stack, 0,
                        ))? {
                            drl_idx += 1;
                        }
                        if drl_idx == 1
                            && n_mvs > 2
                            && decoder.read_drl_bit(
                                crate::tile::context::get_compound_drl_context(&stack, 1),
                            )?
                        {
                            drl_idx += 1;
                        }
                    }
                } else if l0_kind == Some(MvKind::Near) || l1_kind == Some(MvKind::Near) {
                    drl_idx = 1;
                    if n_mvs > 2
                        && decoder.read_drl_bit(crate::tile::context::get_compound_drl_context(
                            &stack, 1,
                        ))?
                    {
                        drl_idx += 1;
                    }
                }

                // Per-direction value: `Nearest`/`Near` read straight from the real stack;
                // `New` uses `stack[drl_idx]` as predictor with the explicit residual added on
                // top (unchanged shape from the single-direction version this replaces);
                // `Global` keeps today's `mv_ctx`-sourced zero approximation unchanged (real
                // `gm_params` values still aren't stored, same already-documented gap as
                // single-ref `GlobalMv` below).
                cu.mv[0] = match l0_kind {
                    Some(MvKind::New) => {
                        let explicit_mv = read_explicit_mv(decoder)?;
                        explicit_mv.add(stack[drl_idx].mv[0])
                    }
                    Some(MvKind::Nearest) | Some(MvKind::Near) => stack[drl_idx].mv[0],
                    Some(MvKind::Global) => {
                        mv_ctx.get_mv_predictor(cu.mode, x, y, cu.ref_frames[0])
                    }
                    None => MotionVector::zero(),
                };
                cu.mv[1] = match l1_kind {
                    Some(MvKind::New) => {
                        let explicit_mv = read_explicit_mv(decoder)?;
                        explicit_mv.add(stack[drl_idx].mv[1])
                    }
                    Some(MvKind::Nearest) | Some(MvKind::Near) => stack[drl_idx].mv[1],
                    Some(MvKind::Global) => {
                        mv_ctx.get_mv_predictor_l1(cu.mode, x, y, cu.ref_frames[1])
                    }
                    None => MotionVector::zero(),
                };

                // compound_type() (spec 5.11.28: jnt_comp vs. segmentation-mask vs. wedge-mask)
                // -- real per-context CDF + adaptation, see `SymbolDecoder::read_mask_comp`'s
                // doc for the desync this closes (previously never read at all for ANY compound
                // block).
                if inter_mode_flags.enable_masked_compound
                    && decoder.read_mask_comp(tile_ctx.mask_comp_context(x4, y4))?
                {
                    // seg/wedge branch
                    if let Some(wctx) = wedge_ctx(width_4x4, height_4x4) {
                        let is_wedge = decoder.read_wedge_comp(wctx)?;
                        if is_wedge {
                            decoder.read_wedge_idx(wctx)?;
                        }
                        decoder.read_bool_equi()?; // mask_sign
                        if is_wedge {
                            4
                        } else {
                            3
                        }
                    } else {
                        decoder.read_bool_equi()?; // mask_sign
                        3 // SEG (no wedge eligible at this size)
                    }
                } else if inter_mode_flags.enable_jnt_comp {
                    let jctx = tile_ctx.jnt_comp_context(x4, y4);
                    1 + u8::from(decoder.read_jnt_comp(jctx)?)
                } else {
                    2 // AVG
                }
            };

            tile_ctx.set_spatial_ref_block(
                x4,
                y4,
                width_4x4,
                height_4x4,
                rav1d_ref0,
                rav1d_ref1,
                cu.mode.l0_mv_kind() == Some(MvKind::New)
                    || cu.mode.l1_mv_kind() == Some(MvKind::New),
                cu.mv[0],
                cu.mv[1],
            );
            tile_ctx.set_comp_type(x4, y4, width_4x4, height_4x4, comp_type);

            tracing::debug!(
                "Compound mode {:?} at ({}, {}): mv0={:?}, mv1={:?}",
                cu.mode,
                x,
                y,
                cu.mv[0],
                cu.mv[1]
            );
        } else {
            // INTER frame - read prediction mode
            let ctx = tile_ctx.inter_mode_context(
                x4,
                y4,
                width_4x4,
                height_4x4,
                rav1d_ref0,
                use_ref_frame_mvs,
            );
            let mode_symbol = decoder.read_inter_mode(ctx)?;
            cu.mode = inter_mode_from_symbol(mode_symbol)?;

            // DRL (spec 7.10.2.10's real `drl_idx` selection, single-ref only) -- real per-context
            // CDF + adaptation, see `SymbolDecoder::read_drl_bit`'s doc for the desync this closes
            // (this crate previously never read any DRL bits at all, always implicitly using
            // index 0 -- `MvPredictorContext::predict_nearest_mv`'s single-neighbor heuristic).
            // Not read for GLOBALMV (real spec: no DRL for that mode at all).
            if cu.mode == PredictionMode::NewMv {
                let explicit_mv = read_explicit_mv(decoder)?;
                let (stack, n_mvs) = tile_ctx.single_ref_mv_stack(
                    x4,
                    y4,
                    width_4x4,
                    height_4x4,
                    rav1d_ref0,
                    use_ref_frame_mvs,
                );
                let mut drl_idx = 0usize;
                if n_mvs > 1 {
                    if decoder.read_drl_bit(crate::tile::context::get_drl_context(&stack, 0))? {
                        drl_idx += 1;
                    }
                    if drl_idx == 1
                        && n_mvs > 2
                        && decoder.read_drl_bit(crate::tile::context::get_drl_context(&stack, 1))?
                    {
                        drl_idx += 1;
                    }
                }
                let predictor = stack[drl_idx].mv;
                cu.mv[0] =
                    MotionVector::new(explicit_mv.x + predictor.x, explicit_mv.y + predictor.y);
                cu.mv[1] = MotionVector::zero();

                tracing::debug!(
                    "NEWMV at ({}, {}): explicit=({:?}), predictor=({:?}), final=({:?})",
                    x,
                    y,
                    explicit_mv,
                    predictor,
                    cu.mv[0]
                );
            } else if cu.mode == PredictionMode::GlobalMv {
                let predictor = mv_ctx.get_mv_predictor(cu.mode, x, y, cu.ref_frames[0]);
                cu.mv = [predictor, MotionVector::zero()];

                tracing::debug!(
                    "Mode {:?} at ({}, {}): using predictor {:?}",
                    cu.mode,
                    x,
                    y,
                    cu.mv[0]
                );
            } else {
                // NEARESTMV / NEARMV
                let (stack, n_mvs) = tile_ctx.single_ref_mv_stack(
                    x4,
                    y4,
                    width_4x4,
                    height_4x4,
                    rav1d_ref0,
                    use_ref_frame_mvs,
                );
                let mut drl_idx = if cu.mode == PredictionMode::NearMv {
                    1usize
                } else {
                    0
                };
                if cu.mode == PredictionMode::NearMv && n_mvs > 2 {
                    if decoder.read_drl_bit(crate::tile::context::get_drl_context(&stack, 1))? {
                        drl_idx += 1;
                    }
                    if drl_idx == 2
                        && n_mvs > 3
                        && decoder.read_drl_bit(crate::tile::context::get_drl_context(&stack, 2))?
                    {
                        drl_idx += 1;
                    }
                }
                cu.mv = [stack[drl_idx].mv, MotionVector::zero()];

                tracing::debug!(
                    "Mode {:?} at ({}, {}): drl_idx={} mv={:?}",
                    cu.mode,
                    x,
                    y,
                    drl_idx,
                    cu.mv[0]
                );
            }

            tile_ctx.set_spatial_ref_block(
                x4,
                y4,
                width_4x4,
                height_4x4,
                rav1d_ref0,
                rav1d_ref1,
                cu.mode == PredictionMode::NewMv,
                cu.mv[0],
                cu.mv[1],
            );

            // interintra (spec 5.11.29) -- real per-context CDF + adaptation, see
            // `SymbolDecoder::read_interintra`'s doc for the desync this closes (previously never
            // read at all for any single-ref inter block).
            let ii_sz_grp = y_mode_size_context(width_4x4, height_4x4);
            let interintra_wedge_ctx = wedge_ctx(width_4x4, height_4x4).filter(|&c| c <= 6);
            let is_interintra = inter_mode_flags.enable_interintra_compound
                && interintra_wedge_ctx.is_some()
                && decoder.read_interintra(ii_sz_grp)?;
            if is_interintra {
                decoder.read_interintra_mode(ii_sz_grp)?;
                // `interintra_wedge_ctx` is real here (`is_interintra` only true when `Some`).
                let wctx = interintra_wedge_ctx.unwrap_or(0);
                if decoder.read_interintra_wedge(wctx)? {
                    decoder.read_wedge_idx(wctx)?;
                }
            }

            // motion_mode (spec 5.11.27) -- real per-exact-block-size CDF + adaptation, see
            // `SymbolDecoder::read_motion_mode`'s doc for the desync this closes (previously
            // never read at all). Real spec gate: switchable, not interintra, both dims >= 8px,
            // not an excluded warped-global-motion case (real: a `GLOBALMV` block -- the only
            // global-motion mode reachable in this single-ref branch, `GLOBAL_GLOBALMV` being
            // compound-only -- whose `GmType[RefFrame[0]]` is more complex than TRANSLATION reads
            // zero bits here, matching real spec's forced `motion_mode = SIMPLE`; `!force_integer_
            // mv` gates the whole check per spec, same as `read_motion_mode`'s other real gates),
            // and has a real overlappable (non-intra) above/left neighbor.
            let have_top = y4 > 0;
            let have_left = x4 > 0;
            let gm_forces_simple = global_motion_forces_simple(
                cu.mode,
                inter_mode_flags.force_integer_mv,
                &inter_mode_flags.gm_type,
                cu.ref_frames[0],
            );
            if inter_mode_flags.switchable_motion_mode
                && !is_interintra
                && !gm_forces_simple
                && width_4x4 >= 2
                && height_4x4 >= 2
                && has_overlappable_neighbors(tile_ctx, x4, y4, width_4x4, height_4x4)
            {
                // `allow_warp`: real spec also requires a real matching-single-reference above/
                // left neighbor (`find_matching_ref`) -- approximated via
                // `has_matching_single_ref` (single-position check, not the full multi-neighbor
                // edge scan real dav1d does -- `TileContext::has_matching_single_ref`'s doc for
                // why a full port is deferred). SVC reference scaling isn't modeled (assumed
                // never scaled, matching this crate's existing no-SVC-support scope).
                let allow_warp = inter_mode_flags.allow_warped_motion
                    && tile_ctx.has_matching_single_ref(x4, y4, have_top, have_left, rav1d_ref0);
                if allow_warp {
                    if let Some(idx) = motion_mode_size_index(width_4x4, height_4x4) {
                        decoder.read_motion_mode(idx)?;
                    }
                } else if let Some(idx) = motion_mode_size_index(width_4x4, height_4x4) {
                    decoder.read_obmc(idx)?;
                }
            }
        }

        // filter (spec 5.11.30, subpel interpolation filter -- one symbol per axis) -- real
        // per-`(dir, ctx)` CDF + adaptation, see `SymbolDecoder::read_filter`'s doc for the
        // desync this closes (previously never read at all). Real spec's `needs_interp_filter()`
        // exclusion is modeled for both real cases now: `skip_mode` forces `false`
        // unconditionally (dav1d `decode.c:1407`, `has_subpel_filter = 0`, checked first since
        // `cu.mode` is always `NearestNearestMv` there -- `needs_interp_filter`'s own `_ => true`
        // catch-all would otherwise wrongly read bits for it), and the `GmType`-dependent
        // GLOBALMV/GLOBAL_GLOBALMV case (verified against dav1d's `decode.c` `has_subpel_filter`
        // computation, source-only re-clone): unconditionally read for NEARESTMV/NEARMV/NEWMV
        // and any compound mode other than GLOBAL_GLOBALMV; for GLOBALMV/GLOBAL_GLOBALMV, only
        // read when the block is minimal size (`min(width_4x4, height_4x4) == 1`) or the
        // relevant ref's `GmType` is exactly TRANSLATION (not `>` -- IDENTITY/ROTZOOM/AFFINE all
        // suppress the read).
        let has_subpel_filter = !cu.skip_mode
            && needs_interp_filter(
                cu.mode,
                width_4x4,
                height_4x4,
                &inter_mode_flags.gm_type,
                cu.ref_frames[0],
                cu.ref_frames[1],
            );
        if inter_mode_flags.subpel_filter_switchable {
            let is_comp = is_compound;
            for dir in 0..2u8 {
                // Real dav1d always records the resulting filter into neighbor context
                // regardless of whether it was actually read -- `0` (`EIGHTTAP_REGULAR`) is the
                // real default `read_filter` never returns via the CDF path (its symbols start
                // at the crate's own regular-tap index), matching dav1d's own
                // `filter[i] = DAV1D_FILTER_8TAP_REGULAR` fallback.
                let filter = if has_subpel_filter {
                    let fctx = tile_ctx.filter_context(x4, y4, is_comp, dir as usize, rav1d_ref0);
                    decoder.read_filter(dir, fctx)?
                } else {
                    0
                };
                tile_ctx.set_filter(x4, y4, width_4x4, height_4x4, dir as usize, filter);
            }
        }

        // read_block_tx_size() (spec 5.11.16/17/18) for INTER blocks -- real recursive var-tx
        // read, see `read_var_tx_size`'s doc for the exact scope (square CUs only) and why.
        cu.tx_blocks = compute_inter_tx_blocks(
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
fn compute_inter_tx_blocks(
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
