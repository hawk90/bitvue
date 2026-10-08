//! Characterization test for `tile::coding_unit` (`parse_coding_unit` and friends).
//!
//! Parses every frame of the real AV1 fixture through `parse_all_coding_units` and hashes the
//! full `Debug` rendering of each frame's coding units (modes, reference frames, MVs, tx sizes,
//! residual stats, ...). Digests are folded per 10 frames so a failure points at a range.
//! Recorded from the unmodified `coding_unit.rs` before it was split by responsibility.
//!
//! To re-record after an intentional parsing change, run with `GOLDEN_PRINT=1`.

use bitvue_av1_codec::obu::{ObuIterator, ObuType};
use bitvue_av1_codec::overlay_extraction::{
    extract_partition_grid_from_parsed, extract_prediction_mode_grid_from_parsed,
    extract_transform_grid_from_parsed, parse_all_coding_units, ParsedFrame,
};
use bitvue_av1_codec::parse_ivf_frames;
use std::path::PathBuf;

const FRAMES_PER_DIGEST: usize = 10;

fn fnv1a(bytes: &[u8], mut h: u64) -> u64 {
    for b in bytes {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

fn fixture() -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../test_data/av1_test.ivf");
    std::fs::read(path).unwrap()
}

/// Parses every fixture frame and folds `render(idx, frame)` into per-chunk digests.
fn chunk_digests(render: impl Fn(usize, &ParsedFrame) -> String) -> Vec<u64> {
    let data = fixture();
    let (_hdr, frames) = parse_ivf_frames(&data).unwrap();
    let seq_bytes = frames
        .iter()
        .take(8)
        .find_map(|f| {
            let mut iter = ObuIterator::new(&f.data);
            while let Some(Ok(found)) = iter.next_obu_with_offset() {
                if found.obu.header.obu_type == ObuType::SequenceHeader {
                    return Some(f.data[found.offset..found.offset + found.consumed].to_vec());
                }
            }
            None
        })
        .expect("fixture has a sequence header");

    let mut out = Vec::new();
    let mut chunk = 0xcbf2_9ce4_8422_2325u64;
    for (idx, frame) in frames.iter().enumerate() {
        let obu_data = [seq_bytes.as_slice(), frame.data.as_slice()].concat();
        let parsed = ParsedFrame::parse(&obu_data).unwrap();
        chunk = fnv1a(render(idx, &parsed).as_bytes(), chunk);
        if (idx + 1) % FRAMES_PER_DIGEST == 0 || idx + 1 == frames.len() {
            out.push(chunk);
            chunk = 0xcbf2_9ce4_8422_2325;
        }
    }
    out
}

/// Per-chunk digests plus the total number of coding units parsed.
fn digests() -> (Vec<u64>, usize) {
    let total = std::cell::Cell::new(0);
    let out = chunk_digests(|idx, parsed| {
        if parsed.has_tile_data() {
            let cus = parse_all_coding_units(parsed)
                .unwrap_or_else(|e| panic!("frame {idx} failed to parse: {e:?}"));
            total.set(total.get() + cus.len());
            format!("{idx}:{cus:?}")
        } else {
            format!("{idx}:no-tile-data")
        }
    });
    (out, total.get())
}

const GOLDEN: &[u64] = &[
    0x9ee5718b498945b2,
    0x0822e3c831000997,
    0xb8477b54f72ce9b3,
    0xe5793aa6e9b2d92e,
    0x8d81cf96e6fe6ed7,
    0x7d05397b0d8dfdaf,
    0xb619e1ce50b2dcdb,
    0x7b9e27af77699f7b,
    0x7725b0bc94cdb279,
    0xae44e0ce78339741,
    0x96d97ca096e95c33,
    0x3adc1cb452e25ab0,
    0x97a114d3a866ac55,
    0x9aca3268432f78b7,
    0x6bc53626bd6b64b4,
    0xb0aaf1f455443b00,
    0x9994ed3056a86451,
    0x78e85616fafa4038,
    0xe685e34bbe03ca15,
    0x236e07ce32078a92,
    0x7f1336f5d8498118,
    0xcdc0d3bfae273f2b,
    0x3838f5ef330239fb,
    0x149d244813b16077,
    0x8005c9385320fe48,
];
const GOLDEN_TOTAL_CUS: usize = 11159;

#[test]
fn real_fixture_coding_units_match_the_recorded_digests() {
    let (got, total) = digests();
    if std::env::var_os("GOLDEN_PRINT").is_some() {
        println!("GOLDEN_TOTAL {total}");
        for g in &got {
            println!("GOLDEN 0x{g:016x}");
        }
        return;
    }
    assert!(
        total > 1000,
        "fixture should yield many coding units, got {total}"
    );
    assert_eq!(total, GOLDEN_TOTAL_CUS, "total coding unit count changed");
    assert_eq!(got.len(), GOLDEN.len(), "digest count changed");
    let bad: Vec<String> = got
        .iter()
        .zip(GOLDEN)
        .enumerate()
        .filter(|(_, (g, w))| g != w)
        .map(|(i, _)| {
            format!(
                "frames {}..{}",
                i * FRAMES_PER_DIGEST,
                (i + 1) * FRAMES_PER_DIGEST
            )
        })
        .collect();
    assert!(
        bad.is_empty(),
        "coding units changed in: {}",
        bad.join(", ")
    );
}

/// The second real parse pass (`overlay_extraction::partition`, used by `get_frame_analysis` for
/// the partition grid) has its own call into the superblock parser, so it is pinned separately.
#[test]
fn real_fixture_overlay_grids_match_the_recorded_digests() {
    let kinds: [(&str, Vec<u64>, Vec<u64>); 3] = [
        (
            "partition",
            chunk_digests(|i, p| format!("{i}:{:?}", extract_partition_grid_from_parsed(p))),
            GOLDEN_PARTITION.to_vec(),
        ),
        (
            "prediction_mode",
            chunk_digests(|i, p| format!("{i}:{:?}", extract_prediction_mode_grid_from_parsed(p))),
            GOLDEN_PREDICTION_MODE.to_vec(),
        ),
        (
            "transform",
            chunk_digests(|i, p| format!("{i}:{:?}", extract_transform_grid_from_parsed(p))),
            GOLDEN_TRANSFORM.to_vec(),
        ),
    ];
    if std::env::var_os("GOLDEN_PRINT").is_some() {
        for (name, got, _) in &kinds {
            for g in got {
                println!("GOLDEN_{} 0x{g:016x}", name.to_uppercase());
            }
        }
        return;
    }
    for (name, got, want) in &kinds {
        assert_eq!(got.len(), want.len(), "{name}: digest count changed");
        let bad: Vec<String> = got
            .iter()
            .zip(want)
            .enumerate()
            .filter(|(_, (g, w))| g != w)
            .map(|(i, _)| {
                format!(
                    "frames {}..{}",
                    i * FRAMES_PER_DIGEST,
                    (i + 1) * FRAMES_PER_DIGEST
                )
            })
            .collect();
        assert!(bad.is_empty(), "{name} grid changed in: {}", bad.join(", "));
    }
}

const GOLDEN_PARTITION: &[u64] = &[
    0x3a49e67387fe7b8c,
    0xcc06c1e0b7964349,
    0x49d2ec0704ca797e,
    0xf6d5b80ed1824464,
    0xe7b1a2adaf192925,
    0x8c0ed40f09754b82,
    0xfac87815a8b2f2dc,
    0xfb826242fdd7f8a9,
    0x4da6830e70bc024f,
    0x53bbef761ef479a1,
    0x335fd498b9e3100b,
    0x21fabcb17c5b881e,
    0x0ec55674a5ce382c,
    0xbefb42d811491197,
    0xa450ed13cfedd226,
    0xb2ac90e65694ee33,
    0x62f8e19e643793a9,
    0xbd898e1b7abeb9fd,
    0xe8f300f4c729b00e,
    0x206e198ae0d95f94,
    0xcf5020931e434f56,
    0x06aea8dff104b656,
    0x3d2d61f7df247e3d,
    0xfd6adef539b8bfce,
    0x76cddfb7b6aed2f8,
];
const GOLDEN_PREDICTION_MODE: &[u64] = &[
    0x52322896f272b773,
    0xeffaf301a3c08694,
    0x69d61bcb787172e7,
    0xccb7f0a61eb58fb9,
    0xcebd3fdea46549a4,
    0x693209fcd21f780b,
    0x9d40295efdf98e48,
    0x44f6bbdb95e5fada,
    0x99d72c1b8ee9ad39,
    0xa6e91f73984bf111,
    0x99863cbe8a3afc75,
    0x26a0db61b4bc897e,
    0x62c4f98b5e8b049f,
    0xdb5b9f71fd8f1861,
    0x417567afb49f51f5,
    0xa9a2b1ad87b1c9fb,
    0x0c994fbf53120a85,
    0xf7eb77c20b020947,
    0xb8a2ff40c3b9a334,
    0xaf59e0d43063edc6,
    0x21e284a49751fde4,
    0x1d1c8ede5fa2cbe1,
    0xbe9586a01e578f51,
    0x7bcd4f1e7a864881,
    0x54d2a18840f8ffc2,
];
const GOLDEN_TRANSFORM: &[u64] = &[
    0x7649b01b340c4974,
    0xe75a9514fb693cea,
    0x09534f5b3883f5f0,
    0x343966e56ab5b46e,
    0x4c3121139f3faa52,
    0x974314443bc525a6,
    0xf42dc71b36eb2dd2,
    0xad6f699db16d556c,
    0xe0c01e9a4385def8,
    0x2c4166333a0728f8,
    0x9a45c8481ddb5864,
    0x86f4762565d62acc,
    0x3622c17235122160,
    0x02c47b28ce127ebe,
    0x11f949ecae1faa88,
    0x83506deeb3cfcc14,
    0x048175b2d1c9a908,
    0x56fe36a9bda2d686,
    0xce97d83698b26962,
    0x901293d4bed9c11a,
    0xb02df994ee7138a8,
    0x0a6a4f85b554d1f0,
    0x95545762dd96acea,
    0xe10da01865482fc6,
    0x3e4d2a4d365316d8,
];
