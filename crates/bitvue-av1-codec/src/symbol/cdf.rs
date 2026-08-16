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

/// CDF context (collection of all CDF tables)
///
/// For MVP, we maintain simplified CDFs.
/// Full implementation would have many more contexts based on neighbors.
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
    use_filter_intra_cdf: [Vec<u16>; 22],
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
    /// MV sign CDF (2 symbols: positive, negative)
    mv_sign_cdf: Vec<u16>,
    /// MV class CDF (11 classes for magnitude range)
    mv_class_cdf: Vec<u16>,
    /// MV bit CDFs for reading magnitude bits
    mv_bit_cdf: Vec<u16>,

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
    eob_bin_16_cdf_chroma: Vec<u16>,
    /// Real `eob_bin` default for a 4x8/8x4 chroma tile (rc area 32) -- only reachable once a
    /// non-square luma coding block's chroma plane isn't itself square (e.g. luma 16x8 -> chroma
    /// 8x4). See `eob_bin_16_cdf_chroma`'s doc for source/shape.
    eob_bin_32_cdf_chroma: Vec<u16>,
    eob_bin_64_cdf_chroma: Vec<u16>,
    /// Real `eob_bin` default for an 8x16/16x8 chroma tile (rc area 128).
    eob_bin_128_cdf_chroma: Vec<u16>,
    eob_bin_256_cdf_chroma: Vec<u16>,
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

/// Build a 2-symbol CDF from `p0`, the probability of the first (index-0) symbol. Returns the
/// real spec/rav1d descending format directly (see `to_descending`'s doc) -- matches the
/// hand-picked-bias style every other CDF in this file uses (see `skip_cdf`).
fn binary_cdf(p0: f32) -> Vec<u16> {
    to_descending(&[0, (CDF_SCALE as f32 * p0) as u16, CDF_SCALE])
}

/// Build a 2-symbol CDF from a raw rav1d default probability (`0..=CDF_SCALE`, already the real
/// spec value -- not a hand-picked fraction like `binary_cdf` takes). Same descending-threshold +
/// adaptation-count-slot shape as `skip_cdf`'s per-context entries (see that field's doc).
fn binary_ctx_cdf(raw_prob: u16) -> Vec<u16> {
    vec![CDF_SCALE - raw_prob, 0, 0]
}

/// Build an N-symbol CDF from `N-1` raw rav1d default probabilities (already the real spec
/// values). Same shape as `binary_ctx_cdf` generalized to more than 2 symbols -- per-element
/// `32768-p`, then the last symbol's implicit-0 boundary and the adaptation-count slot appended.
fn multi_ctx_cdf(raw_probs: &[u16]) -> Vec<u16> {
    let mut out: Vec<u16> = raw_probs.iter().map(|&p| CDF_SCALE - p).collect();
    out.push(0); // last symbol's implicit boundary
    out.push(0); // adaptation count starts at 0
    out
}

/// Convert an ascending CDF (this crate's older convention: `cdf[0]=0 .. cdf[n]=CDF_SCALE`, still
/// how every table in this file is authored/hand-tuned for readability) into the real spec/rav1d
/// descending convention `ArithmeticDecoder::read_symbol`/`update_cdf` require (see their docs in
/// `symbol/arithmetic.rs`): `d[i] = CDF_SCALE - ascending[i+1]` for `i in 0..n_symbols`, with the
/// trailing slot repurposed from "always CDF_SCALE" to the adaptation count (initialized to 0).
///
/// This is a pure format conversion, not a values upgrade -- it preserves whatever
/// representative/hand-picked probability shape the ascending literal already encoded. Context
/// derivation (see `docs/DEVELOPMENT_PHASES.md` Phase 4's AV1 entropy-decoding note) is a
/// per-symbol upgrade tracked separately; `partition`/`skip` get it in this same phase, everything
/// else is deferred.
fn to_descending(ascending: &[u16]) -> Vec<u16> {
    let n_symbols = ascending.len() - 1;
    let mut out: Vec<u16> = (0..n_symbols)
        .map(|i| CDF_SCALE - ascending[i + 1])
        .collect();
    out.push(0); // adaptation count starts at 0
    out
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

impl CdfContext {
    /// Create a new CDF context with default values for dav1d's qindex bucket 0
    /// (`qcat = 0`, `base_q_idx <= 20`). Thin back-compat wrapper -- every pre-existing caller
    /// that doesn't care about qindex-bucket selection keeps working unchanged. See
    /// `new_with_qcat`'s doc for the real 4-bucket behavior.
    pub fn new() -> Self {
        Self::new_with_qcat(0)
    }

    /// Create a new CDF context with default values selected from dav1d's real per-frame qindex
    /// bucket (`qcat`, real formula `(base_q_idx>20) + (base_q_idx>60) + (base_q_idx>120)`,
    /// see `SymbolDecoder::new_with_qcat`'s callers). Only the residual-coefficient CDF family
    /// (`txb_skip`/`dc_sign`/`eob_bin_*`/`eob_hi_bit`/`coeff_base_eob`/`coeff_base`/`coeff_br`,
    /// 13 luma + 13 chroma fields) is qindex-bucketed in real AV1/dav1d -- every other field
    /// below (partition/skip/intra-mode/ref-frame/MV/delta_q CDFs, etc.) is qcat-independent and
    /// built once, same as before. `qcat` is clamped to `0..=3` (dav1d's real bucket count) via
    /// `.min(3)` at each match site rather than asserted, so a future caller passing something
    /// derived from an unclamped formula degrades to the highest real bucket instead of
    /// panicking.
    pub fn new_with_qcat(qcat: u8) -> Self {
        // Real spec/rav1d default CDFs for `partition`, per `[block_size_log2-2][context 0..=3]`.
        // Source: rav1d `Default_Partition_W8/16/32/64/128_Cdf` (`src/cdf.rs`, `memorysafety/rav1d`,
        // BSD-2-Clause). Each row's raw numbers are real spec probabilities needing the same
        // `32768 - p` per-element conversion as `kfym` (not `to_descending`'s ascending-array
        // transform -- these aren't cumulative ascending arrays, just a list of raw probs), with
        // the last real symbol's implicit-0 entry and the adaptation-count slot appended
        // explicitly. Context derivation: `crate::tile::TileContext::partition_context` (real
        // above/left 8x8-unit partition bitmask, mirrors rav1d's `get_partition_ctx`).
        let partition_cdfs: [[Vec<u16>; 4]; 6] = [
            // 4x4: no partition symbol is ever read at this size (spec: 4x4 blocks cannot be
            // split further); kept as a structurally-valid trivial 1-symbol placeholder.
            [vec![0, 0], vec![0, 0], vec![0, 0], vec![0, 0]],
            // 8x8 (rav1d BlockLevel 4)
            [
                vec![13636, 7258, 2376, 0, 0],
                vec![18840, 12913, 4228, 0, 0],
                vec![20246, 9089, 4139, 0, 0],
                vec![22872, 13985, 6915, 0, 0],
            ],
            // 16x16 (rav1d BlockLevel 3)
            [
                vec![17171, 11839, 8197, 6062, 5104, 3947, 3167, 2197, 866, 0, 0],
                vec![
                    24843, 21725, 15983, 10298, 8797, 7725, 6117, 4067, 2934, 0, 0,
                ],
                vec![
                    27354, 19499, 17657, 12280, 10408, 8268, 7231, 6432, 651, 0, 0,
                ],
                vec![
                    30106, 26406, 24154, 11908, 9715, 7990, 6332, 4939, 1597, 0, 0,
                ],
            ],
            // 32x32 (rav1d BlockLevel 2)
            [
                vec![14306, 11848, 9644, 5121, 4541, 3719, 3249, 2590, 1224, 0, 0],
                vec![
                    25079, 23708, 20712, 7776, 7108, 6586, 5817, 4727, 3716, 0, 0,
                ],
                vec![26753, 23759, 22706, 8224, 7359, 6223, 5697, 5242, 721, 0, 0],
                vec![31374, 30560, 29972, 4154, 3707, 3302, 2928, 2583, 869, 0, 0],
            ],
            // 64x64 (rav1d BlockLevel 1)
            [
                vec![12631, 11221, 9690, 3202, 2931, 2507, 2244, 1876, 1044, 0, 0],
                vec![
                    26036, 25278, 23271, 4824, 4518, 4253, 3799, 3138, 2664, 0, 0,
                ],
                vec![26823, 25105, 24420, 4085, 3651, 3019, 2704, 2470, 530, 0, 0],
                vec![31898, 31556, 31281, 1570, 1374, 1194, 1025, 887, 436, 0, 0],
            ],
            // 128x128 (rav1d BlockLevel 0) -- only 8 real symbols (no HORZ_4/VERT_4)
            [
                vec![4869, 4549, 4239, 284, 229, 149, 129, 0, 0],
                vec![26161, 25778, 24500, 708, 549, 430, 397, 0, 0],
                vec![27339, 26092, 25646, 741, 541, 237, 186, 0, 0],
                vec![32057, 31802, 31596, 320, 230, 151, 104, 0, 0],
            ],
        ];

        // Skip flag CDFs, per context (0..=2 above/left-skip-neighbor count). Real spec/rav1d
        // default probabilities (`Default_Skip_Cdf`, `src/cdf.rs:4605`): raw probs 31671/16515/
        // 4576 for contexts 0/1/2 respectively. rav1d's own `cdf0d` helper computes the
        // descending threshold as `32768 - raw_prob` -- the same transform `to_descending` uses,
        // just applied directly here since these are already the final real spec values (not
        // hand-picked placeholders needing the ascending-literal round trip).
        let skip_cdf = [
            vec![32768 - 31671, 0, 0], // context 0 (no skip neighbors): mostly not-skip
            vec![32768 - 16515, 0, 0], // context 1 (one skip neighbor)
            vec![32768 - 4576, 0, 0],  // context 2 (both neighbors skip): mostly skip
        ];

        // `skip_mode` (spec 5.11.5, per-CU gate before `skip`) -- real spec/rav1d default CDFs
        // (`default_cdf.m.skip_mode`, `src/cdf.c`), context 0..=2 (`skip_mode_context`'s doc, same
        // above+left-neighbor-count shape as `skip`). Raw probs 32621/20708/8127.
        let skip_mode_cdf: [Vec<u16>; 3] = [32621, 20708, 8127].map(binary_ctx_cdf);

        // `is_inter` (spec 5.11.5's `read_is_inter`, real per-CU intra/inter dispatch for
        // non-key/non-switch... actually non-intra-only frames) -- real spec/rav1d default CDFs
        // (`default_cdf.m.intra`, `src/cdf.c`), context 0..=3 (`TileContext::intra_ctx`'s doc).
        // Raw probs 806/16662/20186/26538. Previously never read at all (see
        // `SymbolDecoder::read_is_inter`'s doc for the desync this closes).
        let intra_cdf: [Vec<u16>; 4] = [806, 16662, 20186, 26538].map(binary_ctx_cdf);

        // `y_mode` (spec 5.11.7's non-key-frame `intra_block_mode_info()` Y-mode read -- distinct
        // from `kfym`, key-frame-only) -- real spec/rav1d default CDFs (`default_cdf.m.y_mode`,
        // `src/cdf.c`), context 0..=3 from `crate::tile::coding_unit::y_mode_size_context` (a
        // block-size class, NOT an above/left neighbor lookup like `kfym`). 13-symbol alphabet
        // (`N_INTRA_PRED_MODES`), same `multi_ctx_cdf` shape as every other real multi-symbol
        // table here.
        let y_mode_cdf: [Vec<u16>; 4] = [
            multi_ctx_cdf(&[
                22801, 23489, 24293, 24756, 25601, 26123, 26606, 27418, 27945, 29228, 29685, 30349,
            ]),
            multi_ctx_cdf(&[
                18673, 19845, 22631, 23318, 23950, 24649, 25527, 27364, 28152, 29701, 29984, 30852,
            ]),
            multi_ctx_cdf(&[
                19770, 20979, 23396, 23939, 24241, 24654, 25136, 27073, 27830, 29360, 29730, 30659,
            ]),
            multi_ctx_cdf(&[
                20155, 21301, 22838, 23178, 23261, 23533, 23703, 24804, 25352, 26575, 27016, 28049,
            ]),
        ];

        // `motion_mode` (spec 5.11.27, 3-symbol: translation/OBMC/warp) -- real spec/rav1d default
        // CDFs (`default_cdf.m.motion_mode`, `src/cdf.c`), indexed by exact block size (17 real
        // entries -- `motion_mode` is never read for a block smaller than 8x8 in either dimension,
        // `min(bw4,bh4)>=2`'s real spec gate, see `read_motion_mode`'s doc). `obmc` (2-symbol) is
        // the same real per-size indexing, used instead of `motion_mode` when this position's
        // above/left neighbors don't have a real spec-eligible warp candidate
        // (`find_matching_ref`'s doc).
        let motion_mode_cdf: [Vec<u16>; 17] = [
            multi_ctx_cdf(&[7651, 24760]),  // 8x8
            multi_ctx_cdf(&[4738, 24765]),  // 8x16
            multi_ctx_cdf(&[5391, 25528]),  // 16x8
            multi_ctx_cdf(&[19419, 26810]), // 16x16
            multi_ctx_cdf(&[5123, 23606]),  // 16x32
            multi_ctx_cdf(&[11606, 24308]), // 32x16
            multi_ctx_cdf(&[26260, 29116]), // 32x32
            multi_ctx_cdf(&[20360, 28062]), // 32x64
            multi_ctx_cdf(&[21679, 26830]), // 64x32
            multi_ctx_cdf(&[29516, 30701]), // 64x64
            multi_ctx_cdf(&[28898, 30397]), // 64x128
            multi_ctx_cdf(&[30878, 31335]), // 128x64
            multi_ctx_cdf(&[32507, 32558]), // 128x128
            multi_ctx_cdf(&[28799, 31390]), // 8x32
            multi_ctx_cdf(&[28973, 31594]), // 16x64
            multi_ctx_cdf(&[26431, 30774]), // 32x8
            multi_ctx_cdf(&[29742, 31203]), // 64x16
        ];
        let obmc_cdf: [Vec<u16>; 17] = [
            10437, 9371, 9301, 17432, 14423, 15142, 25817, 22823, 22083, 30128, 31014, 31560,
            32638, 23664, 24008, 20901, 26879,
        ]
        .map(binary_ctx_cdf);

        // `interintra`/`interintra_mode`/`interintra_wedge` (spec 5.11.29) -- real spec/rav1d
        // default CDFs. `interintra`: 4 contexts (`y_mode_size_context`'s block-size class,
        // reused verbatim -- real dav1d indexes both by the SAME `dav1d_ymode_size_context[bs]`).
        // `interintra_mode`: 4-symbol, same 4 contexts. `interintra_wedge`: 7 contexts (a REAL
        // subset of the 9 compound-`wedge` contexts -- interintra is only allowed for 7 of the 9
        // wedge-eligible sizes, excluding 8x32/32x8, see `wedge_ctx`'s doc).
        let interintra_cdf: [Vec<u16>; 4] = [16384, 26887, 27597, 30237].map(binary_ctx_cdf);
        let interintra_mode_cdf: [Vec<u16>; 4] = [
            multi_ctx_cdf(&[8192, 16384, 24576]),
            multi_ctx_cdf(&[1875, 11082, 27332]),
            multi_ctx_cdf(&[2473, 9996, 26388]),
            multi_ctx_cdf(&[4238, 11537, 25926]),
        ];
        let interintra_wedge_cdf: [Vec<u16>; 7] =
            [20036, 24957, 26704, 27530, 29564, 29444, 26872].map(binary_ctx_cdf);

        // `wedge_comp`/`wedge_idx` (spec 5.11.28, compound `wedge` selection) -- real spec/rav1d
        // default CDFs, 9 contexts (`wedge_ctx`'s doc). `wedge_idx` (16-symbol) is shared verbatim
        // between compound wedge and `interintra_wedge`'s own wedge-index read (real spec: same
        // `wedge_idx()` syntax element either way).
        let wedge_comp_cdf: [Vec<u16>; 9] =
            [23431, 13171, 11470, 9770, 9100, 8233, 6172, 11820, 7701].map(binary_ctx_cdf);
        let wedge_idx_cdf: [Vec<u16>; 9] = [
            multi_ctx_cdf(&[
                2438, 4440, 6599, 8663, 11005, 12874, 15751, 18094, 20359, 22362, 24127, 25702,
                27752, 29450, 31171,
            ]),
            multi_ctx_cdf(&[
                806, 3266, 6005, 6738, 7218, 7367, 7771, 14588, 16323, 17367, 18452, 19422, 22839,
                26127, 29629,
            ]),
            multi_ctx_cdf(&[
                2779, 3738, 4683, 7213, 7775, 8017, 8655, 14357, 17939, 21332, 24520, 27470, 29456,
                30529, 31656,
            ]),
            multi_ctx_cdf(&[
                1684, 3625, 5675, 7108, 9302, 11274, 14429, 17144, 19163, 20961, 22884, 24471,
                26719, 28714, 30877,
            ]),
            multi_ctx_cdf(&[
                1142, 3491, 6277, 7314, 8089, 8355, 9023, 13624, 15369, 16730, 18114, 19313, 22521,
                26012, 29550,
            ]),
            multi_ctx_cdf(&[
                2742, 4195, 5727, 8035, 8980, 9336, 10146, 14124, 17270, 20533, 23434, 25972,
                27944, 29570, 31416,
            ]),
            multi_ctx_cdf(&[
                1727, 3948, 6101, 7796, 9841, 12344, 15766, 18944, 20638, 22038, 23963, 25311,
                26988, 28766, 31012,
            ]),
            multi_ctx_cdf(&[
                154, 987, 1925, 2051, 2088, 2111, 2151, 23033, 23703, 24284, 24985, 25684, 27259,
                28883, 30911,
            ]),
            multi_ctx_cdf(&[
                1135, 1322, 1493, 2635, 2696, 2737, 2770, 21016, 22935, 25057, 27251, 29173, 30089,
                30960, 31933,
            ]),
        ];

        // `mask_comp`/`jnt_comp` (spec 5.11.28's jnt_comp-vs-seg/wedge selector and the
        // weighted-average bit within the jnt_comp branch) -- real spec/rav1d default CDFs, 6
        // contexts each (`TileContext::mask_comp_context`/`jnt_comp_context`'s doc).
        let mask_comp_cdf: [Vec<u16>; 6] =
            [26828, 24035, 12031, 10640, 2901, 16384].map(binary_ctx_cdf);
        let jnt_comp_cdf: [Vec<u16>; 6] =
            [18244, 12865, 7053, 13259, 9334, 4644].map(binary_ctx_cdf);

        // `filter` (spec 5.11.30 subpel interpolation filter) -- real spec/rav1d default CDFs,
        // `[dir 0..=1][ctx 0..=7]` (`TileContext::filter_context`'s doc), 3-symbol alphabet
        // (`DAV1D_N_SWITCHABLE_FILTERS`).
        let filter_cdf: [[Vec<u16>; 8]; 2] = [
            [
                multi_ctx_cdf(&[31935, 32720]),
                multi_ctx_cdf(&[5568, 32719]),
                multi_ctx_cdf(&[422, 2938]),
                multi_ctx_cdf(&[28244, 32608]),
                multi_ctx_cdf(&[31206, 31953]),
                multi_ctx_cdf(&[4862, 32121]),
                multi_ctx_cdf(&[770, 1152]),
                multi_ctx_cdf(&[20889, 25637]),
            ],
            [
                multi_ctx_cdf(&[31910, 32724]),
                multi_ctx_cdf(&[4120, 32712]),
                multi_ctx_cdf(&[305, 2247]),
                multi_ctx_cdf(&[27403, 32636]),
                multi_ctx_cdf(&[31022, 32009]),
                multi_ctx_cdf(&[2963, 32093]),
                multi_ctx_cdf(&[601, 943]),
                multi_ctx_cdf(&[14969, 21398]),
            ],
        ];

        // `drl_bit` (spec 7.10.2.10's DRL index read, single-ref only -- `read_drl_bit`'s doc) --
        // real spec/rav1d default CDFs (`default_cdf.m.drl_bit`, `src/cdf.c`), 3 contexts
        // (`crate::tile::context::get_drl_context`'s doc). Raw probs 13104/24560/18945.
        let drl_bit_cdf: [Vec<u16>; 3] = [13104, 24560, 18945].map(binary_ctx_cdf);

        // seg_pred/seg_id (spec 5.11.9/5.11.10 `segment_id()`): real spec/rav1d default CDFs
        // (`default_cdf.m.seg_pred`/`.seg_id`, `src/cdf.c`).
        let seg_pred_cdf: [Vec<u16>; 3] = [16384, 16384, 16384].map(binary_ctx_cdf);
        let seg_id_cdf: [Vec<u16>; 3] = [
            multi_ctx_cdf(&[5622, 7893, 16093, 18233, 27809, 28373, 32533]),
            multi_ctx_cdf(&[14274, 18230, 22557, 24935, 29980, 30851, 32344]),
            multi_ctx_cdf(&[27527, 28487, 28723, 28890, 32397, 32647, 32679]),
        ];

        // Palette (spec 5.11.46 `palette_mode_info()` + per-pixel color-index tokens): real
        // spec/rav1d default CDFs (`default_cdf.m.pal_y`/`.pal_uv`/`.pal_sz`/`.color_map`,
        // `src/cdf.c`), first qindex-bucket variant, same precedent as every table above.
        let pal_y_cdf: [[Vec<u16>; 3]; 7] = [
            [31676, 3419, 1261].map(binary_ctx_cdf),
            [31912, 2859, 980].map(binary_ctx_cdf),
            [31823, 3400, 781].map(binary_ctx_cdf),
            [32030, 3561, 904].map(binary_ctx_cdf),
            [32309, 7337, 1462].map(binary_ctx_cdf),
            [32265, 4015, 1521].map(binary_ctx_cdf),
            [32450, 7946, 129].map(binary_ctx_cdf),
        ];
        let pal_uv_cdf: [Vec<u16>; 2] = [32461, 21488].map(binary_ctx_cdf);
        let pal_sz_cdf: [[Vec<u16>; 7]; 2] = [
            [
                multi_ctx_cdf(&[7952, 13000, 18149, 21478, 25527, 29241]),
                multi_ctx_cdf(&[7139, 11421, 16195, 19544, 23666, 28073]),
                multi_ctx_cdf(&[7788, 12741, 17325, 20500, 24315, 28530]),
                multi_ctx_cdf(&[8271, 14064, 18246, 21564, 25071, 28533]),
                multi_ctx_cdf(&[12725, 19180, 21863, 24839, 27535, 30120]),
                multi_ctx_cdf(&[9711, 14888, 16923, 21052, 25661, 27875]),
                multi_ctx_cdf(&[14940, 20797, 21678, 24186, 27033, 28999]),
            ],
            [
                multi_ctx_cdf(&[8713, 19979, 27128, 29609, 31331, 32272]),
                multi_ctx_cdf(&[5839, 15573, 23581, 26947, 29848, 31700]),
                multi_ctx_cdf(&[4426, 11260, 17999, 21483, 25863, 29430]),
                multi_ctx_cdf(&[3228, 9464, 14993, 18089, 22523, 27420]),
                multi_ctx_cdf(&[3768, 8886, 13091, 17852, 22495, 27207]),
                multi_ctx_cdf(&[2464, 8451, 12861, 21632, 25525, 28555]),
                multi_ctx_cdf(&[1269, 5435, 10433, 18963, 21700, 25865]),
            ],
        ];
        let color_map_cdf: [[[Vec<u16>; 5]; 7]; 2] = [
            [
                // y, pal_sz 2..=8
                [28710, 16384, 10553, 27036, 31603].map(binary_ctx_cdf),
                [
                    multi_ctx_cdf(&[27877, 30490]),
                    multi_ctx_cdf(&[11532, 25697]),
                    multi_ctx_cdf(&[6544, 30234]),
                    multi_ctx_cdf(&[23018, 28072]),
                    multi_ctx_cdf(&[31915, 32385]),
                ],
                [
                    multi_ctx_cdf(&[25572, 28046, 30045]),
                    multi_ctx_cdf(&[9478, 21590, 27256]),
                    multi_ctx_cdf(&[7248, 26837, 29824]),
                    multi_ctx_cdf(&[19167, 24486, 28349]),
                    multi_ctx_cdf(&[31400, 31825, 32250]),
                ],
                [
                    multi_ctx_cdf(&[24779, 26955, 28576, 30282]),
                    multi_ctx_cdf(&[8669, 20364, 24073, 28093]),
                    multi_ctx_cdf(&[4255, 27565, 29377, 31067]),
                    multi_ctx_cdf(&[19864, 23674, 26716, 29530]),
                    multi_ctx_cdf(&[31646, 31893, 32147, 32426]),
                ],
                [
                    multi_ctx_cdf(&[23132, 25407, 26970, 28435, 30073]),
                    multi_ctx_cdf(&[7443, 17242, 20717, 24762, 27982]),
                    multi_ctx_cdf(&[6300, 24862, 26944, 28784, 30671]),
                    multi_ctx_cdf(&[18916, 22895, 25267, 27435, 29652]),
                    multi_ctx_cdf(&[31270, 31550, 31808, 32059, 32353]),
                ],
                [
                    multi_ctx_cdf(&[23105, 25199, 26464, 27684, 28931, 30318]),
                    multi_ctx_cdf(&[6950, 15447, 18952, 22681, 25567, 28563]),
                    multi_ctx_cdf(&[7560, 23474, 25490, 27203, 28921, 30708]),
                    multi_ctx_cdf(&[18544, 22373, 24457, 26195, 28119, 30045]),
                    multi_ctx_cdf(&[31198, 31451, 31670, 31882, 32123, 32391]),
                ],
                [
                    multi_ctx_cdf(&[21689, 23883, 25163, 26352, 27506, 28827, 30195]),
                    multi_ctx_cdf(&[6892, 15385, 17840, 21606, 24287, 26753, 29204]),
                    multi_ctx_cdf(&[5651, 23182, 25042, 26518, 27982, 29392, 30900]),
                    multi_ctx_cdf(&[19349, 22578, 24418, 25994, 27524, 29031, 30448]),
                    multi_ctx_cdf(&[31028, 31270, 31504, 31705, 31927, 32153, 32392]),
                ],
            ],
            [
                // uv, pal_sz 2..=8
                [29089, 16384, 8713, 29257, 31610].map(binary_ctx_cdf),
                [
                    multi_ctx_cdf(&[25257, 29145]),
                    multi_ctx_cdf(&[12287, 27293]),
                    multi_ctx_cdf(&[7033, 27960]),
                    multi_ctx_cdf(&[20145, 25405]),
                    multi_ctx_cdf(&[30608, 31639]),
                ],
                [
                    multi_ctx_cdf(&[24210, 27175, 29903]),
                    multi_ctx_cdf(&[9888, 22386, 27214]),
                    multi_ctx_cdf(&[5901, 26053, 29293]),
                    multi_ctx_cdf(&[18318, 22152, 28333]),
                    multi_ctx_cdf(&[30459, 31136, 31926]),
                ],
                [
                    multi_ctx_cdf(&[22980, 25479, 27781, 29986]),
                    multi_ctx_cdf(&[8413, 21408, 24859, 28874]),
                    multi_ctx_cdf(&[2257, 29449, 30594, 31598]),
                    multi_ctx_cdf(&[19189, 21202, 25915, 28620]),
                    multi_ctx_cdf(&[31844, 32044, 32281, 32518]),
                ],
                [
                    multi_ctx_cdf(&[22217, 24567, 26637, 28683, 30548]),
                    multi_ctx_cdf(&[7307, 16406, 19636, 24632, 28424]),
                    multi_ctx_cdf(&[4441, 25064, 26879, 28942, 30919]),
                    multi_ctx_cdf(&[17210, 20528, 23319, 26750, 29582]),
                    multi_ctx_cdf(&[30674, 30953, 31396, 31735, 32207]),
                ],
                [
                    multi_ctx_cdf(&[21239, 23168, 25044, 26962, 28705, 30506]),
                    multi_ctx_cdf(&[6545, 15012, 18004, 21817, 25503, 28701]),
                    multi_ctx_cdf(&[3448, 26295, 27437, 28704, 30126, 31442]),
                    multi_ctx_cdf(&[15889, 18323, 21704, 24698, 26976, 29690]),
                    multi_ctx_cdf(&[30988, 31204, 31479, 31734, 31983, 32325]),
                ],
                [
                    multi_ctx_cdf(&[21442, 23288, 24758, 26246, 27649, 28980, 30563]),
                    multi_ctx_cdf(&[5863, 14933, 17552, 20668, 23683, 26411, 29273]),
                    multi_ctx_cdf(&[3415, 25810, 26877, 27990, 29223, 30394, 31618]),
                    multi_ctx_cdf(&[17965, 20084, 22232, 23974, 26274, 28402, 30390]),
                    multi_ctx_cdf(&[31190, 31329, 31516, 31679, 31825, 32026, 32322]),
                ],
            ],
        ];

        // angle_delta_y/angle_delta_uv (spec `intra_angle_info_y`/`_uv`) CDFs, one per directional
        // mode: real spec/rav1d default values (`default_cdf.m.angle_delta`, `src/cdf.c`).
        let angle_delta_cdf: [Vec<u16>; 8] = [
            multi_ctx_cdf(&[2180, 5032, 7567, 22776, 26989, 30217]),
            multi_ctx_cdf(&[2301, 5608, 8801, 23487, 26974, 30330]),
            multi_ctx_cdf(&[3780, 11018, 13699, 19354, 23083, 31286]),
            multi_ctx_cdf(&[4581, 11226, 15147, 17138, 21834, 28397]),
            multi_ctx_cdf(&[1737, 10927, 14509, 19588, 22745, 28823]),
            multi_ctx_cdf(&[2664, 10176, 12485, 17650, 21600, 30495]),
            multi_ctx_cdf(&[2240, 11096, 15453, 20341, 22561, 28917]),
            multi_ctx_cdf(&[3605, 10428, 12459, 17676, 21244, 30655]),
        ];

        // uv_mode CDFs, `[cfl_allowed][y_mode]` -- real spec/rav1d default values
        // (`default_cdf.m.uv_mode`, `src/cdf.c`). `[0]` (13-symbol, no CFL outcome) and `[1]`
        // (14-symbol, CFL_PRED as an extra outcome) are genuinely distinct default probability
        // sets, not the same values truncated.
        let uv_mode_cdf: [[Vec<u16>; 13]; 2] = [
            [
                multi_ctx_cdf(&[
                    22631, 24152, 25378, 25661, 25986, 26520, 27055, 27923, 28244, 30059, 30941,
                    31961,
                ]),
                multi_ctx_cdf(&[
                    9513, 26881, 26973, 27046, 27118, 27664, 27739, 27824, 28359, 29505, 29800,
                    31796,
                ]),
                multi_ctx_cdf(&[
                    9845, 9915, 28663, 28704, 28757, 28780, 29198, 29822, 29854, 30764, 31777,
                    32029,
                ]),
                multi_ctx_cdf(&[
                    13639, 13897, 14171, 25331, 25606, 25727, 25953, 27148, 28577, 30612, 31355,
                    32493,
                ]),
                multi_ctx_cdf(&[
                    9764, 9835, 9930, 9954, 25386, 27053, 27958, 28148, 28243, 31101, 31744, 32363,
                ]),
                multi_ctx_cdf(&[
                    11825, 13589, 13677, 13720, 15048, 29213, 29301, 29458, 29711, 31161, 31441,
                    32550,
                ]),
                multi_ctx_cdf(&[
                    14175, 14399, 16608, 16821, 17718, 17775, 28551, 30200, 30245, 31837, 32342,
                    32667,
                ]),
                multi_ctx_cdf(&[
                    12885, 13038, 14978, 15590, 15673, 15748, 16176, 29128, 29267, 30643, 31961,
                    32461,
                ]),
                multi_ctx_cdf(&[
                    12026, 13661, 13874, 15305, 15490, 15726, 15995, 16273, 28443, 30388, 30767,
                    32416,
                ]),
                multi_ctx_cdf(&[
                    19052, 19840, 20579, 20916, 21150, 21467, 21885, 22719, 23174, 28861, 30379,
                    32175,
                ]),
                multi_ctx_cdf(&[
                    18627, 19649, 20974, 21219, 21492, 21816, 22199, 23119, 23527, 27053, 31397,
                    32148,
                ]),
                multi_ctx_cdf(&[
                    17026, 19004, 19997, 20339, 20586, 21103, 21349, 21907, 22482, 25896, 26541,
                    31819,
                ]),
                multi_ctx_cdf(&[
                    12124, 13759, 14959, 14992, 15007, 15051, 15078, 15166, 15255, 15753, 16039,
                    16606,
                ]),
            ],
            [
                multi_ctx_cdf(&[
                    10407, 11208, 12900, 13181, 13823, 14175, 14899, 15656, 15986, 20086, 20995,
                    22455, 24212,
                ]),
                multi_ctx_cdf(&[
                    4532, 19780, 20057, 20215, 20428, 21071, 21199, 21451, 22099, 24228, 24693,
                    27032, 29472,
                ]),
                multi_ctx_cdf(&[
                    5273, 5379, 20177, 20270, 20385, 20439, 20949, 21695, 21774, 23138, 24256,
                    24703, 26679,
                ]),
                multi_ctx_cdf(&[
                    6740, 7167, 7662, 14152, 14536, 14785, 15034, 16741, 18371, 21520, 22206,
                    23389, 24182,
                ]),
                multi_ctx_cdf(&[
                    4987, 5368, 5928, 6068, 19114, 20315, 21857, 22253, 22411, 24911, 25380, 26027,
                    26376,
                ]),
                multi_ctx_cdf(&[
                    5370, 6889, 7247, 7393, 9498, 21114, 21402, 21753, 21981, 24780, 25386, 26517,
                    27176,
                ]),
                multi_ctx_cdf(&[
                    4816, 4961, 7204, 7326, 8765, 8930, 20169, 20682, 20803, 23188, 23763, 24455,
                    24940,
                ]),
                multi_ctx_cdf(&[
                    6608, 6740, 8529, 9049, 9257, 9356, 9735, 18827, 19059, 22336, 23204, 23964,
                    24793,
                ]),
                multi_ctx_cdf(&[
                    5998, 7419, 7781, 8933, 9255, 9549, 9753, 10417, 18898, 22494, 23139, 24764,
                    25989,
                ]),
                multi_ctx_cdf(&[
                    10660, 11298, 12550, 12957, 13322, 13624, 14040, 15004, 15534, 20714, 21789,
                    23443, 24861,
                ]),
                multi_ctx_cdf(&[
                    10522, 11530, 12552, 12963, 13378, 13779, 14245, 15235, 15902, 20102, 22696,
                    23774, 25838,
                ]),
                multi_ctx_cdf(&[
                    10099, 10691, 12639, 13049, 13386, 13665, 14125, 15163, 15636, 19676, 20474,
                    23519, 25208,
                ]),
                multi_ctx_cdf(&[
                    3144, 5087, 7382, 7504, 7593, 7690, 7801, 8064, 8232, 9248, 9875, 10521, 29048,
                ]),
            ],
        ];

        // cfl_alpha_signs (spec 5.11.45) CDF -- real spec/rav1d default values
        // (`default_cdf.m.cfl_sign`).
        let cfl_sign_cdf: Vec<u16> =
            multi_ctx_cdf(&[1418, 2123, 13340, 18405, 26972, 28343, 32294]);
        // cfl_alpha_u/cfl_alpha_v CDFs, one per context (0..=5) -- real spec/rav1d default values
        // (`default_cdf.m.cfl_alpha`).
        let cfl_alpha_cdf: [Vec<u16>; 6] = [
            multi_ctx_cdf(&[
                7637, 20719, 31401, 32481, 32657, 32688, 32692, 32696, 32700, 32704, 32708, 32712,
                32716, 32720, 32724,
            ]),
            multi_ctx_cdf(&[
                14365, 23603, 28135, 31168, 32167, 32395, 32487, 32573, 32620, 32647, 32668, 32672,
                32676, 32680, 32684,
            ]),
            multi_ctx_cdf(&[
                11532, 22380, 28445, 31360, 32349, 32523, 32584, 32649, 32673, 32677, 32681, 32685,
                32689, 32693, 32697,
            ]),
            multi_ctx_cdf(&[
                26990, 31402, 32282, 32571, 32692, 32696, 32700, 32704, 32708, 32712, 32716, 32720,
                32724, 32728, 32732,
            ]),
            multi_ctx_cdf(&[
                17248, 26058, 28904, 30608, 31305, 31877, 32126, 32321, 32394, 32464, 32516, 32560,
                32576, 32593, 32622,
            ]),
            multi_ctx_cdf(&[
                14738, 21678, 25779, 27901, 29024, 30302, 30980, 31843, 32144, 32413, 32520, 32594,
                32622, 32656, 32660,
            ]),
        ];

        // use_filter_intra (spec `filter_intra_mode_info()`) CDFs -- real spec/rav1d default
        // values (`default_cdf.m.use_filter_intra`, `src/cdf.c`), reordered from dav1d's `BS_*`
        // array order into this crate's own `BlockSize` discriminant order (`Block4x4=0,
        // Block4x8=1, Block8x4=2, Block8x8=3, Block8x16=4, Block16x8=5, Block16x16=6, Block16x32=7,
        // Block32x16=8, Block32x32=9, Block32x64=10, Block64x32=11, Block64x64=12, Block64x128=13,
        // Block128x64=14, Block128x128=15, Block32x8=16, Block64x16=17, Block128x32=18,
        // Block8x32=19, Block16x64=20, Block32x128=21` -- see `tile::partition::BlockSize`).
        // `Block128x32`/`Block32x128` have no dav1d/spec counterpart (see field doc) -- given the
        // same harmless `16384` dav1d itself uses for every size the real gate can't select.
        let use_filter_intra_cdf: [Vec<u16>; 22] = [
            binary_ctx_cdf(4621),  // Block4x4
            binary_ctx_cdf(6743),  // Block4x8
            binary_ctx_cdf(5893),  // Block8x4
            binary_ctx_cdf(7866),  // Block8x8
            binary_ctx_cdf(12551), // Block8x16
            binary_ctx_cdf(9394),  // Block16x8
            binary_ctx_cdf(12408), // Block16x16
            binary_ctx_cdf(14301), // Block16x32
            binary_ctx_cdf(12756), // Block32x16
            binary_ctx_cdf(22343), // Block32x32
            binary_ctx_cdf(16384), // Block32x64
            binary_ctx_cdf(16384), // Block64x32
            binary_ctx_cdf(16384), // Block64x64
            binary_ctx_cdf(16384), // Block64x128
            binary_ctx_cdf(16384), // Block128x64
            binary_ctx_cdf(16384), // Block128x128
            binary_ctx_cdf(18101), // Block32x8
            binary_ctx_cdf(16384), // Block64x16
            binary_ctx_cdf(16384), // Block128x32 (no dav1d/spec counterpart, see field doc)
            binary_ctx_cdf(20229), // Block8x32
            binary_ctx_cdf(16384), // Block16x64
            binary_ctx_cdf(16384), // Block32x128 (no dav1d/spec counterpart, see field doc)
        ];
        // filter_intra_mode (5-symbol) CDF -- real spec/rav1d default values
        // (`default_cdf.m.filter_intra`).
        let filter_intra_mode_cdf: Vec<u16> = multi_ctx_cdf(&[8949, 12776, 17211, 29558]);

        // txfm_split (spec 5.11.18 `read_var_tx_size`'s `txfm_split` symbol) CDFs, per
        // `[cat 0..=6][ctx 0..=2]`. `cat` packs the candidate size class and recursion depth
        // (`2*(4-candidate_class)-depth`, `crate::tile::coding_unit::read_var_tx_size`'s doc);
        // `ctx` is `TileContext::var_tx_context`'s `a+l` sum. Source: rav1d `Default_Txpart_Cdf`
        // (`src/cdf.rs`, `memorysafety/rav1d`, BSD-2-Clause).
        let txpart_cdf: [[Vec<u16>; 3]; 7] = [
            [
                binary_ctx_cdf(28581),
                binary_ctx_cdf(23846),
                binary_ctx_cdf(20847),
            ],
            [
                binary_ctx_cdf(24315),
                binary_ctx_cdf(18196),
                binary_ctx_cdf(12133),
            ],
            [
                binary_ctx_cdf(18791),
                binary_ctx_cdf(10887),
                binary_ctx_cdf(11005),
            ],
            [
                binary_ctx_cdf(27179),
                binary_ctx_cdf(20004),
                binary_ctx_cdf(11281),
            ],
            [
                binary_ctx_cdf(26549),
                binary_ctx_cdf(19308),
                binary_ctx_cdf(14224),
            ],
            [
                binary_ctx_cdf(28015),
                binary_ctx_cdf(21546),
                binary_ctx_cdf(14400),
            ],
            [
                binary_ctx_cdf(28165),
                binary_ctx_cdf(22401),
                binary_ctx_cdf(16088),
            ],
        ];

        // kfym: real spec/rav1d default CDFs for key-frame intra_mode (spec 5.11.10
        // `kf_y_mode`), indexed [above_mode_class][left_mode_class] (0..=4 each, from
        // `INTRA_MODE_CONTEXT`). Source: rav1d `src/cdf.rs` `kfym` (`memorysafety/rav1d`,
        // BSD-2-Clause). Each leaf already stores the real spec's raw cumulative-style
        // probabilities; converted to this decoder's descending convention via the same
        // `32768 - p` transform `to_descending`/rav1d's `cdf0d` both use, with the 13th
        // (always-0) real entry and the adaptation-count slot appended explicitly.
        let kfym: [[Vec<u16>; 5]; 5] = [
            [
                vec![
                    17180, 15741, 13430, 12550, 12086, 11658, 10943, 9524, 8579, 4603, 3675, 2302,
                    0, 0,
                ],
                vec![
                    20752, 14702, 13252, 12465, 12049, 11324, 10880, 9736, 8334, 4110, 2596, 1359,
                    0, 0,
                ],
                vec![
                    22716, 21997, 10472, 9980, 9713, 9529, 8635, 7148, 6608, 3432, 2839, 1201, 0, 0,
                ],
                vec![
                    18677, 17362, 16326, 13960, 13632, 13222, 12770, 10672, 8022, 3183, 1810, 306,
                    0, 0,
                ],
                vec![
                    20646, 19503, 17165, 16267, 14159, 12735, 10377, 7185, 6331, 2507, 1695, 293,
                    0, 0,
                ],
            ],
            [
                vec![
                    22745, 13183, 11920, 11328, 10936, 10008, 9679, 8745, 7387, 3754, 2286, 1332,
                    0, 0,
                ],
                vec![
                    26785, 8669, 8208, 7882, 7702, 6973, 6855, 6345, 5158, 2863, 1492, 974, 0, 0,
                ],
                vec![
                    25324, 19987, 12591, 12040, 11691, 11161, 10598, 9363, 8299, 4853, 3678, 2276,
                    0, 0,
                ],
                vec![
                    24231, 18079, 17336, 15681, 15360, 14596, 14360, 12943, 8119, 3615, 1672, 558,
                    0, 0,
                ],
                vec![
                    25225, 18537, 17272, 16573, 14863, 12051, 10784, 8252, 6767, 3093, 1787, 774,
                    0, 0,
                ],
            ],
            [
                vec![
                    20155, 19177, 11385, 10764, 10456, 10191, 9367, 7713, 7039, 3230, 2463, 691, 0,
                    0,
                ],
                vec![
                    23081, 19298, 14262, 13538, 13164, 12621, 12073, 10706, 9549, 5025, 3557, 1861,
                    0, 0,
                ],
                vec![
                    26585, 26263, 6744, 6516, 6402, 6334, 5686, 4414, 4213, 2301, 1974, 682, 0, 0,
                ],
                vec![
                    22050, 21034, 17814, 15544, 15203, 14844, 14207, 11245, 8890, 3793, 2481, 516,
                    0, 0,
                ],
                vec![
                    23574, 22910, 16267, 15505, 14344, 13597, 11205, 6807, 6207, 2696, 2031, 305,
                    0, 0,
                ],
            ],
            [
                vec![
                    20166, 18369, 17280, 14387, 13990, 13453, 13044, 11349, 7708, 3072, 1851, 359,
                    0, 0,
                ],
                vec![
                    24565, 18947, 18244, 15663, 15329, 14637, 14364, 13300, 7543, 3283, 1610, 426,
                    0, 0,
                ],
                vec![
                    24317, 23037, 17764, 15125, 14756, 14343, 13698, 11230, 8163, 3650, 2690, 750,
                    0, 0,
                ],
                vec![
                    25054, 23720, 23252, 16101, 15951, 15774, 15615, 14001, 6025, 2379, 1232, 240,
                    0, 0,
                ],
                vec![
                    23925, 22488, 21272, 17451, 16116, 14825, 13660, 10050, 6999, 2815, 1785, 283,
                    0, 0,
                ],
            ],
            [
                vec![
                    20190, 19097, 16789, 15934, 13693, 11855, 9779, 7319, 6549, 2554, 1618, 291, 0,
                    0,
                ],
                vec![
                    23205, 19142, 17688, 16876, 15012, 11905, 10561, 8532, 7388, 3115, 1625, 491,
                    0, 0,
                ],
                vec![
                    24412, 23867, 15152, 14512, 13418, 12662, 10170, 6821, 6302, 2868, 2245, 507,
                    0, 0,
                ],
                vec![
                    21933, 20953, 19644, 16726, 15750, 14729, 13821, 10015, 8153, 3279, 1885, 286,
                    0, 0,
                ],
                vec![
                    25150, 24480, 22909, 22259, 17382, 14111, 9865, 3992, 3588, 1413, 966, 175, 0,
                    0,
                ],
            ],
        ];

        // `inter_mode()` (spec 5.11.23) -- real spec/rav1d default CDFs + real per-context
        // adaptation, one context each for the 3 cascaded booleans (`newmv`/`globalmv`/`refmv`,
        // see `SymbolDecoder::read_inter_mode`'s doc for the decision tree), context from
        // `crate::tile::TileContext::inter_mode_context` (mirrors rav1d's packed
        // `refmv_ctx<<4|globalmv_ctx<<3|newmv_ctx`, `src/refmvs.rs`). Source: rav1d
        // `newmv_mode`/`globalmv_mode`/`refmv_mode` (`memorysafety/rav1d`, BSD-2-Clause,
        // `src/cdf.rs`), same `32768-p` per-context transform as `skip_cdf`/ref_frame's CDFs.
        let newmv_mode_cdf = [24035, 16630, 15339, 8386, 12222, 4676].map(binary_ctx_cdf);
        let globalmv_mode_cdf = [2175, 1054].map(binary_ctx_cdf);
        let refmv_mode_cdf = [23974, 24188, 17848, 28622, 24312, 19923].map(binary_ctx_cdf);

        // `compound_mode` (spec 5.11.24, 8 symbols) -- real spec/rav1d default CDFs + real
        // per-context adaptation, context from
        // `crate::tile::TileContext::compound_mode_context` (mirrors rav1d's compound remap of
        // `refmv_ctx`/`newmv_ctx`, `src/refmvs.rs`). Source: rav1d `comp_inter_mode`
        // (`memorysafety/rav1d`, BSD-2-Clause, `src/cdf.rs`), same per-element `32768-p` + trailing
        // implicit-0 + adaptation-count-slot transform as `kfym`/`partition_cdfs`.
        let compound_mode_cdf: [Vec<u16>; 8] = [
            multi_ctx_cdf(&[7760, 13823, 15808, 17641, 19156, 20666, 26891]),
            multi_ctx_cdf(&[10730, 19452, 21145, 22749, 24039, 25131, 28724]),
            multi_ctx_cdf(&[10664, 20221, 21588, 22906, 24295, 25387, 28436]),
            multi_ctx_cdf(&[13298, 16984, 20471, 24182, 25067, 25736, 26422]),
            multi_ctx_cdf(&[18904, 23325, 25242, 27432, 27898, 28258, 30758]),
            multi_ctx_cdf(&[10725, 17454, 20124, 22820, 24195, 25168, 26046]),
            multi_ctx_cdf(&[17125, 24273, 25814, 27492, 28214, 28704, 30592]),
            multi_ctx_cdf(&[13046, 23214, 24505, 25942, 27435, 28442, 29330]),
        ];

        // MV joint CDF (correlation between horizontal/vertical MV components)
        // Default values from AV1 spec / rav1d reference implementation
        // Source: https://github.com/memorysafety/rav1d (BSD-2-Clause license)
        // Forward CDF calculation from probabilities
        let mv_joint_counts = [
            4096,  // MV_JOINT_ZERO (both zero)
            7168,  // MV_JOINT_HNZVZ (H non-zero, V zero)
            8064,  // MV_JOINT_HZVNZ (H zero, V non-zero)
            13440, // MV_JOINT_HNZVNZ (both non-zero)
        ];
        let mut mv_joint_cdf = Vec::with_capacity(mv_joint_counts.len() + 1);
        mv_joint_cdf.push(0);
        let mut cumulative = 0;
        for &count in &mv_joint_counts {
            cumulative += count;
            mv_joint_cdf.push(cumulative);
        }
        // Runtime check that works in all builds (not just debug)
        // These are hardcoded constants that should sum to CDF_SCALE
        if *mv_joint_cdf.last().unwrap_or(&0) != CDF_SCALE {
            panic!(
                "MV joint CDF counts sum to {}, expected {} (CDF_SCALE) - counts may be miscalculated",
                mv_joint_cdf.last().unwrap_or(&0),
                CDF_SCALE
            );
        }
        let mv_joint_cdf = to_descending(&mv_joint_cdf);

        // MV sign CDF: 50/50 positive/negative (uniform)
        let mv_sign_cdf = vec![
            0,                               // Start
            (CDF_SCALE as f32 * 0.5) as u16, // Positive: 50%
            CDF_SCALE,                       // Negative: 50%
        ];
        let mv_sign_cdf = to_descending(&mv_sign_cdf);

        // MV class CDF (11 classes for magnitude)
        // Per AV1 spec Section 5.11.47 (Motion Vector Component)
        // Default values from AV1 spec / rav1d reference implementation
        // Source: https://github.com/memorysafety/rav1d (BSD-2-Clause license)
        // Forward CDF calculation from probabilities
        let mv_class_counts = [
            28672, // Class 0 (0 qpel)
            2304,  // Class 1 (±1 qpel)
            882,   // Class 2 (±2-3 qpel)
            462,   // Class 3 (±4-7 qpel)
            231,   // Class 4 (±8-15 qpel)
            105,   // Class 5 (±16-31 qpel)
            84,    // Class 6 (±32-63 qpel)
            17,    // Class 7 (±64-127 qpel)
            5,     // Class 8 (±128-255 qpel)
            5,     // Class 9 (±256-511 qpel)
            1,     // Class 10 (±512-1023 qpel)
        ];
        let mut mv_class_cdf = Vec::with_capacity(mv_class_counts.len() + 1);
        mv_class_cdf.push(0);
        let mut cumulative = 0;
        for &count in &mv_class_counts {
            cumulative += count;
            mv_class_cdf.push(cumulative);
        }
        // Runtime check that works in all builds (not just debug)
        // These are hardcoded constants that should sum to CDF_SCALE
        if *mv_class_cdf.last().unwrap_or(&0) != CDF_SCALE {
            panic!(
                "MV class CDF counts sum to {}, expected {} (CDF_SCALE) - counts may be miscalculated",
                mv_class_cdf.last().unwrap_or(&0),
                CDF_SCALE
            );
        }
        let mv_class_cdf = to_descending(&mv_class_cdf);

        // MV bit CDF: 50/50 for each bit (uniform)
        let mv_bit_cdf = vec![
            0,                               // Start
            (CDF_SCALE as f32 * 0.5) as u16, // 0: 50%
            CDF_SCALE,                       // 1: 50%
        ];
        let mv_bit_cdf = to_descending(&mv_bit_cdf);

        // delta_q (spec 5.11.38 `read_delta_qindex`) CDF -- real spec/rav1d default value
        // (`default_cdf.m.delta_q`, `src/cdf.c`, `CDF3(28160, 32120, 32677)`). Real 4-symbol
        // alphabet (`0..=2` used directly, `3` triggers `SymbolDecoder::read_delta_q`'s real
        // golomb extension) -- this crate's previous 5-symbol hand-fabricated version collapsed
        // a nonexistent "4+" outcome into the alphabet, a real shape mismatch (see that method's
        // doc for the desync this caused).
        let delta_q_cdf: Vec<u16> = multi_ctx_cdf(&[28160, 32120, 32677]);

        // delta_lf (spec 5.11.38 `read_delta_lf`) CDFs, one per real spec index (`0` for the
        // single-component case, `1..=4` for `delta_lf_multi`'s per-plane components -- see
        // `SymbolDecoder::read_delta_lf`'s doc) -- real spec/rav1d default value
        // (`default_cdf.m.delta_lf`, all 5 entries share the identical default,
        // `CDF3(28160, 32120, 32677)`, same as `delta_q`'s).
        let delta_lf_cdf: [Vec<u16>; 5] =
            std::array::from_fn(|_| multi_ctx_cdf(&[28160, 32120, 32677]));

        // txb_skip: real spec/rav1d default CDFs, indexed [tx_size_class][ctx 0..=6] (chroma=0
        // fixed -- see `txb_skip_cdf`'s doc). Source: rav1d `coef.skip`, real per-frame
        // qindex-bucket selection.
        let txb_skip_cdf: [[Vec<u16>; 7]; 5] = match qcat.min(3) {
            0 => [
                [31849, 5892, 12112, 21935, 20289, 27473, 32487].map(binary_ctx_cdf),
                [31548, 1549, 10130, 16656, 18591, 26308, 32537].map(binary_ctx_cdf),
                [29957, 5391, 18039, 23566, 22431, 25822, 32197].map(binary_ctx_cdf),
                [17920, 1818, 7282, 25273, 10923, 31554, 32624].map(binary_ctx_cdf),
                [6308, 117, 1638, 2161, 16384, 10923, 30247].map(binary_ctx_cdf),
            ],
            1 => [
                [30371, 7570, 13155, 20751, 20969, 27067, 32013].map(binary_ctx_cdf),
                [31782, 1836, 10689, 17604, 21622, 27518, 32399].map(binary_ctx_cdf),
                [31901, 10311, 18047, 24806, 23288, 27914, 32296].map(binary_ctx_cdf),
                [26726, 1045, 11703, 20590, 18554, 25970, 31938].map(binary_ctx_cdf),
                [26584, 188, 8847, 24519, 22938, 30583, 32608].map(binary_ctx_cdf),
            ],
            2 => [
                [29614, 9068, 12924, 19538, 17737, 24619, 30642].map(binary_ctx_cdf),
                [31957, 3230, 11153, 18123, 20143, 26536, 31986].map(binary_ctx_cdf),
                [32363, 10692, 19090, 24357, 24442, 28312, 32169].map(binary_ctx_cdf),
                [30669, 3832, 11663, 18889, 19782, 23313, 31330].map(binary_ctx_cdf),
                [28573, 3183, 17802, 25977, 26677, 27832, 32387].map(binary_ctx_cdf),
            ],
            3 => [
                [26887, 6729, 10361, 17442, 15045, 22478, 29072].map(binary_ctx_cdf),
                [31903, 2044, 7528, 14618, 16182, 24168, 31037].map(binary_ctx_cdf),
                [32510, 8430, 17318, 24154, 23674, 28789, 32139].map(binary_ctx_cdf),
                [31671, 2056, 11746, 16852, 18635, 24715, 31484].map(binary_ctx_cdf),
                [31539, 8433, 20576, 27904, 27852, 30026, 32441].map(binary_ctx_cdf),
            ],
            _ => unreachable!(),
        };

        // eob_bin: real spec/rav1d default CDFs, indexed [is_1d] (chroma=0 fixed -- see
        // `eob_bin_16_cdf`'s doc). Source: rav1d `eob_bin_16/64/256/1024`, real per-frame
        // qindex-bucket selection.
        let eob_bin_16_cdf = match qcat.min(3) {
            0 => [
                multi_ctx_cdf(&[840, 1039, 1980, 4895]),
                multi_ctx_cdf(&[370, 671, 1883, 4471]),
            ],
            1 => [
                multi_ctx_cdf(&[2125, 2551, 5165, 8946]),
                multi_ctx_cdf(&[513, 765, 1859, 6339]),
            ],
            2 => [
                multi_ctx_cdf(&[4016, 4897, 8881, 14968]),
                multi_ctx_cdf(&[716, 1105, 2646, 10056]),
            ],
            3 => [
                multi_ctx_cdf(&[6708, 8958, 14746, 22133]),
                multi_ctx_cdf(&[1222, 2074, 4783, 15410]),
            ],
            _ => unreachable!(),
        };
        let eob_bin_32_cdf = match qcat.min(3) {
            0 => [
                multi_ctx_cdf(&[400, 520, 977, 2102, 6542]),
                multi_ctx_cdf(&[210, 405, 1315, 3326, 7537]),
            ],
            1 => [
                multi_ctx_cdf(&[989, 1249, 2019, 4151, 10785]),
                multi_ctx_cdf(&[313, 441, 1099, 2917, 8562]),
            ],
            2 => [
                multi_ctx_cdf(&[2515, 3003, 4452, 8162, 16041]),
                multi_ctx_cdf(&[574, 821, 1836, 5089, 13128]),
            ],
            3 => [
                multi_ctx_cdf(&[4617, 5709, 8446, 13584, 23135]),
                multi_ctx_cdf(&[1156, 1702, 3675, 9274, 20539]),
            ],
            _ => unreachable!(),
        };
        let eob_bin_64_cdf = match qcat.min(3) {
            0 => [
                multi_ctx_cdf(&[329, 498, 1101, 1784, 3265, 7758]),
                multi_ctx_cdf(&[335, 730, 1459, 5494, 8755, 12997]),
            ],
            1 => [
                multi_ctx_cdf(&[1260, 1446, 2253, 3712, 6652, 13369]),
                multi_ctx_cdf(&[401, 605, 1029, 2563, 5845, 12626]),
            ],
            2 => [
                multi_ctx_cdf(&[2374, 2772, 4583, 7276, 12288, 19706]),
                multi_ctx_cdf(&[497, 810, 1315, 3000, 7004, 15641]),
            ],
            3 => [
                multi_ctx_cdf(&[6307, 7541, 12060, 16358, 22553, 27865]),
                multi_ctx_cdf(&[1289, 2320, 3971, 7926, 14153, 24291]),
            ],
            _ => unreachable!(),
        };
        let eob_bin_128_cdf = match qcat.min(3) {
            0 => [
                multi_ctx_cdf(&[219, 482, 1140, 2091, 3680, 6028, 12586]),
                multi_ctx_cdf(&[371, 699, 1254, 4830, 9479, 12562, 17497]),
            ],
            1 => [
                multi_ctx_cdf(&[685, 933, 1488, 2714, 4766, 8562, 19254]),
                multi_ctx_cdf(&[217, 352, 618, 2303, 5261, 9969, 17472]),
            ],
            2 => [
                multi_ctx_cdf(&[1366, 1738, 2527, 5016, 9355, 15797, 24643]),
                multi_ctx_cdf(&[354, 558, 944, 2760, 7287, 14037, 21779]),
            ],
            3 => [
                multi_ctx_cdf(&[3472, 4885, 7489, 12481, 18517, 24536, 29635]),
                multi_ctx_cdf(&[886, 1731, 3271, 8469, 15569, 22126, 28383]),
            ],
            _ => unreachable!(),
        };
        let eob_bin_256_cdf = match qcat.min(3) {
            0 => [
                multi_ctx_cdf(&[310, 584, 1887, 3589, 6168, 8611, 11352, 15652]),
                multi_ctx_cdf(&[998, 1850, 2998, 5604, 17341, 19888, 22899, 25583]),
            ],
            1 => [
                multi_ctx_cdf(&[1448, 2109, 4151, 6263, 9329, 13260, 17944, 23300]),
                multi_ctx_cdf(&[399, 1019, 1749, 3038, 10444, 15546, 22739, 27294]),
            ],
            2 => [
                multi_ctx_cdf(&[3089, 3920, 6038, 9460, 14266, 19881, 25766, 29176]),
                multi_ctx_cdf(&[1084, 2358, 3488, 5122, 11483, 18103, 26023, 29799]),
            ],
            3 => [
                multi_ctx_cdf(&[5348, 7113, 11820, 15924, 22106, 26777, 30334, 31757]),
                multi_ctx_cdf(&[2453, 4474, 6307, 8777, 16474, 22975, 29000, 31547]),
            ],
            _ => unreachable!(),
        };
        let eob_bin_512_cdf = match qcat.min(3) {
            0 => multi_ctx_cdf(&[641, 983, 3707, 5430, 10234, 14958, 18788, 23412, 26061]),
            1 => multi_ctx_cdf(&[1230, 2278, 5035, 7776, 11871, 15346, 19590, 24584, 28749]),
            2 => multi_ctx_cdf(&[2624, 3936, 6480, 9686, 13979, 17726, 23267, 28410, 31078]),
            3 => multi_ctx_cdf(&[5927, 7809, 10923, 14597, 19439, 24135, 28456, 31142, 32060]),
            _ => unreachable!(),
        };
        let eob_bin_1024_cdf = match qcat.min(3) {
            0 => multi_ctx_cdf(&[393, 421, 751, 1623, 3160, 6352, 13345, 18047, 22571, 25830]),
            1 => multi_ctx_cdf(&[
                696, 948, 3145, 5702, 9706, 13217, 17851, 21856, 25692, 28034,
            ]),
            2 => multi_ctx_cdf(&[
                2784, 3831, 7041, 10521, 14847, 18844, 23155, 26682, 29229, 31045,
            ]),
            3 => multi_ctx_cdf(&[
                6698, 8334, 11961, 15762, 20186, 23862, 27434, 29326, 31082, 32050,
            ]),
            _ => unreachable!(),
        };

        // eob_hi_bit: real spec/rav1d default CDFs, indexed [tx_size_class][eob_bin] (chroma=0
        // fixed). Source: rav1d `eob_hi_bit`, real per-frame qindex-bucket selection.
        let eob_hi_bit_cdf: [[Vec<u16>; 11]; 5] = match qcat.min(3) {
            0 => [
                [
                    16384, 16384, 16961, 17223, 7621, 16384, 16384, 16384, 16384, 16384, 16384,
                ]
                .map(binary_ctx_cdf),
                [
                    16384, 16384, 20401, 17025, 12845, 12873, 14094, 16384, 16384, 16384, 16384,
                ]
                .map(binary_ctx_cdf),
                [
                    16384, 16384, 23905, 17194, 16170, 17695, 13826, 15810, 12036, 16384, 16384,
                ]
                .map(binary_ctx_cdf),
                [
                    16384, 16384, 27399, 16327, 18071, 19584, 20721, 18432, 19560, 10150, 8805,
                ]
                .map(binary_ctx_cdf),
                [
                    16384, 16384, 23406, 21845, 18432, 16384, 17096, 12561, 17320, 22395, 21370,
                ]
                .map(binary_ctx_cdf),
            ],
            1 => [
                [
                    16384, 16384, 17471, 20223, 11357, 16384, 16384, 16384, 16384, 16384, 16384,
                ]
                .map(binary_ctx_cdf),
                [
                    16384, 16384, 20430, 20662, 15367, 16970, 14657, 16384, 16384, 16384, 16384,
                ]
                .map(binary_ctx_cdf),
                [
                    16384, 16384, 22409, 21012, 15650, 17395, 15469, 20205, 19511, 16384, 16384,
                ]
                .map(binary_ctx_cdf),
                [
                    16384, 16384, 25991, 20314, 17731, 19678, 18649, 17307, 21798, 17549, 15630,
                ]
                .map(binary_ctx_cdf),
                [
                    16384, 16384, 26605, 11304, 16726, 16560, 20866, 23524, 19878, 13469, 23084,
                ]
                .map(binary_ctx_cdf),
            ],
            2 => [
                [
                    16384, 16384, 18983, 20512, 14885, 16384, 16384, 16384, 16384, 16384, 16384,
                ]
                .map(binary_ctx_cdf),
                [
                    16384, 16384, 19139, 21487, 18959, 20910, 19089, 16384, 16384, 16384, 16384,
                ]
                .map(binary_ctx_cdf),
                [
                    16384, 16384, 19833, 21502, 17485, 20267, 18353, 23329, 21478, 16384, 16384,
                ]
                .map(binary_ctx_cdf),
                [
                    16384, 16384, 23312, 21607, 16526, 18957, 18034, 18934, 24247, 16921, 17080,
                ]
                .map(binary_ctx_cdf),
                [
                    16384, 16384, 26998, 16737, 17838, 18922, 19515, 18636, 17333, 15776, 22658,
                ]
                .map(binary_ctx_cdf),
            ],
            3 => [
                [
                    16384, 16384, 20177, 20789, 20262, 16384, 16384, 16384, 16384, 16384, 16384,
                ]
                .map(binary_ctx_cdf),
                [
                    16384, 16384, 20238, 21057, 19159, 22337, 20159, 16384, 16384, 16384, 16384,
                ]
                .map(binary_ctx_cdf),
                [
                    16384, 16384, 19941, 20527, 21470, 22487, 19558, 22354, 20331, 16384, 16384,
                ]
                .map(binary_ctx_cdf),
                [
                    16384, 16384, 21442, 22358, 18503, 20291, 19945, 21294, 21178, 19400, 10556,
                ]
                .map(binary_ctx_cdf),
                [
                    16384, 16384, 26064, 22098, 19613, 20525, 17595, 16618, 20497, 18989, 15513,
                ]
                .map(binary_ctx_cdf),
            ],
            _ => unreachable!(),
        };

        // coeff_base_eob: real spec/rav1d default CDFs, indexed [tx_size_class][ctx] (chroma=0
        // fixed). Source: rav1d `eob_base_tok`, real per-frame qindex-bucket selection.
        let coeff_base_eob_cdf: [[Vec<u16>; 4]; 5] = match qcat.min(3) {
            0 => [
                [
                    multi_ctx_cdf(&[17837, 29055]),
                    multi_ctx_cdf(&[29600, 31446]),
                    multi_ctx_cdf(&[30844, 31878]),
                    multi_ctx_cdf(&[24926, 28948]),
                ],
                [
                    multi_ctx_cdf(&[5717, 26477]),
                    multi_ctx_cdf(&[30491, 31703]),
                    multi_ctx_cdf(&[31550, 32158]),
                    multi_ctx_cdf(&[29648, 31491]),
                ],
                [
                    multi_ctx_cdf(&[1786, 12612]),
                    multi_ctx_cdf(&[30663, 31625]),
                    multi_ctx_cdf(&[32339, 32468]),
                    multi_ctx_cdf(&[31148, 31833]),
                ],
                [
                    multi_ctx_cdf(&[1787, 2532]),
                    multi_ctx_cdf(&[30832, 31662]),
                    multi_ctx_cdf(&[31824, 32682]),
                    multi_ctx_cdf(&[32133, 32569]),
                ],
                [
                    multi_ctx_cdf(&[1725, 3449]),
                    multi_ctx_cdf(&[31102, 31935]),
                    multi_ctx_cdf(&[32457, 32613]),
                    multi_ctx_cdf(&[32412, 32649]),
                ],
            ],
            1 => [
                [
                    multi_ctx_cdf(&[17560, 29888]),
                    multi_ctx_cdf(&[29671, 31549]),
                    multi_ctx_cdf(&[31007, 32056]),
                    multi_ctx_cdf(&[27286, 30006]),
                ],
                [
                    multi_ctx_cdf(&[15239, 29932]),
                    multi_ctx_cdf(&[31315, 32095]),
                    multi_ctx_cdf(&[32130, 32434]),
                    multi_ctx_cdf(&[30864, 31996]),
                ],
                [
                    multi_ctx_cdf(&[2644, 25198]),
                    multi_ctx_cdf(&[32038, 32451]),
                    multi_ctx_cdf(&[32639, 32695]),
                    multi_ctx_cdf(&[32166, 32518]),
                ],
                [
                    multi_ctx_cdf(&[1044, 2257]),
                    multi_ctx_cdf(&[30755, 31923]),
                    multi_ctx_cdf(&[32208, 32693]),
                    multi_ctx_cdf(&[32244, 32615]),
                ],
                [
                    multi_ctx_cdf(&[478, 1834]),
                    multi_ctx_cdf(&[31005, 31987]),
                    multi_ctx_cdf(&[32317, 32724]),
                    multi_ctx_cdf(&[30865, 32648]),
                ],
            ],
            2 => [
                [
                    multi_ctx_cdf(&[20092, 30774]),
                    multi_ctx_cdf(&[30695, 32020]),
                    multi_ctx_cdf(&[31131, 32103]),
                    multi_ctx_cdf(&[28666, 30870]),
                ],
                [
                    multi_ctx_cdf(&[18049, 30489]),
                    multi_ctx_cdf(&[31706, 32286]),
                    multi_ctx_cdf(&[32163, 32473]),
                    multi_ctx_cdf(&[31550, 32184]),
                ],
                [
                    multi_ctx_cdf(&[12854, 29093]),
                    multi_ctx_cdf(&[32272, 32558]),
                    multi_ctx_cdf(&[32667, 32729]),
                    multi_ctx_cdf(&[32306, 32585]),
                ],
                [
                    multi_ctx_cdf(&[2809, 19301]),
                    multi_ctx_cdf(&[32205, 32622]),
                    multi_ctx_cdf(&[32338, 32730]),
                    multi_ctx_cdf(&[31786, 32616]),
                ],
                [
                    multi_ctx_cdf(&[935, 3382]),
                    multi_ctx_cdf(&[30789, 31909]),
                    multi_ctx_cdf(&[32466, 32756]),
                    multi_ctx_cdf(&[30860, 32513]),
                ],
            ],
            3 => [
                [
                    multi_ctx_cdf(&[22497, 31198]),
                    multi_ctx_cdf(&[31715, 32495]),
                    multi_ctx_cdf(&[31606, 32337]),
                    multi_ctx_cdf(&[30388, 31990]),
                ],
                [
                    multi_ctx_cdf(&[21457, 31043]),
                    multi_ctx_cdf(&[31951, 32483]),
                    multi_ctx_cdf(&[32153, 32562]),
                    multi_ctx_cdf(&[31473, 32215]),
                ],
                [
                    multi_ctx_cdf(&[19980, 30591]),
                    multi_ctx_cdf(&[32219, 32597]),
                    multi_ctx_cdf(&[32581, 32706]),
                    multi_ctx_cdf(&[31803, 32287]),
                ],
                [
                    multi_ctx_cdf(&[24647, 30463]),
                    multi_ctx_cdf(&[32412, 32695]),
                    multi_ctx_cdf(&[32468, 32720]),
                    multi_ctx_cdf(&[31269, 32523]),
                ],
                [
                    multi_ctx_cdf(&[12358, 24977]),
                    multi_ctx_cdf(&[31331, 32385]),
                    multi_ctx_cdf(&[32634, 32756]),
                    multi_ctx_cdf(&[30411, 32548]),
                ],
            ],
            _ => unreachable!(),
        };

        // Chroma-plane residual CDFs -- see `txb_skip_cdf_chroma`'s doc for scope. Real chroma
        // (rav1d/dav1d `[chroma=1]` axis) default values, real per-frame qindex-bucket selection
        // (all 4 buckets, same as luma). `txb_skip`
        // indexed `[tx_size_class][ctx 0..=5]` -- ctx is the local remap (`TileContext::
        // txb_skip_context_chroma`'s doc) of dav1d's real chroma ctx 7..=12 from the same
        // `default_coef_cdf[qcat].skip[tx_size_class]` row luma's 0..=6 came from (`src/cdf.c`).
        let txb_skip_cdf_chroma: [[Vec<u16>; 6]; 4] = match qcat.min(3) {
            0 => [
                [7654, 19473, 29984, 9961, 30242, 32117].map(binary_ctx_cdf),
                [5403, 18096, 30003, 16384, 16384, 16384].map(binary_ctx_cdf),
                [3778, 15336, 28981, 16384, 16384, 16384].map(binary_ctx_cdf),
                [1366, 15628, 30462, 146, 5132, 31657].map(binary_ctx_cdf),
            ],
            1 => [
                [5495, 17942, 28280, 16384, 16384, 16384].map(binary_ctx_cdf),
                [4419, 16294, 28345, 16384, 16384, 16384].map(binary_ctx_cdf),
                [4215, 15756, 28341, 16384, 16384, 16384].map(binary_ctx_cdf),
                [5583, 21313, 29390, 641, 22265, 31452].map(binary_ctx_cdf),
            ],
            2 => [
                [4119, 16026, 25657, 16384, 16384, 16384].map(binary_ctx_cdf),
                [3050, 14603, 25155, 16384, 16384, 16384].map(binary_ctx_cdf),
                [3648, 15690, 26815, 16384, 16384, 16384].map(binary_ctx_cdf),
                [5124, 18719, 28468, 3082, 20982, 29443].map(binary_ctx_cdf),
            ],
            3 => [
                [2713, 11861, 20773, 16384, 16384, 16384].map(binary_ctx_cdf),
                [2786, 11194, 20155, 16384, 16384, 16384].map(binary_ctx_cdf),
                [3440, 13117, 22702, 16384, 16384, 16384].map(binary_ctx_cdf),
                [4656, 16074, 24704, 1806, 14645, 25336].map(binary_ctx_cdf),
            ],
            _ => unreachable!(),
        };
        // `dc_sign` chroma: real 3-context row (`default_coef_cdf[0].dc_sign[1]`, `src/cdf.c`).
        let dc_sign_cdf_chroma: [Vec<u16>; 3] = match qcat.min(3) {
            0 => [15232, 12928, 17280].map(binary_ctx_cdf),
            1 => [15232, 12928, 17280].map(binary_ctx_cdf),
            2 => [15232, 12928, 17280].map(binary_ctx_cdf),
            3 => [15232, 12928, 17280].map(binary_ctx_cdf),
            _ => unreachable!(),
        };
        let eob_bin_16_cdf_chroma = match qcat.min(3) {
            0 => multi_ctx_cdf(&[3247, 4950, 9688, 14563]),
            1 => multi_ctx_cdf(&[7637, 9498, 14259, 19108]),
            2 => multi_ctx_cdf(&[11139, 13270, 18241, 23566]),
            3 => multi_ctx_cdf(&[19575, 21766, 26044, 29709]),
            _ => unreachable!(),
        };
        let eob_bin_32_cdf_chroma = match qcat.min(3) {
            0 => multi_ctx_cdf(&[2636, 4273, 7588, 11794, 20401]),
            1 => multi_ctx_cdf(&[8394, 10352, 13932, 18855, 26014]),
            2 => multi_ctx_cdf(&[13468, 16303, 20361, 25105, 29281]),
            3 => multi_ctx_cdf(&[22086, 24282, 27010, 29770, 31743]),
            _ => unreachable!(),
        };
        let eob_bin_64_cdf_chroma = match qcat.min(3) {
            0 => multi_ctx_cdf(&[3505, 5304, 10086, 13814, 17684, 23370]),
            1 => multi_ctx_cdf(&[8609, 10612, 14624, 18714, 22614, 29024]),
            2 => multi_ctx_cdf(&[15050, 17126, 21410, 24886, 28156, 30726]),
            3 => multi_ctx_cdf(&[24212, 25708, 28268, 30035, 31307, 32049]),
            _ => unreachable!(),
        };
        let eob_bin_128_cdf_chroma = match qcat.min(3) {
            0 => multi_ctx_cdf(&[5245, 7456, 12880, 15852, 20033, 23932, 27608]),
            1 => multi_ctx_cdf(&[8045, 11200, 15497, 19595, 23948, 27408, 30938]),
            2 => multi_ctx_cdf(&[13627, 16246, 20173, 24429, 27948, 30415, 31863]),
            3 => multi_ctx_cdf(&[24313, 26062, 28385, 30107, 31217, 31898, 32345]),
            _ => unreachable!(),
        };
        let eob_bin_256_cdf_chroma = match qcat.min(3) {
            0 => multi_ctx_cdf(&[2520, 3240, 5952, 8870, 12577, 17558, 19954, 24168]),
            1 => multi_ctx_cdf(&[6402, 8148, 12623, 15072, 18728, 22847, 26447, 29377]),
            2 => multi_ctx_cdf(&[11514, 13794, 17480, 20754, 24361, 27378, 29492, 31277]),
            3 => multi_ctx_cdf(&[23110, 24597, 27140, 28894, 30167, 30927, 31392, 32094]),
            _ => unreachable!(),
        };
        let eob_bin_512_cdf_chroma = match qcat.min(3) {
            0 => multi_ctx_cdf(&[5095, 6446, 9996, 13354, 16017, 17986, 20919, 26129, 29140]),
            1 => multi_ctx_cdf(&[7265, 9979, 15819, 19250, 21780, 23846, 26478, 28396, 31811]),
            2 => multi_ctx_cdf(&[
                12015, 14769, 19588, 22052, 24222, 25812, 27300, 29219, 32114,
            ]),
            3 => multi_ctx_cdf(&[
                21093, 23043, 25742, 27658, 29097, 29716, 30073, 30820, 31956,
            ]),
            _ => unreachable!(),
        };
        let eob_bin_1024_cdf_chroma = match qcat.min(3) {
            0 => multi_ctx_cdf(&[
                1865, 1988, 2930, 4242, 10533, 16538, 21354, 27255, 28546, 31784,
            ]),
            1 => multi_ctx_cdf(&[
                2672, 3591, 9330, 17084, 22725, 24284, 26527, 28027, 28377, 30876,
            ]),
            2 => multi_ctx_cdf(&[
                9577, 12466, 17739, 20750, 22061, 23215, 24601, 25483, 25843, 32056,
            ]),
            3 => multi_ctx_cdf(&[
                20569, 22426, 25569, 26859, 28053, 28913, 29486, 29724, 29807, 32570,
            ]),
            _ => unreachable!(),
        };
        let eob_hi_bit_cdf_chroma: [[Vec<u16>; 11]; 4] = match qcat.min(3) {
            0 => [
                [
                    16384, 16384, 19069, 22525, 13377, 16384, 16384, 16384, 16384, 16384, 16384,
                ]
                .map(binary_ctx_cdf),
                [
                    16384, 16384, 20681, 20701, 15250, 15017, 14928, 16384, 16384, 16384, 16384,
                ]
                .map(binary_ctx_cdf),
                [
                    16384, 16384, 23959, 20799, 19021, 16203, 17886, 14144, 12010, 16384, 16384,
                ]
                .map(binary_ctx_cdf),
                [
                    16384, 16384, 24932, 20833, 12027, 16670, 19914, 15106, 17662, 13783, 28756,
                ]
                .map(binary_ctx_cdf),
            ],
            1 => [
                [
                    16384, 16384, 20335, 21667, 14818, 16384, 16384, 16384, 16384, 16384, 16384,
                ]
                .map(binary_ctx_cdf),
                [
                    16384, 16384, 22117, 22028, 18650, 16042, 15885, 16384, 16384, 16384, 16384,
                ]
                .map(binary_ctx_cdf),
                [
                    16384, 16384, 24220, 22480, 17737, 18916, 19268, 18412, 18844, 16384, 16384,
                ]
                .map(binary_ctx_cdf),
                [
                    16384, 16384, 26585, 21469, 20432, 17735, 19280, 15235, 20297, 22471, 28997,
                ]
                .map(binary_ctx_cdf),
            ],
            2 => [
                [
                    16384, 16384, 20090, 19444, 17286, 16384, 16384, 16384, 16384, 16384, 16384,
                ]
                .map(binary_ctx_cdf),
                [
                    16384, 16384, 20536, 20664, 20625, 19123, 14862, 16384, 16384, 16384, 16384,
                ]
                .map(binary_ctx_cdf),
                [
                    16384, 16384, 22041, 23434, 20001, 20554, 20951, 20145, 15562, 16384, 16384,
                ]
                .map(binary_ctx_cdf),
                [
                    16384, 16384, 26579, 24910, 18637, 19800, 20388, 9887, 15642, 30198, 24721,
                ]
                .map(binary_ctx_cdf),
            ],
            3 => [
                [
                    16384, 16384, 21416, 20855, 23410, 16384, 16384, 16384, 16384, 16384, 16384,
                ]
                .map(binary_ctx_cdf),
                [
                    16384, 16384, 20125, 20559, 21707, 22296, 17333, 16384, 16384, 16384, 16384,
                ]
                .map(binary_ctx_cdf),
                [
                    16384, 16384, 22752, 25006, 22075, 21576, 17740, 21690, 19211, 16384, 16384,
                ]
                .map(binary_ctx_cdf),
                [
                    16384, 16384, 24648, 24949, 20708, 23905, 20501, 9558, 9423, 30365, 19253,
                ]
                .map(binary_ctx_cdf),
            ],
            _ => unreachable!(),
        };
        let coeff_base_eob_cdf_chroma: [[Vec<u16>; 4]; 4] = match qcat.min(3) {
            0 => [
                [
                    multi_ctx_cdf(&[21365, 30026]),
                    multi_ctx_cdf(&[30512, 32423]),
                    multi_ctx_cdf(&[31658, 32621]),
                    multi_ctx_cdf(&[29630, 31881]),
                ],
                [
                    multi_ctx_cdf(&[12608, 27820]),
                    multi_ctx_cdf(&[30680, 32225]),
                    multi_ctx_cdf(&[30809, 32335]),
                    multi_ctx_cdf(&[31299, 32423]),
                ],
                [
                    multi_ctx_cdf(&[18857, 23865]),
                    multi_ctx_cdf(&[31428, 32428]),
                    multi_ctx_cdf(&[31744, 32373]),
                    multi_ctx_cdf(&[31775, 32526]),
                ],
                [
                    multi_ctx_cdf(&[13751, 22235]),
                    multi_ctx_cdf(&[32089, 32409]),
                    multi_ctx_cdf(&[27084, 27920]),
                    multi_ctx_cdf(&[29291, 32594]),
                ],
            ],
            1 => [
                [
                    multi_ctx_cdf(&[26594, 31212]),
                    multi_ctx_cdf(&[31208, 32582]),
                    multi_ctx_cdf(&[31835, 32637]),
                    multi_ctx_cdf(&[30595, 32206]),
                ],
                [
                    multi_ctx_cdf(&[26279, 30968]),
                    multi_ctx_cdf(&[31142, 32495]),
                    multi_ctx_cdf(&[31713, 32540]),
                    multi_ctx_cdf(&[31929, 32594]),
                ],
                [
                    multi_ctx_cdf(&[17187, 27668]),
                    multi_ctx_cdf(&[31714, 32550]),
                    multi_ctx_cdf(&[32283, 32678]),
                    multi_ctx_cdf(&[31930, 32563]),
                ],
                [
                    multi_ctx_cdf(&[21317, 26207]),
                    multi_ctx_cdf(&[29133, 30868]),
                    multi_ctx_cdf(&[29311, 31231]),
                    multi_ctx_cdf(&[29657, 31087]),
                ],
            ],
            2 => [
                [
                    multi_ctx_cdf(&[27258, 31095]),
                    multi_ctx_cdf(&[31804, 32623]),
                    multi_ctx_cdf(&[31763, 32528]),
                    multi_ctx_cdf(&[31438, 32506]),
                ],
                [
                    multi_ctx_cdf(&[27116, 30842]),
                    multi_ctx_cdf(&[31971, 32598]),
                    multi_ctx_cdf(&[32088, 32576]),
                    multi_ctx_cdf(&[32067, 32664]),
                ],
                [
                    multi_ctx_cdf(&[25476, 30366]),
                    multi_ctx_cdf(&[32169, 32687]),
                    multi_ctx_cdf(&[32479, 32689]),
                    multi_ctx_cdf(&[31673, 32634]),
                ],
                [
                    multi_ctx_cdf(&[22737, 29105]),
                    multi_ctx_cdf(&[30810, 32362]),
                    multi_ctx_cdf(&[30014, 32627]),
                    multi_ctx_cdf(&[30528, 32574]),
                ],
            ],
            3 => [
                [
                    multi_ctx_cdf(&[27877, 31584]),
                    multi_ctx_cdf(&[32170, 32728]),
                    multi_ctx_cdf(&[32155, 32688]),
                    multi_ctx_cdf(&[32219, 32702]),
                ],
                [
                    multi_ctx_cdf(&[27558, 31151]),
                    multi_ctx_cdf(&[32020, 32640]),
                    multi_ctx_cdf(&[32097, 32575]),
                    multi_ctx_cdf(&[32242, 32719]),
                ],
                [
                    multi_ctx_cdf(&[26473, 30507]),
                    multi_ctx_cdf(&[32431, 32723]),
                    multi_ctx_cdf(&[32196, 32611]),
                    multi_ctx_cdf(&[31588, 32528]),
                ],
                [
                    multi_ctx_cdf(&[28482, 31505]),
                    multi_ctx_cdf(&[32152, 32701]),
                    multi_ctx_cdf(&[31732, 32598]),
                    multi_ctx_cdf(&[31767, 32712]),
                ],
            ],
            _ => unreachable!(),
        };
        let coeff_base_cdf_chroma: [[Vec<u16>; 41]; 4] = match qcat.min(3) {
            0 => [
                [
                    multi_ctx_cdf(&[6302, 16444, 21761]),
                    multi_ctx_cdf(&[23040, 31538, 32475]),
                    multi_ctx_cdf(&[15196, 28452, 31496]),
                    multi_ctx_cdf(&[10020, 22946, 28514]),
                    multi_ctx_cdf(&[6533, 16862, 23501]),
                    multi_ctx_cdf(&[3538, 9816, 15076]),
                    multi_ctx_cdf(&[24444, 31875, 32525]),
                    multi_ctx_cdf(&[15881, 28924, 31635]),
                    multi_ctx_cdf(&[9922, 22873, 28466]),
                    multi_ctx_cdf(&[6527, 16966, 23691]),
                    multi_ctx_cdf(&[4114, 11303, 17220]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[20201, 30770, 32209]),
                    multi_ctx_cdf(&[14754, 28071, 31258]),
                    multi_ctx_cdf(&[8378, 20186, 26517]),
                    multi_ctx_cdf(&[5916, 15299, 21978]),
                    multi_ctx_cdf(&[4268, 11583, 17901]),
                    multi_ctx_cdf(&[24361, 32025, 32581]),
                    multi_ctx_cdf(&[18673, 30105, 31943]),
                    multi_ctx_cdf(&[10196, 22244, 27576]),
                    multi_ctx_cdf(&[5495, 14349, 20417]),
                    multi_ctx_cdf(&[2676, 7415, 11498]),
                    multi_ctx_cdf(&[24678, 31958, 32585]),
                    multi_ctx_cdf(&[18629, 29906, 31831]),
                    multi_ctx_cdf(&[9364, 20724, 26315]),
                    multi_ctx_cdf(&[4641, 12318, 18094]),
                    multi_ctx_cdf(&[2758, 7387, 11579]),
                    multi_ctx_cdf(&[25433, 31842, 32469]),
                    multi_ctx_cdf(&[18795, 29289, 31411]),
                    multi_ctx_cdf(&[7644, 17584, 23592]),
                    multi_ctx_cdf(&[3408, 9014, 15047]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                ],
                [
                    multi_ctx_cdf(&[6037, 16771, 21957]),
                    multi_ctx_cdf(&[24774, 31704, 32426]),
                    multi_ctx_cdf(&[16830, 28589, 31056]),
                    multi_ctx_cdf(&[10602, 22828, 27760]),
                    multi_ctx_cdf(&[6733, 16829, 23071]),
                    multi_ctx_cdf(&[3250, 8914, 13556]),
                    multi_ctx_cdf(&[25582, 32220, 32668]),
                    multi_ctx_cdf(&[18659, 30342, 32223]),
                    multi_ctx_cdf(&[12546, 26149, 30515]),
                    multi_ctx_cdf(&[8420, 20451, 26801]),
                    multi_ctx_cdf(&[4636, 12420, 18344]),
                    multi_ctx_cdf(&[27581, 32362, 32639]),
                    multi_ctx_cdf(&[18987, 30083, 31978]),
                    multi_ctx_cdf(&[11327, 24248, 29084]),
                    multi_ctx_cdf(&[7264, 17719, 24120]),
                    multi_ctx_cdf(&[3995, 10768, 16169]),
                    multi_ctx_cdf(&[25893, 31831, 32487]),
                    multi_ctx_cdf(&[16577, 28587, 31379]),
                    multi_ctx_cdf(&[10189, 22748, 28182]),
                    multi_ctx_cdf(&[6832, 17094, 23556]),
                    multi_ctx_cdf(&[3708, 10110, 15334]),
                    multi_ctx_cdf(&[25904, 32282, 32656]),
                    multi_ctx_cdf(&[19721, 30792, 32276]),
                    multi_ctx_cdf(&[12819, 26243, 30411]),
                    multi_ctx_cdf(&[8572, 20614, 26891]),
                    multi_ctx_cdf(&[5364, 14059, 20467]),
                    multi_ctx_cdf(&[26580, 32438, 32677]),
                    multi_ctx_cdf(&[20852, 31225, 32340]),
                    multi_ctx_cdf(&[12435, 25700, 29967]),
                    multi_ctx_cdf(&[8691, 20825, 26976]),
                    multi_ctx_cdf(&[4446, 12209, 17269]),
                    multi_ctx_cdf(&[27350, 32429, 32696]),
                    multi_ctx_cdf(&[21372, 30977, 32272]),
                    multi_ctx_cdf(&[12673, 25270, 29853]),
                    multi_ctx_cdf(&[9208, 20925, 26640]),
                    multi_ctx_cdf(&[5018, 13351, 18732]),
                    multi_ctx_cdf(&[27351, 32479, 32713]),
                    multi_ctx_cdf(&[21398, 31209, 32387]),
                    multi_ctx_cdf(&[12162, 25047, 29842]),
                    multi_ctx_cdf(&[7896, 18691, 25319]),
                    multi_ctx_cdf(&[4670, 12882, 18881]),
                ],
                [
                    multi_ctx_cdf(&[5673, 14302, 19711]),
                    multi_ctx_cdf(&[26251, 30701, 31834]),
                    multi_ctx_cdf(&[12782, 23783, 27803]),
                    multi_ctx_cdf(&[9127, 20657, 25808]),
                    multi_ctx_cdf(&[6368, 16208, 21462]),
                    multi_ctx_cdf(&[2465, 7177, 10822]),
                    multi_ctx_cdf(&[29961, 32563, 32719]),
                    multi_ctx_cdf(&[18318, 29891, 31949]),
                    multi_ctx_cdf(&[11361, 24514, 29357]),
                    multi_ctx_cdf(&[7900, 19603, 25607]),
                    multi_ctx_cdf(&[4002, 10590, 15546]),
                    multi_ctx_cdf(&[29637, 32310, 32595]),
                    multi_ctx_cdf(&[18296, 29913, 31809]),
                    multi_ctx_cdf(&[10144, 21515, 26871]),
                    multi_ctx_cdf(&[5358, 14322, 20394]),
                    multi_ctx_cdf(&[3067, 8362, 13346]),
                    multi_ctx_cdf(&[28652, 32470, 32676]),
                    multi_ctx_cdf(&[17538, 30771, 32209]),
                    multi_ctx_cdf(&[13924, 26882, 30494]),
                    multi_ctx_cdf(&[10496, 22837, 27869]),
                    multi_ctx_cdf(&[7236, 16396, 21621]),
                    multi_ctx_cdf(&[30743, 32687, 32746]),
                    multi_ctx_cdf(&[23006, 31676, 32489]),
                    multi_ctx_cdf(&[14494, 27828, 31120]),
                    multi_ctx_cdf(&[10174, 22801, 28352]),
                    multi_ctx_cdf(&[6242, 15281, 21043]),
                    multi_ctx_cdf(&[25817, 32243, 32720]),
                    multi_ctx_cdf(&[18618, 31367, 32325]),
                    multi_ctx_cdf(&[13997, 28318, 31878]),
                    multi_ctx_cdf(&[12255, 26534, 31383]),
                    multi_ctx_cdf(&[9561, 21588, 28450]),
                    multi_ctx_cdf(&[28188, 32635, 32724]),
                    multi_ctx_cdf(&[22060, 32365, 32728]),
                    multi_ctx_cdf(&[18102, 30690, 32528]),
                    multi_ctx_cdf(&[14196, 28864, 31999]),
                    multi_ctx_cdf(&[12262, 25792, 30865]),
                    multi_ctx_cdf(&[24176, 32109, 32628]),
                    multi_ctx_cdf(&[18280, 29681, 31963]),
                    multi_ctx_cdf(&[10205, 23703, 29664]),
                    multi_ctx_cdf(&[7889, 20025, 27676]),
                    multi_ctx_cdf(&[6060, 16743, 23970]),
                ],
                [
                    multi_ctx_cdf(&[2461, 7013, 9371]),
                    multi_ctx_cdf(&[24749, 29600, 30986]),
                    multi_ctx_cdf(&[9466, 19037, 22417]),
                    multi_ctx_cdf(&[3584, 9280, 14400]),
                    multi_ctx_cdf(&[1505, 3929, 5433]),
                    multi_ctx_cdf(&[677, 1500, 2736]),
                    multi_ctx_cdf(&[23987, 30702, 32117]),
                    multi_ctx_cdf(&[13554, 24571, 29263]),
                    multi_ctx_cdf(&[6211, 14556, 21155]),
                    multi_ctx_cdf(&[3135, 10972, 15625]),
                    multi_ctx_cdf(&[2435, 7127, 11427]),
                    multi_ctx_cdf(&[31300, 32532, 32550]),
                    multi_ctx_cdf(&[14757, 30365, 31954]),
                    multi_ctx_cdf(&[4405, 11612, 18553]),
                    multi_ctx_cdf(&[580, 4132, 7322]),
                    multi_ctx_cdf(&[1695, 10169, 14124]),
                    multi_ctx_cdf(&[30008, 32282, 32591]),
                    multi_ctx_cdf(&[19244, 30108, 31748]),
                    multi_ctx_cdf(&[11180, 24158, 29555]),
                    multi_ctx_cdf(&[5650, 14972, 19209]),
                    multi_ctx_cdf(&[2114, 5109, 8456]),
                    multi_ctx_cdf(&[31856, 32716, 32748]),
                    multi_ctx_cdf(&[23012, 31664, 32572]),
                    multi_ctx_cdf(&[13694, 26656, 30636]),
                    multi_ctx_cdf(&[8142, 19508, 26093]),
                    multi_ctx_cdf(&[4253, 10955, 16724]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                ],
            ],
            1 => [
                [
                    multi_ctx_cdf(&[8455, 19003, 24368]),
                    multi_ctx_cdf(&[23563, 32021, 32604]),
                    multi_ctx_cdf(&[16237, 29446, 31935]),
                    multi_ctx_cdf(&[10724, 23999, 29358]),
                    multi_ctx_cdf(&[6725, 17528, 24416]),
                    multi_ctx_cdf(&[3927, 10927, 16825]),
                    multi_ctx_cdf(&[26313, 32288, 32634]),
                    multi_ctx_cdf(&[17430, 30095, 32095]),
                    multi_ctx_cdf(&[11116, 24606, 29679]),
                    multi_ctx_cdf(&[7195, 18384, 25269]),
                    multi_ctx_cdf(&[4726, 12852, 19315]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[22822, 31648, 32483]),
                    multi_ctx_cdf(&[16724, 29633, 31929]),
                    multi_ctx_cdf(&[10261, 23033, 28725]),
                    multi_ctx_cdf(&[7029, 17840, 24528]),
                    multi_ctx_cdf(&[4867, 13886, 21502]),
                    multi_ctx_cdf(&[25298, 31892, 32491]),
                    multi_ctx_cdf(&[17809, 29330, 31512]),
                    multi_ctx_cdf(&[9668, 21329, 26579]),
                    multi_ctx_cdf(&[4774, 12956, 18976]),
                    multi_ctx_cdf(&[2322, 7030, 11540]),
                    multi_ctx_cdf(&[25472, 31920, 32543]),
                    multi_ctx_cdf(&[17957, 29387, 31632]),
                    multi_ctx_cdf(&[9196, 20593, 26400]),
                    multi_ctx_cdf(&[4680, 12705, 19202]),
                    multi_ctx_cdf(&[2917, 8456, 13436]),
                    multi_ctx_cdf(&[26471, 32059, 32574]),
                    multi_ctx_cdf(&[18458, 29783, 31909]),
                    multi_ctx_cdf(&[8400, 19464, 25956]),
                    multi_ctx_cdf(&[3812, 10973, 17206]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                ],
                [
                    multi_ctx_cdf(&[8639, 19339, 24429]),
                    multi_ctx_cdf(&[24404, 31837, 32525]),
                    multi_ctx_cdf(&[16997, 29425, 31784]),
                    multi_ctx_cdf(&[11253, 24234, 29149]),
                    multi_ctx_cdf(&[6751, 17394, 24028]),
                    multi_ctx_cdf(&[3490, 9830, 15191]),
                    multi_ctx_cdf(&[26283, 32471, 32714]),
                    multi_ctx_cdf(&[19599, 31168, 32442]),
                    multi_ctx_cdf(&[13146, 26954, 30893]),
                    multi_ctx_cdf(&[8214, 20588, 26890]),
                    multi_ctx_cdf(&[4699, 13081, 19300]),
                    multi_ctx_cdf(&[28212, 32458, 32669]),
                    multi_ctx_cdf(&[18594, 30316, 32100]),
                    multi_ctx_cdf(&[11219, 24408, 29234]),
                    multi_ctx_cdf(&[6865, 17656, 24149]),
                    multi_ctx_cdf(&[3678, 10362, 16006]),
                    multi_ctx_cdf(&[25825, 32136, 32616]),
                    multi_ctx_cdf(&[17313, 29853, 32021]),
                    multi_ctx_cdf(&[11197, 24471, 29472]),
                    multi_ctx_cdf(&[6947, 17781, 24405]),
                    multi_ctx_cdf(&[3768, 10660, 16261]),
                    multi_ctx_cdf(&[27352, 32500, 32706]),
                    multi_ctx_cdf(&[20850, 31468, 32469]),
                    multi_ctx_cdf(&[14021, 27707, 31133]),
                    multi_ctx_cdf(&[8964, 21748, 27838]),
                    multi_ctx_cdf(&[5437, 14665, 21187]),
                    multi_ctx_cdf(&[26304, 32492, 32698]),
                    multi_ctx_cdf(&[20409, 31380, 32385]),
                    multi_ctx_cdf(&[13682, 27222, 30632]),
                    multi_ctx_cdf(&[8974, 21236, 26685]),
                    multi_ctx_cdf(&[4234, 11665, 16934]),
                    multi_ctx_cdf(&[26273, 32357, 32711]),
                    multi_ctx_cdf(&[20672, 31242, 32441]),
                    multi_ctx_cdf(&[14172, 27254, 30902]),
                    multi_ctx_cdf(&[9870, 21898, 27275]),
                    multi_ctx_cdf(&[5164, 13506, 19270]),
                    multi_ctx_cdf(&[26725, 32459, 32728]),
                    multi_ctx_cdf(&[20991, 31442, 32527]),
                    multi_ctx_cdf(&[13071, 26434, 30811]),
                    multi_ctx_cdf(&[8184, 20090, 26742]),
                    multi_ctx_cdf(&[4803, 13255, 19895]),
                ],
                [
                    multi_ctx_cdf(&[6465, 16958, 21688]),
                    multi_ctx_cdf(&[25199, 31514, 32360]),
                    multi_ctx_cdf(&[14774, 27149, 30607]),
                    multi_ctx_cdf(&[9257, 21438, 26972]),
                    multi_ctx_cdf(&[5723, 15183, 21882]),
                    multi_ctx_cdf(&[3150, 8879, 13731]),
                    multi_ctx_cdf(&[26989, 32262, 32682]),
                    multi_ctx_cdf(&[17396, 29937, 32085]),
                    multi_ctx_cdf(&[11387, 24901, 29784]),
                    multi_ctx_cdf(&[7289, 18821, 25548]),
                    multi_ctx_cdf(&[3734, 10577, 16086]),
                    multi_ctx_cdf(&[29728, 32501, 32695]),
                    multi_ctx_cdf(&[17431, 29701, 31903]),
                    multi_ctx_cdf(&[9921, 22826, 28300]),
                    multi_ctx_cdf(&[5896, 15434, 22068]),
                    multi_ctx_cdf(&[3430, 9646, 14757]),
                    multi_ctx_cdf(&[28614, 32511, 32705]),
                    multi_ctx_cdf(&[19364, 30638, 32263]),
                    multi_ctx_cdf(&[13129, 26254, 30402]),
                    multi_ctx_cdf(&[8754, 20484, 26440]),
                    multi_ctx_cdf(&[4378, 11607, 17110]),
                    multi_ctx_cdf(&[30292, 32671, 32744]),
                    multi_ctx_cdf(&[21780, 31603, 32501]),
                    multi_ctx_cdf(&[14314, 27829, 31291]),
                    multi_ctx_cdf(&[9611, 22327, 28263]),
                    multi_ctx_cdf(&[4890, 13087, 19065]),
                    multi_ctx_cdf(&[25862, 32567, 32733]),
                    multi_ctx_cdf(&[20794, 32050, 32567]),
                    multi_ctx_cdf(&[17243, 30625, 32254]),
                    multi_ctx_cdf(&[13283, 27628, 31474]),
                    multi_ctx_cdf(&[9669, 22532, 28918]),
                    multi_ctx_cdf(&[27435, 32697, 32748]),
                    multi_ctx_cdf(&[24922, 32390, 32714]),
                    multi_ctx_cdf(&[21449, 31504, 32536]),
                    multi_ctx_cdf(&[16392, 29729, 31832]),
                    multi_ctx_cdf(&[11692, 24884, 29076]),
                    multi_ctx_cdf(&[24193, 32290, 32735]),
                    multi_ctx_cdf(&[18909, 31104, 32563]),
                    multi_ctx_cdf(&[12236, 26841, 31403]),
                    multi_ctx_cdf(&[8171, 21840, 29082]),
                    multi_ctx_cdf(&[7224, 17280, 25275]),
                ],
                [
                    multi_ctx_cdf(&[5244, 12150, 16906]),
                    multi_ctx_cdf(&[20486, 26858, 29701]),
                    multi_ctx_cdf(&[7756, 18317, 23735]),
                    multi_ctx_cdf(&[3452, 9256, 13146]),
                    multi_ctx_cdf(&[2020, 5206, 8229]),
                    multi_ctx_cdf(&[1801, 4993, 7903]),
                    multi_ctx_cdf(&[27051, 31858, 32531]),
                    multi_ctx_cdf(&[15988, 27531, 30619]),
                    multi_ctx_cdf(&[9188, 21484, 26719]),
                    multi_ctx_cdf(&[6273, 17186, 23800]),
                    multi_ctx_cdf(&[3108, 9355, 14764]),
                    multi_ctx_cdf(&[31076, 32520, 32680]),
                    multi_ctx_cdf(&[18119, 30037, 31850]),
                    multi_ctx_cdf(&[10244, 22969, 27472]),
                    multi_ctx_cdf(&[4692, 14077, 19273]),
                    multi_ctx_cdf(&[3694, 11677, 17556]),
                    multi_ctx_cdf(&[30060, 32581, 32720]),
                    multi_ctx_cdf(&[21011, 30775, 32120]),
                    multi_ctx_cdf(&[11931, 24820, 29289]),
                    multi_ctx_cdf(&[7119, 17662, 24356]),
                    multi_ctx_cdf(&[3833, 10706, 16304]),
                    multi_ctx_cdf(&[31954, 32731, 32748]),
                    multi_ctx_cdf(&[23913, 31724, 32489]),
                    multi_ctx_cdf(&[15520, 28060, 31286]),
                    multi_ctx_cdf(&[11517, 23008, 28571]),
                    multi_ctx_cdf(&[6193, 14508, 20629]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                ],
            ],
            2 => [
                [
                    multi_ctx_cdf(&[10808, 22056, 26896]),
                    multi_ctx_cdf(&[25739, 32313, 32676]),
                    multi_ctx_cdf(&[17288, 30203, 32221]),
                    multi_ctx_cdf(&[11359, 24878, 29896]),
                    multi_ctx_cdf(&[6949, 17767, 24893]),
                    multi_ctx_cdf(&[4287, 11796, 18071]),
                    multi_ctx_cdf(&[27880, 32521, 32705]),
                    multi_ctx_cdf(&[19038, 31004, 32414]),
                    multi_ctx_cdf(&[12564, 26345, 30768]),
                    multi_ctx_cdf(&[8269, 19947, 26779]),
                    multi_ctx_cdf(&[5674, 14657, 21674]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[25742, 32319, 32671]),
                    multi_ctx_cdf(&[19557, 31164, 32454]),
                    multi_ctx_cdf(&[13381, 26381, 30755]),
                    multi_ctx_cdf(&[10101, 21466, 26722]),
                    multi_ctx_cdf(&[9209, 19650, 26825]),
                    multi_ctx_cdf(&[27107, 31917, 32432]),
                    multi_ctx_cdf(&[18056, 28893, 31203]),
                    multi_ctx_cdf(&[10200, 21434, 26764]),
                    multi_ctx_cdf(&[4660, 12913, 19502]),
                    multi_ctx_cdf(&[2368, 6930, 12504]),
                    multi_ctx_cdf(&[26960, 32158, 32613]),
                    multi_ctx_cdf(&[18628, 30005, 32031]),
                    multi_ctx_cdf(&[10233, 22442, 28232]),
                    multi_ctx_cdf(&[5471, 14630, 21516]),
                    multi_ctx_cdf(&[3235, 10767, 17109]),
                    multi_ctx_cdf(&[27696, 32440, 32692]),
                    multi_ctx_cdf(&[20032, 31167, 32438]),
                    multi_ctx_cdf(&[8700, 21341, 28442]),
                    multi_ctx_cdf(&[5662, 14831, 21795]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                ],
                [
                    multi_ctx_cdf(&[11016, 22111, 26794]),
                    multi_ctx_cdf(&[25946, 32357, 32677]),
                    multi_ctx_cdf(&[17890, 30452, 32252]),
                    multi_ctx_cdf(&[11678, 25142, 29816]),
                    multi_ctx_cdf(&[6720, 17534, 24584]),
                    multi_ctx_cdf(&[4230, 11665, 17820]),
                    multi_ctx_cdf(&[28400, 32623, 32747]),
                    multi_ctx_cdf(&[21164, 31668, 32575]),
                    multi_ctx_cdf(&[13572, 27388, 31182]),
                    multi_ctx_cdf(&[8234, 20750, 27358]),
                    multi_ctx_cdf(&[5065, 14055, 20897]),
                    multi_ctx_cdf(&[28981, 32547, 32705]),
                    multi_ctx_cdf(&[18681, 30543, 32239]),
                    multi_ctx_cdf(&[10919, 24075, 29286]),
                    multi_ctx_cdf(&[6431, 17199, 24077]),
                    multi_ctx_cdf(&[3819, 10464, 16618]),
                    multi_ctx_cdf(&[26870, 32467, 32693]),
                    multi_ctx_cdf(&[19041, 30831, 32347]),
                    multi_ctx_cdf(&[11794, 25211, 30016]),
                    multi_ctx_cdf(&[6888, 18019, 24970]),
                    multi_ctx_cdf(&[4370, 12363, 18992]),
                    multi_ctx_cdf(&[29578, 32670, 32744]),
                    multi_ctx_cdf(&[23159, 32007, 32613]),
                    multi_ctx_cdf(&[15315, 28669, 31676]),
                    multi_ctx_cdf(&[9298, 22607, 28782]),
                    multi_ctx_cdf(&[6144, 15913, 22968]),
                    multi_ctx_cdf(&[28110, 32499, 32669]),
                    multi_ctx_cdf(&[21574, 30937, 32015]),
                    multi_ctx_cdf(&[12759, 24818, 28727]),
                    multi_ctx_cdf(&[6545, 16761, 23042]),
                    multi_ctx_cdf(&[3649, 10597, 16833]),
                    multi_ctx_cdf(&[28163, 32552, 32728]),
                    multi_ctx_cdf(&[22101, 31469, 32464]),
                    multi_ctx_cdf(&[13160, 25472, 30143]),
                    multi_ctx_cdf(&[7303, 18684, 25468]),
                    multi_ctx_cdf(&[5241, 13975, 20955]),
                    multi_ctx_cdf(&[28400, 32631, 32744]),
                    multi_ctx_cdf(&[22104, 31793, 32603]),
                    multi_ctx_cdf(&[13557, 26571, 30846]),
                    multi_ctx_cdf(&[7749, 19861, 26675]),
                    multi_ctx_cdf(&[4873, 14030, 21234]),
                ],
                [
                    multi_ctx_cdf(&[9185, 19694, 24688]),
                    multi_ctx_cdf(&[26081, 31985, 32621]),
                    multi_ctx_cdf(&[16015, 29000, 31787]),
                    multi_ctx_cdf(&[10542, 23690, 29206]),
                    multi_ctx_cdf(&[6732, 17945, 24677]),
                    multi_ctx_cdf(&[3916, 11039, 16722]),
                    multi_ctx_cdf(&[28224, 32566, 32744]),
                    multi_ctx_cdf(&[19100, 31138, 32485]),
                    multi_ctx_cdf(&[12528, 26620, 30879]),
                    multi_ctx_cdf(&[7741, 20277, 26885]),
                    multi_ctx_cdf(&[4566, 12845, 18990]),
                    multi_ctx_cdf(&[29933, 32593, 32718]),
                    multi_ctx_cdf(&[17670, 30333, 32155]),
                    multi_ctx_cdf(&[10385, 23600, 28909]),
                    multi_ctx_cdf(&[6243, 16236, 22407]),
                    multi_ctx_cdf(&[3976, 10389, 16017]),
                    multi_ctx_cdf(&[28377, 32561, 32738]),
                    multi_ctx_cdf(&[19366, 31175, 32482]),
                    multi_ctx_cdf(&[13327, 27175, 31094]),
                    multi_ctx_cdf(&[8258, 20769, 27143]),
                    multi_ctx_cdf(&[4703, 13198, 19527]),
                    multi_ctx_cdf(&[31086, 32706, 32748]),
                    multi_ctx_cdf(&[22853, 31902, 32583]),
                    multi_ctx_cdf(&[14759, 28186, 31419]),
                    multi_ctx_cdf(&[9284, 22382, 28348]),
                    multi_ctx_cdf(&[5585, 15192, 21868]),
                    multi_ctx_cdf(&[28291, 32652, 32746]),
                    multi_ctx_cdf(&[19849, 32107, 32571]),
                    multi_ctx_cdf(&[14834, 26818, 29214]),
                    multi_ctx_cdf(&[10306, 22594, 28672]),
                    multi_ctx_cdf(&[6615, 17384, 23384]),
                    multi_ctx_cdf(&[28947, 32604, 32745]),
                    multi_ctx_cdf(&[25625, 32289, 32646]),
                    multi_ctx_cdf(&[18758, 28672, 31403]),
                    multi_ctx_cdf(&[10017, 23430, 28523]),
                    multi_ctx_cdf(&[6862, 15269, 22131]),
                    multi_ctx_cdf(&[23933, 32509, 32739]),
                    multi_ctx_cdf(&[19927, 31495, 32631]),
                    multi_ctx_cdf(&[11903, 26023, 30621]),
                    multi_ctx_cdf(&[7026, 20094, 27252]),
                    multi_ctx_cdf(&[5998, 18106, 24437]),
                ],
                [
                    multi_ctx_cdf(&[10202, 20633, 25484]),
                    multi_ctx_cdf(&[27336, 31445, 32352]),
                    multi_ctx_cdf(&[12420, 24384, 28552]),
                    multi_ctx_cdf(&[7648, 18115, 23856]),
                    multi_ctx_cdf(&[5662, 14341, 19902]),
                    multi_ctx_cdf(&[3611, 10328, 15390]),
                    multi_ctx_cdf(&[30945, 32616, 32736]),
                    multi_ctx_cdf(&[18682, 30505, 32253]),
                    multi_ctx_cdf(&[11513, 25336, 30203]),
                    multi_ctx_cdf(&[7449, 19452, 26148]),
                    multi_ctx_cdf(&[4482, 13051, 18886]),
                    multi_ctx_cdf(&[32022, 32690, 32747]),
                    multi_ctx_cdf(&[18578, 30501, 32146]),
                    multi_ctx_cdf(&[11249, 23368, 28631]),
                    multi_ctx_cdf(&[5645, 16958, 22158]),
                    multi_ctx_cdf(&[5009, 11444, 16637]),
                    multi_ctx_cdf(&[31357, 32710, 32748]),
                    multi_ctx_cdf(&[21552, 31494, 32504]),
                    multi_ctx_cdf(&[13891, 27677, 31340]),
                    multi_ctx_cdf(&[9051, 22098, 28172]),
                    multi_ctx_cdf(&[5190, 13377, 19486]),
                    multi_ctx_cdf(&[32364, 32740, 32748]),
                    multi_ctx_cdf(&[24839, 31907, 32551]),
                    multi_ctx_cdf(&[17160, 28779, 31696]),
                    multi_ctx_cdf(&[12452, 24137, 29602]),
                    multi_ctx_cdf(&[6165, 15389, 22477]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                ],
            ],
            3 => [
                [
                    multi_ctx_cdf(&[9320, 22102, 27840]),
                    multi_ctx_cdf(&[27057, 32464, 32724]),
                    multi_ctx_cdf(&[16331, 30268, 32309]),
                    multi_ctx_cdf(&[10319, 23935, 29720]),
                    multi_ctx_cdf(&[6189, 16448, 24106]),
                    multi_ctx_cdf(&[3589, 10884, 18808]),
                    multi_ctx_cdf(&[29026, 32624, 32748]),
                    multi_ctx_cdf(&[19226, 31507, 32587]),
                    multi_ctx_cdf(&[12692, 26921, 31203]),
                    multi_ctx_cdf(&[7049, 19532, 27635]),
                    multi_ctx_cdf(&[7727, 15669, 23252]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[28056, 32625, 32748]),
                    multi_ctx_cdf(&[22383, 32075, 32669]),
                    multi_ctx_cdf(&[15417, 27098, 31749]),
                    multi_ctx_cdf(&[18127, 26493, 27190]),
                    multi_ctx_cdf(&[5461, 16384, 21845]),
                    multi_ctx_cdf(&[27982, 32091, 32584]),
                    multi_ctx_cdf(&[19045, 29868, 31972]),
                    multi_ctx_cdf(&[10397, 22266, 27932]),
                    multi_ctx_cdf(&[5990, 13697, 21500]),
                    multi_ctx_cdf(&[1792, 6912, 15104]),
                    multi_ctx_cdf(&[28198, 32501, 32718]),
                    multi_ctx_cdf(&[21534, 31521, 32569]),
                    multi_ctx_cdf(&[11109, 25217, 30017]),
                    multi_ctx_cdf(&[5671, 15124, 26151]),
                    multi_ctx_cdf(&[4681, 14043, 18725]),
                    multi_ctx_cdf(&[28688, 32580, 32741]),
                    multi_ctx_cdf(&[22576, 32079, 32661]),
                    multi_ctx_cdf(&[10627, 22141, 28340]),
                    multi_ctx_cdf(&[9362, 14043, 28087]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                ],
                [
                    multi_ctx_cdf(&[8626, 20271, 26216]),
                    multi_ctx_cdf(&[26707, 32406, 32711]),
                    multi_ctx_cdf(&[16999, 30329, 32286]),
                    multi_ctx_cdf(&[11445, 25123, 30286]),
                    multi_ctx_cdf(&[6411, 18828, 25601]),
                    multi_ctx_cdf(&[6801, 12458, 20248]),
                    multi_ctx_cdf(&[29918, 32682, 32748]),
                    multi_ctx_cdf(&[20649, 31739, 32618]),
                    multi_ctx_cdf(&[12879, 27773, 31581]),
                    multi_ctx_cdf(&[7896, 21751, 28244]),
                    multi_ctx_cdf(&[5260, 14870, 23698]),
                    multi_ctx_cdf(&[29252, 32593, 32731]),
                    multi_ctx_cdf(&[17072, 30460, 32294]),
                    multi_ctx_cdf(&[10653, 24143, 29365]),
                    multi_ctx_cdf(&[6536, 17490, 23983]),
                    multi_ctx_cdf(&[4929, 13170, 20085]),
                    multi_ctx_cdf(&[28137, 32518, 32715]),
                    multi_ctx_cdf(&[18171, 30784, 32407]),
                    multi_ctx_cdf(&[11437, 25436, 30459]),
                    multi_ctx_cdf(&[7252, 18534, 26176]),
                    multi_ctx_cdf(&[4126, 13353, 20978]),
                    multi_ctx_cdf(&[31162, 32726, 32748]),
                    multi_ctx_cdf(&[23017, 32222, 32701]),
                    multi_ctx_cdf(&[15629, 29233, 32046]),
                    multi_ctx_cdf(&[9387, 22621, 29480]),
                    multi_ctx_cdf(&[6922, 17616, 25010]),
                    multi_ctx_cdf(&[28838, 32265, 32614]),
                    multi_ctx_cdf(&[19701, 30206, 31920]),
                    multi_ctx_cdf(&[11214, 22410, 27933]),
                    multi_ctx_cdf(&[5320, 14177, 23034]),
                    multi_ctx_cdf(&[5049, 12881, 17827]),
                    multi_ctx_cdf(&[27484, 32471, 32734]),
                    multi_ctx_cdf(&[21076, 31526, 32561]),
                    multi_ctx_cdf(&[12707, 26303, 31211]),
                    multi_ctx_cdf(&[8169, 21722, 28219]),
                    multi_ctx_cdf(&[6045, 19406, 27042]),
                    multi_ctx_cdf(&[27753, 32572, 32745]),
                    multi_ctx_cdf(&[20832, 31878, 32653]),
                    multi_ctx_cdf(&[13250, 27356, 31674]),
                    multi_ctx_cdf(&[7718, 21508, 29858]),
                    multi_ctx_cdf(&[7209, 18350, 25559]),
                ],
                [
                    multi_ctx_cdf(&[7833, 18369, 24095]),
                    multi_ctx_cdf(&[26650, 32273, 32702]),
                    multi_ctx_cdf(&[16371, 29961, 32191]),
                    multi_ctx_cdf(&[11055, 24082, 29629]),
                    multi_ctx_cdf(&[6892, 18644, 25400]),
                    multi_ctx_cdf(&[5006, 13057, 19240]),
                    multi_ctx_cdf(&[29834, 32666, 32748]),
                    multi_ctx_cdf(&[19577, 31335, 32570]),
                    multi_ctx_cdf(&[12253, 26509, 31122]),
                    multi_ctx_cdf(&[7991, 20772, 27711]),
                    multi_ctx_cdf(&[5677, 15910, 23059]),
                    multi_ctx_cdf(&[30109, 32532, 32720]),
                    multi_ctx_cdf(&[16747, 30166, 32252]),
                    multi_ctx_cdf(&[10134, 23542, 29184]),
                    multi_ctx_cdf(&[5791, 16176, 23556]),
                    multi_ctx_cdf(&[4362, 10414, 17284]),
                    multi_ctx_cdf(&[29492, 32626, 32748]),
                    multi_ctx_cdf(&[19894, 31402, 32525]),
                    multi_ctx_cdf(&[12942, 27071, 30869]),
                    multi_ctx_cdf(&[8346, 21216, 27405]),
                    multi_ctx_cdf(&[6572, 17087, 23859]),
                    multi_ctx_cdf(&[32035, 32735, 32748]),
                    multi_ctx_cdf(&[22957, 31838, 32618]),
                    multi_ctx_cdf(&[14724, 28572, 31772]),
                    multi_ctx_cdf(&[10364, 23999, 29553]),
                    multi_ctx_cdf(&[7004, 18433, 25655]),
                    multi_ctx_cdf(&[27528, 32277, 32681]),
                    multi_ctx_cdf(&[16959, 31171, 32096]),
                    multi_ctx_cdf(&[10486, 23593, 27962]),
                    multi_ctx_cdf(&[8192, 16384, 23211]),
                    multi_ctx_cdf(&[8937, 17873, 20852]),
                    multi_ctx_cdf(&[27715, 32002, 32615]),
                    multi_ctx_cdf(&[15073, 29491, 31676]),
                    multi_ctx_cdf(&[11264, 24576, 28672]),
                    multi_ctx_cdf(&[2341, 18725, 23406]),
                    multi_ctx_cdf(&[7282, 18204, 25486]),
                    multi_ctx_cdf(&[28547, 32213, 32657]),
                    multi_ctx_cdf(&[20788, 29773, 32239]),
                    multi_ctx_cdf(&[6780, 21469, 30508]),
                    multi_ctx_cdf(&[5958, 14895, 23831]),
                    multi_ctx_cdf(&[16384, 21845, 27307]),
                ],
                [
                    multi_ctx_cdf(&[11206, 21090, 26561]),
                    multi_ctx_cdf(&[28759, 32279, 32671]),
                    multi_ctx_cdf(&[14171, 27952, 31569]),
                    multi_ctx_cdf(&[9743, 22907, 29141]),
                    multi_ctx_cdf(&[6871, 17886, 24868]),
                    multi_ctx_cdf(&[4960, 13152, 19315]),
                    multi_ctx_cdf(&[31077, 32661, 32748]),
                    multi_ctx_cdf(&[19400, 31195, 32515]),
                    multi_ctx_cdf(&[12752, 26858, 31040]),
                    multi_ctx_cdf(&[8370, 22098, 28591]),
                    multi_ctx_cdf(&[5457, 15373, 22298]),
                    multi_ctx_cdf(&[31697, 32706, 32748]),
                    multi_ctx_cdf(&[17860, 30657, 32333]),
                    multi_ctx_cdf(&[12510, 24812, 29261]),
                    multi_ctx_cdf(&[6180, 19124, 24722]),
                    multi_ctx_cdf(&[5041, 13548, 17959]),
                    multi_ctx_cdf(&[31552, 32716, 32748]),
                    multi_ctx_cdf(&[21908, 31769, 32623]),
                    multi_ctx_cdf(&[14470, 28201, 31565]),
                    multi_ctx_cdf(&[9493, 22982, 28608]),
                    multi_ctx_cdf(&[6858, 17240, 24137]),
                    multi_ctx_cdf(&[32543, 32752, 32756]),
                    multi_ctx_cdf(&[24286, 32097, 32666]),
                    multi_ctx_cdf(&[15958, 29217, 32024]),
                    multi_ctx_cdf(&[10207, 24234, 29958]),
                    multi_ctx_cdf(&[6929, 18305, 25652]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                ],
            ],
            _ => unreachable!(),
        };
        let coeff_br_cdf_chroma: [[Vec<u16>; 21]; 4] = match qcat.min(3) {
            0 => [
                [
                    multi_ctx_cdf(&[15967, 22905, 26286]),
                    multi_ctx_cdf(&[13534, 20654, 24579]),
                    multi_ctx_cdf(&[9504, 16092, 20535]),
                    multi_ctx_cdf(&[6975, 12568, 16903]),
                    multi_ctx_cdf(&[5364, 10091, 14020]),
                    multi_ctx_cdf(&[4357, 8370, 11857]),
                    multi_ctx_cdf(&[2506, 4934, 7218]),
                    multi_ctx_cdf(&[23032, 28815, 30936]),
                    multi_ctx_cdf(&[19540, 26704, 29719]),
                    multi_ctx_cdf(&[15158, 22969, 27097]),
                    multi_ctx_cdf(&[11408, 18865, 23650]),
                    multi_ctx_cdf(&[8885, 15448, 20250]),
                    multi_ctx_cdf(&[7108, 12853, 17416]),
                    multi_ctx_cdf(&[4231, 8041, 11480]),
                    multi_ctx_cdf(&[19823, 26490, 29156]),
                    multi_ctx_cdf(&[18890, 25929, 28932]),
                    multi_ctx_cdf(&[15660, 23491, 27433]),
                    multi_ctx_cdf(&[12147, 19776, 24488]),
                    multi_ctx_cdf(&[9728, 16774, 21649]),
                    multi_ctx_cdf(&[7919, 14277, 19066]),
                    multi_ctx_cdf(&[5440, 10170, 14185]),
                ],
                [
                    multi_ctx_cdf(&[15460, 21696, 25469]),
                    multi_ctx_cdf(&[12170, 19249, 23191]),
                    multi_ctx_cdf(&[8723, 15027, 19332]),
                    multi_ctx_cdf(&[6428, 11704, 15874]),
                    multi_ctx_cdf(&[4922, 9292, 13052]),
                    multi_ctx_cdf(&[4139, 7695, 11010]),
                    multi_ctx_cdf(&[2291, 4508, 6598]),
                    multi_ctx_cdf(&[19856, 26920, 29828]),
                    multi_ctx_cdf(&[17923, 25289, 28792]),
                    multi_ctx_cdf(&[14278, 21968, 26297]),
                    multi_ctx_cdf(&[10910, 18136, 22950]),
                    multi_ctx_cdf(&[8423, 14815, 19627]),
                    multi_ctx_cdf(&[6771, 12283, 16774]),
                    multi_ctx_cdf(&[4074, 7750, 11081]),
                    multi_ctx_cdf(&[19852, 26074, 28672]),
                    multi_ctx_cdf(&[19371, 26110, 28989]),
                    multi_ctx_cdf(&[16265, 23873, 27663]),
                    multi_ctx_cdf(&[12758, 20378, 24952]),
                    multi_ctx_cdf(&[10095, 17098, 21961]),
                    multi_ctx_cdf(&[8250, 14628, 19451]),
                    multi_ctx_cdf(&[5205, 9745, 13622]),
                ],
                [
                    multi_ctx_cdf(&[10870, 16684, 20949]),
                    multi_ctx_cdf(&[9664, 15230, 18680]),
                    multi_ctx_cdf(&[6886, 12109, 15408]),
                    multi_ctx_cdf(&[4825, 8900, 12305]),
                    multi_ctx_cdf(&[3630, 7162, 10314]),
                    multi_ctx_cdf(&[3036, 6429, 9387]),
                    multi_ctx_cdf(&[1671, 3296, 4940]),
                    multi_ctx_cdf(&[13819, 19159, 23026]),
                    multi_ctx_cdf(&[11984, 19108, 23120]),
                    multi_ctx_cdf(&[10690, 17210, 21663]),
                    multi_ctx_cdf(&[7984, 14154, 18333]),
                    multi_ctx_cdf(&[6868, 12294, 16124]),
                    multi_ctx_cdf(&[5274, 8994, 12868]),
                    multi_ctx_cdf(&[2988, 5771, 8424]),
                    multi_ctx_cdf(&[19736, 26647, 29141]),
                    multi_ctx_cdf(&[18933, 26070, 28984]),
                    multi_ctx_cdf(&[15779, 23048, 27200]),
                    multi_ctx_cdf(&[12638, 20061, 24532]),
                    multi_ctx_cdf(&[10692, 17545, 22220]),
                    multi_ctx_cdf(&[9217, 15251, 20054]),
                    multi_ctx_cdf(&[5078, 9284, 12594]),
                ],
                [
                    multi_ctx_cdf(&[5842, 9229, 10838]),
                    multi_ctx_cdf(&[2313, 3491, 4276]),
                    multi_ctx_cdf(&[2998, 6104, 7496]),
                    multi_ctx_cdf(&[2420, 7447, 9868]),
                    multi_ctx_cdf(&[3034, 8495, 10923]),
                    multi_ctx_cdf(&[4076, 8937, 10975]),
                    multi_ctx_cdf(&[1086, 2370, 3299]),
                    multi_ctx_cdf(&[9714, 17254, 20444]),
                    multi_ctx_cdf(&[8543, 13698, 17123]),
                    multi_ctx_cdf(&[4918, 9007, 11910]),
                    multi_ctx_cdf(&[4129, 7532, 10553]),
                    multi_ctx_cdf(&[2364, 5533, 8058]),
                    multi_ctx_cdf(&[1834, 3546, 5563]),
                    multi_ctx_cdf(&[1473, 2908, 4133]),
                    multi_ctx_cdf(&[15405, 21193, 25619]),
                    multi_ctx_cdf(&[15691, 21952, 26561]),
                    multi_ctx_cdf(&[12962, 19194, 24165]),
                    multi_ctx_cdf(&[10272, 17855, 22129]),
                    multi_ctx_cdf(&[8588, 15270, 20718]),
                    multi_ctx_cdf(&[8682, 14669, 19500]),
                    multi_ctx_cdf(&[4870, 9636, 13205]),
                ],
            ],
            1 => [
                [
                    multi_ctx_cdf(&[15571, 22232, 25749]),
                    multi_ctx_cdf(&[14506, 21575, 25374]),
                    multi_ctx_cdf(&[10189, 17089, 21569]),
                    multi_ctx_cdf(&[7316, 13301, 17915]),
                    multi_ctx_cdf(&[5783, 10912, 15190]),
                    multi_ctx_cdf(&[4760, 9155, 13088]),
                    multi_ctx_cdf(&[2993, 5966, 8774]),
                    multi_ctx_cdf(&[23424, 28903, 30778]),
                    multi_ctx_cdf(&[20775, 27666, 30290]),
                    multi_ctx_cdf(&[16474, 24410, 28299]),
                    multi_ctx_cdf(&[12471, 20180, 24987]),
                    multi_ctx_cdf(&[9410, 16487, 21439]),
                    multi_ctx_cdf(&[7536, 13614, 18529]),
                    multi_ctx_cdf(&[5048, 9586, 13549]),
                    multi_ctx_cdf(&[21090, 27290, 29756]),
                    multi_ctx_cdf(&[20796, 27402, 30026]),
                    multi_ctx_cdf(&[17819, 25485, 28969]),
                    multi_ctx_cdf(&[13860, 21909, 26462]),
                    multi_ctx_cdf(&[11002, 18494, 23529]),
                    multi_ctx_cdf(&[8953, 15929, 20897]),
                    multi_ctx_cdf(&[6448, 11918, 16454]),
                ],
                [
                    multi_ctx_cdf(&[14899, 21217, 24503]),
                    multi_ctx_cdf(&[13519, 20283, 24047]),
                    multi_ctx_cdf(&[9429, 15966, 20365]),
                    multi_ctx_cdf(&[6700, 12355, 16652]),
                    multi_ctx_cdf(&[5088, 9704, 13716]),
                    multi_ctx_cdf(&[4243, 8154, 11731]),
                    multi_ctx_cdf(&[2702, 5364, 7861]),
                    multi_ctx_cdf(&[22745, 28388, 30454]),
                    multi_ctx_cdf(&[20235, 27146, 29922]),
                    multi_ctx_cdf(&[15896, 23715, 27637]),
                    multi_ctx_cdf(&[11840, 19350, 24131]),
                    multi_ctx_cdf(&[9122, 15932, 20880]),
                    multi_ctx_cdf(&[7488, 13581, 18362]),
                    multi_ctx_cdf(&[5114, 9568, 13370]),
                    multi_ctx_cdf(&[20845, 26553, 28932]),
                    multi_ctx_cdf(&[20981, 27372, 29884]),
                    multi_ctx_cdf(&[17781, 25335, 28785]),
                    multi_ctx_cdf(&[13760, 21708, 26297]),
                    multi_ctx_cdf(&[10975, 18415, 23365]),
                    multi_ctx_cdf(&[9045, 15789, 20686]),
                    multi_ctx_cdf(&[6130, 11199, 15423]),
                ],
                [
                    multi_ctx_cdf(&[13960, 19617, 22829]),
                    multi_ctx_cdf(&[11150, 17341, 21228]),
                    multi_ctx_cdf(&[7150, 12964, 17190]),
                    multi_ctx_cdf(&[5331, 10002, 13867]),
                    multi_ctx_cdf(&[4167, 7744, 11057]),
                    multi_ctx_cdf(&[3480, 6629, 9646]),
                    multi_ctx_cdf(&[1883, 3784, 5686]),
                    multi_ctx_cdf(&[18752, 25660, 28912]),
                    multi_ctx_cdf(&[16968, 24586, 28030]),
                    multi_ctx_cdf(&[13520, 21055, 25313]),
                    multi_ctx_cdf(&[10453, 17626, 22280]),
                    multi_ctx_cdf(&[8386, 14505, 19116]),
                    multi_ctx_cdf(&[6742, 12595, 17008]),
                    multi_ctx_cdf(&[4273, 8140, 11499]),
                    multi_ctx_cdf(&[22120, 27827, 30233]),
                    multi_ctx_cdf(&[20563, 27358, 29895]),
                    multi_ctx_cdf(&[17076, 24644, 28153]),
                    multi_ctx_cdf(&[13362, 20942, 25309]),
                    multi_ctx_cdf(&[10794, 17965, 22695]),
                    multi_ctx_cdf(&[9014, 15652, 20319]),
                    multi_ctx_cdf(&[5708, 10512, 14497]),
                ],
                [
                    multi_ctx_cdf(&[8278, 13242, 15922]),
                    multi_ctx_cdf(&[10547, 15867, 18919]),
                    multi_ctx_cdf(&[9106, 15842, 20609]),
                    multi_ctx_cdf(&[6833, 13007, 17218]),
                    multi_ctx_cdf(&[4811, 9712, 13923]),
                    multi_ctx_cdf(&[3985, 7352, 11128]),
                    multi_ctx_cdf(&[1688, 3458, 5262]),
                    multi_ctx_cdf(&[12951, 21861, 26510]),
                    multi_ctx_cdf(&[9788, 16044, 20276]),
                    multi_ctx_cdf(&[6309, 11244, 14870]),
                    multi_ctx_cdf(&[5183, 9349, 12566]),
                    multi_ctx_cdf(&[4389, 8229, 11492]),
                    multi_ctx_cdf(&[3633, 6945, 10620]),
                    multi_ctx_cdf(&[3600, 6847, 9907]),
                    multi_ctx_cdf(&[21748, 28137, 30255]),
                    multi_ctx_cdf(&[19436, 26581, 29560]),
                    multi_ctx_cdf(&[16359, 24201, 27953]),
                    multi_ctx_cdf(&[13961, 21693, 25871]),
                    multi_ctx_cdf(&[11544, 18686, 23322]),
                    multi_ctx_cdf(&[9372, 16462, 20952]),
                    multi_ctx_cdf(&[6138, 11210, 15390]),
                ],
            ],
            2 => [
                [
                    multi_ctx_cdf(&[17394, 24501, 27895]),
                    multi_ctx_cdf(&[15889, 23420, 27185]),
                    multi_ctx_cdf(&[11561, 19133, 23870]),
                    multi_ctx_cdf(&[8285, 14812, 19844]),
                    multi_ctx_cdf(&[6496, 12043, 16550]),
                    multi_ctx_cdf(&[4771, 9574, 13677]),
                    multi_ctx_cdf(&[3603, 6830, 10144]),
                    multi_ctx_cdf(&[21656, 27704, 30200]),
                    multi_ctx_cdf(&[21324, 27915, 30511]),
                    multi_ctx_cdf(&[17327, 25336, 28997]),
                    multi_ctx_cdf(&[13417, 21381, 26033]),
                    multi_ctx_cdf(&[10132, 17425, 22338]),
                    multi_ctx_cdf(&[8580, 15016, 19633]),
                    multi_ctx_cdf(&[5694, 11477, 16411]),
                    multi_ctx_cdf(&[24116, 29780, 31450]),
                    multi_ctx_cdf(&[23853, 29695, 31591]),
                    multi_ctx_cdf(&[20085, 27614, 30428]),
                    multi_ctx_cdf(&[15326, 24335, 28575]),
                    multi_ctx_cdf(&[11814, 19472, 24810]),
                    multi_ctx_cdf(&[10221, 18611, 24767]),
                    multi_ctx_cdf(&[7689, 14558, 20321]),
                ],
                [
                    multi_ctx_cdf(&[14433, 21155, 24938]),
                    multi_ctx_cdf(&[14658, 21716, 25545]),
                    multi_ctx_cdf(&[9923, 16824, 21557]),
                    multi_ctx_cdf(&[6982, 13052, 17721]),
                    multi_ctx_cdf(&[5419, 10503, 15050]),
                    multi_ctx_cdf(&[4852, 9162, 13014]),
                    multi_ctx_cdf(&[3271, 6395, 9630]),
                    multi_ctx_cdf(&[22210, 27833, 30109]),
                    multi_ctx_cdf(&[20750, 27368, 29821]),
                    multi_ctx_cdf(&[16894, 24828, 28573]),
                    multi_ctx_cdf(&[13247, 21276, 25757]),
                    multi_ctx_cdf(&[10038, 17265, 22563]),
                    multi_ctx_cdf(&[8587, 14947, 20327]),
                    multi_ctx_cdf(&[5645, 11371, 15252]),
                    multi_ctx_cdf(&[22027, 27526, 29714]),
                    multi_ctx_cdf(&[23098, 29146, 31221]),
                    multi_ctx_cdf(&[19886, 27341, 30272]),
                    multi_ctx_cdf(&[15609, 23747, 28046]),
                    multi_ctx_cdf(&[11993, 20065, 24939]),
                    multi_ctx_cdf(&[9637, 18267, 23671]),
                    multi_ctx_cdf(&[7625, 13801, 19144]),
                ],
                [
                    multi_ctx_cdf(&[13569, 19800, 23206]),
                    multi_ctx_cdf(&[13128, 19924, 23869]),
                    multi_ctx_cdf(&[8329, 14841, 19403]),
                    multi_ctx_cdf(&[6130, 10976, 15057]),
                    multi_ctx_cdf(&[4682, 8839, 12518]),
                    multi_ctx_cdf(&[3656, 7409, 10588]),
                    multi_ctx_cdf(&[2577, 5099, 7412]),
                    multi_ctx_cdf(&[22427, 28684, 30585]),
                    multi_ctx_cdf(&[20913, 27750, 30139]),
                    multi_ctx_cdf(&[15840, 24109, 27834]),
                    multi_ctx_cdf(&[12308, 20029, 24569]),
                    multi_ctx_cdf(&[10216, 16785, 21458]),
                    multi_ctx_cdf(&[8309, 14203, 19113]),
                    multi_ctx_cdf(&[6043, 11168, 15307]),
                    multi_ctx_cdf(&[23166, 28901, 30998]),
                    multi_ctx_cdf(&[21899, 28405, 30751]),
                    multi_ctx_cdf(&[18413, 26091, 29443]),
                    multi_ctx_cdf(&[15233, 23114, 27352]),
                    multi_ctx_cdf(&[12683, 20472, 25288]),
                    multi_ctx_cdf(&[10702, 18259, 23409]),
                    multi_ctx_cdf(&[8125, 14464, 19226]),
                ],
                [
                    multi_ctx_cdf(&[12339, 17329, 20140]),
                    multi_ctx_cdf(&[13505, 19895, 23225]),
                    multi_ctx_cdf(&[9847, 16944, 21564]),
                    multi_ctx_cdf(&[7280, 13256, 18348]),
                    multi_ctx_cdf(&[4712, 10009, 14454]),
                    multi_ctx_cdf(&[4361, 7914, 12477]),
                    multi_ctx_cdf(&[2870, 5628, 7995]),
                    multi_ctx_cdf(&[20061, 25504, 28526]),
                    multi_ctx_cdf(&[15235, 22878, 26145]),
                    multi_ctx_cdf(&[12985, 19958, 24155]),
                    multi_ctx_cdf(&[9782, 16641, 21403]),
                    multi_ctx_cdf(&[9456, 16360, 20760]),
                    multi_ctx_cdf(&[6855, 12940, 18557]),
                    multi_ctx_cdf(&[5661, 10564, 15002]),
                    multi_ctx_cdf(&[25656, 30602, 31894]),
                    multi_ctx_cdf(&[22570, 29107, 31092]),
                    multi_ctx_cdf(&[18917, 26423, 29541]),
                    multi_ctx_cdf(&[15940, 23649, 27754]),
                    multi_ctx_cdf(&[12803, 20581, 25219]),
                    multi_ctx_cdf(&[11082, 18695, 23376]),
                    multi_ctx_cdf(&[7939, 14373, 19005]),
                ],
            ],
            3 => [
                [
                    multi_ctx_cdf(&[21425, 27952, 30388]),
                    multi_ctx_cdf(&[18062, 25838, 29034]),
                    multi_ctx_cdf(&[11956, 19881, 24808]),
                    multi_ctx_cdf(&[7718, 15000, 20980]),
                    multi_ctx_cdf(&[5702, 11254, 16143]),
                    multi_ctx_cdf(&[4898, 9088, 16864]),
                    multi_ctx_cdf(&[3679, 6776, 11907]),
                    multi_ctx_cdf(&[23294, 30160, 31663]),
                    multi_ctx_cdf(&[24397, 29896, 31836]),
                    multi_ctx_cdf(&[19245, 27128, 30593]),
                    multi_ctx_cdf(&[13202, 19825, 26404]),
                    multi_ctx_cdf(&[11578, 19297, 23957]),
                    multi_ctx_cdf(&[8073, 13297, 21370]),
                    multi_ctx_cdf(&[5461, 10923, 19745]),
                    multi_ctx_cdf(&[27367, 30521, 31934]),
                    multi_ctx_cdf(&[24904, 30671, 31940]),
                    multi_ctx_cdf(&[23075, 28460, 31299]),
                    multi_ctx_cdf(&[14400, 23658, 30417]),
                    multi_ctx_cdf(&[13885, 23882, 28325]),
                    multi_ctx_cdf(&[14746, 22938, 27853]),
                    multi_ctx_cdf(&[5461, 16384, 27307]),
                ],
                [
                    multi_ctx_cdf(&[17616, 24586, 28112]),
                    multi_ctx_cdf(&[15809, 23299, 27155]),
                    multi_ctx_cdf(&[10767, 18890, 23793]),
                    multi_ctx_cdf(&[7727, 14255, 18865]),
                    multi_ctx_cdf(&[6129, 11926, 16882]),
                    multi_ctx_cdf(&[4482, 9704, 14861]),
                    multi_ctx_cdf(&[3277, 7452, 11522]),
                    multi_ctx_cdf(&[22956, 28551, 30730]),
                    multi_ctx_cdf(&[22724, 28937, 30961]),
                    multi_ctx_cdf(&[18467, 26324, 29580]),
                    multi_ctx_cdf(&[13234, 20713, 25649]),
                    multi_ctx_cdf(&[11181, 17592, 22481]),
                    multi_ctx_cdf(&[8291, 18358, 24576]),
                    multi_ctx_cdf(&[7568, 11881, 14984]),
                    multi_ctx_cdf(&[24948, 29001, 31147]),
                    multi_ctx_cdf(&[25674, 30619, 32151]),
                    multi_ctx_cdf(&[20841, 26793, 29603]),
                    multi_ctx_cdf(&[14669, 24356, 28666]),
                    multi_ctx_cdf(&[11334, 23593, 28219]),
                    multi_ctx_cdf(&[8922, 14762, 22873]),
                    multi_ctx_cdf(&[8301, 13544, 20535]),
                ],
                [
                    multi_ctx_cdf(&[15820, 22738, 26488]),
                    multi_ctx_cdf(&[13530, 20885, 25216]),
                    multi_ctx_cdf(&[8395, 15530, 20452]),
                    multi_ctx_cdf(&[6574, 12321, 16380]),
                    multi_ctx_cdf(&[5353, 10419, 14568]),
                    multi_ctx_cdf(&[4613, 8446, 12381]),
                    multi_ctx_cdf(&[3440, 7158, 9903]),
                    multi_ctx_cdf(&[24247, 29051, 31224]),
                    multi_ctx_cdf(&[22118, 28058, 30369]),
                    multi_ctx_cdf(&[16498, 24768, 28389]),
                    multi_ctx_cdf(&[12920, 21175, 26137]),
                    multi_ctx_cdf(&[10730, 18619, 25352]),
                    multi_ctx_cdf(&[10187, 16279, 22791]),
                    multi_ctx_cdf(&[9310, 14631, 22127]),
                    multi_ctx_cdf(&[24970, 30558, 32057]),
                    multi_ctx_cdf(&[24801, 29942, 31698]),
                    multi_ctx_cdf(&[22432, 28453, 30855]),
                    multi_ctx_cdf(&[19054, 25680, 29580]),
                    multi_ctx_cdf(&[14392, 23036, 28109]),
                    multi_ctx_cdf(&[12495, 20947, 26650]),
                    multi_ctx_cdf(&[12442, 20326, 26214]),
                ],
                [
                    multi_ctx_cdf(&[13981, 20067, 23226]),
                    multi_ctx_cdf(&[16922, 23580, 26783]),
                    multi_ctx_cdf(&[11005, 19039, 24487]),
                    multi_ctx_cdf(&[7389, 14218, 19798]),
                    multi_ctx_cdf(&[5598, 11505, 17206]),
                    multi_ctx_cdf(&[6090, 11213, 15659]),
                    multi_ctx_cdf(&[3820, 7371, 10119]),
                    multi_ctx_cdf(&[21082, 26925, 29675]),
                    multi_ctx_cdf(&[21262, 28627, 31128]),
                    multi_ctx_cdf(&[18392, 26454, 30437]),
                    multi_ctx_cdf(&[14870, 22910, 27096]),
                    multi_ctx_cdf(&[12620, 19484, 24908]),
                    multi_ctx_cdf(&[9290, 16553, 22802]),
                    multi_ctx_cdf(&[6668, 14288, 20004]),
                    multi_ctx_cdf(&[27704, 31055, 31949]),
                    multi_ctx_cdf(&[24709, 29978, 31788]),
                    multi_ctx_cdf(&[21668, 29264, 31657]),
                    multi_ctx_cdf(&[18295, 26968, 30074]),
                    multi_ctx_cdf(&[16399, 24422, 29313]),
                    multi_ctx_cdf(&[14347, 23026, 28104]),
                    multi_ctx_cdf(&[12370, 19806, 24477]),
                ],
            ],
            _ => unreachable!(),
        };

        // transform_type(): real spec/rav1d default CDFs, indexed [tx_size_class][...]. Source:
        // rav1d `txtp_intra1`/`txtp_intra2`/`txtp_inter1`/`txtp_inter2`/`txtp_inter3`, first
        // qindex-bucket variant.
        let txtp_intra1_cdf: [[Vec<u16>; 13]; 2] = [
            [
                multi_ctx_cdf(&[1535, 8035, 9461, 12751, 23467, 27825]),
                multi_ctx_cdf(&[564, 3335, 9709, 10870, 18143, 28094]),
                multi_ctx_cdf(&[672, 3247, 3676, 11982, 19415, 23127]),
                multi_ctx_cdf(&[5279, 13885, 15487, 18044, 23527, 30252]),
                multi_ctx_cdf(&[4423, 6074, 7985, 10416, 25693, 29298]),
                multi_ctx_cdf(&[1486, 4241, 9460, 10662, 16456, 27694]),
                multi_ctx_cdf(&[439, 2838, 3522, 6737, 18058, 23754]),
                multi_ctx_cdf(&[1190, 4233, 4855, 11670, 20281, 24377]),
                multi_ctx_cdf(&[1045, 4312, 8647, 10159, 18644, 29335]),
                multi_ctx_cdf(&[202, 3734, 4747, 7298, 17127, 24016]),
                multi_ctx_cdf(&[447, 4312, 6819, 8884, 16010, 23858]),
                multi_ctx_cdf(&[277, 4369, 5255, 8905, 16465, 22271]),
                multi_ctx_cdf(&[3409, 5436, 10599, 15599, 19687, 24040]),
            ],
            [
                multi_ctx_cdf(&[1870, 13742, 14530, 16498, 23770, 27698]),
                multi_ctx_cdf(&[326, 8796, 14632, 15079, 19272, 27486]),
                multi_ctx_cdf(&[484, 7576, 7712, 14443, 19159, 22591]),
                multi_ctx_cdf(&[1126, 15340, 15895, 17023, 20896, 30279]),
                multi_ctx_cdf(&[655, 4854, 5249, 5913, 22099, 27138]),
                multi_ctx_cdf(&[1299, 6458, 8885, 9290, 14851, 25497]),
                multi_ctx_cdf(&[311, 5295, 5552, 6885, 16107, 22672]),
                multi_ctx_cdf(&[883, 8059, 8270, 11258, 17289, 21549]),
                multi_ctx_cdf(&[741, 7580, 9318, 10345, 16688, 29046]),
                multi_ctx_cdf(&[110, 7406, 7915, 9195, 16041, 23329]),
                multi_ctx_cdf(&[363, 7974, 9357, 10673, 15629, 24474]),
                multi_ctx_cdf(&[153, 7647, 8112, 9936, 15307, 19996]),
                multi_ctx_cdf(&[3511, 6332, 11165, 15335, 19323, 23594]),
            ],
        ];
        let txtp_intra2_uniform =
            || std::array::from_fn::<_, 13, _>(|_| multi_ctx_cdf(&[6554, 13107, 19661, 26214]));
        let txtp_intra2_cdf: [[Vec<u16>; 13]; 3] = [
            txtp_intra2_uniform(),
            txtp_intra2_uniform(),
            [
                multi_ctx_cdf(&[1127, 12814, 22772, 27483]),
                multi_ctx_cdf(&[145, 6761, 11980, 26667]),
                multi_ctx_cdf(&[362, 5887, 11678, 16725]),
                multi_ctx_cdf(&[385, 15213, 18587, 30693]),
                multi_ctx_cdf(&[25, 2914, 23134, 27903]),
                multi_ctx_cdf(&[60, 4470, 11749, 23991]),
                multi_ctx_cdf(&[37, 3332, 14511, 21448]),
                multi_ctx_cdf(&[157, 6320, 13036, 17439]),
                multi_ctx_cdf(&[119, 6719, 12906, 29396]),
                multi_ctx_cdf(&[47, 5537, 12576, 21499]),
                multi_ctx_cdf(&[269, 6076, 11258, 23115]),
                multi_ctx_cdf(&[83, 5615, 12001, 17228]),
                multi_ctx_cdf(&[1968, 5556, 12023, 18547]),
            ],
        ];
        let txtp_inter1_cdf = [
            multi_ctx_cdf(&[
                4458, 5560, 7695, 9709, 13330, 14789, 17537, 20266, 21504, 22848, 23934, 25474,
                27727, 28915, 30631,
            ]),
            multi_ctx_cdf(&[
                1645, 2573, 4778, 5711, 7807, 8622, 10522, 15357, 17674, 20408, 22517, 25010,
                27116, 28856, 30749,
            ]),
        ];
        let txtp_inter2_cdf = multi_ctx_cdf(&[
            770, 2421, 5225, 12907, 15819, 18927, 21561, 24089, 26595, 28526, 30529,
        ]);
        let txtp_inter3_cdf = [16384, 4167, 1998, 748].map(binary_ctx_cdf);

        // tx_size(): real spec/rav1d default CDFs, indexed [max_tx_class-1][ctx]. Source: rav1d
        // `m.txsz`.
        let txsz_cdf: [[Vec<u16>; 3]; 4] = [
            [
                multi_ctx_cdf(&[19968]),
                multi_ctx_cdf(&[19968]),
                multi_ctx_cdf(&[24320]),
            ],
            [
                multi_ctx_cdf(&[12272, 30172]),
                multi_ctx_cdf(&[12272, 30172]),
                multi_ctx_cdf(&[18677, 30848]),
            ],
            [
                multi_ctx_cdf(&[12986, 15180]),
                multi_ctx_cdf(&[12986, 15180]),
                multi_ctx_cdf(&[24302, 25602]),
            ],
            [
                multi_ctx_cdf(&[5782, 11475]),
                multi_ctx_cdf(&[5782, 11475]),
                multi_ctx_cdf(&[16803, 22759]),
            ],
        ];

        // coeff_base: real spec/rav1d default CDFs, indexed [tx_size_class 0..=4][ctx 0..=40]
        // (chroma=0 fixed -- see `coeff_base_cdf`'s doc). Source: rav1d `coef.base_tok`, real
        // per-frame qindex-bucket selection.
        let coeff_base_cdf: [[Vec<u16>; 41]; 5] = match qcat.min(3) {
            0 => [
                [
                    multi_ctx_cdf(&[4034, 8930, 12727]),
                    multi_ctx_cdf(&[18082, 29741, 31877]),
                    multi_ctx_cdf(&[12596, 26124, 30493]),
                    multi_ctx_cdf(&[9446, 21118, 27005]),
                    multi_ctx_cdf(&[6308, 15141, 21279]),
                    multi_ctx_cdf(&[2463, 6357, 9783]),
                    multi_ctx_cdf(&[20667, 30546, 31929]),
                    multi_ctx_cdf(&[13043, 26123, 30134]),
                    multi_ctx_cdf(&[8151, 18757, 24778]),
                    multi_ctx_cdf(&[5255, 12839, 18632]),
                    multi_ctx_cdf(&[2820, 7206, 11161]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[15736, 27553, 30604]),
                    multi_ctx_cdf(&[11210, 23794, 28787]),
                    multi_ctx_cdf(&[5947, 13874, 19701]),
                    multi_ctx_cdf(&[4215, 9323, 13891]),
                    multi_ctx_cdf(&[2833, 6462, 10059]),
                    multi_ctx_cdf(&[19605, 30393, 31582]),
                    multi_ctx_cdf(&[13523, 26252, 30248]),
                    multi_ctx_cdf(&[8446, 18622, 24512]),
                    multi_ctx_cdf(&[3818, 10343, 15974]),
                    multi_ctx_cdf(&[1481, 4117, 6796]),
                    multi_ctx_cdf(&[22649, 31302, 32190]),
                    multi_ctx_cdf(&[14829, 27127, 30449]),
                    multi_ctx_cdf(&[8313, 17702, 23304]),
                    multi_ctx_cdf(&[3022, 8301, 12786]),
                    multi_ctx_cdf(&[1536, 4412, 7184]),
                    multi_ctx_cdf(&[22354, 29774, 31372]),
                    multi_ctx_cdf(&[14723, 25472, 29214]),
                    multi_ctx_cdf(&[6673, 13745, 18662]),
                    multi_ctx_cdf(&[2068, 5766, 9322]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                ],
                [
                    multi_ctx_cdf(&[4536, 10072, 14001]),
                    multi_ctx_cdf(&[25459, 31416, 32206]),
                    multi_ctx_cdf(&[16605, 28048, 30818]),
                    multi_ctx_cdf(&[11008, 22857, 27719]),
                    multi_ctx_cdf(&[6915, 16268, 22315]),
                    multi_ctx_cdf(&[2625, 6812, 10537]),
                    multi_ctx_cdf(&[24257, 31788, 32499]),
                    multi_ctx_cdf(&[16880, 29454, 31879]),
                    multi_ctx_cdf(&[11958, 25054, 29778]),
                    multi_ctx_cdf(&[7916, 18718, 25084]),
                    multi_ctx_cdf(&[3383, 8777, 13446]),
                    multi_ctx_cdf(&[22720, 31603, 32393]),
                    multi_ctx_cdf(&[14960, 28125, 31335]),
                    multi_ctx_cdf(&[9731, 22210, 27928]),
                    multi_ctx_cdf(&[6304, 15832, 22277]),
                    multi_ctx_cdf(&[2910, 7818, 12166]),
                    multi_ctx_cdf(&[20375, 30627, 32131]),
                    multi_ctx_cdf(&[13904, 27284, 30887]),
                    multi_ctx_cdf(&[9368, 21558, 27144]),
                    multi_ctx_cdf(&[5937, 14966, 21119]),
                    multi_ctx_cdf(&[2667, 7225, 11319]),
                    multi_ctx_cdf(&[23970, 31470, 32378]),
                    multi_ctx_cdf(&[17173, 29734, 32018]),
                    multi_ctx_cdf(&[12795, 25441, 29965]),
                    multi_ctx_cdf(&[8981, 19680, 25893]),
                    multi_ctx_cdf(&[4728, 11372, 16902]),
                    multi_ctx_cdf(&[24287, 31797, 32439]),
                    multi_ctx_cdf(&[16703, 29145, 31696]),
                    multi_ctx_cdf(&[10833, 23554, 28725]),
                    multi_ctx_cdf(&[6468, 16566, 23057]),
                    multi_ctx_cdf(&[2415, 6562, 10278]),
                    multi_ctx_cdf(&[26610, 32395, 32659]),
                    multi_ctx_cdf(&[18590, 30498, 32117]),
                    multi_ctx_cdf(&[12420, 25756, 29950]),
                    multi_ctx_cdf(&[7639, 18746, 24710]),
                    multi_ctx_cdf(&[3001, 8086, 12347]),
                    multi_ctx_cdf(&[25076, 32064, 32580]),
                    multi_ctx_cdf(&[17946, 30128, 32028]),
                    multi_ctx_cdf(&[12024, 24985, 29378]),
                    multi_ctx_cdf(&[7517, 18390, 24304]),
                    multi_ctx_cdf(&[3243, 8781, 13331]),
                ],
                [
                    multi_ctx_cdf(&[5487, 10460, 13708]),
                    multi_ctx_cdf(&[21597, 28303, 30674]),
                    multi_ctx_cdf(&[11037, 21953, 26476]),
                    multi_ctx_cdf(&[8147, 17962, 22952]),
                    multi_ctx_cdf(&[5242, 13061, 18532]),
                    multi_ctx_cdf(&[1889, 5208, 8182]),
                    multi_ctx_cdf(&[26774, 32133, 32590]),
                    multi_ctx_cdf(&[17844, 29564, 31767]),
                    multi_ctx_cdf(&[11690, 24438, 29171]),
                    multi_ctx_cdf(&[7542, 18215, 24459]),
                    multi_ctx_cdf(&[2993, 8050, 12319]),
                    multi_ctx_cdf(&[28023, 32328, 32591]),
                    multi_ctx_cdf(&[18651, 30126, 31954]),
                    multi_ctx_cdf(&[12164, 25146, 29589]),
                    multi_ctx_cdf(&[7762, 18530, 24771]),
                    multi_ctx_cdf(&[3492, 9183, 13920]),
                    multi_ctx_cdf(&[27591, 32008, 32491]),
                    multi_ctx_cdf(&[17149, 28853, 31510]),
                    multi_ctx_cdf(&[11485, 24003, 28860]),
                    multi_ctx_cdf(&[7697, 18086, 24210]),
                    multi_ctx_cdf(&[3075, 7999, 12218]),
                    multi_ctx_cdf(&[28268, 32482, 32654]),
                    multi_ctx_cdf(&[19631, 31051, 32404]),
                    multi_ctx_cdf(&[13860, 27260, 31020]),
                    multi_ctx_cdf(&[9605, 21613, 27594]),
                    multi_ctx_cdf(&[4876, 12162, 17908]),
                    multi_ctx_cdf(&[27248, 32316, 32576]),
                    multi_ctx_cdf(&[18955, 30457, 32075]),
                    multi_ctx_cdf(&[11824, 23997, 28795]),
                    multi_ctx_cdf(&[7346, 18196, 24647]),
                    multi_ctx_cdf(&[3403, 9247, 14111]),
                    multi_ctx_cdf(&[29711, 32655, 32735]),
                    multi_ctx_cdf(&[21169, 31394, 32417]),
                    multi_ctx_cdf(&[13487, 27198, 30957]),
                    multi_ctx_cdf(&[8828, 21683, 27614]),
                    multi_ctx_cdf(&[4270, 11451, 17038]),
                    multi_ctx_cdf(&[28708, 32578, 32731]),
                    multi_ctx_cdf(&[20120, 31241, 32482]),
                    multi_ctx_cdf(&[13692, 27550, 31321]),
                    multi_ctx_cdf(&[9418, 22514, 28439]),
                    multi_ctx_cdf(&[4999, 13283, 19462]),
                ],
                [
                    multi_ctx_cdf(&[5141, 7096, 8260]),
                    multi_ctx_cdf(&[27186, 29022, 29789]),
                    multi_ctx_cdf(&[6668, 12568, 15682]),
                    multi_ctx_cdf(&[2172, 6181, 8638]),
                    multi_ctx_cdf(&[1126, 3379, 4531]),
                    multi_ctx_cdf(&[443, 1361, 2254]),
                    multi_ctx_cdf(&[26083, 31153, 32436]),
                    multi_ctx_cdf(&[13486, 24603, 28483]),
                    multi_ctx_cdf(&[6508, 14840, 19910]),
                    multi_ctx_cdf(&[3386, 8800, 13286]),
                    multi_ctx_cdf(&[1530, 4322, 7054]),
                    multi_ctx_cdf(&[29639, 32080, 32548]),
                    multi_ctx_cdf(&[15897, 27552, 30290]),
                    multi_ctx_cdf(&[8588, 20047, 25383]),
                    multi_ctx_cdf(&[4889, 13339, 19269]),
                    multi_ctx_cdf(&[2240, 6871, 10498]),
                    multi_ctx_cdf(&[28165, 32197, 32517]),
                    multi_ctx_cdf(&[20735, 30427, 31568]),
                    multi_ctx_cdf(&[14325, 24671, 27692]),
                    multi_ctx_cdf(&[5119, 12554, 17805]),
                    multi_ctx_cdf(&[1810, 5441, 8261]),
                    multi_ctx_cdf(&[31212, 32724, 32748]),
                    multi_ctx_cdf(&[23352, 31766, 32545]),
                    multi_ctx_cdf(&[14669, 27570, 31059]),
                    multi_ctx_cdf(&[8492, 20894, 27272]),
                    multi_ctx_cdf(&[3644, 10194, 15204]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                ],
                [
                    multi_ctx_cdf(&[601, 983, 1311]),
                    multi_ctx_cdf(&[18725, 23406, 28087]),
                    multi_ctx_cdf(&[5461, 8192, 10923]),
                    multi_ctx_cdf(&[3781, 15124, 21425]),
                    multi_ctx_cdf(&[2587, 7761, 12072]),
                    multi_ctx_cdf(&[106, 458, 810]),
                    multi_ctx_cdf(&[22282, 29710, 31894]),
                    multi_ctx_cdf(&[8508, 20926, 25984]),
                    multi_ctx_cdf(&[3726, 12713, 18083]),
                    multi_ctx_cdf(&[1620, 7112, 10893]),
                    multi_ctx_cdf(&[729, 2236, 3495]),
                    multi_ctx_cdf(&[30163, 32474, 32684]),
                    multi_ctx_cdf(&[18304, 30464, 32000]),
                    multi_ctx_cdf(&[11443, 26526, 29647]),
                    multi_ctx_cdf(&[6007, 15292, 21299]),
                    multi_ctx_cdf(&[2234, 6703, 8937]),
                    multi_ctx_cdf(&[30954, 32177, 32571]),
                    multi_ctx_cdf(&[17363, 29562, 31076]),
                    multi_ctx_cdf(&[9686, 22464, 27410]),
                    multi_ctx_cdf(&[8192, 16384, 21390]),
                    multi_ctx_cdf(&[1755, 8046, 11264]),
                    multi_ctx_cdf(&[31168, 32734, 32748]),
                    multi_ctx_cdf(&[22486, 31441, 32471]),
                    multi_ctx_cdf(&[12833, 25627, 29738]),
                    multi_ctx_cdf(&[6980, 17379, 23122]),
                    multi_ctx_cdf(&[3111, 8887, 13479]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                ],
            ],
            1 => [
                [
                    multi_ctx_cdf(&[6041, 11854, 15927]),
                    multi_ctx_cdf(&[20326, 30905, 32251]),
                    multi_ctx_cdf(&[14164, 26831, 30725]),
                    multi_ctx_cdf(&[9760, 20647, 26585]),
                    multi_ctx_cdf(&[6416, 14953, 21219]),
                    multi_ctx_cdf(&[2966, 7151, 10891]),
                    multi_ctx_cdf(&[23567, 31374, 32254]),
                    multi_ctx_cdf(&[14978, 27416, 30946]),
                    multi_ctx_cdf(&[9434, 20225, 26254]),
                    multi_ctx_cdf(&[6658, 14558, 20535]),
                    multi_ctx_cdf(&[3916, 8677, 12989]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[18088, 29545, 31587]),
                    multi_ctx_cdf(&[13062, 25843, 30073]),
                    multi_ctx_cdf(&[8940, 16827, 22251]),
                    multi_ctx_cdf(&[7654, 13220, 17973]),
                    multi_ctx_cdf(&[5733, 10316, 14456]),
                    multi_ctx_cdf(&[22879, 31388, 32114]),
                    multi_ctx_cdf(&[15215, 27993, 30955]),
                    multi_ctx_cdf(&[9397, 19445, 24978]),
                    multi_ctx_cdf(&[3442, 9813, 15344]),
                    multi_ctx_cdf(&[1368, 3936, 6532]),
                    multi_ctx_cdf(&[25494, 32033, 32406]),
                    multi_ctx_cdf(&[16772, 27963, 30718]),
                    multi_ctx_cdf(&[9419, 18165, 23260]),
                    multi_ctx_cdf(&[2677, 7501, 11797]),
                    multi_ctx_cdf(&[1516, 4344, 7170]),
                    multi_ctx_cdf(&[26556, 31454, 32101]),
                    multi_ctx_cdf(&[17128, 27035, 30108]),
                    multi_ctx_cdf(&[8324, 15344, 20249]),
                    multi_ctx_cdf(&[1903, 5696, 9469]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                ],
                [
                    multi_ctx_cdf(&[6779, 13743, 17678]),
                    multi_ctx_cdf(&[24806, 31797, 32457]),
                    multi_ctx_cdf(&[17616, 29047, 31372]),
                    multi_ctx_cdf(&[11063, 23175, 28003]),
                    multi_ctx_cdf(&[6521, 16110, 22324]),
                    multi_ctx_cdf(&[2764, 7504, 11654]),
                    multi_ctx_cdf(&[25266, 32367, 32637]),
                    multi_ctx_cdf(&[19054, 30553, 32175]),
                    multi_ctx_cdf(&[12139, 25212, 29807]),
                    multi_ctx_cdf(&[7311, 18162, 24704]),
                    multi_ctx_cdf(&[3397, 9164, 14074]),
                    multi_ctx_cdf(&[25988, 32208, 32522]),
                    multi_ctx_cdf(&[16253, 28912, 31526]),
                    multi_ctx_cdf(&[9151, 21387, 27372]),
                    multi_ctx_cdf(&[5688, 14915, 21496]),
                    multi_ctx_cdf(&[2717, 7627, 12004]),
                    multi_ctx_cdf(&[23144, 31855, 32443]),
                    multi_ctx_cdf(&[16070, 28491, 31325]),
                    multi_ctx_cdf(&[8702, 20467, 26517]),
                    multi_ctx_cdf(&[5243, 13956, 20367]),
                    multi_ctx_cdf(&[2621, 7335, 11567]),
                    multi_ctx_cdf(&[26636, 32340, 32630]),
                    multi_ctx_cdf(&[19990, 31050, 32341]),
                    multi_ctx_cdf(&[13243, 26105, 30315]),
                    multi_ctx_cdf(&[8588, 19521, 25918]),
                    multi_ctx_cdf(&[4717, 11585, 17304]),
                    multi_ctx_cdf(&[25844, 32292, 32582]),
                    multi_ctx_cdf(&[19090, 30635, 32097]),
                    multi_ctx_cdf(&[11963, 24546, 28939]),
                    multi_ctx_cdf(&[6218, 16087, 22354]),
                    multi_ctx_cdf(&[2340, 6608, 10426]),
                    multi_ctx_cdf(&[28046, 32576, 32694]),
                    multi_ctx_cdf(&[21178, 31313, 32296]),
                    multi_ctx_cdf(&[13486, 26184, 29870]),
                    multi_ctx_cdf(&[7149, 17871, 23723]),
                    multi_ctx_cdf(&[2833, 7958, 12259]),
                    multi_ctx_cdf(&[27710, 32528, 32686]),
                    multi_ctx_cdf(&[20674, 31076, 32268]),
                    multi_ctx_cdf(&[12413, 24955, 29243]),
                    multi_ctx_cdf(&[6676, 16927, 23097]),
                    multi_ctx_cdf(&[2966, 8333, 12919]),
                ],
                [
                    multi_ctx_cdf(&[7555, 14942, 18501]),
                    multi_ctx_cdf(&[24410, 31178, 32287]),
                    multi_ctx_cdf(&[14394, 26738, 30253]),
                    multi_ctx_cdf(&[8413, 19554, 25195]),
                    multi_ctx_cdf(&[4766, 12924, 18785]),
                    multi_ctx_cdf(&[2029, 5806, 9207]),
                    multi_ctx_cdf(&[26776, 32364, 32663]),
                    multi_ctx_cdf(&[18732, 29967, 31931]),
                    multi_ctx_cdf(&[11005, 23786, 28852]),
                    multi_ctx_cdf(&[6466, 16909, 23510]),
                    multi_ctx_cdf(&[3044, 8638, 13419]),
                    multi_ctx_cdf(&[29208, 32582, 32704]),
                    multi_ctx_cdf(&[20068, 30857, 32208]),
                    multi_ctx_cdf(&[12003, 25085, 29595]),
                    multi_ctx_cdf(&[6947, 17750, 24189]),
                    multi_ctx_cdf(&[3245, 9103, 14007]),
                    multi_ctx_cdf(&[27359, 32465, 32669]),
                    multi_ctx_cdf(&[19421, 30614, 32174]),
                    multi_ctx_cdf(&[11915, 25010, 29579]),
                    multi_ctx_cdf(&[6950, 17676, 24074]),
                    multi_ctx_cdf(&[3007, 8473, 13096]),
                    multi_ctx_cdf(&[29002, 32676, 32735]),
                    multi_ctx_cdf(&[22102, 31849, 32576]),
                    multi_ctx_cdf(&[14408, 28009, 31405]),
                    multi_ctx_cdf(&[9027, 21679, 27931]),
                    multi_ctx_cdf(&[4694, 12678, 18748]),
                    multi_ctx_cdf(&[28216, 32528, 32682]),
                    multi_ctx_cdf(&[20849, 31264, 32318]),
                    multi_ctx_cdf(&[12756, 25815, 29751]),
                    multi_ctx_cdf(&[7565, 18801, 24923]),
                    multi_ctx_cdf(&[3509, 9533, 14477]),
                    multi_ctx_cdf(&[30133, 32687, 32739]),
                    multi_ctx_cdf(&[23063, 31910, 32515]),
                    multi_ctx_cdf(&[14588, 28051, 31132]),
                    multi_ctx_cdf(&[9085, 21649, 27457]),
                    multi_ctx_cdf(&[4261, 11654, 17264]),
                    multi_ctx_cdf(&[29518, 32691, 32748]),
                    multi_ctx_cdf(&[22451, 31959, 32613]),
                    multi_ctx_cdf(&[14864, 28722, 31700]),
                    multi_ctx_cdf(&[9695, 22964, 28716]),
                    multi_ctx_cdf(&[4932, 13358, 19502]),
                ],
                [
                    multi_ctx_cdf(&[3078, 6839, 9890]),
                    multi_ctx_cdf(&[13837, 20450, 24479]),
                    multi_ctx_cdf(&[5914, 14222, 19328]),
                    multi_ctx_cdf(&[3866, 10267, 14762]),
                    multi_ctx_cdf(&[2612, 7208, 11042]),
                    multi_ctx_cdf(&[1067, 2991, 4776]),
                    multi_ctx_cdf(&[25817, 31646, 32529]),
                    multi_ctx_cdf(&[13708, 26338, 30385]),
                    multi_ctx_cdf(&[7328, 18585, 24870]),
                    multi_ctx_cdf(&[4691, 13080, 19276]),
                    multi_ctx_cdf(&[1825, 5253, 8352]),
                    multi_ctx_cdf(&[29386, 32315, 32624]),
                    multi_ctx_cdf(&[17160, 29001, 31360]),
                    multi_ctx_cdf(&[9602, 21862, 27396]),
                    multi_ctx_cdf(&[5915, 15772, 22148]),
                    multi_ctx_cdf(&[2786, 7779, 12047]),
                    multi_ctx_cdf(&[29246, 32450, 32663]),
                    multi_ctx_cdf(&[18696, 29929, 31818]),
                    multi_ctx_cdf(&[10510, 23369, 28560]),
                    multi_ctx_cdf(&[6229, 16499, 23125]),
                    multi_ctx_cdf(&[2608, 7448, 11705]),
                    multi_ctx_cdf(&[30753, 32710, 32748]),
                    multi_ctx_cdf(&[21638, 31487, 32503]),
                    multi_ctx_cdf(&[12937, 26854, 30870]),
                    multi_ctx_cdf(&[8182, 20596, 26970]),
                    multi_ctx_cdf(&[3637, 10269, 15497]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                ],
                [
                    multi_ctx_cdf(&[1035, 2807, 4156]),
                    multi_ctx_cdf(&[13162, 18138, 20939]),
                    multi_ctx_cdf(&[2696, 6633, 8755]),
                    multi_ctx_cdf(&[1373, 4161, 6853]),
                    multi_ctx_cdf(&[1099, 2746, 4716]),
                    multi_ctx_cdf(&[340, 1021, 1599]),
                    multi_ctx_cdf(&[22826, 30419, 32135]),
                    multi_ctx_cdf(&[10395, 21762, 26942]),
                    multi_ctx_cdf(&[4726, 12407, 17361]),
                    multi_ctx_cdf(&[2447, 7080, 10593]),
                    multi_ctx_cdf(&[1227, 3717, 6011]),
                    multi_ctx_cdf(&[28156, 31424, 31934]),
                    multi_ctx_cdf(&[16915, 27754, 30373]),
                    multi_ctx_cdf(&[9148, 20990, 26431]),
                    multi_ctx_cdf(&[5950, 15515, 21148]),
                    multi_ctx_cdf(&[2492, 7327, 11526]),
                    multi_ctx_cdf(&[30602, 32477, 32670]),
                    multi_ctx_cdf(&[20026, 29955, 31568]),
                    multi_ctx_cdf(&[11220, 23628, 28105]),
                    multi_ctx_cdf(&[6652, 17019, 22973]),
                    multi_ctx_cdf(&[3064, 8536, 13043]),
                    multi_ctx_cdf(&[31769, 32724, 32748]),
                    multi_ctx_cdf(&[22230, 30887, 32373]),
                    multi_ctx_cdf(&[12234, 25079, 29731]),
                    multi_ctx_cdf(&[7326, 18816, 25353]),
                    multi_ctx_cdf(&[3933, 10907, 16616]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                ],
            ],
            2 => [
                [
                    multi_ctx_cdf(&[8896, 16227, 20630]),
                    multi_ctx_cdf(&[23629, 31782, 32527]),
                    multi_ctx_cdf(&[15173, 27755, 31321]),
                    multi_ctx_cdf(&[10158, 21233, 27382]),
                    multi_ctx_cdf(&[6420, 14857, 21558]),
                    multi_ctx_cdf(&[3269, 8155, 12646]),
                    multi_ctx_cdf(&[24835, 32009, 32496]),
                    multi_ctx_cdf(&[16509, 28421, 31579]),
                    multi_ctx_cdf(&[10957, 21514, 27418]),
                    multi_ctx_cdf(&[7881, 15930, 22096]),
                    multi_ctx_cdf(&[5388, 10960, 15918]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[20745, 30773, 32093]),
                    multi_ctx_cdf(&[15200, 27221, 30861]),
                    multi_ctx_cdf(&[13032, 20873, 25667]),
                    multi_ctx_cdf(&[12285, 18663, 23494]),
                    multi_ctx_cdf(&[11563, 17481, 21489]),
                    multi_ctx_cdf(&[26260, 31982, 32320]),
                    multi_ctx_cdf(&[15397, 28083, 31100]),
                    multi_ctx_cdf(&[9742, 19217, 24824]),
                    multi_ctx_cdf(&[3261, 9629, 15362]),
                    multi_ctx_cdf(&[1480, 4322, 7499]),
                    multi_ctx_cdf(&[27599, 32256, 32460]),
                    multi_ctx_cdf(&[16857, 27659, 30774]),
                    multi_ctx_cdf(&[9551, 18290, 23748]),
                    multi_ctx_cdf(&[3052, 8933, 14103]),
                    multi_ctx_cdf(&[2021, 5910, 9787]),
                    multi_ctx_cdf(&[29005, 32015, 32392]),
                    multi_ctx_cdf(&[17677, 27694, 30863]),
                    multi_ctx_cdf(&[9204, 17356, 23219]),
                    multi_ctx_cdf(&[2403, 7516, 12814]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                ],
                [
                    multi_ctx_cdf(&[9704, 17294, 21132]),
                    multi_ctx_cdf(&[26762, 32278, 32633]),
                    multi_ctx_cdf(&[18382, 29620, 31819]),
                    multi_ctx_cdf(&[10891, 23475, 28723]),
                    multi_ctx_cdf(&[6358, 16583, 23309]),
                    multi_ctx_cdf(&[3248, 9118, 14141]),
                    multi_ctx_cdf(&[27204, 32573, 32699]),
                    multi_ctx_cdf(&[19818, 30824, 32329]),
                    multi_ctx_cdf(&[11772, 25120, 30041]),
                    multi_ctx_cdf(&[6995, 18033, 25039]),
                    multi_ctx_cdf(&[3752, 10442, 16098]),
                    multi_ctx_cdf(&[27222, 32256, 32559]),
                    multi_ctx_cdf(&[15356, 28399, 31475]),
                    multi_ctx_cdf(&[8821, 20635, 27057]),
                    multi_ctx_cdf(&[5511, 14404, 21239]),
                    multi_ctx_cdf(&[2935, 8222, 13051]),
                    multi_ctx_cdf(&[24875, 32120, 32529]),
                    multi_ctx_cdf(&[15233, 28265, 31445]),
                    multi_ctx_cdf(&[8605, 20570, 26932]),
                    multi_ctx_cdf(&[5431, 14413, 21196]),
                    multi_ctx_cdf(&[2994, 8341, 13223]),
                    multi_ctx_cdf(&[28201, 32604, 32700]),
                    multi_ctx_cdf(&[21041, 31446, 32456]),
                    multi_ctx_cdf(&[13221, 26213, 30475]),
                    multi_ctx_cdf(&[8255, 19385, 26037]),
                    multi_ctx_cdf(&[4930, 12585, 18830]),
                    multi_ctx_cdf(&[28768, 32448, 32627]),
                    multi_ctx_cdf(&[19705, 30561, 32021]),
                    multi_ctx_cdf(&[11572, 23589, 28220]),
                    multi_ctx_cdf(&[5532, 15034, 21446]),
                    multi_ctx_cdf(&[2460, 7150, 11456]),
                    multi_ctx_cdf(&[29874, 32619, 32699]),
                    multi_ctx_cdf(&[21621, 31071, 32201]),
                    multi_ctx_cdf(&[12511, 24747, 28992]),
                    multi_ctx_cdf(&[6281, 16395, 22748]),
                    multi_ctx_cdf(&[3246, 9278, 14497]),
                    multi_ctx_cdf(&[29715, 32625, 32712]),
                    multi_ctx_cdf(&[20958, 31011, 32283]),
                    multi_ctx_cdf(&[11233, 23671, 28806]),
                    multi_ctx_cdf(&[6012, 16128, 22868]),
                    multi_ctx_cdf(&[3427, 9851, 15414]),
                ],
                [
                    multi_ctx_cdf(&[9800, 17635, 21073]),
                    multi_ctx_cdf(&[26153, 31885, 32527]),
                    multi_ctx_cdf(&[15038, 27852, 31006]),
                    multi_ctx_cdf(&[8718, 20564, 26486]),
                    multi_ctx_cdf(&[5128, 14076, 20514]),
                    multi_ctx_cdf(&[2636, 7566, 11925]),
                    multi_ctx_cdf(&[27551, 32504, 32701]),
                    multi_ctx_cdf(&[18310, 30054, 32100]),
                    multi_ctx_cdf(&[10211, 23420, 29082]),
                    multi_ctx_cdf(&[6222, 16876, 23916]),
                    multi_ctx_cdf(&[3462, 9954, 15498]),
                    multi_ctx_cdf(&[29991, 32633, 32721]),
                    multi_ctx_cdf(&[19883, 30751, 32201]),
                    multi_ctx_cdf(&[11141, 24184, 29285]),
                    multi_ctx_cdf(&[6420, 16940, 23774]),
                    multi_ctx_cdf(&[3392, 9753, 15118]),
                    multi_ctx_cdf(&[28465, 32616, 32712]),
                    multi_ctx_cdf(&[19850, 30702, 32244]),
                    multi_ctx_cdf(&[10983, 24024, 29223]),
                    multi_ctx_cdf(&[6294, 16770, 23582]),
                    multi_ctx_cdf(&[3244, 9283, 14509]),
                    multi_ctx_cdf(&[30023, 32717, 32748]),
                    multi_ctx_cdf(&[22940, 32032, 32626]),
                    multi_ctx_cdf(&[14282, 27928, 31473]),
                    multi_ctx_cdf(&[8562, 21327, 27914]),
                    multi_ctx_cdf(&[4846, 13393, 19919]),
                    multi_ctx_cdf(&[29981, 32590, 32695]),
                    multi_ctx_cdf(&[20465, 30963, 32166]),
                    multi_ctx_cdf(&[11479, 23579, 28195]),
                    multi_ctx_cdf(&[5916, 15648, 22073]),
                    multi_ctx_cdf(&[3031, 8605, 13398]),
                    multi_ctx_cdf(&[31146, 32691, 32739]),
                    multi_ctx_cdf(&[23106, 31724, 32444]),
                    multi_ctx_cdf(&[13783, 26738, 30439]),
                    multi_ctx_cdf(&[7852, 19468, 25807]),
                    multi_ctx_cdf(&[3860, 11124, 16853]),
                    multi_ctx_cdf(&[31014, 32724, 32748]),
                    multi_ctx_cdf(&[23629, 32109, 32628]),
                    multi_ctx_cdf(&[14747, 28115, 31403]),
                    multi_ctx_cdf(&[8545, 21242, 27478]),
                    multi_ctx_cdf(&[4574, 12781, 19067]),
                ],
                [
                    multi_ctx_cdf(&[4456, 11274, 15533]),
                    multi_ctx_cdf(&[21219, 29079, 31616]),
                    multi_ctx_cdf(&[11173, 23774, 28567]),
                    multi_ctx_cdf(&[7282, 18293, 24263]),
                    multi_ctx_cdf(&[4890, 13286, 19115]),
                    multi_ctx_cdf(&[1890, 5508, 8659]),
                    multi_ctx_cdf(&[26651, 32136, 32647]),
                    multi_ctx_cdf(&[14630, 28254, 31455]),
                    multi_ctx_cdf(&[8716, 21287, 27395]),
                    multi_ctx_cdf(&[5615, 15331, 22008]),
                    multi_ctx_cdf(&[2675, 7700, 12150]),
                    multi_ctx_cdf(&[29954, 32526, 32690]),
                    multi_ctx_cdf(&[16126, 28982, 31633]),
                    multi_ctx_cdf(&[9030, 21361, 27352]),
                    multi_ctx_cdf(&[5411, 14793, 21271]),
                    multi_ctx_cdf(&[2943, 8422, 13163]),
                    multi_ctx_cdf(&[29539, 32601, 32730]),
                    multi_ctx_cdf(&[18125, 30385, 32201]),
                    multi_ctx_cdf(&[10422, 24090, 29468]),
                    multi_ctx_cdf(&[6468, 17487, 24438]),
                    multi_ctx_cdf(&[2970, 8653, 13531]),
                    multi_ctx_cdf(&[30912, 32715, 32748]),
                    multi_ctx_cdf(&[20666, 31373, 32497]),
                    multi_ctx_cdf(&[12509, 26640, 30917]),
                    multi_ctx_cdf(&[8058, 20629, 27290]),
                    multi_ctx_cdf(&[4231, 12006, 18052]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                ],
                [
                    multi_ctx_cdf(&[2575, 7281, 11077]),
                    multi_ctx_cdf(&[14002, 20866, 25402]),
                    multi_ctx_cdf(&[6343, 15056, 19658]),
                    multi_ctx_cdf(&[4474, 11858, 17041]),
                    multi_ctx_cdf(&[2865, 8299, 12534]),
                    multi_ctx_cdf(&[1344, 3949, 6391]),
                    multi_ctx_cdf(&[24720, 31239, 32459]),
                    multi_ctx_cdf(&[12585, 25356, 29968]),
                    multi_ctx_cdf(&[7181, 18246, 24444]),
                    multi_ctx_cdf(&[5025, 13667, 19885]),
                    multi_ctx_cdf(&[2521, 7304, 11605]),
                    multi_ctx_cdf(&[29908, 32252, 32584]),
                    multi_ctx_cdf(&[17421, 29156, 31575]),
                    multi_ctx_cdf(&[9889, 22188, 27782]),
                    multi_ctx_cdf(&[5878, 15647, 22123]),
                    multi_ctx_cdf(&[2814, 8665, 13323]),
                    multi_ctx_cdf(&[30183, 32568, 32713]),
                    multi_ctx_cdf(&[18528, 30195, 32049]),
                    multi_ctx_cdf(&[10982, 24606, 29657]),
                    multi_ctx_cdf(&[6957, 18165, 25231]),
                    multi_ctx_cdf(&[3508, 10118, 15468]),
                    multi_ctx_cdf(&[31761, 32736, 32748]),
                    multi_ctx_cdf(&[21041, 31328, 32546]),
                    multi_ctx_cdf(&[12568, 26732, 31166]),
                    multi_ctx_cdf(&[8052, 20720, 27733]),
                    multi_ctx_cdf(&[4336, 12192, 18396]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                ],
            ],
            3 => [
                [
                    multi_ctx_cdf(&[7062, 16472, 22319]),
                    multi_ctx_cdf(&[24538, 32261, 32674]),
                    multi_ctx_cdf(&[13675, 28041, 31779]),
                    multi_ctx_cdf(&[8590, 20674, 27631]),
                    multi_ctx_cdf(&[5685, 14675, 22013]),
                    multi_ctx_cdf(&[3655, 9898, 15731]),
                    multi_ctx_cdf(&[26493, 32418, 32658]),
                    multi_ctx_cdf(&[16376, 29342, 32090]),
                    multi_ctx_cdf(&[10594, 22649, 28970]),
                    multi_ctx_cdf(&[8176, 17170, 24303]),
                    multi_ctx_cdf(&[5605, 12694, 19139]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[23888, 31902, 32542]),
                    multi_ctx_cdf(&[18612, 29687, 31987]),
                    multi_ctx_cdf(&[16245, 24852, 29249]),
                    multi_ctx_cdf(&[15765, 22608, 27559]),
                    multi_ctx_cdf(&[19895, 24699, 27510]),
                    multi_ctx_cdf(&[28401, 32212, 32457]),
                    multi_ctx_cdf(&[15274, 27825, 30980]),
                    multi_ctx_cdf(&[9364, 18128, 24332]),
                    multi_ctx_cdf(&[2283, 8193, 15082]),
                    multi_ctx_cdf(&[1228, 3972, 7881]),
                    multi_ctx_cdf(&[29455, 32469, 32620]),
                    multi_ctx_cdf(&[17981, 28245, 31388]),
                    multi_ctx_cdf(&[10921, 20098, 26240]),
                    multi_ctx_cdf(&[3743, 11829, 18657]),
                    multi_ctx_cdf(&[2374, 9593, 15715]),
                    multi_ctx_cdf(&[31068, 32466, 32635]),
                    multi_ctx_cdf(&[20321, 29572, 31971]),
                    multi_ctx_cdf(&[10771, 20255, 27119]),
                    multi_ctx_cdf(&[2795, 10410, 17361]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                ],
                [
                    multi_ctx_cdf(&[7754, 16948, 22142]),
                    multi_ctx_cdf(&[25670, 32330, 32691]),
                    multi_ctx_cdf(&[15663, 29225, 31994]),
                    multi_ctx_cdf(&[9878, 23288, 29158]),
                    multi_ctx_cdf(&[6419, 17088, 24336]),
                    multi_ctx_cdf(&[3859, 11003, 17039]),
                    multi_ctx_cdf(&[27562, 32595, 32725]),
                    multi_ctx_cdf(&[17575, 30588, 32399]),
                    multi_ctx_cdf(&[10819, 24838, 30309]),
                    multi_ctx_cdf(&[7124, 18686, 25916]),
                    multi_ctx_cdf(&[4479, 12688, 19340]),
                    multi_ctx_cdf(&[28385, 32476, 32673]),
                    multi_ctx_cdf(&[15306, 29005, 31938]),
                    multi_ctx_cdf(&[8937, 21615, 28322]),
                    multi_ctx_cdf(&[5982, 15603, 22786]),
                    multi_ctx_cdf(&[3620, 10267, 16136]),
                    multi_ctx_cdf(&[27280, 32464, 32667]),
                    multi_ctx_cdf(&[15607, 29160, 32004]),
                    multi_ctx_cdf(&[9091, 22135, 28740]),
                    multi_ctx_cdf(&[6232, 16632, 24020]),
                    multi_ctx_cdf(&[4047, 11377, 17672]),
                    multi_ctx_cdf(&[29220, 32630, 32718]),
                    multi_ctx_cdf(&[19650, 31220, 32462]),
                    multi_ctx_cdf(&[13050, 26312, 30827]),
                    multi_ctx_cdf(&[9228, 20870, 27468]),
                    multi_ctx_cdf(&[6146, 15149, 21971]),
                    multi_ctx_cdf(&[30169, 32481, 32623]),
                    multi_ctx_cdf(&[17212, 29311, 31554]),
                    multi_ctx_cdf(&[9911, 21311, 26882]),
                    multi_ctx_cdf(&[4487, 13314, 20372]),
                    multi_ctx_cdf(&[2570, 7772, 12889]),
                    multi_ctx_cdf(&[30924, 32613, 32708]),
                    multi_ctx_cdf(&[19490, 30206, 32107]),
                    multi_ctx_cdf(&[11232, 23998, 29276]),
                    multi_ctx_cdf(&[6769, 17955, 25035]),
                    multi_ctx_cdf(&[4398, 12623, 19214]),
                    multi_ctx_cdf(&[30609, 32627, 32722]),
                    multi_ctx_cdf(&[19370, 30582, 32287]),
                    multi_ctx_cdf(&[10457, 23619, 29409]),
                    multi_ctx_cdf(&[6443, 17637, 24834]),
                    multi_ctx_cdf(&[4645, 13236, 20106]),
                ],
                [
                    multi_ctx_cdf(&[7876, 16901, 21741]),
                    multi_ctx_cdf(&[24001, 31898, 32625]),
                    multi_ctx_cdf(&[14529, 27959, 31451]),
                    multi_ctx_cdf(&[8273, 20818, 27258]),
                    multi_ctx_cdf(&[5278, 14673, 21510]),
                    multi_ctx_cdf(&[2983, 8843, 14039]),
                    multi_ctx_cdf(&[28016, 32574, 32732]),
                    multi_ctx_cdf(&[17471, 30306, 32301]),
                    multi_ctx_cdf(&[10224, 24063, 29728]),
                    multi_ctx_cdf(&[6602, 17954, 25052]),
                    multi_ctx_cdf(&[4002, 11585, 17759]),
                    multi_ctx_cdf(&[30190, 32634, 32739]),
                    multi_ctx_cdf(&[17497, 30282, 32270]),
                    multi_ctx_cdf(&[10229, 23729, 29538]),
                    multi_ctx_cdf(&[6344, 17211, 24440]),
                    multi_ctx_cdf(&[3849, 11189, 17108]),
                    multi_ctx_cdf(&[28570, 32583, 32726]),
                    multi_ctx_cdf(&[17521, 30161, 32238]),
                    multi_ctx_cdf(&[10153, 23565, 29378]),
                    multi_ctx_cdf(&[6455, 17341, 24443]),
                    multi_ctx_cdf(&[3907, 11042, 17024]),
                    multi_ctx_cdf(&[30689, 32715, 32748]),
                    multi_ctx_cdf(&[21546, 31840, 32610]),
                    multi_ctx_cdf(&[13547, 27581, 31459]),
                    multi_ctx_cdf(&[8912, 21757, 28309]),
                    multi_ctx_cdf(&[5548, 15080, 22046]),
                    multi_ctx_cdf(&[30783, 32540, 32685]),
                    multi_ctx_cdf(&[17540, 29528, 31668]),
                    multi_ctx_cdf(&[10160, 21468, 26783]),
                    multi_ctx_cdf(&[4724, 13393, 20054]),
                    multi_ctx_cdf(&[2702, 8174, 13102]),
                    multi_ctx_cdf(&[31648, 32686, 32742]),
                    multi_ctx_cdf(&[20954, 31094, 32337]),
                    multi_ctx_cdf(&[12420, 25698, 30179]),
                    multi_ctx_cdf(&[7304, 19320, 26248]),
                    multi_ctx_cdf(&[4366, 12261, 18864]),
                    multi_ctx_cdf(&[31581, 32723, 32748]),
                    multi_ctx_cdf(&[21373, 31586, 32525]),
                    multi_ctx_cdf(&[12744, 26625, 30885]),
                    multi_ctx_cdf(&[7431, 20322, 26950]),
                    multi_ctx_cdf(&[4692, 13323, 20111]),
                ],
                [
                    multi_ctx_cdf(&[5992, 14304, 19765]),
                    multi_ctx_cdf(&[22612, 31238, 32456]),
                    multi_ctx_cdf(&[13456, 27162, 31087]),
                    multi_ctx_cdf(&[8001, 20062, 26504]),
                    multi_ctx_cdf(&[5168, 14105, 20764]),
                    multi_ctx_cdf(&[2632, 7771, 12385]),
                    multi_ctx_cdf(&[27034, 32344, 32709]),
                    multi_ctx_cdf(&[15850, 29415, 31997]),
                    multi_ctx_cdf(&[9494, 22776, 28841]),
                    multi_ctx_cdf(&[6151, 16830, 23969]),
                    multi_ctx_cdf(&[3461, 10039, 15722]),
                    multi_ctx_cdf(&[30134, 32569, 32731]),
                    multi_ctx_cdf(&[15638, 29422, 31945]),
                    multi_ctx_cdf(&[9150, 21865, 28218]),
                    multi_ctx_cdf(&[5647, 15719, 22676]),
                    multi_ctx_cdf(&[3402, 9772, 15477]),
                    multi_ctx_cdf(&[28530, 32586, 32735]),
                    multi_ctx_cdf(&[17139, 30298, 32292]),
                    multi_ctx_cdf(&[10200, 24039, 29685]),
                    multi_ctx_cdf(&[6419, 17674, 24786]),
                    multi_ctx_cdf(&[3544, 10225, 15824]),
                    multi_ctx_cdf(&[31333, 32726, 32748]),
                    multi_ctx_cdf(&[20618, 31487, 32544]),
                    multi_ctx_cdf(&[12901, 27217, 31232]),
                    multi_ctx_cdf(&[8624, 21734, 28171]),
                    multi_ctx_cdf(&[5104, 14191, 20748]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                ],
                [
                    multi_ctx_cdf(&[4137, 10847, 15682]),
                    multi_ctx_cdf(&[17824, 27001, 30058]),
                    multi_ctx_cdf(&[10204, 22796, 28291]),
                    multi_ctx_cdf(&[6076, 15935, 22125]),
                    multi_ctx_cdf(&[3852, 10937, 16816]),
                    multi_ctx_cdf(&[2252, 6324, 10131]),
                    multi_ctx_cdf(&[25840, 32016, 32662]),
                    multi_ctx_cdf(&[15109, 28268, 31531]),
                    multi_ctx_cdf(&[9385, 22231, 28340]),
                    multi_ctx_cdf(&[6082, 16672, 23479]),
                    multi_ctx_cdf(&[3318, 9427, 14681]),
                    multi_ctx_cdf(&[30594, 32574, 32718]),
                    multi_ctx_cdf(&[16836, 29552, 31859]),
                    multi_ctx_cdf(&[9556, 22542, 28356]),
                    multi_ctx_cdf(&[6305, 16725, 23540]),
                    multi_ctx_cdf(&[3376, 9895, 15184]),
                    multi_ctx_cdf(&[29383, 32617, 32745]),
                    multi_ctx_cdf(&[18891, 30809, 32401]),
                    multi_ctx_cdf(&[11688, 25942, 30687]),
                    multi_ctx_cdf(&[7468, 19469, 26651]),
                    multi_ctx_cdf(&[3909, 11358, 17012]),
                    multi_ctx_cdf(&[31564, 32736, 32748]),
                    multi_ctx_cdf(&[20906, 31611, 32600]),
                    multi_ctx_cdf(&[13191, 27621, 31537]),
                    multi_ctx_cdf(&[8768, 22029, 28676]),
                    multi_ctx_cdf(&[5079, 14109, 20906]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                    multi_ctx_cdf(&[8192, 16384, 24576]),
                ],
            ],
            _ => unreachable!(),
        };

        // coeff_br: real spec/rav1d default CDFs, indexed [min(tx_size_class,3)][ctx 0..=20]
        // (chroma=0 fixed -- see `coeff_br_cdf`'s doc). Source: rav1d `coef.br_tok`, real
        // per-frame qindex-bucket selection.
        let coeff_br_cdf: [[Vec<u16>; 21]; 4] = match qcat.min(3) {
            0 => [
                [
                    multi_ctx_cdf(&[14298, 20718, 24174]),
                    multi_ctx_cdf(&[12536, 19601, 23789]),
                    multi_ctx_cdf(&[8712, 15051, 19503]),
                    multi_ctx_cdf(&[6170, 11327, 15434]),
                    multi_ctx_cdf(&[4742, 8926, 12538]),
                    multi_ctx_cdf(&[3803, 7317, 10546]),
                    multi_ctx_cdf(&[1696, 3317, 4871]),
                    multi_ctx_cdf(&[14392, 19951, 22756]),
                    multi_ctx_cdf(&[15978, 23218, 26818]),
                    multi_ctx_cdf(&[12187, 19474, 23889]),
                    multi_ctx_cdf(&[9176, 15640, 20259]),
                    multi_ctx_cdf(&[7068, 12655, 17028]),
                    multi_ctx_cdf(&[5656, 10442, 14472]),
                    multi_ctx_cdf(&[2580, 4992, 7244]),
                    multi_ctx_cdf(&[12136, 18049, 21426]),
                    multi_ctx_cdf(&[13784, 20721, 24481]),
                    multi_ctx_cdf(&[10836, 17621, 21900]),
                    multi_ctx_cdf(&[8372, 14444, 18847]),
                    multi_ctx_cdf(&[6523, 11779, 16000]),
                    multi_ctx_cdf(&[5337, 9898, 13760]),
                    multi_ctx_cdf(&[3034, 5860, 8462]),
                ],
                [
                    multi_ctx_cdf(&[14406, 20862, 24414]),
                    multi_ctx_cdf(&[11824, 18907, 23109]),
                    multi_ctx_cdf(&[8257, 14393, 18803]),
                    multi_ctx_cdf(&[5860, 10747, 14778]),
                    multi_ctx_cdf(&[4475, 8486, 11984]),
                    multi_ctx_cdf(&[3606, 6954, 10043]),
                    multi_ctx_cdf(&[1736, 3410, 5048]),
                    multi_ctx_cdf(&[14430, 20046, 22882]),
                    multi_ctx_cdf(&[15593, 22899, 26709]),
                    multi_ctx_cdf(&[12102, 19368, 23811]),
                    multi_ctx_cdf(&[9059, 15584, 20262]),
                    multi_ctx_cdf(&[6999, 12603, 17048]),
                    multi_ctx_cdf(&[5684, 10497, 14553]),
                    multi_ctx_cdf(&[2822, 5438, 7862]),
                    multi_ctx_cdf(&[15785, 21585, 24359]),
                    multi_ctx_cdf(&[18347, 25229, 28266]),
                    multi_ctx_cdf(&[14974, 22487, 26389]),
                    multi_ctx_cdf(&[11423, 18681, 23271]),
                    multi_ctx_cdf(&[8863, 15350, 20008]),
                    multi_ctx_cdf(&[7153, 12852, 17278]),
                    multi_ctx_cdf(&[3707, 7036, 9982]),
                ],
                [
                    multi_ctx_cdf(&[10563, 16233, 19763]),
                    multi_ctx_cdf(&[9794, 16022, 19804]),
                    multi_ctx_cdf(&[6750, 11945, 15759]),
                    multi_ctx_cdf(&[4963, 9186, 12752]),
                    multi_ctx_cdf(&[3845, 7435, 10627]),
                    multi_ctx_cdf(&[3051, 6085, 8834]),
                    multi_ctx_cdf(&[1311, 2596, 3830]),
                    multi_ctx_cdf(&[11246, 16404, 19689]),
                    multi_ctx_cdf(&[12315, 18911, 22731]),
                    multi_ctx_cdf(&[10557, 17095, 21289]),
                    multi_ctx_cdf(&[8136, 14006, 18249]),
                    multi_ctx_cdf(&[6348, 11474, 15565]),
                    multi_ctx_cdf(&[5196, 9655, 13400]),
                    multi_ctx_cdf(&[2349, 4526, 6587]),
                    multi_ctx_cdf(&[13337, 18730, 21569]),
                    multi_ctx_cdf(&[19306, 26071, 28882]),
                    multi_ctx_cdf(&[15952, 23540, 27254]),
                    multi_ctx_cdf(&[12409, 19934, 24430]),
                    multi_ctx_cdf(&[9760, 16706, 21389]),
                    multi_ctx_cdf(&[8004, 14220, 18818]),
                    multi_ctx_cdf(&[4138, 7794, 10961]),
                ],
                [
                    multi_ctx_cdf(&[2331, 3662, 5244]),
                    multi_ctx_cdf(&[2891, 4771, 6145]),
                    multi_ctx_cdf(&[4598, 7623, 9729]),
                    multi_ctx_cdf(&[3520, 6845, 9199]),
                    multi_ctx_cdf(&[3417, 6119, 9324]),
                    multi_ctx_cdf(&[2601, 5412, 7385]),
                    multi_ctx_cdf(&[600, 1173, 1744]),
                    multi_ctx_cdf(&[7672, 13286, 17469]),
                    multi_ctx_cdf(&[4232, 7792, 10793]),
                    multi_ctx_cdf(&[2915, 5317, 7397]),
                    multi_ctx_cdf(&[2318, 4356, 6152]),
                    multi_ctx_cdf(&[2127, 4000, 5554]),
                    multi_ctx_cdf(&[1850, 3478, 5275]),
                    multi_ctx_cdf(&[977, 1933, 2843]),
                    multi_ctx_cdf(&[18280, 24387, 27989]),
                    multi_ctx_cdf(&[15852, 22671, 26185]),
                    multi_ctx_cdf(&[13845, 20951, 24789]),
                    multi_ctx_cdf(&[11055, 17966, 22129]),
                    multi_ctx_cdf(&[9138, 15422, 19801]),
                    multi_ctx_cdf(&[7454, 13145, 17456]),
                    multi_ctx_cdf(&[3370, 6393, 9013]),
                ],
            ],
            1 => [
                [
                    multi_ctx_cdf(&[14995, 21341, 24749]),
                    multi_ctx_cdf(&[13158, 20289, 24601]),
                    multi_ctx_cdf(&[8941, 15326, 19876]),
                    multi_ctx_cdf(&[6297, 11541, 15807]),
                    multi_ctx_cdf(&[4817, 9029, 12776]),
                    multi_ctx_cdf(&[3731, 7273, 10627]),
                    multi_ctx_cdf(&[1847, 3617, 5354]),
                    multi_ctx_cdf(&[14472, 19659, 22343]),
                    multi_ctx_cdf(&[16806, 24162, 27533]),
                    multi_ctx_cdf(&[12900, 20404, 24713]),
                    multi_ctx_cdf(&[9411, 16112, 20797]),
                    multi_ctx_cdf(&[7056, 12697, 17148]),
                    multi_ctx_cdf(&[5544, 10339, 14460]),
                    multi_ctx_cdf(&[2954, 5704, 8319]),
                    multi_ctx_cdf(&[12464, 18071, 21354]),
                    multi_ctx_cdf(&[15482, 22528, 26034]),
                    multi_ctx_cdf(&[12070, 19269, 23624]),
                    multi_ctx_cdf(&[8953, 15406, 20106]),
                    multi_ctx_cdf(&[7027, 12730, 17220]),
                    multi_ctx_cdf(&[5887, 10913, 15140]),
                    multi_ctx_cdf(&[3793, 7278, 10447]),
                ],
                [
                    multi_ctx_cdf(&[15999, 22208, 25449]),
                    multi_ctx_cdf(&[13050, 19988, 24122]),
                    multi_ctx_cdf(&[8594, 14864, 19378]),
                    multi_ctx_cdf(&[6033, 11079, 15238]),
                    multi_ctx_cdf(&[4554, 8683, 12347]),
                    multi_ctx_cdf(&[3672, 7139, 10337]),
                    multi_ctx_cdf(&[1900, 3771, 5576]),
                    multi_ctx_cdf(&[15788, 21340, 23949]),
                    multi_ctx_cdf(&[16825, 24235, 27758]),
                    multi_ctx_cdf(&[12873, 20402, 24810]),
                    multi_ctx_cdf(&[9590, 16363, 21094]),
                    multi_ctx_cdf(&[7352, 13209, 17733]),
                    multi_ctx_cdf(&[5960, 10989, 15184]),
                    multi_ctx_cdf(&[3232, 6234, 9007]),
                    multi_ctx_cdf(&[15761, 20716, 23224]),
                    multi_ctx_cdf(&[19318, 25989, 28759]),
                    multi_ctx_cdf(&[15529, 23094, 26929]),
                    multi_ctx_cdf(&[11662, 18989, 23641]),
                    multi_ctx_cdf(&[8955, 15568, 20366]),
                    multi_ctx_cdf(&[7281, 13106, 17708]),
                    multi_ctx_cdf(&[4248, 8059, 11440]),
                ],
                [
                    multi_ctx_cdf(&[13549, 19724, 23158]),
                    multi_ctx_cdf(&[11844, 18382, 22246]),
                    multi_ctx_cdf(&[7919, 13619, 17773]),
                    multi_ctx_cdf(&[5486, 10143, 13946]),
                    multi_ctx_cdf(&[4166, 7983, 11324]),
                    multi_ctx_cdf(&[3364, 6506, 9427]),
                    multi_ctx_cdf(&[1598, 3160, 4674]),
                    multi_ctx_cdf(&[15281, 20979, 23781]),
                    multi_ctx_cdf(&[14939, 22119, 25952]),
                    multi_ctx_cdf(&[11363, 18407, 22812]),
                    multi_ctx_cdf(&[8609, 14857, 19370]),
                    multi_ctx_cdf(&[6737, 12184, 16480]),
                    multi_ctx_cdf(&[5506, 10263, 14262]),
                    multi_ctx_cdf(&[2990, 5786, 8380]),
                    multi_ctx_cdf(&[20249, 25253, 27417]),
                    multi_ctx_cdf(&[21070, 27518, 30001]),
                    multi_ctx_cdf(&[16854, 24469, 28074]),
                    multi_ctx_cdf(&[12864, 20486, 25000]),
                    multi_ctx_cdf(&[9962, 16978, 21778]),
                    multi_ctx_cdf(&[8074, 14338, 19048]),
                    multi_ctx_cdf(&[4494, 8479, 11906]),
                ],
                [
                    multi_ctx_cdf(&[5705, 10930, 15725]),
                    multi_ctx_cdf(&[7946, 12765, 16115]),
                    multi_ctx_cdf(&[6801, 12123, 16226]),
                    multi_ctx_cdf(&[5462, 10135, 14200]),
                    multi_ctx_cdf(&[4189, 8011, 11507]),
                    multi_ctx_cdf(&[3191, 6229, 9408]),
                    multi_ctx_cdf(&[1057, 2137, 3212]),
                    multi_ctx_cdf(&[10018, 17067, 21491]),
                    multi_ctx_cdf(&[7380, 12582, 16453]),
                    multi_ctx_cdf(&[6068, 10845, 14339]),
                    multi_ctx_cdf(&[5098, 9198, 12555]),
                    multi_ctx_cdf(&[4312, 8010, 11119]),
                    multi_ctx_cdf(&[3700, 6966, 9781]),
                    multi_ctx_cdf(&[1693, 3326, 4887]),
                    multi_ctx_cdf(&[18757, 24930, 27774]),
                    multi_ctx_cdf(&[17648, 24596, 27817]),
                    multi_ctx_cdf(&[14707, 22052, 26026]),
                    multi_ctx_cdf(&[11720, 18852, 23292]),
                    multi_ctx_cdf(&[9357, 15952, 20525]),
                    multi_ctx_cdf(&[7810, 13753, 18210]),
                    multi_ctx_cdf(&[3879, 7333, 10328]),
                ],
            ],
            2 => [
                [
                    multi_ctx_cdf(&[16138, 22223, 25509]),
                    multi_ctx_cdf(&[15347, 22430, 26332]),
                    multi_ctx_cdf(&[9614, 16736, 21332]),
                    multi_ctx_cdf(&[6600, 12275, 16907]),
                    multi_ctx_cdf(&[4811, 9424, 13547]),
                    multi_ctx_cdf(&[3748, 7809, 11420]),
                    multi_ctx_cdf(&[2254, 4587, 6890]),
                    multi_ctx_cdf(&[15196, 20284, 23177]),
                    multi_ctx_cdf(&[18317, 25469, 28451]),
                    multi_ctx_cdf(&[13918, 21651, 25842]),
                    multi_ctx_cdf(&[10052, 17150, 21995]),
                    multi_ctx_cdf(&[7499, 13630, 18587]),
                    multi_ctx_cdf(&[6158, 11417, 16003]),
                    multi_ctx_cdf(&[4014, 7785, 11252]),
                    multi_ctx_cdf(&[15048, 21067, 24384]),
                    multi_ctx_cdf(&[18202, 25346, 28553]),
                    multi_ctx_cdf(&[14302, 22019, 26356]),
                    multi_ctx_cdf(&[10839, 18139, 23166]),
                    multi_ctx_cdf(&[8715, 15744, 20806]),
                    multi_ctx_cdf(&[7536, 13576, 18544]),
                    multi_ctx_cdf(&[5413, 10335, 14498]),
                ],
                [
                    multi_ctx_cdf(&[16214, 22380, 25770]),
                    multi_ctx_cdf(&[14213, 21304, 25295]),
                    multi_ctx_cdf(&[9213, 15823, 20455]),
                    multi_ctx_cdf(&[6395, 11758, 16139]),
                    multi_ctx_cdf(&[4779, 9187, 13066]),
                    multi_ctx_cdf(&[3821, 7501, 10953]),
                    multi_ctx_cdf(&[2293, 4567, 6795]),
                    multi_ctx_cdf(&[15859, 21283, 23820]),
                    multi_ctx_cdf(&[18404, 25602, 28726]),
                    multi_ctx_cdf(&[14325, 21980, 26206]),
                    multi_ctx_cdf(&[10669, 17937, 22720]),
                    multi_ctx_cdf(&[8297, 14642, 19447]),
                    multi_ctx_cdf(&[6746, 12389, 16893]),
                    multi_ctx_cdf(&[4324, 8251, 11770]),
                    multi_ctx_cdf(&[16532, 21631, 24475]),
                    multi_ctx_cdf(&[20667, 27150, 29668]),
                    multi_ctx_cdf(&[16728, 24510, 28175]),
                    multi_ctx_cdf(&[12861, 20645, 25332]),
                    multi_ctx_cdf(&[10076, 17361, 22417]),
                    multi_ctx_cdf(&[8395, 14940, 19963]),
                    multi_ctx_cdf(&[5731, 10683, 14912]),
                ],
                [
                    multi_ctx_cdf(&[14438, 20798, 24089]),
                    multi_ctx_cdf(&[12621, 19203, 23097]),
                    multi_ctx_cdf(&[8177, 14125, 18402]),
                    multi_ctx_cdf(&[5674, 10501, 14456]),
                    multi_ctx_cdf(&[4236, 8239, 11733]),
                    multi_ctx_cdf(&[3447, 6750, 9806]),
                    multi_ctx_cdf(&[1986, 3950, 5864]),
                    multi_ctx_cdf(&[16208, 22099, 24930]),
                    multi_ctx_cdf(&[16537, 24025, 27585]),
                    multi_ctx_cdf(&[12780, 20381, 24867]),
                    multi_ctx_cdf(&[9767, 16612, 21416]),
                    multi_ctx_cdf(&[7686, 13738, 18398]),
                    multi_ctx_cdf(&[6333, 11614, 15964]),
                    multi_ctx_cdf(&[3941, 7571, 10836]),
                    multi_ctx_cdf(&[22819, 27422, 29202]),
                    multi_ctx_cdf(&[22224, 28514, 30721]),
                    multi_ctx_cdf(&[17660, 25433, 28913]),
                    multi_ctx_cdf(&[13574, 21482, 26002]),
                    multi_ctx_cdf(&[10629, 17977, 22938]),
                    multi_ctx_cdf(&[8612, 15298, 20265]),
                    multi_ctx_cdf(&[5607, 10491, 14596]),
                ],
                [
                    multi_ctx_cdf(&[9040, 14786, 18360]),
                    multi_ctx_cdf(&[9979, 15718, 19415]),
                    multi_ctx_cdf(&[7913, 13918, 18311]),
                    multi_ctx_cdf(&[5859, 10889, 15184]),
                    multi_ctx_cdf(&[4593, 8677, 12510]),
                    multi_ctx_cdf(&[3820, 7396, 10791]),
                    multi_ctx_cdf(&[1730, 3471, 5192]),
                    multi_ctx_cdf(&[11803, 18365, 22709]),
                    multi_ctx_cdf(&[11419, 18058, 22225]),
                    multi_ctx_cdf(&[9418, 15774, 20243]),
                    multi_ctx_cdf(&[7539, 13325, 17657]),
                    multi_ctx_cdf(&[6233, 11317, 15384]),
                    multi_ctx_cdf(&[5137, 9656, 13545]),
                    multi_ctx_cdf(&[2977, 5774, 8349]),
                    multi_ctx_cdf(&[21207, 27246, 29640]),
                    multi_ctx_cdf(&[19547, 26578, 29497]),
                    multi_ctx_cdf(&[16169, 23871, 27690]),
                    multi_ctx_cdf(&[12820, 20458, 25018]),
                    multi_ctx_cdf(&[10224, 17332, 22214]),
                    multi_ctx_cdf(&[8526, 15048, 19884]),
                    multi_ctx_cdf(&[5037, 9410, 13118]),
                ],
            ],
            3 => [
                [
                    multi_ctx_cdf(&[18315, 24289, 27551]),
                    multi_ctx_cdf(&[16854, 24068, 27835]),
                    multi_ctx_cdf(&[10140, 17927, 23173]),
                    multi_ctx_cdf(&[6722, 12982, 18267]),
                    multi_ctx_cdf(&[4661, 9826, 14706]),
                    multi_ctx_cdf(&[3832, 8165, 12294]),
                    multi_ctx_cdf(&[2795, 6098, 9245]),
                    multi_ctx_cdf(&[17145, 23326, 26672]),
                    multi_ctx_cdf(&[20733, 27680, 30308]),
                    multi_ctx_cdf(&[16032, 24461, 28546]),
                    multi_ctx_cdf(&[11653, 20093, 25081]),
                    multi_ctx_cdf(&[9290, 16429, 22086]),
                    multi_ctx_cdf(&[7796, 14598, 19982]),
                    multi_ctx_cdf(&[6502, 12378, 17441]),
                    multi_ctx_cdf(&[21681, 27732, 30320]),
                    multi_ctx_cdf(&[22389, 29044, 31261]),
                    multi_ctx_cdf(&[19027, 26731, 30087]),
                    multi_ctx_cdf(&[14739, 23755, 28624]),
                    multi_ctx_cdf(&[11358, 20778, 25511]),
                    multi_ctx_cdf(&[10995, 18073, 24190]),
                    multi_ctx_cdf(&[9162, 14990, 20617]),
                ],
                [
                    multi_ctx_cdf(&[18274, 24813, 27890]),
                    multi_ctx_cdf(&[15537, 23149, 27003]),
                    multi_ctx_cdf(&[9449, 16740, 21827]),
                    multi_ctx_cdf(&[6700, 12498, 17261]),
                    multi_ctx_cdf(&[4988, 9866, 14198]),
                    multi_ctx_cdf(&[4236, 8147, 11902]),
                    multi_ctx_cdf(&[2867, 5860, 8654]),
                    multi_ctx_cdf(&[17124, 23171, 26101]),
                    multi_ctx_cdf(&[20396, 27477, 30148]),
                    multi_ctx_cdf(&[16573, 24629, 28492]),
                    multi_ctx_cdf(&[12749, 20846, 25674]),
                    multi_ctx_cdf(&[10233, 17878, 22818]),
                    multi_ctx_cdf(&[8525, 15332, 20363]),
                    multi_ctx_cdf(&[6283, 11632, 16255]),
                    multi_ctx_cdf(&[20466, 26511, 29286]),
                    multi_ctx_cdf(&[23059, 29174, 31191]),
                    multi_ctx_cdf(&[19481, 27263, 30241]),
                    multi_ctx_cdf(&[15458, 23631, 28137]),
                    multi_ctx_cdf(&[12416, 20608, 25693]),
                    multi_ctx_cdf(&[10261, 18011, 23261]),
                    multi_ctx_cdf(&[8016, 14655, 19666]),
                ],
                [
                    multi_ctx_cdf(&[17113, 23733, 27081]),
                    multi_ctx_cdf(&[14139, 21406, 25452]),
                    multi_ctx_cdf(&[8552, 15002, 19776]),
                    multi_ctx_cdf(&[5871, 11120, 15378]),
                    multi_ctx_cdf(&[4455, 8616, 12253]),
                    multi_ctx_cdf(&[3469, 6910, 10386]),
                    multi_ctx_cdf(&[2255, 4553, 6782]),
                    multi_ctx_cdf(&[18224, 24376, 27053]),
                    multi_ctx_cdf(&[19290, 26710, 29614]),
                    multi_ctx_cdf(&[14936, 22991, 27184]),
                    multi_ctx_cdf(&[11238, 18951, 23762]),
                    multi_ctx_cdf(&[8786, 15617, 20588]),
                    multi_ctx_cdf(&[7317, 13228, 18003]),
                    multi_ctx_cdf(&[5101, 9512, 13493]),
                    multi_ctx_cdf(&[22639, 28222, 30210]),
                    multi_ctx_cdf(&[23216, 29331, 31307]),
                    multi_ctx_cdf(&[19075, 26762, 29895]),
                    multi_ctx_cdf(&[15014, 23113, 27457]),
                    multi_ctx_cdf(&[11938, 19857, 24752]),
                    multi_ctx_cdf(&[9942, 17280, 22282]),
                    multi_ctx_cdf(&[7167, 13144, 17752]),
                ],
                [
                    multi_ctx_cdf(&[12162, 18785, 22648]),
                    multi_ctx_cdf(&[12749, 19697, 23806]),
                    multi_ctx_cdf(&[8580, 15297, 20346]),
                    multi_ctx_cdf(&[6169, 11749, 16543]),
                    multi_ctx_cdf(&[4836, 9391, 13448]),
                    multi_ctx_cdf(&[3821, 7711, 11613]),
                    multi_ctx_cdf(&[2228, 4601, 7070]),
                    multi_ctx_cdf(&[16319, 24725, 28280]),
                    multi_ctx_cdf(&[15698, 23277, 27168]),
                    multi_ctx_cdf(&[12726, 20368, 25047]),
                    multi_ctx_cdf(&[9912, 17015, 21976]),
                    multi_ctx_cdf(&[7888, 14220, 19179]),
                    multi_ctx_cdf(&[6777, 12284, 17018]),
                    multi_ctx_cdf(&[4492, 8590, 12252]),
                    multi_ctx_cdf(&[23249, 28904, 30947]),
                    multi_ctx_cdf(&[21050, 27908, 30512]),
                    multi_ctx_cdf(&[17440, 25340, 28949]),
                    multi_ctx_cdf(&[14059, 22018, 26541]),
                    multi_ctx_cdf(&[11288, 18903, 23898]),
                    multi_ctx_cdf(&[9411, 16342, 21428]),
                    multi_ctx_cdf(&[6278, 11588, 15944]),
                ],
            ],
            _ => unreachable!(),
        };

        // dc_sign: uniform (no real reason to bias this).
        // dc_sign: real spec/rav1d default CDFs, indexed [ctx 0..=2] (chroma=0 fixed -- see
        // `dc_sign_cdf`'s doc). Source: rav1d `coef.dc_sign`, real per-frame qindex-bucket
        // selection.
        let dc_sign_cdf = match qcat.min(3) {
            0 => [16000, 13056, 18816].map(binary_ctx_cdf),
            1 => [16000, 13056, 18816].map(binary_ctx_cdf),
            2 => [16000, 13056, 18816].map(binary_ctx_cdf),
            3 => [16000, 13056, 18816].map(binary_ctx_cdf),
            _ => unreachable!(),
        };

        // Reference-frame selection: real spec/rav1d default probabilities (`memorysafety/rav1d`,
        // BSD-2-Clause, `src/cdf.rs`'s `comp`/`comp_dir`/`r#ref`/`comp_fwd_ref`/`comp_bwd_ref`/
        // `comp_uni_ref` fields), one raw prob per context -- same `32768 - p` transform as
        // `skip_cdf` (see that field's doc), applied per-context via `binary_ctx_cdf`. Context
        // derivation: `crate::tile::TileContext`'s `comp_mode_context`/`comp_ref_type_context`/
        // `single_ref_p*_context`/`uni_comp_ref_p1_context` (mirrors rav1d's `get_comp_ctx`/
        // `get_comp_dir_ctx`/`av1_get_*_ctx`/`av1_get_uni_p1_ctx`, `src/env.rs`).
        let comp_mode_cdf = [26828, 24035, 12031, 10640, 2901].map(binary_ctx_cdf);
        let comp_ref_type_cdf = [1198, 2070, 9166, 7499, 22475].map(binary_ctx_cdf);
        let single_ref_p1_cdf = [4897, 16973, 29744].map(binary_ctx_cdf);
        let single_ref_p2_cdf = [1555, 16751, 30279].map(binary_ctx_cdf);
        let single_ref_p3_cdf = [4236, 19647, 31194].map(binary_ctx_cdf);
        let single_ref_p4_cdf = [8650, 24773, 31895].map(binary_ctx_cdf);
        let single_ref_p5_cdf = [904, 11014, 26875].map(binary_ctx_cdf);
        let single_ref_p6_cdf = [1444, 15087, 30304].map(binary_ctx_cdf);
        let comp_ref_cdf = [4946, 19891, 30731].map(binary_ctx_cdf);
        let comp_ref_p1_cdf = [9468, 22441, 31059].map(binary_ctx_cdf);
        let comp_ref_p2_cdf = [1503, 15160, 27544].map(binary_ctx_cdf);
        let comp_bwdref_cdf = [2235, 17182, 30606].map(binary_ctx_cdf);
        let comp_bwdref_p1_cdf = [1423, 15175, 30489].map(binary_ctx_cdf);
        let uni_comp_ref_cdf = [5284, 23152, 31774].map(binary_ctx_cdf);
        let uni_comp_ref_p1_cdf = [3865, 14173, 25120].map(binary_ctx_cdf);
        let uni_comp_ref_p2_cdf = [3128, 15270, 26710].map(binary_ctx_cdf);

        // use_intrabc: rare (screen-content-coding only), heavily biased toward false.
        let use_intrabc_cdf = binary_cdf(0.97);

        Self {
            partition_cdfs,
            skip_cdf,
            skip_mode_cdf,
            intra_cdf,
            y_mode_cdf,
            motion_mode_cdf,
            obmc_cdf,
            interintra_cdf,
            interintra_mode_cdf,
            interintra_wedge_cdf,
            wedge_comp_cdf,
            wedge_idx_cdf,
            mask_comp_cdf,
            jnt_comp_cdf,
            filter_cdf,
            drl_bit_cdf,
            seg_pred_cdf,
            seg_id_cdf,
            pal_y_cdf,
            pal_uv_cdf,
            pal_sz_cdf,
            color_map_cdf,
            angle_delta_cdf,
            uv_mode_cdf,
            cfl_sign_cdf,
            cfl_alpha_cdf,
            use_filter_intra_cdf,
            filter_intra_mode_cdf,
            txpart_cdf,
            kfym,
            newmv_mode_cdf,
            globalmv_mode_cdf,
            refmv_mode_cdf,
            compound_mode_cdf,
            mv_joint_cdf,
            mv_sign_cdf,
            mv_class_cdf,
            mv_bit_cdf,
            delta_q_cdf,
            delta_lf_cdf,
            txb_skip_cdf,
            coeff_base_cdf,
            coeff_br_cdf,
            dc_sign_cdf,
            eob_bin_16_cdf,
            eob_bin_32_cdf,
            eob_bin_64_cdf,
            eob_bin_128_cdf,
            eob_bin_256_cdf,
            eob_bin_512_cdf,
            eob_bin_1024_cdf,
            eob_hi_bit_cdf,
            coeff_base_eob_cdf,
            txb_skip_cdf_chroma,
            dc_sign_cdf_chroma,
            eob_bin_16_cdf_chroma,
            eob_bin_32_cdf_chroma,
            eob_bin_64_cdf_chroma,
            eob_bin_128_cdf_chroma,
            eob_bin_256_cdf_chroma,
            eob_bin_512_cdf_chroma,
            eob_bin_1024_cdf_chroma,
            eob_hi_bit_cdf_chroma,
            coeff_base_eob_cdf_chroma,
            coeff_base_cdf_chroma,
            coeff_br_cdf_chroma,
            txtp_intra1_cdf,
            txtp_intra2_cdf,
            txtp_inter1_cdf,
            txtp_inter2_cdf,
            txtp_inter3_cdf,
            txsz_cdf,
            comp_mode_cdf,
            single_ref_p1_cdf,
            single_ref_p2_cdf,
            single_ref_p3_cdf,
            single_ref_p4_cdf,
            single_ref_p5_cdf,
            single_ref_p6_cdf,
            comp_ref_type_cdf,
            uni_comp_ref_cdf,
            uni_comp_ref_p1_cdf,
            uni_comp_ref_p2_cdf,
            comp_ref_cdf,
            comp_ref_p1_cdf,
            comp_ref_p2_cdf,
            comp_bwdref_cdf,
            comp_bwdref_p1_cdf,
            use_intrabc_cdf,
        }
    }

    /// Get mutable `partition` CDF for `(block_size_log2, context)` -- mutable because
    /// `read_partition` adapts it in place via `update_cdf` after every read (see
    /// `partition_cdfs`'s doc: this is real context, not a representative placeholder).
    ///
    /// Block size is log2 of actual size:
    /// - 2 → 4x4
    /// - 3 → 8x8
    /// - 4 → 16x16
    /// - 5 → 32x32
    /// - 6 → 64x64
    /// - 7 → 128x128
    ///
    /// Context is 0..=3, from `crate::tile::TileContext::partition_context`.
    pub fn get_partition_cdf_mut(&mut self, block_size_log2: u8, ctx: u8) -> &mut [u16] {
        let index = (block_size_log2 as usize)
            .saturating_sub(2)
            .min(self.partition_cdfs.len() - 1);
        self.partition_cdfs[index][(ctx as usize).min(3)].as_mut_slice()
    }

    /// Get mutable skip flag CDF for the given context (0..=2, from
    /// `TileContext::skip_context`) -- mutable because `read_skip` adapts it in place via
    /// `update_cdf` after every read (see `skip_cdf`'s doc: this is real context, not a
    /// representative placeholder).
    pub fn get_skip_cdf_mut(&mut self, ctx: u8) -> &mut [u16] {
        &mut self.skip_cdf[(ctx as usize).min(2)]
    }

    /// Get mutable `skip_mode` CDF for the given context (0..=2, from
    /// `TileContext::skip_mode_context`).
    pub fn get_skip_mode_cdf_mut(&mut self, ctx: u8) -> &mut [u16] {
        &mut self.skip_mode_cdf[(ctx as usize).min(2)]
    }

    /// Get mutable `is_inter` CDF for the given context (0..=3, from `TileContext::intra_ctx`).
    pub fn get_intra_cdf_mut(&mut self, ctx: u8) -> &mut [u16] {
        &mut self.intra_cdf[(ctx as usize).min(3)]
    }

    /// Get mutable non-key-frame `y_mode` CDF for the given block-size-class context (0..=3, from
    /// `crate::tile::coding_unit::y_mode_size_context`).
    pub fn get_y_mode_cdf_mut(&mut self, ctx: u8) -> &mut [u16] {
        &mut self.y_mode_cdf[(ctx as usize).min(3)]
    }

    /// Get mutable `motion_mode` CDF for the given exact-block-size index (0..=16, from
    /// `motion_mode_size_index`).
    pub fn get_motion_mode_cdf_mut(&mut self, idx: u8) -> &mut [u16] {
        &mut self.motion_mode_cdf[(idx as usize).min(16)]
    }

    /// Get mutable `obmc` CDF for the given exact-block-size index (same indexing as
    /// `get_motion_mode_cdf_mut`).
    pub fn get_obmc_cdf_mut(&mut self, idx: u8) -> &mut [u16] {
        &mut self.obmc_cdf[(idx as usize).min(16)]
    }

    /// Get mutable `interintra` CDF for the given block-size-class context (0..=3, from
    /// `crate::tile::coding_unit::y_mode_size_context`).
    pub fn get_interintra_cdf_mut(&mut self, ctx: u8) -> &mut [u16] {
        &mut self.interintra_cdf[(ctx as usize).min(3)]
    }

    /// Get mutable `interintra_mode` CDF for the given context (same indexing as
    /// `get_interintra_cdf_mut`).
    pub fn get_interintra_mode_cdf_mut(&mut self, ctx: u8) -> &mut [u16] {
        &mut self.interintra_mode_cdf[(ctx as usize).min(3)]
    }

    /// Get mutable `interintra_wedge` CDF for the given wedge context (0..=6, from `wedge_ctx`
    /// capped to interintra's real 7-size-subset -- see that field's doc).
    pub fn get_interintra_wedge_cdf_mut(&mut self, ctx: u8) -> &mut [u16] {
        &mut self.interintra_wedge_cdf[(ctx as usize).min(6)]
    }

    /// Get mutable `wedge_comp` CDF for the given wedge context (0..=8, from `wedge_ctx`).
    pub fn get_wedge_comp_cdf_mut(&mut self, ctx: u8) -> &mut [u16] {
        &mut self.wedge_comp_cdf[(ctx as usize).min(8)]
    }

    /// Get mutable `wedge_idx` CDF for the given wedge context (0..=8, from `wedge_ctx`) --
    /// shared verbatim between compound wedge and `interintra_wedge`'s own index read.
    pub fn get_wedge_idx_cdf_mut(&mut self, ctx: u8) -> &mut [u16] {
        &mut self.wedge_idx_cdf[(ctx as usize).min(8)]
    }

    /// Get mutable `mask_comp` CDF for the given context (0..=5, from
    /// `TileContext::mask_comp_context`).
    pub fn get_mask_comp_cdf_mut(&mut self, ctx: u8) -> &mut [u16] {
        &mut self.mask_comp_cdf[(ctx as usize).min(5)]
    }

    /// Get mutable `jnt_comp` CDF for the given context (0..=5, from
    /// `TileContext::jnt_comp_context`).
    pub fn get_jnt_comp_cdf_mut(&mut self, ctx: u8) -> &mut [u16] {
        &mut self.jnt_comp_cdf[(ctx as usize).min(5)]
    }

    /// Get mutable `filter` CDF for the given direction (0/1) and context (0..=7, from
    /// `TileContext::filter_context`).
    pub fn get_filter_cdf_mut(&mut self, dir: u8, ctx: u8) -> &mut [u16] {
        &mut self.filter_cdf[(dir as usize).min(1)][(ctx as usize).min(7)]
    }

    /// Get mutable `drl_bit` CDF for the given context (0..=2, from
    /// `crate::tile::context::get_drl_context`).
    pub fn get_drl_bit_cdf_mut(&mut self, ctx: u8) -> &mut [u16] {
        &mut self.drl_bit_cdf[(ctx as usize).min(2)]
    }

    /// Get mutable `seg_pred` CDF for context `ctx` (0..=2 -- `TileContext::seg_pred_context`'s
    /// doc).
    pub fn get_seg_pred_cdf_mut(&mut self, ctx: u8) -> &mut [u16] {
        &mut self.seg_pred_cdf[(ctx as usize).min(2)]
    }

    /// Get mutable `segment_id` CDF for context `ctx` (0..=2 -- `TileContext::
    /// segment_id_context`'s doc).
    pub fn get_seg_id_cdf_mut(&mut self, ctx: u8) -> &mut [u16] {
        &mut self.seg_id_cdf[(ctx as usize).min(2)]
    }

    /// Get mutable `has_palette_y` CDF (`bsize_ctx` 0..=6, `ctx` 0..=2 -- see
    /// `TileContext::has_palette_context`'s doc).
    pub fn get_pal_y_cdf_mut(&mut self, bsize_ctx: u8, ctx: u8) -> &mut [u16] {
        &mut self.pal_y_cdf[(bsize_ctx as usize).min(6)][(ctx as usize).min(2)]
    }

    /// Get mutable `has_palette_uv` CDF (`ctx` 0..=1, `PaletteSizeY > 0`).
    pub fn get_pal_uv_cdf_mut(&mut self, ctx: u8) -> &mut [u16] {
        &mut self.pal_uv_cdf[(ctx as usize).min(1)]
    }

    /// Get mutable `palette_size_{y,uv}_minus_2` CDF (`plane` 0=y/1=uv, `bsize_ctx` 0..=6).
    pub fn get_pal_sz_cdf_mut(&mut self, plane: usize, bsize_ctx: u8) -> &mut [u16] {
        &mut self.pal_sz_cdf[plane.min(1)][(bsize_ctx as usize).min(6)]
    }

    /// Get mutable `color_map` CDF (`plane` 0=y/1=uv, `pal_sz` 2..=8, `ctx` 0..=4 -- see
    /// `SymbolDecoder::read_palette_index_map`'s doc).
    pub fn get_color_map_cdf_mut(&mut self, plane: usize, pal_sz: u8, ctx: u8) -> &mut [u16] {
        let pal_sz_idx = (pal_sz.max(2) - 2).min(6) as usize;
        &mut self.color_map_cdf[plane.min(1)][pal_sz_idx][(ctx as usize).min(4)]
    }

    /// Get mutable `angle_delta` CDF for a directional mode (`mode_minus_vert` 0..=7, i.e.
    /// `mode - V_PRED` -- shared by both `angle_delta_y` and `angle_delta_uv`, see field doc).
    pub fn get_angle_delta_cdf_mut(&mut self, mode_minus_vert: u8) -> &mut [u16] {
        &mut self.angle_delta_cdf[(mode_minus_vert as usize).min(7)]
    }

    /// Get mutable `uv_mode` CDF for `(cfl_allowed, y_mode)` (`y_mode` 0..=12 -- see field doc for
    /// why `cfl_allowed=0`/`1` are genuinely distinct alphabets, not the same table truncated).
    pub fn get_uv_mode_cdf_mut(&mut self, cfl_allowed: bool, y_mode: u8) -> &mut [u16] {
        &mut self.uv_mode_cdf[usize::from(cfl_allowed)][(y_mode as usize).min(12)]
    }

    /// Get mutable `cfl_alpha_signs` CDF (no context, see field doc).
    pub fn get_cfl_sign_cdf_mut(&mut self) -> &mut [u16] {
        &mut self.cfl_sign_cdf
    }

    /// Get mutable `cfl_alpha_u`/`cfl_alpha_v` CDF for the given context (0..=5, see field doc).
    pub fn get_cfl_alpha_cdf_mut(&mut self, ctx: u8) -> &mut [u16] {
        &mut self.cfl_alpha_cdf[(ctx as usize).min(5)]
    }

    /// Get mutable `use_filter_intra` CDF for the given `BlockSize` (see field doc for the
    /// discriminant-order mapping).
    pub fn get_use_filter_intra_cdf_mut(&mut self, bs: crate::tile::BlockSize) -> &mut [u16] {
        &mut self.use_filter_intra_cdf[(bs as usize).min(21)]
    }

    /// Get mutable `filter_intra_mode` CDF (no context, see field doc).
    pub fn get_filter_intra_mode_cdf_mut(&mut self) -> &mut [u16] {
        &mut self.filter_intra_mode_cdf
    }

    /// Get mutable `txfm_split` CDF for `(cat, ctx)` (`cat` 0..=6, `ctx` 0..=2 -- see
    /// `read_var_tx_size`'s doc and `TileContext::var_tx_context`) -- mutable because
    /// `read_txfm_split` adapts it in place via `update_cdf` after every read.
    pub fn get_txpart_cdf_mut(&mut self, cat: u8, ctx: u8) -> &mut [u16] {
        &mut self.txpart_cdf[(cat as usize).min(6)][(ctx as usize).min(2)]
    }

    /// Get mutable key-frame `intra_mode` CDF for the given context (`above_mode_class`,
    /// `left_mode_class`, each 0..=4 -- see `crate::tile::TileContext::intra_mode_context`) --
    /// mutable because `read_intra_mode` adapts it in place via `update_cdf` after every read.
    pub fn get_kfym_cdf_mut(&mut self, above_class: u8, left_class: u8) -> &mut [u16] {
        &mut self.kfym[(above_class as usize).min(4)][(left_class as usize).min(4)]
    }

    /// Get mutable `newmv_mode` CDF for the given context (0..=5, `ctx & 7` of
    /// `crate::tile::TileContext::inter_mode_context`'s packed result).
    pub fn get_newmv_mode_cdf_mut(&mut self, ctx: u8) -> &mut [u16] {
        &mut self.newmv_mode_cdf[(ctx as usize).min(5)]
    }

    /// Get mutable `globalmv_mode` CDF for the given context (0..=1, `ctx >> 3 & 1`).
    pub fn get_globalmv_mode_cdf_mut(&mut self, ctx: u8) -> &mut [u16] {
        &mut self.globalmv_mode_cdf[(ctx as usize).min(1)]
    }

    /// Get mutable `refmv_mode` CDF for the given context (0..=5, `ctx >> 4 & 15`).
    pub fn get_refmv_mode_cdf_mut(&mut self, ctx: u8) -> &mut [u16] {
        &mut self.refmv_mode_cdf[(ctx as usize).min(5)]
    }

    /// Get mutable `compound_mode` CDF for the given context (0..=7, spec 5.11.24, see
    /// `crate::tile::TileContext::compound_mode_context`).
    pub fn get_compound_mode_cdf_mut(&mut self, ctx: u8) -> &mut [u16] {
        &mut self.compound_mode_cdf[(ctx as usize).min(7)]
    }

    /// Get mutable MV joint CDF (4 symbols) -- mutable because `read_mv_joint` adapts it in place
    /// via `update_cdf` after every read (no neighbor context in the real spec either -- MV
    /// adaptation is frame-lifetime-global, unlike `skip`/`kfym`'s above/left context).
    ///
    /// - MV_JOINT_ZERO (both components zero)
    /// - MV_JOINT_HNZVZ (horizontal non-zero, vertical zero)
    /// - MV_JOINT_HZVNZ (horizontal zero, vertical non-zero)
    /// - MV_JOINT_HNZVNZ (both components non-zero)
    pub fn get_mv_joint_cdf_mut(&mut self) -> &mut [u16] {
        &mut self.mv_joint_cdf
    }

    /// Get mutable MV sign CDF (2 symbols: positive, negative) -- adapted after every read.
    pub fn get_mv_sign_cdf_mut(&mut self) -> &mut [u16] {
        &mut self.mv_sign_cdf
    }

    /// Get mutable MV class CDF (12 symbols: magnitude class) -- adapted after every read.
    pub fn get_mv_class_cdf_mut(&mut self) -> &mut [u16] {
        &mut self.mv_class_cdf
    }

    /// Get mutable MV bit CDF (2 symbols: 0, 1) -- adapted after every read.
    pub fn get_mv_bit_cdf_mut(&mut self) -> &mut [u16] {
        &mut self.mv_bit_cdf
    }

    /// Get mutable `delta_q` CDF (spec 5.11.38) -- adapted after every read.
    pub fn get_delta_q_cdf_mut(&mut self) -> &mut [u16] {
        &mut self.delta_q_cdf
    }

    /// Get mutable `delta_lf` CDF for the given real spec index (see
    /// `SymbolDecoder::read_delta_lf`'s doc) -- adapted after every read.
    pub fn get_delta_lf_cdf_mut(&mut self, cdf_index: usize) -> &mut [u16] {
        &mut self.delta_lf_cdf[cdf_index.min(4)]
    }

    /// Get mutable `txb_skip` (all_zero) CDF for one transform block. `tx_size_class`: 0..=4 (see
    /// `tx_size_class`). `ctx`: 0..=6 -- see `SymbolDecoder::read_residual_block`'s doc for when
    /// callers pass a real neighbor-derived value vs. the `0` fallback.
    pub fn get_txb_skip_cdf_mut(&mut self, tx_size_class: usize, ctx: u8) -> &mut [u16] {
        &mut self.txb_skip_cdf[tx_size_class.min(4)][(ctx as usize).min(6)]
    }

    /// Get mutable `eob_bin` CDF for a transform block of `tx_size_px` pixels per side
    /// (4/8/16/32/64) and `is_1d` (`SymbolDecoder::read_transform_type_is_1d`'s result). 64x64
    /// shares the 1024-coefficient class with 32x32 (real AV1 caps the coefficient scan at the
    /// top-left 32x32 sub-area for larger transforms) -- matches `tx_size_class`'s doc. The
    /// 1024-coefficient class has no real `is_1d` axis in rav1d (see `eob_bin_1024_cdf`'s doc),
    /// so `is_1d` is ignored there.
    /// `width_dim`/`height_dim`: transform dims in samples (already 32-capped by the caller, see
    /// `scan::scan_table`'s doc) -- selection is by *total area* (`width_dim*height_dim`), not
    /// either dim alone: real spec/dav1d select `eob_bin`'s default CDF row by total coefficient
    /// count (`16/32/64/128/256/512/1024`), which is symmetric in width/height (a 4x8 and an 8x4
    /// transform share the same row) -- verified against dav1d's `default_coef_cdf`'s
    /// `eob_bin_16/32/64/128/256/512/1024` field names directly, not assumed. `eob_bin_512`/
    /// `_1024` have no real `is_1d` axis (see their field docs).
    pub fn get_eob_bin_cdf_mut(
        &mut self,
        width_dim: u32,
        height_dim: u32,
        is_1d: bool,
    ) -> &mut [u16] {
        let is_1d = usize::from(is_1d);
        match width_dim * height_dim {
            0..=16 => &mut self.eob_bin_16_cdf[is_1d],
            17..=32 => &mut self.eob_bin_32_cdf[is_1d],
            33..=64 => &mut self.eob_bin_64_cdf[is_1d],
            65..=128 => &mut self.eob_bin_128_cdf[is_1d],
            129..=256 => &mut self.eob_bin_256_cdf[is_1d],
            257..=512 => &mut self.eob_bin_512_cdf,
            _ => &mut self.eob_bin_1024_cdf,
        }
    }

    /// Get mutable `eob_hi_bit` CDF -- the single context-coded first bit of `eob`'s extra-bits
    /// suffix (see `eob_hi_bit_cdf`'s doc). `tx_size_class`: 0..=4 (4x4..64x64, see
    /// `tx_size_class`). `eob_bin`: the symbol just read from `get_eob_bin_cdf_mut`'s CDF (0..=10).
    pub fn get_eob_hi_bit_cdf_mut(&mut self, tx_size_class: usize, eob_bin: u8) -> &mut [u16] {
        &mut self.eob_hi_bit_cdf[tx_size_class.min(4)][(eob_bin as usize).min(10)]
    }

    /// Get mutable `coeff_base_eob` CDF (level 1..=3 for the highest-scan-order nonzero
    /// coefficient), real context (`tx_size_class` 0..=4, `ctx` 0..=3 -- see
    /// `SymbolDecoder::coeff_base_eob_context`'s doc).
    pub fn get_coeff_base_eob_cdf_mut(&mut self, tx_size_class: usize, ctx: u8) -> &mut [u16] {
        &mut self.coeff_base_eob_cdf[tx_size_class.min(4)][(ctx as usize).min(3)]
    }

    /// Get mutable chroma `txb_skip` CDF -- see `txb_skip_cdf_chroma`'s doc for scope
    /// (`tx_size_class` 0..=3, `ctx` 0..=5 -- see `TileContext::txb_skip_context_chroma`'s doc).
    pub fn get_txb_skip_cdf_chroma_mut(&mut self, tx_size_class: usize, ctx: u8) -> &mut [u16] {
        &mut self.txb_skip_cdf_chroma[tx_size_class.min(3)][(ctx as usize).min(5)]
    }

    /// Get mutable chroma `dc_sign` CDF -- real 3-context chroma default (see
    /// `dc_sign_cdf_chroma`'s doc), `ctx` 0..=2 (`TileContext::dc_sign_context_chroma`'s doc).
    pub fn get_dc_sign_cdf_chroma_mut(&mut self, ctx: u8) -> &mut [u16] {
        &mut self.dc_sign_cdf_chroma[(ctx as usize).min(2)]
    }

    /// Get mutable chroma `eob_bin` CDF for a `width_px`x`height_px` chroma transform tile (each
    /// independently `<=32`, `txb_skip_cdf_chroma`'s doc) -- selection by *total area*, same
    /// symmetric-in-width/height reasoning as `get_eob_bin_cdf_mut`'s doc. `is_1d` is always
    /// `false` for chroma in this crate's scope, so there's no axis for it (unlike
    /// `get_eob_bin_cdf_mut`).
    pub fn get_eob_bin_cdf_chroma_mut(&mut self, width_px: u32, height_px: u32) -> &mut [u16] {
        match width_px * height_px {
            0..=16 => &mut self.eob_bin_16_cdf_chroma,
            17..=32 => &mut self.eob_bin_32_cdf_chroma,
            33..=64 => &mut self.eob_bin_64_cdf_chroma,
            65..=128 => &mut self.eob_bin_128_cdf_chroma,
            129..=256 => &mut self.eob_bin_256_cdf_chroma,
            257..=512 => &mut self.eob_bin_512_cdf_chroma,
            _ => &mut self.eob_bin_1024_cdf_chroma, // 1024, this crate's chroma scope max
        }
    }

    /// Get mutable chroma `eob_hi_bit` CDF. `tx_size_class`: 0..=3. `eob_bin`: same real,
    /// dynamic indexing as luma's `get_eob_hi_bit_cdf_mut`.
    pub fn get_eob_hi_bit_cdf_chroma_mut(
        &mut self,
        tx_size_class: usize,
        eob_bin: u8,
    ) -> &mut [u16] {
        &mut self.eob_hi_bit_cdf_chroma[tx_size_class.min(3)][(eob_bin as usize).min(10)]
    }

    /// Get mutable chroma `coeff_base_eob` CDF. `tx_size_class`: 0..=3. `ctx`: same real formula
    /// as luma's (`SymbolDecoder::coeff_base_eob_context`, plane-agnostic).
    pub fn get_coeff_base_eob_cdf_chroma_mut(
        &mut self,
        tx_size_class: usize,
        ctx: u8,
    ) -> &mut [u16] {
        &mut self.coeff_base_eob_cdf_chroma[tx_size_class.min(3)][(ctx as usize).min(3)]
    }

    /// Get mutable chroma `coeff_base` CDF. `tx_size_class`: 0..=3. `ctx`: 0..=40, from
    /// `symbol::scan::lo_ctx` (same real formula as luma's, plane-agnostic -- only the default
    /// CDF values differ).
    pub fn get_coeff_base_cdf_chroma_mut(&mut self, tx_size_class: usize, ctx: u8) -> &mut [u16] {
        &mut self.coeff_base_cdf_chroma[tx_size_class.min(3)][(ctx as usize).min(40)]
    }

    /// Get mutable chroma `coeff_br` CDF. `tx_size_class`: 0..=3 (matches `coeff_base_cdf_chroma`
    /// exactly here, unlike luma's `min(tx_size_class,3)` cap against a 5-bucket table -- this
    /// crate's chroma table only ever has 4 buckets, 0..=3 is already the whole range). `ctx`:
    /// 0..=20.
    pub fn get_coeff_br_cdf_chroma_mut(&mut self, tx_size_class: usize, ctx: u8) -> &mut [u16] {
        &mut self.coeff_br_cdf_chroma[tx_size_class.min(3)][(ctx as usize).min(20)]
    }

    /// Get mutable `txtp_intra2` CDF (spec 5.11.47's reduced intra `transform_type()` alphabet).
    /// `tx_size_class`: only 0..=2 (4x4/8x8/16x16) are ever reached -- see
    /// `SymbolDecoder::read_transform_type_is_1d`'s doc.
    pub fn get_txtp_intra2_cdf_mut(&mut self, tx_size_class: usize, y_mode: u8) -> &mut [u16] {
        &mut self.txtp_intra2_cdf[tx_size_class.min(2)][(y_mode as usize).min(12)]
    }

    /// Get mutable `txtp_intra1` CDF (spec 5.11.47's full intra `transform_type()` alphabet).
    /// `tx_size_class`: only 0..=1 (4x4/8x8) are ever reached.
    pub fn get_txtp_intra1_cdf_mut(&mut self, tx_size_class: usize, y_mode: u8) -> &mut [u16] {
        &mut self.txtp_intra1_cdf[tx_size_class.min(1)][(y_mode as usize).min(12)]
    }

    /// Get mutable `txtp_inter3` CDF (spec 5.11.47's reduced/32x32 inter `transform_type()`
    /// alphabet). `tx_size_class`: only 0..=3 (4x4/8x8/16x16/32x32) are ever reached.
    pub fn get_txtp_inter3_cdf_mut(&mut self, tx_size_class: usize) -> &mut [u16] {
        &mut self.txtp_inter3_cdf[tx_size_class.min(3)]
    }

    /// Get mutable `txtp_inter2` CDF (spec 5.11.47's 16x16 inter `transform_type()` alphabet --
    /// only ever reached at that one tx size, so no context axis).
    pub fn get_txtp_inter2_cdf_mut(&mut self) -> &mut [u16] {
        &mut self.txtp_inter2_cdf
    }

    /// Get mutable `txtp_inter1` CDF (spec 5.11.47's full inter `transform_type()` alphabet).
    /// `tx_size_class`: only 0..=1 (4x4/8x8) are ever reached.
    pub fn get_txtp_inter1_cdf_mut(&mut self, tx_size_class: usize) -> &mut [u16] {
        &mut self.txtp_inter1_cdf[tx_size_class.min(1)]
    }

    /// Get mutable `tx_size()` depth CDF. `max_tx_class`: 1..=4 (4x4/class 0 never reaches this,
    /// see `SymbolDecoder::read_tx_size`'s doc). `ctx`: 0..=2, see
    /// `crate::tile::TileContext::tx_size_context`.
    pub fn get_txsz_cdf_mut(&mut self, max_tx_class: usize, ctx: u8) -> &mut [u16] {
        &mut self.txsz_cdf[max_tx_class.clamp(1, 4) - 1][(ctx as usize).min(2)]
    }

    /// Get mutable `coeff_base` CDF (level 0..=3 for every other coefficient position).
    /// `tx_size_class`: 0..=4 (see `tx_size_class`). `ctx`: 0..=40, from `symbol::scan::lo_ctx`.
    pub fn get_coeff_base_cdf_mut(&mut self, tx_size_class: usize, ctx: u8) -> &mut [u16] {
        &mut self.coeff_base_cdf[tx_size_class.min(4)][(ctx as usize).min(40)]
    }

    /// Get mutable `coeff_br` CDF (range-extension increment 0..=3). `capped_tx_size_class`:
    /// 0..=3 (`tx_size_class(..).min(3)`, one fewer bucket than `coeff_base_cdf` -- see this
    /// field's doc). `ctx`: 0..=20.
    pub fn get_coeff_br_cdf_mut(&mut self, capped_tx_size_class: usize, ctx: u8) -> &mut [u16] {
        &mut self.coeff_br_cdf[capped_tx_size_class.min(3)][(ctx as usize).min(20)]
    }

    /// Get mutable `dc_sign` CDF (sign of the DC coefficient). `ctx`: 0..=2 -- see
    /// `SymbolDecoder::read_residual_block`'s doc for when callers pass a real neighbor-derived
    /// value vs. the `0` fallback.
    pub fn get_dc_sign_cdf_mut(&mut self, ctx: u8) -> &mut [u16] {
        &mut self.dc_sign_cdf[(ctx as usize).min(2)]
    }

    /// Get `comp_mode` CDF (single-reference vs compound prediction), real context (0..=4, see
    /// `crate::tile::TileContext::comp_mode_context`) + adaptation, like `get_skip_cdf_mut`.
    pub fn get_comp_mode_cdf_mut(&mut self, ctx: u8) -> &mut [u16] {
        &mut self.comp_mode_cdf[(ctx as usize).min(4)]
    }
    /// Get `single_ref_p1` CDF (forward vs backward reference group), real context (0..=2, see
    /// `crate::tile::TileContext::single_ref_p1_context`) + adaptation.
    pub fn get_single_ref_p1_cdf_mut(&mut self, ctx: u8) -> &mut [u16] {
        &mut self.single_ref_p1_cdf[(ctx as usize).min(2)]
    }
    /// Get `single_ref_p2` CDF (BWDREF/ALTREF2 group vs ALTREF, backward branch), real context.
    pub fn get_single_ref_p2_cdf_mut(&mut self, ctx: u8) -> &mut [u16] {
        &mut self.single_ref_p2_cdf[(ctx as usize).min(2)]
    }
    /// Get `single_ref_p3` CDF (LAST/LAST2 group vs LAST3/GOLDEN group, forward branch), real
    /// context.
    pub fn get_single_ref_p3_cdf_mut(&mut self, ctx: u8) -> &mut [u16] {
        &mut self.single_ref_p3_cdf[(ctx as usize).min(2)]
    }
    /// Get `single_ref_p4` CDF (LAST vs LAST2), real context.
    pub fn get_single_ref_p4_cdf_mut(&mut self, ctx: u8) -> &mut [u16] {
        &mut self.single_ref_p4_cdf[(ctx as usize).min(2)]
    }
    /// Get `single_ref_p5` CDF (LAST3 vs GOLDEN), real context.
    pub fn get_single_ref_p5_cdf_mut(&mut self, ctx: u8) -> &mut [u16] {
        &mut self.single_ref_p5_cdf[(ctx as usize).min(2)]
    }
    /// Get `single_ref_p6` CDF (BWDREF vs ALTREF2), real context.
    pub fn get_single_ref_p6_cdf_mut(&mut self, ctx: u8) -> &mut [u16] {
        &mut self.single_ref_p6_cdf[(ctx as usize).min(2)]
    }
    /// Get `comp_ref_type` CDF (unidirectional vs bidirectional compound reference), real context
    /// (0..=4, see `crate::tile::TileContext::comp_ref_type_context`).
    pub fn get_comp_ref_type_cdf_mut(&mut self, ctx: u8) -> &mut [u16] {
        &mut self.comp_ref_type_cdf[(ctx as usize).min(4)]
    }
    /// Get `uni_comp_ref` CDF ((LAST,LAST2)/(LAST,LAST3-or-GOLDEN) pair vs (BWDREF,ALTREF)), real
    /// context (reuses `single_ref_p1_context`'s formula per rav1d, see `read_ref_frames`).
    pub fn get_uni_comp_ref_cdf_mut(&mut self, ctx: u8) -> &mut [u16] {
        &mut self.uni_comp_ref_cdf[(ctx as usize).min(2)]
    }
    /// Get `uni_comp_ref_p1` CDF ((LAST,LAST2) vs (LAST,LAST3-or-GOLDEN)), real context (see
    /// `crate::tile::TileContext::uni_comp_ref_p1_context`).
    pub fn get_uni_comp_ref_p1_cdf_mut(&mut self, ctx: u8) -> &mut [u16] {
        &mut self.uni_comp_ref_p1_cdf[(ctx as usize).min(2)]
    }
    /// Get `uni_comp_ref_p2` CDF (LAST3 vs GOLDEN, second slot of the unidirectional pair), real
    /// context (reuses `single_ref_p5_context`'s formula per rav1d).
    pub fn get_uni_comp_ref_p2_cdf_mut(&mut self, ctx: u8) -> &mut [u16] {
        &mut self.uni_comp_ref_p2_cdf[(ctx as usize).min(2)]
    }
    /// Get `comp_ref` CDF (forward group choice, bidirectional compound), real context (reuses
    /// `single_ref_p3_context`'s formula per rav1d).
    pub fn get_comp_ref_cdf_mut(&mut self, ctx: u8) -> &mut [u16] {
        &mut self.comp_ref_cdf[(ctx as usize).min(2)]
    }
    /// Get `comp_ref_p1` CDF (LAST vs LAST2, bidirectional compound forward ref), real context
    /// (reuses `single_ref_p4_context`'s formula per rav1d).
    pub fn get_comp_ref_p1_cdf_mut(&mut self, ctx: u8) -> &mut [u16] {
        &mut self.comp_ref_p1_cdf[(ctx as usize).min(2)]
    }
    /// Get `comp_ref_p2` CDF (LAST3 vs GOLDEN, bidirectional compound forward ref), real context
    /// (reuses `single_ref_p5_context`'s formula per rav1d).
    pub fn get_comp_ref_p2_cdf_mut(&mut self, ctx: u8) -> &mut [u16] {
        &mut self.comp_ref_p2_cdf[(ctx as usize).min(2)]
    }
    /// Get `comp_bwdref` CDF (backward group choice, bidirectional compound), real context
    /// (reuses `single_ref_p2_context`'s formula per rav1d).
    pub fn get_comp_bwdref_cdf_mut(&mut self, ctx: u8) -> &mut [u16] {
        &mut self.comp_bwdref_cdf[(ctx as usize).min(2)]
    }
    /// Get `comp_bwdref_p1` CDF (BWDREF vs ALTREF2, bidirectional compound backward ref), real
    /// context (reuses `single_ref_p6_context`'s formula per rav1d).
    pub fn get_comp_bwdref_p1_cdf_mut(&mut self, ctx: u8) -> &mut [u16] {
        &mut self.comp_bwdref_p1_cdf[(ctx as usize).min(2)]
    }
    /// Get `use_intrabc` CDF (intra block copy flag).
    pub fn get_use_intrabc_cdf(&self) -> &[u16] {
        &self.use_intrabc_cdf
    }
}

impl Default for CdfContext {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_partition_cdf_uniform() {
        let cdf = PartitionCdf::uniform(4);
        assert_eq!(cdf.num_symbols, 4);
        assert_eq!(cdf.cdf.len(), 5);
        assert_eq!(cdf.cdf[0], 0);
        assert_eq!(cdf.cdf[4], CDF_SCALE);

        // Check uniform distribution
        assert_eq!(cdf.cdf[1], 8192); // 1/4
        assert_eq!(cdf.cdf[2], 16384); // 2/4
        assert_eq!(cdf.cdf[3], 24576); // 3/4
    }

    #[test]
    fn test_partition_cdf_biased() {
        let cdf = PartitionCdf::biased_none(4);
        assert_eq!(cdf.num_symbols, 4);
        assert_eq!(cdf.cdf[0], 0);
        assert_eq!(cdf.cdf[4], CDF_SCALE);

        // NONE should have highest probability
        assert!(cdf.cdf[1] > cdf.cdf[2] - cdf.cdf[1]);
    }

    #[test]
    fn test_cdf_context_creation() {
        let context = CdfContext::new();

        // Check we have CDFs for all block sizes
        assert_eq!(context.partition_cdfs.len(), 6); // log2(4) to log2(128)
    }

    #[test]
    fn test_cdf_context_get_partition() {
        let mut context = CdfContext::new();

        // 4x4 block (log2 = 2): trivial 1-symbol placeholder, never actually read
        let cdf_4x4 = context.get_partition_cdf_mut(2, 0);
        assert_eq!(cdf_4x4.len(), 2); // 1 symbol + count slot

        // 8x8 block (log2 = 3): 4 real symbols (NONE/HORZ/VERT/SPLIT only)
        let cdf_8x8 = context.get_partition_cdf_mut(3, 0);
        assert_eq!(cdf_8x8.len(), 5); // 4 symbols + count slot

        // 16x16 block (log2 = 4): 10 real symbols
        let cdf_16x16 = context.get_partition_cdf_mut(4, 0);
        assert_eq!(cdf_16x16.len(), 11); // 10 symbols + count slot

        // 128x128 block (log2 = 7): only 8 real symbols (no HORZ_4/VERT_4)
        let cdf_128x128 = context.get_partition_cdf_mut(7, 0);
        assert_eq!(cdf_128x128.len(), 9); // 8 symbols + count slot

        // Every context (0..=3) is independently addressable.
        for ctx in 0..4 {
            let cdf = context.get_partition_cdf_mut(4, ctx);
            assert_eq!(
                cdf.len(),
                11,
                "context {ctx} should have the same alphabet size"
            );
        }
    }

    #[test]
    fn test_cdf_scale() {
        assert_eq!(CDF_SCALE, 32768);
        assert_eq!(CDF_SCALE, 1 << 15);
    }

    #[test]
    fn test_split_or_horz_vert_prob_8x8_no_tail_term() {
        // 8x8/ctx0 real literal: [13636, 7258, 2376, 0, 0] (indices 4..8 don't exist for this
        // 4-symbol alphabet -- HorzA/HorzB/Horz4/VertA/VertB all treated as 0).
        let mut context = CdfContext::new();
        let cdf = context.get_partition_cdf_mut(3, 0);
        assert_eq!(split_or_horz_prob(cdf), 7258); // cdf[1] - 0 + 0
        assert_eq!(split_or_vert_prob(cdf), 8754); // cdf[0] - cdf[1] + cdf[2] - 0
    }

    #[test]
    fn test_split_or_horz_vert_prob_16x16_includes_tail_term() {
        // 16x16/ctx0 real literal (10-symbol alphabet, has Horz4/Vert4):
        // [17171, 11839, 8197, 6062, 5104, 3947, 3167, 2197, 866, 0, 0]
        let mut context = CdfContext::new();
        let cdf = context.get_partition_cdf_mut(4, 0);
        assert_eq!(split_or_horz_prob(cdf), 9351); // 11839 - 5104 + 3947 + (866 - 2197)
        assert_eq!(split_or_vert_prob(cdf), 11693); // 17171-11839+8197-3167 + (2197-866)
    }

    #[test]
    fn test_split_or_horz_vert_prob_128x128_omits_tail_term() {
        // 128x128/ctx0 real literal (8-symbol alphabet, no Horz4/Vert4 at all -- unlike 8x8, the
        // real VertB/HorzB entries here are nonzero, so an unguarded read would wrongly include
        // them in the tail term; the explicit `cdf.len() >= 11` check must skip it instead of
        // relying on padding-as-zero (see `split_or_horz_prob`'s doc).
        let mut context = CdfContext::new();
        let cdf = context.get_partition_cdf_mut(7, 0);
        assert_eq!(cdf, &[4869, 4549, 4239, 284, 229, 149, 129, 0, 0]);
        assert_eq!(split_or_horz_prob(cdf), 4549 - 229 + 149); // no tail term
        assert_eq!(split_or_vert_prob(cdf), 4869 - 4549 + 4239 - 129); // no tail term
    }

    #[test]
    fn test_mv_class_cdf_spec_compliant() {
        let mut context = CdfContext::new();
        let cdf = context.get_mv_class_cdf_mut();

        // Verify length: 11 classes + adaptation count = 12 values
        assert_eq!(cdf.len(), 12);

        // Real spec/rav1d descending convention (see `to_descending`'s doc): last real entry
        // (index 10) is 0, and the trailing count slot starts at 0.
        assert_eq!(cdf[10], 0, "last real entry should be 0");
        assert_eq!(cdf[11], 0, "adaptation count should start at 0");

        // Verify monotonically non-increasing
        for i in 1..cdf.len() - 1 {
            assert!(
                cdf[i] <= cdf[i - 1],
                "CDF should be monotonically non-increasing at index {}",
                i
            );
        }

        // Verify spec-compliant values from rav1d (converted from the ascending
        // rav1d-sourced counts via `d[i] = CDF_SCALE - ascending[i+1]`; see `mv_class_counts`).
        assert_eq!(cdf[0], 4096, "Class 0 (0 qpel) descending threshold");
        assert_eq!(cdf[1], 1792, "Class 1 (±1 qpel) descending threshold");
        assert_eq!(cdf[2], 910, "Class 2 (±2-3 qpel) descending threshold");
        assert_eq!(cdf[3], 448, "Class 3 (±4-7 qpel) descending threshold");

        // Verify realistic distribution (most MVs are small magnitude): class 0's interval width
        // is CDF_SCALE - cdf[0] (the "u - v" width for val=0, since u starts at the full range).
        let prob_class_0 = (CDF_SCALE - cdf[0]) as f32 / CDF_SCALE as f32;
        assert!(
            prob_class_0 > 0.85,
            "Class 0 should be very common (>85%), got {:.1}%",
            prob_class_0 * 100.0
        );
    }

    #[test]
    fn test_mv_joint_cdf_spec_compliant() {
        let mut context = CdfContext::new();
        let cdf = context.get_mv_joint_cdf_mut();

        // Verify length: 4 symbols + adaptation count = 5 values
        assert_eq!(cdf.len(), 5);

        assert_eq!(cdf[3], 0, "last real entry should be 0");
        assert_eq!(cdf[4], 0, "adaptation count should start at 0");

        for i in 1..cdf.len() - 1 {
            assert!(
                cdf[i] <= cdf[i - 1],
                "CDF should be monotonically non-increasing at index {}",
                i
            );
        }

        // Verify spec-compliant values from rav1d (same conversion as mv_class above).
        assert_eq!(cdf[0], 28672, "MV_JOINT_ZERO descending threshold");
        assert_eq!(cdf[1], 21504, "MV_JOINT_HNZVZ descending threshold");
        assert_eq!(cdf[2], 13440, "MV_JOINT_HZVNZ descending threshold");
    }

    #[test]
    fn test_mv_sign_cdf() {
        let mut context = CdfContext::new();
        let cdf = context.get_mv_sign_cdf_mut();

        // Verify length: 2 symbols + adaptation count = 3 values
        assert_eq!(cdf.len(), 3);

        // Verify uniform distribution (50/50): descending threshold at the midpoint.
        assert_eq!(cdf[0], 16384, "Sign should be 50/50");
        assert_eq!(cdf[1], 0, "last real entry should be 0");
        assert_eq!(cdf[2], 0, "adaptation count should start at 0");
    }

    #[test]
    fn test_mv_bit_cdf() {
        let mut context = CdfContext::new();
        let cdf = context.get_mv_bit_cdf_mut();

        // Verify length: 2 symbols + adaptation count = 3 values
        assert_eq!(cdf.len(), 3);

        // Verify uniform distribution (50/50): descending threshold at the midpoint.
        assert_eq!(cdf[0], 16384, "Bit should be 50/50");
        assert_eq!(cdf[1], 0, "last real entry should be 0");
        assert_eq!(cdf[2], 0, "adaptation count should start at 0");
    }

    #[test]
    fn test_to_descending_round_trips_ascending_shape() {
        // A well-formed ascending CDF (cdf[0]=0..cdf[n]=CDF_SCALE) should convert into a
        // well-formed descending one (monotonically non-increasing, last real entry 0, trailing
        // count-slot 0) with the same length.
        let ascending = vec![0u16, 8192, 16384, 24576, CDF_SCALE];
        let descending = to_descending(&ascending);

        assert_eq!(descending.len(), ascending.len());
        assert_eq!(descending, vec![24576, 16384, 8192, 0, 0]);
        for i in 1..descending.len() - 1 {
            assert!(descending[i] <= descending[i - 1]);
        }
    }

    /// The real dav1d/spec qindex-bucket formula (mirrored at both real production call sites --
    /// `overlay_extraction::cu_parser::parse_all_coding_units_with_temporal` and
    /// `overlay_extraction::partition::parse_partition_trees_from_tile_data`) is
    /// `qcat = (base_q_idx>20) + (base_q_idx>60) + (base_q_idx>120)`. Confirms the boundary
    /// values land on the correct side (dav1d's `>`, not `>=`, so `base_q_idx == 20/60/120`
    /// itself stays in the *lower* bucket) -- an off-by-one here would silently apply the wrong
    /// bucket to every frame whose QP lands exactly on a boundary.
    #[test]
    fn qcat_formula_boundaries_match_real_dav1d_thresholds() {
        fn qcat_of(base_q_idx: i16) -> u8 {
            (base_q_idx > 20) as u8 + (base_q_idx > 60) as u8 + (base_q_idx > 120) as u8
        }

        assert_eq!(qcat_of(0), 0);
        assert_eq!(
            qcat_of(20),
            0,
            "== 20 must stay in bucket 0 (dav1d uses > not >=)"
        );
        assert_eq!(qcat_of(21), 1, "21 is the first value in bucket 1");
        assert_eq!(qcat_of(60), 1, "== 60 must stay in bucket 1");
        assert_eq!(qcat_of(61), 2, "61 is the first value in bucket 2");
        assert_eq!(qcat_of(120), 2, "== 120 must stay in bucket 2");
        assert_eq!(qcat_of(121), 3, "121 is the first value in bucket 3");
        assert_eq!(qcat_of(255), 3, "max real base_q_idx stays in bucket 3");
    }

    /// Non-degenerate check that `new_with_qcat`'s 4 match arms actually hold 4 *different* sets
    /// of literals per field -- guards against the failure mode of this port that would be
    /// hardest to catch any other way: accidentally copy-pasting bucket 0's numbers into the
    /// 1/2/3 arms (still compiles, still produces valid CDFs, still passes every shape/self-check
    /// gate the generator script ran -- only a direct value comparison across buckets catches
    /// it). Samples one field per distinct shape family (`txb_skip`/`dc_sign`/`eob_bin_16`
    /// 2D-plane/`coeff_base_eob`/`coeff_base`/`coeff_br`, plus their `_chroma` siblings) rather
    /// than all 26, since a copy-paste bug would essentially never affect only a handful of the
    /// 26 fields while sparing the rest (they were all generated by the same script, from the
    /// same per-field extraction rule).
    #[test]
    fn new_with_qcat_buckets_hold_real_distinct_values() {
        let c0 = CdfContext::new_with_qcat(0);
        let c1 = CdfContext::new_with_qcat(1);
        let c2 = CdfContext::new_with_qcat(2);
        let c3 = CdfContext::new_with_qcat(3);

        // Every pairwise combination must differ for each sampled field -- not just "0 != 3"
        // (which alone wouldn't catch e.g. 1 and 2 being accidental duplicates of each other).
        macro_rules! assert_all_buckets_differ {
            ($field:ident) => {
                assert_ne!(
                    c0.$field,
                    c1.$field,
                    "{}: bucket 0 and 1 are identical -- looks like a copy-paste",
                    stringify!($field)
                );
                assert_ne!(
                    c0.$field,
                    c2.$field,
                    "{}: bucket 0 and 2 are identical -- looks like a copy-paste",
                    stringify!($field)
                );
                assert_ne!(
                    c0.$field,
                    c3.$field,
                    "{}: bucket 0 and 3 are identical -- looks like a copy-paste",
                    stringify!($field)
                );
                assert_ne!(
                    c1.$field,
                    c2.$field,
                    "{}: bucket 1 and 2 are identical -- looks like a copy-paste",
                    stringify!($field)
                );
                assert_ne!(
                    c1.$field,
                    c3.$field,
                    "{}: bucket 1 and 3 are identical -- looks like a copy-paste",
                    stringify!($field)
                );
                assert_ne!(
                    c2.$field,
                    c3.$field,
                    "{}: bucket 2 and 3 are identical -- looks like a copy-paste",
                    stringify!($field)
                );
            };
        }

        assert_all_buckets_differ!(txb_skip_cdf);
        assert_all_buckets_differ!(txb_skip_cdf_chroma);
        // `dc_sign` is the one real exception: dav1d's own `default_coef_cdf[0..=3].dc_sign` is
        // *genuinely* byte-for-byte identical across all 4 qindex buckets in the reference source
        // (confirmed directly against `cdf.c` -- every other field's bucket varies, only this
        // one's sign-bit default doesn't) -- so the non-degenerate check here is the opposite of
        // every other field: bucket 0 and 3 must match, proving the port didn't accidentally
        // invent variation dav1d's source doesn't have.
        assert_eq!(
            c0.dc_sign_cdf, c3.dc_sign_cdf,
            "dc_sign_cdf: dav1d's real default is identical across all 4 qindex buckets -- \
             buckets should match exactly, not diverge"
        );
        assert_eq!(
            c0.dc_sign_cdf_chroma, c3.dc_sign_cdf_chroma,
            "dc_sign_cdf_chroma: dav1d's real default is identical across all 4 qindex buckets -- \
             buckets should match exactly, not diverge"
        );
        assert_all_buckets_differ!(eob_bin_16_cdf);
        assert_all_buckets_differ!(eob_bin_16_cdf_chroma);
        assert_all_buckets_differ!(eob_bin_512_cdf);
        assert_all_buckets_differ!(eob_bin_512_cdf_chroma);
        assert_all_buckets_differ!(eob_hi_bit_cdf);
        assert_all_buckets_differ!(eob_hi_bit_cdf_chroma);
        assert_all_buckets_differ!(coeff_base_eob_cdf);
        assert_all_buckets_differ!(coeff_base_eob_cdf_chroma);
        assert_all_buckets_differ!(coeff_base_cdf);
        assert_all_buckets_differ!(coeff_base_cdf_chroma);
        assert_all_buckets_differ!(coeff_br_cdf);
        assert_all_buckets_differ!(coeff_br_cdf_chroma);

        // Every other (qcat-independent) field must stay identical across buckets -- only the
        // residual-coefficient family is qindex-bucketed in real AV1.
        assert_eq!(c0.partition_cdfs, c3.partition_cdfs);
        assert_eq!(c0.skip_cdf, c3.skip_cdf);
        assert_eq!(c0.txsz_cdf, c3.txsz_cdf);
        assert_eq!(c0.txtp_intra1_cdf, c3.txtp_intra1_cdf);
    }

    /// `qcat.min(3)` should clamp rather than panic for any caller that passes an un-derived
    /// value >3 (dav1d's real bucket count) -- matches `new_with_qcat`'s doc.
    #[test]
    fn new_with_qcat_clamps_values_above_three() {
        let c3 = CdfContext::new_with_qcat(3);
        let c_over = CdfContext::new_with_qcat(200);
        assert_eq!(c3.coeff_base_cdf, c_over.coeff_base_cdf);
        assert_eq!(c3.txb_skip_cdf, c_over.txb_skip_cdf);
    }

    /// `new()` must stay byte-for-byte equivalent to `new_with_qcat(0)` -- every pre-existing
    /// caller (tests, `tile/partition.rs`'s no-real-caller helper) relies on this being a true
    /// zero-behavior-change wrapper.
    #[test]
    fn new_is_equivalent_to_new_with_qcat_zero() {
        let via_new = CdfContext::new();
        let via_explicit = CdfContext::new_with_qcat(0);
        assert_eq!(via_new.txb_skip_cdf, via_explicit.txb_skip_cdf);
        assert_eq!(via_new.coeff_base_cdf, via_explicit.coeff_base_cdf);
        assert_eq!(via_new.coeff_br_cdf, via_explicit.coeff_br_cdf);
        assert_eq!(via_new.dc_sign_cdf, via_explicit.dc_sign_cdf);
        assert_eq!(via_new.partition_cdfs, via_explicit.partition_cdfs);
    }
}
