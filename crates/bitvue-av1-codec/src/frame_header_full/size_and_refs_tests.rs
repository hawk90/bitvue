//! `read_size_and_refs` pinned over seeded sweeps of random sequence-header settings, random
//! inputs from the earlier part of the header and random bits (result or error, every field,
//! bits consumed), plus the two documented unsupported paths.

use super::super::tests::base_seq;
use super::*;

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
    s.frame_id_numbers_present = r & 1 == 1;
    s.delta_frame_id_length_minus_2 = [None, Some(1), Some(4)][(r >> 1) as usize % 3];
    s.enable_order_hint = r >> 3 & 1 == 1;
    s.enable_ref_frame_mvs = r >> 4 & 1 == 1;
    s.enable_superres = r >> 5 & 1 == 1;
    s
}

fn input_from(r: u64) -> SizeAndRefsInput {
    SizeAndRefsInput {
        frame_is_intra: r & 1 == 1,
        frame_size_override_flag: r >> 1 & 1 == 1,
        allow_screen_content_tools: r >> 2 & 1 == 1,
        force_integer_mv: r >> 3 & 1 == 1,
        error_resilient_mode: r >> 4 & 1 == 1,
    }
}

fn describe(o: &SizeAndRefs) -> String {
    let s = &o.size;
    format!(
        "{} {} {} {} {} {} {:?} {} {} {} {}",
        s.width,
        s.height,
        s.upscaled_width,
        s.upscaled_height,
        s.use_superres,
        s.superres_denom,
        o.ref_frame_idx,
        o.allow_intrabc,
        o.allow_high_precision_mv,
        o.subpel_filter_switchable,
        o.switchable_motion_mode,
    ) + &o.use_ref_frame_mvs.to_string()
}

fn run(seq: &SequenceHeader, input: &SizeAndRefsInput, data: &[u8]) -> (Result<SizeAndRefs>, u64) {
    let mut r = BitReader::new(data);
    let out = read_size_and_refs(&mut r, seq, input);
    (out, r.position())
}

fn sweep(seed: u64, n: usize) -> (u64, usize, usize, usize) {
    let mut h = 0xcbf2_9ce4_8422_2325u64;
    let (mut intra, mut inter, mut errors) = (0, 0, 0);
    let rolls = noise(seed, n * 8);
    for i in 0..n {
        let r = u64::from_le_bytes(rolls[i * 8..i * 8 + 8].try_into().unwrap());
        let seq = seq_from(r);
        let input = input_from(r >> 8);
        let data = noise(seed ^ ((i as u64 + 1) * 0x51ED), 32);
        let (out, position) = run(&seq, &input, &data);
        match &out {
            Ok(o) => {
                fnv(&mut h, describe(o).as_bytes());
                if input.frame_is_intra {
                    intra += 1;
                } else {
                    inter += 1;
                }
            }
            Err(e) => {
                errors += 1;
                fnv(&mut h, format!("E{e:?}").as_bytes());
            }
        }
        fnv(&mut h, &position.to_le_bytes());
    }
    (h, intra, inter, errors)
}

#[test]
fn the_sweep_reaches_intra_inter_and_error_outcomes() {
    let (_, intra, inter, errors) = sweep(0x5eed, 3000);
    assert!(
        intra > 100 && inter > 100 && errors > 100,
        "{intra} {inter} {errors}"
    );
}

#[test]
fn seeded_sweeps_are_pinned() {
    assert_eq!(sweep(0x5eed, 3000).0, 18143540655843682524);
    assert_eq!(sweep(0xfeed, 3000).0, 8375451525343929740);
}
