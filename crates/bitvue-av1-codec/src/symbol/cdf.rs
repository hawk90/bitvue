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

    /// Delta Q CDF (for quantization parameter deltas)
    /// Per AV1 Spec Section 5.11.38 (Quantization Parameter Delta)
    /// Delta Q values are in range [-MAX_DELTA_Q, MAX_DELTA_Q] where MAX_DELTA_Q = 63
    /// We encode the absolute value (0..63) and sign separately
    delta_q_cdf: Vec<u16>,

    /// Delta Q sign CDF (2 symbols: positive, negative)
    delta_q_sign_cdf: Vec<u16>,

    /// General diff CDF for reading variable-length differences
    /// Used for delta_q_abs when larger values are needed
    diff_cdf: Vec<u16>,

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
    /// `src/cdf.rs`), first qindex-bucket variant only (see `eob_bin_16_cdf`'s doc).
    txb_skip_cdf: [[Vec<u16>; 7]; 5],
    /// `coeff_base` -- level (0..=3) for every other coefficient position (4 symbols), indexed
    /// `[tx_size_class 0..=4][ctx 0..=40]`. Real neighbor context (`symbol::scan::lo_ctx`, ported
    /// from rav1d `get_lo_ctx`) -- see `SymbolDecoder::read_residual_block`'s doc. Source: rav1d
    /// `coef.base_tok` (`memorysafety/rav1d`, BSD-2-Clause, `src/cdf.rs`), first qindex-bucket
    /// variant only, chroma=0 (luma-only, same precedent as `txb_skip_cdf`).
    coeff_base_cdf: [[Vec<u16>; 41]; 5],
    /// `coeff_br` -- range-extension increment (0..=3), read in a loop while extending a level
    /// past `NUM_BASE_LEVELS` (4 symbols), indexed `[min(tx_size_class,3)][ctx 0..=20]` (one
    /// fewer tx-size bucket than `coeff_base_cdf` -- 32x32 and 64x64 share a bucket here, matching
    /// rav1d's `min(t_dim->ctx, 3)`). Real context (position band + `lo_ctx`'s `hi_mag`, reused
    /// directly rather than recomputed) -- see `SymbolDecoder::read_residual_block`'s doc. Source:
    /// rav1d `coef.br_tok`, first qindex-bucket variant only, chroma=0.
    coeff_br_cdf: [[Vec<u16>; 21]; 4],
    /// `dc_sign` -- sign of the DC (position 0) coefficient (2 symbols), indexed `[ctx 0..=2]`.
    /// Real above/left neighbor context (`TileContext::dc_sign_context`) is wired alongside
    /// `txb_skip_cdf` -- see `SymbolDecoder::read_residual_block`'s doc; other callers pass a
    /// fixed `ctx = 0` (chroma axis fixed to 0). AC coefficient signs are read as literal
    /// (uniform) bits per spec, not CDF-coded. Source: rav1d `coef.dc_sign`, first qindex-bucket
    /// variant only.
    dc_sign_cdf: [Vec<u16>; 3],

    /// `eob_bin` (spec 5.11.39's `eob_pt_*`), real spec/rav1d default CDFs + real adaptation, one
    /// family per coefficient-count class (16/64/256/1024, the only 4 of rav1d's 7 classes a
    /// *square*-only transform ever selects -- see `get_eob_bin_cdf_mut`'s doc) each
    /// indexed by `is_1d` (0..=1, from `SymbolDecoder::read_transform_type_is_1d`; the real
    /// spec's `chroma` axis is always 0 here since this crate only reads luma residual, see
    /// `read_residual_block`'s doc). Source: rav1d `eob_bin_16/64/256/1024`
    /// (`memorysafety/rav1d`, BSD-2-Clause, `src/cdf.rs`) -- only the first of rav1d's 4
    /// redundant qindex-bucket default-CDF variants is ported (same "one representative default"
    /// precedent as every other CDF in this struct; real per-context *selection* is what's new
    /// here, not per-qindex tuning). `eob_bin_1024` has no `is_1d` axis in rav1d itself (only
    /// `chroma`), hence the plain `[Vec<u16>; 1]`-shaped (i.e. unindexed) field below.
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
    /// one. Source: rav1d `eob_hi_bit` (`memorysafety/rav1d`, BSD-2-Clause, `src/cdf.rs`), first
    /// qindex-bucket variant only (see `eob_bin_16_cdf`'s doc).
    eob_hi_bit_cdf: [[Vec<u16>; 11]; 5],
    /// `coeff_base_eob` -- level (1..=3) for the highest-scan-order nonzero coefficient (3
    /// symbols), indexed `[tx_size_class 0..=4][ctx 0..=3]` (chroma axis fixed to 0). Real
    /// spec/rav1d default CDFs + real adaptation -- context is a pure arithmetic function of
    /// `eob` and tx size (see `SymbolDecoder::coeff_base_eob_context`'s doc), no neighbor/above-
    /// left state needed. Source: rav1d `eob_base_tok` (`memorysafety/rav1d`, BSD-2-Clause,
    /// `src/cdf.rs`), first qindex-bucket variant only (see `eob_bin_16_cdf`'s doc).
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
    /// `default_coef_cdf[0].skip`/`.dc_sign`), first qindex-bucket variant only, same precedent as
    /// every luma table above.
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
    /// Create new CDF context with default values
    pub fn new() -> Self {
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

        // seg_pred/seg_id (spec 5.11.9/5.11.10 `segment_id()`): real spec/rav1d default CDFs
        // (`default_cdf.m.seg_pred`/`.seg_id`, `src/cdf.c`).
        let seg_pred_cdf: [Vec<u16>; 3] = [16384, 16384, 16384].map(binary_ctx_cdf);
        let seg_id_cdf: [Vec<u16>; 3] = [
            multi_ctx_cdf(&[5622, 7893, 16093, 18233, 27809, 28373, 32533]),
            multi_ctx_cdf(&[14274, 18230, 22557, 24935, 29980, 30851, 32344]),
            multi_ctx_cdf(&[27527, 28487, 28723, 28890, 32397, 32647, 32679]),
        ];

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

        // Delta Q CDF (for reading delta_q_abs)
        // Per AV1 spec, delta_q_abs is encoded using a variable-length code
        // Values are heavily biased toward 0 (most QP deltas are small)
        // Default values from AV1 spec / rav1d reference implementation
        let delta_q_counts: [u16; 5] = [
            28672, // 0 (no change): ~87%
            3488,  // 1: ~11%
            448,   // 2: ~1.4%
            96,    // 3: ~0.3%
            64,    // 4+: ~0.2% (collapsed into "4+" for simplicity)
        ];
        let mut delta_q_cdf = Vec::with_capacity(delta_q_counts.len() + 1);
        delta_q_cdf.push(0);
        let mut cumulative = 0u16;
        for &count in &delta_q_counts {
            cumulative = cumulative.saturating_add(count);
            delta_q_cdf.push(cumulative.min(CDF_SCALE));
        }
        // Ensure the last entry is exactly CDF_SCALE
        if let Some(last) = delta_q_cdf.last_mut() {
            *last = CDF_SCALE;
        }
        let delta_q_cdf = to_descending(&delta_q_cdf);

        // Delta Q sign CDF: Slightly biased toward positive
        let delta_q_sign_cdf = vec![
            0,                                // Start
            (CDF_SCALE as f32 * 0.55) as u16, // Positive: 55%
            CDF_SCALE,                        // Negative: 45%
        ];
        let delta_q_sign_cdf = to_descending(&delta_q_sign_cdf);

        // General diff CDF for variable-length differences
        // Used when delta_q_abs is >= 4
        // Uses a geometric distribution (higher values less likely)
        let diff_cdf = vec![
            0,                                // Start
            (CDF_SCALE as f32 * 0.50) as u16, // 0: 50%
            (CDF_SCALE as f32 * 0.75) as u16, // 1: 25%
            (CDF_SCALE as f32 * 0.90) as u16, // 2: 15%
            (CDF_SCALE as f32 * 0.97) as u16, // 3: 7%
            (CDF_SCALE as f32 * 0.99) as u16, // 4: 2%
            CDF_SCALE,                        // 5+: 1%
        ];
        let diff_cdf = to_descending(&diff_cdf);

        // txb_skip: real spec/rav1d default CDFs, indexed [tx_size_class][ctx 0..=6] (chroma=0
        // fixed -- see `txb_skip_cdf`'s doc). Source: rav1d `coef.skip`, first qindex-bucket
        // variant.
        let txb_skip_cdf: [[Vec<u16>; 7]; 5] = [
            [31849, 5892, 12112, 21935, 20289, 27473, 32487].map(binary_ctx_cdf),
            [31548, 1549, 10130, 16656, 18591, 26308, 32537].map(binary_ctx_cdf),
            [29957, 5391, 18039, 23566, 22431, 25822, 32197].map(binary_ctx_cdf),
            [17920, 1818, 7282, 25273, 10923, 31554, 32624].map(binary_ctx_cdf),
            [6308, 117, 1638, 2161, 16384, 10923, 30247].map(binary_ctx_cdf),
        ];

        // eob_bin: real spec/rav1d default CDFs, indexed [is_1d] (chroma=0 fixed -- see
        // `eob_bin_16_cdf`'s doc). Source: rav1d `eob_bin_16/64/256/1024`, first qindex-bucket
        // variant.
        let eob_bin_16_cdf = [
            multi_ctx_cdf(&[840, 1039, 1980, 4895]),
            multi_ctx_cdf(&[370, 671, 1883, 4471]),
        ];
        let eob_bin_32_cdf = [
            multi_ctx_cdf(&[400, 520, 977, 2102, 6542]),
            multi_ctx_cdf(&[210, 405, 1315, 3326, 7537]),
        ];
        let eob_bin_64_cdf = [
            multi_ctx_cdf(&[329, 498, 1101, 1784, 3265, 7758]),
            multi_ctx_cdf(&[335, 730, 1459, 5494, 8755, 12997]),
        ];
        let eob_bin_128_cdf = [
            multi_ctx_cdf(&[219, 482, 1140, 2091, 3680, 6028, 12586]),
            multi_ctx_cdf(&[371, 699, 1254, 4830, 9479, 12562, 17497]),
        ];
        let eob_bin_256_cdf = [
            multi_ctx_cdf(&[310, 584, 1887, 3589, 6168, 8611, 11352, 15652]),
            multi_ctx_cdf(&[998, 1850, 2998, 5604, 17341, 19888, 22899, 25583]),
        ];
        let eob_bin_512_cdf =
            multi_ctx_cdf(&[641, 983, 3707, 5430, 10234, 14958, 18788, 23412, 26061]);
        let eob_bin_1024_cdf =
            multi_ctx_cdf(&[393, 421, 751, 1623, 3160, 6352, 13345, 18047, 22571, 25830]);

        // eob_hi_bit: real spec/rav1d default CDFs, indexed [tx_size_class][eob_bin] (chroma=0
        // fixed). Source: rav1d `eob_hi_bit`, first qindex-bucket variant.
        let eob_hi_bit_cdf: [[Vec<u16>; 11]; 5] = [
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
        ];

        // coeff_base_eob: real spec/rav1d default CDFs, indexed [tx_size_class][ctx] (chroma=0
        // fixed). Source: rav1d `eob_base_tok`, first qindex-bucket variant.
        let coeff_base_eob_cdf: [[Vec<u16>; 4]; 5] = [
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
        ];

        // Chroma-plane residual CDFs -- see `txb_skip_cdf_chroma`'s doc for scope. Real chroma
        // (rav1d/dav1d `[chroma=1]` axis) default values, first qindex-bucket variant. `txb_skip`
        // indexed `[tx_size_class][ctx 0..=5]` -- ctx is the local remap (`TileContext::
        // txb_skip_context_chroma`'s doc) of dav1d's real chroma ctx 7..=12 from the same
        // `default_coef_cdf[0].skip[tx_size_class]` row luma's 0..=6 came from (`src/cdf.c`).
        let txb_skip_cdf_chroma: [[Vec<u16>; 6]; 4] = [
            [7654, 19473, 29984, 9961, 30242, 32117].map(binary_ctx_cdf),
            [5403, 18096, 30003, 16384, 16384, 16384].map(binary_ctx_cdf),
            [3778, 15336, 28981, 16384, 16384, 16384].map(binary_ctx_cdf),
            [1366, 15628, 30462, 146, 5132, 31657].map(binary_ctx_cdf),
        ];
        // `dc_sign` chroma: real 3-context row (`default_coef_cdf[0].dc_sign[1]`, `src/cdf.c`).
        let dc_sign_cdf_chroma: [Vec<u16>; 3] = [15232, 12928, 17280].map(binary_ctx_cdf);
        let eob_bin_16_cdf_chroma = multi_ctx_cdf(&[3247, 4950, 9688, 14563]);
        let eob_bin_32_cdf_chroma = multi_ctx_cdf(&[2636, 4273, 7588, 11794, 20401]);
        let eob_bin_64_cdf_chroma = multi_ctx_cdf(&[3505, 5304, 10086, 13814, 17684, 23370]);
        let eob_bin_128_cdf_chroma =
            multi_ctx_cdf(&[5245, 7456, 12880, 15852, 20033, 23932, 27608]);
        let eob_bin_256_cdf_chroma =
            multi_ctx_cdf(&[2520, 3240, 5952, 8870, 12577, 17558, 19954, 24168]);
        let eob_bin_512_cdf_chroma =
            multi_ctx_cdf(&[5095, 6446, 9996, 13354, 16017, 17986, 20919, 26129, 29140]);
        let eob_bin_1024_cdf_chroma = multi_ctx_cdf(&[
            1865, 1988, 2930, 4242, 10533, 16538, 21354, 27255, 28546, 31784,
        ]);
        let eob_hi_bit_cdf_chroma: [[Vec<u16>; 11]; 4] = [
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
        ];
        let coeff_base_eob_cdf_chroma: [[Vec<u16>; 4]; 4] = [
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
        ];
        let coeff_base_cdf_chroma: [[Vec<u16>; 41]; 4] = [
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
        ];
        let coeff_br_cdf_chroma: [[Vec<u16>; 21]; 4] = [
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
        ];

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
        // (chroma=0 fixed -- see `coeff_base_cdf`'s doc). Source: rav1d `coef.base_tok`, first
        // qindex-bucket variant only.
        let coeff_base_cdf: [[Vec<u16>; 41]; 5] = [
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
        ];

        // coeff_br: real spec/rav1d default CDFs, indexed [min(tx_size_class,3)][ctx 0..=20]
        // (chroma=0 fixed -- see `coeff_br_cdf`'s doc). Source: rav1d `coef.br_tok`, first
        // qindex-bucket variant only.
        let coeff_br_cdf: [[Vec<u16>; 21]; 4] = [
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
        ];

        // dc_sign: uniform (no real reason to bias this).
        // dc_sign: real spec/rav1d default CDFs, indexed [ctx 0..=2] (chroma=0 fixed -- see
        // `dc_sign_cdf`'s doc). Source: rav1d `coef.dc_sign`, first qindex-bucket variant.
        let dc_sign_cdf = [16000, 13056, 18816].map(binary_ctx_cdf);

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
            seg_pred_cdf,
            seg_id_cdf,
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
            delta_q_sign_cdf,
            diff_cdf,
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

    /// Get mutable Delta Q CDF (`delta_q_abs`, spec 5.11.38) -- adapted after every read.
    pub fn get_delta_q_cdf_mut(&mut self) -> &mut [u16] {
        &mut self.delta_q_cdf
    }

    /// Get mutable Delta Q sign CDF (2 symbols: positive, negative) -- adapted after every read.
    pub fn get_delta_q_sign_cdf_mut(&mut self) -> &mut [u16] {
        &mut self.delta_q_sign_cdf
    }

    /// Get mutable general diff CDF (used when `delta_q_abs` >= 4) -- adapted after every read.
    pub fn get_diff_cdf_mut(&mut self) -> &mut [u16] {
        &mut self.diff_cdf
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
}
