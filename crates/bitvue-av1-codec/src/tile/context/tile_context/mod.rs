//! Above/left neighbour state of every syntax-element family except the reference-motion-vector
//! map (see `spatial_ref`). The struct is here with its construction; the `impl` blocks live in
//! one file per family:
//!
//! `skip` (skip / skip_mode), `mode` (key-frame intra mode), `partition`, `tx` (tx size and
//! var-tx), `segment` (segment id / seg_pred), `palette`, `residual` (txb_skip / dc_sign, luma and
//! chroma), `inter_block` (intra flag, compound type, interpolation filter), `ref_frames`
//! (`comp_mode` / `ref_frame` contexts) and `ref_mvs` (delegation to the `SpatialRefContext`).

mod inter_block;
mod mode;
mod palette;
mod partition;
mod ref_frames;
mod ref_mvs;
mod residual;
mod segment;
mod skip;
mod tx;

use super::spatial_ref::SpatialRefContext;
use super::tables::VAR_TX_UNSET;

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
    /// Loop-restoration coefficient references per plane (see `tile::restoration`); reset to the
    /// spec defaults with every new tile, i.e. every `TileContext`.
    pub(crate) restoration_refs: [crate::tile::RestorationRef; 3],
    above_skip: Vec<bool>,
    left_skip: Vec<bool>,
    /// `skip_mode` (spec 5.11.5) above/left context -- real dav1d `t->a->skip_mode[bx4]`/
    /// `t->l.skip_mode[by4]`, same direct-sum-no-have-top/left-branch shape as `above_skip`/
    /// `left_skip` (`skip_mode_context`'s doc).
    above_skip_mode: Vec<bool>,
    left_skip_mode: Vec<bool>,
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
    /// `comp_type` (spec 5.11.28) above/left context, real dav1d `CompInterType` numeric encoding
    /// (`NONE=0, WEIGHTED_AVG=1, AVG=2, SEG=3, WEDGE=4` -- verified via the `>=` comparisons
    /// `get_mask_comp_ctx`/`get_jnt_comp_ctx` make against `COMP_INTER_AVG`/`COMP_INTER_SEG`, not
    /// assumed) -- feeds `mask_comp_context`/`jnt_comp_context`. Default `0` (NONE), matching
    /// dav1d's own tile-start reset.
    above_comp_type: Vec<u8>,
    left_comp_type: Vec<u8>,
    /// `filter[dir]` (spec 5.11.30) above/left context -- last chosen subpel filter per direction
    /// (`[0]`=horizontal, `[1]`=vertical), sentinel `3` (`DAV1D_N_SWITCHABLE_FILTERS`) = "no
    /// filter recorded here" (matches real dav1d's own sentinel, `get_filter_ctx`'s doc).
    above_filter: [Vec<u8>; 2],
    left_filter: [Vec<u8>; 2],
    /// `inter_mode`/`compound_mode` context -- see `SpatialRefContext`'s doc (full-grid, not
    /// above/left arrays, and never reset per superblock row).
    spatial_ref: SpatialRefContext,
    /// Intra `tx_size()` (spec 5.11.15/16) context state: the resolved transform's size *class*
    /// (0..=4, `TxSize`'s own discriminant order -- matches rav1d's `TxfmInfo.lw`/`.lh` exactly
    /// for a square transform, so no separate log2 conversion is needed). Defaults to `-1`
    /// (`i8`, never a real class) so an unwritten position can never satisfy the context
    /// formula's `>=` comparison -- matches rav1d's own tile-start reset value for this array
    /// (`memorysafety/rav1d`, BSD-2-Clause, `src/decode.rs`'s `tx_intra` reset), unlike
    /// `above_mode`'s "default reads as a real class" shortcut.
    above_tx_class: Vec<i8>,
    left_tx_class: Vec<i8>,
    /// `txb_skip`/`dc_sign` (spec 8.3.2 `get_txb_skip_ctx`/`get_dc_sign_ctx`) context state, one
    /// entry per 4x4 unit -- unpacked equivalent of rav1d's single combined context byte
    /// (`min(cul_level,63) | (dc_sign_category<<6)`, `memorysafety/rav1d`'s C predecessor
    /// `src/recon_tmpl.c`'s `get_skip_ctx`/`get_dc_sign_ctx`/write side around `decode_coefs`).
    /// `cul_level`: `min(63, sum of absolute coefficient levels)` for the last transform block
    /// covering this position, defaulting to `0` (no coefficients seen). `dc_sign_category`:
    /// `0`=negative DC sign, `1`=neutral (no DC coefficient, or an all-zero/`txb_skip` block),
    /// `2`=positive DC sign -- defaults to `1`, matching rav1d's `0x40` tile/row-boundary reset
    /// value (`>> 6 == 1`). Only ever written/read for key-frame, non-IntraBC coding units (see
    /// `parse_coding_unit`) -- inter blocks and IntraBC still use a heuristic `tx_size`
    /// (`TxSize::from_dimensions`), so their transform-block boundaries don't reliably match the
    /// real encoder's; deriving neighbor context from them previously caused real decode
    /// corruption (see `SymbolDecoder::read_residual_block`'s doc), so those CUs keep the older
    /// fixed-context-0 fallback and never touch these arrays.
    above_cul_level: Vec<u8>,
    left_cul_level: Vec<u8>,
    above_dc_sign_category: Vec<u8>,
    left_dc_sign_category: Vec<u8>,
    /// Inter/IntraBC `read_var_tx_size()` (spec 5.11.17/18, `SymbolDecoder::read_txfm_split`)
    /// above/left context: the leaf transform size *class* last written at each position (0..=4,
    /// `TxSize`'s discriminant order). Defaults to `0` (`TxSize::Tx4x4`'s own class, matching
    /// rav1d's `TxfmSize` `#[derive(Default)]` -- its `.tx` array's own tile-start reset value,
    /// `memorysafety/rav1d`'s `env.rs` `BlockContext.tx` field): `read_txfm_split`'s context
    /// formula (`stored < candidate`) only ever evaluates `candidate` at sizes `>4x4` (spec only
    /// reads `txfm_split` when the candidate is bigger than `TX_4X4`), so a `0` default and any
    /// sentinel strictly below every real class are behaviorally identical for every case that
    /// matters here. Distinct from `above_tx_class`/`left_tx_class` (intra `tx_size()`'s own
    /// separate context array -- real rav1d keeps these as two genuinely independent fields,
    /// `tx_intra` vs `tx`, not one shared array). Currently only written by var-tx leaves (inter,
    /// non-IntraBC -- see `parse_coding_unit`'s doc), never by intra `tx_size()`, so an inter CU's
    /// context lookup against an intra above/left neighbor sees the unwritten default rather than
    /// that neighbor's real chosen size -- a known context-derivation approximation (doesn't
    /// affect bit-position sync, only which adaptive CDF entry gets selected), not a bug.
    above_var_tx: Vec<i8>,
    left_var_tx: Vec<i8>,
    /// Chroma-plane (`[0]`=U, `[1]`=V, real separate arrays per `recon_tmpl.c`'s `t->a->ccoef[pl]`/
    /// `t->l.ccoef[pl]`, not shared between planes) counterparts to `above_cul_level`/
    /// `left_cul_level`/`above_dc_sign_category`/`left_dc_sign_category`, indexed at the same
    /// coordinate scale as the luma arrays (a chroma tile's position is tracked as its luma-CU
    /// origin's `x4/2`/`y4/2`, not a truly independent chroma-plane grid -- an approximation
    /// consistent with this crate's other chroma simplifications, harmless here since only
    /// relative above/left adjacency matters, not absolute physical distance). Added to fix a real
    /// desync bug: `read_chroma_residual_block` previously had no `ctx` parameter at all, so every
    /// chroma transform block in a CU (up to 4, for a 128x128 luma block's 2x2-tiled 64x64 chroma
    /// plane) hit the exact same global CDF slot and over-adapted it relative to what a real
    /// encoder (using real per-position context, spec/rav1d `get_skip_ctx`'s chroma branch)
    /// assumes -- see `TileContext::txb_skip_context_chroma`'s doc.
    above_cul_level_chroma: [Vec<u8>; 2],
    left_cul_level_chroma: [Vec<u8>; 2],
    above_dc_sign_category_chroma: [Vec<u8>; 2],
    left_dc_sign_category_chroma: [Vec<u8>; 2],

    /// `seg_pred` above/left context (spec 5.11.9/5.11.10, real dav1d `t->a->seg_pred[bx4]`/
    /// `t->l.seg_pred[by4]`) -- unlike this struct's other above/left arrays, only ever holds a
    /// single bit per position (whether that position's CU used temporal segment-id prediction),
    /// reset per superblock row like the rest.
    above_seg_pred: Vec<bool>,
    left_seg_pred: Vec<bool>,
    /// `segment_id` per-4x4-unit grid across the *whole tile* (not a thin above/left strip like
    /// this struct's other context arrays) -- real dav1d `get_cur_frame_segid` (`src/env.h`) needs
    /// a genuine above-LEFT diagonal lookup (`cur_seg_map[-(stride+1)]`), which a same-column
    /// above-row array can't provide once a same-row neighbor to the left has already overwritten
    /// that column's "above" slot with a same-row value (see `segment_id_context`'s doc for the
    /// full reasoning) -- so this crate mirrors dav1d's own choice of a real per-tile grid instead
    /// of trying to force the above/left-strip pattern to fit. `-1` sentinel (`i16`, not `i8`, so
    /// the 0..=7 real segment id range plus this sentinel never risk conflating with a real value)
    /// marks "not yet decoded" -- matches `AvailU`/`AvailL` being false for that position (real
    /// spec's `have_top`/`have_left`, simplified to tile-local `x4>0`/`y4>0` -- this crate's
    /// established single-tile-only precedent, see `TileContext`'s own doc history).
    seg_id_grid: Vec<i16>,
    seg_id_grid_stride: u32,

    /// Real palette (spec 5.11.46) above/left context state -- `pal_sz` (`[0]`=Y, `[1]`=UV,
    /// shared by U and V since they're always read together) and `pal_colors` (`[0]`=Y, `[1]`=U,
    /// `[2]`=V, up to 8 colors each) mirror dav1d's `t->a->pal_sz`/`t->pal_sz_uv`/`t->al_pal`
    /// (`src/decode.c`/`src/env.h`) index-for-index. **Chroma is indexed at the same LUMA `x4`/
    /// `y4` positions as Y**, not chroma-scaled -- verified against dav1d's own comment at this
    /// exact array's write site ("see aomedia bug 2183 for why we use luma coordinates here"),
    /// not assumed; this is a real, deliberate dav1d/spec choice, not this crate's usual
    /// chroma-scale-approximation pattern (`TileContext`'s chroma residual field doc).
    above_pal_sz: [Vec<u8>; 2],
    left_pal_sz: [Vec<u8>; 2],
    above_pal_colors: [Vec<[u16; 8]>; 3],
    left_pal_colors: [Vec<[u16; 8]>; 3],
}

impl TileContext {
    /// `tile_width_4x4`/`tile_height_4x4`: tile dimensions in 4x4 units.
    pub fn new(tile_width_4x4: u32, tile_height_4x4: u32) -> Self {
        Self {
            restoration_refs: [crate::tile::RestorationRef::default(); 3],
            above_skip: vec![false; tile_width_4x4.max(1) as usize],
            left_skip: vec![false; tile_height_4x4.max(1) as usize],
            above_skip_mode: vec![false; tile_width_4x4.max(1) as usize],
            left_skip_mode: vec![false; tile_height_4x4.max(1) as usize],
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
            above_comp_type: vec![0; tile_width_4x4.max(1) as usize],
            left_comp_type: vec![0; tile_height_4x4.max(1) as usize],
            above_filter: [
                vec![3; tile_width_4x4.max(1) as usize],
                vec![3; tile_width_4x4.max(1) as usize],
            ],
            left_filter: [
                vec![3; tile_height_4x4.max(1) as usize],
                vec![3; tile_height_4x4.max(1) as usize],
            ],
            spatial_ref: SpatialRefContext::new(tile_width_4x4, tile_height_4x4),
            above_tx_class: vec![-1; tile_width_4x4.max(1) as usize],
            left_tx_class: vec![-1; tile_height_4x4.max(1) as usize],
            above_cul_level: vec![0; tile_width_4x4.max(1) as usize],
            left_cul_level: vec![0; tile_height_4x4.max(1) as usize],
            above_dc_sign_category: vec![1; tile_width_4x4.max(1) as usize],
            left_dc_sign_category: vec![1; tile_height_4x4.max(1) as usize],
            above_var_tx: vec![VAR_TX_UNSET; tile_width_4x4.max(1) as usize],
            left_var_tx: vec![VAR_TX_UNSET; tile_height_4x4.max(1) as usize],
            above_cul_level_chroma: [
                vec![0; tile_width_4x4.max(1) as usize],
                vec![0; tile_width_4x4.max(1) as usize],
            ],
            left_cul_level_chroma: [
                vec![0; tile_height_4x4.max(1) as usize],
                vec![0; tile_height_4x4.max(1) as usize],
            ],
            above_dc_sign_category_chroma: [
                vec![1; tile_width_4x4.max(1) as usize],
                vec![1; tile_width_4x4.max(1) as usize],
            ],
            left_dc_sign_category_chroma: [
                vec![1; tile_height_4x4.max(1) as usize],
                vec![1; tile_height_4x4.max(1) as usize],
            ],
            above_seg_pred: vec![false; tile_width_4x4.max(1) as usize],
            left_seg_pred: vec![false; tile_height_4x4.max(1) as usize],
            seg_id_grid: vec![-1; (tile_width_4x4.max(1) * tile_height_4x4.max(1)) as usize],
            seg_id_grid_stride: tile_width_4x4.max(1),
            above_pal_sz: [
                vec![0; tile_width_4x4.max(1) as usize],
                vec![0; tile_width_4x4.max(1) as usize],
            ],
            left_pal_sz: [
                vec![0; tile_height_4x4.max(1) as usize],
                vec![0; tile_height_4x4.max(1) as usize],
            ],
            above_pal_colors: [
                vec![[0; 8]; tile_width_4x4.max(1) as usize],
                vec![[0; 8]; tile_width_4x4.max(1) as usize],
                vec![[0; 8]; tile_width_4x4.max(1) as usize],
            ],
            left_pal_colors: [
                vec![[0; 8]; tile_height_4x4.max(1) as usize],
                vec![[0; 8]; tile_height_4x4.max(1) as usize],
                vec![[0; 8]; tile_height_4x4.max(1) as usize],
            ],
        }
    }

    /// Reset the left-context arrays at the start of each new superblock row.
    pub fn start_superblock_row(&mut self) {
        self.left_skip.iter_mut().for_each(|v| *v = false);
        self.left_skip_mode.iter_mut().for_each(|v| *v = false);
        self.left_mode.iter_mut().for_each(|v| *v = 0);
        self.left_partition.iter_mut().for_each(|v| *v = 0);
        self.left_ref_intra.iter_mut().for_each(|v| *v = true);
        self.left_ref_comp.iter_mut().for_each(|v| *v = false);
        self.left_ref0.iter_mut().for_each(|v| *v = 0);
        self.left_ref1.iter_mut().for_each(|v| *v = 0);
        self.left_comp_type.iter_mut().for_each(|v| *v = 0);
        for dir in 0..2 {
            self.left_filter[dir].iter_mut().for_each(|v| *v = 3);
        }
        self.left_tx_class.iter_mut().for_each(|v| *v = -1);
        self.left_cul_level.iter_mut().for_each(|v| *v = 0);
        self.left_dc_sign_category.iter_mut().for_each(|v| *v = 1);
        self.left_var_tx.iter_mut().for_each(|v| *v = VAR_TX_UNSET);
        for plane in 0..2 {
            self.left_cul_level_chroma[plane]
                .iter_mut()
                .for_each(|v| *v = 0);
            self.left_dc_sign_category_chroma[plane]
                .iter_mut()
                .for_each(|v| *v = 1);
        }
        self.left_seg_pred.iter_mut().for_each(|v| *v = false);
        // `seg_id_grid` deliberately NOT reset here -- it's a real per-tile grid (this field's
        // doc), not an above/left strip; positions above the current row must stay readable for
        // `segment_id_context`'s above/above-left lookups.
        for plane in 0..2 {
            self.left_pal_sz[plane].iter_mut().for_each(|v| *v = 0);
        }
        for plane in 0..3 {
            self.left_pal_colors[plane]
                .iter_mut()
                .for_each(|v| *v = [0; 8]);
        }
    }
}
