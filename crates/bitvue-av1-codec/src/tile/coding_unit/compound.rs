//! Mode info of a compound-prediction inter block (two references): `compound_mode()`, the joint
//! L0/L1 DRL search, per-direction motion vectors, and `compound_type()` (spec 5.11.24-5.11.28).
//! A `skip_mode` block reads none of it -- the whole tail is derived with zero bits.

use super::contexts::wedge_ctx;
use super::read_explicit_mv;
use super::types::{CodingUnit, MotionVector, MvKind, PredictionMode};
use crate::symbol::SymbolDecoder;
use crate::tile::context::CompoundMvStackEntry;
use crate::tile::{BlockRect, FrameCodingParams, MiRect, TileContext};
use bitvue_engine::{BitvueError, Result};

/// Fills `cu.mode` and `cu.mv[0..2]`, reads the compound-type symbols, and records the block in
/// the spatial reference context. `cu.ref_frames` must already hold the compound pair.
pub(super) fn read_compound_mode_info(
    decoder: &mut SymbolDecoder,
    tile_ctx: &mut TileContext,
    rect: BlockRect,
    mi: MiRect,
    frame: &FrameCodingParams,
    cu: &mut CodingUnit,
) -> Result<()> {
    let BlockRect { x, y, .. } = rect;
    let MiRect {
        x4,
        y4,
        width: width_4x4,
        height: height_4x4,
    } = mi;
    let FrameCodingParams {
        use_ref_frame_mvs, ..
    } = *frame;
    let rav1d_ref0 = cu.ref_frames[0] as i8 - 1;
    let rav1d_ref1 = cu.ref_frames[1] as i8 - 1;

    // skip_mode (spec 5.11.5/5.11.24, dav1d `decode.c:1401-1423`) forces the ENTIRE
    // mode-info tail with zero bits read: `inter_mode = NEARESTMV_NEARESTMV`, `drl_idx =
    // NEAREST` (index 0, no DRL bits), `mv[]` straight from `compound_mv_stack`'s
    // `stack[0]`, `comp_type = AVG`. Real spec never reads `compound_mode()`/DRL/MV-
    // residual/`compound_type()` bits for a skip_mode CU at all -- this crate previously
    // (before this branch existed) fell through to the real-read path below even for
    // skip_mode CUs, a real desync (reading bits a real encoder never wrote).
    let comp_type = if cu.skip_mode {
        derive_skip_mode_tail(tile_ctx, mi, frame, rav1d_ref0, rav1d_ref1, cu)
    } else {
        // compound_mode() (spec 5.11.24) -- a distinct 8-symbol alphabet from the
        // single-ref 4-way `inter_mode`, see `SymbolDecoder::read_compound_mode`'s doc.
        let ctx =
            tile_ctx.compound_mode_context(x4, y4, width_4x4, height_4x4, rav1d_ref0, rav1d_ref1);
        let mode_symbol = decoder.read_compound_mode(ctx)?;
        cu.mode = compound_mode_from_symbol(mode_symbol)?;

        // Real compound DRL (spec 7.10.2.10, joint L0/L1 candidate stack) -- see
        // `SpatialRefContext::compound_mv_stack`'s doc for the real weighted
        // spatial+temporal search this replaces (previously: independent, zero-fallback
        // per-direction lookups via the old placeholder predictor (removed in #215), and no DRL bits read at all --
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
        let drl_idx = read_compound_drl_idx(decoder, &stack, n_mvs, l0_kind, l1_kind)?;

        // Per-direction value: `Nearest`/`Near` read straight from the real stack;
        // `New` uses `stack[drl_idx]` as predictor with the explicit residual added on
        // top (unchanged shape from the single-direction version this replaces);
        // `Global` is the block's global motion vector for that reference.
        cu.mv[0] = match l0_kind {
            Some(MvKind::New) => {
                let explicit_mv = read_explicit_mv(decoder, frame.inter_mode_flags.mv_precision())?;
                explicit_mv.add(stack[drl_idx].mv[0])
            }
            Some(MvKind::Nearest) | Some(MvKind::Near) => stack[drl_idx].mv[0],
            Some(MvKind::Global) => tile_ctx.global_mv(rav1d_ref0, x4, y4, width_4x4, height_4x4),
            None => MotionVector::zero(),
        };
        cu.mv[1] = match l1_kind {
            Some(MvKind::New) => {
                let explicit_mv = read_explicit_mv(decoder, frame.inter_mode_flags.mv_precision())?;
                explicit_mv.add(stack[drl_idx].mv[1])
            }
            Some(MvKind::Nearest) | Some(MvKind::Near) => stack[drl_idx].mv[1],
            Some(MvKind::Global) => tile_ctx.global_mv(rav1d_ref1, x4, y4, width_4x4, height_4x4),
            None => MotionVector::zero(),
        };
        read_compound_type(decoder, tile_ctx, mi, frame, rav1d_ref0, rav1d_ref1)?
    };

    tile_ctx.set_spatial_ref_block(
        x4,
        y4,
        width_4x4,
        height_4x4,
        rav1d_ref0,
        rav1d_ref1,
        cu.mode.l0_mv_kind() == Some(MvKind::New) || cu.mode.l1_mv_kind() == Some(MvKind::New),
        cu.mv[0],
        cu.mv[1],
    );
    if cu.mode == PredictionMode::GlobalGlobalMv {
        tile_ctx.mark_spatial_ref_global_mode(x4, y4, width_4x4, height_4x4);
    }
    tile_ctx.set_comp_type(x4, y4, width_4x4, height_4x4, comp_type);

    tracing::debug!(
        "Compound mode {:?} at ({}, {}): mv0={:?}, mv1={:?}",
        cu.mode,
        x,
        y,
        cu.mv[0],
        cu.mv[1]
    );
    Ok(())
}

/// `skip_mode` tail: the whole mode info is derived with zero bits read. Returns `comp_type`.
fn derive_skip_mode_tail(
    tile_ctx: &mut TileContext,
    mi: MiRect,
    frame: &FrameCodingParams,
    rav1d_ref0: i8,
    rav1d_ref1: i8,
    cu: &mut CodingUnit,
) -> u8 {
    let MiRect {
        x4,
        y4,
        width: width_4x4,
        height: height_4x4,
    } = mi;
    let FrameCodingParams {
        use_ref_frame_mvs, ..
    } = *frame;
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
}

/// Selects the DRL index of a compound block, reading up to two `drl_bit` symbols.
fn read_compound_drl_idx(
    decoder: &mut SymbolDecoder,
    stack: &[CompoundMvStackEntry; 8],
    n_mvs: usize,
    l0_kind: Option<MvKind>,
    l1_kind: Option<MvKind>,
) -> Result<usize> {
    let mut drl_idx = 0usize;
    if l0_kind == Some(MvKind::New) && l1_kind == Some(MvKind::New) {
        if n_mvs > 1 {
            if decoder.read_drl_bit(crate::tile::context::get_compound_drl_context(stack, 0))? {
                drl_idx += 1;
            }
            if drl_idx == 1
                && n_mvs > 2
                && decoder.read_drl_bit(crate::tile::context::get_compound_drl_context(stack, 1))?
            {
                drl_idx += 1;
            }
        }
    } else if l0_kind == Some(MvKind::Near) || l1_kind == Some(MvKind::Near) {
        drl_idx = 1;
        if n_mvs > 2
            && decoder.read_drl_bit(crate::tile::context::get_compound_drl_context(stack, 1))?
        {
            drl_idx += 1;
        }
        // NEAR can still be refined to NEARISH by one more bit when the list is long enough.
        if drl_idx == 2
            && n_mvs > 3
            && decoder.read_drl_bit(crate::tile::context::get_compound_drl_context(stack, 2))?
        {
            drl_idx += 1;
        }
    }
    Ok(drl_idx)
}

/// `compound_type()`: the symbols after the motion vectors. Returns `comp_type`.
fn read_compound_type(
    decoder: &mut SymbolDecoder,
    tile_ctx: &mut TileContext,
    mi: MiRect,
    frame: &FrameCodingParams,
    rav1d_ref0: i8,
    rav1d_ref1: i8,
) -> Result<u8> {
    let MiRect {
        x4,
        y4,
        width: width_4x4,
        height: height_4x4,
    } = mi;
    let inter_mode_flags = frame.inter_mode_flags;

    // compound_type() (spec 5.11.28: jnt_comp vs. segmentation-mask vs. wedge-mask)
    // -- real per-context CDF + adaptation, see `SymbolDecoder::read_mask_comp`'s
    // doc for the desync this closes (previously never read at all for ANY compound
    // block).
    Ok(
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
            let jctx = tile_ctx.jnt_comp_context(x4, y4, rav1d_ref0, rav1d_ref1);
            1 + u8::from(decoder.read_jnt_comp(jctx)?)
        } else {
            2 // AVG
        },
    )
}

/// Convert compound_mode() symbol (spec 5.11.24) to PredictionMode. Symbol ordering matches
/// libaom's `COMPOUND_TYPES`/`compound_mode` enum (`NEAREST_NEARESTMV`=0 .. `NEW_NEWMV`=7).
fn compound_mode_from_symbol(symbol: u8) -> Result<PredictionMode> {
    match symbol {
        0 => Ok(PredictionMode::NearestNearestMv),
        1 => Ok(PredictionMode::NearNearMv),
        2 => Ok(PredictionMode::NearestNewMv),
        3 => Ok(PredictionMode::NewNearestMv),
        4 => Ok(PredictionMode::NearNewMv),
        5 => Ok(PredictionMode::NewNearMv),
        6 => Ok(PredictionMode::GlobalGlobalMv),
        7 => Ok(PredictionMode::NewNewMv),
        _ => Err(BitvueError::InvalidData(format!(
            "Invalid compound mode symbol: {}",
            symbol
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compound_mode_from_symbol_round_trips_all_8() {
        let expected = [
            PredictionMode::NearestNearestMv,
            PredictionMode::NearNearMv,
            PredictionMode::NearestNewMv,
            PredictionMode::NewNearestMv,
            PredictionMode::NearNewMv,
            PredictionMode::NewNearMv,
            PredictionMode::GlobalGlobalMv,
            PredictionMode::NewNewMv,
        ];
        for (symbol, mode) in expected.iter().enumerate() {
            assert_eq!(compound_mode_from_symbol(symbol as u8).unwrap(), *mode);
        }
        assert!(compound_mode_from_symbol(8).is_err());
    }

    // ---- read_compound_drl_idx ----------------------------------------------------------

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

    /// Spec 5.11.23's DRL loop, written out for the two mode families. `ctx` is the context every
    /// position gets from an all-zero-weight stack.
    fn spec_drl(decoder: &mut SymbolDecoder, new_new: bool, near: bool, n_mvs: usize) -> usize {
        let stack = [CompoundMvStackEntry::default(); 8];
        let ctx = |idx| crate::tile::context::get_compound_drl_context(&stack, idx);
        let mut ref_mv_idx = 0;
        if new_new {
            for idx in 0..2 {
                if n_mvs > idx + 1 {
                    if decoder.read_drl_bit(ctx(idx)).unwrap() {
                        ref_mv_idx = idx + 1;
                    } else {
                        ref_mv_idx = idx;
                        break;
                    }
                }
            }
        } else if near {
            ref_mv_idx = 1;
            for idx in 1..3 {
                if n_mvs > idx + 1 {
                    if decoder.read_drl_bit(ctx(idx)).unwrap() {
                        ref_mv_idx = idx + 1;
                    } else {
                        ref_mv_idx = idx;
                        break;
                    }
                }
            }
        }
        ref_mv_idx
    }

    #[test]
    fn the_compound_drl_index_follows_the_specs_loop_for_every_candidate_count() {
        use MvKind::{Global, Near, Nearest, New};
        let stack = [CompoundMvStackEntry::default(); 8];
        // (L0 kind, L1 kind, new_new, has_nearmv)
        let kinds = [
            (New, New, true, false),
            (Near, Near, false, true),
            (Near, New, false, true),
            (New, Near, false, true),
            (Nearest, Nearest, false, false),
            (Nearest, New, false, false),
            (New, Nearest, false, false),
            (Global, Global, false, false),
        ];
        let mut results = std::collections::BTreeSet::new();
        for seed in 1..=24u64 {
            let data = noise(seed, 64);
            for (l0, l1, new_new, near) in kinds {
                for n_mvs in 0..=8usize {
                    let mut d = SymbolDecoder::new(&data).unwrap();
                    let got =
                        read_compound_drl_idx(&mut d, &stack, n_mvs, Some(l0), Some(l1)).unwrap();
                    let mut r = SymbolDecoder::new(&data).unwrap();
                    let want = spec_drl(&mut r, new_new, near, n_mvs);
                    assert_eq!(got, want, "seed {seed} {l0:?}/{l1:?} n_mvs {n_mvs}");
                    assert_eq!(
                        d.read_bools_n(24).unwrap(),
                        r.read_bools_n(24).unwrap(),
                        "seed {seed} {l0:?}/{l1:?} n_mvs {n_mvs}: same bits consumed"
                    );
                    results.insert((new_new, near, n_mvs, got));
                }
            }
        }
        // NEW_NEW reaches 0, 1 and 2; NEAR reaches 1, 2 and 3 once the list is long enough.
        for want in [
            (true, false, 3, 0),
            (true, false, 3, 1),
            (true, false, 3, 2),
        ] {
            assert!(results.contains(&want), "{want:?} never produced");
        }
        for want in [
            (false, true, 4, 1),
            (false, true, 4, 2),
            (false, true, 4, 3),
        ] {
            assert!(results.contains(&want), "{want:?} never produced");
        }
    }

    #[test]
    fn a_single_candidate_means_no_drl_symbols_at_all() {
        let stack = [CompoundMvStackEntry::default(); 8];
        let data = noise(1, 64);
        let untouched = SymbolDecoder::new(&data).unwrap().read_bools_n(24).unwrap();
        for n_mvs in 0..=1usize {
            let mut d = SymbolDecoder::new(&data).unwrap();
            let idx =
                read_compound_drl_idx(&mut d, &stack, n_mvs, Some(MvKind::New), Some(MvKind::New))
                    .unwrap();
            assert_eq!(idx, 0);
            assert_eq!(
                d.read_bools_n(24).unwrap(),
                untouched,
                "NEW_NEW with {n_mvs} candidates"
            );
        }
        // NEAR starts at index 1 and needs more than two candidates to read anything.
        for n_mvs in 0..=2usize {
            let mut d = SymbolDecoder::new(&data).unwrap();
            let idx = read_compound_drl_idx(
                &mut d,
                &stack,
                n_mvs,
                Some(MvKind::Near),
                Some(MvKind::Near),
            )
            .unwrap();
            assert_eq!(idx, 1);
            assert_eq!(
                d.read_bools_n(24).unwrap(),
                untouched,
                "NEAR with {n_mvs} candidates"
            );
        }
    }

    // ---- read_compound_mode_info ---------------------------------------------------------

    use crate::tile::coding_unit::types::RefFrame;

    /// Collects the `message` of every tracing event, per emitting thread. It is installed once as
    /// the process-wide subscriber: a scoped one (`with_default`) races with other tests' callsite
    /// registrations over tracing's global interest cache and sometimes sees nothing (#336).
    #[derive(Default)]
    struct Capture(std::sync::Mutex<std::collections::HashMap<std::thread::ThreadId, Vec<String>>>);

    fn capture() -> &'static Capture {
        static CAPTURE: std::sync::OnceLock<&'static Capture> = std::sync::OnceLock::new();
        CAPTURE.get_or_init(|| {
            let capture: &'static Capture = Box::leak(Box::default());
            tracing::subscriber::set_global_default(CaptureRef(capture))
                .expect("no other test installs a global subscriber");
            capture
        })
    }

    struct CaptureRef(&'static Capture);

    impl tracing::Subscriber for CaptureRef {
        fn enabled(&self, _: &tracing::Metadata<'_>) -> bool {
            true
        }
        fn new_span(&self, _: &tracing::span::Attributes<'_>) -> tracing::span::Id {
            tracing::span::Id::from_u64(1)
        }
        fn record(&self, _: &tracing::span::Id, _: &tracing::span::Record<'_>) {}
        fn record_follows_from(&self, _: &tracing::span::Id, _: &tracing::span::Id) {}
        fn event(&self, event: &tracing::Event<'_>) {
            struct Message(String);
            impl tracing::field::Visit for Message {
                fn record_debug(&mut self, field: &tracing::field::Field, v: &dyn std::fmt::Debug) {
                    if field.name() == "message" {
                        self.0 = format!("{v:?}");
                    }
                }
            }
            let mut m = Message(String::new());
            event.record(&mut m);
            let mut logs = self.0 .0.lock().unwrap();
            logs.entry(std::thread::current().id())
                .or_default()
                .push(m.0);
        }
        fn enter(&self, _: &tracing::span::Id) {}
        fn exit(&self, _: &tracing::span::Id) {}
    }

    fn compound_cu(skip_mode: bool) -> CodingUnit {
        let mut cu = CodingUnit::new(16, 16, 16, 16);
        cu.ref_frames = [RefFrame::Last, RefFrame::BwdRef];
        cu.skip_mode = skip_mode;
        cu
    }

    const RECT: BlockRect = BlockRect {
        x: 16,
        y: 32,
        width: 16,
        height: 16,
    };
    const MI: MiRect = MiRect {
        x4: 4,
        y4: 8,
        width: 4,
        height: 4,
    };

    fn read(data: &[u8], skip_mode: bool) -> (CodingUnit, Vec<String>, u32) {
        let capture = capture();
        let me = std::thread::current().id();
        let mut d = SymbolDecoder::new(data).unwrap();
        let mut ctx = TileContext::new(32, 32);
        let mut cu = compound_cu(skip_mode);
        let frame = FrameCodingParams::for_tests();
        // Only the events of the read itself count, not those of building the decoder above.
        capture.0.lock().unwrap().remove(&me);
        read_compound_mode_info(&mut d, &mut ctx, RECT, MI, &frame, &mut cu).unwrap();
        let logs = capture.0.lock().unwrap().remove(&me).unwrap_or_default();
        (cu, logs, d.read_bools_n(24).unwrap())
    }

    #[test]
    fn a_compound_block_reads_its_mode_and_logs_the_result() {
        let mut modes = std::collections::BTreeSet::new();
        for seed in 1..=48u64 {
            let data = noise(seed, 128);
            let (cu, logs, _) = read(&data, false);

            // The mode is the 8-symbol compound_mode() read in the block's context.
            let mut r = SymbolDecoder::new(&data).unwrap();
            let ctx = TileContext::new(32, 32).compound_mode_context(4, 8, 4, 4, 0, 4);
            let want = compound_mode_from_symbol(r.read_compound_mode(ctx).unwrap()).unwrap();
            assert_eq!(cu.mode, want, "seed {seed}");
            modes.insert(format!("{want:?}"));

            // Without neighbours every candidate is zero, so only NEW components can be
            // non-zero (explicit residual on top of a zero predictor).
            let (k0, k1) = (cu.mode.l0_mv_kind(), cu.mode.l1_mv_kind());
            if k0 != Some(MvKind::New) {
                assert_eq!(cu.mv[0], MotionVector::zero(), "seed {seed} {:?}", cu.mode);
            }
            if k1 != Some(MvKind::New) {
                assert_eq!(cu.mv[1], MotionVector::zero(), "seed {seed} {:?}", cu.mode);
            }

            assert_eq!(
                logs,
                vec![format!(
                    "Compound mode {:?} at (16, 32): mv0={:?}, mv1={:?}",
                    cu.mode, cu.mv[0], cu.mv[1]
                )],
                "seed {seed}: exactly one debug line describing the result"
            );
        }
        assert!(
            modes.len() >= 4,
            "seeds should produce several modes: {modes:?}"
        );
    }

    #[test]
    fn a_skip_mode_block_takes_nearest_nearest_without_reading_and_still_logs() {
        let data = noise(3, 128);
        let untouched = SymbolDecoder::new(&data).unwrap().read_bools_n(24).unwrap();
        let (cu, logs, bits) = read(&data, true);
        assert_eq!(cu.mode, PredictionMode::NearestNearestMv);
        assert_eq!(bits, untouched, "skip_mode reads no symbol at all");
        assert_eq!(logs.len(), 1);
        assert!(
            logs[0].starts_with("Compound mode NearestNearestMv at (16, 32)"),
            "{}",
            logs[0]
        );
    }
}
