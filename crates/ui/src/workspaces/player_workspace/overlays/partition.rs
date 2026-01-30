//! Partition overlay drawing functions
//!
//! Provides partition grid visualization with scaffold and partition modes.
//! Per PARTITION_GRID_IMPLEMENTATION_SPEC.md §2.

/// Implementation of partition overlay drawing
impl super::super::PlayerWorkspace {
    /// Recursively partition a block into smaller blocks
    #[allow(dead_code)]
    fn _recursively_partition(
        grid: &mut bitvue_core::PartitionGrid,
        x: u32,
        y: u32,
        w: u32,
        h: u32,
        depth: u8,
    ) {
        use bitvue_core::partition_grid::{PartitionBlock, PartitionType};

        // Add current block
        grid.add_block(PartitionBlock::new(x, y, w, h, PartitionType::Split, depth));

        // Stop recursion at small blocks
        if w <= 8 || h <= 8 || depth >= 3 {
            return;
        }

        // Decide whether to split further (deterministic pattern)
        let should_split = !(x / 8 + y / 8 + depth as u32).is_multiple_of(3);

        if should_split {
            // Split into 4 quadrants
            let half_w = w / 2;
            let half_h = h / 2;

            Self::_recursively_partition(grid, x, y, half_w, half_h, depth + 1);
            Self::_recursively_partition(grid, x + half_w, y, w - half_w, half_h, depth + 1);
            Self::_recursively_partition(grid, x, y + half_h, half_w, h - half_h, depth + 1);
            Self::_recursively_partition(
                grid,
                x + half_w,
                y + half_h,
                w - half_w,
                h - half_h,
                depth + 1,
            );
        }
    }

    /// Create mock partition grid with hierarchical/nested blocks
    /// Per PARTITION_GRID_IMPLEMENTATION_SPEC.md §1
    #[allow(dead_code)]
    fn _create_mock_partition_grid(width: u32, height: u32) -> bitvue_core::PartitionGrid {
        use bitvue_core::partition_grid::PartitionGrid;

        let sb_size = 64;
        let mut grid = PartitionGrid::new(width, height, sb_size);

        // Generate hierarchical partition tree
        // Superblocks are recursively split into smaller blocks
        for sb_y in (0..height).step_by(sb_size as usize) {
            for sb_x in (0..width).step_by(sb_size as usize) {
                // Each superblock gets recursively partitioned
                Self::_recursively_partition(
                    &mut grid,
                    sb_x,
                    sb_y,
                    sb_size.min(width - sb_x),
                    sb_size.min(height - sb_y),
                    0, // depth
                );
            }
        }

        tracing::info!(
            "Generated partition grid with {} blocks",
            grid.block_count()
        );
        grid
    }

    /// Create mock partition data for testing (uniform grid)
    /// Per PARTITION_GRID_IMPLEMENTATION_SPEC.md §1
    #[allow(dead_code)]
    fn _create_mock_partition_data(width: u32, height: u32) -> bitvue_core::PartitionData {
        // Use 8x8 leaf blocks for AV1
        let leaf_block_w = 8;
        let leaf_block_h = 8;
        let grid_w = width.div_ceil(leaf_block_w);
        let grid_h = height.div_ceil(leaf_block_h);

        // Generate mock partition kinds (checkerboard pattern for testing)
        let mut part_kind = Vec::with_capacity((grid_w * grid_h) as usize);
        for y in 0..grid_h {
            for x in 0..grid_w {
                // Create a pattern: Intra, Inter, Skip, Intra, Inter, ...
                let kind = match (x + y) % 4 {
                    0 => bitvue_core::PartitionKind::Intra,
                    1 => bitvue_core::PartitionKind::Inter,
                    2 => bitvue_core::PartitionKind::Skip,
                    _ => bitvue_core::PartitionKind::Split,
                };
                part_kind.push(kind);
            }
        }

        bitvue_core::PartitionData::new(width, height, leaf_block_w, leaf_block_h, part_kind)
    }

    /// Draw partition overlay
    /// Per PARTITION_GRID_IMPLEMENTATION_SPEC.md §2
    pub fn draw_partition_overlay(&self, ui: &mut egui::Ui, rect: egui::Rect, frame_size: (u32, u32)) {
        let painter = ui.painter();
        let (frame_w, frame_h) = frame_size;

        // Calculate screen scaling
        let scale_x = rect.width() / frame_w as f32;
        let scale_y = rect.height() / frame_h as f32;

        match self.overlays.partition.mode {
            bitvue_core::GridMode::Scaffold => {
                // Scaffold mode: superblock grid (64x64 or 128x128)
                // AV1 uses 64x64 or 128x128 superblocks
                let superblock_size = 64; // Could be 128 for some sequences
                let grid_lines_x = frame_w.div_ceil(superblock_size);
                let grid_lines_y = frame_h.div_ceil(superblock_size);

                // LOD decimation: check grid density
                // Per PARTITION_GRID_IMPLEMENTATION_SPEC.md §2.3
                let screen_spacing_x = superblock_size as f32 * scale_x;
                let screen_spacing_y = superblock_size as f32 * scale_y;
                let min_spacing = screen_spacing_x.min(screen_spacing_y);

                // Superblocks are large, so decimation is rarely needed
                // But we still support it for extreme zoom-out
                let stride = if min_spacing < 3.0 {
                    (3.0 / min_spacing).ceil() as u32
                } else {
                    1
                };

                let decimated = stride > 1;

                // Draw vertical superblock lines (thicker, more visible)
                for i in (0..=grid_lines_x).step_by(stride as usize) {
                    let x_coded = i * superblock_size;
                    let x_screen = rect.min.x + x_coded as f32 * scale_x;
                    painter.line_segment(
                        [
                            egui::pos2(x_screen, rect.min.y),
                            egui::pos2(x_screen, rect.max.y),
                        ],
                        egui::Stroke::new(
                            2.0,
                            egui::Color32::from_rgba_unmultiplied(255, 255, 0, 180),
                        ),
                    );
                }

                // Draw horizontal superblock lines (thicker, more visible)
                for i in (0..=grid_lines_y).step_by(stride as usize) {
                    let y_coded = i * superblock_size;
                    let y_screen = rect.min.y + y_coded as f32 * scale_y;
                    painter.line_segment(
                        [
                            egui::pos2(rect.min.x, y_screen),
                            egui::pos2(rect.max.x, y_screen),
                        ],
                        egui::Stroke::new(
                            2.0,
                            egui::Color32::from_rgba_unmultiplied(255, 255, 0, 180),
                        ),
                    );
                }

                // Show superblock info in corner
                let legend_pos = rect.min + egui::vec2(10.0, 10.0);
                let legend_text = if decimated {
                    format!(
                        "Superblock {}×{} (decimated {}x)",
                        superblock_size, superblock_size, stride
                    )
                } else {
                    format!("Superblock {}×{}", superblock_size, superblock_size)
                };
                painter.text(
                    legend_pos,
                    egui::Align2::LEFT_TOP,
                    legend_text,
                    egui::FontId::proportional(12.0),
                    egui::Color32::from_rgba_unmultiplied(255, 255, 0, 200),
                );
            }
            bitvue_core::GridMode::Partition => {
                // Partition mode: draw actual partition tree (hierarchical blocks)
                // Use cached partition grid if available
                let partition_grid = if let Some(ref grid) = self.overlays.partition.grid {
                    grid
                } else {
                    tracing::warn!("No partition grid available");
                    return;
                };

                // Draw partition boundaries only (no fill)
                // Per feedback: only show boundaries, not tint
                for (idx, block) in partition_grid.blocks.iter().enumerate() {
                    // Screen coordinates
                    let screen_x = rect.min.x + block.x as f32 * scale_x;
                    let screen_y = rect.min.y + block.y as f32 * scale_y;
                    let screen_w = block.width as f32 * scale_x;
                    let screen_h = block.height as f32 * scale_y;

                    // Skip if too small to render
                    if screen_w < 1.0 || screen_h < 1.0 {
                        continue;
                    }

                    // Only draw boundaries (no fill)
                    let is_selected = self.overlays.partition.selected_block == Some(idx);

                    if is_selected {
                        // Selected: thick yellow outline
                        painter.rect_stroke(
                            egui::Rect::from_min_size(
                                egui::pos2(screen_x, screen_y),
                                egui::vec2(screen_w, screen_h),
                            ),
                            0.0,
                            egui::Stroke::new(3.0, egui::Color32::from_rgb(255, 255, 0)),
                        );
                    } else {
                        // Normal: very thin white/gray boundary
                        let alpha = (self.overlays.partition.opacity * 255.0 * 2.0) as u8; // Use opacity for line visibility
                        painter.rect_stroke(
                            egui::Rect::from_min_size(
                                egui::pos2(screen_x, screen_y),
                                egui::vec2(screen_w, screen_h),
                            ),
                            0.0,
                            egui::Stroke::new(
                                0.5,
                                egui::Color32::from_rgba_unmultiplied(200, 200, 200, alpha),
                            ),
                        );
                    }
                }

                // Show partition info in corner
                let legend_pos = rect.min + egui::vec2(10.0, 10.0);
                painter.text(
                    legend_pos,
                    egui::Align2::LEFT_TOP,
                    format!("Partition Tree ({} blocks)", partition_grid.block_count()),
                    egui::FontId::proportional(12.0),
                    egui::Color32::from_rgba_unmultiplied(255, 255, 255, 220),
                );
            }
        }
    }
}
