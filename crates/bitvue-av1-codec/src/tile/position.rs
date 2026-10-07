//! Where a block sits: its own rectangle and the per-superblock state of its enclosing
//! superblock.
//!
//! Both are what `parse_coding_unit` needs besides the tile state and the frame flags, and they
//! used to travel as six loose scalars plus a bare `&mut [i8; 4]`.

/// A block's position and size in 4x4 ("MI") units -- the unit every entropy-context lookup and
/// `TileContext::set_*` call works in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MiRect {
    pub x4: u32,
    pub y4: u32,
    pub width: u32,
    pub height: u32,
}

/// A block's position and size in pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlockRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// State scoped to one superblock, created fresh for each one.
///
/// Origin and size are in 4x4 ("MI") units. Spec `delta_q`/`delta_lf` are read only once per
/// superblock, at whichever leaf sits at `(x4, y4)` (always the first leaf visited in
/// partition-tree order, spec 5.11.4's decode order), not once per coding unit.
#[derive(Debug, Clone)]
pub struct SuperblockCtx {
    pub x4: u32,
    pub y4: u32,
    pub size4: u32,
    /// `cdef_idx()`'s per-superblock "already read" tracker (spec 5.11.56). `-1` (dav1d's
    /// sentinel) = "not yet read"; four slots for `sb128`'s 2x2 grid of 64x64 CDEF units
    /// (`sb64` only touches slot `0`).
    pub cdef_idx: [i8; 4],
}

impl SuperblockCtx {
    /// Context for the superblock whose top-left pixel is `(x, y)` and whose side is `size`
    /// pixels.
    pub fn new(x: u32, y: u32, size: u32) -> Self {
        Self {
            x4: x / 4,
            y4: y / 4,
            size4: size / 4,
            cdef_idx: [-1; 4],
        }
    }
}
