//! The AV1 decoder model (spec Annex E): the smoothing buffer the stream is fed through at the
//! level's `BitRate`, when each frame is removed from it and decoded, and where the stream breaks
//! the conformance requirements of E.6.
//!
//! * [`stream`] reads the frames in decoding order with the bits of each decodable frame group.
//! * [`smoothing`] is E.4: arrival and removal times and the buffer's occupancy.
//! * [`decode`] is E.5: frame buffers, decode times and presentation, and (in resource
//!   availability mode) the removal times themselves.
//!
//! [`analyze`] ties them together for operating point 0, the one a decoder plays by default.

mod decode;
mod smoothing;
mod stream;

pub use smoothing::OccupancyPoint;

use crate::sequence::{buffer_limits, max_decode_rate, SequenceHeader};
use bitvue_engine::FrameType;
use decode::{decode_process, DecodeConfig, Removal};
use smoothing::{
    arrival_and_removal, occupancy_curve, scheduled_removal_timing, DfgTimes, SmoothingParams,
};

/// `BUFFER_POOL_MAX_SIZE - 1 + 1`: with no `initial_display_delay_minus_1` the decoder waits until
/// the pool is full (spec 5.5.2: it defaults to `BUFFER_POOL_MAX_SIZE - 1`).
const DEFAULT_INITIAL_DISPLAY_DELAY: usize = 10;
/// The decoder model parameters of a stream with `timing_info` that is at an equal picture
/// interval but has no `decoder_model_info` (spec E.3.1).
const RESOURCE_MODE_ENCODER_BUFFER_DELAY: u32 = 20_000;
const RESOURCE_MODE_DECODER_BUFFER_DELAY: u32 = 70_000;
/// Floating point slack when comparing times, seconds.
const EPSILON: f64 = 1e-9;

/// One frame of a stream in decoding order, as the decoder model needs it (the same record for a
/// decoded frame and for a `show_existing_frame` header).
#[derive(Debug, Clone)]
pub(crate) struct FrameRecord {
    /// Index of the packet the frame header is in
    pub(crate) packet: usize,
    pub(crate) show_existing_frame: bool,
    pub(crate) show_frame: bool,
    pub(crate) frame_type: FrameType,
    pub(crate) frame_to_show_map_idx: u8,
    pub(crate) refresh_frame_flags: u8,
    /// `time_to_decode_frame()`'s `lumaSamples` (spec E.4.6)
    pub(crate) luma_samples: u64,
    /// `buffer_removal_time[ 0 ]`
    pub(crate) buffer_removal_time: Option<u64>,
    pub(crate) frame_presentation_time: Option<u64>,
    /// `CodedBits` of the frame's decodable frame group; 0 for a `show_existing_frame`
    pub(crate) coded_bits: u64,
}

/// Which of the two operating modes the model runs in (spec E.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// The stream signals `buffer_removal_time` and `frame_presentation_time` for every frame.
    DecodingSchedule,
    /// Removal times follow from when a frame buffer is free; `timing_info` has an equal picture
    /// interval and there is no `decoder_model_info`.
    ResourceAvailability,
}

/// Why no model is computed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Unavailable {
    /// The stream could not be read
    Unreadable(String),
    NoFrames,
    /// Neither mode applies: the stream has no `timing_info`, or it has `decoder_model_info` but
    /// operating point 0 has no decoder model, or the picture interval is not equal and there is
    /// no decoder model
    NoTimingModel,
    /// The level, tier and profile of operating point 0 define no `BitRate` / `MaxDecodeRate`
    NoLevelLimits,
    /// `time_scale` or `num_units_in_decoding_tick` is 0
    BadTicks,
    /// A frame has no `buffer_removal_time` for operating point 0 in decoding schedule mode
    MissingBufferRemovalTime {
        dfg: usize,
    },
}

impl Unavailable {
    pub fn reason(&self) -> String {
        match self {
            Self::Unreadable(why) => format!("the stream cannot be read: {why}"),
            Self::NoFrames => "the stream has no frames".to_string(),
            Self::NoTimingModel => "no decoder model: the first operating point has no \
                decoder model, and the pictures are not at an equal interval"
                .to_string(),
            Self::NoLevelLimits => {
                "the level of the first operating point defines no bit rate".to_string()
            }
            Self::BadTicks => "timing_info or decoder_model_info has a zero tick".to_string(),
            Self::MissingBufferRemovalTime { dfg } => {
                format!("frame {dfg} has no buffer_removal_time for the first operating point")
            }
        }
    }
}

/// What the conformance requirements of E.6 and the decode process of E.5 can be broken by.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViolationKind {
    /// A DFG is not complete in the smoothing buffer at its scheduled removal time, and the
    /// operating point is not in low-delay mode (E.6.4)
    SmoothingBufferUnderflow,
    /// More than `BufferSize` bits are in the smoothing buffer (E.6.3)
    SmoothingBufferOverflow,
    /// `decoder_buffer_delay` is more than `ceil( TimeDelta )` at a key frame (E.6.2)
    DecoderBufferDelayConsistency,
    /// `decoder_buffer_delay` is 0 or longer than `BufferSize ÷ BitRate` (E.6.8)
    DecoderBufferDelayLevel,
    /// `DECODE_BUFFER_AVAILABLE_LATE`
    DecodeBufferAvailableLate,
    /// `DECODE_FRAME_BUF_UNAVAILABLE`
    DecodeFrameBufUnavailable,
    /// `DECODE_EXISTING_FRAME_BUF_EMPTY`
    DecodeExistingFrameBufEmpty,
    /// `DISPLAY_FRAME_LATE`
    DisplayFrameLate,
}

impl ViolationKind {
    /// The name on the wire.
    pub fn code(self) -> &'static str {
        match self {
            Self::SmoothingBufferUnderflow => "smoothing_buffer_underflow",
            Self::SmoothingBufferOverflow => "smoothing_buffer_overflow",
            Self::DecoderBufferDelayConsistency => "decoder_buffer_delay_consistency",
            Self::DecoderBufferDelayLevel => "decoder_buffer_delay_level",
            Self::DecodeBufferAvailableLate => "decode_buffer_available_late",
            Self::DecodeFrameBufUnavailable => "decode_frame_buf_unavailable",
            Self::DecodeExistingFrameBufEmpty => "decode_existing_frame_buf_empty",
            Self::DisplayFrameLate => "display_frame_late",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Violation {
    pub kind: ViolationKind,
    /// The decodable frame group (decoded frame, in decoding order) it is found at; `None` for a
    /// stream-wide requirement
    pub dfg: Option<usize>,
    /// The packet that frame is in
    pub packet: Option<usize>,
}

/// One decodable frame group: a frame that is decoded, with the timing the model gives it.
#[derive(Debug, Clone)]
pub struct ModelFrame {
    pub dfg: usize,
    pub packet: usize,
    pub frame_type: FrameType,
    pub show_frame: bool,
    pub coded_bits: u64,
    pub buffer_removal_time: Option<u64>,
    pub frame_presentation_time: Option<u64>,
    /// seconds
    pub first_bit_arrival: f64,
    pub last_bit_arrival: f64,
    pub scheduled_removal: f64,
    pub removal: f64,
}

#[derive(Debug, Clone)]
pub struct DecoderModel {
    pub mode: Mode,
    /// `BitRate` and `BufferSize`, bits per second and bits
    pub bit_rate: u64,
    pub buffer_size: u64,
    pub encoder_buffer_delay: u32,
    pub decoder_buffer_delay: u32,
    pub low_delay_mode: bool,
    /// `DecCT`, seconds (decoding schedule mode)
    pub dec_ct: Option<f64>,
    pub frames: Vec<ModelFrame>,
    pub curve: Vec<OccupancyPoint>,
    pub peak_bits: f64,
    pub violations: Vec<Violation>,
}

/// Runs the decoder model over a stream's packets (IVF frames) for operating point 0.
pub fn analyze<'a>(
    packets: impl IntoIterator<Item = &'a [u8]>,
) -> Result<DecoderModel, Unavailable> {
    let (seq, records) = stream::read_frames(packets).map_err(Unavailable::Unreadable)?;
    model_of(&seq, &records)
}

/// The mode and parameters operating point 0 gives the model.
struct Setup {
    mode: Mode,
    smoothing: SmoothingParams,
    bit_rate: u64,
    buffer_size: u64,
    max_decode_rate: f64,
    initial_display_delay: usize,
}

fn setup_of(seq: &SequenceHeader) -> Result<Setup, Unavailable> {
    let (Some(op), Some(timing)) = (seq.operating_points.first(), seq.timing_info.as_ref()) else {
        return Err(Unavailable::NoTimingModel);
    };
    if timing.time_scale == 0 || timing.num_units_in_display_tick == 0 {
        return Err(Unavailable::BadTicks);
    }
    let (mode, encoder_buffer_delay, decoder_buffer_delay, low_delay, dec_ct) =
        match (&seq.decoder_model_info, op.operating_parameters) {
            (Some(info), Some(params)) => {
                if info.num_units_in_decoding_tick == 0 {
                    return Err(Unavailable::BadTicks);
                }
                (
                    Mode::DecodingSchedule,
                    params.encoder_buffer_delay,
                    params.decoder_buffer_delay,
                    params.low_delay_mode,
                    Some(f64::from(info.num_units_in_decoding_tick) / f64::from(timing.time_scale)),
                )
            }
            (None, _) if timing.equal_picture_interval => (
                Mode::ResourceAvailability,
                RESOURCE_MODE_ENCODER_BUFFER_DELAY,
                RESOURCE_MODE_DECODER_BUFFER_DELAY,
                false,
                None,
            ),
            _ => return Err(Unavailable::NoTimingModel),
        };
    let limits = buffer_limits(seq.profile, op.seq_level_idx, op.seq_tier)
        .ok_or(Unavailable::NoLevelLimits)?;
    let max_decode_rate = max_decode_rate(op.seq_level_idx).ok_or(Unavailable::NoLevelLimits)?;
    Ok(Setup {
        mode,
        smoothing: SmoothingParams {
            bit_rate: limits.bit_rate_bps as f64,
            encoder_buffer_delay,
            decoder_buffer_delay,
            low_delay,
            dec_ct,
        },
        bit_rate: limits.bit_rate_bps,
        buffer_size: limits.buffer_size_bits,
        max_decode_rate: max_decode_rate as f64,
        initial_display_delay: op
            .initial_display_delay
            .map_or(DEFAULT_INITIAL_DISPLAY_DELAY, usize::from),
    })
}

/// `PresentationTime[ j ] - InitialPresentationDelay` for every shown frame (spec E.4.7), seconds.
/// `None` when a frame has no `frame_presentation_time` and the interval is not equal.
///
/// With an equal picture interval frame `j` is `j * num_ticks_per_picture * DispCT` late. Otherwise
/// it is `frame_presentation_time * DispCT` after the latest shown key frame before it (the first
/// frame is at 0); the value is the remainder of a counter modulo `1 << length`, lifted to the
/// smallest value that keeps the presentation times from running backwards.
fn presentation_offsets(seq: &SequenceHeader, records: &[FrameRecord]) -> Option<Vec<f64>> {
    let timing = seq.timing_info.as_ref()?;
    let disp_ct = f64::from(timing.num_units_in_display_tick) / f64::from(timing.time_scale);
    let shown = records
        .iter()
        .filter(|r| r.show_frame || r.show_existing_frame);
    if timing.equal_picture_interval {
        let step = f64::from(timing.num_ticks_per_picture?) * disp_ct;
        return Some(shown.enumerate().map(|(j, _)| j as f64 * step).collect());
    }
    let modulus = 1u64
        << (seq
            .decoder_model_info
            .as_ref()?
            .frame_presentation_time_length_minus_1
            + 1);
    let mut offsets: Vec<f64> = Vec::new();
    let mut last_key = 0.0;
    for (j, record) in shown.enumerate() {
        let offset = if j == 0 {
            0.0
        } else {
            let previous: f64 = offsets[j - 1];
            let min_ticks = ((previous - last_key) / disp_ct - 1e-9).ceil().max(0.0) as u64;
            let mut ticks = record.frame_presentation_time?;
            if ticks < min_ticks {
                ticks += (min_ticks - ticks).div_ceil(modulus) * modulus;
            }
            last_key + ticks as f64 * disp_ct
        };
        offsets.push(offset);
        if !record.show_existing_frame && record.frame_type == FrameType::Key {
            last_key = offset;
        }
    }
    Some(offsets)
}

fn model_of(seq: &SequenceHeader, records: &[FrameRecord]) -> Result<DecoderModel, Unavailable> {
    let setup = setup_of(seq)?;
    let dfgs: Vec<&FrameRecord> = records.iter().filter(|r| !r.show_existing_frame).collect();
    if dfgs.is_empty() {
        return Err(Unavailable::NoFrames);
    }
    let coded_bits: Vec<u64> = dfgs.iter().map(|r| r.coded_bits).collect();
    let presentation = presentation_offsets(seq, records);
    let config = DecodeConfig {
        max_decode_rate: setup.max_decode_rate,
        initial_display_delay: setup.initial_display_delay,
        presentation: presentation.as_deref(),
    };

    let params = &setup.smoothing;
    let (times, decode_violations) = match setup.mode {
        Mode::DecodingSchedule => {
            let mut removal_times = Vec::with_capacity(dfgs.len());
            for (dfg, record) in dfgs.iter().enumerate() {
                removal_times.push(
                    record
                        .buffer_removal_time
                        .ok_or(Unavailable::MissingBufferRemovalTime { dfg })?,
                );
            }
            let info = seq
                .decoder_model_info
                .as_ref()
                .ok_or(Unavailable::NoTimingModel)?;
            let is_rap: Vec<bool> = dfgs
                .iter()
                .map(|r| r.show_frame && r.frame_type == FrameType::Key)
                .collect();
            let scheduled = scheduled_removal_timing(
                &removal_times,
                &is_rap,
                1u64 << (info.buffer_removal_time_length_minus_1 + 1),
                params.dec_ct.ok_or(Unavailable::BadTicks)?,
                params.decoder_buffer_delay,
            );
            let times = arrival_and_removal(params, &coded_bits, &scheduled);
            let removal: Vec<f64> = times.iter().map(|t| t.removal).collect();
            let outcome = decode_process(records, &config, &Removal::Schedule(&removal));
            (times, outcome.violations)
        }
        Mode::ResourceAvailability => {
            let first_removal = f64::from(params.decoder_buffer_delay) / 90_000.0;
            let outcome = decode_process(records, &config, &Removal::Resource { first_removal });
            let times = arrival_and_removal(params, &coded_bits, &outcome.removal);
            (times, outcome.violations)
        }
    };

    let curve = occupancy_curve(&coded_bits, &times, params.bit_rate);
    let mut violations = smoothing_violations(&setup, &dfgs, &times, &curve);
    // a failure at a decoded frame is at its DFG; one at a `show_existing_frame` has none
    let dfg_of = |index: usize| {
        (!records[index].show_existing_frame).then(|| {
            records[..index]
                .iter()
                .filter(|r| !r.show_existing_frame)
                .count()
        })
    };
    violations.extend(
        decode_violations
            .into_iter()
            .map(|(kind, index)| Violation {
                kind,
                dfg: dfg_of(index),
                packet: Some(records[index].packet),
            }),
    );
    violations.sort_by_key(|v| (v.dfg, v.packet));

    let peak_bits = curve.iter().map(|p| p.bits).fold(0.0, f64::max);
    let frames = dfgs
        .iter()
        .zip(&times)
        .enumerate()
        .map(|(dfg, (record, t))| ModelFrame {
            dfg,
            packet: record.packet,
            frame_type: record.frame_type,
            show_frame: record.show_frame,
            coded_bits: record.coded_bits,
            buffer_removal_time: record.buffer_removal_time,
            frame_presentation_time: record.frame_presentation_time,
            first_bit_arrival: t.first_bit_arrival,
            last_bit_arrival: t.last_bit_arrival,
            scheduled_removal: t.scheduled_removal,
            removal: t.removal,
        })
        .collect();
    Ok(DecoderModel {
        mode: setup.mode,
        bit_rate: setup.bit_rate,
        buffer_size: setup.buffer_size,
        encoder_buffer_delay: params.encoder_buffer_delay,
        decoder_buffer_delay: params.decoder_buffer_delay,
        low_delay_mode: params.low_delay,
        dec_ct: params.dec_ct,
        frames,
        curve,
        peak_bits,
        violations,
    })
}

/// E.6: underflow, overflow and the two `decoder_buffer_delay` requirements.
fn smoothing_violations(
    setup: &Setup,
    dfgs: &[&FrameRecord],
    times: &[DfgTimes],
    curve: &[OccupancyPoint],
) -> Vec<Violation> {
    let params = &setup.smoothing;
    let at = |dfg: usize, kind| Violation {
        kind,
        dfg: Some(dfg),
        packet: Some(dfgs[dfg].packet),
    };
    let mut violations = Vec::new();
    for (dfg, t) in times.iter().enumerate() {
        // ScheduledRemoval[ i ] < LastBitArrival[ i ]
        if !params.low_delay && t.scheduled_removal < t.last_bit_arrival - EPSILON {
            violations.push(at(dfg, ViolationKind::SmoothingBufferUnderflow));
        }
    }
    let mut overflowed = std::collections::BTreeSet::new();
    for point in curve {
        if point.bits > setup.buffer_size as f64 + 1e-6 {
            overflowed.insert(point.dfg);
        }
    }
    violations.extend(
        overflowed
            .into_iter()
            .map(|dfg| at(dfg, ViolationKind::SmoothingBufferOverflow)),
    );
    if setup.mode == Mode::DecodingSchedule {
        let delay = f64::from(params.decoder_buffer_delay);
        for dfg in 1..times.len() {
            if dfgs[dfg].frame_type == FrameType::Key {
                // TimeDelta[ i ] = ( ScheduledRemoval[ i ] - LastBitArrival[ i - 1 ] ) * 90 000
                let time_delta =
                    (times[dfg].scheduled_removal - times[dfg - 1].last_bit_arrival) * 90_000.0;
                if delay > (time_delta - 1e-6).ceil() {
                    violations.push(at(dfg, ViolationKind::DecoderBufferDelayConsistency));
                }
            }
        }
        let longest = 90_000.0 * setup.buffer_size as f64 / setup.bit_rate as f64;
        if params.decoder_buffer_delay == 0 || delay > longest {
            violations.push(Violation {
                kind: ViolationKind::DecoderBufferDelayLevel,
                dfg: None,
                packet: None,
            });
        }
    }
    violations
}

#[cfg(test)]
#[path = "model_test.rs"]
mod model_test;
