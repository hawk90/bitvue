//! CDF (Cumulative Distribution Function) Tables
//!
//! Per AV1 Specification Section 5.11.44 (Partition CDF)
//!
//! CDFs represent probability distributions for symbols.
//! Each CDF is an array where:
//! - `cdf[0]` = 0
//! - `cdf[i]` = cumulative probability for symbols 0..i (scaled to 0..32768)
//! - `cdf[n]` = 32768 (total probability)
//!
//! ## Partition CDFs
//!
//! Partition symbols have different distributions based on:
//! - Block size (larger blocks more likely to split)
//! - Context (neighboring partition types)
//!
//! For MVP, we use simplified uniform distributions.

mod accessors;
mod defaults;
use defaults::{binary_ctx_cdf, multi_ctx_cdf};
mod save;

/// CDF scale (2^15)
pub const CDF_SCALE: u16 = 32768;

/// Partition CDF for a specific block size
///
/// Contains probability distribution for partition types.
/// Number of symbols varies by block size:
/// - 4x4: 1 symbol (NONE only)
/// - 8x8: 4 symbols (NONE, HORZ, VERT, SPLIT)
/// - 16x16+: 10 symbols (all partition types)
#[derive(Debug, Clone)]
pub struct PartitionCdf {
    /// CDF array (cumulative probabilities)
    /// Length = num_symbols + 1
    pub cdf: Vec<u16>,
    /// Number of symbols
    pub num_symbols: usize,
}

impl PartitionCdf {
    /// Create uniform distribution CDF
    ///
    /// All symbols have equal probability.
    pub fn uniform(num_symbols: usize) -> Self {
        let mut cdf = Vec::with_capacity(num_symbols + 1);

        // First entry is always 0
        cdf.push(0);

        // Divide probability space equally
        // Handle remainder to ensure last value is exactly CDF_SCALE (32768)
        let step = CDF_SCALE / num_symbols as u16;
        let remainder = CDF_SCALE % num_symbols as u16;
        for i in 1..=num_symbols {
            let value = if i == num_symbols {
                CDF_SCALE // Last entry must be exactly 32768
            } else {
                // Distribute remainder across early entries
                let base = step * i as u16;
                let extra = remainder.min(i as u16 - 1);
                base + extra
            };
            cdf.push(value);
        }

        Self { cdf, num_symbols }
    }

    /// Create biased CDF (NONE is most likely)
    ///
    /// This better reflects actual AV1 encoding where:
    /// - NONE is very common (no split)
    /// - SPLIT is common for large blocks
    /// - Other partitions are less common
    pub fn biased_none(num_symbols: usize) -> Self {
        let mut cdf = Vec::with_capacity(num_symbols + 1);
        cdf.push(0);

        // Assign probabilities:
        // - NONE: 60%
        // - SPLIT: 20%
        // - Others: divide remaining 20%

        if num_symbols == 1 {
            // 4x4 block - only NONE
            cdf.push(CDF_SCALE);
        } else if num_symbols == 4 {
            // 8x8 block - NONE, HORZ, VERT, SPLIT
            cdf.push((CDF_SCALE as f32 * 0.6) as u16); // NONE: 60%
            cdf.push((CDF_SCALE as f32 * 0.7) as u16); // HORZ: 10%
            cdf.push((CDF_SCALE as f32 * 0.8) as u16); // VERT: 10%
            cdf.push(CDF_SCALE); // SPLIT: 20%
        } else {
            // 16x16+ block - all 10 partition types
            cdf.push((CDF_SCALE as f32 * 0.5) as u16); // NONE: 50%
            cdf.push((CDF_SCALE as f32 * 0.55) as u16); // HORZ: 5%
            cdf.push((CDF_SCALE as f32 * 0.60) as u16); // VERT: 5%
            cdf.push((CDF_SCALE as f32 * 0.75) as u16); // SPLIT: 15%
            cdf.push((CDF_SCALE as f32 * 0.80) as u16); // HORZ_A: 5%
            cdf.push((CDF_SCALE as f32 * 0.85) as u16); // HORZ_B: 5%
            cdf.push((CDF_SCALE as f32 * 0.90) as u16); // VERT_A: 5%
            cdf.push((CDF_SCALE as f32 * 0.93) as u16); // VERT_B: 3%
            cdf.push((CDF_SCALE as f32 * 0.96) as u16); // HORZ_4: 3%
            cdf.push(CDF_SCALE); // VERT_4: 3% (last entry must be exactly 32768)
        }

        Self { cdf, num_symbols }
    }

    /// Get CDF as slice
    pub fn as_slice(&self) -> &[u16] {
        &self.cdf
    }
}

/// The CDFs of one motion-vector component (dav1d `CdfMvComponent`, `src/cdf.h`): sign, magnitude
/// class, the class-0 integer bit, the class>0 integer bits, and the fractional/high-precision
/// bits of each. Defaults are dav1d's `default_cdf.mv.comp` (`src/cdf.c`).
#[derive(Debug, Clone)]
pub struct MvComponentCdfs {
    pub sign: Vec<u16>,
    /// 11 symbols: class 0..=10.
    pub classes: Vec<u16>,
    pub class0: Vec<u16>,
    /// Indexed by the class-0 integer bit.
    pub class0_fp: [Vec<u16>; 2],
    /// One per integer bit of a class > 0 magnitude.
    pub class_n: [Vec<u16>; 10],
    pub class_n_fp: Vec<u16>,
    pub class0_hp: Vec<u16>,
    pub class_n_hp: Vec<u16>,
}

impl MvComponentCdfs {
    fn new() -> Self {
        Self {
            sign: binary_ctx_cdf(16384),
            classes: multi_ctx_cdf(&[
                28672, 30976, 31858, 32320, 32551, 32656, 32740, 32757, 32762, 32767,
            ]),
            class0: binary_ctx_cdf(27648),
            class0_fp: [
                multi_ctx_cdf(&[16384, 24576, 26624]),
                multi_ctx_cdf(&[12288, 21248, 24128]),
            ],
            class_n: [
                17408, 17920, 18944, 20480, 22528, 24576, 28672, 29952, 29952, 30720,
            ]
            .map(binary_ctx_cdf),
            class_n_fp: multi_ctx_cdf(&[8192, 17408, 21248]),
            class0_hp: binary_ctx_cdf(20480),
            class_n_hp: binary_ctx_cdf(16384),
        }
    }
}

/// CDF context (collection of all CDF tables)
///
/// For MVP, we maintain simplified CDFs.
/// Full implementation would have many more contexts based on neighbors.
#[derive(Debug, Clone)]
pub struct CdfContext {
    /// Partition CDFs indexed by `[block_size_log2 - 2][context 0..=3]` (context from
    /// `crate::tile::TileContext::partition_context`, real above/left 8x8-granularity partition
    /// bitmask -- see that method's doc). Real spec/rav1d default values (`memorysafety/rav1d`,
    /// BSD-2-Clause, `src/cdf.rs`'s `partition` field) and real per-context adaptation, like
    /// `skip_cdf`/`kfym` -- not the "representative" placeholder every other CDF in this struct
    /// still is.
    /// - index 0 (block_size_log2=2, 4x4): trivial 1-symbol placeholder, never actually read (no
    ///   `BlockLevel` exists for 4x4 in the real spec -- partition recursion stops one level up).
    /// - index 1 (log2=3, 8x8): 4 symbols (NONE/HORZ/VERT/SPLIT only -- no A/B/4-way splits).
    /// - index 2..=5 (log2=4..=7, 16x16..=128x128): 10 symbols, except index 5 (128x128) is
    ///   really only 8 real symbols (no HORZ_4/VERT_4) -- rav1d encodes this as 7 real CDF
    ///   entries vs. the others' 9, both stored at their natural (non-padded) length here.
    partition_cdfs: [[Vec<u16>; 4]; 6],

    /// Skip flag CDFs, one per context (0..=2, from `TileContext::skip_context` -- see
    /// `SymbolDecoder::read_skip`'s doc). Real spec/rav1d default values (`memorysafety/rav1d`,
    /// BSD-2-Clause, `src/cdf.rs:4605`, `Default_Skip_Cdf`) and real per-context adaptation --
    /// unlike every other CDF in this struct, this one is not a "representative" placeholder.
    skip_cdf: [Vec<u16>; 3],

    /// `skip_mode` CDFs, one per context (0..=2, `TileContext::skip_mode_context`'s doc). Real
    /// spec/rav1d default values + real per-context adaptation.
    skip_mode_cdf: [Vec<u16>; 3],
    /// `is_inter` CDFs, one per context (0..=3, `TileContext::intra_ctx`'s doc). Real spec/rav1d
    /// default values + real per-context adaptation.
    intra_cdf: [Vec<u16>; 4],
    /// Non-key-frame `y_mode` CDFs, one per block-size-class context (0..=3,
    /// `crate::tile::coding_unit::y_mode_size_context`'s doc). Real spec/rav1d default values +
    /// real per-context adaptation.
    y_mode_cdf: [Vec<u16>; 4],
    /// `motion_mode`/`obmc` CDFs, one per exact block size (17 real entries, `read_motion_mode`'s
    /// doc). Real spec/rav1d default values + real per-context adaptation.
    motion_mode_cdf: [Vec<u16>; 17],
    obmc_cdf: [Vec<u16>; 17],
    /// `interintra`/`interintra_mode`/`interintra_wedge` CDFs (`read_interintra`'s doc). Real
    /// spec/rav1d default values + real per-context adaptation.
    interintra_cdf: [Vec<u16>; 4],
    interintra_mode_cdf: [Vec<u16>; 4],
    interintra_wedge_cdf: [Vec<u16>; 7],
    /// `wedge_comp`/`wedge_idx` CDFs (compound wedge, `read_wedge_comp`'s doc; `wedge_idx` shared
    /// with `interintra_wedge`'s own index read). Real spec/rav1d default values + real
    /// per-context adaptation.
    wedge_comp_cdf: [Vec<u16>; 9],
    wedge_idx_cdf: [Vec<u16>; 9],
    /// `mask_comp`/`jnt_comp` CDFs (`read_mask_comp`/`read_jnt_comp`'s doc). Real spec/rav1d
    /// default values + real per-context adaptation.
    mask_comp_cdf: [Vec<u16>; 6],
    jnt_comp_cdf: [Vec<u16>; 6],
    /// `filter` (subpel interpolation) CDFs, `[dir][ctx]` (`read_filter`'s doc). Real spec/rav1d
    /// default values + real per-context adaptation.
    filter_cdf: [[Vec<u16>; 8]; 2],
    /// `drl_bit` CDFs, one per context (0..=2, `crate::tile::context::get_drl_context`'s doc).
    /// Real spec/rav1d default values + real per-context adaptation.
    drl_bit_cdf: [Vec<u16>; 3],

    /// `seg_pred` (temporal segment-id prediction flag) CDFs, one per context (0..=2, from
    /// `TileContext::seg_pred_context`, real spec/rav1d `above_seg_pred[x4] + left_seg_pred[y4]`)
    /// -- real spec/rav1d default values (`memorysafety/rav1d`/`videolan/dav1d`, BSD-2-Clause,
    /// `src/cdf.c`'s `default_cdf.m.seg_pred`, all-uniform `16384`) + real per-context adaptation.
    seg_pred_cdf: [Vec<u16>; 3],
    /// `segment_id` CDFs, one per context (0..=2, from `TileContext::segment_id_context`, real
    /// spec/rav1d `get_cur_frame_segid`'s 3-way above/left/above-left match ctx) -- real spec/
    /// rav1d default values (`src/cdf.c`'s `default_cdf.m.seg_id`) + real per-context adaptation.
    /// 7-symbol alphabet (`DAV1D_MAX_SEGMENTS - 1`, i.e. 8 real segments 0..=7); the decoded
    /// symbol is a `neg_deinterleave`-encoded diff against a predicted segment id, not the raw
    /// segment id itself (`SymbolDecoder::read_segment_id`'s doc).
    seg_id_cdf: [Vec<u16>; 3],

    /// `has_palette_y` CDFs, `[bsizeCtx 0..=6][ctx 0..=2]` (spec 5.11.46's `palette_mode_info()`,
    /// `bsizeCtx = MiWidthLog2 + MiHeightLog2 - 2`, `ctx` from `TileContext::has_palette_context`
    /// -- real spec/rav1d default values (`src/cdf.c`'s `default_cdf.m.pal_y`) + real per-context
    /// adaptation.
    pal_y_cdf: [[Vec<u16>; 3]; 7],
    /// `has_palette_uv` CDFs, one per context (0..=1, `PaletteSizeY > 0`) -- real spec/rav1d
    /// default values (`default_cdf.m.pal_uv`).
    pal_uv_cdf: [Vec<u16>; 2],
    /// `palette_size_y_minus_2`/`palette_size_uv_minus_2` CDFs, `[y_or_uv][bsizeCtx 0..=6]`
    /// (6-symbol alphabet, `PaletteSize = symbol + 2` giving real sizes 2..=8) -- real spec/rav1d
    /// default values (`default_cdf.m.pal_sz`).
    pal_sz_cdf: [[Vec<u16>; 7]; 2],
    /// `color_map` (palette per-pixel index) CDFs, `[y_or_uv][pal_sz - 2][ctx 0..=4]` -- alphabet
    /// size `pal_sz - 1` per spec (fewer symbols for smaller palettes, real spec/rav1d shape, not
    /// padded to a fixed width) -- real spec/rav1d default values (`default_cdf.m.color_map`).
    /// Context is `TileContext`/`order_palette`'s real diagonal-wavefront rank derivation (see
    /// `SymbolDecoder::read_palette_index_map`'s doc), not an above/left count like every other
    /// context in this crate.
    color_map_cdf: [[[Vec<u16>; 5]; 7]; 2],

    /// `angle_delta_y`/`angle_delta_uv` CDFs (spec 5.11.??? `intra_angle_info_y`/`_uv`), one per
    /// directional mode (`mode - V_PRED`, 0..=7 covering `V_PRED..=D67_PRED`, i.e. `VERT_PRED..
    /// VERT_LEFT_PRED` in dav1d's naming) -- shared between Y and UV (real spec/dav1d: same table,
    /// `ts->cdf.m.angle_delta[mode - VERT_PRED]`, indexed by whichever plane's mode is directional
    /// at the call site). Real spec/rav1d default values (`default_cdf.m.angle_delta`,
    /// `src/cdf.c`) + real per-context (per-mode) adaptation. 7-symbol alphabet (`2*MAX_ANGLE_DELTA
    /// + 1 = 7`, decoded value `-3..=3`).
    angle_delta_cdf: [Vec<u16>; 8],
    /// `uv_mode` CDFs, `[cfl_allowed 0..=1][y_mode 0..=12]` -- real spec/rav1d default values
    /// (`default_cdf.m.uv_mode`). `cfl_allowed=0` rows are a genuinely different (shorter,
    /// 13-symbol) alphabet than `cfl_allowed=1` rows (14-symbol, `CFL_PRED` as an extra outcome) --
    /// not the same values truncated, real distinct default probabilities per dav1d source.
    uv_mode_cdf: [[Vec<u16>; 13]; 2],
    /// `cfl_alpha_signs` CDF (spec 5.11.45, 8-symbol) -- real spec/rav1d default values
    /// (`default_cdf.m.cfl_sign`), no context (single fixed slot, real spec: this symbol itself
    /// selects U/V sign combination, not neighbor-derived).
    cfl_sign_cdf: Vec<u16>,
    /// `cfl_alpha_u`/`cfl_alpha_v` CDFs, one per context (0..=5, real spec: derived from the sign
    /// combination just decoded via `cfl_sign_cdf` -- see `SymbolDecoder::read_cfl_alphas`'s doc
    /// for the exact `(sign_u, sign_v) -> ctx` mapping) -- real spec/rav1d default values
    /// (`default_cdf.m.cfl_alpha`). 16-symbol alphabet (decoded value `1..=16`, sign applied
    /// separately).
    cfl_alpha_cdf: [Vec<u16>; 6],
    /// `use_filter_intra` CDFs, indexed by this crate's `BlockSize` discriminant (`bs as usize`,
    /// 0..=21) -- real spec/rav1d default values (`default_cdf.m.use_filter_intra`), reordered from
    /// dav1d's `BS_*` array order to match this crate's own `BlockSize` enum order (see the
    /// construction site's doc for the mapping). `Block128x32`/`Block32x128` (not real dav1d/spec
    /// block sizes -- see that doc) get the same harmless `16384` placeholder dav1d itself uses for
    /// every size the real `filter_intra` gate (`max(bw4,bh4) <= 8`, i.e. both dims `<=32px`) can
    /// never actually select.
    use_filter_intra_cdf: [Vec<u16>; 24],
    /// `filter_intra_mode` CDF (spec 5.11.??? `filter_intra_mode_info()`, 5-symbol) -- real
    /// spec/rav1d default values (`default_cdf.m.filter_intra`), no context (single fixed slot).
    filter_intra_mode_cdf: Vec<u16>,

    /// `txfm_split` CDFs, `[cat 0..=6][ctx 0..=2]` -- real per-context values + adaptation, see
    /// its construction site's doc in `CdfContext::new`.
    txpart_cdf: [[Vec<u16>; 3]; 7],

    /// Key-frame `intra_mode` CDFs, indexed `[above_mode_class][left_mode_class]` (0..=4 each,
    /// see `crate::tile::TileContext::intra_mode_context`). Real spec/rav1d default values +
    /// real per-context adaptation -- like `skip_cdf`, not a "representative" placeholder.
    kfym: [[Vec<u16>; 5]; 5],
    /// `inter_mode()`'s 3 cascaded booleans (spec 5.11.23) -- real per-context CDFs + adaptation,
    /// context from `crate::tile::TileContext::inter_mode_context`. See
    /// `SymbolDecoder::read_inter_mode`'s doc for the decision tree these back.
    newmv_mode_cdf: [Vec<u16>; 6],
    globalmv_mode_cdf: [Vec<u16>; 2],
    refmv_mode_cdf: [Vec<u16>; 6],
    /// `compound_mode` CDF (spec 5.11.24, 8 symbols) -- see `SymbolDecoder::read_compound_mode`'s
    /// doc for the symbol ordering. Real per-context CDFs + adaptation, context from
    /// `crate::tile::TileContext::compound_mode_context` -- not a "representative" placeholder.
    compound_mode_cdf: [Vec<u16>; 8],

    /// Motion Vector CDFs
    /// MV joint CDF (4 symbols: correlation between horizontal/vertical components)
    /// - MV_JOINT_ZERO (both zero)
    /// - MV_JOINT_HNZVZ (horz non-zero, vert zero)
    /// - MV_JOINT_HZVNZ (horz zero, vert non-zero)
    /// - MV_JOINT_HNZVNZ (both non-zero)
    mv_joint_cdf: Vec<u16>,
    /// Per-component MV CDFs, `[0]` = vertical (row), `[1]` = horizontal (column) -- dav1d's
    /// `mv.comp[2]`. See [`MvComponentCdfs`].
    mv_comp: [MvComponentCdfs; 2],

    /// `delta_q` CDF (spec 5.11.38 `read_delta_qindex`) -- real 4-symbol alphabet, see the
    /// construction site's doc. The sign bit is a real equi-probable (50/50) raw bit, not a CDF
    /// (see `SymbolDecoder::read_delta_q`'s doc) -- no separate sign CDF field exists here.
    delta_q_cdf: Vec<u16>,
    /// `delta_lf` CDFs (spec 5.11.38 `read_delta_lf`), one per real spec index -- see
    /// `SymbolDecoder::read_delta_lf`'s doc.
    delta_lf_cdf: [Vec<u16>; 5],

    /// Residual coefficient CDFs -- see `residual` module doc (`symbol/mod.rs`) for why most of
    /// these are still deliberately context-*independent* (one representative CDF per symbol
    /// kind, not indexed by neighbor levels / tx-size-context / plane / is_inter like the real
    /// spec's Section 9.24 tables) -- same simplification precedent as every other CDF in this
    /// struct, just applied to a syntax element where getting *some* real value beats the
    /// previous behavior of reading nothing at all. `eob_pt`/`coeff_base_eob` (below) are the
    /// exception: real per-context CDFs + adaptation, like `skip_cdf`/`ref_frame`'s CDFs.
    /// `txb_skip_cdf`: all_zero flag for one transform block (2 symbols), indexed
    /// `[tx_size_class 0..=4][ctx 0..=6]`. Real above/left neighbor context
    /// (`TileContext::txb_skip_context`) is wired for key-frame, non-IntraBC coding units (where
    /// transform-block boundaries are real, not heuristic) -- see `SymbolDecoder::
    /// read_residual_block`'s doc; other callers pass a fixed `ctx = 0` (chroma axis fixed to 0,
    /// luma-only regardless). Source: rav1d `coef.skip` (`memorysafety/rav1d`, BSD-2-Clause,
    /// `src/cdf.rs`), real per-frame qindex-bucket selection (see `eob_bin_16_cdf`'s doc).
    txb_skip_cdf: [[Vec<u16>; 7]; 5],
    /// `coeff_base` -- level (0..=3) for every other coefficient position (4 symbols), indexed
    /// `[tx_size_class 0..=4][ctx 0..=40]`. Real neighbor context (`symbol::scan::lo_ctx`, ported
    /// from rav1d `get_lo_ctx`) -- see `SymbolDecoder::read_residual_block`'s doc. Source: rav1d
    /// `coef.base_tok` (`memorysafety/rav1d`, BSD-2-Clause, `src/cdf.rs`), real per-frame
    /// qindex-bucket selection, chroma=0 (luma-only, same precedent as `txb_skip_cdf`).
    coeff_base_cdf: [[Vec<u16>; 41]; 5],
    /// `coeff_br` -- range-extension increment (0..=3), read in a loop while extending a level
    /// past `NUM_BASE_LEVELS` (4 symbols), indexed `[min(tx_size_class,3)][ctx 0..=20]` (one
    /// fewer tx-size bucket than `coeff_base_cdf` -- 32x32 and 64x64 share a bucket here, matching
    /// rav1d's `min(t_dim->ctx, 3)`). Real context (position band + `lo_ctx`'s `hi_mag`, reused
    /// directly rather than recomputed) -- see `SymbolDecoder::read_residual_block`'s doc. Source:
    /// rav1d `coef.br_tok`, real per-frame qindex-bucket selection, chroma=0.
    coeff_br_cdf: [[Vec<u16>; 21]; 4],
    /// `dc_sign` -- sign of the DC (position 0) coefficient (2 symbols), indexed `[ctx 0..=2]`.
    /// Real above/left neighbor context (`TileContext::dc_sign_context`) is wired alongside
    /// `txb_skip_cdf` -- see `SymbolDecoder::read_residual_block`'s doc; other callers pass a
    /// fixed `ctx = 0` (chroma axis fixed to 0). AC coefficient signs are read as literal
    /// (uniform) bits per spec, not CDF-coded. Source: rav1d `coef.dc_sign`, real per-frame
    /// qindex-bucket selection.
    dc_sign_cdf: [Vec<u16>; 3],

    /// `eob_bin` (spec 5.11.39's `eob_pt_*`), real spec/rav1d default CDFs + real adaptation, one
    /// family per coefficient-count class (16/64/256/1024, the only 4 of rav1d's 7 classes a
    /// *square*-only transform ever selects -- see `get_eob_bin_cdf_mut`'s doc) each
    /// indexed by `is_1d` (0..=1, from `SymbolDecoder::read_transform_type_is_1d`; the real
    /// spec's `chroma` axis is always 0 here since this crate only reads luma residual, see
    /// `read_residual_block`'s doc). Source: rav1d `eob_bin_16/64/256/1024`
    /// (`memorysafety/rav1d`, BSD-2-Clause, `src/cdf.rs`) -- all 4 of rav1d's real per-frame
    /// qindex-bucket default-CDF variants are ported (`CdfContext::new_with_qcat`'s
    /// `qcat.min(3)` match, real formula `(base_q_idx>20) + (base_q_idx>60) + (base_q_idx>120)`,
    /// same as dav1d's `get_qcat`); every *other* aspect of this CDF (real per-context
    /// *selection*, not per-qindex tuning) was already real before this. `eob_bin_1024` has no
    /// `is_1d` axis in rav1d itself (only `chroma`), hence the plain `[Vec<u16>; 1]`-shaped (i.e.
    /// unindexed) field below.
    eob_bin_16_cdf: [Vec<u16>; 2],
    /// Real `eob_bin` default for 4x8/8x4 (rc area 32) transforms -- see `eob_bin_16_cdf`'s doc
    /// (same shape/source, `dav1d`'s `eob_bin_32`). Only reachable once rectangular var-tx feeds
    /// `read_residual_block` a non-square transform (see that function's doc).
    eob_bin_32_cdf: [Vec<u16>; 2],
    eob_bin_64_cdf: [Vec<u16>; 2],
    /// Real `eob_bin` default for 8x16/16x8 (rc area 128) transforms -- see `eob_bin_16_cdf`'s
    /// doc.
    eob_bin_128_cdf: [Vec<u16>; 2],
    eob_bin_256_cdf: [Vec<u16>; 2],
    /// Real `eob_bin` default for 16x32/32x16 (rc area 512) transforms -- no `is_1d` axis, same
    /// reason as `eob_bin_1024_cdf` (these tx sizes structurally can't carry an H/V-only 1D
    /// transform type per spec's transform-type-per-size restriction table).
    eob_bin_512_cdf: Vec<u16>,
    eob_bin_1024_cdf: Vec<u16>,
    /// `eob_hi_bit` -- the single context-coded first bit of `eob`'s extra-bits suffix (spec
    /// 5.11.39), indexed `[tx_size_class 0..=4][eob_bin 0..=10]` (chroma axis fixed to 0, same
    /// as `eob_bin_*`). Every extra bit *after* the first stays a plain literal 50/50 bit (see
    /// `read_residual_block`'s doc) -- matches the real spec, which only context-codes the first
    /// one. Source: rav1d `eob_hi_bit` (`memorysafety/rav1d`, BSD-2-Clause, `src/cdf.rs`), real
    /// per-frame qindex-bucket selection (see `eob_bin_16_cdf`'s doc).
    eob_hi_bit_cdf: [[Vec<u16>; 11]; 5],
    /// `coeff_base_eob` -- level (1..=3) for the highest-scan-order nonzero coefficient (3
    /// symbols), indexed `[tx_size_class 0..=4][ctx 0..=3]` (chroma axis fixed to 0). Real
    /// spec/rav1d default CDFs + real adaptation -- context is a pure arithmetic function of
    /// `eob` and tx size (see `SymbolDecoder::coeff_base_eob_context`'s doc), no neighbor/above-
    /// left state needed. Source: rav1d `eob_base_tok` (`memorysafety/rav1d`, BSD-2-Clause,
    /// `src/cdf.rs`), real per-frame qindex-bucket selection (see `eob_bin_16_cdf`'s doc).
    coeff_base_eob_cdf: [[Vec<u16>; 4]; 5],

    /// Chroma-plane residual CDFs -- see `SymbolDecoder::read_chroma_residual_block`'s doc for
    /// the (deliberately narrower than luma's) scope these back: square luma coding blocks
    /// 8x8..128x128 only (`tx_size_class` 0..=3 -- chroma real caps at 32x32 regardless of luma
    /// size, per rav1d's `dav1d_max_txfm_size_for_bs` table for 4:2:0, so `tx_size_class` 3
    /// (32x32) is the largest chroma ever needs -- a 128x128 luma block's 64x64 chroma area tiles
    /// 4 real `TX_32X32` blocks, still this same size class), `is_1d` fixed `false` (2D).
    /// `txb_skip`/`dc_sign` now use *real* per-position context (`TileContext::
    /// txb_skip_context_chroma`/`dc_sign_context_chroma`) against a real 6-context (`txb_skip`)/
    /// 3-context (`dc_sign`) default table -- fixes a real desync bug where a single shared CDF
    /// slot per tx-size-class was over-adapted by repeated same-CU chroma tile calls (128x128
    /// luma's 2x2-tiled chroma hit it 4x per plane per CU); see `TileContext::
    /// txb_skip_context_chroma`'s doc. `coeff_base`/`coeff_br` reuse `symbol::scan::lo_ctx`'s real
    /// neighbor-context formula verbatim (it's plane-agnostic) against these chroma-specific
    /// default values. All sourced from rav1d/dav1d's real `[chroma=1]` axis
    /// (`videolan/dav1d`/`memorysafety/rav1d`, BSD-2-Clause, `src/cdf.c`'s
    /// `default_coef_cdf[qcat].skip`/`.dc_sign`), real per-frame qindex-bucket selection (all 4 of
    /// dav1d's buckets ported, see `eob_bin_16_cdf`'s doc), same precedent as every luma table
    /// above.
    txb_skip_cdf_chroma: [[Vec<u16>; 6]; 4],
    dc_sign_cdf_chroma: [Vec<u16>; 3],
    eob_bin_16_cdf_chroma: [Vec<u16>; 2],
    /// Real `eob_bin` default for a 4x8/8x4 chroma tile (rc area 32) -- only reachable once a
    /// non-square luma coding block's chroma plane isn't itself square (e.g. luma 16x8 -> chroma
    /// 8x4). See `eob_bin_16_cdf_chroma`'s doc for source/shape.
    eob_bin_32_cdf_chroma: [Vec<u16>; 2],
    eob_bin_64_cdf_chroma: [Vec<u16>; 2],
    /// Real `eob_bin` default for an 8x16/16x8 chroma tile (rc area 128).
    eob_bin_128_cdf_chroma: [Vec<u16>; 2],
    eob_bin_256_cdf_chroma: [Vec<u16>; 2],
    /// Real `eob_bin` default for a 16x32/32x16 chroma tile (rc area 512).
    eob_bin_512_cdf_chroma: Vec<u16>,
    eob_bin_1024_cdf_chroma: Vec<u16>,
    eob_hi_bit_cdf_chroma: [[Vec<u16>; 11]; 4],
    coeff_base_eob_cdf_chroma: [[Vec<u16>; 4]; 4],
    coeff_base_cdf_chroma: [[Vec<u16>; 41]; 4],
    coeff_br_cdf_chroma: [[Vec<u16>; 21]; 4],

    /// `transform_type()` (spec 5.11.47) CDFs -- read once per transform block, before its
    /// `coeffs()`, to determine `TxClass`/`is_1d` for `eob_bin_*_cdf`'s context axis (see
    /// `SymbolDecoder::read_transform_type_is_1d`'s doc for the decision tree these back and why
    /// only `is_1d`, not the exact `TxType`, is tracked). All indexed `[tx_size_class][...]`
    /// (`txtp_intra1`/`txtp_inter1`: 0..=1 only, `txtp_intra2`: 0..=2, `txtp_inter3`: 0..=3 --
    /// the other tx-size classes are unreachable for that specific CDF, see that method's doc).
    /// Source: rav1d `txtp_intra1`/`txtp_intra2`/`txtp_inter1`/`txtp_inter2`/`txtp_inter3`
    /// (`memorysafety/rav1d`, BSD-2-Clause, `src/cdf.rs`), first qindex-bucket variant only.
    txtp_intra1_cdf: [[Vec<u16>; 13]; 2],
    txtp_intra2_cdf: [[Vec<u16>; 13]; 3],
    txtp_inter1_cdf: [Vec<u16>; 2],
    txtp_inter2_cdf: Vec<u16>,
    txtp_inter3_cdf: [Vec<u16>; 4],

    /// Intra `tx_size()` (spec 5.11.15/16) depth CDF, indexed `[max_tx_class - 1][ctx 0..=2]`
    /// (only classes 1..=4, i.e. 8x8..64x64 -- 4x4 never reads this symbol at all, see
    /// `SymbolDecoder::read_tx_size`'s doc). Real spec/rav1d default CDFs + real adaptation,
    /// context from `crate::tile::TileContext::tx_size_context`. Source: rav1d `m.txsz`
    /// (`memorysafety/rav1d`, BSD-2-Clause, `src/cdf.rs`), first qindex-bucket variant only (this
    /// one isn't itself qindex-bucketed like the residual-coefficient tables, but follows the
    /// same "one representative variant" precedent for consistency).
    txsz_cdf: [[Vec<u16>; 3]; 4],

    /// Reference-frame selection CDFs, indexed by real above/left neighbor context (see
    /// `crate::tile::TileContext`'s `comp_mode_context`/`comp_ref_type_context`/
    /// `single_ref_p*_context`/`uni_comp_ref_p1_context` methods). Real spec/rav1d default values
    /// (`memorysafety/rav1d`, BSD-2-Clause, `src/cdf.rs`'s `comp`/`comp_dir`/`r#ref`/
    /// `comp_fwd_ref`/`comp_bwd_ref`/`comp_uni_ref` fields) and real per-context adaptation, like
    /// `skip_cdf`/`kfym`/the `partition_cdfs` -- not a "representative" placeholder. Naming
    /// mirrors AV1 spec 5.11.25's syntax element names (`single_ref_p1`..`p6`, `comp_ref_type`,
    /// etc.) so the decision-tree structure in `read_ref_frames` is easy to cross-check against
    /// the spec. `comp_mode`/`comp_ref_type` have 5 contexts (`get_comp_ctx`/`get_comp_dir_ctx`
    /// return 0..=4); every other table here has 3 (the `cmp_counts`-derived context functions
    /// return 0..=2).
    comp_mode_cdf: [Vec<u16>; 5],
    single_ref_p1_cdf: [Vec<u16>; 3],
    single_ref_p2_cdf: [Vec<u16>; 3],
    single_ref_p3_cdf: [Vec<u16>; 3],
    single_ref_p4_cdf: [Vec<u16>; 3],
    single_ref_p5_cdf: [Vec<u16>; 3],
    single_ref_p6_cdf: [Vec<u16>; 3],
    comp_ref_type_cdf: [Vec<u16>; 5],
    uni_comp_ref_cdf: [Vec<u16>; 3],
    uni_comp_ref_p1_cdf: [Vec<u16>; 3],
    uni_comp_ref_p2_cdf: [Vec<u16>; 3],
    comp_ref_cdf: [Vec<u16>; 3],
    comp_ref_p1_cdf: [Vec<u16>; 3],
    comp_ref_p2_cdf: [Vec<u16>; 3],
    comp_bwdref_cdf: [Vec<u16>; 3],
    comp_bwdref_p1_cdf: [Vec<u16>; 3],

    /// `use_intrabc` (spec 5.11.6) -- intra block copy flag, read for intra-frame blocks only
    /// when the frame header's `allow_intrabc` is set (rare, screen-content-coding use case).
    use_intrabc_cdf: Vec<u16>,
    /// Loop-restoration unit type CDFs (spec 5.11.58): `restore_switchable` (3 symbols, one of
    /// NONE/WIENER/SGRPROJ), and the on/off flags `use_wiener`/`use_sgrproj`.
    restore_switchable_cdf: Vec<u16>,
    restore_wiener_cdf: Vec<u16>,
    restore_sgrproj_cdf: Vec<u16>,
}

/// Maps a transform block's size in pixels per side (4/8/16/32/64) to rav1d's square `TxfmSize`
/// class (0..=4, `TX_4X4`..`TX_64X64`). Used throughout the residual-coefficient CDFs' context
/// (`eob_hi_bit_cdf`/`coeff_base_eob_cdf`/`txtp_*_cdf`) -- a coarser 0..=3 "coefficient-count
/// class" (16/64/256/1024, folding 64x64 into 32x32's bucket) is used separately by
/// `get_eob_bin_cdf_mut`, matching real AV1's coefficient-scan cap at 32x32.
pub fn tx_size_class(tx_size_px: u32) -> usize {
    match tx_size_px {
        0..=4 => 0,
        5..=8 => 1,
        9..=16 => 2,
        17..=32 => 3,
        _ => 4,
    }
}

/// Read a `partition` CDF slot as a real threshold, or `0` if `idx` falls outside this bucket's
/// real alphabet (`partition_cdfs`' layout: indices `0..alphabet_size-1` are real descending
/// thresholds -- the last of which, `alphabet_size-1`, is always exactly `0` by construction --
/// and index `alphabet_size` is the adaptation count). This mirrors rav1d's own fixed 16-slot
/// `[u16;16]` partition CDF array (`src/cdf.rs`), whose entries past a given block size's real
/// alphabet are permanently `0` padding (`cdf0d`'s untouched tail) -- treating an out-of-range
/// index as `0` here reproduces that padding without needing a fixed-size array of our own.
///
/// Used only by `split_or_horz_prob`/`split_or_vert_prob`: unlike those, do NOT use this to
/// bounds-check away the `bl != BLOCK_128X128` guard those two need explicitly -- see their docs.
fn partition_cdf_real_or_zero(cdf: &[u16], idx: usize) -> i32 {
    if idx + 1 < cdf.len() {
        cdf[idx] as i32
    } else {
        0
    }
}

/// Aggregated probability for the `split_or_horz` symbol (spec 5.11.4: read when `hasCols &&
/// !hasRows`, choosing between `PARTITION_HORZ` and `PARTITION_SPLIT`). Ported index-for-index
/// from rav1d's `gather_top_partition_prob` (`src/env.rs`, `memorysafety/rav1d`, BSD-2-Clause):
/// `out = cdf[HORZ] - cdf[HORZ_A] + cdf[HORZ_B]`, plus `cdf[HORZ_4] - cdf[VERT_B]` for the
/// 10-symbol alphabet only (16x16/32x32/64x64 -- `cdf.len() == 11`). `PartitionType`'s numeric
/// values (`None=0, Horz=1, Vert=2, Split=3, HorzA=4, HorzB=5, VertA=6, VertB=7, Horz4=8,
/// Vert4=9`) match rav1d's `BlockPartition` enum exactly, so these are literal indices, not a
/// reference to the enum (avoids a `tile` module dependency in this file).
///
/// The 128x128 bucket (`cdf.len() == 9`) has real, nonzero `VertB` (index 7) mass -- unlike the
/// 8x8/4x4 buckets, `partition_cdf_real_or_zero`'s padding-as-zero can't stand in for rav1d's
/// explicit `if bl != BLOCK_128X128` guard there, hence the explicit `cdf.len() == 11` check
/// (only 16x16/32x32/64x64 have `HORZ_4`/`VERT_4` at all).
///
/// The returned value feeds a **non-adaptive** binary read (`ArithmeticDecoder::read_symbol`, not
/// `_adaptive`) -- rav1d's `gather_*_partition_prob` results go straight to
/// `rav1d_msac_decode_bool` (no CDF write-back), never to the general adaptive symbol path, so
/// the real `partition_cdfs` entry this was computed from is left untouched.
pub(crate) fn split_or_horz_prob(cdf: &[u16]) -> u16 {
    let mut out = partition_cdf_real_or_zero(cdf, 1) - partition_cdf_real_or_zero(cdf, 4)
        + partition_cdf_real_or_zero(cdf, 5);
    if cdf.len() >= 11 {
        out += partition_cdf_real_or_zero(cdf, 8) - partition_cdf_real_or_zero(cdf, 7);
    }
    out.clamp(0, CDF_SCALE as i32) as u16
}

/// Aggregated probability for the `split_or_vert` symbol (spec 5.11.4: read when `hasRows &&
/// !hasCols`, choosing between `PARTITION_VERT` and `PARTITION_SPLIT`). Ported index-for-index
/// from rav1d's `gather_left_partition_prob` (`src/env.rs`): `out = cdf[NONE] - cdf[HORZ] +
/// cdf[VERT] - cdf[VERT_A]`, plus `cdf[VERT_B] - cdf[HORZ_4]` for the 10-symbol alphabet only --
/// see `split_or_horz_prob`'s doc for the shared indexing/adaptation/128x128 notes (all apply
/// here identically, mirrored).
pub(crate) fn split_or_vert_prob(cdf: &[u16]) -> u16 {
    let mut out = partition_cdf_real_or_zero(cdf, 0) - partition_cdf_real_or_zero(cdf, 1)
        + partition_cdf_real_or_zero(cdf, 2)
        - partition_cdf_real_or_zero(cdf, 6);
    if cdf.len() >= 11 {
        out += partition_cdf_real_or_zero(cdf, 7) - partition_cdf_real_or_zero(cdf, 8);
    }
    out.clamp(0, CDF_SCALE as i32) as u16
}

impl Default for CdfContext {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests;
