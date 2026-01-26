//! Selection conversion helper functions

use super::types::*;
use bitvue_core::{
    selection::{
        SelectionState, TemporalSelection, SpatialBlock, UnitKey,
        SelectionAction, StreamId, FrameKey,
    },
    BitRange, SyntaxNodeId,
};

/// Convert stream ID to string
pub fn stream_id_to_string(stream: StreamId) -> String {
    match stream {
        StreamId::A => "A".to_string(),
        StreamId::B => "B".to_string(),
    }
}

/// Convert string to stream ID
pub fn string_to_stream_id(s: String) -> StreamId {
    match s.as_str() {
        "B" => StreamId::B,
        _ => StreamId::A,
    }
}

/// Convert spatial block to response
pub fn spatial_block_to_response(block: SpatialBlock) -> SpatialBlockResponse {
    SpatialBlockResponse {
        x: block.x,
        y: block.y,
        w: block.w,
        h: block.h,
    }
}

/// Convert response to spatial block
pub fn spatial_block_from_response(block: SpatialBlockResponse) -> SpatialBlock {
    SpatialBlock {
        x: block.x,
        y: block.y,
        w: block.w,
        h: block.h,
    }
}

/// Convert temporal selection to response
pub fn temporal_selection_to_response(sel: TemporalSelection) -> TemporalSelectionResponse {
    match sel {
        TemporalSelection::Block { frame_index, block } => {
            TemporalSelectionResponse::Block {
                frame_index,
                block: spatial_block_to_response(block),
            }
        }
        TemporalSelection::Point { frame_index } => {
            TemporalSelectionResponse::Point { frame_index }
        }
        TemporalSelection::Range { start, end } => {
            TemporalSelectionResponse::Range { start, end }
        }
        TemporalSelection::Marker { frame_index } => {
            TemporalSelectionResponse::Marker { frame_index }
        }
    }
}

/// Convert derived cursor to response
pub fn derived_cursor_to_response(cursor: bitvue_core::selection::DerivedCursor) -> DerivedCursorResponse {
    DerivedCursorResponse {
        frame_index: cursor.frame_index,
        spatial_pos: cursor.spatial_pos,
    }
}

/// Convert unit key to response
pub fn unit_key_to_response(unit: UnitKey) -> UnitKeyResponse {
    UnitKeyResponse {
        stream: stream_id_to_string(unit.stream),
        unit_type: unit.unit_type,
        offset: unit.offset,
        size: unit.size,
    }
}

/// Convert bit range to response
pub fn bit_range_to_response(range: BitRange) -> BitRangeResponse {
    BitRangeResponse {
        offset: range.start_bit,
        length: range.end_bit - range.start_bit,
    }
}

/// Convert response to bit range
pub fn bit_range_from_response(range: BitRangeResponse) -> BitRange {
    BitRange {
        start_bit: range.offset,
        end_bit: range.offset + range.length,
    }
}

/// Convert selection state to response
pub fn selection_state_to_response(state: &SelectionState) -> SelectionStateResponse {
    SelectionStateResponse {
        stream_id: stream_id_to_string(state.stream_id),
        temporal: state.temporal.as_ref().map(|t| temporal_selection_to_response(t.clone())),
        cursor: state.cursor.as_ref().map(|c| derived_cursor_to_response(*c)),
        unit: state.unit.as_ref().map(|u| unit_key_to_response(u.clone())),
        syntax_node: state.syntax_node.as_ref().map(|n| serde_json::to_string(n).unwrap_or_default()),
        bit_range: state.bit_range.as_ref().map(|r| bit_range_to_response(*r)),
        source_view: state.source_view.clone(),
    }
}

/// Convert selection action request to selection action
pub fn selection_action_from_request(action: SelectionActionRequest) -> Result<SelectionAction, String> {
    match action {
        SelectionActionRequest::SelectBlock { frame_index, block } => {
            Ok(SelectionAction::SelectBlock {
                frame_index,
                block: spatial_block_from_response(block),
            })
        }
        SelectionActionRequest::SelectPoint { frame_index } => {
            Ok(SelectionAction::SelectPoint { frame_index })
        }
        SelectionActionRequest::SelectRange { start, end } => {
            Ok(SelectionAction::SelectRange { start, end })
        }
        SelectionActionRequest::SelectMarker { frame_index } => {
            Ok(SelectionAction::SelectMarker { frame_index })
        }
        SelectionActionRequest::SelectFrame { stream, frame_index, pts } => {
            let frame_key = FrameKey {
                stream: string_to_stream_id(stream),
                frame_index,
                pts,
            };
            Ok(SelectionAction::SelectFrame { frame_key })
        }
        SelectionActionRequest::SelectUnit { stream, unit_type, offset, size } => {
            let unit = UnitKey {
                stream: string_to_stream_id(stream),
                unit_type,
                offset,
                size,
            };
            Ok(SelectionAction::SelectUnit { unit })
        }
        SelectionActionRequest::SelectSyntax { node_id, bit_range } => {
            // Parse node_id from JSON string
            let node: SyntaxNodeId = serde_json::from_str(&node_id)
                .map_err(|e| format!("Invalid node_id: {}", e))?;
            Ok(SelectionAction::SelectSyntax {
                node_id: node,
                bit_range: bit_range_from_response(bit_range),
            })
        }
        SelectionActionRequest::SelectBitRange { offset, length } => {
            Ok(SelectionAction::SelectBitRange {
                bit_range: bit_range_from_response(BitRangeResponse { offset, length }),
            })
        }
        SelectionActionRequest::ClearTemporal => {
            Ok(SelectionAction::ClearTemporal)
        }
        SelectionActionRequest::ClearAll => {
            Ok(SelectionAction::ClearAll)
        }
    }
}
