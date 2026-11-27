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

use bitvue_av1_codec::overlay_extraction::{
    extract_bit_grid_from_parsed, extract_energy_grid_from_parsed, extract_mv_grid_from_parsed,
    extract_partition_grid_from_parsed, extract_prediction_mode_grid_from_parsed,
    extract_qp_grid_from_parsed, extract_transform_grid_from_parsed, frame_provenance, BitGrid,
    EnergyGrid, ParsedFrame, PredictionModeGrid, Provenance, TransformGrid,
};
use bitvue_av1_codec::tile::PredictionMode;
use bitvue_engine::mv_overlay::{MVGrid, MotionVector};
use bitvue_engine::partition_grid::{PartitionBlock, PartitionGrid};
use bitvue_engine::qp_heatmap::QPGrid;
use serde_json::{json, Value};

pub fn get_frame_analysis(data: &[u8], frame_index: usize) -> Result<Value, String> {
    let (_hdr, frames) = bitvue_av1_codec::ivf::parse_ivf_frames(data)
        .map_err(|e| format!("IVF parse error: {e}"))?;
    if frame_index >= frames.len() {
        return Err(format!(
            "frame_index {frame_index} out of range (stream has {} frames)",
            frames.len()
        ));
    }

    // Decoded with the state of the frames before it (see `analysis_session`). Only a stream
    // without any sequence header degrades to the scaffold dimensions, as before.
    let parsed = match crate::analysis_session::analyzed_frame(data, frame_index) {
        Ok(frame) => frame,
        Err(crate::analysis_session::AnalysisError::NoSequenceHeader) => std::sync::Arc::new(
            ParsedFrame::parse(&frames[frame_index].data).map_err(|e| e.to_string())?,
        ),
        Err(e) => return Err(e.to_string()),
    };
    let base_qp = parsed.frame_type.base_qp.unwrap_or(0) as i16;

    // Nothing was decoded for this frame: the extractors would hand back invented "scaffold"
    // grids, which are not an approximation of the frame. Send none -- the caller gets the
    // frame's size and the reason, and its overlays stay empty.
    let provenance = frame_provenance(&parsed);
    if provenance == Provenance::Scaffold {
        return Ok(json!({
            "frame_index": frame_index,
            "width": parsed.dimensions.width,
            "height": parsed.dimensions.height,
            "provenance": provenance.as_str(),
        }));
    }

    let qp_grid =
        extract_qp_grid_from_parsed(&parsed, frame_index, base_qp).map_err(|e| e.to_string())?;
    let mv_grid = extract_mv_grid_from_parsed(&parsed).map_err(|e| e.to_string())?;
    let partition_grid = extract_partition_grid_from_parsed(&parsed).map_err(|e| e.to_string())?;
    let prediction_mode_grid =
        extract_prediction_mode_grid_from_parsed(&parsed).map_err(|e| e.to_string())?;
    let transform_grid = extract_transform_grid_from_parsed(&parsed).map_err(|e| e.to_string())?;
    let energy_grid = extract_energy_grid_from_parsed(&parsed).map_err(|e| e.to_string())?;
    let bit_grid = extract_bit_grid_from_parsed(&parsed).map_err(|e| e.to_string())?;

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
        "bit_grid": bit_grid_to_json(&bit_grid),
        "provenance": provenance.as_str(),
    }))
}

/// The fields every per-block grid shares on the wire. `coded` is `Some((width, height))` for
/// the grids that also report the coded frame size (MV/prediction-mode/transform); QP/energy/bit
/// grids don't, and must not gain the keys.
struct GridShape {
    grid_w: u32,
    grid_h: u32,
    block_w: u32,
    block_h: u32,
    coded: Option<(u32, u32)>,
}

/// `GridShape` of any grid struct with the shared field names (the structs come from different
/// crates and have no common trait); `coded` also reads its `coded_width`/`coded_height`.
macro_rules! grid_shape {
    ($g:expr) => {
        GridShape {
            grid_w: $g.grid_w,
            grid_h: $g.grid_h,
            block_w: $g.block_w,
            block_h: $g.block_h,
            coded: None,
        }
    };
    ($g:expr, coded) => {
        GridShape {
            coded: Some(($g.coded_width, $g.coded_height)),
            ..grid_shape!($g)
        }
    };
}

/// The shared shape fields merged with a grid's own fields (a `json!` object).
fn grid_json(shape: GridShape, own: Value) -> Value {
    let mut fields = json!({
        "grid_w": shape.grid_w,
        "grid_h": shape.grid_h,
        "block_w": shape.block_w,
        "block_h": shape.block_h,
    });
    if let Some((coded_width, coded_height)) = shape.coded {
        fields["coded_width"] = coded_width.into();
        fields["coded_height"] = coded_height.into();
    }
    if let (Value::Object(target), Value::Object(own)) = (&mut fields, own) {
        target.extend(own);
    }
    fields
}

fn qp_grid_to_json(g: &QPGrid) -> Value {
    let shape = grid_shape!(g);
    grid_json(
        shape,
        json!({ "qp": g.qp, "qp_min": g.qp_min, "qp_max": g.qp_max }),
    )
}

fn energy_grid_to_json(g: &EnergyGrid) -> Value {
    let shape = grid_shape!(g);
    grid_json(shape, json!({ "energy_bpp": g.energy_bpp }))
}

fn bit_grid_to_json(g: &BitGrid) -> Value {
    let shape = grid_shape!(g);
    grid_json(
        shape,
        json!({ "bits_per_pixel": g.bits_per_pixel, "symbols_per_bit": g.symbols_per_bit }),
    )
}

fn motion_vector_to_json(mv: &MotionVector) -> Value {
    json!({ "dx_qpel": mv.dx_qpel, "dy_qpel": mv.dy_qpel })
}

fn mv_grid_to_json(g: &MVGrid) -> Value {
    let shape = grid_shape!(g, coded);
    grid_json(
        shape,
        json!({
            "mv_l0": g.mv_l0.iter().map(motion_vector_to_json).collect::<Vec<_>>(),
            "mv_l1": g.mv_l1.iter().map(motion_vector_to_json).collect::<Vec<_>>(),
            "mode": g.mode.as_ref().map(|modes| modes.iter().map(|m| *m as u8).collect::<Vec<u8>>()),
        }),
    )
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
    let shape = grid_shape!(g, coded);
    grid_json(
        shape,
        json!({ "modes": g.modes.iter().map(|m| m.map(prediction_mode_to_wire)).collect::<Vec<_>>() }),
    )
}

fn transform_grid_to_json(g: &TransformGrid) -> Value {
    let shape = grid_shape!(g, coded);
    // TxSize's declaration order (Tx4x4=0..Tx64x64=4) already matches the frontend's
    // expected numbering exactly -- direct cast, no remap table needed (unlike PredictionMode).
    let tx_sizes = g
        .tx_sizes
        .iter()
        .map(|t| t.map(|tx| tx as u8))
        .collect::<Vec<_>>();
    grid_json(shape, json!({ "tx_sizes": tx_sizes }))
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

    let params: GetFrameAnalysisParams = match crate::command_support::parse_params(request) {
        Ok(p) => p,
        Err(response) => return single_control_frame(response),
    };

    let byte_cache = match crate::command_support::stream_byte_cache(
        core,
        bitvue_engine::StreamId::A,
        request.id,
    ) {
        Ok(cache) => cache,
        Err(response) => return single_control_frame(response),
    };

    let data = match crate::command_support::read_all(&byte_cache, request.id) {
        Ok(bytes) => bytes,
        Err(response) => return single_control_frame(response),
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
    fn get_frame_analysis_marks_the_key_frame_verified() {
        let result = get_frame_analysis(AV1_IVF_FIXTURE, 0).unwrap();
        assert_eq!(result["provenance"], "verified");
        assert!(result["mv_grid"].is_object());
    }

    /// Inter frames decode with the state of the frames before them and are verified; a damaged
    /// inter frame's grids are still sent, flagged.
    #[test]
    fn get_frame_analysis_verifies_inter_frames_and_flags_a_damaged_one() {
        let early = get_frame_analysis(AV1_IVF_FIXTURE, 2).unwrap();
        assert_eq!(early["provenance"], "verified");
        assert!(early["qp_grid"].is_object());

        let late = get_frame_analysis(AV1_IVF_FIXTURE, 65).unwrap();
        assert_eq!(late["provenance"], "verified");

        let later = get_frame_analysis(
            &crate::test_support::stream_with_a_corrupted_inter_frame(),
            2,
        )
        .unwrap();
        assert_eq!(later["provenance"], "unverified");
        assert!(later["partition_grid"].is_object());
    }

    /// A `show_existing_frame` packet displays a frame decoded earlier: it carries that frame's
    /// analysis.
    #[test]
    fn get_frame_analysis_of_a_show_existing_frame_is_the_shown_frames_analysis() {
        let ivf = crate::test_support::ivf_of_chunks(&[
            &bitvue_av1_codec::ivf::parse_ivf_frames(AV1_IVF_FIXTURE)
                .unwrap()
                .1[0]
                .data,
            crate::test_support::SHOW_EXISTING_SLOT_0,
        ]);
        let key = get_frame_analysis(&ivf, 0).unwrap();
        let shown = get_frame_analysis(&ivf, 1).unwrap();
        assert_eq!(shown["provenance"], "verified");
        assert_eq!(shown["qp_grid"], key["qp_grid"]);
    }

    /// A frame with no tile (here a `show_existing_frame` header) was never decoded: no grids
    /// at all, rather than the invented scaffold ones the extractors would build.
    #[test]
    fn get_frame_analysis_sends_no_grids_for_a_frame_that_was_not_decoded() {
        let ivf = crate::test_support::stream_with_an_undecodable_frame();
        let result = get_frame_analysis(&ivf, 1).unwrap();
        assert_eq!(result["provenance"], "scaffold");
        assert_eq!(result["width"], 320);
        for grid in [
            "qp_grid",
            "mv_grid",
            "partition_grid",
            "prediction_mode_grid",
            "transform_grid",
            "energy_grid",
            "bit_grid",
        ] {
            assert!(result.get(grid).is_none(), "{grid} must not be sent");
        }
    }

    /// End to end through the shared analysis sessions: every packet of a typical encoder's clip
    /// (hidden frames and `show_existing_frame` packets included) comes back verified, in any
    /// order, with real grids.
    #[test]
    fn every_packet_of_the_rav1e_clip_is_verified_through_the_command_functions() {
        let clip = include_bytes!("../../../test_data/av1_rav1e_testsrc2.ivf");
        for index in [7, 2, 11, 0, 1, 3, 4, 5, 6, 8, 9, 10] {
            let result = get_frame_analysis(clip, index).unwrap();
            assert_eq!(result["provenance"], "verified", "packet {index}");
            assert!(result["partition_grid"].is_object(), "packet {index}");
            for check in [
                crate::residual_analysis::get_residual_analysis(clip, index),
                crate::coding_flow::get_coding_flow_analysis(clip, index),
                crate::codec_extended_info::get_codec_extended_info(clip, index),
                crate::deblocking::get_deblocking_analysis(clip, index),
            ] {
                assert_eq!(check.unwrap()["provenance"], "verified", "packet {index}");
            }
        }
    }

    const GRID_KEYS: [&str; 7] = [
        "qp_grid",
        "mv_grid",
        "partition_grid",
        "prediction_mode_grid",
        "transform_grid",
        "energy_grid",
        "bit_grid",
    ];

    /// (name, ivf bytes, frame indices) the golden digests below were taken from.
    fn golden_inputs() -> Vec<(&'static str, &'static [u8], Vec<usize>)> {
        vec![
            ("av1_test", AV1_IVF_FIXTURE, vec![0, 2, 5, 65]),
            (
                "tiles5",
                include_bytes!("../../../test_data/av1_aomenc_tiles5.ivf"),
                vec![0, 1],
            ),
            (
                "testsrc2",
                include_bytes!("../../../test_data/av1_aomenc_testsrc2.ivf"),
                vec![0, 1],
            ),
            (
                "globalmotion",
                include_bytes!("../../../test_data/av1_aomenc_globalmotion.ivf"),
                vec![0, 1, 2],
            ),
        ]
    }

    /// FNV-1a 64 of the serialized grid text: stable across runs and platforms.
    fn fnv1a64(text: &str) -> u64 {
        text.bytes().fold(0xcbf2_9ce4_8422_2325, |h, b| {
            (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
        })
    }

    /// `name/frame/grid -> digest` lines for every golden input. With `GRID_DUMP_DIR` set, the
    /// full serialized grids are also written there (for a byte-for-byte diff across a refactor).
    fn golden_lines() -> Vec<String> {
        let dump_dir = std::env::var_os("GRID_DUMP_DIR");
        let mut lines = Vec::new();
        for (name, data, frames) in golden_inputs() {
            for frame in frames {
                let result = get_frame_analysis(data, frame).unwrap();
                for key in GRID_KEYS {
                    let text = result[key].to_string();
                    if let Some(dir) = &dump_dir {
                        let path =
                            std::path::Path::new(dir).join(format!("{name}_{frame}_{key}.json"));
                        std::fs::write(path, &text).unwrap();
                    }
                    lines.push(format!(
                        "{name}/{frame}/{key} {:016x} {}",
                        fnv1a64(&text),
                        text.len()
                    ));
                }
            }
        }
        lines
    }

    /// The grid JSON the frontend consumes, pinned byte for byte (serialized text digest + length)
    /// so refactors of the `*_grid_to_json` mapping cannot change a key, a value or a null.
    #[test]
    fn grid_json_matches_the_golden_digests() {
        let actual = golden_lines();
        if std::env::var_os("GRID_GOLDEN_PRINT").is_some() {
            for line in &actual {
                println!("GOLDEN {line}");
            }
        }
        assert_eq!(actual.join("\n"), GOLDEN_GRID_DIGESTS.trim());
    }

    const GOLDEN_GRID_DIGESTS: &str = "\
av1_test/0/qp_grid 4c84f7a660c2845d 140
av1_test/0/mv_grid 3bea8643d6bdfb28 1915
av1_test/0/partition_grid bed3cf820d0c435b 82207
av1_test/0/prediction_mode_grid fb545073114b7802 721
av1_test/0/transform_grid eef6ee7e1e9b4312 701
av1_test/0/energy_grid d2d0190d87cc3b0a 196
av1_test/0/bit_grid 8858286e27d7be0d 840
av1_test/2/qp_grid fc6bfc43db6f5387 162
av1_test/2/mv_grid 834b11cabedb87e0 1555
av1_test/2/partition_grid 6c3d72b273b0a458 4224
av1_test/2/prediction_mode_grid 60ba79d7fed1b488 985
av1_test/2/transform_grid 518d688b481b19c7 701
av1_test/2/energy_grid 5fd438ed094c0f1b 144
av1_test/2/bit_grid f2b70227892ec605 892
av1_test/5/qp_grid bd344d9a701da45f 162
av1_test/5/mv_grid 834b11cabedb87e0 1555
av1_test/5/partition_grid 606595cc8f9e5de6 8176
av1_test/5/prediction_mode_grid 26d588b4d66e7f31 997
av1_test/5/transform_grid 4a148be7caa37491 701
av1_test/5/energy_grid 5fd438ed094c0f1b 144
av1_test/5/bit_grid 58b9154fc7e3c2fe 890
av1_test/65/qp_grid dfa1847d1660dd05 140
av1_test/65/mv_grid 038f4eadfca3c0c2 1555
av1_test/65/partition_grid c5956ba17986ceaf 12104
av1_test/65/prediction_mode_grid e0f1f9e13ef73628 996
av1_test/65/transform_grid 6235adc7bbc9502f 701
av1_test/65/energy_grid 1100701bf1713092 149
av1_test/65/bit_grid 195ff0416c1935d6 885
tiles5/0/qp_grid a5fb9725f5d1c96e 153
tiles5/0/mv_grid 3bea8643d6bdfb28 1915
tiles5/0/partition_grid 1fd41d10c37be132 14667
tiles5/0/prediction_mode_grid 73c3f1b32184339c 737
tiles5/0/transform_grid dc5ef83c031fe0ef 701
tiles5/0/energy_grid 92db6f305c3fe023 236
tiles5/0/bit_grid 19fe79e0d0b7fd1d 851
tiles5/1/qp_grid 801110163b045d8f 162
tiles5/1/mv_grid 47ab41f77c437fb0 1564
tiles5/1/partition_grid a7296e740d1286cc 16945
tiles5/1/prediction_mode_grid 4851bfded57f2618 995
tiles5/1/transform_grid cf621fe8a356ab97 701
tiles5/1/energy_grid bce3a8d094229aa3 149
tiles5/1/bit_grid 00b7b0afd3d2057a 883
testsrc2/0/qp_grid 8b4a5f0443c1b726 140
testsrc2/0/mv_grid cf84f9c0ddc83a66 1915
testsrc2/0/partition_grid 8073353a2a33551d 24940
testsrc2/0/prediction_mode_grid b30971900b13fed0 819
testsrc2/0/transform_grid bb6e94c5297ec48a 701
testsrc2/0/energy_grid aaff38166157be33 166
testsrc2/0/bit_grid 0f025f47769db746 865
testsrc2/1/qp_grid e882057706995091 140
testsrc2/1/mv_grid 8b808d8e4c099d26 1548
testsrc2/1/partition_grid 1a4eb7113ba787c3 18250
testsrc2/1/prediction_mode_grid e12063efa542ce67 993
testsrc2/1/transform_grid 9318a8f440854197 701
testsrc2/1/energy_grid 2c3591cf2c621b7b 147
testsrc2/1/bit_grid adbeccc81e349ef5 876
globalmotion/0/qp_grid b4b9e89c5a639433 170
globalmotion/0/mv_grid 65270ff315cb1061 2815
globalmotion/0/partition_grid 4ff47453974746ab 37040
globalmotion/0/prediction_mode_grid a973688a60bff05b 906
globalmotion/0/transform_grid 8ce84ac38cca22bd 893
globalmotion/0/energy_grid 56f44fb6e8863061 208
globalmotion/0/bit_grid add4d302e750b5ad 1260
globalmotion/1/qp_grid 6f0ab899c54c9707 202
globalmotion/1/mv_grid 0ff9d2719157564b 2461
globalmotion/1/partition_grid 0f410d7a1472f118 31550
globalmotion/1/prediction_mode_grid 1b255b74a9c4e378 1277
globalmotion/1/transform_grid db9249ddb22a7836 893
globalmotion/1/energy_grid bb65f601e950ab90 189
globalmotion/1/bit_grid 56874dbb3774fecb 1265
globalmotion/2/qp_grid 6f0ab899c54c9707 202
globalmotion/2/mv_grid 11210f35e9b1c177 2316
globalmotion/2/partition_grid 4894569478333576 26607
globalmotion/2/prediction_mode_grid 07ed680672cb16e0 1284
globalmotion/2/transform_grid c58cf523195a63f5 893
globalmotion/2/energy_grid 5a7bad9a6f6ac129 195
globalmotion/2/bit_grid d5915201b123675a 1265";

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
            .as_chunks::<2>()
            .0
            .iter()
            .map(|c| i16::from_le_bytes(*c) as i64)
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
    fn get_frame_analysis_bit_grid_carries_measured_bits_per_pixel() {
        let core = Core::new();
        open_real_fixture(&core, "A");
        let frames = get_frame_analysis_command(
            &core,
            &Request {
                id: 223,
                method: "get_frame_analysis".to_string(),
                params: serde_json::json!({"frame_index": 0}),
            },
        );
        let (response, _qp_bytes) = decode_frame_analysis_response(frames);
        let result = response.result.unwrap();
        let grid = &result["bit_grid"];
        let cells = (grid["grid_w"].as_u64().unwrap() * grid["grid_h"].as_u64().unwrap()) as usize;
        let bpp: Vec<f64> = grid["bits_per_pixel"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_f64().unwrap())
            .collect();
        let spb = grid["symbols_per_bit"].as_array().unwrap();
        assert_eq!(bpp.len(), cells);
        assert_eq!(spb.len(), cells);
        assert!(
            bpp.iter().any(|&v| v > 0.0),
            "a real key frame spends bits somewhere"
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
