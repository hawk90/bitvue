//! Displacement vector of an intra-block-copy block (spec 5.11.7 `intra_frame_mode_info`'s
//! `use_intrabc` branch, 7.10.2 / 5.11.26 `assign_mv(1)`)
//!
//! An IntraBC block predicts from already-decoded pixels of the same frame. Its vector starts from
//! the first non-zero entry of the reference-MV stack built for the intra "reference" (the cells
//! other IntraBC blocks left behind), or a default pointing one superblock up/left; the coded
//! difference is added; and the result is then moved so that the source area lies inside the
//! decoded part of the tile. Ported from dav1d's `decode_b` (`src/decode.c`, "intra block copy").
//! Vectors are in 1/8 samples but always whole samples.

use super::read_explicit_mv;
use super::types::MotionVector;
use crate::symbol::SymbolDecoder;
use crate::tile::{MiRect, TileContext};
use bitvue_engine::{BitvueError, Result};

/// Reference-stack entry used when it holds no non-zero vector (dav1d: "default" vector).
fn default_vector(y4: u32, sb128: bool) -> MotionVector {
    let sb = i32::from(sb128);
    if (y4 as i32) - (16 << sb) < 0 {
        MotionVector::new(-(512 << sb) - 2048, 0)
    } else {
        MotionVector::new(0, -(512 << sb))
    }
}

/// What the clipping needs to know about the block and the tile (tile origin is `(0, 0)`).
pub(super) struct ClipGeometry {
    pub x4: u32,
    pub y4: u32,
    pub bw4: u32,
    pub bh4: u32,
    /// The tile's last column, in 4x4 units.
    pub col_end: u32,
    pub sb128: bool,
    /// Sub-8x8 chroma blocks sit on the chroma of a neighbour, which moves the borders by 4.
    pub chroma_border_x: bool,
    pub chroma_border_y: bool,
}

/// Move the vector so its source block lies in the already-decoded part of the tile, exactly as
/// dav1d does. `None` means the vector cannot be made valid (the stream is corrupt).
pub(super) fn clip_to_decoded_area(mv: MotionVector, g: &ClipGeometry) -> Option<MotionVector> {
    let (x4, y4, bw4, bh4) = (g.x4 as i32, g.y4 as i32, g.bw4 as i32, g.bh4 as i32);
    let mut border_left = 0;
    let mut border_top = 0;
    if g.chroma_border_x {
        border_left += 4;
    }
    if g.chroma_border_y {
        border_top += 4;
    }
    let mut src_left = x4 * 4 + (mv.x >> 3);
    let mut src_top = y4 * 4 + (mv.y >> 3);
    let mut src_right = src_left + bw4 * 4;
    let mut src_bottom = src_top + bh4 * 4;
    let border_right = ((g.col_end as i32 + (bw4 - 1)) & !(bw4 - 1)) * 4;

    // Left or right tile boundary.
    if src_left < border_left {
        src_right += border_left - src_left;
        src_left = border_left;
    } else if src_right > border_right {
        src_left -= src_right - border_right;
        src_right = border_right;
    }
    // Top tile boundary.
    if src_top < border_top {
        src_bottom += border_top - src_top;
        src_top = border_top;
    }

    let shift = i32::from(g.sb128);
    let sbx = (x4 >> (4 + shift)) << (6 + shift);
    let sby = (y4 >> (4 + shift)) << (6 + shift);
    let sb_size = 1 << (6 + shift);
    // Overlap with the current superblock: move up into the previous superblock row, or left into
    // the previous superblock, whichever is possible.
    if src_bottom > sby && src_right > sbx {
        if src_top - border_top >= src_bottom - sby {
            src_top -= src_bottom - sby;
            src_bottom = sby;
        } else if src_left - border_left >= src_right - sbx {
            src_left -= src_right - sbx;
            src_right = sbx;
        }
    }
    // Below the current superblock row: move up.
    if src_bottom > sby + sb_size {
        src_top -= src_bottom - (sby + sb_size);
        src_bottom = sby + sb_size;
    }
    // Still overlapping the current superblock.
    if src_bottom > sby && src_right > sbx {
        return None;
    }
    Some(MotionVector::new(
        (src_left - x4 * 4) * 8,
        (src_top - y4 * 4) * 8,
    ))
}

/// Read and resolve the displacement vector of an IntraBC block.
#[allow(clippy::too_many_arguments)]
pub(super) fn read_displacement_vector(
    decoder: &mut SymbolDecoder,
    tile_ctx: &TileContext,
    mi: MiRect,
    col_end: u32,
    sb128: bool,
    has_chroma: bool,
    subsampling: (bool, bool),
) -> Result<MotionVector> {
    let (stack, _) = tile_ctx.single_ref_mv_stack(mi.x4, mi.y4, mi.width, mi.height, -1, false);
    let nonzero = |mv: MotionVector| mv.x != 0 || mv.y != 0;
    let reference = if nonzero(stack[0].mv) {
        stack[0].mv
    } else if nonzero(stack[1].mv) {
        stack[1].mv
    } else {
        default_vector(mi.y4, sb128)
    };
    // Whole-sample precision: no fractional bits are coded (`mv_prec == -1`).
    let diff = read_explicit_mv(decoder, -1)?;
    let mv = MotionVector::new(reference.x + diff.x, reference.y + diff.y);
    let geometry = ClipGeometry {
        x4: mi.x4,
        y4: mi.y4,
        bw4: mi.width,
        bh4: mi.height,
        col_end,
        sb128,
        chroma_border_x: has_chroma && mi.width < 2 && subsampling.0,
        chroma_border_y: has_chroma && mi.height < 2 && subsampling.1,
    };
    clip_to_decoded_area(mv, &geometry).ok_or_else(|| {
        BitvueError::Decode("intra block copy vector overlaps the current superblock".into())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn geometry(x4: u32, y4: u32, bw4: u32, bh4: u32) -> ClipGeometry {
        ClipGeometry {
            x4,
            y4,
            bw4,
            bh4,
            col_end: 80,
            sb128: false,
            chroma_border_x: false,
            chroma_border_y: false,
        }
    }

    #[test]
    fn default_vector_points_a_superblock_up_or_left() {
        // First superblock row: left by a superblock plus 256 samples of reach.
        assert_eq!(default_vector(0, false), MotionVector::new(-512 - 2048, 0));
        assert_eq!(default_vector(15, false), MotionVector::new(-2560, 0));
        // Later rows: straight up by one superblock (512 = 64 samples in 1/8 units).
        assert_eq!(default_vector(16, false), MotionVector::new(0, -512));
        assert_eq!(default_vector(32, true), MotionVector::new(0, -1024));
        assert_eq!(default_vector(31, true), MotionVector::new(-3072, 0));
    }

    #[test]
    fn a_vector_into_the_previous_superblock_row_is_kept() {
        // Block at (64, 64) samples, 16x16; source 64 samples up.
        let g = geometry(16, 16, 4, 4);
        let mv = MotionVector::new(0, -512);
        assert_eq!(clip_to_decoded_area(mv, &g), Some(mv));
    }

    #[test]
    fn a_vector_left_of_the_tile_is_moved_to_the_border() {
        // Source 100 samples left of a block at x=64 starts at -36: clamp to 0.
        let g = geometry(16, 16, 4, 4);
        let got = clip_to_decoded_area(MotionVector::new(-800, -512), &g).unwrap();
        assert_eq!(got, MotionVector::new(-64 * 8, -512));
    }

    #[test]
    fn a_vector_overlapping_the_current_superblock_is_rejected_when_nothing_can_move() {
        // Source is the block itself (zero vector) in the first superblock: no room above/left.
        let g = geometry(0, 0, 4, 4);
        assert_eq!(clip_to_decoded_area(MotionVector::zero(), &g), None);
    }

    #[test]
    fn an_overlapping_source_moves_up_into_the_previous_superblock_row_when_it_fits() {
        // Block at (0,64), 16x16: the zero vector overlaps its own superblock (rows 64..80), so
        // the source moves up by exactly its height, to rows 48..64 of the previous superblock.
        let g = geometry(0, 16, 4, 4);
        let got = clip_to_decoded_area(MotionVector::zero(), &g).unwrap();
        assert_eq!(got, MotionVector::new(0, -16 * 8));
    }

    // ---- read_displacement_vector -------------------------------------------------------

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

    fn mv(x: i32, y: i32) -> MotionVector {
        MotionVector::new(x, y)
    }

    /// A tile of 64x64 units with the IntraBC neighbours `blocks` (x4, y4, w4, h4, vector), all
    /// written as references to the intra "frame" (-1).
    fn tile_with(blocks: &[(u32, u32, u32, u32, MotionVector)]) -> TileContext {
        let mut ctx = TileContext::new(64, 64);
        ctx.set_frame_extent(256, 256);
        ctx.set_tile_extent(0, 64, 0, 64);
        for &(x4, y4, w4, h4, v) in blocks {
            ctx.set_spatial_ref_block(x4, y4, w4, h4, -1, -1, false, v, MotionVector::zero());
        }
        ctx
    }

    const BLOCK: MiRect = MiRect {
        x4: 8,
        y4: 40,
        width: 4,
        height: 4,
    };

    /// The vector a decoder with the same bits ends up with when the reference is `reference`.
    fn expected(data: &[u8], reference: MotionVector, g: ClipGeometry) -> Option<MotionVector> {
        let mut d = SymbolDecoder::new(data).unwrap();
        let diff = read_explicit_mv(&mut d, -1).unwrap();
        clip_to_decoded_area(
            MotionVector::new(reference.x + diff.x, reference.y + diff.y),
            &g,
        )
    }

    fn geometry_of(mi: MiRect) -> ClipGeometry {
        ClipGeometry {
            x4: mi.x4,
            y4: mi.y4,
            bw4: mi.width,
            bh4: mi.height,
            col_end: 64,
            sb128: false,
            chroma_border_x: false,
            chroma_border_y: false,
        }
    }

    fn run(data: &[u8], ctx: &TileContext, mi: MiRect) -> Result<MotionVector> {
        let mut d = SymbolDecoder::new(data).unwrap();
        read_displacement_vector(&mut d, ctx, mi, 64, false, false, (true, true))
    }

    #[test]
    fn the_reference_is_the_first_non_zero_stack_entry_else_the_default_vector() {
        // Above the block (row 38..40): zero vectors; to the left: -64. Heavier zero entry first.
        let zero_first = tile_with(&[(8, 38, 4, 2, mv(0, 0)), (6, 40, 2, 4, mv(-64, 0))]);
        let first_nonzero = tile_with(&[(8, 38, 4, 2, mv(32, -16))]);
        let none = tile_with(&[]);

        let mut seen = std::collections::BTreeSet::new();
        for seed in 1..=24u64 {
            let data = noise(seed, 64);
            for (label, ctx, reference) in [
                ("stack[1] after a zero stack[0]", &zero_first, mv(-64, 0)),
                ("stack[0]", &first_nonzero, mv(32, -16)),
                ("default", &none, default_vector(BLOCK.y4, false)),
            ] {
                let want = expected(&data, reference, geometry_of(BLOCK));
                let got = run(&data, ctx, BLOCK);
                match want {
                    Some(v) => {
                        assert_eq!(got.unwrap(), v, "seed {seed}: {label}");
                        seen.insert((label, v.x, v.y));
                    }
                    None => assert!(got.is_err(), "seed {seed}: {label}"),
                }
            }
        }
        let labels: std::collections::BTreeSet<_> = seen.iter().map(|s| s.0).collect();
        assert_eq!(
            labels.len(),
            3,
            "every reference source must produce an accepted vector: {labels:?}"
        );
    }

    #[test]
    fn a_vector_that_cannot_be_made_valid_is_a_decode_error() {
        // First superblock, nothing above or left to copy from: the default vector plus any small
        // difference still overlaps the block's own superblock.
        let none = tile_with(&[]);
        let mi = MiRect {
            x4: 0,
            y4: 0,
            width: 4,
            height: 4,
        };
        let mut errors = 0;
        for seed in 1..=24u64 {
            let data = noise(seed, 64);
            let want = expected(&data, default_vector(0, false), geometry_of(mi));
            let got = run(&data, &none, mi);
            assert_eq!(got.is_err(), want.is_none(), "seed {seed}");
            errors += usize::from(got.is_err());
        }
        assert!(errors > 0, "some seed must hit the rejection");
    }

    #[test]
    fn narrow_chroma_blocks_move_the_clip_borders_by_four() {
        // A block narrower than 8 samples on an axis that is subsampled, with chroma, starts its
        // source area 4 samples further in on that axis.
        let none = tile_with(&[]);
        let positions = [(1u32, 40u32), (1, 1), (5, 1), (9, 0), (1, 17)];
        let (mut x_moved, mut y_moved) = (false, false);
        for (x4, y4) in positions {
            let narrow = MiRect {
                x4,
                y4,
                width: 1,
                height: 1,
            };
            for seed in 1..=48u64 {
                let data = noise(seed, 64);
                let run_flags = |has_chroma: bool, sub: (bool, bool)| {
                    let mut d = SymbolDecoder::new(&data).unwrap();
                    read_displacement_vector(&mut d, &none, narrow, 64, false, has_chroma, sub).ok()
                };
                let mut d = SymbolDecoder::new(&data).unwrap();
                let diff = read_explicit_mv(&mut d, -1).unwrap();
                let r = default_vector(narrow.y4, false);
                let v = MotionVector::new(r.x + diff.x, r.y + diff.y);
                let clip = |bx: bool, by: bool| {
                    let mut g = geometry_of(narrow);
                    g.chroma_border_x = bx;
                    g.chroma_border_y = by;
                    clip_to_decoded_area(v, &g)
                };
                let label = format!("({x4},{y4}) seed {seed}");
                assert_eq!(
                    run_flags(false, (true, true)),
                    clip(false, false),
                    "{label}: no chroma"
                );
                assert_eq!(
                    run_flags(true, (false, false)),
                    clip(false, false),
                    "{label}: 4:4:4"
                );
                assert_eq!(
                    run_flags(true, (true, false)),
                    clip(true, false),
                    "{label}: x only"
                );
                assert_eq!(
                    run_flags(true, (false, true)),
                    clip(false, true),
                    "{label}: y only"
                );
                assert_eq!(
                    run_flags(true, (true, true)),
                    clip(true, true),
                    "{label}: both"
                );
                x_moved |= clip(true, false) != clip(false, false);
                y_moved |= clip(false, true) != clip(false, false);
            }
        }
        assert!(
            x_moved && y_moved,
            "each border must change some result: x {x_moved} y {y_moved}"
        );
    }
}
