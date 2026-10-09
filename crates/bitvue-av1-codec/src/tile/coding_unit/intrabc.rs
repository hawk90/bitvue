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
}
