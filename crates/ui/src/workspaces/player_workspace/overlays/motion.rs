//! Motion vector overlay drawing functions
//!
//! Provides motion vector visualization with L0/L1 layers and density control.
//! Per MV_VECTORS_IMPLEMENTATION_SPEC.md §2.

/// Implementation of motion vector overlay drawing
impl super::super::PlayerWorkspace {
    /// Draw motion vector overlay
    /// Per MV_VECTORS_IMPLEMENTATION_SPEC.md §2
    pub fn draw_mv_overlay(
        &self,
        ui: &mut egui::Ui,
        rect: egui::Rect,
        frame_size: (u32, u32),
        mv_grid_data: Option<&bitvue_core::MVGrid>,
    ) {
        let painter = ui.painter();
        let (frame_w, frame_h) = frame_size;

        // Only draw if we have real MV data
        let mv_grid = if let Some(grid) = mv_grid_data {
            tracing::trace!(
                "Using REAL MV data: {}x{} grid, {} total MVs",
                grid.grid_w,
                grid.grid_h,
                grid.mv_l0.len()
            );
            let non_zero = grid
                .mv_l0
                .iter()
                .filter(|mv| !mv.is_missing() && (mv.dx_qpel != 0 || mv.dy_qpel != 0))
                .count();
            tracing::trace!("  Non-zero MVs: {}", non_zero);
            grid
        } else {
            tracing::trace!("No MV data for this frame - skipping MV overlay");
            return; // Don't draw anything if no MV data
        };

        // Calculate screen scaling
        let scale_x = rect.width() / frame_w as f32;
        let scale_y = rect.height() / frame_h as f32;

        // Compute visible viewport in screen space (clip to UI)
        let clip = ui.clip_rect().intersect(rect);

        // Convert visible viewport to coded pixel coordinates
        let vp_min_x = ((clip.min.x - rect.min.x) / scale_x).clamp(0.0, frame_w as f32) as u32;
        let vp_min_y = ((clip.min.y - rect.min.y) / scale_y).clamp(0.0, frame_h as f32) as u32;
        let vp_max_x = ((clip.max.x - rect.min.x) / scale_x).clamp(0.0, frame_w as f32) as u32;
        let vp_max_y = ((clip.max.y - rect.min.y) / scale_y).clamp(0.0, frame_h as f32) as u32;

        let viewport = bitvue_core::Viewport::new(
            vp_min_x,
            vp_min_y,
            vp_max_x.saturating_sub(vp_min_x),
            vp_max_y.saturating_sub(vp_min_y),
        );

        // Determine block range intersecting viewport
        let bw = mv_grid.block_w.max(1);
        let bh = mv_grid.block_h.max(1);
        let col_start = (viewport.x / bw).min(mv_grid.grid_w.saturating_sub(1));
        let row_start = (viewport.y / bh).min(mv_grid.grid_h.saturating_sub(1));
        let col_end = (viewport.x + viewport.width)
            .div_ceil(bw)
            .min(mv_grid.grid_w);
        let row_end = (viewport.y + viewport.height)
            .div_ceil(bh)
            .min(mv_grid.grid_h);

        // Count visible, present vectors (L0 or L1) to compute stride
        let mut visible_present = 0usize;
        for row in row_start..row_end {
            for col in col_start..col_end {
                let has_l0 = mv_grid
                    .get_l0(col, row)
                    .map(|mv| !mv.is_missing() && (mv.dx_qpel != 0 || mv.dy_qpel != 0))
                    .unwrap_or(false);
                let has_l1 = mv_grid
                    .get_l1(col, row)
                    .map(|mv| !mv.is_missing() && (mv.dx_qpel != 0 || mv.dy_qpel != 0))
                    .unwrap_or(false);
                if has_l0 || has_l1 {
                    visible_present += 1;
                }
            }
        }

        // Calculate stride for density control (max 8000 vectors)
        let stride = bitvue_core::DensityControl::calculate_stride(visible_present.max(1));

        // Draw motion vectors within visible block range
        for row in row_start..row_end {
            for col in col_start..col_end {
                // Apply stride sampling
                if !bitvue_core::DensityControl::should_draw(col, row, stride) {
                    continue;
                }

                // Get block center in coded pixels
                let (block_center_x, block_center_y) = mv_grid.block_center(col, row);

                // Convert to screen coordinates
                let screen_x = rect.min.x + block_center_x * scale_x;
                let screen_y = rect.min.y + block_center_y * scale_y;

                // Draw L0 vectors (if enabled)
                if matches!(
                    self.overlays.mv.layer,
                    bitvue_core::MVLayer::L0Only | bitvue_core::MVLayer::Both
                ) {
                    if let Some(mv) = mv_grid.get_l0(col, row) {
                        // Skip missing or true zero vectors to reduce clutter
                        if !mv.is_missing() && (mv.dx_qpel != 0 || mv.dy_qpel != 0) {
                            self.draw_mv_arrow(
                                painter,
                                screen_x,
                                screen_y,
                                &mv,
                                scale_x.min(scale_y),
                                egui::Color32::from_rgba_unmultiplied(
                                    0,
                                    255,
                                    0,
                                    (self.overlays.mv.opacity * 255.0) as u8,
                                ), // Green for L0
                            );
                        }
                    }
                }

                // Draw L1 vectors (if enabled)
                if matches!(
                    self.overlays.mv.layer,
                    bitvue_core::MVLayer::L1Only | bitvue_core::MVLayer::Both
                ) {
                    if let Some(mv) = mv_grid.get_l1(col, row) {
                        if !mv.is_missing() && (mv.dx_qpel != 0 || mv.dy_qpel != 0) {
                            self.draw_mv_arrow(
                                painter,
                                screen_x,
                                screen_y,
                                &mv,
                                scale_x.min(scale_y),
                                egui::Color32::from_rgba_unmultiplied(
                                    255,
                                    0,
                                    255,
                                    (self.overlays.mv.opacity * 255.0) as u8,
                                ), // Magenta for L1
                            );
                        }
                    }
                }
            }
        }
    }

    /// Draw a single motion vector arrow
    /// Per MV_VECTORS_IMPLEMENTATION_SPEC.md §2.2
    fn draw_mv_arrow(
        &self,
        painter: &egui::Painter,
        start_x: f32,
        start_y: f32,
        mv: &bitvue_core::mv_overlay::MotionVector,
        zoom_scale: f32,
        color: egui::Color32,
    ) {
        // Scale vector
        let (dx, dy) =
            bitvue_core::MVScaling::scale_vector(mv, self.overlays.mv.user_scale, zoom_scale);

        // Clamp to max arrow length
        let (dx_clamped, dy_clamped) =
            bitvue_core::MVScaling::clamp_arrow_length(dx, dy, bitvue_core::MAX_ARROW_LENGTH_PX);

        let end_x = start_x + dx_clamped;
        let end_y = start_y + dy_clamped;

        // Draw arrow segment
        painter.line_segment(
            [egui::pos2(start_x, start_y), egui::pos2(end_x, end_y)],
            egui::Stroke::new(1.5, color),
        );

        // Draw arrow head (simple triangle)
        let magnitude = (dx_clamped * dx_clamped + dy_clamped * dy_clamped).sqrt();
        if magnitude > 2.0 {
            // Arrow head size
            let head_size = 4.0;

            // Normalize direction
            let norm_dx = dx_clamped / magnitude;
            let norm_dy = dy_clamped / magnitude;

            // Perpendicular vector
            let perp_dx = -norm_dy;
            let perp_dy = norm_dx;

            // Arrow head points
            let p1 = egui::pos2(
                end_x - norm_dx * head_size + perp_dx * head_size * 0.5,
                end_y - norm_dy * head_size + perp_dy * head_size * 0.5,
            );
            let p2 = egui::pos2(
                end_x - norm_dx * head_size - perp_dx * head_size * 0.5,
                end_y - norm_dy * head_size - perp_dy * head_size * 0.5,
            );
            let p3 = egui::pos2(end_x, end_y);

            painter.add(egui::Shape::convex_polygon(
                vec![p1, p2, p3],
                color,
                egui::Stroke::NONE,
            ));
        }
    }
}
