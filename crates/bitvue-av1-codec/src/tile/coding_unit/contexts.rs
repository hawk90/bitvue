//! Pure derivations used by `parse_coding_unit`: symbol-to-mode mappings, block-size/context
//! lookups and spec eligibility predicates. None of these read the bitstream.

use super::types::{PredictionMode, RefFrame};
use bitvue_engine::{BitvueError, Result};

/// Convert compound_mode() symbol (spec 5.11.24) to PredictionMode. Symbol ordering matches
/// libaom's `COMPOUND_TYPES`/`compound_mode` enum (`NEAREST_NEARESTMV`=0 .. `NEW_NEWMV`=7).
pub(super) fn compound_mode_from_symbol(symbol: u8) -> Result<PredictionMode> {
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

/// Convert INTRA mode symbol to PredictionMode
pub(super) fn intra_mode_from_symbol(symbol: u8) -> Result<PredictionMode> {
    match symbol {
        0 => Ok(PredictionMode::DcPred),
        1 => Ok(PredictionMode::VPred),
        2 => Ok(PredictionMode::HPred),
        3 => Ok(PredictionMode::D45Pred),
        4 => Ok(PredictionMode::D135Pred),
        5 => Ok(PredictionMode::D113Pred),
        6 => Ok(PredictionMode::D157Pred),
        7 => Ok(PredictionMode::D203Pred),
        8 => Ok(PredictionMode::D67Pred),
        9 => Ok(PredictionMode::SmoothPred),
        10 => Ok(PredictionMode::SmoothVPred),
        11 => Ok(PredictionMode::SmoothHPred),
        12 => Ok(PredictionMode::PaethPred),
        _ => Err(BitvueError::InvalidData(format!(
            "Invalid INTRA mode symbol: {}",
            symbol
        ))),
    }
}

/// Convert INTER mode symbol to PredictionMode
pub(super) fn inter_mode_from_symbol(symbol: u8) -> Result<PredictionMode> {
    match symbol {
        0 => Ok(PredictionMode::NewMv),
        1 => Ok(PredictionMode::NearestMv),
        2 => Ok(PredictionMode::NearMv),
        3 => Ok(PredictionMode::GlobalMv),
        _ => Err(BitvueError::InvalidData(format!(
            "Invalid INTER mode symbol: {}",
            symbol
        ))),
    }
}

/// Decode a `neg_deinterleave`-encoded diff back into a real value (spec 5.11.9/5.11.10's
/// `segment_id()`, also used elsewhere in real AV1 for similarly-encoded values this crate
/// doesn't read) -- ported index-for-index from dav1d's `neg_deinterleave` (`src/decode.c`,
/// `memorysafety/rav1d`/`videolan/dav1d`, BSD-2-Clause), not reimplemented from a description, to
/// avoid an off-by-one in the branch math. `ref_val`/`max` name the spec's `ref`/`max` params
/// (`ref` avoided as a Rust keyword).
pub(super) fn neg_deinterleave(diff: i32, ref_val: i32, max: i32) -> i32 {
    if ref_val == 0 {
        return diff;
    }
    if ref_val >= max - 1 {
        return max - diff - 1;
    }
    if 2 * ref_val < max {
        if diff <= 2 * ref_val {
            if diff & 1 != 0 {
                ref_val + ((diff + 1) >> 1)
            } else {
                ref_val - (diff >> 1)
            }
        } else {
            diff
        }
    } else if diff <= 2 * (max - ref_val - 1) {
        if diff & 1 != 0 {
            ref_val + ((diff + 1) >> 1)
        } else {
            ref_val - (diff >> 1)
        }
    } else {
        max - (diff + 1)
    }
}

/// Map a CU's real pixel dimensions to this crate's `BlockSize` enum -- used only by
/// `read_use_filter_intra`'s CDF lookup (the real spec table is indexed by exact block size, not
/// by the coarser `bsize_ctx`/`tx_size` classes used elsewhere). Every block size the partition
/// tree can produce is covered; the fallback exists only for defensive safety.
pub(super) fn block_size_for_dimensions(width: u32, height: u32) -> crate::tile::BlockSize {
    use crate::tile::BlockSize::*;
    match (width, height) {
        (4, 4) => Block4x4,
        (4, 8) => Block4x8,
        (8, 4) => Block8x4,
        (8, 8) => Block8x8,
        (8, 16) => Block8x16,
        (16, 8) => Block16x8,
        (16, 16) => Block16x16,
        (16, 32) => Block16x32,
        (32, 16) => Block32x16,
        (32, 32) => Block32x32,
        (32, 64) => Block32x64,
        (64, 32) => Block64x32,
        (64, 64) => Block64x64,
        (64, 128) => Block64x128,
        (128, 64) => Block128x64,
        (128, 128) => Block128x128,
        (16, 4) => Block16x4,
        (4, 16) => Block4x16,
        (32, 8) => Block32x8,
        (64, 16) => Block64x16,
        (128, 32) => Block128x32,
        (8, 32) => Block8x32,
        (16, 64) => Block16x64,
        (32, 128) => Block32x128,
        _ => Block4x4,
    }
}

/// Non-key-frame `y_mode`'s block-size-class context (0..=3) -- real spec/dav1d
/// `dav1d_ymode_size_context[bs]` (`memorysafety/rav1d`, BSD-2-Clause, `src/tables.c`), literal
/// per-size lookup (not a formula -- porting the raw table avoids guessing at a closed form from
/// the values, matching this crate's established precedent for lookup-shaped spec tables).
/// `width_4x4`/`height_4x4`: CU dimensions in 4x4 units (matches every real AV1 block size,
/// including `4x16`/`16x4` this crate's own `BlockSize` enum doesn't model as named variants --
/// this function keys on the raw dimensions directly instead, so that gap doesn't apply here).
pub(super) fn y_mode_size_context(width_4x4: u32, height_4x4: u32) -> u8 {
    match (width_4x4, height_4x4) {
        (32, 32) | (32, 16) | (16, 32) | (16, 16) | (16, 8) | (8, 16) | (8, 8) => 3,
        (16, 4) | (8, 4) | (4, 16) | (4, 8) | (4, 4) => 2,
        (8, 2) | (4, 2) | (2, 8) | (2, 4) | (2, 2) => 1,
        _ => 0,
    }
}

/// `read_motion_mode`'s real spec 5.11.27 `GmType`-dependent exclusion: `true` when a
/// `GLOBALMV` block's global motion (`gm_type[ref0 as usize]`) is more complex than TRANSLATION,
/// in which case real spec forces `motion_mode = SIMPLE` and reads zero bits (the whole check is
/// itself gated on `!force_integer_mv`). `GLOBAL_GLOBALMV` never reaches this: it's compound-only,
/// and this crate's `motion_mode` read only happens in the single-ref branch (`parse_coding_unit`'s
/// call site doc).
pub(super) fn global_motion_forces_simple(
    mode: PredictionMode,
    force_integer_mv: bool,
    gm_type: &[u8; 8],
    ref0: RefFrame,
) -> bool {
    !force_integer_mv
        && mode == PredictionMode::GlobalMv
        && gm_type[ref0 as usize] > crate::frame_header_full::GM_TYPE_TRANSLATION
}

/// `needs_interp_filter()` (spec 5.11.30) -- verified against dav1d's `decode.c`
/// `has_subpel_filter` computation (source-only re-clone, no build), which the `filter` read's
/// call site doc explains. `true` unconditionally for every mode except GLOBALMV/GLOBAL_GLOBALMV,
/// where it's `true` only for a minimal-size block (`min(width_4x4, height_4x4) == 1`) or when
/// the relevant ref's `gm_type` is exactly TRANSLATION -- IDENTITY/ROTZOOM/AFFINE all suppress the
/// read (real spec forces `EIGHTTAP_REGULAR` in that case, not a bit read).
pub(super) fn needs_interp_filter(
    mode: PredictionMode,
    width_4x4: u32,
    height_4x4: u32,
    gm_type: &[u8; 8],
    ref0: RefFrame,
    ref1: RefFrame,
) -> bool {
    let is_minimal = width_4x4.min(height_4x4) == 1;
    let is_translation =
        |r: RefFrame| gm_type[r as usize] == crate::frame_header_full::GM_TYPE_TRANSLATION;
    match mode {
        PredictionMode::GlobalMv => is_minimal || is_translation(ref0),
        PredictionMode::GlobalGlobalMv => {
            is_minimal || is_translation(ref0) || is_translation(ref1)
        }
        _ => true,
    }
}

/// `motion_mode`/`obmc`'s exact-block-size CDF index (0..=16) -- real spec/dav1d indexing order
/// matches `CdfContext::motion_mode_cdf`'s literal table order (`read_motion_mode`'s doc); `None`
/// for any size real spec never reads `motion_mode` for at all (`min(bw4,bh4) < 2`, i.e. either
/// dimension `< 8px` -- ported as a direct dimension match rather than reusing
/// `y_mode_size_context`'s coarser 4-class grouping, since `motion_mode` needs the real per-size
/// table, not a class).
pub(super) fn motion_mode_size_index(width_4x4: u32, height_4x4: u32) -> Option<u8> {
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
pub(super) fn has_overlappable_neighbors(
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

/// Compound `wedge`'s real per-size context (0..=8), also compound-wedge/interintra-wedge
/// eligibility gate -- real spec/dav1d `dav1d_wedge_ctx_lut` (`memorysafety/rav1d`,
/// BSD-2-Clause, `src/tables.c`), literal per-size lookup covering exactly the 9 real
/// wedge-eligible sizes (`None` = wedge/interintra not allowed at all for this size).
/// `interintra`'s own eligibility is a REAL subset excluding `8x32`/`32x8` (ctx `7`/`8`) --
/// confirmed via `CdfContext::interintra_wedge_cdf`'s real 7-entry table (not 9): callers gating
/// interintra must additionally check `ctx <= 6`.
pub(super) fn wedge_ctx(width_4x4: u32, height_4x4: u32) -> Option<u8> {
    match (width_4x4, height_4x4) {
        (8, 8) => Some(6), // 32x32
        (8, 4) => Some(5), // 32x16
        (8, 2) => Some(8), // 32x8
        (4, 8) => Some(4), // 16x32
        (4, 4) => Some(3), // 16x16
        (4, 2) => Some(2), // 16x8
        (2, 8) => Some(7), // 8x32
        (2, 4) => Some(1), // 8x16
        (2, 2) => Some(0), // 8x8
        _ => None,
    }
}

/// `HasChroma` (dav1d `decode_b`, spec 5.11.5): whether this block codes the chroma of its area.
/// Blocks narrower or shorter than 8 luma samples share one chroma block with their neighbours
/// (4:2:0), and only the last of them -- the one at an odd 4x4 column / row -- codes it.
pub(super) fn has_chroma(mi: crate::tile::MiRect, flags: &crate::tile::TxTypeFrameFlags) -> bool {
    let ss_x = u32::from(flags.subsampling_x);
    let ss_y = u32::from(flags.subsampling_y);
    !flags.mono_chrome
        && (mi.width > ss_x || (mi.x4 & 1) == 1)
        && (mi.height > ss_y || (mi.y4 & 1) == 1)
}

#[cfg(test)]
mod tests {
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

    #[test]
    fn needs_interp_filter_matches_dav1d_has_subpel_filter() {
        use crate::frame_header_full::{GM_TYPE_IDENTITY, GM_TYPE_ROTZOOM, GM_TYPE_TRANSLATION};
        let translation: [u8; 8] = [GM_TYPE_IDENTITY, GM_TYPE_TRANSLATION, 0, 0, 0, 0, 0, 0];
        let identity: [u8; 8] = [GM_TYPE_IDENTITY; 8];
        let rotzoom_ref2: [u8; 8] = [
            GM_TYPE_IDENTITY,
            GM_TYPE_IDENTITY,
            GM_TYPE_ROTZOOM,
            0,
            0,
            0,
            0,
            0,
        ];

        // Non-global modes always need the filter read, regardless of size/GmType.
        assert!(needs_interp_filter(
            PredictionMode::NewMv,
            16,
            16,
            &identity,
            RefFrame::Last,
            RefFrame::Intra
        ));
        assert!(needs_interp_filter(
            PredictionMode::NearestNearestMv,
            16,
            16,
            &identity,
            RefFrame::Last,
            RefFrame::Last2
        ));

        // GLOBALMV, large block, GmType==TRANSLATION => read.
        assert!(needs_interp_filter(
            PredictionMode::GlobalMv,
            16,
            16,
            &translation,
            RefFrame::Last,
            RefFrame::Intra
        ));
        // GLOBALMV, large block, GmType==IDENTITY => suppressed (forced EIGHTTAP).
        assert!(!needs_interp_filter(
            PredictionMode::GlobalMv,
            16,
            16,
            &identity,
            RefFrame::Last,
            RefFrame::Intra
        ));
        // GLOBALMV, minimal-size block (min dim == 1, i.e. 4px) => always read regardless of GmType.
        assert!(needs_interp_filter(
            PredictionMode::GlobalMv,
            1,
            16,
            &identity,
            RefFrame::Last,
            RefFrame::Intra
        ));

        // GLOBAL_GLOBALMV: either ref being TRANSLATION is enough.
        assert!(needs_interp_filter(
            PredictionMode::GlobalGlobalMv,
            16,
            16,
            &translation,
            RefFrame::Last2,
            RefFrame::Last
        ));
        // GLOBAL_GLOBALMV: neither ref TRANSLATION => suppressed.
        assert!(!needs_interp_filter(
            PredictionMode::GlobalGlobalMv,
            16,
            16,
            &rotzoom_ref2,
            RefFrame::Last,
            RefFrame::Golden
        ));
    }

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

    #[test]
    fn test_neg_deinterleave_ref_zero_returns_diff_directly() {
        assert_eq!(neg_deinterleave(3, 0, 8), 3);
    }

    #[test]
    fn test_neg_deinterleave_ref_at_max_boundary() {
        // ref_val=7 >= max-1=7 -> max - diff - 1.
        assert_eq!(neg_deinterleave(2, 7, 8), 5);
    }

    #[test]
    fn test_neg_deinterleave_low_ref_branch() {
        // ref_val=2, max=8 (2*ref_val=4 < max).
        assert_eq!(neg_deinterleave(3, 2, 8), 4); // diff<=4, odd: ref + (diff+1)/2
        assert_eq!(neg_deinterleave(4, 2, 8), 0); // diff<=4, even: ref - diff/2
        assert_eq!(neg_deinterleave(5, 2, 8), 5); // diff>4: diff unchanged
    }

    #[test]
    fn test_neg_deinterleave_high_ref_branch() {
        // ref_val=5, max=8 (2*ref_val=10 >= max, ref_val=5 < max-1=7).
        assert_eq!(neg_deinterleave(3, 5, 8), 7); // diff<=4, odd: ref + (diff+1)/2
        assert_eq!(neg_deinterleave(4, 5, 8), 3); // diff<=4, even: ref - diff/2
        assert_eq!(neg_deinterleave(5, 5, 8), 2); // diff>4: max - (diff+1)
    }
}
