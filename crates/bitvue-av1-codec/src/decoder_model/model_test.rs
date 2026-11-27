//! The whole model on the committed clips (spec numbers worked out by hand) and on hand-made frame
//! lists for the violations a clip would have to be rebuilt to show.

use super::*;

const WITH_MODEL: &[u8] = include_bytes!("../../../../test_data/av1_aomenc_decoder_model.ivf");
const UNDERFLOW: &[u8] =
    include_bytes!("../../../../test_data/av1_aomenc_decoder_model_underflow.ivf");
const CONSTANT_TIMING: &[u8] =
    include_bytes!("../../../../test_data/av1_aomenc_timing_constant.ivf");
const WITHOUT_TIMING: &[u8] = include_bytes!("../../../../test_data/av1_test.ivf");
const SHOW_EXISTING: &[u8] = include_bytes!("../../../../test_data/av1_aomenc_testsrc2.ivf");

fn packets_of(clip: &[u8]) -> Vec<Vec<u8>> {
    let (_hdr, frames) = crate::ivf::parse_ivf_frames(clip).unwrap();
    frames.iter().map(|f| f.data.to_vec()).collect()
}

fn run(clip: &[u8]) -> Result<DecoderModel, Unavailable> {
    let packets = packets_of(clip);
    analyze(packets.iter().map(Vec::as_slice))
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

/// aomenc `--timing-info=model`: 30 fps, DecCT 1/30 s, decoder and encoder buffer delay 45 000
/// (0.5 s), level 2.0 (1.5 Mbit/s, 1.5 Mbit buffer). Frame removal times are `0.5 s` plus
/// `buffer_removal_time[ i ]` ticks from the first.
#[test]
fn the_decoder_model_clip_runs_in_decoding_schedule_mode() {
    let model = run(WITH_MODEL).unwrap();
    assert_eq!(model.mode, Mode::DecodingSchedule);
    assert_eq!((model.bit_rate, model.buffer_size), (1_500_000, 1_500_000));
    assert_eq!(
        (model.decoder_buffer_delay, model.encoder_buffer_delay),
        (45_000, 45_000)
    );
    assert!(!model.low_delay_mode);
    assert!(close(model.dec_ct.unwrap(), 1.0 / 30.0));
    assert_eq!(model.frames.len(), 3);

    // ScheduledRemoval[ 0 ] = 45 000 / 90 000; the clip's counters are 1, 3, 5 and the others
    // count from the first removal
    let scheduled: Vec<f64> = model.frames.iter().map(|f| f.scheduled_removal).collect();
    for (got, want) in scheduled
        .iter()
        .zip([0.5, 0.5 + 3.0 / 30.0, 0.5 + 5.0 / 30.0])
    {
        assert!(close(*got, want), "{scheduled:?}");
    }
    let counters: Vec<_> = model.frames.iter().map(|f| f.buffer_removal_time).collect();
    assert_eq!(counters, vec![Some(1), Some(3), Some(5)]);

    // the frames are far smaller than the buffer and arrive back to back from time 0
    let packets = packets_of(WITH_MODEL);
    let mut elapsed = 0.0;
    for (frame, packet) in model.frames.iter().zip(&packets) {
        assert_eq!(
            frame.coded_bits,
            packet.len() as u64 * 8,
            "one frame per packet"
        );
        assert!(close(frame.first_bit_arrival, elapsed));
        elapsed += frame.coded_bits as f64 / 1_500_000.0;
        assert!(close(frame.last_bit_arrival, elapsed));
        assert!(close(frame.removal, frame.scheduled_removal));
    }
    // nothing is removed before 0.5 s, so the peak is every bit of the clip
    assert!(close(
        model.peak_bits,
        packets.iter().map(|p| p.len() as f64 * 8.0).sum()
    ));
    assert!(model.violations.is_empty(), "{:?}", model.violations);
}

/// The same clip with `decoder_buffer_delay` and `encoder_buffer_delay` cut to 600 (6.7 ms): the
/// key frame is removed at 6.7 ms but its 14 240 bits take 9.5 ms to arrive.
#[test]
fn a_frame_not_complete_at_its_removal_time_is_an_underflow() {
    let model = run(UNDERFLOW).unwrap();
    assert!(close(model.frames[0].scheduled_removal, 600.0 / 90_000.0));
    assert!(model.frames[0].scheduled_removal < model.frames[0].last_bit_arrival);
    // the later frames wait for LatestArrivalTime and are on time
    for frame in &model.frames[1..] {
        assert!(frame.scheduled_removal > frame.last_bit_arrival);
    }
    assert_eq!(
        model.violations,
        vec![Violation {
            kind: ViolationKind::SmoothingBufferUnderflow,
            dfg: Some(0),
            packet: Some(0)
        }]
    );
    // the occupancy at the early removal is what has arrived minus the whole frame
    let removal = model
        .curve
        .iter()
        .rfind(|p| close(p.time, 600.0 / 90_000.0))
        .unwrap();
    assert!(removal.bits < 0.0, "{removal:?}");
}

/// `--timing-info=constant`: timing_info with an equal picture interval and no decoder model is
/// resource availability mode, with the default delays 70 000 and 20 000 and `low_delay_mode` 0.
#[test]
fn constant_timing_without_a_decoder_model_runs_in_resource_availability_mode() {
    let model = run(CONSTANT_TIMING).unwrap();
    assert_eq!(model.mode, Mode::ResourceAvailability);
    assert_eq!(
        (model.decoder_buffer_delay, model.encoder_buffer_delay),
        (70_000, 20_000)
    );
    assert!(!model.low_delay_mode);
    assert_eq!(model.dec_ct, None);
    assert_eq!(model.frames.len(), 12);
    // the first DFG is removed at decoder_buffer_delay ÷ 90 000, each next as soon as the decoder is
    // done: 128x128 luma samples at the level 2.0 MaxDecodeRate of 5 529 600 per second
    let decode_time = 128.0 * 128.0 / 5_529_600.0;
    assert!(close(model.frames[0].removal, 70_000.0 / 90_000.0));
    for pair in model.frames.windows(2) {
        assert!(
            close(pair[1].removal - pair[0].removal, decode_time),
            "{} -> {}",
            pair[0].removal,
            pair[1].removal
        );
    }
    for frame in &model.frames {
        assert!(close(frame.scheduled_removal, frame.removal));
        assert_eq!(frame.buffer_removal_time, None);
    }
    assert!(model.violations.is_empty(), "{:?}", model.violations);
}

#[test]
fn a_stream_with_no_timing_has_no_model() {
    assert_eq!(run(WITHOUT_TIMING).unwrap_err(), Unavailable::NoTimingModel);
    assert!(Unavailable::NoTimingModel
        .reason()
        .contains("no decoder model"));
}

/// Hidden frames and `show_existing_frame` headers: a header that repeats a frame is no DFG of its
/// own, and its bytes count towards the next decoded frame; every byte up to the last frame is in
/// some DFG.
#[test]
fn show_existing_frames_add_no_dfg_and_their_bits_go_to_the_next_one() {
    let packets = packets_of(SHOW_EXISTING);
    let (seq, records) = stream::read_frames(packets.iter().map(Vec::as_slice)).unwrap();
    assert_eq!(seq.operating_points.len(), 1);
    let existing = records.iter().filter(|r| r.show_existing_frame).count();
    assert_eq!((records.len(), existing), (16, 3));
    assert!(records
        .iter()
        .filter(|r| r.show_existing_frame)
        .all(|r| r.coded_bits == 0));
    let decoded: u64 = records.iter().map(|r| r.coded_bits).sum();
    let total: u64 = packets.iter().map(|p| p.len() as u64 * 8).sum();
    // the stream ends with a frame: nothing is left over
    assert_eq!(decoded, total);
    // a packet that has a hidden frame and a shown one gives two DFGs from one packet
    let per_packet = |p: usize| records.iter().filter(|r| r.packet == p).count();
    assert!((0..packets.len()).any(|p| per_packet(p) > 1));
}

fn operating_point_of(clip: &[u8]) -> (SequenceHeader, Vec<FrameRecord>) {
    let packets = packets_of(clip);
    stream::read_frames(packets.iter().map(Vec::as_slice)).unwrap()
}

#[test]
fn the_mode_follows_what_the_sequence_header_signals() {
    let (seq, _) = operating_point_of(WITH_MODEL);
    assert!(setup_of(&seq).is_ok());

    let mut no_level = seq.clone();
    no_level.operating_points[0].seq_level_idx = 31;
    assert_eq!(setup_of(&no_level).err(), Some(Unavailable::NoLevelLimits));

    let mut no_timing = seq.clone();
    no_timing.timing_info = None;
    assert_eq!(setup_of(&no_timing).err(), Some(Unavailable::NoTimingModel));

    // decoder_model_info present but this operating point has no decoder model
    let mut op_without_model = seq.clone();
    op_without_model.operating_points[0].operating_parameters = None;
    assert_eq!(
        setup_of(&op_without_model).err(),
        Some(Unavailable::NoTimingModel)
    );

    // no decoder model and no equal interval: nothing to run
    let mut unequal = seq.clone();
    unequal.decoder_model_info = None;
    unequal.timing_info.as_mut().unwrap().equal_picture_interval = false;
    assert_eq!(setup_of(&unequal).err(), Some(Unavailable::NoTimingModel));

    let mut zero_tick = seq.clone();
    zero_tick.timing_info.as_mut().unwrap().time_scale = 0;
    assert_eq!(setup_of(&zero_tick).err(), Some(Unavailable::BadTicks));
    let mut zero_decoding_tick = seq;
    zero_decoding_tick
        .decoder_model_info
        .as_mut()
        .unwrap()
        .num_units_in_decoding_tick = 0;
    assert_eq!(
        setup_of(&zero_decoding_tick).err(),
        Some(Unavailable::BadTicks)
    );
}

fn record(frame_type: FrameType, removal_time: u64, coded_bits: u64) -> FrameRecord {
    FrameRecord {
        packet: 0,
        show_existing_frame: false,
        show_frame: true,
        frame_type,
        frame_to_show_map_idx: 0,
        refresh_frame_flags: 0xFF,
        luma_samples: 128 * 128,
        buffer_removal_time: Some(removal_time),
        frame_presentation_time: Some(removal_time),
        coded_bits,
    }
}

fn kinds(model: &DecoderModel) -> Vec<ViolationKind> {
    model.violations.iter().map(|v| v.kind).collect()
}

/// A delay of 10 s lets 15 Mbit arrive before the first removal: more than the 1.5 Mbit buffer
/// holds, and far over the `BufferSize ÷ BitRate` = 1 s the level allows `decoder_buffer_delay`.
#[test]
fn a_buffer_filled_ahead_of_removal_overflows_and_breaks_the_delay_limit() {
    let (mut seq, _) = operating_point_of(WITH_MODEL);
    let params = seq.operating_points[0]
        .operating_parameters
        .as_mut()
        .unwrap();
    params.decoder_buffer_delay = 900_000;
    params.encoder_buffer_delay = 900_000;
    let mut records = vec![record(FrameType::Key, 0, 400_000)];
    records.extend((1..10).map(|i| record(FrameType::Inter, i, 400_000)));
    let model = model_of(&seq, &records).unwrap();
    assert!(model.peak_bits > model.buffer_size as f64);
    let kinds = kinds(&model);
    assert!(
        kinds.contains(&ViolationKind::SmoothingBufferOverflow),
        "{kinds:?}"
    );
    assert!(
        kinds.contains(&ViolationKind::DecoderBufferDelayLevel),
        "{kinds:?}"
    );
    // arrival is bounded by the rate: 4 Mbit take 2.67 s, all before the 10 s removal
    assert!(!kinds.contains(&ViolationKind::SmoothingBufferUnderflow));
}

/// A key frame after the first must not be removed so soon after the previous frame's last bit
/// that `decoder_buffer_delay` would exceed the time delta between them (E.6.2).
#[test]
fn decoder_buffer_delay_may_not_exceed_the_gap_before_a_key_frame() {
    let (seq, _) = operating_point_of(WITH_MODEL);
    // frame 1 (600 kbit) is complete at 0.41 s; the key frame is removed at 0.7 s, 0.29 s later:
    // 26 160 ticks of 90 kHz, less than the delay of 45 000
    let records = vec![
        record(FrameType::Key, 0, 14_000),
        record(FrameType::Inter, 3, 600_000),
        record(FrameType::Key, 6, 1000),
    ];
    let model = model_of(&seq, &records).unwrap();
    let delta = (model.frames[2].scheduled_removal - model.frames[1].last_bit_arrival) * 90_000.0;
    assert!(delta < 45_000.0, "{delta}");
    assert_eq!(
        kinds(&model),
        vec![ViolationKind::DecoderBufferDelayConsistency]
    );
    assert_eq!(model.violations[0].dfg, Some(2));

    // the same gap before a frame that is not a key frame is fine
    let records = vec![
        record(FrameType::Key, 0, 14_000),
        record(FrameType::Inter, 3, 600_000),
        record(FrameType::Inter, 6, 1000),
    ];
    assert!(model_of(&seq, &records).unwrap().violations.is_empty());
}

/// In low-delay mode a late DFG is not an underflow: it is removed late instead.
#[test]
fn low_delay_mode_defers_instead_of_underflowing() {
    let (mut seq, _) = operating_point_of(UNDERFLOW);
    let mut records = operating_point_of(UNDERFLOW).1;
    assert_eq!(
        kinds(&model_of(&seq, &records).unwrap()),
        vec![ViolationKind::SmoothingBufferUnderflow]
    );
    seq.operating_points[0]
        .operating_parameters
        .as_mut()
        .unwrap()
        .low_delay_mode = true;
    let model = model_of(&seq, &records).unwrap();
    assert!(model.violations.is_empty(), "{:?}", model.violations);
    // the first frame is removed at the first decoding tick after its last bit: 1/30 s
    assert!(close(model.frames[0].removal, 1.0 / 30.0));
    assert!(model.frames[0].removal >= model.frames[0].last_bit_arrival);

    // a frame without buffer_removal_time cannot be placed on the schedule
    records[1].buffer_removal_time = None;
    assert_eq!(
        model_of(&seq, &records).err(),
        Some(Unavailable::MissingBufferRemovalTime { dfg: 1 })
    );
}

#[test]
fn presentation_follows_the_interval_the_stream_signals() {
    let (mut seq, mut records) = operating_point_of(WITH_MODEL);
    // fpt 0, 1, 2 ticks of 1/30 s
    let offsets = presentation_offsets(&seq, &records).unwrap();
    for (got, want) in offsets.iter().zip([0.0, 1.0 / 30.0, 2.0 / 30.0]) {
        assert!(close(*got, want), "{offsets:?}");
    }

    // an equal interval of 2 ticks of 1/30 s ignores frame_presentation_time
    seq.timing_info.as_mut().unwrap().equal_picture_interval = true;
    seq.timing_info.as_mut().unwrap().num_ticks_per_picture = Some(2);
    let offsets = presentation_offsets(&seq, &records).unwrap();
    for (got, want) in offsets.iter().zip([0.0, 2.0 / 30.0, 4.0 / 30.0]) {
        assert!(close(*got, want), "{offsets:?}");
    }

    // 10-bit counter: 1020, 1023, 1 (=1025) after the key frame at 0, then a second key frame
    // counts from the first
    seq.timing_info.as_mut().unwrap().equal_picture_interval = false;
    let length = seq
        .decoder_model_info
        .as_ref()
        .unwrap()
        .frame_presentation_time_length_minus_1;
    assert_eq!(length, 9, "a 10-bit counter in this clip");
    records = vec![
        record(FrameType::Key, 0, 1),
        record(FrameType::Inter, 0, 1),
        record(FrameType::Inter, 0, 1),
        record(FrameType::Key, 0, 1),
    ];
    for (record, fpt) in records.iter_mut().zip([0, 1020, 1023, 1]) {
        record.frame_presentation_time = Some(fpt);
    }
    let offsets = presentation_offsets(&seq, &records).unwrap();
    let ticks: Vec<f64> = offsets.iter().map(|o| (o * 30.0).round()).collect();
    assert_eq!(ticks, vec![0.0, 1020.0, 1023.0, 1025.0]);

    // a shown frame with no frame_presentation_time leaves the times unknown
    records[2].frame_presentation_time = None;
    assert_eq!(presentation_offsets(&seq, &records), None);
}
