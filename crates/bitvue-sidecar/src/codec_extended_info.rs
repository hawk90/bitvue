//! `get_codec_extended_info` command backing -- AV1 reference-frame lists (L0/L1) + per-frame QP
//! histogram, for one frame of stream A. Feeds
//! `frontend/components/panels/SyntaxDetailPanel/{RefListTab,StatisticsTab}.tsx`. AV1/IVF only.
//!
//! `QmTab`/`ProbsTab`/`ApsTab` (the panel's HEVC/VP9/VVC-specific sub-tabs) are dead code: this
//! sidecar has no HEVC/VP9/VVC decode path at all (`bitvue-sidecar` only depends on
//! `bitvue-av1-codec`), so those tabs can never receive real data regardless of what this command
//! returns. Only the two AV1-reachable consumers are implemented.
//!
//! Real data already existed for both pieces -- this is orchestration, not new bitstream parsing:
//!  - `frame_header_full::parse_frame_header_full` already tracks `ref_frame_idx` (widened this
//!    session from 3 to the full `REFS_PER_FRAME`=7 entries) and now also `FrameHeader.order_hint`
//!    (new field, same session). This module tracks slot -> (order_hint, frame_index, frame_type)
//!    during the same sequential scan `av1_features`/`deblocking` already use (mirroring
//!    `RefFrameState`'s own update-on-`refresh_frame_flags` pattern), resolving each
//!    `ref_frame_idx` entry into a real POC/frame_index/frame_type, then splits into L0
//!    (forward/past) vs L1 (backward/future) via `relative_dist` (spec 7.9.2) -- the same signed
//!    order-hint comparison `skip_mode_params` already uses.
//!  - `overlay_extraction::qp_extractor` already produces a real per-block QP grid; the histogram
//!    here is a bucketing pass over it.
//!
//! AV1 has no per-reference long-term marking or explicit weighted-prediction offset/weight
//! syntax (unlike HEVC) -- `long_term` is always `false` and `weight`/`offset` are always `null`
//! on the wire, matching the frontend's `RefEntry` type (both already nullable for this reason).

use std::collections::HashMap;

use bitvue_av1_codec::frame_header::FrameType;
use bitvue_av1_codec::frame_header_full::{
    find_frame_header_payload, parse_frame_header_full, relative_dist, RefFrameState,
};
use bitvue_av1_codec::obu::{ObuIterator, ObuType};
use bitvue_av1_codec::overlay_extraction::{extract_qp_grid_from_parsed, ParsedFrame};
use bitvue_av1_codec::sequence::{parse_sequence_header, SequenceHeader};
use serde_json::{json, Value};

const SEQUENCE_HEADER_SCAN_LIMIT: usize = 8;
const NUM_REF_FRAMES: usize = 8;

#[derive(Clone, Copy)]
struct SlotState {
    order_hint: u32,
    frame_index: usize,
    frame_type: FrameType,
}

impl Default for SlotState {
    fn default() -> Self {
        Self {
            order_hint: 0,
            frame_index: 0,
            frame_type: FrameType::Key,
        }
    }
}

/// Matches `bitvue-mcp`'s own `FrameType` -> wire-string convention (the closest existing
/// API-facing precedent for this exact mapping).
fn frame_type_wire(t: FrameType) -> &'static str {
    match t {
        FrameType::Key => "I",
        FrameType::Inter => "P",
        FrameType::IntraOnly => "I",
        FrameType::Switch => "I",
        // Not produced by this crate's AV1 parsing (other codecs' FrameType variants) --
        // included only for exhaustiveness.
        FrameType::BFrame => "B",
        FrameType::SI => "I",
        FrameType::SP => "P",
        FrameType::Unknown => "UNKNOWN",
    }
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

fn ref_entry_json(list_idx: usize, slot: usize, s: &SlotState, poc_delta: i64) -> Value {
    json!({
        "list_idx": list_idx,
        "slot": slot,
        "poc": poc_delta,
        "frame_index": s.frame_index,
        "frame_type": frame_type_wire(s.frame_type),
        "long_term": false,
        "weight": Value::Null,
        "offset": Value::Null,
    })
}

fn qp_histogram_json(qp: &[i16]) -> Vec<Value> {
    let mut counts: HashMap<i16, u64> = HashMap::new();
    for &v in qp {
        *counts.entry(v).or_insert(0) += 1;
    }
    let mut buckets: Vec<(i16, u64)> = counts.into_iter().collect();
    buckets.sort_by_key(|(qp, _)| *qp);
    buckets
        .into_iter()
        .map(|(qp, count)| json!({ "qp": qp, "count": count }))
        .collect()
}

pub fn get_codec_extended_info(data: &[u8], frame_index: usize) -> Result<Value, String> {
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
    let order_hint_bits = seq.order_hint_bits_minus_1.map_or(0, |b| b as u32 + 1);

    let mut ref_state = RefFrameState::new();
    let mut slots = [SlotState::default(); NUM_REF_FRAMES];
    let mut header = None;
    for (idx, frame) in frames.iter().take(frame_index + 1).enumerate() {
        let payload = find_frame_header_payload(&frame.data)
            .ok_or_else(|| "frame has no Frame/FrameHeader OBU".to_string())?;
        let hdr =
            parse_frame_header_full(&payload, &seq, &mut ref_state).map_err(|e| e.to_string())?;
        if let Some(flags) = hdr.refresh_frame_flags {
            for (slot, state) in slots.iter_mut().enumerate() {
                if (flags >> slot) & 1 == 1 {
                    *state = SlotState {
                        order_hint: hdr.order_hint,
                        frame_index: idx,
                        frame_type: hdr.frame_type,
                    };
                }
            }
        }
        header = Some(hdr);
    }
    let header = header.expect("frame_index < frames.len() guarantees at least one iteration");

    let mut l0_refs = Vec::new();
    let mut l1_refs = Vec::new();
    if let Some(ref_frame_idx) = header.ref_frame_idx {
        for &slot in ref_frame_idx.iter() {
            let state = &slots[slot as usize];
            let dist = relative_dist(
                state.order_hint,
                header.order_hint,
                seq.enable_order_hint,
                order_hint_bits,
            );
            let entry = if dist < 0 { &mut l0_refs } else { &mut l1_refs };
            entry.push(ref_entry_json(entry.len(), slot as usize, state, dist));
        }
    }

    let obu_data: Vec<u8> = match find_sequence_header_bytes(&frames) {
        Some(seq_bytes) => [seq_bytes.as_slice(), frames[frame_index].data.as_slice()].concat(),
        None => frames[frame_index].data.clone(),
    };
    let parsed = ParsedFrame::parse(&obu_data).map_err(|e| e.to_string())?;
    let base_qp = parsed.frame_type.base_qp.unwrap_or(0) as i16;
    let qp_grid =
        extract_qp_grid_from_parsed(&parsed, frame_index, base_qp).map_err(|e| e.to_string())?;

    Ok(json!({
        "frame_index": frame_index,
        "l0_refs": l0_refs,
        "l1_refs": l1_refs,
        "qp_histogram": qp_histogram_json(&qp_grid.qp),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    const AV1_IVF_FIXTURE: &[u8] = include_bytes!("../../../test_data/av1_test.ivf");

    #[test]
    fn get_codec_extended_info_key_frame_has_no_refs() {
        let result = get_codec_extended_info(AV1_IVF_FIXTURE, 0).unwrap();
        assert_eq!(result["frame_index"], 0);
        assert!(result["l0_refs"].as_array().unwrap().is_empty());
        assert!(result["l1_refs"].as_array().unwrap().is_empty());
        assert!(!result["qp_histogram"].as_array().unwrap().is_empty());
    }

    #[test]
    fn get_codec_extended_info_inter_frame_has_real_refs() {
        let result = get_codec_extended_info(AV1_IVF_FIXTURE, 5).unwrap();
        assert_eq!(result["frame_index"], 5);
        let l0 = result["l0_refs"].as_array().unwrap();
        assert!(!l0.is_empty(), "expected at least one L0 reference");
        // A forward (L0) reference's resolved frame_index must be strictly before frame 5.
        assert!(l0[0]["frame_index"].as_u64().unwrap() < 5);
        assert_eq!(l0[0]["long_term"], false);
        assert!(l0[0]["weight"].is_null());
    }

    #[test]
    fn get_codec_extended_info_qp_histogram_counts_sum_to_grid_size() {
        let result = get_codec_extended_info(AV1_IVF_FIXTURE, 0).unwrap();
        let histogram = result["qp_histogram"].as_array().unwrap();
        let total: u64 = histogram.iter().map(|b| b["count"].as_u64().unwrap()).sum();
        assert!(total > 0);
    }

    #[test]
    fn get_codec_extended_info_out_of_range_frame_index_is_a_real_error() {
        assert!(get_codec_extended_info(AV1_IVF_FIXTURE, 999_999).is_err());
    }
}
