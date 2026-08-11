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
    /// `txb_skip_cdf`: all_zero flag for one transform block (2 symbols).
    txb_skip_cdf: Vec<u16>,
    /// `coeff_base` -- level (0..=3) for every other coefficient position (4 symbols).
    coeff_base_cdf: Vec<u16>,
    /// `coeff_br` -- range-extension increment (0..=3), read in a loop while extending a level
    /// past `NUM_BASE_LEVELS` (4 symbols).
    coeff_br_cdf: Vec<u16>,
    /// `dc_sign` -- sign of the DC (position 0) coefficient (2 symbols). AC coefficient signs are
    /// read as literal (uniform) bits per spec, not CDF-coded.
    dc_sign_cdf: Vec<u16>,

    /// `eob_bin` (spec 5.11.39's `eob_pt_*`), real spec/rav1d default CDFs + real adaptation, one
    /// family per coefficient-count class (16/64/256/1024, the only 4 of rav1d's 7 classes a
    /// *square*-only transform ever selects -- see `SymbolDecoder::eob_bin_context`'s doc) each
    /// indexed by `is_1d` (0..=1, from `SymbolDecoder::read_transform_type_is_1d`; the real
    /// spec's `chroma` axis is always 0 here since this crate only reads luma residual, see
    /// `read_residual_block`'s doc). Source: rav1d `eob_bin_16/64/256/1024`
    /// (`memorysafety/rav1d`, BSD-2-Clause, `src/cdf.rs`) -- only the first of rav1d's 4
    /// redundant qindex-bucket default-CDF variants is ported (same "one representative default"
    /// precedent as every other CDF in this struct; real per-context *selection* is what's new
    /// here, not per-qindex tuning). `eob_bin_1024` has no `is_1d` axis in rav1d itself (only
    /// `chroma`), hence the plain `[Vec<u16>; 1]`-shaped (i.e. unindexed) field below.
    eob_bin_16_cdf: [Vec<u16>; 2],
    eob_bin_64_cdf: [Vec<u16>; 2],
    eob_bin_256_cdf: [Vec<u16>; 2],
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

        // txb_skip: most transform blocks within a non-skip CU are still fully zero.
        let txb_skip_cdf = to_descending(&[0, (CDF_SCALE as f32 * 0.35) as u16, CDF_SCALE]);

        // eob_bin: real spec/rav1d default CDFs, indexed [is_1d] (chroma=0 fixed -- see
        // `eob_bin_16_cdf`'s doc). Source: rav1d `eob_bin_16/64/256/1024`, first qindex-bucket
        // variant.
        let eob_bin_16_cdf = [
            multi_ctx_cdf(&[840, 1039, 1980, 4895]),
            multi_ctx_cdf(&[370, 671, 1883, 4471]),
        ];
        let eob_bin_64_cdf = [
            multi_ctx_cdf(&[329, 498, 1101, 1784, 3265, 7758]),
            multi_ctx_cdf(&[335, 730, 1459, 5494, 8755, 12997]),
        ];
        let eob_bin_256_cdf = [
            multi_ctx_cdf(&[310, 584, 1887, 3589, 6168, 8611, 11352, 15652]),
            multi_ctx_cdf(&[998, 1850, 2998, 5604, 17341, 19888, 22899, 25583]),
        ];
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

        // coeff_base: most non-EOB positions are zero.
        let coeff_base_cdf = to_descending(&[
            0,
            (CDF_SCALE as f32 * 0.70) as u16,
            (CDF_SCALE as f32 * 0.85) as u16,
            (CDF_SCALE as f32 * 0.95) as u16,
            CDF_SCALE,
        ]);

        // coeff_br: range-extension loop should terminate quickly most of the time.
        let coeff_br_cdf = to_descending(&[
            0,
            (CDF_SCALE as f32 * 0.55) as u16,
            (CDF_SCALE as f32 * 0.80) as u16,
            (CDF_SCALE as f32 * 0.93) as u16,
            CDF_SCALE,
        ]);

        // dc_sign: uniform (no real reason to bias this).
        let dc_sign_cdf = to_descending(&[0, CDF_SCALE / 2, CDF_SCALE]);

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
            eob_bin_64_cdf,
            eob_bin_256_cdf,
            eob_bin_1024_cdf,
            eob_hi_bit_cdf,
            coeff_base_eob_cdf,
            txtp_intra1_cdf,
            txtp_intra2_cdf,
            txtp_inter1_cdf,
            txtp_inter2_cdf,
            txtp_inter3_cdf,
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

    /// Get `txb_skip` (all_zero) CDF for one transform block.
    pub fn get_txb_skip_cdf(&self) -> &[u16] {
        &self.txb_skip_cdf
    }

    /// Get mutable `eob_bin` CDF for a transform block of `tx_size_px` pixels per side
    /// (4/8/16/32/64) and `is_1d` (`SymbolDecoder::read_transform_type_is_1d`'s result). 64x64
    /// shares the 1024-coefficient class with 32x32 (real AV1 caps the coefficient scan at the
    /// top-left 32x32 sub-area for larger transforms) -- matches `tx_size_class`'s doc. The
    /// 1024-coefficient class has no real `is_1d` axis in rav1d (see `eob_bin_1024_cdf`'s doc),
    /// so `is_1d` is ignored there.
    pub fn get_eob_bin_cdf_mut(&mut self, tx_size_px: u32, is_1d: bool) -> &mut [u16] {
        let is_1d = usize::from(is_1d);
        match tx_size_px {
            0..=4 => &mut self.eob_bin_16_cdf[is_1d],
            5..=8 => &mut self.eob_bin_64_cdf[is_1d],
            9..=16 => &mut self.eob_bin_256_cdf[is_1d],
            _ => &mut self.eob_bin_1024_cdf, // 32 and 64
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

    /// Get `coeff_base` CDF (level 0..=3 for every other coefficient position).
    pub fn get_coeff_base_cdf(&self) -> &[u16] {
        &self.coeff_base_cdf
    }

    /// Get `coeff_br` CDF (range-extension increment 0..=3).
    pub fn get_coeff_br_cdf(&self) -> &[u16] {
        &self.coeff_br_cdf
    }

    /// Get `dc_sign` CDF (sign of the DC coefficient).
    pub fn get_dc_sign_cdf(&self) -> &[u16] {
        &self.dc_sign_cdf
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
