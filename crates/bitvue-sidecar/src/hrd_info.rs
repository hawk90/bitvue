//! `get_hrd_info` command backing -- the AV1 decoder model of stream A (sequence header
//! `timing_info`, `decoder_model_info`, `operating_parameters_info`) plus the buffer limits the
//! operating points' levels imply, feeding the HRD Buffer view (#538). AV1/IVF only.
//!
//! AV1 does not signal a bit rate or buffer size: the decoder model's `BitRate` and `BufferSize`
//! follow from the level, tier and profile of the operating point (spec Annex E.2 and A.3), which
//! `bitvue_av1_codec::sequence::buffer_limits` evaluates. The stream signals only the delays
//! (`decoder_buffer_delay`, `encoder_buffer_delay`), `low_delay_mode_flag` and the tick lengths.
//!
//! `model` is the decoder model run over the whole stream (spec Annex E, `bitvue_av1_codec::
//! decoder_model`): per decodable frame group the arrival and removal times, the smoothing
//! buffer's occupancy curve and the conformance violations. It is `null` (with the reason in
//! `model_unavailable`) when the stream signals no timing the model can run on.

use bitvue_av1_codec::decoder_model::{self, DecoderModel, Mode};
use bitvue_av1_codec::sequence::{
    buffer_limits, level_name, OperatingPoint, SequenceHeader, TimingInfo,
};
use serde_json::{json, Value};

/// Frame rate from `timing_info` when the pictures are at an equal interval; `None` otherwise (a
/// variable display rate has no single frame rate).
fn frame_rate(timing: &TimingInfo) -> Option<f64> {
    let ticks = timing.num_ticks_per_picture?;
    let per_picture = f64::from(timing.num_units_in_display_tick) * f64::from(ticks);
    (per_picture > 0.0).then(|| f64::from(timing.time_scale) / per_picture)
}

fn operating_point_json(seq: &SequenceHeader, op: &OperatingPoint) -> Value {
    let limits = buffer_limits(seq.profile, op.seq_level_idx, op.seq_tier);
    json!({
        "idc": op.idc,
        "seq_level_idx": op.seq_level_idx,
        "level": level_name(op.seq_level_idx),
        "tier": if op.seq_tier { "high" } else { "main" },
        "max_bitrate_bps": limits.map(|l| l.bit_rate_bps),
        "buffer_size_bits": limits.map(|l| l.buffer_size_bits),
        "decoder_model": op.operating_parameters.map(|p| json!({
            "decoder_buffer_delay": p.decoder_buffer_delay,
            "encoder_buffer_delay": p.encoder_buffer_delay,
            "low_delay_mode": p.low_delay_mode,
        })),
    })
}

fn model_json(model: &DecoderModel) -> Value {
    json!({
        "mode": match model.mode {
            Mode::DecodingSchedule => "decoding_schedule",
            Mode::ResourceAvailability => "resource_availability",
        },
        "bit_rate_bps": model.bit_rate,
        "buffer_size_bits": model.buffer_size,
        "decoder_buffer_delay": model.decoder_buffer_delay,
        "encoder_buffer_delay": model.encoder_buffer_delay,
        "low_delay_mode": model.low_delay_mode,
        "dec_ct_seconds": model.dec_ct,
        "peak_bits": model.peak_bits,
        "frames": model.frames.iter().map(|f| json!({
            "dfg": f.dfg,
            "packet": f.packet,
            "frame_type": f.frame_type.as_str(),
            "show_frame": f.show_frame,
            "coded_bits": f.coded_bits,
            "buffer_removal_time": f.buffer_removal_time,
            "frame_presentation_time": f.frame_presentation_time,
            "first_bit_arrival": f.first_bit_arrival,
            "last_bit_arrival": f.last_bit_arrival,
            "scheduled_removal": f.scheduled_removal,
            "removal": f.removal,
        })).collect::<Vec<_>>(),
        // [seconds, bits in the smoothing buffer], breakpoints of the piecewise linear curve; a
        // removal is two points at one time
        "curve": model.curve.iter().map(|p| json!([p.time, p.bits])).collect::<Vec<_>>(),
        "violations": model.violations.iter().map(|v| json!({
            "kind": v.kind.code(),
            "dfg": v.dfg,
            "packet": v.packet,
        })).collect::<Vec<_>>(),
    })
}

/// The wire form of a sequence header's decoder model, with the model run over the stream's
/// packets (`Err` when it could not run: the reason goes to `model_unavailable`). `present` is true when the HRD view
/// has something to show: the decoder model ran.
pub fn hrd_info_json(
    seq: &SequenceHeader,
    model: &Result<DecoderModel, decoder_model::Unavailable>,
) -> Value {
    json!({
        "present": model.is_ok(),
        "model": model.as_ref().ok().map(model_json),
        "model_unavailable": model.as_ref().err().map(decoder_model::Unavailable::reason),
        "profile": seq.profile.name(),
        "frame_rate": seq.timing_info.as_ref().and_then(frame_rate),
        "timing": seq.timing_info.as_ref().map(|t| json!({
            "num_units_in_display_tick": t.num_units_in_display_tick,
            "time_scale": t.time_scale,
            "equal_picture_interval": t.equal_picture_interval,
            "num_ticks_per_picture": t.num_ticks_per_picture,
        })),
        "decoder_model": seq.decoder_model_info.as_ref().map(|m| json!({
            "buffer_delay_length": u32::from(m.buffer_delay_length_minus_1) + 1,
            "num_units_in_decoding_tick": m.num_units_in_decoding_tick,
            "buffer_removal_time_length": u32::from(m.buffer_removal_time_length_minus_1) + 1,
            "frame_presentation_time_length": u32::from(m.frame_presentation_time_length_minus_1) + 1,
        })),
        "operating_points": seq
            .operating_points
            .iter()
            .map(|op| operating_point_json(seq, op))
            .collect::<Vec<_>>(),
    })
}

pub fn get_hrd_info(data: &[u8]) -> Result<Value, String> {
    let (_hdr, frames) = bitvue_av1_codec::ivf::parse_ivf_frames(data)
        .map_err(|e| format!("IVF parse error: {e}"))?;
    let seq = bitvue_av1_codec::overlay_extraction::find_sequence_header(&frames)
        .map(|(_, seq)| seq)
        .ok_or_else(|| "no sequence header found in the first few frames".to_string())?;
    let model = decoder_model::analyze(frames.iter().map(|f| f.data.as_slice()));
    Ok(hrd_info_json(&seq, &model))
}

/// The decoder model of stream A -- see this module's doc. Control-only, same reasoning as
/// `av1_features::get_av1_features_command`.
pub fn get_hrd_info_command(
    core: &bitvue_engine::Core,
    request: &bitvue_protocol::Request,
) -> bitvue_protocol::Response {
    use bitvue_protocol::{Response, WireError, WireErrorCode};

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

    match get_hrd_info(data) {
        Ok(value) => Response::success(request.id, value),
        Err(message) => Response::failure(
            request.id,
            WireError {
                code: WireErrorCode::InvalidData,
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

    const WITHOUT_MODEL: &[u8] = include_bytes!("../../../test_data/av1_test.ivf");
    const WITH_MODEL: &[u8] = include_bytes!("../../../test_data/av1_aomenc_decoder_model.ivf");
    const UNDERFLOW: &[u8] =
        include_bytes!("../../../test_data/av1_aomenc_decoder_model_underflow.ivf");
    const CONSTANT_TIMING: &[u8] =
        include_bytes!("../../../test_data/av1_aomenc_timing_constant.ivf");

    /// A clip without a decoder model reports `present: false` and still the level's limits.
    #[test]
    fn a_stream_without_a_decoder_model_is_not_present() {
        let info = get_hrd_info(WITHOUT_MODEL).unwrap();
        assert_eq!(info["present"], false);
        assert!(info["decoder_model"].is_null());
        assert!(info["operating_points"][0]["decoder_model"].is_null());
    }

    /// aomenc `--timing-info=model`: 30 fps, a decoder model with one operating point.
    #[test]
    fn a_stream_with_a_decoder_model_reports_its_delays_and_level_limits() {
        let info = get_hrd_info(WITH_MODEL).unwrap();
        assert_eq!(info["present"], true);
        assert_eq!(info["profile"], "Main");
        assert_eq!(info["timing"]["time_scale"], 30);
        assert_eq!(info["timing"]["num_units_in_display_tick"], 1);
        assert_eq!(info["timing"]["equal_picture_interval"], false);
        assert!(
            info["frame_rate"].is_null(),
            "no equal picture interval, no single rate"
        );

        let model = &info["decoder_model"];
        assert_eq!(model["buffer_delay_length"], 16);
        assert_eq!(model["buffer_removal_time_length"], 10);

        let op = &info["operating_points"][0];
        assert_eq!(op["idc"], 0);
        assert!(op["decoder_model"]["decoder_buffer_delay"].is_u64());
        assert!(op["decoder_model"]["encoder_buffer_delay"].is_u64());
        assert!(op["decoder_model"]["low_delay_mode"].is_boolean());
        // seq_level_idx 0 = level 2.0: MainMbps 1.5
        assert_eq!(op["level"], "2.0");
        assert_eq!(op["max_bitrate_bps"], 1_500_000);
        assert_eq!(op["buffer_size_bits"], 1_500_000);
    }

    /// The model rides along with the decoder model: per decodable frame group its times, the
    /// occupancy curve and no violation for aomenc's own clip (#587).
    #[test]
    fn a_stream_with_a_decoder_model_reports_the_model_run_over_its_frames() {
        let info = get_hrd_info(WITH_MODEL).unwrap();
        assert_eq!(info["model_unavailable"], Value::Null);
        let model = &info["model"];
        assert_eq!(model["mode"], "decoding_schedule");
        assert_eq!(model["bit_rate_bps"], 1_500_000);
        assert_eq!(model["buffer_size_bits"], 1_500_000);
        assert_eq!(model["decoder_buffer_delay"], 45_000);
        assert!((model["dec_ct_seconds"].as_f64().unwrap() - 1.0 / 30.0).abs() < 1e-12);
        assert_eq!(model["violations"], json!([]));

        let frames = model["frames"].as_array().unwrap();
        assert_eq!(frames.len(), 3);
        assert_eq!(frames[0]["frame_type"], "I");
        assert_eq!(frames[0]["buffer_removal_time"], 1);
        assert_eq!(frames[2]["frame_presentation_time"], 2);
        assert_eq!(frames[0]["scheduled_removal"], 0.5);
        for (i, frame) in frames.iter().enumerate() {
            assert_eq!(frame["dfg"], i);
            assert_eq!(frame["packet"], i);
        }

        // [seconds, bits]; the curve peaks with the three frames in the buffer at the first removal
        let curve = model["curve"].as_array().unwrap();
        assert_eq!(curve[0], json!([0.0, 0.0]));
        let peak = model["peak_bits"].as_f64().unwrap();
        assert!(curve.iter().any(|p| p[1].as_f64() == Some(peak)));
        assert!(curve
            .windows(2)
            .all(|w| w[0][0].as_f64() <= w[1][0].as_f64()));
    }

    #[test]
    fn a_decoder_model_that_is_broken_lists_its_violations() {
        let info = get_hrd_info(UNDERFLOW).unwrap();
        assert_eq!(
            info["model"]["violations"],
            json!([{"kind": "smoothing_buffer_underflow", "dfg": 0, "packet": 0}])
        );
    }

    /// Without timing there is no model, and the reason goes to the panel.
    #[test]
    fn a_stream_without_timing_has_no_model_and_says_why() {
        let info = get_hrd_info(WITHOUT_MODEL).unwrap();
        assert_eq!(info["model"], Value::Null);
        assert!(info["model_unavailable"]
            .as_str()
            .unwrap()
            .contains("no decoder model"));
    }

    /// `timing_info` with an equal picture interval and no decoder_model_info is the resource
    /// availability mode: `present` follows the model, not only the signaled decoder model.
    #[test]
    fn constant_timing_runs_the_resource_availability_model() {
        let info = get_hrd_info(CONSTANT_TIMING).unwrap();
        assert_eq!(info["present"], true);
        assert!(info["decoder_model"].is_null(), "nothing signaled");
        assert_eq!(info["model"]["mode"], "resource_availability");
        assert_eq!(info["model"]["frames"].as_array().unwrap().len(), 12);
        assert_eq!(info["model"]["decoder_buffer_delay"], 70_000);
    }

    #[test]
    fn frame_rate_follows_equal_picture_interval_timing() {
        let at = |ticks_per_picture: Option<u32>| TimingInfo {
            num_units_in_display_tick: 1001,
            time_scale: 60_000,
            equal_picture_interval: ticks_per_picture.is_some(),
            num_ticks_per_picture: ticks_per_picture,
        };
        let fps = frame_rate(&at(Some(2))).unwrap();
        assert!((fps - 29.97).abs() < 0.01, "{fps}");
        assert_eq!(frame_rate(&at(None)), None);
        let zero_tick = TimingInfo {
            num_units_in_display_tick: 0,
            ..at(Some(1))
        };
        assert_eq!(frame_rate(&zero_tick), None, "no division by a zero tick");
    }

    #[test]
    fn command_end_to_end_and_stream_not_open() {
        let core = Core::new();
        let closed = crate::dispatch(
            &core,
            &Request {
                id: 1,
                method: "get_hrd_info".to_string(),
                params: serde_json::json!({}),
            },
        );
        assert!(!closed.ok);
        assert_eq!(closed.error.unwrap().code, WireErrorCode::NotFound);

        open_real_fixture(&core, "A");
        let open = crate::dispatch(
            &core,
            &Request {
                id: 2,
                method: "get_hrd_info".to_string(),
                params: serde_json::json!({}),
            },
        );
        assert!(open.ok, "{open:?}");
        assert_eq!(open.result.unwrap()["present"], false);
    }
}
