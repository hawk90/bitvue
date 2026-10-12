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
}
