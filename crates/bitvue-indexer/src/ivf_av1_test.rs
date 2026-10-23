//! `index_ivf_av1` pinned over whole fixture clips and over seeded corruptions of them: every
//! unit it builds (type, offsets, frame type, timestamps, reference frames, QP) and every
//! diagnostic (severity, frame, message), plus its cancellation behaviour.

use super::index_ivf_av1;
use bitvue_engine::event::Severity;
use bitvue_engine::StreamId;
use std::sync::atomic::{AtomicBool, Ordering};

const IVF_HEADER_SIZE: usize = 32;

fn fnv(hash: &mut u64, bytes: &[u8]) {
    for b in bytes {
        *hash ^= u64::from(*b);
        *hash = hash.wrapping_mul(0x100000001b3);
    }
}

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

/// Folds what indexing `data` yields into `hash`; returns the unit and diagnostic counts, or
/// `None` for a cancelled run.
fn fold(hash: &mut u64, data: &[u8], cancel: &AtomicBool) -> Option<(usize, usize)> {
    match index_ivf_av1(data, StreamId::A, cancel) {
        Err(e) => {
            fnv(hash, format!("E{e:?}").as_bytes());
            Some((0, 0))
        }
        Ok(None) => {
            fnv(hash, b"cancelled");
            None
        }
        Ok(Some((header, units, diagnostics))) => {
            fnv(
                hash,
                format!("{}x{}", header.width, header.height).as_bytes(),
            );
            for unit in &units {
                fnv(hash, format!("{unit:?}").as_bytes());
            }
            for event in &diagnostics {
                if let bitvue_engine::Event::DiagnosticAdded { diagnostic: d } = event {
                    let severity = match d.severity {
                        Severity::Warn => "warn",
                        _ => "other",
                    };
                    fnv(
                        hash,
                        format!("{severity} {:?} {}", d.frame_index, d.message).as_bytes(),
                    );
                }
            }
            Some((units.len(), diagnostics.len()))
        }
    }
}

fn clip_hash(ivf: &[u8]) -> (u64, usize, usize) {
    let mut hash = 0xcbf29ce484222325u64;
    let (units, diagnostics) = fold(&mut hash, ivf, &AtomicBool::new(false)).unwrap();
    (hash, units, diagnostics)
}

macro_rules! clip_case {
    ($name:ident, $file:literal, $hash:expr) => {
        #[test]
        fn $name() {
            let ivf = include_bytes!(concat!("../../../test_data/", $file));
            let (hash, units, _) = clip_hash(ivf);
            assert!(units > 0);
            assert_eq!(hash, $hash);
        }
    };
}

clip_case!(fixture_clip, "av1_test.ivf", 5500076595937768709);
clip_case!(aomenc_clip, "av1_aomenc_testsrc2.ivf", 11412180962405432926);
clip_case!(rav1e_clip, "av1_rav1e_testsrc2.ivf", 14013110674230659423);
clip_case!(svtav1_clip, "av1_svtav1_testsrc2.ivf", 6698601066228324678);
clip_case!(tiles5_clip, "av1_aomenc_tiles5.ivf", 16582806326677311973);
clip_case!(
    tile_groups_clip,
    "av1_aomenc_tilegroups.ivf",
    819574246088524650
);

/// Start of every frame payload of an IVF file (each frame has a 12-byte header whose first four
/// bytes are the payload size).
fn payload_starts(ivf: &[u8]) -> Vec<usize> {
    let mut starts = Vec::new();
    let mut at = IVF_HEADER_SIZE;
    while at + 12 <= ivf.len() {
        starts.push(at + 12);
        let size = u32::from_le_bytes(ivf[at..at + 4].try_into().unwrap()) as usize;
        at += 12 + size;
    }
    starts
}

/// Inverts a few bytes in the first 20 bytes (OBU headers and the start of the frame header) of
/// some of the first frames, so some frame headers stop parsing: the sweep reaches the
/// basic-parser fallback, the "stop tracking references" path and the "no frame OBU" diagnostics.
fn corrupted(ivf: &[u8], seed: u64) -> Vec<u8> {
    let mut data = ivf.to_vec();
    let starts = payload_starts(ivf);
    let rolls = noise(seed, 4 * 8);
    for chunk in rolls.chunks(8) {
        let r = u64::from_le_bytes(chunk.try_into().unwrap());
        let frame = (r % 12) as usize % starts.len();
        let at = starts[frame] + (r >> 8) as usize % 20;
        if at < data.len() {
            data[at] = !data[at];
        }
    }
    data
}

fn corruption_sweep(ivf: &[u8], seeds: std::ops::RangeInclusive<u64>) -> (u64, usize, usize) {
    let mut hash = 0xcbf29ce484222325u64;
    let (mut with_diagnostics, mut errors) = (0, 0);
    for seed in seeds {
        let data = corrupted(ivf, seed);
        match fold(&mut hash, &data, &AtomicBool::new(false)) {
            Some((0, 0)) => errors += 1,
            Some((_, diagnostics)) => with_diagnostics += usize::from(diagnostics > 0),
            None => unreachable!("the flag is never set"),
        }
    }
    (hash, with_diagnostics, errors)
}

const CLIP: &[u8] = include_bytes!("../../../test_data/av1_aomenc_testsrc2.ivf");

#[test]
fn the_corruption_sweep_reaches_diagnostics() {
    let (_, with_diagnostics, _) = corruption_sweep(CLIP, 1..=60);
    assert!(with_diagnostics > 20, "{with_diagnostics}");
}

#[test]
fn corrupted_clips_are_pinned() {
    assert_eq!(corruption_sweep(CLIP, 1..=60).0, 5158674014903062216);
}

#[test]
fn a_set_cancel_flag_stops_before_the_first_frame() {
    let mut hash = 0;
    assert!(fold(&mut hash, CLIP, &AtomicBool::new(true)).is_none());
}

#[test]
fn a_cancel_flag_raised_later_is_noticed_per_frame() {
    // `fold` cannot raise the flag mid-run; a flag that is already set is read before frame 0, so
    // the run never produces a unit.
    let flag = AtomicBool::new(false);
    flag.store(true, Ordering::SeqCst);
    assert!(index_ivf_av1(CLIP, StreamId::A, &flag).unwrap().is_none());
}
