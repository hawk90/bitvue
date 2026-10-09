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
    0xf7742b208b960498,
    0xce1d6f3c74c239f2,
    0x1c06fb4e05ad024a,
    0x04323790fdc943d0,
    0xc0067c0d4434d4ab,
    0x8bbb317d200f40c5,
    0x5a39b77ae72bb2f4,
    0x89fe98d219525c5d,
    0x528409dfeb6753aa,
    0x9a9a66d3bf25e704,
    0xcc401a971e703884,
    0x39a60ca4ed08b26e,
    0x9c16a4b87b19537e,
    0xd1cb63103ef77172,
    0xbcc27469ef0238e2,
    0xe74ed13256b874ee,
    0x563443208d74be4e,
    0x4dfe8d2b20b38da5,
    0x6a773891653937a0,
    0x6809d3aee9518ccc,
    0xfacf5cce914a1b3d,
    0x7fb69d9fa085b465,
    0x7d0ecc23a3811f1f,
    0x7d15faef405ae14b,
];
const GOLDEN_PREDICTION_MODE: &[u64] = &[
    0xe89fcded60b22874,
    0x26241b5e2b1dcbbc,
    0x5428aed19935d21d,
    0x41b40522ecf8dbe4,
    0xe4ee9029d9be412a,
    0xa2fe20cc9895b65f,
    0xa7744b9b7ab2a241,
    0xd17b49b0e4431589,
    0x3774720baedf8a3a,
    0x23fcc7ff9e1c1748,
    0x38fa24a9924a6c53,
    0x1e55a7eb247d0c90,
    0xa77a70016002f32e,
    0xe59cfbbe524c5bc0,
    0x9b0a1d7699b54828,
    0x1ade4252bbaeb112,
    0x05654a82ab603d89,
    0x2095572bac61dc55,
    0x84009353828cf992,
    0xe04d3f844a2cf217,
    0xed7aae595d7542dd,
    0xeaa96f8abd42bd33,
    0x44a660d6a84bbfb6,
    0xbafff3ffcc127436,
    0x0df30c9a3dc31a0d,
];
const GOLDEN_TRANSFORM: &[u64] = &[
    0x859ade9f50460a5e,
    0x21b249430014eb8a,
    0x57de8de8d386adc0,
    0x5596d81e5f0c1fb6,
    0xdb60ac7359c6b1b0,
    0x6be64e480d8d2e3c,
    0xb342d408b9b8b0da,
    0xa0556b8a0f4f9022,
    0xe0ad1e6a9e226156,
    0x0fece89e315f6e5e,
    0x4d87e077e2f40efc,
    0x615455e82c528976,
    0xd269b8dec072b8fe,
    0x1e1ef6a45e4efa36,
    0x9a63cf215aeb2136,
    0xb5f9abec75288be8,
    0x59b26e41c5d367c4,
    0x38e73cb6060bcaa2,
    0x854ebe1f834f92da,
    0xb0a2facaee598b9a,
    0xe7c3d67485ec005a,
    0x7b2b14319ef06470,
    0x582d4eff92cbbac0,
    0x04d358011d83fec6,
    0x549855abd53e3ae4,
];
