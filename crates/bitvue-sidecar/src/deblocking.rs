//! `get_deblocking_analysis` command backing -- AV1 loop-filter edges (transform-block edges of the
//! luma plane with the largest filter each allows and its level) plus the frame's loop-filter
//! parameters, for one frame of stream A. Feeds `frontend/components/Player/views/DeblockingView.tsx`.
//! AV1/IVF only.
//!
//! The derivation follows spec 7.14.2-7.14.5 (see `bitvue_av1_codec::overlay_extraction::
//! deblocking`'s module doc, which also lists what it cannot know: the filter actually applied
//! per sample line, and the per-block `delta_lf`). AV1 has no HEVC-style boundary strength; the
//! wire carries `filter_size` (4, 8 or 16 samples) and `level` instead.
//!
//! Same sequential-scan requirement as `av1_features`: `parse_frame_header_full` needs the
//! reference state of every earlier frame, so this reuses the same `RefFrameState`-threading
//! pattern to reach the target frame's real `FrameHeader`.

use bitvue_av1_codec::frame_header_full::{
    find_frame_header_payloads, parse_frame_header_full, thread_ref_state_before,
};
use bitvue_av1_codec::overlay_extraction::{
    extract_deblocking_data_from_parsed, DeblockingData, DeblockingEdge,
};
use serde_json::{json, Value};

pub fn get_deblocking_analysis(data: &[u8], frame_index: usize) -> Result<Value, String> {
    let (_hdr, frames) = bitvue_av1_codec::ivf::parse_ivf_frames(data)
        .map_err(|e| format!("IVF parse error: {e}"))?;
    if frame_index >= frames.len() {
        return Err(format!(
            "frame_index {frame_index} out of range (stream has {} frames)",
            frames.len()
        ));
    }
    let seq = bitvue_av1_codec::overlay_extraction::find_sequence_header(&frames)
        .map(|(_, seq)| seq)
        .ok_or_else(|| "no sequence header found in the first few frames".to_string())?;

    // The frame the analysis describes is the last one of the packet; its loop filter parameters
    // come from its own header, parsed with the state left by the frames before it (the earlier
    // frames of the same packet included).
    let mut header_ref_state =
        thread_ref_state_before(&frames, &seq, frame_index).map_err(|e| e.to_string())?;
    let mut header = None;
    for payload in find_frame_header_payloads(&frames[frame_index].data) {
        header = Some(
            parse_frame_header_full(&payload, &seq, &mut header_ref_state)
                .map_err(|e| e.to_string())?,
        );
    }
    let header = header.ok_or_else(|| "frame has no Frame/FrameHeader OBU".to_string())?;

    let parsed =
        crate::analysis_session::analyzed_frame(data, frame_index).map_err(|e| e.to_string())?;

    let deblocking =
        extract_deblocking_data_from_parsed(&parsed, &header).map_err(|e| e.to_string())?;

    let mut value = deblocking_to_json(frame_index, &deblocking);
    value["provenance"] =
        json!(bitvue_av1_codec::overlay_extraction::frame_provenance(&parsed).as_str());
    Ok(value)
}

fn edge_to_json(e: &DeblockingEdge) -> Value {
    json!({
        "x": e.x,
        "y": e.y,
        "length": e.length,
        "orientation": if e.vertical { "vertical" } else { "horizontal" },
        "filter_size": e.filter_size,
        "filtered": e.filtered,
        "level": e.level,
    })
}

fn deblocking_to_json(frame_index: usize, d: &DeblockingData) -> Value {
    let count = |keep: &dyn Fn(&DeblockingEdge) -> bool| -> usize {
        d.edges
            .iter()
            .filter(|e| keep(e))
            .map(|e| (e.length / 4) as usize)
            .sum()
    };
    let total = count(&|_| true);
    let filtered = count(&|e| e.filtered);

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
        // The levels use DeltaLF = 0 when the frame signals per-block delta_lf (not kept by the parser).
        "delta_lf_ignored": d.delta_lf_ignored,
        "stats": {
            "edge_segments": total,
            "filtered_segments": filtered,
            "unfiltered_segments": total - filtered,
            "filtered_16": count(&|e| e.filtered && e.filter_size == 16),
            "filtered_8": count(&|e| e.filtered && e.filter_size == 8),
            "filtered_4": count(&|e| e.filtered && e.filter_size == 4),
        },
    })
}

#[derive(serde::Deserialize)]
struct GetDeblockingAnalysisParams {
    frame_index: usize,
}

/// AV1 loop-filter edges + real loop-filter parameters for one frame of stream A --
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
        Err(err) => return crate::command_support::engine_error_response(request.id, &err),
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
        // AV1 has no boundary strength: an edge carries the largest filter it allows (4, 8 or 16
        // samples) and its level, both numbers.
        for e in edges {
            assert!(
                matches!(e["filter_size"].as_u64().unwrap(), 4 | 8 | 16),
                "{e}"
            );
            assert!(e["level"].as_u64().unwrap() <= 63);
            assert!(e["length"].as_u64().unwrap() % 4 == 0);
        }
        let stats = &result["stats"];
        let total = stats["edge_segments"].as_u64().unwrap();
        assert!(total > 0);
        assert_eq!(
            stats["filtered_segments"].as_u64().unwrap()
                + stats["unfiltered_segments"].as_u64().unwrap(),
            total
        );
        assert_eq!(
            stats["filtered_16"].as_u64().unwrap()
                + stats["filtered_8"].as_u64().unwrap()
                + stats["filtered_4"].as_u64().unwrap(),
            stats["filtered_segments"].as_u64().unwrap(),
            "every filtered segment has one of the three sizes"
        );
        assert_eq!(result["delta_lf_ignored"], false);
    }

    /// Every edge of a real frame obeys what the spec derivation guarantees, key frame and inter
    /// frame alike (spec 7.14.2-7.14.5), without re-deriving the values.
    #[test]
    fn real_frames_keep_the_loop_filter_invariants() {
        for frame_index in [0usize, 1, 5, 20, 100] {
            let result = get_deblocking_analysis(AV1_IVF_FIXTURE, frame_index).unwrap();
            let (w, h) = (
                result["width"].as_u64().unwrap(),
                result["height"].as_u64().unwrap(),
            );
            let level = result["params"]["level"].clone();
            for e in result["edges"].as_array().unwrap() {
                let (x, y, len) = (
                    e["x"].as_u64().unwrap(),
                    e["y"].as_u64().unwrap(),
                    e["length"].as_u64().unwrap(),
                );
                let vertical = e["orientation"] == "vertical";
                let size = e["filter_size"].as_u64().unwrap();
                assert!(matches!(size, 4 | 8 | 16), "frame {frame_index}: {e}");
                assert!(x % 4 == 0 && y % 4 == 0 && len % 4 == 0 && len > 0, "{e}");
                assert!(x < w && y < h, "on screen: {e}");
                // never on the frame border: the border has no block across it
                assert!(if vertical { x > 0 } else { y > 0 }, "{e}");
                // a size-16 filter needs transforms of 16 or more on both sides, so the edge sits
                // on a multiple of its size
                let across = if vertical { x } else { y };
                assert!(across % size == 0, "frame {frame_index}: {e}");
                // a filtered edge has a positive level, an unfiltered one is either level 0 or a
                // transform edge inside a skipped inter block
                let lvl = e["level"].as_u64().unwrap();
                if e["filtered"] == true {
                    assert!(lvl > 0, "{e}");
                }
                // an edge of a direction whose frame level is 0 can only have a level from deltas
                // or segments; with no deltas and no segmentation it is 0
                let base = level[if vertical { 0 } else { 1 }].as_u64().unwrap();
                if base == 0 && result["params"]["delta_enabled"] == false {
                    assert_eq!(lvl, 0, "{e}");
                }
            }
        }
    }

    /// Every block of a key frame is intra, so `applyFilter` is always 1: an edge is filtered
    /// exactly when its level is above zero.
    #[test]
    fn on_a_key_frame_an_edge_is_filtered_exactly_when_its_level_is_positive() {
        let result = get_deblocking_analysis(AV1_IVF_FIXTURE, 0).unwrap();
        for e in result["edges"].as_array().unwrap() {
            assert_eq!(e["filtered"], e["level"].as_u64().unwrap() > 0, "{e}");
        }
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

    #[test]
    fn get_deblocking_analysis_reports_whether_the_frame_was_verified() {
        // The fixture decodes exactly, packet by packet; a damaged inter frame does not.
        assert_eq!(
            get_deblocking_analysis(AV1_IVF_FIXTURE, 0).unwrap()["provenance"],
            "verified"
        );
        assert_eq!(
            get_deblocking_analysis(AV1_IVF_FIXTURE, 2).unwrap()["provenance"],
            "verified"
        );
        assert_eq!(
            get_deblocking_analysis(
                &crate::test_support::stream_with_a_corrupted_inter_frame(),
                2
            )
            .unwrap()["provenance"],
            "unverified"
        );
    }
}
