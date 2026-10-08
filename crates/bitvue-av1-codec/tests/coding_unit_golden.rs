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
    0x72e89dcde744cf0b,
    0x25b81add8453ae91,
    0x6ad4051b7155989b,
    0xcb1da3016cfdd308,
    0x7d6741535adfda03,
    0x5c6a75c95b4219ec,
    0xe027a5b7549b0204,
    0x86e47623ab045c10,
    0x21f5de3f319c1021,
    0xe3856e366f5bc604,
    0x077deda4837573b0,
    0x8010dcf19a5df939,
    0xb39f3853acdaeb7d,
    0xe36dc46fab57c502,
    0x36ae1d4c4d5ff81a,
    0x33c43142fb0dd7ff,
    0xcf50d97156f9780c,
    0xf1abecb7c0969226,
    0xec07599fca4ec426,
    0x8885242c0ecb0a30,
    0xd3e09c48385113c7,
    0xbc98fb68ec96a51c,
    0xabd5c561fe8938ab,
    0xeb53da67e45f8e92,
    0x11854c9738c78eca,
];
const GOLDEN_TOTAL_CUS: usize = 2466;

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
    0xa0bbb8d600cc93e4,
    0xf64c10c56112f49a,
    0xb372961145b2b9ca,
    0x2a894fcbc2c2e21c,
    0xa9199b8b437c6a13,
    0xff0f4b93682f4233,
    0x6d31a47f086c45e7,
    0xca69e8a702022ebf,
    0xadebf00865fbb1ab,
    0x5a258ecf59c8b28b,
    0x1d87b13b235fe9da,
    0x1d4a78ecdc1a1998,
    0xfa294acba8051e69,
    0x49e9d30c97d845aa,
    0x739b4c53f9a007ed,
    0x3f780de411bd909b,
    0x7d0dca5b196b9bbf,
    0xbb77ad84811cc8c5,
    0x6b10c1414ded9378,
    0xd702935fcd6465c5,
    0x013f11847a711fe9,
    0xfd1fd62892218002,
    0x33f5ba2590f2d410,
    0x55a561988ce50d7b,
    0xce6b98be5c5eaf08,
];
const GOLDEN_PREDICTION_MODE: &[u64] = &[
    0xa707d6c2a98e7d62,
    0x2955010bd02b24b6,
    0x055cf538347548a2,
    0x58e94f96defb6d9e,
    0x37a393a62f5d4a1a,
    0xc5aa677e547466a6,
    0x82a403d67d542a16,
    0x3336dc7913827690,
    0x399f8b7c29467632,
    0xda962905898d6a70,
    0xa9b1298822ff5c1a,
    0x64a1edc8e3d32a36,
    0x4f244450b7110c3e,
    0x6465ab4f3baca170,
    0xe8d1c3fb1abc308a,
    0x15cd2055a546a66f,
    0xafd8080026b736e0,
    0x2812e13786a18446,
    0x0242e0f4d0cf2ae8,
    0x420ee5bdf4515a39,
    0xadbb8a443dc52494,
    0x1a9f7cac2f4f3452,
    0x67d28206de77ac25,
    0x82b0447322e41832,
    0xf2189658cbcddaae,
];
const GOLDEN_TRANSFORM: &[u64] = &[
    0x0157475170952818,
    0xf128d0b506589006,
    0x7f5cdd7605a53b66,
    0xe6c0d2802b0c5bae,
    0x3d9675bd5cc2acaa,
    0xa719b6fbdf1f0df6,
    0x3a039c5b6f8a1090,
    0x320ee5972111d92a,
    0xd67c2d9b7833ec6c,
    0x46d1af9a862d47ec,
    0x64bef7e7fc8bc75e,
    0xe3849b5e705cc27a,
    0xe1973a80cf2b90f2,
    0xa00f8ff12d7f9200,
    0xf07061cf0f828976,
    0x191b3f294ac00be4,
    0x90fe63ae667e39bc,
    0x6daac980023d57d2,
    0xe7059a46583d23fc,
    0x77b9796cea630c50,
    0x42c188861c5ded9e,
    0xbb66ce8c502e7e5c,
    0x3d7870f7d3dab216,
    0x667e26cd8fc070ba,
    0x22c94b94327ab3fe,
];
