//! Constant lookup tables shared by the context derivations.

//! Units throughout are 4x4 pixels (spec's context-array granularity).

/// What dav1d's `reset_context` writes into the var-tx context arrays (`memset(ctx->tx, TX_64X64)`):
/// a not-yet-coded neighbour counts as the largest transform, so it never makes `txfm_split`'s
/// `a`/`l` context bits 1.
pub(in crate::tile::context) const VAR_TX_UNSET: i8 = 4;

/// Maps a raw intra prediction-mode symbol (0..=12, spec `y_mode`/`uv_mode` values -- matches
/// `bitvue_av1_codec::tile::PredictionMode`'s intra-variant declaration order exactly) to one of
/// 5 mode-context classes used to index the key-frame `kfym` CDF. Source: rav1d
/// `DAV1D_INTRA_MODE_CONTEXT` (`memorysafety/rav1d`, BSD-2-Clause, `src/tables.rs`).
pub(in crate::tile::context) const INTRA_MODE_CONTEXT: [u8; 13] =
    [0, 1, 2, 3, 4, 4, 4, 4, 3, 0, 1, 2, 0];

/// `DAV1D_AL_PART_CTX[dir][bl][bp]` -- the bitmask written into `above_partition`/`left_partition`
/// after a `partition` symbol `bp` is decided at level `bl`, one row per `bl` (0=128x128..4=8x8,
/// rav1d's `BlockLevel` convention -- opposite of `block_size_log2`, see `partition_bl`), one
/// column per `PartitionType` (0=None..9=Vert4, matches this crate's `PartitionType` enum
/// ordering exactly). `0xff` marks a partition type that's invalid at that level (never actually
/// written in practice, since callers only reach `set_partition` for partition types
/// `PartitionType::is_allowed` at the block's own size already validated). Source: rav1d
/// `DAV1D_AL_PART_CTX` (`memorysafety/rav1d`, BSD-2-Clause, `src/tables.rs`).
pub(in crate::tile::context) const PARTITION_CTX_TABLE: [[[u8; 10]; 5]; 2] = [
    // above
    [
        [0x00, 0x00, 0x10, 0xff, 0x00, 0x10, 0x10, 0x10, 0xff, 0xff],
        [0x10, 0x10, 0x18, 0xff, 0x10, 0x18, 0x18, 0x18, 0x10, 0x1c],
        [0x18, 0x18, 0x1c, 0xff, 0x18, 0x1c, 0x1c, 0x1c, 0x18, 0x1e],
        [0x1c, 0x1c, 0x1e, 0xff, 0x1c, 0x1e, 0x1e, 0x1e, 0x1c, 0x1f],
        [0x1e, 0x1e, 0x1f, 0x1f, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff],
    ],
    // left
    [
        [0x00, 0x10, 0x00, 0xff, 0x10, 0x10, 0x00, 0x10, 0xff, 0xff],
        [0x10, 0x18, 0x10, 0xff, 0x18, 0x18, 0x10, 0x18, 0x1c, 0x10],
        [0x18, 0x1c, 0x18, 0xff, 0x1c, 0x1c, 0x18, 0x1c, 0x1e, 0x18],
        [0x1c, 0x1e, 0x1c, 0xff, 0x1e, 0x1e, 0x1c, 0x1e, 0x1f, 0x1c],
        [0x1e, 0x1f, 0x1e, 0x1f, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff],
    ],
];

/// `DAV1D_SKIP_CTX[min(la,4)][min(ll,4)]` -- `txb_skip`'s context table, indexed by the OR-reduced
/// above/left `cul_level` values (see `TileContext::txb_skip_context`'s doc). Source: rav1d
/// `dav1d_skip_ctx` (`memorysafety/rav1d`, BSD-2-Clause, `src/tables.rs`).
pub(in crate::tile::context) const DAV1D_SKIP_CTX: [[u8; 5]; 5] = [
    [1, 2, 2, 2, 3],
    [2, 4, 4, 4, 5],
    [2, 4, 4, 4, 5],
    [2, 4, 4, 4, 5],
    [3, 5, 5, 5, 6],
];

/// Maps `block_size_log2` (2..=7, this crate's CDF-lookup convention) to rav1d's `BlockLevel`
/// (0=128x128..4=8x8 -- opposite numeric order). Only valid for `block_size_log2` in 3..=7 (8x8
/// and up); 4x4 blocks (log2=2) never read a `partition` symbol at all, so have no `BlockLevel`.
pub fn partition_bl(block_size_log2: u8) -> u8 {
    7 - block_size_log2.clamp(3, 7)
}
