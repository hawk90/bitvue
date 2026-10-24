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
    0xb87ef9d12e4bbf5d,
    0x5331372a687e3254,
    0x11f70813a494414c,
    0x88009509ea67c5ec,
    0x869681e2bdaed115,
    0x21af796ce947fd3f,
    0x03dfe3c00adafeab,
    0xfdeb37056e117b24,
    0xfc86d5fe12840318,
    0x251d3a0e9eb29698,
    0xe024a86399984c47,
    0x2f9dac90b7d4301a,
    0xc1297c373b4a39c2,
    0x17a1370a82f39b63,
    0x49ddf20e16fc9f30,
    0x072f461a8fea3c28,
    0x99dd126fc2e70278,
    0x7e602abd0a917302,
    0x9f98e9a465f7a508,
    0xcf2ada19ec647eba,
    0xb7451849ed872adc,
    0xc396d61e47ccdcd7,
    0x544059377bee9a9f,
    0x7aa596193e24f397,
    0x175d9b195ab1ba25,
];
const GOLDEN_TOTAL_CUS: usize = 8834;

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
    0x0bc08ad3783995b7,
    0xf32481071f052b41,
    0x73bede0621b67cd5,
    0x5fa83edb1ac1ec98,
    0x8f0dcad272cd4076,
    0xc09f4305e598bdae,
    0x381a9138dc78f991,
    0x3bec7cdba75eb04a,
    0x36c2be06ce664d68,
    0x739de753e96f7a14,
    0x9a9a66d3bf25e704,
    0xab46596b713a2bb2,
    0x39a60ca4ed08b26e,
    0x845a2bff4e8f2dd5,
    0xadf80a26dbdb9e0b,
    0x51307cce4d6bc1ce,
    0x434dd0ba1ec28bff,
    0x07c9af25a3ee0744,
    0x7953ff1861e31c7e,
    0xedfc26fbafacb03d,
    0xbe85941cf7389c95,
    0x16d03d5e12a236e7,
    0xf4e2fa6c3ba5c828,
    0x7d0ecc23a3811f1f,
    0x7d15faef405ae14b,
];
const GOLDEN_PREDICTION_MODE: &[u64] = &[
    0x4a16e8a8155f903e,
    0x085a6dbf60bde57c,
    0x630feec7040181d0,
    0xab562227b9654def,
    0x997dece8b2b10238,
    0xc507be7e17539ec3,
    0x19db41032e804cef,
    0xfe1568e1b03884bc,
    0x4e0c3afd4e55c47a,
    0x497e9b85313e6e80,
    0x38fa24a9924a6c53,
    0x1e091270f02bf810,
    0xa77a70016002f32e,
    0x0a51ac66c9161640,
    0x6f87e358a90cfc67,
    0x58bdc8f077a0cc32,
    0x37e42f04c9a56489,
    0x1ae1d3c08de3084c,
    0x970eea53998598ac,
    0x14ab227100bb8db7,
    0x7066cf9d59062aa5,
    0x986d870b1fdf76fd,
    0xfbdf803e2479dce2,
    0xbafff3ffcc127436,
    0x0df30c9a3dc31a0d,
];
const GOLDEN_TRANSFORM: &[u64] = &[
    0xc99d2e66c8d2b234,
    0x2084be4d51ba2f0a,
    0x12113ba1d3435b76,
    0x553206406f5fd1ee,
    0xbf50c74acf46a038,
    0xdf9bdddbfd9e5090,
    0x37860ee7b54f712e,
    0x121db5e866e9137e,
    0x7d56288a11546bee,
    0xd05db702ec157c92,
    0x4d87e077e2f40efc,
    0xa5d3327a560d22e6,
    0xd269b8dec072b8fe,
    0xfd0a4be0f3b9557e,
    0x38a90d2e6ccc4eaa,
    0xd576d3f4b3ef1b44,
    0x59b26e41c5d367c4,
    0xb702d0b335f43ec4,
    0xab88cf87054add62,
    0x8781733b0b31256a,
    0x6781e1b45ac1d552,
    0x400caf91796b92ba,
    0x7eedf55472156540,
    0x04d358011d83fec6,
    0x549855abd53e3ae4,
];

bitvue_test_alloc::install!();
