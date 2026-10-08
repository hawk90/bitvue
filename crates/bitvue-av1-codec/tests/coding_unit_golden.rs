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
    0x001e3c26b9184642,
    0x3b7c86686cd85891,
    0x61ed794102e0ab06,
    0xa6f4f8c2d796fefe,
    0x447d0192e28ebbbf,
    0xdc8e89364435ee34,
    0x5647e13e195bddb3,
    0x04814da6c717199f,
    0x4e637db4e7f4ecbc,
    0xc9879ef8146a3f38,
    0xf4841d2886d8fd33,
    0xba5a9424f460e519,
    0x48b9501e2d6f26c3,
    0x8ce49b9ede1f2cb8,
    0xb7f11d1ae3da07a7,
    0xd0da4a4b22d5792d,
    0x7d526bce2c04cef7,
    0xfbec40f0ed94561c,
    0xf5c5f492c72870f2,
    0x171e99a3736357ae,
    0xf5605cdc90bdc592,
    0x8e60fb617f2bad09,
    0xba1e875b33e5fa6b,
    0xe7469524142797ea,
    0x0f2930c11b0aac7b,
];
const GOLDEN_TOTAL_CUS: usize = 10166;

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
    0x8433e7a9942a469a,
    0xde99d20c98bd4401,
    0xf8686448276d0e6f,
    0x2ebc05feba88d3d2,
    0x0958b38bf5db175b,
    0x83ada1f65b27a90a,
    0x528f1e78e8581bee,
    0x5194c068978294da,
    0x19edf4b915a2a8ef,
    0x7cc44bff45d60bb9,
    0x43f3ac8403a38383,
    0x8ebe8a811b1a26d9,
    0xc348a2034c35d6d9,
    0x061e623f9c4f4ba5,
    0xd04b902f775ab262,
    0x123f6080410f2f32,
    0x8542b84ff57f84b5,
    0xa93eedf0f37a28c9,
    0x7c15c10e9c31f5d8,
    0x2b4ae710a3e55689,
    0x6716fed6ea1a3886,
    0x4d8c93086072966c,
    0x688aa6556adc2b48,
    0x0bc0e1eb66552810,
    0x3fc45f240667f739,
];
const GOLDEN_PREDICTION_MODE: &[u64] = &[
    0x809eed47083ba8ed,
    0x8dd5d1cd6aca182e,
    0xafc6c9336103eacf,
    0xc706b56e41c98560,
    0xdf2f8ca021376969,
    0x11a416a573afdb1b,
    0x31bafcaa0e971769,
    0x36c8f36b597ae77d,
    0xaddd36f59cbdfef3,
    0x8d154bbf09531dd4,
    0xa433d791aa1ca0fa,
    0xb8105828b77a8227,
    0xfe59ea873927c6fd,
    0x028c2a8cd2ab687c,
    0x9dfbd02fad2cf483,
    0xe7e54b6efa779295,
    0xaf45d31e6e8cf5b7,
    0x3034f953a5a2ebda,
    0x398263b184519be1,
    0x9035f0bce641e66e,
    0xa755f77d569c914d,
    0xad6e3d124e46ab77,
    0xbe920ba02c46c445,
    0xaaaf24cb60e2e187,
    0xb7bcd5c69ba021d2,
];
const GOLDEN_TRANSFORM: &[u64] = &[
    0xd3cbe61b60d08a10,
    0xff2ce8f4970d0b36,
    0x7fae5230da0009ae,
    0x8c64f0ab3ade991a,
    0x940c36a21f630cde,
    0xe6549119beaabce2,
    0xffe311e08309e6a8,
    0xf0021064afc2eb9a,
    0xd099c70f54b63dd4,
    0x224ce823b5d3dd18,
    0x8024a66a2628b502,
    0xd2b6b42ac29b97ec,
    0x2164216c4331e334,
    0xd0d6845d2a0fbc12,
    0x706d575f303d677c,
    0xbac0a3f6e443ff98,
    0x1f0ddb30ae51da9c,
    0x179c538cd344e38e,
    0x04ac406021d9ea5e,
    0xbc9a358f20a95e16,
    0x2642ea8f9eeca806,
    0xbb012d9b1a968b3c,
    0xb08635cb1b74a0b6,
    0x112c9c9dbb6f22b0,
    0x4a91c46a698ffefe,
];
