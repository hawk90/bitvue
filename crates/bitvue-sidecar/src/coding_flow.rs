//! `get_coding_flow_analysis` command backing -- encoder/decoder pipeline stage completion and
//! real sequence-header codec features for one frame of stream A, feeding
//! `frontend/components/Player/views/CodingFlowView.tsx`. AV1/IVF only.
//!
//! No new bitstream parsing here -- this reuses `frame_analysis`'s grid extraction (prediction
//! mode / transform / QP grids already tell us how far the analysis pipeline reaches for a given
//! frame) and `av1_features`'s sequence-header lookup (its boolean flags ARE the real codec
//! features, more accurate than the frontend's static per-codec-name table). "Entropy" and
//! "reconstruction" are always reported incomplete: this codebase has no residual/coefficient
//! entropy decode stage (see `bitvue-av1-codec::tile` module doc, `⏳ pending`) and no pixel
//! reconstruction step in the analysis path (the debug-YUV / `get_decoded_frame_yuv` commands
//! decode real pixels via `dav1d`, a separate code path this command doesn't touch).

use bitvue_av1_codec::frame_header_full::thread_ref_state_before;
use bitvue_av1_codec::obu::{ObuIterator, ObuType};
use bitvue_av1_codec::overlay_extraction::{
    extract_prediction_mode_grid_from_parsed, extract_qp_grid_from_parsed,
    extract_transform_grid_from_parsed, ParsedFrame,
};
use bitvue_av1_codec::sequence::{parse_sequence_header, SequenceHeader};
use serde_json::{json, Value};

const SEQUENCE_HEADER_SCAN_LIMIT: usize = 8;

/// Ordered to match the frontend's `ENCODER_STAGES`/`CODING_STAGES` (`CodingFlowView.tsx`) --
/// `current_stage` is the last entry in this list whose `completed` came back true.
const STAGE_ORDER: &[(&str, &str)] = &[
    ("input", "Input"),
    ("prediction", "Prediction"),
    ("transform", "Transform"),
    ("quantization", "Quantization"),
    ("entropy", "Entropy Coding"),
    ("reconstruction", "Reconstruction"),
];

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

/// Human-readable names for whichever `SequenceHeader` tool flags are actually enabled in this
/// stream -- real per-stream data, not the frontend's static per-codec-name guess.
fn codec_features(seq: &SequenceHeader) -> Vec<&'static str> {
    let mut features = Vec::new();
    if seq.enable_cdef {
        features.push("CDEF");
    }
    if seq.enable_restoration {
        features.push("Loop Restoration");
    }
    if seq.film_grain_params_present {
        features.push("Film Grain");
    }
    if seq.enable_superres {
        features.push("Super Resolution");
    }
    if seq.enable_warped_motion {
        features.push("Warped Motion");
    }
    if seq.enable_dual_filter {
        features.push("Dual Filter");
    }
    if seq.enable_jnt_comp {
        features.push("Distance-Weighted Compound");
    }
    if seq.enable_masked_compound {
        features.push("Masked Compound");
    }
    if seq.enable_interintra_compound {
        features.push("Inter-Intra Compound");
    }
    if seq.enable_ref_frame_mvs {
        features.push("Reference Frame MVs");
    }
    if seq.enable_filter_intra {
        features.push("Filter Intra");
    }
    if seq.enable_intra_edge_filter {
        features.push("Intra Edge Filter");
    }
    if seq.use_128x128_superblock {
        features.push("128x128 Superblocks");
    }
    if seq.seq_choose_screen_content_tools || seq.seq_force_screen_content_tools != 0 {
        features.push("Screen Content Tools");
    }
    if seq.enable_order_hint {
        features.push("Order Hint");
    }
    features
}

pub fn get_coding_flow_analysis(data: &[u8], frame_index: usize) -> Result<Value, String> {
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

    let obu_data: Vec<u8> = match find_sequence_header_bytes(&frames) {
        Some(seq_bytes) => [seq_bytes.as_slice(), frames[frame_index].data.as_slice()].concat(),
        None => frames[frame_index].data.clone(),
    };
    // See `residual_analysis`/`frame_analysis`'s identical comment: a fresh-state parse silently
    // desyncs `tile_data` extraction for any frame needing real `skip_mode_params` state.
    let mut ref_state =
        thread_ref_state_before(&frames, &seq, frame_index).map_err(|e| e.to_string())?;
    let parsed =
        ParsedFrame::parse_with_ref_state(&obu_data, &mut ref_state).map_err(|e| e.to_string())?;
    let base_qp = parsed.frame_type.base_qp.unwrap_or(0) as i16;

    let prediction_mode_grid =
        extract_prediction_mode_grid_from_parsed(&parsed).map_err(|e| e.to_string())?;
    let transform_grid = extract_transform_grid_from_parsed(&parsed).map_err(|e| e.to_string())?;
    let qp_grid =
        extract_qp_grid_from_parsed(&parsed, frame_index, base_qp).map_err(|e| e.to_string())?;

    let prediction_count = prediction_mode_grid
        .modes
        .iter()
        .filter(|m| m.is_some())
        .count();
    let transform_count = transform_grid
        .tx_sizes
        .iter()
        .filter(|t| t.is_some())
        .count();
    let qp_count = qp_grid.qp.len();

    // (completed, data_size) per stage, same order as STAGE_ORDER.
    let stage_data: [(bool, Option<u64>); 6] = [
        (true, Some(frames[frame_index].data.len() as u64)),
        (prediction_count > 0, Some(prediction_count as u64)),
        (transform_count > 0, Some(transform_count as u64)),
        (qp_count > 0, Some(qp_count as u64)),
        (false, None), // entropy: no coefficient decode stage in this codebase yet
        (false, None), // reconstruction: pixel decode is a separate code path (dav1d), not this one
    ];

    let stages: Vec<Value> = STAGE_ORDER
        .iter()
        .zip(stage_data.iter())
        .map(|((id, label), (completed, data_size))| {
            json!({
                "id": id,
                "label": label,
                "completed": completed,
                "data_size": data_size,
            })
        })
        .collect();

    let current_stage = STAGE_ORDER
        .iter()
        .zip(stage_data.iter())
        .rfind(|(_, (completed, _))| *completed)
        .map(|((id, _), _)| *id)
        .unwrap_or("input");

    Ok(json!({
        "frame_index": frame_index,
        "stages": stages,
        "current_stage": current_stage,
        "codec_features": codec_features(&seq),
    }))
}

#[derive(serde::Deserialize)]
struct GetCodingFlowAnalysisParams {
    frame_index: usize,
}

/// Encoder/decoder pipeline stage completion + real codec features for one frame of stream A --
/// see this module's doc. Control-only, same reasoning as
/// `frame_analysis::get_frame_analysis_command`.
pub fn get_coding_flow_analysis_command(
    core: &bitvue_engine::Core,
    request: &bitvue_protocol::Request,
) -> bitvue_protocol::Response {
    use bitvue_protocol::{Response, WireError, WireErrorCode};

    let params: GetCodingFlowAnalysisParams = match serde_json::from_value(request.params.clone()) {
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

    match get_coding_flow_analysis(data, params.frame_index) {
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
    use crate::test_support::open_real_fixture;
    use bitvue_engine::Core;
    use bitvue_protocol::{Request, WireErrorCode};

    const AV1_IVF_FIXTURE: &[u8] = include_bytes!("../../../test_data/av1_test.ivf");

    #[test]
    fn get_coding_flow_analysis_reaches_quantization_for_a_real_frame() {
        let result = get_coding_flow_analysis(AV1_IVF_FIXTURE, 0).unwrap();
        assert_eq!(result["frame_index"], 0);
        assert_eq!(result["current_stage"], "quantization");

        let stages = result["stages"].as_array().unwrap();
        assert_eq!(stages.len(), 6);
        let by_id = |id: &str| stages.iter().find(|s| s["id"] == id).unwrap();
        assert_eq!(by_id("input")["completed"], true);
        assert_eq!(by_id("prediction")["completed"], true);
        assert_eq!(by_id("transform")["completed"], true);
        assert_eq!(by_id("quantization")["completed"], true);
        assert_eq!(by_id("entropy")["completed"], false);
        assert!(by_id("entropy")["data_size"].is_null());
        assert_eq!(by_id("reconstruction")["completed"], false);
        assert!(by_id("reconstruction")["data_size"].is_null());
    }

    #[test]
    fn get_coding_flow_analysis_reports_real_sequence_header_features() {
        let result = get_coding_flow_analysis(AV1_IVF_FIXTURE, 0).unwrap();
        let features = result["codec_features"].as_array().unwrap();
        assert!(
            features.iter().any(|f| f == "CDEF"),
            "fixture's sequence header has enable_cdef=true, expected CDEF in {features:?}"
        );
    }

    #[test]
    fn get_coding_flow_analysis_non_first_frame_still_works() {
        let result = get_coding_flow_analysis(AV1_IVF_FIXTURE, 5).unwrap();
        assert_eq!(result["frame_index"], 5);
        assert_eq!(result["current_stage"], "quantization");
    }

    #[test]
    fn get_coding_flow_analysis_out_of_range_frame_index_is_a_real_error() {
        assert!(get_coding_flow_analysis(AV1_IVF_FIXTURE, 999_999).is_err());
    }

    #[test]
    fn get_coding_flow_analysis_command_end_to_end_reaches_quantization() {
        let core = Core::new();
        open_real_fixture(&core, "A");

        let response = crate::dispatch(
            &core,
            &Request {
                id: 225,
                method: "get_coding_flow_analysis".to_string(),
                params: serde_json::json!({"frame_index": 0}),
            },
        );
        assert!(response.ok, "expected ok response, got {response:?}");
        let result = response.result.unwrap();
        assert_eq!(result["frame_index"], 0);
        assert_eq!(result["current_stage"], "quantization");
        assert_eq!(result["stages"].as_array().unwrap().len(), 6);
        assert!(!result["codec_features"].as_array().unwrap().is_empty());
    }

    #[test]
    fn get_coding_flow_analysis_command_stream_not_open_returns_not_found() {
        let core = Core::new();
        let response = crate::dispatch(
            &core,
            &Request {
                id: 226,
                method: "get_coding_flow_analysis".to_string(),
                params: serde_json::json!({"frame_index": 0}),
            },
        );
        assert!(!response.ok);
        assert_eq!(response.error.unwrap().code, WireErrorCode::NotFound);
    }
}
