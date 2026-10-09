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
    0xb766c42aecb9f859,
    0x99bde3797ac3778f,
    0x062dc3cb80336351,
    0xb327183f231ceb21,
    0x49fcd7b74cd5dd81,
    0xf2e7e8846f5020ac,
    0xce1cf5e9c8ad523b,
    0x96bdf43224aeff50,
    0x24e39e64f027f611,
    0x2493af22880df7b5,
    0x028e127da9fa07fc,
    0x2e80c6ee3021ae09,
    0x26929b2bc2931186,
    0x3884e82d840d59ef,
    0x24e4d66d575894e1,
    0x5a6f9cb5b071c77b,
    0xfb42a369f187a218,
    0x078970b53a3f44a8,
    0xd5f0467758f8519e,
    0x8514a48a92bb1419,
    0x072ca3f056c51f26,
    0x60479fc7a5131707,
    0xd176c5cb906b94ea,
    0xdc9a1cf4cd1bd5ef,
    0xb6298db9838b5f71,
];
const GOLDEN_TOTAL_CUS: usize = 8669;

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
    0x4204846333847b3c,
    0x51da0c86203e90cd,
    0x76fc55d39b3fc069,
    0x3be690e002b52140,
    0x7d52d80ca3cfe748,
    0x6e3304d1082e8314,
    0x1b44be9486c9f4cc,
    0x3b7741b08806c4e7,
    0x2c3c74d654d4fd99,
    0x5156aeb5b6fb6e41,
    0xa04ae981b4cf6cac,
    0x0a2ebedd29024359,
    0xe8cf20e238d96f1c,
    0x9aa833909b7fd321,
    0xc705db49d762c418,
    0xd99f5d50d0590ae3,
    0x512bfe01c9620ce6,
    0x45b224fba360bc64,
    0x6385a89ce8ffa8a8,
    0x8d6bb17db86ff107,
    0x5bed355ba0060658,
    0xfdf44e1c3904e147,
    0x179dab87e8884159,
    0xfef9f56e770b056b,
    0x6c2887180815ce3e,
];
const GOLDEN_PREDICTION_MODE: &[u64] = &[
    0xd500a6ade0214d1e,
    0x2a21291f35683239,
    0x4353715fa7a20b80,
    0xa9d39a415a831dba,
    0x40076937cde5304d,
    0x9ef0ffb359b9aa4c,
    0x602a6e84bbf7ebe6,
    0x402d08de23cc53f3,
    0xcc6c74d1331d2fc5,
    0x4880f936ed46c485,
    0xf9cb17f5a950845e,
    0x9c63c0413d19d4e7,
    0x1faa4b5c20ff146b,
    0x8fc4e1a69ac204c1,
    0x60cefb44b7cb05e1,
    0x87ed3bf34572ac33,
    0x8516c522da5e0446,
    0x65fbd5dd1322eaa3,
    0x162c72fca6ff5153,
    0x2f99078ed7e7635a,
    0x015b14d48c4c60f0,
    0x235601519962feba,
    0xb685099a312ef867,
    0x6a61898d5fb07117,
    0xc91e9a425273e167,
];
const GOLDEN_TRANSFORM: &[u64] = &[
    0x786e32b39a459d10,
    0x586ecacf8ec5fd38,
    0xd497a1600258885e,
    0x19fcd5e17783175a,
    0xcfb505f76a056dea,
    0xd5fb0c738b9230d2,
    0xa4dcbc58c9782cec,
    0x73944f3aa27e8d12,
    0xa354d244fb69ee68,
    0x0b21ed0b5bb07d02,
    0x30f51ffdbeae9e9e,
    0x0ea39746188c9bfa,
    0xe38bffbec77df30a,
    0x99e151657aad0ed2,
    0x65374ace081a2344,
    0xf75b44340fb20218,
    0xe57597c9e8fd360c,
    0xeb29e97110fa6246,
    0xdd682932c589d58c,
    0x4520bb82ddfe6bc6,
    0x1dbb269a36a7d70c,
    0x505ab831dbbdf044,
    0x55c862b94a5eb6fa,
    0xee39a9ba52540bf6,
    0xc511b590f1176662,
];
