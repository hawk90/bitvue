//! Transform sizes.

use serde::{Deserialize, Serialize};

/// Transform size enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TxSize {
    Tx4x4 = 0,
    Tx8x8 = 1,
    Tx16x16 = 2,
    Tx32x32 = 3,
    Tx64x64 = 4,
}

impl TxSize {
    /// Get the size in pixels
    pub fn size(&self) -> u32 {
        match self {
            TxSize::Tx4x4 => 4,
            TxSize::Tx8x8 => 8,
            TxSize::Tx16x16 => 16,
            TxSize::Tx32x32 => 32,
            TxSize::Tx64x64 => 64,
        }
    }

    /// Get TxSize from block dimensions
    pub fn from_dimensions(width: u32, height: u32) -> Self {
        let size = width.max(height);
        match size {
            0..=4 => TxSize::Tx4x4,
            5..=8 => TxSize::Tx8x8,
            9..=16 => TxSize::Tx16x16,
            17..=32 => TxSize::Tx32x32,
            _ => TxSize::Tx64x64,
        }
    }

    /// Inverse of this enum's own discriminant order (0..=4) -- the size-*class* form
    /// `SymbolDecoder::read_tx_size`/`TileContext::tx_size_context` operate on. Clamps rather
    /// than erroring since callers only ever pass values already derived from a `TxSize`.
    pub fn from_class(class: u8) -> Self {
        match class {
            0 => TxSize::Tx4x4,
            1 => TxSize::Tx8x8,
            2 => TxSize::Tx16x16,
            3 => TxSize::Tx32x32,
            _ => TxSize::Tx64x64,
        }
    }
}
