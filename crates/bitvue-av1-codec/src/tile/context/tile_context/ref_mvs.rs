//! Delegation to the [`SpatialRefContext`] (the reference-motion-vector map).

use super::TileContext;
use crate::tile::context::{CompoundMvStackEntry, MvStackEntry};

impl TileContext {
    /// Record a decoded **inter** block's ref/mode state for future `inter_mode`/`compound_mode`
    /// context lookups -- see `SpatialRefContext::set_block`'s doc (never call for intra blocks).
    #[allow(clippy::too_many_arguments)]
    #[allow(clippy::too_many_arguments)]
    pub fn set_spatial_ref_block(
        &mut self,
        x4: u32,
        y4: u32,
        width_4x4: u32,
        height_4x4: u32,
        ref0: i8,
        ref1: i8,
        is_newmv: bool,
        mv0: crate::tile::coding_unit::MotionVector,
        mv1: crate::tile::coding_unit::MotionVector,
    ) {
        self.spatial_ref.set_block(
            x4, y4, width_4x4, height_4x4, ref0, ref1, is_newmv, mv0, mv1,
        );
    }

    /// Records an intra block -- see `SpatialRefContext::set_intra_block`.
    pub fn set_spatial_ref_intra_block(
        &mut self,
        x4: u32,
        y4: u32,
        width_4x4: u32,
        height_4x4: u32,
    ) {
        self.spatial_ref
            .set_intra_block(x4, y4, width_4x4, height_4x4);
    }

    /// Places the tile inside the frame, in 4x4 units (call after `set_frame_extent`): its first
    /// column and row, and the end it ends at (clipped to the frame).
    pub fn set_tile_extent(&mut self, col_start: u32, col_end: u32, row_start: u32, row_end: u32) {
        self.tile_col_start = col_start;
        self.tile_row_start = row_start;
        self.spatial_ref
            .set_tile_extent(col_start, col_end, row_start, row_end);
    }

    /// Whether the row above `y4` is inside the tile.
    pub fn has_top(&self, y4: u32) -> bool {
        y4 > self.tile_row_start
    }

    /// Whether the column left of `x4` is inside the tile.
    pub fn has_left(&self, x4: u32) -> bool {
        x4 > self.tile_col_start
    }

    /// Real frame size -- see `SpatialRefContext::set_frame_extent`.
    pub fn set_frame_extent(&mut self, width: u32, height: u32) {
        self.spatial_ref.set_frame_extent(width, height);
    }

    /// Packed single-ref `inter_mode` context -- see `SpatialRefContext::inter_mode_context`.
    pub fn inter_mode_context(
        &self,
        x4: u32,
        y4: u32,
        bw4: u32,
        bh4: u32,
        ref0: i8,
        use_ref_frame_mvs: bool,
    ) -> u16 {
        self.spatial_ref
            .inter_mode_context(x4, y4, bw4, bh4, ref0, use_ref_frame_mvs)
    }

    /// See `SpatialRefContext::has_matching_edge_ref`.
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
        self.spatial_ref
            .has_matching_edge_ref(x4, y4, bw4, bh4, w4, h4, col_end, ref0)
    }

    /// Real weighted single-ref DRL candidate stack -- see `SpatialRefContext::single_ref_mv_stack`.
    pub fn single_ref_mv_stack(
        &self,
        x4: u32,
        y4: u32,
        bw4: u32,
        bh4: u32,
        ref0: i8,
        use_ref_frame_mvs: bool,
    ) -> ([MvStackEntry; 8], usize) {
        self.spatial_ref
            .single_ref_mv_stack(x4, y4, bw4, bh4, ref0, use_ref_frame_mvs)
    }

    /// Real weighted compound DRL candidate stack -- see `SpatialRefContext::compound_mv_stack`.
    #[allow(clippy::too_many_arguments)]
    pub fn compound_mv_stack(
        &self,
        x4: u32,
        y4: u32,
        bw4: u32,
        bh4: u32,
        ref0: i8,
        ref1: i8,
        use_ref_frame_mvs: bool,
    ) -> ([CompoundMvStackEntry; 8], usize) {
        self.spatial_ref
            .compound_mv_stack(x4, y4, bw4, bh4, ref0, ref1, use_ref_frame_mvs)
    }

    /// `RefFrameSignBias` of the frame -- see `SpatialRefContext::set_sign_bias`.
    pub fn set_sign_bias(&mut self, sign_bias: [bool; 7]) {
        self.spatial_ref.set_sign_bias(sign_bias);
    }

    /// Opt this frame's parse into real temporal MV candidates -- see
    /// `SpatialRefContext::set_temporal_context`.
    pub fn set_temporal_context(
        &mut self,
        projected: crate::tile::motion_field::ProjectedMotionField,
        pocdiff: [i32; 7],
    ) {
        self.spatial_ref.set_temporal_context(projected, pocdiff);
    }

    /// Frame MV precision for temporal candidates -- see `SpatialRefContext::set_mv_precision`.
    pub fn set_mv_precision(&mut self, allow_high_precision_mv: bool, force_integer_mv: bool) {
        self.spatial_ref
            .set_mv_precision(allow_high_precision_mv, force_integer_mv);
    }

    /// `compound_mode` context -- see `SpatialRefContext::compound_mode_context`.
    pub fn compound_mode_context(
        &self,
        x4: u32,
        y4: u32,
        bw4: u32,
        bh4: u32,
        ref0: i8,
        ref1: i8,
    ) -> u8 {
        self.spatial_ref
            .compound_mode_context(x4, y4, bw4, bh4, ref0, ref1)
    }

    /// `(valid, width_4x4, height_4x4)` of the reference-map cell at `(x4, y4)`, for tests.
    #[cfg(test)]
    pub(crate) fn spatial_ref_cell(&self, x4: u32, y4: u32) -> Option<(bool, u8, u8)> {
        self.spatial_ref
            .cell(x4, y4)
            .map(|c| (c.valid, c.width_4x4, c.height_4x4))
    }
}
