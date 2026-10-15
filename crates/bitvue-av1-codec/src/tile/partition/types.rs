//! `PartitionType` and `BlockSize` with their geometry tables.

use serde::{Deserialize, Serialize};

/// Partition type (AV1 Spec Section 5.11.4)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum PartitionType {
    /// No partition (use entire block)
    None = 0,
    /// Horizontal split (2 blocks)
    Horz = 1,
    /// Vertical split (2 blocks)
    Vert = 2,
    /// Split into 4 equal blocks
    Split = 3,
    /// Horizontal split with top half split again
    HorzA = 4,
    /// Horizontal split with bottom half split again
    HorzB = 5,
    /// Vertical split with left half split again
    VertA = 6,
    /// Vertical split with right half split again
    VertB = 7,
    /// 4-way horizontal split
    Horz4 = 8,
    /// 4-way vertical split
    Vert4 = 9,
}

impl PartitionType {
    /// Parse from value (0-9)
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(PartitionType::None),
            1 => Some(PartitionType::Horz),
            2 => Some(PartitionType::Vert),
            3 => Some(PartitionType::Split),
            4 => Some(PartitionType::HorzA),
            5 => Some(PartitionType::HorzB),
            6 => Some(PartitionType::VertA),
            7 => Some(PartitionType::VertB),
            8 => Some(PartitionType::Horz4),
            9 => Some(PartitionType::Vert4),
            _ => None,
        }
    }

    /// Get number of sub-blocks this partition creates
    pub fn sub_block_count(&self) -> usize {
        match self {
            PartitionType::None => 1,
            PartitionType::Horz | PartitionType::Vert => 2,
            PartitionType::Split => 4,
            PartitionType::HorzA | PartitionType::HorzB => 3,
            PartitionType::VertA | PartitionType::VertB => 3,
            PartitionType::Horz4 | PartitionType::Vert4 => 4,
        }
    }

    /// Check if this partition type is allowed for given block size
    pub fn is_allowed(&self, block_size: BlockSize) -> bool {
        match self {
            PartitionType::None => true,
            PartitionType::Horz | PartitionType::Vert => {
                block_size.width() >= 8 && block_size.height() >= 8
            }
            PartitionType::Split => block_size.width() >= 8 && block_size.height() >= 8,
            PartitionType::HorzA | PartitionType::HorzB => {
                block_size.width() >= 16 && block_size.height() >= 16
            }
            PartitionType::VertA | PartitionType::VertB => {
                block_size.width() >= 16 && block_size.height() >= 16
            }
            // spec 5.11.4 / the real `partition_cdfs` alphabet shape (`CdfContext::new`'s doc):
            // HORZ_4/VERT_4 are legal for exactly the square 16x16/32x32/64x64 block sizes (10-
            // symbol partition alphabet) -- NOT 8x8 (4-symbol alphabet, no HORZ_4/VERT_4 at all)
            // and NOT 128x128 (8-symbol alphabet: HORZ_A/B/VERT_A/B but no HORZ_4/VERT_4 either,
            // `Default_Partition_W128_Cdf`'s 7-threshold/8-symbol shape). Previously an asymmetric
            // `width>=16 && height>=32` (Horz4) / `width>=32 && height>=16` (Vert4) check -- wrong
            // shape entirely, rejecting a real, correctly-decoded `Horz4` at a plain 16x16 (height
            // 16, not >=32) as "not allowed" even though it's spec-legal. Found via a real dav1d
            // oracle build cross-check surfacing this as a hard decode error on real fixture
            // frames once tile_data finally started at the right byte offset (see this crate's
            // `parser.rs` `ObuType::Frame` fix).
            PartitionType::Horz4 | PartitionType::Vert4 => {
                let w = block_size.width();
                w == block_size.height() && (16..=64).contains(&w)
            }
        }
    }
}

/// Block size enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlockSize {
    /// 4x4 block
    Block4x4,
    /// 4x8 block
    Block4x8,
    /// 8x4 block
    Block8x4,
    /// 8x8 block
    Block8x8,
    /// 8x16 block
    Block8x16,
    /// 16x8 block
    Block16x8,
    /// 16x16 block
    Block16x16,
    /// 16x32 block
    Block16x32,
    /// 32x16 block
    Block32x16,
    /// 32x32 block
    Block32x32,
    /// 32x64 block
    Block32x64,
    /// 64x32 block
    Block64x32,
    /// 64x64 block
    Block64x64,
    /// 64x128 block
    Block64x128,
    /// 128x64 block
    Block128x64,
    /// 128x128 block
    Block128x128,
    /// 16x4 block (HORZ_4 sub-block of a 16x16 -- the smallest HORZ_4 target, missing until this
    /// crate's `BlockSize` was found to have no way to represent it at all despite `Horz4` being
    /// spec-legal on 16x16; see `PartitionType::is_allowed`'s doc)
    Block16x4,
    /// 32x8 block (HORZ_4 sub-block)
    Block32x8,
    /// 64x16 block (HORZ_4 sub-block)
    Block64x16,
    /// 128x32 block (HORZ_4 sub-block) -- unreachable in practice (`Horz4`/`Vert4` aren't legal on
    /// 128x128, see `is_allowed`'s doc), kept only for enum completeness/symmetry
    Block128x32,
    /// 4x16 block (VERT_4 sub-block of a 16x16, `Block16x4`'s transpose)
    Block4x16,
    /// 8x32 block (VERT_4 sub-block)
    Block8x32,
    /// 16x64 block (VERT_4 sub-block)
    Block16x64,
    /// 32x128 block (VERT_4 sub-block) -- unreachable in practice, see `Block128x32`'s doc
    Block32x128,
}

impl BlockSize {
    /// Get block width in pixels
    pub fn width(&self) -> u32 {
        match self {
            BlockSize::Block4x4 | BlockSize::Block4x8 | BlockSize::Block4x16 => 4,
            BlockSize::Block8x4
            | BlockSize::Block8x8
            | BlockSize::Block8x16
            | BlockSize::Block8x32 => 8,
            BlockSize::Block16x4
            | BlockSize::Block16x8
            | BlockSize::Block16x16
            | BlockSize::Block16x32
            | BlockSize::Block16x64 => 16,
            BlockSize::Block32x8
            | BlockSize::Block32x16
            | BlockSize::Block32x32
            | BlockSize::Block32x64
            | BlockSize::Block32x128 => 32,
            BlockSize::Block64x16
            | BlockSize::Block64x32
            | BlockSize::Block64x64
            | BlockSize::Block64x128 => 64,
            BlockSize::Block128x32 | BlockSize::Block128x64 | BlockSize::Block128x128 => 128,
        }
    }

    /// Get block height in pixels
    pub fn height(&self) -> u32 {
        match self {
            BlockSize::Block4x4 | BlockSize::Block8x4 | BlockSize::Block16x4 => 4,
            BlockSize::Block4x8
            | BlockSize::Block8x8
            | BlockSize::Block16x8
            | BlockSize::Block32x8 => 8,
            BlockSize::Block4x16
            | BlockSize::Block8x16
            | BlockSize::Block16x16
            | BlockSize::Block32x16
            | BlockSize::Block64x16 => 16,
            BlockSize::Block8x32
            | BlockSize::Block16x32
            | BlockSize::Block32x32
            | BlockSize::Block64x32
            | BlockSize::Block128x32 => 32,
            BlockSize::Block16x64
            | BlockSize::Block32x64
            | BlockSize::Block64x64
            | BlockSize::Block128x64 => 64,
            BlockSize::Block32x128 | BlockSize::Block64x128 | BlockSize::Block128x128 => 128,
        }
    }

    /// Get sub-block size after partition
    pub fn sub_block_size(&self, partition: PartitionType) -> Vec<BlockSize> {
        match partition {
            PartitionType::None
            | PartitionType::Horz
            | PartitionType::Vert
            | PartitionType::Split => self.even_sub_blocks(partition),
            PartitionType::HorzB | PartitionType::HorzA => self.horz_ab_sub_blocks(partition),
            PartitionType::VertB | PartitionType::VertA => self.vert_ab_sub_blocks(partition),
            PartitionType::Horz4 | PartitionType::Vert4 => self.strip_sub_blocks(partition),
        }
    }

    /// `None`, `Horz`, `Vert` and `Split`: the equal-sized sub-blocks.
    fn even_sub_blocks(&self, partition: PartitionType) -> Vec<BlockSize> {
        match partition {
            PartitionType::None => vec![*self],
            PartitionType::Horz => {
                // Split horizontally
                match self {
                    BlockSize::Block8x8 => vec![BlockSize::Block8x4, BlockSize::Block8x4],
                    BlockSize::Block16x16 => vec![BlockSize::Block16x8, BlockSize::Block16x8],
                    BlockSize::Block32x32 => vec![BlockSize::Block32x16, BlockSize::Block32x16],
                    BlockSize::Block64x64 => vec![BlockSize::Block64x32, BlockSize::Block64x32],
                    BlockSize::Block128x128 => vec![BlockSize::Block128x64, BlockSize::Block128x64],
                    _ => vec![*self], // Fallback
                }
            }
            PartitionType::Vert => {
                // Split vertically
                match self {
                    BlockSize::Block8x8 => vec![BlockSize::Block4x8, BlockSize::Block4x8],
                    BlockSize::Block16x16 => vec![BlockSize::Block8x16, BlockSize::Block8x16],
                    BlockSize::Block32x32 => vec![BlockSize::Block16x32, BlockSize::Block16x32],
                    BlockSize::Block64x64 => vec![BlockSize::Block32x64, BlockSize::Block32x64],
                    BlockSize::Block128x128 => vec![BlockSize::Block64x128, BlockSize::Block64x128],
                    _ => vec![*self], // Fallback
                }
            }
            PartitionType::Split => {
                // Split into 4 equal blocks
                match self {
                    BlockSize::Block8x8 => vec![BlockSize::Block4x4; 4],
                    BlockSize::Block16x16 => vec![BlockSize::Block8x8; 4],
                    BlockSize::Block32x32 => vec![BlockSize::Block16x16; 4],
                    BlockSize::Block64x64 => vec![BlockSize::Block32x32; 4],
                    BlockSize::Block128x128 => vec![BlockSize::Block64x64; 4],
                    _ => vec![*self], // Fallback
                }
            }
            _ => unreachable!("`sub_block_size` only passes the equal-split partitions"),
        }
    }

    /// `HorzB` and `HorzA`: one full-width block plus two half-width blocks.
    fn horz_ab_sub_blocks(&self, partition: PartitionType) -> Vec<BlockSize> {
        match partition {
            PartitionType::HorzB => {
                // Top row: full-width block at half height
                // Bottom row: left half + right half, each at half height
                // sub-block sizes: [top-full, bottom-left-half, bottom-right-half]
                match self {
                    BlockSize::Block16x16 => vec![
                        BlockSize::Block16x8,
                        BlockSize::Block8x8,
                        BlockSize::Block8x8,
                    ],
                    BlockSize::Block32x32 => vec![
                        BlockSize::Block32x16,
                        BlockSize::Block16x16,
                        BlockSize::Block16x16,
                    ],
                    BlockSize::Block64x64 => vec![
                        BlockSize::Block64x32,
                        BlockSize::Block32x32,
                        BlockSize::Block32x32,
                    ],
                    BlockSize::Block128x128 => vec![
                        BlockSize::Block128x64,
                        BlockSize::Block64x64,
                        BlockSize::Block64x64,
                    ],
                    _ => vec![*self],
                }
            }
            PartitionType::HorzA => {
                // Top row: left half + right half, each at half height
                // Bottom row: full-width block at half height
                // sub-block sizes: [top-left-half, top-right-half, bottom-full]
                match self {
                    BlockSize::Block16x16 => vec![
                        BlockSize::Block8x8,
                        BlockSize::Block8x8,
                        BlockSize::Block16x8,
                    ],
                    BlockSize::Block32x32 => vec![
                        BlockSize::Block16x16,
                        BlockSize::Block16x16,
                        BlockSize::Block32x16,
                    ],
                    BlockSize::Block64x64 => vec![
                        BlockSize::Block32x32,
                        BlockSize::Block32x32,
                        BlockSize::Block64x32,
                    ],
                    BlockSize::Block128x128 => vec![
                        BlockSize::Block64x64,
                        BlockSize::Block64x64,
                        BlockSize::Block128x64,
                    ],
                    _ => vec![*self],
                }
            }
            _ => unreachable!("`sub_block_size` only passes `HorzA` or `HorzB`"),
        }
    }

    /// `VertB` and `VertA`: one full-height block plus two half-height blocks.
    fn vert_ab_sub_blocks(&self, partition: PartitionType) -> Vec<BlockSize> {
        match partition {
            PartitionType::VertB => {
                // Left column: full-height block at half width
                // Right column: top half + bottom half, each at half width
                // sub-block sizes: [left-full, top-right-half, bottom-right-half]
                match self {
                    BlockSize::Block16x16 => vec![
                        BlockSize::Block8x16,
                        BlockSize::Block8x8,
                        BlockSize::Block8x8,
                    ],
                    BlockSize::Block32x32 => vec![
                        BlockSize::Block16x32,
                        BlockSize::Block16x16,
                        BlockSize::Block16x16,
                    ],
                    BlockSize::Block64x64 => vec![
                        BlockSize::Block32x64,
                        BlockSize::Block32x32,
                        BlockSize::Block32x32,
                    ],
                    BlockSize::Block128x128 => vec![
                        BlockSize::Block64x128,
                        BlockSize::Block64x64,
                        BlockSize::Block64x64,
                    ],
                    _ => vec![*self],
                }
            }
            PartitionType::VertA => {
                // Left column: top half + bottom half, each at half width
                // Right column: full-height block at half width
                // sub-block sizes: [top-left-half, bottom-left-half, right-full]
                match self {
                    BlockSize::Block16x16 => vec![
                        BlockSize::Block8x8,
                        BlockSize::Block8x8,
                        BlockSize::Block8x16,
                    ],
                    BlockSize::Block32x32 => vec![
                        BlockSize::Block16x16,
                        BlockSize::Block16x16,
                        BlockSize::Block16x32,
                    ],
                    BlockSize::Block64x64 => vec![
                        BlockSize::Block32x32,
                        BlockSize::Block32x32,
                        BlockSize::Block32x64,
                    ],
                    BlockSize::Block128x128 => vec![
                        BlockSize::Block64x64,
                        BlockSize::Block64x64,
                        BlockSize::Block64x128,
                    ],
                    _ => vec![*self],
                }
            }
            _ => unreachable!("`sub_block_size` only passes `VertA` or `VertB`"),
        }
    }

    /// `Horz4` and `Vert4`: four equal strips.
    fn strip_sub_blocks(&self, partition: PartitionType) -> Vec<BlockSize> {
        match partition {
            PartitionType::Horz4 => {
                // Four equal horizontal strips, each at 1/4 height -- only legal (`is_allowed`)
                // for the square 16x16/32x32/64x64 sizes, so those are the only real arms; the
                // rest fall back to `*self` (never reached in a real spec-legal parse).
                match self {
                    BlockSize::Block16x16 => vec![BlockSize::Block16x4; 4],
                    BlockSize::Block32x32 => vec![BlockSize::Block32x8; 4],
                    BlockSize::Block64x64 => vec![BlockSize::Block64x16; 4],
                    _ => vec![*self],
                }
            }
            PartitionType::Vert4 => {
                // Four equal vertical strips, each at 1/4 width -- same square-only legality as
                // `Horz4` above.
                match self {
                    BlockSize::Block16x16 => vec![BlockSize::Block4x16; 4],
                    BlockSize::Block32x32 => vec![BlockSize::Block8x32; 4],
                    BlockSize::Block64x64 => vec![BlockSize::Block16x64; 4],
                    _ => vec![*self],
                }
            }
            _ => unreachable!("`sub_block_size` only passes `Horz4` or `Vert4`"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL_PARTITIONS: [PartitionType; 10] = [
        PartitionType::None,
        PartitionType::Horz,
        PartitionType::Vert,
        PartitionType::Split,
        PartitionType::HorzA,
        PartitionType::HorzB,
        PartitionType::VertA,
        PartitionType::VertB,
        PartitionType::Horz4,
        PartitionType::Vert4,
    ];

    /// Every `BlockSize` variant.
    const ALL_SIZES: [BlockSize; 24] = [
        BlockSize::Block4x4,
        BlockSize::Block4x8,
        BlockSize::Block8x4,
        BlockSize::Block8x8,
        BlockSize::Block8x16,
        BlockSize::Block16x8,
        BlockSize::Block16x16,
        BlockSize::Block16x32,
        BlockSize::Block32x16,
        BlockSize::Block32x32,
        BlockSize::Block32x64,
        BlockSize::Block64x32,
        BlockSize::Block64x64,
        BlockSize::Block64x128,
        BlockSize::Block128x64,
        BlockSize::Block128x128,
        BlockSize::Block16x4,
        BlockSize::Block32x8,
        BlockSize::Block64x16,
        BlockSize::Block128x32,
        BlockSize::Block4x16,
        BlockSize::Block8x32,
        BlockSize::Block16x64,
        BlockSize::Block32x128,
    ];

    fn square(side: u32) -> BlockSize {
        match side {
            4 => BlockSize::Block4x4,
            8 => BlockSize::Block8x8,
            16 => BlockSize::Block16x16,
            32 => BlockSize::Block32x32,
            64 => BlockSize::Block64x64,
            128 => BlockSize::Block128x128,
            _ => panic!("no square block of side {side}"),
        }
    }

    const SQUARE_SIDES: [u32; 6] = [4, 8, 16, 32, 64, 128];

    fn dims(sizes: &[BlockSize]) -> Vec<(u32, u32)> {
        sizes.iter().map(|b| (b.width(), b.height())).collect()
    }

    // ---- PartitionType ----

    #[test]
    fn from_u8_round_trips_the_ten_symbols_and_rejects_everything_else() {
        for (value, expected) in ALL_PARTITIONS.iter().enumerate() {
            let parsed = PartitionType::from_u8(value as u8);
            assert_eq!(parsed, Some(*expected), "symbol {value}");
            assert_eq!(*expected as u8, value as u8, "discriminant of {expected:?}");
        }
        for value in 10..=u8::MAX {
            assert_eq!(PartitionType::from_u8(value), None, "symbol {value}");
        }
    }

    #[test]
    fn the_sub_block_counts_are_the_spec_counts() {
        let expected = [1, 2, 2, 4, 3, 3, 3, 3, 4, 4];
        for (partition, count) in ALL_PARTITIONS.iter().zip(expected) {
            assert_eq!(partition.sub_block_count(), count, "{partition:?}");
        }
    }

    /// Spec 5.11.4: which partition types a square block can choose.
    fn spec_allows(partition: PartitionType, side: u32) -> bool {
        match partition {
            PartitionType::None => true,
            PartitionType::Horz | PartitionType::Vert | PartitionType::Split => side >= 8,
            PartitionType::HorzA
            | PartitionType::HorzB
            | PartitionType::VertA
            | PartitionType::VertB => side >= 16,
            PartitionType::Horz4 | PartitionType::Vert4 => (16..=64).contains(&side),
        }
    }

    #[test]
    fn square_blocks_allow_exactly_the_spec_partitions() {
        for side in SQUARE_SIDES {
            for partition in ALL_PARTITIONS {
                assert_eq!(
                    partition.is_allowed(square(side)),
                    spec_allows(partition, side),
                    "{partition:?} on {side}x{side}"
                );
            }
        }
    }

    /// A partition needs both sides of the block to be big enough, whatever its shape.
    #[test]
    fn a_short_side_rules_out_the_partition_even_on_a_non_square_block() {
        for size in ALL_SIZES {
            let shortest = size.width().min(size.height());
            for partition in [
                PartitionType::Horz,
                PartitionType::Vert,
                PartitionType::Split,
            ] {
                if shortest < 8 {
                    assert!(!partition.is_allowed(size), "{partition:?} on {size:?}");
                }
            }
            for partition in [
                PartitionType::HorzA,
                PartitionType::HorzB,
                PartitionType::VertA,
                PartitionType::VertB,
            ] {
                if shortest < 16 {
                    assert!(!partition.is_allowed(size), "{partition:?} on {size:?}");
                }
            }
        }
    }

    #[test]
    fn the_four_way_partitions_are_never_allowed_on_non_square_blocks() {
        for size in ALL_SIZES {
            if size.width() == size.height() {
                continue;
            }
            for partition in [PartitionType::Horz4, PartitionType::Vert4] {
                assert!(!partition.is_allowed(size), "{partition:?} on {size:?}");
            }
            assert!(PartitionType::None.is_allowed(size), "None on {size:?}");
        }
    }

    #[test]
    fn four_way_partitions_need_both_sides_in_the_16_to_64_range() {
        // Edge sizes around the legal range, square and not.
        for (w, h, ok) in [
            (8, 8, false),
            (16, 16, true),
            (64, 64, true),
            (128, 128, false),
        ] {
            let size = ALL_SIZES
                .iter()
                .copied()
                .find(|b| (b.width(), b.height()) == (w, h))
                .unwrap();
            assert_eq!(PartitionType::Horz4.is_allowed(size), ok, "{w}x{h}");
            assert_eq!(PartitionType::Vert4.is_allowed(size), ok, "{w}x{h}");
        }
        // 16x32 has a side in the range but is not square.
        assert!(!PartitionType::Horz4.is_allowed(BlockSize::Block16x32));
        assert!(!PartitionType::Vert4.is_allowed(BlockSize::Block32x16));
    }

    // ---- BlockSize ----

    /// The width and height of each size, restated from its variant name (`Block<W>x<H>`).
    #[test]
    fn width_and_height_match_the_variant_names() {
        for size in ALL_SIZES {
            let name = format!("{size:?}");
            let (w, h) = name.strip_prefix("Block").unwrap().split_once('x').unwrap();
            assert_eq!(size.width(), w.parse::<u32>().unwrap(), "{name} width");
            assert_eq!(size.height(), h.parse::<u32>().unwrap(), "{name} height");
        }
    }

    #[test]
    fn every_size_is_distinct() {
        let mut seen = std::collections::BTreeSet::new();
        for size in ALL_SIZES {
            assert!(
                seen.insert((size.width(), size.height())),
                "{size:?} repeats a size"
            );
        }
        assert_eq!(seen.len(), 24);
    }

    /// The sub-blocks of a square `side`-sized block, in decode order, restated from the spec.
    fn expected_sub_blocks(partition: PartitionType, side: u32) -> Vec<(u32, u32)> {
        let (s, h, q) = (side, side / 2, side / 4);
        match partition {
            PartitionType::None => vec![(s, s)],
            PartitionType::Horz => vec![(s, h); 2],
            PartitionType::Vert => vec![(h, s); 2],
            PartitionType::Split => vec![(h, h); 4],
            PartitionType::HorzA => vec![(h, h), (h, h), (s, h)],
            PartitionType::HorzB => vec![(s, h), (h, h), (h, h)],
            PartitionType::VertA => vec![(h, h), (h, h), (h, s)],
            PartitionType::VertB => vec![(h, s), (h, h), (h, h)],
            PartitionType::Horz4 => vec![(s, q); 4],
            PartitionType::Vert4 => vec![(q, s); 4],
        }
    }

    #[test]
    fn allowed_partitions_of_square_blocks_split_as_the_spec_says() {
        for side in SQUARE_SIDES {
            for partition in ALL_PARTITIONS {
                if !spec_allows(partition, side) {
                    continue;
                }
                let subs = square(side).sub_block_size(partition);
                assert_eq!(
                    dims(&subs),
                    expected_sub_blocks(partition, side),
                    "{partition:?} on {side}x{side}"
                );
                assert_eq!(
                    subs.len(),
                    partition.sub_block_count(),
                    "{partition:?} count"
                );
            }
        }
    }

    #[test]
    fn the_sub_blocks_tile_the_parent_exactly() {
        for side in SQUARE_SIDES {
            for partition in ALL_PARTITIONS {
                if !spec_allows(partition, side) {
                    continue;
                }
                let area: u32 = square(side)
                    .sub_block_size(partition)
                    .iter()
                    .map(|b| b.width() * b.height())
                    .sum();
                assert_eq!(area, side * side, "{partition:?} on {side}x{side}");
            }
        }
    }

    /// `parse_partition_recursive` relies on this: a partition that is not allowed (or not
    /// real for the size) hands back the block itself, which it then rejects as "no smaller".
    #[test]
    fn a_partition_without_a_split_for_the_size_returns_the_block_itself() {
        for side in SQUARE_SIDES {
            for partition in ALL_PARTITIONS {
                if spec_allows(partition, side) {
                    continue;
                }
                assert_eq!(
                    square(side).sub_block_size(partition),
                    vec![square(side)],
                    "{partition:?} on {side}x{side}"
                );
            }
        }
        // Non-square blocks have no split at all, for any partition.
        for size in ALL_SIZES {
            if size.width() == size.height() {
                continue;
            }
            for partition in ALL_PARTITIONS {
                assert_eq!(
                    size.sub_block_size(partition),
                    vec![size],
                    "{partition:?} on {size:?}"
                );
            }
        }
    }

    #[test]
    fn every_split_of_an_allowed_square_block_makes_something_smaller() {
        for side in SQUARE_SIDES {
            for partition in ALL_PARTITIONS {
                if partition == PartitionType::None || !spec_allows(partition, side) {
                    continue;
                }
                let parent = square(side);
                assert!(
                    parent
                        .sub_block_size(partition)
                        .iter()
                        .any(|s| s.width() < parent.width() || s.height() < parent.height()),
                    "{partition:?} on {side}x{side}"
                );
            }
        }
    }
}
