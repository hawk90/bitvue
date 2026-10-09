//! Whether an edge neighbour of a block uses a given single reference (dav1d `find_matching_ref`).

use super::*;

impl SpatialRefContext {
    /// Whether any decoded neighbour on the block's top or left edge (or its top-left/top-right
    /// corner) is a single-reference block using `ref0` -- the `mask[0] | mask[1] != 0` outcome of
    /// dav1d's `find_matching_ref` (`src/decode.c`), which decides whether `motion_mode` may be
    /// WARPED (a three-way symbol) or only OBMC (a boolean). `w4`/`h4` are the block's size
    /// clipped to the frame, `bw4`/`bh4` its full size; `col_end` the tile's end column.
    ///
    /// Neighbours are walked block by block (a neighbour wider than the remaining edge ends the
    /// walk), and the corner cells only count when the edge blocks line up with the block: a
    /// top-left cell only if the top and left neighbours start at this block's origin, a
    /// top-right cell only if the top neighbour ends where this block does. A cell that has not
    /// been decoded yet is not `valid`, which stands in for dav1d's top-right availability flag.
    #[allow(clippy::too_many_arguments)]
    pub fn has_matching_edge_ref(
        &self,
        x4: u32,
        y4: u32,
        bw4: u32,
        bh4: u32,
        w4: u32,
        h4: u32,
        col_end: u32,
        ref0: i8,
    ) -> bool {
        let matches = |cell: Option<&SpatialRefCell>| {
            cell.is_some_and(|c| c.valid && c.ref0 == ref0 && c.ref1 == -1)
        };
        let have_top = y4 > self.row_start;
        let have_left = x4 > self.col_start;
        let mut have_topleft = have_top && have_left;
        let mut have_topright = bw4.max(bh4) < 32 && have_top && x4 + bw4 < col_end;
        if have_top {
            let top = self.cell(x4, y4 - 1);
            if matches(top) {
                return true;
            }
            let mut aw4 = top.map_or(1, |c| u32::from(c.width_4x4).max(1));
            if aw4 >= bw4 {
                let off = x4 & (aw4 - 1);
                if off != 0 {
                    have_topleft = false;
                }
                if aw4 - off > bw4 {
                    have_topright = false;
                }
            } else {
                let mut pos = x4;
                let mut x = aw4;
                while x < w4 {
                    pos += aw4;
                    let cell = self.cell(pos, y4 - 1);
                    if matches(cell) {
                        return true;
                    }
                    aw4 = cell.map_or(1, |c| u32::from(c.width_4x4).max(1));
                    x += aw4;
                }
            }
        }
        if have_left {
            let left = self.cell(x4 - 1, y4);
            if matches(left) {
                return true;
            }
            let mut lh4 = left.map_or(1, |c| u32::from(c.height_4x4).max(1));
            if lh4 >= bh4 {
                if y4 & (lh4 - 1) != 0 {
                    have_topleft = false;
                }
            } else {
                let mut pos = y4;
                let mut y = lh4;
                while y < h4 {
                    pos += lh4;
                    let cell = self.cell(x4 - 1, pos);
                    if matches(cell) {
                        return true;
                    }
                    lh4 = cell.map_or(1, |c| u32::from(c.height_4x4).max(1));
                    y += lh4;
                }
            }
        }
        (have_topleft && matches(self.cell(x4 - 1, y4 - 1)))
            || (have_topright && matches(self.cell(x4 + bw4, y4 - 1)))
    }
}
