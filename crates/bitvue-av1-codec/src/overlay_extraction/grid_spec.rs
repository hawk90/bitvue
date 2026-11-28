//! The prologue and cell fill the overlay grid extractors share: the grid's shape and its
//! overflow check, the strict "decode the tile or fail" step, and filling one value per cell from
//! the coding unit that covers it.
//!
//! What the extractors do *differently* stays in the extractors: whether a frame without
//! decodable tile data is an error or an all-zero grid (QP/MV/prediction mode/transform vs
//! energy/bit), the `MAX_GRID_BLOCKS` cap (prediction mode/transform only), and which cell-fill
//! mechanism is used.

use std::sync::Arc;

use bitvue_engine::{limits::MAX_GRID_BLOCKS, BitvueError};

use super::cu_parser::parse_all_coding_units;
use super::cu_spatial_index::CuSpatialIndex;
use super::parser::ParsedFrame;
use super::provenance::{has_decodable_tile, no_decodable_tile};
use crate::tile::CodingUnit;

/// A grid of `grid_w` x `grid_h` cells of `block_w` x `block_h` pixels covering a frame.
#[derive(Debug, Clone, Copy)]
pub(super) struct GridSpec {
    pub grid_w: u32,
    pub grid_h: u32,
    pub block_w: u32,
    pub block_h: u32,
    /// `grid_w * grid_h`.
    pub total: usize,
}

impl GridSpec {
    /// The grid covering `width` x `height` (partial cells at the right/bottom edge round up), or
    /// an error when its cell count does not fit in `u32`.
    pub(super) fn new(
        width: u32,
        height: u32,
        block_w: u32,
        block_h: u32,
    ) -> Result<Self, BitvueError> {
        let grid_w = width.div_ceil(block_w);
        let grid_h = height.div_ceil(block_h);
        let total = grid_w.checked_mul(grid_h).ok_or_else(|| {
            BitvueError::Decode(format!("Grid dimensions too large: {}x{}", grid_w, grid_h))
        })? as usize;
        Ok(Self {
            grid_w,
            grid_h,
            block_w,
            block_h,
            total,
        })
    }

    /// An error when the grid has more than [`MAX_GRID_BLOCKS`] cells.
    pub(super) fn ensure_within_max_blocks(&self) -> Result<(), BitvueError> {
        if self.total > MAX_GRID_BLOCKS {
            return Err(BitvueError::Decode(format!(
                "Grid exceeds maximum size: {}x{} = {} blocks",
                self.grid_w, self.grid_h, self.total
            )));
        }
        Ok(())
    }
}

/// The coding units of `parsed`'s tile; a frame without decodable tile data, or whose tile cannot
/// be parsed, is an error (there is no substitute).
pub(super) fn decode_units_strict(
    parsed: &ParsedFrame,
) -> Result<Arc<Vec<CodingUnit>>, BitvueError> {
    if !has_decodable_tile(parsed) {
        return Err(no_decodable_tile());
    }
    parse_all_coding_units(parsed)
}

/// One value per cell (row-major): `cell_value` gets the coding unit covering the cell (the first
/// in decode order when several do), or `None` when no decoded unit covers it.
pub(super) fn map_cells<T>(
    spec: &GridSpec,
    units: &[CodingUnit],
    mut cell_value: impl FnMut(Option<&CodingUnit>) -> T,
) -> Vec<T> {
    let index = CuSpatialIndex::new(units, spec.grid_w, spec.grid_h, spec.block_w, spec.block_h);
    let mut cells = Vec::with_capacity(spec.total);
    for grid_y in 0..spec.grid_h {
        for grid_x in 0..spec.grid_w {
            cells.push(cell_value(
                index.get_cu_index(grid_x, grid_y).map(|i| &units[i]),
            ));
        }
    }
    cells
}
