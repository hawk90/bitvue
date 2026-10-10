//! Macroblock, motion-vector and macroblock-type data model.

use bitvue_engine::partition_grid::PartitionType;
use serde::{Deserialize, Serialize};

/// Macroblock type for H.264
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MbType {
    /// I macroblock (intra)
    I4x4,
    I16x16,
    IPCM,
    /// P macroblock (predicted)
    PLuma,
    P8x8,
    /// B macroblock (bi-predictive)
    BDirect,
    B16x16,
    B16x8,
    B8x16,
    B8x8,
    /// Skip macroblock
    PSkip,
    BSkip,
}

impl MbType {
    /// Check if this is an INTRA macroblock
    pub fn is_intra(&self) -> bool {
        matches!(self, MbType::I4x4 | MbType::I16x16 | MbType::IPCM)
    }

    /// Check if this is a SKIP macroblock
    pub fn is_skip(&self) -> bool {
        matches!(self, MbType::PSkip | MbType::BSkip)
    }

    /// Get partition type for visualization
    pub fn to_partition_type(self) -> PartitionType {
        match self {
            MbType::I4x4 => PartitionType::Split,
            MbType::I16x16 => PartitionType::None,
            MbType::IPCM => PartitionType::None,
            MbType::PLuma => PartitionType::None,
            MbType::P8x8 => PartitionType::Split,
            MbType::BDirect => PartitionType::None,
            MbType::BSkip | MbType::PSkip => PartitionType::None,
            MbType::B16x16 => PartitionType::None,
            MbType::B16x8 => PartitionType::Horz,
            MbType::B8x16 => PartitionType::Vert,
            MbType::B8x8 => PartitionType::Split,
        }
    }
}

/// H.264 Macroblock information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Macroblock {
    /// Macroblock address (scan order)
    pub mb_addr: u32,
    /// Macroblock position in pixels
    pub x: u32,
    pub y: u32,
    /// Macroblock type
    pub mb_type: MbType,
    /// Skip flag
    pub skip: bool,
    /// QP value (for this macroblock)
    pub qp: i16,
    /// Motion vectors (for INTER blocks)
    /// [mv_l0, mv_l1] where each is (x, y) in quarter-pel units
    pub mv_l0: Option<MotionVector>,
    pub mv_l1: Option<MotionVector>,
    /// Reference frame indices
    pub ref_idx_l0: Option<i8>,
    pub ref_idx_l1: Option<i8>,
}

/// Motion vector for H.264 (quarter-pel precision)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct MotionVector {
    /// Horizontal component (quarter-pel units)
    pub x: i32,
    /// Vertical component (quarter-pel units)
    pub y: i32,
}

impl MotionVector {
    /// Create new motion vector
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    /// Zero motion vector
    pub fn zero() -> Self {
        Self { x: 0, y: 0 }
    }
}
