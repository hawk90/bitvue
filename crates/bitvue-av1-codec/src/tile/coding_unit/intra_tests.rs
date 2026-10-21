//! `read_intra_mode_info` pinned on its outward effects (result, coding unit, neighbour
//! contexts, bits consumed) over seeded sweeps of random streams and frame/block settings, plus
//! the one effect a sweep cannot isolate: a block that cannot carry a palette clears the palette
//! sizes it covers.

use super::*;
use crate::tile::FrameCodingParams;

fn noise(seed: u64, len: usize) -> Vec<u8> {
    let mut x = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1;
    (0..len)
        .map(|_| {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            (x >> 24) as u8
        })
        .collect()
}

fn fnv(h: &mut u64, bytes: &[u8]) {
    for b in bytes {
        *h ^= *b as u64;
        *h = h.wrapping_mul(0x100_0000_01b3);
    }
}

const SIZES: [(u32, u32); 18] = [
    (4, 4),
    (4, 8),
    (8, 4),
    (8, 8),
    (8, 16),
    (16, 8),
    (16, 16),
    (16, 32),
    (32, 16),
    (32, 32),
    (32, 64),
    (64, 32),
    (64, 64),
    (4, 16),
    (16, 4),
    (8, 32),
    (32, 8),
    (64, 16),
];

struct Setup {
    frame: FrameCodingParams,
    rect: BlockRect,
    mi: MiRect,
}

fn setup(r: u64) -> Setup {
    let (width, height) = SIZES[(r % SIZES.len() as u64) as usize];
    let mut frame = FrameCodingParams::for_tests();
    frame.is_key_frame = r >> 8 & 1 == 1;
    frame.allow_screen_content_tools = r >> 9 & 3 != 0;
    frame.enable_filter_intra = r >> 10 & 1 == 1;
    frame.tx_type_flags.coded_lossless = r >> 11 & 7 == 0;
    frame.tx_type_flags.mono_chrome = r >> 14 & 7 == 0;
    frame.tx_type_flags.subsampling_x = r >> 17 & 1 == 1;
    frame.tx_type_flags.subsampling_y = r >> 18 & 1 == 1;
    frame.mi_rows = 32;
    frame.mi_cols = 32;
    let x4 = ((r >> 20) as u32 % (32 / (width / 4))) * (width / 4);
    let y4 = ((r >> 28) as u32 % (32 / (height / 4))) * (height / 4);
    Setup {
        frame,
        rect: BlockRect {
            x: x4 * 4,
            y: y4 * 4,
            width,
            height,
        },
        mi: MiRect {
            x4,
            y4,
            width: width / 4,
            height: height / 4,
        },
    }
}

fn run(data: &[u8], s: &Setup, tile_ctx: &mut TileContext) -> (Result<u8>, CodingUnit, u32) {
    let mut decoder = SymbolDecoder::new(data).unwrap();
    let mut cu = CodingUnit::new(s.rect.x, s.rect.y, s.rect.width, s.rect.height);
    let out = read_intra_mode_info(&mut decoder, tile_ctx, s.rect, s.mi, &s.frame, &mut cu);
    let next = decoder.read_bools_n(24).unwrap_or(u32::MAX);
    (out, cu, next)
}

fn sweep(seed: u64, n: usize) -> u64 {
    let mut h = 0xcbf2_9ce4_8422_2325u64;
    let rolls = noise(seed, n * 8);
    for i in 0..n {
        let r = u64::from_le_bytes(rolls[i * 8..i * 8 + 8].try_into().unwrap());
        let s = setup(r);
        let data = noise(seed ^ ((i as u64 + 1) * 0x51ED), 96);
        let mut tile_ctx = TileContext::new(64, 64);
        let (out, cu, next) = run(&data, &s, &mut tile_ctx);
        match out {
            Ok(m) => fnv(&mut h, &[1, m]),
            Err(_) => fnv(&mut h, &[0]),
        }
        fnv(&mut h, format!("{:?}{:?}", cu.mode, cu.palette).as_bytes());
        fnv(&mut h, format!("{:?}", cu.tx_size).as_bytes());
        fnv(&mut h, &next.to_le_bytes());
        let (x4, y4) = (s.mi.x4, s.mi.y4);
        let (a, l) = tile_ctx.intra_mode_context(x4 + s.mi.width, y4 + s.mi.height);
        fnv(&mut h, &[a, l, tile_ctx.has_palette_y_context(x4, y4)]);
        for plane in 0..2 {
            fnv(&mut h, &[tile_ctx.pal_above(plane, 0, x4).1]);
            fnv(&mut h, &[tile_ctx.pal_left(plane, 0, y4).1]);
        }
        fnv(&mut h, &[tile_ctx.tx_size_context(x4, y4, 3, 3)]);
    }
    h
}

#[test]
fn seeded_sweeps_are_pinned() {
    assert_eq!(sweep(0x5eed, 3000), 759801822928652269);
    assert_eq!(sweep(0xfeed, 3000), 13632656610397451821);
}

#[test]
fn a_block_that_cannot_have_a_palette_clears_the_palette_sizes_it_covers() {
    // 4x4 can never carry a palette, whatever the screen-content flag says.
    let mut s = setup(0);
    s.frame.allow_screen_content_tools = true;
    s.rect = BlockRect {
        x: 8,
        y: 8,
        width: 4,
        height: 4,
    };
    s.mi = MiRect {
        x4: 2,
        y4: 2,
        width: 1,
        height: 1,
    };
    let mut tile_ctx = TileContext::new(64, 64);
    tile_ctx.set_pal_size(0, 0, 0, 16, 16, 5);
    tile_ctx.set_pal_size(1, 0, 0, 16, 16, 5);
    let _ = run(&noise(7, 64), &s, &mut tile_ctx);
    for plane in 0..2 {
        assert_eq!(tile_ctx.pal_above(plane, 0, 2).1, 0);
        assert_eq!(tile_ctx.pal_left(plane, 0, 2).1, 0);
        // Outside the block the earlier sizes survive.
        assert_eq!(tile_ctx.pal_above(plane, 0, 3).1, 5);
    }
}

#[test]
fn screen_content_off_never_reads_a_palette() {
    let mut s = setup(0);
    s.frame.allow_screen_content_tools = false;
    for seed in 0..40u64 {
        let mut tile_ctx = TileContext::new(64, 64);
        let (_, cu, _) = run(&noise(seed, 64), &s, &mut tile_ctx);
        assert_eq!((cu.palette.y_size, cu.palette.uv_size), (0, 0));
    }
}

#[test]
fn the_sweep_reaches_every_branch_of_the_function() {
    let (mut ok, mut pal_y, mut pal_uv, mut filter, mut cfl_or_chroma) = (0, 0, 0, 0, 0);
    let rolls = noise(0x5eed, 3000 * 8);
    for i in 0..3000 {
        let r = u64::from_le_bytes(rolls[i * 8..i * 8 + 8].try_into().unwrap());
        let s = setup(r);
        let data = noise(0x5eed ^ ((i as u64 + 1) * 0x51ED), 96);
        let (out, cu, _) = run(&data, &s, &mut TileContext::new(64, 64));
        if let Ok(tx_mode) = out {
            {
                ok += 1;
                if cu.palette.y_size > 0 {
                    pal_y += 1;
                }
                if cu.palette.uv_size > 0 {
                    pal_uv += 1;
                }
                if cu.mode == PredictionMode::DcPred && tx_mode != 0 {
                    filter += 1;
                }
                if cu.mode != PredictionMode::DcPred {
                    cfl_or_chroma += 1;
                }
            }
        }
    }
    assert!(ok > 0 && pal_y > 0 && pal_uv > 0 && filter > 0 && cfl_or_chroma > 0);
}
