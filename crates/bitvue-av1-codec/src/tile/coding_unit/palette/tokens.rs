//! `palette_tokens()` (AV1 spec 5.11.49): colour ordering and the per-pixel colour-index map.

use super::PaletteInfo;
use crate::symbol::SymbolDecoder;
use bitvue_engine::Result;

/// Write one resolved color index into `row`/`o_idx`/`mask` -- shared by every branch of
/// `order_palette`'s neighbor-agreement decision tree (spec/dav1d's `add()` macro,
/// `src/decode.c`).
fn push_pal_order_entry(v: u8, row: &mut [u8; 8], o_idx: &mut usize, mask: &mut u32) {
    row[*o_idx] = v;
    *o_idx += 1;
    *mask |= 1 << v;
}

/// Real spec/dav1d `order_palette` (`src/decode.c`) -- for one anti-diagonal `i` of the wavefront
/// scan (`i - j` = row, `j` = column, `j` ranging `last..=first`), derives each pixel's real
/// above/left/above-left neighbor-agreement CONTEXT (0..=4) plus a real per-pixel 8-entry `order`
/// permutation (already-seen neighbor colors first, by agreement rank, then every remaining color
/// 0..=7 in ascending order) that the just-decoded `color_map` symbol indexes into to recover the
/// real absolute color index. Ported exactly (including the specific iteration/increment order
/// this depends on -- `pos` advances by `stride - 1` per step, not `stride`, since each step moves
/// one row down AND one column left along the anti-diagonal), not reconstructed from spec
/// pseudocode alone.
fn order_palette(
    pal_tmp: &[u8],
    stride: usize,
    i: usize,
    first: usize,
    last: usize,
) -> (Vec<[u8; 8]>, Vec<u8>) {
    let n = first - last + 1;
    let mut order = vec![[0u8; 8]; n];
    let mut ctx = vec![0u8; n];
    let mut have_top = i > first;
    let mut pos = first + (i - first) * stride;

    for n_idx in 0..n {
        let j = first - n_idx;
        let have_left = j > 0;
        let mut mask: u32 = 0;
        let mut o_idx: usize = 0;
        let row = &mut order[n_idx];

        if !have_left {
            ctx[n_idx] = 0;
            push_pal_order_entry(pal_tmp[pos - stride], row, &mut o_idx, &mut mask);
        } else if !have_top {
            ctx[n_idx] = 0;
            push_pal_order_entry(pal_tmp[pos - 1], row, &mut o_idx, &mut mask);
        } else {
            let l = pal_tmp[pos - 1];
            let t = pal_tmp[pos - stride];
            let tl = pal_tmp[pos - stride - 1];
            let same_t_l = t == l;
            let same_t_tl = t == tl;
            let same_l_tl = l == tl;
            if same_t_l && same_t_tl && same_l_tl {
                ctx[n_idx] = 4;
                push_pal_order_entry(t, row, &mut o_idx, &mut mask);
            } else if same_t_l {
                ctx[n_idx] = 3;
                push_pal_order_entry(t, row, &mut o_idx, &mut mask);
                push_pal_order_entry(tl, row, &mut o_idx, &mut mask);
            } else if same_t_tl || same_l_tl {
                ctx[n_idx] = 2;
                push_pal_order_entry(tl, row, &mut o_idx, &mut mask);
                push_pal_order_entry(if same_t_tl { l } else { t }, row, &mut o_idx, &mut mask);
            } else {
                ctx[n_idx] = 1;
                push_pal_order_entry(l.min(t), row, &mut o_idx, &mut mask);
                push_pal_order_entry(l.max(t), row, &mut o_idx, &mut mask);
                push_pal_order_entry(tl, row, &mut o_idx, &mut mask);
            }
        }

        for bit in 0..8u8 {
            if mask & (1 << bit) == 0 {
                row[o_idx] = bit;
                o_idx += 1;
            }
        }
        debug_assert_eq!(o_idx, 8);

        have_top = true;
        pos += stride - 1;
    }
    debug_assert!(have_top || n == 0);

    (order, ctx)
}

/// Real spec/dav1d `read_pal_indices` (`src/decode.c`) -- reads the full per-pixel palette
/// color-index map for one plane via the diagonal wavefront scan: the first pixel is a direct
/// uniform `NS(pal_sz)` read (`SymbolDecoder::read_uniform`), every subsequent pixel is a
/// real-context `color_map` symbol (`order_palette`'s doc) re-mapped through that diagonal's
/// `order[]` permutation back into an absolute color index (0..pal_sz-1).
///
/// `w4`/`h4`: real spec/dav1d frame-edge-clamped VISIBLE width/height in 4-pixel units (this
/// crate's `mi_rows`/`mi_cols` machinery -- already used by `compute_inter_tx_blocks` for the same
/// reason -- NOT the CU's own nominal `width_4x4`/`height_4x4`, which can extend past the frame
/// edge for an edge CU). `bw4`: the CU's own nominal width in 4-pixel units, used only for the
/// scratch buffer's `stride` (dav1d: `t->scratch.pal_idx_{y,uv}`'s row stride is the nominal block
/// width even though only the visible sub-rectangle is ever read/written).
///
/// Returns a `w4*4 x h4*4` row-major index map (stride `w4*4`, i.e. already cropped to the visible
/// rectangle) -- for `plane=1` (chroma), this single map is shared by BOTH U and V (real spec: one
/// index map indexes into two separate color palettes).
fn read_pal_indices(
    decoder: &mut SymbolDecoder,
    plane: usize,
    pal_sz: u8,
    w4: u32,
    h4: u32,
    bw4: u32,
) -> Result<Vec<u8>> {
    let (w, h) = (w4 * 4, h4 * 4);
    let stride = (bw4 * 4).max(w) as usize;
    let mut pal_tmp = vec![0u8; stride * h as usize];

    pal_tmp[0] = decoder.read_uniform(pal_sz as u32)? as u8;

    let bound = 4 * (w4 as i64 + h4 as i64) - 1;
    for i in 1..bound.max(1) {
        let first = i.min(w as i64 - 1) as usize;
        let last = (i - (h as i64 - 1)).max(0) as usize;
        let (order, ctx) = order_palette(&pal_tmp, stride, i as usize, first, last);
        for (m, j) in (last..=first).rev().enumerate() {
            let color_idx = decoder.read_color_map_index(plane, pal_sz, ctx[m])?;
            pal_tmp[(i as usize - j) * stride + j] = order[m][color_idx as usize];
        }
    }

    let mut out = vec![0u8; (w * h) as usize];
    for row in 0..h as usize {
        out[row * w as usize..(row + 1) * w as usize]
            .copy_from_slice(&pal_tmp[row * stride..row * stride + w as usize]);
    }
    Ok(out)
}

/// Real per-plane palette-token read for one CU (spec: `Y` when `PaletteSizeY > 0`, then `UV`
/// -- shared U/V index map -- when `has_chroma && PaletteSizeUV > 0`) -- wraps `read_pal_indices`
/// with the real frame-edge-clamped `w4`/`h4` computation (`mi_rows`/`mi_cols`, same reasoning as
/// `compute_inter_tx_blocks`) for luma, then the real 4:2:0 chroma-subsampled equivalent
/// (`cw4 = (w4+1)>>1` etc, dav1d's own formula for `ss_hor=ss_ver=1`) for chroma. Returns
/// `(y_index_map, uv_index_map)`, each `None` when that plane's palette size is `0`.
/// `(y_index_map, uv_index_map)` returned by [`read_palette_tokens`]; each entry is `None` when
/// that plane's palette size is `0`.
type PaletteTokenMaps = (Option<Vec<u8>>, Option<Vec<u8>>);

#[allow(clippy::too_many_arguments)]
pub(in crate::tile::coding_unit) fn read_palette_tokens(
    decoder: &mut SymbolDecoder,
    x4: u32,
    y4: u32,
    width_4x4: u32,
    height_4x4: u32,
    has_chroma: bool,
    palette: &PaletteInfo,
    mi_rows: u32,
    mi_cols: u32,
) -> Result<PaletteTokenMaps> {
    let w4 = width_4x4.min(mi_cols.saturating_sub(x4)).max(1);
    let h4 = height_4x4.min(mi_rows.saturating_sub(y4)).max(1);

    let y_map = if palette.y_size > 0 {
        Some(read_pal_indices(
            decoder,
            0,
            palette.y_size,
            w4,
            h4,
            width_4x4,
        )?)
    } else {
        None
    };

    let uv_map = if has_chroma && palette.uv_size > 0 {
        let (cw4, ch4) = (w4.div_ceil(2), h4.div_ceil(2));
        let cbw4 = width_4x4.div_ceil(2);
        Some(read_pal_indices(
            decoder,
            1,
            palette.uv_size,
            cw4,
            ch4,
            cbw4,
        )?)
    } else {
        None
    };

    Ok((y_map, uv_map))
}
