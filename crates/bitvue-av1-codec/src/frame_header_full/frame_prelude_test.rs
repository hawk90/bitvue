//! `read_frame_prelude` pinned over seeded sweeps of random sequence-header settings and random
//! bits (result, every prelude field, bits consumed), plus the decoder-model fields.

use super::super::tests::base_seq;
use super::*;
use crate::sequence::{DecoderModelInfo, OperatingPoint, TimingInfo};

fn noise(seed: u64, len: usize) -> Vec<u8> {
    let mut x = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1;
    (0..len)
        .map(|_| {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            (x >> 24) as u8
        })
        .collect()
}

fn fnv(h: &mut u64, bytes: &[u8]) {
    for b in bytes {
        *h ^= *b as u64;
        *h = h.wrapping_mul(0x100_0000_01b3);
    }
}

fn seq_from(r: u64) -> SequenceHeader {
    let mut s = base_seq();
    s.reduced_still_picture_header = r & 7 == 0;
    s.frame_id_numbers_present = r >> 3 & 1 == 1;
    s.additional_frame_id_length_minus_1 = [None, Some(0), Some(3)][(r >> 4) as usize % 3];
    s.delta_frame_id_length_minus_2 = [None, Some(1), Some(4)][(r >> 6) as usize % 3];
    s.enable_order_hint = r >> 8 & 1 == 1;
    s.order_hint_bits_minus_1 = [None, Some(0), Some(6)][(r >> 9) as usize % 3];
    s.seq_force_screen_content_tools = [0, 1, 2][(r >> 11) as usize % 3];
    s.seq_force_integer_mv = [0, 1, 2][(r >> 13) as usize % 3];
    s
}

struct Run {
    out: Result<FramePrelude>,
    position: u64,
}

fn run(seq: &SequenceHeader, data: &[u8]) -> Run {
    let mut r = BitReader::new(data);
    let out = read_frame_prelude(&mut r, seq, Layer::default());
    Run {
        out,
        position: r.position(),
    }
}

fn describe(p: &FramePrelude) -> String {
    format!(
        "{:?}{}{}{}{}{}{}{}{}{}{}{}",
        p.frame_type,
        p.frame_is_intra,
        p.show_frame,
        p.showable_frame,
        p.error_resilient_mode,
        p.disable_cdf_update,
        p.allow_screen_content_tools,
        p.force_integer_mv,
        p.frame_size_override_flag,
        p.order_hint,
        p.primary_ref_frame,
        p.refresh_frame_flags
    )
}

fn sweep(seed: u64, n: usize) -> u64 {
    let mut h = 0xcbf2_9ce4_8422_2325u64;
    let rolls = noise(seed, n * 8);
    for i in 0..n {
        let r = u64::from_le_bytes(rolls[i * 8..i * 8 + 8].try_into().unwrap());
        let seq = seq_from(r);
        let data = noise(seed ^ ((i as u64 + 1) * 0x51ED), 32);
        let run = run(&seq, &data);
        match &run.out {
            Ok(p) => fnv(&mut h, describe(p).as_bytes()),
            Err(e) => fnv(&mut h, format!("E{e:?}").as_bytes()),
        }
        fnv(&mut h, &run.position.to_le_bytes());
    }
    h
}

#[test]
fn seeded_sweeps_are_pinned() {
    assert_eq!(sweep(0x5eed, 3000), 295573562034634281);
    assert_eq!(sweep(0xfeed, 3000), 10947962063558678831);
}

#[test]
fn the_sweep_reaches_every_frame_type() {
    let (mut key, mut inter, mut intra_only, mut switch, mut reduced) = (0, 0, 0, 0, 0);
    let rolls = noise(0x5eed, 3000 * 8);
    for i in 0..3000 {
        let r = u64::from_le_bytes(rolls[i * 8..i * 8 + 8].try_into().unwrap());
        let seq = seq_from(r);
        let data = noise(0x5eed ^ ((i as u64 + 1) * 0x51ED), 32);
        let run = run(&seq, &data);
        if let Ok(p) = &run.out {
            match p.frame_type {
                FrameType::Key => key += 1,
                FrameType::Inter => inter += 1,
                FrameType::IntraOnly => intra_only += 1,
                FrameType::Switch => switch += 1,
                _ => {}
            }
            reduced += usize::from(seq.reduced_still_picture_header);
        }
    }
    assert!(key > 0 && inter > 0 && intra_only > 0 && switch > 0 && reduced > 0);
}

/// A sequence with a decoder model: 4-bit `frame_presentation_time`, 3-bit `buffer_removal_time`.
fn model_seq(equal_picture_interval: bool, ops: &[(u16, bool)]) -> SequenceHeader {
    let mut seq = seq_from(0x5);
    seq.timing_info = Some(TimingInfo {
        num_units_in_display_tick: 1,
        time_scale: 30,
        equal_picture_interval,
        num_ticks_per_picture: None,
    });
    seq.decoder_model_info = Some(DecoderModelInfo {
        buffer_delay_length_minus_1: 0,
        num_units_in_decoding_tick: 1,
        buffer_removal_time_length_minus_1: 2,
        frame_presentation_time_length_minus_1: 3,
    });
    seq.operating_points = ops
        .iter()
        .map(|&(idc, decoder_model_present)| OperatingPoint {
            idc,
            seq_level_idx: 0,
            seq_tier: false,
            decoder_model_present,
            initial_display_delay: None,
        })
        .collect();
    seq
}

/// Bits consumed by `f` over all-ones data (a set `buffer_removal_time_present_flag`).
fn consumed(f: impl FnOnce(&mut BitReader) -> Result<()>, data: &[u8]) -> u64 {
    let mut r = BitReader::new(data);
    f(&mut r).unwrap();
    r.position()
}

/// Spec 5.9.2: `temporal_point_info()` follows `show_frame` (and `frame_to_show_map_idx`) only when
/// the sequence has a decoder model and the pictures are not at an equal interval.
#[test]
fn temporal_point_info_is_read_only_with_a_decoder_model_and_unequal_picture_intervals() {
    let ones = [0xffu8; 8];
    let tpi = |seq: &SequenceHeader| consumed(|r| skip_temporal_point_info(r, seq), &ones);
    assert_eq!(tpi(&model_seq(false, &[(0, true)])), 4);
    assert_eq!(tpi(&model_seq(true, &[(0, true)])), 0);
    assert_eq!(tpi(&seq_from(0x5)), 0, "no decoder model");

    // decoder_model_info without timing_info cannot happen in a real stream (the flag is read only
    // inside timing_info_present_flag); treated as unequal intervals
    let mut no_timing = model_seq(false, &[(0, true)]);
    no_timing.timing_info = None;
    assert_eq!(tpi(&no_timing), 4);
}

/// Spec 5.9.2: `buffer_removal_time_present_flag`, then one `buffer_removal_time` per operating
/// point that has a decoder model and contains the frame's temporal and spatial layer.
#[test]
fn buffer_removal_times_follow_the_operating_points_that_contain_the_frame() {
    let removal = |seq: &SequenceHeader, layer: Layer, data: &[u8]| {
        consumed(|r| skip_buffer_removal_times(r, seq, layer), data)
    };
    let layer = |temporal_id, spatial_id| Layer {
        temporal_id,
        spatial_id,
    };
    let ones = [0xffu8; 8];
    let flag_clear = [0x7fu8, 0xff, 0xff, 0xff];

    // no decoder model: nothing is read, not even the flag
    assert_eq!(removal(&seq_from(0x5), layer(0, 0), &ones), 0);
    // the flag alone when it is clear
    let model = model_seq(false, &[(0, true)]);
    assert_eq!(removal(&model, layer(0, 0), &flag_clear), 1);
    // idc 0 means "all layers": one 3-bit time
    assert_eq!(removal(&model, layer(3, 1), &ones), 1 + 3);
    // an operating point without a decoder model has no time
    let skipped = model_seq(false, &[(0, true), (0, false), (0, true)]);
    assert_eq!(removal(&skipped, layer(0, 0), &ones), 1 + 3 + 3);

    // idc 0x101 = temporal layer 0 and spatial layer 0; 0x202 = layer 1 of each
    let layered = model_seq(false, &[(0x101, true), (0x202, true)]);
    assert_eq!(removal(&layered, layer(0, 0), &ones), 1 + 3);
    assert_eq!(removal(&layered, layer(1, 1), &ones), 1 + 3);
    assert_eq!(removal(&layered, layer(1, 0), &ones), 1, "in neither point");
    let both = model_seq(false, &[(0x303, true), (0x101, true)]);
    assert_eq!(removal(&both, layer(0, 0), &ones), 1 + 3 + 3);
    assert_eq!(removal(&both, layer(1, 1), &ones), 1 + 3);
}

#[test]
fn a_reduced_still_picture_header_reads_only_the_flags_it_has() {
    let mut seq = seq_from(0);
    seq.reduced_still_picture_header = true;
    seq.frame_id_numbers_present = false;
    seq.enable_order_hint = false;
    seq.order_hint_bits_minus_1 = None;
    seq.seq_force_screen_content_tools = 0;
    seq.seq_force_integer_mv = 0;
    // Only disable_cdf_update remains to read (a Key frame is shown with error resilience and
    // refresh 0xFF implied).
    let run = run(&seq, &[0xFF]);
    let p = run.out.unwrap();
    assert_eq!(p.frame_type, FrameType::Key);
    assert!(p.show_frame && !p.showable_frame && !p.error_resilient_mode);
    assert!(p.disable_cdf_update);
    assert_eq!(
        (p.refresh_frame_flags, p.primary_ref_frame),
        (0xFF, PRIMARY_REF_NONE)
    );
    assert_eq!(run.position, 1);
}

#[test]
fn reading_zero_bits_is_free_so_the_order_hint_guard_is_only_an_early_out() {
    let mut r = BitReader::new(&[]);
    assert_eq!(r.read_bits(0).unwrap(), 0);
    assert_eq!(r.position(), 0);
}
