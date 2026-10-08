//! AV1 Tile Parsing
//!
//! Per AV1 Specification Section 5.11 (Tile Syntax)
//! This module parses tile data to extract block-level information:
//! - Partition structure (superblock → block tree)
//! - Prediction modes (Intra/Inter)
//! - Motion vectors (for Inter frames)
//! - Transform information
//!
//! ## Implementation Status
//!
//! **MVP Phase (KEY Frames Only)**:
//! - ✅ Tile group structure
//! - 🚧 Partition tree parser (in progress)
//! - ⏳ Intra mode extraction
//! - ⏳ Block size grid
//!
//! **Full Implementation (Later)**:
//! - ⏳ Symbol decoder (arithmetic coding)
//! - ⏳ INTER frame support
//! - ⏳ Motion vector extraction
//! - ⏳ Transform coefficient parsing

pub mod coding_unit;
pub mod context;
pub mod frame_params;
pub mod motion_field;
pub mod mv_prediction;
pub mod partition;
pub mod position;
pub mod superblock;
pub mod tile_group;
pub mod tile_state;

pub use coding_unit::{
    parse_coding_unit, CodingUnit, InterModeFlags, MotionVector, PaletteInfo, PredictionMode,
    RefFrame, TxSize, TxTypeFrameFlags,
};
pub use context::TileContext;
pub use frame_params::FrameCodingParams;
pub use motion_field::{
    add_temporal_candidates, project_motion_field, select_motion_field_sources, store_motion_field,
    MfmvSource, MotionFieldGrid, MotionFieldState, ProjectedMotionField, ProjectedMv, SavedMv,
};
pub use mv_prediction::MvPredictorContext;
pub use partition::{
    parse_partition_tree, partition_tree_to_grid, BlockSize, PartitionNode, PartitionType,
};
pub use position::{BlockRect, MiRect, SuperblockCtx};
pub use superblock::{parse_superblock, Superblock};
pub use tile_group::{parse_tile_group, TileGroup, TileInfo};
pub use tile_state::TileState;

use serde::{Deserialize, Serialize};

/// Tile data (single tile within a tile group)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tile {
    /// Tile column index
    pub tile_col: u32,
    /// Tile row index
    pub tile_row: u32,
    /// Tile width in superblocks
    pub sb_cols: u32,
    /// Tile height in superblocks
    pub sb_rows: u32,
    /// Tile data (compressed)
    pub data: Vec<u8>,
    /// Tile data size in bytes
    pub size: usize,
}

impl Tile {
    /// Create a new tile
    pub fn new(tile_col: u32, tile_row: u32, sb_cols: u32, sb_rows: u32, data: Vec<u8>) -> Self {
        let size = data.len();
        Self {
            tile_col,
            tile_row,
            sb_cols,
            sb_rows,
            data,
            size,
        }
    }

    /// Get tile dimensions in pixels (assuming 128x128 superblocks)
    pub fn pixel_dimensions(&self, sb_size: u32) -> (u32, u32) {
        (self.sb_cols * sb_size, self.sb_rows * sb_size)
    }
}

/// Superblock size
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SuperblockSize {
    /// 128x128 superblock
    Sb128x128 = 128,
    /// 64x64 superblock
    Sb64x64 = 64,
}

impl SuperblockSize {
    /// Get size in pixels
    pub fn size(&self) -> u32 {
        match self {
            SuperblockSize::Sb128x128 => 128,
            SuperblockSize::Sb64x64 => 64,
        }
    }

    /// Parse from sequence header flags
    pub fn from_seq_header(use_128x128_superblock: bool) -> Self {
        if use_128x128_superblock {
            SuperblockSize::Sb128x128
        } else {
            SuperblockSize::Sb64x64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tile_creation() {
        let data = vec![0x12, 0x34, 0x56, 0x78];
        let tile = Tile::new(0, 0, 4, 3, data.clone());

        assert_eq!(tile.tile_col, 0);
        assert_eq!(tile.tile_row, 0);
        assert_eq!(tile.sb_cols, 4);
        assert_eq!(tile.sb_rows, 3);
        assert_eq!(tile.size, 4);
        assert_eq!(tile.data, data);
    }

    #[test]
    fn test_tile_pixel_dimensions() {
        let tile = Tile::new(0, 0, 4, 3, vec![]);
        let (width, height) = tile.pixel_dimensions(128);

        assert_eq!(width, 512); // 4 * 128
        assert_eq!(height, 384); // 3 * 128
    }

    #[test]
    fn test_superblock_size() {
        assert_eq!(SuperblockSize::Sb128x128.size(), 128);
        assert_eq!(SuperblockSize::Sb64x64.size(), 64);

        assert_eq!(
            SuperblockSize::from_seq_header(true),
            SuperblockSize::Sb128x128
        );
        assert_eq!(
            SuperblockSize::from_seq_header(false),
            SuperblockSize::Sb64x64
        );
    }

    /// An intra block in an inter frame must be recorded in the reference map with its size and
    /// no motion (dav1d `splat_intraref`): the neighbour scans step by the stored width/height.
    /// Pseudo-random tile data reaches intra blocks in an inter frame without needing a stream.
    #[test]
    fn intra_blocks_in_an_inter_frame_are_recorded_in_the_reference_map() {
        use crate::frame_header::TxfmMode;
        use crate::frame_header_full::SegmentationInfo;
        let params = FrameCodingParams {
            is_key_frame: false,
            delta_q_enabled: false,
            reference_select: false,
            allow_intrabc: false,
            allow_screen_content_tools: false,
            enable_filter_intra: false,
            delta_lf_present: false,
            delta_lf_multi: false,
            use_ref_frame_mvs: false,
            segmentation: SegmentationInfo::default(),
            tx_type_flags: TxTypeFrameFlags {
                coded_lossless: false,
                qidx_is_zero: false,
                reduced_tx_set: false,
                txfm_mode: TxfmMode::default(),
                mono_chrome: false,
                subsampling_x: true,
                subsampling_y: true,
            },
            inter_mode_flags: InterModeFlags {
                switchable_motion_mode: true,
                allow_warped_motion: true,
                enable_interintra_compound: true,
                enable_masked_compound: true,
                enable_jnt_comp: true,
                subpel_filter_switchable: true,
                force_integer_mv: false,
                allow_high_precision_mv: false,
                gm_type: [0; 8],
            },
            mi_rows: 32,
            mi_cols: 32,
            cdef_bits: 2,
            skip_mode_present: false,
            skip_mode_refs: [0, 0],
        };
        let mut checked = 0;
        for seed in 0..32u64 {
            let mut x = seed.wrapping_mul(0x9e37_79b9_7f4a_7c15) | 1;
            let data: Vec<u8> = (0..512)
                .map(|_| {
                    x ^= x << 13;
                    x ^= x >> 7;
                    x ^= x << 17;
                    (x >> 24) as u8
                })
                .collect();
            let mut state = TileState {
                decoder: crate::SymbolDecoder::new(&data).unwrap(),
                mv_ctx: MvPredictorContext::new(2, 2),
                tile_ctx: TileContext::new(32, 32),
            };
            let Ok((sb, _)) = crate::parse_superblock(&mut state, 0, 0, 64, &params, 100) else {
                continue;
            };
            for cu in &sb.coding_units {
                if cu.is_intra() && !cu.use_intrabc {
                    let (x4, y4) = (cu.x / 4, cu.y / 4);
                    assert_eq!(
                        state.tile_ctx.spatial_ref_cell(x4, y4),
                        Some((false, (cu.width / 4) as u8, (cu.height / 4) as u8)),
                        "seed {seed}: intra block at ({x4}, {y4})"
                    );
                    checked += 1;
                }
            }
        }
        assert!(checked > 0, "no intra block reached");
    }
}
