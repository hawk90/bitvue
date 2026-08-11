//! Above/left neighbor-state tracking for entropy-context derivation.
//!
//! Per AV1 spec Section 9.3 (Function `get_ctx`) and rav1d's `BlockContext`
//! (`memorysafety/rav1d`, BSD-2-Clause, `src/env.rs`) -- currently covers the `skip` flag's
//! context (see `SymbolDecoder::read_skip`'s doc) and key-frame `intra_mode`'s context (see
//! `SymbolDecoder::read_intra_mode`'s doc). Real partition-context and inter/compound-mode
//! context derivation both need considerably larger ports deferred to later phases -- partition
//! needs a per-8x8 bitmask tied to dav1d's edge-index tree (`src/decode.rs`'s `decode_sb`);
//! inter/compound mode context needs dav1d's reference-motion-vector-candidate subsystem
//! (`src/refmvs.rs`, `rav1d_refmvs_find`) -- see `docs/DEVELOPMENT_PHASES.md` Phase 4's AV1
//! entropy-decoding note.
//!
//! Units throughout are 4x4 pixels (spec's context-array granularity).

/// Maps a raw intra prediction-mode symbol (0..=12, spec `y_mode`/`uv_mode` values -- matches
/// `bitvue_av1_codec::tile::PredictionMode`'s intra-variant declaration order exactly) to one of
/// 5 mode-context classes used to index the key-frame `kfym` CDF. Source: rav1d
/// `DAV1D_INTRA_MODE_CONTEXT` (`memorysafety/rav1d`, BSD-2-Clause, `src/tables.rs`).
const INTRA_MODE_CONTEXT: [u8; 13] = [0, 1, 2, 3, 4, 4, 4, 4, 3, 0, 1, 2, 0];

/// `DAV1D_AL_PART_CTX[dir][bl][bp]` -- the bitmask written into `above_partition`/`left_partition`
/// after a `partition` symbol `bp` is decided at level `bl`, one row per `bl` (0=128x128..4=8x8,
/// rav1d's `BlockLevel` convention -- opposite of `block_size_log2`, see `partition_bl`), one
/// column per `PartitionType` (0=None..9=Vert4, matches this crate's `PartitionType` enum
/// ordering exactly). `0xff` marks a partition type that's invalid at that level (never actually
/// written in practice, since callers only reach `set_partition` for partition types
/// `PartitionType::is_allowed` at the block's own size already validated). Source: rav1d
/// `DAV1D_AL_PART_CTX` (`memorysafety/rav1d`, BSD-2-Clause, `src/tables.rs`).
const PARTITION_CTX_TABLE: [[[u8; 10]; 5]; 2] = [
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

/// Maps `block_size_log2` (2..=7, this crate's CDF-lookup convention) to rav1d's `BlockLevel`
/// (0=128x128..4=8x8 -- opposite numeric order). Only valid for `block_size_log2` in 3..=7 (8x8
/// and up); 4x4 blocks (log2=2) never read a `partition` symbol at all, so have no `BlockLevel`.
pub fn partition_bl(block_size_log2: u8) -> u8 {
    7 - block_size_log2.clamp(3, 7)
}

/// Tracks above/left neighbor state for one tile, at 4x4-unit granularity.
///
/// `above_*` arrays span the tile's full width and persist for the whole tile (matches spec: the
/// above-context row is only reset at a new tile, not at every superblock row). `left_*` arrays
/// span the tile's full height and are addressed with the same absolute 4x4 coordinates as the
/// `above_*` ones -- real dav1d instead sizes its left-context column to one superblock and
/// addresses it with row-relative offsets (a memory/cache optimization for a real-time decoder);
/// this crate isn't performance-constrained the same way, so `start_superblock_row` simply clears
/// the whole array at each new superblock row, which is behaviorally equivalent (spec's
/// left-context only ever remembers state from within the current superblock row of a
/// raster-scanned tile) without needing relative-offset bookkeeping at every call site.
pub struct TileContext {
    above_skip: Vec<bool>,
    left_skip: Vec<bool>,
    /// Raw intra-mode symbol (0..=12) of the last block covering this 4x4 position, defaulting
    /// to `0` (`DC_PRED`) -- matches dav1d's `BlockContext::mode` default/edge behavior (an
    /// unwritten position reads as `DC_PRED`'s context class, per spec `INTRA_MODE_CONTEXT[0]`).
    above_mode: Vec<u8>,
    left_mode: Vec<u8>,
    /// `partition` context bitmask, one byte per 8x8 unit (not 4x4, like every other field here --
    /// see `PARTITION_CTX_TABLE`'s doc). Sized to half the tile's 4x4-unit extent (rounded up).
    above_partition: Vec<u8>,
    left_partition: Vec<u8>,
    /// `ref_frame()` (spec 5.11.25) context state, one entry per 4x4 unit -- mirrors rav1d
    /// `BlockContext`'s `intra`/`comp_type`/`ref[0]`/`ref[1]` fields (`src/env.rs`). `ref0`/`ref1`
    /// use rav1d's 0..=6 encoding (0..=3 = forward LAST/LAST2/LAST3/GOLDEN, 4..=6 = backward
    /// BWDREF/ALTREF2/ALTREF -- one less than `RefFrame`'s own discriminant, which reserves 0 for
    /// `Intra`; see `set_ref_frames`). All four `above_ref_*`/`left_ref_*` reader methods below
    /// take an explicit `have_top`/`have_left` (computed as `y4 > 0`/`x4 > 0` -- correct because,
    /// unlike `skip`/`mode`, several of rav1d's `ref_frame` context formulas (`get_comp_ctx`,
    /// `get_comp_dir_ctx`) return values for the "no neighbor" case that a mere default array
    /// value can't reproduce, and reading `ref0`/`ref1` as an array index in `uni_comp_ref_p1`'s
    /// formula needs a hard read guard, not a hopeful default) rather than the "default array
    /// value" shortcut `skip_context`/`intra_mode_context`/`partition_context` use.
    above_ref_intra: Vec<bool>,
    left_ref_intra: Vec<bool>,
    above_ref_comp: Vec<bool>,
    left_ref_comp: Vec<bool>,
    above_ref0: Vec<i8>,
    left_ref0: Vec<i8>,
    above_ref1: Vec<i8>,
    left_ref1: Vec<i8>,
}

impl TileContext {
    /// `tile_width_4x4`/`tile_height_4x4`: tile dimensions in 4x4 units.
    pub fn new(tile_width_4x4: u32, tile_height_4x4: u32) -> Self {
        Self {
            above_skip: vec![false; tile_width_4x4.max(1) as usize],
            left_skip: vec![false; tile_height_4x4.max(1) as usize],
            above_mode: vec![0; tile_width_4x4.max(1) as usize],
            left_mode: vec![0; tile_height_4x4.max(1) as usize],
            above_partition: vec![0; tile_width_4x4.div_ceil(2).max(1) as usize],
            left_partition: vec![0; tile_height_4x4.div_ceil(2).max(1) as usize],
            // Default `true` (intra), not `false` -- an unwritten slot must never look like a
            // real forward-ref (`ref0` default `0` = LAST) neighbor to the count-based context
            // functions. In real causal decode order this default is never actually read where
            // `have_top`/`have_left` is true (see the struct doc), but defaulting to `true` keeps
            // that invariant even under non-causal (e.g. test) access patterns, at zero cost.
            above_ref_intra: vec![true; tile_width_4x4.max(1) as usize],
            left_ref_intra: vec![true; tile_height_4x4.max(1) as usize],
            above_ref_comp: vec![false; tile_width_4x4.max(1) as usize],
            left_ref_comp: vec![false; tile_height_4x4.max(1) as usize],
            above_ref0: vec![0; tile_width_4x4.max(1) as usize],
            left_ref0: vec![0; tile_height_4x4.max(1) as usize],
            above_ref1: vec![0; tile_width_4x4.max(1) as usize],
            left_ref1: vec![0; tile_height_4x4.max(1) as usize],
        }
    }

    /// Reset the left-context arrays at the start of each new superblock row.
    pub fn start_superblock_row(&mut self) {
        self.left_skip.iter_mut().for_each(|v| *v = false);
        self.left_mode.iter_mut().for_each(|v| *v = 0);
        self.left_partition.iter_mut().for_each(|v| *v = 0);
        self.left_ref_intra.iter_mut().for_each(|v| *v = true);
        self.left_ref_comp.iter_mut().for_each(|v| *v = false);
        self.left_ref0.iter_mut().for_each(|v| *v = 0);
        self.left_ref1.iter_mut().for_each(|v| *v = 0);
    }

    /// `partition` context index (0..=3) for a block at absolute 8x8-unit position `(x8, y8)`,
    /// per spec/rav1d: `has_bl(above_partition[x8]) + 2*has_bl(left_partition[y8])`, where
    /// `has_bl` extracts the bit for the current level `bl` (0=128x128..4=8x8, see
    /// `partition_bl`) from the neighbor's stored bitmask.
    pub fn partition_context(&self, x8: u32, y8: u32, bl: u8) -> u8 {
        let has_bl = |v: u8| (v >> (4 - bl)) & 1;
        let above = self.above_partition.get(x8 as usize).copied().unwrap_or(0);
        let left = self.left_partition.get(y8 as usize).copied().unwrap_or(0);
        has_bl(above) + 2 * has_bl(left)
    }

    /// Record a decoded `partition` symbol across the block's 8x8-unit footprint (`hsz8` units
    /// wide/tall -- `2^(block_size_log2-3)`), for future context lookups. Per spec/rav1d, the
    /// written value isn't the raw symbol but `PARTITION_CTX_TABLE[dir][bl][partition_symbol]` --
    /// see that table's doc for why this is a lookup, not the symbol value itself.
    pub fn set_partition(&mut self, x8: u32, y8: u32, hsz8: u32, bl: u8, partition_symbol: u8) {
        let above_val = PARTITION_CTX_TABLE[0][bl as usize][partition_symbol as usize];
        let left_val = PARTITION_CTX_TABLE[1][bl as usize][partition_symbol as usize];
        let x_end = (x8 + hsz8).min(self.above_partition.len() as u32);
        for x in x8..x_end {
            self.above_partition[x as usize] = above_val;
        }
        let y_end = (y8 + hsz8).min(self.left_partition.len() as u32);
        for y in y8..y_end {
            self.left_partition[y as usize] = left_val;
        }
    }

    /// Key-frame `intra_mode` context: `(above_mode_class, left_mode_class)`, each 0..=4, for a
    /// block at absolute 4x4 position `(x4, y4)` -- per spec/dav1d:
    /// `INTRA_MODE_CONTEXT[above_mode[x4]]`, `INTRA_MODE_CONTEXT[left_mode[y4]]`.
    pub fn intra_mode_context(&self, x4: u32, y4: u32) -> (u8, u8) {
        let above_raw = self.above_mode.get(x4 as usize).copied().unwrap_or(0);
        let left_raw = self.left_mode.get(y4 as usize).copied().unwrap_or(0);
        (
            INTRA_MODE_CONTEXT[above_raw as usize],
            INTRA_MODE_CONTEXT[left_raw as usize],
        )
    }

    /// Record a decoded intra-mode symbol across the block's 4x4-unit footprint, for future
    /// context lookups.
    pub fn set_mode(&mut self, x4: u32, y4: u32, width_4x4: u32, height_4x4: u32, mode_symbol: u8) {
        let x_end = (x4 + width_4x4).min(self.above_mode.len() as u32);
        for x in x4..x_end {
            self.above_mode[x as usize] = mode_symbol;
        }
        let y_end = (y4 + height_4x4).min(self.left_mode.len() as u32);
        for y in y4..y_end {
            self.left_mode[y as usize] = mode_symbol;
        }
    }

    /// `skip` context index (0..=2) for a block at absolute 4x4 position `(x4, y4)` -- per
    /// spec/dav1d: `above_skip[x4] as u8 + left_skip[y4] as u8`.
    pub fn skip_context(&self, x4: u32, y4: u32) -> u8 {
        let above = self.above_skip.get(x4 as usize).copied().unwrap_or(false);
        let left = self.left_skip.get(y4 as usize).copied().unwrap_or(false);
        u8::from(above) + u8::from(left)
    }

    /// Record a decoded `skip` flag across the block's 4x4-unit footprint, for future context
    /// lookups -- per spec/dav1d, every 4x4 unit the block covers gets the same value in both the
    /// above-row and left-column arrays (`CaseSet::set_disjoint`/`case.set_disjoint` in rav1d).
    pub fn set_skip(&mut self, x4: u32, y4: u32, width_4x4: u32, height_4x4: u32, skip: bool) {
        let x_end = (x4 + width_4x4).min(self.above_skip.len() as u32);
        for x in x4..x_end {
            self.above_skip[x as usize] = skip;
        }
        let y_end = (y4 + height_4x4).min(self.left_skip.len() as u32);
        for y in y4..y_end {
            self.left_skip[y as usize] = skip;
        }
    }

    /// Record a decoded `ref_frame()` result across the block's 4x4-unit footprint, for future
    /// `ref_frame` context lookups. `ref0`/`ref1` use rav1d's 0..=6 encoding (see the struct
    /// fields' doc) -- pass `ref1 = -1` for single-reference blocks (`comp = false`).
    #[allow(clippy::too_many_arguments)]
    pub fn set_ref_frames(
        &mut self,
        x4: u32,
        y4: u32,
        width_4x4: u32,
        height_4x4: u32,
        is_intra: bool,
        comp: bool,
        ref0: i8,
        ref1: i8,
    ) {
        let x_end = (x4 + width_4x4).min(self.above_ref_intra.len() as u32);
        for x in x4..x_end {
            self.above_ref_intra[x as usize] = is_intra;
            self.above_ref_comp[x as usize] = comp;
            self.above_ref0[x as usize] = ref0;
            self.above_ref1[x as usize] = ref1;
        }
        let y_end = (y4 + height_4x4).min(self.left_ref_intra.len() as u32);
        for y in y4..y_end {
            self.left_ref_intra[y as usize] = is_intra;
            self.left_ref_comp[y as usize] = comp;
            self.left_ref0[y as usize] = ref0;
            self.left_ref1[y as usize] = ref1;
        }
    }

    /// `comp_mode` context (0..=4) -- whether this block is likely single- or compound-reference,
    /// derived from the above/left neighbors' reference counts. Source: rav1d `get_comp_ctx`
    /// (`memorysafety/rav1d`, BSD-2-Clause, `src/env.rs`).
    pub fn comp_mode_context(&self, x4: u32, y4: u32) -> u8 {
        let have_top = y4 > 0;
        let have_left = x4 > 0;
        let xi = x4 as usize;
        let yi = y4 as usize;
        let a_comp = || self.above_ref_comp.get(xi).copied().unwrap_or(false);
        let l_comp = || self.left_ref_comp.get(yi).copied().unwrap_or(false);
        let a_ref0 = || self.above_ref0.get(xi).copied().unwrap_or(0);
        let l_ref0 = || self.left_ref0.get(yi).copied().unwrap_or(0);

        if have_top {
            if have_left {
                if a_comp() {
                    if l_comp() {
                        4
                    } else {
                        2 + u8::from(l_ref0() >= 4)
                    }
                } else if l_comp() {
                    2 + u8::from(a_ref0() >= 4)
                } else {
                    u8::from((l_ref0() >= 4) ^ (a_ref0() >= 4))
                }
            } else if a_comp() {
                3
            } else {
                u8::from(a_ref0() >= 4)
            }
        } else if have_left {
            if l_comp() {
                3
            } else {
                u8::from(l_ref0() >= 4)
            }
        } else {
            1
        }
    }

    /// `comp_ref_type` context (0..=4) -- unidirectional vs bidirectional compound likelihood.
    /// Source: rav1d `get_comp_dir_ctx` (`memorysafety/rav1d`, BSD-2-Clause, `src/env.rs`), the
    /// most heavily branched of the `ref_frame` context functions.
    pub fn comp_ref_type_context(&self, x4: u32, y4: u32) -> u8 {
        let have_top = y4 > 0;
        let have_left = x4 > 0;
        let xi = x4 as usize;
        let yi = y4 as usize;
        let (a_intra, a_comp, a_ref0, a_ref1) = (
            self.above_ref_intra.get(xi).copied().unwrap_or(true),
            self.above_ref_comp.get(xi).copied().unwrap_or(false),
            self.above_ref0.get(xi).copied().unwrap_or(0),
            self.above_ref1.get(xi).copied().unwrap_or(0),
        );
        let (l_intra, l_comp, l_ref0, l_ref1) = (
            self.left_ref_intra.get(yi).copied().unwrap_or(true),
            self.left_ref_comp.get(yi).copied().unwrap_or(false),
            self.left_ref0.get(yi).copied().unwrap_or(0),
            self.left_ref1.get(yi).copied().unwrap_or(0),
        );
        let has_uni_comp = |ref0: i8, ref1: i8| (ref0 < 4) == (ref1 < 4);

        if have_top && have_left {
            if a_intra && l_intra {
                return 2;
            }
            if a_intra || l_intra {
                let (edge_comp, edge_ref0, edge_ref1) = if a_intra {
                    (l_comp, l_ref0, l_ref1)
                } else {
                    (a_comp, a_ref0, a_ref1)
                };
                if !edge_comp {
                    return 2;
                }
                return 1 + 2 * u8::from(has_uni_comp(edge_ref0, edge_ref1));
            }

            if !a_comp && !l_comp {
                1 + 2 * u8::from((a_ref0 >= 4) == (l_ref0 >= 4))
            } else if !a_comp || !l_comp {
                let (edge_ref0, edge_ref1) = if a_comp {
                    (a_ref0, a_ref1)
                } else {
                    (l_ref0, l_ref1)
                };
                if !has_uni_comp(edge_ref0, edge_ref1) {
                    1
                } else {
                    3 + u8::from((a_ref0 >= 4) == (l_ref0 >= 4))
                }
            } else {
                let a_uni = has_uni_comp(a_ref0, a_ref1);
                let l_uni = has_uni_comp(l_ref0, l_ref1);
                if !a_uni && !l_uni {
                    0
                } else if !a_uni || !l_uni {
                    2
                } else {
                    3 + u8::from((a_ref0 == 4) == (l_ref0 == 4))
                }
            }
        } else if have_top || have_left {
            let (edge_intra, edge_comp, edge_ref0, edge_ref1) = if have_left {
                (l_intra, l_comp, l_ref0, l_ref1)
            } else {
                (a_intra, a_comp, a_ref0, a_ref1)
            };
            if edge_intra || !edge_comp {
                2
            } else {
                4 * u8::from(has_uni_comp(edge_ref0, edge_ref1))
            }
        } else {
            2
        }
    }

    /// Shared neighbor-counting core for `single_ref_p1`/`single_ref_p2`/`single_ref_p3`/
    /// `single_ref_p4`/`single_ref_p5`/`single_ref_p6` and their compound-CDF reuses
    /// (`uni_comp_ref`/`comp_ref`/`comp_ref_p1`/`comp_ref_p2`/`comp_bwdref`/`comp_bwdref_p1`).
    /// `bucket` maps a neighbor's raw rav1d-encoded ref value (0..=6) to `Some(bucket index)` to
    /// count, or `None` to ignore it (mirrors each rav1d function's own `if ref0 < N`/`>= N`/
    /// `^ N < N` guard before indexing its `cnt` array).
    fn ref_count_context(
        &self,
        x4: u32,
        y4: u32,
        num_buckets: usize,
        bucket: impl Fn(i8) -> Option<usize>,
    ) -> [u8; 4] {
        let have_top = y4 > 0;
        let have_left = x4 > 0;
        let xi = x4 as usize;
        let yi = y4 as usize;
        let mut cnt = [0u8; 4];
        debug_assert!(num_buckets <= 4);
        let above_intra = self.above_ref_intra.get(xi).copied().unwrap_or(true);
        let left_intra = self.left_ref_intra.get(yi).copied().unwrap_or(true);

        if have_top && !above_intra {
            if let Some(b) = bucket(self.above_ref0.get(xi).copied().unwrap_or(0)) {
                cnt[b] += 1;
            }
            if self.above_ref_comp.get(xi).copied().unwrap_or(false) {
                if let Some(b) = bucket(self.above_ref1.get(xi).copied().unwrap_or(0)) {
                    cnt[b] += 1;
                }
            }
        }
        if have_left && !left_intra {
            if let Some(b) = bucket(self.left_ref0.get(yi).copied().unwrap_or(0)) {
                cnt[b] += 1;
            }
            if self.left_ref_comp.get(yi).copied().unwrap_or(false) {
                if let Some(b) = bucket(self.left_ref1.get(yi).copied().unwrap_or(0)) {
                    cnt[b] += 1;
                }
            }
        }
        cnt
    }

    /// `single_ref_p1`/`uni_comp_ref` context (0..=2) -- forward vs backward reference-group
    /// likelihood. Source: rav1d `av1_get_ref_ctx` (`memorysafety/rav1d`, BSD-2-Clause,
    /// `src/env.rs`).
    pub fn single_ref_p1_context(&self, x4: u32, y4: u32) -> u8 {
        let cnt = self.ref_count_context(x4, y4, 2, |r| Some(usize::from(r >= 4)));
        cmp_counts(cnt[0], cnt[1])
    }

    /// `single_ref_p3`/`comp_ref` context (0..=2) -- among forward refs, LAST/LAST2 vs
    /// LAST3/GOLDEN. Source: rav1d `av1_get_fwd_ref_ctx` (`memorysafety/rav1d`, BSD-2-Clause,
    /// `src/env.rs`).
    pub fn single_ref_p3_context(&self, x4: u32, y4: u32) -> u8 {
        let cnt =
            self.ref_count_context(x4, y4, 4, |r| if r < 4 { Some(r as usize) } else { None });
        cmp_counts(cnt[0] + cnt[1], cnt[2] + cnt[3])
    }

    /// `single_ref_p4`/`comp_ref_p1` context (0..=2) -- LAST vs LAST2. Source: rav1d
    /// `av1_get_fwd_ref_1_ctx` (`memorysafety/rav1d`, BSD-2-Clause, `src/env.rs`).
    pub fn single_ref_p4_context(&self, x4: u32, y4: u32) -> u8 {
        let cnt =
            self.ref_count_context(x4, y4, 2, |r| if r < 2 { Some(r as usize) } else { None });
        cmp_counts(cnt[0], cnt[1])
    }

    /// `single_ref_p5`/`comp_ref_p2`/`uni_comp_ref_p2` context (0..=2) -- LAST3 vs GOLDEN.
    /// Source: rav1d `av1_get_fwd_ref_2_ctx` (`memorysafety/rav1d`, BSD-2-Clause, `src/env.rs`).
    pub fn single_ref_p5_context(&self, x4: u32, y4: u32) -> u8 {
        let cnt = self.ref_count_context(x4, y4, 2, |r| {
            if (r ^ 2) < 2 {
                Some((r - 2) as usize)
            } else {
                None
            }
        });
        cmp_counts(cnt[0], cnt[1])
    }

    /// `single_ref_p2`/`comp_bwdref` context (0..=2) -- among backward refs, BWDREF/ALTREF2 vs
    /// ALTREF. Source: rav1d `av1_get_bwd_ref_ctx` (`memorysafety/rav1d`, BSD-2-Clause,
    /// `src/env.rs`).
    pub fn single_ref_p2_context(&self, x4: u32, y4: u32) -> u8 {
        let cnt = self.ref_count_context(x4, y4, 3, |r| {
            if r >= 4 {
                Some((r - 4) as usize)
            } else {
                None
            }
        });
        cmp_counts(cnt[0] + cnt[1], cnt[2])
    }

    /// `single_ref_p6`/`comp_bwdref_p1` context (0..=2) -- BWDREF vs ALTREF2. Source: rav1d
    /// `av1_get_bwd_ref_1_ctx` (`memorysafety/rav1d`, BSD-2-Clause, `src/env.rs`).
    pub fn single_ref_p6_context(&self, x4: u32, y4: u32) -> u8 {
        let cnt = self.ref_count_context(x4, y4, 3, |r| {
            if r >= 4 {
                Some((r - 4) as usize)
            } else {
                None
            }
        });
        cmp_counts(cnt[0], cnt[1])
    }

    /// `uni_comp_ref_p1` context (0..=2) -- LAST2 vs LAST3/GOLDEN (grouped), among unidirectional
    /// compound candidates. Source: rav1d `av1_get_uni_p1_ctx` (`memorysafety/rav1d`,
    /// BSD-2-Clause, `src/env.rs`) -- the one function of the nine whose bucket count (3, indices
    /// 0..=2 for ref values 1..=3) doesn't match its `cnt` array width (also 3): unlike the other
    /// `ref_count_context` uses, out-of-range ref values here are silently dropped (rav1d's
    /// `cnt.get_mut(...)` returns `None` and the increment is skipped) rather than counted, since
    /// LAST (ref value 0) legitimately isn't part of this context's model at all.
    pub fn uni_comp_ref_p1_context(&self, x4: u32, y4: u32) -> u8 {
        let mut cnt = self.ref_count_context(x4, y4, 3, |r| {
            let idx = r - 1;
            if (0..3).contains(&idx) {
                Some(idx as usize)
            } else {
                None
            }
        });
        cnt[1] += cnt[2];
        cmp_counts(cnt[0], cnt[1])
    }
}

/// 3-way ordinal comparison of two neighbor reference-frame counts, used by every count-based
/// `ref_frame` context function above. Source: rav1d `cmp_counts` (`memorysafety/rav1d`,
/// BSD-2-Clause, `src/env.rs`).
fn cmp_counts(c1: u8, c2: u8) -> u8 {
    match c1.cmp(&c2) {
        std::cmp::Ordering::Less => 0,
        std::cmp::Ordering::Equal => 1,
        std::cmp::Ordering::Greater => 2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skip_context_starts_at_zero_with_no_neighbors() {
        let ctx = TileContext::new(16, 16);
        assert_eq!(ctx.skip_context(0, 0), 0);
    }

    #[test]
    fn test_skip_context_reflects_above_neighbor() {
        let mut ctx = TileContext::new(16, 16);
        ctx.set_skip(0, 0, 4, 4, true);
        // A block directly to the right, same 4x4 row -- shares the above-context column but not
        // the left-context row, so only the "above" contribution applies here since we're
        // querying the SAME above-row position that was written.
        assert_eq!(ctx.skip_context(0, 5), 1);
    }

    #[test]
    fn test_skip_context_reflects_left_neighbor() {
        let mut ctx = TileContext::new(16, 16);
        ctx.set_skip(0, 0, 4, 4, true);
        assert_eq!(ctx.skip_context(5, 0), 1);
    }

    #[test]
    fn test_skip_context_sums_both_neighbors() {
        let mut ctx = TileContext::new(16, 16);
        ctx.set_skip(0, 0, 4, 4, true);
        assert_eq!(ctx.skip_context(0, 0), 2);
    }

    #[test]
    fn test_start_superblock_row_resets_left_but_not_above() {
        let mut ctx = TileContext::new(16, 16);
        ctx.set_skip(0, 0, 4, 4, true);
        ctx.start_superblock_row();
        // Left context cleared...
        assert_eq!(ctx.skip_context(5, 0), 0);
        // ...but above context (tile-lifetime) persists.
        assert_eq!(ctx.skip_context(0, 5), 1);
    }

    #[test]
    fn test_set_skip_clamps_to_array_bounds() {
        // A block whose footprint would run past the tile/superblock edge must not panic.
        let mut ctx = TileContext::new(4, 4);
        ctx.set_skip(2, 2, 100, 100, true);
        assert_eq!(ctx.skip_context(3, 3), 2);
    }

    #[test]
    fn test_intra_mode_context_defaults_to_dc_pred_class() {
        // No neighbors written yet -- both default to raw mode 0 (DC_PRED), class 0.
        let ctx = TileContext::new(16, 16);
        assert_eq!(ctx.intra_mode_context(0, 0), (0, 0));
    }

    #[test]
    fn test_intra_mode_context_maps_through_intra_mode_context_table() {
        let mut ctx = TileContext::new(16, 16);
        // Raw mode 8 (D67_PRED) -> class 3 per INTRA_MODE_CONTEXT.
        ctx.set_mode(0, 0, 4, 4, 8);
        assert_eq!(ctx.intra_mode_context(0, 5).0, 3); // above contribution only
        assert_eq!(ctx.intra_mode_context(5, 0).1, 3); // left contribution only
    }

    #[test]
    fn test_intra_mode_context_combines_distinct_above_and_left_classes() {
        let mut ctx = TileContext::new(16, 16);
        // A block at (x4=0, y4=0..4) sets above_mode[0..4] AND left_mode[0..4] -- to isolate the
        // two contributions for a query at (x4=0, y4=4), use two non-overlapping blocks: one
        // covering only above_mode[0] (a block above-and-to-the-left, x4=0 y4=0..4), one covering
        // only left_mode[4] (a block directly above the query's row, y4=4 x4=4..8 -- outside the
        // query's own x4=0 column, so it can't also perturb above_mode[0]).
        ctx.set_mode(0, 0, 4, 4, 8); // above_mode[0..4]=8, left_mode[0..4]=8
        ctx.set_mode(4, 4, 4, 4, 4); // above_mode[4..8]=4, left_mode[4..8]=4
                                     // Query (0, 4): above_mode[0]=8 (class 3, untouched by the second call), left_mode[4]=4
                                     // (class 4, set by the second call).
        assert_eq!(ctx.intra_mode_context(0, 4), (3, 4));
    }

    #[test]
    fn test_start_superblock_row_resets_left_mode_but_not_above_mode() {
        let mut ctx = TileContext::new(16, 16);
        ctx.set_mode(0, 0, 4, 4, 8);
        ctx.start_superblock_row();
        assert_eq!(ctx.intra_mode_context(5, 0).1, 0); // left reset to DC_PRED class
        assert_eq!(ctx.intra_mode_context(0, 5).0, 3); // above persists
    }

    #[test]
    fn test_intra_mode_context_table_matches_rav1d_dav1d_intra_mode_context() {
        // Source: rav1d `DAV1D_INTRA_MODE_CONTEXT` (`src/tables.rs`). Pinning the exact table
        // here catches an accidental edit independent of the round-trip tests above.
        assert_eq!(INTRA_MODE_CONTEXT, [0, 1, 2, 3, 4, 4, 4, 4, 3, 0, 1, 2, 0]);
    }

    #[test]
    fn test_partition_context_starts_at_zero_with_no_neighbors() {
        let ctx = TileContext::new(16, 16);
        assert_eq!(ctx.partition_context(0, 0, 4), 0);
    }

    #[test]
    fn test_partition_context_reflects_above_split_at_8x8() {
        let mut ctx = TileContext::new(16, 16);
        // Split (symbol 3) at 8x8 (bl=4) sets bit 0 of PARTITION_CTX_TABLE's value (0x1f), which
        // `has_bl` for bl=4 reads directly off bit 0.
        ctx.set_partition(0, 0, 1, 4, 3);
        // Query at the same above-column (x8=0), different row -- above contribution only.
        assert_eq!(ctx.partition_context(0, 5, 4), 1);
    }

    #[test]
    fn test_partition_context_reflects_left_split_at_8x8() {
        let mut ctx = TileContext::new(16, 16);
        ctx.set_partition(0, 0, 1, 4, 3);
        assert_eq!(ctx.partition_context(5, 0, 4), 2);
    }

    #[test]
    fn test_partition_context_sums_both_neighbors() {
        let mut ctx = TileContext::new(16, 16);
        ctx.set_partition(0, 0, 1, 4, 3);
        assert_eq!(ctx.partition_context(0, 0, 4), 3);
    }

    #[test]
    fn test_partition_context_none_clears_the_bit() {
        let mut ctx = TileContext::new(16, 16);
        ctx.set_partition(0, 0, 1, 4, 3); // Split first
        ctx.set_partition(0, 0, 1, 4, 0); // None overwrites (0x1e has bit 0 clear)
        assert_eq!(ctx.partition_context(0, 5, 4), 0);
    }

    #[test]
    fn test_start_superblock_row_resets_left_partition_but_not_above() {
        let mut ctx = TileContext::new(16, 16);
        ctx.set_partition(0, 0, 1, 4, 3);
        ctx.start_superblock_row();
        assert_eq!(ctx.partition_context(5, 0, 4), 0); // left reset
        assert_eq!(ctx.partition_context(0, 5, 4), 1); // above persists
    }

    #[test]
    fn test_set_partition_clamps_to_array_bounds() {
        let mut ctx = TileContext::new(4, 4);
        ctx.set_partition(0, 0, 100, 4, 3); // footprint far exceeds the array; must not panic
        assert_eq!(ctx.partition_context(0, 0, 4), 3);
    }

    #[test]
    fn test_partition_ctx_table_matches_rav1d_dav1d_al_part_ctx() {
        // Source: rav1d `DAV1D_AL_PART_CTX` (`src/tables.rs`). Pinning both extremes (128x128's
        // mostly-0xff row, and 8x8's real row) catches an accidental edit.
        assert_eq!(
            PARTITION_CTX_TABLE[0][0],
            [0x00, 0x00, 0x10, 0xff, 0x00, 0x10, 0x10, 0x10, 0xff, 0xff]
        );
        assert_eq!(
            PARTITION_CTX_TABLE[1][4],
            [0x1e, 0x1f, 0x1e, 0x1f, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff]
        );
    }

    #[test]
    fn test_partition_bl_maps_block_size_log2_to_rav1d_block_level() {
        assert_eq!(partition_bl(7), 0); // 128x128 -> rav1d BlockLevel 0
        assert_eq!(partition_bl(3), 4); // 8x8 -> rav1d BlockLevel 4
        assert_eq!(partition_bl(5), 2); // 32x32 -> rav1d BlockLevel 2
    }

    // ref_frame context tests. `ref0`/`ref1` values below use rav1d's raw 0..=6 encoding (see
    // `TileContext`'s struct doc): 0=LAST, 1=LAST2, 2=LAST3, 3=GOLDEN, 4=BWDREF, 5=ALTREF2,
    // 6=ALTREF.

    #[test]
    fn test_comp_mode_context_no_neighbors_returns_one() {
        // Neither have_top nor have_left -- rav1d's `get_comp_ctx` early-returns 1.
        let ctx = TileContext::new(16, 16);
        assert_eq!(ctx.comp_mode_context(0, 0), 1);
    }

    #[test]
    fn test_comp_mode_context_top_only_forward_ref_is_zero() {
        let mut ctx = TileContext::new(16, 16);
        ctx.set_ref_frames(0, 0, 4, 4, false, false, 0, -1); // LAST, single-ref
        assert_eq!(ctx.comp_mode_context(0, 5), 0); // above only, ref0 < 4
    }

    #[test]
    fn test_comp_mode_context_top_only_backward_ref_is_one() {
        let mut ctx = TileContext::new(16, 16);
        ctx.set_ref_frames(0, 0, 4, 4, false, false, 4, -1); // BWDREF, single-ref
        assert_eq!(ctx.comp_mode_context(0, 5), 1); // above only, ref0 >= 4
    }

    #[test]
    fn test_comp_mode_context_left_only_compound_is_three() {
        let mut ctx = TileContext::new(16, 16);
        ctx.set_ref_frames(0, 0, 4, 4, false, true, 0, 4);
        assert_eq!(ctx.comp_mode_context(5, 0), 3); // left only, compound
    }

    #[test]
    fn test_comp_mode_context_both_compound_returns_four() {
        let mut ctx = TileContext::new(16, 16);
        // To get both have_top and have_left true at the query position (5, 5), write the above
        // contribution at column x4=5 (via a block at (5, 0)) and the left contribution at row
        // y4=5 (via a block at (0, 5)) -- each call also writes the other array's irrelevant
        // range, which doesn't affect this query.
        ctx.set_ref_frames(5, 0, 4, 4, false, true, 0, 4); // above_comp[5..9]=true
        ctx.set_ref_frames(0, 5, 4, 4, false, true, 0, 4); // left_comp[5..9]=true
        assert_eq!(ctx.comp_mode_context(5, 5), 4);
    }

    #[test]
    fn test_comp_ref_type_context_no_neighbors_returns_two() {
        let ctx = TileContext::new(16, 16);
        assert_eq!(ctx.comp_ref_type_context(0, 0), 2);
    }

    #[test]
    fn test_comp_ref_type_context_both_intra_returns_two() {
        let mut ctx = TileContext::new(16, 16);
        // Same both-neighbor query-position trick as `test_comp_mode_context_both_compound...`.
        ctx.set_ref_frames(5, 0, 4, 4, true, false, 0, -1); // above_ref_intra[5..9]=true
        ctx.set_ref_frames(0, 5, 4, 4, true, false, 0, -1); // left_ref_intra[5..9]=true
        assert_eq!(ctx.comp_ref_type_context(5, 5), 2);
    }

    #[test]
    fn test_single_ref_p1_context_no_neighbors_is_tie() {
        // cmp_counts(0, 0) == 1 (Equal).
        let ctx = TileContext::new(16, 16);
        assert_eq!(ctx.single_ref_p1_context(0, 0), 1);
    }

    #[test]
    fn test_single_ref_p1_context_counts_forward_vs_backward() {
        let mut ctx = TileContext::new(16, 16);
        // Both-neighbor query-position trick (see `test_comp_mode_context_both_compound...`):
        // above contribution at column x4=5 (query x4), left contribution at row y4=5 (query y4).
        ctx.set_ref_frames(5, 0, 4, 4, false, false, 0, -1); // LAST (forward), above[5..9]
        ctx.set_ref_frames(0, 5, 4, 4, false, false, 4, -1); // BWDREF (backward), left[5..9]
                                                             // cnt[0]=1 (forward), cnt[1]=1 (backward) -> cmp_counts(1,1) == 1 (Equal).
        assert_eq!(ctx.single_ref_p1_context(5, 5), 1);
    }

    #[test]
    fn test_single_ref_p1_context_both_forward_is_greater() {
        let mut ctx = TileContext::new(16, 16);
        ctx.set_ref_frames(5, 0, 4, 4, false, false, 0, -1); // above[5..9]
        ctx.set_ref_frames(0, 5, 4, 4, false, false, 1, -1); // left[5..9]
                                                             // cnt[0]=2, cnt[1]=0 -> cmp_counts(2,0) == 2 (Greater).
        assert_eq!(ctx.single_ref_p1_context(5, 5), 2);
    }

    #[test]
    fn test_single_ref_p1_context_excludes_intra_neighbor() {
        let mut ctx = TileContext::new(16, 16);
        // An intra neighbor must not contribute to either bucket.
        ctx.set_ref_frames(0, 0, 4, 4, true, false, 0, -1);
        assert_eq!(ctx.single_ref_p1_context(0, 5), 1); // still a tie (0 vs 0)
    }

    #[test]
    fn test_single_ref_p3_context_groups_last_last2_vs_last3_golden() {
        let mut ctx = TileContext::new(16, 16);
        ctx.set_ref_frames(5, 0, 4, 4, false, false, 0, -1); // LAST -> bucket 0, above[5..9]
        ctx.set_ref_frames(0, 5, 4, 4, false, false, 3, -1); // GOLDEN -> bucket 3, left[5..9]
                                                             // cnt[0]+cnt[1]=1, cnt[2]+cnt[3]=1 -> tie.
        assert_eq!(ctx.single_ref_p3_context(5, 5), 1);
    }

    #[test]
    fn test_single_ref_p5_context_last3_vs_golden() {
        let mut ctx = TileContext::new(16, 16);
        ctx.set_ref_frames(0, 0, 4, 4, false, false, 3, -1); // GOLDEN -> bucket 1
                                                             // cnt[0]=0 (LAST3), cnt[1]=1 (GOLDEN) -> cmp_counts(0,1) == 0 (Less).
        assert_eq!(ctx.single_ref_p5_context(0, 5), 0);
    }

    #[test]
    fn test_uni_comp_ref_p1_context_groups_last2_vs_last3_golden() {
        let mut ctx = TileContext::new(16, 16);
        ctx.set_ref_frames(0, 0, 4, 4, false, false, 1, -1); // LAST2 -> bucket 0
                                                             // cnt[0]=1, cnt[1]=0 (bucket 1/2 empty, merged via cnt[1]+=cnt[2]) -> cmp_counts(1,0)=2.
        assert_eq!(ctx.uni_comp_ref_p1_context(0, 5), 2);
    }

    #[test]
    fn test_uni_comp_ref_p1_context_excludes_last() {
        let mut ctx = TileContext::new(16, 16);
        // LAST (ref value 0) isn't part of this context's model -- rav1d's `cnt.get_mut(-1)`
        // silently drops it (see `uni_comp_ref_p1_context`'s doc).
        ctx.set_ref_frames(0, 0, 4, 4, false, false, 0, -1);
        assert_eq!(ctx.uni_comp_ref_p1_context(0, 5), 1); // 0 vs 0, tie
    }

    #[test]
    fn test_start_superblock_row_resets_left_ref_state() {
        let mut ctx = TileContext::new(16, 16);
        // Query at x4=5 (have_left=true), y4=0 (have_top=false) so only the left contribution
        // matters; writing at (0, 0) sets left_ref0[0..4], covering the query's y4=0.
        ctx.set_ref_frames(0, 0, 4, 4, false, false, 4, -1); // BWDREF, left_ref0[0..4]=4
        assert_eq!(ctx.single_ref_p1_context(5, 0), 0); // Less: 0 forward vs 1 backward
        ctx.start_superblock_row();
        // After reset, left_ref_intra defaults back to `true` (see `start_superblock_row`'s doc),
        // so the stale backward-ref value at left_ref0[0] is excluded again -- back to a tie.
        assert_eq!(ctx.single_ref_p1_context(5, 0), 1);
    }
}
