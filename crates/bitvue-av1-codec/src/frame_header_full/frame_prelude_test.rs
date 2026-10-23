//! `read_frame_prelude` pinned over seeded sweeps of random sequence-header settings and random
//! bits (result, every prelude field, bits consumed), plus the decoder-model error.

use super::super::tests::base_seq;
use super::*;
use crate::sequence::DecoderModelInfo;

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
    let out = read_frame_prelude(&mut r, seq);
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

#[test]
fn a_stream_with_decoder_model_info_is_rejected_after_the_primary_ref_frame() {
    let mut seq = seq_from(0x5);
    seq.decoder_model_info = Some(DecoderModelInfo {
        buffer_delay_length_minus_1: 0,
        num_units_in_decoding_tick: 1,
        buffer_removal_time_length_minus_1: 0,
        frame_presentation_time_length_minus_1: 0,
    });
    let run = run(&seq, &noise(3, 32));
    let message = format!("{:?}", run.out.err().expect("must be rejected"));
    assert!(message.contains("decoder_model_info_present"), "{message}");
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
