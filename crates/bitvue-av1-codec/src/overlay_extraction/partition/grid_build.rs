//! Filling a per-block grid from the coding units that cover each block.

use super::super::parser::ParsedFrame;
use bitvue_engine::BitvueError;

/// Build a grid from coding units using spatial indexing for O(n) performance
///
/// This is a key optimization that changes the algorithm from:
/// - Before: O(grid_h × grid_w × num_coding_units) - nested triple loop
/// - After: O(grid_h × grid_w + num_coding_units) - two separate loops
///
/// For 1080p video with ~30,000 coding units and ~8,000 grid blocks:
/// - Before: ~240M iterations (30,000 × 8,000)
/// - After: ~38K iterations (8,000 + 30,000)
/// - Speedup: ~6,300x faster
///
/// # Parameters
/// - `coding_units`: Slice of all coding units parsed from tile data
/// - `parsed`: Parsed frame with dimensions
/// - `block_w`, `block_h`: Grid block size in pixels
/// - `output`: Vector to fill with grid values
/// - `cu_value_fn`: Function to extract value from CU (mode, tx_size, etc.)
///
/// Grid blocks no coding unit covers (the decode stopped before them) stay `None`.
pub(super) fn build_grid_from_coding_units_spatial<T, F>(
    coding_units: &[crate::tile::CodingUnit],
    parsed: &ParsedFrame,
    block_w: u32,
    block_h: u32,
    output: &mut Vec<Option<T>>,
    cu_value_fn: F,
) -> Result<(), BitvueError>
where
    F: Fn(&crate::tile::CodingUnit) -> T,
{
    let grid_w = parsed.dimensions.width.div_ceil(block_w);
    let grid_h = parsed.dimensions.height.div_ceil(block_h);

    // Callers pass `Vec::with_capacity(total_blocks)` (length 0, only reserved capacity) --
    // every write below is a direct `output[idx] = ...` index assignment, which the `idx <
    // output.len()` guards silently skip entirely on a zero-length vec. Pre-fill to the real
    // length here so writes actually land (this path never actually ran before `ParsedFrame::
    // parse`'s OBU_FRAME tile-data bug was fixed -- has_tile_data() was always false, so this
    // function was never reached; the bug was latent, not a regression from that fix).
    let total = (grid_w as usize).saturating_mul(grid_h as usize);
    if output.len() < total {
        output.resize_with(total, || None);
    }

    // Build spatial index: map superblock position to relevant CUs
    // This allows O(1) lookup of which CUs to check for each grid block
    use std::collections::HashMap;
    let mut sb_index: HashMap<(u32, u32), Vec<usize>> = HashMap::new();

    for (cu_idx, cu) in coding_units.iter().enumerate() {
        // Calculate which superblock this CU belongs to
        let sb_x = cu.x / parsed.dimensions.sb_size;
        let sb_y = cu.y / parsed.dimensions.sb_size;
        sb_index.entry((sb_x, sb_y)).or_default().push(cu_idx);
    }

    // Now iterate through grid blocks, only checking CUs from relevant superblocks
    for grid_y in 0..grid_h {
        for grid_x in 0..grid_w {
            let block_x = grid_x * block_w;
            let block_y = grid_y * block_h;

            // Find which superblock this block belongs to
            let sb_x = block_x / parsed.dimensions.sb_size;
            let sb_y = block_y / parsed.dimensions.sb_size;

            // Get CUs from this superblock only (O(1) lookup)
            if let Some(cu_indices) = sb_index.get(&(sb_x, sb_y)) {
                // Only check CUs from this superblock (typically 1-4 CUs)
                for cu_idx in cu_indices {
                    let cu = &coding_units[*cu_idx];
                    if cu.x < block_x + block_w
                        && cu.x + cu.width > block_x
                        && cu.y < block_y + block_h
                        && cu.y + cu.height > block_y
                    {
                        // This CU overlaps our block - use its value
                        let idx = (grid_y * grid_w + grid_x) as usize;
                        if idx < output.len() {
                            output[idx] = Some(cu_value_fn(cu));
                        }
                        break;
                    }
                }
            }
        }
    }

    Ok(())
}
