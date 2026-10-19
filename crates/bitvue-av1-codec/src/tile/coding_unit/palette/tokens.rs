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

#[cfg(test)]
mod tests {
    use super::*;

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

    /// Spec 7.11.4 `get_palette_color_context` for the pixel at `(r, c)`, over the full 8-colour
    /// alphabet: returns `(ColorOrder, ColorContext)`.
    fn spec_context(map: &[u8], stride: usize, r: usize, c: usize) -> ([u8; 8], u8) {
        const MULTIPLIERS: [u32; 3] = [1, 2, 2];
        const CONTEXT: [i8; 9] = [-1, -1, 0, -1, -1, 4, 3, 2, 1];
        let mut scores = [0u32; 8];
        let mut order: [u8; 8] = [0, 1, 2, 3, 4, 5, 6, 7];
        if c > 0 {
            scores[map[r * stride + c - 1] as usize] += 2;
        }
        if r > 0 && c > 0 {
            scores[map[(r - 1) * stride + c - 1] as usize] += 1;
        }
        if r > 0 {
            scores[map[(r - 1) * stride + c] as usize] += 2;
        }
        for i in 0..3 {
            let (mut best, mut best_idx) = (scores[i], i);
            for j in i + 1..8 {
                if scores[j] > best {
                    best = scores[j];
                    best_idx = j;
                }
            }
            if best_idx != i {
                let (s, o) = (scores[best_idx], order[best_idx]);
                for k in (i + 1..=best_idx).rev() {
                    scores[k] = scores[k - 1];
                    order[k] = order[k - 1];
                }
                scores[i] = s;
                order[i] = o;
            }
        }
        let hash: u32 = (0..3).map(|i| scores[i] * MULTIPLIERS[i]).sum();
        (order, CONTEXT[hash as usize] as u8)
    }

    fn random_map(seed: u64, len: usize, alphabet: u8) -> Vec<u8> {
        noise(seed, len).iter().map(|b| b % alphabet).collect()
    }

    #[test]
    fn order_palette_matches_the_specs_colour_context_for_every_pixel() {
        let mut contexts = std::collections::BTreeSet::new();
        // (width, height, stride): square, wide, tall, and a stride wider than the block.
        for &(w, h, stride) in &[
            (8usize, 8usize, 8usize),
            (16, 4, 16),
            (4, 16, 4),
            (8, 8, 11),
            (4, 4, 4),
        ] {
            for alphabet in [2u8, 3, 8] {
                for seed in 1..=6u64 {
                    let map = random_map(seed * 31 + alphabet as u64, stride * h, alphabet);
                    for i in 1..(w + h - 1) {
                        let first = i.min(w - 1);
                        let last = i.saturating_sub(h - 1);
                        let (order, ctx) = order_palette(&map, stride, i, first, last);
                        assert_eq!(order.len(), first - last + 1);
                        for (n, j) in (last..=first).rev().enumerate() {
                            let (want_order, want_ctx) = spec_context(&map, stride, i - j, j);
                            assert_eq!(ctx[n], want_ctx, "{w}x{h}/{stride} diag {i} col {j}");
                            assert_eq!(order[n], want_order, "{w}x{h}/{stride} diag {i} col {j}");
                            contexts.insert(ctx[n]);
                        }
                    }
                }
            }
        }
        assert_eq!(
            contexts.into_iter().collect::<Vec<_>>(),
            vec![0, 1, 2, 3, 4]
        );
    }

    #[test]
    fn order_palette_puts_the_only_available_neighbour_first() {
        // Column 0 has only a top neighbour, row 0 only a left one: context 0 either way.
        let map: Vec<u8> = (0..16).map(|i| (i % 8) as u8).collect(); // 4x4, stride 4
        let (order, ctx) = order_palette(&map, 4, 1, 1, 0);
        // Diagonal 1: j = 1 (row 0, left neighbour map[0]) then j = 0 (row 1, top neighbour map[0]).
        assert_eq!(ctx, vec![0, 0]);
        assert_eq!(order[0][0], map[0]);
        assert_eq!(order[1][0], map[0]);
        for row in &order {
            let mut sorted = *row;
            sorted.sort_unstable();
            assert_eq!(
                sorted,
                [0, 1, 2, 3, 4, 5, 6, 7],
                "each row is a permutation"
            );
        }
    }

    #[test]
    fn read_pal_indices_decodes_through_the_specs_orders_and_contexts() {
        for &(pal_sz, w4, h4, bw4) in &[
            (2u8, 2u32, 2u32, 2u32),
            (8, 3, 2, 4),
            (5, 1, 3, 1),
            (3, 4, 4, 4),
        ] {
            for seed in 1..=8u64 {
                let data = noise(seed, 1024);
                let mut d = SymbolDecoder::new(&data).unwrap();
                let got = read_pal_indices(&mut d, 0, pal_sz, w4, h4, bw4).unwrap();
                let (w, h) = ((w4 * 4) as usize, (h4 * 4) as usize);
                assert_eq!(got.len(), w * h);
                assert!(
                    got.iter().all(|&c| c < pal_sz),
                    "indices stay inside the palette"
                );

                // Reference decode with the spec's contexts, into a scratch map with the
                // nominal stride.
                let stride = ((bw4 * 4) as usize).max(w);
                let mut r = SymbolDecoder::new(&data).unwrap();
                let mut map = vec![0u8; stride * h];
                map[0] = r.read_uniform(pal_sz as u32).unwrap() as u8;
                for i in 1..(w + h - 1) {
                    let first = i.min(w - 1);
                    let last = i.saturating_sub(h - 1);
                    for j in (last..=first).rev() {
                        let (order, ctx) = spec_context(&map, stride, i - j, j);
                        let k = r.read_color_map_index(0, pal_sz, ctx).unwrap();
                        map[(i - j) * stride + j] = order[k as usize];
                    }
                }
                let want: Vec<u8> = (0..h)
                    .flat_map(|y| map[y * stride..y * stride + w].to_vec())
                    .collect();
                assert_eq!(got, want, "pal {pal_sz} {w4}x{h4} bw4 {bw4} seed {seed}");
                assert_eq!(
                    d.read_bools_n(24).unwrap(),
                    r.read_bools_n(24).unwrap(),
                    "same number of symbols consumed"
                );
            }
        }
    }

    #[test]
    fn a_one_pixel_wide_map_is_only_the_first_uniform_read_plus_the_column() {
        // w4 = h4 = 1 is the smallest block: a 4x4 map, every pixel after the first via color_map.
        let data = noise(9, 256);
        let mut d = SymbolDecoder::new(&data).unwrap();
        let map = read_pal_indices(&mut d, 1, 4, 1, 1, 1).unwrap();
        assert_eq!(map.len(), 16);
        assert!(map.iter().all(|&c| c < 4));
    }

    fn palette(y: u8, uv: u8) -> PaletteInfo {
        PaletteInfo {
            y_size: y,
            uv_size: uv,
            ..PaletteInfo::default()
        }
    }

    #[test]
    fn tokens_are_read_per_plane_and_only_for_planes_with_a_palette() {
        let data = noise(13, 2048);
        let untouched = SymbolDecoder::new(&data).unwrap().read_bools_n(24).unwrap();
        let run = |p: &PaletteInfo, chroma: bool| {
            let mut d = SymbolDecoder::new(&data).unwrap();
            let maps = read_palette_tokens(&mut d, 0, 0, 4, 4, chroma, p, 16, 16).unwrap();
            (maps, d.read_bools_n(24).unwrap())
        };
        let ((y, uv), bits) = run(&palette(0, 0), true);
        assert!(y.is_none() && uv.is_none());
        assert_eq!(bits, untouched, "no palette, no symbols");

        let ((y, uv), _) = run(&palette(4, 0), true);
        assert_eq!(y.unwrap().len(), 16 * 16);
        assert!(uv.is_none(), "uv_size 0");

        let ((y, uv), _) = run(&palette(0, 3), true);
        assert!(y.is_none());
        assert_eq!(uv.unwrap().len(), 8 * 8, "4:2:0 chroma is half size");

        let ((y, uv), _) = run(&palette(4, 3), false);
        assert!(y.is_some() && uv.is_none(), "no chroma for this block");
    }

    #[test]
    fn the_visible_area_is_clamped_to_the_frame_and_chroma_rounds_up() {
        let data = noise(17, 4096);
        // 16x16-unit block at x4 = 13 in a frame 16 units wide, 9 rows tall at y4 = 6: 3 x 3 visible.
        let mut d = SymbolDecoder::new(&data).unwrap();
        let (y, uv) =
            read_palette_tokens(&mut d, 13, 6, 16, 16, true, &palette(4, 4), 9, 16).unwrap();
        assert_eq!(y.unwrap().len(), 12 * 12, "3x3 units of 4 px");
        assert_eq!(
            uv.unwrap().len(),
            8 * 8,
            "ceil(3/2) = 2 units of 4 px per side"
        );

        // A block starting at or past the edge still decodes the minimum 1x1 unit.
        let mut d = SymbolDecoder::new(&data).unwrap();
        let (y, _) =
            read_palette_tokens(&mut d, 20, 30, 4, 4, false, &palette(2, 0), 9, 16).unwrap();
        assert_eq!(y.unwrap().len(), 16);
    }
}
