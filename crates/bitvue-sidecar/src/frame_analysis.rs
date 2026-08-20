//! `get_frame_analysis` command backing -- QP/MV/partition/prediction-mode/transform-size grids
//! for one frame of stream A, feeding the main viewer's overlay renderers
//! (`frontend/components/panels/OverlayRenderer/renderers/*.tsx` -- QPMapRenderer,
//! MVFieldRenderer, CodingFlowRenderer, PredictionRenderer, TransformRenderer). AV1/IVF only.
//!
//! All the real extraction logic already existed in `bitvue-av1-codec::overlay_extraction`
//! (`extract_qp_grid_from_parsed`/`extract_mv_grid_from_parsed`/
//! `extract_partition_grid_from_parsed`/`extract_prediction_mode_grid_from_parsed`/
//! `extract_transform_grid_from_parsed`) -- fully implemented and unit-tested, just never called
//! from anywhere in the app (not `bitvue-cli`, not `bitvue-sidecar`) until now. This module is
//! orchestration (locating each frame's OBU bytes + a sequence header for real dimensions) and
//! wire mapping, not new bitstream-parsing work.
//!
//! **Wire mapping note**: `QPGrid`/`MVGrid`/`PartitionGrid` derive `Serialize` in
//! `bitvue-engine`, but their enum fields (`PartitionType`, `BlockMode`) would serialize as
//! variant *name strings* under the default derive ("Split", "Inter") -- the frontend's
//! `PartitionType`/`BlockMode` are numeric TS enums expecting the `#[repr(u8)]` discriminant
//! instead. Same issue for `PredictionMode`/`TxSize` (`bitvue-av1-codec`, no `Serialize` derive
//! at all). Every grid here is hand-mapped to JSON explicitly for this reason -- same reasoning
//! `main.rs`'s module doc gives for hand-mapping `bitvue-engine` types generally.

use bitvue_av1_codec::obu::{ObuIterator, ObuType};
use bitvue_av1_codec::overlay_extraction::{
    extract_energy_grid_from_parsed, extract_mv_grid_from_parsed,
    extract_partition_grid_from_parsed, extract_prediction_mode_grid_from_parsed,
    extract_qp_grid_from_parsed, extract_transform_grid_from_parsed, EnergyGrid, ParsedFrame,
    PredictionModeGrid, TransformGrid,
};
use bitvue_av1_codec::tile::PredictionMode;
use bitvue_engine::mv_overlay::{MVGrid, MotionVector};
use bitvue_engine::partition_grid::{PartitionBlock, PartitionGrid};
use bitvue_engine::qp_heatmap::QPGrid;
use serde_json::{json, Value};

/// AV1 streams typically carry the sequence header once (frame 0), not repeated every frame --
/// but `ParsedFrame::parse` needs one present in whatever `obu_data` it's given to resolve real
/// frame dimensions (falls back to a hardcoded 1920x1080 scaffold otherwise, see its doc). Scans
/// a bounded prefix of frames rather than the whole stream -- if the sequence header isn't within
/// the first several frames, something unusual is going on and grids will fall back to the
/// scaffold dimensions rather than this function scanning arbitrarily far into a long stream.
const SEQUENCE_HEADER_SCAN_LIMIT: usize = 8;

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

pub fn get_frame_analysis(data: &[u8], frame_index: usize) -> Result<Value, String> {
    let (_hdr, frames) = bitvue_av1_codec::ivf::parse_ivf_frames(data)
        .map_err(|e| format!("IVF parse error: {e}"))?;
    if frame_index >= frames.len() {
        return Err(format!(
            "frame_index {frame_index} out of range (stream has {} frames)",
            frames.len()
        ));
    }

    // Prepending the sequence header is idempotent -- if the target frame's own chunk already
    // has one (or none is found at all), this still parses correctly.
    let obu_data: Vec<u8> = match find_sequence_header_bytes(&frames) {
        Some(seq) => [seq.as_slice(), frames[frame_index].data.as_slice()].concat(),
        None => frames[frame_index].data.clone(),
    };

    let parsed = ParsedFrame::parse(&obu_data).map_err(|e| e.to_string())?;
    let base_qp = parsed.frame_type.base_qp.unwrap_or(0) as i16;

    let qp_grid =
        extract_qp_grid_from_parsed(&parsed, frame_index, base_qp).map_err(|e| e.to_string())?;
    let mv_grid = extract_mv_grid_from_parsed(&parsed).map_err(|e| e.to_string())?;
    let partition_grid = extract_partition_grid_from_parsed(&parsed).map_err(|e| e.to_string())?;
    let prediction_mode_grid =
        extract_prediction_mode_grid_from_parsed(&parsed).map_err(|e| e.to_string())?;
    let transform_grid = extract_transform_grid_from_parsed(&parsed).map_err(|e| e.to_string())?;
    let energy_grid = extract_energy_grid_from_parsed(&parsed).map_err(|e| e.to_string())?;

    Ok(json!({
        "frame_index": frame_index,
        "width": parsed.dimensions.width,
        "height": parsed.dimensions.height,
        "qp_grid": qp_grid_to_json(&qp_grid),
        "mv_grid": mv_grid_to_json(&mv_grid),
        "partition_grid": partition_grid_to_json(&partition_grid),
        "prediction_mode_grid": prediction_mode_grid_to_json(&prediction_mode_grid),
        "transform_grid": transform_grid_to_json(&transform_grid),
        "energy_grid": energy_grid_to_json(&energy_grid),
    }))
}

fn qp_grid_to_json(g: &QPGrid) -> Value {
    json!({
        "grid_w": g.grid_w,
        "grid_h": g.grid_h,
        "block_w": g.block_w,
        "block_h": g.block_h,
        "qp": g.qp,
        "qp_min": g.qp_min,
        "qp_max": g.qp_max,
    })
}

fn energy_grid_to_json(g: &EnergyGrid) -> Value {
    json!({
        "grid_w": g.grid_w,
        "grid_h": g.grid_h,
        "block_w": g.block_w,
        "block_h": g.block_h,
        "energy_bpp": g.energy_bpp,
    })
}

fn motion_vector_to_json(mv: &MotionVector) -> Value {
    json!({ "dx_qpel": mv.dx_qpel, "dy_qpel": mv.dy_qpel })
}

fn mv_grid_to_json(g: &MVGrid) -> Value {
    json!({
        "coded_width": g.coded_width,
        "coded_height": g.coded_height,
        "block_w": g.block_w,
        "block_h": g.block_h,
        "grid_w": g.grid_w,
        "grid_h": g.grid_h,
        "mv_l0": g.mv_l0.iter().map(motion_vector_to_json).collect::<Vec<_>>(),
        "mv_l1": g.mv_l1.iter().map(motion_vector_to_json).collect::<Vec<_>>(),
        "mode": g.mode.as_ref().map(|modes| modes.iter().map(|m| *m as u8).collect::<Vec<u8>>()),
    })
}

fn partition_block_to_json(b: &PartitionBlock) -> Value {
    json!({
        "x": b.x,
        "y": b.y,
        "width": b.width,
        "height": b.height,
        "partition": b.partition as u8,
        "depth": b.depth,
        "tree_type": b.tree_type,
    })
}

fn partition_grid_to_json(g: &PartitionGrid) -> Value {
    json!({
        "coded_width": g.coded_width,
        "coded_height": g.coded_height,
        "sb_size": g.sb_size,
        "blocks": g.blocks.iter().map(partition_block_to_json).collect::<Vec<_>>(),
    })
}

/// `bitvue_av1_codec::tile::PredictionMode`'s declaration order already matches the real AV1
/// spec's `y_mode` numbering for the 13 intra modes (`DcPred`=0 .. `PaethPred`=12) -- but the
/// frontend's `PREDICTION_MODE_COLORS`/`getPredictionModeName` (`utils/colors.ts`) puts inter
/// modes at 64+ to keep them visually/logically separate from the intra range, which our enum's
/// natural sequential discriminants (13..16) don't provide. Remapped explicitly rather than cast,
/// unlike `PartitionType`/`TxSize` where the raw discriminant already lines up with the frontend.
///
/// The 8 compound modes (`is_compound()`) all collapse to wire value 69 ("Compound modes"), the
/// single bucket the frontend already reserves -- it doesn't yet distinguish NEAREST_NEARESTMV
/// from NEW_NEWMV etc. visually. If per-compound-mode color/label detail is ever wanted, extend
/// `PREDICTION_MODE_COLORS`/`getPredictionModeName` with 8 new slots (70..=77) first.
fn prediction_mode_to_wire(mode: PredictionMode) -> u8 {
    match mode {
        PredictionMode::DcPred => 0,
        PredictionMode::VPred => 1,
        PredictionMode::HPred => 2,
        PredictionMode::D45Pred => 3,
        PredictionMode::D135Pred => 4,
        PredictionMode::D113Pred => 5,
        PredictionMode::D157Pred => 6,
        PredictionMode::D203Pred => 7,
        PredictionMode::D67Pred => 8,
        PredictionMode::SmoothPred => 9,
        PredictionMode::SmoothVPred => 10,
        PredictionMode::SmoothHPred => 11,
        PredictionMode::PaethPred => 12,
        PredictionMode::NewMv => 64,
        PredictionMode::NearestMv => 65,
        PredictionMode::NearMv => 66,
        PredictionMode::GlobalMv => 68,
        PredictionMode::NearestNearestMv
        | PredictionMode::NearNearMv
        | PredictionMode::NearestNewMv
        | PredictionMode::NewNearestMv
        | PredictionMode::NearNewMv
        | PredictionMode::NewNearMv
        | PredictionMode::GlobalGlobalMv
        | PredictionMode::NewNewMv => 69,
    }
}

fn prediction_mode_grid_to_json(g: &PredictionModeGrid) -> Value {
    json!({
        "coded_width": g.coded_width,
        "coded_height": g.coded_height,
        "block_w": g.block_w,
        "block_h": g.block_h,
        "grid_w": g.grid_w,
        "grid_h": g.grid_h,
        "modes": g.modes.iter().map(|m| m.map(prediction_mode_to_wire)).collect::<Vec<_>>(),
    })
}

fn transform_grid_to_json(g: &TransformGrid) -> Value {
    json!({
        "coded_width": g.coded_width,
        "coded_height": g.coded_height,
        "block_w": g.block_w,
        "block_h": g.block_h,
        "grid_w": g.grid_w,
        "grid_h": g.grid_h,
        // TxSize's declaration order (Tx4x4=0..Tx64x64=4) already matches the frontend's
        // expected numbering exactly -- direct cast, no remap table needed (unlike PredictionMode).
        "tx_sizes": g.tx_sizes.iter().map(|t| t.map(|tx| tx as u8)).collect::<Vec<_>>(),
    })
}

#[derive(serde::Deserialize)]
struct GetFrameAnalysisParams {
    frame_index: usize,
}

/// QP/MV/partition/prediction-mode/transform-size grids for one frame of stream A -- see this
/// module's doc. Two-frame response (Control metadata + a Data frame of raw `qp_grid.qp` bytes),
/// unlike the rest of this module's grids: `qp_grid.qp` is a flat `Vec<i16>`, one value per block
/// -- structurally the same "big flat numeric array" shape as `get_decoded_frame_yuv`'s pixel
/// planes, not a tree/struct like `mv_grid`/`partition_grid`/`get_frame_syntax`/`get_timeline`
/// (those stay single-JSON; carve-outs are per-grid, not "make this whole command binary"). At
/// 4K with 8px blocks that's 480x270 = 129,600 values -- JSON-encoding each as ASCII digits+comma
/// costs both more CPU (serialize/parse) and more wire bytes than 2 raw bytes/value. Split out
/// 2026-08-20 (axis-3 cleanup); `qp_grid`'s other fields (grid_w/h, block_w/h, qp_min/max) stay
/// small ints in the Control JSON, same shape as before -- only the `qp` array itself moves.
/// `qp_bytes` is little-endian `i16` per value, row-major (matches `qp`'s existing layout); the
/// TS side (`frontend/services/bridge/frameAnalysis.ts`) reconstructs `qp: number[]` from it so
/// every consumer above the bridge layer sees the exact same `FrameAnalysisData` shape as before.
pub fn get_frame_analysis_command(
    core: &bitvue_engine::Core,
    request: &bitvue_protocol::Request,
) -> Vec<(bitvue_protocol::FrameKind, Vec<u8>)> {
    use crate::command_support::single_control_frame;
    use bitvue_protocol::{FrameKind, Response, WireError, WireErrorCode};

    let params: GetFrameAnalysisParams = match serde_json::from_value(request.params.clone()) {
        Ok(p) => p,
        Err(err) => {
            return single_control_frame(Response::failure(
                request.id,
                WireError {
                    code: WireErrorCode::InvalidData,
                    message: err.to_string(),
                    offset: None,
                },
            ))
        }
    };

    let stream_state = core.get_stream(bitvue_engine::StreamId::A);
    let state = stream_state.read();
    let byte_cache = match state.byte_cache.as_ref() {
        Some(cache) => std::sync::Arc::clone(cache),
        None => {
            return single_control_frame(Response::failure(
                request.id,
                WireError {
                    code: WireErrorCode::NotFound,
                    message: "stream not open".to_string(),
                    offset: None,
                },
            ))
        }
    };
    drop(state);

    let full_len = byte_cache.len() as usize;
    let data = match byte_cache.read_range(0, full_len) {
        Ok(bytes) => bytes,
        Err(err) => {
            return single_control_frame(Response::failure(
                request.id,
                WireError {
                    code: crate::command_support::wire_error_code_for(&err),
                    message: err.to_string(),
                    offset: None,
                },
            ))
        }
    };

    let mut value = match get_frame_analysis(data, params.frame_index) {
        Ok(value) => value,
        Err(message) => {
            return single_control_frame(Response::failure(
                request.id,
                WireError {
                    code: WireErrorCode::FrameNotFound,
                    message,
                    offset: None,
                },
            ))
        }
    };

    // Pull qp_grid.qp out of the JSON before serializing the Control frame -- see this fn's doc.
    // Falls back to an empty Data frame (rather than failing the whole response) if qp_grid or
    // qp is somehow absent -- extraction always populates it in practice (extract_qp_grid_from_
    // parsed has its own scaffold-fallback, never omits the field), so this is defensive, not a
    // real expected path.
    let qp_values: Vec<i16> = value
        .get_mut("qp_grid")
        .and_then(|g| g.get_mut("qp"))
        .map(|qp| std::mem::replace(qp, Value::Null))
        .and_then(|qp| qp.as_array().cloned())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_i64())
                .map(|v| v as i16)
                .collect()
        })
        .unwrap_or_default();
    let mut qp_bytes = Vec::with_capacity(qp_values.len() * 2);
    for v in &qp_values {
        qp_bytes.extend_from_slice(&v.to_le_bytes());
    }

    let meta = Response::success(request.id, value);
    vec![
        (
            FrameKind::Control,
            serde_json::to_vec(&meta).expect("Response always serializes"),
        ),
        (FrameKind::Data, qp_bytes),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::open_real_fixture;
    use bitvue_engine::Core;
    use bitvue_protocol::{Request, Response, WireErrorCode};

    const AV1_IVF_FIXTURE: &[u8] = include_bytes!("../../../test_data/av1_test.ivf");

    #[test]
    fn get_frame_analysis_returns_real_dimensions_not_the_1920x1080_scaffold() {
        let result = get_frame_analysis(AV1_IVF_FIXTURE, 0).unwrap();
        // Ground truth: the same fixture's real resolution, independently pinned in
        // decode_bridge's own test (frame_zero_matches_cli_ground_truth...).
        assert_eq!(result["width"], 320);
        assert_eq!(result["height"], 240);
    }

    #[test]
    fn get_frame_analysis_non_first_frame_still_gets_real_dimensions() {
        // Proves the sequence-header-prepend logic actually works -- frame 5's own IVF chunk
        // almost certainly does NOT repeat the sequence header, so if this came back as the
        // 1920x1080 default scaffold instead of the fixture's real 320x240, the prepend is broken.
        let result = get_frame_analysis(AV1_IVF_FIXTURE, 5).unwrap();
        assert_eq!(result["width"], 320);
        assert_eq!(result["height"], 240);
    }

    #[test]
    fn get_frame_analysis_qp_grid_has_real_block_dimensions() {
        let result = get_frame_analysis(AV1_IVF_FIXTURE, 0).unwrap();
        let qp_grid = &result["qp_grid"];
        assert!(qp_grid["grid_w"].as_u64().unwrap() > 0);
        assert!(qp_grid["grid_h"].as_u64().unwrap() > 0);
        let qp = qp_grid["qp"].as_array().unwrap();
        assert_eq!(
            qp.len() as u64,
            qp_grid["grid_w"].as_u64().unwrap() * qp_grid["grid_h"].as_u64().unwrap()
        );
    }

    #[test]
    fn get_frame_analysis_partition_grid_has_real_leaf_blocks() {
        let result = get_frame_analysis(AV1_IVF_FIXTURE, 0).unwrap();
        let blocks = result["partition_grid"]["blocks"].as_array().unwrap();
        assert!(!blocks.is_empty(), "expected real partition leaf blocks");
        // partition must be a small numeric code (0-9 per PartitionType), never a string --
        // pins the enum-as-u8 mapping, not the default Serialize-as-name-string behavior.
        let first_partition = blocks[0]["partition"].as_u64().unwrap();
        assert!(first_partition <= 9);
    }

    #[test]
    fn get_frame_analysis_prediction_modes_are_numeric_not_strings() {
        let result = get_frame_analysis(AV1_IVF_FIXTURE, 0).unwrap();
        let modes = result["prediction_mode_grid"]["modes"].as_array().unwrap();
        assert!(!modes.is_empty());
        // At least one real (non-null) mode should exist and be a number in the documented
        // 0-12 (intra) or 64-68 (inter) ranges.
        let real_mode = modes.iter().find_map(|m| m.as_u64());
        assert!(
            real_mode.is_some_and(|m| m <= 12 || (64..=68).contains(&m)),
            "expected a real prediction mode value, got {modes:?}"
        );
    }

    #[test]
    fn prediction_mode_to_wire_maps_all_8_compound_modes_to_the_shared_bucket() {
        use bitvue_av1_codec::tile::PredictionMode;
        let compound_modes = [
            PredictionMode::NearestNearestMv,
            PredictionMode::NearNearMv,
            PredictionMode::NearestNewMv,
            PredictionMode::NewNearestMv,
            PredictionMode::NearNewMv,
            PredictionMode::NewNearMv,
            PredictionMode::GlobalGlobalMv,
            PredictionMode::NewNewMv,
        ];
        for mode in compound_modes {
            assert_eq!(
                prediction_mode_to_wire(mode),
                69,
                "{mode:?} should map to the frontend's shared 'Compound modes' wire value"
            );
        }
    }

    #[test]
    fn get_frame_analysis_out_of_range_frame_index_is_a_real_error() {
        let result = get_frame_analysis(AV1_IVF_FIXTURE, 999_999);
        assert!(result.is_err());
    }

    #[test]
    fn find_sequence_header_bytes_finds_it_in_a_real_fixture() {
        let (_hdr, frames) = bitvue_av1_codec::ivf::parse_ivf_frames(AV1_IVF_FIXTURE).unwrap();
        let seq = find_sequence_header_bytes(&frames);
        assert!(
            seq.is_some(),
            "expected a real sequence header OBU in the fixture"
        );
    }

    /// Decodes a `get_frame_analysis_command` two-frame response into (Control `Response`, raw
    /// qp_grid.qp bytes), asserting the frame kinds/count along the way -- shared by every test
    /// below instead of repeating the FrameKind/index bookkeeping in each one.
    fn decode_frame_analysis_response(
        frames: Vec<(bitvue_protocol::FrameKind, Vec<u8>)>,
    ) -> (Response, Vec<u8>) {
        assert_eq!(
            frames.len(),
            2,
            "expected Control metadata + Data qp bytes, got {frames:?}"
        );
        assert_eq!(frames[0].0, bitvue_protocol::FrameKind::Control);
        assert_eq!(frames[1].0, bitvue_protocol::FrameKind::Data);
        let response: Response = serde_json::from_slice(&frames[0].1).unwrap();
        (response, frames[1].1.clone())
    }

    #[test]
    fn get_frame_analysis_end_to_end_returns_real_grids() {
        let core = Core::new();
        open_real_fixture(&core, "A");

        let frames = get_frame_analysis_command(
            &core,
            &Request {
                id: 220,
                method: "get_frame_analysis".to_string(),
                params: serde_json::json!({"frame_index": 0}),
            },
        );
        let (response, qp_bytes) = decode_frame_analysis_response(frames);
        assert!(response.ok, "expected ok response, got {response:?}");
        let result = response.result.unwrap();
        assert_eq!(result["frame_index"], 0);
        assert_eq!(result["width"], 320);
        assert_eq!(result["height"], 240);
        // qp_grid.qp itself moved to the Data frame (see this command's doc) -- Control still
        // carries the grid dimensions.
        assert!(result["qp_grid"]["grid_w"].as_u64().unwrap() > 0);
        assert!(result["qp_grid"]["grid_h"].as_u64().unwrap() > 0);
        assert!(!qp_bytes.is_empty());
        assert_eq!(qp_bytes.len() % 2, 0, "qp values are 2 bytes (i16) each");
        assert!(!result["partition_grid"]["blocks"]
            .as_array()
            .unwrap()
            .is_empty());
        let energy = result["energy_grid"]["energy_bpp"].as_array().unwrap();
        assert!(!energy.is_empty());
        assert_eq!(energy.len(), qp_bytes.len() / 2);
    }

    #[test]
    fn get_frame_analysis_qp_bytes_exactly_match_the_pre_wire_qp_array() {
        // Independent-oracle check for the wire encoding step specifically: get_frame_analysis
        // (the pure function, unchanged by the qp_bytes split) still returns the full `qp` array
        // in its JSON `Value` -- decode qp_bytes back to i16 and assert every value matches it
        // exactly, not just that the byte count is plausible (the other end-to-end test above
        // only checks length/parity). Catches encode-order bugs (e.g. column-major vs row-major)
        // and truncation/overflow that a length-only check would miss.
        let core = Core::new();
        open_real_fixture(&core, "A");

        let ground_truth = get_frame_analysis(crate::test_support::AV1_IVF_FIXTURE, 0)
            .expect("pure function should succeed for a real fixture frame");
        let expected_qp: Vec<i64> = ground_truth["qp_grid"]["qp"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_i64().unwrap())
            .collect();
        assert!(!expected_qp.is_empty());

        let frames = get_frame_analysis_command(
            &core,
            &Request {
                id: 223,
                method: "get_frame_analysis".to_string(),
                params: serde_json::json!({"frame_index": 0}),
            },
        );
        let (_response, qp_bytes) = decode_frame_analysis_response(frames);
        let decoded_qp: Vec<i64> = qp_bytes
            .chunks_exact(2)
            .map(|c| i16::from_le_bytes([c[0], c[1]]) as i64)
            .collect();

        assert_eq!(
            decoded_qp, expected_qp,
            "qp_bytes should decode to exactly the same values get_frame_analysis's own JSON \
             qp array has, in the same order"
        );
    }

    #[test]
    fn get_frame_analysis_energy_grid_reflects_real_residual_data() {
        // Not the QP-only proxy this replaced: an all-zero energy grid would mean the fallback
        // (no-tile-data/parse-failure) path was hit instead of real per-CU residual magnitude.
        let core = Core::new();
        open_real_fixture(&core, "A");

        let frames = get_frame_analysis_command(
            &core,
            &Request {
                id: 222,
                method: "get_frame_analysis".to_string(),
                params: serde_json::json!({"frame_index": 0}),
            },
        );
        let (response, _qp_bytes) = decode_frame_analysis_response(frames);
        assert!(response.ok, "expected ok response, got {response:?}");
        let result = response.result.unwrap();
        let energy = result["energy_grid"]["energy_bpp"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_f64().unwrap())
            .collect::<Vec<_>>();
        assert!(
            energy.iter().any(|&v| v > 0.0),
            "expected at least one block with real residual energy on a real I-frame, got all zeros: {energy:?}"
        );
    }

    #[test]
    fn get_frame_analysis_out_of_range_frame_index_is_a_wire_error() {
        let core = Core::new();
        open_real_fixture(&core, "A");

        let frames = get_frame_analysis_command(
            &core,
            &Request {
                id: 221,
                method: "get_frame_analysis".to_string(),
                params: serde_json::json!({"frame_index": 999_999}),
            },
        );
        assert_eq!(
            frames.len(),
            1,
            "error path is Control-only, got {frames:?}"
        );
        let response: Response = serde_json::from_slice(&frames[0].1).unwrap();
        assert!(!response.ok);
        assert_eq!(response.error.unwrap().code, WireErrorCode::FrameNotFound);
    }

    #[test]
    fn get_frame_analysis_stream_not_open_returns_not_found() {
        let core = Core::new();
        let frames = get_frame_analysis_command(
            &core,
            &Request {
                id: 222,
                method: "get_frame_analysis".to_string(),
                params: serde_json::json!({"frame_index": 0}),
            },
        );
        assert_eq!(
            frames.len(),
            1,
            "error path is Control-only, got {frames:?}"
        );
        let response: Response = serde_json::from_slice(&frames[0].1).unwrap();
        assert!(!response.ok);
        assert_eq!(response.error.unwrap().code, WireErrorCode::NotFound);
    }
}
