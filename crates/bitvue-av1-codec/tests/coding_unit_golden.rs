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
    0x06ed328ef5b041f0,
    0x0ce0b175d3d2b709,
    0x3d67ad677480977d,
    0x4c13fdfc8a4e0b26,
    0xb576a43ef60460d1,
    0x2e1b6128e08eba6d,
    0xedc55134121078cb,
    0x54f23889724220e5,
    0x1d805511ee188242,
    0x0d69e56fb5fb6897,
    0xe024a86399984c47,
    0x2901e6ace4a7b1ca,
    0x5d1e26fde7a07f02,
    0xb715763b2e95f2cb,
    0xc8b8bd401ee07385,
    0xa27c206ec290fa35,
    0x0ce92dcbc7dd021a,
    0x87644612eebae95f,
    0xed08f2279f41865e,
    0xfb8a442d11eb79b8,
    0xf4dee1428d75202b,
    0xe2e4deda9fefde81,
    0x1805bad314e51e05,
    0x7aa596193e24f397,
    0x175d9b195ab1ba25,
];
const GOLDEN_TOTAL_CUS: usize = 8835;

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
    0x2504fcba9a8a2bca,
    0x3155b46e3e71b70e,
    0x83921e73c13c57c1,
    0xa9eefb5546b21159,
    0x4485e24b7da26c81,
    0x9c3b876aafa4abc9,
    0xe08f7371e5dde158,
    0x3ece157ef62082eb,
    0x54e2ff5313df1195,
    0x58fc03ce4bcef715,
    0xce602c06facc7020,
    0x5d3f093b26d4365b,
    0x81b933a642c048e3,
    0x5448d0d7b784aabe,
    0x93962e1822653215,
    0x7f59fa7dcb53e39e,
    0x9f1cf61681123795,
    0x46f58f9bdb0fe741,
    0x10e5be93382b9ca0,
    0x6a773891653937a0,
    0x7fb575aaf6e14090,
    0x942e34d77a595d0e,
    0xb85896263ff5629b,
    0x690ebf151fe4746a,
    0xd826cd0e2934c958,
];
const GOLDEN_PREDICTION_MODE: &[u64] = &[
    0xe89fcded60b22874,
    0x42b81e5a655d2743,
    0x188a44e4a01b8b9e,
    0xc08d7704333fe8db,
    0x67d7557e6b8bfb5f,
    0xaf556e19a8595f80,
    0xdd11db3289999c72,
    0xf316d6c69eb045ba,
    0x09aa7808a6727645,
    0x63a2aa5c43e24431,
    0xe26eb50e3e570238,
    0xaa808cc1b9c6eced,
    0xd89e370790468ef5,
    0x6a6d96a1d80c1ceb,
    0x74dbe452c7bae7fd,
    0x1a1b776eaae1e765,
    0x03b5b89be5959eaa,
    0xd4d110be93d12530,
    0x152ab252bc5e03bd,
    0xe04d3f844a2cf217,
    0x4b48a3ef5b7b0552,
    0x226ac66bb90ffd36,
    0x0b8924658f09ebff,
    0x141a23a97ecd2af7,
    0x3cf74c7361eb6fa0,
];
const GOLDEN_TRANSFORM: &[u64] = &[
    0x859ade9f50460a5e,
    0xf980491fe1535004,
    0x3f8a8fec6b236a52,
    0xd230ac2753361830,
    0x245942d1e6012852,
    0x7ecc1dec3740308e,
    0x26c988082455f834,
    0xd1dca88be259b374,
    0xdacdf14297803608,
    0x56306e20c4ec7eb8,
    0x5387fff24c6554da,
    0x3945b7592fcf151c,
    0x7d82be3a7a1832b4,
    0x66b9a63fb9e9a55c,
    0x9b9395e44b0c60c8,
    0x6c6510773867bbe6,
    0x2bbe2b4233d2ee2a,
    0x121332e692254688,
    0x434ffaee1b902998,
    0xb0a2facaee598b9a,
    0xc497e2b6e0f014b4,
    0xc08912d39889dd7a,
    0x8a8399e3e843f882,
    0xc175822e2688b878,
    0x2a8371d21087f5a6,
];
