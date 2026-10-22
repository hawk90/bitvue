//! `palette_mode_info()` / `palette_tokens()` (AV1 spec 5.11.46, 5.11.49): palette colour cache,
//! colour reads, ordering and the per-pixel colour-index map.

mod mode_info;
#[cfg(test)]
#[path = "mode_info_tests.rs"]
mod mode_info_tests;
mod tokens;

pub(in crate::tile::coding_unit) use mode_info::read_palette_mode_info;
pub(in crate::tile::coding_unit) use tokens::read_palette_tokens;

use serde::{Deserialize, Serialize};

/// Real per-CU palette state from `read_palette_mode_info` (spec 5.11.46) -- `y_size`/`uv_size`
/// `0` when that plane doesn't use palette mode (the common case).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaletteInfo {
    pub y_size: u8,
    pub y_colors: [u16; 8],
    pub uv_size: u8,
    pub u_colors: [u16; 8],
    pub v_colors: [u16; 8],
}
