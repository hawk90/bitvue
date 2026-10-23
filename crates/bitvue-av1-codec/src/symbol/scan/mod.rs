//! Real AV1 coefficient scan order + `coeff_base`/`coeff_br` neighbor-context derivation (spec
//! 8.3.2's `get_coef_base_ctx`/`get_br_ctx`, dav1d's `get_lo_ctx`).
//!
//! Source: rav1d (`memorysafety/rav1d`, BSD-2-Clause) `src/scan.rs` (scan tables) and its C
//! predecessor `src/recon_tmpl.c`'s `get_lo_ctx`/`DECODE_COEFS_CLASS` (context formula --
//! verified by reading the C directly, not from memory, per this session's established
//! precedent). Only the **2D** (default) scan needs literal table data; `TX_CLASS_H`/`_V` compute
//! their coefficient-grid position from a closed-form row-major/column-major formula instead
//! (`x = c % dim, y = c / dim`, where `dim` is the transform's *height* -- H/V transforms only
//! ever arise for genuinely rectangular blocks in real AV1, but the position formula itself
//! doesn't depend on which axis is longer, only on `height_dim`), so no 3-way `TxClass` split is
//! needed for position -- the existing `is_1d: bool` (`SymbolDecoder::read_transform_type_is_1d`)
//! is sufficient. The **2D** neighbor-context *offset table* (`lo_ctx`'s 2D branch), unlike
//! position, does depend on aspect ratio for non-square transforms -- dav1d's real
//! `dav1d_lo_ctx_offsets[3][5][5]` has a square variant (`[0]`, this crate's original
//! `LO_CTX_OFFSETS`) plus two more for `width > height` (`[1]`) and `width < height` (`[2]`,
//! `src/tables.c`) -- both ported below and selected by `lo_ctx`'s new width/height params.
//!
//! Real AV1 caps residual *scanning* to the top-left 32x32 sub-block even for a 64x64-containing
//! transform (`slw = min(t_dim->lw, TX_32X32)` in dav1d) -- matches this crate's existing
//! `coeff_base_eob_context`/`eob_bin_16/64/256/1024` precedent of a size-class cap. For
//! rectangular transforms with a 64-sample axis (16x64/64x16/32x64/64x32), dav1d's own
//! `dav1d_scans` table (`src/scan.c`) reuses the *other* axis's already-32-or-smaller scan table
//! directly (e.g. `RTX_64X32` reuses `scan_32x32`, `RTX_16X64` reuses `scan_16x32`) rather than a
//! distinct 64-something table -- `scan_table`'s callers are expected to have already clamped
//! both dims to `<=32` (mirroring the existing `tx_size_class(..).min(3)`-style square cap) before
//! calling, so no separate 64-handling exists in this module.

mod levels;
mod order;

pub use levels::{lo_ctx, LevelBuffer};
pub use order::coeff_position;
