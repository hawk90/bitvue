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
    0xb14a03a4fd5e8c77,
    0x1bc133f231f27c9f,
    0xd05fb754795f281a,
    0x38d2f88a55de03c7,
    0xe1eecc7d316eaf45,
    0x44e801f2ebf83424,
    0x65a0433fb46c998d,
    0x3ac9b55aef6012d1,
    0x44c949bc2ae6e55d,
    0x70003f3623c6210c,
    0x2813ca66f22d3fc0,
    0x36258f37e9285ec3,
    0x01b6b40278ade9c1,
    0x5a0a8fc237c318e8,
    0x92ab7e53a05bd4e5,
    0x22f5a0fb526534dc,
    0xab14a375b54c5de5,
    0x9803bc341cf85b51,
    0x04130779d24816da,
    0x24017e1bb791f250,
    0x4603788f2f84ac51,
    0x4cf9a2abdf8f9fc1,
    0xd78fbac697ee4633,
    0x1088d4cdd895fe73,
    0x82dc2b8a78e3d549,
];
const GOLDEN_TOTAL_CUS: usize = 11390;

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
    0xc856fd7316261840,
    0xbd124ce27deb62e7,
    0x4ef17399a21422b4,
    0xbafe5db14a24db9d,
    0x17cac2f4ee2726bc,
    0x6470fca10299fbef,
    0x801c54f8893a0215,
    0x1a810d24e58c0702,
    0x334739b278b1fe1a,
    0x178ce05b90f6cc68,
    0xa1647e9871d22b98,
    0x117e26ffc865b529,
    0xe598aa0c24c24272,
    0x34dffe5812f1dfdf,
    0x2604cf90dc49d62d,
    0x1fcf5cf6ca75be6c,
    0xb275ad528a70295f,
    0xfcd60503bd4cf3f7,
    0xf661ca8a64b019c4,
    0x4a24de52784fad6c,
    0x2d795fc96299849a,
    0x5c9ca56477bb7b62,
    0xbda58ef887cdceef,
    0x4c4725387b1b33c2,
    0x0d69090a069775f0,
];
const GOLDEN_PREDICTION_MODE: &[u64] = &[
    0x4bfd1d3ab490c6ee,
    0xd9652785c332b0a1,
    0x9919bb5397d8c1ae,
    0x2ee896363389e14d,
    0x85de53595681c078,
    0x628c1f51bc30f57f,
    0x1089b6bb3e175836,
    0xaf16a293d7391ddd,
    0x40dd0bea368b8de0,
    0x362a927b48058e32,
    0xd99d49aa952013ed,
    0x4db023d3fe736c6e,
    0x3c3474da8f0d52e8,
    0x2dfacbd30efdce1c,
    0x1e05f198db4b0c1f,
    0x7ab1b63e409c7212,
    0x31076607b4a337c6,
    0xce2bf43396a09777,
    0x357916ed5852ecd0,
    0x54bcf476fce5b5fa,
    0xa3938b43b8842515,
    0x22eb6ce9a44b177d,
    0x70c95bef94960447,
    0x46c8940e53112327,
    0xe6029e0b34bfc8b5,
];
const GOLDEN_TRANSFORM: &[u64] = &[
    0xd2f83fa6a27167f8,
    0xa290d297136df37e,
    0x9f0b22d76228a282,
    0x88aaaaf265255be2,
    0x5bcac6c0cbc010f0,
    0xd4a4219f55ab7078,
    0x17c34ba62e5ddecc,
    0x64d141ea55907036,
    0x275dda6bbd661232,
    0x096116a13b68e55e,
    0xd2dbf20fd73a32e0,
    0xd5baafc341584ca2,
    0xac0cd7683fa36e9c,
    0xb11fd8447729bc56,
    0xaf648c11406d4b6c,
    0xb762968dca272acc,
    0x3be1aacefef8b9de,
    0x833b8761c70066b8,
    0xf78cff816bbd9bc6,
    0x8648e0ad91939532,
    0x5c9049cf14fdff66,
    0xa0eb555f218df11a,
    0x164ae55cc8cd6310,
    0xd1e322c2615fe1d0,
    0xe3f4c4a20dd3b96c,
];
