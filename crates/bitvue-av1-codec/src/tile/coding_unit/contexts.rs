//! Pure derivations used by `parse_coding_unit`: symbol-to-mode mappings, block-size/context
//! lookups and spec eligibility predicates. None of these read the bitstream.

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
