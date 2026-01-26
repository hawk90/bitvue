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
        let step = CDF_SCALE / num_symbols as u16;
        for i in 1..=num_symbols {
            let value = if i == num_symbols {
                CDF_SCALE // Last entry must be exactly 32768
            } else {
                step * i as u16
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
    /// Partition CDFs indexed by block size log2 (2..=7)
    /// - block_size_log2 = 2 → 4x4 (1 symbol)
    /// - block_size_log2 = 3 → 8x8 (4 symbols)
    /// - block_size_log2 = 4 → 16x16 (10 symbols)
    /// - block_size_log2 = 5 → 32x32 (10 symbols)
    /// - block_size_log2 = 6 → 64x64 (10 symbols)
    /// - block_size_log2 = 7 → 128x128 (10 symbols)
    partition_cdfs: Vec<PartitionCdf>,

    /// Skip flag CDF (2 symbols: false, true)
    /// [0, prob_true, 32768]
    skip_cdf: Vec<u16>,

    /// Prediction mode CDFs
    /// For INTRA: 13 modes (DC, V, H, D45, D135, D113, D157, D203, D67, SMOOTH, SMOOTH_V, SMOOTH_H, PAETH)
    /// For INTER: 4 modes (NEWMV, NEARESTMV, NEARMV, GLOBALMV)
    intra_mode_cdf: Vec<u16>,
    inter_mode_cdf: Vec<u16>,

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
}

impl CdfContext {
    /// Create new CDF context with default values
    pub fn new() -> Self {
        let mut partition_cdfs = Vec::new();

        // Block size log2 = 2 (4x4): NONE only
        partition_cdfs.push(PartitionCdf::biased_none(1));

        // Block size log2 = 3 (8x8): NONE, HORZ, VERT, SPLIT
        partition_cdfs.push(PartitionCdf::biased_none(4));

        // Block size log2 = 4..=7 (16x16, 32x32, 64x64, 128x128): All 10 partitions
        for _ in 4..=7 {
            partition_cdfs.push(PartitionCdf::biased_none(10));
        }

        // Skip flag CDF: 20% skip rate (most blocks are not skipped)
        let skip_cdf = vec![
            0,                               // false: 0
            (CDF_SCALE as f32 * 0.8) as u16, // false: 80%
            CDF_SCALE,                       // true: 20%
        ];

        // INTRA mode CDF (13 modes)
        // Biased toward DC_PRED (most common)
        let intra_mode_cdf = vec![
            0,                                // Start
            (CDF_SCALE as f32 * 0.30) as u16, // DC_PRED: 30%
            (CDF_SCALE as f32 * 0.40) as u16, // V_PRED: 10%
            (CDF_SCALE as f32 * 0.50) as u16, // H_PRED: 10%
            (CDF_SCALE as f32 * 0.58) as u16, // D45_PRED: 8%
            (CDF_SCALE as f32 * 0.66) as u16, // D135_PRED: 8%
            (CDF_SCALE as f32 * 0.72) as u16, // D113_PRED: 6%
            (CDF_SCALE as f32 * 0.78) as u16, // D157_PRED: 6%
            (CDF_SCALE as f32 * 0.84) as u16, // D203_PRED: 6%
            (CDF_SCALE as f32 * 0.90) as u16, // D67_PRED: 6%
            (CDF_SCALE as f32 * 0.94) as u16, // SMOOTH_PRED: 4%
            (CDF_SCALE as f32 * 0.97) as u16, // SMOOTH_V_PRED: 3%
            (CDF_SCALE as f32 * 0.99) as u16, // SMOOTH_H_PRED: 2%
            CDF_SCALE,                        // PAETH_PRED: 1%
        ];

        // INTER mode CDF (4 modes)
        // Biased toward NEWMV (explicit motion vectors)
        let inter_mode_cdf = vec![
            0,                                // Start
            (CDF_SCALE as f32 * 0.50) as u16, // NEWMV: 50%
            (CDF_SCALE as f32 * 0.75) as u16, // NEARESTMV: 25%
            (CDF_SCALE as f32 * 0.95) as u16, // NEARMV: 20%
            CDF_SCALE,                        // GLOBALMV: 5%
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
        assert_eq!(*mv_joint_cdf.last().unwrap(), CDF_SCALE);

        // MV sign CDF: 50/50 positive/negative (uniform)
        let mv_sign_cdf = vec![
            0,                               // Start
            (CDF_SCALE as f32 * 0.5) as u16, // Positive: 50%
            CDF_SCALE,                       // Negative: 50%
        ];

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
        assert_eq!(*mv_class_cdf.last().unwrap(), CDF_SCALE);

        // MV bit CDF: 50/50 for each bit (uniform)
        let mv_bit_cdf = vec![
            0,                               // Start
            (CDF_SCALE as f32 * 0.5) as u16, // 0: 50%
            CDF_SCALE,                       // 1: 50%
        ];

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

        // Delta Q sign CDF: Slightly biased toward positive
        let delta_q_sign_cdf = vec![
            0,                                // Start
            (CDF_SCALE as f32 * 0.55) as u16, // Positive: 55%
            CDF_SCALE,                        // Negative: 45%
        ];

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

        Self {
            partition_cdfs,
            skip_cdf,
            intra_mode_cdf,
            inter_mode_cdf,
            mv_joint_cdf,
            mv_sign_cdf,
            mv_class_cdf,
            mv_bit_cdf,
            delta_q_cdf,
            delta_q_sign_cdf,
            diff_cdf,
        }
    }

    /// Get partition CDF for block size
    ///
    /// Block size is log2 of actual size:
    /// - 2 → 4x4
    /// - 3 → 8x8
    /// - 4 → 16x16
    /// - 5 → 32x32
    /// - 6 → 64x64
    /// - 7 → 128x128
    pub fn get_partition_cdf(&self, block_size_log2: u8) -> &[u16] {
        let index = (block_size_log2 as usize)
            .saturating_sub(2)
            .min(self.partition_cdfs.len() - 1);
        self.partition_cdfs[index].as_slice()
    }

    /// Update partition CDF (TODO: implement adaptive CDFs)
    #[allow(dead_code)]
    pub fn update_partition_cdf(&mut self, _block_size_log2: u8, _symbol: u8) {
        // TODO: Implement CDF adaptation
        // Per AV1 spec, CDFs are updated after each symbol to improve compression
        // For MVP, we use static CDFs
    }

    /// Get skip flag CDF
    ///
    /// Returns CDF for skip flag (2 symbols: false, true)
    pub fn get_skip_cdf(&self) -> &[u16] {
        &self.skip_cdf
    }

    /// Get INTRA prediction mode CDF
    ///
    /// Returns CDF for INTRA modes (13 symbols)
    pub fn get_intra_mode_cdf(&self) -> &[u16] {
        &self.intra_mode_cdf
    }

    /// Get INTER prediction mode CDF
    ///
    /// Returns CDF for INTER modes (4 symbols)
    pub fn get_inter_mode_cdf(&self) -> &[u16] {
        &self.inter_mode_cdf
    }

    /// Get MV joint CDF
    ///
    /// Returns CDF for MV joint type (4 symbols):
    /// - MV_JOINT_ZERO (both components zero)
    /// - MV_JOINT_HNZVZ (horizontal non-zero, vertical zero)
    /// - MV_JOINT_HZVNZ (horizontal zero, vertical non-zero)
    /// - MV_JOINT_HNZVNZ (both components non-zero)
    pub fn get_mv_joint_cdf(&self) -> &[u16] {
        &self.mv_joint_cdf
    }

    /// Get MV sign CDF
    ///
    /// Returns CDF for MV sign (2 symbols: positive, negative)
    pub fn get_mv_sign_cdf(&self) -> &[u16] {
        &self.mv_sign_cdf
    }

    /// Get MV class CDF
    ///
    /// Returns CDF for MV magnitude class (12 symbols)
    pub fn get_mv_class_cdf(&self) -> &[u16] {
        &self.mv_class_cdf
    }

    /// Get MV bit CDF
    ///
    /// Returns CDF for MV magnitude bits (2 symbols: 0, 1)
    pub fn get_mv_bit_cdf(&self) -> &[u16] {
        &self.mv_bit_cdf
    }

    /// Get Delta Q CDF
    ///
    /// Returns CDF for delta_q_abs (quantization parameter delta)
    /// Per AV1 Spec Section 5.11.38 (Quantization Parameter Delta)
    pub fn get_delta_q_cdf(&self) -> &[u16] {
        &self.delta_q_cdf
    }

    /// Get Delta Q sign CDF
    ///
    /// Returns CDF for delta_q_sign_bit (2 symbols: positive, negative)
    pub fn get_delta_q_sign_cdf(&self) -> &[u16] {
        &self.delta_q_sign_cdf
    }

    /// Get general diff CDF
    ///
    /// Returns CDF for reading variable-length differences
    /// Used when delta_q_abs is >= 4
    pub fn get_diff_cdf(&self) -> &[u16] {
        &self.diff_cdf
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
        let context = CdfContext::new();

        // 4x4 block (log2 = 2): 1 symbol
        let cdf_4x4 = context.get_partition_cdf(2);
        assert_eq!(cdf_4x4.len(), 2); // 1 symbol + end marker

        // 8x8 block (log2 = 3): 4 symbols
        let cdf_8x8 = context.get_partition_cdf(3);
        assert_eq!(cdf_8x8.len(), 5); // 4 symbols + end marker

        // 16x16 block (log2 = 4): 10 symbols
        let cdf_16x16 = context.get_partition_cdf(4);
        assert_eq!(cdf_16x16.len(), 11); // 10 symbols + end marker

        // 128x128 block (log2 = 7): 10 symbols
        let cdf_128x128 = context.get_partition_cdf(7);
        assert_eq!(cdf_128x128.len(), 11);
    }

    #[test]
    fn test_cdf_scale() {
        assert_eq!(CDF_SCALE, 32768);
        assert_eq!(CDF_SCALE, 1 << 15);
    }

    #[test]
    fn test_mv_class_cdf_spec_compliant() {
        let context = CdfContext::new();
        let cdf = context.get_mv_class_cdf();

        // Verify length: 11 classes + start + end = 12 values
        assert_eq!(cdf.len(), 12);

        // Verify first and last values
        assert_eq!(cdf[0], 0, "CDF should start at 0");
        assert_eq!(cdf[11], CDF_SCALE, "CDF should end at 32768");

        // Verify monotonically increasing
        for i in 1..cdf.len() {
            assert!(
                cdf[i] >= cdf[i - 1],
                "CDF should be monotonically increasing at index {}",
                i
            );
        }

        // Verify spec-compliant values from rav1d
        assert_eq!(cdf[1], 28672, "Class 0 (0 qpel) cumulative probability");
        assert_eq!(cdf[2], 30976, "Class 1 (±1 qpel) cumulative probability");
        assert_eq!(cdf[3], 31858, "Class 2 (±2-3 qpel) cumulative probability");
        assert_eq!(cdf[4], 32320, "Class 3 (±4-7 qpel) cumulative probability");

        // Verify realistic distribution (most MVs are small magnitude)
        let prob_class_0 = cdf[1] as f32 / CDF_SCALE as f32;
        assert!(
            prob_class_0 > 0.85,
            "Class 0 should be very common (>85%), got {:.1}%",
            prob_class_0 * 100.0
        );
    }

    #[test]
    fn test_mv_joint_cdf_spec_compliant() {
        let context = CdfContext::new();
        let cdf = context.get_mv_joint_cdf();

        // Verify length: 4 symbols + start = 5 values
        assert_eq!(cdf.len(), 5);

        // Verify first and last values
        assert_eq!(cdf[0], 0, "CDF should start at 0");
        assert_eq!(cdf[4], CDF_SCALE, "CDF should end at 32768");

        // Verify monotonically increasing
        for i in 1..cdf.len() {
            assert!(
                cdf[i] >= cdf[i - 1],
                "CDF should be monotonically increasing at index {}",
                i
            );
        }

        // Verify spec-compliant values from rav1d
        assert_eq!(cdf[1], 4096, "MV_JOINT_ZERO cumulative probability");
        assert_eq!(cdf[2], 11264, "MV_JOINT_HNZVZ cumulative probability");
        assert_eq!(cdf[3], 19328, "MV_JOINT_HZVNZ cumulative probability");
    }

    #[test]
    fn test_mv_sign_cdf() {
        let context = CdfContext::new();
        let cdf = context.get_mv_sign_cdf();

        // Verify length: 2 symbols + start = 3 values
        assert_eq!(cdf.len(), 3);

        // Verify uniform distribution (50/50)
        assert_eq!(cdf[0], 0);
        assert_eq!(cdf[1], 16384, "Sign should be 50/50");
        assert_eq!(cdf[2], CDF_SCALE);
    }

    #[test]
    fn test_mv_bit_cdf() {
        let context = CdfContext::new();
        let cdf = context.get_mv_bit_cdf();

        // Verify length: 2 symbols + start = 3 values
        assert_eq!(cdf.len(), 3);

        // Verify uniform distribution (50/50)
        assert_eq!(cdf[0], 0);
        assert_eq!(cdf[1], 16384, "Bit should be 50/50");
        assert_eq!(cdf[2], CDF_SCALE);
    }
}
