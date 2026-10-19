//! `tx_size()` (spec 5.11.15) for a regular (non-IntraBC) intra block, and the luma transform
//! blocks `residual()` then walks.
//!
//! Transform sizes are rectangular: the largest transform for a block is the block itself, capped
//! at 64 on each axis (`Max_Tx_Size_Rect`), and a coded `tx_depth` halves the larger dimension
//! (both, for a square) that many times. An 8x16 block therefore gets one 8x16 transform, not a
//! 16x16 one; the coefficient CDF family, the `eob` alphabet and the scan all depend on that.
//! Verified against a dav1d trace of the first block of the fixture's key frame (8x16, `tx=7`).

use super::types::{CodingUnit, TxBlock, TxSize};
use crate::frame_header::TxfmMode;
use crate::symbol::cdf::tx_size_class;
use crate::symbol::SymbolDecoder;
use crate::tile::{BlockRect, FrameCodingParams, MiRect, TileContext};
use bitvue_engine::Result;

/// The transform one step smaller (`dav1d_txfm_dimensions[..].sub`): the larger dimension is
/// halved, or both when the transform is square.
fn split_tx(width: u32, height: u32) -> (u32, u32) {
    match width.cmp(&height) {
        std::cmp::Ordering::Equal => (width / 2, height / 2),
        std::cmp::Ordering::Greater => (width / 2, height),
        std::cmp::Ordering::Less => (width, height / 2),
    }
}

/// Reads the block's transform size and fills `cu.tx_size`, `cu.tx_blocks`, and the above/left
/// transform-size context.
pub(super) fn read_intra_tx_size(
    decoder: &mut SymbolDecoder,
    tile_ctx: &mut TileContext,
    rect: BlockRect,
    mi: MiRect,
    frame: &FrameCodingParams,
    cu: &mut CodingUnit,
) -> Result<()> {
    let (mut tx_w, mut tx_h) = if frame.tx_type_flags.coded_lossless {
        (4, 4)
    } else {
        let (max_w, max_h) = (rect.width.min(64), rect.height.min(64));
        match frame.tx_type_flags.txfm_mode {
            TxfmMode::Only4x4 => (4, 4),
            TxfmMode::Largest => (max_w, max_h),
            TxfmMode::Switchable => {
                // Both classes are log2(px / 4); the CDF family is chosen by the larger one and the
                // context compares the neighbours axis by axis.
                let max_class = tx_size_class(max_w.max(max_h)) as u8;
                if max_class > 0 {
                    let ctx = tile_ctx.tx_size_context(
                        mi.x4,
                        mi.y4,
                        tx_size_class(max_w) as u8,
                        tx_size_class(max_h) as u8,
                    );
                    // `read_tx_size` returns the class left after the coded depth.
                    let depth = max_class - decoder.read_tx_size(max_class, ctx)?;
                    let (mut w, mut h) = (max_w, max_h);
                    for _ in 0..depth {
                        (w, h) = split_tx(w, h);
                    }
                    (w, h)
                } else {
                    (max_w, max_h)
                }
            }
        }
    };
    if tx_w == 0 || tx_h == 0 {
        (tx_w, tx_h) = (4, 4);
    }

    cu.tx_size = TxSize::from_class(tx_size_class(tx_w.max(tx_h)) as u8);
    let (w_class, h_class) = (tx_size_class(tx_w) as u8, tx_size_class(tx_h) as u8);
    tile_ctx.set_tx_class(mi.x4, mi.y4, mi.width, mi.height, w_class, h_class);
    // dav1d's intra `set_ctx` writes the same sizes into the var-tx arrays (`edge->tx`) that a
    // later inter block's `txfm_split` context reads.
    tile_ctx.set_var_tx_class(mi.x4, mi.y4, mi.width, mi.height, w_class, h_class);
    cu.tx_blocks = Some(luma_tx_blocks(rect, mi, (tx_w, tx_h), frame));
    Ok(())
}

/// The luma transform blocks of an intra block in decoding order: 64x64 chunks in raster order
/// (only blocks wider or taller than 64 have more than one), and inside each chunk the transform
/// blocks in raster order. Blocks that start outside the frame are not coded.
fn luma_tx_blocks(
    rect: BlockRect,
    mi: MiRect,
    (tx_w, tx_h): (u32, u32),
    frame: &FrameCodingParams,
) -> Vec<TxBlock> {
    let mut blocks = Vec::new();
    for chunk_y in (0..rect.height).step_by(64) {
        for chunk_x in (0..rect.width).step_by(64) {
            let chunk_w = (rect.width - chunk_x).min(64);
            let chunk_h = (rect.height - chunk_y).min(64);
            for ty in (chunk_y..chunk_y + chunk_h).step_by(tx_h as usize) {
                for tx in (chunk_x..chunk_x + chunk_w).step_by(tx_w as usize) {
                    let (x4, y4) = (mi.x4 + tx / 4, mi.y4 + ty / 4);
                    if x4 < frame.mi_cols && y4 < frame.mi_rows {
                        blocks.push(TxBlock {
                            x4,
                            y4,
                            width_px: tx_w,
                            height_px: tx_h,
                        });
                    }
                }
            }
        }
    }
    blocks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splitting_halves_the_larger_dimension_or_both_when_square() {
        assert_eq!(split_tx(64, 64), (32, 32));
        assert_eq!(split_tx(8, 16), (8, 8));
        assert_eq!(split_tx(4, 16), (4, 8));
        assert_eq!(split_tx(64, 16), (32, 16));
        assert_eq!(split_tx(16, 8), (8, 8));
        assert_eq!(split_tx(8, 8), (4, 4));
    }

    #[test]
    fn a_block_wider_than_64_is_walked_in_64x64_chunks() {
        let frame = FrameCodingParams::for_tests();
        let rect = BlockRect {
            x: 0,
            y: 0,
            width: 128,
            height: 64,
        };
        let mi = MiRect {
            x4: 0,
            y4: 0,
            width: 32,
            height: 16,
        };
        let mut frame = frame;
        frame.mi_cols = 64;
        frame.mi_rows = 64;
        // 32x32 transforms: chunk 0 holds (0,0) (32,0) (0,32) (32,32), then chunk 1 follows.
        let blocks = luma_tx_blocks(rect, mi, (32, 32), &frame);
        let origins: Vec<(u32, u32)> = blocks.iter().map(|b| (b.x4, b.y4)).collect();
        assert_eq!(
            origins,
            vec![
                (0, 0),
                (8, 0),
                (0, 8),
                (8, 8),
                (16, 0),
                (24, 0),
                (16, 8),
                (24, 8)
            ]
        );
    }

    #[test]
    fn transform_blocks_starting_outside_the_frame_are_not_coded() {
        let mut frame = FrameCodingParams::for_tests();
        frame.mi_cols = 4; // 16 px wide
        frame.mi_rows = 4;
        let rect = BlockRect {
            x: 0,
            y: 0,
            width: 32,
            height: 32,
        };
        let mi = MiRect {
            x4: 0,
            y4: 0,
            width: 8,
            height: 8,
        };
        let blocks = luma_tx_blocks(rect, mi, (16, 16), &frame);
        assert_eq!(blocks.len(), 1);
        assert_eq!((blocks[0].x4, blocks[0].y4), (0, 0));
    }

    // ---- read_intra_tx_size -------------------------------------------------------------

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

    fn rects(w: u32, h: u32) -> (BlockRect, MiRect) {
        (
            BlockRect {
                x: 0,
                y: 0,
                width: w,
                height: h,
            },
            MiRect {
                x4: 0,
                y4: 0,
                width: w / 4,
                height: h / 4,
            },
        )
    }

    fn frame_with(mode: TxfmMode, coded_lossless: bool) -> FrameCodingParams {
        let mut f = FrameCodingParams::for_tests();
        f.tx_type_flags.txfm_mode = mode;
        f.tx_type_flags.coded_lossless = coded_lossless;
        f.mi_cols = 64;
        f.mi_rows = 64;
        f
    }

    /// Runs `read_intra_tx_size` on a fresh context; returns the unit, the context and the next 24
    /// raw bits the decoder yields (a fingerprint of how many symbols were consumed).
    fn run(
        data: &[u8],
        w: u32,
        h: u32,
        frame: &FrameCodingParams,
    ) -> (CodingUnit, TileContext, u32) {
        let (rect, mi) = rects(w, h);
        let mut decoder = SymbolDecoder::new(data).unwrap();
        let mut ctx = TileContext::new(64, 64);
        let mut cu = CodingUnit::new(0, 0, w, h);
        read_intra_tx_size(&mut decoder, &mut ctx, rect, mi, frame, &mut cu).unwrap();
        let bits = decoder.read_bools_n(24).unwrap();
        (cu, ctx, bits)
    }

    fn dims(cu: &CodingUnit) -> Vec<(u32, u32)> {
        let blocks = cu.tx_blocks.as_ref().expect("tx_blocks is always set");
        blocks.iter().map(|b| (b.width_px, b.height_px)).collect()
    }

    #[test]
    fn lossless_and_only_4x4_modes_use_4x4_transforms_without_reading_a_symbol() {
        let data = noise(1, 32);
        let untouched = SymbolDecoder::new(&data).unwrap().read_bools_n(24).unwrap();
        for (mode, lossless) in [
            (TxfmMode::Switchable, true), // lossless wins over any txfm_mode
            (TxfmMode::Largest, true),
            (TxfmMode::Only4x4, false),
        ] {
            let (cu, _, bits) = run(&data, 16, 8, &frame_with(mode, lossless));
            assert_eq!(cu.tx_size, TxSize::Tx4x4, "{mode:?} {lossless}");
            assert_eq!(dims(&cu), vec![(4, 4); 8], "16x8 block = 4x2 tiles of 4x4");
            assert_eq!(bits, untouched, "{mode:?} {lossless}: nothing was read");
        }
    }

    #[test]
    fn largest_mode_uses_the_block_capped_at_64_without_reading_a_symbol() {
        let data = noise(2, 32);
        let untouched = SymbolDecoder::new(&data).unwrap().read_bools_n(24).unwrap();
        let frame = frame_with(TxfmMode::Largest, false);
        // (block w, h) -> (transform w, h, number of transforms)
        let cases = [
            (16, 8, 16, 8, 1),
            (8, 32, 8, 32, 1),
            (64, 64, 64, 64, 1),
            (128, 64, 64, 64, 2),
            (128, 128, 64, 64, 4),
        ];
        for (w, h, tw, th, n) in cases {
            let (cu, _, bits) = run(&data, w, h, &frame);
            assert_eq!(dims(&cu), vec![(tw, th); n], "{w}x{h}");
            assert_eq!(
                cu.tx_size,
                TxSize::from_class(tx_size_class(tw.max(th)) as u8),
                "{w}x{h}"
            );
            assert_eq!(bits, untouched, "{w}x{h}: nothing was read");
        }
    }

    #[test]
    fn switchable_mode_reads_a_depth_and_halves_the_larger_side_that_many_times() {
        let frame = frame_with(TxfmMode::Switchable, false);
        let mut seen_depths = std::collections::BTreeSet::new();
        for seed in 1..=64u64 {
            let data = noise(seed, 48);
            // Reference: what the decoder reads for a fresh context.
            let (w, h) = (32u32, 16u32);
            let max_class = tx_size_class(w.max(h)) as u8;
            let mut reference = SymbolDecoder::new(&data).unwrap();
            let ctx = TileContext::new(64, 64).tx_size_context(
                0,
                0,
                tx_size_class(w) as u8,
                tx_size_class(h) as u8,
            );
            let depth = max_class - reference.read_tx_size(max_class, ctx).unwrap();
            let after_symbol = reference.read_bools_n(24).unwrap();

            let (cu, _, bits) = run(&data, w, h, &frame);
            let (mut tw, mut th) = (w, h);
            for _ in 0..depth {
                (tw, th) = split_tx(tw, th);
            }
            assert_eq!(dims(&cu)[0], (tw, th), "seed {seed} depth {depth}");
            assert_eq!(
                bits, after_symbol,
                "seed {seed}: exactly one symbol is read"
            );
            seen_depths.insert(depth);
        }
        assert!(
            seen_depths.len() >= 2,
            "seeds must produce different depths: {seen_depths:?}"
        );
    }

    #[test]
    fn a_4x4_block_in_switchable_mode_has_nothing_to_choose_and_reads_nothing() {
        let data = noise(3, 32);
        let untouched = SymbolDecoder::new(&data).unwrap().read_bools_n(24).unwrap();
        let (cu, _, bits) = run(&data, 4, 4, &frame_with(TxfmMode::Switchable, false));
        assert_eq!(cu.tx_size, TxSize::Tx4x4);
        assert_eq!(dims(&cu), vec![(4, 4)]);
        assert_eq!(bits, untouched);
    }

    #[test]
    fn a_zero_sized_block_falls_back_to_4x4_instead_of_stepping_by_zero() {
        let data = noise(4, 32);
        for (w, h) in [(0, 16), (16, 0), (0, 0)] {
            let (rect, mi) = rects(16, 16);
            let rect = BlockRect {
                width: w,
                height: h,
                ..rect
            };
            let mut decoder = SymbolDecoder::new(&data).unwrap();
            let mut ctx = TileContext::new(64, 64);
            let mut cu = CodingUnit::new(0, 0, 16, 16);
            read_intra_tx_size(
                &mut decoder,
                &mut ctx,
                rect,
                mi,
                &frame_with(TxfmMode::Largest, false),
                &mut cu,
            )
            .unwrap();
            assert_eq!(cu.tx_size, TxSize::Tx4x4, "{w}x{h}");
            assert!(
                cu.tx_blocks.as_ref().unwrap().is_empty(),
                "{w}x{h}: no area, no blocks"
            );
        }
    }

    #[test]
    fn the_chosen_size_is_written_to_the_tx_and_var_tx_contexts_of_later_blocks() {
        let data = noise(5, 32);
        let frame = frame_with(TxfmMode::Largest, false);
        let (_, ctx, _) = run(&data, 16, 8, &frame); // 16x8 transform: width class 2, height class 1

        // Above (width) is compared with x4 in 0..4, left (height) with y4 in 0..2.
        assert_eq!(ctx.tx_size_context(0, 0, 2, 1), 2);
        assert_eq!(ctx.tx_size_context(3, 1, 2, 1), 2);
        assert_eq!(
            ctx.tx_size_context(0, 0, 3, 1),
            1,
            "wider than the above neighbour"
        );
        assert_eq!(
            ctx.tx_size_context(0, 0, 2, 2),
            1,
            "taller than the left neighbour"
        );
        assert_eq!(ctx.tx_size_context(0, 0, 3, 2), 0);
        assert_eq!(
            ctx.tx_size_context(4, 2, 0, 0),
            0,
            "outside the block: unwritten"
        );

        // The same sizes feed the later inter block's txfm_split context (a, l = neighbour is
        // smaller than the candidate).
        assert_eq!(ctx.var_tx_context(0, 0, 2, 1), (0, 0));
        assert_eq!(ctx.var_tx_context(0, 0, 3, 2), (1, 1));
        assert_eq!(ctx.var_tx_context(4, 2, 0, 0), (0, 0));
    }
}
