//! SelectionState - Single source of truth for UI selection
//!
//! Monster Pack v3: TRI_SYNC_CONTRACT.md
//! Monster Pack v14 T0-3: SELECTION_PRECEDENCE_RULES.md
//!
//! Key Contracts:
//! - Selection is the source of truth, cursor is derived
//! - Only ONE selection type active (Block > Point > Range > Marker)
//! - Higher priority selections clear lower ones

use crate::{BitRange, SyntaxNodeId};
use serde::{Deserialize, Serialize};

/// Stream identifier (A or B for Dual View)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StreamId {
    A,
    B,
}

/// Frame key (stable identifier)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FrameKey {
    pub stream: StreamId,
    pub frame_index: usize,
    pub pts: Option<u64>,
}

/// Unit key (offset-based, stable)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UnitKey {
    pub stream: StreamId,
    pub unit_type: String, // e.g., "OBU_FRAME_HEADER"
    pub offset: u64,
    pub size: usize,
}

// BitRange and SyntaxNodeId are now imported from crate::types

/// Spatial block (for overlay block selection)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpatialBlock {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

// ============================================================================
// T0-3: SELECTION_PRECEDENCE_RULES Implementation
// ============================================================================

/// Temporal/Spatial selection type with precedence
///
/// Per SELECTION_PRECEDENCE_RULES.md:
/// Priority (highest to lowest): Block > Point > Range > Marker
/// Only ONE selection type may be active at a time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TemporalSelection {
    /// Block selection (spatial cell) - HIGHEST PRIORITY
    /// Spatial block in current frame
    Block {
        frame_index: usize,
        block: SpatialBlock,
    },

    /// Point selection (single frame) - SECOND PRIORITY
    Point { frame_index: usize },

    /// Range selection (frame interval) - THIRD PRIORITY
    Range { start: usize, end: usize },

    /// Marker selection - LOWEST PRIORITY
    Marker { frame_index: usize },
}

impl TemporalSelection {
    /// Get the precedence level (higher = higher priority)
    pub fn precedence(&self) -> u8 {
        match self {
            TemporalSelection::Block { .. } => 4,
            TemporalSelection::Point { .. } => 3,
            TemporalSelection::Range { .. } => 2,
            TemporalSelection::Marker { .. } => 1,
        }
    }

    /// Get the current frame index (or start of range)
    pub fn frame_index(&self) -> usize {
        match self {
            TemporalSelection::Block { frame_index, .. } => *frame_index,
            TemporalSelection::Point { frame_index } => *frame_index,
            TemporalSelection::Range { start, .. } => *start,
            TemporalSelection::Marker { frame_index } => *frame_index,
        }
    }

    /// Get the spatial block if this is a block selection
    pub fn spatial_block(&self) -> Option<SpatialBlock> {
        match self {
            TemporalSelection::Block { block, .. } => Some(*block),
            _ => None,
        }
    }

    /// Check if this is a range selection
    pub fn is_range(&self) -> bool {
        matches!(self, TemporalSelection::Range { .. })
    }

    /// Get range bounds if this is a range selection
    pub fn range_bounds(&self) -> Option<(usize, usize)> {
        match self {
            TemporalSelection::Range { start, end } => Some((*start, *end)),
            _ => None,
        }
    }
}

/// Derived cursor position
///
/// Per SELECTION_PRECEDENCE_RULES.md:
/// "Selection is the source of truth. Cursor is derived."
///
/// The cursor is always derived from the active selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DerivedCursor {
    /// Current frame index
    pub frame_index: usize,

    /// Spatial position (if block selected)
    pub spatial_pos: Option<(u32, u32)>,
}

impl DerivedCursor {
    /// Derive cursor from temporal selection
    pub fn from_selection(selection: &TemporalSelection) -> Self {
        let frame_index = selection.frame_index();
        let spatial_pos = selection.spatial_block().map(|block| (block.x, block.y));

        Self {
            frame_index,
            spatial_pos,
        }
    }
}

/// SelectionState - Authoritative global selection state
///
/// Per T0-3 SELECTION_PRECEDENCE_RULES.md:
/// - Selection is the source of truth (cursor is derived)
/// - Only ONE temporal selection type active (Block > Point > Range > Marker)
///
/// Per TRI_SYNC_CONTRACT §1 (Authority priority):
/// 1. bitRange (most specific)
/// 2. syntaxNode
/// 3. unit
/// 4. temporal (frameIndex / pts)
/// 5. stream_id
///
/// God object refactoring note: This struct is intentionally cohesive
/// as it manages the complete selection state for the application.
/// The fields are all related to selection state management.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectionState {
    /// Stream identifier (A or B for Dual View)
    pub stream_id: StreamId,

    /// Temporal/Spatial selection with precedence (Block > Point > Range > Marker)
    /// Only ONE may be active at a time
    pub temporal: Option<TemporalSelection>,

    /// Derived cursor (computed from temporal selection)
    pub cursor: Option<DerivedCursor>,

    /// Structural selection (tri-sync) - unit key
    pub unit: Option<UnitKey>,

    /// Structural selection (tri-sync) - syntax node ID
    pub syntax_node: Option<SyntaxNodeId>,

    /// Structural selection (tri-sync) - bit range for hex view
    pub bit_range: Option<BitRange>,

    /// Source view (debug/telemetry only) - e.g., "Tree", "Hex", "Timeline"
    pub source_view: Option<String>,
}

impl Default for SelectionState {
    fn default() -> Self {
        Self {
            stream_id: StreamId::A,
            temporal: None,
            cursor: None,
            unit: None,
            syntax_node: None,
            bit_range: None,
            source_view: None,
        }
    }
}

impl SelectionState {
    pub fn new(stream_id: StreamId) -> Self {
        Self {
            stream_id,
            ..Default::default()
        }
    }

    /// Update cursor from temporal selection
    fn update_cursor(&mut self) {
        self.cursor = self.temporal.as_ref().map(DerivedCursor::from_selection);
    }

    /// Get current frame index (from cursor)
    pub fn current_frame(&self) -> Option<usize> {
        self.cursor.map(|c| c.frame_index)
    }

    // ========================================================================
    // T0-3: Temporal Selection Methods (with precedence)
    // ========================================================================

    /// Select a block (highest priority)
    ///
    /// Per SELECTION_PRECEDENCE_RULES.md:
    /// Block selection clears all other selection types.
    pub fn select_block(&mut self, frame_index: usize, block: SpatialBlock) {
        self.temporal = Some(TemporalSelection::Block { frame_index, block });
        self.update_cursor();
    }

    /// Select a point (single frame)
    ///
    /// Per SELECTION_PRECEDENCE_RULES.md:
    /// Point replaces range if range was active.
    pub fn select_point(&mut self, frame_index: usize) {
        self.temporal = Some(TemporalSelection::Point { frame_index });
        self.update_cursor();
    }

    /// Select a range (frame interval)
    ///
    /// Per SELECTION_PRECEDENCE_RULES.md:
    /// Range replaces point/marker.
    pub fn select_range(&mut self, start: usize, end: usize) {
        self.temporal = Some(TemporalSelection::Range { start, end });
        self.update_cursor();
    }

    /// Select a marker
    ///
    /// Per SELECTION_PRECEDENCE_RULES.md:
    /// - If no selection: creates point at marker
    /// - If range exists: clears range and creates point
    pub fn select_marker(&mut self, frame_index: usize) {
        // Check if range is active
        let has_range = matches!(self.temporal, Some(TemporalSelection::Range { .. }));

        if has_range || self.temporal.is_none() {
            // Clear range and create point (or create point if no selection)
            self.temporal = Some(TemporalSelection::Point { frame_index });
        } else {
            // Just store as marker
            self.temporal = Some(TemporalSelection::Marker { frame_index });
        }

        self.update_cursor();
    }

    /// Clear temporal selection
    pub fn clear_temporal(&mut self) {
        self.temporal = None;
        self.cursor = None;
    }

    // ========================================================================
    // Tri-sync Structural Selection (TRI_SYNC_CONTRACT)
    // ========================================================================

    /// Invalidation rule: SelectUnit clears syntax/bitRange
    pub fn select_unit(&mut self, unit: UnitKey) {
        self.unit = Some(unit);
        self.syntax_node = None;
        self.bit_range = None;
    }

    /// Update syntax node + bitRange
    pub fn select_syntax(&mut self, node_id: SyntaxNodeId, bit_range: BitRange) {
        self.syntax_node = Some(node_id);
        self.bit_range = Some(bit_range);
    }

    /// Hex click: derive syntax node via find_nearest_node
    pub fn select_bit_range(&mut self, bit_range: BitRange) {
        self.bit_range = Some(bit_range);
        // TODO: Derive syntax_node via find_nearest_node (TRI_SYNC_CONTRACT §3)
    }

    /// Select spatial block (overlay click) - delegates to select_block
    pub fn select_spatial_block(&mut self, frame_index: usize, block: SpatialBlock) {
        self.select_block(frame_index, block);
    }

    // ========================================================================
    // Resize Handle Support
    // ========================================================================

    /// Resize a block's bottom edge (Y+ axis)
    ///
    /// Updates the height of the current block selection.
    /// Preserves top edge (Y coordinate) while adjusting height.
    pub fn resize_block_bottom(&mut self, new_height: u32) {
        if let Some(TemporalSelection::Block {
            frame_index: _,
            block,
        }) = &mut self.temporal
        {
            let new_block = SpatialBlock {
                x: block.x,
                y: block.y,
                w: block.w,
                h: new_height,
            };
            *block = new_block;
            self.update_cursor();
        }
    }

    /// Resize a block's top edge (Y- axis)
    ///
    /// Updates both Y position and height to maintain bottom edge.
    pub fn resize_block_top(&mut self, new_y: u32, new_height: u32) {
        if let Some(TemporalSelection::Block {
            frame_index: _,
            block,
        }) = &mut self.temporal
        {
            let new_block = SpatialBlock {
                x: block.x,
                y: new_y,
                w: block.w,
                h: new_height,
            };
            *block = new_block;
            self.update_cursor();
        }
    }

    /// Resize a block's left edge (X- axis)
    ///
    /// Updates both X position and width to maintain right edge.
    pub fn resize_block_left(&mut self, new_x: u32, new_width: u32) {
        if let Some(TemporalSelection::Block {
            frame_index: _,
            block,
        }) = &mut self.temporal
        {
            let new_block = SpatialBlock {
                x: new_x,
                y: block.y,
                w: new_width,
                h: block.h,
            };
            *block = new_block;
            self.update_cursor();
        }
    }

    /// Resize a block's right edge (X+ axis)
    ///
    /// Updates the width of the current block selection.
    /// Preserves left edge (X coordinate) while adjusting width.
    pub fn resize_block_right(&mut self, new_width: u32) {
        if let Some(TemporalSelection::Block {
            frame_index: _,
            block,
        }) = &mut self.temporal
        {
            let new_block = SpatialBlock {
                x: block.x,
                y: block.y,
                w: new_width,
                h: block.h,
            };
            *block = new_block;
            self.update_cursor();
        }
    }

    /// Resize a block's top-left corner (X-/Y- axes)
    ///
    /// Updates both position and size to maintain bottom-right corner.
    pub fn resize_block_top_left(
        &mut self,
        new_x: u32,
        new_y: u32,
        new_width: u32,
        new_height: u32,
    ) {
        if let Some(TemporalSelection::Block {
            frame_index: _,
            block,
        }) = &mut self.temporal
        {
            let new_block = SpatialBlock {
                x: new_x,
                y: new_y,
                w: new_width,
                h: new_height,
            };
            *block = new_block;
            self.update_cursor();
        }
    }

    /// Resize a block's top-right corner (X+/Y- axes)
    ///
    /// Updates Y position, width, and height to maintain bottom-left corner.
    pub fn resize_block_top_right(&mut self, new_y: u32, new_width: u32, new_height: u32) {
        if let Some(TemporalSelection::Block {
            frame_index: _,
            block,
        }) = &mut self.temporal
        {
            let new_block = SpatialBlock {
                x: block.x,
                y: new_y,
                w: new_width,
                h: new_height,
            };
            *block = new_block;
            self.update_cursor();
        }
    }

    /// Resize a block's bottom-left corner (X-/Y+ axes)
    ///
    /// Updates X position, width, and height to maintain top-right corner.
    pub fn resize_block_bottom_left(&mut self, new_x: u32, new_width: u32, new_height: u32) {
        if let Some(TemporalSelection::Block {
            frame_index: _,
            block,
        }) = &mut self.temporal
        {
            let new_block = SpatialBlock {
                x: new_x,
                y: block.y,
                w: new_width,
                h: new_height,
            };
            *block = new_block;
            self.update_cursor();
        }
    }

    /// Resize a block's bottom-right corner (X+/Y+ axes)
    ///
    /// Updates width and height to maintain top-left corner.
    pub fn resize_block_bottom_right(&mut self, new_width: u32, new_height: u32) {
        if let Some(TemporalSelection::Block {
            frame_index: _,
            block,
        }) = &mut self.temporal
        {
            let new_block = SpatialBlock {
                x: block.x,
                y: block.y,
                w: new_width,
                h: new_height,
            };
            *block = new_block;
            self.update_cursor();
        }
    }
}

// ============================================================================
// T0-3: SelectionReducer
// ============================================================================

/// Selection action for reducer pattern
#[derive(Debug, Clone)]
pub enum SelectionAction {
    // Temporal actions
    SelectBlock {
        frame_index: usize,
        block: SpatialBlock,
    },
    SelectPoint {
        frame_index: usize,
    },
    SelectRange {
        start: usize,
        end: usize,
    },
    SelectMarker {
        frame_index: usize,
    },
    ClearTemporal,

    // Structural actions (tri-sync)
    SelectUnit {
        unit: UnitKey,
    },
    SelectSyntax {
        node_id: SyntaxNodeId,
        bit_range: BitRange,
    },
    SelectBitRange {
        bit_range: BitRange,
    },

    // Clear all
    ClearAll,
}

/// SelectionReducer - Reducer pattern for selection state updates
///
/// Per T0-3 deliverable: SelectionReducer
///
/// Implements pure reducer pattern for testability and time-travel debugging.
pub struct SelectionReducer;

impl SelectionReducer {
    /// Apply an action to a selection state, returning a new state
    ///
    /// This is a pure function for testability.
    pub fn reduce(mut state: SelectionState, action: SelectionAction) -> SelectionState {
        match action {
            SelectionAction::SelectBlock { frame_index, block } => {
                state.select_block(frame_index, block);
            }
            SelectionAction::SelectPoint { frame_index } => {
                state.select_point(frame_index);
            }
            SelectionAction::SelectRange { start, end } => {
                state.select_range(start, end);
            }
            SelectionAction::SelectMarker { frame_index } => {
                state.select_marker(frame_index);
            }
            SelectionAction::ClearTemporal => {
                state.clear_temporal();
            }
            SelectionAction::SelectUnit { unit } => {
                state.select_unit(unit);
            }
            SelectionAction::SelectSyntax { node_id, bit_range } => {
                state.select_syntax(node_id, bit_range);
            }
            SelectionAction::SelectBitRange { bit_range } => {
                state.select_bit_range(bit_range);
            }
            SelectionAction::ClearAll => {
                state = SelectionState::new(state.stream_id);
            }
        }

        state
    }

    /// Apply an action in-place (mutating version for performance)
    pub fn reduce_mut(state: &mut SelectionState, action: SelectionAction) {
        *state = Self::reduce(state.clone(), action);
    }
}

/// Per generate-tests skill: Comprehensive test suite with Arrange-Act-Assert pattern
#[cfg(test)]
mod tests {
    include!("selection_test.rs");
}
