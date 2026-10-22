//! Per-transform-block summary of the coefficients `SymbolDecoder` read.

/// Summary statistics for one transform block's residual coefficients -- see
/// `SymbolDecoder::read_residual_block`'s doc for what this deliberately does and doesn't capture
/// (aggregate magnitude, not per-position values or real pixel-domain energy).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ResidualBlockStats {
    /// True if `txb_skip` (all_zero) was signaled -- every other field is 0 in that case.
    pub all_zero: bool,
    /// Number of nonzero coefficient levels read.
    pub nonzero_count: u32,
    /// Sum of absolute coefficient levels (a rough energy proxy).
    pub sum_abs_level: u64,
    /// Largest single coefficient level read.
    pub max_level: u16,
    /// The raw `dc_sign` bit read for this block's DC coefficient (`Some(1)`=negative,
    /// `Some(0)`=positive), or `None` if no `dc_sign` bit was read at all (`all_zero`, or the DC
    /// position's level decoded to `0`) -- feeds `TileContext::set_residual_ctx`'s
    /// `dc_sign_symbol` param, see its doc for the "neutral" convention this maps to.
    pub dc_sign_value: Option<u8>,
}

impl ResidualBlockStats {
    /// Accounts for one non-zero coefficient of absolute value `level`.
    pub(super) fn add_level(&mut self, level: u32) {
        self.nonzero_count += 1;
        self.sum_abs_level += u64::from(level);
        self.max_level = self.max_level.max(level.min(u32::from(u16::MAX)) as u16);
    }
}
