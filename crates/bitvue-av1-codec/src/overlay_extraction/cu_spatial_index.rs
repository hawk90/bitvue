//! Spatial index for O(1) coding unit lookup by grid position.

/// Spatial index for O(1) coding unit lookup by grid position
///
/// Pre-computes which coding unit overlaps each grid cell, eliminating
/// the need for O(n) linear search per block. For 1080p, this reduces
/// 510×1000 = 510,000 comparisons to just 510 lookups.
pub struct CuSpatialIndex {
    /// Grid of CU indices (one per grid cell)
    /// Vec index = grid_y * grid_w + grid_x
    /// Value = Some(cu_index) or None if no CU covers this cell
    grid: Vec<Option<usize>>,
    grid_w: u32,
}

impl CuSpatialIndex {
    /// Build spatial index from coding units
    ///
    /// For each coding unit, determine which grid cells it overlaps
    /// and store the CU index in those cells.
    ///
    /// # Arguments
    /// * `coding_units` - Slice of coding units to index
    /// * `grid_w` - Grid width in cells
    /// * `grid_h` - Grid height in cells
    /// * `block_w` - Grid cell width in pixels
    /// * `block_h` - Grid cell height in pixels
    pub fn new(
        coding_units: &[crate::tile::CodingUnit],
        grid_w: u32,
        grid_h: u32,
        block_w: u32,
        block_h: u32,
    ) -> Self {
        let total_cells = (grid_w * grid_h) as usize;
        let mut grid = vec![None; total_cells];

        for (cu_idx, cu) in coding_units.iter().enumerate() {
            // Convert CU pixel coordinates to grid coordinates
            // All values are u32, so division works correctly
            let cu_grid_x_start = cu.x / block_w;
            let cu_grid_y_start = cu.y / block_h;
            let cu_grid_x_end = cu.x.saturating_add(cu.width).saturating_sub(1) / block_w;
            let cu_grid_y_end = cu.y.saturating_add(cu.height).saturating_sub(1) / block_h;

            // Clamp to grid bounds
            let clamped_x_start = cu_grid_x_start.min(grid_w - 1);
            let clamped_y_start = cu_grid_y_start.min(grid_h - 1);
            let clamped_x_end = cu_grid_x_end.min(grid_w - 1);
            let clamped_y_end = cu_grid_y_end.min(grid_h - 1);

            // Mark all grid cells overlapped by this CU
            for grid_y in clamped_y_start..=clamped_y_end {
                for grid_x in clamped_x_start..=clamped_x_end {
                    let cell_idx = (grid_y * grid_w + grid_x) as usize;
                    // First CU wins (earlier CUs take precedence)
                    if grid[cell_idx].is_none() {
                        grid[cell_idx] = Some(cu_idx);
                    }
                }
            }
        }

        Self { grid, grid_w }
    }

    /// Get coding unit index for a grid cell (O(1) lookup)
    #[inline]
    pub fn get_cu_index(&self, grid_x: u32, grid_y: u32) -> Option<usize> {
        let cell_idx = (grid_y * self.grid_w + grid_x) as usize;
        self.grid.get(cell_idx).copied().flatten()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cu_spatial_index() {
        // Create some test CUs
        let cus = vec![
            crate::tile::CodingUnit::new(0, 0, 64, 64),
            crate::tile::CodingUnit::new(64, 0, 64, 64),
        ];

        // Build index with 64x64 grid cells
        let index = CuSpatialIndex::new(&cus, 4, 4, 64, 64);

        // Check that we can find CUs
        assert_eq!(index.get_cu_index(0, 0), Some(0)); // First CU
        assert_eq!(index.get_cu_index(1, 0), Some(1)); // Second CU
        assert_eq!(index.get_cu_index(0, 1), None); // No CU here
    }
}
