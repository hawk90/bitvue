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
    0xe267a81fa549d40c,
    0x2a4e19485470e100,
    0xa559959b4fe4908e,
    0x4f3974be5e0c10be,
    0x3f1b6f2ceb5e1eb8,
    0xde73d6ff97e80210,
    0x33d48b22f73d9e5b,
    0xd6e69d9c89a747ed,
    0x2d5559e3cdc3e323,
    0xb76cbbb7d68dba0c,
    0x259e62fb6e7b81a9,
    0x5be2e3ad62e43817,
    0x085183e006273fb8,
    0xb85138453db87647,
    0x72778898513439f2,
    0xe6ca8eec106c5e39,
    0x280d757ea45e4a36,
    0xeeedd999dc30d308,
    0x21f240566cd25ed7,
    0x3ebc6b67cfa5ae95,
    0x51030987cc639a5c,
    0xf180bee7ef0615ff,
    0xa4de123ce2d8b81b,
    0x884a7cc7937de2cc,
    0x0f2930c11b0aac7b,
];
const GOLDEN_TOTAL_CUS: usize = 7909;

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
    0x0d1ecaaee6b32a38,
    0x1ff06a394bd40781,
    0xb463bc39c5652d4f,
    0x21772ecf3a0ba737,
    0xf203301f177f726a,
    0xaedc30953d84573a,
    0x84c255dc70adef73,
    0x2cd5bee4c66a6c78,
    0x1f8d6792dd3d1182,
    0x312cd27eb9343b7f,
    0xeb31c9e0df9baf35,
    0x87bb7ac690e24c95,
    0x874901925d76e5c3,
    0x5bba0296e746bf64,
    0xace0236bc8892aaf,
    0x12cb81ba2bbae76a,
    0x96c62a79ac8fc154,
    0x992e5489a0a25467,
    0xd0a5ec0efc0ac9ad,
    0xf8079a73cbef1a08,
    0xb7afadfe470f39cb,
    0x5995e5d3dfd13eec,
    0x726f0f9c261c27f4,
    0xfa2b65110a4f225d,
    0x3fc45f240667f739,
];
const GOLDEN_PREDICTION_MODE: &[u64] = &[
    0xc3dc064082b46124,
    0x00497697c2e73202,
    0xb4de3dd6d6f37804,
    0x87c11cecdac14eeb,
    0x8fa5c50388830989,
    0x67214226ae84c332,
    0x745f2f5ce1207a9c,
    0x85b9d744b8b8b3a1,
    0x0511fc3db8af9d29,
    0xd54899bac11ca344,
    0x603a01af5c2ba3bf,
    0x6ce6f63c32ca296f,
    0xd82b724a9cd6a299,
    0x9271685cc780b507,
    0xff4f41de586a6082,
    0x23e55955f6916010,
    0xd7a36fedf8678c47,
    0x6e1ebfa485e857c2,
    0xe8da3c9562170bb8,
    0x3c3193fa9a67c8db,
    0xd5531e96f759a854,
    0xfdbafdbcdb62baaf,
    0x16edd3ad4d0989dd,
    0x8ee248e94766a624,
    0xb7bcd5c69ba021d2,
];
const GOLDEN_TRANSFORM: &[u64] = &[
    0x95451f000507b240,
    0xfc202448eb624282,
    0x768d702838d5a1be,
    0xb7f3dacf04d6de44,
    0x738938355a7f30ce,
    0x15465be69f2fa0b4,
    0xcdc9c9eb569ce828,
    0x480d332f9f0e4370,
    0x871e87944555932c,
    0x659e3405183b3af8,
    0x7ca6f0a5f1ebabba,
    0x94cf1643e2c71790,
    0x236bd8b0c35e9fe8,
    0x283ca559f81345ac,
    0x7fb4c9b49e69244a,
    0xe27b1a75fe0a80be,
    0x7af8b63fad745d90,
    0xfc3b664e987a8818,
    0xfa3be684ba68a35c,
    0xa1b712b76da09628,
    0x8626c9fe817ded58,
    0xea231b394314608c,
    0xc1c8cc5ddbaf7414,
    0x25e255d554a70b14,
    0x4a91c46a698ffefe,
];
