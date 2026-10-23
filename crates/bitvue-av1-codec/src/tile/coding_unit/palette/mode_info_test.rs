//! `read_palette_mode_info` and its helpers pinned: `build_pal_cache` on hand-built neighbours,
//! and seeded sweeps that read many blocks in a row from one tile context (so the neighbour
//! palettes, the colour cache and the 64-row above-cache rule all take part) and hash everything
//! read.

use super::mode_info::{
    build_pal_cache, merge_sorted_colors, narrowed_delta_bits, read_palette_mode_info, ulog2,
};
use crate::symbol::SymbolDecoder;
use crate::tile::TileContext;

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

fn fnv(hash: &mut u64, bytes: &[u8]) {
    for b in bytes {
        *hash ^= u64::from(*b);
        *hash = hash.wrapping_mul(0x100000001b3);
    }
}

/// Blocks of 2x2 units on a grid whose rows include 0, 15, 16 and 17 (the 64-row boundary the
/// above cache stops at), all in DC prediction with chroma.
fn sweep(seeds: std::ops::RangeInclusive<u64>) -> (u64, usize, usize) {
    let mut hash = 0xcbf29ce484222325u64;
    let (mut with_y, mut with_uv) = (0, 0);
    for seed in seeds {
        let bytes = noise(seed, 400);
        let mut decoder = SymbolDecoder::new(&bytes).unwrap();
        let mut ctx = TileContext::new(64, 64);
        'blocks: for y4 in [0u32, 2, 14, 16, 18] {
            for x4 in [0u32, 2, 4, 6] {
                let bsize_ctx = ((x4 + y4) % 7) as u8;
                match read_palette_mode_info(
                    &mut decoder,
                    &mut ctx,
                    x4,
                    y4,
                    2,
                    2,
                    bsize_ctx,
                    true,
                    true,
                    true,
                ) {
                    Ok(info) => {
                        with_y += usize::from(info.y_size > 0);
                        with_uv += usize::from(info.uv_size > 0);
                        fnv(&mut hash, format!("{info:?}").as_bytes());
                    }
                    Err(e) => {
                        fnv(&mut hash, format!("{e:?}").as_bytes());
                        break 'blocks;
                    }
                }
            }
        }
        fnv(
            &mut hash,
            &decoder.read_bools_n(24).unwrap_or(0).to_le_bytes(),
        );
    }
    (hash, with_y, with_uv)
}

#[test]
fn the_sweep_reads_palettes_in_both_planes() {
    let (_, with_y, with_uv) = sweep(1..=300);
    assert!(with_y > 50, "luma palettes read: {with_y}");
    assert!(with_uv > 20, "chroma palettes read: {with_uv}");
}

#[test]
fn palette_reads_over_a_seeded_sweep_are_pinned() {
    assert_eq!(sweep(1..=300).0, 8510894113538728293);
}

#[test]
fn the_cache_merges_two_ascending_lists_without_duplicates() {
    let (cache, n) = build_pal_cache(
        [10, 20, 30, 0, 0, 0, 0, 0],
        3,
        [5, 20, 40, 0, 0, 0, 0, 0],
        3,
    );
    assert_eq!(&cache[..n], &[5, 10, 20, 30, 40]);
}

#[test]
fn the_cache_of_one_neighbour_is_that_neighbour_and_of_none_is_empty() {
    let (cache, n) = build_pal_cache([7, 9, 0, 0, 0, 0, 0, 0], 2, [0; 8], 0);
    assert_eq!(&cache[..n], &[7, 9]);
    let (cache, n) = build_pal_cache([0; 8], 0, [3, 3, 4, 0, 0, 0, 0, 0], 3);
    assert_eq!(&cache[..n], &[3, 4]);
    assert_eq!(build_pal_cache([0; 8], 0, [0; 8], 0).1, 0);
}

#[test]
fn ulog2_is_the_floor_of_the_base_two_logarithm_and_zero_counts_as_one() {
    let got: Vec<u32> = [0u32, 1, 2, 3, 4, 7, 8, 255, 256]
        .iter()
        .map(|&x| ulog2(x))
        .collect();
    assert_eq!(got, vec![0, 0, 1, 1, 2, 2, 3, 7, 8]);
}

#[test]
fn merging_interleaves_the_cache_colours_and_the_new_ones_in_ascending_order() {
    assert_eq!(
        merge_sorted_colors(&[10, 40], &[5, 20, 50], 5),
        [5, 10, 20, 40, 50, 0, 0, 0]
    );
}

#[test]
fn merging_lets_the_cache_colour_win_a_tie_and_keeps_both() {
    assert_eq!(
        merge_sorted_colors(&[7], &[7, 9], 3),
        [7, 7, 9, 0, 0, 0, 0, 0]
    );
}

#[test]
fn merging_with_one_side_empty_gives_the_other_and_stops_at_the_palette_size() {
    assert_eq!(
        merge_sorted_colors(&[], &[1, 2, 3], 3),
        [1, 2, 3, 0, 0, 0, 0, 0]
    );
    assert_eq!(
        merge_sorted_colors(&[4, 5], &[], 2),
        [4, 5, 0, 0, 0, 0, 0, 0]
    );
    assert_eq!(
        merge_sorted_colors(&[1, 2, 3], &[4, 5], 2),
        [1, 2, 0, 0, 0, 0, 0, 0]
    );
}

#[test]
fn the_delta_width_shrinks_with_the_room_left_below_255() {
    // width = 1 + floor(log2(255 - prev - not_pl)), worked out by hand for each `prev`.
    let widths = |not_pl| -> Vec<u32> {
        [0u32, 100, 126, 127, 190, 191, 222, 238, 246, 250, 252, 253]
            .iter()
            .map(|&prev| narrowed_delta_bits(8, prev, not_pl))
            .collect()
    };
    assert_eq!(widths(0), vec![8, 8, 8, 8, 7, 7, 6, 5, 4, 3, 2, 2]);
    // The Y plane's deltas are at least 1, so its room is one smaller.
    assert_eq!(widths(1), vec![8, 8, 8, 7, 7, 6, 6, 5, 4, 3, 2, 1]);
}

#[test]
fn the_delta_width_never_grows() {
    assert_eq!(narrowed_delta_bits(3, 0, 0), 3);
    assert_eq!(narrowed_delta_bits(3, 200, 0), 3);
    assert_eq!(narrowed_delta_bits(3, 253, 1), 1);
}
