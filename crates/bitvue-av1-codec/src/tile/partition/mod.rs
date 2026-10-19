//! Partition Tree Parsing
//!
//! Per AV1 Specification Section 5.11.4 (Decode Partition)
//!
//! AV1 uses a recursive partition structure:
//! - Superblock (128x128 or 64x64) is the top level
//! - Each block can be partitioned into smaller blocks
//! - Minimum block size is 4x4
//!
//! ## Partition Types
//!
//! ```text
//! PARTITION_NONE:      PARTITION_HORZ:     PARTITION_VERT:
//! ┌───────────┐        ┌───────────┐        ┌─────┬─────┐
//! │           │        ├───────────┤        │     │     │
//! │           │        │           │        │     │     │
//! └───────────┘        └───────────┘        └─────┴─────┘
//!
//! PARTITION_SPLIT:     PARTITION_HORZ_A:   PARTITION_HORZ_B:
//! ┌─────┬─────┐        ┌───────────┐        ┌─────┬─────┐
//! ├─────┼─────┤        ├─────┬─────┤        ├───────────┤
//! │     │     │        │     │     │        │           │
//! └─────┴─────┘        └─────┴─────┘        └───────────┘
//!
//! PARTITION_VERT_A:    PARTITION_VERT_B:   PARTITION_HORZ_4:
//! ┌─────┬─────┐        ┌───────────┐        ┌───────────┐
//! │     ├─────┤        ├─────┬─────┐        ├───────────┤
//! │     │     │        │     │     │        ├───────────┤
//! └─────┴─────┘        └─────┴─────┘        ├───────────┤
//!                                            └───────────┘
//!
//! PARTITION_VERT_4:
//! ┌──┬──┬──┬──┐
//! │  │  │  │  │
//! │  │  │  │  │
//! └──┴──┴──┴──┘
//! ```

mod parse;
mod tree;
mod types;

pub(crate) use parse::parse_partition_recursive;
pub use parse::{parse_partition_tree, partition_tree_to_grid};
#[cfg(test)]
use tree::child_position;
pub(crate) use tree::mi_units;
pub use tree::PartitionNode;
pub use types::{BlockSize, PartitionType};

#[cfg(test)]
mod tests {
    /// Block layout of the four "A/B" partitions, as AV1 spec 5.11.4 `decode_partition` decodes
    /// them (the order matters: each block's symbols follow its predecessor's). Cross-checked
    /// against dav1d, whose `T_*_SPLIT` names say which half is split:
    ///
    /// - `HORZ_A` (`T_TOP_SPLIT`):  TL square, TR square, then the full-width bottom block
    /// - `HORZ_B` (`T_BOTTOM_SPLIT`): full-width top block, then BL, BR squares
    /// - `VERT_A` (`T_LEFT_SPLIT`):  TL square, BL square, then the full-height right block
    /// - `VERT_B` (`T_RIGHT_SPLIT`): full-height left block, then TR, BR squares
    ///
    /// Confirmed on a real stream: the dav1d oracle decodes the first block of a `bp=7` (`VERT_B`)
    /// 16x16 partition as an 8x16 block.
    #[test]
    fn a_b_partitions_decode_blocks_in_spec_order() {
        use BlockSize::*;
        // (partition, expected [(size, x, y); 3]) for a 16x16 parent at the origin.
        let cases = [
            (
                PartitionType::HorzA,
                [(Block8x8, 0, 0), (Block8x8, 8, 0), (Block16x8, 0, 8)],
            ),
            (
                PartitionType::HorzB,
                [(Block16x8, 0, 0), (Block8x8, 0, 8), (Block8x8, 8, 8)],
            ),
            (
                PartitionType::VertA,
                [(Block8x8, 0, 0), (Block8x8, 0, 8), (Block8x16, 8, 0)],
            ),
            (
                PartitionType::VertB,
                [(Block8x16, 0, 0), (Block8x8, 8, 0), (Block8x8, 8, 8)],
            ),
        ];
        for (partition, expected) in cases {
            let sizes = Block16x16.sub_block_size(partition);
            let got: Vec<_> = sizes
                .iter()
                .enumerate()
                .map(|(i, &size)| {
                    let (x, y) = child_position(0, 0, i, partition, Block16x16);
                    (size, x, y)
                })
                .collect();
            assert_eq!(got, expected.to_vec(), "{partition:?}");
        }
    }

    use super::*;

    #[test]
    fn test_partition_type_sub_block_count() {
        assert_eq!(PartitionType::None.sub_block_count(), 1);
        assert_eq!(PartitionType::Horz.sub_block_count(), 2);
        assert_eq!(PartitionType::Vert.sub_block_count(), 2);
        assert_eq!(PartitionType::Split.sub_block_count(), 4);
        assert_eq!(PartitionType::HorzA.sub_block_count(), 3);
        assert_eq!(PartitionType::HorzB.sub_block_count(), 3);
        assert_eq!(PartitionType::VertA.sub_block_count(), 3);
        assert_eq!(PartitionType::VertB.sub_block_count(), 3);
        assert_eq!(PartitionType::Horz4.sub_block_count(), 4);
        assert_eq!(PartitionType::Vert4.sub_block_count(), 4);
    }

    #[test]
    fn mi_units_rounds_up_to_whole_8_pixel_pairs_of_4x4_units() {
        // 2 * ceil(pixels / 8): the frame-edge check counts 4x4 units in pairs.
        for (pixels, units) in [(0, 0), (1, 2), (4, 2), (8, 2), (9, 4), (16, 4), (17, 6)] {
            assert_eq!(mi_units(pixels), units, "{pixels} pixels");
        }
    }

    #[test]
    fn a_block_that_is_not_partitioned_starts_where_its_parent_does() {
        assert_eq!(
            child_position(8, 16, 0, PartitionType::None, BlockSize::Block16x16),
            (8, 16)
        );
    }

    #[test]
    fn the_grid_has_one_block_per_leaf_with_its_depth_and_partition() {
        let mut root = PartitionNode::new(0, 0, BlockSize::Block32x32, PartitionType::Split);
        for (x, y, partition) in [
            (0, 0, PartitionType::None),
            (16, 0, PartitionType::Horz),
            (0, 16, PartitionType::Vert),
            (16, 16, PartitionType::None),
        ] {
            root.children
                .push(PartitionNode::new(x, y, BlockSize::Block16x16, partition));
        }

        let grid = partition_tree_to_grid(&root, 32, 32, 32);

        let blocks: Vec<_> = grid
            .blocks
            .iter()
            .map(|b| (b.x, b.y, b.width, b.height, b.partition, b.depth))
            .collect();
        use bitvue_engine::partition_grid::PartitionType as Grid;
        assert_eq!(
            blocks,
            vec![
                (0, 0, 16, 16, Grid::None, 1),
                (16, 0, 16, 16, Grid::Horz, 1),
                (0, 16, 16, 16, Grid::Vert, 1),
                (16, 16, 16, 16, Grid::None, 1),
            ]
        );
    }

    #[test]
    fn a_tree_that_is_a_single_leaf_is_one_block_at_depth_zero() {
        let root = PartitionNode::new(0, 0, BlockSize::Block64x64, PartitionType::None);

        let grid = partition_tree_to_grid(&root, 64, 64, 64);

        assert_eq!(grid.blocks.len(), 1);
        assert_eq!(
            (
                grid.blocks[0].width,
                grid.blocks[0].height,
                grid.blocks[0].depth
            ),
            (64, 64, 0)
        );
    }

    #[test]
    fn test_block_size_dimensions() {
        assert_eq!(BlockSize::Block8x8.width(), 8);
        assert_eq!(BlockSize::Block8x8.height(), 8);
        assert_eq!(BlockSize::Block16x32.width(), 16);
        assert_eq!(BlockSize::Block16x32.height(), 32);
        assert_eq!(BlockSize::Block128x128.width(), 128);
        assert_eq!(BlockSize::Block128x128.height(), 128);
    }

    #[test]
    fn test_block_size_sub_blocks() {
        let subs = BlockSize::Block16x16.sub_block_size(PartitionType::Split);
        assert_eq!(subs.len(), 4);
        assert_eq!(subs[0], BlockSize::Block8x8);

        let subs = BlockSize::Block32x32.sub_block_size(PartitionType::Horz);
        assert_eq!(subs.len(), 2);
        assert_eq!(subs[0], BlockSize::Block32x16);
    }

    #[test]
    fn test_partition_node_leaf_blocks() {
        let node = PartitionNode::new(0, 0, BlockSize::Block16x16, PartitionType::None);
        let leaves = node.leaf_blocks();

        assert_eq!(leaves.len(), 1);
        assert_eq!(leaves[0], (0, 0, BlockSize::Block16x16));
    }

    #[test]
    fn test_partition_node_with_children() {
        let mut parent = PartitionNode::new(0, 0, BlockSize::Block32x32, PartitionType::Split);
        parent.children.push(PartitionNode::new(
            0,
            0,
            BlockSize::Block16x16,
            PartitionType::None,
        ));
        parent.children.push(PartitionNode::new(
            16,
            0,
            BlockSize::Block16x16,
            PartitionType::None,
        ));
        parent.children.push(PartitionNode::new(
            0,
            16,
            BlockSize::Block16x16,
            PartitionType::None,
        ));
        parent.children.push(PartitionNode::new(
            16,
            16,
            BlockSize::Block16x16,
            PartitionType::None,
        ));

        assert!(!parent.is_leaf());
        let leaves = parent.leaf_blocks();
        assert_eq!(leaves.len(), 4);
    }

    // ---- parse_partition_recursive error paths --------------------------------------------

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

    /// Runs `parse_partition_recursive` on a fresh state for `data` and returns the result
    /// message (`Err`) or the node.
    fn recurse(
        data: &[u8],
        (x, y): (u32, u32),
        block_size: BlockSize,
        (mi_rows, mi_cols): (u32, u32),
        depth: u8,
    ) -> bitvue_engine::Result<Option<PartitionNode>> {
        use crate::tile::{MvPredictorContext, TileContext, TileState};
        let mut state = TileState {
            decoder: crate::symbol::SymbolDecoder::new(data).unwrap(),
            mv_ctx: MvPredictorContext::new(1, 1),
            tile_ctx: TileContext::new(64, 64),
        };
        parse_partition_recursive(
            &mut state,
            x,
            y,
            block_size,
            mi_rows,
            mi_cols,
            depth,
            &mut |_, _| Ok(()),
        )
    }

    fn message(r: bitvue_engine::Result<Option<PartitionNode>>) -> Option<String> {
        match r {
            Err(bitvue_engine::BitvueError::InvalidData(m)) => Some(m),
            Err(other) => panic!("unexpected error kind: {other:?}"),
            Ok(_) => None,
        }
    }

    #[test]
    fn recursion_stops_with_an_error_at_the_depth_limit() {
        let data = noise(1, 64);
        let (rows, cols) = (mi_units(8), mi_units(8));
        let at_limit = message(recurse(
            &data,
            (0, 0),
            BlockSize::Block8x8,
            (rows, cols),
            10,
        ));
        assert!(at_limit.unwrap().contains("depth exceeded maximum (10)"));
        assert!(message(recurse(
            &data,
            (0, 0),
            BlockSize::Block8x8,
            (rows, cols),
            200
        ))
        .is_some());
        // The limit is checked before the frame-edge test: even an outside block errors.
        assert!(message(recurse(
            &data,
            (4096, 0),
            BlockSize::Block8x8,
            (rows, cols),
            10
        ))
        .is_some());
        // One level below the limit it is just a normal parse (no depth error).
        let below = message(recurse(&data, (0, 0), BlockSize::Block8x8, (rows, cols), 9));
        assert!(
            below.is_none_or(|m| !m.contains("depth")),
            "depth 9 is allowed"
        );
    }

    #[test]
    fn a_block_outside_the_frame_returns_nothing_and_reads_nothing() {
        let data = noise(2, 64);
        // x = 64 px = 16 units is beyond a frame 8 units wide; likewise rows.
        for pos in [(64, 0), (0, 64), (64, 64)] {
            let r = recurse(&data, pos, BlockSize::Block16x16, (8, 8), 0).unwrap();
            assert!(r.is_none(), "{pos:?}");
        }
        // Exactly on the edge (unit 8 of 8) is outside as well; unit 7 is inside.
        assert!(recurse(&data, (32, 0), BlockSize::Block8x8, (8, 8), 0)
            .unwrap()
            .is_none());
        assert!(recurse(&data, (28, 0), BlockSize::Block8x8, (8, 8), 0)
            .unwrap()
            .is_some());
    }

    #[test]
    fn a_partition_that_does_not_fit_the_block_shape_is_an_error_not_a_bad_tree() {
        // The recursion also runs on the non-square children of terminal partitions; a
        // symbol that is legal for a square block of that size can be illegal (or produce
        // same-size children) for a rectangular one. Both must surface as errors.
        use BlockSize::*;
        let mut not_allowed = std::collections::BTreeSet::new();
        let mut same_size = std::collections::BTreeSet::new();
        for bs in [
            Block4x8,
            Block16x32,
            Block32x64,
            Block16x8,
            Block32x16,
            Block64x128,
        ] {
            for seed in 1..=300u64 {
                let data = noise(seed, 64);
                let (rows, cols) = (mi_units(bs.height()), mi_units(bs.width()));
                if let Some(m) = message(recurse(&data, (0, 0), bs, (rows, cols), 0)) {
                    if m.contains("not allowed for block size") {
                        assert!(m.contains(&format!("{bs:?}")), "{m}");
                        not_allowed.insert((bs.width(), bs.height()));
                    } else if m.contains("produces sub-blocks of same size") {
                        assert!(m.contains(&format!("{bs:?}")), "{m}");
                        same_size.insert((bs.width(), bs.height()));
                    }
                }
            }
        }
        assert!(not_allowed.contains(&(4, 8)), "{not_allowed:?}");
        assert!(not_allowed.contains(&(16, 32)), "{not_allowed:?}");
        assert!(same_size.contains(&(16, 8)), "{same_size:?}");
        assert!(same_size.contains(&(32, 64)), "{same_size:?}");
    }

    #[test]
    fn square_blocks_never_hit_the_shape_errors() {
        use BlockSize::*;
        for bs in [Block8x8, Block16x16, Block32x32, Block64x64, Block128x128] {
            for seed in 1..=200u64 {
                let data = noise(seed, 128);
                let (rows, cols) = (mi_units(bs.height()), mi_units(bs.width()));
                if let Some(m) = message(recurse(&data, (0, 0), bs, (rows, cols), 0)) {
                    assert!(
                        !m.contains("not allowed")
                            && !m.contains("same size")
                            && !m.contains("depth"),
                        "{bs:?} seed {seed}: {m}"
                    );
                }
            }
        }
    }
}
