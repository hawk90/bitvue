//! `get_av1_features` command backing -- CDEF/loop-restoration/film-grain/super-resolution data
//! for one frame of stream A, feeding `frontend/components/Player/views/AV1FeaturesView.tsx`
//! (full dashboard, `currentMode === "av1-features"`) and `frontend/hooks/useAv1Features.ts`
//! (single-feature overlay modes on `VideoCanvas`). AV1/IVF only.
//!
//! The extraction functions this calls (`bitvue_av1_codec::advanced_features::extract_cdef_data`
//! etc.) already existed and worked -- the real gap was that nothing in the codebase ever
//! produced a `FrameHeader` with `cdef_damping`/`loop_restoration`/`film_grain`/
//! `super_resolution` populated. That gap is `bitvue_av1_codec::frame_header_full`'s
//! `parse_frame_header_full` -- see its module doc for why this needs a *sequential* scan from
//! frame 0 (not a single isolated frame, unlike `frame_analysis::get_frame_analysis`) and for the
//! documented unsupported case (short reference-frame signaling).

use bitvue_av1_codec::advanced_features::{
    extract_cdef_data, extract_film_grain_data, extract_loop_restoration_data,
    extract_super_resolution_data, CdefBlock, CdefData, FilmGrainData, LoopRestorationData,
    RestorationUnit, SuperResolutionData,
};
use bitvue_av1_codec::frame_header_full::{
    find_frame_header_payload, parse_frame_header_full, RefFrameState,
};
use bitvue_av1_codec::obu::{ObuIterator, ObuType};
use bitvue_av1_codec::sequence::{parse_sequence_header, SequenceHeader};
use serde_json::{json, Value};

/// Fixed AV1 CDEF operating block size -- always 8x8 per spec, not a per-stream/per-frame value,
/// so `extract_cdef_data` doesn't return it on `CdefData`; injected here at the wire boundary
/// since the frontend's `Av1CdefData`/`AV1FeaturesView` inline type both expect it.
const CDEF_BLOCK_SIZE: u32 = 8;

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

/// `frame_index` is the display index (decode order, same convention as every other stream-A
/// command). Scans every frame from 0 up to and including `frame_index` to build the reference
/// order-hint state `parse_frame_header_full` needs -- see module doc.
pub fn get_av1_features(data: &[u8], frame_index: usize) -> Result<Value, String> {
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

    let mut ref_state = RefFrameState::new();
    let mut header = None;
    for frame in frames.iter().take(frame_index + 1) {
        let payload = find_frame_header_payload(&frame.data)
            .ok_or_else(|| "frame has no Frame/FrameHeader OBU".to_string())?;
        header = Some(
            parse_frame_header_full(&payload, &seq, &mut ref_state).map_err(|e| e.to_string())?,
        );
    }
    let header = header.expect("frame_index < frames.len() guarantees at least one iteration");

    let cdef = extract_cdef_data(&header);
    let loop_restoration = extract_loop_restoration_data(&header);
    let film_grain = extract_film_grain_data(&header);
    let super_resolution = extract_super_resolution_data(&header);

    Ok(json!({
        "frame_index": frame_index,
        "cdef": cdef.as_ref().map(cdef_to_json),
        "loop_restoration": loop_restoration.as_ref().map(loop_restoration_to_json),
        "film_grain": film_grain.as_ref().map(film_grain_to_json),
        "super_resolution": super_resolution.as_ref().map(super_resolution_to_json),
    }))
}

fn cdef_block_to_json(b: &CdefBlock) -> Value {
    json!({ "x": b.x, "y": b.y, "size": b.size, "direction": b.direction, "strength": b.strength })
}

fn cdef_to_json(d: &CdefData) -> Value {
    json!({
        "width": d.width,
        "height": d.height,
        "block_size": CDEF_BLOCK_SIZE,
        "blocks": d.block_strengths.iter().map(cdef_block_to_json).collect::<Vec<_>>(),
        "damping": d.damping,
        "y_primary_strength": d.y_primary_strength,
        "y_secondary_strength": d.y_secondary_strength,
    })
}

fn restoration_unit_to_json(u: &RestorationUnit) -> Value {
    json!({ "x": u.x, "y": u.y, "size": u.size, "restoration_type": u.restoration_type as u8 })
}

fn loop_restoration_to_json(d: &LoopRestorationData) -> Value {
    json!({
        "width": d.width,
        "height": d.height,
        "unit_size": d.unit_size,
        "y_type": d.y_restoration_type as u8,
        "units": d.units.iter().map(restoration_unit_to_json).collect::<Vec<_>>(),
    })
}

fn film_grain_to_json(d: &FilmGrainData) -> Value {
    json!({
        "enabled": d.enabled,
        "seed": d.seed,
        "scaling_shift": d.scaling_shift,
        "ar_coeff_lag": d.ar_coeff_lag,
        "chroma_scaling_from_luma": d.chroma_scaling_from_luma,
        "overlap": d.overlap,
    })
}

fn super_resolution_to_json(d: &SuperResolutionData) -> Value {
    json!({
        "enabled": d.enabled,
        "scale_denominator": d.scale_denominator,
        "upscaled_width": d.upscaled_width,
        "upscaled_height": d.upscaled_height,
    })
}

#[derive(serde::Deserialize)]
struct GetAv1FeaturesParams {
    frame_index: usize,
}

/// CDEF/loop-restoration/film-grain/super-resolution data for one frame of stream A -- see this
/// module's doc. Control-only, same reasoning as `frame_analysis::get_frame_analysis_command`.
pub fn get_av1_features_command(
    core: &bitvue_engine::Core,
    request: &bitvue_protocol::Request,
) -> bitvue_protocol::Response {
    use bitvue_protocol::{Response, WireError, WireErrorCode};

    let params: GetAv1FeaturesParams = match serde_json::from_value(request.params.clone()) {
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

    match get_av1_features(data, params.frame_index) {
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
    fn get_av1_features_end_to_end_returns_real_cdef_data() {
        let result = get_av1_features(AV1_IVF_FIXTURE, 0).unwrap();
        assert_eq!(result["frame_index"], 0);
        let cdef = &result["cdef"];
        assert!(
            !cdef.is_null(),
            "fixture's sequence header has enable_cdef=true"
        );
        assert_eq!(cdef["width"], 320);
        assert_eq!(cdef["height"], 240);
        assert_eq!(cdef["block_size"], 8);
        assert!(!cdef["blocks"].as_array().unwrap().is_empty());
        // Numeric, not a variant-name string -- pins the enum-as-u8 wire mapping.
        assert!(
            result["loop_restoration"].is_null()
                || result["loop_restoration"]["y_type"].is_number()
        );
    }

    #[test]
    fn get_av1_features_non_first_frame_uses_sequential_ref_state() {
        // Proves the sequential-scan + RefFrameState threading actually runs across many real
        // inter frames without erroring (skip_mode_params/global_motion/tile_info all touch
        // per-frame bits that must stay correctly aligned frame after frame).
        let result = get_av1_features(AV1_IVF_FIXTURE, 100).unwrap();
        assert_eq!(result["frame_index"], 100);
    }

    #[test]
    fn get_av1_features_out_of_range_frame_index_is_a_real_error() {
        assert!(get_av1_features(AV1_IVF_FIXTURE, 999_999).is_err());
    }

    #[test]
    fn get_av1_features_last_frame_of_the_real_fixture_parses() {
        let (_hdr, frames) = bitvue_av1_codec::ivf::parse_ivf_frames(AV1_IVF_FIXTURE).unwrap();
        let result = get_av1_features(AV1_IVF_FIXTURE, frames.len() - 1).unwrap();
        assert_eq!(result["frame_index"], frames.len() - 1);
    }

    #[test]
    fn get_av1_features_command_end_to_end_returns_real_cdef_data() {
        let core = Core::new();
        open_real_fixture(&core, "A");

        let response = crate::dispatch(
            &core,
            &Request {
                id: 223,
                method: "get_av1_features".to_string(),
                params: serde_json::json!({"frame_index": 0}),
            },
        );
        assert!(response.ok, "expected ok response, got {response:?}");
        let result = response.result.unwrap();
        assert_eq!(result["frame_index"], 0);
        assert!(!result["cdef"].is_null());
        assert_eq!(result["cdef"]["width"], 320);
        assert_eq!(result["cdef"]["height"], 240);
    }

    #[test]
    fn get_av1_features_command_stream_not_open_returns_not_found() {
        let core = Core::new();
        let response = crate::dispatch(
            &core,
            &Request {
                id: 224,
                method: "get_av1_features".to_string(),
                params: serde_json::json!({"frame_index": 0}),
            },
        );
        assert!(!response.ok);
        assert_eq!(response.error.unwrap().code, WireErrorCode::NotFound);
    }
}
