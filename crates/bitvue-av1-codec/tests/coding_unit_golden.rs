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
    0xf3c065c060544506,
    0xae238ff8cd643f8c,
    0x17e7a7c96e1da8e4,
    0x84c79c1cca4dfbd2,
    0x746b2d656ab17736,
    0x8f1070199736bafa,
    0x03b9bc6cbd729617,
    0xe24da258c00c2e8b,
    0x920dfcf135993cb3,
    0xfbaae5012715255c,
    0xf0cfc7ffd483a67a,
    0x6d4084abaed63ca7,
    0x04baef46ccc06608,
    0x0f42ee6c083d6107,
    0x8625a29d2f57f1a8,
    0xda25b27aac6a5b09,
    0xda026790afc9c052,
    0xee9f4174e855e7dc,
    0xd7fa61bdda45e53d,
    0xf4731a1a7a6f7fcf,
    0x720f7d9b8f539bb4,
    0xd670fceac2d54d78,
    0xb16fde5536310145,
    0x410f5e838b214ebc,
    0xa2037d049e158836,
];
const GOLDEN_TOTAL_CUS: usize = 5783;

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
    0xcd4221f33d95a44d,
    0x86c2ef7e2c1f63b6,
    0xb3c1143a8cf0d98c,
    0x78a2e4d60913ca06,
    0x8815818e33df2a16,
    0xb73f4b5809397288,
    0xe855db7867177130,
    0xcc1fdd9c7df656e3,
    0xfc81e6dee3521b7c,
    0xdafc3336934b8df1,
    0x36590a188cb1f24d,
    0x0b72e7df4c002943,
    0xa5f2d3ceb312b058,
    0x028a39ac69ae6a92,
    0x6114768c8da2c18f,
    0x2328eff295571495,
    0x79eb10e0a4508705,
    0x9f4c1201f6d6d6a5,
    0x74958a326bed1acd,
    0x777beb780b12c116,
    0x0c575699714c8a5d,
    0x64c402dc886c96f3,
    0x14302223fc7bc24a,
    0xbac5c9986eb0afec,
    0xc6ec90741cd8b35b,
];
const GOLDEN_PREDICTION_MODE: &[u64] = &[
    0xa3e980ef8320a6ac,
    0x00bf77f8b161dfda,
    0x3ee333760e8a95ee,
    0x9fb7da2a673073ce,
    0xe097d3571a8068c9,
    0x8b296a4ba6789ce8,
    0x8b3887176afa47c9,
    0xb455ddc769170333,
    0xd12b2dc8b7ca058b,
    0xd21c28d245ecdde8,
    0x224cd72f1bc2e824,
    0x5892b472fdcbdc16,
    0x67e14184c03cc889,
    0xdee612e4b6bb6757,
    0x4e61c1330488be3a,
    0xca8c80cb3854eae4,
    0xb4a92cebe268299e,
    0x759921a7db44b06a,
    0xdc9cba1aa598e732,
    0x3e872bee180b2c9a,
    0x6127d1e5319de6a5,
    0xdb2d3b236553851b,
    0x71df70e7865a5795,
    0xb844bedef7522f0f,
    0x5a87ecfcf6fcc716,
];
const GOLDEN_TRANSFORM: &[u64] = &[
    0xf98c33a936092850,
    0x375961ca701183cc,
    0xdaa24fcf78c1d82c,
    0x3068d252b6854e7a,
    0x2f08f345a90ef66c,
    0x0b54abd83994a8e0,
    0xe73283e95879cebc,
    0xa5f8ad35b85b3b62,
    0x3b10ce01a9a4287e,
    0xb4fb115b95ba77ee,
    0x1ead1665a4851b6a,
    0x96980fdef9289c98,
    0xb0880df733915524,
    0xf13a46da0c2cec3c,
    0xd799477373ec9ef0,
    0x4d24369658f5d864,
    0xe77742fde39c252a,
    0xbfdadd6280676c3c,
    0x77f8fe7ac255f752,
    0x6c284bdfb13bc8cc,
    0xacf696a7311f7dae,
    0xd39ff5c90d3326b8,
    0xa7783b7949ff0882,
    0x0b9c80fce5b7c7da,
    0xa95c00a11901a0de,
];
