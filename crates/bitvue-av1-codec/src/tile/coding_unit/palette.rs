//! `palette_mode_info()` / `palette_tokens()` (AV1 spec 5.11.46, 5.11.49): palette colour cache,
//! colour reads, ordering and the per-pixel colour-index map.

use crate::symbol::SymbolDecoder;
use bitvue_engine::Result;
use serde::{Deserialize, Serialize};

/// Real per-CU palette state from `read_palette_mode_info` (spec 5.11.46) -- `y_size`/`uv_size`
/// `0` when that plane doesn't use palette mode (the common case).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaletteInfo {
    pub y_size: u8,
    pub y_colors: [u16; 8],
    pub uv_size: u8,
    pub u_colors: [u16; 8],
    pub v_colors: [u16; 8],
}

/// `floor(log2(x))` for `x >= 1` (dav1d's `ulog2`, used by the palette new-color delta bit-width
/// shrink -- `read_pal_plane_colors`'s doc).
fn ulog2(x: u32) -> u32 {
    31 - x.max(1).leading_zeros()
}

/// Real palette color-cache sorted merge (spec 5.11.46, ported from dav1d's `read_pal_plane`'s
/// cache-building loop, `src/recon_tmpl.c`) -- merges the above/left neighbors' already-decoded
/// palette colors into one deduplicated, ascending `cache` (real spec: this determines which
/// colors are *offered* for reuse, not their bit cost -- the bit cost is exactly `n_cache`
/// booleans read at the call site regardless of what's in the cache, so getting the cache
/// *contents* wrong doesn't desync, only which colors get reused vs. re-signaled -- but see
/// `read_pal_plane_colors`'s doc for why `n_cache` itself, and thus bit *position*, does depend on
/// getting the SB64-boundary `above_count` masking right).
fn build_pal_cache(
    above_colors: [u16; 8],
    above_count: u8,
    left_colors: [u16; 8],
    left_count: u8,
) -> ([u16; 16], usize) {
    let mut cache = [0u16; 16];
    let mut n_cache = 0usize;
    let (mut li, mut lc) = (0usize, left_count as usize);
    let (mut ai, mut ac) = (0usize, above_count as usize);

    while lc > 0 && ac > 0 {
        let (lv, av) = (left_colors[li], above_colors[ai]);
        if lv < av {
            if n_cache == 0 || cache[n_cache - 1] != lv {
                cache[n_cache] = lv;
                n_cache += 1;
            }
            li += 1;
            lc -= 1;
        } else {
            if av == lv {
                li += 1;
                lc -= 1;
            }
            if n_cache == 0 || cache[n_cache - 1] != av {
                cache[n_cache] = av;
                n_cache += 1;
            }
            ai += 1;
            ac -= 1;
        }
    }
    while lc > 0 {
        let lv = left_colors[li];
        if n_cache == 0 || cache[n_cache - 1] != lv {
            cache[n_cache] = lv;
            n_cache += 1;
        }
        li += 1;
        lc -= 1;
    }
    while ac > 0 {
        let av = above_colors[ai];
        if n_cache == 0 || cache[n_cache - 1] != av {
            cache[n_cache] = av;
            n_cache += 1;
        }
        ai += 1;
        ac -= 1;
    }

    (cache, n_cache)
}

/// Real palette color read for the Y or U plane (spec 5.11.46, ported from dav1d's
/// `read_pal_plane`, `src/recon_tmpl.c`) -- V has its own separate encoding (`read_pal_v_colors`).
/// Returns the real decoded `(pal_sz, colors)` (`colors[0..pal_sz]` valid ascending, rest `0`).
///
/// `above_count`'s real dav1d/spec quirk: cache reuse against the *above* neighbor is only
/// allowed when this CU's `y4` isn't 64px-row-aligned ("don't reuse above palette outside SB64
/// boundaries", verified against dav1d's source comment directly, not reinterpreted) -- ported
/// exactly since this genuinely gates how many cache-reuse booleans get read (`n_cache`), i.e.
/// real bitstream *position*, not just which colors get offered for reuse.
#[allow(clippy::too_many_arguments)]
fn read_pal_plane_colors(
    decoder: &mut SymbolDecoder,
    tile_ctx: &mut crate::tile::TileContext,
    color_plane: usize,
    size_plane: usize,
    cdf_plane: usize,
    x4: u32,
    y4: u32,
    bsize_ctx: u8,
) -> Result<(u8, [u16; 8])> {
    let pal_sz = decoder.read_pal_size(cdf_plane, bsize_ctx)?;

    let (left_colors, left_count) = tile_ctx.pal_left(color_plane, size_plane, y4);
    let (above_colors, above_count_raw) = tile_ctx.pal_above(color_plane, size_plane, x4);
    let above_count = if !y4.is_multiple_of(16) {
        above_count_raw
    } else {
        0
    };
    let (cache, n_cache) = build_pal_cache(above_colors, above_count, left_colors, left_count);

    let mut used_cache = [0u16; 8];
    let mut n_used_cache = 0usize;
    for &c in cache.iter().take(n_cache) {
        if n_used_cache >= pal_sz as usize {
            break;
        }
        if decoder.read_bool_equi()? {
            used_cache[n_used_cache] = c;
            n_used_cache += 1;
        }
    }

    let mut new_entries = [0u16; 8];
    let mut n_new = 0usize;
    if n_used_cache < pal_sz as usize {
        let not_pl = if color_plane == 0 { 1u32 } else { 0u32 };
        let max = 255u32;
        let mut prev = decoder.read_bools_n(8)?;
        new_entries[0] = prev as u16;
        n_new = 1;
        if n_used_cache + n_new < pal_sz as usize {
            let mut bits = 8 - 3 + decoder.read_bools_n(2)?;
            loop {
                let delta = decoder.read_bools_n(bits)?;
                prev = (prev + delta + not_pl).min(max);
                new_entries[n_new] = prev as u16;
                n_new += 1;
                if prev + not_pl >= max {
                    for slot in new_entries
                        .iter_mut()
                        .take(pal_sz as usize - n_used_cache)
                        .skip(n_new)
                    {
                        *slot = max as u16;
                    }
                    n_new = pal_sz as usize - n_used_cache;
                    break;
                }
                if n_used_cache + n_new >= pal_sz as usize {
                    break;
                }
                bits = bits.min(1 + ulog2(max - prev - not_pl));
            }
        }
    }

    let mut colors = [0u16; 8];
    let (mut ci, mut ni) = (0usize, 0usize);
    for slot in colors.iter_mut().take(pal_sz as usize) {
        *slot = if ci < n_used_cache && (ni >= n_new || used_cache[ci] <= new_entries[ni]) {
            let v = used_cache[ci];
            ci += 1;
            v
        } else {
            let v = new_entries[ni];
            ni += 1;
            v
        };
    }

    Ok((pal_sz, colors))
}

/// Real V-plane palette color read (spec 5.11.46, ported from dav1d's `read_pal_uv`'s V-specific
/// tail, `src/recon_tmpl.c`) -- genuinely different scheme from Y/U: no color cache, a real
/// `delta_encode_palette_colors_v` flag choosing between a signed-delta chain (wrapping `& max`,
/// not clamping -- unlike Y/U) or fully-literal per-entry colors.
fn read_pal_v_colors(decoder: &mut SymbolDecoder, pal_sz: u8) -> Result<[u16; 8]> {
    let mut colors = [0u16; 8];
    let max = 255i32;
    if decoder.read_bool_equi()? {
        let bits = 8 - 4 + decoder.read_bools_n(2)?;
        let mut prev = decoder.read_bools_n(8)? as i32;
        colors[0] = prev as u16;
        for slot in colors.iter_mut().take(pal_sz as usize).skip(1) {
            let mut delta = decoder.read_bools_n(bits)? as i32;
            if delta != 0 && decoder.read_bool_equi()? {
                delta = -delta;
            }
            prev = (prev + delta) & max;
            *slot = prev as u16;
        }
    } else {
        for slot in colors.iter_mut().take(pal_sz as usize) {
            *slot = decoder.read_bools_n(8)? as u16;
        }
    }
    Ok(colors)
}

/// Real `palette_mode_info()` (spec 5.11.46) -- Y colors (only when `y_mode_is_dc`, real spec:
/// palette only ever applies to `DC_PRED` blocks), then UV colors (`has_chroma && uv_mode_is_dc`).
/// `bsize_ctx`: `Mi_Width_Log2 + Mi_Height_Log2 - 2` (real spec formula -- callers gate on the
/// real eligibility range, block width/height both `8..=64`, which keeps `bsize_ctx` in the real
/// `0..=6` CDF range). Always writes real (possibly all-zero) state to `tile_ctx`'s palette
/// context arrays regardless of whether palette was actually used, matching dav1d's own
/// unconditional `copy_pal_block_*` call sites.
#[allow(clippy::too_many_arguments)]
pub(super) fn read_palette_mode_info(
    decoder: &mut SymbolDecoder,
    tile_ctx: &mut crate::tile::TileContext,
    x4: u32,
    y4: u32,
    width_4x4: u32,
    height_4x4: u32,
    bsize_ctx: u8,
    y_mode_is_dc: bool,
    has_chroma: bool,
    uv_mode_is_dc: bool,
) -> Result<PaletteInfo> {
    let mut info = PaletteInfo::default();

    if y_mode_is_dc {
        let ctx = tile_ctx.has_palette_y_context(x4, y4);
        if decoder.read_has_palette_y(bsize_ctx, ctx)? {
            let (sz, colors) =
                read_pal_plane_colors(decoder, tile_ctx, 0, 0, 0, x4, y4, bsize_ctx)?;
            info.y_size = sz;
            info.y_colors = colors;
        }
    }
    tile_ctx.set_pal_size(0, x4, y4, width_4x4, height_4x4, info.y_size);
    tile_ctx.set_pal_colors(0, x4, y4, width_4x4, height_4x4, info.y_colors);

    if has_chroma && uv_mode_is_dc {
        let ctx = u8::from(info.y_size > 0);
        if decoder.read_has_palette_uv(ctx)? {
            let (sz, u_colors) =
                read_pal_plane_colors(decoder, tile_ctx, 1, 1, 1, x4, y4, bsize_ctx)?;
            info.uv_size = sz;
            info.u_colors = u_colors;
            info.v_colors = read_pal_v_colors(decoder, sz)?;
        }
    }
    tile_ctx.set_pal_size(1, x4, y4, width_4x4, height_4x4, info.uv_size);
    tile_ctx.set_pal_colors(1, x4, y4, width_4x4, height_4x4, info.u_colors);
    tile_ctx.set_pal_colors(2, x4, y4, width_4x4, height_4x4, info.v_colors);

    Ok(info)
}

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
pub(super) fn read_palette_tokens(
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
