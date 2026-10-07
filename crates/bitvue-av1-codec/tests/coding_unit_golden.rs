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
    0x059ca65e6ae97f99,
    0x94dda5dbb31c9ce1,
    0xa71caf72e4dbf7e0,
    0xa7b18bcb6412df80,
    0x093376fabfa978f3,
    0xed1b2a2d30d9929f,
    0x16beecc46304875c,
    0xfd2109efb474a83c,
    0xf33b163050befce9,
    0xf3b77d4a2e77dc2f,
    0xdb6822184c80aa16,
    0xa3eea21d92a3f538,
    0x1991a12d38598c06,
    0x7c5c28dd7d1ed605,
    0xe0474a99ac50ce3d,
    0x8e06324aef6c13fe,
    0xeb1cf24349430918,
    0x466f3881667d66e7,
    0x5d8128124698cdf5,
    0x34220c558a32f32e,
    0xd10010a6ec4c8b61,
    0xd580f2af75c6c9b0,
    0x575f51bb2af30ec6,
    0xfa2508b04cafb2a2,
    0xd111f86e0a3b6f30,
];
const GOLDEN_TOTAL_CUS: usize = 2398;

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
    0x306c24045411b825,
    0xd842cb61368c25b7,
    0xf2630c85334091ae,
    0xc7774fd4e07f382b,
    0x86a8940d90a3e56a,
    0x88080ecd2fe13396,
    0x97fd0087e106374b,
    0x614ebfcc58066361,
    0x9388b0eeb3c60595,
    0x8bade4d7e6fb77c3,
    0xe7b2a1b9fcf82d4d,
    0x21ef984d5998c3ff,
    0x36219308fb5d06da,
    0xfcc88d7a285097a1,
    0x818322b01141df4f,
    0xcccd9c2be9948611,
    0x479efbeda8bb58c4,
    0xddf61b6f41760548,
    0xeef0905bf13c3388,
    0x48fb2ff9ee55320b,
    0xab2066050402ffaf,
    0x495c687493f83d47,
    0x7973f977374b93e5,
    0xdd842c15cb577d0e,
    0xb15bd57fbacbbaed,
];
const GOLDEN_PREDICTION_MODE: &[u64] = &[
    0x34610affea849b4c,
    0x80a29f96b188e09e,
    0x4076632f5084cdbf,
    0x2723843294d21dbc,
    0x4ee7a37fc0f6939a,
    0xa1fe03991ecaace3,
    0x6071687ab909f427,
    0x346ad8b3f94b5f85,
    0x298f6387fca8ecf2,
    0x7bc2bbc9a980f9d4,
    0x5db610babc52c908,
    0x4310ca4a783519b4,
    0x9d9af2f6e74fd521,
    0x95558608a368505a,
    0x369bb109f55d09f0,
    0x42605aaff0b8e8ca,
    0x4abe318b155c3208,
    0x9153c40a16f8b98a,
    0x36d6fa755e097730,
    0x7d90a10cc1f79e75,
    0xa4232a12e7beebda,
    0x74a6979038d2ccfe,
    0x1df1f2b7ba797251,
    0x116802244cb0d070,
    0x73d7e9cb6d4c24c6,
];
const GOLDEN_TRANSFORM: &[u64] = &[
    0x880dfe08afefef08,
    0xc143aa37d5e38d06,
    0x374058f171f3ef7a,
    0x104144271fb68e6c,
    0x674c4da64ef333cc,
    0xb07a11267e740152,
    0xd80f4baae466a77a,
    0x313535042da93546,
    0xc7e8709a028315bc,
    0x17b5fc1c5ca220a2,
    0x2c8f1d54586055ec,
    0x1c4d17d53716863a,
    0x17e091bfd79ac42e,
    0xd9a73ed6d9cfdc22,
    0x3aa462a82c13f4f4,
    0xbc09465cde772f0e,
    0xb9017cb3017b1124,
    0x77e6b60237e33f4a,
    0xdfe441366591599c,
    0xa655ed3d47310f02,
    0xdab843a2f3dcbd90,
    0x11e95728d881a844,
    0xec2d14b782704be6,
    0x4576317746832a8a,
    0x1836bd28f91055ca,
];
