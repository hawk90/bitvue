//! `motion_mode` and OBMC of a single-reference inter block (spec 5.11.27): the eligibility gates
//! (block size, overlappable neighbours, global-motion exclusion) and the symbols they guard.

use super::types::{CodingUnit, PredictionMode, RefFrame};
use crate::symbol::SymbolDecoder;
use crate::tile::{FrameCodingParams, MiRect, TileContext};
use bitvue_engine::Result;

/// `motion_mode` / OBMC symbols of a single-ref block. Returns whether the block is `WARPED_CAUSAL`.
pub(super) fn read_motion_mode_info(
    decoder: &mut SymbolDecoder,
    tile_ctx: &mut TileContext,
    mi: MiRect,
    frame: &FrameCodingParams,
    rav1d_ref0: i8,
    cu: &CodingUnit,
    is_interintra: bool,
) -> Result<bool> {
    let MiRect {
        x4,
        y4,
        width: width_4x4,
        height: height_4x4,
    } = mi;
    let FrameCodingParams {
        inter_mode_flags, ..
    } = *frame;
    // motion_mode (spec 5.11.27) -- real per-exact-block-size CDF + adaptation, see
    // `SymbolDecoder::read_motion_mode`'s doc for the desync this closes (previously
    // never read at all). Real spec gate: switchable, not interintra, both dims >= 8px,
    // not an excluded warped-global-motion case (real: a `GLOBALMV` block -- the only
    // global-motion mode reachable in this single-ref branch, `GLOBAL_GLOBALMV` being
    // compound-only -- whose `GmType[RefFrame[0]]` is more complex than TRANSLATION reads
    // zero bits here, matching real spec's forced `motion_mode = SIMPLE`; `!force_integer_
    // mv` gates the whole check per spec, same as `read_motion_mode`'s other real gates),
    // and has a real overlappable (non-intra) above/left neighbor.
    let gm_forces_simple = global_motion_forces_simple(
        cu.mode,
        inter_mode_flags.force_integer_mv,
        &inter_mode_flags.gm_type,
        cu.ref_frames[0],
    );
    let mut is_warp = false;
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
        // why a full port is deferred). A scaled reference (every reference of a superres
        // frame) is never warped: it reads `use_obmc` instead.
        let allow_warp = inter_mode_flags.allow_warped_motion
            && !inter_mode_flags.ref_scaled[cu.ref_frames[0] as usize]
            && tile_ctx.has_matching_edge_ref(
                x4,
                y4,
                width_4x4,
                height_4x4,
                width_4x4.min(frame.mi_cols.saturating_sub(x4)),
                height_4x4.min(frame.mi_rows.saturating_sub(y4)),
                frame.mi_cols,
                rav1d_ref0,
            );
        if allow_warp {
            if let Some(idx) = motion_mode_size_index(width_4x4, height_4x4) {
                is_warp = decoder.read_motion_mode(idx)? == 2;
            }
        } else if let Some(idx) = motion_mode_size_index(width_4x4, height_4x4) {
            decoder.read_obmc(idx)?;
        }
    }
    Ok(is_warp)
}

/// `read_motion_mode`'s real spec 5.11.27 `GmType`-dependent exclusion: `true` when a
/// `GLOBALMV` block's global motion (`gm_type[ref0 as usize]`) is more complex than TRANSLATION,
/// in which case real spec forces `motion_mode = SIMPLE` and reads zero bits (the whole check is
/// itself gated on `!force_integer_mv`). `GLOBAL_GLOBALMV` never reaches this: it's compound-only,
/// and this crate's `motion_mode` read only happens in the single-ref branch (`parse_coding_unit`'s
/// call site doc).
fn global_motion_forces_simple(
    mode: PredictionMode,
    force_integer_mv: bool,
    gm_type: &[u8; 8],
    ref0: RefFrame,
) -> bool {
    !force_integer_mv
        && mode == PredictionMode::GlobalMv
        && gm_type[ref0 as usize] > crate::frame_header_full::GM_TYPE_TRANSLATION
}

/// `motion_mode`/`obmc`'s exact-block-size CDF index (0..=16) -- real spec/dav1d indexing order
/// matches `CdfContext::motion_mode_cdf`'s literal table order (`read_motion_mode`'s doc); `None`
/// for any size real spec never reads `motion_mode` for at all (`min(bw4,bh4) < 2`, i.e. either
/// dimension `< 8px` -- ported as a direct dimension match rather than reusing
/// `y_mode_size_context`'s coarser 4-class grouping, since `motion_mode` needs the real per-size
/// table, not a class).
fn motion_mode_size_index(width_4x4: u32, height_4x4: u32) -> Option<u8> {
    match (width_4x4, height_4x4) {
        (2, 2) => Some(0),    // 8x8
        (2, 4) => Some(1),    // 8x16
        (4, 2) => Some(2),    // 16x8
        (4, 4) => Some(3),    // 16x16
        (4, 8) => Some(4),    // 16x32
        (8, 4) => Some(5),    // 32x16
        (8, 8) => Some(6),    // 32x32
        (8, 16) => Some(7),   // 32x64
        (16, 8) => Some(8),   // 64x32
        (16, 16) => Some(9),  // 64x64
        (16, 32) => Some(10), // 64x128
        (32, 16) => Some(11), // 128x64
        (32, 32) => Some(12), // 128x128
        (2, 8) => Some(13),   // 8x32
        (4, 16) => Some(14),  // 16x64
        (8, 2) => Some(15),   // 32x8
        (16, 4) => Some(16),  // 64x16
        _ => None,
    }
}

/// `motion_mode`'s real "has overlappable neighbours" eligibility gate (spec 5.11.27) -- true
/// when at least one ODD-offset 4x4 unit along this CU's own above or left edge belongs to a
/// real INTER neighbor (source: rav1d's `findoddzero` scanning `t->a->intra`/`t->l.intra`,
/// `memorysafety/rav1d`, BSD-2-Clause, `src/decode.c`) -- ported using this crate's own
/// `above_ref_intra`/`left_ref_intra` arrays directly (same per-4x4-unit granularity real dav1d's
/// `BlockContext.intra` tracks, no approximation needed here unlike `has_matching_single_ref`).
fn has_overlappable_neighbors(
    tile_ctx: &crate::tile::TileContext,
    x4: u32,
    y4: u32,
    width_4x4: u32,
    height_4x4: u32,
) -> bool {
    if tile_ctx.has_left(x4) {
        let len = height_4x4 / 2;
        for n in 0..len {
            if !tile_ctx.left_is_intra(y4 + 1 + n * 2) {
                return true;
            }
        }
    }
    if tile_ctx.has_top(y4) {
        let len = width_4x4 / 2;
        for n in 0..len {
            if !tile_ctx.above_is_intra(x4 + 1 + n * 2) {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::super::types::MotionVector;
    use super::*;

    #[test]
    fn global_motion_forces_simple_matches_spec_5_11_27_exclusion() {
        use crate::frame_header_full::{GM_TYPE_IDENTITY, GM_TYPE_TRANSLATION};
        let rotzoom: [u8; 8] = [GM_TYPE_IDENTITY, GM_TYPE_TRANSLATION + 1, 0, 0, 0, 0, 0, 0];
        let translation_only: [u8; 8] = [GM_TYPE_IDENTITY, GM_TYPE_TRANSLATION, 0, 0, 0, 0, 0, 0];
        let identity: [u8; 8] = [GM_TYPE_IDENTITY; 8];

        // GLOBALMV + GmType[ref0] > TRANSLATION + !force_integer_mv => excluded (forced SIMPLE).
        assert!(global_motion_forces_simple(
            PredictionMode::GlobalMv,
            false,
            &rotzoom,
            RefFrame::Last
        ));
        // Same, but force_integer_mv=true => spec's outer gate disables the whole check.
        assert!(!global_motion_forces_simple(
            PredictionMode::GlobalMv,
            true,
            &rotzoom,
            RefFrame::Last
        ));
        // GmType[ref0] == TRANSLATION (not `>` TRANSLATION) => not excluded.
        assert!(!global_motion_forces_simple(
            PredictionMode::GlobalMv,
            false,
            &translation_only,
            RefFrame::Last
        ));
        // GmType[ref0] == IDENTITY => not excluded.
        assert!(!global_motion_forces_simple(
            PredictionMode::GlobalMv,
            false,
            &identity,
            RefFrame::Last
        ));
        // Any non-GLOBALMV mode => never excluded by this check, regardless of GmType.
        assert!(!global_motion_forces_simple(
            PredictionMode::NewMv,
            false,
            &rotzoom,
            RefFrame::Last
        ));
        // Indexed by the real ref0, not a fixed slot -- a ROTZOOM GmType on a *different* ref
        // than the one this block actually uses must not trigger the exclusion.
        assert!(!global_motion_forces_simple(
            PredictionMode::GlobalMv,
            false,
            &rotzoom,
            RefFrame::Last2
        ));
    }

    // ---- read_motion_mode_info ----------------------------------------------------------

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

    /// rav1d's reference number of LAST, used for both the neighbours and the block.
    const REF0: i8 = 0;

    fn mi(w4: u32, h4: u32) -> MiRect {
        MiRect {
            x4: 8,
            y4: 8,
            width: w4,
            height: h4,
        }
    }

    /// A tile where the block at (8, 8) has an inter neighbour above (and to the left) whose single
    /// reference is `neighbour_ref`; `inter == false` leaves every cell intra (the default).
    fn tile(inter: bool, neighbour_ref: i8) -> TileContext {
        let mut ctx = TileContext::new(64, 64);
        ctx.set_frame_extent(256, 256);
        ctx.set_tile_extent(0, 64, 0, 64);
        if inter {
            for (x4, y4, w4, h4) in [(0u32, 7u32, 64u32, 1u32), (7, 0, 1, 64)] {
                ctx.set_intra_flag(x4, y4, w4, h4, false);
                ctx.set_spatial_ref_block(
                    x4,
                    y4,
                    w4,
                    h4,
                    neighbour_ref,
                    -1,
                    false,
                    MotionVector::zero(),
                    MotionVector::zero(),
                );
            }
        }
        ctx
    }

    fn frame(switchable: bool, allow_warp: bool) -> FrameCodingParams {
        let mut f = FrameCodingParams::for_tests();
        f.inter_mode_flags.switchable_motion_mode = switchable;
        f.inter_mode_flags.allow_warped_motion = allow_warp;
        f.mi_cols = 64;
        f.mi_rows = 64;
        f
    }

    fn cu(mode: PredictionMode) -> CodingUnit {
        let mut cu = CodingUnit::new(32, 32, 16, 16);
        cu.mode = mode;
        cu.ref_frames = [RefFrame::Last, RefFrame::Intra];
        cu
    }

    /// Returns `(is_warp, next 24 bits)`.
    fn run(
        data: &[u8],
        ctx: &mut TileContext,
        m: MiRect,
        f: &FrameCodingParams,
        cu: &CodingUnit,
        interintra: bool,
    ) -> (bool, u32) {
        let mut d = SymbolDecoder::new(data).unwrap();
        let warp = read_motion_mode_info(&mut d, ctx, m, f, REF0, cu, interintra).unwrap();
        (warp, d.read_bools_n(24).unwrap())
    }

    fn untouched(data: &[u8]) -> u32 {
        SymbolDecoder::new(data).unwrap().read_bools_n(24).unwrap()
    }

    #[test]
    fn a_matching_neighbour_and_allow_warped_motion_read_motion_mode_otherwise_use_obmc() {
        let m = mi(4, 4); // 16x16 -> size index 3
        let (mut warp_seen, mut simple_seen) = (false, false);
        for seed in 1..=48u64 {
            let data = noise(seed, 64);
            let mut r = SymbolDecoder::new(&data).unwrap();
            let sym = r.read_motion_mode(3).unwrap();
            let after_motion_mode = r.read_bools_n(24).unwrap();
            let mut r = SymbolDecoder::new(&data).unwrap();
            r.read_obmc(3).unwrap();
            let after_obmc = r.read_bools_n(24).unwrap();

            // warp allowed: neighbour references the block's own reference.
            let (warp, bits) = run(
                &data,
                &mut tile(true, REF0),
                m,
                &frame(true, true),
                &cu(PredictionMode::NewMv),
                false,
            );
            assert_eq!(warp, sym == 2, "seed {seed}");
            assert_eq!(
                bits, after_motion_mode,
                "seed {seed}: one motion_mode symbol"
            );
            warp_seen |= warp;
            simple_seen |= !warp;

            // neighbour uses another reference, or warping is off: OBMC bit instead.
            for (label, ctx, f) in [
                ("other reference", tile(true, 3), frame(true, true)),
                ("warp disabled", tile(true, REF0), frame(true, false)),
            ] {
                let mut ctx = ctx;
                let (warp, bits) = run(&data, &mut ctx, m, &f, &cu(PredictionMode::NewMv), false);
                assert!(!warp, "seed {seed}: {label}");
                assert_eq!(bits, after_obmc, "seed {seed}: {label} reads use_obmc");
            }
        }
        assert!(
            warp_seen && simple_seen,
            "seeds must decode both warp and non-warp"
        );
    }

    #[test]
    fn a_scaled_reference_is_never_warped_so_it_reads_use_obmc() {
        let m = mi(4, 4);
        for seed in 1..=24u64 {
            let data = noise(seed, 64);
            let mut r = SymbolDecoder::new(&data).unwrap();
            r.read_obmc(3).unwrap();
            let after_obmc = r.read_bools_n(24).unwrap();
            let mut f = frame(true, true);
            // Only the block's own reference being scaled matters, not the others.
            f.inter_mode_flags.ref_scaled = [false, true, false, false, false, false, false, false];
            let (warp, bits) = run(
                &data,
                &mut tile(true, REF0),
                m,
                &f,
                &cu(PredictionMode::NewMv),
                false,
            );
            assert!(!warp && bits == after_obmc, "seed {seed}");
            // Scaled elsewhere, unscaled here: still warpable.
            f.inter_mode_flags.ref_scaled = [false, false, true, true, true, true, true, true];
            let mut r = SymbolDecoder::new(&data).unwrap();
            let sym = r.read_motion_mode(3).unwrap();
            let after_mode = r.read_bools_n(24).unwrap();
            let (warp, bits) = run(
                &data,
                &mut tile(true, REF0),
                m,
                &f,
                &cu(PredictionMode::NewMv),
                false,
            );
            assert!(warp == (sym == 2) && bits == after_mode, "seed {seed}");
        }
    }

    #[test]
    fn every_gate_that_fails_reads_nothing() {
        use crate::frame_header_full::GM_TYPE_TRANSLATION;
        let data = noise(4, 64);
        let none = untouched(&data);
        let m = mi(4, 4);
        let ok_cu = cu(PredictionMode::NewMv);

        // Sanity: the baseline does read.
        assert_ne!(
            run(
                &data,
                &mut tile(true, REF0),
                m,
                &frame(true, true),
                &ok_cu,
                false
            )
            .1,
            none
        );

        let mut rotzoom = frame(true, true);
        rotzoom.inter_mode_flags.gm_type[RefFrame::Last as usize] = GM_TYPE_TRANSLATION + 1;
        let cases: Vec<(
            &str,
            TileContext,
            MiRect,
            FrameCodingParams,
            CodingUnit,
            bool,
        )> = vec![
            (
                "not switchable",
                tile(true, REF0),
                m,
                frame(false, true),
                ok_cu.clone(),
                false,
            ),
            (
                "interintra",
                tile(true, REF0),
                m,
                frame(true, true),
                ok_cu.clone(),
                true,
            ),
            (
                "global motion forces simple",
                tile(true, REF0),
                m,
                rotzoom,
                cu(PredictionMode::GlobalMv),
                false,
            ),
            (
                "4 px wide",
                tile(true, REF0),
                mi(1, 4),
                frame(true, true),
                ok_cu.clone(),
                false,
            ),
            (
                "4 px tall",
                tile(true, REF0),
                mi(4, 1),
                frame(true, true),
                ok_cu.clone(),
                false,
            ),
            (
                "only intra neighbours",
                tile(false, REF0),
                m,
                frame(true, true),
                ok_cu.clone(),
                false,
            ),
        ];
        for (label, mut ctx, m, f, c, interintra) in cases {
            let (warp, bits) = run(&data, &mut ctx, m, &f, &c, interintra);
            assert!(!warp, "{label}");
            assert_eq!(bits, none, "{label}: no symbol");
        }
    }

    #[test]
    fn a_block_shape_without_a_motion_mode_cdf_reads_nothing_even_when_eligible() {
        // 128x32 (32 x 8 units) passes every gate but is not an AV1 block size, so no per-size CDF
        // exists: neither the warp nor the OBMC read can be made.
        let data = noise(6, 64);
        let none = untouched(&data);
        for allow_warp in [true, false] {
            let (warp, bits) = run(
                &data,
                &mut tile(true, REF0),
                mi(32, 8),
                &frame(true, allow_warp),
                &cu(PredictionMode::NewMv),
                false,
            );
            assert!(!warp, "allow_warp {allow_warp}");
            assert_eq!(bits, none, "allow_warp {allow_warp}");
        }
        assert_eq!(motion_mode_size_index(32, 8), None);
        assert_eq!(motion_mode_size_index(8, 2), Some(15));
        assert_eq!(motion_mode_size_index(2, 2), Some(0));
    }

    #[test]
    fn a_single_overlappable_neighbour_on_either_edge_is_enough() {
        let m = mi(4, 4);
        let data = noise(8, 64);
        let none = untouched(&data);
        // Only the row above is inter.
        let mut above_only = TileContext::new(64, 64);
        above_only.set_frame_extent(256, 256);
        above_only.set_tile_extent(0, 64, 0, 64);
        above_only.set_intra_flag(9, 7, 1, 1, false); // x4 + 1: the odd offset the scan checks
        let (_, bits) = run(
            &data,
            &mut above_only,
            m,
            &frame(true, false),
            &cu(PredictionMode::NewMv),
            false,
        );
        assert_ne!(bits, none, "above neighbour at the odd offset");

        // Only the column to the left.
        let mut left_only = TileContext::new(64, 64);
        left_only.set_frame_extent(256, 256);
        left_only.set_tile_extent(0, 64, 0, 64);
        left_only.set_intra_flag(7, 9, 1, 1, false);
        let (_, bits) = run(
            &data,
            &mut left_only,
            m,
            &frame(true, false),
            &cu(PredictionMode::NewMv),
            false,
        );
        assert_ne!(bits, none, "left neighbour at the odd offset");

        // An inter cell at an even offset is not looked at.
        let mut even = TileContext::new(64, 64);
        even.set_frame_extent(256, 256);
        even.set_tile_extent(0, 64, 0, 64);
        even.set_intra_flag(8, 7, 1, 1, false);
        even.set_intra_flag(7, 8, 1, 1, false);
        let (_, bits) = run(
            &data,
            &mut even,
            m,
            &frame(true, false),
            &cu(PredictionMode::NewMv),
            false,
        );
        assert_eq!(bits, none, "only odd-offset cells count");
    }
}
