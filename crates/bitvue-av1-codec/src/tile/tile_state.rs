//! Mutable state shared by every block of one tile.
//!
//! The symbol decoder and the neighbour-context tracker always travel together through
//! `parse_superblock` -> `parse_coding_units_recursive` -> `parse_coding_unit` (and the
//! partition parser), and a caller creates them together once per tile. Owning them in one struct
//! keeps those signatures to "the tile state + what varies".

use crate::symbol::SymbolDecoder;
use crate::tile::TileContext;

/// See the module doc. `'a` is the lifetime of the tile data the decoder reads.
pub struct TileState<'a> {
    /// Arithmetic/symbol decoder positioned in the tile's data; every syntax element is read
    /// from this one shared instance, so a single unread element desyncs everything after it.
    pub decoder: SymbolDecoder<'a>,
    /// Above/left neighbour-state tracker for entropy contexts, shared across every superblock
    /// in the tile (see [`TileContext`]'s doc). Callers looping over superblock rows should call
    /// `tile_ctx.start_superblock_row()` at the start of each row.
    pub tile_ctx: TileContext,
}
