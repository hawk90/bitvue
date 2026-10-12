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
