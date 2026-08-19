//! `get_residual_analysis` command backing -- per-block residual coefficient magnitude
//! statistics for one frame of stream A. Feeds
//! `frontend/components/Player/views/ResidualsView.tsx`. AV1/IVF only.
//!
//! Real coefficient data now exists thanks to `CodingUnit.residual`
//! (`bitvue_av1_codec::tile::coding_unit`, `SymbolDecoder::read_residual_block`) -- see that
//! module's doc for what "real" means here: aggregate per-CU magnitude (nonzero count, sum of
//! absolute levels, max level), read with context-independent representative CDFs rather than
//! the real spec's neighbor-context-adaptive ones, and no dequantization/inverse-transform/pixel
//! reconstruction. This was previously a hard stub (`bitvue-cli` printed "requires a full AV1
//! tile-group decoder, not yet implemented"; the GUI rendered a QP-derived approximation and
//! never called this command for real) -- this is the first real implementation, not a wiring
//! task on top of pre-existing data like several sibling commands this session.
//!
//! "Energy" here is `sum_abs_level` (sum of absolute coefficient levels) per block -- a real,
//! monotonic-with-actual-residual-magnitude quantity, not a spatial-domain energy metric (that
//! would require the inverse transform this crate doesn't implement).

use bitvue_av1_codec::obu::{ObuIterator, ObuType};
use bitvue_av1_codec::overlay_extraction::{parse_all_coding_units, ParsedFrame};
use serde_json::{json, Value};

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

pub fn get_residual_analysis(data: &[u8], frame_index: usize) -> Result<Value, String> {
    let (_hdr, frames) = bitvue_av1_codec::ivf::parse_ivf_frames(data)
        .map_err(|e| format!("IVF parse error: {e}"))?;
    if frame_index >= frames.len() {
        return Err(format!(
            "frame_index {frame_index} out of range (stream has {} frames)",
            frames.len()
        ));
    }

    let obu_data: Vec<u8> = match find_sequence_header_bytes(&frames) {
        Some(seq_bytes) => [seq_bytes.as_slice(), frames[frame_index].data.as_slice()].concat(),
        None => frames[frame_index].data.clone(),
    };
    let parsed = ParsedFrame::parse(&obu_data).map_err(|e| e.to_string())?;
    let coding_units = parse_all_coding_units(&parsed).map_err(|e| e.to_string())?;

    let mut block_residuals = Vec::with_capacity(coding_units.len());
    let mut energies = Vec::with_capacity(coding_units.len());
    let mut zero_count = 0u64;
    let mut non_zero_count = 0u64;
    let mut max_coeff = 0u16;

    for cu in coding_units.iter() {
        let (energy, cu_max_coeff, non_zeros) = match &cu.residual {
            Some(r) => (r.sum_abs_level as f64, r.max_level, r.nonzero_count),
            None => (0.0, 0u16, 0u32),
        };
        if non_zeros == 0 {
            zero_count += 1;
        } else {
            non_zero_count += 1;
        }
        max_coeff = max_coeff.max(cu_max_coeff);
        energies.push(energy);
        block_residuals.push(json!({
            "x": cu.x,
            "y": cu.y,
            "width": cu.width,
            "height": cu.height,
            "energy": energy,
            "max_coeff": cu_max_coeff,
            "non_zeros": non_zeros,
        }));
    }

    let total_energy: f64 = energies.iter().sum();
    let count = energies.len().max(1) as f64;
    let mean = total_energy / count;
    let variance = if energies.is_empty() {
        0.0
    } else {
        energies.iter().map(|e| (e - mean).powi(2)).sum::<f64>() / count
    };
    let min_energy = energies.iter().cloned().fold(f64::INFINITY, f64::min);
    let min_energy = if min_energy.is_finite() {
        min_energy
    } else {
        0.0
    };
    let max_energy = energies.iter().cloned().fold(0.0f64, f64::max);

    Ok(json!({
        "frame_index": frame_index,
        "width": parsed.dimensions.width,
        "height": parsed.dimensions.height,
        "coefficient_stats": {
            "min": min_energy,
            "max": max_energy.max(max_coeff as f64),
            "mean": mean,
            "variance": variance,
            "energy": total_energy,
            "zero_count": zero_count,
            "non_zero_count": non_zero_count,
        },
        "block_residuals": block_residuals,
    }))
}

#[derive(serde::Deserialize)]
struct GetResidualAnalysisParams {
    frame_index: usize,
}

/// Per-block residual coefficient magnitude statistics for one frame of stream A -- see this
/// module's doc. Control-only, same reasoning as `frame_analysis::get_frame_analysis_command`.
pub fn get_residual_analysis_command(
    core: &bitvue_engine::Core,
    request: &bitvue_protocol::Request,
) -> bitvue_protocol::Response {
    use bitvue_protocol::{Response, WireError, WireErrorCode};

    let params: GetResidualAnalysisParams = match serde_json::from_value(request.params.clone()) {
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

    match get_residual_analysis(data, params.frame_index) {
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
    fn get_residual_analysis_end_to_end_returns_real_blocks() {
        let result = get_residual_analysis(AV1_IVF_FIXTURE, 0).unwrap();
        assert_eq!(result["frame_index"], 0);
        assert_eq!(result["width"], 320);
        assert_eq!(result["height"], 240);
        let blocks = result["block_residuals"].as_array().unwrap();
        assert!(!blocks.is_empty(), "expected real per-CU blocks");
        assert!(result["coefficient_stats"]["energy"].as_f64().unwrap() >= 0.0);
    }

    #[test]
    fn get_residual_analysis_non_first_frame_parses() {
        let result = get_residual_analysis(AV1_IVF_FIXTURE, 100).unwrap();
        assert_eq!(result["frame_index"], 100);
    }

    #[test]
    fn get_residual_analysis_all_250_frames_parse_without_error() {
        let (_hdr, frames) = bitvue_av1_codec::ivf::parse_ivf_frames(AV1_IVF_FIXTURE).unwrap();
        for idx in 0..frames.len() {
            let result = get_residual_analysis(AV1_IVF_FIXTURE, idx);
            assert!(result.is_ok(), "frame {idx} failed: {:?}", result.err());
        }
    }

    #[test]
    fn get_residual_analysis_out_of_range_frame_index_is_a_real_error() {
        assert!(get_residual_analysis(AV1_IVF_FIXTURE, 999_999).is_err());
    }
}
