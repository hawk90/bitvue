//! The partition symbol's CDFs: the per-block-size distribution and the aggregated
//! probabilities of the `split_or_horz` / `split_or_vert` symbols (spec 5.11.4).

use super::CDF_SCALE;

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
