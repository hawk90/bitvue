//! Characterization of the six grid extractors (QP, MV, energy, bit, prediction mode, transform):
//! the complete `Debug` text of every result (grid, or the error message) is folded into one
//! pinned FNV-1a digest + length per extractor, over (a) every frame of several real clips and
//! (b) synthetic frames covering the failure paths (no tile bytes, zero/huge/overflowing sizes,
//! unparseable tile bytes, empty / out-of-frame / zero-sized / frame-straddling coding units).
//!
//! The digests were recorded BEFORE the extractors' shared prologue/fill code was refactored (#557)
//! and must not change with it. To print the current values run with `GOLDEN_PRINT=1`.

use super::*;
use crate::frame_header_full::RefFrameState;
use crate::obu::{ObuIterator, ObuType};
use crate::symbol::ResidualBlockStats;
use crate::tile::{CodingUnit, MotionVector, PredictionMode, RefFrame, TxSize};

const KINDS: [&str; 6] = ["qp", "mv", "energy", "bit", "pred", "tx"];

const CLIPS: [(&str, &[u8]); 10] = [
    (
        "av1_test",
        include_bytes!("../../../../test_data/av1_test.ivf"),
    ),
    (
        "tiles5",
        include_bytes!("../../../../test_data/av1_aomenc_tiles5.ivf"),
    ),
    (
        "testsrc2",
        include_bytes!("../../../../test_data/av1_aomenc_testsrc2.ivf"),
    ),
    (
        "globalmotion",
        include_bytes!("../../../../test_data/av1_aomenc_globalmotion.ivf"),
    ),
    (
        "lossless_tiled",
        include_bytes!("../../../../test_data/av1_aomenc_lossless_tiled.ivf"),
    ),
    (
        "intrabc_glyphs",
        include_bytes!("../../../../test_data/av1_aomenc_intrabc_glyphs.ivf"),
    ),
    (
        "odd_194x72",
        include_bytes!("../../../../test_data/av1_aomenc_194x72.ivf"),
    ),
    (
        "palette_114x136",
        include_bytes!("../../../../test_data/av1_aomenc_114x136_palette.ivf"),
    ),
    (
        "superres",
        include_bytes!("../../../../test_data/av1_aomenc_superres.ivf"),
    ),
    (
        "svtav1_testsrc2",
        include_bytes!("../../../../test_data/av1_svtav1_testsrc2.ivf"),
    ),
];

const MAX_FRAMES_PER_CLIP: usize = 12;

struct Fnv {
    hash: u64,
    len: usize,
}

impl Fnv {
    fn new() -> Self {
        Fnv {
            hash: 0xcbf2_9ce4_8422_2325,
            len: 0,
        }
    }
    fn feed(&mut self, text: &str) {
        for b in text.bytes() {
            self.hash ^= u64::from(b);
            self.hash = self.hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
        self.len += text.len();
    }
}

fn err_text<T: std::fmt::Debug>(r: Result<T, bitvue_engine::BitvueError>) -> String {
    match r {
        Ok(v) => format!("Ok({v:?})"),
        Err(e) => format!("Err({e})"),
    }
}

/// The Debug text of what each extractor returns for `parsed`, in `KINDS` order.
fn all_grids(parsed: &ParsedFrame) -> [String; 6] {
    [
        err_text(extract_qp_grid_from_parsed(parsed, 0, 32)),
        err_text(extract_mv_grid_from_parsed(parsed)),
        err_text(extract_energy_grid_from_parsed(parsed)),
        err_text(extract_bit_grid_from_parsed(parsed)),
        err_text(extract_prediction_mode_grid_from_parsed(parsed)),
        err_text(extract_transform_grid_from_parsed(parsed)),
    ]
}

fn fold(acc: &mut [Fnv; 6], label: &str, texts: &[String; 6]) {
    for (a, t) in acc.iter_mut().zip(texts) {
        a.feed(label);
        a.feed(t);
    }
}

fn count_errs(counts: &mut [usize; 6], texts: &[String; 6]) {
    for (c, t) in counts.iter_mut().zip(texts) {
        if t.starts_with("Err(") {
            *c += 1;
        }
    }
}

fn printing() -> bool {
    std::env::var_os("GOLDEN_PRINT").is_some()
}

fn clip_frames(ivf: &[u8]) -> Vec<ParsedFrame> {
    let (_h, packets) = crate::parse_ivf_frames(ivf).unwrap();
    let seq = packets
        .iter()
        .find_map(|p| {
            let mut obus = ObuIterator::new(&p.data);
            while let Some(Ok(f)) = obus.next_obu_with_offset() {
                if f.obu.header.obu_type == ObuType::SequenceHeader {
                    return Some(p.data[f.offset..f.offset + f.consumed].to_vec());
                }
            }
            None
        })
        .unwrap();
    let mut state = RefFrameState::new();
    packets
        .iter()
        .take(MAX_FRAMES_PER_CLIP)
        .map(|p| {
            let bytes = [seq.as_slice(), p.data.as_slice()].concat();
            ParsedFrame::parse_with_ref_state(&bytes, &mut state).unwrap()
        })
        .collect()
}

fn check(name: &str, acc: &[Fnv; 6], golden: [(u64, usize); 6]) {
    if printing() {
        for (k, a) in KINDS.iter().zip(acc) {
            println!("GOLDEN {name} {k} ({}, {})", a.hash, a.len);
        }
        return;
    }
    for ((k, a), g) in KINDS.iter().zip(acc).zip(golden) {
        assert_eq!((a.hash, a.len), g, "{name}: {k} grid changed");
    }
}

#[test]
fn real_clips_give_the_same_grids() {
    let mut acc: [Fnv; 6] = std::array::from_fn(|_| Fnv::new());
    let mut err_counts = [0usize; 6];
    let mut total = 0usize;
    for (name, ivf) in CLIPS {
        for (i, parsed) in clip_frames(ivf).iter().enumerate() {
            let texts = all_grids(parsed);
            total += 1;
            count_errs(&mut err_counts, &texts);
            fold(&mut acc, &format!("{name}#{i}:"), &texts);
        }
    }
    if printing() {
        println!("GOLDEN real total {total} err_counts {err_counts:?}");
    } else {
        assert_eq!(total, REAL_TOTAL);
        assert_eq!(err_counts, REAL_ERR, "frames that produce an error");
    }
    check("real", &acc, REAL_GOLDEN);
}

/// A frame of `width` x `height` with `tile_len` tile bytes; `units` is its cached decode (none:
/// the extractor parses the tile bytes itself).
fn frame_with(
    width: u32,
    height: u32,
    tile_len: usize,
    units: Option<Vec<CodingUnit>>,
) -> ParsedFrame {
    let mut parsed = ParsedFrame::parse(&[]).unwrap();
    parsed.dimensions.width = width;
    parsed.dimensions.height = height;
    parsed.tile_data = std::sync::Arc::from(vec![0u8; tile_len]);
    parsed.decoded = units.map(|units| ParsedCodingUnits {
        units: std::sync::Arc::new(units),
        outcome: Default::default(),
    });
    parsed
}

/// Deterministic, varied coding units: sizes 0..=128, some straddling the frame edge, some wholly
/// outside it, with every field a grid extractor reads varied.
fn synthetic_units(width: u32, height: u32, count: usize) -> Vec<CodingUnit> {
    let mut s = 0x9e37_79b9_7f4a_7c15u64 ^ (u64::from(width) << 20) ^ u64::from(height);
    let mut next = move || {
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        s
    };
    let modes = [
        PredictionMode::DcPred,
        PredictionMode::VPred,
        PredictionMode::HPred,
        PredictionMode::D45Pred,
        PredictionMode::D135Pred,
    ];
    let txs = [
        TxSize::Tx4x4,
        TxSize::Tx8x8,
        TxSize::Tx16x16,
        TxSize::Tx32x32,
        TxSize::Tx64x64,
    ];
    let refs = [
        RefFrame::Intra,
        RefFrame::Last,
        RefFrame::Golden,
        RefFrame::BwdRef,
    ];
    (0..count)
        .map(|_| {
            let r = next();
            let size = [0u32, 4, 8, 16, 32, 64, 128][(r % 7) as usize];
            let x = ((next() % u64::from(width + 80)) as u32) & !3;
            let y = ((next() % u64::from(height + 80)) as u32) & !3;
            let r = next();
            let mut cu = CodingUnit::new(x, y, size, [size, size / 2, size][(r % 3) as usize]);
            cu.mode = modes[(r >> 3) as usize % modes.len()];
            cu.tx_size = txs[(r >> 6) as usize % txs.len()];
            cu.skip = (r >> 9) & 1 == 1;
            cu.use_intrabc = (r >> 10) % 11 == 0;
            cu.ref_frames = [refs[(r >> 12) as usize % 4], refs[(r >> 14) as usize % 4]];
            cu.mv = [
                MotionVector::new((r >> 16) as i32 % 777 - 300, (r >> 26) as i32 % 91 - 40),
                MotionVector::new(
                    i32::MAX - (r >> 40) as i32 % 3,
                    i32::MIN + (r >> 44) as i32 % 3,
                ),
            ];
            cu.qp = ((r >> 20) % 3 == 0).then_some((r >> 22) as i16 % 255);
            cu.residual = ((r >> 30) % 4 != 0).then(|| ResidualBlockStats {
                all_zero: false,
                nonzero_count: 1,
                sum_abs_level: (r >> 32) % 100_000,
                max_level: 1,
                dc_sign_value: None,
            });
            cu.bits = ((r >> 8) % 5000) as f64 * 0.37;
            cu.symbols = ((r >> 18) % 900) as u32;
            cu.partition_bits = ((r >> 28) % 40) as f64 * 0.5;
            cu.partition_symbols = ((r >> 38) % 20) as f64;
            cu
        })
        .collect()
}

#[test]
fn synthetic_frames_give_the_same_grids_and_errors() {
    let dims = [
        (0, 0),
        (0, 40),
        (40, 0),
        (1, 1),
        (15, 17),
        (16, 16),
        (63, 63),
        (64, 64),
        (65, 65),
        (130, 65),
        (200, 100),
        (1920, 1080),
        (8192, 4320),
        (16000, 16000),
        (32768, 32768),
        (3_000_000, 1),
        (1, 3_000_000),
        (u32::MAX, u32::MAX),
    ];
    let mut acc: [Fnv; 6] = std::array::from_fn(|_| Fnv::new());
    let mut err_counts = [0usize; 6];
    let mut total = 0usize;
    for (w, h) in dims {
        // Units only where the frame is small enough to hold them (the extractors allocate grids).
        let small = u64::from(w) * u64::from(h) <= 2_000_000;
        for tile_len in [0usize, 1, 64] {
            for variant in 0..3 {
                let units = match variant {
                    0 => Some(Vec::new()),
                    1 if small => Some(synthetic_units(w, h, 40)),
                    2 if small => Some(synthetic_units(w ^ 1, h, 3)),
                    _ => continue,
                };
                let texts = all_grids(&frame_with(w, h, tile_len, units));
                total += 1;
                count_errs(&mut err_counts, &texts);
                fold(
                    &mut acc,
                    &format!("{w}x{h}/t{tile_len}/v{variant}:"),
                    &texts,
                );
            }
        }
    }
    // Tile bytes the extractor has to parse itself (no cached units), garbage included.
    // (7000, 6000) is over the decode limit: the tile is present but its parse fails.
    for (w, h) in [
        (16, 16),
        (64, 64),
        (130, 65),
        (1920, 1080),
        (33, 7),
        (7000, 6000),
    ] {
        for fill in [0x00u8, 0xFF, 0x5A] {
            for tile_len in [1usize, 5, 40, 400] {
                let mut parsed = frame_with(w, h, 0, None);
                parsed.tile_data = std::sync::Arc::from(vec![fill; tile_len]);
                let texts = all_grids(&parsed);
                total += 1;
                count_errs(&mut err_counts, &texts);
                fold(
                    &mut acc,
                    &format!("raw{w}x{h}/f{fill}/t{tile_len}:"),
                    &texts,
                );
            }
        }
    }
    // Out-of-range base QP is rejected before anything else, whatever the frame.
    let parsed = frame_with(64, 64, 8, Some(Vec::new()));
    for qp in [-1i16, 256, i16::MAX, i16::MIN] {
        assert!(extract_qp_grid_from_parsed(&parsed, 0, qp).is_err());
    }
    if printing() {
        println!("GOLDEN synthetic total {total} err_counts {err_counts:?}");
    } else {
        assert_eq!(total, SYNTH_TOTAL);
        assert_eq!(err_counts, SYNTH_ERR, "scenarios that produce an error");
    }
    check("synthetic", &acc, SYNTH_GOLDEN);
}

// Recorded with `GOLDEN_PRINT=1` on the code before the #557 refactor.
// Order: qp, mv, energy, bit, prediction mode, transform.
const REAL_GOLDEN: [(u64, usize); 6] = [
    (2169891902965230871, 16061),
    (9462468701204883848, 184123),
    (5844071075137566189, 19914),
    (5481267514857490576, 49682),
    (555716188603119127, 240749),
    (13465840930249902657, 228365),
];
const REAL_TOTAL: usize = 89;
// Frames of the clips without decodable tile data: qp/mv/pred/tx error, energy/bit give zero grids.
const REAL_ERR: [usize; 6] = [15, 15, 0, 0, 15, 15];
const SYNTH_GOLDEN: [(u64, usize); 6] = [
    (2003557981491682935, 3468783),
    (17170702697539259885, 106847907),
    (10232343661937863865, 7097220),
    (15616482689352703705, 14214236),
    (777749414567615704, 7103305),
    (14333610733932936203, 7131009),
];
const SYNTH_TOTAL: usize = 192;
// energy/bit only error on cell-count overflow; the others also on missing tile bytes, an
// unparseable tile, and (prediction mode/transform) more than MAX_GRID_BLOCKS cells.
const SYNTH_ERR: [usize; 6] = [54, 54, 3, 3, 58, 58];
