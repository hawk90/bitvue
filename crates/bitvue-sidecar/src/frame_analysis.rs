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
    extract_mv_grid_from_parsed, extract_partition_grid_from_parsed,
    extract_prediction_mode_grid_from_parsed, extract_qp_grid_from_parsed,
    extract_transform_grid_from_parsed, ParsedFrame, PredictionModeGrid, TransformGrid,
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

    Ok(json!({
        "frame_index": frame_index,
        "width": parsed.dimensions.width,
        "height": parsed.dimensions.height,
        "qp_grid": qp_grid_to_json(&qp_grid),
        "mv_grid": mv_grid_to_json(&mv_grid),
        "partition_grid": partition_grid_to_json(&partition_grid),
        "prediction_mode_grid": prediction_mode_grid_to_json(&prediction_mode_grid),
        "transform_grid": transform_grid_to_json(&transform_grid),
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
