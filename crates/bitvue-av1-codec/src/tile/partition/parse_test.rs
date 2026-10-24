//! `parse_partition_recursive` pinned over seeded sweeps of random streams, root block sizes and
//! frame extents: the resulting tree (or error), the leaves handed to the visitor in order, the
//! partition context the walk leaves behind and the bits it consumed. The error paths and the
//! frame-edge cases have their own tests in `mod.rs` (noise never reaches an error from a root at
//! the frame origin, so the sweep pins only successful walks).

use super::*;
use crate::symbol::SymbolDecoder;
use crate::tile::{MvPredictorContext, TileContext};

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

const ROOTS: [BlockSize; 5] = [
    BlockSize::Block8x8,
    BlockSize::Block16x16,
    BlockSize::Block32x32,
    BlockSize::Block64x64,
    BlockSize::Block128x128,
];

struct Walk<'a> {
    result: Result<Option<PartitionNode>>,
    leaves: Vec<(u32, u32, BlockSize, PartitionType)>,
    state: TileState<'a>,
}

fn walk(data: &[u8], root: BlockSize, (mi_rows, mi_cols): (u32, u32)) -> Walk<'_> {
    let mut state = TileState {
        decoder: SymbolDecoder::new(data).unwrap(),
        mv_ctx: MvPredictorContext::new(1, 1),
        tile_ctx: TileContext::new(64, 64),
    };
    let mut leaves = Vec::new();
    let result = parse_partition_recursive(
        &mut state,
        0,
        0,
        root,
        mi_rows,
        mi_cols,
        0,
        &mut |_, node| {
            leaves.push((node.x, node.y, node.size, node.partition));
            Ok(())
        },
    );
    Walk {
        result,
        leaves,
        state,
    }
}

fn sweep(seed: u64, n: usize) -> u64 {
    let mut h = 0xcbf2_9ce4_8422_2325u64;
    let rolls = noise(seed, n * 4);
    for i in 0..n {
        let root = ROOTS[(rolls[i * 4] % 5) as usize];
        let rows = 1 + (rolls[i * 4 + 1] as u32 % 40);
        let cols = 1 + (rolls[i * 4 + 2] as u32 % 40);
        let data = noise(seed ^ ((i as u64 + 1) * 0x51ED), 128);
        let mut w = walk(&data, root, (rows, cols));
        match &w.result {
            Ok(Some(node)) => fnv(&mut h, format!("S{node:?}").as_bytes()),
            Ok(None) => fnv(&mut h, b"N"),
            Err(e) => fnv(&mut h, format!("E{e:?}").as_bytes()),
        }
        fnv(&mut h, format!("{:?}", w.leaves).as_bytes());
        for x8 in 0..16 {
            for bl in 0..4 {
                fnv(
                    &mut h,
                    &[w.state.tile_ctx.partition_context(x8, x8 / 2, bl)],
                );
            }
        }
        fnv(
            &mut h,
            &w.state.decoder.read_bools_n(24).unwrap().to_le_bytes(),
        );
    }
    h
}

#[test]
fn seeded_sweeps_are_pinned() {
    assert_eq!(sweep(0x5eed, 1500), 5373345362319663676);
    assert_eq!(sweep(0xfeed, 1500), 8189990971748291970);
}

#[test]
fn the_sweep_reaches_trees_edges_and_errors() {
    let (mut split, mut terminal, mut clipped) = (0, 0, 0);
    let rolls = noise(0x5eed, 1500 * 4);
    for i in 0..1500 {
        let root = ROOTS[(rolls[i * 4] % 5) as usize];
        let rows = 1 + (rolls[i * 4 + 1] as u32 % 40);
        let cols = 1 + (rolls[i * 4 + 2] as u32 % 40);
        let data = noise(0x5eed ^ ((i as u64 + 1) * 0x51ED), 128);
        let w = walk(&data, root, (rows, cols));
        if let Ok(Some(node)) = &w.result {
            {
                if node.partition == PartitionType::Split {
                    split += 1;
                } else if node.partition != PartitionType::None {
                    terminal += 1;
                }
                if node.children.len() < 4 && node.partition == PartitionType::Split {
                    clipped += 1;
                }
            }
        }
    }
    assert!(
        split > 0 && terminal > 0 && clipped > 0,
        "{split} {terminal} {clipped}"
    );
}

#[test]
fn a_split_just_below_the_depth_limit_fails_in_its_children() {
    // The children of a split get `depth + 1`, so a split at `MAX - 1` must fail when the first
    // child is checked, while every non-split node there parses normally.
    let (mut failed_children, mut parsed) = (0, 0);
    for seed in 0..60u64 {
        let data = noise(seed, 64);
        let mut state = TileState {
            decoder: SymbolDecoder::new(&data).unwrap(),
            mv_ctx: MvPredictorContext::new(1, 1),
            tile_ctx: TileContext::new(64, 64),
        };
        let result = parse_partition_recursive(
            &mut state,
            0,
            0,
            BlockSize::Block64x64,
            16,
            16,
            MAX_PARTITION_DEPTH - 1,
            &mut |_, _| Ok(()),
        );
        match result {
            Ok(Some(node)) => {
                assert_ne!(node.partition, PartitionType::Split);
                parsed += 1;
            }
            Err(BitvueError::InvalidData(m)) if m.contains("depth exceeded") => {
                failed_children += 1
            }
            other => panic!("unexpected {other:?}"),
        }
    }
    assert!(
        failed_children > 0 && parsed > 0,
        "{failed_children} {parsed}"
    );
}
