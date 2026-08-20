//! `get_deblocking_analysis` command backing -- AV1 loop-filter boundary strength (BS) per coding-
//! unit edge, plus real loop-filter parameters, for one frame of stream A. Feeds
//! `frontend/components/Player/views/DeblockingView.tsx`. AV1/IVF only.
//!
//! Two real gaps closed to make this possible (neither was "just wiring"):
//!  - `bitvue_av1_codec::frame_header_full`'s `loop_filter_params()` used to discard every value
//!    (`skip_loop_filter_params`) -- now parsed for real into `FrameHeader.loop_filter`.
//!  - Boundary-strength derivation (AV1 spec 7.14.2) didn't exist anywhere in this workspace --
//!    see `bitvue_av1_codec::overlay_extraction::deblocking`'s module doc for the algorithm and
//!    its documented simplifications.
//!
//! Same sequential-scan requirement as `av1_features`: `loop_filter_params()`'s bit layout
//! doesn't depend on cross-frame state (unlike `skip_mode_params`), but `parse_frame_header_full`
//! itself does, so this reuses the same `RefFrameState`-threading pattern to reach the target
//! frame's real `FrameHeader`.

use bitvue_av1_codec::frame_header_full::{
    find_frame_header_payload, parse_frame_header_full, thread_ref_state_before,
};
use bitvue_av1_codec::obu::{ObuIterator, ObuType};
use bitvue_av1_codec::overlay_extraction::{
    extract_deblocking_data_from_parsed, DeblockingData, DeblockingEdge, ParsedFrame,
};
use bitvue_av1_codec::sequence::{parse_sequence_header, SequenceHeader};
use serde_json::{json, Value};

const SEQUENCE_HEADER_SCAN_LIMIT: usize = 8;

fn find_sequence_header(frames: &[bitvue_av1_codec::ivf::IvfFrame]) -> Option<SequenceHeader> {
    for frame in frames.iter().take(SEQUENCE_HEADER_SCAN_LIMIT) {
        let mut iter = ObuIterator::new(&frame.data);
        while let Some(Ok(found)) = iter.next_obu_with_offset() {
            if found.obu.header.obu_type == ObuType::SequenceHeader {
                return parse_sequence_header(&found.obu.payload).ok();
            }
        }
    }
    None
}

fn find_sequence_header_bytes(frames: &[bitvue_av1_codec::ivf::IvfFrame]) -> Option<Vec<u8>> {
    for frame in frames.iter().take(SEQUENCE_HEADER_SCAN_LIMIT) {
        let mut iter = ObuIterator::new(&frame.data);
        while let Some(Ok(found)) = iter.next_obu_with_offset() {
            if found.obu.header.obu_type == ObuType::SequenceHeader {
                return Some(frame.data[found.offset..found.offset + found.consumed].to_vec());
            }
        }
    }
    None
}

pub fn get_deblocking_analysis(data: &[u8], frame_index: usize) -> Result<Value, String> {
    let (_hdr, frames) = bitvue_av1_codec::ivf::parse_ivf_frames(data)
        .map_err(|e| format!("IVF parse error: {e}"))?;
    if frame_index >= frames.len() {
        return Err(format!(
            "frame_index {frame_index} out of range (stream has {} frames)",
            frames.len()
        ));
    }
    let seq = find_sequence_header(&frames)
        .ok_or_else(|| "no sequence header found in the first few frames".to_string())?;

    // `ref_state` reflects frames `[0, frame_index)` -- exactly what both this frame's own
    // header parse (below) and `ParsedFrame::parse_with_ref_state`'s `tile_data` extraction need
    // as their starting state (see that method's doc: a fresh state silently desyncs `tile_data`
    // for any frame needing real `skip_mode_params` state). Cloned before the header parse
    // mutates it with frame_index's own contribution, since `parse_with_ref_state` below needs
    // the pre-frame_index state, not the post-frame_index one.
    let mut ref_state =
        thread_ref_state_before(&frames, &seq, frame_index).map_err(|e| e.to_string())?;
    let mut header_ref_state = ref_state.clone();
    let payload = find_frame_header_payload(&frames[frame_index].data)
        .ok_or_else(|| "frame has no Frame/FrameHeader OBU".to_string())?;
    let header = parse_frame_header_full(&payload, &seq, &mut header_ref_state)
        .map_err(|e| e.to_string())?;

    let obu_data: Vec<u8> = match find_sequence_header_bytes(&frames) {
        Some(seq_bytes) => [seq_bytes.as_slice(), frames[frame_index].data.as_slice()].concat(),
        None => frames[frame_index].data.clone(),
    };
    let parsed =
        ParsedFrame::parse_with_ref_state(&obu_data, &mut ref_state).map_err(|e| e.to_string())?;

    let deblocking = extract_deblocking_data_from_parsed(&parsed, &header.loop_filter)
        .map_err(|e| e.to_string())?;

    Ok(deblocking_to_json(frame_index, &deblocking))
}

fn edge_to_json(e: &DeblockingEdge) -> Value {
    json!({
        "x": e.x,
        "y": e.y,
        "length": e.length,
        "orientation": if e.vertical { "vertical" } else { "horizontal" },
        "boundary_strength": e.boundary_strength,
        "filtered": e.filtered,
        "strength": e.strength,
    })
}

fn deblocking_to_json(frame_index: usize, d: &DeblockingData) -> Value {
    let total_edges = d.edges.len();
    let filtered_edges = d.edges.iter().filter(|e| e.filtered).count();
    let strong_edges = d.edges.iter().filter(|e| e.boundary_strength == 2).count();
    let weak_edges = d.edges.iter().filter(|e| e.boundary_strength == 1).count();

    json!({
        "frame_index": frame_index,
        "width": d.width,
        "height": d.height,
        "edges": d.edges.iter().map(edge_to_json).collect::<Vec<_>>(),
        "params": {
            "level": d.loop_filter.level,
            "sharpness": d.loop_filter.sharpness,
            "delta_enabled": d.loop_filter.delta_enabled,
            "ref_deltas": d.loop_filter.ref_deltas,
            "mode_deltas": d.loop_filter.mode_deltas,
        },
        "stats": {
            "total_edges": total_edges,
            "filtered_edges": filtered_edges,
            "strong_edges": strong_edges,
            "weak_edges": weak_edges,
        },
    })
}

#[derive(serde::Deserialize)]
struct GetDeblockingAnalysisParams {
    frame_index: usize,
}

/// AV1 loop-filter boundary strength + real loop-filter parameters for one frame of stream A --
/// see this module's doc. Control-only, same reasoning as
/// `frame_analysis::get_frame_analysis_command`.
pub fn get_deblocking_analysis_command(
    core: &bitvue_engine::Core,
    request: &bitvue_protocol::Request,
) -> bitvue_protocol::Response {
    use bitvue_protocol::{Response, WireError, WireErrorCode};

    let params: GetDeblockingAnalysisParams = match serde_json::from_value(request.params.clone()) {
        Ok(p) => p,
        Err(err) => {
            return Response::failure(
                request.id,
                WireError {
                    code: WireErrorCode::InvalidData,
                    message: err.to_string(),
                    offset: None,
                },
            )
        }
    };

    let stream_state = core.get_stream(bitvue_engine::StreamId::A);
    let state = stream_state.read();
    let byte_cache = match state.byte_cache.as_ref() {
        Some(cache) => std::sync::Arc::clone(cache),
        None => {
            return Response::failure(
                request.id,
                WireError {
                    code: WireErrorCode::NotFound,
                    message: "stream not open".to_string(),
                    offset: None,
                },
            )
        }
    };
    drop(state);

    let full_len = byte_cache.len() as usize;
    let data = match byte_cache.read_range(0, full_len) {
        Ok(bytes) => bytes,
        Err(err) => {
            return Response::failure(
                request.id,
                WireError {
                    code: crate::command_support::wire_error_code_for(&err),
                    message: err.to_string(),
                    offset: None,
                },
            )
        }
    };

    match get_deblocking_analysis(data, params.frame_index) {
        Ok(value) => Response::success(request.id, value),
        Err(message) => Response::failure(
            request.id,
            WireError {
                code: WireErrorCode::FrameNotFound,
                message,
                offset: None,
            },
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const AV1_IVF_FIXTURE: &[u8] = include_bytes!("../../../test_data/av1_test.ivf");

    #[test]
    fn get_deblocking_analysis_end_to_end_returns_real_edges() {
        let result = get_deblocking_analysis(AV1_IVF_FIXTURE, 0).unwrap();
        assert_eq!(result["frame_index"], 0);
        assert_eq!(result["width"], 320);
        assert_eq!(result["height"], 240);
        let edges = result["edges"].as_array().unwrap();
        assert!(!edges.is_empty(), "expected real boundary edges");
        // boundary_strength must be a small numeric code (0-2), never a string.
        let bs = edges[0]["boundary_strength"].as_u64().unwrap();
        assert!(bs <= 2);
        assert!(result["stats"]["total_edges"].as_u64().unwrap() > 0);
    }

    #[test]
    fn get_deblocking_analysis_non_first_frame_uses_sequential_ref_state() {
        let result = get_deblocking_analysis(AV1_IVF_FIXTURE, 100).unwrap();
        assert_eq!(result["frame_index"], 100);
    }

    #[test]
    fn get_deblocking_analysis_out_of_range_frame_index_is_a_real_error() {
        assert!(get_deblocking_analysis(AV1_IVF_FIXTURE, 999_999).is_err());
    }

    #[test]
    fn get_deblocking_analysis_last_frame_of_the_real_fixture_parses() {
        let (_hdr, frames) = bitvue_av1_codec::ivf::parse_ivf_frames(AV1_IVF_FIXTURE).unwrap();
        let result = get_deblocking_analysis(AV1_IVF_FIXTURE, frames.len() - 1).unwrap();
        assert_eq!(result["frame_index"], frames.len() - 1);
    }
}
