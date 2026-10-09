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
    0xca3ef18a107bc4e9,
    0x556a60fb8114b831,
    0x0961d18382c36aa3,
    0x8f4be84d96dff9a9,
    0x0c0448101b94339c,
    0x78058947ef31ff9f,
    0x1c1cd27aba9562b4,
    0x1b9e925e7605670c,
];
const GOLDEN_TOTAL_CUS: usize = 8807;

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
    0x34d4c1ea22d743e7,
    0x4a011b9b87e7a2d6,
    0x57d9b06e71c88522,
    0x644c0aa29b84e1fb,
    0x7d52d80ca3cfe748,
    0xd974e2f2b9780681,
    0x1b44be9486c9f4cc,
    0x758792594e6f6b38,
    0x2c3c74d654d4fd99,
    0x09056fec88c88e92,
    0x1fb482b249b79f0f,
    0xd8daa6ac47231f5e,
    0xbb95a1bddf33cad7,
    0x9aa833909b7fd321,
    0xc705db49d762c418,
    0x4f6cfdd9190a2688,
    0x5e8c122140599b9d,
    0x4f26848473152fbb,
    0x028163a76573d512,
    0x5cae88df9a0e1ce8,
    0x9a17311a9f6813ae,
    0x705cefb7627b967f,
    0x179dab87e8884159,
    0xfef9f56e770b056b,
    0x68362b01010718fc,
];
const GOLDEN_PREDICTION_MODE: &[u64] = &[
    0x11233082395293f0,
    0xf450f519c087dd17,
    0x05f911d4173fe485,
    0x65e86ba538916ed2,
    0x40076937cde5304d,
    0xe9c27087560aa5a4,
    0x602a6e84bbf7ebe6,
    0x15e480a532893af1,
    0xcc6c74d1331d2fc5,
    0xc805bf1a30afec94,
    0x658dc7efaa85068b,
    0xa45525ab289d86f8,
    0x27459d12a717ca27,
    0x8fc4e1a69ac204c1,
    0x60cefb44b7cb05e1,
    0xbaeaeeb0f47f4e66,
    0x2cdfcc73f7e3bc22,
    0xc13d2c35b2cac14b,
    0x83bfff34768e14a1,
    0x8b55925d969a3c44,
    0x3460146625dd6cb8,
    0x20778d093e3860b5,
    0xb685099a312ef867,
    0x6a61898d5fb07117,
    0x3c0d6ff78c7f6e97,
];
const GOLDEN_TRANSFORM: &[u64] = &[
    0xc4694152e43b601e,
    0x49fc07683af69f54,
    0x85c7fc5ab6a7d834,
    0x86e9a9c1a12c1a74,
    0xcfb505f76a056dea,
    0xfd11779c74869e56,
    0xa4dcbc58c9782cec,
    0xf32b7cd402065738,
    0xa354d244fb69ee68,
    0x6ff8e0ac88bb2224,
    0xa43fce7a210f791c,
    0xbc95eafd3a51d21a,
    0x4610dda3a2fc58d0,
    0x99e151657aad0ed2,
    0x65374ace081a2344,
    0x29675593f838d492,
    0xddd8ca209e3a92d0,
    0xde2d18e1012c1c6a,
    0xd36e489ae145f8b8,
    0x6b86aeb8d0351710,
    0x1ffc3f013654d7de,
    0xda0974e4c14ce2b2,
    0x55c862b94a5eb6fa,
    0xee39a9ba52540bf6,
    0xc38b71c565063662,
];
