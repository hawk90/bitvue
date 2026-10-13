//! The partition grid: the coding-unit rectangles of a frame, as `PartitionGrid` blocks.

use super::super::cu_parser::parse_all_coding_units;
use super::super::parser::ParsedFrame;
use crate::tile::PredictionMode;
use bitvue_engine::{partition_grid::PartitionGrid, partition_grid::PartitionType, BitvueError};

/// Extract Partition Grid from AV1 bitstream data
///
/// **Current Implementation**:
/// - Attempts to parse actual partition trees from tile data
/// - Falls back to scaffold grid if parsing fails
/// - Uses SymbolDecoder for entropy decoding
///
/// # Performance
///
/// - O(n) where n = number of superblocks
/// - Falls back to O(1) scaffold if tile data unavailable
pub fn extract_partition_grid(
    obu_data: &[u8],
    _frame_index: usize,
) -> Result<PartitionGrid, BitvueError> {
    let parsed = ParsedFrame::parse(obu_data)?;

    extract_partition_grid_from_parsed(&parsed)
}

/// Extract Partition Grid from cached frame data
///
/// This is more efficient when extracting multiple overlays
/// from the same frame.
///
/// Attempts real partition parsing first, falls back to scaffold.
pub fn extract_partition_grid_from_parsed(
    parsed: &ParsedFrame,
) -> Result<PartitionGrid, BitvueError> {
    if !super::super::provenance::has_decodable_tile(parsed) {
        return Err(super::super::provenance::no_decodable_tile());
    }
    parse_partition_trees_from_tile_data(parsed)
}

/// The partition grid of a decoded frame: one block per coding unit of the canonical parse (the
/// same cached one every other extractor uses, so the grids always agree with each other).
///
/// This used to run its own parse, with a fresh MV context per superblock, smaller superblocks
/// at the frame edge, and an invented block wherever a superblock failed -- a second decoder
/// that disagreed with the real one. A frame the decode does not complete simply has no blocks
/// past that point; its provenance (`frame_provenance`) says so.
fn parse_partition_trees_from_tile_data(
    parsed: &ParsedFrame,
) -> Result<PartitionGrid, BitvueError> {
    let units = parse_all_coding_units(parsed)?;
    let mut grid = PartitionGrid::new(
        parsed.dimensions.width,
        parsed.dimensions.height,
        parsed.dimensions.sb_size,
    );
    for cu in units.iter() {
        grid.add_block(bitvue_engine::partition_grid::PartitionBlock::new(
            cu.x,
            cu.y,
            cu.width,
            cu.height,
            partition_type_from_prediction_mode(cu.mode),
            0,
        ));
    }
    Ok(grid)
}

/// Convert prediction mode to partition type for visualization
fn partition_type_from_prediction_mode(mode: PredictionMode) -> PartitionType {
    match mode {
        PredictionMode::DcPred => PartitionType::None,
        PredictionMode::VPred => PartitionType::Vert,
        PredictionMode::HPred => PartitionType::Horz,
        _ => PartitionType::None,
    }
}
