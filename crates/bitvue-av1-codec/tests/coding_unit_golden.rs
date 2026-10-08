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
    0x41041da376a6f13d,
    0x9759901e41613e8d,
    0x945994926b00f714,
    0x70b4da643db0836c,
    0xb402ffbaf865cc08,
    0x6dabaa29a60c7993,
    0x72f31137a8d58055,
    0x90ec8356ca5959e9,
    0x24e39e64f027f611,
    0xdff7200b30a2f7c4,
    0xbb3ef1609203e55c,
    0x4f929cf431645135,
    0x9c49e1e5c3722bb6,
    0x3884e82d840d59ef,
    0x0f737bd3e590fc20,
    0x123b05731ba3f24c,
    0xe1d3709a71b989e2,
    0xa930b68546b31fd7,
    0x43c0df98b2096953,
    0x0961d18382c36aa3,
    0x8f4be84d96dff9a9,
    0x0c0448101b94339c,
    0xa6fa694fcc214560,
    0x1c1cd27aba9562b4,
    0x1b9e925e7605670c,
];
const GOLDEN_TOTAL_CUS: usize = 8468;

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
    0xd1eb763f6613996f,
    0xa853517131f848a0,
    0xa6e0c57a23fbd5cb,
    0x0029d9a839840db1,
    0x18f4f66a0510c0cd,
    0xc2dcc0c1dd51846a,
    0xbf6d7ad0f787ba4c,
    0x3453f79c82473564,
    0x98b0798760c5fbef,
    0x9f43e7d361bceb2a,
    0x57b9b7ebc53d2411,
    0x1a65d8c6df0207b4,
    0xe8e686daac5c449c,
    0xa91b87117931ec2f,
    0x6e092f86e4a5efff,
    0xb92093ca936cae67,
    0xa969cedd15131cf1,
    0x91a16e379abffa8d,
    0xfd39e921a0e20c91,
    0x97beb17c5e366f6a,
    0x0fb8a0d34030e9f2,
    0xa26e8616cfc33fc9,
    0xee0ae4384703da6e,
    0x882001e04e2ef030,
    0x059028076acb9450,
];
const GOLDEN_PREDICTION_MODE: &[u64] = &[
    0x4ed58748ad145f19,
    0x4e59e96d197c80a6,
    0x988f68eee2e0a457,
    0xf6aeeaa66b52a695,
    0x71a0c41e39146da8,
    0xd1ccb7ab1619ac22,
    0x0d952bd974fc5973,
    0xebe95b79def0434a,
    0x55ea1151a7e07c28,
    0xa5916be5aef2fdbd,
    0x4a3f65465e5d9d73,
    0x8faf50b313885ef8,
    0xaeccef7bc96eb02f,
    0x8239a60439724e36,
    0xf3fbc55dc5145942,
    0xb1030366875b56d5,
    0xc28de591442e0aa5,
    0x87dbce1a179729a9,
    0xdf2ef7ffb5435d04,
    0x1e3a3f9ea34b0118,
    0x442e1ef711e635de,
    0x59f302b34e43a784,
    0xa16cf861b137dab5,
    0x5a54cf91e5ed96ab,
    0xec0a517223b4d3ba,
];
const GOLDEN_TRANSFORM: &[u64] = &[
    0x5e400afd203d7b4e,
    0x130cc777d62c03b2,
    0x7b4fb62414b80f0a,
    0x9697e001c51ed272,
    0x4f12c56603de1400,
    0x722326edb41304a4,
    0x351f745b268f0eaa,
    0x71cd91c2305f2066,
    0xf51c69b72533ba8e,
    0xc236eae4cc944dca,
    0x03f4567a30f673d6,
    0x050d8b9fccfa4abc,
    0xb1af31b51fd59ba2,
    0xb8c855ae47df8464,
    0xfddddba69518123e,
    0xf616d92c9b07423c,
    0xf731cffe8b9b99da,
    0xd14e9d50f5fc88d0,
    0x24d20bb404ef38fa,
    0xcd0d0ceaba933b78,
    0x0aaa0b76cbffd464,
    0x02d047a54d25f4c8,
    0x3fc712a0ca275e02,
    0x9c2e27d4d3c0b55c,
    0xff70c6e3ab32c3c0,
];
