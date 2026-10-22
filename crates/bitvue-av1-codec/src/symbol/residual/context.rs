//! Pure context derivations for coefficient reading: no decoder state, only transform size and `eob`.

use super::cdf;

/// `coeff_base_eob`'s context (0..=3): purely a function of `eob` (the raw last-scan-index value,
/// see `read_residual_block`'s doc for why this is `real coefficient count - 1`, not a count) and
/// tx size, no neighbor/level state needed (unlike `coeff_base`/`coeff_br`, still deferred -- see
/// `read_residual_block`'s doc). Source: rav1d's `decode_coefs` (`memorysafety/rav1d`,
/// BSD-2-Clause, `src/recon_tmpl.c`): the single-coefficient case (`eob == 0`) is a distinct
/// "dc-only" branch that reads `eob_cdf[0]` -- context `0` hardcoded, NOT the generic formula
/// (which would otherwise give `1`, since `0` is never `>` either positive threshold) -- only
/// `eob >= 1` goes through `1 + (eob > 2<<tx2dszctx) + (eob > 4<<tx2dszctx)`, where `tx2dszctx` is
/// `2 * min(tx_size_class, 3)` for a square transform (real AV1 caps the 2D coefficient scan's
/// size class at 32x32).
pub(super) fn coeff_base_eob_context(eob: u32, tx_width_px: u32, tx_height_px: u32) -> u8 {
    if eob == 0 {
        return 0;
    }
    // dav1d: `tx2dszctx = min(lw, TX_32X32) + min(lh, TX_32X32)` (log2 of px / 4 per side).
    let tx2dszctx =
        (cdf::tx_size_class(tx_width_px).min(3) + cdf::tx_size_class(tx_height_px).min(3)) as u32;
    1 + u8::from(eob > (2 << tx2dszctx)) + u8::from(eob > (4 << tx2dszctx))
}

/// Spec `txSzCtx` (dav1d `TxfmInfo.ctx`) of a transform: `(Tx_Size_Sqr + Tx_Size_Sqr_Up + 1) >> 1`,
/// i.e. the average of the smaller and the larger side's size class (log2 of px / 4), rounded up.
/// It selects the coefficient CDF family. Equal to the larger side's class for square and 2:1
/// transforms and one lower for 4:1 ones.
pub(crate) fn tx_size_ctx(tx_width_px: u32, tx_height_px: u32) -> usize {
    let a = cdf::tx_size_class(tx_width_px);
    let b = cdf::tx_size_class(tx_height_px);
    (a.min(b) + a.max(b) + 1) >> 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coeff_base_eob_context_increases_with_eob() {
        // tx_size_px=16 -> tx_size_class=2 -> tx2dszctx=4 -> thresholds 2<<4=32, 4<<4=64.
        assert_eq!(coeff_base_eob_context(10, 16, 16), 1); // below both thresholds
        assert_eq!(coeff_base_eob_context(40, 16, 16), 2); // above first only
        assert_eq!(coeff_base_eob_context(100, 16, 16), 3); // above both
    }

    /// dav1d: `tx2dszctx = min(lw, 3) + min(lh, 3)` -- the sum of the two sides' size classes, not
    /// twice the larger one. A 4x8 transform has `tx2dszctx = 0 + 1 = 1` (thresholds 4 and 8), where
    /// a square 8x8 has 2.
    #[test]
    fn test_coeff_base_eob_context_uses_the_sum_of_both_sides() {
        assert_eq!(coeff_base_eob_context(5, 4, 8), 2); // 4 < 5 <= 8
        assert_eq!(coeff_base_eob_context(9, 4, 8), 3); // 9 > 8
        assert_eq!(coeff_base_eob_context(5, 8, 8), 1); // 8x8: thresholds 8 and 16
    }

    #[test]
    fn test_coeff_base_eob_context_caps_tx2dszctx_at_32x32() {
        // 32x32 and 64x64 share the same tx2dszctx (real AV1 caps the 2D coefficient scan's size
        // class at 32x32) -- same context for the same `eob`.
        assert_eq!(
            coeff_base_eob_context(500, 32, 32),
            coeff_base_eob_context(500, 64, 64)
        );
    }

    /// `eob == 0` (rav1d's "dc-only" branch, a single-coefficient transform block) is context `0`,
    /// NOT the generic formula's result (`1`, since `0` is never `>` either positive threshold) --
    /// see this function's doc for the desync this closes.
    #[test]
    fn test_coeff_base_eob_context_dc_only_is_context_zero() {
        assert_eq!(coeff_base_eob_context(0, 16, 16), 0);
        assert_eq!(coeff_base_eob_context(0, 32, 32), 0);
    }
}
