//! `palette_mode_info()` (AV1 spec 5.11.46): the palette colour cache and the Y/U/V colour reads.

use super::PaletteInfo;
use crate::symbol::SymbolDecoder;
use bitvue_engine::Result;

/// `floor(log2(x))` for `x >= 1` (dav1d's `ulog2`, used by the palette new-color delta bit-width
/// shrink -- `read_pal_plane_colors`'s doc).
pub(super) fn ulog2(x: u32) -> u32 {
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
pub(super) fn build_pal_cache(
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
pub(in crate::tile::coding_unit) fn read_palette_mode_info(
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
