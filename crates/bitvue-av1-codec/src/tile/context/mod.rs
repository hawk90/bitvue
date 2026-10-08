//! Above/left neighbor-state tracking for entropy-context derivation.
//!
//! Per AV1 spec Section 9.3 (Function `get_ctx`) and rav1d's `BlockContext`
//! (`memorysafety/rav1d`, BSD-2-Clause, `src/env.rs`) -- covers the `skip` flag's context (see
//! `SymbolDecoder::read_skip`'s doc), key-frame `intra_mode`'s context (see
//! `SymbolDecoder::read_intra_mode`'s doc), real partition-context (per-8x8 bitmask, ported from
//! dav1d's edge-index tree), and the real spatial+temporal reference-motion-vector-candidate
//! subsystem (`refmvs`, ported from `src/refmvs.c`) backing `inter_mode`/`compound_mode`/DRL
//! context -- see `docs/DEVELOPMENT_PHASES.md` Phase 4's AV1 entropy-decoding notes for the full
//! history. Residual (`coeff_base`/`coeff_br`/etc.) context is real for neighbor/position axes
//! but still approximates the plane (luma-only) and qindex-bucket (first-bucket-only) axes --
//! see `SpatialRefContext`'s residual CDF selection and `symbol/cdf.rs`.
//!
//! Split by responsibility (the module was one 3,950-line file):
//!
//! - [`SpatialRefContext`] (`spatial_ref/`): the reference-motion-vector subsystem (dav1d
//!   `refmvs`) -- the per-4x4 map of decoded blocks, the `inter_mode`/`compound_mode`
//!   contexts, and the candidate stacks the DRL bits and MV predictors come from.
//! - [`TileContext`] (`tile_context/`): the above/left neighbour state of everything else, one
//!   file per syntax-element family.
//! - `drl`: the DRL context and the stack entries.
//! - `tables`: the constant lookup tables.

mod drl;
mod spatial_ref;
mod tables;
mod tile_context;

pub use drl::{get_compound_drl_context, get_drl_context, CompoundMvStackEntry, MvStackEntry};
pub use spatial_ref::SpatialRefContext;
pub use tables::partition_bl;
pub use tile_context::TileContext;
