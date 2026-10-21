//! Recursive partition parsing (spec 5.11.4) and the conversion of a parsed tree to a grid.

use super::tree::*;
use super::types::{BlockSize, PartitionType};
use crate::symbol::SymbolDecoder;
use crate::tile::TileState;
use bitvue_engine::{BitvueError, Result};

/// Recursively parse partition tree using symbol decoder.
///
/// This is the sole partition-tree walker in the crate (a duplicate, weaker copy used to live in
/// `tile/superblock.rs` -- no depth guard, no partition-legality validation, and a `child_position`
/// that silently returned the parent's own position for 6 of the 10 partition types (`HorzA`/
/// `HorzB`/`VertA`/`VertB`/`Horz4`/`Vert4`) instead of erroring or computing real child offsets --
/// consolidated onto this implementation instead of fixing the weaker one in place).
///
/// Returns `Ok(None)` when `(x, y)` (converted to MI units) falls entirely outside the frame
/// (`r >= mi_rows || c >= mi_cols`, spec 5.11.4's very first check) -- per spec this position
/// contributes nothing at all (no symbol read, no coded block), so the caller must drop it rather
/// than emit a placeholder leaf (a placeholder would desync `parse_coding_unit`, which expects
/// every leaf it's handed to have real bitstream-coded syntax). Reached whenever a `SPLIT` at a
/// frame-edge superblock recurses into a quadrant that's now fully beyond `MiRows`/`MiCols` (a
/// straddling-but-not-fully-outside quadrant instead gets `has_rows`/`has_cols`'s reduced
/// alphabet -- see `SymbolDecoder::read_partition`'s doc -- which still reads a real symbol).
///
/// `mi_rows`/`mi_cols` (`mi_units`'s output for the frame's real pixel width/height) are
/// frame-global constants threaded down unchanged through the recursion; `has_rows`/`has_cols`
/// are recomputed at every call from the CURRENT `(x, y, block_size)`, per spec -- they are not
/// inherited from the parent (a block can straddle the edge while its parent didn't, or vice
/// versa, depending on which quadrant of a `SPLIT` it is).
///
/// Per spec, only `PARTITION_SPLIT` recurses into another `decode_partition` call -- `HORZ`/
/// `VERT`/the `_A`/`_B`/`_4` variants are terminal (their sub-blocks go straight to
/// `decode_block`, no further partition symbol read); this function's children loop branches on
/// exactly that distinction (see its body). This was NOT always true here: until this change, the
/// loop recursed into `parse_partition_recursive` again for every non-`None` type, including
/// terminal ones -- which are often non-square (e.g. a `HORZ`-split 64x64's two 64x32 halves) --
/// and `block_size_log2` still resolved a real CDF bucket for them via their larger dimension, so
/// that read a second, spec-nonexistent `partition` symbol at that position. Dormant for most of
/// this crate's history (the spurious symbol usually decoded to `None`, terminating recursion
/// there with a plausible-but-wrong leaf size) until real `has_rows`/`has_cols` (this same change)
/// started actually forcing HORZ/VERT choices at frame edges, where the spurious read's bucket
/// mismatch surfaced as outright decode errors -- found and fixed in the same pass rather than
/// worked around, since real `has_rows`/`has_cols` is what makes these terminal choices reachable
/// at all.
pub(crate) fn parse_partition_recursive<V>(
    state: &mut TileState<'_>,
    x: u32,
    y: u32,
    block_size: BlockSize,
    mi_rows: u32,
    mi_cols: u32,
    depth: u8,
    visit_leaf: &mut V,
) -> Result<Option<PartitionNode>>
where
    V: FnMut(&mut TileState<'_>, &PartitionNode) -> Result<()>,
{
    // Prevent infinite recursion from malformed bitstreams
    if depth >= MAX_PARTITION_DEPTH {
        return Err(BitvueError::InvalidData(format!(
            "Partition recursion depth exceeded maximum ({}), possible infinite loop in bitstream",
            MAX_PARTITION_DEPTH
        )));
    }

    let r = y / 4;
    let c = x / 4;
    if r >= mi_rows || c >= mi_cols {
        return Ok(None);
    }

    let partition = read_partition_type(state, x, y, block_size, (mi_rows, mi_cols))?;
    record_partition_context(state, x, y, block_size, partition);

    let mut node = PartitionNode::new(x, y, block_size, partition);
    if partition == PartitionType::Split {
        parse_split_children(state, &mut node, (mi_rows, mi_cols), depth, visit_leaf)?;
    } else if partition != PartitionType::None {
        visit_terminal_children(state, &mut node, (mi_rows, mi_cols), visit_leaf)?;
    } else {
        visit_leaf(state, &node)?;
    }

    Ok(Some(node))
}

/// Reads and validates the `partition` symbol for the block at `(x, y)`.
fn read_partition_type(
    state: &mut TileState<'_>,
    x: u32,
    y: u32,
    block_size: BlockSize,
    (mi_rows, mi_cols): (u32, u32),
) -> Result<PartitionType> {
    // spec 5.11.4: hasRows/hasCols from the CURRENT block's own extent -- num4x4 uses `width()`
    // since decode_partition is only ever spec-invoked on square blocks (this fn's one known
    // exception, terminal-partition children, is documented above; width()==height() for every
    // other caller).
    let num4x4 = block_size.width() / 4;
    let half_block4x4 = num4x4 >> 1;
    let has_rows = y / 4 + half_block4x4 < mi_rows;
    let has_cols = x / 4 + half_block4x4 < mi_cols;

    // Get block size log2 for CDF lookup
    let bsize_log2 = block_size_log2(block_size);

    // Real context (spec 9.3, `crate::tile::TileContext::partition_context`) only exists for
    // block sizes >= 8x8 (log2 >= 3) -- 4x4 blocks (log2 == 2) never read a real `partition`
    // symbol at all (see `partition_cdfs`' doc), so `ctx` is a don't-care 0 there.
    let ctx = if bsize_log2 >= 3 {
        state.tile_ctx.partition_context(
            x / 8,
            y / 8,
            crate::tile::context::partition_bl(bsize_log2),
        )
    } else {
        0
    };

    // Read partition symbol from bitstream
    let partition_symbol = state
        .decoder
        .read_partition(bsize_log2, ctx, has_rows, has_cols)?;

    // Convert symbol to partition type
    let partition = PartitionType::from_u8(partition_symbol).ok_or_else(|| {
        BitvueError::InvalidData(format!("Invalid partition symbol: {}", partition_symbol))
    })?;

    // Validate partition is allowed for this block size
    if !partition.is_allowed(block_size) {
        return Err(BitvueError::InvalidData(format!(
            "Partition {:?} not allowed for block size {:?}",
            partition, block_size
        )));
    }

    // Validate sub_block_size doesn't return same size (prevents infinite recursion)
    if partition != PartitionType::None {
        // Check that at least one sub-block is smaller than parent
        let has_smaller = block_size
            .sub_block_size(partition)
            .iter()
            .any(|&s| s.width() < block_size.width() || s.height() < block_size.height());
        if !has_smaller {
            return Err(BitvueError::InvalidData(format!(
                "Partition {:?} on block size {:?} produces sub-blocks of same size - not supported",
                partition, block_size
            )));
        }
    }
    Ok(partition)
}

/// Record this node's own context footprint for future above/left lookups -- per spec/rav1d,
/// skipped only for `Split` above 8x8, since in that case each recursively-parsed child (which
/// exists at real `BlockLevel`s down to 8x8) writes its own footprint instead; at 8x8 `Split`
/// the children are 4x4 (no context tracked there per `partition_cdfs`' doc), so the 8x8 level
/// itself must still write. See `crate::tile::context::PARTITION_CTX_TABLE`'s doc.
fn record_partition_context(
    state: &mut TileState<'_>,
    x: u32,
    y: u32,
    block_size: BlockSize,
    partition: PartitionType,
) {
    let bsize_log2 = block_size_log2(block_size);
    if bsize_log2 >= 3 && (partition != PartitionType::Split || bsize_log2 == 3) {
        let bl = crate::tile::context::partition_bl(bsize_log2);
        let hsz8 = 1u32 << (bsize_log2 - 3);
        state
            .tile_ctx
            .set_partition(x / 8, y / 8, hsz8, bl, partition as u8);
    }
}

/// spec 5.11.4: only SPLIT recurses into another decode_partition call -- its children are
/// always half-size squares, so `block_size_log2`/`read_partition`'s bucket lookup stays
/// meaningful for them too. `mi_rows`/`mi_cols` pass through unchanged (frame-global); a child
/// that is fully outside the frame contributes no node at all (see this fn's doc).
fn parse_split_children<V>(
    state: &mut TileState<'_>,
    node: &mut PartitionNode,
    extent: (u32, u32),
    depth: u8,
    visit_leaf: &mut V,
) -> Result<()>
where
    V: FnMut(&mut TileState<'_>, &PartitionNode) -> Result<()>,
{
    let (x, y, block_size) = (node.x, node.y, node.size);
    let sub_sizes = block_size.sub_block_size(PartitionType::Split);
    for (i, sub_size) in sub_sizes.iter().enumerate() {
        let (child_x, child_y) = child_position(x, y, i, PartitionType::Split, block_size);
        let child = parse_partition_recursive(
            state,
            child_x,
            child_y,
            *sub_size,
            extent.0,
            extent.1,
            depth + 1,
            visit_leaf,
        )?;
        if let Some(child) = child {
            node.children.push(child);
        }
    }
    Ok(())
}

/// spec 5.11.4: HORZ/VERT/the `_A`/`_B`/`_4` variants are terminal -- their sub-blocks go
/// straight to `decode_block` (a `parse_coding_unit` leaf here), with NO further `partition`
/// symbol read (see this module's `parse_partition_recursive` doc for the bug this replaced).
/// Sub-blocks starting outside the frame are dropped.
fn visit_terminal_children<V>(
    state: &mut TileState<'_>,
    node: &mut PartitionNode,
    (mi_rows, mi_cols): (u32, u32),
    visit_leaf: &mut V,
) -> Result<()>
where
    V: FnMut(&mut TileState<'_>, &PartitionNode) -> Result<()>,
{
    let (x, y, block_size, partition) = (node.x, node.y, node.size, node.partition);
    let sub_sizes = block_size.sub_block_size(partition);
    for (i, sub_size) in sub_sizes.iter().enumerate() {
        let (child_x, child_y) = child_position(x, y, i, partition, block_size);
        if child_y / 4 < mi_rows && child_x / 4 < mi_cols {
            let leaf = PartitionNode::new(child_x, child_y, *sub_size, PartitionType::None);
            visit_leaf(state, &leaf)?;
            node.children.push(leaf);
        }
    }
    Ok(())
}

/// Parse partition tree from tile data
///
/// Uses symbol decoder to read partition symbols from the bitstream
/// and recursively builds the partition tree.
///
/// # Arguments
///
/// * `x` - Block X position in pixels
/// * `y` - Block Y position in pixels
/// * `block_size` - Starting block size (typically superblock size)
/// * `tile_data` - Tile bitstream data
///
/// # Returns
///
/// Root partition node with recursively parsed children.
pub fn parse_partition_tree(
    x: u32,
    y: u32,
    block_size: BlockSize,
    tile_data: &[u8],
) -> Result<PartitionNode> {
    // Create symbol decoder for tile data
    let decoder = SymbolDecoder::new(tile_data)?;

    // Throwaway context sized to just this one block -- this function has no real caller (see
    // `parse_partition_recursive`'s doc; production code goes through `parse_superblock`, which
    // threads a real tile-wide `TileContext` shared across the whole tile).
    let extent_4x4 = (block_size.width().max(block_size.height()) / 4).max(1);
    let tile_ctx = crate::tile::TileContext::new(extent_4x4, extent_4x4);
    let mut state = TileState {
        decoder,
        mv_ctx: crate::tile::MvPredictorContext::new(1, 1),
        tile_ctx,
    };

    // Recursively parse partition tree starting at depth 0. No real frame extent is known here
    // (see this fn's doc -- no real caller), so treat `block_size` itself as exactly filling the
    // frame (mi_rows/mi_cols derived from its own dimensions): every position within it is then
    // unconditionally in-bounds, matching this fn's pre-existing "assume all blocks are within
    // frame boundaries" MVP semantics.
    let mi_rows = mi_units(block_size.height());
    let mi_cols = mi_units(block_size.width());
    parse_partition_recursive(
        &mut state,
        x,
        y,
        block_size,
        mi_rows,
        mi_cols,
        0,
        &mut |_, _| Ok(()),
    )
    .map(|node| node.unwrap_or_else(|| PartitionNode::new(x, y, block_size, PartitionType::None)))
}

/// Flatten partition tree to list of leaf blocks
///
/// Converts hierarchical partition tree to a flat list of blocks
/// for visualization in PartitionGrid.
fn flatten_partition_tree(
    node: &PartitionNode,
    depth: u8,
    blocks: &mut Vec<(u32, u32, u32, u32, u8, u8)>,
) {
    if node.is_leaf() {
        // Leaf node: add as block
        blocks.push((
            node.x,
            node.y,
            node.size.width(),
            node.size.height(),
            node.partition as u8,
            depth,
        ));
    } else {
        // Non-leaf: recursively process children
        for child in &node.children {
            flatten_partition_tree(child, depth + 1, blocks);
        }
    }
}

/// Convert partition tree to PartitionGrid
///
/// Extracts all leaf blocks from partition tree and creates a PartitionGrid
/// suitable for visualization.
pub fn partition_tree_to_grid(
    root: &PartitionNode,
    coded_width: u32,
    coded_height: u32,
    sb_size: u32,
) -> bitvue_engine::PartitionGrid {
    let mut grid = bitvue_engine::PartitionGrid::new(coded_width, coded_height, sb_size);

    // Flatten tree to list of blocks
    let mut blocks = Vec::new();
    flatten_partition_tree(root, 0, &mut blocks);

    // Convert to PartitionGrid blocks
    for (x, y, width, height, partition, depth) in blocks {
        let partition_type = bitvue_engine::partition_grid::PartitionType::from(partition);
        let block = bitvue_engine::partition_grid::PartitionBlock::new(
            x,
            y,
            width,
            height,
            partition_type,
            depth,
        );
        grid.add_block(block);
    }

    grid
}

#[cfg(test)]
#[path = "parse_tests.rs"]
mod parse_tests;
